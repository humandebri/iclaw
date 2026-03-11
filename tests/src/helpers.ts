// where: iclaw/tests/src/helpers.ts
// what: Shared PocketIC test setup helpers for iclaw
// why: Keep the integration tests concise and deterministic, and use gzip-compressed Wasm to stay below PocketIC's ingress size limit

import { IcpConfigFlag, PocketIc, PocketIcServer } from '@dfinity/pic';
import { IDL } from '@icp-sdk/core/candid';
import { Principal } from '@icp-sdk/core/principal';
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { candid, type CanisterConfig, type _SERVICE, idlFactory } from './declarations.js';

export const wasmPath = resolve(process.cwd(), '..', 'target', 'ic', 'iclaw.wasm.gz');
export const wasmBytes = new Uint8Array(readFileSync(wasmPath));

const anonymousPrincipal = Principal.fromText('2vxsx-fae');
const defaultOperatorConfig: CanisterConfig = {
  provider: [],
  context: [],
  allowed_principals: [[anonymousPrincipal]],
};

export const providerConfig: CanisterConfig = {
  provider: [
    {
      api_url: 'https://api.openai.com/v1',
      api_key: 'openai-test-key',
      default_model: 'gpt-4o-mini',
      timeout_secs: [30n],
    },
  ],
  context: [],
  allowed_principals: [[anonymousPrincipal]],
};

export function encodeInitArg(value?: CanisterConfig): Uint8Array {
  return IDL.encode([IDL.Opt(candid.canisterConfig)], [value === undefined ? [defaultOperatorConfig] : [value]]);
}

export function encodeEmptyArgs(): Uint8Array {
  return IDL.encode([], []);
}

export function encodeNoConfigInitArg(): Uint8Array {
  return encodeInitArg(defaultOperatorConfig);
}

export async function setupCanister(
  server: PocketIcServer,
  config?: CanisterConfig,
): Promise<{
  pic: PocketIc;
  actor: _SERVICE;
  canisterId: Principal;
}> {
  const pic = await PocketIc.create(server.getUrl(), {
    icpConfig: {
      canisterExecutionRateLimiting: IcpConfigFlag.Disabled,
    },
  });
  const fixture = await pic.setupCanister<_SERVICE>({
    idlFactory,
    wasm: wasmBytes,
    arg: encodeInitArg(config),
  });

  return {
    pic,
    actor: fixture.actor,
    canisterId: fixture.canisterId,
  };
}
