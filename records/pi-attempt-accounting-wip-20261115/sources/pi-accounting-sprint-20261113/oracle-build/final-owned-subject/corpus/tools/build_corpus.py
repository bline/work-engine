"""P1.28 corpus builder: writes corpus/cases/<case_id>.json from the literal case definitions below.

Every expected value is written by hand from the contract (`contract/CONTRACT.md` 1.0-s1) and the release rate files;
derivations are re-checked arithmetically by the independent validator. No reader, reducer or pricing code is used.
Amounts are nano-USD integer strings. Usage: build_corpus.py <corpus dir>"""
import json, os, sys
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from rows import *  # noqa: F401,F403

ALIAS = {S1: 'S1', S2: 'S2', S3: 'S3'}
OA1K = oa_usage()
D_OA = lambda model, usage, n_, tier='1', bound='both': deriv('openai', model, {k: usage[k] for k in ('input_tokens', 'output_tokens', 'cached_tokens', 'cache_write_tokens')}, n_, tier, bound)
D_AN = lambda model, usage, n_, remainder='max', split='reported', bound='both': {**deriv('anthropic', model, usage, n_, '1', bound), 'remainder_rate': remainder, 'split': split}
ZERO = {'low': '0', 'high': {'kind': 'exact', 'nano': '0'}}
CASES = []


def esum(low, kind, high):
    return {'low': str(low), 'high': {'kind': kind, 'nano': str(high)}}


def obj_exp(name='o', status='established', reasons=(), provisional=False, closed=None, outstanding=None, pred=None,
            commitments=(('a1', 'retired', '0'),)):
    return {'objective': name, 'status': status, 'reasons': sorted(reasons), 'provisional': provisional,
            'owned_closed': closed or ZERO, 'owned_outstanding': outstanding or ZERO, 'predecessor_obligations': pred or ZERO,
            'commitments': [{'admission_id': a, 'state': s, 'committed': c} for a, s, c in commitments]}


def attr(key, obj='o', ownership='owned', placement='inside', cause=None, candidates=None, provisional_retained=False):
    a = {'attempt': key, 'objective': obj, 'ownership': ownership, 'placement': placement}
    if cause: a['cause'] = cause
    if candidates: a['candidates'] = candidates
    if provisional_retained: a['provisional_retained'] = True
    return a


def adm(decision, new='0.1', limit='1.0', obj='o'):
    return {'objective': obj, 'new': nano(new), 'limit': nano(limit), 'decision': decision}


def ev(sources_lines):
    return [f'{s}:{n}' for s, n in sources_lines]


def C(cid, *, group, role, cls, reach, findings, origin, invariants, sessions, attempts, evaluations=None,
      objectives=None, attribution=None, objectives_exp=None, admission=None, request_new='0.1', request_obj='o',
      lifecycle=None, non_attempt=(), unreadable=(), inventory=(), controls=(), reference='07:00:00', ambiguity=None,
      notes='', classes_extra=()):
    if evaluations is None:
        evaluations = [{'name': 'default', 'objectives': objectives if objectives is not None else [objective()],
                        'admission_request': {'objective': request_obj, 'new': nano(request_new)},
                        'expected': {'attribution': attribution or [], 'objectives': objectives_exp or [], 'admission': admission}}]
    for s in sessions: s['alias'] = ALIAS[s['session_id']]
    CASES.append({'schema': 'pi-accounting-corpus/1', 'contract': 'pi-accounting-contract/1.0-s1', 'case_id': cid,
                  'group': group, 'role': role, 'class': cls, 'reachability': reach, 'findings': list(findings),
                  'provenance': origin, 'invariants': sorted(invariants), 'controls': list(controls),
                  'inputs': {'reference_time': t(reference), 'rates': RATES, 'sessions': sessions},
                  'evaluations': evaluations,
                  'expected': {'attempts': attempts, 'non_attempt': list(non_attempt), 'unreadable': list(unreadable),
                               'inventory': list(inventory),
                               'lifecycle': lifecycle or {'S1': {'state': 'retired', 'clock': 'trusted'}}},
                  'ambiguity': ambiguity, 'notes': notes, 'historical': {}, 'rationale': None})


def P(script, test, variant=None, extra=None):
    o = {'origin': f'{script}::{test}'}
    if variant: o['variant'] = variant
    if extra: o.update(extra)
    return o


K1 = 'res:S1/turn-1:rid=1'
SRC = 'S1/turn-1'
OK_OBJ = lambda closed=None, outstanding=None, commitments=(('a1', 'retired', '0'),): [obj_exp(closed=closed, outstanding=outstanding, commitments=commitments)]

# =============================================================================================== valid controls
v = valued([ASTRA], 15000000, 15000000, True, [D_OA(ASTRA, OA1K, 15000000)])
C('VC01-openai-usage-settlement', group='VC', role='valid_control', cls='producer_valid', reach='producer_reachable',
  findings=[], origin=P('contract', 'producer pi_container.py:1554-1571 (UsdBudget.settle usage)'),
  invariants=['INV-F1', 'INV-F2', 'INV-A1'],
  sessions=[session(logs={'turn-1': [reserve(1, 1, 0.5, '06:30:00'), oa_request(1, '06:30:02', usd=settle(1, 0.5, 0.015, 'usage'))]})],
  attempts=[attempt(K1, ev([(SRC, 1), (SRC, 2)]), sd('valid', 'usage'), CLOSED_SETTLED, v,
                    exposure(15000000, 'usage_valuation', 'exact', 15000000, 'usage_valuation'), reserved=0.5, charged=0.015)],
  attribution=[attr(K1)], objectives_exp=OK_OBJ(closed=esum(15000000, 'exact', 15000000)), admission=adm('admitted'))

AU = an_usage(i=1000, o=500)
C('VC02-anthropic-usage-settlement', group='VC', role='valid_control', cls='producer_valid', reach='producer_reachable',
  findings=[], origin=P('contract', 'producer pi_container.py:1043-1072 (AnthropicUsdBudget.settle usage)'),
  invariants=['INV-F1', 'INV-A1'],
  sessions=[session(model='anthropic/' + FABLE, logs={'turn-1': [reserve(1, 1, 0.3, '06:30:00'), an_request(1, '06:30:02', AU, usd=settle(1, 0.3, 0.028, 'usage'))]})],
  attempts=[attempt(K1, ev([(SRC, 1), (SRC, 2)]), sd('valid', 'usage'), CLOSED_SETTLED,
                    valued([FABLE], 28000000, 28000000, True, [D_AN(FABLE, AU, 28000000)]),
                    exposure(28000000, 'usage_valuation', 'exact', 28000000, 'usage_valuation'), reserved=0.3, charged=0.028)],
  attribution=[attr(K1)], objectives_exp=OK_OBJ(closed=esum(28000000, 'exact', 28000000)), admission=adm('admitted'))

C('VC03-reservation-settlement-with-usage', group='VC', role='valid_control', cls='producer_valid', reach='producer_reachable',
  findings=[], origin=P('contract', 'producer: terminal null -> settle(None) keeps the reservation (pi_container.py:1571, :1014)'),
  invariants=['INV-F3', 'INV-F11'],
  sessions=[session(logs={'turn-1': [reserve(1, 1, 0.3, '06:30:00'), oa_request(1, '06:30:02', terminal=None, usd=settle(1, 0.3, 0.3, 'reservation'))]})],
  attempts=[attempt(K1, ev([(SRC, 1), (SRC, 2)]), sd('valid', 'reservation'), {'state': 'closed', 'amount_source': 'reservation_confirmed'},
                    valued([ASTRA], 15000000, 15000000, False, [D_OA(ASTRA, OA1K, 15000000)]),
                    exposure(15000000, 'usage_valuation', 'exact', 300000000, 'proxy_reservation'), diagnostics=['PartialUsage'],
                    reserved=0.3, charged=0.3)],
  attribution=[attr(K1)], objectives_exp=OK_OBJ(closed=esum(15000000, 'exact', 300000000)), admission=adm('admitted'))

C('VC04-upstream-error-keeps-reservation', group='A1-01', role='valid_control', cls='producer_valid', reach='producer_reachable',
  findings=['A1-01'], origin=P('repro_p111.py', 'A1-01', 'control'), invariants=['INV-F1', 'INV-F3'],
  sessions=[session(logs={'turn-1': [reserve(1, 1, 0.3, '06:30:00'), upstream_error(1, '06:30:02', usd=settle(1, 0.3, 0.3, 'reservation'))]})],
  attempts=[attempt(K1, ev([(SRC, 1), (SRC, 2)]), sd('valid', 'upstream_error'), {'state': 'closed', 'amount_source': 'reservation_confirmed'},
                    NOT_REPORTED, exposure(0, 'none', 'exact', 300000000, 'proxy_reservation'), reserved=0.3, charged=0.3)],
  attribution=[attr(K1)], objectives_exp=OK_OBJ(closed=esum(0, 'exact', 300000000)), admission=adm('admitted'))

C('VC05-unsettled-reservation', group='S-04', role='valid_control', cls='producer_valid', reach='producer_reachable',
  findings=[], origin=P('contract', 'killed stream: reservation written, handler never settled'), invariants=['INV-F3', 'INV-T3'],
  sessions=[session(logs={'turn-1': [reserve(1, 1, 0.3, '06:30:00')]})],
  attempts=[attempt(K1, ev([(SRC, 1)]), sd('absent'), OUTSTANDING, NOT_REPORTED, exposure(0, 'none', 'exact', 300000000, 'proxy_reservation'), reserved=0.3)],
  attribution=[attr(K1)], objectives_exp=OK_OBJ(outstanding=esum(0, 'exact', 300000000)), admission=adm('admitted'))

R1L1 = 'row:S1/turn-1:line=1'
UNB_UNCERTAIN = lambda keys, blocking=('OwnershipUncertain',): dict(
    attribution=[attr(k, ownership='uncertain', placement='uncertain', cause='NoAdmission', candidates=['o']) for k in keys],
    objectives_exp=[obj_exp(status='not_established', reasons=list(blocking), commitments=())], admission=adm('not_established'))
C('VC06-unbudgeted-completed-request', group='VC', role='valid_control', cls='producer_valid', reach='producer_reachable',
  findings=[], origin=P('contract', 'API session without --max-usd: request row, no usd'), invariants=['INV-F2', 'INV-T2'],
  sessions=[session(logs={'turn-1': [oa_request(1, '06:30:02')]})],
  attempts=[attempt(R1L1, ev([(SRC, 1)]), sd('not_applicable'), NO_OBLIGATION, v, exposure(15000000, 'usage_valuation', 'exact', 15000000, 'usage_valuation'))],
  objectives=[objective(admissions=[])], **UNB_UNCERTAIN([R1L1]),
  notes='Attempt facts are a valid control (exact amount, no attempt-level finding). Admission is not established because an unbudgeted session has no admission record (O-6, owner question Q8).')

C('VC07-read-failure-paired', group='N-04', role='valid_control', cls='producer_valid', reach='producer_reachable',
  findings=[], origin=P('test_usage_ledger_pi_a134.py', 'test_N04_read_failure_pairs_only_with_its_own_later_request_row', 'control'),
  invariants=['INV-F6'],
  sessions=[session(logs={'turn-1': [stream_error(1, '06:30:01'), oa_request(1, '06:30:02')]})],
  attempts=[attempt('row:S1/turn-1:line=2', ev([(SRC, 1), (SRC, 2)]), sd('not_applicable'), NO_OBLIGATION, v,
                    exposure(15000000, 'usage_valuation', 'exact', 15000000, 'usage_valuation'), diagnostics=['ReadFailurePaired'],
                    correlation='producer_shape', rule='RS-1')],
  objectives=[objective(admissions=[])], **UNB_UNCERTAIN(['row:S1/turn-1:line=2']))

ADM = 'adm:S1/turn-1:rid=1'
SUB = dict(model='openai-codex/' + ASTRA, lane='subscription')
C('VC08-subscription-measured-request', group='VC', role='valid_control', cls='producer_valid', reach='producer_reachable',
  findings=[], origin=P('contract', 'subscription cost-proxy admission + measured labelled request (pi_container.py:1419, :1459)'),
  invariants=['INV-F9', 'INV-T5'],
  sessions=[session(**SUB, logs={'turn-1': [admitted(1, 1, '06:30:00'), sub_request(1, '06:30:02', 1)]})],
  attempts=[attempt(ADM, ev([(SRC, 1), (SRC, 2)]), sd('not_applicable'), NO_OBLIGATION, {'state': 'not_applicable'}, CODEX)],
  objectives=[objective(admissions=[])],
  attribution=[attr(ADM, ownership='uncertain', placement='uncertain', cause='NoAdmission', candidates=['o'])],
  objectives_exp=[obj_exp(commitments=())], admission=adm('admitted'),
  notes='Codex-pool attempt with uncertain ownership does not block pi_api admission (O-5).')

NULLC = oa_usage(cached=None, cw=None)
C('N06-null-cache-counts', group='N-06', role='finding', cls='producer_valid', reach='producer_reachable',
  findings=['N-06'], origin=P('repro_p125.py', 'test_N06_absent_cache_counts', 'claimed'), invariants=['INV-A6', 'INV-F11'],
  controls=['VC01-openai-usage-settlement'],
  sessions=[session(logs={'turn-1': [reserve(1, 1, 0.5, '06:30:00'), oa_request(1, '06:30:02', usage=NULLC, usd=settle(1, 0.5, 0.015, 'usage'))]})],
  attempts=[attempt(K1, ev([(SRC, 1), (SRC, 2)]), sd('valid', 'usage'), CLOSED_SETTLED,
                    valued([ASTRA], 6000000, 17500000, True, [D_OA(ASTRA, oa_usage(cached=1000, cw=0), 6000000, bound='low'),
                                                              D_OA(ASTRA, oa_usage(cached=0, cw=1000), 17500000, bound='high'),
                                                              D_OA(ASTRA, OA1K, 15000000, bound='none')]),
                    exposure(6000000, 'usage_valuation', 'exact', 17500000, 'usage_valuation'), reserved=0.5, charged=0.015)],
  attribution=[attr(K1)], objectives_exp=OK_OBJ(closed=esum(6000000, 'exact', 17500000)), admission=adm('admitted'))
C('VC11-multi-turn-session', group='VC', role='valid_control', cls='producer_valid', reach='producer_reachable',
  findings=[], origin=P('contract', 'launch, deliver (no retirement between), retire'), invariants=['INV-T3', 'INV-A1'],
  sessions=[session(lifecycle=[('launch', '06:00:10', 1), ('deliver', '06:35:00', 2), ('retire', '06:50:00', None)],
                    logs={'turn-1': [reserve(1, 1, 0.5, '06:30:00'), oa_request(1, '06:30:02', usd=settle(1, 0.5, 0.015, 'usage'))],
                          'turn-2': [reserve(1, 1, 0.5, '06:40:00'), oa_request(1, '06:40:02', resp='resp_2', usd=settle(1, 0.5, 0.015, 'usage'))]})],
  attempts=[attempt(K1, ev([(SRC, 1), (SRC, 2)]), sd('valid', 'usage'), CLOSED_SETTLED, v, exposure(15000000, 'usage_valuation', 'exact', 15000000, 'usage_valuation'), reserved=0.5, charged=0.015),
            attempt('res:S1/turn-2:rid=1', ev([('S1/turn-2', 1), ('S1/turn-2', 2)]), sd('valid', 'usage'), CLOSED_SETTLED, v, exposure(15000000, 'usage_valuation', 'exact', 15000000, 'usage_valuation'), reserved=0.5, charged=0.015)],
  attribution=[attr(K1), attr('res:S1/turn-2:rid=1')], objectives_exp=OK_OBJ(closed=esum(30000000, 'exact', 30000000)), admission=adm('admitted'))

