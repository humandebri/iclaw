import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

const {
  createRun,
  fetchRunEvents,
  fetchRuns,
  fetchHealth,
  fetchObserve,
  fetchSummary,
  fetchAllowedPrincipals,
  sessionState,
} = vi.hoisted(() => ({
  createRun: vi.fn(),
  fetchRunEvents: vi.fn(),
  fetchRuns: vi.fn(),
  fetchHealth: vi.fn(),
  fetchObserve: vi.fn(),
  fetchSummary: vi.fn(),
  fetchAllowedPrincipals: vi.fn(),
  sessionState: {
    sessions: [{ id: "alpha", label: "alpha", lastUsedAt: "2026-03-11T00:00:00.000Z" }],
    sessionId: "alpha",
    setSessionId: vi.fn(),
  },
}));

vi.mock("@/hooks/useSessionState", () => ({
  useSessionState: () => sessionState,
}));

vi.mock("@/lib/api", () => ({
  cancelRun: vi.fn(),
  createRun,
  currentPrincipalText: vi.fn(async () => "aaaaa-aa"),
  ensureOperatorAccess: vi.fn(async () => undefined),
  fetchAllowedPrincipals,
  fetchHealth,
  fetchRunEvents,
  fetchRuns,
  fetchMemoryCount: vi.fn(async () => 0n),
  fetchMemoryGet: vi.fn(async () => null),
  fetchMemoryList: vi.fn(async () => []),
  fetchMemoryRecall: vi.fn(async () => []),
  fetchObserve,
  fetchSummary,
  forgetMemory: vi.fn(async () => false),
  isAuthenticated: vi.fn(async () => true),
  login: vi.fn(),
  logout: vi.fn(async () => undefined),
  normalizeError: (error: unknown) =>
    typeof error === "object" && error && "code" in error && "message" in error
      ? (error as { code: string; message: string })
      : { code: "internal", message: "unknown" },
  storeMemory: vi.fn(async () => undefined),
  updateAllowedPrincipals: vi.fn(async () => []),
}));

