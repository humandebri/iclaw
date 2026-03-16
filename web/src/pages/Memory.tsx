// where: iclaw/web/src/pages/Memory.tsx
// what: Core editing, browser-safe manifest ingestion, and memory query tools in one page
// why: This page is the thin operator surface for durable knowledge and seed operations

import { useMemo, useState } from "react";
import { CORE_CATEGORY } from "@/types/ui";
import { Badge, Card } from "@/components/ui/Card";
import type { ManifestPreviewEntry, MemoryUiState } from "@/types/ui";
import type { MemoryCategory, MemoryItem } from "@/generated/iclaw.did";

const CATEGORY_OPTIONS: Array<{ label: string; value: MemoryCategory }> = [
  { label: "core", value: { core: null } },
  { label: "conversation", value: { conversation: null } },
  { label: "daily", value: { daily: null } },
];

const inputClassName = "w-full rounded-xl border border-zinc-200 bg-zinc-50/80 px-4 py-3 text-sm text-zinc-900";
const textareaClassName = "w-full rounded-2xl border border-zinc-200 bg-zinc-50/80 px-4 py-3 text-sm text-zinc-900";
const panelClassName = "rounded-2xl border border-zinc-200 bg-zinc-50/70 p-4";
const subtlePanelClassName = "rounded-xl border border-zinc-200 bg-white/70 px-3 py-3 text-sm text-zinc-700";

