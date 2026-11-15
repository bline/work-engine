"""P1.21 offline reproductions of the sealed one-shot review's findings (review/sealed/reviewer-response.md,
sha256 41c86b0d...; seal df5928a9...). Each `test_U..` asserts the CLAIMED behaviour, so "ok" means reproduced;
whether it is a defect, its severity and its scope are decided in the report. Controls are printed.

Runs unchanged against any reader: READER_TREE is the tree whose metrics/scripts/usage_ledger.py is under test (it is
imported first, so the helpers below use it); HELPER_TREE supplies the test helpers (the frozen candidate's tests).
Gated: refuses unless no provider key, a dead DOCKER_HOST and no `pi` on PATH. In-process on temporary fixtures.
Run: env -i PATH=/usr/bin:/bin DOCKER_HOST=unix:///nonexistent/docker.sock READER_TREE=<tree> HELPER_TREE=<tree> \
     python3 -m unittest repro_p121 -v"""
import json
import os
import shutil
import sys
import unittest

assert not any(k in os.environ for k in ('OPENAI_API_KEY', 'ANTHROPIC_API_KEY')), 'provider key present'
assert os.environ.get('DOCKER_HOST') == 'unix:///nonexistent/docker.sock' and shutil.which('pi') is None, 'gates missing'
R, H = os.environ['READER_TREE'], os.environ['HELPER_TREE']
sys.path.insert(0, os.path.join(R, 'metrics/scripts'))
import usage_ledger as L  # noqa: E402  (reader under test, cached before the helpers import it)
sys.path.insert(1, os.path.join(H, 'metrics/scripts/tests'))
import test_usage_ledger_pi_a1 as t1  # noqa: E402
import test_usage_ledger_pi_s2 as s2  # noqa: E402
assert s2.L is L, 'helpers must use the reader under test'
print('# reader under test:', L.digest(os.path.join(R, 'metrics/scripts/usage_ledger.py'))[:16], file=sys.stderr)
reserve, openai, OU, S1, S2 = s2.reserve, s2.openai, s2.OU, t1.S1, t1.S2
D = '2026-11-10T'


def req429(rid, at, n):
    r = dict(event='request', at=at, http_status=429, model='gpt-6.1-sol', response_id=None, usage={},
             usd=dict(rid=rid, reserved_usd=0.8, charged_usd=0.0, charge_basis='error_status'))
    if n is not ...: r['n'] = n
    return r


