//! Port of `packages/core/src/worker/validate.ts`.

use std::collections::HashSet;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};

use crate::errors::{CheaperError, ErrorCode};
use crate::model::tier::{parse_tier, Tier};
use crate::nodepath;
use crate::tools::vocabulary::{is_scope_gated_tool, normalize_tool_names, ToolName};
use crate::worker::parse::ParsedWorkerFile;
use crate::worker::schema::{
    compile_schema, load_schema, parse_schema_ref, CompiledSchema, SchemaKind, SchemaRef,
    MAX_DETAIL_ERRORS,
};

/// `WorkerOrigin`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum WorkerOrigin {
    Project,
    User,
    Builtin,
    Explicit,
}

impl WorkerOrigin {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Project => "project",
            Self::User => "user",
            Self::Builtin => "builtin",
            Self::Explicit => "explicit",
        }
    }
}

/// `cheaper.model` section.
#[derive(Debug, Clone, PartialEq)]
pub struct WorkerModel {
    pub tier: Tier,
    /// Workload profile (issue #46). `output_per_input` = output tokens
    /// per input token, used by the auto tier classifier's effectiveCost.
    pub workload: Option<WorkerWorkload>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WorkerWorkload {
    pub output_per_input: f64,
}

/// `{schema: SchemaRef}` holder for input/output sections.
#[derive(Debug, Clone, PartialEq)]
pub struct WorkerIo {
    pub schema: SchemaRef,
}

/// `WorkerDefinition["cheaper"]`.
#[derive(Debug, Clone, PartialEq)]
pub struct WorkerCheaper {
    pub model: WorkerModel,
    pub input: Option<WorkerIo>,
    pub output: WorkerIo,
    /// `writeScope`: input field whose `string[]`/`{path}[]` value limits
    /// write/edit/mkdir targets (SPEC-0010).
    pub write_scope: Option<String>,
    /// `optionalTools`: `tools` に宣言された scope-gated tool
    /// (`is_scope_gated_tool`)の部分集合。content-gate(nonempty
    /// tasks/grants)不成立時に未登録へ縮退してよい tool の opt-in
    /// 宣言(ADR-034ESBcMTWxNCMDI9obdb8 Option C1)。未指定時は空 —
    /// 非 opt-in worker の意味論は不変。
    pub optional_tools: Vec<ToolName>,
}

/// Port of `WorkerDefinition`.
#[derive(Debug)]
pub struct WorkerDefinition {
    pub name: String,
    pub description: String,
    pub tools: Vec<ToolName>,
    pub cheaper: WorkerCheaper,
    /// Parsed body (the worker prompt).
    pub instructions: String,
    pub source_path: String,
    pub origin: WorkerOrigin,
    /// compiled input schema。`CompiledSchema` は immutable なので
    /// `Arc` で workflow 側の validation snapshot と共有できる。
    pub input_schema: Option<Arc<CompiledSchema>>,
    pub output_schema: Arc<CompiledSchema>,
}

const NAME_PATTERN: &str = "^[a-z0-9][a-z0-9-]*$";

/// worker name の正規表現（resolve/validate で共有する一権威定義）。
pub(crate) fn name_re() -> &'static regex::Regex {
    static RE: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();
    RE.get_or_init(|| regex::Regex::new(NAME_PATTERN).unwrap())
}

const CHEAPER_KEYS: &[&str] = &["model", "input", "output", "writeScope", "optionalTools"];
const MODEL_KEYS: &[&str] = &["tier", "workload"];
const WORKLOAD_KEYS: &[&str] = &["outputPerInput"];
const IO_KEYS: &[&str] = &["schema"];

fn invalid(source_path: &str, reason: &str) -> CheaperError {
    let mut details = Map::new();
    details.insert("sourcePath".to_string(), json!(source_path));
    details.insert("reason".to_string(), json!(reason));
    CheaperError::new(
        ErrorCode::InvalidWorkerDefinition,
        format!("invalid worker definition: {reason}"),
    )
    .with_details(details)
}

