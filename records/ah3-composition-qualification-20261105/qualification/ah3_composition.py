"""SYNTHETIC disposable joined AH/browser/sync checkpoint; no deployed calls."""
import contextlib, http.server, json, os, pathlib, socket, hashlib, sqlite3, copy, subprocess, sys, threading, time
sys.path.insert(0,str(pathlib.Path(os.environ['AH3_WEB_SOURCE'])/'laptop'))
import ah3_queries as joined
import approval_sync as sync
root=pathlib.Path(os.environ['AH3_WEB_SOURCE'])
out=pathlib.Path(sys.argv[1]); out.mkdir(parents=True,exist_ok=True)
producer=sys.argv[2]; binary=sys.argv[3]
fixture=pathlib.Path(subprocess.check_output([producer],text=True).strip())
(out/'fixture-path.txt').write_text(str(fixture))
mail={'requests':[],'answers':[]}; signed=[]
class Mailbox(http.server.BaseHTTPRequestHandler):
 def log_message(self,*a):pass
 def respond(self,value):
  data=json.dumps(value).encode();self.send_response(200);self.send_header('Content-Type','application/json');self.send_header('Content-Length',str(len(data)));self.end_headers();self.wfile.write(data)
 def body(self):return json.loads(self.rfile.read(int(self.headers['Content-Length'])))
 def do_PUT(self):mail.update(self.body());self.respond({'count':len(mail['requests'])})
 def do_GET(self):
  try:
   if self.path=='/fixture/local':return self.respond(sync.ah_snapshot(consumer))
   if self.path=='/api/approval-keys':return self.respond({'keys':[{'credential_id':'key'}]})
   self.respond({'answers':mail['answers']} if self.path=='/api/answers' else mail)
  except sync.SyncError as e:
   self.send_error(503,str(e))
 def do_POST(self):
  b=self.body();mail['answers'].append(b);signed.append(b);self.respond({'stored':True,'decision':'unestablished','execution_admission':'untested'})
 def do_DELETE(self):mail['answers'].clear();self.respond({'deleted':True})
