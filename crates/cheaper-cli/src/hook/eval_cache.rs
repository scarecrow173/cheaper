//! `hook::eval_cache` — canonical request fingerprint ベースの verdict
//! cache + leader election(DESIGN-034IefWW6OJ9rvESaNvKUK、issue #127)。
//!
//! 同一 tool call に重複発火する cheaper hook 登録(Devin CLI の
//! `read_config_from` import merge 由来)が、同一 file の token 計測・
//! bounded materialization・bounded search scan を高々 1 回へ収束させる
//! 協調層。責務は「`evaluate` を呼ぶか共有済み verdict を返すか」の
//! 分岐のみ — render・normalize・verdict 判定には関与しない。
//!
//! store は `<userConfigDir>/hook-eval/` 配下のみ(workspace・
//! `.cheaper/` には書かない):
//!
//! ```text
//! <fp>.json                     verdict record(atomic temp+rename)
//! <fp>.lock                     leader election marker(owner token JSON)
//! .<fp>.json.tmp-<pid>-<nanos>  atomic_write temp(orphan sweep 対象)
//! ```
//!
//! 全失敗は `eval_fn` 直接呼出し(通常評価)へ畳まれる fail-open 設計 —
//! 本 module は panic 経路を持たない。

use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use cheaper_core::config::dirs::{resolve_user_config_dir, UserConfigDirOptions};
use cheaper_core::fs_util::{atomic_write, AtomicWriteOptions};
use cheaper_core::token::count::DEFAULT_COUNT_MAX_BYTES;

use super::routing_policy::{
    evaluate, fingerprint_targets, threshold, EvaluateOptions, FingerprintContext, ResolvedKind,
    ResolvedTarget, RoutingRequest, SearchOutputMode, Verdict, MEASURE_ENV_KEYS, MEASURE_TIMEOUT,
};
use crate::count::count_max_bytes;

/// `<userConfigDir>` 配下の dedup store dir 名。
const STORE_DIR_NAME: &str = "hook-eval";

/// fingerprint 構成の format version tag(構成変更時の混同防止)。
const FINGERPRINT_TAG: &str = "hook-eval-v1";

/// 協調の時間定数。production 値は [`DedupLimits::PRODUCTION`]。
/// test は小さい値へ差し替えて wait/stale/TTL を ms スケールで検証する
/// (integration seam)。
#[doc(hidden)]
#[derive(Clone, Copy)]
pub struct DedupLimits {
    /// verdict record の有効期間(discovery 暫定指針 ~60s)。
    pub verdict_ttl: Duration,
    /// lock file の stale 判定閾値。`follower_wait` を十分に超える値を
    /// 取る。長時間 eval 中の live leader が stale と誤判定されても、
    /// takeover は owner token 照合(delete-if-owner)により冗長評価
    /// のみに抑えられる。
    pub lock_stale: Duration,
    /// follower が共有 verdict を待つ上限 — `MEASURE_TIMEOUT` と同値。
    /// これを超えて待っても leader の評価完了は保証されないため、
    /// deadline で自前評価へ移る(dedup による追加 latency の上界)。
    pub follower_wait: Duration,
    /// verdict file の poll 間隔。
    pub poll_interval: Duration,
}

impl DedupLimits {
    /// production 値 — verdict TTL 60s・lock stale 120s・follower wait
    /// 30s(`MEASURE_TIMEOUT` と同一 source、DRY)・poll 50ms。
    pub const PRODUCTION: DedupLimits = DedupLimits {
        verdict_ttl: Duration::from_millis(60_000),
        lock_stale: Duration::from_millis(120_000),
        follower_wait: MEASURE_TIMEOUT,
        poll_interval: Duration::from_millis(50),
    };
}

/// `handle_hook` が呼ぶ wrapper。dedup 適用可能な request では cache
/// hit / leader 評価 / follower 共有を行い、適用外・あらゆる失敗では
/// 通常 `evaluate` へ縮退する。
pub async fn evaluate_dedup(request: &RoutingRequest, opts: &EvaluateOptions<'_>) -> Verdict {
    evaluate_dedup_with(request, opts, &DedupLimits::PRODUCTION, || {
        evaluate(request, opts)
    })
    .await
}

