import type { Principal } from '@icp-sdk/core/principal';
import type { ActorMethod } from '@icp-sdk/core/agent';
import { IDL } from '@icp-sdk/core/candid';
import { idlFactory as runtimeIdlFactory, init as runtimeInit } from './declarations_runtime.js';

export interface Agent {
  'id' : string,
  'status' : string,
  'requires_tool_approval' : boolean,
  'name' : string,
  'description' : string,
  'system_prompt_override' : [] | [string],
  'enabled_tool_names' : Array<string>,
}
export interface AgentCreateRequest { 'draft' : AgentDraft }
export interface AgentDraft {
  'id' : string,
  'status' : string,
  'requires_tool_approval' : boolean,
  'name' : string,
  'description' : string,
  'system_prompt_override' : [] | [string],
  'enabled_tool_names' : Array<string>,
}
export interface AgentGetRequest { 'agent_id' : string }
export interface AgentObservation {
  'conversation_turn_count' : bigint,
  'enable_conversation_summary' : boolean,
  'auto_promoted_keys' : Array<string>,
  'core_keys' : Array<string>,
  'conversation_summary_key' : [] | [string],
  'tool_loop_enabled' : boolean,
  'history_limit' : bigint,
  'max_tool_iterations' : bigint,
  'conversation_summary_present' : boolean,
  'workspace_keys' : Array<string>,
  'enable_auto_promote' : boolean,
}
export interface AgentObserveRequest { 'session_id' : [] | [string] }
export interface AgentUpdateRequest { 'agent' : Agent }
export interface AllowedPrincipalsResponse {
  'allowed_principals' : Array<Principal>,
}
export interface ApiError { 'code' : string, 'message' : string }
export interface CanisterConfig {
  'allowed_principals' : [] | [Array<Principal>],
  'context' : [] | [ContextConfig],
  'provider' : [] | [ProviderConfig],
}
export interface ContextConfig {
  'enable_conversation_summary' : [] | [boolean],
  'cycle_balance_warning_threshold' : [] | [bigint],
  'llm_summary_model' : [] | [string],
  'skills_dir' : [] | [string],
  'memory_recall_limit' : [] | [bigint],
  'llm_summary_request_bytes_threshold' : [] | [bigint],
  'retry_provider_once' : [] | [boolean],
  'history_limit' : [] | [bigint],
  'summary_max_chars' : [] | [bigint],
  'llm_summary_max_chars' : [] | [bigint],
  'max_tool_iterations' : [] | [bigint],
  'enable_tool_loop' : [] | [boolean],
  'max_static_context_chars' : [] | [bigint],
  'max_skill_context_chars' : [] | [bigint],
  'workspace_files' : [] | [Array<string>],
  'enable_autosave' : [] | [boolean],
  'llm_summary_on_overflow' : [] | [boolean],
  'enable_lightweight_skill_actions' : [] | [boolean],
  'max_prompt_chars' : [] | [bigint],
  'memory_min_score' : [] | [number],
  'max_request_bytes_budget' : [] | [bigint],
  'enable_auto_promote' : [] | [boolean],
}
export interface ConversationSummaryGetRequest { 'session_id' : string }
export interface HealthResponse {
  'status' : string,
  'memory_ready' : boolean,
  'provider_ready' : boolean,
  'version' : string,
  'runtime' : string,
}
export interface HttpRequest {
  'url' : string,
  'method' : string,
  'body' : Uint8Array | number[],
  'headers' : Array<[string, string]>,
  'certificate_version' : [] | [number],
}
export interface HttpResponse {
  'body' : Uint8Array | number[],
  'headers' : Array<[string, string]>,
  'upgrade' : [] | [boolean],
  'status_code' : number,
}
export type MemoryCategory = { 'custom' : string } |
  { 'core' : null } |
  { 'conversation' : null } |
  { 'daily' : null };
