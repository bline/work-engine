"""Adapter validation and presentation; synthetic transport has no domain authority."""
import json,os,pathlib,socket,subprocess,threading
os.umask(0o077)
t=pathlib.Path(os.environ['FIXTURE_DIR']);t.mkdir(mode=0o700);exe=os.environ['COMMAND_BINARY']
cred=t/'transport';cred.write_text('T'*40);cred.chmod(0o600)
sock=socket.socket();sock.bind(('127.0.0.1',0));sock.listen();sock.settimeout(.2);port=sock.getsockname()[1]
profile={'version':1,'kind':'local-service','endpoint':f'127.0.0.1:{port}','credential_file':str(cred),'timeout_seconds':2};p=t/'profile';p.write_text(json.dumps(profile));p.chmod(0o600)
a={'request_id':'11111111-1111-4111-8111-111111111111','grant_credential':'private-domain-grant','session_id':'any\nidentity','workspace':'/tmp/a','input_text':'hello\nΩ'};f=t/'input';f.write_text(json.dumps(a));f.chmod(0o600)
def run(argv,stdin=None): return subprocess.run([exe,*map(str,argv)],input=stdin,text=True,capture_output=True,timeout=8)
def no_connection():
 try: conn,_=sock.accept();conn.close();raise AssertionError('unexpected connection')
 except socket.timeout: pass
for i,raw in enumerate(['{}','[]','{"request_id":"x"}',json.dumps(dict(a,extra=1)),json.dumps(a)[:-1]+',"request_id":"11111111-1111-4111-8111-111111111111"}',json.dumps(dict(a,request_id='bad')),'{']):
 bad=t/f'bad-{i}';bad.write_text(raw);bad.chmod(0o600);r=run([p,'submit_session_request','--json',bad]);assert r.returncode and json.loads(r.stdout)['error']['code']=='invalid_arguments';no_connection()
for i,raw in enumerate([json.dumps(dict(profile,kind='direct')),json.dumps(dict(profile,extra=1)),json.dumps(profile)[:-1]+',"version":1}',json.dumps(dict(profile,endpoint='0.0.0.0:1')),'{']):
 bad=t/f'bad-profile-{i}';bad.write_text(raw);bad.chmod(0o600);r=run([bad,'submit_session_request','--json',f]);assert r.returncode and json.loads(r.stdout)['error']['code']=='invalid_profile';no_connection()
for argv in ([p,'wrong','--json',f],[p,'submit_session_request','--json'],[p,'submit_session_request','--human',f,'extra']):
 r=run(argv);assert r.returncode;no_connection()
f.chmod(0o644);assert run([p,'submit_session_request','--json',f]).returncode;no_connection();f.chmod(0o600)
observed=[];envelope={'schema_version':1,'request_id':'work-engine-command','server_identity':'sha256:test','server_build':'sha256:test','owner_result':{'operation_id':'id\x1b[2J','entry_state':'entry_uncertain\x9b'}}
def respond():
 conn,_=sock.accept();data=b''
 while b'\r\n\r\n' not in data:data+=conn.recv(4096)
 head,body=data.split(b'\r\n\r\n',1);size=int(next(x.split(b':',1)[1] for x in head.split(b'\r\n') if x.lower().startswith(b'content-length:')))
 while len(body)<size:body+=conn.recv(4096)
 observed.append((head,json.loads(body)));payload=json.dumps(envelope).encode();conn.sendall(b'HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: '+str(len(payload)).encode()+b'\r\n\r\n'+payload);conn.close()
for fmt in ('--json','--human'):
 thread=threading.Thread(target=respond);thread.start();r=run([p,'submit_session_request',fmt,'-'],json.dumps(a));thread.join();assert r.returncode==0
 if fmt=='--json': assert json.loads(r.stdout)['envelope']==envelope
 else: assert '\x1b' not in r.stdout and '\x9b' not in r.stdout and '\\u{1b}' in r.stdout and '\\u{9b}' in r.stdout and 'not established' in r.stdout
# Presentation-only envelopes: preserve the full machine object and distinguish native outcomes.
base={'schema_version':1,'request_id':'work-engine-command','server_identity':'sha256:test','server_build':'sha256:test'}
def present(payload, fmt):
 global envelope
 envelope=dict(base,**payload)
 thread=threading.Thread(target=respond);thread.start();r=run([p,'submit_session_request',fmt,'-'],json.dumps(a));thread.join();return r
for code in ('timeout','owner_unavailable','output_too_large','custody_lost','identity_conflict','query_rejected','future_unknown'):
 payload={'owner_error':{'code':code}}
 h=present(payload,'--human');assert h.returncode and code in h.stderr and 'Entry, cancellation and settlement are unknown' in h.stderr and 'original request' in h.stderr
 j=present(payload,'--json');assert j.returncode and json.loads(j.stdout)['envelope']==dict(base,**payload)
h=present({'owner_error':{'code':'unauthorized'}},'--human');assert h.returncode and 'unauthorized' in h.stderr and 'settlement are unknown' not in h.stderr
for status in ('completed','failed','interrupted'):
 owner={'operation_id':'op\x1b','entry_state':'terminal','native_status':status,'native_turn_id':'turn\x9b','last_reconciliation_at':123,'limitations':['partial\x07data'],'completion':{'available':False},'result':{'availability':'partial'},'process_closure':{'available':False}}
 payload={'owner_result':owner};h=present(payload,'--human');assert h.returncode==0
 for text in ('Entry: terminal','Native status: '+status,'Native turn: turn\\u{9b}','Last reconciliation: 123','Completion: unavailable','Result: partial','Process closure: unavailable','partial\\u{7}data','Settlement: not established'):
  assert text in h.stdout,(text,h.stdout)
 assert all(c not in h.stdout for c in ('\x1b','\x9b','\x07'))
 j=present(payload,'--json');assert json.loads(j.stdout)['envelope']==dict(base,**payload)
payload={'owner_result':{'operation_id':'op','entry_state':'entry_uncertain','limitations':None}};h=present(payload,'--human');assert all(x in h.stdout for x in ('Entry: entry_uncertain','Native status: unknown','Native turn: unknown','Last reconciliation: unknown','Completion: unknown','Result: unknown','Process closure: unknown','Limitations: unknown'))
for head,body in observed:
 assert b'Bearer '+b'T'*40 in head and b'private-domain-grant' not in head
 assert body['arguments']==a and body['request_id']=='work-engine-command' and body['operation']=='submit_session_request'
sock.close();(t/'adapter-observation.json').write_text(json.dumps({'invalid_cases':16,'envelope_preserved':True,'stdin_exact':True,'credential_separation':True,'human_controls_escaped':True}));print('PASS adapter validation and presentation')
