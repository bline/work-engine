"""One-shot finite serialization of worker-authored literal inventory. No raw financial evaluator."""
import json,pathlib,hashlib,tarfile
ROOT=pathlib.Path('/home/bline/cbm-work/pi-accounting-continuation-20261114')
S=ROOT/'realization/s1'; OUT=S/'oracle/r8-completion-B';OUT.mkdir(exist_ok=True)
rows=json.loads((ROOT/'completion/PARTITIONS.json').read_text())['partitions']['B']
def digest(x):return hashlib.sha256(json.dumps(x,sort_keys=True,separators=(',',':')).encode()).hexdigest()
def put(p,x):p.write_text(json.dumps(x,indent=2,sort_keys=True)+'\n')
# Exact indices pinned by PARTITIONS.json: all 106 entries, not an open-domain classifier.
closed=set([0,2,3,4,7,8,10,15,16,17,18,19,20,22,23,32,33,39,46,47,50,52,54,57,59,60,65,67,69,75,76,77,78,80,82,83,85,86,87,92,93,94,95,96,101,103,104,105])
outstanding=set([0,1,2,3,9,11,12,13,21,24,25,33,37,38,51,55,56,58,61,62,63,64,66,68,70,73,79,84,88,91,97])
# A launched admission belongs to predecessor p in 13 and 105; all other admitted cases own o.
# None-obligation auxiliary anchors remain outstanding high units; they have no usage seed.
extra_anchors={20:['row:S1/turn-1:line=2'],21:['row:S1/turn-1:line=2'],23:['row:S1/turn-1:line=2'],32:['row:S1/turn-1:line=3'],33:['row:S1/turn-1:line=4']}
# Independent arithmetic in M nano: Astra1000 input+100 output=15; Sonnet10000 input=20;
# Fable1000 input+1000 5m writes=18; unknown cache allocation supports6; Terra12000 output=60;
# Astra12000 output=600; Fable1000 writes min5m=10; Fable1000input+500out=28.
closed15=set([0,3,4,7,8,15,18,32,33,46,47,54,57,59,67,69,75,76,77,78,80,82,92,93,95,101,103,104,105])
closed_values={i:'15000000' for i in closed15};closed_values.update({16:'18000000',17:'18000000',19:'20000000',20:'20000000',22:'20000000',23:'20000000',39:'6000000',60:'1000000000',85:'600000000',86:'20000000',87:'10000000',94:'28000000'})
out_values={i:'15000000' for i in [1,51,70,79]};out_values.update({84:'60000000',88:'10000000'})
# Exact raw usage-origin membership for each nonempty priced scope; no value comes from expected output.
usage_refs={i:['S1/turn-1:2'] for i in set(closed_values)|set(out_values)}
usage_refs.update({15:['S1/turn-1:2','S1/turn-1:4'],20:['S1/turn-1:3'],23:['S1/turn-1:3'],57:['S1/turn-1:4'],101:['S1/turn-1:2','S1/turn-2:2']})
# Nonpriced usage origins remain alternatives; null counts, unpriceable identity or missing tier support0.
zero_usage={10:['S1/turn-1:2'],11:['S1/turn-1:2'],24:['S1/turn-1:2'],25:['S1/turn-1:2'],50:['S1/turn-1:2'],52:['S1/turn-1:2'],55:['S1/turn-1:2'],58:['S1/turn-1:2'],61:['S1/turn-1:2'],64:['S1/turn-1:2'],65:['S1/turn-1:2'],66:['S1/turn-1:1'],73:['S1/turn-1:2'],83:['S1/turn-1:2']}
assertions=[];derivations=[];changed=[]
with tarfile.open(ROOT/'completion/baseline-freeze/subject.tar') as baseline_archive:
 oldclaims={x['claim_id']:x for x in json.load(baseline_archive.extractfile('oracle/R8-LOW-ASSERTIONS.json'))['assertions']}
