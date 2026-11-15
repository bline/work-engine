// TEST ONLY. Every turn.step is answered here; this makes no model requests.
// Load only into an isolated, disposable CLI session. Writes probe.json in cwd.
let step = 0;
let sourceSessionId = null;
const observations = [];

async function record($, value) {
  observations.push(value);
  await $.fs.write('probe.json', JSON.stringify(observations, null, 2));
}

export function register(on) {
  on('session.start', async ($, e, next) => {
    sourceSessionId = await $.session.id();
    await record($, { event: 'start', sourceSessionId, interactive: e.isInteractive });
    return next(e);
  });

  on('session.end', async ($, e, next) => {
    await record($, { event: 'end', sessionId: e.sessionId, reason: e.reason });
    return next(e);
  });

  on('turn.step', async function* ($, e) {
    const sessionId = await $.session.id();
    const phase = step++ % 3;
    if (phase === 0) {
      sourceSessionId = sessionId;
      yield { kind: 'tool', index: 0, id: 'native-reset-probe', name: 'mcp__work-engine-context__new_context' };
      yield { kind: 'input', index: 0, json: '{}' };
      return { turnId: e.turnId, index: e.index, answer: '', toolUses: [{ name: 'mcp__work-engine-context__new_context', input: {} }], stopReason: 'tool_use', usage: null };
    }
    if (phase === 1) {
      await record($, { event: 'requesting_turn_answer', sessionId });
      yield { kind: 'text', index: 0, text: 'Reset requested; ending the test turn.' };
      return { turnId: e.turnId, index: e.index, answer: 'Reset requested; ending the test turn.', toolUses: [], stopReason: 'end_turn', usage: null };
    }
    const messages = await $.session.messages();
    const texts = JSON.stringify(messages);
    await record($, {
      event: 'recovery_turn', sessionId, sourceSessionId,
      changedSession: sessionId !== sourceSessionId,
      oldMarkerPresent: texts.includes('PRE_CLEAR_ONLY_MARKER'),
      predecessorPointerPresent: texts.includes('context-recovery:' + sourceSessionId),
      windWalkerPresent: texts.includes('$wind-walker'),
    });
    yield { kind: 'text', index: 0, text: 'NATIVE_CONTEXT_PROBE_FINISHED' };
    return { turnId: e.turnId, index: e.index, answer: 'NATIVE_CONTEXT_PROBE_FINISHED', toolUses: [], stopReason: 'end_turn', usage: null };
  });

  on('turn.complete', async ($, e, next) => {
    const result = await next(e);
    if (step > 0 && step % 3 === 0) {
      $.clock.after(0, async () => {
        const status = await $.command.run({ command: 'context-reset-status' });
        await record($, { event: 'adapter_status', status: JSON.parse(status.text) });
      });
    }
    return result;
  });
}
