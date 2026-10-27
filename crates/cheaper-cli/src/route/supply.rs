//! `route::supply` — route resolve が要する外部材料(effective
//! routes・effective command policy・denylist・canonical root)の
//! 一回 load(design §入力供給 — hook eval では 1 firing 1 回、
//! `route resolve`/`route run` は 1 command 1 回)。3 caller が同一の
//! effective set を見るための共有 loader。
//!
//! hook 経路では全ての load 失敗が `None`(route arm 不活性)へ
//! 畳まれる — routing 層の不備が親の tool call を阻害しない
//! fail-open 規約(R39)。`route resolve`/`route run` 側は error を
//! そのまま呼び出しへ返す厳密経路([`load`])と、失敗を `None` へ
//! 畳む hook 用経路([`load_or_none`])を分ける。

use std::path::{Path, PathBuf};

use cheaper_core::config::dirs::{resolve_config_dirs, ConfigDirs, ConfigDirsOptions, EnvLookup};
use cheaper_core::errors::{CheaperError, ErrorCode};
use cheaper_core::policy::load::{load_policy, read_policy_yaml, LoadPolicyOptions};
use cheaper_core::policy::schema::RuntimePolicy;
use cheaper_core::policy::security::ExecDenyList;
use cheaper_core::worker::resolve::{
    resolve_worker_with_sources, ResolveWorkerError, WorkerSourceCandidate,
};
use cheaper_core::worker::schema::FilesystemContentSource;
use cheaper_core::worker::validate::{WorkerDefinition, WorkerOrigin};
use cheaper_core::workspace::deny_paths::{create_path_denylist, PathDenylist};
use cheaper_core::workspace::paths::canonicalize_workspace_root;
use serde_json::{Map, Value};

use crate::cli::CliHooks;
use crate::embedded::EmbeddedResources;

use super::resolve::ExecPolicyView;
use super::routes::{
    self, EffectiveRoutes, Route, RouteLayer, RouteLayerEvaluation, RouteLoadOptions,
    RouteMergeOutcome,
};

/// resolve の入力材料 — hook eval では firing 中に owned で保持し、
/// `RouteEvalInput`/`ExecPolicyView` はこれへの borrow として
/// 構築する。
pub struct RouteSupply {
    pub routes: EffectiveRoutes,
    pub policy: RuntimePolicy,
    pub denylist: PathDenylist,
    /// canonicalized workspace root(`ResolveInput.cwd`・policy
    /// preflight root の基準)。
    pub root: String,
    /// `security.execDenyPatterns` の compiled deny(issue #164)—
    /// load 済み `policy.security` から supply 構築で 1 回 compile
    /// し、viability の `ExecPolicyView` へ供給する。
    pub exec_deny: ExecDenyList,
}

impl RouteSupply {
    /// viability 評価の参照面 — `check_exec_execution` と同一
    /// predicate の policy 環境(unified `policy.exec` — design §(j))。
    pub fn exec_view(&self) -> ExecPolicyView<'_> {
        ExecPolicyView {
            policy: &self.policy.exec,
            root: &self.root,
            symlink_policy: self.policy.filesystem.symlinks,
            denylist: Some(&self.denylist),
            exec_deny: &self.exec_deny,
        }
    }

    /// `EvaluateOptions.route` へそのまま渡せる参照面。
    pub fn eval_input(&self) -> crate::hook::routing_policy::RouteEvalInput<'_> {
        crate::hook::routing_policy::RouteEvalInput {
            routes: &self.routes,
            exec_policy: self.exec_view(),
        }
    }
}

/// `route check` の layer resource 取得先。diagnostics は
/// filesystem/embedded の差異をこの source 経由で解決し、declared
/// output には logical path だけを使う。
#[derive(Debug)]
#[allow(dead_code)]
pub(crate) enum RouteLayerSource {
    Filesystem {
        routes_file: PathBuf,
        workflow_root: PathBuf,
        /// resolver が `workers/<name>.md` を join する layer base dir。
        worker_base_dir: PathBuf,
    },
    Embedded {
        catalog: EmbeddedResources,
    },
}

/// route declaration 層から取得した document。`source` は report
/// へ出せる `<layer>:<relative>` logical path、`text` は shared
/// validator への入力。
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub(crate) struct RouteDocument {
    pub(crate) source: String,
    pub(crate) text: String,
}

