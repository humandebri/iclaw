// where: iclaw/web/src/lib/env.ts
// what: Browser environment helpers for deriving the current canister and II endpoints
// why: The single-canister UI has to bootstrap itself from the serving URL without extra config APIs

const MAINNET_II_URL = "https://identity.internetcomputer.org";
const DEFAULT_LOCAL_II_CANISTER_ID = "rdmx6-jaaaa-aaaaa-aaadq-cai";

function searchParam(name: string): string | null {
  return new URLSearchParams(window.location.search).get(name);
}

export function resolveCanisterId(): string {
  const queryCanisterId = searchParam("canisterId");
  if (queryCanisterId) {
    return queryCanisterId;
  }

  const host = window.location.hostname;
  if (host.endsWith(".localhost")) {
    return host.split(".")[0] ?? "";
  }

  const segments = host.split(".");
  if (segments.length > 3) {
    return segments[0] ?? "";
  }

  const saved = window.localStorage.getItem("iclaw.canister_id");
  if (saved) {
    return saved;
  }

  throw new Error("canisterId を URL から特定できません。?canisterId=... を付けて開いてください。");
}

export function resolveHost(): string {
  return window.location.origin;
}

export function isLocalReplica(): boolean {
  const host = window.location.hostname;
  return host === "127.0.0.1" || host === "localhost" || host.endsWith(".localhost");
}

export function resolveIdentityProvider(): string {
  const explicit = searchParam("ii");
  if (explicit) {
    return explicit;
  }

  if (!isLocalReplica()) {
    return MAINNET_II_URL;
  }

  const canisterId =
    window.localStorage.getItem("iclaw.ii_canister_id") ?? DEFAULT_LOCAL_II_CANISTER_ID;
  const port = window.location.port || "4943";
  return `http://${canisterId}.localhost:${port}`;
}
