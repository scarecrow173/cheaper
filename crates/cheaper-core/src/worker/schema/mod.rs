//! Port of `packages/core/src/worker/schema.ts`.
//!
//! Ajv 2020 is replaced by `jsonschema` draft 2020-12 (`draft202012::new`).
//! Validation errors map to `{instancePath, message}`; `instancePath` is
//! the only contractual field (ajv `instancePath` and `jsonschema`
//! `Location` both render JSON Pointer).

use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};

use crate::errors::{CheaperError, ErrorCode};
use crate::nodepath;

/// `SchemaRef`: `{file}` / `{inline}` are exclusive.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SchemaRef {
    File(String),
    Inline(Map<String, Value>),
}

/// `SchemaKind`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SchemaKind {
    Input,
    Output,
}

impl SchemaKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Input => "input",
            Self::Output => "output",
        }
    }

    fn invalid_code(self) -> ErrorCode {
        match self {
            Self::Input => ErrorCode::InvalidInputSchema,
            Self::Output => ErrorCode::InvalidOutputSchema,
        }
    }

    fn not_found_code(self) -> ErrorCode {
        match self {
            Self::Input => ErrorCode::InputSchemaNotFound,
            Self::Output => ErrorCode::OutputSchemaNotFound,
        }
    }
}

const JSON_SCHEMA_2020: &str = "https://json-schema.org/draft/2020-12/schema";

/// `parseSchemaRef`.
pub fn parse_schema_ref(
    raw: &Value,
    kind: SchemaKind,
    source_path: &str,
) -> Result<SchemaRef, CheaperError> {
    let fail = |reason: &str| -> CheaperError {
        let mut details = Map::new();
        details.insert("sourcePath".to_string(), json!(source_path));
        details.insert("reason".to_string(), json!(reason));
        details.insert("kind".to_string(), json!(kind.as_str()));
        CheaperError::new(
            ErrorCode::InvalidWorkerDefinition,
            format!("invalid {} schema ref: {reason}", kind.as_str()),
        )
        .with_details(details)
    };
    let Value::Object(record) = raw else {
        return Err(fail("schema-not-object"));
    };
    let has_file = record.contains_key("file");
    let has_inline = record.contains_key("inline");
    if has_file == has_inline {
        return Err(fail("schema-exactly-one"));
    }
    if has_file {
        return match record["file"].as_str() {
            Some(f) if !f.is_empty() => Ok(SchemaRef::File(f.to_string())),
            _ => Err(fail("schema-file-not-string")),
        };
    }
    match record.get("inline") {
        Some(Value::Object(m)) => Ok(SchemaRef::Inline(m.clone())),
        _ => Err(fail("schema-inline-not-object")),
    }
}

/// `loadSchema`: `.json` / `.yaml` / `.yml` extension restriction,
/// NOT_FOUND / INVALID_*_SCHEMA mapping.
pub fn load_schema(
    schema_ref: &SchemaRef,
    base_dir: &str,
    kind: SchemaKind,
) -> Result<Map<String, Value>, CheaperError> {
    if let SchemaRef::Inline(inline) = schema_ref {
        return Ok(inline.clone());
    }
    let SchemaRef::File(file) = schema_ref else {
        unreachable!()
    };
    let ext = nodepath::extname(file).to_lowercase();
    if ext != ".json" && ext != ".yaml" && ext != ".yml" {
        let mut details = Map::new();
        details.insert("file".to_string(), json!(file));
        details.insert("reason".to_string(), json!("schema-extension"));
        return Err(CheaperError::new(
            kind.invalid_code(),
            format!("unsupported schema extension: {file}"),
        )
        .with_details(details));
    }
    let file_path = nodepath::resolve(base_dir, file);
    // Node readFile("utf8") replaces invalid UTF-8; mirror with lossy decode.
    let bytes = std::fs::read(&file_path).map_err(|e| {
        let mut details = Map::new();
        details.insert("file".to_string(), json!(file));
        details.insert("path".to_string(), json!(file_path));
        CheaperError::new(
            kind.not_found_code(),
            format!("schema file not found: {file}"),
        )
        .with_details(details)
        .with_cause(e)
    })?;
    let text = String::from_utf8_lossy(&bytes);
    let parsed: Result<Value, String> = if ext == ".json" {
        serde_json::from_str(&text).map_err(|e| e.to_string())
    } else {
        crate::yaml::parse_yaml(&text).map_err(|e| e.to_string())
    };
    match parsed {
        Ok(Value::Object(m)) => Ok(m),
        Ok(_) => Err(schema_parse_error(file, kind)),
        Err(_) => Err(schema_parse_error(file, kind)),
    }
}

