import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("@/hooks/useSessionState", () => ({
  useSessionState: () => ({ sessions: [], sessionId: "alpha", setSessionId: vi.fn() }),
}));

vi.mock("@/lib/api", () => ({
  cancelRun: vi.fn(),
  createAgent: vi.fn(),
  createSchedule: vi.fn(),
  createRun: vi.fn(),
  createWebhook: vi.fn(),
  currentPrincipalText: vi.fn(async () => "aaaaa-aa"),
  deleteSchedule: vi.fn(),
  ensureOperatorAccess: vi.fn(async () => {
    throw { code: "unauthorized", message: "blocked by allowlist" };
  }),
  fetchAgent: vi.fn(),
  fetchAgents: vi.fn(async () => []),
  fetchAllowedPrincipals: vi.fn(async () => ["2vxsx-fae"]),
  fetchHealth: vi.fn(async () => ({ status: "ok", runtime: "icp-canister", version: "0.1.0", provider_ready: true, memory_ready: true })),
  fetchWebhookRejections: vi.fn(async () => []),
  fetchWebhooks: vi.fn(async () => []),
  fetchRunEvents: vi.fn(async () => []),
  fetchRuns: vi.fn(async () => []),
  fetchSchedule: vi.fn(),
  fetchSchedules: vi.fn(async () => []),
  fetchSession: vi.fn(async () => null),
  fetchMemoryCount: vi.fn(),
  fetchMemoryGet: vi.fn(),
  fetchMemoryList: vi.fn(),
  fetchMemoryRecall: vi.fn(),
  fetchObserve: vi.fn(),
  fetchSummary: vi.fn(),
  fetchToolPolicies: vi.fn(async () => []),
  forgetMemory: vi.fn(),
  isAuthenticated: vi.fn(async () => true),
  login: vi.fn(),
  logout: vi.fn(async () => undefined),
  normalizeError: (error: unknown) => (typeof error === "object" && error && "code" in error && "message" in error ? error as { code: string; message: string } : { code: "internal", message: "unknown" }),
  resumeRun: vi.fn(),
  rotateWebhookSecret: vi.fn(),
  storeMemory: vi.fn(),
  triggerSchedule: vi.fn(),
  updateAgent: vi.fn(),
  updateSchedule: vi.fn(),
  updateAllowedPrincipals: vi.fn(),
  updateToolPolicy: vi.fn(),
  updateWebhook: vi.fn(),
}));

describe("App access control", () => {
  let container: HTMLDivElement;
  let root: Root;

  beforeEach(() => {
    container = document.createElement("div");
    document.body.appendChild(container);
    root = createRoot(container);
    window.location.hash = "#/";
  });

  afterEach(() => {
    if (root) {
      act(() => root.unmount());
    }
    container?.remove();
  });

  it("renders access denied screen for unauthorized principals", async () => {
    const { default: App } = await import("@/App");

    await act(async () => {
      root.render(<App />);
      await Promise.resolve();
      await Promise.resolve();
    });

    expect(container.textContent).toContain("Access denied");
    expect(container.textContent).toContain("aaaaa-aa");
    expect(container.textContent).toContain("blocked by allowlist");
  });
});
