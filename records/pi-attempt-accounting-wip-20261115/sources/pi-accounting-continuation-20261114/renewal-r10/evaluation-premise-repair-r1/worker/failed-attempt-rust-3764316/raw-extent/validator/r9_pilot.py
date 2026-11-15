"""Independently authored finite R9 claims. No generator or spec decision imports."""
import hashlib,json,os
from pathlib import Path
REQUIRED={'P9-RETAINED-CHARGE':'retained_charge','PRIMARY/ANCHOR-APPLICATION':'primary_anchor','RESPONSE-MODEL7-ACTUAL':'malformed_response_model','EQUAL-AMOUNT-DISTINCT-RESERVATIONS':'reservation_classes'}

def root(): return Path(os.environ.get('R9_QUALIFICATION_SUBJECT',Path(__file__).resolve().parent.parent))
def load_registry():
 d=json.loads((root()/'oracle/r9-pilot/claims.json').read_text()); cs=d['claims']
 allowed={'retained_charge':{'P9-claimed-charge-above-reservation'},'primary_anchor':{'VC11-multi-turn-session','P132-FG-VC11-gated','VC07-read-failure-paired'},'malformed_response_model':{'VC01-openai-usage-settlement','A04-undated-settlement-between-dated-neighbours'},'reservation_classes':{'VC01-openai-usage-settlement'}}
 if len(cs)!=4 or {c['claim_id']:c['relation'] for c in cs}!=REQUIRED: raise ValueError('R9 mandatory four-claim registration missing/duplicate/inapplicable')
 from jsonschema import Draft202012Validator
 schema=json.loads((root()/'oracle/r9-pilot/claims.schema.json').read_text())
 if list(Draft202012Validator(schema).iter_errors(d)):raise ValueError('R9 finite claim schema invalid')
 expected=[1,3,2,1]
 for c in cs:
  for b in c['cases']: selected_binding(c,b)
  if {b['case_id'] for b in c['cases']}!=allowed[c['relation']]:raise ValueError('R9 missing/inapplicable fixed case binding')
  if len(c['cases']) != expected[list(REQUIRED).index(c['claim_id'])]: raise ValueError('R9 required case binding count')
  if len({(b['case_id'],b.get('control_file')) for b in c['cases']})!=len(c['cases']): raise ValueError('R9 duplicate case binding')
  if not c['cases']: raise ValueError('R9 unconsumed empty claim '+c['claim_id'])
 return cs

def raw_map(c):
 return {f"{s['alias']}/{t}:{i}":l for s in c['inputs']['sessions'] for t,g in s['logs'].items() for i,l in enumerate(g['lines'],1)}
def premise(c,b,F):
 raw=raw_map(c)
 if set(raw)!={x['origin_ref'] for x in b['origins']}: F.add(c['case_id'],'R9 raw inventory differs from registered independent premise')
 for p in b['origins']:
  l=raw.get(p['origin_ref']); coord=p['source_origin_id']+':'+str(p['original_position'])
  if coord!=p['origin_ref'] or l is None or hashlib.sha256(l.encode()).hexdigest()!=p['raw_bytes_sha256']: F.add(c['case_id'],'R9 exact raw origin premise changed: '+p['origin_ref'])

def attempt_policy(cid,a,rows,F):
 for row in rows:
  if not isinstance(row,dict): continue
  bad=[k for k in ('model','response_model','authorized_model') if k in row and row[k] is not None and (not isinstance(row[k],str) or not row[k])]
  if bad and (a['valuation'].get('complete') or a['replacement']['eligible']): F.add(cid,'R9 malformed actual identity prevents complete usage/replacement: '+','.join(bad))