/// route target workflow の validation 入力。filesystem layer は
/// ここで読まず、canonical `load_workflow_file` が policy-guarded
/// I/O を行うために `file_root` と relative `path` を返す。
/// `file_root` は内部用の host root で、report へ serialize しない。
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub(crate) enum RouteWorkflowInput {
    Document(RouteDocument),
    File {
        source: String,
        file_root: String,
        path: String,
    },
    Missing {
        source: String,
    },
}

static FILESYSTEM_CONTENT_SOURCE: FilesystemContentSource = FilesystemContentSource;

impl RouteLayerSource {
    /// この層の routes.yaml を shared per-layer validator へ渡す。
    /// filesystem/embedded の取得方法だけが異なり、schema・duplicate
    /// 隔離・trust 規則は `route::routes` の同一関数を使う。
    pub(crate) fn evaluate_routes(&self, layer: RouteLayer) -> RouteLayerEvaluation {
        match self {
            Self::Filesystem {
                routes_file,
                workflow_root,
                ..
            } => routes::evaluate_filesystem_layer(layer, routes_file, workflow_root),
            Self::Embedded { catalog } => {
                let Some(bytes) = catalog.read("routes.yaml") else {
                    return RouteLayerEvaluation::absent(layer, Path::new("/builtin/routes.yaml"));
                };
                routes::evaluate_routes_text(
                    layer,
                    "/builtin/routes.yaml",
                    &String::from_utf8_lossy(bytes),
                    Path::new("/builtin/workflows"),
                )
            }
        }
    }

    /// この層の policy document を shared `load_policy_documents`
    /// へ渡す object として取得する。filesystem layer は既存
    /// `read_policy_yaml` の read/parse/object 規約を再利用し、
    /// embedded builtin は logical catalog read + 同じ YAML/object
    /// 判定を行う。どちらも validator/merge は呼ばない。
    pub(crate) fn policy_document(
        &self,
        layer: RouteLayer,
    ) -> Result<Option<Map<String, Value>>, CheaperError> {
        match self {
            Self::Filesystem {
                worker_base_dir, ..
            } => {
                let file = worker_base_dir.join("policy.yaml");
                read_policy_yaml(&file.to_string_lossy(), false)
            }
            Self::Embedded { catalog } => {
                let Some(bytes) = catalog.read("policy.yaml") else {
                    return Ok(None);
                };
                let text = String::from_utf8_lossy(bytes);
                let parsed = cheaper_core::yaml::parse_yaml(&text).map_err(|e| {
                    let mut details = Map::new();
                    details.insert("reason".to_string(), Value::from("yaml-parse"));
                    CheaperError::new(
                        ErrorCode::InvalidRuntimePolicy,
                        format!("policy yaml parse failed: {}:policy.yaml", layer.as_str()),
                    )
                    .with_details(details)
                    .with_cause(e)
                })?;
                match parsed {
                    Value::Object(document) => Ok(Some(document)),
                    _ => {
                        let mut details = Map::new();
                        details.insert("reason".to_string(), Value::from("not-object"));
                        Err(CheaperError::new(
                            ErrorCode::InvalidRuntimePolicy,
                            format!("policy is not an object: {}:policy.yaml", layer.as_str()),
                        )
                        .with_details(details))
                    }
                }
            }
        }
    }

    /// route target workflow の validation 入力を declaration 層
    /// source から解決する。filesystem は policy guard を bypass
    /// せず、`load_workflow_file` の `file_root + path` を返す。
    /// embedded builtin は `/builtin/workflows/...` を catalog key
    /// `workflows/...` へ正規化して read-only に取得する。
    #[allow(dead_code)]
    pub(crate) fn workflow_input(&self, route: &Route) -> RouteWorkflowInput {
        match self {
            Self::Filesystem {
                workflow_root,
                worker_base_dir,
                ..
            } => RouteWorkflowInput::File {
                source: route_document_source(route.layer, worker_base_dir, &route.target_path),
                file_root: workflow_root.to_string_lossy().into_owned(),
                path: relative_path(workflow_root, &route.target_path),
            },
            Self::Embedded { catalog } => {
                let logical_path = route.target_path.to_string_lossy().replace('\\', "/");
                let source = route_document_source(
                    route.layer,
                    Path::new(EmbeddedResources::LOGICAL_ROOT),
                    &route.target_path,
                );
                match catalog.read(&logical_path) {
                    Some(bytes) => RouteWorkflowInput::Document(RouteDocument {
                        source,
                        text: String::from_utf8_lossy(bytes).into_owned(),
                    }),
                    None => RouteWorkflowInput::Missing { source },
                }
            }
        }
    }

