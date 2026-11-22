// Provider-disabled coding receiver in a VM: fake SDK and Host IPC, no network
// or access. Ported from the retired signed-subscription receiver test (F3):
// the pressure observation and checkpoint insertion of the provider-disabled
// fixture path still run through this receiver.
import test from 'node:test';
import assert from 'node:assert/strict';
import vm from 'node:vm';
import {EventEmitter} from 'node:events';
import {readFileSync} from 'node:fs';
async function fixtureReceiver(){
 let now=100,fetches=0,runtimes=0,accessReads=0;
 const frames=[],input=new EventEmitter();input.close=()=>{};
 const sessionId='00000000-0000-4000-8000-000000000001';
 const stream=()=>{const items=[];return {push:x=>items.push(x),end:()=>{},async *[Symbol.asyncIterator](){yield*items;}};};
 const sdk={
  SessionManager:{create:()=>({getSessionId:()=>sessionId})},
  SettingsManager:{inMemory:()=>({})},
  DefaultResourceLoader:class{async reload(){}},
  ModelRuntime:{async create(){runtimes++;return {getModel:()=>undefined,getModels:()=>[{id:'fixture-model',provider:'openai',contextWindow:8192}],async setRuntimeApiKey(){}};}},
  async createAgentSession(){const agent={streamFunction:async()=>stream()};
   const session={sessionId,agent,messages:[],pendingMessageCount:0,getContextUsage:()=>undefined,setAutoCompactionEnabled(){},setAutoRetryEnabled(){},subscribe(){},getActiveToolNames:()=>['workspace_exec'],dispose(){},abort(){},
    async sendCustomMessage(message){this.messages.push(message);},
    async prompt(){const response=await agent.streamFunction({provider:'openai',id:'fixture-model'},{},{});for await(const _ of response){}},
    sessionManager:{getEntries:()=>[],getHeader:()=>({})}};
   return {session};},
  async createAgentSessionServices(){return {modelRuntime:{},diagnostics:[]};},
  async createAgentSessionFromServices(){return sdk.createAgentSession();},
  async createAgentSessionRuntime(factory){await factory({});return {async newSession(){},async dispose(){}};},
 };
 const send=frame=>input.emit('line',JSON.stringify({sessionId,...frame}));
 const process={argv:['node','/receiver/receiver.mjs'],exitCode:0,stdin:{destroy(){}},stdout:{write(bytes){const f=JSON.parse(bytes);frames.push(f);
  if(f.type==='ready')queueMicrotask(()=>{send({type:'pressure_context'});send({type:'fixture_start',command:'true',pressureCheckpoint:{schema:'work-engine.pi-pressure-fixture.v2',session_id:sessionId,generation:'composition-regression',input_digest:'fixture-digest',pressure:'observation only'}});});
 }}};
 const context=vm.createContext({Buffer,process,Headers,URL,Date:class extends Date{static now(){return now;}},queueMicrotask,setTimeout:()=>1,clearTimeout(){},fetch:async()=>{fetches++;return {};}});
 const moduleFor=async values=>{const m=new vm.SyntheticModule(Object.keys(values),function(){for(const [key,value] of Object.entries(values))this.setExport(key,value);},{context});await m.link(()=>{});await m.evaluate();return m;};
 const modules={
  'node:http':{default:{createServer(){throw Error('fixture opened an HTTP sink');}}},
  'node:readline':{createInterface:()=>input},
  'node:fs':{existsSync:()=>false,readFileSync:p=>{if(p.startsWith('/auth/'))accessReads++;return '{}';},writeFileSync(){},openSync:()=>1,fsyncSync(){},closeSync(){},readdirSync:()=>[],lstatSync:()=>({}),mkdirSync(){throw Error('fixture created a directory');}},
  'node:crypto':{createHash:()=>({update(){return this;},digest:()=>'0'.repeat(64)})},
  'node:zlib':{zstdDecompressSync:b=>b},
  '/pi/node_modules/typebox/build/index.mjs':{Type:new Proxy({},{get:()=>()=>({})})},
  '/pi/node_modules/@earendil-works/pi-ai/dist/utils/event-stream.js':{createAssistantMessageEventStream:stream},
  '/pi/node_modules/@earendil-works/pi-coding-agent/dist/core/agent-session.js':{markRuntimeStreamFunction:f=>f},
  '/pi/node_modules/@earendil-works/pi-coding-agent/dist/index.js':sdk,
 };
 const receiver=new vm.SourceTextModule(readFileSync(new URL('../receiver/pi-coding.mjs',import.meta.url),'utf8'),{context,importModuleDynamically:async()=>{throw Error('no dynamic import in the fixture path');}});
 await receiver.link(name=>moduleFor(modules[name]??(()=>{throw Error('unexpected import '+name);})()));await receiver.evaluate();
 await new Promise(r=>setImmediate(r));
 return {fetches,runtimes,accessReads,frames};
}
test('provider-disabled receiver keeps pressure observation and checkpoint insertion without network or access',{skip:!vm.SourceTextModule},async()=>{
 const r=await fixtureReceiver();
 assert.equal(r.fetches,0);assert.equal(r.accessReads,0);assert.equal(r.frames.some(f=>f.type==='error'),false,JSON.stringify(r.frames));
 const observed=r.frames.filter(f=>f.type==='pressure_context');assert.equal(observed.length,1);assert.equal(observed[0].snapshot.idle,true);assert.equal(observed[0].snapshot.model.id,'fixture-model');
 const inserted=r.frames.find(f=>f.type==='pressure_inserted');assert.equal(inserted.generation,'composition-regression');assert.equal(inserted.messages[0].details.authority,'observation_only');
 assert.equal(r.frames.filter(f=>f.type==='pressure_request').length,1);assert.equal(r.frames.at(-1).provider,'disabled_fixture');
});
