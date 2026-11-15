// Provider-disabled Pi SDK interoperability adapter for the review-delivery experiment.
// Rust owns the contract, the attempt, the correspondence checks and the consumer. This file only
// drives the real SDK session (real prompt/auth/agent-loop/history) and reports what it observed.
// Fixture behaviour, not an OS sandbox: it reads only its argv paths, SDK files and platform runtime
// files, and writes nothing. Protocol: JSON lines on stdin/stdout, bounded frames, one start, one prompt.
import { pathToFileURL } from 'node:url';
import { createInterface } from 'node:readline';
import { createHash } from 'node:crypto';
import { readFileSync } from 'node:fs';

const argv = process.argv.slice(2);
const arg = (name) => { const i = argv.indexOf(name); return i >= 0 ? argv[i + 1] : undefined; };
const sdkRoot = arg('--sdk');
const fixtureRoot = arg('--fixture');
const cwd = arg('--cwd');
const control = arg('--control') ?? 'positive';
const MAX_FRAME = 1 << 20;
const counters = { ordinary_stream: 0, fetch: 0 };
// Any network entry is a terminal fixture refusal, counted.
globalThis.fetch = async () => { counters.fetch++; throw new Error('network forbidden in review-delivery fixture'); };

const emit = (value, then) => {
  let text = JSON.stringify(value);
  // Raw fixture injection: preserve both members on the real stdout pipe. Never use a JS
  // object to represent duplicates, since doing so would erase the counterexample itself.
  const duplicate = /^duplicate-(allocation|observation|context|history)-(equal|conflict)-(first|last)$/.exec(control);
  if (duplicate) {
    const [, object, mode, order] = duplicate;
    const target = object === 'allocation' && value.type === 'allocated' ? value
      : value.type === 'observation' ? (object === 'observation' ? value : object === 'context' ? value.context : object === 'history' ? value.history[0] : undefined) : undefined;
    if (target) {
      const key = object === 'allocation' ? 'sdk_version' : object === 'observation' ? 'request_count' : object === 'context' ? 'messages' : 'role';
      const member = JSON.stringify(key) + ':' + JSON.stringify(mode === 'equal' ? target[key] : typeof target[key] === 'number' ? target[key] + 1 : 'contradictory');
      const original = JSON.stringify(target);
      const modified = order === 'first' ? '{' + member + ',' + original.slice(1) : original.slice(0, -1) + ',' + member + '}';
      text = text.replace(original, modified);
    }
  }
  const bytes = text + '\n';
  if (Buffer.byteLength(bytes) > MAX_FRAME) throw new Error('output frame bound');
  process.stdout.write(bytes, then);
};
let refused = false;
const fail = (message) => { refused = true; try { emit({ type: 'error', message: String(message) }, () => process.exit(2)); } catch { process.exit(2); } };

