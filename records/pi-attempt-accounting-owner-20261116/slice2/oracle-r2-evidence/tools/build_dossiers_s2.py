# Slice-2a dossier assembler (rendering only): authored literal claims + byte-exact SourceRefs.
# Computes no money, ownership, placement or low. Fails closed when a referenced token/field lexeme differs.
import hashlib, json, os

EV = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
SUBJ = os.path.join(os.path.dirname(EV), 'oracle-r2')
SLICE1 = os.path.join(os.path.dirname(os.path.dirname(EV)), 'slice1')
FAM = ['raw_E_identity_cardinality', 'namespace_grants', 'primary_membership_validity_exact_F', 'valuation_retained_amounts',
       'T_ownership_candidates_lifecycle_placement', 'gate_commitment_provisionality', 'finite_continuity_order']
E, NS, PM, VA, T, G, CO = FAM
DN = {'kind': 'DerivedNominal'}
def RU(reason, owner, prem=()): return {'kind': 'RequiredUnmet', 'reason': reason, 'decision_owner': owner, 'unmet_premise_refs': list(prem)}
OUTSIDE = RU('Not derived in slice 2a: outside the finding controls this case supports (full S1 out of scope per slice-2a packet).', 'oracle (future batch)')
def sha(p): return hashlib.sha256(open(p, 'rb').read()).hexdigest()
POL = {'BASE': '/home/bline/cbm-work/pi-accounting-continuation-20261114/design/PLAN.md', 'AMD': '/home/bline/cbm-work/pi-accounting-continuation-20261114/reviews/design-r7/PLAN-AMENDMENT.md',
       'CK': SUBJ + '/spec/ck.json', 'CONTRACT': SUBJ + '/contract/CONTRACT.md', 'RD1': SLICE1 + '/ROOT-DECISIONS-R1.json', 'RA': SLICE1 + '/ROOT-ADMISSIONS.json',
       'RATES-OPENAI': SUBJ + '/oracle/rates/openai_api_rates.json'}
POLICY_REFS = [{'policy_id': k, 'path': v, 'sha256': sha(v)} for k, v in POL.items()]
def P(pid, sec): assert pid in POL; return {'policy_id': pid, 'section': sec}
def ARG(anchor): return {'file': 'slice2/oracle-r2-evidence/DERIVATIONS-S2A.md', 'anchor': anchor}

class Case:
    def __init__(self, cid):
        self.cid = cid; self.sr = json.load(open(os.path.join(EV, 'sourcerefs', cid + '.json')))
        self.lines = {l['origin_id']: l for l in self.sr['lines']}
    def tok(self, ptr, lexeme=None):
        t = self.sr['tokens'][ptr]
        if lexeme is not None: assert t.get('lexeme') == lexeme, (self.cid, ptr, t.get('lexeme'), lexeme)
        r = {'ref': 'case_token', 'file': self.sr['file'], 'file_sha256': self.sr['file_sha256'], 'pointer': ptr, 'byte_start': t['byte_start'],
             'byte_end_exclusive': t['byte_end_exclusive'], 'token_sha256': t['token_sha256'], 'presence': 'null' if t['json_type'] == 'null' else 'value', 'json_type': t['json_type']}
        if 'lexeme' in t: r['lexeme'] = t['lexeme']
        return r
    def line(self, o):
        l = self.lines[o]
        return {'ref': 'raw_line', 'file': self.sr['file'], 'file_sha256': self.sr['file_sha256'], 'origin_id': o, 'pointer': l['pointer'], 'byte_start': l['byte_start'],
                'byte_end_exclusive': l['byte_end_exclusive'], 'token_sha256': l['token_sha256'], 'raw_utf8_sha256': l['raw_utf8_sha256'], 'raw_utf8_length': l['raw_utf8_length']}
    def f(self, o, fp, lexeme='__any__'):
        l = self.lines[o]
        if fp not in l['fields']:
            assert lexeme in ('__absent__', '__any__'), (self.cid, o, fp); return {'ref': 'raw_field', 'origin_id': o, 'raw_utf8_sha256': l['raw_utf8_sha256'], 'field': fp, 'presence': 'absent'}
        x = l['fields'][fp]; assert lexeme != '__absent__', (self.cid, o, fp)
        if lexeme != '__any__': assert x.get('lexeme') == lexeme, (self.cid, o, fp, x.get('lexeme'), lexeme)
        r = {'ref': 'raw_field', 'origin_id': o, 'raw_utf8_sha256': l['raw_utf8_sha256'], 'field': fp, 'presence': x['presence'], 'json_type': x['json_type']}
        if 'lexeme' in x: r['lexeme'] = x['lexeme']
        return r

