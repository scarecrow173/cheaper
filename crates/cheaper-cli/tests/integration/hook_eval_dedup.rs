//! `cheaper hook` の eval dedup(verdict cache + leader election、issue
//! #127)contract test —
//! `docs/test-specs/hook-eval-dedup-verdict-cache-contract.md` の
//! Examples に 1:1 対応する。
//!
//! 検証は 2 層:
//! - module 層 seam: `cheaper_cli::hook::eval_cache::evaluate_dedup_with`
//!   (`#[doc(hidden)] pub`)へ `Arc<AtomicUsize>` で計数する `eval_fn`
//!   (200ms 級 sleep で重なり確定)と ms スケールの `DedupLimits` を
//!   注入し、協調・fingerprint invalidation・fallback を timing race
//!   に依存せず決定的に検証する。fingerprint は最初の 1 firing で
//!   `<cfg>/hook-eval/` に生成される `<16hex>.json` の file 名から
//!   取得し、lock/record の手配置に使う。
//! - binary 層: `env!("CARGO_BIN_EXE_cheaper")` の dual spawn で
//!   per-target render・exit code・store file 残存を検証する。
//!
//! 隔離: `hook.rs` の `pub(crate)` `Fixture` が `CHEAPER_CONFIG_DIR`・
//! `HOME`・`USERPROFILE` を tempdir へ向ける env map を供給するため、
//! test は実 `~/.config/cheaper/hook-eval/` を一切触れない。
//! process-global cwd mutation(`set_current_dir`)は使わない — cwd
//! spelling 差は tempdir 配下の絶対 path 表現のみで検証する。

use std::io;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, SystemTime};

use cheaper_cli::hook::eval_cache::{evaluate_dedup_with, DedupLimits};
use cheaper_cli::hook::routing_policy::{
    EvaluateOptions, RoutingRequest, SearchOutputMode, Verdict,
};

use crate::hook::{
    big_text, devin_payload, run_hook, tool_use_payload, windsurf_read, Fixture, HookResult,
};

type TestResult = Result<(), Box<dyn std::error::Error>>;

/// dedup 協調が走る default limits — follower_wait は計測 sleep
/// (200–500ms)を十分に上回り、TTL/stale は手配置 mtime 操作で跨ぐ。
fn default_limits() -> DedupLimits {
    DedupLimits {
        verdict_ttl: Duration::from_secs(60),
        lock_stale: Duration::from_secs(120),
        follower_wait: Duration::from_secs(10),
        poll_interval: Duration::from_millis(10),
    }
}

/// `EvaluateOptions` の組立 — `env` field は `&dyn Fn`(非 Send)の
/// ため、env map を move した lookup を Box で保持し `opts()` の借用
/// を self へ anchor する。
type EnvLookup = Box<dyn Fn(&str) -> Option<String>>;

struct SeamEnv {
    lookup: EnvLookup,
    builtin_dir: PathBuf,
}

impl SeamEnv {
    fn new(fx: &Fixture, extra: &[(&str, &str)]) -> Self {
        let mut map = fx.env.clone();
        for (k, v) in extra {
            map.insert((*k).to_string(), (*v).to_string());
        }
        SeamEnv {
            lookup: Box::new(move |n: &str| map.get(n).cloned()),
            builtin_dir: fx.builtin_dir.clone(),
        }
    }

    fn opts(&self) -> EvaluateOptions<'_> {
        EvaluateOptions {
            env: &*self.lookup,
            builtin_dir: Some(&self.builtin_dir),
            route: None,
        }
    }
}

/// `<userConfigDir>/hook-eval/` — `Fixture.env` の
/// `CHEAPER_CONFIG_DIR` 起点(store root は env 分岐)。
fn store_dir(fx: &Fixture) -> PathBuf {
    Path::new(&fx.env["CHEAPER_CONFIG_DIR"]).join("hook-eval")
}

/// `<fp>.json` record file の path。
fn record_path(store: &Path, fp: &str) -> PathBuf {
    store.join(format!("{fp}.json"))
}

fn lock_path(store: &Path, fp: &str) -> PathBuf {
    store.join(format!("{fp}.lock"))
}

/// `hook-eval/` 内の `<fp>.json` の stem(`<fp>`)を昇順で列挙。
/// dir 不存在・orphan `.tmp-*`・`.lock` は含まない。
fn record_fps(store: &Path) -> Vec<String> {
    let mut out = Vec::new();
    if let Ok(rd) = std::fs::read_dir(store) {
        for entry in rd.flatten() {
            let name = entry.file_name();
            if let Some(stem) = name.to_str().and_then(|n| n.strip_suffix(".json")) {
                out.push(stem.to_string());
            }
        }
    }
    out.sort();
    out
}

/// 最初の 1 firing で生成された唯一の record fingerprint。
fn single_fp(store: &Path) -> io::Result<String> {
    let fps = record_fps(store);
    match fps.as_slice() {
        [fp] => Ok(fp.clone()),
        _ => Err(io::Error::other(format!(
            "expected exactly one verdict record, got {fps:?}"
        ))),
    }
}

/// lock/record の mtime を付け替える(`File::set_modified` — 新規 dep
/// 不要)。
fn set_mtime(path: &Path, time: SystemTime) -> io::Result<()> {
    std::fs::File::options()
        .write(true)
        .open(path)?
        .set_modified(time)
}

/// `FileRead` の canonical request。
fn file_read(path: &str, cwd: &str) -> RoutingRequest {
    RoutingRequest::FileRead {
        path: path.to_string(),
        cwd: cwd.to_string(),
        source_tool: Some("test".to_string()),
        range: None,
    }
}

