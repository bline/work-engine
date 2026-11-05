// Exact Pi1.5 interoperability. Rust owns admission, budgets, execution and custody.
import {createAgentSession,createAgentSessionRuntime,createAgentSessionServices,createAgentSessionFromServices,DefaultResourceLoader,ModelRuntime,SessionManager,SettingsManager} from '/pi/node_modules/@earendil-works/pi-coding-agent/dist/index.js';
import {createAssistantMessageEventStream} from '/pi/node_modules/@earendil-works/pi-ai/dist/utils/event-stream.js';
import {markRuntimeStreamFunction} from '/pi/node_modules/@earendil-works/pi-coding-agent/dist/core/agent-session.js';
import {createHash} from 'node:crypto';
import {Type} from '/pi/node_modules/typebox/build/index.mjs';
import {createInterface} from 'node:readline';
import {openSync,writeFileSync,fsyncSync,closeSync,existsSync,readFileSync} from 'node:fs';
const bridge=process.argv.includes('--lifecycle-bridge');
const fixtureSigned=existsSync('/auth/signed-fixture.json');
const live=existsSync('/profile/job.json');
if(bridge&&(fixtureSigned||live))throw Error('bridge is provider-disabled only');
if(bridge)globalThis.fetch=async()=>{throw Error('network forbidden in coding bridge');};
if(fixtureSigned&&live)throw Error('fixture/live conflict');
const config=live?JSON.parse(readFileSync('/profile/job.json','utf8')):null;
const signedSubscription=live&&config.schema==='work-engine.pi-coding-signed-subscription.v1';
const signed=fixtureSigned||signedSubscription;
if(fixtureSigned)globalThis.fetch=async()=>{throw Error('network forbidden in signed fixture');};
const schema=bridge?'work-engine.pi-coding-lifecycle-fixture.v1':fixtureSigned?'work-engine.pi-coding-signed-fixture.v1':signedSubscription?config.schema:live?'work-engine.pi-coding-sdk.v1':'work-engine.pi-coding-fixture.v1';
let runtime,operation,bridgeCalls=0,bridgeResets=0,bridgeActive=false,bridgeWake;
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
 if(Buffer.byteLength(bytes)>65536)throw Error('output frame bound');
 process.stdout.write(bytes);
};
let startResolve;
const start=new Promise(resolve=>{startResolve=resolve;});
if(bridge)startResolve=null;
const pending=new Map();
const grants=new Map();
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
  else if(bridge&&value.type==='bridge_provider'&&grants.has(value.id)&&value.sessionId===session?.sessionId){grants.get(value.id)(value);grants.delete(value.id);}
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
 let modelRuntime,rawTerminalEvidence;
 if(live){
  if(config.schema!==schema||config.provider!=='openai-codex'||config.model!=='gpt-6-luna')throw Error('provider configuration rejected');
  const nativeAccess=await import('/receiver/pi-native-access.mjs');
  if(signed)checkConsumption(signedDeadline);
  const {createNativeAccessStore}=nativeAccess;rawTerminalEvidence=nativeAccess.rawTerminalEvidence;
  const grant=JSON.parse(readFileSync('/auth/access.json','utf8'));
  const credentials=createNativeAccessStore(grant);
  modelRuntime=await ModelRuntime.create({credentials,modelsPath:null,allowModelNetwork:false,refreshOnCreate:false});
  if(signed)checkConsumption(signedDeadline);
  model=modelRuntime.getModel('openai-codex',config.model);
  if(!model||model.api!=='openai-codex-responses'||model.baseUrl!=='https://chatgpt.com/backend-api')throw Error('exact subscription model absent');
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
 const baseOptions={cwd:'/workspace',agentDir,settingsManager,resourceLoader,modelRuntime,model,thinkingLevel:'off',noTools:'builtin',tools:['workspace_exec'],customTools:[tool]};
 function bindBridge(selected,services){
  session=selected;
  session.setAutoCompactionEnabled(false);session.setAutoRetryEnabled(false);
  if(bridgeResets)session.setActiveToolsByName([]);
  session.subscribe(event=>{if(bridgeActive)emit({type:'native_event',event});});
  const stream=async(requestModel,context)=>{
   const id=++bridgeCalls;if(id>4)throw Error('bridge request budget');
   const grant=await new Promise(resolve=>{grants.set(id,resolve);emit({type:'coding_provider_request',id,producer:'pi-1.5.0',invocationId:operation.invocationId,turnCorrelation:operation.turnCorrelation,prompt_sha256:createHash('sha256').update(operation.prompt).digest('hex'),messages:context.messages,agentMessages:session.messages,requestModel:{id:requestModel.id,provider:requestModel.provider,contextWindow:requestModel.contextWindow},usage:session.getContextUsage()??null});});
   checkConsumption(consumptionDeadline(grant,Number.MAX_SAFE_INTEGER));
   if((id===1)!==(grant.mode==='tool'))throw Error('bridge tool profile conflict');
   const message={role:'assistant',content:id===1?[{type:'toolCall',id:'fixture-tool-1',name:'workspace_exec',arguments:{command:operation.command}}]:[{type:'text',text:grant.reply}],api:model.api,provider:model.provider,model:model.id,usage:{input:1,output:1,cacheRead:0,cacheWrite:0,totalTokens:2,cost:{input:0,output:0,cacheRead:0,cacheWrite:0,total:0}},stopReason:id===1?'toolUse':'stop',timestamp:Date.now()};
   const out=createAssistantMessageEventStream();out.push({type:'done',reason:message.stopReason,message});out.end();return out;
  };
  markRuntimeStreamFunction(stream,services.modelRuntime);session.agent.streamFunction=stream;
 }
 if(bridge){
  const factory=async options=>{const services=await createAgentSessionServices({...baseOptions,...options,resourceLoaderOptions:{noExtensions:true,disabledBuiltinExtensions:['mcp'],noSkills:true,noPromptTemplates:true,noThemes:true,noContextFiles:true}});const result=await createAgentSessionFromServices({...baseOptions,...options,services});bindBridge(result.session,services);return {...result,services,diagnostics:services.diagnostics};};
  runtime=await createAgentSessionRuntime(factory,{cwd:'/workspace',agentDir,sessionManager:SessionManager.create('/workspace','/export/sessions')});
 }else{({session}=await createAgentSession({...baseOptions,sessionManager:manager??SessionManager.create('/workspace','/export/sessions')}));}
 if(signed)checkConsumption(signedDeadline);
 session.setAutoCompactionEnabled(false);session.setAutoRetryEnabled(false);
 if(!bridge)session.subscribe(event=>emit({type:'native_event',event}));
 let command,turn=0;
 if(live){
  const original=session.agent.streamFunction;
  let request=0;
  session.agent.streamFunction=async(requestModel,context,options)=>{
   const id=++request;
   const failure=()=>{
    const stream=createAssistantMessageEventStream();
    const error={role:'assistant',content:[],api:model.api,provider:model.provider,model:model.id,usage:{input:0,output:0,cacheRead:0,cacheWrite:0,totalTokens:0,cost:{input:0,output:0,cacheRead:0,cacheWrite:0,total:0}},stopReason:'aborted',errorMessage:'outside host admission unavailable',timestamp:Date.now()};
    stream.push({type:'error',reason:'aborted',error});stream.end();return stream;
   };
   try{
    const contextBytes=Buffer.byteLength(JSON.stringify(context));
    if(id>config.maxRequests||contextBytes>config.maxContextBytes||Date.now()>=config.deadline*1000)return failure();
    const grant=await new Promise((resolve,reject)=>{
     grants.set(id,resolve);
     options?.signal?.addEventListener('abort',()=>{grants.delete(id);reject(Error('provider grant cancelled'));},{once:true});
     emit({type:'provider_request',id,provider:requestModel.provider,model:requestModel.id,contextBytes});
    });
    const until=consumptionDeadline(grant,config.deadline*1000);
    checkConsumption(until);
    const inner=await original(requestModel,context,{...options,transport:'sse',maxRetries:0,cacheRetention:'none',fetch:(url,init)=>{checkConsumption(until);return globalThis.fetch(url,{...init,redirect:'error'});},timeoutMs:Math.max(1,Math.min(30000,until-Date.now())),
     onResponse:response=>emit({type:'provider_observation',id,httpStatus:response.status}),
     onProviderStreamEvent:data=>{const backendModel=data?.response?.model;const rawTerminal=rawTerminalEvidence(data);if(typeof backendModel==='string'||rawTerminal)emit({type:'provider_observation',id,...(typeof backendModel==='string'?{backendModel}:{}),...(rawTerminal?{rawTerminal}:{})});},
    });
    const relay=createAssistantMessageEventStream();
    (async()=>{
     try{
      for await(const event of inner){
       if(event.type==='done'||event.type==='error'){
        const message=event.type==='done'?event.message:event.error;
        emit({type:'provider_complete',id,stopReason:message.stopReason,usage:message.usage});
       }
       relay.push(event);
      }
      relay.end();
     }catch{const aborted=failure();for await(const event of aborted)relay.push(event);relay.end();}
    })();
    return relay;
   }catch{return failure();}
  };
  deadline=setTimeout(()=>{process.exitCode=2;session.abort();input.close();process.stdin.destroy();},Math.max(1,config.deadline*1000-Date.now()));
 }else if(!bridge){
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
  emit({type:'ready',tools:session.getActiveToolNames(),snapshot:snapshot()});
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
   if(session.isStreaming||session.isCompacting||session.pendingMessageCount!==0||pending.size||grants.size)throw Error('bridge turn unsettled');
   const history={type:'coding_history',producer:'pi-1.5.0',sessionId:session.sessionId,invocationId:op.invocationId,messages:session.messages,branch:session.sessionManager.getBranch(),header:session.sessionManager.getHeader(),leafId:session.sessionManager.getLeafId(),snapshot:snapshot()};
   writeFileSync(`/export/coding-history-${op.invocationId}.json`,JSON.stringify(history)+'\n',{flag:'wx',mode:0o600});emit(history);
  }
  const history={schema,provider:'disabled_fixture',model:null,sessionId:session.sessionId,messages:session.messages,entries:session.sessionManager.getEntries(),header:session.sessionManager.getHeader(),counts:{providerRequests:bridgeCalls,resets:bridgeResets}};
  writeFileSync('/export/coding-native-history.json',JSON.stringify(history)+'\n',{flag:'wx',mode:0o600});emit({type:'complete',provider:'disabled_fixture',counts:history.counts});
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
 emit({type:'complete',provider:history.provider});
 }
}catch{emit({type:'error',message:'Pi coding profile failed'});process.exitCode=2;}
finally{clearTimeout(deadline);if(runtime)await runtime.dispose();else session?.dispose();input.close();process.stdin.destroy();}
