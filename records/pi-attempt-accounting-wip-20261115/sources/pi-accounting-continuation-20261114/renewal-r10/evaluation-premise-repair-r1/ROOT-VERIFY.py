import hashlib,json,stat
from pathlib import Path
base=Path(__file__).resolve().parent
campaign=base.parent
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
roots=[(base/'subject',base/'worker/freeze/manifest.json'),(campaign/'low-target-repair-r1/subject',campaign/'low-target-repair-r1/worker/freeze/manifest.json'),(campaign/'reservation-repair-r1/subject',campaign/'reservation-repair-r1/worker/freeze/manifest.json'),(campaign/'subject',campaign/'freeze/manifest.json')]
verified=[]
for root,mp in roots:
 m=json.loads(mp.read_text());entries=m['entries']
 assert {str(p.relative_to(root)) for p in root.rglob('*') if p.is_file()}=={e['path'] for e in entries},root
 for e in entries:
  p=root/e['path'];assert sha(p)==e['sha256'] and stat.S_IMODE(p.stat().st_mode)==e['mode'],str(p)
 verified.append({'root':str(root),'manifest':str(mp),'sha256':sha(mp),'files':len(entries)})
new=base/'subject';old=campaign/'low-target-repair-r1/subject'
allow={'validator/r9_pilot.py','validator/validate_corpus.py','validator/r10_admitted.json','validator/test_r10_qualification.py','tooling/spec-check/tests/independent_raw_axes.rs'}
changed={str(p.relative_to(new)) for p in new.rglob('*') if p.is_file() and p.read_bytes()!=(old/p.relative_to(new)).read_bytes()}
assert changed==allow,changed
raw=0
for p in new.rglob('*.json'):
 try:d=json.loads(p.read_text());o=json.loads((old/p.relative_to(new)).read_text())
 except ValueError:continue
 if isinstance(d,dict):
  for k in ('inputs','original_inputs'):
   if k in d:assert d[k]==o[k],str(p);raw+=1
assert raw==355,raw
a=json.loads((new/'validator/r10_admitted.json').read_text());prior=json.loads((old/'validator/r10_admitted.json').read_text())
assert len(a['admissions'])==11
for bundle,entry in a['admissions'].items():
 c=json.loads((new/entry.get('case_file',entry.get('conditional_raw_file'))).read_text())
 assert sorted(c)==entry['premise_inventory']
 projection={}
 for key,value in c.items():
  if key in set(a['premise_metadata_exclusions'])|{'expected'}:continue
  if key=='evaluations':value=[{k:v for k,v in ev.items() if k!='expected'} for ev in value]
  projection[key]=value
 assert json.dumps(projection,sort_keys=True)==json.dumps(entry['premise_record'],sort_keys=True),bundle
 for k in ('premise_record','premise_inventory'):entry.pop(k)
for path,want in a.pop('rate_premises').items():assert json.loads((new/path).read_text())==want,path
a.pop('premise_metadata_exclusions');assert a==prior
records=[]
for folder in ['worker/final-custody-2','worker/join-custody']:
 sm=base/folder/'summary.json';s=json.loads(sm.read_text());assert s['status']=='passed' and not s['not_run']
 assert sha(Path(s['plan']))==s['plan_sha256']
 for step in s['steps']:
  rp=Path(step['record']);assert sha(rp)==step['record_sha256'];r=json.loads(rp.read_text());assert r['returncode']==0 and not r['leftover_processes']
  for item in r['inputs']:assert sha(Path(item['path']))==item['sha256'],item['path']
 records.append({'summary':str(sm),'sha256':sha(sm),'steps':len(s['steps'])})
pres=json.loads((base/'worker/PRESERVATION.json').read_text())
for path,want in pres['binary_sha256'].items():assert sha(Path(path))==want,path
assert sha(base/'worker/freeze/subject.tar')==pres['archive_sha256']
assert not Path('/tmp/r10-rust-3764316').exists()
assert (base/'worker/failed-attempt-rust-3764316').is_dir()
proof={'status':'pass','verified_frozen_trees':verified,'file_count':sum(t['files'] for t in verified),'changed':sorted(changed),'raw_fields':raw,'complete_selected_premises':11,'old_admission_fields_unchanged':True,'worker_custody':records,'binary_sha256':pres['binary_sha256'],'worker_final_native':sha(base/'WORKER-FINAL-NATIVE.json'),'worker_result':sha(base/'worker/RESULT.md'),'failed_attempt_relocated':True}
(base/'ROOT-VERIFY.json').write_text(json.dumps(proof,indent=2)+'\n');print(json.dumps(proof))