/// `Arc<AtomicUsize>` で計数し `delay` だけ待ってから `reason` の
/// deny を返す計測 seam — 「評価が高々 1 回」「重なり」の直接観測用。
fn fake_eval(
    calls: Arc<AtomicUsize>,
    delay: Duration,
    reason: &'static str,
) -> impl Fn() -> std::pin::Pin<Box<dyn std::future::Future<Output = Verdict>>> {
    move || {
        let calls = calls.clone();
        Box::pin(async move {
            calls.fetch_add(1, Ordering::SeqCst);
            if !delay.is_zero() {
                tokio::time::sleep(delay).await;
            }
            Verdict::Deny {
                reason: reason.to_string(),
            }
        }) as std::pin::Pin<Box<dyn std::future::Future<Output = Verdict>>>
    }
}

/// valid lock owner token JSON — `(pid, createdAt, nonce)` triple が
/// owner を一意に識別する(`LockToken` serde 表現)。
fn lock_token_json(nonce: u64) -> String {
    format!("{{\"pid\":4242,\"createdAt\":1,\"nonce\":{nonce}}}")
}

/// directory link を作る — Windows の `junction` は privilege 不要
/// (`cmd /c mklink /J`、golden/differential harness と同一手法)。
/// 失敗時は `false`(symlink 不可環境の case skip 扱い)。
fn dir_link(target: &Path, link: &Path) -> bool {
    #[cfg(windows)]
    {
        std::process::Command::new("cmd")
            .args(["/c", "mklink", "/J"])
            .arg(link)
            .arg(target)
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }
    #[cfg(not(windows))]
    {
        std::os::unix::fs::symlink(target, link).is_ok()
    }
}

// --- binary 層 spawn helper ----------------------------------------------

/// spawned child の env — ambient の OS 必須 key + `Fixture.env` の
/// 隔離 map(`CHEAPER_CONFIG_DIR`・`HOME`・`USERPROFILE`)+ TEMP/TMP/
/// TMPDIR を tempdir へ向ける(embedded builtin materialize 先)。
fn child_env(fx: &Fixture) -> Vec<(String, String)> {
    let mut env: Vec<(String, String)> = Vec::new();
    for key in [
        "PATH",
        "SYSTEMROOT",
        "WINDIR",
        "PATHEXT",
        "COMSPEC",
        "NUMBER_OF_PROCESSORS",
    ] {
        if let Some(v) = std::env::var_os(key) {
            env.push((key.to_string(), v.to_string_lossy().into_owned()));
        }
    }
    for (k, v) in &fx.env {
        env.push((k.clone(), v.clone()));
    }
    let tmp = fx
        .ws
        .parent()
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_default();
    for key in ["TEMP", "TMP", "TMPDIR"] {
        env.push((key.to_string(), tmp.clone()));
    }
    env
}

/// `cheaper hook` を `env_clear` + 隔離 env・`ws` cwd で spawn。
fn spawn_hook(args: &[&str], ws: &Path, env: &[(String, String)]) -> io::Result<Child> {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_cheaper"));
    cmd.args(args)
        .current_dir(ws)
        .env_clear()
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    for (k, v) in env {
        cmd.env(k, v);
    }
    cmd.spawn()
}

/// child の stdin を書き切って close する(payload は小さいので
/// pipe buffer 詰まりは無い)。`wait` とは分離し、dual spawn では両
/// child の stdin を先に feed してから wait する(真の overlap)。
fn feed_stdin(child: &mut Child, stdin: &[u8]) -> io::Result<()> {
    use std::io::Write;
    if let Some(mut s) = child.stdin.take() {
        s.write_all(stdin)?;
        drop(s);
    }
    Ok(())
}

/// spawn 済み child の終了を待って出力を回収する。
fn wait_child(child: Child) -> io::Result<HookResult> {
    let out = child.wait_with_output()?;
    Ok(HookResult {
        code: out.status.code().unwrap_or(-1),
        stdout: String::from_utf8(out.stdout)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?,
        stderr: String::from_utf8(out.stderr)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?,
    })
}

// --- AC1 / AC2 ----------------------------------------------------------

/// `concurrent_firings_single_evaluation` — 同一 `RoutingRequest`・同一
/// `CHEAPER_CONFIG_DIR` で 2 つの `evaluate_dedup_with` を `tokio::join!`
/// すると eval_fn 計数 = 1・両 verdict が同一内容。3 発目は計数不変の
/// 純粋 cache hit。
#[tokio::test(flavor = "current_thread")]
async fn concurrent_firings_single_evaluation() -> TestResult {
    let fx = Fixture::new();
    fx.write_big("big.txt");
    let env = SeamEnv::new(&fx, &[]);
    let opts = env.opts();
    let req = file_read("big.txt", &fx.ws.to_string_lossy());
    let limits = default_limits();
    let calls = Arc::new(AtomicUsize::new(0));
    let (va, vb) = tokio::join!(
        evaluate_dedup_with(
            &req,
            &opts,
            &limits,
            fake_eval(calls.clone(), Duration::from_millis(200), "r")
        ),
        evaluate_dedup_with(
            &req,
            &opts,
            &limits,
            fake_eval(calls.clone(), Duration::from_millis(200), "r")
        ),
    );
    assert_eq!(va, vb, "両 firing が同一 verdict");
    assert_eq!(calls.load(Ordering::SeqCst), 1, "評価は高々 1 回");
    // 3 発目は純粋 cache hit — 計数不変。
    let vc = evaluate_dedup_with(
        &req,
        &opts,
        &limits,
        fake_eval(calls.clone(), Duration::ZERO, "r"),
    )
    .await;
    assert_eq!(vc, va, "cache hit が同一 verdict を返す");
    assert_eq!(
        calls.load(Ordering::SeqCst),
        1,
        "3 発目は eval_fn を呼ばない"
    );
    assert_eq!(
        record_fps(&store_dir(&fx)).len(),
        1,
        "verdict record は 1 件"
    );
    Ok(())
}

