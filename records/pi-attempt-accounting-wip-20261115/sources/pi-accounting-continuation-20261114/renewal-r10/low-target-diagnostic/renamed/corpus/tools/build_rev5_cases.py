"""Cases added in revision 5 (contract 1.4-s1, P1.33): one targeted case per P1.32 Module A finding, each with a valid
single-condition control. Attempt facts are written here from raw rows and the contract text; evaluations are re-derived
by rev5_lib.finalize. Historical (A1.3.4) blocks for current-producer cases are added separately.
Usage: build_rev5_cases.py <corpus_dir> <out_cases_dir>"""
import copy, json, os, sys
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import rev5_lib as L

corpus, out = sys.argv[1], sys.argv[2]
load = lambda cid: json.load(open(os.path.join(corpus, 'cases', cid + '.json')))
AUTH = 'wev-admission-authority-1'
written = []
def lines(c): return c['inputs']['sessions'][0]['logs']['turn-1']['lines']
def edit_row(c, n, fn):
    ls = lines(c); r = json.loads(ls[n - 1]); fn(r); ls[n - 1] = json.dumps(r)
def att(c): return c['expected']['attempts'][0]
def deriv(model, usage, nano, mult='1', fr='both'):
    return {'for': fr, 'model': model, 'nano': str(nano), 'provider': 'openai', 'tier_multiplier': mult,
            'usage': {k: usage[k] for k in ('input_tokens', 'output_tokens', 'cached_tokens', 'cache_write_tokens')}}


def new(src, cid, *, role, cls, reach, findings, controls, invariants, notes, profile=None):
    c = copy.deepcopy(load(src))
    c.update(case_id=cid, group='P1.33', role=role, findings=findings, controls=controls, invariants=invariants, reachability=reach, notes=notes,
             provenance={'origin': f'contract::P1.33 Module A repair (from {src})'}, rationale=None, ambiguity=None, revisions=[], revisions_r3=[], revisions_r4=[],
             revisions_r5=[], added_in='1.4-s1', contract='pi-accounting-contract/1.4-s1', profile=profile or c['profile'],
             historical={} if (profile or c['profile']) == 'future_qualified_gate' else {})
    c['class'] = cls
    for k in ('twin_of', 'evidence_pair'): c.pop(k, None)
    return c


def finish_attempt(a):
    a['replacement'] = L.replacement(a)
    if a['exposure'].get('pool') == 'pi_api': a['billing'] = 'established' if a['exposure']['high']['kind'] == 'exact' else 'unresolved'


def save(c):
    for a in c['expected']['attempts']: finish_attempt(a)
    L.finalize(c); p = os.path.join(out, c['case_id'] + '.json'); json.dump(c, open(p, 'w'), indent=1, sort_keys=True); open(p, 'a').write('\n'); written.append(c['case_id'])


def rejected(a, reasons, R, extra_blocking=()):
    s = a['settlement']; a['settlement'] = {'decision': 'rejected', 'reasons': reasons, 'reserved': s['reserved'], 'charged': s.get('charged', '0')}
    a['obligation'] = {'state': 'outstanding', 'amount_source': 'reservation_bound'}
    v = a['valuation']; vh = int(v['high']) if v.get('state') == 'valued' else (int(v['priced_high']) if v.get('priced_high') else None)
    hi, basis = L.high_of([(R, 'proxy_reservation'), (int(a['settlement']['charged']), 'proxy_charge'), (vh, 'usage_valuation')])
    a['exposure'] = {'pool': 'pi_api', 'low': {'nano': '0', 'basis': 'none'}, 'high': {'kind': 'at_least', 'nano': str(hi), 'basis': basis}}
    a['blocking'] = sorted({'SettlementRejected', 'UnresolvedExposure', *extra_blocking})


# ---------------- A-01: incomplete usage above a small reservation keeps the usage component, and never replaces R
c = new('P130-R01-usage-incomplete-terminal', 'P133-A01-incomplete-usage-above-reservation', role='finding', cls='producer_valid', reach='producer_reachable',
        findings=['P1.32-A-01'], controls=['P130-R01-usage-incomplete-terminal'], invariants=['INV-F3', 'INV-F13'],
        notes='Single condition changed from P130-R01: the reservation is 0.01, below the reported usage.')
edit_row(c, 1, lambda r: r.update(reserved_usd=0.01)); edit_row(c, 2, lambda r: r['usd'].update(reserved_usd=0.01))
a = att(c); a['settlement']['reserved'] = '10000000'
a['replacement'] = L.replacement(a); a['obligation'] = {'state': 'closed', 'amount_source': 'reservation_bound'}; a['exposure'] = L.retained_exposure(a); save(c)

