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

class BadRecords(unittest.TestCase):
    def test_bad_usage_and_timestamp_do_not_stop_build(self):
        with tempfile.TemporaryDirectory() as d:
            project = os.path.join(d, '.claude', 'projects', 'p'); os.makedirs(project)
            claude = os.path.join(project, 's.jsonl'); codex = os.path.join(d, 'rollout.jsonl')
            rows = [dict(type='assistant', timestamp='broken', sessionId='s', message=dict(id='bad-time', model='claude-opus-5-5', usage=dict(input_tokens=1, cache_creation_input_tokens=0, cache_read_input_tokens=0, output_tokens=1))),
                    dict(type='assistant', timestamp='2026-01-01T01:00:00Z', sessionId='s', message=dict(id='bad-usage', model='claude-opus-5-5', usage=dict(input_tokens='bad', cache_creation_input_tokens=0, cache_read_input_tokens=0, output_tokens=1))),
                    dict(type='assistant', timestamp='2026-01-01T01:00:01Z', sessionId='s', message=dict(id='good', model='claude-opus-5-5', usage=dict(input_tokens=1, cache_creation_input_tokens=0, cache_read_input_tokens=0, output_tokens=1))),
                    dict(type='assistant', timestamp='broken-later', sessionId='s', message=dict(id='good', model='claude-opus-5-5', usage=dict(input_tokens=1, cache_creation_input_tokens=0, cache_read_input_tokens=0, output_tokens=2))),
                    dict(type='assistant', timestamp='2026-01-01T01:00:02Z', sessionId='s', message=dict(id='bad-shape', model='claude-opus-5-5', usage=['bad'])),
                    dict(type='system', subtype='compact_boundary', timestamp='2026-01-01T01:00:03Z', sessionId='s', compactMetadata={'preTokens':'bad'}),
                    dict(type='user', isCompactSummary=True, message={'content':3}),
                    dict(type='cost-state', sessionId='s', modelUsage={'claude-opus-5-5':{'inputTokens':0,'costUSD':'bad'}})]
            with open(claude, 'w') as f:
                for row in rows: f.write(json.dumps(row, separators=(',', ':'))+'\n')
            records = [dict(type='session_meta', timestamp='2026-01-01T00:00:00Z', payload={'id':'t'}),
                       dict(type='token_usage_record', timestamp='bad', payload=dict(thread_id='t', response_id='r1', usage=dict(input_tokens=1, output_tokens=1))),
                       dict(type='token_usage_record', timestamp='2026-01-01T01:00:00Z', payload=dict(thread_id='t', response_id='r2', usage=dict(input_tokens='oops', output_tokens=1))),
                       dict(type='token_usage_record', timestamp='2026-01-01T01:00:01Z', payload=dict(thread_id='t', response_id='r3', usage=dict(input_tokens=1, output_tokens=1)))]
            with open(codex, 'w') as f:
                for row in records: f.write(json.dumps(row, separators=(',', ':'))+'\n')
            out = os.path.join(d, 'out'); args = argparse.Namespace(since='2026-01-01T00:00:00Z', until=None, out=out, no_codex=False, no_claude=False, streams=None)
            L.ISSUES.clear()
            with patch.object(L, 'CODEX_GLOBS', [codex]), patch.object(L, 'CLAUDE_GLOB', claude), patch.object(L, 'thread_index', return_value={}), contextlib.redirect_stdout(io.StringIO()): L.cmd_build(args)
            with open(os.path.join(out, 'calls.jsonl')) as f: calls = {r['call_id']: r for r in map(json.loads, f)}
            self.assertEqual(set(calls), {'bad-time','bad-usage','good','bad-shape','r1','r2','r3'})
            for name in ('bad-time','bad-usage','bad-shape','good','r1','r2'): self.assertIsNone(calls[name]['usd'])
            self.assertIsNone(calls['bad-usage']['weighted']); self.assertIsNone(calls['r2']['weighted'])
            self.assertEqual(calls['bad-usage']['usage']['input_tokens'], 'bad')
            with open(os.path.join(out,'issues.jsonl')) as f: kinds = {r['kind'] for r in map(json.loads,f)}
            self.assertTrue({'invalid_call_timestamp', 'invalid_call_usage', 'invalid_compaction_tokens', 'invalid_compaction_summary'} <= kinds)