def case_claim(c,F,bundle=None):
 for claim in load_registry():
  for b in claim['cases']:
   selected_binding(claim,b)
   if b['case_id']!=c['case_id'] or 'control_file' in b: continue
   premise(c,b,F);cid=c['case_id'];rel=claim['relation']
   if rel=='retained_charge':
    a=next((x for x in c['expected']['attempts'] if x['key']==b['attempt_key']),None)
    raw=raw_map(c);row=json.loads(raw['S1/turn-1:2']);res=json.loads(raw['S1/turn-1:1'])
    if row['usd']['charged_usd']!=0.7 or row['usd']['reserved_usd']!=0.5 or res['reserved_usd']!=0.4: F.add(cid,'R9 P9 raw numeric charge/reservation premise')
    components={(x['origin_ref'],x['kind'],x['nano']) for x in b['components']}
    if len(b['components'])!=4 or components!={('S1/turn-1:1','reservation','400000000'),('S1/turn-1:2','claimed_reserved','500000000'),('S1/turn-1:2','charge','700000000'),('S1/turn-1:2','usage','15000000')}:F.add(cid,'R9 independent component premises disagree with raw400/500/700/15M')
    if a is None or a['settlement'].get('charged')!='700000000' or a['settlement'].get('reserved')!='500000000' or a['exposure']['high']['nano']!='700000000' or a['valuation'].get('high')!='15000000': F.add(cid,'R9 P9 raw retained charge700M violated')
    ev=next((e for e in c['evaluations'] if e['name']==b['evaluation']),None)
    ob=next((o for o in ev['expected']['objectives'] if o['objective']==b['objective']),None) if ev else None
    if ob is None or ob['owned_outstanding']['high']['nano']!='700000000' or next((x['committed'] for x in ob['commitments'] if x['admission_id']==b['admission_id']),None)!='300000000': F.add(cid,'R9 P9 raw rollup/commitment700M/300M violated')

    if bundle is not None:
     r=next(x for x in bundle['documents'] if x['kind']=='accounting_result')
     actual={(x['origin_ref'],x['role'],x['nano_usd']) for x in r.get('retained_components',[])}
     want={('S1/turn-1:1','reservation','400000000'),('S1/turn-1:2','claimed_reserved','500000000'),('S1/turn-1:2','charge','700000000'),('S1/turn-1:2','usage','15000000')}
     if actual!=want or len(r.get('retained_components',[]))!=4: F.add(cid,'R9 P9 raw retained component inventory400/500/700/15M')
     ids={x['component_id'] for x in r.get('retained_components',[])}
     owners=[x for x in r['units'] if x['unit_id']==b['attempt_key'] and x['state']=='active']
     if len(ids)!=4 or len(owners)!=1 or set(owners[0].get('retained_component_refs',[]))!=ids:F.add(cid,'R9 P9 actual retained component ownership/references')
   elif rel=='primary_anchor' and bundle is not None and cid in ('VC11-multi-turn-session','P132-FG-VC11-gated'):
    result=next(x for x in bundle['documents'] if x['kind']=='accounting_result')
    want=[]
    for selector in b['obligations']:
     r=selector['reservation_origin_ref'];s=selector['settlement_origin_ref']
     want.append({'obligation_id':selector['obligation_id'],'reservation_classes':[{'origin_refs':[r],'representative_origin_ref':r}],'settlement_classes':[{'origin_refs':[s],'representative_origin_ref':s}],'usage_origin_refs':[s]})
    raw=raw_map(c);raw_primary={o for o,l in raw.items() if json.loads(l).get('event')=='usd_reserved' or isinstance(json.loads(l).get('usd'),dict)}
    claimed={o for m in want for role in ('reservation_classes','settlement_classes') for cl in m[role] for o in cl['origin_refs']}
    if raw_primary!=claimed:F.add(cid,'R9 registered primary premises omit raw evidence')
    if result.get('primary_memberships')!=want:F.add(cid,'R9 exhaustive raw primary membership/role/equivalence violated')
    expected_components={(m['reservation_classes'][0]['representative_origin_ref'],'reservation','500000000') for m in want}
    for m in want:
     origin=m['settlement_classes'][0]['representative_origin_ref'];expected_components|={(origin,'claimed_reserved','500000000'),(origin,'charge','15000000'),(origin,'usage','15000000')}
    components=result.get('retained_components',[])
    if len(components)!=8 or {(x['origin_ref'],x['role'],x['nano_usd']) for x in components}!=expected_components:F.add(cid,'R9 VC11 retained component facts disagree with exact raw500/500/15/15M')

   elif rel=='primary_anchor' and bundle is not None and cid=='VC07-read-failure-paired':
    result=next(x for x in bundle['documents'] if x['kind']=='accounting_result');components=result.get('retained_components',[])
    if len(components)!=1 or (components[0]['origin_ref'],components[0]['role'],components[0]['nano_usd'])!=('S1/turn-1:2','usage','15000000'):F.add(cid,'R9 historical VC07 retained usage fact disagrees with raw15M/no-dollar')


