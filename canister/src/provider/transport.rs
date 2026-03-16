//! where: iclaw/canister/src/provider/transport.rs
//! what: HTTPS outcall transport and host validation for the ICP provider
//! why: keep the provider surface compact while isolating canister-specific request/response transport details and excluding websocket-style session transport

use crate::types::ProviderConfig;
use async_trait::async_trait;
#[cfg(test)]
use candid::Principal;
#[cfg(target_arch = "wasm32")]
use canhttp::{http::HttpConversionLayer, Client, HttpsOutcallError};
#[cfg(any(test, target_arch = "wasm32"))]
use canhttp::{
    IsReplicatedRequestExtension, MaxResponseBytesRequestExtension,
    TransformContextRequestExtension,
};
#[cfg(any(test, target_arch = "wasm32"))]
use http::{Method, Request};
#[cfg(target_arch = "wasm32")]
use ic_cdk::management_canister::transform_context_from_query;
#[cfg(test)]
use ic_cdk::management_canister::{TransformContext, TransformFunc};
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
#[cfg(target_arch = "wasm32")]
use tower::{Service, ServiceBuilder};

const DEFAULT_BASE_URL: &str = "https://api.openai.com/v1";
const DEFAULT_TIMEOUT_SECS: u64 = 120;
#[cfg(any(test, target_arch = "wasm32"))]
const TRANSFORM_METHOD: &str = "iclaw_http_transform";

#[derive(Debug, Clone)]
pub(crate) struct HttpResponse {
    pub(crate) status_code: u16,
    pub(crate) body: Vec<u8>,
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub(crate) struct RawHttpRequest {
    pub(crate) url: String,
    pub(crate) method: String,
    pub(crate) headers: Vec<(String, String)>,
    pub(crate) body: Option<Vec<u8>>,
    pub(crate) max_response_bytes: usize,
}

#[async_trait(?Send)]
pub(crate) trait OutboundHttp: Send + Sync {
    async fn post_json(
        &self,
        url: &str,
        bearer_token: &str,
        body: Vec<u8>,
        max_response_bytes: usize,
    ) -> anyhow::Result<HttpResponse>;
}

#[derive(Debug)]
pub(crate) struct CanisterHttpTransport {
    allowed_host: String,
    timeout_secs: u64,
}

impl CanisterHttpTransport {
    pub(crate) fn new(config: &ProviderConfig) -> anyhow::Result<Self> {
        let base_url = normalize_base_url(&config.api_url)?;
        let allowed_host = parse_https_host(&base_url)?;
        if is_private_or_local_host(&allowed_host) {
            anyhow::bail!("Blocked local/private host: {allowed_host}");
        }

        Ok(Self {
            allowed_host,
            timeout_secs: config.timeout_secs.unwrap_or(DEFAULT_TIMEOUT_SECS),
        })
    }

