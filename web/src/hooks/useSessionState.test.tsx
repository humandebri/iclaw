import { act, useEffect } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { useSessionState } from "@/hooks/useSessionState";

const { fetchSessions } = vi.hoisted(() => ({
  fetchSessions: vi.fn(),
}));

vi.mock("@/lib/api", () => ({ fetchSessions }));

describe("useSessionState", () => {
  let container: HTMLDivElement;
  let root: Root;

  beforeEach(() => {
    window.localStorage.clear();
    container = document.createElement("div");
    document.body.appendChild(container);
    root = createRoot(container);
    fetchSessions.mockReset();
  });

  afterEach(() => {
    act(() => root.unmount());
    container.remove();
  });

  it("uses canister sessions as the primary selector source", async () => {
    window.localStorage.setItem(
      "iclaw.sessions",
      JSON.stringify([
        {
          id: "browser-only",
          label: "browser only",
          lastUsedAt: "2026-03-10T00:00:00.000Z",
        },
        {
          id: "session-a",
          label: "remembered label",
          lastUsedAt: "2026-03-11T00:00:00.000Z",
        },
      ]),
    );
    fetchSessions.mockResolvedValue([
      {
        id: "session-a",
        agent_id: "default",
        title: "canister title",
        created_at: "2026-03-11T00:00:00.000Z",
        updated_at: "2026-03-11T01:00:00.000Z",
        last_run_id: [],
      },
    ]);

    const observed: Array<{ sessionId: string; sessions: Array<{ id: string; label: string }> }> = [];

    function Harness() {
      const state = useSessionState();
      observed.push({
        sessionId: state.sessionId,
        sessions: state.sessions.map((session) => ({ id: session.id, label: session.label })),
      });
      return null;
    }

    await act(async () => {
      root.render(<Harness />);
      await Promise.resolve();
      await Promise.resolve();
    });

    const latest = observed.at(-1);
    expect(latest?.sessionId).toBe("browser-only");
    expect(latest?.sessions).toEqual([{ id: "session-a", label: "canister title" }]);
  });

  it("preserves a user-entered session id until the canister knows about it", async () => {
    fetchSessions.mockResolvedValue([
      {
        id: "session-a",
        agent_id: "default",
        title: "canister title",
        created_at: "2026-03-11T00:00:00.000Z",
        updated_at: "2026-03-11T01:00:00.000Z",
        last_run_id: [],
      },
    ]);

    const observed: Array<{ sessionId: string; sessions: Array<{ id: string; label: string }> }> = [];

    function Harness() {
      const state = useSessionState();
      observed.push({
        sessionId: state.sessionId,
        sessions: state.sessions.map((session) => ({ id: session.id, label: session.label })),
      });

      useEffect(() => {
        if (state.sessionId !== "draft-session") {
          state.setSessionId("draft-session");
        }
      }, [state.sessionId, state.setSessionId]);

      return null;
    }

    await act(async () => {
      root.render(<Harness />);
      await Promise.resolve();
      await Promise.resolve();
      await Promise.resolve();
    });

    const latest = observed.at(-1);
    expect(latest?.sessionId).toBe("draft-session");
    expect(latest?.sessions).toEqual([{ id: "session-a", label: "canister title" }]);
  });
});
