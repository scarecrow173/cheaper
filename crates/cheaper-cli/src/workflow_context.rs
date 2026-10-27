//! `workflow-context.ts` の compiler factory 組立 seam(T7):
//! builtin dir → workspace root canonicalize → config dirs → policy →
//! `loadWorkflowFile`(worker 事前解込込み compiler factory)→ tool
//! catalog → `WorkspaceResourceInference`。
//!
//! TS との構造差分: TS は `prepareWorkflowContext` が
//! `loadWorkflowFile` まで一度に済ませて `{root, dirs, policy, loaded,
//! workers}` を返す。Rust では `WorkflowCompiler` が ports への借用を
//! 保持する(`WorkflowCompiler<'a>`)ため、self-referential 構造を避ける
//! 形に分割する — `prepare_workflow_context` は root/dirs/policy/tool
//! catalog/inference を所有する [`WorkflowContext`] を組み立て、file
//! read + worker 事前解決 + compiler 構築は [`WorkflowContext::
//! load_workflow`] が行う(呼出し側は `LoadedWorkflow<'ctx>` を介して
//! compiler を使う)。tool catalog は resolved process policy からの
//! 直接構築(`ToolCatalog::from_resolved_policy`)で失敗し得ないため
//! factory 内ではなく context 側で組み立てる。

use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, OnceLock};

use cheaper_core::config::dirs::ConfigDirs;
use cheaper_core::errors::ErrorCode;
use cheaper_core::js::code_unit_compare;
use cheaper_core::policy::load::{load_policy, LoadPolicyOptions};
use cheaper_core::policy::schema::RuntimePolicy;
use cheaper_core::worker::resolve::{resolve_worker, ResolveWorkerError};
use cheaper_core::worker::validate::WorkerDefinition;
use cheaper_core::workspace::deny_paths::create_path_denylist;
use cheaper_core::workspace::paths::SymlinkPolicy;
use cheaper_tools::catalog::ToolCatalog;
use cheaper_tools::ToolError;
use cheaper_workflow::{
    load_workflow_document, load_workflow_file, LoadedWorkflow, ResourceInference,
    ResourceInferenceJob, ResourceInferenceResult, WorkerCatalog, WorkflowCompiler,
    WorkflowCompilerPorts, WorkflowFileOptions, WorkspaceResourceInference,
};
use serde_json::{Map, Value};

use crate::cli::{BoxError, CliHooks, CliIo};

/// `failures.get(name)` throw を `ToolError` で共有するための器。
/// `Box<dyn Error>` は Clone 不可のため `Arc` で持ち、`source()` を
/// 内側へ透過させる — engine の diagnostic(`find_cheaper_error` の
/// cause chain walk)が内部の `CheaperError`/`WorkerReadError` に到達
/// できる(TS は同一 error object を再 throw する)。
#[derive(Clone)]
pub(crate) struct SharedToolError(Arc<ToolError>);

impl SharedToolError {
    pub(crate) fn new(error: ToolError) -> Self {
        Self(Arc::new(error))
    }

    /// `throw failures.get(name)` 相当 — 新しい box として返す。
    pub(crate) fn boxed(&self) -> ToolError {
        Box::new(self.clone())
    }

    /// 内側の error(`failures.get(name)` の保持値)への参照。
    /// envelope 側の `isCheaperError` 判定で wrapper を剥がすために使う
    /// (TS は同一 error object を再 throw するため wrapper 自体が
    /// 見えない)。
    pub(crate) fn inner(&self) -> &(dyn Error + 'static) {
        self.0.as_ref().as_ref()
    }
}

impl std::fmt::Debug for SharedToolError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(&self.0, f)
    }
}

impl std::fmt::Display for SharedToolError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(&self.0, f)
    }
}

impl Error for SharedToolError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        // wrapper 自身ではなく内側の error を source として返す
        // (`WorkerRunError` と同じ idiom)。`Box<dyn Error>` の
        // `source()` にそのまま委譲すると内側 error 自身が skip され、
        // `find_cheaper_error` の cause chain walk が `failures.get`
        // に積まれた `CheaperError` へ到達できない。
        Some(self.inner())
    }
}

