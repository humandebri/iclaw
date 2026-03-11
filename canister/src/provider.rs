//! where: iclaw/canister/src/provider.rs
//! what: ICP-safe OpenAI-compatible provider with message-history and native tool-call support
//! why: the canister agent loop needs one HTTPS-outcall-backed provider surface without native runtime dependencies

mod messages;
pub(crate) mod transport;

use crate::provider::messages::{
    convert_messages, convert_tools, parse_chat_response, simple_messages, OpenAiChatRequest,
    OpenAiChatResponse,
};
use crate::types::ProviderConfig;
use async_trait::async_trait;
use iclaw_core::providers::{
    ChatRequest as ProviderChatRequest, ChatResponse as ProviderChatResponse, ConversationMessage,
    Provider, ProviderCapabilities,
};
use iclaw_core::tools::ToolSpec;
use std::sync::Arc;
#[cfg(test)]
use transport::HttpResponse;
use transport::{normalize_base_url, CanisterHttpTransport, OutboundHttp};

pub(crate) const MAX_REQUEST_BYTES: usize = 256 * 1024;
const MAX_RESPONSE_BYTES: usize = 1_000_000;

pub fn build_provider(
    config: Option<&ProviderConfig>,
) -> anyhow::Result<Option<Arc<dyn IcCanisterProvider>>> {
    let Some(provider) = config.filter(|provider| provider.is_configured()) else {
        return Ok(None);
    };

    Ok(Some(Arc::new(IcOpenAiProvider::from_config(
        provider,
        Arc::new(CanisterHttpTransport::new(provider)?),
    )?)))
}

#[derive(Clone, Debug)]
pub struct ProviderChatResult {
    pub response: ProviderChatResponse,
    pub model: Option<String>,
}

#[async_trait]
pub trait IcCanisterProvider: Send + Sync {
    async fn chat(
        &self,
        messages: &[ConversationMessage],
        tools: Option<&[ToolSpec]>,
        model: &str,
        temperature: f64,
    ) -> anyhow::Result<ProviderChatResult>;
    fn capabilities(&self) -> ProviderCapabilities;
}

pub struct IcOpenAiProvider {
    base_url: String,
    api_key: String,
    transport: Arc<dyn OutboundHttp>,
}

impl IcOpenAiProvider {
    fn from_config(
        config: &ProviderConfig,
        transport: Arc<dyn OutboundHttp>,
    ) -> anyhow::Result<Self> {
        Ok(Self {
            base_url: normalize_base_url(&config.api_url)?,
            api_key: config.api_key.clone(),
            transport,
        })
    }

    #[cfg(test)]
    fn with_transport(
        config: &ProviderConfig,
        transport: Arc<dyn OutboundHttp>,
    ) -> anyhow::Result<Self> {
        Self::from_config(config, transport)
    }

    async fn perform_chat(
        &self,
        messages: &[ConversationMessage],
        tools: Option<&[ToolSpec]>,
        model: &str,
        temperature: f64,
    ) -> anyhow::Result<ProviderChatResult> {
        let request = OpenAiChatRequest {
            model: model.to_string(),
            messages: convert_messages(messages),
            temperature,
            tool_choice: tools.map(|_| "auto".to_string()),
            tools: convert_tools(tools),
        };
        let body = serde_json::to_vec(&request)?;
        if body.len() > MAX_REQUEST_BYTES {
            anyhow::bail!(
                "Request body exceeded configured size limit ({MAX_REQUEST_BYTES} bytes)"
            );
        }

        let response = self
            .transport
            .post_json(
                &format!("{}/chat/completions", self.base_url),
                &self.api_key,
                body,
                MAX_RESPONSE_BYTES,
            )
            .await?;
        if response.status_code >= 400 {
            anyhow::bail!(
                "OpenAI-compatible API error ({}): {}",
                response.status_code,
                sanitize_api_error(&String::from_utf8_lossy(&response.body))
            );
        }

        let decoded: OpenAiChatResponse = serde_json::from_slice(&response.body)?;
        let provider_response = decoded
            .choices
            .into_iter()
            .next()
            .map(|choice| parse_chat_response(choice.message))
            .ok_or_else(|| anyhow::anyhow!("No response from OpenAI-compatible provider"))?;

        Ok(ProviderChatResult {
            response: provider_response,
            model: decoded.model,
        })
    }
}

#[async_trait]
impl IcCanisterProvider for IcOpenAiProvider {
    async fn chat(
        &self,
        messages: &[ConversationMessage],
        tools: Option<&[ToolSpec]>,
        model: &str,
        temperature: f64,
    ) -> anyhow::Result<ProviderChatResult> {
        self.perform_chat(messages, tools, model, temperature).await
    }

    fn capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities {
            native_tool_calling: true,
            vision: false,
        }
    }
}

#[async_trait]
impl Provider for IcOpenAiProvider {
    fn capabilities(&self) -> ProviderCapabilities {
        IcCanisterProvider::capabilities(self)
    }

    async fn chat_with_system(
        &self,
        system_prompt: Option<&str>,
        message: &str,
        model: &str,
        temperature: f64,
    ) -> anyhow::Result<String> {
        self.perform_chat(
            &simple_messages(system_prompt, message),
            None,
            model,
            temperature,
        )
        .await
        .map(|result| result.response.text_or_empty().to_string())
    }

    async fn chat(
        &self,
        request: ProviderChatRequest<'_>,
        model: &str,
        temperature: f64,
    ) -> anyhow::Result<ProviderChatResponse> {
        let history = request
            .messages
            .iter()
            .cloned()
            .map(ConversationMessage::Chat)
            .collect::<Vec<_>>();
        self.perform_chat(&history, request.tools, model, temperature)
            .await
            .map(|result| result.response)
    }
}

fn sanitize_api_error(raw: &str) -> String {
    let compact = raw.split_whitespace().collect::<Vec<_>>().join(" ");
    if compact.is_empty() {
        "upstream provider returned an empty error body".to_string()
    } else {
        compact
    }
}

#[cfg(test)]
mod tests;
