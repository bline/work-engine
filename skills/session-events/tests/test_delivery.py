"""Offline opt-in delivery facade: pinned fake helper and real Rust owner."""
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest
from unittest import mock
from test_session_events import SE

REPO = Path(__file__).resolve().parents[3]
BINARY = os.environ.get('SESSION_DELIVERY_TEST_BINARY')


class DeliveryFacade(unittest.TestCase):
    def setUp(self):
        # Bridge's existing pin checks intentionally reject world-writable ancestors.
        base = REPO / '.delivery-evidence'
        base.mkdir(mode=0o700, exist_ok=True)
        self.root = Path(tempfile.mkdtemp(prefix='facade-', dir=base))
        self.addCleanup(shutil.rmtree, self.root, True)
        self.config = self.root / 'config.json'
        self.targets = [dict(recipient='fixture', principal='tester', job='job', lane='lane',
                             session='fixture-session', workspace=str(self.root), generation=1,
                             address='role:fixture', fixture_mode='lost_ack',
                             expires_at=4102444800, active=True)]
        self.cfg = dict(schema='session-delivery-fixture-config-v1', fixture_only=True,
                        database=str(self.root/'store.sqlite'), repository=str(self.root/'board'),
                        repository_identity=str(self.root/'board/.git'), source_id='fixture-board',
                        board_reader=str(REPO/'skills/durable-state/scripts/codex_chatboard.py'),
                        source_adapter=str(REPO/'rust/crates/session-delivery/scripts/board_source.py'),
                        targets=self.targets)
        self.config.write_text(json.dumps(self.cfg)); self.config.chmod(0o600)
        self.homes = SE.Homes(self.root/'codex', self.root/'claude', self.root/'events')

    def helper(self, binary):
        return SE.DeliveryHelper(str(binary), hashlib.sha256(binary.read_bytes()).hexdigest(),
                                 str(self.config), hashlib.sha256(self.config.read_bytes()).hexdigest())

    def call(self, server, name, arguments):
        return server.handle(dict(jsonrpc='2.0', id=1, method='tools/call',
                                 params=dict(name=name, arguments=arguments)))['result']

    def test_default_disabled_and_fake_configured_helper(self):
        server = SE.Server(self.homes, 'tester', SE.CallbackPolicy())
        names = [t['name'] for t in server.handle(dict(jsonrpc='2.0', id=1, method='tools/list'))['result']['tools']]
        self.assertIn('delivery_status', names)
        self.assertNotIn('delivery_subscribe', names)
        fake = self.root/'helper'
        fake.write_text('''#!/usr/bin/env python3
import json,sys
v=json.load(sys.stdin)
assert v['principal']=='tester' and v['recipient']=='fixture'
print(json.dumps(dict(schema_version=1,protocol='session-delivery-rpc-v1',ok=True,result=dict(schema_version=1,method=v['method'],fixture_only=True))))
'''); fake.chmod(0o700)
        server = SE.Server(self.homes, 'tester', SE.CallbackPolicy(), delivery=self.helper(fake))
        names = [t['name'] for t in server.handle(dict(jsonrpc='2.0', id=1, method='tools/list'))['result']['tools']]
        self.assertTrue({'delivery_subscribe','delivery_inspect','delivery_ack','delivery_status'} <= set(names))
        for name in ('delivery_subscribe','delivery_inspect'):
            self.assertFalse(self.call(server,name,dict(recipient='fixture')).get('isError',False))
        a=dict(schema='work-engine.delivery-observation/v1',attempt_id='a'*64,delivery_id='b'*64,
               event_id='c'*64,binding_sha256='d'*64,envelope_sha256='e'*64,payload_sha256='f'*64,
               generation=1,stage='target_observed')
        self.assertFalse(self.call(server,'delivery_ack',dict(recipient='fixture',ack=a)).get('isError',False))
        for extra in ('principal','binary','database','source_adapter'):
            self.assertTrue(self.call(server,'delivery_subscribe',dict(recipient='fixture',**{extra:'caller'}))['isError'])
        self.assertTrue(self.call(server,'delivery_subscribe',dict(recipient='other'))['isError'])
        other = SE.Server(self.homes, 'intruder', SE.CallbackPolicy(), delivery=self.helper(fake))
        self.assertTrue(self.call(other,'delivery_subscribe',dict(recipient='fixture'))['isError'])
        a['stage']='completed'
        self.assertTrue(self.call(server,'delivery_ack',dict(recipient='fixture',ack=a))['isError'])
        fake.write_text(fake.read_text()+'# changed\n')
        self.assertTrue(self.call(server,'delivery_inspect',dict(recipient='fixture'))['isError'])

    def test_explicit_switch_required(self):
        with mock.patch.dict(os.environ, {}, clear=True):
            self.assertIsNone(SE.DeliveryHelper.from_environment())
        with mock.patch.dict(os.environ, {'SESSION_DELIVERY_BINARY':'/caller'}, clear=True):
            with self.assertRaises(ValueError): SE.DeliveryHelper.from_environment()

    @unittest.skipUnless(BINARY, 'requires custody-built Rust helper')
    def test_joined_rust_owner_board_receiver_and_ack(self):
        binary = self.root/'rust-helper'; shutil.copy2(BINARY,binary);binary.chmod(0o700)
        server = SE.Server(self.homes,'tester',SE.CallbackPolicy(),delivery=self.helper(binary))
        self.assertFalse(self.call(server,'delivery_subscribe',dict(recipient='fixture')).get('isError',False))
        subprocess.run(['git','init','-q',self.cfg['repository']],check=True)
        env={**os.environ, 'GIT_AUTHOR_NAME':'fixture','GIT_COMMITTER_NAME':'fixture',
             'GIT_AUTHOR_EMAIL':'fixture@example.invalid','GIT_COMMITTER_EMAIL':'fixture@example.invalid'}
        p=subprocess.run(['python3',self.cfg['board_reader'],'--repository',self.cfg['repository'],
                          'post','--author','fixture','--session-id','11111111-1111-4111-8111-111111111111',
                          '--topic','fixture','--body','MCP joined nonce fixture','--to','role:fixture'],
                         env=env,check=True,capture_output=True)
        self.assertIn('message',json.loads(p.stdout))
        subprocess.run([binary,'--config',self.config,'observe'],check=True)
        subprocess.run([binary,'--config',self.config,'dispatch'],check=True)
        status=self.call(server,'delivery_inspect',dict(recipient='fixture'))['structuredContent']
        d=status['deliveries'][0];self.assertEqual(d['stage'],'unknown')
        a={k:d[k] for k in ('attempt_id','delivery_id','event_id','binding_sha256','envelope_sha256','payload_sha256','generation')}
        a.update(schema='work-engine.delivery-observation/v1',stage='target_observed')
        self.assertFalse(self.call(server,'delivery_ack',dict(recipient='fixture',ack=a)).get('isError',False))
        self.assertTrue(self.call(server,'delivery_ack',dict(recipient='fixture',ack=a))['structuredContent']['duplicate'])
        a['attempt_id']='0'*64
        self.assertTrue(self.call(server,'delivery_ack',dict(recipient='fixture',ack=a))['isError'])
        final=self.call(server,'delivery_inspect',dict(recipient='fixture'))['structuredContent']['deliveries'][0]
        self.assertTrue(final['target_observed']);self.assertFalse(final['transport_accepted'])
        self.assertIsNone(final['application_result'])
        if path := os.environ.get('SESSION_DELIVERY_MCP_EVIDENCE'):
            Path(path).write_text(json.dumps(dict(fixture_only=True,
                binary_sha256=hashlib.sha256(binary.read_bytes()).hexdigest(),
                config_sha256=hashlib.sha256(self.config.read_bytes()).hexdigest(),
                unknown_before_ack=status, final_observation=final,
                wrong_attempt_rejected=True, duplicate_ack_idempotent=True), indent=2))

    @unittest.skipUnless(BINARY and os.environ.get('PRESSURE_TEST_READER'), 'requires pinned actual pressure reader and Rust owner')
    def test_joined_pressure_reader_status_and_attempt_ack(self):
        binary = self.root/'rust-helper'; shutil.copy2(BINARY,binary);binary.chmod(0o700)
        reader = Path(os.environ['PRESSURE_TEST_READER'])
        self.assertEqual(hashlib.sha256(reader.read_bytes()).hexdigest(), os.environ['PRESSURE_TEST_READER_SHA256'])
        session = '11111111-1111-4111-8111-111111111111'
        rollout = self.root/'synthetic-rollout.jsonl'
        rows = [dict(type='session_meta',payload=dict(id=session)),
                dict(type='event_msg',timestamp='2026-11-04T00:20:00Z',payload=dict(type='token_count',info=dict(
                    last_token_usage=dict(input_tokens=700,cached_input_tokens=0),model_context_window=1000)))]
        rollout.write_text(''.join(json.dumps(r)+'\n' for r in rows))
        self.cfg['pressure'] = dict(reader=str(reader), reader_sha256=os.environ['PRESSURE_TEST_READER_SHA256'],
            incarnation='disposable-mcp-pressure', session=session, host='codex', agent=None,
            rollout=str(rollout),levels=dict(window_percent=dict(trigger=58.0,band_floor=66.0)),
            step_percent=5.0,lead_calls=3,recipients=['fixture'])
        self.config.write_text(json.dumps(self.cfg))
        server = SE.Server(self.homes,'tester',SE.CallbackPolicy(),delivery=self.helper(binary))
        self.assertFalse(self.call(server,'delivery_subscribe',dict(recipient='fixture')).get('isError',False))
        subprocess.run([binary,'--config',self.config,'observe-pressure'],check=True)
        subprocess.run([binary,'--config',self.config,'dispatch'],check=True)
        status=self.call(server,'delivery_inspect',dict(recipient='fixture'))['structuredContent']
        self.assertTrue(status['pressure']['cursor_committed']);self.assertIsNone(status['pressure']['hold'])
        self.assertNotIn('rollout',json.dumps(status));self.assertNotIn(str(rollout),json.dumps(status))
        d=status['deliveries'][0];self.assertEqual(d['stage'],'unknown')
        a={k:d[k] for k in ('attempt_id','delivery_id','event_id','binding_sha256','envelope_sha256','payload_sha256','generation')}
        a.update(schema='work-engine.delivery-observation/v1',stage='target_observed')
        wrong=dict(a,attempt_id='0'*64)
        self.assertTrue(self.call(server,'delivery_ack',dict(recipient='fixture',ack=wrong))['isError'])
        self.assertFalse(self.call(server,'delivery_ack',dict(recipient='fixture',ack=a)).get('isError',False))
        self.assertTrue(self.call(server,'delivery_ack',dict(recipient='fixture',ack=a))['structuredContent']['duplicate'])
        final=self.call(server,'delivery_inspect',dict(recipient='fixture'))['structuredContent']['deliveries'][0]
        self.assertTrue(final['target_observed']);self.assertIsNone(final['application_result'])