/// TS `workers`/`failures` の組み合わせ — `WorkerCatalog` port は
/// `failures` を throw し `workers` hit を返す。`worker(name)` は
/// executor/lease 経路の `workers.get(name)` 相当で resolved 定義のみ
/// を見る(`failures` は compile 時に既に `workers.get` throw として
/// 伝播済みであり、engine の WORKER_NOT_FOUND 分岐へ到達しない)。
pub(crate) struct WorkerIndex {
    workers: HashMap<String, Arc<WorkerDefinition>>,
    failures: HashMap<String, SharedToolError>,
}

impl WorkerIndex {
    /// `workers.get(name)`(TS `workers.get` — failure は含まない)。
    pub(crate) fn worker(&self, name: &str) -> Option<Arc<WorkerDefinition>> {
        self.workers.get(name).cloned()
    }
}

impl WorkerCatalog for WorkerIndex {
    /// `workers: {get(name) { if (failures.has(name)) throw
    /// failures.get(name); return workers.get(name); }}`。
    fn get(&self, name: &str) -> Result<Option<&WorkerDefinition>, ToolError> {
        if let Some(failure) = self.failures.get(name) {
            return Err(failure.boxed());
        }
        Ok(self.workers.get(name).map(Arc::as_ref))
    }
}

/// `createCompiler` 内の worker 事前解決:
/// `jobs` object の `kind === "worker"` entry から worker 名を集め、
/// code-unit sort 順に `resolveWorker` する。成功は `workers`、
/// `WORKER_NOT_FOUND`/`INVALID_WORKER_NAME` 以外の失敗は `failures`
/// に積む(TS L38-62 — `CheaperError` 以外の error(例: `Read` variant)
/// も `failures` 行きで preserve される)。
fn resolve_worker_index(raw: &Value, dirs: &ConfigDirs) -> WorkerIndex {
    resolve_worker_index_with(raw, |name| resolve_worker(name, dirs))
}

/// `resolve_worker_index` の source-aware 版。worker definition
/// 取得だけを差し替え、worker 名収集・code-unit sort・failure
/// bucket 規則は filesystem 既定と同一に保つ。
pub(crate) fn resolve_worker_index_with<F>(raw: &Value, mut resolver: F) -> WorkerIndex
where
    F: FnMut(&str) -> Result<WorkerDefinition, ResolveWorkerError>,
{
    let mut index = WorkerIndex {
        workers: HashMap::new(),
        failures: HashMap::new(),
    };
    // `raw && typeof raw === "object"` + `jobs && typeof jobs ===
    // "object" && !Array.isArray(jobs)` — serde の object 判定は
    // array/scalar を含まないため `is_object` のみで同じ条件になる。
    let Some(jobs) = raw.get("jobs").filter(|v| v.is_object()) else {
        return index;
    };
    let mut names: HashSet<String> = HashSet::new();
    for job in jobs.as_object().expect("is_object").values() {
        // `job && typeof job === "object"` — non-object は `get` が
        // None を返すので自然に skip される。
        if job.get("kind").and_then(Value::as_str) == Some("worker") {
            if let Some(name) = job.get("worker").and_then(Value::as_str) {
                names.insert(name.to_string());
            }
        }
    }
    // `[...names].sort()` — JS 既定 sort は UTF-16 code unit 順。
    let mut sorted: Vec<String> = names.into_iter().collect();
    sorted.sort_by(|a, b| code_unit_compare(a, b));
    for name in sorted {
        match resolver(&name) {
            Ok(worker) => {
                index.workers.insert(name, Arc::new(worker));
            }
            Err(ResolveWorkerError::Cheaper(e))
                if matches!(
                    e.code(),
                    ErrorCode::WorkerNotFound | ErrorCode::InvalidWorkerName
                ) =>
            {
                // preload で無視するのはこの 2 code のみ。
            }
            Err(ResolveWorkerError::Cheaper(e)) => {
                index
                    .failures
                    .insert(name, SharedToolError::new(Box::new(e)));
            }
            Err(ResolveWorkerError::Read(e)) => {
                index
                    .failures
                    .insert(name, SharedToolError::new(Box::new(e)));
            }
        }
    }
    index
}

