"""Actual evidence CLI and standard Draft2020-12 controls for fixed VC07 records."""
import copy,json,os,pathlib,subprocess,tempfile,unittest
from jsonschema import Draft202012Validator
R=pathlib.Path(__file__).resolve().parents[1];D=R/'oracle/r8-controls'
BIN=os.environ.get('SPEC_CHECK_BINARY','/home/bline/cbm-work/pi-accounting-continuation-20261114/build/spec/debug/pi-accounting-spec-check')
class ExactCoverageInterfaces(unittest.TestCase):
 def cli(self,path):return subprocess.run([BIN,'evidence',str(R),str(path)],capture_output=True,text=True)
 def test_actual_positive_and_relabel_negatives(self):
  for filename,exitcode in [('VC07-unlinked.bundle.json',0),('VC07-conditional.bundle.json',0),('VC07-missing-completion.wrong.json',1),('VC07-relabel-dollar.wrong.json',1),('VC07-relabel-dollar-hidden.wrong.json',1),('VC07-unlinked-release.wrong.json',1)]:
   p=self.cli(D/filename);print(filename,'exit',p.returncode,(p.stdout+p.stderr).strip()[:250]);self.assertEqual(p.returncode,exitcode,filename+':'+p.stdout+p.stderr)
 def test_erased_dollar_primary_rejected_by_actual_raw_assertion(self):
  p=subprocess.run(['python3',str(R/'validator/validate_corpus.py'),'assertions',str(D/'DOLLAR-BAD-ORDER.raw.json'),str(D/'DOLLAR-BAD-ORDER-erased-primary.wrong.json'),str(D/'DOLLAR-BAD-ORDER.expected.json')],capture_output=True,text=True)
  self.assertEqual(p.returncode,1,p.stdout+p.stderr);self.assertIn('independent exact assertion failed',p.stdout)
 def test_standard_schema_and_rust_shape_agreement(self):
  schema=json.loads((R/'spec/accounting.schema.json').read_text());Draft202012Validator.check_schema(schema);v=Draft202012Validator(schema)
  positive=json.loads((D/'VC07-conditional.bundle.json').read_text())
  for doc in positive['documents']:self.assertFalse(list(v.iter_errors(doc)))
  for mode in ('kind_interchange','unknown_authority'):
   wrong=copy.deepcopy(positive)
   if mode=='kind_interchange':wrong['documents'][1]['kind']='accounting_input_proof'
   else:wrong['documents'][0]['verified']=True
   self.assertTrue(list(v.iter_errors(wrong['documents'][1 if mode=='kind_interchange' else 0])))
   with tempfile.TemporaryDirectory() as td:
    p=pathlib.Path(td)/'wrong.json';p.write_text(json.dumps(wrong));self.assertEqual(self.cli(p).returncode,1)
if __name__=='__main__':unittest.main()
