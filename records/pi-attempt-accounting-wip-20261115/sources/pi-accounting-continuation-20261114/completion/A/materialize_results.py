"""Ephemeral serialization of supplied finite output records, never independent oracle arithmetic."""
exec(open('/home/bline/cbm-work/pi-accounting-continuation-20261114/completion/A/author.py').read().split('old={')[0])
for p in PART:
 c=json.load(open(S/p['path']));cid=c['case_id'];path=OUT/f'{cid}.bundle.json';bundle=json.load(open(path));ip,result=bundle['documents'];raw={}
 for ses in c['inputs']['sessions']:
  for ln,log in ses.get('logs',{}).items():
   for n,line in enumerate(log.get('lines',[]),1):
    try:d=json.loads(line) if isinstance(line,str) else line
    except ValueError:d=None
    raw[f"{ses['alias']}/{ln}:{n}"]=d
 result.update(valuation_records=[],unbudgeted_applicabilities=[],unbudgeted_completions=[])
 ip['initial_contracts']=[];ip['grants']=[];contracts={}
 for origin in ip['origins']:
  ref=origin['origin_id'];d=raw[ref];source=origin['source_origin_id'];dollar=isinstance(d,dict) and (d.get('event')=='usd_reserved' or isinstance(d.get('usd'),dict))
  contract='initial:'+source if dollar else 'no-ticket:'+source;origin['initial_contract_ref']=contract
  if contract not in contracts:
   contracts[contract]={'contract_id':contract,'reference_namespace':'dollar_ticket' if dollar else 'unestablished','duplicate_rule':'same_origin_only'}
   if dollar:contracts[contract].update(scope_id=f'{cid}/{source}',grant_ref='scope:'+source)
 ip['initial_contracts']=list(contracts.values());ip['continuity']['preserved_initial_contract_refs']=sorted(contracts)
 for k,v in contracts.items():
  if v['reference_namespace']=='dollar_ticket':
   refs=sorted(x['origin_id'] for x in ip['origins'] if x['initial_contract_ref']==k)
   ip['grants'].append({'grant_id':v['grant_ref'],'guarantee_kind':'initial_reference_scope','admission':'conditional_fixture','exact_payload_ref':f'oracle/r8-completion-A/DERIVATIONS.json#{cid}/initial-scope/{v["scope_id"]}','covered_origin_refs':refs,'covered_extent_ref':'sha256:'+p['raw_input_sha256'],'authority_ref':'ROOT-POLICY-R6.md@3dbe535668a7d625e025e16c9fa75e51a59dec109f5c842d877791b1d7332200','observer_or_harness_ref':'S1-inherited-conditional-corpus','assumption_refs':['conditional initial ticket namespace in exact source; no auxiliary linkage or independence grant']})
 result['continuity_result']='conditional'
 for a in c['expected']['attempts']:
  uid=a['key'];refs=sorted(a['evidence']);rr=[x for x in refs if isinstance(raw[x],dict) and raw[x].get('event')=='usd_reserved'];ss=[x for x in refs if isinstance(raw[x],dict) and isinstance(raw[x].get('usd'),dict)];us=[x for x in refs if isinstance(raw[x],dict) and 'usage'in raw[x]]
  if a.get('settlement'):
   result['primary_validities'].append({'obligation_id':'obligation:'+uid,'distinct_reservation_refs':rr,'distinct_settlement_refs':ss,'validity_result':a['settlement']['decision'],'applicable_check_refs':[f'case:{cid}#attempt:{uid}/settlement']})
  if us:
   result['valuation_records'].append({'valuation_id':'valuation:'+uid,'usage_origin_refs':us,'complete':a['valuation'].get('complete',False),'wire_ref':f'case:{cid}#attempt:{uid}/valuation'})
   for u in us:result['financial_candidates'].append({'usage_origin_ref':u,'context_origin_ref':u,'predicate_ref':'same-origin-identity-context'})
  if a['exposure']['high']['kind']!='exact' or a['exposure']['pool']!='pi_api':continue
  # Existing finite outputs are materialized with explicit F witnesses; no raw eligibility evaluator.
  assert us and a['valuation']['complete'],(cid,uid)
  hold='held:'+uid;result['units'].append({'unit_id':hold,'state':'inactive','origin_refs':refs,'successor_ref':uid,'exposure_ref':f'case:{cid}#attempt:{uid}/prior-held'})
  cov={'coverage_id':'coverage:'+uid,'status':'accepted','unit_refs':[hold],'monetary_origin_refs':refs,'anchor_origin_refs':[],'definite_link_refs':[],'retained_component_refs':ss+us,'reason_refs':[f'case:{cid}#attempt:{uid}/replacement'],'result_unit_ref':uid,'valuation_ref':'valuation:'+uid}
  cov['retained_component_refs']=sorted(set(cov['retained_component_refs']))
  if ss:
   cov.update(route='budgeted',obligation_id='obligation:'+uid,validity_ref='obligation:'+uid)
  else:
   assert not rr
   cov.update(route='unbudgeted_completion',completion_ref='completion:'+uid)
   result['unbudgeted_applicabilities'].append({'applicability_id':'applicability:'+uid,'usage_origin_ref':us[0],'raw_subject_ref':'sha256:'+p['raw_input_sha256'],'classification':'genuine_unbudgeted','dollar_origin_refs':[]})
   result['unbudgeted_completions'].append({'completion_id':'completion:'+uid,'usage_origin_ref':us[0],'applicability_ref':'applicability:'+uid,'valuation_ref':'valuation:'+uid,'complete':True})
  result['coverage_decisions'].append(cov)
 dump(path,bundle)
print('materialized69 supplied bundles with C/V/F witnesses; output-lane-only')
