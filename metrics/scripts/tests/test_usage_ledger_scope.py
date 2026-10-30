import argparse
import contextlib
import io
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



def assistant(sid, mid, ts, input_tokens):
    return json.dumps({'type': 'assistant', 'timestamp': ts, 'sessionId': sid,
                       'message': {'id': mid, 'model': 'claude-opus-5-5',
                                   'usage': {'input_tokens': input_tokens, 'output_tokens': 10,
                                             'cache_creation_input_tokens': 0, 'cache_read_input_tokens': 0}}})


class ClaudeScope(unittest.TestCase):
    """Review finding 1: a transcript can hold several sessions' rows (copied history)."""

    def run_tree(self, d, transcript):
        rec = os.path.join(d, 'record'); record(rec, [{'session_id': ROOT, 'provider': 'claude', 'from': T0}])
        with open(transcript, 'w') as f:
            f.write(assistant(OTHER, 'm-other', T1, 100) + '\n')
            f.write(assistant(ROOT, 'm-root', T1, 100) + '\n')
            f.write(assistant(ROOT, 'm-root', T1, 200) + '\n')
        with patch.object(L, 'HOME', d), patch.object(L, 'CODEX_GLOBS', []), \
                patch.object(L, 'CLAUDE_GLOB', os.path.join(d, '.claude', 'projects', '**', '*.jsonl')), \
                patch.object(L, 'thread_index', return_value={}):
            return L.cost_tree(rec, role_ledger=os.path.join(d, 'absent'))[0]

    def test_member_issue_in_main_transcript_with_outside_row_first(self):
        with tempfile.TemporaryDirectory() as d:
            project = os.path.join(d, '.claude', 'projects', 'p'); os.makedirs(project)
            tree = self.run_tree(d, os.path.join(project, ROOT + '.jsonl'))
            self.assertEqual(tree['status'], 'partial')
            self.assertIn('conflicting_native_counter', [i['kind'] for i in tree['issues']])
            self.assertNotIn('conflicting_native_counter', tree['outside']['issues'])

    def test_member_issue_in_subagent_transcript_with_outside_row_first(self):
        with tempfile.TemporaryDirectory() as d:
            project = os.path.join(d, '.claude', 'projects', 'p'); os.makedirs(os.path.join(project, ROOT, 'subagents'))
            with open(os.path.join(project, ROOT + '.jsonl'), 'w') as f: f.write(assistant(ROOT, 'm-main', T1, 100) + '\n')
            tree = self.run_tree(d, os.path.join(project, ROOT, 'subagents', 'agent-x.jsonl'))
            self.assertEqual(tree['status'], 'partial')
            self.assertIn('conflicting_native_counter', [i['kind'] for i in tree['issues']])


class ScopeBoundaries(unittest.TestCase):
    """Review finding 2: unmapped files, range-only outside calls, early shape, control coverage."""

    def test_unmapped_file_without_session_id_stays_in_scope(self):
        with tempfile.TemporaryDirectory() as d:
            root = os.path.join(d, f'rollout-x-{ROOT}.jsonl'); rollout(root, ROOT, [('r1', T1, GOOD)])
            rec = os.path.join(d, 'record'); record(rec, [{'session_id': ROOT, 'provider': 'codex', 'from': T0}])
            def index_with_issue():
                L.issue('malformed_session_metadata', path=os.path.join(d, 'unknown.jsonl')); return {}
            with patch.object(L, 'CODEX_GLOBS', [root]), patch.object(L, 'CLAUDE_GLOB', os.path.join(d, 'none')), \
                    patch.object(L, 'thread_index', side_effect=index_with_issue):
                tree = L.cost_tree(rec, role_ledger=os.path.join(d, 'absent'))[0]
            self.assertEqual(tree['status'], 'partial')
            self.assertIn('malformed_session_metadata', [i['kind'] for i in tree['issues']])

    def test_outside_range_only_call_leaves_objective_complete(self):
        with tempfile.TemporaryDirectory() as d:
            root = os.path.join(d, f'rollout-x-{ROOT}.jsonl'); other = os.path.join(d, f'rollout-x-{OTHER}.jsonl')
            rollout(root, ROOT, [('r1', T1, GOOD)])
            rows = [{'timestamp': T0, 'type': 'session_meta', 'payload': {'id': OTHER}},
                    {'timestamp': T0, 'type': 'turn_context', 'payload': {'model': 'gpt-6.1-sol'}},
                    {'timestamp': T1, 'type': 'token_usage_record', 'payload': {'thread_id': OTHER, 'response_id': 'o1', 'usage': GOOD}}]
            with open(other, 'w') as f:
                for row in rows: f.write(json.dumps(row, separators=(',', ':')) + '\n')
            tree = IssueScope().tree(d, [root, other])
            w = tree['window_totals']['codex']
            self.assertNotEqual(w['quota_usd_low'], w['quota_usd_high'])
            self.assertEqual(tree['status'], 'complete')

    def test_not_established_document_has_outside(self):
        with tempfile.TemporaryDirectory() as d:
            rec = os.path.join(d, 'record'); record(rec, [])
            with patch.object(L, 'CODEX_GLOBS', []), patch.object(L, 'CLAUDE_GLOB', os.path.join(d, 'none')), \
                    patch.object(L, 'thread_index', return_value={}):
                tree = L.cost_tree(rec, role_ledger=os.path.join(d, 'absent'))[0]
            self.assertEqual(tree['status'], 'not_established')
            self.assertEqual(tree['outside'], {'issues': {}, 'undated_calls': 0})

    def test_control_coverage_ignores_outside_issues(self):
        with tempfile.TemporaryDirectory() as d:
            root = os.path.join(d, f'rollout-x-{ROOT}.jsonl'); other = os.path.join(d, f'rollout-x-{OTHER}.jsonl')
            rollout(root, ROOT, [('r1', T1, GOOD)]); rollout(other, OTHER, [('o1', T1, {'input_tokens': 'bad'})])
            rec = os.path.join(d, 'record'); record(rec, [{'session_id': ROOT, 'provider': 'codex', 'from': T0}])
            args = argparse.Namespace(record=rec, cursor=None, limit_fractions='0.5,0.8,1.0', hours_fractions='0.8,1.0',
                                      role_host_ledger=os.path.join(d, 'absent'))
            out = io.StringIO()
            with patch.object(L, 'CODEX_GLOBS', [root, other]), patch.object(L, 'CLAUDE_GLOB', os.path.join(d, 'none')), \
                    patch.object(L, 'thread_index', return_value={}), contextlib.redirect_stdout(out):
                L.cmd_control(args)
            doc = json.loads(out.getvalue())
            self.assertEqual(doc['snapshot']['status'], 'complete')
            self.assertNotIn('coverage_changed', [e['kind'] for e in doc['events']])

    def test_build_failure_document_has_outside(self):
        with tempfile.TemporaryDirectory() as d:
            rec = os.path.join(d, 'record'); record(rec, [{'session_id': ROOT, 'provider': 'codex', 'from': T0}])
            with patch.object(L, 'build_codex', side_effect=OSError('unreadable rollouts')), \
                    patch.object(L, 'CLAUDE_GLOB', os.path.join(d, 'none')), patch.object(L, 'thread_index', return_value={}):
                tree = L.cost_tree(rec, role_ledger=os.path.join(d, 'absent'))[0]
            self.assertEqual(tree['status'], 'not_established')
            self.assertIn('build_failed', [i['kind'] for i in tree['issues']])
            self.assertEqual(tree['outside'], {'issues': {}, 'undated_calls': 0})


if __name__ == '__main__':
    unittest.main()