/// `binary_dual_spawn_same_verdict` — `cheaper hook` を `--target devin`
/// と `--target pretooluse` で同時 spawn(同一 `CHEAPER_CONFIG_DIR`・
/// 同一 cwd)し、両 process に同一の devin-shaped payload を流す。
/// 余分な `hook_event_name` key は `tool_use_normalize` で無視され、両
/// target が同一 FileRead request へ normalize されるため cross-target
/// dedup が証明できる。`file_path` は threshold 超過の実 file —
/// deny verdict が token 計測経路を通る(fast-path gate でも skip され
/// ない)。
#[test]
fn binary_dual_spawn_same_verdict() -> TestResult {
    let fx = Fixture::new();
    fx.write_big("big.txt");
    let big = fx.ws.join("big.txt").to_string_lossy().into_owned();
    let stdin = devin_payload(
        "read",
        &format!("{{\"file_path\":{}}}", serde_json::to_string(&big)?),
    );
    let env = child_env(&fx);
    let mut a = spawn_hook(&["hook", "pre-tool-use", "--target", "devin"], &fx.ws, &env)?;
    let mut b = spawn_hook(
        &["hook", "pre-tool-use", "--target", "pretooluse"],
        &fx.ws,
        &env,
    )?;
    // 両 process の stdin を wait 前に書き切る — write→wait の逐次だと
    // overlap が起きず cross-process dedup を実証しない。
    feed_stdin(&mut a, stdin.as_bytes())?;
    feed_stdin(&mut b, stdin.as_bytes())?;
    let ra = wait_child(a)?;
    let rb = wait_child(b)?;
    assert_eq!(ra.code, 0, "devin stderr: {:?}", ra.stderr);
    assert_eq!(rb.code, 0, "pretooluse stderr: {:?}", rb.stderr);
    let da: serde_json::Value = serde_json::from_str(ra.stdout.trim())?;
    let db: serde_json::Value = serde_json::from_str(rb.stdout.trim())?;
    assert_eq!(da["decision"], "block", "{}", ra.stdout);
    assert_eq!(
        db["hookSpecificOutput"]["permissionDecision"], "deny",
        "{}",
        rb.stdout
    );
    // 同一 verdict の render — devin は multi-line reason、pretooluse
    // は flatten 済み(`flatten_reason` 差のみで内容は同一)。
    let reason_a = da["reason"]
        .as_str()
        .ok_or("devin reason missing")?
        .replace('\n', " ");
    let reason_b = db["hookSpecificOutput"]["permissionDecisionReason"]
        .as_str()
        .ok_or("pretooluse reason missing")?;
    assert_eq!(reason_a, reason_b, "同一 deny verdict の render");
    assert!(reason_a.contains("bulk-read"), "{reason_a}");
    // dedup — verdict record は 1 件、lock は残存しない。
    let store = store_dir(&fx);
    assert_eq!(record_fps(&store).len(), 1, "record は 1 件のみ");
    if let Ok(rd) = std::fs::read_dir(&store) {
        for entry in rd.flatten() {
            assert!(
                !entry.file_name().to_string_lossy().ends_with(".lock"),
                "lock must be released: {:?}",
                entry.file_name()
            );
        }
    }
    Ok(())
}

/// `per_target_render_identity` — 同一 payload 2 連続実行で 1 回目
/// (leader 評価)と 2 回目(cache serve)の stdout/stderr/exit が byte
/// 同一 — 5 target 全て。cache されるのは canonical `Verdict` のみで
/// render 済み応答は共有しない設計の固定。
#[tokio::test(flavor = "current_thread")]
async fn per_target_render_identity() -> TestResult {
    let fx = Fixture::new();
    fx.write_big("big.txt");
    // 5 target 全てが FileRead{"big.txt", ws} の同一 canonical request
    // へ normalize される(payload key の余剰・`source_tool` 差は
    // fingerprint に入らない)。
    let cases: Vec<(&str, &[&str], String, &str)> = vec![
        (
            "windsurf",
            &["pre-read-code", "--target", "windsurf"],
            windsurf_read("big.txt"),
            "stderr-deny",
        ),
        (
            "copilot",
            &["pre-tool-use", "--target", "copilot"],
            "{\"toolName\":\"view\",\"toolArgs\":{\"path\":\"big.txt\"}}".to_string(),
            "stdout-deny",
        ),
        (
            "pretooluse",
            &["pre-tool-use", "--target", "pretooluse"],
            tool_use_payload("Read", "{\"file_path\":\"big.txt\"}"),
            "stdout-deny",
        ),
        (
            "devin",
            &["pre-tool-use", "--target", "devin"],
            devin_payload("read", "{\"file_path\":\"big.txt\"}"),
            "stdout-block",
        ),
        (
            "cursor",
            &["before-read-file", "--target", "cursor"],
            "{\"file_path\":\"big.txt\"}".to_string(),
            "stdout-deny",
        ),
    ];
    for (name, args, stdin, expect) in &cases {
        let r1 = run_hook(&fx, args, stdin, &[]).await;
        let r2 = run_hook(&fx, args, stdin, &[]).await;
        // deny が実際に render されている(allow silent の accidental
        // pass 防止 — serve 側も deny verdict を返した証明)。
        match *expect {
            "stderr-deny" => {
                assert_eq!(r1.code, 2, "{name}");
                assert!(r1.stderr.contains("bulk-read"), "{name}: {}", r1.stderr);
            }
            "stdout-block" => {
                assert_eq!(r1.code, 0, "{name}");
                assert!(
                    r1.stdout.contains("\"decision\":\"block\""),
                    "{name}: {}",
                    r1.stdout
                );
            }
            _ => {
                assert_eq!(r1.code, 0, "{name}");
                assert!(
                    r1.stdout.contains("\"permissionDecision\":\"deny\"")
                        || r1.stdout.contains("\"permission\":\"deny\""),
                    "{name}: {}",
                    r1.stdout
                );
            }
        }
        assert_eq!(
            (r1.code, &r1.stdout, &r1.stderr),
            (r2.code, &r2.stdout, &r2.stderr),
            "{name}: 1 回目(評価)と 2 回目(cache serve)が byte 同一"
        );
    }
    // cross-target で同一 fingerprint — record は 1 件のみ。
    assert_eq!(record_fps(&store_dir(&fx)).len(), 1);
    Ok(())
}