C('VC12-reused-n-without-errors', group='N-04', role='valid_control', cls='producer_valid', reach='producer_reachable',
  findings=[], origin=P('test_usage_ledger_pi_a134.py', 'test_N04_ambiguous_attempt_number_fails_closed_without_banning_reuse', 'no error'),
  invariants=['INV-F2', 'INV-F6'],
  sessions=[session(logs={'turn-1': [oa_request(2, '06:30:02', resp='resp_a'), oa_request(2, '06:30:04', resp='resp_b')]})],
  attempts=[attempt('row:S1/turn-1:line=1', ev([(SRC, 1)]), sd('not_applicable'), NO_OBLIGATION, v, exposure(15000000, 'usage_valuation', 'exact', 15000000, 'usage_valuation')),
            attempt('row:S1/turn-1:line=2', ev([(SRC, 2)]), sd('not_applicable'), NO_OBLIGATION, v, exposure(15000000, 'usage_valuation', 'exact', 15000000, 'usage_valuation'))],
  objectives=[objective(admissions=[])], **UNB_UNCERTAIN(['row:S1/turn-1:line=1', 'row:S1/turn-1:line=2']),
  notes='Reused n without an error is not prohibited: both attempts priced exactly, no attempt-level finding.')

C('VC13-equal-timestamps', group='N-05', role='valid_control', cls='producer_valid', reach='producer_reachable',
  findings=[], origin=P('test_usage_ledger_pi_a134.py', 'test_N05_controls', 'equal timestamps'), invariants=['INV-T4'],
  sessions=[session(lifecycle=[('launch', '06:00:10', 1), ('retire', '06:00:12', None)],
                    logs={'turn-1': [reserve(1, 1, 0.5, '06:00:10'), oa_request(1, '06:00:12', usd=settle(1, 0.5, 0.015, 'usage'))]})],
  attempts=[attempt(K1, ev([(SRC, 1), (SRC, 2)]), sd('valid', 'usage'), CLOSED_SETTLED, v, exposure(15000000, 'usage_valuation', 'exact', 15000000, 'usage_valuation'), reserved=0.5, charged=0.015)],
  attribution=[attr(K1)], objectives_exp=OK_OBJ(closed=esum(15000000, 'exact', 15000000)), admission=adm('admitted'))

open_s = session(lifecycle=[('launch', '06:00:10', 1)], logs={'turn-1': [reserve(1, 1, 0.5, '06:30:00'), oa_request(1, '06:30:02', usd=settle(1, 0.5, 0.015, 'usage'))]})
C('VC14-open-session-commitment', group='VC', role='valid_control', cls='producer_valid', reach='producer_reachable',
  findings=[], origin=P('contract', 'journal pending+launched, session open: committed = cap - spent high (A-3)'), invariants=['INV-A3', 'INV-A1'],
  sessions=[open_s],
  attempts=[attempt(K1, ev([(SRC, 1), (SRC, 2)]), sd('valid', 'usage'), CLOSED_SETTLED, v, exposure(15000000, 'usage_valuation', 'exact', 15000000, 'usage_valuation'), reserved=0.5, charged=0.015)],
  lifecycle={'S1': {'state': 'open', 'clock': 'trusted'}},
  evaluations=[{'name': 'limit-1.0', 'objectives': [objective(admissions=[journal_admission('a1', S1, 0.5)])],
                'admission_request': {'objective': 'o', 'new': nano('0.1')},
                'expected': {'attribution': [attr(K1)],
                             'objectives': [obj_exp(closed=esum(15000000, 'exact', 15000000), commitments=(('a1', 'launched_open', '485000000'),))],
                             'admission': adm('admitted')}},
               {'name': 'limit-0.55', 'objectives': [objective(limit='0.55', admissions=[journal_admission('a1', S1, 0.5)])],
                'admission_request': {'objective': 'o', 'new': nano('0.1')},
                'expected': {'attribution': [attr(K1)],
                             'objectives': [obj_exp(closed=esum(15000000, 'exact', 15000000), commitments=(('a1', 'launched_open', '485000000'),))],
                             'admission': adm('refused', limit='0.55')}}])

pred_settled = session(lifecycle=[('launch', '06:00:10', 1), ('retire', '06:38:00', None)],
                       logs={'turn-1': [reserve(1, 1, 0.5, '06:30:00'), oa_request(1, '06:30:02', usd=settle(1, 0.5, 0.015, 'usage'))]})
C('VC15-successor-not-charged-settled-predecessor-spend', group='D1', role='valid_control', cls='producer_valid', reach='producer_reachable',
  findings=[], origin=P('contract', 'D1 predecessor obligations (12.2): settled spend never transferred'), invariants=['INV-A4'],
  sessions=[pred_settled],
  attempts=[attempt(K1, ev([(SRC, 1), (SRC, 2)]), sd('valid', 'usage'), CLOSED_SETTLED, v, exposure(15000000, 'usage_valuation', 'exact', 15000000, 'usage_valuation'), reserved=0.5, charged=0.015)],
  evaluations=[{'name': 'successor', 'objectives': [objective('p', closed='06:40:00'), objective('s', start='06:45:00', tree=(), admissions=[], predecessor='p')],
                'admission_request': {'objective': 's', 'new': nano('0.1')},
                'expected': {'attribution': [attr(K1, obj='p')],
                             'objectives': [obj_exp('p', closed=esum(15000000, 'exact', 15000000)),
                                            obj_exp('s', commitments=())],
                             'admission': adm('admitted', obj='s')}}])

pred_open = session(lifecycle=[('launch', '06:00:10', 1)], logs={'turn-1': [reserve(1, 1, 0.5, '06:30:00')]})
C('D1-successor-constrained-by-predecessor-obligations', group='D1', role='finding', cls='producer_valid', reach='producer_reachable',
  findings=['D1'], origin=P('contract', 'D1 predecessor obligations: outstanding reservation + open-session commitment'),
  invariants=['INV-A4', 'INV-T3'], controls=['VC15-successor-not-charged-settled-predecessor-spend'],
  sessions=[pred_open], lifecycle={'S1': {'state': 'open', 'clock': 'trusted'}},
  attempts=[attempt(K1, ev([(SRC, 1)]), sd('absent'), OUTSTANDING, NOT_REPORTED, exposure(0, 'none', 'exact', 500000000, 'proxy_reservation'), reserved=0.5)],
  evaluations=[{'name': 'successor', 'objectives': [objective('p', closed='06:40:00'), objective('s', start='06:45:00', tree=(), admissions=[], predecessor='p')],
                'admission_request': {'objective': 's', 'new': nano('0.1')},
                'expected': {'attribution': [attr(K1, obj='p', provisional_retained=True)],
                             'objectives': [obj_exp('p', provisional=True, outstanding=esum(0, 'exact', 500000000), commitments=(('a1', 'launched_open', '500000000'),)),
                                            obj_exp('s', pred=esum(0, 'exact', 1000000000), commitments=())],
                             'admission': adm('refused', obj='s')}}],
  notes='Successor line = predecessor outstanding 0.5 + predecessor open-session commitment (cap 1.0 - spent high 0.5) = 1.0; 1.0 + 0.1 > 1.0.')

NT = oa_usage()
C('U08-missing-observed-tier', group='U-08', role='finding', cls='producer_valid', reach='producer_reachable',
  findings=['U-08'], origin=P('repro_p121.py', 'test_U08_missing_tier_usage_settlement_rejected', 'claimed'),
  invariants=['INV-A6'], controls=['VC01-openai-usage-settlement'],
  sessions=[session(logs={'turn-1': [reserve(1, 1, 0.5, '06:30:00'), oa_request(1, '06:30:02', tier=None, usd=settle(1, 0.5, 0.03, 'usage'))]})],
  attempts=[attempt(K1, ev([(SRC, 1), (SRC, 2)]), sd('valid', 'usage'), CLOSED_SETTLED,
                    valued([ASTRA], 7500000, 30000000, True, [D_OA(ASTRA, NT, 7500000, tier='0.5', bound='low'), D_OA(ASTRA, NT, 30000000, tier='2', bound='high')]),
                    exposure(7500000, 'usage_valuation', 'exact', 30000000, 'usage_valuation'), reserved=0.5, charged=0.03)],
  attribution=[attr(K1)], objectives_exp=OK_OBJ(closed=esum(7500000, 'exact', 30000000)), admission=adm('admitted'),
  notes='Producer charges a missing tier at the highest multiplier (2.0); the valuation range spans the cheapest (flex 0.5) to the dearest.')

# =============================================================================================== identity (N-01 and relatives)
AN10K = an_usage(i=10000)
def refusal_logs(pinned, served, reserved, with_refusal=True, resp='msg_x'):
    rows_ = [reserve(1, 1, reserved, '06:30:00')]
    if with_refusal and served != pinned: rows_.append(identity_refusal(1, '06:30:01', pinned, served, resp=resp))
    rows_.append(an_request(1, '06:30:01', AN10K, model=pinned, response_model=served, terminal=None, resp=resp,
                            check='response_model_mismatch' if served != pinned else 'verified', usd=settle(1, reserved, reserved, 'reservation')))
    return rows_

C('N01-identity-refusal-dearer-served', group='N-01', role='finding', cls='producer_valid', reach='producer_reachable',
  findings=['N-01'], origin=P('repro_p125.py', 'test_N01_identity_refusal_reservation', 'claimed: producer refusal pair, response fable'),
  invariants=['INV-F4', 'INV-F5', 'INV-F3', 'INV-A2'], controls=['N01-control-served-pinned'],
  sessions=[session(model='anthropic/' + SONNET, logs={'turn-1': refusal_logs(SONNET, FABLE, 0.05)})],
  attempts=[attempt(K1, ev([(SRC, 1), (SRC, 2), (SRC, 3)]), sd('valid', 'reservation'), {'state': 'closed', 'amount_source': 'reservation_confirmed'},
                    valued([SONNET, FABLE], 20000000, 80000000, False, [D_AN(SONNET, AN10K, 20000000, bound='low'), D_AN(FABLE, AN10K, 80000000, bound='high')]),
                    exposure(20000000, 'usage_valuation', 'at_least', 80000000, 'usage_valuation'),
                    blocking=['IdentityContradiction', 'UnresolvedExposure'], diagnostics=['PartialUsage'], correlation='producer_shape', rule='RS-3',
                    reserved=0.05, charged=0.05)],
  objectives=[objective(limit='0.10')], attribution=[attr(K1)],
  objectives_exp=[obj_exp(status='not_established', reasons=['IdentityContradiction', 'UnresolvedExposure'], closed=esum(20000000, 'at_least', 80000000))],
  admission=adm('not_established', new='0.04', limit='0.10'), request_new='0.04')
C('N01-control-served-pinned', group='N-01', role='control', cls='producer_valid', reach='producer_reachable',
  findings=['N-01'], origin=P('repro_p125.py', 'test_N01_identity_refusal_reservation', 'control: response model = pinned sonnet'),
  invariants=['INV-F4', 'INV-A1'],
  sessions=[session(model='anthropic/' + SONNET, logs={'turn-1': refusal_logs(SONNET, SONNET, 0.05)})],
  attempts=[attempt(K1, ev([(SRC, 1), (SRC, 2)]), sd('valid', 'reservation'), {'state': 'closed', 'amount_source': 'reservation_confirmed'},
                    valued([SONNET], 20000000, 20000000, False, [D_AN(SONNET, AN10K, 20000000)]),
                    exposure(20000000, 'usage_valuation', 'exact', 50000000, 'proxy_reservation'), diagnostics=['PartialUsage'], reserved=0.05, charged=0.05)],
  objectives=[objective(limit='0.10')], attribution=[attr(K1)],
  objectives_exp=[obj_exp(closed=esum(20000000, 'exact', 50000000))], admission=adm('admitted', new='0.04', limit='0.10'), request_new='0.04')
C('N01-variant-no-refusal-row', group='N-01', role='finding', cls='malformed', reach='producer_unreachable',
  findings=['N-01'], origin=P('repro_p125.py', 'test_N01_identity_refusal_reservation', 'claimed: request row only'),
  invariants=['INV-F4', 'INV-F5'], controls=['N01-control-served-pinned'],
  sessions=[session(model='anthropic/' + SONNET, logs={'turn-1': refusal_logs(SONNET, FABLE, 0.05, with_refusal=False)})],
  attempts=[attempt(K1, ev([(SRC, 1), (SRC, 2)]), sd('valid', 'reservation'), {'state': 'closed', 'amount_source': 'reservation_confirmed'},
                    valued([SONNET, FABLE], 20000000, 80000000, False, [D_AN(SONNET, AN10K, 20000000, bound='low'), D_AN(FABLE, AN10K, 80000000, bound='high')]),
                    exposure(20000000, 'usage_valuation', 'at_least', 80000000, 'usage_valuation'),
                    blocking=['IdentityContradiction', 'UnresolvedExposure'], diagnostics=['PartialUsage'], reserved=0.05, charged=0.05)],
  objectives=[objective(limit='0.10')], attribution=[attr(K1)],
  objectives_exp=[obj_exp(status='not_established', reasons=['IdentityContradiction', 'UnresolvedExposure'], closed=esum(20000000, 'at_least', 80000000))],
  admission=adm('not_established', new='0.04', limit='0.10'), request_new='0.04')
C('N01-variant-reservation-above-both', group='N-01', role='finding', cls='producer_valid', reach='producer_reachable',
  findings=['N-01'], origin=P('repro_p125.py', 'test_N01_identity_refusal_reservation', 'control: reservation 0.10'),
  invariants=['INV-F5'], controls=['N01-control-served-pinned'],
  sessions=[session(model='anthropic/' + SONNET, logs={'turn-1': refusal_logs(SONNET, FABLE, 0.10)})],
  attempts=[attempt(K1, ev([(SRC, 1), (SRC, 2), (SRC, 3)]), sd('valid', 'reservation'), {'state': 'closed', 'amount_source': 'reservation_confirmed'},
                    valued([SONNET, FABLE], 20000000, 80000000, False, [D_AN(SONNET, AN10K, 20000000, bound='low'), D_AN(FABLE, AN10K, 80000000, bound='high')]),
                    exposure(20000000, 'usage_valuation', 'at_least', 100000000, 'proxy_reservation'),
                    blocking=['IdentityContradiction', 'UnresolvedExposure'], diagnostics=['PartialUsage'], correlation='producer_shape', rule='RS-3', reserved=0.10, charged=0.10)],
  objectives=[objective(limit='0.10')], attribution=[attr(K1)],
  objectives_exp=[obj_exp(status='not_established', reasons=['IdentityContradiction', 'UnresolvedExposure'], closed=esum(20000000, 'at_least', 100000000))],
  admission=adm('not_established', new='0.04', limit='0.10'), request_new='0.04',
  notes='A known partial amount (0.10 reservation, above both usage prices) still does not cap the unresolved served-model cost (X-2).')
