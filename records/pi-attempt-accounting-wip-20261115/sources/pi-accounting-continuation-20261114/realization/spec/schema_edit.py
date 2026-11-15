import json,pathlib
p=pathlib.Path('/home/bline/cbm-work/pi-accounting-continuation-20261114/realization/s1/spec')
def dump(n,v): (p/n).write_text(json.dumps(v,indent=2,sort_keys=True)+'\n')
s=json.loads((p/'ck.json').read_text())
for x in s['projections']:
 if x['type_id']=='Exposure':x['wire_tag']='pool'
 if x['type_id']=='High':x['wire_tag']='kind';x['fields']={'nano':{'type':'Nano','source':'bound.nano'},'basis':{'type':'String','source':'bound.basis'}}
 if x['type_id']=='Valuation' and x['variant']=='valued':
  x['fields']['low']={'type':'Nano','source':'low.nano'};x['fields']['high']={'type':'Nano','source':'high.nano'};x['fields']['derivations']={'type':'List<Json>','source':'splits'};del x['fields']['splits']
 if x['type_id']=='Valuation' and x['variant']=='unpriceable':
  for k in ['supported_low','priced_high']:x['fields'][k]={'type':'Option<Nano>','source':k+'.nano'}
dump('ck.json',s)
S={'type':'string','minLength':1};N={'type':'string','pattern':'^(0|[1-9][0-9]*)$'}
def arr(item=S):return {'type':'array','items':item,'uniqueItems':True}
def enum(*v):return {'enum':list(v)}
def obj(fields,optional=()):return {'type':'object','properties':fields,'required':[k for k in fields if k not in optional],'additionalProperties':False}
def ref(k):return {'$ref':'#/$defs/'+k}
def rec(k,fields,optional=()):defs[k]=obj(fields,optional);return ref(k)
defs={}
origin=rec('Origin',dict(origin_id=S,source_origin_id=S,original_position={'type':'integer','minimum':0},raw_bytes_sha256={'type':'string','pattern':'^[0-9a-f]{64}$'},source_revision_ref=S,initial_contract_ref=S))
contract=rec('InitialReferenceContract',dict(contract_id=S,scope_id=S,reference_namespace=enum('dollar_ticket','admission','unestablished'),duplicate_rule=enum('scope_exact_line','same_origin_only'),grant_ref=S),('scope_id','grant_ref'))
grant=rec('Grant',dict(grant_id=S,guarantee_kind=enum('initial_reference_scope','exact_copy_equivalence','producer_attempt_binding','exact_row_coreference','physical_bill_independence','origin_slot_binding','origin_slot_distinctness','continuity'),admission=enum('conditional_fixture','established'),exact_payload_ref=S,producer_revision_ref=S,incarnation_ref=S,covered_origin_refs=arr(),covered_extent_ref=S,authority_ref=S,observer_or_harness_ref=S,assumption_refs=arr()),('producer_revision_ref','incarnation_ref'))
identity=rec('IdentityBinding',dict(origin_ref=S,producer_attempt_id=S,grant_ref=S));link=rec('InputLink',dict(from_origin_ref=S,to_origin_ref=S,grant_ref=S))
slot=rec('Slot',dict(slot_id=S,bill_occurrence_ref=S,grant_ref=S));binding=rec('SlotBinding',dict(origin_ref=S,slot_id=S,grant_ref=S))
frame=rec('IndependenceFrame',dict(frame_id=S,currency={'const':'USD'},grant_ref=S,slots=arr(slot),origin_slots=arr(binding),distinct_origin_slot=arr(binding)))
continuity=rec('Continuity',dict(predecessor_subject_refs=arr(),predecessor_origin_inventory_ref=S,preserved_initial_contract_refs=arr(),preserved_fact_refs=arr(),exact_origin_map_ref=S),('predecessor_origin_inventory_ref',))
base=rec('Base',dict(base_id=S,origin_refs=arr(),atomize={'type':'boolean'},atomize_fact_refs=arr(),break_copies={'type':'boolean'},break_copies_fact_refs=arr()))
unit=rec('Unit',dict(unit_id=S,state=enum('active','inactive'),origin_refs=arr(),successor_ref=S,exposure_ref=S),('successor_ref',))
owned=rec('OriginOwner',dict(origin_ref=S,base_id=S,evidence_atom_id=S,active_high_unit_id=S))
validity=rec('PrimaryValidity',dict(obligation_id=S,distinct_reservation_refs=arr(),distinct_settlement_refs=arr(),validity_result=enum('valid','rejected','conflicting','absent','not_applicable'),applicable_check_refs=arr()))
candidate=rec('Candidate',dict(usage_origin_ref=S,context_origin_ref=S,predicate_ref=S))
definite=rec('DefiniteLink',dict(from_origin_ref=S,to_origin_ref=S,admitted_input_link_ref=S))
coverage=rec('Coverage',dict(coverage_id=S,obligation_id=S,status=enum('accepted','rejected'),unit_refs=arr(),monetary_origin_refs=arr(),anchor_origin_refs=arr(),validity_ref=S,valuation_ref=S,definite_link_refs=arr(),retained_component_refs=arr(),result_unit_ref=S,reason_refs=arr()),('obligation_id','validity_ref','valuation_ref','result_unit_ref'))
credit=rec('Credit',dict(slot_ref=S,alternative_origin_refs=arr(),low=N,valuation_refs=arr()))
rollup=rec('LowRollup',dict(scope_ref=S,seed_origin_refs=arr(),alternative_origin_refs=arr(),single_floor=N,credits=arr(credit),independent_floor=N,selected_low=N,credit_mode=enum('independent_slots','single_ambiguous','none'),basis=enum('usage_valuation','none')))
assertion=rec('Assertion',dict(claim_id=S,raw_derivation_refs=arr(),expected_fields_or_relations={'type':'object'},negative_control_refs=arr()))
common=dict(id=S,version={'const':'1.7-s1'})
def env(kind,fields,optional=()):return obj(dict(kind={'const':kind},**common,**fields),optional)
defs['AccountingInputProof']=env('accounting_input_proof',dict(lineage_id=S,origins=arr(origin),initial_contracts=arr(contract),grants=arr(grant),identity_bindings=arr(identity),exact_links=arr(link),independence_frame=frame,continuity=continuity),('independence_frame',))
defs['AccountingResult']=env('accounting_result',dict(input_proof_ref=S,lineage_id=S,origins=arr(owned),bases=arr(base),units=arr(unit),primary_validities=arr(validity),financial_candidates=arr(candidate),definite_links=arr(definite),coverage_decisions=arr(coverage),low_rollups=arr(rollup),continuity_result=enum('established','conditional','unestablished'),failure_refs=arr()))
defs['AccountingExpectedAssertion']=env('accounting_expected_assertion',dict(raw_subject_ref=S,policy_refs=arr(),authored_by=S,exact_subject_refs=arr(),assertions=arr(assertion)))
schema={'$schema':'https://json-schema.org/draft/2020-12/schema','$id':'urn:pi-accounting:evidence:1.7-s1','$defs':defs,'oneOf':[ref(k)for k in ['AccountingInputProof','AccountingResult','AccountingExpectedAssertion']]};dump('accounting.schema.json',schema)
# Actual external attempt fragments, retaining known auxiliary fields explicitly.
basis=enum('usage_valuation','proxy_reservation','proxy_charge','none')
bound=obj(dict(nano=N,basis=basis))
high=obj(dict(kind=enum('exact','at_least'),nano=N,basis=basis))
exposure={'oneOf':[obj(dict(pool={'const':'pi_api'},low=bound,high=high)),obj(dict(pool={'const':'codex'},billing=S),('billing',))]}
obligation={'oneOf':[obj(dict(state={'const':'none'},amount_source={'const':'none'},cause=enum('unbudgeted','no_reservation','orphan','subscription'))),obj(dict(state={'const':'outstanding'},amount_source={'const':'reservation_bound'})),obj(dict(state={'const':'closed'},amount_source=enum('settlement_amount','reservation_confirmed','reservation_bound'),by=S))]}
valuation={'oneOf':[obj(dict(state=enum('not_reported','not_applicable'))),obj(dict(state={'const':'valued'},low=N,high=N,identities=arr(),complete={'type':'boolean'},derivations={'type':'array'},reasons=arr()),('reasons',)),obj(dict(state={'const':'unpriceable'},reasons=arr(),supported_low=N,priced_high=N),('supported_low','priced_high'))]}
wire=obj(dict(exposure=exposure,valuation=valuation,obligation=obligation,accounting_result_ref=S),('valuation','obligation','accounting_result_ref'))
wire['$schema']='https://json-schema.org/draft/2020-12/schema';dump('wire.schema.json',wire)
