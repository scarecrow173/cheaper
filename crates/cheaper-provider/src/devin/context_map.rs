//! Pi `Context` → Devin chat message/tool 変換（TS
//! `providers/devin/context-map.ts` の port — vendored from pi-devin,
//! Copyright (c) 2026 Kashyab Ambarani, MIT License）。
//!
//! TS は Pi の `Context`/`Message` を読むが、Rust 側の対応物は Rig
//! `CompletionRequest`（`chat_history` + `tools` + `documents`）。
//! 対応表: `context.systemPrompt` → `chat_history` 内の
//! `Message::System`、`context.messages` → `chat_history`、
//! `context.tools` → `tools`。

use base64::Engine;
use rig::completion::CompletionRequest;
use rig::message::{
    AssistantContent, DocumentSourceKind, Image, Message, MimeType, ToolResultContent, UserContent,
};

/// TS `ContentPart`（user content の text/image）。
#[derive(Debug, Clone, PartialEq)]
pub enum ContentPart {
    Text(String),
    /// `mimeType`・`base64Data` は TS の optional field 相当。
    Image {
        mime_type: Option<String>,
        base64_data: Option<String>,
    },
}

/// TS `ChatHistoryItem.role`。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChatRole {
    User,
    Assistant,
    System,
    Tool,
}

