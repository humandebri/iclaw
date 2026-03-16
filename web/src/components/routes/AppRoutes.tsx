// where: iclaw/web/src/components/routes/AppRoutes.tsx
// what: Authorized route tree for the operator console
// why: Keep App.tsx focused on state orchestration while route composition stays readable and testable

import { HashRouter, Navigate, Route, Routes } from "react-router-dom";
import { Layout } from "@/components/layout/Layout";
import { Dashboard } from "@/pages/Dashboard";
import { ChatPage } from "@/pages/Chat";
import { MemoryPage } from "@/pages/Memory";
import { ObservePage } from "@/pages/Observe";
import { RunsPage } from "@/pages/Runs";
import { AgentsPage } from "@/pages/Agents";
import { SchedulesPage } from "@/pages/Schedules";
import { WebhooksPage } from "@/pages/Webhooks";
import type { Agent, HealthResponse, MemoryCategory, MemoryItem, Run, Schedule, ToolPolicy, Webhook } from "@/generated/iclaw.did";
import type {
  AsyncActionState,
  AgentsViewModel,
  ChatMessage,
  ManifestPreviewEntry,
  MemoryUiState,
  ObserveViewModel,
  RunsViewModel,
  ScheduleAlertItem,
  SchedulesViewModel,
  WebhooksViewModel,
} from "@/types/ui";

