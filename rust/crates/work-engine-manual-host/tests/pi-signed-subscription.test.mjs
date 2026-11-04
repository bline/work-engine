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
