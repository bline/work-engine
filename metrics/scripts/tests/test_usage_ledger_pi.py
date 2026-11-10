"""Usage ledger: Pi container turns read from role_host's host-side egress-proxy logs."""
import json
import os
import sys
import tempfile
import unittest

sys.path.insert(0, os.path.join(os.path.dirname(__file__), '..'))
import usage_ledger as L  # noqa: E402

SID = '8d4c8664-9253-4e76-8631-ea7bf43e605f'
HOST_SID = '00000000-0000-4000-8000-000000000009'


def usage(i, cached, out, cw=None):
    u = {'input_tokens': i, 'output_tokens': out, 'cached_tokens': cached, 'reasoning_tokens': 0}
    if cw is not None:
        u['cache_write_tokens'] = cw
    return u


class PiReaderTests(unittest.TestCase):
    def setUp(self):
        L.ISSUES.clear()
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        d = self.tmp.name
        self.state = os.path.join(d, 'state')
        sessions = os.path.join(d, 'sessions')
        os.makedirs(os.path.join(self.state, 'pi', SID, 'host'))
        os.makedirs(sessions)
        self.proxy = os.path.join(self.state, 'pi', SID, 'host', 'turn-1.proxy.jsonl')
        rows = [
            # Current proxy: cache writes logged.
            dict(event='request', at='2026-11-10T06:21:29Z', http_status=200, model='gpt-6-astra', response_model='gpt-6-astra',
                 requested_tier='flex', observed_tier='flex', response_id='resp_a', usage=usage(110000, 100000, 500, cw=9000), seconds=4.1),
            # Older proxy: no cache-write field; recovered from Pi's session file.
            dict(event='request', at='2026-11-10T06:21:33Z', http_status=200, model='gpt-6-astra', response_model='gpt-6-astra',
                 requested_tier='flex', observed_tier='flex', response_id='resp_b', usage=usage(120000, 110000, 300), seconds=3.9),
            dict(event='request', at='2026-11-10T06:21:37Z', http_status=200, model='gpt-6-astra', response_model='gpt-6-astra',
                 requested_tier='flex', observed_tier='flex', response_id='resp_a', usage=usage(1, 0, 1, cw=0)),
            dict(event='refused', at='2026-11-10T06:21:38Z', reason='model_not_pinned', http_status=403),
            dict(event='request', at='2026-11-10T06:21:39Z', http_status=429, model='gpt-6-astra', response_id=None, usage={}),
            # Tier the price table does not know.
            dict(event='request', at='2026-11-10T06:21:41Z', http_status=200, model='gpt-6-astra', response_model='gpt-6-astra',
                 requested_tier='flex', observed_tier='scale', response_id='resp_c', usage=usage(1000, 0, 10, cw=0)),
            # Cache writes nowhere to be found: unpriced, never guessed.
            dict(event='request', at='2026-11-10T06:21:45Z', http_status=200, model='gpt-6-astra', response_model='gpt-6-astra',
                 requested_tier='flex', observed_tier='flex', response_id='resp_d', usage=usage(5000, 0, 10)),
            # Outside the window used below.
            dict(event='request', at='2026-11-10T09:00:00Z', http_status=200, model='gpt-6-astra', response_model='gpt-6-astra',
                 requested_tier='flex', observed_tier='flex', response_id='resp_e', usage=usage(10, 0, 1, cw=0)),
        ]
        with open(self.proxy, 'w') as fh:
            fh.write('\n'.join(json.dumps(r) for r in rows) + '\n')
        with open(os.path.join(sessions, f'2026-11-10T06-21-24-419Z_{SID}.jsonl'), 'w') as fh:
            fh.write(json.dumps({'type': 'session', 'id': SID}) + '\n')
            fh.write(json.dumps({'type': 'message', 'message': {'role': 'assistant', 'responseId': 'resp_b',
                                 'usage': {'input': 0, 'cacheRead': 110000, 'cacheWrite': 10000, 'output': 300}}}) + '\n')
        ledger = [
            dict(host='pi', event='launch', session_id=SID, turn=1, thinking='high', auth_lane='api', parent_session_id='parent',
                 files={'proxy_log': self.proxy}, container_environment={'session_dir': sessions}),
            dict(host='pi', event='launch', session_id=HOST_SID, turn=1, files={'events': '/x'}),
            dict(host='codex', event='launch', session_id='other'),
        ]
        with open(os.path.join(self.state, 'ledger.jsonl'), 'w') as fh:
            fh.write('\n'.join(json.dumps(r) for r in ledger) + '\n')

    def build(self):
        return L.build_pi(L.ts_of('2026-11-10T06:00:00Z'), L.ts_of('2026-11-10T07:00:00Z'), self.state)

    def test_flex_calls_marked_and_priced_at_half(self):
        rows, cov = self.build()
        self.assertEqual(cov, {'pi_container_turns': 1, 'pi_host_turns_not_read': 1, 'pi_proxy_logs_missing': 0})
        by = {r['call_id']: r for r in rows}
        self.assertEqual(sorted(by), ['resp_a', 'resp_b', 'resp_c', 'resp_d'])
        a = by['resp_a']
        self.assertEqual((a['provider'], a['service_tier'], a['quota_multiplier'], a['effort'], a['thread']),
                         ('pi', 'flex', 0.5, 'high', SID))
        standard = (1000 * 10 + 100000 * 1 + 9000 * 12.5) / 1e6 + 500 * 50 / 1e6
        self.assertAlmostEqual(a['usd'], standard)
        self.assertAlmostEqual(a['speed_adjusted_api_usd_low'], standard * 0.5)
        self.assertAlmostEqual(a['speed_adjusted_api_usd_high'], standard * 0.5)
        self.assertEqual(a['cache_write_basis'], 'egress_proxy')

    def test_cache_writes_recovered_from_pi_session(self):
        b = {r['call_id']: r for r in self.build()[0]}['resp_b']
        self.assertEqual((b['cache_write_basis'], b['usage']['cache_write_input_tokens']), ('pi_session', 10000))
        self.assertAlmostEqual(b['usd'], (110000 * 1 + 10000 * 12.5) / 1e6 + 300 * 50 / 1e6)

    def test_unknown_tier_gets_the_usual_bounds(self):
        c = {r['call_id']: r for r in self.build()[0]}['resp_c']
        self.assertIsNone(c['quota_multiplier'])
        self.assertAlmostEqual(c['speed_adjusted_api_usd_low'], c['usd'] * L.PRICING_RULES['unknown_tier_low'])
        self.assertAlmostEqual(c['speed_adjusted_api_usd_high'], c['usd'] * L.PRICING_RULES['unknown_tier_high'])

    def test_unknown_cache_writes_stay_unpriced_and_rows_are_not_double_counted(self):
        rows, _ = self.build()
        d = {r['call_id']: r for r in rows}['resp_d']
        self.assertIsNone(d['usd'])
        kinds = [i['kind'] for i in L.ISSUES]
        self.assertIn('duplicate_response_id', kinds)
        self.assertIn('pi_request_without_response', kinds)
        self.assertIn('unpriced_call', kinds)

    def test_tier_is_the_cell_key(self):
        rows, _ = self.build()
        cells = L.f11_summary(rows, [])['by_cell']
        self.assertIn('pi|gpt-6-astra|high|flex', cells)

    def test_missing_state_is_empty_not_an_error(self):
        rows, cov = L.build_pi(None, None, os.path.join(self.tmp.name, 'nowhere'))
        self.assertEqual((rows, cov['pi_container_turns']), ([], 0))

    def tree(self, extra_ledger=()):
        from unittest import mock
        ledger = os.path.join(self.state, 'ledger.jsonl')
        if extra_ledger:
            with open(ledger, 'a') as fh:
                fh.write(''.join(json.dumps(r) + '\n' for r in extra_ledger))
        record = os.path.join(self.tmp.name, 'record.json')
        with open(record, 'w') as fh:
            json.dump({'scope_budget': {'objective_id': 'o', 'started_at': '2026-11-10T06:00:00Z',
                                        'closed_at': '2026-11-10T07:00:00Z',
                                        'sessions': [{'session_id': 'parent', 'provider': 'codex'}]}}, fh)

        def codex(since, until, turns=None, metadata=None):
            metadata['parent'] = {}
            return [], [], [], 0
        with mock.patch.object(L, 'build_codex', side_effect=codex), \
                mock.patch.object(L, 'build_claude', return_value=([], [], 0)):
            return L.cost_tree(record, role_ledger=ledger)[0]

    def test_tree_attaches_pi_child_calls(self):
        tree = self.tree()
        node = {n['node_id']: n for n in tree['nodes']}[SID]
        self.assertEqual((node['provider'], node['link'], node['parent_node_id']), ('pi', 'role_host', 'parent'))
        missing = [i for i in tree['issues'] if i['kind'] == 'transcript_missing']
        self.assertEqual(missing, [])
        # resp_a..resp_d are in the window (resp_d unpriced); resp_e is after close.
        self.assertEqual(tree['totals']['pi_api']['calls'], 4)
        self.assertEqual(tree['totals']['pi_api']['unpriced_calls'], 1)
        priced = [r for r in self.build()[0] if r['usd'] is not None]
        self.assertAlmostEqual(tree['totals']['pi_api']['usd_low'], sum(r['usd'] for r in priced))
        self.assertEqual(tree['nodes'][0]['inclusive']['pi_api']['calls'], 4)
        self.assertEqual(tree['post_close']['candidate_totals']['pi_api']['calls'], 1)

    def test_tree_keeps_host_mode_pi_child_unreadable(self):
        host_child = '00000000-0000-4000-8000-00000000000a'
        tree = self.tree([dict(host='pi', event='launch', session_id=host_child, turn=1,
                               parent_session_id='parent', files={'events': '/x'})])
        missing = {i.get('node_id') for i in tree['issues'] if i['kind'] == 'transcript_missing'}
        self.assertEqual(missing, {host_child})
        self.assertEqual(tree['status'], 'partial')


if __name__ == '__main__':
    unittest.main()
