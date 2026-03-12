// where: iclaw/web/src/lib/api.ts
// what: Typed actor wrapper for the iclaw caller console
// why: Centralize actor creation, II integration, and result normalization in one place

import { Actor, HttpAgent } from "@dfinity/agent";
import { AuthClient } from "@dfinity/auth-client";
import { Principal } from "@dfinity/principal";
import type {
  _SERVICE,
  Agent,
  AgentDraft,
  AgentCreateRequest,
  AgentGetRequest,
  AgentObservation,
  AgentObserveRequest,
  AgentUpdateRequest,
  AllowedPrincipalsResponse,
  ConversationSummaryGetRequest,
  HealthResponse,
  MemoryCategory,
  MemoryGetRequest,
  MemoryItem,
  MemoryListRequest,
  MemoryRecallRequest,
  MemoryStoreRequest,
  Run,
  RunCancelRequest,
  RunCreateRequest,
  RunEvent,
  RunGetRequest,
  RunListRequest,
  RunResumeRequest,
  Schedule,
  ScheduleCreateRequest,
  ScheduleDraft,
  ScheduleGetRequest,
  ScheduleUpdateRequest,
  Session,
  SessionGetRequest,
  ToolPolicy,
  ToolPolicyListRequest,
  ToolPolicyUpdateRequest,
  Webhook,
  WebhookCreateRequest,
  WebhookDraft,
  WebhookGetRequest,
  WebhookInvokeRequest,
  WebhookRejection,
  WebhookRejectionsListRequest,
  WebhookSecretRotateRequest,
  WebhookSecretRotateResponse,
  WebhookUpdateRequest,
} from "@/generated/iclaw.did";
import { idlFactory } from "@/generated/iclaw.did.js";
import { isLocalReplica, resolveCanisterId, resolveHost, resolveIdentityProvider } from "@/lib/env";
import type { UiError } from "@/types/ui";

let authClientPromise: Promise<AuthClient> | null = null;

function normalizeError(error: unknown): UiError {
  if (typeof error === "object" && error && "code" in error && "message" in error) {
    const value = error as { code?: unknown; message?: unknown };
    return {
      code: typeof value.code === "string" ? value.code : "internal",
      message: typeof value.message === "string" ? value.message : "Unknown error",
    };
  }

  if (error instanceof Error) {
    return { code: "internal", message: error.message };
  }

  return { code: "internal", message: "Unknown error" };
}

function unwrapResult<T>(result: { Ok: T } | { Err: { code: string; message: string } }): T {
  if ("Err" in result) {
    throw result.Err;
  }
  return result.Ok;
}

export async function getAuthClient(): Promise<AuthClient> {
  authClientPromise ??= AuthClient.create();
  return authClientPromise;
}

async function createActor(): Promise<_SERVICE> {
  const authClient = await getAuthClient();
  const identity = await authClient.getIdentity();
  const agent = new HttpAgent({
    identity,
    host: resolveHost(),
  });

  if (isLocalReplica()) {
    await agent.fetchRootKey();
  }

  return Actor.createActor<_SERVICE>(idlFactory, {
    agent,
    canisterId: resolveCanisterId(),
  });
}

export async function login(): Promise<void> {
  const authClient = await getAuthClient();
  await new Promise<void>((resolve, reject) => {
    authClient.login({
      identityProvider: resolveIdentityProvider(),
      onSuccess: () => resolve(),
      onError: (error) => reject(error),
    });
  });
}

export async function logout(): Promise<void> {
  const authClient = await getAuthClient();
  await authClient.logout();
}

export async function isAuthenticated(): Promise<boolean> {
  return (await getAuthClient()).isAuthenticated();
}

export async function currentPrincipalText(): Promise<string> {
  const identity = (await getAuthClient()).getIdentity();
  return identity.getPrincipal().toText();
}

export async function ensureOperatorAccess(sessionId?: string): Promise<void> {
  await fetchObserve(sessionId);
}

