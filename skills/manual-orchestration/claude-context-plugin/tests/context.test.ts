import { expect, mock, test } from 'claude-code/testing';

const tool = 'mcp__work-engine-context__new_context';
const source = '11111111-1111-4111-8111-111111111111';
const target = '22222222-2222-4222-8222-222222222222';

async function setup(kit, on, options = {}) {
  const clock = mock.clock(on);
  const saved = new Map(Object.entries(options.saved ?? {}));
  const calls = [];
  let sessionId = source;
  let cwd = '/work';
  on('store.get', (_, e) => ({ value: saved.get(e.key) }));
  on('store.set', (_, e) => {
    if (options.storeFailure) return { deny: 'store unavailable' };
    saved.set(e.key, JSON.parse(JSON.stringify(e.value)));
    return { value: undefined };
  });
  on('session.id', () => ({ value: sessionId }));
  on('session.cwd', () => ({ value: cwd }));
  on('tool.register', (_, e) => { calls.push(['register', e.tool ?? e]); return { value: undefined }; });
  on('command.register', () => ({ value: undefined }));
  on('tool.check', () => ({ decision: options.permission ?? 'allow' }));
  on('ui.status', (_, e) => { calls.push(['status', e.text]); return { value: undefined }; });
  on('session.start', () => ({ cwd }));
  on('session.end', (_, e) => ({ sessionId: e.sessionId }));
  on('turn.start', (_, e) => ({ turnId: e.turnId }));
  on('turn.complete', () => { calls.push(['turn_ended']); return { text: '' }; });
  on('command.run', async (_, e) => {
    calls.push(['command', e.command]);
    if (options.interceptClear) return {};
    await kit.session.end({ reason: 'clear', sessionId, resume: { id: sessionId } });
    sessionId = target;
    return {};
  });
  on('prompt.submit', (_, e) => {
    calls.push(['prompt', e.text]);
    return options.dropPrompt ? { drop: 'the host has ended its input' } : { text: e.text };
  });
  await kit.session.start({ cwd, surface: 'terminal', isInteractive: true });
  await kit.turn.start({ turnId: 't1' });
  return {
    clock, saved, calls,
    request: () => kit.tool.call({ tool }),
    end: (extra = {}) => kit.turn.complete({ turnId: 't1', answer: 'Ready for reset', durationMs: 1, isAborted: false, reason: 'answer', ...extra }),
    record: () => saved.get('reset:' + source),
    move: () => { cwd = '/elsewhere'; },
  };
}

test('returns a pending acknowledgement, then clears and submits recovery after turn end', async ($, on) => {
  const h = await setup($, on);
  const answer = await h.request();
  expect(JSON.parse(answer.result).reset).toBe('pending_turn_end');
  expect(h.calls.filter(x => x[0] === 'command')).toHaveLength(0);
  await h.end();
  expect(h.calls.filter(x => x[0] === 'command')).toHaveLength(0);
  await h.clock.settle();
  expect(h.calls.filter(x => x[0] === 'command')).toEqual([['command', 'clear']]);
  expect(h.record().checkpoint).toBe('agent_owned_unverified');
  expect(h.record().reset).toBe('confirmed');
  expect(h.record().targetSessionId).toBe(target);
  expect(h.record().continuation).toBe('submitted');
  const prompt = h.calls.find(x => x[0] === 'prompt')[1];
  expect(prompt).toContain('context-recovery:' + source);
  expect(prompt).toContain('$wind-walker');
  expect(prompt).toContain('not a new grant');
  expect(h.saved.get('source:' + target)).toBe(source);
});

test('duplicate calls and duplicate completion events produce one clear', async ($, on) => {
  const h = await setup($, on);
  await h.request();
  await h.request();
  await h.end();
  await h.end();
  await h.clock.settle();
  expect(h.calls.filter(x => x[0] === 'command')).toHaveLength(1);
  expect(h.calls.filter(x => x[0] === 'prompt')).toHaveLength(1);
});

