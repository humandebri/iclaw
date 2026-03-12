import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, describe, expect, it } from "vitest";
import { WebhooksPage } from "@/pages/Webhooks";
import type { Agent, Webhook } from "@/generated/iclaw.did";
import type { AsyncActionState, WebhooksViewModel } from "@/types/ui";

const idleAction: AsyncActionState = {
  pending: false,
  error: null,
  success: null,
};

const rotateSuccessAction: AsyncActionState = {
  pending: false,
  error: null,
  success: "rotated daily-brief secret=brand-new-secret",
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

const selectedWebhook: Webhook = {
  id: "daily-brief",
  name: "Daily Brief",
  agent_id: "default",
  session_mode: "reuse_fixed",
  fixed_session_id: ["daily-session"],
  secret: "secr…tail",
  enabled: true,
  created_at: "2026-03-11T00:00:00Z",
  updated_at: "2026-03-11T00:00:00Z",
  last_run_id: ["run-1"],
  last_secret_rotated_at: ["2026-03-11T00:30:00Z"],
  last_invoked_at: ["2026-03-11T00:45:00Z"],
  last_rejection_at: ["2026-03-11T01:00:00Z"],
  last_rejection_reason: ["webhook secret is invalid"],
};

const webhooks: WebhooksViewModel = {
  items: [selectedWebhook],
  selectedWebhook,
  latestRuns: {
    "daily-brief": {
      id: "run-1",
      agent_id: "default",
      session_id: "daily-session",
      status: "failed",
      prompt: "brief me",
      response: [],
      model: [],
      provider_ready: false,
      memory_ready: true,
      created_at: "2026-03-11T00:00:00Z",
      started_at: [],
      finished_at: [],
      error: ["provider missing"],
      trigger_kind: "webhook",
      trigger_id: ["daily-brief"],
      pending_tool_calls: [],
      pending_assistant_text: [],
    },
  },
  rejections: {
    "daily-brief": [
      {
        id: "daily-brief:1",
        webhook_id: "daily-brief",
        reason: "webhook secret is invalid",
        timestamp: "2026-03-11T01:00:00Z",
      },
    ],
  },
};

describe("WebhooksPage", () => {
  let container: HTMLDivElement;
  let root: Root;

  afterEach(() => {
    if (root) {
      act(() => root.unmount());
    }
    container?.remove();
  });

  it("shows selected webhook detail and latest run", () => {
    container = document.createElement("div");
    document.body.appendChild(container);
    root = createRoot(container);

    act(() => {
      root.render(
        <WebhooksPage
          webhooks={webhooks}
          agents={agents}
          action={idleAction}
          onCreate={async () => undefined}
          onSelectWebhook={() => undefined}
          onUpdate={async () => undefined}
          onToggleEnabled={async () => undefined}
          onRotateSecret={async () => undefined}
        />,
      );
    });

    expect(container.textContent).toContain("Daily Brief");
    expect(container.textContent).toContain("secr…tail");
    expect(container.textContent).toContain("run-1");
    expect(container.textContent).toContain("reuse_fixed");
    expect(container.textContent).toContain("webhook secret is invalid");
    expect(container.textContent).toContain("Recent Rejections");
    expect(container.textContent).toContain("Last Invoked");
    expect(container.textContent).toContain("Last Secret Rotation");
    expect(container.textContent).toContain("Rotate secret");
  });

  it("shows the empty state and create form when no webhook is selected", () => {
    container = document.createElement("div");
    document.body.appendChild(container);
    root = createRoot(container);

    act(() => {
      root.render(
        <WebhooksPage
          webhooks={{ items: [], selectedWebhook: null, latestRuns: {}, rejections: {} }}
          agents={agents}
          action={idleAction}
          onCreate={async () => undefined}
          onSelectWebhook={() => undefined}
          onUpdate={async () => undefined}
          onToggleEnabled={async () => undefined}
          onRotateSecret={async () => undefined}
        />,
      );
    });

    expect(container.textContent).toContain("webhook はまだありません。");
    expect(container.textContent).toContain("Create webhook");
    expect(container.textContent).toContain("webhook を選ぶと詳細が見えます。");
  });

  it("shows the one-time secret notice after rotate", () => {
    container = document.createElement("div");
    document.body.appendChild(container);
    root = createRoot(container);

    act(() => {
      root.render(
        <WebhooksPage
          webhooks={webhooks}
          agents={agents}
          action={rotateSuccessAction}
          onCreate={async () => undefined}
          onSelectWebhook={() => undefined}
          onUpdate={async () => undefined}
          onToggleEnabled={async () => undefined}
          onRotateSecret={async () => undefined}
        />,
      );
    });

    expect(container.textContent).toContain("rotated daily-brief");
    expect(container.textContent).toContain("brand-new-secret");
    expect(container.textContent).toContain("この secret は今しか表示されません。");
    expect(container.textContent).toContain("旧 secret が即無効になります。");
  });
});
