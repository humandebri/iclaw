// where: iclaw/tests/src/iclaw-ic.test.ts
// what: PocketIC integration tests for iclaw_ic public canister API
// why: Validate query/update behavior before mainnet verification

import { after, before, test } from 'node:test';
import assert from 'node:assert/strict';
import { PocketIcServer } from '@dfinity/pic';
import { Principal } from '@icp-sdk/core/principal';
import { idlFactory, type _SERVICE, type CanisterConfig, type RunCreateRequest } from './declarations.js';
import { encodeEmptyArgs, encodeInitArg, encodeNoConfigInitArg, providerConfig, readWasmBytes, setupCanister } from './helpers.js';

let server: PocketIcServer;
const textDecoder = new TextDecoder();

async function waitForPendingOutcall(pic: Awaited<ReturnType<typeof setupCanister>>['pic']) {
  for (let attempt = 0; attempt < 5; attempt += 1) {
    await pic.tick(2);
    const pendingOutcalls = await pic.getPendingHttpsOutcalls();
    if (pendingOutcalls.length > 0) {
      return pendingOutcalls;
    }
  }
  return [];
}

before(async () => {
  server = await PocketIcServer.start();
});

after(async () => {
  await server.stop();
});

test('health query reflects unconfigured provider state', async () => {
  const { pic, actor, canisterId } = await setupCanister(server);

  try {
    await pic.queryCall({
      canisterId,
      method: 'health',
      arg: encodeEmptyArgs(),
    });
    const response = await actor.health();

    assert.equal(response.status, 'degraded');
    assert.equal(response.provider_ready, false);
    assert.equal(response.memory_ready, true);
    assert.equal(response.runtime, 'icp-canister');
  } finally {
    await pic.tearDown();
  }
});

test('health query reports configured provider as ready', async () => {
  const { pic, actor } = await setupCanister(server, providerConfig);

  try {
    const response = await actor.health();
    assert.equal(response.provider_ready, true);
    assert.equal(response.memory_ready, true);
    assert.equal(response.status, 'ok');
  } finally {
    await pic.tearDown();
  }
});

test('memory APIs round-trip through PocketIC', async () => {
  const { pic, actor } = await setupCanister(server);
  const key = 'pocketic-memory-entry';

  try {
    const store = await actor.memory_store({
      key,
      content: 'hello pocketic',
      category: { conversation: null },
      session_id: ['session-a'],
    });
    assert.ok('Ok' in store);

    const fetched = await actor.memory_get({ key });
    assert.ok('Ok' in fetched);
    assert.equal(fetched.Ok[0]?.content, 'hello pocketic');

    const listed = await actor.memory_list({
      category: [{ conversation: null }],
      session_id: ['session-a'],
    });
    assert.ok('Ok' in listed);
    assert.equal(listed.Ok.length, 1);

    const recalled = await actor.memory_recall({
      query: 'hello',
      limit: 10n,
      session_id: ['session-a'],
    });
    assert.ok('Ok' in recalled);
    assert.equal(recalled.Ok.length, 1);

    const counted = await actor.memory_count();
    assert.ok('Ok' in counted);
    assert.equal(counted.Ok >= 1n, true);

    const forgotten = await actor.memory_forget({ key });
    assert.ok('Ok' in forgotten);
    assert.equal(forgotten.Ok, true);
  } finally {
    await pic.tearDown();
  }
});

test('memory survives canister upgrade on PocketIC', async () => {
  const { pic, actor, canisterId } = await setupCanister(server);
  const key = 'upgrade-persistent-entry';

  try {
    const stored = await actor.memory_store({
      key,
      content: 'persists across upgrade',
      category: { core: null },
      session_id: ['upgrade-session'],
    });
    assert.ok('Ok' in stored);

    // PocketIC models install_code rate limiting. Move time forward before the
    // upgrade so the test covers stable-memory persistence instead of local
    // scheduler throttling.
    await pic.advanceTime(5 * 60 * 1_000);
    await pic.tick(2);

    await pic.upgradeCanister({
      canisterId,
      wasm: readWasmBytes(),
      arg: encodeNoConfigInitArg(),
    });

    const upgradedActor = pic.createActor<_SERVICE>(idlFactory, canisterId);
    const fetched = await upgradedActor.memory_get({ key });
    assert.ok('Ok' in fetched);
    assert.equal(fetched.Ok[0]?.content, 'persists across upgrade');

    const counted = await upgradedActor.memory_count();
    assert.ok('Ok' in counted);
    assert.equal(counted.Ok >= 1n, true);
  } finally {
    await pic.tearDown();
  }
});

test('run_create auto-creates a session and exposes the failed run through run queries', async () => {
  const { pic, actor } = await setupCanister(server);

  try {
    const created = await actor.run_create({
      agent_id: [],
      session_id: [],
      prompt: 'phase1 run test',
      model: [],
      temperature: [],
    });
    assert.ok('Ok' in created);
    assert.equal(created.Ok.status, 'failed');
    assert.equal(created.Ok.session_id.length > 0, true);
    assert.equal(created.Ok.error[0]?.includes('not configured'), true);

    const fetchedSession = await actor.session_get({ session_id: created.Ok.session_id });
    assert.ok('Ok' in fetchedSession);
    assert.equal(fetchedSession.Ok[0]?.id, created.Ok.session_id);

    const listedSessions = await actor.sessions_list([]);
    assert.ok('Ok' in listedSessions);
    assert.equal(listedSessions.Ok.some((session) => session.id === created.Ok.session_id), true);

    const listedRuns = await actor.run_list({ session_id: [created.Ok.session_id], limit: [10n] });
    assert.ok('Ok' in listedRuns);
    assert.equal(listedRuns.Ok.length, 1);
    assert.equal(listedRuns.Ok[0]?.id, created.Ok.id);

    const events = await actor.run_events_get({ run_id: created.Ok.id });
    assert.ok('Ok' in events);
    assert.deepEqual(
      events.Ok.map((event) => event.kind),
      ['queued', 'started', 'failed'],
    );
  } finally {
    await pic.tearDown();
  }
});

