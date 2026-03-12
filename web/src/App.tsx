// where: iclaw/web/src/App.tsx
// what: Main app composition for the single-canister caller console
// why: Keep auth, access control, session state, and per-screen async status in one orchestration layer

import { useEffect, useRef, useState } from "react";
import { AccessDeniedScreen, UnauthenticatedScreen } from "@/components/access/AuthScreens";
import { AppRoutes } from "@/components/routes/AppRoutes";
import {
  cancelRun,
  createSchedule,
  createRun,
  createWebhook,
  currentPrincipalText,
  ensureOperatorAccess,
  fetchAgents,
  fetchAllowedPrincipals,
  fetchHealth,
  fetchObserve,
  fetchRun,
  fetchSchedules,
  fetchSession,
  fetchRunEvents,
  fetchRuns,
  fetchSummary,
  fetchToolPolicies,
  triggerSchedule,
  fetchWebhooks,
  fetchWebhookRejections,
  isAuthenticated,
  login,
  logout,
  normalizeError,
  resumeRun,
  rotateWebhookSecret,
  updateSchedule,
  updateAllowedPrincipals,
  updateWebhook,
} from "@/lib/api";
import { useMemoryController } from "@/hooks/useMemoryController";
import { useSessionState } from "@/hooks/useSessionState";
import { isFailingSchedule, isStaleSchedule } from "@/lib/schedule-health";
import type { Agent, HealthResponse, Run, Schedule, Session, ToolPolicy, Webhook, WebhookRejection } from "@/generated/iclaw.did";
import type {
  AccessState,
  AgentsViewModel,
  AsyncActionState,
  ChatMessage,
  ObserveViewModel,
  RunsViewModel,
  ScheduleAlertItem,
  SchedulesViewModel,
  WebhooksViewModel,
} from "@/types/ui";

const IDLE_ACTION: AsyncActionState = { pending: false, error: null, success: null };
const EMPTY_RUNS: RunsViewModel = { items: [], selectedRun: null, events: [] };