C('N01-orphan-identity-refusal', group='N-01', role='finding', cls='malformed', reach='producer_reachable',
  findings=['N-01'], origin=P('test_usage_ledger_pi_a134.py', 'test_N01_identity_refusal_without_its_request_row'),
  invariants=['INV-F5', 'INV-F6'], controls=['VC05-unsettled-reservation'],
  sessions=[session(model='anthropic/' + SONNET, logs={'turn-1': [reserve(1, 1, 0.05, '06:30:00'), identity_refusal(1, '06:30:01', SONNET, FABLE)]})],
  attempts=[attempt(K1, ev([(SRC, 1), (SRC, 2)]), sd('absent'), OUTSTANDING, NOT_REPORTED,
                    exposure(0, 'none', 'at_least', 50000000, 'proxy_reservation'), blocking=['IdentityContradiction', 'UnresolvedExposure'],
                    correlation='producer_shape', rule='RS-3', reserved=0.05)],
  attribution=[attr(K1)],
  objectives_exp=[obj_exp(status='not_established', reasons=['IdentityContradiction', 'UnresolvedExposure'], outstanding=esum(0, 'at_least', 50000000))],
  admission=adm('not_established'), notes='Handler died between the refusal and its request row (crash window); producer-reachable only under a crash.')
U12K = oa_usage(i=0, o=12000)
C('V01-cheaper-response-model-undercharge', group='V-01', role='finding', cls='adversarial_synthetic', reach='producer_unreachable',
  findings=['V-01'], origin=P('repro_p123.py', 'test_V01', 'claimed: charge 0.06 for usage priced 0.6'),
  invariants=['INV-F1', 'INV-F3'], controls=['V01-control-producer-price'],
  sessions=[session(logs={'turn-1': [reserve(1, 1, 0.8, '06:30:00'), oa_request(1, '06:30:02', usage=U12K, response_model=TERRA, usd=settle(1, 0.8, 0.06, 'usage'))]})],
  attempts=[attempt(K1, ev([(SRC, 1), (SRC, 2)]), sd('rejected', reasons=['Undercharge']), OUTSTANDING,
                    valued([ASTRA, TERRA], 60000000, 600000000, True, [D_OA(TERRA, U12K, 60000000, bound='low'), D_OA(ASTRA, U12K, 600000000, bound='high')]),
                    exposure(0, 'none', 'exact', 800000000, 'proxy_reservation'), blocking=['IdentityContradiction', 'SettlementRejected'], reserved=0.8, charged=0.06)],
  attribution=[attr(K1)],
  objectives_exp=[obj_exp(status='not_established', reasons=['IdentityContradiction', 'SettlementRejected'], outstanding=esum(0, 'exact', 800000000))],
  admission=adm('not_established'))
C('V01-control-producer-price', group='V-01', role='control', cls='producer_valid', reach='producer_reachable',
  findings=['V-01'], origin=P('repro_p123.py', 'test_V01', 'control: producer price, same model'), invariants=['INV-F1'],
  sessions=[session(logs={'turn-1': [reserve(1, 1, 0.8, '06:30:00'), oa_request(1, '06:30:02', usage=U12K, usd=settle(1, 0.8, 0.6, 'usage'))]})],
  attempts=[attempt(K1, ev([(SRC, 1), (SRC, 2)]), sd('valid', 'usage'), CLOSED_SETTLED,
                    valued([ASTRA], 600000000, 600000000, True, [D_OA(ASTRA, U12K, 600000000)]),
                    exposure(600000000, 'usage_valuation', 'exact', 600000000, 'usage_valuation'), reserved=0.8, charged=0.6)],
  attribution=[attr(K1)], objectives_exp=OK_OBJ(closed=esum(600000000, 'exact', 600000000)), admission=adm('admitted'))
C('V01-usage-settlement-other-response-model', group='V-01', role='finding', cls='producer_valid', reach='producer_reachable',
  findings=['V-01', 'D6'], origin=P('test_usage_ledger_pi_a134.py', 'test_N01_usage_settlement_with_another_response_model'),
  invariants=['INV-F4', 'INV-A2'], controls=['VC02-anthropic-usage-settlement'],
  sessions=[session(model='anthropic/' + FABLE, logs={'turn-1': [reserve(1, 1, 0.3, '06:30:00'), an_request(1, '06:30:01', AN10K, response_model=SONNET, usd=settle(1, 0.3, 0.08, 'usage'))]})],
  attempts=[attempt(K1, ev([(SRC, 1), (SRC, 2)]), sd('valid', 'usage'), CLOSED_SETTLED,
                    valued([FABLE, SONNET], 20000000, 80000000, True, [D_AN(SONNET, AN10K, 20000000, bound='low'), D_AN(FABLE, AN10K, 80000000, bound='high')]),
                    exposure(20000000, 'usage_valuation', 'exact', 80000000, 'usage_valuation'), blocking=['IdentityContradiction'], reserved=0.3, charged=0.08)],
  attribution=[attr(K1)],
  objectives_exp=[obj_exp(status='not_established', reasons=['IdentityContradiction'], closed=esum(20000000, 'exact', 80000000))],
  admission=adm('not_established'), notes='Complete usage priced over both identities gives a finite range; the contradiction still blocks (D6).')
C('P2-usage-settlement-unpriced-identity-keeps-charge', group='P1.26-P2', role='finding', cls='adversarial_synthetic', reach='producer_reachable',
  findings=['P1.26-P2'], origin=P('test_usage_ledger_pi_a134.py', 'test_N01_unpriced_identity_on_a_usage_settlement_keeps_the_charge'),
  invariants=['INV-F5', 'INV-F3'], controls=['V01-control-producer-price'],
  sessions=[session(logs={'turn-1': [reserve(1, 1, 0.8, '06:30:00'), oa_request(1, '06:30:02', usage=U12K, response_model='gpt-9-unpriced', usd=settle(1, 0.8, 0.6, 'usage'))]})],
  attempts=[attempt(K1, ev([(SRC, 1), (SRC, 2)]), sd('valid', 'usage'), CLOSED_SETTLED,
                    unpriceable(['IdentityWithoutRate'], None, 600000000, [D_OA(ASTRA, U12K, 600000000, bound='high')]),
                    exposure(0, 'none', 'at_least', 600000000, 'usage_valuation'), blocking=['IdentityContradiction', 'UnresolvedExposure'], reserved=0.8, charged=0.6)],
  attribution=[attr(K1)],
  objectives_exp=[obj_exp(status='not_established', reasons=['IdentityContradiction', 'UnresolvedExposure'], closed=esum(0, 'at_least', 600000000))],
  admission=adm('not_established'),
  notes='The served model has no rate: the validated 0.6 charge stays as a known partial amount (AtLeast), never zero; no minimum is supported (V-6).')
C('R06-launch-row-missing', group='R-06', role='finding', cls='attribution_uncertain', reach='producer_reachable',
  findings=['R-06'], origin=P('P1.13 report', 'R-06 (inherited): a lost launch row detaches a child'),
  invariants=['INV-T2', 'INV-F1'], controls=['VC01-openai-usage-settlement'],
  sessions=[session(launch=False, lifecycle=[('retire', '06:50:00', None)], logs={'turn-1': [reserve(1, 1, 0.5, '06:30:00'), oa_request(1, '06:30:02', usd=settle(1, 0.5, 0.015, 'usage'))]})],
  attempts=[attempt(K1, ev([(SRC, 1), (SRC, 2)]), sd('rejected', reasons=['ModelMismatch']), OUTSTANDING, v,
                    exposure(0, 'none', 'exact', 500000000, 'proxy_reservation'), blocking=['InventoryIncomplete', 'SettlementRejected'], reserved=0.5, charged=0.015)],
  inventory=[{'session': 'S1', 'kind': 'missing_launch', 'blocking': ['InventoryIncomplete']}],
  lifecycle={'S1': {'state': 'contradicted', 'clock': 'untrusted', 'reason': 'NoLaunchRow'}},
  attribution=[attr(K1, placement='unplaceable', cause='ClockUntrusted')],
  objectives_exp=[obj_exp(status='not_established', reasons=['InventoryIncomplete', 'PlacementUnplaceable', 'SettlementRejected'], outstanding=esum(0, 'exact', 500000000))],
  admission=adm('not_established'),
  notes='The turn record (normally the launch row) is missing but the journal names the session: ownership comes from the admission (D1), the pinned model is unknown so the usage settlement cannot be validated, and the lifecycle cannot bound anything.',
  ambiguity={'fields': ['lifecycle'], 'note': 'Stage E needs a turn record to find the log; this case assumes the log was found by the host-directory inventory. Whether such a log is evidence at all (vs. only an inventory finding) is owner question Q9.'})

# =============================================================================================== valuation details
HA_BAD = an_usage(i=1000, cw=1000, w5=1000, w1=500)
HA_OK = an_usage(i=1000, cw=1000, w5=1000, w1=0)
def ha_logs(u):
    return [reserve(1, 1, 0.3, '06:30:00'), an_request(1, '06:30:02', u, terminal=None, usd=settle(1, 0.3, 0.3, 'reservation'))]
C('Ha-inconsistent-cache-split-keeps-supported-low', group='H-a', role='finding', cls='producer_valid', reach='producer_reachable',
  findings=['H-a'], origin=P('probe_p127.py', 'test_Ha', 'inconsistent split'), invariants=['INV-F4', 'INV-F11'],
  controls=['Ha-control-consistent-split'],
  sessions=[session(model='anthropic/' + FABLE, logs={'turn-1': ha_logs(HA_BAD)})],
  attempts=[attempt(K1, ev([(SRC, 1), (SRC, 2)]), sd('valid', 'reservation'), {'state': 'closed', 'amount_source': 'reservation_confirmed'},
                    valued([FABLE], 18000000, 24000000, False, [D_AN(FABLE, HA_BAD, 18000000, remainder='min', split='ignored', bound='low'),
                                                              D_AN(FABLE, HA_BAD, 24000000, remainder='max', split='ignored', bound='high')]),
                    exposure(18000000, 'usage_valuation', 'exact', 300000000, 'proxy_reservation'), diagnostics=['InconsistentCacheSplit', 'PartialUsage'],
                    reserved=0.3, charged=0.3)],
  attribution=[attr(K1)], objectives_exp=OK_OBJ(closed=esum(18000000, 'exact', 300000000)), admission=adm('admitted'))
C('Ha-control-consistent-split', group='H-a', role='control', cls='producer_valid', reach='producer_reachable',
  findings=['H-a'], origin=P('probe_p127.py', 'test_Ha', 'consistent split'), invariants=['INV-F11'],
  sessions=[session(model='anthropic/' + FABLE, logs={'turn-1': ha_logs(HA_OK)})],
  attempts=[attempt(K1, ev([(SRC, 1), (SRC, 2)]), sd('valid', 'reservation'), {'state': 'closed', 'amount_source': 'reservation_confirmed'},
                    valued([FABLE], 18000000, 18000000, False, [D_AN(FABLE, HA_OK, 18000000)]),
                    exposure(18000000, 'usage_valuation', 'exact', 300000000, 'proxy_reservation'), diagnostics=['PartialUsage'], reserved=0.3, charged=0.3)],
  attribution=[attr(K1)], objectives_exp=OK_OBJ(closed=esum(18000000, 'exact', 300000000)), admission=adm('admitted'))
UNSPLIT = an_usage(cw=1000)
C('V02-unsplit-writes-undercharged', group='V-02', role='finding', cls='adversarial_synthetic', reach='producer_unreachable',
  findings=['V-02'], origin=P('repro_p123.py', 'test_V02', 'claimed: charge 0.01 for unsplit writes'), invariants=['INV-F1'],
  controls=['V02-control-producer-price'],
  sessions=[session(model='anthropic/' + FABLE, logs={'turn-1': [reserve(1, 1, 0.3, '06:30:00'), an_request(1, '06:30:02', UNSPLIT, usd=settle(1, 0.3, 0.01, 'usage'))]})],
  attempts=[attempt(K1, ev([(SRC, 1), (SRC, 2)]), sd('rejected', reasons=['Undercharge']), OUTSTANDING,
                    valued([FABLE], 10000000, 16000000, True, [D_AN(FABLE, UNSPLIT, 10000000, remainder='min', bound='low'), D_AN(FABLE, UNSPLIT, 16000000, remainder='max', bound='high')]),
                    exposure(0, 'none', 'exact', 300000000, 'proxy_reservation'), blocking=['SettlementRejected'], reserved=0.3, charged=0.01)],
  attribution=[attr(K1)], objectives_exp=[obj_exp(status='not_established', reasons=['SettlementRejected'], outstanding=esum(0, 'exact', 300000000))],
  admission=adm('not_established'))
C('V02-control-producer-price', group='V-02', role='control', cls='producer_valid', reach='producer_reachable',
  findings=['V-02'], origin=P('repro_p123.py', 'test_V02', 'control: charge 0.016'), invariants=['INV-F1'],
  sessions=[session(model='anthropic/' + FABLE, logs={'turn-1': [reserve(1, 1, 0.3, '06:30:00'), an_request(1, '06:30:02', UNSPLIT, usd=settle(1, 0.3, 0.016, 'usage'))]})],
  attempts=[attempt(K1, ev([(SRC, 1), (SRC, 2)]), sd('valid', 'usage'), CLOSED_SETTLED,
                    valued([FABLE], 10000000, 16000000, True, [D_AN(FABLE, UNSPLIT, 10000000, remainder='min', bound='low'), D_AN(FABLE, UNSPLIT, 16000000, remainder='max', bound='high')]),
                    exposure(10000000, 'usage_valuation', 'exact', 16000000, 'usage_valuation'), reserved=0.3, charged=0.016)],
  attribution=[attr(K1)], objectives_exp=OK_OBJ(closed=esum(10000000, 'exact', 16000000)), admission=adm('admitted'))
C('N02-malformed-anthropic-tier', group='N-02', role='finding', cls='malformed', reach='producer_unreachable',
  findings=['N-02'], origin=P('repro_p125.py', 'test_N02_malformed_anthropic_tier', 'observed_tier=7'), invariants=['INV-F1', 'INV-A5'],
  controls=['VC02-anthropic-usage-settlement'],
  sessions=[session(model='anthropic/' + FABLE, logs={'turn-1': [reserve(1, 1, 0.3, '06:30:00'), an_request(1, '06:30:02', AU, tier=7, usd=settle(1, 0.3, 0.028, 'usage'))]})],
  attempts=[attempt(K1, ev([(SRC, 1), (SRC, 2)]), sd('rejected', reasons=['UsageUnpriceable']), OUTSTANDING, unpriceable(['MalformedTier']),
                    exposure(0, 'none', 'exact', 300000000, 'proxy_reservation'), blocking=['SettlementRejected'], reserved=0.3, charged=0.028)],
  attribution=[attr(K1)], objectives_exp=[obj_exp(status='not_established', reasons=['SettlementRejected'], outstanding=esum(0, 'exact', 300000000))],
  admission=adm('not_established'))