def C(s, n, fam, subj, status, prem, clauses, expected=None, anchor=None, pos=(), neg=(), findings=(), note=None):
    c = {'claim_id': f'{s}/{n}', 'family': fam, 'subject': subj, 'status': status, 'premise_refs': list(prem), 'policy_clauses': list(clauses),
         'argument_ref': ARG(anchor or s.lower()), 'expected': expected if status['kind'] == 'DerivedNominal' else None, 'consequence_if_admitted': None,
         'controls': {'positive': list(pos), 'negative': [{'mutation': m, 'intended_predicate': p} for m, p in neg]}, 'findings': [{'finding_id': a, 'relation': b} for a, b in findings]}
    if note: c['note'] = note
    assert (status['kind'] == 'DerivedNominal') == (expected is not None)
    return c

def outside(s, fam, n): return C(s, n, fam, {'case': s}, OUTSIDE, [], [P('CK', 'n/a')], note='family not derived in slice 2a')

def scope(cid, ev, obj, lab, status, prem, low=None, high=None, high_status=None, note=None):
    r = {'scope_id': f'{cid}/{ev}/{obj}/{lab}', 'evaluation': ev, 'objective': obj, 'label': lab, 'status': status, 'premise_refs': prem, 'policy_clauses': [P('CK', 'SUPPORTED-LOW'), P('BASE', '§5')],
         'argument_ref': ARG(cid.split('-')[0].lower()), 'expected_low': low if status['kind'] == 'DerivedNominal' else None,
         'high': {'status': high_status or status, 'expected': high}, 'consequence_if_admitted': None}
    if note: r['note'] = note
    return r
def LOW(units, U, seed, single='0', sel='0', mode='none', basis='none'):
    return {'owned_unit_origin_sets': units, 'U': U, 'seed': seed, 'slots': [], 'single_floor': {'nano': single}, 'independent_floor': {'nano': '0'}, 'credits': [], 'selected_low': {'nano': sel}, 'credit_mode': mode, 'basis': basis}
def H(kind, n, hist): return {'kind': kind, 'nano': n, 'basis_histogram': hist}

TOP = {'expected': 'supplied_outcome', 'findings': 'supplied_outcome', 'historical': 'supplied_outcome', 'rationale': 'supplied_outcome', 'evaluations': 'governing_premise', 'inputs': 'governing_premise',
       'profile': 'governing_premise', 'case_id': 'governing_premise', 'contract': 'governing_premise', 'schema': 'governing_premise', 'role': 'governing_premise', 'trusted_fixture_inputs': 'governing_premise'}
