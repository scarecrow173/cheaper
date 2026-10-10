use bytes::Bytes;
use futures::StreamExt;

use cheaper_provider_devin::context_map::{
    ChatContent, ChatHistoryItem, ChatRole, ContentPart, ToolDef,
};
use cheaper_provider_devin::stream::*;
use cheaper_provider_devin::wire::*;

fn user(text: &str) -> ChatHistoryItem {
    ChatHistoryItem {
        role: ChatRole::User,
        content: ChatContent::Text(text.to_string()),
        tool_call_id: None,
        tool_calls: None,
    }
}

fn args<'a>(messages: &'a [ChatHistoryItem]) -> ChatRequestArgs<'a> {
    ChatRequestArgs {
        api_key: "key",
        model_uid: "model-uid-high",
        messages,
        tools: None,
        cascade_id: "cascade",
        prompt_id: "prompt",
        session_id: "session",
        request_id: 1_700_000,
        trigger_id: "trigger",
        max_output_tokens: None,
    }
}

#[test]
fn request_wire_layout_matches_ts() {
    let parts = build_get_chat_message_request_parts(&args(&[user("hello")]));
    // tail: field 3 prompt → 7 varint → 8 config → 16/21/22 strings。
    let fields: Vec<ProtoField> = iter_fields(&parts.tail).collect::<Result<_, _>>().unwrap();
    let nums: Vec<u32> = fields.iter().map(|f| f.num).collect();
    assert_eq!(nums, vec![3, 7, 8, 16, 21, 22]);
    // prompt: {2: source=1(user), 3: "hello", 4: tokens, 5: 1}
    let prompt: Vec<ProtoField> = iter_fields(fields[0].bytes().unwrap())
        .collect::<Result<_, _>>()
        .unwrap();
    assert_eq!(prompt[0].varint(), Some(1));
    assert_eq!(prompt[1].string().as_deref(), Some("hello"));
    assert_eq!(prompt[3].varint(), Some(1));
    // field 7 = 5（TS 固定値）、field 21 = model uid。
    assert_eq!(fields[1].varint(), Some(5));
    assert_eq!(fields[4].string().as_deref(), Some("model-uid-high"));
    assert_eq!(fields[5].string().as_deref(), Some("prompt"));
}

#[test]
fn request_encodes_assistant_tool_calls_and_tool_results() {
    let messages = vec![
        ChatHistoryItem {
            role: ChatRole::Assistant,
            content: ChatContent::Text("calling".to_string()),
            tool_call_id: None,
            tool_calls: Some(vec![cheaper_provider_devin::context_map::ChatToolCall {
                id: "call_1".to_string(),
                name: "read".to_string(),
                arguments: "{\"path\":\"a\"}".to_string(),
            }]),
        },
        ChatHistoryItem {
            role: ChatRole::Tool,
            content: ChatContent::Text("result".to_string()),
            tool_call_id: Some("call_1".to_string()),
            tool_calls: None,
        },
    ];
    let parts = build_get_chat_message_request_parts(&args(&messages));
    let prompt_bufs: Vec<Vec<u8>> = iter_fields(&parts.tail)
        .flatten()
        .filter(|f| f.num == 3)
        .map(|f| f.bytes().unwrap().to_vec())
        .collect();
    let prompts: Vec<Vec<ProtoField>> = prompt_bufs
        .iter()
        .map(|buf| iter_fields(buf).collect::<Result<_, _>>().unwrap())
        .collect();
    assert_eq!(prompts.len(), 2);
    // assistant: source=2、toolCalls field 6。
    assert_eq!(prompts[0][0].varint(), Some(2));
    let call_buf = prompts[0]
        .iter()
        .find(|f| f.num == 6)
        .unwrap()
        .bytes()
        .unwrap()
        .to_vec();
    let call: Vec<ProtoField> = iter_fields(&call_buf).collect::<Result<_, _>>().unwrap();
    assert_eq!(call[0].string().as_deref(), Some("call_1"));
    assert_eq!(call[1].string().as_deref(), Some("read"));
    // tool result: source=4、toolCallId field 7。
    assert_eq!(prompts[1][0].varint(), Some(4));
    assert_eq!(
        prompts[1]
            .iter()
            .find(|f| f.num == 7)
            .unwrap()
            .string()
            .as_deref(),
        Some("call_1")
    );
}

#[test]
fn collapse_system_into_user_wraps_next_user() {
    let messages = vec![
        ChatHistoryItem {
            role: ChatRole::System,
            content: ChatContent::Text("sys".to_string()),
            tool_call_id: None,
            tool_calls: None,
        },
        user("body"),
    ];
    let collapsed = collapse_system_into_user(&messages);
    assert_eq!(collapsed.len(), 1);
    match &collapsed[0].content {
        ChatContent::Parts(parts) => match &parts[0] {
            ContentPart::Text(t) => assert_eq!(t, "<system>\nsys\n</system>\nbody"),
            _ => panic!("expected text part"),
        },
        _ => panic!("expected parts"),
    }
}

