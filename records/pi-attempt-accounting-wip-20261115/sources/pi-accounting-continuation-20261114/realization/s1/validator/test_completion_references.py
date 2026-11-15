"""Adversarial controls through the actual Python and Rust finite consumers."""
import copy, importlib.util, json, os, pathlib, shutil, subprocess, tempfile, unittest
ROOT=pathlib.Path(__file__).resolve().parents[1]
BIN=os.environ.get('SPEC_CHECK_BINARY','/home/bline/cbm-work/pi-accounting-continuation-20261114/build/completion-integration/debug/pi-accounting-spec-check')
CID='P130-R02-rounded-up-charge-admitted'
class CompletionReferences(unittest.TestCase):
 @classmethod
 def setUpClass(cls):
  cls.temp=tempfile.TemporaryDirectory();cls.root=pathlib.Path(cls.temp.name)/'s1'
  shutil.copytree(ROOT,cls.root,ignore=shutil.ignore_patterns('__pycache__','target'))
  cls.casepath=cls.root/'corpus/cases'/f'{CID}.json';cls.bundlepath=cls.root/'oracle/r8-completion-A'/f'{CID}.bundle.json'
  cls.case=json.loads(cls.casepath.read_text());cls.bundle=json.loads(cls.bundlepath.read_text())
  spec=importlib.util.spec_from_file_location('completion_validator',cls.root/'validator/validate_corpus.py');cls.validator=importlib.util.module_from_spec(spec);spec.loader.exec_module(cls.validator)
 @classmethod
 def tearDownClass(cls):cls.temp.cleanup()
 def run_candidate(self,case,bundle):
  self.casepath.write_text(json.dumps(case));self.bundlepath.write_text(json.dumps(bundle))
  failures=self.validator.Failures();self.validator.check_accounting_references(case,failures)
  rust=subprocess.run([BIN,'corpus',str(self.root),'A'],capture_output=True,text=True)
  return failures,rust
 def test_both_actual_consumers(self):
  controls={
   'missing_attempt_ref':lambda c,b:c['expected']['attempts'][0].pop('accounting_result_ref'),
   'dangling_result_ref':lambda c,b:c['expected']['attempts'][0].update(accounting_result_ref='missing#result:absent'),
   'interchanged_result_ref':lambda c,b:c['expected']['attempts'][0].update(accounting_result_ref=f'oracle/r8-completion-A/{CID}.bundle.json#input:{CID}'),
   'wrong_result_kind':lambda c,b:b['documents'][1].update(kind='accounting_expected_assertion'),
   'wrong_subject':lambda c,b:b['documents'][1].update(lineage_id='another-case'),
   'raw_digest':lambda c,b:b['documents'][0]['origins'][0].update(source_revision_ref='sha256:'+'0'*64),
   'raw_bytes':lambda c,b:b['documents'][0]['origins'][0].update(raw_bytes_sha256='0'*64),
   'missing_unit':lambda c,b:b['documents'][1].update(units=[]),
   'switched_unit':lambda c,b:b['documents'][1]['units'][0].update(unit_id='different-unit'),
   'missing_zero_scope_ref':lambda c,b:c['evaluations'][0]['expected']['objectives'][0]['owned_outstanding'].pop('supported_low_evidence_ref'),
   'wrong_scope':lambda c,b:c['evaluations'][0]['expected']['objectives'][0]['owned_closed'].update(supported_low_evidence_ref=f'oracle/r8-completion-A/{CID}.bundle.json#{CID}/default/o/owned_outstanding'),
   'missing_zero_scope':lambda c,b:b['documents'][1]['low_rollups'].pop(),
   'missing_F':lambda c,b:b['documents'][1].update(coverage_decisions=[]),
   'missing_valuation':lambda c,b:b['documents'][1].update(valuation_records=[]),
   'wrong_valuation_wire':lambda c,b:b['documents'][1]['valuation_records'][0].update(wire_ref='case:other#valuation'),
   'invalid_C':lambda c,b:b['documents'][1]['primary_validities'][0].update(validity_result='rejected'),
  }
  def wrong_low(c,b):
   c['expected']['attempts'][0]['exposure']['low']['nano']='0'
   c['evaluations'][0]['expected']['objectives'][0]['owned_closed']['low']='0'
   b['documents'][1]['low_rollups'][0].update(selected_low='0',single_floor='0',basis='none')
  controls['consistent_wrong_attempt_result_rollup']=wrong_low
  def switched_owner(c,b):
   c['evaluations'][0]['expected']['attribution'][0].update(ownership='unowned',objective=None)
  controls['switched_owner_same_money']=switched_owner
  failures,rust=self.run_candidate(copy.deepcopy(self.case),copy.deepcopy(self.bundle))
  self.assertFalse(failures,failures);self.assertEqual(rust.returncode,0,rust.stderr)
  for name,mutate in controls.items():
   with self.subTest(name=name):
    c,b=copy.deepcopy(self.case),copy.deepcopy(self.bundle);mutate(c,b)
    f,r=self.run_candidate(c,b);self.assertTrue(f,name);self.assertNotEqual(r.returncode,0,name+':'+r.stdout+r.stderr)
  self.run_candidate(self.case,self.bundle)
if __name__=='__main__':unittest.main()
