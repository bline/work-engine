"""Usage ledger: claude-sdk (Claude subscription) Pi container turns from role_host's host proxy logs.

Rows follow the producer schema of skills/manual-orchestration/scripts/pi_container.py
(claude_sdk_admitted + request rows matched by n) and its turn-intent files.
"""
import json
import os
import sys
import tempfile
import unittest

sys.path.insert(0, os.path.join(os.path.dirname(__file__), '..'))
import usage_ledger as L  # noqa: E402

SID = '00000000-0000-4000-8000-0000000000c6'
T0 = '2026-11-25T01:00:0{}Z'


def admitted(n, at=0):
    return dict(at=T0.format(at), event='claude_sdk_admitted', provider='claude-sdk', auth_lane='subscription', n=n,
                model='claude-haiku-5-5', effort='medium', system_sha256='s', tools_sha256='t', tool_names=['mcp__pi__read'])


def request(n, at=1, disposition='completed', usage_status='reported', **kw):
    row = dict(at=T0.format(at), event='request', provider='claude-sdk', auth_lane='subscription', n=n,
               model='claude-haiku-5-5', disposition=disposition, http_status=200, response_id=f'msg_{n}',
               response_model='claude-haiku-5-5', billing_verdict={'verdict': 'allowed'}, usage_status=usage_status,
               usage={'input_tokens': 100, 'cache_read_input_tokens': 5, 'cache_creation_input_tokens': 0,
                      'ephemeral_5m_input_tokens': None, 'ephemeral_1h_input_tokens': None, 'output_tokens': 7},
               billing='unknown', quota='unknown', seconds=0.5)
    row.update(kw)
    return row


