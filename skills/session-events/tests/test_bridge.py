"""Offline bridge integration: real HTTP/helper/SQLite against a disposable fake native daemon."""
from __future__ import annotations
import contextlib, base64, hashlib, http.client, http.server, json, os, shutil, socket, sqlite3, struct, subprocess, tempfile, threading, time, unittest, uuid
from pathlib import Path
from test_session_events import SE, CODEX_ID, codex_line, Receiver, whsec, URL

BINARY = os.environ.get("SESSION_BRIDGE_TEST_BINARY")
ROOT = Path(os.environ.get("SESSION_BRIDGE_TEST_ROOT", "/home/bline/.local/state/work-engine"))

class FakeNative:
    def __init__(self, path, workspace):
        self.path, self.workspace, self.turns, self.starts = path, str(workspace), [], 0
        self.lost_ack = False
        self.ack_override = None
        self.native_error = None
        self.loaded = True
        self.status = 'idle'
        self.turn_status = 'completed'
        self.repeat_cursor = False
        self.block_method = None
        self.blocked = threading.Event()
        self.release = threading.Event()
        self.sock = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
        self.sock.bind(str(path)); self.sock.listen(); self.sock.settimeout(.2)
        self.stop = False
        self.thread = threading.Thread(target=self.run, daemon=True); self.thread.start()
    def close(self):
        self.stop = True; self.release.set(); self.thread.join(6); self.sock.close()
    @staticmethod
    def exact(c,n):
        out=b""
        while len(out)<n:
            x=c.recv(n-len(out))
            if not x: raise EOFError()
            out+=x
        return out
    def frame(self,c):
        b=self.exact(c,2); n=b[1]&127
        if n==126: n=struct.unpack('!H',self.exact(c,2))[0]
        if n==127: n=struct.unpack('!Q',self.exact(c,8))[0]
        mask=self.exact(c,4) if b[1]&128 else b''
        data=self.exact(c,n)
        return bytes(v^mask[i%4] for i,v in enumerate(data)) if mask else data
    def send(self,c,obj):
        data=json.dumps(obj).encode(); n=len(data)
        header=bytes([0x81,n]) if n<126 else bytes([0x81,126])+struct.pack('!H',n) if n<65536 else bytes([0x81,127])+struct.pack('!Q',n)
        c.sendall(header+data)
    def run(self):
        while not self.stop:
            try: c,_=self.sock.accept()
            except socket.timeout: continue
            try:
                c.settimeout(2); head=b''
                while b'\r\n\r\n' not in head: head+=c.recv(4096)
                key=next(line.split(b':',1)[1].strip() for line in head.split(b'\r\n') if line.lower().startswith(b'sec-websocket-key:'))
                accept=base64.b64encode(hashlib.sha1(key+b'258EAFA5-E914-47DA-95CA-C5AB0DC85B11').digest())
                c.sendall(b'HTTP/1.1 101 Switching Protocols\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Accept: '+accept+b'\r\n\r\n')
                while True:
                    msg=json.loads(self.frame(c)); method=msg['method']; p=msg.get('params',{})
                    if 'id' not in msg: continue
                    if self.native_error and method == 'thread/read':
                        self.send(c, {'id':msg['id'], 'error':{'message':self.native_error}}); continue
                    if method == self.block_method:
                        self.blocked.set(); self.release.wait(5)
                    thread={'id':CODEX_ID,'cwd':self.workspace,'status':{'type':self.status}}
                    if method=='initialize': result={}
                    elif method=='thread/loaded/list': result={'data':[CODEX_ID] if self.loaded else [], 'nextCursor':None}
                    elif method in ('thread/read','thread/resume'): result={'thread':thread}
                    elif method=='thread/turns/list': result={'data':self.turns,'nextCursor':'same' if self.repeat_cursor else None}
                    elif method=='turn/start':
                        self.starts+=1; turn_id=str(uuid.uuid4())
                        self.turns.append({'id':turn_id,'status':self.turn_status,'items':[{'type':'userMessage','clientId':p['clientUserMessageId'],'content':p['input']}]})
                        result={'turn':{'id':self.ack_override or turn_id}}
                        if self.block_method == 'turn/start_after':
                            self.blocked.set(); self.release.wait(5)
                        if self.lost_ack: self.lost_ack=False; break
                    else: raise ValueError(method)
                    self.send(c,{'id':msg['id'],'result':result})
            except (OSError,EOFError,ValueError,KeyError): pass
            finally: c.close()

