import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, describe, expect, it, vi } from "vitest";
import { RunsPage } from "@/pages/Runs";
import type { Run, RunEvent } from "@/generated/iclaw.did";
import type { RunsViewModel } from "@/types/ui";

const blockedRun: Run = {
  id: "run-blocked",
  agent_id: "default",
  session_id: "session-1",
  status: "blocked",
  prompt: "store this",
  response: [],
  model: ["gpt-4.1-mini"],
  provider_ready: true,
  memory_ready: true,
  created_at: "2026-03-12T00:00:00Z",
  started_at: ["2026-03-12T00:00:01Z"],
  finished_at: ["2026-03-12T00:00:02Z"],
  error: ["tool 'memory_store' requires approval"],
  trigger_kind: "manual",
  trigger_id: [],
  pending_tool_calls: [
    {
      id: "call-1",
      name: "memory_store",
      arguments: '{"key":"note/1","content":"blocked"}',
    },
  ],
  pending_assistant_text: ["let me store that"],
};

const approvalEvents: RunEvent[] = [
  {
    id: "event-1",
    run_id: "run-blocked",
    kind: "approved",
    message: "operator approved memory_store",
    timestamp: "2026-03-12T00:00:03Z",
  },
  {
    id: "event-2",
    run_id: "run-blocked",
    kind: "resumed",
    message: "run resumed after approval",
    timestamp: "2026-03-12T00:00:04Z",
  },
  {
    id: "event-3",
    run_id: "run-blocked",
    kind: "tool_succeeded",
    message: "memory_store saved note/1",
    timestamp: "2026-03-12T00:00:05Z",
  },
];

describe("RunsPage", () => {
  let container: HTMLDivElement;
  let root: Root;

  afterEach(() => {
    if (root) {
      act(() => root.unmount());
    }
    container?.remove();
  });

  it("shows pending tool details and resume action for blocked runs", () => {
    const onResumeRun = vi.fn(async () => undefined);
    const runs: RunsViewModel = {
      items: [blockedRun],
      selectedRun: blockedRun,
      events: approvalEvents,
    };

    container = document.createElement("div");
    document.body.appendChild(container);
    root = createRoot(container);

    act(() => {
      root.render(
        <RunsPage
          sessionId="session-1"
          runs={runs}
          onRefresh={async () => undefined}
          onSelectRun={async () => undefined}
          onCancelRun={async () => undefined}
          onResumeRun={onResumeRun}
        />,
      );
    });

    expect(container.textContent).toContain("Pending Tool Calls");
    expect(container.textContent).toContain("memory_store");
    expect(container.textContent).toContain("let me store that");
    expect(container.textContent).toContain("Resume run");
    expect(container.textContent).toContain("Approval Flow");
    expect(container.textContent).toContain("operator approved memory_store");
    expect(container.textContent).toContain("run resumed after approval");
    expect(container.textContent).toContain("memory_store saved note/1");
  });
});