#[test]
fn decode_chat_frame_maps_all_event_kinds() {
    let mut proto = Vec::new();
    proto.extend_from_slice(&encode_string(3, "answer"));
    proto.extend_from_slice(&encode_string(9, "thinking"));
    let mut call = encode_string(1, "id1");
    call.extend_from_slice(&encode_string(2, "read"));
    call.extend_from_slice(&encode_string(3, "{\"x\":"));
    proto.extend_from_slice(&encode_message(6, &call));
    proto.extend_from_slice(&encode_varint_field(5, 10));
    let events = decode_chat_frame(&proto).unwrap();
    assert_eq!(events[0], ChatEvent::Text("answer".to_string()));
    assert_eq!(events[1], ChatEvent::Reasoning("thinking".to_string()));
    assert_eq!(
        events[2],
        ChatEvent::ToolCallStart {
            id: "id1".to_string(),
            name: "read".to_string()
        }
    );
    assert_eq!(
        events[3],
        ChatEvent::ToolCallArgs {
            delta: "{\"x\":".to_string(),
            id: Some("id1".to_string())
        }
    );
    assert_eq!(events[4], ChatEvent::Finish(ChatFinishReason::ToolCalls));
}

#[test]
fn decode_chat_frame_drops_empty_text_and_empty_call_id() {
    let mut proto = encode_string(3, "");
    // 空 id + args delta → `id` は drop（TS `...(id ? {id} : {})`）。
    let mut call = encode_string(1, "");
    call.extend_from_slice(&encode_string(2, "read"));
    call.extend_from_slice(&encode_string(3, "{}"));
    proto.extend_from_slice(&encode_message(6, &call));
    let events = decode_chat_frame(&proto).unwrap();
    // id+name が存在すれば ToolCallStart は emit される（id 空文字でも —
    // drop されるのは ToolCallArgs の `id` だけ）。
    assert_eq!(events.len(), 2);
    assert_eq!(
        events[0],
        ChatEvent::ToolCallStart {
            id: String::new(),
            name: "read".to_string()
        }
    );
    assert_eq!(
        events[1],
        ChatEvent::ToolCallArgs {
            delta: "{}".to_string(),
            id: None
        }
    );
}

#[test]
fn decode_chat_frame_maps_finish_reasons() {
    for (wire, expected) in [
        (10, ChatFinishReason::ToolCalls),
        (11, ChatFinishReason::ContentFilter),
        (1, ChatFinishReason::Length),
        (3, ChatFinishReason::Length),
        (0, ChatFinishReason::Stop),
    ] {
        let proto = encode_varint_field(5, wire);
        assert_eq!(
            decode_chat_frame(&proto).unwrap(),
            vec![ChatEvent::Finish(expected)]
        );
    }
}

// {2: {5:"<metric name>", 4:{2: float32le}}} usage metric record。
fn metric(name: &str, value: f32) -> Vec<u8> {
    let mut dim = Vec::new();
    // field 2 wire 5 fixed32。
    dim.extend_from_slice(&encode_tag(2, 5));
    dim.extend_from_slice(&value.to_le_bytes());
    let mut m = encode_string(5, name);
    m.extend_from_slice(&encode_message(4, &dim));
    m
}

#[test]
fn decode_chat_frame_reads_usage_metrics() {
    let mut usage_msg = encode_message(2, &metric("input_tokens", 12.0));
    usage_msg.extend_from_slice(&encode_message(2, &metric("output_tokens", 34.0)));
    usage_msg.extend_from_slice(&encode_message(2, &metric("cached_input_tokens", 5.0)));
    let proto = encode_message(28, &usage_msg);
    let events = decode_chat_frame(&proto).unwrap();
    assert_eq!(
        events,
        vec![ChatEvent::Usage(ChatUsage {
            prompt_tokens: Some(12),
            completion_tokens: Some(34),
            total_tokens: Some(46),
            cached_input_tokens: Some(5),
            cache_creation_input_tokens: None,
        })]
    );
}

