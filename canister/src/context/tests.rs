//! where: standalone/canister/src/context/tests.rs
//! what: focused tests for static docs, memory recall filtering, skills summaries, and action dispatch
//! why: keep context.rs compact while validating the canister-specific prompt assembly rules

use super::*;
use async_trait::async_trait;
use iclaw_standalone_core::memory::{MemoryCategory, MemoryEntry};
use parking_lot::Mutex;

fn entry(key: &str, content: &str, score: Option<f64>) -> MemoryEntry {
    MemoryEntry {
        id: key.to_string(),
        key: key.to_string(),
        content: content.to_string(),
        category: MemoryCategory::Core,
        timestamp: "2026-03-09T00:00:00Z".to_string(),
        session_id: None,
        score,
    }
}

struct TestMemory {
    entries: Vec<MemoryEntry>,
    recalls: Vec<MemoryEntry>,
    last_recall_limit: Mutex<Option<usize>>,
}

#[async_trait]
impl Memory for TestMemory {
    fn name(&self) -> &str {
        "test-memory"
    }

    async fn store(
        &self,
        _key: &str,
        _content: &str,
        _category: MemoryCategory,
        _session_id: Option<&str>,
    ) -> anyhow::Result<()> {
        anyhow::bail!("store is not used in context tests")
    }

    async fn recall(
        &self,
        _query: &str,
        limit: usize,
        _session_id: Option<&str>,
    ) -> anyhow::Result<Vec<MemoryEntry>> {
        *self.last_recall_limit.lock() = Some(limit);
        Ok(self.recalls.clone())
    }

    async fn get(&self, key: &str) -> anyhow::Result<Option<MemoryEntry>> {
        Ok(self.entries.iter().find(|entry| entry.key == key).cloned())
    }

    async fn list(
        &self,
        _category: Option<&MemoryCategory>,
        _session_id: Option<&str>,
    ) -> anyhow::Result<Vec<MemoryEntry>> {
        Ok(self.entries.clone())
    }

    async fn forget(&self, _key: &str) -> anyhow::Result<bool> {
        anyhow::bail!("forget is not used in context tests")
    }

    async fn count(&self) -> anyhow::Result<usize> {
        Ok(self.entries.len())
    }

    async fn health_check(&self) -> bool {
        true
    }
}

#[test]
fn memory_context_filters_low_score_autosave_and_duplicates() {
    let rendered = render_memory_context(
        &[
            entry("note/a", "keep me", Some(0.8)),
            entry("assistant_resp_legacy", "drop me", Some(1.0)),
            entry("note/b", "keep me", Some(0.9)),
            entry("note/c", "too low", Some(0.1)),
        ],
        0.5,
    );

    assert!(rendered.contains("note/a: keep me"));
    assert!(!rendered.contains("assistant_resp_legacy"));
    assert!(!rendered.contains("too low"));
    assert_eq!(rendered.matches("keep me").count(), 1);
}

#[test]
fn skills_context_only_includes_skill_docs_and_is_deterministic() {
    let doc = parse_skill_doc(entry(
        "workspace/skills/doc/SKILL.md",
        "# Doc\nSummarize docs safely.\n\nUse memory_recall before answering.\nActions: skill_summary, workspace_lookup",
        None,
    ))
    .expect("skill doc should parse");

    assert_eq!(doc.name, "doc");
    assert!(doc.summary.contains("Summarize docs safely."));
    assert!(doc.tools.contains(&"memory_recall".to_string()));
    assert!(doc.actions.contains(&LightweightSkillAction::SkillSummary));
    assert!(doc
        .actions
        .contains(&LightweightSkillAction::WorkspaceLookup));

    let rendered = render_skills_context(&[doc], 512);
    assert!(rendered.contains("[Available skills]"));
    assert!(rendered.contains("- doc:"));
}

#[test]
fn action_context_only_uses_allowed_actions() {
    let docs = vec![WorkspaceDoc {
        key: "workspace/AGENTS.md".to_string(),
        file_name: "AGENTS.md".to_string(),
        content: "Always explain the flow.".to_string(),
    }];
    let skills = vec![SkillDoc {
        name: "doc".to_string(),
        summary: "Explain docs and memory.".to_string(),
        tools: vec!["memory_recall".to_string()],
        actions: vec![
            LightweightSkillAction::WorkspaceLookup,
            LightweightSkillAction::MemoryRecall,
            LightweightSkillAction::SkillSummary,
        ],
    }];

    let rendered = render_action_context(
        "Use AGENTS.md and doc skill memory",
        &docs,
        &skills,
        "[Memory context]\n- note: persisted guidance",
    );

    assert!(rendered.contains("workspace_lookup"));
    assert!(rendered.contains("memory_recall"));
    assert!(rendered.contains("skill_summary"));
    assert!(!rendered.contains("shell"));
}

#[test]
fn workspace_context_only_injects_known_docs() {
    let rendered = render_workspace_context(
        &[
            WorkspaceDoc {
                key: "workspace/AGENTS.md".to_string(),
                file_name: "AGENTS.md".to_string(),
                content: "Agent rules".to_string(),
            },
            WorkspaceDoc {
                key: "workspace/UNLISTED.md".to_string(),
                file_name: "UNLISTED.md".to_string(),
                content: "Should not be configured".to_string(),
            },
        ],
        512,
    );

    assert!(rendered.contains("### AGENTS.md"));
    assert!(rendered.contains("### UNLISTED.md"));
    assert!(is_allowed_workspace_file("AGENTS.md"));
    assert!(!is_allowed_workspace_file("UNLISTED.md"));
}

