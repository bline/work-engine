# Authored claims for the six dollar-reservation cases. Literal values only (see DERIVATIONS-BATCH1.md).
from build_dossiers import *

E, NS, PM, VA, T, G, CO = FAMILIES

def scope_premise(case, short, origins, payload_ref, contract_id, grant_id, authority):
    return {'premise_id': '%s/PREM-SCOPE' % short, 'kind': 'payload_premise',
            'statement': 'Conditional initial_reference_scope for source S1/turn-1: reference_namespace=dollar_ticket, duplicate_rule=same_origin_only, covered origins %s (declared contract %s, grant %s, payload %s, authority_ref %s).' % (origins, contract_id, grant_id, payload_ref, authority),
            'status': RU('Declared conditional_fixture payload is located and raw-consistent (see %s/NS2) but no root admission of this per-case declaration exists; the cited authority_ref (%s) does not admit per-case scopes. Base plan §3 requires an expressly admitted initial reference scope for TicketClaim grouping.' % (short, authority), 'root', []),
            'source_refs': [case.line(o) for o in origins], 'policy_clauses': [P('BASE', '§2 provenance admission; §3 canonical high ownership'), P('CK', 'INITIAL-REFERENCE rev2'), P('FRPLAN', 'Finite truth and applicability interfaces')]}

RATES_PREMISE = lambda case, short: {'premise_id': '%s/PREM-RATES' % short, 'kind': 'policy_clause',
    'statement': 'inputs.rates binds openai_api_rates.json c0bdfd66… and anthropic_api_rates.json 8a286433…, byte-equal to the pinned oracle/rates files (pinned premise, not live pricing). Rates are USD per million tokens; 1 USD/M = 1000 nano per token.',
    'status': DN, 'source_refs': [case.tok('/inputs/rates/openai_api_rates.json'), case.tok('/inputs/rates/anthropic_api_rates.json')],
    'policy_clauses': [P('CK', 'MONEY rates'), P('RATES-OPENAI', 'models.gpt-6-astra; tier_multiplier'), P('RATES-ANTHROPIC', 'models; source unit statement')]}

COMMON_E_NEG = [('delete an origin from the RawOriginInventory while outputs stay coherent', 'E-1 totality / RawOriginInventory equality'),
                ('re-encode a raw line (key order/whitespace) keeping decoded JSON equal', 'raw_utf8_sha256 / exact-byte origin identity')]

def lifecycle_claim(case, short, retire_at, rows_at, arg):
    s = case.tok('/inputs/sessions/0/lifecycle')
    refs = [s, case.tok('/inputs/sessions/0/lifecycle/0/at', '2026-11-10T06:00:10Z'), case.tok('/inputs/reference_time', '2026-11-10T07:00:00Z')]
    if retire_at:
        refs.append(case.tok('/inputs/sessions/0/lifecycle/1/at', retire_at))
    state = {'state': 'retired', 'at': retire_at} if retire_at else {'state': 'open'}
    return C(short, 'T1', T, {'session': 'S1'}, DN, refs + [case.f(o, '/at', a) for o, a in rows_at],
             [P('CONTRACT', '§10 L-1..L-6')], arg=arg,
             expected={'variant': 'SessionLifecycle', 'alias': 'S1', **state, 'reopened': False, 'clock': 'trusted',
                       'checks': {'L1_rows_monotone': True, 'L2_rows_not_before_turn_start': True, 'L3_rows_not_after_retire': True, 'L6_no_row_after_reference_time': True}},
             pos=['accept lifecycle %s/reopened false/clock trusted' % state['state']],
             neg=[('supplied lifecycle state open (or reopened true) with unchanged rows', 'L-4/L-5 lifecycle derivation'),
                  ('move a proxy row after reference_time without updating clock', 'L-6 clock')])

