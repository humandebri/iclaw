import { beforeEach, describe, expect, it } from "vitest";
import { currentSessionId, rememberSession, loadSessions } from "@/lib/session-store";

describe("session store", () => {
  beforeEach(() => {
    window.localStorage.clear();
  });

  it("restores the most recent session", () => {
    rememberSession("alpha");
    rememberSession("beta");
    expect(currentSessionId()).toBe("beta");
    expect(loadSessions()).toHaveLength(2);
  });
});
