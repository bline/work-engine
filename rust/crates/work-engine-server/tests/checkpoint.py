"""Exercise both adapters against the pinned real query owner; no native bridge."""
import contextlib, hashlib, importlib.util, json, os, pathlib, shutil, signal, socket, subprocess, sys, tempfile, time
root=pathlib.Path(__file__).resolve().parents[4]
exe=pathlib.Path(os.environ['SERVER_BINARY'])
sid='11111111-1111-4111-8111-111111111111'
secret='webridge_'+'A'*43
with contextlib.nullcontext(os.environ.get('FIXTURE_PATH') or tempfile.mkdtemp(prefix='preserved-fixture-', dir=os.environ['FIXTURE_ROOT'])) as tmp:
 t=pathlib.Path(tmp); reuse=os.environ.get('FIXTURE_REUSE')=='1'
 c=t/'codex'; cl=t/'claude'; state=t/'state'; path=c/('sessions/2026/01/01/rollout-fixture-'+sid+'.jsonl')
 source=root/'skills/session-events/scripts/session_events.py'; owner=t/'owner.py'; profile=t/'profile.json'; cred=t/'credential'; config=t/'server.json'; proof=t/'proof'
 if not reuse:
  t.mkdir(mode=0o700,exist_ok=True); t.chmod(0o700); cl.mkdir(); state.mkdir(); path.parent.mkdir(parents=True)
  records=[{'type':'session_meta','timestamp':'2026-01-01T00:00:00Z','payload':{'cwd':str(t),'timestamp':'2026-01-01T00:00:00Z'}},{'type':'response_item','timestamp':'2026-01-01T00:00:01Z','payload':{'type':'message','role':'user','content':[{'type':'input_text','text':'hello fixture '+secret}]}}]
  path.write_text(''.join(json.dumps(r)+'\n' for r in records)); (c/'session_index.jsonl').write_text(json.dumps({'id':sid,'thread_name':'fixture'})+'\n')
  owner.write_bytes(source.read_bytes())
  profile.write_text(json.dumps({'version':1,'owner':str(owner),'owner_sha256':hashlib.sha256(owner.read_bytes()).hexdigest(),'codex_home':str(c),'claude_home':str(cl),'state_dir':str(state),'bridge':None,'context':None})); profile.chmod(0o600)
  cred.write_text('A'*40); cred.chmod(0o600)
  with socket.socket() as sock: sock.bind(('127.0.0.1',0)); port=sock.getsockname()[1]
  config.write_text(json.dumps({'version':1,'listen':f'127.0.0.1:{port}','mcp':True,'query_profile':str(profile),'credential_file':str(cred),'max_connections':4,'max_queries':2,'request_timeout_seconds':40})); config.chmod(0o600)
  proof.mkdir(mode=0o700)
  for name,item in [('config.json',config),('profile.json',profile),('credential',cred),('owner.py',owner),('transcript.jsonl',path),('index.jsonl',c/'session_index.jsonl')]: shutil.copyfile(item,proof/name); (proof/name).chmod(0o600)
 else:
  port=int(json.loads(config.read_text())['listen'].rsplit(':',1)[1])
 print('PRESERVED_FIXTURE='+str(t),flush=True)
 if os.environ.get('FIXTURE_PREPARE_ONLY')=='1': sys.exit(0)
 sys.dont_write_bytecode=True
 spec=importlib.util.spec_from_file_location('session_events_fixture', owner); module=importlib.util.module_from_spec(spec); sys.modules[spec.name]=module; spec.loader.exec_module(module)
 direct=module.Server(module.Homes(c,cl,state),'local-query',module.CallbackPolicy())
 proc=subprocess.Popen([str(exe),str(config)],stdin=subprocess.PIPE,stdout=subprocess.PIPE,stderr=subprocess.PIPE,text=True)
 try:
  for _ in range(300):
   try:
    sock=socket.create_connection(('127.0.0.1',port),timeout=0.1); sock.close(); break
   except OSError: time.sleep(.05)
  else: raise AssertionError('server unavailable')
  def rpc(v):
   proc.stdin.write(json.dumps(v)+'\n'); proc.stdin.flush(); return json.loads(proc.stdout.readline())
  invalid=rpc({'jsonrpc':'2.0','id':0,'method':'initialize','params':{'protocolVersion':'1900-01-01','capabilities':{},'clientInfo':{'name':'fixture','version':'1'}}}); assert invalid['error']['code']==-32602
  init=rpc({'jsonrpc':'2.0','id':1,'method':'initialize','params':{'protocolVersion':'2025-06-18','capabilities':{},'clientInfo':{'name':'fixture','version':'1'}}}); assert init['result']['protocolVersion']=='2025-06-18'
  notification={'jsonrpc':'2.0','method':'notifications/initialized'}; proc.stdin.write(json.dumps(notification)+'\n'); proc.stdin.flush()
  discovery_request={'jsonrpc':'2.0','id':11,'method':'tools/list','params':{}}; discovery=rpc(discovery_request)
  tools=discovery['result']['tools']; assert len(tools)==2 and {v['name'] for v in tools}=={'list_sessions','read_session'}
  for name,value in [('initialized-notification',notification),('discovery-request',discovery_request),('discovery-response',discovery)]:
   (proof/(name+'.json')).write_text(json.dumps(value,indent=2)); (proof/(name+'.json')).chmod(0o600)
  print('MCP initialized notification and exact two-tool discovery passed',flush=True)
  for op,args in [('list_sessions',{}),('read_session',{'session_id':sid})]:
   rpc_request={'jsonrpc':'2.0','id':2,'method':'tools/call','params':{'name':op,'arguments':args}}
   rpc_response=rpc(rpc_request); m=rpc_response['result']['structuredContent']
   body=json.dumps({'schema_version':1,'operation':op,'arguments':args,'request_id':'mcp'}).encode()
   with socket.create_connection(('127.0.0.1',port),timeout=45) as s:
    header=f'POST /v1/session-queries HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nAuthorization: Bearer '+ 'A'*40 +f'\r\nContent-Type: application/json\r\nContent-Length: {len(body)}\r\n\r\n'
    s.sendall(header.encode()+body); chunks=[]
    while True:
     b=s.recv(65536)
     if not b: break
     chunks.append(b)
   wire=b''.join(chunks); assert wire.startswith(b'HTTP/1.1 200 '),wire
   h=json.loads(wire.split(b'\r\n\r\n',1)[1]); assert m==h,(m,h)
   expected=direct.call_tool({'name':op,'arguments':args})['structuredContent']; assert h['owner_result']==expected,(h,expected)
   for suffix,value in [('mcp-request',rpc_request),('mcp-response',rpc_response),('http-request',json.loads(body)),('http-response',h),('direct-owner-result',expected)]:
    (proof/(op+'-'+suffix+'.json')).write_text(json.dumps(value,indent=2)); (proof/(op+'-'+suffix+'.json')).chmod(0o600)
   assert 'owner_result' in h and (h['owner_result']['matched']==1 if op=='list_sessions' else h['owner_result']['entries'][0]['text']=='hello fixture [REDACTED:bridge_credential]'),h
   assert secret not in json.dumps(h)
   print(op,'actual owner-backed MCP/HTTP/direct parity and redaction',flush=True)
  client=json.loads(subprocess.check_output([os.environ['CLIENT_BINARY'],f'127.0.0.1:{port}',str(cred),'{}'],text=True,timeout=45)); assert client['owner_result']['matched']==1
  print('Rust local client actual owner query passed',flush=True)
  def http(method, target, body=b'', token='A'*40, host=None, extra=''):
   hdr=f'{method} {target} HTTP/1.1\r\nHost: {host or f"127.0.0.1:{port}"}\r\nAuthorization: Bearer {token}\r\n{extra}'
   if method=='POST': hdr+=f'Content-Type: application/json\r\nContent-Length: {len(body)}\r\n'
   with socket.create_connection(('127.0.0.1',port),timeout=45) as sock:
    sock.sendall((hdr+'\r\n').encode()+body); chunks=[]
    while True:
     b=sock.recv(65536)
     if not b: break
     chunks.append(b)
   wire=b''.join(chunks); return int(wire.split(b' ',2)[1]),json.loads(wire.split(b'\r\n\r\n',1)[1])
  assert http('GET','/live')[1]['live']
  assert http('GET','/ready')[1]['list_sessions']
  assert http('GET','/ready',token='wrong')[0]==401
  wrong_cred=t/'wrong-credential'; wrong_cred.write_text('B'*40); wrong_cred.chmod(0o600)
  failed_client=subprocess.run([os.environ['CLIENT_BINARY'],f'127.0.0.1:{port}',str(wrong_cred),'{}'],text=True,capture_output=True,timeout=45)
  assert failed_client.returncode!=0 and json.loads(failed_client.stdout)=={'transport_error':'transport_rejected'}
  (proof/'client-transport-error.json').write_text(failed_client.stdout)

  assert http('GET','/live',host='localhost:'+str(port))[0]==400
  assert http('GET','/live',extra='Origin: http://localhost\r\n')[0]==400
  assert http('POST','/v1/session-queries',b'{')[0]==400
  assert http('POST','/v1/session-queries',json.dumps({'schema_version':1,'operation':'list_sessions','arguments':{},'request_id':'x','profile':'/tmp/untrusted'}).encode())[0]==400
  assert http('POST','/v1/session-queries',b'{"schema_version":1,"operation":"list_sessions","arguments":{"limit":1,"limit":2},"request_id":"x"}')[0]==400
  def query(op, args, **kw):
   return http('POST','/v1/session-queries',json.dumps({'schema_version':1,'operation':op,'arguments':args,'request_id':'neg',**kw}).encode())[1]
  assert query('submit_session_request',{})['error']['code']=='invalid_request'
  assert query('list_sessions',{'mutation':True})['error']['code']=='invalid_arguments'
  assert http('POST','/v1/session-queries',json.dumps({'schema_version':2,'operation':'list_sessions','arguments':{},'request_id':'neg'}).encode())[1]['error']['code']=='invalid_request'
  with socket.create_connection(('127.0.0.1',port),timeout=5) as sock:
   sock.sendall((f'POST /v1/session-queries HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nAuthorization: Bearer '+ 'A'*40 +'\r\nContent-Type: application/json\r\nContent-Length: 262145\r\n\r\n').encode()); assert sock.recv(4096).startswith(b'HTTP/1.1 400 ')
  def owner_error_parity(expected):
   direct_api=json.loads(subprocess.check_output([os.environ['OWNER_PROBE_BINARY'],str(profile)],text=True,timeout=45))
   mcp_result=rpc({'jsonrpc':'2.0','id':21,'method':'tools/call','params':{'name':'list_sessions','arguments':{}}})['result']
   http_result=query('list_sessions',{})
   client_result=json.loads(subprocess.check_output([os.environ['CLIENT_BINARY'],f'127.0.0.1:{port}',str(cred),'{}'],text=True,timeout=45))
   assert direct_api=={'owner_error':{'code':expected}}
   assert mcp_result['isError'] and mcp_result['structuredContent']['owner_error']==direct_api['owner_error']
   assert http_result['owner_error']==direct_api['owner_error']==client_result['owner_error']
   assert http('GET','/ready',token='wrong')[0]==401
   for name,value in [('query-api',direct_api),('mcp',mcp_result),('http',http_result),('client',client_result)]:
    path=proof/(expected+'-'+name+'.json'); path.write_text(json.dumps(value,indent=2)); path.chmod(0o600)
   print(expected,'query API/MCP/HTTP/client owner-error parity; transport 401 distinct',flush=True)
  owner.rename(t/'unavailable-owner.py')
  assert http('GET','/live')[1]['live'] and not http('GET','/ready')[1]['list_sessions']
  owner_error_parity('owner_unavailable')
  (t/'unavailable-owner.py').rename(owner)
  owner.write_text('modified')
  assert not http('GET','/ready')[1]['list_sessions']
  owner_error_parity('owner_pin_mismatch')
  owner.write_bytes(source.read_bytes())
  assert rpc({'jsonrpc':'2.0','id':3,'method':'does_not_exist'})['error']['code']==-32601
  assert rpc({'jsonrpc':'2.0','id':33,'method':'tools/list','params':{'profile':'/tmp/untrusted'}})['error']['code']==-32602
  assert rpc({'jsonrpc':'2.0','id':4,'method':'tools/call','params':{'name':'submit_session_request','arguments':{}}})['result']['isError']
  print('HTTP trust, schema, readiness and MCP refusal checks passed',flush=True)
  proc.stdin.write(json.dumps({'jsonrpc':'2.0','id':5,'method':'tools/call','params':{'name':'list_sessions','arguments':{}}})+'\n'); proc.stdin.flush()
  observed=False
  for _ in range(2000):
   for childfile in pathlib.Path(f'/proc/{proc.pid}/task').glob('*/children'):
    try:
     if childfile.read_text().strip(): observed=True; break
    except FileNotFoundError: pass
   if observed: break
   time.sleep(.001)
  assert observed,'MCP owner helper was not observed in flight'
  proc.send_signal(signal.SIGTERM)
  stopped=False
  for _ in range(500):
   try:
    s=socket.create_connection(('127.0.0.1',port),timeout=.02); s.close()
   except OSError: stopped=True; break
   time.sleep(.001)
  assert stopped,'listener did not stop admission'
  result=json.loads(proc.stdout.readline()); assert result['result']['structuredContent']['owner_result']['matched']==1
  proc.wait(timeout=45); assert proc.returncode==0 and 'drain complete' in proc.stderr.read()
  print('SIGTERM during MCP owner helper drained accepted query',flush=True)


 finally:
  if proc.poll() is None: proc.send_signal(signal.SIGTERM)
  try: proc.wait(timeout=50)
  except subprocess.TimeoutExpired: proc.kill(); proc.wait(); raise
  assert proc.returncode==0,(proc.returncode,proc.stderr.read())
 # Declared MCP EOF shutdown, first idle and then with an accepted owner query in flight.
 for inflight in (False,True):
  eof_proc=subprocess.Popen([str(exe),str(config)],stdin=subprocess.PIPE,stdout=subprocess.PIPE,stderr=subprocess.PIPE,text=True)
  try:
   for _ in range(300):
    try: conn=socket.create_connection(('127.0.0.1',port),timeout=.1); conn.close(); break
    except OSError: time.sleep(.05)
   else:
    status=eof_proc.poll()
    detail=eof_proc.stderr.read() if status is not None else 'process alive; startup listener deadline exceeded'
    raise AssertionError(f'EOF server unavailable: status={status}; {detail}')
   query_sock=None
   if inflight:
    body=json.dumps({'schema_version':1,'operation':'list_sessions','arguments':{},'request_id':'eof-drain'}).encode()
    query_sock=socket.create_connection(('127.0.0.1',port),timeout=45)
    header=f'POST /v1/session-queries HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nAuthorization: Bearer '+ 'A'*40 +f'\r\nContent-Type: application/json\r\nContent-Length: {len(body)}\r\n\r\n'
    query_sock.sendall(header.encode()+body); observed=False
    for _ in range(2000):
     for childfile in pathlib.Path(f'/proc/{eof_proc.pid}/task').glob('*/children'):
      try:
       if childfile.read_text().strip(): observed=True; break
      except FileNotFoundError: pass
     if observed: break
     time.sleep(.001)
    assert observed,'EOF concurrent query helper not observed'
   eof_proc.stdin.close()
   closed=False
   for _ in range(500):
    try: conn=socket.create_connection(('127.0.0.1',port),timeout=.02); conn.close()
    except OSError: closed=True; break
    time.sleep(.001)
   assert closed,'EOF listener stayed open'
   if query_sock:
    chunks=[]
    while True:
     chunk=query_sock.recv(65536)
     if not chunk: break
     chunks.append(chunk)
    query_sock.close(); wire=b''.join(chunks); assert wire.startswith(b'HTTP/1.1 200 '),wire
    response=json.loads(wire.split(b'\r\n\r\n',1)[1]); assert response['owner_result']['matched']==1
    (proof/'eof-concurrent-http-request.json').write_bytes(body); (proof/'eof-concurrent-http-response.json').write_text(json.dumps(response,indent=2))
   eof_proc.wait(timeout=45); err=eof_proc.stderr.read(); assert eof_proc.returncode==0 and 'drain complete' in err and 'panicked' not in err,(eof_proc.returncode,err)
   (proof/('eof-concurrent-stderr.txt' if inflight else 'eof-idle-stderr.txt')).write_text(err)
   print('MCP EOF', 'concurrent HTTP owner query' if inflight else 'idle', 'listener closed and clean drain',flush=True)
  finally:
   if eof_proc.poll() is None: eof_proc.kill(); eof_proc.wait()
 # Same private configuration cannot start twice; HTTP stays available with MCP disabled.
 config_data=json.loads(config.read_text()); config_data['mcp']=False; config_data['request_timeout_seconds']=5; config.write_text(json.dumps(config_data))
 for iteration in range(2):
  proc=subprocess.Popen([str(exe),str(config)],stdin=subprocess.DEVNULL,stdout=subprocess.PIPE,stderr=subprocess.PIPE,text=True)
  try:
   for _ in range(300):
    try:
     sock=socket.create_connection(('127.0.0.1',port),timeout=.1); sock.close(); break
    except OSError: time.sleep(.05)
   else: raise AssertionError('disabled MCP server unavailable')
   duplicate=subprocess.run([str(exe),str(config)],stdin=subprocess.DEVNULL,capture_output=True,text=True,timeout=5)
   assert duplicate.returncode!=0
   other=t/'other-config.json'; other.write_text(config.read_text()); other.chmod(0o600)
   duplicate=subprocess.run([str(exe),str(other)],stdin=subprocess.DEVNULL,capture_output=True,text=True,timeout=5); assert duplicate.returncode!=0
   client=json.loads(subprocess.check_output([os.environ['CLIENT_BINARY'],f'127.0.0.1:{port}',str(cred),'{}'],text=True,timeout=45)); assert client['owner_result']['matched']==1
   with socket.create_connection(('127.0.0.1',port),timeout=10) as slow:
    slow.sendall(b'GET /live HTTP/1.1\r\n'); assert slow.recv(4096).startswith(b'HTTP/1.1 408 ')
   if iteration==0:
    body=json.dumps({'schema_version':1,'operation':'list_sessions','arguments':{},'request_id':'drain'}).encode()
    sock=socket.create_connection(('127.0.0.1',port),timeout=45)
    header=f'POST /v1/session-queries HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nAuthorization: Bearer '+ 'A'*40 +f'\r\nContent-Type: application/json\r\nContent-Length: {len(body)}\r\n\r\n'
    sock.sendall(header.encode()+body)
    observed=False
    for _ in range(2000):
     for childfile in pathlib.Path(f'/proc/{proc.pid}/task').glob('*/children'):
      try:
       if childfile.read_text().strip(): observed=True; break
      except FileNotFoundError: pass
     if observed: break
     time.sleep(.001)
    assert observed,'owner helper was not observed in flight'
    proc.send_signal(signal.SIGTERM)
    chunks=[]
    while True:
     data=sock.recv(65536)
     if not data: break
     chunks.append(data)
    sock.close(); result=json.loads(b''.join(chunks).split(b'\r\n\r\n',1)[1]); assert result['owner_result']['matched']==1
    proc.wait(timeout=45); assert proc.returncode==0 and 'drain complete' in proc.stderr.read()
    print('SIGTERM during observed actual owner helper drained accepted query',flush=True)

  finally:
   if proc.poll() is None: proc.send_signal(signal.SIGTERM); proc.wait(timeout=45)
   assert proc.returncode==0
 print('MCP-disabled client, timeout, single-config lock and restart passed',flush=True)
 # Untrusted file mode and bridge profile fail before listening.
 config.chmod(0o644); bad=subprocess.run([str(exe),str(config)],capture_output=True,text=True,timeout=5); assert bad.returncode!=0; config.chmod(0o600)
 data=json.loads(profile.read_text()); data['bridge']={'binary':'/tmp/b','binary_sha256':'0'*64,'config':'/tmp/c','config_sha256':'0'*64}; profile.write_text(json.dumps(data))
 bad=subprocess.run([str(exe),str(config)],capture_output=True,text=True,timeout=5); assert bad.returncode!=0
 print('invalid trust configuration rejected at startup',flush=True)
 # Restore exact original inputs at their original paths after negative checks.
 shutil.copyfile(proof/'config.json',config); shutil.copyfile(proof/'profile.json',profile); shutil.copyfile(proof/'owner.py',owner); shutil.copyfile(proof/'credential',cred)
 for f in (config,profile,owner,cred): f.chmod(0o600)
