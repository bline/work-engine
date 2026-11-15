import json,itertools,pathlib
root=pathlib.Path('/home/bline/cbm-work/p137-s1r7/s1'); p=root/'spec/ck.json';s=json.loads(p.read_text())
def save(p,x):p.write_text(json.dumps(x,indent=2,sort_keys=True)+'\n')
s['contract_version']='pi-accounting-contract/1.6-s1'
# A fixed declaration space. No raw evidence evaluator is introduced.
dims={'lane':['pi_api','unbudgeted','subscription'],'reservation':['none','retained'],'settlement':['usage_eligible','usage_ineligible','reservation','upstream_error','error_status','absent','rejected','conflicting','not_applicable'],'valuation':['complete','incomplete','unpriceable','not_reported','not_applicable'],'charge':['none','present']}
branches=[]
for vals in itertools.product(*dims.values()):
 q=dict(zip(dims,vals));l,r,t,v,c=[q[k] for k in dims];why=None
 valid=t in ['usage_eligible','usage_ineligible','reservation','upstream_error','error_status']
 if l=='subscription' and (r!='none' or t!='not_applicable' or v!='not_applicable' or c!='none'):why='Dollar evidence is PiApi even with subscription labels; clean subscription has no monetary components.'
 elif l=='unbudgeted' and (r!='none' or t!='not_applicable' or c!='none'):why='Unbudgeted requests have no reservation-backed settlement or settlement charge.'
 elif l=='pi_api' and t=='not_applicable':why='PiApi dollar-evidence groups always have an applicable settlement decision.'
 elif valid and (r!='retained' or c!='present'):why='Valid settlement requires unique reservation and a well-formed charge; witness owned by settlement validity.'
 elif t=='usage_eligible' and v!='complete':why='EligibleUsageBound requires complete established fully priced usage.'
 elif t=='absent' and c!='none':why='Absent settlement has no claimed settlement charge.'
 result=None
 if not why:
  available=['zero']+(['reservation'] if r=='retained' else [])+(['charge'] if c=='present' else [])+(['usage_low','usage_high'] if v in ['complete','incomplete'] else ['supported_low','priced_high'] if v=='unpriceable' else [])
  low='usage_low' if v in ['complete','incomplete'] else 'supported_low' if v=='unpriceable' else 'zero'
  if l=='subscription':result={'pool':'codex','low':None,'high':None,'components':[],'billing':'unknown','witnesses':[]}
  else:
   exact=t=='usage_eligible' or (l=='unbudgeted' and v=='complete')
   components=[x for x in available if x not in ['zero','usage_low','supported_low']]
   if t=='usage_eligible':components.remove('reservation')
   if not components:components=['zero']
   result={'pool':'pi_api','low':low,'high':'exact' if exact else 'at_least','components':components,'billing':'established' if exact else 'unresolved','witnesses':['EligibleUsageBound'] if t=='usage_eligible' else ['CompleteUnbudgetedUsage'] if exact else []}
 branches.append({'id':f'X-BRANCH-{len(branches)+1:03d}','revision':1,'guard':{k:[x] for k,x in q.items()},'exclusion':why,'result':result,'refs':['X-COMPONENTS','SETTLEMENT-WITNESS','Valuation','Exposure']})
s['decisions']=[{'id':'X-DECISION','revision':1,'stage':'X','dimensions':dims,'branches':branches,'refs':['C','S','V','F','X-COMPONENTS']}]
s['rules']=[r for r in s['rules'] if r['partition'] not in ['X-EXPOSURE','F-OBLIGATION']]
s['formulas']=[f for f in s['formulas'] if f['id'] not in ['ERROR-EXPOSURE','ELIGIBLE-EXPOSURE','INELIGIBLE-EXPOSURE']]
# Keep formula IDs as noncompeting arithmetic primitives for existing invalidation clients.
for id in ['ERROR-EXPOSURE','ELIGIBLE-EXPOSURE','INELIGIBLE-EXPOSURE']:
 s['formulas'].append({'id':id,'revision':2,'text':'Arithmetic only: low = selected low component; high = selected High(max(selected components)); branch selection and component lists owned exclusively by X-DECISION. Error example renders AtLeast(max(R, valued.high or priced_high or 0)).','inputs':['components'],'refs':['X-COMPONENTS'],'depends':[]})
for t in s['types']:
 if t['id']=='Valuation':t['variants'].append({'name':'not_applicable','fields':{}})
# Total projection cases with legal cross-field constants; no enum sampling.
interface=json.loads((root/'spec/interface.json').read_text());interface['contract']=s['contract_version'];interface['revision']=7
projections=[]
for t in s['types']:
 for v in t['variants']:
  fields={k:{'type':ty,'source':k} for k,ty in v['fields'].items()}
  if t['id']=='Obligation':fields['amount_source']['allowed']={'none':['none'],'outstanding':['reservation_bound'],'closed':['settlement_amount','reservation_confirmed','reservation_bound']}[v['name']]
  projections.append({'id':'PROJECT-'+t['id']+'-'+v['name'],'revision':1,'type_id':t['id'],'variant':v['name'],'external_state':v['name'],'fields':fields,'refs':[t['id']]})
