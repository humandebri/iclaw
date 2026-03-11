// where: standalone/canister/src/context.rs
// what: Memory-backed prompt context builders for the ICP canister chat flow
// why: The canister cannot read host files, so workspace docs, skills, and recalled memory
// must be assembled from durable memory before calling the provider

use crate::types::ContextConfig;
use iclaw_standalone_core::memory::Memory;

#[path = "context/parts.rs"]
mod parts;

use parts::{
    is_allowed_workspace_file, parse_skill_doc, render_action_context, render_memory_context,
    render_skills_context, render_tooling_context, render_workspace_context, trim_memory_first,
};

pub(crate) const DEFAULT_WORKSPACE_FILES: [&str; 6] = [
    "AGENTS.md",
    "SOUL.md",
    "TOOLS.md",
    "IDENTITY.md",
    "USER.md",
    "MEMORY.md",
];
const DEFAULT_SKILLS_DIR: &str = "skills";
const DEFAULT_STATIC_CONTEXT_CHARS: usize = 8_000;
const DEFAULT_SKILL_CONTEXT_CHARS: usize = 4_000;
const DEFAULT_MEMORY_RECALL_LIMIT: usize = 5;
const DEFAULT_MEMORY_MIN_SCORE: f64 = 0.0;
const DEFAULT_HISTORY_LIMIT: usize = 8;
const DEFAULT_MAX_TOOL_ITERATIONS: usize = 3;
const DEFAULT_SUMMARY_MAX_CHARS: usize = 600;
pub(crate) const DEFAULT_CYCLE_BALANCE_WARNING_THRESHOLD: u128 = 1_000_000_000_000;
const MAX_SYSTEM_PROMPT_CHARS: usize = 12_000;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct WorkspaceDoc {
    pub key: String,
    pub file_name: String,
    pub content: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct SkillDoc {
    pub name: String,
    pub summary: String,
    pub tools: Vec<String>,
    pub actions: Vec<LightweightSkillAction>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum LightweightSkillAction {
    WorkspaceLookup,
    MemoryRecall,
    HttpRequestGuidance,
    SkillSummary,
}

#[derive(Clone, Debug, PartialEq)]
struct ResolvedContextConfig {
    workspace_files: Vec<String>,
    skills_dir: String,
    max_static_context_chars: usize,
    max_skill_context_chars: usize,
    memory_recall_limit: usize,
    memory_min_score: f64,
    enable_lightweight_skill_actions: bool,
    history_limit: usize,
    max_tool_iterations: usize,
    enable_autosave: bool,
    enable_tool_loop: bool,
    enable_auto_promote: bool,
    enable_conversation_summary: bool,
    summary_max_chars: usize,
    retry_provider_once: bool,
    cycle_balance_warning_threshold: Option<u128>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct PromptContext {
    pub system_prompt: String,
    pub memory_context: String,
}

impl From<Option<&ContextConfig>> for ResolvedContextConfig {
    fn from(value: Option<&ContextConfig>) -> Self {
        Self {
            workspace_files: value
                .and_then(|config| config.workspace_files.clone())
                .unwrap_or_else(|| {
                    DEFAULT_WORKSPACE_FILES
                        .iter()
                        .map(|file| file.to_string())
                        .collect()
                }),
            skills_dir: value
                .and_then(|config| config.skills_dir.clone())
                .filter(|name| !name.trim().is_empty())
                .unwrap_or_else(|| DEFAULT_SKILLS_DIR.to_string()),
            max_static_context_chars: value
                .and_then(|config| config.max_static_context_chars)
                .map(|count| count as usize)
                .unwrap_or(DEFAULT_STATIC_CONTEXT_CHARS),
            max_skill_context_chars: value
                .and_then(|config| config.max_skill_context_chars)
                .map(|count| count as usize)
                .unwrap_or(DEFAULT_SKILL_CONTEXT_CHARS),
            memory_recall_limit: value
                .and_then(|config| config.memory_recall_limit)
                .map(|count| count as usize)
                .unwrap_or(DEFAULT_MEMORY_RECALL_LIMIT),
            memory_min_score: value
                .and_then(|config| config.memory_min_score)
                .unwrap_or(DEFAULT_MEMORY_MIN_SCORE),
            enable_lightweight_skill_actions: value
                .and_then(|config| config.enable_lightweight_skill_actions)
                .unwrap_or(true),
            history_limit: value
                .and_then(|config| config.history_limit)
                .map(|count| count as usize)
                .unwrap_or(DEFAULT_HISTORY_LIMIT),
            max_tool_iterations: value
                .and_then(|config| config.max_tool_iterations)
                .map(|count| count as usize)
                .unwrap_or(DEFAULT_MAX_TOOL_ITERATIONS),
            enable_autosave: value
                .and_then(|config| config.enable_autosave)
                .unwrap_or(true),
            enable_tool_loop: value
                .and_then(|config| config.enable_tool_loop)
                .unwrap_or(true),
            enable_auto_promote: value
                .and_then(|config| config.enable_auto_promote)
                .unwrap_or(false),
            enable_conversation_summary: value
                .and_then(|config| config.enable_conversation_summary)
                .unwrap_or(true),
            summary_max_chars: value
                .and_then(|config| config.summary_max_chars)
                .map(|count| count as usize)
                .unwrap_or(DEFAULT_SUMMARY_MAX_CHARS),
            retry_provider_once: value
                .and_then(|config| config.retry_provider_once)
                .unwrap_or(true),
            cycle_balance_warning_threshold: value
                .and_then(|config| config.cycle_balance_warning_threshold)
                .map(u128::from)
                .or(Some(DEFAULT_CYCLE_BALANCE_WARNING_THRESHOLD)),
        }
    }
}

pub(crate) async fn build_prompt_context(
    memory: Option<&dyn Memory>,
    config: Option<&ContextConfig>,
    prompt: &str,
    session_id: Option<&str>,
    tool_names: &[String],
) -> PromptContext {
    let resolved = ResolvedContextConfig::from(config);
    let docs = load_workspace_docs(memory, &resolved).await;
    let skills = load_skill_docs(memory, &resolved).await;
    let mut memory_context = load_memory_context(memory, &resolved, prompt, session_id).await;
    let mut sections = vec![
        "You are iclaw running inside an ICP canister runtime.\nUse only the canister-safe surfaces described below.".to_string(),
        render_tooling_context(tool_names),
        "Safety:\n- The canister cannot use shell, browser automation, local file I/O, or arbitrary native tools.\n- Workspace docs and skills are memory-backed snapshots, not live filesystem reads.".to_string(),
        render_skills_context(&skills, resolved.max_skill_context_chars),
        render_workspace_context(&docs, resolved.max_static_context_chars),
        if resolved.enable_lightweight_skill_actions {
            render_action_context(prompt, &docs, &skills, &memory_context)
        } else {
            String::new()
        },
    ];
    trim_memory_first(&mut sections, &mut memory_context, MAX_SYSTEM_PROMPT_CHARS);
    PromptContext {
        system_prompt: sections
            .into_iter()
            .filter(|section| !section.trim().is_empty())
            .collect::<Vec<_>>()
            .join("\n\n"),
        memory_context,
    }
}

pub(crate) fn history_limit(config: Option<&ContextConfig>) -> usize {
    ResolvedContextConfig::from(config).history_limit
}

pub(crate) fn max_tool_iterations(config: Option<&ContextConfig>) -> usize {
    ResolvedContextConfig::from(config).max_tool_iterations
}

pub(crate) fn enable_autosave(config: Option<&ContextConfig>) -> bool {
    ResolvedContextConfig::from(config).enable_autosave
}

pub(crate) fn enable_tool_loop(config: Option<&ContextConfig>) -> bool {
    ResolvedContextConfig::from(config).enable_tool_loop
}

pub(crate) fn enable_auto_promote(config: Option<&ContextConfig>) -> bool {
    ResolvedContextConfig::from(config).enable_auto_promote
}

pub(crate) fn enable_conversation_summary(config: Option<&ContextConfig>) -> bool {
    ResolvedContextConfig::from(config).enable_conversation_summary
}

pub(crate) fn summary_max_chars(config: Option<&ContextConfig>) -> usize {
    ResolvedContextConfig::from(config).summary_max_chars
}

pub(crate) fn retry_provider_once(config: Option<&ContextConfig>) -> bool {
    ResolvedContextConfig::from(config).retry_provider_once
}

pub(crate) fn cycle_balance_warning_threshold(config: Option<&ContextConfig>) -> Option<u128> {
    ResolvedContextConfig::from(config).cycle_balance_warning_threshold
}

async fn load_workspace_docs(
    memory: Option<&dyn Memory>,
    config: &ResolvedContextConfig,
) -> Vec<WorkspaceDoc> {
    let Some(memory) = memory else {
        return Vec::new();
    };
    let mut docs = Vec::new();
    let mut remaining = config.max_static_context_chars;
    for file_name in config
        .workspace_files
        .iter()
        .filter(|name| is_allowed_workspace_file(name))
    {
        if remaining == 0 {
            break;
        }
        let key = format!("workspace/{file_name}");
        if let Ok(Some(entry)) = memory.get(&key).await {
            let content = entry.content.chars().take(remaining).collect::<String>();
            if !content.trim().is_empty() {
                remaining = remaining.saturating_sub(content.chars().count());
                docs.push(WorkspaceDoc {
                    key,
                    file_name: file_name.clone(),
                    content,
                });
            }
        }
    }
    docs
}

async fn load_skill_docs(
    memory: Option<&dyn Memory>,
    config: &ResolvedContextConfig,
) -> Vec<SkillDoc> {
    let Some(memory) = memory else {
        return Vec::new();
    };
    let Ok(entries) = memory.list(None, None).await else {
        return Vec::new();
    };
    let prefix = format!("workspace/{}/", config.skills_dir.trim_matches('/'));
    let mut docs = entries
        .into_iter()
        .filter(|entry| entry.key.starts_with(&prefix) && entry.key.ends_with("/SKILL.md"))
        .filter_map(parse_skill_doc)
        .collect::<Vec<_>>();
    docs.sort_by(|left, right| left.name.cmp(&right.name));
    docs
}

async fn load_memory_context(
    memory: Option<&dyn Memory>,
    config: &ResolvedContextConfig,
    prompt: &str,
    session_id: Option<&str>,
) -> String {
    let Some(memory) = memory else {
        return String::new();
    };
    let Ok(entries) = memory
        .recall(prompt, config.memory_recall_limit, session_id)
        .await
    else {
        return String::new();
    };
    render_memory_context(&entries, config.memory_min_score)
}

#[cfg(test)]
mod tests;
