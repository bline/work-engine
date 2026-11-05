import concurrent.futures,hashlib,json,os,pathlib,shutil,socket,sqlite3,subprocess,sys,time,uuid
root=pathlib.Path(os.environ['REPO_ROOT']); out=pathlib.Path(os.environ['BUILDER_DIR']); t=pathlib.Path(os.environ['FIXTURE_DIR']); t.mkdir(mode=0o700); os.umask(0o077)
sys.path.insert(0,str(root/'skills/session-events/tests'))
from test_bridge import FakeNative
from test_session_events import CODEX_ID,codex_line
bridge=pathlib.Path(os.environ['BRIDGE_BINARY']); server=pathlib.Path(os.environ['SERVER_BINARY'])
pinned=t/'bridge'; shutil.copyfile(bridge,pinned); pinned.chmod(0o700); bridge=pinned
workspace=t/'workspace'; workspace.mkdir(mode=0o700); locks=t/'locks'; locks.mkdir(mode=0o700); native=FakeNative(t/'native.sock',workspace); proc=None
try:
 config=t/'bridge.json'; db=t/'store.sqlite'; request=str(uuid.uuid4()); text='synthetic exact request\n'; inp=t/'input'; inp.write_text(text); inp.chmod(0o600)
 def cli(*args):
  r=json.loads(subprocess.check_output([str(bridge),'--config',str(config),*map(str,args)],timeout=20)); assert r['ok'],r; return r['result']
 cli('store','init','--database',db,'--lock-dir',locks,'--native-socket',native.path)
 common=['--request-id',request,'--session-id',CODEX_ID,'--workspace',workspace,'--input-file',inp]; preview=cli('grant','preview',*common); grant=t/'grant'; cli('grant','issue',*common,'--admit-until','2099-01-01T00:00:00Z','--credential-out',grant,'--dedicated-session','--reviewed-wire-sha256',preview['wire_sha256']); token=grant.read_text().strip()
 owner=t/'owner.py'; shutil.copyfile(root/'skills/session-events/scripts/session_events.py',owner)
 digest=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
 codex=t/'codex'; claude=t/'claude'; state=t/'events'; claude.mkdir(); state.mkdir(); day=codex/'sessions/2026/11/01'; day.mkdir(parents=True); transcript=day/f'rollout-2026-11-01T00-00-00-{CODEX_ID}.jsonl'; transcript.write_text(codex_line('session_meta',id=CODEX_ID,cwd=str(workspace)))
 profile={'version':1,'owner':str(owner),'owner_sha256':digest(owner),'codex_home':str(codex),'claude_home':str(claude),'state_dir':str(state),'bridge':None,'context':None}
 def write(p,v): p.write_text(json.dumps(v)); p.chmod(0o600)
 query=t/'query.json'; write(query,profile); profile['bridge']={'binary':str(bridge),'binary_sha256':digest(bridge),'config':str(config),'config_sha256':digest(config)}; command=t/'command.json'; write(command,profile)
 credential=t/'http-credential'; credential.write_text('A'*40); credential.chmod(0o600)
 with socket.socket() as sock: sock.bind(('127.0.0.1',0)); port=sock.getsockname()[1]
 cfg=t/'server.json'; write(cfg,{'version':1,'listen':f'127.0.0.1:{port}','mcp':False,'query_profile':str(query),'command_profile':str(command),'command_capability':'offline-synthetic-v1','credential_file':str(credential),'max_connections':4,'max_queries':2,'request_timeout_seconds':40})
 cp=t/'client.json'; write(cp,{'version':1,'kind':'local-service','endpoint':f'127.0.0.1:{port}','credential_file':str(credential),'timeout_seconds':45})
 args={'request_id':request,'grant_credential':token,'session_id':CODEX_ID,'workspace':str(workspace),'input_text':text}; argfile=t/'submit.json'; write(argfile,args); refs=t/'refs.json'; write(refs,{'request_id':request,'grant_credential':token})
 with sqlite3.connect(db) as src,sqlite3.connect(t/'store.initial.sqlite') as dst: src.backup(dst)
 proc=subprocess.Popen([str(server),str(cfg)],stdin=subprocess.DEVNULL,stdout=subprocess.PIPE,stderr=subprocess.PIPE,text=True)
 for _ in range(300):
  try: s=socket.create_connection(('127.0.0.1',port),.1); s.close(); break
  except OSError: time.sleep(.05)
 else: raise RuntimeError('server unavailable')
 def invoke(exe,op,file,name):
  r=subprocess.run([str(out/exe),str(cp),op,'--json',str(file)],capture_output=True,text=True,timeout=55); (t/(name+'.stdout')).write_text(r.stdout); (t/(name+'.stderr')).write_text(r.stderr); assert r.returncode==0,(name,r.stdout,r.stderr); return json.loads(r.stdout)['envelope']['owner_result']
 def reject(label, data, expected):
  f=t/(label+'.json'); write(f,data)
  r=subprocess.run([str(out/'work-engine-command-gen4'),str(cp),'submit_session_request','--json',str(f)],capture_output=True,text=True,timeout=50)
  (t/(label+'.stdout')).write_text(r.stdout); (t/(label+'.stderr')).write_text(r.stderr)
  v=json.loads(r.stdout); assert r.returncode and v['envelope']['owner_error']['code']==expected,(label,v)
  assert token not in r.stdout+r.stderr and 'A'*40 not in r.stdout+r.stderr
 native.status='active'; reject('busy',args,'session_busy'); native.status='idle'
 reject('bad-grant',dict(args,grant_credential='webridge_bad'),'unauthorized')
 reject('bad-workspace',dict(args,workspace=str(t)),'target_mismatch')
 reject('bad-payload',dict(args,input_text='wrong'),'payload_conflict')
 with sqlite3.connect(db) as conn: conn.execute('UPDATE grants SET expires=0')
 reject('expired',args,'expired')
 with sqlite3.connect(db) as conn: conn.execute('UPDATE grants SET expires=4070908800')
 assert native.starts==0
 native.lost_ack=True
 first=invoke('work-engine-command-gen4' ,'submit_session_request',argfile,'first'); saved=native.turns[:]; native.turns=[]; repeat=invoke('work-engine-command-gen4','submit_session_request',argfile,'repeat');
 with concurrent.futures.ThreadPoolExecutor(max_workers=2) as pool:
  concurrent=list(pool.map(lambda n: invoke('work-engine-command-gen4','submit_session_request',argfile,'concurrent'+str(n)),range(2)))
 assert all(x['operation_id']==first['operation_id'] and x['entry_state']=='entry_uncertain' for x in concurrent) and native.starts==1
 pending=invoke('work-engine-query-gen2','read_session_operation',refs,'pending'); assert pending['entry_state']=='entry_uncertain'
 with sqlite3.connect(db) as conn: assert conn.execute('SELECT state FROM operations WHERE request_id=?',(request,)).fetchone()[0]=='entry_uncertain'
 proc.terminate(); oldout,olderr=proc.communicate(timeout=50); assert proc.returncode==0; (t/'server-first.stderr').write_text(olderr)
 native.turns=saved; proc=subprocess.Popen([str(server),str(cfg)],stdin=subprocess.DEVNULL,stdout=subprocess.PIPE,stderr=subprocess.PIPE,text=True)
 for _ in range(300):
  try: sock=socket.create_connection(('127.0.0.1',port),.1); sock.close(); break
  except OSError: time.sleep(.05)
 else: raise RuntimeError('restart unavailable')
 read=invoke('work-engine-query-gen2','read_session_operation',refs,'read'); result=invoke('work-engine-query-gen2','read_session_result',refs,'result')
 assert first['entry_state'] in ('admitted','entry_uncertain'),first
 assert first['operation_id']==repeat['operation_id']==read['operation_id']; assert native.starts==1,native.starts
 with sqlite3.connect(db) as conn: row=conn.execute('SELECT state FROM operations WHERE request_id=?',(request,)).fetchone()
 assert row and row[0] in ('entry_uncertain','terminal'),(row,first)
 cli('grant','revoke','--grant-id',first['grant_id']); reject('revoked',args,'unauthorized')
 owner.rename(t/'owner-away'); reject('owner-unavailable',args,'owner_unavailable'); (t/'owner-away').rename(owner)
 # Client timeout and process disconnect are observed while the native call is blocked.
 timeout_profile=t/'short-profile.json'; write(timeout_profile,dict(json.loads(cp.read_text()),timeout_seconds=1))
 failure_observations=[]
 for label in ('timeout','disconnect'):
  rid=str(uuid.uuid4()); common2=['--request-id',rid,'--session-id',CODEX_ID,'--workspace',workspace,'--input-file',inp]
  preview2=cli('grant','preview',*common2); grant2=t/(label+'-grant'); cli('grant','issue',*common2,'--admit-until','2099-01-01T00:00:00Z','--credential-out',grant2,'--dedicated-session','--reviewed-wire-sha256',preview2['wire_sha256'])
  arg2=t/(label+'-input.json'); write(arg2,dict(args,request_id=rid,grant_credential=grant2.read_text().strip()))
  native.block_method='turn/start_after'; native.blocked.clear(); native.release.clear(); native.lost_ack=True; before=native.starts
  client=subprocess.Popen([str(out/'work-engine-command-gen4'),str(timeout_profile if label=='timeout' else cp),'submit_session_request','--json',str(arg2)],stdout=subprocess.PIPE,stderr=subprocess.PIPE,text=True)
  assert native.blocked.wait(15),label
  if label=='disconnect': client.kill()
  clientout,clienterr=client.communicate(timeout=8); (t/(label+'.stdout')).write_text(clientout); (t/(label+'.stderr')).write_text(clienterr)
  if label=='timeout': assert json.loads(clientout)['error']['code'] in ('transport_timeout','transport_unavailable')
  native.release.set(); native.block_method=None
  for _ in range(300):
   with sqlite3.connect(db) as conn: state2=conn.execute('SELECT state FROM operations WHERE request_id=?',(rid,)).fetchone()
   if state2 and state2[0]=='entry_uncertain': break
   time.sleep(.05)
  assert state2 and state2[0]=='entry_uncertain' and native.starts==before+1,(label,state2,native.starts,before)
  repeated=invoke('work-engine-command-gen4','submit_session_request',arg2,label+'-repeat'); assert repeated['request_id']==rid and native.starts==before+1
  failure_observations.append({'case':label,'request_id':rid,'operation_id':repeated['operation_id'],'store_state':state2[0],'native_entries':native.starts-before,'client_exit':client.returncode})
  refs2=t/(label+'-refs.json'); write(refs2,{'request_id':rid,'grant_credential':grant2.read_text().strip()}); reconciled=invoke('work-engine-query-gen2','read_session_operation',refs2,label+'-read'); assert reconciled['entry_state']=='terminal' and reconciled['operation_id']==repeated['operation_id']
 write(t/'failure-observations.json',failure_observations)
 write(t/'observation.json',{'request_id':request,'first':first,'repeat':repeat,'read':read,'result':result,'native_starts':native.starts,'owner_store_state':row[0]})
 print('PROOF',json.dumps({'operation_id':first['operation_id'],'entry_state':first['entry_state'],'native_starts':native.starts}))
finally:
 if proc:
  proc.terminate()
  try: stdout,stderr=proc.communicate(timeout=50)
  except subprocess.TimeoutExpired: proc.kill(); stdout,stderr=proc.communicate()
  (t/'server.stdout').write_text(stdout); (t/'server.stderr').write_text(stderr); print('server exit',proc.returncode)
 native.close()
