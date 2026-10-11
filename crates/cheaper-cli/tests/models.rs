//! `cheaper models`(T3)の in-process test — `executeModels` の
//! envelope shape・`--provider`/`--tier`/`--worker` filter を
//! `run_cli` 経路で検証する。実 credential 不要(catalog の configured
//! 判定が空でも envelope 契約は同じ)。

use std::collections::BTreeMap;
use std::path::Path;

use cheaper_cli::cli::{run_cli, CliHooks, CliIo, CliStdin, CliWrite};
use serde_json::{json, Value};

struct VecSink(Vec<u8>);
impl CliWrite for VecSink {
    fn write_str(&mut self, s: &str) {
        self.0.extend_from_slice(s.as_bytes());
    }
}

struct PipeStdin;
impl CliStdin for PipeStdin {
    fn is_tty(&self) -> bool {
        true
    }
    fn read_all(&mut self) -> std::io::Result<Vec<u8>> {
        Ok(Vec::new())
    }
}

/// ambient env fallback — `CHEAPER_LOG*` は絶対に継承しない
/// (in-process test binary で global subscriber を install すると
/// 後続 test が "subscriber already installed" になり stray
/// .cheaper/logs を残す)。
fn ambient_env(name: &str) -> Option<String> {
    if name.starts_with("CHEAPER_LOG") {
        return None;
    }
    std::env::var(name).ok()
}

struct Fixture {
    ws: tempfile::TempDir,
    /// `CHEAPER_CONFIG_DIR` 先の keep-alive(field 保持で temp dir 生存)。
    _cfg: tempfile::TempDir,
    env_map: BTreeMap<String, String>,
    stdin: PipeStdin,
    out: VecSink,
    err: VecSink,
}

impl Fixture {
    fn new() -> Self {
        let ws = tempfile::tempdir().unwrap();
        let cfg = tempfile::tempdir().unwrap();
        let mut env_map = BTreeMap::new();
        env_map.insert(
            "CHEAPER_CONFIG_DIR".to_string(),
            cfg.path().to_string_lossy().into_owned(),
        );
        Self {
            ws,
            _cfg: cfg,
            env_map,
            stdin: PipeStdin,
            out: VecSink(Vec::new()),
            err: VecSink(Vec::new()),
        }
    }

    /// `providers/devin/catalog.json` に malformed snapshot を配置する
    /// — `Runtime::create` の snapshot load error が catalog errors に
    /// 記録され `warning:` 行として stderr に出る fixture。
    fn write_malformed_devin_catalog(&self) {
        let provider_dir = self._cfg.path().join("providers").join("devin");
        std::fs::create_dir_all(&provider_dir).unwrap();
        std::fs::write(provider_dir.join("catalog.json"), "{ not json").unwrap();
    }

    async fn run(&mut self, argv: &[&str]) -> (i32, Value, String) {
        let env_map = &self.env_map;
        let env_lookup = move |name: &str| env_map.get(name).cloned().or_else(|| ambient_env(name));
        let cwd = self.ws.path().to_string_lossy().into_owned();
        let mut io = CliIo {
            stdin: &mut self.stdin,
            stdout: &mut self.out,
            stderr: &mut self.err,
            env: &env_lookup,
            cwd,
        };
        let hooks = CliHooks {
            builtin_dir: Some(Path::new(env!("CARGO_MANIFEST_DIR")).join("resources")),
            ..Default::default()
        };
        let code = run_cli(
            &argv.iter().map(|s| s.to_string()).collect::<Vec<_>>(),
            &mut io,
            &hooks,
        )
        .await;
        drop(io);
        (
            code,
            serde_json::from_str(&String::from_utf8_lossy(&self.out.0)).unwrap(),
            String::from_utf8_lossy(&self.err.0).into_owned(),
        )
    }
}

#[tokio::test(flavor = "current_thread")]
async fn models_utility_envelope_shape() {
    let mut f = Fixture::new();
    let (code, env, err) = f.run(&["models"]).await;
    assert_eq!(code, 0, "stderr: {err}");
    // utility envelope — `{schemaVersion, ok, worker:null, result, meta}`。
    assert_eq!(env["schemaVersion"], json!(1));
    assert_eq!(env["ok"], json!(true));
    assert_eq!(env["worker"], Value::Null);
    assert!(env["result"]["models"].is_array());
    assert!(env["meta"]["durationMs"].is_number());
    // 各 entry の必須 key(TS object literal 構築順)。
    for m in env["result"]["models"].as_array().unwrap() {
        assert!(m["provider"].is_string(), "{m}");
        assert!(m["id"].is_string(), "{m}");
        assert!(m["excluded"].is_boolean(), "{m}");
        assert!(m["tier"].is_string(), "{m}");
        assert!(m["tierSource"].is_string(), "{m}");
        assert!(m.get("pricing").is_some(), "{m}");
    }
}

