//! where: standalone/canister/src/service/tests.rs
//! what: service-level tests for provider readiness, history/autosave, and tool-loop behavior
//! why: validate the canister-facing agent flow without overloading service.rs

use super::*;
use crate::provider::{IcCanisterProvider, ProviderChatResult};
use async_trait::async_trait;
use iclaw_standalone_core::memory::{Memory, MemoryCategory, MemoryEntry};
use iclaw_standalone_core::providers::{
    ChatResponse as ProviderResponse, ConversationMessage, ProviderCapabilities, ToolCall,
};
use parking_lot::Mutex;

#[derive(Clone, Debug)]
struct ProviderCall {
    messages: Vec<ConversationMessage>,
    tool_names: Vec<String>,
    model: String,
}

struct MockProvider {
    responses: Mutex<Vec<Result<ProviderChatResult, String>>>,
    calls: Mutex<Vec<ProviderCall>>,
}

struct TestMemory {
    listed: Mutex<Vec<MemoryEntry>>,
    recalled: Vec<MemoryEntry>,
    stored: Mutex<Vec<StoredRecord>>,
    forgotten: Mutex<Vec<String>>,
}

#[derive(Clone, Debug)]
struct StoredRecord {
    key: String,
    content: String,
    category: MemoryCategory,
    session_id: Option<String>,
}

#[async_trait]
impl Memory for TestMemory {
    fn name(&self) -> &str {
        "test-memory"
    }

    async fn store(
        &self,
        key: &str,
        content: &str,
        category: MemoryCategory,
        session_id: Option<&str>,
    ) -> anyhow::Result<()> {
        self.stored.lock().push(StoredRecord {
            key: key.to_string(),
            content: content.to_string(),
            category: category.clone(),
            session_id: session_id.map(str::to_string),
        });
        let mut listed = self.listed.lock();
        let sequence = listed.len() + 1;
        let next = MemoryEntry {
            id: key.to_string(),
            key: key.to_string(),
            content: content.to_string(),
            category,
            timestamp: format!("2026-03-09T00:00:{sequence:02}Z"),
            session_id: session_id.map(str::to_string),
            score: None,
        };
        if let Some(existing) = listed.iter_mut().find(|entry| entry.key == key) {
            *existing = next;
        } else {
            listed.push(next);
        }
        Ok(())
    }

    async fn recall(
        &self,
        _query: &str,
        _limit: usize,
        _session_id: Option<&str>,
    ) -> anyhow::Result<Vec<MemoryEntry>> {
        Ok(self.recalled.clone())
    }

    async fn get(&self, key: &str) -> anyhow::Result<Option<MemoryEntry>> {
        Ok(self
            .listed
            .lock()
            .iter()
            .find(|entry| entry.key == key)
            .cloned())
    }

    async fn list(
        &self,
        category: Option<&MemoryCategory>,
        session_id: Option<&str>,
    ) -> anyhow::Result<Vec<MemoryEntry>> {
        Ok(self
            .listed
            .lock()
            .iter()
            .filter(|entry| category.is_none_or(|expected| &entry.category == expected))
            .filter(|entry| {
                session_id.is_none_or(|expected| entry.session_id.as_deref() == Some(expected))
            })
            .cloned()
            .collect())
    }

    async fn forget(&self, key: &str) -> anyhow::Result<bool> {
        self.forgotten.lock().push(key.to_string());
        let mut listed = self.listed.lock();
        let before = listed.len();
        listed.retain(|entry| entry.key != key);
        Ok(listed.len() != before)
    }

    async fn count(&self) -> anyhow::Result<usize> {
        Ok(self.listed.lock().len())
    }

    async fn health_check(&self) -> bool {
        true
    }
}

#[async_trait]
impl IcCanisterProvider for MockProvider {
    async fn chat(
        &self,
        messages: &[ConversationMessage],
        tools: Option<&[iclaw_standalone_core::tools::ToolSpec]>,
        model: &str,
        _temperature: f64,
    ) -> anyhow::Result<ProviderChatResult> {
        self.calls.lock().push(ProviderCall {
            messages: messages.to_vec(),
            tool_names: tools
                .unwrap_or(&[])
                .iter()
                .map(|tool| tool.name.clone())
                .collect(),
            model: model.to_string(),
        });
        match self.responses.lock().remove(0) {
            Ok(response) => Ok(response),
            Err(error) => anyhow::bail!(error),
        }
    }

    fn capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities {
            native_tool_calling: true,
            vision: false,
        }
    }
}

fn configured_provider() -> ProviderConfig {
    ProviderConfig {
        api_url: "https://api.openai.com/v1".to_string(),
        api_key: "openai-test-key".to_string(),
        default_model: "gpt-4o-mini".to_string(),
        timeout_secs: Some(30),
    }
}