// --- AC3: fingerprint invalidation --------------------------------------

/// `stat_change_new_fingerprint` — file の size 変化で別 fingerprint →
/// 再評価(計数増・別 `<fp>.json`)。
#[tokio::test(flavor = "current_thread")]
async fn stat_change_new_fingerprint() -> TestResult {
    let fx = Fixture::new();
    fx.write_big("big.txt");
    let env = SeamEnv::new(&fx, &[]);
    let opts = env.opts();
    let req = file_read("big.txt", &fx.ws.to_string_lossy());
    let limits = default_limits();
    let calls = Arc::new(AtomicUsize::new(0));
    let store = store_dir(&fx);
    evaluate_dedup_with(
        &req,
        &opts,
        &limits,
        fake_eval(calls.clone(), Duration::ZERO, "r"),
    )
    .await;
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    let fp1 = single_fp(&store)?;
    // size 変化(mtime も進む)→ 別 fingerprint → 再評価。
    fx.write("big.txt", format!("{} tail", big_text()).as_bytes());
    evaluate_dedup_with(
        &req,
        &opts,
        &limits,
        fake_eval(calls.clone(), Duration::ZERO, "r"),
    )
    .await;
    assert_eq!(calls.load(Ordering::SeqCst), 2, "stat 変化で再評価");
    let fps = record_fps(&store);
    assert_eq!(fps.len(), 2, "別 fingerprint の record が独立に残る");
    assert!(fps.contains(&fp1), "旧 record は TTL 内で残存する");
    Ok(())
}

/// `threshold_change_new_fingerprint` — `CHEAPER_READ_TOKEN_THRESHOLD`
/// 変化で別 fingerprint → 再評価。
#[tokio::test(flavor = "current_thread")]
async fn threshold_change_new_fingerprint() -> TestResult {
    let fx = Fixture::new();
    fx.write_big("big.txt");
    let env1 = SeamEnv::new(&fx, &[]);
    let opts1 = env1.opts();
    let env2 = SeamEnv::new(&fx, &[("CHEAPER_READ_TOKEN_THRESHOLD", "1024")]);
    let opts2 = env2.opts();
    let req = file_read("big.txt", &fx.ws.to_string_lossy());
    let limits = default_limits();
    let calls = Arc::new(AtomicUsize::new(0));
    evaluate_dedup_with(
        &req,
        &opts1,
        &limits,
        fake_eval(calls.clone(), Duration::ZERO, "r"),
    )
    .await;
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    evaluate_dedup_with(
        &req,
        &opts2,
        &limits,
        fake_eval(calls.clone(), Duration::ZERO, "r"),
    )
    .await;
    assert_eq!(
        calls.load(Ordering::SeqCst),
        2,
        "threshold 変化で別 fingerprint → 再評価"
    );
    assert_eq!(record_fps(&store_dir(&fx)).len(), 2);
    Ok(())
}

/// `policy_and_count_cap_new_fingerprint` — user `<userConfigDir>/
/// policy.yaml` の stat 変化と `CHEAPER_COUNT_MAX_BYTES` 変化がそれぞれ
/// 別 fingerprint を生成し独立評価される(R2 — verdict を変え得る
/// 観測可能入力の同一性キー反映)。
#[tokio::test(flavor = "current_thread")]
async fn policy_and_count_cap_new_fingerprint() -> TestResult {
    let fx = Fixture::new();
    fx.write_big("big.txt");
    let env1 = SeamEnv::new(&fx, &[]);
    let opts1 = env1.opts();
    let req = file_read("big.txt", &fx.ws.to_string_lossy());
    let limits = default_limits();
    let calls = Arc::new(AtomicUsize::new(0));
    let store = store_dir(&fx);
    evaluate_dedup_with(
        &req,
        &opts1,
        &limits,
        fake_eval(calls.clone(), Duration::ZERO, "r"),
    )
    .await;
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    assert_eq!(record_fps(&store).len(), 1);
    // user policy 層の stat signature: Absent → File で別 fingerprint。
    std::fs::write(
        Path::new(&fx.env["CHEAPER_CONFIG_DIR"]).join("policy.yaml"),
        "version: 2\n",
    )?;
    evaluate_dedup_with(
        &req,
        &opts1,
        &limits,
        fake_eval(calls.clone(), Duration::ZERO, "r"),
    )
    .await;
    assert_eq!(
        calls.load(Ordering::SeqCst),
        2,
        "policy 層 stat 変化で再評価"
    );
    assert_eq!(record_fps(&store).len(), 2);
    // count cap 変化でも別 fingerprint(policy 層は維持)。
    let env2 = SeamEnv::new(&fx, &[("CHEAPER_COUNT_MAX_BYTES", "1024")]);
    let opts2 = env2.opts();
    evaluate_dedup_with(
        &req,
        &opts2,
        &limits,
        fake_eval(calls.clone(), Duration::ZERO, "r"),
    )
    .await;
    assert_eq!(calls.load(Ordering::SeqCst), 3, "count cap 変化で再評価");
    assert_eq!(record_fps(&store).len(), 3);
    Ok(())
}

