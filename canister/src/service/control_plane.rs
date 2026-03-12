//! where: iclaw/canister/src/service/control_plane.rs
//! what: Durable agent and tool-policy helpers for the phase-2a control plane
//! why: keep run orchestration focused on execution while durable control-plane records stay isolated

use crate::types::{
    Agent, AgentCreateRequest, AgentDraft, AgentUpdateRequest, ToolPolicy, ToolPolicyUpdateRequest,
};
use iclaw_core::memory::{Memory, MemoryCategory};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

const AGENTS_PREFIX: &str = "agents/";
const TOOL_POLICIES_PREFIX: &str = "tool_policies/";
const DEFAULT_AGENT_ID: &str = "default";
const DEFAULT_AGENT_NAME: &str = "Default Agent";
const DEFAULT_AGENT_DESCRIPTION: &str = "Single-canister default agent";

#[derive(Clone, Debug, Serialize, Deserialize)]
struct StoredAgent {
    id: String,
    name: String,
    description: String,
    enabled_tool_names: Vec<String>,
    requires_tool_approval: bool,
    system_prompt_override: Option<String>,
    status: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct StoredToolPolicy {
    agent_id: String,
    tool_name: String,
    enabled: bool,
    requires_approval: bool,
}

pub(crate) fn default_agent(tool_names: &[String]) -> Agent {
    Agent {
        id: DEFAULT_AGENT_ID.to_string(),
        name: DEFAULT_AGENT_NAME.to_string(),
        description: DEFAULT_AGENT_DESCRIPTION.to_string(),
        enabled_tool_names: tool_names.to_vec(),
        requires_tool_approval: false,
        system_prompt_override: None,
        status: "active".to_string(),
    }
}

pub(crate) async fn list_agents(
    memory: Option<&Arc<dyn Memory>>,
    tool_names: &[String],
) -> anyhow::Result<Vec<Agent>> {
    let mut agents = Vec::new();
    if let Some(memory) = memory {
        for entry in memory.list(Some(&MemoryCategory::Core), None).await? {
            if !entry.key.starts_with(AGENTS_PREFIX) {
                continue;
            }
            let payload: StoredAgent = serde_json::from_str(&entry.content)?;
            agents.push(payload.into_agent());
        }
    }
    if !agents.iter().any(|agent| agent.id == DEFAULT_AGENT_ID) {
        agents.push(default_agent(tool_names));
    }
    agents.sort_by(|left, right| left.id.cmp(&right.id));
    Ok(agents)
}

pub(crate) async fn get_agent(
    memory: Option<&Arc<dyn Memory>>,
    tool_names: &[String],
    agent_id: &str,
) -> anyhow::Result<Option<Agent>> {
    if agent_id == DEFAULT_AGENT_ID {
        if let Some(memory) = memory {
            if let Some(entry) = memory.get(&agent_key(agent_id)).await? {
                let payload: StoredAgent = serde_json::from_str(&entry.content)?;
                return Ok(Some(payload.into_agent()));
            }
        }
        return Ok(Some(default_agent(tool_names)));
    }
    let Some(memory) = memory else {
        return Ok(None);
    };
    let Some(entry) = memory.get(&agent_key(agent_id)).await? else {
        return Ok(None);
    };
    let payload: StoredAgent = serde_json::from_str(&entry.content)?;
    Ok(Some(payload.into_agent()))
}

pub(crate) async fn create_agent(
    memory: &Arc<dyn Memory>,
    request: AgentCreateRequest,
) -> anyhow::Result<Agent> {
    let agent = Agent::from(request.draft);
    store_agent(memory, &agent).await?;
    Ok(agent)
}

pub(crate) async fn update_agent(
    memory: &Arc<dyn Memory>,
    request: AgentUpdateRequest,
) -> anyhow::Result<Agent> {
    let agent = request.agent;
    store_agent(memory, &agent).await?;
    Ok(agent)
}

pub(crate) async fn list_tool_policies(
    memory: Option<&Arc<dyn Memory>>,
    _tool_names: &[String],
    agent_id: Option<&str>,
) -> anyhow::Result<Vec<ToolPolicy>> {
    let Some(memory) = memory else {
        return Ok(Vec::new());
    };
    let mut policies = Vec::new();
    for entry in memory.list(Some(&MemoryCategory::Core), None).await? {
        if !entry.key.starts_with(TOOL_POLICIES_PREFIX) {
            continue;
        }
        let payload: StoredToolPolicy = serde_json::from_str(&entry.content)?;
        let policy = payload.into_policy();
        if agent_id.is_none_or(|id| policy.agent_id == id) {
            policies.push(policy);
        }
    }
    policies.sort_by(|left, right| {
        left.agent_id
            .cmp(&right.agent_id)
            .then(left.tool_name.cmp(&right.tool_name))
    });
    Ok(policies)
}

pub(crate) async fn upsert_tool_policy(
    memory: &Arc<dyn Memory>,
    _tool_names: &[String],
    request: ToolPolicyUpdateRequest,
) -> anyhow::Result<ToolPolicy> {
    let policy = request.policy;
    let payload = serde_json::to_string(&StoredToolPolicy::from_policy(&policy))?;
    memory
        .store(
            &tool_policy_key(&policy.agent_id, &policy.tool_name),
            &payload,
            MemoryCategory::Core,
            None,
        )
        .await?;
    Ok(policy)
}

async fn store_agent(memory: &Arc<dyn Memory>, agent: &Agent) -> anyhow::Result<()> {
    let payload = serde_json::to_string(&StoredAgent::from_agent(agent))?;
    memory
        .store(&agent_key(&agent.id), &payload, MemoryCategory::Core, None)
        .await
}

fn agent_key(agent_id: &str) -> String {
    format!("{AGENTS_PREFIX}{agent_id}")
}

fn tool_policy_key(agent_id: &str, tool_name: &str) -> String {
    format!(
        "{TOOL_POLICIES_PREFIX}{}/{}",
        sanitize_key_segment(agent_id),
        sanitize_key_segment(tool_name)
    )
}

fn sanitize_key_segment(value: &str) -> String {
    value.replace('/', "~")
}

impl StoredAgent {
    fn from_agent(agent: &Agent) -> Self {
        Self {
            id: agent.id.clone(),
            name: agent.name.clone(),
            description: agent.description.clone(),
            enabled_tool_names: agent.enabled_tool_names.clone(),
            requires_tool_approval: agent.requires_tool_approval,
            system_prompt_override: agent.system_prompt_override.clone(),
            status: agent.status.clone(),
        }
    }

    fn into_agent(self) -> Agent {
        Agent {
            id: self.id,
            name: self.name,
            description: self.description,
            enabled_tool_names: self.enabled_tool_names,
            requires_tool_approval: self.requires_tool_approval,
            system_prompt_override: self.system_prompt_override,
            status: self.status,
        }
    }
}

impl StoredToolPolicy {
    fn from_policy(policy: &ToolPolicy) -> Self {
        Self {
            agent_id: policy.agent_id.clone(),
            tool_name: policy.tool_name.clone(),
            enabled: policy.enabled,
            requires_approval: policy.requires_approval,
        }
    }

    fn into_policy(self) -> ToolPolicy {
        ToolPolicy {
            agent_id: self.agent_id,
            tool_name: self.tool_name,
            enabled: self.enabled,
            requires_approval: self.requires_approval,
        }
    }
}

impl From<AgentDraft> for Agent {
    fn from(value: AgentDraft) -> Self {
        Self {
            id: value.id,
            name: value.name,
            description: value.description,
            enabled_tool_names: value.enabled_tool_names,
            requires_tool_approval: value.requires_tool_approval,
            system_prompt_override: value.system_prompt_override,
            status: value.status,
        }
    }
}
