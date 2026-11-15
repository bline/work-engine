"""Actual dependency impact/digest mutation controls; no duplicate implementation."""
import json,os,pathlib,shutil,subprocess,tempfile,unittest
R=pathlib.Path(__file__).resolve().parents[1]
BIN=os.environ.get('SPEC_CHECK_BINARY','/home/bline/cbm-work/pi-accounting-continuation-20261114/build/spec/debug/pi-accounting-spec-check')
PROTECTED=['USAGE-PRESENCE','RAW-FIELD-PREDICATES','INITIAL-REFERENCE','ATOMIZATION','FINANCIAL-CANDIDATES','DEFINITE-LINK','SUPPORTED-LOW','WHOLE-COVERAGE','CONTINUITY','EXACT-WIRE']
REQUIRED={'python-validator','independent-raw-assertions','consumer-R','view-contract','corpus-wire','oracle-r8-low-assertions','python-corpus-reference-consumer','rust-corpus-reference-consumer','independent-completion-shards'}
def impact(root,*nodes):return json.loads(subprocess.check_output([BIN,'impact',str(root),*nodes],text=True))
class ActualInvalidation(unittest.TestCase):
 def test_actual_registered_impact_and_digest_invalidation(self):
  baseline=impact(R)
  state=json.loads(subprocess.check_output([BIN,'check',str(R)],text=True))
  self.assertTrue(REQUIRED<=set(state['unestablished_consumers']),'actual command must retain null consumers as unestablished')
  with tempfile.TemporaryDirectory() as td:
   tmp=pathlib.Path(td);shutil.copytree(R/'spec',tmp/'spec');original=json.loads((tmp/'spec/ck.json').read_text())
   deps=json.loads((tmp/'spec/dependencies.json').read_text())
   self.assertTrue(all(x['acknowledgement'] is None for x in deps['consumers'] if x['id'] in REQUIRED),'null acknowledgements remain unestablished')
   mutations=[(node,'allow an unproved '+node+' inference') for node in PROTECTED]+[('RAW-FIELD-PREDICATES','treat missing billing count as zero'),('RAW-FIELD-PREDICATES','collapse two distinct reservation rows by normalized JSON'),('RAW-FIELD-PREDICATES','coerce numeric model7 to a priced model string')]
   for node,semantic_change in mutations:
    data=json.loads(json.dumps(original));entry=next(x for values in data.values() if isinstance(values,list) for x in values if isinstance(x,dict) and x.get('id')==node)
    entry['text']+=' COUNTERFACTUAL POLICY MUTATION: '+semantic_change
    (tmp/'spec/ck.json').write_text(json.dumps(data))
    reached=set(impact(tmp,node)['affected_registered_consumers']);self.assertTrue(REQUIRED<=reached,(node,sorted(REQUIRED-reached)))
    changed=impact(tmp)
    for consumer in REQUIRED:self.assertNotEqual(baseline[consumer],changed[consumer],(node,consumer))
    print(node,'actual consumers invalidated',','.join(sorted(REQUIRED)))
if __name__=='__main__':unittest.main()
