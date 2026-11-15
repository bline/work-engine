#!/usr/bin/env python3
"""R9 prospective common inventory. Preparation only, never acceptance or qualified execution.
Old maps are material; all current source inputs, producer inventory and runtime declarations
are reconciled for this new subject. Frozen descendants retain finite source-bound standing.
"""
import collections,hashlib,json,os,re,shutil,stat,subprocess,sys
from pathlib import Path
N=Path('/home/bline/cbm-work/org-pi-review-continuation-20261114');R=N/'implementation-r6/preparation-03';OLD=N/'implementation-r5';Q=N/'realization-r9/evidence-gate';P=Path('/home/bline/cbm-work/wt-p1-vertical-20261113');D=P/'rust/crates/work-engine-review-delivery'
C=Path('/home/bline/code/work-engine/skills/manual-orchestration/scripts/custody.py')
read=lambda p:json.loads(Path(p).read_text());sha=lambda p:hashlib.sha256(Path(p).read_bytes()).hexdigest()
def write(p,x):
 with Path(p).open('x')as f:json.dump(x,f,indent=1);f.write('\n')
def artifact(p):return {'path':str(p),'sha256':sha(p)}
def observe(p):
 p=Path(p)
 if p.is_symlink():return {'kind':'symlink','sha256':hashlib.sha256(os.fsencode(os.readlink(p))).hexdigest(),'link_target':os.readlink(p)}
 assert p.is_file(),p
 return {'kind':'file','sha256':sha(p)}
def source(p,patterns=()):
 lines=Path(p).read_text().splitlines()
 return {'path':str(p),'sha256':sha(p),'binding':'Whole exact file enters source manifest; selected lines locate semantics only.','selected_lines':[{'line':i+1,'text':x}for i,x in enumerate(lines)if any(y in x for y in patterns)]}
def call(argv):subprocess.run(argv,check=True)
def fixture():
 dest=R/'fixture-inputs/wrong-writer-authority.json';dest.parent.mkdir(exist_ok=True)
 original=P/'rust/crates/work-engine-organization/tests/fixtures/frozen/episode-authority.json';x=read(original);x['writer']['actorId']='a-different-writer';data=json.dumps(x,sort_keys=True,separators=(',',':'),ensure_ascii=False).encode()
 actual=N/'implementation-r6/diagnostic-04/run/controls/wrong-writer/authority.json';assert data==actual.read_bytes()
 with dest.open('xb')as f:f.write(data)
 return {'input':artifact(dest),'baseline':artifact(original),'derivation':'Parse original, change only writer.actorId to a-different-writer, compact lexical-key JSON matching actual Authority.value capture.','actual_diagnostic_capture':artifact(actual)}