def control_claim(claim,b,c,F):
 premise(c,b,F);a=next(x for x in c['expected']['attempts'] if x['key']==b['attempt_key']);raw=raw_map(c)
 if claim['relation']=='malformed_response_model':
  row=json.loads(raw[b['origin_ref']]);v=row.get('response_model')
  if b['response_model_state']!='present_number' or type(b['response_model_value']) is not int or b['response_model_value']!=7 or type(v) is not int or v!=7:F.add(c['case_id'],'R9 actual response_model presence/type/value premise')
  if a['valuation']!={'state':'unpriceable','reasons':['IdentityWithoutRate'],'supported_low':None,'priced_high':'15000000'} or a['replacement']['eligible'] is not False or a['exposure']['low']!={'nano':'0','basis':'none'} or a['exposure']['high']!={'nano':'500000000','basis':'proxy_reservation','kind':'at_least'}:F.add(c['case_id'],'R10 model7 unsupported minimum/priced high/conservative hold')
 elif claim['relation']=='reservation_classes':
  ls=[raw[x] for x in b['reservation_origin_refs']];rs=[json.loads(x) for x in ls]
  if b['expected_classes']!=2 or b['maximum_reservation_nano']!='500000000' or b['comparison_applicable'] is not False:F.add(c['case_id'],'R9 independent reservation cardinality premises disagree with raw')
  if len(set(ls))!=2 or any(x['reserved_usd']!=0.5 for x in rs):F.add(c['case_id'],'R9 two byte-distinct500M reservation premises')
  if a['replacement']['eligible'] or a['settlement']['decision']=='valid':F.add(c['case_id'],'R9 evidence cardinality2 prohibits comparison/release despite max500M')
  reservation_consequence(c,a,b,raw,F)
  bundle=json.loads((root()/b['bundle_file']).read_text());reservation_inventory(bundle,F,c['case_id']);inp=bundle['documents'][0];provenance=json.loads((root()/'oracle/r9-pilot/cases/equal-reservations.provenance.json').read_text());scope=provenance['grant_payloads'][0]
  if scope['source_origin_id']!='S1/turn-1' or scope['exact_origin_refs']!=sorted(raw) or provenance['raw_input_sha256']!=b['raw_input_sha256'] or inp['grants'][0]['covered_origin_refs']!=scope['exact_origin_refs'] or inp['grants'][0]['exact_payload_ref']!='oracle/r9-pilot/cases/equal-reservations.provenance.json#S1/turn-1':F.add(c['case_id'],'R9 equal-reservation exact inherited scope extent binding')


