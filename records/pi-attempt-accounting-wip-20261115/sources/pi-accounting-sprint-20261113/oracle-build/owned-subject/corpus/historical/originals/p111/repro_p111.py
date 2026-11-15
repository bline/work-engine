"""Coordinator reproductions of the sealed P1.11 blind findings (A1-01..A1-06, A-05) and the shared-root
commitment shape. Offline; nothing is launched and no code is changed. Each test asserts the DEFECT behaviour,
so a pass means "reproduced". A1 = the frozen P1.10 candidate worktree; BASE = a private copy of deac8099.

Run: env -i ... A1_TREE=<worktree> BASE_LEDGER=<copy> python3 -m unittest repro_p111 -v
"""
import glob
import importlib.util
import json
import os
import shutil
import subprocess
import sys
import tempfile
import unittest
from unittest import mock

A1_TREE = os.environ['A1_TREE']
sys.path.insert(0, os.path.join(A1_TREE, 'metrics/scripts'))
sys.path.insert(0, os.path.join(A1_TREE, 'metrics/scripts/tests'))
import usage_ledger as L  # noqa: E402  (A1)
import test_usage_ledger_pi_s2 as s2  # noqa: E402
import test_usage_ledger_pi_a1 as t1  # noqa: E402

BASE_LEDGER = os.environ['BASE_LEDGER']
ROLE_HOST = os.environ['ROLE_HOST']
reserve, openai, anth, OU = s2.reserve, s2.openai, s2.anth, s2.OU
S1 = t1.S1


def load(path, name):
    spec = importlib.util.spec_from_file_location(name, path)
    m = importlib.util.module_from_spec(spec); spec.loader.exec_module(m); return m


BASE = load(BASE_LEDGER, 'usage_ledger_deac8099')


def upstream_error(rid, charged, at='2026-11-10T06:31:00Z', provider=None):
    """The deployed proxy's row when the upstream connection fails (pi_container.py: settle(None, {}))."""
    row = dict(event='upstream_error', at=at, n=rid, detail='timed out',
               usd=dict(rid=rid, reserved_usd=charged, charged_usd=charged, charge_basis='reservation',
                        session_spent_usd=charged, multiplier_floor=0.0))
    if provider: row['provider'] = provider
    return row


