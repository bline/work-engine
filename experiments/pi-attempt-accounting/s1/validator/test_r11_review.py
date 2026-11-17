"""Review-r2 remediation controls (oracle side: D-1, D-2 Python, D-3, D-4, D-5) at the actual Python consumer.

Each counterexample from slice2/review-r2/REVIEW.md must fail for its named predicate; positives still pass. The only admission
source is R11_ROOT_ADMISSIONS (outside the subject); the D-1 counterexample is checked with and without it.
"""
import copy, json, os, sys, unittest
from pathlib import Path

HERE = Path(__file__).resolve().parent
SUBJECT = HERE.parent
sys.path.insert(0, str(HERE))
import validate_corpus as v  # noqa: E402
from test_r11_findings import SubjectCopy, case, check, refs, tagged  # noqa: E402

VC08 = 'VC08-subscription-measured-request'
CTL = SUBJECT / 'oracle/r11-controls/admission-namespace'


def self_declared_vc08(tmp, unestablished=False):
    """Reviewer p1/p15 shape: historical VC08 raw bytes, ONE grouped unit over both origins, with a subject-declared
    admission scope + grant (p1) or an unestablished contract and no grant (p15). Built from the private control (same raw)."""
    old = 'R11-CTL-admission-namespace-grouped'
    c = json.loads((CTL / 'case.json').read_text().replace(old, VC08).replace('oracle/r11-controls/admission-namespace/bundle.json', f'oracle/r8-completion-B/{VC08}.bundle.json'))
    b = json.loads((CTL / 'bundle.json').read_text().replace(old, VC08).replace('oracle/r11-controls/admission-namespace/bundle.json', f'oracle/r8-completion-B/{VC08}.bundle.json')
                   .replace('oracle/r11-controls/admission-namespace/provenance.json', f'oracle/r8-completion-B/{VC08}.provenance.json'))
    p = json.loads((CTL / 'provenance.json').read_text().replace(old, VC08))
    p['authority_ref'] = b['documents'][0]['grants'][0]['authority_ref'] = 'self-declared-by-subject'
    if unestablished:
        inp = b['documents'][0]
        inp['initial_contracts'] = [{'contract_id': 'unestablished-scope:S1/turn-1', 'duplicate_rule': 'same_origin_only', 'reference_namespace': 'unestablished'}]
        for o in inp['origins']: o['initial_contract_ref'] = 'unestablished-scope:S1/turn-1'
        inp['continuity']['preserved_initial_contract_refs'] = ['unestablished-scope:S1/turn-1']; inp['grants'] = []
    (tmp / f'oracle/r8-completion-B/{VC08}.bundle.json').write_text(json.dumps(b, sort_keys=True) + '\n')
    (tmp / f'oracle/r8-completion-B/{VC08}.provenance.json').write_text(json.dumps(p, indent=2, sort_keys=True) + '\n')
    return c


class EnvAdmissions:
    def __init__(self, value): self.value = value
    def __enter__(self):
        self.old = os.environ.get('R11_ROOT_ADMISSIONS')
        if self.value is None: os.environ.pop('R11_ROOT_ADMISSIONS', None)
        else: os.environ['R11_ROOT_ADMISSIONS'] = self.value
    def __exit__(self, *a):
        if self.old is None: os.environ.pop('R11_ROOT_ADMISSIONS', None)
        else: os.environ['R11_ROOT_ADMISSIONS'] = self.old


class D1AdmissionAuthority(unittest.TestCase):
    def test_self_declared_admission_scope_is_unmet_with_or_without_root_admissions(self):
        root = os.environ.get('R11_ROOT_ADMISSIONS')
        for value in (None, root):
            with EnvAdmissions(value), SubjectCopy() as tmp:
                c = self_declared_vc08(tmp)
                F = refs(c)
                self.assertTrue(tagged(F, 'REQUIRED/UNMET: no exact root admission (R8-NAMESPACE-01)'), (value, F))

    def test_subject_supplied_admissions_file_is_not_an_admission_source(self):
        with SubjectCopy() as tmp:
            c = self_declared_vc08(tmp)
            forged = tmp / 'oracle/ROOT-ADMISSIONS.json'
            decl = json.loads((CTL / 'declaration.json').read_text()); decl['case_id'] = VC08
            forged.write_text(json.dumps({'entries': [decl]}))
            with EnvAdmissions(str(forged)):
                self.assertTrue(tagged(refs(c), 'REQUIRED/UNMET'))

    def test_positives(self):
        self.assertEqual(tagged(refs(case(VC08)), 'R8-NAMESPACE-01'), [])
        root = os.environ.get('R11_ROOT_ADMISSIONS')
        if root:
            c = json.loads((CTL / 'case.json').read_text()); b = json.loads((CTL / 'bundle.json').read_text())
            F = v.Failures(); v.admission_namespace_control(c, b, root, F); self.assertEqual(list(F), [])


