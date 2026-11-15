"""Coordinator offline reproductions of the sealed P1.15 blind findings (S-01 .. S-08) against frozen A1.2.

Each `test_S..` asserts the DEFECT as the reviewer stated it, so "ok" means reproduced. Each has a control that
changes only the input the finding blames, to show that input is the cause. Every scenario also runs through the
deac8099 reader (BASE) to separate defects A1.2 introduced from inherited ones.

Gated: refuses to run unless no provider key is set, DOCKER_HOST points at a nonexistent socket and `pi` is not on
PATH. Nothing is launched; the reader is called in-process on temporary fixtures; no code is changed.
Run: run-gated.sh (A12_TREE=<a12 worktree> BASE_LEDGER=<deac8099 copy> python3 -m unittest repro_p115 -v)
"""
import importlib.util
import json
import os
import shutil
import sys
import unittest
from unittest import mock

# ---- gates: offline only
assert not any(k in os.environ for k in ('OPENAI_API_KEY', 'ANTHROPIC_API_KEY')), 'provider key present'
assert os.environ.get('DOCKER_HOST') == 'unix:///nonexistent/docker.sock', 'DOCKER_HOST gate missing'
assert not os.path.exists('/nonexistent/docker.sock'), 'docker socket gate missing'
assert shutil.which('pi') is None, 'pi on PATH'

T = os.environ['A12_TREE']
sys.path.insert(0, os.path.join(T, 'metrics/scripts')); sys.path.insert(0, os.path.join(T, 'metrics/scripts/tests'))
import usage_ledger as L  # noqa: E402  (A1.2)
import test_usage_ledger_pi_s2 as s2  # noqa: E402
import test_usage_ledger_pi_a1 as t1  # noqa: E402


def load(path, name):
    spec = importlib.util.spec_from_file_location(name, path); m = importlib.util.module_from_spec(spec); spec.loader.exec_module(m); return m


BASE = load(os.environ['BASE_LEDGER'], 'usage_ledger_deac8099')
reserve, openai, OU = s2.reserve, s2.openai, s2.OU
S1, S2, S3 = t1.S1, t1.S2, t1.S3
AT = '2026-11-10T06:31:00Z'


def upstream(rid, reserved, charged, at=AT, basis='reservation', **usd_extra):
    usd = dict(rid=rid, reserved_usd=reserved, charged_usd=charged, charge_basis=basis, **usd_extra)
    if reserved is None: usd.pop('reserved_usd')
    return dict(event='upstream_error', at=at, n=rid, detail='timed out', usd=usd)


def request(rid, status, charged, basis, reserved=0.8, at=AT, n=None, response_id=None):
    return dict(event='request', at=at, n=rid if n is None else n, http_status=status, model='gpt-6.1-sol', requested_tier='flex',
                observed_tier=None, response_id=response_id, usage=dict(t1.NO_USAGE), seconds=1.0,
                usd=dict(rid=rid, reserved_usd=reserved, charged_usd=charged, charge_basis=basis))


