"""Finite revision metadata repair; amounts, inputs and references are invariant."""
import json,pathlib,copy,hashlib,tarfile
r=pathlib.Path('/home/bline/cbm-work/pi-accounting-continuation-20261114');s=r/'realization/s1';b=r/'completion/B';rows=json.loads((r/'completion/PARTITIONS.json').read_text())['partitions']['B'];records=[];count=0
with tarfile.open(r/'completion/baseline-freeze/subject.tar') as archive:
 for row in rows:
  p=s/row['path'];before=p.read_bytes();c=json.loads(before);base=json.load(archive.extractfile(row['path']));old=copy.deepcopy(c);entries=[]
  rule='R8 finite completion B exact refs; authored01a1d840-f94e-7631-aef9-15c312b1ae20; derivation oracle/r8-completion-B/DERIVATIONS.json#'+c['case_id']
  for a in c['expected']['attempts']:entries.append({'field':'attempts['+a['key']+'].accounting_result_ref','from':'__absent__','to':a['accounting_result_ref'],'rule':rule})
  for ev in c['evaluations']:
   for o in ev['expected']['objectives']:
    for label in ['owned_closed','owned_outstanding']:entries.append({'field':ev['name']+'/'+o['objective']+'.'+label+'.supported_low_evidence_ref','from':'__absent__','to':o[label]['supported_low_evidence_ref'],'rule':rule})
  existing=c.setdefault('revisions_r8',[]);assert not any(x['rule'].startswith('R8 finite completion B exact refs;')for x in existing)
  existing.extend(entries);check=copy.deepcopy(c);check['revisions_r8']=check['revisions_r8'][:-len(entries)]
  if 'revisions_r8' not in old:check.pop('revisions_r8')
  assert check==old
  # Preserve every preexisting whole-expected/evaluations revision payload byte-for-value.
  assert c['revisions_r8'][:-len(entries)]==base.get('revisions_r8',[])
  p.write_text(json.dumps(c,indent=1,sort_keys=True)+'\n');count+=len(entries)
  records.append({'path':row['path'],'before_sha256':hashlib.sha256(before).hexdigest(),'after_sha256':hashlib.sha256(p.read_bytes()).hexdigest(),'preserved_preexisting_revisions':len(base.get('revisions_r8',[])),'new_reversible_entries':entries,'only_revision_metadata_changed':True})
(b/'REVISION-REPAIR.json').write_text(json.dumps({'author':'01a1d840-f94e-7631-aef9-15c312b1ae20','root_authority':'Retained revision-metadata-only task after joined revisions gate failure','entries_added':count,'cases':records,'first_join_failure':'completion/integration/join-1/revisions.stdout'},indent=2)+'\n');print('revision-only repair',len(records),'cases',count,'entries')
