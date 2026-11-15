"""Ephemeral finite literal authoring/serialization; no pricing or financial reducer."""
import json,pathlib,hashlib,copy
ROOT=pathlib.Path('/home/bline/cbm-work/pi-accounting-continuation-20261114');S=ROOT/'realization/s1';OUT=S/'oracle/r8-completion-A';OUT.mkdir(exist_ok=True)
PART=json.load(open(ROOT/'completion/PARTITIONS.json'))['partitions']['A'];H=lambda v:hashlib.sha256(v).hexdigest();dump=lambda p,v:p.write_text(json.dumps(v,indent=2,sort_keys=True)+'\n')
# Literal established raw-policy facts. Default pattern is exactly Astra 1000 uncached input +100 output,
# 1000*10*1000+100*50*1000 =15000000 nano. Exceptions below are independently derived.
FACTS={
 'P130-R02-rounded-up-charge-admitted':('525','21 cached Terra tokens *0.05 USD/M *flex0.5 =525 nano; logged rounded charge1000 is not usage floor'),
 'P132-FG-P130-R02-gated':('525','21 cached Terra tokens *0.05 USD/M *flex0.5 =525 nano; gate does not change usage'),
 'P130-R04-priority-overrun-above-cap':('948000000','(2400*10+9000*50)*1000*priority2=948000000'),
 'P130-R05-conflicting-keeps-valuation-high':('20000000','10000 Sonnet5 input*2*1000=20000000; Fable alternative80000000; minimum20000000; duplicate observations not summed'),
 'P130-R14-anthropic-null-tier-admitted':('28000000','Fable5.2 1000*8*1000+500*40*1000=28000000; null Anthropic tier default1'),
 'P132-FG-P130-R14-gated':('28000000','Fable5.2 1000*8*1000+500*40*1000=28000000; null Anthropic tier default1'),
 'P132-FG-VC02-gated':('28000000','Fable5.2 1000*8*1000+500*40*1000=28000000'),
 'P132-FG-N06-gated':('6000000','Astra1000 input unknown cache vertices: minimum1000*1*1000+100*50*1000=6000000; null cache is not zero'),
 'P132-FG-V01-gated':('600000000','Astra12000 output*50*1000=600000000'),
 'P132-FG-V02-gated':('10000000','Fable5.2 unsplit1000 cache writes; minimum5m1000*10*1000=10000000, high1h16000000'),
 'P133-A02-error-status-with-priceable-usage':('1000000000','Astra20000 output*50*1000=1000000000; error-status charge0 cannot erase usage'),
 'P133-A04-cache-counts-both-absent':('6000000','Astra1000 unknown cached/write minimum all cached:1000*1*1000+100*50*1000=6000000'),
 'P133-A04-cache-write-absent-legacy':('15000000','Astra cached0, unknown write: minimum all uncached1000*10*1000+100*50*1000=15000000'),
 'P133-A04-cached-malformed':('0','cached_tokens string400 is malformed; no supported minimum; reservation/charge do not establish usage'),
 'P133-A04-cached-nonzero':('11400000','Astra(1000-400)*10*1000+400*1*1000+100*50*1000=11400000'),
 'P133-A04-mixed-observations':('0','one15000000 observation plus malformed cached string alternative; V8 minimum0'),
 'P133-A05-model-case-variant':('0','GPT-6-Astra is unknown exact model identity; priced response alternative cannot establish minimum over unpriced request identity'),
 'P133-A06-error-status-null':('0','all billing counts null: NotReported; no usage support'),
}
ZERO=['P130-R06-reservation-amount-malformed','P130-R06-reservation-rid-missing','P130-R07-closed-unresolved-predecessor-constrains-successor','P130-R10-reopened-then-retired','P130-R11-open-session-before-retirement','P130-R12-outstanding-interval-keeps-candidate','P132-CP-closed-predecessor-outstanding','P132-CP-known-over-limit','P132-CP-unknown-below-limit','P132-FG-N07-gated','P132-FG-gate-contradicted-by-reopening']
for cid in ZERO:FACTS[cid]=('0','No reported priceable usage origin in pinned raw logs; reservations, errors, empty logs and lifecycle/gates give no usage floor')
OUTSTANDING={'P130-R05-conflicting-keeps-valuation-high','P133-A04-mixed-observations','P133-A05-model-case-variant','P133-A06-error-status-null','P133-A06-usage-settlement-status-absent','P133-A09-conflicting-reservations-with-settlement','P130-R06-reservation-amount-malformed','P130-R06-reservation-rid-missing','P130-R12-outstanding-interval-keeps-candidate','P132-CP-closed-predecessor-outstanding','P132-CP-known-over-limit','P132-CP-unknown-below-limit'}
NO_OWN={'P130-R12-outstanding-interval-keeps-candidate','P130-R08-admissions-without-tree-still-block','P130-R12-untrusted-clock-keeps-candidate','P133-A04-cache-counts-both-absent','P133-A04-cache-write-absent-legacy'}
old={a['claim_id']:a for a in json.load(open(S/'oracle/R8-LOW-ASSERTIONS.json'))['assertions']};assertions=[];derivations=[];bindings=[];diffs=[]
for p in PART:
 path=S/p['path'];c=json.load(open(path));cid=c['case_id'];assert H(json.dumps(c['inputs'],sort_keys=True,separators=(',',':')).encode())==p['raw_input_sha256'];lo,reason=FACTS.get(cid,('15000000','Astra1000 uncached input*10*1000 +100 output*50*1000 =15000000; completion/status/settlement affects release, not local supported usage'))
 raw={};proj=[]
 for ses in c['inputs']['sessions']:
  for logname,log in ses.get('logs',{}).items():
   for pos,line in enumerate(log.get('lines',[]),1):
    ref=f"{ses['alias']}/{logname}:{pos}";b=line.encode() if isinstance(line,str) else json.dumps(line,sort_keys=True,separators=(',',':')).encode();
    try:d=json.loads(line) if isinstance(line,str) else line
    except ValueError:d=None
    raw[ref]=(b,d);proj.append({'origin_ref':ref,'raw_bytes_sha256':H(b),'raw':d})
 # Whole source projection remains independent of supplied financial evidence subset.
 usage=sorted(ref for ref,(_,d) in raw.items() if isinstance(d,dict) and 'usage'in d)
 claimrows=[]
 for e in c['evaluations']:
  for obj in e['objectives']:
   oid=obj['objective'];own=cid not in NO_OWN and oid!='s' # p predecessor owns its own bill; successor does not borrow it
   state='owned_outstanding' if cid in OUTSTANDING else 'owned_closed'
   # Empty attempt cases are literal empty financial subjects, not fabricated zero anchors.
   unitkeys=[a['key'] for a in c['expected']['attempts']] if own else []
   for label in ['owned_closed','owned_outstanding']:
    keys=unitkeys if label==state else []
    if cid=='P130-R05-conflicting-keeps-valuation-high':keys=['res:S1/turn-1:rid=1'] if label=='owned_outstanding' else ['row:S1/turn-1:line=2']
    if cid=='P132-CP-retired-session-resumed-without-admission':keys=['res:S1/turn-1:rid=1'] if label=='owned_closed' else []
    alts=usage if keys and label==state else [];seed=alts if own and label==state else []
    if cid=='P132-CP-retired-session-resumed-without-admission':seed=[x for x in seed if '/turn-1:'in x]
    value=lo if seed else '0';claim=f'{cid}/{e["name"]}/{oid}/{label}'
    a={'case_id':cid,'evaluation':e['name'],'objective':oid,'label':label,'claim_id':claim,'expected_low':value,'raw_input_sha256':p['raw_input_sha256'],'policy_ref':'PLAN.md@43d0e7c8df270eb77d9aff32c87e03f5000bc89163da676c8a50daf7affdbe61:sections5,7','raw_derivation_refs':[f'oracle/r8-completion-A/DERIVATIONS.json#{cid}'],'assumptions':['conditional inherited initial scope; no physical independence frame','finite raw T admission/closure membership; no live provenance qualification'],'qualification':'bounded exact low and scope membership; not full raw-to-wire conformance','seed_origin_refs':seed,'alternative_origin_refs':alts,'owned_unit_refs':sorted(keys),'authored_by':'01a1d840-e1a4-7b11-9b77-151fba94962a'}
    if claim in old:assert old[claim]['expected_low']==value,(claim,old[claim]['expected_low'],value)
    assertions.append(a);claimrows.append(a)
    dest=next(x for x in e['expected']['objectives'] if x['objective']==oid)[label]
    if dest['low']!=value:diffs.append((claim,dest['low'],value))
    dest['supported_low_evidence_ref']=f'oracle/r8-completion-A/{cid}.bundle.json#{claim}'
 origins=[];owners=[];bases=[];units=[]
 for attempt in c['expected']['attempts']:
  refs=sorted(attempt['evidence']);uid=attempt['key'];bid='base:'+uid
  bases.append({'base_id':bid,'origin_refs':refs,'atomize':False,'atomize_fact_refs':[],'break_copies':False,'break_copies_fact_refs':[]})
  units.append({'unit_id':uid,'state':'active','origin_refs':refs,'exposure_ref':f'case:{cid}#attempt:{uid}'})
  for ref in refs:
   b,d=raw[ref];source,pos=ref.rsplit(':',1);origins.append({'origin_id':ref,'source_origin_id':source,'original_position':int(pos),'raw_bytes_sha256':H(b),'source_revision_ref':'sha256:'+p['raw_input_sha256'],'initial_contract_ref':'inherited-unestablished'})
   owners.append({'origin_ref':ref,'base_id':bid,'evidence_atom_id':'atom:'+ref,'active_high_unit_id':uid})
  attempt['accounting_result_ref']=f'oracle/r8-completion-A/{cid}.bundle.json#result:{cid}'
 # Inputs carry only finite financial origin subset; excluded raw rows retained in projection.
 ip={'kind':'accounting_input_proof','id':'input:'+cid,'version':'1.7-s1','lineage_id':cid,'origins':sorted(origins,key=lambda x:x['origin_id']),'initial_contracts':[{'contract_id':'inherited-unestablished','reference_namespace':'unestablished','duplicate_rule':'same_origin_only'}],'grants':[],'identity_bindings':[],'exact_links':[],'continuity':{'predecessor_subject_refs':[],'preserved_initial_contract_refs':['inherited-unestablished'],'preserved_fact_refs':[],'exact_origin_map_ref':f'oracle/r8-completion-A/DERIVATIONS.json#{cid}'}}
 result={'kind':'accounting_result','id':'result:'+cid,'version':'1.7-s1','input_proof_ref':'input:'+cid,'lineage_id':cid,'origins':owners,'bases':bases,'units':units,'primary_validities':[],'financial_candidates':[],'definite_links':[],'coverage_decisions':[],'low_rollups':[{'scope_ref':a['claim_id'],'seed_origin_refs':a['seed_origin_refs'],'alternative_origin_refs':a['alternative_origin_refs'],'single_floor':a['expected_low'],'credits':[],'independent_floor':'0','selected_low':a['expected_low'],'credit_mode':'single_ambiguous' if a['expected_low']!='0' else 'none','basis':'usage_valuation' if a['expected_low']!='0' else 'none'} for a in claimrows],'continuity_result':'unestablished','failure_refs':[]}
 dump(OUT/f'{cid}.bundle.json',{'documents':[ip,result]});dump(path,c)
 derivations.append({'case_id':cid,'raw_input_sha256':p['raw_input_sha256'],'local_low_literal':lo,'arithmetic_or_zero_reason':reason,'raw_origins':proj,'evaluation_inputs':[{k:v for k,v in e.items() if k!='expected'} for e in c['evaluations']],'scope_claims':[{k:a[k] for k in ['claim_id','expected_low','owned_unit_refs','seed_origin_refs','alternative_origin_refs']} for a in claimrows],'scope_policy':'Unique admission owner gets bill independent of display/time window. R08 two admissions conflict: no owned seed. No-admission unbudgeted rows remain candidate: no owned seed. Logical outstanding remains outstanding despite numeric usage. p retains bill; s has none. No frame: one V8 floor over all scope usage alternatives, no additive bill assertion.'})
 bindings.append({'path':p['path'],'baseline_sha256':p['baseline_sha256'],'final_sha256':H(path.read_bytes()),'raw_input_sha256':p['raw_input_sha256']})
dump(OUT/'LOW-ASSERTIONS.json',{'assertions':assertions});dump(OUT/'DERIVATIONS.json',{'authored_by':'01a1d840-e1a4-7b11-9b77-151fba94962a','policy':'PLAN.md@43d0e7c8df270eb77d9aff32c87e03f5000bc89163da676c8a50daf7affdbe61','rate_files':['oracle/rates/openai_api_rates.json','oracle/rates/anthropic_api_rates.json'],'cases':derivations});dump(ROOT/'completion/A/SOURCE-BINDINGS.json',bindings);dump(ROOT/'completion/A/LOW-DISAGREEMENTS.json',diffs)
print('cases',len(PART),'scopes',len(assertions),'existing claims preserved',sum(a['claim_id']in old for a in assertions),'low disagreements',diffs)
