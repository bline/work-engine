import test from 'node:test';
import {zstdCompressSync} from 'node:zlib';
import assert from 'node:assert/strict';
import {fixedSyntheticFetch,syntheticToken} from './fixtures/pi-subscription-preload.mjs';
const url='https://chatgpt.com/backend-api/codex/responses';
const init=()=>({method:'POST',redirect:'error',headers:{authorization:'Bearer '+syntheticToken()},body:JSON.stringify({model:'gpt-6-luna',stream:true})});
test('synthetic fixed-fetch rejects all other endpoints/credentials/methods without network',async()=>{
 for(const [u,i] of [['https://example.com',init()],[url,{...init(),method:'GET'}],[url,{...init(),redirect:'follow'}],[url,{...init(),headers:{authorization:'Bearer not-synthetic'}}]])await assert.rejects(fixedSyntheticFetch('fixture')(u,i));
});
test('SSE raw evidence is explicit and bounded; no SDK normalized zero stands in for usage',async()=>{
 const fetch=fixedSyntheticFetch('fixture');
 for(let n=0;n<2;n++){
  const response=await fetch(url,init());assert.equal(response.status,200);
  const events=(await response.text()).trim().split('\n\n').map(s=>JSON.parse(s.slice(6)));
  const raw=events.at(-1);assert.equal(raw.type,'response.completed');assert.equal(raw.response.status,'completed');
  assert.deepEqual(raw.response.usage.input_tokens,10);assert.equal(raw.response.model,'gpt-6-luna');
  assert.equal(raw.response.output[0].type,n===0?'function_call':'message');
 }
 await assert.rejects(fetch(url,init()));
});

test('synthetic endpoint decodes actual SDK zstd request without changing production transport',async()=>{
 const request=init();request.body=zstdCompressSync(request.body);request.headers['content-encoding']='zstd';
 const response=await fixedSyntheticFetch('fixture')(url,request);assert.equal(response.status,200);
 const bad=init();bad.headers['content-encoding']='gzip';await assert.rejects(fixedSyntheticFetch('fixture')(url,bad));
});

