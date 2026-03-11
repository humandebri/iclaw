// where: iclaw/canister/src/lib.rs
// what: ICP canister entrypoints, Candid export, and public API tests for iclaw v1
// why: the canister boundary must stay request/response-oriented and avoid websocket or native session management surfaces

mod auth;
mod context;
mod http;
mod memory;
mod provider;
mod service;
mod tools;
mod types;

pub use types::{
    AgentObservation, AgentObserveRequest, ApiError, ApiErrorCode, CanisterConfig, ChatRequest,
    ChatResponse, ContextConfig, ConversationSummaryGetRequest, HealthResponse, MemoryCategory,
    MemoryForgetRequest, MemoryGetRequest, MemoryItem, MemoryListRequest, MemoryRecallRequest,
    MemoryStoreRequest, ProviderConfig,
};

use service::{init_service, post_upgrade_service, with_service};

#[ic_cdk::init]
fn init(config: Option<CanisterConfig>) {
    auth::init_access(config.as_ref());
    init_service(config);
    http::init_assets();
}

#[ic_cdk::post_upgrade]
fn post_upgrade(config: Option<CanisterConfig>) {
    auth::init_access(config.as_ref());
    post_upgrade_service(config);
    http::init_assets();
}

#[ic_cdk::query]
async fn health() -> HealthResponse {
    with_service().health().await
}

#[ic_cdk::update]
async fn chat(request: ChatRequest) -> Result<ChatResponse, ApiError> {
    auth::ensure_allowed_caller()?;
    with_service().chat(request).await
}

#[ic_cdk::update]
async fn memory_store(request: MemoryStoreRequest) -> Result<(), ApiError> {
    auth::ensure_allowed_caller()?;
    with_service().memory_store(request).await
}

#[ic_cdk::query]
async fn memory_recall(request: MemoryRecallRequest) -> Result<Vec<MemoryItem>, ApiError> {
    auth::ensure_allowed_caller()?;
    with_service().memory_recall(request).await
}

#[ic_cdk::query]
async fn memory_get(request: MemoryGetRequest) -> Result<Option<MemoryItem>, ApiError> {
    auth::ensure_allowed_caller()?;
    with_service().memory_get(request).await
}

#[ic_cdk::query]
async fn memory_list(request: MemoryListRequest) -> Result<Vec<MemoryItem>, ApiError> {
    auth::ensure_allowed_caller()?;
    with_service().memory_list(request).await
}

#[ic_cdk::update]
async fn memory_forget(request: MemoryForgetRequest) -> Result<bool, ApiError> {
    auth::ensure_allowed_caller()?;
    with_service().memory_forget(request).await
}

#[ic_cdk::query]
async fn memory_count() -> Result<u64, ApiError> {
    auth::ensure_allowed_caller()?;
    with_service().memory_count().await
}

#[ic_cdk::query]
async fn conversation_summary_get(
    request: ConversationSummaryGetRequest,
) -> Result<Option<MemoryItem>, ApiError> {
    auth::ensure_allowed_caller()?;
    with_service().conversation_summary_get(request).await
}

#[ic_cdk::query]
async fn agent_observe(request: AgentObserveRequest) -> Result<AgentObservation, ApiError> {
    auth::ensure_allowed_caller()?;
    with_service().agent_observe(request).await
}

#[ic_cdk::query]
fn http_request(
    request: ic_http_certification::HttpRequest<'static>,
) -> ic_http_certification::HttpResponse<'static> {
    http::serve(request)
}

ic_cdk::export_candid!();

#[cfg(test)]
mod tests {
    use super::*;
    use candid::Principal;

    fn test_config() -> CanisterConfig {
        CanisterConfig {
            provider: None,
            context: None,
            allowed_principals: Some(vec![Principal::anonymous()]),
        }
    }