# ---------------- A-02: an error-status row carrying priceable usage keeps it (synthetic: reachability not established)
c = new('S02-control-error-status-after-reservation', 'P133-A02-error-status-with-priceable-usage', role='finding', cls='adversarial_synthetic', reach='not_established',
        findings=['P1.32-A-02'], controls=['S02-control-error-status-after-reservation'], invariants=['INV-F3', 'INV-F4'],
        notes='Single condition changed from S02-control: the 429 row reports 20000 output tokens. The OpenAI writer takes usage only from a terminal SSE event and settles a non-200 as error status, so this shape is not established as producer-writable.')
U = {'input_tokens': 0, 'output_tokens': 20000, 'cached_tokens': 0, 'cache_write_tokens': 0, 'reasoning_tokens': 0}
edit_row(c, 2, lambda r: r.update(usage=U))
a = att(c); val = 20000 * 50000
a['valuation'] = {'state': 'valued', 'complete': False, 'identities': ['gpt-6-astra'], 'low': str(val), 'high': str(val), 'derivations': [deriv('gpt-6-astra', U, val)]}
a['exposure'] = L.retained_exposure(a); save(c)

# ---------------- A-03: authenticated numerical billing bound (future profile)
def bound_gate(c, amount='500000000', currency='USD', covers=None, tamper=False):
    g = c['inputs']['gates'][0]; covers = covers if covers is not None else list(L.COVERS)
    b = {'amount_nano': amount, 'currency': currency, 'scope': 'session', 'covers': covers,
         'binding_sha256': L.binding(g['gate_id'], g['authority']['id'], g['session'], amount, currency, g['effective_from'], g['effective_until'], sorted(covers))}
    if tamper: b['amount_nano'] = str(int(amount) - 1000)
    g['mechanisms']['billing_bound']['bound'] = b
def fg(cid, role, findings, controls, notes, **kw):
    c = new('P132-FG-VC14-open-session-billing-bound', cid, role=role, cls='future_vertical', reach='producer_unreachable', findings=findings, controls=controls,
            invariants=['INV-A9', 'INV-A8'], notes=notes, profile='future_qualified_gate')
    bound_gate(c, **kw); return c
save(fg('P133-FG-VC14-authenticated-bound', 'valid_control', [], [], 'Open session; the gate binds an enforced USD 0.5 session bound (equal to the cap) covering forwarding, retries, partial streams, failures and settlement.'))
save(fg('P133-FG-bound-exceeds-cap', 'finding', ['P1.32-A-03'], ['P133-FG-VC14-authenticated-bound'], 'The enforced bound (0.6) exceeds the authorized cap (0.5).', amount='600000000'))
save(fg('P133-FG-bound-wrong-currency', 'finding', ['P1.32-A-03'], ['P133-FG-VC14-authenticated-bound'], 'The bound is stated in EUR.', currency='EUR'))
save(fg('P133-FG-bound-coverage-incomplete', 'finding', ['P1.32-A-03'], ['P133-FG-VC14-authenticated-bound'], 'The bound does not cover partial streams.',
        covers=['failures', 'forwarding', 'retries', 'settlement']))
save(fg('P133-FG-bound-binding-mismatch', 'finding', ['P1.32-A-03'], ['P133-FG-VC14-authenticated-bound'], 'The amount was changed after the binding digest was computed.', tamper=True))

# ---------------- A-04: OpenAI cache-count states and mixed observations
c = new('VC06-unbudgeted-completed-request', 'P133-A04-cache-write-absent-legacy', role='finding', cls='historical_legacy', reach='legacy',
        findings=['P1.32-A-04'], controls=['VC06-unbudgeted-completed-request'], invariants=['INV-F5'],
        notes='Single condition changed from VC06: the row has no cache_write_tokens key, as written by producers before ab2a0f84 (59 real unbudgeted rows). The count is unknown, not zero.')
edit_row(c, 1, lambda r: r['usage'].pop('cache_write_tokens'))
a = att(c); lo_u = {'input_tokens': 1000, 'output_tokens': 100, 'cached_tokens': 0, 'cache_write_tokens': 0}; hi_u = dict(lo_u, cache_write_tokens=1000)
a['valuation'] = {'state': 'valued', 'complete': True, 'identities': ['gpt-6-astra'], 'low': '15000000', 'high': '17500000',
                  'derivations': [deriv('gpt-6-astra', lo_u, 15000000, fr='low'), deriv('gpt-6-astra', hi_u, 17500000, fr='high')]}