export interface MemoryForgetRequest { 'key' : string }
export interface MemoryGetRequest { 'key' : string }
export interface MemoryItem {
  'id' : string,
  'key' : string,
  'content' : string,
  'session_id' : [] | [string],
  'score' : [] | [number],
  'timestamp' : string,
  'category' : MemoryCategory,
}
export interface MemoryListRequest {
  'session_id' : [] | [string],
  'category' : [] | [MemoryCategory],
}
export interface MemoryRecallRequest {
  'session_id' : [] | [string],
  'query' : string,
  'limit' : bigint,
}
export interface MemoryStoreRequest {
  'key' : string,
  'content' : string,
  'session_id' : [] | [string],
  'category' : MemoryCategory,
}
export interface PendingToolCall {
  'id' : string,
  'name' : string,
  'arguments' : string,
}
export interface ProviderConfig {
  'api_key' : string,
  'api_url' : string,
  'default_model' : string,
  'timeout_secs' : [] | [bigint],
}
export type Result = { 'Ok' : Agent } |
  { 'Err' : ApiError };
export type Result_1 = { 'Ok' : [] | [Agent] } |
  { 'Err' : ApiError };
export type Result_10 = { 'Ok' : Run } |
  { 'Err' : ApiError };
export type Result_11 = { 'Ok' : Array<RunEvent> } |
  { 'Err' : ApiError };
export type Result_12 = { 'Ok' : [] | [Run] } |
  { 'Err' : ApiError };
export type Result_13 = { 'Ok' : Array<Run> } |
  { 'Err' : ApiError };
export type Result_14 = { 'Ok' : [] | [Session] } |
  { 'Err' : ApiError };
export type Result_15 = { 'Ok' : Array<Session> } |
  { 'Err' : ApiError };
export type Result_16 = { 'Ok' : Array<ToolPolicy> } |
  { 'Err' : ApiError };
export type Result_17 = { 'Ok' : ToolPolicy } |
  { 'Err' : ApiError };
export type Result_18 = { 'Ok' : Webhook } |
  { 'Err' : ApiError };
export type Result_19 = { 'Ok' : [] | [Webhook] } |
  { 'Err' : ApiError };
export type Result_2 = { 'Ok' : AgentObservation } |
  { 'Err' : ApiError };
export type Result_20 = { 'Ok' : Array<WebhookRejection> } |
  { 'Err' : ApiError };
export type Result_21 = { 'Ok' : Array<Webhook> } |
  { 'Err' : ApiError };
export type Result_22 = { 'Ok' : Schedule } |
  { 'Err' : ApiError };
export type Result_23 = { 'Ok' : [] | [Schedule] } |
  { 'Err' : ApiError };
export type Result_24 = { 'Ok' : Array<Schedule> } |
  { 'Err' : ApiError };
export type Result_25 = { 'Ok' : WebhookSecretRotateResponse } |
  { 'Err' : ApiError };
export type Result_3 = { 'Ok' : Array<Agent> } |
  { 'Err' : ApiError };
export type Result_4 = { 'Ok' : AllowedPrincipalsResponse } |
  { 'Err' : ApiError };
export type Result_5 = { 'Ok' : [] | [MemoryItem] } |
  { 'Err' : ApiError };
export type Result_6 = { 'Ok' : bigint } |
  { 'Err' : ApiError };
export type Result_7 = { 'Ok' : boolean } |
  { 'Err' : ApiError };
export type Result_8 = { 'Ok' : Array<MemoryItem> } |
  { 'Err' : ApiError };
export type Result_9 = { 'Ok' : null } |
  { 'Err' : ApiError };
