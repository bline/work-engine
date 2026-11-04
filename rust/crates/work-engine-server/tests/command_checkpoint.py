"""Offline command checkpoint against the actual owner, bridge, and fake native socket."""
import hashlib, http.client, json, os, pathlib, shutil, signal, socket, sqlite3, subprocess, sys, tempfile, time, uuid, concurrent.futures
root=pathlib.Path(__file__).resolve().parents[4]
sys.path.insert(0,str(root/'skills/session-events/tests'))
from test_bridge import FakeNative
from test_session_events import CODEX_ID, codex_line
out=pathlib.Path(os.environ['CHECKPOINT_DIR']); out.mkdir(mode=0o700,parents=True,exist_ok=True)
t=pathlib.Path(tempfile.mkdtemp(prefix='ah2c-offline-',dir='/home/bline/.local/state/work-engine')); t.chmod(0o700)
bridge=pathlib.Path(os.environ['BRIDGE_BINARY']); server=pathlib.Path(os.environ['SERVER_BINARY'])
workspace=t/'workspace'; workspace.mkdir(); locks=t/'locks'; locks.mkdir(mode=0o700)
native=FakeNative(t/'native.sock',workspace); proc=None
evidence=t/'evidence'; evidence.mkdir(mode=0o700)
try:
 config=t/'bridge.json'; db=t/'store.sqlite'; request=str(uuid.uuid4()); text='synthetic exact request\n'; inp=t/'input'; inp.write_text(text)
 def cli(*args):
  record={'argv':[str(bridge),'--config',str(config),*map(str,args)],'input_sha256':hashlib.sha256(inp.read_bytes()).hexdigest() if inp.exists() else None,'config_sha256':hashlib.sha256(config.read_bytes()).hexdigest() if config.exists() else None}; f=evidence/f'cli-{uuid.uuid4()}.json'; f.write_text(json.dumps(record)); f.chmod(0o600)
  result=json.loads(subprocess.check_output([str(bridge),'--config',str(config),*map(str,args)],timeout=20)); assert result['ok'],result; return result['result']
 cli('store','init','--database',db,'--lock-dir',locks,'--native-socket',native.path)
 common=['--request-id',request,'--session-id',CODEX_ID,'--workspace',workspace,'--input-file',inp]
 preview=cli('grant','preview',*common); grant=t/'grant'; cli('grant','issue',*common,'--admit-until','2099-01-01T00:00:00Z','--credential-out',grant,'--dedicated-session','--reviewed-wire-sha256',preview['wire_sha256']); token=grant.read_text().strip()
 owner=t/'owner.py'; shutil.copyfile(root/'skills/session-events/scripts/session_events.py',owner)
 pinned=t/'bridge'; shutil.copyfile(bridge,pinned); pinned.chmod(0o700)
 digest=lambda p: hashlib.sha256(p.read_bytes()).hexdigest()
 codex=t/'codex'; claude=t/'claude'; state=t/'events'; claude.mkdir(); state.mkdir(); day=codex/'sessions/2026/11/01'; day.mkdir(parents=True)
 transcript=day/f'rollout-2026-11-01T00-00-00-{CODEX_ID}.jsonl'; transcript.write_text(codex_line('session_meta',id=CODEX_ID,cwd=str(workspace)))
 profile={'version':1,'owner':str(owner),'owner_sha256':digest(owner),'codex_home':str(codex),'claude_home':str(claude),'state_dir':str(state),'bridge':None,'context':None}
 query=t/'query.json'; query.write_text(json.dumps(profile)); query.chmod(0o600)
 profile['bridge']={'binary':str(pinned),'binary_sha256':digest(pinned),'config':str(config),'config_sha256':digest(config)}
 command=t/'command.json'; command.write_text(json.dumps(profile)); command.chmod(0o600)
 credential=t/'http-credential'; credential.write_text('A'*40); credential.chmod(0o600)
 with socket.socket() as sock: sock.bind(('127.0.0.1',0)); port=sock.getsockname()[1]
 cfg=t/'server.json'; cfg.write_text(json.dumps({'version':1,'listen':f'127.0.0.1:{port}','mcp':True,'query_profile':str(query),'command_profile':str(command),'command_capability':'offline-synthetic-v1','credential_file':str(credential),'max_connections':4,'max_queries':2,'request_timeout_seconds':40})); cfg.chmod(0o600)
 unsupported=t/'unsupported.json'; unsupported_data=json.loads(cfg.read_text()); unsupported_data['command_capability']='pi'; unsupported.write_text(json.dumps(unsupported_data)); unsupported.chmod(0o600)
 (evidence/'unsupported.sha256').write_text(hashlib.sha256(unsupported.read_bytes()).hexdigest()); (evidence/'unsupported.sha256').chmod(0o600)
 rejected=subprocess.run([str(server),str(unsupported)],stdin=subprocess.DEVNULL,capture_output=True,text=True,timeout=10); assert rejected.returncode!=0 and 'invalid_profile' in rejected.stderr and token not in rejected.stderr
 # Retain each exact generated startup input before starting the server.
 for f in (owner,pinned,config,query,command,credential,cfg,grant,inp,transcript,root/'skills/session-events/tests/test_bridge.py',root/'skills/session-events/tests/test_session_events.py'):
  dst=evidence/(f.name+'.initial'); shutil.copyfile(f,dst); dst.chmod(0o600)
 (evidence/'startup-manifest.json').write_text(json.dumps({f.name:digest(f) for f in evidence.iterdir()},indent=2)); (evidence/'startup-manifest.json').chmod(0o600)
 def start():
  p=subprocess.Popen([str(server),str(cfg)],stdin=subprocess.PIPE,stdout=subprocess.PIPE,stderr=subprocess.PIPE,text=True)
  for _ in range(300):
   try: s=socket.create_connection(('127.0.0.1',port),.1); s.close(); break
   except OSError: time.sleep(.05)
  else: raise AssertionError('server unavailable')
  return p
 proc=start()
 def snapshot_store(label):
  target=evidence/f'store-{label}.sqlite'
  with sqlite3.connect(db) as source, sqlite3.connect(target) as destination: source.backup(destination)
  target.chmod(0o600); marker=target.with_suffix('.sha256'); marker.write_text(digest(target)); marker.chmod(0o600)
 def rpc(method,params=None):
  snapshot_store('mcp-'+str(uuid.uuid4()))
  line=json.dumps({'jsonrpc':'2.0','id':1,'method':method,'params':params or {}})+'\n'; f=evidence/f'mcp-request-{uuid.uuid4()}.json'; f.write_text(line); f.chmod(0o600); binding=f.with_suffix('.sha256'); binding.write_text(hashlib.sha256(line.encode()).hexdigest()); binding.chmod(0o600)
  proc.stdin.write(line); proc.stdin.flush(); return json.loads(proc.stdout.readline())['result']
 assert rpc('initialize',{'protocolVersion':'2025-06-18','capabilities':{},'clientInfo':{'name':'test','version':'1'}})['protocolVersion']=='2025-06-18'
 assert {x['name'] for x in rpc('tools/list')['tools']} == {'list_sessions','read_session','submit_session_request','read_session_operation','read_session_result'}
 sequence=0
 def call(name,args,ok=True):
  global sequence
  sequence+=1
  body=json.dumps({'schema_version':1,'operation':name,'arguments':args,'request_id':'fixture'})
  ident=str(uuid.uuid4()); snapshot_store('http-'+ident); path=evidence/f'http-request-{ident}.json'; path.write_text(body); path.chmod(0o600)
  transcript_snapshot=evidence/f'transcript-{ident}.jsonl'; shutil.copyfile(transcript,transcript_snapshot); transcript_snapshot.chmod(0o600)
  binding=evidence/f'input-binding-{ident}.json'; binding.write_text(json.dumps({'request_sha256':digest(path),'transcript_sha256':digest(transcript_snapshot),'config_sha256':digest(cfg),'query_profile_sha256':digest(query),'command_profile_sha256':digest(command),'bridge_config_sha256':digest(config),'bridge_sha256':digest(pinned) if pinned.exists() else None,'owner_sha256':digest(owner) if owner.exists() else None})); binding.chmod(0o600)
  conn=http.client.HTTPConnection('127.0.0.1',port,timeout=45)
  conn.request('POST','/v1/session-queries',body,{'Authorization':'Bearer '+'A'*40,'Content-Type':'application/json'}); response=conn.getresponse(); value=json.loads(response.read()); conn.close(); assert response.status==200 and ('owner_result' in value)==ok,value; assert token not in json.dumps(value); return value.get('owner_result',value)
 args={'request_id':request,'grant_credential':token,'session_id':CODEX_ID,'workspace':str(workspace),'input_text':text}
 if os.environ.get('FAULT_BEFORE_ATTEMPT') == '1':
  native.block_method='thread/resume'; native.blocked.clear(); native.release.clear()
  with concurrent.futures.ThreadPoolExecutor(max_workers=1) as pool:
   future=pool.submit(call,'submit_session_request',args,False); assert native.blocked.wait(10)
   with sqlite3.connect(db) as conn: assert conn.execute('SELECT state FROM operations WHERE request_id=?',(request,)).fetchone()[0]=='admitted'
   def descendants(pid):
    pending=[pid]; result=[]
    while pending:
     parent=pending.pop()
     for path in pathlib.Path(f'/proc/{parent}/task').glob('*/children'):
      try: children=[int(x) for x in path.read_text().split()]
      except (FileNotFoundError,ProcessLookupError): continue
      for child in children:
       if child not in result: result.append(child); pending.append(child)
    return result
   matches=[]
   for pid in descendants(proc.pid):
    try: cmd=pathlib.Path(f'/proc/{pid}/cmdline').read_bytes().split(b'\0')
    except FileNotFoundError: continue
    if cmd and cmd[0]==str(pinned).encode() and b'rpc' in cmd: matches.append(pid)
   assert len(matches)==1,matches
   os.kill(matches[0],signal.SIGKILL); native.release.set(); failure=future.result(timeout=45); assert 'owner_error' in failure
  with sqlite3.connect(db) as conn: assert conn.execute('SELECT state FROM operations WHERE request_id=?',(request,)).fetchone()[0]=='admitted'
  refs={'request_id':request,'grant_credential':token}; projection=call('read_session_operation',refs); assert projection['entry_state']=='admitted_no_attempt' and native.starts==0
  retry=call('submit_session_request',args); assert retry['operation_id']==projection['operation_id'] and retry['entry_state']=='admitted_no_attempt' and native.starts==0
  proof={'fixture_dir':str(t),'killed_bridge_pid':matches[0],'request_id':request,'read':projection,'retry':retry,'native_starts':native.starts,'startup_manifest':str(evidence/'startup-manifest.json')}
  (out/'pre-attempt-proof.json').write_text(json.dumps(proof,indent=2)); (out/'pre-attempt-proof.json').chmod(0o600)
  print('PASS killed bridge after admission before attempt; read and retry remain stranded without send',flush=True)
  sys.exit(0)
 # Busy is an owner refusal, not generic query rejection; no native turn may start.
 native.status='active'
 busy_http=call('submit_session_request',args,False); assert busy_http['owner_error']['code']=='session_busy' and token not in json.dumps(busy_http)
 busy_mcp=rpc('tools/call',{'name':'submit_session_request','arguments':args}); assert busy_mcp['isError'] and busy_mcp['structuredContent']['owner_error']['code']=='session_busy' and token not in json.dumps(busy_mcp)
 busy_input=json.dumps({'operation':'submit_session_request','arguments':args}); busy_file=evidence/'client-busy.json'; busy_file.write_text(busy_input); busy_file.chmod(0o600); (evidence/'client-busy.sha256').write_text(digest(busy_file)); (evidence/'client-busy.sha256').chmod(0o600); snapshot_store('client-busy')
 busy_client=json.loads(subprocess.check_output([os.environ['CLIENT_BINARY'],f'127.0.0.1:{port}',str(credential)],input=busy_input,text=True,timeout=45)); assert busy_client['owner_error']['code']=='session_busy' and token not in json.dumps(busy_client) and native.starts==0
 native.status='idle'
 for bad in [dict(args,grant_credential='webridge_bad'),dict(args,workspace=str(t)),dict(args,input_text='wrong')]:
  failure=call('submit_session_request',bad,False); assert failure['owner_error']['code'] in ('unauthorized','target_mismatch','payload_conflict'),failure
 assert native.starts==0
 with sqlite3.connect(db) as conn: conn.execute('UPDATE grants SET expires=0')
 expired=call('submit_session_request',args,False); assert expired['owner_error']['code']=='expired',expired
 with sqlite3.connect(db) as conn: conn.execute('UPDATE grants SET expires=4070908800')
 native.lost_ack=True
 first=call('submit_session_request',args); assert first['entry_state']=='entry_uncertain' and native.starts==1
 saved=native.turns[:]; native.turns=[]
 mcp_submit=rpc('tools/call',{'name':'submit_session_request','arguments':args}); assert mcp_submit['structuredContent']['owner_result']['operation_id']==first['operation_id'] and not mcp_submit['isError']
 client_submit_input=json.dumps({'operation':'submit_session_request','arguments':args}); client_submit_file=evidence/'client-submit.json'; client_submit_file.write_text(client_submit_input); client_submit_file.chmod(0o600); (evidence/'client-submit.sha256').write_text(digest(client_submit_file)); (evidence/'client-submit.sha256').chmod(0o600)
 snapshot_store('client-submit'); client_submit=json.loads(subprocess.check_output([os.environ['CLIENT_BINARY'],f'127.0.0.1:{port}',str(credential)],input=client_submit_input,text=True,timeout=45)); assert client_submit['owner_result']['operation_id']==first['operation_id'] and token not in json.dumps(client_submit) and native.starts==1
 with concurrent.futures.ThreadPoolExecutor(max_workers=2) as pool: retries=list(pool.map(lambda _: call('submit_session_request',args),range(2)))
 assert all(x['operation_id']==first['operation_id'] and x['entry_state']=='entry_uncertain' for x in retries) and native.starts==1
 refs={'request_id':request,'grant_credential':token}; pending=call('read_session_operation',refs); assert pending['entry_state']=='entry_uncertain' and native.starts==1
 with sqlite3.connect(db) as conn: assert conn.execute('SELECT state FROM operations WHERE request_id=?',(request,)).fetchone()[0]=='entry_uncertain'
 # A real process stop cannot turn the durable unknown into settlement.
 proc.send_signal(signal.SIGTERM); proc.wait(timeout=45); first_stderr=proc.stderr.read(); assert proc.returncode==0 and token not in first_stderr and 'drain complete' in first_stderr
 with sqlite3.connect(db) as conn: assert conn.execute('SELECT state FROM operations WHERE request_id=?',(request,)).fetchone()[0]=='entry_uncertain'
 native.turns=saved; proc=start()
 operation=call('read_session_operation',refs); assert operation['operation_id']==first['operation_id'] and operation['entry_state']=='terminal' and native.starts==1
 assert all(k in operation for k in ('workers','usage','process_closure'))
 with transcript.open('a') as f: f.write(codex_line('event_msg',type='task_complete',turn_id=operation['native_turn_id'],last_agent_message='synthetic final result'))
 result=call('read_session_result',refs); assert result['result']['available'] and result['result']['text']=='synthetic final result'
 # The Rust client reads its entire request over stdin and uses the same owner handler.
 req={'operation':'read_session_result','arguments':refs}; client_input=json.dumps(req); client_file=evidence/'client-input.json'; client_file.write_text(client_input); client_file.chmod(0o600); (evidence/'client-input.sha256').write_text(digest(client_file)); (evidence/'client-input.sha256').chmod(0o600)
 snapshot_store('client-result'); client=json.loads(subprocess.check_output([os.environ['CLIENT_BINARY'],f'127.0.0.1:{port}',str(credential)],input=client_input,text=True,timeout=45)); assert client['owner_result']==result and token not in json.dumps(client)
 assert rpc('initialize',{'protocolVersion':'2025-06-18','capabilities':{},'clientInfo':{'name':'test','version':'1'}})['protocolVersion']=='2025-06-18'
 mcp=rpc('tools/call',{'name':'read_session_result','arguments':refs}); assert mcp['structuredContent']['owner_result']==result and not mcp['isError']
 mcp_op=rpc('tools/call',{'name':'read_session_operation','arguments':refs}); assert mcp_op['structuredContent']['owner_result']['operation_id']==operation['operation_id'] and not mcp_op['isError']
 client_op_input=json.dumps({'operation':'read_session_operation','arguments':refs}); client_op_file=evidence/'client-operation.json'; client_op_file.write_text(client_op_input); client_op_file.chmod(0o600); (evidence/'client-operation.sha256').write_text(digest(client_op_file)); (evidence/'client-operation.sha256').chmod(0o600)
 snapshot_store('client-operation'); client_op=json.loads(subprocess.check_output([os.environ['CLIENT_BINARY'],f'127.0.0.1:{port}',str(credential)],input=client_op_input,text=True,timeout=45)); assert client_op['owner_result']['operation_id']==operation['operation_id'] and token not in json.dumps(client_op)
 # Bridge pin/config mismatch and owner outage return static errors.
 pinned.rename(t/'bridge-away'); call('read_session_operation',refs,False); (t/'bridge-away').rename(pinned)
 original=config.read_bytes(); config.write_bytes(original+b' '); call('read_session_operation',refs,False); config.write_bytes(original)
 owner.rename(t/'owner-away'); call('read_session_operation',refs,False); (t/'owner-away').rename(owner)
 cli('grant','revoke','--grant-id',first['grant_id']); revoked=call('read_session_operation',refs,False); assert revoked['owner_error']['code']=='unauthorized',revoked
 # EOF while a helper is in flight closes admission and waits for the helper.
 request2=str(uuid.uuid4()); common2=['--request-id',request2,'--session-id',CODEX_ID,'--workspace',workspace,'--input-file',inp]
 preview2=cli('grant','preview',*common2); grant2=t/'grant2'; cli('grant','issue',*common2,'--admit-until','2099-01-01T00:00:00Z','--credential-out',grant2,'--dedicated-session','--reviewed-wire-sha256',preview2['wire_sha256'])
 shutil.copyfile(grant2,evidence/'grant2.initial'); (evidence/'grant2.initial').chmod(0o600)
 args2=dict(args,request_id=request2,grant_credential=grant2.read_text().strip()); native.block_method='turn/start_after'; native.blocked.clear(); native.release.clear(); native.lost_ack=True
 with concurrent.futures.ThreadPoolExecutor(max_workers=1) as pool:
  future=pool.submit(call,'submit_session_request',args2); assert native.blocked.wait(15)
  proc.stdin.close()
  for _ in range(300):
   try: s=socket.create_connection(('127.0.0.1',port),.05); s.close(); time.sleep(.01)
   except OSError: break
  else: raise AssertionError('EOF did not stop admission')
  native.release.set(); uncertain=future.result(timeout=45); proc.wait(timeout=45); assert proc.returncode==0 and uncertain['entry_state']=='entry_uncertain'
 with sqlite3.connect(db) as conn: assert conn.execute('SELECT state FROM operations WHERE request_id=?',(request2,)).fetchone()[0]=='entry_uncertain'
 assert native.starts==2
 retry=retries[0]
 proof={'busy_http':busy_http,'busy_mcp':busy_mcp,'busy_client':busy_client,'busy_native_starts':0,'first':first,'retry':retry,'operation':operation,'result':result,'native_starts':native.starts,'request_id':request,'owner_sha256':digest(owner),'bridge_sha256':digest(pinned),'bridge_config_sha256':digest(config),'server_sha256':digest(server),'query_profile_sha256':digest(query),'command_profile_sha256':digest(command),'server_config_sha256':digest(cfg),'fixture_dir':str(t),'startup_manifest':str(evidence/'startup-manifest.json'),'request_inputs':str(evidence),'pending':pending,'eof_uncertain':uncertain,'client_result':client['owner_result']}
 (out/'proof.json').write_text(json.dumps(proof,indent=2)); (out/'proof.json').chmod(0o600)
 print('PASS admitted synthetic request, exact retry identity, one native start, enriched operation/result',flush=True)
finally:
 if proc is not None:
  if proc.poll() is None: proc.send_signal(signal.SIGTERM)
  proc.wait(timeout=50); stderr=proc.stderr.read(); assert token not in stderr and (not locals().get('args2') or args2['grant_credential'] not in stderr); print('server exit',proc.returncode,stderr,flush=True)
 native.close()
