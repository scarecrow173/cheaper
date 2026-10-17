//! budget 枯渇 checkpoint(issue #111)— `MaxTurns`/`MaxToolCalls` の
//! policy abort 時に、それまでに観測した tool 実行結果を
//! `<userConfigDir>/checkpoints/<id>.json` へ保存し、`cheaper run
//! --resume <path>` で再開できるようにする。
//!
//! 観測は main session の `on_event` funnel で `ToolCommitted`
//! (hook patch 後の引数)と `ToolResult` を `call_id` で join して行う。
//! `ModelTurnRetried` で rollback された未完 call と admission 拒否の
//! error result(ToolCommitted を持たない ToolResult)は含めない。
//! timeout・signal abort・workflow job は対象外。
//!
//! 保存は best-effort — 失敗しても abort 分類・exit code は変わらず、
//! envelope 側の `details.checkpoint` が省略されるだけ。

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use cheaper_core::fs_util::{atomic_write, AtomicWriteOptions};
use cheaper_rig_adapter::{RunEvent, RunSnapshot};
use serde_json::{json, Map, Value};

/// `schemaVersion` — resume 時に読める形式の version。
pub const CHECKPOINT_SCHEMA_VERSION: u64 = 1;
/// 各 entry の `resultExcerpt` 上限(文字数)。
const ENTRY_EXCERPT_MAX_CHARS: usize = 2_000;
/// observations 配列の合計上限(概算 byte)。
const OBSERVATIONS_MAX_BYTES: usize = 100 * 1024;

/// 1 観測 — commit 済み tool call の引数 + 結果抜粋。
#[derive(Debug, Clone)]
struct Observation {
    call_id: String,
    tool: String,
    args: Value,
    result_excerpt: String,
    /// `ToolResult` 受領済みか(result 未到着の pending commit は
    /// `ModelTurnRetried` の rollback で取り除く対象)。
    has_result: bool,
}

/// main session の `ToolCommitted`/`ToolResult` event から構築する
/// 観測 buffer。`call_id` で join し、entry あたり・合計の両方に
/// cap を掛ける。
#[derive(Debug, Default)]
pub struct RunObservations {
    /// resume 由来の前回観測(serialized entry `{tool,args,
    /// resultExcerpt}`)。連鎖 resume で新規観測と累積して保存する。
    prior: Vec<Value>,
    entries: Vec<Observation>,
    /// `call_id` → entries index。result 未到着の pending を探す用。
    by_call: HashMap<String, usize>,
    /// 概算の累積 byte 数(`OBSERVATIONS_MAX_BYTES` まで)。
    /// prior は entry 全体の serialized size、新規は excerpt 長で計る
    /// (いずれも概算 — prior 側がやや保守的)。
    total_bytes: usize,
    /// cap で打ち切ったことがあるか。
    truncated: bool,
}

impl RunObservations {
    pub fn new() -> Self {
        Self::default()
    }

    /// 前回 checkpoint の観測を seed して連鎖 resume を可能にする
    /// (design §3d)。合計 byte cap を越える分は古い観測から落とす。
    pub fn with_prior(prior: Vec<Value>) -> Self {
        let mut sized: std::collections::VecDeque<(usize, Value)> =
            prior.into_iter().map(|o| (entry_size(&o), o)).collect();
        let mut total: usize = sized.iter().map(|(n, _)| n).sum();
        let mut truncated = false;
        while total > OBSERVATIONS_MAX_BYTES {
            let Some((n, _)) = sized.pop_front() else {
                break;
            };
            total -= n;
            truncated = true;
        }
        Self {
            prior: sized.into_iter().map(|(_, o)| o).collect(),
            total_bytes: total,
            truncated,
            ..Self::default()
        }
    }

    /// `on_event` funnel から呼ぶ観測記録。
    pub fn note(&mut self, event: &RunEvent) {
        match event {
            RunEvent::ToolCommitted {
                call_id,
                name,
                arguments,
            } => {
                self.by_call.insert(call_id.clone(), self.entries.len());
                self.entries.push(Observation {
                    call_id: call_id.clone(),
                    tool: name.clone(),
                    args: arguments.clone(),
                    result_excerpt: String::new(),
                    has_result: false,
                });
            }
            RunEvent::ToolResult {
                call_id, output, ..
            } => {
                // admission 拒否など commit されていない call の result
                // は観測に含めない。
                let Some(&index) = self.by_call.get(call_id) else {
                    return;
                };
                let excerpt = excerpt_chars(&output.to_string(), ENTRY_EXCERPT_MAX_CHARS);
                let added = excerpt.len();
                if self.total_bytes + added > OBSERVATIONS_MAX_BYTES {
                    self.truncated = true;
                    return;
                }
                self.total_bytes += added;
                let entry = &mut self.entries[index];
                entry.result_excerpt = excerpt;
                entry.has_result = true;
            }
            // rollback された turn(invalid-tool retry)の commit 済み
            // pending entry — result が来ない call は観測から外す。
            RunEvent::ModelTurnRetried { .. } => {
                self.entries.retain(|o| o.has_result);
                // retain で index がずれるため map を張り直す。
                self.by_call = self
                    .entries
                    .iter()
                    .enumerate()
                    .map(|(i, o)| (o.call_id.clone(), i))
                    .collect();
            }
            _ => {}
        }
    }

