"""P1.25 offline reproductions of the sealed independent A1.3.3 review (review/sealed/reviewer-response.md,
sha256 587d988d...; SEAL 759f7b26...). Each test prints the outcome of the CLAIMED input and of its controls and asserts
nothing about whether it is a defect: classification, severity and scope are decided in the report.
Outcome: (decision, spent_high, spent_low, sorted blocking-issue kinds) from pi_admission with an objective pi_api limit.

READER_TREE: tree whose metrics/scripts/usage_ledger.py is under test (imported first); HELPER_TREE: the frozen
A1.3.3 candidate's test helpers. Gated: refuses with a provider key, a live DOCKER_HOST or `pi` on PATH.
Temporary fixtures only. Expected amounts are hand-computed from the installed producer rate files
(anthropic_api_rates.json, openai_api_rates.json), never from the reader.
Run: env -i ... READER_TREE=<tree> HELPER_TREE=<tree> python3 -m unittest repro_p125 -v"""
import json
import os
import shutil
import sys
import unittest

assert not any(k in os.environ for k in ('OPENAI_API_KEY', 'ANTHROPIC_API_KEY')), 'provider key present'
assert os.environ.get('DOCKER_HOST') == 'unix:///nonexistent/docker.sock' and shutil.which('pi') is None, 'gates missing'
R, H = os.environ['READER_TREE'], os.environ['HELPER_TREE']
sys.path.insert(0, os.path.join(R, 'metrics/scripts'))
import usage_ledger as L  # noqa: E402
sys.path.insert(1, os.path.join(H, 'metrics/scripts/tests'))
import test_usage_ledger_pi_a1 as t1  # noqa: E402
import test_usage_ledger_pi_s2 as s2  # noqa: E402
assert s2.L is L, 'helpers must use the reader under test'
print('# reader under test:', L.digest(os.path.join(R, 'metrics/scripts/usage_ledger.py'))[:16], file=sys.stderr)
reserve, S1, S2, D, FABLE, SONNET = s2.reserve, t1.S1, t1.S2, '2026-11-10T', s2.FABLE, 'claude-sonnet-5'
# Producer rates (anthropic_api_rates.json): sonnet-5 input 2/M; fable-5-2 input 8/M, output 40/M. openai astra input 10/M, output 50/M.


def anth_usage(i=0, cr=0, cw=0, o=0, **ttl):
    return dict(input_tokens=i, cache_read_input_tokens=cr, cache_creation_input_tokens=cw, output_tokens=o, **ttl)


def refusal_rows(resp_model, resp_id='msg_x', reserved=0.05, usage=None, refused_row=True):
    """The producer's Anthropic identity-refusal pair (pi_container.py:1300-1321): response_model_refused, then the request
    row with model_mismatch and a reservation-basis settlement (settle(None, ...) keeps the reservation)."""
    usage = usage or anth_usage(i=10000)
    rows = []
    if refused_row:
        rows.append(dict(event='response_model_refused', provider='anthropic', at=D + '06:30:01Z', n=1, reason='response_model_mismatch',
                         authorized_model=SONNET, response_model=resp_model, response_id=resp_id, verified_bytes_forwarded=False, detail=None))
    req = dict(event='request', provider='anthropic', at=D + '06:30:01Z', n=1, http_status=200, model=SONNET, requested_tier=None,
               observed_tier=None, terminal=None, response_id=resp_id, response_model=resp_model,
               response_model_check='response_model_mismatch' if resp_model != SONNET else 'verified', usage=usage, seconds=1.0,
               usd=dict(rid=1, reserved_usd=reserved, charged_usd=reserved, charge_basis='reservation'))
    if resp_model != SONNET:
        req['model_mismatch'] = True
    rows.append(req)
    return rows


