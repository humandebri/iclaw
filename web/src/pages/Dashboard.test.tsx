import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { MemoryRouter } from "react-router-dom";
import { afterEach, describe, expect, it } from "vitest";
import { Dashboard } from "@/pages/Dashboard";
import type { Agent, HealthResponse, Run, ToolPolicy } from "@/generated/iclaw.did";
import type { ObserveViewModel, ScheduleAlertItem } from "@/types/ui";

const health: HealthResponse = {
  status: "ok",
  version: "0.1.0",
  runtime: "icp-canister",
  provider_ready: true,
  memory_ready: true,
};

const currentAgent: Agent = {
  id: "default",
  name: "Default Agent",
  description: "default agent",
  enabled_tool_names: ["memory_store"],
  requires_tool_approval: false,
  system_prompt_override: [],
  status: "active",
};

const observe: ObserveViewModel = {
  observation: null,
  summary: null,
};

const toolPolicies: ToolPolicy[] = [];

const scheduleAlerts: ScheduleAlertItem[] = [
  {
    id: "hourly-brief",
    name: "Hourly Brief",
    enabled: true,
    running: false,
    stale: true,
    consecutiveFailureCount: 3n,
    lastSuccessAt: null,
    nextRunAt: "2026-03-12T01:00:00Z",
  },
  {
    id: "running-hourly",
    name: "Running Hourly",
    enabled: true,
    running: true,
    stale: false,
    consecutiveFailureCount: 1n,
    lastSuccessAt: "2026-03-12T00:15:00Z",
    nextRunAt: "2026-03-12T01:15:00Z",
  },
];

const blockedRun: Run = {
  id: "run-blocked",
  agent_id: "default",
  session_id: "session-1",
  status: "blocked",
  prompt: "store this",
  response: [],
  model: [],
  provider_ready: true,
  memory_ready: true,
  created_at: "2026-03-12T00:00:00Z",
  started_at: [],
  finished_at: [],
  error: ["tool requires approval"],
  trigger_kind: "manual",
  trigger_id: [],
  pending_tool_calls: [
    {
      id: "call-1",
      name: "memory_store",
      arguments: "{\"key\":\"note/1\"}",
    },
  ],
  pending_assistant_text: ["let me store that"],
};

const secondBlockedRun: Run = {
  ...blockedRun,
  id: "run-blocked-2",
  created_at: "2026-03-12T00:10:00Z",
  pending_tool_calls: [
    {
      id: "call-2",
      name: "memory_store",
      arguments: "{\"key\":\"note/2\"}",
    },
  ],
};

describe("Dashboard", () => {
  let container: HTMLDivElement;
  let root: Root;

  afterEach(() => {
    if (root) {
      act(() => root.unmount());
    }
    container?.remove();
  });

  it("shows the refreshed dashboard hierarchy and keeps action links intact", () => {
    container = document.createElement("div");
    document.body.appendChild(container);
    root = createRoot(container);

    act(() => {
      root.render(
        <MemoryRouter>
          <Dashboard
            health={health}
            observe={observe}
            currentAgent={currentAgent}
            latestRun={null}
            latestScheduleRun={null}
            latestScheduleFailure={null}
            failingScheduleCount={2}
            staleScheduleCount={1}
            runningScheduleCount={1}
            blockedRunCount={2}
            blockedRuns={[secondBlockedRun, blockedRun]}
            scheduleAlerts={scheduleAlerts}
            latestWebhookRun={null}
            latestWebhookFailure={null}
            scheduleCount={2}
            webhookCount={0}
            runCount={0}
            toolPolicies={toolPolicies}
            allowlist={{
              principals: [],
              currentPrincipal: "aaaaa-aa",
              pending: false,
              error: null,
              success: null,
            }}
            loading={false}
            error={null}
            onRefresh={async () => undefined}
            onAllowlistRefresh={async () => undefined}
            onAllowlistSave={async () => undefined}
          />
        </MemoryRouter>,
      );
    });

    expect(container.textContent).toContain("Operator Summary");
    expect(container.textContent).toContain("Runtime");
    expect(container.textContent).toContain("Selected Agent");
    expect(container.textContent).toContain("Stale Schedules");
    expect(container.textContent).toContain("Blocked Runs");
    expect(container.textContent).toContain("next run-blocked-2");
    expect(container.textContent).toContain("Summary Strip");
    expect(container.textContent).toContain("Schedule Alerts");
    expect(container.textContent).toContain("Quick Actions");
    expect(container.textContent).toContain("run-blocked");
    expect(container.textContent).toContain("run-blocked-2");
    expect(container.textContent).toContain("pending approval 1 tool");
    expect(container.textContent).toContain("Hourly Brief");
    expect(container.textContent).toContain("failures 3");
    expect(container.textContent).toContain("stale");
    expect(container.textContent).toContain("last success まだ成功なし");
    expect(container.textContent).toContain("Running Hourly");
    expect(container.textContent).toContain("Reload from canister");
    expect(container.textContent).toContain("Save allowlist");
    const links = Array.from(container.querySelectorAll("a")).map((link) => link.getAttribute("href"));
    expect(links).toContain("/runs?runId=run-blocked");
    expect(links).toContain("/runs?runId=run-blocked-2");
    expect(links).toContain("/schedules");
  });
});