// Execute the unchanged fixed receiver in a VM: fake SDK/IPC/clock, no network.
// Grant flush is an IPC observation only. The fetch counter is the dispatch proof.
import vm from 'node:vm';
import {EventEmitter} from 'node:events';
import {readFileSync} from 'node:fs';
async function receiverExpiry({job=200,authority=150,grantAt=100,consumeAt=100,prepareAt=100,initializeAt=100,runtimeAt=100,legacyPressure=false}){
 let now=100,fetches=0,runtimes=0,accessReads=0,flushedAt;
 const frames=[],input=new EventEmitter();input.close=()=>{};
 const sessionId='00000000-0000-4000-8000-000000000001';
 const stream=()=>{const items=[];return {push:x=>items.push(x),end:()=>{},async *[Symbol.asyncIterator](){yield*items;}};};
 const sdk={
  SessionManager:{create:()=>({getSessionId:()=>sessionId})},
  SettingsManager:{inMemory:()=>({})},
  DefaultResourceLoader:class{async reload(){}},
  ModelRuntime:{async create(){runtimes++;now=runtimeAt;return {getModel:()=>({id:'gpt-6-luna',api:'openai-codex-responses',provider:'openai-codex',baseUrl:'https://chatgpt.com/backend-api'}),getModels:()=>[{id:'fixture-model',provider:'openai',contextWindow:8192}],async setRuntimeApiKey(){}};}},
  async createAgentSession(){const agent={streamFunction:async(_m,_c,options)=>{await Promise.resolve();now=prepareAt;await options.fetch('https://chatgpt.com/backend-api/codex/responses',{});return stream();}};
   const session={sessionId,agent,messages:[],pendingMessageCount:0,getContextUsage:()=>undefined,setAutoCompactionEnabled(){},setAutoRetryEnabled(){},subscribe(){},getActiveToolNames:()=>['workspace_exec'],dispose(){},abort(){},
    async sendCustomMessage(message){this.messages.push(message);},
    async prompt(){const response=await agent.streamFunction({provider:'openai-codex',id:'gpt-6-luna'},{},{});for await(const _ of response){}},
    sessionManager:{getEntries:()=>[],getHeader:()=>({})}};
   return {session};}
 };
 const config={schema:'work-engine.pi-coding-signed-subscription.v1',provider:'openai-codex',model:'gpt-6-luna',deadline:job/1000,maxRequests:2,maxContextBytes:1000};
 const send=frame=>input.emit('line',JSON.stringify({sessionId,...frame}));
 const process={exitCode:0,stdin:{destroy(){}},stdout:{write(bytes){const f=JSON.parse(bytes);frames.push(f);
  if(f.type==='allocated')queueMicrotask(()=>{now=initializeAt;send({type:'initialize',deadline:job/1000,consumptionDeadlineMs:authority});});
  if(f.type==='ready')queueMicrotask(()=>{
   if(legacyPressure){send({type:'pressure_context'});send({type:'fixture_start',command:'true',pressureCheckpoint:{schema:'work-engine.pi-pressure-fixture.v2',session_id:sessionId,generation:'composition-regression',input_digest:'fixture-digest',pressure:'observation only'}});}
   else send({type:'task_start'});
  });
  if(f.type==='provider_request')queueMicrotask(()=>{now=grantAt;flushedAt=now;send({type:'provider_grant',id:f.id,consumptionDeadlineMs:Math.min(authority,job)});now=consumeAt;});
 }}};
 const context=vm.createContext({Buffer,process,Date:class extends Date{static now(){return now;}},queueMicrotask,setTimeout:()=>1,clearTimeout(){},fetch:async()=>{fetches++;return {};}});
 const moduleFor=async values=>{const m=new vm.SyntheticModule(Object.keys(values),function(){for(const [key,value] of Object.entries(values))this.setExport(key,value);},{context});await m.link(()=>{});await m.evaluate();return m;};
 const native=await moduleFor({createNativeAccessStore:()=>({}),rawTerminalEvidence:()=>null});
 const modules={
  'node:readline':{createInterface:()=>input},
  'node:fs':{existsSync:p=>!legacyPressure&&p==='/profile/job.json',readFileSync:p=>{if(p==='/auth/access.json')accessReads++;return JSON.stringify(p==='/profile/job.json'?config:{});},writeFileSync(){},openSync:()=>1,fsyncSync(){},closeSync(){}},
  '/pi/node_modules/typebox/build/index.mjs':{Type:{Object:()=>({}),String:()=>({})}},
  '/pi/node_modules/@earendil-works/pi-ai/dist/utils/event-stream.js':{createAssistantMessageEventStream:stream},
  '/pi/node_modules/@earendil-works/pi-coding-agent/dist/index.js':sdk,
 };
 const receiver=new vm.SourceTextModule(readFileSync(new URL('../receiver/pi-coding.mjs',import.meta.url),'utf8'),{context,importModuleDynamically:async()=>native});
 await receiver.link(name=>moduleFor(modules[name]));await receiver.evaluate();
 return {fetches,runtimes,accessReads,frames,flushedAt,job,authority};
}
for(const [name,options] of [
 ['grant flushed valid, consumed after earlier signed expiry',{consumeAt:151}],
 ['job expires during grant await',{job:140,authority:180,consumeAt:141}],
 ['SDK asynchronous preparation crosses signed expiry',{prepareAt:151}],
 ['SDK asynchronous preparation crosses job expiry',{job:140,authority:180,prepareAt:141}],
])test(name,{skip:!vm.SourceTextModule},async()=>{const r=await receiverExpiry(options);assert.ok(r.flushedAt<Math.min(r.job,r.authority));assert.equal(r.fetches,0);assert.equal(r.frames.filter(f=>f.type==='provider_request').length,1);assert.equal(r.frames.filter(f=>f.type==='provider_complete').length,0);});
test('valid consumption reaches fixed fetch once',{skip:!vm.SourceTextModule},async()=>{assert.equal((await receiverExpiry({})).fetches,1);});
test('delayed initialization refuses before credential read or ModelRuntime',{skip:!vm.SourceTextModule},async()=>{const r=await receiverExpiry({initializeAt:151});assert.equal(r.fetches,0);assert.equal(r.runtimes,0);assert.equal(r.accessReads,0);});
test('initialization SDK preparation crossing expiry refuses before ready',{skip:!vm.SourceTextModule},async()=>{const r=await receiverExpiry({runtimeAt:151});assert.equal(r.fetches,0);assert.equal(r.runtimes,1);assert.equal(r.frames.some(f=>f.type==='ready'),false);});
test('composed receiver retains provider-disabled pressure observation and checkpoint insertion',{skip:!vm.SourceTextModule},async()=>{
 const r=await receiverExpiry({legacyPressure:true});
 assert.equal(r.fetches,0);assert.equal(r.accessReads,0);assert.equal(r.frames.some(f=>f.type==='error'),false);
 const observed=r.frames.filter(f=>f.type==='pressure_context');assert.equal(observed.length,1);assert.equal(observed[0].snapshot.idle,true);assert.equal(observed[0].snapshot.model.id,'fixture-model');
 const inserted=r.frames.find(f=>f.type==='pressure_inserted');assert.equal(inserted.generation,'composition-regression');assert.equal(inserted.messages[0].details.authority,'observation_only');
 assert.equal(r.frames.filter(f=>f.type==='pressure_request').length,1);assert.equal(r.frames.at(-1).provider,'disabled_fixture');
});
