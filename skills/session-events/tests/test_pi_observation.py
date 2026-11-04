"""Offline MCP -> actual adapter ELF -> fake Host/native evidence -> Store ACK.
No Pi, Docker daemon, provider, credentials or network is used by these fixtures.
"""
import concurrent.futures
import hashlib
import io
import email.message
import subprocess
import types
import json
import os
from pathlib import Path
import shutil
import tempfile
import time
import unittest
from unittest import mock
from test_session_events import SE

REPO = Path(__file__).resolve().parents[3]
BIN = REPO / 'rust/target/debug/examples/mcp_pi_observation'
NATIVE = REPO / 'rust/target/debug/examples/native_ack_fixture'
BASE = REPO / 'rust/target/debug/examples/pi_docker_fixture'
READER = Path('/home/bline/.local/state/work-engine/tools/context-status/d16c5f3799e04da374adc5595d9643354d64457ebca3cf3468cf4566824f1204/work-engine-context-status')
SCHEMA = 'work-engine.mcp-pi-observation.v1'

def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()

def artifact(path):
    return dict(path=str(path), sha256=sha(path))

def write(path, value):
    path.write_text(json.dumps(value, separators=(',', ':')))
    path.chmod(0o600)
    return artifact(path)

class Observation(unittest.TestCase):
    def setUp(self):
        root = REPO / '.mcp-pi-evidence'
        root.mkdir(exist_ok=True, mode=0o700)
        retained = os.environ.get('PI_OBSERVATION_TEST_KEEP_ROOT') if self._testMethodName == 'test_positive_exact_result_restart_duplicate_and_reads' else None
        if retained:
            self.root = Path(retained).resolve()
            self.root.mkdir(mode=0o700)
        else:
            self.root = Path(tempfile.mkdtemp(prefix='offline-', dir=root)).resolve()
            self.addCleanup(shutil.rmtree, self.root, True)
        p = self.root
        for name in ('runtime', 'workspace', 'native', 'operation'):
            (p/name).mkdir(mode=0o700)
        sdk = p/'runtime/node_modules/@earendil-works/pi-coding-agent'
        (sdk/'dist/bundle').mkdir(parents=True)
        (sdk/'dist/bundle/cli.js').write_text('fake never executed')
        (sdk/'dist/index.js').write_text('fake never executed')
        write(sdk/'package.json', dict(name='@earendil-works/pi-coding-agent', version='1.5.0'))
        runtime = {str(f.relative_to(p/'runtime')):dict(kind='file',sha256=sha(f))
                   for f in sorted((p/'runtime').rglob('*')) if f.is_file()}
        exe = p/'fake-docker'
        shutil.copy2(NATIVE, exe)
        shutil.copy2(BASE, exe.with_suffix('.base'))
        exe.with_suffix('.mode').write_text('observe')
        self.calls = exe.with_suffix('.calls')
        deadline = int(time.time()) + 25
        recipient = dict(recipient='target', principal='tester', job='job', lane='lane',
                         workspace=str(p/'workspace'), generation=1, address='role:target',
                         fixture_mode='observe', expires_at=deadline+20, active=True)
        rollout = p/'synthetic.jsonl'
        sid = '11111111-1111-4111-8111-111111111111'
        rollout.write_text(json.dumps(dict(type='session_meta', payload=dict(id=sid)))+'\n'+
            json.dumps(dict(type='event_msg', timestamp='2026-11-04T00:20:00Z',payload=dict(type='token_count',info=dict(last_token_usage=dict(input_tokens=700,cached_input_tokens=0),model_context_window=1000))))+'\n')
        cfg = dict(schema='session-delivery-fixture-config-v1',fixture_only=True,database=str(p/'store.sqlite'),
                   repository=str(p), repository_identity='synthetic', source_id='unused-board',board_reader='/unused/board',source_adapter='/unused/adapter',targets=[],
                   pressure=dict(reader=str(READER),reader_sha256=sha(READER),incarnation='synthetic',session=sid,host='codex',agent=None,rollout=str(rollout),
                     levels=dict(window_percent=dict(trigger=58.0,band_floor=66.0)),step_percent=5.0,lead_calls=3,recipients=['target']))
        event = p/'event.json'
        job = dict(schema='work-engine.pi-docker-custody.v1',job_id='job',state_dir=str(p/'native'),
                   authority=write(p/'authority.json',dict(fixture_only=True)),packet=write(p/'packet.json',dict(fixture_only=True)),docker=artifact(exe),
                   image='node@sha256:4b150b48b4116e2a22f2c39cc27025fbf4f153e2078fd3b14d159fdb5c7279da',
                   runtime=dict(root=str(p/'runtime'),manifest=write(p/'runtime.json',runtime)),
                   workspace=dict(root=str(p/'workspace'),manifest=write(p/'workspace.json',{})),
                   deadline_unix_seconds=deadline,command_timeout_seconds=3,idle_seconds=0,
                   receiver=dict(schema='work-engine.pi-observation-sdk.v1',shim=artifact(REPO/'rust/crates/work-engine-manual-host/receiver/pi-observation.mjs'),
                                 event=dict(path=str(event),sha256='0'*64),attempt_number=1,recipient_without_session=recipient))
        startup = dict(config=cfg,event_path=str(event),ready_path=str(p/'native/receiver-ready.json'),envelope_path=str(p/'envelope.json'),
                       release_path=str(p/'native/receiver-release.json'),input_digest='',container_id='',ready_sha256='',job='job',deadline_unix_seconds=deadline,recipient=recipient)
        self.cfg = dict(schema=SCHEMA,principal='tester',subject='fixed-subject',target='target',handle='one-observation',revision='r1',idempotency_key='one-shot',
                        expires_at=deadline+20,revoked_path=str(p/'native/cancel.json'),operation_dir=str(p/'operation'),job=job,startup=startup)
        self.binary = p/'adapter'
        shutil.copy2(BIN,self.binary); self.binary.chmod(0o700)
        self.config = p/'config.json'
        self.args = dict(handle='one-observation',revision='r1',idempotency_key='one-shot')
        self.save()
        homes = SE.Homes(p/'codex',p/'claude',p/'events')
        self.server = SE.Server(homes,'tester',SE.CallbackPolicy(),observation=self.helper())

    def save(self):
        write(self.config,self.cfg)

    def helper(self):
        return SE.ObservationHelper(str(self.binary),sha(self.binary),str(self.config),sha(self.config))

    def call(self, method, args=None):
        return self.server.handle(dict(jsonrpc='2.0',id=1,method='tools/call',params=dict(name='pi_observation_'+method,arguments=self.args if args is None else args)))['result']

    def value(self, method):
        result = self.call(method)
        self.assertNotIn('isError',result, result)
        return result['structuredContent']['observation']

    def test_positive_exact_result_restart_duplicate_and_reads(self):
        self.assertEqual(self.value('result')['state'],'not_started')
        self.assertFalse(self.calls.exists())
        v = self.value('observe')
        self.assertEqual(v['state'],'target_observed',v)
        self.assertEqual(v['native_session_id'],'fixture-native-session')
        self.assertTrue(v['target_observed'])
        self.assertIsNone(v['application_result'])
        self.assertIsNone(v['completion'])
        self.assertTrue(v['fixture_only'])
        self.assertGreaterEqual(len(v['evidence']),6)
        write(self.root/'mcp-request.json', dict(jsonrpc='2.0',id=1,method='tools/call',params=dict(name='pi_observation_observe',arguments=self.args)))
        write(self.root/'mcp-result.json', v)
        before = self.calls.read_bytes()
        self.server.observation = self.helper() # fresh process for every helper call
        self.assertEqual(self.value('result'),v)
        self.assertEqual(self.value('status'),v)
        self.assertEqual(self.value('observe'),v)
        self.assertEqual(self.calls.read_bytes(),before)
        # Any changed immutable startup config conflicts, even on the same handle.
        original = self.config.read_bytes()
        self.cfg['startup']['recipient']['lane']='changed'
        self.cfg['job']['receiver']['recipient_without_session']['lane']='changed'
        self.save(); self.server.observation=self.helper()
        self.assertTrue(self.call('observe')['isError'])
        self.assertEqual(self.calls.read_bytes(),before)
        self.config.write_bytes(original)

    def test_authority_injection_and_unknown_calls_never_launch(self):
        for k in ('principal','target','path','binary','database','payload','session','ack'):
            self.assertTrue(self.call('observe',self.args|{k:'caller'})['isError'])
        for k in self.args:
            self.assertTrue(self.call('observe',self.args|{k:'changed'})['isError'])
        self.server.principal='other'
        self.assertTrue(self.call('observe')['isError'])
        self.assertFalse(self.calls.exists())

    def test_expired_revoked_wrong_target(self):
        deadline=self.cfg['job']['deadline_unix_seconds']
        self.cfg['expires_at']=int(time.time())-1
        self.cfg['job']['deadline_unix_seconds']=int(time.time())-2
        self.cfg['startup']['deadline_unix_seconds']=self.cfg['job']['deadline_unix_seconds']
        self.save(); self.server.observation=self.helper()
        expired=self.call('observe')
        self.assertTrue(expired['isError'])
        self.assertIn('grant_expired_or_revoked',expired['content'][0]['text'])
        self.cfg['job']['deadline_unix_seconds']=deadline
        self.cfg['startup']['deadline_unix_seconds']=deadline
        self.cfg['expires_at']=deadline+20
        self.save(); self.server.observation=self.helper()
        (self.root/'native/cancel.json').touch()
        self.assertTrue(self.call('observe')['isError'])
        (self.root/'native/cancel.json').unlink()
        self.cfg['target']='wrong'
        self.save(); self.server.observation=self.helper()
        self.assertTrue(self.call('observe')['isError'])
        self.assertFalse(self.calls.exists())

    def test_concurrent_duplicate_single_effect(self):
        with concurrent.futures.ThreadPoolExecutor(max_workers=8) as pool:
            results=list(pool.map(lambda _:self.call('observe'),range(8)))
        self.assertEqual(self.value('result')['state'],'target_observed',results)
        self.assertEqual(self.calls.read_text().splitlines().count('start'),1)
        before=self.calls.read_bytes()
        self.assertTrue(self.call('observe',self.args|dict(idempotency_key='changed'))['isError'])
        self.assertEqual(before,self.calls.read_bytes())

    def test_native_failure_unknown_and_restart_never_resends(self):
        (self.root/'fake-docker.mode').write_text('missing-history')
        v=self.value('observe')
        self.assertEqual(v['state'],'unknown')
        self.assertIsNone(v['target_observed'])
        before=self.calls.read_bytes()
        self.server.observation=self.helper()
        self.assertEqual(self.value('observe'),v)
        self.assertEqual(self.value('result'),v)
        self.assertEqual(before,self.calls.read_bytes())

    def test_lost_terminal_ack_no_resend(self):
        self.assertEqual(self.value('observe')['state'],'target_observed')
        (self.root/'operation/result.json').unlink() # simulate lost response/terminal projection
        before=self.calls.read_bytes()
        self.assertEqual(self.value('result')['state'],'unknown')
        self.assertEqual(self.value('observe')['state'],'unknown')
        self.assertEqual(before,self.calls.read_bytes())

    def test_revocation_during_create_prevents_start_and_release(self):
        (self.root/'fake-docker.mode').write_text('revoke-on-create')
        v=self.value('observe')
        self.assertEqual(v['state'],'unknown')
        calls=self.calls.read_text().splitlines()
        self.assertIn('create',calls)
        self.assertNotIn('start',calls)
        self.assertTrue((self.root/'native/cancel.json').exists())
        self.assertFalse((self.root/'native/receiver-release.json').exists())
        journal=json.loads((self.root/'native/pi-state.json').read_text())
        self.assertTrue(journal['removed'])
        self.assertTrue(all(p['status'] is not None and not p['leftovers'] for p in journal['processes']))
        before=self.calls.read_bytes()
        self.assertEqual(self.value('result')['state'],'unknown')
        self.assertEqual(before,self.calls.read_bytes())

    def test_unbound_revocation_path_is_rejected(self):
        self.cfg['revoked_path']=str(self.root/'other-marker')
        self.save(); self.server.observation=self.helper()
        self.assertTrue(self.call('observe')['isError'])
        self.assertFalse(self.calls.exists())

    def test_result_binding_and_success_coherence(self):
        self.assertEqual(self.value('observe')['state'],'target_observed')
        path=self.root/'operation/result.json'
        path.chmod(0o600) # owner-only tamper simulation; production publication is read-only
        original=json.loads(path.read_text())
        before=self.calls.read_bytes()
        mutations=[dict(input_binding_sha256='0'*64), dict(evidence=[]),
                   dict(native_session_id='other-session'),dict(native_receipt_id='0'*64),
                   dict(store_ack=original['store_ack']|dict(stage='accepted')),
                   dict(observation_ack=original['observation_ack']|dict(event_id='0'*64)),
                   dict(state='unknown')]
        for change in mutations:
            write(path,original|change)
            self.assertTrue(self.call('result')['isError'],change)
            self.assertEqual(before,self.calls.read_bytes())
        write(path,original)
        self.assertEqual(self.value('result')['state'],'target_observed')

    def test_swapped_result_is_rejected_even_with_relabelled_binding(self):
        self.assertEqual(self.value('observe')['state'],'target_observed')
        other=Observation('test_authority_injection_and_unknown_calls_never_launch')
        other.setUp()
        self.addCleanup(other.doCleanups)
        self.assertEqual(other.value('observe')['state'],'target_observed')
        path=self.root/'operation/result.json'
        path.chmod(0o600) # owner-only tamper simulation; production publication is read-only
        own=json.loads(path.read_text())
        foreign=json.loads((other.root/'operation/result.json').read_text())
        before=self.calls.read_bytes(); foreign_before=other.calls.read_bytes()
        for relabel in (False,True):
            if relabel: foreign['input_binding_sha256']=own['input_binding_sha256']
            write(path,foreign)
            self.assertTrue(self.call('result')['isError'])
            self.assertEqual(before,self.calls.read_bytes())
            self.assertEqual(foreign_before,other.calls.read_bytes())
        write(path,own)

    def test_config_pin_changed_rejected(self):
        self.config.write_text(self.config.read_text()+' ')
        self.assertTrue(self.call('observe')['isError'])
        self.assertFalse(self.calls.exists())

    def test_http_authorization_to_exact_fake_result_without_sockets(self):
        from test_http_auth import AuthHttp, b64, SUBJECT, RESOURCE, ISSUER, VERIFIER
        self.key=self.root/'key.pem'
        subprocess.run(['openssl','genpkey','-quiet','-algorithm','RSA','-pkeyopt','rsa_keygen_bits:2048','-out',str(self.key)],check=True,stderr=subprocess.DEVNULL)
        modulus=subprocess.check_output(['openssl','rsa','-in',str(self.key),'-noout','-modulus'],stderr=subprocess.DEVNULL).decode().strip().split('=')[1]
        write(self.root/'jwks.json', {'keys':[dict(kty='RSA',kid='one',use='sig',alg='RS256',n=b64(bytes.fromhex(modulus)),e='AQAB')]})
        auth_path=self.root/'auth.json'
        write(auth_path,dict(resource=RESOURCE,issuer=ISSUER,subject=SUBJECT,write_scope='mcp:write',jwks_snapshot=str(self.root/'jwks.json'),jwks_max_age_seconds=300,verifier=str(VERIFIER)))
        self.cfg['subject']=SUBJECT
        self.save(); self.server.observation=self.helper()
        auth=SE.HttpAuth(str(auth_path))
        handler=SE.make_http_handler(self.server,'/mcp',auth,observation_only=True)
        def request(message, token=None):
            h=handler.__new__(handler)
            h.headers=email.message.Message()
            h.headers['Host']='127.0.0.1:8787'
            h.headers['Content-Type']='application/json'
            raw=json.dumps(message).encode()
            h.headers['Content-Length']=str(len(raw))
            if token is not None: h.headers['Authorization']='Bearer '+token
            h.path='/mcp'; h.command='POST'
            h.server=types.SimpleNamespace(server_address=('127.0.0.1',8787))
            h.rfile=io.BytesIO(raw)
            out=[]
            h._reply=lambda status, body=b'', *args, **kw: out.append((status,json.loads(body) if body else None))
            h.do_POST()
            return out[0]
        msg=dict(jsonrpc='2.0',id=1,method='tools/call',params=dict(name='pi_observation_observe',arguments=self.args))
        token=lambda overrides=None: AuthHttp.token(self,overrides)
        for bearer,status in ((None,401),(token({'sub':'different-subject'}),401),
                (token({'exp':int(time.time())-1}),401),(token({'scope':'openid'}),403)):
            self.assertEqual(request(msg,bearer)[0],status)
        self.assertFalse(self.calls.exists())
        malformed=msg|dict(params=dict(name=[],arguments={}))
        self.assertEqual(request(malformed,token())[0],403)
        good=token()
        status,body=request(msg,good)
        self.assertEqual(status,200)
        self.assertEqual(body['result']['structuredContent']['observation']['state'],'target_observed',body)
        before=self.calls.read_bytes()
        for name in ('submit_session_request','create_session_target','issue_session_request_grant','delivery_ack'):
            denied=msg|dict(params=dict(name=name,arguments={}))
            self.assertEqual(request(denied,good)[0],403)
        for method in ('events/subscribe','events/unsubscribe'):
            self.assertEqual(request(dict(jsonrpc='2.0',id=1,method=method),good)[0],403)
        status,listing=request(dict(jsonrpc='2.0',id=1,method='tools/list'),good)
        self.assertEqual(status,200)
        tools=listing['result']['tools']
        self.assertEqual({t['name'] for t in tools},{'pi_observation_observe','pi_observation_status','pi_observation_result','delivery_status','replay_events'})
        self.assertFalse(next(t for t in tools if t['name']=='pi_observation_observe')['annotations']['readOnlyHint'])
        read=msg|dict(params=dict(name='pi_observation_result',arguments=self.args))
        self.assertEqual(request(read,good)[1]['result']['structuredContent']['observation']['state'],'target_observed')
        self.assertEqual(before,self.calls.read_bytes())

    def test_default_disabled_and_http_binding(self):
        with mock.patch.dict(os.environ,{},clear=True):
            self.assertIsNone(SE.ObservationHelper.from_environment())
        with mock.patch.dict(os.environ,{'PI_OBSERVATION_ENABLED':'1'},clear=True):
            self.assertRaises(ValueError,SE.ObservationHelper.from_environment)
        auth=type('Auth',(),dict(config=dict(subject='fixed-subject')))()
        for args in ({},{'auth':auth},{'auth':auth,'authenticated_read_only':True},{'observation_only':True}):
            with self.assertRaises(ValueError): SE.make_http_handler(self.server,'/mcp',**args)
        SE.make_http_handler(self.server,'/mcp',auth,observation_only=True)
        auth.config['subject']='other'
        with self.assertRaises(ValueError): SE.make_http_handler(self.server,'/mcp',auth,observation_only=True)

if __name__=='__main__': unittest.main()
