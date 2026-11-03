#!/usr/bin/env python3
"""One-off dated feasibility probe; no inference, downloads or global changes."""
import hashlib,json,pathlib,subprocess,time
S=pathlib.Path('/home/bline/.local/state/work-engine/manual-host-launch-remediation-20261103/docker')
N='we-manual-host-docker-20261103-probe'
B='/usr/lib/node_modules/@openai/codex/node_modules/@openai/codex-linux-x64/vendor/x86_64-unknown-linux-musl/bin/codex'
records=[]
def run(label,args,timeout=40):
 p=subprocess.run(args,capture_output=True,text=True,timeout=timeout)
 (S/(label+'.stdout')).write_text(p.stdout);(S/(label+'.stderr')).write_text(p.stderr)
 records.append({'label':label,'argv':args,'exit':p.returncode,'stdout_sha256':hashlib.sha256(p.stdout.encode()).hexdigest(),'stderr_sha256':hashlib.sha256(p.stderr.encode()).hexdigest()})
 (S/'offline-commands.json').write_text(json.dumps(records,indent=2)+'\n')
 print(label,p.returncode,flush=True)
 return p
import sys
if '--finish' not in sys.argv:
 assert run('create',['docker','create','--name',N,'--user','1000:1000','--network','none','--cap-drop','ALL','--security-opt','no-new-privileges','--pids-limit','128','--memory','1g','--read-only','--tmpfs','/tmp:rw,nosuid,nodev,size=64m','--mount','type=bind,src='+B+',dst=/usr/local/bin/codex,readonly','--mount','type=bind,src='+str(S/'runtime')+',dst=/state','--mount','type=bind,src='+str(S/'tools')+',dst=/opt/probe,readonly','-e','CODEX_HOME=/state/codex','-e','ROLE_HOST_CODEX_SOCKET=/state/codex/control.sock','-w','/state/workspace','python:3.12-slim','sleep','infinity']).returncode==0
 assert run('start',['docker','start',N]).returncode==0
 assert run('native-version',['docker','exec',N,'codex','--version']).returncode==0
 # OSS local provider, network disabled, timeout: deterministic native exec startup, no remote inference.
 run('exec-offline',['docker','exec',N,'timeout','8','codex','exec','--oss','--local-provider','ollama','--skip-git-repo-check','--json','Respond OK'],15)
 assert run('exec-socket-check',['docker','exec',N,'python3','-c',"import pathlib,json; p=pathlib.Path('/state/codex'); print(json.dumps([str(x) for x in p.rglob('*') if x.is_socket()])); assert not any(x.is_socket() for x in p.rglob('*'))"]).returncode==0
 assert run('explicit-server',['docker','exec','-d',N,'sh','-c','exec codex app-server --listen unix:///state/codex/control.sock >/state/app-server.stdout 2>/state/app-server.stderr']).returncode==0
 for i in range(100):
  if run('socket-ready-'+str(i),['docker','exec',N,'test','-S','/state/codex/control.sock']).returncode==0:break
  time.sleep(.1)
 assert run('socket-ready-final',['docker','exec',N,'test','-S','/state/codex/control.sock']).returncode==0
else:
 records=json.loads((S/'offline-commands.json').read_text())
code="import sys,json,pathlib;sys.path.insert(0,'/opt/probe');import codex_app; c=codex_app.Connection(pathlib.Path('/state/codex/control.sock')); print(json.dumps(c.call('thread/list',{'limit':5}))); c.close(); assert pathlib.Path('/state/workspace/fixture.txt').read_text().strip()=='public fixture: sum(2, 2)'; assert not pathlib.Path('/home/bline').exists(); assert not pathlib.Path('/var/run/docker.sock').exists(); assert not pathlib.Path('/state/codex/auth.json').exists(); pathlib.Path('/state/durable.txt').write_text('retained across exec clients'+chr(10))"
assert run('rpc-boundary',['docker','exec',N,'python3','-c',code]).returncode==0
assert run('durability',['docker','exec',N,'cat','/state/durable.txt']).stdout=='retained across exec clients\n'
run('inspect',['docker','inspect',N])
(S/'offline-result.json').write_text(json.dumps({'offline_pass':True,'container':N,'container_id':(S/'create.stdout').read_text().strip(),'exec_publishes_socket':False,'explicit_app_server_socket':True,'network':'none','inference_calls':0,'live_container':True},indent=2)+'\n')
