// where: iclaw/canister/src/types.rs
// what: Candid-facing DTOs and stable API error codes for the ICP canister
// why: Track D must freeze the wire contract without exposing native internal types

use candid::{CandidType, Deserialize, Principal};
use iclaw_core::memory::{MemoryCategory as CoreMemoryCategory, MemoryEntry};

#[derive(CandidType, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct ApiError {
    pub code: String,
    pub message: String,
}

#[derive(CandidType, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct ProviderConfig {
    pub api_url: String,
    pub api_key: String,
    pub default_model: String,
    pub timeout_secs: Option<u64>,
}

impl ProviderConfig {
    pub fn is_configured(&self) -> bool {
        !self.api_url.trim().is_empty()
            && !self.api_key.trim().is_empty()
            && !self.default_model.trim().is_empty()
    }
}

#[derive(CandidType, Deserialize, Clone, Debug, PartialEq)]
pub struct ContextConfig {
    pub workspace_files: Option<Vec<String>>,
    pub skills_dir: Option<String>,
    pub max_static_context_chars: Option<u64>,
    pub max_skill_context_chars: Option<u64>,
    pub memory_recall_limit: Option<u64>,
    pub memory_min_score: Option<f64>,
    pub enable_lightweight_skill_actions: Option<bool>,
    pub history_limit: Option<u64>,
    pub max_tool_iterations: Option<u64>,
    pub enable_autosave: Option<bool>,
    pub enable_tool_loop: Option<bool>,
    pub enable_auto_promote: Option<bool>,
    pub enable_conversation_summary: Option<bool>,
    pub summary_max_chars: Option<u64>,
    pub retry_provider_once: Option<bool>,
    pub cycle_balance_warning_threshold: Option<u64>,
    pub max_prompt_chars: Option<u64>,
    pub max_request_bytes_budget: Option<u64>,
    pub llm_summary_on_overflow: Option<bool>,
    pub llm_summary_model: Option<String>,
    pub llm_summary_max_chars: Option<u64>,
    pub llm_summary_request_bytes_threshold: Option<u64>,
}

#[derive(CandidType, Deserialize, Clone, Debug, PartialEq)]
pub struct CanisterConfig {
    pub provider: Option<ProviderConfig>,
    pub context: Option<ContextConfig>,
    pub allowed_principals: Option<Vec<Principal>>,
}

#[derive(CandidType, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct AllowedPrincipalsResponse {
    pub allowed_principals: Vec<Principal>,
}

impl ApiError {
    pub fn new(code: ApiErrorCode, message: impl Into<String>) -> Self {
        Self {
            code: code.as_str().to_string(),
            message: message.into(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ApiErrorCode {
    InvalidArgument,
    NotSupported,
    ProviderError,
    MemoryError,
    Unauthorized,
    Internal,
}

impl ApiErrorCode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::InvalidArgument => "invalid_argument",
            Self::NotSupported => "not_supported",
            Self::ProviderError => "provider_error",
            Self::MemoryError => "memory_error",
            Self::Unauthorized => "unauthorized",
            Self::Internal => "internal",
        }
    }
}

#[derive(CandidType, Deserialize, Clone, Debug, PartialEq)]
pub struct ChatRequest {
    pub prompt: String,
    pub session_id: Option<String>,
    pub model: Option<String>,
    pub temperature: Option<f64>,
}

#[derive(CandidType, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct ChatResponse {
    pub response: String,
    pub session_id: Option<String>,
    pub model: Option<String>,
    pub provider_ready: bool,
    pub memory_ready: bool,
}

#[derive(CandidType, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
    pub runtime: String,
    pub provider_ready: bool,
    pub memory_ready: bool,
}

impl HealthResponse {
    pub fn degraded(provider_ready: bool, memory_ready: bool) -> Self {
        Self {
            status: if provider_ready && memory_ready {
                "ok".to_string()
            } else {
                "degraded".to_string()
            },
            version: env!("CARGO_PKG_VERSION").to_string(),
            runtime: "icp-canister".to_string(),
            provider_ready,
            memory_ready,
        }
    }
}