    pub(crate) async fn request(&self, request: RawHttpRequest) -> anyhow::Result<HttpResponse> {
        request_raw(self, request).await
    }
}

#[cfg(target_arch = "wasm32")]
#[ic_cdk::query(name = "iclaw_http_transform")]
fn iclaw_http_transform(
    args: ic_cdk::management_canister::TransformArgs,
) -> ic_cdk::management_canister::HttpRequestResult {
    ic_cdk::management_canister::HttpRequestResult {
        status: args.response.status,
        headers: Vec::new(),
        body: args.response.body,
    }
}

#[cfg(target_arch = "wasm32")]
#[async_trait(?Send)]
impl OutboundHttp for CanisterHttpTransport {
    async fn post_json(
        &self,
        url: &str,
        bearer_token: &str,
        body: Vec<u8>,
        max_response_bytes: usize,
    ) -> anyhow::Result<HttpResponse> {
        self.request(RawHttpRequest {
            url: url.to_string(),
            method: "POST".to_string(),
            headers: vec![
                (
                    "Authorization".to_string(),
                    format!("Bearer {bearer_token}"),
                ),
                ("Content-Type".to_string(), "application/json".to_string()),
            ],
            body: Some(body),
            max_response_bytes,
        })
        .await
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[async_trait(?Send)]
impl OutboundHttp for CanisterHttpTransport {
    async fn post_json(
        &self,
        url: &str,
        bearer_token: &str,
        body: Vec<u8>,
        max_response_bytes: usize,
    ) -> anyhow::Result<HttpResponse> {
        self.request(RawHttpRequest {
            url: url.to_string(),
            method: "POST".to_string(),
            headers: vec![
                (
                    "Authorization".to_string(),
                    format!("Bearer {bearer_token}"),
                ),
                ("Content-Type".to_string(), "application/json".to_string()),
            ],
            body: Some(body),
            max_response_bytes,
        })
        .await
    }
}

#[cfg(target_arch = "wasm32")]
async fn request_raw(
    transport: &CanisterHttpTransport,
    request: RawHttpRequest,
) -> anyhow::Result<HttpResponse> {
    validate_outcall_url(&request.url, &transport.allowed_host)?;
    let _timeout_hint = transport.timeout_secs;
    let http_request = build_canhttp_request(&request)?;

    let mut service = ServiceBuilder::new()
        .layer(HttpConversionLayer)
        .service(Client::new_with_box_error());
    let response = service.call(http_request).await.map_err(|error| {
        if error.is_response_too_large() {
            anyhow::anyhow!(
                "Response exceeded configured size limit ({} bytes)",
                request.max_response_bytes
            )
        } else {
            anyhow::anyhow!(error.to_string())
        }
    })?;
    let status_code = response.status().as_u16();
    let body = response.into_body();
    if body.len() > request.max_response_bytes {
        anyhow::bail!(
            "Response exceeded configured size limit ({} bytes)",
            request.max_response_bytes
        );
    }

    Ok(HttpResponse { status_code, body })
}

#[cfg(any(test, target_arch = "wasm32"))]
fn build_canhttp_request(request: &RawHttpRequest) -> anyhow::Result<Request<Vec<u8>>> {
    let method = match request.method.trim().to_ascii_uppercase().as_str() {
        "GET" => Method::GET,
        "POST" => Method::POST,
        "HEAD" => Method::HEAD,
        other => anyhow::bail!("Unsupported HTTP method for ICP outcall tool: {other}"),
    };
    let mut builder = Request::builder()
        .method(method)
        .uri(&request.url)
        .max_response_bytes(u64::try_from(request.max_response_bytes)?)
        .replicated(false)
        .transform_context(iclaw_transform_context());
    for (name, value) in &request.headers {
        builder = builder.header(name, value);
    }
    builder
        .body(request.body.clone().unwrap_or_default())
        .map_err(Into::into)
}

#[cfg(target_arch = "wasm32")]
fn iclaw_transform_context() -> ic_cdk::management_canister::TransformContext {
    transform_context_from_query(TRANSFORM_METHOD.to_string(), Vec::new())
}

#[cfg(test)]
fn iclaw_transform_context() -> TransformContext {
    TransformContext {
        function: TransformFunc(candid::Func {
            principal: Principal::anonymous(),
            method: TRANSFORM_METHOD.to_string(),
        }),
        context: Vec::new(),
    }
}

#[cfg(not(target_arch = "wasm32"))]
async fn request_raw(
    transport: &CanisterHttpTransport,
    request: RawHttpRequest,
) -> anyhow::Result<HttpResponse> {
    validate_outcall_url(&request.url, &transport.allowed_host)?;
    let _timeout_hint = transport.timeout_secs;
    let _ = request;
    anyhow::bail!("ICP HTTP transport is only available on wasm32 ICP builds")
}

pub(crate) fn normalize_base_url(raw: &str) -> anyhow::Result<String> {
    let trimmed = raw.trim();
    let candidate = if trimmed.is_empty() {
        DEFAULT_BASE_URL
    } else {
        trimmed
    };
    let normalized = candidate.trim_end_matches('/').to_string();
    let _ = parse_https_host(&normalized)?;
    Ok(normalized)
}

fn validate_outcall_url(url: &str, allowed_host: &str) -> anyhow::Result<()> {
    let host = parse_https_host(url)?;
    if is_private_or_local_host(&host) {
        anyhow::bail!("Blocked local/private host: {host}");
    }
    if host != allowed_host {
        anyhow::bail!("Host '{host}' is not in the configured allowlist");
    }
    Ok(())
}

fn parse_https_host(url: &str) -> anyhow::Result<String> {
    let rest = url
        .strip_prefix("https://")
        .ok_or_else(|| anyhow::anyhow!("Only https:// URLs are allowed"))?;
    let authority = rest
        .split(['/', '?', '#'])
        .next()
        .ok_or_else(|| anyhow::anyhow!("Invalid URL"))?;
    if authority.is_empty() || authority.contains('@') || authority.starts_with('[') {
        anyhow::bail!("URL must include a valid host");
    }

    let host = authority
        .split(':')
        .next()
        .unwrap_or_default()
        .trim()
        .trim_end_matches('.')
        .to_ascii_lowercase();
    if host.is_empty() {
        anyhow::bail!("URL must include a valid host");
    }

    Ok(host)
}

fn is_private_or_local_host(host: &str) -> bool {
    let bare = host
        .strip_prefix('[')
        .and_then(|value| value.strip_suffix(']'))
        .unwrap_or(host);
    let has_local_tld = bare
        .rsplit('.')
        .next()
        .is_some_and(|segment| segment.eq_ignore_ascii_case("local"));
    if bare.eq_ignore_ascii_case("localhost") || bare.ends_with(".localhost") || has_local_tld {
        return true;
    }

    match bare.parse::<IpAddr>() {
        Ok(IpAddr::V4(ipv4)) => is_non_global_v4(ipv4),
        Ok(IpAddr::V6(ipv6)) => is_non_global_v6(ipv6),
        Err(_) => false,
    }
}

fn is_non_global_v4(ipv4: Ipv4Addr) -> bool {
    if ipv4.is_private()
        || ipv4.is_loopback()
        || ipv4.is_link_local()
        || ipv4.is_broadcast()
        || ipv4.is_documentation()
        || ipv4.is_unspecified()
        || ipv4.is_multicast()
    {
        return true;
    }

    let octets = ipv4.octets();
    octets[0] >= 240
        || (octets[0] == 100 && (octets[1] & 0b1100_0000) == 64)
        || (octets[0] == 198 && (octets[1] == 18 || octets[1] == 19))
}

fn is_non_global_v6(ipv6: Ipv6Addr) -> bool {
    let segments = ipv6.segments();
    ipv6.is_loopback()
        || ipv6.is_unspecified()
        || ipv6.is_multicast()
        || (segments[0] & 0xfe00) == 0xfc00
        || (segments[0] & 0xffc0) == 0xfe80
        || (segments[0] == 0x2001 && segments[1] == 0x0db8)
        || ipv6.to_ipv4_mapped().is_some_and(is_non_global_v4)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canhttp_request_keeps_non_replicated_flag() {
        let request = RawHttpRequest {
            url: "https://api.openai.com/v1/chat/completions".to_string(),
            method: "POST".to_string(),
            headers: vec![("content-type".to_string(), "application/json".to_string())],
            body: Some(br#"{"hello":"world"}"#.to_vec()),
            max_response_bytes: 32_000,
        };

        let built = build_canhttp_request(&request).expect("request should build");

        assert_eq!(built.get_is_replicated(), Some(false));
        assert_eq!(built.get_max_response_bytes(), Some(32_000));
    }
}
