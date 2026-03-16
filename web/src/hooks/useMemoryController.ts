// where: iclaw/web/src/hooks/useMemoryController.ts
// what: Memory page state and actions for the operator UI
// why: App.tsx should orchestrate auth and top-level refreshes, not own every memory-specific callback inline

import { useState } from "react";
import { fetchMemoryCount, fetchMemoryGet, fetchMemoryList, fetchMemoryRecall, forgetMemory, normalizeError, storeMemory } from "@/lib/api";
import { previewManifest, parseManifest, resolveManifestContent } from "@/lib/manifest";
import type { MemoryCategory, MemoryItem } from "@/generated/iclaw.did";
import { CORE_CATEGORY } from "@/types/ui";
import type { AsyncActionState, ManifestEntryDraft, ManifestPreviewEntry, ManifestRunEntry, MemoryUiState } from "@/types/ui";

const IDLE_ACTION: AsyncActionState = {
  pending: false,
  error: null,
  successMessage: null,
  details: { secretNotice: null },
};

function initialMemoryState(): MemoryUiState {
  return { core: IDLE_ACTION, manifest: IDLE_ACTION, advanced: IDLE_ACTION, lookup: IDLE_ACTION, query: IDLE_ACTION, manifestRuns: [] };
}

export function useMemoryController({
  refreshObserve,
  onProtectedError,
}: {
  refreshObserve: () => Promise<void>;
  onProtectedError: (error: unknown) => Promise<void>;
}) {
  const [queryResult, setQueryResult] = useState<MemoryItem[]>([]);
  const [getResult, setGetResult] = useState<MemoryItem | null>(null);
  const [count, setCount] = useState<bigint | null>(null);
  const [manifestEntries, setManifestEntries] = useState<ManifestEntryDraft[]>([]);
  const [manifestPreview, setManifestPreview] = useState<ManifestPreviewEntry[]>([]);
  const [manifestFiles, setManifestFiles] = useState<Map<string, File>>(new Map());
  const [manifestNote, setManifestNote] = useState("manifest は browser subset のみ対応です。workspace-file は upload 済み file 名にだけ一致します。");
  const [memoryUi, setMemoryUi] = useState<MemoryUiState>(initialMemoryState);

  const setMemoryAction = (key: keyof Omit<MemoryUiState, "manifestRuns">, next: AsyncActionState) => {
    setMemoryUi((prev) => ({ ...prev, [key]: next }));
  };

  const runMemoryAction = async (
    key: keyof Omit<MemoryUiState, "manifestRuns">,
    action: () => Promise<void>,
    success: string,
  ) => {
    setMemoryAction(key, { pending: true, error: null, successMessage: null, details: { secretNotice: null } });
    try {
      await action();
      setMemoryAction(key, { pending: false, error: null, successMessage: success, details: { secretNotice: null } });
    } catch (error) {
      const normalized = normalizeError(error);
      setMemoryAction(key, { pending: false, error: normalized.message, successMessage: null, details: { secretNotice: null } });
      await onProtectedError(error);
      throw error;
    }
  };

  const updateManifestPreview = (entries: ManifestEntryDraft[], files: Map<string, File>) => {
    setManifestPreview(previewManifest(entries, files));
  };

  return {
    queryResult,
    getResult,
    count,
    manifestPreview,
    manifestNote,
    memoryUi,
    onSaveCore: (key: string, content: string) => runMemoryAction("core", async () => { await storeMemory({ key, content, category: CORE_CATEGORY }); await refreshObserve(); }, `saved ${key}`),
    onAdvancedStore: (key: string, content: string, category: MemoryCategory, sessionId: string) => runMemoryAction("advanced", async () => { await storeMemory({ key, content, category, sessionId: sessionId || undefined }); await refreshObserve(); }, `stored ${key}`),
    onForget: (key: string) => runMemoryAction("lookup", async () => { await forgetMemory(key); setGetResult(null); await refreshObserve(); }, `forgot ${key}`),
    onLookup: (key: string) => runMemoryAction("lookup", async () => { setGetResult(await fetchMemoryGet(key)); }, `loaded ${key}`),
    onList: (category?: MemoryCategory, sessionId?: string) => runMemoryAction("query", async () => { setQueryResult(await fetchMemoryList(category, sessionId || undefined)); }, "list updated"),
    onRecall: (query: string, limit: bigint, sessionId?: string) => runMemoryAction("query", async () => { setQueryResult(await fetchMemoryRecall(query, limit, sessionId || undefined)); }, `recall for ${query}`),
    onCount: () => runMemoryAction("lookup", async () => { setCount(await fetchMemoryCount()); }, "count refreshed"),
    onManifestTextChange: (raw: string) => {
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
    },
    onManifestFiles: (files: FileList | null) => {
      const next = new Map<string, File>();
      for (const file of Array.from(files ?? [])) next.set(file.name, file);
      setManifestFiles(next);
      updateManifestPreview(manifestEntries, next);
    },
    onRunManifest: () => runMemoryAction("manifest", async () => {
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
    }, `manifest stored ${manifestEntries.length} entries`),
  };
}