def reference_relations(bundle,F,cid):
 inp=next(x for x in bundle['documents'] if x['kind']=='accounting_input_proof');r=next(x for x in bundle['documents'] if x['kind']=='accounting_result')
 ms={m['obligation_id']:m for m in r.get('primary_memberships',[])};vs={v['obligation_id']:v for v in r['primary_validities']}
 if len(ms)!=len(r.get('primary_memberships',[])) or set(ms)!=set(vs):F.add(cid,'R9 missing/duplicate primary membership')
 primary={};origin_records={o['origin_id']:o for o in inp['origins']};contracts={x['contract_id']:x for x in inp['initial_contracts']}
 for k,m in ms.items():
  for role,field in [('reservation_classes','distinct_reservation_refs'),('settlement_classes','distinct_settlement_refs')]:
   reps=[]
   for cl in m[role]:
    origins=cl['origin_refs'];rep=cl['representative_origin_ref'];reps.append(rep)
    if not origins or origins!=sorted(set(origins)) or rep!=min(origins):F.add(cid,'R9 malformed evidence class')
    if len(origins)>1:
     records=[origin_records.get(o,{}) for o in origins];cs=[contracts.get(x.get('initial_contract_ref'),{}) for x in records]
     grants=[g for g in inp['grants'] if g['guarantee_kind']=='exact_copy_equivalence' and set(origins)<=set(g['covered_origin_refs'])]
     if not grants or len({x.get('raw_bytes_sha256') for x in records})!=1 or any(x.get('duplicate_rule')!='scope_exact_line' or not x.get('scope_id') for x in cs) or len({x.get('scope_id') for x in cs})!=1:F.add(cid,'R9 multi-origin class lacks admitted exact-copy equivalence')
    for o in origins:
     if o in primary:F.add(cid,'R9 primary origin in multiple classes')
     primary[o]=k
   if k in vs and sorted(reps)!=sorted(vs[k][field]):F.add(cid,'R9 validity membership representative mismatch')
 components={x['component_id']:x for x in r.get('retained_components',[])};units={x['unit_id']:x for x in r['units']};owners={x['origin_ref']:x['active_high_unit_id'] for x in r['origins']}
 if len(components)!=len(r.get('retained_components',[])):F.add(cid,'R9 duplicate retained component ID')
 for component in components.values():
  owner=units.get(owners.get(component['origin_ref']),{})
  if component['component_id'] not in owner.get('retained_component_refs',[]):F.add(cid,'R9 component omitted by actual active owner')
 for unit in r['units']:
  for ref in unit.get('retained_component_refs',[]):
   component=components.get(ref)
   if not component or component['origin_ref'] not in unit['origin_refs']:F.add(cid,'R9 dangling/wrong-unit retained component')
 links={l['link_id']:l for l in r['definite_links']};il={l['link_id']:l for l in inp['exact_links']};grants={g['grant_id']:g for g in inp['grants']}
 for l in links.values():
  i=il.get(l['admitted_input_link_ref']);g=grants.get(i['grant_ref']) if i else None
  if not i or l['from_origin_ref']!=i['from_origin_ref'] or l['to_origin_ref']!=i['to_origin_ref']:F.add(cid,'R9 exact admitted link mismatch');continue
  payload=g.get('admitted_payload',{}) if g else {};pairs=payload.get('pairs',[payload]);pair={'from':i['from_origin_ref'],'to':i['to_origin_ref']}
  if i['from_origin_ref']==i['to_origin_ref'] or pair not in pairs:F.add(cid,'R9 grant does not admit exact endpoint pair')
 vals={v['valuation_id']:v for v in r.get('valuation_records',[])};comps={v['completion_id']:v for v in r.get('unbudgeted_completions',[])};units={v['unit_id']:v for v in r['units']}
 for c in r['coverage_decisions']:
  if c['status']!='accepted':continue
  money=set(c['monetary_origin_refs']);anchors=set(c['anchor_origin_refs']);covered=money|anchors
  for ref in c['retained_component_refs']:
   component=components.get(ref)
   contexts={x['context_origin_ref'] for x in r.get('financial_candidates',[]) if x['usage_origin_ref'] in covered}
   if not component or component['origin_ref'] not in covered|contexts:F.add(cid,'R9 dangling/wrong-coverage retained component')
  held=[units.get(x) for x in c['unit_refs']]
  if money&anchors or any(x is None for x in held) or set().union(*(set(x['origin_refs']) for x in held if x))!=covered:F.add(cid,'R9 coverage whole unit inventory')
  if c['route']=='budgeted':
   m=ms.get(c.get('validity_ref'));v=vs.get(c.get('validity_ref'))
   if not m or c.get('obligation_id')!=m['obligation_id']:F.add(cid,'R9 coverage obligation/witness applicability');continue
   ps={o for role in ('reservation_classes','settlement_classes') for cl in m[role] for o in cl['origin_refs']};usage=set(m['usage_origin_refs'])
   if not ps<=money or any(primary.get(o,m['obligation_id'])!=m['obligation_id'] for o in covered):F.add(cid,'R9 coverage primary inventory applicability')
   targets=ps|usage
  else:
   comp=comps.get(c.get('completion_ref'),{});usage=set(comp.get('usage_origin_refs',[]));targets=usage
  valuation=vals.get(c['valuation_ref'],{})
  if not usage or set(valuation.get('usage_origin_refs',[]))!=usage or not usage<=covered:F.add(cid,'R9 valuation usage applicability')
  cited=[links.get(x) for x in c['definite_link_refs']]
  for anchor in anchors:
   if not any(l and ((l['from_origin_ref']==anchor and l['to_origin_ref'] in targets) or (l['to_origin_ref']==anchor and l['from_origin_ref'] in targets)) for l in cited):F.add(cid,'R9 released anchor lacks cited applicable exact link')