def p9():
    case = Case('P9-claimed-charge-above-reservation'); s = 'P9'; cid = case.cid
    o1, o2 = 'S1/turn-1:1', 'S1/turn-1:2'
    prem = [scope_premise(case, s, [o1, o2], 'oracle/r8-completion-B/P9-claimed-charge-above-reservation.provenance.json#S1/turn-1', 'conditional-ticket:S1/turn-1', 'initial-scope:S1/turn-1', 'ROOT-ADOPTION-R6.json'), RATES_PREMISE(case, s)]
    SC = '%s/PREM-SCOPE' % s
    arg = 'DERIVATIONS-BATCH1.md#p9'
    cl = []
    cl.append(C(s, 'E1', E, {'origins': [o1, o2]}, DN, [case.line(o1), case.line(o2), case.f(o1, '/event', 'usd_reserved'), case.f(o2, '/event', 'request'), case.f(o2, '/usd')],
        [P('CK', 'EVIDENCE E-1, E-2'), P('BASE', '§2 Origin identity')], arg=arg,
        expected={'variant': 'RawOriginInventory', 'origin_count': 2, 'origins': [
            {'origin_id': o1, 'original_position': 1, 'evidence_kind': 'Reservation', 'raw_utf8_sha256': case.lines[o1]['raw_utf8_sha256']},
            {'origin_id': o2, 'original_position': 2, 'evidence_kind': 'Settlement(event=request, usd present)', 'raw_utf8_sha256': case.lines[o2]['raw_utf8_sha256']}],
            'unreadable': [], 'non_attempt': [], 'final_newline': True,
            'retained_nonshape_fields': {o1: ['/input_bound', '/bound_basis'], o2: ['/seconds', '/usage/reasoning_tokens', '/usd/session_spent_usd', '/usd/multiplier_floor']}},
        pos=['accept exact two-origin inventory'], neg=COMMON_E_NEG))
    cl.append(C(s, 'E2', E, {'origins': [o1, o2]}, DN, [case.line(o1), case.line(o2)],
        [P('CK', 'LOGICAL-PRIMARY rev2'), P('AMD', '§2 primary_memberships evidence classes')], arg=arg,
        expected={'variant': 'EvidenceClassCardinality', 'reservation_classes': [[o1]], 'settlement_classes': [[o2]],
                  'reservation_class_count': 1, 'settlement_class_count': 1, 'exact_copy_classes': [], 'byte_identical_pairs': []},
        pos=['accept one reservation class and one settlement class'],
        neg=[('supply a second reservation class (or merge classes) with matching validity refs', 'R8-RESERVATION-01 class cardinality by admitted evidence equivalence')],
        findings=[('R8-RESERVATION-01', 'bears_on')]))
    cl.append(C(s, 'E3', E, {'origin': o2}, DN,
        [case.tok('/inputs/sessions/0/launch/model', 'openai/gpt-6-astra'), case.tok('/inputs/sessions/0/launch/auth_lane', 'api'),
         case.f(o2, '/model', 'gpt-6-astra'), case.f(o2, '/response_model', 'gpt-6-astra'), case.f(o2, '/provider', '__absent__'),
         case.f(o2, '/requested_tier', 'null'), case.f(o2, '/observed_tier', 'default'), case.f(o2, '/response_id', 'resp_1')],
        [P('CK', 'EVIDENCE MC-1..MC-3'), P('CK', 'VALUATION V-1')], arg=arg,
        expected={'variant': 'IdentityStates', 'pinned': {'family': 'openai', 'model_id': 'gpt-6-astra'},
                  'row_identity_fields': {'model': {'presence': 'value', 'json_type': 'string', 'lexeme': 'gpt-6-astra'}, 'response_model': {'presence': 'value', 'json_type': 'string', 'lexeme': 'gpt-6-astra'}},
                  'provider_label': 'absent (MC-2 OpenAI default)', 'canonical_identity_set': [{'family': 'openai', 'model_id': 'gpt-6-astra'}],
                  'malformed_identities': [], 'producer_identity_bindings': []},
        pos=['accept single canonical identity'], neg=[('replace response_model by numeric 7 while keeping eligible/complete outputs', 'R8-IDENTITY-01 presence/type before identity collection')]))
    # namespace
    cl.append(C(s, 'NS1', NS, {'origins': [o1, o2]}, DN, [case.f(o1, '/rid', '1'), case.f(o2, '/usd/rid', '1'), case.tok('/inputs/sessions/0/launch/auth_lane', 'api')],
        [P('BASE', '§3 C base IDs'), P('AMD', '§3 NAMESPACE-APPLICATION')], arg=arg,
        expected={'variant': 'RawReferenceNamespace', 'per_origin': {o1: {'namespace': 'dollar_ticket', 'rid': '1', 'usable': True}, o2: {'namespace': 'dollar_ticket', 'rid': '1', 'usable': True}}, 'admission_namespace_origins': []},
        pos=['dollar_ticket applies to both origins'], neg=[('declare reference_namespace=admission for these dollar origins', 'NAMESPACE-APPLICATION mismatch')]))
    cl.append(C(s, 'NS2', NS, {'declared_contract': 'conditional-ticket:S1/turn-1'}, DN, [case.line(o1), case.line(o2)],
        [P('CK', 'INITIAL-REFERENCE rev2'), P('R3', 'Exact applicability')], arg=arg,
        expected={'variant': 'DeclaredPayloadRawConsistency', 'declared': {'reference_namespace': 'dollar_ticket', 'duplicate_rule': 'same_origin_only', 'covered_origin_refs': [o1, o2], 'covered_extent': 'sha256:' + case.sr['inputs_canonical_sha256']},
                  'raw_consistent': True, 'mismatches': [], 'establishes_admission': False},
        pos=['declared payload consistent with raw inventory/namespace'], neg=[('stale covered_extent digest or one covered origin dropped', 'INITIAL-REFERENCE supplied-payload binding'), ('swap payload fragment to another case', 'payload fragment resolution')]))
    cl.append(C(s, 'NS3', NS, {'grant': 'initial-scope:S1/turn-1'}, RU('Per-case conditional initial_reference_scope admission absent', 'root', [SC]), [SC],
        [P('BASE', '§2-§3'), P('FRPLAN', 'Finite truth and applicability interfaces')], arg=arg,
        if_admitted={'variant': 'NamespaceApplication', 'initial_base': {'kind': 'TicketClaim', 'scope': cid + '/S1/turn-1', 'rid': '1', 'origins': [o1, o2]}, 'atomize': False, 'break_copies': False, 'hold_units': [[o1, o2]],
                     'if_not_admitted_sketch': 'OriginBase per origin; no logical primary group (no admitted exact link); settlement becomes NoReservation orphan; materially different money (see DERIVATIONS-BATCH1.md#p9).'},
        pos=['root-admitted exact conditional declaration'], neg=[('admission namespace or another source scope applied to these origins', 'NAMESPACE-APPLICATION')]))
    # primary
    cl.append(C(s, 'PM1', PM, {'origins': [o1, o2]}, DN,
        [case.f(o1, '/n', '1'), case.f(o1, '/reserved_usd', '0.4'), case.f(o2, '/n', '1'), case.f(o2, '/usd/charged_usd', '0.7'), case.f(o2, '/usd/reserved_usd', '0.5'),
         case.f(o2, '/usd/charge_basis', 'usage'), case.f(o2, '/http_status', '200'), case.f(o2, '/terminal', 'response.completed'), '%s/PREM-RATES' % s],
        [P('CK', 'SETTLEMENT §5 conditions 3,4,5,7,8'), P('CK', 'MONEY producer amounts'), P('CK', 'TOLERANCE')], arg=arg,
        expected={'variant': 'OriginLocalChecks', 'reservation': {o1: {'n': 'positive_integer', 'reserved_nano': '400000000'}},
                  'settlement': {o2: {'n': 'positive_integer', 'charged_nano': '700000000', 'claimed_reserved_nano': '500000000', 'basis': 'usage', 'status': 'integer_200', 'request_identity_equals_pinned': True,
                                      'producer_price_nano': '15000000', 'tolerance_range_ok': True, 'undercharge': False}},
                  'local_failures': []},
        pos=['accept local field states'], neg=[('mark charged_usd malformed or status StatusType', 'origin-local SETTLEMENT checks')]))
    cl.append(C(s, 'PM2', PM, {'obligation': 'res:S1/turn-1:rid=1'}, RU('Logical R/S grouping requires admitted TicketClaim scope', 'root', [SC]), [SC, case.line(o1), case.line(o2)],
        [P('CK', 'LOGICAL-PRIMARY rev2'), P('CK', 'SETTLEMENT §5 cond.4'), P('CK', 'TEMPORAL-AND-FINALIZATION §6'), P('CK', 'F-REPLACEMENT-2'), P('CK', 'WHOLE-COVERAGE rev3')], arg=arg,
        if_admitted={'variant': 'PrimaryCoverage', 'primary_membership': {'obligation_id': 'res:S1/turn-1:rid=1', 'reservation_classes': [{'origin_refs': [o1], 'representative_origin_ref': o1}],
                                                                          'settlement_classes': [{'origin_refs': [o2], 'representative_origin_ref': o2}], 'usage_origin_refs': [o2]},
                     'settlement_validity': {'decision': 'rejected', 'reasons': ['ReservedMismatch']}, 'temporal_obligation': 'outstanding',
                     'obligation': {'state': 'outstanding', 'amount_source': 'reservation_bound'}, 'replacement': {'eligible': False, 'reason': 'NotUsageSettlement'},
                     'execution': 'terminal', 'forwarding': 'forwarded', 'accepted_coverage': [], 'definite_links': []},
        pos=['admitted-branch rejected/outstanding/reservation_bound'], neg=[('coherent Valid/closed/settlement_amount output with unchanged raw', 'SETTLEMENT cond.4 ReservedMismatch (500000000 != 400000000)')],
        findings=[('R8-OWNER-01', 'bears_on'), ('R8-COVERAGE-01', 'bears_on')]))
    # valuation
    cl.append(C(s, 'VA1', VA, {'usage_origin': o2}, DN,
        [case.f(o2, '/usage/input_tokens', '1000'), case.f(o2, '/usage/output_tokens', '100'), case.f(o2, '/usage/cached_tokens', '0'), case.f(o2, '/usage/cache_write_tokens', '0'),
         case.f(o2, '/observed_tier', 'default'), case.f(o2, '/http_status', '200'), case.f(o2, '/terminal', 'response.completed'), '%s/PREM-RATES' % s],
        [P('CK', 'VALUATION V-1..V-5'), P('CK', 'PRICE-OPENAI'), P('CK', 'USAGE-PRESENCE'), P('RATES-OPENAI', 'gpt-6-astra input 10, output 50; tier default 1.0')], arg=arg,
        expected={'variant': 'ObservationValuation', 'origin': o2, 'state': 'valued', 'identities': [{'family': 'openai', 'model_id': 'gpt-6-astra'}], 'tier_set': ['default'], 'splits': {'cached': '0', 'write': '0'},
                  'complete': True, 'low': nano('15000000'), 'high': nano('15000000'), 'derivation': '1000*10000 + 100*50000 = 15000000 nano'},
        pos=['accept valued 15000000/15000000 complete'], neg=[('valuation low 0 or unpriceable with unchanged usage', 'V-1..V-5 / L4 supported low')]))
    cl.append(C(s, 'VA2', VA, {'origins': [o1, o2]}, DN, [case.f(o1, '/reserved_usd', '0.4'), case.f(o2, '/usd/reserved_usd', '0.5'), case.f(o2, '/usd/charged_usd', '0.7'), s + '/VA1'],
        [P('BASE', '§3 components follow owning origin; §6 retained charges'), P('CK', 'MONEY FB-1/FB-2'), P('CK', 'WHOLE-COVERAGE rev3'), P('AMD', '§3 P9-RETAINED-CHARGE'), P('FRPLAN', 'Finding matrix R8-FIN-01')], arg=arg,
        expected={'variant': 'RetainedComponents', 'components': [
            {'kind': 'reservation', 'origin_ref': o1, 'nano': '400000000', 'role': 'high_component'},
            {'kind': 'claimed_reserved', 'origin_ref': o2, 'nano': '500000000', 'role': 'validity_premise_only_not_a_high_component'},
            {'kind': 'charge', 'origin_ref': o2, 'nano': '700000000', 'role': 'high_component'},
            {'kind': 'usage_high', 'origin_ref': o2, 'nano': '15000000', 'role': 'high_component'}],
            'invariant': 'Under every lawful unit-structure branch the 700000000 charge component is retained by its owning origin S1/turn-1:2; scope high is at_least and >= 700000000; no branch replaces it by 400000000 or 500000000.'},
        pos=['accept 400/700/15M retained components'], neg=[('coherent rewrite charge/component/unit/rollup 700000000->500000000 and commitment 300000000->500000000', 'R8-FIN-01 retained raw charge'), ('omit charge component from unit while totals agree', 'WHOLE-COVERAGE component retention')],
        findings=[('R8-FIN-01', 'direct')]))
    cl.append(C(s, 'VA3', VA, {'unit': 'res:S1/turn-1:rid=1'}, RU('Unit structure depends on admitted TicketClaim scope', 'root', [SC]), [SC, s + '/VA2'],
        [P('CK', 'X-DECISION X-BRANCH-025'), P('CK', 'BASIS-TIE'), P('CK', 'X-COMPONENTS rev2')], arg=arg,
        if_admitted={'variant': 'UnitExposure', 'unit_origins': [o1, o2], 'pool': 'pi_api', 'components': {'reservation': '400000000', 'charge': '700000000', 'usage_high': '15000000'},
                     'high': {'kind': 'at_least', 'nano': '700000000', 'basis': 'proxy_charge'}, 'low': {'nano': '15000000', 'basis': 'usage_valuation'}, 'billing': 'unresolved', 'blocking_must_include': ['UnresolvedExposure']},
        pos=['admitted-branch AtLeast700000000 proxy_charge, low 15000000'], neg=[('high exact or 500000000', 'FB-2 / R8-FIN-01')], findings=[('R8-FIN-01', 'direct')]))
    # T
    cl.append(lifecycle_claim(case, s, '2026-11-10T06:50:00Z', [(o1, '2026-11-10T06:30:00Z'), (o2, '2026-11-10T06:30:02Z')], arg))
    cl.append(C(s, 'T2', T, {'origins': [o1, o2], 'evaluation': 'default'}, DN,
        [case.tok('/evaluations/0/objectives/0/admissions/0/rows/1/event', 'launched'), case.tok('/evaluations/0/objectives/0/admissions/0/rows/1/session_id', '00000000-0000-4000-8000-0000000000c1'),
         case.tok('/inputs/sessions/0/session_id', '00000000-0000-4000-8000-0000000000c1'), case.tok('/evaluations/0/objectives/0/tree_sessions/0')],
        [P('CONTRACT', '§11.1 O-1, O-3a, O-7')], arg=arg,
        expected={'variant': 'Ownership', 'per_origin': {o1: {'ownership': 'owned', 'objective': 'o', 'admission': 'a1 launched row (evaluations/0/objectives/0/admissions/0/rows/1)'}, o2: {'ownership': 'owned', 'objective': 'o', 'admission': 'a1 launched row'}}, 'other_candidates': []},
        pos=['owned by o via a1'], neg=[('ownership uncertain/unadmitted with unchanged journal', 'O-1 admission ownership')]))
    cl.append(C(s, 'T3', T, {'origins': [o1, o2], 'evaluation': 'default', 'objective': 'o'}, DN,
        [case.tok('/evaluations/0/objectives/0/started_at', '2026-11-10T06:00:00Z'), case.tok('/evaluations/0/objectives/0/closed_at'), case.absent('/evaluations/0/objectives/0/close'), case.f(o1, '/at', '2026-11-10T06:30:00Z'), case.f(o2, '/at', '2026-11-10T06:30:02Z'), case.tok('/inputs/sessions/0/lifecycle/1/at', '2026-11-10T06:50:00Z')],
        [P('CONTRACT', '§11.2 P-1..P-4'), P('CONTRACT', '§12.4 step 1 closure claim')], arg=arg,
        expected={'variant': 'Placement', 'window': {'start': '2026-11-10T06:00:00Z', 'end': 'open', 'closure': 'no_claim'}, 'per_origin': {o1: 'inside', o2: 'inside'},
                  'invariant': 'Every candidate interval (closed [06:30:00,06:30:02] or outstanding [06:30:00,06:50:00]) lies inside the open window, so placement is inside under every unit-structure branch.'},
        pos=['inside'], neg=[('placement before/straddles with matching reasons', 'R8-PLACEMENT-01 raw interval placement')], findings=[('R8-PLACEMENT-01', 'bears_on')]))
    # gate/commitment
    cl.append(C(s, 'G1', G, {'admission': 'a1'}, DN, [case.tok('/profile', 'current_producer'), case.absent('/inputs/gates'), case.absent('/trusted_fixture_inputs')],
        [P('CONTRACT', '§2.3 ExecutionGate; Revision6 gate trust boundary'), P('CONTRACT', '§12.3 A-5')], arg=arg,
        expected={'variant': 'GateQualification', 'qualified': False, 'reasons': ['NoGateEvidence'], 'provenance': 'none'},
        pos=['qualified false NoGateEvidence'], neg=[('gate qualified true from raw fields', 'Revision6 trust boundary')]))
    cl.append(C(s, 'G2', G, {'admission': 'a1'}, DN, [case.tok('/evaluations/0/objectives/0/admissions/0/rows/0/max_usd', '1.0'), case.tok('/evaluations/0/objectives/0/admissions/0/rows/1/event', 'launched'), s + '/T1', s + '/G1'],
        [P('CONTRACT', '§12.3 A-1, A-3, A-5'), P('CONTRACT', '§2.3 producer fact')], arg=arg,
        expected={'variant': 'CommitmentState', 'admission_id': 'a1', 'session': '00000000-0000-4000-8000-0000000000c1', 'cap': nano('1000000000'), 'state': 'retired_resumable', 'kind': 'at_least', 'blocking_must_include': ['UnenforcedCommitment']},
        pos=['retired_resumable at_least'], neg=[('state retired / kind exact without qualified gate', 'L-7 / A-3 / A-5')]))
    cl.append(C(s, 'G3', G, {'admission': 'a1', 'field': 'committed'}, RU('committed = max(0, cap - session owned high) depends on owned high, which depends on unit structure', 'root', [SC]), [SC, s + '/G2', s + '/VA3'],
        [P('CONTRACT', '§12.3 A-3'), P('CK', 'COMMITMENT-RESIDUAL'), P('AMD', '§3 P9-RETAINED-CHARGE')], arg=arg,
        if_admitted={'variant': 'CommitmentAmount', 'committed': nano('300000000'), 'derivation': 'max(0, 1000000000 - 700000000)', 'kind': 'at_least'},
        pos=['admitted-branch committed 300000000'], neg=[('committed 500000000 with high 500000000 (coherent P9 mutant)', 'R8-FIN-01')], findings=[('R8-FIN-01', 'direct')]))
    cl.append(C(s, 'G4', G, {'evaluation': 'default', 'objective': 'o'}, DN, [s + '/VA2', s + '/G2', case.tok('/evaluations/0/objectives/0/predecessor')],
        [P('CONTRACT', '§12.1 status/provisional'), P('CK', 'R-FINANCIAL-PROVISIONAL rev2'), P('CONTRACT', '§12.2')], arg=arg,
        expected={'variant': 'ObjectiveStanding', 'status': 'not_established', 'reasons_must_include': ['UnenforcedCommitment', 'UnresolvedExposure'], 'provisional': True,
                  'predecessor_obligations': {'kind': 'exact', 'nano': '0'}, 'exact_reason_set': 'not asserted (K/§9 code catalogue is not present in canonical ck.json)'},
        pos=['not_established provisional true'], neg=[('provisional false with AtLeast owned line', 'R-FINANCIAL-PROVISIONAL')]))
    cl.append(C(s, 'G5', G, {'evaluation': 'default', 'admission_request': 'o'}, DN, [case.tok('/evaluations/0/admission_request/new', '100000000'), case.tok('/evaluations/0/objectives/0/limit', '1000000000'), s + '/VA2', s + '/G2'],
        [P('CONTRACT', '§12.4 steps 1-3')], arg=arg,
        expected={'variant': 'AdmissionDecision', 'objective': 'o', 'new': nano('100000000'), 'limit': nano('1000000000'), 'decision': 'refused', 'step': 'known_over_limit',
                  'invariant': 'admitted branch known=0+700000000+0+300000000+100000000=1100000000; unadmitted branch known>=1200000000; both >1000000000.'},
        pos=['refused'], neg=[('admit/not_established with unchanged premises', '§12.4 step 3')]))
    cl.append(C(s, 'CO1', CO, {'case': cid}, DN, [case.tok('/evaluations')],
        [P('CK', 'CONTINUITY rev1'), P('CK', 'ATOMIZATION rev1'), P('CK', 'INV-F7'), P('BASE', '§6 continuity; §8 H2/F1/C1/F3')], arg=arg,
        expected={'variant': 'FiniteContinuity', 'continuity_applicable': False, 'reason': 'no predecessor accounting subject or revision lineage in the raw premise', 'cross_evaluation_equalities': [],
                  'order_invariance': 'final-fact function: R-first, S-first and all-at-once over these two origins converge to the PM2/VA3 final; no route lowers the 700000000 charge'},
        pos=['inapplicable continuity, final-fact invariance'], neg=[('S-first route reported with permanent lower high', 'INV-F3 / ATOMIZATION final-fact')]))
    sp = [SC]
    scopes = [
        scope(cid, 'default', 'o', 'owned_closed', RU('Which line holds the usage unit depends on admitted scope (admitted: outstanding; unadmitted: orphan none-obligation in closed)', 'root', sp), sp, [P('CK', 'SUPPORTED-LOW'), P('CONTRACT', '§12.1 rollup lines')],
              if_admitted={'low': EMPTY_LOW, 'high': EMPTY_HIGH}, arg=arg),
        scope(cid, 'default', 'o', 'owned_outstanding', RU('as above', 'root', sp), sp, [P('CK', 'SUPPORTED-LOW'), P('BASE', '§5'), P('CONTRACT', '§12.1')],
              if_admitted={'low': LOW([[o1, o2]], [o2], [o2], '15000000', '15000000', 'single_ambiguous', 'usage_valuation'), 'high': HIGH('at_least', '700000000', {'proxy_charge': 1})}, arg=arg),
    ]
    kinds = {o1: 'Reservation', o2: 'Settlement(request)'}
    rec = header(case, '')
    rec.update({'causal_admission': causal_admission(case, [x['scope_id'] for x in scopes]), 'origins': case.origins_record(kinds), 'premises': prem,
                'claims': cl, 'family_status': family_status(cl), 'scope_lows': scopes,
                'excluded_channels': [{'channel': 'K attempt finding codes', 'reason': 'Not one of the seven families; canonical vocabulary only partially present (UnresolvedExposure, UnenforcedCommitment, NoGateEvidence).'}],
                'reuse': [{'source': 'subject:oracle/r9-pilot/claims.json P9-RETAINED-CHARGE', 'what': '400/500/700/15M components with origin refs', 'verified_how': 'independently re-derived from raw lexemes; origin raw_bytes_sha256 equal to sourcerefs'},
                          {'source': 'subject:oracle/R8-RAW-DERIVATIONS.md#P9', 'what': 'usage 15M local low', 'verified_how': 're-derived with pinned rates'}]})
    write(cid, rec)