/// `cwd_spelling_same_fingerprint` — tempdir 配下の絶対 spelling 差
/// (`<ws>/./sub`・`<ws>/sub/`・symlink `<ws>/link` → `<ws>/sub`)が
/// `canonicalize_workspace_root` + canonicalize 済み resolved path で
/// 同一 fingerprint に収束する。相対 spelling(`./sub`)は process cwd
/// に anchor されるため `set_current_dir` が要り global state race を
/// 招く — 使わない。
#[tokio::test(flavor = "current_thread")]
async fn cwd_spelling_same_fingerprint() -> TestResult {
    let fx = Fixture::new();
    let sub = fx.ws.join("sub");
    std::fs::create_dir_all(&sub)?;
    fx.write("sub/big.txt", big_text().as_bytes());
    let ws = fx.ws.to_string_lossy();
    let mut spellings = vec![format!("{ws}/./sub"), format!("{ws}/sub/")];
    // symlink/junction 経由 — 作成不能な環境では spelling 追加を skip。
    let link = fx.ws.join("link");
    if dir_link(&sub, &link) {
        spellings.push(link.to_string_lossy().into_owned());
    }
    let env = SeamEnv::new(&fx, &[]);
    let opts = env.opts();
    let limits = default_limits();
    let calls = Arc::new(AtomicUsize::new(0));
    for cwd in &spellings {
        let req = file_read("big.txt", cwd);
        evaluate_dedup_with(
            &req,
            &opts,
            &limits,
            fake_eval(calls.clone(), Duration::ZERO, "r"),
        )
        .await;
    }
    assert_eq!(
        calls.load(Ordering::SeqCst),
        1,
        "cwd spelling 差は同一 fingerprint へ収束(計数 1): {spellings:?}"
    );
    assert_eq!(record_fps(&store_dir(&fx)).len(), 1);
    Ok(())
}

// --- AC4: leader lock ----------------------------------------------------

/// `stale_lock_takeover` — `<fp>.lock` 手配置 + `set_modified` で
/// `lock_stale` 超過の古い mtime → follower が takeover して評価完遂。
/// 実際に通る機構は `prepare_dedup` の `sweep_store` — backdated lock
/// は poll 開始前の sweep で除去され、`poll_tick` は `Absent` から
/// `create_new` で取得する(rename-claim `claim_stale_lock` 経路は
/// 踏まない)。claim 経路は `fresh_lock_stale_crossing_takeover` と
/// unit test `claim_stale_lock_verifies_observed_token` がカバーする。
#[tokio::test(flavor = "current_thread")]
async fn stale_lock_takeover() -> TestResult {
    let fx = Fixture::new();
    fx.write_big("big.txt");
    let env = SeamEnv::new(&fx, &[]);
    let opts = env.opts();
    let req = file_read("big.txt", &fx.ws.to_string_lossy());
    let limits = default_limits();
    let calls = Arc::new(AtomicUsize::new(0));
    let store = store_dir(&fx);
    evaluate_dedup_with(
        &req,
        &opts,
        &limits,
        fake_eval(calls.clone(), Duration::ZERO, "r"),
    )
    .await;
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    let fp = single_fp(&store)?;
    // leader crash 模擬 — record を消し、stale mtime の lock を手配置。
    std::fs::remove_file(record_path(&store, &fp))?;
    let lock = lock_path(&store, &fp);
    std::fs::write(&lock, lock_token_json(7))?;
    set_mtime(&lock, SystemTime::now() - Duration::from_secs(3600))?;
    let v = evaluate_dedup_with(
        &req,
        &opts,
        &limits,
        fake_eval(calls.clone(), Duration::ZERO, "r"),
    )
    .await;
    assert_eq!(calls.load(Ordering::SeqCst), 2, "takeover で評価完遂");
    assert_eq!(
        v,
        Verdict::Deny {
            reason: "r".to_string()
        }
    );
    assert!(!lock.exists(), "takeover した leader が自 lock を release");
    assert!(record_path(&store, &fp).exists(), "verdict を publish");
    Ok(())
}

/// `fresh_lock_stale_crossing_takeover` — **fresh** mtime で植えた
/// lock が follower の poll 中に `lock_stale` を跨ぐ経路(`lock_stale`
/// 50ms << `follower_wait`)。初回 tick は Fresh → Wait、age が stale
/// を越えた tick で Stale → rename-claim → takeover が決定的に走る
/// (backdate 手配置の `stale_lock_takeover` では到達しない mid-poll
/// の Fresh→Stale 遷移と claim 経路を検証)。
#[tokio::test(flavor = "current_thread")]
async fn fresh_lock_stale_crossing_takeover() -> TestResult {
    let fx = Fixture::new();
    fx.write_big("big.txt");
    let env = SeamEnv::new(&fx, &[]);
    let opts = env.opts();
    let req = file_read("big.txt", &fx.ws.to_string_lossy());
    let limits = DedupLimits {
        lock_stale: Duration::from_millis(50),
        follower_wait: Duration::from_millis(2_000),
        ..default_limits()
    };
    let calls = Arc::new(AtomicUsize::new(0));
    let store = store_dir(&fx);
    evaluate_dedup_with(
        &req,
        &opts,
        &limits,
        fake_eval(calls.clone(), Duration::ZERO, "r"),
    )
    .await;
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    let fp = single_fp(&store)?;
    std::fs::remove_file(record_path(&store, &fp))?;
    // fresh mtime の valid-token lock を植える — poll の初回 tick で
    // Fresh → Wait、~50ms 経過後の tick で Stale → claim → takeover。
    let lock = lock_path(&store, &fp);
    std::fs::write(&lock, lock_token_json(21))?;
    let v = evaluate_dedup_with(
        &req,
        &opts,
        &limits,
        fake_eval(calls.clone(), Duration::ZERO, "r"),
    )
    .await;
    assert_eq!(
        calls.load(Ordering::SeqCst),
        2,
        "mid-poll stale crossing で takeover → 評価完遂"
    );
    assert_eq!(
        v,
        Verdict::Deny {
            reason: "r".to_string()
        }
    );
    assert!(!lock.exists(), "takeover leader は自 lock を release");
    assert!(record_path(&store, &fp).exists(), "verdict を publish");
    Ok(())
}