#[test]
fn workspace_context_truncates_deterministically() {
    let rendered = render_workspace_context(
        &[WorkspaceDoc {
            key: "workspace/AGENTS.md".to_string(),
            file_name: "AGENTS.md".to_string(),
            content: "abcdefghij".to_string(),
        }],
        15,
    );

    assert_eq!(rendered, "### AGENTS.md\na");
}

#[test]
fn context_defaults_history_limit_to_eight_and_auto_promote_off_for_canister_runtime() {
    assert_eq!(history_limit(None), 8);
    assert!(!enable_auto_promote(None));
    assert_eq!(
        cycle_balance_warning_threshold(None),
        Some(DEFAULT_CYCLE_BALANCE_WARNING_THRESHOLD)
    );
}

#[tokio::test]
async fn prompt_context_respects_section_caps_and_recall_limit() {
    let memory = TestMemory {
        entries: vec![
            entry("workspace/AGENTS.md", "Agent rules", None),
            entry(
                "workspace/skills/doc/SKILL.md",
                "# Doc\nSummarize docs safely.\n\nUse memory_recall before answering.",
                None,
            ),
        ],
        recalls: vec![entry("note/a", "remember this", Some(0.9))],
        last_recall_limit: Mutex::new(None),
    };
    let config = ContextConfig {
        workspace_files: Some(vec!["AGENTS.md".to_string()]),
        skills_dir: Some("skills".to_string()),
        max_static_context_chars: Some(32),
        max_skill_context_chars: Some(128),
        memory_recall_limit: Some(3),
        memory_min_score: Some(0.0),
        enable_lightweight_skill_actions: Some(true),
        history_limit: Some(8),
        max_tool_iterations: Some(3),
        enable_autosave: Some(true),
        enable_tool_loop: Some(true),
        enable_auto_promote: Some(true),
        enable_conversation_summary: Some(true),
        summary_max_chars: Some(240),
        retry_provider_once: Some(true),
        cycle_balance_warning_threshold: None,
        max_prompt_chars: None,
        llm_summary_on_overflow: None,
        llm_summary_model: None,
        llm_summary_max_chars: None,
    };

    let prompt = build_prompt_context(
        Some(&memory),
        Some(&config),
        "Use the doc skill",
        Some("session-a"),
        &["memory_recall".to_string()],
    )
    .await;

    assert!(prompt.system_prompt.contains("### AGENTS.md"));
    assert!(prompt.system_prompt.contains("[Available skills]"));
    assert!(prompt.memory_context.contains("[Memory context]"));
    assert!(prompt.system_prompt.len() <= 12_000);
    assert!(
        render_workspace_context(
            &[WorkspaceDoc {
                key: "workspace/AGENTS.md".to_string(),
                file_name: "AGENTS.md".to_string(),
                content: "Agent rules".to_string(),
            }],
            32,
        )
        .len()
            <= 32
    );
    assert!(
        render_skills_context(
            &[parse_skill_doc(entry(
                "workspace/skills/doc/SKILL.md",
                "# Doc\nSummarize docs safely.\n\nUse memory_recall before answering.",
                None,
            ))
            .expect("skill doc should parse")],
            128,
        )
        .len()
            <= 128
    );
    assert_eq!(*memory.last_recall_limit.lock(), Some(3));
}

#[tokio::test]
async fn prompt_context_keeps_action_context_when_memory_is_trimmed() {
    let memory = TestMemory {
        entries: vec![
            entry("workspace/AGENTS.md", "Always explain the flow.", None),
            entry(
                "workspace/skills/doc/SKILL.md",
                "# Doc\nSummarize docs safely.\n\nActions: workspace_lookup, skill_summary, memory_recall",
                None,
            ),
        ],
        recalls: vec![entry(
            "note/large",
            &"remember ".repeat(2_000),
            Some(0.9),
        )],
        last_recall_limit: Mutex::new(None),
    };

    let prompt = build_prompt_context(
        Some(&memory),
        Some(&context_config()),
        "Use AGENTS.md and doc skill memory",
        Some("session-a"),
        &["memory_recall".to_string()],
    )
    .await;

    assert!(prompt.system_prompt.contains("[Skill action context]"));
    assert!(prompt.system_prompt.contains("workspace_lookup"));
    assert!(!prompt.system_prompt.contains("[Memory context]"));
    assert!(prompt.memory_context.contains("[Memory context]"));
    assert!(
        prompt.memory_context.len()
            < "[Memory context]\n- note/large: ".len() + "remember ".repeat(2_000).len()
    );
}

fn context_config() -> ContextConfig {
    ContextConfig {
        workspace_files: Some(vec!["AGENTS.md".to_string()]),
        skills_dir: Some("skills".to_string()),
        max_static_context_chars: Some(512),
        max_skill_context_chars: Some(512),
        memory_recall_limit: Some(3),
        memory_min_score: Some(0.0),
        enable_lightweight_skill_actions: Some(true),
        history_limit: Some(8),
        max_tool_iterations: Some(3),
        enable_autosave: Some(true),
        enable_tool_loop: Some(true),
        enable_auto_promote: Some(true),
        enable_conversation_summary: Some(true),
        summary_max_chars: Some(240),
        retry_provider_once: Some(true),
        cycle_balance_warning_threshold: None,
        max_prompt_chars: None,
        llm_summary_on_overflow: None,
        llm_summary_model: None,
        llm_summary_max_chars: None,
    }
}