# ---------- shared helpers for the remaining dollar cases ----------
def std_E(case, s, kinds, arg, usage_origin=None, extra_refs=()):
    origins = list(case.lines)
    cl = [C(s, 'E1', E, {'origins': origins}, DN, [case.line(o) for o in origins] + [case.f(o, '/event') for o in origins],
            [P('CK', 'EVIDENCE E-1, E-2'), P('BASE', '§2 Origin identity')], arg=arg,
            expected={'variant': 'RawOriginInventory', 'origin_count': len(origins),
                      'origins': [{'origin_id': o, 'original_position': int(o.split(':')[1]), 'evidence_kind': kinds[o], 'raw_utf8_sha256': case.lines[o]['raw_utf8_sha256']} for o in origins],
                      'unreadable': [], 'non_attempt': [], 'final_newline': True},
            pos=['accept exact origin inventory'], neg=COMMON_E_NEG)]
    res = [o for o in origins if kinds[o] == 'Reservation']
    sett = [o for o in origins if kinds[o].startswith('Settlement')]
    cl.append(C(s, 'E2', E, {'origins': origins}, DN, [case.line(o) for o in origins],
            [P('CK', 'LOGICAL-PRIMARY rev2'), P('AMD', '§2 primary_memberships evidence classes')], arg=arg,
            expected={'variant': 'EvidenceClassCardinality', 'reservation_classes': [[o] for o in res], 'settlement_classes': [[o] for o in sett],
                      'reservation_class_count': len(res), 'settlement_class_count': len(sett), 'exact_copy_classes': [], 'byte_identical_pairs': []},
            pos=['accept class counts'], neg=[('add/merge/delete an evidence class with matching validity refs', 'R8-RESERVATION-01 class cardinality')],
            findings=[('R8-RESERVATION-01', 'bears_on')]))
    return cl

def std_NS(case, s, cid, origins, declared, arg, decisive=True):
    SC = '%s/PREM-SCOPE' % s
    rid_refs = [case.f(o, '/rid', '1') if '/rid' in case.lines[o]['fields'] else case.f(o, '/usd/rid', '1') for o in origins]
    cl = [C(s, 'NS1', NS, {'origins': origins}, DN, rid_refs + [case.tok('/inputs/sessions/0/launch/auth_lane', 'api')],
            [P('BASE', '§3 C base IDs'), P('AMD', '§3 NAMESPACE-APPLICATION')], arg=arg,
            expected={'variant': 'RawReferenceNamespace', 'per_origin': {o: {'namespace': 'dollar_ticket', 'rid': '1', 'usable': True} for o in origins}, 'admission_namespace_origins': []},
            pos=['dollar_ticket applies'], neg=[('declare reference_namespace=admission for these dollar origins', 'NAMESPACE-APPLICATION mismatch')]),
          C(s, 'NS2', NS, {'declared_contract': declared}, DN, [case.line(o) for o in origins],
            [P('CK', 'INITIAL-REFERENCE rev2'), P('R3', 'Exact applicability')], arg=arg,
            expected={'variant': 'DeclaredPayloadRawConsistency', 'declared': {'reference_namespace': 'dollar_ticket', 'duplicate_rule': 'same_origin_only', 'covered_origin_refs': origins},
                      'raw_consistent': True, 'mismatches': [], 'establishes_admission': False},
            pos=['declared payload consistent with raw'], neg=[('stale extent/dropped covered origin/foreign fragment', 'INITIAL-REFERENCE supplied-payload binding')]),
          C(s, 'NS3', NS, {'source': 'S1/turn-1'}, RU('Per-case conditional initial_reference_scope admission absent' + ('' if decisive else ' (non-decisive here: single origin, identical hold unit under TicketClaim or OriginBase)'), 'root', [SC]), [SC],
            [P('BASE', '§2-§3'), P('FRPLAN', 'Finite truth and applicability interfaces')], arg=arg,
            if_admitted={'variant': 'NamespaceApplication', 'initial_base': {'kind': 'TicketClaim', 'scope': cid + '/S1/turn-1', 'rid': '1', 'origins': origins}, 'atomize': False, 'break_copies': False, 'hold_units': [origins]},
            pos=['root-admitted exact conditional declaration'], neg=[('admission namespace applied to dollar origins', 'NAMESPACE-APPLICATION')])]
    return cl

def std_T(case, s, arg, retire_at, rows_at, ev_obj_pairs, placement, owner='o', extra_place_refs=(), window=None, findings=()):
    cl = [lifecycle_claim(case, s, retire_at, rows_at, arg)]
    origins = list(case.lines)
    ev, oi = ev_obj_pairs[0]
    cl.append(C(s, 'T2', T, {'origins': origins, 'evaluations': sorted({e for e, _ in ev_obj_pairs})}, DN,
        [case.tok('/evaluations/%d/objectives/%d/admissions/0/rows/1/event' % (ev, oi), 'launched'), case.tok('/evaluations/%d/objectives/%d/admissions/0/rows/1/session_id' % (ev, oi), '00000000-0000-4000-8000-0000000000c1'),
         case.tok('/inputs/sessions/0/session_id', '00000000-0000-4000-8000-0000000000c1')],
        [P('CONTRACT', '§11.1 O-1, O-3a, O-7')], arg=arg,
        expected={'variant': 'Ownership', 'per_origin': {o: {'ownership': 'owned', 'objective': owner, 'admission': 'a1 journal launched row'} for o in origins},
                  'other_candidates': [], 'note': 'other objectives (if any) have no admission naming S1 and do not contain S1 in tree_sessions'},
        pos=['owned by %s via a1' % owner], neg=[('ownership uncertain or successor-owned with unchanged journal/tree', 'O-1/O-3a/O-7')]))
    cl.append(C(s, 'T3', T, {'origins': origins, 'objective': owner}, DN, list(extra_place_refs) + [case.f(o, '/at') for o in origins],
        [P('CONTRACT', '§11.2 P-1..P-5'), P('CONTRACT', '§12.4 step 1 closure establishment')], arg=arg,
        expected={'variant': 'Placement', 'window': window, 'per_origin': {o: placement for o in origins}}, pos=['placement %s' % placement],
        neg=[('supplied placement inside/before with matching reasons under unchanged raw interval', 'R8-PLACEMENT-01 raw placement')], findings=findings))
    return cl

def finish(case, s, prem, cl, scopes, kinds, reuse, extra_excl=()):
    rec = header(case, '')
    rec.update({'causal_admission': causal_admission(case, [x['scope_id'] for x in scopes]), 'origins': case.origins_record(kinds), 'premises': prem,
                'claims': cl, 'family_status': family_status(cl), 'scope_lows': scopes,
                'excluded_channels': [{'channel': 'K attempt finding codes', 'reason': 'Not one of the seven families; canonical vocabulary only partially present.'}] + list(extra_excl),
                'reuse': reuse})
    write(case.cid, rec)