/// `WorkspaceResourceInference` は port が `&'a dyn ToolCatalog` を
/// 要求するため、context 側で `Arc<ToolCatalog>` を所有して使い回す
/// wrapper(self-referential を避ける)。
struct CliResourceInference {
    workspace_root: String,
    symlink_policy: SymlinkPolicy,
    tools: Arc<ToolCatalog>,
}

impl ResourceInference for CliResourceInference {
    fn infer<'a>(
        &'a self,
        jobs: &'a [(String, ResourceInferenceJob)],
    ) -> Pin<Box<dyn Future<Output = Result<ResourceInferenceResult, ToolError>> + Send + 'a>> {
        Box::pin(async move {
            let inner = WorkspaceResourceInference {
                workspace_root: self.workspace_root.clone(),
                symlink_policy: self.symlink_policy,
                tools: &*self.tools,
            };
            inner.infer(jobs).await
        })
    }
}

/// `prepareWorkflowContext` の戻り値相当(root/dirs/policy + compiler
/// ports の供給源)。`loaded`/`workers` は [`WorkflowContext::
/// load_workflow`] が設定する(compiler factory 内で index を組み立て
/// てから ports を結線するため)。
pub(crate) struct WorkflowContext {
    pub root: String,
    pub dirs: ConfigDirs,
    pub policy: RuntimePolicy,
    tools: Arc<ToolCatalog>,
    inference: CliResourceInference,
    workers: OnceLock<Arc<WorkerIndex>>,
}

impl WorkflowContext {
    pub(crate) fn from_policy(
        root: String,
        dirs: ConfigDirs,
        policy: RuntimePolicy,
    ) -> WorkflowContext {
        let tools = Arc::new(ToolCatalog::from_resolved_policy(policy.exec.clone()));
        let inference = CliResourceInference {
            workspace_root: root.clone(),
            symlink_policy: policy.filesystem.symlinks,
            tools: Arc::clone(&tools),
        };
        WorkflowContext {
            root,
            dirs,
            policy,
            tools,
            inference,
            workers: OnceLock::new(),
        }
    }

    /// `loadWorkflowFile(options, createCompiler)`。`createCompiler`
    /// は file read 後の raw definition から worker 名を走査して事前
    /// 解決する — 検証(`validate_definition`)は loader 側に残る。
    pub(crate) async fn load_workflow<'a>(
        &'a self,
        path: &str,
    ) -> Result<LoadedWorkflow<'a>, ToolError> {
        self.load_workflow_from(path, &self.root).await
    }

    pub(crate) async fn load_workflow_from<'a>(
        &'a self,
        path: &str,
        file_root: &str,
    ) -> Result<LoadedWorkflow<'a>, ToolError> {
        let deny: Vec<&str> = self
            .policy
            .filesystem
            .deny_paths
            .iter()
            .map(String::as_str)
            .collect();
        let options = WorkflowFileOptions {
            root: file_root.to_string(),
            path: path.to_string(),
            symlink_policy: self.policy.filesystem.symlinks,
            denylist: create_path_denylist(&deny),
        };
        load_workflow_file(&options, |raw| {
            self.create_compiler(raw, |raw| resolve_worker_index(raw, &self.dirs))
        })
        .await
    }

    /// `load_workflow_file` の document-input 版。workflow source の
    /// 取得層と policy-guarded file read を分離し、embedded/logical
    /// source でも同じ compiler validation と worker index 規則を使う。
    #[allow(dead_code)]
    pub(crate) async fn load_workflow_document<'a, F>(
        &'a self,
        source: &str,
        index_for: F,
    ) -> Result<LoadedWorkflow<'a>, ToolError>
    where
        F: FnOnce(&Value) -> WorkerIndex + 'a,
    {
        load_workflow_document(source, |raw| self.create_compiler(raw, index_for)).await
    }

    fn create_compiler<'a, F>(
        &'a self,
        raw: &Value,
        index_for: F,
    ) -> impl Future<Output = Result<WorkflowCompiler<'a>, ToolError>> + 'a
    where
        F: FnOnce(&Value) -> WorkerIndex + 'a,
    {
        let index = Arc::new(index_for(raw));
        let workers: Arc<dyn WorkerCatalog> = index.clone();
        // `load_workflow` は command 1 回につき 1 回しか呼ばれない
        // (TS も prepareWorkflowContext 内で 1 回 load)。runtime 側
        // `worker_index()` のために最初の index を残すが、compiler
        // 自身はこの document 用に解決した index を所有する。
        let _ = self.workers.set(index);
        async move {
            Ok(WorkflowCompiler::new(WorkflowCompilerPorts {
                tools: &*self.tools,
                workers,
                resources: Some(&self.inference),
            }))
        }
    }

    /// `load_workflow` が組み立てた worker index(executor/lease 経路
    /// の `workers.get` 相当)。
    pub(crate) fn worker_index(&self) -> Option<Arc<WorkerIndex>> {
        self.workers.get().cloned()
    }
}

