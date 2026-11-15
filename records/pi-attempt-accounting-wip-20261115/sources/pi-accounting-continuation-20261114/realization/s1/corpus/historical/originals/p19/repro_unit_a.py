"""Coordinator reproductions of the sealed Unit A pass-1 findings (P1.9). Offline; no code is changed.

Run against a tree: REPRO_TREE=<split tree> python3 -m unittest repro_unit_a -v
Each test asserts the DEFECT behaviour, so a pass means "reproduced". Baseline comparisons use REPRO_BASE.
"""
import copy
import importlib.util
import json
import os
import sys
import unittest

TREE = os.environ['REPRO_TREE']
BASE = os.environ.get('REPRO_BASE')
TESTS = os.path.join(TREE, 'metrics/scripts/tests')
sys.path.insert(0, os.path.join(TREE, 'metrics/scripts'))
sys.path.insert(0, TESTS)
import usage_ledger as L  # noqa: E402  (candidate when TREE is the A tree)
import test_usage_ledger_pi_s2 as s2  # noqa: E402
import test_usage_ledger_pi_flex as fx  # noqa: E402

OA2, OA3 = fx.OA2, fx.OA3
reserve, b1, b2 = s2.reserve, fx.b1, fx.b2


def load_baseline():
    spec = importlib.util.spec_from_file_location('usage_ledger_baseline', os.path.join(BASE, 'metrics/scripts/usage_ledger.py'))
    m = importlib.util.module_from_spec(spec); spec.loader.exec_module(m); return m


def recon_row(source, decision, at, **kw):
    return dict(schema='pi-reconciliation-v1', source=source, decision=decision, evidence='forged or replayed',
                set_by='user', at=at, **kw)