test('run state survives canister upgrade on PocketIC', async () => {
  const { pic, actor, canisterId } = await setupCanister(server);

  try {
    const created = await actor.run_create({
      agent_id: [],
      session_id: ['upgrade-run-session'],
      prompt: 'persist this failed run',
      model: [],
      temperature: [],
    });
    assert.ok('Ok' in created);

    await pic.advanceTime(5 * 60 * 1_000);
    await pic.tick(2);
    await pic.upgradeCanister({
      canisterId,
      wasm: readWasmBytes(),
      arg: encodeNoConfigInitArg(),
    });

    const upgradedActor = pic.createActor<_SERVICE>(idlFactory, canisterId);
    const fetched = await upgradedActor.run_get({ run_id: created.Ok.id });
    assert.ok('Ok' in fetched);
    assert.equal(fetched.Ok[0]?.session_id, 'upgrade-run-session');

    const listed = await upgradedActor.run_list({ session_id: ['upgrade-run-session'], limit: [10n] });
    assert.ok('Ok' in listed);
    assert.equal(listed.Ok.length, 1);

    const session = await upgradedActor.session_get({ session_id: 'upgrade-run-session' });
    assert.ok('Ok' in session);
    assert.equal(session.Ok[0]?.id, 'upgrade-run-session');
  } finally {
    await pic.tearDown();
  }
});

test('run_cancel records cancelled state and event without interrupt semantics', async () => {
  const { pic, actor } = await setupCanister(server);

  try {
    const created = await actor.run_create({
      agent_id: [],
      session_id: ['cancel-session'],
      prompt: 'cancel this failed run',
      model: [],
      temperature: [],
    });
    assert.ok('Ok' in created);
    assert.equal(created.Ok.status, 'failed');

    const cancelled = await actor.run_cancel({ run_id: created.Ok.id });
    assert.ok('Ok' in cancelled);
    assert.equal(cancelled.Ok, false);

    const events = await actor.run_events_get({ run_id: created.Ok.id });
    assert.ok('Ok' in events);
    assert.equal(events.Ok.some((event) => event.kind === 'cancelled'), false);
  } finally {
    await pic.tearDown();
  }
});

test('schedule APIs round-trip and manual trigger creates schedule-tagged runs', async () => {
  const { pic, actor } = await setupCanister(server);

  try {
    const created = await actor.schedule_create({
      draft: {
        id: 'hourly-brief',
        name: 'Hourly Brief',
        agent_id: 'default',
        prompt: 'summarize the workspace',
        interval_minutes: 60n,
        session_mode: 'create_new',
        fixed_session_id: [],
        enabled: true,
      },
    });
    assert.ok('Ok' in created);
    assert.equal(created.Ok.next_run_at.length, 1);
    assert.equal(created.Ok.consecutive_failure_count, 0n);
    assert.equal(created.Ok.last_success_at.length, 0);

    const listed = await actor.schedules_list();
    assert.ok('Ok' in listed);
    assert.equal(listed.Ok.some((schedule) => schedule.id === 'hourly-brief'), true);

    const triggered = await actor.schedule_trigger({ schedule_id: 'hourly-brief' });
    assert.ok('Ok' in triggered);
    assert.equal(triggered.Ok.trigger_kind, 'schedule');
    assert.equal(triggered.Ok.trigger_id[0], 'hourly-brief');

    const fetched = await actor.schedule_get({ schedule_id: 'hourly-brief' });
    assert.ok('Ok' in fetched);
    assert.equal(fetched.Ok[0]?.last_run_id[0], triggered.Ok.id);
    assert.equal(typeof fetched.Ok[0]?.last_finished_at[0], 'string');
    assert.equal(fetched.Ok[0]?.consecutive_failure_count, 1n);
    assert.equal(fetched.Ok[0]?.last_success_at.length, 0);
  } finally {
    await pic.tearDown();
  }
});

test('schedule update preserves cadence for metadata changes and manual trigger works while disabled', async () => {
  const { pic, actor } = await setupCanister(server);

  try {
    const created = await actor.schedule_create({
      draft: {
        id: 'stable-cadence',
        name: 'Stable Cadence',
        agent_id: 'default',
        prompt: 'summarize the workspace',
        interval_minutes: 60n,
        session_mode: 'create_new',
        fixed_session_id: [],
        enabled: true,
      },
    });
    assert.ok('Ok' in created);
    const fetchedBeforeUpdate = await actor.schedule_get({ schedule_id: 'stable-cadence' });
    assert.ok('Ok' in fetchedBeforeUpdate);
    const originalNextRunAt = fetchedBeforeUpdate.Ok[0]?.next_run_at[0];
    assert.equal(typeof originalNextRunAt, 'string');

    const metadataOnly = await actor.schedule_update({
      schedule: {
        ...created.Ok,
        name: 'Stable Cadence Renamed',
      },
    });
    assert.ok('Ok' in metadataOnly);
    assert.equal(metadataOnly.Ok.next_run_at[0], originalNextRunAt);

    const intervalChanged = await actor.schedule_update({
      schedule: {
        ...metadataOnly.Ok,
        interval_minutes: 30n,
      },
    });
    assert.ok('Ok' in intervalChanged);
    assert.notEqual(intervalChanged.Ok.next_run_at[0], originalNextRunAt);

    const disabled = await actor.schedule_update({
      schedule: {
        ...intervalChanged.Ok,
        enabled: false,
      },
    });
    assert.ok('Ok' in disabled);
    assert.equal(disabled.Ok.next_run_at.length, 0);

    const triggered = await actor.schedule_trigger({ schedule_id: 'stable-cadence' });
    assert.ok('Ok' in triggered);
    assert.equal(triggered.Ok.trigger_kind, 'schedule');
    assert.equal(triggered.Ok.trigger_id[0], 'stable-cadence');

    const fetchedAfterTrigger = await actor.schedule_get({ schedule_id: 'stable-cadence' });
    assert.ok('Ok' in fetchedAfterTrigger);
    assert.equal(fetchedAfterTrigger.Ok[0]?.consecutive_failure_count, 1n);
    assert.equal(fetchedAfterTrigger.Ok[0]?.last_success_at.length, 0);
  } finally {
    await pic.tearDown();
  }
});

