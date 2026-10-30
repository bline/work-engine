import json
import os
import sys
import tempfile
import unittest
from unittest.mock import patch

sys.path.insert(0, os.path.dirname(os.path.dirname(__file__)))
import usage_ledger as L

T0 = '2026-01-01T00:00:00Z'
T1 = '2026-01-02T00:00:00Z'
ROOT = '01a18305-58a4-79c0-b319-7d0cf7ab1e8a'
OTHER = '885afb19-09d7-49d8-b268-68b7f6971520'


def record(path, sessions):
    with open(path, 'w') as f:
        json.dump({'scope_budget': {'objective_id': 'obj', 'started_at': T0, 'sessions': sessions, 'bindings': []}}, f)


def rollout(path, sid, calls=(), extra=()):
    # Key order and separators match real rollouts: the ledger's fast path reads the
    # timestamp/type prefix of each line.
    rows = [{'timestamp': T0, 'type': 'session_meta', 'payload': {'id': sid}},
            {'timestamp': T0, 'type': 'turn_context', 'payload': {'model': 'gpt-6.1-sol'}},
            {'timestamp': T0, 'type': 'event_msg', 'payload': {'type': 'thread_settings_applied', 'thread_id': sid,
                                                               'thread_settings': {'service_tier': 'default'}}}]
    for rid, ts, usage in calls:
        rows.append({'timestamp': ts, 'type': 'token_usage_record',
                     'payload': {'thread_id': sid, 'response_id': rid, 'usage': usage}})
    with open(path, 'w') as f:
        for row in rows: f.write(json.dumps(row, separators=(',', ':')) + '\n')
        for line in extra: f.write(line + '\n')


GOOD = {'input_tokens': 100, 'output_tokens': 10}


class IssueScope(unittest.TestCase):
    def tree(self, d, files):
        rec = os.path.join(d, 'record'); record(rec, [{'session_id': ROOT, 'provider': 'codex', 'from': T0}])
        with patch.object(L, 'CODEX_GLOBS', files), patch.object(L, 'CLAUDE_GLOB', os.path.join(d, 'none')), \
                patch.object(L, 'thread_index', return_value={}):
            return L.cost_tree(rec, role_ledger=os.path.join(d, 'absent'))[0]

    def test_outside_session_issue_does_not_make_objective_partial(self):
        with tempfile.TemporaryDirectory() as d:
            root = os.path.join(d, f'rollout-x-{ROOT}.jsonl'); other = os.path.join(d, f'rollout-x-{OTHER}.jsonl')
            rollout(root, ROOT, [('r1', T1, GOOD)])
            rollout(other, OTHER, [('o1', T1, {'input_tokens': 'bad'})])
            tree = self.tree(d, [root, other])
            self.assertEqual(tree['status'], 'complete')
            self.assertIn('invalid_call_usage', tree['outside']['issues'])
            self.assertNotIn('invalid_call_usage', [i['kind'] for i in tree['issues']])
            self.assertEqual(tree['unattributed']['totals']['codex']['calls'], 1)

    def test_member_session_issue_makes_objective_partial(self):
        with tempfile.TemporaryDirectory() as d:
            root = os.path.join(d, f'rollout-x-{ROOT}.jsonl')
            rollout(root, ROOT, [('r1', T1, GOOD), ('r2', T1, {'input_tokens': 'bad'})])
            tree = self.tree(d, [root])
            self.assertEqual(tree['status'], 'partial')
            self.assertIn('invalid_call_usage', [i['kind'] for i in tree['issues']])
            self.assertEqual(tree['outside']['issues'], {})

    def test_issue_without_a_session_stays_in_scope(self):
        with tempfile.TemporaryDirectory() as d:
            root = os.path.join(d, f'rollout-x-{ROOT}.jsonl'); rollout(root, ROOT, [('r1', T1, GOOD)])
            rec = os.path.join(d, 'record'); record(rec, [{'session_id': ROOT, 'provider': 'codex', 'from': T0}])
            def failing_index():
                L.issue('thread_index_unreadable', error='locked'); return {}
            with patch.object(L, 'CODEX_GLOBS', [root]), patch.object(L, 'CLAUDE_GLOB', os.path.join(d, 'none')), \
                    patch.object(L, 'thread_index', side_effect=failing_index):
                tree = L.cost_tree(rec, role_ledger=os.path.join(d, 'absent'))[0]
            self.assertEqual(tree['status'], 'partial')
            self.assertIn('thread_index_unreadable', [i['kind'] for i in tree['issues']])

    def test_outside_unpriced_and_undated_calls_do_not_affect_objective(self):
        with tempfile.TemporaryDirectory() as d:
            root = os.path.join(d, f'rollout-x-{ROOT}.jsonl'); other = os.path.join(d, f'rollout-x-{OTHER}.jsonl')
            rollout(root, ROOT, [('r1', T1, GOOD)])
            rollout(other, OTHER, [('o1', T1, {'input_tokens': 'bad'}), ('o2', 'not-a-time', GOOD)])
            tree = self.tree(d, [root, other])
            self.assertEqual(tree['status'], 'complete')
            self.assertEqual(tree['undated']['counts']['codex'], 0)
            self.assertEqual(tree['undated']['sessions'], [])
            self.assertEqual(tree['outside']['undated_calls'], 1)
            self.assertGreater(tree['window_totals']['codex']['unpriced_calls'], 0)

    def test_member_undated_call_still_counts(self):
        with tempfile.TemporaryDirectory() as d:
            root = os.path.join(d, f'rollout-x-{ROOT}.jsonl')
            rollout(root, ROOT, [('r1', T1, GOOD), ('r2', 'not-a-time', GOOD)])
            tree = self.tree(d, [root])
            self.assertEqual(tree['status'], 'partial')
            self.assertEqual(tree['undated']['counts']['codex'], 1)
            self.assertEqual(tree['outside']['undated_calls'], 0)


if __name__ == '__main__':
    unittest.main()
