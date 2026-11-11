"""S2: Pi spend in objective trees (OpenAI + Anthropic, cache reads/writes, cancelled streams, retained
reservations) and objective-level pi_api admission. T1-T10 and A1-A9 of
~/cbm-work/pi-anthropic-release-planning-20261111/decision-packet.md."""
import json
import os
import shutil
import sys
import tempfile
import unittest
from unittest import mock

sys.path.insert(0, os.path.join(os.path.dirname(__file__), '..'))
import usage_ledger as L  # noqa: E402

FIXTURE = os.path.join(os.path.dirname(__file__), 'fixtures', 'pi-p3-anthropic')
FABLE = 'claude-fable-5-2'
OA, AN, GRAND = ('00000000-0000-4000-8000-0000000000a1', '00000000-0000-4000-8000-0000000000a2',
                 '00000000-0000-4000-8000-0000000000a3')


def reserve(rid, amount, at='2026-11-10T06:30:00Z'):
    return dict(event='usd_reserved', at=at, n=rid, rid=rid, reserved_usd=amount, input_bound=1000, bound_basis='bytes')


def anth(rid, resp, usage, charged, at='2026-11-10T06:30:01Z', status=200, tier='standard', check='verified'):
    return dict(event='request', provider='anthropic', at=at, n=rid, http_status=status, model=FABLE,
                response_model=FABLE, response_model_check=check, observed_tier=tier, response_id=resp,
                usage=usage, seconds=1.0, usd=dict(rid=rid, reserved_usd=0.3, charged_usd=charged))


def openai(rid, resp, usage, charged, tier='flex', at='2026-11-10T06:30:02Z'):
    return dict(event='request', at=at, n=rid, http_status=200, model='gpt-6-astra', response_model='gpt-6-astra',
                requested_tier=tier, observed_tier=tier, response_id=resp, usage=usage, seconds=1.0,
                usd=dict(rid=rid, reserved_usd=0.5, charged_usd=charged))


def au(i=1000, read=2000, w5=1000, w1=2000, out=500):
    return dict(input_tokens=i, cache_read_input_tokens=read, cache_creation_input_tokens=w5 + w1,
                ephemeral_5m_input_tokens=w5, ephemeral_1h_input_tokens=w1, output_tokens=out)


T2_USD = (1000 * 8 + 2000 * 0.2 + 1000 * 10 + 2000 * 16 + 500 * 40) / 1e6   # 0.0704 at Fable rates
OU = dict(input_tokens=11000, output_tokens=500, cached_tokens=10000, reasoning_tokens=0, cache_write_tokens=900)


