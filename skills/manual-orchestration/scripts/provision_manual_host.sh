#!/usr/bin/env bash
# Build and pin the bounded manual host and a compatible adapter bundle.
# Usage: provision_manual_host.sh [REPOSITORY]
set -euo pipefail
repo="${1:-$(git rev-parse --show-toplevel)}"
state="${WORK_ENGINE_STATE:-$HOME/.local/state/work-engine}"
target="${MANUAL_HOST_TARGET:-$state/tools/manual-host/target}"
mkdir -p "$state"
run="$(mktemp -d "$state/manual-host-build.XXXXXXXX")"
plan="$run/plan.json"
python3 - "$repo" "$target" "$plan" <<'PY'
import json,sys
repo,target,plan=sys.argv[1:]
json.dump({'cwd':repo,'env':{'CARGO_TARGET_DIR':target},'steps':[{'name':'build','argv':['cargo','build','--locked','--release','--manifest-path',repo+'/rust/Cargo.toml','-p','work-engine-manual-host'],'timeout':600,'kill_leftovers':True}]},open(plan,'w'))
PY
python3 -I "$repo/skills/manual-orchestration/scripts/custody.py" run --out "$run/custody" --plan "$plan" >&2
python3 - "$repo" "$target/release/work-engine-manual-host" "$state" "$run" <<'PY'
import hashlib,json,os,pathlib,shutil,subprocess,sys,tempfile
repo=pathlib.Path(sys.argv[1]).resolve();binary=pathlib.Path(sys.argv[2]);state=pathlib.Path(sys.argv[3]);run=sys.argv[4]
paths=['skills/manual-orchestration/scripts/role_host.py','skills/manual-orchestration/scripts/codex_app.py',
 'skills/manual-orchestration/scripts/scope_budget.py','skills/manual-orchestration/scripts/child_context.py',
 'skills/durable-state/scripts/codex_chatboard.py','metrics/scripts/usage_ledger.py']
# Import closure: task profiling and board use sibling/root script libraries.
# Preserve layouts and pin every copied script; no mutable repo imports follow.
files={p.relative_to(repo).as_posix():p.read_bytes() for folder in
 ['skills/manual-orchestration/scripts','skills/durable-state/scripts','metrics/scripts']
 for p in (repo/folder).rglob('*.py') if '__pycache__' not in p.parts}
files['work-engine-manual-host']=binary.read_bytes()
files['schemas.json']=subprocess.check_output([str(binary),'schema'])
files['provision_manual_host.sh']=(repo/'skills/manual-orchestration/scripts/provision_manual_host.sh').read_bytes()
manifest={p:hashlib.sha256(b).hexdigest() for p,b in sorted(files.items())}
bundle=hashlib.sha256(json.dumps(manifest,sort_keys=True,separators=(',',':')).encode()).hexdigest()
parent=state/'tools/manual-host';parent.mkdir(parents=True,exist_ok=True);pin=parent/bundle
if not pin.exists():
 stage=pathlib.Path(tempfile.mkdtemp(prefix='.pin-',dir=parent))
 try:
  for name,b in files.items():
   dest=stage/name;dest.parent.mkdir(parents=True,exist_ok=True);dest.write_bytes(b);dest.chmod(0o555 if name=='work-engine-manual-host' else 0o444)
  (stage/'manifest.json').write_text(json.dumps(manifest,indent=2)+'\n')
  (stage/'provenance.json').write_text(json.dumps({'bundle_schema':'work-engine.manual-host.bundle.v1','source_commit':subprocess.check_output(['git','-C',str(repo),'rev-parse','HEAD'],text=True).strip(),'source_status':subprocess.check_output(['git','-C',str(repo),'status','--porcelain'],text=True),'build_custody':run+'/custody','schema':'work-engine.manual-host.v1'},indent=2)+'\n')
  try:stage.rename(pin)
  except FileExistsError:shutil.rmtree(stage)
 except BaseException:
  shutil.rmtree(stage,ignore_errors=True);raise
for name,sha in manifest.items():
 if hashlib.sha256((pin/name).read_bytes()).hexdigest()!=sha:raise SystemExit('existing bundle conflict: '+name)
print(json.dumps({'binary':str(pin/'work-engine-manual-host'),'binary_sha256':manifest['work-engine-manual-host'],'bundle':str(pin),'bundle_sha256':bundle,'manifest':str(pin/'manifest.json'),'build_custody':run+'/custody'},sort_keys=True))
PY
