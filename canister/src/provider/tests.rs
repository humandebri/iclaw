//! where: iclaw/canister/src/provider/tests.rs
//! what: focused provider tests for the ICP OpenAI-compatible adapter
//! why: keep provider.rs compact while validating allowlist, tool calls, and response handling

use super::*;
use iclaw_core::providers::{ChatMessage, ConversationMessage};
use iclaw_core::tools::ToolSpec;
use std::sync::Mutex;

struct MockTransport {
    responses: Mutex<Vec<anyhow::Result<HttpResponse>>>,
}

#[async_trait]
impl OutboundHttp for MockTransport {
    async fn post_json(
        &self,
        _url: &str,
        _bearer_token: &str,
        _body: Vec<u8>,
        _max_response_bytes: usize,
    ) -> anyhow::Result<HttpResponse> {
        self.responses
            .lock()
            .expect("mock transport lock should succeed")
            .pop()
            .expect("mock transport should have one response queued")
    }
}

fn config(url: &str) -> ProviderConfig {
    ProviderConfig {
        api_url: url.to_string(),
        api_key: "openai-test-key".to_string(),
        default_model: "gpt-4o-mini".to_string(),
        timeout_secs: Some(30),
    }
}

#[tokio::test]
async fn openai_provider_supports_native_tools() {
    let provider = IcOpenAiProvider::with_transport(
        &config("https://api.openai.com/v1"),
        Arc::new(MockTransport {
            responses: Mutex::new(vec![Ok(HttpResponse {
                status_code: 200,
                body: br#"{"choices":[{"message":{"content":"hello from mock"}}]}"#.to_vec(),
            })]),
        }),
    )
    .unwrap();

    let response = Provider::chat_with_system(&provider, None, "hello", "gpt-4o-mini", 0.1)
        .await
        .unwrap();
    assert_eq!(response, "hello from mock");
    let capabilities = IcCanisterProvider::capabilities(&provider);
    assert!(capabilities.native_tool_calling);
    assert!(!capabilities.vision);
}

#[tokio::test]
async fn canister_provider_returns_upstream_model_metadata() {
    let provider = IcOpenAiProvider::with_transport(
        &config("https://api.openai.com/v1"),
        Arc::new(MockTransport {
            responses: Mutex::new(vec![Ok(HttpResponse {
                status_code: 200,
                body: br#"{"model":"gpt-4.1-mini","choices":[{"message":{"content":"hello from mock"}}]}"#
                    .to_vec(),
            })]),
        }),
    )
    .unwrap();

    let response = IcCanisterProvider::chat(
        &provider,
        &[ConversationMessage::Chat(ChatMessage::user("hello"))],
        None,
        "gpt-4o-mini",
        0.1,
    )
    .await
    .unwrap();

    assert_eq!(response.response.text.as_deref(), Some("hello from mock"));
    assert_eq!(response.model.as_deref(), Some("gpt-4.1-mini"));
}

#[tokio::test]
async fn canister_provider_parses_native_tool_calls() {
    let provider = IcOpenAiProvider::with_transport(
        &config("https://api.openai.com/v1"),
        Arc::new(MockTransport {
            responses: Mutex::new(vec![Ok(HttpResponse {
                status_code: 200,
                body: br#"{"model":"gpt-4.1-mini","choices":[{"message":{"content":"checking","tool_calls":[{"id":"call_1","type":"function","function":{"name":"memory_recall","arguments":"{\"query\":\"hello\"}"}}]}}]}"#
                    .to_vec(),
            })]),
        }),
    )
    .unwrap();

    let response = IcCanisterProvider::chat(
        &provider,
        &[ConversationMessage::Chat(ChatMessage::user("hello"))],
        Some(&[ToolSpec {
            name: "memory_recall".to_string(),
            description: "Recall memory".to_string(),
            parameters: serde_json::json!({"type":"object"}),
        }]),
        "gpt-4o-mini",
        0.1,
    )
    .await
    .unwrap();

    assert_eq!(response.response.text.as_deref(), Some("checking"));
    assert_eq!(response.response.tool_calls.len(), 1);
    assert_eq!(response.response.tool_calls[0].name, "memory_recall");
}

#[test]
fn transport_rejects_private_host() {
    let error = CanisterHttpTransport::new(&config("https://127.0.0.1:8080/v1"))
        .unwrap_err()
        .to_string();
    assert!(error.contains("Blocked local/private host"));
}

#[test]
fn transport_rejects_shared_address_space_host() {
    let error = CanisterHttpTransport::new(&config("https://100.64.0.1/v1"))
        .unwrap_err()
        .to_string();
    assert!(error.contains("Blocked local/private host"));
}

#[test]
fn transport_rejects_private_172_range_host() {
    let error = CanisterHttpTransport::new(&config("https://172.16.0.1/v1"))
        .unwrap_err()
        .to_string();
    assert!(error.contains("Blocked local/private host"));
}

#[test]
fn transport_rejects_ipv6_literal_host() {
    let error = CanisterHttpTransport::new(&config("https://[fe80::1]/v1"))
        .unwrap_err()
        .to_string();
    assert!(error.contains("URL must include a valid host"));
}

#[test]
fn transport_rejects_unspecified_ipv4_host() {
    let error = CanisterHttpTransport::new(&config("https://0.0.0.0/v1"))
        .unwrap_err()
        .to_string();
    assert!(error.contains("Blocked local/private host"));
}

#[test]
fn transport_requires_https() {
    let error = CanisterHttpTransport::new(&config("http://api.openai.com/v1"))
        .unwrap_err()
        .to_string();
    assert!(error.contains("Only https:// URLs are allowed"));
}
