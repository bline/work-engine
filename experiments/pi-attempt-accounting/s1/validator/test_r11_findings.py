"""Slice 2a (r11) oracle-side finding controls at the actual Python consumer.

Each test runs the real validate_corpus entry points (check_case / check_accounting_references) on the current
subject, or on an isolated temporary copy for supplied-bundle mutants. Each negative keeps raw inputs unchanged
(except identity-injection controls, which change exactly one raw field) and must fail for its named predicate tag.
Root-owned admissions are bound from outside the subject via R11_ROOT_ADMISSIONS (path), never from the subject.
"""
import copy, json, os, shutil, sys, tempfile, unittest
from pathlib import Path

HERE = Path(__file__).resolve().parent
SUBJECT = HERE.parent
sys.path.insert(0, str(HERE))
import validate_corpus as v  # noqa: E402

RATES = v.load_rates(str(SUBJECT / 'oracle/rates'))


def case(cid, base='corpus/cases'):
    return json.loads((SUBJECT / base / f'{cid}.json').read_text())


def check(c):
    F = v.Failures(); v.check_case(c, RATES, F); return list(F)


def tagged(F, tag): return [f for f in F if tag in f]


class SubjectCopy:
    """Isolated subject for supplied-bundle mutants; the actual consumer resolves bundles through r9_pilot.root()."""
    def __enter__(self):
        self.tmp = tempfile.mkdtemp(prefix='r11-subject-')
        for d in ('oracle', 'corpus', 'spec', 'validator'):
            shutil.copytree(SUBJECT / d, Path(self.tmp) / d, ignore=shutil.ignore_patterns('__pycache__'))
        self.old = os.environ.get('R9_QUALIFICATION_SUBJECT'); os.environ['R9_QUALIFICATION_SUBJECT'] = self.tmp
        return Path(self.tmp)

    def __exit__(self, *a):
        if self.old is None: os.environ.pop('R9_QUALIFICATION_SUBJECT', None)
        else: os.environ['R9_QUALIFICATION_SUBJECT'] = self.old
        shutil.rmtree(self.tmp)


def refs(c):
    F = v.Failures(); v.check_accounting_references(c, F); return list(F)


def edit_json(path, fn, compact=True):
    d = json.loads(path.read_text()); fn(d)
    path.write_text((json.dumps(d, sort_keys=True) if compact else json.dumps(d, indent=2, sort_keys=True)) + '\n')


class CorpusPositive(unittest.TestCase):
    def test_all_175_cases_pass_check_case_with_new_predicates(self):
        F = v.Failures()
        cases = v.corpus_cases(str(SUBJECT / 'corpus'))
        for c in cases: v.check_case(c, RATES, F)
        self.assertEqual(len(cases), 175)
        self.assertEqual(list(F), [])

    def test_no_r11_reference_predicate_fires_on_current_bundles(self):
        F = v.Failures()
        for c in v.corpus_cases(str(SUBJECT / 'corpus')): v.check_accounting_references(c, F)
        self.assertEqual([f for f in F if 'R8-' in f], [])


class Fin01(unittest.TestCase):
    def test_coherent_700_to_500_rewrite_rejected_for_raw_charge(self):
        c = case('P9-claimed-charge-above-reservation')
        self.assertEqual(tagged(check(c), 'R8-FIN-01'), [])
        a = c['expected']['attempts'][0]
        a['settlement']['charged'] = '500000000'; a['exposure']['high']['nano'] = '500000000'
        for e in c['evaluations']:
            o = e['expected']['objectives'][0]
            o['owned_outstanding']['high']['nano'] = '500000000'; o['commitments'][0]['committed'] = '500000000'
        F = check(c)
        self.assertTrue(tagged(F, 'retained raw charge 700000000 (R8-FIN-01)'), F)


    def test_non_pilot_coherent_charge_rewrite_rejected(self):
        # P130-R02 is outside the R10 pilot registration: the rounded-up raw charge 1000 nano is the retained high.
        c = case('P130-R02-rounded-up-charge-admitted')
        self.assertEqual(check(c), [])
        a = c['expected']['attempts'][0]
        a['settlement']['charged'] = '525'; a['exposure']['high'] = {'basis': 'usage_valuation', 'kind': 'exact', 'nano': '525'}
        for e in c['evaluations']:
            for o in e['expected']['objectives']:
                if o['owned_closed']['high']['nano'] == '1000':
                    o['owned_closed']['high']['nano'] = '525'
                    for cm in o['commitments']: cm['committed'] = str(int(cm['committed']) + 475)
        F = check(c)
        self.assertTrue(tagged(F, 'retained raw charge 1000 (R8-FIN-01)'), F)


