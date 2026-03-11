import type { Principal } from '@dfinity/principal';
import type { ActorMethod } from '@dfinity/agent';
import type { IDL } from '@dfinity/candid';

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
export interface ApiError { 'code' : string, 'message' : string }
export interface CanisterConfig {
  'allowed_principals' : [] | [Array<Principal>],
  'context' : [] | [ContextConfig],
  'provider' : [] | [ProviderConfig],
}
export interface ChatRequest {
  'model' : [] | [string],
  'session_id' : [] | [string],
  'temperature' : [] | [number],
  'prompt' : string,
}
export interface ChatResponse {
  'model' : [] | [string],
  'session_id' : [] | [string],
  'memory_ready' : boolean,
  'provider_ready' : boolean,
  'response' : string,
}
export interface ContextConfig {
  'enable_conversation_summary' : [] | [boolean],
  'cycle_balance_warning_threshold' : [] | [bigint],
  'llm_summary_model' : [] | [string],
  'skills_dir' : [] | [string],
  'memory_recall_limit' : [] | [bigint],
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
export interface ProviderConfig {
  'api_key' : string,
  'api_url' : string,
  'default_model' : string,
  'timeout_secs' : [] | [bigint],
}
export type Result = { 'Ok' : AgentObservation } |
  { 'Err' : ApiError };
export type Result_1 = { 'Ok' : ChatResponse } |
  { 'Err' : ApiError };
export type Result_2 = { 'Ok' : [] | [MemoryItem] } |
  { 'Err' : ApiError };
export type Result_3 = { 'Ok' : bigint } |
  { 'Err' : ApiError };
export type Result_4 = { 'Ok' : boolean } |
  { 'Err' : ApiError };
export type Result_5 = { 'Ok' : Array<MemoryItem> } |
  { 'Err' : ApiError };
export type Result_6 = { 'Ok' : null } |
  { 'Err' : ApiError };
export interface _SERVICE {
  'agent_observe' : ActorMethod<[AgentObserveRequest], Result>,
  'chat' : ActorMethod<[ChatRequest], Result_1>,
  'conversation_summary_get' : ActorMethod<
    [ConversationSummaryGetRequest],
    Result_2
  >,
  'health' : ActorMethod<[], HealthResponse>,
  'http_request' : ActorMethod<[HttpRequest], HttpResponse>,
  'memory_count' : ActorMethod<[], Result_3>,
  'memory_forget' : ActorMethod<[MemoryForgetRequest], Result_4>,
  'memory_get' : ActorMethod<[MemoryGetRequest], Result_2>,
  'memory_list' : ActorMethod<[MemoryListRequest], Result_5>,
  'memory_recall' : ActorMethod<[MemoryRecallRequest], Result_5>,
  'memory_store' : ActorMethod<[MemoryStoreRequest], Result_6>,
}
export declare const idlFactory: IDL.InterfaceFactory;
export declare const init: (args: { IDL: typeof IDL }) => IDL.Type[];
