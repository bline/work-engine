"""P1.23 offline reproductions of the sealed independent review (review/sealed/reviewer-response.md, sha256 664be9c0...;
SEAL f7f47f6c...). Each test prints the outcome of the CLAIMED input and of its control and asserts nothing about
whether it is a defect: classification, severity and scope are decided in the report. Outcomes are
(decision, spent_high, spent_low, sorted blocking-issue kinds) from pi_admission with an objective pi_api limit.

READER_TREE: tree whose metrics/scripts/usage_ledger.py is under test (imported first); HELPER_TREE: the frozen
candidate's test helpers. Gated: refuses with a provider key, a live DOCKER_HOST or `pi` on PATH. Temporary fixtures only.
Run: env -i ... READER_TREE=<tree> HELPER_TREE=<tree> python3 -m unittest repro_p123 -v"""
import json
import os
import shutil
import stat
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
reserve, S1, S2, D, FABLE = s2.reserve, t1.S1, t1.S2, '2026-11-10T', s2.FABLE
U12K = dict(input_tokens=0, output_tokens=12000, cached_tokens=0, cache_write_tokens=0, reasoning_tokens=0)


def usage_req(resp_model, charged, tier='default', rid=1):
    return dict(event='request', at=D + '06:30:01Z', n=rid, http_status=200, model='gpt-6-astra', response_model=resp_model,
                requested_tier=tier, observed_tier=tier, response_id='resp_1', usage=dict(U12K), seconds=1.0,
                usd=dict(rid=rid, reserved_usd=0.8, charged_usd=charged, charge_basis='usage'))


def anth_req(usage, charged, resp='msg_1', basis='usage', reserved=0.3):
    return dict(event='request', provider='anthropic', at=D + '06:30:01Z', n=1, http_status=200, model=FABLE,
                response_model=FABLE, response_model_check='verified', observed_tier='standard', response_id=resp,
                usage=usage, seconds=1.0, usd=dict(rid=1, reserved_usd=reserved, charged_usd=charged, charge_basis=basis))


