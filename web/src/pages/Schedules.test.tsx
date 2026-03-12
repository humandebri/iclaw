import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, describe, expect, it } from "vitest";
import { SchedulesPage } from "@/pages/Schedules";
import type { Agent, Schedule } from "@/generated/iclaw.did";
import type { AsyncActionState, SchedulesViewModel } from "@/types/ui";

const idleAction: AsyncActionState = {
  pending: false,
  error: null,
  success: null,
};

const agents: Agent[] = [
  {
    id: "default",
    name: "Default Agent",
    description: "default agent",
    enabled_tool_names: ["memory_store"],
    requires_tool_approval: false,
    system_prompt_override: [],
    status: "active",
  },
];

const selectedSchedule: Schedule = {
  id: "hourly-brief",
  name: "Hourly Brief",
  agent_id: "default",
  prompt: "brief me",
  interval_minutes: 60n,
  session_mode: "reuse_fixed",
  fixed_session_id: ["brief-session"],
  enabled: true,
  created_at: "2026-03-12T00:00:00Z",
  updated_at: "2026-03-12T00:00:00Z",
  next_run_at: ["2026-03-12T01:00:00Z"],
  last_started_at: [],
  last_finished_at: ["2026-03-12T00:30:00Z"],
  last_run_id: ["run-1"],
  last_error: ["provider missing"],
  consecutive_failure_count: 2n,
  last_success_at: [],
  running: false,
};

const healthySchedule: Schedule = {
  ...selectedSchedule,
  id: "healthy-hourly",
  name: "Healthy Hourly",
  next_run_at: ["2026-03-13T01:00:00Z"],
  last_error: [],
  consecutive_failure_count: 0n,
  last_success_at: ["2026-03-12T00:10:00Z"],
};

const schedules: SchedulesViewModel = {
  items: [selectedSchedule, healthySchedule],
  selectedSchedule,
  latestRuns: {
    "hourly-brief": {
      id: "run-1",
      agent_id: "default",
      session_id: "brief-session",
      status: "failed",
      prompt: "brief me",
      response: [],
      model: [],
      provider_ready: false,
      memory_ready: true,
      created_at: "2026-03-12T00:30:00Z",
      started_at: [],
      finished_at: ["2026-03-12T00:30:05Z"],
      error: ["provider missing"],
      trigger_kind: "schedule",
      trigger_id: ["hourly-brief"],
      pending_tool_calls: [],
      pending_assistant_text: [],
    },
    "healthy-hourly": null,
  },
};

describe("SchedulesPage", () => {
  let container: HTMLDivElement;
  let root: Root;

  function renderSchedulesPage(viewModel: SchedulesViewModel) {
    container = document.createElement("div");
    document.body.appendChild(container);
    root = createRoot(container);

    act(() => {
      root.render(
        <SchedulesPage
          schedules={viewModel}
          agents={agents}
          action={idleAction}
          onCreate={async () => undefined}
          onSelectSchedule={() => undefined}
          onUpdate={async () => undefined}
          onToggleEnabled={async () => undefined}
          onTrigger={async () => undefined}
        />,
      );
    });
  }

  function clickFilter(label: string) {
    const filterButton = Array.from(container.querySelectorAll("button")).find(
      (button) => button.textContent === label,
    );
    expect(filterButton).toBeTruthy();

    act(() => {
      filterButton?.dispatchEvent(new MouseEvent("click", { bubbles: true }));
    });
  }

  afterEach(() => {
    if (root) {
      act(() => root.unmount());
    }
    container?.remove();
  });

  it("shows selected schedule detail and latest run", () => {
    renderSchedulesPage(schedules);

    expect(container.textContent).toContain("Hourly Brief");
    expect(container.textContent).toContain("run-1");
    expect(container.textContent).toContain("next 2026-03-12T01:00:00Z");
    expect(container.textContent).toContain("failures 2");
    expect(container.textContent).toContain("last success まだ成功なし");
    expect(container.textContent).toContain("last finished 2026-03-12T00:30:00Z");
    expect(container.textContent).toContain("provider missing");
    expect(container.textContent).toContain("disabled は timer の自動実行だけを止めます。");
    expect(container.textContent).toContain("Trigger now");
  });

  it("filters list to failing schedules only", () => {
    renderSchedulesPage(schedules);

    expect(container.textContent).toContain("Healthy Hourly");
    clickFilter("failing");

    expect(container.textContent).toContain("Hourly Brief");
    expect(container.textContent).not.toContain("Healthy Hourly");
  });

  it("hides detail actions when the selected schedule is filtered out", () => {
    renderSchedulesPage({
      ...schedules,
      selectedSchedule: healthySchedule,
    });

    expect(container.textContent).toContain("Healthy Hourly");
    expect(container.textContent).toContain("Trigger now");

    clickFilter("failing");

    expect(container.textContent).toContain("Hourly Brief");
    expect(container.textContent).not.toContain("Healthy Hourly");
    expect(container.textContent).toContain("schedule を選ぶと詳細が見えます。");
    expect(container.textContent).not.toContain("Trigger now");
    expect(container.textContent).not.toContain("Disable schedule");
    expect(container.textContent).not.toContain("Save detail changes");
  });

  it("keeps detail visible when the selected schedule remains in the failing list", () => {
    renderSchedulesPage(schedules);

    clickFilter("failing");

    expect(container.textContent).toContain("Hourly Brief");
    expect(container.textContent).toContain("Trigger now");
    expect(container.textContent).toContain("Disable schedule");
    expect(container.textContent).toContain("Save detail changes");
    expect(container.textContent).not.toContain("schedule を選ぶと詳細が見えます。");
  });
});