def prepare():
 vb=read(R/'verifier-bootstrap-check-01.json');assert vb['ok'];BIN=vb['retained']['path'];assert sha(BIN)==vb['retained']['sha256']
 oldbin=read(OLD/'qualifier-manifest.json')['retained_executable']['path'];m=read(OLD/'obligation-map.json')
 text=json.dumps(m).replace(oldbin,BIN).replace(str(N/'realization-r8/evidence-gate'),str(Q)).replace(str(OLD),str(R)).replace('inv-r8-qualify-01','inv-r9-qualify-01')
 m=json.loads(text);m['map_id']='r9-capture-lifecycle-complete-observed-bundle-v1';m['authority']='PLAN-R9.md '+sha(N/'PLAN-R9.md')+'; ROOT-R9-DESIGN-DECISION.json '+sha(N/'ROOT-R9-DESIGN-DECISION.json')+' authorizes bounded repair. History revision34; all18findings open. Root map reconciliation and qualified execution pending.'
 # Remove obsolete preparation paths. Retain exact current source closure and support.
 m['required_files']=[p for p in m['required_files']if not p.startswith(str(R)+'/')]
 m['required_files'] += [str(R/x)for x in ['prepare-r9.py','verifier_bootstrap.py','verifier-prospective-inputs-01.json','bootstrap-source-reconciliation.json','bundle-producer-reconciliation.json','obligation-source-reconciliation.json','runtime-source-reconciliation.json','fixture-input-reconciliation.json','shell-default-observation/shell-default-path.stdout']]
 m['required_files'] += [g['retained_source'] for g in vb['generated_derivations']]
 for p in (Q/'src').glob('*.rs'):
  if str(p)not in m['required_files']:m['required_files'].append(str(p))
 f=fixture();write(R/'fixture-input-reconciliation.json',f)
 m['required_files'].append(f['input']['path'])
 bootstrap=read(R/'verifier-build-01/build.json');pins={p:observe(p) for p in m['external_inputs'] if Path(p).is_file() and not p.startswith(str(R)+'/')}
 for e in bootstrap['inputs']:
  p=e['path']
  if p.startswith(str(Q)+'/')or p.startswith(str(P)+'/')or p.startswith(str(R)+'/'):
   if p not in m['required_files']:m['required_files'].append(p)
  else:pins[p]=observe(p)
 pins[BIN]=observe(BIN)
 # Retain all current external objects from declarations, resolving each link/endpoint separately.
 for p in m['processes']:
  if p['step']=='test-native':p['min_tests']=21
  if p['step']=='qualifier-test':p['min_tests']=41
  if p['step'] in ['qualifier-build','build','build-default','clippy-default','clippy-fixture','doctest-fixture','doctest-default','metadata']and 'qualifier_registry'not in p['inputs']:p['inputs'].append('qualifier_registry')
  for r in p.get('runtime_inputs',[]):
   for name in dict.fromkeys([r['lookup'],*r['links'],r['endpoint']]):pins[name]=observe(name)
 # Qualifier formatting/warnings are explicit members of the same common plan.
 qb=next(p for p in m['processes']if p['step']=='qualifier-build');idx=m['processes'].index(qb)
 for offset,(step,args)in enumerate([('qualifier-fmt',['fmt','--check']),('qualifier-clippy',['clippy','--offline','--locked','--all-targets','--','-D','warnings'])]):
  p=json.loads(json.dumps(qb));p['step']=step;p['argv']=['/home/bline/.cargo/bin/cargo',*args];p.pop('cargo_json');p.pop('exe_targets');p['inputs']=[x for x in p['inputs']if x!='runtime_qualifier_build']+['runtime_'+step.replace('-','_')]
  for x in p['runtime_inputs']:x['id']=step+'-top';x['argv']=p['argv']
  m['classes']['runtime_'+step.replace('-','_')]=m['classes']['runtime_qualifier_build'][:];m['processes'].insert(idx+offset,p)
 m['external_inputs']=pins;m['classes']['gate_binary']=[BIN]
 m['classes']['qualifier_registry']=sorted(p for p in pins if '/.cargo/'in p or '/.rustup/'in p)
 # The full producer inventory is source-derived; diagnostic output independently reconciles it.
 delivery=['missing-key','ordinary-stream','die-during-prompt','wrong-session','duplicate-start','duplicate-start-per-frame','duplicate-start-chunked','exit-nonzero','hang','early-eof','die-after-allocation','frame-limit','total-limit']
 delivery += [f'trail-{label}-{delay}'for label in ['duplicate','malformed','error','invalid-utf8','incomplete']for delay in ['prompt','delayed']]
 delivery += [f'wire-{obj}-{conflict}-{last}'for obj in ['Allocation','Observation','Context','History']for conflict in ['false','true']for last in ['false','true']]
 consumer=['stale-revision','wrong-writer','wrong-subject-result','missing-evidence','not-established']
 stages={name:'delivery_refused'for name in delivery};stages.update({name:'consumer_refused'for name in consumer});stages.update({'assertion-unwind':'delivered_and_consumed','write-failure':'delivered_and_consumed','delivery-refusal':'delivery_refused','setup-refusal':'native_error','consumer-refusal':'consumer_refused','wrong-delivered':'delivered_and_consumed','wrong-delivery-refusal':'delivery_refused','wrong-setup-refusal':'native_error','wrong-consumer-refusal':'consumer_refused'})
 actual=N/'implementation-r6/diagnostic-04/run/controls';assert set(stages)=={p.name for p in actual.iterdir()} and len(stages)==53
 controls=[];producer=[]
 for name,stage in sorted(stages.items()):
  d=actual/name;manifest=read(d/'output-manifest.json');capture=read(d/'capture.json');assert capture['stage']==stage
  expected='failed'if name.startswith('wrong-')and name in ['wrong-delivered','wrong-delivery-refusal','wrong-setup-refusal','wrong-consumer-refusal']or name=='assertion-unwind'else 'incomplete'if name=='write-failure'else 'passed'
  assert manifest['outcome']==expected
  scratch='control-'+name if name in ['assertion-unwind','write-failure','delivery-refusal','setup-refusal','consumer-refusal']else name
  c={'name':name,'scratch':scratch,'stage':stage,'outcome':expected,'preserved':sorted(e['name']for e in manifest['files'])}
  if name=='write-failure':c['write_failure']='episode-after.json'
  if name=='wrong-writer':c['authority_source']=f['input']['path']
  controls.append(c);producer.append({'declaration':c,'diagnostic_manifest':artifact(d/'output-manifest.json'),'diagnostic_capture':artifact(d/'capture.json')})
 m['controls']=controls
 limits='Finite actor-attributed prelaunch observations and source-bound descendant declarations. No atomic resolution, kernel-load, hostile-writer, hermeticity, confinement, production-provider, principal-independence or acceptance claim. Five future build/direct-run joins require actual Cargo emission and matching custody executable bytes. Each declared bundle appraises all physically supplied entries and applicable roles; whole run-root or unmapped sibling absence is not claimed. 1 positive and53 complete control bundles declared; unfinalized-manifest-control has explicit incomplete/unestablished underlying standing and only its source-bound preservation test verdict is mapped. Diagnostic success cannot discharge qualified execution or root/reviewer judgment; all18findings remain open.'
 m['limits']=limits
 write(R/'bootstrap-source-reconciliation.json',{'schema':'r9-bootstrap-source-reconciliation/v1','bootstrap':artifact(R/'verifier-bootstrap-check-01.json'),'prospective_inputs':vb['inputs'],'relative_tokens':vb['relative_tokens'],'uncovered':vb['uncovered'],'generated_derivations':vb['generated_derivations'],'registry_archive_correspondence':vb['registry_archive_correspondence'],'limitations':vb['limitations']})
 native=D/'tests/native_vertical.rs';gate=Q/'tests/gate.rs'
 write(R/'bundle-producer-reconciliation.json',{'schema':'r9-bundle-producer-reconciliation/v1','producer':source(native,['run_case(', 'refused_attempt(', 'DeliveryCapture::invoke','"wrong-', '"trail-', '"wire-', 'controls.join', 'join("controls")','unfinalized-manifest-control']),'declared_controls':producer,'source_inventory_derivation':{'delivery_named':delivery,'consumer_named':consumer,'adjacent_and_later_failure':{k:v for k,v in stages.items()if k not in delivery+consumer}},'unfinalized_control':{'path':'unfinalized-manifest-control','underlying_standing':'incomplete/unestablished, no successful output manifest, excluded from complete-bundle controls','test':'test-native:wrong_control_verdicts_and_manifest_failure_retain_returned_facts','proposition':'Expected manifest failure retains available raw/state/scratch and cannot authorize cleanup. Passing test is not successful underlying bundle.','source_binding':source(native,['manifest failure','unfinalized-manifest-control','manifest-write-failure','failed.is_err()'])},'limits':limits})
 nt=re.findall(r'#\[test\]\s*fn\s+(\w+)',native.read_text());gt=[x for x in re.findall(r'#\[test\]\s*fn\s+(\w+)',gate.read_text())if x!='t'];assert len(nt)==21 and len(gt)==41
 inherited=read(OLD/'successor-1/obligation-source-reconciliation.json')['obligation_families'];inherited['R9 capture ownership/failure/manifest distinction']=['test-native:wrong_control_verdicts_and_manifest_failure_retain_returned_facts','test-native:post_delivery_failures_keep_native_evidence_states_and_scratch'];inherited['R9 observed-entry and meaningful failure support']=['qualifier-test:r9_write_failure_cannot_hide_surviving_entries','qualifier-test:r9_failure_support_requires_meaning_and_exact_retained_scratch','qualifier-test:r9_scratch_identity_excludes_siblings_and_requires_one_real_directory']
 write(R/'obligation-source-reconciliation.json',{'schema':'r9-obligation-source-reconciliation/v1','native_tests':nt,'qualifier_tests':gt,'source_bindings':[source(native),source(gate),source(D/'tests/pure.rs'),source(Q/'src/outputs.rs'),source(Q/'src/spec.rs')],'obligation_families':inherited,'all_native_tests_execution':'test-native','all_qualifier_tests_execution':'qualifier-test','unfinalized_underlying_distinction':'See bundle-producer-reconciliation.json; no finalized bundle is claimed.','standing':limits})
 declarations=[]
 for p in m['processes']:
  for r in p.get('runtime_inputs',[]):
   declarations.append({'process':p['step'],'runtime_input':r,'source_bindings':[source(r['source'],['Command::new','cmd.args','cmd.arg(','.args([','.env_clear()', '.current_dir(','.env(', 'for p in r.links','let status = cmd','sleep ', 'readlink ', 'fn node()', 'split_paths','d.join("node")'])]if r['source']else [],'standing':'source-bound descendant expression, not observed concrete descendant argv'if r['source']else 'prospective fixed top-level expectation; actual custody pending'})
 write(R/'runtime-source-reconciliation.json',{'schema':'r9-runtime-source-reconciliation/v1','declarations':declarations,'native_node_selection':source(native,['fn node()', 'split_paths','d.join("node")','node: node()', 'adapter:', 'sdk_root:', 'fixture_root:']),'shell_PATH_observation':artifact(R/'shell-default-observation/shell-default-path.json'),'five_future_emissions':[{'producer':p['exe']['step'],'target':p['exe']['target'],'direct_run':p['step']}for p in m['processes']if p.get('exe')],'limits':limits})
 assert len([p for p in m['processes']if p.get('exe')])==5
 m['required_files']=list(dict.fromkeys(m['required_files']));assert all(Path(p).is_file()for p in m['required_files'])
 m['checks']=[];empty=R/'map-input-checks-empty.json';write(empty,m)
 call([BIN,'derive-checks','--map',str(empty),'--out',str(R/'derived-checks.json')]);dc=read(R/'derived-checks.json');m['checks']=dc['checks']if isinstance(dc,dict)else dc
 write(R/'obligation-map.json',m);call([BIN,'manifest','--map',str(R/'obligation-map.json'),'--out',str(R/'source-manifest.json')])
 call([BIN,'make-plan','--map',str(R/'obligation-map.json'),'--invocation-id','inv-r9-qualify-01','--run-dir',str(R/'runs/inv-r9-qualify-01/run'),'--records-dir',str(R/'runs/inv-r9-qualify-01/custody'),'--expect-map-sha256',sha(R/'obligation-map.json'),'--expect-manifest-sha256',sha(R/'source-manifest.json'),'--out',str(R/'plan-inv-r9-qualify-01.json')])
 print(json.dumps({'processes':len(m['processes']),'controls':len(m['controls']),'checks':len(m['checks']),'sources':len(read(R/'source-manifest.json')['entries']),'external_objects':len(pins),'standing':'prepared only; diagnostics, freezes and launch reconciliation pending'}))
if __name__=='__main__':prepare()
