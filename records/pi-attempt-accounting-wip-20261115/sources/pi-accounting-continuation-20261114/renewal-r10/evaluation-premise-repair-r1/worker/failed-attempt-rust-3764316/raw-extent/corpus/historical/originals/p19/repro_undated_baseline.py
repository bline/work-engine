"""Is the undated-row admission hole pre-existing? Same scenarios through the baseline ledger module."""
import os, sys, unittest
import repro_unit_a as RA

class Undated(RA.Repro):
    def test_baseline_undated_usage_row(self):
        base = RA.load_baseline(); rec = self.record(pi_limit=1.0)
        row = RA.s2.openai(1, 'resp_v', RA.s2.OU, 0.4); row.pop('at')
        self.session(RA.OA2, 'root', [[RA.reserve(1, 0.5), row]], retired=True)
        b = self.admit_with(base, rec, 0.10); c = self.admit_with(RA.L, rec, 0.10)
        print(f"\nundated usage row: baseline={b['decision']} spent_high={b.get('spent_high_usd')}; candidate={c['decision']} spent_high={c.get('spent_high_usd')}")
    def test_baseline_undated_retained_reservation(self):
        base = RA.load_baseline(); rec = self.record(pi_limit=1.0)
        r = RA.reserve(1, 0.5); r.pop('at')
        self.session(RA.OA2, 'root', [[r]], retired=True)
        b = self.admit_with(base, rec, 0.60); c = self.admit_with(RA.L, rec, 0.60)
        print(f"\nundated unmatched reservation: baseline={b['decision']} spent_high={b.get('spent_high_usd')}; candidate={c['decision']} spent_high={c.get('spent_high_usd')}")

for n in [n for n in dir(RA.Repro) if n.startswith('test_A') or n.startswith('test_replay')]:
    setattr(Undated, n, None)
