//! Port of `packages/tool-runtime/src/catalog.ts`.
//!
//! Tool metadata catalog: input/output JSON schema(TypeBox 生成物の
//! literal 化)と `resources(input)` 宣言。副作用なし — workspace/model/
//! helper の構築は不要。`get`/`list` は呼び出しごとに新しい metadata を
//! 返す(TS `list()` が毎回 `structuredClone` するのと同じ分離)。

use cheaper_core::policy::process::{
    process_task_resources, resolve_process_policy, ResolvedProcessPolicy,
};
use cheaper_core::resources::normalization::{ResourceManifestDeclaration, ScopeDeclaration};
use cheaper_core::resources::types::ResourceScope;
use cheaper_core::tools::vocabulary::ToolName;
use serde::Serialize;
use serde_json::{json, Value};

use crate::tools::process::{process_details_schema, process_schema};
use crate::ToolError;

/// `JsonValue`(TS `null | boolean | number | string | JsonValue[] |
/// {[k:string]: JsonValue}`)。serde_json::Value がそのまま対応する。
pub type JsonValue = Value;

/// `ToolMetadata`。`resources` は input の既知アクセス範囲を宣言する
/// (unknown は `None` bucket / process task 解決失敗は Err)。
pub struct ToolMetadata {
    pub name: ToolName,
    pub resources: ResourcesDeclaration,
    pub input_schema: Value,
    pub output_schema: Value,
}

/// `resources?(input)` callback。
pub type ResourcesDeclaration =
    Box<dyn Fn(&JsonValue) -> Result<ResourceManifestDeclaration, ToolError> + Send + Sync>;

/// `ToolCatalog`。`Clone` は `ResolvedProcessPolicy` の共有(resource
/// access 層・registry・invocation が同一 catalog を共有するため)。
#[derive(Clone)]
pub struct ToolCatalog {
    policy: ResolvedProcessPolicy,
}

impl ToolCatalog {
    /// `catalog.get(name)`。
    pub fn get(&self, name: &str) -> Option<ToolMetadata> {
        self.list().into_iter().find(|t| t.name.as_str() == name)
    }

    /// `catalog.list()`。呼び出しごとに metadata を構築する。
    pub fn list(&self) -> Vec<ToolMetadata> {
        NAMES.iter().map(|&name| self.metadata(name)).collect()
    }

    fn metadata(&self, name: ToolName) -> ToolMetadata {
        let policy = self.policy.clone();
        let resources: ResourcesDeclaration = match name {
            ToolName::Process => Box::new(move |input| {
                // `(input as {task}).task` — schema 検証前は task 欠落が
                // あり得る。TS は `undefined` を渡し `resolveProcessTask` が
                // 未知 task error にする。ここでも非 string は空文字で同じ
                // error 経路に乗せる。
                let task = input
                    .get("task")
                    .and_then(Value::as_str)
                    .unwrap_or_default();
                process_task_resources(&policy, task).map_err(|e| Box::new(e) as ToolError)
            }),
            _ => Box::new(move |input| Ok(resources(name, input))),
        };
        ToolMetadata {
            name,
            resources,
            input_schema: input_schema(name),
            output_schema: output_schema(name),
        }
    }
}

/// `{type:"text", text}` content item。
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ToolContent {
    #[serde(rename = "type")]
    pub content_type: &'static str,
    pub text: String,
}

impl ToolContent {
    /// `{type:"text", text}`。
    pub fn text(text: impl Into<String>) -> Self {
        Self {
            content_type: "text",
            text: text.into(),
        }
    }
}

/// `CanonicalToolResult`。
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct CanonicalToolResult {
    pub content: Vec<ToolContent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<Value>,
}

impl CanonicalToolResult {
    /// `{content: [{type:"text", text}]}`(`details` なし)。
    pub fn text(text: impl Into<String>) -> Self {
        Self {
            content: vec![ToolContent::text(text)],
            details: None,
        }
    }
}

const NAMES: &[ToolName] = &[
    ToolName::Read,
    ToolName::Write,
    ToolName::Edit,
    ToolName::Ls,
    ToolName::Grep,
    ToolName::Find,
    ToolName::CountTokens,
    ToolName::Process,
];

// ---------------------------------------------------------------------------
// input schemas(TypeBox 生成物の literal。key 順は TypeBox emission 順
// `type → required → properties → additionalProperties` に揃える)。
// ---------------------------------------------------------------------------

