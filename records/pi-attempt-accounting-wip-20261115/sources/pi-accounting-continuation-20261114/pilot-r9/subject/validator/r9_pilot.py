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

def control_claim(claim,b,c,F):
 premise(c,b,F);a=next(x for x in c['expected']['attempts'] if x['key']==b['attempt_key']);raw=raw_map(c)
 if claim['relation']=='malformed_response_model':
  row=json.loads(raw[b['origin_ref']]);v=row.get('response_model')
  if b['response_model_state']!='present_number' or type(b['response_model_value']) is not int or b['response_model_value']!=7 or type(v) is not int or v!=7:F.add(c['case_id'],'R9 actual response_model presence/type/value premise')
  if a['valuation'].get('complete') or a['replacement']['eligible'] or a['valuation'].get('high')!='15000000' or a['exposure']['high']['nano']!='500000000' or a['exposure']['high']['kind']!='at_least':F.add(c['case_id'],'R9 model7 must retain supported15M/high500M and block complete release')
 elif claim['relation']=='reservation_classes':
  ls=[raw[x] for x in b['reservation_origin_refs']];rs=[json.loads(x) for x in ls]
  if b['expected_classes']!=2 or b['maximum_reservation_nano']!='500000000' or b['comparison_applicable'] is not False:F.add(c['case_id'],'R9 independent reservation cardinality premises disagree with raw')
  if len(set(ls))!=2 or any(x['reserved_usd']!=0.5 for x in rs):F.add(c['case_id'],'R9 two byte-distinct500M reservation premises')
  if a['replacement']['eligible'] or a['settlement']['decision']=='valid':F.add(c['case_id'],'R9 evidence cardinality2 prohibits comparison/release despite max500M')

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
 raw_bytes=(root()/'oracle/r8-controls/VC07-exact.raw.json').read_bytes()
 binding=next(b for c in load_registry() if c['relation']=='primary_anchor' for b in c['cases'] if b['case_id']=='VC07-read-failure-paired')
 if hashlib.sha256(raw_bytes).hexdigest()!=binding['conditional_raw_sha256']:F.add('VC07','R9 conditional raw premise custody changed')
 raw=json.loads(raw_bytes)
 bundle=json.loads((root()/'oracle/r8-controls/VC07-conditional.bundle.json').read_text())
 inp=bundle['documents'][0];assumption=raw['conditional_harness_assumptions']['exact_row_coreference'];want={'from':assumption['from'],'to':assumption['to']}
 grants=[g for g in inp['grants'] if g['guarantee_kind']=='exact_row_coreference']
 if len(grants)!=1 or grants[0].get('admitted_payload')!=want or grants[0]['exact_payload_ref']!='VC07-exact.raw.json':F.add('VC07','R9 conditional exact admitted payload differs from pinned raw assumption')
 reference_relations(bundle,F,'VC07-conditional')
 reference_relations(json.loads((root()/'oracle/r8-controls/VC07-unlinked.bundle.json').read_text()),F,'VC07-unlinked')