/// 計測呼出しを差し替え可能にした内部実装(integration test seam)。
/// `eval_fn` は通常 `|| evaluate(request, opts)` を capture する
/// closure。test は計数 closure を注入して「計測が高々 1 回」を直接
/// 観測する。`Fn() -> Fut`(引数非依存)にするのは借用を取る closure
/// の HRTB 問題を構造で消すため。
#[doc(hidden)]
pub async fn evaluate_dedup_with<F, Fut>(
    request: &RoutingRequest,
    opts: &EvaluateOptions<'_>,
    limits: &DedupLimits,
    eval_fn: F,
) -> Verdict
where
    F: Fn() -> Fut,
    Fut: std::future::Future<Output = Verdict>,
{
    // fingerprint 対象列挙(sync — `resolve_target` 相当の metadata
    // 呼出しは `evaluate` 内部と同じく async context 内の同期 fs で
    // 既存規則どおり)。targets が完全に空なら定数 allow 経路なので
    // cache を介さず直接評価する(共有する意味も signature も無い)。
    let ctx = fingerprint_targets(request);
    if ctx.targets.is_empty() {
        tracing::debug!(dedup = "bypass", "hook eval dedup");
        return eval_fn().await;
    }

    // `EnvLookup`(&dyn Fn)は非 Send — fingerprint・store root が読む
    // env(`MEASURE_ENV_KEYS` ∪ `CHEAPER_READ_TOKEN_THRESHOLD`)を async
    // 側で owned map へ写す(`measure_tokens` と同じ snapshot pattern。
    // blocking phase へ closure 自体は渡さない)。
    let mut env_map: HashMap<String, String> = HashMap::new();
    for key in MEASURE_ENV_KEYS {
        if let Some(v) = (opts.env)(key) {
            env_map.insert((*key).to_string(), v);
        }
    }
    if let Some(v) = (opts.env)("CHEAPER_READ_TOKEN_THRESHOLD") {
        env_map.insert("CHEAPER_READ_TOKEN_THRESHOLD".to_string(), v);
    }
    let store_dir = resolve_user_config_dir(UserConfigDirOptions {
        env: Some(opts.env),
        homedir: None,
    })
    .join(STORE_DIR_NAME);
    let builtin_dir = opts.builtin_dir.cloned();
    let variant = variant_inputs(request);
    let limits = *limits;

    // phase A(blocking): cache dir 確保 → sweep → policy/env 入力確定
    // → stat signature 収集 → policy 層 signature → fingerprint 確定。
    let prepared = match tokio::task::spawn_blocking(move || {
        prepare_dedup(ctx, variant, env_map, builtin_dir, store_dir, limits)
    })
    .await
    {
        Ok(Some(prepared)) => prepared,
        // create_dir_all 失敗・JoinError — 全て直接評価へ畳む。
        _ => {
            tracing::debug!(dedup = "fallback", "hook eval dedup");
            return eval_fn().await;
        }
    };

    // fast-path gate: 全 target が `len <= threshold` の File(byte
    // fast path で allow 確定)または全て NonFile(評価対象実 file
    // 無し)は協調を skip する — eval が自明に安く、lock・poll の
    // overhead が dedup 効果を上回るため。NonFile の absent marker
    // 自体は fingerprint 構成に残している。
    if prepared.fast_path {
        tracing::debug!(
            dedup = "bypass",
            fingerprint = %prepared.fingerprint,
            "hook eval dedup"
        );
        return eval_fn().await;
    }

    // phase B + follower bounded poll: 1 tick 分の fs 操作(record
    // read → lock `create_new` retry)を `spawn_blocking` へ逃がす。
    // `follower_wait` deadline 到達で自前評価へ移る。
    let deadline = tokio::time::Instant::now() + limits.follower_wait;
    let mut waited = false;
    loop {
        let store_dir = prepared.store_dir.clone();
        let fp = prepared.fingerprint.clone();
        let outcome =
            match tokio::task::spawn_blocking(move || poll_tick(&store_dir, &fp, &limits)).await {
                Ok(outcome) => outcome,
                // JoinError(panic/cancel)も fail-open。
                Err(_) => TickOutcome::Fallback,
            };
        match outcome {
            TickOutcome::Serve(verdict) => {
                tracing::debug!(
                    dedup = if waited { "follower" } else { "hit" },
                    fingerprint = %prepared.fingerprint,
                    "hook eval dedup"
                );
                return verdict;
            }
            TickOutcome::Acquired(token) => {
                // leader: 評価 → verdict publish(atomic_write 完了) →
                // delete-if-owner で lock release。「lock 無し・verdict
                // 未 commit」の窓を消す publish 順序。record write 失敗時
                // も eval_fn は再実行せず、その verdict をそのまま返す
                // (評価後の失敗に「直接評価へ畳む」規則は適用しない)。
                tracing::debug!(
                    dedup = "leader",
                    fingerprint = %prepared.fingerprint,
                    "hook eval dedup"
                );
                let verdict = eval_fn().await;
                let body = verdict_record_body(&prepared.fingerprint, &verdict);
                let store_dir = prepared.store_dir.clone();
                let fp = prepared.fingerprint.clone();
                let _ = tokio::task::spawn_blocking(move || {
                    publish_and_release(&store_dir, &fp, body.as_deref(), &token)
                })
                .await;
                return verdict;
            }
            TickOutcome::Wait => {
                waited = true;
                let now = tokio::time::Instant::now();
                if now >= deadline {
                    // follower_wait deadline → 自前評価(cache write し
                    // ない — publish 経路を leader 単独に保つ)。
                    tracing::debug!(
                        dedup = "fallback",
                        fingerprint = %prepared.fingerprint,
                        "hook eval dedup"
                    );
                    return eval_fn().await;
                }
                tokio::time::sleep(
                    limits
                        .poll_interval
                        .min(deadline.saturating_duration_since(now)),
                )
                .await;
            }
            TickOutcome::Fallback => {
                tracing::debug!(
                    dedup = "fallback",
                    fingerprint = %prepared.fingerprint,
                    "hook eval dedup"
                );
                return eval_fn().await;
            }
        }
    }
}