class LineageBoundary(unittest.TestCase):
    def setUp(self):
        available = patch.object(L, 'transcript_available', return_value=True)
        available.start(); self.addCleanup(available.stop)
    def test_precedence_recalculates_descendants_and_rejects_cycle(self):
        early = '2026-01-01T01:00:00Z'; late = '2026-01-01T03:00:00Z'
        sb = dict(objective_id='id', started_at=early, sessions=[dict(session_id='root', provider='codex', **{'from':early}), dict(session_id='succ', provider='codex', **{'from':late})],
                  bindings=[dict(session_id='child', provider='codex', parent_session_id='parent'), dict(session_id='parent', provider='codex', parent_session_id='desc')])
        call = dict(provider='codex', thread='desc', ts='2026-01-01T02:00:00Z', model='gpt-6.1-sol', service_tier='default', quota_multiplier=1, usd=1, pools=['codex'])
        with tempfile.TemporaryDirectory() as d:
            record = os.path.join(d,'record'); ledger = os.path.join(d,'ledger')
            with open(record,'w') as f: json.dump({'scope_budget':sb}, f)
            with open(ledger,'w') as f:
                for child, parent in [('child','root'),('desc','child'),('parent','succ')]:
                    f.write(json.dumps(dict(event='launch', session_id=child, parent_session_id=parent, host='codex'))+'\n')
            with patch.object(L,'build_codex', return_value=([call], [], [], 0)), patch.object(L,'build_claude', return_value=([], [], 0)), patch.object(L,'unrecorded_claude', return_value=[]):
                tree, _, _ = L.cost_tree(record, role_ledger=ledger)
        nodes = {n['node_id']:n for n in tree['nodes']}
        self.assertEqual(nodes['child']['parent_node_id'], 'parent')
        self.assertEqual(nodes['parent']['parent_node_id'], 'succ')
        self.assertEqual(nodes['desc']['parent_node_id'], 'child')
        self.assertEqual(tree['totals']['codex']['calls'], 0)
        self.assertEqual(tree['unattributed']['totals']['codex']['calls'], 1)
        self.assertTrue(any(x['kind']=='lineage_cycle' for x in tree['issues']))
        self.assertTrue(any(x['kind']=='multiple_parents' for x in tree['issues']))

