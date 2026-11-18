"""Pi sessions outside role_host: Pi's own session files joined to Claude Code transcripts."""
import json
import os
import sys
import tempfile
import unittest
from unittest.mock import patch

sys.path.insert(0, os.path.dirname(os.path.dirname(__file__)))
import usage_ledger as L

T0 = '2026-01-01T00:00:00Z'
ROOT = '01a18305-58a4-79c0-b319-7d0cf7ab1e8a'
CHILD = '01a18305-9999-79c0-b319-7d0cf7ab1e8a'
STRAY = '01a18305-7777-79c0-b319-7d0cf7ab1e8a'
CC_ROOT = '885afb19-09d7-49d8-b268-68b7f6971520'
CC_CHILD = '885afb19-1111-49d8-b268-68b7f6971520'
CC_STRAY = '885afb19-2222-49d8-b268-68b7f6971520'


def at(minute):
    return f'2026-01-01T00:{minute:02d}:00.000Z'


def usage(i, o, cr=0, cw=0):
    return {'input_tokens': i, 'output_tokens': o, 'cache_read_input_tokens': cr, 'cache_creation_input_tokens': cw,
            'cache_creation': {'ephemeral_5m_input_tokens': 0, 'ephemeral_1h_input_tokens': cw}}


def cc(sid, mid, ts, u, model='claude-opus-5-5'):
    return json.dumps({'type': 'assistant', 'timestamp': ts, 'sessionId': sid,
                       'message': {'id': mid, 'model': model, 'usage': u}})


def pi_msg(rid, ts, provider, i, o, cr=0, cw=0, w1h=None, model='claude-opus-5-5', entry=None):
    u = {'input': i, 'output': o, 'cacheRead': cr, 'cacheWrite': cw, 'totalTokens': i + o + cr + cw}
    if w1h is not None: u['cacheWrite1h'] = w1h
    m = {'role': 'assistant', 'provider': provider, 'model': model, 'usage': u, 'stopReason': 'toolUse'}
    if rid: m['responseId'] = rid
    return json.dumps({'type': 'message', 'id': entry or (rid or 'e')[-8:], 'timestamp': ts, 'message': m})


def auth(rid, billing_class, **overrides):
    data = dict(responseId=rid, provider='anthropic', model='claude-opus-5-5', billing_class=billing_class,
                confidence='configuration_snapshot', captured_at=at(4), source=None)
    data.update(overrides)
    return json.dumps(dict(type='custom', customType='auth-provenance', data=data))


def write(path, lines):
    os.makedirs(os.path.dirname(path), exist_ok=True)
    with open(path, 'w') as f: f.write(''.join(x + '\n' for x in lines))