export function AppRoutes({
  sessionId,
  principal,
  onLogout,
  dashboard,
  chat,
  memory,
  observe,
  runs,
  agents,
  schedules,
  webhooks,
}: {
  sessionId: string;
  principal: string;
  onLogout: () => Promise<void>;
  dashboard: {
    health: HealthResponse | null;
    observe: ObserveViewModel;
    currentAgent: Agent | null;
    latestRun: Run | null;
    latestScheduleRun: Run | null;
    latestScheduleFailure: Run | null;
    failingScheduleCount: number;
    staleScheduleCount: number;
    runningScheduleCount: number;
    blockedRunCount: number;
    blockedRuns: Run[];
    scheduleAlerts: ScheduleAlertItem[];
    latestWebhookRun: Run | null;
    latestWebhookFailure: Run | null;
    scheduleCount: number;
    webhookCount: number;
    runCount: number;
    toolPolicies: ToolPolicy[];
    allowlist: {
      principals: string[];
      currentPrincipal: string;
      pending: boolean;
      error: string | null;
      success: string | null;
    };
    loading: boolean;
    error: string | null;
    onRefresh: () => Promise<void>;
    onAllowlistRefresh: () => Promise<void>;
    onAllowlistSave: (principals: string[]) => Promise<void>;
  };
  chat: {
    messages: ChatMessage[];
    pending: boolean;
    agentId: string;
    agentLocked: boolean;
    agents: Agent[];
    sessions: { id: string; label: string; lastUsedAt: string }[];
    setAgentId: (nextAgentId: string) => void;
    setSessionId: (nextSessionId: string) => void;
    onSend: (prompt: string) => Promise<void>;
    observe: ObserveViewModel;
  };
  memory: {
    queryResult: MemoryItem[];
    getResult: MemoryItem | null;
    count: bigint | null;
    manifestPreview: ManifestPreviewEntry[];
    manifestNote: string;
    memoryUi: MemoryUiState;
    onSaveCore: (key: string, content: string) => Promise<void>;
    onAdvancedStore: (key: string, content: string, category: MemoryCategory, sessionId: string) => Promise<void>;
    onForget: (key: string) => Promise<void>;
    onLookup: (key: string) => Promise<void>;
    onList: (category?: MemoryCategory, sessionId?: string) => Promise<void>;
    onRecall: (query: string, limit: bigint, sessionId?: string) => Promise<void>;
    onCount: () => Promise<void>;
    onManifestTextChange: (raw: string) => void;
    onManifestFiles: (files: FileList | null) => void;
    onRunManifest: () => Promise<void>;
  };
  observe: {
    filter: string;
    setFilter: (value: string) => void;
    viewModel: ObserveViewModel;
    onRefresh: () => Promise<void>;
  };
  runs: {
    viewModel: RunsViewModel;
    onRefresh: () => Promise<void>;
    onSelectRun: (runId: string) => Promise<void>;
    onCancelRun: (runId: string) => Promise<void>;
    onResumeRun: (runId: string) => Promise<void>;
  };
  agents: {
    viewModel: AgentsViewModel;
    currentAgentId: string;
    onSelectAgent: (agentId: string) => void;
  };
  schedules: {
    viewModel: SchedulesViewModel;
    agents: Agent[];
    action: AsyncActionState;
    onCreate: (draft: {
      id: string;
      name: string;
      agentId: string;
      prompt: string;
      intervalMinutes: bigint;
      sessionMode: string;
      fixedSessionId?: string;
      enabled: boolean;
    }) => Promise<void>;
    onSelectSchedule: (scheduleId: string) => void;
    onUpdate: (schedule: Schedule) => Promise<void>;
    onToggleEnabled: (schedule: Schedule) => Promise<void>;
    onTrigger: (scheduleId: string) => Promise<void>;
  };
  webhooks: {
    viewModel: WebhooksViewModel;
    agents: Agent[];
    action: AsyncActionState;
    onCreate: (draft: {
      id: string;
      name: string;
      agentId: string;
      sessionMode: string;
      fixedSessionId?: string;
      secret: string;
      enabled: boolean;
    }) => Promise<void>;
    onSelectWebhook: (webhookId: string) => void;
    onUpdate: (webhook: Webhook, secretOverride?: string) => Promise<void>;
    onToggleEnabled: (webhook: Webhook) => Promise<void>;
    onRotateSecret: (webhookId: string) => Promise<void>;
  };
}) {
  return (
    <HashRouter>
      <Routes>
        <Route element={<Layout sessionId={sessionId} onLogout={onLogout} principal={principal} />}>
          <Route path="/" element={<Dashboard {...dashboard} />} />
          <Route path="/chat" element={<ChatPage messages={chat.messages} pending={chat.pending} agentId={chat.agentId} agentLocked={chat.agentLocked} agents={chat.agents} sessionId={sessionId} sessions={chat.sessions} setAgentId={chat.setAgentId} setSessionId={chat.setSessionId} onSend={chat.onSend} observe={chat.observe} />} />
          <Route path="/runs" element={<RunsPage sessionId={sessionId} runs={runs.viewModel} onRefresh={runs.onRefresh} onSelectRun={runs.onSelectRun} onCancelRun={runs.onCancelRun} onResumeRun={runs.onResumeRun} />} />
          <Route path="/agents" element={<AgentsPage agents={agents.viewModel} currentAgentId={agents.currentAgentId} onSelectAgent={agents.onSelectAgent} />} />
          <Route path="/schedules" element={<SchedulesPage schedules={schedules.viewModel} agents={schedules.agents} action={schedules.action} onCreate={schedules.onCreate} onSelectSchedule={schedules.onSelectSchedule} onUpdate={schedules.onUpdate} onToggleEnabled={schedules.onToggleEnabled} onTrigger={schedules.onTrigger} />} />
          <Route path="/webhooks" element={<WebhooksPage webhooks={webhooks.viewModel} agents={webhooks.agents} action={webhooks.action} onCreate={webhooks.onCreate} onSelectWebhook={webhooks.onSelectWebhook} onUpdate={webhooks.onUpdate} onToggleEnabled={webhooks.onToggleEnabled} onRotateSecret={webhooks.onRotateSecret} />} />
          <Route path="/memory" element={<MemoryPage {...memory} />} />
          <Route path="/observe" element={<ObservePage sessionId={sessionId} filter={observe.filter} setFilter={observe.setFilter} observe={observe.viewModel} onRefresh={observe.onRefresh} />} />
          <Route path="*" element={<Navigate to="/" replace />} />
        </Route>
      </Routes>
    </HashRouter>
  );
}