/// `readSchema`(`packages/tool-runtime/src/read.ts`)。
pub fn read_schema() -> Value {
    json!({
        "anyOf": [
            {
                "type": "object",
                "required": ["path"],
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Workspace-relative file path",
                        "pattern": "^(?!artifact://)",
                    },
                    "offset": {
                        "type": "integer",
                        "minimum": 1,
                        "description": "1-indexed start line (default 1)",
                    },
                    "limit": {
                        "type": "integer",
                        "minimum": 1,
                        "maximum": 2000,
                        "default": 2000,
                    },
                    "byteInLine": {
                        "type": "integer",
                        "minimum": 0,
                        "description": "Byte offset within the `offset` line, for continuing an oversized line",
                    },
                },
                "additionalProperties": false,
            },
            {
                "type": "object",
                "required": ["path", "artifact"],
                "properties": {
                    "path": {"type": "string", "pattern": "^(?!artifact://)"},
                    "artifact": {"type": "string", "minLength": 1},
                },
                "additionalProperties": false,
            },
            {
                "type": "object",
                "required": ["source", "offset", "maxBytes"],
                "properties": {
                    "source": {
                        "type": "object",
                        "required": ["kind", "reference"],
                        "properties": {
                            "kind": {"type": "string", "const": "artifact"},
                            "reference": artifact_reference_schema(),
                        },
                        "additionalProperties": false,
                    },
                    "offset": {"type": "integer", "minimum": 0, "maximum": cheaper_core::js::MAX_SAFE_INTEGER_I64},
                    "maxBytes": {"type": "integer", "minimum": 1, "maximum": cheaper_core::js::MAX_SAFE_INTEGER_I64},
                },
                "additionalProperties": false,
            },
        ],
        "type": "object",
    })
}

/// `artifactReferenceSchema`(`read.ts` export)。
pub fn artifact_reference_schema() -> Value {
    json!({
        "type": "object",
        "required": ["artifact", "runId", "producer", "bytes", "preview", "truncated"],
        "properties": {
            "artifact": {"type": "string"},
            "runId": {"type": "string"},
            "producer": {
                "type": "object",
                "required": ["jobId", "name"],
                "properties": {
                    "jobId": {"type": "string"},
                    "name": {"type": "string"},
                },
                "additionalProperties": false,
            },
            "bytes": {"type": "integer", "minimum": 0, "maximum": cheaper_core::js::MAX_SAFE_INTEGER_I64},
            "preview": {"type": "string"},
            "truncated": {"type": "boolean"},
        },
        "additionalProperties": false,
    })
}

/// `writeSchema`(catalog.ts 内の Pi write 準拠 schema)。
pub fn write_schema() -> Value {
    json!({
        "type": "object",
        "required": ["path", "content"],
        "properties": {
            "path": {
                "type": "string",
                "description": "Path to the file to write (relative or absolute)",
            },
            "content": {
                "type": "string",
                "description": "Content to write to the file",
            },
        },
    })
}

/// `editSchema`(catalog.ts 内の Pi edit 準拠 schema)。
pub fn edit_schema() -> Value {
    json!({
        "type": "object",
        "required": ["path", "edits"],
        "properties": {
            "path": {
                "type": "string",
                "description": "Path to the file to edit (relative or absolute)",
            },
            "edits": {
                "type": "array",
                "items": {
                    "type": "object",
                    "required": ["oldText", "newText"],
                    "properties": {
                        "oldText": {
                            "type": "string",
                            "description": "Exact text for one targeted replacement. It must be unique in the original file and must not overlap with any other edits[].oldText in the same call.",
                        },
                        "newText": {
                            "type": "string",
                            "description": "Replacement text for this targeted edit.",
                        },
                    },
                },
                "description": "One or more targeted replacements. Each edit is matched against the original file, not incrementally. Do not include overlapping or nested edits. If two changes touch the same block or nearby lines, merge them into one edit instead.",
            },
        },
    })
}

/// `lsSchema`(catalog.ts 内の Pi ls 準拠 schema)。
pub fn ls_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "path": {
                "type": "string",
                "description": "Directory to list (default: current directory)",
            },
            "limit": {
                "type": "number",
                "description": "Maximum number of entries to return (default: 500)",
            },
        },
    })
}

/// `grepSchema`(`packages/tool-runtime/src/grep.ts`)。
pub fn grep_schema() -> Value {
    json!({
        "type": "object",
        "required": ["pattern"],
        "properties": {
            "pattern": {
                "type": "string",
                "description": "JavaScript regular expression",
            },
            "path": {
                "type": "string",
                "description": "Search root, workspace-relative (default: .)",
            },
            "glob": {
                "type": "string",
                "description": "Glob filter on workspace-relative paths",
            },
            "ignoreCase": {"type": "boolean"},
            "maxResults": {"type": "integer", "minimum": 1, "maximum": 500, "default": 100},
            "context": {"type": "integer", "minimum": 0, "maximum": 3, "default": 0},
        },
    })
}

