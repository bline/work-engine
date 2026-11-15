"""P1.26 handoff evidence only (not a repair, not a candidate test): N-07 on the journal commitment path. A journal-admitted
session (pending + launched, cap 0.6, empty turn) is retired, then a delivery reopens it (role_host allows delivery after
retirement). Outcome: (decision, committed_usd, exposure_usd) for a new 0.6 session against an objective limit of 1.0.
Run gated with READER_TREE=<tree>; helpers are that tree's tests."""
import json, os, sys, unittest
from unittest import mock
R = os.environ['READER_TREE']
sys.path.insert(0, os.path.join(R, 'metrics/scripts')); sys.path.insert(1, os.path.join(R, 'metrics/scripts/tests'))
import usage_ledger as L  # noqa: E402
import test_usage_ledger_pi_s2 as T  # noqa: E402
assert T.L is L
print('# reader', L.digest(os.path.join(R, 'metrics/scripts/usage_ledger.py'))[:16], file=sys.stderr)

class N07(T.PiTree):
    def run_case(self, retire, deliver):
        jdir = os.path.join(self.d, 'locks'); os.makedirs(jdir, exist_ok=True)
        p = mock.patch.dict(os.environ, {'ROLE_HOST_OBJECTIVE_LOCK_DIR': jdir}); p.start(); self.addCleanup(p.stop)
        rec = self.record(pi_limit=1.0)
        self.session(T.AN, 'root', [[]] + ([[]] if deliver else []))
        launch, extra = self.rows[0], self.rows[1:]; self.rows = [launch]
        if retire: self.rows.append(dict(host='pi', event='retire', session_id=T.AN))
        self.rows += extra
        with open(L.admission_journal_path(rec), 'w') as fh:
            for r in (dict(event='pending', admission_id='a', max_usd=0.6), dict(event='launched', admission_id='a', session_id=T.AN)):
                fh.write(json.dumps(dict(state_ledger=self.ledger, **r)) + '\n')
        out = self.admit(rec, 0.6)
        return out['decision'], out.get('committed_usd'), out.get('exposure_usd')

    def test_journal_path(self):
        for label, retire, deliver in (('claimed: retire, then deliver (reopened)', True, True),
                                       ('control: retire, no deliver', True, False), ('control: no retire', False, False)):
            self.setUp()
            print(f'\nN-07 journal {label}: {self.run_case(retire, deliver)}', file=sys.stderr)

if __name__ == '__main__':
    unittest.main()
