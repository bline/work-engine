// Claude Mods is the native host for this adapter. Semantic checkpoints remain
// owned by the agent's existing durable continuation and chatboard recovery note.
let pending = null;
let activeTurn = null;
let accepting = false;
let sawClear = false;
let clearing = false;

function inFlight(request) {
  return request && ['pending_turn_end', 'scheduled', 'requested'].includes(request.reset);
}

async function save($, request, delta) {
  Object.assign(request, delta);
  await $.store.set('reset:' + request.sourceSessionId, request);
}

async function cancel($, reason) {
  if (!clearing && inFlight(pending)) await save($, pending, { reset: 'cancelled', detail: reason });
}

function recoveryPrompt(request) {
  return 'A native context reset has completed. This is recovery of the existing ' +
    'assignment, not a new grant or a transfer of collection ownership. ' +
    'Predecessor session: ' + request.sourceSessionId + '. Repository: ' + request.cwd + '. ' +
    'Read AGENTS.md and invoke $wind-walker (or read skills/wind-walker/SKILL.md), ' +
    'then follow docs/contracts/context-recovery.md. Find the predecessor recovery ' +
    'note on chatboard topic context-recovery:' + request.sourceSessionId +
    '; page the board beyond the newest 20 messages if needed. Recover through ' +
    'the durable owners named there. Verify your current session and transcript, ' +
    'rebind context-pressure observation, and reconcile retained reviews, children ' +
    'and process collection before continuing the authorized objective. ' +
    'The adapter has not verified checkpoint contents or semantic recovery. ' +
    'If continuation-critical meaning is missing, report the recovery failure ' +
    'and stop dependent work.';
}

async function replaceContext($, request) {
  try {
    if (request !== pending || request.reset !== 'scheduled') return;
    if (await $.session.id() !== request.sourceSessionId ||
        await $.session.cwd() !== request.cwd || activeTurn !== null) {
      await cancel($, 'Session, directory or active turn changed before reset.');
      return;
    }
    sawClear = false;
    await save($, request, { reset: 'requested' });
    if (request !== pending || request.reset !== 'requested' || activeTurn !== null) return;
    // Our own command/prompt hooks are skipped for calls this plugin raises.
    // Serialize incoming prompts until native clear and recovery submission settle.
    clearing = true;
    const cleared = await $.command.run({ command: 'clear' });
    if (cleared?.deny || cleared?.drop) throw new Error(cleared.deny ?? cleared.drop);
    const targetSessionId = await $.session.id();
    if (!sawClear || targetSessionId === request.sourceSessionId) {
      await save($, request, { reset: 'unconfirmed', detail: 'Native clear was not confirmed; recover or inspect before another request.' });
      await $.ui.status('Context reset unconfirmed. Use /context-reset-status.');
      return;
    }
    await save($, request, { reset: 'confirmed', targetSessionId, continuation: 'pending' });
    await $.store.set('source:' + targetSessionId, request.sourceSessionId);
    const outcome = await $.prompt.submit({ text: recoveryPrompt(request) });
    if (outcome?.drop) throw new Error(outcome.drop);
    await save($, request, { continuation: 'submitted' });
  } catch (error) {
    const detail = String(error?.message ?? error).slice(0, 1000);
    try {
      // A command can throw after the clear happened. Reconcile that observation
      // before labelling a failed command as a failed reset.
      const targetSessionId = await $.session.id();
      const confirmed = request.reset === 'confirmed' ||
        (sawClear && targetSessionId !== request.sourceSessionId);
      await save($, request, confirmed
        ? { reset: 'confirmed', targetSessionId, continuation: 'failed', detail }
        : { reset: 'failed', detail });
      if (confirmed) await $.store.set('source:' + targetSessionId, request.sourceSessionId);
    } finally {
      await $.ui.status('Context transition needs attention. Predecessor ' +
        request.sourceSessionId + '. Use /context-reset-status; recover before another reset.');
    }
  } finally {
    clearing = false;
  }
}

