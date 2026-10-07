//! `cheaper-tools` の `ToolRuntime`(worker scope)を Rig `DynamicTool` へ
//! 接着する。tool 実行の所有権は cheaper-tools 側に残る — sandbox/access
//! policy/budget/scheduler/mutation journal/resource scope は全て
//! `ToolRuntime::execute` が担い、ここでは name/schema/callback の橋渡しと
//! per-run `AbortSignal` の注入だけを行う。
//!
//! signal 経路: `AgentRunner::tool_context` に scope として載せた
//! `AbortSignal` が各 dispatch の `ToolContext` へ `for_dispatch` で引き継が
//! れ、callback が `context.scope::<AbortSignal>()` で読み出して
//! `ToolCallContext.signal` へ渡す。scope 側 controller との複合は
//! `ToolRuntime::execute` が既に担う(TS `createSandboxedTools` の signal
//! 合成 parity)。
//!
//! `details`(model 非可視の host metadata)は `insert_result` で
//! `ToolResultContext` に publish し、hook が `ToolDetails` side-channel
//! 経由で `RunEvent::ToolResult` に添付する(TS `{...result, details}` の
//! model 非露出 parity)。

use serde::Serialize;

use cheaper_tools::operation::AbortSignal;
use cheaper_tools::registry::{ToolCallContext, ToolRuntime};

use rig::completion::message::ToolResultContent;
use rig::tool::{ContextValue, DynamicTool, ToolContext, ToolExecutionError, ToolOutput};

/// `CanonicalToolResult.details` の publish キー(ToolResultContext slot)。
#[derive(Debug, Clone, Serialize, serde::Deserialize)]
#[serde(transparent)]
pub(crate) struct ToolCallDetails(pub serde_json::Value);

impl ContextValue for ToolCallDetails {
    const KEY: &'static str = "cheaper.tool.details";
}

/// `CanonicalToolResult` → モデル可視の `ToolOutput`。`details` は
/// model に見せない(TS の `{...result, details}` parity)。
fn to_tool_output(result: cheaper_tools::catalog::CanonicalToolResult) -> ToolOutput {
    let content: Vec<ToolResultContent> = result
        .content
        .iter()
        .map(|item| ToolResultContent::text(item.text.clone()))
        .collect();
    // `ToolOutput::content` は empty を拒否する — content 0 件は空 text で
    // 表現する(ToolOutput の contract 上 `""` が空結果の表現)。
    if content.is_empty() {
        ToolOutput::text("")
    } else {
        ToolOutput::content(content).unwrap_or_else(|_| ToolOutput::text(""))
    }
}

/// `ToolError` → `ToolExecutionError`。`CheaperError` の code を machine
/// code として保持し、normalized kind は最も近い `ToolErrorKind` へ写す。
fn to_tool_error(error: cheaper_tools::ToolError) -> ToolExecutionError {
    use cheaper_core::errors::ErrorCode;
    use rig::tool::ToolErrorKind;
    match cheaper_tools::tools::process::find_cheaper_error(error.as_ref()) {
        Some(ce) => {
            let kind = match ce.code() {
                ErrorCode::PolicyViolation => ToolErrorKind::PermissionDenied,
                ErrorCode::Aborted => ToolErrorKind::Cancelled,
                ErrorCode::Timeout => ToolErrorKind::Timeout,
                ErrorCode::FileTooLarge => ToolErrorKind::InvalidArgs,
                _ => ToolErrorKind::Other,
            };
            ToolExecutionError::new(kind, ce.message()).with_code(ce.code().as_str())
        }
        None => ToolExecutionError::new(ToolErrorKind::Other, error.to_string()),
    }
}

/// worker scope の `definitions()` を全て Rig `DynamicTool` へ変換する。
/// `runtime` は呼び出しごとに `execute(name, input, context)` へ委譲する。
///
/// `ToolRuntime` の clone は inner Arc を共有するので、作成した tool は
/// scope の生存期間と同じだけ dispatch 可能(scope dispose で execute は
/// 失敗する — cheaper-tools の abort 経路)。
pub(crate) fn rig_tools(runtime: ToolRuntime) -> Vec<DynamicTool> {
    runtime
        .definitions()
        .iter()
        .map(|definition| {
            let runtime = runtime.clone();
            let name = definition.name.as_str().to_owned();
            DynamicTool::new_with_context(
                name.clone(),
                definition.description.clone(),
                serde_json::Value::Object(definition.parameters.clone()),
                move |context: &mut ToolContext, arguments| {
                    let runtime = runtime.clone();
                    let name = name.clone();
                    Box::pin(async move {
                        let signal = context
                            .scope::<AbortSignal>()
                            .map(|scoped| (*scoped).clone());
                        let mut result = runtime
                            .execute(&name, &arguments, Some(ToolCallContext { signal }))
                            .await
                            .map_err(to_tool_error)?;
                        if let Some(details) = result.details.take() {
                            // publish 失敗(serialize/decode 不一致)は tool の
                            // 成否を変えない — details は model 非可視 metadata。
                            let _ = context.insert_result(ToolCallDetails(details));
                        }
                        Ok(to_tool_output(result))
                    })
                },
            )
        })
        .collect()
}