fn context_config() -> ContextConfig {
    ContextConfig {
        workspace_files: Some(vec!["AGENTS.md".to_string(), "MEMORY.md".to_string()]),
        skills_dir: Some("skills".to_string()),
        max_static_context_chars: Some(512),
        max_skill_context_chars: Some(512),
        memory_recall_limit: Some(5),
        memory_min_score: Some(0.5),
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

fn context_config_with_limit(max_tool_iterations: u64) -> ContextConfig {
    let mut config = context_config();
    config.max_tool_iterations = Some(max_tool_iterations);
    config
}

fn entry(
    key: &str,
    content: &str,
    category: MemoryCategory,
    session_id: Option<&str>,
    score: Option<f64>,
) -> MemoryEntry {
    MemoryEntry {
        id: key.to_string(),
        key: key.to_string(),
        content: content.to_string(),
        category,
        timestamp: "2026-03-09T00:00:00Z".to_string(),
        session_id: session_id.map(str::to_string),
        score,
    }
}

#[tokio::test]
async fn health_reports_provider_ready_when_configured() {
    let service = ConnectedIclawIcService::with_dependencies(
        Some(Arc::new(TestMemory {
            listed: Mutex::new(Vec::new()),
            recalled: Vec::new(),
            stored: Mutex::new(Vec::new()),
            forgotten: Mutex::new(Vec::new()),
        })),
        None,
        Some(configured_provider()),
        Some(Arc::new(MockProvider {
            responses: Mutex::new(vec![Ok(ProviderChatResult {
                response: ProviderResponse {
                    text: Some("ok".to_string()),
                    tool_calls: Vec::new(),
                    usage: None,
                    reasoning_content: None,
                },
                model: Some("gpt-4o-mini".to_string()),
            })]),
            calls: Mutex::new(Vec::new()),
        })),
        None,
        Some(context_config()),
    );

    let health = service.health().await;
    assert!(health.provider_ready);
    assert!(health.memory_ready);
    assert_eq!(health.status, "ok");
}

#[tokio::test]
async fn chat_builds_history_and_autosaves_turns() {
    set_test_cycle_balance_override(None);
    let memory = Arc::new(TestMemory {
        listed: Mutex::new(vec![
            entry(
                "workspace/AGENTS.md",
                "Always explain the canister flow.",
                MemoryCategory::Core,
                None,
                None,
            ),
            entry(
                "workspace/MEMORY.md",
                "Gemini smoke already passed.",
                MemoryCategory::Core,
                None,
                None,
            ),
            entry(
                "workspace/skills/doc/SKILL.md",
                "# doc\nExplain docs clearly.",
                MemoryCategory::Core,
                None,
                None,
            ),
            entry(
                "conversation/session-a/user/old",
                "old user",
                MemoryCategory::Conversation,
                Some("session-a"),
                None,
            ),
            entry(
                "conversation/session-a/assistant/old",
                "old assistant",
                MemoryCategory::Conversation,
                Some("session-a"),
                None,
            ),
        ]),
        recalled: vec![
            entry(
                "conversation/relevant",
                "User cares about skills.",
                MemoryCategory::Conversation,
                Some("session-a"),
                Some(0.9),
            ),
            entry(
                "conversation/session-a/assistant/legacy",
                "must be filtered",
                MemoryCategory::Conversation,
                Some("session-a"),
                Some(1.0),
            ),
        ],
        stored: Mutex::new(Vec::new()),
        forgotten: Mutex::new(Vec::new()),
    });
    let provider = Arc::new(MockProvider {
        responses: Mutex::new(vec![Ok(ProviderChatResult {
            response: ProviderResponse {
                text: Some("transport-ok".to_string()),
                tool_calls: Vec::new(),
                usage: None,
                reasoning_content: None,
            },
            model: Some("provider-model".to_string()),
        })]),
        calls: Mutex::new(Vec::new()),
    });
    let service = ConnectedIclawIcService::with_dependencies(
        Some(memory.clone()),
        None,
        Some(configured_provider()),
        Some(provider.clone()),
        None,
        Some(context_config()),
    );

    let response = service
        .chat(ChatRequest {
            prompt: "Use AGENTS.md and the doc skill memory".to_string(),
            session_id: Some("session-a".to_string()),
            model: None,
            temperature: Some(0.2),
        })
        .await
        .expect("configured provider should succeed");

    assert_eq!(response.response, "transport-ok");
    let calls = provider.calls.lock();
    let call = calls.last().expect("provider should be called");
    assert_eq!(call.model, "gpt-4o-mini");
    assert!(call.tool_names.contains(&"memory_recall".to_string()));
    assert!(
        matches!(&call.messages[0], ConversationMessage::Chat(chat) if chat.role == "system" && chat.content.contains("### AGENTS.md"))
    );
    assert!(
        matches!(&call.messages[1], ConversationMessage::Chat(chat) if chat.role == "user" && chat.content.contains("[Memory context]"))
    );
    assert!(call.messages.iter().any(|message| matches!(message, ConversationMessage::Chat(chat) if chat.role == "user" && chat.content == "old user")));
    assert!(call.messages.iter().any(|message| matches!(message, ConversationMessage::Chat(chat) if chat.role == "assistant" && chat.content == "old assistant")));
    assert!(!call.messages.iter().any(|message| matches!(message, ConversationMessage::Chat(chat) if chat.content == "must be filtered")));

    let stored = memory.stored.lock();
    assert!(stored.iter().any(|record| record.key.contains("/user/")
        && record.content == "Use AGENTS.md and the doc skill memory"));
    assert!(stored
        .iter()
        .any(|record| record.key.contains("/assistant/") && record.content == "transport-ok"));
    assert!(stored
        .iter()
        .any(|record| record.category == MemoryCategory::Conversation
            && record.session_id.as_deref() == Some("session-a")));
}

#[tokio::test]
async fn chat_injects_cycle_warning_into_system_prompt_when_balance_is_low() {
    set_test_cycle_balance_override(Some(900));
    let provider = Arc::new(MockProvider {
        responses: Mutex::new(vec![Ok(ProviderChatResult {
            response: ProviderResponse {
                text: Some("warning acknowledged".to_string()),
                tool_calls: Vec::new(),
                usage: None,
                reasoning_content: None,
            },
            model: Some("provider-model".to_string()),
        })]),
        calls: Mutex::new(Vec::new()),
    });
    let service = ConnectedIclawIcService::with_dependencies(
        Some(Arc::new(TestMemory {
            listed: Mutex::new(Vec::new()),
            recalled: Vec::new(),
            stored: Mutex::new(Vec::new()),
            forgotten: Mutex::new(Vec::new()),
        })),
        None,
        Some(configured_provider()),
        Some(provider.clone()),
        None,
        Some(ContextConfig {
            cycle_balance_warning_threshold: Some(1_000),
            ..context_config()
        }),
    );

    let response = service
        .chat(ChatRequest {
            prompt: "hello".to_string(),
            session_id: Some("session-low-cycle".to_string()),
            model: None,
            temperature: Some(0.0),
        })
        .await
        .expect("chat should succeed");

    assert_eq!(response.response, "warning acknowledged");
    let calls = provider.calls.lock();
    let first_message = calls
        .first()
        .and_then(|call| call.messages.first())
        .expect("system message should exist");
    assert!(matches!(
        first_message,
        ConversationMessage::Chat(chat)
            if chat.role == "system"
                && chat.content.contains("[Runtime cycle warning]")
                && chat.content.contains("Liquid cycle balance is low: 900 <= 1000")
    ));
    set_test_cycle_balance_override(None);
}

#[tokio::test]
async fn chat_injects_session_summary_before_recent_history() {
    let memory = Arc::new(TestMemory {
        listed: Mutex::new(vec![
            entry(
                "conversation_summary/session-summary",
                "turn_count:8\n[Session summary]\n- user: prior context matters",
                MemoryCategory::Conversation,
                Some("session-summary"),
                None,
            ),
            entry(
                "conversation/session-summary/user/recent",
                "recent user",
                MemoryCategory::Conversation,
                Some("session-summary"),
                None,
            ),
        ]),
        recalled: Vec::new(),
        stored: Mutex::new(Vec::new()),
        forgotten: Mutex::new(Vec::new()),
    });
    let provider = Arc::new(MockProvider {
        responses: Mutex::new(vec![Ok(ProviderChatResult {
            response: ProviderResponse {
                text: Some("summary-ok".to_string()),
                tool_calls: Vec::new(),
                usage: None,
                reasoning_content: None,
            },
            model: Some("provider-model".to_string()),
        })]),
        calls: Mutex::new(Vec::new()),
    });
    let service = ConnectedIclawIcService::with_dependencies(
        Some(memory),
        None,
        Some(configured_provider()),
        Some(provider.clone()),
        None,
        Some(context_config()),
    );

    let _ = service
        .chat(ChatRequest {
            prompt: "use prior context".to_string(),
            session_id: Some("session-summary".to_string()),
            model: None,
            temperature: Some(0.0),
        })
        .await
        .expect("chat should succeed");

    let call = provider
        .calls
        .lock()
        .last()
        .cloned()
        .expect("provider call");
    assert!(matches!(
        &call.messages[0],
        ConversationMessage::Chat(chat) if chat.role == "system"
    ));
    assert!(matches!(
        &call.messages[1],
        ConversationMessage::Chat(chat) if chat.role == "user" && chat.content.contains("[Session summary]")
    ));
    assert!(call.messages.iter().any(|message| matches!(
        message,
        ConversationMessage::Chat(chat) if chat.role == "user" && chat.content == "recent user"
    )));
}

#[tokio::test]
async fn chat_compacts_payload_and_keeps_recent_history_under_budget() {
    let memory = Arc::new(TestMemory {
        listed: Mutex::new(vec![
            entry(
                "conversation/session-compact/user/01",
                &"older user ".repeat(20),
                MemoryCategory::Conversation,
                Some("session-compact"),
                None,
            ),
            entry(
                "conversation/session-compact/assistant/02",
                &"older assistant ".repeat(20),
                MemoryCategory::Conversation,
                Some("session-compact"),
                None,
            ),
            entry(
                "conversation/session-compact/user/03",
                &"recent user ".repeat(18),
                MemoryCategory::Conversation,
                Some("session-compact"),
                None,
            ),
            entry(
                "conversation/session-compact/assistant/04",
                &"recent assistant ".repeat(18),
                MemoryCategory::Conversation,
                Some("session-compact"),
                None,
            ),
        ]),
        recalled: Vec::new(),
        stored: Mutex::new(Vec::new()),
        forgotten: Mutex::new(Vec::new()),
    });
    let provider = Arc::new(MockProvider {
        responses: Mutex::new(vec![Ok(ProviderChatResult {
            response: ProviderResponse {
                text: Some("compacted".to_string()),
                tool_calls: Vec::new(),
                usage: None,
                reasoning_content: None,
            },
            model: Some("provider-model".to_string()),
        })]),
        calls: Mutex::new(Vec::new()),
    });
    let service = ConnectedIclawIcService::with_dependencies(
        Some(memory),
        None,
        Some(configured_provider()),
        Some(provider.clone()),
        None,
        Some(ContextConfig {
            llm_summary_on_overflow: Some(false),
            max_prompt_chars: Some(700),
            ..context_config()
        }),
    );

    let _ = service
        .chat(ChatRequest {
            prompt: "latest prompt".to_string(),
            session_id: Some("session-compact".to_string()),
            model: None,
            temperature: Some(0.0),
        })
        .await
        .expect("chat should succeed");

    let call = provider.calls.lock().last().cloned().expect("provider call");
    assert!(crate::service::compression::estimate_messages_chars(&call.messages) <= 700);
    assert!(matches!(
        &call.messages[0],
        ConversationMessage::Chat(chat)
            if chat.role == "system" && chat.content.contains("Earlier history was compacted.")
    ));
    assert!(!call.messages.iter().any(|message| matches!(
        message,
        ConversationMessage::Chat(chat) if chat.content.contains("older assistant older assistant")
    )));
    assert!(call.messages.iter().any(|message| matches!(
        message,
        ConversationMessage::Chat(chat) if chat.role == "user" && chat.content == "latest prompt"
    )));
}

#[tokio::test]
async fn chat_recompacts_summary_without_llm_when_truncation_is_enough() {
    let memory = Arc::new(TestMemory {
        listed: Mutex::new(vec![
            entry(
                "conversation_summary/session-summary-trim",
                &format!("turn_count:12\nsummary_turn_count:12\n[Session summary]\n{}", "summary ".repeat(120)),
                MemoryCategory::Conversation,
                Some("session-summary-trim"),
                None,
            ),
            entry(
                "conversation/session-summary-trim/user/recent",
                "recent user",
                MemoryCategory::Conversation,
                Some("session-summary-trim"),
                None,
            ),
        ]),
        recalled: Vec::new(),
        stored: Mutex::new(Vec::new()),
        forgotten: Mutex::new(Vec::new()),
    });
    let provider = Arc::new(MockProvider {
        responses: Mutex::new(vec![Ok(ProviderChatResult {
            response: ProviderResponse {
                text: Some("trimmed".to_string()),
                tool_calls: Vec::new(),
                usage: None,
                reasoning_content: None,
            },
            model: Some("provider-model".to_string()),
        })]),
        calls: Mutex::new(Vec::new()),
    });
    let service = ConnectedIclawIcService::with_dependencies(
        Some(memory),
        None,
        Some(configured_provider()),
        Some(provider.clone()),
        None,
        Some(ContextConfig {
            max_prompt_chars: Some(760),
            llm_summary_model: Some("summary-model".to_string()),
            llm_summary_max_chars: Some(80),
            ..context_config()
        }),
    );

    let _ = service
        .chat(ChatRequest {
            prompt: "use compacted summary".to_string(),
            session_id: Some("session-summary-trim".to_string()),
            model: None,
            temperature: Some(0.0),
        })
        .await
        .expect("chat should succeed");

    let calls = provider.calls.lock();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].model, "gpt-4o-mini");
    assert!(crate::service::compression::estimate_messages_chars(&calls[0].messages) <= 760);
}

