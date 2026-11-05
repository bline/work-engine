import hashlib,json,os,pathlib,shutil,socket,sqlite3,subprocess,time,signal
T=pathlib.Path(os.environ['FIXTURE_DIR']); R=pathlib.Path(os.environ['REPO_ROOT']); F=pathlib.Path(os.environ['RUNTIME_FREEZE']); B=pathlib.Path(os.environ['BUILDER_DIR'])
os.umask(0o077)
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
def put(p,v):p.write_text(json.dumps(v));p.chmod(0o600)
sid='11111111-1111-4111-8111-111111111111'; req='22222222-2222-4222-8222-222222222222'; gid='33333333-3333-4333-8333-333333333333'; opid='44444444-4444-4444-8444-444444444444'; turn='55555555-5555-4555-8555-555555555555'; token='webridge_'+'A'*43
c=T/'codex'; cl=T/'claude'; state=T/'state'; workspace=T/'workspace'; locks=T/'locks'
for p in (cl,state,workspace,locks):p.mkdir(mode=0o700)
roll=c/('sessions/2026/01/01/rollout-fixture-'+sid+'.jsonl');roll.parent.mkdir(parents=True)
records=[{'type':'session_meta','timestamp':'2026-01-01T00:00:00Z','payload':{'cwd':str(workspace),'timestamp':'2026-01-01T00:00:00Z'}},{'type':'event_msg','timestamp':'2026-01-01T00:00:02Z','payload':{'type':'task_complete','turn_id':turn,'last_agent_message':'synthetic checkpoint result '+token+' \x1b]52;c;example\x07\x9b\r'}}];roll.write_text(''.join(json.dumps(x)+'\n' for x in records));os.utime(roll,(1767225600,1767225600))
(c/'session_index.jsonl').write_text(json.dumps({'id':sid,'thread_name':'fixture'})+'\n')
bridge=T/'bridge';shutil.copyfile(F/'session-bridge',bridge);bridge.chmod(0o700);assert sha(bridge)=='fb54335946eaf20bdb70e0d849c3c9548aa6a9aff56b785467adb25c565d134a'
server=T/'server';shutil.copyfile(F/'work-engine-server',server);server.chmod(0o700);assert sha(server)=='e150213c670b070ede4501af29788777b3ae318ce33f1eb2df05e5bf9f67ddac'
owner=T/'owner.py';shutil.copyfile(R/'skills/session-events/scripts/session_events.py',owner);assert sha(owner)=='d5d51ab3a870a6efa82f5824a4e3cb9ba5468263724f6068c0d46b5ec586b8a2'
config=T/'bridge.json';db=T/'state.sqlite'; subprocess.run([str(bridge),'--config',str(config),'store','init','--database',str(db),'--lock-dir',str(locks),'--native-socket',str(T/'unused.sock')],check=True,stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL,timeout=10)
h=lambda b:hashlib.sha256(b).hexdigest(); wire='preexisting wire'; wh=h(wire.encode()); ph=h(b'preexisting payload'); ident={'protocol':'session-bridge-rpc-v1','envelope':'runtime-envelope-v1','profile':'dedicated-loaded-session-v1','request_id':req,'session_id':sid,'workspace':str(workspace),'payload_sha256':ph,'wire_sha256':wh}; oh=h(json.dumps(ident,separators=(',',':'),ensure_ascii=False).encode()); th=h(('work-engine/session-bridge/token/v1\0'+token).encode()); (locks/(opid+'.input')).write_text(wire)
with sqlite3.connect(db) as sql:
 sql.execute('INSERT INTO grants VALUES (?,?,?,?,?,?,?,?,?,?,?,?)',(gid,th,req,sid,str(workspace),ph,wh,oh,4102444800,os.geteuid(),1767225600,0));sql.execute('INSERT INTO operations VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?)',(req,opid,gid,sid,str(workspace),ph,wh,oh,opid,1767225600,'terminal','[]',turn,turn,'completed',1767225600))