#[tokio::test(flavor = "current_thread")]
async fn models_provider_filter_matches_output() {
    let mut f = Fixture::new();
    let (_code, env, _err) = f.run(&["models", "--provider", "no-such-provider"]).await;
    // 未存在 provider → 空配列。
    assert_eq!(env["result"]["models"], json!([]));
}

#[tokio::test(flavor = "current_thread")]
async fn models_provider_filter_keeps_population_and_filters_rows() {
    // catalog に model がある場合のみ検証できるため、先に全量を取る。
    let mut f = Fixture::new();
    let (_c, env, _e) = f.run(&["models"]).await;
    let models = env["result"]["models"].as_array().unwrap();
    let Some(first) = models.first() else {
        // configured provider が無い環境では空 — provider filter が
        // 空配列を返すことだけ確認(filter は throw しない)。
        return;
    };
    let provider = first["provider"].as_str().unwrap().to_string();
    let (_c, env2, _e) = f.run(&["models", "--provider", &provider]).await;
    let filtered = env2["result"]["models"].as_array().unwrap();
    assert!(!filtered.is_empty());
    for m in filtered {
        assert_eq!(m["provider"], json!(provider));
    }
    assert_eq!(
        filtered.len(),
        models.iter().filter(|m| m["provider"] == provider).count()
    );
}

#[tokio::test(flavor = "current_thread")]
async fn models_tier_filter() {
    let mut f = Fixture::new();
    let (_c, env, _e) = f.run(&["models", "--tier", "high"]).await;
    let models = env["result"]["models"].as_array().unwrap();
    for m in models {
        assert_eq!(m["tier"], json!("high"));
    }
}

#[tokio::test(flavor = "current_thread")]
async fn models_worker_option_resolves_builtin_worker() {
    // `--worker bulk-reader` — builtin worker の workload
    // (outputPerInput 0.05)が auto 分類 context に入る。worker が
    // resolve されれば error せず成功する(name 不正/不存在は
    // resolve_worker が error にする経路の担保)。
    let mut f = Fixture::new();
    let (code, env, err) = f.run(&["models", "--worker", "bulk-reader"]).await;
    assert_eq!(code, 0, "stderr: {err}");
    assert!(env["result"]["models"].is_array());
}

#[tokio::test(flavor = "current_thread")]
async fn models_worker_not_found_errors() {
    let mut f = Fixture::new();
    let (code, env, _err) = f.run(&["models", "--worker", "no-such-worker"]).await;
    assert_eq!(env["ok"], json!(false));
    assert_eq!(env["error"]["code"], json!("WORKER_NOT_FOUND"));
    assert_ne!(code, 0);
}

// ---------------------------------------------------------------------------
// catalog error の stderr surfacing(spec Req 4)
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "current_thread")]
async fn models_emits_catalog_warnings_for_malformed_plugin_catalog() {
    // spec Req 4 — snapshot load error は `warning:` 行として stderr へ
    // 出る(emit は fallible `get_available` より先)。envelope
    // `result` shape・`schemaVersion`・exit code は不変。
    let mut f = Fixture::new();
    f.write_malformed_devin_catalog();
    let (code, env, err) = f.run(&["models"]).await;
    assert_eq!(code, 0, "stderr: {err}");
    assert_eq!(env["schemaVersion"], json!(1));
    assert_eq!(env["ok"], json!(true));
    assert!(env["result"]["models"].is_array());
    assert!(
        err.contains("warning: Provider \"devin\":"),
        "stderr: {err}"
    );
    assert!(err.contains("plugin catalog"), "stderr: {err}");
}

#[tokio::test(flavor = "current_thread")]
async fn models_quiet_suppresses_catalog_warnings() {
    let mut f = Fixture::new();
    f.write_malformed_devin_catalog();
    let (code, env, err) = f.run(&["models", "--quiet"]).await;
    assert_eq!(code, 0, "stderr: {err}");
    assert_eq!(env["ok"], json!(true));
    assert!(!err.contains("warning:"), "stderr: {err}");
}
