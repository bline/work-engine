from pathlib import Path
import subprocess,json,sys,datetime,hashlib
root=Path('/home/bline/cbm-work/pi-accounting-continuation-20261114'); repo=Path('/home/bline/code/work-engine'); host=repo/'skills/manual-orchestration/scripts/role_host.py'; budget=repo/'skills/manual-orchestration/scripts/scope_budget.py'; sidroot='01a1d6e2-32ba-7701-9115-740dcb41af11'
c=json.loads((root/'reviews/implementation-r8/dispatch/ORGANIZATION-CANDIDATE-R2.json').read_text()); ledgerpath=root/'review-assignments.json'; ledger=json.loads(ledgerpath.read_text())
for role in sys.argv[1:]:
 spec=c['roles'][role]; assert ledger['roles'].get(role,{}).get('state','not_launched')=='not_launched',role
 packet=Path(spec['packet']);assert hashlib.sha256(packet.read_bytes()).hexdigest()==spec['packet_sha256']
 args=['python3',str(host),'launch','codex','--cwd',str(repo),'--name',f'PI S1 R8 {role} independent review','--prompt-file',str(packet),'--parent',sidroot,'--host-arg=-m','--host-arg=gpt-6-astra','--host-arg=-c',f'--host-arg=model_reasoning_effort="{spec["effort"]}"','--host-arg=-c','--host-arg=service_tier="default"','--host-arg=--sandbox','--host-arg=read-only','--host-arg=--ask-for-approval','--host-arg=never']
 started=datetime.datetime.now(datetime.timezone.utc).isoformat();p=subprocess.run(args,text=True,capture_output=True);(root/f'control/{role}-launch-r2.txt').write_text(p.stdout+p.stderr)
 if p.returncode:raise RuntimeError(f'{role} launch exit{p.returncode}: '+p.stdout+p.stderr)
 data=json.loads(p.stdout);(root/f'control/{role}-launch-r2.json').write_text(json.dumps(data,indent=2)+'\n');sid=data['session_id']
 entry=ledger['roles'].setdefault(role,{});entry.update({'state':'launched_identity_pending','launch':data,'session_id':sid,'launch_requested_at':started,'packet':str(packet),'packet_sha256':spec['packet_sha256'],'model_requested':'gpt-6-astra','effort':spec['effort'],'service_tier_requested':'default','max_proxy_usd':spec['cap'],'collector':sidroot,'fresh_reason':'New separate top-level context, no builder/designer fork; exactR2 organization; requested native read-only/never.','collection_parent':sidroot})
 ledgerpath.write_text(json.dumps(ledger,indent=2)+'\n')
 q=subprocess.run(['python3',str(budget),'cost-limit','--record',str(root/'assignment.json'),'--pool','codex','--node',sid,'--max-usd',str(spec['cap']),'--mark-usd',str(max(4,spec['cap']-2)),'--set-by','supervisor','--reason',f'ExactR2 {role} bounded independent review; no repair/delegation; original100/deadline retained'],text=True,capture_output=True);(root/f'control/{role}-budget-binding.json').write_text(q.stdout+q.stderr);assert q.returncode==0
 print(json.dumps({'role':role,'session_id':sid,'launch':data,'cap':spec['cap']}),flush=True)
