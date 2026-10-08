use rig::test_utils::{MockCompletionModel, MockTurn};

/// Scripted provider models for deterministic probes. `MockScript` is the
/// wire; `MockRuntime` (built into `MockCompletionModel`) is the transport —
/// the same seam a custom provider uses.
pub fn provider_error_model() -> MockCompletionModel {
    MockCompletionModel::from_turns([MockTurn::provider_response_error(
        http::StatusCode::INTERNAL_SERVER_ERROR,
        "scripted provider failure",
        "req-poc-1",
    )])
}

/// First turn calls an unregistered tool; the run must roll back with
/// corrective feedback (`InvalidToolCallAction::Retry`) and the second turn
/// answers cleanly. Turn 1 carries nonzero usage so the probe proves billed
/// usage survives the rollback (`ModelTurnFinished` never fires for it).
pub fn unknown_tool_call_model() -> MockCompletionModel {
    MockCompletionModel::from_turns([
        MockTurn::tool_call("call-1", "does_not_exist", serde_json::json!({})).with_usage(
            rig::completion::Usage {
                input_tokens: Some(100),
                output_tokens: Some(10),
                total_tokens: Some(110),
                ..Default::default()
            },
        ),
        MockTurn::text("recovered without tools").with_usage(rig::completion::Usage {
            input_tokens: Some(150),
            output_tokens: Some(5),
            total_tokens: Some(155),
            ..Default::default()
        }),
    ])
}

/// A model that fails with the given status, for error classification.
pub fn status_error_model(status: http::StatusCode) -> MockCompletionModel {
    MockCompletionModel::from_turns([MockTurn::provider_response_error(
        status,
        format!("scripted {status}"),
        "req-probe",
    )])
}

/// A request-construction failure (`ErrorKind::Request`): permanent by
/// cheaper's classification. A real transport timeout is `ErrorKind::Http`
/// and is exercised separately as a synthesized error in `error-probe`.
pub fn request_error_model() -> MockCompletionModel {
    MockCompletionModel::from_turns([MockTurn::request_error("malformed request")])
}
