# Authored claims for VC08 (subscription/admission namespace) and N07 (no raw origins). Literal values only.
from build_dossiers import *
from cases_dollar import lifecycle_claim, COMMON_E_NEG, finish
E, NS, PM, VA, T, G, CO = FAMILIES

def vc08():
    case = Case('VC08-subscription-measured-request'); s = 'VC08'; cid = case.cid; arg = 'DERIVATIONS-BATCH1.md#vc08'
    o1, o2 = 'S1/turn-1:1', 'S1/turn-1:2'; NSP = s + '/PREM-ADMISSION-NAMESPACE'
    prem = [{'premise_id': NSP, 'kind': 'payload_premise', 'statement': 'A conditional admission-namespace initial reference contract for S1/turn-1 (admission rid 1 on origins %s and %s).' % (o1, o2),
             'status': RU('No admission-namespace declaration exists; the only declared contract (conditional-ticket:S1/turn-1) is dollar_ticket and is inapplicable (VC08/NS2). Root owns whether an exact separate conditional admission declaration is authorized (Q-NAMESPACE).', 'root', []),
             'source_refs': [case.line(o1), case.line(o2)], 'policy_clauses': [P('BASE', '§3 admission references use a separate namespace'), P('AMD', '§3 NAMESPACE-APPLICATION'), P('FRPLAN', 'Finding matrix R8-NAMESPACE-01')]}]
    kinds = {o1: 'Admission', o2: 'Request(subscription) with Measurement'}
    cl = []
    cl.append(C(s, 'E1', E, {'origins': [o1, o2]}, DN, [case.line(o1), case.line(o2), case.f(o1, '/event', 'subscription_proxy_admitted'), case.f(o2, '/event', 'request'), case.f(o2, '/usd', '__absent__')],
        [P('CK', 'EVIDENCE E-1, E-2')], arg=arg,
        expected={'variant': 'RawOriginInventory', 'origin_count': 2, 'origins': [{'origin_id': o, 'original_position': i + 1, 'evidence_kind': kinds[o], 'raw_utf8_sha256': case.lines[o]['raw_utf8_sha256']} for i, o in enumerate([o1, o2])],
                  'unreadable': [], 'non_attempt': [], 'final_newline': True}, pos=['exact inventory'], neg=COMMON_E_NEG))
    cl.append(C(s, 'E2', E, {'origins': [o1, o2]}, DN, [case.line(o1), case.line(o2)], [P('CK', 'LOGICAL-PRIMARY rev2')], arg=arg,
        expected={'variant': 'EvidenceClassCardinality', 'reservation_classes': [], 'settlement_classes': [], 'admission_origins': [o1], 'subscription_request_origins': [o2], 'exact_copy_classes': []},
        pos=['no dollar primary classes'], neg=[('admission row counted as a dollar reservation class', 'NAMESPACE-APPLICATION / LOGICAL-PRIMARY')]))
    cl.append(C(s, 'E3', E, {'origin': o2}, DN, [case.tok('/inputs/sessions/0/launch/model', 'openai-codex/gpt-6-astra'), case.tok('/inputs/sessions/0/launch/auth_lane', 'subscription'), case.f(o2, '/provider', 'openai-codex'), case.f(o2, '/auth_lane', 'subscription'),
        case.f(o2, '/model', 'gpt-6-astra'), case.f(o2, '/response_model', 'gpt-6-astra'), case.f(o2, '/observed_tier', 'null'), case.f(o1, '/model', 'gpt-6-astra')],
        [P('CK', 'EVIDENCE MC-1..MC-3')], arg=arg,
        expected={'variant': 'IdentityStates', 'pinned': {'family': 'openai', 'model_id': 'gpt-6-astra'}, 'canonical_identity_set': [{'family': 'openai', 'model_id': 'gpt-6-astra'}], 'lane_labels': {'launch': 'subscription', 'row': 'subscription', 'provider': 'openai-codex'}, 'lane_contradiction': False},
        pos=['clean subscription lane'], neg=[('lane contradiction claimed', 'INV-F9 pool integrity')]))
    cl.append(C(s, 'NS1', NS, {'origins': [o1, o2]}, DN, [case.f(o1, '/rid', '1'), case.f(o2, '/subscription_proxy/rid', '1'), case.f(o2, '/usd', '__absent__'), case.tok('/inputs/sessions/0/launch/auth_lane', 'subscription')],
        [P('BASE', '§3'), P('AMD', '§3 NAMESPACE-APPLICATION')], arg=arg,
        expected={'variant': 'RawReferenceNamespace', 'per_origin': {o1: {'namespace': 'admission', 'rid': '1'}, o2: {'namespace': 'admission', 'rid': '1', 'via': 'subscription_proxy.rid'}}, 'dollar_ticket_origins': []},
        pos=['admission namespace'], neg=[('dollar_ticket applied', 'R8-NAMESPACE-01')], findings=[('R8-NAMESPACE-01', 'direct')]))
    cl.append(C(s, 'NS2', NS, {'declared_contract': 'conditional-ticket:S1/turn-1'}, DN, [case.line(o1), case.line(o2)], [P('CK', 'INITIAL-REFERENCE rev2'), P('AMD', '§3 NAMESPACE-APPLICATION'), P('FRPLAN', 'Finding matrix R8-NAMESPACE-01')], arg=arg,
        expected={'variant': 'DeclaredPayloadRawConsistency', 'declared': {'reference_namespace': 'dollar_ticket', 'duplicate_rule': 'same_origin_only', 'covered_origin_refs': [o1, o2], 'payload_guarantee_text': 'Usable dollar rid is interpreted only within this immutable conditional fixture source scope'},
                  'raw_consistent': False, 'mismatches': ['declared dollar_ticket namespace vs raw admission-namespace references on both origins; no usable dollar rid exists'], 'applicable': False, 'establishes_admission': False},
        pos=['separately admitted exact conditional admission-namespace declaration (future positive)'], neg=[('accept dollar_ticket payload for admission origins even when bundle/refs agree', 'R8-NAMESPACE-01 namespace/payload application')],
        findings=[('R8-NAMESPACE-01', 'direct')]))
    cl.append(C(s, 'NS3', NS, {'source': 'S1/turn-1'}, RU('No admission-namespace grant', 'root', [NSP]), [NSP], [P('BASE', '§2-§3')], arg=arg,
        if_admitted={'variant': 'NamespaceApplication', 'initial_base': {'kind': 'AdmissionClaim', 'scope': cid + '/S1/turn-1', 'admission_rid': '1', 'origins': [o1, o2]}, 'hold_units': [[o1, o2]],
                     'if_not_admitted': 'OriginBase per origin (two codex units); money identical (none) in both branches'},
        pos=['exact conditional admission declaration'], neg=[('label change dollar_ticket->admission without root admission', 'no guarantee from relabelling')], findings=[('R8-NAMESPACE-01', 'direct')]))
    cl.append(C(s, 'PM1', PM, {'origins': [o1, o2]}, DN, [case.tok('/inputs/sessions/0/launch/auth_lane', 'subscription'), case.f(o2, '/auth_lane', 'subscription'), case.f(o2, '/billing', 'unknown'), case.f(o2, '/usd', '__absent__'), case.f(o1, '/reserved_usd', '__absent__')],
        [P('CK', 'C-KEY-5'), P('CK', 'X-DECISION X-BRANCH-017'), P('CK', 'INV-F9'), P('CK', 'F-REPLACEMENT-1; types.Replacement')], arg=arg,
        expected={'variant': 'PrimaryCoverage', 'key_fact': 'subscription', 'dollar_primary_obligations': [], 'settlement_validity': 'not_applicable', 'obligation': {'state': 'none', 'cause': 'subscription', 'amount_source': 'none'}, 'replacement': {'eligible': False, 'reason': 'NoReservation'}, 'replacement_note': 'Post-derivation self-correction (typed vocabulary, not output-matching): ck.json type Replacement has only eligible/ineligible and F-REPLACEMENT-1 maps the no_reservation fact to NoReservation; the original label not_applicable was not a lawful variant.',
                  'accepted_coverage': [], 'definite_links': [], 'invariant': 'holds for every unit in both namespace branches'},
        pos=['no dollar obligation'], neg=[('NoReservation orphan dollar settlement invented', 'X-BRANCH-016/018 exclusions')]))
    cl.append(C(s, 'PM2', PM, {'units': 'S1/turn-1'}, RU('unit grouping (and so per-unit execution/forwarding) depends on admission-namespace grant', 'root', [NSP]), [NSP], [P('CK', 'TEMPORAL-AND-FINALIZATION §6.1-6.2')], arg=arg,
        if_admitted={'variant': 'UnitFacts', 'unit_origins': [o1, o2], 'execution': 'terminal', 'forwarding': 'forwarded', 'pool': 'codex', 'billing': 'not_applicable(unknown)'},
        pos=['one codex unit terminal/forwarded'], neg=[('two units with dollar money', 'INV-F9')]))
    cl.append(C(s, 'VA1', VA, {'origin': o2}, DN, [case.f(o2, '/subscription_proxy/cost_proxy_usd', '0.015'), case.f(o2, '/subscription_proxy/measurement', 'api_equivalent_cost_proxy'), case.f(o2, '/usage/input_tokens', '1000')],
        [P('CK', 'X-DECISION X-BRANCH-017'), P('CONTRACT', '§2.2 mechanism 2 cost-proxy'), P('CK', 'INV-F9')], arg=arg,
        expected={'variant': 'ObservationValuation', 'origin': o2, 'state': 'not_applicable', 'reason': 'clean subscription lane: Codex pool, billing unknown; cost_proxy_usd is an API-equivalent cost proxy, not dollar evidence'},
        pos=['not_applicable'], neg=[('valued 15000000 PiApi exposure', 'INV-F9 Codex attempts carry no money')]))
    cl.append(C(s, 'VA2', VA, {'origins': [o1, o2]}, DN, [case.f(o2, '/usd', '__absent__'), case.f(o1, '/reserved_usd', '__absent__')], [P('CK', 'X-DECISION X-BRANCH-017'), P('CK', 'EXACT-WIRE')], arg=arg,
        expected={'variant': 'RetainedComponents', 'components': [], 'exposure_every_unit': {'pool': 'codex', 'billing': 'unknown'}, 'invariant': 'both namespace branches'},
        pos=['codex/unknown'], neg=[('pi_api exposure with nano', 'INV-F9')]))
    cl.append(lifecycle_claim(case, s, '2026-11-10T06:50:00Z', [(o1, '2026-11-10T06:30:00Z'), (o2, '2026-11-10T06:30:02Z')], arg))
    cl.append(C(s, 'T2', T, {'origins': [o1, o2], 'evaluation': 'default'}, DN, [case.tok('/evaluations/0/objectives/0/admissions'), case.tok('/evaluations/0/objectives/0/tree_sessions/0', '00000000-0000-4000-8000-0000000000c1'), case.tok('/evaluations/0/objectives/0/started_at', '2026-11-10T06:00:00Z'), case.tok('/evaluations/0/objectives/0/closed_at')],
        [P('CONTRACT', '§11.1 O-5, O-6, O-7'), P('CONTRACT', 'INVARIANTS T5')], arg=arg,
        expected={'variant': 'Ownership', 'per_origin': {o1: {'ownership': 'unadmitted', 'candidates': ['o'], 'pool': 'codex'}, o2: {'ownership': 'unadmitted', 'candidates': ['o'], 'pool': 'codex'}},
                  'blocks_pi_api_admission': False}, pos=['unadmitted candidate o, codex only'], neg=[('HistoricalUnadmitted blocking for codex attempt', 'O-5 / T5')]))
    cl.append(C(s, 'T3', T, {'origins': [o1, o2]}, DN, [case.f(o1, '/at', '2026-11-10T06:30:00Z'), case.f(o2, '/at', '2026-11-10T06:30:02Z')], [P('CONTRACT', '§11.1 O-7; §11.2')], arg=arg,
        expected={'variant': 'Placement', 'window': {'start': '2026-11-10T06:00:00Z', 'end': 'open', 'closure': 'no_claim'}, 'owned_placements': {}, 'candidate_interval_relation': {o1: 'inside', o2: 'inside'}},
        pos=['candidate inside'], neg=[('candidate dropped as before/after', 'O-7')]))
    cl.append(C(s, 'G1', G, {'objective': 'o'}, DN, [case.tok('/evaluations/0/objectives/0/admissions'), case.tok('/profile', 'current_producer'), case.absent('/trusted_fixture_inputs')], [P('CONTRACT', '§12.3')], arg=arg,
        expected={'variant': 'CommitmentState', 'commitments': [], 'gate': 'no admissions, so no gate decision'}, pos=['no commitments'], neg=[('commitment invented from proxy admission row', 'subscription_proxy_admitted is evidence, not objective admission')]))
    cl.append(C(s, 'G2', G, {'evaluation': 'default', 'objective': 'o'}, DN, [s + '/T2', s + '/VA2'], [P('CONTRACT', '§12.1'), P('CK', 'R-FINANCIAL-PROVISIONAL rev2')], arg=arg,
        expected={'variant': 'ObjectiveStanding', 'status': 'established', 'reasons': [], 'provisional': False, 'owned_closed': {'kind': 'exact', 'nano': '0'}, 'owned_outstanding': {'kind': 'exact', 'nano': '0'}, 'predecessor_obligations': {'kind': 'exact', 'nano': '0'}},
        pos=['established'], neg=[('not_established due to codex candidate', 'O-5')]))
    cl.append(C(s, 'G3', G, {'evaluation': 'default', 'admission_request': 'o'}, DN, [case.tok('/evaluations/0/admission_request/new', '100000000'), case.tok('/evaluations/0/objectives/0/limit', '1000000000')], [P('CONTRACT', '§12.4 step 5')], arg=arg,
        expected={'variant': 'AdmissionDecision', 'objective': 'o', 'known': nano('100000000'), 'decision': 'admit', 'step': 'headroom'}, pos=['admit'], neg=[('refused/not_established', '§12.4')]))
    cl.append(C(s, 'CO1', CO, {'case': cid}, DN, [case.tok('/evaluations')], [P('CK', 'CONTINUITY rev1')], arg=arg,
        expected={'variant': 'FiniteContinuity', 'continuity_applicable': False, 'reason': 'no predecessor accounting subject; single evaluation', 'cross_evaluation_equalities': [], 'order_invariance': 'final-fact function'}, pos=['inapplicable'], neg=[('route-dependent', 'final-fact')]))
    scopes = [scope(cid, 'default', 'o', lab, DN, [s + '/T2', s + '/VA1'], [P('CK', 'SUPPORTED-LOW'), P('CONTRACT', '§11.1 O-5')], low=EMPTY_LOW, high=EMPTY_HIGH, arg=arg,
                    note='U empty: codex usage is not a PiApi financial observation (X-BRANCH-017 valuation not_applicable); nothing is owned.') for lab in ('owned_closed', 'owned_outstanding')]
    finish(case, s, prem, cl, scopes, kinds, [{'source': 'derivation-stage DERIVATIONS.md#N5', 'what': 'namespace mismatch argument', 'verified_how': 'raw fields and declared payload re-read'}])

