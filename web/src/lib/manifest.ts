// where: iclaw/web/src/lib/manifest.ts
// what: Browser-safe manifest parsing and preview helpers for seed ingestion
// why: The browser can only ingest content it already has, so file-path manifests need explicit upload reconciliation

import type { ManifestEntryDraft, ManifestPreviewEntry } from "@/types/ui";

function isObject(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}

function basename(input: string): string {
  return input.split(/[\\/]/).pop() ?? input;
}

export function parseManifest(raw: string): ManifestEntryDraft[] {
  const parsed = JSON.parse(raw) as unknown;
  if (!isObject(parsed) || !Array.isArray(parsed.entries)) {
    throw new Error("manifest must contain an entries array");
  }

  return parsed.entries.map((entry, index) => {
    if (!isObject(entry) || typeof entry.kind !== "string" || typeof entry.key !== "string") {
      throw new Error(`entries[${index}] must include string kind/key`);
    }

    if (entry.kind !== "core-entry" && entry.kind !== "workspace-file") {
      throw new Error(`entries[${index}] has unsupported kind '${entry.kind}'`);
    }

    const draft: ManifestEntryDraft = {
      kind: entry.kind,
      key: entry.key,
    };
    if (typeof entry.text === "string") {
      draft.text = entry.text;
    }
    if (typeof entry.file === "string") {
      draft.file = entry.file;
    }
    return draft;
  });
}

export function previewManifest(
  entries: ManifestEntryDraft[],
  filesByName: Map<string, File>,
): ManifestPreviewEntry[] {
  return entries.map((entry) => {
    if (entry.kind === "workspace-file") {
      if (!entry.key.startsWith("workspace/")) {
        return { entry, status: "invalid", detail: "workspace-file keys must start with workspace/" };
      }
      if (!entry.file) {
        return { entry, status: "invalid", detail: "workspace-file requires file" };
      }
      const matched = filesByName.get(entry.file) ?? filesByName.get(basename(entry.file));
      return matched
        ? { entry, status: "ready", detail: matched.name }
        : { entry, status: "missing-file", detail: `upload required for ${entry.file}` };
    }

    if (!entry.key.startsWith("core/")) {
      return { entry, status: "invalid", detail: "core-entry keys must start with core/" };
    }
    const hasText = typeof entry.text === "string";
    const hasFile = typeof entry.file === "string";
    if (hasText === hasFile) {
      return { entry, status: "invalid", detail: "core-entry needs exactly one of text/file" };
    }
    if (hasText) {
      return { entry, status: "ready", detail: "inline text" };
    }
    const matched = filesByName.get(entry.file!) ?? filesByName.get(basename(entry.file!));
    return matched
      ? { entry, status: "ready", detail: matched.name }
      : { entry, status: "missing-file", detail: `upload required for ${entry.file}` };
  });
}

export async function resolveManifestContent(
  entry: ManifestEntryDraft,
  filesByName: Map<string, File>,
): Promise<string> {
  if (typeof entry.text === "string") {
    return entry.text;
  }
  if (!entry.file) {
    throw new Error(`${entry.key}: file is required`);
  }
  const candidate = filesByName.get(entry.file) ?? filesByName.get(basename(entry.file));
  if (!candidate) {
    throw new Error(`${entry.key}: uploaded file for '${entry.file}' was not provided`);
  }
  return candidate.text();
}
