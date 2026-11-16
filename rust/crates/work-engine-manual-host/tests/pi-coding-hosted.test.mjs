// Hosted coding receiver in a VM: fake SDK and Host IPC, no network or access.
// The receiver's request leaves only as a provider_fetch frame and its
// response arrives only through the Host-relayed stream (openResponse).
import test from 'node:test';
import assert from 'node:assert/strict';
import vm from 'node:vm';
import {EventEmitter} from 'node:events';
import {readFileSync} from 'node:fs';
import {createHash} from 'node:crypto';
import {zstdCompressSync, zstdDecompressSync} from 'node:zlib';
import {openResponse} from '../receiver/pi-native-tools.mjs';

const binding = {session: 'hosted-session', attempt: 'job-attempt', generation: 1, profile: 'work-engine.pi-coding-hosted.v1'};
const sessionId = '00000000-0000-4000-8000-000000000001';
const requestBody = '{"model":"gpt-6-luna","stream":true}';
const sse = 'data: {"type":"response.completed"}\n\n';

function queueStream() {
  const items = [];
  let ended = false, wake = null;
  const poke = () => { if (wake) { const w = wake; wake = null; w(); } };
  return {
    push: x => { items.push(x); poke(); },
    end: () => { ended = true; poke(); },
    async *[Symbol.asyncIterator]() {
      for (;;) {
        if (items.length) { yield items.shift(); continue; }
        if (ended) return;
        await new Promise(r => { wake = r; });
      }
    },
  };
}

async function runHosted({grantStream = {binding, sequence: 1, stream: 1}, zstd = false} = {}) {
  const frames = [], input = new EventEmitter();
  input.close = () => {};
  const seen = {globalFetches: 0, accessReads: 0, accessImports: 0, credentials: null, responses: []};
  const send = frame => queueMicrotask(() => input.emit('line', JSON.stringify(frame)));
  const env = (type, data) => ({type, binding, sequence: 1, stream: 1, ...data});
  let pulls = 0;
  const session = {
    sessionId, messages: [], isStreaming: false, isCompacting: false, pendingMessageCount: 0, autoCompactionEnabled: false,
    agent: {
      // The unchanged SDK stream function: it uses only the injected fetch.
      streamFunction: async (_m, _c, options) => {
        const response = await options.fetch('https://chatgpt.com/backend-api/codex/responses',
          zstd ? {method: 'POST', headers: {authorization: 'Bearer marker', 'content-type': 'application/json', 'content-encoding': 'zstd'}, body: new Uint8Array(zstdCompressSync(requestBody))}
            : {method: 'POST', headers: {authorization: 'Bearer marker', 'content-type': 'application/json'}, body: requestBody});
        const text = await response.text();
        seen.responses.push({status: response.status, text});
        const out = queueStream();
        out.push({type: 'done', reason: 'stop', message: {role: 'assistant', content: [{type: 'text', text}], stopReason: 'stop', usage: {input: 1, output: 1}}});
        out.end();
        return out;
      },
    },
    getContextUsage: () => undefined, setAutoCompactionEnabled() {}, setAutoRetryEnabled() {}, subscribe() {},
    getActiveToolNames: () => ['workspace_exec'], setActiveToolsByName() {}, abort() {}, dispose() {},
    sessionManager: {getBranch: () => [], getHeader: () => ({}), getLeafId: () => null, getEntries: () => []},
    async prompt() {
      const stream = await this.agent.streamFunction({provider: 'openai-codex', id: 'gpt-6-luna', contextWindow: 272000}, {messages: []}, {});
      for await (const _ of stream) {}
    },
  };
  const model = {id: 'gpt-6-luna', api: 'openai-codex-responses', provider: 'openai-codex', baseUrl: 'https://chatgpt.com/backend-api', contextWindow: 272000, maxTokens: 128000, cost: {}};
  const sdk = {
    SessionManager: {create: () => ({getSessionId: () => sessionId})},
    SettingsManager: {inMemory: () => ({})},
    DefaultResourceLoader: class { async reload() {} },
    ModelRuntime: {async create(options) { seen.credentials = options.credentials; return {getModel: () => model}; }},
    async createAgentSessionServices() { return {modelRuntime: {}, diagnostics: []}; },
    async createAgentSessionFromServices() { return {session}; },
    async createAgentSessionRuntime(factory) { await factory({}); return {async newSession() {}, async dispose() {}}; },
    async createAgentSession() { return {session}; },
  };
  const config = {schema: 'work-engine.pi-coding-hosted.v1', provider: 'openai-codex', model: 'gpt-6-luna', deadline: 10, maxRequests: 4, maxContextBytes: 24000};
  const process = {argv: ['node', '/receiver/receiver.mjs', '--lifecycle-bridge'], exitCode: 0, stdin: {destroy() {}}, stdout: {write(bytes) {
    const f = JSON.parse(bytes);
    frames.push(f);
    if (f.type === 'ready') send({type: 'bridge_prompt', sessionId, invocationId: 'coding', turnCorrelation: 'turn:coding', prompt: 'p', consumptionDeadlineMs: 9000});
    if (f.type === 'coding_provider_request') send({type: 'bridge_provider', sessionId, id: f.id, mode: 'live', consumptionDeadlineMs: 9000, headTimeoutMs: 60000, ...(grantStream ? {stream: grantStream} : {})});
    if (f.type === 'provider_fetch') send(env('response-head', {status: 200}));
    if (f.type === 'pull') {
      pulls++;
      send(pulls === 1 ? env('response-chunk', {ordinal: f.ordinal, body: Buffer.from(sse).toString('base64')}) : env('response-end', {ordinal: f.ordinal}));
    }
    if (f.type === 'coding_history') send({type: 'bridge_close', sessionId, consumptionDeadlineMs: 9000});
  }}};
  const context = vm.createContext({Buffer, process, Headers, URL, queueMicrotask,
    Date: class extends Date { static now() { return 1000; } },
    setTimeout: () => 1, clearTimeout() {},
    fetch: async () => { seen.globalFetches++; throw Error('network'); }});
  const moduleFor = async values => {
    const m = new vm.SyntheticModule(Object.keys(values), function () { for (const [k, v] of Object.entries(values)) this.setExport(k, v); }, {context});
    await m.link(() => {}); await m.evaluate(); return m;
  };
  const modules = {
    'node:readline': {createInterface: () => input},
    'node:crypto': {createHash},
    'node:zlib': {zstdDecompressSync},
    'node:fs': {
      existsSync: p => p === '/profile/job.json',
      readFileSync: p => { if (p.startsWith('/auth')) seen.accessReads++; return JSON.stringify(p === '/profile/job.json' ? config : {}); },
      writeFileSync() {}, openSync: () => 1, fsyncSync() {}, closeSync() {}, readdirSync: () => [], lstatSync: () => ({}),
    },
    '/pi/node_modules/typebox/build/index.mjs': {Type: new Proxy({}, {get: () => () => ({})})},
    '/pi/node_modules/@earendil-works/pi-ai/dist/utils/event-stream.js': {createAssistantMessageEventStream: queueStream},
    '/pi/node_modules/@earendil-works/pi-coding-agent/dist/core/agent-session.js': {markRuntimeStreamFunction() {}},
    '/pi/node_modules/@earendil-works/pi-coding-agent/dist/index.js': sdk,
  };
  const dynamic = async specifier => {
    if (specifier === '/receiver/pi-native-tools.mjs') return moduleFor({openResponse});
    seen.accessImports++;
    return moduleFor({createNativeAccessStore: () => ({}), rawTerminalEvidence: () => null, codingSessionOptions: () => ({})});
  };
  const receiver = new vm.SourceTextModule(readFileSync(new URL('../receiver/pi-coding.mjs', import.meta.url), 'utf8'), {context, importModuleDynamically: dynamic});
  await receiver.link(name => moduleFor(modules[name]));
  await receiver.evaluate();
  for (let i = 0; i < 50 && !frames.some(f => f.type === 'complete' || f.type === 'error'); i++) await new Promise(r => setImmediate(r));
  return {frames, seen, exitCode: process.exitCode};
}