fn schema_parse_error(file: &str, kind: SchemaKind) -> CheaperError {
    let mut details = Map::new();
    details.insert("file".to_string(), json!(file));
    details.insert("reason".to_string(), json!("schema-parse"));
    CheaperError::new(kind.invalid_code(), format!("schema parse failed: {file}"))
        .with_details(details)
}

/// error `details.errors` に入れる上限 — `INVALID_INPUT`/
/// `INVALID_OUTPUT` 両経路で共通(envelope 肥大化防止、issue #113)。
pub const MAX_DETAIL_ERRORS: usize = 20;

/// One entry of the `{instancePath, message}` error array contract.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SchemaErrorEntry {
    #[serde(rename = "instancePath")]
    pub instance_path: String,
    pub message: String,
}

/// ajv `validate.errors` 相当の error entry 配列を構築する共有 helper。
/// `instance_path` は JSON Pointer（ajv の `instancePath` と同形式）。
pub(crate) fn schema_error_entries(
    validator: &jsonschema::Validator,
    data: &Value,
) -> Vec<SchemaErrorEntry> {
    validator
        .iter_errors(data)
        .map(|err| SchemaErrorEntry {
            instance_path: err.instance_path().to_string(),
            message: err.to_string(),
        })
        .collect()
}

/// Port of `CompiledSchema`.
pub struct CompiledSchema {
    pub schema: Map<String, Value>,
    validator: jsonschema::Validator,
}

impl std::fmt::Debug for CompiledSchema {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CompiledSchema")
            .field("schema", &self.schema)
            .finish_non_exhaustive()
    }
}

impl CompiledSchema {
    /// `validate(data)`: `{ok: true}` → `Ok(())`, else the error entries.
    pub fn validate(&self, data: &Value) -> Result<(), Vec<SchemaErrorEntry>> {
        if self.validator.is_valid(data) {
            return Ok(());
        }
        let errors = schema_error_entries(&self.validator, data);
        Err(errors)
    }
}

mod strict;
/// `compileSchema`: a declared `$schema` other than draft 2020-12 is
/// rejected before compile; compile errors map to `schema-compile`.
pub fn compile_schema(
    schema: Map<String, Value>,
    kind: SchemaKind,
) -> Result<CompiledSchema, CheaperError> {
    if let Some(declared) = schema.get("$schema") {
        if declared.as_str() != Some(JSON_SCHEMA_2020) {
            let mut details = Map::new();
            details.insert("reason".to_string(), json!("schema-draft"));
            details.insert("$schema".to_string(), declared.clone());
            return Err(CheaperError::new(
                kind.invalid_code(),
                format!("unsupported $schema: {}", crate::js::js_string(declared)),
            )
            .with_details(details));
        }
    }
    let compile_error = |e: SchemaCompileError| {
        let mut details = Map::new();
        details.insert("reason".to_string(), json!("schema-compile"));
        CheaperError::new(kind.invalid_code(), "schema compile failed")
            .with_details(details)
            .with_cause(e)
    };
    let root = Value::Object(schema.clone());
    if strict::strict_schema_check(&root).is_err() {
        return Err(compile_error(SchemaCompileError(
            "ajv strict-mode rejection".to_string(),
        )));
    }
    let validator = jsonschema::draft202012::new(&root)
        .map_err(|e| compile_error(SchemaCompileError(e.to_string())))?;
    Ok(CompiledSchema { schema, validator })
}

/// Wrapper so the `jsonschema` compile failure can be an `Error::source`
/// (its `ValidationError` borrows; store the display text).
#[derive(Debug)]
struct SchemaCompileError(String);

