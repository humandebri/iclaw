// where: iclaw/web/e2e/helpers/icp.ts
// what: Node wrapper around the local icp + canister lifecycle script used by Playwright
// why: Browser tests should not embed shell details or temp project bookkeeping directly

import { execFileSync } from "node:child_process";
import { mkdtempSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";

const REPO_ROOT = resolve(import.meta.dirname, "../../..");
const SCRIPT_PATH = resolve(REPO_ROOT, "scripts/icp_ii_e2e.sh");

interface DeploymentState {
  baseUrl: string;
  canisterId: string;
  gatewayPort: string;
  gatewayUrl: string;
  iiCanisterId: string;
  projectRoot: string;
  providerConfigured: boolean;
}

let stateFilePath: string | null = null;

function runScript(args: string[]): string {
  return execFileSync("bash", [SCRIPT_PATH, ...args], {
    cwd: REPO_ROOT,
    encoding: "utf8",
    env: process.env,
    stdio: ["ignore", "pipe", "inherit"],
  }).trim();
}

function ensureStateFile(): string {
  if (!stateFilePath) {
    const directory = mkdtempSync(join(tmpdir(), "iclaw-ii-playwright-"));
    stateFilePath = join(directory, "state.json");
  }
  return stateFilePath;
}

export interface DeploymentInfo {
  baseUrl: string;
  canisterId: string;
  gatewayUrl: string;
  iiCanisterId: string;
  providerConfigured: boolean;
}

export function deployPlaceholderAllowlist(): DeploymentInfo {
  const output = runScript(["setup", ensureStateFile()]);
  const state = JSON.parse(output) as DeploymentState;
  return {
    baseUrl: state.baseUrl,
    canisterId: state.canisterId,
    gatewayUrl: state.gatewayUrl,
    iiCanisterId: state.iiCanisterId,
    providerConfigured: state.providerConfigured,
  };
}

export function allowPrincipal(principalText: string): void {
  runScript(["allow-principal", ensureStateFile(), principalText]);
}

export function teardownDeployment(): void {
  if (!stateFilePath) {
    return;
  }
  runScript(["teardown", stateFilePath]);
  stateFilePath = null;
}
