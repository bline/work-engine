"""P1.19 offline reproductions of the reviewer's sealed interim checkpoint claims (review/sealed/reviewer-interim-text.md,
b39948d7...) against frozen A1.3 (59c798ba). Each `test_C..` asserts the CLAIMED behaviour, so "ok" means the behaviour
is reproduced; whether it is a defect is decided in the report. Every test has a control.
Gated: refuses to import unless no provider key, a dead DOCKER_HOST and no `pi` on PATH. Reader called in-process on
temporary fixtures; nothing launched. Timestamps for C1/C2 are relative to the real clock (the claims concern rows
ahead of the reader's clock). Run: env -i ... A13_TREE=<tree> python3 -m unittest repro_p119 -v"""
import datetime
import json
import os
import shutil
import sys
import unittest
from unittest import mock

assert not any(k in os.environ for k in ('OPENAI_API_KEY', 'ANTHROPIC_API_KEY')), 'provider key present'
assert os.environ.get('DOCKER_HOST') == 'unix:///nonexistent/docker.sock' and shutil.which('pi') is None, 'gates missing'
T = os.environ['A13_TREE']
sys.path.insert(0, os.path.join(T, 'metrics/scripts')); sys.path.insert(0, os.path.join(T, 'metrics/scripts/tests'))
import usage_ledger as L  # noqa: E402
import test_usage_ledger_pi_a1 as t1  # noqa: E402
import test_usage_ledger_pi_s2 as s2  # noqa: E402
print('# reader under test:', L.digest(os.path.join(T, 'metrics/scripts/usage_ledger.py'))[:16])
reserve, openai, OU, S1 = s2.reserve, s2.openai, s2.OU, t1.S1


def iso(delta_s):
    return (datetime.datetime.now(datetime.timezone.utc) + datetime.timedelta(seconds=delta_s)).strftime('%Y-%m-%dT%H:%M:%SZ')


def upstream(rid, amount, at, n=None):
    return dict(event='upstream_error', at=at, n=rid if n is None else n, detail='x',
                usd=dict(rid=rid, reserved_usd=amount, charged_usd=amount, charge_basis='reservation'))


def req429(rid, at, basis):
    usd = dict(rid=rid, reserved_usd=0.8, charged_usd=0.0)
    if basis: usd['charge_basis'] = basis
    return dict(event='request', at=at, n=rid, http_status=429, model='gpt-6.1-sol', response_id=None, usage={}, usd=usd)


