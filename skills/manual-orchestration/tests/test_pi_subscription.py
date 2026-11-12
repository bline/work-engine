"""Subscription transport: synthetic credentials and local upstream only."""
import base64
import json
import pathlib
import subprocess
import socket
import socketserver
import threading
import struct
from types import SimpleNamespace
import sys
import time
import unittest
from unittest.mock import patch

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parents[1] / 'scripts'))
import pi_container as pc
import role_host as rh
import test_pi_container as base
from test_pi_container import ProxyHarness, Upstream, UnixHTTP, sample_env, load_ledger


def token(**changes):
    claims = {'iss': 'https://auth.openai.com', 'exp': time.time() + 3600,
              'https://api.openai.com/auth': {'chatgpt_account_id': 'test-host-account'}}
    claims.update(changes)
    return 'e30.' + base64.urlsafe_b64encode(json.dumps(claims).encode()).decode().rstrip('=') + '.fake'


class SubscriptionTests(ProxyHarness):
    raw = base.EgressProxyTests.raw
    def proxy(self, **kw):
        p = pc.EgressProxy(str(self.tmp / 'egress.sock'),
                           upstream=f'http://127.0.0.1:{self.upstream.server_port}/backend-api/codex',
                           key=token(), model_id='gpt-6.1-sol', tier=None,
                           log_path=str(self.tmp / 'proxy.jsonl'), provider='openai-codex', **kw).start()
        self.addCleanup(p.stop)
        return p

    def post(self, body, **kw):
        return super().post(body, path='/codex/responses', **kw)

    def body(self):
        return {'model': 'gpt-6.1-sol', 'input': [], 'stream': True, 'store': False}

    def test_route_host_credential_and_observation(self):
        p = self.proxy(max_requests=1)
        self.assertEqual(self.post(self.body())[0], 200)
        sent = Upstream.seen[-1]
        self.assertEqual(sent['path'], '/backend-api/codex/responses')
        self.assertEqual(sent['auth'], 'Bearer ' + p.key)
        headers = {k.lower(): v for k, v in sent['headers'].items()}
        self.assertEqual(headers['chatgpt-account-id'], 'test-host-account')
        self.assertEqual(headers['originator'], 'pi')
        self.assertEqual(headers['openai-beta'], 'responses=experimental')
        self.assertNotIn('content-encoding', headers)
        self.assertNotIn('service_tier', sent['body'])
        row = self.log()[-1]
        self.assertEqual((row['auth_lane'], row['provider'], row['billing'], row['quota']),
                         ('subscription', 'openai-codex', 'unknown', 'unknown'))
        self.assertNotIn(p.key, (self.tmp / 'proxy.jsonl').read_text())
        self.assertEqual(self.post(self.body())[0], 429)

    def test_compressed_request_and_both_size_bounds(self):
        import zstandard
        self.proxy(max_body=1024)
        raw = zstandard.ZstdCompressor().compress(json.dumps(self.body()).encode())
        conn = UnixHTTP(str(self.tmp / 'egress.sock'))
        conn.request('POST', '/codex/responses', raw, {'content-encoding': 'zstd'})
        resp = conn.getresponse()
        self.assertEqual(resp.status, 200)
        resp.read(); conn.close()
        large = zstandard.ZstdCompressor().compress(json.dumps({**self.body(), 'instructions': 'a' * 4096}).encode())
        conn = UnixHTTP(str(self.tmp / 'egress.sock'))
        conn.request('POST', '/codex/responses', large, {'content-encoding': 'zstd'})
        resp = conn.getresponse(); self.assertEqual(resp.status, 400); resp.read(); conn.close()
        self.assertIn(b' 413 ', self.raw(b'POST /codex/responses HTTP/1.1\r\ncontent-length: 1025\r\n\r\n'))
        self.assertEqual(len(Upstream.seen), 1)

    def test_invalid_compression_and_protected_fields(self):
        self.proxy()
        self.assertIn(b' 400 ', self.raw(b'POST /codex/responses HTTP/1.1\r\ncontent-encoding: zstd\r\ncontent-length: 3\r\n\r\nbad'))
        for fields in ({'model': 'other'}, {'store': True}, {'service_tier': 'priority'},
                       {'max_output_tokens': 64}, {'previous_response_id': 'other'},
                       {'tools': [{'type': 'web_search'}]}, {'stream': False}):
            self.assertEqual(self.post({**self.body(), **fields})[0], 403)
        self.assertEqual(Upstream.seen, [])

    def test_single_frame_boundary_including_skippable_frames(self):
        import zstandard as z
        self.proxy(max_body=1024)
        normal = z.ZstdCompressor().compress(json.dumps(self.body()).encode())
        # A small decoded frame can advertise a larger window without allocating it here.
        compressor = z.ZstdCompressor(compression_params=z.ZstdCompressionParameters.from_level(
            1, window_log=20, force_max_window=True, write_content_size=False)).compressobj()
        later = compressor.compress(json.dumps(self.body()).encode()) + compressor.flush()
        self.assertGreater(z.get_frame_parameters(later).window_size, 1024)
        empty = z.ZstdCompressor().compress(b'')
        skip = struct.pack('<II', 0x184D2A50, 0)
        for raw in (empty + later, normal + normal, skip + normal, normal + skip):
            conn = UnixHTTP(str(self.tmp / 'egress.sock'))
            conn.request('POST', '/codex/responses', raw, {'content-encoding': 'zstd'})
            resp = conn.getresponse(); self.assertEqual(resp.status, 400); resp.read(); conn.close()
        self.assertEqual(Upstream.seen, [])
        self.assertEqual(self.post(self.body())[0], 200)

    def test_response_done_and_uncertain_requests_are_accounting_calls(self):
        self.proxy()
        Upstream.terminal = 'response.done'
        self.assertEqual(self.post(self.body())[0], 200)
        self.assertEqual(self.log()[-1]['terminal'], 'response.done')
        self.assertEqual(self.log()[-1]['response_id'], 'r1')
        self.assertEqual(self.log()[-1]['usage']['input_tokens'], 7)
        Upstream.terminal = None
        self.assertEqual(self.post(self.body())[0], 200)
        # A connection failure is also uncertain after forwarding starts.
        with (self.tmp / 'proxy.jsonl').open('a') as f:
            f.write(json.dumps({'event': 'upstream_error', 'at': '2026-11-12T03:00:00Z', 'model': 'gpt-6.1-sol'}) + '\n')
        ledger = load_ledger()
        state = self.tmp / 'state'; state.mkdir()
        (state / 'ledger.jsonl').write_text(json.dumps({'host': 'pi', 'event': 'launch', 'turn': 1,
            'session_id': 'sub-test', 'auth_lane': 'subscription', 'files': {'proxy_log': str(self.tmp / 'proxy.jsonl')}}) + '\n')
        calls, _ = ledger.build_pi(None, None, str(state))
        self.assertEqual(len(calls), 3)
        self.assertEqual(calls[0]['call_id'], 'r1')
        self.assertEqual(calls[0]['usage']['input_tokens'], 7)
        self.assertEqual(sum(ledger.call_spend(c)['unpriced_calls'] for c in calls), 3)
        self.assertTrue(all(c['pools'] == ['codex'] and c['quota_multiplier'] is None for c in calls))

    def test_host_auth_mount_refused_at_launch_and_delivery(self):
        home = self.tmp / 'home'; home.mkdir()
        codex = home / '.codex'; codex.mkdir()
        auth = codex / 'auth.json'; auth.write_text('{}')
        skills = codex / 'skills'; skills.mkdir()
        ws = self.tmp / 'ws'; ws.mkdir()
        alias = self.tmp / 'auth-alias'; alias.symlink_to(auth)
        state = self.tmp / 'state'; state.mkdir()
        ledger = SimpleNamespace(path=state / 'ledger.jsonl')
        host = rh.Pi('unused')
        opts = rh.Pi.options(['--model', 'openai-codex/gpt-6.1-sol'], 'subscription', None)
        opts['container_subscription'] = True
        runtime = {'root': '/runtime', 'resolved': '/runtime', 'pi_version': 'test'}
        with patch.object(pathlib.Path, 'home', return_value=home), patch.object(pc, 'runtime_paths', return_value=runtime), \
             patch.object(pc, 'fingerprint', return_value={'sha256': 'test'}), \
             patch.object(rh, '_run', return_value=SimpleNamespace(returncode=0)):
            for path in (auth, codex, alias):
                with self.subTest(path=path):
                    with self.assertRaises(rh.HostError):
                        host.container_environment('s', ledger, cwd=str(ws), container={'ro': [str(path)]}, opts=opts)
                    env = {'runtime': runtime, 'mounts': [[str(path), 'ro'], [str(ws), 'rw']], 'home': str(home)}
                    with self.assertRaises(rh.HostError) as cm:
                        host._check_environment(env, cwd=str(ws), ledger=ledger)
                    self.assertEqual(cm.exception.status, 'environment_changed')
            env = host.container_environment('safe', ledger, cwd=str(ws), container={'ro': [str(skills)]}, opts=opts)
            self.assertEqual(host._check_environment(env, cwd=str(ws), ledger=ledger)['mounts'], env['mounts'])

    def test_auth_validation_and_no_secret_errors(self):
        self.assertEqual(pc.codex_account(token()), 'test-host-account')
        for bad in ('sk-api-key', token(exp=0), token(exp=float('nan')), token(iss='other'), pc.CODEX_PLACEHOLDER, 'a.bad.c'):
            with self.assertRaises(ValueError) as cm:
                pc.codex_account(bad)
            self.assertNotIn(bad, str(cm.exception))
        opts = {'provider': 'openai-codex', 'container_subscription': True}
        host = rh.Pi('unused')
        with patch.object(pathlib.Path, 'home', return_value=self.tmp):
            (self.tmp / '.codex').mkdir()
            auth = self.tmp / '.codex/auth.json'
            auth.write_text(json.dumps({'auth_mode': 'chatgpt', 'tokens': {'access_token': token()}}))
            before = auth.read_bytes()
            self.assertEqual(pc.codex_account(host.provider_key(opts, cwd=str(self.tmp))), 'test-host-account')
            self.assertEqual(auth.read_bytes(), before)
            auth.write_text(json.dumps({'auth_mode': 'apikey', 'tokens': {'access_token': token()}}))
            with self.assertRaises(rh.HostError):
                host.provider_key(opts, cwd=str(self.tmp))

    def test_subscription_controls_and_container_has_only_public_placeholder(self):
        opts = rh.Pi.options(['--model', 'openai-codex/gpt-6.1-sol'], 'subscription', None)
        opts['container_subscription'] = True
        for bad in ({'max_usd': 1}, {'max_output_tokens': 64}, {'usd_strict_tier': True}):
            with self.assertRaises(pc.ContainerError):
                rh.Pi._usd_limit(bad, opts)
        with self.assertRaises(rh.HostError):
            rh.Pi.options(['--model', 'openai-codex/gpt-6.1-sol'], 'subscription', 'priority')
        env = sample_env(self.tmp)
        env['selection'].update(model=opts['model'], tier=None, lane='subscription')
        values = pc.container_env(env, '/record')
        self.assertEqual(values['OPENAI_CODEX_API_KEY'], pc.CODEX_PLACEHOLDER)
        self.assertNotIn('test-host-account', json.dumps(values))

    @unittest.skipUnless(pathlib.Path('/snap/pi-coding-agent/77/node/bin/node').exists(), 'native Pi snap unavailable')
    def test_native_pi_sse_transport_through_proxy(self):
        from pi_container_entry import pump
        self.proxy()
        sockpath = str(self.tmp / 'egress.sock')
        class Relay(socketserver.BaseRequestHandler):
            def handle(self):
                upstream = socket.socket(socket.AF_UNIX)
                upstream.connect(sockpath)
                t = threading.Thread(target=pump, args=(self.request, upstream), daemon=True)
                t.start()
                pump(upstream, self.request)
                t.join(timeout=2)
                upstream.close()
        relay = socketserver.ThreadingTCPServer(('127.0.0.1', 0), Relay)
        self.addCleanup(relay.server_close)
        self.addCleanup(relay.shutdown)
        threading.Thread(target=relay.serve_forever, daemon=True).start()
        script = """
import {stream} from '/snap/pi-coding-agent/77/lib/pi/packages/ai/dist/api/openai-codex-responses.js';
const model = {id:'gpt-6.1-sol',provider:'openai-codex',api:'openai-codex-responses',name:'mock',
 baseUrl:process.argv[1],reasoning:false,input:['text'],contextWindow:272000,maxTokens:16000,
 cost:{input:0,output:0,cacheRead:0,cacheWrite:0}};
const events = stream(model, {messages:[{role:'user',content:'Hello',timestamp:Date.now()}]},
 {apiKey:process.argv[2],transport:'sse',maxRetries:0});
for await (const event of events) { if(event.type === 'error') process.exitCode = 1; }
"""
        run = subprocess.run(['/snap/pi-coding-agent/77/node/bin/node', '--input-type=module', '-e', script,
                              f'http://127.0.0.1:{relay.server_address[1]}', pc.CODEX_PLACEHOLDER],
                             capture_output=True, text=True, timeout=20, env={'PATH': '/usr/bin:/bin'})
        self.assertEqual(run.returncode, 0, run.stderr)
        self.assertEqual(len(Upstream.seen), 1)
        self.assertEqual(Upstream.seen[0]['path'], '/backend-api/codex/responses')
        self.assertEqual(self.log()[-1]['terminal'], 'response.completed')

    def test_accounting_never_labels_subscription_as_api_spend(self):
        self.proxy()
        self.post(self.body())
        ledger = load_ledger()
        state = self.tmp / 'state'; state.mkdir()
        (state / 'ledger.jsonl').write_text(json.dumps({'host': 'pi', 'event': 'launch', 'turn': 1,
            'session_id': 'sub-test', 'auth_lane': 'subscription', 'files': {'proxy_log': str(self.tmp / 'proxy.jsonl')}}) + '\n')
        calls, _ = ledger.build_pi(None, None, str(state))
        self.assertEqual(len(calls), 1)
        self.assertEqual(calls[0]['pools'], ['codex'])
        self.assertIsNone(calls[0]['usd'])
        self.assertIsNone(calls[0]['quota_multiplier'])


