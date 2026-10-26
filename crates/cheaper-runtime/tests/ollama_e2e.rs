//! Real-provider e2e gate(Phase 3 最終 gap)— Ollama `openai-completions`
//! endpoint + built-in worker 3 種(bulk-reader / code-writer / doc-writer)
//! が `run_worker` 経路で成功することを検証する。
//!
//! `packages/cli/test/e2e/real-provider-ollama.test.ts` の parity:
//! `CHEAPER_E2E_OLLAMA=1` が無い・`/v1/models` が 3s 以内に応答しない・
//! model が未 pull のときは全 test を早期 return で skip する(失敗させない)。
//!
//! - model: `CHEAPER_E2E_OLLAMA_MODEL`(default `qwen3.5:9b`)
//! - base : `CHEAPER_E2E_OLLAMA_BASE`(default `http://localhost:11434`)
//!
//! 本 test は production session 経路(`RigSessionFactory` → Rig `Agent` →
//! provider wire)を通る唯一の gate であり、session seam は
//! `CompletionModelFactory` の test-local 実装が
//! `cheaper_rig_adapter::builtin_completion_model` を呼ぶだけの形に留める
//! — この関数は Phase 4 CLI composition root が再利用する seam。

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use cheaper_core::errors::CheaperError;
use cheaper_core::model::config::{ModelConfig, TierPreferences};
use cheaper_core::model::tier::Tier;
use cheaper_core::policy::exec::{ExecDefaults, ExecHeartbeat, ResolvedExecPolicy};
use cheaper_core::policy::schema::{
    ExecutionPolicy, FilesystemPolicy, ModelPolicy, OutputPolicy, RetryPolicy, RoutePolicy,
    RuntimePolicy, TierFallback, ToolsPolicy,
};
use cheaper_core::policy::security::SecurityPolicy;
use cheaper_core::policy::workflow::{WorkflowArtifactPolicy, WorkflowPolicy};
use cheaper_core::tools::vocabulary::ToolName;
use cheaper_core::worker::parse::parse_worker_file;
use cheaper_core::worker::validate::{validate_worker_definition, WorkerDefinition, WorkerOrigin};
use cheaper_core::workspace::paths::SymlinkPolicy;
use cheaper_rig_adapter::{
    builtin_completion_model, BuiltinAuth, BuiltinModelSpec, CompletionModel,
};
use cheaper_runtime::auth::api_key::ProviderEnv;
use cheaper_runtime::run::{
    run_worker, CompletionModelFactory, RigSessionFactory, RunWorkerDeps, RunWorkerOk, SessionSpec,
};
use cheaper_runtime::Runtime;
use serde_json::{json, Value};

// ---------------------------------------------------------------------------
// gate
// ---------------------------------------------------------------------------

fn ollama_base() -> String {
    std::env::var("CHEAPER_E2E_OLLAMA_BASE")
        .unwrap_or_else(|_| "http://localhost:11434".to_string())
}

fn e2e_model() -> String {
    std::env::var("CHEAPER_E2E_OLLAMA_MODEL").unwrap_or_else(|_| "qwen3.5:9b".to_string())
}

/// TS `probeOllama` 相当 — 3s timeout で `/v1/models` を叩き、model id が
/// 一覧にあれば true。`CHEAPER_E2E_OLLAMA=1` が無い場合は probe 自体を
/// 行わず skip する。
fn ollama_available() -> bool {
    if std::env::var("CHEAPER_E2E_OLLAMA").as_deref() != Ok("1") {
        return false;
    }
    let base = ollama_base();
    let model = e2e_model();
    let res = ureq::get(&format!("{base}/v1/models"))
        .timeout(Duration::from_secs(3))
        .call();
    let Ok(res) = res else {
        return false;
    };
    // `ureq` の `json` feature は有効化していないため text → serde_json で parse。
    let body: Value = match res
        .into_string()
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
    {
        Some(v) => v,
        None => return false,
    };
    body["data"]
        .as_array()
        .map(|models| {
            models.iter().filter_map(|m| m["id"].as_str()).any(|id| {
                id == model
                    || id.starts_with(&format!("{model}:"))
                    || id == format!("{model}:latest")
            })
        })
        .unwrap_or(false)
}

fn skip_reason() -> Option<&'static str> {
    if ollama_available() {
        None
    } else {
        Some("Ollama unreachable or CHEAPER_E2E_OLLAMA unset; see AGENTS.md")
    }
}

// ---------------------------------------------------------------------------
// test-local CompletionModelFactory — Phase 4 composition root が再利用する
// `builtin_completion_model` seam をここで先に exerciser として使う。
// ---------------------------------------------------------------------------

struct BuiltinFactory;

impl CompletionModelFactory for BuiltinFactory {
    fn completion_model(&self, spec: &SessionSpec<'_>) -> Result<CompletionModel, CheaperError> {
        builtin_completion_model(&BuiltinModelSpec {
            api: &spec.model.api,
            model_id: &spec.model.id,
            base_url: &spec.model.base_url,
            max_tokens: Some(spec.model.max_tokens as u64),
            auth: spec.auth.map(|a| BuiltinAuth {
                api_key: a.api_key.clone(),
                base_url: a.base_url.clone(),
                headers: a.headers.clone(),
            }),
        })
    }
}

