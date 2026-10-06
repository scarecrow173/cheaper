use std::sync::Arc;

use rig::agent::MultiTurnStreamItem;
use rig::completion::PromptError;
use rig::run::response::PromptResponse;
use rig::tool::DynamicTool;
use rig::{Agent, AgentBuilder, DynModel, operation::Completion};

use crate::hooks::AccumulatorHook;
use crate::usage::Accumulator;

/// Build an agent on a model handle with the accumulator hook installed and,
/// optionally, the PoC tool set registered.
pub fn build(
    model: impl Into<DynModel<Completion>>,
    acc: Arc<Accumulator>,
    tools: Vec<DynamicTool>,
) -> Agent {
    AgentBuilder::new(model)
        .dynamic_tools(tools)
        .add_hook(AccumulatorHook { acc })
        .build()
}

/// Blocking run: `PromptResponse` is consumed only for the output text;
/// accounting comes from the accumulator. `invalid_retries` sets the
/// `InvalidToolCallAction::Retry` budget — it defaults to zero and silently
/// degrades to `Fail` without it.
pub async fn run_blocking(
    agent: &Agent,
    prompt: &str,
    max_turns: usize,
    invalid_retries: usize,
) -> Result<PromptResponse, PromptError> {
    agent
        .prompt(prompt)
        .max_turns(max_turns)
        .max_invalid_tool_call_retries(invalid_retries)
        .await
}

/// Streaming run over `MultiTurnStreamItem`s; the accumulator still records
/// every turn and tool execution through hooks.
pub async fn run_streaming(
    agent: &Agent,
    prompt: &str,
    max_turns: usize,
    invalid_retries: usize,
) -> Result<Option<PromptResponse>, PromptError> {
    use futures::StreamExt;
    use rig::agent::StreamingError;
    let mut stream = agent
        .prompt(prompt)
        .max_turns(max_turns)
        .max_invalid_tool_call_retries(invalid_retries)
        .stream();
    let mut final_response = None;
    while let Some(item) = stream.next().await {
        let item = item.map_err(|e| match e {
            StreamingError::Prompt(e) => e,
            StreamingError::Completion(e) => PromptError::CompletionError(e),
            StreamingError::Report(r) => PromptError::Report(r),
        })?;
        if let MultiTurnStreamItem::FinalResponse(response) = item {
            final_response = Some(response);
        }
    }
    Ok(final_response)
}
