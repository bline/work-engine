# One-shot scratch migration, not shipped tooling or a reference reducer.
import json,pathlib,copy,hashlib,importlib.util
R=pathlib.Path('/home/bline/cbm-work/p136-s1r6/s1');BASE=pathlib.Path('/home/bline/cbm-work/p133-s1r5/s1/corpus');facts=json.loads((R/'oracle/FROZEN-FACTS.json').read_text());facts={c['case_id']:c for c in facts['cases']}
spec=importlib.util.spec_from_file_location('legacy',R/'validator/validate_corpus.py');v=importlib.util.module_from_spec(spec);spec.loader.exec_module(v)
# Validator used below for serialization/aggregate consistency only. Raw arithmetic remains frozen facts.
def ns(x):return int(x)
def sum_exp(atts):
 return {'low':str(sum(ns(a['exposure']['low']['nano']) for a in atts)), 'high':{'kind':'at_least' if any(a['exposure']['high']['kind']=='at_least' for a in atts) else 'exact','nano':str(sum(ns(a['exposure']['high']['nano']) for a in atts))}}
for path in sorted((R/'corpus/cases').glob('*.json')):
 if not (BASE/'cases'/path.name).exists():continue
 c=json.loads((BASE/'cases'/path.name).read_text());old=copy.deepcopy(c);cid=c['case_id'];f=facts.get(cid,{})
 rows={};factrows={}
 for s in c['inputs']['sessions']:
  for name,log in s['logs'].items():
   for n,line in enumerate(log['lines'],1):
    try: rows[f"{s['alias']}/{name}:{n}"]=json.loads(line)
    except ValueError: pass
 for session in f.get('sessions',[]):
  for source in session['sources']:
   for row in source['rows']:factrows[row['ref']]=row
 # Explicitly separate conditional fixture trust from producer inputs. Preserve raw gates unchanged.
 if c['profile']=='future_qualified_gate':
  c['trusted_fixture_inputs']={'gate_verifications':{g['gate_id']:{'source':'test_mock','payload_sha256':hashlib.sha256(json.dumps(g,sort_keys=True,separators=(',',':'),ensure_ascii=False).encode()).hexdigest(),'authority_id':g.get('authority',{}).get('id'),'session':g.get('session'),'policy_ref':'root-P5-conditional-only','verification_evidence':'independent-test-harness:exact-payload-binding'} for g in c['inputs'].get('gates',[])}}
 for a in c['expected']['attempts']:
  val=a['valuation'];refs=a['evidence'];uf=[factrows[r]['usage_facts'] for r in refs if r in factrows and 'usage_facts'in factrows[r] and factrows[r].get('event')=='request']
  if 'identities'in val:
   ids=set()
   for d in val.get('derivations',[]):ids.add(d['provider']+'/'+d['model'])
   for u in uf:
    for family,model in u.get('identities',[]):ids.add(family+'/'+model)
   if not ids:
    session=next((s for s in c['inputs']['sessions'] if s['alias']==a['key'].split(':')[1].split('/')[0]),{});lm=(session.get('launch')or{}).get('model','');fam=lm.split('/')[0];fam='openai'if fam=='openai-codex'else fam
    ids={x if '/'in x else fam+'/'+x for x in val['identities']}
   val['identities']=sorted(ids)
  if val.get('state')=='valued' and uf:
   val['complete']=all(u.get('arithmetic_state')=='valued' and u.get('complete_conjunction_candidate') is True for u in uf)
  if val.get('state')=='unpriceable' and uf:
   reasons=set(val.get('reasons',[]))
   for u in uf:
    for field in u.get('required_count_unavailable_fields',[]):reasons.add('MissingCount('+field+')')
    for field in u.get('malformed_count_fields',[]):reasons.add('MalformedCount('+field+')')
   if any(x.startswith('MalformedCount(')for x in reasons):reasons.discard('MalformedCount')
   if any(x.startswith('MissingCount(')for x in reasons):reasons.discard('MissingCount')
   val['reasons']=sorted(reasons)
  if a['settlement']['decision'] not in ('rejected','conflicting'):
   for u in uf:
    for field in u.get('malformed_count_fields',[]):
     if field!='cache_sum_exceeds_input':a['blocking']=sorted(set(a['blocking'])|{'MalformedEvidence('+field+')'})
  if cid=='N03-api-row-other-provider':
   val={'state':'unpriceable','identities':['anthropic/claude-fable-5-2','openai/claude-fable-5-2'],'reasons':['UsageSchemaMismatch','IdentityWithoutRate']};a['valuation']=val
   a['settlement']['decision']='rejected';a['settlement']['reasons']=['ModelMismatch','UsageUnpriceable'];a['settlement'].pop('basis',None);a['blocking']=sorted(set(a['blocking'])|{'SettlementRejected','IdentityContradiction','UnresolvedExposure'});a['obligation']={'state':'outstanding','amount_source':'reservation_bound'}
  if cid=='R06-launch-row-missing':
   # P6/Q9: the row cannot manufacture a priced launch identity.
   a['valuation']={'state':'unpriceable','identities':['openai/gpt-6-astra'],'reasons':['ModelUnresolved']};val=a['valuation']
  if a['obligation']['state']=='none':
   a['obligation']['amount_source']='none';a['obligation']['cause']='subscription'if a['exposure']['pool']=='codex'else 'no_reservation'
  elif a['obligation']['state']=='closed':
   a['obligation']['by']=next(r for r in refs if isinstance(rows.get(r),dict) and isinstance(rows[r].get('usd'),dict))
  a['replacement']=v.replacement_of(a)
  if a['obligation']['state']=='closed' and a['settlement'].get('basis')=='usage':a['obligation']['amount_source']='settlement_amount'if a['replacement']['eligible']else'reservation_bound'
  # Only affected P1/P2 valuation disposition changes financial exposure here.
  if cid in ('N03-api-row-other-provider','R06-launch-row-missing'):
   a['exposure']['low']={'nano':'0','basis':'none'};a['exposure']['high']={'nano':a['settlement']['reserved'],'kind':'at_least','basis':'proxy_reservation'};a['billing']='unresolved'
 # Migrate aggregate projections using existing validated attempt facts; no re-correlation or settlement decisions.
 for e in c['evaluations']:
  ee=e['expected'];ats={a['key']:a for a in c['expected']['attempts']};objs={o['objective']:o for o in e['objectives']};eobjs={o['objective']:o for o in ee['objectives']};sessions={s['session_id']:s for s in c['inputs']['sessions']}
  for oe in ee['objectives']:
   o=objs[oe['objective']];owned=[ats[x['attempt']]for x in ee['attribution']if x['ownership']=='owned'and x['objective']==oe['objective']and ats[x['attempt']]['exposure']['pool']=='pi_api']
   for label,state in [('owned_closed',('closed','none')),('owned_outstanding',('outstanding',))]:oe[label]=sum_exp([a for a in owned if a['obligation']['state']in state])
   for cm in oe['commitments']:
    adm=next(a for a in o['admissions']if a['admission_id']==cm['admission_id']);sess=sessions.get(adm['session']);alias=sess['alias']if sess else None;spent=sum(int(a['exposure']['high']['nano'])for a in owned if a['key'].split(':')[1].split('/')[0]==alias)
    caprows=[r for r in adm['rows']if r.get('event')in('pending','objective_admission')];cap=int(round(caprows[0].get('max_usd',0)*1e9))if caprows else 0
    if adm.get('gate_id'):
     purpose='release'if cm['state']in('retired','retired_resumable','reopened')else'bound';q,rs,_=v.gate_qualification(c,o,adm,purpose,c['inputs']['reference_time'],c['expected']['lifecycle'],sessions,cap);cm['gate']=v.gate_projection(c,adm,q,rs)
     if q and purpose=='bound':cm['committed']=str(max(0,v.gate_qualification.ceiling-spent));cm['kind']='exact'
    elif cm['state']in('retired_resumable','reopened','launched_open'):cm['committed']=str(max(0,cap-spent))
   reasons=set(oe['reasons']);reasons.discard('CommitmentInvalid')
   if any(cm['state']=='invalid'for cm in oe['commitments']):reasons.add('CommitmentInvalid')
   for at in ee['attribution']:
    if(at['ownership']=='owned'and at['objective']==oe['objective'])or(at['ownership']in('uncertain','unadmitted')and oe['objective']in at.get('candidates',[])):reasons.update(ats[at['attempt']]['blocking'])
   oe['reasons']=sorted(reasons);oe['status']='not_established'if reasons else'established'
  for oe in ee['objectives']:
   p=objs[oe['objective']].get('predecessor')
   if p:
    unres=[a for a in c['expected']['attempts']if any(x['attempt']==a['key']and x['ownership']=='owned'and x['objective']==p for x in ee['attribution'])and a['exposure']['pool']=='pi_api'and a['exposure']['high']['kind']=='at_least'];com=[cm for cm in eobjs[p]['commitments']if cm['state']in v.OPEN_COMMIT];oe['predecessor_obligations']={'low':'0','high':{'kind':'at_least'if unres or any(cm['kind']=='at_least'for cm in com)else'exact','nano':str(sum(int(a['exposure']['high']['nano'])for a in unres)+sum(int(cm['committed'])for cm in com))}}
  if ee.get('admission'):
   req=e['admission_request'];ee['admission']['decision']=v.admission_decision(c,objs[req['objective']],eobjs[req['objective']],req,c['inputs']['reference_time'])[0]
 if cid in ('P133-A01-incomplete-usage-above-reservation','VC03-reservation-settlement-with-usage','S02-control-error-status-after-reservation'):
  c['class']='adversarial_synthetic';c['reachability']='producer_unreachable'
 c['contract']='pi-accounting-contract/1.5-s1';c['revisions_r6']=[]
 for key in sorted(set(old)|set(c)):
  if key in('contract','rationale','revisions_r6'):continue
  if old.get(key,'__absent__')!=c.get(key,'__absent__'):c['revisions_r6'].append({'field':key,'from':old.get(key,'__absent__'),'to':copy.deepcopy(c.get(key,'__absent__')),'rule':'root P1-P6 approval07303d5c; frozen raw facts f097e5a5; explicit conjunction resolution and family preservation'})
 c['rationale']=(old.get('rationale')or'')+' Revision6: root policy P1-P6; independently frozen source/line arithmetic f097e5a5. Qualified identities preserve family; successful terminal AND integer200 AND required billing counts establishes completion. Closed obligations retain evidence by; none has amount_source none. Future gates use separate conditional test_mock witness; no live qualification.'
 path.write_text(json.dumps(c,indent=1,sort_keys=True)+'\n')
print('migrated',len(list((R/'corpus/cases').glob('*.json'))))
