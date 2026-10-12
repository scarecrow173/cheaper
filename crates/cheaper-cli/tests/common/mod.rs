//! cheaper-cli integration test 共通 helper。
//!
#![allow(dead_code)]

//! 各 test file で重複していた `VecSink`/`PipeStdin`/`ambient_env`/
//! `Fixture` を集約する。seam 依存 helper(`with_fake_stream`)は
//! env_map 操作のみなので feature-gate せず共通提供する(feature off
//! では fixture file は絶対に読まれない)。

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use cheaper_cli::cli::{run_cli, CliHooks, CliIo, CliStdin, CliWrite};
use serde_json::Value;

/// `Vec<u8>` 直保持の stdout/stderr sink。
pub struct VecSink(pub Vec<u8>);
impl CliWrite for VecSink {
    fn write_str(&mut self, s: &str) {
        self.0.extend_from_slice(s.as_bytes());
    }
}

/// `Rc<RefCell<Vec<u8>>>` の shared sink — `CliIo` drop 後も caller 側の
/// Rc handle から内容を読む経路(usage_report 系)用。
pub struct SharedVecSink(pub std::rc::Rc<std::cell::RefCell<Vec<u8>>>);
impl CliWrite for SharedVecSink {
    fn write_str(&mut self, s: &str) {
        self.0.borrow_mut().extend_from_slice(s.as_bytes());
    }
}

/// pipe stdin — `read_all` は `data` を消費する(mem::take)。
pub struct PipeStdin {
    pub data: Vec<u8>,
    pub tty: bool,
}
impl PipeStdin {
    pub fn empty(tty: bool) -> Self {
        Self {
            data: Vec::new(),
            tty,
        }
    }
}
impl CliStdin for PipeStdin {
    fn is_tty(&self) -> bool {
        self.tty
    }
    fn read_all(&mut self) -> std::io::Result<Vec<u8>> {
        Ok(std::mem::take(&mut self.data))
    }
}

/// ambient env fallback — `CHEAPER_LOG*` は絶対に継承しない
/// (in-process test binary で global subscriber を install すると
/// 後続 test が "subscriber already installed" になり stray
/// .cheaper/logs を残す)。
pub fn ambient_env(name: &str) -> Option<String> {
    if name.starts_with("CHEAPER_LOG") {
        return None;
    }
    std::env::var(name).ok()
}

/// crate 内 bundled resource dir — embedded materialize を避け、
/// workers/ + policy.yaml が同一 layout で存在する resource root
/// (TS test の `builtinDir: cliRoot` 相当)。
pub fn builtin_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("resources")
}

/// `builtin_dir` 指定済みの default hooks。
pub fn default_hooks() -> CliHooks {
    CliHooks {
        builtin_dir: Some(builtin_dir()),
        ..Default::default()
    }
}

/// `env_map` → ambient env fallback の lookup closure。
pub fn env_lookup(env_map: &BTreeMap<String, String>) -> impl Fn(&str) -> Option<String> + '_ {
    move |name| env_map.get(name).cloned().or_else(|| ambient_env(name))
}

/// test 用組立一式。`env_map` は `io.env` 経由で lookup され、
/// 未設定名は process env へ fallback する(`CHEAPER_LOG*` 除く)。
pub struct Fixture {
    pub ws: tempfile::TempDir,
    /// `CHEAPER_CONFIG_DIR` 先の temp dir。`env_map` からのみ参照
    /// されるため field 自体は read されない(drop まで生存必須)。
    pub cfg: tempfile::TempDir,
    pub env_map: BTreeMap<String, String>,
    pub stdin: PipeStdin,
    pub out: VecSink,
    pub err: VecSink,
    pub hooks: CliHooks,
}

impl Fixture {
    pub fn new() -> Self {
        let ws = tempfile::tempdir().unwrap();
        let cfg = tempfile::tempdir().unwrap();
        let mut env_map = BTreeMap::new();
        env_map.insert(
            "CHEAPER_CONFIG_DIR".to_string(),
            cfg.path().to_string_lossy().into_owned(),
        );
        Self {
            ws,
            cfg,
            env_map,
            stdin: PipeStdin::empty(false),
            out: VecSink(Vec::new()),
            err: VecSink(Vec::new()),
            hooks: default_hooks(),
        }
    }

    /// stdin へ input JSON(bytes)を流す builder。
    pub fn with_input(mut self, input_json: &str) -> Self {
        self.stdin.data = input_json.as_bytes().to_vec();
        self
    }

    /// fake-stream fixture を `fixture.json` へ書き、
    /// `NODE_ENV=test` + `CHEAPER_TEST_FAKE_STREAM` を env_map へ登録
    /// する builder。
    pub fn with_fake_stream(mut self, fixture_json: &str) -> Self {
        let path = self.ws.path().join("fixture.json");
        std::fs::write(&path, fixture_json).unwrap();
        self.env_map
            .insert("NODE_ENV".to_string(), "test".to_string());
        self.env_map.insert(
            "CHEAPER_TEST_FAKE_STREAM".to_string(),
            path.to_string_lossy().into_owned(),
        );
        self
    }

    /// workflow definition を `workflow.json` へ書き込む builder。
    pub fn with_workflow(self, definition: Value) -> Self {
        let path = self.ws.path().join("workflow.json");
        std::fs::write(&path, serde_json::to_string(&definition).unwrap()).unwrap();
        self
    }

    /// `providers/devin/catalog.json` に malformed snapshot を配置する
    /// — `Runtime::create` の snapshot load error が catalog errors に
    /// 記録され `warning:` 行として stderr に出る fixture。
    pub fn write_malformed_devin_catalog(&self) {
        let provider_dir = self.cfg.path().join("providers").join("devin");
        std::fs::create_dir_all(&provider_dir).unwrap();
        std::fs::write(provider_dir.join("catalog.json"), "{ not json").unwrap();
    }

    /// `<cfg>/usage.jsonl` の内容。
    pub fn usage_log(&self) -> Option<String> {
        std::fs::read_to_string(self.cfg.path().join("usage.jsonl")).ok()
    }

    pub fn ws_path(&self) -> PathBuf {
        self.ws.path().to_path_buf()
    }

    /// `run_cli` を実行し `(exit code, stdout envelope Value, stderr)`
    /// を返す。stdout は JSON envelope 前提(`from_str` unwrap)。
    pub async fn run(&mut self, argv: &[&str]) -> (i32, Value, String) {
        let env_lookup = env_lookup(&self.env_map);
        let cwd = self.ws.path().to_string_lossy().into_owned();
        let mut io = CliIo {
            stdin: &mut self.stdin,
            stdout: &mut self.out,
            stderr: &mut self.err,
            env: &env_lookup,
            cwd,
        };
        let code = run_cli(
            &argv.iter().map(|s| s.to_string()).collect::<Vec<_>>(),
            &mut io,
            &self.hooks,
        )
        .await;
        drop(io);
        let out_str = String::from_utf8_lossy(&self.out.0).into_owned();
        let err_str = String::from_utf8_lossy(&self.err.0).into_owned();
        (code, serde_json::from_str(&out_str).unwrap(), err_str)
    }
}
