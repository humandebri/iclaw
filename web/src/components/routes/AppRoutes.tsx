// where: iclaw/web/src/components/routes/AppRoutes.tsx
// what: Authorized route tree for the operator console
// why: Keep App.tsx focused on state orchestration while route composition stays readable and testable

import { HashRouter, Navigate, Route, Routes } from "react-router-dom";
import { Layout } from "@/components/layout/Layout";
import { Dashboard } from "@/pages/Dashboard";
import { ChatPage } from "@/pages/Chat";
import { MemoryPage } from "@/pages/Memory";
import { ObservePage } from "@/pages/Observe";
import type { HealthResponse, MemoryCategory, MemoryItem } from "@/generated/iclaw.did";
import type {
  AsyncActionState,
  ChatMessage,
  ManifestPreviewEntry,
  MemoryUiState,
  ObserveViewModel,
} from "@/types/ui";

export function AppRoutes({
  sessionId,
  principal,
  onLogout,
  dashboard,
  chat,
  memory,
  observe,
}: {
  sessionId: string;
  principal: string;
  onLogout: () => Promise<void>;
  dashboard: {
    health: HealthResponse | null;
    observe: ObserveViewModel;
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
    sessions: { id: string; label: string; lastUsedAt: string }[];
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
}) {
  return (
    <HashRouter>
      <Routes>
        <Route element={<Layout sessionId={sessionId} onLogout={onLogout} principal={principal} />}>
          <Route path="/" element={<Dashboard {...dashboard} />} />
          <Route path="/chat" element={<ChatPage messages={chat.messages} pending={chat.pending} sessionId={sessionId} sessions={chat.sessions} setSessionId={chat.setSessionId} onSend={chat.onSend} observe={chat.observe} />} />
          <Route path="/memory" element={<MemoryPage {...memory} />} />
          <Route path="/observe" element={<ObservePage sessionId={sessionId} filter={observe.filter} setFilter={observe.setFilter} observe={observe.viewModel} onRefresh={observe.onRefresh} />} />
          <Route path="*" element={<Navigate to="/" replace />} />
        </Route>
      </Routes>
    </HashRouter>
  );
}