class Repro(t1.A1):
    journal_rows = ()

    def via(self, mod, rec, new):
        self.write()
        jpath = os.path.join(self.d, 'j.jsonl')
        with open(jpath, 'w') as fh: fh.write(''.join(json.dumps(r) + '\n' for r in self.journal_rows))
        real = mod.transcript_available
        ps = [mock.patch.object(mod, 'build_codex', return_value=([], [], [], 0)), mock.patch.object(mod, 'build_claude', return_value=([], [], 0)),
              mock.patch.object(mod, 'unrecorded_claude', return_value=[]),
              mock.patch.object(mod, 'transcript_available', side_effect=lambda p, sid, *a, **kw: True if sid == 'root' else real(p, sid, *a, **kw))]
        for p in ps: p.start()
        try:
            mod.ISSUES.clear(); out = mod.pi_admission(rec, new, 'root', self.ledger, jpath)
            sh = out.get('spent_high_usd')
            return (out['decision'], None if sh is None else round(sh, 6), sorted({i['kind'] for i in out.get('blocking_issues') or []}),
                    out.get('committed_usd'))
        finally:
            for p in ps: p.stop()

    def both(self, label, rec, new):
        a, b = self.via(L, rec, new), self.via(BASE, rec, new)
        print(f"\n{label}:\n    A1.2     {a}\n    deac8099 {b}")
        return a, b

    def all_issue_kinds(self, rec):
        """Every issue A1.2 raised while building the tree, including ones scoped outside the objective."""
        self.write(); ps = self.patches()
        for p in ps: p.start()
        try:
            L.ISSUES.clear(); doc = L.cost_tree(rec, role_ledger=self.ledger)[0]
            return sorted(i['kind'] for i in doc['issues']), doc['outside']['issues']
        finally:
            for p in ps: p.stop()

    # ------------------------------------------------------------------ S-01
    def test_S01_reservation_basis_settlement_below_the_reservation_releases_it(self):
        rec = self.record(pi_limit=1.0)
        cases = [('upstream_error charged 0 (basis reservation)', upstream(1, 0.8, 0.0)),
                 ('upstream_error charged 0, reserved_usd omitted', upstream(1, None, 0.0)),
                 ('upstream_error charged 0.25 of 0.8', upstream(1, 0.8, 0.25)),
                 ('request basis reservation charged 0', request(1, 200, 0.0, 'reservation'))]
        got = {}
        for label, row in cases:
            self.rows = []
            self.session(S1, 'root', [[reserve(1, 0.8), row]], retired=True)
            got[label] = self.both(f'S-01 reserve 0.8 then {label}; limit 1, new 0.5', rec, 0.5)
        self.rows = []
        self.session(S1, 'root', [[reserve(1, 0.8), upstream(1, 0.8, 0.8)]], retired=True)
        control = self.both('S-01 control: upstream_error retains the full 0.8', rec, 0.5)
        for label, (a, _) in got.items():
            self.assertEqual(a[0], 'admitted', label); self.assertLess(a[1], 0.8 - 1e-9, label)
        self.assertEqual(control[0][:2], ('refused', 0.8))

    # ------------------------------------------------------------------ S-02
    def test_S02_settlement_appended_before_its_reservation_releases_it(self):
        rec = self.record(pi_limit=1.0)
        settle = request(1, 429, 0.0, 'error_status', at=AT, n=1)
        self.session(S1, 'root', [[settle, dict(reserve(1, 0.8, at=AT), n=2)]], retired=True)
        a, _ = self.both('S-02 429 settlement (n=1) appended before usd_reserved (n=2) for rid 1, same second', rec, 0.5)
        self.rows = []
        self.session(S1, 'root', [[dict(reserve(1, 0.8, at=AT), n=2)]], retired=True)
        control = self.both('S-02 control: the reservation alone', rec, 0.5)
        self.rows = []
        self.session(S1, 'root', [[reserve(1, 0.8, at=AT), request(1, 429, 0.0, 'error_status', at=AT, n=7)]], retired=True)
        nmis = self.both('S-02 note: reservation n=1 then 429 settlement with n=7 (attempt number differs)', rec, 0.5)
        self.assertEqual(a[:2], ('admitted', 0.0))
        self.assertEqual(control[0][:2], ('refused', 0.8))
        self.assertEqual(nmis[0][:2], ('admitted', 0.0))

    # ------------------------------------------------------------------ S-03
    def test_S03_future_dated_settlement_removes_an_in_window_obligation(self):
        rec = self.record(pi_limit=1.0)
        self.session(S1, 'root', [[reserve(1, 0.8, at=AT), upstream(1, 0.8, 0.8, at='2099-01-01T00:00:00Z')]], retired=True)
        a, _ = self.both('S-03 reserve 0.8 at 2026-11-10, reservation settlement 0.8 stamped 2099', rec, 0.5)
        print('    A1.2 issues (in scope, outside):', self.all_issue_kinds(rec))
        self.rows = []
        self.session(S1, 'root', [[reserve(1, 0.8, at=AT), upstream(1, 0.8, 0.8, at='2026-11-10T06:32:00Z')]], retired=True)
        control = self.both('S-03 control: same settlement stamped one minute later', rec, 0.5)
        self.assertEqual(a[:3], ('admitted', 0.0, []))
        self.assertEqual(control[0][:2], ('refused', 0.8))

    # ------------------------------------------------------------------ S-04
    def test_S04_unrelated_next_row_closes_an_unsettled_reservation(self):
        rec = self.record(pi_limit=1.0, started='2026-11-10T06:30:00Z')
        refused = dict(event='refused', at='2026-11-10T06:29:59Z', n=2, reason='model not allowed')
        self.session(S1, 'root', [[reserve(1, 0.8, at='2026-11-10T06:29:58Z'), refused]], retired=True)
        a, _ = self.both('S-04 objective from 06:30; reserve 0.8 at 06:29:58 (never settled), refused (other request) at 06:29:59', rec, 0.5)
        print('    A1.2 issues (in scope, outside):', self.all_issue_kinds(rec))
        self.rows = []
        self.session(S1, 'root', [[reserve(1, 0.8, at='2026-11-10T06:29:58Z')]], retired=True)
        control = self.both('S-04 control: the same unsettled reservation without the unrelated row', rec, 0.5)
        self.assertEqual(a[:3], ('admitted', 0.0, []))
        # straddling the start with no closing row, A1.2 fails closed (first run expected 'refused'; corrected)
        self.assertEqual(control[0][:3], ('not_established', None, ['pi_undated_call_unattributable']))

    # ------------------------------------------------------------------ S-05
    def test_S05a_turn_conflict_under_another_sessions_path_is_scoped_away(self):
        rec = self.record(pi_limit=1.0)
        self.session(S2, 'root', [[reserve(1, 0.5), openai(1, 'resp_b', OU, 0.1)]], retired=True)
        other = os.path.join(self.d, 'state', 'pi', S1, 'host', 'turn-1.proxy.jsonl')
        os.makedirs(os.path.dirname(other))
        with open(other, 'w') as fh:
            fh.write(''.join(json.dumps(r) + '\n' for r in [reserve(1, 0.8), upstream(1, 0.8, 0.8)]))
        retire = self.rows.pop()
        self.rows += [dict(host='pi', event='deliver', session_id=S2, turn=1, files=dict(proxy_log=other)), retire]
        a, _ = self.both("S-05a member S2's turn 1 also recorded with a 0.8-charged log under S1's path", rec, 0.5)
        print('    A1.2 issues (in scope, outside):', self.all_issue_kinds(rec))
        own = os.path.join(self.d, 'state', 'pi', S2, 'host', 'turn-1b.proxy.jsonl')
        os.rename(other, own); shutil.rmtree(os.path.dirname(other))
        self.rows[-2]['files']['proxy_log'] = own
        control = self.both("S-05a control: the same conflicting log under S2's own path", rec, 0.5)
        self.assertEqual((a[0], a[1]), ('admitted', 0.1))
        self.assertEqual(control[0][0], 'not_established'); self.assertIn('pi_turn_conflict', control[0][2])

    def test_S05b_orphan_log_of_an_unrelated_session_blocks_this_objective(self):
        rec = self.record(pi_limit=1.0)
        self.session(S1, 'root', [[reserve(1, 0.5), openai(1, 'resp_a', OU, 0.1)]], retired=True)
        logs = os.path.join(self.d, 'state', 'shared-logs'); os.makedirs(logs)
        mine = os.path.join(logs, 'turn-1.proxy.jsonl')
        with open(mine, 'w') as fh: fh.write(json.dumps(reserve(1, 0.0)) + '\n')
        with open(os.path.join(logs, 'stray.proxy.jsonl'), 'w') as fh: fh.write(json.dumps(reserve(9, 0.3)) + '\n')
        self.rows += [dict(host='pi', event='launch', session_id=S3, turn=1, parent_session_id='unrelated-root', files=dict(proxy_log=mine)),
                      dict(host='pi', event='retire', session_id=S3)]
        a, _ = self.both('S-05b unrelated S3 (parent unrelated-root) has an unreferenced log beside its own in a UUID-free directory', rec, 0.1)
        os.remove(os.path.join(logs, 'stray.proxy.jsonl'))
        control = self.both('S-05b control: without the stray log', rec, 0.1)
        self.assertEqual(a[0], 'not_established'); self.assertIn('pi_proxy_log_unreferenced', a[2])
        self.assertEqual(control[0][0], 'admitted')

    # ------------------------------------------------------------------ S-06
    def test_S06_string_http_status_releases_a_reservation_without_a_blocking_issue(self):
        rec = self.record(pi_limit=1.0)
        self.session(S1, 'root', [[reserve(1, 0.8), request(1, '200', 0.0, 'usage')]], retired=True)
        a, _ = self.both("S-06 reserve 0.8, request http_status '200' (string), no response id, usage settlement charged 0", rec, 0.5)
        print('    A1.2 issues (in scope, outside):', self.all_issue_kinds(rec))
        self.rows = []
        self.session(S1, 'root', [[reserve(1, 0.8), request(1, 200, 0.0, 'usage')]], retired=True)
        control = self.both('S-06 control: the same row with http_status 200 (integer)', rec, 0.5)
        self.assertEqual(a[:3], ('admitted', 0.0, []))
        self.assertEqual(control[0][0], 'not_established'); self.assertIn('pi_request_unaccounted', control[0][2])

    # ------------------------------------------------------------------ S-08
    def test_S08_negative_pending_journal_cap_creates_room(self):
        rec = self.record(pi_limit=1.0)
        self.session(S1, 'root', [[reserve(1, 0.0)]], retired=True)
        self.journal_rows = ()
        control = self.both('S-08 control: empty journal, new 1.5 under limit 1', rec, 1.5)
        self.journal_rows = (dict(admission_id='a-neg', event='pending', max_usd=-1),)
        a, b = self.both('S-08 journal pending row max_usd=-1, new 1.5 under limit 1', rec, 1.5)
        self.assertEqual(control[0][0], 'refused')
        self.assertEqual((a[0], a[3]), ('admitted', -1))
        self.assertEqual((b[0], b[3]), ('admitted', -1))


for _n in [n for n in dir(t1.A1) if n.startswith('test_')]:
    setattr(Repro, _n, None)

if __name__ == '__main__':
    unittest.main()