def causal(case, scopes):
    inv = []
    for k in case.sr['top_inventory']:
        cls = 'supplied_outcome' if k.startswith('revisions') else TOP.get(k, 'noncausal_metadata')
        rec = {'key': k, 'classification': cls, 'reason': 'see batch-1 DOSSIER-SCHEMA classification table (same rules)'}
        rec['source_ref'] = {kk: vv for kk, vv in case.tok('/' + k).items() if kk != 'lexeme'} if '/' + k in case.sr['tokens'] else {'ref': 'excluded_output_member', 'pointer': '/' + k}
        inv.append(rec)
    tfi = case.tok('/trusted_fixture_inputs') if '/trusted_fixture_inputs' in case.sr['tokens'] else {'ref': 'case_token', 'file': case.sr['file'], 'file_sha256': case.sr['file_sha256'], 'pointer': '/trusted_fixture_inputs', 'presence': 'absent'}
    return {'top_inventory': inv, 'trusted_fixture_inputs': {k: v for k, v in tfi.items() if k != 'lexeme'}, 'profile': case.tok('/profile'), 'contract': case.tok('/contract'),
            'schema': case.tok('/schema'), 'role': case.tok('/role'), 'inputs_canonical_sha256': case.sr['inputs_canonical_sha256'], 'log_final_newline': case.sr.get('log_final_newline', {}),
            'required_scopes': scopes, 'additional_channels': []}

def write(case, claims, scopes, premises=(), reuse=(), note=''):
    fs = {}
    for fam in FAM:
        cs = [c for c in claims if c['family'] == fam]; assert cs, (case.cid, fam)
        un = [c['claim_id'] for c in cs if c['status']['kind'] != 'DerivedNominal']
        fs[fam] = {'status': 'DerivedNominal' if not un else 'RequiredUnmet', 'claim_ids': [c['claim_id'] for c in cs], 'unmet_claim_ids': un,
                   'decision_owners': sorted({c['status']['decision_owner'] for c in cs if c['status']['kind'] != 'DerivedNominal'})}
    rec = {'schema': 'pi-accounting-oracle-dossier/1', 'case_id': case.cid, 'author': {'role': 'independent oracle (slice 2a)', 'owner_session': '01a1e569-2805-72ff-b837-03d1577c405d'},
           'standing': 'Independently authored finite derivation for slice-2a finding controls; not consumer qualification. ' + note,
           'subject': {'branch': 'oracle-r2', 'case_file': {'path': case.sr['file'], 'sha256': case.sr['file_sha256']}, 'inputs_canonical_sha256': case.sr['inputs_canonical_sha256']},
           'policy_refs': POLICY_REFS, 'causal_admission': causal(case, [s['scope_id'] for s in scopes]),
           'origins': [{'origin_id': o, 'source_origin_id': o.split(':')[0], 'original_position': int(o.split(':')[1]), 'source_ref': case.line(o), 'parse_state': l['parse_state'], 'field_states': l['fields']} for o, l in case.lines.items()],
           'premises': list(premises), 'claims': claims, 'family_status': fs, 'scope_lows': scopes,
           'excluded_channels': [{'channel': 'K attempt finding codes', 'reason': 'not one of the seven families'}], 'reuse': list(reuse)}
    json.dump(rec, open(os.path.join(EV, 'dossier', case.cid + '.json'), 'w'), indent=1)
    print('wrote', case.cid, len(claims), 'claims', len(scopes), 'scopes')

# ------------------------------------------------------------------ namespace split cases (R8-NAMESPACE-01)
QNS = {'premise_id': 'Q-NAMESPACE', 'kind': 'policy_clause', 'statement': 'ROOT-DECISIONS-R1 Q-NAMESPACE: no admission-namespace scope is admitted for these historical cases; the not-admitted branch is nominal (root reply 2026-11-16: OriginBase split).',
       'status': DN, 'source_refs': [{'ref': 'root_admission', 'record': 'ROOT-DECISIONS-R1.json', 'entry_id': 'Q-NAMESPACE'}], 'policy_clauses': [P('RD1', 'Q-NAMESPACE'), P('BASE', '§3')]}