class Identity01(unittest.TestCase):
    def test_model7_conservative_controls_accepted_and_favourable_outputs_rejected(self):
        for f in ('model7-VC01-openai-usage-settlement', 'model7-A04-undated-settlement-between-dated-neighbours'):
            c = case(f, 'oracle/r9-pilot/cases')
            self.assertEqual(tagged(check(c), 'R8-IDENTITY-01'), [], f)
            w = copy.deepcopy(c); a = w['expected']['attempts'][0]
            a['valuation'] = {'state': 'valued', 'identities': ['openai/gpt-6-astra'], 'low': '15000000', 'high': '15000000', 'complete': True, 'derivations': []}
            self.assertTrue(tagged(check(w), 'R8-IDENTITY-01'), f)
            w2 = copy.deepcopy(c); w2['expected']['attempts'][0]['valuation']['supported_low'] = '15000000'
            self.assertTrue(tagged(check(w2), 'R8-IDENTITY-01'), f)

    def test_injected_malformed_identity_with_unchanged_favourable_outputs_rejected(self):
        base = case('P132-FG-VC15-gated')
        for bad in ('', True, {}, ' gpt-6-astra', 7):
            c = copy.deepcopy(base)
            row = json.loads(c['inputs']['sessions'][0]['logs']['turn-1']['lines'][1]); row['response_model'] = bad
            c['inputs']['sessions'][0]['logs']['turn-1']['lines'][1] = json.dumps(row)
            self.assertTrue(tagged(check(c), 'R8-IDENTITY-01'), repr(bad))

    def test_priced_multiidentity_stays_finite(self):
        self.assertEqual(check(case('V01-usage-settlement-other-response-model')), [])


class Reservation01(unittest.TestCase):
    def test_two_byte_distinct_equal_reservations_cannot_be_unique(self):
        c = case('equal-reservations', 'oracle/r9-pilot/cases')
        self.assertEqual(check(c), [])
        w = copy.deepcopy(c)
        a = next(x for x in w['expected']['attempts'] if len([r for r in x['evidence']]) >= 3)
        a['settlement'] = {'decision': 'valid', 'basis': 'usage', 'reasons': [], 'charged': a['settlement'].get('charged', '15000000'), 'reserved': a['settlement'].get('reserved', '500000000')}
        F = check(w)
        self.assertTrue(tagged(F, 'valid settlement without UniqueReservation (P3)'), F)


class GateIdentity01(unittest.TestCase):
    def mutate(self, value):
        c = case('P132-FG-VC15-gated')
        targets = [(c['evaluations'][0]['objectives'][0]['admission_authority'], 'id'), (c['evaluations'][0]['objectives'][1]['admission_authority'], 'id'),
                   (c['inputs']['gates'][0]['authority'], 'id'), (c['trusted_fixture_inputs']['gate_verifications']['g-S1'], 'authority_id')]
        for d, k in targets:
            if value is None: d.pop(k, None)
            else: d[k] = value
        g = c['inputs']['gates'][0]
        c['trusted_fixture_inputs']['gate_verifications']['g-S1']['payload_sha256'] = v.hashlib.sha256(json.dumps(g, sort_keys=True, separators=(',', ':'), ensure_ascii=False).encode()).hexdigest()
        return c

    def test_positive_and_triple_absent_empty_rejected(self):
        self.assertEqual(check(case('P132-FG-VC15-gated')), [])
        for value in (None, '', ' '):
            c = self.mutate(value)
            o = c['evaluations'][0]['objectives'][0]
            q, reasons, _ = v.gate_qualification(c, o, o['admissions'][0], 'release', c['inputs']['reference_time'], {}, {s['session_id']: s for s in c['inputs']['sessions']})
            self.assertFalse(q, repr(value)); self.assertIn('GateAuthorityMismatch', reasons)
            F = check(c)   # supplied commitment still claims qualified/retired: the actual check path rejects it
            self.assertTrue([f for f in F if 'commitment' in f or 'gate' in f.lower()], (value, F))


class LowInventory01(unittest.TestCase):
    SCOPES = [('P130-R08-admissions-without-tree-still-block', 'default/o/owned_closed'), ('P130-R08-admissions-without-tree-still-block', 'default/p/owned_closed'),
              ('P130-R12-untrusted-clock-keeps-candidate', 'default/o/owned_closed'), ('P133-A04-cache-counts-both-absent', 'default/o/owned_closed'),
              ('P133-A04-cache-write-absent-legacy', 'default/o/owned_closed')]

    def test_jointly_omitted_U_rejected_and_candidate_seed_rejected(self):
        for cid, scope in self.SCOPES:
            c = case(cid)
            self.assertEqual(tagged(refs(c), 'R8-LOW-INVENTORY-01'), [])
            for mode in ('omit', 'seed'):
                with SubjectCopy() as tmp:
                    def mut(d):
                        for doc in d['documents']:
                            for lr in doc.get('low_rollups', []):
                                if lr['scope_ref'].endswith(scope):
                                    if mode == 'omit': lr['alternative_origin_refs'] = []
                                    else: lr['seed_origin_refs'] = list(lr['alternative_origin_refs'])
                    edit_json(tmp / f'oracle/r8-completion-A/{cid}.bundle.json', mut)
                    def mut_claims(d):        # joint omission: the authored assertion agrees with the wrong bundle
                        for x in d['assertions']:
                            if x['claim_id'] == f'{cid}/{scope}':
                                if mode == 'omit': x['alternative_origin_refs'] = []
                                else: x['seed_origin_refs'] = list(x['alternative_origin_refs'])
                    edit_json(tmp / 'oracle/r8-completion-A/LOW-ASSERTIONS.json', mut_claims)
                    F = refs(c)
                    self.assertTrue(tagged(F, 'R8-LOW-INVENTORY-01'), (cid, scope, mode, F))


