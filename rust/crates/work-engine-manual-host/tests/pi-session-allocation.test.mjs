// Actual pinned SDK initialization-order proof; not signed authority or live coding.
// Run in the existing pinned Pi image, network-none, without any auth mount.
import assert from 'node:assert/strict';
import {existsSync, mkdirSync, writeFileSync, readFileSync} from 'node:fs';
import {SessionManager, SettingsManager, DefaultResourceLoader, ModelRuntime, createAgentSession} from '/pi/node_modules/@earendil-works/pi-coding-agent/dist/index.js';
let networkAttempts = 0;
globalThis.fetch = async () => { networkAttempts++; throw Error('provider disabled in allocation proof'); };
const fakeAccessPath = '/export/fake-access.json';
assert.equal(existsSync(fakeAccessPath), false);
assert.equal(existsSync('/auth/access.json'), false);
let modelRuntimeConstructed = false;
let accessReads = 0;
// This is the actual SDK factory, with its own native UUID, not randomUUID here.
const manager = SessionManager.create('/workspace', '/export/sessions');
const allocated = manager.getSessionId();
assert.match(allocated, /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/);
assert.equal(manager.getHeader().id, allocated);
assert.equal(modelRuntimeConstructed, false);
assert.equal(accessReads, 0);
assert.equal(existsSync(fakeAccessPath), false);
const allocation = {schema:'work-engine.pi-session-allocation-proof.v1', type:'allocated', sessionId:allocated, modelRuntimeConstructed, accessReads, provider:'disabled_fixture'};
writeFileSync('/export/allocation.json', JSON.stringify(allocation)+'\n', {flag:'wx',mode:0o600});
// Test-only phase release stands in for scheduling, NOT an authority owner.
// Production must seal the allocation under its signed current registry first.
writeFileSync(fakeAccessPath, JSON.stringify({fixture_only:true, access:'FAKE-NEVER-A-CREDENTIAL'}), {flag:'wx',mode:0o600});
accessReads++;
assert.equal(JSON.parse(readFileSync(fakeAccessPath,'utf8')).fixture_only, true);
const settingsManager = SettingsManager.inMemory({compaction:{enabled:false},retry:{enabled:false},cacheWarming:'off'});
const resourceLoader = new DefaultResourceLoader({cwd:'/workspace',agentDir:'/tmp/pi-empty',settingsManager,noExtensions:true,disabledBuiltinExtensions:['mcp'],noSkills:true,noPromptTemplates:true,noThemes:true,noContextFiles:true});
await resourceLoader.reload();
modelRuntimeConstructed = true;
const modelRuntime = await ModelRuntime.create({authPath:'/tmp/fake-auth.json',modelsPath:null,allowModelNetwork:false,refreshOnCreate:false});
const model = modelRuntime.getModels().find(m=>m.provider==='openai');
assert.ok(model);
await modelRuntime.setRuntimeApiKey(model.provider,'FAKE-FIXTURE-NOT-A-CREDENTIAL');
let session;
try {
 ({session}=await createAgentSession({cwd:'/workspace',agentDir:'/tmp/pi-empty',settingsManager,resourceLoader,modelRuntime,model,thinkingLevel:'off',sessionManager:manager,noTools:'builtin',tools:[],customTools:[]}));
 assert.equal(session.sessionManager, manager);
 assert.equal(session.sessionId, allocated);
 assert.equal(manager.getSessionId(), allocated);
 assert.equal(session.isStreaming, false);
 assert.equal(networkAttempts, 0);
 const proof={schema:'work-engine.pi-session-allocation-proof.v1', classification:'actual pinned SDK; fake credentials; no prompt/inference; NOT authority or Host integration', allocated, agentSessionId:session.sessionId, sameManager:session.sessionManager===manager, accessReadsBeforeAllocation:allocation.accessReads, modelRuntimeBeforeAllocation:allocation.modelRuntimeConstructed, networkAttempts, postReleaseFakeAccessReads:accessReads, nativeSessionFilePersisted:existsSync(manager.getSessionFile())};
 writeFileSync('/export/proof.json', JSON.stringify(proof)+'\n', {flag:'wx',mode:0o600});
 console.log(JSON.stringify(proof));
} finally { session?.dispose(); }
