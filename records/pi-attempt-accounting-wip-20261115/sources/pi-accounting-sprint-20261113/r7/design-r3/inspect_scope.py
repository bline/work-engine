import json, hashlib
from pathlib import Path
root=Path('/home/bline/cbm-work/p137-s1r7/s1')
out={'kind':'raw identity census only; no classifier, financial reducer, or writer reachability proof','cases':0,'sources':0,'source_key_failures':[],'incarnation_fields':[],'rid_claims':0,'collision_groups':[],'scope_basis':'existing case container, session_id and log name; authentic one-proxy-incarnation provenance not supplied by these keys'}
for p in sorted((root/'corpus/cases').glob('*.json')):
 c=json.loads(p.read_text()); out['cases']+=1; seen=set()
 for s in c['inputs']['sessions']:
  for name,log in s.get('logs',{}).items():
   key=(s.get('session_id'),name)
   if key in seen or not key[0]:out['source_key_failures'].append([c['case_id'],key])
   seen.add(key);out['sources']+=1; groups={}
   for n,line in enumerate(log['lines'],1):
    try:r=json.loads(line)
    except (ValueError,TypeError):continue
    if not isinstance(r,dict):continue
    if any(k in r for k in ['incarnation','attempt_id','producer_attempt']):out['incarnation_fields'].append([c['case_id'],name,n])
    usd=r.get('usd'); usd=usd if isinstance(usd,dict) else {}
    rid=r.get('rid') if r.get('event')=='usd_reserved' else usd.get('rid')
    if type(rid) is int and rid>0:
     out['rid_claims']+=1;groups.setdefault(rid,[]).append({'line':n,'event':r.get('event'),'n':r.get('n'),'row_sha256':hashlib.sha256(line.encode()).hexdigest()})
   for rid,rows in groups.items():
    rr={r['row_sha256'] for r in rows if r['event']=='usd_reserved'}
    ss={r['row_sha256'] for r in rows if r['event'] in ['request','upstream_error']}
    if len(rr)>1 or len(ss)>1:out['collision_groups'].append({'case':c['case_id'],'source':key,'rid':rid,'distinct_reservations':len(rr),'distinct_settlements':len(ss),'rows':rows})
print(json.dumps(out,indent=2))