export function MemoryPage(props: {
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
}) {
  const [coreKey, setCoreKey] = useState("core/user_preferences/response_style");
  const [coreContent, setCoreContent] = useState("");
  const [advancedKey, setAdvancedKey] = useState("");
  const [advancedContent, setAdvancedContent] = useState("");
  const [advancedSessionId, setAdvancedSessionId] = useState("");
  const [advancedCategory, setAdvancedCategory] = useState("core");
  const [lookupKey, setLookupKey] = useState("");
  const [listSessionId, setListSessionId] = useState("");
  const [recallQuery, setRecallQuery] = useState("");
  const [recallLimit, setRecallLimit] = useState("10");
  const [manifestText, setManifestText] = useState("");

  const selectedCategory = useMemo(
    () => CATEGORY_OPTIONS.find((option) => option.label === advancedCategory)?.value ?? CORE_CATEGORY,
    [advancedCategory],
  );
  const runAction = (action: () => Promise<void>) => {
    void action().catch(() => undefined);
  };

  return (
    <div className="space-y-6">
      <div className="grid gap-6 xl:grid-cols-[1.1fr_0.9fr]">
        <Card title="Manual Core Save" subtitle="category=core / session_id=null を既定に固定">
          <div className="space-y-3">
            <input
              value={coreKey}
              onChange={(event) => setCoreKey(event.target.value)}
              className={inputClassName}
            />
            <textarea
              value={coreContent}
              onChange={(event) => setCoreContent(event.target.value)}
              rows={6}
              placeholder="durable content"
              className={textareaClassName}
            />
            <div className="flex items-center justify-between">
              <Badge tone="info">core + null session</Badge>
              <button
                type="button"
                disabled={props.memoryUi.core.pending}
                onClick={() => runAction(() => props.onSaveCore(coreKey, coreContent))}
                className="rounded-xl bg-blue-600 px-4 py-2 text-sm font-medium text-white hover:bg-blue-500"
              >
                {props.memoryUi.core.pending ? "Saving..." : "Save core"}
              </button>
            </div>
            <StatusPanel success={props.memoryUi.core.successMessage} error={props.memoryUi.core.error} />
          </div>
        </Card>

        <Card title="Manifest Seed" subtitle="browser subset only: local path resolution はしない">
          <div className="space-y-3">
            <textarea
              value={manifestText}
              onChange={(event) => {
                const value = event.target.value;
                setManifestText(value);
                props.onManifestTextChange(value);
              }}
              rows={8}
              placeholder='{"entries":[{"kind":"core-entry","key":"core/project_facts/runtime","text":"..."}]}'
              className={textareaClassName}
            />
            <input
              type="file"
              multiple
              onChange={(event) => props.onManifestFiles(event.target.files)}
              className="block w-full text-sm text-zinc-600 file:mr-4 file:rounded-xl file:border-0 file:bg-zinc-900 file:px-4 file:py-2 file:text-sm file:font-medium file:text-white"
            />
            <p className="text-sm text-zinc-500">{props.manifestNote}</p>
            <div className="max-h-52 space-y-2 overflow-y-auto rounded-2xl border border-zinc-200 bg-zinc-50/70 p-3">
              {props.manifestPreview.length === 0 && (
                <p className="text-sm text-zinc-500">manifest preview はここに出ます。</p>
              )}
              {props.manifestPreview.map((preview) => (
                <div key={`${preview.entry.kind}:${preview.entry.key}`} className="rounded-xl border border-zinc-200 bg-white/70 px-3 py-2">
                  <div className="flex items-center gap-2">
                    <Badge tone={preview.status === "ready" ? "good" : preview.status === "missing-file" ? "warn" : "neutral"}>
                      {preview.status}
                    </Badge>
                    <span className="text-sm text-zinc-800">{preview.entry.key}</span>
                  </div>
                  <p className="mt-2 text-xs text-zinc-500">{preview.detail}</p>
                </div>
              ))}
            </div>
            <button
              type="button"
              disabled={props.memoryUi.manifest.pending}
              onClick={() => runAction(props.onRunManifest)}
              className="rounded-xl bg-blue-600 px-4 py-2 text-sm font-medium text-white hover:bg-blue-500"
            >
              {props.memoryUi.manifest.pending ? "Running..." : "Run manifest seed"}
            </button>
            <StatusPanel success={props.memoryUi.manifest.successMessage} error={props.memoryUi.manifest.error} />
            {props.memoryUi.manifestRuns.length > 0 && (
              <div className="space-y-2 rounded-2xl border border-zinc-200 bg-zinc-50/70 p-3">
                {props.memoryUi.manifestRuns.map((entry) => (
                  <div key={entry.key} className="flex items-center justify-between gap-3 text-sm">
                    <span className="text-zinc-800">{entry.key}</span>
                    <Badge tone={entry.status === "success" ? "good" : "warn"}>{entry.detail}</Badge>
                  </div>
                ))}
              </div>
            )}
          </div>
        </Card>
      </div>

      <div className="grid gap-6 xl:grid-cols-[0.8fr_1.2fr]">
        <Card title="Advanced Store" subtitle="必要なときだけ category / session_id を開く">
          <div className="space-y-3">
            <input value={advancedKey} onChange={(event) => setAdvancedKey(event.target.value)} placeholder="key" className={inputClassName} />
            <textarea value={advancedContent} onChange={(event) => setAdvancedContent(event.target.value)} rows={5} placeholder="content" className={textareaClassName} />
            <div className="grid gap-3 md:grid-cols-2">
              <select value={advancedCategory} onChange={(event) => setAdvancedCategory(event.target.value)} className={inputClassName}>
                {CATEGORY_OPTIONS.map((option) => (
                  <option key={option.label} value={option.label}>{option.label}</option>
                ))}
              </select>
              <input value={advancedSessionId} onChange={(event) => setAdvancedSessionId(event.target.value)} placeholder="optional session_id" className={inputClassName} />
            </div>
            <button
              type="button"
              disabled={props.memoryUi.advanced.pending}
              onClick={() => runAction(() => props.onAdvancedStore(advancedKey, advancedContent, selectedCategory, advancedSessionId))}
              className="rounded-xl border border-zinc-200 px-4 py-2 text-sm text-zinc-800 hover:bg-zinc-50"
            >
              {props.memoryUi.advanced.pending ? "Storing..." : "Store advanced memory"}
            </button>
            <StatusPanel success={props.memoryUi.advanced.successMessage} error={props.memoryUi.advanced.error} />
          </div>
        </Card>

        <Card title="Query Tools" subtitle="memory_get / list / recall / forget / count">
          <div className="grid gap-4 md:grid-cols-2">
            <div className={panelClassName}>
              <p className="text-xs uppercase tracking-[0.2em] text-zinc-500">Get / Forget</p>
              <input value={lookupKey} onChange={(event) => setLookupKey(event.target.value)} placeholder="memory key" className={inputClassName} />
              <div className="flex gap-2">
                <button type="button" disabled={props.memoryUi.lookup.pending} onClick={() => runAction(() => props.onLookup(lookupKey))} className="rounded-xl bg-blue-600 px-4 py-2 text-sm text-white hover:bg-blue-500">Get</button>
                <button type="button" disabled={props.memoryUi.lookup.pending} onClick={() => runAction(() => props.onForget(lookupKey))} className="rounded-xl border border-zinc-200 px-4 py-2 text-sm text-zinc-800 hover:bg-zinc-50">Forget</button>
                <button type="button" disabled={props.memoryUi.lookup.pending} onClick={() => runAction(props.onCount)} className="rounded-xl border border-zinc-200 px-4 py-2 text-sm text-zinc-800 hover:bg-zinc-50">Count</button>
              </div>
              <StatusPanel success={props.memoryUi.lookup.successMessage} error={props.memoryUi.lookup.error} />
              {props.getResult && (
                <div className={subtlePanelClassName}>
                  <p className="font-medium">{props.getResult.key}</p>
                  <p className="mt-2 whitespace-pre-wrap text-zinc-600">{props.getResult.content}</p>
                </div>
              )}
              {props.count !== null && <Badge tone="info">count {props.count.toString()}</Badge>}
            </div>

            <div className={panelClassName}>
              <p className="text-xs uppercase tracking-[0.2em] text-zinc-500">List / Recall</p>
              <input value={listSessionId} onChange={(event) => setListSessionId(event.target.value)} placeholder="session_id for list/recall" className={inputClassName} />
              <div className="flex flex-wrap gap-2">
                {CATEGORY_OPTIONS.map((option) => (
                  <button key={option.label} type="button" disabled={props.memoryUi.query.pending} onClick={() => runAction(() => props.onList(option.value, listSessionId))} className="rounded-xl border border-zinc-200 px-3 py-2 text-sm text-zinc-800 hover:bg-zinc-50">
                    list {option.label}
                  </button>
                ))}
                <button type="button" disabled={props.memoryUi.query.pending} onClick={() => runAction(() => props.onList(undefined, listSessionId))} className="rounded-xl border border-zinc-200 px-3 py-2 text-sm text-zinc-800 hover:bg-zinc-50">
                  list all
                </button>
              </div>
              <div className="grid gap-3 md:grid-cols-[1fr_auto]">
                <input value={recallQuery} onChange={(event) => setRecallQuery(event.target.value)} placeholder="recall query" className={inputClassName} />
                <input value={recallLimit} onChange={(event) => setRecallLimit(event.target.value)} placeholder="10" className="w-24 rounded-xl border border-zinc-200 bg-zinc-50/80 px-4 py-3 text-sm text-zinc-900" />
              </div>
              <button type="button" disabled={props.memoryUi.query.pending} onClick={() => runAction(() => props.onRecall(recallQuery, BigInt(recallLimit || "10"), listSessionId))} className="rounded-xl bg-blue-600 px-4 py-2 text-sm text-white hover:bg-blue-500">
                {props.memoryUi.query.pending ? "Running..." : "Recall"}
              </button>
              <StatusPanel success={props.memoryUi.query.successMessage} error={props.memoryUi.query.error} />
            </div>
          </div>
        </Card>
      </div>

      <Card title="Results">
        <div className="space-y-3">
          {props.queryResult.length === 0 && <p className="text-sm text-zinc-500">query result はまだありません。</p>}
          {props.queryResult.map((item) => (
            <div key={item.id} className="rounded-2xl border border-zinc-200 bg-zinc-50/70 px-4 py-3">
              <div className="flex flex-wrap items-center gap-2">
                <Badge tone="info">{renderCategory(item.category)}</Badge>
                <span className="text-sm text-zinc-800">{item.key}</span>
              </div>
              <p className="mt-3 whitespace-pre-wrap text-sm text-zinc-600">{item.content}</p>
            </div>
          ))}
        </div>
      </Card>
    </div>
  );
}

function StatusPanel({ success, error }: { success: string | null; error: string | null }) {
  if (!success && !error) {
    return null;
  }
  return (
    <div className={`rounded-xl px-3 py-2 text-sm ${error ? "border border-rose-200 bg-rose-50 text-rose-700" : "border border-emerald-200 bg-emerald-50 text-emerald-700"}`}>
      {error ?? success}
    </div>
  );
}

function renderCategory(category: MemoryCategory): string {
  if ("core" in category) return "core";
  if ("daily" in category) return "daily";
  if ("conversation" in category) return "conversation";
  return `custom:${category.custom}`;
}
