//! where: iclaw/canister/src/service/webhooks.rs
//! what: Durable webhook records for automation entry
//! why: keep webhook persistence and shared validation separate from run execution orchestration

use crate::types::{
    Webhook, WebhookCreateRequest, WebhookDraft, WebhookGetRequest, WebhookInvokeRequest,
    WebhookRejection, WebhookRejectionsListRequest, WebhookSecretRotateResponse,
    WebhookUpdateRequest,
};
use chrono::Utc;
use iclaw_core::memory::{Memory, MemoryCategory};
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

const WEBHOOKS_PREFIX: &str = "webhooks/";
const WEBHOOK_REJECTIONS_PREFIX: &str = "webhook_rejections/";
const WEBHOOK_REJECTION_RETENTION_LIMIT: usize = 100;
static SECRET_SEQUENCE: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Debug, Serialize, Deserialize)]
struct StoredWebhook {
    id: String,
    name: String,
    agent_id: String,
    session_mode: String,
    fixed_session_id: Option<String>,
    secret: String,
    enabled: bool,
    created_at: String,
    updated_at: String,
    last_run_id: Option<String>,
    last_secret_rotated_at: Option<String>,
    last_invoked_at: Option<String>,
    last_rejection_at: Option<String>,
    last_rejection_reason: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct StoredWebhookRejection {
    id: String,
    webhook_id: String,
    reason: String,
    timestamp: String,
}

pub(crate) async fn list_webhooks(
    memory: Option<&Arc<dyn Memory>>,
) -> anyhow::Result<Vec<Webhook>> {
    let Some(memory) = memory else {
        return Ok(Vec::new());
    };
    let mut webhooks = Vec::new();
    for entry in memory.list(Some(&MemoryCategory::Core), None).await? {
        if !entry.key.starts_with(WEBHOOKS_PREFIX) {
            continue;
        }
        let payload: StoredWebhook = serde_json::from_str(&entry.content)?;
        webhooks.push(payload.into_public_webhook());
    }
    webhooks.sort_by(|left, right| left.id.cmp(&right.id));
    Ok(webhooks)
}

pub(crate) async fn get_webhook(
    memory: Option<&Arc<dyn Memory>>,
    request: &WebhookGetRequest,
) -> anyhow::Result<Option<Webhook>> {
    let Some(memory) = memory else {
        return Ok(None);
    };
    let Some(entry) = memory.get(&webhook_key(&request.webhook_id)).await? else {
        return Ok(None);
    };
    let payload: StoredWebhook = serde_json::from_str(&entry.content)?;
    Ok(Some(payload.into_public_webhook()))
}

pub(crate) async fn load_webhook(
    memory: Option<&Arc<dyn Memory>>,
    webhook_id: &str,
) -> anyhow::Result<Option<Webhook>> {
    let Some(memory) = memory else {
        return Ok(None);
    };
    let Some(entry) = memory.get(&webhook_key(webhook_id)).await? else {
        return Ok(None);
    };
    let payload: StoredWebhook = serde_json::from_str(&entry.content)?;
    Ok(Some(payload.into_webhook()))
}

pub(crate) async fn list_rejections(
    memory: Option<&Arc<dyn Memory>>,
    request: &WebhookRejectionsListRequest,
) -> anyhow::Result<Vec<WebhookRejection>> {
    let Some(memory) = memory else {
        return Ok(Vec::new());
    };
    let mut rejections = load_rejections(memory, &request.webhook_id)
        .await?
        .into_iter()
        .map(|(_, payload)| WebhookRejection {
            id: payload.id,
            webhook_id: payload.webhook_id,
            reason: payload.reason,
            timestamp: payload.timestamp,
        })
        .collect::<Vec<_>>();
    let limit = request.limit.unwrap_or(10) as usize;
    rejections.truncate(limit);
    Ok(rejections)
}

pub(crate) async fn create_webhook(
    memory: &Arc<dyn Memory>,
    request: WebhookCreateRequest,
) -> anyhow::Result<Webhook> {
    if load_webhook(Some(memory), &request.draft.id).await?.is_some() {
        anyhow::bail!("webhook already exists");
    }
    let webhook = Webhook::from(request.draft);
    store_webhook(memory, &webhook).await?;
    Ok(webhook)
}

pub(crate) async fn update_webhook(
    memory: &Arc<dyn Memory>,
    request: WebhookUpdateRequest,
) -> anyhow::Result<Webhook> {
    let existing = load_webhook(Some(memory), &request.webhook.id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("webhook does not exist"))?;
    let webhook = Webhook {
        id: existing.id,
        name: request.webhook.name,
        agent_id: request.webhook.agent_id,
        session_mode: request.webhook.session_mode,
        fixed_session_id: request.webhook.fixed_session_id,
        secret: request.secret_override.unwrap_or(existing.secret),
        enabled: request.webhook.enabled,
        created_at: existing.created_at,
        updated_at: now_text(),
        last_run_id: existing.last_run_id,
        last_secret_rotated_at: existing.last_secret_rotated_at,
        last_invoked_at: existing.last_invoked_at,
        last_rejection_at: existing.last_rejection_at,
        last_rejection_reason: existing.last_rejection_reason,
    };
    store_webhook(memory, &webhook).await?;
    Ok(webhook)
}

pub(crate) async fn delete_webhook(
    memory: &Arc<dyn Memory>,
    webhook_id: &str,
) -> anyhow::Result<bool> {
    memory.forget(&webhook_key(webhook_id)).await
}

pub(crate) async fn touch_last_run(
    memory: &Arc<dyn Memory>,
    webhook: &Webhook,
    run_id: &str,
) -> anyhow::Result<Webhook> {
    let now = now_text();
    let next = Webhook {
        updated_at: now.clone(),
        last_run_id: Some(run_id.to_string()),
        last_invoked_at: Some(now),
        ..webhook.clone()
    };
    store_webhook(memory, &next).await?;
    Ok(next)
}

pub(crate) async fn rotate_secret(
    memory: &Arc<dyn Memory>,
    webhook_id: &str,
) -> anyhow::Result<WebhookSecretRotateResponse> {
    let existing = load_webhook(Some(memory), webhook_id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("webhook does not exist"))?;
    let now = now_text();
    let new_secret = generate_secret();
    let webhook = Webhook {
        updated_at: now.clone(),
        secret: new_secret.clone(),
        last_secret_rotated_at: Some(now),
        ..existing
    };
    store_webhook(memory, &webhook).await?;
    Ok(WebhookSecretRotateResponse {
        webhook: StoredWebhook::from_webhook(&webhook).into_public_webhook(),
        new_secret,
    })
}

pub(crate) async fn record_rejected_invoke(
    memory: &Arc<dyn Memory>,
    webhook: &Webhook,
    reason: &str,
) -> anyhow::Result<Webhook> {
    let now = now_text();
    let next = Webhook {
        updated_at: now.clone(),
        last_rejection_at: Some(now.clone()),
        last_rejection_reason: Some(reason.to_string()),
        ..webhook.clone()
    };
    store_webhook(memory, &next).await?;
    let rejection = StoredWebhookRejection {
        id: format!("{}:{}", webhook.id, now),
        webhook_id: webhook.id.clone(),
        reason: reason.to_string(),
        timestamp: now,
    };
    memory
        .store(
            &webhook_rejection_key(&rejection.webhook_id, &rejection.id),
            &serde_json::to_string(&rejection)?,
            MemoryCategory::Core,
            None,
        )
        .await?;
    prune_rejections(memory, &webhook.id).await?;
    Ok(next)
}

pub(crate) fn resolve_session_id(
    webhook: &Webhook,
    invoke: &WebhookInvokeRequest,
) -> Option<String> {
    match webhook.session_mode.as_str() {
        "reuse_fixed" => webhook.fixed_session_id.clone(),
        "create_new" => None,
        _ => invoke.session_id.clone(),
    }
}

async fn store_webhook(memory: &Arc<dyn Memory>, webhook: &Webhook) -> anyhow::Result<()> {
    let payload = serde_json::to_string(&StoredWebhook::from_webhook(webhook))?;
    memory
        .store(
            &webhook_key(&webhook.id),
            &payload,
            MemoryCategory::Core,
            None,
        )
        .await
}

fn webhook_key(webhook_id: &str) -> String {
    format!("{WEBHOOKS_PREFIX}{webhook_id}")
}

fn webhook_rejection_prefix(webhook_id: &str) -> String {
    format!("{WEBHOOK_REJECTIONS_PREFIX}{webhook_id}/")
}

fn webhook_rejection_key(webhook_id: &str, rejection_id: &str) -> String {
    format!("{}{rejection_id}", webhook_rejection_prefix(webhook_id))
}

fn now_text() -> String {
    Utc::now().to_rfc3339()
}

async fn load_rejections(
    memory: &Arc<dyn Memory>,
    webhook_id: &str,
) -> anyhow::Result<Vec<(String, StoredWebhookRejection)>> {
    let prefix = webhook_rejection_prefix(webhook_id);
    let mut rejections = Vec::new();
    for entry in memory.list(Some(&MemoryCategory::Core), None).await? {
        if !entry.key.starts_with(&prefix) {
            continue;
        }
        let payload: StoredWebhookRejection = serde_json::from_str(&entry.content)?;
        rejections.push((entry.key, payload));
    }
    // Reject logs can share the same timestamp, so keep id as a deterministic tie-breaker.
    rejections.sort_by(|left, right| {
        right
            .1
            .timestamp
            .cmp(&left.1.timestamp)
            .then_with(|| right.1.id.cmp(&left.1.id))
    });
    Ok(rejections)
}

async fn prune_rejections(memory: &Arc<dyn Memory>, webhook_id: &str) -> anyhow::Result<()> {
    let rejections = load_rejections(memory, webhook_id).await?;
    for (key, _) in rejections
        .into_iter()
        .skip(WEBHOOK_REJECTION_RETENTION_LIMIT)
    {
        memory.forget(&key).await?;
    }
    Ok(())
}

impl StoredWebhook {
    fn from_webhook(webhook: &Webhook) -> Self {
        Self {
            id: webhook.id.clone(),
            name: webhook.name.clone(),
            agent_id: webhook.agent_id.clone(),
            session_mode: webhook.session_mode.clone(),
            fixed_session_id: webhook.fixed_session_id.clone(),
            secret: webhook.secret.clone(),
            enabled: webhook.enabled,
            created_at: webhook.created_at.clone(),
            updated_at: webhook.updated_at.clone(),
            last_run_id: webhook.last_run_id.clone(),
            last_secret_rotated_at: webhook.last_secret_rotated_at.clone(),
            last_invoked_at: webhook.last_invoked_at.clone(),
            last_rejection_at: webhook.last_rejection_at.clone(),
            last_rejection_reason: webhook.last_rejection_reason.clone(),
        }
    }