/// request variant 由来の hash 入力へ写す。enum discriminant が kind
/// を識別する。raw `path`/`cwd`・`target`・`source_tool` は含めない
/// — path の同一性は canonicalize 済み resolved path、cwd の同一性は
/// `workspace_root` でカバーし、spelling 差による fingerprint の分裂を
/// 防ぐ。`Shell.command` は raw 文字列で残す(deny reason が
/// `collapse_command(command)` を埋め込むため文字列差は verdict 差)。
#[derive(Hash)]
enum VariantInputs {
    FileRead {
        range: Option<(u64, Option<u64>)>,
    },
    Search {
        pattern: String,
        max_results: Option<u64>,
        context: u64,
        glob: Option<String>,
        /// `"content"`/`"files_with_matches"`/`"count"`/`"unknown"`。
        output_mode: &'static str,
        case_insensitive: bool,
    },
    Shell {
        command: String,
    },
}

/// target stat signature — `(resolved, kind, mtime, len)` の enum 表現
/// (discriminant が `"file"`/`"dir"`/`"absent"` marker)。
#[derive(Hash)]
enum TargetSig {
    File {
        resolved: String,
        mtime_nanos: u128,
        len: u64,
    },
    /// dir の len は platform 依存で意味を持たないため除外。
    Dir { resolved: String, mtime_nanos: u128 },
    /// `NonFile`・metadata 失敗は absent marker へ畳む(fs race は
    /// fingerprint 差異として自然に再評価へ倒れる)。
    Absent { raw: String },
}

/// policy 層 signature — `policy.yaml` の `(mtime_nanos, len)` stat
/// または marker。
#[derive(Hash)]
enum PolicySig {
    File {
        mtime_nanos: u128,
        len: u64,
    },
    Absent,
    /// builtin_dir `None`(embedded 解決)は内容が binary 固定のため
    /// `embedded_fingerprint()` の値を marker として hash する。
    Embedded(String),
    /// `workspace_root` 未解決時の project 層。
    Unresolved,
}

/// fingerprint の hash 入力 — field 単位の明示列挙。`RoutingRequest`
/// への `derive(Hash)` は使わない(`source_tool` 混入で firing 間に
/// key が割れる)。session・pid・config-dir 系 env・hook event 名も
/// 含めない(store root 分岐で構造的分離済み・verdict 非入力)。
#[derive(Hash)]
struct FingerprintInput {
    tag: &'static str,
    /// canonicalize 済み実効 cwd(`None` = unresolved marker)。
    workspace_root: Option<String>,
    variant: VariantInputs,
    /// `ctx.targets` 列挙順の stat signature。
    targets: Vec<TargetSig>,
    /// `threshold()` の `f64::to_bits()`。
    threshold_bits: u64,
    /// `count_max_bytes()` の実効値(default 畳み込み済み)。
    count_max_bytes: u64,
    builtin_policy: PolicySig,
    user_policy: PolicySig,
    project_policy: PolicySig,
}

/// verdict record(`<fp>.json`)の read 側 wire schema。`createdAt` は
/// 観察用 field で freshness 判定には使わない — TTL は atomic rename
/// 時に fs が付与する file mtime の checked-age で判定するため read
/// struct には含めない(未知 field は serde が無視する forward-compat
/// 規約、`usage.jsonl` と同じ)。
#[derive(serde::Deserialize)]
struct VerdictRecord {
    #[serde(rename = "schemaVersion")]
    schema_version: u32,
    fingerprint: String,
    verdict: Verdict,
}

