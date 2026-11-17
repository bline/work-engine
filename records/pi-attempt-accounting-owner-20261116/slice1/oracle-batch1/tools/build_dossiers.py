# Dossier assembler (rendering only). Every expected value below is an authored literal from
# DERIVATIONS-BATCH1.md. This script computes NO money, validity, ownership, placement or low:
# it only attaches byte-exact SourceRefs (from sourcerefs/*.json) and policy hashes to authored claims,
# and fails closed when a referenced token/field does not exist with the stated lexeme.
import hashlib, json, os, sys

HERE = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
SUBJ = '/home/bline/cbm-work/pi-accounting-s1-owner-20261116/successor/subject'
CONT = '/home/bline/cbm-work/pi-accounting-continuation-20261114'

def sha(p):
    return hashlib.sha256(open(p, 'rb').read()).hexdigest()

POLICIES = {
    'BASE': CONT + '/design/PLAN.md',
    'AMD': CONT + '/reviews/design-r7/PLAN-AMENDMENT.md',
    'R3': CONT + '/pilot-r9/PLAN-AMENDMENT-R3.md',
    'EPR': CONT + '/renewal-r10/design-evaluation-premise-r1/PLAN-R2.md',
    'LOWT': CONT + '/renewal-r10/design-low-target-r1/PLAN-R2.md',
    'RP5': CONT + '/reviews/implementation-r8/policies/ROOT-POLICY-R5.md',
    'RA6': CONT + '/ROOT-ADOPTION-R6.json',
    'RP6': CONT + '/ROOT-POLICY-R6.md',
    'FRPLAN': CONT + '/renewal-r10/full-realization-r1/planning/PLAN.md',
    'CK': SUBJ + '/spec/ck.json',
    'CONTRACT': SUBJ + '/contract/CONTRACT.md',
    'PAPPROVAL': SUBJ + '/oracle/POLICY-APPROVAL.md',
    'RATES-OPENAI': SUBJ + '/oracle/rates/openai_api_rates.json',
    'RATES-ANTHROPIC': SUBJ + '/oracle/rates/anthropic_api_rates.json',
}
POLICY_REFS = [{'policy_id': k, 'path': v.replace(SUBJ + '/', 'subject:').replace(CONT + '/', 'continuation:'), 'sha256': sha(v)}
               for k, v in POLICIES.items()]

def P(policy, section):
    assert policy in POLICIES
    return {'policy_id': policy, 'section': section}

DN = {'kind': 'DerivedNominal'}
def RU(reason, owner, premises):
    return {'kind': 'RequiredUnmet', 'reason': reason, 'decision_owner': owner, 'unmet_premise_refs': premises}

def nano(s):
    assert isinstance(s, str) and s.isdigit()
    return {'nano': s}

class Case:
    def __init__(self, cid):
        self.cid = cid
        self.sr = json.load(open(os.path.join(HERE, 'sourcerefs', cid + '.json')))
        self.lines = {l['origin_id']: l for l in self.sr['lines']}

    def tok(self, ptr, lexeme=None):
        t = self.sr['tokens'][ptr]
        if lexeme is not None:
            assert t.get('lexeme') == lexeme, (self.cid, ptr, t.get('lexeme'), lexeme)
        r = {'ref': 'case_token', 'file': self.sr['file'], 'file_sha256': self.sr['file_sha256'], 'pointer': ptr,
             'byte_start': t['byte_start'], 'byte_end_exclusive': t['byte_end_exclusive'],
             'token_sha256': t['token_sha256'], 'presence': 'null' if t['json_type'] == 'null' else 'value',
             'json_type': t['json_type']}
        if 'lexeme' in t:
            r['lexeme'] = t['lexeme']
        return r

    def absent(self, ptr):
        assert ptr not in self.sr['tokens'], (self.cid, ptr)
        return {'ref': 'case_token', 'file': self.sr['file'], 'file_sha256': self.sr['file_sha256'], 'pointer': ptr, 'presence': 'absent'}

    def line(self, oid):
        l = self.lines[oid]
        return {'ref': 'raw_line', 'file': self.sr['file'], 'file_sha256': self.sr['file_sha256'], 'origin_id': oid,
                'pointer': l['pointer'], 'byte_start': l['byte_start'], 'byte_end_exclusive': l['byte_end_exclusive'],
                'token_sha256': l['token_sha256'], 'raw_utf8_sha256': l['raw_utf8_sha256'], 'raw_utf8_length': l['raw_utf8_length']}

    def f(self, oid, fp, lexeme='__any__'):
        l = self.lines[oid]
        if fp not in l['fields']:
            assert lexeme in ('__absent__', '__any__'), (self.cid, oid, fp)
            return {'ref': 'raw_field', 'origin_id': oid, 'raw_utf8_sha256': l['raw_utf8_sha256'], 'field': fp, 'presence': 'absent'}
        x = l['fields'][fp]
        assert lexeme != '__absent__', (self.cid, oid, fp)
        if lexeme not in ('__any__',):
            assert x.get('lexeme') == lexeme, (self.cid, oid, fp, x.get('lexeme'), lexeme)
        r = {'ref': 'raw_field', 'origin_id': oid, 'raw_utf8_sha256': l['raw_utf8_sha256'], 'field': fp,
             'presence': x['presence'], 'json_type': x['json_type']}
        if 'lexeme' in x:
            r['lexeme'] = x['lexeme']
        return r

    def origins_record(self, kinds):
        out = []
        for oid, l in self.lines.items():
            fields = {}
            for fp, x in sorted(l['fields'].items()):
                fields[fp] = {k: v for k, v in x.items()}
            out.append({'origin_id': oid, 'source_origin_id': oid.split(':')[0], 'original_position': int(oid.split(':')[1]),
                        'source_ref': self.line(oid), 'parse_state': l['parse_state'], 'evidence_kind': kinds[oid],
                        'field_states': fields})
        return out

