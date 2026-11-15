#!/usr/bin/env python3
"""Prospective R9 cold verifier inputs, actual dep-info reconciliation and retained emission.
Finite actor custody; generated OUT_DIR files are derived outputs, never exempt source inputs.
"""
import hashlib,json,os,re,shutil,sys,tarfile,tomllib
from pathlib import Path
R=Path('/home/bline/cbm-work/org-pi-review-continuation-20261114/implementation-r6/preparation-03')
Q=Path('/home/bline/cbm-work/org-pi-review-continuation-20261114/realization-r9/evidence-gate');P=Path('/home/bline/cbm-work/wt-p1-vertical-20261113/rust')
DIAG=Path('/home/bline/cbm-work/org-pi-review-continuation-20261114/implementation-r6/diagnostic-04/qualifier-target');TARGET=R/'verifier-cold-target-01';OUT=R/'verifier-build-01'
TOOL=Path('/home/bline/.rustup/toolchains/1.92.0-x86_64-unknown-linux-gnu');C=Path('/home/bline/code/work-engine/skills/manual-orchestration/scripts/custody.py')
sha=lambda p:hashlib.sha256(Path(p).read_bytes()).hexdigest()
read=lambda p:json.loads(Path(p).read_text())
def write(p,x):
 with Path(p).open('x') as f:json.dump(x,f,indent=1);f.write('\n')
def depinfo(target):
 result=set();relative=set();files=[]
 for f in sorted(target.rglob('*.d')):
  if not f.is_file():continue
  files.append(str(f))
  for line in f.read_text().replace('\\\n',' ').splitlines():
   if line.startswith('#') or ':' not in line:continue
   for token in re.split(r'(?<!\\)\s+',line.split(':',1)[1].strip()):
    if not token:continue
    token=token.replace('\\ ',' ').replace('\\#','#')
    if not os.path.isabs(token):relative.add(token)
    result.add(os.path.normpath(str(Path(token) if os.path.isabs(token) else Q/token)))
 return result,relative,files

def plan():
 assert not TARGET.exists()
 deps,rel,ds=depinfo(DIAG);assert deps
 inputs={p for p in deps if not p.startswith(str(DIAG)+'/')}
 inputs|={str(p) for p in (Q/'src').rglob('*.rs')}
 inputs|={str(Q/n) for n in ['Cargo.toml','Cargo.lock','rust-toolchain.toml']}
 inputs|={str(P/'Cargo.toml'),str(P/'Cargo.lock'),str(P/'rust-toolchain.toml'),str(C),str(Path(__file__).absolute())}
 for p in list(inputs):
  for root,depth in [(P/'crates',1),(Path('/home/bline/.cargo/registry/src'),2)]:
   if p.startswith(str(root)+'/'):
    pkg=root.joinpath(*Path(p).relative_to(root).parts[:depth])
    inputs|={str(pkg/n) for n in ['Cargo.toml','build.rs'] if (pkg/n).is_file()}
    if depth==2:
     archive=Path('/home/bline/.cargo/registry/cache')/pkg.parent.name/(pkg.name+'.crate');inputs.add(str(archive))
 for d in [Q,*Q.parents]:
  inputs|={str(d/n) for n in ['.cargo/config','.cargo/config.toml'] if (d/n).is_file()}
 inputs|={str(p) for p in [Path('/home/bline/.cargo/config'),Path('/home/bline/.cargo/config.toml'),Path('/home/bline/.rustup/settings.toml')] if p.is_file()}
 inputs|={str(Path('/home/bline/.cargo/bin')/n) for n in ['cargo','rustc','rustup']}
 inputs|={str(TOOL/'bin'/n) for n in ['cargo','rustc']}
 inputs|={str(p) for p in (TOOL/'lib').glob('*.so')}
 inputs|={str(p) for p in (TOOL/'lib/rustlib/x86_64-unknown-linux-gnu/lib').glob('*.rlib')}
 assert all(Path(p).is_file() for p in inputs)
 prospective={'inputs':sorted(inputs),'diagnostic_depinfo_files':ds,'relative_diagnostic_tokens':sorted(rel),'meaning':'Candidate discovery only; cold actual dep-info separately reconciled including relative paths. Generated target files are outputs.'}
 write(R/'verifier-prospective-inputs-01.json',prospective)
 inputs.add(str(R/'verifier-prospective-inputs-01.json'))
 step={'name':'build','argv':['/home/bline/.cargo/bin/cargo','build','--offline','--locked','--bin','we-evidence-gate','--message-format=json'],'digest':sorted(inputs),'cargo_json':True,'timeout':1800,'kill_leftovers':True}
 write(R/'verifier-build-plan-01.json',{'cwd':str(Q),'env_clear':True,'env':{'PATH':'/home/bline/.cargo/bin:/usr/bin:/bin','HOME':'/home/bline','CARGO_TARGET_DIR':str(TARGET)},'steps':[step]})
 print('prospective inputs',len(inputs),'relative tokens',len(rel))