class Repro(fx.ReservationCharges):
    def admit_with(self, mod, record, new, journal=None):
        """pi_admission through an explicit ledger module (candidate or baseline)."""
        self.write()
        saved = s2.L
        ps = [p for p in self.patches()]
        # patches() targets s2.L; re-target to mod
        from unittest import mock
        real = mod.transcript_available
        avail = lambda provider, sid, *a, **kw: True if sid == 'root' else real(provider, sid, *a, **kw)
        ps = [mock.patch.object(mod, 'build_codex', return_value=([], [], [], 0)),
              mock.patch.object(mod, 'build_claude', return_value=([], [], 0)),
              mock.patch.object(mod, 'unrecorded_claude', return_value=[]),
              mock.patch.object(mod, 'transcript_available', side_effect=avail)]
        for p in ps: p.start()
        try:
            mod.ISSUES.clear()
            return mod.pi_admission(record, new, 'root', self.ledger, journal)
        finally:
            for p in ps: p.stop()

    # A-01: a same-UID process writes set_by='user' and the ledger accepts it as owner authority.
    def test_A01_forged_reconciliation_releases_budget(self):
        rec = self.record(pi_limit=1.0)
        self.session(OA2, 'root', [[reserve(1, 0.54105), b1(1, 0.54105)]], retired=True)
        before = self.admit(rec, 0.60)
        self.recon(recon_row(f'{self.log_of(OA2)}:2', 'confirmed_unbilled', '2026-11-12T10:00:00Z'))
        after = self.admit(rec, 0.60)
        print(f"\nA-01 before forgery: {before['decision']} spent_high={before.get('spent_high_usd')}; "
              f"after forgery: {after['decision']} spent_high={after.get('spent_high_usd')} exposure={after.get('exposure_usd')}; "
              f"conservative exposure = {0.54105 + 0.60:.5f}")
        self.assertEqual(before['decision'], 'refused')
        self.assertEqual(after['decision'], 'admitted')

    # A-02: last-append-wins; replaying an old zero decision reverses a later bill.
    def test_A02_replayed_decision_reverses_bill(self):
        self.session(OA2, 'root', [[reserve(1, 0.54105), b1(1, 0.54105)]], retired=True)
        src = f'{self.log_of(OA2)}:2'
        first = recon_row(src, 'confirmed_unbilled', '2026-11-12T10:00:00Z')
        self.recon(first, recon_row(src, 'provider_billed', '2026-11-13T10:00:00Z', provider_usd=0.20), first)
        [row] = [r for r in self.pi_rows() if r['source'] == src]
        print(f"\nA-02 after unbilled, billed 0.20, replay of unbilled: usd_high={row['usd_high']} billing={row['billing']} "
              f"history={row['reconciliation_history']}; issues={[i['kind'] for i in L.ISSUES]}")
        self.assertEqual(row['usd_high'], 0.0)

    # A-02 variant: timestamps are ignored; an older-dated decision appended later wins.
    def test_A02b_older_timestamp_appended_later_wins(self):
        self.session(OA2, 'root', [[reserve(1, 0.54105), b1(1, 0.54105)]], retired=True)
        src = f'{self.log_of(OA2)}:2'
        self.recon(recon_row(src, 'provider_billed', '2026-11-13T10:00:00Z', provider_usd=0.20),
                   recon_row(src, 'confirmed_unbilled', '2026-11-01T10:00:00Z'))
        [row] = [r for r in self.pi_rows() if r['source'] == src]
        print(f"\nA-02b older-dated unbilled appended after a billed decision: usd_high={row['usd_high']}")
        self.assertEqual(row['usd_high'], 0.0)

    # A-03: a reconciliation line without its newline is accepted (visible before any durable commit), and a
    # lost append after an admission leaves exposure above the limit.
    def test_A03_unterminated_line_accepted_and_lost_append_overcommits(self):
        rec = self.record(pi_limit=1.0)
        self.session(OA2, 'root', [[reserve(1, 0.80), b2(1, 'resp_x', 0.80)]], retired=True)
        path = os.path.join(self.d, 'pi-reconciliations.jsonl')
        with open(path, 'w') as fh:   # no trailing newline, no fsync: an append still in progress
            fh.write(json.dumps(recon_row(f'{self.log_of(OA2)}:2', 'confirmed_unbilled', '2026-11-12T10:00:00Z')))
        journal = os.path.join(self.d, 'journal.jsonl')
        first = self.admit_with(L, rec, 0.90, journal)
        with open(journal, 'w') as fh:   # what role_host appends under the lock after an admission
            fh.write(json.dumps(dict(event='pending', admission_id='new', max_usd=0.90, objective_record=rec)) + '\n')
        os.remove(path)               # the append never became durable; restart sees no reconciliation
        later = self.admit_with(L, rec, 0.01, journal)
        print(f"\nA-03 with unterminated line visible: {first['decision']} (spent_high={first.get('spent_high_usd')}); "
              f"after the append is lost: spent_high={later.get('spent_high_usd')} committed={later.get('committed_usd')} "
              f"spent+committed={later.get('spent_high_usd', 0) + later.get('committed_usd', 0):.2f} against limit 1.0")
        self.assertEqual(first['decision'], 'admitted')
        self.assertGreater(later['spent_high_usd'] + later['committed_usd'], 1.0)

    def test_A03b_no_reconciliation_writer_in_cli(self):
        src = open(L.__file__).read()
        cmds = [l for l in src.splitlines() if 'sp.add_parser(' in l]
        print('\nA-03b CLI subcommands:', [c.split("add_parser('")[1].split("'")[0] for c in cmds if "add_parser('" in c])
        self.assertFalse(any('reconcil' in c and 'trial' not in c for c in cmds))

    # A-04: an undated reservation settlement is dropped from admission spend.
    def test_A04_undated_reservation_charge_admitted_at_zero(self):
        rec = self.record(pi_limit=1.0)
        row = b2(1, 'resp_u', 0.356727); row.pop('at')
        self.session(OA2, 'root', [[reserve(1, 0.356727), row]], retired=True)
        out = self.admit(rec, 0.80)
        print(f"\nA-04 undated reservation charge: {out['decision']} spent_high={out.get('spent_high_usd')} "
              f"tree_status={out.get('tree_status')}; conservative exposure = {0.356727 + 0.80:.6f}")
        self.assertEqual(out['decision'], 'admitted')
        self.assertEqual(out['spent_high_usd'], 0.0)

    def test_A04_baseline_undated_usage_row_blocks(self):
        """Contrast: an undated usage-basis row is unpriced (valid_timestamp) and blocks, in baseline and candidate."""
        rec = self.record(pi_limit=1.0)
        row = s2.openai(1, 'resp_v', s2.OU, 0.4); row.pop('at')
        self.session(OA2, 'root', [[reserve(1, 0.5), row]], retired=True)
        out = self.admit(rec, 0.10)
        print(f"\nA-04 contrast, undated usage row (candidate): {out['decision']}")
        self.assertEqual(out['decision'], 'not_established')

    # A-05: restoring the baseline reader drops the no-response-id reservation charge from spend.
    @unittest.skipUnless(BASE, 'REPRO_BASE not set')
    def test_A05_baseline_reader_undercounts_after_rollback(self):
        base = load_baseline()
        rec = self.record(pi_limit=1.0)
        self.session(OA2, 'root', [[reserve(1, 0.54105), b1(1, 0.54105)]], retired=True)
        cand = self.admit_with(L, rec, 0.60)
        old = self.admit_with(base, rec, 0.60)
        print(f"\nA-05 candidate: {cand['decision']} spent_high={cand.get('spent_high_usd')}; "
              f"baseline (rollback): {old['decision']} spent_high={old.get('spent_high_usd')}")
        self.assertEqual(cand['decision'], 'refused')
        self.assertEqual((old['decision'], old['spent_high_usd']), ('admitted', 0.0))

    # A-06: an Anthropic stream that ended before message_stop (usage present, charge_basis reservation) loses
    # its usage and usage-backed low bound under the candidate.
    @unittest.skipUnless(BASE, 'REPRO_BASE not set')
    def test_A06_anthropic_retained_stream_loses_usage(self):
        base = load_baseline()
        usage = dict(input_tokens=10, cache_read_input_tokens=0, cache_creation_input_tokens=0,
                     ephemeral_5m_input_tokens=0, ephemeral_1h_input_tokens=0, output_tokens=1)
        req = s2.anth(1, 'msg_partial', usage, 0.25); req['usd']['charge_basis'] = 'reservation'; req['http_status'] = None
        req2 = copy.deepcopy(req); req2['http_status'] = 200
        out = {}
        for label, r in (('http_status None', req), ('http_status 200', req2)):
            self.rows = []
            self.session(s2.AN, 'root', [[reserve(1, 0.25), r]])
            self.write()
            for name, mod in (('candidate', L), ('baseline', base)):
                mod.ISSUES.clear()
                rows, _ = mod.build_pi(None, None, state=os.path.dirname(self.ledger), ledger=self.ledger)
                rr = [x for x in rows if x['thread'] == s2.AN]
                out[(label, name)] = [(x.get('charge_basis'), x.get('usage') not in ({}, None), x.get('usd_low'), x.get('usd_high'),
                                       x.get('usd')) for x in rr] + [sorted({i['kind'] for i in mod.ISSUES})]
        for k, v in out.items(): print(f"\nA-06 {k}: {v}")
        cand200 = out[('http_status 200', 'candidate')][0]
        base200 = out[('http_status 200', 'baseline')][0]
        self.assertFalse(cand200[1]); self.assertTrue(base200[1])

    # A-07: one malformed line in the shared reconciliation file blocks an unrelated objective.
    def test_A07_malformed_line_blocks_unrelated_objective(self):
        rec = self.record(pi_limit=5.0)
        self.session(OA2, 'root', [[reserve(1, 0.5), s2.openai(1, 'resp_ok', s2.OU, 0.4)]], retired=True)
        clean = self.admit(rec, 0.1)
        with open(os.path.join(self.d, 'pi-reconciliations.jsonl'), 'w') as fh:
            fh.write('{"schema": "pi-reconciliation-v1", "source": "/some/other/objective/turn-1.proxy.jsonl:2", "deci')
        blocked = self.admit(rec, 0.1)
        print(f"\nA-07 unrelated objective, no failed requests: before={clean['decision']} "
              f"after a truncated line for another objective={blocked['decision']}")
        self.assertEqual((clean['decision'], blocked['decision']), ('admitted', 'not_established'))

    # Replay note from pass 1: the P1.7 replay test omits the review session fixture.
    def test_replay_full_p17_objective_including_review(self):
        def rows(name):
            with open(os.path.join(fx.P17, name)) as fh:
                return [json.loads(l) for l in fh if l.strip()]
        self.session(OA2, 'root', [rows('author-turn-1.proxy.jsonl'), rows('author-turn-2.proxy.jsonl')], retired=True)
        self.session(OA3, 'root', [rows('repair-turn-1.proxy.jsonl')])
        self.session('00000000-0000-4000-8000-0000000000b4', 'root', [rows('review-turn-1.proxy.jsonl')])
        t = self.tree(self.record(started='2026-11-11T18:00:00Z'))['totals']['pi_api']
        charged = sum(r['usd']['charged_usd'] for n in ('author-turn-1', 'author-turn-2', 'repair-turn-1', 'review-turn-1')
                      for r in rows(f'{n}.proxy.jsonl') if r.get('event') == 'request' and isinstance(r.get('usd'), dict))
        print(f"\nfull P1.7 replay (4 logs): high={t['quota_usd_high']:.6f} low={t['quota_usd_low']:.6f} "
              f"unpriced={t['unpriced_calls']}; sum of proxy charged_usd={charged:.6f}")
        self.assertEqual(t['unpriced_calls'], 0)
        self.assertAlmostEqual(t['quota_usd_high'], charged, places=5)


for _name in [n for n in dir(fx.ReservationCharges) if n.startswith('test_r')]:
    setattr(Repro, _name, None)

if __name__ == '__main__':
    unittest.main()
