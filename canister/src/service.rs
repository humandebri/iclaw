// where: iclaw/canister/src/service.rs
// what: Connected service layer behind the public ICP canister entrypoints
// why: Track C wires memory and HTTPS-outcall-backed provider behavior into the canister without exposing native-only surfaces

#[path = "service/agent.rs"]
mod agent;
#[path = "service/compression/mod.rs"]
mod compression;
#[path = "service/control_plane.rs"]
mod control_plane;
#[path = "service/observe.rs"]
mod observe;
#[path = "service/policies.rs"]
pub(crate) mod policies;
#[path = "service/schedule_runtime.rs"]
mod schedule_runtime;
#[path = "service/schedules.rs"]
mod schedules;
#[path = "service/runs.rs"]
mod runs;
#[path = "service/webhooks.rs"]
mod webhooks;

use crate::context;
use crate::memory::build_memory;
use crate::provider::{build_provider, IcCanisterProvider};
use crate::tools;
use crate::types::{
    Agent, AgentCreateRequest, AgentGetRequest, AgentObservation, AgentObserveRequest,
    AgentUpdateRequest, ApiError, ApiErrorCode, CanisterConfig, ContextConfig,
    ConversationSummaryGetRequest, HealthResponse, MemoryCountResult, MemoryForgetRequest,
    MemoryForgetResult, MemoryGetRequest, MemoryItem, MemoryListRequest, MemoryRecallRequest,
    MemoryStoreRequest, PendingToolCall, ProviderConfig, Run, RunCancelRequest,
    RunCreateRequest, RunEvent, RunEventsGetRequest, RunGetRequest, RunListRequest,
    RunResumeRequest, Schedule, ScheduleCreateRequest, ScheduleGetRequest, ScheduleUpdateRequest,
    Session, SessionGetRequest, ToolPolicy, ToolPolicyListRequest, ToolPolicyUpdateRequest,
    UnitResult, Webhook, WebhookCreateRequest, WebhookGetRequest, WebhookInvokeRequest,
    WebhookRejection, WebhookRejectionsListRequest, WebhookSecretRotateRequest,
    WebhookSecretRotateResponse, WebhookUpdateRequest,
};
use async_trait::async_trait;
use iclaw_core::memory::Memory;
use iclaw_core::tools::Tool;
use std::cell::RefCell;
use std::sync::Arc;

#[async_trait]
pub trait IclawIcService: Send + Sync {
    async fn health(&self) -> HealthResponse;
    async fn agents_list(&self) -> Result<Vec<Agent>, ApiError>;
    async fn agent_get(&self, request: AgentGetRequest) -> Result<Option<Agent>, ApiError>;
    async fn agent_create(&self, request: AgentCreateRequest) -> Result<Agent, ApiError>;
    async fn agent_update(&self, request: AgentUpdateRequest) -> Result<Agent, ApiError>;
    async fn tool_policy_list(
        &self,
        request: ToolPolicyListRequest,
    ) -> Result<Vec<ToolPolicy>, ApiError>;
    async fn tool_policy_update(
        &self,
        request: ToolPolicyUpdateRequest,
    ) -> Result<ToolPolicy, ApiError>;
    async fn sessions_list(&self, agent_id: Option<String>) -> Result<Vec<Session>, ApiError>;
    async fn session_get(&self, request: SessionGetRequest) -> Result<Option<Session>, ApiError>;
    async fn schedules_list(&self) -> Result<Vec<Schedule>, ApiError>;
    async fn schedule_get(&self, request: ScheduleGetRequest) -> Result<Option<Schedule>, ApiError>;
    async fn schedule_create(&self, request: ScheduleCreateRequest) -> Result<Schedule, ApiError>;
    async fn schedule_update(&self, request: ScheduleUpdateRequest) -> Result<Schedule, ApiError>;
    async fn schedule_delete(&self, request: ScheduleGetRequest) -> Result<bool, ApiError>;
    async fn schedule_trigger(&self, request: ScheduleGetRequest) -> Result<Run, ApiError>;
    async fn webhooks_list(&self) -> Result<Vec<Webhook>, ApiError>;
    async fn webhook_get(&self, request: WebhookGetRequest) -> Result<Option<Webhook>, ApiError>;
    async fn webhook_create(&self, request: WebhookCreateRequest) -> Result<Webhook, ApiError>;
    async fn webhook_update(&self, request: WebhookUpdateRequest) -> Result<Webhook, ApiError>;
    async fn webhook_delete(&self, request: WebhookGetRequest) -> Result<bool, ApiError>;
    async fn webhook_rejections_list(
        &self,
        request: WebhookRejectionsListRequest,
    ) -> Result<Vec<WebhookRejection>, ApiError>;
    async fn webhook_invoke(&self, request: WebhookInvokeRequest) -> Result<Run, ApiError>;
    async fn webhook_rotate_secret(
        &self,
        request: WebhookSecretRotateRequest,
    ) -> Result<WebhookSecretRotateResponse, ApiError>;
    async fn run_create(&self, request: RunCreateRequest) -> Result<Run, ApiError>;
    async fn run_get(&self, request: RunGetRequest) -> Result<Option<Run>, ApiError>;
    async fn run_list(&self, request: RunListRequest) -> Result<Vec<Run>, ApiError>;
    async fn run_events_get(&self, request: RunEventsGetRequest)
        -> Result<Vec<RunEvent>, ApiError>;
    async fn run_cancel(&self, request: RunCancelRequest) -> Result<bool, ApiError>;
    async fn run_resume(&self, request: RunResumeRequest) -> Result<Run, ApiError>;
    async fn memory_store(&self, request: MemoryStoreRequest) -> UnitResult;
    async fn memory_recall(
        &self,
        request: MemoryRecallRequest,
    ) -> Result<Vec<MemoryItem>, ApiError>;
    async fn memory_get(&self, request: MemoryGetRequest) -> Result<Option<MemoryItem>, ApiError>;
    async fn memory_list(&self, request: MemoryListRequest) -> Result<Vec<MemoryItem>, ApiError>;
    async fn memory_forget(&self, request: MemoryForgetRequest) -> MemoryForgetResult;
    async fn memory_count(&self) -> MemoryCountResult;
    async fn conversation_summary_get(
        &self,
        request: ConversationSummaryGetRequest,
    ) -> Result<Option<MemoryItem>, ApiError>;
    async fn agent_observe(
        &self,
        request: AgentObserveRequest,
    ) -> Result<AgentObservation, ApiError>;
    #[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
    async fn schedule_fire(&self, schedule_id: String) -> Result<(), ApiError>;
}

