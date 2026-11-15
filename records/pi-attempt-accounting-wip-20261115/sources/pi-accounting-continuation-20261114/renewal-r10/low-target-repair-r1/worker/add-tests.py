from pathlib import Path
import json
r=Path('/home/bline/cbm-work/pi-accounting-continuation-20261114/renewal-r10/low-target-repair-r1/subject')
for n in ['model7-VC01-openai-usage-settlement','model7-A04-undated-settlement-between-dated-neighbours']:
 p=r/f'oracle/r9-pilot/cases/{n}.json';c=json.loads(p.read_text());c['expected']['attempts'][0]['replacement']['reason']='UsageUnpriceable';p.write_text(json.dumps(c,indent=2,sort_keys=True)+'\n')
p=r/'validator/test_r10_qualification.py';s=p.read_text();needle="if __name__=='__main__':unittest.main()";assert needle in s
new=''' def test_selected_complete_low_witnesses(self):
  names=['equal-reservations','model7-VC01-openai-usage-settlement','model7-A04-undated-settlement-between-dated-neighbours']
  kinds=['absent','empty','renamed','duplicate','substituted','extra','wrong-scope-ref','stale-low-ref','cross-control-low-ref','cross-control-result-ref','wrong-result','wrong-unit','wrong-origin-owner','wrong-valuation','wrong-wire-ref','missing-evaluation','duplicate-evaluation','missing-objective','duplicate-objective','missing-expected-objective','duplicate-expected-objective','wrong-owner','wrong-placement','coherent-low','single-floor','independent-floor','basis','mode','credits','seed','alternatives','assertion']
  for name in names:
   case='oracle/r9-pilot/cases/'+name+'.json';bundle='oracle/r9-pilot/cases/'+name+'.bundle.json';model7=name.startswith('model7')
   for kind in kinds+(['old15M','no-release','unsupported-alternative-deletion'] if model7 else []):
    with self.subTest(control=name,kind=kind):
     def mutate(r):
      before_c=json.loads((r/case).read_text());before_i=json.loads((r/bundle).read_text())['documents'][0]
      label='owned_closed' if model7 else 'owned_outstanding';idx=0 if model7 else 1
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
        old=json.loads((BASE/case).read_text());a['valuation']={'state':'valued','complete':False,'low':'15000000','high':'15000000','identities':['openai/gpt-6-astra'],'derivations':[{'for':'both','provider':'openai','model':'gpt-6-astra','tier_multiplier':'1','nano':'15000000','usage':{'input_tokens':1000,'output_tokens':100,'cached_tokens':0,'cache_write_tokens':0}}]};a['replacement']['reason']='UsageIncomplete';a['exposure']['low']={'basis':'usage_valuation','nano':'15000000'};ob[label]['low']='15000000'
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
       elif kind=='basis':low['basis']='usage_valuation' if model7 else 'none'
       elif kind=='mode':low['credit_mode']='independent_slots'
       elif kind=='credits':low['credits']=[{'slot_ref':'unadmitted','alternative_origin_refs':low['alternative_origin_refs'],'low':'0','valuation_refs':[]}]
       elif kind=='seed':low['seed_origin_refs']=[]
       elif kind in ('alternatives','unsupported-alternative-deletion'):low['alternative_origin_refs']=[]
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
      print('R10 selected mutant',name,kind,'structural_exit',structural.returncode,'rust_exit',rust.returncode,'reason',reason,'raw_and_input_unchanged')
     rc,msg=self.run_subject(mutate)
     self.assertEqual(rc,1,msg)
     # Absent field is rejected structurally before Python's finite list check.
     if kind=='absent':self.assertIn('accounting schema',msg)
     else:
      reason=('model7 unsupported minimum' if kind in ('old15M','no-release') or (model7 and kind=='coherent-low') else 'selected priceable usage minimum' if not model7 and kind=='coherent-low' else 'selected')
      self.assertIn(reason,msg)
     if kind=='old15M':self.assertNotIn('applicability',msg);self.assertNotIn('wire schema',msg);self.assertNotIn('accounting schema',msg)
     print('R10 selected mutant',name,kind,'python_exit',rc,'intended predicate consumed')
'''
s=s.replace(needle,new+needle);p.write_text(s)
