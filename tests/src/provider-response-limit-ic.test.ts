// where: iclaw/tests/src/provider-response-limit-ic.test.ts
// what: PocketIC integration tests for provider response byte limits
// why: keep the lowered max_response_bytes contract pinned to both the outcall request and the canister-visible failure mode

import { after, before, test } from 'node:test';
import assert from 'node:assert/strict';
import { PocketIcServer, type PocketIc } from '@dfinity/pic';
import { type _SERVICE, idlFactory, type RunCreateRequest } from './declarations.js';
import { providerConfig, setupCanister } from './helpers.js';

let server: PocketIcServer;

async function waitForSinglePendingOutcall(pic: PocketIc) {
  for (let attempt = 0; attempt < 5; attempt += 1) {
    await pic.tick(2);
    const pendingOutcalls = await pic.getPendingHttpsOutcalls();
    if (pendingOutcalls.length > 0) {
      assert.equal(pendingOutcalls.length, 1);
      return pendingOutcalls[0];
    }
  }
  assert.fail('pending HTTPS outcall was not queued in time');
}

before(async () => {
  server = await PocketIcServer.start();
});

after(async () => {
  await server.stop();
});

test('provider outcalls advertise max_response_bytes=32000 and fail on larger response bodies', async () => {
  const { pic, canisterId } = await setupCanister(server, providerConfig);

  try {
    const deferredActor = pic.createDeferredActor<_SERVICE>(idlFactory, canisterId);
    const request: RunCreateRequest = {
      agent_id: [],
      prompt: 'return too much data',
      session_id: ['provider-response-limit'],
      model: [],
      temperature: [0.2],
    };
    const executeRun = await deferredActor.run_create(request);

    const outcall = await waitForSinglePendingOutcall(pic);
    assert.equal(outcall.maxResponseBytes, 32_000);

    const oversizedContent = 'x'.repeat(32_001);
    await pic.mockPendingHttpsOutcall({
      requestId: outcall.requestId,
      subnetId: outcall.subnetId,
      response: {
        type: 'success',
        statusCode: 200,
        headers: [['content-type', 'application/json']],
        body: new TextEncoder().encode(
          JSON.stringify({
            id: 'chatcmpl-provider-oversized',
            object: 'chat.completion',
            created: 1741569952,
            model: 'gpt-4.1-mini',
            choices: [
              {
                index: 0,
                message: {
                  role: 'assistant',
                  content: oversizedContent,
                },
                finish_reason: 'stop',
              },
            ],
          }),
        ),
      },
    });

    const result = await executeRun();
    assert.ok('Ok' in result);
    assert.equal(result.Ok.status, 'failed');
    assert.match(
      result.Ok.error[0] ?? '',
      /Transformed http response exceeds limit: 32000/,
    );
  } finally {
    await pic.tearDown();
  }
});