export function register(on) {
  on('session.start', async ($, e, next) => {
    const sessionId = await $.session.id();
    const source = await $.store.get('source:' + sessionId) ?? sessionId;
    pending = await $.store.get('reset:' + source) ?? null;
    // A reload cancels timers. Never replay a destructive action from stale state.
    await cancel($, 'Plugin reloaded; prepare recovery and explicitly request again.');
    await $.tool.register({
      name: 'new_context',
      description: 'Request native context replacement after this main-conversation turn ends. ' +
        'Before calling, follow Work Engine native-context recovery: preserve continuation-critical ' +
        'meaning with its durable owners and publish a recovery note on chatboard topic ' +
        'context-recovery:<verified current session ID>. Call with {} and end this turn promptly. ' +
        'The result acknowledges a pending request; it does not verify a checkpoint or confirm a reset. ' +
        'After native clear, the plugin submits a recovery prompt bound to the predecessor. ' +
        'Native subagent contexts are unsupported; never use this to reset a parent.',
      inputSchema: { type: 'object', properties: {}, additionalProperties: false },
      isDeferred: false,
    });
    await $.command.register({ name: 'context-reset-status', description: 'Show the native context transition observation for this session.' });
    return next(e);
  });

  // This host's tool.call precedes its permission check. Consult that check
  // explicitly before answering the registered tool in place of the engine.
  on('tool.call', { tool: 'mcp__work-engine-context__new_context' }, async ($, e) => {
    if (e.agentId !== undefined) return { deny: 'Native subagent reset is unsupported; this command clears the main conversation.' };
    const permission = await $.tool.check({ tool: e.tool, input: {} });
    if (permission.decision !== 'allow') return { deny: 'Context reset requires tool permission: ' + (permission.reason ?? permission.decision) };
    if (accepting) return { deny: 'A context request is being recorded; wait for its result.' };
    if (activeTurn === null) return { deny: 'Request context replacement from an active main-conversation turn.' };
    accepting = true;
    try {
      if (inFlight(pending)) return { result: JSON.stringify(pending) };
      const sourceSessionId = await $.session.id();
      const cwd = await $.session.cwd();
      if (pending?.reset === 'confirmed' && pending.continuation !== 'submitted') {
        return { deny: 'The reset already completed. Recover this context using /context-reset-status; do not clear again.' };
      }
      const request = {
        schema: 'work-engine-claude-context-v1', requestId: e.tool_use_id,
        sourceSessionId, cwd, turnId: activeTurn,
        checkpoint: 'agent_owned_unverified', reset: 'pending_turn_end',
        targetSessionId: null, continuation: 'not_requested', detail: null,
      };
      await save($, request, {});
      await $.store.set('source:' + sourceSessionId, sourceSessionId);
      pending = request;
      return { result: JSON.stringify(request) };
    } finally {
      accepting = false;
    }
  }).catch(($, e, next) => ({ deny: 'Context request was not accepted: ' + next.error.message }));

  on('turn.start', async ($, e, next) => {
    if (e.agentId === undefined) {
      if (inFlight(pending) && pending.turnId !== e.turnId) await cancel($, 'Another main turn started before reset.');
      activeTurn = e.turnId;
    }
    return next(e);
  });

  on('prompt.submit', async ($, e, next) => {
    if (clearing) return { drop: 'Native context replacement is in progress. Retry this input after recovery, or stop the pending reset.' };
    // New input invalidates a request that has not submitted native clear yet.
    await cancel($, 'New input arrived before reset. Prepare recovery again.');
    return next(e);
  });

  on('turn.complete', async ($, e, next) => {
    const result = await next(e);
    if (e.agentId !== undefined) return result;
    activeTurn = null;
    if (pending?.reset !== 'pending_turn_end' || pending.turnId !== e.turnId) return result;
    if (e.isAborted || e.reason !== 'answer') {
      await cancel($, 'Requesting turn did not finish normally.');
      return result;
    }
    await save($, pending, { reset: 'scheduled' });
    const request = pending;
    // command.run is refused in hooks blocking a turn. The timer leaves that stack.
    $.clock.after(0, () => replaceContext($, request));
    return result;
  });

  on('command.run', { command: 'clear' }, async ($, e, next) => {
    // This observes other callers only; Mods skips our hook for our own call.
    if (clearing) return { text: 'A native context replacement is already in progress.' };
    await cancel($, 'A separate clear command superseded the request.');
    return next(e);
  }).catch(() => ({ text: 'Context clear withheld because its session binding could not be checked.' }));

  on('session.end', async ($, e, next) => {
    if (pending?.reset === 'requested' && e.reason === 'clear' &&
        e.sessionId === pending.sourceSessionId) sawClear = true;
    else await cancel($, 'Session ended or changed before the requested clear.');
    return next(e);
  });

  on('command.run', { command: 'context-reset-status' }, async ($) => {
    const sessionId = await $.session.id();
    const observed = pending && [pending.sourceSessionId, pending.targetSessionId].includes(sessionId) ? pending : null;
    try {
      const source = observed?.sourceSessionId ?? await $.store.get('source:' + sessionId) ?? sessionId;
      const request = await $.store.get('reset:' + source);
      return { text: JSON.stringify(observed
        ? { ...observed, persistence: JSON.stringify(observed) === JSON.stringify(request) ? 'recorded' : 'unconfirmed' }
        : request ?? { reset: 'not_recorded', sessionId }, null, 2) };
    } catch (error) {
      return { text: JSON.stringify({ ...observed, sessionId, persistence: 'unavailable', error: String(error?.message ?? error) }, null, 2) };
    }
  });
}