shutil.copyfile(db,T/'state-before.sqlite')
profile={'version':1,'owner':str(owner),'owner_sha256':sha(owner),'codex_home':str(c),'claude_home':str(cl),'state_dir':str(state),'bridge':None};query=T/'query.json';put(query,profile); profile['bridge']={'binary':str(bridge),'binary_sha256':sha(bridge),'config':str(config),'config_sha256':sha(config)};command=T/'command.json';put(command,profile)
cred=T/'credential';cred.write_text('B'*40);cred.chmod(0o600)
with socket.socket() as s:s.bind(('127.0.0.1',0));port=s.getsockname()[1]
configserver=T/'server.json';put(configserver,{'version':1,'listen':f'127.0.0.1:{port}','mcp':False,'query_profile':str(query),'command_profile':str(command),'command_capability':'offline-synthetic-v1','credential_file':str(cred),'max_connections':4,'max_queries':2,'request_timeout_seconds':40})
cli_profile=T/'cli-profile.json';put(cli_profile,{'version':1,'kind':'local-service','endpoint':f'127.0.0.1:{port}','credential_file':str(cred),'timeout_seconds':45}); inp=T/'request.json';put(inp,{'request_id':req,'grant_credential':token})

# All fixture inputs are recorded before any CLI/server invocation.
inputs=[p for p in T.rglob('*') if p.is_file()]
put(T/'fixture-input-manifest.json',{'inputs':[{'path':str(p),'sha256':sha(p)} for p in inputs], 'domain_request_id':req,'operation_id':opid,'transport_credential_sha256':sha(cred),'domain_credential_sha256':h(token.encode()),'native_socket':'absent and never started'})
cli=pathlib.Path(os.environ['CLI_BINARY']); n=0

def run(name,args,fmt='--json',expected=0,profile=cli_profile):
 global n
 n+=1; request=T/f'request-{n}.json';put(request,args)
 p=subprocess.run([str(cli),str(profile),name,fmt,str(request)],capture_output=True,timeout=50)
 (T/f'output-{n}').write_bytes(p.stdout);(T/f'error-{n}').write_bytes(p.stderr)
 assert p.returncode==expected,(name,n,p.returncode)
 assert token.encode() not in p.stdout+p.stderr and cred.read_bytes() not in p.stdout+p.stderr
 return json.loads(p.stdout) if fmt=='--json' else p.stdout.decode()

def start():
 p=subprocess.Popen([str(server),str(configserver)],stdin=subprocess.DEVNULL,stdout=subprocess.DEVNULL,stderr=subprocess.PIPE)
 for _ in range(200):
  assert p.poll() is None
  try:
   with socket.create_connection(('127.0.0.1',port),.1):return p
  except OSError:time.sleep(.05)
 raise AssertionError('server unavailable')
def stop(p,label):
 p.send_signal(signal.SIGTERM)
 try: _,err=p.communicate(timeout=50)
 except subprocess.TimeoutExpired:p.kill();p.communicate();raise
 (T/f'server-{label}.stderr').write_bytes(err);assert p.returncode==0 and b'drain complete' in err
 with socket.socket() as s:assert s.connect_ex(('127.0.0.1',port))!=0