def control_low_assertion(c,evaluation,objective,label):
 actual=hashlib.sha256(json.dumps(c['inputs'],sort_keys=True,separators=(',',':')).encode()).hexdigest()
 for claim in load_registry():
  if claim['relation'] not in ('malformed_response_model','reservation_classes'):continue
  for b in claim['cases']:
   if b['case_id']!=c['case_id'] or b['raw_input_sha256']!=actual:continue
   for low in b['low_assertions']:
    if (low['evaluation'],low['objective'],low['label'])==(evaluation,objective,label):return dict(low,raw_input_sha256=b['raw_input_sha256'],claim_id=claim['claim_id'])
 return None


def conditional_raw_qualification(F):
 before=len(F)
 raw_bytes=(root()/'oracle/r8-controls/VC07-exact.raw.json').read_bytes()
 binding=next(b for c in load_registry() if c['relation']=='primary_anchor' for b in c['cases'] if b['case_id']=='VC07-read-failure-paired')
 if hashlib.sha256(raw_bytes).hexdigest()!=binding['conditional_raw_sha256']:F.add('VC07','R9 conditional raw premise custody changed')
 raw=json.loads(raw_bytes)
 bundle=json.loads((root()/'oracle/r8-controls/VC07-conditional.bundle.json').read_text())
 inp=bundle['documents'][0];assumption=raw['conditional_harness_assumptions']['exact_row_coreference'];want={'from':assumption['from'],'to':assumption['to']}
 grants=[g for g in inp['grants'] if g['guarantee_kind']=='exact_row_coreference']
 if len(grants)!=1 or grants[0].get('admitted_payload')!=want or grants[0]['exact_payload_ref']!='VC07-exact.raw.json':F.add('VC07','R9 conditional exact admitted payload differs from pinned raw assumption')
 components=bundle['documents'][1].get('retained_components',[])
 if len(components)!=1 or (components[0]['origin_ref'],components[0]['role'],components[0]['nano_usd'])!=('u','usage','15000000'):F.add('VC07','R9 conditional retained usage fact disagrees with raw15M/no-dollar')
 exact_applicability(bundle,'oracle/r8-controls/VC07-conditional.bundle.json',F)
 conditional_consequence(bundle,True,F)
 conditional_consequence(json.loads((root()/'oracle/r8-controls/VC07-unlinked.bundle.json').read_text()),False,F)
 reference_relations(bundle,F,'VC07-conditional')
 exact_applicability(json.loads((root()/'oracle/r8-controls/VC07-unlinked.bundle.json').read_text()),'oracle/r8-controls/VC07-unlinked.bundle.json',F)
 reference_relations(json.loads((root()/'oracle/r8-controls/VC07-unlinked.bundle.json').read_text()),F,'VC07-unlinked')
 print('R10 evaluated',json.dumps({'relation':'primary_anchor_conditional','subjects':['oracle/r8-controls/VC07-conditional.bundle.json','oracle/r8-controls/VC07-unlinked.bundle.json'],'outcome':'pass' if len(F)==before else 'fail'},sort_keys=True))

