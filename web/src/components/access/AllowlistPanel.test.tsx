import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, describe, expect, it, vi } from "vitest";
import { AllowlistPanel } from "@/components/access/AllowlistPanel";

describe("AllowlistPanel", () => {
  let container: HTMLDivElement;
  let root: Root;

  afterEach(() => {
    if (root) {
      act(() => root.unmount());
    }
    container?.remove();
  });

  it("blocks saving when the current caller is removed from the draft", async () => {
    const onSave = vi.fn(async () => undefined);

    container = document.createElement("div");
    document.body.appendChild(container);
    root = createRoot(container);

    await act(async () => {
      root.render(
        <AllowlistPanel
          principals={["bbbbb-bb"]}
          currentPrincipal="aaaaa-aa"
          pending={false}
          error={null}
          success={null}
          onRefresh={async () => undefined}
          onSave={onSave}
        />,
      );
    });

    const saveButton = container.querySelector('[data-tid="allowlist-save-button"]');
    if (!(saveButton instanceof HTMLButtonElement)) {
      throw new Error("allowlist controls were not rendered");
    }

    await act(async () => {
      saveButton.click();
    });

    expect(onSave).not.toHaveBeenCalled();
    expect(container.textContent).toContain("現在の principal は allowlist に残す必要があります。");
  });
});