C('N08-model-without-producer-rate', group='N-08', role='finding', cls='producer_valid', reach='producer_reachable',
  findings=['N-08'], origin=P('repro_p125.py', 'test_N08_haiku_long_context'), invariants=['INV-F5'],
  controls=['N08-control-priced-model'],
  sessions=[session(model='anthropic/' + HAIKU, logs={'turn-1': [an_request(1, '06:30:02', AN10K, model=HAIKU, response_model=HAIKU)]})],
  attempts=[attempt(R1L1, ev([(SRC, 1)]), sd('not_applicable'), NO_OBLIGATION, unpriceable(['IdentityWithoutRate']),
                    exposure(0, 'none', 'at_least', 0, 'none'), blocking=['UnresolvedExposure'])],
  objectives=[objective(admissions=[])], **UNB_UNCERTAIN([R1L1], blocking=('OwnershipUncertain', 'UnresolvedExposure')))
C('N08-control-priced-model', group='N-08', role='control', cls='producer_valid', reach='producer_reachable',
  findings=['N-08'], origin=P('contract', 'unbudgeted Anthropic request at a priced model'), invariants=['INV-F5'],
  sessions=[session(model='anthropic/' + SONNET, logs={'turn-1': [an_request(1, '06:30:02', AN10K, model=SONNET, response_model=SONNET)]})],
  attempts=[attempt(R1L1, ev([(SRC, 1)]), sd('not_applicable'), NO_OBLIGATION, valued([SONNET], 20000000, 20000000, True, [D_AN(SONNET, AN10K, 20000000)]),
                    exposure(20000000, 'usage_valuation', 'exact', 20000000, 'usage_valuation'))],
  objectives=[objective(admissions=[])], **UNB_UNCERTAIN([R1L1]))
NOCW = dict(OA1K); NOCW['cache_write_tokens'] = None
C('A1-06-session-file-is-not-evidence', group='A1-06', role='finding', cls='historical_legacy', reach='legacy',
  findings=['A1-06'], origin=P('repro_p111.py', 'A1-06', 'cacheWrite 0 in the writable Pi session file'), invariants=['INV-F6', 'INV-F3'],
  controls=['A1-06-control-no-session-file'],
  sessions=[session(logs={'turn-1': [oa_request(1, '06:30:02', usage=NOCW)]}, ignored={'pi-session-file': {'responseId': 'resp_1', 'cacheWrite': 0}})],
  attempts=[attempt(R1L1, ev([(SRC, 1)]), sd('not_applicable'), NO_OBLIGATION,
                    valued([ASTRA], 15000000, 17500000, True, [D_OA(ASTRA, OA1K, 15000000, bound='low'), D_OA(ASTRA, oa_usage(cw=1000), 17500000, bound='high')]),
                    exposure(15000000, 'usage_valuation', 'exact', 17500000, 'usage_valuation'))],
  objectives=[objective(admissions=[])], **UNB_UNCERTAIN([R1L1]))
C('A1-06-control-no-session-file', group='A1-06', role='control', cls='historical_legacy', reach='legacy',
  findings=['A1-06'], origin=P('repro_p111.py', 'A1-06', 'control without the session file'), invariants=['INV-F3'],
  sessions=[session(logs={'turn-1': [oa_request(1, '06:30:02', usage=NOCW)]})],
  attempts=[attempt(R1L1, ev([(SRC, 1)]), sd('not_applicable'), NO_OBLIGATION,
                    valued([ASTRA], 15000000, 17500000, True, [D_OA(ASTRA, OA1K, 15000000, bound='low'), D_OA(ASTRA, oa_usage(cw=1000), 17500000, bound='high')]),
                    exposure(15000000, 'usage_valuation', 'exact', 17500000, 'usage_valuation'))],
  objectives=[objective(admissions=[])], **UNB_UNCERTAIN([R1L1]))
C('A06-reservation-usage-null-status-no-id', group='A-06', role='finding', cls='producer_valid', reach='producer_reachable',
  findings=['A-06', 'A1-05', 'V-05'], origin=P('repro_unit_a.py', 'A-06', extra={'also': ['repro_p111.py::A1-05', 'repro_p123.py::V05']}),
  invariants=['INV-F11', 'INV-F3'], controls=['VC03-reservation-settlement-with-usage'],
  sessions=[session(logs={'turn-1': [reserve(1, 1, 0.25, '06:30:00'), oa_request(1, '06:30:02', status=None, resp=None, terminal=None, usd=settle(1, 0.25, 0.25, 'reservation'))]})],
  attempts=[attempt(K1, ev([(SRC, 1), (SRC, 2)]), sd('valid', 'reservation'), {'state': 'closed', 'amount_source': 'reservation_confirmed'},
                    valued([ASTRA], 15000000, 15000000, False, [D_OA(ASTRA, OA1K, 15000000)]),
                    exposure(15000000, 'usage_valuation', 'exact', 250000000, 'proxy_reservation'), diagnostics=['IdentityAbsent', 'PartialUsage'], reserved=0.25, charged=0.25)],
  attribution=[attr(K1)], objectives_exp=OK_OBJ(closed=esum(15000000, 'exact', 250000000)), admission=adm('admitted'))
C('U05-usage-settlement-without-response-id', group='U-05', role='finding', cls='producer_valid', reach='producer_reachable',
  findings=['U-05'], origin=P('repro_p121.py', 'test_U05', 'consistent charge'), invariants=['INV-F11'],
  controls=['VC01-openai-usage-settlement'],
  sessions=[session(logs={'turn-1': [reserve(1, 1, 0.5, '06:30:00'), oa_request(1, '06:30:02', resp=None, usd=settle(1, 0.5, 0.015, 'usage'))]})],
  attempts=[attempt(K1, ev([(SRC, 1), (SRC, 2)]), sd('valid', 'usage'), CLOSED_SETTLED, v,
                    exposure(15000000, 'usage_valuation', 'exact', 15000000, 'usage_valuation'), diagnostics=['IdentityAbsent'], reserved=0.5, charged=0.015)],
  attribution=[attr(K1)], objectives_exp=OK_OBJ(closed=esum(15000000, 'exact', 15000000)), admission=adm('admitted'))
C('U05-inconsistent-charge-without-response-id', group='U-05', role='finding', cls='adversarial_synthetic', reach='producer_unreachable',
  findings=['U-05'], origin=P('repro_p121.py', 'test_U05', 'inconsistent charge'), invariants=['INV-F1', 'INV-F3'],
  controls=['U05-usage-settlement-without-response-id'],
  sessions=[session(logs={'turn-1': [reserve(1, 1, 0.5, '06:30:00'), oa_request(1, '06:30:02', resp=None, usd=settle(1, 0.5, 0.01, 'usage'))]})],
  attempts=[attempt(K1, ev([(SRC, 1), (SRC, 2)]), sd('rejected', reasons=['Undercharge']), OUTSTANDING, v,
                    exposure(0, 'none', 'exact', 500000000, 'proxy_reservation'), blocking=['SettlementRejected'], diagnostics=['IdentityAbsent'], reserved=0.5, charged=0.01)],
  attribution=[attr(K1)], objectives_exp=[obj_exp(status='not_established', reasons=['SettlementRejected'], outstanding=esum(0, 'exact', 500000000))],
  admission=adm('not_established'))

# =============================================================================================== settlement validity
def rejected_case(cid, findings, origin, logs, reasons, reserved, charged_claim, high, high_basis, extra_blocking=(), diag=(), ev_lines=(1, 2), group=None, controls=('VC04-upstream-error-keeps-reservation',), reach='producer_unreachable', cls='malformed', inv=('INV-F1', 'INV-F3'), notes=''):
    C(cid, group=group or findings[0], role='finding', cls=cls, reach=reach, findings=findings, origin=origin, invariants=list(inv), controls=list(controls),
      sessions=[session(logs={'turn-1': logs})],
      attempts=[attempt(K1, ev([(SRC, n_) for n_ in ev_lines]), sd('rejected', reasons=reasons), OUTSTANDING, NOT_REPORTED,
                        exposure(0, 'none', 'exact', high, high_basis), blocking=['SettlementRejected', *extra_blocking], diagnostics=list(diag),
                        reserved=reserved, charged=charged_claim)],
      attribution=[attr(K1)], objectives_exp=[obj_exp(status='not_established', reasons=['SettlementRejected', *extra_blocking], outstanding=esum(0, 'exact', high))],
      admission=adm('not_established'), notes=notes)

rejected_case('S01-upstream-error-charged-zero', ['S-01'], P('repro_p115.py', 'test_S01', 'upstream_error charged 0'),
              [reserve(1, 1, 0.8, '06:30:00'), upstream_error(1, '06:30:02', usd=settle(1, 0.8, 0.0, 'reservation'))], ['KeepAll'], 0.8, 0.0, 800000000, 'proxy_reservation')
rejected_case('S01-reservation-basis-charged-less', ['S-01'], P('repro_p115.py', 'test_S01', 'reservation basis charged 0.25'),
              [reserve(1, 1, 0.8, '06:30:00'), oa_request(1, '06:30:02', terminal=None, usage=oa_usage(i=None, o=None, cached=None, cw=None), resp=None, usd=settle(1, 0.8, 0.25, 'reservation'))],
              ['KeepAll'], 0.8, 0.25, 800000000, 'proxy_reservation')
s01c = settle(1, 0.8, 0.0, 'reservation'); del s01c['reserved_usd']
rejected_case('S01-reserved-usd-omitted', ['S-01'], P('repro_p115.py', 'test_S01', 'charged 0 with no reserved_usd'),
              [reserve(1, 1, 0.8, '06:30:00'), upstream_error(1, '06:30:02', usd=s01c)], ['AmountType', 'KeepAll'], None, 0.0, 800000000, 'proxy_reservation')
rejected_case('S02-settlement-before-reservation', ['S-02'], P('repro_p115.py', 'test_S02'),
              [oa_request(1, '06:30:00', status=429, usage=oa_usage(i=None, o=None, cached=None, cw=None), resp=None, terminal=None, usd=settle(1, 0.8, 0.0, 'error_status')), reserve(1, 1, 0.8, '06:30:00')],
              ['Order'], 0.8, 0.0, 800000000, 'proxy_reservation', controls=('S02-control-error-status-after-reservation',))
rejected_case('S02-attempt-number-differs', ['S-02'], P('repro_p115.py', 'test_S02', 'note: settlement n=7'),
              [reserve(1, 1, 0.8, '06:30:00'), oa_request(7, '06:30:02', status=429, usage=oa_usage(i=None, o=None, cached=None, cw=None), resp=None, terminal=None, usd=settle(1, 0.8, 0.0, 'error_status'))],
              ['AttemptNumber'], 0.8, 0.0, 800000000, 'proxy_reservation', controls=('S02-control-error-status-after-reservation',))
u01 = oa_request(1, '06:30:02', status=429, usage=oa_usage(i=None, o=None, cached=None, cw=None), resp=None, terminal=None, usd=settle(1, 0.8, 0.0, 'error_status')); u01['n'] = True
rejected_case('U01-malformed-attempt-number', ['U-01'], P('repro_p121.py', 'test_U01_malformed_attempt_number_releases', 'n=true'),
              [reserve(1, 1, 0.8, '06:30:00'), u01], ['AttemptNumber'], 0.8, 0.0, 800000000, 'proxy_reservation', controls=('S02-control-error-status-after-reservation',))
r02 = settle(1, 0.8, '0.8', 'error_status')
rejected_case('R02-string-charge', ['R-02'], P('repro_p113.py', 'test_R02_malformed_charge_cancels_a_reservation'),
              [reserve(1, 1, 0.8, '06:30:00'), oa_request(1, '06:30:02', status=429, usage=oa_usage(i=None, o=None, cached=None, cw=None), resp=None, terminal=None, usd=r02)],
              ['AmountType'], 0.8, None, 800000000, 'proxy_reservation', controls=('S02-control-error-status-after-reservation',))
c4 = settle(1, 0.8, 0.0, 'error_status'); del c4['charge_basis']
rejected_case('C4-missing-charge-basis', ['C4'], P('P1.19 report', 'C4: 429 settlement without a basis'),
              [reserve(1, 1, 0.8, '06:30:00'), oa_request(1, '06:30:02', status=429, usage=oa_usage(i=None, o=None, cached=None, cw=None), resp=None, terminal=None, usd=c4)],
              ['BasisMissing'], 0.8, 0.0, 800000000, 'proxy_reservation', controls=('S02-control-error-status-after-reservation',))
rejected_case('S06-string-http-status', ['S-06'], P('repro_p115.py', 'test_S06'),
              [reserve(1, 1, 0.8, '06:30:00'), oa_request(1, '06:30:02', status='200', resp=None, usd=settle(1, 0.8, 0.0, 'usage'))],
              ['StatusType', 'Undercharge'], 0.8, 0.0, 800000000, 'proxy_reservation', controls=('VC01-openai-usage-settlement',))
rejected_case('P9-claimed-charge-above-reservation', ['P1.26-P9'], P('test_usage_ledger_pi_a134.py', 'test_contradicted_settlement_never_lowers_the_retained_reservation'),
              [reserve(1, 1, 0.4, '06:30:00'), oa_request(1, '06:30:02', usd=settle(1, 0.5, 0.7, 'usage'))], ['ReservedMismatch'], 0.5, 0.7, 700000000, 'proxy_charge',
              controls=('VC01-openai-usage-settlement',), inv=('INV-F3',))
CASES[-1]['expected']['attempts'][0]['valuation'] = v
C('S02-control-error-status-after-reservation', group='S-02', role='control', cls='producer_valid', reach='producer_reachable',
  findings=['S-02', 'D4'], origin=P('repro_p115.py', 'test_S02', 'control: reservation then 429'), invariants=['INV-F1', 'INV-A1'],
  sessions=[session(logs={'turn-1': [reserve(1, 1, 0.8, '06:30:00'), oa_request(1, '06:30:02', status=429, usage=oa_usage(i=None, o=None, cached=None, cw=None), resp=None, terminal=None, usd=settle(1, 0.8, 0.0, 'error_status'))]})],
  attempts=[attempt(K1, ev([(SRC, 1), (SRC, 2)]), sd('valid', 'error_status'), CLOSED_RES, NOT_REPORTED,
                    exposure(0, 'none', 'exact', 800000000, 'proxy_reservation'), diagnostics=['ErrorStatusSettlement', 'NonBillingUnverified'], reserved=0.8, charged=0.0)],
  attribution=[attr(K1)], objectives_exp=OK_OBJ(closed=esum(0, 'exact', 800000000)), admission=adm('admitted'),
  notes='D4/Q2: the 429 settlement closes the attempt but does not replace its exposure; the reservation stays the bound.')