export async function fetchHealth(): Promise<HealthResponse> {
  return (await createActor()).health();
}

export async function fetchAllowedPrincipals(): Promise<string[]> {
  const response: AllowedPrincipalsResponse = unwrapResult(await (await createActor()).allowed_principals_get());
  return response.allowed_principals.map((principal) => principal.toText());
}

export async function updateAllowedPrincipals(principalTexts: string[]): Promise<string[]> {
  const allowed_principals = principalTexts.map((principalText) => Principal.fromText(principalText.trim()));
  const response: AllowedPrincipalsResponse = unwrapResult(
    await (await createActor()).allowed_principals_set({ allowed_principals }),
  );
  return response.allowed_principals.map((principal) => principal.toText());
}

export async function fetchObserve(sessionId?: string): Promise<AgentObservation> {
  const request: AgentObserveRequest = { session_id: sessionId ? [sessionId] : [] };
  return unwrapResult(await (await createActor()).agent_observe(request));
}

export async function fetchSummary(sessionId: string): Promise<MemoryItem | null> {
  const request: ConversationSummaryGetRequest = { session_id: sessionId };
  const result = unwrapResult(await (await createActor()).conversation_summary_get(request));
  return result[0] ?? null;
}

export async function fetchAgents(): Promise<Agent[]> {
  return unwrapResult(await (await createActor()).agents_list());
}

export async function fetchAgent(agentId: string): Promise<Agent | null> {
  const payload: AgentGetRequest = { agent_id: agentId };
  const result = unwrapResult(await (await createActor()).agent_get(payload));
  return result[0] ?? null;
}

export async function createAgent(request: {
  id: string;
  name: string;
  description: string;
  enabledToolNames: string[];
  requiresToolApproval: boolean;
  systemPromptOverride?: string;
  status: string;
}): Promise<Agent> {
  const draft: AgentDraft = {
    id: request.id,
    name: request.name,
    description: request.description,
    enabled_tool_names: request.enabledToolNames,
    requires_tool_approval: request.requiresToolApproval,
    system_prompt_override: request.systemPromptOverride ? [request.systemPromptOverride] : [],
    status: request.status,
  };
  const payload: AgentCreateRequest = { draft };
  return unwrapResult(await (await createActor()).agent_create(payload));
}

export async function updateAgent(request: {
  id: string;
  name: string;
  description: string;
  enabledToolNames: string[];
  requiresToolApproval: boolean;
  systemPromptOverride?: string;
  status: string;
}): Promise<Agent> {
  const agent: Agent = {
    id: request.id,
    name: request.name,
    description: request.description,
    enabled_tool_names: request.enabledToolNames,
    requires_tool_approval: request.requiresToolApproval,
    system_prompt_override: request.systemPromptOverride ? [request.systemPromptOverride] : [],
    status: request.status,
  };
  const payload: AgentUpdateRequest = { agent };
  return unwrapResult(await (await createActor()).agent_update(payload));
}

export async function fetchToolPolicies(agentId?: string): Promise<ToolPolicy[]> {
  const payload: ToolPolicyListRequest = { agent_id: agentId ? [agentId] : [] };
  return unwrapResult(await (await createActor()).tool_policy_list(payload));
}

export async function updateToolPolicy(request: {
  agentId: string;
  toolName: string;
  enabled: boolean;
  requiresApproval: boolean;
}): Promise<ToolPolicy> {
  const policy: ToolPolicy = {
    agent_id: request.agentId,
    tool_name: request.toolName,
    enabled: request.enabled,
    requires_approval: request.requiresApproval,
  };
  const payload: ToolPolicyUpdateRequest = { policy };
  return unwrapResult(await (await createActor()).tool_policy_update(payload));
}

export async function fetchSessions(agentId?: string): Promise<Session[]> {
  return unwrapResult(await (await createActor()).sessions_list(agentId ? [agentId] : []));
}

