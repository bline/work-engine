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

    def test_addendum_20261110_rates(self):
        # Rates from metrics/probes/price-table-addendum-20261110.json.
        u = dict(input_tokens=1000, output_tokens=1000, cache_read_input_tokens=1000000,
                 cache_creation_input_tokens=2000000, ephemeral_5m_input_tokens=1000000,
                 ephemeral_1h_input_tokens=1000000)
        for model, usd in (('claude-fable-5-2', 26.248), ('claude-sonnet-5', 6.712)):
            low, high, unknown = L.native_usd(u, model)
            self.assertAlmostEqual(low, usd)
            self.assertAlmostEqual(high, usd)
            self.assertEqual(unknown, 0)
        # Fable 5.2 has no prompt-length tier.
        big = dict(input_tokens=500000, output_tokens=0, cache_read_input_tokens=0, cache_creation_input_tokens=0)
        self.assertAlmostEqual(L.native_usd(big, 'claude-fable-5-2')[0], 4.0)


class Measurements(unittest.TestCase):
    def test_call_aliases_preserve_cost_and_expose_context(self):
        row = dict(provider='codex', usd=2, quota_multiplier=2.5, weighted=30000,
                   usage=dict(input_tokens=300000, cached_input_tokens=300000, output_tokens=0))
        original = dict(row)
        L.annotate_call_measurements(row)
        self.assertEqual({k: row[k] for k in original}, original)
        self.assertEqual(row['api_equivalent_usd_high'], 2)
        self.assertEqual(row['speed_adjusted_api_usd_high'], 5)
        self.assertEqual(row['input_context_tokens'], 300000)
        self.assertTrue(row['api_long_context_input'])
        row.update(usd=None)
        L.annotate_call_measurements(row)
        self.assertIsNone(row['api_equivalent_usd_high'])
        self.assertIsNone(row['speed_adjusted_api_usd_high'])

    def test_projections_do_not_mutate_aggregates_or_invent_quota(self):
        import copy
        spend = L.pools()
        L.add_spend(spend['codex'], L.call_spend(dict(provider='codex', usd=2, quota_multiplier=None)))
        original = copy.deepcopy(spend)
        view = L.cost_measurements(spend)['codex']
        self.assertEqual(spend, original)
        self.assertEqual((view['speed_adjusted_api_usd']['low'], view['speed_adjusted_api_usd']['high']), (2, 5))
        self.assertEqual(view['estimated_quota_points']['status'], 'not_established')
        self.assertIsNone(view['estimated_quota_points']['high'])
        self.assertIsNone(view['estimated_quota_points']['calibration_version'])
        self.assertFalse(L.pricing_scenario()['subscription_quota'])

    def test_scenario_digest_covers_speed_and_context_rules(self):
        from unittest.mock import patch
        original = L.pricing_scenario()
        with patch.dict(L.QUOTA_MULTIPLIER, priority=3):
            changed = L.pricing_scenario()
            self.assertEqual(changed['rates_sha256'], original['rates_sha256'])
            self.assertNotEqual(changed['scenario_sha256'], original['scenario_sha256'])
        with patch.dict(L.PRICING_RULES, codex_long_input_multiplier=3):
            self.assertNotEqual(L.pricing_scenario()['scenario_sha256'], original['scenario_sha256'])

    def test_account_headroom_fresh_stale_reset_and_missing(self):
        snapshot = '2026-01-01T00:00:00Z'
        q = dict(ts=snapshot, window_minutes=10080, used_percent=70, resets_at=L.ts_of(snapshot).timestamp() + 3600)
        fresh = L.account_quota_observation([q], snapshot)
        self.assertEqual((fresh['status'], fresh['remaining_percent']), ('observed', 30))
        for readings, at, status in [([], snapshot, 'not_established'), ([q], '2026-01-01T00:06:00Z', 'stale'),
                                      ([q], '2026-01-01T02:00:00Z', 'expired')]:
            with self.subTest(status=status):
                result = L.account_quota_observation(readings, at)
                self.assertEqual(result['status'], status)
                self.assertIsNone(result['remaining_percent'])
                self.assertTrue(result['account_wide'])

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
        args = argparse.Namespace(record='x', cursor=None, limit_fractions='0.5,0.8,1.0', role_host_ledger=None)
        def run():
            out = io.StringIO()
            with contextlib.redirect_stdout(out), patch.object(L, 'cost_tree', return_value=(doc, [], sb)):
                L.cmd_control(args)
            return json.loads(out.getvalue())
        first = run(); self.assertIsNone(first['closed_at']); self.assertEqual(len(first['events']), 2)
        args.cursor = first['next_budget_cursor']; self.assertEqual(run()['events'], [])
        spend['codex']['quota_usd_high'] = spend['codex']['quota_usd_low'] = 1.7
        self.assertEqual([x['kind'] for x in run()['events']], ['limit_fraction'])
        sb['objective_id'] = 'another'
        with self.assertRaises(ValueError): run()

    def test_closed_window_and_control(self):
        import argparse, base64, contextlib, io, json, tempfile
        from unittest.mock import patch
        start, close, later = '2026-01-01T00:00:00Z', '2026-01-01T01:00:00Z', '2026-01-01T02:00:00Z'
        sb = dict(started_at=start, closed_at=close, close=dict(at=close, disposition='accepted', reason='done', set_by='user'),
                  objective_id='id', sessions=[dict(session_id='root', provider='codex', **{'from': start})],
                  cost_limits=[dict(pool='codex', marks_usd=[1], max_usd=2, set_by='user')], limits=dict(max_hours=.1))
        calls = [dict(provider='codex', thread='root', parent_thread=None, ts=t, model='gpt-6.1-sol',
                      service_tier=None, quota_multiplier=None, usd=1, pools=['codex']) for t in (close, later)]
        def build(since, until, **kw):
            return ([c for c in calls if L.ts_of(c['ts']) <= until], [], [], 0)
        with tempfile.TemporaryDirectory() as d:
            path = os.path.join(d, 'record')
            with open(path, 'w') as f: json.dump(dict(scope_budget=sb), f)
            with patch.object(L, 'build_codex', side_effect=build), patch.object(L, 'build_claude', return_value=([], [], 0)), patch.object(L, 'unrecorded_claude', return_value=[]):
                tree, _, _ = L.cost_tree(path, role_ledger=os.path.join(d, 'absent'))
                self.assertEqual(tree['totals']['codex']['calls'], 1)
                self.assertEqual(tree['window']['closed_at'], close)
                self.assertEqual(L.cost_tree(path, until_arg=later, role_ledger=os.path.join(d, 'absent'))[0]['totals']['codex']['calls'], 2)
                prior_key = json.dumps(['mark_crossed', 'codex', None, 1, 'user'], separators=(',', ':'))
                def cursor(states):
                    return base64.urlsafe_b64encode(json.dumps(dict(objective_id='id', states=states)).encode()).decode().rstrip('=')
                args = argparse.Namespace(record=path, cursor=cursor({prior_key: True}), limit_fractions='0.5,0.8,1', role_host_ledger=os.path.join(d, 'absent'))
                def run():
                    out = io.StringIO()
                    with contextlib.redirect_stdout(out): L.cmd_control(args)
                    return json.loads(out.getvalue())
                first = run()
                self.assertEqual(first['closed_at'], close)
                self.assertEqual([e['kind'] for e in first['events']], ['objective_closed'])
                self.assertEqual(first['events'][0]['observed']['disposition'], 'accepted')
                states = json.loads(base64.urlsafe_b64decode(first['next_budget_cursor'] + '=' * (-len(first['next_budget_cursor']) % 4)))['states']
                self.assertTrue(states[prior_key])
                args.cursor = first['next_budget_cursor']; self.assertEqual(run()['events'], [])
                args.cursor = cursor({json.dumps(['objective_closed', None, 'bad', 'closed']): True})
                with self.assertRaises(ValueError): run()
    def test_closed_objective_preserves_account_cursor_and_emits_no_new_limits(self):
        import argparse, base64, contextlib, io, json
        from unittest.mock import patch
        close = '2026-01-01T01:00:00Z'
        sb = dict(objective_id='closed-account', started_at='2026-01-01T00:00:00Z',
                  closed_at=close, close=dict(disposition='accepted'),
                  limits=dict(max_weekly_percent=90, max_hours=.1),
                  cost_limits=[dict(pool='codex', max_usd=1, set_by='user')])
        spend = L.pools()
        spend['codex'].update(usd_low=2, usd_high=2, quota_usd_low=2, quota_usd_high=2)
        doc = dict(snapshot_at=close, status='complete', totals=spend, nodes=[])
        prior = {'objective_id': 'closed-account', 'states': {}, 'account_quota_status': 'expired'}
        cursor = base64.urlsafe_b64encode(json.dumps(prior).encode()).decode().rstrip('=')
        args = argparse.Namespace(record='x', cursor=cursor, limit_fractions='1', role_host_ledger=None)
        quota = [dict(ts=close, window_minutes=10080, used_percent=95,
                      resets_at=1767319200, limit_id='codex')]
        out = io.StringIO()
        with contextlib.redirect_stdout(out), patch.object(L, 'cost_tree', return_value=(doc, quota, sb)):
            L.cmd_control(args)
        result = json.loads(out.getvalue())
        self.assertEqual([e['kind'] for e in result['events']], ['objective_closed'])
        self.assertEqual(result['closed_at'], close)
        self.assertIn('measurements', result['snapshot'])
        saved = json.loads(base64.urlsafe_b64decode(result['next_budget_cursor'] + '=' * (-len(result['next_budget_cursor']) % 4)))
        self.assertEqual(saved['account_quota_status'], 'expired')

    def test_closed_real_rollouts_keep_tail_candidates_and_current_account_separate(self):
        import argparse, contextlib, datetime, io, json, tempfile
        from unittest.mock import patch
        now = datetime.datetime.now(datetime.timezone.utc)
        start, close, later = [(now - datetime.timedelta(seconds=s)).isoformat() for s in (180, 120, 60)]
        sb = dict(objective_id='closed-real', started_at=start, closed_at=close,
                  sessions=[dict(session_id='root', provider='codex', **{'from': start})],
                  close=dict(disposition='accepted'), limits={}, cost_limits=[])
        with tempfile.TemporaryDirectory() as d:
            record = os.path.join(d, 'record.json')
            with open(record, 'w') as f: json.dump(dict(scope_budget=sb), f)
            files = []
            for sid, parent, times, reading in (
                    ('root', None, [close, later], 20), ('child', 'root', [later], None),
                    ('unrelated', None, [later], 95)):
                path = os.path.join(d, sid + '.jsonl'); files.append(path)
                rows = [dict(timestamp=start, type='session_meta', payload=dict(id=sid, parent_thread_id=parent)),
                        dict(timestamp=start, type='turn_context', payload=dict(model='gpt-6.1-sol', effort='low')),
                        dict(timestamp=start, type='event_msg', payload=dict(type='thread_settings_applied',
                             thread_id=sid, thread_settings=dict(service_tier='default')))]
                for i, ts in enumerate(times):
                    rows.append(dict(timestamp=ts, type='token_usage_record', payload=dict(thread_id=sid,
                        response_id=sid + str(i), usage=dict(input_tokens=100, output_tokens=10))))
                if reading is not None:
                    rows.append(dict(timestamp=close if sid == 'root' else later, type='event_msg',
                        payload=dict(type='token_count', rate_limits=dict(limit_id='codex', secondary=dict(
                            window_minutes=10080, used_percent=reading, resets_at=(now + datetime.timedelta(days=1)).timestamp())))))
                with open(path, 'w') as f:
                    for row in rows: f.write(json.dumps(row, separators=(',', ':')) + '\n')
            with (patch.object(L, 'CODEX_GLOBS', files), patch.object(L, 'CLAUDE_GLOB', os.path.join(d, 'none')),
                    patch.object(L, 'thread_index', return_value={})):
                tree, _, _ = L.cost_tree(record, role_ledger=os.path.join(d, 'no-ledger'))
                self.assertEqual(tree['status'], 'complete')
                self.assertEqual(tree['totals']['codex']['calls'], 1)
                tail = tree['post_close']
                self.assertEqual(tail['candidate_totals']['codex']['calls'], 2)
                self.assertEqual({s['session_id'] for s in tail['sessions']}, {'root', 'child'})
                self.assertEqual(tail['settlement']['status'], 'not_established')
                self.assertEqual(tail['coverage'], 'complete')
                self.assertEqual(tree['measurements']['account_quota']['used_percent'], 95)
                self.assertEqual(tree['measurements']['account_quota']['status'], 'observed')
                args = argparse.Namespace(record=record, cursor=None, limit_fractions='1',
                                          role_host_ledger=os.path.join(d, 'no-ledger'))
                out = io.StringIO()
                with contextlib.redirect_stdout(out): L.cmd_control(args)
                snapshot = json.loads(out.getvalue())['snapshot']
                self.assertEqual(snapshot['post_close']['candidate_totals']['codex']['calls'], 2)
                self.assertEqual(snapshot['measurements']['account_quota']['used_percent'], 95)
                extended, _, _ = L.cost_tree(record, until_arg=later, role_ledger=os.path.join(d, 'no-ledger'))
                self.assertEqual(extended['totals']['codex']['calls'], 3)
                self.assertEqual(extended['post_close']['candidate_totals']['codex']['calls'], 0)
                self.assertEqual(extended['post_close']['settlement']['status'], 'not_established')

    def test_closed_reader_failure_keeps_tail_unavailable(self):
        import json, tempfile
        from unittest.mock import patch
        with tempfile.TemporaryDirectory() as d:
            record = os.path.join(d, 'record.json')
            with open(record, 'w') as f: json.dump(dict(scope_budget=dict(
                started_at='2026-01-01T00:00:00Z', closed_at='2026-01-01T01:00:00Z')), f)
            with patch.object(L, 'build_codex', side_effect=OSError('reader unavailable')):
                tree, _, _ = L.cost_tree(record)
        self.assertEqual(tree['status'], 'not_established')
        self.assertEqual(tree['post_close']['coverage'], 'not_established')
        self.assertIsNone(tree['post_close']['candidate_totals'])

    def test_cost_and_account_controls_are_independent(self):
        import argparse, contextlib, io, json
        from unittest.mock import patch
        at = '2026-01-01T00:00:00Z'
        sb = dict(objective_id='one', cost_limits=[dict(pool='codex', max_usd=10, set_by='user')],
                  limits=dict(max_weekly_percent=90))
        args = argparse.Namespace(record='x', cursor=None, limit_fractions='1', role_host_ledger=None)
        for dollars, percent, expected in [(1, 95, ['account_ceiling']), (12, 20, ['limit_reached']), (1, 20, [])]:
            with self.subTest(dollars=dollars, percent=percent):
                spend = L.pools(); spend['codex'].update(usd_low=dollars, usd_high=dollars,
                                                       quota_usd_low=dollars, quota_usd_high=dollars)
                doc = dict(snapshot_at=at, status='complete', totals=spend, nodes=[])
                q = [dict(ts='2025-12-31T23:59:59Z', window_minutes=10080, used_percent=percent, resets_at=L.ts_of(at).timestamp()+3600),
                     dict(ts=at, window_minutes=10080, used_percent=99, resets_at=L.ts_of(at).timestamp()+3600, limit_id='other')]
                out = io.StringIO()
                with contextlib.redirect_stdout(out), patch.object(L, 'cost_tree', return_value=(doc, q, sb)):
                    L.cmd_control(args)
                result = json.loads(out.getvalue())
                self.assertEqual([e['kind'] for e in result['events']], expected)
                self.assertEqual(result['snapshot']['totals'], spend)
                self.assertEqual(result['snapshot']['cost_controls'][0]['max_usd'], 10)
                self.assertEqual(result['snapshot']['measurements']['account_quota']['headroom_to_ceiling_percent'], 90-percent)

    def test_unknown_account_headroom_is_observed_once_and_old_cursor_works(self):
        import argparse, base64, contextlib, io, json
        from unittest.mock import patch
        sb = dict(objective_id='one', cost_limits=[], limits=dict(max_weekly_percent=90))
        doc = dict(snapshot_at='2026-01-01T00:00:00Z', status='complete', totals=L.pools(), nodes=[])
        args = argparse.Namespace(record='x', cursor=None, limit_fractions='1', role_host_ledger=None)
        def run():
            out = io.StringIO()
            with contextlib.redirect_stdout(out), patch.object(L, 'cost_tree', return_value=(doc, [], sb)):
                L.cmd_control(args)
            return json.loads(out.getvalue())
        # A cursor from before measurement metadata existed is still accepted.
        args.cursor = base64.urlsafe_b64encode(json.dumps(dict(objective_id='one', states={})).encode()).decode().rstrip('=')
        first = run()
        self.assertEqual([e['kind'] for e in first['events']], ['coverage_changed'])
        self.assertEqual(first['events'][0]['observed']['account_quota']['status'], 'not_established')
        self.assertIsNone(first['snapshot']['measurements']['account_quota']['remaining_percent'])
        args.cursor = first['next_budget_cursor']
        self.assertEqual(run()['events'], [])

    def test_idle_and_resume_do_not_wake_budget_listener(self):
        import argparse, contextlib, io, json
        from unittest.mock import patch
        at = '2026-01-01T00:00:00Z'
        sb = dict(objective_id='one', cost_limits=[], limits=dict(max_weekly_percent=90))
        doc = dict(snapshot_at=at, status='complete', totals=L.pools(), nodes=[])
        q = [dict(ts=at, window_minutes=10080, used_percent=20, resets_at=L.ts_of(at).timestamp()+3600)]
        args = argparse.Namespace(record='x', cursor=None, limit_fractions='1', role_host_ledger=None)
        def run():
            out = io.StringIO()
            with contextlib.redirect_stdout(out), patch.object(L, 'cost_tree', return_value=(doc, q, sb)):
                L.cmd_control(args)
            result = json.loads(out.getvalue()); args.cursor = result['next_budget_cursor']; return result
        self.assertEqual(run()['events'], [])
        doc['snapshot_at'] = '2026-01-01T00:06:00Z'
        stale = run()
        self.assertEqual(stale['events'], [])
        self.assertIsNone(stale['snapshot']['measurements']['account_quota']['remaining_percent'])
        q[0]['ts'] = doc['snapshot_at']
        self.assertEqual(run()['events'], [])
        doc['snapshot_at'] = '2026-01-01T02:00:00Z'
        expired = run()
        self.assertEqual([e['kind'] for e in expired['events']], ['coverage_changed'])
        self.assertFalse(any(e['kind'] == 'account_ceiling' for e in expired['events']))

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
