// Receiver response adapter against a scripted host. No SDK, Docker or network.
import {test} from 'node:test';
import assert from 'node:assert/strict';
import {openResponse} from '../receiver/pi-native-tools.mjs';

const binding = {session: 's', profile: 'pi-native-inmemory-sse-v1', generation: 1};
// Scripted host: `reply(frame)` returns host frames for each sent frame.
function host(reply) {
  const sent = [], queue = [], waiters = [];
  const deliver = f => { const w = waiters.shift(); w ? w(f) : queue.push(f); };
  return {
    sent, deliver,
    sendRaw: async v => { sent.push(v); for (const f of reply(v)) deliver(f); },
    read: () => queue.length ? Promise.resolve(queue.shift()) : new Promise(r => waiters.push(r)),
  };
}
const id = (stream = 1, sequence = 1) => ({binding, sequence, stream});
const frame = (v, type, data = {}) => ({type, binding, sequence: v.sequence, stream: v.stream, ordinal: v.ordinal, ...data});
const head = (i = id(), status = 200) => ({type: 'response-head', binding, sequence: i.sequence, stream: i.stream, status});
const b64 = s => Buffer.from(s).toString('base64');
const types = h => h.sent.map(v => [v.type, v.ordinal]);
const tick = () => new Promise(r => setImmediate(r));

test('head, chunks and end produce the body and one ack', async () => {
  const h = host(v => v.type === 'pull'
    ? [v.ordinal < 3 ? frame(v, 'response-chunk', {body: b64(`c${v.ordinal};`)}) : frame(v, 'response-end')] : []);
  h.deliver(head());
  const r = openResponse({id: id(), sendRaw: h.sendRaw, read: h.read});
  const res = await r.response;
  assert.equal(res.status, 200);
  assert.equal(await res.text(), 'c1;c2;');
  assert.deepEqual(types(h), [['pull', 1], ['pull', 2], ['pull', 3], ['ack', 4]]);
});

test('abort while awaiting the head sends cancel(1) and rejects as aborted', async () => {
  const h = host(v => v.type === 'cancel' ? [frame(v, 'response-cancelled')] : []);
  const ac = new AbortController();
  const r = openResponse({id: id(), sendRaw: h.sendRaw, read: h.read, signal: ac.signal});
  await tick();
  ac.abort();
  await assert.rejects(r.response, /Request was aborted/);
  assert.deepEqual(types(h), [['cancel', 1]]);
});

test('head crossing a head-phase cancel is discarded until confirmation', async () => {
  const h = host(v => v.type === 'cancel' ? [head(), frame(v, 'response-cancelled')] : []);
  const ac = new AbortController();
  const r = openResponse({id: id(), sendRaw: h.sendRaw, read: h.read, signal: ac.signal});
  await tick();
  ac.abort();
  await assert.rejects(r.response, /Request was aborted/);
  await r.release();
  assert.deepEqual(types(h), [['cancel', 1]]);
});

test('unavailable route and head failure reject; failure is acknowledged once', async () => {
  const h = host(() => []);
  h.deliver({type: 'response-unavailable', binding, sequence: 1, stream: 1, code: 'attempt_limit'});
  await assert.rejects(openResponse({id: id(), sendRaw: h.sendRaw, read: h.read}).response, /route_unavailable_attempt_limit/);
  assert.equal(h.sent.length, 0);
  const g = host(() => []);
  g.deliver({type: 'response-failure', binding, sequence: 1, stream: 1, ordinal: 0, code: 'upstream_version'});
  await assert.rejects(openResponse({id: id(), sendRaw: g.sendRaw, read: g.read}).response, /relay_upstream_version/);
  assert.deepEqual(types(g), [['ack', 1]]);
});

test('release of an undrained earlier response keeps its own stream identity', async () => {
  const h = host(v => v.type === 'pull' ? [frame(v, 'response-chunk', {body: b64('x')})]
    : v.type === 'cancel' ? [frame(v, 'response-cancelled')] : []);
  let settled = 0;
  h.deliver(head(id(1)));
  const first = openResponse({id: id(1), sendRaw: h.sendRaw, read: h.read, onSettled: () => settled++});
  const reader = (await first.response).body.getReader();
  await reader.read();
  h.deliver(head(id(2)));
  const second = openResponse({id: id(2), sendRaw: h.sendRaw, read: h.read});
  await second.response;
  await first.release();
  const cancel = h.sent.find(v => v.type === 'cancel');
  assert.deepEqual([cancel.stream, cancel.sequence, cancel.ordinal], [1, 1, 2]);
  assert.equal(settled, 1);
});

test('cancel overtakes an outstanding pull; idle timer fires while waiting', async () => {
  const h = host(v => v.type === 'cancel' ? [frame(v, 'response-cancelled')] : []);
  let idle = 0;
  h.deliver(head(id(1, 2)));
  const r = openResponse({id: id(1, 2), sendRaw: h.sendRaw, read: h.read, idleMs: 20, onIdle: () => idle++});
  const reader = (await r.response).body.getReader();
  const pending = reader.read();
  await new Promise(res => setTimeout(res, 40));
  assert.equal(idle, 1);
  await reader.cancel();
  assert.deepEqual(types(h), [['pull', 1], ['cancel', 2]]);
  assert.equal((await pending).done, true);
});

test('cancel racing a terminal end waits for confirmation and sends no ack', async () => {
  let pull;
  const h = host(v => {
    if (v.type === 'pull') { pull = v; return []; }
    if (v.type === 'cancel') return [frame(pull, 'response-end'), frame(v, 'response-cancelled')];
    return [];
  });
  h.deliver(head());
  const r = openResponse({id: id(), sendRaw: h.sendRaw, read: h.read});
  const reader = (await r.response).body.getReader();
  const pending = reader.read();
  await tick();
  await Promise.race([reader.cancel(), new Promise((_, j) => setTimeout(() => j(Error('release hung')), 1000))]);
  assert.equal((await pending).done, true);
  assert.deepEqual(types(h), [['pull', 1], ['cancel', 2]]);
  await r.release();
  assert.equal(h.sent.length, 2);
});

test('foreign frame and relayed failure surface as errors', async () => {
  const h = host(v => v.type === 'pull' ? [{...frame(v, 'response-chunk', {body: b64('x')}), stream: 9}] : []);
  h.deliver(head());
  await assert.rejects((await openResponse({id: id(), sendRaw: h.sendRaw, read: h.read}).response).text(), /foreign_frame/);
  const g = host(v => v.type === 'pull' ? [frame(v, 'response-failure', {code: 'upstream_truncated'})] : []);
  g.deliver(head());
  await assert.rejects((await openResponse({id: id(), sendRaw: g.sendRaw, read: g.read}).response).text(), /relay_upstream_truncated/);
  assert.deepEqual(types(g), [['pull', 1], ['ack', 2]]);
});

test('unexpected body frame errors and still closes with a cancel handshake', async () => {
  const h = host(v => v.type === 'pull' ? [frame(v, 'response-weird')] : v.type === 'cancel' ? [frame(v, 'response-cancelled')] : []);
  h.deliver(head());
  const r = openResponse({id: id(), sendRaw: h.sendRaw, read: h.read});
  await assert.rejects((await r.response).text(), /stream_frame/);
  await tick();
  assert.deepEqual(types(h), [['pull', 1], ['cancel', 2]]);
});