def n07():
    case = Case('N07-control-retired-legacy'); s = 'N07'; cid = case.cid; arg = 'DERIVATIONS-BATCH1.md#n07'
    cl = []
    lines = case.tok('/inputs/sessions/0/logs/turn-1/lines')
    cl.append(C(s, 'E1', E, {'source': 'S1/turn-1'}, DN, [lines, case.tok('/inputs/sessions/0/logs/turn-1/final_newline', 'true')], [P('CK', 'EVIDENCE E-1, E-5')], arg=arg,
        expected={'variant': 'RawOriginInventory', 'origin_count': 0, 'origins': [], 'unreadable': [], 'non_attempt': [], 'final_newline': True, 'inventory': 'turn-1 log referenced and empty'},
        pos=['zero origins'], neg=[('invented attempt', 'E-1 totality')]))
    cl.append(C(s, 'E2', E, {'source': 'S1/turn-1'}, DN, [lines], [P('CK', 'LOGICAL-PRIMARY rev2')], arg=arg,
        expected={'variant': 'EvidenceClassCardinality', 'reservation_classes': [], 'settlement_classes': []}, pos=['none'], neg=[('class added', 'cardinality')]))
    cl.append(C(s, 'E3', E, {'session': 'S1'}, DN, [case.tok('/inputs/sessions/0/launch/model', 'openai/gpt-6-astra')], [P('CK', 'EVIDENCE MC-1')], arg=arg,
        expected={'variant': 'IdentityStates', 'pinned': {'family': 'openai', 'model_id': 'gpt-6-astra'}, 'usage_observations': []}, pos=['no observations'], neg=[('valuation invented', 'E-2')]))
    cl.append(C(s, 'NS1', NS, {'source': 'S1/turn-1'}, DN, [lines], [P('BASE', '§2-§3')], arg=arg,
        expected={'variant': 'NamespaceApplication', 'applicable': False, 'reason': 'no raw origins, so no initial reference contract or grant is required'}, pos=['no initial contracts'], neg=[('declared scope covering nonexistent origin', 'INITIAL-REFERENCE origin inventory binding')]))
    cl.append(C(s, 'PM1', PM, {'source': 'S1/turn-1'}, DN, [lines], [P('CK', 'LOGICAL-PRIMARY'), P('CK', 'WHOLE-COVERAGE')], arg=arg,
        expected={'variant': 'PrimaryCoverage', 'applicable': False, 'primary_obligations': [], 'accepted_coverage': []}, pos=['none'], neg=[('attempt inferred from admission row', 'attempts come only from evidence lines')]))
    cl.append(C(s, 'VA1', VA, {'source': 'S1/turn-1'}, DN, [lines], [P('CK', 'VALUATION'), P('CK', 'X-DECISION')], arg=arg,
        expected={'variant': 'RetainedComponents', 'applicable': False, 'components': [], 'units': []}, pos=['no money'], neg=[('max_usd treated as spend', 'FB-2: declared cap is not spend or bound')]))
    cl.append(lifecycle_claim(case, s, '2026-11-10T06:20:00Z', [], arg))
    cl.append(C(s, 'T2', T, {'evaluation': 'default'}, DN, [case.tok('/evaluations/0/objectives/0/admissions/0/source', 'role_host_objective_admission'), case.tok('/evaluations/0/objectives/0/admissions/0/rows/0/event', 'objective_admission'), case.tok('/evaluations/0/objectives/0/admissions/0/rows/0/session_id', '00000000-0000-4000-8000-0000000000c1')],
        [P('CONTRACT', '§11.1 O-1')], arg=arg,
        expected={'variant': 'Ownership', 'attempts': [], 'admitted_sessions': {'00000000-0000-4000-8000-0000000000c1': {'objective': 'o', 'admission': 'legacy:00000000-0000-4000-8000-0000000000c1 objective_admission row'}}},
        pos=['no attempts; session admitted to o'], neg=[('placement/ownership records for nonexistent attempts', 'T totality')]))
    cl.append(C(s, 'G1', G, {'admission': 'legacy:00000000-0000-4000-8000-0000000000c1'}, DN, [case.tok('/profile', 'current_producer'), case.absent('/inputs/gates'), case.absent('/trusted_fixture_inputs')], [P('CONTRACT', '§2.3; Revision6')], arg=arg,
        expected={'variant': 'GateQualification', 'qualified': False, 'reasons': ['NoGateEvidence'], 'provenance': 'none'}, pos=['NoGateEvidence'], neg=[('retirement treated as gate', 'L-7')]))
    cl.append(C(s, 'G2', G, {'admission': 'legacy:00000000-0000-4000-8000-0000000000c1'}, DN, [case.tok('/evaluations/0/objectives/0/admissions/0/rows/0/max_usd', '0.8'), case.tok('/inputs/sessions/0/lifecycle/1/at', '2026-11-10T06:20:00Z'), s + '/T1', s + '/G1'],
        [P('CONTRACT', '§11.1 O-1 (legacy row = admission record)'), P('CONTRACT', '§2.3 producer fact; MONEY consequence Q10/OQ-1'), P('CONTRACT', '§12.3 A-2, A-3, A-5')], arg=arg,
        expected={'variant': 'CommitmentState', 'admission_id': 'legacy:00000000-0000-4000-8000-0000000000c1', 'cap': nano('800000000'), 'state': 'retired_resumable', 'committed': nano('800000000'), 'derivation': 'max(0, 800000000 - 0)', 'kind': 'at_least',
                  'blocking_must_include': ['UnenforcedCommitment'],
                  'interpretation': 'Legacy objective_admission row is the session admission record (O-1), so A-3 Launched applies; the alternative pending-like reading (A-2 committed in full) yields the same committed 800000000 at_least. Only the state label depends on this reading.'},
        pos=['retired_resumable AtLeast800000000'], neg=[('released/retired because the session retired with no gate', 'L-7 / A-3'), ('committed 0 because no spend', 'A-3 ceiling minus owned high')]))
    cl.append(C(s, 'G3', G, {'evaluation': 'default', 'objective': 'o'}, DN, [s + '/G2'], [P('CONTRACT', '§12.1'), P('CK', 'R-FINANCIAL-PROVISIONAL rev2')], arg=arg,
        expected={'variant': 'ObjectiveStanding', 'status': 'not_established', 'reasons_must_include': ['UnenforcedCommitment'], 'provisional': True, 'owned_closed': {'kind': 'exact', 'nano': '0'}, 'owned_outstanding': {'kind': 'exact', 'nano': '0'}, 'predecessor_obligations': {'kind': 'exact', 'nano': '0'}},
        pos=['not_established provisional'], neg=[('established because no spend and session retired', 'MONEY consequence: retired session can resume')]))
    cl.append(C(s, 'G4', G, {'evaluation': 'default', 'admission_request': 'o'}, DN, [case.tok('/evaluations/0/admission_request/new', '500000000'), case.tok('/evaluations/0/objectives/0/limit', '1000000000'), s + '/G2'], [P('CONTRACT', '§12.4 step 3')], arg=arg,
        expected={'variant': 'AdmissionDecision', 'objective': 'o', 'known': nano('1300000000'), 'derivation': '0+0+0+800000000+500000000', 'decision': 'refused', 'step': 'known_over_limit'}, pos=['refused'], neg=[('admit ignoring retired_resumable commitment', '§12.4 step 3')]))
    cl.append(C(s, 'CO1', CO, {'case': cid}, DN, [case.tok('/evaluations')], [P('CK', 'CONTINUITY rev1')], arg=arg,
        expected={'variant': 'FiniteContinuity', 'continuity_applicable': False, 'reason': 'no origins, no predecessor subject, single evaluation', 'cross_evaluation_equalities': []}, pos=['inapplicable'], neg=[('n/a', 'n/a')]))
    scopes = [scope(cid, 'default', 'o', lab, DN, [s + '/E1'], [P('CK', 'SUPPORTED-LOW')], low=EMPTY_LOW, high=EMPTY_HIGH, arg=arg) for lab in ('owned_closed', 'owned_outstanding')]
    finish(case, s, [], cl, scopes, {}, [])

if __name__ == '__main__':
    vc08(); n07()