@unittest.skipIf(base.SKIP, base.SKIP or '')
class RealSubscriptionContainerTests(unittest.TestCase):
    setUp = base.RealContainerTests.setUp
    remove_containers = base.RealContainerTests.remove_containers
    host = base.RealContainerTests.host
    wait_idle = base.RealContainerTests.wait_idle

    def test_launch_deliver_custody_and_cleanup_with_synthetic_host_auth(self):
        authdir = self.tmp / 'home/.codex'
        authdir.mkdir()
        credential = token()
        authpath = authdir / 'auth.json'
        authpath.write_text(json.dumps({'auth_mode': 'chatgpt', 'tokens': {'access_token': credential}}))
        original = authpath.read_bytes()
        self.env['ROLE_HOST_PI_UPSTREAM_CODEX'] = self.env['ROLE_HOST_PI_UPSTREAM'].removesuffix('/v1') + '/backend-api/codex'
        prompt = self.tmp / 'prompt'; prompt.write_text('Reply briefly.')
        code, row = self.host('launch', 'pi', '--cwd', str(self.tmp / 'ws'), '--name', 'subscription-container-test',
                              '--prompt-file', str(prompt), '--auth', 'subscription', '--container', '--max-requests', '2',
                              '--host-arg=--model', '--host-arg=openai-codex/gpt-6.1-sol')
        self.assertEqual(code, 0, row)
        sid = row['session_id']
        code, status = self.wait_idle(sid)
        self.assertEqual(code, 0, status)
        self.assertEqual(status['last_turn']['outcome'], 'completed', status)
        self.assertTrue(status['last_turn']['container']['removed'])
        self.assertEqual(status['last_turn']['service_tier']['status'], 'not_requested')
        prompt.write_text('Again.')
        code, got = self.host('deliver', 'pi', sid, '--cwd', str(self.tmp / 'ws'), '--message-file', str(prompt))
        self.assertEqual((code, got.get('turn')), (0, 2), got)
        code, status = self.wait_idle(sid)
        self.assertEqual(status['last_turn']['outcome'], 'completed', status)
        self.assertTrue(status['last_turn']['container']['removed'])
        self.assertEqual(authpath.read_bytes(), original)
        for path in (self.tmp / 'state').rglob('*'):
            if path.is_file():
                self.assertNotIn(credential.encode(), path.read_bytes(), str(path))
                self.assertNotIn(b'test-host-account', path.read_bytes(), str(path))
        code, retired = self.host('retire', 'pi', sid)
        self.assertEqual(code, 0, retired)


if __name__ == '__main__':
    unittest.main()