try {
  if (!sdkRoot || !fixtureRoot || !cwd) throw new Error('missing --sdk/--fixture/--cwd');
  if (!process.env.HOME || !process.env.HOME.startsWith(fixtureRoot)) throw new Error('HOME is not an owned fixture directory');
  const dist = `${sdkRoot}/node_modules/@earendil-works/pi-coding-agent/dist`;
  const imp = (p) => import(pathToFileURL(p).href);
  const sdk = await imp(`${dist}/index.js`);
  const { AuthStorage } = await imp(`${dist}/core/auth-storage.js`);
  const { createAssistantMessageEventStream } = await imp(`${sdkRoot}/node_modules/@earendil-works/pi-ai/dist/utils/event-stream.js`);
  const { createAgentSession, createExtensionRuntime, ModelRuntime, SessionManager, SettingsManager } = sdk;
  const sdkVersion = JSON.parse(readFileSync(`${sdkRoot}/node_modules/@earendil-works/pi-coding-agent/package.json`, 'utf8')).version;

  // Constant fixture preamble: fixed experiment data, not a generated review instruction.
  const PREAMBLE = '{"classification":"synthetic fixture: review-delivery experiment; no authority; not a review instruction"}';
  const unsupported = (name) => () => { throw new Error(`inert loader refuses ${name}`); };
  class InertLoader {
    calls = {};
    constructor(runtime) { this.runtime = runtime; }
    #t(name) { this.calls[name] = (this.calls[name] ?? 0) + 1; }
    getExtensions() { this.#t('getExtensions'); return { extensions: [], errors: [], runtime: this.runtime }; }
    getSkills() { this.#t('getSkills'); return { skills: [], diagnostics: [] }; }
    getPrompts() { this.#t('getPrompts'); return { prompts: [], diagnostics: [] }; }
    getThemes() { this.#t('getThemes'); return { themes: [], diagnostics: [] }; }
    getAgentsFiles() { this.#t('getAgentsFiles'); return { agentsFiles: [] }; }
    getSystemPrompt() { this.#t('getSystemPrompt'); return PREAMBLE; }
    getSystemPromptSource() { this.#t('getSystemPromptSource'); return undefined; }
    getAppendSystemPrompt() { this.#t('getAppendSystemPrompt'); return []; }
    getAppendSystemPromptSources() { this.#t('getAppendSystemPromptSources'); return []; }
    extendResources() { this.#t('extendResources'); unsupported('extendResources')(); }
    async reload() { this.#t('reload'); }
  }
  const loader = new InertLoader(createExtensionRuntime());

  // Explicit in-memory credential store; no file store, no real credential.
  const credentials = AuthStorage.inMemory();
  const modelRuntime = await ModelRuntime.create({ credentials, modelsPath: null, allowModelNetwork: false, refreshOnCreate: false });
  // Ordinary provider entry is trapped: counted and refused.
  for (const name of ['streamSimple', 'stream']) {
    modelRuntime[name] = () => { counters.ordinary_stream++; throw new Error(`provider entry refused: ${name}`); };
  }
  // Selected model is catalogue metadata, never a served-model claim.
  const model = modelRuntime.getModels().find((m) => m.provider === 'openai');
  if (!model) throw new Error('fixture catalogue model absent');
  if (control !== 'missing-key') await modelRuntime.setRuntimeApiKey(model.provider, 'FAKE-FIXTURE-NOT-A-CREDENTIAL');

  const settingsManager = SettingsManager.inMemory({ compaction: { enabled: false }, retry: { enabled: false }, cacheWarming: 'off' });
  const sessionManager = SessionManager.inMemory(cwd);
  const { session } = await createAgentSession({
    cwd, agentDir: `${fixtureRoot}/agent`, modelRuntime, model, thinkingLevel: 'off',
    noTools: 'all', tools: [], resourceLoader: loader, sessionManager, settingsManager,
  });
  session.setAutoCompactionEnabled(false);
  session.setAutoRetryEnabled(false);

  emit({
    type: 'allocated', session_id: session.sessionId, sdk_version: sdkVersion, node_version: process.version,
    model: { provider: model.provider, id: model.id, api: model.api, standing: 'catalogue metadata; not a served-model claim' },
    system_prompt: session.systemPrompt, active_tools: session.getActiveToolNames(), loader: { kind: 'inert-fixture-loader', calls: { ...loader.calls } },
    preamble: PREAMBLE,
  }, control === 'die-after-allocation' ? () => process.exit(4) : undefined);

  let started = false, requests = 0, captured = null, promptError = null, lastObservation = null;
  // Fixture misbehaviour AFTER a valid observation: trailing bytes at the real pipe boundary,
  // a nonzero exit, a hang, an oversize frame or a flood past the total bound.
  const afterObservation = () => {
    const m = /^trail-(duplicate|malformed|error|invalid-utf8|incomplete)-(prompt|delayed)$/.exec(control);
    const finish = () => process.exit(0);
    if (m) {
      const write = () => {
        const bytes = {
          duplicate: Buffer.from(JSON.stringify(lastObservation) + '\n'),
          malformed: Buffer.from('{"type":"observation", not json\n'),
          error: Buffer.from(JSON.stringify({ type: 'error', message: 'late fixture error' }) + '\n'),
          'invalid-utf8': Buffer.from([0x7b, 0xff, 0xfe, 0x7d, 0x0a]),
          incomplete: Buffer.from('{"type":"observation","session_id":"cut'),
        }[m[1]];
        process.stdout.write(bytes, finish);
      };
      if (m[2] === 'prompt') write(); else setTimeout(write, 500);
      return;
    }
    if (control === 'exit-nonzero-after-observation') return process.exit(5);
    if (control === 'hang-after-observation') { setInterval(() => {}, 1000); return; }
    if (control === 'oversize-after-observation') return process.stdout.write(Buffer.alloc((1 << 20) + 4096, 0x61), finish);
    if (control === 'flood-after-observation') {
      const line = Buffer.from(JSON.stringify({ type: 'other', pad: 'x'.repeat(1000) }) + '\n');
      const chunk = Buffer.concat(Array.from({ length: 256 }, () => line));
      let left = 40; // ~10 MiB of valid small frames, past the 8 MiB total bound
      const pump = () => { if (left-- <= 0) return finish(); if (process.stdout.write(chunk)) pump(); else process.stdout.once('drain', pump); };
      return pump();
    }
    finish();
  };
  // Every host frame is adjudicated before an observation can exist: the observation waits until
  // the host ended its input, and readline delivers every received line before 'close'. A later
  // frame (e.g. a duplicate start) is therefore processed and refused first, however the host's
  // writes were grouped or read. The refusal names the host frame and the stdin reads seen.
  let hostFrames = 0, stdinReads = 0;
  process.stdin.on('data', () => { stdinReads++; });
  const input = createInterface({ input: process.stdin, crlfDelay: Infinity });
  const hostEnded = new Promise((resolve) => input.on('close', resolve));
  input.on('close', () => { if (!started) process.exit(2); });
  input.on('line', async (line) => {
    const hostFrame = ++hostFrames;
    try {
      if (Buffer.byteLength(line) > 8 * MAX_FRAME) throw new Error('input frame bound');
      const frame = JSON.parse(line);
      if (frame.type !== 'start') throw new Error('unexpected host frame');
      if (started) throw new Error(`duplicate start refused (host frame ${hostFrame}, ${stdinReads} stdin reads)`);
      if (frame.session_id !== session.sessionId) throw new Error('start names a session this adapter did not allocate');
      if (typeof frame.attempt_id !== 'string' || !Number.isSafeInteger(frame.generation)) throw new Error('start lacks attempt identity');
      if (typeof frame.contract !== 'string' || createHash('sha256').update(frame.contract).digest('hex') !== frame.contract_sha256) throw new Error('contract digest mismatch');
      if (typeof frame.result_text !== 'string') throw new Error('start lacks the synthetic result');
      started = true;
      if (control === 'clean-exit-no-observation') process.exit(0);
      if (control !== 'ordinary-stream') {
        // Deterministic provider-disabled response; the real prompt/auth/agent-loop still runs.
        session.agent.streamFunction = async (requestModel, context) => {
          requests++;
          if (requests > 1) throw new Error('second request refused');
          if (requestModel.provider !== model.provider || requestModel.id !== model.id) throw new Error('unexpected model selection');
          captured = JSON.parse(JSON.stringify({ messages: context.messages, tools: context.tools ?? [] }));
          if (control === 'die-during-prompt') process.exit(3);
          const message = {
            role: 'assistant', content: [{ type: 'text', text: frame.result_text }], api: model.api, provider: model.provider, model: model.id,
            usage: { input: 1, output: 1, cacheRead: 0, cacheWrite: 0, totalTokens: 2, cost: { input: 0, output: 0, cacheRead: 0, cacheWrite: 0, total: 0 } },
            stopReason: 'stop', timestamp: Date.now(),
          };
          const out = createAssistantMessageEventStream();
          out.push({ type: 'done', reason: 'stop', message });
          out.end();
          return out;
        };
      }
      try { await session.prompt(frame.contract); } catch (error) { promptError = String(error?.message ?? error); }
      await session.waitForIdle();
      const observationFrame = {
        type: 'observation', session_id: session.sessionId, attempt_id: frame.attempt_id, generation: frame.generation,
        request_count: requests, context: captured, history: JSON.parse(JSON.stringify(session.messages)),
        active_tools: session.getActiveToolNames(), terminal_idle: session.isIdle, counters: { ...counters },
        prompt_error: promptError, system_prompt: session.systemPrompt, loader_calls: { ...loader.calls },
      };
      await hostEnded;
      if (refused) return;
      lastObservation = observationFrame;
      emit(observationFrame, () => afterObservation(frame));
    } catch (error) { fail(error?.message ?? error); }
  });
} catch (error) { fail(error?.message ?? error); }