// ---------------------------------------------------------------------------
// fixture — tmpdir userConfigDir(models.json custom provider)+ workspace
// ---------------------------------------------------------------------------

struct MapEnv;
impl ProviderEnv for MapEnv {
    fn env(&self, _name: &str) -> Option<String> {
        None
    }
    fn file_exists(&self, _path: &str) -> bool {
        false
    }
    fn home_dir(&self) -> Option<PathBuf> {
        None
    }
}

/// repo 内の canonical worker 定義(TS 側 bundled worker と同一 file を読む —
/// differential parity)。schema file ref `./schemas/…` は worker の dir から
/// 相対解決されるため、そのままの配置で load できる。
fn repo_worker(name: &str) -> WorkerDefinition {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../cheaper-cli/resources/workers")
        .join(format!("{name}.md"));
    let path = path.canonicalize().expect("worker file must exist");
    let content = std::fs::read_to_string(&path).expect("read worker");
    let source = path.to_string_lossy().to_string();
    let parsed = parse_worker_file(&content, &source).expect("parse worker");
    validate_worker_definition(&parsed, &source, WorkerOrigin::Builtin).expect("validate worker")
}

struct E2eFixture {
    _dir: tempfile::TempDir,
    root: PathBuf,
    runtime: Runtime,
    policy: RuntimePolicy,
    model_config: ModelConfig,
}

/// user catalog: models.json に `ollama` custom provider(api
/// `openai-completions`・baseUrl `{base}/v1`・literal `apiKey`)を定義する。
/// TS e2e の recipe(IMPL-0008)と同じ shape。
fn fixture() -> E2eFixture {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("workspace");
    let config_dir = dir.path().join("config");
    std::fs::create_dir_all(&root).unwrap();
    std::fs::create_dir_all(&config_dir).unwrap();

    let model = e2e_model();
    let models_json = json!({
        "providers": {
            "ollama": {
                "name": "Ollama (local)",
                "api": "openai-completions",
                "baseUrl": format!("{}/v1", ollama_base()),
                "apiKey": "ollama",
                "models": [{
                    "id": model,
                    "name": format!("{model} (e2e)"),
                    "reasoning": true,
                    "cost": { "input": 0, "output": 0, "cacheRead": 0, "cacheWrite": 0 },
                    "contextWindow": 262144,
                    "maxTokens": 32768,
                    // TS e2e の reasoning_effort:"none" / temperature:0 相当。
                    // 現状 builtin factory は samplingParams を wire へ載せない
                    // (residual — request 単位 additional_params は session 側)。
                    "samplingParams": { "reasoning_effort": "none", "temperature": 0 }
                }]
            }
        }
    });
    std::fs::write(
        config_dir.join("models.json"),
        serde_json::to_string_pretty(&models_json).unwrap(),
    )
    .unwrap();

    let runtime = Runtime::create(config_dir, Arc::new(MapEnv), &[], BTreeMap::new()).unwrap();

    // TS models.yaml 相当 — 9b は built-in 分類表に match しないため
    // override で low に pin する(explicit_model 経路でも tier 解決が走る)。
    let key = format!("ollama/{model}");
    let model_config = ModelConfig {
        overrides: vec![(key.clone(), Tier::Low)],
        preferences: TierPreferences {
            low: Some(vec![key]),
            ..TierPreferences::default()
        },
        exclude: None,
    };

    // TS e2e の policy.yaml 相当(timeout 10m / maxTurns 16 / repairAttempts 4)。
    let policy = RuntimePolicy {
        workflow: WorkflowPolicy {
            max_jobs: 4,
            artifacts: WorkflowArtifactPolicy {
                max_artifact_bytes: 4096,
                max_total_bytes: 16384,
                max_preview_bytes: 4096,
                max_read_bytes: 4096,
            },
        },
        exec: ResolvedExecPolicy {
            grants: BTreeMap::new(),
            process: ExecDefaults {
                max_concurrent: 4,
                timeout: "10m".into(),
                max_output_bytes: 4096,
                kill_grace: "2s".into(),
                heartbeat: ExecHeartbeat {
                    interval: "30s".into(),
                },
            },
            command: ExecDefaults {
                max_concurrent: 4,
                timeout: "10m".into(),
                max_output_bytes: 4096,
                kill_grace: "2s".into(),
                heartbeat: ExecHeartbeat {
                    interval: "30s".into(),
                },
            },
        },
        route: RoutePolicy::default(),
        security: SecurityPolicy::default(),
        execution: ExecutionPolicy {
            timeout_ms: 600_000,
            max_turns: Some(16),
            max_tool_calls: 50,
            concurrency: None,
        },
        tools: ToolsPolicy {
            // built-in worker 3 種の tools union(process は使わない)。
            allow: vec![
                ToolName::Read,
                ToolName::Grep,
                ToolName::Find,
                ToolName::Ls,
                ToolName::Write,
                ToolName::Edit,
                ToolName::CountTokens,
            ],
        },
        filesystem: FilesystemPolicy {
            symlinks: SymlinkPolicy::Deny,
            deny_paths: vec![],
        },
        model: ModelPolicy {
            max_tier: Tier::High,
            tier_fallback: TierFallback::Higher,
        },
        output: OutputPolicy {
            max_tokens: 8192,
            repair_attempts: 4,
        },
        retry: RetryPolicy {
            transient_attempts: 2,
        },
    };

    E2eFixture {
        _dir: dir,
        root,
        runtime,
        policy,
        model_config,
    }
}