class D2Grouping(unittest.TestCase):
    def test_grouping_without_scope_or_link_rejected(self):
        with SubjectCopy() as tmp:
            c = self_declared_vc08(tmp, unestablished=True)
            F = refs(c)
            self.assertTrue(tagged(F, 'without an admitted scope or exact link/identity binding; BASE 3 OriginBase per origin (review-r2 D-2)'), F)

    def test_current_bundles_have_no_ungrounded_grouping(self):
        F = v.Failures()
        for c in v.corpus_cases(str(SUBJECT / 'corpus')): v.check_accounting_references(c, F)
        self.assertEqual(tagged(list(F), 'review-r2 D-2'), [])


class D3NullIdentity(unittest.TestCase):
    def test_null_identity_with_favourable_outputs_rejected(self):
        base = case('P132-FG-VC15-gated')
        for field in ('response_model', 'model', 'authorized_model'):
            c = copy.deepcopy(base)
            row = json.loads(c['inputs']['sessions'][0]['logs']['turn-1']['lines'][1]); row[field] = None
            c['inputs']['sessions'][0]['logs']['turn-1']['lines'][1] = json.dumps(row)
            self.assertTrue(tagged(check(c), 'R8-IDENTITY-01'), field)

    def test_absent_identity_member_keeps_v1_handling(self):
        c = case('P132-FG-VC15-gated')
        row = json.loads(c['inputs']['sessions'][0]['logs']['turn-1']['lines'][1]); del row['response_model']
        c['inputs']['sessions'][0]['logs']['turn-1']['lines'][1] = json.dumps(row)
        self.assertEqual(tagged(check(c), 'R8-IDENTITY-01'), [])


class D4ClassifierAgreement(unittest.TestCase):
    def test_python_classifier_reproduces_shared_census(self):
        census = json.loads((SUBJECT / 'oracle/r11-controls/raw-namespace-census.json').read_text())['rows']
        corpus = {}
        for c in v.corpus_cases(str(SUBJECT / 'corpus')):
            lines, _ = v.source_lines(c)
            for oid, raw in lines.items():
                if isinstance(raw, str): corpus[(c['case_id'], oid)] = raw
        self.assertEqual({(r['case_id'], r['origin_id']) for r in census if r['case_id'] != 'synthetic'}, set(corpus))
        for r in census:
            if r['case_id'] != 'synthetic': self.assertEqual(r['raw'], corpus[(r['case_id'], r['origin_id'])])
            self.assertEqual(v.raw_reference_namespace(v.parse(r['raw'])), r['class'], (r['case_id'], r['origin_id']))

    def test_admission_contract_over_settlement_usd_rid_row_rejected(self):
        c = case('P9-claimed-charge-above-reservation')
        inp = {'initial_contracts': [{'contract_id': 'x', 'reference_namespace': 'admission', 'duplicate_rule': 'same_origin_only', 'grant_ref': 'g', 'scope_id': 's'}],
               'origins': [{'origin_id': 'S1/turn-1:2', 'initial_contract_ref': 'x'}]}
        F = v.Failures(); v.namespace_application(c, inp, F)
        self.assertTrue(tagged(list(F), 'raw namespace dollar_ticket'), list(F))


class D5DanglingContract(unittest.TestCase):
    def test_dangling_initial_contract_ref_rejected(self):
        with SubjectCopy() as tmp:
            p = tmp / f'oracle/r8-completion-B/{VC08}.bundle.json'
            b = json.loads(p.read_text())
            for o in b['documents'][0]['origins']: o['initial_contract_ref'] = 'nope'
            p.write_text(json.dumps(b, sort_keys=True) + '\n')
            F = refs(case(VC08))
            self.assertTrue(tagged(F, "dangling initial_contract_ref 'nope'"), F)


if __name__ == '__main__':
    unittest.main()
