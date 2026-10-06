mod agent;
mod errors;
mod fake_provider;
mod hooks;
mod output;
mod providers;
mod repair;
mod tools;
mod usage;

use std::process::exit;
use std::sync::Arc;

use serde_json::json;
use usage::Accumulator;

fn arg_value(name: &str) -> Option<String> {
    let args: Vec<String> = std::env::args().collect();
    args.windows(2)
        .find(|w| w[0] == name)
        .map(|w| w[1].clone())
}

#[tokio::main]
async fn main() {
    let command = std::env::args().nth(1).unwrap_or_default();
    let code = match command.as_str() {
        "run" => cmd_run().await,
        "run-scripted" => cmd_run_scripted().await,
        "repair-run" => cmd_repair().await,
        "error-probe" => cmd_error_probe().await,
        "abort-probe" => cmd_abort_probe().await,
        "panic-probe" => cmd_panic_probe().await,
        _ => {
            eprintln!(
                "usage: poc-rig <run|run-scripted --scenario S|repair-run|error-probe|abort-probe|panic-probe>"
            );
            2
        }
    };
    exit(code);
}

/// T1+T2: real Ollama run with tools; hook accumulator is the only usage
/// source and is emitted on success and failure alike.
async fn cmd_run() -> i32 {
    let model_name = arg_value("--model").unwrap_or_else(|| "qwen3.5:9b".into());
    let prompt = arg_value("--prompt").unwrap_or_else(|| {
        "Use the `grep` tool to find lines containing 'fn' in crates/poc-rig/src/main.rs, \
         then answer with one sentence."
            .into()
    });
    let client = match rig::providers::ollama::Ollama::from_env() {
        Ok(c) => c,
        Err(e) => {
            output::emit(&json!({"status":"error","error":{"code":"CONFIG","message":e.to_string()}}));
            return 70;
        }
    };
    let acc = Arc::new(Accumulator::default());
    let agent = agent::build(
        client.completion(model_name).erase(),
        acc.clone(),
        vec![tools::read_tool(), tools::grep_tool(), tools::write_tool()],
    );
    match agent::run_streaming(&agent, &prompt, 6, 2).await {
        Ok(Some(response)) => {
            output::emit(&json!({
                "status": "ok",
                "result": response.output,
                "meta": acc.snapshot(),
            }));
            0
        }
        Ok(None) => {
            output::emit(&json!({"status":"error","error":{"code":"EMPTY"},"meta":acc.snapshot()}));
            70
        }
        Err(e) => {
            let class = errors::classify_prompt(&e);
            output::emit(&json!({
                "status": "error",
                "error": {"code": class, "message": e.to_string()},
                "meta": acc.snapshot(),
            }));
            class.exit_code()
        }
    }
}

/// T2: scripted scenarios — deterministic, no provider needed.
async fn cmd_run_scripted() -> i32 {
    let scenario = arg_value("--scenario").unwrap_or_default();
    let acc = Arc::new(Accumulator::default());
    match scenario.as_str() {
        "provider-error" => {
            let agent = agent::build(
                fake_provider::provider_error_model().erase(),
                acc.clone(),
                vec![tools::read_tool()],
            );
            let result = agent::run_blocking(&agent, "say hi", 4, 2).await;
            let (code, message, exit) = match &result {
                Ok(_) => (json!("UNEXPECTED_OK"), "run succeeded".to_string(), 70),
                Err(e) => {
                    let class = errors::classify_prompt(e);
                    (json!(class), e.to_string(), class.exit_code())
                }
            };
            output::emit(&json!({
                "status": "error",
                "error": {"code": code, "message": message},
                "meta": acc.snapshot(),
            }));
            exit
        }
        "unknown-tool-call" => {
            let agent = agent::build(
                fake_provider::unknown_tool_call_model().erase(),
                acc.clone(),
                vec![tools::read_tool(), tools::grep_tool()],
            );
            match agent::run_blocking(&agent, "use a tool", 4, 2).await {
                Ok(response) => {
                    let snap = acc.snapshot();
                    // Recovery is proven only if the invalid call was seen, a
                    // second turn ran, and the answer came back — and the
                    // rolled-back attempt's billed usage (100/10/110) is in
                    // meta even though its turn was never accepted.
                    let recovered = snap.invalid_tool_calls == 1
                        && snap.tool_calls == 0
                        && snap.rejected_tool_calls == 0
                        && snap.turns == 1
                        && snap.usage.input_tokens == 250
                        && !response.output.is_empty();
                    output::emit(&json!({
                        "status": "ok",
                        "result": response.output,
                        "meta": snap,
                        "invalidCallCounted": recovered,
                    }));
                    if recovered { 0 } else { 70 }
                }
                Err(e) => {
                    output::emit(&json!({
                        "status": "error",
                        "error": {"message": e.to_string()},
                        "meta": acc.snapshot(),
                    }));
                    70
                }
            }
        }
        _ => {
            output::emit(&json!({"status":"error","error":{"code":"USAGE","message":"unknown scenario"}}));
            2
        }
    }
}