class Repro(t1.A1):
    def decide(self, rec, new=0.5):
        out = self.admit_j(rec, new, os.path.join(self.d, 'j.jsonl'))
        sh, sl = out.get('spent_high_usd'), out.get('spent_low_usd')
        return (out['decision'], None if sh is None else round(sh, 6), None if sl is None else round(sl, 6),
                sorted(set(self.kinds(out))))

    def show(self, label, got):
        print(f'\n{label}: {got}', file=sys.stderr); return got

    def reset(self):
        self.rows = []; L.ISSUES.clear()
        shutil.rmtree(os.path.join(self.d, 'state', 'pi'), ignore_errors=True)   # no stale turn logs between scenarios

    def one(self, rows, launch_lane=None, max_usd=1.0, new=0.5, limit=1.0, launch_extra=None):
        self.reset(); self.session(S1, 'root', [rows], retired=True, max_usd=max_usd)
        if launch_lane == 'subscription': self.rows[0].update(auth_lane='subscription', model='openai-codex/gpt-6-astra')
        if max_usd is None: self.rows[0].pop('usd_limit', None)
        if launch_extra: self.rows[0].update(launch_extra)
        return self.decide(self.record(pi_limit=limit), new)

    def pools(self):
        return [(c.get('pools'), c.get('usd_low'), c.get('usd_high'), c.get('response_identity'), c.get('proxy_settlement'))
                for c in self.pi_rows()]

    def all_issue_kinds(self):
        self.pi_rows(); return sorted({i.get('kind') for i in L.ISSUES})

    # N-01: reservation fallback deletes the identity-conflict issue and the widened range
    def test_N01_identity_refusal_reservation(self):
        # sonnet 10000 input = 0.02; fable 10000 input = 0.08; reservation 0.05; objective limit 0.10, new 0.04
        for label, rows in (
                ('N-01 claimed: producer refusal pair, response fable, id msg_x', refusal_rows(FABLE)),
                ('N-01 claimed: request row only (no response_model_refused row)', refusal_rows(FABLE, refused_row=False)),
                ('N-01 claimed variant: response fable, no response id', refusal_rows(FABLE, resp_id=None)),
                ('N-01 control: response model = pinned sonnet (no conflict)', refusal_rows(SONNET)),
                ('N-01 control: response fable, reservation 0.10 (above both usage prices)', refusal_rows(FABLE, reserved=0.10))):
            self.show(label + ' [limit 0.10, new 0.04]', self.one([reserve(1, rows[-1]['usd']['reserved_usd'])] + rows, new=0.04, limit=0.10))
            self.show('   rows', self.pools())
            self.show('   all issue kinds', self.all_issue_kinds())

    # N-02: malformed (non-string) Anthropic observed_tier treated as standard
    def test_N02_malformed_anthropic_tier(self):
        u = anth_usage(i=1000, o=500)   # fable: 0.008 + 0.020 = 0.028
        def row(tier, charged=0.028, basis='usage'):
            return dict(event='request', provider='anthropic', at=D + '06:30:01Z', n=1, http_status=200, model=FABLE,
                        response_model=FABLE, response_model_check='verified', observed_tier=tier, response_id='msg_1',
                        usage=u, seconds=1.0, usd=dict(rid=1, reserved_usd=0.3, charged_usd=charged, charge_basis=basis))
        for tier in (7, None, 'standard', 'priority', ['standard'], True):
            self.show(f'N-02 usage basis 0.028, observed_tier={tier!r}', self.one([reserve(1, 0.3), row(tier)]))
        self.show('N-02 producer shape for tier 7: reservation basis 0.3', self.one([reserve(1, 0.3), row(7, 0.3, 'reservation')]))

    # N-03: contradicted labels on an admission (and refused rows) in a subscription session
    def test_N03_contradicted_admission(self):
        adm = dict(event='subscription_proxy_admitted', at=D + '06:30:00Z', n=1, rid=1, model='gpt-6-astra')
        bad = dict(adm, provider='openai', auth_lane='api')
        self.show('N-03 control: producer admission (unlabelled), no measurement', self.one([adm], 'subscription'))
        self.show('   rows', self.pools())
        self.show('N-03 claimed: admission labelled openai/api, no measurement', self.one([bad], 'subscription'))
        self.show('   rows', self.pools())
        self.show('   all issue kinds', self.all_issue_kinds())
        self.show('N-03 claimed: admission labelled anthropic provider', self.one([dict(adm, provider='anthropic')], 'subscription'))
        ref = dict(event='refused', at=D + '06:30:00Z', reason='body not received in time')
        self.show('N-03 control: refused row unlabelled', self.one([ref], 'subscription'))
        self.show('N-03 claimed: refused row labelled openai/api', self.one([dict(ref, provider='openai', auth_lane='api')], 'subscription'))
        self.show('   all issue kinds', self.all_issue_kinds())
        self.show('N-03 claimed: response_model_refused labelled openai/api',
                  self.one([dict(event='response_model_refused', at=D + '06:30:00Z', n=1, provider='openai', auth_lane='api')], 'subscription'))
        self.show('N-03 reference: measured stream_error labelled openai/api after a valid admission',
                  self.one([adm, dict(event='stream_error', at=D + '06:30:01Z', n=1, provider='openai', auth_lane='api',
                                      subscription_proxy=dict(rid=1, status='unknown', cost_proxy_usd=None, measurement='api_equivalent_cost_proxy'))],
                           'subscription'))

    # N-04: reused attempt number hides a header-write stream_error (API, no dollar limit)
    def test_N04_reused_n(self):
        req = dict(event='request', at=D + '06:30:02Z', n=2, http_status=200, model='gpt-6-astra', response_model='gpt-6-astra',
                   requested_tier=None, observed_tier='default', terminal='response.completed', response_id='resp_2',
                   usage=dict(input_tokens=1000, output_tokens=100, cached_tokens=0, cache_write_tokens=0, reasoning_tokens=0), seconds=1.0)
        ref = dict(event='refused', at=D + '06:30:01Z', reason='body not received in time')   # handler A released n=1
        se = lambda n, at='06:30:03Z': dict(event='stream_error', at=D + at, n=n)
        self.show('N-04 claimed: refused(A), request n=2 (B), stream_error n=2 (C header failure)', self.one([ref, req, se(2)], max_usd=None))
        self.show('   rows', self.pools())
        self.show('N-04 claimed: stream_error n=2 before the request n=2', self.one([ref, se(2, '06:30:01Z'), req], max_usd=None))
        self.show('N-04 control: request n=2, stream_error n=3 (distinct number)', self.one([req, se(3)], max_usd=None))
        self.show('N-04 control: request n=2 alone', self.one([req], max_usd=None))

    # N-05: earlier retirement excluded from retirement chronology checks
    def lifecycle(self, events, rows_turn1, rows_turn2=(), started='2026-11-10T06:20:00Z', new=0.5):
        self.reset()
        logs = []
        turns = ((1, rows_turn1), (2, rows_turn2)) if any(ev == 'deliver' for ev, _ in events) else ((1, rows_turn1),)
        for t, rows in turns:
            path = self.log_of(S1, t); os.makedirs(os.path.dirname(path), exist_ok=True)
            open(path, 'w').write(''.join(json.dumps(r) + '\n' for r in rows)); logs.append(path)
        for ev, at in events:
            r = dict(host='pi', event=ev, session_id=S1, at=D + at)
            if ev == 'launch':
                r.update(turn=1, parent_session_id='root', thinking='high', model='openai/gpt-6-astra',
                         files=dict(proxy_log=logs[0]), usd_limit=dict(max_usd=1.0))
            elif ev == 'deliver':
                r.update(turn=2, parent_session_id=None, thinking='high', model='openai/gpt-6-astra',
                         files=dict(proxy_log=logs[1]), usd_limit=dict(max_usd=1.0))
            self.rows.append(r)
        return self.decide(self.record(pi_limit=1.0, started=started), new)

    def test_N05_earlier_retirement(self):
        res = [reserve(1, 0.8, at=D + '06:00:00Z')]
        self.show('N-05 claimed: launch 05:59, res 06:00, retire 06:40, deliver 06:05 (empty), retire 06:10; objective 06:20',
                  self.lifecycle([('launch', '05:59:00Z'), ('retire', '06:40:00Z'), ('deliver', '06:05:00Z'), ('retire', '06:10:00Z')], res))
        self.show('   all issue kinds', self.all_issue_kinds())
        self.show('N-05 control: launch 05:59, res 06:00, retire 06:40 only',
                  self.lifecycle([('launch', '05:59:00Z'), ('retire', '06:40:00Z')], res))
        self.show('N-05 control: launch 05:59, res 06:00, retire 06:10 only (consistent; genuinely before the objective)',
                  self.lifecycle([('launch', '05:59:00Z'), ('retire', '06:10:00Z')], res))
        self.show('N-05 control: monotone clock: retire 06:40, deliver 06:45, retire 06:50',
                  self.lifecycle([('launch', '05:59:00Z'), ('retire', '06:40:00Z'), ('deliver', '06:45:00Z'), ('retire', '06:50:00Z')], res))

    # N-06: OpenAI usage settlement with absent cache counts (producer logs cached_tokens/cache_write_tokens null)
    def test_N06_absent_cache_counts(self):
        def row(cached, cw):
            return dict(event='request', at=D + '06:30:01Z', n=1, http_status=200, model='gpt-6-astra', response_model='gpt-6-astra',
                        requested_tier=None, observed_tier='default', terminal='response.completed', response_id='resp_1',
                        usage=dict(input_tokens=1000, output_tokens=100, cached_tokens=cached, cache_write_tokens=cw, reasoning_tokens=None),
                        seconds=1.0, usd=dict(rid=1, reserved_usd=0.5, charged_usd=0.015, charge_basis='usage'))
        self.show('N-06 claimed: cached null, cache_write null, charged 0.015', self.one([reserve(1, 0.5), row(None, None)]))
        self.show('N-06 variant: cached null, cache_write 0', self.one([reserve(1, 0.5), row(None, 0)]))
        self.show('N-06 control: cached 0, cache_write null', self.one([reserve(1, 0.5), row(0, None)]))
        self.show('N-06 control: cached 0, cache_write 0', self.one([reserve(1, 0.5), row(0, 0)]))

    # N-07: legacy objective_admission commitment stays released after a delivery reopens the session
    def test_N07_legacy_commitment_reopened(self):
        def run(retire, deliver):
            self.reset(); rec = self.record(pi_limit=1.0)
            self.session(S1, 'root', [[]] + ([[]] if deliver else []), max_usd=0.8)
            launch = self.rows[0]; extra = self.rows[1:]; self.rows = [launch]
            self.admit_row(S1, 0.8, rec)
            if retire: self.rows.append(dict(host='pi', event='retire', session_id=S1))
            self.rows += extra   # the deliver row, if any, after the retire row
            return self.decide(rec, 0.5)
        self.show('N-07 claimed: legacy allocation 0.8, retire, then deliver (empty turn); new 0.5', run(True, True))
        self.show('N-07 control: retire, no deliver', run(True, False))
        self.show('N-07 control: no retire', run(False, False))

    # N-08: Haiku long context (reader-only rate; the producer has no claude-haiku-5-5 rate)
    def test_N08_haiku_long_context(self):
        print('\nN-08 producer rate for claude-haiku-5-5:', 'absent (anthropic_api_rates.json lists no haiku model)', file=sys.stderr)
        u = dict(input_tokens=100001, cache_read_input_tokens=0, cache_creation_input_tokens=0, output_tokens=0)
        try:
            self.show('N-08 native_usd(haiku, 100001 input)', L.native_usd(dict(u), 'claude-haiku-5-5'))
            self.show('N-08 native_usd(haiku, 100000 input)', L.native_usd(dict(u, input_tokens=100000), 'claude-haiku-5-5'))
        except Exception as e:
            self.show('N-08 native_usd raised', repr(e))


if __name__ == '__main__':
    unittest.main()
