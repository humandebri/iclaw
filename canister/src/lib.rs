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
    Agent, AgentCreateRequest, AgentDraft, AgentGetRequest, AgentObservation, AgentObserveRequest,
    AgentUpdateRequest, AllowedPrincipalsResponse, ApiError, ApiErrorCode, CanisterConfig,
    ContextConfig, ConversationSummaryGetRequest, HealthResponse, MemoryCategory,
    MemoryForgetRequest, MemoryGetRequest, MemoryItem, MemoryListRequest, MemoryRecallRequest,
    MemoryStoreRequest, ProviderConfig, Run, RunCancelRequest, RunCreateRequest, RunEvent,
    RunEventsGetRequest, RunGetRequest, RunListRequest, RunResumeRequest, Schedule,
    ScheduleCreateRequest, ScheduleDraft, ScheduleGetRequest, ScheduleUpdateRequest, Session,
    SessionGetRequest, ToolPolicy, ToolPolicyListRequest, ToolPolicyUpdateRequest, Webhook,
    WebhookCreateRequest, WebhookDraft, WebhookGetRequest, WebhookInvokeRequest, WebhookRejection,
    WebhookRejectionsListRequest, WebhookSecretRotateRequest, WebhookSecretRotateResponse,
    WebhookUpdateRequest,
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

#[ic_cdk::query]
fn allowed_principals_get() -> Result<AllowedPrincipalsResponse, ApiError> {
    auth::allowed_principals_get()
}

