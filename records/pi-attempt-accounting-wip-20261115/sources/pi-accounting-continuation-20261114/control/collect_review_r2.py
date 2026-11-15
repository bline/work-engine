from pathlib import Path
import json,hashlib,sys,datetime
root=Path('/home/bline/cbm-work/pi-accounting-continuation-20261114');lp=root/'review-assignments.json';ledger=json.loads(lp.read_text())
for role in sys.argv[1:]:
 e=ledger['roles'][role];p=Path(e['rollout']);finals=[]
 for n,b in enumerate(p.read_bytes().splitlines(keepends=True),1):
  d=json.loads(b);x=d.get('payload',{})
  if d.get('type')=='response_item' and x.get('role')=='assistant' and x.get('phase')=='final_answer':finals.append((n,b,x))
 assert finals,role
 n,b,x=finals[-1];txt='\n'.join(c.get('text','') for c in x.get('content',[]) if c.get('type')=='output_text');assert len(txt)>50
 d=root/('reviews/design-r7' if role=='D7' else 'reviews/implementation-r8');out=d/f'{role}-REVIEW.md';assert not out.exists(),str(out);out.write_text(txt+'\n')
 record={'session_id':e['session_id'],'turn_id':e['launch']['turn_id'],'status':'completed','native_rollout':str(p),'native_line':n,'native_line_sha256':hashlib.sha256(b).hexdigest(),'report':str(out),'report_sha256':hashlib.sha256(out.read_bytes()).hexdigest(),'collected_at':datetime.datetime.now(datetime.timezone.utc).isoformat(),'collection':'Native wait_threads completed exactturn; exact native rollout final sealed','assessment':'pending full report reading','identity_binding':'Consult native final; initial delivery refusedbusy; root file published and named in packet'}
 (d/f'{role}-COLLECTION.json').write_text(json.dumps(record,indent=2)+'\n');e.update(state='final_collected_unassessed',report=str(out),report_sha256=record['report_sha256'],collection=str(d/f'{role}-COLLECTION.json'));lp.write_text(json.dumps(ledger,indent=2)+'\n')
 print(json.dumps(record),flush=True); print(txt,flush=True)