def a06():
    case = Case('A06-reservation-usage-null-status-no-id'); s = 'A06'; cid = case.cid; arg = 'DERIVATIONS-BATCH1.md#a06'
    o1, o2 = 'S1/turn-1:1', 'S1/turn-1:2'; SC = s + '/PREM-SCOPE'
    prem = [scope_premise(case, s, [o1, o2], 'oracle/r8-completion-B/%s.provenance.json#S1/turn-1' % cid, 'conditional-ticket:S1/turn-1', 'initial-scope:S1/turn-1', 'ROOT-ADOPTION-R6.json'), RATES_PREMISE(case, s)]
    kinds = {o1: 'Reservation', o2: 'Settlement(request)'}
    cl = std_E(case, s, kinds, arg)
    cl.append(C(s, 'E3', E, {'origin': o2}, DN, [case.tok('/inputs/sessions/0/launch/model', 'openai/gpt-6-astra'), case.f(o2, '/model', 'gpt-6-astra'), case.f(o2, '/response_model', 'gpt-6-astra'),
        case.f(o2, '/response_id', 'null'), case.f(o2, '/http_status', 'null'), case.f(o2, '/terminal', 'null'), case.f(o2, '/observed_tier', 'default')],
        [P('CK', 'EVIDENCE MC-1..MC-3; E-2 fidelity')], arg=arg,
        expected={'variant': 'IdentityStates', 'canonical_identity_set': [{'family': 'openai', 'model_id': 'gpt-6-astra'}], 'malformed_identities': [],
                  'null_fields': ['/http_status', '/terminal', '/response_id'], 'note': 'null is preserved as null, distinct from absent and from integer 200'},
        pos=['accept null states'], neg=[('present http_status 200 / terminal response.completed substituted for null', 'E-2 fidelity; V-5')]))
    cl += std_NS(case, s, cid, [o1, o2], 'conditional-ticket:S1/turn-1', arg)
    cl.append(C(s, 'PM1', PM, {'origins': [o1, o2]}, DN, [case.f(o1, '/n', '1'), case.f(o1, '/reserved_usd', '0.25'), case.f(o2, '/n', '1'), case.f(o2, '/usd/charged_usd', '0.25'),
        case.f(o2, '/usd/reserved_usd', '0.25'), case.f(o2, '/usd/charge_basis', 'reservation'), case.f(o2, '/http_status', 'null')],
        [P('CK', 'SETTLEMENT §5 conditions 3,4,5,7')], arg=arg,
        expected={'variant': 'OriginLocalChecks', 'reservation': {o1: {'n': 'positive_integer', 'reserved_nano': '250000000'}},
                  'settlement': {o2: {'n': 'positive_integer', 'charged_nano': '250000000', 'claimed_reserved_nano': '250000000', 'basis': 'reservation', 'status': 'null_on_request_row'}},
                  'local_failures': ['StatusType']},
        pos=['StatusType local failure'], neg=[('status accepted (no StatusType) with null http_status', 'SETTLEMENT cond.7 Reservation path on request row requires integer')]))
    cl.append(C(s, 'PM2', PM, {'obligation': 'res:S1/turn-1:rid=1'}, RU('Logical R/S grouping requires admitted TicketClaim scope', 'root', [SC]), [SC],
        [P('CK', 'SETTLEMENT §5'), P('CK', 'TEMPORAL-AND-FINALIZATION §6, §6.1, §6.2'), P('CK', 'F-REPLACEMENT-2')], arg=arg,
        if_admitted={'variant': 'PrimaryCoverage', 'primary_membership': {'obligation_id': 'res:S1/turn-1:rid=1', 'reservation_classes': [{'origin_refs': [o1], 'representative_origin_ref': o1}], 'settlement_classes': [{'origin_refs': [o2], 'representative_origin_ref': o2}], 'usage_origin_refs': [o2]},
                     'settlement_validity': {'decision': 'rejected', 'reasons': ['StatusType']}, 'temporal_obligation': 'outstanding', 'obligation': {'state': 'outstanding', 'amount_source': 'reservation_bound'},
                     'replacement': {'eligible': False, 'reason': 'NotUsageSettlement'}, 'execution': 'terminal', 'forwarding': 'forwarded', 'accepted_coverage': [], 'definite_links': []},
        pos=['admitted-branch rejected StatusType'], neg=[('coherent Valid/closed output', 'SETTLEMENT cond.7')], findings=[('R8-OWNER-01', 'bears_on')]))
    cl.append(C(s, 'VA1', VA, {'usage_origin': o2}, DN, [case.f(o2, '/usage/input_tokens', '1000'), case.f(o2, '/usage/output_tokens', '100'), case.f(o2, '/usage/cached_tokens', '0'), case.f(o2, '/usage/cache_write_tokens', '0'), case.f(o2, '/observed_tier', 'default'), s + '/PREM-RATES'],
        [P('CK', 'VALUATION V-1..V-5'), P('CK', 'PRICE-OPENAI'), P('BASE', '§5 L4 table')], arg=arg,
        expected={'variant': 'ObservationValuation', 'origin': o2, 'state': 'valued', 'identities': [{'family': 'openai', 'model_id': 'gpt-6-astra'}], 'tier_set': ['default'], 'splits': {'cached': '0', 'write': '0'},
                  'complete': False, 'incomplete_because': ['http_status null', 'terminal null'], 'low': nano('15000000'), 'high': nano('15000000'), 'derivation': '1000*10000 + 100*50000 = 15000000 nano'},
        pos=['valued incomplete 15000000'], neg=[('low forced 0 because settlement rejected (L4)', 'SUPPORTED-LOW / X local low survives rejection'), ('complete true', 'V-5')], findings=[('R8-LOW-INVENTORY-01', 'bears_on')]))
    cl.append(C(s, 'VA2', VA, {'origins': [o1, o2]}, DN, [case.f(o1, '/reserved_usd', '0.25'), case.f(o2, '/usd/charged_usd', '0.25'), s + '/VA1'],
        [P('BASE', '§3 components follow origin'), P('CK', 'MONEY FB-2'), P('CK', 'WHOLE-COVERAGE rev3')], arg=arg,
        expected={'variant': 'RetainedComponents', 'components': [{'kind': 'reservation', 'origin_ref': o1, 'nano': '250000000', 'role': 'high_component'}, {'kind': 'claimed_reserved', 'origin_ref': o2, 'nano': '250000000', 'role': 'validity_premise_only_not_a_high_component'},
            {'kind': 'charge', 'origin_ref': o2, 'nano': '250000000', 'role': 'high_component'}, {'kind': 'usage_high', 'origin_ref': o2, 'nano': '15000000', 'role': 'high_component'}],
            'invariant': 'no lawful branch releases the 250000000 reservation/charge (no eligible usage bound exists: incomplete usage)'},
        pos=['retained 250/250/15M'], neg=[('high reduced to 15000000 exact', 'FB-1 no usage bound')], findings=[('R8-FIN-01', 'bears_on')]))
    cl.append(C(s, 'VA3', VA, {'unit': 'res:S1/turn-1:rid=1'}, RU('Unit structure depends on admitted scope', 'root', [SC]), [SC, s + '/VA2'],
        [P('CK', 'X-DECISION X-BRANCH-025'), P('CK', 'BASIS-TIE')], arg=arg,
        if_admitted={'variant': 'UnitExposure', 'unit_origins': [o1, o2], 'pool': 'pi_api', 'components': {'reservation': '250000000', 'charge': '250000000', 'usage_high': '15000000'},
                     'high': {'kind': 'at_least', 'nano': '250000000', 'basis': 'proxy_reservation'}, 'low': {'nano': '15000000', 'basis': 'usage_valuation'}, 'billing': 'unresolved', 'blocking_must_include': ['UnresolvedExposure']},
        pos=['AtLeast250000000 proxy_reservation (tie), low 15000000'], neg=[('basis proxy_charge on exact tie', 'BASIS-TIE')]))
    cl += std_T(case, s, arg, '2026-11-10T06:50:00Z', [(o1, '2026-11-10T06:30:00Z'), (o2, '2026-11-10T06:30:02Z')], [(0, 0)], 'inside',
                extra_place_refs=[case.tok('/evaluations/0/objectives/0/started_at', '2026-11-10T06:00:00Z'), case.tok('/evaluations/0/objectives/0/closed_at')],
                window={'start': '2026-11-10T06:00:00Z', 'end': 'open', 'closure': 'no_claim'})
    cl.append(C(s, 'G1', G, {'admission': 'a1'}, DN, [case.tok('/profile', 'current_producer'), case.absent('/inputs/gates'), case.absent('/trusted_fixture_inputs')], [P('CONTRACT', '§2.3; Revision6')], arg=arg,
        expected={'variant': 'GateQualification', 'qualified': False, 'reasons': ['NoGateEvidence'], 'provenance': 'none'}, pos=['NoGateEvidence'], neg=[('qualified true', 'Revision6 trust boundary')]))
    cl.append(C(s, 'G2', G, {'admission': 'a1'}, DN, [case.tok('/evaluations/0/objectives/0/admissions/0/rows/0/max_usd', '1.0'), s + '/T1', s + '/G1'], [P('CONTRACT', '§12.3 A-1, A-3, A-5')], arg=arg,
        expected={'variant': 'CommitmentState', 'admission_id': 'a1', 'cap': nano('1000000000'), 'state': 'retired_resumable', 'kind': 'at_least', 'blocking_must_include': ['UnenforcedCommitment']}, pos=['retired_resumable'], neg=[('retired/exact', 'A-3/L-7')]))
    cl.append(C(s, 'G3', G, {'admission': 'a1', 'field': 'committed'}, RU('depends on owned high (unit structure)', 'root', [SC]), [SC, s + '/VA3'], [P('CONTRACT', '§12.3 A-3')], arg=arg,
        if_admitted={'variant': 'CommitmentAmount', 'committed': nano('750000000'), 'derivation': 'max(0, 1000000000 - 250000000)', 'kind': 'at_least'}, pos=['750000000'], neg=[('committed 985000000 (high 15M)', 'A-3 with retained 250M')]))
    cl.append(C(s, 'G4', G, {'evaluation': 'default', 'objective': 'o'}, DN, [s + '/VA2', s + '/G2'], [P('CONTRACT', '§12.1'), P('CK', 'R-FINANCIAL-PROVISIONAL rev2')], arg=arg,
        expected={'variant': 'ObjectiveStanding', 'status': 'not_established', 'reasons_must_include': ['UnenforcedCommitment', 'UnresolvedExposure'], 'provisional': True, 'predecessor_obligations': {'kind': 'exact', 'nano': '0'}},
        pos=['not_established provisional'], neg=[('provisional false', 'R-FINANCIAL-PROVISIONAL')]))
    cl.append(C(s, 'G5', G, {'evaluation': 'default', 'admission_request': 'o'}, DN, [case.tok('/evaluations/0/admission_request/new', '100000000'), case.tok('/evaluations/0/objectives/0/limit', '1000000000'), s + '/G2'], [P('CONTRACT', '§12.4')], arg=arg,
        expected={'variant': 'AdmissionDecision', 'objective': 'o', 'decision': 'refused', 'step': 'known_over_limit', 'invariant': 'admitted 250+750+100=1100M; unadmitted 500+500+100=1100M; both > 1000M'},
        pos=['refused'], neg=[('admit', '§12.4 step 3')]))
    cl.append(C(s, 'CO1', CO, {'case': cid}, DN, [case.tok('/evaluations')], [P('CK', 'CONTINUITY rev1'), P('CK', 'ATOMIZATION rev1')], arg=arg,
        expected={'variant': 'FiniteContinuity', 'continuity_applicable': False, 'reason': 'no predecessor accounting subject', 'cross_evaluation_equalities': [], 'order_invariance': 'final-fact function'},
        pos=['inapplicable'], neg=[('route-dependent final', 'final-fact invariance')]))
    sp = [SC]
    scopes = [scope(cid, 'default', 'o', 'owned_closed', RU('usage unit line depends on admitted scope', 'root', sp), sp, [P('CK', 'SUPPORTED-LOW')], if_admitted={'low': EMPTY_LOW, 'high': EMPTY_HIGH}, arg=arg),
              scope(cid, 'default', 'o', 'owned_outstanding', RU('usage unit line depends on admitted scope', 'root', sp), sp, [P('CK', 'SUPPORTED-LOW'), P('BASE', '§5 L4')],
                    if_admitted={'low': LOW([[o1, o2]], [o2], [o2], '15000000', '15000000', 'single_ambiguous', 'usage_valuation'), 'high': HIGH('at_least', '250000000', {'proxy_reservation': 1})}, arg=arg)]
    finish(case, s, prem, cl, scopes, kinds, [{'source': 'subject:oracle/R8-RAW-DERIVATIONS.md#A06', 'what': '15M supported local low survives rejection', 'verified_how': 're-derived from raw counts and pinned rates'}])

