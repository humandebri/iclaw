//! where: iclaw/core/src/providers/traits.rs | what: provider trait definition | why: runtime crates need one chat/provider contract without assuming websocket sessions, gateway wiring, or other native composition details

use super::instructions::build_tool_instructions_text;
use super::messages::{ChatMessage, ChatRequest, ChatResponse, ProviderCapabilities, ToolsPayload};
use super::streaming::{StreamChunk, StreamOptions, StreamResult};
use crate::tools::ToolSpec;
use async_trait::async_trait;
use futures_util::{stream, StreamExt};

#[async_trait]
pub trait Provider: Send + Sync {
    fn capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities::default()
    }

    fn convert_tools(&self, tools: &[ToolSpec]) -> ToolsPayload {
        ToolsPayload::PromptGuided {
            instructions: build_tool_instructions_text(tools),
        }
    }

    async fn simple_chat(
        &self,
        message: &str,
        model: &str,
        temperature: f64,
    ) -> anyhow::Result<String> {
        self.chat_with_system(None, message, model, temperature)
            .await
    }

    async fn chat_with_system(
        &self,
        system_prompt: Option<&str>,
        message: &str,
        model: &str,
        temperature: f64,
    ) -> anyhow::Result<String>;

    async fn chat_with_history(
        &self,
        messages: &[ChatMessage],
        model: &str,
        temperature: f64,
    ) -> anyhow::Result<String> {
        let system = messages
            .iter()
            .find(|m| m.role == "system")
            .map(|m| m.content.as_str());
        let last_user = messages
            .iter()
            .rfind(|m| m.role == "user")
            .map(|m| m.content.as_str())
            .unwrap_or("");
        self.chat_with_system(system, last_user, model, temperature)
            .await
    }

    async fn chat(
        &self,
        request: ChatRequest<'_>,
        model: &str,
        temperature: f64,
    ) -> anyhow::Result<ChatResponse> {
        if let Some(tools) = request.tools {
            if !tools.is_empty() && !self.supports_native_tools() {
                let tool_instructions = match self.convert_tools(tools) {
                    ToolsPayload::PromptGuided { instructions } => instructions,
                    payload => anyhow::bail!(
                        "Provider returned non-prompt-guided tools payload ({payload:?}) while supports_native_tools() is false"
                    ),
                };
                let mut modified_messages = request.messages.to_vec();
                if let Some(system_message) =
                    modified_messages.iter_mut().find(|m| m.role == "system")
                {
                    if !system_message.content.is_empty() {
                        system_message.content.push_str("\n\n");
                    }
                    system_message.content.push_str(&tool_instructions);
                } else {
                    modified_messages.insert(0, ChatMessage::system(tool_instructions));
                }

                return Ok(ChatResponse {
                    text: Some(
                        self.chat_with_history(&modified_messages, model, temperature)
                            .await?,
                    ),
                    tool_calls: Vec::new(),
                    usage: None,
                    reasoning_content: None,
                });
            }
        }

        Ok(ChatResponse {
            text: Some(
                self.chat_with_history(request.messages, model, temperature)
                    .await?,
            ),
            tool_calls: Vec::new(),
            usage: None,
            reasoning_content: None,
        })
    }

    fn supports_native_tools(&self) -> bool {
        self.capabilities().native_tool_calling
    }

    fn supports_vision(&self) -> bool {
        self.capabilities().vision
    }

    async fn warmup(&self) -> anyhow::Result<()> {
        Ok(())
    }

    async fn chat_with_tools(
        &self,
        messages: &[ChatMessage],
        _tools: &[serde_json::Value],
        model: &str,
        temperature: f64,
    ) -> anyhow::Result<ChatResponse> {
        Ok(ChatResponse {
            text: Some(self.chat_with_history(messages, model, temperature).await?),
            tool_calls: Vec::new(),
            usage: None,
            reasoning_content: None,
        })
    }

    fn supports_streaming(&self) -> bool {
        false
    }

    fn stream_chat_with_system(
        &self,
        _system_prompt: Option<&str>,
        _message: &str,
        _model: &str,
        _temperature: f64,
        _options: StreamOptions,
    ) -> stream::BoxStream<'static, StreamResult<StreamChunk>> {
        stream::empty().boxed()
    }

    fn stream_chat_with_history(
        &self,
        _messages: &[ChatMessage],
        _model: &str,
        _temperature: f64,
        _options: StreamOptions,
    ) -> stream::BoxStream<'static, StreamResult<StreamChunk>> {
        stream::once(async { Ok(StreamChunk::error("provider does not support streaming")) })
            .boxed()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct EchoProvider;

    #[async_trait]
    impl Provider for EchoProvider {
        async fn chat_with_system(
            &self,
            system_prompt: Option<&str>,
            message: &str,
            _model: &str,
            _temperature: f64,
        ) -> anyhow::Result<String> {
            Ok(format!("{}::{message}", system_prompt.unwrap_or_default()))
        }
    }

    #[tokio::test]
    async fn provider_chat_falls_back_to_history() {
        let provider = EchoProvider;
        let messages = [ChatMessage::system("sys"), ChatMessage::user("hello")];
        let response = provider
            .chat(
                ChatRequest {
                    messages: &messages,
                    tools: None,
                },
                "model",
                0.0,
            )
            .await
            .unwrap();
        assert_eq!(response.text_or_empty(), "sys::hello");
    }
}