class PiTree(unittest.TestCase):
    def setUp(self):
        L.ISSUES.clear()
        self.tmp = tempfile.TemporaryDirectory(); self.addCleanup(self.tmp.cleanup)
        self.d = self.tmp.name
        self.ledger = os.path.join(self.d, 'state', 'ledger.jsonl')
        os.makedirs(os.path.dirname(self.ledger))
        self.rows = []

    def session(self, sid, parent, turns, max_usd=1.0, retired=False):
        for t, proxy in enumerate(turns, 1):
            path = os.path.join(self.d, 'state', 'pi', sid, 'host', f'turn-{t}.proxy.jsonl')
            os.makedirs(os.path.dirname(path), exist_ok=True)
            with open(path, 'w') as fh:
                fh.write(''.join(json.dumps(r) + '\n' for r in proxy))
            self.rows.append(dict(host='pi', event='launch' if t == 1 else 'deliver', session_id=sid, turn=t,
                                  parent_session_id=parent if t == 1 else None, thinking='high', model=FABLE,
                                  files=dict(proxy_log=path), usd_limit=dict(max_usd=max_usd)))
        if retired:
            self.rows.append(dict(host='pi', event='retire', session_id=sid))

    def admit_row(self, sid, max_usd, record):
        self.rows.append(dict(host='pi', event='objective_admission', session_id=sid,
                              objective_record=os.path.abspath(record), max_usd=max_usd))

    def record(self, closed_at=None, pi_limit=None, started='2026-11-10T06:00:00Z', session_from=None):
        sb = dict(objective_id='o', started_at=started, bindings=[],
                  sessions=[dict(session_id='root', provider='codex', **{'from': session_from or started})])
        if closed_at:
            sb['closed_at'] = closed_at
        if pi_limit is not None:
            sb['cost_limits'] = [dict(pool='pi_api', node=None, max_usd=pi_limit, marks_usd=[], set_by='user')]
        path = os.path.join(self.d, 'record.json')
        with open(path, 'w') as fh:
            json.dump(dict(scope_budget=sb), fh)
        return path

    def patches(self):
        real = L.transcript_available
        avail = lambda provider, sid, *a, **kw: True if sid == 'root' else real(provider, sid, *a, **kw)
        return [mock.patch.object(L, 'build_codex', return_value=([], [], [], 0)),
                mock.patch.object(L, 'build_claude', return_value=([], [], 0)),
                mock.patch.object(L, 'unrecorded_claude', return_value=[]),
                mock.patch.object(L, 'transcript_available', side_effect=avail)]

    def write(self):
        with open(self.ledger, 'w') as fh:
            fh.write(''.join(json.dumps(r) + '\n' for r in self.rows))

    def tree(self, record):
        self.write()
        ps = self.patches()
        for p in ps: p.start()
        try:
            return L.cost_tree(record, role_ledger=self.ledger)[0]
        finally:
            for p in ps: p.stop()

    def admit(self, record, new, parent='root'):
        self.write()
        ps = self.patches()
        for p in ps: p.start()
        try:
            return L.pi_admission(record, new, parent, self.ledger)
        finally:
            for p in ps: p.stop()

    @staticmethod
    def node(tree, sid):
        return {n['node_id']: n for n in tree['nodes']}[sid]

    # ---------------------------------------------------------------- tree
    def test_t1_openai_child_flex_with_cache_writes(self):
        self.session(OA, 'root', [[reserve(1, 0.5), openai(1, 'resp_1', OU, 0.01)]])
        tree = self.tree(self.record())
        price = L.codex_usd(dict(input_tokens=11000, cached_input_tokens=10000, cache_write_input_tokens=900,
                                 output_tokens=500, reasoning_output_tokens=0, total_tokens=11500), 'gpt-6-astra')
        direct = self.node(tree, OA)['direct']['pi_api']
        self.assertAlmostEqual(direct['usd_low'], price)
        self.assertAlmostEqual(direct['quota_usd_high'], price * 0.5)            # flex is half price
        self.assertAlmostEqual(self.node(tree, 'root')['inclusive']['pi_api']['quota_usd_high'], price * 0.5)
        self.assertEqual(tree['totals']['claude_all_models']['calls'], 0)       # Pi never enters a Claude pool
        self.assertEqual(tree['totals']['codex']['calls'], 0)

    def test_t2_anthropic_child_every_counter_at_fable_rates(self):
        self.session(AN, 'root', [[reserve(1, 0.3), anth(1, 'msg_1', au(), T2_USD)]])
        tree = self.tree(self.record())
        direct = self.node(tree, AN)['direct']['pi_api']
        self.assertAlmostEqual(direct['usd_low'], T2_USD, places=12)
        self.assertAlmostEqual(direct['usd_high'], T2_USD, places=12)
        self.assertAlmostEqual(direct['quota_usd_high'], T2_USD, places=12)
        self.assertEqual(tree['status'], 'complete')

    def test_t3_two_children_and_a_grandchild_sum_without_double_count(self):
        self.session(OA, 'root', [[reserve(1, 0.5), openai(1, 'resp_1', OU, 0.01)],
                                  [reserve(1, 0.5, '2026-11-10T06:40:00Z'), openai(1, 'resp_2', OU, 0.01, at='2026-11-10T06:40:01Z')]])
        self.session(AN, 'root', [[reserve(1, 0.3), anth(1, 'msg_1', au(), T2_USD)]])
        self.session(GRAND, AN, [[reserve(1, 0.3), anth(1, 'msg_2', au(), T2_USD)]])
        tree = self.tree(self.record())
        oa, an = self.node(tree, OA)['direct']['pi_api'], self.node(tree, AN)
        self.assertEqual(oa['calls'], 2)
        self.assertEqual(an['inclusive']['pi_api']['calls'], 2)
        self.assertAlmostEqual(an['inclusive']['pi_api']['quota_usd_high'], 2 * T2_USD)
        root = self.node(tree, 'root')['inclusive']['pi_api']
        self.assertEqual(root['calls'], 4)
        self.assertAlmostEqual(root['quota_usd_high'], oa['quota_usd_high'] + 2 * T2_USD)
        self.assertAlmostEqual(tree['totals']['pi_api']['quota_usd_high'], root['quota_usd_high'])

    def test_t4_cancelled_stream_keeps_its_reservation_as_a_range(self):
        self.session(AN, 'root', [[reserve(1, 0.3), anth(1, 'msg_1', au(), T2_USD)],
                                  [reserve(1, 0.289328, '2026-11-10T06:50:00Z')]])
        tree = self.tree(self.record())
        t = tree['totals']['pi_api']
        self.assertAlmostEqual(t['usd_low'], T2_USD)
        self.assertAlmostEqual(t['quota_usd_high'], T2_USD + 0.289328)
        self.assertEqual(t['unpriced_calls'], 0)
        self.assertEqual(tree['status'], 'partial')
        rows, _ = L.build_pi(None, None, ledger=self.ledger)
        [kept] = [r for r in rows if r.get('charge_basis') == 'retained_reservation']
        self.assertEqual((kept['usd_low'], kept['usd_high'], kept['settlement']), (0.0, 0.289328, 'unsettled'))

    def test_t5_identity_refusal_low_is_usage_high_is_the_kept_reservation(self):
        small = au(i=10, read=0, w5=0, w1=0, out=1)
        refused = anth(1, 'msg_x', small, 0.25, check='response_model_mismatch')
        self.session(AN, 'root', [[reserve(1, 0.25), refused]])
        tree = self.tree(self.record())
        t = tree['totals']['pi_api']
        self.assertEqual(t['calls'], 1)                                        # one request, no extra reservation row
        self.assertAlmostEqual(t['usd_low'], (10 * 8 + 40) / 1e6)
        self.assertAlmostEqual(t['quota_usd_high'], 0.25)

    def test_t5b_openai_kept_charge_is_the_high_bound_at_tier_price(self):
        self.session(OA, 'root', [[reserve(1, 0.5), openai(1, 'resp_1', OU, 0.4)]])
        t = self.tree(self.record())['totals']['pi_api']
        self.assertAlmostEqual(t['quota_usd_high'], 0.4)                        # proxy charge is already at flex price
        self.assertLess(t['quota_usd_low'], 0.4)

    def test_t6_errors_and_usd_limit_refusals_cost_nothing(self):
        error = dict(event='request', provider='anthropic', at='2026-11-10T06:30:01Z', http_status=529, model=FABLE,
                     response_id=None, usage={}, usd=dict(rid=1, reserved_usd=0.3, charged_usd=0.0))
        limit = dict(event='refused', at='2026-11-10T06:30:02Z', reason='usd_limit: spent 0 + worst case 0.3 > limit 0.1')
        self.session(AN, 'root', [[reserve(1, 0.3), error, limit]])
        tree = self.tree(self.record())
        t = tree['totals']['pi_api']
        self.assertEqual((t['calls'], t['quota_usd_high']), (0, 0))

    def test_t7_unknown_split_is_a_range_and_unknown_tier_is_unpriced(self):
        nosplit = dict(input_tokens=0, cache_read_input_tokens=0, cache_creation_input_tokens=1000, output_tokens=0)
        self.session(AN, 'root', [[reserve(1, 0.3), anth(1, 'msg_1', nosplit, 0.016)],
                                  [reserve(1, 0.3, '2026-11-10T06:40:00Z'), anth(1, 'msg_2', au(), 0.07, tier='priority', at='2026-11-10T06:40:01Z')]])
        tree = self.tree(self.record())
        t = tree['totals']['pi_api']
        self.assertAlmostEqual((t['usd_low'], t['usd_high'])[0], 1000 * 10 / 1e6)
        self.assertAlmostEqual(t['usd_high'], 1000 * 16 / 1e6)
        self.assertEqual(t['unpriced_calls'], 1)

    def test_t8_pi_spend_before_started_at_counts_from_the_session_start(self):
        self.session(AN, 'root', [[reserve(1, 0.3, '2026-11-10T05:40:00Z'), anth(1, 'msg_1', au(), T2_USD, at='2026-11-10T05:40:01Z')]])
        tree = self.tree(self.record(session_from='2026-11-10T05:30:00Z'))
        self.assertAlmostEqual(tree['totals']['pi_api']['quota_usd_high'], T2_USD)

    def test_t9_pi_spend_after_close_is_post_close_and_in_reconciled_all_in(self):
        self.session(AN, 'root', [[reserve(1, 0.3), anth(1, 'msg_1', au(), T2_USD)],
                                  [reserve(1, 0.2, '2026-11-10T07:10:00Z')]])
        tree = self.tree(self.record(closed_at='2026-11-10T07:00:00Z', pi_limit=0.25))
        self.assertAlmostEqual(tree['totals']['pi_api']['quota_usd_high'], T2_USD)
        self.assertAlmostEqual(tree['post_close']['candidate_totals']['pi_api']['quota_usd_high'], 0.2)
        allin = tree['reconciled_all_in']
        self.assertAlmostEqual(allin['totals']['pi_api']['quota_usd_high'], T2_USD + 0.2)
        [b] = allin['hard_limit_breaches']
        self.assertEqual((b['pool'], b['status']), ('pi_api', 'exceeded'))

    def test_t10_p3_replay_matches_the_proxy(self):
        state = os.path.join(self.d, 'p3')
        shutil.copytree(FIXTURE, state)
        with open(os.path.join(state, 'ledger.jsonl')) as fh:
            text = fh.read().replace('@STATE@', state)
        rows = [json.loads(l) for l in text.splitlines()]
        for r in rows:
            if r.get('event') == 'launch':
                r['parent_session_id'] = 'root'
        self.rows = rows
        tree = self.tree(self.record(started='2026-11-11T01:00:00Z'))
        t = tree['totals']['pi_api']
        self.assertEqual(t['calls'], 4)                                        # 3 requests + 1 retained reservation
        self.assertAlmostEqual(t['usd_low'], 0.044957, places=6)
        self.assertAlmostEqual(t['quota_usd_high'], 0.334285, places=6)        # = proxy session_spent_usd
        self.assertEqual(tree['status'], 'partial')

    # ---------------------------------------------------------------- admission
    def test_a1_no_owner_pi_limit_refuses(self):
        out = self.admit(self.record(), 0.5)
        self.assertEqual(out['decision'], 'refused')

    def test_a2_admits_within_the_limit(self):
        self.session(AN, 'root', [[reserve(1, 0.3), anth(1, 'msg_1', au(), T2_USD)]], retired=True)
        out = self.admit(self.record(pi_limit=1.0), 0.5)
        self.assertEqual(out['decision'], 'admitted', out)
        self.assertAlmostEqual(out['exposure_usd'], T2_USD + 0.5)

    def test_a3_open_session_commitment_counts(self):
        rec = self.record(pi_limit=1.0)
        self.session(AN, 'root', [[reserve(1, 0.3), anth(1, 'msg_1', au(), T2_USD)]], max_usd=0.6)
        self.admit_row(AN, 0.6, rec)
        out = self.admit(rec, 0.5)
        self.assertEqual(out['decision'], 'refused', out)
        self.assertAlmostEqual(out['committed_usd'], 0.6 - T2_USD)
        self.assertAlmostEqual(out['exposure_usd'], 1.1)

    def test_a4_retire_receipt_releases_the_commitment(self):
        rec = self.record(pi_limit=1.0)
        self.session(AN, 'root', [[reserve(1, 0.3), anth(1, 'msg_1', au(), T2_USD)]], max_usd=0.6, retired=True)
        self.admit_row(AN, 0.6, rec)
        out = self.admit(rec, 0.5)
        self.assertEqual((out['decision'], out['committed_usd']), ('admitted', 0.0), out)

    def test_a5_retained_reservation_counts_at_its_high_bound(self):
        rec = self.record(pi_limit=0.5)
        self.session(AN, 'root', [[reserve(1, 0.3)]], retired=True)
        out = self.admit(rec, 0.25)
        self.assertEqual(out['decision'], 'refused', out)
        self.assertAlmostEqual(out['spent_high_usd'], 0.3)

    def test_a6_unpriced_pi_spend_is_not_established(self):
        rec = self.record(pi_limit=5.0)
        self.session(AN, 'root', [[reserve(1, 0.3), anth(1, 'msg_1', au(), 0.07, tier='priority')]], retired=True)
        self.assertEqual(self.admit(rec, 0.1)['decision'], 'not_established')

    def test_a7_parent_outside_the_tree_refuses(self):
        self.assertEqual(self.admit(self.record(pi_limit=5.0), 0.1, parent='elsewhere')['decision'], 'refused')

    def test_a8_closed_objective_refuses(self):
        self.assertEqual(self.admit(self.record(pi_limit=5.0, closed_at='2026-11-10T07:00:00Z'), 0.1)['decision'], 'refused')

    def test_a9_session_spend_above_its_own_limit_is_inconsistent(self):
        rec = self.record(pi_limit=5.0)
        self.session(AN, 'root', [[reserve(1, 0.3), anth(1, 'msg_1', au(), T2_USD)]], max_usd=0.05)
        self.admit_row(AN, 0.05, rec)
        self.assertEqual(self.admit(rec, 0.1)['decision'], 'not_established')

    def test_a10_unreadable_proxy_log_is_not_established(self):
        rec = self.record(pi_limit=5.0)
        self.session(AN, 'root', [[reserve(1, 0.3), anth(1, 'msg_1', au(), T2_USD)]], retired=True)
        os.remove(self.rows[0]['files']['proxy_log'])
        self.assertEqual(self.admit(rec, 0.1)['decision'], 'not_established')


if __name__ == '__main__':
    unittest.main()