def v01():
    case = Case('V01-usage-settlement-other-response-model'); s = 'V01'; cid = case.cid; arg = 'DERIVATIONS-BATCH1.md#v01'
    o1, o2 = 'S1/turn-1:1', 'S1/turn-1:2'; SC = s + '/PREM-SCOPE'
    prem = [scope_premise(case, s, [o1, o2], 'oracle/r8-completion-B/%s.provenance.json#S1/turn-1' % cid, 'conditional-ticket:S1/turn-1', 'initial-scope:S1/turn-1', 'ROOT-ADOPTION-R6.json'), RATES_PREMISE(case, s)]
    kinds = {o1: 'Reservation', o2: 'Settlement(request)'}
    cl = std_E(case, s, kinds, arg)
    fable = {'family': 'anthropic', 'model_id': 'claude-fable-5-2'}; sonnet = {'family': 'anthropic', 'model_id': 'claude-sonnet-5'}
    cl.append(C(s, 'E3', E, {'origin': o2}, DN, [case.tok('/inputs/sessions/0/launch/model', 'anthropic/claude-fable-5-2'), case.f(o2, '/provider', 'anthropic'), case.f(o2, '/model', 'claude-fable-5-2'),
        case.f(o2, '/response_model', 'claude-sonnet-5'), case.f(o2, '/response_model_check', 'verified'), case.f(o2, '/model_mismatch', 'true'), case.f(o2, '/observed_tier', 'standard')],
        [P('CK', 'EVIDENCE MC-1..MC-3'), P('CK', 'VALUATION V-1')], arg=arg,
        expected={'variant': 'IdentityStates', 'pinned': fable, 'canonical_identity_set': [fable, sonnet], 'malformed_identities': [], 'all_identities_priced': True,
                  'note': 'response_model_check/model_mismatch are retained raw labels; identity set comes from MC-1..MC-3 only'},
        pos=['two priced identities'], neg=[('drop claude-sonnet-5 from identity set (narrowing)', 'V-1 / FINANCIAL-CANDIDATES no narrowing'), ('treat set as unpriceable', 'R8-IDENTITY-01 priced multiidentity remains finite')],
        findings=[('R8-IDENTITY-01', 'direct')]))
    cl += std_NS(case, s, cid, [o1, o2], 'conditional-ticket:S1/turn-1', arg)
    cl.append(C(s, 'PM1', PM, {'origins': [o1, o2]}, DN, [case.f(o1, '/n', '1'), case.f(o1, '/reserved_usd', '0.3'), case.f(o2, '/n', '1'), case.f(o2, '/usd/charged_usd', '0.08'), case.f(o2, '/usd/reserved_usd', '0.3'),
        case.f(o2, '/usd/charge_basis', 'usage'), case.f(o2, '/http_status', '200'), s + '/PREM-RATES'],
        [P('CK', 'SETTLEMENT §5 conditions 3-8'), P('CK', 'TOLERANCE'), P('CK', 'VALUATION §7.1 producer_price')], arg=arg,
        expected={'variant': 'OriginLocalChecks', 'reservation': {o1: {'n': 'positive_integer', 'reserved_nano': '300000000'}},
                  'settlement': {o2: {'n': 'positive_integer', 'charged_nano': '80000000', 'claimed_reserved_nano': '300000000', 'basis': 'usage', 'status': 'integer_200', 'request_identity_equals_pinned': True,
                                      'producer_price_nano': '80000000', 'producer_price_basis': 'pinned claude-fable-5-2, standard tier', 'tolerance_range_ok': True, 'undercharge': False}},
                  'local_failures': []},
        pos=['all local checks pass'], neg=[('ModelMismatch from response_model', 'SETTLEMENT cond.8 compares request model to pinned')]))
    cl.append(C(s, 'PM2', PM, {'obligation': 'res:S1/turn-1:rid=1'}, RU('grouping and F coverage require admitted scope', 'root', [SC]), [SC],
        [P('CK', 'SETTLEMENT §5'), P('CK', 'F-REPLACEMENT-5'), P('CK', 'WHOLE-COVERAGE rev3'), P('CK', 'F-PROJECTION rev2'), P('BASE', '§6; §8 V01 example')], arg=arg,
        if_admitted={'variant': 'PrimaryCoverage', 'primary_membership': {'obligation_id': 'res:S1/turn-1:rid=1', 'reservation_classes': [{'origin_refs': [o1], 'representative_origin_ref': o1}], 'settlement_classes': [{'origin_refs': [o2], 'representative_origin_ref': o2}], 'usage_origin_refs': [o2]},
                     'settlement_validity': {'decision': 'valid', 'basis': 'usage'}, 'temporal_obligation': 'closed', 'obligation': {'state': 'closed', 'amount_source': 'settlement_amount', 'by': o2},
                     'replacement': {'eligible': True, 'reason': 'UsageBound'}, 'execution': 'terminal', 'forwarding': 'forwarded',
                     'accepted_coverage': [{'route': 'budgeted', 'obligation_id': 'res:S1/turn-1:rid=1', 'unit_origins': [o1, o2], 'monetary_origin_refs': [o1, o2], 'anchor_origin_refs': [], 'valuation_usage_origin_refs': [o2], 'definite_link_refs': [], 'retained_components': ['charge 80000000 @' + o2]}],
                     'definite_links': []},
        pos=['admitted-branch Valid/closed/eligible exact coverage'], neg=[('coverage valuation usage set swapped/omitted', 'R8-COVERAGE-01 exact application'), ('ineligible because of IdentityContradiction', 'USAGE-PRESENCE: priced multiidentity finite')],
        findings=[('R8-COVERAGE-01', 'bears_on'), ('R8-IDENTITY-01', 'direct')]))
    cl.append(C(s, 'VA1', VA, {'usage_origin': o2}, DN, [case.f(o2, '/usage/input_tokens', '10000'), case.f(o2, '/usage/cache_read_input_tokens', '0'), case.f(o2, '/usage/cache_creation_input_tokens', '0'), case.f(o2, '/usage/output_tokens', '0'),
        case.f(o2, '/usage/ephemeral_5m_input_tokens', '__absent__'), case.f(o2, '/usage/ephemeral_1h_input_tokens', '__absent__'), case.f(o2, '/terminal', 'message_stop'), case.f(o2, '/http_status', '200'), s + '/PREM-RATES'],
        [P('CK', 'VALUATION V-1..V-5'), P('CK', 'PRICE-ANTHROPIC'), P('CK', 'USAGE-PRESENCE'), P('RATES-ANTHROPIC', 'claude-fable-5-2 input 8; claude-sonnet-5 input 2')], arg=arg,
        expected={'variant': 'ObservationValuation', 'origin': o2, 'state': 'valued', 'identities': [fable, sonnet], 'tier_set': ['standard'], 'splits': {'w5m': '0 (absent=zero)', 'w1h': '0 (absent=zero)', 'cache_write': '0'},
                  'complete': True, 'low': nano('20000000'), 'high': nano('80000000'), 'derivation': 'sonnet 10000*2000=20000000; fable 10000*8000=80000000'},
        pos=['valued 20M-80M complete'], neg=[('unpriceable / low 80M / high 20M', 'V-2 range over identities')], findings=[('R8-IDENTITY-01', 'direct')]))
    cl.append(C(s, 'VA2', VA, {'origins': [o1, o2]}, DN, [case.f(o1, '/reserved_usd', '0.3'), case.f(o2, '/usd/charged_usd', '0.08'), s + '/VA1'], [P('BASE', '§3, §6'), P('CK', 'WHOLE-COVERAGE rev3')], arg=arg,
        expected={'variant': 'RetainedComponents', 'components': [{'kind': 'reservation', 'origin_ref': o1, 'nano': '300000000', 'role': 'high_component_until_exact_F_coverage'}, {'kind': 'charge', 'origin_ref': o2, 'nano': '80000000', 'role': 'high_component'},
            {'kind': 'usage_high', 'origin_ref': o2, 'nano': '80000000', 'role': 'high_component'}], 'invariant': 'charge 80000000 and priced high 80000000 retained in every branch; reservation released only by accepted exact F coverage'},
        pos=['components'], neg=[('high 20000000 (cheapest identity)', 'retained priced context high')]))
    cl.append(C(s, 'VA3', VA, {'unit': 'res:S1/turn-1:rid=1'}, RU('release requires admitted scope', 'root', [SC]), [SC, s + '/PM2'], [P('CK', 'X-DECISION X-BRANCH-023'), P('CK', 'BASIS-TIE'), P('CK', 'MONEY FB-1')], arg=arg,
        if_admitted={'variant': 'UnitExposure', 'unit_origins': [o1, o2], 'pool': 'pi_api', 'high': {'kind': 'exact', 'nano': '80000000', 'basis': 'usage_valuation'}, 'low': {'nano': '20000000', 'basis': 'usage_valuation'}, 'billing': 'established', 'blocking_must_include': ['IdentityContradiction']},
        pos=['Exact80000000 usage_valuation, low 20000000'], neg=[('AtLeast300000000 retained reservation despite eligible coverage', 'WHOLE-COVERAGE release'), ('Exact20000000', 'max over identities')]))
    cl += std_T(case, s, arg, '2026-11-10T06:50:00Z', [(o1, '2026-11-10T06:30:00Z'), (o2, '2026-11-10T06:30:01Z')], [(0, 0)], 'inside',
                extra_place_refs=[case.tok('/evaluations/0/objectives/0/started_at', '2026-11-10T06:00:00Z'), case.tok('/evaluations/0/objectives/0/closed_at')], window={'start': '2026-11-10T06:00:00Z', 'end': 'open', 'closure': 'no_claim'})
    cl.append(C(s, 'G1', G, {'admission': 'a1'}, DN, [case.tok('/profile', 'current_producer'), case.absent('/inputs/gates'), case.absent('/trusted_fixture_inputs')], [P('CONTRACT', '§2.3; Revision6')], arg=arg,
        expected={'variant': 'GateQualification', 'qualified': False, 'reasons': ['NoGateEvidence'], 'provenance': 'none'}, pos=['NoGateEvidence'], neg=[('qualified true', 'Revision6')]))
    cl.append(C(s, 'G2', G, {'admission': 'a1'}, DN, [case.tok('/evaluations/0/objectives/0/admissions/0/rows/0/max_usd', '1.0'), s + '/T1'], [P('CONTRACT', '§12.3 A-3, A-5')], arg=arg,
        expected={'variant': 'CommitmentState', 'admission_id': 'a1', 'cap': nano('1000000000'), 'state': 'retired_resumable', 'kind': 'at_least', 'blocking_must_include': ['UnenforcedCommitment']}, pos=['retired_resumable'], neg=[('retired exact', 'L-7')]))
    cl.append(C(s, 'G3', G, {'admission': 'a1', 'field': 'committed'}, RU('depends on owned high', 'root', [SC]), [SC, s + '/VA3'], [P('CONTRACT', '§12.3 A-3')], arg=arg,
        if_admitted={'variant': 'CommitmentAmount', 'committed': nano('920000000'), 'derivation': 'max(0, 1000000000 - 80000000)', 'kind': 'at_least'}, pos=['920000000'], neg=[('980000000 using 20M low', 'A-3 uses owned high')]))
    cl.append(C(s, 'G4', G, {'evaluation': 'default', 'objective': 'o'}, DN, [s + '/G2'], [P('CONTRACT', '§12.1'), P('CK', 'R-FINANCIAL-PROVISIONAL rev2')], arg=arg,
        expected={'variant': 'ObjectiveStanding', 'status': 'not_established', 'reasons_must_include': ['UnenforcedCommitment'], 'provisional': True, 'predecessor_obligations': {'kind': 'exact', 'nano': '0'},
                  'invariant': 'commitment is at_least in every branch, so provisional true and not_established'}, pos=['not_established provisional'], neg=[('established/provisional false because attempt Exact', 'A-5 / R-FINANCIAL-PROVISIONAL')]))
    cl.append(C(s, 'G5', G, {'evaluation': 'default', 'admission_request': 'o'}, DN, [case.tok('/evaluations/0/admission_request/new', '100000000'), case.tok('/evaluations/0/objectives/0/limit', '1000000000')], [P('CONTRACT', '§12.4')], arg=arg,
        expected={'variant': 'AdmissionDecision', 'objective': 'o', 'decision': 'refused', 'step': 'known_over_limit', 'invariant': 'admitted 80+920+100=1100M; unadmitted 380+620+100=1100M'}, pos=['refused'], neg=[('admit', '§12.4 step 3')]))
    cl.append(C(s, 'CO1', CO, {'case': cid}, DN, [case.tok('/evaluations')], [P('CK', 'CONTINUITY rev1'), P('BASE', '§8 M1')], arg=arg,
        expected={'variant': 'FiniteContinuity', 'continuity_applicable': False, 'reason': 'no predecessor accounting subject', 'cross_evaluation_equalities': [], 'order_invariance': 'final-fact function'}, pos=['inapplicable'], neg=[('route-dependent final', 'final-fact')]))
    sp = [SC]
    scopes = [scope(cid, 'default', 'o', 'owned_closed', DN, [s + '/VA1', s + '/T2'], [P('CK', 'SUPPORTED-LOW'), P('BASE', '§5')],
                    low=LOW(None, [o2], [o2], '20000000', '20000000', 'single_ambiguous', 'usage_valuation'), high_status=RU('high/owned units depend on admitted scope', 'root', sp),
                    if_admitted={'owned_unit_origin_sets': [[o1, o2]], 'high': HIGH('exact', '80000000', {'usage_valuation': 1})}, arg=arg,
                    note='Low invariant: the usage origin is in a closed-line unit in both branches (admitted: closed obligation; unadmitted: orphan none-obligation).'),
              scope(cid, 'default', 'o', 'owned_outstanding', DN, [s + '/VA1'], [P('CK', 'SUPPORTED-LOW')], low=LOW(None, [], [], '0', '0', 'none', 'none'),
                    high_status=RU('high depends on admitted scope (unadmitted: reservation-only AtLeast300000000)', 'root', sp), if_admitted={'owned_unit_origin_sets': [], 'high': EMPTY_HIGH}, arg=arg)]
    finish(case, s, prem, cl, scopes, kinds, [{'source': 'continuation:design/PLAN.md §8 V01 statement', 'what': 'Exact80M/low20M expectation', 'verified_how': 'policy statement checked against independently re-derived raw arithmetic and V/F clauses'}])

def gate_none(case, s, arg):
    return C(s, 'G1', G, {'admission': 'a1'}, DN, [case.tok('/profile', 'current_producer'), case.absent('/inputs/gates'), case.absent('/trusted_fixture_inputs')], [P('CONTRACT', '§2.3; Revision6')], arg=arg,
             expected={'variant': 'GateQualification', 'qualified': False, 'reasons': ['NoGateEvidence'], 'provenance': 'none'}, pos=['NoGateEvidence'], neg=[('qualified true from raw fields', 'Revision6 trust boundary')])