def C(cid_short, n, family, subject, status, premises, clauses, expected=None, if_admitted=None, pos=(), neg=(), findings=(), arg=None, note=None):
    c = {'claim_id': '%s/%s' % (cid_short, n), 'family': family, 'subject': subject, 'status': status,
         'premise_refs': list(premises), 'policy_clauses': list(clauses), 'argument_ref': arg,
         'expected': expected, 'consequence_if_admitted': if_admitted,
         'controls': {'positive': list(pos), 'negative': [{'mutation': m, 'intended_predicate': p} for m, p in neg]},
         'findings': [{'finding_id': fid, 'relation': rel} for fid, rel in findings]}
    if note:
        c['note'] = note
    if status['kind'] == 'DerivedNominal':
        assert expected is not None
    else:
        assert expected is None
    return c

FAMILIES = ['raw_E_identity_cardinality', 'namespace_grants', 'primary_membership_validity_exact_F',
            'valuation_retained_amounts', 'T_ownership_candidates_lifecycle_placement',
            'gate_commitment_provisionality', 'finite_continuity_order']

TOP_CLASS = {
    'ambiguity': ('noncausal_metadata', 'Annotation; read by no adopted policy clause for any claim here.'),
    'case_id': ('governing_premise', 'Opaque equality selector only; no financial meaning.'),
    'class': ('noncausal_metadata', 'Corpus coverage label; no adopted financial/attribution clause reads it. Consumer-coverage side-condition only.'),
    'contract': ('governing_premise', 'Exact contract selector pi-accounting-contract/1.7-s1 binds the policy version used.'),
    'controls': ('noncausal_metadata', 'Cross-case control relation; selects no consequence of this case.'),
    'evaluations': ('governing_premise', 'Ordered evaluation configurations minus expected: objectives, windows, closure, predecessor, tree, admissions/journal rows, limits, admission_request, authority/gate ids.'),
    'expected': ('supplied_outcome', 'Challenged output; never nominal truth. Read only after derivation (AGREEMENT.json).'),
    'findings': ('supplied_outcome', 'Historical finding labels; no truth.'),
    'group': ('noncausal_metadata', 'Grouping label.'),
    'historical': ('supplied_outcome', 'Historical reader outcome; comparison only.'),
    'inputs': ('governing_premise', 'Complete raw sessions/launch/lifecycle/log lines, reference_time, rate digests, gates.'),
    'invariants': ('noncausal_metadata', 'Invariant coverage labels; no truth.'),
    'notes': ('noncausal_metadata', 'Free text.'),
    'profile': ('governing_premise', 'Selects gate/closure semantics (CONTRACT §2.3, §12.4 step 1).'),
    'provenance': ('noncausal_metadata', 'Origin test label; confers no grant (base plan §2: labels cannot mint admission).'),
    'rationale': ('supplied_outcome', 'Narrative about output revisions; no truth.'),
    'reachability': ('noncausal_metadata', 'Producer reachability label; no financial clause reads it.'),
    'role': ('governing_premise', 'Selector (finding/control/valid_control); qualification mode only, no money.'),
    'schema': ('governing_premise', 'Corpus schema selector.'),
    'added_in': ('noncausal_metadata', 'Revision history label.'),
    'trusted_fixture_inputs': ('governing_premise', 'Test-only harness gate verification channel (CONTRACT Revision6 gate trust boundary; POLICY-APPROVAL P5).'),
}

