import { IDL } from '@icp-sdk/core/candid';

export const idlFactory: IDL.InterfaceFactory = ({ IDL }) => {
  const ContextConfig = IDL.Record({
    'enable_conversation_summary' : IDL.Opt(IDL.Bool),
    'cycle_balance_warning_threshold' : IDL.Opt(IDL.Nat64),
    'llm_summary_model' : IDL.Opt(IDL.Text),
    'skills_dir' : IDL.Opt(IDL.Text),
    'memory_recall_limit' : IDL.Opt(IDL.Nat64),
    'llm_summary_request_bytes_threshold' : IDL.Opt(IDL.Nat64),
    'retry_provider_once' : IDL.Opt(IDL.Bool),
    'history_limit' : IDL.Opt(IDL.Nat64),
    'summary_max_chars' : IDL.Opt(IDL.Nat64),
    'llm_summary_max_chars' : IDL.Opt(IDL.Nat64),
    'max_tool_iterations' : IDL.Opt(IDL.Nat64),
    'enable_tool_loop' : IDL.Opt(IDL.Bool),
    'max_static_context_chars' : IDL.Opt(IDL.Nat64),
    'max_skill_context_chars' : IDL.Opt(IDL.Nat64),
    'workspace_files' : IDL.Opt(IDL.Vec(IDL.Text)),
    'enable_autosave' : IDL.Opt(IDL.Bool),
    'llm_summary_on_overflow' : IDL.Opt(IDL.Bool),
    'enable_lightweight_skill_actions' : IDL.Opt(IDL.Bool),
    'max_prompt_chars' : IDL.Opt(IDL.Nat64),
    'memory_min_score' : IDL.Opt(IDL.Float64),
    'max_request_bytes_budget' : IDL.Opt(IDL.Nat64),
    'enable_auto_promote' : IDL.Opt(IDL.Bool),
  });
  const ProviderConfig = IDL.Record({
    'api_key' : IDL.Text,
    'api_url' : IDL.Text,
    'default_model' : IDL.Text,
    'timeout_secs' : IDL.Opt(IDL.Nat64),
  });
  const CanisterConfig = IDL.Record({
    'allowed_principals' : IDL.Opt(IDL.Vec(IDL.Principal)),
    'context' : IDL.Opt(ContextConfig),
    'provider' : IDL.Opt(ProviderConfig),
  });
  const AgentDraft = IDL.Record({
    'id' : IDL.Text,
    'status' : IDL.Text,
    'requires_tool_approval' : IDL.Bool,
    'name' : IDL.Text,
    'description' : IDL.Text,
    'system_prompt_override' : IDL.Opt(IDL.Text),
    'enabled_tool_names' : IDL.Vec(IDL.Text),
  });
  const AgentCreateRequest = IDL.Record({ 'draft' : AgentDraft });
  const Agent = IDL.Record({
    'id' : IDL.Text,
    'status' : IDL.Text,
    'requires_tool_approval' : IDL.Bool,
    'name' : IDL.Text,
    'description' : IDL.Text,
    'system_prompt_override' : IDL.Opt(IDL.Text),
    'enabled_tool_names' : IDL.Vec(IDL.Text),
  });
  const ApiError = IDL.Record({ 'code' : IDL.Text, 'message' : IDL.Text });
  const Result = IDL.Variant({ 'Ok' : Agent, 'Err' : ApiError });
  const AgentGetRequest = IDL.Record({ 'agent_id' : IDL.Text });
  const Result_1 = IDL.Variant({ 'Ok' : IDL.Opt(Agent), 'Err' : ApiError });
  const AgentObserveRequest = IDL.Record({ 'session_id' : IDL.Opt(IDL.Text) });
  const AgentObservation = IDL.Record({
    'conversation_turn_count' : IDL.Nat64,
    'enable_conversation_summary' : IDL.Bool,
    'auto_promoted_keys' : IDL.Vec(IDL.Text),
    'core_keys' : IDL.Vec(IDL.Text),
    'conversation_summary_key' : IDL.Opt(IDL.Text),
    'tool_loop_enabled' : IDL.Bool,
    'history_limit' : IDL.Nat64,
    'max_tool_iterations' : IDL.Nat64,
    'conversation_summary_present' : IDL.Bool,
    'workspace_keys' : IDL.Vec(IDL.Text),
    'enable_auto_promote' : IDL.Bool,
  });
  const Result_2 = IDL.Variant({ 'Ok' : AgentObservation, 'Err' : ApiError });
  const AgentUpdateRequest = IDL.Record({ 'agent' : Agent });
  const Result_3 = IDL.Variant({ 'Ok' : IDL.Vec(Agent), 'Err' : ApiError });
  const AllowedPrincipalsResponse = IDL.Record({
    'allowed_principals' : IDL.Vec(IDL.Principal),
  });
  const Result_4 = IDL.Variant({
    'Ok' : AllowedPrincipalsResponse,
    'Err' : ApiError,
  });
  const ConversationSummaryGetRequest = IDL.Record({ 'session_id' : IDL.Text });
  const MemoryCategory = IDL.Variant({
    'custom' : IDL.Text,
    'core' : IDL.Null,
    'conversation' : IDL.Null,
    'daily' : IDL.Null,
  });
  const MemoryItem = IDL.Record({
    'id' : IDL.Text,
    'key' : IDL.Text,
    'content' : IDL.Text,
    'session_id' : IDL.Opt(IDL.Text),
    'score' : IDL.Opt(IDL.Float64),
    'timestamp' : IDL.Text,
    'category' : MemoryCategory,
  });
  const Result_5 = IDL.Variant({
    'Ok' : IDL.Opt(MemoryItem),
    'Err' : ApiError,
  });
  const HealthResponse = IDL.Record({
    'status' : IDL.Text,
    'memory_ready' : IDL.Bool,
    'provider_ready' : IDL.Bool,
    'version' : IDL.Text,
    'runtime' : IDL.Text,
  });
  const HttpRequest = IDL.Record({
    'url' : IDL.Text,
    'method' : IDL.Text,
    'body' : IDL.Vec(IDL.Nat8),
    'headers' : IDL.Vec(IDL.Tuple(IDL.Text, IDL.Text)),
    'certificate_version' : IDL.Opt(IDL.Nat16),
  });
  const HttpResponse = IDL.Record({
    'body' : IDL.Vec(IDL.Nat8),
    'headers' : IDL.Vec(IDL.Tuple(IDL.Text, IDL.Text)),
    'upgrade' : IDL.Opt(IDL.Bool),
    'status_code' : IDL.Nat16,
  });
  const Result_6 = IDL.Variant({ 'Ok' : IDL.Nat64, 'Err' : ApiError });
  const MemoryForgetRequest = IDL.Record({ 'key' : IDL.Text });
  const Result_7 = IDL.Variant({ 'Ok' : IDL.Bool, 'Err' : ApiError });
  const MemoryGetRequest = IDL.Record({ 'key' : IDL.Text });
  const MemoryListRequest = IDL.Record({
    'session_id' : IDL.Opt(IDL.Text),
    'category' : IDL.Opt(MemoryCategory),
  });
  const Result_8 = IDL.Variant({
    'Ok' : IDL.Vec(MemoryItem),
    'Err' : ApiError,
  });
  const MemoryRecallRequest = IDL.Record({
    'session_id' : IDL.Opt(IDL.Text),
    'query' : IDL.Text,
    'limit' : IDL.Nat64,
  });
  const MemoryStoreRequest = IDL.Record({
    'key' : IDL.Text,
    'content' : IDL.Text,
    'session_id' : IDL.Opt(IDL.Text),
    'category' : MemoryCategory,
  });
  const Result_9 = IDL.Variant({ 'Ok' : IDL.Null, 'Err' : ApiError });
  const RunCancelRequest = IDL.Record({ 'run_id' : IDL.Text });
  const RunCreateRequest = IDL.Record({
    'model' : IDL.Opt(IDL.Text),
    'session_id' : IDL.Opt(IDL.Text),
    'temperature' : IDL.Opt(IDL.Float64),
    'agent_id' : IDL.Opt(IDL.Text),
    'prompt' : IDL.Text,
  });
  const PendingToolCall = IDL.Record({
    'id' : IDL.Text,
    'name' : IDL.Text,
    'arguments' : IDL.Text,
  });
  const Run = IDL.Record({
    'id' : IDL.Text,
    'status' : IDL.Text,
    'model' : IDL.Opt(IDL.Text),
    'pending_assistant_text' : IDL.Opt(IDL.Text),
    'session_id' : IDL.Text,
    'memory_ready' : IDL.Bool,
    'provider_ready' : IDL.Bool,
    'created_at' : IDL.Text,
    'agent_id' : IDL.Text,
    'error' : IDL.Opt(IDL.Text),
    'response' : IDL.Opt(IDL.Text),
    'prompt' : IDL.Text,
    'pending_tool_calls' : IDL.Vec(PendingToolCall),
    'trigger_id' : IDL.Opt(IDL.Text),
    'trigger_kind' : IDL.Text,
    'started_at' : IDL.Opt(IDL.Text),
    'finished_at' : IDL.Opt(IDL.Text),
  });
  const Result_10 = IDL.Variant({ 'Ok' : Run, 'Err' : ApiError });
  const RunEvent = IDL.Record({
    'id' : IDL.Text,
    'run_id' : IDL.Text,
    'kind' : IDL.Text,
    'message' : IDL.Text,
    'timestamp' : IDL.Text,
  });
  const Result_11 = IDL.Variant({ 'Ok' : IDL.Vec(RunEvent), 'Err' : ApiError });
  const RunGetRequest = IDL.Record({ 'run_id' : IDL.Text });
  const Result_12 = IDL.Variant({ 'Ok' : IDL.Opt(Run), 'Err' : ApiError });
  const RunListRequest = IDL.Record({
    'session_id' : IDL.Opt(IDL.Text),
    'limit' : IDL.Opt(IDL.Nat64),
  });
  const Result_13 = IDL.Variant({ 'Ok' : IDL.Vec(Run), 'Err' : ApiError });
  const RunResumeRequest = IDL.Record({ 'run_id' : IDL.Text });
  const ScheduleDraft = IDL.Record({
    'id' : IDL.Text,
    'name' : IDL.Text,
    'agent_id' : IDL.Text,
    'enabled' : IDL.Bool,
    'interval_minutes' : IDL.Nat64,
    'prompt' : IDL.Text,
    'fixed_session_id' : IDL.Opt(IDL.Text),
    'session_mode' : IDL.Text,
  });
  const ScheduleCreateRequest = IDL.Record({ 'draft' : ScheduleDraft });
  const Schedule = IDL.Record({
    'id' : IDL.Text,
    'last_error' : IDL.Opt(IDL.Text),
    'updated_at' : IDL.Text,
    'name' : IDL.Text,
    'created_at' : IDL.Text,
    'agent_id' : IDL.Text,
    'last_success_at' : IDL.Opt(IDL.Text),
    'enabled' : IDL.Bool,
    'last_started_at' : IDL.Opt(IDL.Text),
    'interval_minutes' : IDL.Nat64,
    'consecutive_failure_count' : IDL.Nat64,
    'last_run_id' : IDL.Opt(IDL.Text),
    'next_run_at' : IDL.Opt(IDL.Text),
    'prompt' : IDL.Text,
    'fixed_session_id' : IDL.Opt(IDL.Text),
    'running' : IDL.Bool,
    'last_finished_at' : IDL.Opt(IDL.Text),
    'session_mode' : IDL.Text,
  });
  const Result_22 = IDL.Variant({ 'Ok' : Schedule, 'Err' : ApiError });
  const ScheduleGetRequest = IDL.Record({ 'schedule_id' : IDL.Text });
  const Result_23 = IDL.Variant({ 'Ok' : IDL.Opt(Schedule), 'Err' : ApiError });
  const ScheduleUpdateRequest = IDL.Record({ 'schedule' : Schedule });
  const Result_24 = IDL.Variant({ 'Ok' : IDL.Vec(Schedule), 'Err' : ApiError });
  const SessionGetRequest = IDL.Record({ 'session_id' : IDL.Text });
  const Session = IDL.Record({
    'id' : IDL.Text,
    'title' : IDL.Text,
    'updated_at' : IDL.Text,
    'created_at' : IDL.Text,
    'agent_id' : IDL.Text,
    'last_run_id' : IDL.Opt(IDL.Text),
  });
  const Result_14 = IDL.Variant({ 'Ok' : IDL.Opt(Session), 'Err' : ApiError });
  const Result_15 = IDL.Variant({ 'Ok' : IDL.Vec(Session), 'Err' : ApiError });
  const ToolPolicyListRequest = IDL.Record({ 'agent_id' : IDL.Opt(IDL.Text) });
  const ToolPolicy = IDL.Record({
    'agent_id' : IDL.Text,
    'enabled' : IDL.Bool,
    'tool_name' : IDL.Text,
    'requires_approval' : IDL.Bool,
  });
  const Result_16 = IDL.Variant({
    'Ok' : IDL.Vec(ToolPolicy),
    'Err' : ApiError,
  });
  const ToolPolicyUpdateRequest = IDL.Record({ 'policy' : ToolPolicy });
  const Result_17 = IDL.Variant({ 'Ok' : ToolPolicy, 'Err' : ApiError });
  const WebhookDraft = IDL.Record({
    'id' : IDL.Text,
    'name' : IDL.Text,
    'secret' : IDL.Text,
    'agent_id' : IDL.Text,
    'enabled' : IDL.Bool,
    'fixed_session_id' : IDL.Opt(IDL.Text),
    'session_mode' : IDL.Text,
  });
  const WebhookCreateRequest = IDL.Record({ 'draft' : WebhookDraft });
  const Webhook = IDL.Record({
    'id' : IDL.Text,
    'updated_at' : IDL.Text,
    'last_secret_rotated_at' : IDL.Opt(IDL.Text),
    'name' : IDL.Text,
    'secret' : IDL.Text,
    'created_at' : IDL.Text,
    'agent_id' : IDL.Text,
    'enabled' : IDL.Bool,
    'last_run_id' : IDL.Opt(IDL.Text),
    'last_invoked_at' : IDL.Opt(IDL.Text),
    'last_rejection_reason' : IDL.Opt(IDL.Text),
    'last_rejection_at' : IDL.Opt(IDL.Text),
    'fixed_session_id' : IDL.Opt(IDL.Text),
    'session_mode' : IDL.Text,
  });
  const Result_18 = IDL.Variant({ 'Ok' : Webhook, 'Err' : ApiError });
  const WebhookGetRequest = IDL.Record({ 'webhook_id' : IDL.Text });
  const Result_19 = IDL.Variant({ 'Ok' : IDL.Opt(Webhook), 'Err' : ApiError });
  const WebhookInvokeRequest = IDL.Record({
    'model' : IDL.Opt(IDL.Text),
    'session_id' : IDL.Opt(IDL.Text),
    'temperature' : IDL.Opt(IDL.Float64),
    'secret' : IDL.Text,
    'prompt' : IDL.Text,
    'webhook_id' : IDL.Text,
  });
  const WebhookRejectionsListRequest = IDL.Record({
    'limit' : IDL.Opt(IDL.Nat64),
    'webhook_id' : IDL.Text,
  });
  const WebhookRejection = IDL.Record({
    'id' : IDL.Text,
    'timestamp' : IDL.Text,
    'webhook_id' : IDL.Text,
    'reason' : IDL.Text,
  });
  const Result_20 = IDL.Variant({
    'Ok' : IDL.Vec(WebhookRejection),
    'Err' : ApiError,
  });
  const WebhookSecretRotateRequest = IDL.Record({ 'webhook_id' : IDL.Text });
  const WebhookSecretRotateResponse = IDL.Record({
    'new_secret' : IDL.Text,
    'webhook' : Webhook,
  });
  const Result_25 = IDL.Variant({
    'Ok' : WebhookSecretRotateResponse,
    'Err' : ApiError,
  });
  const WebhookUpdateRequest = IDL.Record({
    'webhook' : Webhook,
    'secret_override' : IDL.Opt(IDL.Text),
  });
  const Result_21 = IDL.Variant({ 'Ok' : IDL.Vec(Webhook), 'Err' : ApiError });
  return IDL.Service({
    'agent_create' : IDL.Func([AgentCreateRequest], [Result], []),
    'agent_get' : IDL.Func([AgentGetRequest], [Result_1], ['query']),
    'agent_observe' : IDL.Func([AgentObserveRequest], [Result_2], ['query']),
    'agent_update' : IDL.Func([AgentUpdateRequest], [Result], []),
    'agents_list' : IDL.Func([], [Result_3], ['query']),
    'allowed_principals_get' : IDL.Func([], [Result_4], ['query']),
    'allowed_principals_set' : IDL.Func(
        [AllowedPrincipalsResponse],
        [Result_4],
        [],
      ),
    'conversation_summary_get' : IDL.Func(
        [ConversationSummaryGetRequest],
        [Result_5],
        ['query'],
      ),
    'health' : IDL.Func([], [HealthResponse], ['query']),
    'http_request' : IDL.Func([HttpRequest], [HttpResponse], ['query']),
    'memory_count' : IDL.Func([], [Result_6], ['query']),
    'memory_forget' : IDL.Func([MemoryForgetRequest], [Result_7], []),
    'memory_get' : IDL.Func([MemoryGetRequest], [Result_5], ['query']),
    'memory_list' : IDL.Func([MemoryListRequest], [Result_8], ['query']),
    'memory_recall' : IDL.Func([MemoryRecallRequest], [Result_8], ['query']),
    'memory_store' : IDL.Func([MemoryStoreRequest], [Result_9], []),
    'run_cancel' : IDL.Func([RunCancelRequest], [Result_7], []),
    'run_create' : IDL.Func([RunCreateRequest], [Result_10], []),
    'run_events_get' : IDL.Func([RunCancelRequest], [Result_11], ['query']),
    'run_get' : IDL.Func([RunGetRequest], [Result_12], ['query']),
    'run_list' : IDL.Func([RunListRequest], [Result_13], ['query']),
    'run_resume' : IDL.Func([RunResumeRequest], [Result_10], []),
    'schedule_create' : IDL.Func([ScheduleCreateRequest], [Result_22], []),
    'schedule_delete' : IDL.Func([ScheduleGetRequest], [Result_7], []),
    'schedule_get' : IDL.Func([ScheduleGetRequest], [Result_23], ['query']),
    'schedule_trigger' : IDL.Func([ScheduleGetRequest], [Result_10], []),
    'schedule_update' : IDL.Func([ScheduleUpdateRequest], [Result_22], []),
    'schedules_list' : IDL.Func([], [Result_24], ['query']),
    'session_get' : IDL.Func([SessionGetRequest], [Result_14], ['query']),
    'sessions_list' : IDL.Func([IDL.Opt(IDL.Text)], [Result_15], ['query']),
    'tool_policy_list' : IDL.Func(
        [ToolPolicyListRequest],
        [Result_16],
        ['query'],
      ),
    'tool_policy_update' : IDL.Func([ToolPolicyUpdateRequest], [Result_17], []),
    'webhook_create' : IDL.Func([WebhookCreateRequest], [Result_18], []),
    'webhook_delete' : IDL.Func([WebhookGetRequest], [Result_7], []),
    'webhook_get' : IDL.Func([WebhookGetRequest], [Result_19], ['query']),
    'webhook_invoke' : IDL.Func([WebhookInvokeRequest], [Result_10], []),
    'webhook_rejections_list' : IDL.Func(
        [WebhookRejectionsListRequest],
        [Result_20],
        ['query'],
      ),
    'webhook_rotate_secret' : IDL.Func(
        [WebhookSecretRotateRequest],
        [Result_25],
        [],
      ),
    'webhook_update' : IDL.Func([WebhookUpdateRequest], [Result_18], []),
    'webhooks_list' : IDL.Func([], [Result_21], ['query']),
  });
};
export const init = ({ IDL }: Parameters<IDL.InterfaceFactory>[0]) => {
  const ContextConfig = IDL.Record({
    'enable_conversation_summary' : IDL.Opt(IDL.Bool),
    'cycle_balance_warning_threshold' : IDL.Opt(IDL.Nat64),
    'llm_summary_model' : IDL.Opt(IDL.Text),
    'skills_dir' : IDL.Opt(IDL.Text),
    'memory_recall_limit' : IDL.Opt(IDL.Nat64),
    'llm_summary_request_bytes_threshold' : IDL.Opt(IDL.Nat64),
    'retry_provider_once' : IDL.Opt(IDL.Bool),
    'history_limit' : IDL.Opt(IDL.Nat64),
    'summary_max_chars' : IDL.Opt(IDL.Nat64),
    'llm_summary_max_chars' : IDL.Opt(IDL.Nat64),
    'max_tool_iterations' : IDL.Opt(IDL.Nat64),
    'enable_tool_loop' : IDL.Opt(IDL.Bool),
    'max_static_context_chars' : IDL.Opt(IDL.Nat64),
    'max_skill_context_chars' : IDL.Opt(IDL.Nat64),
    'workspace_files' : IDL.Opt(IDL.Vec(IDL.Text)),
    'enable_autosave' : IDL.Opt(IDL.Bool),
    'llm_summary_on_overflow' : IDL.Opt(IDL.Bool),
    'enable_lightweight_skill_actions' : IDL.Opt(IDL.Bool),
    'max_prompt_chars' : IDL.Opt(IDL.Nat64),
    'memory_min_score' : IDL.Opt(IDL.Float64),
    'max_request_bytes_budget' : IDL.Opt(IDL.Nat64),
    'enable_auto_promote' : IDL.Opt(IDL.Bool),
  });
  const ProviderConfig = IDL.Record({
    'api_key' : IDL.Text,
    'api_url' : IDL.Text,
    'default_model' : IDL.Text,
    'timeout_secs' : IDL.Opt(IDL.Nat64),
  });
  const CanisterConfig = IDL.Record({
    'allowed_principals' : IDL.Opt(IDL.Vec(IDL.Principal)),
    'context' : IDL.Opt(ContextConfig),
    'provider' : IDL.Opt(ProviderConfig),
  });
  return [IDL.Opt(CanisterConfig)];
};