/// `findSchema`(`packages/tool-runtime/src/find.ts`)。
pub fn find_schema() -> Value {
    json!({
        "type": "object",
        "required": ["pattern"],
        "properties": {
            "pattern": {
                "type": "string",
                "description": "Glob matched against workspace-relative paths",
            },
            "path": {
                "type": "string",
                "description": "Search root, workspace-relative (default: .)",
            },
            "maxResults": {"type": "integer", "minimum": 1, "maximum": 1000, "default": 200},
        },
    })
}

/// `countTokensSchema`(`packages/tool-runtime/src/count-tokens.ts`)。
pub fn count_tokens_schema() -> Value {
    json!({
        "type": "object",
        "required": ["path"],
        "properties": {
            "path": {
                "type": "string",
                "description": "Workspace-relative file path to count tokens for",
            },
        },
    })
}

fn input_schema(name: ToolName) -> Value {
    match name {
        ToolName::Read => read_schema(),
        ToolName::Write => write_schema(),
        ToolName::Edit => edit_schema(),
        ToolName::Ls => ls_schema(),
        ToolName::Grep => grep_schema(),
        ToolName::Find => find_schema(),
        ToolName::CountTokens => count_tokens_schema(),
        ToolName::Process => process_schema(),
    }
}

// ---------------------------------------------------------------------------
// output schemas(`resultSchema` の clone + name 別 details)。
// ---------------------------------------------------------------------------

/// `resultSchema`(`details` は caller 側で差し替える)。
fn result_schema() -> Value {
    json!({
        "type": "object",
        "required": ["content"],
        "additionalProperties": false,
        "properties": {
            "content": {
                "type": "array",
                "items": {
                    "type": "object",
                    "required": ["type", "text"],
                    "additionalProperties": false,
                    "properties": {
                        "type": {"const": "text"},
                        "text": {"type": "string"},
                    },
                },
            },
            "details": {"$ref": "#/$defs/json"},
        },
        "$defs": {
            "json": {
                "anyOf": [
                    {"type": "null"},
                    {"type": "boolean"},
                    {"type": "number"},
                    {"type": "string"},
                    {"type": "array", "items": {"$ref": "#/$defs/json"}},
                    {"type": "object", "additionalProperties": {"$ref": "#/$defs/json"}},
                ],
            },
        },
    })
}

/// `readDetails`。
fn read_details_schema() -> Value {
    json!({
        "type": "object",
        "required": ["startLine", "endLine", "eof", "truncatedBy"],
        "properties": {
            "startLine": {"type": "integer"},
            "endLine": {"type": "integer"},
            "eof": {"type": "boolean"},
            "truncatedBy": {
                "anyOf": [
                    {"type": "string", "const": "limit"},
                    {"type": "string", "const": "bytes"},
                    {"type": "null"},
                ],
            },
            "continuation": {
                "type": "object",
                "required": ["offset"],
                "properties": {
                    "offset": {"type": "integer"},
                    "byteInLine": {"type": "integer"},
                },
            },
            "totalLines": {"type": "integer"},
        },
    })
}

/// `countDetails`。
fn count_details_schema() -> Value {
    json!({
        "type": "object",
        "required": ["path", "tokens", "bytes", "tokenizer"],
        "properties": {
            "path": {"type": "string"},
            "tokens": {"type": "integer", "minimum": 0},
            "bytes": {"type": "integer", "minimum": 0},
            "tokenizer": {
                "type": "object",
                "required": ["name", "implementation", "version"],
                "properties": {
                    "name": {"type": "string"},
                    "implementation": {"type": "string"},
                    "version": {"type": "string"},
                },
            },
        },
    })
}