describe("App run_create failure handling", () => {
  let container: HTMLDivElement;
  let root: Root;

  beforeEach(() => {
    container = document.createElement("div");
    document.body.appendChild(container);
    root = createRoot(container);
    window.location.hash = "#/chat";
    createRun.mockReset();
    sessionState.sessions = [{ id: "alpha", label: "alpha", lastUsedAt: "2026-03-11T00:00:00.000Z" }];
    sessionState.sessionId = "alpha";
    sessionState.setSessionId = vi.fn();
    fetchRunEvents.mockResolvedValue([]);
    fetchRuns.mockResolvedValue([
      {
        id: "run-failed",
        agent_id: "default",
        session_id: "alpha",
        status: "failed",
        prompt: "hello",
        response: [],
        model: [],
        provider_ready: false,
        memory_ready: true,
        created_at: "2026-03-11T00:00:00.000Z",
        started_at: [],
        finished_at: ["2026-03-11T00:00:01.000Z"],
        error: ["provider is not configured"],
      },
    ]);
    fetchHealth.mockResolvedValue({
      status: "ok",
      runtime: "icp-canister",
      version: "0.1.0",
      provider_ready: true,
      memory_ready: true,
    });
    fetchObserve.mockResolvedValue({
      workspace_keys: [],
      core_keys: [],
      conversation_summary_key: [],
      conversation_summary_present: false,
      conversation_turn_count: 0n,
      auto_promoted_keys: [],
      history_limit: 8n,
      enable_auto_promote: false,
      enable_conversation_summary: true,
      tool_loop_enabled: true,
      max_tool_iterations: 3n,
    });
    fetchSummary.mockResolvedValue(null);
    fetchAllowedPrincipals.mockResolvedValue(["aaaaa-aa"]);
  });

  afterEach(() => {
    act(() => root.unmount());
    container.remove();
  });

  it("treats failed runs as normal responses and renders the failed run in chat history", async () => {
    createRun.mockResolvedValue({
      id: "run-failed",
      agent_id: "default",
      session_id: "alpha",
      status: "failed",
      prompt: "hello",
      response: [],
      model: [],
      provider_ready: false,
      memory_ready: true,
      created_at: "2026-03-11T00:00:00.000Z",
      started_at: [],
      finished_at: ["2026-03-11T00:00:01.000Z"],
      error: ["provider is not configured"],
    });

    const { default: App } = await import("@/App");

    await act(async () => {
      root.render(<App />);
      await Promise.resolve();
      await Promise.resolve();
      await Promise.resolve();
    });

    const prompt = container.querySelector("[data-tid='chat-prompt-input']");
    const send = container.querySelector("[data-tid='chat-send-button']");
    expect(prompt).not.toBeNull();
    expect(send).not.toBeNull();

    await act(async () => {
      if (!(prompt instanceof HTMLTextAreaElement) || !(send instanceof HTMLButtonElement)) {
        throw new Error("chat controls not found");
      }
      const setValue = Object.getOwnPropertyDescriptor(HTMLTextAreaElement.prototype, "value")?.set;
      setValue?.call(prompt, "hello");
      prompt.dispatchEvent(new Event("input", { bubbles: true }));
      prompt.dispatchEvent(new Event("change", { bubbles: true }));
      send.click();
      await Promise.resolve();
      await Promise.resolve();
      await Promise.resolve();
    });

    expect(createRun).toHaveBeenCalledTimes(1);
    expect(container.textContent).toContain("error: provider is not configured");
  });

  it("refreshes observe, summary, and runs with the new session id after auto-creating a session", async () => {
    sessionState.sessions = [];
    sessionState.sessionId = "";

    createRun.mockResolvedValue({
      id: "run-beta",
      agent_id: "default",
      session_id: "beta",
      status: "completed",
      prompt: "boot a session",
      response: ["created session response"],
      model: ["gpt-4o-mini"],
      provider_ready: true,
      memory_ready: true,
      created_at: "2026-03-11T00:00:00.000Z",
      started_at: ["2026-03-11T00:00:00.100Z"],
      finished_at: ["2026-03-11T00:00:01.000Z"],
      error: [],
    });
    fetchRuns.mockImplementation(async (sessionId?: string) =>
      sessionId === "beta"
        ? [
            {
              id: "run-beta",
              agent_id: "default",
              session_id: "beta",
              status: "completed",
              prompt: "boot a session",
              response: ["created session response"],
              model: ["gpt-4o-mini"],
              provider_ready: true,
              memory_ready: true,
              created_at: "2026-03-11T00:00:00.000Z",
              started_at: ["2026-03-11T00:00:00.100Z"],
              finished_at: ["2026-03-11T00:00:01.000Z"],
              error: [],
            },
          ]
        : [],
    );
    fetchSummary.mockImplementation(async (sessionId?: string) =>
      sessionId === "beta"
        ? {
            id: "summary-beta",
            key: "conversation/beta/summary",
            content: "beta summary",
            category: { core: null },
            timestamp: "2026-03-11T00:00:02.000Z",
            session_id: ["beta"],
          }
        : null,
    );

    const { default: App } = await import("@/App");

    await act(async () => {
      root.render(<App />);
      await Promise.resolve();
      await Promise.resolve();
      await Promise.resolve();
    });

    fetchObserve.mockClear();
    fetchSummary.mockClear();
    fetchRuns.mockClear();

    const prompt = container.querySelector("[data-tid='chat-prompt-input']");
    const send = container.querySelector("[data-tid='chat-send-button']");
    expect(prompt).not.toBeNull();
    expect(send).not.toBeNull();

    await act(async () => {
      if (!(prompt instanceof HTMLTextAreaElement) || !(send instanceof HTMLButtonElement)) {
        throw new Error("chat controls not found");
      }
      const setValue = Object.getOwnPropertyDescriptor(HTMLTextAreaElement.prototype, "value")?.set;
      setValue?.call(prompt, "boot a session");
      prompt.dispatchEvent(new Event("input", { bubbles: true }));
      prompt.dispatchEvent(new Event("change", { bubbles: true }));
      send.click();
      await Promise.resolve();
      await Promise.resolve();
      await Promise.resolve();
    });

    expect(sessionState.setSessionId).toHaveBeenCalledWith("beta");
    expect(fetchObserve).toHaveBeenCalledWith("beta");
    expect(fetchSummary).toHaveBeenCalledWith("beta");
    expect(fetchRuns).toHaveBeenCalledWith("beta", 50n);
    expect(container.textContent).toContain("created session response");
  });
});
