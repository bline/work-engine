"""Ledger side of atomic admission: journal commitments, receipts, corruption."""
import json
import os
import sys
import unittest
from unittest import mock

sys.path.insert(0, os.path.dirname(os.path.dirname(__file__)))
import test_usage_ledger_pi_s2 as T  # noqa: E402


class Journal(T.PiTree):
    def setUp(self):
        super().setUp()
        self.jdir = os.path.join(self.d, 'locks'); os.makedirs(self.jdir)
        p = mock.patch.dict(os.environ, {'ROLE_HOST_OBJECTIVE_LOCK_DIR': self.jdir}); p.start(); self.addCleanup(p.stop)
        self.rec = self.record(pi_limit=1.0)

    def j(self, *rows, raw=None):
        path = T.L.admission_journal_path(self.rec)
        with open(path, 'a') as fh:
            for r in rows: fh.write(json.dumps(dict(state_ledger=self.ledger, **r)) + '\n')
            if raw: fh.write(raw)

    def test_pending_without_receipt_is_committed_in_full(self):
        self.j(dict(event='pending', admission_id='a', max_usd=0.6, pid=999999, at='2020-01-01T00:00:00Z'))
        out = self.admit(self.rec, 0.6)
        self.assertEqual(out['decision'], 'refused', out)
        self.assertAlmostEqual(out['committed_usd'], 0.6)

    def test_released_receipt_frees_capacity(self):
        self.j(dict(event='pending', admission_id='a', max_usd=0.6), dict(event='released', admission_id='a'))
        self.assertEqual(self.admit(self.rec, 0.6)['decision'], 'admitted')

    def test_launched_session_committed_until_retire(self):
        self.session(T.AN, 'root', [[T.reserve(1, 0.3), T.anth(1, 'msg_1', T.au(), T.T2_USD)]])
        self.j(dict(event='pending', admission_id='a', max_usd=0.6), dict(event='launched', admission_id='a', session_id=T.AN))
        out = self.admit(self.rec, 0.6)
        self.assertEqual(out['decision'], 'refused', out)
        self.assertAlmostEqual(out['exposure_usd'], 1.2)                   # spend + unspent limit + new
        self.rows.append(dict(host='pi', event='retire', session_id=T.AN))
        self.assertEqual(self.admit(self.rec, 0.6)['decision'], 'admitted')

    def test_journal_and_legacy_row_for_one_session_count_once(self):
        self.session(T.AN, 'root', [[T.reserve(1, 0.3), T.anth(1, 'msg_1', T.au(), T.T2_USD)]])
        self.admit_row(T.AN, 0.6, self.rec)
        self.j(dict(event='pending', admission_id='a', max_usd=0.6), dict(event='launched', admission_id='a', session_id=T.AN))
        out = self.admit(self.rec, 0.3)
        self.assertEqual(len(out['commitments']), 1, out)

    def test_corrupted_or_incomplete_journal_is_not_established(self):
        self.j(raw='{oops\n')
        self.assertEqual(self.admit(self.rec, 0.1)['decision'], 'not_established')

    def test_launched_without_pending_is_not_established(self):
        self.j(dict(event='launched', admission_id='b', session_id='x'))
        self.assertEqual(self.admit(self.rec, 0.1)['decision'], 'not_established')

    def test_launched_and_released_is_not_established(self):
        self.j(dict(event='pending', admission_id='c', max_usd=0.2), dict(event='launched', admission_id='c', session_id='x'),
               dict(event='released', admission_id='c'))
        self.assertEqual(self.admit(self.rec, 0.1)['decision'], 'not_established')


if __name__ == '__main__':
    unittest.main()