/// lock file(`<fp>.lock`)の owner token payload — `(pid, createdAt,
/// nonce)` triple が lock owner を一意に識別する。delete-if-owner は
/// remove 直前に payload を読み直してこの token を照合する。
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
struct LockToken {
    pid: u32,
    #[serde(rename = "createdAt")]
    created_at: u64,
    nonce: u64,
}

/// nonce 生成の process 内 counter(epoch nanos と合成)。
static LOCK_NONCE: AtomicU64 = AtomicU64::new(0);

fn epoch_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

fn new_lock_token() -> LockToken {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0);
    LockToken {
        pid: std::process::id(),
        created_at: epoch_millis(),
        nonce: nanos.wrapping_add(LOCK_NONCE.fetch_add(1, Ordering::Relaxed)),
    }
}

fn variant_inputs(request: &RoutingRequest) -> VariantInputs {
    match request {
        RoutingRequest::FileRead { range, .. } => VariantInputs::FileRead {
            range: range.map(|r| (r.offset, r.limit)),
        },
        RoutingRequest::Search {
            pattern,
            max_results,
            context,
            glob,
            output_mode,
            case_insensitive,
            ..
        } => VariantInputs::Search {
            pattern: pattern.clone(),
            max_results: *max_results,
            context: *context,
            glob: glob.clone(),
            output_mode: match output_mode {
                SearchOutputMode::Content => "content",
                SearchOutputMode::FilesWithMatches => "files_with_matches",
                SearchOutputMode::Count => "count",
                SearchOutputMode::Unknown => "unknown",
            },
            case_insensitive: *case_insensitive,
        },
        RoutingRequest::Shell { command, .. } => VariantInputs::Shell {
            command: command.clone(),
        },
    }
}

/// `metadata.modified()` の nanos(0 未満の epoch 差分は metadata
/// 失敗と同じく `None` へ畳む)+ `len`。
fn stat_signature(path: &Path) -> Option<(u128, u64)> {
    let md = std::fs::metadata(path).ok()?;
    let nanos = md
        .modified()
        .ok()?
        .duration_since(UNIX_EPOCH)
        .ok()?
        .as_nanos();
    Some((nanos, md.len()))
}

fn target_signature(target: &ResolvedTarget) -> TargetSig {
    if target.kind == ResolvedKind::NonFile {
        return TargetSig::Absent {
            raw: target.resolved.clone(),
        };
    }
    match stat_signature(Path::new(&target.resolved)) {
        Some((mtime_nanos, len)) => match target.kind {
            ResolvedKind::File => TargetSig::File {
                resolved: target.resolved.clone(),
                mtime_nanos,
                len,
            },
            ResolvedKind::Dir | ResolvedKind::NonFile => TargetSig::Dir {
                resolved: target.resolved.clone(),
                mtime_nanos,
            },
        },
        None => TargetSig::Absent {
            raw: target.resolved.clone(),
        },
    }
}

fn policy_file_sig(path: PathBuf) -> PolicySig {
    match stat_signature(&path) {
        Some((mtime_nanos, len)) => PolicySig::File { mtime_nanos, len },
        None => PolicySig::Absent,
    }
}