#[derive(CandidType, Deserialize, Clone, Debug, PartialEq, Eq)]
pub enum MemoryCategory {
    #[serde(rename = "core")]
    Core,
    #[serde(rename = "daily")]
    Daily,
    #[serde(rename = "conversation")]
    Conversation,
    #[serde(rename = "custom")]
    Custom(String),
}

impl From<MemoryCategory> for CoreMemoryCategory {
    fn from(value: MemoryCategory) -> Self {
        match value {
            MemoryCategory::Core => Self::Core,
            MemoryCategory::Daily => Self::Daily,
            MemoryCategory::Conversation => Self::Conversation,
            MemoryCategory::Custom(name) => Self::Custom(name),
        }
    }
}

impl From<&MemoryCategory> for CoreMemoryCategory {
    fn from(value: &MemoryCategory) -> Self {
        match value {
            MemoryCategory::Core => Self::Core,
            MemoryCategory::Daily => Self::Daily,
            MemoryCategory::Conversation => Self::Conversation,
            MemoryCategory::Custom(name) => Self::Custom(name.clone()),
        }
    }
}

impl From<CoreMemoryCategory> for MemoryCategory {
    fn from(value: CoreMemoryCategory) -> Self {
        match value {
            CoreMemoryCategory::Core => Self::Core,
            CoreMemoryCategory::Daily => Self::Daily,
            CoreMemoryCategory::Conversation => Self::Conversation,
            CoreMemoryCategory::Custom(name) => Self::Custom(name),
        }
    }
}

#[derive(CandidType, Deserialize, Clone, Debug, PartialEq)]
pub struct MemoryItem {
    pub id: String,
    pub key: String,
    pub content: String,
    pub category: MemoryCategory,
    pub timestamp: String,
    pub session_id: Option<String>,
    pub score: Option<f64>,
}

impl From<MemoryEntry> for MemoryItem {
    fn from(entry: MemoryEntry) -> Self {
        Self {
            id: entry.id,
            key: entry.key,
            content: entry.content,
            category: entry.category.into(),
            timestamp: entry.timestamp,
            session_id: entry.session_id,
            score: entry.score,
        }
    }
}

#[derive(CandidType, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct MemoryStoreRequest {
    pub key: String,
    pub content: String,
    pub category: MemoryCategory,
    pub session_id: Option<String>,
}

#[derive(CandidType, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct MemoryRecallRequest {
    pub query: String,
    pub limit: u64,
    pub session_id: Option<String>,
}

#[derive(CandidType, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct MemoryGetRequest {
    pub key: String,
}

#[derive(CandidType, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct MemoryListRequest {
    pub category: Option<MemoryCategory>,
    pub session_id: Option<String>,
}

#[derive(CandidType, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct MemoryForgetRequest {
    pub key: String,
}

#[derive(CandidType, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct ConversationSummaryGetRequest {
    pub session_id: String,
}

#[derive(CandidType, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct AgentObserveRequest {
    pub session_id: Option<String>,
}

#[derive(CandidType, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct AgentObservation {
    pub workspace_keys: Vec<String>,
    pub core_keys: Vec<String>,
    pub conversation_summary_key: Option<String>,
    pub conversation_summary_present: bool,
    pub conversation_turn_count: u64,
    pub auto_promoted_keys: Vec<String>,
    pub history_limit: u64,
    pub enable_auto_promote: bool,
    pub enable_conversation_summary: bool,
    pub tool_loop_enabled: bool,
    pub max_tool_iterations: u64,
}

pub type UnitResult = Result<(), ApiError>;
pub type MemoryForgetResult = Result<bool, ApiError>;
pub type MemoryCountResult = Result<u64, ApiError>;
