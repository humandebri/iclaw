import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, describe, expect, it } from "vitest";
import { MemoryPage } from "@/pages/Memory";
import type { MemoryUiState } from "@/types/ui";

const idleMemoryState: MemoryUiState = {
  core: { pending: false, error: null, successMessage: null, details: { secretNotice: null } },
  manifest: { pending: false, error: "manifest failed", successMessage: null, details: { secretNotice: null } },
  advanced: { pending: false, error: null, successMessage: null, details: { secretNotice: null } },
  lookup: { pending: false, error: "lookup failed", successMessage: null, details: { secretNotice: null } },
  query: { pending: false, error: null, successMessage: null, details: { secretNotice: null } },
  manifestRuns: [{ key: "core/project_facts/runtime", status: "error", detail: "missing file" }],
};

describe("MemoryPage", () => {
  let container: HTMLDivElement;
  let root: Root;

  afterEach(() => {
    if (root) {
      act(() => root.unmount());
    }
    container?.remove();
  });

  it("shows section-level failures and manifest entry results", () => {
    container = document.createElement("div");
    document.body.appendChild(container);
    root = createRoot(container);

    act(() => {
      root.render(
        <MemoryPage
          queryResult={[]}
          getResult={null}
          count={null}
          manifestPreview={[]}
          manifestNote="note"
          memoryUi={idleMemoryState}
          onSaveCore={async () => undefined}
          onAdvancedStore={async () => undefined}
          onForget={async () => undefined}
          onLookup={async () => undefined}
          onList={async () => undefined}
          onRecall={async () => undefined}
          onCount={async () => undefined}
          onManifestTextChange={() => undefined}
          onManifestFiles={() => undefined}
          onRunManifest={async () => undefined}
        />,
      );
    });

    expect(container.textContent).toContain("manifest failed");
    expect(container.textContent).toContain("lookup failed");
    expect(container.textContent).toContain("missing file");
  });
});
