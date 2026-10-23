//! `cheaper policy grant {add,list,remove}` — user-scope
//! `<userConfigDir>/policy.yaml` の `command.grants` を validation 付きで
//! 管理する utility command(DESIGN-034FntkIZGiffhdU4caSkN §(f))。
//!
//! - write 経路(add/remove)は read → 構造前提 pre-check → grants map
//!   畳み込み → **merged full object** の `validate_policy_schema(Partial)`
//!   → `stringify_yaml` → `atomic_write`(`windows_remove_first: false`、
//!   POSIX `0600`)の順。merged object の検査は `resolve_command_policy`
//!   経由で executable 宣言 shape・argv pattern compile・env/cwd
//!   規則を継承し、`version` const・`grants` の `maxProperties` により
//!   既存 file の pre-existing corruption も捕捉する — invalid spec・
//!   破損 file は一切書き換えない。`security.execDenyPatterns`
//!   (issue #164)の deny 照合もここで発火する — declared executable
//!   が user policy の deny に match する spec は add 時点で
//!   `InvalidRuntimePolicy` に fail-closed する(適用点 1)。
//! - `list` は parse + project のみ(validation・write を伴わない)。
//! - utility command 規約: `record_usage` は dispatch 側も含め非呼出
//!   し、success envelope の `result` に top-level `tokens` key を
//!   載せない(`usage_log::build_usage_record` の count-record 判定
//!   との衝突回避 — AC9・AC16)。

use std::path::{Path, PathBuf};

use cheaper_core::config::dirs::{resolve_user_config_dir, UserConfigDirOptions};
use cheaper_core::errors::{CheaperError, ErrorCode};
use cheaper_core::fs_util::{atomic_write, AtomicWriteOptions};
use cheaper_core::policy::load::read_policy_yaml;
use cheaper_core::policy::schema::{validate_policy_schema, PolicySchemaMode};
use cheaper_core::yaml::stringify_yaml;
use serde_json::{json, Map, Value};
use tracing::Instrument;

use crate::args::{PolicyAction, PolicyCliArgs};
use crate::cli::{BoxError, CliHooks, CliIo, CommandOut};
use crate::envelope::utility_success_envelope;
use crate::errors::invalid_request;

/// `policy grant` command 本体。`cheaper.cli.command` span
/// (design §instrumentation 配置)。utility command のため envelope は
/// `utility_success_envelope`(`worker: null`・`result.tokens` 不在)。
pub async fn execute(
    args: &PolicyCliArgs,
    io: &mut CliIo<'_>,
    hooks: &CliHooks,
    started_at: f64,
) -> Result<CommandOut, BoxError> {
    execute_inner(args, io, hooks, started_at)
        .instrument(tracing::info_span!(
            "cheaper.cli.command",
            command = "policy",
        ))
        .await
}

async fn execute_inner(
    args: &PolicyCliArgs,
    io: &mut CliIo<'_>,
    hooks: &CliHooks,
    started_at: f64,
) -> Result<CommandOut, BoxError> {
    // `resolve_config_dirs`(builtin dir・project dir 解決を含む)は使わず
    // user scope のみ直接解決する — `--cwd` は parse 段で reject 済みで
    // policy path は cwd に依存しない。
    let user_dir = resolve_user_config_dir(UserConfigDirOptions {
        env: Some(io.env),
        homedir: None,
    });
    // `load_policy` と同一の byte shape(nodepath join)で組み立てる。
    let policy_path =
        cheaper_core::nodepath::join(&[user_dir.to_string_lossy().as_ref(), "policy.yaml"]);
    let result = match &args.action {
        PolicyAction::GrantList => grant_list(&policy_path)?,
        PolicyAction::GrantAdd { name, spec } => grant_add(&policy_path, name, spec)?,
        PolicyAction::GrantRemove { name } => grant_remove(&policy_path, name)?,
    };
    Ok(CommandOut {
        envelope: utility_success_envelope(result, crate::cli::now(hooks) - started_at),
        exit_code: 0,
        model_line: None,
    })
}

/// `list` — name 順 sort の `{grants:[{name,executable}]}`。file
/// absent は空配列。parse + project のみで validation は伴わない
/// (read 段の yaml-parse / not-object 失敗は `read_policy_yaml` が
/// `InvalidRuntimePolicy` で surface する)。
fn grant_list(policy_path: &str) -> Result<Map<String, Value>, CheaperError> {
    let grants = match read_policy_yaml(policy_path, false)? {
        Some(policy) => {
            let mut entries: Vec<(&String, &Value)> = policy
                .get("command")
                .and_then(Value::as_object)
                .and_then(|c| c.get("grants"))
                .and_then(Value::as_object)
                .map(|g| g.iter().collect())
                .unwrap_or_default();
            entries.sort_by_key(|(name, _)| *name);
            entries
                .into_iter()
                .map(|(name, spec)| {
                    json!({
                        "name": name,
                        "executable": spec.get("executable").cloned().unwrap_or(Value::Null),
                    })
                })
                .collect()
        }
        None => Vec::new(),
    };
    let mut result = Map::new();
    result.insert("grants".to_string(), Value::Array(grants));
    Ok(result)
}

