"""Join regression (JOIN-REMEDIATION Step 1.4): every bundle changed by the slice-2a oracle and the R11 private
admission-namespace control must be accepted by the actual joined Rust `evidence` command, and the R11 control must
also qualify through the Python route with root admissions bound from outside the subject (R11_ROOT_ADMISSIONS).

The binary comes from SPEC_CHECK_BINARY (or R10_EVIDENCE_EXECUTABLE), which replay_gates.sh exports; a missing
binary is a failure, never a skip.
"""
import json, os, subprocess, sys, unittest
from pathlib import Path

HERE = Path(__file__).resolve().parent
SUBJECT = HERE.parent
sys.path.insert(0, str(HERE))
import validate_corpus as v  # noqa: E402

CHANGED = [f'oracle/r8-completion-B/{c}.bundle.json' for c in (
    'VC08-subscription-measured-request', 'U06-subscription-upstream-error-unlabelled', 'V03-control-labelled-upstream-error',
    'V03-subscription-upstream-error-labelled-api', 'N03-contradicted-admission', 'N03-control-unlabelled-admission')] + \
    [f'oracle/r8-completion-A/{c}.bundle.json' for c in (
        'P130-R08-admissions-without-tree-still-block', 'P130-R12-untrusted-clock-keeps-candidate',
        'P133-A04-cache-counts-both-absent', 'P133-A04-cache-write-absent-legacy')]
CONTROL = 'oracle/r11-controls/admission-namespace/bundle.json'


def binary():
    b = os.environ.get('SPEC_CHECK_BINARY') or os.environ.get('R10_EVIDENCE_EXECUTABLE')
    if not b or not Path(b).is_file(): raise AssertionError('SPEC_CHECK_BINARY (joined pi-accounting-spec-check) is required')
    return b


class JoinedRustEvidence(unittest.TestCase):
    def test_changed_bundles_and_control_accepted(self):
        b = binary()
        for rel in CHANGED[:6] + [CONTROL]:
            run = subprocess.run([b, 'evidence', str(SUBJECT), str(SUBJECT / rel)], capture_output=True, text=True)
            self.assertEqual(run.returncode, 0, f'{rel}: {run.stdout.strip()} {run.stderr.strip()}')
            self.assertIn('structurally_valid', run.stdout + run.stderr, rel)

    def test_low_inventory_bundles_introduce_no_new_rejection(self):
        # The four R8-LOW-INVENTORY-01 bundles were already rejected at base 2729107 by the slice-1b accounting schema
        # (missing primary_memberships etc.: the full-S1 bundle-shape gap, out of slice-2 scope). The r11 change
        # (candidate origin added to alternative_origin_refs) must not change the joined evidence verdict.
        import tempfile
        b = binary()
        for rel in CHANGED[6:]:
            base = subprocess.run(['git', '-C', str(SUBJECT), 'show', f'2729107:{rel}'], capture_output=True, text=True, check=True).stdout
            with tempfile.NamedTemporaryFile('w', suffix='.bundle.json', delete=False) as f: f.write(base)
            try:
                old = subprocess.run([b, 'evidence', str(SUBJECT), f.name], capture_output=True, text=True)
                new = subprocess.run([b, 'evidence', str(SUBJECT), str(SUBJECT / rel)], capture_output=True, text=True)
            finally: os.unlink(f.name)
            self.assertEqual(old.returncode, new.returncode, rel)
            self.assertEqual((old.stdout + old.stderr).split(':')[0], (new.stdout + new.stderr).split(':')[0], rel)

    def test_corrected_historical_contracts_are_unestablished(self):
        for rel in CHANGED[:6]:
            inp = json.loads((SUBJECT / rel).read_text())['documents'][0]
            self.assertEqual([c['reference_namespace'] for c in inp['initial_contracts']], ['unestablished'], rel)
            self.assertEqual(inp['grants'], [], rel)


class PythonNamespacePredicate(unittest.TestCase):
    def test_admission_contract_without_grant_rejected_dollar_ticket_still_rejected(self):
        cid = 'VC08-subscription-measured-request'
        c = json.loads((SUBJECT / f'corpus/cases/{cid}.json').read_text())
        inp = json.loads((SUBJECT / CHANGED[0]).read_text())['documents'][0]
        F = v.Failures(); v.namespace_application(c, inp, F); self.assertEqual(list(F), [])     # corrected unestablished form
        for ns, tag in (('admission', 'without grant_ref/scope_id'), ('dollar_ticket', 'does not apply to')):
            w = json.loads(json.dumps(inp)); w['initial_contracts'][0]['reference_namespace'] = ns
            F = v.Failures(); v.namespace_application(c, w, F)
            self.assertTrue([f for f in F if tag in f and 'R8-NAMESPACE-01' in f], (ns, list(F)))


class PythonControlRoute(unittest.TestCase):
    def test_control_qualifies_only_with_root_admission_bound_from_outside(self):
        ctl = SUBJECT / 'oracle/r11-controls/admission-namespace'
        c = json.loads((ctl / 'case.json').read_text()); bundle = json.loads((ctl / 'bundle.json').read_text())
        path = os.environ.get('R11_ROOT_ADMISSIONS')
        self.assertTrue(path and Path(path).is_file() and not Path(path).resolve().is_relative_to(SUBJECT.resolve()),
                        'R11_ROOT_ADMISSIONS must name the root admissions file outside the subject')
        F = v.Failures(); v.admission_namespace_control(c, bundle, path, F)
        self.assertEqual(list(F), [])
        F = v.Failures(); v.admission_namespace_control(c, bundle, None, F)
        self.assertTrue([f for f in F if 'REQUIRED/UNMET' in f])


if __name__ == '__main__':
    unittest.main()