server=http.server.ThreadingHTTPServer(('127.0.0.1',0),Mailbox)
threading.Thread(target=server.serve_forever,daemon=True).start()
sock=socket.socket();sock.bind(('127.0.0.1',0));port=sock.getsockname()[1];sock.close()
config={'version':1,'listen':f'127.0.0.1:{port}','mcp':os.environ.get('AH3_MCP')=='1','credential_file':str(fixture/'token'),'task_profile':str(fixture/'task-profile.json'),'task_capability':'synthetic-owner-v1','max_connections':8,'max_queries':2,'request_timeout_seconds':30}
query_profile=joined.prepare(fixture);config['query_profile']=str(query_profile)
configpath=fixture/'server.json';configpath.write_text(json.dumps(config));os.chmod(configpath,0o600)
joined.capture(fixture,out,'pre-server')
proc=subprocess.Popen([binary,str(configpath)],stdin=subprocess.PIPE,stdout=subprocess.PIPE,stderr=(out/'ah.stderr').open('wb'))
try:
 for _ in range(200):
  try:
   with socket.create_connection(('127.0.0.1',port),timeout=.1):break
  except OSError:
   if proc.poll() is not None:raise RuntimeError('AH exited')
   time.sleep(.05)
 else:raise RuntimeError('AH startup timeout')
 sync.BASE_URL=f'http://127.0.0.1:{server.server_port}'
 sync.mailbox_token=lambda:'SYNTHETIC-only'
 sync.STATE_DIR=fixture/'sync-state'
 profile=json.loads((fixture/'task-profile.json').read_text())
 consumer={'owner_addresses':['role:owner'],'ah':{'url':f'http://127.0.0.1:{port}','credential_file':str(fixture/'token'),'task_ids':['fixture-task'],
  'server_identity':'sha256:'+hashlib.sha256(configpath.read_bytes()+query_profile.read_bytes()+(fixture/'task-profile.json').read_bytes()).hexdigest(),
  'server_build':'sha256:'+hashlib.sha256(pathlib.Path(binary).read_bytes()).hexdigest(),
  'source_sha256':profile['board']['script']['sha256'],'policy_sha256':profile['policy']['sha256']}}
 (fixture/'consumer.json').write_text(json.dumps(consumer))
 # Readiness is explicitly unestablished, never inferred healthy from startup.
 import urllib.request
 request=urllib.request.Request(consumer['ah']['url']+'/ready',headers={'Authorization':'Bearer '+(fixture/'token').read_text()})
 try:
  with urllib.request.urlopen(request) as response:ready=json.load(response)
 except urllib.error.HTTPError as response:ready=json.load(response)
 assert ready['task_owner'] is False and ready['task_owner_readiness']['state']=='unestablished',ready
 (out/'ready.json').write_text(json.dumps(ready,indent=2))
 for key,value in [('source_sha256','0'*64),('policy_sha256','0'*64),('task_ids',['missing-task'])]:
  unavailable=copy.deepcopy(consumer);unavailable['ah'][key]=value
  try:sync.push(unavailable);raise AssertionError('unsupported source/capability became healthy')
  except sync.SyncError:pass
 try:sync.ah_call(consumer,'unsupported_owner_capability',{});raise AssertionError('unsupported operation became healthy')
 except sync.SyncError:pass

 joined.query_suite(proc,fixture,config,out)
 assert sync.push(consumer)==1
 (out/'published-mailbox.json').write_text(json.dumps(mail,indent=2))
 subprocess.run(['node','tests/ah-browser-checkpoint.mjs',str(fixture),sync.BASE_URL,str(out/'browser.json')],cwd=root,check=True,timeout=90)
 assert len(signed)==1
 browser=json.loads((out/'browser.json').read_text())
 original=copy.deepcopy(mail['answers']);results={}
 if os.environ.get('AH3_PROTOCOL_ONLY'):
  joined.protocol_consume(proc,fixture,config,browser,out)
  print('PASS fresh real protocol first consume with exact browser-signed strings')
  sys.exit(0)
 routing=os.environ.get('A42_ROUTING_REGRESSION')
 if routing:
  # The actual browser-produced answer passes the unmodified legacy verifier:
  # same owner, pinned key, exact open board row and explicit synthetic time.
  policy=json.loads((fixture/'policy.json').read_text())
  row=json.loads(mail['requests'][0]['request_json'])
  real_verify=sync.oa.verify_answer
  now=sync.dt.datetime(2026,11,2,1,tzinfo=sync.dt.timezone.utc)
  assert real_verify(original[0]['answer_json'],original[0]['credential'],pinned_keys=policy['keys'],owner='owner',request_row=row,now=now)['decision']=='approve'
  registry=json.loads((sync.STATE_DIR/'ah-selections.json').read_text())
  assert len(registry)==1,'same-scope repeated publication must deduplicate'
  before_rows=json.loads(subprocess.check_output(['/usr/bin/python3','-I',profile['board']['script']['path'],'--repository',str(fixture/'repo'),'owner-boundary-io','--command-json','{"operation":"snapshot"}'],text=True))['rows']
  if routing=='first-scope':
   replacement=copy.deepcopy(consumer);replacement['owner_addresses']=['different-routing-scope']
   subprocess.run([producer,str(fixture),'replace'],check=True)
   assert sync.push(replacement)==1
   assert mail['requests'][0]['request_json']==sync.oa.request_json(row)
   assert mail['requests'][0]['request_digest']==json.loads(original[0]['answer_json'])['request_digest']
   assert len(json.loads((sync.STATE_DIR/'ah-selections.json').read_text()))==2
   real_call=sync.ah_call;attempts=[]
   def observe_scope(c,operation,args):attempts.append(operation);return real_call(c,operation,args)
   sync.ah_call=observe_scope
   receipts=sync.pull(replacement)
   assert attempts==[],('rebound first consume',attempts,receipts)
   assert receipts[0]['state']=='held_or_unknown'
   assert mail['answers']==original
   sync.ah_call=real_call
   details={'regression':routing,'real_legacy_verification_passed':True,'attempts':attempts,'retained':True,'distinct_publications':2,'receipts':receipts}
  else:
   # Actual AH outage retains an uncertain input while the board request is open.
   proc.terminate();proc.wait(timeout=10)
   assert sync.pull(consumer)[0]['state']=='held_or_unknown'
   assert mail['answers']==original
   (fixture/'repo/skills').mkdir()
   (fixture/'repo/skills/durable-state').symlink_to(pathlib.Path(profile['board']['script']['path']).parent.parent,target_is_directory=True)
   sync.OWNER_USER_ID='owner';sync.pinned_keys=lambda:policy['keys']
   sync.oa.verify_answer=lambda *a,**kw:real_verify(*a,**kw,now=now)
   legacy={'board_repository':fixture/'repo','owner_addresses':['role:owner']}
   real_post=sync.post_reply;posts=[]
   def no_post(*args,**kwargs):posts.append(True);raise AssertionError('AH answer reached legacy post')
   sync.post_reply=no_post
   for mode in ['omitted','null']:
    disabled=copy.deepcopy(legacy)
    if mode=='null':disabled['ah']=None
    sync.pull(disabled)
    assert mail['answers']==original
   assert not posts
   # Unchanged legacy control: same cryptographically valid answer, fresh
   # unrelated consumer state with no AH association, real board post/delete.
   sync.STATE_DIR=fixture/'legacy-control-state';sync.post_reply=real_post
   sync.pull(legacy)
   assert not mail['answers']
   details={'regression':routing,'real_legacy_verification_passed':True,'AH_omitted_and_null_retained':True,'legacy_posts_for_AH_answer':0,'unassociated_legacy_control_posted_and_deleted':True}
  after_rows=json.loads(subprocess.check_output(['/usr/bin/python3','-I',profile['board']['script']['path'],'--repository',str(fixture/'repo'),'owner-boundary-io','--command-json','{"operation":"snapshot"}'],text=True))['rows']
  assert len(after_rows)==len(before_rows)+(0 if routing=='first-scope' else 1)
  details.update(board_rows_before=len(before_rows),board_rows_after=len(after_rows))
  (out/'routing-regression.json').write_text(json.dumps(details,indent=2))
  (out/'config.json').write_text(json.dumps({'server':config,'consumer':consumer,'fixture':str(fixture),'clock':'SYNTHETIC2026-11-02T01:00:00Z'},indent=2))
  print('PASS actual composed routing regression '+routing)
  sys.exit(0)

 for name,vector in browser['vectors'].items():
  mail['answers']=[vector]
  receipts=sync.pull(consumer)
  assert receipts[0]['state']=='held_or_unknown',(name,receipts)
  assert len(mail['answers'])==1
  results[name]=receipts
 mail['answers']=original
 if os.environ.get('A42_OWNER_MUTATION'):
  mode=os.environ['A42_OWNER_MUTATION']
  subprocess.run([producer,str(fixture),mode],check=True)
  if mode=='replace':
   assert sync.push(consumer)==1
  else:
   try:sync.push(consumer);raise AssertionError('withdrawn task became healthy')
   except sync.SyncError:pass
  receipts=sync.pull(consumer)
  assert receipts[0]['state']=='held_or_unknown',receipts
  assert mail['answers']==original
  (out/'mutation.json').write_text(json.dumps({'mode':mode,'retained':True,'receipts':receipts}))
  print('PASS actual owner mutation held exact prior selection: '+mode)
  sys.exit(0)
 # Exercise a lost consume acknowledgement against the ACTUAL AH. The call
 # executes normally, then this consumer drops its transport acknowledgement.
 real_call=sync.ah_call;calls=[]
 def lose_ack(c,operation,args):
  calls.append(operation);result=real_call(c,operation,args)
  if operation=='consume_owner_answer':raise sync.SyncError('fixture lost ACK after actual owner commit')
  return result
 sync.ah_call=lose_ack
 receipts=sync.pull(consumer);assert receipts[0]['state']=='held_or_unknown'
 sync.ah_call=real_call
 assert calls==['consume_owner_answer']
 # Restart actual AH and a fresh Python sync process before read reconciliation.
 proc.terminate();proc.wait(timeout=10)
 joined.capture(fixture,out,'pre-restart')
 proc=subprocess.Popen([binary,str(configpath)],stdin=subprocess.PIPE,stdout=subprocess.PIPE,stderr=(out/'ah-restart.stderr').open('wb'))
 for _ in range(200):
  try:
   with socket.create_connection(('127.0.0.1',port),timeout=.1):break
  except OSError:
   if proc.poll() is not None:raise RuntimeError('AH restart exited')
   time.sleep(.05)
 else:raise RuntimeError('AH restart timeout')
 def counts():
  db=sqlite3.connect(fixture/'registry.sqlite')
  effects={t:db.execute('SELECT count(*) FROM '+t).fetchone()[0] for t in ['admission','outbox','capacity_claim','scoped_claim','pi_claim']};db.close()
  snapshot=subprocess.check_output(['/usr/bin/python3','-I',profile['board']['script']['path'],'--repository',str(fixture/'repo'),'owner-boundary-io','--command-json','{"operation":"snapshot"}'],text=True)
  return {'rows':len(json.loads(snapshot)['rows']),'effects':effects}
 before=counts()
 command="import sys,json,pathlib;sys.path.insert(0,sys.argv[1]);import approval_sync as s;s.BASE_URL=sys.argv[2];s.mailbox_token=lambda:'SYNTHETIC';s.STATE_DIR=pathlib.Path(sys.argv[3]);s.pull(json.loads(pathlib.Path(sys.argv[4]).read_text()))"
 subprocess.run([sys.executable,'-c',command,str(root/'laptop'),sync.BASE_URL,str(sync.STATE_DIR),str(fixture/'consumer.json')],check=True)
 assert counts()==before
 journal=json.loads((sync.STATE_DIR/'ah-deliveries.json').read_text());record=journal[sync.ah_digest({'answer_json':original[0]['answer_json'],'credential_id':original[0]['credential']['id'],'authenticator_data':original[0]['credential']['response']['authenticatorData'],'client_data_json':original[0]['credential']['response']['clientDataJSON'],'signature':original[0]['credential']['response']['signature']})]
 assert record['state']=='owner_verified_decision'
 assert record['decision']['signed']==record['input']['signed']
 assert record['decision']['current_standing']=='held'
 joined.signed_read(proc,config,record,out)
 assert len(mail['answers'])==1
 # Repeat retains the same owner decision/effect, with no consume retry.
 calls=[]
 def observe(c,operation,args):calls.append(operation);return real_call(c,operation,args)
 sync.ah_call=observe
 assert sync.pull(consumer)[0]['answer_retained'];assert calls==['read_owner_answer'];assert counts()==before
 sync.ah_call=real_call
 # Changed startup source/config cannot silently select a second consume path.
 changed=copy.deepcopy(consumer);changed['ah']['server_build']='sha256:'+'0'*64
 assert sync.pull(changed)[0]['state']=='held_or_unknown';assert counts()==before
 changed=copy.deepcopy(consumer);changed['owner_addresses']=['different-routing-scope']
 assert sync.pull(changed)[0]['state']=='held_or_unknown';assert counts()==before
 # Owner outage retains signed bytes; it never becomes empty/healthy.
 proc.terminate();proc.wait(timeout=10)
 assert sync.pull(consumer)[0]['state']=='held_or_unknown';assert mail['answers']==original
 try:sync.push(consumer);raise AssertionError('outage became healthy')
 except sync.SyncError:pass
 (out/'decision.json').write_text(json.dumps(record,indent=2))
 (out/'negative-results.json').write_text(json.dumps({'negative_vectors':results,'lost_ack_calls':['consume_owner_answer'],'reconcile_calls':calls,'before_after':before,'retained_held_answer':True,'outage_retained':True},indent=2))
 (out/'config.json').write_text(json.dumps({'server':config,'consumer':consumer,'clock':'SYNTHETIC2026-11-02T01:00:00Z','fixture':str(fixture),'binary':binary},indent=2))
 print('PASS actual AH composed queryCLI/publicRust/MCPoptional rendered consumer views, signed negatives, lost ACK/restart/read-only reconciliation, held retention and outage; execution untested')
finally:
 proc.terminate()
 try:proc.wait(timeout=10)
 except subprocess.TimeoutExpired:proc.kill();proc.wait()
 server.shutdown();server.server_close()