test('disabled schedule manual trigger records last_success_at on provider success', async () => {
  const { pic, actor, canisterId } = await setupCanister(server, providerConfig);

  try {
    const created = await actor.schedule_create({
      draft: {
        id: 'disabled-success',
        name: 'Disabled Success',
        agent_id: 'default',
        prompt: 'summarize the workspace',
        interval_minutes: 60n,
        session_mode: 'create_new',
        fixed_session_id: [],
        enabled: false,
      },
    });
    assert.ok('Ok' in created);
    assert.equal(created.Ok.last_success_at.length, 0);

    const deferredActor = pic.createDeferredActor<_SERVICE>(idlFactory, canisterId);
    const executeTrigger = await deferredActor.schedule_trigger({ schedule_id: 'disabled-success' });

    const pendingOutcalls = await waitForPendingOutcall(pic);
    assert.equal(pendingOutcalls.length, 1);
    const pendingOutcall = pendingOutcalls[0];
    assert.ok(pendingOutcall);

    await pic.mockPendingHttpsOutcall({
      requestId: pendingOutcall.requestId,
      subnetId: pendingOutcall.subnetId,
      response: {
        type: 'success',
        statusCode: 200,
        headers: [['content-type', 'application/json']],
        body: new TextEncoder().encode(
          JSON.stringify({
            id: 'chatcmpl-schedule-success',
            object: 'chat.completion',
            created: 1741569952,
            model: 'gpt-4.1-mini',
            choices: [
              {
                index: 0,
                message: {
                  role: 'assistant',
                  content: 'scheduled success',
                },
                finish_reason: 'stop',
              },
            ],
          }),
        ),
      },
    });

    const triggered = await executeTrigger();
    assert.ok('Ok' in triggered);
    assert.equal(triggered.Ok.status, 'completed');

    const fetched = await actor.schedule_get({ schedule_id: 'disabled-success' });
    assert.ok('Ok' in fetched);
    assert.equal(fetched.Ok[0]?.consecutive_failure_count, 0n);
    assert.equal(typeof fetched.Ok[0]?.last_success_at[0], 'string');
  } finally {
    await pic.tearDown();
  }
});

test('schedule state survives canister upgrade and disable clears next_run_at', async () => {
  const { pic, actor, canisterId } = await setupCanister(server);

  try {
    const created = await actor.schedule_create({
      draft: {
        id: 'upgrade-schedule',
        name: 'Upgrade Schedule',
        agent_id: 'default',
        prompt: 'run after upgrade',
        interval_minutes: 15n,
        session_mode: 'create_new',
        fixed_session_id: [],
        enabled: true,
      },
    });
    assert.ok('Ok' in created);

    await pic.advanceTime(5 * 60 * 1_000);
    await pic.tick(2);
    await pic.upgradeCanister({
      canisterId,
      wasm: readWasmBytes(),
      arg: encodeNoConfigInitArg(),
    });

    const upgradedActor = pic.createActor<_SERVICE>(idlFactory, canisterId);
    const fetched = await upgradedActor.schedule_get({ schedule_id: 'upgrade-schedule' });
    assert.ok('Ok' in fetched);
    assert.equal(fetched.Ok[0]?.id, 'upgrade-schedule');
    assert.equal(fetched.Ok[0]?.enabled, true);

    const disabled = await upgradedActor.schedule_update({
      schedule: {
        ...fetched.Ok[0]!,
        enabled: false,
      },
    });
    assert.ok('Ok' in disabled);
    assert.equal(disabled.Ok.next_run_at.length, 0);
  } finally {
    await pic.tearDown();
  }
});

test('agent and tool policy APIs round-trip through PocketIC', async () => {
  const { pic, actor } = await setupCanister(server);

  try {
    const createdAgent = await actor.agent_create({
      draft: {
        id: 'writer',
        name: 'Writer Agent',
        description: 'Stores and recalls memory',
        enabled_tool_names: ['memory_store', 'memory_recall'],
        requires_tool_approval: false,
        system_prompt_override: [],
        status: 'active',
      },
    });
    assert.ok('Ok' in createdAgent);
    assert.equal(createdAgent.Ok.id, 'writer');

    const fetchedAgent = await actor.agent_get({ agent_id: 'writer' });
    assert.ok('Ok' in fetchedAgent);
    assert.equal(fetchedAgent.Ok[0]?.name, 'Writer Agent');

    const updatedPolicy = await actor.tool_policy_update({
      policy: {
        agent_id: 'writer',
        tool_name: 'memory_store',
        enabled: true,
        requires_approval: false,
      },
    });
    assert.ok('Ok' in updatedPolicy);
    assert.equal(updatedPolicy.Ok.tool_name, 'memory_store');

    const listedAgents = await actor.agents_list();
    assert.ok('Ok' in listedAgents);
    assert.equal(listedAgents.Ok.some((agent) => agent.id === 'writer'), true);

    const listedPolicies = await actor.tool_policy_list({ agent_id: ['writer'] });
    assert.ok('Ok' in listedPolicies);
    assert.equal(listedPolicies.Ok.some((policy) => policy.tool_name === 'memory_store'), true);
  } finally {
    await pic.tearDown();
  }
});

