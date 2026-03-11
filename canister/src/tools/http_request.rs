//! where: standalone/canister/src/tools/http_request.rs
//! what: ICP-safe reserved http_request tool backed by the same canister transport policy as the provider
//! why: the agent loop needs a constrained HTTPS outcall surface without widening host permissions

use crate::provider::transport::{CanisterHttpTransport, HttpResponse, RawHttpRequest};
use crate::types::ProviderConfig;
use async_trait::async_trait;
use iclaw_standalone_core::tools::{Tool, ToolResult};
use std::sync::Arc;

const DEFAULT_MAX_RESPONSE_BYTES: usize = 16 * 1024;

#[async_trait]
pub trait HttpRequestExecutor: Send + Sync {
    async fn execute(&self, request: RawHttpRequest) -> anyhow::Result<HttpResponse>;
}

struct TransportExecutor {
    transport: CanisterHttpTransport,
}

#[async_trait]
impl HttpRequestExecutor for TransportExecutor {
    async fn execute(&self, request: RawHttpRequest) -> anyhow::Result<HttpResponse> {
        self.transport.request(request).await
    }
}

pub struct IcHttpRequestTool {
    executor: Option<Arc<dyn HttpRequestExecutor>>,
}

impl IcHttpRequestTool {
    pub fn new(config: Option<&ProviderConfig>) -> Self {
        let executor = config
            .filter(|provider| provider.is_configured())
            .and_then(|provider| CanisterHttpTransport::new(provider).ok())
            .map(|transport| {
                Arc::new(TransportExecutor { transport }) as Arc<dyn HttpRequestExecutor>
            });
        Self { executor }
    }

    #[cfg(test)]
    fn with_executor(executor: Arc<dyn HttpRequestExecutor>) -> Self {
        Self {
            executor: Some(executor),
        }
    }
}

#[async_trait]
impl Tool for IcHttpRequestTool {
    fn name(&self) -> &str {
        "http_request"
    }

    fn description(&self) -> &str {
        "Reserved ICP HTTPS outcall surface restricted to the configured provider host."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "required": ["url"],
            "properties": {
                "url": { "type": "string" },
                "method": { "type": "string", "enum": ["GET", "POST", "HEAD"] },
                "body": { "type": "string" },
                "max_response_bytes": { "type": "integer" }
            }
        })
    }

    async fn execute(&self, args: serde_json::Value) -> anyhow::Result<ToolResult> {
        let Some(executor) = self.executor.as_ref() else {
            return Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some(
                    "provider transport is not configured for reserved http_request".into(),
                ),
            });
        };
        let url = args
            .get("url")
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default()
            .trim()
            .to_string();
        if url.is_empty() {
            return Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some("url must not be empty".into()),
            });
        }
        let method = args
            .get("method")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("GET")
            .to_string();
        let body = args
            .get("body")
            .and_then(serde_json::Value::as_str)
            .map(|text| text.as_bytes().to_vec());
        let max_response_bytes = args
            .get("max_response_bytes")
            .and_then(serde_json::Value::as_u64)
            .map(|value| value as usize)
            .unwrap_or(DEFAULT_MAX_RESPONSE_BYTES);

        match executor
            .execute(RawHttpRequest {
                url,
                method,
                headers: Vec::new(),
                body,
                max_response_bytes,
            })
            .await
        {
            Ok(response) => Ok(ToolResult {
                success: response.status_code < 400,
                output: serde_json::json!({
                    "status_code": response.status_code,
                    "body": String::from_utf8_lossy(&response.body),
                })
                .to_string(),
                error: None,
            }),
            Err(error) => Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some(error.to_string()),
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use parking_lot::Mutex;

    struct MockExecutor {
        requests: Mutex<Vec<RawHttpRequest>>,
        response: Result<HttpResponse, String>,
    }

    #[async_trait]
    impl HttpRequestExecutor for MockExecutor {
        async fn execute(&self, request: RawHttpRequest) -> anyhow::Result<HttpResponse> {
            self.requests.lock().push(request);
            match &self.response {
                Ok(response) => Ok(response.clone()),
                Err(error) => anyhow::bail!(error.clone()),
            }
        }
    }

    #[tokio::test]
    async fn http_request_tool_executes_reserved_request() {
        let tool = IcHttpRequestTool::with_executor(Arc::new(MockExecutor {
            requests: Mutex::new(Vec::new()),
            response: Ok(HttpResponse {
                status_code: 200,
                body: b"ok".to_vec(),
            }),
        }));

        let result = tool
            .execute(serde_json::json!({
                "url": "https://api.openai.com/v1/models",
                "method": "GET"
            }))
            .await
            .expect("tool execution should succeed");

        assert!(result.success);
        assert!(result.output.contains("\"status_code\":200"));
    }
}