/// `outputSchema(name)`: `read`/`count_tokens`/`process` は details を
/// 専用 schema に差し替え required に `"details"` を追加する。
fn output_schema(name: ToolName) -> Value {
    let mut result = result_schema();
    match name {
        ToolName::Read | ToolName::CountTokens | ToolName::Process => {
            let details = match name {
                ToolName::Read => json!({
                    "anyOf": [
                        read_details_schema(),
                        {
                            "type": "object",
                            "required": ["artifact"],
                            "properties": {
                                "artifact": artifact_reference_schema(),
                            },
                            "additionalProperties": false,
                        },
                        {
                            "type": "object",
                            "required": ["offset", "nextOffset", "eof"],
                            "properties": {
                                "offset": {"type": "integer", "minimum": 0},
                                "nextOffset": {"type": "integer", "minimum": 0},
                                "eof": {"type": "boolean"},
                            },
                            "additionalProperties": false,
                        },
                    ],
                }),
                ToolName::Process => process_details_schema(),
                _ => count_details_schema(),
            };
            let properties = result["properties"]
                .as_object_mut()
                .expect("resultSchema has properties");
            properties.insert("details".to_string(), details);
            let required = result["required"]
                .as_array_mut()
                .expect("resultSchema has required");
            required.push(json!("details"));
            result
        }
        _ => result,
    }
}

// ---------------------------------------------------------------------------
// resources() 宣言
// ---------------------------------------------------------------------------

/// `resources(name, input)`。
///
/// - read + `source` key → 全 bucket 空の既知宣言(artifact byte read は
///   workspace access を伴わない)
/// - `ls`/`grep`/`find` は subtree scope、それ以外は exact scope
/// - 未知 path(absolute・drive letter・非 string)は `None` bucket
/// - `write` は writes のみ、`edit` は reads+writes
fn resources(name: ToolName, input: &JsonValue) -> ResourceManifestDeclaration {
    if name == ToolName::Read && input.get("source").is_some() {
        return ResourceManifestDeclaration {
            reads: Some(Vec::new()),
            writes: Some(Vec::new()),
            produces: Some(Vec::new()),
        };
    }
    let subtree = matches!(name, ToolName::Ls | ToolName::Grep | ToolName::Find);
    // `input.path ?? (subtree ? "." : undefined)` — `??` は nullish
    // coalescing(`path: null` も fallback)。input が object 以外のときは
    // fallback 自体を適用しない(undefined = 未知)。
    let value: Option<Value> = match input {
        Value::Object(obj) => match obj.get("path") {
            Some(v) if !v.is_null() => Some(v.clone()),
            _ => subtree.then(|| json!(".")),
        },
        _ => None,
    };
    // absolute input は D5 relative declaration に変換せず未知として公開する。
    let known = matches!(
        &value,
        Some(Value::String(v)) if !v.is_empty() && !is_unknown_absolute(v)
    );
    let scope = if known {
        Some(vec![ScopeDeclaration {
            path: value.expect("known").as_str().expect("string").to_string(),
            scope: if subtree {
                ResourceScope::Subtree
            } else {
                ResourceScope::Exact
            },
        }])
    } else {
        None
    };
    ResourceManifestDeclaration {
        reads: if name == ToolName::Write {
            Some(Vec::new())
        } else {
            scope.clone()
        },
        writes: if matches!(name, ToolName::Write | ToolName::Edit) {
            scope
        } else {
            Some(Vec::new())
        },
        produces: Some(Vec::new()),
    }
}

/// `path.isAbsolute(v) || path.win32.isAbsolute(v) || /^[a-z]:/i`。
/// `path.isAbsolute` は posix 環境では `/` 始まりのみ、win32 では
/// `/`・`\`・drive letter。ここでは host 非依存に両方言語を拒否する
/// (catalog の宣言は host の path semantic に依存しない保守側の判定)。
fn is_unknown_absolute(value: &str) -> bool {
    // posix isAbsolute: "/" prefix。win32 isAbsolute: "/" or "\" prefix,
    // drive-letter prefix("c:\\", "c:/", "c:" は isAbsolute ではないが
    // 別 regex で捕捉)。
    if value.starts_with('/') || value.starts_with('\\') {
        return true;
    }
    // /^[a-z]:/i
    let bytes = value.as_bytes();
    bytes.len() >= 2 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':'
}

/// `createToolCatalog({process})`。`resolveProcessPolicy` の失敗は Err
/// として伝播する(TS では constructor が同期 throw)。
pub fn create_tool_catalog(process: Option<&Value>) -> Result<ToolCatalog, ToolError> {
    let policy = resolve_process_policy(process).map_err(|e| Box::new(e) as ToolError)?;
    Ok(ToolCatalog { policy })
}

impl ToolCatalog {
    /// resolve 済み policy からの直接構築(registry/invocation が
    /// `resolve_process_policy` 済みの値を共有する seam。serialize →
    /// 再 resolve の二重経路を避ける)。
    pub fn from_resolved_policy(policy: ResolvedProcessPolicy) -> Self {
        Self { policy }
    }
}

