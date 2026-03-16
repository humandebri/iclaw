//! where: iclaw/canister/src/service/compression/tests.rs
//! what: focused tests for request-size estimation alignment
//! why: keep payload-size preflight locked to the provider's actual JSON request shape

use super::estimate_request_bytes;
use crate::provider::messages::build_chat_request;
use iclaw_core::providers::{ChatMessage, ConversationMessage, ToolCall, ToolResultMessage};
use iclaw_core::tools::ToolSpec;
use serde_json::json;

#[test]
fn estimate_request_bytes_matches_chat_only_request_shape() {
    let messages = vec![
        ConversationMessage::Chat(ChatMessage::system("system prompt")),
        ConversationMessage::Chat(ChatMessage::user("hello")),
    ];
    assert_request_bytes_match(&messages, None, "gpt-4o-mini", 0.0);
}

#[test]
fn estimate_request_bytes_matches_tool_call_request_shape() {
    let messages = vec![ConversationMessage::AssistantToolCalls {
        text: Some("calling tool".to_string()),
        tool_calls: vec![ToolCall {
            id: "call-1".to_string(),
            name: "memory_store".to_string(),
            arguments: "{\"key\":\"note\"}".to_string(),
        }],
        reasoning_content: Some("reasoning".to_string()),
    }];
    assert_request_bytes_match(&messages, None, "gpt-4o-mini", 0.0);
}

#[test]
fn estimate_request_bytes_matches_tool_result_request_shape() {
    let messages = vec![ConversationMessage::ToolResults(vec![ToolResultMessage {
        tool_call_id: "call-1".to_string(),
        content: "{\"success\":true}".to_string(),
    }])];
    assert_request_bytes_match(&messages, None, "gpt-4o-mini", 0.0);
}

#[test]
fn estimate_request_bytes_matches_request_shape_with_tools() {
    let messages = vec![ConversationMessage::Chat(ChatMessage::user("use tools"))];
    let tools = vec![ToolSpec {
        name: "memory_store".to_string(),
        description: "Store memory".to_string(),
        parameters: json!({
            "type": "object",
            "properties": {
                "key": { "type": "string" },
            },
            "required": ["key"],
        }),
    }];
    assert_request_bytes_match(&messages, Some(&tools), "gpt-4o-mini", 0.2);
}

fn assert_request_bytes_match(
    messages: &[ConversationMessage],
    tools: Option<&[ToolSpec]>,
    model: &str,
    temperature: f64,
) {
    let estimated = estimate_request_bytes(messages, tools, model, temperature);
    let request = build_chat_request(messages, tools, model, temperature);
    let serialized = serde_json::to_vec(&request).expect("request should serialize");
    assert_eq!(estimated, serialized.len());
}