def ns_case(cid, s, unitB, objective_status, admission, lane_note):
    c = Case(cid); o1, o2 = 'S1/turn-1:1', 'S1/turn-1:2'; cl = []
    cl.append(C(s, 'E1', E, {'origins': [o1, o2]}, DN, [c.line(o1), c.line(o2), c.f(o1, '/event', 'subscription_proxy_admitted'), c.f(o2, '/event'), c.f(o2, '/usd', '__absent__')], [P('CK', 'EVIDENCE E-1, E-2')],
                expected={'variant': 'RawOriginInventory', 'origin_count': 2, 'origins': [{'origin_id': o1, 'original_position': 1, 'evidence_kind': 'Admission', 'raw_utf8_sha256': c.lines[o1]['raw_utf8_sha256']},
                          {'origin_id': o2, 'original_position': 2, 'evidence_kind': unitB['kind'], 'raw_utf8_sha256': c.lines[o2]['raw_utf8_sha256']}], 'unreadable': [], 'non_attempt': [], 'final_newline': True}))
    cl.append(C(s, 'NS1', NS, {'origins': [o1, o2]}, DN, [c.f(o1, '/rid', '1'), c.f(o2, '/subscription_proxy/rid', '1'), c.f(o2, '/usd', '__absent__')], [P('BASE', '§3'), P('AMD', '§3 NAMESPACE-APPLICATION')],
                expected={'variant': 'RawReferenceNamespace', 'per_origin': {o1: {'namespace': 'admission', 'rid': '1'}, o2: {'namespace': 'admission', 'rid': '1', 'via': 'subscription_proxy.rid'}}, 'dollar_ticket_origins': []},
                pos=['corrected input proof: one unestablished initial contract (no grant, no scope), OriginBase per origin (join remediation)'], neg=[('re-declare dollar_ticket contract with matching grant/refs', 'R8-NAMESPACE-01 namespace_application')], findings=[('R8-NAMESPACE-01', 'direct')]))
    cl.append(C(s, 'NS2', NS, {'historical_declared_contract': 'conditional-ticket:S1/turn-1'}, DN, [c.line(o1), c.line(o2)], [P('CK', 'INITIAL-REFERENCE rev2'), P('AMD', '§3 NAMESPACE-APPLICATION')],
                expected={'variant': 'DeclaredPayloadRawConsistency', 'declared': {'reference_namespace': 'dollar_ticket', 'covered_origin_refs': [o1, o2]}, 'raw_consistent': False,
                          'mismatches': ['declared dollar_ticket vs raw admission-namespace references'], 'establishes_admission': False, 'disposition': 'withdrawn from supplied outputs (r11 revision)'}, findings=[('R8-NAMESPACE-01', 'direct')]))
    cl.append(C(s, 'NS3', NS, {'source': 'S1/turn-1'}, DN, ['Q-NAMESPACE'], [P('RD1', 'Q-NAMESPACE'), P('BASE', '§3 unestablished scope => OriginBase')],
                expected={'variant': 'NamespaceApplication', 'initial_base': [{'kind': 'OriginBase', 'origins': [o1]}, {'kind': 'OriginBase', 'origins': [o2]}], 'atomize': False, 'break_copies': False, 'hold_units': [[o1], [o2]]},
                note='Grouped single-unit form exists only in the private conditional control oracle/r11-controls/admission-namespace under a root-admitted declaration.', findings=[('R8-NAMESPACE-01', 'direct')]))
    cl.append(C(s, 'PM2', PM, {'units': ['adm:S1/turn-1:rid=1', 'row:S1/turn-1:line=2']}, DN, [s + '/NS3', c.line(o1), c.line(o2)], [P('CK', 'TEMPORAL-AND-FINALIZATION §6.1-6.2'), P('CK', 'C-KEY-5')],
                expected={'variant': 'PrimaryCoverage', 'dollar_primary_obligations': [], 'units': {'adm:S1/turn-1:rid=1': {'origins': [o1], 'execution': 'not_terminal', 'forwarding': 'unknown', 'settlement_validity': 'not_applicable',
                          'obligation': {'state': 'none', 'cause': 'subscription', 'amount_source': 'none'}, 'replacement': {'eligible': False, 'reason': 'NoReservation'}},
                          'row:S1/turn-1:line=2': dict(unitB['facts'], origins=[o2])}, 'accepted_coverage': [], 'definite_links': []}, findings=[('R8-NAMESPACE-01', 'direct')]))
    cl.append(C(s, 'VA2', VA, {'units': ['adm:S1/turn-1:rid=1', 'row:S1/turn-1:line=2']}, DN, [c.f(o2, '/usd', '__absent__'), c.f(o1, '/reserved_usd', '__absent__')], [P('CK', 'X-DECISION X-BRANCH-017'), P('CK', 'INV-F9')],
                expected={'variant': 'RetainedComponents', 'components': [], 'exposure': {'adm:S1/turn-1:rid=1': {'pool': 'codex', 'billing': 'unknown'}, 'row:S1/turn-1:line=2': unitB['exposure']}}, note=lane_note))
    cl.append(C(s, 'T2', T, {'units': ['adm:S1/turn-1:rid=1', 'row:S1/turn-1:line=2']}, DN, [c.tok('/evaluations/0/objectives/0/admissions'), c.tok('/evaluations/0/objectives/0/tree_sessions/0')], [P('CONTRACT', '§11.1 O-5, O-6, O-7')],
                expected={'variant': 'Ownership', 'per_unit': {'adm:S1/turn-1:rid=1': {'ownership': 'unadmitted', 'candidates': ['o']}, 'row:S1/turn-1:line=2': {'ownership': 'unadmitted', 'candidates': ['o']}}}))
    cl.append(C(s, 'G2', G, {'evaluation': 'default', 'objective': 'o'}, DN, [s + '/T2', s + '/VA2', c.tok('/evaluations/0/admission_request/new', '100000000')], [P('CONTRACT', '§12.1, §12.4')],
                expected={'variant': 'ObjectiveStandingAndAdmission', 'objective': 'o', **objective_status, 'commitments': [], 'admission': admission, 'invariant': 'unchanged by the OriginBase split (money identical in both branches)'}))
    cl.append(C(s, 'CO1', CO, {'case': cid}, DN, [c.tok('/evaluations')], [P('CK', 'CONTINUITY rev1')], expected={'variant': 'FiniteContinuity', 'continuity_applicable': False, 'reason': 'no predecessor accounting subject', 'cross_evaluation_equalities': []}))
    sc = [scope(cid, 'default', 'o', lab, DN, [s + '/T2'], low=LOW([], [], []), high=H('exact', '0', {}), note='no owned unit; the PiApi candidate (if any) has no usage observation') for lab in ('owned_closed', 'owned_outstanding')]
    write(c, cl, sc, premises=[QNS], note='R8-NAMESPACE-01 OriginBase truth.')

