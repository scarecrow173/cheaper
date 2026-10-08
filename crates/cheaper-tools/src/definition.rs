//! Port of `packages/tool-runtime/src/definition.ts`.
//!
//! Pi に依存しない内部 descriptor(`CanonicalToolDefinition`)と、
//! 構築時 schema compile + 実行入口 1 回検証の executor。
//! 公開実行入口は registry 側(この層は dispatch を持たない)。

use cheaper_core::errors::{CheaperError, ErrorCode};
use cheaper_core::tools::vocabulary::ToolName;
use cheaper_core::worker::schema::{compile_schema, CompiledSchema, SchemaKind};
use serde_json::{json, Map, Value};

use crate::catalog::CanonicalToolResult;
use crate::operation::AbortSignal;
use crate::ToolError;

/// tool `execute` の戻り値 future。fs tool は同期処理だが process は
/// async subprocess 監理のため canonical seam は future で一本化する
/// (TS の `Promise<CanonicalToolResult>` parity)。`'a` は callback が
/// 借用する options 群の lifetime(root / denylist / journal 等)。
pub type CanonicalExecuteFuture<'a> = std::pin::Pin<
    Box<dyn std::future::Future<Output = Result<CanonicalToolResult, ToolError>> + Send + 'a>,
>;

/// tool `execute` callback。schema 検証済み input(`Static<T>` 相当の
/// `Value`)と optional abort signal を owned で受け取る — 返り値
/// future が呼出し scope を超えて生きうるため。`signal` は canonical
/// `AbortSignal`(未指定時は `None` — tool 側で detached 相当に扱う)。
pub type CanonicalExecute<'a> =
    dyn Fn(Value, Option<AbortSignal>) -> CanonicalExecuteFuture<'a> + Send + Sync + 'a;

/// `prepareArguments` callback(schema 検証前に input を正規化する hook)。
pub type CanonicalPrepare<'a> = dyn Fn(&Value) -> Value + Send + Sync + 'a;

/// `CanonicalToolDefinition`。`parameters` は TypeBox 生成物を literal
/// JSON 化した schema object(`{...definition.parameters}` 相当の clone を
/// compile 時に作る)。
pub struct CanonicalToolDefinition<'a> {
    pub name: ToolName,
    pub label: String,
    pub description: String,
    pub parameters: Map<String, Value>,
    /// `prepareArguments` 相当(Pi session 側で schema 検証前に input を
    /// 正規化する hook。edit の edits-as-JSON-string / 単一 edit object /
    /// legacy oldText/newText の吸収)。executor は検証**前**に適用する。
    /// `Arc` は `{...definition}` spread parity — registry が公開向け
    /// definition を複製する際に同じ hook を共有できるようにするため。
    pub prepare_arguments: Option<std::sync::Arc<CanonicalPrepare<'a>>>,
    pub execute: Box<CanonicalExecute<'a>>,
}

/// `defineCanonicalTool`: identity(TS と同じく構造を通すだけ)。
pub fn define_canonical_tool(
    definition: CanonicalToolDefinition<'_>,
) -> CanonicalToolDefinition<'_> {
    definition
}

/// `createCanonicalExecutor` の戻り値。validator は構築時に compile
/// 済み。`execute` は `prepareArguments` → `validateToolInput` →
/// `definition.execute` の順で 1 回だけ検証する。default は適用しない。
pub struct CanonicalExecutor<'a> {
    definition: CanonicalToolDefinition<'a>,
    schema: CompiledSchema,
}

impl CanonicalExecutor<'_> {
    pub async fn execute(
        &self,
        input: &Value,
        signal: Option<&AbortSignal>,
    ) -> Result<CanonicalToolResult, ToolError> {
        // `prepareArguments` は schema 検証の前段で非正規形を正規化する
        // (Pi wrapper の prepareArguments と同じ位置付け)。
        let prepared = match &self.definition.prepare_arguments {
            Some(prepare) => prepare(input),
            None => input.clone(),
        };
        validate_tool_input(self.definition.name.as_str(), &self.schema, &prepared)?;
        (self.definition.execute)(prepared, signal.cloned()).await
    }

    pub fn name(&self) -> ToolName {
        self.definition.name
    }

    pub fn label(&self) -> &str {
        &self.definition.label
    }

    pub fn description(&self) -> &str {
        &self.definition.description
    }

    /// `parameters` JSON schema(compile 済みと同一 object の clone)。
    pub fn parameters(&self) -> Value {
        Value::Object(self.definition.parameters.clone())
    }
}

/// `createCanonicalExecutor`: `parameters` を `{...spread}` して
/// `compileSchema(_, "input")`。
pub fn create_canonical_executor(
    definition: CanonicalToolDefinition<'_>,
) -> Result<CanonicalExecutor<'_>, ToolError> {
    let schema = compile_schema(definition.parameters.clone(), SchemaKind::Input)
        .map_err(|e| Box::new(e) as ToolError)?;
    Ok(CanonicalExecutor { definition, schema })
}