/// `add <name> <spec-json>` — spec parse → 既存 file 読取り(absent は
/// `{"version":1}` base)→ 構造前提 pre-check → merged full-object
/// `validate_policy_schema(Partial)` → atomic write(0600)。
fn grant_add(
    policy_path: &str,
    name: &str,
    spec: &str,
) -> Result<Map<String, Value>, CheaperError> {
    // spec は `command.grants` の map value shape — JSON object 必須で
    // `name` key を含まない(name は map key として載せる)。
    let spec_value = parse_grant_spec(spec)?;
    let mut policy = read_policy_yaml(policy_path, false)?.unwrap_or_else(|| {
        json!({"version": 1})
            .as_object()
            .expect("object literal")
            .clone()
    });
    // 構造前提 pre-check — 非 object の `command`/`command.grants` は
    // `InvalidRuntimePolicy`(silent に空 map へ畳み替えない)。
    if let Some(grants) = existing_grants(&policy, policy_path)? {
        if grants.contains_key(name) {
            // 既存名 — fail-loud(置き換えは `remove` → `add` の導線)。
            return Err(invalid_request(
                format!("grant already exists: {name}"),
                Some(Map::from_iter([("grant".to_string(), json!(name))])),
            ));
        }
    }
    let executable = spec_value.get("executable").cloned().unwrap_or(Value::Null);
    grants_map_mut(&mut policy)?.insert(name.to_string(), spec_value);
    write_validated(policy_path, policy)?;
    Ok(single_grant_result(name, &executable))
}

/// `remove <name>` — add と同じ構造前提 pre-check + delete + 同じ
/// full-object validation + 同じ write path。file absent・name 不在は
/// `InvalidRequest`。
fn grant_remove(policy_path: &str, name: &str) -> Result<Map<String, Value>, CheaperError> {
    let mut policy = read_policy_yaml(policy_path, false)?.ok_or_else(|| {
        invalid_request(
            format!("grant not found: {name}"),
            Some(Map::from_iter([("grant".to_string(), json!(name))])),
        )
    })?;
    let removed = match existing_grants(&policy, policy_path)?.and_then(|g| g.get(name)) {
        Some(spec) => spec.clone(),
        None => {
            return Err(invalid_request(
                format!("grant not found: {name}"),
                Some(Map::from_iter([("grant".to_string(), json!(name))])),
            ))
        }
    };
    grants_map_mut(&mut policy)?.remove(name);
    write_validated(policy_path, policy)?;
    let executable = removed.get("executable").cloned().unwrap_or(Value::Null);
    Ok(single_grant_result(name, &executable))
}

/// `add`/`remove` 共通の success `result` — `{grant:{name,executable}}`
/// (top-level `tokens` key は載せない — AC16)。
fn single_grant_result(name: &str, executable: &Value) -> Map<String, Value> {
    let mut grant = Map::new();
    grant.insert("name".to_string(), json!(name));
    grant.insert("executable".to_string(), executable.clone());
    let mut result = Map::new();
    result.insert("grant".to_string(), Value::Object(grant));
    result
}

/// spec-json を parse する。object でない・`name` key を含む入力は
/// `InvalidRequest`(spec は `command.grants` の map value shape であり
/// name を含まない)。cmdline 由来値は error message/details へ echo
/// しない(`grant.rs` の flag 値規約と同一)。
fn parse_grant_spec(spec: &str) -> Result<Value, CheaperError> {
    let value: Value = serde_json::from_str(spec)
        .map_err(|_| invalid_request("invalid grant spec: malformed JSON", None))?;
    let Value::Object(_) = value else {
        return Err(invalid_request(
            "invalid grant spec: expected a JSON object",
            None,
        ));
    };
    if value.get("name").is_some() {
        return Err(invalid_request(
            "invalid grant spec: `name` must not be included (it is the map key)",
            None,
        ));
    }
    // `--grant`(`grant.rs::build_invocation_layer`)と同じ CLI 入口
    // validation(issue #165 design §(c)) — `fixed:true`/`scope` は
    // route 不可視の grant を生むため CLI spec では reject する。
    // trusted YAML での直接 authoring は合法(schema は受理)。
    if value.get("fixed") == Some(&Value::Bool(true)) {
        return Err(invalid_request(
            "invalid grant spec: `fixed: true` is not allowed \
             (`policy grant` registers non-fixed command grants only)",
            None,
        ));
    }
    if value.get("scope").is_some() {
        return Err(invalid_request(
            "invalid grant spec: `scope` is not allowed \
             (it is derived from the grant's origin layer)",
            None,
        ));
    }
    Ok(value)
}

