// Minimal native SDK interoperability. Host owns dispatch and checked retention.
// No real access material, discovery or network permission. Native tools write only volatile isolated scratch.
import fs from 'node:fs';
import net from 'node:net';
import {syncBuiltinESMExports} from 'node:module';
const MAX=2*1024*1024;

// Consumer-driven incremental response body: one host chunk per native read
// demand. Every frame uses this response's immutable identity. A cancel may
// overtake an outstanding pull; it then owns the frame reader until the host
// confirms cancellation for the cancel's ordinal, including when the pull was
// answered by a terminal frame (the host confirms such a racing cancel).
// After a terminal frame the consumer acknowledges exactly once.
export function createResponseStream({id,sendRaw,read,idleMs=0,onIdle=()=>{},onSettled=()=>{}}){
 const key=JSON.stringify(id.binding);
 const env=(type,data)=>({type,binding:id.binding,sequence:id.sequence,stream:id.stream,...data});
 const mine=f=>{if(JSON.stringify(f.binding)!==key||f.sequence!==id.sequence||f.stream!==id.stream)throw Error('foreign_frame');return f;};
 let ordinal=0,done=false,inflight=null,cancelling=null;
 const settle=()=>{done=true;onSettled();};
 const release=()=>{if(done)return Promise.resolve();if(cancelling)return cancelling;
   cancelling=(async()=>{const my=++ordinal;await sendRaw(env('cancel',{ordinal:my}));
     let f=inflight?await inflight.catch(()=>null):null;
     for(;;){if(f&&f.type==='response-cancelled'&&f.ordinal===my)break;
       if(f&&!['response-chunk','response-end','response-failure'].includes(f.type))throw Error('stream_frame');
       f=mine(await read());}
     settle();})();return cancelling;};
 const body=new ReadableStream({
   async pull(c){if(done||cancelling)return;const my=++ordinal;await sendRaw(env('pull',{ordinal:my}));
     const idle=idleMs?setTimeout(onIdle,idleMs):null;
     inflight=read().then(mine);let f;try{f=await inflight;}finally{clearTimeout(idle);inflight=null;}
     if(cancelling)return;
     if(f.ordinal!==my)throw Error('foreign_frame');
     if(f.type==='response-chunk'){c.enqueue(new Uint8Array(Buffer.from(f.body,'base64')));}
     else if(f.type==='response-end'||f.type==='response-failure'){
       // Settle synchronously so a concurrent release cannot issue a cancel;
       // the ack bytes are written before any later envelope.
       settle();const ack=sendRaw(env('ack',{ordinal:++ordinal}));
       if(f.type==='response-end')c.close();else c.error(Error('relay_'+f.code));await ack;}
     else{settle();throw Error('stream_frame');}},
   cancel(){return release();}
 },{highWaterMark:0});
 return {body,release};
}
// Runs only as the container entrypoint; importing for tests has no effects.
if(import.meta.url==='file:///receiver.mjs')await main();
async function main(){
const observation={writeAttempts:[],networkAttempts:0,streamIdleAborts:0};
net.Socket.prototype.connect=function(){observation.networkAttempts++;throw Error('native_network_refused');};
globalThis.fetch=()=>{observation.networkAttempts++;throw Error('global_fetch_refused');};
syncBuiltinESMExports();
let buffer=Buffer.alloc(0),waiting=null,ended=false;
process.stdin.on('data',b=>{buffer=Buffer.concat([buffer,b]);if(buffer.length>MAX+4){process.exit(3);}pump();});
process.stdin.on('end',()=>{ended=true;pump();});
function pump(){if(!waiting)return;if(buffer.length>=4){const n=buffer.readUInt32BE();if(n===0||n>MAX)process.exit(3);if(buffer.length>=4+n){const b=buffer.subarray(4,4+n);buffer=buffer.subarray(4+n);const w=waiting;waiting=null;try{w.resolve(JSON.parse(b));}catch{w.reject(Error('frame_json'));}return;}}if(ended){waiting.reject(Error('frame_eof'));waiting=null;}}
function read(){if(waiting)throw Error('concurrent_read');return new Promise((resolve,reject)=>{waiting={resolve,reject};pump();});}
// Any open host response stream is released before another envelope, so the
// host relay never observes an interleaved frame.
let open=null;
async function send(v){if(open){const o=open;open=null;await o.release();}await sendRaw(v);}
async function sendRaw(v){const b=Buffer.from(JSON.stringify(v));if(b.length>MAX)throw Error('frame_bound');const h=Buffer.alloc(4);h.writeUInt32BE(b.length);await new Promise((r,j)=>process.stdout.write(Buffer.concat([h,b]),e=>e?j(e):r()));}
let bind,seq=0,stream=0,streamIdleMs=0,session;
const envelope=(type,data={})=>({type,binding:bind,sequence:seq,stream,...data});
const observe=()=>({...observation,scratch:fs.readdirSync("/scratch",{recursive:true})});
function joined(v,type){if(v.type!==type||JSON.stringify(v.binding)!==JSON.stringify(bind)||v.sequence!==seq||v.stream!==stream)throw Error('foreign_frame');}
try {
 const init=await read();if(init.type!=='init'||init.binding?.profile!=='pi-native-inmemory-sse-v1'||init.binding?.generation!==1||init.sequence!==0||init.stream!==0)throw Error('init');bind=init.binding;streamIdleMs=Number.isInteger(init.streamIdleMs)?init.streamIdleMs:0;
 const {createAgentSession,DefaultResourceLoader,ModelRuntime,SessionManager,SettingsManager,createWriteTool,createBashTool}=await import('/pi/node_modules/@earendil-works/pi-coding-agent/dist/index.js');
 const manager=SessionManager.inMemory('/scratch',{id:bind.session});
 const settings=SettingsManager.inMemory({compaction:{enabled:false},retry:{enabled:false},cacheWarming:'off',transport:'sse'});
 const loader=new DefaultResourceLoader({cwd:'/scratch',agentDir:'/scratch/empty',settingsManager:settings,noExtensions:true,disabledBuiltinExtensions:['mcp'],noSkills:true,noPromptTemplates:true,noThemes:true,noContextFiles:true});await loader.reload();
 const marker='syntactic.'+Buffer.from(JSON.stringify({'https://api.openai.com/auth':{chatgpt_account_id:'nonsecret-marker'}})).toString('base64')+'.nonsecret';
 const reject=async()=>{throw Error('credential_mutation');};
 const credentials={read:async p=>p==='openai-codex'?{type:'oauth',access:marker,refresh:'',expires:4102444800000,accountId:'nonsecret-marker'}:undefined,list:async()=>[{providerId:'openai-codex',type:'oauth'}],modify:reject,delete:reject};
 const runtime=await ModelRuntime.create({credentials,modelsPath:null,allowModelNetwork:false,refreshOnCreate:false});
 const model=runtime.getModel('openai-codex','gpt-6-luna');if(!model||model.api!=='openai-codex-responses')throw Error('model');
 let toolOrdinal=0;
 const tools=[createWriteTool('/scratch'),createBashTool('/scratch',{shellPath:'/bin/bash',exposeSessionEnvironment:false})];
 for(const tool of tools){const execute=tool.execute.bind(tool);tool.execute=async(id,parameters,signal,onUpdate,ctx)=>{
   const ordinal=++toolOrdinal;
   await send(envelope('tool',{id,name:tool.name,parameters,toolOrdinal:ordinal}));
   const grant=await read();joined(grant,'tool-grant');
   if(grant.id!==id||grant.toolOrdinal!==ordinal||grant.allow!==true)throw Error('tool_denied');
   // The host's exact-call capability is consumed here, immediately before the
   // actual SDK builtin. No native tool starts before this handshake.
   const result=await execute(id,parameters,signal,onUpdate,ctx);
   await send(envelope('tool-observed',{id,name:tool.name,toolOrdinal:ordinal,result}));
   return result;
 };}
 ({session}=await createAgentSession({cwd:'/scratch',agentDir:'/scratch/empty',sessionManager:manager,settingsManager:settings,resourceLoader:loader,modelRuntime:runtime,model,thinkingLevel:'off',noTools:'builtin',tools:['write','bash'],customTools:tools}));
 // Keep the actual native stream function and parser; inject only fetch transport.
 const original=runtime.streamSimple.bind(runtime);
 runtime.streamSimple=(m,c,o)=>original(m,c,{...o,transport:'sse',maxRetries:0,timeoutMs:10000,cacheRetention:'none',fetch:async(url,req)=>{
   stream++;const body=typeof req.body==='string'?Buffer.from(req.body):Buffer.from(req.body);if(body.length>131072)throw Error('request_bound');
   await send(envelope('fetch',{url:String(url),method:req.method,headers:Object.fromEntries(req.headers),body:body.toString('base64')}));
   const head=await read();if(head.type==='response-unavailable'){joined(head,'response-unavailable');throw Error('route_unavailable_'+head.code);}joined(head,'response-head');if(!Number.isInteger(head.status))throw Error('response_head');
   // The response identity is fixed here; a later fetch cannot change it.
   let handle;handle=createResponseStream({id:{binding:bind,sequence:seq,stream},sendRaw,read,idleMs:streamIdleMs,
     onIdle:()=>{observation.streamIdleAborts++;session.abort();},onSettled:()=>{if(open===handle)open=null;}});
   open=handle;const responseBody=handle.body;
   return new Response(responseBody,{status:head.status,headers:{'content-type':'text/event-stream'}});
 }});
 await send(envelope('ready',{sessionId:session.sessionId,header:manager.getHeader(),tools:session.getActiveToolNames(),observation:observe()}));
 for(;;){const op=await read();if(op.type==='close'){await session.dispose();await send(envelope('closed',{observation:observe()}));process.stdin.destroy();break;}seq++;stream=0;joined(op,'prompt');await session.prompt(op.prompt);await send(envelope('snapshot',{sessionId:session.sessionId,header:manager.getHeader(),entries:manager.getEntries(),leaf:manager.getLeafId(),messages:session.messages,exports:fs.existsSync('/scratch/dist.mjs')?[{name:'dist.mjs',data:fs.readFileSync('/scratch/dist.mjs').toString('base64')}]:[],observation:observe()}));}
}catch{try{await send(envelope('failure',{code:'native_refused',observation}));}catch{}process.exitCode=2;process.stdin.destroy();}
}
