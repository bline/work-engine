import assert from "node:assert/strict";
import { mkdtemp, rm, unlink, writeFile } from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import test from "node:test";
import { CodexAppServerAdapter } from "../src/codex-app-server-adapter.mjs";
import { FileRoleBindingRegistry, BindingConflictError } from "../src/role-binding-registry.mjs";
import { RetainedTurnOutputStore } from "../src/retained-turn-output-store.mjs";

class Transport {
  constructor(){this.handlers=[];}
  onServerRequest(){} onClosed(){}
  onNotification(handler){this.handlers.push(handler);return()=>{};}
  emit(value){for(const handler of this.handlers) handler(value);}
}
function completed(threadId,turnId,text){return {method:"turn/completed",params:{threadId,turn:{
  id:turnId,status:"completed",items:text===null?[]:[{type:"agentMessage",phase:"final_answer",text}]}}};}
async function delivery(registry,{role="builder:one",message="message-1",thread="thread-1",turn="turn-1"}={}){
  await registry.beginDelivery({logicalRoleInstanceId:role,clientUserMessageId:message,threadId:thread,requestFingerprint:"fingerprint"});
  await registry.completeDelivery({logicalRoleInstanceId:role,clientUserMessageId:message,threadId:thread,turnId:turn});
}
function adapter(transport,registry,store){return new CodexAppServerAdapter({transport,registry,
  retainedTurnOutputStore:store,skillResolver:{}});}

test("completed output writes blob then immutable metadata and replays after restart",async(t)=>{
  const root=await mkdtemp(path.join(os.tmpdir(),"adapter-output-"));t.after(()=>rm(root,{recursive:true,force:true}));
  const registry=new FileRoleBindingRegistry(path.join(root,"bindings.json"));
  const store=new RetainedTurnOutputStore(path.join(root,"retained-turn-outputs"));await delivery(registry);
  const firstTransport=new Transport(),first=adapter(firstTransport,registry,undefined);
  const waiting=first.waitForTurnCompletion({threadId:"thread-1",turnId:"turn-1"});
  firstTransport.emit(completed("thread-1","turn-1","exact bytes"));
  assert.equal((await waiting).outputText,"exact bytes");
  const metadata=(await registry.getDeliveryByTurn("thread-1","turn-1")).terminalOutput;
  assert.equal(metadata.availability,"available");
  const restarted=adapter(new Transport(),new FileRoleBindingRegistry(path.join(root,"bindings.json")),undefined);
  const replay=await restarted.waitForTurnCompletion({threadId:"thread-1",turnId:"turn-1",replayedDelivery:true});
  assert.equal(replay.outputText,"exact bytes");assert.equal(replay.outputDigest,metadata.digest);
  assert.equal((await registry.attachTerminalOutput({threadId:"thread-1",turnId:"turn-1",terminalOutput:metadata})).status,"replayed");
  await assert.rejects(registry.attachTerminalOutput({threadId:"thread-1",turnId:"turn-1",
    terminalOutput:{...metadata,byteLength:99}}),BindingConflictError);
});

test("legacy absence and missing blob are truthful and never inferred",async(t)=>{
  const root=await mkdtemp(path.join(os.tmpdir(),"adapter-output-"));t.after(()=>rm(root,{recursive:true,force:true}));
  const registry=new FileRoleBindingRegistry(path.join(root,"bindings.json"));const store=new RetainedTurnOutputStore(path.join(root,"outputs"));
  await delivery(registry);
  let replay=await adapter(new Transport(),registry,store).waitForTurnCompletion({threadId:"thread-1",turnId:"turn-1",replayedDelivery:true});
  assert.equal(replay.outputAvailability,"unavailable");assert.equal(replay.reason,"legacy_output_metadata_absent");
  const transport=new Transport(),live=adapter(transport,registry,store);
  const wait=live.waitForTurnCompletion({threadId:"thread-1",turnId:"turn-1"});transport.emit(completed("thread-1","turn-1","durable"));await wait;
  const metadata=(await registry.getDeliveryByTurn("thread-1","turn-1")).terminalOutput;
  await unlink(path.join(root,"outputs",metadata.digest));
  replay=await adapter(new Transport(),registry,store).waitForTurnCompletion({threadId:"thread-1",turnId:"turn-1",replayedDelivery:true});
  assert.equal(replay.outputAvailability,"unavailable");assert.equal(replay.reason,"blob_missing");
  await writeFile(path.join(root,"outputs",metadata.digest),"corrupt");
  replay=await adapter(new Transport(),registry,store).waitForTurnCompletion({threadId:"thread-1",turnId:"turn-1",replayedDelivery:true});
  assert.equal(replay.outputAvailability,"integrity_failed");assert.equal(replay.reason,"blob_digest_mismatch");
});