for (const reason of ['aborted', 'error', 'refusal']) {
  test(reason + ' cancels the request without clearing a later turn', async ($, on) => {
    const h = await setup($, on);
    await h.request();
    await h.end({ reason, isAborted: reason === 'aborted' });
    await $.turn.start({ turnId: 't2' });
    await $.turn.complete({ turnId: 't2', answer: '', durationMs: 1, isAborted: false, reason: 'answer' });
    await h.clock.settle();
    expect(h.record().reset).toBe('cancelled');
    expect(h.calls.filter(x => x[0] === 'command')).toHaveLength(0);
  });
}

test('a subagent completion does not schedule the main reset', async ($, on) => {
  const h = await setup($, on);
  await h.request();
  await h.end({ agentId: 'child' });
  await h.clock.settle();
  expect(h.record().reset).toBe('pending_turn_end');
  expect(h.calls.filter(x => x[0] === 'command')).toHaveLength(0);
});

test('a subagent cannot request clearing its parent', async ($, on) => {
  const h = await setup($, on);
  const result = await $.tool.call({ tool, agentId: 'child' });
  expect(result.deny).toContain('subagent reset is unsupported');
  expect(h.record()).toBeUndefined();
});

for (const permission of ['deny', 'ask']) {
  test('honors ' + permission + ' permission without recording a request', async ($, on) => {
    const h = await setup($, on, { permission });
    const result = await h.request();
    expect(result.deny).toContain('requires tool permission');
    expect(h.record()).toBeUndefined();
  });
}

test('store failure refuses a request and never schedules clear', async ($, on) => {
  const h = await setup($, on, { storeFailure: true });
  const result = await h.request();
  expect(result.deny).toContain('not accepted');
  await h.end();
  await h.clock.settle();
  expect(h.calls.filter(x => x[0] === 'command')).toHaveLength(0);
});

test('an intercepted clear without a native transition is unconfirmed', async ($, on) => {
  const h = await setup($, on, { interceptClear: true });
  await h.request();
  await h.end();
  await h.clock.settle();
  expect(h.record().reset).toBe('unconfirmed');
  expect(h.calls.filter(x => x[0] === 'prompt')).toHaveLength(0);
});

test('a refused continuation preserves the confirmed reset', async ($, on) => {
  const h = await setup($, on, { dropPrompt: true });
  await h.request();
  await h.end();
  await h.clock.settle();
  expect(h.record().reset).toBe('confirmed');
  expect(h.record().continuation).toBe('failed');
  await $.turn.start({ turnId: 't2' });
  expect((await h.request()).deny).toContain('reset already completed');
});

test('new input before the timer cancels a pending clear', async ($, on) => {
  const h = await setup($, on);
  await h.request();
  await h.end();
  await $.prompt.submit({ text: 'Keep working here' });
  await h.clock.settle();
  expect(h.record().reset).toBe('cancelled');
  expect(h.calls.filter(x => x[0] === 'command')).toHaveLength(0);
});

test('a directory change before the timer cancels clear', async ($, on) => {
  const h = await setup($, on);
  await h.request();
  await h.end();
  h.move();
  await h.clock.settle();
  expect(h.record().reset).toBe('cancelled');
  expect(h.calls.filter(x => x[0] === 'command')).toHaveLength(0);
});

test('reload exposes cancelled request instead of replaying stale reset state', async ($, on) => {
  const h = await setup($, on, { saved: {
    ['reset:' + source]: { sourceSessionId: source, reset: 'requested', continuation: 'not_requested' },
  } });
  await h.end();
  await h.clock.settle();
  expect(h.record().reset).toBe('cancelled');
  expect(h.calls.filter(x => x[0] === 'command')).toHaveLength(0);
  const status = await $.command.run({ command: 'context-reset-status' });
  expect(JSON.parse(status.text).reset).toBe('cancelled');
});
