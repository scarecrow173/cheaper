use rig::completion::CompletionRequest;
use rig::message::{
    AssistantContent, CallId, DocumentSourceKind, Image, ImageMediaType, Message, Text, ToolCall,
    ToolFunction, ToolName, ToolResult, ToolResultContent, UserContent,
};

use cheaper_provider_devin::context_map::*;

fn request(chat_history: Vec<Message>) -> CompletionRequest {
    CompletionRequest {
        model: None,
        chat_history,
        documents: vec![],
        tools: vec![],
        temperature: None,
        max_tokens: None,
        tool_choice: None,
        additional_params: None,
        output_schema: None,
        record_telemetry_content: false,
    }
}

fn tool_name(name: &str) -> ToolName {
    ToolName::try_from(name).unwrap()
}

#[test]
fn maps_system_user_assistant_roles() {
    let mapped = map_request_to_chat(&request(vec![
        Message::System {
            content: "sys".to_string(),
        },
        Message::User {
            content: vec![UserContent::Text(Text::new("hi"))],
        },
        Message::Assistant {
            id: None,
            content: vec![
                AssistantContent::Text(Text::new("a1")),
                AssistantContent::Text(Text::new("a2")),
            ],
        },
    ]));
    assert_eq!(mapped.messages.len(), 3);
    assert_eq!(mapped.messages[0].role, ChatRole::System);
    assert_eq!(mapped.messages[1].role, ChatRole::User);
    assert_eq!(mapped.messages[2].role, ChatRole::Assistant);
    // assistant text part は "\n" join。
    assert_eq!(
        mapped.messages[2].content,
        ChatContent::Text("a1\na2".to_string())
    );
}

#[test]
fn maps_base64_image_to_image_part() {
    let mapped = map_request_to_chat(&request(vec![Message::User {
        content: vec![
            UserContent::Text(Text::new("see")),
            UserContent::Image(Image {
                data: DocumentSourceKind::Base64("aGk=".to_string()),
                media_type: Some(ImageMediaType::PNG),
                detail: None,
                additional_params: None,
            }),
        ],
    }]));
    match &mapped.messages[0].content {
        ChatContent::Parts(parts) => {
            assert_eq!(parts.len(), 2);
            assert_eq!(
                parts[1],
                ContentPart::Image {
                    mime_type: Some("image/png".to_string()),
                    base64_data: Some("aGk=".to_string()),
                }
            );
        }
        _ => panic!("expected parts"),
    }
}

#[test]
fn url_image_is_omitted_from_wire_parts() {
    // URL 参照は Devin wire に送れる形が無い（TS 同様 data のみ）。
    let mapped = map_request_to_chat(&request(vec![Message::User {
        content: vec![UserContent::Image(Image {
            data: DocumentSourceKind::Url("https://x/img.png".to_string()),
            media_type: None,
            detail: None,
            additional_params: None,
        })],
    }]));
    match &mapped.messages[0].content {
        ChatContent::Parts(parts) => assert_eq!(
            parts[0],
            ContentPart::Image {
                mime_type: None,
                base64_data: None
            }
        ),
        _ => panic!("expected parts"),
    }
}

#[test]
fn maps_tool_calls_and_tool_results() {
    let mapped = map_request_to_chat(&request(vec![
        Message::Assistant {
            id: None,
            content: vec![
                AssistantContent::Text(Text::new("call")),
                AssistantContent::ToolCall(ToolCall::new(
                    CallId::from_wire("call_9"),
                    ToolFunction::new(tool_name("read"), serde_json::json!({"path":"x"})),
                )),
            ],
        },
        Message::User {
            content: vec![UserContent::ToolResult(ToolResult {
                call: CallId::from_wire("call_9"),
                name: tool_name("read"),
                content: vec![
                    ToolResultContent::Text(Text::new("l1")),
                    ToolResultContent::Text(Text::new("l2")),
                ],
            })],
        },
    ]));
    assert_eq!(mapped.messages.len(), 2);
    let assistant = &mapped.messages[0];
    let calls = assistant.tool_calls.as_ref().unwrap();
    assert_eq!(calls[0].id, "call_9");
    assert_eq!(calls[0].name, "read");
    assert_eq!(calls[0].arguments, "{\"path\":\"x\"}");
    let tool = &mapped.messages[1];
    assert_eq!(tool.role, ChatRole::Tool);
    assert_eq!(tool.tool_call_id.as_deref(), Some("call_9"));
    assert_eq!(tool.content, ChatContent::Text("l1\nl2".to_string()));
}

#[test]
fn empty_user_message_still_emits() {
    // TS: content が空でも user item を 1 件 emit する。
    let mapped = map_request_to_chat(&request(vec![Message::User { content: vec![] }]));
    assert_eq!(mapped.messages.len(), 1);
    assert_eq!(mapped.messages[0].role, ChatRole::User);
}

#[test]
fn tool_definitions_map_name_description_parameters() {
    let mut req = request(vec![Message::User {
        content: vec![UserContent::Text(Text::new("x"))],
    }]);
    req.tools = vec![rig::completion::ToolDefinition {
        name: "read".to_string(),
        description: "read file".to_string(),
        parameters: serde_json::json!({"type":"object"}),
    }];
    let mapped = map_request_to_chat(&req);
    assert_eq!(mapped.tools.len(), 1);
    assert_eq!(mapped.tools[0].name, "read");
}
