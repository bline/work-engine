#!/usr/bin/env python3
"""One-off revision2 full cached native package offline bootstrap."""
import pathlib,subprocess,json,time,hashlib
S=pathlib.Path('/home/bline/.local/state/work-engine/manual-host-launch-remediation-20261103/docker/clean2')
T=S.parent/'tools';P='/usr/lib/node_modules/@openai/codex/node_modules/@openai/codex-linux-x64/vendor/x86_64-unknown-linux-musl'
rows=[]
def run(label,a,timeout=30):
 p=subprocess.run(a,capture_output=True,text=True,timeout=timeout);(S/(label+'.stdout')).write_text(p.stdout);(S/(label+'.stderr')).write_text(p.stderr);rows.append({'label':label,'argv':a,'exit':p.returncode,'stdout_sha256':hashlib.sha256(p.stdout.encode()).hexdigest(),'stderr_sha256':hashlib.sha256(p.stderr.encode()).hexdigest()});(S/'commands.json').write_text(json.dumps(rows,indent=2)+'\n');print(label,p.returncode,flush=True);assert p.returncode==0,p.stderr;return p.stdout
N='we-manual-host-docker-20261103-clean2-offline'
def create(n,network,auth=False):
 a=['docker','create','--name',n,'--user','1000:1000','--network',network,'--cap-drop','ALL','--security-opt','no-new-privileges','--pids-limit','128','--memory','1g','--read-only','--tmpfs','/tmp:rw,nosuid,nodev,size=64m','--mount','type=bind,src='+P+',dst=/opt/codex,readonly','--mount','type=bind,src='+str(S/'runtime')+',dst=/state','--mount','type=bind,src='+str(T)+',dst=/opt/probe,readonly','-e','PATH=/opt/codex/bin:/usr/local/bin:/usr/bin:/bin','-e','CODEX_HOME=/state/codex','-e','ROLE_HOST_CODEX_SOCKET=/state/codex/control.sock','-w','/state/workspace']
 if auth:a+=['--mount','type=bind,src=/home/bline/.codex/auth.json,dst=/state/codex/auth.json,readonly','--mount','type=bind,src=/home/bline/.codex/sessions/we-manual-host-docker-20261103-clean2,dst=/state/codex/sessions']
 a+=['python:3.12-slim','sh','-c','exec codex app-server --listen unix:///state/codex/control.sock >/state/server.stdout 2>/state/server.stderr'];return a
run('create-offline',create(N,'none'));run('start-offline',['docker','start',N])
run('native-version',['docker','exec',N,'codex','--version']);run('companion-help',['docker','exec',N,'/opt/codex/bin/codex-code-mode-host','--help'])
for i in range(100):
 p=subprocess.run(['docker','exec',N,'test','-S','/state/codex/control.sock'],capture_output=True)
 if p.returncode==0:break
 time.sleep(.1)
code="import sys,pathlib,json;sys.path.insert(0,'/opt/probe');import codex_app;c=codex_app.Connection(pathlib.Path('/state/codex/control.sock')); print(json.dumps(c.call('thread/list',{'limit':5}))); r=c.call('command/exec',{'command':['/bin/sh','-c','printf bootstrap-ok > offline-result.txt; cat fixture.txt'],'cwd':'/state/workspace','sandboxPolicy':{'type':'dangerFullAccess'},'timeoutMs':3000});print(json.dumps(r));assert r['exitCode']==0;assert pathlib.Path('/state/workspace/offline-result.txt').read_text()=='bootstrap-ok';assert not pathlib.Path('/home/bline').exists();assert not pathlib.Path('/var/run/docker.sock').exists();assert not pathlib.Path('/state/codex/auth.json').exists();c.close()"
run('rpc-shell-files',['docker','exec',N,'python3','-c',code])
run('inspect-offline',['docker','inspect',N]);run('stop-offline',['docker','stop','--timeout','10',N]);run('wait-offline',['docker','wait',N]);run('remove-offline',['docker','rm',N])
E=pathlib.Path('/home/bline/.codex/sessions/we-manual-host-docker-20261103-clean2');E.mkdir()
N='we-manual-host-docker-20261103-clean2-retained';cid=run('create-retained',create(N,'bridge',True)).strip();run('start-retained',['docker','start',N]);run('inspect-retained',['docker','inspect',N])
(S/'offline-result.json').write_text(json.dumps({'offline_pass':True,'full_distribution':P,'container_id':cid,'native_version':'0.166.0','inference_calls':0,'credential_route':'single readonly native auth file bind only for retained container'},indent=2)+'\n')