#[tokio::test]
async fn chat_uses_llm_summary_once_and_persists_compacted_summary() {
    let memory = Arc::new(TestMemory {
        listed: Mutex::new(vec![
            entry(
                "conversation_summary/session-llm-summary",
                &format!("turn_count:16\nsummary_turn_count:16\n[Session summary]\n{}", "older summary ".repeat(120)),
                MemoryCategory::Conversation,
                Some("session-llm-summary"),
                None,
            ),
            entry(
                "conversation/session-llm-summary/user/01",
                &"old user ".repeat(30),
                MemoryCategory::Conversation,
                Some("session-llm-summary"),
                None,
            ),
            entry(
                "conversation/session-llm-summary/assistant/02",
                &"old assistant ".repeat(30),
                MemoryCategory::Conversation,
                Some("session-llm-summary"),
                None,
            ),
            entry(
                "conversation/session-llm-summary/user/03",
                &"recent user ".repeat(24),
                MemoryCategory::Conversation,
                Some("session-llm-summary"),
                None,
            ),
        ]),
        recalled: Vec::new(),
        stored: Mutex::new(Vec::new()),
        forgotten: Mutex::new(Vec::new()),
    });
    let provider = Arc::new(MockProvider {
        responses: Mutex::new(vec![
            Ok(ProviderChatResult {
                response: ProviderResponse {
                    text: Some("compressed durable context".to_string()),
                    tool_calls: Vec::new(),
                    usage: None,
                    reasoning_content: None,
                },
                model: Some("summary-model".to_string()),
            }),
            Ok(ProviderChatResult {
                response: ProviderResponse {
                    text: Some("done".to_string()),
                    tool_calls: Vec::new(),
                    usage: None,
                    reasoning_content: None,
                },
                model: Some("provider-model".to_string()),
            }),
        ]),
        calls: Mutex::new(Vec::new()),
    });
    let service = ConnectedIclawIcService::with_dependencies(
        Some(memory.clone()),
        None,
        Some(configured_provider()),
        Some(provider.clone()),
        None,
        Some(ContextConfig {
            max_prompt_chars: Some(420),
            llm_summary_model: Some("summary-model".to_string()),
            llm_summary_max_chars: Some(80),
            ..context_config()
        }),
    );

    let response = service
        .chat(ChatRequest {
            prompt: "finish the response".to_string(),
            session_id: Some("session-llm-summary".to_string()),
            model: None,
            temperature: Some(0.0),
        })
        .await
        .expect("chat should succeed");

    assert_eq!(response.response, "done");
    let calls = provider.calls.lock();
    assert_eq!(calls.len(), 2);
    assert_eq!(calls[0].model, "summary-model");
    assert_eq!(calls[1].model, "gpt-4o-mini");
    assert!(calls[1].messages.iter().any(|message| matches!(
        message,
        ConversationMessage::Chat(chat) if chat.content.contains("[Compacted session summary]")
    )));
    let stored = memory
        .get("conversation_summary/session-llm-summary")
        .await
        .expect("summary lookup should succeed")
        .expect("summary should exist");
    assert!(stored.content.contains("[Compacted session summary]"));
    assert!(stored.content.contains("compressed durable context"));
}