    fn into_webhook(self) -> Webhook {
        Webhook {
            id: self.id,
            name: self.name,
            agent_id: self.agent_id,
            session_mode: self.session_mode,
            fixed_session_id: self.fixed_session_id,
            secret: self.secret,
            enabled: self.enabled,
            created_at: self.created_at,
            updated_at: self.updated_at,
            last_run_id: self.last_run_id,
            last_secret_rotated_at: self.last_secret_rotated_at,
            last_invoked_at: self.last_invoked_at,
            last_rejection_at: self.last_rejection_at,
            last_rejection_reason: self.last_rejection_reason,
        }
    }

    fn into_public_webhook(self) -> Webhook {
        let mut webhook = self.into_webhook();
        webhook.secret = mask_secret(&webhook.secret);
        webhook
    }
}

impl From<WebhookDraft> for Webhook {
    fn from(value: WebhookDraft) -> Self {
        let now = now_text();
        Self {
            id: value.id,
            name: value.name,
            agent_id: value.agent_id,
            session_mode: value.session_mode,
            fixed_session_id: value.fixed_session_id,
            secret: value.secret,
            enabled: value.enabled,
            created_at: now.clone(),
            updated_at: now.clone(),
            last_run_id: None,
            last_secret_rotated_at: Some(now),
            last_invoked_at: None,
            last_rejection_at: None,
            last_rejection_reason: None,
        }
    }
}

fn generate_secret() -> String {
    #[cfg(target_arch = "wasm32")]
    {
        let mut bytes = [0_u8; 24];
        if getrandom::fill(&mut bytes).is_ok() {
            return bytes
                .iter()
                .map(|value| format!("{value:02x}"))
                .collect::<String>();
        }
    }

    let sequence = SECRET_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    format!("whsec-{}-{sequence}", Utc::now().timestamp_nanos_opt().unwrap_or_default())
}

fn mask_secret(secret: &str) -> String {
    if secret.len() <= 8 {
        return "********".to_string();
    }
    format!("{}…{}", &secret[..4], &secret[secret.len() - 4..])
}