class Repro(t1.A1):
    def decide(self, rec, new):
        out = self.admit_j(rec, new, os.path.join(self.d, 'j.jsonl'))
        sh, sl = out.get('spent_high_usd'), out.get('spent_low_usd')
        return out['decision'], None if sh is None else round(sh, 6), sorted(set(self.kinds(out)))

    def show(self, label, got):
        print(f'\n{label}: {got}', file=sys.stderr); return got

    def reset(self):
        self.rows = []; L.ISSUES.clear()

    # U-01: attempt n compared with untyped equality (true == 1; missing == missing)
    def test_U01_malformed_attempt_number_releases(self):
        rec = self.record(pi_limit=1.0)
        self.session(S1, 'root', [[reserve(1, 0.8, at=D + '06:30:00Z'), req429(1, D + '06:30:01Z', True)]], retired=True)
        got = self.show('U-01 n=true on the settlement, reservation n=1', self.decide(rec, 0.5))
        self.reset(); r = reserve(1, 0.8, at=D + '06:30:00Z'); r.pop('n')
        self.session(S1, 'root', [[r, req429(1, D + '06:30:01Z', ...)]], retired=True)
        both = self.show('U-01 n missing on both records', self.decide(rec, 0.5))
        self.reset(); self.session(S1, 'root', [[reserve(1, 0.8, at=D + '06:30:00Z'), req429(1, D + '06:30:01Z', 2)]], retired=True)
        self.show('U-01 control: n=2 on the settlement', self.decide(rec, 0.5))
        self.reset(); self.session(S1, 'root', [[reserve(1, 0.8, at=D + '06:30:00Z'), req429(1, D + '06:30:01Z', 1)]], retired=True)
        self.show('U-01 control: valid n=1 (deployed shape)', self.decide(rec, 0.5))
        self.assertEqual(got, ('admitted', 0.0, []))
        self.assertEqual(both, ('admitted', 0.0, []))

    # U-02: an API settlement without its final newline is processed as a settlement
    def test_U02_unterminated_api_settlement_releases(self):
        rec = self.record(pi_limit=1.0)
        self.session(S1, 'root', [[reserve(1, 0.8, at=D + '06:30:00Z')]], retired=True)
        with open(self.log_of(S1), 'a') as fh: fh.write(json.dumps(req429(1, D + '06:30:01Z', 1)))   # no newline
        got = self.show('U-02 error_status settlement without final newline', self.decide(rec, 0.5))
        self.reset(); self.session(S1, 'root', [[reserve(1, 0.8, at=D + '06:30:00Z'), req429(1, D + '06:30:01Z', 1)]], retired=True)
        self.show('U-02 control: same settlement with its newline', self.decide(rec, 0.5))
        self.reset(); self.session(S1, 'root', [[reserve(1, 0.8, at=D + '06:30:00Z')]], retired=True)
        with open(self.log_of(S1), 'a') as fh: fh.write(json.dumps(req429(1, D + '06:30:01Z', 1))[:-7])   # cut inside the JSON
        self.show('U-02 control: settlement cut inside the JSON', self.decide(rec, 0.5))
        self.assertEqual(got, ('admitted', 0.0, []))

    # U-03: a retirement stamp earlier than later proxy evidence excludes an unsettled reservation
    def test_U03_retirement_before_later_proxy_row_drops_obligation(self):
        rec = self.record(pi_limit=1.0, started=D + '06:20:00Z')
        rows = [reserve(1, 0.8, at=D + '06:00:00Z'), dict(event='refused', at=D + '06:30:00Z', n=2, reason='usd_limit')]
        self.session(S1, 'root', [rows]); self.rows.append(dict(host='pi', event='retire', session_id=S1, at=D + '06:10:00Z'))
        got = self.show('U-03 reserve 06:00, refused row 06:30, retire stamped 06:10; objective from 06:20', self.decide(rec, 0.5))
        self.reset(); self.session(S1, 'root', [rows]); self.rows.append(dict(host='pi', event='retire', session_id=S1, at=D + '06:40:00Z'))
        self.show('U-03 control: retire 06:40', self.decide(rec, 0.5))
        self.reset(); self.session(S1, 'root', [rows[:1]]); self.rows.append(dict(host='pi', event='retire', session_id=S1, at=D + '06:10:00Z'))
        self.show('U-03 control: no later proxy row, retire 06:10 (consistent: before the objective)', self.decide(rec, 0.5))
        self.assertEqual(got, ('admitted', 0.0, []))

    # U-04: closed objective, settlement after closure, interval crossing the closure
    def test_U04_closed_window_drops_interval_crossing_closure(self):
        def view(tree):
            pa = tree['totals']['pi_api']
            pc = (tree.get('post_close') or {}).get('candidate_totals') or {}
            return dict(status=tree['status'], window_usd_high=round(pa['usd_high'], 6), window_calls=pa.get('calls'),
                        issues=sorted({i['kind'] for i in tree['issues']}),
                        post_close_pi_api_high=round((pc.get('pi_api') or {}).get('usd_high', 0), 6) if pc else None)
        rec = self.record(closed_at=D + '07:00:00Z')
        self.session(S1, 'root', [[reserve(1, 0.5, at=D + '06:50:00Z'), openai(1, 'resp_u4', OU, 0.02, at=D + '07:10:00Z')]], retired=True)
        got = self.show('U-04 objective 06:00-07:00; reserve 06:50, valid usage settlement 07:10', view(self.tree(rec)))
        self.reset(); self.session(S1, 'root', [[reserve(1, 0.5, at=D + '06:50:00Z'), openai(1, 'resp_u4', OU, 0.02, at=D + '06:55:00Z')]], retired=True)
        ctl = self.show('U-04 control: settlement 06:55 (inside)', view(self.tree(rec)))
        self.reset(); self.session(S1, 'root', [[reserve(1, 0.5, at=D + '07:05:00Z'), openai(1, 'resp_u4', OU, 0.02, at=D + '07:10:00Z')]], retired=True)
        self.show('U-04 control: reserve 07:05, settle 07:10 (wholly after closure)', view(self.tree(rec)))
        self.assertGreater(ctl['window_usd_high'], 0)
        self.assertEqual(got['window_usd_high'], 0.0)
        self.assertNotIn('pi_undated_call_unattributable', got['issues'])

    # U-05: usage settlement without response id: charge range replaces priceable usage
    def test_U05_usage_settlement_without_response_id_loses_usage_exposure(self):
        rec = self.record(pi_limit=1.0)
        usage = dict(input_tokens=0, output_tokens=12000, cached_tokens=0, cache_write_tokens=0, reasoning_tokens=0)
        def row(rid_resp, charged):
            return dict(event='request', at=D + '06:30:01Z', n=1, http_status=200, model='gpt-6-astra', response_model='gpt-6-astra',
                        requested_tier='default', observed_tier='default', response_id=rid_resp, usage=usage, seconds=1.0,
                        usd=dict(rid=1, reserved_usd=0.8, charged_usd=charged, charge_basis='usage'))
        self.session(S1, 'root', [[reserve(1, 0.8, at=D + '06:30:00Z'), row(None, 0.01)]], retired=True)
        got = self.show('U-05 no response id, usage prices to 0.6, charged 0.01', self.decide(rec, 0.5))
        self.reset(); self.session(S1, 'root', [[reserve(1, 0.8, at=D + '06:30:00Z'), row('resp_u5', 0.01)]], retired=True)
        self.show('U-05 control: same with a response id', self.decide(rec, 0.5))
        self.reset(); self.session(S1, 'root', [[reserve(1, 0.8, at=D + '06:30:00Z'), row(None, 0.6)]], retired=True)
        cons = self.show('U-05 control: no response id, consistent charge 0.6', self.decide(rec, 0.5))
        out = self.admit_j(rec, 0.5, os.path.join(self.d, 'j2.jsonl'))
        print(f"U-05 consistent: spent_low={out.get('spent_low_usd')}", file=sys.stderr)
        self.assertEqual(got, ('admitted', 0.01, []))

    # U-06: launch lane alone moves an unlabelled upstream_error out of API accounting
    def test_U06_launch_lane_hides_api_upstream_error(self):
        rec = self.record(pi_limit=1.0)
        err = dict(event='upstream_error', at=D + '06:30:01Z', n=1, detail='timeout')
        self.session(S1, 'root', [[err]], retired=True)
        api = self.show('U-06 control: API launch, unlabelled upstream_error without usd', self.decide(rec, 0.5))
        self.reset(); self.session(S1, 'root', [[err]], retired=True)
        self.rows[0].update(auth_lane='subscription', model='openai-codex/gpt-6-astra')
        sub = self.show('U-06 same row, launch auth_lane=subscription', self.decide(rec, 0.5))
        self.reset(); self.session(S1, 'root', [[dict(err, provider='openai-codex', auth_lane='subscription', model='gpt-6-astra')]], retired=True)
        self.rows[0].update(auth_lane='subscription', model='openai-codex/gpt-6-astra')
        self.show('U-06 control: subscription launch, producer-labelled upstream_error', self.decide(rec, 0.5))
        self.assertEqual(api[0], 'not_established')
        self.assertEqual(sub[0], 'admitted')

    # U-07: malformed nested usage in an unrelated session aborts evaluation
    def test_U07_unrelated_malformed_usage_aborts(self):
        rec = self.record(pi_limit=1.0)
        bad = dict(event='request', at=D + '06:30:01Z', n=1, http_status=200, model='gpt-6-astra', response_id='other', usage=[1])
        self.session(S1, 'root', [[reserve(1, 0.5, at=D + '06:30:00Z'), openai(1, 'resp_m', OU, 0.02, at=D + '06:30:02Z')]], retired=True)
        self.session(S2, 'unrelated-root', [[bad]], retired=True)
        try:
            got = self.show('U-07 unrelated session with usage=[1]', self.decide(rec, 0.5))
        except Exception as e:
            got = self.show('U-07 unrelated session with usage=[1]', ('raised', type(e).__name__, str(e)[:80]))
        self.reset()
        self.session(S1, 'root', [[reserve(1, 0.5, at=D + '06:30:00Z'), openai(1, 'resp_m', OU, 0.02, at=D + '06:30:02Z')]], retired=True)
        self.session(S2, 'unrelated-root', [[dict(bad, usage={'input_tokens': 'x'})]], retired=True)
        try:
            self.show('U-07 control: unrelated session with malformed dict usage', self.decide(rec, 0.5))
        except Exception as e:
            self.show('U-07 control: unrelated session with malformed dict usage', ('raised', type(e).__name__))
        self.assertEqual(got[0], 'raised')

    # U-08: producer-supported OpenAI usage settlement with no service tier is rejected
    def test_U08_missing_tier_usage_settlement_rejected(self):
        rec = self.record(pi_limit=1.0)
        r = openai(1, 'resp_t', OU, 0.03, tier=None, at=D + '06:30:01Z')
        self.session(S1, 'root', [[reserve(1, 0.5, at=D + '06:30:00Z'), r]], retired=True)
        got = self.show('U-08 usage settlement, observed_tier null (producer prices at the maximum multiplier)', self.decide(rec, 0.3))
        self.reset(); self.session(S1, 'root', [[reserve(1, 0.5, at=D + '06:30:00Z'), openai(1, 'resp_t', OU, 0.03, tier='default', at=D + '06:30:01Z')]], retired=True)
        self.show('U-08 control: observed_tier default', self.decide(rec, 0.3))
        self.assertEqual(got[0], 'not_established')
        self.assertIn('pi_settlement_invalid', got[2])


for _n in [n for n in dir(t1.A1) if n.startswith('test_') and n not in Repro.__dict__]:
    setattr(Repro, _n, None)

if __name__ == '__main__':
    unittest.main()
