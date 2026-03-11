// where: iclaw/web/src/App.tsx
// what: Main app composition for the single-canister caller console
// why: Keep auth, access control, session state, and per-screen async status in one orchestration layer

import { useEffect, useRef, useState } from "react";
import { HashRouter, Navigate, Route, Routes } from "react-router-dom";
import { Layout } from "@/components/layout/Layout";
import { Dashboard } from "@/pages/Dashboard";
import { ChatPage } from "@/pages/Chat";
import { MemoryPage } from "@/pages/Memory";
import { ObservePage } from "@/pages/Observe";
import {
  currentPrincipalText,
  ensureOperatorAccess,
  fetchHealth,
  fetchMemoryCount,
  fetchMemoryGet,
  fetchMemoryList,
  fetchMemoryRecall,
  fetchObserve,
  fetchSummary,
  forgetMemory,
  isAuthenticated,
  login,
  logout,
  normalizeError,
  sendChat,
  storeMemory,
} from "@/lib/api";
import { sessionChanged } from "@/lib/chat-state";
import { previewManifest, parseManifest, resolveManifestContent } from "@/lib/manifest";
import { useSessionState } from "@/hooks/useSessionState";
import type { HealthResponse, MemoryItem } from "@/generated/iclaw.did";
import { CORE_CATEGORY } from "@/types/ui";
import type {
  AccessState,
  AsyncActionState,
  ChatMessage,
  ManifestEntryDraft,
  ManifestPreviewEntry,
  ManifestRunEntry,
  MemoryUiState,
  ObserveViewModel,
} from "@/types/ui";

const IDLE_ACTION: AsyncActionState = { pending: false, error: null, success: null };

function initialMemoryState(): MemoryUiState {
  return { core: IDLE_ACTION, manifest: IDLE_ACTION, advanced: IDLE_ACTION, lookup: IDLE_ACTION, query: IDLE_ACTION, manifestRuns: [] };
}