/// `prepareWorkflowContext`(TS L17-80)の前半 — builtin dir 解決 →
/// workspace root canonicalize → config dirs → policy → tool catalog →
/// resource inference。`loadWorkflowFile` 相当は [`WorkflowContext::
/// load_workflow`]。
/// `invocation` は `--grant` 由来の合成 policy layer — `workflow run`
/// 経路のみが渡し、`workflow interface` は `None` 固定。
pub(crate) fn prepare_workflow_context(
    cwd: Option<&str>,
    io: &CliIo<'_>,
    hooks: &CliHooks,
    invocation: Option<&Map<String, Value>>,
) -> Result<WorkflowContext, BoxError> {
    let layout = crate::compose::resolve_workspace_layout(cwd, io, hooks)?;
    let builtin_dir = layout.builtin_dir;
    let root = layout.root;
    let dirs = layout.dirs;
    let builtin_policy_path = builtin_dir.join("policy.yaml");
    let policy = load_policy(&LoadPolicyOptions {
        builtin_policy_path: &builtin_policy_path.to_string_lossy(),
        user_config_dir: &dirs.user_config_dir.to_string_lossy(),
        project_dir: &dirs.project_dir.to_string_lossy(),
        invocation,
    })?;
    // `createToolCatalog({exec: policy.exec})` — resolved policy
    // からの直接構築(serialize→再 resolve の二重経路を避ける —
    // `ToolCatalog::from_resolved_policy` は同一経路)。
    Ok(WorkflowContext::from_policy(root, dirs, policy))
}

#[cfg(test)]
mod tests {
    use super::*;
    use cheaper_core::policy::load::{load_policy_documents, LoadPolicyDocuments};
    use cheaper_workflow::WorkerCatalog;
    use serde_json::json;
    use std::collections::HashMap;
    use std::path::PathBuf;

    #[tokio::test(flavor = "current_thread")]
    async fn builtin_routed_exec_workflow_compiles() {
        let builtin = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("resources");
        let tmp = tempfile::tempdir().unwrap();
        let ws = tmp.path().join("ws");
        std::fs::create_dir_all(&ws).unwrap();
        let ws_root =
            cheaper_core::workspace::paths::canonicalize_workspace_root(&ws.to_string_lossy())
                .unwrap();
        let env = |_k: &str| None::<String>;
        let supply = crate::route::supply::load(&ws_root, &env, Some(&builtin)).unwrap();
        let dirs = ConfigDirs {
            user_config_dir: tmp.path().join("user"),
            project_dir: tmp.path().join("project"),
            builtin_dir: builtin.clone(),
        };
        let ctx = WorkflowContext::from_policy(ws_root.clone(), dirs, supply.policy);
        let file_root = builtin.join("workflows").to_string_lossy().into_owned();
        let loaded = ctx
            .load_workflow_from("routed-exec.workflow.json", &file_root)
            .await
            .unwrap();
        let raw = loaded.definition.raw();
        assert_eq!(
            raw["jobs"]["analyze"]["when"]["left"]["path"],
            "/details/outputTokens"
        );
        assert_eq!(
            raw["jobs"]["analyze"]["when"]["right"]["$input"],
            "tokenThreshold"
        );
        let inputs = json!({
            "grant": "cargo-test",
            "executable": "/bin/cargo",
            "argv": ["cargo", "test"],
            "args": ["test"],
            "execArgs": ["test"],
            "cwd": ws_root,
            "tokenThreshold": 8192,
            "runId": "1700000000000-1-deadbeef"
        });
        loaded.compiler.compile(raw, Some(&inputs)).await.unwrap();
    }

