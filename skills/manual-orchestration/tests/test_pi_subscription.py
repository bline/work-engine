"""Subscription transport: synthetic credentials and local upstream only."""
import base64
import io
import multiprocessing
import os
import tempfile
import json
import pathlib
import subprocess
import socket
import socketserver
import shutil
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

    def test_fresh_proxy_turns_ignore_planted_imports(self):
        """Actual proxy in fresh interpreters, synthetic token/local upstream.

        Simulate a tool planting a module between turns. Also leave an extra
        module beside the script to discriminate the original script-directory
        import flaw. No Docker or native credential source is involved.
        """
        import zstandard
        workspace = self.tmp / 'workspace'; workspace.mkdir()
        trusted = self.tmp / 'host'; trusted.mkdir()
        proxy_source = trusted / 'pi_container.py'
        shutil.copyfile(pc.__file__, proxy_source)
        marker = self.tmp / 'import-executed'
        poison = f"from pathlib import Path\nPath({str(marker)!r}).write_text('executed')\nraise RuntimeError('planted module')\n"
        probe = trusted / 'probe.py'
        probe.write_text('''import base64, http.client, json, runpy, socket, sys
config = json.load(sys.stdin)
pc = runpy.run_path(config['source'])
class UnixHTTP(http.client.HTTPConnection):
    def connect(self):
        self.sock = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
        self.sock.connect(config['socket'])
proxy = pc['EgressProxy'](config['socket'], upstream=config['upstream'], key=config['key'],
    model_id='gpt-6.1-sol', tier=None, log_path=config['log'], provider='openai-codex').start()
try:
    conn = UnixHTTP('localhost', timeout=5)
    conn.request('POST', '/codex/responses', base64.b64decode(config['body']), {'content-encoding': 'zstd'})
    response = conn.getresponse()
    response.read(); conn.close()
    print(response.status)
finally:
    proxy.stop()
''')
        body = base64.b64encode(zstandard.ZstdCompressor().compress(json.dumps(self.body()).encode())).decode()
        config = {'source': str(proxy_source), 'socket': str(self.tmp / 'fresh.sock'),
                  'upstream': f'http://127.0.0.1:{self.upstream.server_port}/backend-api/codex',
                  'key': token(), 'log': str(self.tmp / 'fresh.jsonl'), 'body': body}
        for turn in (1, 2):
            # New and modified files in the tool workspace must never become
            # host imports, including when supplied through PYTHONPATH.
            (workspace / 'zstandard.py').write_text(poison)
            (workspace / 'sitecustomize.py').write_text(poison)
            if turn == 2:
                (trusted / 'zstandard.py').write_text(poison)
            env = {**os.environ, 'PYTHONPATH': str(workspace), 'PYTHONUSERBASE': str(workspace)}
            r = subprocess.run([sys.executable, '-I', str(probe)], cwd=workspace, env=env,
                               input=json.dumps(config), capture_output=True, text=True, timeout=10)
            self.assertEqual(r.returncode, 0, r.stderr)
            self.assertEqual(r.stdout.strip(), '200', r.stderr)
            self.assertFalse(marker.exists())
        self.assertEqual(len(Upstream.seen), 2)

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

    def test_writable_active_host_checkout_refused_before_launch_effects(self):
        ws = self.tmp / 'checkout'; ws.mkdir()
        host = rh.Pi('unused')
        ledger = SimpleNamespace(path=self.tmp / 'state/ledger.jsonl')
        opts = rh.Pi.options(['--model', 'openai-codex/gpt-6.1-sol'], 'subscription', None)
        opts['container_subscription'] = True
        with patch.object(pc, 'HERE', ws / 'skills/host/scripts'), \
             patch.object(pc, 'runtime_paths', return_value={}), patch.object(rh, '_run') as run:
            with self.assertRaises(rh.HostError) as cm:
                host.container_environment('test', ledger, cwd=str(ws), container={}, opts=opts)
        self.assertIn('trusted host', cm.exception.detail)
        run.assert_not_called()

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
        for bad in ({'max_usd': float('inf')}, {'max_usd': float('nan')}, {'max_usd': 0}, {'max_usd': -1}, {'max_output_tokens': 64}, {'usd_strict_tier': True}):
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


