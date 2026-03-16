// where: iclaw/web/src/lib/session-store.ts
// what: Browser-side session preference cache for the caller UX
// why: The canister is the source of truth for session lists, but the browser still remembers the last selected session

import type { CurrentSession } from "@/types/ui";

const STORAGE_KEY = "iclaw.sessions";
const MAX_SESSIONS = 12;

function parseSessions(raw: string | null): CurrentSession[] {
  if (!raw) {
    return [];
  }
  try {
    const parsed = JSON.parse(raw) as CurrentSession[];
    return Array.isArray(parsed) ? parsed.filter((entry) => entry.id.trim().length > 0) : [];
  } catch {
    return [];
  }
}

export function loadSessions(): CurrentSession[] {
  return parseSessions(window.localStorage.getItem(STORAGE_KEY));
}

export function saveSessions(sessions: CurrentSession[]): void {
  window.localStorage.setItem(STORAGE_KEY, JSON.stringify(sessions.slice(0, MAX_SESSIONS)));
}

export function rememberSession(id: string, label?: string): CurrentSession[] {
  const trimmed = id.trim();
  if (!trimmed) {
    return loadSessions();
  }
  const next: CurrentSession = {
    id: trimmed,
    label: label?.trim() || trimmed,
    lastUsedAt: new Date().toISOString(),
  };
  const remaining = loadSessions().filter((entry) => entry.id !== trimmed);
  const sessions = [next, ...remaining].slice(0, MAX_SESSIONS);
  saveSessions(sessions);
  return sessions;
}

export function currentSessionId(): string {
  return loadSessions()[0]?.id ?? "";
}
