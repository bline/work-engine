"""Coordinator offline reproductions of the sealed P1.13 blind findings. Each test asserts the DEFECT, so "ok"
means reproduced. Every scenario is also run through the deac8099 baseline (BASE) to separate defects A1.1 left
wrong from defects it introduced. Offline; nothing is launched; no code is changed.
Run: env -i ... A11_TREE=<a11 worktree> BASE_LEDGER=<deac8099 copy> python3 -m unittest repro_p113 -v"""
import importlib.util
import json
import os
import sys
import unittest
from unittest import mock

T = os.environ['A11_TREE']
sys.path.insert(0, os.path.join(T, 'metrics/scripts')); sys.path.insert(0, os.path.join(T, 'metrics/scripts/tests'))
import usage_ledger as L  # noqa: E402  (A1.1)
import test_usage_ledger_pi_s2 as s2  # noqa: E402
import test_usage_ledger_pi_a1 as t1  # noqa: E402

def load(path, name):
    spec = importlib.util.spec_from_file_location(name, path); m = importlib.util.module_from_spec(spec); spec.loader.exec_module(m); return m

BASE = load(os.environ['BASE_LEDGER'], 'usage_ledger_deac8099')
reserve, openai, OU = s2.reserve, s2.openai, s2.OU
S1, S2 = t1.S1, t1.S2


