// Exact Pi1.5 interoperability. Rust owns admission, budgets, execution and custody.
import {createAgentSession,createAgentSessionRuntime,createAgentSessionServices,createAgentSessionFromServices,DefaultResourceLoader,ModelRuntime,SessionManager,SettingsManager} from '/pi/node_modules/@earendil-works/pi-coding-agent/dist/index.js';
import {createAssistantMessageEventStream} from '/pi/node_modules/@earendil-works/pi-ai/dist/utils/event-stream.js';
import {markRuntimeStreamFunction} from '/pi/node_modules/@earendil-works/pi-coding-agent/dist/core/agent-session.js';
import {createHash} from 'node:crypto';
import {Type} from '/pi/node_modules/typebox/build/index.mjs';
import {createInterface} from 'node:readline';
import {zstdDecompressSync} from 'node:zlib';
import {openSync,writeFileSync,fsyncSync,closeSync,existsSync,readFileSync,readdirSync,lstatSync,mkdirSync} from 'node:fs';
import http from 'node:http';
const bridge=process.argv.includes('--lifecycle-bridge');
const fixtureSigned=existsSync('/auth/signed-fixture.json');
const live=existsSync('/profile/job.json');
const claimsTools=process.argv.includes('--claims-tools');
const CLAIMS_SCHEMA='work-engine.pi-coding-lifecycle-claims.v1';
const CLAIMS_NAMES=['claims_discover','claims_resolve','claims_evidence','claims_currency'];
if(claimsTools&&(!process.argv.includes('--lifecycle-bridge')||existsSync('/profile/job.json')||existsSync('/auth/signed-fixture.json')))throw Error('claims tools require the provider-disabled lifecycle fixture bridge');
const config=live?JSON.parse(readFileSync('/profile/job.json','utf8')):null;
// Hosted transport: no network or access material here; the Host dispatches
// each request and relays the response stream (pull/cancel/ack).
// Hosted analysis: same transport and lifecycle bridge; no shell, only the
// Host-served read-only snapshot tools.
const analysis=live&&config.schema==='work-engine.pi-analysis-hosted.v1';
const hosted=live&&(config.schema==='work-engine.pi-coding-hosted.v1'||analysis);
const SNAPSHOT_NAMES=['snapshot_list','snapshot_read'];
const FRAME_BYTES=analysis?config.frameBytes:65536;
const MAX_BODY=analysis?config.maxBodyBytes:45000;
const MAX_REQUESTS=analysis?config.maxRequests:4;
// claude-sdk: Pi's provider is the pinned pi-claude-subscription extension,
// whose Claude Code child process sends its Messages requests to a loopback
// forwarder here. Each request becomes one provider_fetch under the current
// grant's stream identity; the Host dispatches it with real access. The
// container holds only a dummy credential.
const claudeSdk=hosted&&config.provider==='claude-sdk';
const CLAUDE_MODELS=['claude-haiku-5-5'];
const EXTENSION='/pi/node_modules/pi-claude-subscription';
const ANTHROPIC_BASE='https://api.anthropic.com';
// Claude Code's own retries and fallbacks stay off: every upstream request is
// one Host-admitted attempt (a second request under one grant is refused here).
const CLAUDE_CODE_ENV={CLAUDE_CODE_MAX_RETRIES:'0',CLAUDE_CODE_DISABLE_NONSTREAMING_FALLBACK:'1',CLAUDE_CODE_NO_MODEL_FALLBACK:'1',CLAUDE_CODE_DISABLE_MODEL_ACCESS_FALLBACK:'1',CLAUDE_CODE_DISABLE_REFUSAL_FALLBACK:'1',CLAUDE_CODE_DISABLE_REFUSAL_RETRY:'1',CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC:'1',NO_PROXY:'127.0.0.1,localhost',TMPDIR:'/tmp'};
const DUMMY_TOKEN=kind=>`sk-ant-${kind}01-${'D'.repeat(95)}`;
const DUMMY_CREDENTIALS={claudeAiOauth:{accessToken:DUMMY_TOKEN('oat'),refreshToken:DUMMY_TOKEN('ort'),expiresAt:4102444800000,refreshTokenExpiresAt:4102444800000,scopes:['user:file_upload','user:inference','user:mcp_servers','user:plugins','user:privacy_consents:write','user:profile','user:sessions:claude_code'],subscriptionType:'max',subscriptionTypeSource:'sign_in',rateLimitTier:'default_claude_max_5x'}};
// The hosted profile is the only live profile; credential-mounted live
// profiles are retired. It always runs on the lifecycle bridge.
if(live&&(!hosted||!bridge))throw Error('only the hosted live profile on the lifecycle bridge is supported');
const liveBridge=bridge&&live&&hosted;
if(bridge&&fixtureSigned)throw Error('bridge is provider-disabled only');
if(hosted)globalThis.fetch=async()=>{throw Error('network forbidden in hosted profile');};
if(bridge&&!liveBridge)globalThis.fetch=async()=>{throw Error('network forbidden in coding bridge');};
if(fixtureSigned&&live)throw Error('fixture/live conflict');
const signed=fixtureSigned;
if(fixtureSigned)globalThis.fetch=async()=>{throw Error('network forbidden in signed fixture');};
const schema=bridge?(liveBridge?config.schema:(claimsTools?CLAIMS_SCHEMA:'work-engine.pi-coding-lifecycle-fixture.v1')):fixtureSigned?'work-engine.pi-coding-signed-fixture.v1':'work-engine.pi-coding-fixture.v1';
let runtime,operation,bridgeCalls=0,bridgeResets=0,bridgeActive=false,bridgeWake,liveBridgeFailed=false;
const bridgeQueue=[];
const bridgeNext=()=>bridgeQueue.length?Promise.resolve(bridgeQueue.shift()):new Promise(resolve=>{bridgeWake=resolve;});
let session,manager,signedDeadline,pressureCheckpoint,model,pressureContextRequested=false;
// Deadlines arrive only on the private Host IPC channel. They restrict use;
// they do not replace the Host's signed current/effect/reservation checks.
const consumptionDeadline=(frame,upper)=>{
 const value=frame.consumptionDeadlineMs;
 if(!Number.isSafeInteger(value)||value<=0)throw Error('consumption deadline absent');
 return Math.min(value,upper);
};
const checkConsumption=until=>{
 if(!Number.isSafeInteger(until)||Date.now()>=until)throw Error('consumption deadline expired');
};
let initializeResolve;
const initialization=new Promise(resolve=>{initializeResolve=resolve;});
const emit=value=>{
 const bytes=JSON.stringify({schema,sessionId:session?.sessionId??manager?.getSessionId(),...(fixtureSigned?{classification:'fixture provider/signing/identity; actual SDK runtime'}:{}),...value})+'\n';
 if(Buffer.byteLength(bytes)>FRAME_BYTES)throw Error('output frame bound');
 process.stdout.write(bytes);
};
// Signed fixture only: /export is a size-limited tmpfs the Host cannot read, so its regular files are
// sent in-band, with explicit bounds and a final manifest, before completion is reported.
const exportBundle=()=>{
 const MAX_TOTAL=1048576,MAX_FILES=16,MAX_PART=24576,MAX_PATH=128;
 const files=[];
 const walk=(dir,rel,depth)=>{
  if(depth>6)throw Error('export depth');
  for(const name of readdirSync(dir).sort()){
   const path=rel?`${rel}/${name}`:name;const full=`${dir}/${name}`;const st=lstatSync(full);
   if(st.isDirectory())walk(full,path,depth+1);
   else if(st.isFile())files.push({path,full,size:st.size});
   else throw Error('export special file');
  }
 };
 walk('/export','',1);
 const total=files.reduce((n,f)=>n+f.size,0);
 if(files.length===0||files.length>MAX_FILES||total>MAX_TOTAL)throw Error('export bounds');
 files.sort((a,b)=>a.path<b.path?-1:a.path>b.path?1:0);
 let seq=0;const lines=[];
 for(const f of files){
  if(f.path.length>MAX_PATH||!/^[A-Za-z0-9._\/-]+$/.test(f.path))throw Error('export path');
  const bytes=readFileSync(f.full);if(bytes.length!==f.size)throw Error('export size changed');
  const sha=createHash('sha256').update(bytes).digest('hex');lines.push(`${f.path}\t${f.size}\t${sha}\n`);
  let offset=0;
  do{
   const chunk=bytes.subarray(offset,offset+MAX_PART);
   emit({type:'export_part',seq:seq++,path:f.path,offset,total:bytes.length,sha256:sha,data:chunk.toString('base64')});
   offset+=chunk.length;
  }while(offset<bytes.length);
 }
 emit({type:'export_final',files:files.length,bytes:total,manifest_sha256:createHash('sha256').update(lines.join('')).digest('hex')});
};
let startResolve;
const start=new Promise(resolve=>{startResolve=resolve;});
if(bridge)startResolve=null;
const pending=new Map();
const grants=new Map();
// Host response frames for the one open hosted stream, read in order.
const RESPONSE_FRAMES=['response-head','response-chunk','response-end','response-failure','response-unavailable','response-cancelled'];
const hostedFrames={open:false,queue:[],waiting:null,
 push(f){if(!this.open)throw Error('response frame without stream');if(this.waiting){const w=this.waiting;this.waiting=null;w(f);}else this.queue.push(f);},
 read(){if(this.waiting)throw Error('concurrent_read');return this.queue.length?Promise.resolve(this.queue.shift()):new Promise(r=>{this.waiting=r;});}};