refs={'request_id':req,'grant_credential':token};p=start()
try:
 wrong=T/'wrong-transport';wrong.write_text('C'*40);wrong.chmod(0o600);bad_profile=T/'bad-transport.json';put(bad_profile,{'version':1,'kind':'local-service','endpoint':f'127.0.0.1:{port}','credential_file':str(wrong),'timeout_seconds':10})
 x=run('read_session_operation',refs,expected=1,profile=bad_profile);assert x['transport_error']['code']=='transport_rejected'
 for invalid in ({'version':2},{'timeout_seconds':0},{'extra':1},{'endpoint':'0.0.0.0:1'},{'timeout_seconds':'1'}):
  profile_data=json.loads(cli_profile.read_text());profile_data.update(invalid);bad=T/f'invalid-profile-{len(invalid)}-{n}.json';put(bad,profile_data);x=run('read_session_operation',refs,expected=1,profile=bad);assert x['error']['code']=='invalid_profile'
 for name in ('list_sessions','read_session'):
  a={} if name=='list_sessions' else {'session_id':sid};v=run(name,a);assert v['ok'] and v['schema']=='work-engine.session-query.service.v1' and v['envelope']['schema_version']==1
  if name=='list_sessions':assert v['envelope']['owner_result']['matched']==1
  else:assert v['envelope']['owner_result']['sessionId']==sid
 op=run('read_session_operation',refs);o=op['envelope']['owner_result'];assert op['ok'] and o['operation_id']==opid and o['request_id']==req and o['entry_state']=='terminal' and o['native_status']=='completed' and o['usage']['available'] is False and o['process_closure']['available'] is False
 human=run('read_session_operation',refs,'--human');assert 'Operation '+opid+': terminal' in human and 'Usage: unavailable' in human and 'Process closure: unavailable' in human
 result=run('read_session_result',refs);v=result['envelope']['owner_result'];assert result['ok'] and v['operation']['operation_id']==opid and v['completion']['status']=='completed' and v['result']['text'].startswith('synthetic checkpoint result ') and '[REDACTED:bridge_credential]' in v['result']['text'] and token not in v['result']['text'] and v['workers']['available'] is False
 human=run('read_session_result',refs,'--human');assert 'Result: available' in human and 'synthetic checkpoint result' in human and 'Completion: completed' in human and 'Workers: unavailable' in human and '\\u{1b}' in human and '\\u{9b}' in human and '\x1b' not in human
 page=run('read_session_result',dict(refs,text_offset=10,max_chars=5))['envelope']['owner_result']['result'];assert page['text']=='check' and page['text_offset']==10 and page['truncated']; boundary=run('read_session_result',dict(refs,text_offset=30,max_chars=8))['envelope']['owner_result']['result'];assert token not in boundary['text'] and 'webridge_' not in boundary['text']
 for name,args,code in [('read_session_operation',{'request_id':req},'invalid_arguments'),('read_session_operation',dict(refs,grant_credential='wrong'),'unauthorized'),('read_session_result',dict(refs,request_id=sid),'unauthorized'),('read_session_result',dict(refs,extra=True),'invalid_arguments'),('submit_session_request',{},'invalid_query'),('unknown',{},'invalid_query')]:
  x=run(name,args,expected=1); e=x.get('error') or x['envelope'].get('owner_error');assert e['code']==code,(name,e)
 # Repeat same credential and request identity before and after a server restart.
 assert run('read_session_operation',refs)['envelope']['owner_result']['operation_id']==opid
finally:stop(p,'first')
p=start()
try:assert run('read_session_result',refs)['envelope']['owner_result']['operation']['operation_id']==opid
finally:stop(p,'restart')
# Change only private preexisting test grant state while server is stopped.
for label,statement in [('revoked','UPDATE grants SET revoked=1'),('expired','UPDATE grants SET revoked=0, expires=1')]:
 try:
  with sqlite3.connect(db) as sql:sql.execute(statement)
 except sqlite3.OperationalError:
  # Schema names are inspected below if the fixture version differs.
  with sqlite3.connect(db) as sql:print('fixture schema',sql.execute('PRAGMA table_info(grants)').fetchall());raise
 p=start()
 try:
  
  if label=='revoked':
   x=run('read_session_operation',refs,expected=1);assert x['envelope']['owner_error']['code']=='unauthorized'
  else:
   # Expiry limits admission, not a read of this already admitted operation.
   x=run('read_session_operation',refs);assert x['envelope']['owner_result']['operation_id']==opid
 finally:stop(p,label)