# R10 finite applicability: independently admitted premise records, not outputs.
ADMITTED = json.loads((Path(__file__).parent/'r10_admitted.json').read_text())
# Parse at the actual consumer boundary, before any consequence/consumed marker.
def strict_read(path):
 def members(pairs):
  d={}
  for k,v in pairs:
   if k in d:raise ValueError('duplicate member '+k)
   d[k]=v
  return d
 def invalid(v):raise ValueError('non JSON number '+v)
 return json.loads(path.read_text(),object_pairs_hook=members,parse_constant=invalid)

def finite_premises(F):
 try:
  exclusions=set(ADMITTED['premise_metadata_exclusions'])|{'expected'}
  for path,a in ADMITTED['admissions'].items():
   source=a.get('case_file',a.get('conditional_raw_file'))
   c=strict_read(root()/source)
   if not isinstance(c,dict) or sorted(c)!=a['premise_inventory']:raise ValueError(source+' top-level inventory')
   projected={k:([{n:v for n,v in e.items() if n!='expected'} for e in value] if k=='evaluations' else value) for k,value in c.items() if k not in exclusions}
   # Canonical JSON preserves bool/number, array ordering, absence/null and types.
   if json.dumps(projected,sort_keys=True,separators=(',',':'))!=json.dumps(a['premise_record'],sort_keys=True,separators=(',',':')):raise ValueError(source+' governing values/types/inventory')
   strict_read(root()/path)
   for payload in a['payloads']:strict_read(root()/payload)
  for path,want in ADMITTED['rate_premises'].items():
   if json.dumps(strict_read(root()/path),sort_keys=True)!=json.dumps(want,sort_keys=True):raise ValueError(path+' valuation rate')
  strict_read(root()/'oracle/r9-pilot/claims.json')
 except (OSError,ValueError,TypeError,KeyError,AttributeError) as e:
  F.add('finite admission','R10 finite premise outside independent admission: '+str(e));return False
 return True

def selected_binding(claim,b):
 key=claim['relation']+':'+b['case_id']
 s=ADMITTED['selected'].get(key)
 if s is None:raise ValueError('R10 inapplicable relation/subject')
 for k in ('control_file','bundle_file','raw_file'):
  if b.get(k)!=s.get(k) and not (k=='bundle_file' and s['control_file'] is None and k not in b):
   raise ValueError('R10 inapplicable relation selector '+key+' '+k)
 return s