/// `validateToolInput`: invalid input は `CheaperError("INVALID_INPUT",
/// "invalid <tool> input", {tool, errors})`。
pub fn validate_tool_input(
    name: &str,
    schema: &CompiledSchema,
    input: &Value,
) -> Result<(), ToolError> {
    match schema.validate(input) {
        Ok(()) => Ok(()),
        Err(errors) => Err(Box::new(
            CheaperError::new(ErrorCode::InvalidInput, format!("invalid {name} input"))
                .with_details(Map::from_iter([
                    ("tool".to_string(), json!(name)),
                    ("errors".to_string(), json!(errors)),
                ])),
        )),
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    /// 同期 closure を `CanonicalExecute`(future 返却)へ包む helper。
    fn sync_execute(
        f: impl Fn(Value) -> Result<CanonicalToolResult, ToolError> + Send + Sync + 'static,
    ) -> Box<CanonicalExecute<'static>> {
        Box::new(move |input, _signal| {
            let result = f(input);
            Box::pin(async move { result })
        })
    }

    /// prepareArguments → validate → execute の順序と、schema 違反時に
    /// execute が呼ばれないことを確かめる。
    #[tokio::test]
    async fn executor_prepares_before_validation_and_blocks_execute() {
        let definition = CanonicalToolDefinition {
            name: ToolName::Write,
            label: "t".to_string(),
            description: "d".to_string(),
            parameters: json!({
                "type": "object",
                "required": ["path"],
                "properties": {"path": {"type": "string"}},
            })
            .as_object()
            .unwrap()
            .clone(),
            prepare_arguments: Some(std::sync::Arc::new(|_input: &Value| {
                // 非正規形 → schema 適合形。
                json!({"path": "prepared"})
            })),
            execute: sync_execute(|input| {
                assert_eq!(input["path"], "prepared");
                Ok(CanonicalToolResult::text("ok"))
            }),
        };
        let executor = create_canonical_executor(definition).unwrap();
        let result = executor.execute(&json!({}), None).await.unwrap();
        assert_eq!(result.content[0].text, "ok");
    }

    #[tokio::test]
    async fn executor_rejects_invalid_input_with_invalid_input_code() {
        let definition = CanonicalToolDefinition {
            name: ToolName::Write,
            label: "t".to_string(),
            description: "d".to_string(),
            parameters: json!({
                "type": "object",
                "required": ["path"],
                "properties": {"path": {"type": "string"}},
            })
            .as_object()
            .unwrap()
            .clone(),
            prepare_arguments: None,
            execute: sync_execute(|_| panic!("execute must not run on invalid input")),
        };
        let executor = create_canonical_executor(definition).unwrap();
        let err = executor.execute(&json!({}), None).await.unwrap_err();
        let cheaper = err.downcast_ref::<CheaperError>().expect("CheaperError");
        assert_eq!(cheaper.code(), ErrorCode::InvalidInput);
        assert_eq!(cheaper.message(), "invalid write input");
        let details = cheaper.details().expect("details");
        assert_eq!(details["tool"], "write");
        assert!(!details["errors"].as_array().unwrap().is_empty());
    }

    #[tokio::test]
    async fn executor_runs_prepare_even_when_result_still_invalid() {
        // prepare が通しても schema 違反なら INVALID_INPUT(二重検証なし)。
        let definition = CanonicalToolDefinition {
            name: ToolName::Read,
            label: "t".to_string(),
            description: "d".to_string(),
            parameters: json!({
                "type": "object",
                "required": ["path"],
                "properties": {"path": {"type": "string"}},
            })
            .as_object()
            .unwrap()
            .clone(),
            prepare_arguments: Some(std::sync::Arc::new(|_input: &Value| json!({}))),
            execute: sync_execute(|_| panic!("no execute")),
        };
        let executor = create_canonical_executor(definition).unwrap();
        assert!(executor.execute(&json!({"path": "x"}), None).await.is_err());
    }

    #[test]
    fn executor_metadata_accessors() {
        let definition = CanonicalToolDefinition {
            name: ToolName::Ls,
            label: "ls".to_string(),
            description: "list".to_string(),
            parameters: json!({"type": "object"}).as_object().unwrap().clone(),
            prepare_arguments: None,
            execute: sync_execute(|_| Ok(CanonicalToolResult::text(""))),
        };
        let executor = create_canonical_executor(definition).unwrap();
        assert_eq!(executor.name().as_str(), "ls");
        assert_eq!(executor.label(), "ls");
        assert_eq!(executor.description(), "list");
        assert_eq!(executor.parameters(), json!({"type": "object"}));
    }
}
