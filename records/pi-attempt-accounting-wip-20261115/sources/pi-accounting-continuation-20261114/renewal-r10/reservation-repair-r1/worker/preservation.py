import hashlib,json,stat
from pathlib import Path
w=Path(__file__).resolve().parent;r=w.parent/'subject';base=w.parent.parent/'subject'
h=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
m=json.loads((w/'freeze/manifest.json').read_text());old=json.loads((w.parent.parent/'freeze/manifest.json').read_text())
assert m['entry_count']==old['entry_count']==967
current={str(p.relative_to(r)) for p in r.rglob('*') if p.is_file()}
assert current=={e['path'] for e in m['entries']}
for e in m['entries']:
 p=r/e['path'];assert h(p)==e['sha256'] and stat.S_IMODE(p.stat().st_mode)==e['mode'],e['path']
for e in old['entries']:assert h(base/e['path'])==e['sha256'],('immutable predecessor changed',e['path'])
changes=json.loads((w/'freeze/changes.json').read_text());assert len(changes['changed'])==7 and not changes['added'] and not changes['removed']
raw=[];schemas={};changed=[]
for e in m['entries']:
 p=r/e['path'];q=base/e['path']
 if p.suffix=='.json':
  try:a=json.loads(p.read_text());b=json.loads(q.read_text())
  except ValueError:continue
  if isinstance(a,dict):
   for key in ('inputs','original_inputs'):
    if key in a:assert a[key]==b[key],e['path'];raw.append(e['path']+'/'+key)
 if e['path'].startswith(('spec/','contract/')):assert h(p)==h(q),e['path']
 if e['path'].endswith('.schema.json'):schemas[e['path']]=h(p)
 if e['path'] in changes['changed']:changed.append({'path':e['path'],'sha256':h(p),'baseline_sha256':h(q)})
for name in changes['changed']:
 if not name.endswith('.json'):continue
 a=json.loads((r/name).read_text());b=json.loads((base/name).read_text())
 # All selected case changes are exactly current reference projections.
 for at,bt in zip(a['expected']['attempts'],b['expected']['attempts']):at['accounting_result_ref']=bt['accounting_result_ref']
 for ev,be in zip(a['evaluations'],b['evaluations']):
  for ob,bo in zip(ev['expected']['objectives'],be['expected']['objectives']):
   for label in ('owned_closed','owned_outstanding'):ob[label]['supported_low_evidence_ref']=bo[label]['supported_low_evidence_ref']
 assert a==b,name
bins={str(p):h(p) for p in [w/'cargo-target/debug/pi-accounting-spec-check',w/'cargo-target/debug/deps/independent_raw_axes-3ee1ad2ed02bb648']}
proof={'frozen_manifest_sha256':h(w/'freeze/manifest.json'),'predecessor_manifest_sha256':h(w.parent.parent/'freeze/manifest.json'),'all967_frozen_files_match':True,'all967_predecessor_files_match_original_freeze':True,'raw_fields_unchanged':raw,'raw_field_count':len(raw),'spec_and_contract_unchanged':True,'case_changes_only_selected_current_reference_projection':True,'changed':changed,'binary_sha256':bins,'schema_sha256':schemas,'qualification_plan_sha256':h(w/'QUALIFICATION.plan.json'),'qualification_summary_sha256':h(w/'final-custody/summary.json')}
(w/'PRESERVATION.json').write_text(json.dumps(proof,indent=2)+'\n');print(json.dumps({'preservation':'pass','raw_fields':len(raw),'files':967,'changed':len(changed),'manifest':proof['frozen_manifest_sha256']}))
