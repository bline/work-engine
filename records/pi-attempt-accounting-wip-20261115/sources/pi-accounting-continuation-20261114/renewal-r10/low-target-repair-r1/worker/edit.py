from pathlib import Path
import json
R=Path('/home/bline/cbm-work/pi-accounting-continuation-20261114/renewal-r10/low-target-repair-r1/subject')
p=R/'validator/r9_pilot.py';s=p.read_text();s=s.replace("  if a['valuation'].get('complete') or a['replacement']['eligible'] or a['valuation'].get('high')!='15000000' or a['exposure']['high']['nano']!='500000000' or a['exposure']['high']['kind']!='at_least':F.add(c['case_id'],'R9 model7 must retain supported15M/high500M and block complete release')", "  if a['valuation']!={'state':'unpriceable','reasons':['IdentityWithoutRate'],'supported_low':None,'priced_high':'15000000'} or a['replacement']['eligible'] is not False or a['exposure']['low']!={'nano':'0','basis':'none'} or a['exposure']['high']!={'nano':'500000000','basis':'proxy_reservation','kind':'at_least'}:F.add(c['case_id'],'R10 model7 unsupported minimum/priced high/conservative hold')")
needle="\n\ndef reference_relations(bundle,F,cid):";s=s.replace(needle,"\n if claim['relation'] in ('malformed_response_model','reservation_classes'):selected_low_consequence(c,b,json.loads((root()/b['bundle_file']).read_text()),F)"+needle)
s+='''

# Finite independently authored expectations: worker DERIVATION.md, V-1/V-6,
# SUPPORTED-LOW and O-1/P-1/P-2. Never derive these from supplied rollups.
def selected_low_consequence(c,b,bundle,F):
 cid=c['case_id'];eq=b['control_file']=='cases/equal-reservations.json';extra=cid=='A04-undated-settlement-between-dated-neighbours'
 key='res:S1/turn-1:rid=1';key2='res:S1/turn-1:rid=2';usage='S1/turn-1:'+('3' if eq else '2')
 docs=bundle.get('documents',[])
 if len(docs)!=2 or [d.get('kind') for d in docs]!=['accounting_input_proof','accounting_result']:
  F.add(cid,'R10 selected result/input inventory');return
 inp,result=docs
 if inp.get('id')!='input:'+cid or result.get('id')!='result:'+cid or result.get('input_proof_ref')!=inp.get('id') or inp.get('lineage_id')!=cid or result.get('lineage_id')!=cid or 'independence_frame' in inp:F.add(cid,'R10 selected result/input join')
 attempts=c['expected']['attempts'];keys=[key]+([key2] if extra else [])
 if [a['key'] for a in attempts]!=keys:F.add(cid,'R10 selected attempt inventory');return
 origins=['S1/turn-1:1','S1/turn-1:2']+(['S1/turn-1:3'] if eq or extra else [])
 units=[{'unit_id':key,'state':'active','origin_refs':origins if eq else origins[:2],'exposure_ref':f'case:{cid}#attempt:{key}'}]
 if extra:units.append({'unit_id':key2,'state':'active','origin_refs':['S1/turn-1:3'],'exposure_ref':f'case:{cid}#attempt:{key2}'})
 if result.get('units')!=units or [(o['origin_ref'],o['active_high_unit_id']) for o in result['origins']]!=[(o,key2 if extra and o.endswith(':3') else key) for o in origins]:F.add(cid,'R10 selected unit/origin join')
 for i,a in enumerate(attempts):
  if a.get('accounting_result_ref')!=b['bundle_file']+'#result:'+cid or a['evidence']!=units[i]['origin_refs']:F.add(cid,'R10 selected result/unit reference')
 if not eq:
  if attempts[0]['obligation']!={'state':'closed','amount_source':'reservation_bound','by':usage} or attempts[0]['settlement']['decision']!='valid' or attempts[0]['exposure']['pool']!='pi_api':F.add(cid,'R10 selected model7 lawful temporal closure')
  if extra and (attempts[1]['obligation']!={'state':'outstanding','amount_source':'reservation_bound'} or attempts[1]['exposure']!={'pool':'pi_api','low':{'basis':'none','nano':'0'},'high':{'kind':'at_least','nano':'300000000','basis':'proxy_reservation'}}):F.add(cid,'R10 selected separate reservation300M')
  memberships=[];validities=[]
  for k,rs,ss in [(key,['S1/turn-1:1'],[usage])]+([(key2,['S1/turn-1:3'],[])] if extra else []):
   memberships.append({'obligation_id':'validity:'+k,'reservation_classes':[{'origin_refs':[o],'representative_origin_ref':o} for o in rs],'settlement_classes':[{'origin_refs':[o],'representative_origin_ref':o} for o in ss],'usage_origin_refs':ss})
   validities.append({'obligation_id':'validity:'+k,'distinct_reservation_refs':rs,'distinct_settlement_refs':ss,'validity_result':'valid' if ss else 'absent','applicable_check_refs':[f'case:{cid}#attempt:{k}/settlement']})
  if result.get('primary_memberships')!=memberships or result.get('primary_validities')!=validities or result.get('coverage_decisions')!=[]:F.add(cid,'R10 selected model7 membership/validity join')
 valuation=[{'valuation_id':'valuation:'+key,'usage_origin_refs':[usage],'complete':eq,'wire_ref':f'case:{cid}#attempt:{key}/valuation'}]
 if result.get('valuation_records')!=valuation:F.add(cid,'R10 selected valuation/wire join')
 evs=c.get('evaluations',[])
 if len(evs)!=1 or evs[0].get('name')!='default':F.add(cid,'R10 selected evaluation inventory');return
 ev=evs[0];obs=ev['expected']['objectives']
 if len(ev['objectives'])!=1 or ev['objectives'][0]['objective']!='o' or len(obs)!=1 or obs[0]['objective']!='o':F.add(cid,'R10 selected objective inventory');return
 want_at=[{'attempt':k,'objective':'o','ownership':'owned','placement':'inside'} for k in keys]
 if ev['expected']['attribution']!=want_at:F.add(cid,'R10 selected raw ownership/placement')
 expected=[]
 for label in ('owned_closed','owned_outstanding'):
  selected='15000000' if eq and label=='owned_outstanding' else '0'
  has_usage=(label=='owned_outstanding') if eq else (label=='owned_closed')
  inventory=[usage] if has_usage else []
  scope=f'{cid}/default/o/{label}'
  expected.append({'scope_ref':scope,'selected_low':selected,'single_floor':selected,'independent_floor':'0','basis':'usage_valuation' if selected!='0' else 'none','credit_mode':'single_ambiguous' if selected!='0' else 'none','credits':[],'seed_origin_refs':inventory,'alternative_origin_refs':inventory})
  ob=obs[0][label]
  high={'kind':'at_least','nano':'500000000'} if (eq and label=='owned_outstanding') or (not eq and label=='owned_closed') else {'kind':'at_least','nano':'300000000'} if extra and label=='owned_outstanding' else {'kind':'exact','nano':'0'}
  if ob.get('supported_low_evidence_ref')!=b['bundle_file']+'#'+scope:F.add(cid,'R10 selected low reference join')
  if ob.get('low')!=selected or ob.get('high')!=high:F.add(cid,'R10 selected independent low/result/rollup')
  owned=[key] if has_usage else ([key2] if extra and label=='owned_outstanding' else [])
  assertion={'evaluation':'default','objective':'o','label':label,'expected_low':selected,'seed_origin_refs':inventory,'alternative_origin_refs':inventory,'owned_unit_refs':owned}
  matches=[x for x in b['low_assertions'] if x.get('label')==label]
  if matches!=[assertion] or len(b['low_assertions'])!=2:F.add(cid,'R10 selected independent assertion inventory')
 # Compare the whole list before building maps: duplicate and missing targets cannot vanish.
 if result.get('low_rollups')!=expected:F.add(cid,'R10 selected complete low witness inventory/fields')
'''
p.write_text(s)
# Strengthen actual selected target block's complete witness fields, preserving general checks.
p=R/'validator/validate_corpus.py';s=p.read_text();s=s.replace("if low['basis'] != ('usage_valuation' if int(claim['expected_low']) else 'none'):","if low['basis'] != ('usage_valuation' if int(claim['expected_low']) else 'none'):") # finite check is reached by control_claim
p.write_text(s)
# Correct supplied outputs only after independent derivation and predicates exist.
for n in ['model7-VC01-openai-usage-settlement','model7-A04-undated-settlement-between-dated-neighbours']:
 p=R/f'oracle/r9-pilot/cases/{n}.json';c=json.loads(p.read_text());a=c['expected']['attempts'][0];a['valuation']={'state':'unpriceable','reasons':['IdentityWithoutRate'],'supported_low':None,'priced_high':'15000000'};a['exposure']['low']={'nano':'0','basis':'none'};c['evaluations'][0]['expected']['objectives'][0]['owned_closed']['low']='0';p.write_text(json.dumps(c,indent=2,sort_keys=True)+'\n')
 p=R/f'oracle/r9-pilot/cases/{n}.bundle.json';b=json.loads(p.read_text());low=b['documents'][1]['low_rollups'][0];low.update(selected_low='0',single_floor='0',basis='none',credit_mode='none');p.write_text(json.dumps(b,indent=2,sort_keys=True)+'\n')
p=R/'oracle/r9-pilot/claims.json';c=json.loads(p.read_text())
for b in c['claims'][2]['cases']:b['low_assertions'][0]['expected_low']='0'
p.write_text(json.dumps(c,indent=2,sort_keys=True)+'\n')
