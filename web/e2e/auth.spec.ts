// where: iclaw/web/e2e/auth.spec.ts
// what: Real-browser Internet Identity coverage for denied, allowed, and session-switch operator flows
// why: The caller UI depends on II plus canister allowlists, so unit tests are not enough to validate the full boundary

import { expect, type BrowserContext, type Locator, type Page } from "@playwright/test";
import { testWithII as test } from "@dfinity/internet-identity-playwright";
import {
  deployPlaceholderAllowlist,
  teardownDeployment,
  upgradeAllowlist,
  type DeploymentInfo,
} from "./helpers/icp";

test.describe.configure({ mode: "serial" });

let deployment: DeploymentInfo;
let allowedPrincipal = "";

async function becomesVisible(locator: Locator, timeout = 2_000): Promise<boolean> {
  try {
    await locator.waitFor({ state: "visible", timeout });
    return true;
  } catch {
    return false;
  }
}

async function finishNewIdentityFlow(popup: Page): Promise<void> {
  if (await becomesVisible(popup.getByRole("button", { name: "Create Internet Identity", exact: true }))) {
    await popup.getByRole("button", { name: "Create Internet Identity", exact: true }).click();
  }

  if (await becomesVisible(popup.getByRole("button", { name: "I saved it, continue", exact: true }), 30_000)) {
    await popup.getByRole("button", { name: "I saved it, continue", exact: true }).click();
  }
}

async function finishExistingIdentityFlow(popup: Page): Promise<void> {
  if (await becomesVisible(popup.getByRole("button", { name: "Use existing", exact: true }))) {
    await popup.getByRole("button", { name: "Use existing", exact: true }).click();
  }

  if (await becomesVisible(popup.getByRole("button", { name: "More options …", exact: true }))) {
    await popup.getByRole("button", { name: "More options …", exact: true }).click();
  }

  const knownIdentity = popup.locator("button").filter({ hasText: /^\d+$/ }).first();
  if (await becomesVisible(knownIdentity)) {
    await knownIdentity.click();
    return;
  }

  const continueButton = popup.getByRole("button", { name: "Continue", exact: true });
  if (await becomesVisible(continueButton)) {
    await continueButton.click();
  }
}

async function signInWithCurrentIiState(appPage: Page, context: BrowserContext): Promise<void> {
  if (await becomesVisible(appPage.locator("#logout"), 2_000)) {
    return;
  }

  const popupPromise = context.waitForEvent("page");
  await appPage.locator("[data-tid=login-button]").click();
  const popup = await popupPromise;
  await popup.waitForLoadState("domcontentloaded");
  await expect(popup).toHaveTitle("Internet Identity");

  if (await becomesVisible(popup.getByRole("button", { name: "Create Internet Identity", exact: true }), 5_000)) {
    await finishNewIdentityFlow(popup);
  } else {
    await finishExistingIdentityFlow(popup);
  }

  await expect.poll(() => popup.isClosed(), { timeout: 30_000 }).toBe(true);
}

async function signInWithNewIdentity(appPage: Page, context: BrowserContext): Promise<void> {
  const popupPromise = context.waitForEvent("page");
  await appPage.locator("[data-tid=login-button]").click();
  const popup = await popupPromise;
  await popup.waitForLoadState("domcontentloaded");
  await expect(popup).toHaveTitle("Internet Identity");

  if (await becomesVisible(popup.getByRole("button", { name: "Create Internet Identity", exact: true }), 5_000)) {
    await finishNewIdentityFlow(popup);
  } else {
    if (await becomesVisible(popup.getByRole("button", { name: "More options …", exact: true }), 5_000)) {
      await popup.getByRole("button", { name: "More options …", exact: true }).click();
    }
    await popup.getByRole("button", { name: "Create New", exact: true }).click();
    await finishNewIdentityFlow(popup);
  }

  await expect.poll(() => popup.isClosed(), { timeout: 30_000 }).toBe(true);
}

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

test("operator auth journey covers denied, allowlist upgrade, session switch, and second identity denial", async ({ page, context }) => {
  await page.goto(deployment.baseUrl);
  await signInWithCurrentIiState(page, context);

  await expect(page.getByRole("heading", { name: "Operator principal is required" })).toBeVisible();

  const principalText = await page.locator('[data-tid="access-denied-principal"]').textContent();
  const match = principalText?.match(/principal:\s*([a-z2-7-]+)/);
  if (!match) {
    throw new Error("denied principal text was not visible");
  }

  allowedPrincipal = match[1];
  await expect(page.locator('[data-tid="access-denied-principal"]')).toContainText(allowedPrincipal);
  if (!allowedPrincipal) {
    throw new Error("allowed principal was not captured from the denied flow");
  }

  upgradeAllowlist(allowedPrincipal);

  await page.goto(deployment.baseUrl);
  await signInWithCurrentIiState(page, context);

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

  await signInWithNewIdentity(page, context);

  await expect(page.getByRole("heading", { name: "Operator principal is required" })).toBeVisible();
  await expect(page.locator('[data-tid="access-denied-principal"]')).not.toContainText(allowedPrincipal);
  await expect(page.getByText(/not allowed to use this canister/i)).toBeVisible();
});