def p130r07():
    case = Case('P130-R07-closed-unresolved-predecessor-constrains-successor'); s = 'P130R07'; cid = case.cid; arg = 'DERIVATIONS-BATCH1.md#p130-r07'
    o1, o2 = 'S1/turn-1:1', 'S1/turn-1:2'; SC = s + '/PREM-SCOPE'
    prem = [scope_premise(case, s, [o1, o2], 'oracle/r8-completion-A/DERIVATIONS.json#%s/initial_reference_scope_payloads/scope:S1/turn-1' % cid, 'initial:S1/turn-1', 'scope:S1/turn-1', 'ROOT-POLICY-R6.md@3dbe5356')]
    kinds = {o1: 'Reservation', o2: 'Settlement(upstream_error)'}
    cl = std_E(case, s, kinds, arg)
    cl.append(C(s, 'E3', E, {'origins': [o1, o2]}, DN, [case.tok('/inputs/sessions/0/launch/model', 'openai/gpt-6-astra'), case.f(o2, '/model', '__absent__'), case.f(o2, '/usage', '__absent__'), case.f(o2, '/http_status', '__absent__'), case.f(o2, '/detail', 'connection reset')],
        [P('CK', 'EVIDENCE MC-1; E-2')], arg=arg,
        expected={'variant': 'IdentityStates', 'pinned': {'family': 'openai', 'model_id': 'gpt-6-astra'}, 'usage_observations': [], 'row_identity_fields': 'absent on both origins', 'canonical_identity_set': 'not applicable (no usage observation to value)'},
        pos=['no usage observation'], neg=[('invent a usage observation/valuation for the upstream_error', 'V-7/E-2 fidelity')]))
    cl += std_NS(case, s, cid, [o1, o2], 'initial:S1/turn-1', arg)
    cl.append(C(s, 'PM1', PM, {'origins': [o1, o2]}, DN, [case.f(o1, '/n', '1'), case.f(o1, '/reserved_usd', '0.5'), case.f(o2, '/n', '1'), case.f(o2, '/usd/charged_usd', '0.5'), case.f(o2, '/usd/reserved_usd', '0.5'), case.f(o2, '/usd/charge_basis', 'reservation'), case.f(o2, '/event', 'upstream_error')],
        [P('CK', 'SETTLEMENT §5 conditions 3-7 (upstream_error path)')], arg=arg,
        expected={'variant': 'OriginLocalChecks', 'reservation': {o1: {'n': 'positive_integer', 'reserved_nano': '500000000'}}, 'settlement': {o2: {'event': 'upstream_error', 'n': 'positive_integer', 'charged_nano': '500000000', 'claimed_reserved_nano': '500000000', 'basis': 'reservation', 'status': 'absent_accepted_on_upstream_error'}}, 'local_failures': []},
        pos=['local checks pass'], neg=[('StatusType for absent status on upstream_error', 'SETTLEMENT cond.7 table')]))
    cl.append(C(s, 'PM2', PM, {'obligation': 'res:S1/turn-1:rid=1'}, RU('grouping requires admitted scope', 'root', [SC]), [SC], [P('CK', 'SETTLEMENT §5 cond.6'), P('CK', 'TEMPORAL-AND-FINALIZATION'), P('CK', 'F-PROJECTION')], arg=arg,
        if_admitted={'variant': 'PrimaryCoverage', 'primary_membership': {'obligation_id': 'res:S1/turn-1:rid=1', 'reservation_classes': [{'origin_refs': [o1], 'representative_origin_ref': o1}], 'settlement_classes': [{'origin_refs': [o2], 'representative_origin_ref': o2}], 'usage_origin_refs': []},
                     'settlement_validity': {'decision': 'valid', 'basis': 'upstream_error(reservation)'}, 'temporal_obligation': 'closed', 'obligation': {'state': 'closed', 'amount_source': 'reservation_confirmed', 'by': o2},
                     'replacement': {'eligible': False, 'reason': 'NotUsageSettlement'}, 'execution': 'terminal', 'forwarding': 'unknown', 'accepted_coverage': [], 'definite_links': []},
        pos=['Valid upstream_error closed reservation_confirmed'], neg=[('settlement_amount / billing established', 'F13 / INV-F12')]))
    cl.append(C(s, 'VA1', VA, {'origins': [o1, o2]}, DN, [case.f(o2, '/usage', '__absent__')], [P('CK', 'V-PRESENCE-1'), P('CK', 'VALUATION')], arg=arg,
        expected={'variant': 'ObservationValuation', 'usage_observations': [], 'unit_valuation_state': 'not_reported', 'note': 'money identical under not_applicable (X-BRANCH-027 guard covers both)'},
        pos=['not_reported'], neg=[('valued 15M', 'no usage evidence')]))
    cl.append(C(s, 'VA2', VA, {'origins': [o1, o2]}, DN, [case.f(o1, '/reserved_usd', '0.5'), case.f(o2, '/usd/charged_usd', '0.5')], [P('CK', 'MONEY FB-2; §6.2 error exposure'), P('BASE', '§3')], arg=arg,
        expected={'variant': 'RetainedComponents', 'components': [{'kind': 'reservation', 'origin_ref': o1, 'nano': '500000000', 'role': 'high_component'}, {'kind': 'charge', 'origin_ref': o2, 'nano': '500000000', 'role': 'high_component'}],
                  'invariant': 'no usage bound exists, so every branch keeps at_least >= 500000000; execution completion does not establish billing'},
        pos=['500M retained'], neg=[('Exact(0) for upstream error', 'D4 nonbillable class empty')]))
    cl.append(C(s, 'VA3', VA, {'unit': 'res:S1/turn-1:rid=1'}, RU('unit structure depends on admitted scope', 'root', [SC]), [SC], [P('CK', 'X-DECISION X-BRANCH-027'), P('CK', 'BASIS-TIE')], arg=arg,
        if_admitted={'variant': 'UnitExposure', 'unit_origins': [o1, o2], 'pool': 'pi_api', 'high': {'kind': 'at_least', 'nano': '500000000', 'basis': 'proxy_reservation'}, 'low': {'nano': '0', 'basis': 'none'}, 'billing': 'unresolved', 'blocking_must_include': ['UnresolvedExposure']},
        pos=['AtLeast500000000'], neg=[('exact', 'FB-2')]))
    cl += std_T(case, s, arg, '2026-11-10T06:38:00Z', [(o1, '2026-11-10T06:30:00Z'), (o2, '2026-11-10T06:30:02Z')], [(0, 0)], 'inside', owner='p',
                extra_place_refs=[case.tok('/evaluations/0/objectives/0/started_at', '2026-11-10T06:00:00Z'), case.tok('/evaluations/0/objectives/0/closed_at', '2026-11-10T06:40:00Z'), case.absent('/evaluations/0/objectives/0/close'),
                                  case.tok('/evaluations/0/objectives/1/tree_sessions'), case.tok('/evaluations/0/objectives/1/admissions')],
                window={'start': '2026-11-10T06:00:00Z', 'end': 'open (closed_at 06:40 claimed without close entry: ClosureNotEstablished, P-1 treats window open)', 'closure': 'claimed_not_established'},
                findings=[('R8-PLACEMENT-01', 'bears_on')])
    cl.append(gate_none(case, s, arg))
    cl.append(C(s, 'G2', G, {'objective': 'p', 'admission': 'a1'}, DN, [case.tok('/evaluations/0/objectives/0/admissions/0/rows/0/max_usd', '1.0'), s + '/T1'], [P('CONTRACT', '§12.3 A-3, A-5')], arg=arg,
        expected={'variant': 'CommitmentState', 'admission_id': 'a1', 'cap': nano('1000000000'), 'state': 'retired_resumable', 'kind': 'at_least', 'blocking_must_include': ['UnenforcedCommitment']}, pos=['retired_resumable'], neg=[('retired', 'L-7')]))
    cl.append(C(s, 'G3', G, {'objective': 'p', 'admission': 'a1', 'field': 'committed'}, RU('depends on owned high', 'root', [SC]), [SC, s + '/VA3'], [P('CONTRACT', '§12.3 A-3')], arg=arg,
        if_admitted={'variant': 'CommitmentAmount', 'committed': nano('500000000'), 'derivation': 'max(0, 1000000000 - 500000000)', 'kind': 'at_least'}, pos=['500000000'], neg=[('0', 'A-3')]))
    cl.append(C(s, 'G4', G, {'evaluation': 'successor', 'objective': 'p'}, DN, [case.tok('/evaluations/0/objectives/0/closed_at', '2026-11-10T06:40:00Z'), case.absent('/evaluations/0/objectives/0/close'), s + '/G2', s + '/VA2'], [P('CONTRACT', '§12.1; §12.4 step 1'), P('CK', 'R-FINANCIAL-PROVISIONAL rev2')], arg=arg,
        expected={'variant': 'ObjectiveStanding', 'status': 'not_established', 'reasons_must_include': ['ClosureNotEstablished', 'UnenforcedCommitment', 'UnresolvedExposure'], 'provisional': True, 'closure': 'claimed_not_established', 'predecessor_obligations': {'kind': 'exact', 'nano': '0'}},
        pos=['p not_established'], neg=[('closure established without close entry', '§12.4 current_producer closure')]))
    cl.append(C(s, 'G5', G, {'evaluation': 'successor', 'objective': 's', 'field': 'predecessor_obligations'}, RU('sum of predecessor unresolved attempt highs and open commitments depends on unit structure', 'root', [SC]), [SC, s + '/G3', s + '/VA3'],
        [P('CONTRACT', '§12.2')], arg=arg,
        if_admitted={'variant': 'PredecessorObligations', 'kind': 'at_least', 'nano': '1000000000', 'terms': ['attempt unit res:S1/turn-1:rid=1 AtLeast500000000 (billing unresolved)', 'p commitment a1 retired_resumable AtLeast500000000']},
        pos=['AtLeast1000000000'], neg=[('exact 0 because p closed / attempt terminal', '§12.2 execution completion does not discharge')]))
    cl.append(C(s, 'G6', G, {'evaluation': 'successor', 'objective': 's'}, DN, [case.tok('/evaluations/0/objectives/1/predecessor', 'p'), case.tok('/evaluations/0/objectives/1/admissions'), case.tok('/evaluations/0/objectives/1/tree_sessions'), s + '/VA2'],
        [P('CONTRACT', '§12.1-§12.2'), P('CK', 'R-FINANCIAL-PROVISIONAL rev2')], arg=arg,
        expected={'variant': 'ObjectiveStanding', 'objective': 's', 'owned_closed': {'kind': 'exact', 'nano': '0'}, 'owned_outstanding': {'kind': 'exact', 'nano': '0'}, 'commitments': [], 'predecessor_obligations_kind': 'at_least',
                  'status': 'not_established', 'provisional': True, 'invariant': 'predecessor line is at_least in every branch'},
        pos=['s not_established provisional'], neg=[('s established/provisional false', '§12.2 predecessor constraint')]))
    cl.append(C(s, 'G7', G, {'evaluation': 'successor', 'admission_request': 's'}, DN, [case.tok('/evaluations/0/admission_request/objective', 's'), case.tok('/evaluations/0/admission_request/new', '100000000'), case.tok('/evaluations/0/objectives/1/limit', '1000000000'), case.tok('/evaluations/0/objectives/1/closed_at')],
        [P('CONTRACT', '§12.4')], arg=arg,
        expected={'variant': 'AdmissionDecision', 'objective': 's', 'decision': 'refused', 'step': 'known_over_limit', 'invariant': 'admitted known 0+0+1000000000+0+100000000=1100000000; unadmitted 1000000000+0+100000000; both > 1000000000'},
        pos=['refused'], neg=[('admit', '§12.4 step 3 counts predecessor line')]))
    cl.append(C(s, 'CO1', CO, {'case': cid}, DN, [case.tok('/evaluations')], [P('CK', 'CONTINUITY rev1')], arg=arg,
        expected={'variant': 'FiniteContinuity', 'continuity_applicable': False, 'reason': 'objective predecessor is a T/R §12.2 relation, not accounting-subject continuity; single evaluation', 'cross_evaluation_equalities': [], 'order_invariance': 'final-fact function'},
        pos=['inapplicable'], neg=[('route-dependent', 'final-fact')]))
    sp = [SC]
    scopes = [scope(cid, 'successor', 'p', 'owned_closed', DN, [s + '/VA1'], [P('CK', 'SUPPORTED-LOW')], low=LOW(None, [], [], '0', '0', 'none', 'none'), high_status=RU('high depends on admitted scope', 'root', sp),
                    if_admitted={'owned_unit_origin_sets': [[o1, o2]], 'high': HIGH('at_least', '500000000', {'proxy_reservation': 1})}, arg=arg, note='No usage observation exists in the case; every low is 0 in every branch.'),
              scope(cid, 'successor', 'p', 'owned_outstanding', DN, [s + '/VA1'], [P('CK', 'SUPPORTED-LOW')], low=LOW(None, [], [], '0', '0', 'none', 'none'), high_status=RU('high depends on admitted scope', 'root', sp),
                    if_admitted={'owned_unit_origin_sets': [], 'high': EMPTY_HIGH}, arg=arg),
              scope(cid, 'successor', 's', 'owned_closed', DN, [s + '/T2'], [P('CK', 'SUPPORTED-LOW')], low=EMPTY_LOW, high=EMPTY_HIGH, arg=arg),
              scope(cid, 'successor', 's', 'owned_outstanding', DN, [s + '/T2'], [P('CK', 'SUPPORTED-LOW')], low=EMPTY_LOW, high=EMPTY_HIGH, arg=arg)]
    finish(case, s, prem, cl, scopes, kinds, [])

