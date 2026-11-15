"""Actual main entrypoint controls; financial mutants preserve lawful raw premises."""
import contextlib,io,json,os,pathlib,shutil,tempfile,unittest,subprocess
import warnings
warnings.simplefilter("ignore", ResourceWarning)
import validate_corpus as v
BASE=pathlib.Path(os.environ['R9_QUALIFICATION_SUBJECT'])
EQ='oracle/r9-pilot/cases/equal-reservations.bundle.json'
VC='oracle/r8-completion-B/VC11-multi-turn-session.bundle.json'
P9='oracle/r8-completion-B/P9-claimed-charge-above-reservation.bundle.json'
def edit(r,p,f):
 d=json.loads((r/p).read_text());f(d);(r/p).write_text(json.dumps(d))
class Pilot(unittest.TestCase):
 def run_subject(self,mutate=None):
  with tempfile.TemporaryDirectory(prefix='r10-python-') as tmp:
   r=pathlib.Path(tmp)/'subject';shutil.copytree(BASE,r,ignore=shutil.ignore_patterns('target','__pycache__'))
   if mutate:mutate(r)
   out=io.StringIO()
   try:
    with contextlib.redirect_stdout(out):rc=v.main(['validator','check',str(r/'corpus'),'--pilot'])
   except ValueError as e:rc=1;out.write(str(e))
   finally:os.environ['R9_QUALIFICATION_SUBJECT']=str(BASE)
   return rc,out.getvalue()
 def test_positive(self):
  rc,msg=self.run_subject();self.assertEqual(rc,0,msg);self.assertIn('R10 evaluated',msg)
 def test_structural_selected_and_composed(self):
  exe=os.environ.get('R10_EVIDENCE_EXECUTABLE')
  if not exe:self.skipTest('structural executable not supplied')
  for path in v.r9_pilot.ADMITTED['admissions']:
   with self.subTest(path=path):
    p=subprocess.run([exe,'evidence',str(BASE),str(BASE/path)],capture_output=True,text=True);self.assertEqual(p.returncode,0,p.stdout+p.stderr);print('R10 resolved structural positive',path,'exit',p.returncode)
 def test_applicability_defects(self):
  mutants={
   'extent':(EQ,lambda d:d['documents'][0]['grants'][0].update(covered_extent_ref='sha256:'+'0'*64),'applicability grants'),
   'interchanged':(VC,lambda d:d['documents'][0]['grants'][0].update(exact_payload_ref='oracle/r8-completion-B/P9-claimed-charge-above-reservation.provenance.json#S1/turn-1'),'applicability grants'),
   'fragment':(VC,lambda d:d['documents'][0]['grants'][0].update(exact_payload_ref='oracle/r8-completion-B/VC11-multi-turn-session.provenance.json#S1/turn-2'),'applicability grants'),
   'scope':(EQ,lambda d:d['documents'][0]['initial_contracts'][0].update(scope_id='different'),'applicability initial_contracts'),
   'namespace':(EQ,lambda d:d['documents'][0]['initial_contracts'][0].update(reference_namespace='different'),'applicability initial_contracts'),
   'endpoint':('oracle/r8-controls/VC07-conditional.bundle.json',lambda d:d['documents'][0]['origins'][0].update(raw_bytes_sha256='0'*64),'applicability origins'),
   'exact_link':('oracle/r8-controls/VC07-conditional.bundle.json',lambda d:d['documents'][0]['exact_links'][0].update(to_origin_ref='a'),'applicability exact_links'),
   'link':('oracle/r8-controls/VC07-conditional.bundle.json',lambda d:d['documents'][0]['grants'][0]['admitted_payload'].update(to='a'),'applicability grants'),
   'copy':(EQ,lambda d:d['documents'][0]['initial_contracts'][0].update(duplicate_rule='scope_exact_line'),'applicability initial_contracts'),
   'stale_payload':('oracle/r9-pilot/cases/model7-VC01-openai-usage-settlement.provenance.json',lambda d:d.update(raw_input_sha256='0'*64),'referenced payload fragment/scope/namespace')}
  for name,(p,f,reason) in mutants.items():
   with self.subTest(name=name):
    rc,msg=self.run_subject(lambda r:edit(r,p,f));self.assertEqual(rc,1,msg);self.assertIn(reason,msg)
 def test_omitted_reservation_and_validity(self):
  def f(d):
   d['documents'][1]['primary_memberships'][0]['reservation_classes'].pop();d['documents'][1]['primary_validities'][0]['distinct_reservation_refs'].pop()
  rc,msg=self.run_subject(lambda r:edit(r,EQ,f));self.assertEqual(rc,1,msg);self.assertIn('raw reservation-class actual membership/validity',msg)
 def test_required_registration_fail_closed(self):
  def missing(d):d['claims'].pop()
  def duplicate(d):d['claims'][1]=d['claims'][0]
  def empty(d):d['claims'][0]['cases']=[]
  def case_missing(d):d['claims'][1]['cases'].pop()
  for f in [missing,duplicate,empty,case_missing]:
   with self.subTest(name=f.__name__):
    rc,msg=self.run_subject(lambda r:edit(r,'oracle/r9-pilot/claims.json',f));self.assertEqual(rc,1,msg)
 def test_wrong_copy_class_merge(self):
  def f(d):
   m=d['documents'][1]['primary_memberships'][0];m['reservation_classes']=[{'origin_refs':['S1/turn-1:1','S1/turn-1:2'],'representative_origin_ref':'S1/turn-1:1'}];d['documents'][1]['primary_validities'][0]['distinct_reservation_refs']=['S1/turn-1:1']
  rc,msg=self.run_subject(lambda r:edit(r,EQ,f));self.assertEqual(rc,1,msg);self.assertIn('raw reservation-class actual membership/validity',msg);self.assertIn('lacks admitted exact-copy',msg)
 def test_actual_raw_extent_changed(self):
  def f(d):d['inputs']['sessions'][0]['logs']['turn-1']['lines'][0]+=' '
  rc,msg=self.run_subject(lambda r:edit(r,'oracle/r9-pilot/cases/equal-reservations.json',f));self.assertEqual(rc,1,msg);self.assertIn('raw subject outside independent admission',msg)
 def test_unlinked_financial_anchor_erasure(self):
  def f(d):d['documents'][1]['units'][0]['exposure_ref']='covered:Exact15000000'
  rc,msg=self.run_subject(lambda r:edit(r,'oracle/r8-controls/VC07-unlinked.bundle.json',f));self.assertEqual(rc,1,msg);self.assertIn('raw conditional anchor/usage active financial inventory',msg)
 def test_route_bypass(self):
  def f(d):d['claims'][0]['cases'][0].update(control_file='../../corpus/cases/P9-claimed-charge-above-reservation.json',bundle_file=P9)
  rc,msg=self.run_subject(lambda r:edit(r,'oracle/r9-pilot/claims.json',f));self.assertEqual(rc,1,msg);self.assertIn('inapplicable relation selector',msg)
 def test_financial_undercharge(self):
  def mutate(r):
   def case(c):
    c['expected']=json.loads(json.dumps(c['expected']).replace('700000000','500000000'));
    for ev in c['evaluations']:ev['expected']=json.loads(json.dumps(ev['expected']).replace('700000000','500000000').replace('300000000','500000000'))
   edit(r,'corpus/cases/P9-claimed-charge-above-reservation.json',case)
   def bundle(b):
    b['documents'][1]=json.loads(json.dumps(b['documents'][1]).replace('700000000','500000000').replace('300000000','500000000'))
   edit(r,P9,bundle)
   exe=os.environ.get('R10_EVIDENCE_EXECUTABLE')
   if exe:
    structural=subprocess.run([exe,'evidence',str(r),str(r/P9)],capture_output=True,text=True);self.assertEqual(structural.returncode,0,structural.stdout+structural.stderr);print('R10 coherent wrong P9 structural pass; identical lawful raw/premises')
  rc,msg=self.run_subject(mutate);self.assertEqual(rc,1,msg);self.assertIn('P9 raw retained charge700M',msg);self.assertNotIn('R10 exact raw applicability',msg);self.assertNotIn('R10 referenced payload',msg)
 def test_reservation_financial_and_selected_joins(self):
  case='oracle/r9-pilot/cases/equal-reservations.json'
  for kind in ('high400','coherent400','kind','basis','pool','reserved','source','obligation','replacement','decision','stale-result','substituted-result','unit','unit-origin','unit-exposure','membership','validity','validity-check','input-ref','low-ref','stale-bundle'):
   with self.subTest(kind=kind):
    def mutate(r):
     def change(c):
      a=c['expected']['attempts'][0]
      if kind in ('high400','coherent400'):a['exposure']['high']['nano']='400000000'
      elif kind=='kind':a['exposure']['high']['kind']='exact'
      elif kind=='basis':a['exposure']['high']['basis']='usage_valuation'
      elif kind=='pool':a['exposure']['pool']='non_api'
      elif kind=='reserved':a['settlement']['reserved']='400000000'
      elif kind=='source':a['obligation']['amount_source']='settlement_amount'
      elif kind=='obligation':a['obligation']['state']='closed'
      elif kind=='replacement':a['replacement']['eligible']=True
      elif kind=='decision':a['settlement']['decision']='valid'
      elif kind=='stale-result':a['accounting_result_ref']='oracle/r8-completion-B/VC01-openai-usage-settlement.bundle.json#result:'+c['case_id']
      elif kind=='substituted-result':a['accounting_result_ref']=EQ+'#input:'+c['case_id']
      elif kind=='low-ref':c['evaluations'][0]['expected']['objectives'][0]['owned_outstanding']['supported_low_evidence_ref']='oracle/r8-completion-B/VC01-openai-usage-settlement.bundle.json#'+c['case_id']+'/default/o/owned_outstanding'
      if kind=='coherent400':
       ob=c['evaluations'][0]['expected']['objectives'][0];ob['owned_outstanding']['high']['nano']='400000000';ob['commitments'][0]['committed']='600000000'
     edit(r,case,change)
     def change_bundle(b):
      res=b['documents'][1]
      if kind=='unit':res['units'][0]['unit_id']='substituted'
      elif kind=='unit-origin':res['units'][0]['origin_refs'].pop()
      elif kind=='unit-exposure':res['units'][0]['exposure_ref']='case:other#attempt:other'
      elif kind=='membership':res['primary_memberships'][0]['obligation_id']='other'
      elif kind=='validity':res['primary_validities'][0]['validity_result']='valid'
      elif kind=='validity-check':res['primary_validities'][0]['applicable_check_refs']=['case:other#attempt:other/settlement']
      elif kind=='input-ref':res['input_proof_ref']='input:other'
     edit(r,EQ,change_bundle)
     if kind=='stale-bundle':shutil.copyfile(r/'oracle/r8-completion-B/VC01-openai-usage-settlement.bundle.json',r/EQ)
     reason=('raw reservation maximum/conservative high' if kind in ('high400','coherent400','kind','basis','pool') else 'raw reservation fixed settlement reserved amount' if kind=='reserved' else 'raw reservation outstanding obligation/replacement' if kind in ('source','obligation','replacement','decision') else 'reservation selected low reference join' if kind=='low-ref' else 'reservation selected raw/result/unit/validity join' if kind!='stale-bundle' else 'applicability')
     exe=os.environ['R10_FINANCIAL_EXECUTABLE'];env=dict(os.environ,R9_QUALIFICATION_SUBJECT=str(r))
     rust=subprocess.run([exe,'--exact','r9_registered_actual_subject_qualification','--nocapture'],env=env,capture_output=True,text=True)
     self.assertNotEqual(rust.returncode,0,rust.stdout+rust.stderr);self.assertIn(reason,rust.stdout+rust.stderr)
     structural=subprocess.run([os.environ['R10_EVIDENCE_EXECUTABLE'],'evidence',str(r),str(r/EQ)],capture_output=True,text=True)
     if kind in ('high400','coherent400','kind','basis','reserved','source','obligation','replacement','decision','stale-result','substituted-result','low-ref'):self.assertEqual(structural.returncode,0,structural.stdout+structural.stderr)
     print('R10 reservation mutant',kind,'structural_exit',structural.returncode,'rust_financial_exit',rust.returncode,'reason',reason)
    rc,msg=self.run_subject(mutate);self.assertEqual(rc,1,msg)
    reason=('raw reservation maximum/conservative high' if kind in ('high400','coherent400','kind','basis','pool') else 'raw reservation fixed settlement reserved amount' if kind=='reserved' else 'raw reservation outstanding obligation/replacement' if kind in ('source','obligation','replacement','decision') else 'reservation selected low reference join' if kind=='low-ref' else 'reservation selected raw/result/unit/validity join' if kind!='stale-bundle' else 'applicability')
    self.assertIn(reason,msg);print('R10 reservation mutant',kind,'python_financial_exit',rc,'reason',reason)
if __name__=='__main__':unittest.main()
