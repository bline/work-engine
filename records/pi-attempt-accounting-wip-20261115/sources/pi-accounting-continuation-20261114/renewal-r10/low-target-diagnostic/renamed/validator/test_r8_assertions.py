"""Actual CLI controls for named finite literals; not raw financial execution."""
import json,pathlib,subprocess,sys,tempfile,unittest
R=pathlib.Path(__file__).resolve().parents[1];V=R/'validator/validate_corpus.py'; C=R/'oracle/r8-controls'
class FiniteActualCLI(unittest.TestCase):
 def test_positive_and_self_consistent_wrong_records(self):
  for expected in sorted(C.glob('*.expected.json')):
   cid=expected.name[:-len('.expected.json')]
   if cid == 'DOLLAR-BAD-ORDER': continue
   raw=C/(cid+'.raw.json'); positive=C/(cid+'.result.json'); wrong=C/(cid+'.wrong.json')
   for result,code in [(positive,0),(wrong,1)]:
    p=subprocess.run([sys.executable,str(V),'assertions',str(raw),str(result),str(expected)],capture_output=True,text=True)
    self.assertEqual(p.returncode,code,cid+':'+p.stdout+p.stderr)
 def test_expected_cannot_masquerade_as_result(self):
  p=subprocess.run([sys.executable,str(V),'assertions',str(C/'H2.raw.json'),str(C/'H2.expected.json'),str(C/'H2.expected.json')],capture_output=True,text=True)
  self.assertEqual(p.returncode,1,p.stdout);self.assertIn('kind interchange',p.stdout)
 def test_wrong_identity_maps_with_same_money(self):
  expected=C/'H2.expected.json'; data=json.loads((C/'H2.result.json').read_text());data['units'][0]['origins']=['s'];data['units'][1]['origins']=['r']
  with tempfile.TemporaryDirectory() as td:
   path=pathlib.Path(td)/'wrong.json';path.write_text(json.dumps(data))
   p=subprocess.run([sys.executable,str(V),'assertions',str(C/'H2.raw.json'),str(path),str(expected)],capture_output=True,text=True)
   self.assertEqual(p.returncode,1,p.stdout)
 def test_raw_change_invalidates_claim(self):
  with tempfile.TemporaryDirectory() as td:
   path=pathlib.Path(td)/'changed.json';path.write_bytes((C/'L1.raw.json').read_bytes()+b' ')
   p=subprocess.run([sys.executable,str(V),'assertions',str(path),str(C/'L1.result.json'),str(C/'L1.expected.json')],capture_output=True,text=True)
   self.assertEqual(p.returncode,1,p.stdout);self.assertIn('binding changed',p.stdout)
if __name__=='__main__':unittest.main()
