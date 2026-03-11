// where: iclaw/tests/src/iclaw-ic.test.ts
// what: PocketIC integration tests for iclaw_ic public canister API
// why: Validate query/update behavior before mainnet verification

import { after, before, test } from 'node:test';
import assert from 'node:assert/strict';
import { PocketIcServer } from '@dfinity/pic';
import { IDL } from '@icp-sdk/core/candid';
import { candid, idlFactory, type _SERVICE, type CanisterConfig, type ChatRequest } from './declarations.js';
import { encodeEmptyArgs, encodeInitArg, encodeNoConfigInitArg, providerConfig, setupCanister, wasmBytes } from './helpers.js';

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
      wasm: wasmBytes,
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
      wasm: wasmBytes,
      arg: encodeInitArg(config),
    });

    const upgradedActor = pic.createActor<_SERVICE>(idlFactory, canisterId);
    const deferredActor = pic.createDeferredActor<_SERVICE>(idlFactory, canisterId);
    const request: ChatRequest = {
      prompt: 'Use AGENTS.md and the doc skill memory',
      session_id: ['session-context'],
      model: [],
      temperature: [0.2],
    };
    const executeChat = await deferredActor.chat(request);

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
    assert.equal(result.Ok.response, 'context aware response');

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

test('chat update completes native tool loop when upstream returns tool_calls', async () => {
  const { pic, actor, canisterId } = await setupCanister(server, providerConfig);

  try {
    const deferredActor = pic.createDeferredActor<_SERVICE>(idlFactory, canisterId);
    const request: ChatRequest = {
      prompt: 'remember this via tool loop',
      session_id: ['tool-loop'],
      model: [],
      temperature: [0.2],
    };
    const executeChat = await deferredActor.chat(request);

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
    assert.equal(result.Ok.response, 'tool loop complete');

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

test('chat update exposes structured unknown_tool recovery payload', async () => {
  const { pic, canisterId } = await setupCanister(server, providerConfig);

  try {
    const deferredActor = pic.createDeferredActor<_SERVICE>(idlFactory, canisterId);
    const request: ChatRequest = {
      prompt: 'recover from unknown tool',
      session_id: ['tool-recovery'],
      model: [],
      temperature: [0.0],
    };
    const executeChat = await deferredActor.chat(request);

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
    assert.equal(result.Ok.response, 'recovered from unknown tool');
  } finally {
    await pic.tearDown();
  }
});

test('chat update fails fast on repeated identical failing tool call', async () => {
  const { pic, actor, canisterId } = await setupCanister(server, providerConfig);

  try {
    const deferredActor = pic.createDeferredActor<_SERVICE>(idlFactory, canisterId);
    const request: ChatRequest = {
      prompt: 'repeat broken tool',
      session_id: ['tool-repeat'],
      model: [],
      temperature: [0.0],
    };
    const executeChat = await deferredActor.chat(request);

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
    assert.ok('Err' in result);
    assert.equal(result.Err.code, 'provider_error');
    assert.match(result.Err.message, /repeated the same failing call/);

    const counted = await actor.memory_count();
    assert.ok('Ok' in counted);
  } finally {
    await pic.tearDown();
  }
});

test('chat update returns not_supported when provider is unset', async () => {
  const { pic, actor, canisterId } = await setupCanister(server);
  const request: ChatRequest = {
    prompt: 'hello from pocketic',
    session_id: ['session-a'],
    model: ['gpt-4o-mini'],
    temperature: [0.2],
  };

  try {
    await pic.updateCall({
      canisterId,
      method: 'chat',
      arg: IDL.encode([candid.chatRequest], [request]),
    });
    const result = await actor.chat(request);

    assert.ok('Err' in result);
    assert.equal(result.Err.code, 'not_supported');
  } finally {
    await pic.tearDown();
  }
});

test('chat update enters provider path and surfaces provider_error when configured', async () => {
  const { pic, canisterId } = await setupCanister(server, providerConfig);

  try {
    const deferredActor = pic.createDeferredActor<_SERVICE>(idlFactory, canisterId);
    const request: ChatRequest = {
      prompt: 'hello from configured provider',
      session_id: [],
      model: [],
      temperature: [0.2],
    };
    const executeChat = await deferredActor.chat(request);

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

    assert.ok('Err' in result);
    assert.equal(result.Err.code, 'provider_error');
    assert.match(result.Err.message, /OpenAI-compatible API error/);
  } finally {
    await pic.tearDown();
  }
});

test('chat update retries a transient provider failure once', async () => {
  const { pic, canisterId } = await setupCanister(server, providerConfig);

  try {
    const deferredActor = pic.createDeferredActor<_SERVICE>(idlFactory, canisterId);
    const request: ChatRequest = {
      prompt: 'hello after transient failure',
      session_id: ['session-retry'],
      model: [],
      temperature: [0.2],
    };
    const executeChat = await deferredActor.chat(request);

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
    assert.equal(result.Ok.response, 'retry recovered');
  } finally {
    await pic.tearDown();
  }
});

test('chat update returns provider success payload when configured', async () => {
  const { pic, canisterId } = await setupCanister(server, providerConfig);

  try {
    const deferredActor = pic.createDeferredActor<_SERVICE>(idlFactory, canisterId);
    const request: ChatRequest = {
      prompt: 'hello from configured provider success',
      session_id: ['session-success'],
      model: [],
      temperature: [0.2],
    };
    const executeChat = await deferredActor.chat(request);

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
    assert.equal(result.Ok.response, 'mock success response');
    assert.equal(result.Ok.model[0], 'gpt-4.1-mini');
    assert.equal(result.Ok.session_id[0], 'session-success');
    assert.equal(result.Ok.provider_ready, true);
    assert.equal(result.Ok.memory_ready, true);
  } finally {
    await pic.tearDown();
  }
});

test('chat update does not auto-promote preferences when config leaves auto-promote unset', async () => {
  const { pic, actor, canisterId } = await setupCanister(server, providerConfig);

  try {
    const forgotten = await actor.memory_forget({ key: 'core/user_preferences/response_style' });
    assert.ok('Ok' in forgotten);

    const deferredActor = pic.createDeferredActor<_SERVICE>(idlFactory, canisterId);
    const request: ChatRequest = {
      prompt: 'Prefer concise answers with concrete implementation details.',
      session_id: ['session-no-auto-promote'],
      model: [],
      temperature: [0.0],
    };
    const executeChat = await deferredActor.chat(request);

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