export default function App() {
  const { sessions, sessionId, setSessionId } = useSessionState();
  const [authenticated, setAuthenticated] = useState(false);
  const [authReady, setAuthReady] = useState(false);
  const [authError, setAuthError] = useState<string | null>(null);
  const [access, setAccess] = useState<AccessState>({ status: "checking", principal: "", message: null });
  const [pageError, setPageError] = useState<string | null>(null);
  const [health, setHealth] = useState<HealthResponse | null>(null);
  const [agents, setAgents] = useState<Agent[]>([]);
  const [draftAgentId, setDraftAgentId] = useState("default");
  const [sessionContext, setSessionContext] = useState<Session | null>(null);
  const [toolPolicies, setToolPolicies] = useState<ToolPolicy[]>([]);
  const [schedules, setSchedules] = useState<Schedule[]>([]);
  const [selectedScheduleId, setSelectedScheduleId] = useState("");
  const [latestScheduleRuns, setLatestScheduleRuns] = useState<Record<string, Run | null>>({});
  const [webhooks, setWebhooks] = useState<Webhook[]>([]);
  const [selectedWebhookId, setSelectedWebhookId] = useState("");
  const [latestWebhookRuns, setLatestWebhookRuns] = useState<Record<string, Run | null>>({});
  const [webhookRejections, setWebhookRejections] = useState<Record<string, WebhookRejection[]>>({});
  const [allowedPrincipals, setAllowedPrincipals] = useState<string[]>([]);
  const [allowlistAction, setAllowlistAction] = useState<AsyncActionState>(IDLE_ACTION);
  const [scheduleAction, setScheduleAction] = useState<AsyncActionState>(IDLE_ACTION);
  const [webhookAction, setWebhookAction] = useState<AsyncActionState>(IDLE_ACTION);
  const [observe, setObserve] = useState<ObserveViewModel>({ observation: null, summary: null });
  const [observeFilter, setObserveFilter] = useState("");
  const [sending, setSending] = useState(false);
  const [runs, setRuns] = useState<RunsViewModel>(EMPTY_RUNS);
  const dashboardRequestRef = useRef(0);

  const activeObserveSession = observeFilter || sessionId;
  const agentId = sessionContext?.agent_id ?? draftAgentId;
  const selectedAgent = agents.find((agent) => agent.id === agentId) ?? null;
  const agentsViewModel: AgentsViewModel = {
    items: agents,
    selectedAgent,
    toolPolicies,
  };
  const selectedSchedule = schedules.find((schedule) => schedule.id === selectedScheduleId) ?? schedules[0] ?? null;
  const schedulesViewModel: SchedulesViewModel = {
    items: schedules,
    selectedSchedule,
    latestRuns: latestScheduleRuns,
  };
  const selectedWebhook = webhooks.find((webhook) => webhook.id === selectedWebhookId) ?? webhooks[0] ?? null;
  const webhooksViewModel: WebhooksViewModel = {
    items: webhooks,
    selectedWebhook,
    latestRuns: latestWebhookRuns,
    rejections: webhookRejections,
  };
  const latestWebhookRun =
    webhooks
      .map((webhook) => latestWebhookRuns[webhook.id] ?? null)
      .filter((run): run is Run => run !== null)
      .sort((left, right) => right.created_at.localeCompare(left.created_at))[0] ?? null;
  const latestWebhookFailure =
    webhooks
      .map((webhook) => latestWebhookRuns[webhook.id] ?? null)
      .filter((run): run is Run => run !== null && run.status !== "completed")
      .sort((left, right) => right.created_at.localeCompare(left.created_at))[0] ?? null;
  const latestScheduleRun =
    schedules
      .map((schedule) => latestScheduleRuns[schedule.id] ?? null)
      .filter((run): run is Run => run !== null)
      .sort((left, right) => right.created_at.localeCompare(left.created_at))[0] ?? null;
  const latestScheduleFailure =
    schedules
      .map((schedule) => latestScheduleRuns[schedule.id] ?? null)
      .filter((run): run is Run => run !== null && run.status !== "completed")
      .sort((left, right) => right.created_at.localeCompare(left.created_at))[0] ?? null;
  const latestBlockedRun =
    runs.items
      .filter((run) => run.status === "blocked")
      .sort((left, right) => right.created_at.localeCompare(left.created_at))[0] ?? null;
  const failingSchedules = schedules.filter(isFailingSchedule);
  const runningSchedules = schedules.filter((schedule) => schedule.running);
  const staleSchedules = schedules.filter(isStaleSchedule);
  const scheduleAlerts: ScheduleAlertItem[] = schedules
    .filter((schedule) => isFailingSchedule(schedule) || isStaleSchedule(schedule))
    .slice()
    .sort((left, right) => {
      const leftStale = isStaleSchedule(left);
      const rightStale = isStaleSchedule(right);
      if (leftStale !== rightStale) {
        return rightStale ? 1 : -1;
      }
      if (left.consecutive_failure_count !== right.consecutive_failure_count) {
        return right.consecutive_failure_count > left.consecutive_failure_count ? 1 : -1;
      }
      return (right.last_finished_at[0] ?? "").localeCompare(left.last_finished_at[0] ?? "");
    })
    .slice(0, 5)
    .map((schedule) => ({
      id: schedule.id,
      name: schedule.name,
      enabled: schedule.enabled,
      running: schedule.running,
      stale: isStaleSchedule(schedule),
      consecutiveFailureCount: schedule.consecutive_failure_count,
      lastSuccessAt: schedule.last_success_at[0] ?? null,
      nextRunAt: schedule.next_run_at[0] ?? null,
    }));

  const mapRunsToMessages = (items: RunsViewModel["items"]): ChatMessage[] =>
    items
      .slice()
      .reverse()
      .flatMap((run) => {
        const next: ChatMessage[] = [
          { id: `${run.id}:user`, role: "user", content: run.prompt, timestamp: run.created_at },
        ];
        if (run.response[0]) {
          next.push({
            id: `${run.id}:assistant`,
            role: "assistant",
            content: run.response[0],
            timestamp: run.finished_at[0] ?? run.created_at,
          });
        } else if (run.error[0]) {
          next.push({
            id: `${run.id}:assistant`,
            role: "assistant",
            content: `error: ${run.error[0]}`,
            timestamp: run.finished_at[0] ?? run.created_at,
          });
        }
        return next;
      });

  const denyAccess = async (message: string) => {
    setAccess({
      status: "denied",
      principal: await currentPrincipalText().catch(() => ""),
      message,
    });
  };

  const handleProtectedError = async (error: unknown) => {
    const normalized = normalizeError(error);
    if (normalized.code === "unauthorized") {
      await denyAccess(normalized.message);
      return;
    }
    setPageError(normalized.message);
  };

  const refreshRuns = async (targetSessionId = sessionId, preferredRunId?: string) => {
    if (!targetSessionId) {
      setRuns(EMPTY_RUNS);
      return;
    }
    const nextRuns = await fetchRuns(targetSessionId, 50n);
    const selectedRun =
      nextRuns.find((run) => run.id === preferredRunId) ??
      nextRuns[0] ??
      null;
    const events = selectedRun ? await fetchRunEvents(selectedRun.id) : [];
    setRuns({ items: nextRuns, selectedRun, events });
  };

  const refreshDashboard = async (targetSessionId = sessionId, targetAgentId = draftAgentId) => {
    const requestId = dashboardRequestRef.current + 1;
    dashboardRequestRef.current = requestId;
    try {
      const [nextHealth, nextObserve, nextSummary, nextAllowedPrincipals, nextAgents, nextSession, nextSchedules, nextWebhooks] = await Promise.all([
        fetchHealth(),
        fetchObserve(targetSessionId || undefined),
        targetSessionId ? fetchSummary(targetSessionId) : Promise.resolve(null),
        fetchAllowedPrincipals(),
        fetchAgents(),
        targetSessionId ? fetchSession(targetSessionId) : Promise.resolve(null),
        fetchSchedules(),
        fetchWebhooks(),
      ]);
      if (dashboardRequestRef.current !== requestId) {
        return;
      }
      const resolvedAgentId =
        nextSession?.agent_id ??
        nextAgents.find((agent) => agent.id === targetAgentId)?.id ??
        nextAgents[0]?.id ??
        "default";
      const nextPolicies = await fetchToolPolicies(resolvedAgentId);
      const nextLatestScheduleRunsEntries = await Promise.all(
        nextSchedules.map(async (schedule) => [
          schedule.id,
          schedule.last_run_id[0] ? await fetchRun(schedule.last_run_id[0]) : null,
        ] as const),
      );
      const nextLatestWebhookRunsEntries = await Promise.all(
        nextWebhooks.map(async (webhook) => [
          webhook.id,
          webhook.last_run_id[0] ? await fetchRun(webhook.last_run_id[0]) : null,
        ] as const),
      );
      const nextWebhookRejectionsEntries = await Promise.all(
        nextWebhooks.map(async (webhook) => [webhook.id, await fetchWebhookRejections(webhook.id)] as const),
      );
      if (dashboardRequestRef.current !== requestId) {
        return;
      }
      setHealth(nextHealth);
      setAgents(nextAgents);
      setSessionContext(nextSession);
      if (!nextSession) {
        setDraftAgentId(resolvedAgentId);
      }
      setToolPolicies(nextPolicies);
      setSchedules(nextSchedules);
      setLatestScheduleRuns(Object.fromEntries(nextLatestScheduleRunsEntries));
      setSelectedScheduleId((prev) => prev || nextSchedules[0]?.id || "");
      setWebhooks(nextWebhooks);
      setLatestWebhookRuns(Object.fromEntries(nextLatestWebhookRunsEntries));
      setWebhookRejections(Object.fromEntries(nextWebhookRejectionsEntries));
      setSelectedWebhookId((prev) => prev || nextWebhooks[0]?.id || "");
      setAllowedPrincipals(nextAllowedPrincipals);
      setObserve({ observation: nextObserve, summary: nextSummary });
      await refreshRuns(targetSessionId);
      setPageError(null);
      setAccess((prev) => ({ ...prev, status: "allowed", message: null }));
    } catch (error) {
      await handleProtectedError(error);
    }
  };

  const refreshAllowlist = async () => {
    try {
      setAllowedPrincipals(await fetchAllowedPrincipals());
      setAllowlistAction((prev) => ({ ...prev, error: null }));
    } catch (error) {
      const normalized = normalizeError(error);
      setAllowlistAction({ pending: false, error: normalized.message, success: null });
      if (normalized.code === "unauthorized") {
        await denyAccess(normalized.message);
      }
    }
  };

  const refreshObserve = async () => {
    try {
      const [nextObserve, nextSummary] = await Promise.all([
        fetchObserve(activeObserveSession || undefined),
        activeObserveSession ? fetchSummary(activeObserveSession) : Promise.resolve(null),
      ]);
      setObserve({ observation: nextObserve, summary: nextSummary });
      setPageError(null);
    } catch (error) {
      await handleProtectedError(error);
    }
  };

  const memoryController = useMemoryController({
    refreshObserve,
    onProtectedError: handleProtectedError,
  });

  useEffect(() => {
    void (async () => {
      try {
        const nextAuthenticated = await isAuthenticated();
        setAuthenticated(nextAuthenticated);
      } catch (error) {
        setAuthError(normalizeError(error).message);
      } finally {
        setAuthReady(true);
      }
    })();
  }, []);

  useEffect(() => {
    if (!authenticated) {
      return;
    }
    void (async () => {
      const principal = await currentPrincipalText().catch(() => "");
      setAccess({ status: "checking", principal, message: null });
      try {
        await ensureOperatorAccess(sessionId || undefined);
        await refreshDashboard();
      } catch (error) {
        await handleProtectedError(error);
      }
    })();
  }, [authenticated, sessionId]);

  useEffect(() => {
    if (!authenticated || access.status !== "allowed") {
      return;
    }
    void refreshObserve();
  }, [authenticated, access.status, activeObserveSession]);

  useEffect(() => {
    if (!authenticated || access.status !== "allowed") {
      return;
    }
    void refreshRuns(sessionId);
  }, [authenticated, access.status, sessionId]);

  useEffect(() => {
    if (!authenticated || access.status !== "allowed") {
      return;
    }
    void (async () => {
      try {
        setToolPolicies(await fetchToolPolicies(agentId));
      } catch (error) {
        await handleProtectedError(error);
      }
    })();
  }, [authenticated, access.status, agentId]);

  const handleLogin = async () => {
    setAuthError(null);
    try {
      await login();
      setAuthenticated(true);
    } catch (error) {
      setAuthError(normalizeError(error).message);
    }
  };

  const handleLogout = async () => {
    await logout();
    setAuthenticated(false);
    setAccess({ status: "checking", principal: "", message: null });
    setRuns(EMPTY_RUNS);
  };

  const handleSend = async (prompt: string) => {
    setSending(true);
    setPageError(null);
    try {
      const run = await createRun({ prompt, agentId, sessionId: sessionId || undefined, temperature: 0 });
      if (run.session_id !== sessionId) {
        setSessionId(run.session_id);
      }
      await refreshRuns(run.session_id, run.id);
      await refreshDashboard(run.session_id, run.agent_id);
      if (run.status !== "completed" && run.error[0]) {
        setPageError(run.error[0]);
      }
    } catch (error) {
      await handleProtectedError(error);
    } finally {
      setSending(false);
    }
  };

  const handleRunSelect = async (runId: string) => {
    const selectedRun = runs.items.find((run) => run.id === runId) ?? null;
    setRuns((prev) => ({ ...prev, selectedRun, events: prev.events }));
    if (!selectedRun) {
      setRuns((prev) => ({ ...prev, events: [] }));
      return;
    }
    try {
      const events = await fetchRunEvents(selectedRun.id);
      setRuns((prev) => ({ ...prev, selectedRun, events }));
    } catch (error) {
      await handleProtectedError(error);
    }
  };

  const handleRunCancel = async (runId: string) => {
    try {
      await cancelRun(runId);
      await refreshRuns(sessionId, runId);
    } catch (error) {
      await handleProtectedError(error);
    }
  };

  const handleRunResume = async (runId: string) => {
    try {
      const run = await resumeRun(runId);
      await refreshRuns(run.session_id, run.id);
      await refreshDashboard(run.session_id, run.agent_id);
      if (run.status !== "completed" && run.error[0]) {
        setPageError(run.error[0]);
      }
    } catch (error) {
      await handleProtectedError(error);
    }
  };

  const handleAllowlistSave = async (principals: string[]) => {
    setAllowlistAction({ pending: true, error: null, success: null });
    try {
      const nextAllowedPrincipals = await updateAllowedPrincipals(principals);
      setAllowedPrincipals(nextAllowedPrincipals);
      setAllowlistAction({
        pending: false,
        error: null,
        success: `saved ${nextAllowedPrincipals.length} operator principal${nextAllowedPrincipals.length === 1 ? "" : "s"}`,
      });
    } catch (error) {
      const normalized = normalizeError(error);
      setAllowlistAction({ pending: false, error: normalized.message, success: null });
      if (normalized.code === "unauthorized") {
        await denyAccess(normalized.message);
      }
    }
  };

  const refreshSchedules = async () => {
    try {
      const nextSchedules = await fetchSchedules();
      const nextLatestScheduleRunsEntries = await Promise.all(
        nextSchedules.map(async (schedule) => [
          schedule.id,
          schedule.last_run_id[0] ? await fetchRun(schedule.last_run_id[0]) : null,
        ] as const),
      );
      setSchedules(nextSchedules);
      setLatestScheduleRuns(Object.fromEntries(nextLatestScheduleRunsEntries));
      setSelectedScheduleId((prev) => prev || nextSchedules[0]?.id || "");
      setScheduleAction((prev) => ({ ...prev, error: null }));
    } catch (error) {
      await handleProtectedError(error);
    }
  };

  const handleScheduleCreate = async (draft: {
    id: string;
    name: string;
    agentId: string;
    prompt: string;
    intervalMinutes: bigint;
    sessionMode: string;
    fixedSessionId?: string;
    enabled: boolean;
  }) => {
    setScheduleAction({ pending: true, error: null, success: null });
    try {
      const created = await createSchedule(draft);
      await refreshSchedules();
      setSelectedScheduleId(created.id);
      setScheduleAction({
        pending: false,
        error: null,
        success: `created ${created.id}`,
      });
    } catch (error) {
      const normalized = normalizeError(error);
      setScheduleAction({ pending: false, error: normalized.message, success: null });
    }
  };

  const handleScheduleUpdate = async (schedule: Schedule) => {
    setScheduleAction({ pending: true, error: null, success: null });
    try {
      await updateSchedule(schedule);
      await refreshSchedules();
      setSelectedScheduleId(schedule.id);
      setScheduleAction({
        pending: false,
        error: null,
        success: `${schedule.id} updated`,
      });
    } catch (error) {
      const normalized = normalizeError(error);
      setScheduleAction({ pending: false, error: normalized.message, success: null });
    }
  };

  const handleScheduleToggle = async (schedule: Schedule) => {
    setScheduleAction({ pending: true, error: null, success: null });
    try {
      await updateSchedule({
        ...schedule,
        enabled: !schedule.enabled,
      });
      await refreshSchedules();
      setScheduleAction({
        pending: false,
        error: null,
        success: `${schedule.id} ${schedule.enabled ? "disabled" : "enabled"}`,
      });
    } catch (error) {
      const normalized = normalizeError(error);
      setScheduleAction({ pending: false, error: normalized.message, success: null });
    }
  };

  const handleScheduleTrigger = async (scheduleId: string) => {
    setScheduleAction({ pending: true, error: null, success: null });
    try {
      const run = await triggerSchedule(scheduleId);
      await refreshSchedules();
      if (run.session_id !== sessionId) {
        setSessionId(run.session_id);
      }
      await refreshRuns(run.session_id, run.id);
      setScheduleAction({
        pending: false,
        error: null,
        success: `${scheduleId} triggered manually (disabled でも実行可)`,
      });
      if (run.status !== "completed" && run.error[0]) {
        setPageError(run.error[0]);
      }
    } catch (error) {
      const normalized = normalizeError(error);
      setScheduleAction({ pending: false, error: normalized.message, success: null });
    }
  };

  const refreshWebhooks = async () => {
    try {
      const nextWebhooks = await fetchWebhooks();
      const nextLatestWebhookRunsEntries = await Promise.all(
        nextWebhooks.map(async (webhook) => [
          webhook.id,
          webhook.last_run_id[0] ? await fetchRun(webhook.last_run_id[0]) : null,
        ] as const),
      );
      const nextWebhookRejectionsEntries = await Promise.all(
        nextWebhooks.map(async (webhook) => [webhook.id, await fetchWebhookRejections(webhook.id)] as const),
      );
      setWebhooks(nextWebhooks);
      setLatestWebhookRuns(Object.fromEntries(nextLatestWebhookRunsEntries));
      setWebhookRejections(Object.fromEntries(nextWebhookRejectionsEntries));
      setSelectedWebhookId((prev) => prev || nextWebhooks[0]?.id || "");
      setWebhookAction((prev) => ({ ...prev, error: null }));
    } catch (error) {
      await handleProtectedError(error);
    }
  };

  const handleWebhookCreate = async (draft: {
    id: string;
    name: string;
    agentId: string;
    sessionMode: string;
    fixedSessionId?: string;
    secret: string;
    enabled: boolean;
  }) => {
    setWebhookAction({ pending: true, error: null, success: null });
    try {
      const created = await createWebhook(draft);
      await refreshWebhooks();
      setSelectedWebhookId(created.id);
      setWebhookAction({
        pending: false,
        error: null,
        success: `created ${created.id} secret=${created.secret}`,
      });
    } catch (error) {
      const normalized = normalizeError(error);
      setWebhookAction({ pending: false, error: normalized.message, success: null });
    }
  };

  const handleWebhookToggle = async (webhook: Webhook) => {
    setWebhookAction({ pending: true, error: null, success: null });
    try {
      await updateWebhook({
        ...webhook,
        enabled: !webhook.enabled,
      });
      await refreshWebhooks();
      setWebhookAction({
        pending: false,
        error: null,
        success: `${webhook.id} ${webhook.enabled ? "disabled" : "enabled"}`,
      });
    } catch (error) {
      const normalized = normalizeError(error);
      setWebhookAction({ pending: false, error: normalized.message, success: null });
    }
  };

  const handleWebhookUpdate = async (webhook: Webhook, secretOverride?: string) => {
    setWebhookAction({ pending: true, error: null, success: null });
    try {
      await updateWebhook(webhook, secretOverride);
      await refreshWebhooks();
      setSelectedWebhookId(webhook.id);
      setWebhookAction({
        pending: false,
        error: null,
        success: secretOverride ? `${webhook.id} updated with a new secret` : `${webhook.id} updated`,
      });
    } catch (error) {
      const normalized = normalizeError(error);
      setWebhookAction({ pending: false, error: normalized.message, success: null });
    }
  };

  const handleWebhookRotate = async (webhookId: string) => {
    setWebhookAction({ pending: true, error: null, success: null });
    try {
      const rotated = await rotateWebhookSecret(webhookId);
      await refreshWebhooks();
      setSelectedWebhookId(rotated.webhook.id);
      setWebhookAction({
        pending: false,
        error: null,
        success: `rotated ${rotated.webhook.id} secret=${rotated.new_secret}`,
      });
    } catch (error) {
      const normalized = normalizeError(error);
      setWebhookAction({ pending: false, error: normalized.message, success: null });
    }
  };

  if (!authReady) {
    return <div className="flex min-h-screen items-center justify-center bg-slate-950 text-slate-200">Loading...</div>;
  }

  if (!authenticated) {
    return <UnauthenticatedScreen authError={authError} onLogin={handleLogin} />;
  }

  if (access.status === "denied") {
    return <AccessDeniedScreen principal={access.principal} message={access.message} onLogout={handleLogout} onRetry={refreshDashboard} />;
  }

  return (
    <AppRoutes
      sessionId={sessionId}
      principal={access.principal}
      onLogout={handleLogout}
      dashboard={{
        health,
        observe,
        currentAgent: selectedAgent,
        latestRun: runs.items[0] ?? null,
        latestScheduleRun,
        latestScheduleFailure,
        failingScheduleCount: failingSchedules.length,
        staleScheduleCount: staleSchedules.length,
        runningScheduleCount: runningSchedules.length,
        blockedRunCount: runs.items.filter((run) => run.status === "blocked").length,
        latestBlockedRun,
        scheduleAlerts,
        latestWebhookRun,
        latestWebhookFailure,
        scheduleCount: schedules.length,
        webhookCount: webhooks.length,
        runCount: runs.items.length,
        toolPolicies,
        allowlist: {
          principals: allowedPrincipals,
          currentPrincipal: access.principal,
          pending: allowlistAction.pending,
          error: allowlistAction.error,
          success: allowlistAction.success,
        },
        loading: access.status === "checking",
        error: pageError,
        onRefresh: refreshDashboard,
        onAllowlistRefresh: refreshAllowlist,
        onAllowlistSave: handleAllowlistSave,
      }}
      chat={{
        messages: mapRunsToMessages(runs.items),
        pending: sending,
        agentId,
        agentLocked: sessionContext !== null,
        agents,
        sessions,
        setAgentId: setDraftAgentId,
        setSessionId,
        onSend: handleSend,
        observe,
      }}
      memory={memoryController}
      runs={{
        viewModel: runs,
        onRefresh: () => refreshRuns(sessionId),
        onSelectRun: handleRunSelect,
        onCancelRun: handleRunCancel,
        onResumeRun: handleRunResume,
      }}
      agents={{
        viewModel: agentsViewModel,
        currentAgentId: agentId,
        onSelectAgent: setDraftAgentId,
      }}
      schedules={{
        viewModel: schedulesViewModel,
        agents,
        action: scheduleAction,
        onCreate: handleScheduleCreate,
        onSelectSchedule: setSelectedScheduleId,
        onUpdate: handleScheduleUpdate,
        onToggleEnabled: handleScheduleToggle,
        onTrigger: handleScheduleTrigger,
      }}
      webhooks={{
        viewModel: webhooksViewModel,
        agents,
        action: webhookAction,
        onCreate: handleWebhookCreate,
        onSelectWebhook: setSelectedWebhookId,
        onUpdate: handleWebhookUpdate,
        onToggleEnabled: handleWebhookToggle,
        onRotateSecret: handleWebhookRotate,
      }}
      observe={{ filter: observeFilter, setFilter: setObserveFilter, viewModel: observe, onRefresh: refreshObserve }}
    />
  );
}
