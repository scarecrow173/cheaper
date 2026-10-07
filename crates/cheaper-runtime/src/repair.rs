//! Structured output 修復ループ(packages/runtime-pi/src/repair.ts 相当)。
//!
//! 各 attempt は tool-less の新規 session で走り、直前の raw output と
//! schema errors を corrective prompt に載せる。usage/turns/retries は
//! main run と同じ accumulator 相当へ畳み込む(TS の shared `acc`)。
//!
//! TS 差異(adapter 構造由来):
//! - `session.messages`/`stopReason === "error"` の走査は adapter の
//!   `RunStream` 終端 error(code 分類済み `CheaperError`)に包含される。
//!   prompt rejection も `Rejected(CheaperError)` なので、TS が
//!   PROVIDER_FAILURE へ包み直していた分岐は adapter 側の分類をそのまま使う。
//! - `samplingParams.response_format` の json_schema hint は
//!   `SessionSpec.output_schema` として factory へ渡す。Rig 経路への
//!   配線は provider/adapter 側の責務(未配線なら corrective prompt と
//!   回数上限のみが contract を担保する)。

use cheaper_core::errors::{CheaperError, ErrorCode};
use cheaper_core::worker::schema::{CompiledSchema, SchemaErrorEntry};
use cheaper_rig_adapter::RunSnapshot;
use cheaper_tools::operation::AbortSignal;
use serde_json::{Map, Value};

use crate::auth::ResolvedAuth;
use crate::catalog::CatalogModel;
use crate::grace::{prompt_with_abort_grace, PromptOutcome};
use crate::meta::merge_snapshots;
use crate::output::{extract_output, ExtractedOutput};
use crate::run::{SessionFactory, SessionSpec};

/// error `details` に入れる上限(TS `MAX_DETAIL_ERRORS`/`MAX_DETAIL_RAW`)。
const MAX_DETAIL_ERRORS: usize = 20;
const MAX_DETAIL_RAW: usize = 2000;

/// `RepairOptions`。`on_attempt` は TS `onAttempt` — attempt 開始毎に
/// 呼ばれ、run 側が `meta.repairs`(消費 attempt 数)を記録する。
pub struct RepairOptions<'a> {
    pub session_factory: &'a dyn SessionFactory,
    /// 解決済み model(catalog entry)。
    pub model: &'a CatalogModel,
    pub auth: Option<&'a ResolvedAuth>,
    pub schema: &'a CompiledSchema,
    /// 初回の失敗 raw output + schema errors。
    pub raw_output: String,
    pub errors: Vec<SchemaErrorEntry>,
    /// `policy.output.repairAttempts`。
    pub attempts: u64,
    pub max_output_tokens: u64,
    /// `policy.retry.transientAttempts`。
    pub transient_retries: usize,
    /// `policy.execution.maxTurns` — session factory が内部 turn guard
    /// (`max(1024, policy maxTurns)`)を算出する材料(F7)。
    pub policy_max_turns: u64,
    /// run 集約 abort signal(`runAbort.signal`)。abort で残り attempt を打ち切る。
    pub signal: AbortSignal,
    /// `promptWithAbortGrace` の grace ms(既定 5000)。
    pub grace_ms: f64,
    pub on_attempt: &'a mut dyn FnMut(u64),
}

/// `RepairResult`。
pub struct RepairResult {
    pub result: Map<String, Value>,
    /// 成功した attempt 番号(=消費 repair 回数)。
    pub repairs: u64,
}

