import { describe, expect, it } from "vitest";
import { sessionChanged } from "@/lib/chat-state";

describe("chat session state", () => {
  it("treats session switches as browser-log reset boundaries", () => {
    expect(sessionChanged("alpha", "beta")).toBe(true);
    expect(sessionChanged("alpha", "")).toBe(true);
    expect(sessionChanged("alpha", "alpha")).toBe(false);
  });
});