/// `meter.run` の result から `content[].text` を引く duck-typing seam
/// (accounting.rs `ResultContent` の canonical 実装 — 所有は catalog)。
impl crate::accounting::ResultContent for CanonicalToolResult {
    fn content_texts(&self) -> Vec<&str> {
        self.content
            .iter()
            .filter(|item| item.content_type == "text")
            .map(|item| item.text.as_str())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use cheaper_core::resources::normalization::ResourceManifestDeclaration;

    fn manifest_of(cat: &ToolCatalog, name: &str, input: &Value) -> ResourceManifestDeclaration {
        (cat.get(name).expect(name).resources)(input).expect("resources")
    }

    #[test]
    fn catalog_lists_all_canonical_tools_in_order() {
        let cat = create_tool_catalog(None).unwrap();
        let names: Vec<&str> = cat.list().iter().map(|t| t.name.as_str()).collect();
        assert_eq!(
            names,
            [
                "read",
                "write",
                "edit",
                "ls",
                "grep",
                "find",
                "count_tokens",
                "process"
            ]
        );
    }

    #[test]
    fn catalog_metadata_isolation_and_unknown() {
        let cat = create_tool_catalog(None).unwrap();
        let a = cat.get("read").unwrap();
        let b = cat.get("read").unwrap();
        // 毎回新しい Value(分離)だが内容は等しい。
        assert_eq!(a.input_schema, b.input_schema);
        assert_eq!(a.output_schema["required"], json!(["content", "details"]));
        assert!(cat.get("nonexistent").is_none());
    }

    #[test]
    fn catalog_resources_subtree_vs_exact() {
        let cat = create_tool_catalog(None).unwrap();
        let read = manifest_of(&cat, "read", &json!({"path": "src/main.ts"}));
        assert_eq!(read.reads.as_ref().unwrap()[0].scope, ResourceScope::Exact);
        assert_eq!(read.reads.as_ref().unwrap()[0].path, "src/main.ts");
        assert_eq!(read.writes.as_ref().unwrap().len(), 0);

        let ls = manifest_of(&cat, "ls", &json!({}));
        assert_eq!(ls.reads.as_ref().unwrap()[0].scope, ResourceScope::Subtree);
        assert_eq!(ls.reads.as_ref().unwrap()[0].path, ".");

        let write = manifest_of(&cat, "write", &json!({"path": "a.txt"}));
        assert_eq!(write.reads.as_ref().unwrap().len(), 0);
        assert_eq!(
            write.writes.as_ref().unwrap()[0].scope,
            ResourceScope::Exact
        );

        let edit = manifest_of(&cat, "edit", &json!({"path": "a.txt"}));
        assert_eq!(edit.reads.as_ref().unwrap()[0].path, "a.txt");
        assert_eq!(edit.writes.as_ref().unwrap()[0].path, "a.txt");

        let find = manifest_of(&cat, "find", &json!({"path": "src"}));
        assert_eq!(
            find.reads.as_ref().unwrap()[0].scope,
            ResourceScope::Subtree
        );
    }

    #[test]
    fn catalog_resources_artifact_source_and_unknown_path() {
        let cat = create_tool_catalog(None).unwrap();
        // read + source → 全 bucket 空の既知宣言。
        let m = manifest_of(
            &cat,
            "read",
            &json!({"source": {"kind": "artifact", "reference": {}}}),
        );
        assert_eq!(m.reads.as_ref().unwrap().len(), 0);
        assert_eq!(m.writes.as_ref().unwrap().len(), 0);
        assert_eq!(m.produces.as_ref().unwrap().len(), 0);
        // absolute path は unknown(None bucket)。
        for p in ["/abs/x", "C:\\x", "\\\\share\\x"] {
            let m = manifest_of(&cat, "read", &json!({"path": p}));
            assert!(m.reads.is_none(), "{p}");
        }
    }

    #[test]
    fn catalog_output_schemas() {
        let cat = create_tool_catalog(None).unwrap();
        let read = cat.get("read").unwrap();
        assert_eq!(
            read.output_schema["properties"]["details"]["anyOf"][0]["required"],
            json!(["startLine", "endLine", "eof", "truncatedBy"])
        );
        // write 等は details 差し替えなしの result schema。
        let write = cat.get("write").unwrap();
        assert_eq!(write.output_schema["required"], json!(["content"]));
        let count = cat.get("count_tokens").unwrap();
        assert_eq!(
            count.output_schema["properties"]["details"]["required"],
            json!(["path", "tokens", "bytes", "tokenizer"])
        );
    }
}