fn compute_fingerprint(input: &FingerprintInput) -> String {
    let mut hasher = DefaultHasher::new();
    input.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

/// phase A の成果物。
struct Prepared {
    store_dir: PathBuf,
    fingerprint: String,
    /// optional fast-path gate — 全 target が `len <= threshold` の
    /// File または全て NonFile のとき協調を skip する。
    fast_path: bool,
}

/// blocking phase A — cache dir 確保 → best-effort sweep → policy/env
/// 入力確定 → target stat signature → policy 層 signature →
/// fingerprint 確定。`create_dir_all` 失敗のみ `None`(直接評価へ畳む)。
fn prepare_dedup(
    ctx: FingerprintContext,
    variant: VariantInputs,
    env_map: HashMap<String, String>,
    builtin_dir: Option<PathBuf>,
    store_dir: PathBuf,
    limits: DedupLimits,
) -> Option<Prepared> {
    std::fs::create_dir_all(&store_dir).ok()?;
    sweep_store(&store_dir, &limits);
    let env_lookup = |name: &str| env_map.get(name).cloned();
    let limit = threshold(&env_lookup);
    let count_cap = count_max_bytes(&env_lookup).unwrap_or(DEFAULT_COUNT_MAX_BYTES);
    let mut target_sigs = Vec::with_capacity(ctx.targets.len());
    let mut all_file_within = true;
    let mut all_nonfile = true;
    for target in &ctx.targets {
        let sig = target_signature(target);
        all_file_within &= matches!(&sig, TargetSig::File { len, .. } if (*len as f64) <= limit);
        all_nonfile &= target.kind == ResolvedKind::NonFile;
        target_sigs.push(sig);
    }
    let builtin_policy = match &builtin_dir {
        Some(dir) => policy_file_sig(dir.join("policy.yaml")),
        None => PolicySig::Embedded(crate::embedded::embedded_fingerprint()),
    };
    let user_policy = store_dir
        .parent()
        .map(|dir| policy_file_sig(dir.join("policy.yaml")))
        .unwrap_or(PolicySig::Absent);
    let project_policy = match &ctx.workspace_root {
        Some(root) => policy_file_sig(Path::new(root).join(".cheaper").join("policy.yaml")),
        None => PolicySig::Unresolved,
    };
    let input = FingerprintInput {
        tag: FINGERPRINT_TAG,
        workspace_root: ctx.workspace_root,
        variant,
        targets: target_sigs,
        threshold_bits: limit.to_bits(),
        count_max_bytes: count_cap,
        builtin_policy,
        user_policy,
        project_policy,
    };
    Some(Prepared {
        store_dir,
        fingerprint: compute_fingerprint(&input),
        fast_path: all_file_within || all_nonfile,
    })
}

/// `<fp>.json` を checked-age 規則で読む — `schemaVersion == 1`・
/// `fingerprint` field 一致・`verdict` deserialize 成功・file mtime が
/// `Some(age) && age <= verdict_ttl` を全て満たす record のみ serve。
/// 不存在・parse 失敗・未知 version・fingerprint 不一致・TTL 超過・
/// 未来 mtime(`None` — clock skew・fs timestamp 異常)は全て absent。
fn read_fresh_verdict(store_dir: &Path, fp: &str, ttl: Duration) -> Option<Verdict> {
    let path = store_dir.join(format!("{fp}.json"));
    let md = std::fs::metadata(&path).ok()?;
    if !md.is_file() {
        return None;
    }
    let age = SystemTime::now().duration_since(md.modified().ok()?).ok()?;
    if age > ttl {
        return None;
    }
    let record: VerdictRecord = serde_json::from_str(&std::fs::read_to_string(&path).ok()?).ok()?;
    if record.schema_version != 1 || record.fingerprint != fp {
        return None;
    }
    Some(record.verdict)
}

/// `{"schemaVersion":1,"createdAt":<epoch ms>,"fingerprint":<16hex>,
/// "verdict":<Verdict serde>}` — `createdAt` は観察用 field
/// (freshness 判定には使わない)。
fn verdict_record_body(fp: &str, verdict: &Verdict) -> Option<String> {
    let mut map = serde_json::Map::new();
    map.insert("schemaVersion".to_string(), serde_json::Value::from(1));
    map.insert(
        "createdAt".to_string(),
        serde_json::Value::from(epoch_millis()),
    );
    map.insert("fingerprint".to_string(), serde_json::Value::from(fp));
    map.insert("verdict".to_string(), serde_json::to_value(verdict).ok()?);
    serde_json::to_string(&serde_json::Value::Object(map)).ok()
}

/// lock payload の read — parse 不能は `None`(ownership を証明でき
/// ないため削除しない。mtime sweep でのみ期限切れ除去される)。
fn read_lock_token(lock_path: &Path) -> Option<LockToken> {
    serde_json::from_str(&std::fs::read_to_string(lock_path).ok()?).ok()
}

/// delete-if-owner — remove 直前に payload を読み直し、観測した token
/// と一致する場合に限り削除する。stale 判定 → takeover で後継 leader
/// が lock を再取得した後に旧 leader が後継の lock を削除する経路を
/// 防ぐ。payload が parse 不能な lock は一切削除しない(保守的)。
fn remove_if_owner(lock_path: &Path, owner: &LockToken) {
    if read_lock_token(lock_path).as_ref() == Some(owner) {
        let _ = std::fs::remove_file(lock_path);
    }
}

/// lock file の checked-age 分類。
enum LockState {
    /// 不存在・metadata 取得失敗(poll 中の fs error は継続扱い)。
    Absent,
    /// `age <= lock_stale` の live lock。
    Fresh,
    /// `age > lock_stale` または checked-age `None`(未来 mtime —
    /// clock skew・fs timestamp 異常は stale 扱い)。`token` は観測した
    /// owner token(payload parse 不能なら `None` → 削除不可)。
    Stale { token: Option<LockToken> },
}

fn lock_state(lock_path: &Path, stale_after: Duration) -> LockState {
    let Ok(md) = std::fs::metadata(lock_path) else {
        return LockState::Absent;
    };
    let age = md
        .modified()
        .ok()
        .and_then(|mtime| SystemTime::now().duration_since(mtime).ok());
    match age {
        Some(age) if age <= stale_after => LockState::Fresh,
        _ => LockState::Stale {
            token: read_lock_token(lock_path),
        },
    }
}

/// `<fp>.lock` を `create_new`(O_EXCL 相当の atomic create)で取得し、
/// owner token JSON を書き込む。token 書込みに失敗した lock は owner
/// を証明できないため取得失敗として畳み、file は sweep が回収する
/// (unparseable payload は delete-if-owner で消せないため自前 remove
/// もしない)。
fn acquire_lock(lock_path: &Path) -> io::Result<LockToken> {
    use std::io::Write;
    let token = new_lock_token();
    let body =
        serde_json::to_string(&token).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(lock_path)?;
    file.write_all(body.as_bytes())?;
    Ok(token)
}

fn acquire_or_wait(lock_path: &Path) -> TickOutcome {
    match acquire_lock(lock_path) {
        Ok(token) => TickOutcome::Acquired(token),
        Err(e) if e.kind() == io::ErrorKind::AlreadyExists => TickOutcome::Wait,
        // 非 AlreadyExists の create_new error は直接評価へ畳む。
        Err(_) => TickOutcome::Fallback,
    }
}

/// 1 poll tick 分の fs 操作(`spawn_blocking` から呼ばれる)。
enum TickOutcome {
    /// fresh+valid な record を serve。
    Serve(Verdict),
    /// lock を取得した(新規・takeover 問わず)— caller は leader。
    Acquired(LockToken),
    /// 協調継続中 — `poll_interval` sleep して次 tick。
    Wait,
    /// 協調不能 — caller は `eval_fn` 直接呼出しへ畳む。
    Fallback,
}

/// follower poll tick — (1) record fresh+valid → serve、(2) lock 不在
/// → `create_new` retry、(3) lock stale(checked-age `> lock_stale`
/// または `None`)→ delete-if-owner remove + retry、(4) lock fresh →
/// 次 tick。poll 中の fs error は「verdict 未着」として継続する。
fn poll_tick(store_dir: &Path, fp: &str, limits: &DedupLimits) -> TickOutcome {
    if let Some(verdict) = read_fresh_verdict(store_dir, fp, limits.verdict_ttl) {
        return TickOutcome::Serve(verdict);
    }
    let lock_path = store_dir.join(format!("{fp}.lock"));
    match lock_state(&lock_path, limits.lock_stale) {
        LockState::Fresh => TickOutcome::Wait,
        LockState::Stale { token } => {
            if let Some(token) = token {
                remove_if_owner(&lock_path, &token);
            }
            // remove 不能(unparseable payload・token 交替済み)なら
            // `create_new` は AlreadyExists → Wait で期限切れを待つ。
            acquire_or_wait(&lock_path)
        }
        LockState::Absent => acquire_or_wait(&lock_path),
    }
}

/// leader の publish — verdict `atomic_write` 完了 → delete-if-owner
/// で lock release(「lock 無し・verdict 未 commit」の窓を消す順序)。
/// record write 失敗は共有できないだけなので lock は外して verdict を
/// そのまま返させる。
fn publish_and_release(store_dir: &Path, fp: &str, body: Option<&str>, token: &LockToken) {
    if let Some(body) = body {
        let _ = atomic_write(
            &store_dir.join(format!("{fp}.json")),
            body,
            &AtomicWriteOptions {
                windows_remove_first: false,
                #[cfg(unix)]
                unix_mode: Some(0o600),
            },
        );
    }
    remove_if_owner(&store_dir.join(format!("{fp}.lock")), token);
}

/// dedup 経路先頭(blocking phase)で `hook-eval/` を best-effort sweep
/// — `mtime > verdict_ttl` の `*.json`、`mtime > lock_stale` の
/// `*.lock`、TTL 超過の `.<fp>.json.tmp-*` orphan を remove する。
/// checked-age `None`(未来 mtime)も stale として remove 対象に含める。
/// 失敗は無視(無制限蓄積を防ぐ起点であり、正確性には関与しない)。
fn sweep_store(store_dir: &Path, limits: &DedupLimits) {
    let Ok(entries) = std::fs::read_dir(store_dir) else {
        return;
    };
    let now = SystemTime::now();
    for entry in entries.flatten() {
        let name = entry.file_name();
        let Some(name) = name.to_str() else {
            continue;
        };
        let stale_after = if name.ends_with(".json") {
            limits.verdict_ttl
        } else if name.ends_with(".lock") {
            limits.lock_stale
        } else if name.starts_with('.') && name.contains(".json.tmp-") {
            // atomic_write の orphan temp(`temp_path` の命名規則)。
            limits.verdict_ttl
        } else {
            continue;
        };
        let Ok(md) = entry.metadata() else {
            continue;
        };
        if !md.is_file() {
            continue;
        }
        let age = md
            .modified()
            .ok()
            .and_then(|mtime| now.duration_since(mtime).ok());
        if age.is_none_or(|age| age > stale_after) {
            let _ = std::fs::remove_file(entry.path());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    fn token(nonce: u64) -> LockToken {
        LockToken {
            pid: 42,
            created_at: 1_700_000_000_000,
            nonce,
        }
    }

    fn set_mtime(path: &Path, time: SystemTime) -> io::Result<()> {
        std::fs::File::options()
            .write(true)
            .open(path)?
            .set_modified(time)
    }

    /// delete-if-owner primitive — 観測 token が一致する lock のみ削除
    /// する(live leader 誤判定・stale takeover 後に旧 leader が後継
    /// lock を消せないことを primitive 層で固定)。
    #[test]
    fn remove_if_owner_deletes_only_matching_token() -> TestResult {
        let dir = tempfile::tempdir()?;
        let lock = dir.path().join("abc.lock");
        std::fs::write(&lock, serde_json::to_string(&token(1))?)?;
        remove_if_owner(&lock, &token(2));
        assert!(lock.exists(), "token 不一致の lock は残る");
        remove_if_owner(&lock, &token(1));
        assert!(!lock.exists(), "owner token 一致で削除");
        Ok(())
    }

    /// payload が parse 不能な lock は ownership を証明できないため
    /// 一切削除しない(mtime sweep のみで期限切れ除去)。
    #[test]
    fn remove_if_owner_never_deletes_unparseable_lock() -> TestResult {
        let dir = tempfile::tempdir()?;
        let lock = dir.path().join("abc.lock");
        for body in ["not json", "", "{\"pid\":1}"] {
            std::fs::write(&lock, body)?;
            remove_if_owner(&lock, &token(1));
            assert!(lock.exists(), "unparseable lock must survive: {body:?}");
            std::fs::remove_file(&lock)?;
        }
        Ok(())
    }

    /// checked-age 規則 — TTL 超過・未来 mtime(`checked_duration_since`
    /// → `None`)の record は stale 扱いで serve しない。非 JSON・未知
    /// `schemaVersion`・`fingerprint` 不一致も absent。
    #[test]
    fn read_fresh_verdict_checked_age_rules() -> TestResult {
        let dir = tempfile::tempdir()?;
        let store = dir.path();
        let fp = "0123456789abcdef";
        let record = store.join(format!("{fp}.json"));
        std::fs::write(
            &record,
            "{\"schemaVersion\":1,\"createdAt\":0,\"fingerprint\":\"0123456789abcdef\",\"verdict\":{\"decision\":\"deny\",\"reason\":\"r\"}}",
        )?;
        let ttl = Duration::from_millis(60_000);
        match read_fresh_verdict(store, fp, ttl) {
            Some(Verdict::Deny { reason }) => assert_eq!(reason, "r"),
            other => return Err(format!("want deny record, got {other:?}").into()),
        }
        // TTL 超過の backdated mtime → absent。
        set_mtime(&record, SystemTime::now() - Duration::from_secs(3600))?;
        assert!(read_fresh_verdict(store, fp, ttl).is_none());
        // 未来 mtime(checked-age None)→ absent。
        set_mtime(&record, SystemTime::now() + Duration::from_secs(3600))?;
        assert!(read_fresh_verdict(store, fp, ttl).is_none());
        // fresh mtime へ戻す — fingerprint 不一致は absent。
        set_mtime(&record, SystemTime::now())?;
        assert!(read_fresh_verdict(store, "ffffffffffffffff", ttl).is_none());
        // 未知 schemaVersion・非 JSON は absent。
        std::fs::write(
            &record,
            "{\"schemaVersion\":2,\"fingerprint\":\"0123456789abcdef\",\"verdict\":{\"decision\":\"allow\"}}",
        )?;
        assert!(read_fresh_verdict(store, fp, ttl).is_none());
        std::fs::write(&record, "not json")?;
        assert!(read_fresh_verdict(store, fp, ttl).is_none());
        Ok(())
    }

    /// lock の checked-age 分類 — 不在 → Absent、fresh → Fresh、
    /// `lock_stale` 超過・未来 mtime → Stale(owner token 観測付き、
    /// unparseable payload は token 無し)。
    #[test]
    fn lock_state_checked_age_rules() -> TestResult {
        let dir = tempfile::tempdir()?;
        let lock = dir.path().join("abc.lock");
        let stale = Duration::from_millis(120_000);
        assert!(matches!(lock_state(&lock, stale), LockState::Absent));
        std::fs::write(&lock, serde_json::to_string(&token(7))?)?;
        assert!(matches!(lock_state(&lock, stale), LockState::Fresh));
        set_mtime(&lock, SystemTime::now() - Duration::from_secs(3600))?;
        match lock_state(&lock, stale) {
            LockState::Stale { token: Some(t) } => assert_eq!(t, token(7)),
            _ => return Err("want stale lock with owner token".into()),
        }
        set_mtime(&lock, SystemTime::now() + Duration::from_secs(3600))?;
        assert!(matches!(lock_state(&lock, stale), LockState::Stale { .. }));
        // unparseable payload + stale mtime → token 無しの Stale
        // (delete-if-owner の照合材料を持たず削除されない)。
        std::fs::write(&lock, "garbage")?;
        set_mtime(&lock, SystemTime::now() - Duration::from_secs(3600))?;
        assert!(matches!(
            lock_state(&lock, stale),
            LockState::Stale { token: None }
        ));
        Ok(())
    }

    /// sweep — TTL 超過の record・stale lock・orphan tmp を remove し、
    /// fresh entry は残す(未来 mtime も stale 扱いで除去)。
    #[test]
    fn sweep_removes_only_stale_entries() -> TestResult {
        let dir = tempfile::tempdir()?;
        let store = dir.path();
        let limits = DedupLimits {
            verdict_ttl: Duration::from_millis(1_000),
            lock_stale: Duration::from_millis(2_000),
            follower_wait: Duration::from_millis(100),
            poll_interval: Duration::from_millis(10),
        };
        let fresh_record = store.join("a.json");
        let stale_record = store.join("b.json");
        let fresh_lock = store.join("a.lock");
        let stale_lock = store.join("b.lock");
        let orphan_tmp = store.join(".a.json.tmp-1-1");
        let future_record = store.join("c.json");
        let unrelated = store.join("keep.txt");
        for path in [
            &fresh_record,
            &stale_record,
            &fresh_lock,
            &stale_lock,
            &orphan_tmp,
            &future_record,
            &unrelated,
        ] {
            std::fs::write(path, "x")?;
        }
        let old = SystemTime::now() - Duration::from_secs(3600);
        let future = SystemTime::now() + Duration::from_secs(3600);
        set_mtime(&stale_record, old)?;
        set_mtime(&stale_lock, old)?;
        set_mtime(&orphan_tmp, old)?;
        set_mtime(&future_record, future)?;
        sweep_store(store, &limits);
        assert!(fresh_record.exists());
        assert!(fresh_lock.exists());
        assert!(unrelated.exists());
        assert!(!stale_record.exists());
        assert!(!stale_lock.exists());
        assert!(!orphan_tmp.exists());
        assert!(!future_record.exists(), "future mtime is stale");
        Ok(())
    }

    /// fingerprint — `DefaultHasher` → 16hex、同一入力で決定的、
    /// 観測可能入力の差で別 fingerprint になる。
    #[test]
    fn fingerprint_is_deterministic_and_input_sensitive() {
        let input = |threshold_bits: u64| FingerprintInput {
            tag: FINGERPRINT_TAG,
            workspace_root: Some("/ws".to_string()),
            variant: VariantInputs::FileRead {
                range: Some((2, Some(10))),
            },
            targets: vec![TargetSig::File {
                resolved: "/ws/big.txt".to_string(),
                mtime_nanos: 42,
                len: 100,
            }],
            threshold_bits,
            count_max_bytes: DEFAULT_COUNT_MAX_BYTES,
            builtin_policy: PolicySig::Embedded("deadbeefdeadbeef".to_string()),
            user_policy: PolicySig::Absent,
            project_policy: PolicySig::Unresolved,
        };
        let base = compute_fingerprint(&input(8192.0f64.to_bits()));
        assert_eq!(base, compute_fingerprint(&input(8192.0f64.to_bits())));
        assert_eq!(base.len(), 16);
        assert!(base.chars().all(|c| c.is_ascii_hexdigit()));
        assert_ne!(base, compute_fingerprint(&input(1024.0f64.to_bits())));
    }
}