class Repro(t1.A1):
    def admit_with(self, mod, record, new, journal=None):
        self.write()
        real = mod.transcript_available
        ps = [mock.patch.object(mod, 'build_codex', return_value=([], [], [], 0)),
              mock.patch.object(mod, 'build_claude', return_value=([], [], 0)),
              mock.patch.object(mod, 'unrecorded_claude', return_value=[]),
              mock.patch.object(mod, 'transcript_available',
                                side_effect=lambda p, sid, *a, **kw: True if sid == 'root' else real(p, sid, *a, **kw))]
        for p in ps: p.start()
        try:
            mod.ISSUES.clear()
            return mod.pi_admission(record, new, 'root', self.ledger, journal)
        finally:
            for p in ps: p.stop()

    def both(self, rec, new):
        a, b = self.admit_with(L, rec, new), self.admit_with(BASE, rec, new)
        return a, b

    # A1-01: deployed upstream_error settlements erase their reservation.
    def test_A1_01a_upstream_error_openai_and_anthropic(self):
        rec = self.record(pi_limit=1.0)
        self.session(S1, 'root', [[reserve(1, 0.8), upstream_error(1, 0.8)]], retired=True)
        self.session(s2.AN, 'root', [[reserve(1, 0.3), upstream_error(1, 0.3, provider='anthropic')]], retired=True)
        a, b = self.both(rec, 0.5)
        print(f"\nA1-01a upstream_error 0.8 + 0.3 kept by the proxy: A1 {a['decision']} spent={a.get('spent_high_usd')}; "
              f"deac8099 {b['decision']} spent={b.get('spent_high_usd')}; conservative exposure would be {0.8 + 0.3 + 0.5:.1f}")
        self.assertEqual((a['decision'], a['spent_high_usd']), ('admitted', 0.0))
        self.assertEqual((b['decision'], b['spent_high_usd']), ('admitted', 0.0))

    def test_A1_01b_unknown_event_with_usd_rid_cancels_a_reservation(self):
        rec = self.record(pi_limit=1.0)
        self.session(S1, 'root', [[reserve(1, 0.8), dict(event='settlement_note', at='2026-11-10T06:31:00Z', usd=dict(rid=1))]], retired=True)
        a = self.admit_with(L, rec, 0.5)
        print(f"\nA1-01b unknown event carrying usd.rid: A1 {a['decision']} spent={a.get('spent_high_usd')}")
        self.assertEqual((a['decision'], a['spent_high_usd']), ('admitted', 0.0))

    def test_A1_01c_contradictory_error_status_charge_is_dropped(self):
        rec = self.record(pi_limit=1.0)
        row = dict(event='request', at='2026-11-10T06:31:00Z', n=1, http_status=429, model='gpt-6.1-sol', response_id=None,
                   usage=dict(t1.NO_USAGE), usd=dict(rid=1, reserved_usd=0.8, charged_usd=0.8, charge_basis='error_status'))
        self.session(S1, 'root', [[reserve(1, 0.8), row]], retired=True)
        a = self.admit_with(L, rec, 0.5)
        print(f"\nA1-01c 429 row with charge_basis error_status but charged 0.8: A1 {a['decision']} spent={a.get('spent_high_usd')}")
        self.assertEqual((a['decision'], a['spent_high_usd']), ('admitted', 0.0))

    # A1-02: inverted neighbour bounds exclude an undated charge.
    def test_A1_02_inverted_bounds_exclude_an_undated_charge(self):
        rec = self.record(pi_limit=1.0, started='2026-11-10T06:30:00Z')
        self.session(S1, 'root', [[reserve(1, 0.8, at='2026-11-10T06:40:00Z'), t1.undated(t1.b2(1, 'resp_i', 0.8)),
                                   reserve(2, 0.0, at='2026-11-10T06:20:00Z')]], retired=True)
        a = self.admit_with(L, rec, 0.5)
        print(f"\nA1-02 bounds [06:40, 06:20] around an undated 0.8 charge, objective from 06:30: A1 {a['decision']} spent={a.get('spent_high_usd')}")
        self.assertEqual((a['decision'], a['spent_high_usd']), ('admitted', 0.0))

    # A1-03: a turn whose role_host record is missing is invisible.
    def test_A1_03_missing_turn_record_hides_its_log(self):
        rec = self.record(pi_limit=1.0)
        self.session(S1, 'root', [[reserve(1, 0.0)], [reserve(1, 0.8), t1.b2(1, 'resp_hidden', 0.8)], [reserve(1, 0.0)]], retired=True)
        self.rows = [r for r in self.rows if r.get('turn') != 2]          # turn 2's log exists, its ledger row does not
        assert os.path.exists(self.log_of(S1, 2))
        a = self.admit_with(L, rec, 0.5)
        print(f"\nA1-03 turn-2 log with a 0.8 charge, no turn-2 ledger row: A1 {a['decision']} spent={a.get('spent_high_usd')} "
              f"(the unmatched 0.0 reservations of turns 1 and 3 are the only rows)")
        self.assertEqual(a['decision'], 'admitted')
        self.assertLess(a['spent_high_usd'], 0.8)

    # A1-05: an Anthropic reservation row with usable usage but http_status null loses its low bound.
    def test_A1_05_null_http_status_drops_the_usage_low_bound(self):
        usage = dict(input_tokens=10, cache_read_input_tokens=0, cache_creation_input_tokens=0,
                     ephemeral_5m_input_tokens=0, ephemeral_1h_input_tokens=0, output_tokens=1)
        row = anth(1, 'msg_null', usage, 0.25); row['usd']['charge_basis'] = 'reservation'; row['http_status'] = None
        self.session(s2.AN, 'root', [[reserve(1, 0.25), row]])
        [r] = self.pi_rows()
        print(f"\nA1-05 http_status null: charge_basis={r['charge_basis']} low={r['usd_low']} high={r['usd_high']} provider_usage={r['provider_usage']}")
        self.assertEqual(r['usd_low'], 0.0)

    # A1-06: legacy OpenAI rows without usd take cache writes from the writable Pi session file.
    def test_A1_06_session_file_cache_write_lowers_a_legacy_charge(self):
        sd = os.path.join(self.d, 'sessions'); os.makedirs(sd)
        legacy = dict(event='request', at='2026-11-10T06:31:00Z', n=1, http_status=200, model='gpt-6.1-sol',
                      response_model='gpt-6.1-sol', observed_tier='default', response_id='r1',
                      usage=dict(input_tokens=100000, cached_tokens=0, output_tokens=0, reasoning_tokens=0), seconds=1.0)
        out = {}
        for cw in (100000, 0):
            self.rows = []
            self.session(S1, 'root', [[legacy]], retired=True)
            self.rows[0]['container_environment'] = dict(session_dir=sd)
            with open(os.path.join(sd, f'x-{S1}.jsonl'), 'w') as fh:
                fh.write(json.dumps(dict(type='message', message=dict(role='assistant', responseId='r1', usage=dict(cacheWrite=cw)))) + '\n')
            rec = self.record(pi_limit=0.24)
            out[cw] = (self.admit_with(L, rec, 0.03), self.admit_with(BASE, rec, 0.03))
        for cw, (a, b) in out.items():
            print(f"\nA1-06 session-file cacheWrite={cw}: A1 {a['decision']} spent={a.get('spent_high_usd'):.4f}; deac8099 {b['decision']} spent={b.get('spent_high_usd'):.4f}")
        self.assertEqual([out[100000][0]['decision'], out[0][0]['decision']], ['refused', 'admitted'])
        self.assertEqual([out[100000][1]['decision'], out[0][1]['decision']], ['refused', 'admitted'])

    # Attribution: objectives sharing a root see each other's spend but not each other's commitments.
    def test_shared_root_other_objectives_commitment_is_invisible(self):
        rec_b = os.path.join(self.d, 'b.json')
        with open(self.record(pi_limit=1.0)) as fh: doc = json.load(fh)
        rec_a = os.path.join(self.d, 'record.json')
        with open(rec_b, 'w') as fh: json.dump(doc, fh)
        self.session(S1, 'root', [[reserve(1, 0.0)]], max_usd=1.0)       # B's session: open, cap 1, nothing spent yet
        jb = os.path.join(self.d, 'b.admissions.jsonl')
        with open(jb, 'w') as fh:
            for r in (dict(event='pending', admission_id='b1', max_usd=1.0, objective_record=rec_b, state_ledger=self.ledger),
                      dict(event='launched', admission_id='b1', session_id=S1)):
                fh.write(json.dumps(r) + '\n')
        ja = os.path.join(self.d, 'a.admissions.jsonl')
        a = self.admit_with(L, rec_a, 1.0, ja)
        b = self.admit_with(L, rec_b, 0.5, jb)
        print(f"\nshared root: A admits 1.0 against limit 1.0 -> {a['decision']} (committed {a.get('committed_usd')}); "
              f"B's open session S1 (cap 1.0) is in A's tree too, so A's eventual exposure can reach 2.0. B with its own journal -> {b['decision']}")
        self.assertEqual(a['decision'], 'admitted')
        self.assertIn(S1, {n['node_id'] for n in self.tree(rec_a)['nodes']})


