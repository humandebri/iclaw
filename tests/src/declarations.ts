// where: iclaw/tests/src/declarations.ts
// what: Manual Candid declarations for iclaw_ic PocketIC tests
// why: Keep the integration tests self-contained without a codegen step

import type { ActorMethod } from '@dfinity/pic';
import { IDL } from '@icp-sdk/core/candid';
import type { Principal } from '@icp-sdk/core/principal';

export interface ApiError {
  code: string;
  message: string;
}

export type Result<T> = { Ok: T } | { Err: ApiError };
export type MemoryCategory = { core: null } | { daily: null } | { conversation: null } | { custom: string };

export interface ProviderConfig {
  api_url: string;
  api_key: string;
  default_model: string;
  timeout_secs: [] | [bigint];
}

export interface ContextConfig {
  workspace_files: [] | [string[]];
  skills_dir: [] | [string];
  max_static_context_chars: [] | [bigint];
  max_skill_context_chars: [] | [bigint];
  memory_recall_limit: [] | [bigint];
  memory_min_score: [] | [number];
  enable_lightweight_skill_actions: [] | [boolean];
  history_limit: [] | [bigint];
  max_tool_iterations: [] | [bigint];
  enable_autosave: [] | [boolean];
  enable_tool_loop: [] | [boolean];
  enable_auto_promote: [] | [boolean];
  enable_conversation_summary: [] | [boolean];
  summary_max_chars: [] | [bigint];
  retry_provider_once: [] | [boolean];
  max_prompt_chars: [] | [bigint];
  max_request_bytes_budget: [] | [bigint];
  llm_summary_on_overflow: [] | [boolean];
  llm_summary_model: [] | [string];
  llm_summary_max_chars: [] | [bigint];
  llm_summary_request_bytes_threshold: [] | [bigint];
}

export interface CanisterConfig {
  provider: [] | [ProviderConfig];
  context: [] | [ContextConfig];
  allowed_principals: [] | [Principal[]];
}

export interface ChatRequest {
  prompt: string;
  session_id: [] | [string];
  model: [] | [string];
  temperature: [] | [number];
}

export interface ChatResponse {
  response: string;
  session_id: [] | [string];
  model: [] | [string];
  provider_ready: boolean;
  memory_ready: boolean;
}

export interface HealthResponse {
  status: string;
  version: string;
  runtime: string;
  provider_ready: boolean;
  memory_ready: boolean;
}

export interface ConversationSummaryGetRequest {
  session_id: string;
}

export interface AgentObserveRequest {
  session_id: [] | [string];
}

export interface AgentObservation {
  workspace_keys: string[];
  core_keys: string[];
  conversation_summary_key: [] | [string];
  conversation_summary_present: boolean;
  conversation_turn_count: bigint;
  auto_promoted_keys: string[];
  history_limit: bigint;
  enable_auto_promote: boolean;
  enable_conversation_summary: boolean;
  tool_loop_enabled: boolean;
  max_tool_iterations: bigint;
}

export interface MemoryItem {
  id: string;
  key: string;
  content: string;
  category: MemoryCategory;
  timestamp: string;
  session_id: [] | [string];
  score: [] | [number];
}

export interface MemoryStoreRequest {
  key: string;
  content: string;
  category: MemoryCategory;
  session_id: [] | [string];
}

export interface MemoryRecallRequest {
  query: string;
  limit: bigint;
  session_id: [] | [string];
}

export interface MemoryGetRequest {
  key: string;
}

export interface MemoryListRequest {
  category: [] | [MemoryCategory];
  session_id: [] | [string];
}

export interface MemoryForgetRequest {
  key: string;
}

export interface _SERVICE {
  agent_observe: ActorMethod<[AgentObserveRequest], Result<AgentObservation>>;
  chat: ActorMethod<[ChatRequest], Result<ChatResponse>>;
  conversation_summary_get: ActorMethod<[ConversationSummaryGetRequest], Result<[] | [MemoryItem]>>;
  health: ActorMethod<[], HealthResponse>;
  memory_count: ActorMethod<[], Result<bigint>>;
  memory_forget: ActorMethod<[MemoryForgetRequest], Result<boolean>>;
  memory_get: ActorMethod<[MemoryGetRequest], Result<[] | [MemoryItem]>>;
  memory_list: ActorMethod<[MemoryListRequest], Result<MemoryItem[]>>;
  memory_recall: ActorMethod<[MemoryRecallRequest], Result<MemoryItem[]>>;
  memory_store: ActorMethod<[MemoryStoreRequest], Result<null>>;
}