def anth_usage(i=0, cr=0, cw=0, o=0, **ttl):
    return dict(input_tokens=i, cache_read_input_tokens=cr, cache_creation_input_tokens=cw, output_tokens=o, **ttl)


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

    def one(self, rows, launch_lane=None, max_usd=1.0, new=0.5, launch_extra=None):
        self.reset(); self.session(S1, 'root', [rows], retired=True, max_usd=max_usd)
        if launch_lane == 'subscription': self.rows[0].update(auth_lane='subscription', model='openai-codex/gpt-6-astra')
        if max_usd is None: self.rows[0].pop('usd_limit', None)
        if launch_extra: self.rows[0].update(launch_extra)
        return self.decide(self.record(pi_limit=1.0), new)

    def pools(self):
        return [(c.get('pools'), c.get('usd_low'), c.get('usd_high')) for c in self.pi_rows()]

    # V-01: usage floor priced at response_model rates, not the producer's pinned (launch-model) rates
    def test_V01_response_model_floor(self):
        r = reserve(1, 0.8)
        self.show('V-01 claimed: model astra, response_model terra, 12k output, charged 0.06', self.one([r, usage_req('gpt-6.1-terra', 0.06)]))
        self.show('V-01 control: response_model astra, charged 0.06', self.one([r, usage_req('gpt-6-astra', 0.06)]))
        self.show('V-01 control: response_model astra, charged 0.6 (producer price)', self.one([r, usage_req('gpt-6-astra', 0.6)]))
        self.show('V-01 claimed variant: response_model terra, charged 0.6 (producer price)', self.one([r, usage_req('gpt-6.1-terra', 0.6)]))

    # V-02: Anthropic cache writes without a TTL split: floor at the 5m rate, producer charges max(5m, 1h)
    def test_V02_anthropic_unsplit_cache_write_floor(self):
        r = reserve(1, 0.3)
        self.show('V-02 claimed: cache_creation 1000 unsplit, charged 0.01', self.one([r, anth_req(anth_usage(cw=1000), 0.01)]))
        self.show('V-02 control: charged 0.016 (producer max-rate)', self.one([r, anth_req(anth_usage(cw=1000), 0.016)]))
        self.show('V-02 control: split 1000 as 1h, charged 0.01', self.one([r, anth_req(anth_usage(cw=1000, ephemeral_1h_input_tokens=1000), 0.01)]))
        self.show('V-02 control: charged 0.009 (below both rates)', self.one([r, anth_req(anth_usage(cw=1000), 0.009)]))

    # V-03: a measured stream_error with conflicting labels in a subscription session
    def test_V03_conflicting_measured_stream_error(self):
        meas = dict(rid=1, status='unknown', cost_proxy_usd=None, measurement='api_equivalent_cost_proxy')
        adm = dict(event='subscription_proxy_admitted', at=D + '06:30:00Z', n=1, rid=1, model='gpt-6-astra')
        conflict = dict(event='stream_error', at=D + '06:30:01Z', n=1, provider='openai', auth_lane='api', subscription_proxy=meas)
        plain = dict(event='stream_error', at=D + '06:30:01Z', n=1, subscription_proxy=meas)
        self.show('V-03 claimed: subscription launch, stream_error labelled openai/api, measured', self.one([adm, conflict], 'subscription'))
        self.show('V-03 claimed pools', self.pools())
        self.show('V-03 control: unlabelled measured stream_error (producer shape)', self.one([adm, plain], 'subscription'))
        self.show('V-03 control: same conflicting row, API launch', self.one([adm, conflict]))
        req = dict(event='request', at=D + '06:30:01Z', n=1, http_status=200, model='gpt-6-astra', provider='openai', auth_lane='api',
                   response_id=None, usage={}, subscription_proxy=meas)
        self.show('V-03 control: conflicting measured REQUEST, subscription launch', self.one([adm, req], 'subscription'))
        self.show('V-03 variant: measurement with only an integer rid (no admission row)',
                  self.one([dict(event='stream_error', at=D + '06:30:01Z', n=1, provider='openai', auth_lane='api', subscription_proxy={'rid': 7})], 'subscription'))

    # V-04: a lone stream_error (header-write failure after the upstream answered) in a session without a dollar limit
    def test_V04_lone_stream_error(self):
        se = dict(event='stream_error', at=D + '06:30:01Z', n=1)
        self.show('V-04 claimed: API launch without usd_limit, lone stream_error', self.one([se], max_usd=None))
        self.show('V-04 claimed pi rows', self.pools())
        self.show('V-04 control: API launch without usd_limit, lone upstream_error', self.one([dict(event='upstream_error', at=D + '06:30:01Z', n=1, detail='x')], max_usd=None))
        self.show('V-04 variant: subscription launch, lone unmeasured unlabelled stream_error', self.one([se], 'subscription', max_usd=None))
        self.show('V-04 variant pi rows', self.pools())
        self.show('V-04 control: dollar-limited API session, reservation then lone stream_error', self.one([reserve(1, 0.3), se]))

    # V-05: reservation-basis settlement without a response id loses its usage low bound
    def test_V05_reservation_basis_without_id(self):
        r = reserve(1, 0.3); u = anth_usage(i=10, o=1)
        self.show('V-05 claimed: reservation basis, response_id null, usage i10/o1, charged 0.3', self.one([r, anth_req(u, 0.3, resp=None, basis='reservation')]))
        self.show('V-05 claimed rows', self.pools())
        self.show('V-05 control: same with response_id msg_1', self.one([r, anth_req(u, 0.3, basis='reservation')]))
        self.show('V-05 control rows', self.pools())

    # V-06: n not validated on an unsettled reservation
    def test_V06_unsettled_reservation_attempt(self):
        for n in (True, None, ..., 1):
            r = reserve(1, 0.8)
            if n is ...: r.pop('n')
            else: r['n'] = n
            self.show(f'V-06 unsettled reservation n={n!r}, new 0.1', self.one([r], new=0.1))

    # V-07: an unreadable proxy log of an unrelated session
    def test_V07_unrelated_unreadable_log(self):
        rec = self.record(pi_limit=1.0)
        self.reset()
        self.session(S1, 'root', [[reserve(1, 0.3), anth_req(anth_usage(i=10, o=1), 0.00012)]], retired=True)
        self.session(S2, 'unrelated-parent', [[reserve(1, 0.3)]], retired=True)
        self.show('V-07 control: unrelated log readable', self.decide(rec))
        p = self.log_of(S2); os.chmod(p, 0)
        try:
            readable = os.access(p, os.R_OK)
            self.show('V-07 claimed: unrelated log mode 000 (readable by this uid: %s)' % readable, self.decide(rec))
        finally:
            os.chmod(p, stat.S_IRUSR | stat.S_IWUSR)


class Static(unittest.TestCase):
    def test_V08_expectations_from_the_reader(self):
        a1 = open(os.path.join(H, 'metrics/scripts/tests/test_usage_ledger_pi_a1.py')).read().split('\n')
        m = open(os.path.join(H, 'metrics/scripts/tests/test_usage_ledger_pi_a132_mutation.py')).read().split('\n')
        print('\nV-08 test_usage_ledger_pi_a1.py:292-296:\n' + '\n'.join(f'{i}: {a1[i - 1]}' for i in range(292, 297)), file=sys.stderr)
        print('V-08 test_usage_ledger_pi_a132_mutation.py:85-89:\n' + '\n'.join(f'{i}: {m[i - 1]}' for i in range(85, 90)), file=sys.stderr)


for _n in [n for n in dir(t1.A1) if n.startswith('test_') and n not in Repro.__dict__]:
    setattr(Repro, _n, None)   # only this module's cases run (the candidate's own tests are not reproductions)