    fn normalize_candid(input: &str) -> String {
        input
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[tokio::test]
    async fn health_response_uses_fixed_runtime_shape() {
        init(Some(CanisterConfig {
            provider: Some(ProviderConfig {
                api_url: "https://api.openai.com/v1".to_string(),
                api_key: "openai-test-key".to_string(),
                default_model: "gpt-4o-mini".to_string(),
                timeout_secs: Some(30),
            }),
            context: None,
            allowed_principals: Some(vec![Principal::anonymous()]),
        }));
        let response = health().await;

        assert_eq!(response.runtime, "icp-canister");
        assert_eq!(response.status, "ok");
        assert!(response.provider_ready);
        assert!(response.memory_ready);
    }

    #[tokio::test]
    async fn chat_returns_not_supported_until_provider_is_connected() {
        init(Some(test_config()));
        let request = ChatRequest {
            prompt: "hello from test".to_string(),
            session_id: Some("session-a".to_string()),
            model: Some("test-model".to_string()),
            temperature: Some(0.2),
        };

        let error = chat(request)
            .await
            .expect_err("provider should be unavailable");
        assert_eq!(error.code, ApiErrorCode::NotSupported.as_str());
        assert!(error.message.contains("not configured"));
    }

    #[tokio::test]
    async fn memory_round_trip_works_through_canister_api() {
        init(Some(test_config()));
        let key = "memory-roundtrip-entry";
        let _ = memory_forget(MemoryForgetRequest {
            key: key.to_string(),
        })
        .await;

        memory_store(MemoryStoreRequest {
            key: key.to_string(),
            content: "hello ic".to_string(),
            category: MemoryCategory::Conversation,
            session_id: Some("session-a".to_string()),
        })
        .await
        .expect("memory_store should succeed");

        let item = memory_get(MemoryGetRequest {
            key: key.to_string(),
        })
        .await
        .expect("memory_get should succeed")
        .expect("stored item should exist");
        assert_eq!(item.content, "hello ic");

        let listed = memory_list(MemoryListRequest {
            category: Some(MemoryCategory::Conversation),
            session_id: Some("session-a".to_string()),
        })
        .await
        .expect("memory_list should succeed");
        assert_eq!(listed.len(), 1);

        let recalled = memory_recall(MemoryRecallRequest {
            query: "hello".to_string(),
            limit: 10,
            session_id: Some("session-a".to_string()),
        })
        .await
        .expect("memory_recall should succeed");
        assert_eq!(recalled.len(), 1);

        assert!(memory_count().await.expect("memory_count should succeed") >= 1);

        assert!(memory_forget(MemoryForgetRequest {
            key: key.to_string(),
        })
        .await
        .expect("memory_forget should succeed"));
    }

    #[tokio::test]
    async fn protected_apis_reject_unauthorized_callers() {
        init(Some(test_config()));
        auth::set_test_caller(Principal::management_canister());

        let error = memory_count()
            .await
            .expect_err("unauthorized caller should be rejected");
        assert_eq!(error.code, ApiErrorCode::Unauthorized.as_str());

        auth::clear_test_caller();
    }

    #[test]
    fn candid_file_matches_exported_interface() {
        let exported = normalize_candid(&__export_service());
        let checked_in = normalize_candid(include_str!("../iclaw_ic.did"));

        assert_eq!(exported, checked_in);
    }

    #[test]
    fn exported_interface_stays_request_response_only() {
        let exported = normalize_candid(&__export_service()).to_ascii_lowercase();

        assert!(!exported.contains("websocket"));
        assert!(!exported.contains("subscribe"));
        assert!(!exported.contains("stream"));
    }

    #[test]
    fn reserved_error_codes_are_stable() {
        assert_eq!(ApiErrorCode::InvalidArgument.as_str(), "invalid_argument");
        assert_eq!(ApiErrorCode::NotSupported.as_str(), "not_supported");
        assert_eq!(ApiErrorCode::ProviderError.as_str(), "provider_error");
        assert_eq!(ApiErrorCode::MemoryError.as_str(), "memory_error");
        assert_eq!(ApiErrorCode::Unauthorized.as_str(), "unauthorized");
        assert_eq!(ApiErrorCode::Internal.as_str(), "internal");
    }
}