class Placement01(unittest.TestCase):
    def test_raw_straddle_and_before_window(self):
        c = case('P132-CP-closed-predecessor-outstanding')
        self.assertEqual(check(c), [])
        for e in c['evaluations']:
            e['expected']['attribution'][0]['placement'] = 'inside'
            o = next(x for x in e['expected']['objectives'] if x['objective'] == 'p'); o['reasons'] = ['UnenforcedCommitment', 'UnresolvedExposure']
        F = check(c)
        self.assertEqual(len(tagged(F, 'raw interval placement straddles_closure (R8-PLACEMENT-01)')), 2, F)
        t = case('P130-T-owned-before-window-summed')
        self.assertEqual(check(t), [])
        for e in t['evaluations']:
            for at in e['expected']['attribution']:
                if at['placement'] == 'before': at['placement'] = 'inside'
        self.assertTrue(tagged(check(t), 'R8-PLACEMENT-01'))


class Namespace01(unittest.TestCase):
    CIDS = ('VC08-subscription-measured-request', 'U06-subscription-upstream-error-unlabelled', 'V03-control-labelled-upstream-error',
            'V03-subscription-upstream-error-labelled-api', 'N03-contradicted-admission', 'N03-control-unlabelled-admission')

    def test_dollar_ticket_on_admission_origins_rejected_even_when_bundle_agrees(self):
        for cid in self.CIDS:
            c = case(cid)
            self.assertEqual(tagged(refs(c), 'R8-NAMESPACE-01'), [], cid)
            with SubjectCopy() as tmp:
                def mut(d):
                    inp = d['documents'][0]
                    inp['initial_contracts'] = [{'contract_id': 'conditional-ticket:S1/turn-1', 'duplicate_rule': 'same_origin_only', 'grant_ref': 'initial-scope:S1/turn-1',
                                                 'reference_namespace': 'dollar_ticket', 'scope_id': f'{cid}/S1/turn-1'}]
                    for o in inp['origins']: o['initial_contract_ref'] = 'conditional-ticket:S1/turn-1'
                    inp['continuity']['preserved_initial_contract_refs'] = ['conditional-ticket:S1/turn-1']
                edit_json(tmp / f'oracle/r8-completion-B/{cid}.bundle.json', mut)
                F = refs(c)
                self.assertTrue(tagged(F, 'R8-NAMESPACE-01'), (cid, F))

    def test_origin_base_split_and_private_admission_control(self):
        for cid in self.CIDS[:4]:
            self.assertEqual([a['evidence'] for a in case(cid)['expected']['attempts']], [['S1/turn-1:1'], ['S1/turn-1:2']], cid)
        ctl = SUBJECT / 'oracle/r11-controls/admission-namespace'
        c = json.loads((ctl / 'case.json').read_text()); b = json.loads((ctl / 'bundle.json').read_text())
        F = v.Failures(); v.namespace_application(c, b['documents'][0], F); self.assertEqual(list(F), [])
        # grouped single unit is structurally/legacy coherent; corpus low registration does not apply to private controls
        self.assertEqual([f for f in check(c) if 'independent low scope REQUIRED/UNMET' not in f], [])
        F = v.Failures(); v.admission_namespace_control(c, b, os.environ.get('R11_ROOT_ADMISSIONS'), F)
        decl = json.loads((ctl / 'declaration.json').read_text())
        admitted = os.environ.get('R11_ROOT_ADMISSIONS') and any(e.get('case_id') == c['case_id'] for e in json.loads(Path(os.environ['R11_ROOT_ADMISSIONS']).read_text())['entries'])
        if admitted: self.assertEqual(list(F), [])
        else: self.assertTrue(tagged(F, 'REQUIRED/UNMET'), F)
        # with an exact admission record supplied from outside the subject the grouping qualifies; a stale one does not
        with tempfile.TemporaryDirectory() as d:
            p = Path(d) / 'admissions.json'
            p.write_text(json.dumps({'entries': [dict(decl, entry_id='ADM-proposed')]}))
            F = v.Failures(); v.admission_namespace_control(c, b, str(p), F); self.assertEqual(list(F), [])
            stale = dict(decl, entry_id='ADM-stale', covered_origins=[{'origin_id': 'S1/turn-1:1', 'raw_utf8_sha256': '0' * 64}])
            p.write_text(json.dumps({'entries': [stale]}))
            F = v.Failures(); v.admission_namespace_control(c, b, str(p), F); self.assertTrue(F)


if __name__ == '__main__':
    unittest.main()