/// T3: schema-violating reply → repair retry → valid JSON; repairs and
/// repair usage merge into the same accumulator.
async fn cmd_repair() -> i32 {
    let model = rig::test_utils::MockCompletionModel::from_turns([
        rig::test_utils::MockTurn::text("{\"not\":\"valid\"}"),
        rig::test_utils::MockTurn::text("{\"answer\":\"fixed\"}"),
    ]);
    let report = repair::run_repair(model.erase()).await;
    output::emit(&json!({"status": if report.schema_valid { "ok" } else { "error" }, "repair": report}));
    if report.schema_valid { 0 } else { 70 }
}

/// T5(error): 429 / 5xx / transport failure classification.
async fn cmd_error_probe() -> i32 {
    let cases = [
        ("429", fake_provider::status_error_model(http::StatusCode::TOO_MANY_REQUESTS)),
        ("500", fake_provider::status_error_model(http::StatusCode::INTERNAL_SERVER_ERROR)),
        ("request-error", fake_provider::request_error_model()),
    ];
    let mut results = Vec::new();
    for (name, model) in cases {
        let acc = Arc::new(Accumulator::default());
        let agent = agent::build(model.erase(), acc.clone(), vec![]);
        match agent::run_blocking(&agent, "hi", 2, 0).await {
            Err(e) => {
                let class = errors::classify_prompt(&e);
                results.push(json!({
                    "case": name,
                    "class": class,
                    "transient": class.is_transient(),
                    "meta": acc.snapshot(),
                }));
            }
            other => results.push(json!({"case": name, "unexpected": format!("{other:?}")})),
        }
    }
    // Transport failure with no reply (the case a real timeout lands in):
    // `ErrorKind::Http`, no status. Synthesized directly since the scripted
    // model cannot produce one.
    let transport = rig::error::ProviderError::Http(std::sync::Arc::new(
        rig::http_client::Error::StreamEnded,
    ));
    let class = errors::classify(&transport);
    results.push(json!({"case": "transport", "class": class, "transient": class.is_transient()}));

    output::emit(&json!({"status":"ok","cases":results}));
    0
}

/// T5(abort): streaming run cancelled mid-flight. Turn 1 settles (usage and
/// a tool execution recorded), then the stream is dropped during turn 2 —
/// `partialMeta` must carry the settled turn's accounting, proving partial
/// snapshots survive cancellation.
async fn cmd_abort_probe() -> i32 {
    use futures::StreamExt;
    use rig::completion::{FinishReason, Usage};
    use rig::operation::Finish;
    use rig::test_utils::{MockCompletionModel, MockStreamEvent};

    let usage = |input: u64| Usage {
        input_tokens: Some(input),
        output_tokens: Some(2),
        total_tokens: Some(input + 2),
        ..Default::default()
    };
    let model = MockCompletionModel::from_stream_turns([
        vec![
            MockStreamEvent::ToolCall {
                id: "call-1".into(),
                name: "read".into(),
                arguments: serde_json::json!({"path": "Cargo.toml"}),
                call_id: None,
            },
            MockStreamEvent::FinalResponse(Finish {
                usage: usage(10),
                reason: Some(FinishReason::ToolCalls),
                ..Default::default()
            }),
        ],
        vec![
            MockStreamEvent::text("chunk-1 "),
            MockStreamEvent::text("chunk-2 "),
            MockStreamEvent::FinalResponse(Finish {
                usage: usage(20),
                reason: Some(FinishReason::Stop),
                ..Default::default()
            }),
        ],
    ]);
    let acc = Arc::new(Accumulator::default());
    let agent = agent::build(model.erase(), acc.clone(), vec![tools::read_tool()]);
    let mut stream = agent.prompt("hi").max_turns(4).stream();

    // Consume until turn 1 is settled in the accumulator, then cancel.
    let mut items_seen = 0u64;
    while let Some(item) = stream.next().await {
        items_seen += 1;
        if item.is_err() || acc.snapshot().turns >= 1 {
            break;
        }
    }
    drop(stream);

    let partial = acc.snapshot();
    output::emit(&json!({
        "status": "ok",
        "cancelled": true,
        "reason": "client-abort",
        "itemsSeen": items_seen,
        "partialMeta": partial,
        "partialAccountingProven": partial.turns >= 1 && partial.usage.input_tokens > 0,
    }));
    0
}

/// T5(panic): a panicking task maps to INTERNAL_ERROR on stdout, exit 70.
async fn cmd_panic_probe() -> i32 {
    // Quiet the panic hook so the panic text never reaches stdout.
    std::panic::set_hook(Box::new(|_| {}));

    let joined = tokio::spawn(async {
        panic!("scripted panic");
    })
    .await;

    let catch = std::panic::catch_unwind(|| {
        panic!("scripted sync panic");
    });

    output::emit(&json!({
        "status": "error",
        "error": {"code": "INTERNAL_ERROR"},
        "joinErrorIsPanic": joined.as_ref().err().map(|e| e.is_panic()),
        "catchUnwindIsErr": catch.is_err(),
    }));
    70
}