CODEX_B = lambda terminal, fwd: {'kind': 'Request/UpstreamError (subscription-labelled)', 'facts': {'execution': terminal, 'forwarding': fwd, 'settlement_validity': 'not_applicable',
                                  'obligation': {'state': 'none', 'cause': 'subscription', 'amount_source': 'none'}, 'replacement': {'eligible': False, 'reason': 'NoReservation'}}, 'exposure': {'pool': 'codex', 'billing': 'unknown'}}
PIAPI_B = {'kind': 'UpstreamError (lane-contradicted/unlabelled)', 'facts': {'execution': 'terminal', 'forwarding': 'unknown', 'settlement_validity': 'not_applicable',
           'obligation': {'state': 'none', 'cause': 'no_reservation', 'amount_source': 'none'}, 'replacement': {'eligible': False, 'reason': 'NoReservation'}, 'blocking_must_include': ['LaneContradiction', 'UnresolvedExposure']},
           'exposure': {'pool': 'pi_api', 'high': {'kind': 'at_least', 'nano': '0', 'basis': 'none'}, 'low': {'nano': '0', 'basis': 'none'}}}
EST = {'status': 'established', 'reasons': [], 'provisional': False}
NEST = {'status': 'not_established', 'reasons_must_include': ['HistoricalUnadmitted', 'LaneContradiction', 'UnresolvedExposure'], 'provisional': True}
ns_case('VC08-subscription-measured-request', 'VC08', CODEX_B('terminal', 'forwarded'), EST, {'decision': 'admitted', 'known': '100000000'}, 'clean subscription lane on both origins')
ns_case('V03-control-labelled-upstream-error', 'V03C', CODEX_B('terminal', 'unknown'), EST, {'decision': 'admitted', 'known': '100000000'}, 'upstream_error labelled openai-codex/subscription: clean subscription')
ns_case('U06-subscription-upstream-error-unlabelled', 'U06', PIAPI_B, NEST, {'decision': 'not_established'},
        'Unlabelled upstream_error keeps the historical lane determination (PiApi AtLeast0, LaneContradiction) on its own OriginBase unit; the admission row alone is clean subscription (same as N03-control-unlabelled-admission).')