class PiNative(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory(); self.addCleanup(self.tmp.cleanup)
        d = self.d = self.tmp.name
        self.project = os.path.join(d, '.claude', 'projects', 'p')
        self.pi = os.path.join(d, '.pi', 'agent', 'sessions', '--p--')
        self.root_file = os.path.join(self.pi, f'2026-01-01T00-00-00-000Z_{ROOT}.jsonl')
        # Claude Code: a matched call, a continuation folded into the next call, synthesized history.
        write(os.path.join(self.project, CC_ROOT + '.jsonl'), [
            cc(CC_ROOT, 'msg_pi_0_885afb19', at(0), {'input_tokens': 0, 'output_tokens': 0}),
            cc(CC_ROOT, 'msg_A', at(1), usage(10, 100, cw=1000)),
            cc(CC_ROOT, 'msg_B0', at(2), usage(5, 50, cr=1000)),
            cc(CC_ROOT, 'msg_B', at(3), usage(5, 60, cr=1000)),
        ])
        write(os.path.join(self.project, CC_CHILD + '.jsonl'), [cc(CC_CHILD, 'msg_C', at(5), usage(1, 10))])
        write(os.path.join(self.project, CC_STRAY + '.jsonl'), [cc(CC_STRAY, 'msg_S', at(6), usage(1, 10))])

    def root(self, b_usage=(10, 110, 2000, 0), provenance=()):
        write(self.root_file, [
            json.dumps({'type': 'session', 'version': 3, 'id': ROOT, 'timestamp': at(0), 'cwd': '/p'}),
            pi_msg('msg_A', at(1), 'claude-sdk', 10, 100, cw=1000, w1h=1000),
            pi_msg('msg_B', at(3), 'claude-sdk', *b_usage),
            pi_msg('msg_D', at(4), 'anthropic', 100, 10, w1h=0),                 # direct: in no transcript
            pi_msg('msg_E', at(4), 'claude-sdk', 1, 1000, model='claude-fable-5-1'),  # aborted: no transcript row
            pi_msg(None, at(4), 'claude-sdk', 0, 0),                            # error turn: no usage
            *provenance,
        ])

    def child(self):
        write(os.path.join(self.root_file[:-6], 'run-id', 'run-0', 'session.jsonl'), [
            json.dumps({'type': 'session', 'version': 3, 'id': CHILD, 'timestamp': at(5), 'parentSession': self.root_file}),
            pi_msg('msg_A', at(1), 'claude-sdk', 10, 100, cw=1000, w1h=1000),  # copied history: the root's call
            pi_msg('msg_C', at(5), 'claude-sdk', 1, 10),
            pi_msg('msg_F', at(5), 'openai-codex', 1, 10, model='gpt-6.1-sol'),
        ])
        write(os.path.join(self.pi, f'2026-01-01T00-06-00-000Z_{STRAY}.jsonl'), [
            json.dumps({'type': 'session', 'version': 3, 'id': STRAY, 'timestamp': at(6)}),
            pi_msg('msg_S', at(6), 'claude-sdk', 1, 10)])

    def tree(self, sessions, bindings=()):
        rec = os.path.join(self.d, 'record.json')
        with open(rec, 'w') as f:
            json.dump({'scope_budget': {'objective_id': 'o', 'started_at': T0, 'sessions': sessions,
                                        'bindings': list(bindings)}}, f)
        with patch.object(L, 'HOME', self.d), patch.object(L, 'CODEX_GLOBS', []), \
                patch.object(L, 'CLAUDE_GLOB', os.path.join(self.d, '.claude', 'projects', '**', '*.jsonl')), \
                patch.object(L, 'PI_SESSION_GLOB', os.path.join(self.d, '.pi', 'agent', 'sessions', '**', '*.jsonl')), \
                patch.object(L, 'thread_index', return_value={}):
            return L.cost_tree(rec, '2026-01-01T00:00:00Z', '2026-01-01T01:00:00Z', role_ledger=os.path.join(self.d, 'absent'))[0]

    def test_root_children_folds_and_pools(self):
        self.root(); self.child()
        tree = self.tree([{'session_id': ROOT, 'provider': 'pi', 'from': T0}])
        nodes = {n['node_id']: n for n in tree['nodes']}
        self.assertEqual(set(nodes), {ROOT, CHILD})
        self.assertEqual((nodes[CHILD]['link'], nodes[CHILD]['parent_node_id']), ('pi_parent', ROOT))
        root = nodes[ROOT]['direct']
        # msg_A, msg_B0 + msg_B (one folded turn), and msg_E from Pi's usage; synthesized history is not a call.
        self.assertEqual(root['claude_all_models']['calls'], 4)
        self.assertEqual(root['claude_fable']['calls'], 1)
        self.assertEqual((root['claude_extra_usage']['calls'], root['pi_api']['calls']), (1, 1))
        self.assertEqual(root['claude_extra_usage']['usd_high'], root['pi_api']['usd_high'])
        self.assertEqual(nodes[CHILD]['direct']['claude_all_models']['calls'], 1)
        r = L.NATIVE_RATES['claude-opus-5-5']
        self.assertAlmostEqual(root['claude_extra_usage']['usd_high'], (100 * r['input'] + 10 * r['output']) / 1e6)
        f = L.NATIVE_RATES['claude-fable-5-1']
        self.assertAlmostEqual(root['claude_fable']['usd_high'], (1 * f['input'] + 1000 * f['output']) / 1e6)
        kinds = [i['kind'] for i in tree['issues'] if i['kind'] != 'role_host_ledger_absent']
        self.assertEqual(kinds, ['pi_provider_unpriced'])   # msg_F: an unlisted provider is never priced
        self.assertEqual(tree['status'], 'partial')
        self.assertEqual([s['session_id'] for s in tree['unattributed']['sessions']], [CC_STRAY])

    def test_billing_joins_by_session_response_and_model_conservatively(self):
        cases = [
            ([auth('msg_D', 'oauth')], ['claude_extra_usage'], 'extra_usage_observed'),
            ([auth('msg_D', 'api_key')] * 2, ['pi_api'], 'api_key_observed'),
            *[(entries, ['pi_api', 'claude_extra_usage'], 'api_key_or_extra_usage') for entries in (
                [],
                [auth('msg_missing', 'api_key')],
                [auth('msg_D', 'api_key', model='wrong-model')],
                [auth('msg_D', 'api_key', provider='claude-sdk')],
                [auth('msg_D', 'api_key', confidence='unknown')],
                [auth('msg_D', 'api_key'), auth('msg_D', 'oauth')],
                [auth('msg_D', 'bearer')],
                [auth('msg_D', 'unknown')],
                [auth('msg_D', [])],
            )],
        ]
        # Provenance in another session with no matching response is not evidence.
        write(os.path.join(self.pi, 'stray.jsonl'), [
            json.dumps({'type': 'session', 'id': STRAY, 'timestamp': at(5)}),
            auth('msg_D', 'oauth'),
        ])
        for entries, pools, billing in cases:
            with self.subTest(entries=entries):
                self.root(provenance=entries)
                sessions = L.pi_native_sessions(None, os.path.join(self.pi, '*.jsonl'))
                [direct] = [r for r in L.pi_native_join([], sessions) if r['call_id'] == 'msg_D']
                self.assertEqual((direct['pools'], direct['billing']), (pools, billing))
                tree = self.tree([{'session_id': ROOT, 'provider': 'pi', 'from': T0}])
                root = tree['nodes'][0]['direct']
                for pool in ('pi_api', 'claude_extra_usage'):
                    self.assertEqual(root[pool]['calls'], int(pool in pools))

    def test_openai_pricing_and_api_pool_routing(self):
        model = 'gpt-6.2-sol'
        self.root(provenance=[auth('msg_D', 'oauth')])
        with open(self.root_file, 'a') as f:
            for rid in ('api', 'oauth', 'missing', 'conflict', 'no-rate'):
                f.write(pi_msg(rid, at(7), 'openai', 180, 10, cr=800, cw=20,
                               model=model if rid != 'no-rate' else 'unpriced') + '\n')
            for rid, billing_class in [('api', 'api_key'), ('oauth', 'oauth'),
                                       ('conflict', 'oauth'), ('conflict', 'api_key')]:
                f.write(auth(rid, billing_class, provider='openai', model=model) + '\n')
            # An Anthropic entry for the right id/model can't establish OpenAI billing.
            f.write(auth('missing', 'oauth', model=model) + '\n')
        sessions = L.pi_native_sessions(None, self.root_file)
        rows = {r['call_id']: r for r in L.pi_native_join([], sessions)}
        self.assertEqual((rows['oauth']['billing'], rows['oauth']['pools']), ('chatgpt_oauth_observed', []))
        for rid, billing in [('api', 'api_key_observed'), ('missing', 'possible_api_spend'),
                             ('conflict', 'possible_api_spend')]:
            r = rows[rid]
            self.assertEqual((r['billing'], r['pools']), (billing, ['pi_api']))
            self.assertEqual(r['usage']['input_tokens'], 1000)
            self.assertEqual(r['usage']['total_tokens'], 1010)
            self.assertAlmostEqual(r['usd'], .00059)
            self.assertAlmostEqual(r['speed_adjusted_api_usd_low'], .00059 if rid == 'api' else 0)
            self.assertAlmostEqual(r['speed_adjusted_api_usd_high'], .001475)
        tree = self.tree([{'session_id': ROOT, 'provider': 'pi', 'from': T0}])
        pools = tree['nodes'][0]['direct']
        self.assertEqual((pools['pi_api']['calls'], pools['pi_api']['unpriced_calls']), (4, 1))
        self.assertAlmostEqual(pools['pi_api']['quota_usd_high'], .001475 * 3)
        self.assertEqual(pools['codex']['calls'], 0)
        self.assertEqual(sum(i['kind'] == 'pi_openai_billing_unconfirmed' for i in tree['issues']), 3)

    def test_complete_when_every_call_is_priced(self):
        self.root()
        tree = self.tree([{'session_id': ROOT, 'provider': 'pi', 'from': T0}])
        self.assertEqual(tree['status'], 'complete')
        self.assertEqual([i['kind'] for i in tree['issues']], ['role_host_ledger_absent'])

    def test_folded_usage_mismatch_is_an_issue(self):
        self.root(b_usage=(5, 60, 1000, 0))   # Pi saw only the last response of the folded turn
        tree = self.tree([{'session_id': ROOT, 'provider': 'pi', 'from': T0}])
        mismatch = [i for i in tree['issues'] if i['kind'] == 'pi_transcript_usage_mismatch']
        self.assertEqual(len(mismatch), 1)
        self.assertEqual((mismatch[0]['response_id'], mismatch[0]['rows']), ('msg_B', 2))
        self.assertEqual(tree['status'], 'partial')

    def test_claude_binding_keeps_rows_when_the_pi_session_is_not_a_member(self):
        self.root(); self.child()
        tree = self.tree([{'session_id': CC_STRAY, 'provider': 'claude', 'from': T0}])
        node = tree['nodes'][0]
        self.assertEqual((node['node_id'], node['direct']['claude_all_models']['calls']), (CC_STRAY, 1))

    def test_bound_pi_session_without_a_session_file_is_missing(self):
        tree = self.tree([{'session_id': ROOT, 'provider': 'pi', 'from': T0}])
        self.assertIn('transcript_missing', [i['kind'] for i in tree['issues']])
        self.assertEqual(tree['status'], 'not_established')

    def test_records_without_pi_sessions_do_not_read_pi_files(self):
        with patch.object(L, 'pi_native_sessions', side_effect=AssertionError('read')):
            self.tree([{'session_id': CC_STRAY, 'provider': 'claude', 'from': T0}])

    def test_user_cap_on_a_pi_child_trips(self):
        import argparse, contextlib, io
        self.root(provenance=[auth('msg_D', 'oauth')]); self.child()
        with open(self.root_file, 'a') as f:
            f.write(pi_msg('msg_openai', at(7), 'openai', 100, 10, model='gpt-6.2-sol') + '\n')
        rec = os.path.join(self.d, 'record.json')
        with open(rec, 'w') as f:
            json.dump({'scope_budget': {'objective_id': 'o', 'started_at': T0,
                                        'sessions': [{'session_id': ROOT, 'provider': 'pi', 'from': T0}], 'bindings': [],
                                        'cost_limits': [{'pool': pool, 'node': node, 'max_usd': 1e-6,
                                                         'marks_usd': [], 'set_by': 'user'} for pool, node in (
                                                             ('claude_all_models', CHILD), ('pi_api', ROOT),
                                                             ('claude_extra_usage', ROOT))]}}, f)
        out = io.StringIO()
        args = argparse.Namespace(record=rec, cursor=None, limit_fractions='1.0', role_host_ledger=os.path.join(self.d, 'absent'))
        with patch.object(L, 'HOME', self.d), patch.object(L, 'CODEX_GLOBS', []), \
                patch.object(L, 'CLAUDE_GLOB', os.path.join(self.d, '.claude', 'projects', '**', '*.jsonl')), \
                patch.object(L, 'PI_SESSION_GLOB', os.path.join(self.d, '.pi', 'agent', 'sessions', '**', '*.jsonl')), \
                patch.object(L, 'thread_index', return_value={}), contextlib.redirect_stdout(out):
            L.cmd_control(args)
        events = json.loads(out.getvalue())['events']
        reached = {(e.get('pool'), e.get('node_id'), e.get('set_by'))
                   for e in events if e.get('kind') == 'limit_reached'}
        self.assertEqual(reached, {('claude_all_models', CHILD, 'user'), ('pi_api', ROOT, 'user'),
                                   ('claude_extra_usage', ROOT, 'user')})

if __name__ == '__main__':
    unittest.main()
