import { expect, test } from 'claude-code/testing';
import { register } from '../hooks/register.js';

// The kit's command.run stub models event answers, not native host promise
// rejection. Inject that host failure directly through the production hooks.
async function host(options = {}) {
  const hooks = [];
  const saved = new Map();
  const prompts = [];
  let sessionId = 'source';
  let timer;
  let submittedInput;
  register((event, matcher, handler) => {
    if (typeof matcher === 'function') { handler = matcher; matcher = {}; }
    hooks.push({ event, matcher, handler });
    return { catch() {} };
  });
  const fire = async (event, input) => {
    const matches = hooks.filter(h => h.event === event && Object.entries(h.matcher).every(([k, v]) => input[k] === v));
    const next = async e => matches.length ? matches.shift().handler(api, e, next) : {};
    return next(input);
  };
  const api = {
    session: { id: async () => sessionId, cwd: async () => '/work' },
    store: {
      get: async key => saved.get(key),
      set: async (key, value) => {
        if (options.failAfterClear && sessionId === 'target') throw new Error('store failed after clear');
        saved.set(key, JSON.parse(JSON.stringify(value)));
      },
    },
    tool: { register: async () => {}, check: async () => ({ decision: 'allow' }) },
    command: {
      register: async () => {},
      run: async () => {
        if (options.rejectBeforeClear) throw new Error('the host has ended its input');
        if (options.inputWhileQueued) submittedInput = await fire('prompt.submit', { text: 'new user work' });
        await fire('session.end', { reason: 'clear', sessionId: 'source' });
        sessionId = 'target';
        if (options.rejectAfterClear) throw new Error('downstream command hook failed');
        return {};
      },
    },
    prompt: { submit: async args => { prompts.push(args.text); return {}; } },
    clock: { after: (_, fn) => { timer = fn; } },
    ui: { status: async () => {} },
  };
  await fire('session.start', {});
  await fire('turn.start', { turnId: 'turn' });
  await fire('tool.call', { tool: 'mcp__work-engine-context__new_context', tool_use_id: 'call' });
  await fire('turn.complete', { turnId: 'turn', reason: 'answer', isAborted: false });
  return { fire, run: () => timer(), saved, prompts, submittedInput: () => submittedInput };
}

test('host command rejection before clear records failure without continuation', async () => {
  const h = await host({ rejectBeforeClear: true });
  await h.run();
  expect(h.saved.get('reset:source').reset).toBe('failed');
  expect(h.saved.get('reset:source').detail).toContain('ended its input');
  expect(h.prompts).toHaveLength(0);
});

test('command rejection after native clear preserves the observed reset', async () => {
  const h = await host({ rejectAfterClear: true });
  await h.run();
  expect(h.saved.get('reset:source').reset).toBe('confirmed');
  expect(h.saved.get('reset:source').targetSessionId).toBe('target');
  expect(h.saved.get('reset:source').continuation).toBe('failed');
  expect(h.prompts).toHaveLength(0);
});

test('new user work is refused while an uncancellable native command is queued', async () => {
  const h = await host({ inputWhileQueued: true });
  await h.run();
  expect(h.submittedInput().drop).toContain('Retry this input after recovery');
  expect(h.saved.get('reset:source').reset).toBe('confirmed');
});

test('status preserves in-memory reset evidence when persistence fails after clear', async () => {
  const h = await host({ failAfterClear: true });
  await expect(h.run()).rejects.toThrow('store failed after clear');
  const status = JSON.parse((await h.fire('command.run', { command: 'context-reset-status' })).text);
  expect(status.reset).toBe('confirmed');
  expect(status.targetSessionId).toBe('target');
  expect(status.persistence).toBe('unconfirmed');
});
