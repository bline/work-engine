// Provider-free capture of the pinned actual Pi adapter's serialized HTTP request.
import {pathToFileURL} from 'node:url';
import {readFileSync} from 'node:fs';
import assert from 'node:assert/strict';
import {createHash} from 'node:crypto';
const root=process.argv[2];
const {streamSimple}=await import(pathToFileURL(`${root}/node_modules/@earendil-works/pi-ai/dist/api/openai-codex-responses.js`));
const catalog=JSON.parse(readFileSync(`${root}/node_modules/@earendil-works/pi-ai/dist/providers/data/openai-codex.json`));
const model=catalog['openai-codex-responses']['chat:gpt-6.1-sol'];
assert.equal(model.id,'gpt-6.1-sol');
const account='deterministic-fixture-payer';
const {codingSessionOptions}=await import('../../receiver/pi-native-access.mjs');
const config={schema:'work-engine.pi-coding-lifecycle-sol-high.v1',model:'gpt-6.1-sol',solHigh:{schema:'work-engine.sol-high.v1',reasoning_effort:'high',payer_account_id:account}};
const options=codingSessionOptions(config,{account_id:account});
for(const mutation of [{...config,model:'gpt-6-luna'},{...config,solHigh:{...config.solHigh,reasoning_effort:'medium'}},{...config,solHigh:{...config.solHigh,payer_account_id:'foreign'}}])assert.throws(()=>codingSessionOptions(mutation,{account_id:account}));
const jwt=`${Buffer.from(JSON.stringify({alg:'RS256'})).toString('base64url')}.${Buffer.from(JSON.stringify({'https://api.openai.com/auth':{chatgpt_account_id:account}})).toString('base64url')}.ZmFrZQ`;
let capture,calls=0;
const stream=streamSimple(model,{systemPrompt:'Offline wire qualification',messages:[{role:'user',content:'Capture request only',timestamp:0}]},{apiKey:jwt,reasoning:options.thinkingLevel,transport:'sse',maxRetries:0,timeoutMs:1000,cacheRetention:'none',fetch:async(url,init)=>{
 calls++;const headers=Object.fromEntries(new Headers(init.headers));
 const wire=typeof init.body==='string'?Buffer.from(init.body):Buffer.from(init.body);
 const decoded=headers['content-encoding']==='zstd'?process.getBuiltinModule('node:zlib').zstdDecompressSync(wire):wire;
 capture={url,headers,wire_body_base64:wire.toString('base64'),wire_body_sha256:createHash('sha256').update(wire).digest('hex'),body:JSON.parse(decoded.toString())};
 throw Error('DETERMINISTIC_TRANSPORT_STOP');
}});
for await(const event of stream) { /* SDK errors are expected after the captured request. */ }
assert.equal(calls,1);
assert.equal(capture.url,'https://chatgpt.com/backend-api/codex/responses');
assert.equal(capture.body.model,'gpt-6.1-sol');
assert.equal(capture.body.reasoning.effort,'high');
assert.equal(capture.headers['chatgpt-account-id'],account);
delete capture.headers.authorization;
console.log(JSON.stringify({schema:'work-engine.sol-high-wire-capture.v1',provider_execution:false,transport:'deterministic-substitute',request:capture}));
