import pathlib,json,hashlib
R=pathlib.Path('/home/bline/cbm-work/pi-accounting-continuation-20261114');S=R/'realization/s1';O=S/'oracle/r8-completion-A';dump=lambda p,v:p.write_text(json.dumps(v,indent=2,sort_keys=True)+'\n');h=lambda p:hashlib.sha256(p.read_bytes()).hexdigest();parts=json.load(open(R/'completion/PARTITIONS.json'))['partitions']['A'];der=json.load(open(O/'DERIVATIONS.json'));by={x['case_id']:x for x in der['cases']};bindings=[]
for p in parts:
 cid=p['case_id'];bundle=json.load(open(O/f'{cid}.bundle.json'));ip,res=bundle['documents'];by[cid]['initial_reference_scope_payloads']=[{'grant_id':g['grant_id'],'scope_id':next(c['scope_id'] for c in ip['initial_contracts'] if c.get('grant_ref')==g['grant_id']),'reference_namespace':'dollar_ticket','covered_origin_refs':g['covered_origin_refs'],'guarantee':'Conditional original source-local dollar ticket namespace only; no copy equivalence, auxiliary link, physical independence or live establishment','authority':'ROOT-POLICY-R6.md@3dbe535668a7d625e025e16c9fa75e51a59dec109f5c842d877791b1d7332200'} for g in ip['grants']]
 for g in ip['grants']:g['exact_payload_ref']=f'oracle/r8-completion-A/DERIVATIONS.json#{cid}/initial_reference_scope_payloads/{g["grant_id"]}'
 by[cid]['supplied_output_notice']='C/V/F records materialize existing finite computed output projections separately from independent low literals. Coverage includes inactive held unit and exact active replacement. No general reducer or live guarantee is supplied.'
 if cid=='P130-R05-conflicting-keeps-valuation-high':by[cid]['scope_membership_correction']='First private pass assigned auxiliary refusal to outstanding. Raw refusal carries no reservation obligation: owned_closed includes its none-state AtLeast0 unit; request primary remains outstanding. Scope low0/20M unchanged.'
 if cid=='P130-R12-outstanding-interval-keeps-candidate':by[cid]['scope_membership_correction']='Outstanding reservation predates admission/window and stays unadmitted possible owner. No owned unit/seed in either label.'
 if cid=='P132-CP-retired-session-resumed-without-admission':by[cid]['scope_membership_correction']='Turn2 reopened without admission stays possible alternative, not seed/owned unit. Only turn1 is positively owned.'
 # Exact finite candidate inventories: mixed observations share same raw n/source; refusal is pricing context.
 if cid in ['P130-R05-conflicting-keeps-valuation-high','P133-A04-mixed-observations']:
  urefs=['S1/turn-1:3','S1/turn-1:4'] if cid.startswith('P130') else ['S1/turn-1:2','S1/turn-1:3']
  contexts=['S1/turn-1:2','S1/turn-1:3','S1/turn-1:4'] if cid.startswith('P130') else urefs
  res['financial_candidates']=[{'usage_origin_ref':u,'context_origin_ref':c,'predicate_ref':'same-logical-source-same-usable-raw-n:1'} for u in urefs for c in contexts]
 by[cid]['financial_candidate_inventory']=res['financial_candidates']
 dump(O/f'{cid}.bundle.json',bundle)
 path=S/p['path'];c=json.load(open(path));rule='R8 finite completion A exact refs; authored01a1d840-e1a4-7b11-9b77-151fba94962a; derivation oracle/r8-completion-A/DERIVATIONS.json#'+cid
 amendments=[]
 for a in c['expected']['attempts']:amendments.append({'field':'attempts['+a['key']+'].accounting_result_ref','from':'__absent__','to':a['accounting_result_ref'],'rule':rule})
 for e in c['evaluations']:
  for o in e['expected']['objectives']:
   for label in ['owned_closed','owned_outstanding']:amendments.append({'field':e['name']+'/'+o['objective']+'.'+label+'.supported_low_evidence_ref','from':'__absent__','to':o[label]['supported_low_evidence_ref'],'rule':rule})
 c['revisions_r8']=[x for x in c.get('revisions_r8',[]) if not x.get('rule','').startswith('R8 finite completion A')]+amendments;dump(path,c);bindings.append({'path':p['path'],'baseline_sha256':p['baseline_sha256'],'final_sha256':h(path),'raw_input_sha256':p['raw_input_sha256']})
dump(O/'DERIVATIONS.json',der);dump(R/'completion/A/SOURCE-BINDINGS.json',{'cases':bindings,'authored_artifacts':[{'path':str(p.relative_to(S)),'sha256':h(p)} for p in sorted(O.iterdir())]})
print('finalized',len(bindings),'case reference amendments; all raw input digests unchanged')