#[tokio::test]
async fn chat_falls_back_when_llm_summary_generation_fails() {
    let provider = Arc::new(MockProvider {
        responses: Mutex::new(vec![
            Err("summary failed".to_string()),
            Ok(ProviderChatResult {
                response: ProviderResponse {
                    text: Some("fallback-ok".to_string()),
                    tool_calls: Vec::new(),
                    usage: None,
                    reasoning_content: None,
                },
                model: Some("provider-model".to_string()),
            }),
        ]),
        calls: Mutex::new(Vec::new()),
    });
    let service = ConnectedIclawIcService::with_dependencies(
        Some(Arc::new(TestMemory {
            listed: Mutex::new(vec![
                entry(
                    "conversation_summary/session-llm-fail",
                    &format!("turn_count:10\nsummary_turn_count:10\n[Session summary]\n{}", "overflow ".repeat(100)),
                    MemoryCategory::Conversation,
                    Some("session-llm-fail"),
                    None,
                ),
                entry(
                    "conversation/session-llm-fail/user/01",
                    &"old ".repeat(40),
                    MemoryCategory::Conversation,
                    Some("session-llm-fail"),
                    None,
                ),
            ]),
            recalled: Vec::new(),
            stored: Mutex::new(Vec::new()),
            forgotten: Mutex::new(Vec::new()),
        })),
        None,
        Some(configured_provider()),
        Some(provider.clone()),
        None,
        Some(ContextConfig {
            max_prompt_chars: Some(320),
            llm_summary_model: Some("summary-model".to_string()),
            llm_summary_max_chars: Some(60),
            ..context_config()
        }),
    );

    let response = service
        .chat(ChatRequest {
            prompt: "continue".to_string(),
            session_id: Some("session-llm-fail".to_string()),
            model: None,
            temperature: Some(0.0),
        })
        .await
        .expect("chat should succeed");

    assert_eq!(response.response, "fallback-ok");
    let calls = provider.calls.lock();
    assert_eq!(calls.len(), 2);
    assert_eq!(calls[0].model, "summary-model");
    assert!(matches!(
        &calls[1].messages[0],
        ConversationMessage::Chat(chat)
            if chat.role == "system"
                && chat.content.contains("Earlier history was compacted.")
    ));
}

