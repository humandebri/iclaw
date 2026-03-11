// where: standalone/web/src/types/ui.ts
// what: UI-local state shapes for the iclaw caller console
// why: Keep browser orchestration types separate from generated canister bindings

import type { AgentObservation, MemoryCategory, MemoryItem } from "@/generated/iclaw.did";

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
  success: string | null;
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

export const CORE_CATEGORY: MemoryCategory = { core: null };