/// 構造前提 pre-check — `command` が存在し object でない場合、および
/// `command.grants` が存在し object でない場合は `InvalidRuntimePolicy`
/// で fail する(非 object を silent に空 map へ畳み替えない)。`command`
/// /`grants` 不在は `None`(write 側が新規 object を作る)。
fn existing_grants<'p>(
    policy: &'p Map<String, Value>,
    policy_path: &str,
) -> Result<Option<&'p Map<String, Value>>, CheaperError> {
    let Some(command) = policy.get("command") else {
        return Ok(None);
    };
    let Some(command) = command.as_object() else {
        return Err(structural_error(policy_path, "command"));
    };
    match command.get("grants") {
        None => Ok(None),
        Some(grants) => match grants.as_object() {
            Some(grants) => Ok(Some(grants)),
            None => Err(structural_error(policy_path, "command.grants")),
        },
    }
}

/// `command`/`command.grants` へ到達する mutable accessor — 不在なら
/// object を新設する。`existing_grants` の pre-check 通過後にのみ呼ぶ
/// (非 object は先に reject 済み)。
fn grants_map_mut(
    policy: &mut Map<String, Value>,
) -> Result<&mut Map<String, Value>, CheaperError> {
    let command = policy
        .entry("command".to_string())
        .or_insert_with(|| Value::Object(Map::new()))
        .as_object_mut()
        .expect("existing_grants pre-check guarantees object");
    Ok(command
        .entry("grants".to_string())
        .or_insert_with(|| Value::Object(Map::new()))
        .as_object_mut()
        .expect("existing_grants pre-check guarantees object"))
}

/// merged full object を `validate_policy_schema(Partial)` で検査し、
/// 通過した場合のみ `stringify_yaml` → `atomic_write` で書き込む。
/// 失敗時は file を一切変更しない。検査は `resolve_command_policy`
/// 経由の executable 宣言 shape・argv pattern compile・env/cwd 規則
/// に加え、`version` const・`maxProperties`・`security.execDenyPatterns`
/// の declared deny 照合を含む。
fn write_validated(policy_path: &str, policy: Map<String, Value>) -> Result<(), CheaperError> {
    let validated = validate_policy_schema(&Value::Object(policy), PolicySchemaMode::Partial)?;
    let body = stringify_yaml(&Value::Object(validated))
        .map_err(|e| CheaperError::new(ErrorCode::InternalError, format!("{e}")))?;
    let file = PathBuf::from(policy_path);
    // `atomic_write` は parent dir を作らない(caller 責務)ため
    // `<userConfigDir>` 不在に備えて先に作る。
    let dir = file
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    std::fs::create_dir_all(dir)
        .map_err(|e| io_error("policy directory の作成に失敗しました", &file, e))?;
    // `windows_remove_first: false` — `std::fs::rename` の
    // `MoveFileExW`+`MOVEFILE_REPLACE_EXISTING` で置換が成立し、事前
    // remove の非 atomic な窓を作らない(credentials 継承 workaround は
    // 持ち込まない)。POSIX `0600` — grant の `env` 値が secret を
    // 保持しうるため owner-only readable。
    atomic_write(
        &file,
        &body,
        &AtomicWriteOptions {
            windows_remove_first: false,
            #[cfg(unix)]
            unix_mode: Some(0o600),
        },
    )
    .map_err(|e| io_error("policy.yaml の保存に失敗しました", &file, e))
}

/// 構造前提違反(非 object の `command`/`command.grants`)の error —
/// schema の型制約と同じ判断を write 側で先に表面化する。
fn structural_error(policy_path: &str, section: &str) -> CheaperError {
    CheaperError::new(
        ErrorCode::InvalidRuntimePolicy,
        format!("policy {section} section is not an object: {policy_path}"),
    )
    .with_details(Map::from_iter([("file".to_string(), json!(policy_path))]))
}

/// write 失敗の io error — `InternalError` + `details {file, ioError}`
/// (`auth/store.rs::io_error` と同一 shape)。
fn io_error(message: &str, file: &Path, e: std::io::Error) -> CheaperError {
    let mut details = Map::new();
    details.insert("file".to_string(), Value::from(file.display().to_string()));
    details.insert("ioError".to_string(), Value::from(e.to_string()));
    CheaperError::new(ErrorCode::InternalError, format!("{message}: {e}")).with_details(details)
}
