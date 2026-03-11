// where: iclaw/web/e2e/auth.spec.ts
// what: Real-browser Internet Identity coverage for denied, allowed, and session-switch operator flows
// why: The caller UI depends on II plus canister allowlists, so unit tests are not enough to validate the full boundary

import { expect, testWithII as test } from "@dfinity/internet-identity-playwright";
import { allowPrincipal, deployPlaceholderAllowlist, teardownDeployment, type DeploymentInfo } from "./helpers/icp";

test.describe.configure({ mode: "serial" });

let deployment: DeploymentInfo;
let allowedPrincipal = "";

test.beforeAll(() => {
  deployment = deployPlaceholderAllowlist();
});

test.afterAll(() => {
  teardownDeployment();
});

test.beforeEach(async ({ iiPage }) => {
  await iiPage.waitReady({
    url: deployment.gatewayUrl,
    canisterId: deployment.iiCanisterId,
    timeout: 120_000,
  });
});

test("operator auth journey covers denied, allowlist update, session switch, and second identity denial", async ({ page, iiPage }) => {
  await page.goto(deployment.baseUrl);
  const allowedIdentity = await iiPage.signInWithNewIdentity();

  await expect(page.getByRole("heading", { name: "Operator principal is required" })).toBeVisible();

  const principalText = await page.locator('[data-tid="access-denied-principal"]').textContent();
  const match = principalText?.match(/principal:\s*([a-z2-7-]+)/);
  if (!match) {
    throw new Error("denied principal text was not visible");
  }

  allowedPrincipal = match[1];
  await expect(page.locator('[data-tid="access-denied-principal"]')).toContainText(allowedPrincipal);
  allowPrincipal(allowedPrincipal);

  await page.goto(deployment.baseUrl);
  await iiPage.signInWithIdentity({ identity: allowedIdentity });

  await expect(page.getByRole("heading", { name: "Caller UX Dashboard" })).toBeVisible();
  await expect(page.locator('[data-tid="principal-badge"]')).toContainText(allowedPrincipal);

  if (deployment.providerConfigured) {
    await page.goto(`${deployment.baseUrl}#/chat`);
    const prompt = `iclaw-session-switch-${Date.now()}`;

    await page.locator('[data-tid="chat-session-input"]').fill("alpha");
    await page.locator('[data-tid="chat-prompt-input"]').fill(prompt);
    await page.locator('[data-tid="chat-send-button"]').click();

    await expect(page.locator('[data-tid="chat-message"][data-role="user"]')).toContainText(prompt);
    await expect(page.locator('[data-tid="chat-message"][data-role="assistant"]')).toHaveCount(1, {
      timeout: 120_000,
    });

    await page.locator('[data-tid="chat-session-input"]').fill("beta");
    await expect(page.locator('[data-tid="chat-empty-state"]')).toBeVisible();
    await expect(page.locator('[data-tid="chat-message"]')).toHaveCount(0);
    await expect(page.getByText("current session: beta")).toBeVisible();
  }

  await page.goto(deployment.baseUrl);
  await expect(page.getByRole("heading", { name: "Caller UX Dashboard" })).toBeVisible();
  await page.locator("#logout").click();

  await iiPage.signInWithNewIdentity();

  await expect(page.getByRole("heading", { name: "Operator principal is required" })).toBeVisible();
  await expect(page.locator('[data-tid="access-denied-principal"]')).not.toContainText(allowedPrincipal);
  await expect(page.getByText(/not allowed to use this canister/i)).toBeVisible();
});