export interface Run {
  'id' : string,
  'status' : string,
  'model' : [] | [string],
  'pending_assistant_text' : [] | [string],
  'session_id' : string,
  'memory_ready' : boolean,
  'provider_ready' : boolean,
  'created_at' : string,
  'agent_id' : string,
  'error' : [] | [string],
  'response' : [] | [string],
  'prompt' : string,
  'pending_tool_calls' : Array<PendingToolCall>,
  'trigger_id' : [] | [string],
  'trigger_kind' : string,
  'started_at' : [] | [string],
  'finished_at' : [] | [string],
}
export interface RunCancelRequest { 'run_id' : string }
export interface RunCreateRequest {
  'model' : [] | [string],
  'session_id' : [] | [string],
  'temperature' : [] | [number],
  'agent_id' : [] | [string],
  'prompt' : string,
}
export interface RunEvent {
  'id' : string,
  'run_id' : string,
  'kind' : string,
  'message' : string,
  'timestamp' : string,
}
export interface RunGetRequest { 'run_id' : string }
export interface RunListRequest {
  'session_id' : [] | [string],
  'limit' : [] | [bigint],
}
export interface RunResumeRequest { 'run_id' : string }
export interface Schedule {
  'id' : string,
  'last_error' : [] | [string],
  'updated_at' : string,
  'name' : string,
  'created_at' : string,
  'agent_id' : string,
  'last_success_at' : [] | [string],
  'enabled' : boolean,
  'last_started_at' : [] | [string],
  'interval_minutes' : bigint,
  'consecutive_failure_count' : bigint,
  'last_run_id' : [] | [string],
  'next_run_at' : [] | [string],
  'prompt' : string,
  'fixed_session_id' : [] | [string],
  'running' : boolean,
  'last_finished_at' : [] | [string],
  'session_mode' : string,
}
export interface ScheduleCreateRequest { 'draft' : ScheduleDraft }
export interface ScheduleDraft {
  'id' : string,
  'name' : string,
  'agent_id' : string,
  'enabled' : boolean,
  'interval_minutes' : bigint,
  'prompt' : string,
  'fixed_session_id' : [] | [string],
  'session_mode' : string,
}
export interface ScheduleGetRequest { 'schedule_id' : string }
export interface ScheduleUpdateRequest { 'schedule' : Schedule }
export interface Session {
  'id' : string,
  'title' : string,
  'updated_at' : string,
  'created_at' : string,
  'agent_id' : string,
  'last_run_id' : [] | [string],
}
export interface SessionGetRequest { 'session_id' : string }
export interface ToolPolicy {
  'agent_id' : string,
  'enabled' : boolean,
  'tool_name' : string,
  'requires_approval' : boolean,
}
export interface ToolPolicyListRequest { 'agent_id' : [] | [string] }
export interface ToolPolicyUpdateRequest { 'policy' : ToolPolicy }
export interface Webhook {
  'id' : string,
  'updated_at' : string,
  'last_secret_rotated_at' : [] | [string],
  'name' : string,
  'secret' : string,
  'created_at' : string,
  'agent_id' : string,
  'enabled' : boolean,
  'last_run_id' : [] | [string],
  'last_invoked_at' : [] | [string],
  'last_rejection_reason' : [] | [string],
  'last_rejection_at' : [] | [string],
  'fixed_session_id' : [] | [string],
  'session_mode' : string,
}
export interface WebhookCreateRequest { 'draft' : WebhookDraft }
export interface WebhookDraft {
  'id' : string,
  'name' : string,
  'secret' : string,
  'agent_id' : string,
  'enabled' : boolean,
  'fixed_session_id' : [] | [string],
  'session_mode' : string,
}
export interface WebhookGetRequest { 'webhook_id' : string }
export interface WebhookInvokeRequest {
  'model' : [] | [string],
  'session_id' : [] | [string],
  'temperature' : [] | [number],
  'secret' : string,
  'prompt' : string,
  'webhook_id' : string,
}
export interface WebhookRejection {
  'id' : string,
  'timestamp' : string,
  'webhook_id' : string,
  'reason' : string,
}
export interface WebhookRejectionsListRequest {
  'limit' : [] | [bigint],
  'webhook_id' : string,
}
export interface WebhookSecretRotateRequest { 'webhook_id' : string }
export interface WebhookSecretRotateResponse {
  'new_secret' : string,
  'webhook' : Webhook,
}
export interface WebhookUpdateRequest {
  'webhook' : Webhook,
  'secret_override' : [] | [string],
}
export interface _SERVICE {
  'agent_create' : ActorMethod<[AgentCreateRequest], Result>,
  'agent_get' : ActorMethod<[AgentGetRequest], Result_1>,
  'agent_observe' : ActorMethod<[AgentObserveRequest], Result_2>,
  'agent_update' : ActorMethod<[AgentUpdateRequest], Result>,
  'agents_list' : ActorMethod<[], Result_3>,
  'allowed_principals_get' : ActorMethod<[], Result_4>,
  'allowed_principals_set' : ActorMethod<[AllowedPrincipalsResponse], Result_4>,
  'conversation_summary_get' : ActorMethod<
    [ConversationSummaryGetRequest],
    Result_5
  >,
  'health' : ActorMethod<[], HealthResponse>,
  'http_request' : ActorMethod<[HttpRequest], HttpResponse>,
  'memory_count' : ActorMethod<[], Result_6>,
  'memory_forget' : ActorMethod<[MemoryForgetRequest], Result_7>,
  'memory_get' : ActorMethod<[MemoryGetRequest], Result_5>,
  'memory_list' : ActorMethod<[MemoryListRequest], Result_8>,
  'memory_recall' : ActorMethod<[MemoryRecallRequest], Result_8>,
  'memory_store' : ActorMethod<[MemoryStoreRequest], Result_9>,
  'run_cancel' : ActorMethod<[RunCancelRequest], Result_7>,
  'run_create' : ActorMethod<[RunCreateRequest], Result_10>,
  'run_events_get' : ActorMethod<[RunCancelRequest], Result_11>,
  'run_get' : ActorMethod<[RunGetRequest], Result_12>,
  'run_list' : ActorMethod<[RunListRequest], Result_13>,
  'run_resume' : ActorMethod<[RunResumeRequest], Result_10>,
  'schedule_create' : ActorMethod<[ScheduleCreateRequest], Result_22>,
  'schedule_delete' : ActorMethod<[ScheduleGetRequest], Result_7>,
  'schedule_get' : ActorMethod<[ScheduleGetRequest], Result_23>,
  'schedule_trigger' : ActorMethod<[ScheduleGetRequest], Result_10>,
  'schedule_update' : ActorMethod<[ScheduleUpdateRequest], Result_22>,
  'schedules_list' : ActorMethod<[], Result_24>,
  'session_get' : ActorMethod<[SessionGetRequest], Result_14>,
  'sessions_list' : ActorMethod<[[] | [string]], Result_15>,
  'tool_policy_list' : ActorMethod<[ToolPolicyListRequest], Result_16>,
  'tool_policy_update' : ActorMethod<[ToolPolicyUpdateRequest], Result_17>,
  'webhook_create' : ActorMethod<[WebhookCreateRequest], Result_18>,
  'webhook_delete' : ActorMethod<[WebhookGetRequest], Result_7>,
  'webhook_get' : ActorMethod<[WebhookGetRequest], Result_19>,
  'webhook_invoke' : ActorMethod<[WebhookInvokeRequest], Result_10>,
  'webhook_rejections_list' : ActorMethod<
    [WebhookRejectionsListRequest],
    Result_20
  >,
  'webhook_rotate_secret' : ActorMethod<
    [WebhookSecretRotateRequest],
    Result_25
  >,
  'webhook_update' : ActorMethod<[WebhookUpdateRequest], Result_18>,
  'webhooks_list' : ActorMethod<[], Result_21>,
}
export const idlFactory = runtimeIdlFactory;
export const init = runtimeInit;
export const candid = { canisterConfig: init({ IDL })[0] };