/// `fresh_lock_no_verdict_self_eval` — fresh lock + verdict 無し(crash
/// mid-eval 模擬)で follower が `follower_wait` deadline 後に
/// self-eval(計数 = 2)。self-eval は cache write しない(publish
/// 経路は leader 単独)。
#[tokio::test(flavor = "current_thread")]
async fn fresh_lock_no_verdict_self_eval() -> TestResult {
    let fx = Fixture::new();
    fx.write_big("big.txt");
    let env = SeamEnv::new(&fx, &[]);
    let opts = env.opts();
    let req = file_read("big.txt", &fx.ws.to_string_lossy());
    let limits = DedupLimits {
        follower_wait: Duration::from_millis(300),
        ..default_limits()
    };
    let calls = Arc::new(AtomicUsize::new(0));
    let store = store_dir(&fx);
    evaluate_dedup_with(
        &req,
        &opts,
        &limits,
        fake_eval(calls.clone(), Duration::ZERO, "r"),
    )
    .await;
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    let fp = single_fp(&store)?;
    // live leader が mid-eval で死んだ状態 — fresh lock・verdict 無し。
    std::fs::remove_file(record_path(&store, &fp))?;
    let lock = lock_path(&store, &fp);
    std::fs::write(&lock, lock_token_json(9))?;
    let v = evaluate_dedup_with(
        &req,
        &opts,
        &limits,
        fake_eval(calls.clone(), Duration::ZERO, "r"),
    )
    .await;
    assert_eq!(
        calls.load(Ordering::SeqCst),
        2,
        "follower_wait deadline 後に self-eval"
    );
    assert_eq!(
        v,
        Verdict::Deny {
            reason: "r".to_string()
        }
    );
    assert!(lock.exists(), "他人の fresh lock は follower が消さない");
    assert!(
        !record_path(&store, &fp).exists(),
        "self-eval は cache write しない"
    );
    Ok(())
}

/// `live_leader_delete_if_owner` — delete-if-owner の integration 層
/// 観測。primitive の token 照合規則(T1 ≠ T2 → 残存・T1 = T1 → 削除・
/// unparseable → 非削除)は `eval_cache::tests` の unit test
/// (`remove_if_owner_*`)が決定的に固定済み — 本 test は binary 経路の
/// 協調を通じて「live leader が持つ後続 lock を旧 leader 相当の
/// token-mismatch remove が消せない」ことと「unparseable payload の
/// lock が follower に削除されず self-eval へ進む」ことを固定する。
/// 3-party timing race の観察には依存しない。
#[tokio::test(flavor = "current_thread")]
async fn live_leader_delete_if_owner() -> TestResult {
    let fx = Fixture::new();
    fx.write_big("big.txt");
    let env = SeamEnv::new(&fx, &[]);
    let opts = env.opts();
    let req = file_read("big.txt", &fx.ws.to_string_lossy());
    let limits = DedupLimits {
        follower_wait: Duration::from_millis(300),
        ..default_limits()
    };
    let calls = Arc::new(AtomicUsize::new(0));
    let store = store_dir(&fx);
    // (a) leader が自 token と一致する lock を release する — 正常系の
    // delete-if-owner は lock 非残存として観測できる。
    evaluate_dedup_with(
        &req,
        &opts,
        &limits,
        fake_eval(calls.clone(), Duration::ZERO, "r"),
    )
    .await;
    let fp = single_fp(&store)?;
    let lock = lock_path(&store, &fp);
    assert!(!lock.exists(), "leader release は owner token 一致で削除");
    // (b) live leader の eval 中に lock payload が別 token へ交替して
    // いた場合、旧 leader の release(remove_if_owner)は token 不一致
    // で後続 lock を消せない — file が残存する。
    std::fs::remove_file(record_path(&store, &fp))?;
    let calls2 = Arc::new(AtomicUsize::new(0));
    let swapper = {
        let lock = lock.clone();
        async move {
            for _ in 0..2000 {
                if let Ok(body) = std::fs::read_to_string(&lock) {
                    if body.contains("\"nonce\"") {
                        std::fs::write(&lock, lock_token_json(2))?;
                        return Ok::<(), io::Error>(());
                    }
                }
                tokio::time::sleep(Duration::from_millis(5)).await;
            }
            Err(io::Error::new(
                io::ErrorKind::TimedOut,
                "leader lock never observed",
            ))
        }
    };
    let (v2, swapped) = tokio::join!(
        evaluate_dedup_with(
            &req,
            &opts,
            &limits,
            fake_eval(calls2.clone(), Duration::from_millis(500), "r2"),
        ),
        swapper,
    );
    swapped?;
    assert_eq!(calls2.load(Ordering::SeqCst), 1);
    assert_eq!(
        v2,
        Verdict::Deny {
            reason: "r2".to_string()
        }
    );
    let body = std::fs::read_to_string(&lock)?;
    assert!(
        body.contains("\"nonce\":2"),
        "token 不一致の release は後続 lock を消せない: {body}"
    );
    // (c) unparseable payload の fresh lock は follower が削除しない —
    // fresh lock は sweep 対象外(stale 判定は mtime のみ)で、follower
    // 側の delete-if-owner も payload を照合できず除去しないため
    // `follower_wait` deadline の self-eval へ進み lock は残存する。
    // (stale backdated lock は sweep が mtime だけで回収するため、
    // delete-if-owner の unparseable 拒否を経路観測するには fresh を
    // 使う必要がある。`remove_if_owner` 自体の unparseable 拒否は
    // unit test `remove_if_owner_never_deletes_unparseable_lock` が固定。)
    std::fs::remove_file(record_path(&store, &fp))?;
    std::fs::write(&lock, "not a token")?;
    let calls3 = Arc::new(AtomicUsize::new(0));
    let v3 = evaluate_dedup_with(
        &req,
        &opts,
        &limits,
        fake_eval(calls3.clone(), Duration::ZERO, "r3"),
    )
    .await;
    assert_eq!(
        calls3.load(Ordering::SeqCst),
        1,
        "unparseable fresh lock は削除されず deadline で self-eval"
    );
    assert_eq!(
        v3,
        Verdict::Deny {
            reason: "r3".to_string()
        }
    );
    assert_eq!(
        std::fs::read_to_string(&lock)?,
        "not a token",
        "unparseable payload の lock は残存(sweep のみで期限切れ除去)"
    );
    Ok(())
}