#[tokio::test]
async fn tool_loop_reapplies_payload_compaction_before_second_provider_call() {
    let provider = Arc::new(MockProvider {
        responses: Mutex::new(vec![
            Ok(ProviderChatResult {
                response: ProviderResponse {
                    text: Some("tool step".to_string()),
                    tool_calls: vec![ToolCall {
                        id: "call-1".to_string(),
                        name: "memory_store".to_string(),
                        arguments: serde_json::json!({
                            "key": "note/tool",
                            "content": "stored from tool",
                            "session_id": "session-tool-compact"
                        })
                        .to_string(),
                    }],
                    usage: None,
                    reasoning_content: None,
                },
                model: Some("provider-model".to_string()),
            }),
            Ok(ProviderChatResult {
                response: ProviderResponse {
                    text: Some("final answer".to_string()),
                    tool_calls: Vec::new(),
                    usage: None,
                    reasoning_content: None,
                },
                model: Some("provider-model".to_string()),
            }),
        ]),
        calls: Mutex::new(Vec::new()),
    });
    let service = ConnectedIclawIcService::with_dependencies(
        Some(Arc::new(TestMemory {
            listed: Mutex::new(vec![
                entry(
                    "conversation/session-tool-compact/user/01",
                    &"older user ".repeat(20),
                    MemoryCategory::Conversation,
                    Some("session-tool-compact"),
                    None,
                ),
                entry(
                    "conversation/session-tool-compact/assistant/02",
                    &"older assistant ".repeat(20),
                    MemoryCategory::Conversation,
                    Some("session-tool-compact"),
                    None,
                ),
            ]),
            recalled: Vec::new(),
            stored: Mutex::new(Vec::new()),
            forgotten: Mutex::new(Vec::new()),
        })),
        None,
        Some(configured_provider()),
        Some(provider.clone()),
        None,
        Some(ContextConfig {
            max_prompt_chars: Some(760),
            llm_summary_on_overflow: Some(false),
            ..context_config()
        }),
    );

    let response = service
        .chat(ChatRequest {
            prompt: "store this with bounded payload".to_string(),
            session_id: Some("session-tool-compact".to_string()),
            model: None,
            temperature: Some(0.0),
        })
        .await
        .expect("tool loop should succeed");

    assert_eq!(response.response, "final answer");
    let calls = provider.calls.lock();
    assert_eq!(calls.len(), 2);
    assert!(calls
        .iter()
        .all(|call| crate::service::compression::estimate_messages_chars(&call.messages) <= 760));
}

#[tokio::test]
async fn conversation_summary_get_returns_summary_item_for_session() {
    let memory = Arc::new(TestMemory {
        listed: Mutex::new(vec![entry(
            "conversation_summary/session-observe",
            "turn_count:8\nsummary_turn_count:8\n[Session summary]\nPreferences:\n- Prefer concise answers.",
            MemoryCategory::Conversation,
            Some("session-observe"),
            None,
        )]),
        recalled: Vec::new(),
        stored: Mutex::new(Vec::new()),
        forgotten: Mutex::new(Vec::new()),
    });
    let service = ConnectedIclawIcService::with_dependencies(
        Some(memory),
        None,
        Some(configured_provider()),
        None,
        Some("provider missing".to_string()),
        Some(context_config()),
    );

    let summary = service
        .conversation_summary_get(ConversationSummaryGetRequest {
            session_id: "session-observe".to_string(),
        })
        .await
        .expect("summary query should succeed")
        .expect("summary should exist");

    assert_eq!(summary.key, "conversation_summary/session-observe");
    assert!(summary.content.contains("[Session summary]"));
}

#[tokio::test]
async fn agent_observe_reports_workspace_core_and_session_state() {
    let memory = Arc::new(TestMemory {
        listed: Mutex::new(vec![
            entry(
                "workspace/AGENTS.md",
                "Always explain the flow.",
                MemoryCategory::Core,
                None,
                None,
            ),
            entry(
                "core/user_preferences/response_style",
                "Prefer concise answers.",
                MemoryCategory::Core,
                None,
                None,
            ),
            entry(
                "core/project_facts/runtime",
                "Runs as an ICP canister runtime.",
                MemoryCategory::Core,
                None,
                None,
            ),
            entry(
                "conversation_summary/session-observe",
                "turn_count:4\nsummary_turn_count:4\n[Session summary]\nFacts:\n- context carried forward",
                MemoryCategory::Conversation,
                Some("session-observe"),
                None,
            ),
            entry(
                "conversation/session-observe/user/1",
                "first user turn",
                MemoryCategory::Conversation,
                Some("session-observe"),
                None,
            ),
            entry(
                "conversation/session-observe/assistant/1",
                "first assistant turn",
                MemoryCategory::Conversation,
                Some("session-observe"),
                None,
            ),
        ]),
        recalled: Vec::new(),
        stored: Mutex::new(Vec::new()),
        forgotten: Mutex::new(Vec::new()),
    });
    let service = ConnectedIclawIcService::with_dependencies(
        Some(memory),
        None,
        Some(configured_provider()),
        None,
        Some("provider missing".to_string()),
        Some(context_config()),
    );

    let observation = service
        .agent_observe(AgentObserveRequest {
            session_id: Some("session-observe".to_string()),
        })
        .await
        .expect("observation query should succeed");

    assert_eq!(
        observation.workspace_keys,
        vec!["workspace/AGENTS.md".to_string()]
    );
    assert!(observation
        .core_keys
        .contains(&"core/user_preferences/response_style".to_string()));
    assert!(observation
        .core_keys
        .contains(&"core/project_facts/runtime".to_string()));
    assert_eq!(
        observation.conversation_summary_key,
        Some("conversation_summary/session-observe".to_string())
    );
    assert!(observation.conversation_summary_present);
    assert_eq!(observation.conversation_turn_count, 2);
    assert!(observation
        .auto_promoted_keys
        .contains(&"core/user_preferences/response_style".to_string()));
    assert!(observation.enable_auto_promote);
    assert!(observation.enable_conversation_summary);
    assert!(observation.tool_loop_enabled);
    assert_eq!(observation.history_limit, 8);
    assert_eq!(observation.max_tool_iterations, 3);
}