class ClaudeSdkLedgerTests(unittest.TestCase):
    def setUp(self):
        L.ISSUES.clear()
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.state = os.path.join(self.tmp.name, 'state')
        self.host = os.path.join(self.state, 'pi', SID, 'host')
        os.makedirs(self.host)
        self.ledger = os.path.join(self.state, 'ledger.jsonl')
        self.rows = []

    def turn(self, turn, proxy_rows=None, raw=None, recorded=True):
        log = os.path.join(self.host, f'turn-{turn}.proxy.jsonl')
        if raw is not None or proxy_rows is not None:
            with open(log, 'w') as fh:
                fh.write(raw if raw is not None else ''.join(json.dumps(r) + '\n' for r in proxy_rows))
        with open(os.path.join(self.host, f'turn-{turn}.intent.json'), 'w') as fh:
            json.dump(dict(kind='claude_sdk_turn_intent', session_id=SID, turn=turn, proxy_log=log,
                           container=f'we-pi-{SID}-{turn}'), fh)
        if recorded:
            row = dict(host='pi', event='launch' if turn == 1 else 'deliver', session_id=SID, turn=turn,
                       model='claude-sdk/claude-haiku-5-5', auth_lane='subscription', files=dict(proxy_log=log))
            with open(self.ledger, 'a') as fh:
                fh.write(json.dumps(row) + '\n')
        return log

    def build(self):
        sessions = {}
        rows, cov = L.build_pi(None, None, self.state, sessions=sessions)
        return rows, cov, sessions

    def kinds(self):
        return [i['kind'] for i in L.ISSUES]

    def test_completed_calls_are_subscription_rows_without_dollars_or_api_pool(self):
        self.turn(1, [admitted(1), request(1), admitted(2, 2), request(2, 3)])
        rows, cov, _ = self.build()
        self.assertEqual(len(rows), 2)
        for r in rows:
            self.assertEqual(r['pools'], ['claude_all_models'])
            self.assertNotIn('pi_api', r['pools'])
            self.assertIsNone(r['usd'])
            self.assertIsNone(r['api_equivalent_usd_low'])
            self.assertEqual((r['billing'], r['quota'], r['lane']), ('unknown', 'unknown', 'subscription'))
            self.assertEqual(r['usage']['output_tokens'], 7)
            self.assertEqual(r['claude_sdk']['disposition'], 'completed')
        self.assertEqual(cov['pi_container_turns'], 1)
        self.assertNotIn('pi_claude_sdk_turn_unresolved', self.kinds())

    def test_refused_before_dispatch_is_no_call(self):
        self.turn(1, [dict(at=T0.format(0), event='refused', reason='hosted_tool_refused:web_search')])
        rows, _, _ = self.build()
        self.assertEqual(rows, [])

    def test_dispatched_incomplete_and_unresolved_are_calls_with_unknown_usage(self):
        self.turn(1, [admitted(1), request(1, disposition='upstream_error', usage_status=None, http_status=500,
                                           usage=None), admitted(2, 2)])
        rows, _, _ = self.build()
        self.assertEqual([r['claude_sdk']['disposition'] for r in rows], ['upstream_error', 'dispatch_unknown'])
        for r in rows:
            self.assertEqual(r['usage'], {})
            self.assertEqual(r['claude_sdk']['usage_status'], 'not_established')
        self.assertIn('pi_claude_sdk_turn_unresolved', self.kinds())
        self.assertEqual(self.kinds().count('pi_claude_sdk_usage_unknown'), 2)

    def test_missing_and_corrupt_logs_are_reported(self):
        self.turn(1)  # no proxy log
        log = self.turn(2, raw=json.dumps(admitted(1)) + '\n' + json.dumps(request(1)))  # final row without LF
        rows, cov, _ = self.build()
        self.assertEqual(cov['pi_proxy_logs_missing'], 1)
        self.assertIn('pi_proxy_log_missing', self.kinds())
        self.assertIn('partial_pi_proxy_row', self.kinds())
        # An uncommitted final row is corrupt evidence, stronger than unresolved; neither is zero.
        self.assertIn('pi_claude_sdk_turn_corrupt', self.kinds())
        self.assertEqual([r['claude_sdk']['disposition'] for r in rows], ['dispatch_unknown'])
        self.assertTrue(rows[0]['source'].startswith(log))

    def test_settlement_without_admission_is_corrupt_not_counted(self):
        self.turn(1, [request(1)])
        rows, _, _ = self.build()
        self.assertEqual(rows, [])
        self.assertIn('pi_claude_sdk_turn_corrupt', self.kinds())

    def test_unrecorded_intent_is_discovered_without_a_launch_row(self):
        self.turn(1, [admitted(1)], recorded=False)
        rows, cov, sessions = self.build()
        self.assertEqual(cov.get('pi_unrecorded_turns'), 1)
        self.assertIn('pi_unrecorded_turn', self.kinds())
        self.assertEqual(len(rows), 1)
        self.assertEqual((rows[0]['claude_sdk']['disposition'], rows[0]['claude_sdk']['recorded']),
                         ('dispatch_unknown', False))
        self.assertIn(SID, sessions)

    def test_native_pi_session_of_a_container_session_is_skipped(self):
        """The native Pi/Claude Code join skips sessions role_host accounts for, so a
        duplicate native transcript entry of a container turn is not counted twice."""
        self.turn(1, [admitted(1), request(1)])
        rows, _, sessions = self.build()
        self.assertIn(SID, sessions)
        native = {SID: dict(session_id=SID, path='/x', started='2026-11-25T00:59:00Z', calls=[
            dict(responseId='msg_1', model='claude-haiku-5-5', provider='claude-sdk', ts=T0.format(1))])}
        self.assertEqual(L.pi_native_join([], native, skip=set(sessions)), [])

    def test_rows_are_deduplicated_across_rebuilds_of_the_same_log(self):
        log = self.turn(1, [admitted(1), request(1)])
        seen = set()
        first, _ = L.pi_claude_sdk_turn(log, SID, 1, {}, None, None, seen)
        again, _ = L.pi_claude_sdk_turn(log, SID, 1, {}, None, None, seen)
        self.assertEqual((len(first), len(again)), (1, 0))


if __name__ == '__main__':
    unittest.main()