test('run_create returns blocked when tool policy requires approval', async () => {
  const { pic, actor, canisterId } = await setupCanister(server, providerConfig);

  try {
    const createdAgent = await actor.agent_create({
      draft: {
        id: 'guarded',
        name: 'Guarded Agent',
        description: 'Requires approval before tool use',
        enabled_tool_names: ['memory_store'],
        requires_tool_approval: false,
        system_prompt_override: [],
        status: 'active',
      },
    });
    assert.ok('Ok' in createdAgent);
    assert.equal(createdAgent.Ok.id, 'guarded');

    const updatedPolicy = await actor.tool_policy_update({
      policy: {
        agent_id: 'guarded',
        tool_name: 'memory_store',
        enabled: true,
        requires_approval: true,
      },
    });
    assert.ok('Ok' in updatedPolicy);

    const deferredActor = pic.createDeferredActor<_SERVICE>(idlFactory, canisterId);
    const request: RunCreateRequest = {
      agent_id: ['guarded'],
      prompt: 'store this after approval',
      session_id: ['guarded-session'],
      model: [],
      temperature: [0.2],
    };
    const executeRun = await deferredActor.run_create(request);

    await pic.tick(2);

    const pendingOutcalls = await pic.getPendingHttpsOutcalls();
    assert.equal(pendingOutcalls.length, 1);
    const firstOutcall = pendingOutcalls[0];
    assert.ok(firstOutcall);

    await pic.mockPendingHttpsOutcall({
      requestId: firstOutcall.requestId,
      subnetId: firstOutcall.subnetId,
      response: {
        type: 'success',
        statusCode: 200,
        headers: [['content-type', 'application/json']],
        body: new TextEncoder().encode(
          JSON.stringify({
            id: 'chatcmpl-pocketic-guarded',
            object: 'chat.completion',
            created: 1741569952,
            model: 'gpt-4.1-mini',
            choices: [
              {
                index: 0,
                message: {
                  role: 'assistant',
                  content: 'let me store that',
                  tool_calls: [
                    {
                      id: 'call_guarded',
                      type: 'function',
                      function: {
                        name: 'memory_store',
                        arguments: '{"key":"note/guarded","content":"blocked","session_id":"guarded-session"}',
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

    const result = await executeRun();
    assert.ok('Ok' in result);
    assert.equal(result.Ok.status, 'blocked');
    assert.match(result.Ok.error[0] ?? '', /requires approval/);
    assert.equal(result.Ok.pending_tool_calls.length, 1);
    assert.equal(result.Ok.pending_assistant_text[0] ?? '', 'let me store that');

    const events = await actor.run_events_get({ run_id: result.Ok.id });
    assert.ok('Ok' in events);
    assert.deepEqual(
      events.Ok.map((event) => event.kind),
      ['queued', 'started', 'tool_requested', 'tool_blocked', 'blocked'],
    );

    const executeResume = await deferredActor.run_resume({ run_id: result.Ok.id });
    await pic.tick(2);
    const resumeOutcalls = await pic.getPendingHttpsOutcalls();
    assert.equal(resumeOutcalls.length, 1);
    const resumeOutcall = resumeOutcalls[0];
    assert.ok(resumeOutcall);

    await pic.mockPendingHttpsOutcall({
      requestId: resumeOutcall.requestId,
      subnetId: resumeOutcall.subnetId,
      response: {
        type: 'success',
        statusCode: 200,
        headers: [['content-type', 'application/json']],
        body: new TextEncoder().encode(
          JSON.stringify({
            id: 'chatcmpl-pocketic-guarded-resume',
            object: 'chat.completion',
            created: 1741569953,
            model: 'gpt-4.1-mini',
            choices: [
              {
                index: 0,
                message: {
                  role: 'assistant',
                  content: 'stored after approval',
                },
                finish_reason: 'stop',
              },
            ],
          }),
        ),
      },
    });

    const resumed = await executeResume();
    assert.ok('Ok' in resumed);
    assert.equal(resumed.Ok.id, result.Ok.id);
    assert.equal(resumed.Ok.status, 'completed');
    assert.equal(resumed.Ok.response[0] ?? '', 'stored after approval');

    const resumedEvents = await actor.run_events_get({ run_id: result.Ok.id });
    assert.ok('Ok' in resumedEvents);
    assert.deepEqual(
      resumedEvents.Ok.map((event) => event.kind),
      ['queued', 'started', 'tool_requested', 'tool_blocked', 'blocked', 'approved', 'resumed', 'tool_succeeded', 'assistant_message', 'completed'],
    );
  } finally {
    await pic.tearDown();
  }
});

test('webhook secret rotate invalidates the old secret and tracks invocation metadata', async () => {
  const { pic, actor, canisterId } = await setupCanister(server, providerConfig);

  try {
    const created = await actor.webhook_create({
      draft: {
        id: 'rotate-me',
        name: 'Rotate Me',
        agent_id: 'default',
        session_mode: 'create_new',
        fixed_session_id: [],
        secret: 'secret-1',
        enabled: true,
      },
    });
    assert.ok('Ok' in created);

    const rotated = await actor.webhook_rotate_secret({ webhook_id: 'rotate-me' });
    assert.ok('Ok' in rotated);
    assert.notEqual(rotated.Ok.new_secret, 'secret-1');
    assert.equal(rotated.Ok.webhook.secret.includes('secret-1'), false);
    assert.equal(rotated.Ok.webhook.last_secret_rotated_at.length, 1);

    const rejected = await actor.webhook_invoke({
      webhook_id: 'rotate-me',
      secret: 'secret-1',
      prompt: 'old secret should fail',
      session_id: [],
      model: [],
      temperature: [],
    });
    assert.ok('Err' in rejected);
    assert.equal(rejected.Err.code, 'unauthorized');

    const deferredActor = pic.createDeferredActor<_SERVICE>(idlFactory, canisterId);
    const executeInvoke = await deferredActor.webhook_invoke({
      webhook_id: 'rotate-me',
      secret: rotated.Ok.new_secret,
      prompt: 'new secret should work',
      session_id: [],
      model: [],
      temperature: [],
    });

    await pic.tick(2);
    const pendingOutcalls = await pic.getPendingHttpsOutcalls();
    assert.equal(pendingOutcalls.length, 1);
    const pendingOutcall = pendingOutcalls[0];
    assert.ok(pendingOutcall);

    await pic.mockPendingHttpsOutcall({
      requestId: pendingOutcall.requestId,
      subnetId: pendingOutcall.subnetId,
      response: {
        type: 'success',
        statusCode: 200,
        headers: [['content-type', 'application/json']],
        body: new TextEncoder().encode(
          JSON.stringify({
            id: 'chatcmpl-pocketic-webhook-rotate',
            object: 'chat.completion',
            created: 1741569954,
            model: 'gpt-4.1-mini',
            choices: [
              {
                index: 0,
                message: {
                  role: 'assistant',
                  content: 'webhook ok',
                },
                finish_reason: 'stop',
              },
            ],
          }),
        ),
      },
    });

    const invoked = await executeInvoke();
    assert.ok('Ok' in invoked);
    assert.equal(invoked.Ok.status, 'completed');

    const fetched = await actor.webhook_get({ webhook_id: 'rotate-me' });
    assert.ok('Ok' in fetched);
    assert.equal(fetched.Ok[0]?.last_invoked_at.length, 1);
    assert.equal(fetched.Ok[0]?.last_secret_rotated_at.length, 1);
  } finally {
    await pic.tearDown();
  }
});

test('run_create rejects mismatched agent_id for an existing session', async () => {
  const { pic, actor } = await setupCanister(server);

  try {
    const createdAgent = await actor.agent_create({
      draft: {
        id: 'writer',
        name: 'Writer Agent',
        description: 'Stores and recalls memory',
        enabled_tool_names: ['memory_store'],
        requires_tool_approval: false,
        system_prompt_override: [],
        status: 'active',
      },
    });
    assert.ok('Ok' in createdAgent);

    const firstRun = await actor.run_create({
      agent_id: ['writer'],
      session_id: ['writer-session'],
      prompt: 'first writer run',
      model: [],
      temperature: [],
    });
    assert.ok('Ok' in firstRun);

    const secondRun = await actor.run_create({
      agent_id: ['default'],
      session_id: ['writer-session'],
      prompt: 'mismatched agent run',
      model: [],
      temperature: [],
    });
    assert.ok('Err' in secondRun);
    assert.equal(secondRun.Err.code, 'invalid_argument');
  } finally {
    await pic.tearDown();
  }
});

test('allowed_principals APIs update, survive upgrade, and unblock run authorization', async () => {
  const { pic, actor, canisterId } = await setupCanister(server);

  try {
    const initial = await actor.allowed_principals_get();
    assert.ok('Ok' in initial);
    assert.equal(initial.Ok.allowed_principals.length, 1);

    const updated = await actor.allowed_principals_set({
      allowed_principals: [Principal.fromText('2vxsx-fae'), Principal.managementCanister()],
    });
    assert.ok('Ok' in updated);
    assert.equal(updated.Ok.allowed_principals.length, 2);

    const chatResult = await actor.run_create({
      agent_id: [],
      prompt: 'hello from allowlist test',
      session_id: [],
      model: [],
      temperature: [],
    });
    assert.ok('Ok' in chatResult);
    assert.equal(chatResult.Ok.status, 'failed');
    assert.equal(chatResult.Ok.error[0]?.includes('not configured'), true);

    await pic.advanceTime(5 * 60 * 1_000);
    await pic.tick(2);
    await pic.upgradeCanister({
      canisterId,
      wasm: readWasmBytes(),
      arg: encodeNoConfigInitArg(),
    });

    const upgradedActor = pic.createActor<_SERVICE>(idlFactory, canisterId);
    const fetched = await upgradedActor.allowed_principals_get();
    assert.ok('Ok' in fetched);
    assert.equal(fetched.Ok.allowed_principals.length, 2);
    assert.equal(
      fetched.Ok.allowed_principals.some((principal) => principal.toText() === Principal.managementCanister().toText()),
      true,
    );
  } finally {
    await pic.tearDown();
  }
});

test('context config survives upgrade and affects the provider request body', async () => {
  const config: CanisterConfig = {
    ...providerConfig,
    context: [
      {
        workspace_files: [['AGENTS.md', 'MEMORY.md']],
        skills_dir: ['skills'],
        history_limit: [12n],
        max_tool_iterations: [3n],
        enable_tool_loop: [true],
        enable_auto_promote: [true],
        enable_conversation_summary: [true],
        max_static_context_chars: [512n],
        max_skill_context_chars: [512n],
        summary_max_chars: [240n],
        retry_provider_once: [true],
        memory_recall_limit: [5n],
        enable_autosave: [true],
        max_prompt_chars: [],
        max_request_bytes_budget: [],
        llm_summary_on_overflow: [],
        llm_summary_model: [],
        llm_summary_max_chars: [],
        llm_summary_request_bytes_threshold: [],
        cycle_balance_warning_threshold: [],
        memory_min_score: [0.5],
        enable_lightweight_skill_actions: [true],
      },
    ],
    allowed_principals: providerConfig.allowed_principals,
  };
  const { pic, actor, canisterId } = await setupCanister(server, config);

  try {
    assert.ok('Ok' in (await actor.memory_store({
      key: 'workspace/AGENTS.md',
      content: 'Always explain the flow.',
      category: { core: null },
      session_id: [],
    })));
    assert.ok('Ok' in (await actor.memory_store({
      key: 'workspace/MEMORY.md',
      content: 'Gemini smoke already passed.',
      category: { core: null },
      session_id: [],
    })));
    assert.ok('Ok' in (await actor.memory_store({
      key: 'workspace/skills/doc/SKILL.md',
      content: '# doc\nExplain docs clearly.\n\nUse memory_recall first.\nActions: workspace_lookup, skill_summary',
      category: { core: null },
      session_id: [],
    })));
    assert.ok('Ok' in (await actor.memory_store({
      key: 'conversation/relevant',
      content: 'Use AGENTS.md and the doc skill memory',
      category: { conversation: null },
      session_id: ['session-context'],
    })));

    await pic.advanceTime(5 * 60 * 1_000);
    await pic.tick(2);
    await pic.upgradeCanister({
      canisterId,
      wasm: readWasmBytes(),
      arg: encodeInitArg(config),
    });

    const upgradedActor = pic.createActor<_SERVICE>(idlFactory, canisterId);
    const deferredActor = pic.createDeferredActor<_SERVICE>(idlFactory, canisterId);
    const request: RunCreateRequest = {
      agent_id: [],
      prompt: 'Use AGENTS.md and the doc skill memory',
      session_id: ['session-context'],
      model: [],
      temperature: [0.2],
    };
    const executeChat = await deferredActor.run_create(request);

    await pic.tick(2);

    const pendingOutcalls = await pic.getPendingHttpsOutcalls();
    assert.equal(pendingOutcalls.length, 1);
    const pendingOutcall = pendingOutcalls[0];
    assert.ok(pendingOutcall);
    const body = textDecoder.decode(pendingOutcall.body);
    assert.match(body, /Always explain the flow\./);
    assert.match(body, /Gemini smoke already passed\./);
    assert.match(body, /Explain docs clearly\./);
    assert.match(body, /\[Memory context\]/);
    assert.match(body, /conversation\/relevant: Use AGENTS\.md and the doc skill memory/);

    await pic.mockPendingHttpsOutcall({
      requestId: pendingOutcall.requestId,
      subnetId: pendingOutcall.subnetId,
      response: {
        type: 'success',
        statusCode: 200,
        headers: [['content-type', 'application/json']],
        body: new TextEncoder().encode(
          JSON.stringify({
            id: 'chatcmpl-pocketic-context',
            object: 'chat.completion',
            created: 1741569952,
            model: 'gpt-4.1-mini',
            choices: [
              {
                index: 0,
                message: {
                  role: 'assistant',
                  content: 'context aware response',
                },
                finish_reason: 'stop',
              },
            ],
          }),
        ),
      },
    });

    const result = await executeChat();
    assert.ok('Ok' in result);
    assert.equal(result.Ok.response[0], 'context aware response');

    const fetched = await upgradedActor.memory_get({ key: 'workspace/AGENTS.md' });
    assert.ok('Ok' in fetched);
    assert.equal(fetched.Ok[0]?.content, 'Always explain the flow.');
  } finally {
    await pic.tearDown();
  }
});

test('agent observation query reports seeded memory and session summary state', async () => {
  const { pic, actor } = await setupCanister(server, providerConfig);

  try {
    assert.ok('Ok' in (await actor.memory_store({
      key: 'workspace/AGENTS.md',
      content: 'Always explain the flow.',
      category: { core: null },
      session_id: [],
    })));
    assert.ok('Ok' in (await actor.memory_store({
      key: 'core/user_preferences/response_style',
      content: 'Prefer concise answers.',
      category: { core: null },
      session_id: [],
    })));
    assert.ok('Ok' in (await actor.memory_store({
      key: 'core/project_facts/runtime',
      content: 'Runs as an ICP canister runtime.',
      category: { core: null },
      session_id: [],
    })));
    assert.ok('Ok' in (await actor.memory_store({
      key: 'conversation_summary/session-observe',
      content: 'turn_count:4\nsummary_turn_count:4\n[Session summary]\nFacts:\n- context carried forward',
      category: { conversation: null },
      session_id: ['session-observe'],
    })));
    assert.ok('Ok' in (await actor.memory_store({
      key: 'conversation/session-observe/user/1',
      content: 'first user turn',
      category: { conversation: null },
      session_id: ['session-observe'],
    })));
    assert.ok('Ok' in (await actor.memory_store({
      key: 'conversation/session-observe/assistant/1',
      content: 'first assistant turn',
      category: { conversation: null },
      session_id: ['session-observe'],
    })));

    const summary = await actor.conversation_summary_get({ session_id: 'session-observe' });
    assert.ok('Ok' in summary);
    assert.equal(summary.Ok[0]?.key, 'conversation_summary/session-observe');

    const observation = await actor.agent_observe({ session_id: ['session-observe'] });
    assert.ok('Ok' in observation);
    assert.deepEqual(observation.Ok.workspace_keys, ['workspace/AGENTS.md']);
    assert.equal(observation.Ok.conversation_summary_present, true);
    assert.equal(observation.Ok.conversation_turn_count, 2n);
    assert.equal(observation.Ok.enable_auto_promote, false);
    assert.equal(observation.Ok.enable_conversation_summary, true);
    assert.equal(observation.Ok.tool_loop_enabled, true);
    assert.equal(observation.Ok.history_limit, 8n);
    assert.equal(observation.Ok.max_tool_iterations, 3n);
    assert.ok(observation.Ok.core_keys.includes('core/user_preferences/response_style'));
    assert.ok(observation.Ok.core_keys.includes('core/project_facts/runtime'));
  } finally {
    await pic.tearDown();
  }
});

test('run_create completes native tool loop when upstream returns tool_calls', async () => {
  const { pic, actor, canisterId } = await setupCanister(server, providerConfig);

  try {
    const deferredActor = pic.createDeferredActor<_SERVICE>(idlFactory, canisterId);
    const request: RunCreateRequest = {
      agent_id: [],
      prompt: 'remember this via tool loop',
      session_id: ['tool-loop'],
      model: [],
      temperature: [0.2],
    };
    const executeChat = await deferredActor.run_create(request);

    let pendingOutcalls = await waitForPendingOutcall(pic);
    assert.equal(pendingOutcalls.length, 1);
    const firstOutcall = pendingOutcalls[0];
    assert.ok(firstOutcall);

    await pic.mockPendingHttpsOutcall({
      requestId: firstOutcall.requestId,
      subnetId: firstOutcall.subnetId,
      response: {
        type: 'success',
        statusCode: 200,
        headers: [['content-type', 'application/json']],
        body: new TextEncoder().encode(
          JSON.stringify({
            id: 'chatcmpl-pocketic-tool-1',
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
                      id: 'call_1',
                      type: 'function',
                      function: {
                        name: 'memory_store',
                        arguments: JSON.stringify({
                          key: 'conversation/tool-loop/note',
                          content: 'remembered via provider tool call',
                          session_id: 'tool-loop',
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

    pendingOutcalls = await waitForPendingOutcall(pic);
    assert.equal(pendingOutcalls.length, 1);
    const secondOutcall = pendingOutcalls[0];
    assert.ok(secondOutcall);
    const secondBody = textDecoder.decode(secondOutcall.body);
    assert.match(secondBody, /tool_calls/);
    assert.match(secondBody, /tool-call-missing-id|call_1/);
    assert.match(secondBody, /remembered via provider tool call/);
    assert.match(secondBody, /\\"success\\":true/);

    await pic.mockPendingHttpsOutcall({
      requestId: secondOutcall.requestId,
      subnetId: secondOutcall.subnetId,
      response: {
        type: 'success',
        statusCode: 200,
        headers: [['content-type', 'application/json']],
        body: new TextEncoder().encode(
          JSON.stringify({
            id: 'chatcmpl-pocketic-tool-2',
            object: 'chat.completion',
            created: 1741569953,
            model: 'gpt-4.1-mini',
            choices: [
              {
                index: 0,
                message: {
                  role: 'assistant',
                  content: 'tool loop complete',
                },
                finish_reason: 'stop',
              },
            ],
          }),
        ),
      },
    });

    const result = await executeChat();
    assert.ok('Ok' in result);
    assert.equal(result.Ok.response[0], 'tool loop complete');

    const recalled = await actor.memory_recall({
      query: 'remembered via provider tool call',
      limit: 10n,
      session_id: ['tool-loop'],
    });
    assert.ok('Ok' in recalled);
    assert.equal(recalled.Ok.some((item) => item.content === 'remembered via provider tool call'), true);
  } finally {
    await pic.tearDown();
  }
});

test('run_create exposes structured unknown_tool recovery payload', async () => {
  const { pic, canisterId } = await setupCanister(server, providerConfig);

  try {
    const actor = pic.createActor<_SERVICE>(idlFactory, canisterId);
    const createdAgent = await actor.agent_create({
      draft: {
        id: 'tool-recovery-agent',
        name: 'Tool Recovery Agent',
        description: 'Allows synthetic unknown-tool tests',
        enabled_tool_names: ['missing_tool'],
        requires_tool_approval: false,
        system_prompt_override: [],
        status: 'active',
      },
    });
    assert.ok('Ok' in createdAgent);

    const deferredActor = pic.createDeferredActor<_SERVICE>(idlFactory, canisterId);
    const request: RunCreateRequest = {
      agent_id: ['tool-recovery-agent'],
      prompt: 'recover from unknown tool',
      session_id: ['tool-recovery'],
      model: [],
      temperature: [0.0],
    };
    const executeChat = await deferredActor.run_create(request);

    let pendingOutcalls = await waitForPendingOutcall(pic);
    assert.equal(pendingOutcalls.length, 1);
    const firstOutcall = pendingOutcalls[0];
    assert.ok(firstOutcall);

    await pic.mockPendingHttpsOutcall({
      requestId: firstOutcall.requestId,
      subnetId: firstOutcall.subnetId,
      response: {
        type: 'success',
        statusCode: 200,
        headers: [['content-type', 'application/json']],
        body: new TextEncoder().encode(
          JSON.stringify({
            id: 'chatcmpl-pocketic-tool-unknown-1',
            object: 'chat.completion',
            created: 1741569952,
            model: 'gpt-4.1-mini',
            choices: [
              {
                index: 0,
                message: {
                  role: 'assistant',
                  content: 'trying an unsupported tool',
                  tool_calls: [
                    {
                      id: 'call_unknown',
                      type: 'function',
                      function: {
                        name: 'missing_tool',
                        arguments: '{}',
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

    pendingOutcalls = await waitForPendingOutcall(pic);
    assert.equal(pendingOutcalls.length, 1);
    const secondOutcall = pendingOutcalls[0];
    assert.ok(secondOutcall);
    const secondBody = textDecoder.decode(secondOutcall.body);
    assert.match(secondBody, /\\"error_code\\":\\"unknown_tool\\"/);
    assert.match(secondBody, /\\"retryable\\":false/);

    await pic.mockPendingHttpsOutcall({
      requestId: secondOutcall.requestId,
      subnetId: secondOutcall.subnetId,
      response: {
        type: 'success',
        statusCode: 200,
        headers: [['content-type', 'application/json']],
        body: new TextEncoder().encode(
          JSON.stringify({
            id: 'chatcmpl-pocketic-tool-unknown-2',
            object: 'chat.completion',
            created: 1741569953,
            model: 'gpt-4.1-mini',
            choices: [
              {
                index: 0,
                message: {
                  role: 'assistant',
                  content: 'recovered from unknown tool',
                },
                finish_reason: 'stop',
              },
            ],
          }),
        ),
      },
    });

    const result = await executeChat();
    assert.ok('Ok' in result);
    assert.equal(result.Ok.response[0], 'recovered from unknown tool');
  } finally {
    await pic.tearDown();
  }
});

test('run_create fails fast on repeated identical failing tool call', async () => {
  const { pic, actor, canisterId } = await setupCanister(server, providerConfig);

  try {
    const createdAgent = await actor.agent_create({
      draft: {
        id: 'tool-repeat-agent',
        name: 'Tool Repeat Agent',
        description: 'Allows repeated missing-tool tests',
        enabled_tool_names: ['missing_tool'],
        requires_tool_approval: false,
        system_prompt_override: [],
        status: 'active',
      },
    });
    assert.ok('Ok' in createdAgent);

    const deferredActor = pic.createDeferredActor<_SERVICE>(idlFactory, canisterId);
    const request: RunCreateRequest = {
      agent_id: ['tool-repeat-agent'],
      prompt: 'repeat broken tool',
      session_id: ['tool-repeat'],
      model: [],
      temperature: [0.0],
    };
    const executeChat = await deferredActor.run_create(request);

    let pendingOutcalls = await waitForPendingOutcall(pic);
    assert.equal(pendingOutcalls.length, 1);
    const firstOutcall = pendingOutcalls[0];
    assert.ok(firstOutcall);

    const repeatedToolCallBody = JSON.stringify({
      id: 'chatcmpl-pocketic-tool-repeat',
      object: 'chat.completion',
      created: 1741569952,
      model: 'gpt-4.1-mini',
      choices: [
        {
          index: 0,
          message: {
            role: 'assistant',
            content: 'still trying the same tool',
            tool_calls: [
              {
                id: 'call_repeat',
                type: 'function',
                function: {
                  name: 'missing_tool',
                  arguments: '{"key":"same"}',
                },
              },
            ],
          },
          finish_reason: 'tool_calls',
        },
      ],
    });

    await pic.mockPendingHttpsOutcall({
      requestId: firstOutcall.requestId,
      subnetId: firstOutcall.subnetId,
      response: {
        type: 'success',
        statusCode: 200,
        headers: [['content-type', 'application/json']],
        body: new TextEncoder().encode(repeatedToolCallBody),
      },
    });

    pendingOutcalls = await waitForPendingOutcall(pic);
    assert.equal(pendingOutcalls.length, 1);
    const secondOutcall = pendingOutcalls[0];
    assert.ok(secondOutcall);

    await pic.mockPendingHttpsOutcall({
      requestId: secondOutcall.requestId,
      subnetId: secondOutcall.subnetId,
      response: {
        type: 'success',
        statusCode: 200,
        headers: [['content-type', 'application/json']],
        body: new TextEncoder().encode(repeatedToolCallBody),
      },
    });

    const result = await executeChat();
    assert.ok('Ok' in result);
    assert.equal(result.Ok.status, 'failed');
    assert.match(result.Ok.error[0] ?? '', /repeated the same failing call/);

    const counted = await actor.memory_count();
    assert.ok('Ok' in counted);
  } finally {
    await pic.tearDown();
  }
});

test('run_create returns Ok(failed run) when provider is unset', async () => {
  const { pic, actor, canisterId } = await setupCanister(server);
  const request: RunCreateRequest = {
      agent_id: [],
    prompt: 'hello from pocketic',
    session_id: ['session-a'],
    model: ['gpt-4o-mini'],
    temperature: [0.2],
  };

  try {
    const result = await actor.run_create(request);

    assert.ok('Ok' in result);
    assert.equal(result.Ok.status, 'failed');
    assert.equal(result.Ok.error[0]?.includes('not configured'), true);
  } finally {
    await pic.tearDown();
  }
});

test('run_create enters provider path and surfaces provider_error when configured', async () => {
  const { pic, canisterId } = await setupCanister(server, providerConfig);

  try {
    const deferredActor = pic.createDeferredActor<_SERVICE>(idlFactory, canisterId);
    const request: RunCreateRequest = {
      agent_id: [],
      prompt: 'hello from configured provider',
      session_id: [],
      model: [],
      temperature: [0.2],
    };
    const executeChat = await deferredActor.run_create(request);

    await pic.tick(2);

    let pendingOutcalls = await pic.getPendingHttpsOutcalls();
    assert.equal(pendingOutcalls.length, 1);
    const pendingOutcall = pendingOutcalls[0];
    assert.ok(pendingOutcall);
    assert.equal(pendingOutcall.url, 'https://api.openai.com/v1/chat/completions');
    assert.equal(pendingOutcall.httpMethod, 'POST');

    await pic.mockPendingHttpsOutcall({
      requestId: pendingOutcall.requestId,
      subnetId: pendingOutcall.subnetId,
      response: {
        type: 'success',
        statusCode: 500,
        headers: [['content-type', 'application/json']],
        body: new TextEncoder().encode(
          JSON.stringify({
            error: {
              message: 'mock upstream failure',
            },
          }),
        ),
      },
    });

    pendingOutcalls = await waitForPendingOutcall(pic);
    assert.equal(pendingOutcalls.length, 1);
    const retryOutcall = pendingOutcalls[0];
    assert.ok(retryOutcall);

    await pic.mockPendingHttpsOutcall({
      requestId: retryOutcall.requestId,
      subnetId: retryOutcall.subnetId,
      response: {
        type: 'success',
        statusCode: 500,
        headers: [['content-type', 'application/json']],
        body: new TextEncoder().encode(
          JSON.stringify({
            error: {
              message: 'mock upstream failure',
            },
          }),
        ),
      },
    });

    const result = await executeChat();

    assert.ok('Ok' in result);
    assert.equal(result.Ok.status, 'failed');
    assert.match(result.Ok.error[0] ?? '', /OpenAI-compatible API error/);
  } finally {
    await pic.tearDown();
  }
});

test('run_create retries a transient provider failure once', async () => {
  const { pic, canisterId } = await setupCanister(server, providerConfig);

  try {
    const deferredActor = pic.createDeferredActor<_SERVICE>(idlFactory, canisterId);
    const request: RunCreateRequest = {
      agent_id: [],
      prompt: 'hello after transient failure',
      session_id: ['session-retry'],
      model: [],
      temperature: [0.2],
    };
    const executeChat = await deferredActor.run_create(request);

    let pendingOutcalls = await waitForPendingOutcall(pic);
    assert.equal(pendingOutcalls.length, 1);
    const firstOutcall = pendingOutcalls[0];
    assert.ok(firstOutcall);

    await pic.mockPendingHttpsOutcall({
      requestId: firstOutcall.requestId,
      subnetId: firstOutcall.subnetId,
      response: {
        type: 'success',
        statusCode: 503,
        headers: [['content-type', 'application/json']],
        body: new TextEncoder().encode(
          JSON.stringify({
            error: {
              message: 'temporarily unavailable',
            },
          }),
        ),
      },
    });

    pendingOutcalls = await waitForPendingOutcall(pic);
    assert.equal(pendingOutcalls.length, 1);
    const secondOutcall = pendingOutcalls[0];
    assert.ok(secondOutcall);

    await pic.mockPendingHttpsOutcall({
      requestId: secondOutcall.requestId,
      subnetId: secondOutcall.subnetId,
      response: {
        type: 'success',
        statusCode: 200,
        headers: [['content-type', 'application/json']],
        body: new TextEncoder().encode(
          JSON.stringify({
            id: 'chatcmpl-pocketic-retry-success',
            object: 'chat.completion',
            created: 1741569952,
            model: 'gpt-4.1-mini',
            choices: [
              {
                index: 0,
                message: {
                  role: 'assistant',
                  content: 'retry recovered',
                },
                finish_reason: 'stop',
              },
            ],
          }),
        ),
      },
    });

    const result = await executeChat();
    assert.ok('Ok' in result);
    assert.equal(result.Ok.response[0], 'retry recovered');
  } finally {
    await pic.tearDown();
  }
});

test('run_create returns provider success payload when configured', async () => {
  const { pic, canisterId } = await setupCanister(server, providerConfig);

  try {
    const deferredActor = pic.createDeferredActor<_SERVICE>(idlFactory, canisterId);
    const request: RunCreateRequest = {
      agent_id: [],
      prompt: 'hello from configured provider success',
      session_id: ['session-success'],
      model: [],
      temperature: [0.2],
    };
    const executeChat = await deferredActor.run_create(request);

    await pic.tick(2);

    const pendingOutcalls = await pic.getPendingHttpsOutcalls();
    assert.equal(pendingOutcalls.length, 1);
    const pendingOutcall = pendingOutcalls[0];
    assert.ok(pendingOutcall);

    await pic.mockPendingHttpsOutcall({
      requestId: pendingOutcall.requestId,
      subnetId: pendingOutcall.subnetId,
      response: {
        type: 'success',
        statusCode: 200,
        headers: [['content-type', 'application/json']],
        body: new TextEncoder().encode(
          JSON.stringify({
            id: 'chatcmpl-pocketic-success',
            object: 'chat.completion',
            created: 1741569952,
            model: 'gpt-4.1-mini',
            choices: [
              {
                index: 0,
                message: {
                  role: 'assistant',
                  content: 'mock success response',
                },
                finish_reason: 'stop',
              },
            ],
          }),
        ),
      },
    });

    const result = await executeChat();

    assert.ok('Ok' in result);
    assert.equal(result.Ok.response[0], 'mock success response');
    assert.equal(result.Ok.model[0], 'gpt-4.1-mini');
    assert.equal(result.Ok.session_id, 'session-success');
    assert.equal(result.Ok.provider_ready, true);
    assert.equal(result.Ok.memory_ready, true);
  } finally {
    await pic.tearDown();
  }
});

test('run_create does not auto-promote preferences when config leaves auto-promote unset', async () => {
  const { pic, actor, canisterId } = await setupCanister(server, providerConfig);

  try {
    const forgotten = await actor.memory_forget({ key: 'core/user_preferences/response_style' });
    assert.ok('Ok' in forgotten);

    const deferredActor = pic.createDeferredActor<_SERVICE>(idlFactory, canisterId);
    const request: RunCreateRequest = {
      agent_id: [],
      prompt: 'Prefer concise answers with concrete implementation details.',
      session_id: ['session-no-auto-promote'],
      model: [],
      temperature: [0.0],
    };
    const executeChat = await deferredActor.run_create(request);

    const pendingOutcalls = await waitForPendingOutcall(pic);
    assert.equal(pendingOutcalls.length, 1);
    const pendingOutcall = pendingOutcalls[0];
    assert.ok(pendingOutcall);

    await pic.mockPendingHttpsOutcall({
      requestId: pendingOutcall.requestId,
      subnetId: pendingOutcall.subnetId,
      response: {
        type: 'success',
        statusCode: 200,
        headers: [['content-type', 'application/json']],
        body: new TextEncoder().encode(
          JSON.stringify({
            id: 'chatcmpl-pocketic-no-auto-promote',
            object: 'chat.completion',
            created: 1741569952,
            model: 'gpt-4.1-mini',
            choices: [
              {
                index: 0,
                message: {
                  role: 'assistant',
                  content: 'acknowledged',
                },
                finish_reason: 'stop',
              },
            ],
          }),
        ),
      },
    });

    const result = await executeChat();
    assert.ok('Ok' in result);

    const freshActor = pic.createActor<_SERVICE>(idlFactory, canisterId);
    const fetched = await freshActor.memory_get({ key: 'core/user_preferences/response_style' });
    assert.ok('Ok' in fetched);
    assert.equal(fetched.Ok.length, 0);
  } finally {
    await pic.tearDown();
  }
});
