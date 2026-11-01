"""Disposable fake-native tests for the opt-in provisioning path."""
import base64, hashlib, json, os, shutil, socket, sqlite3, struct, subprocess, tempfile, threading, time, unittest, uuid
from pathlib import Path
from test_bridge import BINARY, ROOT, FakeNative
from test_session_events import SE

FLAGS = ['apps','plugins','remote_plugin','multi_agent','multi_agent_v2']
SERVERS = ['codebase-memory-mcp','cua_repl','node_repl']
PLUGINS = ['browser@openai-bundled','chrome@openai-bundled','code-review@openai-bundled','codex-app-tools@openai-bundled','documents@openai-primary-runtime','github@openai-curated','pdf@openai-primary-runtime','presentations@openai-primary-runtime','spreadsheets@openai-primary-runtime','template-creator@openai-primary-runtime','unified-computer-use@openai-bundled','visualize@openai-bundled']

class ProvisionNative(FakeNative):
    def __init__(self, path, workspace):
        super().__init__(path, workspace)
        self.target = str(uuid.uuid4()); self.loaded = False; self.allocations = 0
        self.injections = 0; self.archives = 0; self.lost_allocation = False; self.lost_close = False
        self.bad_config = False; self.bad_features = False; self.bad_ack = False; self.unreadable = False; self.unmaterialized = True; self.lost_injection = False; self.workers = []
        self.delay_fragment=False; self.held=None; self.pong=threading.Event(); self.ping_payload=b'idle-heartbeat'; self.methods=[]
    def run(self):
        while not self.stop:
            try: c,_ = self.sock.accept()
            except socket.timeout: continue
            t = threading.Thread(target=self.client,args=(c,),daemon=True); t.start(); self.workers.append(t)
    def close(self):
        super().close()
        for t in self.workers: t.join(3)
    def frame(self,c):
        header=self.exact(c,2); opcode=header[0]&15; n=header[1]&127
        if n==126: n=struct.unpack('!H',self.exact(c,2))[0]
        if n==127: n=struct.unpack('!Q',self.exact(c,8))[0]
        mask=self.exact(c,4) if header[1]&128 else b''
        data=self.exact(c,n)
        if mask: data=bytes(v^mask[i%4] for i,v in enumerate(data))
        if opcode==10:
            if data==self.ping_payload:self.pong.set()
            return None
        return data
    def send_ping(self):
        notification=json.dumps({'method':'fixture/notification','params':{}}).encode()
        self.held.sendall(bytes([0x81,len(notification)])+notification+bytes([0x89,len(self.ping_payload)])+self.ping_payload)
    def client(self,c):
        try:
            c.settimeout(30); head=b''
            while b'\r\n\r\n' not in head:
                chunk=c.recv(4096)
                if not chunk: return
                head+=chunk
            key=next(line.split(b':',1)[1].strip() for line in head.split(b'\r\n') if line.lower().startswith(b'sec-websocket-key:'))
            accept=base64.b64encode(hashlib.sha1(key+b'258EAFA5-E914-47DA-95CA-C5AB0DC85B11').digest())
            c.sendall(b'HTTP/1.1 101 Switching Protocols\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Accept: '+accept+b'\r\n\r\n')
            while True:
                raw=self.frame(c)
                if raw is None: continue
                msg=json.loads(raw); method=msg['method']; p=msg.get('params',{}); self.methods.append(method)
                if 'id' not in msg: continue
                if self.delay_fragment:
                    time.sleep(3); c.sendall(b'\x80\x09fixture"}'); self.delay_fragment=False
                thread={'id':self.target,'cwd':self.workspace,'status':{'type':'idle' if self.loaded else 'notLoaded'}}
                ack={'thread':thread,'model':'gpt-6.1-terra','reasoningEffort':'medium','serviceTier':'default','approvalPolicy':'never','sandbox':{'type':'readOnly','networkAccess':False}}
                if self.bad_ack: ack['model']='unexpected-model'
                if method=='initialize': result={}
                elif method=='config/read':
                    config={'model':'gpt-6.1-terra','model_reasoning_effort':'medium','service_tier':'default','approval_policy':'never','sandbox_mode':'read-only','web_search':'disabled','features':{k:False for k in FLAGS},'mcp_servers':{k:{'enabled':False} for k in SERVERS},'plugins':{k:{'enabled':False} for k in PLUGINS},'apps':{'_default':{'enabled':False}}}
                    if self.bad_config: config['apps']['_default']['enabled']=True
                    result={'config':config,'origins':{},'layers':[{'name':{'type':'project','dotCodexFolder':self.workspace+'/.codex'},'disabledReason':None},{'name':{'type':'user','file':str(Path(self.workspace).parent/'global.toml')},'disabledReason':None}]}
                elif method=='thread/start':
                    self.allocations+=1; self.loaded=True; self.held=c; ack['thread']['status']['type']='idle'; result=ack
                    if self.lost_allocation: return
                elif method=='experimentalFeature/list':
                    result={'data':[{'name':k,'enabled':self.bad_features and k=='apps'} for k in FLAGS],'nextCursor':None}
                elif method=='thread/inject_items':
                    self.injections+=1; result={}
                    if self.lost_injection: return
                elif method=='thread/loaded/list': result={'data':[self.target] if self.loaded else [],'nextCursor':None}
                elif method=='thread/read': result={'thread':thread}
                elif method=='thread/resume': result=ack
                elif method=='thread/turns/list':
                    if self.unreadable or (self.unmaterialized and not self.injections):
                        self.send(c,{'id':msg['id'],'error':{'message':'unreadable'}}); continue
                    result={'data':self.turns,'nextCursor':None}
                elif method=='turn/start':
                    self.starts+=1; tid=str(uuid.uuid4()); self.turns.append({'id':tid,'status':'completed','items':[{'type':'userMessage','clientId':p['clientUserMessageId'],'content':p['input']}]}); result={'turn':{'id':tid}}
                elif method=='thread/archive':
                    self.archives+=1; self.loaded=False; result={}
                    if self.lost_close: return
                else: raise ValueError(method)
                self.send(c,{'id':msg['id'],'result':result})
        except (OSError,EOFError,ValueError,KeyError): pass
        finally: c.close()