test('hosted receiver dispatches only through the Host relay, without network or access', {skip: !vm.SourceTextModule}, async () => {
  const {frames, seen, exitCode} = await runHosted();
  assert.equal(frames.some(f => f.type === 'error'), false, JSON.stringify(frames.filter(f => f.type === 'error')));
  assert.equal(exitCode, 0);
  assert.equal(frames.at(-1).type, 'complete');
  assert.deepEqual([seen.globalFetches, seen.accessReads, seen.accessImports], [0, 0, 0]);
  const access = await seen.credentials.read('openai-codex');
  assert.equal(access.accountId, 'nonsecret-marker');
  assert.match(access.access, /^syntactic\..*\.nonsecret$/);
  await assert.rejects(seen.credentials.modify());
  const fetches = frames.filter(f => f.type === 'provider_fetch');
  assert.equal(fetches.length, 1);
  assert.deepEqual([fetches[0].id, fetches[0].binding, fetches[0].sequence, fetches[0].stream], [1, binding, 1, 1]);
  assert.equal(fetches[0].url, 'https://chatgpt.com/backend-api/codex/responses');
  assert.equal(Buffer.from(fetches[0].body, 'base64').toString(), requestBody);
  assert.deepEqual(seen.responses, [{status: 200, text: sse}]);
  assert.deepEqual(frames.filter(f => ['pull', 'ack'].includes(f.type)).map(f => [f.type, f.ordinal]), [['pull', 1], ['pull', 2], ['ack', 3]]);
  // Settlement evidence is the Host's: the container sends no observations.
  assert.equal(frames.some(f => f.type === 'provider_observation'), false);
  assert.equal(frames.filter(f => f.type === 'provider_complete').length, 1);
});

test('hosted receiver sends the zstd-compressed SDK body to the Host as plain bounded JSON', {skip: !vm.SourceTextModule}, async () => {
  const {frames} = await runHosted({zstd: true});
  assert.equal(frames.at(-1).type, 'complete', JSON.stringify(frames.filter(f => f.type === 'error')));
  const [fetch] = frames.filter(f => f.type === 'provider_fetch');
  assert.equal(Buffer.from(fetch.body, 'base64').toString(), requestBody);
  assert.equal(fetch.headers['content-encoding'], undefined);
});

test('hosted grant without a stream identity fails before any fetch frame', {skip: !vm.SourceTextModule}, async () => {
  const {frames, seen} = await runHosted({grantStream: null});
  assert.equal(frames.some(f => f.type === 'provider_fetch'), false);
  assert.equal(seen.globalFetches, 0);
  assert.equal(frames.some(f => f.type === 'complete'), false);
});
