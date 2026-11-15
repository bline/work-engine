import hashlib,json,stat
from pathlib import Path
w=Path(__file__).resolve().parent;r=w.parent/'subject';base=w.parent.parent/'low-target-repair-r1/subject';oldw=base.parent/'worker'
h=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
m=json.loads((w/'freeze/manifest.json').read_text());old=json.loads((oldw/'freeze/manifest.json').read_text())
assert m['entry_count']==old['entry_count']==967
for tree,manifest in [(r,m),(base,old)]:
 assert {str(p.relative_to(tree)) for p in tree.rglob('*') if p.is_file()}=={e['path'] for e in manifest['entries']}
 for e in manifest['entries']:
  p=tree/e['path'];assert h(p)==e['sha256'] and stat.S_IMODE(p.stat().st_mode)==e['mode'],str(p)
changes=json.loads((w/'freeze/changes.json').read_text());assert not changes['added'] and not changes['removed']
allowed={'tooling/spec-check/tests/independent_raw_axes.rs','validator/r9_pilot.py','validator/validate_corpus.py','validator/test_r10_qualification.py','validator/r10_admitted.json'}
assert set(changes['changed'])==allowed,changes
raw=[];schemas={}
for e in m['entries']:
 p=r/e['path'];q=base/e['path']
 if e['path'] not in allowed:assert p.read_bytes()==q.read_bytes(),e['path']
 if p.suffix=='.json':
  try:a=json.loads(p.read_text());b=json.loads(q.read_text())
  except ValueError:continue
  if isinstance(a,dict):
   for key in ('inputs','original_inputs'):
    if key in a:assert a[key]==b[key],e['path'];raw.append(e['path']+'/'+key)
 if e['path'].endswith('.schema.json'):schemas[e['path']]=h(p)
assert len(raw)==355
oldad=json.loads((base/'validator/r10_admitted.json').read_text());newad=json.loads((r/'validator/r10_admitted.json').read_text())
for entry in newad['admissions'].values():entry.pop('premise_record');entry.pop('premise_inventory')
newad.pop('premise_metadata_exclusions');newad.pop('rate_premises');assert newad==oldad
for dirname in ['qualification-1','qualification-2','final-custody','final-custody-2','join-custody']:
 for p in (w/dirname).glob('*.json'):
  record=json.loads(p.read_text())
  if 'leftover_processes' in record:assert not record['leftover_processes'],p
for name in ['build','joined-rust-positive','python-main-pilot']:
 record=json.loads((w/f'join-custody/{name}.json').read_text());assert record['returncode']==0
 for item in record['inputs']:assert h(Path(item['path']))==item['sha256'],item
proof={'manifest_sha256':h(w/'freeze/manifest.json'),'archive_sha256':h(w/'freeze/subject.tar'),'predecessor_manifest_sha256':h(oldw/'freeze/manifest.json'),'all967_current_files_match_freeze':True,'all967_predecessor_files_unchanged':True,'unchanged_files':962,'changed_files':sorted(allowed),'raw_field_count':len(raw),'raw_fields':raw,'all_original_case_and_control_bytes_premises_and_outputs_preserved':True,'contracts_specs_schemas_generated_dependency_bytes_unchanged':True,'all_prior_admission_fields_preserved':True,'schema_sha256':schemas,'binary_sha256':{str(p):h(p) for p in [w/'cargo-target/debug/pi-accounting-spec-check',w/'cargo-target/debug/deps/independent_raw_axes-3ee1ad2ed02bb648']},'join_summary_sha256':h(w/'join-custody/summary.json'),'qualification_summary_sha256':h(w/'final-custody-2/summary.json'),'failed_attempts_retained':'qualification-1'}
(w/'PRESERVATION.json').write_text(json.dumps(proof,indent=2)+'\n');print(json.dumps({'preservation':'pass','raw_fields':len(raw),'files':967,'changed':5,'manifest':proof['manifest_sha256']}))