def p132fg():
    case = Case('P132-FG-VC15-gated'); s = 'P132FG'; cid = case.cid; arg = 'DERIVATIONS-BATCH1.md#p132-fg'
    o1, o2 = 'S1/turn-1:1', 'S1/turn-1:2'; SC = s + '/PREM-SCOPE'
    prem = [scope_premise(case, s, [o1, o2], 'oracle/r8-completion-A/DERIVATIONS.json#%s/initial_reference_scope_payloads/scope:S1/turn-1' % cid, 'initial:S1/turn-1', 'scope:S1/turn-1', 'ROOT-POLICY-R6.md@3dbe5356'), RATES_PREMISE(case, s),
            {'premise_id': s + '/PREM-TESTMOCK', 'kind': 'trusted_fixture', 'statement': 'Top-level trusted_fixture_inputs.gate_verifications.g-S1 is a test-only harness verification (source test_mock) bound to the exact gate payload; admitted as a channel by CONTRACT Revision6 / POLICY-APPROVAL P5. Conditional policy proof only; no live gate qualification.',
             'status': DN, 'source_refs': [case.tok('/trusted_fixture_inputs/gate_verifications/g-S1/source', 'test_mock'), case.tok('/trusted_fixture_inputs/gate_verifications/g-S1/payload_sha256', 'b38088ca7e018f98b1775998d4ff4f5b7b1f027bfa1d5836b38479dcc75de868'), case.tok('/inputs/gates/0')],
             'policy_clauses': [P('CONTRACT', 'Revision6 gate trust boundary'), P('PAPPROVAL', 'P5'), P('EPR', 'trusted_fixture_inputs admission')]}]
    kinds = {o1: 'Reservation', o2: 'Settlement(request)'}
    cl = std_E(case, s, kinds, arg)
    cl.append(C(s, 'E3', E, {'origin': o2}, DN, [case.tok('/inputs/sessions/0/launch/model', 'openai/gpt-6-astra'), case.f(o2, '/model', 'gpt-6-astra'), case.f(o2, '/response_model', 'gpt-6-astra'), case.f(o2, '/observed_tier', 'default')], [P('CK', 'EVIDENCE MC-1..MC-3')], arg=arg,
        expected={'variant': 'IdentityStates', 'canonical_identity_set': [{'family': 'openai', 'model_id': 'gpt-6-astra'}], 'malformed_identities': []}, pos=['single identity'], neg=[('numeric response_model with eligible output', 'R8-IDENTITY-01')]))
    cl += std_NS(case, s, cid, [o1, o2], 'initial:S1/turn-1', arg)
    cl.append(C(s, 'PM1', PM, {'origins': [o1, o2]}, DN, [case.f(o1, '/n', '1'), case.f(o1, '/reserved_usd', '0.5'), case.f(o2, '/n', '1'), case.f(o2, '/usd/charged_usd', '0.015'), case.f(o2, '/usd/reserved_usd', '0.5'), case.f(o2, '/usd/charge_basis', 'usage'), case.f(o2, '/http_status', '200'), s + '/PREM-RATES'],
        [P('CK', 'SETTLEMENT §5'), P('CK', 'TOLERANCE')], arg=arg,
        expected={'variant': 'OriginLocalChecks', 'reservation': {o1: {'n': 'positive_integer', 'reserved_nano': '500000000'}}, 'settlement': {o2: {'n': 'positive_integer', 'charged_nano': '15000000', 'claimed_reserved_nano': '500000000', 'basis': 'usage', 'status': 'integer_200', 'request_identity_equals_pinned': True, 'producer_price_nano': '15000000', 'tolerance_range_ok': True, 'undercharge': False}}, 'local_failures': []},
        pos=['local checks pass'], neg=[('Undercharge flagged at 15000000', 'TOLERANCE')]))
    cl.append(C(s, 'PM2', PM, {'obligation': 'res:S1/turn-1:rid=1'}, RU('grouping/F coverage require admitted scope', 'root', [SC]), [SC], [P('CK', 'WHOLE-COVERAGE rev3'), P('CK', 'F-REPLACEMENT-5'), P('CK', 'F-PROJECTION')], arg=arg,
        if_admitted={'variant': 'PrimaryCoverage', 'primary_membership': {'obligation_id': 'res:S1/turn-1:rid=1', 'reservation_classes': [{'origin_refs': [o1], 'representative_origin_ref': o1}], 'settlement_classes': [{'origin_refs': [o2], 'representative_origin_ref': o2}], 'usage_origin_refs': [o2]},
                     'settlement_validity': {'decision': 'valid', 'basis': 'usage'}, 'temporal_obligation': 'closed', 'obligation': {'state': 'closed', 'amount_source': 'settlement_amount', 'by': o2}, 'replacement': {'eligible': True, 'reason': 'UsageBound'},
                     'execution': 'terminal', 'forwarding': 'forwarded', 'accepted_coverage': [{'route': 'budgeted', 'obligation_id': 'res:S1/turn-1:rid=1', 'unit_origins': [o1, o2], 'monetary_origin_refs': [o1, o2], 'anchor_origin_refs': [], 'valuation_usage_origin_refs': [o2], 'definite_link_refs': []}], 'definite_links': []},
        pos=['eligible exact coverage'], neg=[('coverage omitting the reservation origin', 'R8-COVERAGE-01')], findings=[('R8-COVERAGE-01', 'bears_on')]))
    cl.append(C(s, 'VA1', VA, {'usage_origin': o2}, DN, [case.f(o2, '/usage/input_tokens', '1000'), case.f(o2, '/usage/output_tokens', '100'), case.f(o2, '/usage/cached_tokens', '0'), case.f(o2, '/usage/cache_write_tokens', '0'), case.f(o2, '/terminal', 'response.completed'), s + '/PREM-RATES'],
        [P('CK', 'VALUATION V-1..V-5'), P('CK', 'PRICE-OPENAI')], arg=arg,
        expected={'variant': 'ObservationValuation', 'origin': o2, 'state': 'valued', 'identities': [{'family': 'openai', 'model_id': 'gpt-6-astra'}], 'tier_set': ['default'], 'complete': True, 'low': nano('15000000'), 'high': nano('15000000'), 'derivation': '1000*10000 + 100*50000'},
        pos=['valued 15M complete'], neg=[('incomplete', 'V-5')]))
    cl.append(C(s, 'VA2', VA, {'origins': [o1, o2]}, DN, [case.f(o1, '/reserved_usd', '0.5'), case.f(o2, '/usd/charged_usd', '0.015')], [P('BASE', '§3, §6')], arg=arg,
        expected={'variant': 'RetainedComponents', 'components': [{'kind': 'reservation', 'origin_ref': o1, 'nano': '500000000', 'role': 'high_component_until_exact_F_coverage'}, {'kind': 'charge', 'origin_ref': o2, 'nano': '15000000', 'role': 'high_component'}, {'kind': 'usage_high', 'origin_ref': o2, 'nano': '15000000', 'role': 'high_component'}]},
        pos=['components'], neg=[('charge dropped', 'WHOLE-COVERAGE retention')]))
    cl.append(C(s, 'VA3', VA, {'unit': 'res:S1/turn-1:rid=1'}, RU('release requires admitted scope', 'root', [SC]), [SC], [P('CK', 'X-DECISION X-BRANCH-023'), P('CK', 'BASIS-TIE')], arg=arg,
        if_admitted={'variant': 'UnitExposure', 'unit_origins': [o1, o2], 'pool': 'pi_api', 'high': {'kind': 'exact', 'nano': '15000000', 'basis': 'usage_valuation'}, 'low': {'nano': '15000000', 'basis': 'usage_valuation'}, 'billing': 'established'},
        pos=['Exact15000000'], neg=[('AtLeast500000000', 'release by eligible coverage')]))
    cl += std_T(case, s, arg, '2026-11-10T06:38:00Z', [(o1, '2026-11-10T06:30:00Z'), (o2, '2026-11-10T06:30:02Z')], [(0, 0)], 'inside', owner='p',
                extra_place_refs=[case.tok('/evaluations/0/objectives/0/closed_at', '2026-11-10T06:40:00Z'), case.absent('/evaluations/0/objectives/0/closure_record'), case.tok('/profile', 'future_qualified_gate')],
                window={'start': '2026-11-10T06:00:00Z', 'end': 'open (future profile closure needs authority closure_record; absent => ClosureNotEstablished)', 'closure': 'claimed_not_established'})
    gp = '/inputs/gates/0'
    cl.append(C(s, 'G1', G, {'gate': 'g-S1', 'admission': 'a1'}, DN,
        [case.tok('/evaluations/0/objectives/0/admission_authority/id', 'wev-admission-authority-1'), case.tok(gp + '/authority/id', 'wev-admission-authority-1'), case.tok('/trusted_fixture_inputs/gate_verifications/g-S1/authority_id', 'wev-admission-authority-1'),
         case.tok(gp + '/authority/kind', 'vertical_admission_authority'), case.tok(gp + '/writer', 'authority'), case.tok(gp + '/session', '00000000-0000-4000-8000-0000000000c1'), case.tok('/trusted_fixture_inputs/gate_verifications/g-S1/session', '00000000-0000-4000-8000-0000000000c1'),
         case.tok(gp + '/scope/kind', 'session'), case.tok(gp + '/effective_from', '2026-11-10T06:00:05Z'), case.tok(gp + '/effective_until'), case.tok('/evaluations/0/objectives/0/admissions/0/rows/1/at', '2026-11-10T06:00:10Z'),
         case.tok(gp + '/mechanisms/new_forwarding/enforced', 'true'), case.tok(gp + '/mechanisms/retries/enforced', 'true'), case.tok(gp + '/mechanisms/reopening/enforced', 'true'),
         case.tok(gp + '/mechanisms/new_forwarding/receipt/sha256'), case.tok(gp + '/mechanisms/retries/receipt/sha256'), case.tok(gp + '/mechanisms/reopening/receipt/sha256'),
         case.tok('/evaluations/0/objectives/0/admissions/0/gate_id', 'g-S1'), case.tok('/trusted_fixture_inputs/gate_verifications/g-S1/policy_ref', 'root-P5-conditional-only'), s + '/PREM-TESTMOCK'],
        [P('CONTRACT', '§2.3 G-1..G-7'), P('CONTRACT', 'Revision6 gate trust boundary'), P('FRPLAN', 'Finding matrix R8-GATE-IDENTITY-01'), P('AMD', '§3 GATE-AUTHORITY-IDENTITY')], arg=arg,
        expected={'variant': 'GateQualification', 'qualified': True, 'provenance': 'test_mock', 'policy_ref': 'root-P5-conditional-only',
                  'identity_triple': {'objective_authority_id': 'wev-admission-authority-1', 'gate_authority_id': 'wev-admission-authority-1', 'witness_authority_id': 'wev-admission-authority-1', 'each_present_nonempty_string': True, 'equal': True},
                  'payload_binding': {'payload_sha256': 'b38088ca7e018f98b1775998d4ff4f5b7b1f027bfa1d5836b38479dcc75de868', 'equals_sha256_of_canonical_json(inputs.gates[0])': True},
                  'rules': {'G1': 'pass', 'G2': 'pass', 'G3': 'pass', 'G4': 'pass (06:00:05 <= launched 06:00:10; until open)', 'G5_release': 'pass (new_forwarding, retries, reopening enforced)', 'G6': 'pass', 'G7': 'pass (no reopen)'},
                  'live_qualification': False},
        pos=['qualified true, test_mock'], neg=[('triple absent/empty with recomputed payload digest', 'R8-GATE-IDENTITY-01 nonempty identity'), ('witness authority mismatch', 'identity equality'), ('payload_sha256 stale after gate edit', 'exact payload binding'), ('raw verified:true without trusted_fixture_inputs', 'Revision6')],
        findings=[('R8-GATE-IDENTITY-01', 'direct')]))
    cl.append(C(s, 'G2', G, {'objective': 'p', 'admission': 'a1'}, DN, [case.tok('/evaluations/0/objectives/0/admissions/0/rows/0/max_usd', '1.0'), s + '/T1', s + '/G1'], [P('CONTRACT', '§12.3 A-3, A-5'), P('CK', 'COMMITMENT-RESIDUAL')], arg=arg,
        expected={'variant': 'CommitmentState', 'admission_id': 'a1', 'cap': nano('1000000000'), 'state': 'retired', 'committed': nano('0'), 'kind': 'exact', 'gate': {'qualified': True, 'provenance': 'test_mock'},
                  'invariant': 'release by qualified gate is independent of owned high'},
        pos=['retired exact 0 (test_mock)'], neg=[('retired_resumable with qualified gate', 'A-3 G-5'), ('gate release used to make attempt high exact', 'qualified gate never changes money')]))
    cl.append(C(s, 'G3', G, {'evaluation': 'successor', 'objective': 'p', 'field': 'status'}, DN, [case.tok('/evaluations/0/objectives/0/closed_at', '2026-11-10T06:40:00Z'), case.absent('/evaluations/0/objectives/0/closure_record')], [P('CONTRACT', '§12.1; §12.4 step 1 future_qualified_gate')], arg=arg,
        expected={'variant': 'ObjectiveStanding', 'objective': 'p', 'status': 'not_established', 'reasons_must_include': ['ClosureNotEstablished'], 'closure': 'claimed_not_established', 'predecessor_obligations': {'kind': 'exact', 'nano': '0'}},
        pos=['p not_established ClosureNotEstablished'], neg=[('closure established from legacy closed_at alone', '§12.4 future profile')]))
    cl.append(C(s, 'G4', G, {'evaluation': 'successor', 'objective': 'p', 'field': 'provisional'}, RU('owned high kind depends on admitted scope', 'root', [SC]), [SC, s + '/VA3', s + '/G2'], [P('CK', 'R-FINANCIAL-PROVISIONAL rev2')], arg=arg,
        if_admitted={'variant': 'Provisionality', 'objective': 'p', 'provisional': False}, pos=['false'], neg=[('true with all Exact/retired', 'R-FINANCIAL-PROVISIONAL')]))
    cl.append(C(s, 'G5', G, {'evaluation': 'successor', 'objective': 's'}, RU('predecessor line depends on billing of p-owned attempt (unit structure)', 'root', [SC]), [SC, s + '/G2', s + '/VA3'], [P('CONTRACT', '§12.1, §12.2, §12.4')], arg=arg,
        if_admitted={'variant': 'ObjectiveStandingAndAdmission', 'objective': 's', 'predecessor_obligations': {'kind': 'exact', 'nano': '0'}, 'owned_closed': {'kind': 'exact', 'nano': '0'}, 'owned_outstanding': {'kind': 'exact', 'nano': '0'}, 'commitments': [],
                     'status': 'established', 'provisional': False, 'admission': {'new': '100000000', 'limit': '1000000000', 'known': '100000000', 'decision': 'admit', 'conditional_on': 'test_mock gate (no live authority)'}},
        pos=['admit conditional'], neg=[('admit with predecessor AtLeast (gate residual clearing AtLeast)', 'COMMITMENT-RESIDUAL / G1 control')]))
    cl.append(C(s, 'CO1', CO, {'case': cid}, DN, [case.tok('/evaluations')], [P('CK', 'CONTINUITY rev1')], arg=arg,
        expected={'variant': 'FiniteContinuity', 'continuity_applicable': False, 'reason': 'single evaluation; predecessor is §12.2 relation', 'cross_evaluation_equalities': [], 'order_invariance': 'final-fact function'}, pos=['inapplicable'], neg=[('route-dependent', 'final-fact')]))
    sp = [SC]
    scopes = [scope(cid, 'successor', 'p', 'owned_closed', DN, [s + '/VA1', s + '/T2'], [P('CK', 'SUPPORTED-LOW')], low=LOW(None, [o2], [o2], '15000000', '15000000', 'single_ambiguous', 'usage_valuation'),
                    high_status=RU('high depends on admitted scope', 'root', sp), if_admitted={'owned_unit_origin_sets': [[o1, o2]], 'high': HIGH('exact', '15000000', {'usage_valuation': 1})}, arg=arg,
                    note='Low invariant: usage origin lies in a closed-line unit in both branches.'),
              scope(cid, 'successor', 'p', 'owned_outstanding', DN, [s + '/VA1'], [P('CK', 'SUPPORTED-LOW')], low=LOW(None, [], [], '0', '0', 'none', 'none'), high_status=RU('unadmitted: reservation-only AtLeast500000000', 'root', sp), if_admitted={'owned_unit_origin_sets': [], 'high': EMPTY_HIGH}, arg=arg),
              scope(cid, 'successor', 's', 'owned_closed', DN, [s + '/T2'], [P('CK', 'SUPPORTED-LOW')], low=EMPTY_LOW, high=EMPTY_HIGH, arg=arg),
              scope(cid, 'successor', 's', 'owned_outstanding', DN, [s + '/T2'], [P('CK', 'SUPPORTED-LOW')], low=EMPTY_LOW, high=EMPTY_HIGH, arg=arg)]
    finish(case, s, prem, cl, scopes, kinds, [])

