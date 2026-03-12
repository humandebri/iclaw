// where: iclaw/tests/src/webhook-ic.test.ts
// what: PocketIC integration tests for webhook automation entry APIs
// why: Phase 3a adds a public invoke path, so webhook storage and trigger metadata must stay stable

import { after, before, test } from 'node:test';
import assert from 'node:assert/strict';
import { PocketIcServer } from '@dfinity/pic';
import { setupCanister } from './helpers.js';

let server: PocketIcServer;

before(async () => {
  server = await PocketIcServer.start();
});

after(async () => {
  if (server) {
    await server.stop();
  }
});

test('webhook CRUD APIs round-trip through PocketIC', async () => {
  const { pic, actor } = await setupCanister(server);

  try {
    const created = await actor.webhook_create({
      draft: {
        id: 'daily-brief',
        name: 'Daily Brief',
        agent_id: 'default',
        session_mode: 'create_new',
        fixed_session_id: [],
        secret: 'secret-1',
        enabled: true,
      },
    });
    assert.ok('Ok' in created);
    assert.equal(created.Ok.id, 'daily-brief');
    assert.equal(created.Ok.secret, 'secret-1');

    const fetched = await actor.webhook_get({ webhook_id: 'daily-brief' });
    assert.ok('Ok' in fetched);
    assert.equal(fetched.Ok[0]?.name, 'Daily Brief');
    assert.equal(fetched.Ok[0]?.secret, '********');

    const listed = await actor.webhooks_list();
    assert.ok('Ok' in listed);
    assert.equal(listed.Ok.some((webhook) => webhook.id === 'daily-brief'), true);
    assert.equal(listed.Ok[0]?.secret, '********');

    const updated = await actor.webhook_update({
      webhook: {
        ...created.Ok,
        secret: '********',
        enabled: false,
      },
      secret_override: ['secret-2'],
    });
    assert.ok('Ok' in updated);
    assert.equal(updated.Ok.enabled, false);
    assert.equal(updated.Ok.secret, 'secret-2');

    const deleted = await actor.webhook_delete({ webhook_id: 'daily-brief' });
    assert.ok('Ok' in deleted);
    assert.equal(deleted.Ok, true);
  } finally {
    await pic.tearDown();
  }
});

test('webhook_update preserves last run and rejection metadata', async () => {
  const { pic, actor } = await setupCanister(server);

  try {
    const created = await actor.webhook_create({
      draft: {
        id: 'preserve-hook',
        name: 'Preserve Hook',
        agent_id: 'default',
        session_mode: 'reuse_fixed',
        fixed_session_id: ['preserve-session'],
        secret: 'secret-keep',
        enabled: true,
      },
    });
    assert.ok('Ok' in created);

    const invoked = await actor.webhook_invoke({
      webhook_id: 'preserve-hook',
      secret: 'secret-keep',
      prompt: 'trigger webhook run',
      session_id: [],
      model: [],
      temperature: [],
    });
    assert.ok('Ok' in invoked);

    const rejected = await actor.webhook_invoke({
      webhook_id: 'preserve-hook',
      secret: 'wrong-secret',
      prompt: 'reject me',
      session_id: [],
      model: [],
      temperature: [],
    });
    assert.ok('Err' in rejected);

    const updated = await actor.webhook_update({
      webhook: {
        id: 'preserve-hook',
        name: 'Preserve Hook Updated',
        agent_id: 'default',
        session_mode: 'reuse_fixed',
        fixed_session_id: ['preserve-session'],
        secret: '********',
        enabled: false,
        created_at: 'stale-created-at',
        updated_at: 'stale-updated-at',
        last_run_id: [],
        last_secret_rotated_at: [],
        last_invoked_at: [],
        last_rejection_at: [],
        last_rejection_reason: [],
      },
      secret_override: ['secret-next'],
    });
    assert.ok('Ok' in updated);
    assert.equal(updated.Ok.created_at, created.Ok.created_at);
    assert.equal(updated.Ok.last_run_id[0], invoked.Ok.id);
    assert.equal(updated.Ok.last_rejection_reason[0], 'webhook secret is invalid');
    assert.equal(updated.Ok.secret, 'secret-next');
  } finally {
    await pic.tearDown();
  }
});

test('webhook_invoke records webhook trigger metadata and updates last_run_id', async () => {
  const { pic, actor } = await setupCanister(server);

  try {
    const created = await actor.webhook_create({
      draft: {
        id: 'fixed-session-hook',
        name: 'Fixed Session Hook',
        agent_id: 'default',
        session_mode: 'reuse_fixed',
        fixed_session_id: ['webhook-session'],
        secret: 'secret-2',
        enabled: true,
      },
    });
    assert.ok('Ok' in created);

    const invoked = await actor.webhook_invoke({
      webhook_id: 'fixed-session-hook',
      secret: 'secret-2',
      prompt: 'trigger webhook run',
      session_id: [],
      model: [],
      temperature: [],
    });
    assert.ok('Ok' in invoked);
    assert.equal(invoked.Ok.trigger_kind, 'webhook');
    assert.equal(invoked.Ok.trigger_id[0], 'fixed-session-hook');
    assert.equal(invoked.Ok.session_id, 'webhook-session');

    const events = await actor.run_events_get({ run_id: invoked.Ok.id });
    assert.ok('Ok' in events);
    assert.equal(events.Ok[0]?.kind, 'trigger_received');
    assert.equal(events.Ok.some((event) => event.kind === 'failed'), true);

    const fetched = await actor.webhook_get({ webhook_id: 'fixed-session-hook' });
    assert.ok('Ok' in fetched);
    assert.equal(fetched.Ok[0]?.last_run_id[0], invoked.Ok.id);
    assert.equal(fetched.Ok[0]?.secret, '********');
  } finally {
    await pic.tearDown();
  }
});

