// where: standalone/web/src/hooks/useSessionState.ts
// what: Shared session selector state for the IC caller UI
// why: Dashboard, chat, memory, and observe should all operate on the same caller-selected session

import { useEffect, useState } from "react";
import { currentSessionId, loadSessions, rememberSession } from "@/lib/session-store";
import type { CurrentSession } from "@/types/ui";

export function useSessionState() {
  const [sessions, setSessions] = useState<CurrentSession[]>(() => loadSessions());
  const [sessionId, setSessionId] = useState<string>(() => currentSessionId());

  useEffect(() => {
    if (!sessionId) {
      return;
    }
    setSessions(rememberSession(sessionId));
  }, [sessionId]);

  const saveSession = (value: string, label?: string) => {
    setSessionId(value);
    if (value.trim()) {
      setSessions(rememberSession(value, label));
    }
  };

  return {
    sessions,
    sessionId,
    setSessionId: saveSession,
  };
}