ns_case('V03-subscription-upstream-error-labelled-api', 'V03A', PIAPI_B, NEST, {'decision': 'not_established'},
        'upstream_error labelled provider openai/auth_lane api on a subscription launch: LaneContradiction keeps PiApi AtLeast0 (INV-F9); the admission row alone is clean subscription.')

def n03(cid, s, exposure):
    c = Case(cid); o1 = 'S1/turn-1:1'
    cl = [C(s, 'E1', E, {'origins': [o1]}, DN, [c.line(o1), c.f(o1, '/event', 'subscription_proxy_admitted')], [P('CK', 'EVIDENCE E-1')],
            expected={'variant': 'RawOriginInventory', 'origin_count': 1, 'origins': [{'origin_id': o1, 'original_position': 1, 'evidence_kind': 'Admission', 'raw_utf8_sha256': c.lines[o1]['raw_utf8_sha256']}], 'unreadable': [], 'non_attempt': [], 'final_newline': True}),
          C(s, 'NS1', NS, {'origins': [o1]}, DN, [c.f(o1, '/rid', '1')], [P('BASE', '§3'), P('AMD', '§3 NAMESPACE-APPLICATION')], expected={'variant': 'RawReferenceNamespace', 'per_origin': {o1: {'namespace': 'admission', 'rid': '1'}}},
            neg=[('dollar_ticket contract on this origin', 'R8-NAMESPACE-01')], findings=[('R8-NAMESPACE-01', 'direct')]),
          C(s, 'NS3', NS, {'source': 'S1/turn-1'}, DN, ['Q-NAMESPACE'], [P('RD1', 'Q-NAMESPACE')], expected={'variant': 'NamespaceApplication', 'initial_base': [{'kind': 'OriginBase', 'origins': [o1]}], 'hold_units': [[o1]]},
            note='single origin: unit structure unchanged; only the false dollar_ticket contract/grant was withdrawn'),
          C(s, 'VA2', VA, {'unit': 'adm:S1/turn-1:rid=1'}, DN, [c.line(o1)], [P('CK', 'X-DECISION'), P('CK', 'INV-F9')], expected={'variant': 'RetainedComponents', 'components': [], 'exposure': exposure})]
    for fam, n in ((PM, 'PM'), (T, 'T'), (G, 'G'), (CO, 'CO')): cl.append(outside(s, fam, n))
    write(c, cl, [], premises=[QNS], note='Only the namespace claims are derived; supplied money/attribution unchanged by the r11 revision.')
n03('N03-contradicted-admission', 'N03X', {'pool': 'pi_api', 'high': {'kind': 'at_least', 'nano': '0', 'basis': 'none'}, 'note': 'admission row labelled api on subscription launch: lane contradiction'})
n03('N03-control-unlabelled-admission', 'N03C', {'pool': 'codex', 'billing': 'unknown'})