export default function App() {
  const { sessions, sessionId, setSessionId } = useSessionState();
  const previousSessionId = useRef(sessionId);
  const [authenticated, setAuthenticated] = useState(false);
  const [authReady, setAuthReady] = useState(false);
  const [authError, setAuthError] = useState<string | null>(null);
  const [access, setAccess] = useState<AccessState>({ status: "checking", principal: "", message: null });
  const [pageError, setPageError] = useState<string | null>(null);
  const [health, setHealth] = useState<HealthResponse | null>(null);
  const [observe, setObserve] = useState<ObserveViewModel>({ observation: null, summary: null });
  const [observeFilter, setObserveFilter] = useState("");
  const [messages, setMessages] = useState<ChatMessage[]>([]);
  const [sending, setSending] = useState(false);
  const [queryResult, setQueryResult] = useState<MemoryItem[]>([]);
  const [getResult, setGetResult] = useState<MemoryItem | null>(null);
  const [count, setCount] = useState<bigint | null>(null);
  const [manifestEntries, setManifestEntries] = useState<ManifestEntryDraft[]>([]);
  const [manifestPreview, setManifestPreview] = useState<ManifestPreviewEntry[]>([]);
  const [manifestFiles, setManifestFiles] = useState<Map<string, File>>(new Map());
  const [manifestNote, setManifestNote] = useState("manifest は browser subset のみ対応です。workspace-file は upload 済み file 名にだけ一致します。");
  const [memoryUi, setMemoryUi] = useState<MemoryUiState>(initialMemoryState);

  const activeObserveSession = observeFilter || sessionId;

  const setMemoryAction = (key: keyof Omit<MemoryUiState, "manifestRuns">, next: AsyncActionState) => {
    setMemoryUi((prev) => ({ ...prev, [key]: next }));
  };

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
      const [nextHealth, nextObserve, nextSummary] = await Promise.all([
        fetchHealth(),
        fetchObserve(sessionId || undefined),
        sessionId ? fetchSummary(sessionId) : Promise.resolve(null),
      ]);
      setHealth(nextHealth);
      setObserve({ observation: nextObserve, summary: nextSummary });
      setPageError(null);
      setAccess((prev) => ({ ...prev, status: "allowed", message: null }));
    } catch (error) {
      await handleProtectedError(error);
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

  const runMemoryAction = async (
    key: keyof Omit<MemoryUiState, "manifestRuns">,
    action: () => Promise<void>,
    success: string,
  ) => {
    setMemoryAction(key, { pending: true, error: null, success: null });
    try {
      await action();
      setMemoryAction(key, { pending: false, error: null, success });
    } catch (error) {
      const normalized = normalizeError(error);
      setMemoryAction(key, { pending: false, error: normalized.message, success: null });
      if (normalized.code === "unauthorized") {
        await denyAccess(normalized.message);
      }
    }
  };

  const updateManifestPreview = (entries: ManifestEntryDraft[], files: Map<string, File>) => {
    setManifestPreview(previewManifest(entries, files));
  };

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

  if (!authReady) {
    return <div className="flex min-h-screen items-center justify-center bg-slate-950 text-slate-200">Loading...</div>;
  }

  if (!authenticated) {
    return (
      <div className="flex min-h-screen items-center justify-center bg-slate-950 px-6 text-white">
        <div className="w-full max-w-lg rounded-[28px] border border-white/10 bg-slate-900/80 p-8 shadow-2xl shadow-slate-950/50">
          <p className="text-sm uppercase tracking-[0.24em] text-blue-200/70">Operator Access</p>
          <h1 className="mt-3 text-3xl font-semibold">iclaw</h1>
          <p className="mt-4 text-sm text-slate-400">この caller UI は運用者 principal 向けです。Internet Identity でログインし、allowlist 済み principal でのみ利用できます。</p>
          {authError && <p className="mt-4 rounded-xl border border-red-400/20 bg-red-500/10 px-4 py-3 text-sm text-red-100">{authError}</p>}
          <button data-tid="login-button" type="button" onClick={() => void handleLogin()} className="mt-6 w-full rounded-2xl bg-blue-600 px-4 py-3 text-sm font-medium text-white transition-colors hover:bg-blue-500">
            Sign in with Internet Identity
          </button>
        </div>
      </div>
    );
  }

  if (access.status === "denied") {
    return (
      <div className="flex min-h-screen items-center justify-center bg-slate-950 px-6 text-white">
        <div className="w-full max-w-xl rounded-[28px] border border-red-400/20 bg-slate-900/90 p-8 shadow-2xl shadow-slate-950/50">
          <p className="text-sm uppercase tracking-[0.24em] text-red-200/70">Access denied</p>
          <h1 className="mt-3 text-3xl font-semibold">Operator principal is required</h1>
          <p
            data-tid="access-denied-principal"
            className="mt-4 rounded-xl border border-white/10 bg-slate-950/60 px-4 py-3 text-sm text-slate-200"
          >
            principal: {access.principal || "unavailable"}
          </p>
          <p className="mt-4 text-sm text-slate-400">{access.message ?? "この principal は canister allowlist に含まれていません。"}</p>
          <div className="mt-6 flex gap-3">
            <button type="button" onClick={() => void handleLogout()} className="rounded-2xl border border-white/10 px-4 py-3 text-sm text-slate-100 hover:bg-white/5">Sign out</button>
            <button type="button" onClick={() => void refreshDashboard()} className="rounded-2xl bg-blue-600 px-4 py-3 text-sm font-medium text-white hover:bg-blue-500">Retry access check</button>
          </div>
        </div>
      </div>
    );
  }

  return (
    <HashRouter>
      <Routes>
        <Route element={<Layout sessionId={sessionId} onLogout={handleLogout} principal={access.principal} />}>
          <Route path="/" element={<Dashboard health={health} observe={observe} loading={access.status === "checking"} error={pageError} onRefresh={refreshDashboard} />} />
          <Route path="/chat" element={<ChatPage messages={messages} pending={sending} sessionId={sessionId} sessions={sessions} setSessionId={setSessionId} onSend={handleSend} observe={observe} />} />
          <Route
            path="/memory"
            element={
              <MemoryPage
                queryResult={queryResult}
                getResult={getResult}
                count={count}
                manifestPreview={manifestPreview}
                manifestNote={manifestNote}
                memoryUi={memoryUi}
                onSaveCore={(key, content) => runMemoryAction("core", async () => { await storeMemory({ key, content, category: CORE_CATEGORY }); await refreshObserve(); }, `saved ${key}`)}
                onAdvancedStore={(key, content, category, currentSessionId) => runMemoryAction("advanced", async () => { await storeMemory({ key, content, category, sessionId: currentSessionId || undefined }); await refreshObserve(); }, `stored ${key}`)}
                onForget={(key) => runMemoryAction("lookup", async () => { await forgetMemory(key); setGetResult(null); await refreshObserve(); }, `forgot ${key}`)}
                onLookup={(key) => runMemoryAction("lookup", async () => { setGetResult(await fetchMemoryGet(key)); }, `loaded ${key}`)}
                onList={(category, currentSessionId) => runMemoryAction("query", async () => { setQueryResult(await fetchMemoryList(category, currentSessionId || undefined)); }, "list updated")}
                onRecall={(query, limit, currentSessionId) => runMemoryAction("query", async () => { setQueryResult(await fetchMemoryRecall(query, limit, currentSessionId || undefined)); }, `recall for ${query}`)}
                onCount={() => runMemoryAction("lookup", async () => { setCount(await fetchMemoryCount()); }, "count refreshed")}
                onManifestTextChange={(raw) => {
                  try {
                    const entries = parseManifest(raw);
                    setManifestEntries(entries);
                    setManifestNote("browser subset: local path resolution is disabled; uploaded files are matched by name.");
                    updateManifestPreview(entries, manifestFiles);
                  } catch (error) {
                    setManifestEntries([]);
                    setManifestPreview([]);
                    setManifestNote(normalizeError(error).message);
                  }
                }}
                onManifestFiles={(files) => {
                  const next = new Map<string, File>();
                  for (const file of Array.from(files ?? [])) next.set(file.name, file);
                  setManifestFiles(next);
                  updateManifestPreview(manifestEntries, next);
                }}
                onRunManifest={() => runMemoryAction("manifest", async () => {
                  const results: ManifestRunEntry[] = [];
                  for (const preview of manifestPreview) if (preview.status !== "ready") throw new Error(`manifest contains ${preview.status}: ${preview.entry.key}`);
                  for (const entry of manifestEntries) {
                    try {
                      const content = await resolveManifestContent(entry, manifestFiles);
                      await storeMemory({ key: entry.key, content, category: CORE_CATEGORY });
                      results.push({ key: entry.key, status: "success", detail: "stored" });
                    } catch (error) {
                      results.push({ key: entry.key, status: "error", detail: normalizeError(error).message });
                    }
                  }
                  setMemoryUi((prev) => ({ ...prev, manifestRuns: results }));
                  await refreshObserve();
                  const failed = results.find((entry) => entry.status === "error");
                  if (failed) throw new Error(`manifest failed at ${failed.key}: ${failed.detail}`);
                }, `manifest stored ${manifestEntries.length} entries`)}
              />
            }
          />
          <Route path="/observe" element={<ObservePage sessionId={sessionId} filter={observeFilter} setFilter={setObserveFilter} observe={observe} onRefresh={refreshObserve} />} />
          <Route path="*" element={<Navigate to="/" replace />} />
        </Route>
      </Routes>
    </HashRouter>
  );
}