#[ic_cdk::update]
fn allowed_principals_set(
    request: AllowedPrincipalsResponse,
) -> Result<AllowedPrincipalsResponse, ApiError> {
    auth::allowed_principals_set(request)
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
async fn agents_list() -> Result<Vec<Agent>, ApiError> {
    auth::ensure_allowed_caller()?;
    with_service().agents_list().await
}

#[ic_cdk::query]
async fn agent_get(request: AgentGetRequest) -> Result<Option<Agent>, ApiError> {
    auth::ensure_allowed_caller()?;
    with_service().agent_get(request).await
}

#[ic_cdk::update]
async fn agent_create(request: AgentCreateRequest) -> Result<Agent, ApiError> {
    auth::ensure_allowed_caller()?;
    with_service().agent_create(request).await
}

#[ic_cdk::update]
async fn agent_update(request: AgentUpdateRequest) -> Result<Agent, ApiError> {
    auth::ensure_allowed_caller()?;
    with_service().agent_update(request).await
}

#[ic_cdk::query]
async fn tool_policy_list(request: ToolPolicyListRequest) -> Result<Vec<ToolPolicy>, ApiError> {
    auth::ensure_allowed_caller()?;
    with_service().tool_policy_list(request).await
}

#[ic_cdk::update]
async fn tool_policy_update(request: ToolPolicyUpdateRequest) -> Result<ToolPolicy, ApiError> {
    auth::ensure_allowed_caller()?;
    with_service().tool_policy_update(request).await
}

#[ic_cdk::query]
async fn sessions_list(agent_id: Option<String>) -> Result<Vec<Session>, ApiError> {
    auth::ensure_allowed_caller()?;
    with_service().sessions_list(agent_id).await
}

#[ic_cdk::query]
async fn schedules_list() -> Result<Vec<Schedule>, ApiError> {
    auth::ensure_allowed_caller()?;
    with_service().schedules_list().await
}

#[ic_cdk::query]
async fn schedule_get(request: ScheduleGetRequest) -> Result<Option<Schedule>, ApiError> {
    auth::ensure_allowed_caller()?;
    with_service().schedule_get(request).await
}

#[ic_cdk::update]
async fn schedule_create(request: ScheduleCreateRequest) -> Result<Schedule, ApiError> {
    auth::ensure_allowed_caller()?;
    with_service().schedule_create(request).await
}

#[ic_cdk::update]
async fn schedule_update(request: ScheduleUpdateRequest) -> Result<Schedule, ApiError> {
    auth::ensure_allowed_caller()?;
    with_service().schedule_update(request).await
}

#[ic_cdk::update]
async fn schedule_delete(request: ScheduleGetRequest) -> Result<bool, ApiError> {
    auth::ensure_allowed_caller()?;
    with_service().schedule_delete(request).await
}

#[ic_cdk::update]
async fn schedule_trigger(request: ScheduleGetRequest) -> Result<Run, ApiError> {
    auth::ensure_allowed_caller()?;
    with_service().schedule_trigger(request).await
}

#[ic_cdk::query]
async fn webhooks_list() -> Result<Vec<Webhook>, ApiError> {
    auth::ensure_allowed_caller()?;
    with_service().webhooks_list().await
}

#[ic_cdk::query]
async fn webhook_get(request: WebhookGetRequest) -> Result<Option<Webhook>, ApiError> {
    auth::ensure_allowed_caller()?;
    with_service().webhook_get(request).await
}

#[ic_cdk::update]
async fn webhook_create(request: WebhookCreateRequest) -> Result<Webhook, ApiError> {
    auth::ensure_allowed_caller()?;
    with_service().webhook_create(request).await
}

#[ic_cdk::update]
async fn webhook_update(request: WebhookUpdateRequest) -> Result<Webhook, ApiError> {
    auth::ensure_allowed_caller()?;
    with_service().webhook_update(request).await
}

#[ic_cdk::update]
async fn webhook_delete(request: WebhookGetRequest) -> Result<bool, ApiError> {
    auth::ensure_allowed_caller()?;
    with_service().webhook_delete(request).await
}

#[ic_cdk::query]
async fn webhook_rejections_list(
    request: WebhookRejectionsListRequest,
) -> Result<Vec<WebhookRejection>, ApiError> {
    auth::ensure_allowed_caller()?;
    with_service().webhook_rejections_list(request).await
}

#[ic_cdk::update]
async fn webhook_invoke(request: WebhookInvokeRequest) -> Result<Run, ApiError> {
    with_service().webhook_invoke(request).await
}

#[ic_cdk::query]
async fn session_get(request: SessionGetRequest) -> Result<Option<Session>, ApiError> {
    auth::ensure_allowed_caller()?;
    with_service().session_get(request).await
}

#[ic_cdk::update]
async fn run_create(request: RunCreateRequest) -> Result<Run, ApiError> {
    auth::ensure_allowed_caller()?;
    with_service().run_create(request).await
}

#[ic_cdk::query]
async fn run_get(request: RunGetRequest) -> Result<Option<Run>, ApiError> {
    auth::ensure_allowed_caller()?;
    with_service().run_get(request).await
}

#[ic_cdk::query]
async fn run_list(request: RunListRequest) -> Result<Vec<Run>, ApiError> {
    auth::ensure_allowed_caller()?;
    with_service().run_list(request).await
}

#[ic_cdk::query]
async fn run_events_get(request: RunEventsGetRequest) -> Result<Vec<RunEvent>, ApiError> {
    auth::ensure_allowed_caller()?;
    with_service().run_events_get(request).await
}

#[ic_cdk::update]
async fn run_cancel(request: RunCancelRequest) -> Result<bool, ApiError> {
    auth::ensure_allowed_caller()?;
    with_service().run_cancel(request).await
}

#[ic_cdk::update]
async fn run_resume(request: RunResumeRequest) -> Result<Run, ApiError> {
    auth::ensure_allowed_caller()?;
    with_service().run_resume(request).await
}

#[ic_cdk::update]
async fn webhook_rotate_secret(
    request: WebhookSecretRotateRequest,
) -> Result<WebhookSecretRotateResponse, ApiError> {
    auth::ensure_allowed_caller()?;
    with_service().webhook_rotate_secret(request).await
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
            .flat_map(str::split_whitespace)
            .collect::<Vec<_>>()
            .join(" ")
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
    fn allowlist_apis_round_trip_and_protect_caller_membership() {
        init(Some(test_config()));
        let updated = allowed_principals_set(AllowedPrincipalsResponse {
            allowed_principals: vec![Principal::anonymous(), Principal::management_canister()],
        })
        .expect("allowlist update should succeed");
        assert_eq!(updated.allowed_principals.len(), 2);
        let fetched = allowed_principals_get().expect("allowlist get should succeed");
        assert_eq!(fetched.allowed_principals, updated.allowed_principals);
    }

    #[test]
    fn post_upgrade_prefers_persisted_allowlist_over_init_args() {
        init(Some(test_config()));
        allowed_principals_set(AllowedPrincipalsResponse {
            allowed_principals: vec![Principal::anonymous(), Principal::management_canister()],
        })
        .expect("allowlist update should succeed");
        post_upgrade(Some(test_config()));
        let fetched = allowed_principals_get().expect("allowlist get should succeed");
        assert_eq!(fetched.allowed_principals.len(), 2);
        assert!(fetched
            .allowed_principals
            .iter()
            .any(|principal| principal == &Principal::management_canister()));
        auth::clear_test_persisted_allowlist();
    }

    #[test]
    fn candid_file_matches_exported_interface() {
        let exported = normalize_candid(&__export_service());
        let checked_in = normalize_candid(include_str!("../iclaw_ic.did"));

        for marker in [
            "webhook_rejections_list",
            "type WebhookRejection",
            "type WebhookRejectionsListRequest",
            "type WebhookUpdateRequest = record { webhook : Webhook; secret_override : opt text; };",
            "schedule_trigger",
            "type Schedule",
            "type ScheduleCreateRequest",
        ] {
            assert!(exported.contains(marker), "exported interface missing marker: {marker}");
            assert!(
                checked_in.contains(marker),
                "checked-in did missing marker: {marker}"
            );
        }
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