test('webhook_invoke rejects invalid secrets without creating a run', async () => {
  const { pic, actor } = await setupCanister(server);

  try {
    const created = await actor.webhook_create({
      draft: {
        id: 'guarded-hook',
        name: 'Guarded Hook',
        agent_id: 'default',
        session_mode: 'create_new',
        fixed_session_id: [],
        secret: 'secret-3',
        enabled: true,
      },
    });
    assert.ok('Ok' in created);

    const invoked = await actor.webhook_invoke({
      webhook_id: 'guarded-hook',
      secret: 'wrong-secret',
      prompt: 'should fail',
      session_id: [],
      model: [],
      temperature: [],
    });
    assert.ok('Err' in invoked);
    assert.equal(invoked.Err.code, 'unauthorized');

    const runs = await actor.run_list({ session_id: [], limit: [10n] });
    assert.ok('Ok' in runs);
    assert.equal(runs.Ok.length, 0);

    const fetched = await actor.webhook_get({ webhook_id: 'guarded-hook' });
    assert.ok('Ok' in fetched);
    assert.equal(fetched.Ok[0]?.last_rejection_reason[0], 'webhook secret is invalid');
    assert.equal(typeof fetched.Ok[0]?.last_rejection_at[0], 'string');
  } finally {
    await pic.tearDown();
  }
});

test('webhook rejection logs retain only the latest 100 entries', async () => {
  const { pic, actor } = await setupCanister(server);

  try {
    const created = await actor.webhook_create({
      draft: {
        id: 'retention-hook',
        name: 'Retention Hook',
        agent_id: 'default',
        session_mode: 'create_new',
        fixed_session_id: [],
        secret: 'secret-4',
        enabled: true,
      },
    });
    assert.ok('Ok' in created);

    for (let index = 0; index < 105; index += 1) {
      const invoked = await actor.webhook_invoke({
        webhook_id: 'retention-hook',
        secret: 'wrong-secret',
        prompt: `reject ${index}`,
        session_id: [],
        model: [],
        temperature: [],
      });
      assert.ok('Err' in invoked);
      assert.equal(invoked.Err.code, 'unauthorized');
    }

    const rejections = await actor.webhook_rejections_list({
      webhook_id: 'retention-hook',
      limit: [200n],
    });
    assert.ok('Ok' in rejections);
    assert.equal(rejections.Ok.length, 100);
    assert.equal(rejections.Ok[0]?.reason, 'webhook secret is invalid');

    const fetched = await actor.webhook_get({ webhook_id: 'retention-hook' });
    assert.ok('Ok' in fetched);
    assert.equal(fetched.Ok[0]?.last_rejection_reason[0], 'webhook secret is invalid');
  } finally {
    await pic.tearDown();
  }
});

test('webhook_invoke rejects fixed sessions that belong to another agent', async () => {
  const { pic, actor } = await setupCanister(server);

  try {
    const createdAgent = await actor.agent_create({
      draft: {
        id: 'secondary-agent',
        name: 'Secondary Agent',
        description: 'second agent',
        enabled_tool_names: [],
        requires_tool_approval: false,
        system_prompt_override: [],
        status: 'active',
      },
    });
    assert.ok('Ok' in createdAgent);

    const defaultWebhook = await actor.webhook_create({
      draft: {
        id: 'seed-default-session',
        name: 'Seed Default Session',
        agent_id: 'default',
        session_mode: 'reuse_fixed',
        fixed_session_id: ['shared-session'],
        secret: 'seed-secret',
        enabled: true,
      },
    });
    assert.ok('Ok' in defaultWebhook);

    const seededRun = await actor.webhook_invoke({
      webhook_id: 'seed-default-session',
      secret: 'seed-secret',
      prompt: 'create shared session',
      session_id: [],
      model: [],
      temperature: [],
    });
    assert.ok('Ok' in seededRun);
    assert.equal(seededRun.Ok.session_id, 'shared-session');

    const conflictingWebhook = await actor.webhook_create({
      draft: {
        id: 'conflict-hook',
        name: 'Conflict Hook',
        agent_id: 'secondary-agent',
        session_mode: 'reuse_fixed',
        fixed_session_id: ['shared-session'],
        secret: 'conflict-secret',
        enabled: true,
      },
    });
    assert.ok('Ok' in conflictingWebhook);

    const invoked = await actor.webhook_invoke({
      webhook_id: 'conflict-hook',
      secret: 'conflict-secret',
      prompt: 'should fail',
      session_id: [],
      model: [],
      temperature: [],
    });
    assert.ok('Err' in invoked);
    assert.equal(invoked.Err.code, 'invalid_argument');
    assert.match(invoked.Err.message, /session agent_id must match webhook agent_id/);
  } finally {
    await pic.tearDown();
  }
});
