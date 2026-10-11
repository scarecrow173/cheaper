//! `GetChatMessage` Connect-RPC stream transport（TS
//! `providers/devin/stream.ts` の port — vendored from pi-devin,
//! Copyright (c) 2026 Kashyab Ambarani, MIT License。`reader.closed`/
//! `body.cancel()` の unhandled rejection 対策は mizorewww/pi-devin の
//! 修正に倣い、Rust では response stream drop が同じ役目を安全に担う）。
//!
//! - request: `encodeMessage(1, metadata)` + prompt fields + config +
//!   tools + ids（byte 配置は TS `buildGetChatMessageRequest` 同一）
//! - transport frame: Connect `flags|len|payload`（`0x01` gzip、
//!   `0x02` EOS trailer）
//! - response proto → `ChatEvent` 列（text/reasoning/tool_call_*/
//!   finish/usage）

use std::collections::HashMap;
use std::sync::Mutex;

use bytes::Bytes;
use futures::Stream;

use crate::devin::context_map::{ChatContent, ChatHistoryItem, ChatRole, ContentPart, ToolDef};
use crate::devin::metadata::{build_metadata, MetadataInput};
use crate::devin::wire::{
    encode_fixed64_field, encode_message, encode_string, encode_varint_field, gunzip, iter_fields,
    DecodeError,
};

/// Devin chat endpoint（`POST {host}/exa.api_server_pb.ApiServerService/GetChatMessage`）。
pub const CHAT_ENDPOINT: &str = "/exa.api_server_pb.ApiServerService/GetChatMessage";

/// TS `SOURCE_BY_ROLE`。
fn source_of(role: ChatRole) -> u64 {
    match role {
        ChatRole::User | ChatRole::System => 1,
        ChatRole::Assistant => 2,
        ChatRole::Tool => 4,
    }
}

/// TS `CloudChatEvent`。
#[derive(Debug, Clone, PartialEq)]
pub enum ChatEvent {
    Text(String),
    Reasoning(String),
    ToolCallStart {
        id: String,
        name: String,
    },
    /// `id` は frame が続きの call を名指すときのみ（TS `id?: string`）。
    ToolCallArgs {
        delta: String,
        id: Option<String>,
    },
    Finish(ChatFinishReason),
    Usage(ChatUsage),
}

/// TS `finish.reason`（wire value 10/11/1|3 → tool_calls/content_filter/
/// length、その他 → stop）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChatFinishReason {
    Stop,
    ToolCalls,
    Length,
    ContentFilter,
}

/// TS `usage` event。
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ChatUsage {
    pub prompt_tokens: Option<u64>,
    pub completion_tokens: Option<u64>,
    pub total_tokens: Option<u64>,
    pub cached_input_tokens: Option<u64>,
    pub cache_creation_input_tokens: Option<u64>,
}

fn normalize_content(content: &ChatContent) -> Vec<ContentPart> {
    match content {
        ChatContent::Text(text) => vec![ContentPart::Text(text.clone())],
        ChatContent::Parts(parts) => parts.clone(),
    }
}