#[tokio::test]
async fn chat_refreshes_summary_after_history_is_pruned() {
    let mut config = context_config();
    config.history_limit = Some(2);
    let memory = Arc::new(TestMemory {
        listed: Mutex::new(vec![
            entry(
                "conversation_summary/session-summary",
                "turn_count:2\nsummary_turn_count:0\n[Session summary]\nFacts:\n- old summary",
                MemoryCategory::Conversation,
                Some("session-summary"),
                None,
            ),
            entry(
                "conversation/session-summary/user/one",
                "previous user turn",
                MemoryCategory::Conversation,
                Some("session-summary"),
                None,
            ),
            entry(
                "conversation/session-summary/assistant/one",
                "previous assistant turn",
                MemoryCategory::Conversation,
                Some("session-summary"),
                None,
            ),
        ]),
        recalled: Vec::new(),
        stored: Mutex::new(Vec::new()),
        forgotten: Mutex::new(Vec::new()),
    });
    let service = ConnectedIclawIcService::with_dependencies(
        Some(memory.clone()),
        None,
        Some(configured_provider()),
        Some(Arc::new(MockProvider {
            responses: Mutex::new(vec![Ok(ProviderChatResult {
                response: ProviderResponse {
                    text: Some("new assistant turn".to_string()),
                    tool_calls: Vec::new(),
                    usage: None,
                    reasoning_content: None,
                },
                model: Some("provider-model".to_string()),
            })]),
            calls: Mutex::new(Vec::new()),
        })),
        None,
        Some(config),
    );

    let _ = service
        .chat(ChatRequest {
            prompt: "new user turn".to_string(),
            session_id: Some("session-summary".to_string()),
            model: None,
            temperature: Some(0.0),
        })
        .await
        .expect("chat should succeed");

    let summary = memory
        .get("conversation_summary/session-summary")
        .await
        .expect("summary lookup should succeed")
        .expect("summary should exist");
    assert!(summary.content.contains("turn_count:4"));
    assert!(summary.content.contains("summary_turn_count:4"));
    assert!(summary.content.contains("new assistant turn"));
}

#[tokio::test]
async fn autosave_prunes_old_history_for_session() {
    let memory = Arc::new(TestMemory {
        listed: Mutex::new(vec![
            entry(
                "conversation/session-prune/user/01",
                "older user",
                MemoryCategory::Conversation,
                Some("session-prune"),
                None,
            ),
            entry(
                "conversation/session-prune/assistant/02",
                "older assistant",
                MemoryCategory::Conversation,
                Some("session-prune"),
                None,
            ),
            entry(
                "conversation/session-prune/user/03",
                "newer user",
                MemoryCategory::Conversation,
                Some("session-prune"),
                None,
            ),
        ]),
        recalled: Vec::new(),
        stored: Mutex::new(Vec::new()),
        forgotten: Mutex::new(Vec::new()),
    });
    let config = ContextConfig {
        history_limit: Some(2),
        ..context_config()
    };
    let memory_dyn: Arc<dyn Memory> = memory.clone();

    agent::autosave_turn(
        Some(&memory_dyn),
        Some("session-prune"),
        "assistant",
        "latest assistant",
        Some(&config),
    )
    .await
    .expect("autosave should succeed");

    let history = agent::load_history(Some(&memory_dyn), Some("session-prune"), Some(&config))
        .await
        .expect("history load should succeed");

    assert_eq!(history.len(), 2);
    assert!(history.iter().any(|message| matches!(
        message,
        ConversationMessage::Chat(chat) if chat.role == "user" && chat.content == "newer user"
    )));
    assert!(history.iter().any(|message| matches!(
        message,
        ConversationMessage::Chat(chat)
            if chat.role == "assistant" && chat.content == "latest assistant"
    )));
    let forgotten = memory.forgotten.lock();
    assert_eq!(forgotten.len(), 2);
    assert!(forgotten.contains(&"conversation/session-prune/user/01".to_string()));
    assert!(forgotten.contains(&"conversation/session-prune/assistant/02".to_string()));
}

#[tokio::test]
async fn autosave_without_session_id_skips_prune() {
    let memory = Arc::new(TestMemory {
        listed: Mutex::new(Vec::new()),
        recalled: Vec::new(),
        stored: Mutex::new(Vec::new()),
        forgotten: Mutex::new(Vec::new()),
    });
    let memory_dyn: Arc<dyn Memory> = memory.clone();

    agent::autosave_turn(
        Some(&memory_dyn),
        None,
        "user",
        "stateless",
        Some(&context_config()),
    )
    .await
    .expect("stateless autosave should be a no-op");

    assert!(memory.stored.lock().is_empty());
    assert!(memory.forgotten.lock().is_empty());
}

#[tokio::test]
async fn chat_auto_promotes_preference_once_for_session_scoped_chat() {
    let memory = Arc::new(TestMemory {
        listed: Mutex::new(Vec::new()),
        recalled: Vec::new(),
        stored: Mutex::new(Vec::new()),
        forgotten: Mutex::new(Vec::new()),
    });
    let service = ConnectedIclawIcService::with_dependencies(
        Some(memory.clone()),
        None,
        Some(configured_provider()),
        Some(Arc::new(MockProvider {
            responses: Mutex::new(vec![Ok(ProviderChatResult {
                response: ProviderResponse {
                    text: Some("Understood.".to_string()),
                    tool_calls: Vec::new(),
                    usage: None,
                    reasoning_content: None,
                },
                model: Some("provider-model".to_string()),
            })]),
            calls: Mutex::new(Vec::new()),
        })),
        None,
        Some(context_config()),
    );

    let _ = service
        .chat(ChatRequest {
            prompt: "Prefer concise answers with concrete implementation details.".to_string(),
            session_id: Some("session-promote".to_string()),
            model: None,
            temperature: Some(0.0),
        })
        .await
        .expect("chat should succeed");

    let stored = memory.stored.lock();
    assert!(stored.iter().any(|record| {
        record.key == "core/user_preferences/response_style"
            && record.category == MemoryCategory::Core
            && record.session_id.is_none()
    }));
}

#[tokio::test]
async fn chat_skips_duplicate_auto_promotion_content() {
    let memory = Arc::new(TestMemory {
        listed: Mutex::new(vec![entry(
            "core/user_preferences/response_style",
            "Prefer concise answers with concrete implementation details.",
            MemoryCategory::Core,
            None,
            None,
        )]),
        recalled: Vec::new(),
        stored: Mutex::new(Vec::new()),
        forgotten: Mutex::new(Vec::new()),
    });
    let service = ConnectedIclawIcService::with_dependencies(
        Some(memory.clone()),
        None,
        Some(configured_provider()),
        Some(Arc::new(MockProvider {
            responses: Mutex::new(vec![Ok(ProviderChatResult {
                response: ProviderResponse {
                    text: Some("Understood.".to_string()),
                    tool_calls: Vec::new(),
                    usage: None,
                    reasoning_content: None,
                },
                model: Some("provider-model".to_string()),
            })]),
            calls: Mutex::new(Vec::new()),
        })),
        None,
        Some(context_config()),
    );

    let _ = service
        .chat(ChatRequest {
            prompt: "Prefer concise answers with concrete implementation details.".to_string(),
            session_id: Some("session-promote".to_string()),
            model: None,
            temperature: Some(0.0),
        })
        .await
        .expect("chat should succeed");

    let stored = memory.stored.lock();
    assert!(!stored.iter().any(|record| {
        record.key == "core/user_preferences/response_style"
            && record.category == MemoryCategory::Core
    }));
}

