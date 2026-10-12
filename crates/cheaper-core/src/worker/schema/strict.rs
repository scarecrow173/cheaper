//! AJV `strict: true` mode の再実装 — `schema.rs` から分離。

use serde_json::{Map, Value};

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
pub(super) fn strict_schema_check(schema: &Value) -> Result<(), ()> {
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

    /// Keyword が要求する data type を declared type が満たすか。
    /// AJV は `integer` を `number` の subtype として扱うため、
    /// `type: "integer"` は `minimum` 等の `number` 要求を満たす
    /// (`includes_type` は文脈 narrowing 方向の判定で、こちらは逆方向)。
    fn covers_type(types: &[&'static str], req: &'static str) -> bool {
        types
            .iter()
            .any(|t| *t == req || (*t == "integer" && req == "number"))
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
                    if !req.iter().any(|t| covers_type(&data_types, t)) {
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