export async function fetchSession(sessionId: string): Promise<Session | null> {
  const payload: SessionGetRequest = { session_id: sessionId };
  const result = unwrapResult(await (await createActor()).session_get(payload));
  return result[0] ?? null;
}

export async function fetchSchedules(): Promise<Schedule[]> {
  return unwrapResult(await (await createActor()).schedules_list());
}

export async function fetchSchedule(scheduleId: string): Promise<Schedule | null> {
  const payload: ScheduleGetRequest = { schedule_id: scheduleId };
  const result = unwrapResult(await (await createActor()).schedule_get(payload));
  return result[0] ?? null;
}

export async function createSchedule(request: {
  id: string;
  name: string;
  agentId: string;
  prompt: string;
  intervalMinutes: bigint;
  sessionMode: string;
  fixedSessionId?: string;
  enabled: boolean;
}): Promise<Schedule> {
  const draft: ScheduleDraft = {
    id: request.id,
    name: request.name,
    agent_id: request.agentId,
    prompt: request.prompt,
    interval_minutes: request.intervalMinutes,
    session_mode: request.sessionMode,
    fixed_session_id: request.fixedSessionId ? [request.fixedSessionId] : [],
    enabled: request.enabled,
  };
  const payload: ScheduleCreateRequest = { draft };
  return unwrapResult(await (await createActor()).schedule_create(payload));
}

export async function updateSchedule(schedule: Schedule): Promise<Schedule> {
  const payload: ScheduleUpdateRequest = { schedule };
  return unwrapResult(await (await createActor()).schedule_update(payload));
}

export async function deleteSchedule(scheduleId: string): Promise<boolean> {
  const payload: ScheduleGetRequest = { schedule_id: scheduleId };
  return unwrapResult(await (await createActor()).schedule_delete(payload));
}

export async function triggerSchedule(scheduleId: string): Promise<Run> {
  const payload: ScheduleGetRequest = { schedule_id: scheduleId };
  return unwrapResult(await (await createActor()).schedule_trigger(payload));
}

export async function fetchWebhooks(): Promise<Webhook[]> {
  return unwrapResult(await (await createActor()).webhooks_list());
}

export async function fetchWebhook(webhookId: string): Promise<Webhook | null> {
  const payload: WebhookGetRequest = { webhook_id: webhookId };
  const result = unwrapResult(await (await createActor()).webhook_get(payload));
  return result[0] ?? null;
}

export async function fetchWebhookRejections(webhookId: string, limit = 5n): Promise<WebhookRejection[]> {
  const payload: WebhookRejectionsListRequest = {
    webhook_id: webhookId,
    limit: [limit],
  };
  return unwrapResult(await (await createActor()).webhook_rejections_list(payload));
}

export async function createWebhook(request: {
  id: string;
  name: string;
  agentId: string;
  sessionMode: string;
  fixedSessionId?: string;
  secret: string;
  enabled: boolean;
}): Promise<Webhook> {
  const draft: WebhookDraft = {
    id: request.id,
    name: request.name,
    agent_id: request.agentId,
    session_mode: request.sessionMode,
    fixed_session_id: request.fixedSessionId ? [request.fixedSessionId] : [],
    secret: request.secret,
    enabled: request.enabled,
  };
  const payload: WebhookCreateRequest = { draft };
  return unwrapResult(await (await createActor()).webhook_create(payload));
}

export async function updateWebhook(webhook: Webhook, secretOverride?: string): Promise<Webhook> {
  const payload: WebhookUpdateRequest = {
    webhook,
    secret_override: secretOverride ? [secretOverride] : [],
  };
  return unwrapResult(await (await createActor()).webhook_update(payload));
}

export async function deleteWebhook(webhookId: string): Promise<boolean> {
  const payload: WebhookGetRequest = { webhook_id: webhookId };
  return unwrapResult(await (await createActor()).webhook_delete(payload));
}