    /// 記録済み観測数(`details.checkpoint.observationsCount`)—
    /// resume 由来の prior を含む累積数。
    pub fn len(&self) -> usize {
        self.prior.len() + self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.prior.is_empty() && self.entries.is_empty()
    }

    fn to_json(&self) -> Vec<Value> {
        self.prior
            .iter()
            .cloned()
            .chain(self.entries.iter().map(|o| {
                json!({
                    "tool": o.tool,
                    "args": o.args,
                    "resultExcerpt": o.result_excerpt,
                })
            }))
            .collect()
    }
}

/// serialized observation entry の概算 byte 数(prior cap 判定用)。
fn entry_size(o: &Value) -> usize {
    serde_json::to_string(o).map(|s| s.len()).unwrap_or(0)
}

/// 保存済み checkpoint の envelope 表面情報。
#[derive(Debug)]
pub struct CheckpointInfo {
    pub path: PathBuf,
    pub observations_count: usize,
}

/// budget abort 時の checkpoint 保存。失敗は `Err` で返し、呼び出し側が
/// `details.checkpoint` を省略するだけ(best-effort)。
pub fn save_checkpoint(
    user_config_dir: &Path,
    worker: &str,
    input: &Value,
    abort_reason: &str,
    snapshot: &RunSnapshot,
    observations: &RunObservations,
) -> std::io::Result<CheckpointInfo> {
    let dir = user_config_dir.join("checkpoints");
    std::fs::create_dir_all(&dir)?;
    let id = format!(
        "run-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    );
    let path = dir.join(format!("{id}.json"));
    let body = json!({
        "schemaVersion": CHECKPOINT_SCHEMA_VERSION,
        "worker": worker,
        "input": input,
        "abortReason": abort_reason,
        "turns": snapshot.turns,
        "toolCalls": snapshot.tool_calls,
        "rejectedToolCalls": snapshot.rejected_tool_calls,
        "observationsTruncated": observations.truncated,
        "observations": observations.to_json(),
    });
    atomic_write(
        &path,
        &body.to_string(),
        &AtomicWriteOptions {
            windows_remove_first: false,
            #[cfg(unix)]
            unix_mode: Some(0o600),
        },
    )?;
    Ok(CheckpointInfo {
        path,
        observations_count: observations.len(),
    })
}

/// `cheaper run --resume` で注入される前回 run の継続状態。
/// session 内部は serialize しない — 観測と counter のみを運ぶ
/// (design §3c)。
#[derive(Debug, Clone)]
pub struct ResumeCheckpoint {
    pub abort_reason: String,
    pub turns: u64,
    pub tool_calls: u64,
    /// serialized observation entry 列(`{tool,args,resultExcerpt}`)。
    pub observations: Vec<Value>,
}

impl ResumeCheckpoint {
    /// `load_checkpoint` の検証済み object → resume 入力。entry の
    /// field 型は schemaVersion=1 の save 側が保証するため、ここでは
    /// 抜き出すだけ(厳格な per-entry 検証は行わない)。
    pub fn from_checkpoint(obj: &Map<String, Value>) -> Self {
        Self {
            abort_reason: obj
                .get("abortReason")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string(),
            turns: obj.get("turns").and_then(Value::as_u64).unwrap_or(0),
            tool_calls: obj.get("toolCalls").and_then(Value::as_u64).unwrap_or(0),
            observations: obj
                .get("observations")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default(),
        }
    }
}

/// checkpoint file の load + 基本検証(`--resume` 用)。
/// 不正な file・version 不一致は `INVALID_INPUT`。
pub fn load_checkpoint(
    path: &Path,
) -> Result<Map<String, Value>, cheaper_core::errors::CheaperError> {
    use cheaper_core::errors::{CheaperError, ErrorCode};
    let invalid = |msg: &str| {
        CheaperError::new(
            ErrorCode::InvalidInput,
            format!("invalid checkpoint: {msg}"),
        )
    };
    let body = std::fs::read_to_string(path)
        .map_err(|e| invalid(&format!("cannot read {}: {e}", path.display())))?;
    let value: Value =
        serde_json::from_str(&body).map_err(|e| invalid(&format!("malformed JSON: {e}")))?;
    let obj = value
        .as_object()
        .ok_or_else(|| invalid("top-level value is not an object"))?;
    if obj.get("schemaVersion").and_then(Value::as_u64) != Some(CHECKPOINT_SCHEMA_VERSION) {
        return Err(invalid("schemaVersion is not 1"));
    }
    for key in ["worker", "abortReason"] {
        if !obj.get(key).is_some_and(Value::is_string) {
            return Err(invalid(&format!("`{key}` is not a string")));
        }
    }
    if !obj.get("input").is_some_and(Value::is_object) {
        return Err(invalid("`input` is not an object"));
    }
    if !obj.get("observations").is_some_and(Value::is_array) {
        return Err(invalid("`observations` is not an array"));
    }
    Ok(obj.clone())
}

/// 文字数 cap(`result_excerpt` 用)— `String::len` ではなく chars 数で
/// 切る(UTF-8 境界で panic しないため)。
fn excerpt_chars(text: &str, max: usize) -> String {
    if text.chars().count() <= max {
        return text.to_string();
    }
    let mut out: String = text.chars().take(max).collect();
    out.push_str("…[truncated]");
    out
}
