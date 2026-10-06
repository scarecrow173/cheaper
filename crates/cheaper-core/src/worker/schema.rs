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

/// One entry of the `{instancePath, message}` error array contract.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SchemaErrorEntry {
    #[serde(rename = "instancePath")]
    pub instance_path: String,
    pub message: String,
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
        let errors = self
            .validator
            .iter_errors(data)
            .map(|err| SchemaErrorEntry {
                // JSON Pointer; ajv's instancePath is the same format.
                instance_path: err.instance_path().to_string(),
                message: err.to_string(),
            })
            .collect();
        Err(errors)
    }
}

/// Approximation of AJV 8 `strict: true` rejection at `ajv.compile`
/// (`packages/core/src/worker/schema.ts` uses
/// `new Ajv2020({ strict: true, allErrors: true })`).
/// `jsonschema::draft202012::new` treats unknown keywords as annotations,
/// so the strict-mode checks are reproduced here as a recursive walk of
/// the *compiled* schema positions:
///
/// - unknown keywords outside the AJV Draft 2020-12 `RULES.keywords`
///   allowlist → `unknown keyword`;
/// - `format` present in a compiled position → `unknown format` (this
///   project registers no format plugin, so AJV rejects every format —
///   even standard names like `email`/`uuid`/`date-time`);
/// - `strictTypes`: each keyword's required type(s) must be present in
///   the inherited data-type context (new-data positions reset to
///   unconstrained; `propertyNames` constrains to `string`; same-data
///   applicators — `allOf`/`anyOf`/`oneOf`/`not`/`if`/`then`/`else`/
///   `dependentSchemas`/object-valued `dependencies` — inherit and
///   narrow); union `type` with >2 members (or 2 non-`null`) errors;
/// - `strictTuples`: `prefixItems` requires `minItems` == len and
///   (`maxItems` == len or `items: false`);
/// - `strictRequired`: every non-empty `required` key must exist in
///   `properties`;
/// - `then`/`else` without `if` and `if` without `then`/`else` error,
///   and the `if` subschema is compiled only when a `then`/`else`
///   keyword carries rules (always-valid `then`/`else` leaves `if`
///   unchecked);
/// - `$defs`/`definitions`/`contentSchema` are lazy positions: content
///   is checked only when reachable via a local `#/…`/`#` `$ref` from a
///   compiled position (refs are chased transitively, fresh type
///   context).
///
/// Residual divergence: anchor/`$dynamicRef`/`$recursiveRef` targets and
/// `$id`-scoped resolution are not chased; `$ref` JSON-pointer unescape
/// is limited to `~0`/`~1` (no percent-decoding).
fn strict_schema_check(schema: &Value) -> Result<(), ()> {
    /// AJV `RULES.keywords` for the bundled Draft 2020-12 vocabulary —
    /// the strict-mode "known keyword" allowlist.
    fn is_known_keyword(k: &str) -> bool {
        matches!(
            k,
            "$async"
                | "$dynamicAnchor"
                | "$dynamicRef"
                | "$recursiveAnchor"
                | "$recursiveRef"
                | "$schema"
                | "$id"
                | "$defs"
                | "$vocabulary"
                | "$comment"
                | "definitions"
                | "id"
                | "$ref"
                | "maximum"
                | "minimum"
                | "exclusiveMaximum"
                | "exclusiveMinimum"
                | "multipleOf"
                | "maxLength"
                | "minLength"
                | "pattern"
                | "maxProperties"
                | "minProperties"
                | "required"
                | "maxItems"
                | "minItems"
                | "uniqueItems"
                | "type"
                | "nullable"
                | "const"
                | "enum"
                | "not"
                | "anyOf"
                | "oneOf"
                | "allOf"
                | "if"
                | "then"
                | "else"
                | "propertyNames"
                | "additionalProperties"
                | "dependencies"
                | "properties"
                | "patternProperties"
                | "prefixItems"
                | "items"
                | "contains"
                | "format"
                | "title"
                | "description"
                | "default"
                | "deprecated"
                | "readOnly"
                | "writeOnly"
                | "examples"
                | "contentMediaType"
                | "contentEncoding"
                | "contentSchema"
                | "dependentRequired"
                | "dependentSchemas"
                | "maxContains"
                | "minContains"
                | "unevaluatedProperties"
                | "unevaluatedItems"
        )
    }

    /// AJV `RULES.all` — keywords that count as validation rules for
    /// `alwaysValidSchema` (the `if` laziness gate). Annotation-only
    /// subschemas skip compilation.
    fn is_rule_keyword(k: &str) -> bool {
        matches!(
            k,
            "$dynamicAnchor"
                | "$dynamicRef"
                | "$recursiveAnchor"
                | "$recursiveRef"
                | "$comment"
                | "id"
                | "$ref"
                | "maximum"
                | "minimum"
                | "exclusiveMaximum"
                | "exclusiveMinimum"
                | "multipleOf"
                | "maxLength"
                | "minLength"
                | "pattern"
                | "maxProperties"
                | "minProperties"
                | "required"
                | "maxItems"
                | "minItems"
                | "uniqueItems"
                | "type"
                | "nullable"
                | "const"
                | "enum"
                | "not"
                | "anyOf"
                | "oneOf"
                | "allOf"
                | "if"
                | "then"
                | "else"
                | "propertyNames"
                | "additionalProperties"
                | "dependencies"
                | "properties"
                | "patternProperties"
                | "prefixItems"
                | "items"
                | "contains"
                | "format"
                | "dependentRequired"
                | "dependentSchemas"
                | "maxContains"
                | "minContains"
                | "unevaluatedProperties"
                | "unevaluatedItems"
        )
    }

    /// strictTypes keyword → required JSON types.
    fn keyword_types(k: &str) -> Option<&'static [&'static str]> {
        match k {
            "maximum" | "minimum" | "exclusiveMaximum" | "exclusiveMinimum" | "multipleOf" => {
                Some(&["number"])
            }
            "maxLength" | "minLength" | "pattern" => Some(&["string"]),
            "maxProperties"
            | "minProperties"
            | "required"
            | "propertyNames"
            | "additionalProperties"
            | "dependencies"
            | "properties"
            | "patternProperties"
            | "dependentRequired"
            | "dependentSchemas"
            | "unevaluatedProperties" => Some(&["object"]),
            "maxItems" | "minItems" | "uniqueItems" | "prefixItems" | "items" | "contains"
            | "maxContains" | "minContains" | "unevaluatedItems" => Some(&["array"]),
            _ => None,
        }
    }

    fn json_type(t: &str) -> Result<&'static str, ()> {
        match t {
            "string" => Ok("string"),
            "number" => Ok("number"),
            "integer" => Ok("integer"),
            "boolean" => Ok("boolean"),
            "null" => Ok("null"),
            "object" => Ok("object"),
            "array" => Ok("array"),
            _ => Err(()),
        }
    }

    fn includes_type(types: &[&'static str], t: &'static str) -> bool {
        types.contains(&t) || (t == "integer" && types.contains(&"number"))
    }

    /// `getSchemaTypes`: `type` (+ `nullable`) → allowed JSON types.
    /// Malformed values are rejected (AJV throws or the meta-schema
    /// fails — the same compile-rejection boundary).
    fn schema_types(s: &Map<String, Value>) -> Result<Vec<&'static str>, ()> {
        let mut types: Vec<&'static str> = Vec::new();
        match s.get("type") {
            None => {}
            Some(Value::String(t)) => types.push(json_type(t)?),
            Some(Value::Array(ts)) => {
                for t in ts {
                    match t {
                        Value::String(t) => types.push(json_type(t)?),
                        _ => return Err(()),
                    }
                }
            }
            Some(_) => return Err(()),
        }
        match s.get("nullable") {
            None => Ok(types),
            Some(Value::Bool(true)) => {
                if types.is_empty() {
                    // `"nullable" cannot be used without "type"`.
                    return Err(());
                }
                if !types.contains(&"null") {
                    types.push("null");
                }
                Ok(types)
            }
            Some(Value::Bool(false)) => {
                if types.is_empty() || types.contains(&"null") {
                    // `"nullable" cannot be used without "type"` /
                    // `type: null contradicts nullable: false`.
                    return Err(());
                }
                Ok(types)
            }
            // `data/nullable must be boolean` (meta-schema).
            Some(_) => Err(()),
        }
    }

    /// `alwaysValidSchema`: boolean → itself; `{}` → true; objects
    /// without any rule keyword → true. Unknown keywords throw.
    fn always_valid_schema(v: &Value) -> Result<bool, ()> {
        match v {
            Value::Bool(b) => Ok(*b),
            Value::Object(o) if o.is_empty() => Ok(true),
            Value::Object(o) => {
                for k in o.keys() {
                    if !is_known_keyword(k) {
                        return Err(());
                    }
                }
                Ok(!o.keys().any(|k| is_rule_keyword(k)))
            }
            _ => Err(()),
        }
    }

    /// `ToPropertyKey` for `strictRequired` (`props?.[requiredKey]`):
    /// a required entry of `5` matches `properties.{"5"}` etc.
    fn property_key(v: &Value) -> String {
        match v {
            Value::String(s) => s.clone(),
            Value::Null => "null".to_string(),
            Value::Bool(b) => b.to_string(),
            Value::Number(n) => match n.as_f64() {
                Some(f) if f.fract() == 0.0 && f.abs() <= 9_007_199_254_740_992.0 => {
                    format!("{f:.0}")
                }
                _ => n.to_string(),
            },
            _ => v.to_string(),
        }
    }

    /// Resolve a local `#`/`#/…` JSON-pointer `$ref` (`~0`/`~1` unescape).
    fn resolve_pointer<'a>(root: &'a Value, r: &str) -> Option<&'a Value> {
        if r == "#" {
            return Some(root);
        }
        let mut cur = root;
        for raw in r.strip_prefix("#/")?.split('/') {
            let seg = raw.replace("~1", "/").replace("~0", "~");
            cur = match cur {
                Value::Object(o) => o.get(&seg)?,
                Value::Array(a) => a.get(seg.parse::<usize>().ok()?)?,
                _ => return None,
            };
        }
        Some(cur)
    }

    struct State<'a> {
        root: &'a Value,
        pending: Vec<String>,
        visited: std::collections::HashSet<String>,
    }

    impl State<'_> {
        /// Check one compiled schema-position value (boolean schemas
        /// are trivially valid). `ctx` is the inherited data-type
        /// context (`[]` = unconstrained, `["string"]` inside
        /// `propertyNames`).
        fn node(&mut self, v: &Value, ctx: &[&'static str]) -> Result<(), ()> {
            let obj = match v {
                Value::Bool(_) => return Ok(()),
                Value::Object(o) => o,
                // Meta-invalid schema value → compile rejection.
                _ => return Err(()),
            };

            // Unknown keywords (AJV `checkUnknownRules`).
            for k in obj.keys() {
                if !is_known_keyword(k) {
                    return Err(());
                }
            }

            // strictTypes: declared types, context check, narrowing.
            let types = schema_types(obj)?;
            let data_types: Vec<&'static str> = if types.is_empty() {
                ctx.to_vec()
            } else if ctx.is_empty() {
                types.clone()
            } else {
                for t in &types {
                    if !includes_type(ctx, t) {
                        // `type "x" not allowed by context`.
                        return Err(());
                    }
                }
                let mut narrowed: Vec<&'static str> = Vec::new();
                for ct in ctx {
                    if includes_type(&types, ct) {
                        narrowed.push(ct);
                    } else if *ct == "number" && types.contains(&"integer") {
                        narrowed.push("integer");
                    }
                }
                narrowed
            };
            // Union `type` (>2, or 2 without `null`) → allowUnionTypes error.
            if types.len() > 2 || (types.len() == 2 && !types.contains(&"null")) {
                return Err(());
            }
            // Keyword type requirements.
            for k in obj.keys() {
                if let Some(req) = keyword_types(k) {
                    if !req.iter().any(|t| includes_type(&data_types, t)) {
                        return Err(());
                    }
                }
            }

            // `format`: this project registers no formats, so any format
            // keyword in a compiled position is `unknown format`.
            if obj.contains_key("format") {
                return Err(());
            }

            // strictTuples.
            if let Some(pi) = obj.get("prefixItems") {
                match pi {
                    Value::Array(items) => {
                        let len = items.len() as f64;
                        let min_ok = obj.get("minItems").and_then(Value::as_f64) == Some(len);
                        let max_ok = obj.get("maxItems").and_then(Value::as_f64) == Some(len)
                            || obj.get("items") == Some(&Value::Bool(false));
                        if !(min_ok && max_ok) {
                            return Err(());
                        }
                    }
                    _ => return Err(()),
                }
            }

            // strictRequired (non-empty `required` keys ⊆ `properties`).
            if let Some(req) = obj.get("required") {
                match req {
                    Value::Array(keys) if !keys.is_empty() => {
                        let props = obj.get("properties").and_then(Value::as_object);
                        for k in keys {
                            let key = property_key(k);
                            if props.and_then(|p| p.get(&key)).is_none() {
                                return Err(());
                            }
                        }
                    }
                    Value::Array(_) => {}
                    _ => return Err(()),
                }
            }

            // if/then/else pairing + `if` laziness.
            let has_if = obj.contains_key("if");
            let has_then_key = obj.contains_key("then");
            let has_else_key = obj.contains_key("else");
            if !has_if && (has_then_key || has_else_key) {
                return Err(());
            }
            if has_if && !has_then_key && !has_else_key {
                return Err(());
            }

            // Collect local `$ref`s from compiled positions.
            if let Some(Value::String(r)) = obj.get("$ref") {
                if r == "#" || r.starts_with("#/") {
                    self.pending.push(r.clone());
                }
            }

            // Descend into compiled subschema positions. New-data
            // positions reset the context to unconstrained.
            for key in ["properties", "patternProperties"] {
                if let Some(m) = obj.get(key) {
                    match m {
                        Value::Object(m) => {
                            for v in m.values() {
                                self.node(v, &[])?;
                            }
                        }
                        _ => return Err(()),
                    }
                }
            }
            for key in [
                "additionalProperties",
                "items",
                "contains",
                "unevaluatedProperties",
                "unevaluatedItems",
            ] {
                if let Some(v) = obj.get(key) {
                    self.node(v, &[])?;
                }
            }
            if let Some(Value::Array(items)) = obj.get("prefixItems") {
                for v in items {
                    self.node(v, &[])?;
                }
            }
            if let Some(v) = obj.get("propertyNames") {
                self.node(v, &["string"])?;
            }

            // Same-data positions inherit the narrowed context.
            for key in ["allOf", "anyOf", "oneOf"] {
                if let Some(arr) = obj.get(key) {
                    match arr {
                        Value::Array(items) => {
                            for v in items {
                                self.node(v, &data_types)?;
                            }
                        }
                        _ => return Err(()),
                    }
                }
            }
            if let Some(v) = obj.get("not") {
                self.node(v, &data_types)?;
            }
            if has_if {
                let then_compiled = has_then_key && !always_valid_schema(&obj["then"])?;
                let else_compiled = has_else_key && !always_valid_schema(&obj["else"])?;
                if then_compiled || else_compiled {
                    self.node(&obj["if"], &data_types)?;
                }
                if then_compiled {
                    self.node(&obj["then"], &data_types)?;
                }
                if else_compiled {
                    self.node(&obj["else"], &data_types)?;
                }
            }
            if let Some(m) = obj.get("dependentSchemas") {
                match m {
                    Value::Object(m) => {
                        for v in m.values() {
                            self.node(v, &data_types)?;
                        }
                    }
                    _ => return Err(()),
                }
            }
            if let Some(m) = obj.get("dependencies") {
                match m {
                    Value::Object(m) => {
                        for v in m.values() {
                            match v {
                                Value::Object(_) => self.node(v, &data_types)?,
                                // Array values are `dependentRequired`
                                // data; other shapes are meta-invalid.
                                Value::Array(_) => {}
                                _ => return Err(()),
                            }
                        }
                    }
                    _ => return Err(()),
                }
            }
            // `$defs`/`definitions`/`contentSchema`: lazy positions, not
            // descended here (see the `$ref` worklist in `check`).

            Ok(())
        }

        fn check(&mut self) -> Result<(), ()> {
            let root = self.root;
            self.node(root, &[])?;
            while let Some(r) = self.pending.pop() {
                if !self.visited.insert(r.clone()) {
                    continue;
                }
                // Unresolvable references fail inside the validator
                // compile (`can't resolve reference`) — same boundary.
                if let Some(target) = resolve_pointer(self.root, &r) {
                    self.node(target, &[])?;
                }
            }
            Ok(())
        }
    }

    let mut st = State {
        root: schema,
        pending: Vec::new(),
        visited: std::collections::HashSet::new(),
    };
    st.check()
}

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
    if strict_schema_check(&root).is_err() {
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
