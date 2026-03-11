import { describe, expect, it } from "vitest";
import { parseManifest, previewManifest } from "@/lib/manifest";

describe("manifest parser", () => {
  it("rejects invalid root payloads", () => {
    expect(() => parseManifest("{}")).toThrow(/entries array/);
  });

  it("marks unresolved file entries as missing-file", () => {
    const entries = parseManifest(
      JSON.stringify({
        entries: [{ kind: "workspace-file", key: "workspace/AGENTS.md", file: "AGENTS.md" }],
      }),
    );
    const preview = previewManifest(entries, new Map());
    expect(preview[0]?.status).toBe("missing-file");
  });
});
