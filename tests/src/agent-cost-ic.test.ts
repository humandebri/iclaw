// where: iclaw/tests/src/agent-cost-ic.test.ts
// what: PocketIC integration test for representative operation cycle costs
// why: Keep one stable, repo-owned way to compare the main canister cost buckets without ad-hoc scripts

import { after, before, test } from 'node:test';
import assert from 'node:assert/strict';
import { PocketIcServer, type PocketIc } from '@dfinity/pic';
import { type _SERVICE, idlFactory } from './declarations.js';
import { providerConfig, setupCanister } from './helpers.js';

let server: PocketIcServer;

type CostSample = {
  name: string;
  cycles: bigint;
};

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

async function measureCost(
  name: string,
  config: typeof providerConfig | undefined,
  execute: (fixture: Awaited<ReturnType<typeof setupCanister>>) => Promise<void>,
): Promise<CostSample> {
  const fixture = await setupCanister(server, config);

  try {
    const before = await fixture.pic.getCyclesBalance(fixture.canisterId);
    await execute(fixture);
    const after = await fixture.pic.getCyclesBalance(fixture.canisterId);
    return {
      name,
      cycles: before - after,
    };
  } finally {
    await fixture.pic.tearDown();
  }
}

function logSamples(samples: CostSample[]) {
  console.info(
    JSON.stringify(
      {
        type: 'agent-cost-samples',
        samples: samples.map((sample) => ({
          name: sample.name,
          cycles: sample.cycles.toString(),
        })),
      },
      null,
      0,
    ),
  );
}

before(async () => {
  server = await PocketIcServer.start();
});

after(async () => {
  if (server) {
    await server.stop();
  }
});

test('representative canister operations keep the expected cost ordering', async () => {
  const samples: CostSample[] = [];

  samples.push(
    await measureCost('health(query)', undefined, async ({ actor }) => {
      await actor.health();
    }),
  );

  samples.push(
    await measureCost('memory_store', undefined, async ({ actor }) => {
      const stored = await actor.memory_store({
        key: 'cost/memory-store',
        content: 'hello',
        category: { conversation: null },
        session_id: ['cost-session'],
      });
      assert.ok('Ok' in stored);
    }),
  );

  samples.push(
    await measureCost('run_create(no_provider)', undefined, async ({ actor }) => {
      const created = await actor.run_create({
        agent_id: [],
        session_id: ['cost-no-provider'],
        prompt: 'hello',
        model: [],
        temperature: [],
      });
      assert.ok('Ok' in created);
      assert.equal(created.Ok.status, 'failed');
    }),
  );

  samples.push(
    await measureCost(
      'run_create(provider_success_1_outcall)',
      providerConfig,
      async ({ pic, canisterId }) => {
        const deferredActor = pic.createDeferredActor<_SERVICE>(idlFactory, canisterId);
        const executeRun = await deferredActor.run_create({
          agent_id: [],
          session_id: ['cost-provider-1'],
          prompt: 'hello once',
          model: [],
          temperature: [0.2],
        });

        const outcall = await waitForSinglePendingOutcall(pic);
        await pic.mockPendingHttpsOutcall({
          requestId: outcall.requestId,
          subnetId: outcall.subnetId,
          response: {
            type: 'success',
            statusCode: 200,
            headers: [['content-type', 'application/json']],
            body: new TextEncoder().encode(
              JSON.stringify({
                id: 'chatcmpl-cost-1',
                object: 'chat.completion',
                created: 1741569952,
                model: 'gpt-4.1-mini',
                choices: [
                  {
                    index: 0,
                    message: {
                      role: 'assistant',
                      content: 'single outcall ok',
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
        assert.equal(result.Ok.status, 'completed');
      },
    ),
  );

  samples.push(
    await measureCost(
      'run_create(provider_tool_loop_2_outcalls)',
      providerConfig,
      async ({ pic, canisterId, actor }) => {
        const deferredActor = pic.createDeferredActor<_SERVICE>(idlFactory, canisterId);
        const executeRun = await deferredActor.run_create({
          agent_id: [],
          session_id: ['cost-provider-2'],
          prompt: 'remember and answer',
          model: [],
          temperature: [0.2],
        });

        const firstOutcall = await waitForSinglePendingOutcall(pic);
        await pic.mockPendingHttpsOutcall({
          requestId: firstOutcall.requestId,
          subnetId: firstOutcall.subnetId,
          response: {
            type: 'success',
            statusCode: 200,
            headers: [['content-type', 'application/json']],
            body: new TextEncoder().encode(
              JSON.stringify({
                id: 'chatcmpl-cost-2a',
                object: 'chat.completion',
                created: 1741569952,
                model: 'gpt-4.1-mini',
                choices: [
                  {
                    index: 0,
                    message: {
                      role: 'assistant',
                      content: 'storing memory',
                      tool_calls: [
                        {
                          id: 'call_cost',
                          type: 'function',
                          function: {
                            name: 'memory_store',
                            arguments: JSON.stringify({
                              key: 'conversation/cost-provider-2/note',
                              content: 'stored',
                              session_id: 'cost-provider-2',
                            }),
                          },
                        },
                      ],
                    },
                    finish_reason: 'tool_calls',
                  },
                ],
              }),
            ),
          },
        });

        const secondOutcall = await waitForSinglePendingOutcall(pic);
        await pic.mockPendingHttpsOutcall({
          requestId: secondOutcall.requestId,
          subnetId: secondOutcall.subnetId,
          response: {
            type: 'success',
            statusCode: 200,
            headers: [['content-type', 'application/json']],
            body: new TextEncoder().encode(
              JSON.stringify({
                id: 'chatcmpl-cost-2b',
                object: 'chat.completion',
                created: 1741569953,
                model: 'gpt-4.1-mini',
                choices: [
                  {
                    index: 0,
                    message: {
                      role: 'assistant',
                      content: 'tool loop ok',
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
        assert.equal(result.Ok.status, 'completed');

        const recalled = await actor.memory_recall({
          query: 'stored',
          limit: 10n,
          session_id: ['cost-provider-2'],
        });
        assert.ok('Ok' in recalled);
        assert.equal(recalled.Ok.some((item) => item.content === 'stored'), true);
      },
    ),
  );

  logSamples(samples);

  const health = samples[0];
  const memoryStore = samples[1];
  const noProviderRun = samples[2];
  const oneOutcall = samples[3];
  const twoOutcalls = samples[4];

  assert.equal(health?.cycles, 0n);
  assert.ok(memoryStore && memoryStore.cycles > 0n);
  assert.ok(noProviderRun && noProviderRun.cycles > memoryStore.cycles);
  assert.ok(oneOutcall && oneOutcall.cycles > noProviderRun.cycles);
  assert.ok(twoOutcalls && twoOutcalls.cycles > oneOutcall.cycles);
});
