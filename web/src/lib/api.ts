// where: iclaw/web/src/lib/api.ts
// what: Typed actor wrapper for the iclaw caller console
// why: Centralize actor creation, II integration, and result normalization in one place

import { Actor, HttpAgent } from "@dfinity/agent";
import { AuthClient } from "@dfinity/auth-client";
import { Principal } from "@dfinity/principal";
import type {
  _SERVICE,
  AgentObservation,
  AgentObserveRequest,
  ChatRequest,
  ChatResponse,
  ConversationSummaryGetRequest,
  HealthResponse,
  MemoryCategory,
  MemoryGetRequest,
  MemoryItem,
  MemoryListRequest,
  MemoryRecallRequest,
  MemoryStoreRequest,
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

export async function fetchObserve(sessionId?: string): Promise<AgentObservation> {
  const request: AgentObserveRequest = { session_id: sessionId ? [sessionId] : [] };
  return unwrapResult(await (await createActor()).agent_observe(request));
}

export async function fetchSummary(sessionId: string): Promise<MemoryItem | null> {
  const request: ConversationSummaryGetRequest = { session_id: sessionId };
  const result = unwrapResult(await (await createActor()).conversation_summary_get(request));
  return result[0] ?? null;
}

export async function sendChat(request: {
  prompt: string;
  sessionId?: string;
  model?: string;
  temperature?: number;
}): Promise<ChatResponse> {
  const payload: ChatRequest = {
    prompt: request.prompt,
    session_id: request.sessionId ? [request.sessionId] : [],
    model: request.model ? [request.model] : [],
    temperature: typeof request.temperature === "number" ? [request.temperature] : [],
  };
  return unwrapResult(await (await createActor()).chat(payload));
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