def causal_admission(case, scopes, extra_channels=()):
    inv = []
    for k in case.sr['top_inventory']:
        cls, why = TOP_CLASS[k] if not k.startswith('revisions') else ('supplied_outcome', 'Output revision annotation; no truth.')
        rec = {'key': k, 'classification': cls, 'reason': why}
        if k in case.sr['tokens']:
            rec['source_ref'] = case.tok('/' + k) if False else {kk: vv for kk, vv in case.tok('/' + k).items() if kk != 'lexeme'}
        else:
            rec['source_ref'] = {'ref': 'excluded_output_member', 'pointer': '/' + k}
        inv.append(rec)
    tfi = case.tok('/trusted_fixture_inputs') if '/trusted_fixture_inputs' in case.sr['tokens'] else case.absent('/trusted_fixture_inputs')
    return {'top_inventory': inv, 'trusted_fixture_inputs': {k: v for k, v in tfi.items() if k != 'lexeme'},
            'profile': case.tok('/profile'), 'contract': case.tok('/contract'), 'schema': case.tok('/schema'), 'role': case.tok('/role'),
            'inputs_canonical_sha256': case.sr['inputs_canonical_sha256'], 'log_final_newline': case.sr.get('log_final_newline', {}),
            'required_scopes': scopes, 'additional_channels': list(extra_channels)}

def family_status(claims):
    out = {}
    for fam in FAMILIES:
        cs = [c for c in claims if c['family'] == fam]
        assert cs, fam
        unmet = [c['claim_id'] for c in cs if c['status']['kind'] != 'DerivedNominal']
        out[fam] = {'status': 'DerivedNominal' if not unmet else 'RequiredUnmet', 'claim_ids': [c['claim_id'] for c in cs],
                    'unmet_claim_ids': unmet,
                    'decision_owners': sorted({c['status']['decision_owner'] for c in cs if c['status']['kind'] != 'DerivedNominal'})}
    return out

def scope(cid, ev, obj, label, status, premises, clauses, low=None, high=None, high_status=None, if_admitted=None, arg=None, note=None):
    r = {'scope_id': '%s/%s/%s/%s' % (cid, ev, obj, label), 'evaluation': ev, 'objective': obj, 'label': label,
         'status': status, 'premise_refs': premises, 'policy_clauses': clauses, 'argument_ref': arg,
         'expected_low': low, 'high': {'status': high_status or status, 'expected': high}, 'consequence_if_admitted': if_admitted}
    if note:
        r['note'] = note
    if status['kind'] == 'DerivedNominal':
        assert low is not None
    else:
        assert low is None and if_admitted is not None
    return r

def LOW(owned_units, U, seed, single, selected, mode, basis, independent='0', slots=(), credits=()):
    return {'owned_unit_origin_sets': owned_units, 'U': U, 'seed': seed, 'slots': list(slots),
            'single_floor': nano(single), 'independent_floor': nano(independent), 'credits': list(credits),
            'selected_low': nano(selected), 'credit_mode': mode, 'basis': basis}

def HIGH(kind, n, hist):
    return {'kind': kind, 'nano': n, 'basis_histogram': hist}

EMPTY_LOW = LOW([], [], [], '0', '0', 'none', 'none')
EMPTY_HIGH = HIGH('exact', '0', {})

def write(cid, rec):
    os.makedirs(os.path.join(HERE, 'dossier'), exist_ok=True)
    json.dump(rec, open(os.path.join(HERE, 'dossier', cid + '.json'), 'w'), indent=1, sort_keys=False)
    print('wrote', cid, len(rec['claims']), 'claims', len(rec['scope_lows']), 'scopes')

def header(case, standing_note):
    return {'schema': 'pi-accounting-oracle-dossier/0.1-proposal', 'case_id': case.cid,
            'author': {'role': 'independent oracle (slice1 oracle-batch1 worker)', 'owner_session': '01a1e569-2805-72ff-b837-03d1577c405d'},
            'standing': 'Independently authored finite derivation proposal; not consumer qualification; root binds/admits. ' + standing_note,
            'subject': {'manifest_sha256_prefix': '5e594cac', 'case_file': {'path': case.sr['file'], 'sha256': case.sr['file_sha256']},
                        'inputs_canonical_sha256': case.sr['inputs_canonical_sha256']},
            'policy_refs': POLICY_REFS}