export async function invokeWebhook(request: {
  webhookId: string;
  secret: string;
  prompt: string;
  sessionId?: string;
  model?: string;
  temperature?: number;
}): Promise<Run> {
  const payload: WebhookInvokeRequest = {
    webhook_id: request.webhookId,
    secret: request.secret,
    prompt: request.prompt,
    session_id: request.sessionId ? [request.sessionId] : [],
    model: request.model ? [request.model] : [],
    temperature: typeof request.temperature === "number" ? [request.temperature] : [],
  };
  return unwrapResult(await (await createActor()).webhook_invoke(payload));
}

export async function rotateWebhookSecret(webhookId: string): Promise<WebhookSecretRotateResponse> {
  const payload: WebhookSecretRotateRequest = { webhook_id: webhookId };
  return unwrapResult(await (await createActor()).webhook_rotate_secret(payload));
}

export async function createRun(request: {
  agentId?: string;
  sessionId?: string;
  prompt: string;
  model?: string;
  temperature?: number;
}): Promise<Run> {
  const payload: RunCreateRequest = {
    agent_id: request.agentId ? [request.agentId] : [],
    session_id: request.sessionId ? [request.sessionId] : [],
    prompt: request.prompt,
    model: request.model ? [request.model] : [],
    temperature: typeof request.temperature === "number" ? [request.temperature] : [],
  };
  return unwrapResult(await (await createActor()).run_create(payload));
}

export async function fetchRun(runId: string): Promise<Run | null> {
  const payload: RunGetRequest = { run_id: runId };
  const result = unwrapResult(await (await createActor()).run_get(payload));
  return result[0] ?? null;
}

export async function fetchRuns(sessionId?: string, limit?: bigint): Promise<Run[]> {
  const payload: RunListRequest = {
    session_id: sessionId ? [sessionId] : [],
    limit: typeof limit === "bigint" ? [limit] : [],
  };
  return unwrapResult(await (await createActor()).run_list(payload));
}

export async function fetchRunEvents(runId: string): Promise<RunEvent[]> {
  const payload: { run_id: string } = { run_id: runId };
  return unwrapResult(await (await createActor()).run_events_get(payload));
}

export async function cancelRun(runId: string): Promise<boolean> {
  const payload: RunCancelRequest = { run_id: runId };
  return unwrapResult(await (await createActor()).run_cancel(payload));
}

export async function resumeRun(runId: string): Promise<Run> {
  const payload: RunResumeRequest = { run_id: runId };
  return unwrapResult(await (await createActor()).run_resume(payload));
}

export async function storeMemory(args: {
  key: string;
  content: string;
  category: MemoryCategory;
  sessionId?: string;
}): Promise<void> {
  const payload: MemoryStoreRequest = {
    key: args.key,
    content: args.content,
    category: args.category,
    session_id: args.sessionId ? [args.sessionId] : [],
  };
  unwrapResult(await (await createActor()).memory_store(payload));
}

export async function fetchMemoryGet(key: string): Promise<MemoryItem | null> {
  const request: MemoryGetRequest = { key };
  const result = unwrapResult(await (await createActor()).memory_get(request));
  return result[0] ?? null;
}

export async function fetchMemoryList(category?: MemoryCategory, sessionId?: string): Promise<MemoryItem[]> {
  const request: MemoryListRequest = {
    category: category ? [category] : [],
    session_id: sessionId ? [sessionId] : [],
  };
  return unwrapResult(await (await createActor()).memory_list(request));
}

export async function fetchMemoryRecall(query: string, limit: bigint, sessionId?: string): Promise<MemoryItem[]> {
  const request: MemoryRecallRequest = {
    query,
    limit,
    session_id: sessionId ? [sessionId] : [],
  };
  return unwrapResult(await (await createActor()).memory_recall(request));
}

export async function forgetMemory(key: string): Promise<boolean> {
  return unwrapResult(await (await createActor()).memory_forget({ key }));
}

export async function fetchMemoryCount(): Promise<bigint> {
  return unwrapResult(await (await createActor()).memory_count());
}

export function anonymousPrincipalText(): string {
  return Principal.anonymous().toText();
}

export { normalizeError };
