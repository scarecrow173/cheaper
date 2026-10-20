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

use cheaper_core::config::dirs::{resolve_config_dirs, ConfigDirsOptions, EnvLookup};
use cheaper_core::errors::{CheaperError, ErrorCode};
use cheaper_core::policy::load::{load_policy, LoadPolicyOptions};
use cheaper_core::policy::schema::RuntimePolicy;
use cheaper_core::workspace::deny_paths::{create_path_denylist, PathDenylist};
use cheaper_core::workspace::paths::canonicalize_workspace_root;

use super::resolve::CommandPolicyView;
use super::routes::{self, EffectiveRoutes, RouteLoadOptions};

/// resolve の入力材料 — hook eval では firing 中に owned で保持し、
/// `RouteEvalInput`/`CommandPolicyView` はこれへの borrow として
/// 構築する。
pub struct RouteSupply {
    pub routes: EffectiveRoutes,
    pub policy: RuntimePolicy,
    pub denylist: PathDenylist,
    /// canonicalized workspace root(`ResolveInput.cwd`・policy
    /// preflight root の基準)。
    pub root: String,
}

impl RouteSupply {
    /// viability 評価の参照面 — `check_command_execution` と同一
    /// predicate の policy 環境。
    pub fn command_view(&self) -> CommandPolicyView<'_> {
        CommandPolicyView {
            policy: &self.policy.command,
            root: &self.root,
            symlink_policy: self.policy.filesystem.symlinks,
            denylist: Some(&self.denylist),
        }
    }

    /// `EvaluateOptions.route` へそのまま渡せる参照面。
    pub fn eval_input(&self) -> crate::hook::routing_policy::RouteEvalInput<'_> {
        crate::hook::routing_policy::RouteEvalInput {
            routes: &self.routes,
            command_policy: self.command_view(),
        }
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
    Ok(RouteSupply {
        routes,
        policy,
        denylist,
        root,
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
