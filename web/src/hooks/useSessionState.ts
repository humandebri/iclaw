// where: iclaw/web/src/hooks/useSessionState.ts
// what: Shared session selector state for the IC caller UI
// why: Dashboard, chat, memory, and observe should all operate on the same caller-selected session

import { useEffect, useState } from "react";
import { fetchSessions } from "@/lib/api";
import { currentSessionId, loadSessions, rememberSession } from "@/lib/session-store";
import type { CurrentSession } from "@/types/ui";

export function useSessionState() {
  const [sessions, setSessions] = useState<CurrentSession[]>([]);
  const [sessionId, setSessionId] = useState<string>(() => currentSessionId());

  useEffect(() => {
    let ignore = false;
    void (async () => {
      try {
        const stored = loadSessions();
        const fetched = await fetchSessions();
        if (ignore) {
          return;
        }
        const mapped = fetched.map((session) => {
          const remembered = stored.find((entry) => entry.id === session.id);
          return {
            id: session.id,
            label: session.title.trim() || remembered?.label || session.id,
            lastUsedAt: remembered?.lastUsedAt || session.updated_at,
          };
        });
        if (!sessionId && mapped[0]) {
          setSessionId(mapped[0].id);
          rememberSession(mapped[0].id, mapped[0].label);
        } else if (!mapped[0] && sessionId) {
          const rememberedCurrent = stored.find((entry) => entry.id === sessionId);
          if (rememberedCurrent) {
            mapped.unshift(rememberedCurrent);
          }
        }
        setSessions(mapped);
      } catch {
        if (!ignore) {
          setSessions(loadSessions());
        }
      }
    })();
    return () => {
      ignore = true;
    };
  }, [sessionId]);

  const saveSession = (value: string, label?: string) => {
    setSessionId(value);
    if (value.trim()) {
      rememberSession(value, label);
    }
  };

  return {
    sessions,
    sessionId,
    setSessionId: saveSession,
  };
}
