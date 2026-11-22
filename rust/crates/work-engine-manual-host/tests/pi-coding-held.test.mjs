// Exact receiver source with the real, read-only Pi 1.5.0 npm tree. The VM
// replaces the external wait/IPC and maps /pi imports; no Docker or provider.
// Run with --experimental-vm-modules and WE_TEST_PI_RUNTIME / WE_TEST_PI_RUNTIME_MANIFEST.
import test from 'node:test';
import assert from 'node:assert/strict';
import vm from 'node:vm';
import fs from 'node:fs';
import net from 'node:net';
import child from 'node:child_process';
import {syncBuiltinESMExports} from 'node:module';
import {createHash} from 'node:crypto';
import {join} from 'node:path';
import {pathToFileURL} from 'node:url';

const root=process.env.WE_TEST_PI_RUNTIME;
const manifest=process.env.WE_TEST_PI_RUNTIME_MANIFEST;
const flag='--held-coding-public';
const source=fs.readFileSync(new URL('../receiver/pi-coding.mjs',import.meta.url),'utf8');
const pin='cf9d096e61d8ffd2c35df238ff74e1f8d7dacfb9bc5813333beef6cc73010985';
const sha=bytes=>createHash('sha256').update(bytes).digest('hex');
const checkPath=p=>/^\/(?:auth|profile|config|workspace|export|spool)(?:\/|$)/.test(String(p).replace(/^file:\/\//,''));

function verifyRuntime(){
 assert.equal(fs.realpathSync(root),root);
 const bytes=fs.readFileSync(manifest);
 assert.equal(sha(bytes),pin);
 const expected=JSON.parse(bytes);
 let seen=0;
 function walk(base){
  for(const d of fs.readdirSync(join(root,base),{withFileTypes:true})){
   const rel=join(base,d.name),path=join(root,rel);
   if(d.isDirectory()){walk(rel);continue;}
   seen++;
   if(d.isSymbolicLink())assert.deepEqual({kind:'symlink',target:fs.readlinkSync(path)},expected[rel],rel);
   else{
    assert(d.isFile(),rel);
    assert.equal(expected[rel]?.kind,'file',rel);
    assert.equal(sha(fs.readFileSync(path)),expected[rel].sha256,rel);
   }
  }
 }
 walk('');
 assert.equal(seen,Object.keys(expected).length);
}

function guardEffects(t){
 const attempts=[];
 const deny=name=>(...args)=>{attempts.push(name);throw Error(`unexpected held effect: ${name}`);};
 for(const api of [fs,fs.promises]){
  for(const name of ['readFile','readFileSync','readdir','readdirSync','stat','statSync','lstat','lstatSync','existsSync','access','accessSync','realpath','realpathSync','createReadStream','open','openSync']){
   const original=api[name];
   if(!original)continue;
   t.mock.method(api,name,function(path,...args){
    if(path===0||checkPath(path))return deny(`input:${String(path)}`)();
    if(name==='open'||name==='openSync'){
     const f=args[0]??'r';
     if(typeof f==='number'?(f&(fs.constants.O_ACCMODE|fs.constants.O_CREAT|fs.constants.O_TRUNC|fs.constants.O_APPEND))!==0:!['r','rs','sr'].includes(f))return deny('open for write')();
    }
    return original.call(this,path,...args);
   });
  }
  for(const name of ['writeFile','writeFileSync','appendFile','appendFileSync','write','writeSync','mkdir','mkdirSync','rename','renameSync','unlink','unlinkSync','rm','rmSync','createWriteStream','truncate','truncateSync']){
   if(api[name])t.mock.method(api,name,deny(name));
  }
 }
 for(const name of ['exec','execSync','execFile','execFileSync','spawn','spawnSync','fork'])t.mock.method(child,name,deny(name));
 t.mock.method(globalThis,'fetch',deny('fetch'));
 t.mock.method(net.Socket.prototype,'connect',deny('socket connect'));
 t.mock.method(net.Server.prototype,'listen',deny('socket listen'));
 syncBuiltinESMExports();
 return {attempts,deny};
}

async function evaluate({argv,offline,deny}){
 const markers=[],sessions=[],waits=[];
 const stopped=new Error('test observed process.exit(2)');
 const process={argv:['node','/receiver/receiver.mjs',...argv],env:{PI_OFFLINE:offline},
  get stdin(){return deny('task_start input')();},
  get stdout(){return deny('stdout IPC')();},
  exit(code){assert.equal(code,2);throw stopped;}};
 const context=vm.createContext({process,setTimeout(fn,ms){waits.push(ms);queueMicrotask(fn);}});
 const linked=new Map();
 async function load(name){
  if(linked.has(name))return linked.get(name);
  const exposed={...await import(name.startsWith('/pi/')?pathToFileURL(join(root,name.slice(4))).href:name)};
  if(name==='node:fs')exposed.writeSync=(fd,string)=>markers.push([fd,string]);
  if(name==='node:readline')exposed.createInterface=deny('task_start dispatcher');
  if(name.endsWith('/pi-coding-agent/dist/index.js')){
   const create=exposed.createAgentSession;
   exposed.createAgentSession=async options=>{
    const result=await create(options);
    sessions.push({options,session:result.session});
    return result;
   };
  }
  const m=new vm.SyntheticModule(Object.keys(exposed),function(){for(const [k,v] of Object.entries(exposed))this.setExport(k,v);},{context});
  linked.set(name,m);
  return m;
 }
 const receiver=new vm.SourceTextModule(source,{context,async importModuleDynamically(name){const m=await load(name);assert.equal(m.status,'evaluated');return m;}});
 await receiver.link(load);
 await assert.rejects(receiver.evaluate(),e=>e===stopped);
 return {markers,sessions,waits};
}

test('exact public Pi receiver constructs an in-memory no-input session and holds without sinks',{skip:!root||!manifest},async t=>{
 verifyRuntime();
 assert.equal(process.env.PI_OFFLINE,'1','run the offline test lane with PI_OFFLINE=1');
 const {attempts,deny}=guardEffects(t);
 try{
  const {markers,sessions,waits}=await evaluate({argv:[flag],offline:'1',deny});
  assert.deepEqual(markers,[[2,'work-engine: inference-entry held\n']]);
  assert.deepEqual(waits,[14400000]);
  assert.equal(sessions.length,1);
  const {options,session}=sessions[0];
  assert.equal(options.cwd,'/__work_engine_public_coding__');
  assert.equal(options.sessionManager.getSessionFile(),undefined);
  assert.equal(options.resourceLoader.getSystemPrompt(),'Public Pi coding initialization only.');
  assert.deepEqual(options.resourceLoader.getExtensions().extensions,[]);
  assert.deepEqual(session.messages,[]);
  assert.deepEqual(session.getActiveToolNames(),[]);
  await assert.rejects(session.agent.streamFunction(),/no provider or credential sink/);
  assert.deepEqual(attempts,[]);
 }finally{t.mock.restoreAll();syncBuiltinESMExports();}
});

test('a held start with crossed flags or without offline environment refuses instead of reading legacy config or input',{skip:!root||!manifest},async t=>{
 const {attempts,deny}=guardEffects(t);
 try{
  for(const [argv,offline] of [[[flag,'--lifecycle-bridge'],'1'],[[flag,flag],'1'],[[flag],'0']]){
   assert.deepEqual(await evaluate({argv,offline,deny}),{markers:[],sessions:[],waits:[]});
  }
  assert.deepEqual(attempts,[]);
 }finally{t.mock.restoreAll();syncBuiltinESMExports();}
});