def check():
 recpath=OUT/'build.json';rec=read(recpath);assert rec['returncode']==0 and not rec['leftover_processes']
 inputs={x['path']:x for x in rec['inputs']};assert inputs and all(x.get('sha256') for x in inputs.values())
 for p,e in inputs.items():
  q=Path(p);h=hashlib.sha256(os.fsencode(os.readlink(q))).hexdigest() if q.is_symlink() else sha(q)
  assert h==e['sha256'],p
  if q.is_symlink():assert str(q.resolve()) in inputs,p
 emitted,relative,depfiles=depinfo(TARGET);generated=sorted(p for p in emitted if p.startswith(str(TARGET)+'/'))
 uncovered=sorted(emitted-set(inputs)-set(generated));assert not uncovered,uncovered
 stdout=Path(rec['stdout']['path']);assert sha(stdout)==rec['stdout']['sha256']
 messages=[json.loads(x)for x in stdout.read_text().splitlines() if x.startswith('{')]
 artifacts=[x for x in messages if x.get('reason')=='compiler-artifact']
 exes=[x for x in artifacts if x.get('executable')];assert len(exes)==1 and exes[0]['target']['name']=='we-evidence-gate' and exes[0]['fresh'] is False
 bs=[x for x in messages if x.get('reason')=='build-script-executed'];gen=[]
 for name in generated:
  path=Path(name);parents=[x for x in bs if path.is_relative_to(Path(x['out_dir']))];assert len(parents)==1,(name,parents)
  event=parents[0];a=[x for x in artifacts if x['package_id']==event['package_id'] and 'custom-build' in x['target']['kind']];assert len(a)==1
  source=Path(a[0]['target']['src_path']);assert str(source) in inputs and sha(source)==inputs[str(source)]['sha256']
  dest=R/'bootstrap-generated'/path.relative_to(TARGET);dest.parent.mkdir(parents=True,exist_ok=True)
  with path.open('rb') as f,dest.open('xb') as g:shutil.copyfileobj(f,g)
  gen.append({'actual_generated_path':name,'sha256':sha(path),'retained_source':str(dest),'retained_sha256':sha(dest),'build_script_event':event,'compiler_artifact':a[0],'build_script_source_sha256':sha(source),'tool_source_binding':{p:inputs[p]['sha256'] for p in inputs if p in [str(TOOL/'bin/rustc'),str(TOOL/'bin/cargo')]}})
 lock=tomllib.loads((Q/'Cargo.lock').read_text());checks={p['name']+'-'+p['version']:p['checksum']for p in lock['package']if 'checksum'in p};registry=[]
 for p,e in inputs.items():
  if '/.cargo/registry/src/'not in p:continue
  path=Path(p);parts=path.parts;idx=parts.index('src');index,pkg=parts[idx+1:idx+3];arc=Path('/home/bline/.cargo/registry/cache')/index/(pkg+'.crate');member=os.path.normpath('/'.join(parts[idx+2:]))
  assert str(arc) in inputs and sha(arc)==checks[pkg]
  with tarfile.open(arc,'r:gz')as t:
   f=t.extractfile(member);assert f is not None;assert hashlib.sha256(f.read()).hexdigest()==e['sha256'],p
  registry.append({'path':p,'sha256':e['sha256'],'archive':str(arc),'archive_sha256':sha(arc),'member':member})
 src=Path(exes[0]['executable']);digest=sha(src);dest=R/'verifier-bin'/digest/'we-evidence-gate';dest.parent.mkdir(parents=True,exist_ok=False)
 with src.open('rb')as f,dest.open('xb')as g:shutil.copyfileobj(f,g)
 dest.chmod(0o555);st=dest.stat();assert st.st_nlink==1 and (st.st_dev,st.st_ino)!=(src.stat().st_dev,src.stat().st_ino) and sha(dest)==digest
 report={'schema':'r9-verifier-bootstrap/v1','ok':True,'record':str(recpath),'record_sha256':sha(recpath),'inputs':len(inputs),'dep_info_inputs_of_cold_build':len(emitted),'relative_tokens':sorted(relative),'depinfo_files':[{'path':x,'sha256':sha(x)}for x in depfiles],'generated_in_cold_target':generated,'generated_derivations':gen,'registry_archive_correspondence':registry,'uncovered':uncovered,'changed_after':[],'emitted':{'path':str(src),'sha256':digest,'fresh':False,'custody_stdout_sha256':sha(stdout),'compiler_artifact':exes[0]},'retained':{'path':str(dest),'sha256':digest,'mode':oct(st.st_mode),'st_nlink':st.st_nlink,'inode_differs':True},'limitations':'Finite actor-attributed custody and actual dep-info/OUT_DIR derivation; no hermeticity or kernel-load attestation.'}
 write(R/'verifier-bootstrap-check-01.json',report);print('bootstrap OK',len(inputs),len(emitted),len(relative),len(gen),digest)
if __name__=='__main__':{'plan':plan,'check':check}[sys.argv[1]]()