// --- AC5: record corruption / fs failure fail-open ------------------------

/// `corrupt_record_fallback` — 非 JSON・未知 `schemaVersion`・
/// `fingerprint` field 不一致・TTL 超過 backdated mtime の record が
/// serve されず通常評価へ fallback。
#[tokio::test(flavor = "current_thread")]
async fn corrupt_record_fallback() -> TestResult {
    let fx = Fixture::new();
    fx.write_big("big.txt");
    let env = SeamEnv::new(&fx, &[]);
    let opts = env.opts();
    let req = file_read("big.txt", &fx.ws.to_string_lossy());
    let limits = default_limits();
    let calls = Arc::new(AtomicUsize::new(0));
    let store = store_dir(&fx);
    evaluate_dedup_with(
        &req,
        &opts,
        &limits,
        fake_eval(calls.clone(), Duration::ZERO, "r"),
    )
    .await;
    let fp = single_fp(&store)?;
    let record = record_path(&store, &fp);
    let valid_allow = format!(
        "{{\"schemaVersion\":1,\"fingerprint\":\"{fp}\",\"verdict\":{{\"decision\":\"allow\"}}}}"
    );
    // 各 corruption は serve されず通常評価へ fallback — leader が
    // fresh+valid な record へ書き直す。planted verdict(allow)が
    // serve されれば Deny との不一致で検出できる。
    for (i, body) in [
        "not json {".to_string(),
        format!(
            "{{\"schemaVersion\":2,\"fingerprint\":\"{fp}\",\"verdict\":{{\"decision\":\"allow\"}}}}"
        ),
        "{\"schemaVersion\":1,\"fingerprint\":\"deadbeefdeadbeef\",\"verdict\":{\"decision\":\"allow\"}}"
            .to_string(),
        valid_allow.clone(),
    ]
    .into_iter()
    .enumerate()
    {
        std::fs::write(&record, &body)?;
        if i == 3 {
            // TTL 超過 — valid record を backdate する。
            set_mtime(&record, SystemTime::now() - Duration::from_secs(3600))?;
        }
        let v = evaluate_dedup_with(
            &req,
            &opts,
            &limits,
            fake_eval(calls.clone(), Duration::ZERO, "r"),
        )
        .await;
        assert_eq!(
            v,
            Verdict::Deny {
                reason: "r".to_string()
            },
            "corrupt record #{i} は serve されず再評価"
        );
        // leader が valid record へ書き直す。
        let written: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&record)?)?;
        assert_eq!(written["fingerprint"], fp.as_str());
        assert_eq!(written["schemaVersion"], 1);
        assert_eq!(written["verdict"]["decision"], "deny");
    }
    assert_eq!(calls.load(Ordering::SeqCst), 5);
    Ok(())
}

/// `copilot_unwritable_cache_exit_zero` — `CHEAPER_CONFIG_DIR` を
/// regular file path へ向け cache dir 作成不能にしても copilot target
/// が常に exit 0 + schema-valid JSON。read-only attribute/ACL は
/// platform・権限依存で flaky のため「dir を期待する path に regular
/// file を置く」方式で portable に失敗させる(observability.rs の
/// `unwritable_log_root` と同一手法)。
#[tokio::test(flavor = "current_thread")]
async fn copilot_unwritable_cache_exit_zero() -> TestResult {
    let fx = Fixture::new();
    fx.write_big("big.txt");
    let cfg_file = Path::new(&fx.env["CHEAPER_CONFIG_DIR"]).with_file_name("cfg-as-file");
    std::fs::write(&cfg_file, b"not a dir")?;
    let cfg = cfg_file.to_string_lossy().into_owned();
    let r = run_hook(
        &fx,
        &["pre-tool-use", "--target", "copilot"],
        "{\"toolName\":\"view\",\"toolArgs\":{\"path\":\"big.txt\"}}",
        &[("CHEAPER_CONFIG_DIR", cfg.as_str())],
    )
    .await;
    assert_eq!(r.code, 0, "copilot never-nonzero: {}", r.stderr);
    let out: serde_json::Value = serde_json::from_str(r.stdout.trim())?;
    assert_eq!(out["permissionDecision"], "deny", "{}", r.stdout);
    Ok(())
}

// --- AC6: TTL / sweep -----------------------------------------------------

/// `future_mtime_not_served` — 未来 mtime の record / lock が serve
/// されず stale 扱い(checked-age `None`)になる。
#[tokio::test(flavor = "current_thread")]
async fn future_mtime_not_served() -> TestResult {
    let fx = Fixture::new();
    fx.write_big("big.txt");
    let env = SeamEnv::new(&fx, &[]);
    let opts = env.opts();
    let req = file_read("big.txt", &fx.ws.to_string_lossy());
    let limits = default_limits();
    let calls = Arc::new(AtomicUsize::new(0));
    let store = store_dir(&fx);
    evaluate_dedup_with(
        &req,
        &opts,
        &limits,
        fake_eval(calls.clone(), Duration::ZERO, "r"),
    )
    .await;
    let fp = single_fp(&store)?;
    let record = record_path(&store, &fp);
    // 未来 mtime の record — checked-age `None` で serve されず
    // (sweep でも除去対象)再評価へ倒れる。
    set_mtime(&record, SystemTime::now() + Duration::from_secs(3600))?;
    evaluate_dedup_with(
        &req,
        &opts,
        &limits,
        fake_eval(calls.clone(), Duration::ZERO, "r"),
    )
    .await;
    assert_eq!(
        calls.load(Ordering::SeqCst),
        2,
        "未来 mtime record は serve されない"
    );
    // 未来 mtime の lock も stale 扱い — takeover で評価完遂。
    std::fs::remove_file(&record)?;
    let lock = lock_path(&store, &fp);
    std::fs::write(&lock, lock_token_json(11))?;
    set_mtime(&lock, SystemTime::now() + Duration::from_secs(3600))?;
    evaluate_dedup_with(
        &req,
        &opts,
        &limits,
        fake_eval(calls.clone(), Duration::ZERO, "r"),
    )
    .await;
    assert_eq!(
        calls.load(Ordering::SeqCst),
        3,
        "未来 mtime lock は stale 扱い"
    );
    assert!(!lock.exists(), "takeover 後の lock は release される");
    Ok(())
}

