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
//! <fp>.json                      verdict record(atomic temp+rename)
//! <fp>.lock                      leader election marker(owner token JSON)
//! <fp>.lock.claim-<pid>-<nonce>  stale takeover の rename-claim 残骸
//! .<fp>.json.tmp-<pid>-<nanos>   atomic_write temp(orphan sweep 対象)
//! ```
//!
//! 全失敗は `eval_fn` 直接呼出し(通常評価)へ畳まれる fail-open 設計 —
//! 本 module は panic 経路を持たない。協調が触れる全ての blocking fs
//! I/O は `spawn_blocking` へ逃がし `DEDUP_IO_BOUND`(`MEASURE_TIMEOUT`
//! と同値)で囲む — hang した fs(FIFO・dead network FS)でも verdict
//! が bounded time で resolve する(`measure_tokens` と同じ規則)。

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

/// 協調の blocking fs I/O(`spawn_blocking` の await)に掛ける上界 —
/// `MEASURE_TIMEOUT` と同値(DRY)。`spawn_blocking` から timeout で
/// 切り離された task は cancel できず thread は継続する。
/// `measure_tokens` と異なり、abandon された `poll_tick` thread は
/// acquire 済みの fresh valid-token `<fp>.lock` を残し得る — 以降の
/// 同一 fp 発火は lock が `lock_stale` を越えて sweep・stale 判定で
/// 除去されるまで `follower_wait` を支払い自前評価へ倒れる。
/// bounded・自己治癒・hang した fs の病理経路のみの受容済み妥協。
const DEDUP_IO_BOUND: Duration = MEASURE_TIMEOUT;

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
    /// deadline で自前評価へ移る。dedup が追加する latency の真の上界
    /// ではない — worst case は `DEDUP_IO_BOUND`(prepare)+
    /// `follower_wait` + in-flight tick の slack で ~60s 超に達し得る
    /// (hang した fs の病理経路のみ。通常経路では `follower_wait` +
    /// `poll_interval` が実効 bound)。
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
    if !store_dir.is_absolute() {
        // 相対 store root(相対 `CHEAPER_CONFIG_DIR` 等の env 由来)は
        // process cwd 依存の解釈になり file 名衝突・意図しない書き込み
        // 先を招くため協調を bypass する(fail-open)。
        tracing::debug!(dedup = "bypass", "hook eval dedup");
        return eval_fn().await;
    }
    let builtin_dir = opts.builtin_dir.cloned();
    let variant = variant_inputs(request);
    let request = request.clone();
    let limits = *limits;

    // phase A(`DEDUP_IO_BOUND` で囲んだ blocking): fingerprint 対象
    // 列挙(`resolve_target` 相当の sync metadata・canonicalize を含む
    // — hang した fs でも timeout で畳めるよう async context には出さ
    // ない)→ cache dir 確保 → sweep → policy/env 入力確定 → stat
    // signature → policy 層 signature → fingerprint 確定。
    // targets が完全に空なら定数 allow 経路なので cache を介さず直接
    // 評価する(共有する意味も signature も無い)。
    let prepared = match tokio::time::timeout(
        DEDUP_IO_BOUND,
        tokio::task::spawn_blocking(move || {
            let ctx = fingerprint_targets(&request);
            if ctx.targets.is_empty() {
                return PrepareOutcome::Bypass;
            }
            match prepare_dedup(ctx, variant, env_map, builtin_dir, store_dir, limits) {
                Some(prepared) => PrepareOutcome::Ready(prepared),
                None => PrepareOutcome::Fallback,
            }
        }),
    )
    .await
    {
        Ok(Ok(PrepareOutcome::Ready(prepared))) => prepared,
        Ok(Ok(PrepareOutcome::Bypass)) => {
            tracing::debug!(dedup = "bypass", "hook eval dedup");
            return eval_fn().await;
        }
        // create_dir_all 失敗・JoinError・`DEDUP_IO_BOUND` 超過 —
        // 全て直接評価へ畳む。
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
    // read → lock `create_new` retry)を `spawn_blocking` へ逃がし
    // `DEDUP_IO_BOUND` で囲む。`follower_wait` deadline 到達で自前評価
    // へ移る。
    let deadline = tokio::time::Instant::now() + limits.follower_wait;
    let mut waited = false;
    loop {
        let store_dir = prepared.store_dir.clone();
        let fp = prepared.fingerprint.clone();
        let outcome = match tokio::time::timeout(
            DEDUP_IO_BOUND,
            tokio::task::spawn_blocking(move || poll_tick(&store_dir, &fp, &limits)),
        )
        .await
        {
            Ok(Ok(outcome)) => outcome,
            // JoinError(panic/cancel)・`DEDUP_IO_BOUND` 超過も fail-open。
            _ => TickOutcome::Fallback,
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
                // publish も `DEDUP_IO_BOUND` で囲む — timeout・JoinError
                // 時は共有を諦め、計算済み verdict をそのまま返す。
                // abandon された blocking thread は atomic_write・
                // release を継続するが、`measure_tokens` と異なり観測
                // 可能な残留を残し得る: release 完了まで fresh
                // valid-token `<fp>.lock` が残り、同一 fp 発火は
                // `follower_wait` を支払い自前評価へ倒れる
                // (`lock_stale`・sweep までの bounded・自己治癒の
                // 病理経路のみ)。
                let _ = tokio::time::timeout(
                    DEDUP_IO_BOUND,
                    tokio::task::spawn_blocking(move || {
                        publish_and_release(&store_dir, &fp, body.as_deref(), &token)
                    }),
                )
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

/// target stat signature — `(resolved, rel_posix, kind, mtime, len)` の
/// enum 表現(discriminant が `"file"`/`"dir"`/`"absent"` marker)。
/// `resolved`/`raw` は `OsStr::as_encoded_bytes` の生 bytes — lossy
/// 変換で非 UTF-8 path を潰さない。`rel_posix` は deny reason が
/// 埋め込む workspace-relative path で verdict 入力の一部。
#[derive(Hash)]
enum TargetSig {
    File {
        resolved: Vec<u8>,
        rel_posix: String,
        mtime_nanos: u128,
        len: u64,
    },
    /// dir の len は platform 依存で意味を持たないため除外。
    Dir {
        resolved: Vec<u8>,
        rel_posix: String,
        mtime_nanos: u128,
    },
    /// `NonFile`・metadata 失敗は absent marker へ畳む(fs race は
    /// fingerprint 差異として自然に再評価へ倒れる)。
    Absent { raw: Vec<u8> },
}

/// policy 層 signature — `policy.yaml` の `(mtime_nanos, len)` stat
/// または marker。
#[derive(Hash)]
enum PolicySig {
    File {
        mtime_nanos: u128,
        len: u64,
    },
    Contents(u64),
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
    /// `routes.yaml` 3 層(builtin/user/project)の stat signature —
    /// Shell variant のみ立てる(route 宣言は shell eval のみの
    /// verdict 入力。非 shell request は空 vec で、routes 変更が
    /// File/Search の fingerprint を不必要に割らない)。route
    /// declaration の変更で stale shell verdict が serve されない
    /// ことを保証する invalidation 入力(C3・design §dedup)。
    routes_sig: Vec<PolicySig>,
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

/// `resolved` の `OsStr::as_encoded_bytes` — 生 bytes で hash し、
/// lossy 変換による非 UTF-8 path の衝突を防ぐ。
fn resolved_bytes(path: &Path) -> Vec<u8> {
    path.as_os_str().as_encoded_bytes().to_vec()
}

fn target_signature(target: &ResolvedTarget) -> TargetSig {
    if target.kind == ResolvedKind::NonFile {
        return TargetSig::Absent {
            raw: resolved_bytes(&target.resolved),
        };
    }
    match stat_signature(&target.resolved) {
        Some((mtime_nanos, len)) => match target.kind {
            ResolvedKind::File => TargetSig::File {
                resolved: resolved_bytes(&target.resolved),
                rel_posix: target.rel_posix.clone(),
                mtime_nanos,
                len,
            },
            ResolvedKind::Dir | ResolvedKind::NonFile => TargetSig::Dir {
                resolved: resolved_bytes(&target.resolved),
                rel_posix: target.rel_posix.clone(),
                mtime_nanos,
            },
        },
        None => TargetSig::Absent {
            raw: resolved_bytes(&target.resolved),
        },
    }
}

fn policy_file_sig(path: PathBuf) -> PolicySig {
    match stat_signature(&path) {
        Some((mtime_nanos, len)) => PolicySig::File { mtime_nanos, len },
        None => PolicySig::Absent,
    }
}

fn route_file_sig(path: PathBuf) -> Option<PolicySig> {
    let mut file = match std::fs::File::open(&path) {
        Ok(file) => file,
        Err(e) if cheaper_core::errors::is_absent_file_error(&e) => return Some(PolicySig::Absent),
        Err(_) => return None,
    };
    let mut hasher = DefaultHasher::new();
    let mut buf = [0u8; 8192];
    loop {
        match std::io::Read::read(&mut file, &mut buf) {
            Ok(0) => return Some(PolicySig::Contents(hasher.finish())),
            Ok(n) => std::hash::Hasher::write(&mut hasher, &buf[..n]),
            Err(_) => return None,
        }
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

/// phase A(blocking)の結果分類。
enum PrepareOutcome {
    /// fingerprint 対象が空 — 定数 allow 経路で協調不要。
    Bypass,
    /// fingerprint 確定・協調へ進める。
    Ready(Prepared),
    /// 協調不能(`create_dir_all` 失敗等)— 直接評価へ畳む。
    Fallback,
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
    // routes 3 層 signature — Shell variant のみ(design §dedup の
    // route fingerprint)。層 path は `route::routes::load` と同じ
    // 解決規約(builtin_dir/routes.yaml・userConfigDir/routes.yaml・
    // <root>/.cheaper/routes.yaml)。builtin は embedded marker と
    // file sig を policy と同じ分岐で選ぶ。
    let routes_sig = if matches!(variant, VariantInputs::Shell { .. }) {
        let user_dir = store_dir.parent().map(|d| d.join("routes.yaml"));
        let project_routes = ctx
            .workspace_root
            .as_ref()
            .map(|root| Path::new(root).join(".cheaper").join("routes.yaml"));
        vec![
            match &builtin_dir {
                Some(dir) => route_file_sig(dir.join("routes.yaml"))?,
                None => PolicySig::Embedded(crate::embedded::embedded_fingerprint()),
            },
            match user_dir {
                Some(p) => route_file_sig(p)?,
                None => PolicySig::Absent,
            },
            match project_routes {
                Some(p) => route_file_sig(p)?,
                None => PolicySig::Unresolved,
            },
        ]
    } else {
        Vec::new()
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
        routes_sig,
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
    // shareable でない verdict は serve しない(防御的 gate —
    // publish 側が既に書き込まないが、手作業・version 差で混入した
    // record を後続 firing へ返さない)。
    is_shareable(&record.verdict).then_some(record.verdict)
}

/// dedup cache で persist/serve してよい verdict の gate。
/// `Verdict::Rewrite` は rewrite command の env 値を含み得るため
/// 共有対象外(C3・design §dedup) — env leak を防ぐ意味でも
/// allowlist 形の exhaustive match にし、variant 追加時に明示
/// 判断を強制する。
fn is_shareable(verdict: &Verdict) -> bool {
    match verdict {
        Verdict::Allow | Verdict::Deny { .. } => true,
        // rewrite command の payload は allowlist capture した env
        // 値を含む — cache record へ永続化・共有しない(R32)。
        Verdict::Rewrite { .. } => false,
    }
}

/// `{"schemaVersion":1,"createdAt":<epoch ms>,"fingerprint":<16hex>,
/// "verdict":<Verdict serde>}` — `createdAt` は観察用 field
/// (freshness 判定には使わない)。
fn verdict_record_body(fp: &str, verdict: &Verdict) -> Option<String> {
    if !is_shareable(verdict) {
        return None;
    }
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
/// と一致する場合に限り削除する。leader の release(remove 対象は自身
/// が作った lock)に使う。stale takeover 側は read→remove 間に旧
/// leader release + 新 leader acquire が挟まり得る TOCTOU を閉じる
/// rename-claim(`claim_stale_lock`)へ分離済み。
/// payload が parse 不能な lock は一切削除しない(保守的)。
///
/// 受容済み残存窓: `read`→`remove` の sub-µs gap に「自分の lock が
/// rename-claim で取り去られ後継 lock が再設置」されると後継の live
/// lock を消し得る。害は冗長評価のみで、観測も困難なため許容とする。
fn remove_if_owner(lock_path: &Path, owner: &LockToken) {
    if read_lock_token(lock_path).as_ref() == Some(owner) {
        let _ = std::fs::remove_file(lock_path);
    }
}

/// checked-age stale 判定 — 過去 mtime は `age > stale_after`、未来
/// mtime(`now.duration_since` が `Err`)は `now + stale_after` を
/// 超えるもののみ stale とみなす。skew 範囲内の未来 mtime・mtime
/// 取得不能は「期限切れを証明できない」ため `false`(削除しない側)。
fn past_stale_boundary(mtime: SystemTime, now: SystemTime, stale_after: Duration) -> bool {
    match now.duration_since(mtime) {
        Ok(age) => age > stale_after,
        // 未来 mtime — `stale_after` より先まで進んでいると自然 expire
        // しないため stale 扱い(takeover・sweep を許可)。
        Err(_) => mtime
            .duration_since(now)
            .map(|ahead| ahead > stale_after)
            .unwrap_or(true),
    }
}

/// lock file の checked-age 分類。
enum LockState {
    /// 不存在・metadata 取得失敗(poll 中の fs error は継続扱い)。
    Absent,
    /// live lock — `age <= lock_stale`、または skew 範囲内の未来
    /// mtime(`now + lock_stale` 以下 — 削除せず待機側へ倒す)、
    /// mtime 取得不能(期限切れを証明できない)。
    Fresh,
    /// `past_stale_boundary` を越えた死骸(過去 `age > lock_stale`
    /// または `now + lock_stale` を超える未来 mtime)。`token` は観測
    /// した owner token(payload parse 不能なら `None` → claim 対象外)。
    Stale { token: Option<LockToken> },
    /// file でない(dir・FIFO・socket 等)— `create_new` は永久に
    /// AlreadyExists で、FIFO への `read_to_string` は無期限に
    /// blocking し得るため payload read・削除の双方を行わない協調不能
    /// 分類(caller は即時 self-eval 経路)。
    Unrecoverable,
}

fn lock_state(lock_path: &Path, stale_after: Duration) -> LockState {
    let Ok(md) = std::fs::metadata(lock_path) else {
        return LockState::Absent;
    };
    if !md.is_file() {
        return LockState::Unrecoverable;
    }
    let now = SystemTime::now();
    let Ok(mtime) = md.modified() else {
        // mtime を取れない platform/entry では期限切れを証明できない —
        // 削除せず live lock として待機側へ倒す(follower_wait が bound)。
        return LockState::Fresh;
    };
    if past_stale_boundary(mtime, now, stale_after) {
        LockState::Stale {
            token: read_lock_token(lock_path),
        }
    } else {
        LockState::Fresh
    }
}

/// stale lock の takeover claim — `<fp>.lock` を
/// `<fp>.lock.claim-<pid>-<nonce>` へ `rename` する。rename は atomic
/// であり、成功した 1 者のみが claim file の owner token を観測値と
/// 照合して削除する — 削除対象が「観測した stale file そのもの」と
/// 証明できるため、`read`→`remove` 間に旧 leader の release と新
/// leader の acquire が挟まり得る `remove_if_owner` 直接適用の
/// TOCTOU を閉じる。
///
/// 戻り値は「stale file を除去した」か — 呼出し側は成否に関わらず
/// `create_new` retry へ進む(claim 成功 → 不在で取得可、他者が先に
/// acquire → AlreadyExists で Wait)。token 不一致・file でない claim
/// (rename に先行して新 lock / 非 file が差し替わった残存窓)は file
/// を殺さず `<fp>.lock` へ戻す(best-effort — 既に再占有されている
/// 場合は上書きを避けて orphan とし、sweep の `claim-*` 規則で回収)。
fn claim_stale_lock(lock_path: &Path, observed: &LockToken) -> bool {
    let Some(name) = lock_path.file_name().and_then(|n| n.to_str()) else {
        return false;
    };
    let claim_path = lock_path.with_file_name(format!(
        "{name}.claim-{}-{}",
        std::process::id(),
        LOCK_NONCE.fetch_add(1, Ordering::Relaxed)
    ));
    // rename 失敗(NotFound = 他者が先に削除/claim 済み等)は敗者。
    if std::fs::rename(lock_path, &claim_path).is_err() {
        return false;
    }
    // claim した object が観測した stale lock かを照合。file でない
    // (rename 先行で FIFO 等へ差し替わった残存窓)・token 不一致の
    // 場合は削除しない — claim path の名前は nonce 入りで他者に知られ
    // ないため、この照合の対象は rename 時点の file で確定している。
    let claimed_is_file = std::fs::metadata(&claim_path)
        .map(|md| md.is_file())
        .unwrap_or(false);
    if claimed_is_file && read_lock_token(&claim_path).as_ref() == Some(observed) {
        let _ = std::fs::remove_file(&claim_path);
        return true;
    }
    // 観測した file と別物 — live lock を殺さないよう元の名前へ戻す。
    // `exists` 確認後に rename する residual race(sub-µs)は受容
    // (remove_if_owner と同じ受容済み窓)。
    if !lock_path.exists() {
        let _ = std::fs::rename(&claim_path, lock_path);
    }
    false
}

/// `<fp>.lock` を `create_new`(O_EXCL 相当の atomic create)で取得し、
/// owner token JSON を書き込む。token 書込みに失敗した lock は owner
/// を証明できないため取得失敗として畳む — `create_new` 成功済みなら
/// その file は自プロセスの産物と確定しているので partial write の
/// 残骸は自前で remove する(他人の lock を消す経路ではないため
/// unparseable 非削除規則とは無関係)。
fn acquire_lock(lock_path: &Path) -> io::Result<LockToken> {
    use std::io::Write;
    let token = new_lock_token();
    let body =
        serde_json::to_string(&token).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(lock_path)?;
    if let Err(e) = file.write_all(body.as_bytes()) {
        let _ = std::fs::remove_file(lock_path);
        return Err(e);
    }
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
/// → `create_new` retry、(3) lock stale(`past_stale_boundary` 超過)
/// → rename-claim で stale file を atomic に除去 + retry、(4) lock
/// fresh → 次 tick、(5) lock が file でない → 協調不能。poll 中の
/// fs error は「verdict 未着」として継続する。
fn poll_tick(store_dir: &Path, fp: &str, limits: &DedupLimits) -> TickOutcome {
    if let Some(verdict) = read_fresh_verdict(store_dir, fp, limits.verdict_ttl) {
        return TickOutcome::Serve(verdict);
    }
    let lock_path = store_dir.join(format!("{fp}.lock"));
    match lock_state(&lock_path, limits.lock_stale) {
        LockState::Fresh => TickOutcome::Wait,
        LockState::Unrecoverable => TickOutcome::Fallback,
        LockState::Stale { token } => {
            // rename-claim は観測 token を持つ stale lock に限る —
            // unparseable payload は「観測した file との同一性」を照合
            // できないため保守的に削除しない(mtime sweep のみで期限切れ
            // 除去)。claim の成否に関わらず `create_new` retry は正規
            // 経路(他者が先に acquire 済みなら AlreadyExists → Wait)。
            if let Some(token) = token {
                claim_stale_lock(&lock_path, &token);
            }
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
/// — `past_stale_boundary(verdict_ttl)` を越えた `*.json`、
/// `past_stale_boundary(lock_stale)` を越えた `*.lock`・
/// `*.lock.claim-*` orphan、TTL 超過の `.<fp>.json.tmp-*` orphan を
/// remove する。checked-age `None`(未来 mtime)は `now + stale_after`
/// を超えるもののみ stale — skew 範囲内の未来 mtime の live lock を
/// 誤削除しない。失敗は無視(無制限蓄積を防ぐ起点であり、正確性に
/// は関与しない)。
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
        } else if name.ends_with(".lock") || name.contains(".lock.claim-") {
            // `<fp>.lock.claim-<pid>-<nonce>` は rename-claim の orphan
            // (claim 後 crash・復元失敗)— mtime は元 lock のまま。
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
        // file でない `*.lock.claim-*` orphan 等は一切回収されない —
        // 病理経路のみ・stray dir entry が残るだけの bounded な害。
        if !md.is_file() {
            continue;
        }
        let Ok(mtime) = md.modified() else {
            continue;
        };
        if past_stale_boundary(mtime, now, stale_after) {
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
    /// `lock_stale` 超過 → Stale(owner token 観測付き、unparseable
    /// payload は token 無し)。未来 mtime は skew 範囲内(`now +
    /// lock_stale` 以下)→ Fresh、`now + lock_stale` 超過 → Stale。
    /// file でない path は Unrecoverable(payload read・削除不可)。
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
        // skew 範囲内の未来 mtime → live lock 扱いで削除不可(Fresh)。
        set_mtime(&lock, SystemTime::now() + Duration::from_secs(60))?;
        assert!(matches!(lock_state(&lock, stale), LockState::Fresh));
        // `now + lock_stale` を超える未来 mtime → 自然 expire しない
        // ため stale 扱い(takeover 可)。
        set_mtime(&lock, SystemTime::now() + Duration::from_secs(3600))?;
        assert!(matches!(lock_state(&lock, stale), LockState::Stale { .. }));
        // unparseable payload + stale mtime → token 無しの Stale
        // (claim の照合材料を持たず削除されない)。
        std::fs::write(&lock, "garbage")?;
        set_mtime(&lock, SystemTime::now() - Duration::from_secs(3600))?;
        assert!(matches!(
            lock_state(&lock, stale),
            LockState::Stale { token: None }
        ));
        // file でない path(dir)は協調不能 — payload read・削除ともに
        // 行わない immediate self-eval 分類。
        let not_file = dir.path().join("dir.lock");
        std::fs::create_dir(&not_file)?;
        assert!(matches!(
            lock_state(&not_file, stale),
            LockState::Unrecoverable
        ));
        Ok(())
    }

    /// rename-claim — rename が atomic に file を capture し、観測
    /// token と一致した claim のみ file を除去して takeover へ進める。
    /// token 不一致(rename 窓で観測した file と別物)は `<fp>.lock`
    /// へ戻して諦める(live lock を殺さない)。
    #[test]
    fn claim_stale_lock_verifies_observed_token() -> TestResult {
        let dir = tempfile::tempdir()?;
        let lock = dir.path().join("abc.lock");
        // 不存在 → rename 失敗 → 敗者。
        assert!(!claim_stale_lock(&lock, &token(1)));
        // token 不一致(観測した file ではない)→ file を元へ戻す。
        std::fs::write(&lock, serde_json::to_string(&token(3))?)?;
        assert!(!claim_stale_lock(&lock, &token(9)));
        assert_eq!(
            std::fs::read_to_string(&lock)?,
            serde_json::to_string(&token(3))?,
            "token 不一致の file は `<fp>.lock` へ復元される"
        );
        // token 一致 → claim file を除去して takeover 可。
        assert!(claim_stale_lock(&lock, &token(3)));
        assert!(!lock.exists(), "stale file は claim 名へ退避後に除去");
        assert!(
            std::fs::read_dir(dir.path())?.next().is_none(),
            "claim 残骸が残らない"
        );
        Ok(())
    }

    /// sweep — TTL 超過の record・stale lock・stale claim orphan・
    /// orphan tmp を remove し、fresh entry は残す。未来 mtime は
    /// `now + stale_after` 超過のみ除去 — skew 範囲内の未来 mtime は
    /// live の可能性があるため残す。
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
        let stale_claim = store.join("b.lock.claim-1-9");
        let orphan_tmp = store.join(".a.json.tmp-1-1");
        let future_record = store.join("c.json");
        let near_future_lock = store.join("d.lock");
        let far_future_lock = store.join("e.lock");
        let unrelated = store.join("keep.txt");
        for path in [
            &fresh_record,
            &stale_record,
            &fresh_lock,
            &stale_lock,
            &stale_claim,
            &orphan_tmp,
            &future_record,
            &near_future_lock,
            &far_future_lock,
            &unrelated,
        ] {
            std::fs::write(path, "x")?;
        }
        let old = SystemTime::now() - Duration::from_secs(3600);
        let future = SystemTime::now() + Duration::from_secs(3600);
        set_mtime(&stale_record, old)?;
        set_mtime(&stale_lock, old)?;
        set_mtime(&stale_claim, old)?;
        set_mtime(&orphan_tmp, old)?;
        set_mtime(&future_record, future)?;
        set_mtime(&far_future_lock, future)?;
        // skew 範囲内の未来 mtime(1s < lock_stale 2s)は残る。
        set_mtime(
            &near_future_lock,
            SystemTime::now() + Duration::from_secs(1),
        )?;
        sweep_store(store, &limits);
        assert!(fresh_record.exists());
        assert!(fresh_lock.exists());
        assert!(unrelated.exists());
        assert!(!stale_record.exists());
        assert!(!stale_lock.exists());
        assert!(!stale_claim.exists(), "stale claim orphan を除去");
        assert!(!orphan_tmp.exists());
        assert!(!future_record.exists(), "far-future mtime is stale");
        assert!(
            !far_future_lock.exists(),
            "horizon 超過の未来 mtime lock を除去"
        );
        assert!(
            near_future_lock.exists(),
            "skew 範囲内の未来 mtime lock は残る"
        );
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
                resolved: b"/ws/big.txt".to_vec(),
                rel_posix: "big.txt".to_string(),
                mtime_nanos: 42,
                len: 100,
            }],
            threshold_bits,
            count_max_bytes: DEFAULT_COUNT_MAX_BYTES,
            builtin_policy: PolicySig::Embedded("deadbeefdeadbeef".to_string()),
            user_policy: PolicySig::Absent,
            project_policy: PolicySig::Unresolved,
            routes_sig: Vec::new(),
        };
        let base = compute_fingerprint(&input(8192.0f64.to_bits()));
        assert_eq!(base, compute_fingerprint(&input(8192.0f64.to_bits())));
        assert_eq!(base.len(), 16);
        assert!(base.chars().all(|c| c.is_ascii_hexdigit()));
        assert_ne!(base, compute_fingerprint(&input(1024.0f64.to_bits())));
    }

    /// routes signature は fingerprint 入力 — routes.yaml の内容
    /// 変化(stat 差)が fingerprint を割る。同一 store_dir・同一
    /// request で routes 差だけ key が変われば stale verdict の
    /// serve は構造上あり得ない。
    #[test]
    fn fingerprint_includes_routes_signature() {
        let input = |routes_sig: Vec<PolicySig>| FingerprintInput {
            tag: FINGERPRINT_TAG,
            workspace_root: Some("/ws".to_string()),
            variant: VariantInputs::Shell {
                command: "cargo test".to_string(),
            },
            targets: vec![TargetSig::Absent {
                raw: b"/ws/x".to_vec(),
            }],
            threshold_bits: 8192.0f64.to_bits(),
            count_max_bytes: DEFAULT_COUNT_MAX_BYTES,
            builtin_policy: PolicySig::Absent,
            user_policy: PolicySig::Absent,
            project_policy: PolicySig::Absent,
            routes_sig,
        };
        let base = compute_fingerprint(&input(vec![
            PolicySig::Absent,
            PolicySig::Absent,
            PolicySig::Absent,
        ]));
        assert_ne!(
            base,
            compute_fingerprint(&input(vec![
                PolicySig::Absent,
                PolicySig::File {
                    mtime_nanos: 7,
                    len: 42
                },
                PolicySig::Absent,
            ])),
            "user 層 routes.yaml 出現で fingerprint が変わる"
        );
        assert_ne!(
            base,
            compute_fingerprint(&input(vec![
                PolicySig::Absent,
                PolicySig::Absent,
                PolicySig::File {
                    mtime_nanos: 9,
                    len: 42
                },
            ])),
            "project 層 routes.yaml 出現で fingerprint が変わる"
        );
        let contents_a = compute_fingerprint(&input(vec![
            PolicySig::Absent,
            PolicySig::Contents(1),
            PolicySig::Absent,
        ]));
        assert_ne!(
            contents_a,
            compute_fingerprint(&input(vec![
                PolicySig::Absent,
                PolicySig::Contents(2),
                PolicySig::Absent,
            ])),
            "user 層 routes.yaml の内容差分は fingerprint を変える"
        );
    }

    #[test]
    fn routes_content_change_with_preserved_stat_changes_fingerprint() -> TestResult {
        let dir = tempfile::tempdir()?;
        let user_cfg = dir.path().join("cfg");
        let workspace = dir.path().join("ws");
        std::fs::create_dir_all(&user_cfg)?;
        std::fs::create_dir_all(&workspace)?;
        let store = user_cfg.join("store");
        let routes = user_cfg.join("routes.yaml");
        std::fs::write(
            &routes,
            "version: 1\nroutes:\n  - {name: r, match: {command: [cargo]}, target: {kind: workflow, path: w.json}, grant: aaa}\n",
        )?;
        let before = std::fs::metadata(&routes)?;
        let times = std::fs::FileTimes::new().set_modified(before.modified()?);
        let limits = DedupLimits {
            verdict_ttl: Duration::from_millis(1_000),
            lock_stale: Duration::from_millis(2_000),
            follower_wait: Duration::from_millis(100),
            poll_interval: Duration::from_millis(10),
        };
        let mk_ctx = || FingerprintContext {
            workspace_root: Some(
                workspace
                    .canonicalize()
                    .unwrap()
                    .to_string_lossy()
                    .into_owned(),
            ),
            targets: vec![ResolvedTarget {
                resolved: workspace.join("a.txt"),
                rel_posix: "a.txt".to_string(),
                kind: ResolvedKind::NonFile,
            }],
        };
        let env_map: HashMap<String, String> = HashMap::new();
        let shell = || VariantInputs::Shell {
            command: "cargo test".to_string(),
        };
        let fp1 = prepare_dedup(
            mk_ctx(),
            shell(),
            env_map.clone(),
            None,
            store.clone(),
            limits,
        )
        .ok_or("prepare_dedup fallback")?
        .fingerprint;
        let file_fp1 = prepare_dedup(
            mk_ctx(),
            VariantInputs::FileRead {
                range: Some((0, Some(10))),
            },
            env_map.clone(),
            None,
            store.clone(),
            limits,
        )
        .ok_or("prepare_dedup fallback")?
        .fingerprint;
        std::fs::write(
            &routes,
            "version: 1\nroutes:\n  - {name: r, match: {command: [cargo]}, target: {kind: workflow, path: w.json}, grant: bbb}\n",
        )?;
        std::fs::OpenOptions::new()
            .write(true)
            .open(&routes)?
            .set_times(times)?;
        let after = std::fs::metadata(&routes)?;
        assert_eq!(before.len(), after.len(), "byte 長は不変");
        assert_eq!(before.modified()?, after.modified()?, "mtime 復元確認");
        let fp2 = prepare_dedup(
            mk_ctx(),
            shell(),
            env_map.clone(),
            None,
            store.clone(),
            limits,
        )
        .ok_or("prepare_dedup fallback")?
        .fingerprint;
        assert_ne!(fp1, fp2, "同 stat 内容差分で fingerprint が変わる");
        let file_fp2 = prepare_dedup(
            mk_ctx(),
            VariantInputs::FileRead {
                range: Some((0, Some(10))),
            },
            env_map,
            None,
            store,
            limits,
        )
        .ok_or("prepare_dedup fallback")?
        .fingerprint;
        assert_eq!(file_fp1, file_fp2, "非 Shell は routes 非入力のまま");
        Ok(())
    }

    /// `prepare_dedup` は Shell variant のみ routes signature を
    /// fingerprint へ載せる — File/Search request では routes.yaml
    /// の有無が fingerprint を割らない(route 非入力の eval を
    /// routes 変更で不必要に invalidate しない)。
    #[test]
    fn prepare_dedup_scopes_routes_sig_to_shell_variant() -> TestResult {
        let dir = tempfile::tempdir()?;
        let store = dir.path().join("store");
        let user_cfg = dir.path();
        let workspace = dir.path().join("ws");
        std::fs::create_dir_all(&workspace)?;
        let limits = DedupLimits {
            verdict_ttl: Duration::from_millis(1_000),
            lock_stale: Duration::from_millis(2_000),
            follower_wait: Duration::from_millis(100),
            poll_interval: Duration::from_millis(10),
        };
        let mk_ctx = |target: &str| FingerprintContext {
            workspace_root: Some(
                workspace
                    .canonicalize()
                    .unwrap()
                    .to_string_lossy()
                    .into_owned(),
            ),
            targets: vec![ResolvedTarget {
                resolved: workspace.join(target),
                rel_posix: target.to_string(),
                kind: ResolvedKind::NonFile,
            }],
        };
        let env_map: HashMap<String, String> = HashMap::new();
        // user 層 routes.yaml が無い状態の baseline。
        let ctx = mk_ctx("a.txt");
        let shell_base = prepare_dedup(
            ctx,
            VariantInputs::Shell {
                command: "cargo test".to_string(),
            },
            env_map.clone(),
            None,
            store.clone(),
            limits,
        )
        .ok_or("prepare_dedup fallback")?
        .fingerprint;
        let file_base = prepare_dedup(
            mk_ctx("a.txt"),
            VariantInputs::FileRead {
                range: Some((0, Some(10))),
            },
            env_map.clone(),
            None,
            store.clone(),
            limits,
        )
        .ok_or("prepare_dedup fallback")?
        .fingerprint;
        // user 層 routes.yaml 出現(store_dir の parent = user config)。
        std::fs::write(user_cfg.join("routes.yaml"), "routes: []\n")?;
        let shell_after = prepare_dedup(
            mk_ctx("a.txt"),
            VariantInputs::Shell {
                command: "cargo test".to_string(),
            },
            env_map.clone(),
            None,
            store.clone(),
            limits,
        )
        .ok_or("prepare_dedup fallback")?
        .fingerprint;
        let file_after = prepare_dedup(
            mk_ctx("a.txt"),
            VariantInputs::FileRead {
                range: Some((0, Some(10))),
            },
            env_map,
            None,
            store,
            limits,
        )
        .ok_or("prepare_dedup fallback")?
        .fingerprint;
        assert_ne!(
            shell_base, shell_after,
            "routes.yaml 出現で shell fingerprint が変わる(stale verdict 無効化)"
        );
        assert_eq!(
            file_base, file_after,
            "file-read fingerprint は routes 変更に不変"
        );
        Ok(())
    }

    /// publish 側 gate — shareable verdict のみ record body を生成。
    /// `Verdict::Rewrite` 追加時は `is_shareable` の exhaustive
    /// match が compile error で明示判断を強制する。
    #[test]
    fn verdict_record_body_only_serializes_shareable_verdicts() -> TestResult {
        let body =
            verdict_record_body("0123456789abcdef", &Verdict::Allow).ok_or("allow は shareable")?;
        let json: serde_json::Value = serde_json::from_str(&body)?;
        assert_eq!(json["fingerprint"], "0123456789abcdef");
        assert_eq!(json["verdict"]["decision"], "allow");
        assert!(verdict_record_body(
            "0123456789abcdef",
            &Verdict::Deny {
                reason: "r".to_string()
            }
        )
        .is_some());
        Ok(())
    }
}