    /// viability の `missing-target` 判定だけを source-aware にする。
    /// filesystem layer は既存 runtime と同じ `Path::is_file`、
    /// embedded builtin は logical `/builtin/` path を catalog key に
    /// 写して `contains` で確認する(temp materialization はしない)。
    #[allow(dead_code)]
    pub(crate) fn target_exists(&self, route: &Route) -> bool {
        match self {
            Self::Filesystem { .. } => route.target_path.is_file(),
            Self::Embedded { catalog } => catalog.contains(&slash_path(&route.target_path)),
        }
    }

    /// worker resolver へ渡すこの層の content source + base dir。
    fn worker_candidate(&self, origin: WorkerOrigin) -> WorkerSourceCandidate<'_> {
        match self {
            Self::Filesystem {
                worker_base_dir, ..
            } => WorkerSourceCandidate {
                base_dir: worker_base_dir.to_string_lossy().into_owned(),
                origin,
                source: &FILESYSTEM_CONTENT_SOURCE,
            },
            Self::Embedded { catalog } => WorkerSourceCandidate {
                base_dir: EmbeddedResources::LOGICAL_ROOT
                    .trim_end_matches('/')
                    .to_string(),
                origin,
                source: catalog,
            },
        }
    }
}

fn slash_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

fn relative_path(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .ok()
        .map(slash_path)
        .or_else(|| {
            path.file_name()
                .map(|name| name.to_string_lossy().replace('\\', "/"))
        })
        .unwrap_or_default()
}

fn route_document_source(layer: RouteLayer, layer_root: &Path, path: &Path) -> String {
    format!("{}:{}", layer.as_str(), relative_path(layer_root, path))
}

/// `route check` が評価する 3 層 source。`root` は内部評価専用の
/// canonical workspace root で、report へ serialize しない。
#[derive(Debug)]
#[allow(dead_code)]
pub(crate) struct RouteCheckSources {
    pub(crate) root: String,
    /// workflow context に渡す config-dir 解決結果。filesystem source
    /// と同一の user/project dir を保持し、builtin sentinel は
    /// report・host path 解決へ使わない。
    pub(crate) dirs: ConfigDirs,
    pub(crate) builtin: RouteLayerSource,
    pub(crate) user: RouteLayerSource,
    pub(crate) project: RouteLayerSource,
}

/// shared merge と層別 status の組。route-check report はこれを shallow
/// row/layer section へ投影する。
#[derive(Debug)]
#[allow(dead_code)]
pub(crate) struct RouteCheckRouteEvaluation {
    pub(crate) layers: Vec<RouteLayerEvaluation>,
    pub(crate) merged: RouteMergeOutcome,
}

#[allow(dead_code)]
impl RouteCheckSources {
    pub(crate) fn layers(&self) -> [(RouteLayer, &RouteLayerSource); 3] {
        [
            (RouteLayer::Builtin, &self.builtin),
            (RouteLayer::User, &self.user),
            (RouteLayer::Project, &self.project),
        ]
    }

    /// builtin/user/project を独立に評価してから shared merge する。
    /// 1 層 failure はその層の error だけで、valid sibling の row を残す。
    pub(crate) fn evaluate_route_layers(&self) -> RouteCheckRouteEvaluation {
        let layers = self
            .layers()
            .into_iter()
            .map(|(layer, source)| source.evaluate_routes(layer))
            .collect::<Vec<_>>();
        RouteCheckRouteEvaluation {
            merged: routes::merge_route_layers(&layers),
            layers,
        }
    }

    /// worker resolution precedence — project → user → builtin。
    /// filesystem layer は host path、embedded builtin は logical
    /// `/builtin/` root を catalog read へ写像する。
    pub(crate) fn resolve_worker(
        &self,
        name: &str,
    ) -> Result<WorkerDefinition, ResolveWorkerError> {
        resolve_worker_with_sources(
            name,
            &[
                self.project.worker_candidate(WorkerOrigin::Project),
                self.user.worker_candidate(WorkerOrigin::User),
                self.builtin.worker_candidate(WorkerOrigin::Builtin),
            ],
        )
    }