impl std::fmt::Display for SchemaCompileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for SchemaCompileError {}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn parses_file_and_inline_refs() {
        let r = parse_schema_ref(&json!({"file": "s.json"}), SchemaKind::Output, "w.md").unwrap();
        assert_eq!(r, SchemaRef::File("s.json".to_string()));
        let r = parse_schema_ref(
            &json!({"inline": {"type": "object"}}),
            SchemaKind::Input,
            "w.md",
        )
        .unwrap();
        assert!(matches!(r, SchemaRef::Inline(_)));
    }

    #[test]
    fn rejects_invalid_refs() {
        for (raw, reason) in [
            (json!("x"), "schema-not-object"),
            (json!({"file": "a", "inline": {}}), "schema-exactly-one"),
            (json!({}), "schema-exactly-one"),
            (json!({"file": ""}), "schema-file-not-string"),
            (json!({"file": 3}), "schema-file-not-string"),
            (json!({"inline": "x"}), "schema-inline-not-object"),
        ] {
            let e = parse_schema_ref(&raw, SchemaKind::Input, "w.md").unwrap_err();
            assert_eq!(e.code(), ErrorCode::InvalidWorkerDefinition);
            assert_eq!(e.details().unwrap()["reason"], json!(reason));
            assert_eq!(e.details().unwrap()["kind"], json!("input"));
        }
    }

    #[test]
    fn load_schema_extension_restriction() {
        let dir = tempfile::tempdir().unwrap();
        let e = load_schema(
            &SchemaRef::File("s.txt".to_string()),
            dir.path().to_str().unwrap(),
            SchemaKind::Output,
        )
        .unwrap_err();
        assert_eq!(e.code(), ErrorCode::InvalidOutputSchema);
        assert_eq!(e.details().unwrap()["reason"], json!("schema-extension"));
    }

    #[test]
    fn load_schema_not_found() {
        let dir = tempfile::tempdir().unwrap();
        let e = load_schema(
            &SchemaRef::File("missing.json".to_string()),
            dir.path().to_str().unwrap(),
            SchemaKind::Input,
        )
        .unwrap_err();
        assert_eq!(e.code(), ErrorCode::InputSchemaNotFound);
    }

    #[test]
    fn load_schema_json_and_yaml() {
        let dir = tempfile::tempdir().unwrap();
        let base = dir.path().to_str().unwrap();
        std::fs::write(dir.path().join("s.json"), r#"{"type":"object"}"#).unwrap();
        std::fs::write(dir.path().join("s.yaml"), "type: string\n").unwrap();
        let j = load_schema(
            &SchemaRef::File("s.json".to_string()),
            base,
            SchemaKind::Input,
        )
        .unwrap();
        assert_eq!(j["type"], json!("object"));
        let y = load_schema(
            &SchemaRef::File("s.yaml".to_string()),
            base,
            SchemaKind::Input,
        )
        .unwrap();
        assert_eq!(y["type"], json!("string"));
        let bad = dir.path().join("bad.json");
        std::fs::write(&bad, "{ not json").unwrap();
        let e = load_schema(
            &SchemaRef::File("bad.json".to_string()),
            base,
            SchemaKind::Input,
        )
        .unwrap_err();
        assert_eq!(e.code(), ErrorCode::InvalidInputSchema);
        assert_eq!(e.details().unwrap()["reason"], json!("schema-parse"));
    }

    #[test]
    fn compile_rejects_non_2020_draft() {
        let e = compile_schema(
            Map::from_iter([(
                "$schema".to_string(),
                json!("http://json-schema.org/draft-07/schema#"),
            )]),
            SchemaKind::Input,
        )
        .unwrap_err();
        assert_eq!(e.code(), ErrorCode::InvalidInputSchema);
        assert_eq!(e.details().unwrap()["reason"], json!("schema-draft"));
    }

    #[test]
    fn compiled_schema_reports_instance_paths() {
        let schema = Map::from_iter([
            ("type".to_string(), json!("object")),
            ("required".to_string(), json!(["x"])),
            ("properties".to_string(), json!({"x": {"type": "integer"}})),
        ]);
        let compiled = compile_schema(schema, SchemaKind::Input).unwrap();
        assert!(compiled.validate(&json!({"x": 1})).is_ok());
        let errors = compiled.validate(&json!({"x": "s"})).unwrap_err();
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].instance_path, "/x");
        let errors = compiled.validate(&json!({})).unwrap_err();
        assert_eq!(errors[0].instance_path, "");
    }

    /// AJV `strict: true` parity cases — expected outcomes probed
    /// against the project's ajv 8.20.0 (`new Ajv2020({strict:true})`).
    #[test]
    fn compile_applies_ajv_strict_mode() {
        let reject = |s: Value| {
            let Value::Object(m) = s else {
                panic!("test schema must be an object")
            };
            let e = compile_schema(m, SchemaKind::Input).unwrap_err();
            assert_eq!(e.code(), ErrorCode::InvalidInputSchema);
            assert_eq!(e.details().unwrap()["reason"], json!("schema-compile"));
        };
        let accept = |s: Value| {
            let Value::Object(m) = s else {
                panic!("test schema must be an object")
            };
            compile_schema(m, SchemaKind::Input).unwrap();
        };

        // Unknown keywords at compiled positions.
        reject(json!({"type": "object", "propertiez": {}}));
        reject(json!({"properties": {"a": {"typo": 1}}}));
        reject(json!({"minLength": 1, "if": {}})); // `then` absent → pairing
        reject(json!({"if": {"type": "string"}})); // `if` without then/else
        reject(json!({"then": true})); // `then` without `if`
                                       // `if` is lazily compiled: unknown keys inside `if` pass when
                                       // `then`/`else` carry no rules (probed: ok).
        accept(json!({"if": {"typo": 1}, "then": true}));
        accept(json!({"if": {"typo": 1}, "then": {}}));
        reject(json!({"if": {"typo": 1}, "then": {"type": "string"}}));
        reject(json!({"if": {"typo": 1}, "then": {"typo": 2}}));
        accept(json!({"if": {}, "then": {}}));
        // $defs/definitions are lazy positions.
        accept(json!({"$defs": {"a": {"typo": 1}}}));
        accept(json!({"definitions": {"a": {"typo": 1}}}));
        reject(json!({"$defs": {"a": {"typo": 1}}, "$ref": "#/$defs/a"}));
        reject(json!({"$ref": "#/properties/a", "type": "object",
                      "properties": {"a": {"typo": 1}}}));
        reject(
            json!({"type": "object", "properties": {"a": {"$ref": "#/$defs/i"}},
                      "$defs": {"i": {"minimum": 1}}}),
        ); // ref'd, missing type
           // contentSchema is never compiled.
        accept(json!({"contentSchema": {"typo": 1}}));
        // Unknown formats — every format is unregistered here.
        reject(json!({"type": "string", "format": "email"}));
        reject(json!({"type": "string", "format": "uuid"}));
        accept(json!({"enum": [{"format": "email"}]})); // data position
                                                        // strictTypes.
        reject(json!({"items": []})); // items without array context (meta-invalid shape too)
        reject(json!({"items": {"type": "string"}})); // `items` needs array ctx
        reject(json!({"type": "object", "minContains": 1}));
        reject(json!({"minLength": 1}));
        reject(json!({"type": "object", "allOf": [{"type": "string"}]}));
        reject(json!({"type": "object", "oneOf": [{"type": "null"}]}));
        reject(json!({"type": "object", "properties": {"a": {"minLength": 1}}}));
        reject(json!({"dependentSchemas": {"a": {"type": "string"}}}));
        reject(json!({"type": "object", "dependentSchemas": {"a": {"type": "string"}}}));
        accept(json!({"type": "object", "dependentSchemas": {"a": {"type": "object"}}}));
        reject(json!({"type": "object", "not": {"type": "string"}}));
        reject(json!({"type": "object", "unevaluatedProperties": {"minLength": 1}}));
        reject(json!({"type": "object", "propertyNames": {"type": "number"}}));
        accept(json!({"type": "object", "propertyNames": {"minLength": 1}}));
        accept(json!({"type": "array", "items": {"type": "object"}}));
        accept(json!({"allOf": [{"type": "number"}, {"type": "integer"}]}));
        // Applicator siblings each get the parent context — no
        // progressive narrowing between `allOf` elements (probed: ok).
        accept(json!({"allOf": [{"type": "integer"}, {"type": "number"}]}));
        // Union types.
        accept(json!({"type": ["object", "null"]}));
        reject(json!({"type": ["object", "string", "null"]}));
        reject(json!({"type": ["object", "string"]}));
        // nullable.
        accept(json!({"type": "string", "nullable": true}));
        reject(json!({"nullable": true}));
        reject(json!({"type": ["string", "null"], "nullable": false}));
        // strictTuples.
        reject(json!({"type": "array", "prefixItems": [{}]}));
        reject(json!({"type": "array", "prefixItems": [{}], "minItems": 1}));
        accept(json!({"type": "array", "prefixItems": [{}], "minItems": 1, "items": false}));
        accept(json!({"type": "array", "prefixItems": [{}], "minItems": 1, "maxItems": 1}));
        // strictRequired.
        reject(json!({"type": "object", "required": ["a"]}));
        accept(json!({"type": "object", "required": ["a"], "properties": {"a": {}}}));
        accept(json!({"type": "object", "required": []}));
        // Misc probed rejects.
        reject(json!({"bogus": 1, "type": "object"}));
        reject(json!({"bogus": 1})); // schema without type still checked
        reject(json!({"prefixItems": [{"type": "string"}], "additionalItems": {}}));
        reject(json!({"type": "nonsense"}));
        reject(json!({"format": "x"})); // format without context types
        accept(json!({"type": "object"}));
        accept(json!({}));
        accept(
            json!({"type": "object", "properties": {"a": {"type": "string"}},
                      "required": ["a"]}),
        );
    }
}