a['exposure'] = {'pool': 'pi_api', 'low': {'nano': '15000000', 'basis': 'usage_valuation'}, 'high': {'kind': 'exact', 'nano': '17500000', 'basis': 'usage_valuation'}}; save(c)
c = new('VC06-unbudgeted-completed-request', 'P133-A04-cache-counts-both-absent', role='finding', cls='adversarial_synthetic', reach='not_established',
        findings=['P1.32-A-04'], controls=['VC06-unbudgeted-completed-request'], invariants=['INV-F5'],
        notes='Both cache-count keys absent (no OpenAI row in the real corpus has this shape): every vertex of cached + write <= input is enumerated.')
edit_row(c, 1, lambda r: (r['usage'].pop('cache_write_tokens'), r['usage'].pop('cached_tokens')))
a = att(c); lo_u = {'input_tokens': 1000, 'output_tokens': 100, 'cached_tokens': 1000, 'cache_write_tokens': 0}; hi_u = {'input_tokens': 1000, 'output_tokens': 100, 'cached_tokens': 0, 'cache_write_tokens': 1000}
a['valuation'] = {'state': 'valued', 'complete': True, 'identities': ['gpt-6-astra'], 'low': '6000000', 'high': '17500000',
                  'derivations': [deriv('gpt-6-astra', lo_u, 6000000, fr='low'), deriv('gpt-6-astra', hi_u, 17500000, fr='high')]}
a['exposure'] = {'pool': 'pi_api', 'low': {'nano': '6000000', 'basis': 'usage_valuation'}, 'high': {'kind': 'exact', 'nano': '17500000', 'basis': 'usage_valuation'}}; save(c)
c = new('VC01-openai-usage-settlement', 'P133-A04-cached-nonzero', role='valid_control', cls='producer_valid', reach='producer_reachable',
        findings=['P1.32-A-04'], controls=[], invariants=['INV-F11'], notes='Present cache counts (400 cached, 0 written): the split is known; the usage settlement replaces the reservation.')
U = {'input_tokens': 1000, 'output_tokens': 100, 'cached_tokens': 400, 'cache_write_tokens': 0, 'reasoning_tokens': 0}
edit_row(c, 2, lambda r: (r.update(usage=U), r['usd'].update(charged_usd=0.0114, session_spent_usd=0.0114)))
a = att(c); val = 600 * 10000 + 400 * 1000 + 100 * 50000; a['settlement']['charged'] = str(val)
a['valuation'] = {'state': 'valued', 'complete': True, 'identities': ['gpt-6-astra'], 'low': str(val), 'high': str(val), 'derivations': [deriv('gpt-6-astra', U, val)]}
a['exposure'] = {'pool': 'pi_api', 'low': {'nano': str(val), 'basis': 'usage_valuation'}, 'high': {'kind': 'exact', 'nano': str(val), 'basis': 'usage_valuation'}}; save(c)
c = new('VC01-openai-usage-settlement', 'P133-A04-cached-malformed', role='finding', cls='malformed', reach='not_established',
        findings=['P1.32-A-04'], controls=['VC01-openai-usage-settlement'], invariants=['INV-F5'],
        notes='cached_tokens is the string "400": the producer cannot price it, so it keeps the reservation (reservation basis, charged = R); the valuation is Unpriceable{MalformedCount}.')
edit_row(c, 2, lambda r: (r['usage'].update(cached_tokens='400'), r['usd'].update(charged_usd=0.5, charge_basis='reservation', session_spent_usd=0.5)))
a = att(c); a['settlement'] = {'decision': 'valid', 'basis': 'reservation', 'reasons': [], 'reserved': '500000000', 'charged': '500000000'}
a['obligation'] = {'state': 'closed', 'amount_source': 'reservation_confirmed'}
a['valuation'] = {'state': 'unpriceable', 'reasons': ['MalformedCount'], 'supported_low': None, 'priced_high': None}
a['exposure'] = {'pool': 'pi_api', 'low': {'nano': '0', 'basis': 'none'}, 'high': {'kind': 'at_least', 'nano': '500000000', 'basis': 'proxy_reservation'}}
a['blocking'] = ['UnresolvedExposure']; save(c)
c = new('VC01-openai-usage-settlement', 'P133-A04-mixed-observations', role='finding', cls='adversarial_synthetic', reach='producer_unreachable',
        findings=['P1.32-A-04'], controls=['VC01-openai-usage-settlement'], invariants=['INV-F3', 'INV-F5'],
        notes='Two distinct settlements for rid 1: one priced (15M), one with a malformed cache count. V-8: Unpriceable, no supported low, priced_high 15M.')
