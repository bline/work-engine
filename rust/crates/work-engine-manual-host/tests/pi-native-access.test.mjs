import test from 'node:test';
import assert from 'node:assert/strict';
import {createNativeAccessStore} from '../receiver/pi-native-access.mjs';
test('access-only subscription store cannot refresh or select another provider',async()=>{
 const grant={access:'FAKE_ACCESS',expires_ms:2000000,account_id:'FAKE_ACCOUNT'};
 let time=1000000;const store=createNativeAccessStore(grant,()=>time);
 assert.equal(await store.read('openai'),undefined);
 const credential=await store.read('openai-codex');assert.equal(credential.type,'oauth');assert.equal(credential.refresh,'');
 let callbackCalled=false;await assert.rejects(store.modify('openai-codex',async()=>{callbackCalled=true;}));
 assert.equal(callbackCalled,false);await assert.rejects(store.delete('openai-codex'));
 time=1700000;await assert.rejects(store.read('openai-codex'));
});
test('rejects extra key material and near-expiry projection before network',()=>{
 assert.throws(()=>createNativeAccessStore({access:'fake',account_id:'fake',expires_ms:1300000},()=>1000000));
 assert.throws(()=>createNativeAccessStore({access:'fake',account_id:'fake',expires_ms:2000000,refresh_token:'never'},()=>1000000));
});
test('pinned Pi runtime consumes fake access-only store without network or refresh',{skip:!process.env.S10_PI_SDK},async()=>{
 const {pathToFileURL}=await import('node:url');
 const {ModelRuntime}=await import(pathToFileURL(process.env.S10_PI_SDK+'/dist/index.js').href);
 const header=Buffer.from(JSON.stringify({alg:'RS256',typ:'JWT'})).toString('base64url');
 const payload=Buffer.from(JSON.stringify({exp:Math.floor(Date.now()/1000)+3600,'https://api.openai.com/auth':{chatgpt_account_id:'FAKE_ACCOUNT'}})).toString('base64url');
 const access=header+'.'+payload+'.'+Buffer.from('FAKE_SIGNATURE').toString('base64url');
 const grant={access,expires_ms:Date.now()+3600000,account_id:'FAKE_ACCOUNT'};
 const credentials=createNativeAccessStore(grant);
 const previousFetch=globalThis.fetch;globalThis.fetch=()=>{throw Error('network forbidden in fake SDK interoperability test');};
 try{
  const runtime=await ModelRuntime.create({credentials,modelsPath:null,allowModelNetwork:false,refreshOnCreate:false});
  const model=runtime.getModel('openai-codex','gpt-6-luna');assert.equal(model.id,'gpt-6-luna');
  assert.equal(model.contextWindow,272000);assert.equal(model.maxTokens,128000);
  const result=await runtime.getAuth(model);assert.equal(result.auth.apiKey,access);
  assert.equal((await runtime.checkAuth('openai-codex')).type,'oauth');
  assert.equal(runtime.getProvider('openai-codex').auth.oauth.isSubscription,true);
  // refreshOnCreate:false intentionally leaves UI snapshot flags unpopulated.
 }finally{globalThis.fetch=previousFetch;}
});
test('raw terminal adapter preserves missing usage/status instead of SDK zeros',async()=>{
 const {rawTerminalEvidence}=await import('../receiver/pi-native-access.mjs');
 for(const response of [{model:'gpt-6-luna'},{model:'gpt-6-luna',status:'queued',usage:{}},{status:'completed',usage:{input_tokens:1}}]){
  const evidence=rawTerminalEvidence({type:'response.completed',response});
  assert.equal(evidence.status,response.status);assert.equal(evidence.usage.output_tokens,undefined);
 }
 assert.equal(rawTerminalEvidence({type:'response.in_progress',response:{}}),undefined);
});
test('pinned SDK stream preserves raw missing/status cases despite normalized defaults',{skip:!process.env.S10_PI_SDK},async()=>{
 const {pathToFileURL}=await import('node:url');const {ModelRuntime}=await import(pathToFileURL(process.env.S10_PI_SDK+'/dist/index.js').href);
 const {rawTerminalEvidence}=await import('../receiver/pi-native-access.mjs');
 const payload=Buffer.from(JSON.stringify({exp:Math.floor(Date.now()/1000)+3600,'https://api.openai.com/auth':{chatgpt_account_id:'FAKE_ACCOUNT'}})).toString('base64url');
 const access='eyJhbGciOiJSUzI1NiJ9.'+payload+'.ZmFrZQ';const credentials=createNativeAccessStore({access,expires_ms:Date.now()+3600000,account_id:'FAKE_ACCOUNT'});
 const runtime=await ModelRuntime.create({credentials,modelsPath:null,allowModelNetwork:false,refreshOnCreate:false});const model=runtime.getModel('openai-codex','gpt-6-luna');
 for(const terminal of [{},{status:'completed'},{status:'completed',usage:{input_tokens:0}},{status:'in_progress',usage:{input_tokens:0,output_tokens:0,total_tokens:0}}]){
  let raw;let calls=0;const response={id:'fake-response',model:'gpt-6-luna',output:[],...terminal};
  const stream=runtime.streamSimple(model,{messages:[{role:'user',content:'fake stream only',timestamp:Date.now()}]},{transport:'sse',maxRetries:0,fetch:async()=>{calls++;return new Response('data: '+JSON.stringify({type:'response.completed',response})+'\n\ndata: [DONE]\n\n',{status:200,headers:{'Content-Type':'text/event-stream'}});},onProviderStreamEvent:data=>{raw=rawTerminalEvidence(data)||raw;}});
  for await(const ignored of stream){}
  assert.equal(calls,1);assert.ok(raw);assert.equal(raw.status,terminal.status);assert.equal(raw.usage.output_tokens,terminal.usage?.output_tokens);
 }
});
