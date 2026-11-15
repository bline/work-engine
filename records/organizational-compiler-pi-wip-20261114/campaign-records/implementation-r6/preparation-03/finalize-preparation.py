#!/usr/bin/env python3
"""Reconcile immutable R9 preparation for root inspection; launch parameters are never executed."""
import hashlib,json,os,tarfile
from pathlib import Path
R=Path('/home/bline/cbm-work/org-pi-review-continuation-20261114/implementation-r6/preparation-03');N=Path('/home/bline/cbm-work/org-pi-review-continuation-20261114');P=Path('/home/bline/cbm-work/wt-p1-vertical-20261113');D=P/'rust/crates/work-engine-review-delivery';Q=N/'realization-r9/evidence-gate';C=Path('/home/bline/code/work-engine/skills/manual-orchestration/scripts/custody.py')
sha=lambda p:hashlib.sha256(Path(p).read_bytes()).hexdigest();read=lambda p:json.loads(Path(p).read_text())
def write(p,x):
 with Path(p).open('x')as f:json.dump(x,f,indent=1);f.write('\n')
def artifact(p):return {'path':str(p),'sha256':sha(p)}
def typed(p):
 p=Path(p)
 return {'kind':'symlink','sha256':hashlib.sha256(os.fsencode(os.readlink(p))).hexdigest(),'link_target':os.readlink(p)}if p.is_symlink()else{'kind':'file','sha256':sha(p)}
m=read(R/'obligation-map.json');sm=read(R/'source-manifest.json');vb=read(R/'verifier-bootstrap-check-01.json');BIN=vb['retained']['path'];assert typed(BIN)['sha256']==vb['retained']['sha256'];assert Path(BIN).stat().st_nlink==1 and Path(BIN).stat().st_mode&0o777==0o555
assert not (R/'runs/inv-r9-qualify-01').exists() and not (R/'root-validation-r9').exists()
assert sha(R/'obligation-map.json')==sm['map_sha256']
source={x['path']:x['sha256']for x in sm['entries']};assert all(sha(p)==h for p,h in source.items())
assert all(typed(p)==pin for p,pin in m['external_inputs'].items())
bootstrap_inputs=read(R/'verifier-build-01/build.json')['inputs'];assert all(e['path']in source or e['path']in m['external_inputs']for e in bootstrap_inputs)
freezes={};archives={}
for name in ['delivery','qualifier','source-closure','dependency-closure']:
 root=R/('qualifier-source-freeze-02' if name=='qualifier' else name+'-freeze');manifest=read(root/'manifest.json');entries={e['path']:e for e in manifest['entries']}
 with tarfile.open(root/'subject.tar')as t:
  members=t.getmembers();assert {x.name for x in members}==set(entries)
  for x in members:
   e=entries[x.name]
   if e['type']=='symlink':assert x.issym()and hashlib.sha256(os.fsencode(x.linkname)).hexdigest()==e['sha256']
   else:
    assert x.isfile();f=t.extractfile(x);assert f is not None and hashlib.sha256(f.read()).hexdigest()==e['sha256']
 archives[name]={'entries':len(entries),'archive_exact':True}
 freezes[name]={'manifest':artifact(root/'manifest.json'),'archive':artifact(root/'subject.tar')}
 if name=='source-closure':assert {p.lstrip('/'):h for p,h in source.items()}=={p:e['sha256']for p,e in entries.items()}
 if name in ['delivery','qualifier']:
  live=D if name=='delivery'else Q
  for p,e in entries.items():assert sha(live/p)==e['sha256']
idx=read(R/'dependency-object-closure-01/index.json')['entries'];dep={e['path']:e for e in read(R/'dependency-closure-freeze/manifest.json')['entries']};assert len(idx)==len(m['external_inputs'])
for e in idx:
 assert e['pin']==m['external_inputs'][e['original_path']];f=dep[e['stored_path']];assert f['sha256']==e['pin']['sha256'] and f['type']==('symlink'if e['pin']['kind']=='symlink'else 'file')
common=R/'runs/inv-r9-diagnostic-07/custody';summ=read(common/'summary.json');post=read(R/'diagnostic-07-preparation/postflight.json');assert post['qualified']and not post['incomplete']
# Check actual direct-run emitted identities from the diagnostic custody; future qualified joins
# still require their own actual emissions and direct records.
joins=[]
for p in m['processes']:
 if not p.get('exe'):continue
 producer=read(common/(p['exe']['step']+'.json'));run=read(common/(p['step']+'.json'));target=p['exe']['target']
 # custody schema records target mapping as either a list or dictionary across CLI revisions.
 exes=producer['cargo_executables']
 if isinstance(exes,dict):
  v=exes[target];path=v['path']if isinstance(v,dict)else v
 else:
  matches=[x for x in exes if x.get('target')==target or x.get('name')==target];assert len(matches)==1;path=matches[0].get('path',matches[0].get('executable'))
 assert run['argv'][0]==path
 assert any(x['path']==path and x['sha256']==sha(path)for x in run['inputs'])
 joins.append({'producer':p['exe']['step'],'target':target,'direct_run':p['step'],'executable':artifact(path),'producer_record':artifact(common/(p['exe']['step']+'.json')),'direct_record':artifact(common/(p['step']+'.json'))})