class Matrix(unittest.TestCase):
    def setUp(self):
        available = patch.object(L, 'transcript_available', return_value=True)
        available.start(); self.addCleanup(available.stop)
    def test_conservation_every_pool_with_unattributed(self):
        ts = '2026-01-01T01:00:00Z'
        sb = dict(started_at=ts, objective_id='id', sessions=[dict(session_id='root',provider='codex', **{'from':ts})],
                  bindings=[dict(session_id='c',provider='claude',parent_session_id='root')])
        def codex(sid): return dict(provider='codex', thread=sid, ts=ts, model='gpt-6.1-sol', service_tier='default', quota_multiplier=1, usd=1, pools=['codex'])
        def claude(sid, model): return dict(provider='anthropic', session=sid, subagent=False, ts=ts, model=model, speed='standard', usd=2, usd_low=2, usd_high=2, pools=['claude_all_models'] + (['claude_fable'] if model.startswith('claude-fable-') else []))
        with tempfile.TemporaryDirectory() as d:
            record = os.path.join(d,'record')
            with open(record,'w') as f: json.dump({'scope_budget':sb},f)
            with patch.object(L,'build_codex',return_value=([codex('root'),codex('outside')],[],[],0)), patch.object(L,'build_claude',return_value=([claude('c','claude-fable-5-1'),claude('else','claude-fable-5-1'),claude('else','claude-opus-5')],[],0)), patch.object(L,'unrecorded_claude',return_value=[]):
                tree, _, _ = L.cost_tree(record,role_ledger=os.path.join(d,'none'))
        for pool in L.POOLS:
            for key in L.empty_spend():
                self.assertAlmostEqual(tree['totals'][pool][key] + tree['unattributed']['totals'][pool][key], tree['window_totals'][pool][key])
            self.assertGreater(tree['totals'][pool]['calls'],0)
            self.assertGreater(tree['unattributed']['totals'][pool]['calls'],0)

    def test_retired_hours_account_and_coverage_cursor(self):
        # max_hours in an old record raises nothing; elapsed time is past it throughout.
        sb = {'objective_id':'id', 'started_at':'2026-01-01T00:00:00Z', 'limits':{'max_hours':1,'max_weekly_percent':90}, 'cost_limits':[]}
        time_block = dict(basis='model_call_latency', status='complete', window_hours=9.0, model_busy_hours=2.0, model_call_hours=3.0, calls=4, calls_without_latency=0)
        doc = dict(snapshot_at='2026-01-01T09:00:00Z', status='partial', totals=L.pools(), nodes=[], time=time_block)
        quota = [dict(ts='2026-01-01T08:00:00Z', window_minutes=10080, used_percent=95, resets_at=L.ts_of('2026-01-02T00:00:00Z').timestamp()),
                 dict(ts='2026-01-01T07:00:00Z', window_minutes=10080, used_percent=5, resets_at=L.ts_of('2026-01-02T00:00:00Z').timestamp())]
        args = argparse.Namespace(record='x', cursor=None, limit_fractions='0.5,0.8,1.0', role_host_ledger=None)
        def run():
            out = io.StringIO()
            with contextlib.redirect_stdout(out), patch.object(L,'cost_tree',return_value=(doc,quota,sb)): L.cmd_control(args)
            return json.loads(out.getvalue())
        a=run(); self.assertEqual({x['kind'] for x in a['events']}, {'account_ceiling','coverage_changed'})
        self.assertEqual(a['snapshot']['time'], time_block)
        self.assertTrue(next(x for x in a['events'] if x['kind']=='account_ceiling')['observed']['account_wide'])
        args.cursor=a['next_budget_cursor']; self.assertEqual(run()['events'],[])
        doc['status']='complete'; doc['snapshot_at']='2026-01-01T11:00:00Z'
        b=run(); self.assertEqual({x['kind'] for x in b['events']}, {'coverage_changed'})
        args.cursor=b['next_budget_cursor']; self.assertEqual(run()['events'],[])
        # A cursor written by an earlier reader may hold hours states: accepted, dropped, never raised.
        state = json.loads(L.base64.urlsafe_b64decode(b['next_budget_cursor'] + '=' * (-len(b['next_budget_cursor']) % 4)))
        state['states'].update({'["hours_fraction",null,null,0.8]': True, '["hours_reached",null,null,1.0]': False})
        args.cursor = L.base64.urlsafe_b64encode(json.dumps(state).encode()).decode().rstrip('=')
        c=run(); self.assertEqual(c['events'], [])
        kept = json.loads(L.base64.urlsafe_b64decode(c['next_budget_cursor'] + '=' * (-len(c['next_budget_cursor']) % 4)))['states']
        self.assertFalse(any('hours' in key for key in kept))
        import subprocess
        retired = subprocess.run([sys.executable, '-I', L.__file__, 'control', '--record', 'x', '--hours-fractions', '1'],
                                 capture_output=True, text=True)
        self.assertEqual(retired.returncode, 2)
        self.assertIn('--hours-fractions', retired.stderr)

    def test_model_time_is_the_union_of_call_intervals(self):
        since = '2026-01-01T01:00:00Z'
        sb = dict(started_at=since, objective_id='id', sessions=[dict(session_id='root',provider='codex', **{'from':since})],
                  bindings=[dict(session_id='c',provider='claude',parent_session_id='root')])
        def codex(sid, ts, latency): return dict(provider='codex', thread=sid, ts=ts, latency_s=latency, model='gpt-6.1-sol', service_tier='default', quota_multiplier=1, usd=1, pools=['codex'])
        def claude(ts, latency): return dict(provider='anthropic', session='c', subagent=False, ts=ts, latency_s=latency, model='claude-opus-5-5', speed='standard', usd=1, usd_low=1, usd_high=1, pools=['claude_all_models'])
        calls = [codex('root', '2026-01-01T01:10:00Z', 900),     # starts before the window: 01:00-01:10 counts
                 claude('2026-01-01T01:12:00Z', 240),             # 01:08-01:12 overlaps the root call
                 codex('root', '2026-01-01T01:20:00Z', 300),      # 01:15-01:20
                 claude('2026-01-01T01:30:00Z', None),            # no latency: counted, not zero
                 codex('other', '2026-01-01T01:40:00Z', 3000)]    # another objective's session
        with tempfile.TemporaryDirectory() as d:
            record = os.path.join(d,'record')
            with open(record,'w') as f: json.dump({'scope_budget':sb},f)
            with patch.object(L,'build_codex',return_value=([c for c in calls if c['provider']=='codex'],[],[],0)), \
                 patch.object(L,'build_claude',return_value=([c for c in calls if c['provider']=='anthropic'],[],0)):
                tree, _, _ = L.cost_tree(record, until_arg='2026-01-01T02:00:00Z', role_ledger=os.path.join(d,'none'))
            self.assertEqual(tree['time'], dict(basis='model_call_latency', status='partial', window_hours=1.0,
                model_busy_hours=round(17/60, 4), model_call_hours=round(19/60, 4), calls=4, calls_without_latency=1))
            with open(record,'w') as f: json.dump({'scope_budget':dict(sb, sessions=[])},f)
            with patch.object(L,'build_codex',return_value=([],[],[],0)), patch.object(L,'build_claude',return_value=([],[],0)):
                tree, _, _ = L.cost_tree(record, until_arg='2026-01-01T02:00:00Z', role_ledger=os.path.join(d,'none'))
            self.assertEqual((tree['time']['status'], tree['time']['model_busy_hours'], tree['time']['model_call_hours']),
                             ('not_established', None, None))

    def test_build_failure_and_reader_error(self):
        with tempfile.TemporaryDirectory() as d:
            record=os.path.join(d,'record')
            with open(record,'w') as f: json.dump({'scope_budget':{'objective_id':'id','started_at':'2026-01-01T00:00:00Z','sessions':[{'session_id':'a','provider':'codex','from':'2026-01-01T00:00:00Z'}]}},f)
            with patch.object(L,'build_codex',side_effect=OSError('unreadable')):
                tree, quota, _ = L.cost_tree(record)
            self.assertEqual(tree['status'],'not_established')
            self.assertEqual(tree['issues'][0]['kind'],'build_failed')
            self.assertEqual(quota,[])
            with self.assertRaises(OSError): L.cost_tree(os.path.join(d,'missing'))
