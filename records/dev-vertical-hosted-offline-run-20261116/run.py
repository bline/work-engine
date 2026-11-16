#!/usr/bin/env python3
"""Offline hosted-profile Docker run (dev-vertical phase 4 piece (b) step 4).

Synthetic loopback routes only; synthetic Host-held access; no live provider,
credentials or network for the inference container. Usage: run.py <run-dir-name>
"""
import hashlib, json, os, shutil, subprocess, sys, time
from pathlib import Path

REPO = Path('/home/bline/code/work-engine')
HOST_SRC = REPO / 'rust/crates/work-engine-manual-host'
QUAL = REPO / 'rust/crates/pi-pressure-context-host/target/release/coding-qualification'
OBSERVER = REPO / 'rust/target/debug/examples/pi_hosted_coding_observer'
RUNTIME = Path('/home/bline/.local/state/work-engine/s10-supervision-20261103/pi-docker-base/builder/pi')
TOOLCHAIN = Path('/home/bline/.local/state/work-engine/s10-supervision-20261103/real-job-20261104/readiness/implementation-evidence/offline-rustc-1.92.0')
PRIOR = Path('/home/bline/.local/state/work-engine/repeatable-development-sol-20261106T0320/builder/actual/coding-job-Qld9NF/prepared-bundle')
IMAGE = 'node@sha256:4b150b48b4116e2a22f2c39cc27025fbf4f153e2078fd3b14d159fdb5c7279da'
CATALOG = 'node_modules/@earendil-works/pi-ai/dist/providers/data/openai-codex.json'

def sha(b): return hashlib.sha256(b).hexdigest()
def fsha(p): return sha(Path(p).read_bytes())
def art(p): return {'path': str(p), 'sha256': fsha(p)}
def canon(v): return json.dumps(v, separators=(',', ':'), sort_keys=True)
def inventory(root):
    out = {}
    for dp, dns, fns in os.walk(root):
        for f in fns:
            p = Path(dp) / f
            out[str(p.relative_to(root))] = {'kind': 'file', 'sha256': fsha(p)}
    return dict(sorted(out.items()))
def write(p, data, mode=0o644):
    p = Path(p); p.write_bytes(data if isinstance(data, bytes) else data.encode()); p.chmod(mode); return p

base = Path(__file__).resolve().parent / sys.argv[1]
base.mkdir()
inputs = base / 'inputs'; inputs.mkdir()
state = base / 'state'; state.mkdir()
private = base / 'private'; private.mkdir(mode=0o700)

obs_out = open(base / 'observer.jsonl', 'w')
obs = subprocess.Popen([str(OBSERVER)], stdout=subprocess.PIPE, stderr=open(base / 'observer.stderr', 'w'), text=True)
ready = json.loads(obs.stdout.readline()); obs_out.write(json.dumps(ready) + '\n'); obs_out.flush()

workspace = inputs / 'workspace'; (workspace / 'src').mkdir(parents=True)
write(workspace / 'src/lib.rs', '// hosted offline fixture\n')
write(inputs / 'workspace-manifest.json', canon(inventory(workspace)))
config = inputs / 'pi-config'; config.mkdir()
write(inputs / 'pi-config-manifest.json', canon(inventory(config)))
native = write(private / 'native-never-read.json', b'{"never":"read"}', 0o600)
task = write(inputs / 'task.txt', 'Add pub fn answer() -> u32 returning 42 to src/lib.rs using one workspace_exec command, then report briefly.')
acceptance = write(inputs / 'acceptance.rs', '#[path="/workspace/src/lib.rs"] mod task; #[test] fn answer(){assert_eq!(task::answer(),42);}')
shim = write(inputs / 'pi-coding.mjs', (HOST_SRC / 'receiver/pi-coding.mjs').read_bytes())
access_adapter = write(inputs / 'pi-native-access.mjs', (HOST_SRC / 'receiver/pi-native-access.mjs').read_bytes())
stream_adapter = write(inputs / 'pi-native-tools.mjs', (HOST_SRC / 'receiver/pi-native-tools.mjs').read_bytes())
shutil.copy(PRIOR / 'runtime-manifest.json', inputs / 'runtime-manifest.json')
shutil.copy(PRIOR / 'toolchain-manifest.json', inputs / 'toolchain-manifest.json')