class Repro(t1.A1):
    def via(self, mod, rec, new):
        self.write()
        real = mod.transcript_available
        ps = [mock.patch.object(mod, 'build_codex', return_value=([], [], [], 0)), mock.patch.object(mod, 'build_claude', return_value=([], [], 0)),
              mock.patch.object(mod, 'unrecorded_claude', return_value=[]),
              mock.patch.object(mod, 'transcript_available', side_effect=lambda p, sid, *a, **kw: True if sid == 'root' else real(p, sid, *a, **kw))]
        for p in ps: p.start()
        try:
            mod.ISSUES.clear(); out = mod.pi_admission(rec, new, 'root', self.ledger, os.path.join(self.d, 'j.jsonl'))
            return out['decision'], out.get('spent_high_usd'), sorted({i['kind'] for i in out.get('blocking_issues') or []})
        finally:
            for p in ps: p.stop()

    def both(self, label, rec, new):
        a, b = self.via(L, rec, new), self.via(BASE, rec, new)
        print(f"\n{label}: A1.1 {a}; deac8099 {b}")
        return a, b

    def test_R01_dated_clock_conflict_erases_a_reservation(self):
        rec = self.record(pi_limit=1.0, started='2026-11-10T06:30:00Z')
        self.session(S1, 'root', [[reserve(1, 0.8, at='2026-11-10T06:40:00Z'), t1.b2(1, 'resp_r', 0.8, at='2026-11-10T06:20:00Z')]], retired=True)
        a, b = self.both('R-01 reserve 06:40, settlement 06:20, objective from 06:30', rec, 0.5)
        self.assertEqual(a[:2], ('admitted', 0.0))

    def test_R02_malformed_charge_cancels_a_reservation(self):
        rec = self.record(pi_limit=1.0)
        row = dict(event='request', at='2026-11-10T06:31:00Z', n=1, http_status=429, model='gpt-6.1-sol', response_id=None,
                   usage=dict(t1.NO_USAGE), usd=dict(rid=1, charge_basis='usage', charged_usd='0.8'))
        self.session(S1, 'root', [[reserve(1, 0.8), row]], retired=True)
        a, b = self.both("R-02 429 settlement with charged_usd '0.8' (string)", rec, 0.5)
        self.assertEqual(a[:2], ('admitted', 0.0))

    def test_R03_orphan_log_for_a_recorded_turn_number(self):
        rec = self.record(pi_limit=1.0)
        a_dir = os.path.join(self.d, 'state', 'pi', S1, 'host'); b_dir = os.path.join(self.d, 'state', 'pi', S1, 'host-moved')
        os.makedirs(a_dir); os.makedirs(b_dir)
        def write(p, rows):
            with open(p, 'w') as fh: fh.write(''.join(json.dumps(r) + '\n' for r in rows))
        write(os.path.join(a_dir, 'turn-1.proxy.jsonl'), [reserve(1, 0.0)])
        write(os.path.join(b_dir, 'turn-2.proxy.jsonl'), [reserve(1, 0.0)])
        write(os.path.join(a_dir, 'turn-2.proxy.jsonl'), [reserve(1, 0.8), t1.b2(1, 'resp_orphan', 0.8)])   # unreferenced
        self.rows += [dict(host='pi', event='launch', session_id=S1, turn=1, parent_session_id='root', files=dict(proxy_log=os.path.join(a_dir, 'turn-1.proxy.jsonl'))),
                      dict(host='pi', event='deliver', session_id=S1, turn=2, files=dict(proxy_log=os.path.join(b_dir, 'turn-2.proxy.jsonl'))),
                      dict(host='pi', event='retire', session_id=S1)]
        a, b = self.both('R-03 charged A/turn-2 log unreferenced; ledger names A/turn-1 and B/turn-2', rec, 0.5)
        self.assertEqual(a[:2], ('admitted', 0.0))

    def test_R04_reused_reservation_id_overwrites_exposure(self):
        rec = self.record(pi_limit=0.85)
        self.session(S1, 'root', [[reserve(1, 0.8), reserve(1, 0.1)]], retired=True)
        a, b = self.both('R-04 reserve rid 1 for 0.8 then 0.1, no settlement', rec, 0.1)
        self.assertEqual((a[0], round(a[1], 6)), ('admitted', 0.1))

    def test_R05_valid_json_non_object_line_is_skipped(self):
        rec = self.record(pi_limit=1.0)
        self.session(S1, 'root', [[reserve(1, 0.5), openai(1, 'resp_ok', OU, 0.4)]], retired=True)
        with open(self.log_of(S1), 'a') as fh: fh.write('null\n[]\n')
        a, b = self.both('R-05 log with extra lines null and []', rec, 0.1)
        self.assertEqual((a[0], a[2]), ('admitted', []))

    def test_R06_lost_launch_link_hides_a_child(self):
        rec = self.record(pi_limit=1.0)
        self.session(S1, 'root', [[reserve(1, 0.0)], [reserve(1, 0.8), t1.b2(1, 'resp_child', 0.8)]], retired=True)
        self.rows = [r for r in self.rows if r.get('event') != 'launch']          # the child's launch row is lost
        a, b = self.both('R-06 child launch row lost (deliver turn 2 + retire remain), no journal', rec, 0.5)
        self.assertEqual(a[:2], ('admitted', 0.0))

    def test_R09_editing_the_record_limit_flips_admission(self):
        rec = self.record(pi_limit=1.0)
        self.session(S1, 'root', [[reserve(1, 0.5), openai(1, 'resp_ok', OU, 0.9)]], retired=True)
        before = self.via(L, rec, 0.5)
        doc = json.load(open(rec)); doc['scope_budget']['cost_limits'][0]['max_usd'] = 100.0; json.dump(doc, open(rec, 'w'))
        after = self.via(L, rec, 0.5)
        print(f"\nR-09 same-UID edit of the record limit (set_by stays 'user'): before {before}; after {after}")
        self.assertEqual((before[0], after[0]), ('refused', 'admitted'))

    def test_NE_legacy_openai_reservation_without_cache_writes_loses_low_bound(self):
        row = t1.b2(1, 'resp_l', 0.3); row['usage'] = dict(input_tokens=11000, cached_tokens=10000, output_tokens=500, cache_write_tokens=None, reasoning_tokens=0)
        self.session(S1, 'root', [[reserve(1, 0.3), row]])
        [r] = self.pi_rows()
        print(f"\nNE legacy OpenAI reservation row, cache writes missing: basis={r['charge_basis']} low={r['usd_low']} high={r['usd_high']} provider_usage={bool(r.get('provider_usage'))}")
        self.assertEqual((r['charge_basis'], r['usd_low']), ('proxy_reservation_charge', 0.0))


for _n in [n for n in dir(t1.A1) if n.startswith('test_')]:
    setattr(Repro, _n, None)

if __name__ == '__main__':
    unittest.main()