const apiError = IDL.Record({ code: IDL.Text, message: IDL.Text });
const providerConfig = IDL.Record({
  api_url: IDL.Text,
  api_key: IDL.Text,
  default_model: IDL.Text,
  timeout_secs: IDL.Opt(IDL.Nat64),
});
const contextConfig = IDL.Record({
  skills_dir: IDL.Opt(IDL.Text),
  memory_recall_limit: IDL.Opt(IDL.Nat64),
  history_limit: IDL.Opt(IDL.Nat64),
  max_tool_iterations: IDL.Opt(IDL.Nat64),
  enable_tool_loop: IDL.Opt(IDL.Bool),
  enable_auto_promote: IDL.Opt(IDL.Bool),
  enable_conversation_summary: IDL.Opt(IDL.Bool),
  max_static_context_chars: IDL.Opt(IDL.Nat64),
  max_skill_context_chars: IDL.Opt(IDL.Nat64),
  summary_max_chars: IDL.Opt(IDL.Nat64),
  retry_provider_once: IDL.Opt(IDL.Bool),
  workspace_files: IDL.Opt(IDL.Vec(IDL.Text)),
  enable_autosave: IDL.Opt(IDL.Bool),
  max_prompt_chars: IDL.Opt(IDL.Nat64),
  max_request_bytes_budget: IDL.Opt(IDL.Nat64),
  llm_summary_on_overflow: IDL.Opt(IDL.Bool),
  llm_summary_model: IDL.Opt(IDL.Text),
  llm_summary_max_chars: IDL.Opt(IDL.Nat64),
  llm_summary_request_bytes_threshold: IDL.Opt(IDL.Nat64),
  memory_min_score: IDL.Opt(IDL.Float64),
  enable_lightweight_skill_actions: IDL.Opt(IDL.Bool),
});
const canisterConfig = IDL.Record({
  provider: IDL.Opt(providerConfig),
  context: IDL.Opt(contextConfig),
  allowed_principals: IDL.Opt(IDL.Vec(IDL.Principal)),
});
const chatRequest = IDL.Record({
  prompt: IDL.Text,
  session_id: IDL.Opt(IDL.Text),
  model: IDL.Opt(IDL.Text),
  temperature: IDL.Opt(IDL.Float64),
});
const chatResponse = IDL.Record({
  response: IDL.Text,
  session_id: IDL.Opt(IDL.Text),
  model: IDL.Opt(IDL.Text),
  provider_ready: IDL.Bool,
  memory_ready: IDL.Bool,
});
const healthResponse = IDL.Record({
  status: IDL.Text,
  version: IDL.Text,
  runtime: IDL.Text,
  provider_ready: IDL.Bool,
  memory_ready: IDL.Bool,
});
const conversationSummaryGetRequest = IDL.Record({
  session_id: IDL.Text,
});
const agentObserveRequest = IDL.Record({
  session_id: IDL.Opt(IDL.Text),
});
const agentObservation = IDL.Record({
  workspace_keys: IDL.Vec(IDL.Text),
  core_keys: IDL.Vec(IDL.Text),
  conversation_summary_key: IDL.Opt(IDL.Text),
  conversation_summary_present: IDL.Bool,
  conversation_turn_count: IDL.Nat64,
  auto_promoted_keys: IDL.Vec(IDL.Text),
  history_limit: IDL.Nat64,
  enable_auto_promote: IDL.Bool,
  enable_conversation_summary: IDL.Bool,
  tool_loop_enabled: IDL.Bool,
  max_tool_iterations: IDL.Nat64,
});
const memoryCategory = IDL.Variant({
  core: IDL.Null,
  daily: IDL.Null,
  conversation: IDL.Null,
  custom: IDL.Text,
});
const memoryItem = IDL.Record({
  id: IDL.Text,
  key: IDL.Text,
  content: IDL.Text,
  category: memoryCategory,
  timestamp: IDL.Text,
  session_id: IDL.Opt(IDL.Text),
  score: IDL.Opt(IDL.Float64),
});
const memoryStoreRequest = IDL.Record({
  key: IDL.Text,
  content: IDL.Text,
  category: memoryCategory,
  session_id: IDL.Opt(IDL.Text),
});
const memoryRecallRequest = IDL.Record({
  query: IDL.Text,
  limit: IDL.Nat64,
  session_id: IDL.Opt(IDL.Text),
});
const memoryGetRequest = IDL.Record({ key: IDL.Text });
const memoryListRequest = IDL.Record({
  category: IDL.Opt(memoryCategory),
  session_id: IDL.Opt(IDL.Text),
});
const memoryForgetRequest = IDL.Record({ key: IDL.Text });

const result = (ok: IDL.Type) => IDL.Variant({ Ok: ok, Err: apiError });

export const idlFactory: IDL.InterfaceFactory = ({ IDL }) =>
  IDL.Service({
    agent_observe: IDL.Func([agentObserveRequest], [result(agentObservation)], ['query']),
    chat: IDL.Func([chatRequest], [result(chatResponse)], []),
    conversation_summary_get: IDL.Func([conversationSummaryGetRequest], [result(IDL.Opt(memoryItem))], ['query']),
    health: IDL.Func([], [healthResponse], ['query']),
    memory_count: IDL.Func([], [result(IDL.Nat64)], ['query']),
    memory_forget: IDL.Func([memoryForgetRequest], [result(IDL.Bool)], []),
    memory_get: IDL.Func([memoryGetRequest], [result(IDL.Opt(memoryItem))], ['query']),
    memory_list: IDL.Func([memoryListRequest], [result(IDL.Vec(memoryItem))], ['query']),
    memory_recall: IDL.Func([memoryRecallRequest], [result(IDL.Vec(memoryItem))], ['query']),
    memory_store: IDL.Func([memoryStoreRequest], [result(IDL.Null)], []),
  });

export const candid = {
  apiError,
  canisterConfig,
  contextConfig,
  chatRequest,
  chatResponse,
  conversationSummaryGetRequest,
  healthResponse,
  agentObserveRequest,
  agentObservation,
  memoryForgetRequest,
  memoryGetRequest,
  memoryItem,
  memoryListRequest,
  memoryRecallRequest,
  memoryStoreRequest,
};