routes = {'routes': [
    {'id': 'primary', 'payer': 'synthetic-subscription', 'endpoint': f"loopback:{ready['primary']}"},
    {'id': 'secondary', 'payer': 'synthetic-api', 'endpoint': f"loopback:{ready['secondary']}"}],
    'max_attempts_per_request': 2}
routes_digest = sha(canon({'schema': 'work-engine.pi-hosted-routes.v1', 'routes': routes['routes'], 'max_attempts_per_request': 2}).encode())

now = int(time.time())
deadline = now + 290
job_id = 'dev-vertical-hosted-step4-' + sys.argv[1]
catalog = RUNTIME / CATALOG
budget = 1_000_000
authority = write(inputs / 'authority.json', canon({
    'schema': 'work-engine.pi-coding-runtime-admission.v1', 'job_id': job_id,
    'binary_sha256': fsha(QUAL), 'model': 'gpt-6-luna', 'native_auth_source': str(native),
    'pi_config_manifest_sha256': fsha(inputs / 'pi-config-manifest.json'),
    'task_sha256': fsha(task), 'catalog_sha256': fsha(catalog), 'max_requests': 4,
    'budget_microusd': budget, 'allow_native_access_projection': True, 'allow_provider': True,
    'hosted_routes_sha256': routes_digest}))
bounds = {'inference_containers': 1, 'tool_containers': 1, 'validator_containers': 1, 'sdk_sessions': 2,
          'workspace_exec_calls': 1, 'fake_provider_requests': 0, 'physical_resets': 1, 'live_provider_requests': 4}
packet = write(inputs / 'packet.json', canon({
    'schema': 'work-engine.coding-live-continuity-packet.v1', 'bounds': bounds, 'pressure_basis_points': 9500,
    'budget': {'schema': 'work-engine.coding-budget-control.v1', 'classification': 'live_provider_observation',
               'available': True, 'max_provider_requests': 4, 'max_tool_calls': 1, 'observed_ms': now * 1000,
               'max_age_ms': 300000, 'authority': 'observation_only', 'sequence': 1, 'expires_ms': deadline * 1000,
               'source': 'offline synthetic hosted qualification (no provider quota observed)',
               'generation': 'dev-vertical-hosted-step4'}}))
job = {
    'schema': 'work-engine.pi-docker-custody.v1', 'job_id': job_id, 'state_dir': str(state),
    'authority': art(authority), 'packet': art(packet), 'docker': art('/usr/bin/docker'), 'image': IMAGE,
    'runtime': {'root': str(RUNTIME), 'manifest': art(inputs / 'runtime-manifest.json')},
    'workspace': {'root': str(workspace), 'manifest': art(inputs / 'workspace-manifest.json')},
    'deadline_unix_seconds': deadline, 'command_timeout_seconds': 30, 'idle_seconds': 0,
    'coding': {
        'schema': 'work-engine.pi-coding-hosted.v1', 'shim': art(shim),
        'toolchain': {'root': str(TOOLCHAIN), 'manifest': art(inputs / 'toolchain-manifest.json')},
        'acceptance': art(acceptance), 'max_tool_calls': 1, 'workspace_write_boundary': 'src_lib_rs',
        'provider': {
            'lane': {'pi_config': {'root': str(config), 'manifest': art(inputs / 'pi-config-manifest.json')},
                     'native_auth_source': str(native)},
            'task': art(task), 'catalog': art(catalog), 'access_adapter': art(access_adapter),
            'stream_adapter': art(stream_adapter), 'hosted': routes, 'model': 'gpt-6-luna',
            'max_requests': 4, 'budget_microusd': budget, 'max_context_bytes': 24000}}}
job_path = write(base / 'job.json', json.dumps(job, indent=1))

started = time.time()
q = subprocess.run([str(QUAL), '--execute', str(job_path)], capture_output=True, text=True, timeout=600)
(base / 'qualification.stdout').write_text(q.stdout); (base / 'qualification.stderr').write_text(q.stderr)
try:
    rest, _ = obs.communicate(timeout=15)
except subprocess.TimeoutExpired:
    obs.kill(); rest, _ = obs.communicate()
obs_out.write(rest); obs_out.close()
print(json.dumps({'exit': q.returncode, 'seconds': round(time.time() - started, 1), 'stdout': q.stdout[-2000:], 'stderr': q.stderr[-3000:], 'observer_exit': obs.returncode}, indent=1))