# ------------------------------------------------------------------ R8-LOW-INVENTORY-01
def low_case(cid, s, usage_origin, cands, cand_reason, objs, valuation, extra_refs):
    c = Case(cid); cl = []
    cl.append(C(s, 'T2', T, {'attempt_usage_origin': usage_origin}, DN, extra_refs, [P('CONTRACT', '§11.1 O-3, O-3a, O-6, O-7; §10 L-2, L-6; §11.2 P-2')],
                expected={'variant': 'Ownership', 'usage_origin': usage_origin, 'ownership': cand_reason[0], 'candidates': cands, 'owned_by': None, 'reason': cand_reason[1]}, findings=[('R8-LOW-INVENTORY-01', 'direct')]))
    cl.append(C(s, 'VA1', VA, {'usage_origin': usage_origin}, DN, [c.line(usage_origin), c.f(usage_origin, '/usage')], [P('CK', 'VALUATION V-1..V-5'), P('RATES-OPENAI', 'gpt-6-astra')], expected=valuation))
    for fam, n in ((E, 'E'), (NS, 'NS'), (PM, 'PM'), (G, 'G'), (CO, 'CO')): cl.append(outside(s, fam, n))
    sc = []
    for o in objs:
        sc.append(scope(cid, 'default', o, 'owned_closed', DN, [s + '/T2', s + '/VA1'], low=LOW([], [usage_origin], [], '0', '0', 'none', 'none'), high_status=OUTSIDE,
                        note='candidate usage stays in U (possible alternative) with no owned Seed, slot or credit: single_floor=0 because Seed is empty'))
        sc.append(scope(cid, 'default', o, 'owned_outstanding', DN, [s + '/T2'], low=LOW([], [], []), high_status=OUTSIDE))
    write(c, cl, sc, note='Scope lows for R8-LOW-INVENTORY-01; other families not derived in slice 2a.')
R08 = Case('P130-R08-admissions-without-tree-still-block')
low_case(R08.cid, 'R08', 'S1/turn-1:2', ['o', 'p'], ('uncertain', 'two admitting objectives (o a1, p b1) both launched session c1: O-3 Uncertain; O-3a keeps every admitting objective as candidate though neither tree contains S1'), ['o', 'p'],
         {'variant': 'ObservationValuation', 'origin': 'S1/turn-1:2', 'state': 'valued', 'complete': True, 'low': {'nano': '15000000'}, 'high': {'nano': '15000000'}, 'derivation': '1000*10000+100*50000'},
         [R08.tok('/evaluations/0/objectives/0/admissions/0/rows/1/event', 'launched'), R08.tok('/evaluations/0/objectives/1/admissions/0/rows/1/event', 'launched'), R08.tok('/evaluations/0/objectives/0/tree_sessions'), R08.tok('/evaluations/0/objectives/1/tree_sessions')])
R12 = Case('P130-R12-untrusted-clock-keeps-candidate')
low_case(R12.cid, 'R12', 'S1/turn-1:1', ['o'], ('unadmitted', 'no admission names S1 (O-6); row 06:30:02 precedes turn start 06:35:00 (L-2) so the clock is untrusted/contradicted and the interval is unplaceable (P-2); O-7 keeps tree objective o'), ['o'],
         {'variant': 'ObservationValuation', 'origin': 'S1/turn-1:1', 'state': 'valued', 'complete': True, 'low': {'nano': '15000000'}, 'high': {'nano': '15000000'}, 'derivation': '1000*10000+100*50000'},
         [R12.tok('/inputs/sessions/0/lifecycle/0/at', '2026-11-10T06:35:00Z'), R12.f('S1/turn-1:1', '/at', '2026-11-10T06:30:02Z'), R12.tok('/evaluations/0/objectives/0/admissions'), R12.tok('/evaluations/0/objectives/0/tree_sessions/0')])
for cid, s, val in (('P133-A04-cache-counts-both-absent', 'A04B', {'variant': 'ObservationValuation', 'origin': 'S1/turn-1:1', 'state': 'valued', 'low_nano': 'see supplied 6000000 (V-4 vertices; not re-derived here)', 'note': 'U membership depends only on candidate usage presence'}),
                    ('P133-A04-cache-write-absent-legacy', 'A04W', {'variant': 'ObservationValuation', 'origin': 'S1/turn-1:1', 'state': 'valued', 'note': 'U membership depends only on candidate usage presence'})):
    k = Case(cid)
    low_case(cid, s, 'S1/turn-1:1', ['o'], ('unadmitted', 'no admission names S1 (O-6); tree of o contains S1 and its window [06:00, open) contains the row (O-7)'), ['o'], val,
             [k.tok('/evaluations/0/objectives/0/admissions'), k.tok('/evaluations/0/objectives/0/tree_sessions/0'), k.f('S1/turn-1:1', '/at', '2026-11-10T06:30:02Z')])