def exact_applicability(bundle,bundle_path,F):
 admitted=ADMITTED['admissions'].get(bundle_path)
 if admitted is None:F.add(bundle_path,'R10 unadmitted selected subject');return False
 before=len(F);inp=next(x for x in bundle['documents'] if x['kind']=='accounting_input_proof')
 if 'case_file' in admitted:
  c=json.loads((root()/admitted['case_file']).read_text());inputs=c['inputs'];raw=raw_map(c)
 else:
  d=json.loads((root()/admitted['conditional_raw_file']).read_text());inputs=d['original_inputs'];raw={'a':inputs['sessions'][0]['logs']['turn-1']['lines'][0],'u':inputs['sessions'][0]['logs']['turn-1']['lines'][1]}
 digest=hashlib.sha256(json.dumps(inputs,sort_keys=True,separators=(',',':')).encode()).hexdigest()
 if digest!=admitted['raw_input_sha256']:F.add(bundle_path,'R10 raw subject outside independent admission')
 for field in ('origins','initial_contracts','grants','exact_links'):
  if inp.get(field)!=admitted[field]:F.add(bundle_path,'R10 exact raw applicability '+field)
 for origin in inp.get('origins',[]):
  line=raw.get(origin['origin_id']);p=origin['original_position'];source=origin['source_origin_id']
  if line is None or hashlib.sha256(line.encode()).hexdigest()!=origin['raw_bytes_sha256'] or origin['source_revision_ref']!='sha256:'+digest or (origin['origin_id'] not in ('a','u') and origin['origin_id']!=source+':'+str(p)):
   F.add(bundle_path,'R10 endpoint original bytes/position/raw extent applicability')
 for path,want in admitted['payloads'].items():
  try:actual=json.loads((root()/path).read_text())
  except (OSError,ValueError):F.add(bundle_path,'R10 missing referenced exact payload');continue
  if actual!=want:F.add(bundle_path,'R10 referenced payload fragment/scope/namespace differs from independent admission')
 return len(F)==before

def reservation_inventory(bundle,F,cid):
 result=next(x for x in bundle['documents'] if x['kind']=='accounting_result')
 want=[{'origin_refs':[o],'representative_origin_ref':o} for o in ('S1/turn-1:1','S1/turn-1:2')]
 ms=result.get('primary_memberships',[]);vs=result.get('primary_validities',[])
 if len(ms)!=1 or ms[0]['reservation_classes']!=want or len(vs)!=1 or vs[0]['distinct_reservation_refs']!=['S1/turn-1:1','S1/turn-1:2']:
  F.add(cid,'R10 raw reservation-class actual membership/validity inventory must contain both classes')

def reservation_composed_controls(F):
 for control in ADMITTED['reservation_controls']:
  before=len(F)
  path=control['bundle_file'];bundle=json.loads((root()/path).read_text());exact_applicability(bundle,path,F)
  inp,result=bundle['documents'];origins={x['origin_id']:x for x in inp['origins']};observed=set(control['observations'])
  classes=[]
  for oid in sorted(observed):
   if oid not in origins:F.add(path,'R10 unknown observed origin');continue
   item=origins[oid];key=(item['source_origin_id'],item['raw_bytes_sha256'])
   merge=any(g['guarantee_kind']=='exact_copy_equivalence' and oid in g['covered_origin_refs'] for g in inp['grants'])
   match=next((c for k,c in classes if merge and k==key),None)
   if match is None:classes.append((key,[oid]))
   else:match.append(oid)
  want=[{'origin_refs':c,'representative_origin_ref':c[0]} for _,c in classes]
  ms=result.get('primary_memberships',[]);vs=result['primary_validities']
  if len(classes)!=control['expected_classes'] or len(ms)!=1 or ms[0]['reservation_classes']!=want or len(vs)!=1 or vs[0]['distinct_reservation_refs']!=[c[0] for _,c in classes]:F.add(path,'R10 composed raw reservation class inventory')
  reference_relations(bundle,F,path)
  print('R10 evaluated',json.dumps({'relation':'reservation_classes','subject':path,'label':control['label'],'actual_classes':len(classes),'outcome':'pass' if len(F)==before else 'fail'},sort_keys=True))

def conditional_consequence(bundle,linked,F):
 r=bundle['documents'][1];active={u['unit_id']:(u['origin_refs'],u['exposure_ref']) for u in r['units'] if u['state']=='active'}
 want={'unit:F:a-u':(['a','u'],'covered:Exact15000000')} if linked else {'unit:a':(['a'],'anchor:AtLeast0'),'unit:u':(['u'],'usage:Exact15000000')}
 owners={o['origin_ref']:o['active_high_unit_id'] for o in r['origins']}
 if active!=want or owners!=({'a':'unit:F:a-u','u':'unit:F:a-u'} if linked else {'a':'unit:a','u':'unit:u'}):F.add('VC07','R10 raw conditional anchor/usage active financial inventory')
 covers=r['coverage_decisions'];links=r['definite_links']
 if linked:
  if len(covers)!=1 or covers[0]['status']!='accepted' or covers[0]['anchor_origin_refs']!=['a'] or covers[0]['monetary_origin_refs']!=['u'] or covers[0]['definite_link_refs']!=['result-link:a-u'] or len(links)!=1 or links[0]['from_origin_ref']!='a' or links[0]['to_origin_ref']!='u':F.add('VC07','R10 raw exact-link conditional financial consequence')
 elif covers or links:F.add('VC07','R10 unlinked raw anchor cannot be covered/released')


