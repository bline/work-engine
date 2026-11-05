// SDK-required glue. All provider responses are labelled fixture data. Rust
// alone owns admission, checkpoint verification and durable settlement.
import {createInterface} from 'node:readline';
import {createHash} from 'node:crypto';
import {mkdirSync,writeFileSync,appendFileSync} from 'node:fs';
import {createAgentSessionRuntime, createAgentSessionServices, createAgentSessionFromServices, SessionManager, SettingsManager} from '/runtime/node_modules/@earendil-works/pi-coding-agent/dist/index.js';
import {markRuntimeStreamFunction} from '/runtime/node_modules/@earendil-works/pi-coding-agent/dist/core/agent-session.js';
import {createAssistantMessageEventStream} from '/runtime/node_modules/@earendil-works/pi-ai/dist/index.js';
const emit=v=>{ const raw=JSON.stringify(v); appendFileSync('/work/actual-sdk.jsonl',raw+'\n'); process.stdout.write(raw+'\n'); };
const model={id:'fixture-disabled',name:'fixture-disabled',api:'openai-completions',provider:'fixture',baseUrl:'http://provider.invalid',reasoning:false,input:['text'],cost:{input:0,output:0,cacheRead:0,cacheWrite:0},contextWindow:32768,maxTokens:1024};
let runtime, reply='',active=false,callCount=0,replacements=0,subscriptions=0;
const agentDir='/work/agent'; mkdirSync(agentDir,{recursive:true});
async function factory(options) {
 const settingsManager=SettingsManager.inMemory({compaction:{enabled:false},retry:{enabled:false}});
 const services=await createAgentSessionServices({...options,settingsManager,resourceLoaderOptions:{noExtensions:true,noSkills:true,noPromptTemplates:true,noThemes:true,noContextFiles:true,systemPrompt:'Provider-disabled context transition fixture.'}});
 services.modelRuntime.registerProvider('fixture',{api:'openai-completions',baseUrl:'http://provider.invalid',apiKey:'fixture-data-not-a-credential',authHeader:false,models:[model]});
 const result=await createAgentSessionFromServices({services,sessionManager:options.sessionManager,sessionStartEvent:options.sessionStartEvent,model,thinkingLevel:'off',tools:[],noTools:'all'});
 const stream=(selected,context)=>{
   callCount++; const s=createAssistantMessageEventStream();
   emit({type:'pi.provider_request',producer:'pi-1.5.0',sessionId:result.session.sessionId,callCount,fixture:true,invocationId,turnCorrelation,model:selected,messages:context.messages,agentMessages:result.session.messages,prompt_sha256:createHash('sha256').update(currentPrompt).digest('hex')});
   const message={role:'assistant',content:[{type:'text',text:reply}],api:model.api,provider:model.provider,model:model.id,usage:{input:1,output:1,cacheRead:0,cacheWrite:0,totalTokens:2,cost:{input:0,output:0,cacheRead:0,cacheWrite:0,total:0}},stopReason:'stop',timestamp:Date.now()};
   queueMicrotask(()=>{s.push({type:'start',partial:{...message,content:[]}});s.push({type:'done',reason:'stop',message});s.end(message);});
   return s;
 };
 markRuntimeStreamFunction(stream,services.modelRuntime); result.session.agent.streamFunction=stream;
 return {...result,services,diagnostics:services.diagnostics};
}
let currentPrompt='',invocationId='',turnCorrelation='';
async function bind(session) {
 subscriptions++; session.subscribe(e=>{if(active)emit(e);});
 session.subscribe(e=>{appendFileSync('/work/pressure-observations.jsonl',JSON.stringify({producer:'pi-1.5.0',sessionId:session.sessionId,eventType:e.type,sourceMessageTimestamp:e.message?.timestamp,observedAt:new Date().toISOString(),contextUsage:session.getContextUsage(),contextWindow:session.model.contextWindow})+'\n');});
 session.observationBinding={sessionId:session.sessionId,contextWindow:session.model.contextWindow,pressureAtBind:session.getContextUsage(),collectionSubscriptions:1,pressureSubscriptions:1};
}
for await(const line of createInterface({input:process.stdin})) {
 try {
  const command=JSON.parse(line);
  if(command.op==='init') {
   runtime=await createAgentSessionRuntime(factory,{cwd:'/work',agentDir,sessionManager:SessionManager.create('/work','/work/sessions')});
   runtime.setRebindSession(bind); await bind(runtime.session);
   emit({type:'pi.ready',sessionId:runtime.session.sessionId,subscriptions,observationBinding:runtime.session.observationBinding});
  } else if(command.op==='prompt') {
   currentPrompt=command.text;reply=command.fixture_reply;invocationId=command.invocationId;turnCorrelation=command.turnCorrelation;active=true;
   await runtime.session.prompt(currentPrompt);active=false;
   emit({type:'pi.session_history',producer:'pi-1.5.0',sessionId:runtime.session.sessionId,messages:runtime.session.messages,branch:runtime.session.sessionManager.getBranch(),header:runtime.session.sessionManager.getHeader(),leafId:runtime.session.sessionManager.getLeafId(),contextTokens:runtime.session.getContextUsage(),subscriptions,fixture:true});
  } else if(command.op==='replace') {
   if(replacements!==0)throw Error('finite actuator budget exhausted'); replacements++;
   const old=runtime.session.sessionId;const result=await runtime.newSession();
   emit({type:'pi.replaced',oldSessionId:old,sessionId:runtime.session.sessionId,result,subscriptions,observationBinding:runtime.session.observationBinding,messages:runtime.session.messages});
  } else if(command.op==='close') {
   await runtime.dispose();writeFileSync('/work/sdk-counts.json',JSON.stringify({callCount,replacements,subscriptions}));emit({type:'pi.closed',callCount,replacements,subscriptions});break;
  } else throw Error('unsupported glue command');
 } catch(error) {active=false;emit({type:'pi.error',message:String(error),stack:error.stack});}
}