for i,r in enumerate(rows):
 p=S/r['path']; c=json.loads(p.read_text());cid=c['case_id'];rawsha=digest(c['inputs']); assert rawsha==r['raw_input_sha256']
 # Obtain only exact immutable raw bytes/addresses mechanically. Classification and amounts above are authored literals.
 raw={}
 for se in c['inputs'].get('sessions',[]):
  for turn,log in se.get('logs',{}).items():
   for n,line in enumerate(log.get('lines',[]),1):raw[f'{se["alias"]}/{turn}:{n}']=line
 units_closed=[];units_out=[]
 if i in closed: units_closed=['res:S1/turn-1:rid=1']
 if i in outstanding:units_out=['res:S1/turn-1:rid=1']
 if i in [0,2,3,33]:units_out=['res:S1/turn-1:rid=2'] if i!=33 else ['res:S1/turn-1:rid=2']
 if i==15:units_closed+=['res:S1/turn-1:rid=2']
 if i in [46,47]:units_out=['res:S2/turn-1:rid=1']
 if i==101:units_closed+=['res:S1/turn-2:rid=1']
 units_closed+=extra_anchors.get(i,[])
 owner='p' if i in [13,105] else 'o'
 scopes=[]
 for ev in c.get('evaluations',[]):
  for obj in ev.get('objectives',[]):
   for label in ['owned_closed','owned_outstanding']:
    applicable=obj['objective']==owner
    own=(units_closed if label=='owned_closed' else units_out) if applicable else []
    value=(closed_values if label=='owned_closed' else out_values).get(i,'0') if applicable else '0'
    seeded=(i in closed_values if label=='owned_closed' else i in out_values) and applicable
    seeds=usage_refs.get(i,[]) if seeded else (zero_usage.get(i,[]) if own and any(k=='res:S1/turn-1:rid=1' for k in own) else [])
    # Candidate alternatives cannot establish owned seeds; retain them in U even for low0.
    candidate_usage={5:['S1/turn-1:1'],6:['S1/turn-1:1'],14:['S1/turn-1:1'],31:['S1/turn-1:1','S1/turn-1:3'],34:['S1/turn-1:2'],35:['S1/turn-1:3'],36:['S1/turn-1:2'],44:['S1/turn-1:1'],45:['S1/turn-1:1'],48:['S1/turn-1:2'],49:['S1/turn-1:2'],53:['S1/turn-1:1'],98:['S1/turn-1:1'],99:['S1/turn-1:2'],102:['S1/turn-1:1','S1/turn-1:2']}
    alternatives=list(seeds)
    if i in candidate_usage and label=='owned_closed' and not(i==53 and ev['name']=='window-starts-later'):
     alternatives=candidate_usage[i]
    scope=f'{cid}/{ev["name"]}/{obj["objective"]}/{label}' 
    claim={'case_id':cid,'claim_id':scope,'evaluation':ev['name'],'objective':obj['objective'],'label':label,'expected_low':value,'raw_input_sha256':rawsha,'policy_ref':'PLAN.md@43d0e7c8df270eb77d9aff32c87e03f5000bc89163da676c8a50daf7affdbe61:sections5,7','raw_derivation_refs':[f'oracle/r8-completion-B/DERIVATIONS.json#{cid}'],'assumptions':['Inherited conditional initial-reference scope only; no physical-bill independence frame or new exact-link grant.','Owned membership follows unique launched admission; auxiliary anchors remain separate.'],'qualification':'Finite independently authored raw/policy/rate assertion; no arbitrary raw-to-output or live grant claim.','authored_by':'01a1d840-f94e-7631-aef9-15c312b1ae20','owned_unit_refs':sorted(own),'seed_origin_refs':sorted(seeds),'alternative_origin_refs':sorted(alternatives)}
    if scope in oldclaims:
     assert oldclaims[scope]['expected_low']==value
     claim['prior_assertion_ref']='oracle/R8-LOW-ASSERTIONS.json#'+scope
    assertions.append(claim)
    scopes.append({'scope_ref':scope,'seed_origin_refs':sorted(seeds),'alternative_origin_refs':sorted(alternatives),'single_floor':value,'credits':[],'independent_floor':'0','selected_low':value,'credit_mode':'single_ambiguous' if int(value)>0 else 'none','basis':'usage_valuation' if int(value)>0 else 'none'})
    rollup=next(o for o in ev['expected']['objectives'] if o['objective']==obj['objective'])[label]
    assert rollup['low']==value,(i,scope,rollup['low'],value)
    rollup['supported_low_evidence_ref']=f'oracle/r8-completion-B/{cid}.bundle.json#{scope}'
 origins=[];owners=[];bases=[];units=[];seen=set()
 for a in c['expected']['attempts']:
  a['accounting_result_ref']=f'oracle/r8-completion-B/{cid}.bundle.json#result:{cid}'
  refs=sorted(a['evidence']); base='base:'+a['key']
  bases.append({'base_id':base,'origin_refs':refs,'atomize':False,'atomize_fact_refs':[],'break_copies':False,'break_copies_fact_refs':[]})
  units.append({'unit_id':a['key'],'state':'active','origin_refs':refs,'exposure_ref':f'case:{cid}#attempt:{a["key"]}'})
  for ref in refs:
   assert ref not in seen,(cid,ref); seen.add(ref)
   line=raw[ref];source,position=ref.rsplit(':',1)
   origins.append({'origin_id':ref,'source_origin_id':source,'original_position':int(position),'raw_bytes_sha256':hashlib.sha256(line.encode()).hexdigest(),'source_revision_ref':'sha256:'+rawsha,'initial_contract_ref':'inherited-conditional-scope'})
   owners.append({'origin_ref':ref,'base_id':base,'evidence_atom_id':'atom:'+ref,'active_high_unit_id':a['key']})
 inp={'kind':'accounting_input_proof','id':'input:'+cid,'version':'1.7-s1','lineage_id':cid,'origins':origins,'initial_contracts':[{'contract_id':'inherited-conditional-scope','reference_namespace':'unestablished','duplicate_rule':'same_origin_only'}],'grants':[],'identity_bindings':[],'exact_links':[],'continuity':{'predecessor_subject_refs':['baseline:'+r['baseline_sha256']],'preserved_initial_contract_refs':['inherited-conditional-scope'],'preserved_fact_refs':[],'exact_origin_map_ref':f'oracle/r8-completion-B/DERIVATIONS.json#{cid}'}}
 result={'kind':'accounting_result','id':'result:'+cid,'version':'1.7-s1','input_proof_ref':'input:'+cid,'lineage_id':cid,'origins':owners,'bases':bases,'units':units,'primary_validities':[],'financial_candidates':[],'definite_links':[],'coverage_decisions':[],'low_rollups':scopes,'continuity_result':'unestablished','failure_refs':[]}
 # Supplied computed completeness records for the independently identified finite release positives.
 budgeted_exact={0,3,4,7,8,15,18,32,33,39,46,47,48,49,54,57,67,69,75,76,77,78,80,82,85,86,87,92,93,94,101,103,104,105}
 unbudgeted_exact={5,6,31,34,35,36,44,53,98,99,102}
 result['valuation_records']=[];result['unbudgeted_applicabilities']=[];result['unbudgeted_completions']=[]
 for a in c['expected']['attempts']:
  key=a['key'];refs=sorted(a['evidence']); wire=f'case:{cid}#attempt:{key}'
  usage=[];reservations=[];settlements=[];unique_r=set();unique_s=set()
  for ref in refs:
   try: row=json.loads(raw[ref])
   except: continue
   if not isinstance(row,dict):continue
   if 'usage' in row: usage.append(ref)
   if row.get('event')=='usd_reserved':
    if raw[ref] not in unique_r:reservations.append(ref);unique_r.add(raw[ref])
   if isinstance(row.get('usd'),dict):
    if raw[ref] not in unique_s:settlements.append(ref);unique_s.add(raw[ref])
  validity='validity:'+key; valuation='valuation:'+key
  if reservations or settlements:
   result['primary_validities'].append({'obligation_id':validity,'distinct_reservation_refs':sorted(reservations),'distinct_settlement_refs':sorted(settlements),'validity_result':a['settlement']['decision'],'applicable_check_refs':[wire+'/settlement']})
  if usage:result['valuation_records'].append({'valuation_id':valuation,'usage_origin_refs':sorted(usage),'complete':a.get('valuation',{}).get('complete',False),'wire_ref':wire+'/valuation'})
  release=(i in budgeted_exact and key.startswith('res:') and a['exposure'].get('high',{}).get('kind')=='exact') or (i in unbudgeted_exact and key.startswith('row:') and a['exposure'].get('high',{}).get('kind')=='exact')
  if release:
   hold='held:'+key
   result['units'].append({'unit_id':hold,'state':'inactive','origin_refs':refs,'successor_ref':key,'exposure_ref':wire+'/pre-release-hold'})
   coverage={'coverage_id':'coverage:'+key,'status':'accepted','unit_refs':[hold],'monetary_origin_refs':refs,'anchor_origin_refs':[],'definite_link_refs':[],'retained_component_refs':[wire+'/retained-charge-and-context'],'result_unit_ref':key,'reason_refs':[wire+'/replacement'],'valuation_ref':valuation}
   if key.startswith('res:'):coverage.update(route='budgeted',obligation_id=validity,validity_ref=validity)
   else:
    assert len(usage)==1
    app='applicability:'+key;done='completion:'+key
    result['unbudgeted_applicabilities'].append({'applicability_id':app,'usage_origin_ref':usage[0],'raw_subject_ref':'sha256:'+rawsha,'classification':'genuine_unbudgeted','dollar_origin_refs':[]})
    result['unbudgeted_completions'].append({'completion_id':done,'usage_origin_ref':usage[0],'applicability_ref':app,'valuation_ref':valuation,'complete':True})
    coverage.update(route='unbudgeted_completion',completion_ref=done)
   result['coverage_decisions'].append(coverage)
  for ref in usage:result['financial_candidates'].append({'usage_origin_ref':ref,'context_origin_ref':ref,'predicate_ref':'same-original-usage-origin'})
 # Finite retained identity contexts; context references do not add a usage bill.
 if i in [20,23]:
  result['financial_candidates'].append({'usage_origin_ref':'S1/turn-1:3','context_origin_ref':'S1/turn-1:2','predicate_ref':'same-source-same-raw-n'})
 if i in [31,102]:
  pair=['S1/turn-1:1','S1/turn-1:3'] if i==31 else ['S1/turn-1:1','S1/turn-1:2']
  for u,context in [pair,list(reversed(pair))]:result['financial_candidates'].append({'usage_origin_ref':u,'context_origin_ref':context,'predicate_ref':'same-source-same-raw-n'})
 # Restate only inherited initial dollar scope, without auxiliary links or physical-independence grants.
 contracts=[];grants=[]
 for source in sorted({o['source_origin_id'] for o in origins}):
  source_origins=[o['origin_id'] for o in origins if o['source_origin_id']==source]
  contract='conditional-ticket:'+source;g='initial-scope:'+source
  contracts.append({'contract_id':contract,'scope_id':cid+'/'+source,'reference_namespace':'dollar_ticket','duplicate_rule':'same_origin_only','grant_ref':g})
  grants.append({'grant_id':g,'guarantee_kind':'initial_reference_scope','admission':'conditional_fixture','exact_payload_ref':f'oracle/r8-completion-B/{cid}.provenance.json#{source}','covered_origin_refs':source_origins,'covered_extent_ref':'sha256:'+rawsha,'authority_ref':'ROOT-ADOPTION-R6.json','observer_or_harness_ref':'inherited-corpus-conditional-fixture','assumption_refs':['Inherited immutable ticket namespace only; no exact auxiliary link or physical-bill independence.']})
  for o in origins:
   if o['source_origin_id']==source:o['initial_contract_ref']=contract
 if i==10:
  for contract in contracts:contract['duplicate_rule']='scope_exact_line'
  for g in list(grants):
   copy=dict(g);copy['grant_id']='copy:'+g['grant_id'];copy['guarantee_kind']='exact_copy_equivalence';copy['assumption_refs']=['Inherited exact original line bytes within the immutable initial reference scope are copies; no physical independence.'];grants.append(copy)
  for o in owners:o['evidence_atom_id']='atom:'+hashlib.sha256(raw[o['origin_ref']].encode()).hexdigest()
 inp['initial_contracts']=contracts;inp['grants']=grants
 inp['continuity']['preserved_initial_contract_refs']=[x['contract_id'] for x in contracts]
 put(OUT/f'{cid}.provenance.json',{'case_id':cid,'raw_input_sha256':rawsha,'authority_ref':'ROOT-ADOPTION-R6.json','admission':'conditional_fixture','initial_scope_contracts':contracts,'grant_payloads':[{ 'source_origin_id':source,'immutable_reference_namespace':cid+'/'+source,'exact_origin_refs':[o['origin_id'] for o in origins if o['source_origin_id']==source],'guarantee':'Usable dollar rid is interpreted only within this immutable conditional fixture source scope; same-origin-only duplicate rule except the inherited C3 exact-byte-copy control.','exact_copy_equivalence':i==10,'physical_bill_independence':'unestablished','auxiliary_exact_row_coreference':'unestablished'}for source in sorted({o['source_origin_id'] for o in origins})]})
 put(OUT/f'{cid}.bundle.json',{'documents':[inp,result]})
 derivations.append({'case_id':cid,'partition_index':i,'raw_input_sha256':rawsha,'baseline_case_sha256':r['baseline_sha256'],'exact_raw_origins':[{ 'origin_ref':ref,'raw_bytes_sha256':hashlib.sha256(line.encode()).hexdigest(),'raw':line}for ref,line in raw.items()],'rate_refs':c['inputs'].get('rates',{}),'admission_derivation':'Unique launched session admission owns o; 13/105 own predecessor p. Missing, conflicting or nonmatching launched admission creates no owned seed. Window placement and session closure do not transfer ownership.','authored_scope_literals':[{'scope_ref':x['scope_ref'],'low':x['selected_low'],'seeds':x['seed_origin_refs'],'alternatives':x['alternative_origin_refs']}for x in scopes],'owned_closed_units':units_closed,'owned_outstanding_units':units_out,'excluded_unreadable_preserved':{'inventory':c['expected'].get('inventory',[]),'non_attempt':c['expected'].get('non_attempt',[]),'unreadable':c['expected'].get('unreadable',[])}})
 p.write_text(json.dumps(c,indent=1,sort_keys=True)+'\n'); changed.append(r['path'])
put(OUT/'LOW-ASSERTIONS.json',{'assertions':assertions});put(OUT/'DERIVATIONS.json',{'authored_by':'01a1d840-f94e-7631-aef9-15c312b1ae20','policy_sha256':'43d0e7c8df270eb77d9aff32c87e03f5000bc89163da676c8a50daf7affdbe61','cases':derivations})
put(ROOT/'completion/B/CHANGED-PATHS.json',{'case_paths':changed,'new_directory':'oracle/r8-completion-B'})
print('serialized',len(rows),'cases',len(assertions),'independent scopes')
