import json
import contextlib
import io
import pathlib
import sys
import tempfile
import unittest

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parents[1] / 'scripts'))
import role_host

SID = '00000000-0000-0000-0000-000000000001'
OTHER = '00000000-0000-0000-0000-000000000002'


class SettingsTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.path = pathlib.Path(self.temp.name) / 'rollout.jsonl'

    def write(self, records):
        self.path.write_text('\n'.join(json.dumps(r) for r in records) + '\n')

    def applied(self, tier, sid=SID):
        return {'type': 'event_msg', 'timestamp': '2026-10-28T20:00:00Z', 'payload': {
            'type': 'thread_settings_applied', 'thread_id': sid,
            'thread_settings': {'model': 'gpt-6.1-sol', 'reasoning_effort': 'xhigh', 'service_tier': tier}}}

    def test_latest_bound_setting_preserves_fast_explicitly(self):
        self.write([{'type': 'session_meta', 'payload': {'id': SID}}, self.applied('default'),
                    self.applied('priority'), self.applied('default', OTHER)])
        got = role_host.codex_settings(SID, self.path)
        self.assertEqual(got['service_tier'], 'priority')
        self.assertEqual(got['source']['line'], 3)
        params, effort = role_host.Codex.thread_options(got['successor_host_args'])
        self.assertEqual(params['config']['service_tier'], 'fast')
        self.assertEqual(params['model'], 'gpt-6.1-sol')
        self.assertEqual(effort, 'xhigh')

    def test_unknown_tier_remains_observed_without_guessing_launch(self):
        for tier in ['future-tier', [], {}, 17]:
            with self.subTest(tier=tier):
                self.write([{'type': 'session_meta', 'payload': {'id': SID}}, self.applied(tier)])
                got = role_host.codex_settings(SID, self.path)
                self.assertEqual(got['service_tier'], tier)
                self.assertIsNone(got['successor_host_args'])
                self.assertEqual(got['preservation'], 'not_established')

    def test_parent_rollout_cannot_supply_child_settings(self):
        self.write([{'type': 'session_meta', 'payload': {'id': OTHER}}, self.applied('priority', OTHER)])
        with self.assertRaises(role_host.HostError) as raised:
            role_host.codex_settings(SID, self.path)
        self.assertEqual(raised.exception.status, 'identity_mismatch')

    def test_missing_or_incomplete_evidence_does_not_fall_back_to_config(self):
        for records in ([{'type': 'session_meta', 'payload': {'id': SID}}], []):
            self.write(records)
            with self.assertRaises(role_host.HostError):
                role_host.codex_settings(SID, self.path)
        self.path.write_text(json.dumps({'type': 'session_meta', 'payload': {'id': SID}}) + '\n{')
        with self.assertRaises(role_host.HostError):
            role_host.codex_settings(SID, self.path)

    def test_valid_json_with_invalid_record_shapes_is_unestablished(self):
        meta = {'type': 'session_meta', 'payload': {'id': SID}}
        for records in ([[]], [meta, []], [meta, {'type': 'event_msg', 'payload': []}],
                        [meta, {'type': 'event_msg', 'payload': {'type': 'thread_settings_applied',
                                                                'thread_id': SID, 'thread_settings': []}}]):
            with self.subTest(records=records):
                self.write(records)
                with self.assertRaises(role_host.HostError) as raised:
                    role_host.codex_settings(SID, self.path)
                self.assertEqual(raised.exception.code, role_host.NOT_ESTABLISHED)

    def test_incomplete_launch_settings_do_not_generate_partial_defaults(self):
        for missing in ['model', 'reasoning_effort', 'service_tier']:
            event = self.applied('default')
            del event['payload']['thread_settings'][missing]
            self.write([{'type': 'session_meta', 'payload': {'id': SID}}, event])
            self.assertIsNone(role_host.codex_settings(SID, self.path)['successor_host_args'])

    def test_cli_rejects_wrong_host_and_invalid_id(self):
        for host, sid in [('claude', SID), ('codex', 'not-a-uuid')]:
            with self.subTest(host=host, sid=sid), contextlib.redirect_stderr(io.StringIO()):
                code = role_host.main(['--state', self.temp.name, 'settings', host, sid, '--rollout', str(self.path)])
                self.assertEqual(code, role_host.USAGE)

    def test_cli_reports_malformed_evidence_with_documented_exit(self):
        self.write([[]])
        errors = io.StringIO()
        with contextlib.redirect_stderr(errors):
            code = role_host.main(['--state', self.temp.name, 'settings', 'codex', SID, '--rollout', str(self.path)])
        self.assertEqual(code, role_host.NOT_ESTABLISHED)
        self.assertEqual(json.loads(errors.getvalue())['status'], 'settings_not_established')


if __name__ == '__main__':
    unittest.main()
