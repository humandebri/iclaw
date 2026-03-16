//! where: iclaw/canister/src/service/tests.rs
//! what: service-level tests for provider readiness, history/autosave, and tool-loop behavior
//! why: validate the canister-facing agent flow without overloading service.rs

use super::*;
use crate::provider::{IcCanisterProvider, ProviderChatResult};
use crate::types::{
    AgentDraft, RunCreateRequest, RunResumeRequest, ScheduleCreateRequest, ScheduleDraft,
    ScheduleGetRequest, ScheduleUpdateRequest, ToolPolicyUpdateRequest, WebhookCreateRequest,
    WebhookDraft, WebhookGetRequest, WebhookInvokeRequest, WebhookRejectionsListRequest,
    WebhookSecretRotateRequest, WebhookUpdateRequest,
};
use async_trait::async_trait;
use iclaw_core::memory::{Memory, MemoryCategory, MemoryEntry};
use iclaw_core::providers::{
    ChatResponse as ProviderResponse, ConversationMessage, ProviderCapabilities, ToolCall,
};
use parking_lot::{Condvar, Mutex};
use std::thread;

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

struct BlockingProvider {
    response: Mutex<Option<ProviderChatResult>>,
    calls: Mutex<Vec<ProviderCall>>,
    started: (Mutex<bool>, Condvar),
    released: (Mutex<bool>, Condvar),
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

#[async_trait(?Send)]
impl IcCanisterProvider for MockProvider {
    async fn chat(
        &self,
        messages: &[ConversationMessage],
        tools: Option<&[iclaw_core::tools::ToolSpec]>,
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

impl BlockingProvider {
    fn new(response: ProviderChatResult) -> Self {
        Self {
            response: Mutex::new(Some(response)),
            calls: Mutex::new(Vec::new()),
            started: (Mutex::new(false), Condvar::new()),
            released: (Mutex::new(false), Condvar::new()),
        }
    }

    fn wait_until_started(&self) {
        let mut started = self.started.0.lock();
        while !*started {
            self.started.1.wait(&mut started);
        }
    }

    fn release(&self) {
        let mut released = self.released.0.lock();
        *released = true;
        self.released.1.notify_all();
    }
}

#[async_trait(?Send)]
impl IcCanisterProvider for BlockingProvider {
    async fn chat(
        &self,
        messages: &[ConversationMessage],
        tools: Option<&[iclaw_core::tools::ToolSpec]>,
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

        {
            let mut started = self.started.0.lock();
            *started = true;
            self.started.1.notify_all();
        }

        let mut released = self.released.0.lock();
        while !*released {
            self.released.1.wait(&mut released);
        }

        self.response
            .lock()
            .take()
            .ok_or_else(|| anyhow::anyhow!("blocking provider response missing"))
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
        max_request_bytes_budget: None,
        llm_summary_on_overflow: None,
        llm_summary_model: None,
        llm_summary_max_chars: None,
        llm_summary_request_bytes_threshold: None,
    }
}

fn context_config_with_limit(max_tool_iterations: u64) -> ContextConfig {
    let mut config = context_config();
    config.max_tool_iterations = Some(max_tool_iterations);
    config
}

fn test_agent() -> Agent {
    test_agent_with_tools(&[
        "memory_store".to_string(),
        "memory_recall".to_string(),
        "memory_forget".to_string(),
        "http_request".to_string(),
    ])
}

fn test_agent_with_tools(tool_names: &[String]) -> Agent {
    control_plane::default_agent(tool_names)
}

fn empty_test_memory() -> Arc<TestMemory> {
    Arc::new(TestMemory {
        listed: Mutex::new(Vec::new()),
        recalled: Vec::new(),
        stored: Mutex::new(Vec::new()),
        forgotten: Mutex::new(Vec::new()),
    })
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
async fn run_create_preserves_cancelled_state_when_provider_finishes_later() {
    let memory = empty_test_memory();
    let memory_backend: Arc<dyn Memory> = memory.clone();
    let provider = Arc::new(BlockingProvider::new(ProviderChatResult {
        response: ProviderResponse {
            text: Some("late provider response".to_string()),
            tool_calls: Vec::new(),
            usage: None,
            reasoning_content: None,
        },
        model: Some("gpt-4o-mini".to_string()),
    }));
    let service = Arc::new(ConnectedIclawIcService::with_dependencies(
        Some(memory_backend.clone()),
        None,
        Some(configured_provider()),
        Some(provider.clone()),
        None,
        Some(context_config()),
    ));
    let service_for_thread = service.clone();

    let handle = thread::spawn(move || {
        tokio::runtime::Builder::new_current_thread()
            .build()
            .expect("runtime")
            .block_on(async move {
                service_for_thread
                    .run_create(RunCreateRequest {
                        agent_id: None,
                        session_id: Some("session-cancelled".to_string()),
                        prompt: "cancel me while running".to_string(),
                        model: None,
                        temperature: Some(0.0),
                    })
                    .await
            })
    });

    provider.wait_until_started();

    let run_id = runs::list_runs(Some(&memory_backend), Some("session-cancelled"), 10)
        .await
        .expect("list runs")
        .into_iter()
        .next()
        .expect("running run present")
        .id;

    let cancelled = service
        .run_cancel(RunCancelRequest {
            run_id: run_id.clone(),
        })
        .await
        .expect("cancel succeeds");
    assert!(cancelled);

    provider.release();

    let result = handle
        .join()
        .expect("thread join")
        .expect("run_create returns latest run");
    assert_eq!(result.status, "cancelled");
    assert_eq!(result.error.as_deref(), Some("run cancelled"));

    let persisted = runs::get_run(Some(&memory_backend), &run_id)
        .await
        .expect("load cancelled run")
        .expect("cancelled run exists");
    assert_eq!(persisted.status, "cancelled");
    assert_eq!(persisted.response, None);

    let events = runs::list_run_events(Some(&memory_backend), &run_id)
        .await
        .expect("list events");
    assert_eq!(
        events
            .iter()
            .map(|event| event.kind.as_str())
            .collect::<Vec<_>>(),
        vec!["queued", "started", "cancelled"]
    );

    let session = runs::get_session(Some(&memory_backend), "session-cancelled")
        .await
        .expect("load session")
        .expect("session exists");
    assert_eq!(session.last_run_id.as_deref(), Some(run_id.as_str()));
}

#[tokio::test]
async fn run_create_updates_session_metadata_after_failed_run() {
    let memory = empty_test_memory();
    let memory_backend: Arc<dyn Memory> = memory.clone();
    let service = ConnectedIclawIcService::with_dependencies(
        Some(memory_backend.clone()),
        None,
        None,
        None,
        None,
        Some(context_config()),
    );

    let run = service
        .run_create(RunCreateRequest {
            agent_id: None,
            session_id: Some("session-failed".to_string()),
            prompt: "provider missing".to_string(),
            model: None,
            temperature: Some(0.0),
        })
        .await
        .expect("failed run should still persist");
    assert_eq!(run.status, "failed");

    let session = runs::get_session(Some(&memory_backend), "session-failed")
        .await
        .expect("load session")
        .expect("session exists");
    assert_eq!(session.last_run_id.as_deref(), Some(run.id.as_str()));
    assert_ne!(session.updated_at, session.created_at);

    let sessions = runs::list_sessions(Some(&memory_backend), None)
        .await
        .expect("list sessions");
    assert_eq!(sessions[0].id, "session-failed");
    assert_eq!(sessions[0].last_run_id.as_deref(), Some(run.id.as_str()));
}

#[tokio::test]
async fn run_execution_builds_history_and_autosaves_turns() {
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
                "conversation/session-a/assistant/autosave",
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
        .execute_run(RunExecutionRequest {
            prompt: "Use AGENTS.md and the doc skill memory".to_string(),
            session_id: Some("session-a".to_string()),
            model: None,
            temperature: Some(0.2),
            agent: test_agent(),
        })
        .await
        .expect("configured provider should succeed");

    assert_eq!(response.response.as_deref(), Some("transport-ok"));
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
async fn run_execution_injects_cycle_warning_into_system_prompt_when_balance_is_low() {
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
        .execute_run(RunExecutionRequest {
            prompt: "hello".to_string(),
            session_id: Some("session-low-cycle".to_string()),
            model: None,
            temperature: Some(0.0),
            agent: test_agent(),
        })
        .await
        .expect("chat should succeed");

    assert_eq!(response.response.as_deref(), Some("warning acknowledged"));
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
async fn run_execution_injects_session_summary_before_recent_history() {
    let memory = Arc::new(TestMemory {
        listed: Mutex::new(vec![
            entry(
                "conversation_summary/session-summary",
                "turn_count:8\nsummary_turn_count:8\n[Session summary]\n- user: prior context matters",
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
        .execute_run(RunExecutionRequest {
            prompt: "use prior context".to_string(),
            session_id: Some("session-summary".to_string()),
            model: None,
            temperature: Some(0.0),
            agent: test_agent(),
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
async fn run_execution_compacts_payload_and_keeps_recent_history_under_budget() {
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
        .execute_run(RunExecutionRequest {
            prompt: "latest prompt".to_string(),
            session_id: Some("session-compact".to_string()),
            model: None,
            temperature: Some(0.0),
            agent: test_agent(),
        })
        .await
        .expect("chat should succeed");

    let call = provider
        .calls
        .lock()
        .last()
        .cloned()
        .expect("provider call");
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
async fn run_execution_compacts_when_chars_fit_but_request_bytes_do_not() {
    let memory = Arc::new(TestMemory {
        listed: Mutex::new(vec![
            entry(
                "conversation/session-bytes/user/01",
                &"圧縮対象".repeat(40),
                MemoryCategory::Conversation,
                Some("session-bytes"),
                None,
            ),
            entry(
                "conversation/session-bytes/assistant/02",
                &"以前の応答".repeat(40),
                MemoryCategory::Conversation,
                Some("session-bytes"),
                None,
            ),
            entry(
                "conversation/session-bytes/user/03",
                "latest prompt context",
                MemoryCategory::Conversation,
                Some("session-bytes"),
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
                text: Some("bytes-ok".to_string()),
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
            max_prompt_chars: Some(10_000),
            max_request_bytes_budget: Some(950),
            ..context_config()
        }),
    );

    let _ = service
        .execute_run(RunExecutionRequest {
            prompt: "latest prompt".to_string(),
            session_id: Some("session-bytes".to_string()),
            model: None,
            temperature: Some(0.0),
            agent: test_agent(),
        })
        .await
        .expect("chat should succeed");

    let call = provider
        .calls
        .lock()
        .last()
        .cloned()
        .expect("provider call");
    assert!(crate::service::compression::estimate_messages_chars(&call.messages) <= 10_000);
    assert!(
        crate::service::compression::estimate_request_bytes(&call.messages, None, &call.model, 0.0)
            <= 950
    );
    assert!(!call.messages.iter().any(|message| matches!(
        message,
        ConversationMessage::Chat(chat) if chat.content.contains("以前の応答以前の応答")
    )));
}

#[tokio::test]
async fn run_execution_recompacts_summary_without_llm_when_truncation_is_enough() {
    let memory = Arc::new(TestMemory {
        listed: Mutex::new(vec![
            entry(
                "conversation_summary/session-summary-trim",
                &format!(
                    "turn_count:12\nsummary_turn_count:12\n[Session summary]\n{}",
                    "summary ".repeat(120)
                ),
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
            max_request_bytes_budget: Some(4_000),
            llm_summary_model: Some("summary-model".to_string()),
            llm_summary_max_chars: Some(80),
            llm_summary_request_bytes_threshold: Some(4_000),
            ..context_config()
        }),
    );

    let _ = service
        .execute_run(RunExecutionRequest {
            prompt: "use compacted summary".to_string(),
            session_id: Some("session-summary-trim".to_string()),
            model: None,
            temperature: Some(0.0),
            agent: test_agent(),
        })
        .await
        .expect("chat should succeed");

    let calls = provider.calls.lock();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].model, "gpt-4o-mini");
    assert!(crate::service::compression::estimate_messages_chars(&calls[0].messages) <= 760);
    assert!(
        crate::service::compression::estimate_request_bytes(
            &calls[0].messages,
            None,
            &calls[0].model,
            0.0,
        ) <= 4_000
    );
}

#[tokio::test]
async fn run_execution_uses_llm_summary_once_and_persists_compacted_summary() {
    let memory = Arc::new(TestMemory {
        listed: Mutex::new(vec![
            entry(
                "conversation_summary/session-llm-summary",
                &format!(
                    "turn_count:16\nsummary_turn_count:16\n[Session summary]\n{}",
                    "older summary ".repeat(120)
                ),
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
            max_request_bytes_budget: Some(1_000),
            llm_summary_model: Some("summary-model".to_string()),
            llm_summary_max_chars: Some(80),
            llm_summary_request_bytes_threshold: Some(1_000),
            ..context_config()
        }),
    );

    let response = service
        .execute_run(RunExecutionRequest {
            prompt: "finish the response".to_string(),
            session_id: Some("session-llm-summary".to_string()),
            model: None,
            temperature: Some(0.0),
            agent: test_agent(),
        })
        .await
        .expect("chat should succeed");

    assert_eq!(response.response.as_deref(), Some("done"));
    let calls = provider.calls.lock();
    assert_eq!(calls.len(), 2);
    assert_eq!(calls[0].model, "summary-model");
    assert_eq!(calls[1].model, "gpt-4o-mini");
    assert!(
        crate::service::compression::estimate_request_bytes(
            &calls[1].messages,
            None,
            &calls[1].model,
            0.0,
        ) <= 1_000
    );
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
async fn run_execution_falls_back_when_llm_summary_generation_fails() {
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
                    &format!(
                        "turn_count:10\nsummary_turn_count:10\n[Session summary]\n{}",
                        "overflow ".repeat(100)
                    ),
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
            max_request_bytes_budget: Some(850),
            llm_summary_model: Some("summary-model".to_string()),
            llm_summary_max_chars: Some(60),
            llm_summary_request_bytes_threshold: Some(850),
            ..context_config()
        }),
    );

    let response = service
        .execute_run(RunExecutionRequest {
            prompt: "continue".to_string(),
            session_id: Some("session-llm-fail".to_string()),
            model: None,
            temperature: Some(0.0),
            agent: test_agent(),
        })
        .await
        .expect("chat should succeed");

    assert_eq!(response.response.as_deref(), Some("fallback-ok"));
    let calls = provider.calls.lock();
    assert_eq!(calls.len(), 2);
    assert_eq!(calls[0].model, "summary-model");
    assert_eq!(calls[1].model, "gpt-4o-mini");
    assert!(matches!(
        &calls[1].messages[0],
        ConversationMessage::Chat(chat)
            if chat.role == "system"
                && chat.content.contains("Earlier history was compacted.")
    ));
}

#[tokio::test]
async fn refresh_normalizes_compacted_summary_on_later_turn() {
    let memory = Arc::new(TestMemory {
        listed: Mutex::new(vec![
            entry(
                "conversation_summary/session-refresh-normalize",
                "turn_count:18\nsummary_turn_count:18\n[Compacted session summary]\nFacts:\n- compacted fact",
                MemoryCategory::Conversation,
                Some("session-refresh-normalize"),
                None,
            ),
            entry(
                "conversation/session-refresh-normalize/user/01",
                "older user",
                MemoryCategory::Conversation,
                Some("session-refresh-normalize"),
                None,
            ),
            entry(
                "conversation/session-refresh-normalize/assistant/02",
                "older assistant",
                MemoryCategory::Conversation,
                Some("session-refresh-normalize"),
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
                    text: Some("first".to_string()),
                    tool_calls: Vec::new(),
                    usage: None,
                    reasoning_content: None,
                },
                model: Some("provider-model".to_string()),
            }),
            Ok(ProviderChatResult {
                response: ProviderResponse {
                    text: Some("second".to_string()),
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
        Some(provider),
        None,
        Some(ContextConfig {
            max_prompt_chars: Some(10_000),
            max_request_bytes_budget: Some(8_000),
            llm_summary_on_overflow: Some(false),
            ..context_config()
        }),
    );

    for prompt in ["first turn", "second turn"] {
        let _ = service
            .execute_run(RunExecutionRequest {
                prompt: prompt.to_string(),
                session_id: Some("session-refresh-normalize".to_string()),
                model: None,
                temperature: Some(0.0),
                agent: test_agent(),
            })
            .await
            .expect("chat should succeed");
    }

    let summary = memory
        .get("conversation_summary/session-refresh-normalize")
        .await
        .expect("summary lookup should succeed")
        .expect("summary should exist");
    assert!(summary.content.contains("[Session summary]"));
    assert!(!summary.content.contains("[Compacted session summary]"));
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
        .execute_run(RunExecutionRequest {
            prompt: "store this with bounded payload".to_string(),
            session_id: Some("session-tool-compact".to_string()),
            model: None,
            temperature: Some(0.0),
            agent: test_agent(),
        })
        .await
        .expect("tool loop should succeed");

    assert_eq!(response.response.as_deref(), Some("final answer"));
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
async fn run_execution_refreshes_summary_after_history_is_pruned() {
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
        .execute_run(RunExecutionRequest {
            prompt: "new user turn".to_string(),
            session_id: Some("session-summary".to_string()),
            model: None,
            temperature: Some(0.0),
            agent: test_agent(),
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
async fn run_execution_auto_promotes_preference_once_for_session_scoped_run() {
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
        .execute_run(RunExecutionRequest {
            prompt: "Prefer concise answers with concrete implementation details.".to_string(),
            session_id: Some("session-promote".to_string()),
            model: None,
            temperature: Some(0.0),
            agent: test_agent(),
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
async fn run_execution_skips_duplicate_auto_promotion_content() {
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
        .execute_run(RunExecutionRequest {
            prompt: "Prefer concise answers with concrete implementation details.".to_string(),
            session_id: Some("session-promote".to_string()),
            model: None,
            temperature: Some(0.0),
            agent: test_agent(),
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
async fn run_execution_executes_tool_loop_and_retries_provider() {
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
        .execute_run(RunExecutionRequest {
            prompt: "store this".to_string(),
            session_id: Some("session-tool".to_string()),
            model: None,
            temperature: Some(0.2),
            agent: test_agent(),
        })
        .await
        .expect("tool loop should succeed");

    assert_eq!(response.response.as_deref(), Some("final answer"));
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
async fn run_execution_tool_loop_returns_structured_unknown_tool_error_to_provider() {
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
        .execute_run(RunExecutionRequest {
            prompt: "do tool recovery".to_string(),
            session_id: Some("session-unknown-tool".to_string()),
            model: None,
            temperature: Some(0.0),
            agent: test_agent_with_tools(&["missing_tool".to_string()]),
        })
        .await
        .expect("provider should recover from structured error");

    assert_eq!(response.response.as_deref(), Some("recovered"));
    let calls = provider.calls.lock();
    let second_call = calls.last().expect("second provider call");
    assert!(second_call.messages.iter().any(|message| matches!(
        message,
        ConversationMessage::ToolResults(results)
            if results.iter().any(|result| result.content.contains("\"error_code\":\"unknown_tool\""))
    )));
}

#[tokio::test]
async fn run_execution_fails_fast_on_repeated_identical_failing_tool_call() {
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
        .execute_run(RunExecutionRequest {
            prompt: "repeat broken tool".to_string(),
            session_id: Some("session-repeat".to_string()),
            model: None,
            temperature: Some(0.0),
            agent: test_agent_with_tools(&["missing_tool".to_string()]),
        })
        .await
        .expect_err("repeated failing tool call should fail fast");

    assert_eq!(error.code, ApiErrorCode::ProviderError.as_str());
    assert!(error.message.contains("repeated the same failing call"));
    assert_eq!(provider.calls.lock().len(), 2);
}

#[tokio::test]
async fn run_execution_fails_fast_when_tool_loop_exceeds_iteration_limit() {
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
        .execute_run(RunExecutionRequest {
            prompt: "loop forever".to_string(),
            session_id: Some("session-limit".to_string()),
            model: None,
            temperature: Some(0.2),
            agent: test_agent(),
        })
        .await
        .expect_err("tool loop should fail fast");

    assert_eq!(error.code, ApiErrorCode::ProviderError.as_str());
    assert!(error.message.contains("iteration limit"));
}

#[tokio::test]
async fn run_execution_returns_provider_error_when_upstream_fails() {
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
        .execute_run(RunExecutionRequest {
            prompt: "hello".to_string(),
            session_id: None,
            model: Some("gpt-4o".to_string()),
            temperature: Some(0.2),
            agent: test_agent(),
        })
        .await
        .expect_err("provider failure should surface as ApiError");

    assert_eq!(error.code, ApiErrorCode::ProviderError.as_str());
    assert!(error.message.contains("allowlist"));
}

#[tokio::test]
async fn run_execution_retries_transient_provider_failure_once() {
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
        .execute_run(RunExecutionRequest {
            prompt: "hello".to_string(),
            session_id: Some("session-retry".to_string()),
            model: None,
            temperature: Some(0.2),
            agent: test_agent(),
        })
        .await
        .expect("retry should recover");

    assert_eq!(response.response.as_deref(), Some("retry success"));
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
async fn run_execution_keeps_http_request_tool_when_memory_is_unavailable() {
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
        .execute_run(RunExecutionRequest {
            prompt: "check provider host".to_string(),
            session_id: Some("session-http".to_string()),
            model: None,
            temperature: Some(0.2),
            agent: test_agent(),
        })
        .await
        .expect("http_request should remain available without memory");

    assert_eq!(response.response.as_deref(), Some("final answer"));
    let calls = provider.calls.lock();
    assert_eq!(calls.len(), 2);
    assert_eq!(calls[0].tool_names, vec!["http_request".to_string()]);
    assert!(calls[1]
        .messages
        .iter()
        .any(|message| matches!(message, ConversationMessage::ToolResults(_))));
}

#[tokio::test]
async fn webhook_rejection_retention_keeps_latest_hundred_entries() {
    let memory = empty_test_memory();
    let memory_backend: Arc<dyn Memory> = memory.clone();
    let webhook = Webhook::from(WebhookDraft {
        id: "retention-hook".to_string(),
        name: "Retention Hook".to_string(),
        agent_id: "default".to_string(),
        session_mode: "create_new".to_string(),
        fixed_session_id: None,
        secret: "secret".to_string(),
        enabled: true,
    });

    webhooks::create_webhook(
        &memory_backend,
        WebhookCreateRequest {
            draft: WebhookDraft {
                id: webhook.id.clone(),
                name: webhook.name.clone(),
                agent_id: webhook.agent_id.clone(),
                session_mode: webhook.session_mode.clone(),
                fixed_session_id: webhook.fixed_session_id.clone(),
                secret: webhook.secret.clone(),
                enabled: webhook.enabled,
            },
        },
    )
    .await
    .expect("create webhook");

    let mut latest = webhook.clone();
    for index in 0..105 {
        latest = webhooks::record_rejected_invoke(
            &memory_backend,
            &latest,
            &format!("reject-{index:03}"),
        )
        .await
        .expect("record rejection");
    }

    let stored = webhooks::list_rejections(
        Some(&memory_backend),
        &WebhookRejectionsListRequest {
            webhook_id: webhook.id.clone(),
            limit: Some(200),
        },
    )
    .await
    .expect("list retained rejections");
    assert_eq!(stored.len(), 100);
    assert_eq!(stored[0].reason, "reject-104");
    assert_eq!(
        stored.last().map(|entry| entry.reason.as_str()),
        Some("reject-005")
    );

    let limited = webhooks::list_rejections(
        Some(&memory_backend),
        &WebhookRejectionsListRequest {
            webhook_id: webhook.id.clone(),
            limit: Some(5),
        },
    )
    .await
    .expect("list top five");
    assert_eq!(limited.len(), 5);
    assert_eq!(limited[0].reason, "reject-104");
    assert_eq!(limited[4].reason, "reject-100");

    assert_eq!(latest.last_rejection_reason.as_deref(), Some("reject-104"));
    assert!(latest.last_rejection_at.is_some());

    let forgotten = memory.forgotten.lock();
    assert_eq!(forgotten.len(), 5);
}

#[tokio::test]
async fn ensure_session_rejects_existing_session_with_different_agent() {
    let memory = empty_test_memory();
    let memory_backend: Arc<dyn Memory> = memory.clone();

    let existing =
        runs::ensure_session(&memory_backend, "agent-a", Some("shared-session"), "hello")
            .await
            .expect("create session");
    assert_eq!(existing.agent_id, "agent-a");

    let error = runs::ensure_session(
        &memory_backend,
        "agent-b",
        Some("shared-session"),
        "hello again",
    )
    .await
    .expect_err("mismatched agent should fail");
    assert!(error
        .to_string()
        .contains("session agent_id does not match the requested agent_id"));
}

#[tokio::test]
async fn webhook_update_preserves_server_managed_fields() {
    let memory = empty_test_memory();
    let memory_backend: Arc<dyn Memory> = memory.clone();

    let created = webhooks::create_webhook(
        &memory_backend,
        WebhookCreateRequest {
            draft: WebhookDraft {
                id: "daily-brief".to_string(),
                name: "Daily Brief".to_string(),
                agent_id: "default".to_string(),
                session_mode: "create_new".to_string(),
                fixed_session_id: None,
                secret: "secret-1".to_string(),
                enabled: true,
            },
        },
    )
    .await
    .expect("create webhook");
    let with_run = webhooks::touch_last_run(&memory_backend, &created, "run-1")
        .await
        .expect("touch run");
    let with_rejection =
        webhooks::record_rejected_invoke(&memory_backend, &with_run, "webhook secret is invalid")
            .await
            .expect("record rejection");

    let updated = webhooks::update_webhook(
        &memory_backend,
        WebhookUpdateRequest {
            webhook: Webhook {
                id: with_rejection.id.clone(),
                name: "Updated Daily Brief".to_string(),
                agent_id: "default".to_string(),
                session_mode: "create_new".to_string(),
                fixed_session_id: None,
                secret: "********".to_string(),
                enabled: false,
                created_at: "stale-created-at".to_string(),
                updated_at: "stale-updated-at".to_string(),
                last_run_id: None,
                last_secret_rotated_at: None,
                last_invoked_at: None,
                last_rejection_at: None,
                last_rejection_reason: None,
            },
            secret_override: Some("secret-2".to_string()),
        },
    )
    .await
    .expect("update webhook");

    assert_eq!(updated.name, "Updated Daily Brief");
    assert_eq!(updated.secret, "secret-2");
    assert!(!updated.enabled);
    assert_eq!(updated.created_at, created.created_at);
    assert_eq!(updated.last_run_id.as_deref(), Some("run-1"));
    assert_eq!(
        updated.last_rejection_reason.as_deref(),
        Some("webhook secret is invalid")
    );
    assert!(updated.last_rejection_at.is_some());
}

#[tokio::test]
async fn schedule_crud_preserves_next_run_for_metadata_only_updates() {
    let memory = empty_test_memory();
    let memory_backend: Arc<dyn Memory> = memory.clone();

    let created = schedules::create_schedule(
        &memory_backend,
        ScheduleCreateRequest {
            draft: ScheduleDraft {
                id: "hourly-brief".to_string(),
                name: "Hourly Brief".to_string(),
                agent_id: "default".to_string(),
                prompt: "brief me".to_string(),
                interval_minutes: 60,
                session_mode: "create_new".to_string(),
                fixed_session_id: None,
                enabled: true,
            },
        },
    )
    .await
    .expect("create schedule");
    assert!(created.next_run_at.is_some());
    assert_eq!(created.consecutive_failure_count, 0);
    assert_eq!(created.last_success_at, None);

    let listed = schedules::list_schedules(Some(&memory_backend))
        .await
        .expect("list schedules");
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].id, "hourly-brief");

    let updated = schedules::update_schedule(
        &memory_backend,
        ScheduleUpdateRequest {
            schedule: Schedule {
                name: "Hourly Brief Renamed".to_string(),
                ..created.clone()
            },
        },
    )
    .await
    .expect("update schedule");
    assert_eq!(updated.name, "Hourly Brief Renamed");
    assert_eq!(updated.created_at, created.created_at);
    assert_eq!(updated.next_run_at, created.next_run_at);
}

#[tokio::test]
async fn schedule_crud_recomputes_next_run_when_interval_changes() {
    let memory = empty_test_memory();
    let memory_backend: Arc<dyn Memory> = memory.clone();

    let created = schedules::create_schedule(
        &memory_backend,
        ScheduleCreateRequest {
            draft: ScheduleDraft {
                id: "daily-brief".to_string(),
                name: "Daily Brief".to_string(),
                agent_id: "default".to_string(),
                prompt: "brief me".to_string(),
                interval_minutes: 60,
                session_mode: "create_new".to_string(),
                fixed_session_id: None,
                enabled: true,
            },
        },
    )
    .await
    .expect("create schedule");

    let updated = schedules::update_schedule(
        &memory_backend,
        ScheduleUpdateRequest {
            schedule: Schedule {
                interval_minutes: 30,
                ..created.clone()
            },
        },
    )
    .await
    .expect("update schedule");
    assert_eq!(updated.interval_minutes, 30);
    assert_eq!(updated.created_at, created.created_at);
    assert_ne!(updated.next_run_at, created.next_run_at);
}

#[tokio::test]
async fn schedule_update_clears_next_run_when_disabled() {
    let memory = empty_test_memory();
    let memory_backend: Arc<dyn Memory> = memory.clone();

    let created = schedules::create_schedule(
        &memory_backend,
        ScheduleCreateRequest {
            draft: ScheduleDraft {
                id: "disabled-brief".to_string(),
                name: "Disabled Brief".to_string(),
                agent_id: "default".to_string(),
                prompt: "brief me".to_string(),
                interval_minutes: 60,
                session_mode: "create_new".to_string(),
                fixed_session_id: None,
                enabled: true,
            },
        },
    )
    .await
    .expect("create schedule");

    let updated = schedules::update_schedule(
        &memory_backend,
        ScheduleUpdateRequest {
            schedule: Schedule {
                enabled: false,
                ..created
            },
        },
    )
    .await
    .expect("disable schedule");
    assert_eq!(updated.next_run_at, None);
}

#[tokio::test]
async fn schedule_create_rejects_duplicate_id() {
    let memory = empty_test_memory();
    let memory_backend: Arc<dyn Memory> = memory.clone();

    schedules::create_schedule(
        &memory_backend,
        ScheduleCreateRequest {
            draft: ScheduleDraft {
                id: "daily-brief".to_string(),
                name: "Daily Brief".to_string(),
                agent_id: "default".to_string(),
                prompt: "brief me".to_string(),
                interval_minutes: 60,
                session_mode: "create_new".to_string(),
                fixed_session_id: None,
                enabled: true,
            },
        },
    )
    .await
    .expect("seed schedule");

    let error = schedules::create_schedule(
        &memory_backend,
        ScheduleCreateRequest {
            draft: ScheduleDraft {
                id: "daily-brief".to_string(),
                name: "Conflicting Brief".to_string(),
                agent_id: "default".to_string(),
                prompt: "overwrite me".to_string(),
                interval_minutes: 5,
                session_mode: "create_new".to_string(),
                fixed_session_id: None,
                enabled: true,
            },
        },
    )
    .await
    .expect_err("duplicate id must fail");
    assert!(error.to_string().contains("schedule already exists"));
}

#[tokio::test]
async fn schedule_trigger_records_schedule_run_metadata() {
    let memory = empty_test_memory();
    let memory_backend: Arc<dyn Memory> = memory.clone();
    let service = ConnectedIclawIcService::with_dependencies(
        Some(memory_backend.clone()),
        None,
        None,
        None,
        None,
        None,
    );

    let created = service
        .schedule_create(ScheduleCreateRequest {
            draft: ScheduleDraft {
                id: "daily-brief".to_string(),
                name: "Daily Brief".to_string(),
                agent_id: "default".to_string(),
                prompt: "brief me".to_string(),
                interval_minutes: 60,
                session_mode: "create_new".to_string(),
                fixed_session_id: None,
                enabled: true,
            },
        })
        .await
        .expect("create schedule");

    let run = service
        .schedule_trigger(ScheduleGetRequest {
            schedule_id: created.id.clone(),
        })
        .await
        .expect("manual trigger should still return run");
    assert_eq!(run.trigger_kind, "schedule");
    assert_eq!(run.trigger_id.as_deref(), Some("daily-brief"));

    let fetched = schedules::get_schedule(
        Some(&memory_backend),
        &ScheduleGetRequest {
            schedule_id: created.id,
        },
    )
    .await
    .expect("get schedule")
    .expect("schedule exists");
    assert_eq!(fetched.last_run_id.as_deref(), Some(run.id.as_str()));
    assert!(fetched.last_finished_at.is_some());
    assert_eq!(fetched.consecutive_failure_count, 1);
    assert_eq!(fetched.last_success_at, None);
}

#[tokio::test]
async fn schedule_create_returns_invalid_argument_for_duplicate_id() {
    let memory = empty_test_memory();
    let memory_backend: Arc<dyn Memory> = memory.clone();
    let service = ConnectedIclawIcService::with_dependencies(
        Some(memory_backend),
        None,
        None,
        None,
        None,
        None,
    );

    service
        .schedule_create(ScheduleCreateRequest {
            draft: ScheduleDraft {
                id: "daily-brief".to_string(),
                name: "Daily Brief".to_string(),
                agent_id: "default".to_string(),
                prompt: "brief me".to_string(),
                interval_minutes: 60,
                session_mode: "create_new".to_string(),
                fixed_session_id: None,
                enabled: true,
            },
        })
        .await
        .expect("seed schedule");

    let error = service
        .schedule_create(ScheduleCreateRequest {
            draft: ScheduleDraft {
                id: "daily-brief".to_string(),
                name: "Duplicate Daily Brief".to_string(),
                agent_id: "default".to_string(),
                prompt: "brief me again".to_string(),
                interval_minutes: 60,
                session_mode: "create_new".to_string(),
                fixed_session_id: None,
                enabled: true,
            },
        })
        .await
        .expect_err("duplicate must fail");
    assert_eq!(error.code, ApiErrorCode::InvalidArgument.as_str());
    assert!(error.message.contains("schedule_id already exists"));
}

#[tokio::test]
async fn schedule_trigger_allows_manual_run_while_disabled() {
    let memory = empty_test_memory();
    let memory_backend: Arc<dyn Memory> = memory.clone();
    let provider = Arc::new(MockProvider {
        responses: Mutex::new(vec![Ok(ProviderChatResult {
            response: ProviderResponse {
                text: Some("scheduled-ok".to_string()),
                tool_calls: Vec::new(),
                usage: None,
                reasoning_content: None,
            },
            model: Some("gpt-4o-mini".to_string()),
        })]),
        calls: Mutex::new(Vec::new()),
    });
    let service = ConnectedIclawIcService::with_dependencies(
        Some(memory_backend.clone()),
        None,
        Some(configured_provider()),
        Some(provider),
        None,
        None,
    );

    service
        .schedule_create(ScheduleCreateRequest {
            draft: ScheduleDraft {
                id: "disabled-manual".to_string(),
                name: "Disabled Manual".to_string(),
                agent_id: "default".to_string(),
                prompt: "brief me".to_string(),
                interval_minutes: 60,
                session_mode: "create_new".to_string(),
                fixed_session_id: None,
                enabled: false,
            },
        })
        .await
        .expect("create disabled schedule");

    let run = service
        .schedule_trigger(ScheduleGetRequest {
            schedule_id: "disabled-manual".to_string(),
        })
        .await
        .expect("disabled schedule should still allow manual trigger");
    assert_eq!(run.trigger_kind, "schedule");
    assert_eq!(run.trigger_id.as_deref(), Some("disabled-manual"));
    assert_eq!(run.status, "completed");

    let fetched = schedules::get_schedule(
        Some(&memory_backend),
        &ScheduleGetRequest {
            schedule_id: "disabled-manual".to_string(),
        },
    )
    .await
    .expect("get schedule")
    .expect("schedule exists");
    assert_eq!(fetched.consecutive_failure_count, 0);
    assert!(fetched.last_success_at.is_some());
}

#[tokio::test]
async fn schedule_success_resets_failure_counter_and_sets_last_success_at() {
    let memory = empty_test_memory();
    let memory_backend: Arc<dyn Memory> = memory.clone();
    let failing_service = ConnectedIclawIcService::with_dependencies(
        Some(memory_backend.clone()),
        None,
        None,
        None,
        None,
        None,
    );

    failing_service
        .schedule_create(ScheduleCreateRequest {
            draft: ScheduleDraft {
                id: "eventual-success".to_string(),
                name: "Eventual Success".to_string(),
                agent_id: "default".to_string(),
                prompt: "brief me".to_string(),
                interval_minutes: 60,
                session_mode: "create_new".to_string(),
                fixed_session_id: None,
                enabled: true,
            },
        })
        .await
        .expect("create schedule");
    failing_service
        .schedule_trigger(ScheduleGetRequest {
            schedule_id: "eventual-success".to_string(),
        })
        .await
        .expect("failed run is still returned");

    let provider = Arc::new(MockProvider {
        responses: Mutex::new(vec![Ok(ProviderChatResult {
            response: ProviderResponse {
                text: Some("scheduled-ok".to_string()),
                tool_calls: Vec::new(),
                usage: None,
                reasoning_content: None,
            },
            model: Some("gpt-4o-mini".to_string()),
        })]),
        calls: Mutex::new(Vec::new()),
    });
    let succeeding_service = ConnectedIclawIcService::with_dependencies(
        Some(memory_backend.clone()),
        None,
        Some(configured_provider()),
        Some(provider),
        None,
        Some(context_config()),
    );

    let run = succeeding_service
        .schedule_trigger(ScheduleGetRequest {
            schedule_id: "eventual-success".to_string(),
        })
        .await
        .expect("second trigger succeeds");
    assert_eq!(run.status, "completed");

    let fetched = schedules::get_schedule(
        Some(&memory_backend),
        &ScheduleGetRequest {
            schedule_id: "eventual-success".to_string(),
        },
    )
    .await
    .expect("get schedule")
    .expect("schedule exists");
    assert_eq!(fetched.consecutive_failure_count, 0);
    assert!(fetched.last_success_at.is_some());
}

#[tokio::test]
async fn schedule_trigger_rejects_fixed_session_for_another_agent() {
    let memory = empty_test_memory();
    let memory_backend: Arc<dyn Memory> = memory.clone();
    let service = ConnectedIclawIcService::with_dependencies(
        Some(memory_backend.clone()),
        None,
        None,
        None,
        None,
        None,
    );

    service
        .agent_create(AgentCreateRequest {
            draft: AgentDraft {
                id: "secondary".to_string(),
                name: "Secondary".to_string(),
                description: "secondary".to_string(),
                enabled_tool_names: vec![],
                requires_tool_approval: false,
                system_prompt_override: None,
                status: "active".to_string(),
            },
        })
        .await
        .expect("create agent");

    runs::ensure_session(&memory_backend, "default", Some("shared-schedule"), "seed")
        .await
        .expect("seed fixed session");

    service
        .schedule_create(ScheduleCreateRequest {
            draft: ScheduleDraft {
                id: "fixed-secondary".to_string(),
                name: "Fixed Secondary".to_string(),
                agent_id: "secondary".to_string(),
                prompt: "run".to_string(),
                interval_minutes: 5,
                session_mode: "reuse_fixed".to_string(),
                fixed_session_id: Some("shared-schedule".to_string()),
                enabled: true,
            },
        })
        .await
        .expect("create schedule");

    let error = service
        .schedule_trigger(ScheduleGetRequest {
            schedule_id: "fixed-secondary".to_string(),
        })
        .await
        .expect_err("mismatched agent should fail");
    assert_eq!(error.code, ApiErrorCode::InvalidArgument.as_str());
    assert!(error
        .message
        .contains("session agent_id must match schedule agent_id"));
}

#[tokio::test]
async fn schedule_fire_skips_when_previous_execution_is_still_running() {
    let memory = empty_test_memory();
    let memory_backend: Arc<dyn Memory> = memory.clone();
    let service = ConnectedIclawIcService::with_dependencies(
        Some(memory_backend.clone()),
        None,
        None,
        None,
        None,
        None,
    );

    let created = service
        .schedule_create(ScheduleCreateRequest {
            draft: ScheduleDraft {
                id: "running-schedule".to_string(),
                name: "Running Schedule".to_string(),
                agent_id: "default".to_string(),
                prompt: "run".to_string(),
                interval_minutes: 5,
                session_mode: "create_new".to_string(),
                fixed_session_id: None,
                enabled: true,
            },
        })
        .await
        .expect("create schedule");
    let running = schedules::mark_schedule_running(&memory_backend, &created)
        .await
        .expect("mark running");

    service
        .schedule_fire(running.id.clone())
        .await
        .expect("timer fire should not propagate");

    let fetched = schedules::get_schedule(
        Some(&memory_backend),
        &ScheduleGetRequest {
            schedule_id: running.id,
        },
    )
    .await
    .expect("get schedule")
    .expect("schedule exists");
    assert!(fetched.running);
    assert_eq!(
        fetched.last_error.as_deref(),
        Some("schedule skipped because the previous execution is still running")
    );
    assert!(fetched.next_run_at.is_some());
    assert_eq!(fetched.consecutive_failure_count, 1);
    assert_eq!(fetched.last_success_at, None);
}

#[tokio::test]
async fn run_resume_continues_a_blocked_run_with_pending_tool_calls() {
    let memory = empty_test_memory();
    let provider = Arc::new(MockProvider {
        responses: Mutex::new(vec![
            Ok(ProviderChatResult {
                response: ProviderResponse {
                    text: Some("let me store that".to_string()),
                    tool_calls: vec![ToolCall {
                        id: "call-guarded".to_string(),
                        name: "memory_store".to_string(),
                        arguments: serde_json::json!({
                            "key": "note/guarded",
                            "content": "approved",
                            "session_id": "guarded-session"
                        })
                        .to_string(),
                    }],
                    usage: None,
                    reasoning_content: Some("first pass".to_string()),
                },
                model: Some("provider-model".to_string()),
            }),
            Ok(ProviderChatResult {
                response: ProviderResponse {
                    text: Some("stored after approval".to_string()),
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

    service
        .agent_create(AgentCreateRequest {
            draft: AgentDraft {
                id: "guarded".to_string(),
                name: "Guarded".to_string(),
                description: "approval flow".to_string(),
                enabled_tool_names: vec!["memory_store".to_string()],
                requires_tool_approval: false,
                system_prompt_override: None,
                status: "active".to_string(),
            },
        })
        .await
        .expect("create guarded agent");
    service
        .tool_policy_update(ToolPolicyUpdateRequest {
            policy: ToolPolicy {
                agent_id: "guarded".to_string(),
                tool_name: "memory_store".to_string(),
                enabled: true,
                requires_approval: true,
            },
        })
        .await
        .expect("require approval");

    let blocked = service
        .run_create(RunCreateRequest {
            agent_id: Some("guarded".to_string()),
            session_id: Some("guarded-session".to_string()),
            prompt: "store this after approval".to_string(),
            model: None,
            temperature: Some(0.2),
        })
        .await
        .expect("blocked run");
    assert_eq!(blocked.status, "blocked");
    assert_eq!(blocked.pending_tool_calls.len(), 1);
    assert_eq!(
        blocked.pending_assistant_text.as_deref(),
        Some("let me store that")
    );

    service
        .tool_policy_update(ToolPolicyUpdateRequest {
            policy: ToolPolicy {
                agent_id: "guarded".to_string(),
                tool_name: "memory_store".to_string(),
                enabled: true,
                requires_approval: false,
            },
        })
        .await
        .expect("approve tool");

    let resumed = service
        .run_resume(RunResumeRequest {
            run_id: blocked.id.clone(),
        })
        .await
        .expect("resume run");
    assert_eq!(resumed.id, blocked.id);
    assert_eq!(resumed.status, "completed");
    assert_eq!(resumed.response.as_deref(), Some("stored after approval"));
    assert!(resumed.pending_tool_calls.is_empty());

    let events = service
        .run_events_get(RunEventsGetRequest {
            run_id: blocked.id.clone(),
        })
        .await
        .expect("load events");
    let event_kinds = events
        .into_iter()
        .map(|event| event.kind)
        .collect::<Vec<_>>();
    assert_eq!(
        event_kinds,
        vec![
            "queued",
            "started",
            "tool_requested",
            "tool_blocked",
            "blocked",
            "approved",
            "resumed",
            "tool_succeeded",
            "assistant_message",
            "completed",
        ]
    );

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
}

#[tokio::test]
async fn run_resume_rechecks_tool_policy_before_executing_pending_calls() {
    let memory = empty_test_memory();
    let provider = Arc::new(MockProvider {
        responses: Mutex::new(vec![Ok(ProviderChatResult {
            response: ProviderResponse {
                text: Some("let me store that".to_string()),
                tool_calls: vec![ToolCall {
                    id: "call-guarded".to_string(),
                    name: "memory_store".to_string(),
                    arguments: serde_json::json!({
                        "key": "note/guarded",
                        "content": "approved",
                        "session_id": "guarded-session"
                    })
                    .to_string(),
                }],
                usage: None,
                reasoning_content: Some("first pass".to_string()),
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

    service
        .agent_create(AgentCreateRequest {
            draft: AgentDraft {
                id: "guarded".to_string(),
                name: "Guarded".to_string(),
                description: "approval flow".to_string(),
                enabled_tool_names: vec!["memory_store".to_string()],
                requires_tool_approval: false,
                system_prompt_override: None,
                status: "active".to_string(),
            },
        })
        .await
        .expect("create guarded agent");
    service
        .tool_policy_update(ToolPolicyUpdateRequest {
            policy: ToolPolicy {
                agent_id: "guarded".to_string(),
                tool_name: "memory_store".to_string(),
                enabled: true,
                requires_approval: true,
            },
        })
        .await
        .expect("require approval");

    let blocked = service
        .run_create(RunCreateRequest {
            agent_id: Some("guarded".to_string()),
            session_id: Some("guarded-session".to_string()),
            prompt: "store this after approval".to_string(),
            model: None,
            temperature: Some(0.2),
        })
        .await
        .expect("blocked run");
    assert_eq!(blocked.status, "blocked");

    service
        .tool_policy_update(ToolPolicyUpdateRequest {
            policy: ToolPolicy {
                agent_id: "guarded".to_string(),
                tool_name: "memory_store".to_string(),
                enabled: false,
                requires_approval: false,
            },
        })
        .await
        .expect("disable tool");

    let resumed = service
        .run_resume(RunResumeRequest {
            run_id: blocked.id.clone(),
        })
        .await
        .expect("resume returns blocked run");
    assert_eq!(resumed.status, "blocked");
    assert_eq!(resumed.pending_tool_calls.len(), 1);
    assert_eq!(
        resumed.error.as_deref(),
        Some("tool 'memory_store' is disabled by policy")
    );

    let calls = provider.calls.lock();
    assert_eq!(calls.len(), 1);
}

#[tokio::test]
async fn webhook_rotate_secret_replaces_old_secret_and_tracks_invocation() {
    let memory = empty_test_memory();
    let provider = Arc::new(MockProvider {
        responses: Mutex::new(vec![Ok(ProviderChatResult {
            response: ProviderResponse {
                text: Some("webhook ok".to_string()),
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
        Some(provider),
        None,
        Some(context_config()),
    );

    let created = service
        .webhook_create(WebhookCreateRequest {
            draft: WebhookDraft {
                id: "rotate-me".to_string(),
                name: "Rotate Me".to_string(),
                agent_id: "default".to_string(),
                session_mode: "create_new".to_string(),
                fixed_session_id: None,
                secret: "secret-1".to_string(),
                enabled: true,
            },
        })
        .await
        .expect("create webhook");

    let rotated = service
        .webhook_rotate_secret(WebhookSecretRotateRequest {
            webhook_id: created.id.clone(),
        })
        .await
        .expect("rotate secret");
    assert_ne!(rotated.new_secret, "secret-1");
    assert!(rotated.webhook.last_secret_rotated_at.is_some());
    assert_ne!(rotated.webhook.secret, rotated.new_secret);

    let old_secret_error = service
        .webhook_invoke(WebhookInvokeRequest {
            webhook_id: created.id.clone(),
            secret: "secret-1".to_string(),
            prompt: "call with old secret".to_string(),
            session_id: None,
            model: None,
            temperature: None,
        })
        .await
        .expect_err("old secret must fail");
    assert_eq!(old_secret_error.code, ApiErrorCode::Unauthorized.as_str());

    let run = service
        .webhook_invoke(WebhookInvokeRequest {
            webhook_id: created.id.clone(),
            secret: rotated.new_secret.clone(),
            prompt: "call with new secret".to_string(),
            session_id: None,
            model: None,
            temperature: None,
        })
        .await
        .expect("new secret works");
    assert_eq!(run.status, "completed");

    let fetched = service
        .webhook_get(WebhookGetRequest {
            webhook_id: created.id,
        })
        .await
        .expect("reload webhook")
        .expect("webhook exists");
    assert!(fetched.last_secret_rotated_at.is_some());
    assert!(fetched.last_invoked_at.is_some());
}

#[tokio::test]
async fn webhook_create_returns_invalid_argument_for_duplicate_id() {
    let memory = empty_test_memory();
    let service =
        ConnectedIclawIcService::with_dependencies(Some(memory), None, None, None, None, None);

    service
        .webhook_create(WebhookCreateRequest {
            draft: WebhookDraft {
                id: "incoming-alerts".to_string(),
                name: "Incoming Alerts".to_string(),
                agent_id: "default".to_string(),
                session_mode: "create_new".to_string(),
                fixed_session_id: None,
                secret: "secret-1".to_string(),
                enabled: true,
            },
        })
        .await
        .expect("seed webhook");

    let error = service
        .webhook_create(WebhookCreateRequest {
            draft: WebhookDraft {
                id: "incoming-alerts".to_string(),
                name: "Conflicting Alerts".to_string(),
                agent_id: "default".to_string(),
                session_mode: "create_new".to_string(),
                fixed_session_id: None,
                secret: "secret-2".to_string(),
                enabled: true,
            },
        })
        .await
        .expect_err("duplicate must fail");
    assert_eq!(error.code, ApiErrorCode::InvalidArgument.as_str());
    assert!(error.message.contains("webhook_id already exists"));
}

#[tokio::test]
async fn webhook_create_rejects_duplicate_id() {
    let memory = empty_test_memory();
    let memory_backend: Arc<dyn Memory> = memory.clone();

    webhooks::create_webhook(
        &memory_backend,
        WebhookCreateRequest {
            draft: WebhookDraft {
                id: "incoming-alerts".to_string(),
                name: "Incoming Alerts".to_string(),
                agent_id: "default".to_string(),
                session_mode: "create_new".to_string(),
                fixed_session_id: None,
                secret: "secret-1".to_string(),
                enabled: true,
            },
        },
    )
    .await
    .expect("seed webhook");

    let error = webhooks::create_webhook(
        &memory_backend,
        WebhookCreateRequest {
            draft: WebhookDraft {
                id: "incoming-alerts".to_string(),
                name: "Conflicting Alerts".to_string(),
                agent_id: "default".to_string(),
                session_mode: "create_new".to_string(),
                fixed_session_id: None,
                secret: "secret-2".to_string(),
                enabled: true,
            },
        },
    )
    .await
    .expect_err("duplicate id must fail");
    assert!(error.to_string().contains("webhook already exists"));
}
