// Minimal native SDK interoperability. Host owns dispatch and checked retention.
// No real access material, auth files, tools, discovery or network permission.
import fs from 'node:fs';
import net from 'node:net';
import {syncBuiltinESMExports} from 'node:module';
const MAX=2*1024*1024;
const observation={writeAttempts:[],networkAttempts:0};
const deny=(name)=>(...args)=>{if(observation.writeAttempts.length<16)observation.writeAttempts.push({name,path:String(args[0]).slice(0,256)});throw Error('native_write_refused');};
for(const name of ['writeFile','writeFileSync','appendFile','appendFileSync','mkdir','mkdirSync','rename','renameSync','unlink','unlinkSync','createWriteStream','truncate','truncateSync']) {fs[name]=deny(name);if(fs.promises[name])fs.promises[name]=deny(name);}
for(const name of ['open','openSync']){const orig=fs[name];fs[name]=function(path,flags,...args){if(typeof flags==='string'&&!['r','rs'].includes(flags)||typeof flags==='number'&&(flags&3)!==0)return deny(name)(path);return orig.call(this,path,flags,...args);};}
const origOpen=fs.promises.open;fs.promises.open=async function(path,flags,...args){if(typeof flags==='string'&&!['r','rs'].includes(flags)||typeof flags==='number'&&(flags&3)!==0)return deny('promises.open')(path);return origOpen.call(this,path,flags,...args);};
net.Socket.prototype.connect=function(){observation.networkAttempts++;throw Error('native_network_refused');};
globalThis.fetch=()=>{observation.networkAttempts++;throw Error('global_fetch_refused');};
syncBuiltinESMExports();
let buffer=Buffer.alloc(0),waiting=null,ended=false;
process.stdin.on('data',b=>{buffer=Buffer.concat([buffer,b]);if(buffer.length>MAX+4){process.exit(3);}pump();});
process.stdin.on('end',()=>{ended=true;pump();});
function pump(){if(!waiting)return;if(buffer.length>=4){const n=buffer.readUInt32BE();if(n===0||n>MAX)process.exit(3);if(buffer.length>=4+n){const b=buffer.subarray(4,4+n);buffer=buffer.subarray(4+n);const w=waiting;waiting=null;try{w.resolve(JSON.parse(b));}catch{w.reject(Error('frame_json'));}return;}}if(ended){waiting.reject(Error('frame_eof'));waiting=null;}}
function read(){if(waiting)throw Error('concurrent_read');return new Promise((resolve,reject)=>{waiting={resolve,reject};pump();});}
async function send(v){const b=Buffer.from(JSON.stringify(v));if(b.length>MAX)throw Error('frame_bound');const h=Buffer.alloc(4);h.writeUInt32BE(b.length);await new Promise((r,j)=>process.stdout.write(Buffer.concat([h,b]),e=>e?j(e):r()));}
let bind,seq=0,stream=0;
const envelope=(type,data={})=>({type,binding:bind,sequence:seq,stream,...data});
const observe=()=>({...observation,scratch:fs.readdirSync("/scratch",{recursive:true})});
function joined(v,type){if(v.type!==type||JSON.stringify(v.binding)!==JSON.stringify(bind)||v.sequence!==seq||v.stream!==stream)throw Error('foreign_frame');}
try {
 const init=await read();if(init.type!=='init'||init.binding?.profile!=='pi-native-inmemory-sse-v1'||init.binding?.generation!==1||init.sequence!==0||init.stream!==0)throw Error('init');bind=init.binding;
 const {createAgentSession,DefaultResourceLoader,ModelRuntime,SessionManager,SettingsManager}=await import('/pi/node_modules/@earendil-works/pi-coding-agent/dist/index.js');
 const manager=SessionManager.inMemory('/scratch',{id:bind.session});
 const settings=SettingsManager.inMemory({compaction:{enabled:false},retry:{enabled:false},cacheWarming:'off',transport:'sse'});
 const loader=new DefaultResourceLoader({cwd:'/scratch',agentDir:'/scratch/empty',settingsManager:settings,noExtensions:true,disabledBuiltinExtensions:['mcp'],noSkills:true,noPromptTemplates:true,noThemes:true,noContextFiles:true});await loader.reload();
 const marker='syntactic.'+Buffer.from(JSON.stringify({'https://api.openai.com/auth':{chatgpt_account_id:'nonsecret-marker'}})).toString('base64')+'.nonsecret';
 const reject=async()=>{throw Error('credential_mutation');};
 const credentials={read:async p=>p==='openai-codex'?{type:'oauth',access:marker,refresh:'',expires:4102444800000,accountId:'nonsecret-marker'}:undefined,list:async()=>[{providerId:'openai-codex',type:'oauth'}],modify:reject,delete:reject};
 const runtime=await ModelRuntime.create({credentials,modelsPath:null,allowModelNetwork:false,refreshOnCreate:false});
 const model=runtime.getModel('openai-codex','gpt-6-luna');if(!model||model.api!=='openai-codex-responses')throw Error('model');
 const {session}=await createAgentSession({cwd:'/scratch',agentDir:'/scratch/empty',sessionManager:manager,settingsManager:settings,resourceLoader:loader,modelRuntime:runtime,model,thinkingLevel:'off',noTools:true,tools:[]});
 // Keep the actual native stream function and parser; inject only fetch transport.
 const original=runtime.streamSimple.bind(runtime);
 runtime.streamSimple=(m,c,o)=>original(m,c,{...o,transport:'sse',maxRetries:0,timeoutMs:10000,cacheRetention:'none',fetch:async(url,req)=>{
   stream++;const body=typeof req.body==='string'?Buffer.from(req.body):Buffer.from(req.body);if(body.length>131072)throw Error('request_bound');
   await send(envelope('fetch',{url:String(url),method:req.method,headers:Object.fromEntries(req.headers),body:body.toString('base64')}));
   const answer=await read();joined(answer,'response');return new Response(Buffer.from(answer.body,'base64'),{status:answer.status,headers:{'content-type':'text/event-stream'}});
 }});
 await send(envelope('ready',{sessionId:session.sessionId,header:manager.getHeader(),tools:session.getActiveToolNames(),observation:observe()}));
 for(;;){const op=await read();if(op.type==='close'){await session.dispose();await send(envelope('closed',{observation:observe()}));process.stdin.destroy();break;}seq++;stream=0;joined(op,'prompt');await session.prompt(op.prompt);await send(envelope('snapshot',{sessionId:session.sessionId,header:manager.getHeader(),entries:manager.getEntries(),leaf:manager.getLeafId(),messages:session.messages,observation:observe()}));}
}catch{try{await send(envelope('failure',{code:'native_refused',observation}));}catch{}process.exitCode=2;process.stdin.destroy();}