#[tokio::test]
async fn chat_executes_tool_loop_and_retries_provider() {
    let memory = Arc::new(TestMemory {
        listed: Mutex::new(vec![entry(
            "workspace/AGENTS.md",
            "Use tools safely.",
            MemoryCategory::Core,
            None,
            None,
        )]),
        recalled: Vec::new(),
        stored: Mutex::new(Vec::new()),
        forgotten: Mutex::new(Vec::new()),
    });
    let provider = Arc::new(MockProvider {
        responses: Mutex::new(vec![
            Ok(ProviderChatResult {
                response: ProviderResponse {
                    text: Some("Let me check memory".to_string()),
                    tool_calls: vec![ToolCall {
                        id: "call-1".to_string(),
                        name: "memory_store".to_string(),
                        arguments: serde_json::json!({
                            "key": "note/tool",
                            "content": "stored from tool",
                            "session_id": "session-tool"
                        })
                        .to_string(),
                    }],
                    usage: None,
                    reasoning_content: None,
                },
                model: Some("provider-model".to_string()),
            }),
            Ok(ProviderChatResult {
                response: ProviderResponse {
                    text: Some("final answer".to_string()),
                    tool_calls: Vec::new(),
                    usage: None,
                    reasoning_content: None,
                },
                model: Some("provider-model".to_string()),
            }),
        ]),
        calls: Mutex::new(Vec::new()),
    });
    let service = ConnectedIclawIcService::with_dependencies(
        Some(memory.clone()),
        None,
        Some(configured_provider()),
        Some(provider.clone()),
        None,
        Some(context_config()),
    );

    let response = service
        .chat(ChatRequest {
            prompt: "store this".to_string(),
            session_id: Some("session-tool".to_string()),
            model: None,
            temperature: Some(0.2),
        })
        .await
        .expect("tool loop should succeed");

    assert_eq!(response.response, "final answer");
    let calls = provider.calls.lock();
    assert_eq!(calls.len(), 2);
    assert!(calls[1]
        .messages
        .iter()
        .any(|message| matches!(message, ConversationMessage::AssistantToolCalls { .. })));
    assert!(calls[1]
        .messages
        .iter()
        .any(|message| matches!(message, ConversationMessage::ToolResults(_))));
    assert!(memory
        .stored
        .lock()
        .iter()
        .any(|record| record.content == "stored from tool"));
}

#[tokio::test]
async fn chat_tool_loop_returns_structured_unknown_tool_error_to_provider() {
    let provider = Arc::new(MockProvider {
        responses: Mutex::new(vec![
            Ok(ProviderChatResult {
                response: ProviderResponse {
                    text: Some("trying tool".to_string()),
                    tool_calls: vec![ToolCall {
                        id: "call-unknown".to_string(),
                        name: "missing_tool".to_string(),
                        arguments: "{}".to_string(),
                    }],
                    usage: None,
                    reasoning_content: None,
                },
                model: Some("provider-model".to_string()),
            }),
            Ok(ProviderChatResult {
                response: ProviderResponse {
                    text: Some("recovered".to_string()),
                    tool_calls: Vec::new(),
                    usage: None,
                    reasoning_content: None,
                },
                model: Some("provider-model".to_string()),
            }),
        ]),
        calls: Mutex::new(Vec::new()),
    });
    let service = ConnectedIclawIcService::with_dependencies(
        Some(Arc::new(TestMemory {
            listed: Mutex::new(Vec::new()),
            recalled: Vec::new(),
            stored: Mutex::new(Vec::new()),
            forgotten: Mutex::new(Vec::new()),
        })),
        None,
        Some(configured_provider()),
        Some(provider.clone()),
        None,
        Some(context_config()),
    );

    let response = service
        .chat(ChatRequest {
            prompt: "do tool recovery".to_string(),
            session_id: Some("session-unknown-tool".to_string()),
            model: None,
            temperature: Some(0.0),
        })
        .await
        .expect("provider should recover from structured error");

    assert_eq!(response.response, "recovered");
    let calls = provider.calls.lock();
    let second_call = calls.last().expect("second provider call");
    assert!(second_call.messages.iter().any(|message| matches!(
        message,
        ConversationMessage::ToolResults(results)
            if results.iter().any(|result| result.content.contains("\"error_code\":\"unknown_tool\""))
    )));
}

#[tokio::test]
async fn chat_fails_fast_on_repeated_identical_failing_tool_call() {
    let provider = Arc::new(MockProvider {
        responses: Mutex::new(vec![
            Ok(ProviderChatResult {
                response: ProviderResponse {
                    text: Some("trying tool".to_string()),
                    tool_calls: vec![ToolCall {
                        id: "call-repeat-1".to_string(),
                        name: "missing_tool".to_string(),
                        arguments: "{\"key\":\"same\"}".to_string(),
                    }],
                    usage: None,
                    reasoning_content: None,
                },
                model: Some("provider-model".to_string()),
            }),
            Ok(ProviderChatResult {
                response: ProviderResponse {
                    text: Some("trying tool again".to_string()),
                    tool_calls: vec![ToolCall {
                        id: "call-repeat-2".to_string(),
                        name: "missing_tool".to_string(),
                        arguments: "{\"key\":\"same\"}".to_string(),
                    }],
                    usage: None,
                    reasoning_content: None,
                },
                model: Some("provider-model".to_string()),
            }),
        ]),
        calls: Mutex::new(Vec::new()),
    });
    let service = ConnectedIclawIcService::with_dependencies(
        Some(Arc::new(TestMemory {
            listed: Mutex::new(Vec::new()),
            recalled: Vec::new(),
            stored: Mutex::new(Vec::new()),
            forgotten: Mutex::new(Vec::new()),
        })),
        None,
        Some(configured_provider()),
        Some(provider.clone()),
        None,
        Some(context_config()),
    );

    let error = service
        .chat(ChatRequest {
            prompt: "repeat broken tool".to_string(),
            session_id: Some("session-repeat".to_string()),
            model: None,
            temperature: Some(0.0),
        })
        .await
        .expect_err("repeated failing tool call should fail fast");

    assert_eq!(error.code, ApiErrorCode::ProviderError.as_str());
    assert!(error.message.contains("repeated the same failing call"));
    assert_eq!(provider.calls.lock().len(), 2);
}