async fn run(fx: &E2eFixture, worker: &WorkerDefinition, input: Value) -> RunWorkerOk {
    let factory = RigSessionFactory::new(BuiltinFactory);
    let model_ref = format!("ollama/{}", e2e_model());
    let deps = RunWorkerDeps {
        worker,
        input: &input,
        root: fx.root.to_str().unwrap().to_string(),
        policy: &fx.policy,
        model_config: &fx.model_config,
        runtime: &fx.runtime,
        session_factory: &factory,
        explicit_model: Some(model_ref.as_str()),
        signal: None,
        abort_grace_ms: None,
        execution: None,
        operation_clock: None,
        invocation_grants: &[],
        resume: None,
        execution_correlation: None,
    };
    let started = Instant::now();
    let outcome = run_worker(&deps).await;
    let elapsed = started.elapsed();
    match outcome {
        Ok(ok) => {
            eprintln!(
                "[ollama-e2e] {} ok: turns={} toolCalls={} usage(in={},out={},total={}) \
                 repairs={} transientRetries={} duration={:.1}s",
                worker.name,
                ok.meta.turns,
                ok.meta.tool_calls,
                ok.meta.usage.input,
                ok.meta.usage.output,
                ok.meta.usage.total_tokens,
                ok.meta.repairs,
                ok.meta.transient_retries,
                elapsed.as_secs_f64(),
            );
            ok
        }
        Err(e) => panic!(
            "worker {} failed after {:.1}s: {}",
            worker.name,
            elapsed.as_secs_f64(),
            e
        ),
    }
}

/// success envelope 相当の assert — result が schema-valid で返り、
/// usage/model meta が実測値を持つ。
fn assert_real_envelope(ok: &RunWorkerOk, provider: &str, model: &str) {
    assert_eq!(ok.meta.model.provider, provider);
    assert_eq!(ok.meta.model.id, model);
    assert!(ok.meta.usage.input > 0, "real usage metrics present");
    assert!(ok.meta.usage.output > 0, "real usage metrics present");
    assert!(ok.meta.turns >= 1);
}

// ---------------------------------------------------------------------------
// tests — built-in worker 3 種
// ---------------------------------------------------------------------------

#[tokio::test]
async fn bulk_reader_reaches_ollama_and_returns_findings() {
    if let Some(reason) = skip_reason() {
        eprintln!("skip: {reason}");
        return;
    }
    let fx = fixture();
    std::fs::create_dir_all(fx.root.join("src")).unwrap();
    std::fs::write(
        fx.root.join("src/greeter.ts"),
        "export function greet(name: string): string { return \"hi \" + name; }\nexport const DEFAULT_GREETING = \"hi\";\n",
    )
    .unwrap();
    let worker = repo_worker("bulk-reader");
    let ok = run(
        &fx,
        &worker,
        json!({
            "task": "List the exported names in src/greeter.ts",
            "paths": ["src/greeter.ts"],
            "maxFindings": 3
        }),
    )
    .await;
    assert_real_envelope(&ok, "ollama", &e2e_model());
    assert!(
        ok.result["findings"].is_array(),
        "findings array: {:?}",
        ok.result
    );
}

#[tokio::test]
async fn code_writer_reaches_ollama_and_creates_file() {
    if let Some(reason) = skip_reason() {
        eprintln!("skip: {reason}");
        return;
    }
    let fx = fixture();
    let worker = repo_worker("code-writer");
    let ok = run(
        &fx,
        &worker,
        json!({
            "task": "Create src/farewell.ts exporting a farewell() function that returns a goodbye string.",
            "targets": [{ "path": "src/farewell.ts", "intent": "create" }]
        }),
    )
    .await;
    assert_real_envelope(&ok, "ollama", &e2e_model());
    assert!(!ok.meta.changes.files.is_empty(), "changes.files non-empty");
    assert!(fx.root.join("src/farewell.ts").exists());
}

#[tokio::test]
async fn doc_writer_reaches_ollama_and_creates_document() {
    if let Some(reason) = skip_reason() {
        eprintln!("skip: {reason}");
        return;
    }
    let fx = fixture();
    let worker = repo_worker("doc-writer");
    let ok = run(
        &fx,
        &worker,
        json!({
            "task": "Write a short readme describing this workspace.",
            "documentType": "readme",
            "targets": [{ "path": "docs/hello.md", "intent": "create" }]
        }),
    )
    .await;
    assert_real_envelope(&ok, "ollama", &e2e_model());
    assert!(fx.root.join("docs/hello.md").exists());
}
