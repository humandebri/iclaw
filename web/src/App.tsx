// where: iclaw/web/src/App.tsx
// what: Main app composition for the single-canister caller console
// why: Keep auth, access control, session state, and per-screen async status in one orchestration layer

import { useEffect, useState } from "react";
import { AccessDeniedScreen, UnauthenticatedScreen } from "@/components/access/AuthScreens";
import { AppRoutes } from "@/components/routes/AppRoutes";
import {
  cancelRun,
  createRun,
  currentPrincipalText,
  ensureOperatorAccess,
  fetchAllowedPrincipals,
  fetchHealth,
  fetchObserve,
  fetchRunEvents,
  fetchRuns,
  fetchSummary,
  isAuthenticated,
  login,
  logout,
  normalizeError,
  updateAllowedPrincipals,
} from "@/lib/api";
import { useMemoryController } from "@/hooks/useMemoryController";
import { useSessionState } from "@/hooks/useSessionState";
import type { HealthResponse } from "@/generated/iclaw.did";
import type { AccessState, AsyncActionState, ChatMessage, ObserveViewModel, RunsViewModel } from "@/types/ui";

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
  const [allowedPrincipals, setAllowedPrincipals] = useState<string[]>([]);
  const [allowlistAction, setAllowlistAction] = useState<AsyncActionState>(IDLE_ACTION);
  const [observe, setObserve] = useState<ObserveViewModel>({ observation: null, summary: null });
  const [observeFilter, setObserveFilter] = useState("");
  const [sending, setSending] = useState(false);
  const [runs, setRuns] = useState<RunsViewModel>(EMPTY_RUNS);

  const activeObserveSession = observeFilter || sessionId;

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

  const refreshDashboard = async (targetSessionId = sessionId) => {
    try {
      const [nextHealth, nextObserve, nextSummary, nextAllowedPrincipals] = await Promise.all([
        fetchHealth(),
        fetchObserve(targetSessionId || undefined),
        targetSessionId ? fetchSummary(targetSessionId) : Promise.resolve(null),
        fetchAllowedPrincipals(),
      ]);
      setHealth(nextHealth);
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
      const run = await createRun({ prompt, sessionId: sessionId || undefined, temperature: 0 });
      if (run.session_id !== sessionId) {
        setSessionId(run.session_id);
      }
      await refreshRuns(run.session_id, run.id);
      await refreshDashboard(run.session_id);
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
        latestRun: runs.items[0] ?? null,
        runCount: runs.items.length,
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
        sessions,
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
      }}
      observe={{ filter: observeFilter, setFilter: setObserveFilter, viewModel: observe, onRefresh: refreshObserve }}
    />
  );
}