/// `repairOutput(...)`。
/// `usage` に各 attempt の session snapshot を逐次畳み込む(中断・失敗
/// 途中の分も残る — TS shared collector 相当)。
pub async fn repair_output(
    opts: &mut RepairOptions<'_>,
    usage: &mut RunSnapshot,
) -> Result<RepairResult, CheaperError> {
    let mut raw_output = opts.raw_output.clone();
    let mut errors = opts.errors.clone();

    for attempt in 1..=opts.attempts {
        if opts.signal.is_aborted() {
            break;
        }
        (opts.on_attempt)(attempt);
        let session = opts.session_factory.create_session(&SessionSpec {
            model: opts.model,
            auth: opts.auth,
            system_prompt: repair_system_prompt(opts.schema),
            tool_runtime: None,
            output_schema: Some(opts.schema),
            max_output_tokens: opts.max_output_tokens,
            transient_retries: opts.transient_retries,
            policy_max_turns: opts.policy_max_turns,
        })?;
        let mut sink = |_event: &cheaper_rig_adapter::RunEvent| {};
        let outcome = prompt_with_abort_grace(
            &session,
            repair_user_prompt(&raw_output, &errors),
            &opts.signal,
            opts.grace_ms,
            &mut sink,
        )
        .await;
        merge_snapshots(usage, &session.snapshot());
        // 終端 error はそのまま投げる(TS: CheaperError → rethrow)。
        if let PromptOutcome::Rejected(err) = outcome {
            return Err(err);
        }
        // abort 済みなら extraction せず抜ける(分類は呼び出し側の abort
        // reason の責務 — Grace も abort 後のみ起きるので同じ扱い)。
        if opts.signal.is_aborted() {
            break;
        }
        let PromptOutcome::Settled { output } = outcome else {
            break;
        };
        match extract_output(output.as_deref(), opts.schema) {
            ExtractedOutput::Ok { result, .. } => {
                return Ok(RepairResult {
                    result,
                    repairs: attempt,
                });
            }
            ExtractedOutput::Err {
                raw_output: new_raw,
                errors: new_errors,
                ..
            } => {
                raw_output = new_raw;
                errors = new_errors;
            }
        }
    }

    Err(invalid_output_error(&errors, &raw_output))
}

/// `throw new CheaperError("INVALID_OUTPUT", ...)` — details は
/// `errors` 先頭 20 件・`rawOutput` 先頭 2000 文字。
fn invalid_output_error(errors: &[SchemaErrorEntry], raw_output: &str) -> CheaperError {
    let capped_errors: Vec<Value> = errors
        .iter()
        .take(MAX_DETAIL_ERRORS)
        .map(|e| serde_json::to_value(e).unwrap_or(Value::Null))
        .collect();
    CheaperError::new(
        ErrorCode::InvalidOutput,
        "worker output failed schema validation",
    )
    .with_details(Map::from_iter([
        ("errors".to_string(), Value::Array(capped_errors)),
        (
            "rawOutput".to_string(),
            Value::from(truncate_chars(raw_output, MAX_DETAIL_RAW)),
        ),
    ]))
}

/// 先頭 `n` 文字までに切り詰め。TS `slice(0, 2000)` は UTF-16 code unit
/// 基準だが、ここでは char(Unicode scalar)基準 — non-BMP 文字を含む
/// raw output でのみ切断位置がずれる(contract は「先頭 2000 文字」)。
fn truncate_chars(s: &str, n: usize) -> String {
    if s.chars().count() <= n {
        return s.to_string();
    }
    s.chars().take(n).collect()
}

/// `repairSystemPrompt` — TS と同一 text。schema embed は
/// `JSON.stringify(…, null, 2)` 規則(F10 — number 表現が Rust と
/// 異なるため serde_json の pretty printer は使わない)。
fn repair_system_prompt(schema: &CompiledSchema) -> String {
    let schema_json =
        cheaper_rig_adapter::js_json::stringify_pretty(&Value::Object(schema.schema.clone()));
    format!(
        "You are a JSON repair worker. Return ONLY a JSON object that conforms to the\n\
         schema below. No prose, no markdown, no code fences.\n\
         \n\
         Schema:\n\
         {schema_json}"
    )
}

/// `repairUserPrompt` — TS と同一 text。`instancePath || "/"` の
/// fallback を含む。
fn repair_user_prompt(raw_output: &str, errors: &[SchemaErrorEntry]) -> String {
    let lines: Vec<String> = errors
        .iter()
        .map(|e| {
            format!(
                "- {}: {}",
                if e.instance_path.is_empty() {
                    "/"
                } else {
                    e.instance_path.as_str()
                },
                e.message
            )
        })
        .collect();
    let error_lines = if lines.is_empty() {
        "- (no details)".to_string()
    } else {
        lines.join("\n")
    };
    format!(
        "The previous output failed schema validation.\n\
         \n\
         Raw output:\n\
         {raw_output}\n\
         \n\
         Validation errors:\n\
         {error_lines}\n\
         \n\
         Return the corrected JSON object only."
    )
}