    /// workflow compiler の `WorkerCatalog` port と同じ preload 規則で
    /// worker index を組み立てる。worker definition/schema acquisition
    /// だけが source-aware になる。
    pub(crate) fn resolve_worker_index(&self, raw: &Value) -> crate::workflow_context::WorkerIndex {
        crate::workflow_context::resolve_worker_index_with(raw, |name| self.resolve_worker(name))
    }

    /// route の declaration layer に対応する source で target existence
    /// を判定する。shared viability へ `|route| self.target_exists(route)`
    /// として注入する。
    #[allow(dead_code)]
    pub(crate) fn target_exists(&self, route: &Route) -> bool {
        self.layers()
            .into_iter()
            .find(|(layer, _)| *layer == route.layer)
            .is_some_and(|(_, source)| source.target_exists(route))
    }
}

/// `route check` 専用 supply builder — config-dir/root 解決規約は
/// 実行経路と同じだが、`builtin_dir` 未指定の production path では
/// embedded catalog を read-only source として使い、builtin temp
/// materialization は行わない。user/project は常に filesystem。
pub(crate) fn check_sources(
    cwd: &str,
    env: EnvLookup<'_>,
    hooks: &CliHooks,
) -> Result<RouteCheckSources, CheaperError> {
    let root = canonicalize_workspace_root(cwd)?;
    let builtin_dir = hooks
        .builtin_dir
        .clone()
        .unwrap_or_else(|| PathBuf::from(EmbeddedResources::LOGICAL_ROOT));
    // `resolve_config_dirs` は builtin_dir を返すため logical root を
    // sentinel として渡すが、check source model では `ConfigDirs.builtin_dir`
    // を host path として使わない。
    let dirs = resolve_config_dirs(&ConfigDirsOptions {
        cwd: Path::new(&root),
        builtin_dir: &builtin_dir,
        env: Some(env),
        homedir: None,
    });
    let builtin = match hooks.builtin_dir.as_deref() {
        Some(dir) => filesystem_source(dir.join("routes.yaml"), dir.join("workflows"), dir),
        None => RouteLayerSource::Embedded {
            catalog: EmbeddedResources::new(),
        },
    };
    let user = filesystem_source(
        dirs.user_config_dir.join("routes.yaml"),
        dirs.user_config_dir.clone(),
        &dirs.user_config_dir,
    );
    let project = filesystem_source(
        dirs.project_dir.join("routes.yaml"),
        dirs.project_dir.clone(),
        &dirs.project_dir,
    );
    Ok(RouteCheckSources {
        root,
        dirs,
        builtin,
        user,
        project,
    })
}

fn filesystem_source(
    routes_file: PathBuf,
    workflow_root: PathBuf,
    worker_base_dir: &Path,
) -> RouteLayerSource {
    RouteLayerSource::Filesystem {
        routes_file,
        workflow_root,
        worker_base_dir: worker_base_dir.to_path_buf(),
    }
}

/// `request_cwd` 起点で routes + command policy を load する厳密
/// 経路 — 失敗は `CheaperError` として caller へ返す(`route
/// resolve`/`route run` の utility/run 経路)。
///
/// `builtin_dir` が `None` の場合は embedded resource を
/// materialize して使う(hook 経路と同じ規約)。
pub fn load(
    cwd: &str,
    env: EnvLookup<'_>,
    builtin_dir: Option<&Path>,
) -> Result<RouteSupply, CheaperError> {
    let builtin = match builtin_dir {
        Some(d) => d.to_path_buf(),
        None => crate::embedded::builtin_dir().map_err(|e| {
            CheaperError::new(ErrorCode::InternalError, "builtin dir resolve failed").with_details(
                serde_json::json!({"error": e.to_string()})
                    .as_object()
                    .unwrap()
                    .clone(),
            )
        })?,
    };
    load_with_builtin(cwd, env, &builtin)
}

