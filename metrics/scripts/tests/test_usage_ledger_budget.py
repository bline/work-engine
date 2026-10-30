import os
import sys
import unittest

sys.path.insert(0, os.path.dirname(os.path.dirname(__file__)))
import usage_ledger as L


class Pricing(unittest.TestCase):
    def test_inconsistent_split_is_range(self):
        L.ISSUES.clear()
        u = dict(input_tokens=0, output_tokens=0, cache_read_input_tokens=0,
                 cache_creation_input_tokens=0, ephemeral_1h_input_tokens=100)
        low, high, _ = L.native_usd(u, 'claude-opus-5-5')
        self.assertEqual((low, high), (0, .0008))
        self.assertEqual(L.ISSUES[-1]['kind'], 'inconsistent_cache_split')

    def test_haiku_boundary(self):
        u = dict(input_tokens=100000, output_tokens=0, cache_read_input_tokens=0, cache_creation_input_tokens=0)
        self.assertAlmostEqual(L.native_usd(u, 'claude-haiku-5-5')[0], .01)
        u['input_tokens'] += 1
        self.assertAlmostEqual(L.native_usd(u, 'claude-haiku-5-5')[0], .0500005)

class Tree(unittest.TestCase):
    def setUp(self):
        from unittest.mock import patch
        self.available = patch.object(L, 'transcript_available', return_value=True)
        self.available.start(); self.addCleanup(self.available.stop)
    def test_lineage_conservation_and_successor_boundary(self):
        import json
        import tempfile
        from unittest.mock import patch
        def call(sid, ts, parent=None, model='gpt-6.1-sol'):
            return dict(provider='codex', thread=sid, parent_thread=parent, ts=ts, model=model,
                        service_tier=None, quota_multiplier=None, usd=1, pools=['codex'])
        calls = [call('root', '2026-01-01T01:00:00Z'), call('kid', '2026-01-01T01:00:00Z', 'root'),
                 call('successor', '2026-01-01T01:00:00Z')]
        sb = dict(started_at='2026-01-01T00:00:00Z', objective_id='id', sessions=[
            dict(session_id='root', provider='codex', **{'from':'2026-01-01T00:00:00Z'}),
            dict(session_id='successor', provider='codex', **{'from':'2026-01-01T02:00:00Z'})], bindings=[])
        with tempfile.TemporaryDirectory() as d:
            path = os.path.join(d, 'record'); open(path, 'w').write(json.dumps(dict(scope_budget=sb)))
            with patch.object(L, 'build_codex', side_effect=lambda *args, **kw: (kw['metadata'].update({'kid': {'parent_thread_id': 'root'}}) or (calls, [], [], 0))), patch.object(L, 'build_claude', return_value=([], [], 0)), patch.object(L, 'unrecorded_claude', return_value=[]):
                tree, _, _ = L.cost_tree(path, role_ledger=os.path.join(d, 'absent'))
        self.assertEqual(tree['totals']['codex']['calls'], 2)
        self.assertEqual(tree['unattributed']['totals']['codex']['calls'], 1)
        self.assertEqual(tree['window_totals']['codex']['calls'], 3)
        self.assertEqual(tree['totals']['codex']['quota_usd_high'], 5)
        self.assertEqual(tree['status'], 'partial')
        self.assertEqual(next(n for n in tree['nodes'] if n['node_id']=='kid')['link'], 'codex_parent')

    def test_control_cursor(self):
        import argparse
        import contextlib
        import io
        import json
        from unittest.mock import patch
        sb = {'objective_id':'one', 'cost_limits':[{'pool':'codex', 'marks_usd':[1], 'max_usd':2, 'set_by':'user'}], 'limits':{}}
        spend = L.pools(); spend['codex']['quota_usd_high'] = spend['codex']['quota_usd_low'] = 1.2
        doc = dict(snapshot_at='2026-01-01T00:00:00Z', status='complete', totals=spend, nodes=[])
        args = argparse.Namespace(record='x', cursor=None, limit_fractions='0.5,0.8,1.0', hours_fractions='0.8,1.0', role_host_ledger=None)
        def run():
            out = io.StringIO()
            with contextlib.redirect_stdout(out), patch.object(L, 'cost_tree', return_value=(doc, [], sb)):
                L.cmd_control(args)
            return json.loads(out.getvalue())
        first = run(); self.assertEqual(len(first['events']), 2)
        args.cursor = first['next_budget_cursor']; self.assertEqual(run()['events'], [])
        spend['codex']['quota_usd_high'] = spend['codex']['quota_usd_low'] = 1.7
        self.assertEqual([x['kind'] for x in run()['events']], ['limit_fraction'])
        sb['objective_id'] = 'another'
        with self.assertRaises(ValueError): run()

    def test_all_links_pools_and_statuses(self):
        import json
        import tempfile
        from unittest.mock import patch
        ts = '2026-01-01T01:00:00Z'
        sb = dict(started_at='2026-01-01T00:00:00Z', objective_id='id', sessions=[dict(session_id='root', provider='codex', **{'from':ts})], bindings=[dict(session_id='bound', provider='claude', parent_session_id='root')])
        def claude(sid, agent=None):
            return dict(provider='anthropic', session=sid, subagent=bool(agent), agent=agent, agent_type=None,
                        ts=ts, model='claude-fable-5-1', speed='standard', usd=1, usd_low=1, usd_high=1,
                        pools=['claude_all_models', 'claude_fable'])
        rows = [claude('bound'), claude('role'), claude('bound', 'agent-a')]
        with tempfile.TemporaryDirectory() as d:
            path = os.path.join(d,'record'); ledger = os.path.join(d,'ledger')
            with open(path, 'w') as f: json.dump(dict(scope_budget=sb), f)
            with open(ledger,'w') as f:
                f.write(json.dumps(dict(event='launch', session_id='role', parent_session_id='root', host='claude', name='worker'))+'\n')
                f.write(json.dumps(dict(event='launch', session_id='bound', parent_session_id='role', host='claude'))+'\n')
            with patch.object(L, 'build_codex', return_value=([], [], [], 0)), patch.object(L, 'build_claude', return_value=(rows, [], 0)), patch.object(L, 'unrecorded_claude', return_value=[]):
                tree, _, _ = L.cost_tree(path, role_ledger=ledger)
                links = {n['node_id']:n['link'] for n in tree['nodes']}
                self.assertEqual(links['bound'], 'binding')
                self.assertEqual(links['role'], 'role_host')
                self.assertEqual(links['bound/agent:agent-a'], 'claude_subagent')
                self.assertTrue(any(i['kind']=='multiple_parents' for i in tree['issues']))
                self.assertEqual(tree['totals']['claude_fable']['calls'], 3)
                self.assertEqual(tree['totals']['claude_all_models']['calls'], 3)
                self.assertEqual(tree['status'], 'partial')
                sb['sessions'] = []
                with open(path,'w') as f: json.dump(dict(scope_budget=sb), f)
                self.assertEqual(L.cost_tree(path, role_ledger=ledger)[0]['status'], 'not_established')
                sb['sessions'] = [dict(session_id='root', provider='codex', **{'from':ts})]; sb['bindings'] = []
                with open(path,'w') as f: json.dump(dict(scope_budget=sb), f)
                self.assertEqual(L.cost_tree(path, role_ledger=os.path.join(d,'missing'))[0]['status'], 'complete')

    def test_compaction_estimate_and_fast_call(self):
        import json
        import tempfile
        from unittest.mock import patch
        with tempfile.TemporaryDirectory() as d:
            project = os.path.join(d, '.claude', 'projects', 'p'); os.makedirs(project)
            path = os.path.join(project, 's.jsonl')
            u = dict(input_tokens=0, cache_creation_input_tokens=0, cache_read_input_tokens=0, output_tokens=1, speed='fast')
            rows = [dict(type='assistant', timestamp='2026-01-01T00:00:00Z', sessionId='s', message=dict(id='m', model='claude-opus-5-5', usage=u)),
                    dict(type='system', subtype='compact_boundary', timestamp='2026-01-01T00:00:01Z', sessionId='s', compactMetadata=dict(preTokens=1000)),
                    dict(type='user', isCompactSummary=True, message=dict(content='abcdef'))]
            with open(path, 'w') as f:
                for x in rows: f.write(json.dumps(x, separators=(',', ':'))+'\n')
            with patch.object(L, 'CLAUDE_GLOB', path):
                L.ISSUES.clear()
                calls, _, _ = L.build_claude(None, None)
                comp = L.unrecorded_claude(None, None)
            self.assertIsNone(calls[0]['usd'])
            self.assertTrue(any(x['kind']=='fast_mode_unpriced' for x in L.ISSUES))
            self.assertAlmostEqual(comp[0]['usd_low'], .0002)
            self.assertAlmostEqual(comp[0]['usd_high'], .00804)