    #[tokio::test(flavor = "current_thread")]
    async fn embedded_routed_exec_document_validates_with_source_aware_workers() {
        let tmp = tempfile::tempdir().unwrap();
        let ws = tmp.path().join("ws");
        let user = tmp.path().join("user");
        std::fs::create_dir_all(&ws).unwrap();
        std::fs::create_dir_all(&user).unwrap();
        let env_map = HashMap::from([(
            "CHEAPER_CONFIG_DIR".to_string(),
            user.to_string_lossy().into_owned(),
        )]);
        let env = |name: &str| env_map.get(name).cloned();
        let sources =
            crate::route::supply::check_sources(ws.to_str().unwrap(), &env, &CliHooks::default())
                .unwrap();
        let evaluated = sources.evaluate_route_layers();
        let route = evaluated
            .merged
            .rows
            .iter()
            .find_map(|row| row.route())
            .expect("builtin route");
        let crate::route::supply::RouteWorkflowInput::Document(document) =
            sources.builtin.workflow_input(route)
        else {
            panic!("embedded workflow document")
        };

        let embedded = crate::embedded::EmbeddedResources::new();
        let policy_text = embedded.read_text("policy.yaml").unwrap();
        let policy_doc = cheaper_core::yaml::parse_yaml(policy_text)
            .unwrap()
            .as_object()
            .unwrap()
            .clone();
        let policy = load_policy_documents(&LoadPolicyDocuments {
            builtin: Some(&policy_doc),
            user: None,
            invocation: None,
            project: None,
        })
        .unwrap()
        .policy;
        let ctx = WorkflowContext::from_policy(
            sources.root.clone(),
            ConfigDirs {
                user_config_dir: user.clone(),
                project_dir: ws.join(".cheaper"),
                builtin_dir: PathBuf::from("/builtin"),
            },
            policy,
        );
        let loaded = ctx
            .load_workflow_document(&document.text, |raw| sources.resolve_worker_index(raw))
            .await
            .unwrap();
        crate::workflow_run::validate_routed_convention(loaded.definition.raw()).unwrap();
        let index = ctx.worker_index().unwrap();
        let worker = index.get("condense").unwrap().unwrap();
        assert_eq!(
            worker.origin,
            cheaper_core::worker::validate::WorkerOrigin::Builtin
        );

        // 同一 context で 2 件目の document を検証しても、compiler
        // はその document 用に解決した worker index を使う。
        let other = "second-worker";
        let other_doc = serde_json::json!({
            "schemaVersion": 1,
            "inputs": {},
            "jobs": {
                "analyze": {
                    "kind": "worker",
                    "worker": other,
                    "input": {}
                }
            },
            "outputs": {}
        })
        .to_string();
        let other_worker = cheaper_core::worker::validate::validate_worker_definition(
            &cheaper_core::worker::parse::parse_worker_file(
                &format!(
                    "---\nname: {other}\ndescription: test\ntools: [read]\ncheaper:\n  model: {{tier: high}}\n  output: {{schema: {{inline: {{type: string}}}}}}\n---\nbody\n"
                ),
                "second-worker.md",
            )
            .unwrap(),
            "second-worker.md",
            cheaper_core::worker::validate::WorkerOrigin::Project,
        )
        .unwrap();
        ctx.load_workflow_document(&other_doc, |_| WorkerIndex {
            workers: HashMap::from([(other.to_string(), Arc::new(other_worker))]),
            failures: HashMap::new(),
        })
        .await
        .unwrap();
        assert_eq!(worker.source_path, "/builtin/workers/condense.md");
    }
}
