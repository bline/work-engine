"""P1.27 design-checkpoint probes on the frozen A1.3.4 reader (read-only, temporary fixtures). Prints outcomes; no assertions.
H-a: a non-identity diagnostic raised inside a pricing helper still selects the reservation-only representation (low -> 0).
H-b: validation findings depend on the requested time window (accounting facts computed inside attribution).
Run gated with READER_TREE=<tree>."""
import os, sys, json, unittest
R = os.environ['READER_TREE']
sys.path.insert(0, os.path.join(R, 'metrics/scripts')); sys.path.insert(1, os.path.join(R, 'metrics/scripts/tests'))
import usage_ledger as L  # noqa
import test_usage_ledger_pi_a134 as T  # noqa
assert T.L is L
print('# reader', L.digest(os.path.join(R, 'metrics/scripts/usage_ledger.py'))[:16], file=sys.stderr)
D = T.D
class Probe(T.A134):
    def rows_and_kinds(self):
        rows = self.pi_rows(); return [(r.get('charge_basis'), r.get('usd_low'), r.get('usd_high')) for r in rows], sorted({i['kind'] for i in L.ISSUES})
    def test_Ha(self):
        base = T.refusal(T.FABLE, T.FABLE, 0.3, usage=dict(input_tokens=1000, cache_read_input_tokens=0, cache_creation_input_tokens=1000,
                         output_tokens=0, ephemeral_5m_input_tokens=1000))
        for label, w1 in (('consistent split (5m 1000 of 1000)', 0), ('inconsistent split (5m 1000 + 1h 500 of 1000)', 500)):
            rows = json.loads(json.dumps(base)); rows[-1]['usage']['ephemeral_1h_input_tokens'] = w1
            print(f'\nH-a {label}: admission {self.one(rows)}; rows/kinds {self.rows_and_kinds()}', file=sys.stderr)
    def test_Hb(self):
        rows = T.refusal(T.SONNET, T.FABLE, 0.05)
        for started in ('2026-11-10T06:00:00Z', '2026-11-10T07:00:00Z'):
            self.fresh(); self.session(T.S1, 'root', [rows], retired=True)
            out = self.decide(self.record(pi_limit=1.0, started=started))
            print(f'\nH-b identity-refusal session, objective from {started}: {out}; issue kinds raised {sorted({i["kind"] for i in L.ISSUES})}', file=sys.stderr)
if __name__ == '__main__': unittest.main()