C('D4-unbudgeted-error-status', group='D4', role='finding', cls='producer_valid', reach='producer_reachable',
  findings=['D4'], origin=P('contract', 'D4: unbudgeted non-200 with no usage'), invariants=['INV-F5', 'INV-A2'],
  controls=['VC06-unbudgeted-completed-request'],
  sessions=[session(logs={'turn-1': [oa_request(1, '06:30:02', status=529, usage=oa_usage(i=None, o=None, cached=None, cw=None), resp=None, terminal=None)]})],
  attempts=[attempt(R1L1, ev([(SRC, 1)]), sd('not_applicable'), NO_OBLIGATION, NOT_REPORTED, exposure(0, 'none', 'at_least', 0, 'none'), blocking=['UnresolvedExposure'])],
  objectives=[objective(admissions=[])], **UNB_UNCERTAIN([R1L1], blocking=('OwnershipUncertain', 'UnresolvedExposure')))
C('R04-reused-reservation-id', group='R-04', role='finding', cls='malformed', reach='producer_unreachable',
  findings=['R-04'], origin=P('repro_p113.py', 'test_R04'), invariants=['INV-F2', 'INV-F3'], controls=['VC05-unsettled-reservation'],
  sessions=[session(logs={'turn-1': [reserve(1, 1, 0.8, '06:30:00'), reserve(2, 1, 0.1, '06:30:01')]})],
  attempts=[attempt(K1, ev([(SRC, 1), (SRC, 2)]), sd('absent'), OUTSTANDING, NOT_REPORTED, exposure(0, 'none', 'exact', 900000000, 'proxy_reservation'), blocking=['ReservationConflict'])],
  attribution=[attr(K1)], objectives_exp=[obj_exp(status='not_established', reasons=['ReservationConflict'], outstanding=esum(0, 'exact', 900000000))],
  admission=adm('not_established'))
rc = reserve(1, 1, 0.8, '06:30:00')
C('C3-identical-reservation-copy', group='C3', role='finding', cls='malformed', reach='producer_unreachable',
  findings=['C3'], origin=P('P1.19 report', 'C3: exact reservation replay after settlement'), invariants=['INV-F2', 'INV-F6'],
  controls=['S02-control-error-status-after-reservation'],
  sessions=[session(logs={'turn-1': [rc, oa_request(1, '06:30:00', status=429, usage=oa_usage(i=None, o=None, cached=None, cw=None), resp=None, terminal=None, usd=settle(1, 0.8, 0.0, 'error_status')), rc]})],
  attempts=[attempt(K1, ev([(SRC, 1), (SRC, 2), (SRC, 3)]), sd('valid', 'error_status'), CLOSED_RES, NOT_REPORTED,
                    exposure(0, 'none', 'exact', 800000000, 'proxy_reservation'), diagnostics=['ErrorStatusSettlement', 'NonBillingUnverified', 'RepeatedIdenticalRow'], reserved=0.8, charged=0.0)],
  attribution=[attr(K1)], objectives_exp=OK_OBJ(closed=esum(0, 'exact', 800000000)), admission=adm('admitted'))
C('DUP-response-id-across-attempts', group='P1.26-P8', role='finding', cls='adversarial_synthetic', reach='producer_unreachable',
  findings=['P1.26-P8'], origin=P('test_usage_ledger_pi_a134.py', 'test_duplicate_response_id_keeps_the_reservation_counted'),
  invariants=['INV-F2', 'INV-F4'], controls=['VC11-multi-turn-session'],
  sessions=[session(logs={'turn-1': [reserve(1, 1, 0.5, '06:30:00'), oa_request(1, '06:30:02', resp='resp_dup', usd=settle(1, 0.5, 0.015, 'usage')),
                                     reserve(2, 2, 0.4, '06:30:04'), oa_request(2, '06:30:05', resp='resp_dup', usd=settle(2, 0.4, 0.015, 'usage'))]})],
  attempts=[attempt(K1, ev([(SRC, 1), (SRC, 2)]), sd('valid', 'usage'), CLOSED_SETTLED, v, exposure(15000000, 'usage_valuation', 'exact', 15000000, 'usage_valuation'), blocking=['DuplicateResponseIdentity'], reserved=0.5, charged=0.015),
            attempt('res:S1/turn-1:rid=2', ev([(SRC, 3), (SRC, 4)]), sd('valid', 'usage'), CLOSED_SETTLED, v, exposure(15000000, 'usage_valuation', 'exact', 15000000, 'usage_valuation'), blocking=['DuplicateResponseIdentity'], reserved=0.4, charged=0.015)],
  attribution=[attr(K1), attr('res:S1/turn-1:rid=2')],
  objectives_exp=[obj_exp(status='not_established', reasons=['DuplicateResponseIdentity'], closed=esum(30000000, 'exact', 30000000))],
  admission=adm('not_established'), notes='The duplicate identity blocks; by K-1 it does not change either attempt\'s money (no second fallback).')
C('A1-01-upstream-error-and-unknown-event', group='A1-01', role='finding', cls='malformed', reach='producer_unreachable',
  findings=['A1-01'], origin=P('repro_p111.py', 'A1-01', 'upstream_error 0.8 + unknown event carrying usd'), invariants=['INV-F3', 'INV-F6'],
  controls=['VC04-upstream-error-keeps-reservation'],
  sessions=[session(logs={'turn-1': [reserve(1, 1, 0.8, '06:30:00'), upstream_error(1, '06:30:02', usd=settle(1, 0.8, 0.8, 'reservation')),
                                     reserve(2, 2, 0.3, '06:30:03'), {'at': t('06:30:04'), 'event': 'retry_settlement', 'n': 2, 'usd': settle(2, 0.3, 0.0, 'reservation')}]})],
  attempts=[attempt(K1, ev([(SRC, 1), (SRC, 2)]), sd('valid', 'upstream_error'), {'state': 'closed', 'amount_source': 'reservation_confirmed'}, NOT_REPORTED,
                    exposure(0, 'none', 'exact', 800000000, 'proxy_reservation'), reserved=0.8, charged=0.8),
            attempt('res:S1/turn-1:rid=2', ev([(SRC, 3)]), sd('absent'), OUTSTANDING, NOT_REPORTED, exposure(0, 'none', 'exact', 300000000, 'proxy_reservation'), reserved=0.3)],
  unreadable=[{'evidence': 'S1/turn-1:4', 'reason': 'UnknownEvent', 'blocking': ['UnreadableEvidence']}],
  attribution=[attr(K1), attr('res:S1/turn-1:rid=2')],
  objectives_exp=[obj_exp(status='not_established', reasons=['UnreadableEvidence'], closed=esum(0, 'exact', 800000000), outstanding=esum(0, 'exact', 300000000))],
  admission=adm('not_established'))
C('V06-reservation-attempt-number-missing', group='V-06', role='finding', cls='malformed', reach='producer_unreachable',
  findings=['V-06'], origin=P('repro_p123.py', 'test_V06'), invariants=['INV-F3', 'INV-A5'], controls=['VC05-unsettled-reservation'],
  sessions=[session(logs={'turn-1': [{k: x for k, x in reserve(1, 1, 0.8, '06:30:00').items() if k != 'n'}]})],
  attempts=[attempt(K1, ev([(SRC, 1)]), sd('absent'), OUTSTANDING, NOT_REPORTED, exposure(0, 'none', 'exact', 800000000, 'proxy_reservation'), blocking=['MalformedEvidence'], reserved=0.8)],
  attribution=[attr(K1)], objectives_exp=[obj_exp(status='not_established', reasons=['MalformedEvidence'], outstanding=esum(0, 'exact', 800000000))],
  admission=adm('not_established'))

# =============================================================================================== unreadable and inventory
C('R05-non-object-json-rows', group='R-05', role='finding', cls='malformed', reach='producer_unreachable',
  findings=['R-05'], origin=P('repro_p113.py', 'test_R05'), invariants=['INV-F6', 'INV-A5'], controls=['VC01-openai-usage-settlement'],
  sessions=[session(logs={'turn-1': [reserve(1, 1, 0.5, '06:30:00'), 'null', '[]', oa_request(1, '06:30:02', usd=settle(1, 0.5, 0.015, 'usage'))]})],
  attempts=[attempt(K1, ev([(SRC, 1), (SRC, 4)]), sd('valid', 'usage'), CLOSED_SETTLED, v, exposure(15000000, 'usage_valuation', 'exact', 15000000, 'usage_valuation'), reserved=0.5, charged=0.015)],
  unreadable=[{'evidence': 'S1/turn-1:2', 'reason': 'NotObject', 'blocking': ['UnreadableEvidence']}, {'evidence': 'S1/turn-1:3', 'reason': 'NotObject', 'blocking': ['UnreadableEvidence']}],
  attribution=[attr(K1)], objectives_exp=[obj_exp(status='not_established', reasons=['UnreadableEvidence'], closed=esum(15000000, 'exact', 15000000))],
  admission=adm('not_established'))
C('U02-final-line-without-newline', group='U-02', role='finding', cls='malformed', reach='producer_reachable',
  findings=['U-02', 'C5'], origin=P('repro_p121.py', 'test_U02_unterminated_line'), invariants=['INV-F6', 'INV-A5'], controls=['VC05-unsettled-reservation'],
  sessions=[session(logs={'turn-1': {'lines': [line(reserve(1, 1, 0.3, '06:30:00'))], 'final_newline': False}})],
  attempts=[], unreadable=[{'evidence': 'S1/turn-1:1', 'reason': 'PartialFinalLine', 'blocking': ['UnreadableEvidence']}],
  objectives_exp=[obj_exp(status='not_established', reasons=['UnreadableEvidence'])], admission=adm('not_established'),
  notes='Owner question Q7: a complete JSON object without its newline is still unverifiable (crash mid-write), so it is unreadable evidence, never a counted reservation.')
C('A1-03-unreferenced-log', group='A1-03', role='finding', cls='attribution_uncertain', reach='producer_reachable',
  findings=['A1-03', 'R-03'], origin=P('repro_p111.py', 'A1-03', extra={'also': ['repro_p113.py::test_R03']}), invariants=['INV-F6', 'INV-A5'],
  controls=['VC01-openai-usage-settlement'],
  sessions=[session(logs={'turn-1': [reserve(1, 1, 0.5, '06:30:00'), oa_request(1, '06:30:02', usd=settle(1, 0.5, 0.015, 'usage'))]},
                    extra_files={'turn-2.proxy.jsonl': {'lines': [line(reserve(1, 1, 0.8, '06:40:00'))], 'final_newline': True}})],
  attempts=[attempt(K1, ev([(SRC, 1), (SRC, 2)]), sd('valid', 'usage'), CLOSED_SETTLED, v, exposure(15000000, 'usage_valuation', 'exact', 15000000, 'usage_valuation'), reserved=0.5, charged=0.015)],
  inventory=[{'session': 'S1', 'kind': 'unreferenced_log', 'file': 'turn-2.proxy.jsonl', 'blocking': ['InventoryIncomplete']}],
  attribution=[attr(K1)], objectives_exp=[obj_exp(status='not_established', reasons=['InventoryIncomplete'], closed=esum(15000000, 'exact', 15000000))],
  admission=adm('not_established'))
unrel = lambda logs, extra=None: session(S2, parent='other-root', logs=logs, extra_files=extra)
base_ok = session(logs={'turn-1': [reserve(1, 1, 0.5, '06:30:00'), oa_request(1, '06:30:02', usd=settle(1, 0.5, 0.015, 'usage'))]})
UNREL_LC = {'S1': {'state': 'retired', 'clock': 'trusted'}, 'S2': {'state': 'retired', 'clock': 'trusted'}}
C('S05-unrelated-orphan-log-does-not-block', group='S-05', role='finding', cls='attribution_uncertain', reach='producer_reachable',
  findings=['S-05'], origin=P('repro_p115.py', 'test_S05', '(b) unrelated orphan log'), invariants=['INV-A5', 'INV-T2'],
  controls=['A1-03-unreferenced-log'],
  sessions=[base_ok, unrel({'turn-1': []}, {'turn-9.proxy.jsonl': {'lines': [line(reserve(1, 1, 0.8, '06:40:00'))], 'final_newline': True}})],
  attempts=[attempt(K1, ev([(SRC, 1), (SRC, 2)]), sd('valid', 'usage'), CLOSED_SETTLED, v, exposure(15000000, 'usage_valuation', 'exact', 15000000, 'usage_valuation'), reserved=0.5, charged=0.015)],
  inventory=[{'session': 'S2', 'kind': 'unreferenced_log', 'file': 'turn-9.proxy.jsonl', 'blocking': ['InventoryIncomplete']}],
  lifecycle=UNREL_LC, attribution=[attr(K1)], objectives_exp=OK_OBJ(closed=esum(15000000, 'exact', 15000000)), admission=adm('admitted'))
C('V07-unrelated-unreadable-log-does-not-block', group='V-07', role='finding', cls='attribution_uncertain', reach='producer_reachable',
  findings=['V-07'], origin=P('repro_p123.py', 'test_V07'), invariants=['INV-A5', 'INV-T2'], controls=['R05-non-object-json-rows'],
  sessions=[base_ok, unrel({'turn-1': ['{not json']})],
  attempts=[attempt(K1, ev([(SRC, 1), (SRC, 2)]), sd('valid', 'usage'), CLOSED_SETTLED, v, exposure(15000000, 'usage_valuation', 'exact', 15000000, 'usage_valuation'), reserved=0.5, charged=0.015)],
  unreadable=[{'evidence': 'S2/turn-1:1', 'reason': 'NotJson', 'blocking': ['UnreadableEvidence']}],
  lifecycle=UNREL_LC, attribution=[attr(K1)], objectives_exp=OK_OBJ(closed=esum(15000000, 'exact', 15000000)), admission=adm('admitted'))
u07 = oa_request(1, '06:30:02', resp='other'); u07['usage'] = [1]
C('U07-unrelated-non-object-usage-does-not-block', group='U-07', role='finding', cls='attribution_uncertain', reach='producer_unreachable',
  findings=['U-07'], origin=P('repro_p121.py', 'test_U07_unrelated_malformed_usage_aborts'), invariants=['INV-A5', 'INV-T2'],
  controls=['VC01-openai-usage-settlement'],
  sessions=[base_ok, unrel({'turn-1': [u07]})],
  attempts=[attempt(K1, ev([(SRC, 1), (SRC, 2)]), sd('valid', 'usage'), CLOSED_SETTLED, v, exposure(15000000, 'usage_valuation', 'exact', 15000000, 'usage_valuation'), reserved=0.5, charged=0.015),
            attempt('row:S2/turn-1:line=1', ev([('S2/turn-1', 1)]), sd('not_applicable'), NO_OBLIGATION, unpriceable(['MalformedUsage']),
                    exposure(0, 'none', 'at_least', 0, 'none'), blocking=['MalformedEvidence', 'UnresolvedExposure'])],
  lifecycle=UNREL_LC, attribution=[attr(K1), attr('row:S2/turn-1:line=1', ownership='unowned', placement='not_evaluated')],
  objectives_exp=OK_OBJ(closed=esum(15000000, 'exact', 15000000)), admission=adm('admitted'))