r2 = json.loads(lines(c)[1]); r2['at'] = '2026-11-10T06:30:05Z'; r2['usage']['cached_tokens'] = '400'; lines(c).append(json.dumps(r2))
a = att(c); a['evidence'].append('S1/turn-1:3')
a['settlement'] = {'decision': 'conflicting', 'reasons': [], 'reserved': '500000000', 'charged': '15000000'}
a['obligation'] = {'state': 'outstanding', 'amount_source': 'reservation_bound'}
a['valuation'] = {'state': 'unpriceable', 'reasons': ['MalformedCount'], 'supported_low': None, 'priced_high': '15000000'}
a['exposure'] = {'pool': 'pi_api', 'low': {'nano': '0', 'basis': 'none'}, 'high': {'kind': 'at_least', 'nano': '500000000', 'basis': 'proxy_reservation'}}
a['blocking'] = ['SettlementConflict', 'UnresolvedExposure']; save(c)

# ---------------- A-05: a case variant of the pinned model is a different identity
c = new('VC01-openai-usage-settlement', 'P133-A05-model-case-variant', role='finding', cls='adversarial_synthetic', reach='not_established',
        findings=['P1.32-A-05'], controls=['VC01-openai-usage-settlement'], invariants=['INV-F5'],
        notes='Single condition changed from VC01: the request model is "GPT-6-Astra". MC-3 compares ids byte for byte, so it differs from the pinned "gpt-6-astra" and has no rate.')
edit_row(c, 2, lambda r: r.update(model='GPT-6-Astra'))
a = att(c); a['valuation'] = {'state': 'unpriceable', 'reasons': ['IdentityWithoutRate'], 'supported_low': None, 'priced_high': '15000000'}
rejected(a, ['ModelMismatch'], 500000000, extra_blocking=('IdentityContradiction',)); save(c)

# ---------------- A-06: status rules by path
c = new('VC01-openai-usage-settlement', 'P133-A06-usage-settlement-status-absent', role='finding', cls='adversarial_synthetic', reach='producer_unreachable',
        findings=['P1.32-A-06'], controls=['VC01-openai-usage-settlement'], invariants=['INV-F1'],
        notes='Single condition changed from VC01: the request row has no http_status. Every pi_container revision writes an integer status on request rows, so the usage path rejects it.')
edit_row(c, 2, lambda r: r.pop('http_status'))
a = att(c); a['valuation']['complete'] = False
rejected(a, ['StatusType'], 500000000); save(c)
c = new('S02-control-error-status-after-reservation', 'P133-A06-error-status-null', role='finding', cls='adversarial_synthetic', reach='producer_unreachable',
        findings=['P1.32-A-06'], controls=['S02-control-error-status-after-reservation'], invariants=['INV-F1'],
        notes='Single condition changed from S02-control: http_status is null on an error-status settlement, which needs an integer non-200 status.')
edit_row(c, 2, lambda r: r.update(http_status=None))
a = att(c); a['diagnostics'] = [d for d in a['diagnostics'] if d not in ('ErrorStatusSettlement', 'NonBillingUnverified')]
rejected(a, ['StatusType'], 800000000); save(c)

# ---------------- A-09: two different reservations for one rid with a settlement
c = new('VC01-openai-usage-settlement', 'P133-A09-conflicting-reservations-with-settlement', role='finding', cls='adversarial_synthetic', reach='producer_unreachable',
        findings=['P1.32-A-09'], controls=['VC01-openai-usage-settlement', 'R04-reused-reservation-id', 'C3-identical-reservation-copy'], invariants=['INV-F2'],
        notes='A second, different reservation names rid 1 before the settlement. Both stay visible; R is the larger (0.5), never 0.8; the settlement is rejected.')
r0 = json.loads(lines(c)[0]); r0['reserved_usd'] = 0.3; r0['at'] = '2026-11-10T06:30:01Z'; lines(c).insert(1, json.dumps(r0))
a = att(c); a['evidence'] = ['S1/turn-1:1', 'S1/turn-1:2', 'S1/turn-1:3']
rejected(a, ['ReservationConflict'], 500000000, extra_blocking=('ReservationConflict',)); save(c)
print(f'{len(written)} cases written')
