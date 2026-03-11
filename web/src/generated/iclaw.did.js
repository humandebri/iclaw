export const idlFactory = ({ IDL }) => {
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
  const ApiError = IDL.Record({ 'code' : IDL.Text, 'message' : IDL.Text });
  const Result = IDL.Variant({ 'Ok' : AgentObservation, 'Err' : ApiError });
  const ChatRequest = IDL.Record({
    'model' : IDL.Opt(IDL.Text),
    'session_id' : IDL.Opt(IDL.Text),
    'temperature' : IDL.Opt(IDL.Float64),
    'prompt' : IDL.Text,
  });
  const ChatResponse = IDL.Record({
    'model' : IDL.Opt(IDL.Text),
    'session_id' : IDL.Opt(IDL.Text),
    'memory_ready' : IDL.Bool,
    'provider_ready' : IDL.Bool,
    'response' : IDL.Text,
  });
  const Result_1 = IDL.Variant({ 'Ok' : ChatResponse, 'Err' : ApiError });
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
  const Result_2 = IDL.Variant({
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
  const Result_3 = IDL.Variant({ 'Ok' : IDL.Nat64, 'Err' : ApiError });
  const MemoryForgetRequest = IDL.Record({ 'key' : IDL.Text });
  const Result_4 = IDL.Variant({ 'Ok' : IDL.Bool, 'Err' : ApiError });
  const MemoryGetRequest = IDL.Record({ 'key' : IDL.Text });
  const MemoryListRequest = IDL.Record({
    'session_id' : IDL.Opt(IDL.Text),
    'category' : IDL.Opt(MemoryCategory),
  });
  const Result_5 = IDL.Variant({
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
  const Result_6 = IDL.Variant({ 'Ok' : IDL.Null, 'Err' : ApiError });
  return IDL.Service({
    'agent_observe' : IDL.Func([AgentObserveRequest], [Result], ['query']),
    'chat' : IDL.Func([ChatRequest], [Result_1], []),
    'conversation_summary_get' : IDL.Func(
        [ConversationSummaryGetRequest],
        [Result_2],
        ['query'],
      ),
    'health' : IDL.Func([], [HealthResponse], ['query']),
    'http_request' : IDL.Func([HttpRequest], [HttpResponse], ['query']),
    'memory_count' : IDL.Func([], [Result_3], ['query']),
    'memory_forget' : IDL.Func([MemoryForgetRequest], [Result_4], []),
    'memory_get' : IDL.Func([MemoryGetRequest], [Result_2], ['query']),
    'memory_list' : IDL.Func([MemoryListRequest], [Result_5], ['query']),
    'memory_recall' : IDL.Func([MemoryRecallRequest], [Result_5], ['query']),
    'memory_store' : IDL.Func([MemoryStoreRequest], [Result_6], []),
  });
};
export const init = ({ IDL }) => {
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