fn require_strict_keys(
    obj: &Map<String, Value>,
    allowed: &[&str],
    where_: &str,
    source_path: &str,
) -> Result<(), CheaperError> {
    for key in obj.keys() {
        if !allowed.contains(&key.as_str()) {
            return Err(invalid(
                source_path,
                &format!("unknown-field-{where_}-{key}"),
            ));
        }
    }
    Ok(())
}

/// `validateWorkerDefinition` (sync — `std::fs` per the plan).
pub fn validate_worker_definition(
    parsed: &ParsedWorkerFile,
    source_path: &str,
    origin: WorkerOrigin,
) -> Result<WorkerDefinition, CheaperError> {
    let Value::Object(fm) = &parsed.frontmatter else {
        return Err(invalid(source_path, "frontmatter-not-object"));
    };

    let name = match fm.get("name").and_then(Value::as_str) {
        Some(n) if !n.is_empty() => n.to_string(),
        _ => return Err(invalid(source_path, "name-missing")),
    };
    if !name_re().is_match(&name) {
        let mut details = Map::new();
        details.insert("sourcePath".to_string(), json!(source_path));
        details.insert("name".to_string(), json!(name));
        return Err(CheaperError::new(
            ErrorCode::InvalidWorkerName,
            format!("invalid worker name: {name}"),
        )
        .with_details(details));
    }
    let ext = nodepath::extname(source_path);
    let stem = nodepath::basename(source_path, Some(&ext));
    if stem != name {
        let mut details = Map::new();
        details.insert("sourcePath".to_string(), json!(source_path));
        details.insert("name".to_string(), json!(name));
        details.insert("stem".to_string(), json!(stem));
        return Err(CheaperError::new(
            ErrorCode::WorkerNameMismatch,
            format!("worker name {name} does not match file stem {stem}"),
        )
        .with_details(details));
    }

    let description = match fm.get("description").and_then(Value::as_str) {
        Some(d) if !crate::js::js_trim(d).is_empty() => d.to_string(),
        _ => return Err(invalid(source_path, "description-missing")),
    };
    let tools = normalize_tool_names(fm.get("tools").unwrap_or(&Value::Null))?;

    let cheaper = match fm.get("cheaper") {
        Some(Value::Object(m)) => m,
        _ => return Err(invalid(source_path, "cheaper-missing")),
    };
    require_strict_keys(cheaper, CHEAPER_KEYS, "cheaper", source_path)?;

    let model = match cheaper.get("model") {
        Some(Value::Object(m)) => m,
        _ => return Err(invalid(source_path, "model-missing")),
    };
    require_strict_keys(model, MODEL_KEYS, "model", source_path)?;
    let tier = match model.get("tier").and_then(parse_tier) {
        Some(t) => t,
        None => return Err(invalid(source_path, "tier-invalid")),
    };

    let mut output_per_input: Option<f64> = None;
    if let Some(workload) = model.get("workload") {
        let Value::Object(workload) = workload else {
            return Err(invalid(source_path, "workload-invalid"));
        };
        require_strict_keys(workload, WORKLOAD_KEYS, "workload", source_path)?;
        let opi = workload.get("outputPerInput");
        let valid = opi
            .and_then(Value::as_f64)
            .is_some_and(|v| v.is_finite() && v >= 0.0);
        if !valid {
            return Err(invalid(source_path, "workload-outputPerInput-invalid"));
        }
        output_per_input = opi.and_then(Value::as_f64);
    }

    let output = match cheaper.get("output") {
        Some(Value::Object(m)) => m,
        _ => return Err(invalid(source_path, "output-missing")),
    };
    require_strict_keys(output, IO_KEYS, "output", source_path)?;
    let output_ref = parse_schema_ref(
        output.get("schema").unwrap_or(&Value::Null),
        SchemaKind::Output,
        source_path,
    )?;

    let mut input_ref: Option<SchemaRef> = None;
    if let Some(input) = cheaper.get("input") {
        let Value::Object(input) = input else {
            return Err(invalid(source_path, "input-invalid"));
        };
        require_strict_keys(input, IO_KEYS, "input", source_path)?;
        input_ref = Some(parse_schema_ref(
            input.get("schema").unwrap_or(&Value::Null),
            SchemaKind::Input,
            source_path,
        )?);
    }

    let base_dir = nodepath::dirname(source_path);
    let output_schema = Arc::new(compile_schema(
        load_schema(&output_ref, &base_dir, SchemaKind::Output)?,
        SchemaKind::Output,
    )?);
    let input_schema = match &input_ref {
        Some(r) => Some(Arc::new(compile_schema(
            load_schema(r, &base_dir, SchemaKind::Input)?,
            SchemaKind::Input,
        )?)),
        None => None,
    };

    let write_scope = match cheaper.get("writeScope") {
        None => None,
        Some(Value::String(s)) if !s.is_empty() => Some(s.clone()),
        Some(_) => return Err(invalid(source_path, "writeScope-invalid")),
    };

    // `optionalTools` — opt-in で content-gate 不成立時に未登録へ縮退して
    // よい scope-gated tool の `tools` 真部分集合。全 reject は
    // `InvalidWorkerDefinition`(`invalid()` 経由。`tools` field の
    // `UnknownTool` code は使わない — ADR-034ESBcMTWxNCMDI9obdb8)。
    let optional_tools = match cheaper.get("optionalTools") {
        None => Vec::new(),
        Some(raw) => {
            let Some(arr) = raw.as_array() else {
                return Err(invalid(source_path, "optionalTools-invalid"));
            };
            if !arr.iter().all(Value::is_string) {
                return Err(invalid(source_path, "optionalTools-invalid"));
            }
            if arr.is_empty() {
                return Err(invalid(source_path, "optionalTools-empty"));
            }
            let mut resolved = Vec::with_capacity(arr.len());
            for v in arr {
                let name = v.as_str().expect("checked is_string");
                match ToolName::resolve(name) {
                    Some(tool) => resolved.push(tool),
                    None => return Err(invalid(source_path, "optionalTools-unknown-tool")),
                }
            }
            if resolved.iter().any(|t| !tools.contains(t)) {
                return Err(invalid(source_path, "optionalTools-undeclared"));
            }
            if resolved.iter().any(|t| !is_scope_gated_tool(*t)) {
                return Err(invalid(source_path, "optionalTools-not-gated"));
            }
            let mut seen = HashSet::with_capacity(resolved.len());
            if !resolved.iter().all(|t| seen.insert(*t)) {
                return Err(invalid(source_path, "optionalTools-duplicate"));
            }
            // `resolved ⊆ tools` は undeclared/duplicate 検査済みで保証
            // されるため、`tools` を全て覆う = tool 0 個の run を許す
            // nonsense 宣言をここで reject する。
            if tools.iter().all(|t| seen.contains(t)) {
                return Err(invalid(source_path, "optionalTools-not-proper-subset"));
            }
            resolved
        }
    };

    Ok(WorkerDefinition {
        name,
        description,
        tools,
        cheaper: WorkerCheaper {
            model: WorkerModel {
                tier,
                workload: output_per_input.map(|v| WorkerWorkload {
                    output_per_input: v,
                }),
            },
            input: input_ref.map(|schema| WorkerIo { schema }),
            output: WorkerIo { schema: output_ref },
            write_scope,
            optional_tools,
        },
        instructions: parsed.body.clone(),
        source_path: source_path.to_string(),
        origin,
        input_schema,
        output_schema,
    })
}