/// `builtin_dir` 解決済みの内部経路 — `route_cmd` 等 workspace
/// layout を既に解決した caller が embedded fallback を二重に
/// 行わないための公開口。
pub fn load_with_builtin(
    cwd: &str,
    env: EnvLookup<'_>,
    builtin_dir: &Path,
) -> Result<RouteSupply, CheaperError> {
    let root = canonicalize_workspace_root(cwd)?;
    let dirs = resolve_config_dirs(&ConfigDirsOptions {
        cwd: Path::new(&root),
        builtin_dir,
        env: Some(env),
        homedir: None,
    });
    let routes = routes::load(&RouteLoadOptions {
        builtin_dir,
        user_config_dir: &dirs.user_config_dir,
        project_dir: &dirs.project_dir,
    })?;
    let builtin_policy_path = builtin_dir.join("policy.yaml");
    let policy = load_policy(&LoadPolicyOptions {
        builtin_policy_path: &builtin_policy_path.to_string_lossy(),
        user_config_dir: &dirs.user_config_dir.to_string_lossy(),
        project_dir: &dirs.project_dir.to_string_lossy(),
        invocation: None,
    })?;
    let deny: Vec<&str> = policy
        .filesystem
        .deny_paths
        .iter()
        .map(String::as_str)
        .collect();
    let denylist = create_path_denylist(&deny);
    // load 済み policy の `security.execDenyPatterns` を compiled
    // deny 化(issue #164)。load 済みのため compile 失敗は到達不能
    // 想定だが `?` で伝搬する(design 確定)。
    let exec_deny = ExecDenyList::compile(&policy.security.exec_deny_patterns)?;
    Ok(RouteSupply {
        routes,
        policy,
        denylist,
        root,
        exec_deny,
    })
}