/// TS `content: string | ContentPart[]`。
#[derive(Debug, Clone, PartialEq)]
pub enum ChatContent {
    Text(String),
    Parts(Vec<ContentPart>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct ChatToolCall {
    pub id: String,
    pub name: String,
    /// `JSON.stringify(arguments)` 済みの JSON 文字列。
    pub arguments: String,
}

/// TS `ChatHistoryItem`。
#[derive(Debug, Clone, PartialEq)]
pub struct ChatHistoryItem {
    pub role: ChatRole,
    pub content: ChatContent,
    pub tool_call_id: Option<String>,
    pub tool_calls: Option<Vec<ChatToolCall>>,
}

/// TS `ToolDef`。
#[derive(Debug, Clone, PartialEq)]
pub struct ToolDef {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
}

pub struct MappedChat {
    pub messages: Vec<ChatHistoryItem>,
    pub tools: Vec<ToolDef>,
}

fn user_image_part(image: &Image) -> ContentPart {
    let mime_type = image
        .media_type
        .as_ref()
        .map(|m| m.to_mime_type().to_string());
    let base64_data = match &image.data {
        DocumentSourceKind::Base64(data) | DocumentSourceKind::String(data) => Some(data.clone()),
        DocumentSourceKind::Raw(bytes) => {
            Some(base64::engine::general_purpose::STANDARD.encode(bytes))
        }
        // URL/FileId/Unknown は Devin wire へ送れる形が無い（TS は
        // `data` 相当しか拾わない）→ 省略。
        _ => None,
    };
    ContentPart::Image {
        mime_type,
        base64_data,
    }
}

fn tool_result_text(contents: &[ToolResultContent]) -> String {
    contents
        .iter()
        .filter_map(|part| match part {
            ToolResultContent::Text(text) => Some(text.text.clone()),
            // structured JSON は text 化して運ぶ（Pi toolResult の
            // text part 相当 — TS は text のみ拾う）。
            ToolResultContent::Json { value } => Some(value.to_string()),
            ToolResultContent::Image(_) => None,
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// user message 1 件分（`UserContent` 列）を messages へ畳む。
/// `ToolResult` は TS で独立した "tool" message なので、出現位置で
/// user part の run を切り分ける。
fn push_user_message(messages: &mut Vec<ChatHistoryItem>, contents: &[UserContent]) {
    let mut pending: Vec<ContentPart> = Vec::new();
    let mut saw_non_tool_part = false;
    let mut pushed_any = false;
    let flush = |messages: &mut Vec<ChatHistoryItem>,
                 pending: &mut Vec<ContentPart>,
                 pushed_any: &mut bool| {
        messages.push(ChatHistoryItem {
            role: ChatRole::User,
            content: ChatContent::Parts(std::mem::take(pending)),
            tool_call_id: None,
            tool_calls: None,
        });
        *pushed_any = true;
    };
    for part in contents {
        match part {
            UserContent::ToolResult(result) => {
                if saw_non_tool_part {
                    flush(messages, &mut pending, &mut pushed_any);
                    saw_non_tool_part = false;
                }
                pushed_any = true;
                messages.push(ChatHistoryItem {
                    role: ChatRole::Tool,
                    content: ChatContent::Text(tool_result_text(&result.content)),
                    tool_call_id: Some(result.call.wire().into_owned()),
                    tool_calls: None,
                });
            }
            UserContent::Text(text) => {
                saw_non_tool_part = true;
                pending.push(ContentPart::Text(text.text.clone()));
            }
            UserContent::Image(image) => {
                saw_non_tool_part = true;
                pending.push(user_image_part(image));
            }
            // Document は normalized_documents が String data で持つ —
            // text として運ぶ（TS に対応物は無いが drop しない）。
            UserContent::Document(doc) => {
                saw_non_tool_part = true;
                pending.push(ContentPart::Text(doc.data.to_string()));
            }
            // Audio/Video は TS 同様 skip。
            _ => {}
        }
    }
    // TS は content が空でも user item を必ず 1 件 emit する
    // （`content: []` → prompt text ""）。skip しない。
    if saw_non_tool_part || !pushed_any {
        flush(messages, &mut pending, &mut pushed_any);
    }
}

/// `mapContextToChat` — Rig `CompletionRequest` 版。
pub fn map_request_to_chat(request: &CompletionRequest) -> MappedChat {
    let mut messages: Vec<ChatHistoryItem> = Vec::new();

    // request.documents は「先頭の system message 群の直後」に user
    // message として挿入する（Rig `insert_after_leading_system` の
    // telemetry/send 一致規則に揃える）。
    let documents = request
        .normalized_documents()
        .and_then(|message| match message {
            Message::User { content } => Some(content),
            _ => None,
        });
    let mut documents_inserted = documents.is_none();

    for message in &request.chat_history {
        if !documents_inserted && !matches!(message, Message::System { .. }) {
            if let Some(content) = &documents {
                push_user_message(&mut messages, content);
            }
            documents_inserted = true;
        }
        match message {
            Message::System { content } => {
                messages.push(ChatHistoryItem {
                    role: ChatRole::System,
                    content: ChatContent::Text(content.clone()),
                    tool_call_id: None,
                    tool_calls: None,
                });
            }
            Message::User { content } => push_user_message(&mut messages, content),
            Message::Assistant { content, .. } => {
                let mut texts: Vec<String> = Vec::new();
                let mut tool_calls: Vec<ChatToolCall> = Vec::new();
                for part in content {
                    match part {
                        AssistantContent::Text(text) => texts.push(text.text.clone()),
                        AssistantContent::ToolCall(call) => tool_calls.push(ChatToolCall {
                            id: call.id.wire().into_owned(),
                            name: call.function.name.as_str().to_string(),
                            // TS `JSON.stringify(part.arguments ?? {})`。
                            arguments: serde_json::to_string(&call.function.arguments)
                                .unwrap_or_else(|_| "{}".to_string()),
                        }),
                        // Reasoning/Image は TS 同様 skip。
                        _ => {}
                    }
                }
                messages.push(ChatHistoryItem {
                    role: ChatRole::Assistant,
                    content: ChatContent::Text(texts.join("\n")),
                    tool_call_id: None,
                    tool_calls: if tool_calls.is_empty() {
                        None
                    } else {
                        Some(tool_calls)
                    },
                });
            }
        }
    }
    if !documents_inserted {
        if let Some(content) = &documents {
            push_user_message(&mut messages, content);
        }
    }

    let tools = request
        .tools
        .iter()
        .map(|tool| ToolDef {
            name: tool.name.clone(),
            description: tool.description.clone(),
            parameters: tool.parameters.clone(),
        })
        .collect();

    MappedChat { messages, tools }
}