# =============================================================================================== lanes
C('N03-contradicted-admission', group='N-03', role='finding', cls='adversarial_synthetic', reach='producer_unreachable',
  findings=['N-03'], origin=P('repro_p125.py', 'test_N03_contradicted_admission', 'admission labelled openai/api'), invariants=['INV-F9', 'INV-A2'],
  controls=['N03-control-unlabelled-admission'],
  sessions=[session(**SUB, logs={'turn-1': [admitted(1, 1, '06:30:00', provider='openai', auth_lane='api')]})],
  attempts=[attempt(ADM, ev([(SRC, 1)]), sd('not_applicable'), NO_OBLIGATION, NOT_REPORTED, exposure(0, 'none', 'at_least', 0, 'none'),
                    blocking=['LaneContradiction', 'UnaccountedAttempt', 'UnresolvedExposure'])],
  objectives=[objective(admissions=[])], lifecycle=None,
  attribution=[attr(ADM, ownership='uncertain', placement='uncertain', cause='NoAdmission', candidates=['o'])],
  objectives_exp=[obj_exp(status='not_established', reasons=['LaneContradiction', 'OwnershipUncertain', 'UnaccountedAttempt', 'UnresolvedExposure'], commitments=())],
  admission=adm('not_established'))
C('N03-control-unlabelled-admission', group='N-03', role='control', cls='producer_valid', reach='producer_reachable',
  findings=['N-03'], origin=P('repro_p125.py', 'test_N03_contradicted_admission', 'control: producer admission, no measurement'), invariants=['INV-F9'],
  sessions=[session(**SUB, logs={'turn-1': [admitted(1, 1, '06:30:00')]})],
  attempts=[attempt(ADM, ev([(SRC, 1)]), sd('not_applicable'), NO_OBLIGATION, {'state': 'not_applicable'}, CODEX, diagnostics=['UnfinishedAdmission'])],
  objectives=[objective(admissions=[])],
  attribution=[attr(ADM, ownership='uncertain', placement='uncertain', cause='NoAdmission', candidates=['o'])],
  objectives_exp=[obj_exp(commitments=())], admission=adm('admitted'))
C('N03-contradicted-refused-row', group='N-03', role='finding', cls='adversarial_synthetic', reach='producer_unreachable',
  findings=['N-03'], origin=P('repro_p125.py', 'test_N03_contradicted_admission', 'refused row labelled openai/api'), invariants=['INV-F9'],
  controls=['N03-control-unlabelled-refused-row'],
  sessions=[session(**SUB, logs={'turn-1': [refused('06:30:00', provider='openai', auth_lane='api')]})],
  attempts=[], non_attempt=[{'evidence': 'S1/turn-1:1', 'kind': 'refused', 'blocking': ['LaneContradiction']}],
  objectives=[objective(admissions=[])], objectives_exp=[obj_exp(status='not_established', reasons=['LaneContradiction'], commitments=())],
  admission=adm('not_established'))
C('N03-control-unlabelled-refused-row', group='N-03', role='control', cls='producer_valid', reach='producer_reachable',
  findings=['N-03'], origin=P('repro_p125.py', 'test_N03_contradicted_admission', 'control: refused row unlabelled'), invariants=['INV-F9'],
  sessions=[session(**SUB, logs={'turn-1': [refused('06:30:00')]})],
  attempts=[], non_attempt=[{'evidence': 'S1/turn-1:1', 'kind': 'refused', 'blocking': []}],
  objectives=[objective(admissions=[])], objectives_exp=[obj_exp(commitments=())], admission=adm('admitted'))
C('N03-identity-refusal-in-subscription', group='N-03', role='finding', cls='adversarial_synthetic', reach='producer_unreachable',
  findings=['N-03'], origin=P('test_usage_ledger_pi_a134.py', 'test_N03_contradicted_refused_and_any_identity_refusal_block'), invariants=['INV-F9'],
  controls=['VC08-subscription-measured-request'],
  sessions=[session(**SUB, logs={'turn-1': [identity_refusal(1, '06:30:00', ASTRA, FABLE)]})],
  attempts=[attempt(R1L1, ev([(SRC, 1)]), sd('not_applicable'), NO_OBLIGATION, NOT_REPORTED, exposure(0, 'none', 'at_least', 0, 'none'),
                    blocking=['IdentityContradiction', 'LaneContradiction', 'UnaccountedAttempt', 'UnresolvedExposure'], correlation='unattributed')],
  objectives=[objective(admissions=[])],
  attribution=[attr(R1L1, ownership='uncertain', placement='uncertain', cause='NoAdmission', candidates=['o'])],
  objectives_exp=[obj_exp(status='not_established', reasons=['IdentityContradiction', 'LaneContradiction', 'OwnershipUncertain', 'UnaccountedAttempt', 'UnresolvedExposure'], commitments=())],
  admission=adm('not_established'))
MEAS_UNKNOWN = measurement(1, status='unknown', reason='exchange_not_completed')
C('V03-subscription-upstream-error-labelled-api', group='V-03', role='finding', cls='adversarial_synthetic', reach='producer_unreachable',
  findings=['V-03'], origin=P('repro_p123.py', 'test_V03'), invariants=['INV-F9', 'INV-A2'], controls=['V03-control-labelled-upstream-error'],
  sessions=[session(**SUB, logs={'turn-1': [admitted(1, 1, '06:30:00'), upstream_error(1, '06:30:02', labels={'provider': 'openai', 'auth_lane': 'api', 'subscription_proxy': MEAS_UNKNOWN})]})],
  attempts=[attempt(ADM, ev([(SRC, 1), (SRC, 2)]), sd('not_applicable'), NO_OBLIGATION, NOT_REPORTED, exposure(0, 'none', 'at_least', 0, 'none'),
                    blocking=['LaneContradiction', 'UnaccountedAttempt', 'UnresolvedExposure'])],
  objectives=[objective(admissions=[])],
  attribution=[attr(ADM, ownership='uncertain', placement='uncertain', cause='NoAdmission', candidates=['o'])],
  objectives_exp=[obj_exp(status='not_established', reasons=['LaneContradiction', 'OwnershipUncertain', 'UnaccountedAttempt', 'UnresolvedExposure'], commitments=())],
  admission=adm('not_established'))
C('U06-subscription-upstream-error-unlabelled', group='U-06', role='finding', cls='adversarial_synthetic', reach='producer_unreachable',
  findings=['U-06'], origin=P('repro_p121.py', 'test_U06'), invariants=['INV-F9'], controls=['V03-control-labelled-upstream-error'],
  sessions=[session(**SUB, logs={'turn-1': [admitted(1, 1, '06:30:00'), upstream_error(1, '06:30:02', labels={'subscription_proxy': MEAS_UNKNOWN})]})],
  attempts=[attempt(ADM, ev([(SRC, 1), (SRC, 2)]), sd('not_applicable'), NO_OBLIGATION, NOT_REPORTED, exposure(0, 'none', 'at_least', 0, 'none'),
                    blocking=['LaneContradiction', 'UnaccountedAttempt', 'UnresolvedExposure'])],
  objectives=[objective(admissions=[])],
  attribution=[attr(ADM, ownership='uncertain', placement='uncertain', cause='NoAdmission', candidates=['o'])],
  objectives_exp=[obj_exp(status='not_established', reasons=['LaneContradiction', 'OwnershipUncertain', 'UnaccountedAttempt', 'UnresolvedExposure'], commitments=())],
  admission=adm('not_established'), notes='The subscription proxy labels every upstream_error it writes; an absent label there is contradicting evidence.')
C('V03-control-labelled-upstream-error', group='V-03', role='control', cls='producer_valid', reach='producer_reachable',
  findings=['V-03', 'U-06'], origin=P('repro_p123.py', 'test_V03', 'control: producer-labelled measured upstream_error'), invariants=['INV-F9'],
  sessions=[session(**SUB, logs={'turn-1': [admitted(1, 1, '06:30:00'), upstream_error(1, '06:30:02', labels={**SUB_LABELS, 'model': ASTRA, 'subscription_proxy': MEAS_UNKNOWN})]})],
  attempts=[attempt(ADM, ev([(SRC, 1), (SRC, 2)]), sd('not_applicable'), NO_OBLIGATION, {'state': 'not_applicable'}, CODEX, diagnostics=['UncertainCodexCall'])],
  objectives=[objective(admissions=[])],
  attribution=[attr(ADM, ownership='uncertain', placement='uncertain', cause='NoAdmission', candidates=['o'])],
  objectives_exp=[obj_exp(commitments=())], admission=adm('admitted'))
C('N03-api-row-other-provider', group='N-03', role='finding', cls='adversarial_synthetic', reach='producer_unreachable',
  findings=['N-03'], origin=P('test_usage_ledger_pi_a134.py', 'test_N03_api_session_provider_evidence'), invariants=['INV-F9'],
  controls=['VC02-anthropic-usage-settlement'],
  sessions=[session(model='anthropic/' + FABLE, logs={'turn-1': [reserve(1, 1, 0.3, '06:30:00'), an_request(1, '06:30:02', AU, usd=settle(1, 0.3, 0.028, 'usage'), provider='openai')]})],
  attempts=[attempt(K1, ev([(SRC, 1), (SRC, 2)]), sd('valid', 'usage'), CLOSED_SETTLED, valued([FABLE], 28000000, 28000000, True, [D_AN(FABLE, AU, 28000000)]),
                    exposure(28000000, 'usage_valuation', 'exact', 28000000, 'usage_valuation'), blocking=['LaneContradiction'], reserved=0.3, charged=0.028)],
  attribution=[attr(K1)], objectives_exp=[obj_exp(status='not_established', reasons=['LaneContradiction'], closed=esum(28000000, 'exact', 28000000))],
  admission=adm('not_established'),
  ambiguity={'fields': ['attempts.valuation'], 'note': 'Which usage schema applies when the row names another provider is undetermined; the contract prices by the session launch provider, and the lane contradiction blocks regardless.'})

# =============================================================================================== correlation of errors
ORPH = lambda key, blocking=('UnaccountedAttempt', 'UnresolvedExposure'), corr='unattributed', cand=None: attempt(
    key, ev([(SRC, int(key.split('=')[-1]))]), sd('not_applicable'), NO_OBLIGATION, NOT_REPORTED, exposure(0, 'none', 'at_least', 0, 'none'),
    blocking=list(blocking), correlation=corr, candidates=cand)
C('N04-header-write-error-reused-n', group='N-04', role='finding', cls='adversarial_synthetic', reach='producer_reachable',
  findings=['N-04', 'V-04'], origin=P('repro_p125.py', 'test_N04_reused_n', 'claimed: refused(A), request n=2 (B), stream_error n=2 (C)'),
  invariants=['INV-F6', 'INV-F3'], controls=['N04-control-request-alone'],
  sessions=[session(logs={'turn-1': [refused('06:30:01', reason='body not received in time', status=408), oa_request(2, '06:30:02', resp='resp_2'), stream_error(2, '06:30:03', read_failure=False)]})],
  attempts=[attempt('row:S1/turn-1:line=2', ev([(SRC, 2)]), sd('not_applicable'), NO_OBLIGATION, v, exposure(15000000, 'usage_valuation', 'exact', 15000000, 'usage_valuation')),
            ORPH('row:S1/turn-1:line=3')],
  non_attempt=[{'evidence': 'S1/turn-1:1', 'kind': 'refused', 'blocking': []}],
  objectives=[objective(admissions=[])], **UNB_UNCERTAIN(['row:S1/turn-1:line=2', 'row:S1/turn-1:line=3'], blocking=('OwnershipUncertain', 'UnaccountedAttempt', 'UnresolvedExposure')))
C('N04-control-request-alone', group='N-04', role='control', cls='producer_valid', reach='producer_reachable',
  findings=['N-04'], origin=P('repro_p125.py', 'test_N04_reused_n', 'control: request n=2 alone'), invariants=['INV-F6'],
  sessions=[session(logs={'turn-1': [refused('06:30:01', reason='body not received in time', status=408), oa_request(2, '06:30:02', resp='resp_2')]})],
  attempts=[attempt('row:S1/turn-1:line=2', ev([(SRC, 2)]), sd('not_applicable'), NO_OBLIGATION, v, exposure(15000000, 'usage_valuation', 'exact', 15000000, 'usage_valuation'))],
  non_attempt=[{'evidence': 'S1/turn-1:1', 'kind': 'refused', 'blocking': []}],
  objectives=[objective(admissions=[])], **UNB_UNCERTAIN(['row:S1/turn-1:line=2']))
C('N04-ambiguous-read-failure', group='N-04', role='finding', cls='adversarial_synthetic', reach='producer_reachable',
  findings=['N-04'], origin=P('test_usage_ledger_pi_a134.py', 'test_N04_ambiguous_attempt_number_fails_closed_without_banning_reuse'),
  invariants=['INV-F6'], controls=['VC12-reused-n-without-errors'],
  sessions=[session(logs={'turn-1': [oa_request(2, '06:30:02', resp='resp_a'), stream_error(2, '06:30:03'), oa_request(2, '06:30:04', resp='resp_b')]})],
  attempts=[attempt('row:S1/turn-1:line=1', ev([(SRC, 1)]), sd('not_applicable'), NO_OBLIGATION, v, exposure(15000000, 'usage_valuation', 'exact', 15000000, 'usage_valuation')),
            ORPH('row:S1/turn-1:line=2', corr='ambiguous', cand=['row:S1/turn-1:line=1', 'row:S1/turn-1:line=3']),
            attempt('row:S1/turn-1:line=3', ev([(SRC, 3)]), sd('not_applicable'), NO_OBLIGATION, v, exposure(15000000, 'usage_valuation', 'exact', 15000000, 'usage_valuation'))],
  objectives=[objective(admissions=[])], **UNB_UNCERTAIN(['row:S1/turn-1:line=1', 'row:S1/turn-1:line=2', 'row:S1/turn-1:line=3'], blocking=('OwnershipUncertain', 'UnaccountedAttempt', 'UnresolvedExposure')))
se1 = stream_error(1, '06:30:01')
C('N04-duplicated-read-failure', group='N-04', role='finding', cls='malformed', reach='producer_unreachable',
  findings=['N-04', 'P1.26-E-4'], origin=P('test_usage_ledger_pi_a134.py', 'test_N04_read_failure_pairs_only_with_its_own_later_request_row', 'two read failures'),
  invariants=['INV-F6'], controls=['VC07-read-failure-paired'],
  sessions=[session(logs={'turn-1': [se1, se1, oa_request(1, '06:30:02')]})],
  attempts=[ORPH('row:S1/turn-1:line=2'),
            attempt('row:S1/turn-1:line=3', ev([(SRC, 1), (SRC, 3)]), sd('not_applicable'), NO_OBLIGATION, v, exposure(15000000, 'usage_valuation', 'exact', 15000000, 'usage_valuation'),
                    diagnostics=['ReadFailurePaired'], correlation='producer_shape', rule='RS-1')],
  objectives=[objective(admissions=[])], **UNB_UNCERTAIN(['row:S1/turn-1:line=2', 'row:S1/turn-1:line=3'], blocking=('OwnershipUncertain', 'UnaccountedAttempt', 'UnresolvedExposure')),
  notes='Byte-identical stream_error lines are two evidence items (E-1); one request row answers one read failure.')
