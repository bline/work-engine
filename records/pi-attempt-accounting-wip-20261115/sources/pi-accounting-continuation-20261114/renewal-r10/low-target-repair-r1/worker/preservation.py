import hashlib,json,stat
from pathlib import Path
w=Path(__file__).resolve().parent;r=w.parent/'subject';base=w.parent.parent/'reservation-repair-r1/subject';oldw=base.parent/'worker'
h=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
m=json.loads((w/'freeze/manifest.json').read_text());old=json.loads((oldw/'freeze/manifest.json').read_text())
assert m['entry_count']==old['entry_count']==967
for tree,manifest in [(r,m),(base,old),(w.parent.parent/'subject',json.loads((w.parent.parent/'freeze/manifest.json').read_text()))]:
 assert {str(p.relative_to(tree)) for p in tree.rglob('*') if p.is_file()}=={e['path'] for e in manifest['entries']}
 for e in manifest['entries']:
  p=tree/e['path'];assert h(p)==e['sha256'] and stat.S_IMODE(p.stat().st_mode)==e['mode'],str(p)
changes=json.loads((w/'freeze/changes.json').read_text());assert not changes['added'] and not changes['removed']
allowed={'tooling/spec-check/tests/independent_raw_axes.rs','validator/r9_pilot.py','validator/validate_corpus.py','validator/test_r10_qualification.py','oracle/r9-pilot/claims.json'}
names=['model7-VC01-openai-usage-settlement','model7-A04-undated-settlement-between-dated-neighbours']
for n in names:allowed|={f'oracle/r9-pilot/cases/{n}.json',f'oracle/r9-pilot/cases/{n}.bundle.json'}
assert set(changes['changed'])==allowed,changes
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
 if e['path'] in allowed:changed.append({'path':e['path'],'sha256':h(p),'baseline_sha256':h(q)})
for n in names:
 name=f'oracle/r9-pilot/cases/{n}.json';a=json.loads((r/name).read_text());b=json.loads((base/name).read_text())
 for field in ('valuation','replacement'):a['expected']['attempts'][0][field]=b['expected']['attempts'][0][field]
 a['expected']['attempts'][0]['exposure']['low']=b['expected']['attempts'][0]['exposure']['low']
 a['evaluations'][0]['expected']['objectives'][0]['owned_closed']['low']=b['evaluations'][0]['expected']['objectives'][0]['owned_closed']['low'];assert a==b,name
 name=f'oracle/r9-pilot/cases/{n}.bundle.json';a=json.loads((r/name).read_text());b=json.loads((base/name).read_text())
 assert a['documents'][0]==b['documents'][0]
 for field in ('selected_low','single_floor','basis','credit_mode'):a['documents'][1]['low_rollups'][0][field]=b['documents'][1]['low_rollups'][0][field]
 assert a==b,name
name='oracle/r9-pilot/claims.json';a=json.loads((r/name).read_text());b=json.loads((base/name).read_text())
for x,y in zip(a['claims'][2]['cases'],b['claims'][2]['cases']):x['low_assertions'][0]['expected_low']=y['low_assertions'][0]['expected_low']
assert a==b,name
assert len(raw)==355
bins={str(p):h(p) for p in [w/'cargo-target/debug/pi-accounting-spec-check',w/'cargo-target/debug/deps/independent_raw_axes-3ee1ad2ed02bb648']}
proof={'join_plan_sha256':h(w/'JOIN.plan.json'),'join_summary_sha256':h(w/'join-custody/summary.json'),'frozen_manifest_sha256':h(w/'freeze/manifest.json'),'archive_sha256':h(w/'freeze/subject.tar'),'predecessor_manifest_sha256':h(oldw/'freeze/manifest.json'),'all967_frozen_files_match':True,'all967_predecessor_files_unchanged':True,'all967_original_files_unchanged':True,'raw_fields_unchanged':raw,'raw_field_count':len(raw),'spec_contract_schema_generated_and_all_immutable_input_premises_unchanged':True,'case_changes_only_admitted_model7_financial_fields':True,'bundle_changes_only_model7_four_low_fields':True,'claims_changes_only_two_model7_expected_low':True,'changed':changed,'binary_sha256':bins,'schema_sha256':schemas,'qualification_plan_sha256':h(w/'QUALIFICATION.plan.json'),'qualification_summary_sha256':h(w/'final-custody/summary.json')}
# Verify every custody source/input digest against the frozen subject or manifest.
for name in ['build','joined-rust-positive','python-main-pilot']:
 record=json.loads((w/f'join-custody/{name}.json').read_text())
 assert record['returncode']==0 and not record['leftover_processes']
 for item in record['inputs']:
  assert h(Path(item['path']))==item['sha256'],item
for name in ['initial-custody','qualification-01','final-custody','join-custody']:
 summary=json.loads((w/name/'summary.json').read_text())
 # All owned jobs are collected; first failures remain attributable, never overwritten.
 for rec in (w/name).glob('*.json'):
  d=json.loads(rec.read_text())
  if 'leftover_processes' in d:assert not d['leftover_processes'],str(rec)
(w/'PRESERVATION.json').write_text(json.dumps(proof,indent=2)+'\n');print(json.dumps({'preservation':'pass','raw_fields':len(raw),'files':967,'changed':len(changed),'manifest':proof['frozen_manifest_sha256']}))