assert len(joins)==5
rec={'schema':'r9-preparation-reconciliation/v1','status':'map_ready preparation only; root reconciliation and qualified execution pending','processes':len(m['processes']),'checks':len(m['checks']),'controls':len(m['controls']),'source_entries':len(source),'external_objects':len(m['external_inputs']),'source_closure_equal':True,'dependency_closure_equal':True,'live_source_mismatches':[],'live_external_mismatches':[],'archive_reconciliation':archives,'bootstrap':artifact(R/'verifier-bootstrap-check-01.json'),'diagnostic_preflight':artifact(R/'diagnostic-07-preparation/preflight-02.json'),'diagnostic_summary':artifact(common/'summary.json'),'diagnostic_postflight':artifact(R/'diagnostic-07-preparation/postflight.json'),'actual_diagnostic_emission_joins':joins,'unfinalized_underlying_distinction':read(R/'bundle-producer-reconciliation.json')['unfinalized_control'],'semantic_reconciliations':{name:artifact(R/name)for name in ['bootstrap-source-reconciliation.json','bundle-producer-reconciliation.json','obligation-source-reconciliation.json','runtime-source-reconciliation.json','fixture-input-reconciliation.json']},'previous_subjects':{name:artifact(N/'implementation-r5'/name)for name in ['obligation-map.json','source-manifest.json','qualifier-manifest.json']},'limits':m['limits']}
write(R/'reconciliation.json',rec)
ids={'obligation_map':sha(R/'obligation-map.json'),'source_manifest':sha(R/'source-manifest.json'),'plan':sha(R/'plan-inv-r9-qualify-01.json'),'qualifier_binary':vb['retained']['sha256'],'reconciliation':sha(R/'reconciliation.json')}
for name,x in freezes.items():ids[name.replace('-','_')+'_freeze_manifest']=x['manifest']['sha256']
qm={'schema':'r9-qualifier-manifest/v1','status':'map_ready preparation only; no root reconciliation/qualification/acceptance','identities':dict(ids),'retained_executable':vb['retained'],'emitted':vb['emitted'],'build_record':artifact(R/'verifier-build-01/build.json'),'build_plan':artifact(R/'verifier-build-plan-01.json'),'bootstrap':artifact(R/'verifier-bootstrap-check-01.json'),'source_freezes':freezes,'reconciliation':artifact(R/'reconciliation.json'),'diagnostics':rec['diagnostic_postflight'],'finalizer_source':artifact(Path(__file__).absolute())}
write(R/'qualifier-manifest.json',qm);ids['qualifier_manifest']=sha(R/'qualifier-manifest.json')
INV='inv-r9-qualify-01';RUN=R/'runs'/INV;RV=R/'root-validation-r9';MAP=R/'obligation-map.json';MAN=R/'source-manifest.json';PLAN=R/'plan-inv-r9-qualify-01.json'
dig=[BIN,str(MAP),str(MAN),str(PLAN),str(R/'qualifier-manifest.json'),str(N/'ROOT-R9-DESIGN-DECISION.json'),str(N/'PLAN-R9.md'),str(N/'architectural-risk-history.json'),str(R/'MAP-APPROVAL.json')]
def gate(mode):return [BIN,mode,'--map',str(MAP),'--plan',str(PLAN),'--manifest',str(MAN),'--expect-map-sha256',ids['obligation_map'],'--expect-plan-sha256',ids['plan'],'--expect-manifest-sha256',ids['source_manifest'],'--invocation-id',INV,'--run-dir',str(RUN/'run'),'--records-dir',str(RUN/'custody'),'--out',str(RV/(mode+'.json'))]
def outer(mode):
 args=['/usr/bin/python3','-I',str(C),'run','--out',str(RV/(mode+'-custody')),'--name',mode,'--cwd',str(R),'--env-clear','--env','PATH=/usr/bin:/bin','--timeout','1800','--kill-leftovers']
 for p in dig:args+=['--digest',p]
 return args+['--']+gate(mode)
lp={'schema':'r9-launch-parameters/v1','status':'prepared only, NOT executed; root must reconcile and approve exact identities','invocation_id':INV,'run_dir':str(RUN/'run'),'records_dir':str(RUN/'custody'),'root_validation_dir':str(RV),'identities':ids,'root_must_first':['Independently reconcile exact source, complete controls/map, archives, prospective input and cold verifier custody, all five future emission/direct-run joins and failure lineage.','Write implementation-r6/preparation-03/MAP-APPROVAL.json only after exact root reconciliation; deliberately absent now.','Confirm inv-r9-qualify-01 and root-validation-r9 absent.','After common execution retain actual Cargo-emitted binaries and all five actual direct-run custody joins.'],'steps':[{'name':'preflight','argv':outer('preflight'),'expect':'exit0; all preflight checks passed'},{'name':'common-plan','argv':['/usr/bin/python3','-I',str(C),'run','--plan',str(PLAN),'--out',str(RUN/'custody')],'expect':'20/20 steps, no leftovers; actual native21/qualifier41 plus all mandatory checks'},{'name':'postflight','argv':outer('postflight'),'expect':'exit0, every approved required obligation passed; no incomplete; cannot establish acceptance alone'}],'limits':m['limits']}
write(R/'launch-parameters.json',lp)
arts={'schema':'r9-preparation-artifacts/v1','status':'map_ready','identity':artifact(R.parent/'IDENTITY.json'),'packet':artifact(R.parent/'BUILDER-PACKET.md'),'adoption':artifact(N/'ROOT-R9-DESIGN-DECISION.json'),'plan_authority':artifact(N/'PLAN-R9.md'),'artifacts':{name:artifact(R/name)for name in ['obligation-map.json','source-manifest.json','plan-inv-r9-qualify-01.json','qualifier-manifest.json','reconciliation.json','launch-parameters.json','verifier-bootstrap-check-01.json','finalize-preparation.py']},'freezes':freezes,'unperformed':['qualified inv-r9-qualify-01','root acceptance','four independent reviews'],'all18findings':'OPEN','limits':m['limits']}
write(R/'ARTIFACTS.json',arts);print(json.dumps({'status':'map_ready','identities':ids,'artifacts_sha256':sha(R/'ARTIFACTS.json')}))