for _name in [n for n in dir(t1.A1) if n.startswith('test_')]:
    setattr(Repro, _name, None)


class LauncherHalt(unittest.TestCase):
    """A1-04 / A-05 through the deployed launcher's own admission function (role_host.objective_admission), which
    runs the reader named by ROLE_HOST_USAGE_LEDGER as a subprocess. No container, no provider."""

    def test_A1_04_halt_ignored_and_charge_dropped_when_the_reader_is_deac8099(self):
        rh = load(ROLE_HOST, 'role_host_deployed')
        src = open(ROLE_HOST).read()
        self.assertNotIn('ADMISSIONS-HALTED', src)
        root = '9cbaebad-2397-4a15-8f8a-cc0d5f74357b'   # the retired P1.11 attempt-1 root; its transcript is real
        tr = glob.glob(os.path.expanduser(f'~/.claude/projects/*/{root}.jsonl'))
        self.assertEqual(len(tr), 1)
        first = next(json.loads(l)['timestamp'] for l in open(tr[0]) if isinstance(json.loads(l).get('timestamp'), str))
        d = tempfile.mkdtemp(dir=os.environ.get('TMPDIR')); self.addCleanup(shutil.rmtree, d)
        sid = '00000000-0000-4000-8000-0000000000e1'
        log = os.path.join(d, 'state', 'pi', sid, 'host', 'turn-1.proxy.jsonl'); os.makedirs(os.path.dirname(log))
        with open(log, 'w') as fh:
            for r in (reserve(1, 0.54105, at=first), t1.b1(1, 0.54105, at=first)):
                fh.write(json.dumps(r) + '\n')
        ledger = os.path.join(d, 'state', 'ledger.jsonl')
        with open(ledger, 'w') as fh:
            for r in (dict(host='pi', event='launch', session_id=sid, turn=1, parent_session_id=root, files=dict(proxy_log=log)),
                      dict(host='pi', event='retire', session_id=sid)):
                fh.write(json.dumps(r) + '\n')
        rec = os.path.join(d, 'record.json')
        with open(rec, 'w') as fh:
            json.dump(dict(scope_budget=dict(objective_id='o', started_at=first, bindings=[],
                                             sessions=[dict(session_id=root, provider='claude', **{'from': first})],
                                             cost_limits=[dict(pool='pi_api', node=None, max_usd=1.0, set_by='user')])), fh)
        jdir = os.path.join(d, 'admissions'); os.makedirs(jdir)
        journal = os.path.join(jdir, 'x.admissions.jsonl')
        open(os.path.join(jdir, 'ADMISSIONS-HALTED'), 'w').write('P1.11 repro\n')
        out = {}
        for label, reader in (('deac8099', BASE_LEDGER), ('A1', os.path.join(A1_TREE, 'metrics/scripts/usage_ledger.py'))):
            with mock.patch.dict(os.environ, {'ROLE_HOST_USAGE_LEDGER': reader}):
                try:
                    r = rh.objective_admission(rec, 0.60, root, ledger, journal)
                    out[label] = ('admitted', r.get('spent_high_usd'))
                except rh.HostError as e:
                    out[label] = (e.status, json.loads(e.detail).get('reasons') if e.detail.startswith('{') else e.detail[:120])
        print(f"\nA1-04 deployed launcher admission, halt file present, retired 0.54105 no-id failure, limit 1, new 0.60: {out}")
        self.assertEqual(out['deac8099'], ('admitted', 0.0))
        self.assertEqual(out['A1'][0], 'objective_accounting_unavailable')


if __name__ == '__main__':
    unittest.main()