# Missing transcript produces unavailable completion/result rather than settlement.
roll.rename(T/'transcript-away');p=start()
try:
 v=run('read_session_result',refs)['envelope']['owner_result'];assert not v['completion']['available'] and not v['result']['available']
 human=run('read_session_result',refs,'--human');assert 'Result: unavailable' in human and 'Completion: unavailable' in human
finally:stop(p,'unavailable')
roll=(T/'transcript-away');roll.rename(c/('sessions/2026/01/01/rollout-fixture-'+sid+'.jsonl'))
with sqlite3.connect(db) as sql:
 sql.execute("UPDATE operations SET state='identity_conflict', native_status=NULL, turn_id=NULL WHERE request_id=?",(req,));sql.execute('INSERT INTO occupancy VALUES (?,?)',(sid,req))
p=start()
try:
 v=run('read_session_operation',refs)['envelope']['owner_result'];assert v['entry_state']=='identity_conflict' and v['native_status'] is None
 assert 'identity_conflict' in run('read_session_operation',refs,'--human')
finally:stop(p,'uncertain')

with sqlite3.connect(db) as sql:
 assert sql.execute('select count(*) from grants').fetchone()[0]==1 and sql.execute('select count(*) from operations').fetchone()[0]==1 and sql.execute('select count(*) from transitions').fetchone()[0]==0
assert not (T/'unused.sock').exists();print('PASS service list/read, both granted reads, human, paging, errors, restart, grant states, no native send')
# Synthetic HTTP responder exercises presentation and transport failures, not owner authorization.
import threading

def responder(envelope, fmt, name, args, *, delay=0):
 with socket.socket() as listener:
  listener.bind(('127.0.0.1',0));listener.listen();addr=listener.getsockname(); profile=T/f'responder-{n}.json';put(profile,{'version':1,'kind':'local-service','endpoint':f'{addr[0]}:{addr[1]}','credential_file':str(cred),'timeout_seconds':1 if delay else 10})
  def serve():
   conn,_=listener.accept()
   with conn:
    conn.settimeout(5);data=b''
    while b'\r\n\r\n' not in data:data+=conn.recv(4096)
    headers,body=data.split(b'\r\n\r\n',1);length=int(next(x.split(b':',1)[1] for x in headers.split(b'\r\n') if x.lower().startswith(b'content-length:')))
    while len(body)<length:body+=conn.recv(4096)
    if delay:time.sleep(delay);return
    raw=json.dumps(envelope).encode();conn.sendall(b'HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: '+str(len(raw)).encode()+b'\r\n\r\n'+raw)
  thread=threading.Thread(target=serve);thread.start()
  try:return run(name,args,fmt,expected=1 if delay or 'owner_error' in envelope else 0,profile=profile)
  finally:thread.join(timeout=8);assert not thread.is_alive()
base={'schema_version':1,'request_id':'work-engine-query','server_identity':'sha256:test','server_build':'sha256:test'}
observation={'operation':{'entry_state':'entry_uncertain'},'result':{'text':'safe\u001b]52;c;attack\u0007\u009b\r'},'completion':{},'workers':{'availability':'partial'},'usage':{},'process_closure':{'available':False}}
human=responder(dict(base,owner_result=observation),'--human','read_session_result',refs);assert 'Result: unknown' in human and 'Completion: unknown' in human and 'Workers: partial' in human and 'Usage: unknown' in human and '\\u{1b}' in human and '\\u{9b}' in human and '\\r' in human and '\x1b' not in human
# Human errors are escaped too.
error=responder(dict(base,owner_error={'code':'bad\u001b]52;c;attack\u0007'}),'--human','read_session_operation',refs);assert error==''; error_text=(T/f'error-{n}').read_text();assert '\\u{1b}' in error_text and '\\u{7}' in error_text and '\x1b' not in error_text
# Whole request deadline includes a peer that accepts and then stalls.
timeout=responder(dict(base,owner_result={}), '--json','read_session_result',refs,delay=2);assert timeout['transport_error']['code']=='transport_timeout'
print('PASS synthetic responder unknown/partial, terminal escaping, error path and total deadline')