/// hook eval 用の fail-open 経路 — あらゆる load 失敗を `None`
/// (route arm 不活性)へ畳む。resolve 自体は `Some` 時に限り走る。
pub fn load_or_none(
    cwd: &str,
    env: EnvLookup<'_>,
    builtin_dir: Option<&PathBuf>,
) -> Option<RouteSupply> {
    load(cwd, env, builtin_dir.map(PathBuf::as_path))
        .map_err(|e| {
            tracing::debug!(
                error = %e,
                "route supply load failed — route arm inactive"
            );
            e
        })
        .ok()
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use cheaper_workflow::WorkerCatalog;

    use super::*;

    fn env_lookup<'a>(pairs: &'a HashMap<String, String>) -> impl Fn(&str) -> Option<String> + 'a {
        move |name| pairs.get(name).cloned()
    }

    #[test]
    fn check_sources_uses_embedded_builtin_on_production_path() {
        let ws = tempfile::tempdir().unwrap();
        let user = tempfile::tempdir().unwrap();
        let env = HashMap::from([(
            "CHEAPER_CONFIG_DIR".to_string(),
            user.path().to_string_lossy().into_owned(),
        )]);
        let env = env_lookup(&env);
        let before = crate::embedded::builtin_dir_call_count();
        let sources =
            check_sources(ws.path().to_str().unwrap(), &env, &CliHooks::default()).unwrap();
        assert_eq!(
            sources.root,
            canonicalize_workspace_root(ws.path().to_str().unwrap()).unwrap()
        );
        let RouteLayerSource::Embedded { catalog } = &sources.builtin else {
            panic!("production builtin must use EmbeddedResources")
        };
        assert!(catalog.contains("routes.yaml"));
        assert!(catalog.contains("policy.yaml"));
        assert!(catalog.contains("workflows/routed-exec.workflow.json"));
        assert!(catalog.contains("workers/condense.md"));
        assert!(catalog.contains("workers/schemas/condense.input.schema.json"));
        assert_eq!(crate::embedded::builtin_dir_call_count(), before);

        for (layer, source) in sources.layers() {
            match layer {
                RouteLayer::Builtin => continue,
                RouteLayer::User => {
                    let RouteLayerSource::Filesystem {
                        routes_file,
                        workflow_root,
                        worker_base_dir,
                    } = source
                    else {
                        panic!("user layer must be filesystem")
                    };
                    assert_eq!(routes_file, &user.path().join("routes.yaml"));
                    assert_eq!(workflow_root, user.path());
                    assert_eq!(worker_base_dir, user.path());
                }
                RouteLayer::Project => {
                    let project = ws.path().join(".cheaper");
                    let RouteLayerSource::Filesystem {
                        routes_file,
                        workflow_root,
                        worker_base_dir,
                    } = source
                    else {
                        panic!("project layer must be filesystem")
                    };
                    assert_eq!(routes_file, &project.join("routes.yaml"));
                    assert_eq!(workflow_root, &project);
                    assert_eq!(worker_base_dir, &project);
                }
            }
        }
    }

    #[test]
    fn check_sources_uses_filesystem_builtin_only_with_hook_override() {
        let ws = tempfile::tempdir().unwrap();
        let builtin = tempfile::tempdir().unwrap();
        let env_map = HashMap::new();
        let env = env_lookup(&env_map);
        let hooks = CliHooks {
            builtin_dir: Some(builtin.path().to_path_buf()),
            ..Default::default()
        };
        let before = crate::embedded::builtin_dir_call_count();
        let sources = check_sources(ws.path().to_str().unwrap(), &env, &hooks).unwrap();
        let RouteLayerSource::Filesystem {
            routes_file,
            workflow_root,
            worker_base_dir,
        } = &sources.builtin
        else {
            panic!("hook builtin_dir override must use filesystem")
        };
        assert_eq!(routes_file, &builtin.path().join("routes.yaml"));
        assert_eq!(workflow_root, &builtin.path().join("workflows"));
        assert_eq!(worker_base_dir, builtin.path());
        assert_eq!(crate::embedded::builtin_dir_call_count(), before);
    }

    #[test]
    fn filesystem_workflow_input_returns_guarded_file_boundary_and_logical_source() {
        let ws = tempfile::tempdir().unwrap();
        let user = tempfile::tempdir().unwrap();
        std::fs::write(
            user.path().join("routes.yaml"),
            r#"
version: 1
routes:
  - name: user-route
    match: {command: [user, route]}
    target: {kind: workflow, path: workflows/user.workflow.json}
    grant: user-grant
"#,
        )
        .unwrap();
        let env = HashMap::from([(
            "CHEAPER_CONFIG_DIR".to_string(),
            user.path().to_string_lossy().into_owned(),
        )]);
        let env = env_lookup(&env);
        let sources =
            check_sources(ws.path().to_str().unwrap(), &env, &CliHooks::default()).unwrap();
        let evaluated = sources.evaluate_route_layers();
        let route = evaluated
            .merged
            .rows
            .iter()
            .find_map(|row| row.route().filter(|route| route.layer == RouteLayer::User))
            .expect("user route row");

        let RouteWorkflowInput::File {
            source,
            file_root,
            path,
        } = sources.user.workflow_input(route)
        else {
            panic!("filesystem workflow must stay on the guarded file boundary")
        };
        assert_eq!(source, "user:workflows/user.workflow.json");
        assert_eq!(file_root, user.path().to_string_lossy());
        assert_eq!(path, "workflows/user.workflow.json");
        assert!(!sources.target_exists(route));
        std::fs::create_dir_all(route.target_path.parent().unwrap()).unwrap();
        std::fs::write(&route.target_path, "{}").unwrap();
        assert!(sources.target_exists(route));
    }

    #[test]
    fn embedded_builtin_route_workflow_and_worker_share_logical_sources() {
        let ws = tempfile::tempdir().unwrap();
        let user = tempfile::tempdir().unwrap();
        let env = HashMap::from([(
            "CHEAPER_CONFIG_DIR".to_string(),
            user.path().to_string_lossy().into_owned(),
        )]);
        let env = env_lookup(&env);
        let sources =
            check_sources(ws.path().to_str().unwrap(), &env, &CliHooks::default()).unwrap();

        let evaluated = sources.evaluate_route_layers();
        assert_eq!(evaluated.layers.len(), 3);
        assert_eq!(evaluated.layers[0].status, routes::RouteLayerStatus::Loaded);
        assert_eq!(evaluated.layers[1].status, routes::RouteLayerStatus::Absent);
        assert_eq!(evaluated.layers[2].status, routes::RouteLayerStatus::Absent);
        let route = evaluated
            .merged
            .rows
            .iter()
            .find_map(|row| row.route())
            .expect("builtin route row");
        assert_eq!(route.layer, RouteLayer::Builtin);
        assert!(sources.target_exists(route));
        let mut missing = route.clone();
        missing.target_path = Path::new("/builtin/workflows/missing.workflow.json").to_path_buf();
        assert!(!sources.target_exists(&missing));

        let RouteWorkflowInput::Document(document) = sources.builtin.workflow_input(route) else {
            panic!("embedded routed workflow must be a document input")
        };
        assert_eq!(
            document.source,
            "builtin:workflows/routed-exec.workflow.json"
        );
        let raw: Value = serde_json::from_str(&document.text).unwrap();
        let index = sources.resolve_worker_index(&raw);
        let worker = index
            .get("condense")
            .expect("worker catalog lookup")
            .expect("condense worker");
        assert_eq!(worker.origin, WorkerOrigin::Builtin);
        assert_eq!(worker.source_path, "/builtin/workers/condense.md");
        worker
            .input_schema
            .as_ref()
            .expect("embedded input schema")
            .validate(&serde_json::json!({
                "logRef": "1-1-abc",
                "originalTokens": 10,
                "command": ["cargo", "test"]
            }))
            .unwrap();
        worker
            .output_schema
            .validate(&serde_json::json!({"summary": "ok"}))
            .unwrap();
    }

    #[test]
    fn source_aware_target_exists_drives_shared_viability() {
        let ws = tempfile::tempdir().unwrap();
        let user = tempfile::tempdir().unwrap();
        let env = HashMap::from([(
            "CHEAPER_CONFIG_DIR".to_string(),
            user.path().to_string_lossy().into_owned(),
        )]);
        let env = env_lookup(&env);
        let sources =
            check_sources(ws.path().to_str().unwrap(), &env, &CliHooks::default()).unwrap();
        let evaluated = sources.evaluate_route_layers();
        let argv = vec!["cargo".to_string(), "test".to_string()];
        let policy =
            cheaper_core::policy::exec::resolve_exec_policy(None, &ExecDenyList::default())
                .unwrap();
        let exec_deny = ExecDenyList::default();
        let view = ExecPolicyView {
            policy: &policy,
            root: &sources.root,
            symlink_policy: cheaper_core::workspace::paths::SymlinkPolicy::Deny,
            denylist: None,
            exec_deny: &exec_deny,
        };
        let result = crate::route::resolve::resolve_with_target_exists(
            &crate::route::resolve::ResolveInput {
                argv: &argv,
                cwd: &sources.root,
                routes: &evaluated.merged.effective,
                exec_policy: &view,
                unsafe_command: false,
            },
            |route| sources.target_exists(route),
        );
        assert_eq!(result.matched.selected.unwrap().name, "cargo-test");
        assert_eq!(
            result.resolution.viability,
            Some(crate::route::resolve::Viability::NoGrant)
        );
    }

    #[test]
    fn source_aware_worker_resolution_preserves_project_user_builtin_order() {
        let ws = tempfile::tempdir().unwrap();
        let user = tempfile::tempdir().unwrap();
        let env = HashMap::from([(
            "CHEAPER_CONFIG_DIR".to_string(),
            user.path().to_string_lossy().into_owned(),
        )]);
        let env = env_lookup(&env);
        let sources =
            check_sources(ws.path().to_str().unwrap(), &env, &CliHooks::default()).unwrap();
        let write_worker = |root: &Path| {
            let workers = root.join("workers");
            std::fs::create_dir_all(&workers).unwrap();
            std::fs::write(
                workers.join("condense.md"),
                r#"---
name: condense
description: override
tools: [read]
cheaper:
  model: {tier: high}
  output: {schema: {inline: {type: string}}}
---
override
"#,
            )
            .unwrap();
        };
        write_worker(&ws.path().join(".cheaper"));
        write_worker(user.path());
        assert_eq!(
            sources.resolve_worker("condense").unwrap().origin,
            WorkerOrigin::Project
        );
        std::fs::remove_file(ws.path().join(".cheaper/workers/condense.md")).unwrap();
        assert_eq!(
            sources.resolve_worker("condense").unwrap().origin,
            WorkerOrigin::User
        );
        std::fs::remove_file(user.path().join("workers/condense.md")).unwrap();
        let builtin = sources.resolve_worker("condense").unwrap();
        assert_eq!(builtin.origin, WorkerOrigin::Builtin);
        assert_eq!(builtin.source_path, "/builtin/workers/condense.md");
    }

    #[test]
    fn embedded_worker_not_found_is_a_diagnostic_error() {
        let ws = tempfile::tempdir().unwrap();
        let env_map = HashMap::new();
        let env = env_lookup(&env_map);
        let sources =
            check_sources(ws.path().to_str().unwrap(), &env, &CliHooks::default()).unwrap();
        let err = match sources.resolve_worker("missing-worker") {
            Err(ResolveWorkerError::Cheaper(err)) => err,
            other => panic!("expected worker-not-found CheaperError: {other:?}"),
        };
        assert_eq!(err.code(), ErrorCode::WorkerNotFound);
        let searched = err.details().unwrap()["searched"].as_array().unwrap();
        assert_eq!(
            searched.last().unwrap().as_str().unwrap(),
            "/builtin/workers/missing-worker.md"
        );
    }
}