fn text_of(parts: &[ContentPart]) -> String {
    parts
        .iter()
        .filter_map(|part| match part {
            ContentPart::Text(text) if !text.is_empty() => Some(text.clone()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// `collapseSystemIntoUser` — system message は次の user message の
/// `<system>…</system>` prefix に畳み込む（末尾に残れば単独 user 化）。
pub fn collapse_system_into_user(messages: &[ChatHistoryItem]) -> Vec<ChatHistoryItem> {
    let mut out: Vec<ChatHistoryItem> = Vec::new();
    let mut pending: Vec<String> = Vec::new();

    for message in messages {
        if message.role == ChatRole::System {
            let text = text_of(&normalize_content(&message.content));
            if !text.is_empty() {
                pending.push(text);
            }
            continue;
        }
        if message.role == ChatRole::User && !pending.is_empty() {
            let user_parts = normalize_content(&message.content);
            let user_text = text_of(&user_parts);
            let mut content = vec![ContentPart::Text(format!(
                "<system>\n{}\n</system>\n{}",
                pending.join("\n\n"),
                user_text
            ))];
            content.extend(
                user_parts
                    .into_iter()
                    .filter(|part| matches!(part, ContentPart::Image { .. })),
            );
            out.push(ChatHistoryItem {
                role: ChatRole::User,
                content: ChatContent::Parts(content),
                tool_call_id: None,
                tool_calls: None,
            });
            pending.clear();
            continue;
        }
        out.push(message.clone());
    }
    if !pending.is_empty() {
        out.push(ChatHistoryItem {
            role: ChatRole::User,
            content: ChatContent::Text(format!("<system>\n{}\n</system>", pending.join("\n\n"))),
            tool_call_id: None,
            tool_calls: None,
        });
    }
    out
}

/// `encodeImageData` — `{1: base64, 2: mime}`。
fn encode_image_data(mime_type: Option<&str>, base64_data: Option<&str>) -> Vec<u8> {
    let mut out = encode_string(1, base64_data.unwrap_or(""));
    out.extend_from_slice(&encode_string(2, mime_type.unwrap_or("image/png")));
    out
}

/// `encodeChatToolCall` — `{1: id, 2: name, 3: arguments}`。
fn encode_chat_tool_call(id: &str, name: &str, arguments: &str) -> Vec<u8> {
    let mut out = encode_string(1, id);
    out.extend_from_slice(&encode_string(2, name));
    out.extend_from_slice(&encode_string(3, arguments));
    out
}

/// `encodeChatMessagePrompt` — `{2: source, 3: text, 4: tokens, 5: 1,
/// 7?: toolCallId, 6*: toolCalls, 10*: images}`。
fn encode_chat_message_prompt(
    content: &[ContentPart],
    source: u64,
    tool_call_id: Option<&str>,
    tool_calls: Option<&[crate::devin::context_map::ChatToolCall]>,
) -> Vec<u8> {
    let text = content
        .iter()
        .filter_map(|part| match part {
            ContentPart::Text(text) => Some(text.as_str()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("\n");
    let mut out = Vec::new();
    out.extend_from_slice(&encode_varint_field(2, source));
    out.extend_from_slice(&encode_string(3, &text));
    // TS `Math.max(1, Math.floor(text.length / 4))` — JS length は UTF-16
    // 単位、Rust chars() も Unicode scalar 単位なので Astral 面で差が
    // 出得る（概算値のため実害なし — divergence として記録）。
    out.extend_from_slice(&encode_varint_field(
        4,
        (text.chars().count() as u64 / 4).max(1),
    ));
    out.extend_from_slice(&encode_varint_field(5, 1));
    if let Some(id) = tool_call_id {
        out.extend_from_slice(&encode_string(7, id));
    }
    for call in tool_calls.into_iter().flatten() {
        out.extend_from_slice(&encode_message(
            6,
            &encode_chat_tool_call(&call.id, &call.name, &call.arguments),
        ));
    }
    for part in content {
        if let ContentPart::Image {
            mime_type,
            base64_data,
        } = part
        {
            out.extend_from_slice(&encode_message(
                10,
                &encode_image_data(mime_type.as_deref(), base64_data.as_deref()),
            ));
        }
    }
    out
}

/// `encodeCompletionConfiguration` — `{1:1, 2:64000, 3:maxOut, 5:0.7,
/// 6:0.95, 7:50, 8:1.0, 11:1.0}`。
fn encode_completion_configuration(max_output_tokens: Option<u64>) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(&encode_varint_field(1, 1));
    out.extend_from_slice(&encode_varint_field(2, 64_000));
    out.extend_from_slice(&encode_varint_field(
        3,
        max_output_tokens.unwrap_or(128_000),
    ));
    out.extend_from_slice(&encode_fixed64_field(5, 0.7));
    out.extend_from_slice(&encode_fixed64_field(6, 0.95));
    out.extend_from_slice(&encode_varint_field(7, 50));
    out.extend_from_slice(&encode_fixed64_field(8, 1.0));
    out.extend_from_slice(&encode_fixed64_field(11, 1.0));
    out
}

/// `encodeToolDef` — `{1: name, 2: description(<=6998), 3: parameters JSON}`。
fn encode_tool_def(tool: &ToolDef) -> Vec<u8> {
    let description = if tool.description.chars().count() > 6_998 {
        // TS `slice(0, 6995) + "..."` — char 単位で近似。
        format!(
            "{}...",
            tool.description.chars().take(6_995).collect::<String>()
        )
    } else {
        tool.description.clone()
    };
    let mut out = encode_string(1, &tool.name);
    out.extend_from_slice(&encode_string(2, &description));
    out.extend_from_slice(&encode_string(
        3,
        &serde_json::to_string(&tool.parameters).unwrap_or_else(|_| "{}".to_string()),
    ));
    out
}

/// `buildGetChatMessageRequest` の metadata 部分（JWT field 21 を除く —
/// transport が mint 後に append する）。session id は呼び出し側が解決。
pub struct RequestParts {
    /// `encodeMessage(1, …)` の中身（metadata field 群、jwt 無し）。
    pub metadata_body: Vec<u8>,
    /// field 3 prompts 以降の全部（byte 順は TS 同一）。
    pub tail: Vec<u8>,
}

pub struct ChatRequestArgs<'a> {
    pub api_key: &'a str,
    pub model_uid: &'a str,
    pub messages: &'a [ChatHistoryItem],
    pub tools: Option<&'a [ToolDef]>,
    pub cascade_id: &'a str,
    pub prompt_id: &'a str,
    pub session_id: &'a str,
    /// `Date.now()`（epoch millis）。
    pub request_id: u64,
    pub trigger_id: &'a str,
    pub max_output_tokens: Option<u64>,
}

/// `buildGetChatMessageRequest` — jwt は transport が mint するためここでは
/// metadata（field 21 無し）+ tail に分離して返す。wire byte 順は TS と
/// 同一（`encodeMessage(1, metadata_body + encodeString(21, jwt)) + tail`
/// と組み立てれば完全一致）。
pub fn build_get_chat_message_request_parts(args: &ChatRequestArgs<'_>) -> RequestParts {
    let metadata_body = build_metadata(&MetadataInput {
        api_key: args.api_key,
        user_jwt: None,
        session_id: args.session_id,
        request_id: args.request_id,
        trigger_id: args.trigger_id,
        version: None,
        ide: None,
    });
    let mut tail = Vec::new();
    for message in collapse_system_into_user(args.messages) {
        let tool_call_id = (message.role == ChatRole::Tool)
            .then_some(message.tool_call_id.as_deref())
            .flatten();
        let tool_calls = (message.role == ChatRole::Assistant)
            .then_some(message.tool_calls.as_deref())
            .flatten();
        tail.extend_from_slice(&encode_message(
            3,
            &encode_chat_message_prompt(
                &normalize_content(&message.content),
                source_of(message.role),
                tool_call_id,
                tool_calls,
            ),
        ));
    }
    tail.extend_from_slice(&encode_varint_field(7, 5));
    tail.extend_from_slice(&encode_message(
        8,
        &encode_completion_configuration(args.max_output_tokens),
    ));
    for tool in args.tools.into_iter().flatten() {
        tail.extend_from_slice(&encode_message(10, &encode_tool_def(tool)));
    }
    tail.extend_from_slice(&encode_string(16, args.cascade_id));
    tail.extend_from_slice(&encode_string(21, args.model_uid));
    tail.extend_from_slice(&encode_string(22, args.prompt_id));
    RequestParts {
        metadata_body,
        tail,
    }
}

/// `decodeChatFrame` — response proto 1 frame → `ChatEvent` 列。
/// truncated varint は TS の throw 相当で `Err`。
pub fn decode_chat_frame(proto: &[u8]) -> Result<Vec<ChatEvent>, DecodeError> {
    let mut events = Vec::new();
    for field in iter_fields(proto) {
        let field = field?;
        match (field.num, field.wire) {
            (3, 2) => {
                if let Some(text) = field.string() {
                    if !text.is_empty() {
                        events.push(ChatEvent::Text(text));
                    }
                }
            }
            (9, 2) => {
                if let Some(text) = field.string() {
                    if !text.is_empty() {
                        events.push(ChatEvent::Reasoning(text));
                    }
                }
            }
            (6, 2) => {
                if let Some(inner_buf) = field.bytes() {
                    let mut id: Option<String> = None;
                    let mut name: Option<String> = None;
                    let mut args_delta: Option<String> = None;
                    for inner in iter_fields(inner_buf) {
                        let inner = inner?;
                        if inner.wire != 2 {
                            continue;
                        }
                        match inner.num {
                            1 => id = inner.string(),
                            2 => name = inner.string(),
                            3 => args_delta = inner.string(),
                            _ => {}
                        }
                    }
                    if let (Some(id), Some(name)) = (&id, &name) {
                        events.push(ChatEvent::ToolCallStart {
                            id: id.clone(),
                            name: name.clone(),
                        });
                    }
                    if let Some(delta) = args_delta {
                        // TS `...(id ? {id} : {})` — 空文字 id は drop。
                        let id = id.filter(|id| !id.is_empty());
                        events.push(ChatEvent::ToolCallArgs { delta, id });
                    }
                }
            }
            (5, 0) => {
                if let Some(value) = field.varint() {
                    let reason = match value {
                        10 => ChatFinishReason::ToolCalls,
                        11 => ChatFinishReason::ContentFilter,
                        1 | 3 => ChatFinishReason::Length,
                        _ => ChatFinishReason::Stop,
                    };
                    events.push(ChatEvent::Finish(reason));
                }
            }
            (28, 2) => {
                if let Some(buf) = field.bytes() {
                    if let Some(usage) = decode_usage(buf)? {
                        events.push(ChatEvent::Usage(usage));
                    }
                }
            }
            _ => {}
        }
    }
    Ok(events)
}

/// `decodeUsage` — `{2*: {5: metric, 4: {2: float32le}}}`。
fn decode_usage(buf: &[u8]) -> Result<Option<ChatUsage>, DecodeError> {
    let mut usage = ChatUsage::default();
    for field in iter_fields(buf) {
        let field = field?;
        if field.num != 2 || field.wire != 2 {
            continue;
        }
        let Some(inner_buf) = field.bytes() else {
            continue;
        };
        let mut metric: Option<String> = None;
        let mut value: Option<f64> = None;
        for inner in iter_fields(inner_buf) {
            let inner = inner?;
            match (inner.num, inner.wire) {
                (5, 2) => metric = inner.string(),
                (4, 2) => {
                    if let Some(dim_buf) = inner.bytes() {
                        for dim in iter_fields(dim_buf) {
                            let dim = dim?;
                            if dim.num == 2 && dim.wire == 5 {
                                if let Some(bytes) = dim.bytes() {
                                    if bytes.len() == 4 {
                                        value = Some(f32::from_le_bytes([
                                            bytes[0], bytes[1], bytes[2], bytes[3],
                                        ])
                                            as f64);
                                    }
                                }
                            }
                        }
                    }
                }
                _ => {}
            }
        }
        let (Some(metric), Some(raw)) = (metric, value) else {
            continue;
        };
        if !raw.is_finite() {
            continue;
        }
        let n = raw.round() as u64;
        if metric == "input_tokens" {
            usage.prompt_tokens = Some(n);
        } else if metric == "output_tokens" {
            usage.completion_tokens = Some(n);
        } else if metric.contains("cached") || metric.contains("cache_read") {
            usage.cached_input_tokens = Some(n);
        } else if metric.contains("cache_creation") {
            usage.cache_creation_input_tokens = Some(n);
        }
    }
    if usage.prompt_tokens.is_none() && usage.completion_tokens.is_none() {
        return Ok(None);
    }
    usage.total_tokens =
        Some(usage.prompt_tokens.unwrap_or(0) + usage.completion_tokens.unwrap_or(0));
    Ok(Some(usage))
}

/// `(sessionId, cascadeId)` — `(host, apiKey)` 単位の process 内 cache
/// （TS `sessionCache` module Map 相当）。
pub struct SessionIds {
    pub session_id: String,
    pub cascade_id: String,
}

static SESSIONS: Mutex<Option<HashMap<String, (String, String)>>> = Mutex::new(None);

/// `sessionIds(apiKey, host)`。
pub fn session_ids(api_key: &str, host: &str) -> SessionIds {
    let key = format!("{host}\u{1f}{api_key}");
    let mut guard = SESSIONS.lock().unwrap_or_else(|e| e.into_inner());
    let map = guard.get_or_insert_with(HashMap::new);
    let (session_id, cascade_id) = map.entry(key).or_insert_with(|| {
        (
            uuid::Uuid::new_v4().to_string(),
            uuid::Uuid::new_v4().to_string(),
        )
    });
    SessionIds {
        session_id: session_id.clone(),
        cascade_id: cascade_id.clone(),
    }
}

/// test 用の session cache 消去（TS は process 内 cache を持つが
/// test 境界では process 分離される）。
#[doc(hidden)]
pub fn __clear_session_cache_for_tests() {
    *SESSIONS.lock().unwrap_or_else(|e| e.into_inner()) = None;
}

/// Connect trailer frame（flags `0x02`）の error 抽出 — TS
/// `text.includes('"error"')` → `parsed.error?.message ?? text`。
pub fn trailer_error(payload: &[u8]) -> Option<String> {
    let text = String::from_utf8_lossy(payload).into_owned();
    if !text.contains("\"error\"") {
        return None;
    }
    match serde_json::from_str::<serde_json::Value>(&text) {
        Ok(parsed) => Some(
            parsed
                .get("error")
                .and_then(|e| e.get("message"))
                .and_then(|m| m.as_str())
                .map(str::to_owned)
                .unwrap_or(text),
        ),
        Err(_) => Some(text),
    }
}

/// transport が decoder へ渡す 1 frame。`EndOfStream` は Connect trailer
/// （flag 0x02）— `error` は trailer body の error.message 抽出結果。
#[derive(Debug)]
pub enum DevinFrame {
    /// 応答 proto 1 message。
    Message(Vec<u8>),
    /// EOS trailer（`Ok` 終端 or error payload）。
    EndOfStream { error: Option<String> },
}

/// `deframe` 時の失敗 — chunk transport error（`E`）または gzip decode。
/// rig_provider が `ProviderError` へ写像する。
#[derive(Debug, thiserror::Error)]
pub enum DeframeError<E> {
    /// body chunk stream の transport error（reader read throw 相当）。
    #[error("stream chunk error: {0}")]
    Chunk(E),
    /// gzip payload の伸長失敗（`zlib.gunzipSync` throw 相当）。
    #[error(transparent)]
    Decode(#[from] DecodeError),
}

/// Connect byte stream → frame 列（TS の peek/drop queue + gunzip +
/// EOS handling の port）。`body` は chunk stream（`Bytes`）。
/// EOS 未到達の自然終端はそのまま stream を閉じる（`eof` 判定は
/// decoder 側の責務 — TS `!sawEos → error` に対応）。
pub fn deframe_connect<S, E>(body: S) -> impl Stream<Item = Result<DevinFrame, DeframeError<E>>>
where
    S: Stream<Item = Result<Bytes, E>>,
{
    async_stream::stream! {
        let mut buf: Vec<u8> = Vec::new();
        futures::pin_mut!(body);
        while let Some(chunk) = futures::StreamExt::next(&mut body).await {
            match chunk {
                Ok(bytes) => buf.extend_from_slice(&bytes),
                Err(e) => {
                    // stream 境界の `cheaper.provider.devin` event(design
                    // §instrumentation 配置) — chunk 本文は載せない。
                    tracing::debug!(target: "cheaper.provider.devin", "stream error");
                    yield Err(DeframeError::Chunk(e));
                    return;
                }
            }
            while buf.len() >= 5 {
                let flags = buf[0];
                let len = u32::from_be_bytes([buf[1], buf[2], buf[3], buf[4]]) as usize;
                if buf.len() < 5 + len {
                    break;
                }
                let raw: Vec<u8> = buf.drain(..5 + len).skip(5).collect();
                let payload = if flags & 0x01 != 0 {
                    match gunzip(&raw) {
                        Ok(p) => p,
                        Err(e) => {
                            tracing::debug!(target: "cheaper.provider.devin", "stream error");
                            yield Err(DeframeError::Decode(e));
                            return;
                        }
                    }
                } else {
                    raw
                };
                if flags & 0x02 != 0 {
                    tracing::debug!(target: "cheaper.provider.devin", "stream end");
                    yield Ok(DevinFrame::EndOfStream { error: trailer_error(&payload) });
                    continue;
                }
                yield Ok(DevinFrame::Message(payload));
            }
        }
    }
}
