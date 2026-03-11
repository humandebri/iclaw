import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("@/hooks/useSessionState", () => ({
  useSessionState: () => ({ sessions: [], sessionId: "alpha", setSessionId: vi.fn() }),
}));

vi.mock("@/lib/api", () => ({
  currentPrincipalText: vi.fn(async () => "aaaaa-aa"),
  ensureOperatorAccess: vi.fn(async () => {
    throw { code: "unauthorized", message: "blocked by allowlist" };
  }),
  fetchAllowedPrincipals: vi.fn(async () => ["2vxsx-fae"]),
  fetchHealth: vi.fn(async () => ({ status: "ok", runtime: "icp-canister", version: "0.1.0", provider_ready: true, memory_ready: true })),
  fetchMemoryCount: vi.fn(),
  fetchMemoryGet: vi.fn(),
  fetchMemoryList: vi.fn(),
  fetchMemoryRecall: vi.fn(),
  fetchObserve: vi.fn(),
  fetchSummary: vi.fn(),
  forgetMemory: vi.fn(),
  isAuthenticated: vi.fn(async () => true),
  login: vi.fn(),
  logout: vi.fn(async () => undefined),
  normalizeError: (error: unknown) => (typeof error === "object" && error && "code" in error && "message" in error ? error as { code: string; message: string } : { code: "internal", message: "unknown" }),
  sendChat: vi.fn(),
  storeMemory: vi.fn(),
  updateAllowedPrincipals: vi.fn(),
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