class SubscriptionBudgetTests(SubscriptionTests):
    # Inherited tests also verify the unchanged opt-out route.
    def budget(self, max_usd=1, spent=0):
        return pc.SubscriptionProxyBudget(max_usd=max_usd, rates=pc.usd_rates('gpt-6.1-sol'), spent_before_usd=spent)

    def test_threshold_crossing_and_one_response_overshoot(self):
        stops = []
        b = self.budget(max_usd=1e-9)
        self.proxy(budget=b, on_usd_exhausted=stops.append)
        self.assertEqual(self.post(self.body())[0], 200)
        self.assertEqual(self.post(self.body())[0], 429)
        self.assertEqual(len(Upstream.seen), 1)
        m = next(r['subscription_proxy'] for r in self.log() if 'subscription_proxy' in r)
        self.assertGreater(m['cost_proxy_usd'], b.max_usd)
        self.assertTrue(m['limit_reached'])
        self.assertTrue(stops)
        self.assertNotIn('max_output_tokens', Upstream.seen[0]['body'])

    def test_required_usage_unknown_stops_and_replay_preserves_admission(self):
        self.proxy(budget=self.budget())
        Upstream.usage = {'input_tokens': True, 'output_tokens': 1}
        self.post(self.body())
        self.assertEqual(self.post(self.body())[0], 429)
        totals = pc.prior_subscription_proxy_state([self.tmp / 'proxy.jsonl'])
        self.assertTrue(totals['accounting_unknown'])
        self.assertEqual(pc.subscription_proxy_totals(self.log())['unresolved_admissions'], [1])

    def test_concurrent_admission_refused_before_forward(self):
        admitted = threading.Event(); unblock = threading.Event()
        original = Upstream.do_POST
        def held(handler):
            admitted.set(); unblock.wait(5); original(handler)
        self.proxy(budget=self.budget())
        result = []
        with patch.object(Upstream, 'do_POST', held):
            thread = threading.Thread(target=lambda: result.append(self.post(self.body())[0]))
            thread.start()
            try:
                self.assertTrue(admitted.wait(3))
                self.assertEqual(self.post(self.body())[0], 429)
                self.assertEqual(len([r for r in self.log() if r['event'] == 'subscription_proxy_admitted']), 1)
            finally:
                unblock.set(); thread.join(5)
        self.assertEqual(result, [200])
        self.assertEqual(len(Upstream.seen), 1)

    def test_cache_unknown_is_conservative_and_tiny_totals_survive_turns(self):
        self.proxy(budget=self.budget())
        Upstream.usage = {'input_tokens': 1, 'output_tokens': 1}
        self.post(self.body())
        rows = self.log()
        m = rows[-1]['subscription_proxy']
        self.assertEqual(m['valuation_basis'], 'conservative_all_input_at_dearest_rate')
        self.assertGreater(m['cost_proxy_usd'], 0)
        self.assertLess(m['cost_proxy_usd'], .01)
        path = self.tmp / 'proxy.jsonl'
        prior = pc.prior_subscription_proxy_state([path])
        self.assertEqual(prior['spent_before_usd'], m['cost_proxy_usd'])
        self.assertFalse(prior['accounting_unknown'])
        resumed = self.budget(max_usd=m['cost_proxy_usd'] * 1.5, spent=prior['spent_before_usd'])
        ticket, _ = resumed.admit()
        second = resumed.settle(ticket, http_status=200,
                                response={'id': 'r2', 'model': 'gpt-6.1-sol', 'usage': Upstream.usage},
                                terminal='response.done', stream_error=False)
        self.assertTrue(second['limit_reached'])
        self.assertEqual(second['session_cost_proxy_usd'], m['cost_proxy_usd'] * 2)
        with path.open('a') as f:
            f.write('{"event":"subscription_proxy_admitted","rid":2}\n{"partial"')
        prior = pc.prior_subscription_proxy_state([path])
        self.assertEqual(prior['spent_before_usd'], m['cost_proxy_usd'])
        self.assertTrue(prior['accounting_unknown'])

    def test_identity_failures_stream_failures_and_invalid_usage(self):
        for changes in ({'model': 'other'}, {'model': None}, {'id': None}, {'usage': None},
                        {'usage': {'input_tokens': -1, 'output_tokens': 1}},
                        {'usage': {'input_tokens': 1.5, 'output_tokens': 1}},
                        {'usage': {'input_tokens': float('inf'), 'output_tokens': 1}},
                        {'usage': {'input_tokens': 1, 'output_tokens': False}},
                        {'usage': {'input_tokens': 1, 'output_tokens': 1, 'input_tokens_details': {'cached_tokens': 2}}}):
            b = self.budget(); ticket, _ = b.admit()
            response = {'id': 'r', 'model': 'gpt-6.1-sol', 'usage': base.USAGE, **changes}
            m = b.settle(ticket, http_status=200, response=response, terminal='response.done', stream_error=False)
            self.assertEqual(m['status'], 'unknown', changes)
            self.assertIsNone(b.admit()[0])
        for terminal, error, status in ((None, False, 200), ('response.failed', False, 200),
                                         ('response.completed', True, 200), ('response.completed', False, 500)):
            b = self.budget(); ticket, _ = b.admit()
            m = b.settle(ticket, http_status=status, response={'id': 'r', 'model': 'gpt-6.1-sol', 'usage': base.USAGE},
                         terminal=terminal, stream_error=error)
            self.assertEqual(m['status'], 'unknown')
            self.assertIsNone(b.admit()[0])

    def test_realistic_native_done_and_exact_request_attribution(self):
        self.proxy(budget=self.budget())
        Upstream.terminal = 'response.done'
        Upstream.usage = {'input_tokens': 7, 'output_tokens': 2,
                          'input_tokens_details': {'cached_tokens': 3},
                          'output_tokens_details': {'reasoning_tokens': 1}}
        self.post(self.body()); self.post(self.body())
        ledger = load_ledger(); state = self.tmp / 'state'; state.mkdir()
        (state / 'ledger.jsonl').write_text(json.dumps({'host': 'pi', 'event': 'launch', 'turn': 1,
            'session_id': 'sub', 'auth_lane': 'subscription', 'files': {'proxy_log': str(self.tmp / 'proxy.jsonl')}}) + '\n')
        calls, _ = ledger.build_pi(None, None, str(state))
        self.assertEqual(len(calls), 2)  # repeated mock response ID must not erase an admitted call
        self.assertNotEqual(calls[0]['call_id'], calls[1]['call_id'])
        self.assertTrue(all(c['usd'] is None and c['quota_multiplier'] is None and c['pools'] == ['codex'] for c in calls))
        self.assertTrue(all(c['subscription_proxy']['status'] == 'valued' for c in calls))
        total = pc.prior_subscription_proxy_state([self.tmp / 'proxy.jsonl'])['spent_before_usd']
        self.assertEqual(total, sum(c['subscription_proxy']['cost_proxy_usd'] for c in calls))

    def test_unknown_exchange_refuses_delivery_and_status_preserves_known_cost(self):
        self.proxy(budget=self.budget())
        self.post(self.body())
        charge = self.log()[-1]['subscription_proxy']['cost_proxy_usd']
        Upstream.terminal = None
        self.post(self.body())
        path = self.tmp / 'proxy.jsonl'
        with path.open('a') as f:
            f.write('broken\n{"partial"')
        lim = {'measurement': pc.SUBSCRIPTION_PROXY_BASIS, 'max_usd': 1,
               'spent_before_usd': 0, 'rates': pc.usd_rates('gpt-6.1-sol')}
        host = rh.Pi('unused')
        row = {'session_id': 'sub', 'turn': 1, 'usd_limit': lim,
               'files': {k: str(self.tmp / k) for k in ('events', 'exit', 'tier_record', 'runner_record')}}
        row['files']['proxy_log'] = str(path)
        status = host.observe_turn(row)['egress']['subscription_proxy']
        self.assertEqual(status['cost_proxy_usd'], charge)
        self.assertTrue(status['accounting_unknown'])
        self.assertEqual(status['unresolved_admissions'], [2])
        with patch.object(host, '_turns', return_value=[row]):
            with self.assertRaises(rh.HostError) as cm:
                host._prior_usd('sub', object(), {'usd_limit': lim})
        self.assertEqual(cm.exception.status, 'usd_spend_not_established')

    def test_partial_native_completion_remains_unknown(self):
        stops = []
        self.proxy(budget=self.budget(), on_usd_exhausted=stops.append)
        def partial(handler):
            handler.rfile.read(int(handler.headers['content-length']))
            handler.send_response(200); handler.end_headers()
            handler.wfile.write(b'data: {"type":"response.done","response":{"id":"r","model":"gpt-6.1-sol"')
        with patch.object(Upstream, 'do_POST', partial):
            self.post(self.body())
        self.assertTrue(stops)
        self.assertTrue(pc.subscription_proxy_totals(self.log())['accounting_unknown'])
        self.assertEqual(self.post(self.body())[0], 429)

    def test_identity_and_stream_errors_stop_actual_proxy(self):
        stops = []
        self.proxy(budget=self.budget(), on_usd_exhausted=stops.append)
        def wrong(handler):
            handler.rfile.read(int(handler.headers['content-length']))
            handler.send_response(200); handler.end_headers()
            event = {'type': 'response.done', 'response': {'id': 'bad', 'model': 'other', 'usage': base.USAGE}}
            handler.wfile.write(('data: ' + json.dumps(event) + '\n\n').encode())
        with patch.object(Upstream, 'do_POST', wrong):
            self.post(self.body())
        self.assertEqual(self.post(self.body())[0], 429)
        self.assertEqual(self.log()[-2]['subscription_proxy']['reason'], 'response_identity_not_established')
        self.assertTrue(stops)

    def test_upstream_error_is_unknown_and_admission_precedes_effect(self):
        stops = []
        self.proxy(budget=self.budget(), on_usd_exhausted=stops.append)
        def failed(conn, *args, **kwargs):
            rows = self.log()
            self.assertEqual(rows[-1]['event'], 'subscription_proxy_admitted')
            raise OSError('synthetic upstream failure')
        # Patch only the upstream route; the Unix test client has its own connect method.
        with patch.object(pc.http.client.HTTPConnection, 'connect', failed):
            self.assertEqual(self.post(self.body())[0], 502)
        self.assertTrue(stops)
        self.assertTrue(pc.subscription_proxy_totals(self.log())['accounting_unknown'])
        self.assertEqual(self.post(self.body())[0], 429)

    def test_persistence_failure_never_forwards_and_stops_runner(self):
        stops = []
        self.proxy(budget=self.budget(), on_usd_exhausted=stops.append)
        with patch.object(pc.os, 'fsync', side_effect=OSError('synthetic disk error')):
            self.assertEqual(self.post(self.body())[0], 429)
        self.assertEqual(Upstream.seen, [])
        self.assertTrue(stops)
        self.assertTrue(pc.prior_subscription_proxy_state([self.tmp / 'proxy.jsonl'])['accounting_unknown'])

    def test_interrupted_stream_after_terminal_stays_unknown(self):
        stops = []
        self.proxy(budget=self.budget(), on_usd_exhausted=stops.append)
        original = pc.http.client.HTTPResponse.read1
        calls = 0
        def interrupted(response, *args):
            nonlocal calls
            calls += 1
            if calls > 1:
                raise OSError('synthetic stream interruption')
            return original(response, *args)
        with patch.object(pc.http.client.HTTPResponse, 'read1', interrupted):
            self.post(self.body())
        self.assertTrue(stops)
        self.assertTrue(pc.subscription_proxy_totals(self.log())['accounting_unknown'])
        self.assertEqual(self.post(self.body())[0], 429)

    def test_objective_admission_refused_before_process_or_reader(self):
        prompt = self.tmp / 'prompt'; prompt.write_text('mock')
        with patch.object(rh, 'admit_transaction') as admit, patch.object(rh.Pi, 'launch') as launch:
            rc = rh.main(['--state', str(self.tmp / 'state'), 'launch', 'pi', '--cwd', str(self.tmp),
                          '--name', 'mock', '--prompt-file', str(prompt), '--auth', 'subscription',
                          '--container', '--max-usd', '1', '--objective-record', str(self.tmp / 'objective'),
                          '--host-arg=--model', '--host-arg=openai-codex/gpt-6.1-sol'])
        self.assertEqual(rc, rh.USAGE)
        admit.assert_not_called(); launch.assert_not_called()


    def test_malformed_log_shapes_retain_prior_cost_and_fail_closed(self):
        self.proxy(budget=self.budget())
        self.post(self.body())
        path = self.tmp / 'proxy.jsonl'
        known = self.log()[-1]['subscription_proxy']['cost_proxy_usd']
        with path.open('a') as f:
            for row in ({'event': 'request', 'usage': 'bad'},
                        {'subscription_proxy': {'rid': [], 'status': 'valued', 'cost_proxy_usd': 0}}, []):
                f.write(json.dumps(row) + '\n')
        totals = pc.prior_subscription_proxy_state([path])
        self.assertEqual(totals['spent_before_usd'], known)
        self.assertTrue(totals['accounting_unknown'])
        # Even syntactically complete JSON without LF is an uncommitted tail.
        clean = self.tmp / 'tail'; clean.write_text('{"event":"request"}')
        self.assertTrue(pc.prior_subscription_proxy_state([clean])['accounting_unknown'])

    def test_terminal_before_truncated_http_body_is_unknown(self):
        stops = []
        self.proxy(budget=self.budget(), on_usd_exhausted=stops.append)
        def truncated(handler):
            handler.rfile.read(int(handler.headers['content-length']))
            event = {'type': 'response.done', 'response': {'id': 'r', 'model': 'gpt-6.1-sol', 'usage': base.USAGE}}
            payload = ('data: ' + json.dumps(event) + '\n\n').encode()
            handler.send_response(200)
            handler.send_header('content-length', str(len(payload) + 1000))
            handler.end_headers(); handler.wfile.write(payload)
        with patch.object(Upstream, 'do_POST', truncated):
            self.post(self.body())
        self.assertTrue(stops)
        self.assertEqual(self.post(self.body())[0], 429)
        self.assertTrue(pc.prior_subscription_proxy_state([self.tmp / 'proxy.jsonl'])['accounting_unknown'])
        host = rh.Pi('unused')
        with patch.object(host, '_turns', return_value=[{'turn': 1, 'files': {'proxy_log': str(self.tmp / 'proxy.jsonl')}}]):
            with self.assertRaises(rh.HostError):
                host._prior_usd('s', None, {'usd_limit': {'measurement': pc.SUBSCRIPTION_PROXY_BASIS, 'max_usd': 1}})

    def test_terminal_with_complete_http_body_is_valued(self):
        self.proxy(budget=self.budget())
        def complete(handler):
            handler.rfile.read(int(handler.headers['content-length']))
            event = {'type': 'response.done', 'response': {'id': 'r', 'model': 'gpt-6.1-sol', 'usage': base.USAGE}}
            payload = ('data: ' + json.dumps(event) + '\n\n').encode()
            handler.send_response(200); handler.send_header('content-length', str(len(payload)))
            handler.end_headers(); handler.wfile.write(payload)
        with patch.object(Upstream, 'do_POST', complete):
            self.post(self.body())
        self.assertFalse(pc.prior_subscription_proxy_state([self.tmp / 'proxy.jsonl'])['accounting_unknown'])

    def test_launch_rates_and_output_cap_semantics(self):
        opts = rh.Pi.options(['--model', 'openai-codex/gpt-6.1-sol'], 'subscription', None)
        opts['container_subscription'] = True
        lim, output = rh.Pi._usd_limit({'max_usd': .1}, opts)
        self.assertIsNone(output)
        self.assertEqual(lim['measurement'], pc.SUBSCRIPTION_PROXY_BASIS)
        self.assertEqual(lim['valuation_tier'], 'default')
        with self.assertRaises(pc.ContainerError):
            rh.Pi._usd_limit({'max_usd': .1}, {**opts, 'model': 'openai-codex/unknown'})

    def test_unfinished_admissions_are_unpriced_ledger_calls(self):
        path = self.tmp / 'proxy.jsonl'
        path.write_text(json.dumps({'event': 'subscription_proxy_admitted', 'at': '2026-11-12T03:00:00Z',
                                    'rid': 1, 'model': 'gpt-6.1-sol'}) + '\n{"partial"')
        ledger = load_ledger(); state = self.tmp / 'state'; state.mkdir()
        (state / 'ledger.jsonl').write_text(json.dumps({'host': 'pi', 'event': 'launch', 'turn': 1,
            'session_id': 'sub', 'auth_lane': 'subscription', 'files': {'proxy_log': str(path)}}) + '\n')
        calls, _ = ledger.build_pi(None, None, str(state))
        self.assertEqual(len(calls), 1)
        self.assertEqual(calls[0]['pools'], ['codex'])
        self.assertIsNone(calls[0]['usd'])
        self.assertEqual(calls[0]['subscription_proxy']['status'], 'unknown')