@unittest.skipUnless(BINARY, 'set SESSION_BRIDGE_TEST_BINARY')
class Provisioning(unittest.TestCase):
    def setUp(self):
        self.root=Path(tempfile.mkdtemp(prefix='pv-',dir=ROOT)); self.root.chmod(0o700)
        self.addCleanup(shutil.rmtree,self.root,True)
        self.locks=self.root/'locks'; self.locks.mkdir(mode=0o700)
        self.workspace=self.root/'workspace'; self.workspace.mkdir(mode=0o700)
        subprocess.run(['/usr/bin/git','init','-q',str(self.workspace)],check=True)
        (self.workspace/'a').write_text('checkpoint\n')
        self.git('add','a'); self.git('-c','user.name=Fixture','-c','user.email=fixture@example.invalid','commit','-qm','fixture')
        self.head=subprocess.check_output(['/usr/bin/git','-C',str(self.workspace),'rev-parse','HEAD'],text=True).strip()
        (self.workspace/'.git/info/exclude').write_text('.codex/\n')
        (self.workspace/'.codex').mkdir(); self.local=self.workspace/'.codex/config.toml'; self.local.write_text('fixture\n')
        self.global_cfg=self.root/'global.toml'; self.global_cfg.write_text('fixture\n')
        self.native=ProvisionNative(self.root/'native.sock',self.workspace); self.addCleanup(self.native.close)
        self.config=self.root/'config.json'; self.db=self.root/'state.sqlite'
        self.cli('store','init','--database',str(self.db),'--lock-dir',str(self.locks),'--native-socket',str(self.native.path))
        self.cli('provisioning','init')
        self.payload='fixture request'; self.request=str(uuid.uuid4()); self.key=str(uuid.uuid4())
        self.policy={'schema_version':1,'task_revision':'fixture-task-rev-1','profile_name':'diagnostic-v1','workspace':str(self.workspace),'checkpoint':self.head,'model':'gpt-6.1-terra','reasoning_effort':'medium','service_tier':'default','sandbox':'read-only','network_access':False,'approval_policy':'never','global_config':str(self.global_cfg),'global_config_sha256':self.hash(self.global_cfg),'workspace_config':str(self.local),'workspace_config_sha256':self.hash(self.local),'request_id':self.request,'payload_sha256':hashlib.sha256(self.payload.encode()).hexdigest(),'admit_until':int(time.time())+120,'close_by':int(time.time())+180,'budget_reference':'fixture-budget','budget_units':'USD','budget_amount':'25','reserve_controller':'operator','reserve_amount':'10','first_funded_clock':'supervisor-owned'}
        self.policy_file=self.root/'policy.json'; self.policy_file.write_text(json.dumps(self.policy)); self.policy_file.chmod(0o600)
        self.credential_file=self.root/'permission'; self.permission=self.cli('provisioning','issue','--policy-file',str(self.policy_file),'--credential-out',str(self.credential_file))['result']; self.token=self.credential_file.read_text().strip()
        self.owner=None; self.start_owner(); self.addCleanup(self.stop_owner)
    def git(self,*args): subprocess.run(['/usr/bin/git','-C',str(self.workspace),*args],check=True,stdout=subprocess.DEVNULL)
    @staticmethod
    def hash(p): return hashlib.sha256(p.read_bytes()).hexdigest()
    def cli(self,*args,input=None):
        p=subprocess.run([BINARY,'--config',str(self.config),*args],input=input,text=True,capture_output=True,timeout=30,check=True)
        return json.loads(p.stdout)
    def start_owner(self):
        path=self.locks/'provision-owner.sock'
        self.owner=subprocess.Popen([BINARY,'--config',str(self.config),'provisioning','owner','serve'],stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL)
        for _ in range(200):
            if path.exists() and self.owner.poll() is None:
                result=self.rpc('read_session_target')
                if result['ok'] and result['result'].get('owner_health') != 'unavailable': return
            if self.owner.poll() is not None: break
            time.sleep(.01)
        self.fail('owner did not start')
    def stop_owner(self):
        if self.owner and self.owner.poll() is None: self.owner.terminate()
        if self.owner: self.owner.wait(timeout=5)
    def rpc(self,method,token=None,**kwargs):
        return self.cli('rpc',input=json.dumps({'schema_version':1,'method':method,'provisioning_credential':self.token if token is None else token,**kwargs}))
    def create(self): return self.rpc('create_session_target',idempotency_key=self.key)
    def test_vertical_and_isolation(self):
        self.assertFalse(self.rpc('create_session_target',token='weprovision_wrong',idempotency_key=self.key)['ok'])
        created=self.create()['result']; self.assertEqual(created['setup_state'],'ready'); self.assertEqual(created['synthetic_initialization'],'one_empty_assistant_output_text')
        self.assertEqual(created['effective_profile']['model'],'gpt-6.1-terra'); self.assertFalse(created['effective_profile']['network_access'])
        self.assertEqual(self.create()['result']['target_id'],created['target_id']); self.assertEqual(self.native.allocations,1)
        self.assertFalse(self.rpc('create_session_target',idempotency_key=str(uuid.uuid4()))['ok'])
        self.assertFalse(self.rpc('issue_session_request_grant',input_text='different')['ok'])
        grant=self.rpc('issue_session_request_grant',input_text=self.payload)['result']; self.assertEqual(grant['grant_handle'],self.rpc('issue_session_request_grant',input_text=self.payload)['result']['grant_handle'])
        request={'request_id':self.request,'grant_handle':grant['grant_handle'],'session_id':created['target_id'],'workspace':str(self.workspace),'input_text':self.payload}
        self.assertFalse(self.rpc('submit_session_request',**dict(request,grant_handle=str(uuid.uuid4())))['ok'])
        mixed={'schema_version':1,'method':'submit_session_request','provisioning_credential':self.token,'grant_credential':'webridge_bad',**request}
        self.assertFalse(self.cli('rpc',input=json.dumps(mixed))['ok'])
        first=self.rpc('submit_session_request',**request)['result']; self.assertEqual(first['entry_state'],'entry_uncertain')
        with sqlite3.connect(self.db) as db:
            child_secret=db.execute('SELECT child_token FROM provision_permissions WHERE permission_id=?',(self.permission['permission_id'],)).fetchone()[0]
        self.assertNotIn(self.token,json.dumps(self.native.turns)); self.assertNotIn(child_secret,json.dumps(self.native.turns))
        self.assertEqual(self.rpc('submit_session_request',**request)['result']['operation_id'],first['operation_id']); self.assertEqual(self.native.starts,1)
        self.assertEqual(self.rpc('issue_session_request_grant',input_text=self.payload)['result']['grant_handle'],grant['grant_handle'])
        read=self.rpc('read_session_operation',request_id=self.request,grant_handle=grant['grant_handle']); self.assertTrue(read['ok'],read)
        closed=self.rpc('close_session_target')['result']; self.assertEqual(closed['closure_state'],'closed'); self.assertEqual(self.native.archives,1)
        self.assertFalse(self.rpc('read_session_operation',request_id=self.request,grant_handle=grant['grant_handle'])['ok'])
        self.assertNotIn(self.token,json.dumps(closed)); self.assertGreater(closed['audit_transition_count'],3)
        with sqlite3.connect(self.db) as db:
            audit=json.dumps(db.execute('SELECT * FROM provision_transitions').fetchall())
        self.assertNotIn(self.token,audit); self.assertNotIn('webridge_',audit)
    def test_lost_allocation_is_never_retried(self):
        self.native.lost_allocation=True
        first=self.create()['result']; self.assertEqual(first['setup_state'],'allocation_uncertain')
        self.assertEqual(self.create()['result']['setup_state'],'allocation_uncertain'); self.assertEqual(self.native.allocations,1)
        self.assertEqual(self.rpc('close_session_target')['result']['closure_state'],'uncertain')
    def test_foreign_socket_is_not_removed_on_restart(self):
        self.stop_owner(); path=self.locks/'provision-owner.sock'; path.unlink()
        foreign=socket.socket(socket.AF_UNIX,socket.SOCK_STREAM); foreign.bind(str(path)); foreign.listen()
        try:
            before=path.stat().st_ino
            result=self.cli('provisioning','owner','serve')
            self.assertFalse(result['ok']); self.assertEqual(path.stat().st_ino,before)
        finally:
            foreign.close(); path.unlink()
    def test_custody_loss_and_close(self):
        self.assertEqual(self.create()['result']['setup_state'],'ready')
        self.stop_owner(); self.start_owner()
        read=self.rpc('read_session_target')['result']; self.assertEqual(read['setup_state'],'custody_lost')
        self.assertFalse(self.rpc('issue_session_request_grant',input_text=self.payload)['ok'])
        self.assertEqual(self.rpc('close_session_target')['result']['closure_state'],'closed')
    def test_profile_and_history_fail_closed(self):
        self.native.bad_config=True
        self.assertFalse(self.create()['ok']); self.assertEqual(self.native.allocations,0)
        self.native.bad_config=False; self.native.unreadable=True
        result=self.create()['result']; self.assertEqual(result['setup_state'],'initialization_uncertain')
        self.assertFalse(self.rpc('issue_session_request_grant',input_text=self.payload)['ok'])
    def test_lost_issuance_ack_returns_same_handle(self):
        self.assertEqual(self.create()['result']['setup_state'],'ready')
        sock=socket.socket(socket.AF_UNIX,socket.SOCK_STREAM); sock.connect(str(self.locks/'provision-owner.sock'))
        sock.sendall((json.dumps({'method':'issue_session_request_grant','provisioning_credential':self.token,'input_text':self.payload})+'\n').encode()); sock.close()
        handle=None
        for _ in range(100):
            state=self.rpc('read_session_target')['result']; handle=state.get('grant_handle')
            if handle: break
            time.sleep(.02)
        self.assertIsNotNone(handle)
        self.assertEqual(self.rpc('issue_session_request_grant',input_text=self.payload)['result']['grant_handle'],handle)
    def test_mismatched_native_ack_cannot_issue(self):
        self.native.bad_ack=True
        result=self.create()['result']; self.assertEqual(result['setup_state'],'identified')
        self.assertEqual(self.native.injections,0)
        self.assertFalse(self.rpc('issue_session_request_grant',input_text=self.payload)['ok'])
    def test_policy_rejects_bearer_material_in_projected_fields(self):
        bad=dict(self.policy,task_revision='task-'+self.token)
        policy=self.root/'secret-policy'; policy.write_text(json.dumps(bad)); policy.chmod(0o600)
        result=self.cli('provisioning','issue','--policy-file',str(policy),'--credential-out',str(self.root/'unused-secret'))
        self.assertFalse(result['ok']); self.assertNotIn(self.token,json.dumps(result))
    def test_policy_file_edits_do_not_change_issued_authority(self):
        altered=dict(self.policy,payload_sha256=hashlib.sha256(b'other').hexdigest())
        self.policy_file.write_text(json.dumps(altered))
        self.assertEqual(self.create()['result']['setup_state'],'ready')
        self.assertFalse(self.rpc('issue_session_request_grant',input_text='other')['ok'])
        self.assertTrue(self.rpc('issue_session_request_grant',input_text=self.payload)['ok'])
    def test_unknown_allocation_blocks_another_permission(self):
        self.native.lost_allocation=True
        self.assertEqual(self.create()['result']['setup_state'],'allocation_uncertain')
        other=dict(self.policy,task_revision='other-unknown',request_id=str(uuid.uuid4()))
        policy=self.root/'other-unknown-policy'; policy.write_text(json.dumps(other)); policy.chmod(0o600)
        credential=self.root/'other-unknown-credential'; self.cli('provisioning','issue','--policy-file',str(policy),'--credential-out',str(credential))
        self.assertFalse(self.rpc('create_session_target',token=credential.read_text().strip(),idempotency_key=str(uuid.uuid4()))['ok'])
        self.assertEqual(self.native.allocations,1)
    def test_unpinned_ignored_files_and_git_filters_are_rejected(self):
        (self.workspace/'.git/info/exclude').write_text('.codex/\nignored-secret\n')
        (self.workspace/'ignored-secret').write_text('unbound\n')
        self.assertFalse(self.create()['ok']); self.assertEqual(self.native.allocations,0)
        (self.workspace/'ignored-secret').unlink()
        self.git('config','filter.evil.clean','/bin/false')
        self.assertFalse(self.create()['ok']); self.assertEqual(self.native.allocations,0)
    def test_changed_checkpoint_prevents_allocation(self):
        (self.workspace/'a').write_text('changed\n')
        self.assertFalse(self.create()['ok']); self.assertEqual(self.native.allocations,0)
    def test_hidden_index_changes_block_allocation_and_submission(self):
        for flag in ('--assume-unchanged', '--skip-worktree'):
            with self.subTest(stage='allocation', flag=flag):
                self.git('update-index',flag,'a'); (self.workspace/'a').write_text('hidden change\n')
                self.assertEqual(subprocess.check_output(['/usr/bin/git','-C',str(self.workspace),'status','--porcelain']),b'')
                self.assertFalse(self.create()['ok']); self.assertEqual(self.native.allocations,0)
                self.git('update-index',flag.replace('--','--no-',1),'a'); (self.workspace/'a').write_text('checkpoint\n')
        created=self.create()['result']; grant=self.rpc('issue_session_request_grant',input_text=self.payload)['result']
        request={'request_id':self.request,'grant_handle':grant['grant_handle'],'session_id':created['target_id'],'workspace':str(self.workspace),'input_text':self.payload}
        for flag in ('--assume-unchanged', '--skip-worktree'):
            with self.subTest(stage='submission', flag=flag):
                self.git('update-index',flag,'a'); (self.workspace/'a').write_text('hidden change\n')
                self.assertEqual(subprocess.check_output(['/usr/bin/git','-C',str(self.workspace),'status','--porcelain']),b'')
                self.assertFalse(self.rpc('submit_session_request',**request)['ok']); self.assertEqual(self.native.starts,0)
                self.git('update-index',flag.replace('--','--no-',1),'a'); (self.workspace/'a').write_text('checkpoint\n')
    def test_close_and_submit_are_serialized(self):
        created=self.create()['result']; grant=self.rpc('issue_session_request_grant',input_text=self.payload)['result']
        request={'request_id':self.request,'grant_handle':grant['grant_handle'],'session_id':created['target_id'],'workspace':str(self.workspace),'input_text':self.payload}
        results=[]
        threads=[threading.Thread(target=lambda:results.append(self.rpc('submit_session_request',**request))),threading.Thread(target=lambda:results.append(self.rpc('close_session_target')))]
        for t in threads:t.start()
        for t in threads:t.join(30); self.assertFalse(t.is_alive())
        self.assertEqual(len(results),2); self.assertLessEqual(self.native.starts,1)
        self.assertFalse(self.rpc('submit_session_request',**request)['ok'])
    def test_initialization_ack_loss_is_not_replayed(self):
        self.native.lost_injection=True
        first=self.create()['result']; self.assertEqual(first['setup_state'],'initialization_uncertain')
        self.assertEqual(first['synthetic_initialization'],'uncertain'); self.assertEqual(self.native.injections,1)
        self.stop_owner(); self.start_owner()
        self.assertEqual(self.create()['result']['setup_state'],'initialization_uncertain')
        self.assertEqual(self.native.injections,1); self.assertEqual(self.native.allocations,1)
        self.assertFalse(self.rpc('issue_session_request_grant',input_text=self.payload)['ok'])
    def test_readable_history_does_not_inject(self):
        self.native.unmaterialized=False
        first=self.create()['result']; self.assertEqual(first['setup_state'],'ready')
        self.assertEqual(first['synthetic_initialization'],'not_needed'); self.assertEqual(self.native.injections,0)
    def test_config_change_blocks_grant(self):
        self.assertEqual(self.create()['result']['setup_state'],'ready')
        self.local.write_text('changed\n')
        self.assertFalse(self.rpc('issue_session_request_grant',input_text=self.payload)['ok'])
        self.assertEqual(self.native.starts,0)
    def test_cross_task_handle_and_revocation(self):
        created=self.create()['result']; grant=self.rpc('issue_session_request_grant',input_text=self.payload)['result']
        other=dict(self.policy, task_revision='fixture-task-rev-2',request_id=str(uuid.uuid4()))
        policy=self.root/'other-policy'; policy.write_text(json.dumps(other)); policy.chmod(0o600)
        credential=self.root/'other-credential'; self.cli('provisioning','issue','--policy-file',str(policy),'--credential-out',str(credential))
        token=credential.read_text().strip()
        self.assertFalse(self.rpc('read_session_operation',token=token,request_id=self.request,grant_handle=grant['grant_handle'])['ok'])
        self.cli('provisioning','revoke','--permission-id',self.permission['permission_id'])
        self.assertFalse(self.rpc('read_session_target')['ok'])
        self.assertFalse(self.rpc('submit_session_request',request_id=self.request,grant_handle=grant['grant_handle'],session_id=created['target_id'],workspace=str(self.workspace),input_text=self.payload)['ok'])
        self.assertEqual(self.rpc('close_session_target')['result']['closure_state'],'closed')
    def test_restart_preserves_close_deadline_for_known_target(self):
        other=dict(self.policy,task_revision='restart-expiry',request_id=str(uuid.uuid4()),admit_until=int(time.time())+4,close_by=int(time.time())+19)
        policy=self.root/'restart-expiry-policy'; policy.write_text(json.dumps(other)); policy.chmod(0o600)
        credential=self.root/'restart-expiry-credential'; self.cli('provisioning','issue','--policy-file',str(policy),'--credential-out',str(credential))
        token=credential.read_text().strip()
        result=self.rpc('create_session_target',token=token,idempotency_key=str(uuid.uuid4()))
        self.assertEqual(result['result']['setup_state'],'ready')
        self.stop_owner(); self.start_owner()
        self.assertEqual(self.rpc('read_session_target',token=token)['result']['setup_state'],'custody_lost')
        until=time.monotonic()+13
        while time.monotonic()<until:
            status=self.rpc('read_session_target',token=token)['result']
            if status['closure_state']=='closed': break
            time.sleep(.1)
        self.assertEqual(status['closure_state'],'closed'); self.assertEqual(self.native.archives,1)
    def test_close_ack_loss_stays_uncertain_without_replay(self):
        self.assertEqual(self.create()['result']['setup_state'],'ready')
        self.native.lost_close=True
        self.assertEqual(self.rpc('close_session_target')['result']['closure_state'],'uncertain')
        observed=self.rpc('close_session_target')['result']
        self.assertEqual(observed['closure_state'],'uncertain')
        self.assertEqual(observed['closure_observation']['status'],'notLoaded')
        self.assertIsNotNone(observed['closure_observation']['observed_at'])
        self.assertEqual(self.native.archives,1)
    def test_operation_receipt_survives_native_history_unavailability(self):
        target=self.create()['result']['target_id']; handle=self.rpc('issue_session_request_grant',input_text=self.payload)['result']['grant_handle']
        request={'request_id':self.request,'grant_handle':handle,'session_id':target,'workspace':str(self.workspace),'input_text':self.payload}
        self.assertEqual(self.rpc('submit_session_request',**request)['result']['entry_state'],'entry_uncertain')
        self.native.loaded=False
        result=self.rpc('read_session_operation',request_id=self.request,grant_handle=handle)['result']
        self.assertEqual(result['entry_state'],'entry_uncertain'); self.assertEqual(result['reconciliation'],'unavailable')
    def test_unloaded_target_is_observed_without_claiming_archive(self):
        self.assertEqual(self.create()['result']['setup_state'],'ready')
        self.native.loaded=False
        read=self.rpc('read_session_target')['result']
        self.assertEqual(read['owner_health'],'custody_unverified'); self.assertEqual(read['setup_state'],'custody_lost')
        closed=self.rpc('close_session_target')['result']
        self.assertEqual(closed['closure_state'],'pending'); self.assertEqual(closed['closure_observation']['status'],'notLoaded')
        self.assertEqual(self.native.archives,0)
    def test_idle_ping_gets_pong_without_a_tool_call(self):
        created=self.create(); self.assertTrue(created['ok'],(created,self.native.methods)); self.assertEqual(created['result']['setup_state'],'ready')
        self.native.send_ping()
        self.assertTrue(self.native.pong.wait(3),'owner did not answer the idle Ping')
        self.assertEqual(self.rpc('read_session_target')['result']['owner_health'],'held')
        self.assertEqual(self.rpc('close_session_target')['result']['closure_state'],'closed')
    def test_stalled_fragment_fences_custody(self):
        self.assertEqual(self.create()['result']['setup_state'],'ready')
        self.native.held.sendall(b'\x81\x05a')
        time.sleep(11)
        read=self.rpc('read_session_target')['result']
        self.assertEqual(read['setup_state'],'custody_lost'); self.assertNotEqual(read['owner_health'],'held')
        self.assertFalse(self.rpc('issue_session_request_grant',input_text=self.payload)['ok'])
    def test_fragment_ping_then_rpc_retains_deadline_and_fences(self):
        self.assertEqual(self.create()['result']['setup_state'],'ready')
        self.native.delay_fragment=True
        # An incomplete notification and interleaved Ping let idle_poll return.
        self.native.held.sendall(b'\x01\x0b{"method":"' + bytes([0x89,len(self.native.ping_payload)]) + self.native.ping_payload)
        self.assertTrue(self.native.pong.wait(3))
        time.sleep(8.5)
        start=time.monotonic()
        read=self.rpc('read_session_target')
        self.assertLess(time.monotonic()-start,2.8)
        self.assertTrue(read['ok'],read)
        self.assertEqual(read['result']['setup_state'],'custody_lost')
        self.assertNotEqual(read['result']['owner_health'],'held')
        self.assertFalse(self.rpc('issue_session_request_grant',input_text=self.payload)['ok'])
    def test_idle_beyond_single_rpc_deadline_keeps_custody(self):
        self.assertEqual(self.create()['result']['setup_state'],'ready')
        time.sleep(11)
        read=self.rpc('read_session_target')['result']
        self.assertEqual(read['owner_health'],'held'); self.assertEqual(read['setup_state'],'ready')
        self.assertEqual(self.rpc('close_session_target')['result']['closure_state'],'closed')
    def test_concurrent_conflicting_keys_only_allocate_once(self):
        results=[]; keys=[self.key,str(uuid.uuid4())]
        threads=[threading.Thread(target=lambda k=k: results.append(self.rpc('create_session_target',idempotency_key=k))) for k in keys]
        for t in threads: t.start()
        for t in threads: t.join(30); self.assertFalse(t.is_alive())
        self.assertEqual(sum(bool(x['ok']) for x in results),1)
        self.assertEqual(self.native.allocations,1)
    def test_expiry_retains_read_and_cleanup(self):
        other=dict(self.policy, task_revision='expiry-fixture',request_id=str(uuid.uuid4()),admit_until=int(time.time())+2,close_by=int(time.time())+40)
        policy=self.root/'expiry-policy'; policy.write_text(json.dumps(other)); policy.chmod(0o600)
        credential=self.root/'expiry-credential'; self.cli('provisioning','issue','--policy-file',str(policy),'--credential-out',str(credential))
        token=credential.read_text().strip(); time.sleep(3)
        self.assertFalse(self.rpc('create_session_target',token=token,idempotency_key=str(uuid.uuid4()))['ok'])
        self.assertTrue(self.rpc('read_session_target',token=token)['ok'])
        self.assertEqual(self.rpc('close_session_target',token=token)['result']['closure_state'],'closed')
    def test_expired_admission_keeps_operation_read_and_cleanup(self):
        req=str(uuid.uuid4()); other=dict(self.policy,task_revision='expiry-after-entry',request_id=req,admit_until=int(time.time())+5,close_by=int(time.time())+40)
        policy=self.root/'expiry-entered-policy'; policy.write_text(json.dumps(other)); policy.chmod(0o600)
        credential=self.root/'expiry-entered-credential'; self.cli('provisioning','issue','--policy-file',str(policy),'--credential-out',str(credential))
        token=credential.read_text().strip()
        target=self.rpc('create_session_target',token=token,idempotency_key=str(uuid.uuid4()))['result']['target_id']
        grant=self.rpc('issue_session_request_grant',token=token,input_text=self.payload)['result']['grant_handle']
        request={'request_id':req,'grant_handle':grant,'session_id':target,'workspace':str(self.workspace),'input_text':self.payload}
        self.assertTrue(self.rpc('submit_session_request',token=token,**request)['ok'])
        time.sleep(max(0,other['admit_until']-time.time()+.2))
        self.assertFalse(self.rpc('submit_session_request',token=token,**request)['ok'])
        read=self.rpc('read_session_operation',token=token,request_id=req,grant_handle=grant)
        self.assertTrue(read['ok'],read)
        self.assertEqual(self.rpc('close_session_target',token=token)['result']['closure_state'],'closed')
    def test_opt_in_catalog_and_mixed_form(self):
        from test_session_events import Receiver
        binary=self.root/'bridge'; shutil.copyfile(BINARY,binary); binary.chmod(0o700)
        bridge=SE.Bridge(str(binary),self.hash(binary),str(self.config),self.hash(self.config),provisioning=True)
        homes=SE.Homes(self.root/'codex',self.root/'claude',self.root/'events')
        server=SE.Server(homes,'tunnel',SE.CallbackPolicy(),Receiver(),bridge=bridge)
        names=[t['name'] for t in SE.PROVISIONED_BRIDGE_TOOLS+SE.PROVISION_TOOLS]
        self.assertEqual(len(names),7); self.assertIn('create_session_target',names)
        result=server.call_tool({'name':'create_session_target','arguments':{'provisioning_credential':self.token,'idempotency_key':self.key}})
        self.assertEqual(result['structuredContent']['setup_state'],'ready')
        mixed=server.call_tool({'name':'read_session_operation','arguments':{'request_id':self.request,'grant_credential':'bad','provisioning_credential':self.token,'grant_handle':str(uuid.uuid4())}})
        self.assertTrue(mixed['isError']); self.assertNotIn(self.token,json.dumps(mixed))
        for extra in ({'provisioning_credential':self.token},{'grant_handle':str(uuid.uuid4())}):
            partial=server.call_tool({'name':'read_session_operation','arguments':{'request_id':self.request,'grant_credential':'bad',**extra}})
            self.assertTrue(partial['isError']); self.assertNotIn(self.token,json.dumps(partial))
    def test_close_before_create_and_redaction(self):
        self.assertEqual(self.rpc('close_session_target')['result']['closure_state'],'closed')
        self.assertFalse(self.create()['ok']); self.assertEqual(self.native.allocations,0)
        self.assertNotIn(self.token,SE.redact('credential '+self.token)[0])
