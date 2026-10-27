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
use cheaper_core::policy::load::{load_policy, LoadPolicyOptions};
use cheaper_core::policy::schema::RuntimePolicy;
use cheaper_core::policy::security::ExecDenyList;
use cheaper_core::workspace::deny_paths::{create_path_denylist, PathDenylist};
use cheaper_core::workspace::paths::canonicalize_workspace_root;

use crate::cli::CliHooks;
use crate::embedded::EmbeddedResources;

use super::resolve::ExecPolicyView;
use super::routes::{self, EffectiveRoutes, RouteLayer, RouteLoadOptions};

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

/// `route check` が評価する 3 層 source。`root` は内部評価専用の
/// canonical workspace root で、report へ serialize しない。
#[derive(Debug)]
#[allow(dead_code)]
pub(crate) struct RouteCheckSources {
    pub(crate) root: String,
    pub(crate) builtin: RouteLayerSource,
    pub(crate) user: RouteLayerSource,
    pub(crate) project: RouteLayerSource,
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
    let ConfigDirs {
        user_config_dir,
        project_dir,
        ..
    } = resolve_config_dirs(&ConfigDirsOptions {
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
        user_config_dir.join("routes.yaml"),
        user_config_dir.clone(),
        &user_config_dir,
    );
    let project = filesystem_source(
        project_dir.join("routes.yaml"),
        project_dir.clone(),
        &project_dir,
    );
    Ok(RouteCheckSources {
        root,
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
}