def p132cp():
    case = Case('P132-CP-closed-predecessor-outstanding'); s = 'P132CP'; cid = case.cid; arg = 'DERIVATIONS-BATCH1.md#p132-cp'
    o1 = 'S1/turn-1:1'; SC = s + '/PREM-SCOPE'
    prem = [scope_premise(case, s, [o1], 'oracle/r8-completion-A/DERIVATIONS.json#%s/initial_reference_scope_payloads/scope:S1/turn-1' % cid, 'initial:S1/turn-1', 'scope:S1/turn-1', 'ROOT-POLICY-R6.md@3dbe5356')]
    kinds = {o1: 'Reservation'}
    cl = std_E(case, s, kinds, arg)
    cl.append(C(s, 'E3', E, {'origin': o1}, DN, [case.tok('/inputs/sessions/0/launch/model', 'openai/gpt-6-astra')], [P('CK', 'EVIDENCE MC-1')], arg=arg,
        expected={'variant': 'IdentityStates', 'pinned': {'family': 'openai', 'model_id': 'gpt-6-astra'}, 'usage_observations': []}, pos=['no usage'], neg=[('invented usage', 'E-2')]))
    cl += std_NS(case, s, cid, [o1], 'initial:S1/turn-1', arg, decisive=False)
    cl.append(C(s, 'PM1', PM, {'origin': o1}, DN, [case.f(o1, '/n', '1'), case.f(o1, '/reserved_usd', '0.5')], [P('CK', 'SETTLEMENT §5 cond.3-4'), P('CK', 'RESERVATION-MAX')], arg=arg,
        expected={'variant': 'OriginLocalChecks', 'reservation': {o1: {'n': 'positive_integer', 'reserved_nano': '500000000'}}, 'settlement': {}, 'local_failures': []}, pos=['ok'], neg=[('malformed reserved', 'MONEY producer amounts')]))
    cl.append(C(s, 'PM2', PM, {'obligation': 'res:S1/turn-1:rid=1'}, DN, [case.line(o1), s + '/E2'], [P('CK', 'SETTLEMENT §5; S-unique-zero'), P('CK', 'TEMPORAL-AND-FINALIZATION'), P('CK', 'F-REPLACEMENT-2'), P('CK', 'F-PROJECTION')], arg=arg,
        expected={'variant': 'PrimaryCoverage', 'primary_membership': {'obligation_id': 'res:S1/turn-1:rid=1', 'reservation_classes': [{'origin_refs': [o1], 'representative_origin_ref': o1}], 'settlement_classes': [], 'usage_origin_refs': []},
                  'settlement_validity': {'decision': 'absent'}, 'temporal_obligation': 'outstanding', 'obligation': {'state': 'outstanding', 'amount_source': 'reservation_bound'}, 'replacement': {'eligible': False, 'reason': 'NotUsageSettlement'},
                  'execution': 'not_terminal', 'forwarding': 'unknown', 'accepted_coverage': [], 'definite_links': [],
                  'invariant': 'single origin: identical group/hold under TicketClaim or OriginBase; only the base label depends on PREM-SCOPE'},
        pos=['absent/outstanding/reservation_bound'], neg=[('closed or released because p closed', '§12.2 / P-4')]))
    cl.append(C(s, 'VA1', VA, {'origin': o1}, DN, [case.line(o1)], [P('CK', 'V-PRESENCE-1')], arg=arg,
        expected={'variant': 'ObservationValuation', 'usage_observations': [], 'unit_valuation_state': 'not_reported', 'note': 'money identical under not_applicable (X-BRANCH-009)'}, pos=['not_reported'], neg=[('valued', 'no usage')]))
    cl.append(C(s, 'VA2', VA, {'origin': o1}, DN, [case.f(o1, '/reserved_usd', '0.5')], [P('CK', 'MONEY FB-2'), P('CK', 'RESERVATION-MAX')], arg=arg,
        expected={'variant': 'RetainedComponents', 'components': [{'kind': 'reservation', 'origin_ref': o1, 'nano': '500000000', 'role': 'high_component'}]}, pos=['500M'], neg=[('component dropped after closure', 'P-5 T modifies no amount')]))
    cl.append(C(s, 'VA3', VA, {'unit': 'res:S1/turn-1:rid=1'}, DN, [s + '/VA2', s + '/PM2'], [P('CK', 'X-DECISION X-BRANCH-009')], arg=arg,
        expected={'variant': 'UnitExposure', 'unit_origins': [o1], 'pool': 'pi_api', 'high': {'kind': 'at_least', 'nano': '500000000', 'basis': 'proxy_reservation'}, 'low': {'nano': '0', 'basis': 'none'}, 'billing': 'unresolved', 'blocking_must_include': ['UnresolvedExposure']},
        pos=['AtLeast500000000'], neg=[('exact 500000000', 'FB-2')]))
    win = {'start': '2026-11-10T06:00:00Z', 'end': '2026-11-10T06:40:00Z', 'closure': 'established (current_producer: close.at == closed_at, within [started_at, reference_time], no later admission row, single close entry)'}
    cl += std_T(case, s, arg, None, [(o1, '2026-11-10T06:30:00Z')], [(0, 0), (1, 0)], 'straddles_closure (interval [06:30:00, open), owner p retained by P-4; reason PlacementStraddles)', owner='p',
                extra_place_refs=[case.tok('/evaluations/%d/objectives/0/%s' % (e, k), v) for e in (0, 1) for k, v in (('closed_at', '2026-11-10T06:40:00Z'), ('close/at', '2026-11-10T06:40:00Z'), ('started_at', '2026-11-10T06:00:00Z'))]
                                 + [case.tok('/inputs/sessions/0/lifecycle'), case.tok('/inputs/reference_time', '2026-11-10T07:00:00Z')],
                window=win, findings=[('R8-PLACEMENT-01', 'direct')])
    cl.append(gate_none(case, s, arg))
    cl.append(C(s, 'G2', G, {'objective': 'p', 'admission': 'a1', 'evaluations': ['request-successor', 'request-predecessor']}, DN, [case.tok('/evaluations/0/objectives/0/admissions/0/rows/0/max_usd', '1.0'), case.tok('/evaluations/1/objectives/0/admissions/0/rows/0/max_usd', '1.0'), s + '/T1', s + '/VA3'],
        [P('CONTRACT', '§12.3 A-3, A-5')], arg=arg,
        expected={'variant': 'CommitmentState', 'admission_id': 'a1', 'cap': nano('1000000000'), 'state': 'launched_open', 'committed': nano('500000000'), 'derivation': 'max(0, 1000000000 - 500000000)', 'kind': 'at_least', 'blocking_must_include': ['UnenforcedCommitment']},
        pos=['launched_open AtLeast500000000'], neg=[('released/retired because objective closed', 'A-3 lifecycle state; closure does not release')]))
    cl.append(C(s, 'G3', G, {'objective': 'p'}, DN, [s + '/T3', s + '/G2', s + '/VA3'], [P('CONTRACT', '§12.1'), P('CK', 'R-FINANCIAL-PROVISIONAL rev2')], arg=arg,
        expected={'variant': 'ObjectiveStanding', 'objective': 'p', 'status': 'not_established', 'reasons_must_include': ['PlacementStraddles', 'UnenforcedCommitment', 'UnresolvedExposure'], 'provisional': True, 'closure': 'established', 'predecessor_obligations': {'kind': 'exact', 'nano': '0'}},
        pos=['p not_established'], neg=[('PlacementStraddles missing with placement inside', 'R8-PLACEMENT-01')], findings=[('R8-PLACEMENT-01', 'direct')]))
    cl.append(C(s, 'G4', G, {'objective': 's'}, DN, [case.tok('/evaluations/0/objectives/1/predecessor', 'p'), case.tok('/evaluations/0/objectives/1/limit', '2000000000'), s + '/G2', s + '/VA3'], [P('CONTRACT', '§12.2'), P('CK', 'R-FINANCIAL-PROVISIONAL rev2')], arg=arg,
        expected={'variant': 'ObjectiveStanding', 'objective': 's', 'predecessor_obligations': {'kind': 'at_least', 'nano': '1000000000', 'terms': ['attempt res:S1/turn-1:rid=1 AtLeast500000000 (outstanding)', 'p a1 launched_open AtLeast500000000']},
                  'owned_closed': {'kind': 'exact', 'nano': '0'}, 'owned_outstanding': {'kind': 'exact', 'nano': '0'}, 'commitments': [], 'status': 'not_established', 'provisional': True},
        pos=['s predecessor AtLeast1000000000'], neg=[('closure of p transfers spend to s owned lines', 'P-4/§12.2'), ('predecessor line exact 0 after closure', '§12.2')]))
    cl.append(C(s, 'G5', G, {'evaluation': 'request-successor', 'admission_request': 's'}, DN, [case.tok('/evaluations/0/admission_request/objective', 's'), case.tok('/evaluations/0/admission_request/new', '100000000'), case.tok('/evaluations/0/objectives/1/limit', '2000000000'), s + '/G4'],
        [P('CONTRACT', '§12.4 steps 1-4')], arg=arg,
        expected={'variant': 'AdmissionDecision', 'objective': 's', 'known': nano('1100000000'), 'limit': nano('2000000000'), 'decision': 'not_established', 'step': 'accounting (known <= limit; s not_established)'},
        pos=['not_established'], neg=[('admit with AtLeast predecessor line', '§12.4 step 4'), ('refused', 'known 1100000000 <= 2000000000')]))
    cl.append(C(s, 'G6', G, {'evaluation': 'request-predecessor', 'admission_request': 'p'}, DN, [case.tok('/evaluations/1/admission_request/objective', 'p'), case.tok('/evaluations/1/objectives/0/close/at', '2026-11-10T06:40:00Z')], [P('CONTRACT', '§12.4 step 1')], arg=arg,
        expected={'variant': 'AdmissionDecision', 'objective': 'p', 'decision': 'refused', 'step': 'closure_established'}, pos=['refused'], neg=[('not_established', 'established closure gives refused first')]))
    cl.append(C(s, 'CO1', CO, {'case': cid}, DN, [case.tok('/evaluations/0/objectives'), case.tok('/evaluations/1/objectives')], [P('CK', 'INV-F7'), P('CK', 'CONTINUITY rev1')], arg=arg,
        expected={'variant': 'FiniteContinuity', 'continuity_applicable': False, 'reason': 'no predecessor accounting subject',
                  'cross_evaluation_equalities': ['attempt C-X facts identical across request-successor and request-predecessor (INV-F7)', 'objective configurations identical across both evaluations; p/s rollups, commitments and placement identical; only admission_request differs'],
                  'order_invariance': 'single origin'},
        pos=['equal facts across evaluations'], neg=[('attempt facts differ between evaluations', 'INV-F7 window independence')]))
    scopes = []
    for ev in ('request-successor', 'request-predecessor'):
        scopes += [scope(cid, ev, 'p', 'owned_closed', DN, [s + '/PM2'], [P('CK', 'SUPPORTED-LOW'), P('CONTRACT', '§12.1')], low=EMPTY_LOW, high=EMPTY_HIGH, arg=arg),
                   scope(cid, ev, 'p', 'owned_outstanding', DN, [s + '/PM2', s + '/VA3', s + '/T2'], [P('CK', 'SUPPORTED-LOW'), P('CONTRACT', '§12.1 (owned summed whatever placement)')], low=LOW([[o1]], [], [], '0', '0', 'none', 'none'), high=HIGH('at_least', '500000000', {'proxy_reservation': 1}), arg=arg),
                   scope(cid, ev, 's', 'owned_closed', DN, [s + '/T2'], [P('CK', 'SUPPORTED-LOW')], low=EMPTY_LOW, high=EMPTY_HIGH, arg=arg),
                   scope(cid, ev, 's', 'owned_outstanding', DN, [s + '/T2'], [P('CK', 'SUPPORTED-LOW')], low=EMPTY_LOW, high=EMPTY_HIGH, arg=arg)]
    finish(case, s, prem, cl, scopes, kinds, [{'source': 'derivation-stage DERIVATIONS.md#N6', 'what': 'straddle argument', 'verified_how': 'closure establishment and open interval independently re-checked against §12.4/§11.2'}])