s['projections']=projections
for n in s['invariants']:
 if n['id']=='X-TABLE':n['text']='X-DECISION is the sole monetary branch owner. Each legal typed fact tuple has one result: low selects a supported usage component, high uses max of its declared available components. Reservation retained except EligibleUsageBound. Lane contradiction does not erase dollar components. Subscription-clean outputs Codex unknown.';n['refs']=['X-DECISION']
 if n['id']=='SERIALIZATION':n['text']=json.dumps(interface,sort_keys=True);n['refs']=[x['id'] for x in projections]
 if n['id']=='CLASSIFICATION':
  # Preserve attempt-local catalogue, explicitly fence nonlocal emitters.
  n['text']='K emits attempt-local findings from C/S/V/F/X only: malformed evidence, identity/lane contradiction, settlement failures, usage valuation failures and forwarding diagnostics. L owns lifecycle/session findings; T owns objective ownership/admission/window findings; R owns aggregate/commitment/budget findings. Shared code vocabulary never grants K access to L/T/R facts.'
 if n['id']=='CORRELATION':n['text']=n['text'].replace('## 4. Correlation (stage C)','## 4. Primary correlation and auxiliary attachment (stage C)')+'\nC-primary groups only primary reservation/settlement keys. SETTLEMENT-WITNESS reads those immutable primary facts. C-auxiliary attaches RS1–3 using that witness; auxiliary cannot mutate primary groups or validity. S consumes validity; it does not re-decide it.'
 if n['id']=='SETTLEMENT':n['text']=n['text'].replace('stage S','primary settlement-validity boundary')+'\nThe single-owned SETTLEMENT-WITNESS is computed after primary grouping and before auxiliary attachment. S-DECISION declarations expose that witness only; S consumes it.'
 if n['id']=='TEMPORAL-AND-FINALIZATION':n['text']+='\nFinancial provisionality is exclusively R-FINANCIAL-PROVISIONAL; closure/retirement does not establish financial finality. Obligation amount_source is selected by F-PROJECTION, never by generic enum sampling.'
def node(id,text,refs=[]):return {'id':id,'revision':1,'text':text,'refs':refs,'depends':[]}
s['invariants'] += [node('X-COMPONENTS','Typed available components: reservation R=max distinct well-formed amounts; charge=max applicable well-formed claimed settlement charges; usage_low/high from valued; supported_low/priced_high from unpriceable (absent optional amount contributes zero); zero=0. high=max declared components with declared exact/at_least tag; low is selected component. No K input. Arithmetic and independent examples require semantic review.', ['RESERVATION-MAX','Valuation']),node('SETTLEMENT-WITNESS','Owner: primary settlement validation. Inputs only C-primary reservation and settlement facts. UniqueReservation gates relative order/equality/KeepAll; no/conflicting reservations suppress those comparisons and retain all applicable local failures. Multiple settlements Conflicting precedence. Output immutable SettlementDecision witness. Auxiliary evidence cannot change validity.', ['SettlementDecision','SETTLEMENT']),node('F-PROJECTION','Owner F. none -> amount_source none and preserved cause; outstanding -> reservation_bound; closed -> preserve by; EligibleUsageBound -> settlement_amount, valid Reservation or UpstreamError KeepAll -> reservation_confirmed; all other closed -> reservation_bound.', ['TemporalObligation','Replacement','Obligation','SETTLEMENT-WITNESS']),node('R-FINANCIAL-PROVISIONAL','R owns provisional=true iff an owned attempt has Outstanding obligation OR high AtLeast; predecessor_obligations.high is AtLeast; any current non-invalid commitment is AtLeast; OR an uncertain/unadmitted PiApi candidate for this objective has unresolved billing. Finite Exact commitments alone do not imply unresolvedness. Malformed/invalid evidence blocks Established under existing reason, independently. Temporal closure and retirement cannot clear financial unresolvedness.', ['Obligation','High','BOUNDARY-CONSUMERS'])]
# Real causal boundaries within C, with no stage cycle.
s['boundaries']=[{'id':'C-primary','revision':1,'reads':['E'],'outputs':['primary_groups']},{'id':'C-validity','revision':1,'reads':['C-primary','SETTLEMENT-WITNESS'],'outputs':['settlement_validity']},{'id':'C-auxiliary','revision':1,'reads':['C-primary','C-validity'],'outputs':['enriched_groups']},{'id':'S-consume','revision':1,'reads':['C-validity','C-auxiliary'],'outputs':['temporal_obligation','execution','forwarding']}]
for st in s['stages']:
 if st['id']=='C':st['outputs']+=['settlement_validity','primary_groups','enriched_groups']
for r in s['rules']:
 if r['partition']=='S-DECISION':r['text']='Projection of immutable C-validity settlement witness; no duplicate validation. '+r['text'];r['refs']+=['SETTLEMENT-WITNESS']
# Refs embedded formerly only in prose are registered explicitly.
for n in s['invariants']:
 if n['id']=='X-TABLE':n['depends']=[]
# Avoid policy snapshot claiming old revision authority; historical original remains policy file.
for n in s['owner_policies']:
 if n['id']=='POLICY-R6':n['text']='P1–P6/D3 remain authoritative in POLICY-AND-PLAN-APPROVAL.md; revision7 amendment05e2c10e72e6ffc8495ff9a61334ef4801041e71ef1eea5b9d8fb7a6005348d4 supersedes causal/provisionality definitions. See root durable files; this reference creates no acceptance.'
save(p,s);save(root/'spec/interface.json',interface)
d=json.loads((root/'spec/dependencies.json').read_text());ids={r['id'] for r in s['rules']}
for c in d['consumers']:
 c['dependencies']=[v for v in c['dependencies'] if not v.startswith(('X-EXPOSURE-','F-OBLIGATION-'))]
 if c['id'] in ['view-contract','view-transitions','oracle-cases','consumer-T','consumer-R']:c['dependencies']+=['X-DECISION','F-PROJECTION','SETTLEMENT-WITNESS']
 if c['id']=='view-schema':c['dependencies'] +=[p['id'] for p in projections]
 if c['id']=='consumer-R':c['dependencies']+=['R-FINANCIAL-PROVISIONAL']
save(root/'spec/dependencies.json',d)
