import { describe, expect, it } from "vitest";
import { normalizeError } from "@/lib/api";

describe("api error normalization", () => {
  it("keeps canister-style errors stable", () => {
    expect(normalizeError({ code: "memory_error", message: "boom" })).toEqual({
      code: "memory_error",
      message: "boom",
    });
  });

  it("preserves unauthorized errors for access denied flows", () => {
    expect(normalizeError({ code: "unauthorized", message: "blocked" })).toEqual({
      code: "unauthorized",
      message: "blocked",
    });
  });
});