pub fn init_service(config: Option<CanisterConfig>) {
    schedule_runtime::clear_registered_timers();
    SERVICE.with(|service| {
        *service.borrow_mut() = Arc::new(ConnectedIclawIcService::new(config));
    });
    schedule_runtime::spawn_restore();
}

pub fn post_upgrade_service(config: Option<CanisterConfig>) {
    init_service(config);
}

pub fn with_service() -> Arc<dyn IclawIcService> {
    SERVICE.with(|service| Arc::clone(&service.borrow()))
}

thread_local! {
    static SERVICE: RefCell<Arc<dyn IclawIcService>> =
        RefCell::new(Arc::new(ConnectedIclawIcService::new(None)));
}

struct ConnectedIclawIcService {
    memory: Option<Arc<dyn Memory>>,
    memory_ready: bool,
    memory_error: Option<String>,
    provider: Option<Arc<dyn IcCanisterProvider>>,
    provider_config: Option<ProviderConfig>,
    provider_error: Option<String>,
    context_config: Option<ContextConfig>,
    tools: Vec<Box<dyn Tool>>,
    tool_names: Vec<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct RunExecutionRequest {
    pub prompt: String,
    pub session_id: Option<String>,
    pub model: Option<String>,
    pub temperature: Option<f64>,
    pub agent: Agent,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct RunExecutionResult {
    pub response: Option<String>,
    pub session_id: Option<String>,
    pub model: Option<String>,
    pub provider_ready: bool,
    pub memory_ready: bool,
    pub status: String,
    pub error: Option<String>,
    pub events: Vec<agent::ToolLoopEvent>,
    pub pending_tool_calls: Vec<PendingToolCall>,
    pub pending_assistant_text: Option<String>,
    pub pending_reasoning_content: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ResumeExecutionPayload {
    pending_tool_calls: Vec<PendingToolCall>,
    pending_assistant_text: Option<String>,
    pending_reasoning_content: Option<String>,
}

#[cfg(not(target_arch = "wasm32"))]
thread_local! {
    static TEST_CYCLE_BALANCE_OVERRIDE: RefCell<Option<u128>> = const { RefCell::new(None) };
}

impl ConnectedIclawIcService {
    fn new(config: Option<CanisterConfig>) -> Self {
        let (memory, memory_error) = match build_memory() {
            Ok(memory) => (Some(memory), None),
            Err(error) => (None, Some(error.to_string())),
        };
        let provider_config = config.as_ref().and_then(|args| args.provider.clone());
        let context_config = config.and_then(|args| args.context);
        let tools = tools::ic_tools(memory.clone(), provider_config.as_ref());
        let tool_names = tools
            .iter()
            .map(|tool| tool.name().to_string())
            .collect::<Vec<_>>();
        let (provider, provider_error) = match build_provider(provider_config.as_ref()) {
            Ok(provider) => (provider, None),
            Err(error) => (None, Some(error.to_string())),
        };

        Self {
            memory,
            memory_ready: memory_error.is_none(),
            memory_error,
            provider,
            provider_config,
            provider_error,
            context_config,
            tools,
            tool_names,
        }
    }

    #[cfg(test)]
    fn with_dependencies(
        memory: Option<Arc<dyn Memory>>,
        memory_error: Option<String>,
        provider_config: Option<ProviderConfig>,
        provider: Option<Arc<dyn IcCanisterProvider>>,
        provider_error: Option<String>,
        context_config: Option<ContextConfig>,
    ) -> Self {
        let tools = tools::ic_tools(memory.clone(), provider_config.as_ref());
        let tool_names = tools
            .iter()
            .map(|tool| tool.name().to_string())
            .collect::<Vec<_>>();

        Self {
            memory,
            memory_ready: memory_error.is_none(),
            memory_error,
            provider,
            provider_config,
            provider_error,
            context_config,
            tools,
            tool_names,
        }
    }

    fn invalid_argument(message: &str) -> ApiError {
        ApiError::new(ApiErrorCode::InvalidArgument, message)
    }

    fn provider_error(error: anyhow::Error) -> ApiError {
        ApiError::new(ApiErrorCode::ProviderError, error.to_string())
    }

    fn memory_error(error: anyhow::Error) -> ApiError {
        ApiError::new(ApiErrorCode::MemoryError, error.to_string())
    }

    fn memory(&self) -> Result<&Arc<dyn Memory>, ApiError> {
        self.memory
            .as_ref()
            .ok_or_else(|| match self.memory_error.as_deref() {
                Some(error) => ApiError::new(ApiErrorCode::MemoryError, error),
                None => ApiError::new(
                    ApiErrorCode::MemoryError,
                    "ICP memory backend is not available for this canister",
                ),
            })
    }

    fn cycle_balance_warning(&self) -> Option<String> {
        let threshold = context::cycle_balance_warning_threshold(self.context_config.as_ref())?;
        let balance = current_cycle_balance();
        if balance > threshold {
            return None;
        }
        Some(format!(
            "[Runtime cycle warning]\nLiquid cycle balance is low: {balance} <= {threshold}. Tell the user the canister needs more cycles soon."
        ))
    }

    async fn finalize_run(
        &self,
        memory: &Arc<dyn Memory>,
        session: &Session,
        execution: RunExecutionRequest,
        trigger_kind: &str,
        trigger_id: Option<&str>,
    ) -> Result<Run, ApiError> {
        let mut run = runs::create_run_record(
            memory,
            session,
            &execution.prompt,
            execution.model.as_deref(),
            execution.temperature,
            self.provider.is_some(),
            self.memory_ready,
            trigger_kind,
            trigger_id,
        )
        .await
        .map_err(Self::memory_error)?;
        if trigger_kind == "webhook" {
            runs::append_run_event(memory, &run, "trigger_received", "webhook accepted")
                .await
                .map_err(Self::memory_error)?;
        }
        runs::append_run_event(memory, &run, "queued", "run queued")
            .await
            .map_err(Self::memory_error)?;
        run = runs::mark_run_started(memory, &run)
            .await
            .map_err(Self::memory_error)?;
        runs::append_run_event(memory, &run, "started", "run started")
            .await
            .map_err(Self::memory_error)?;

        let chat_result = self.execute_run(execution).await;
        let persisted_run = self.latest_run_or_memory_error(memory, &run.id).await?;
        if persisted_run.status == "cancelled" {
            return Ok(persisted_run);
        }

        let terminal_run = match &chat_result {
            Ok(response) => {
                if response.status == "blocked" {
                    runs::mark_run_blocked(
                        memory,
                        &persisted_run,
                        response.error.as_deref().unwrap_or("run blocked"),
                        response.provider_ready,
                        response.memory_ready,
                        &response.pending_tool_calls,
                        response.pending_assistant_text.as_deref(),
                        response.pending_reasoning_content.as_deref(),
                    )
                    .await
                    .map_err(Self::memory_error)
                } else {
                    runs::mark_run_completed(
                        memory,
                        &persisted_run,
                        response.response.as_deref().unwrap_or(""),
                        response.model.as_deref(),
                        response.provider_ready,
                        response.memory_ready,
                    )
                    .await
                    .map_err(Self::memory_error)
                }
            }
            Err(error) => runs::mark_run_failed(
                memory,
                &persisted_run,
                &error.message,
                self.provider.is_some(),
                self.memory_ready,
            )
            .await
            .map_err(Self::memory_error),
        }?;

        match &chat_result {
            Ok(response) => {
                for event in &response.events {
                    runs::append_run_event(memory, &terminal_run, &event.kind, &event.message)
                        .await
                        .map_err(Self::memory_error)?;
                }
                if let Some(message) = response.response.as_deref() {
                    runs::append_run_event(memory, &terminal_run, "assistant_message", message)
                        .await
                        .map_err(Self::memory_error)?;
                    runs::append_run_event(memory, &terminal_run, "completed", "run completed")
                        .await
                        .map_err(Self::memory_error)?;
                } else {
                    runs::append_run_event(
                        memory,
                        &terminal_run,
                        "blocked",
                        response.error.as_deref().unwrap_or("run blocked"),
                    )
                    .await
                    .map_err(Self::memory_error)?;
                }
            }
            Err(error) => {
                runs::append_run_event(memory, &terminal_run, "failed", &error.message)
                    .await
                    .map_err(Self::memory_error)?;
            }
        }

        runs::touch_session_after_run(memory, session, &terminal_run)
            .await
            .map_err(Self::memory_error)?;
        Ok(terminal_run)
    }

    async fn execute_run(
        &self,
        request: RunExecutionRequest,
    ) -> Result<RunExecutionResult, ApiError> {
        self.execute_run_internal(request, None).await
    }

    async fn execute_run_internal(
        &self,
        request: RunExecutionRequest,
        resume_payload: Option<ResumeExecutionPayload>,
    ) -> Result<RunExecutionResult, ApiError> {
        if request.prompt.trim().is_empty() {
            return Err(Self::invalid_argument("prompt must not be empty"));
        }

        let provider =
            self.provider
                .as_ref()
                .ok_or_else(|| match self.provider_error.as_deref() {
                    Some(error) => ApiError::new(ApiErrorCode::ProviderError, error),
                    None => ApiError::new(
                        ApiErrorCode::NotSupported,
                        "ICP provider adapter is not configured for this canister",
                    ),
                })?;

        let default_model = self
            .provider_config
            .as_ref()
            .map(|config| config.default_model.as_str())
            .unwrap_or("provider-unconfigured");
        let model = request
            .model
            .clone()
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| default_model.to_string());
        let temperature = request.temperature.unwrap_or(0.0);
        let prompt_context = context::build_prompt_context(
            self.memory.as_deref(),
            self.context_config.as_ref(),
            &request.prompt,
            request.session_id.as_deref(),
            &self.tool_names,
        )
        .await;
        let prompt_context = if let Some(warning) = self.cycle_balance_warning() {
            context::PromptContext {
                system_prompt: format!("{}\n\n{}", prompt_context.system_prompt, warning),
                memory_context: prompt_context.memory_context,
            }
        } else {
            prompt_context
        };
        let prompt_context = if let Some(override_prompt) = request
            .agent
            .system_prompt_override
            .as_deref()
            .filter(|value| !value.trim().is_empty())
        {
            context::PromptContext {
                system_prompt: format!("{override_prompt}\n\n{}", prompt_context.system_prompt),
                memory_context: prompt_context.memory_context,
            }
        } else {
            prompt_context
        };
        let history = agent::load_history(
            self.memory.as_ref(),
            request.session_id.as_deref(),
            self.context_config.as_ref(),
        )
        .await
        .map_err(Self::memory_error)?;
        let session_summary = policies::load_conversation_summary(
            self.memory.as_ref(),
            request.session_id.as_deref(),
            self.context_config.as_ref(),
        )
        .await
        .map_err(Self::memory_error)?;
        if resume_payload.is_none() {
            agent::autosave_turn(
                self.memory.as_ref(),
                request.session_id.as_deref(),
                "user",
                &request.prompt,
                self.context_config.as_ref(),
            )
            .await
            .map_err(Self::memory_error)?;
        }
        let mut messages = if resume_payload.is_some() {
            agent::build_messages_without_prompt(&prompt_context, &session_summary, &history)
        } else {
            agent::build_messages(&prompt_context, &session_summary, &history, &request.prompt)
        };
        let mut compression_state = compression::CompressionState::default();
        let tool_policies = control_plane::list_tool_policies(
            self.memory.as_ref(),
            &self.tool_names,
            Some(request.agent.id.as_str()),
        )
        .await
        .map_err(Self::memory_error)?;
        let authorization = agent::ToolAuthorization {
            agent_id: request.agent.id.clone(),
            enabled_tool_names: request.agent.enabled_tool_names.clone(),
            requires_tool_approval: request.agent.requires_tool_approval,
            policies: tool_policies,
        };
        let outcome = match resume_payload {
            Some(payload) => agent::resume_tool_loop(
                provider,
                &self.tools,
                &mut messages,
                &model,
                temperature,
                &authorization,
                self.context_config.as_ref(),
                &mut compression_state,
                &payload.pending_tool_calls,
                payload.pending_assistant_text.as_deref(),
                payload.pending_reasoning_content.as_deref(),
            )
            .await
            .map_err(Self::provider_error)?,
            None => agent::run_tool_loop(
                provider,
                &self.tools,
                &mut messages,
                &model,
                temperature,
                &authorization,
                self.context_config.as_ref(),
                &mut compression_state,
            )
            .await
            .map_err(Self::provider_error)?,
        };
        let (response, status, error, events, pending_tool_calls, pending_assistant_text, pending_reasoning_content) =
            match outcome {
                agent::ToolLoopOutcome::Completed { response, events } => (
                    Some(response),
                    "completed".to_string(),
                    None,
                    events,
                    Vec::new(),
                    None,
                    None,
                ),
                agent::ToolLoopOutcome::Blocked {
                    message,
                    events,
                    pending_tool_calls,
                    pending_assistant_text,
                    pending_reasoning_content,
                } => (
                    None,
                    "blocked".to_string(),
                    Some(message),
                    events,
                    pending_tool_calls,
                    pending_assistant_text,
                    pending_reasoning_content,
                ),
            };
        if response.is_none() {
            return Ok(RunExecutionResult {
                response: None,
                session_id: request.session_id,
                model: Some(model),
                provider_ready: true,
                memory_ready: self.memory_ready,
                status,
                error,
                events,
                pending_tool_calls,
                pending_assistant_text,
                pending_reasoning_content,
            });
        };
        let response = response.expect("response checked above");
        agent::autosave_turn(
            self.memory.as_ref(),
            request.session_id.as_deref(),
            "assistant",
            response.response.text_or_empty(),
            self.context_config.as_ref(),
        )
        .await
        .map_err(Self::memory_error)?;
        let promoted = policies::maybe_auto_promote(
            self.memory.as_ref(),
            request.session_id.as_deref(),
            &request.prompt,
            response.response.text_or_empty(),
            self.context_config.as_ref(),
        )
        .await
        .map_err(Self::memory_error)?;
        let updated_history = agent::load_history(
            self.memory.as_ref(),
            request.session_id.as_deref(),
            self.context_config.as_ref(),
        )
        .await
        .map_err(Self::memory_error)?;
        policies::refresh_conversation_summary(
            self.memory.as_ref(),
            request.session_id.as_deref(),
            &updated_history,
            promoted.as_ref(),
            self.context_config.as_ref(),
        )
        .await
        .map_err(Self::memory_error)?;
        if let Some(summary) = compression_state.generated_summary.as_deref() {
            policies::replace_conversation_summary(
                self.memory.as_ref(),
                request.session_id.as_deref(),
                updated_history.len(),
                summary,
            )
            .await
            .map_err(Self::memory_error)?;
        }

        Ok(RunExecutionResult {
            response: Some(response.response.text_or_empty().to_string()),
            session_id: request.session_id,
            model: response.model.or(Some(model)),
            provider_ready: true,
            memory_ready: self.memory_ready,
            status: "completed".to_string(),
            error: None,
            events,
            pending_tool_calls: Vec::new(),
            pending_assistant_text: None,
            pending_reasoning_content: None,
        })
    }

    async fn latest_run_or_memory_error(
        &self,
        memory: &Arc<dyn Memory>,
        run_id: &str,
    ) -> Result<Run, ApiError> {
        runs::get_run(Some(memory), run_id)
            .await
            .map_err(Self::memory_error)?
            .ok_or_else(|| {
                ApiError::new(
                    ApiErrorCode::MemoryError,
                    "run record disappeared during execution",
                )
            })
    }

    fn validate_session_mode(session_mode: &str, fixed_session_id: Option<&str>) -> Result<(), ApiError> {
        if session_mode != "reuse_fixed" && session_mode != "create_new" {
            return Err(Self::invalid_argument("unsupported session_mode"));
        }
        if session_mode == "reuse_fixed"
            && fixed_session_id.unwrap_or_default().trim().is_empty()
        {
            return Err(Self::invalid_argument(
                "fixed_session_id is required for reuse_fixed",
            ));
        }
        Ok(())
    }

    fn validate_schedule_interval(interval_minutes: u64) -> Result<(), ApiError> {
        if !(1..=10_080).contains(&interval_minutes) {
            return Err(Self::invalid_argument(
                "interval_minutes must be between 1 and 10080",
            ));
        }
        Ok(())
    }

    async fn schedule_execute(
        &self,
        schedule_id: &str,
        advance_next_run: bool,
        require_enabled: bool,
    ) -> Result<Run, ApiError> {
        let memory = self.memory()?;
        let schedule = schedules::get_schedule(
            Some(memory),
            &ScheduleGetRequest {
                schedule_id: schedule_id.to_string(),
            },
        )
        .await
        .map_err(Self::memory_error)?
        .ok_or_else(|| Self::invalid_argument("schedule_id does not exist"))?;
        if require_enabled && !schedule.enabled {
            return Err(Self::invalid_argument("schedule is disabled"));
        }
        if schedule.running {
            if advance_next_run {
                let next = schedules::mark_schedule_skipped(
                    memory,
                    &schedule,
                    "schedule skipped because the previous execution is still running",
                )
                .await
                .map_err(Self::memory_error)?;
                schedule_runtime::register_schedule(&next);
            }
            return Err(Self::invalid_argument("schedule is already running"));
        }
        let running = schedules::mark_schedule_running(memory, &schedule)
            .await
            .map_err(Self::memory_error)?;
        let execution = async {
            let agent = control_plane::get_agent(self.memory.as_ref(), &self.tool_names, &running.agent_id)
                .await
                .map_err(Self::memory_error)?
                .ok_or_else(|| Self::invalid_argument("schedule agent_id does not exist"))?;
            let session = runs::ensure_session(
                memory,
                &agent.id,
                schedules::resolve_session_id(&running).as_deref(),
                &running.prompt,
            )
            .await
            .map_err(|error| {
                if error
                    .to_string()
                    .contains("session agent_id does not match the requested agent_id")
                {
                    Self::invalid_argument("session agent_id must match schedule agent_id")
                } else {
                    Self::memory_error(error)
                }
            })?;
            self.finalize_run(
                memory,
                &session,
                RunExecutionRequest {
                    prompt: running.prompt.clone(),
                    session_id: Some(session.id.clone()),
                    model: None,
                    temperature: Some(0.0),
                    agent,
                },
                "schedule",
                Some(running.id.as_str()),
            )
            .await
        }
        .await;

        let persisted = match &execution {
            Ok(run) => schedules::mark_schedule_finished(
                memory,
                &running,
                Some(run),
                None,
                advance_next_run,
            )
            .await
            .map_err(Self::memory_error)?,
            Err(error) => schedules::mark_schedule_finished(
                memory,
                &running,
                None,
                Some(&error.message),
                advance_next_run,
            )
            .await
            .map_err(Self::memory_error)?,
        };
        if advance_next_run && persisted.enabled {
            schedule_runtime::register_schedule(&persisted);
        }
        execution
    }
}

#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
pub(crate) fn spawn_schedule_fire(schedule_id: String) {
    #[cfg(target_arch = "wasm32")]
    ic_cdk::futures::spawn(async move {
        let _ = with_service().schedule_fire(schedule_id).await;
    });
    #[cfg(not(target_arch = "wasm32"))]
    let _ = schedule_id;
}

#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
pub(crate) async fn restore_schedule_timers() {
    let schedules = with_service().schedules_list().await.unwrap_or_default();
    for schedule in schedules
        .into_iter()
        .filter(|entry| entry.enabled && entry.next_run_at.is_some())
    {
        schedule_runtime::register_schedule(&schedule);
    }
}

#[async_trait]
impl IclawIcService for ConnectedIclawIcService {
    async fn health(&self) -> HealthResponse {
        let memory_ready = match self.memory.as_ref() {
            Some(memory) => self.memory_ready && memory.health_check().await,
            None => false,
        };
        HealthResponse::degraded(self.provider.is_some(), memory_ready)
    }

    async fn agents_list(&self) -> Result<Vec<Agent>, ApiError> {
        control_plane::list_agents(self.memory.as_ref(), &self.tool_names)
            .await
            .map_err(Self::memory_error)
    }

    async fn agent_get(&self, request: AgentGetRequest) -> Result<Option<Agent>, ApiError> {
        if request.agent_id.trim().is_empty() {
            return Err(Self::invalid_argument("agent_id must not be empty"));
        }
        control_plane::get_agent(self.memory.as_ref(), &self.tool_names, &request.agent_id)
            .await
            .map_err(Self::memory_error)
    }

    async fn agent_create(&self, request: AgentCreateRequest) -> Result<Agent, ApiError> {
        if request.draft.id.trim().is_empty() || request.draft.name.trim().is_empty() {
            return Err(Self::invalid_argument(
                "agent id and name must not be empty",
            ));
        }
        let memory = self.memory()?;
        control_plane::create_agent(memory, request)
            .await
            .map_err(Self::memory_error)
    }

    async fn agent_update(&self, request: AgentUpdateRequest) -> Result<Agent, ApiError> {
        if request.agent.id.trim().is_empty() || request.agent.name.trim().is_empty() {
            return Err(Self::invalid_argument(
                "agent id and name must not be empty",
            ));
        }
        let memory = self.memory()?;
        control_plane::update_agent(memory, request)
            .await
            .map_err(Self::memory_error)
    }

    async fn tool_policy_list(
        &self,
        request: ToolPolicyListRequest,
    ) -> Result<Vec<ToolPolicy>, ApiError> {
        control_plane::list_tool_policies(
            self.memory.as_ref(),
            &self.tool_names,
            request.agent_id.as_deref(),
        )
        .await
        .map_err(Self::memory_error)
    }

    async fn tool_policy_update(
        &self,
        request: ToolPolicyUpdateRequest,
    ) -> Result<ToolPolicy, ApiError> {
        if request.policy.agent_id.trim().is_empty() {
            return Err(Self::invalid_argument("agent_id must not be empty"));
        }
        if request.policy.tool_name.trim().is_empty() {
            return Err(Self::invalid_argument("tool_name must not be empty"));
        }
        let memory = self.memory()?;
        control_plane::upsert_tool_policy(memory, &self.tool_names, request)
            .await
            .map_err(Self::memory_error)
    }

    async fn sessions_list(&self, agent_id: Option<String>) -> Result<Vec<Session>, ApiError> {
        runs::list_sessions(self.memory.as_ref(), agent_id.as_deref())
            .await
            .map_err(Self::memory_error)
    }

    async fn session_get(&self, request: SessionGetRequest) -> Result<Option<Session>, ApiError> {
        if request.session_id.trim().is_empty() {
            return Err(Self::invalid_argument("session_id must not be empty"));
        }

        runs::get_session(self.memory.as_ref(), &request.session_id)
            .await
            .map_err(Self::memory_error)
    }

    async fn schedules_list(&self) -> Result<Vec<Schedule>, ApiError> {
        schedules::list_schedules(self.memory.as_ref())
            .await
            .map_err(Self::memory_error)
    }

    async fn schedule_get(&self, request: ScheduleGetRequest) -> Result<Option<Schedule>, ApiError> {
        if request.schedule_id.trim().is_empty() {
            return Err(Self::invalid_argument("schedule_id must not be empty"));
        }
        schedules::get_schedule(self.memory.as_ref(), &request)
            .await
            .map_err(Self::memory_error)
    }

    async fn schedule_create(&self, request: ScheduleCreateRequest) -> Result<Schedule, ApiError> {
        if request.draft.id.trim().is_empty()
            || request.draft.name.trim().is_empty()
            || request.draft.prompt.trim().is_empty()
        {
            return Err(Self::invalid_argument(
                "schedule id, name, and prompt must not be empty",
            ));
        }
        Self::validate_schedule_interval(request.draft.interval_minutes)?;
        Self::validate_session_mode(
            &request.draft.session_mode,
            request.draft.fixed_session_id.as_deref(),
        )?;
        if schedules::get_schedule(
            self.memory.as_ref(),
            &ScheduleGetRequest {
                schedule_id: request.draft.id.clone(),
            },
        )
        .await
        .map_err(Self::memory_error)?
        .is_some()
        {
            return Err(Self::invalid_argument("schedule_id already exists"));
        }
        let memory = self.memory()?;
        let Some(_) = control_plane::get_agent(
            self.memory.as_ref(),
            &self.tool_names,
            &request.draft.agent_id,
        )
        .await
        .map_err(Self::memory_error)?
        else {
            return Err(Self::invalid_argument("agent_id does not exist"));
        };
        let created = schedules::create_schedule(memory, request)
            .await
            .map_err(Self::memory_error)?;
        if created.enabled {
            schedule_runtime::register_schedule(&created);
        }
        Ok(created)
    }

    async fn schedule_update(&self, request: ScheduleUpdateRequest) -> Result<Schedule, ApiError> {
        if request.schedule.id.trim().is_empty()
            || request.schedule.name.trim().is_empty()
            || request.schedule.prompt.trim().is_empty()
        {
            return Err(Self::invalid_argument(
                "schedule id, name, and prompt must not be empty",
            ));
        }
        Self::validate_schedule_interval(request.schedule.interval_minutes)?;
        Self::validate_session_mode(
            &request.schedule.session_mode,
            request.schedule.fixed_session_id.as_deref(),
        )?;
        let existing = schedules::get_schedule(
            self.memory.as_ref(),
            &ScheduleGetRequest {
                schedule_id: request.schedule.id.clone(),
            },
        )
        .await
        .map_err(Self::memory_error)?
        .ok_or_else(|| Self::invalid_argument("schedule_id does not exist"))?;
        if existing.running {
            return Err(Self::invalid_argument("running schedules cannot be updated"));
        }
        let memory = self.memory()?;
        let Some(_) = control_plane::get_agent(
            self.memory.as_ref(),
            &self.tool_names,
            &request.schedule.agent_id,
        )
        .await
        .map_err(Self::memory_error)?
        else {
            return Err(Self::invalid_argument("agent_id does not exist"));
        };
        let updated = schedules::update_schedule(memory, request)
            .await
            .map_err(Self::memory_error)?;
        if updated.enabled {
            schedule_runtime::register_schedule(&updated);
        } else {
            schedule_runtime::unregister_schedule(&updated.id);
        }
        Ok(updated)
    }

    async fn schedule_delete(&self, request: ScheduleGetRequest) -> Result<bool, ApiError> {
        if request.schedule_id.trim().is_empty() {
            return Err(Self::invalid_argument("schedule_id must not be empty"));
        }
        let Some(schedule) = schedules::get_schedule(self.memory.as_ref(), &request)
            .await
            .map_err(Self::memory_error)?
        else {
            return Ok(false);
        };
        if schedule.running {
            return Err(Self::invalid_argument("running schedules cannot be deleted"));
        }
        let memory = self.memory()?;
        let deleted = schedules::delete_schedule(memory, &request.schedule_id)
            .await
            .map_err(Self::memory_error)?;
        if deleted {
            schedule_runtime::unregister_schedule(&request.schedule_id);
        }
        Ok(deleted)
    }

    async fn schedule_trigger(&self, request: ScheduleGetRequest) -> Result<Run, ApiError> {
        if request.schedule_id.trim().is_empty() {
            return Err(Self::invalid_argument("schedule_id must not be empty"));
        }
        self.schedule_execute(&request.schedule_id, false, false).await
    }

    async fn webhooks_list(&self) -> Result<Vec<Webhook>, ApiError> {
        webhooks::list_webhooks(self.memory.as_ref())
            .await
            .map_err(Self::memory_error)
    }

    async fn webhook_get(&self, request: WebhookGetRequest) -> Result<Option<Webhook>, ApiError> {
        if request.webhook_id.trim().is_empty() {
            return Err(Self::invalid_argument("webhook_id must not be empty"));
        }
        webhooks::get_webhook(self.memory.as_ref(), &request)
            .await
            .map_err(Self::memory_error)
    }

    async fn webhook_create(&self, request: WebhookCreateRequest) -> Result<Webhook, ApiError> {
        if request.draft.id.trim().is_empty() || request.draft.name.trim().is_empty() {
            return Err(Self::invalid_argument(
                "webhook id and name must not be empty",
            ));
        }
        if request.draft.secret.trim().is_empty() {
            return Err(Self::invalid_argument("webhook secret must not be empty"));
        }
        if request.draft.session_mode != "reuse_fixed" && request.draft.session_mode != "create_new"
        {
            return Err(Self::invalid_argument("unsupported session_mode"));
        }
        if request.draft.session_mode == "reuse_fixed"
            && request
                .draft
                .fixed_session_id
                .as_deref()
                .unwrap_or_default()
                .trim()
                .is_empty()
        {
            return Err(Self::invalid_argument(
                "fixed_session_id is required for reuse_fixed",
            ));
        }
        if webhooks::load_webhook(self.memory.as_ref(), &request.draft.id)
            .await
            .map_err(Self::memory_error)?
            .is_some()
        {
            return Err(Self::invalid_argument("webhook_id already exists"));
        }
        let memory = self.memory()?;
        let Some(_) = control_plane::get_agent(
            self.memory.as_ref(),
            &self.tool_names,
            &request.draft.agent_id,
        )
        .await
        .map_err(Self::memory_error)?
        else {
            return Err(Self::invalid_argument("agent_id does not exist"));
        };
        webhooks::create_webhook(memory, request)
            .await
            .map_err(Self::memory_error)
    }

    async fn webhook_update(&self, request: WebhookUpdateRequest) -> Result<Webhook, ApiError> {
        if request.webhook.id.trim().is_empty() || request.webhook.name.trim().is_empty() {
            return Err(Self::invalid_argument(
                "webhook id and name must not be empty",
            ));
        }
        if request
            .secret_override
            .as_deref()
            .is_some_and(|secret| secret.trim().is_empty())
        {
            return Err(Self::invalid_argument(
                "secret_override must not be empty when provided",
            ));
        }
        if request.webhook.session_mode != "reuse_fixed"
            && request.webhook.session_mode != "create_new"
        {
            return Err(Self::invalid_argument("unsupported session_mode"));
        }
        if request.webhook.session_mode == "reuse_fixed"
            && request
                .webhook
                .fixed_session_id
                .as_deref()
                .unwrap_or_default()
                .trim()
                .is_empty()
        {
            return Err(Self::invalid_argument(
                "fixed_session_id is required for reuse_fixed",
            ));
        }
        let memory = self.memory()?;
        let Some(_) = control_plane::get_agent(
            self.memory.as_ref(),
            &self.tool_names,
            &request.webhook.agent_id,
        )
        .await
        .map_err(Self::memory_error)?
        else {
            return Err(Self::invalid_argument("agent_id does not exist"));
        };
        webhooks::update_webhook(memory, request)
            .await
            .map_err(Self::memory_error)
    }

    async fn webhook_delete(&self, request: WebhookGetRequest) -> Result<bool, ApiError> {
        if request.webhook_id.trim().is_empty() {
            return Err(Self::invalid_argument("webhook_id must not be empty"));
        }
        let memory = self.memory()?;
        webhooks::delete_webhook(memory, &request.webhook_id)
            .await
            .map_err(Self::memory_error)
    }

    async fn webhook_rejections_list(
        &self,
        request: WebhookRejectionsListRequest,
    ) -> Result<Vec<WebhookRejection>, ApiError> {
        if request.webhook_id.trim().is_empty() {
            return Err(Self::invalid_argument("webhook_id must not be empty"));
        }
        webhooks::list_rejections(self.memory.as_ref(), &request)
            .await
            .map_err(Self::memory_error)
    }

    async fn webhook_invoke(&self, request: WebhookInvokeRequest) -> Result<Run, ApiError> {
        if request.webhook_id.trim().is_empty() {
            return Err(Self::invalid_argument("webhook_id must not be empty"));
        }
        if request.prompt.trim().is_empty() {
            return Err(Self::invalid_argument("prompt must not be empty"));
        }
        let memory = self.memory()?;
        let webhook = webhooks::load_webhook(
            self.memory.as_ref(),
            &request.webhook_id,
        )
        .await
        .map_err(Self::memory_error)?
        .ok_or_else(|| Self::invalid_argument("webhook_id does not exist"))?;
        if !webhook.enabled {
            let _ = webhooks::record_rejected_invoke(memory, &webhook, "webhook is disabled").await;
            return Err(ApiError::new(
                ApiErrorCode::InvalidArgument,
                "webhook is disabled",
            ));
        }
        if webhook.secret != request.secret {
            let _ =
                webhooks::record_rejected_invoke(memory, &webhook, "webhook secret is invalid")
                    .await;
            return Err(ApiError::new(
                ApiErrorCode::Unauthorized,
                "webhook secret is invalid",
            ));
        }
        let agent =
            control_plane::get_agent(self.memory.as_ref(), &self.tool_names, &webhook.agent_id)
                .await
                .map_err(Self::memory_error)?
                .ok_or_else(|| Self::invalid_argument("webhook agent_id does not exist"))?;
        let session = runs::ensure_session(
            memory,
            &agent.id,
            webhooks::resolve_session_id(&webhook, &request).as_deref(),
            &request.prompt,
        )
        .await
        .map_err(|error| {
            if error
                .to_string()
                .contains("session agent_id does not match the requested agent_id")
            {
                Self::invalid_argument("session agent_id must match webhook agent_id")
            } else {
                Self::memory_error(error)
            }
        })?;
        let run = self
            .finalize_run(
                memory,
                &session,
                RunExecutionRequest {
                    prompt: request.prompt,
                    session_id: Some(session.id.clone()),
                    model: request.model,
                    temperature: request.temperature,
                    agent,
                },
                "webhook",
                Some(webhook.id.as_str()),
            )
            .await?;
        let _ = webhooks::touch_last_run(memory, &webhook, &run.id)
            .await
            .map_err(Self::memory_error)?;
        Ok(run)
    }

    async fn webhook_rotate_secret(
        &self,
        request: WebhookSecretRotateRequest,
    ) -> Result<WebhookSecretRotateResponse, ApiError> {
        if request.webhook_id.trim().is_empty() {
            return Err(Self::invalid_argument("webhook_id must not be empty"));
        }
        let memory = self.memory()?;
        webhooks::rotate_secret(memory, &request.webhook_id)
            .await
            .map_err(Self::memory_error)
    }

    async fn run_create(&self, request: RunCreateRequest) -> Result<Run, ApiError> {
        if request.prompt.trim().is_empty() {
            return Err(Self::invalid_argument("prompt must not be empty"));
        }

        let memory = self.memory()?;
        let existing_session = match request.session_id.as_deref() {
            Some(session_id) if !session_id.trim().is_empty() => {
                runs::get_session(Some(memory), session_id)
                    .await
                    .map_err(Self::memory_error)?
            }
            _ => None,
        };
        let resolved_agent = if let Some(session) = existing_session.as_ref() {
            if let Some(requested_agent_id) = request.agent_id.as_deref() {
                if requested_agent_id != session.agent_id {
                    return Err(Self::invalid_argument(
                        "agent_id must match the existing session agent_id",
                    ));
                }
            }
            control_plane::get_agent(self.memory.as_ref(), &self.tool_names, &session.agent_id)
                .await
                .map_err(Self::memory_error)?
                .ok_or_else(|| Self::invalid_argument("session agent_id does not exist"))?
        } else {
            let agent_id = request.agent_id.as_deref().unwrap_or("default");
            control_plane::get_agent(self.memory.as_ref(), &self.tool_names, agent_id)
                .await
                .map_err(Self::memory_error)?
                .ok_or_else(|| Self::invalid_argument("agent_id does not exist"))?
        };
        let session = if let Some(session) = existing_session {
            session
        } else {
            runs::ensure_session(
                memory,
                &resolved_agent.id,
                request.session_id.as_deref(),
                &request.prompt,
            )
            .await
            .map_err(Self::memory_error)?
        };
        self.finalize_run(
            memory,
            &session,
            RunExecutionRequest {
                prompt: request.prompt,
                session_id: Some(session.id.clone()),
                model: request.model,
                temperature: request.temperature,
                agent: resolved_agent.clone(),
            },
            "manual",
            None,
        )
        .await
    }

    async fn run_get(&self, request: RunGetRequest) -> Result<Option<Run>, ApiError> {
        if request.run_id.trim().is_empty() {
            return Err(Self::invalid_argument("run_id must not be empty"));
        }
        runs::get_run(self.memory.as_ref(), &request.run_id)
            .await
            .map_err(Self::memory_error)
    }

    async fn run_list(&self, request: RunListRequest) -> Result<Vec<Run>, ApiError> {
        runs::list_runs(
            self.memory.as_ref(),
            request.session_id.as_deref(),
            request.limit.unwrap_or(50) as usize,
        )
        .await
        .map_err(Self::memory_error)
    }

    async fn run_events_get(
        &self,
        request: RunEventsGetRequest,
    ) -> Result<Vec<RunEvent>, ApiError> {
        if request.run_id.trim().is_empty() {
            return Err(Self::invalid_argument("run_id must not be empty"));
        }
        runs::list_run_events(self.memory.as_ref(), &request.run_id)
            .await
            .map_err(Self::memory_error)
    }

    async fn run_cancel(&self, request: RunCancelRequest) -> Result<bool, ApiError> {
        if request.run_id.trim().is_empty() {
            return Err(Self::invalid_argument("run_id must not be empty"));
        }
        let memory = self.memory()?;
        runs::cancel_run(memory, &request.run_id)
            .await
            .map_err(Self::memory_error)
    }

    async fn run_resume(&self, request: RunResumeRequest) -> Result<Run, ApiError> {
        if request.run_id.trim().is_empty() {
            return Err(Self::invalid_argument("run_id must not be empty"));
        }
        let memory = self.memory()?;
        let resume_state = runs::get_resume_state(Some(memory), &request.run_id)
            .await
            .map_err(Self::memory_error)?
            .ok_or_else(|| Self::invalid_argument("run_id does not exist"))?;
        if resume_state.run.status != "blocked" {
            return Err(Self::invalid_argument("run must be blocked to resume"));
        }
        if resume_state.run.pending_tool_calls.is_empty() {
            return Err(Self::invalid_argument("run has no pending tool calls"));
        }

        let agent = control_plane::get_agent(
            self.memory.as_ref(),
            &self.tool_names,
            &resume_state.run.agent_id,
        )
        .await
        .map_err(Self::memory_error)?
        .ok_or_else(|| Self::invalid_argument("run agent_id does not exist"))?;
        let session = runs::get_session(Some(memory), &resume_state.run.session_id)
            .await
            .map_err(Self::memory_error)?
            .ok_or_else(|| Self::invalid_argument("run session_id does not exist"))?;

        let running_run = runs::mark_run_resumed(memory, &resume_state.run)
            .await
            .map_err(Self::memory_error)?;
        runs::append_run_event(memory, &running_run, "approved", "tool approval granted")
            .await
            .map_err(Self::memory_error)?;
        runs::append_run_event(memory, &running_run, "resumed", "run resumed")
            .await
            .map_err(Self::memory_error)?;

        let chat_result = self
            .execute_run_internal(
                RunExecutionRequest {
                    prompt: running_run.prompt.clone(),
                    session_id: Some(running_run.session_id.clone()),
                    model: running_run.model.clone(),
                    temperature: resume_state.requested_temperature,
                    agent,
                },
                Some(ResumeExecutionPayload {
                    pending_tool_calls: resume_state.run.pending_tool_calls.clone(),
                    pending_assistant_text: resume_state.run.pending_assistant_text.clone(),
                    pending_reasoning_content: resume_state.pending_reasoning_content.clone(),
                }),
            )
            .await;
        let persisted_run = self.latest_run_or_memory_error(memory, &running_run.id).await?;

        let terminal_run = match &chat_result {
            Ok(response) => {
                if response.status == "blocked" {
                    runs::mark_run_blocked(
                        memory,
                        &persisted_run,
                        response.error.as_deref().unwrap_or("run blocked"),
                        response.provider_ready,
                        response.memory_ready,
                        &response.pending_tool_calls,
                        response.pending_assistant_text.as_deref(),
                        response.pending_reasoning_content.as_deref(),
                    )
                    .await
                    .map_err(Self::memory_error)
                } else {
                    runs::mark_run_completed(
                        memory,
                        &persisted_run,
                        response.response.as_deref().unwrap_or(""),
                        response.model.as_deref(),
                        response.provider_ready,
                        response.memory_ready,
                    )
                    .await
                    .map_err(Self::memory_error)
                }
            }
            Err(error) => runs::mark_run_failed(
                memory,
                &persisted_run,
                &error.message,
                self.provider.is_some(),
                self.memory_ready,
            )
            .await
            .map_err(Self::memory_error),
        }?;

        match &chat_result {
            Ok(response) => {
                for event in &response.events {
                    runs::append_run_event(memory, &terminal_run, &event.kind, &event.message)
                        .await
                        .map_err(Self::memory_error)?;
                }
                if let Some(message) = response.response.as_deref() {
                    runs::append_run_event(memory, &terminal_run, "assistant_message", message)
                        .await
                        .map_err(Self::memory_error)?;
                    runs::append_run_event(memory, &terminal_run, "completed", "run completed")
                        .await
                        .map_err(Self::memory_error)?;
                } else {
                    runs::append_run_event(
                        memory,
                        &terminal_run,
                        "blocked",
                        response.error.as_deref().unwrap_or("run blocked"),
                    )
                    .await
                    .map_err(Self::memory_error)?;
                }
            }
            Err(error) => {
                runs::append_run_event(memory, &terminal_run, "failed", &error.message)
                    .await
                    .map_err(Self::memory_error)?;
            }
        }

        runs::touch_session_after_run(memory, &session, &terminal_run)
            .await
            .map_err(Self::memory_error)?;
        Ok(terminal_run)
    }

    async fn memory_store(&self, request: MemoryStoreRequest) -> UnitResult {
        if request.key.trim().is_empty() {
            return Err(Self::invalid_argument("key must not be empty"));
        }

        self.memory()?
            .store(
                &request.key,
                &request.content,
                (&request.category).into(),
                request.session_id.as_deref(),
            )
            .await
            .map_err(Self::memory_error)
    }

    async fn memory_recall(
        &self,
        request: MemoryRecallRequest,
    ) -> Result<Vec<MemoryItem>, ApiError> {
        if request.query.trim().is_empty() {
            return Err(Self::invalid_argument("query must not be empty"));
        }

        self.memory()?
            .recall(
                &request.query,
                request.limit as usize,
                request.session_id.as_deref(),
            )
            .await
            .map(|entries| entries.into_iter().map(Into::into).collect())
            .map_err(Self::memory_error)
    }

    async fn memory_get(&self, request: MemoryGetRequest) -> Result<Option<MemoryItem>, ApiError> {
        if request.key.trim().is_empty() {
            return Err(Self::invalid_argument("key must not be empty"));
        }

        self.memory()?
            .get(&request.key)
            .await
            .map(|entry| entry.map(Into::into))
            .map_err(Self::memory_error)
    }

    async fn memory_list(&self, request: MemoryListRequest) -> Result<Vec<MemoryItem>, ApiError> {
        let category = request.category.as_ref().map(Into::into);
        self.memory()?
            .list(category.as_ref(), request.session_id.as_deref())
            .await
            .map(|entries| entries.into_iter().map(Into::into).collect())
            .map_err(Self::memory_error)
    }

    async fn memory_forget(&self, request: MemoryForgetRequest) -> MemoryForgetResult {
        if request.key.trim().is_empty() {
            return Err(Self::invalid_argument("key must not be empty"));
        }

        self.memory()?
            .forget(&request.key)
            .await
            .map_err(Self::memory_error)
    }

    async fn memory_count(&self) -> MemoryCountResult {
        self.memory()?
            .count()
            .await
            .map(|count| count as u64)
            .map_err(Self::memory_error)
    }

    async fn conversation_summary_get(
        &self,
        request: ConversationSummaryGetRequest,
    ) -> Result<Option<MemoryItem>, ApiError> {
        if request.session_id.trim().is_empty() {
            return Err(Self::invalid_argument("session_id must not be empty"));
        }

        observe::conversation_summary_item(self.memory.as_ref(), &request.session_id)
            .await
            .map_err(Self::memory_error)
    }

    async fn agent_observe(
        &self,
        request: AgentObserveRequest,
    ) -> Result<AgentObservation, ApiError> {
        observe::observe(
            self.memory.as_ref(),
            request.session_id.as_deref(),
            self.context_config.as_ref(),
        )
        .await
        .map_err(Self::memory_error)
    }

    async fn schedule_fire(&self, schedule_id: String) -> Result<(), ApiError> {
        let _ = self.schedule_execute(&schedule_id, true, true).await;
        Ok(())
    }
}

#[cfg(target_arch = "wasm32")]
fn current_cycle_balance() -> u128 {
    ic_cdk::api::canister_liquid_cycle_balance()
}

#[cfg(not(target_arch = "wasm32"))]
fn current_cycle_balance() -> u128 {
    TEST_CYCLE_BALANCE_OVERRIDE.with(|value| value.borrow().unwrap_or(u128::MAX))
}

#[cfg(test)]
pub(crate) fn set_test_cycle_balance_override(value: Option<u128>) {
    #[cfg(not(target_arch = "wasm32"))]
    TEST_CYCLE_BALANCE_OVERRIDE.with(|slot| {
        *slot.borrow_mut() = value;
    });
}

#[cfg(test)]
mod tests;