/// `sweep_removes_stale` — TTL 超過 `.json`・stale `.lock`・orphan
/// `.tmp-*` が次回起動(prepare 先頭の best-effort sweep)で除去される。
#[tokio::test(flavor = "current_thread")]
async fn sweep_removes_stale() -> TestResult {
    let fx = Fixture::new();
    fx.write_big("big.txt");
    let env = SeamEnv::new(&fx, &[]);
    let opts = env.opts();
    let req = file_read("big.txt", &fx.ws.to_string_lossy());
    let limits = default_limits();
    let calls = Arc::new(AtomicUsize::new(0));
    let store = store_dir(&fx);
    evaluate_dedup_with(
        &req,
        &opts,
        &limits,
        fake_eval(calls.clone(), Duration::ZERO, "r"),
    )
    .await;
    let fp = single_fp(&store)?;
    let stale_record = store.join("0123456789abcdef.json");
    let stale_lock = store.join("0123456789abcdef.lock");
    let orphan_tmp = store.join(".0123456789abcdef.json.tmp-1-2");
    let fresh_lock = store.join("ffffffffffffffff.lock");
    let unrelated = store.join("keep.me");
    for p in [
        &stale_record,
        &stale_lock,
        &orphan_tmp,
        &fresh_lock,
        &unrelated,
    ] {
        std::fs::write(p, "x")?;
    }
    let old = SystemTime::now() - Duration::from_secs(3600);
    for p in [&stale_record, &stale_lock, &orphan_tmp] {
        set_mtime(p, old)?;
    }
    // 次回の dedup 経路先頭で sweep — serve path(count 不変)でも発火する。
    evaluate_dedup_with(
        &req,
        &opts,
        &limits,
        fake_eval(calls.clone(), Duration::ZERO, "r"),
    )
    .await;
    assert_eq!(calls.load(Ordering::SeqCst), 1, "fresh record は serve");
    assert!(!stale_record.exists(), "TTL 超過 record を除去");
    assert!(!stale_lock.exists(), "stale lock を除去");
    assert!(!orphan_tmp.exists(), "orphan tmp を除去");
    assert!(fresh_lock.exists(), "fresh lock は残る");
    assert!(unrelated.exists(), "無関係 file は残る");
    assert!(record_path(&store, &fp).exists(), "fresh record は残る");
    Ok(())
}

/// `dir_nested_edit_ttl_bound` — dir target の nested file 編集は dir
/// stat signature(`<dir>` 自身の mtime)に現れないため、TTL 内では
/// 同一 verdict が serve される(staleness bound = TTL + 評価時間で
/// あることを固定 — 即時 invalidation は要求しない)。
#[tokio::test(flavor = "current_thread")]
async fn dir_nested_edit_ttl_bound() -> TestResult {
    let fx = Fixture::new();
    std::fs::create_dir_all(fx.ws.join("sub"))?;
    fx.write("sub/big.txt", big_text().as_bytes());
    let env = SeamEnv::new(&fx, &[]);
    let opts = env.opts();
    let req = RoutingRequest::Search {
        path: "sub".to_string(),
        cwd: fx.ws.to_string_lossy().into_owned(),
        source_tool: Some("test".to_string()),
        pattern: "lorem".to_string(),
        max_results: Some(1),
        context: 0,
        glob: None,
        output_mode: SearchOutputMode::Content,
        case_insensitive: false,
    };
    let limits = default_limits();
    let calls = Arc::new(AtomicUsize::new(0));
    let v1 = evaluate_dedup_with(
        &req,
        &opts,
        &limits,
        fake_eval(calls.clone(), Duration::ZERO, "r"),
    )
    .await;
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    // nested file の内容編集 — dir の mtime は直下 entry の
    // create/delete/rename でのみ進むため signature 不変。
    fx.write("sub/big.txt", b"different nested content");
    let v2 = evaluate_dedup_with(
        &req,
        &opts,
        &limits,
        fake_eval(calls.clone(), Duration::ZERO, "other"),
    )
    .await;
    assert_eq!(
        calls.load(Ordering::SeqCst),
        1,
        "nested 編集は dir signature に現れず同一 verdict を serve"
    );
    assert_eq!(v2, v1);
    Ok(())
}

/// `no_workspace_writes` — dedup 状態(`hook-eval/`・`.cheaper/`)は
/// workspace 内に一切書かれない(R8 — `<ws>` 配下に新規 file/dir が
/// 無いことを assert)。
#[tokio::test(flavor = "current_thread")]
async fn no_workspace_writes() -> TestResult {
    fn tree(root: &Path) -> Vec<String> {
        let mut out = Vec::new();
        let mut stack = vec![root.to_path_buf()];
        while let Some(dir) = stack.pop() {
            if let Ok(rd) = std::fs::read_dir(&dir) {
                for entry in rd.flatten() {
                    let p = entry.path();
                    if let Ok(rel) = p.strip_prefix(root) {
                        out.push(rel.to_string_lossy().into_owned());
                    }
                    if p.is_dir() {
                        stack.push(p);
                    }
                }
            }
        }
        out.sort();
        out
    }
    let fx = Fixture::new();
    fx.write_big("big.txt");
    let before = tree(&fx.ws);
    let r = run_hook(
        &fx,
        &["pre-read-code", "--target", "windsurf"],
        &windsurf_read("big.txt"),
        &[],
    )
    .await;
    assert_eq!(r.code, 2);
    let after = tree(&fx.ws);
    assert_eq!(
        before, after,
        "workspace 配下に新規 file/dir が無い(hook-eval/ 等)"
    );
    assert_eq!(after, vec!["big.txt".to_string()]);
    Ok(())
}
