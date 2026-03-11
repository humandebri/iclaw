// where: iclaw/web/src/App.tsx
// what: Main app composition for the single-canister caller console
// why: Keep auth, access control, session state, and per-screen async status in one orchestration layer

import { useEffect, useRef, useState } from "react";
import { AccessDeniedScreen, UnauthenticatedScreen } from "@/components/access/AuthScreens";
import { AppRoutes } from "@/components/routes/AppRoutes";
import {
  currentPrincipalText,
  ensureOperatorAccess,
  fetchAllowedPrincipals,
  fetchHealth,
  fetchObserve,
  fetchSummary,
  isAuthenticated,
  login,
  logout,
  normalizeError,
  sendChat,
  updateAllowedPrincipals,
} from "@/lib/api";
import { sessionChanged } from "@/lib/chat-state";
import { useMemoryController } from "@/hooks/useMemoryController";
import { useSessionState } from "@/hooks/useSessionState";
import type { HealthResponse } from "@/generated/iclaw.did";
import type { AccessState, AsyncActionState, ChatMessage, ObserveViewModel } from "@/types/ui";

const IDLE_ACTION: AsyncActionState = { pending: false, error: null, success: null };

export default function App() {
  const { sessions, sessionId, setSessionId } = useSessionState();
  const previousSessionId = useRef(sessionId);
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
  const [messages, setMessages] = useState<ChatMessage[]>([]);
  const [sending, setSending] = useState(false);

  const activeObserveSession = observeFilter || sessionId;

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

  const refreshDashboard = async () => {
    try {
      const [nextHealth, nextObserve, nextSummary, nextAllowedPrincipals] = await Promise.all([
        fetchHealth(),
        fetchObserve(sessionId || undefined),
        sessionId ? fetchSummary(sessionId) : Promise.resolve(null),
        fetchAllowedPrincipals(),
      ]);
      setHealth(nextHealth);
      setAllowedPrincipals(nextAllowedPrincipals);
      setObserve({ observation: nextObserve, summary: nextSummary });
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
    if (sessionChanged(previousSessionId.current, sessionId)) {
      setMessages([]);
      previousSessionId.current = sessionId;
    }
  }, [sessionId]);

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
    setMessages([]);
  };

  const handleSend = async (prompt: string) => {
    setSending(true);
    setPageError(null);
    setMessages((prev) => [...prev, { id: crypto.randomUUID(), role: "user", content: prompt, timestamp: new Date().toISOString() }]);
    try {
      const response = await sendChat({ prompt, sessionId: sessionId || undefined, temperature: 0 });
      setMessages((prev) => [...prev, { id: crypto.randomUUID(), role: "assistant", content: response.response, timestamp: new Date().toISOString() }]);
      await refreshDashboard();
    } catch (error) {
      await handleProtectedError(error);
      setMessages((prev) => prev.slice(0, -1));
    } finally {
      setSending(false);
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
      chat={{ messages, pending: sending, sessions, setSessionId, onSend: handleSend, observe }}
      memory={memoryController}
      observe={{ filter: observeFilter, setFilter: setObserveFilter, viewModel: observe, onRefresh: refreshObserve }}
    />
  );
}