#[tokio::test]
async fn chat_fails_fast_when_tool_loop_exceeds_iteration_limit() {
    let service = ConnectedIclawIcService::with_dependencies(
        Some(Arc::new(TestMemory {
            listed: Mutex::new(vec![entry(
                "workspace/AGENTS.md",
                "Use tools safely.",
                MemoryCategory::Core,
                None,
                None,
            )]),
            recalled: Vec::new(),
            stored: Mutex::new(Vec::new()),
            forgotten: Mutex::new(Vec::new()),
        })),
        None,
        Some(configured_provider()),
        Some(Arc::new(MockProvider {
            responses: Mutex::new(vec![Ok(ProviderChatResult {
                response: ProviderResponse {
                    text: Some("still calling tools".to_string()),
                    tool_calls: vec![ToolCall {
                        id: "call-limit".to_string(),
                        name: "memory_recall".to_string(),
                        arguments: serde_json::json!({ "query": "hello" }).to_string(),
                    }],
                    usage: None,
                    reasoning_content: None,
                },
                model: Some("provider-model".to_string()),
            })]),
            calls: Mutex::new(Vec::new()),
        })),
        None,
        Some(context_config_with_limit(0)),
    );

    let error = service
        .chat(ChatRequest {
            prompt: "loop forever".to_string(),
            session_id: Some("session-limit".to_string()),
            model: None,
            temperature: Some(0.2),
        })
        .await
        .expect_err("tool loop should fail fast");

    assert_eq!(error.code, ApiErrorCode::ProviderError.as_str());
    assert!(error.message.contains("iteration limit"));
}

#[tokio::test]
async fn chat_returns_provider_error_when_upstream_fails() {
    let service = ConnectedIclawIcService::with_dependencies(
        Some(Arc::new(TestMemory {
            listed: Mutex::new(Vec::new()),
            recalled: Vec::new(),
            stored: Mutex::new(Vec::new()),
            forgotten: Mutex::new(Vec::new()),
        })),
        None,
        Some(configured_provider()),
        Some(Arc::new(MockProvider {
            responses: Mutex::new(vec![Err(
                "Host 'example.com' is not in the configured allowlist".to_string(),
            )]),
            calls: Mutex::new(Vec::new()),
        })),
        None,
        Some(context_config()),
    );

    let error = service
        .chat(ChatRequest {
            prompt: "hello".to_string(),
            session_id: None,
            model: Some("gpt-4o".to_string()),
            temperature: Some(0.2),
        })
        .await
        .expect_err("provider failure should surface as ApiError");

    assert_eq!(error.code, ApiErrorCode::ProviderError.as_str());
    assert!(error.message.contains("allowlist"));
}

#[tokio::test]
async fn chat_retries_transient_provider_failure_once() {
    let provider = Arc::new(MockProvider {
        responses: Mutex::new(vec![
            Err("temporary transport failure".to_string()),
            Ok(ProviderChatResult {
                response: ProviderResponse {
                    text: Some("retry success".to_string()),
                    tool_calls: Vec::new(),
                    usage: None,
                    reasoning_content: None,
                },
                model: Some("provider-model".to_string()),
            }),
        ]),
        calls: Mutex::new(Vec::new()),
    });
    let service = ConnectedIclawIcService::with_dependencies(
        Some(Arc::new(TestMemory {
            listed: Mutex::new(Vec::new()),
            recalled: Vec::new(),
            stored: Mutex::new(Vec::new()),
            forgotten: Mutex::new(Vec::new()),
        })),
        None,
        Some(configured_provider()),
        Some(provider.clone()),
        None,
        Some(context_config()),
    );

    let response = service
        .chat(ChatRequest {
            prompt: "hello".to_string(),
            session_id: Some("session-retry".to_string()),
            model: None,
            temperature: Some(0.2),
        })
        .await
        .expect("retry should recover");

    assert_eq!(response.response, "retry success");
    assert_eq!(provider.calls.lock().len(), 2);
}

#[tokio::test]
async fn health_reports_memory_not_ready_when_backend_init_fails() {
    let service = ConnectedIclawIcService::with_dependencies(
        None,
        Some("sqlite init failed".to_string()),
        None,
        None,
        None,
        Some(context_config()),
    );

    let health = service.health().await;
    assert!(!health.memory_ready);
    assert_eq!(health.status, "degraded");
}

#[tokio::test]
async fn chat_keeps_http_request_tool_when_memory_is_unavailable() {
    let provider = Arc::new(MockProvider {
        responses: Mutex::new(vec![
            Ok(ProviderChatResult {
                response: ProviderResponse {
                    text: Some("checking".to_string()),
                    tool_calls: vec![ToolCall {
                        id: "call-http".to_string(),
                        name: "http_request".to_string(),
                        arguments: serde_json::json!({
                            "url": "https://api.openai.com/v1/models",
                            "method": "GET"
                        })
                        .to_string(),
                    }],
                    usage: None,
                    reasoning_content: None,
                },
                model: Some("provider-model".to_string()),
            }),
            Ok(ProviderChatResult {
                response: ProviderResponse {
                    text: Some("final answer".to_string()),
                    tool_calls: Vec::new(),
                    usage: None,
                    reasoning_content: None,
                },
                model: Some("provider-model".to_string()),
            }),
        ]),
        calls: Mutex::new(Vec::new()),
    });
    let service = ConnectedIclawIcService::with_dependencies(
        None,
        Some("sqlite init failed".to_string()),
        Some(configured_provider()),
        Some(provider.clone()),
        None,
        Some(context_config()),
    );

    let response = service
        .chat(ChatRequest {
            prompt: "check provider host".to_string(),
            session_id: Some("session-http".to_string()),
            model: None,
            temperature: Some(0.2),
        })
        .await
        .expect("http_request should remain available without memory");

    assert_eq!(response.response, "final answer");
    let calls = provider.calls.lock();
    assert_eq!(calls.len(), 2);
    assert_eq!(calls[0].tool_names, vec!["http_request".to_string()]);
    assert!(calls[1]
        .messages
        .iter()
        .any(|message| matches!(message, ConversationMessage::ToolResults(_))));
}