# ------------------------------------------------------------------ R8-PLACEMENT-01 before-window control and R8-FIN-01 non-pilot control
PT = Case('P130-T-owned-before-window-summed')
cl = [C('P130T', 'T3', T, {'attempt': 'res:S1/turn-1:rid=1', 'objective': 'o'}, RU('Closed interval [06:30:00, 06:30:02] requires the R/S grouping, which needs the declared dollar_ticket scope (proposed admission ADM-PROPOSED-P130T); unadmitted branch: reservation-only outstanding [06:30, 06:50] straddles_start and orphan settlement before.', 'root', ['P130T/PREM-SCOPE']),
        ['P130T/PREM-SCOPE', PT.tok('/evaluations/0/objectives/0/started_at', '2026-11-10T06:40:00Z'), PT.f('S1/turn-1:1', '/at', '2026-11-10T06:30:00Z'), PT.f('S1/turn-1:2', '/at', '2026-11-10T06:30:02Z')], [P('CONTRACT', '§11.2 P-1..P-3; §11.1 O-3a')],
        note='if admitted: placement before; owner o retained (admission a1) with AttributionContradiction; supplied output already says before', findings=[('R8-PLACEMENT-01', 'direct')])]
for fam, n in ((E, 'E'), (NS, 'NS'), (PM, 'PM'), (VA, 'VA'), (G, 'G'), (CO, 'CO')): cl.append(outside('P130T', fam, n))
write(PT, cl, [], premises=[{'premise_id': 'P130T/PREM-SCOPE', 'kind': 'payload_premise', 'statement': 'declared conditional dollar_ticket scope initial:S1/turn-1 covering S1/turn-1:1,2 (raw-consistent: usd_reserved rid 1 and usd.rid 1 on API lane; extent = complete source; no byte copies)',
                             'status': RU('proposed for ROOT-ADMISSIONS (see RESULT.md)', 'root'), 'source_refs': [PT.line('S1/turn-1:1'), PT.line('S1/turn-1:2')], 'policy_clauses': [P('RD1', 'D-SCOPE admission_rule')]}],
      note='Placement control positive; status RequiredUnmet until root admits the scope.')
R02 = Case('P130-R02-rounded-up-charge-admitted')
cl = [C('P130R02', 'VA2', VA, {'origin': 'S1/turn-1:2'}, DN, [R02.f('S1/turn-1:2', '/usd/charged_usd'), R02.f('S1/turn-1:2', '/usd/charge_basis', 'usage')], [P('CK', 'X-COMPONENTS rev2'), P('CK', 'MONEY R-02 rounding'), P('BASE', '§3, §6')],
        expected={'variant': 'RetainedComponents', 'components': [{'kind': 'charge', 'origin_ref': 'S1/turn-1:2', 'nano': '1000', 'role': 'high_component'}],
                  'raw_lexeme': '1e-06', 'invariant': 'the logged rounded-up charge (exponent-form lexeme 1e-06 USD, an exact nano amount under MONEY producer-amount rules) = 1000 nano is retained by its origin in every branch; any supplied settlement.charged/high below 1000 contradicts raw'}, findings=[('R8-FIN-01', 'direct')])]
for fam, n in ((E, 'E'), (NS, 'NS'), (PM, 'PM'), (T, 'T'), (G, 'G'), (CO, 'CO')): cl.append(outside('P130R02', fam, n))
write(R02, cl, [], note='Non-pilot retained-charge control.')