#[test]
fn decode_chat_frame_maps_devin_cache_hit_usage_shape() {
    // Issue #86 の形状: cache hit 時は Devin が input_tokens に
    // uncached 分のみ、cache-hit 分を cached_input_tokens に載せる。
    // `total_tokens` metric は無視され input+output で再計算される
    // 契約を pin する。
    let mut usage_msg = encode_message(2, &metric("input_tokens", 1.0));
    usage_msg.extend_from_slice(&encode_message(2, &metric("output_tokens", 211.0)));
    usage_msg.extend_from_slice(&encode_message(2, &metric("cached_input_tokens", 3196.0)));
    usage_msg.extend_from_slice(&encode_message(
        2,
        &metric("cache_creation_input_tokens", 7.0),
    ));
    usage_msg.extend_from_slice(&encode_message(2, &metric("total_tokens", 9999.0)));
    let proto = encode_message(28, &usage_msg);
    let events = decode_chat_frame(&proto).unwrap();
    assert_eq!(
        events,
        vec![ChatEvent::Usage(ChatUsage {
            prompt_tokens: Some(1),
            completion_tokens: Some(211),
            total_tokens: Some(212),
            cached_input_tokens: Some(3196),
            cache_creation_input_tokens: Some(7),
        })]
    );
}

#[test]
fn decode_chat_frame_drops_cache_only_usage_frame() {
    // 現行挙動の pin: prompt/completion を持たない usage frame は
    // cache metric のみでも破棄される（spec の deferred concern;
    // 変更ではなく記録）。
    let usage_msg = encode_message(2, &metric("cached_input_tokens", 3196.0));
    let proto = encode_message(28, &usage_msg);
    let events = decode_chat_frame(&proto).unwrap();
    assert_eq!(events, vec![]);
}

#[test]
fn deframe_splits_chunks_and_handles_eos() {
    let body = frame_connect_stream(&encode_string(3, "hi"), false);
    let eos = {
        let mut f = Vec::new();
        f.push(0x02u8);
        f.extend_from_slice(&7u32.to_be_bytes());
        f.extend_from_slice(br#"{"x":1}"#);
        f
    };
    let wire_bytes = [body, eos].concat();
    // 1 byte ずつ刻んで partial header/payload を跨がせる。
    let chunks: Vec<Result<Bytes, std::io::Error>> = wire_bytes
        .chunks(3)
        .map(|c| Ok(Bytes::copy_from_slice(c)))
        .collect();
    let stream = futures::stream::iter(chunks);
    let frames: Vec<DevinFrame> =
        futures::executor::block_on(deframe_connect(stream).collect::<Vec<_>>())
            .into_iter()
            .collect::<Result<_, _>>()
            .unwrap();
    assert_eq!(frames.len(), 2);
    match &frames[0] {
        DevinFrame::Message(proto) => {
            assert_eq!(
                decode_chat_frame(proto).unwrap(),
                vec![ChatEvent::Text("hi".to_string())]
            );
        }
        _ => panic!("expected message"),
    }
    assert!(matches!(frames[1], DevinFrame::EndOfStream { error: None }));
}

#[test]
fn deframe_gunzips_and_extracts_trailer_error() {
    let body = frame_connect_stream(&encode_string(3, "x"), true);
    let trailer_payload = br#"{"error":{"message":"boom"}}"#;
    let mut eos = vec![0x02u8];
    eos.extend_from_slice(&(trailer_payload.len() as u32).to_be_bytes());
    eos.extend_from_slice(trailer_payload);
    let stream = futures::stream::iter(vec![Ok::<_, std::io::Error>(Bytes::from(
        [body, eos].concat(),
    ))]);
    let frames: Vec<DevinFrame> =
        futures::executor::block_on(deframe_connect(stream).collect::<Vec<_>>())
            .into_iter()
            .collect::<Result<_, _>>()
            .unwrap();
    assert!(matches!(
        &frames[1],
        DevinFrame::EndOfStream { error: Some(e) } if e == "boom"
    ));
}

#[test]
fn session_ids_are_cached_per_key() {
    __clear_session_cache_for_tests();
    let a = session_ids("k1", "h");
    let b = session_ids("k1", "h");
    let c = session_ids("k2", "h");
    assert_eq!(a.session_id, b.session_id);
    assert_ne!(a.session_id, c.session_id);
}

#[test]
fn tools_encode_as_field_10() {
    let messages = [user("x")];
    let mut a = args(&messages);
    let tools = vec![ToolDef {
        name: "read".to_string(),
        description: "desc".to_string(),
        parameters: serde_json::json!({"type":"object"}),
    }];
    a.tools = Some(&tools);
    let parts = build_get_chat_message_request_parts(&a);
    let tool_field = iter_fields(&parts.tail)
        .flatten()
        .find(|f| f.num == 10)
        .expect("tool field");
    let tool: Vec<ProtoField> = iter_fields(tool_field.bytes().unwrap())
        .collect::<Result<_, _>>()
        .unwrap();
    assert_eq!(tool[0].string().as_deref(), Some("read"));
    assert_eq!(tool[1].string().as_deref(), Some("desc"));
    assert_eq!(tool[2].string().as_deref(), Some("{\"type\":\"object\"}"));
}