def reservation_consequence(c,a,b,raw,F):
 from decimal import Decimal
 cid=c['case_id'];maximum=max(int(Decimal(str(json.loads(raw[o])['reserved_usd']))*1000000000) for o in b['reservation_origin_refs'])
 if a['exposure']['high']!={'kind':'at_least','basis':'proxy_reservation','nano':str(maximum)} or a['exposure']['pool']!='pi_api':F.add(cid,'R10 raw reservation maximum/conservative high')
 if a['settlement'].get('reserved')!=str(maximum):F.add(cid,'R10 raw reservation fixed settlement reserved amount')
 if a['obligation']!={'amount_source':'reservation_bound','state':'outstanding'} or a['replacement'].get('eligible') is not False or a['settlement'].get('decision')!='rejected':F.add(cid,'R10 raw reservation outstanding obligation/replacement')
 bundle=json.loads((root()/b['bundle_file']).read_text());inp,result=bundle['documents'];key=a['key'];obligation='validity:'+key;exposure='case:'+cid+'#attempt:'+key
 units=result['units'];ms=result.get('primary_memberships',[]);vs=result['primary_validities'];origins=sorted(raw)
 if (inp.get('kind')!='accounting_input_proof' or inp.get('id')!='input:'+cid or inp.get('lineage_id')!=cid or result.get('kind')!='accounting_result' or result.get('id')!='result:'+cid or result.get('lineage_id')!=cid or result.get('input_proof_ref')!=inp['id'] or a.get('accounting_result_ref')!=b['bundle_file']+'#'+result['id'] or sorted(a['evidence'])!=origins or units!=[{'unit_id':key,'state':'active','origin_refs':origins,'exposure_ref':exposure}] or len(ms)!=1 or ms[0]['obligation_id']!=obligation or ms[0]['settlement_classes']!=[{'origin_refs':['S1/turn-1:3'],'representative_origin_ref':'S1/turn-1:3'}] or ms[0]['usage_origin_refs']!=['S1/turn-1:3'] or len(vs)!=1 or vs[0]['obligation_id']!=obligation or vs[0]['validity_result']!='rejected' or vs[0]['applicable_check_refs']!=[exposure+'/settlement'] or vs[0]['distinct_settlement_refs']!=['S1/turn-1:3'] or result['coverage_decisions'] or sorted((x['origin_ref'],x['active_high_unit_id']) for x in result['origins'])!=[(o,key) for o in origins]):F.add(cid,'R10 reservation selected raw/result/unit/validity join')
 for ev in c['evaluations']:
  for ob in ev['expected']['objectives']:
   for label in ('owned_closed','owned_outstanding'):
    scope=cid+'/'+ev['name']+'/'+ob['objective']+'/'+label
    if ob[label].get('supported_low_evidence_ref')!=b['bundle_file']+'#'+scope:F.add(cid,'R10 reservation selected low reference join')


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
 if eq and (attempts[0]['valuation'].get('state')!='valued' or attempts[0]['valuation'].get('low')!='15000000' or attempts[0]['valuation'].get('high')!='15000000' or attempts[0]['valuation'].get('complete') is not True or attempts[0]['exposure']['low']!={'basis':'usage_valuation','nano':'15000000'}):F.add(cid,'R10 selected priceable usage minimum')
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