C('N04-budgeted-header-failure-no-own-reservation', group='N-04', role='finding', cls='adversarial_synthetic', reach='producer_reachable',
  findings=['N-04'], origin=P('test_usage_ledger_pi_a134.py', 'test_N04_budgeted_header_failure_is_covered_by_its_own_open_reservation', 'control in A1.3.4'),
  invariants=['INV-F6', 'INV-A2'], controls=['N04-budgeted-header-failure-own-reservation'],
  sessions=[session(logs={'turn-1': [reserve(1, 1, 0.5, '06:30:00'), oa_request(1, '06:30:02', usd=settle(1, 0.5, 0.015, 'usage')), stream_error(1, '06:30:05', read_failure=False)]})],
  attempts=[attempt(K1, ev([(SRC, 1), (SRC, 2)]), sd('valid', 'usage'), CLOSED_SETTLED, v, exposure(15000000, 'usage_valuation', 'exact', 15000000, 'usage_valuation'), reserved=0.5, charged=0.015),
            ORPH('row:S1/turn-1:line=3')],
  attribution=[attr(K1), attr('row:S1/turn-1:line=3')],
  objectives_exp=[obj_exp(status='not_established', reasons=['UnaccountedAttempt', 'UnresolvedExposure'], closed=esum(15000000, 'at_least', 15000000))],
  admission=adm('not_established'))
C('N04-budgeted-header-failure-own-reservation', group='N-04', role='control', cls='producer_valid', reach='producer_reachable',
  findings=['N-04'], origin=P('test_usage_ledger_pi_a134.py', 'test_N04_budgeted_header_failure_is_covered_by_its_own_open_reservation'),
  invariants=['INV-F6', 'INV-A1'],
  sessions=[session(logs={'turn-1': [reserve(1, 1, 0.5, '06:30:00'), oa_request(1, '06:30:02', usd=settle(1, 0.5, 0.015, 'usage')), reserve(2, 2, 0.4, '06:30:04'), stream_error(2, '06:30:05', read_failure=False)]})],
  attempts=[attempt(K1, ev([(SRC, 1), (SRC, 2)]), sd('valid', 'usage'), CLOSED_SETTLED, v, exposure(15000000, 'usage_valuation', 'exact', 15000000, 'usage_valuation'), reserved=0.5, charged=0.015),
            attempt('res:S1/turn-1:rid=2', ev([(SRC, 3), (SRC, 4)]), sd('absent'), OUTSTANDING, NOT_REPORTED, exposure(0, 'none', 'exact', 400000000, 'proxy_reservation'),
                    correlation='producer_shape', rule='RS-2', reserved=0.4)],
  attribution=[attr(K1), attr('res:S1/turn-1:rid=2')],
  objectives_exp=OK_OBJ(closed=esum(15000000, 'exact', 15000000), outstanding=esum(0, 'exact', 400000000)), admission=adm('admitted'))

# =============================================================================================== lifecycle and time
UNPL = lambda k: attr(k, placement='unplaceable', cause='ClockUntrusted')
def settled_logs(res_at='06:30:00', set_at='06:30:02', rid=1, usd_res=0.5):
    return [reserve(rid, rid, usd_res, res_at), oa_request(rid, set_at, usd=settle(rid, usd_res, 0.015, 'usage'))]
VALID_ATT = lambda ev_lines=((SRC, 1), (SRC, 2)): attempt(K1, ev(list(ev_lines)), sd('valid', 'usage'), CLOSED_SETTLED, v, exposure(15000000, 'usage_valuation', 'exact', 15000000, 'usage_valuation'), reserved=0.5, charged=0.015)
N05_OBJ = lambda: [objective(start='05:50:00', admissions=[journal_admission('a1', S1, 1.0, at='05:59:00')])]
C('N05-retirement-before-later-delivery', group='N-05', role='finding', cls='adversarial_synthetic', reach='producer_unreachable',
  findings=['N-05'], origin=P('repro_p125.py', 'test_N05_earlier_retirement', 'claimed (re-expressed with D1 ownership)'),
  invariants=['INV-T4', 'INV-A5', 'INV-F7'], controls=['N05-control-consistent-retirement'],
  sessions=[session(lifecycle=[('launch', '05:59:10', 1), ('retire', '06:40:00', None), ('deliver', '06:05:00', 2), ('retire', '06:10:00', None)],
                    logs={'turn-1': [reserve(1, 1, 0.8, '06:00:00')], 'turn-2': []})],
  attempts=[attempt(K1, ev([(SRC, 1)]), sd('absent'), OUTSTANDING, NOT_REPORTED, exposure(0, 'none', 'exact', 800000000, 'proxy_reservation'), reserved=0.8)],
  lifecycle={'S1': {'state': 'contradicted', 'clock': 'untrusted', 'reason': 'LifecycleBackwards'}},
  objectives=N05_OBJ(), attribution=[UNPL(K1)],
  objectives_exp=[obj_exp(status='not_established', reasons=['PlacementUnplaceable', 'ReopenedWithoutAdmission'], outstanding=esum(0, 'exact', 800000000), commitments=(('a1', 'reopened', '200000000'),))],
  admission=adm('not_established'),
  notes='Under D1 the attempt is owned through the admission, so it can no longer fall outside the window; the rollback still makes the session clock untrusted.')
C('N05-control-consistent-retirement', group='N-05', role='control', cls='producer_valid', reach='producer_reachable',
  findings=['N-05'], origin=P('repro_p125.py', 'test_N05_earlier_retirement', 'control: consistent retirement'), invariants=['INV-T3', 'INV-A1'],
  sessions=[session(lifecycle=[('launch', '05:59:10', 1), ('retire', '06:10:00', None)], logs={'turn-1': [reserve(1, 1, 0.8, '06:00:00')]})],
  attempts=[attempt(K1, ev([(SRC, 1)]), sd('absent'), OUTSTANDING, NOT_REPORTED, exposure(0, 'none', 'exact', 800000000, 'proxy_reservation'), reserved=0.8)],
  objectives=N05_OBJ(), attribution=[attr(K1)],
  objectives_exp=[obj_exp(outstanding=esum(0, 'exact', 800000000))], admission=adm('admitted'))
C('U03-retirement-before-turn-evidence', group='U-03', role='finding', cls='adversarial_synthetic', reach='producer_unreachable',
  findings=['U-03'], origin=P('repro_p121.py', 'test_U03'), invariants=['INV-T4', 'INV-F7'], controls=['U03-control'],
  sessions=[session(lifecycle=[('launch', '06:00:10', 1), ('retire', '06:40:00', None)], logs={'turn-1': settled_logs(set_at='06:45:00')})],
  attempts=[VALID_ATT()], lifecycle={'S1': {'state': 'contradicted', 'clock': 'untrusted', 'reason': 'TurnEvidenceAfterRetirement'}},
  attribution=[UNPL(K1)], objectives_exp=[obj_exp(status='not_established', reasons=['PlacementUnplaceable'], closed=esum(15000000, 'exact', 15000000))],
  admission=adm('not_established'))
C('U03-control', group='U-03', role='control', cls='producer_valid', reach='producer_reachable',
  findings=['U-03'], origin=P('repro_p121.py', 'test_U03', 'control'), invariants=['INV-T4'],
  sessions=[session(lifecycle=[('launch', '06:00:10', 1), ('retire', '06:40:00', None)], logs={'turn-1': settled_logs(set_at='06:35:00')})],
  attempts=[VALID_ATT()], attribution=[attr(K1)], objectives_exp=OK_OBJ(closed=esum(15000000, 'exact', 15000000)), admission=adm('admitted'))
for cid, findings, origin, res_at, set_at, ctrl in (
        ('C1-settlement-after-reference-time', ['C1'], P('P1.19 report', 'C1: valid settlement at now+2 min'), '06:55:00', '07:02:00', 'C1-control-before-reference'),
        ('S03-settlement-stamped-far-future', ['S-03'], P('repro_p115.py', 'test_S03', 'settlement stamped 2099'), '06:55:00', None, 'C1-control-before-reference')):
    logs = settled_logs(res_at=res_at, set_at=set_at or '06:56:00')
    if set_at is None: logs[1]['at'] = '2099-01-01T00:00:00Z'
    C(cid, group=findings[0], role='finding', cls='adversarial_synthetic', reach='producer_unreachable', findings=findings, origin=origin,
      invariants=['INV-T4', 'INV-F7'], controls=[ctrl],
      sessions=[session(lifecycle=[('launch', '06:00:10', 1)], logs={'turn-1': logs})], attempts=[VALID_ATT()],
      lifecycle={'S1': {'state': 'open', 'clock': 'untrusted', 'reason': 'AheadOfReference'}},
      objectives=[objective(admissions=[journal_admission('a1', S1, 0.5)])], attribution=[UNPL(K1)],
      objectives_exp=[obj_exp(status='not_established', reasons=['PlacementUnplaceable'], closed=esum(15000000, 'exact', 15000000), commitments=(('a1', 'launched_open', '485000000'),))],
      admission=adm('not_established'))
C('C1-control-before-reference', group='C1', role='control', cls='producer_valid', reach='producer_reachable',
  findings=['C1', 'S-03'], origin=P('P1.19 report', 'C1 control: settlement at now-1 min'), invariants=['INV-T4'],
  sessions=[session(lifecycle=[('launch', '06:00:10', 1)], logs={'turn-1': settled_logs(res_at='06:55:00', set_at='06:59:00')})],
  attempts=[VALID_ATT()], lifecycle={'S1': {'state': 'open', 'clock': 'trusted'}}, attribution=[attr(K1)],
  objectives=[objective(admissions=[journal_admission('a1', S1, 0.5)])],
  objectives_exp=[obj_exp(closed=esum(15000000, 'exact', 15000000), commitments=(('a1', 'launched_open', '485000000'),))],
  admission=adm('admitted'))
C('C2-unsettled-reservation-after-reference-time', group='C2', role='finding', cls='adversarial_synthetic', reach='producer_unreachable',
  findings=['C2'], origin=P('P1.19 report', 'C2: unsettled reserve at now+2 min'), invariants=['INV-T4'], controls=['VC05-unsettled-reservation'],
  sessions=[session(lifecycle=[('launch', '06:00:10', 1)], logs={'turn-1': [reserve(1, 1, 0.8, '07:02:00')]})],
  attempts=[attempt(K1, ev([(SRC, 1)]), sd('absent'), OUTSTANDING, NOT_REPORTED, exposure(0, 'none', 'exact', 800000000, 'proxy_reservation'), reserved=0.8)],
  lifecycle={'S1': {'state': 'open', 'clock': 'untrusted', 'reason': 'AheadOfReference'}}, attribution=[UNPL(K1)],
  objectives_exp=[obj_exp(status='not_established', reasons=['PlacementUnplaceable'], outstanding=esum(0, 'exact', 800000000), commitments=(('a1', 'launched_open', '200000000'),))],
  admission=adm('not_established'))
C('R01-dated-rows-backwards', group='R-01', role='finding', cls='adversarial_synthetic', reach='producer_unreachable',
  findings=['R-01', 'P1.26-P10'], origin=P('repro_p113.py', 'test_R01_dated_clock_conflict_erases_a_reservation'), invariants=['INV-T4', 'INV-F7', 'INV-T1'],
  controls=['VC01-openai-usage-settlement'],
  sessions=[session(logs={'turn-1': settled_logs(res_at='06:40:00', set_at='06:20:00')})],
  attempts=[VALID_ATT()], lifecycle={'S1': {'state': 'retired', 'clock': 'untrusted', 'reason': 'ProxyRowsOutOfOrder'}},
  attribution=[UNPL(K1)], objectives_exp=[obj_exp(status='not_established', reasons=['PlacementUnplaceable'], closed=esum(15000000, 'exact', 15000000))],
  admission=adm('not_established'), notes='The settlement stays valid and priced while unplaceable (P1.26 P10): attribution never changes money.')
und = oa_request(1, None, usd=settle(1, 0.5, 0.015, 'usage')); del und['at']
C('A04-undated-settlement-between-dated-neighbours', group='A-04', role='finding', cls='historical_legacy', reach='not_established',
  findings=['A-04'], origin=P('repro_unit_a.py', 'A-04', 'undated reservation settlement'), invariants=['INV-T4', 'INV-F7'],
  controls=['VC01-openai-usage-settlement'],
  sessions=[session(logs={'turn-1': [reserve(1, 1, 0.5, '06:30:00'), und, reserve(2, 2, 0.3, '06:30:05')]})],
  attempts=[VALID_ATT(), attempt('res:S1/turn-1:rid=2', ev([(SRC, 3)]), sd('absent'), OUTSTANDING, NOT_REPORTED, exposure(0, 'none', 'exact', 300000000, 'proxy_reservation'), reserved=0.3)],
  attribution=[attr(K1), attr('res:S1/turn-1:rid=2')],
  objectives_exp=OK_OBJ(closed=esum(15000000, 'exact', 15000000), outstanding=esum(0, 'exact', 300000000)), admission=adm('admitted'),
  notes='Placed by its consistent dated neighbours (P-2).', ambiguity={'fields': ['reachability'], 'note': 'Whether any producer epoch wrote undated rows is not established.'})
C('A1-02-undated-row-inverted-neighbours', group='A1-02', role='finding', cls='adversarial_synthetic', reach='producer_unreachable',
  findings=['A1-02'], origin=P('repro_p111.py', 'A1-02'), invariants=['INV-T4'], controls=['A04-undated-settlement-between-dated-neighbours'],
  sessions=[session(logs={'turn-1': [reserve(1, 1, 0.5, '06:40:00'), und, reserve(2, 2, 0.3, '06:20:00')]})],
  attempts=[VALID_ATT(), attempt('res:S1/turn-1:rid=2', ev([(SRC, 3)]), sd('absent'), OUTSTANDING, NOT_REPORTED, exposure(0, 'none', 'exact', 300000000, 'proxy_reservation'), reserved=0.3)],
  lifecycle={'S1': {'state': 'retired', 'clock': 'untrusted', 'reason': 'ProxyRowsOutOfOrder'}},
  attribution=[UNPL(K1), UNPL('res:S1/turn-1:rid=2')],
  objectives_exp=[obj_exp(status='not_established', reasons=['PlacementUnplaceable'], closed=esum(15000000, 'exact', 15000000), outstanding=esum(0, 'exact', 300000000))],
  admission=adm('not_established'))
C('S04-next-unrelated-row-does-not-close', group='S-04', role='finding', cls='producer_valid', reach='producer_reachable',
  findings=['S-04'], origin=P('repro_p115.py', 'test_S04'), invariants=['INV-T3', 'INV-F3'], controls=['VC05-unsettled-reservation'],
  sessions=[session(logs={'turn-1': [reserve(1, 1, 0.8, '06:30:00'), refused('06:30:01')]})],
  attempts=[attempt(K1, ev([(SRC, 1)]), sd('absent'), OUTSTANDING, NOT_REPORTED, exposure(0, 'none', 'exact', 800000000, 'proxy_reservation'), reserved=0.8)],
  non_attempt=[{'evidence': 'S1/turn-1:2', 'kind': 'refused', 'blocking': []}],
  objectives=[objective(start='06:30:00')], attribution=[attr(K1)],
  objectives_exp=[obj_exp(outstanding=esum(0, 'exact', 800000000))], admission=adm('admitted'),
  notes='The obligation stays outstanding until the retirement; the refused row says nothing about when the attempt ended.')
