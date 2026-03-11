// where: iclaw/web/playwright.config.ts
// what: Local-only Playwright defaults for Internet Identity E2E coverage
// why: These tests need deterministic browser artifacts and one Chromium worker while they drive a managed local replica

import { defineConfig, devices } from "@playwright/test";

export default defineConfig({
  testDir: "./e2e",
  fullyParallel: false,
  workers: 1,
  timeout: 300_000,
  expect: {
    timeout: 20_000,
  },
  reporter: [["list"]],
  use: {
    baseURL: process.env.ICLAW_E2E_GATEWAY_URL ?? "http://127.0.0.1:4943",
    screenshot: "only-on-failure",
    trace: "on-first-retry",
    video: "retain-on-failure",
  },
  projects: [
    {
      name: "chromium",
      use: {
        ...devices["Desktop Chrome"],
        channel: "chromium",
      },
    },
  ],
});