class SubscriptionTurnAdmissionTests(unittest.TestCase):
    def setUp(self):
        folder = tempfile.TemporaryDirectory(); self.addCleanup(folder.cleanup)
        self.tmp = pathlib.Path(folder.name)
        self.sid = '00000000-0000-4000-8000-000000000001'
        self.ledger = rh.Ledger(self.tmp / 'state')
        self.host = rh.Pi('unused')
        self.env = sample_env(self.tmp)
        self.env['selection'].update(model='openai-codex/gpt-6.1-sol', tier=None, lane='subscription')
        self.env.update(container_dir=str(self.tmp / 'container'), host_dir=str(self.tmp / 'host'),
                        session_dir=str(self.tmp / 'sessions'), fingerprint={'sha256': 'test'}, max_output_tokens=None,
                        usd_limit={'max_usd': 1, 'measurement': pc.SUBSCRIPTION_PROXY_BASIS, 'rates': pc.usd_rates('gpt-6.1-sol')})
        for key in ('container_dir', 'host_dir', 'session_dir'):
            pathlib.Path(self.env[key]).mkdir()
        self.opts = {'argv': [], 'model': 'openai-codex/gpt-6.1-sol', 'provider': 'openai-codex',
                     'thinking': None, 'lane': 'subscription', 'tier': None}
        initial = self.tmp / 'turn-1.proxy.jsonl'; initial.write_text('')
        self.ledger.append({'host': 'pi', 'event': 'launch', 'session_id': self.sid, 'turn': 1, 'pid_start': 1,
                            'cwd': str(self.tmp), 'host_argv': [], 'model': self.opts['model'], 'thinking': None,
                            'auth_lane': 'subscription', 'service_tier': None, 'container_environment': self.env,
                            'files': {'proxy_log': str(initial)}})
        self.enterContext(patch.object(self.host, '_check_environment', return_value=self.env))
        self.enterContext(patch.object(self.host, 'check_lane', return_value={}))
        self.enterContext(patch.object(self.host, 'provider_key', return_value='synthetic'))
        self.enterContext(patch.object(self.host, '_verify_header'))
        self.enterContext(patch.object(self.host, 'socket_root', return_value=self.tmp / 'sockets'))
        self.enterContext(patch.object(self.host, 'status', side_effect=self.status))

    def status(self, *args):
        turns = self.host._turns(self.sid, self.ledger)
        return {'state': 'running', 'turn': 2} if len(turns) > 1 else {'state': 'idle'}

    def deliver(self):
        return self.host.deliver(self.sid, message='mock', cwd=str(self.tmp), ledger=self.ledger)

    def test_credential_runner_is_launched_with_isolated_python(self):
        def runner(argv, **kwargs):
            self.assertEqual(argv[:2], [sys.executable, '-I'])
            self.assertEqual(pathlib.Path(argv[2]).name, 'pi_container.py')
            self.assertEqual(argv[3], 'run-turn')
            spec = json.loads(pathlib.Path(argv[-1]).read_text())
            pathlib.Path(spec['files']['proxy_log']).write_text('')
            return SimpleNamespace(pid=os.getpid(), stdin=io.BytesIO())
        with patch.object(rh.subprocess, 'Popen', runner):
            self.assertEqual(self.deliver()['turn'], 2)

    def test_two_processes_cannot_admit_during_unrecorded_runner_start(self):
        ctx = multiprocessing.get_context('fork')
        started, release = ctx.Event(), ctx.Event()
        results = ctx.Queue(); self.addCleanup(results.close)
        count = ctx.Value('i', 0)
        def runner(argv, **kwargs):
            spec = json.loads(pathlib.Path(argv[-1]).read_text())
            pathlib.Path(spec['files']['proxy_log']).write_text('')
            with count.get_lock(): count.value += 1
            started.set()
            if not release.wait(8): raise RuntimeError('test did not release runner')
            return SimpleNamespace(pid=os.getpid(), stdin=io.BytesIO())
        def call():
            try: results.put(('ok', self.deliver()['turn']))
            except rh.HostError as e: results.put(('error', e.status))
        with patch.object(rh.subprocess, 'Popen', runner):
            first, second = ctx.Process(target=call), ctx.Process(target=call)
            try:
                first.start(); self.assertTrue(started.wait(5))
                second.start(); second.join(5)
                self.assertFalse(second.is_alive())
                self.assertEqual(results.get(timeout=1), ('error', 'refused_busy'))
                self.assertEqual(count.value, 1)
                self.assertEqual(len(self.ledger.rows()), 1)  # runner exists before its ledger row
                release.set(); first.join(5)
                self.assertFalse(first.is_alive())
                self.assertEqual(results.get(timeout=1), ('ok', 2))
                self.assertEqual(count.value, 1)
                self.assertEqual((first.exitcode, second.exitcode), (0, 0))
            finally:
                release.set()
                for proc in (first, second):
                    if proc.pid is not None:
                        if proc.is_alive(): proc.terminate()
                        proc.join(5)
                        proc.close()

    def test_stale_idle_result_is_rechecked_after_other_turn_was_recorded(self):
        calls = 0
        def interleaved(*args):
            nonlocal calls
            calls += 1
            if calls == 1:
                # Another delivery publishes a still-empty turn after this caller's idle observation.
                log = self.tmp / 'turn-2.proxy.jsonl'; log.write_text('')
                self.ledger.append({'host': 'pi', 'event': 'deliver', 'session_id': self.sid, 'turn': 2,
                                    'pid_start': 1, 'files': {'proxy_log': str(log)}})
                return {'state': 'idle'}
            return self.status()
        with patch.object(self.host, 'status', side_effect=interleaved), patch.object(rh.subprocess, 'Popen') as spawn:
            with self.assertRaises(rh.HostError) as cm: self.deliver()
        self.assertEqual(cm.exception.status, 'refused_busy')
        self.assertEqual(calls, 2); spawn.assert_not_called()

    def test_unrecorded_started_turn_keeps_durable_ordinal_and_unknown_request(self):
        def lost_after_start(argv, **kwargs):
            spec = json.loads(pathlib.Path(argv[-1]).read_text())
            pathlib.Path(spec['files']['proxy_log']).write_text(json.dumps(
                {'event': 'subscription_proxy_admitted', 'rid': 1, 'model': 'gpt-6.1-sol'}) + '\n')
            raise RuntimeError('simulate launcher loss after runner effect before ledger publication')
        with patch.object(rh.subprocess, 'Popen', lost_after_start), patch.object(rh.os, 'fsync', wraps=rh.os.fsync) as sync:
            with self.assertRaises(RuntimeError): self.deliver()
        self.assertGreaterEqual(sync.call_count, 2)  # host admission and its directory durable before runner
        self.assertEqual(len(self.ledger.rows()), 1)
        log = pathlib.Path(self.env['host_dir']) / 'turn-2.proxy.jsonl'
        marker = pathlib.Path(self.env['host_dir']) / 'turn-2.admission.json'
        admission = json.loads(marker.read_text())
        self.assertEqual(admission, {'session_id': self.sid, 'turn': 2, 'proxy_log': str(log)})
        # The agent may unlink its prompt; host-only admission remains authoritative.
        prompt = pathlib.Path(self.env['container_dir']) / 'turn-2.prompt'
        prompt.unlink()
        self.assertFalse(prompt.exists())
        self.assertNotIn(str(marker.parent), [src for src, _ in self.env['mounts']])
        self.assertNotEqual(marker.parent, pathlib.Path(self.env['container_dir']))
        marker_bytes = marker.read_bytes()
        self.assertTrue(pc.prior_subscription_proxy_state([log])['accounting_unknown'])
        with patch.object(rh.subprocess, 'Popen') as spawn:
            with self.assertRaises(rh.HostError) as cm: self.deliver()
        self.assertEqual(cm.exception.status, 'turn_not_established')
        self.assertIn('turn-2.admission.json', cm.exception.detail)
        spawn.assert_not_called()
        self.assertEqual(marker.read_bytes(), marker_bytes)
        self.assertFalse(prompt.exists())
        self.assertTrue(pc.prior_subscription_proxy_state([log])['accounting_unknown'])

    def test_unknown_recorded_runner_identity_refuses_before_spawn(self):
        row = self.ledger.rows()[0]; row['pid_start'] = None
        self.ledger.path.write_text(json.dumps(row) + '\n')
        with patch.object(rh.subprocess, 'Popen') as spawn:
            with self.assertRaises(rh.HostError) as cm: self.deliver()
        self.assertEqual(cm.exception.status, 'usd_spend_not_established')
        spawn.assert_not_called()



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