// claude-sdk forwarder: at most one request per grant, only while the grant is
// open and no stream is; anything else is answered here and never leaves.
let forwardSlot=null,openForwarded=null,forwarderRefused=0;
// Every locally answered request is counted and reported at completion; the
// Host completes a job only if there were none.
const forwardRefuse=(res,status,message)=>{forwarderRefused++;const b=JSON.stringify({type:'error',error:{type:'forwarder_refused',message}});res.writeHead(status,{'content-type':'application/json','content-length':Buffer.byteLength(b)});res.end(b);};
const HOP=['host','connection','keep-alive','content-length','accept-encoding','transfer-encoding','proxy-connection','te','upgrade','expect'];
const forwarder=claudeSdk?http.createServer((req,res)=>{
 const chunks=[];let size=0,over=false;
 req.on('data',c=>{size+=c.length;if(size>MAX_BODY){if(!over){over=true;forwardRefuse(res,413,'request bound');}}else chunks.push(c);});
 req.on('end',async()=>{
  if(over)return;
  const slot=forwardSlot;
  if(req.method!=='POST'||!/^\/v1\/messages(\?beta=true)?$/.test(req.url))return forwardRefuse(res,404,'only POST /v1/messages');
  if(!slot||slot.used||hostedFrames.open||Date.now()>=slot.until)return forwardRefuse(res,503,'no open provider grant');
  const headers={};
  for(const [k,v] of Object.entries(req.headers))if(!HOP.includes(k))headers[k]=Array.isArray(v)?v.join(', '):v;
  if(headers['content-encoding']!==undefined)return forwardRefuse(res,415,'encoded request body');
  slot.used=true;
  const abort=new AbortController();
  res.on('close',()=>{if(!res.writableFinished)abort.abort();});
  const headTimer=setTimeout(()=>abort.abort(),Math.max(1,Math.min(slot.headTimeoutMs,slot.until-Date.now())));
  hostedFrames.open=true;
  emit({type:'provider_fetch',id:slot.id,binding:slot.sid.binding,sequence:slot.sid.sequence,stream:slot.sid.stream,url:ANTHROPIC_BASE+req.url,method:'POST',headers,body:Buffer.concat(chunks).toString('base64')});
  const handle=openForwarded({id:slot.sid,sendRaw:async v=>emit(v),read:()=>hostedFrames.read(),signal:abort.signal,onSettled:()=>{hostedFrames.open=false;hostedFrames.queue=[];slot.settle();}});
  try{
   const response=await handle.response;clearTimeout(headTimer);
   res.writeHead(response.status,{'content-type':response.status===200?'text/event-stream':'application/json'});
   for await(const chunk of response.body)res.write(chunk);
   res.end();
  }catch{clearTimeout(headTimer);if(!res.headersSent)forwardRefuse(res,502,'host relay failed');else res.destroy();}
 });
}):null;
const input=createInterface({input:process.stdin,crlfDelay:Infinity});
input.on('line',line=>{
 try {
  if(Buffer.byteLength(line)>131072)throw Error('input frame bound');
  const value=JSON.parse(line);
  if(signed&&value.type==='initialize'&&initializeResolve&&value.sessionId===manager?.getSessionId()&&(!fixtureSigned||value.fixtureAccess==='FAKE-NEVER-A-CREDENTIAL')){initializeResolve(value);initializeResolve=null;}
  else if(value.type==='pressure_context'&&!live&&!signed&&!pressureContextRequested&&startResolve&&value.sessionId===session?.sessionId&&Object.keys(value).sort().join(',')==='sessionId,type'){
   pressureContextRequested=true;
   const usage=session.getContextUsage();
   emit({type:'pressure_context',snapshot:{sessionId:session.sessionId,idle:!session.isStreaming&&!session.isCompacting&&session.pendingMessageCount===0,observedMs:Date.now(),model:{id:model.id,provider:model.provider,contextWindow:model.contextWindow},usage:usage===undefined?{availability:'undefined'}:{availability:'estimate',value:usage}}});
  }
  else if(value.type===(live?'task_start':'fixture_start')&&startResolve&&value.sessionId===session?.sessionId){startResolve(value);startResolve=null;}
  else if(bridge&&['bridge_prompt','bridge_replace','bridge_close'].includes(value.type)&&value.sessionId===session?.sessionId&&!bridgeActive&&bridgeQueue.length===0){if(bridgeWake){const wake=bridgeWake;bridgeWake=null;wake(value);}else bridgeQueue.push(value);}
  else if(hosted&&RESPONSE_FRAMES.includes(value.type))hostedFrames.push(value);
  else if(bridge&&value.type==='bridge_provider'&&grants.has(value.id)&&value.sessionId===session?.sessionId){grants.get(value.id)(value);grants.delete(value.id);}
  else if(value.type==='claims_result'&&claimsTools&&pending.has(value.id)&&value.sessionId===undefined){pending.get(value.id)(value);pending.delete(value.id);}
  else if(value.type==='snapshot_result'&&analysis&&pending.has(value.id)&&value.sessionId===undefined){pending.get(value.id)(value);pending.delete(value.id);}
  else if(value.type==='tool_result'&&pending.has(value.id)&&(!signed||value.sessionId===manager?.getSessionId())){pending.get(value.id)(value);pending.delete(value.id);}
  else if(value.type==='provider_grant'&&grants.has(value.id)&&(!signed||(value.sessionId===manager?.getSessionId()&&(!fixtureSigned||value.classification==='fake_fixture')))){grants.get(value.id)(value);grants.delete(value.id);}
  else throw Error('unexpected host frame');
 }catch{emit({type:'error',message:'host protocol rejected'});process.exitCode=2;session?.abort();input.close();}
});
input.on('close',()=>{if(bridge&&bridgeWake){const wake=bridgeWake;bridgeWake=null;wake({type:'bridge_abort'});}if(startResolve||pending.size||grants.size||bridgeActive){process.exitCode=2;session?.abort();}});
let deadline;
try {
 if(signed){
  manager=SessionManager.create('/workspace','/export/sessions');
  emit({type:'allocated',classification:'actual SDK allocation; no ModelRuntime/access yet',accessPresent:existsSync('/auth/access.json')});
  const initialized=await initialization;
  signedDeadline=consumptionDeadline(initialized,initialized.deadline*1000);
  checkConsumption(signedDeadline);
  if(fixtureSigned){
  const access=JSON.parse(readFileSync('/auth/access.json','utf8'));
  if(access.classification!=='fake_fixture'||access.access!=='FAKE-NEVER-A-CREDENTIAL')throw Error('fixture access rejected');
  writeFileSync('/tmp/fixture-auth.json','{}',{flag:'wx',mode:0o600});
  }
 }
 const publicSettings=live&&existsSync('/config/settings.json')?JSON.parse(readFileSync('/config/settings.json','utf8')):{};
 const settingsManager=SettingsManager.inMemory({...publicSettings,compaction:{enabled:false},retry:{enabled:false},cacheWarming:'off'});
 const agentDir=live?'/config':'/tmp/pi-empty';
 const resourceLoader=new DefaultResourceLoader({cwd:'/workspace',agentDir,settingsManager,noExtensions:true,disabledBuiltinExtensions:['mcp'],noSkills:true,noPromptTemplates:true,noThemes:true,noContextFiles:true});
 await resourceLoader.reload();
 let modelRuntime,providerOptions={thinkingLevel:'off'};
 if(live){
  if(claudeSdk){
   if(config.schema!==schema||!analysis||!CLAUDE_MODELS.includes(config.model))throw Error('provider configuration rejected');
   const reject=async()=>{throw Error('credential_mutation');};
   modelRuntime=await ModelRuntime.create({credentials:{read:async()=>undefined,list:async()=>[],modify:reject,delete:reject},modelsPath:null,allowModelNetwork:false,refreshOnCreate:false});
   ({openResponse:openForwarded}=await import('/receiver/pi-native-tools.mjs'));
   await new Promise((resolve,reject)=>{forwarder.once('error',reject);forwarder.listen(0,'127.0.0.1',resolve);});
   mkdirSync(`${process.env.HOME}/.claude`,{recursive:true,mode:0o700});
   writeFileSync(`${process.env.HOME}/.claude/.credentials.json`,JSON.stringify(DUMMY_CREDENTIALS),{mode:0o600,flag:'wx'});
   Object.assign(process.env,CLAUDE_CODE_ENV,{ANTHROPIC_BASE_URL:`http://127.0.0.1:${forwarder.address().port}`});
   // The extension registers its provider while the session services load.
  }else{
  if(config.schema!==schema||config.provider!=='openai-codex'||config.model!=='gpt-6-luna')throw Error('provider configuration rejected');
  // Nonauthenticating marker only; the Host holds and applies real access.
  const marker='syntactic.'+Buffer.from(JSON.stringify({'https://api.openai.com/auth':{chatgpt_account_id:'nonsecret-marker'}})).toString('base64')+'.nonsecret';
  const reject=async()=>{throw Error('credential_mutation');};
  const credentials={read:async p=>p==='openai-codex'?{type:'oauth',access:marker,refresh:'',expires:4102444800000,accountId:'nonsecret-marker'}:undefined,list:async()=>[{providerId:'openai-codex',type:'oauth'}],modify:reject,delete:reject};
  modelRuntime=await ModelRuntime.create({credentials,modelsPath:null,allowModelNetwork:false,refreshOnCreate:false});
  if(signed)checkConsumption(signedDeadline);
  model=modelRuntime.getModel('openai-codex',config.model);
  if(!model||model.api!=='openai-codex-responses'||model.baseUrl!=='https://chatgpt.com/backend-api')throw Error('exact subscription model absent');
  }
 }else{
  if(signed)checkConsumption(signedDeadline);
  modelRuntime=await ModelRuntime.create({authPath:'/tmp/fixture-auth.json',modelsPath:null,allowModelNetwork:false,refreshOnCreate:false});
  model=modelRuntime.getModels().find(m=>m.provider==='openai');
  if(!model)throw Error('fixture catalog model absent');
  await modelRuntime.setRuntimeApiKey(model.provider,'FAKE-FIXTURE-NOT-A-CREDENTIAL');
 }
 if(signed)checkConsumption(signedDeadline);
 const tool={name:'workspace_exec',label:'Workspace command',description:'Execute a shell command only in the separate disposable network-disabled workspace container. Workdir /workspace. rustc and cc are available. Immutable tests are /acceptance.rs; compile with rustc --edition=2024 --test /acceptance.rs -o /tmp/check and run /tmp/check. Only edit src/lib.rs.',
  promptSnippet:'Execute commands in disposable workspace',parameters:Type.Object({command:Type.String()}),executionMode:'sequential',
  execute:async(id,args,signal)=>{
   const result=await new Promise((resolve,reject)=>{
    pending.set(id,resolve);
    signal?.addEventListener('abort',()=>{pending.delete(id);reject(Error('tool cancelled'));},{once:true});
    emit({type:'tool_request',id,command:args.command});
   });
   return {content:[{type:'text',text:result.output}],details:{hostReportedSuccess:result.ok}};
  }};
 // Read-only claims tools: the schemas below carry no server, profile, scope or
 // producer parameter; Rust validates every request and holds the trusted pins.
 const claimsDescriptions={claims_discover:'Read-only discovery of repository fact candidates in the Host-selected claims scope. Pages of native owner JSON; repeat with offset and blob_sha256 to page one result.',claims_resolve:'Read-only exact resolution of one selected claim revision (claim_id, revision_id, revision_sha256; never latest). Pages of the complete native projection.',claims_evidence:'Read-only paging of one evidence artifact listed by the exact selected revision.',claims_currency:'Read-only source-currency comparison for a listed evidence artifact; reports equality, change or unavailability.'};
 const idText=Type.String({minLength:1,maxLength:200}),shaText=Type.String({pattern:'^[0-9a-f]{64}$'}),offsetInt=Type.Optional(Type.Integer({minimum:0})),pageInt=Type.Optional(Type.Integer({minimum:4,maximum:8192}));
 const exactProps={claim_id:idText,revision_id:idText,revision_sha256:shaText};
 const claimsParameters={claims_discover:Type.Object({query:Type.String({minLength:1,maxLength:512}),limit:Type.Optional(Type.Integer({minimum:1,maximum:20})),cursor:Type.Optional(Type.Object({})),offset:offsetInt,blob_sha256:Type.Optional(shaText),max_bytes:pageInt}),claims_resolve:Type.Object({...exactProps,offset:offsetInt,max_bytes:pageInt}),claims_evidence:Type.Object({...exactProps,sha256:shaText,offset:offsetInt,max_bytes:pageInt}),claims_currency:Type.Object({...exactProps,sha256:shaText})};
 const claimsCustom=claimsTools?CLAIMS_NAMES.map(name=>({name,label:name,description:claimsDescriptions[name],promptSnippet:claimsDescriptions[name],parameters:claimsParameters[name],executionMode:'sequential',
  execute:async(id,args,signal)=>{
   const result=await new Promise((resolve,reject)=>{
    pending.set(id,resolve);
    signal?.addEventListener('abort',()=>{pending.delete(id);reject(Error('claims cancelled'));},{once:true});
    emit({type:'claims_request',id,tool:name,arguments:args});
   });
   return {content:[{type:'text',text:JSON.stringify(result.ok?result.result:{error:result.error})}],details:{hostReportedSuccess:result.ok}};
  }})):[];
 // Analysis tools: the Host reads the manifest-pinned snapshot and validates
 // every request; nothing here can name another path, server or scope.
 const snapshotExecute=name=>async(id,args,signal)=>{
  const result=await new Promise((resolve,reject)=>{
   pending.set(id,resolve);
   signal?.addEventListener('abort',()=>{pending.delete(id);reject(Error('snapshot read cancelled'));},{once:true});
   emit({type:'snapshot_request',id,tool:name,arguments:args});
  });
  return {content:[{type:'text',text:JSON.stringify(result.ok?result.result:{error:result.error})}],details:{hostReportedSuccess:result.ok}};
 };
 const snapshotCustom=analysis?[
  {name:'snapshot_list',label:'snapshot_list',description:'List the members of the read-only analysis snapshot (path, size in bytes, SHA-256). The snapshot is the only material available.',promptSnippet:'List the snapshot members',parameters:Type.Object({}),executionMode:'sequential',execute:snapshotExecute('snapshot_list')},
  {name:'snapshot_read',label:'snapshot_read',description:`Read one snapshot member as UTF-8 text, one page at a time: at most ${config.pageBytes} bytes from offset (default 0); next_offset is null at the end of the member.`,promptSnippet:'Read a page of a snapshot member',parameters:Type.Object({path:Type.String({minLength:1,maxLength:256}),offset:Type.Optional(Type.Integer({minimum:0})),max_bytes:Type.Optional(Type.Integer({minimum:1,maximum:config.pageBytes}))}),executionMode:'sequential',execute:snapshotExecute('snapshot_read')},
 ]:[];
 const activeNames=analysis?SNAPSHOT_NAMES:['workspace_exec',...(claimsTools?CLAIMS_NAMES:[])];
 const baseOptions={cwd:'/workspace',agentDir,settingsManager,resourceLoader,modelRuntime,model,...providerOptions,noTools:'builtin',tools:activeNames,customTools:analysis?snapshotCustom:[tool,...claimsCustom]};
 function bindBridge(selected,services){
  session=selected;
  session.setAutoCompactionEnabled(false);session.setAutoRetryEnabled(false);
  if(bridgeResets)session.setActiveToolsByName(claimsTools?activeNames:[]);
  // Live bridge: message_update events are not forwarded. Each carries the
  // whole message so far (twice), so forwarding them grows the output
  // quadratically with answer length (open item D4). message_end carries the
  // final message, and the Host retains the provider's exact stream bytes.
  session.subscribe(event=>{if(bridgeActive&&!(liveBridge&&event.type==='message_update'))emit({type:'native_event',event});});
  if(liveBridge){
   // Real inference through the same SDK stream the direct live profile wraps.
   // Re-bound for every SDK session (the factory runs again on newSession()); one
   // counter spans both sessions. Nothing is manufactured: tool ids, assistant
   // text and usage come from the provider. Any deviation fails closed.
   const original=session.agent.streamFunction;
   const failure=message=>{
    liveBridgeFailed=true;
    const stream=createAssistantMessageEventStream();
    const error={role:'assistant',content:[],api:model.api,provider:model.provider,model:model.id,usage:{input:0,output:0,cacheRead:0,cacheWrite:0,totalTokens:0,cost:{input:0,output:0,cacheRead:0,cacheWrite:0,total:0}},stopReason:'aborted',errorMessage:message,timestamp:Date.now()};
    stream.push({type:'error',reason:'aborted',error});stream.end();return stream;
   };
   session.agent.streamFunction=async(requestModel,context,options)=>{
    try{
     const id=++bridgeCalls;
     const contextBytes=Buffer.byteLength(JSON.stringify(context));
     if(!bridgeActive||id>config.maxRequests||id>MAX_REQUESTS||contextBytes>config.maxContextBytes||Date.now()>=config.deadline*1000||requestModel.id!==model.id||requestModel.provider!==model.provider)return failure('outside host admission unavailable');
     const grant=await new Promise((resolve,reject)=>{
      grants.set(id,resolve);
      options?.signal?.addEventListener('abort',()=>{grants.delete(id);reject(Error('provider grant cancelled'));},{once:true});
      emit({type:'coding_provider_request',id,producer:'pi-1.5.0',invocationId:operation.invocationId,turnCorrelation:operation.turnCorrelation,prompt_sha256:createHash('sha256').update(operation.prompt).digest('hex'),messages:context.messages,agentMessages:session.messages,requestModel:{id:requestModel.id,provider:requestModel.provider,contextWindow:requestModel.contextWindow},usage:session.getContextUsage()??null,contextBytes});
     });
     if(grant.mode!=='live')throw Error('bridge grant mode');
     const until=consumptionDeadline(grant,config.deadline*1000);
     checkConsumption(until);
     if(hosted){
      // One response stream identity per logical request; Host failover
      // attempts stay behind it. Container observations are not evidence here.
      const sid=grant.stream;
      if(!sid||typeof sid.binding!=='object'||!Number.isSafeInteger(sid.sequence)||!Number.isSafeInteger(sid.stream)||!Number.isSafeInteger(grant.headTimeoutMs)||grant.headTimeoutMs<=0)throw Error('hosted stream identity');
      const {openResponse}=await import('/receiver/pi-native-tools.mjs');
      let fetched=false;
      if(claudeSdk){forwardSlot={id,sid,until,headTimeoutMs:grant.headTimeoutMs,used:false};forwardSlot.settled=new Promise(r=>{forwardSlot.settle=r;});}
      const slot=forwardSlot;
      const inner=claudeSdk?await original(requestModel,context,options):await original(requestModel,context,{...options,transport:'sse',maxRetries:0,cacheRetention:'none',timeoutMs:Math.max(1,Math.min(grant.headTimeoutMs,until-Date.now())),fetch:async(url,init)=>{
       checkConsumption(until);
       if(fetched||hostedFrames.open)throw Error('hosted fetch already used');
       fetched=true;
       let body=typeof init.body==='string'?Buffer.from(init.body):Buffer.from(init.body??[]);
       const headers=Object.fromEntries(new Headers(init.headers));
       // The SDK zstd-compresses Codex SSE bodies. The Host validates and
       // forwards plain JSON (the SDK's own uncompressed fallback), so the
       // bounded decompression happens here, never on the Host.
       if(headers['content-encoding']==='zstd'){body=zstdDecompressSync(body,{maxOutputLength:MAX_BODY+1});delete headers['content-encoding'];}
       else if(headers['content-encoding']!==undefined)throw Error('hosted request encoding');
       if(body.length>MAX_BODY)throw Error('hosted request bound');
       hostedFrames.open=true;
       emit({type:'provider_fetch',id,binding:sid.binding,sequence:sid.sequence,stream:sid.stream,url:String(url),method:init.method,headers,body:body.toString('base64')});
       const handle=openResponse({id:sid,sendRaw:async v=>emit(v),read:()=>hostedFrames.read(),signal:init.signal,onSettled:()=>{hostedFrames.open=false;hostedFrames.queue=[];}});
       return handle.response;
      }});
      const relay=createAssistantMessageEventStream();
      (async()=>{
       try{
        for await(const event of inner){
         if(event.type==='done'||event.type==='error'){
          const message=event.type==='done'?event.message:event.error;
          // Claude Code ends its turn at message_stop, before the forwarder
          // has relayed the stream's end; the Host settles only a closed relay.
          if(claudeSdk&&slot.used)await slot.settled;
          emit({type:'provider_complete',id,stopReason:message.stopReason,usage:message.usage});
         }
         relay.push(event);
        }
        relay.end();
       }catch{const aborted=failure('provider stream failed');for await(const event of aborted)relay.push(event);relay.end();}
       finally{forwardSlot=null;}
      })();
      return relay;
     }
     throw Error('only the hosted transport is supported');
    }catch{return failure('provider entry failed');}
   };
   return;
  }
  // Derive an exact-reference call from the owner checkpoint text in the
  // successor's prompt; refuses rather than guessing.
  const checkpointCall=c=>{
   const marker='CLAIMS_HANDOFF_V1 ';const at=operation.prompt.indexOf(marker);
   if(at<0)throw Error('checkpoint handoff absent');
   const rest=operation.prompt.slice(at+marker.length);
   const ends=[rest.indexOf('\n'),rest.indexOf('\\n')].filter(n=>n>=0);
   const body=rest.slice(0,ends.length?Math.min(...ends):rest.length);
   let handoff;try{handoff=JSON.parse(body);}catch{handoff=JSON.parse(body.replaceAll('\\"','"').replaceAll('\\\\','\\'));}
   const ref=handoff.refs?.[c.ref??0];if(!ref)throw Error('checkpoint reference absent');
   const exact={claim_id:ref.claim_id,revision_id:ref.revision_id,revision_sha256:ref.revision_sha256};
   if(c.name==='claims_resolve')return exact;
   const sha=ref.evidence_sha256?.[c.evidence??0];if(!sha)throw Error('checkpoint evidence absent');
   return {...exact,sha256:sha};
  };
  const stream=async(requestModel,context)=>{
   const id=++bridgeCalls;if(id>4)throw Error('bridge request budget');
   const grant=await new Promise(resolve=>{grants.set(id,resolve);emit({type:'coding_provider_request',id,producer:'pi-1.5.0',invocationId:operation.invocationId,turnCorrelation:operation.turnCorrelation,prompt_sha256:createHash('sha256').update(operation.prompt).digest('hex'),messages:context.messages,agentMessages:session.messages,requestModel:{id:requestModel.id,provider:requestModel.provider,contextWindow:requestModel.contextWindow},usage:session.getContextUsage()??null});});
   checkConsumption(consumptionDeadline(grant,Number.MAX_SAFE_INTEGER));
   let scripted=null;
   if(claimsTools){
    // Fake-provider script (routing and exact-byte recovery only; no model
    // comprehension). Calls are limited to the five admitted tool names.
    if(grant.mode!=='script'||!Array.isArray(grant.calls)||grant.calls.length>4||(grant.calls.length===0&&typeof grant.reply!=='string'))throw Error('claims script conflict');
    scripted=grant.calls.map((c,i)=>{
     if(c.name==='workspace_exec')return {type:'toolCall',id:`fixture-${id}-${i}`,name:'workspace_exec',arguments:{command:c.command??operation.command}};
     if(!CLAIMS_NAMES.includes(c.name))throw Error('claims script tool refused');
     return {type:'toolCall',id:`fixture-${id}-${i}`,name:c.name,arguments:c.fromCheckpoint?checkpointCall(c):c.arguments};
    });
   }else if((id===1)!==(grant.mode==='tool'))throw Error('bridge tool profile conflict');
   const calls=claimsTools?scripted:(id===1?[{type:'toolCall',id:'fixture-tool-1',name:'workspace_exec',arguments:{command:operation.command}}]:[]);
   const message={role:'assistant',content:calls.length?calls:[{type:'text',text:grant.reply}],api:model.api,provider:model.provider,model:model.id,usage:{input:1,output:1,cacheRead:0,cacheWrite:0,totalTokens:2,cost:{input:0,output:0,cacheRead:0,cacheWrite:0,total:0}},stopReason:calls.length?'toolUse':'stop',timestamp:Date.now()};
   const out=createAssistantMessageEventStream();out.push({type:'done',reason:message.stopReason,message});out.end();return out;
  };
  markRuntimeStreamFunction(stream,services.modelRuntime);session.agent.streamFunction=stream;
 }
 if(bridge){
  const factory=async options=>{const services=await createAgentSessionServices({...baseOptions,...options,resourceLoaderOptions:{noExtensions:true,disabledBuiltinExtensions:['mcp'],noSkills:true,noPromptTemplates:true,noThemes:true,noContextFiles:true,...(claudeSdk?{additionalExtensionPaths:[EXTENSION]}:{})}});
   if(claudeSdk){
    // The pinned extension's provider and model; the Host checks the ready frame's model against the catalog.
    model=services.modelRuntime.getModel('claude-sdk',config.model);
    if(!model||model.api!=='claude-agent-sdk'||model.provider!=='claude-sdk')throw Error('exact claude-sdk model absent');
   }
   const result=await createAgentSessionFromServices({...baseOptions,...options,model,services});bindBridge(result.session,services);return {...result,services,diagnostics:services.diagnostics};};
  runtime=await createAgentSessionRuntime(factory,{cwd:'/workspace',agentDir,sessionManager:SessionManager.create('/workspace','/export/sessions')});
 }else{({session}=await createAgentSession({...baseOptions,sessionManager:manager??SessionManager.create('/workspace','/export/sessions')}));}
 if(signed)checkConsumption(signedDeadline);
 session.setAutoCompactionEnabled(false);session.setAutoRetryEnabled(false);
 if(!bridge)session.subscribe(event=>emit({type:'native_event',event}));
 let command,turn=0;
 if(!bridge){
  session.agent.streamFunction=async(requestModel,context)=>{
   if(pressureCheckpoint){
    emit({type:'pressure_request',classification:'actual SDK request; fake provider stream',generation:pressureCheckpoint.generation,input_digest:pressureCheckpoint.input_digest,requestModel:{id:requestModel.id,provider:requestModel.provider,contextWindow:requestModel.contextWindow},context});
    pressureCheckpoint=null;
   }
   const stream=createAssistantMessageEventStream();const toolTurn=turn++===0;
   const id=turn;
   if(signed){
    const grant=await new Promise(resolve=>{grants.set(id,resolve);emit({type:'provider_request',id,provider:'fake_fixture',model:'fake-pi-model',contextBytes:1});});
    checkConsumption(consumptionDeadline(grant,Number.MAX_SAFE_INTEGER));
    emit({type:'provider_observation',id,httpStatus:200,backendModel:'fake-pi-model',rawTerminal:{classification:'fake_fixture',eventType:'response.completed',status:'completed',usage:{input_tokens:1,output_tokens:1,total_tokens:2}}});
   }
   const message={role:'assistant',content:toolTurn?[{type:'toolCall',id:'fixture-tool-1',name:'workspace_exec',arguments:{command}}]:[{type:'text',text:'Provider-disabled tool-path fixture completed.'}],api:model.api,provider:model.provider,model:model.id,
    usage:{input:0,output:0,cacheRead:0,cacheWrite:0,totalTokens:0,cost:{input:0,output:0,cacheRead:0,cacheWrite:0,total:0}},stopReason:toolTurn?'toolUse':'stop',timestamp:Date.now()};
   if(signed){
    message.provider='fake_fixture';message.model='fake-pi-model';
    message.usage={input:1,output:1,cacheRead:0,cacheWrite:0,totalTokens:2,cost:{input:0,output:0,cacheRead:0,cacheWrite:0,total:0}};
    emit({type:'provider_complete',id,stopReason:message.stopReason,usage:message.usage});
   }
   stream.push({type:'done',reason:message.stopReason,message});stream.end();return stream;
  };
 }
 if(bridge){
  const snapshot=()=>({sessionId:session.sessionId,idle:!session.isStreaming&&!session.isCompacting&&session.pendingMessageCount===0,observedMs:Date.now(),observedAt:new Date().toISOString(),model:{id:model.id,provider:model.provider,contextWindow:model.contextWindow},usage:session.getContextUsage()??null});
  if(liveBridge)deadline=setTimeout(()=>{process.exitCode=2;session.abort();input.close();process.stdin.destroy();},Math.max(1,config.deadline*1000-Date.now()));
  emit({type:'ready',tools:session.getActiveToolNames(),snapshot:snapshot(),...(liveBridge?{model:{id:model.id,provider:model.provider,api:model.api,contextWindow:model.contextWindow,maxTokens:model.maxTokens,cost:model.cost}}:{})});
  for(;;){
   const op=await bridgeNext();checkConsumption(consumptionDeadline(op,Number.MAX_SAFE_INTEGER));
   if(op.type==='bridge_close')break;
   if(op.type==='bridge_replace'){
    if(bridgeResets++!==0||pending.size||grants.size||session.isStreaming)throw Error('bridge reset budget/uncertainty');
    const old=session.sessionId;const result=await runtime.newSession();
    emit({type:'coding_replaced',oldSessionId:old,result,messages:session.messages,branch:session.sessionManager.getBranch(),header:session.sessionManager.getHeader(),leafId:session.sessionManager.getLeafId(),autoCompactionEnabled:session.autoCompactionEnabled,tools:session.getActiveToolNames(),snapshot:snapshot()});continue;
   }
   operation=op;bridgeActive=true;
   await session.prompt(op.prompt,{expandPromptTemplates:false});bridgeActive=false;
   if(liveBridgeFailed)throw Error('live bridge provider failure');
   if(session.isStreaming||session.isCompacting||session.pendingMessageCount!==0||pending.size||grants.size)throw Error('bridge turn unsettled');
   const history={type:'coding_history',producer:'pi-1.5.0',sessionId:session.sessionId,invocationId:op.invocationId,messages:session.messages,branch:session.sessionManager.getBranch(),header:session.sessionManager.getHeader(),leafId:session.sessionManager.getLeafId(),snapshot:snapshot()};
   writeFileSync(`/export/coding-history-${op.invocationId}.json`,JSON.stringify(history)+'\n',{flag:'wx',mode:0o600});emit(history);
  }
  const history={schema,provider:liveBridge?config.provider:'disabled_fixture',model:liveBridge?model.id:null,sessionId:session.sessionId,messages:session.messages,entries:session.sessionManager.getEntries(),header:session.sessionManager.getHeader(),counts:{providerRequests:bridgeCalls,resets:bridgeResets,...(claudeSdk?{forwarderRefused}:{})}};
  writeFileSync('/export/coding-native-history.json',JSON.stringify(history)+'\n',{flag:'wx',mode:0o600});emit({type:'complete',provider:history.provider,counts:history.counts});
 }else{
 emit({type:'ready',tools:session.getActiveToolNames(),...(live?{model:{id:model.id,provider:model.provider,api:model.api,contextWindow:model.contextWindow,maxTokens:model.maxTokens,cost:model.cost}}:{})});
 const started=await start;command=started.command;
 if(started.pressureCheckpoint){
  const p=started.pressureCheckpoint;
  if(live||signed||!pressureContextRequested||p.schema!=='work-engine.pi-pressure-fixture.v2'||p.session_id!==session.sessionId||typeof p.generation!=='string'||typeof p.pressure!=='string'||Buffer.byteLength(p.pressure)>24000)throw Error('pressure checkpoint rejected');
  pressureCheckpoint=p;
  await session.sendCustomMessage({customType:'work-engine.pressure-fixture.v2',content:[{type:'text',text:p.pressure}],display:false,details:{input_digest:p.input_digest,generation:p.generation,authority:'observation_only'}},{triggerTurn:false});
  emit({type:'pressure_inserted',input_digest:p.input_digest,generation:p.generation,messages:session.messages});
 }
 await session.prompt(live?config.task:'Exercise the fixed provider-disabled workspace tool fixture.',{expandPromptTemplates:false});
 if(session.isStreaming||session.isCompacting||session.pendingMessageCount!==0)throw Error('session not idle');
 const history={schema,...(fixtureSigned?{classification:'fake provider messages/usage; actual SDK manager and catalog configuration',nativeCatalog:{provider:model.provider,model:model.id}}:{}),provider:fixtureSigned?'fake_fixture':live?'openai-codex':'disabled_fixture',model:fixtureSigned?'fake-pi-model':live?model.id:null,sessionId:session.sessionId,messages:session.messages,entries:session.sessionManager.getEntries(),header:session.sessionManager.getHeader()};
 const fd=openSync('/export/coding-native-history.json','wx',0o600);
 try{writeFileSync(fd,JSON.stringify(history)+'\n');fsyncSync(fd);}finally{closeSync(fd);}
 if(fixtureSigned)exportBundle();
 emit({type:'complete',provider:history.provider});
 }
}catch{emit({type:'error',message:'Pi coding profile failed'});process.exitCode=2;}
finally{clearTimeout(deadline);if(runtime)await runtime.dispose();else session?.dispose();input.close();process.stdin.destroy();
 // claude-sdk: the forwarder and Claude Code's child process would keep the
 // process alive; exit runs the extension's shutdown of Claude Code.
 if(forwarder){forwarder.close();forwarder.closeAllConnections();setTimeout(()=>process.exit(),2000).unref();}}
