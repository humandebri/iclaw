// where: iclaw/web/src/types/ui.ts
// what: UI-local state shapes for the iclaw caller console
// why: Keep browser orchestration types separate from generated canister bindings

import type {
  Agent,
  AgentObservation,
  MemoryCategory,
  MemoryItem,
  Run,
  RunEvent,
  Schedule,
  ToolPolicy,
  Webhook,
  WebhookRejection,
} from "@/generated/iclaw.did";

export interface CurrentSession {
  id: string;
  label: string;
  lastUsedAt: string;
}

export interface ObserveViewModel {
  observation: AgentObservation | null;
  summary: MemoryItem | null;
}

export interface UiError {
  code: string;
  message: string;
}

export interface AccessState {
  status: "checking" | "allowed" | "denied";
  principal: string;
  message: string | null;
}

export interface AsyncActionState {
  pending: boolean;
  error: string | null;
  successMessage: string | null;
  details: {
    secretNotice: {
      summary: string;
      secret: string;
    } | null;
  };
}

export interface ManifestRunEntry {
  key: string;
  status: "success" | "error";
  detail: string;
}

export interface MemoryUiState {
  core: AsyncActionState;
  manifest: AsyncActionState;
  advanced: AsyncActionState;
  lookup: AsyncActionState;
  query: AsyncActionState;
  manifestRuns: ManifestRunEntry[];
}

export interface ManifestEntryDraft {
  kind: "core-entry" | "workspace-file";
  key: string;
  text?: string;
  file?: string;
}

export interface ManifestPreviewEntry {
  entry: ManifestEntryDraft;
  status: "ready" | "missing-file" | "invalid";
  detail: string;
}

export interface ChatMessage {
  id: string;
  role: "user" | "assistant";
  content: string;
  timestamp: string;
}

export interface RunsViewModel {
  items: Run[];
  selectedRun: Run | null;
  events: RunEvent[];
}

export interface AgentsViewModel {
  items: Agent[];
  selectedAgent: Agent | null;
  toolPolicies: ToolPolicy[];
}

export interface WebhooksViewModel {
  items: Webhook[];
  selectedWebhook: Webhook | null;
  latestRuns: Record<string, Run | null>;
  rejections: Record<string, WebhookRejection[]>;
}

export interface SchedulesViewModel {
  items: Schedule[];
  selectedSchedule: Schedule | null;
  latestRuns: Record<string, Run | null>;
}

export interface ScheduleAlertItem {
  id: string;
  name: string;
  enabled: boolean;
  running: boolean;
  stale: boolean;
  consecutiveFailureCount: bigint;
  lastSuccessAt: string | null;
  nextRunAt: string | null;
}

export const CORE_CATEGORY: MemoryCategory = { core: null };
