// where: iclaw/canister/src/service.rs
// what: Connected service layer behind the public ICP canister entrypoints
// why: Track C wires memory and HTTPS-outcall-backed provider behavior into the canister without exposing native-only surfaces

#[path = "service/agent.rs"]
mod agent;
#[path = "service/compression/mod.rs"]
mod compression;
#[path = "service/observe.rs"]
mod observe;
#[path = "service/policies.rs"]
pub(crate) mod policies;
#[path = "service/runs.rs"]
mod runs;

use crate::context;
use crate::memory::build_memory;
use crate::provider::{build_provider, IcCanisterProvider};
use crate::tools;
use crate::types::{
    Agent, AgentObservation, AgentObserveRequest, ApiError, ApiErrorCode, CanisterConfig,
    ContextConfig, ConversationSummaryGetRequest, HealthResponse, MemoryCountResult,
    MemoryForgetRequest, MemoryForgetResult, MemoryGetRequest, MemoryItem, MemoryListRequest,
    MemoryRecallRequest, MemoryStoreRequest, ProviderConfig, Run,
    RunCancelRequest, RunCreateRequest, RunEvent, RunEventsGetRequest, RunGetRequest,
    RunListRequest, Session, SessionGetRequest, UnitResult,
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
    async fn sessions_list(&self, agent_id: Option<String>) -> Result<Vec<Session>, ApiError>;
    async fn session_get(&self, request: SessionGetRequest) -> Result<Option<Session>, ApiError>;
    async fn run_create(&self, request: RunCreateRequest) -> Result<Run, ApiError>;
    async fn run_get(&self, request: RunGetRequest) -> Result<Option<Run>, ApiError>;
    async fn run_list(&self, request: RunListRequest) -> Result<Vec<Run>, ApiError>;
    async fn run_events_get(&self, request: RunEventsGetRequest) -> Result<Vec<RunEvent>, ApiError>;
    async fn run_cancel(&self, request: RunCancelRequest) -> Result<bool, ApiError>;
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
}

pub fn init_service(config: Option<CanisterConfig>) {
    SERVICE.with(|service| {
        *service.borrow_mut() = Arc::new(ConnectedIclawIcService::new(config));
    });
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
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct RunExecutionResult {
    pub response: String,
    pub session_id: Option<String>,
    pub model: Option<String>,
    pub provider_ready: bool,
    pub memory_ready: bool,
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

    async fn execute_run(&self, request: RunExecutionRequest) -> Result<RunExecutionResult, ApiError> {
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
        agent::autosave_turn(
            self.memory.as_ref(),
            request.session_id.as_deref(),
            "user",
            &request.prompt,
            self.context_config.as_ref(),
        )
        .await
        .map_err(Self::memory_error)?;
        let mut messages =
            agent::build_messages(&prompt_context, &session_summary, &history, &request.prompt);
        let mut compression_state = compression::CompressionState::default();
        let response = agent::run_tool_loop(
            provider,
            &self.tools,
            &mut messages,
            &model,
            temperature,
            self.context_config.as_ref(),
            &mut compression_state,
        )
        .await
        .map_err(Self::provider_error)?;
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
            response: response.response.text_or_empty().to_string(),
            session_id: request.session_id,
            model: response.model.or(Some(model)),
            provider_ready: true,
            memory_ready: self.memory_ready,
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
        Ok(vec![runs::default_agent()])
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

    async fn run_create(&self, request: RunCreateRequest) -> Result<Run, ApiError> {
        if request.prompt.trim().is_empty() {
            return Err(Self::invalid_argument("prompt must not be empty"));
        }

        let memory = self.memory()?;
        let session = runs::ensure_session(
            memory,
            request.agent_id.as_deref(),
            request.session_id.as_deref(),
            &request.prompt,
        )
        .await
        .map_err(Self::memory_error)?;
        let mut run = runs::create_run_record(
            memory,
            &session,
            &request.prompt,
            request.model.as_deref(),
            self.provider.is_some(),
            self.memory_ready,
        )
        .await
        .map_err(Self::memory_error)?;
        runs::append_run_event(memory, &run, "queued", "run queued")
            .await
            .map_err(Self::memory_error)?;
        run = runs::mark_run_started(memory, &run)
            .await
            .map_err(Self::memory_error)?;
        runs::append_run_event(memory, &run, "started", "run started")
            .await
            .map_err(Self::memory_error)?;

        let chat_result = self
            .execute_run(RunExecutionRequest {
                prompt: request.prompt,
                session_id: Some(session.id.clone()),
                model: request.model,
                temperature: request.temperature,
            })
            .await;

        let persisted_run = self.latest_run_or_memory_error(memory, &run.id).await?;
        if persisted_run.status == "cancelled" {
            return Ok(persisted_run);
        }

        let terminal_run = match &chat_result {
            Ok(response) => {
                runs::mark_run_completed(
                    memory,
                    &persisted_run,
                    response.response.as_str(),
                    response.model.as_deref(),
                    response.provider_ready,
                    response.memory_ready,
                )
                .await
                .map_err(Self::memory_error)
            }
            Err(error) => {
                runs::mark_run_failed(
                    memory,
                    &persisted_run,
                    &error.message,
                    self.provider.is_some(),
                    self.memory_ready,
                )
                .await
                .map_err(Self::memory_error)
            }
        }?;

        match &chat_result {
            Ok(response) => {
                runs::append_run_event(memory, &terminal_run, "assistant_message", &response.response)
                    .await
                    .map_err(Self::memory_error)?;
                runs::append_run_event(memory, &terminal_run, "completed", "run completed")
                    .await
                    .map_err(Self::memory_error)?;
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

    async fn run_events_get(&self, request: RunEventsGetRequest) -> Result<Vec<RunEvent>, ApiError> {
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
