// Receiver response-stream adapter against a scripted host. No SDK, Docker or network.
import {test} from 'node:test';
import assert from 'node:assert/strict';
import {createResponseStream} from '../receiver/pi-native-tools.mjs';

const binding = {session: 's', profile: 'pi-native-inmemory-sse-v1', generation: 1};
// Scripted host: `reply(frame)` returns zero or more host frames for each sent frame.
function host(reply) {
  const sent = [], queue = [], waiters = [];
  const deliver = f => { const w = waiters.shift(); w ? w(f) : queue.push(f); };
  return {
    sent,
    deliver,
    sendRaw: async v => { sent.push(v); for (const f of reply(v)) deliver(f); },
    read: () => queue.length ? Promise.resolve(queue.shift()) : new Promise(r => waiters.push(r)),
  };
}
const frame = (v, type, data = {}) => ({type, binding, sequence: v.sequence, stream: v.stream, ordinal: v.ordinal, ...data});
const b64 = s => Buffer.from(s).toString('base64');

test('chunks then end close the body in order', async () => {
  const h = host(v => v.type === 'pull'
    ? [v.ordinal < 3 ? frame(v, 'response-chunk', {body: b64(`c${v.ordinal};`)}) : frame(v, 'response-end')]
    : []);
  const s = createResponseStream({id: {binding, sequence: 1, stream: 1}, sendRaw: h.sendRaw, read: h.read});
  assert.equal(await new Response(s.body).text(), 'c1;c2;');
  assert.deepEqual(h.sent.map(v => [v.type, v.ordinal]), [['pull', 1], ['pull', 2], ['pull', 3], ['ack', 4]]);
});

test('release of an undrained earlier response keeps its own stream identity', async () => {
  const h = host(v => v.type === 'pull' ? [frame(v, 'response-chunk', {body: b64('x')})]
    : v.type === 'cancel' ? [frame(v, 'response-cancelled')] : []);
  let settled = [];
  const first = createResponseStream({id: {binding, sequence: 1, stream: 1}, sendRaw: h.sendRaw, read: h.read, onSettled: () => settled.push(1)});
  const reader = first.body.getReader();
  await reader.read();
  // A second fetch's response exists before the first is released.
  createResponseStream({id: {binding, sequence: 1, stream: 2}, sendRaw: h.sendRaw, read: h.read});
  await first.release();
  const cancel = h.sent.find(v => v.type === 'cancel');
  assert.equal(cancel.stream, 1);
  assert.equal(cancel.sequence, 1);
  assert.equal(cancel.ordinal, 2);
  assert.deepEqual(settled, [1]);
});

test('cancel overtakes an outstanding pull and consumes the host confirmation', async () => {
  let pendingPull;
  const h = host(v => {
    if (v.type === 'pull') { pendingPull = v; return []; }  // host withholds the chunk
    if (v.type === 'cancel') return [frame(v, 'response-cancelled')];
    return [];
  });
  let idle = 0;
  const s = createResponseStream({id: {binding, sequence: 2, stream: 1}, sendRaw: h.sendRaw, read: h.read, idleMs: 20, onIdle: () => idle++});
  const reader = s.body.getReader();
  const pending = reader.read();
  await new Promise(r => setTimeout(r, 40));
  assert.equal(idle, 1, 'idle abort fired while the pull was outstanding');
  await reader.cancel();
  assert.deepEqual(h.sent.map(v => [v.type, v.ordinal]), [['pull', 1], ['cancel', 2]]);
  assert.equal(pendingPull.ordinal, 1);
  assert.equal((await pending).done, true);
});

test('chunk racing the cancel is discarded until confirmation', async () => {
  const h = host(v => v.type === 'pull' ? [] : v.type === 'cancel'
    ? [frame({...v, ordinal: 1}, 'response-chunk', {body: b64('late')}), frame(v, 'response-cancelled')] : []);
  const s = createResponseStream({id: {binding, sequence: 1, stream: 1}, sendRaw: h.sendRaw, read: h.read});
  const reader = s.body.getReader();
  const pending = reader.read();
  await new Promise(r => setImmediate(r));
  await reader.cancel();
  assert.equal((await pending).done, true);
});

test('foreign frame and failure surface as errors', async () => {
  const h = host(v => v.type === 'pull' ? [{...frame(v, 'response-chunk', {body: b64('x')}), stream: 9}] : []);
  const s = createResponseStream({id: {binding, sequence: 1, stream: 1}, sendRaw: h.sendRaw, read: h.read});
  await assert.rejects(new Response(s.body).text(), /foreign_frame/);
  const g = host(v => v.type === 'pull' ? [frame(v, 'response-failure', {code: 'upstream_truncated'})] : []);
  const t = createResponseStream({id: {binding, sequence: 1, stream: 1}, sendRaw: g.sendRaw, read: g.read});
  await assert.rejects(new Response(t.body).text(), /relay_upstream_truncated/);
});

test('cancel racing a terminal end waits for confirmation and sends no ack', async () => {
  // Host answers the pending pull with end, then confirms the crossing cancel.
  let pull;
  const h = host(v => {
    if (v.type === 'pull') { pull = v; return []; }
    if (v.type === 'cancel') return [frame(pull, 'response-end'), frame(v, 'response-cancelled')];
    return [];
  });
  let settled = 0;
  const s = createResponseStream({id: {binding, sequence: 1, stream: 1}, sendRaw: h.sendRaw, read: h.read, onSettled: () => settled++});
  const reader = s.body.getReader();
  const pending = reader.read();
  await new Promise(r => setImmediate(r));
  await Promise.race([reader.cancel(), new Promise((_, j) => setTimeout(() => j(Error('release hung')), 1000))]);
  assert.equal((await pending).done, true);
  assert.deepEqual(h.sent.map(v => [v.type, v.ordinal]), [['pull', 1], ['cancel', 2]]);
  assert.equal(settled, 1);
  // A later release is a no-op.
  await s.release();
  assert.equal(h.sent.length, 2);
});

test('release after a terminal frame sends nothing more', async () => {
  const h = host(v => v.type === 'pull' ? [frame(v, 'response-end')] : []);
  const s = createResponseStream({id: {binding, sequence: 1, stream: 1}, sendRaw: h.sendRaw, read: h.read});
  assert.equal(await new Response(s.body).text(), '');
  await s.release();
  assert.deepEqual(h.sent.map(v => v.type), ['pull', 'ack']);
});