@unittest.skipUnless(BINARY, 'set SESSION_BRIDGE_TEST_BINARY to the custody-built binary')
class BridgeIntegration(unittest.TestCase):
    def setUp(self):
        self.root=Path(tempfile.mkdtemp(prefix='bridge-offline-',dir=ROOT)); self.root.chmod(0o700)
        self.addCleanup(shutil.rmtree,self.root,True)
        self.locks=self.root/'locks'; self.locks.mkdir(mode=0o700)
        self.workspace=self.root/'workspace'; self.workspace.mkdir(mode=0o700)
        self.native=FakeNative(self.root/'native.sock',self.workspace); self.addCleanup(self.native.close)
        self.config=self.root/'config.json'; self.db=self.root/'state.sqlite'
        self.run_cli('store','init','--database',str(self.db),'--lock-dir',str(self.locks),'--native-socket',str(self.native.path))
        self.request=str(uuid.uuid4()); self.payload='test input exact\n'; self.input=self.root/'input'; self.input.write_text(self.payload)
        self.common=['--request-id',self.request,'--session-id',CODEX_ID,'--workspace',str(self.workspace),'--input-file',str(self.input)]
        preview=self.run_cli('grant','preview',*self.common)
        self.credential_file=self.root/'credential'
        self.grant=self.run_cli('grant','issue',*self.common,'--admit-until','2099-01-01T00:00:00Z','--credential-out',str(self.credential_file),'--dedicated-session','--reviewed-wire-sha256',preview['wire_sha256'])
        self.token=self.credential_file.read_text().strip()
        homes=SE.Homes(self.root/'codex',self.root/'claude',self.root/'events')
        day=homes.codex/'sessions'/'2026'/'11'/'01'; day.mkdir(parents=True)
        self.transcript=day/f'rollout-2026-11-01T00-00-00-{CODEX_ID}.jsonl'
        self.transcript.write_text(codex_line('session_meta',id=CODEX_ID,cwd=str(self.workspace)))
        digest=lambda p: hashlib.sha256(Path(p).read_bytes()).hexdigest()
        pinned=self.root/'session-bridge'; shutil.copyfile(BINARY,pinned); pinned.chmod(0o700)
        bridge=SE.Bridge(str(pinned),digest(pinned),str(self.config),digest(self.config))
        self.receiver=Receiver()
        self.server=SE.Server(homes,'tunnel',SE.CallbackPolicy(),self.receiver,bridge=bridge)
        self.daemon=SE.Daemon(homes,SE.CallbackPolicy(),self.receiver,log=lambda row: None)
        self.http=http.server.ThreadingHTTPServer(('127.0.0.1',0),SE.make_http_handler(self.server,'/mcp'))
        threading.Thread(target=self.http.serve_forever,daemon=True).start(); self.addCleanup(self.http.server_close); self.addCleanup(self.http.shutdown)
    def run_cli(self,*args,ok=True):
        p=subprocess.run([BINARY,'--config',str(self.config),*args],capture_output=True,check=True)
        response=json.loads(p.stdout)
        self.assertEqual(response['ok'],ok,response)
        return response.get('result') if ok else response['error']
    def post(self,method,params=None):
        conn=http.client.HTTPConnection('127.0.0.1',self.http.server_address[1],timeout=10)
        conn.request('POST','/mcp',json.dumps({'jsonrpc':'2.0','id':1,'method':method,'params':params or {}}),{'Content-Type':'application/json','Accept':'application/json, text/event-stream'})
        response=conn.getresponse(); result=json.loads(response.read()); conn.close(); return result['result']
    def call(self,name,arguments): return self.post('tools/call',{'name':name,'arguments':arguments})
    def test_lost_ack_duplicate_exact_terminal_and_revocation(self):
        self.assertIn('submit_session_request',[t['name'] for t in self.post('tools/list')['tools']])
        subscription=self.post('events/subscribe',{'name':SE.TURN_EVENT_NAME,'arguments':{'session_ids':[CODEX_ID]},'delivery':{'mode':'webhook','url':URL,'secret':whsec()}})['id']
        self.native.lost_ack=True
        args={'request_id':self.request,'grant_credential':self.token,'session_id':CODEX_ID,'workspace':str(self.workspace),'input_text':self.payload}
        first=self.call('submit_session_request',args)['structuredContent']
        self.assertEqual(first['entry_state'],'entry_uncertain'); self.assertEqual(self.native.starts,1)
        duplicate=self.call('submit_session_request',args)['structuredContent']
        self.assertEqual(duplicate['operation_id'],first['operation_id']); self.assertEqual(self.native.starts,1)
        wrong=dict(args,input_text='changed'); self.assertTrue(self.call('submit_session_request',wrong)['isError'])
        read={'request_id':self.request,'grant_credential':self.token}
        op=self.call('read_session_operation',read)['structuredContent']
        self.assertEqual(op['entry_state'],'terminal'); turn=op['native_turn_id']
        final='before '+self.token+' after'
        with self.transcript.open('a') as f: f.write(codex_line('event_msg',type='task_complete',turn_id=turn,last_agent_message=final))
        self.daemon.tick()
        replay=self.call('replay_events',{'subscription_id':subscription})['structuredContent']
        self.assertEqual(len(replay['events']),1)
        event=replay['events'][0]['event']
        self.assertEqual(event['data']['turn_id'],turn)
        self.assertNotIn(self.token,json.dumps(event))
        result=self.call('read_session_result',dict(read,max_chars=100))['structuredContent']
        self.assertNotIn('event_id',result['completion'])
        self.assertEqual(result['completion']['turn_id'],turn); self.assertNotIn(self.token,json.dumps(result)); self.assertIn('[REDACTED:bridge_credential]',result['result']['text'])
        self.assertFalse(result['process_closure']['available']); self.assertFalse(result['workers']['available'])
        self.run_cli('grant','revoke','--grant-id',self.grant['grant_id'])
        self.assertTrue(self.call('read_session_operation',read)['isError'])
        logs=(self.server.store.root/'requests.log').read_text()
        self.assertNotIn(self.token,logs); self.assertNotIn(self.payload,logs)

    def test_expiry_conflicts_and_unauthorized(self):
        args={'request_id':self.request,'grant_credential':self.token,'session_id':CODEX_ID,'workspace':str(self.workspace),'input_text':self.payload}
        self.assertTrue(self.call('submit_session_request',dict(args,grant_credential='webridge_bad'))['isError'])
        self.assertTrue(self.call('submit_session_request',dict(args,workspace=str(self.root)))['isError'])
        self.assertTrue(self.call('submit_session_request',dict(args,input_text='altered'))['isError'])
        with contextlib.closing(sqlite3.connect(self.db)) as db, db: db.execute('UPDATE grants SET expires=0')
        self.assertTrue(self.call('submit_session_request',args)['isError'])
        self.assertEqual(self.native.starts,0)

    def test_expiry_after_admission_preserves_read_and_conflict_stays_blocked(self):
        args={'request_id':self.request,'grant_credential':self.token,'session_id':CODEX_ID,'workspace':str(self.workspace),'input_text':self.payload}
        op=self.call('submit_session_request',args)['structuredContent']
        with contextlib.closing(sqlite3.connect(self.db)) as db, db: db.execute('UPDATE grants SET expires=0')
        self.native.turns[0]['items'][0]['content'][0]['text']='wrong wire'
        read={'request_id':self.request,'grant_credential':self.token}
        conflict=self.call('read_session_operation',read)['structuredContent']
        self.assertEqual(conflict['entry_state'],'identity_conflict')
        with contextlib.closing(sqlite3.connect(self.db)) as db, db:
            self.assertEqual(db.execute('SELECT count(*) FROM occupancy').fetchone()[0],1)
        self.assertEqual(self.native.starts,1)

    def test_unavailable_history_and_duplicate_concurrent_calls(self):
        args={'request_id':self.request,'grant_credential':self.token,'session_id':CODEX_ID,'workspace':str(self.workspace),'input_text':self.payload}
        self.native.repeat_cursor=True
        self.assertTrue(self.call('submit_session_request',args)['isError'])
        self.assertEqual(self.native.starts,0)
        self.native.repeat_cursor=False
        results=[]
        threads=[threading.Thread(target=lambda: results.append(self.call('submit_session_request',args))) for _ in range(2)]
        for thread in threads: thread.start()
        for thread in threads: thread.join(10)
        self.assertEqual(len(results),2)
        self.assertEqual(self.native.starts,1)
        self.assertEqual(results[0]['structuredContent']['operation_id'],results[1]['structuredContent']['operation_id'])

    def test_killed_after_admission_is_stranded(self):
        self.native.block_method='thread/resume'
        payload={'schema_version':1,'method':'submit_session_request','request_id':self.request,'grant_credential':self.token,'session_id':CODEX_ID,'workspace':str(self.workspace),'input_text':self.payload}
        child=subprocess.Popen([BINARY,'--config',str(self.config),'rpc'],stdin=subprocess.PIPE,stdout=subprocess.PIPE,stderr=subprocess.DEVNULL)
        try:
            child.stdin.write(json.dumps(payload).encode()); child.stdin.close()
            self.assertTrue(self.native.blocked.wait(8))
            child.kill(); child.wait(timeout=5)
        finally:
            if child.poll() is None: child.kill(); child.wait()
            child.stdout.close(); self.native.release.set()
        read={'request_id':self.request,'grant_credential':self.token}
        result=self.call('read_session_operation',read)['structuredContent']
        self.assertEqual(result['entry_state'],'admitted_no_attempt')
        self.assertEqual(self.native.starts,0)
        with contextlib.closing(sqlite3.connect(self.db)) as db, db: self.assertEqual(db.execute('SELECT count(*) FROM occupancy').fetchone()[0],1)

    def test_disabled_surface_and_bad_pin(self):
        plain=SE.Server(self.server.homes,'tunnel',SE.CallbackPolicy())
        listed=plain.handle({'jsonrpc':'2.0','id':1,'method':'tools/list'})['result']['tools']
        self.assertNotIn('submit_session_request',[t['name'] for t in listed])
        with self.assertRaises(ValueError): SE.Bridge(str(self.root/'session-bridge'),'0'*64,str(self.config),hashlib.sha256(self.config.read_bytes()).hexdigest())

    def test_kill_after_native_write_does_not_resend(self):
        self.native.block_method='turn/start_after'
        payload={'schema_version':1,'method':'submit_session_request','request_id':self.request,'grant_credential':self.token,'session_id':CODEX_ID,'workspace':str(self.workspace),'input_text':self.payload}
        child=subprocess.Popen([BINARY,'--config',str(self.config),'rpc'],stdin=subprocess.PIPE,stdout=subprocess.PIPE,stderr=subprocess.DEVNULL)
        try:
            child.stdin.write(json.dumps(payload).encode()); child.stdin.close()
            self.assertTrue(self.native.blocked.wait(8))
            child.kill(); child.wait(timeout=5)
        finally:
            if child.poll() is None: child.kill(); child.wait()
            child.stdout.close(); self.native.release.set()
        args={k:v for k,v in payload.items() if k not in ('schema_version','method')}
        duplicate=self.call('submit_session_request',args)['structuredContent']
        self.assertEqual(duplicate['entry_state'],'entry_uncertain')
        read={'request_id':self.request,'grant_credential':self.token}
        result=self.call('read_session_operation',read)['structuredContent']
        self.assertEqual(result['entry_state'],'terminal'); self.assertEqual(self.native.starts,1)

    def test_revocation_before_admission_wins(self):
        self.native.block_method='thread/read'
        payload={'schema_version':1,'method':'submit_session_request','request_id':self.request,'grant_credential':self.token,'session_id':CODEX_ID,'workspace':str(self.workspace),'input_text':self.payload}
        child=subprocess.Popen([BINARY,'--config',str(self.config),'rpc'],stdin=subprocess.PIPE,stdout=subprocess.PIPE,stderr=subprocess.DEVNULL)
        try:
            child.stdin.write(json.dumps(payload).encode()); child.stdin.close()
            self.assertTrue(self.native.blocked.wait(8))
            self.run_cli('grant','revoke','--grant-id',self.grant['grant_id'])
            self.native.release.set()
            result=json.loads(child.stdout.read()); child.wait(timeout=5)
            self.assertFalse(result['ok']); self.assertEqual(result['error'],'unauthorized')
            self.assertEqual(self.native.starts,0)
            with contextlib.closing(sqlite3.connect(self.db)) as db, db: self.assertEqual(db.execute('SELECT count(*) FROM operations').fetchone()[0],0)
        finally:
            self.native.release.set()
            if child.poll() is None: child.kill(); child.wait()
            child.stdout.close()

    def test_revocation_after_admission_does_not_cancel(self):
        self.native.block_method='thread/resume'
        payload={'schema_version':1,'method':'submit_session_request','request_id':self.request,'grant_credential':self.token,'session_id':CODEX_ID,'workspace':str(self.workspace),'input_text':self.payload}
        child=subprocess.Popen([BINARY,'--config',str(self.config),'rpc'],stdin=subprocess.PIPE,stdout=subprocess.PIPE,stderr=subprocess.DEVNULL)
        try:
            child.stdin.write(json.dumps(payload).encode()); child.stdin.close()
            self.assertTrue(self.native.blocked.wait(8))
            self.run_cli('grant','revoke','--grant-id',self.grant['grant_id'])
            self.native.release.set(); result=json.loads(child.stdout.read()); child.wait(timeout=5)
            self.assertTrue(result['ok']); self.assertEqual(self.native.starts,1)
            self.assertTrue(self.call('read_session_operation',{'request_id':self.request,'grant_credential':self.token})['isError'])
        finally:
            self.native.release.set()
            if child.poll() is None: child.kill(); child.wait()
            child.stdout.close()

    def test_partial_wrong_turn_and_redaction_before_paging(self):
        args={'request_id':self.request,'grant_credential':self.token,'session_id':CODEX_ID,'workspace':str(self.workspace),'input_text':self.payload}
        self.call('submit_session_request',args)
        read={'request_id':self.request,'grant_credential':self.token}
        operation=self.call('read_session_operation',read)['structuredContent']
        turn=operation['native_turn_id']
        with self.transcript.open('a') as f: f.write(codex_line('event_msg',type='task_complete',turn_id='wrong',last_agent_message='wrong'))
        pending=self.call('read_session_result',read)['structuredContent']
        self.assertFalse(pending['result']['available'])
        with self.transcript.open('a') as f: f.write('{"partial":')
        self.assertFalse(self.call('read_session_result',read)['structuredContent']['result']['available'])
        with self.transcript.open('rb+') as f:
            f.seek(-11,2); f.truncate()
        secret='webridge_'+'a'*42+'-'
        with self.transcript.open('a') as f: f.write(codex_line('event_msg',type='task_complete',turn_id=turn,last_agent_message='12345'+secret+' tail'))
        page=self.call('read_session_result',dict(read,text_offset=5,max_chars=8))['structuredContent']
        self.assertEqual(page['result']['text'],'[REDACTE'); self.assertNotIn(secret,json.dumps(page))

    def test_missing_and_corrupt_store_fail_closed(self):
        self.db.rename(self.root/'saved.sqlite')
        response=self.run_cli('operation','inspect','--request-id',self.request,ok=False)
        self.assertEqual(response,'bridge_unavailable')
        self.db.write_bytes(b'not sqlite'); self.db.chmod(0o600)
        self.assertEqual(self.run_cli('operation','inspect','--request-id',self.request,ok=False),'bridge_unavailable')

    def test_delayed_history_never_replays_and_then_correlates(self):
        args={'request_id':self.request,'grant_credential':self.token,'session_id':CODEX_ID,'workspace':str(self.workspace),'input_text':self.payload}
        self.native.lost_ack=True
        self.call('submit_session_request',args)
        saved=self.native.turns[:]; self.native.turns=[]
        read={'request_id':self.request,'grant_credential':self.token}
        pending=self.call('read_session_operation',read)['structuredContent']
        self.assertEqual(pending['entry_state'],'entry_uncertain')
        self.assertEqual(self.native.starts,1)
        self.native.turns=saved
        self.assertEqual(self.call('read_session_operation',read)['structuredContent']['entry_state'],'terminal')

    def test_mismatched_ack_and_duplicate_correlations_conflict(self):
        self.native.ack_override=str(uuid.uuid4())
        args={'request_id':self.request,'grant_credential':self.token,'session_id':CODEX_ID,'workspace':str(self.workspace),'input_text':self.payload}
        self.call('submit_session_request',args)
        read={'request_id':self.request,'grant_credential':self.token}
        self.assertEqual(self.call('read_session_operation',read)['structuredContent']['entry_state'],'identity_conflict')
        self.assertEqual(self.native.starts,1)
        with contextlib.closing(sqlite3.connect(self.db)) as db, db: self.assertEqual(db.execute('SELECT count(*) FROM occupancy').fetchone()[0],1)

    def test_not_loaded_and_busy_fail_without_send(self):
        args={'request_id':self.request,'grant_credential':self.token,'session_id':CODEX_ID,'workspace':str(self.workspace),'input_text':self.payload}
        self.native.loaded=False
        self.assertTrue(self.call('submit_session_request',args)['isError'])
        self.native.loaded=True; self.native.status='active'
        busy=self.call('submit_session_request',args)
        self.assertTrue(busy['isError']); self.assertIn('session_busy',json.dumps(busy))
        self.assertEqual(self.native.starts,0)

    def test_store_permissions_and_private_input_not_in_db(self):
        args={'request_id':self.request,'grant_credential':self.token,'session_id':CODEX_ID,'workspace':str(self.workspace),'input_text':self.payload}
        self.config.chmod(0o644)
        self.assertTrue(self.call('submit_session_request',args)['isError'])
        self.config.chmod(0o600)
        self.call('submit_session_request',args)
        content=self.db.read_bytes()
        self.assertNotIn(self.payload.encode(),content); self.assertNotIn(self.token.encode(),content)
        for journal in self.root.glob('*.sqlite-journal'):
            self.assertNotIn(self.payload.encode(),journal.read_bytes())
            self.assertNotIn(self.token.encode(),journal.read_bytes())

    @unittest.skipUnless(os.environ.get('SESSION_BRIDGE_TEST_HOOK_BINARY'), 'requires separate crash-test feature build')
    def test_kill_after_intent_before_native_write_remains_uncertain(self):
        payload={'schema_version':1,'method':'submit_session_request','request_id':self.request,'grant_credential':self.token,'session_id':CODEX_ID,'workspace':str(self.workspace),'input_text':self.payload}
        env=dict(os.environ, SESSION_BRIDGE_TEST_STOP_BEFORE_WRITE='1')
        child=subprocess.Popen([env['SESSION_BRIDGE_TEST_HOOK_BINARY'],'--config',str(self.config),'rpc'],stdin=subprocess.PIPE,stdout=subprocess.PIPE,stderr=subprocess.DEVNULL,env=env)
        try:
            child.stdin.write(json.dumps(payload).encode()); child.stdin.close()
            deadline=time.monotonic()+10; stopped=False
            while time.monotonic()<deadline:
                pid,status=os.waitpid(child.pid,os.WNOHANG|os.WUNTRACED)
                if pid and os.WIFSTOPPED(status): stopped=True; break
                time.sleep(.01)
            self.assertTrue(stopped)
            child.kill(); child.wait(timeout=5)
        finally:
            if child.poll() is None: child.kill(); child.wait()
            child.stdout.close()
        read={'request_id':self.request,'grant_credential':self.token}
        state=self.call('read_session_operation',read)['structuredContent']
        self.assertEqual(state['entry_state'],'entry_uncertain')
        self.assertEqual(self.native.starts,0)
        args={k:v for k,v in payload.items() if k not in ('schema_version','method')}
        self.assertEqual(self.call('submit_session_request',args)['structuredContent']['entry_state'],'entry_uncertain')
        self.assertEqual(self.native.starts,0)

    def test_kill_before_admission_leaves_grant_unused(self):
        self.native.block_method='thread/read'
        payload={'schema_version':1,'method':'submit_session_request','request_id':self.request,'grant_credential':self.token,'session_id':CODEX_ID,'workspace':str(self.workspace),'input_text':self.payload}
        child=subprocess.Popen([BINARY,'--config',str(self.config),'rpc'],stdin=subprocess.PIPE,stdout=subprocess.PIPE,stderr=subprocess.DEVNULL)
        try:
            child.stdin.write(json.dumps(payload).encode()); child.stdin.close()
            self.assertTrue(self.native.blocked.wait(8)); child.kill(); child.wait(timeout=5)
        finally:
            if child.poll() is None: child.kill(); child.wait()
            child.stdout.close(); self.native.release.set()
        with contextlib.closing(sqlite3.connect(self.db)) as db, db: self.assertEqual(db.execute('SELECT count(*) FROM operations').fetchone()[0],0)
        self.assertEqual(self.native.starts,0)

    def test_multiple_matching_turns_block_occupancy(self):
        args={'request_id':self.request,'grant_credential':self.token,'session_id':CODEX_ID,'workspace':str(self.workspace),'input_text':self.payload}
        self.call('submit_session_request',args)
        duplicate=json.loads(json.dumps(self.native.turns[0])); duplicate['id']=str(uuid.uuid4()); self.native.turns.append(duplicate)
        read={'request_id':self.request,'grant_credential':self.token}
        self.assertEqual(self.call('read_session_operation',read)['structuredContent']['entry_state'],'identity_conflict')
        with contextlib.closing(sqlite3.connect(self.db)) as db, db: self.assertEqual(db.execute('SELECT count(*) FROM occupancy').fetchone()[0],1)

    def test_lost_baseline_history_is_unavailable(self):
        old={'id':str(uuid.uuid4()),'status':'completed','items':[]}; self.native.turns.append(old)
        args={'request_id':self.request,'grant_credential':self.token,'session_id':CODEX_ID,'workspace':str(self.workspace),'input_text':self.payload}
        self.call('submit_session_request',args); self.native.turns.remove(old)
        result=self.call('read_session_operation',{'request_id':self.request,'grant_credential':self.token})
        self.assertTrue(result['isError']); self.assertEqual(self.native.starts,1)
        with contextlib.closing(sqlite3.connect(self.db)) as db, db: self.assertEqual(db.execute('SELECT count(*) FROM occupancy').fetchone()[0],1)

    def test_native_error_never_leaks_and_failed_turn_has_empty_final(self):
        args={'request_id':self.request,'grant_credential':self.token,'session_id':CODEX_ID,'workspace':str(self.workspace),'input_text':self.payload}
        self.native.native_error='raw secret '+self.token
        result=self.call('submit_session_request',args)
        self.assertTrue(result['isError']); self.assertNotIn(self.token,json.dumps(result))
        self.native.native_error=None
        self.call('submit_session_request',args)
        self.native.turns[0]['status']='failed'
        read={'request_id':self.request,'grant_credential':self.token}
        op=self.call('read_session_operation',read)['structuredContent']; turn=op['native_turn_id']
        with self.transcript.open('a') as f:
            f.write(codex_line('event_msg',type='task_complete',turn_id=turn,last_agent_message='',error={'message':self.token}))
        result=self.call('read_session_result',read)['structuredContent']
        self.assertEqual(result['result']['text'],''); self.assertNotIn(self.token,json.dumps(result))

    @unittest.skipUnless(os.environ.get('SESSION_BRIDGE_TEST_HOOK_BINARY'), 'requires separate crash-test feature build')
    def test_kill_after_ack_before_persist_recovers(self):
        payload={'schema_version':1,'method':'submit_session_request','request_id':self.request,'grant_credential':self.token,'session_id':CODEX_ID,'workspace':str(self.workspace),'input_text':self.payload}
        env=dict(os.environ, SESSION_BRIDGE_TEST_STOP_BEFORE_ACK_PERSIST='1')
        child=subprocess.Popen([env['SESSION_BRIDGE_TEST_HOOK_BINARY'],'--config',str(self.config),'rpc'],stdin=subprocess.PIPE,stdout=subprocess.PIPE,stderr=subprocess.DEVNULL,env=env)
        try:
            child.stdin.write(json.dumps(payload).encode()); child.stdin.close()
            deadline=time.monotonic()+10; stopped=False
            while time.monotonic()<deadline:
                pid,status=os.waitpid(child.pid,os.WNOHANG|os.WUNTRACED)
                if pid and os.WIFSTOPPED(status): stopped=True; break
                time.sleep(.01)
            self.assertTrue(stopped); child.kill(); child.wait(timeout=5)
        finally:
            if child.poll() is None: child.kill(); child.wait()
            child.stdout.close()
        self.assertEqual(self.native.starts,1)
        read={'request_id':self.request,'grant_credential':self.token}
        self.assertEqual(self.call('read_session_operation',read)['structuredContent']['entry_state'],'terminal')
        self.assertEqual(self.native.starts,1)

    def test_missing_client_id_and_payload_only_never_correlate(self):
        args={'request_id':self.request,'grant_credential':self.token,'session_id':CODEX_ID,'workspace':str(self.workspace),'input_text':self.payload}
        self.native.lost_ack=True; self.call('submit_session_request',args)
        item=self.native.turns[0]['items'][0]; client=item.pop('clientId')
        read={'request_id':self.request,'grant_credential':self.token}
        self.assertEqual(self.call('read_session_operation',read)['structuredContent']['entry_state'],'entry_uncertain')
        item['clientId']=client; item['content'][0]['text']=self.payload
        self.assertEqual(self.call('read_session_operation',read)['structuredContent']['entry_state'],'identity_conflict')
        self.assertEqual(self.native.starts,1)

    def test_unknown_authority_fields_rejected(self):
        args={'request_id':self.request,'grant_credential':self.token,'session_id':CODEX_ID,'workspace':str(self.workspace),'input_text':self.payload}
        for key in ('approval','mandate','decision_batch'):
            self.assertTrue(self.call('submit_session_request',dict(args,**{key:'spoof'}))['isError'])
        self.assertEqual(self.native.starts,0)

    def test_occupied_session_refuses_another_grant(self):
        req=str(uuid.uuid4()); common=['--request-id',req,'--session-id',CODEX_ID,'--workspace',str(self.workspace),'--input-file',str(self.input)]
        preview=self.run_cli('grant','preview',*common); out=self.root/'other-token'
        self.run_cli('grant','issue',*common,'--admit-until','2099-01-01T00:00:00Z','--credential-out',str(out),'--dedicated-session','--reviewed-wire-sha256',preview['wire_sha256'])
        self.native.turn_status='inProgress'
        args={'request_id':self.request,'grant_credential':self.token,'session_id':CODEX_ID,'workspace':str(self.workspace),'input_text':self.payload}
        self.call('submit_session_request',args)
        self.assertTrue(self.call('submit_session_request',dict(args,request_id=req,grant_credential=out.read_text().strip()))['isError'])
        self.assertEqual(self.native.starts,1)

    def test_durable_occupancy_refuses_when_native_appears_idle(self):
        req=str(uuid.uuid4()); common=['--request-id',req,'--session-id',CODEX_ID,'--workspace',str(self.workspace),'--input-file',str(self.input)]
        preview=self.run_cli('grant','preview',*common); out=self.root/'other-uncertain-token'
        self.run_cli('grant','issue',*common,'--admit-until','2099-01-01T00:00:00Z','--credential-out',str(out),'--dedicated-session','--reviewed-wire-sha256',preview['wire_sha256'])
        self.native.lost_ack=True
        args={'request_id':self.request,'grant_credential':self.token,'session_id':CODEX_ID,'workspace':str(self.workspace),'input_text':self.payload}
        first=self.call('submit_session_request',args)['structuredContent']
        self.assertEqual(first['entry_state'],'entry_uncertain')
        self.native.turns=[]  # fake history and status now appear idle, with no positive receipt
        second=dict(args,request_id=req,grant_credential=out.read_text().strip())
        refused=self.call('submit_session_request',second)
        self.assertTrue(refused['isError']); self.assertIn('session_busy',json.dumps(refused))
        with contextlib.closing(sqlite3.connect(self.db)) as db, db:
            self.assertEqual(db.execute('SELECT request_id FROM occupancy WHERE session_id=?',(CODEX_ID,)).fetchone(),(self.request,))
            self.assertEqual(db.execute('SELECT count(*) FROM operations').fetchone()[0],1)
        self.assertEqual(self.native.starts,1)
