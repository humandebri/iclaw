// where: iclaw/core/src/providers/mod.rs
// what: provider-facing DTOs and traits for the iclaw canister workspace
// why: the canister keeps the existing provider contract shape while dropping root runtime dependencies

mod instructions;
mod messages;
mod streaming;
mod traits;

pub use instructions::build_tool_instructions_text;
pub use messages::{
    ChatMessage, ChatRequest, ChatResponse, ConversationMessage, ProviderCapabilities,
    ProviderCapabilityError, TokenUsage, ToolCall, ToolResultMessage, ToolsPayload,
};
pub use streaming::{StreamChunk, StreamError, StreamOptions, StreamResult};
pub use traits::Provider;
