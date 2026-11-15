"""Decisive controls through the actual composed Python finite check consumer."""
import contextlib,copy,io,json,os,shutil,tempfile,unittest,warnings
warnings.filterwarnings("ignore",category=ResourceWarning)
from pathlib import Path
import validate_corpus as v
import r9_pilot as q
SOURCE=Path(__file__).resolve().parent.parent
class Pilot(unittest.TestCase):
 def setUp(self):
  warnings.filterwarnings("ignore",category=ResourceWarning)
  self.tmp=tempfile.TemporaryDirectory(prefix='r9-python-');self.subject=Path(self.tmp.name)/'subject';shutil.copytree(SOURCE,self.subject,ignore=shutil.ignore_patterns('target','__pycache__'));os.environ['R9_QUALIFICATION_SUBJECT']=str(self.subject)
 def tearDown(self):self.tmp.cleanup();os.environ['R9_QUALIFICATION_SUBJECT']=str(SOURCE)
 def edit(self,rel,fn):
  p=self.subject/rel;d=json.loads(p.read_text());fn(d);p.write_text(json.dumps(d))
 def qualify(self):
  out=io.StringIO()
  with contextlib.redirect_stdout(out):code=v.main(['validator','check',str(self.subject/'corpus'),str(self.subject/'oracle/rates'),'--pilot'])
  return code,out.getvalue()
 def reject(self,cause):
  code,out=self.qualify();self.assertEqual(code,1,out);self.assertIn(cause,out);self.assertNotIn('schema:',out);self.assertNotIn('structure error',out);self.assertNotIn('binding changed',out)
 def test_correct_composed_positive(self):self.assertEqual(self.qualify()[0],0,self.qualify()[1])
 def test_p9_consistent_output_and_components(self):
  def mutate(c):
   a=c['expected']['attempts'][0];a['settlement']['charged']='500000000';a['exposure']['high']['nano']='500000000';o=c['evaluations'][0]['expected']['objectives'][0];o['owned_outstanding']['high']['nano']='500000000';o['commitments'][0]['committed']='500000000'
  self.edit('corpus/cases/P9-claimed-charge-above-reservation.json',mutate)
  self.edit('oracle/r8-completion-B/P9-claimed-charge-above-reservation.bundle.json',lambda b: next(x for x in b['documents'][1]['retained_components'] if x['role']=='charge').update(nano_usd='500000000'))
  self.reject('R9 P9 raw retained charge700M')
 def test_p9_matching_wrong_premises_and_components(self):
  self.edit('oracle/r9-pilot/claims.json',lambda d:next(x for x in d['claims'][0]['cases'][0]['components'] if x['kind']=='charge').update(nano='500000000'))
  self.edit('oracle/r8-completion-B/P9-claimed-charge-above-reservation.bundle.json',lambda b:next(x for x in b['documents'][1]['retained_components'] if x['role']=='charge').update(nano_usd='500000000'))
  self.reject('R9 independent component premises disagree with raw')
 def test_joint_primary_omission(self):
  def mutate(b):
   r=b['documents'][1];omitted=r['primary_memberships'][1]['obligation_id'];r['primary_memberships']=[m for m in r['primary_memberships'] if m['obligation_id']!=omitted];r['primary_validities']=[m for m in r['primary_validities'] if m['obligation_id']!=omitted];r['coverage_decisions']=[c for c in r['coverage_decisions'] if c.get('validity_ref')!=omitted]
  self.edit('oracle/r8-completion-B/VC11-multi-turn-session.bundle.json',mutate);self.reject('exhaustive raw primary membership')
 def test_old_model7_complete_eligible(self):
  original=json.loads((SOURCE/'corpus/cases/VC01-openai-usage-settlement.json').read_text())
  self.edit('oracle/r9-pilot/cases/model7-VC01-openai-usage-settlement.json',lambda c:(c.update(expected=copy.deepcopy(original['expected'])),c.__setitem__('evaluations',copy.deepcopy(original['evaluations']))))
  old=json.loads((SOURCE/'oracle/r8-completion-B/VC01-openai-usage-settlement.bundle.json').read_text())
  def mutate_bundle(b):
   memberships=b['documents'][1]['primary_memberships'];b['documents'][1]=copy.deepcopy(old['documents'][1]);b['documents'][1]['primary_memberships']=memberships
   result=b['documents'][1];component=result['coverage_decisions'][0]['retained_component_refs'][0];result['retained_components']=[{'component_id':component,'origin_ref':'S1/turn-1:2','role':'charge','nano_usd':'15000000'}];next(u for u in result['units'] if u['state']=='active')['retained_component_refs']=[component]
  self.edit('oracle/r9-pilot/cases/model7-VC01-openai-usage-settlement.bundle.json',mutate_bundle)
  self.reject('R9 model7 must retain supported15M')
 def test_equal_amount_not_unique(self):
  def mutate(c):
   a=c['expected']['attempts'][0];a['settlement'].update(decision='valid',basis='usage',reasons=[]);a['replacement']={'eligible':True,'reason':'UsageBound'}
  self.edit('oracle/r9-pilot/cases/equal-reservations.json',mutate);self.reject('R9 evidence cardinality2')
 def test_missing_registration(self):
  self.edit('oracle/r9-pilot/claims.json',lambda d:d['claims'].pop())
  with self.assertRaisesRegex(ValueError,'mandatory four-claim'):self.qualify()
 def test_duplicate_registration(self):
  self.edit('oracle/r9-pilot/claims.json',lambda d:d['claims'].__setitem__(1,copy.deepcopy(d['claims'][0])))
  with self.assertRaisesRegex(ValueError,'mandatory four-claim'):self.qualify()
 def test_unconsumed_registration(self):
  self.edit('oracle/r9-pilot/claims.json',lambda d:d['claims'][1]['cases'].pop())
  with self.assertRaisesRegex(ValueError,'case binding'):self.qualify()
 def test_same_origin_and_admitted_exact_copy_are_distinct_controls(self):
  b=json.loads((self.subject/'oracle/r8-completion-B/VC11-multi-turn-session.bundle.json').read_text());F=v.Failures();q.reference_relations(b,F,'same-origin');self.assertFalse(F,F)
  i,r=b['documents'];original=copy.deepcopy(i['origins'][0]);original['origin_id']='copy:reservation';original['initial_contract_ref']='copy-scope';i['origins'][0]['initial_contract_ref']='copy-scope';i['origins'].append(original);i['initial_contracts'].append({'contract_id':'copy-scope','duplicate_rule':'scope_exact_line','scope_id':'copy-extent','reference_namespace':'unestablished'})
  cl=r['primary_memberships'][0]['reservation_classes'][0];cl['origin_refs']=sorted(cl['origin_refs']+['copy:reservation'])
  # Without admission, even identical bytes at two original origins are distinct.
  F=v.Failures();q.reference_relations(b,F,'unadmitted-copy');self.assertIn('R9 multi-origin class lacks admitted exact-copy equivalence',' '.join(F))
  i['grants'].append({'grant_id':'copy-equivalence','guarantee_kind':'exact_copy_equivalence','covered_origin_refs':cl['origin_refs']})
  for u in r['units']:
   if 'S1/turn-1:1' in u['origin_refs']:u['origin_refs'].append('copy:reservation')
  for coverage in r['coverage_decisions']:
   if 'S1/turn-1:1' in coverage['monetary_origin_refs']:coverage['monetary_origin_refs'].append('copy:reservation')
  F=v.Failures();q.reference_relations(b,F,'admitted-copy');self.assertFalse(F,F)
 def test_python_exact_link_controls(self):
  bundle=json.loads((self.subject/'oracle/r8-controls/VC07-conditional.bundle.json').read_text());F=v.Failures();q.reference_relations(bundle,F,'positive');self.assertFalse(F,F)
  for kind in ['self','uncited','other-request','shared-grant-substitution']:
   b=copy.deepcopy(bundle);i,r=b['documents'];l=r['definite_links'][0]
   if kind=='self':l['to_origin_ref']='a'
   elif kind=='uncited':r['coverage_decisions'][0]['definite_link_refs']=[]
   elif kind=='other-request':l['to_origin_ref']='other';i['exact_links'][0]['to_origin_ref']='other';i['grants'][0]['admitted_payload']={'from':'a','to':'other'}
   else:
    i['grants'][0]['admitted_payload']={'pairs':[{'from':'a','to':'u'},{'from':'b','to':'v'}]};i['grants'][0]['covered_origin_refs']=['a','u','b','v'];i['exact_links'][0]['to_origin_ref']='v';l['to_origin_ref']='v'
   F=v.Failures();q.reference_relations(b,F,kind);self.assertTrue(F,kind)
