import json,hashlib,subprocess
from pathlib import Path
R=Path('/home/bline/cbm-work/org-pi-review-continuation-20261114/implementation-r6/preparation-03');O=R/'diagnostic-07-preparation';O.mkdir();B=json.loads((R/'verifier-bootstrap-check-01.json').read_text())['retained']['path'];C='/home/bline/code/work-engine/skills/manual-orchestration/scripts/custody.py';read=lambda p:json.loads(p.read_text());sha=lambda p:hashlib.sha256(Path(p).read_bytes()).hexdigest()
m=json.loads(json.dumps(read(R/'obligation-map.json')).replace('inv-r9-qualify-01','inv-r9-diagnostic-07'));M=O/'obligation-map.json';S=O/'source-manifest.json';P=O/'plan-02.json';INV='inv-r9-diagnostic-07';RUN=R/'runs'/INV;M.write_text(json.dumps(m,indent=1)+'\n')
subprocess.run([B,'manifest','--map',str(M),'--out',str(S)],check=True)
subprocess.run([B,'make-plan','--map',str(M),'--invocation-id',INV,'--run-dir',str(RUN/'run'),'--records-dir',str(RUN/'custody'),'--expect-map-sha256',sha(M),'--expect-manifest-sha256',sha(S),'--out',str(P)],check=True)
def gate(mode):return [B,mode,'--map',str(M),'--plan',str(P),'--manifest',str(S),'--expect-map-sha256',sha(M),'--expect-plan-sha256',sha(P),'--expect-manifest-sha256',sha(S),'--invocation-id',INV,'--run-dir',str(RUN/'run'),'--records-dir',str(RUN/'custody'),'--out',str(O/('preflight-02.json'if mode=='preflight'else 'postflight.json'))]
dig=[B,str(M),str(S),str(P),C];steps=[]
for name,args in [('preflight',gate('preflight')),('common-plan',['/usr/bin/python3','-I',C,'run','--plan',str(P),'--out',str(RUN/'custody')]),('postflight',gate('postflight'))]:steps.append({'name':name,'argv':args,'digest':dig,'timeout':1800,'kill_leftovers':True})
(R/'diagnostic-07-pipeline-plan.json').write_text(json.dumps({'env_clear':True,'cwd':str(R),'env':{'PATH':'/usr/bin:/bin'},'steps':steps},indent=1)+'\n')
print('diagnostic07 prospective pipeline ready')