/// `MUTATING_TOOL_NAMES`: tools that may mutate the workspace.
/// `command` は child process の副作用が未知のため mutating 扱い —
/// `savings.directlyComparable` が `command` 使用 worker で false になる
/// 効果もここに集約される(design §(h))。
pub fn is_mutating_tool(name: ToolName) -> bool {
    matches!(
        name,
        ToolName::Write | ToolName::Edit | ToolName::Process | ToolName::Command
    )
}

/// `isDirectlyComparableWorker` (SPEC-034rzGnCaRQc5ceI95lf5i req 2):
/// only read-only workers (no write tools, no writeScope) produce results
/// directly comparable to parent-model execution.
pub fn is_directly_comparable_worker(worker: &WorkerDefinition) -> bool {
    worker.cheaper.write_scope.is_none() && !worker.tools.iter().any(|t| is_mutating_tool(*t))
}

/// `validateWorkerInput`.
pub fn validate_worker_input(def: &WorkerDefinition, input: &Value) -> Result<(), CheaperError> {
    let Some(schema) = &def.input_schema else {
        return Ok(());
    };
    match schema.validate(input) {
        Ok(()) => Ok(()),
        Err(errors) => {
            let mut details = Map::new();
            // `INVALID_OUTPUT` と同じ cap — envelope 肥大化防止
            // (issue #113)。違反箇所は先頭 20 件の instancePath で明示。
            let capped: Vec<_> = errors.iter().take(MAX_DETAIL_ERRORS).collect();
            details.insert(
                "errors".to_string(),
                serde_json::to_value(&capped).unwrap_or(Value::Null),
            );
            Err(CheaperError::new(
                ErrorCode::InvalidInput,
                "input does not match worker input schema",
            )
            .with_details(details))
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::worker::parse;

    const BASE_FM: &str = "name: w\ndescription: d\ntools: [read]\ncheaper:\n  model: {tier: low}\n  output: {schema: {inline: {type: string}}}";
    /// `tools` に gated tool(`process` / `command`)を含む variant —
    /// `optionalTools` test の base。
    const GATED_FM: &str = "name: w\ndescription: d\ntools: [read, process, command]\ncheaper:\n  model: {tier: low}\n  output: {schema: {inline: {type: string}}}";

    /// Build a worker file in a fresh tmpdir and validate it.
    fn validate(frontmatter: &str, stem: &str) -> Result<WorkerDefinition, CheaperError> {
        let dir = tempfile::tempdir().unwrap();
        let source_path = dir.path().join(format!("{stem}.md"));
        let content = format!("---\n{frontmatter}\n---\nbody\n");
        let parsed = parse::parse_worker_file(&content, source_path.to_str().unwrap()).unwrap();
        validate_worker_definition(
            &parsed,
            source_path.to_str().unwrap(),
            WorkerOrigin::Project,
        )
    }

    #[test]
    fn accepts_a_valid_definition() {
        let def = validate(BASE_FM, "w").unwrap();
        assert_eq!(def.name, "w");
        assert_eq!(def.cheaper.model.tier, Tier::Low);
        assert!(def.cheaper.input.is_none());
        assert!(def.input_schema.is_none());
        assert!(is_directly_comparable_worker(&def));
    }

    #[test]
    fn rejects_missing_name() {
        let e = validate("description: d\ntools: [read]\ncheaper:\n  model: {tier: low}\n  output: {schema: {inline: {}}}", "w").unwrap_err();
        assert_eq!(e.code(), ErrorCode::InvalidWorkerDefinition);
        assert_eq!(e.details().unwrap()["reason"], json!("name-missing"));
    }

    #[test]
    fn rejects_feff_only_description() {
        // JS `trim` strips U+FEFF — a FEFF-only description is missing.
        let fm = BASE_FM.replace("description: d", "description: \"\\uFEFF\"");
        let e = validate(&fm, "w").unwrap_err();
        assert_eq!(e.details().unwrap()["reason"], json!("description-missing"));
    }

    #[test]
    fn write_scope_on_is_string_under_yaml_12() {
        // YAML 1.2: `on` is a string, so `writeScope: on` is a valid
        // input-field name rather than a boolean (writeScope-invalid).
        let fm = BASE_FM.replace(
            "output: {schema: {inline: {type: string}}}",
            "output: {schema: {inline: {type: string}}}\n  writeScope: on",
        );
        let def = validate(&fm, "w").unwrap();
        assert_eq!(def.cheaper.write_scope.as_deref(), Some("on"));
    }

    #[test]
    fn rejects_bad_name_and_stem_mismatch() {
        let e = validate(BASE_FM.replace("name: w", "name: Bad Name").as_str(), "w").unwrap_err();
        assert_eq!(e.code(), ErrorCode::InvalidWorkerName);
        let e = validate(BASE_FM.replace("name: w", "name: other").as_str(), "w").unwrap_err();
        assert_eq!(e.code(), ErrorCode::WorkerNameMismatch);
    }

    #[test]
    fn rejects_unknown_fields_and_bad_tier() {
        let fm = BASE_FM.replace("model: {tier: low}", "model: {tier: low, extra: 1}");
        let e = validate(&fm, "w").unwrap_err();
        assert_eq!(
            e.details().unwrap()["reason"],
            json!("unknown-field-model-extra")
        );
        let fm = BASE_FM.replace("tier: low", "tier: ultra");
        let e = validate(&fm, "w").unwrap_err();
        assert_eq!(e.details().unwrap()["reason"], json!("tier-invalid"));
    }

    #[test]
    fn validates_workload_output_per_input() {
        let fm = BASE_FM.replace(
            "model: {tier: low}",
            "model: {tier: low, workload: {outputPerInput: 2.5}}",
        );
        let def = validate(&fm, "w").unwrap();
        assert_eq!(def.cheaper.model.workload.unwrap().output_per_input, 2.5);
        let fm = BASE_FM.replace(
            "model: {tier: low}",
            "model: {tier: low, workload: {outputPerInput: -1}}",
        );
        let e = validate(&fm, "w").unwrap_err();
        assert_eq!(
            e.details().unwrap()["reason"],
            json!("workload-outputPerInput-invalid")
        );
    }

    #[test]
    fn write_scope_and_mutating_tools_gate_comparability() {
        let fm = BASE_FM.replace(
            "output: {schema: {inline: {type: string}}}",
            "output: {schema: {inline: {type: string}}}\n  writeScope: targets",
        );
        let def = validate(&fm, "w").unwrap();
        assert_eq!(def.cheaper.write_scope.as_deref(), Some("targets"));
        assert!(!is_directly_comparable_worker(&def));

        let fm = BASE_FM.replace("tools: [read]", "tools: [read, write]");
        let def = validate(&fm, "w").unwrap();
        assert!(!is_directly_comparable_worker(&def));
        assert!(is_mutating_tool(ToolName::Write));
        assert!(is_mutating_tool(ToolName::Edit));
        assert!(is_mutating_tool(ToolName::Process));
        assert!(!is_mutating_tool(ToolName::Read));
    }

    #[test]
    fn optional_tools_defaults_to_empty() {
        let def = validate(BASE_FM, "w").unwrap();
        assert!(def.cheaper.optional_tools.is_empty());
    }

    #[test]
    fn accepts_and_normalizes_optional_tools() {
        let fm = GATED_FM.replace("cheaper:", "cheaper:\n  optionalTools: [process, command]");
        let def = validate(&fm, "w").unwrap();
        assert_eq!(
            def.cheaper.optional_tools,
            [ToolName::Process, ToolName::Command]
        );
    }

    #[test]
    fn rejects_optional_tools_non_array_and_non_string() {
        for opt in [
            "optionalTools: process",
            "optionalTools: {a: 1}",
            "optionalTools: [process, 3]",
        ] {
            let fm = GATED_FM.replace("cheaper:", &format!("cheaper:\n  {opt}"));
            let e = validate(&fm, "w").unwrap_err();
            assert_eq!(e.code(), ErrorCode::InvalidWorkerDefinition, "{opt}");
            assert_eq!(
                e.details().unwrap()["reason"],
                json!("optionalTools-invalid"),
                "{opt}"
            );
        }
    }

    #[test]
    fn rejects_optional_tools_empty_array() {
        let fm = GATED_FM.replace("cheaper:", "cheaper:\n  optionalTools: []");
        let e = validate(&fm, "w").unwrap_err();
        assert_eq!(e.details().unwrap()["reason"], json!("optionalTools-empty"));
    }

    #[test]
    fn rejects_optional_tools_unknown_tool() {
        let fm = GATED_FM.replace("cheaper:", "cheaper:\n  optionalTools: [bash]");
        let e = validate(&fm, "w").unwrap_err();
        assert_eq!(e.code(), ErrorCode::InvalidWorkerDefinition);
        assert_eq!(
            e.details().unwrap()["reason"],
            json!("optionalTools-unknown-tool")
        );
    }

    #[test]
    fn rejects_optional_tools_undeclared() {
        // `command` は解決できるが `tools` 未宣言。
        let fm = "name: w\ndescription: d\ntools: [read, process]\ncheaper:\n  optionalTools: [command]\n  model: {tier: low}\n  output: {schema: {inline: {type: string}}}";
        let e = validate(fm, "w").unwrap_err();
        assert_eq!(
            e.details().unwrap()["reason"],
            json!("optionalTools-undeclared")
        );
    }

    #[test]
    fn rejects_optional_tools_not_gated() {
        // `read` は宣言済みだが scope-gated でない。alias 名 `Read` も
        // `ToolName::resolve` で正規化された上で同じ検査に落ちる。
        for opt in ["optionalTools: [read]", "optionalTools: [Read]"] {
            let fm = GATED_FM.replace("cheaper:", &format!("cheaper:\n  {opt}"));
            let e = validate(&fm, "w").unwrap_err();
            assert_eq!(
                e.details().unwrap()["reason"],
                json!("optionalTools-not-gated"),
                "{opt}"
            );
        }
    }

    #[test]
    fn rejects_optional_tools_duplicates() {
        let fm = GATED_FM.replace("cheaper:", "cheaper:\n  optionalTools: [process, process]");
        let e = validate(&fm, "w").unwrap_err();
        assert_eq!(
            e.details().unwrap()["reason"],
            json!("optionalTools-duplicate")
        );
    }

    #[test]
    fn rejects_optional_tools_covering_all_tools() {
        // `optionalTools ⊇ tools` — 全 tool が縮退し得る宣言は reject。
        let fm = "name: w\ndescription: d\ntools: [process, command]\ncheaper:\n  optionalTools: [process, command]\n  model: {tier: low}\n  output: {schema: {inline: {type: string}}}";
        let e = validate(fm, "w").unwrap_err();
        assert_eq!(
            e.details().unwrap()["reason"],
            json!("optionalTools-not-proper-subset")
        );
    }

    #[test]
    fn validate_worker_input_uses_input_schema() {
        let fm = BASE_FM.replace(
            "cheaper:",
            "cheaper:\n  input: {schema: {inline: {type: object, required: [x], properties: {x: {}}}}}",
        );
        let def = validate(&fm, "w").unwrap();
        assert!(validate_worker_input(&def, &json!({"x": 1})).is_ok());
        let e = validate_worker_input(&def, &json!({})).unwrap_err();
        assert_eq!(e.code(), ErrorCode::InvalidInput);
        let errors = e.details().unwrap()["errors"].as_array().unwrap();
        assert_eq!(errors[0]["instancePath"], json!(""));
    }

    #[test]
    fn validate_worker_input_caps_error_details() {
        // 21 件超の違反でも `details.errors` は MAX_DETAIL_ERRORS(20)に
        // cap — `INVALID_OUTPUT` 経路と同じ診断面(issue #113)。
        // instancePath は違反箇所の明示として残る。
        let props: String = (0..25)
            .map(|i| format!("p{i}: {{type: integer}}"))
            .collect::<Vec<_>>()
            .join(", ");
        let fm = format!(
            "cheaper:\n  input: {{schema: {{inline: {{type: object, properties: {{{props}}}}}}}}}\n  model: {{tier: low}}\n  output: {{schema: {{inline: {{type: string}}}}}}"
        );
        let fm = BASE_FM.replace(
            "cheaper:\n  model: {tier: low}\n  output: {schema: {inline: {type: string}}}",
            &fm,
        );
        let def = validate(&fm, "w").unwrap();
        let input: serde_json::Map<String, Value> = (0..25)
            .map(|i| (format!("p{i}"), json!("not-an-integer")))
            .collect();
        let e = validate_worker_input(&def, &Value::Object(input)).unwrap_err();
        assert_eq!(e.code(), ErrorCode::InvalidInput);
        let errors = e.details().unwrap()["errors"].as_array().unwrap();
        assert_eq!(errors.len(), MAX_DETAIL_ERRORS);
        assert!(errors
            .iter()
            .all(|entry| entry["instancePath"].as_str().unwrap().starts_with("/p")));
    }
}