C('U04-settled-after-closure-stays-with-owner', group='U-04', role='finding', cls='producer_valid', reach='producer_reachable',
  findings=['U-04', 'D1'], origin=P('repro_p121.py', 'test_U04', 'reservation before closure, settlement after'), invariants=['INV-T3'],
  controls=['U04-control-settled-before-closure'],
  sessions=[session(logs={'turn-1': settled_logs(res_at='06:35:00', set_at='06:45:00')})], attempts=[VALID_ATT()],
  objectives=[objective(closed='06:40:00')], attribution=[attr(K1, provisional_retained=True)],
  objectives_exp=[obj_exp(closed=esum(15000000, 'exact', 15000000))], admission=adm('refused'),
  notes='C1: the closed objective keeps its attempt; the closed objective admits nothing (refused).')
C('U04-control-settled-before-closure', group='U-04', role='control', cls='producer_valid', reach='producer_reachable',
  findings=['U-04'], origin=P('repro_p121.py', 'test_U04', 'control'), invariants=['INV-T3'],
  sessions=[session(logs={'turn-1': settled_logs(res_at='06:35:00', set_at='06:38:00')})], attempts=[VALID_ATT()],
  objectives=[objective(closed='06:40:00')], attribution=[attr(K1)],
  objectives_exp=[obj_exp(closed=esum(15000000, 'exact', 15000000))], admission=adm('refused'))
C('D1-closed-objective-provisional', group='D1', role='finding', cls='producer_valid', reach='producer_reachable',
  findings=['D1'], origin=P('contract', 'D1: closed objective with an unresolved obligation'), invariants=['INV-T3'],
  controls=['U04-control-settled-before-closure'],
  sessions=[session(lifecycle=[('launch', '06:00:10', 1)], logs={'turn-1': [reserve(1, 1, 0.5, '06:35:00')]})],
  attempts=[attempt(K1, ev([(SRC, 1)]), sd('absent'), OUTSTANDING, NOT_REPORTED, exposure(0, 'none', 'exact', 500000000, 'proxy_reservation'), reserved=0.5)],
  lifecycle={'S1': {'state': 'open', 'clock': 'trusted'}}, objectives=[objective(closed='06:40:00')],
  attribution=[attr(K1, provisional_retained=True)],
  objectives_exp=[obj_exp(provisional=True, outstanding=esum(0, 'exact', 500000000), commitments=(('a1', 'launched_open', '500000000'),))],
  admission=adm('refused'))
hb_logs = [reserve(1, 1, 0.5, '06:10:00'), oa_request(1, '06:30:00', usd=settle(1, 0.5, 0.015, 'usage'))]
C('Hb-window-independence', group='H-b', role='finding', cls='attribution_uncertain', reach='producer_reachable',
  findings=['H-b'], origin=P('probe_p127.py', 'test_Hb'), invariants=['INV-F7', 'INV-T1'], controls=['VC01-openai-usage-settlement'],
  sessions=[session(logs={'turn-1': hb_logs})], attempts=[VALID_ATT()],
  evaluations=[{'name': 'window-from-06:00', 'objectives': [objective(start='06:00:00')], 'admission_request': {'objective': 'o', 'new': nano('0.1')},
                'expected': {'attribution': [attr(K1)], 'objectives': OK_OBJ(closed=esum(15000000, 'exact', 15000000)), 'admission': adm('admitted')}},
               {'name': 'window-from-06:20', 'objectives': [objective(start='06:20:00')], 'admission_request': {'objective': 'o', 'new': nano('0.1')},
                'expected': {'attribution': [attr(K1, placement='straddles_start')],
                             'objectives': [obj_exp(status='not_established', reasons=['PlacementStraddles'], closed=esum(15000000, 'exact', 15000000))],
                             'admission': adm('not_established')}}],
  notes='Attempt facts are shared by both evaluations; only attribution and admission differ.')

# =============================================================================================== ownership and commitments
C('OWN-two-admitting-objectives', group='D1', role='finding', cls='attribution_uncertain', reach='producer_reachable',
  findings=['D1'], origin=P('contract', 'D1/O-3: one session admitted by two objectives'), invariants=['INV-T2'], controls=['VC01-openai-usage-settlement'],
  sessions=[session(logs={'turn-1': settled_logs()})], attempts=[VALID_ATT()],
  evaluations=[{'name': 'default', 'objectives': [objective('o'), objective('p', admissions=[journal_admission('b1', S1, 1.0)])],
                'admission_request': {'objective': 'o', 'new': nano('0.1')},
                'expected': {'attribution': [attr(K1, obj=None, ownership='uncertain', placement='uncertain', cause='SeveralAdmissions', candidates=['o', 'p'])],
                             'objectives': [obj_exp('o', status='not_established', reasons=['OwnershipUncertain']),
                                            obj_exp('p', status='not_established', reasons=['OwnershipUncertain'], commitments=(('b1', 'retired', '0'),))],
                             'admission': adm('not_established')}}])
C('OWN-budgeted-session-without-admission', group='D1', role='finding', cls='attribution_uncertain', reach='producer_reachable',
  findings=['D1', 'XO-1'], origin=P('contract', 'D1/O-3: session in the tree, no admission record'), invariants=['INV-T2'],
  controls=['VC01-openai-usage-settlement'],
  sessions=[session(logs={'turn-1': settled_logs()})], attempts=[VALID_ATT()], objectives=[objective(admissions=[])],
  attribution=[attr(K1, ownership='uncertain', placement='uncertain', cause='NoAdmission', candidates=['o'])],
  objectives_exp=[obj_exp(status='not_established', reasons=['OwnershipUncertain'], commitments=())], admission=adm('not_established'))
reopen_lc = [('launch', '06:00:10', 1), ('retire', '06:20:00', None), ('deliver', '06:25:00', 2)]
for path, adms, aid in (('legacy', [legacy_admission(S1, 0.8)], 'legacy:' + S1), ('journal', [journal_admission('a1', S1, 0.8)], 'a1')):
    C(f'N07-reopened-session-{path}', group='N-07', role='finding', cls='producer_valid', reach='producer_reachable',
      findings=['N-07', 'D2'], origin=P('repro_p125.py' if path == 'legacy' else 'repro_n07_journal.py',
                                        'test_N07_legacy_commitment_reopened' if path == 'legacy' else 'test_journal_path', 'claimed'),
      invariants=['INV-A3', 'INV-A5'], controls=[f'N07-control-retired-{path}'],
      sessions=[session(lifecycle=reopen_lc, logs={'turn-1': [], 'turn-2': []})], attempts=[],
      lifecycle={'S1': {'state': 'reopened', 'clock': 'trusted'}}, objectives=[objective(admissions=adms)],
      objectives_exp=[obj_exp(status='not_established', reasons=['ReopenedWithoutAdmission'], commitments=((aid, 'reopened', '800000000'),))],
      admission=adm('not_established', new='0.5'), request_new='0.5')
    C(f'N07-control-retired-{path}', group='N-07', role='control', cls='producer_valid', reach='producer_reachable',
      findings=['N-07'], origin=P('repro_p125.py' if path == 'legacy' else 'repro_n07_journal.py',
                                  'test_N07_legacy_commitment_reopened' if path == 'legacy' else 'test_journal_path', 'control: retire, no deliver'),
      invariants=['INV-A3'], sessions=[session(lifecycle=[('launch', '06:00:10', 1), ('retire', '06:20:00', None)], logs={'turn-1': []})], attempts=[],
      objectives=[objective(admissions=adms)], objectives_exp=[obj_exp(commitments=((aid, 'retired', '0'),))],
      admission=adm('admitted', new='0.5'), request_new='0.5')
C('S08-negative-commitment-cap', group='S-08', role='finding', cls='malformed', reach='producer_unreachable',
  findings=['S-08'], origin=P('P1.15 report', 'S-08: pending max_usd -1'), invariants=['INV-A3'], controls=['S08-control-pending-commitment'],
  sessions=[], attempts=[], lifecycle={},
  objectives=[objective(tree=(), admissions=[journal_admission('a9', S3, -1, launched=False)])],
  objectives_exp=[obj_exp(status='not_established', reasons=['CommitmentInvalid'], commitments=(('a9', 'invalid', '0'),))],
  admission=adm('not_established', new='0.4'), request_new='0.4')
C('S08-control-pending-commitment', group='S-08', role='control', cls='producer_valid', reach='producer_reachable',
  findings=['S-08'], origin=P('contract', 'pending allocation committed in full (A-2)'), invariants=['INV-A3', 'INV-A1'],
  sessions=[], attempts=[], lifecycle={},
  objectives=[objective(tree=(), admissions=[journal_admission('a9', S3, 0.5, launched=False)])],
  objectives_exp=[obj_exp(commitments=(('a9', 'pending', '500000000'),))],
  admission=adm('admitted', new='0.4'), request_new='0.4')

# =============================================================================================== order independence and determinism
twin_s1 = session(logs={'turn-1': settled_logs()})
twin_s2 = session(S2, logs={'turn-1': [reserve(1, 1, 0.3, '06:31:00')]})
for cid, order in (('ORD-sessions-in-ledger-order', (twin_s1, twin_s2)), ('ORD-sessions-reversed', (twin_s2, twin_s1))):
    C(cid, group='INV-F8', role='valid_control', cls='producer_valid', reach='producer_reachable', findings=[],
      origin=P('contract', 'INV-F8/F10: presentation order and determinism'), invariants=['INV-F8', 'INV-F10'],
      sessions=[json.loads(json.dumps(x)) for x in order],
      attempts=[VALID_ATT(), attempt('res:S2/turn-1:rid=1', ev([('S2/turn-1', 1)]), sd('absent'), OUTSTANDING, NOT_REPORTED,
                                     exposure(0, 'none', 'exact', 300000000, 'proxy_reservation'), reserved=0.3)],
      lifecycle=UNREL_LC,
      objectives=[objective(tree=(S1, S2), admissions=[journal_admission('a1', S1, 1.0), journal_admission('a2', S2, 1.0)])],
      attribution=[attr(K1), attr('res:S2/turn-1:rid=1')],
      objectives_exp=[obj_exp(closed=esum(15000000, 'exact', 15000000), outstanding=esum(0, 'exact', 300000000),
                              commitments=(('a1', 'retired', '0'), ('a2', 'retired', '0')))],
      admission=adm('admitted'))
CASES[-1]['twin_of'] = 'ORD-sessions-in-ledger-order'

# =============================================================================================== write
UNB = ('A1.3.4 attributes by session tree and time window and admits; the contract requires an admission record for '
       'ownership (D1), and an unbudgeted session has none (O-6, owner question Q8), so the objective is not established. '
       'The attempt-level facts are the point of this case.')
RATIONALE = {
    'VC06-unbudgeted-completed-request': UNB, 'VC07-read-failure-paired': UNB, 'VC12-reused-n-without-errors': UNB,
    'N04-control-request-alone': UNB, 'N08-control-priced-model': UNB, 'A1-06-session-file-is-not-evidence': UNB,
    'A1-06-control-no-session-file': UNB,
    'N08-model-without-producer-rate': UNB + ' In addition, the model has no producer rate, so the exposure is AtLeast(0) (N-08).',
    'D4-unbudgeted-error-status': 'A1.3.4 treats an unbudgeted non-200 row as non-blocking (pi_request_without_response, assumed unbilled); D4 makes unknown provider billing AtLeast(0), blocking. Ownership is also uncertain (O-6).',
    'N02-malformed-anthropic-tier': 'A1.3.4 reads a non-string Anthropic tier as standard (known N-02); the contract makes it MalformedTier, so the usage settlement is rejected.',
    'N07-reopened-session-journal': 'A1.3.4 releases a commitment on any retirement ever recorded (known N-07); D2 refuses reopening until atomic re-admission exists, so the objective is not established.',
    'N07-reopened-session-legacy': 'A1.3.4 releases a commitment on any retirement ever recorded (known N-07); D2 refuses reopening until atomic re-admission exists, so the objective is not established.',
    'OWN-two-admitting-objectives': 'A1.3.4 has no ownership concept and counts the session in the tree being evaluated; D1/O-3 makes two admitting objectives uncertain ownership, blocking both.',
    'OWN-budgeted-session-without-admission': 'A1.3.4 counts tree members; D1 never infers ownership from ancestry, so a budgeted session with no admission record is uncertain and blocks its candidate.',
    'S08-negative-commitment-cap': 'A1.3.4 accepts a negative pending cap (known S-08, unintegrated); A-1 makes it CommitmentInvalid.',
    'U02-final-line-without-newline': 'A1.3.4 reads a complete API-lane object without its newline (known U-02); E-1 makes the final partial line unreadable evidence (owner question Q7).',
    'U07-unrelated-non-object-usage-does-not-block': 'A1.3.4 raises on a non-object usage in an unrelated session, failing every objective (known U-07); the contract records MalformedEvidence scoped to that session only.',
    'U08-missing-observed-tier': 'A1.3.4 rejects a usage settlement with a missing tier (known U-08); the contract prices it over the cheapest and dearest multipliers, consistent with the producer charging the dearest (owner question Q5).',
    'V06-reservation-attempt-number-missing': 'A1.3.4 counts the reservation without an issue (known V-06); the contract counts it and adds blocking MalformedEvidence.',
    'D1-successor-constrained-by-predecessor-obligations': 'A1.3.4 has no predecessor concept; it reports the session as straddling the successor window (unattributable). The contract applies D1: predecessor obligations constrain the successor (12.2), giving a refusal.',
}


def fix():
    """Post-definition corrections recorded in code so the change is visible."""
    by = {c['case_id']: c for c in CASES}
    for cid, text in RATIONALE.items(): by[cid]['rationale'] = text
    a = by['R06-launch-row-missing']['expected']['attempts'][0]
    a['blocking'] = ['SettlementRejected']                     # inventory findings are session-level, not attempt-level
    for c in CASES:
        for e in c['evaluations']:
            for at in e['expected']['attribution']:
                if at['ownership'] == 'unowned': at['objective'] = None


def main(out):
    fix()
    os.makedirs(os.path.join(out, 'cases'), exist_ok=True)
    ids = [c['case_id'] for c in CASES]
    assert len(ids) == len(set(ids)), 'duplicate case ids'
    for c in CASES:
        prev = os.path.join(out, 'cases', c['case_id'] + '.json')
        if os.path.exists(prev): c['historical'] = json.load(open(prev)).get('historical') or {}
        with open(os.path.join(out, 'cases', c['case_id'] + '.json'), 'w') as fh:
            json.dump(c, fh, indent=1, sort_keys=True); fh.write('\n')
    print(f'{len(CASES)} cases written')


if __name__ == '__main__':
    main(sys.argv[1])
