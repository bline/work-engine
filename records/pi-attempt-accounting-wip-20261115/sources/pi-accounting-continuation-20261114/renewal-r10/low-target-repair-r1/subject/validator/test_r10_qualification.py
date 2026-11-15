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
 def test_selected_complete_low_witnesses(self):
  names=['equal-reservations','model7-VC01-openai-usage-settlement','model7-A04-undated-settlement-between-dated-neighbours']
  kinds=['absent','empty','renamed','duplicate','substituted','extra','wrong-scope-ref','stale-low-ref','cross-control-low-ref','cross-control-result-ref','wrong-result','wrong-unit','wrong-origin-owner','wrong-valuation','wrong-wire-ref','missing-evaluation','duplicate-evaluation','missing-objective','duplicate-objective','missing-expected-objective','duplicate-expected-objective','wrong-owner','wrong-placement','coherent-low','single-floor','independent-floor','basis','mode','credits','seed','alternatives','assertion']
  for name in names:
   case='oracle/r9-pilot/cases/'+name+'.json';bundle='oracle/r9-pilot/cases/'+name+'.bundle.json';model7=name.startswith('model7')
   pairs=[(k,False) for k in kinds+(['old15M','no-release','unsupported-alternative-deletion'] if model7 else [])]+[(k,True) for k in ['single-floor','independent-floor','basis','mode','credits','seed','alternatives']]
   for kind,other_scope in pairs:
    with self.subTest(control=name,kind=kind,other_scope=other_scope):
     def mutate(r):
      before_c=json.loads((r/case).read_text());before_i=json.loads((r/bundle).read_text())['documents'][0]
      label=('owned_outstanding' if model7 else 'owned_closed') if other_scope else ('owned_closed' if model7 else 'owned_outstanding');idx=0 if label=='owned_closed' else 1
      def change(c):
       ev=c['evaluations'][0];ob=ev['expected']['objectives'][0];a=c['expected']['attempts'][0]
       other='equal-reservations' if model7 else 'model7-VC01-openai-usage-settlement';otherpath='oracle/r9-pilot/cases/'+other+'.bundle.json'
       if kind=='wrong-scope-ref':ob[label]['supported_low_evidence_ref']=bundle+'#'+c['case_id']+'/default/o/'+('owned_outstanding' if model7 else 'owned_closed')
       elif kind=='stale-low-ref':ob[label]['supported_low_evidence_ref']='oracle/r8-completion-B/'+c['case_id']+'.bundle.json#'+c['case_id']+'/default/o/'+label
       elif kind=='cross-control-low-ref':ob[label]['supported_low_evidence_ref']=otherpath+'#'+c['case_id']+'/default/o/'+label
       elif kind=='cross-control-result-ref':a['accounting_result_ref']=otherpath+'#result:'+c['case_id']
       elif kind=='missing-evaluation':c['evaluations']=[]
       elif kind=='duplicate-evaluation':c['evaluations'].append(json.loads(json.dumps(ev)))
       elif kind=='missing-objective':ev['objectives']=[]
       elif kind=='duplicate-objective':ev['objectives'].append(json.loads(json.dumps(ev['objectives'][0])))
       elif kind=='missing-expected-objective':ev['expected']['objectives']=[]
       elif kind=='duplicate-expected-objective':ev['expected']['objectives'].append(json.loads(json.dumps(ob)))
       elif kind=='wrong-owner':
        ev['expected']['attribution'][0]={'attempt':a['key'],'ownership':'unowned','placement':'inside'}
        ob[label]['low']='0';ob[label]['high']={'kind':'exact','nano':'0'}
       elif kind=='wrong-placement':ev['expected']['attribution'][0]['placement']='before'
       elif kind=='coherent-low':ob[label]['low']='1';a['exposure']['low']={'basis':'usage_valuation','nano':'1'}
       elif kind=='old15M':
        a['valuation']={'state':'valued','complete':False,'low':'15000000','high':'15000000','identities':['openai/gpt-6-astra'],'derivations':[{'for':'both','provider':'openai','model':'gpt-6-astra','tier_multiplier':'1','nano':'15000000','usage':{'input_tokens':1000,'output_tokens':100,'cached_tokens':0,'cache_write_tokens':0}}]};a['replacement']['reason']='UsageIncomplete';a['exposure']['low']={'basis':'usage_valuation','nano':'15000000'};ob[label]['low']='15000000'
       elif kind=='no-release':a['replacement']={'eligible':True,'reason':'UsageBound'}
      edit(r,case,change)
      def change_bundle(b):
       res=b['documents'][1];low=res['low_rollups'][idx]
       if kind=='absent':res.pop('low_rollups')
       elif kind=='empty':res['low_rollups']=[]
       elif kind=='renamed':low['scope_ref']+='-renamed'
       elif kind=='duplicate':dup=json.loads(json.dumps(low));dup['single_floor']='2';res['low_rollups'].append(dup)
       elif kind=='substituted':res['low_rollups'][idx]=json.loads(json.dumps(res['low_rollups'][1-idx]))
       elif kind=='extra':dup=json.loads(json.dumps(low));dup['scope_ref']+='-extra';res['low_rollups'].append(dup)
       elif kind=='wrong-result':res['id']='result:other'
       elif kind=='wrong-unit':res['units'][0]['exposure_ref']='case:other#attempt:other'
       elif kind=='wrong-origin-owner':res['origins'][0]['active_high_unit_id']='other'
       elif kind=='wrong-valuation':res['valuation_records'][0]['usage_origin_refs']=['S1/turn-1:1']
       elif kind=='wrong-wire-ref':res['valuation_records'][0]['wire_ref']='case:other#attempt:other/valuation'
       elif kind=='coherent-low':low.update(selected_low='1',single_floor='1',basis='usage_valuation',credit_mode='single_ambiguous')
       elif kind=='single-floor':low['single_floor']='1'
       elif kind=='independent-floor':low['independent_floor']='1'
       elif kind=='basis':low['basis']='none' if low['basis']=='usage_valuation' else 'usage_valuation'
       elif kind=='mode':low['credit_mode']='independent_slots'
       elif kind=='credits':low['credits']=[{'slot_ref':'unadmitted','alternative_origin_refs':low['alternative_origin_refs'],'low':'0','valuation_refs':[]}]
       elif kind=='seed':low['seed_origin_refs']=['S1/turn-1:1'] if other_scope else []
       elif kind in ('alternatives','unsupported-alternative-deletion'):low['alternative_origin_refs']=['S1/turn-1:1'] if other_scope else []
       elif kind=='wrong-owner':low.update(selected_low='0',single_floor='0',basis='none',credit_mode='none',seed_origin_refs=[],alternative_origin_refs=[])
       elif kind=='old15M':low.update(selected_low='15000000',single_floor='15000000',basis='usage_valuation',credit_mode='single_ambiguous')
      edit(r,bundle,change_bundle)
      if kind in ('old15M','coherent-low','assertion'):
       def claims(d):
        for cl in d['claims']:
         for b in cl['cases']:
          if b.get('bundle_file')==bundle:
           for a in b['low_assertions']:
            if a['label']==label:a['expected_low']='15000000' if kind=='old15M' else '1'
       edit(r,'oracle/r9-pilot/claims.json',claims)
      self.assertEqual(before_c['inputs'],json.loads((r/case).read_text())['inputs'])
      self.assertEqual(before_i,json.loads((r/bundle).read_text())['documents'][0])
      rust=subprocess.run([os.environ['R10_FINANCIAL_EXECUTABLE'],'--exact','r9_registered_actual_subject_qualification','--nocapture'],env=dict(os.environ,R9_QUALIFICATION_SUBJECT=str(r)),capture_output=True,text=True)
      reason=('model7 unsupported minimum' if kind in ('old15M','no-release') or (model7 and kind=='coherent-low') else 'selected priceable usage minimum' if not model7 and kind=='coherent-low' else 'selected')
      self.assertNotEqual(rust.returncode,0,rust.stdout+rust.stderr);self.assertIn(reason,rust.stdout+rust.stderr)
      structural=subprocess.run([os.environ['R10_EVIDENCE_EXECUTABLE'],'evidence',str(r),str(r/bundle)],capture_output=True,text=True)
      if kind=='old15M':self.assertEqual(structural.returncode,0,structural.stdout+structural.stderr)
      print('R10 selected mutant',name,kind,'other_scope',other_scope,'structural_exit',structural.returncode,'rust_exit',rust.returncode,'reason',reason,'raw_and_input_unchanged')
     rc,msg=self.run_subject(mutate)
     self.assertEqual(rc,1,msg)
     # Absent field is rejected structurally before Python's finite list check.
     if kind=='absent':self.assertIn('selected complete low witness',msg)
     else:
      reason=('model7 unsupported minimum' if kind in ('old15M','no-release') or (model7 and kind=='coherent-low') else 'selected priceable usage minimum' if not model7 and kind=='coherent-low' else 'selected')
      self.assertIn(reason,msg)
     if kind=='old15M':self.assertNotIn('applicability',msg);self.assertNotIn('wire schema',msg);self.assertNotIn('accounting schema',msg)
     print('R10 selected mutant',name,kind,'other_scope',other_scope,'python_exit',rc,'intended predicate consumed')
if __name__=='__main__':unittest.main()