class Repro(t1.A1):
    def decide(self, rec, new):
        out = self.admit_j(rec, new, os.path.join(self.d, 'j.jsonl'))
        sh = out.get('spent_high_usd')
        return out['decision'], None if sh is None else round(sh, 6), sorted(set(self.kinds(out)))

    def show(self, label, got):
        print(f'\n{label}: {got}'); return got

    # C1: interval spanning the window; settlement within the 300 s clock tolerance, reservation before the start
    def test_C1_spanning_settlement_inside_tolerance_is_dropped(self):
        rec = self.record(pi_limit=1.0, started=iso(-300))
        self.session(S1, 'root', [[reserve(1, 0.8, at=iso(-600)), upstream(1, 0.8, iso(+120))]], retired=True)
        got = self.show('C1 reserve 0.8 at now-10min (objective from now-5min), settlement at now+2min', self.decide(rec, 0.5))
        self.fresh_ctrl = True
        self.rows = []; self.session(S1, 'root', [[reserve(1, 0.8, at=iso(-600)), upstream(1, 0.8, iso(-60))]], retired=True)
        ctl = self.show('C1 control: settlement at now-1min (in window)', self.decide(rec, 0.5))
        self.rows = []; self.session(S1, 'root', [[reserve(1, 0.8, at=iso(-600)), upstream(1, 0.8, iso(+600))]], retired=True)
        far = self.show('C1 control: settlement at now+10min (beyond tolerance)', self.decide(rec, 0.5))
        self.assertEqual(got, ('admitted', 0.0, []))
        self.assertEqual(ctl[:2], ('refused', 0.8))
        self.assertEqual(far[0], 'not_established')

    # C2: rows ahead of the reader clock but inside the 300 s tolerance are excluded without blocking
    def test_C2_rows_ahead_inside_tolerance_are_excluded(self):
        rec = self.record(pi_limit=1.0, started=iso(-3600))
        self.session(S1, 'root', [[reserve(1, 0.8, at=iso(+120))]], retired=True)
        unsettled = self.show('C2 unsettled reservation 0.8 at now+2min', self.decide(rec, 0.5))
        self.rows = []
        row = openai(1, 'resp_f', OU, 0.1, at=iso(+120)); row.pop('usd')
        self.session(S1, 'root', [[row]], retired=True)
        usage = self.show('C2 no-limit usage row at now+2min', self.decide(rec, 0.5))
        self.rows = []; self.session(S1, 'root', [[reserve(1, 0.8, at=iso(-60))]], retired=True)
        ctl = self.show('C2 control: same reservation at now-1min', self.decide(rec, 0.5))
        self.rows = []; self.session(S1, 'root', [[reserve(1, 0.8, at=iso(+600))]], retired=True)
        far = self.show('C2 control: reservation at now+10min (beyond tolerance)', self.decide(rec, 0.5))
        self.assertEqual(unsettled, ('admitted', 0.0, []))
        self.assertEqual(usage, ('admitted', 0.0, []))
        self.assertEqual(ctl[:2], ('refused', 0.8))
        self.assertEqual(far[0], 'not_established')

    # C3: an exact copy of the reservation appended after its settlement (same second) is collapsed and released
    def test_C3_identical_reservation_copy_after_settlement_is_collapsed(self):
        rec = self.record(pi_limit=1.0)
        at = '2026-11-10T06:31:00Z'
        self.session(S1, 'root', [[reserve(1, 0.8, at=at), req429(1, at, 'error_status'), reserve(1, 0.8, at=at)]], retired=True)
        got = self.show('C3 reserve, 429 settlement, identical reserve copy (same second)', self.decide(rec, 0.5))
        self.rows = []; self.session(S1, 'root', [[reserve(1, 0.8, at=at), req429(1, at, 'error_status'), dict(reserve(1, 0.8, at=at), n=2)]], retired=True)
        ctl = self.show('C3 control: same rid reserved again with another n after the settlement', self.decide(rec, 0.5))
        self.assertEqual(got, ('admitted', 0.0, []))
        self.assertEqual(ctl[0], 'not_established')

    # C4: a settlement without charge_basis is inferred (here: 429 -> error_status) and releases the reservation
    def test_C4_missing_charge_basis_is_inferred_and_releases(self):
        rec = self.record(pi_limit=1.0)
        at = '2026-11-10T06:31:00Z'
        self.session(S1, 'root', [[reserve(1, 0.8, at=at), req429(1, at, None)]], retired=True)
        got = self.show('C4 429 settlement without charge_basis, charged 0', self.decide(rec, 0.5))
        self.rows = []; self.session(S1, 'root', [[reserve(1, 0.8, at=at), req429(1, at, 'error_status')]], retired=True)
        ctl = self.show('C4 control: the same with charge_basis error_status (deployed shape)', self.decide(rec, 0.5))
        self.assertEqual(got, ('admitted', 0.0, []))
        self.assertEqual(ctl, ('admitted', 0.0, []))

    # C5: an API log's final line without a newline is read (no partial-row issue); does it lower exposure?
    def test_C5_api_final_line_without_newline_is_read(self):
        rec = self.record(pi_limit=1.0)
        self.session(S1, 'root', [[reserve(1, 0.5), openai(1, 'resp_a', OU, 0.1)]], retired=True)
        with open(self.log_of(S1), 'a') as fh: fh.write(json.dumps(reserve(2, 0.8, at='2026-11-10T06:30:03Z')))   # no newline
        got = self.show('C5 API log, final reservation 0.8 without newline', self.decide(rec, 0.5))
        self.write(); L.ISSUES.clear(); L.build_pi(None, None, state=os.path.dirname(self.ledger), ledger=self.ledger)
        kinds = sorted({i['kind'] for i in L.ISSUES})
        print(f'C5 issues: {kinds}')
        self.assertNotIn('partial_pi_proxy_row', kinds)
        self.assertEqual(got[0], 'refused')          # the partial row is counted, not dropped: exposure is not lowered


for _n in [n for n in dir(t1.A1) if n.startswith('test_') and n not in Repro.__dict__]:
    setattr(Repro, _n, None)

if __name__ == '__main__':
    unittest.main()
