#!/usr/bin/env python3
"""Hosted analysis qualification run (H2 Pi-route slice 3).

Usage: analyze.py <root-dir> <run-name> <observer-scenario|live> [snapshot-dir] [task-file]
Offline scenarios use the synthetic loopback observer and synthetic Host-held
access; `live` uses the real HTTPS routes with access named in a credentials
file (operator-authorized). The inference container never has network,
credentials or the snapshot; the Host serves the snapshot.
Writes <root-dir>/<run-name>/ and prints a JSON summary (also summary.json).

ANALYSIS_PROVIDER=claude-sdk (W4): Pi's claude-sdk provider (Claude Code in
the container, Messages family) on the runtime root built by
claude_runtime.py (ANALYSIS_RUNTIME_DIR, holding root/ and
runtime-manifest.json); offline scenarios use pi_hosted_messages_observer,
`live` the Claude subscription route only (Host-held access from Claude
Code's credentials file).
"""
import hashlib, json, os, shutil, subprocess, sys, time
from pathlib import Path

REPO = Path('/home/bline/code/work-engine')
HOST_SRC = REPO / 'rust/crates/work-engine-manual-host'
QUAL = Path(os.environ.get('ANALYSIS_QUAL', REPO / 'rust/crates/pi-pressure-context-host/target/release/analysis-qualification'))
OBSERVER = REPO / 'rust/target/debug/examples/pi_hosted_analysis_observer'
RUNTIME = Path('/home/bline/.local/state/work-engine/s10-supervision-20261103/pi-docker-base/builder/pi')
PRIOR = Path('/home/bline/.local/state/work-engine/repeatable-development-sol-20261106T0320/builder/actual/coding-job-Qld9NF/prepared-bundle')
IMAGE = 'node@sha256:4b150b48b4116e2a22f2c39cc27025fbf4f153e2078fd3b14d159fdb5c7279da'
CATALOG = 'node_modules/@earendil-works/pi-ai/dist/providers/data/openai-codex.json'
PI_AUTH = Path('/home/bline/.pi/agent/auth.json')
MAX_REQUESTS = int(os.environ.get('ANALYSIS_MAX_REQUESTS', 8))
MAX_READS = int(os.environ.get('ANALYSIS_MAX_READS', 16))
BUDGET, CONTEXT = 5_000_000, 400_000
PROVIDER = os.environ.get('ANALYSIS_PROVIDER', 'openai-codex')
CLAUDE = PROVIDER == 'claude-sdk'
MODEL = 'claude-haiku-5-5' if CLAUDE else 'gpt-6-luna'
if CLAUDE:
    RUNTIME_DIR = Path(os.environ['ANALYSIS_RUNTIME_DIR'])
    RUNTIME = RUNTIME_DIR / 'root'
    CATALOG = 'node_modules/@earendil-works/pi-ai/dist/providers/data/anthropic.json'
    OBSERVER = REPO / 'rust/target/debug/examples/pi_hosted_messages_observer'
    # Full catalog reservation of the 1M-context model: 945 001 per request.
    BUDGET = 945_001 * MAX_REQUESTS
CLAUDE_AUTH = Path.home() / '.claude/.credentials.json'

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

def prepare(base, scenario, snapshot_src, task_src):
    live = scenario == 'live'
    base.mkdir()
    inputs = base / 'inputs'; inputs.mkdir()
    state = base / 'state'; state.mkdir()
    private = base / 'private'; private.mkdir(mode=0o700)
    obs_out = open(base / 'observer.jsonl', 'w')
    if live:
        obs = subprocess.Popen(['sleep', '900'], stdout=subprocess.PIPE, text=True)
        ready = None
    else:
        obs = subprocess.Popen([str(OBSERVER)], env={**os.environ, 'HOSTED_OBSERVER_SCENARIO': scenario, 'HOSTED_OBSERVER_MAX_REQUESTS': str(MAX_REQUESTS)},
                               stdout=subprocess.PIPE, stderr=open(base / 'observer.stderr', 'w'), text=True)
        ready = json.loads(obs.stdout.readline()); obs_out.write(json.dumps(ready) + '\n'); obs_out.flush()
    snapshot = inputs / 'snapshot'
    if snapshot_src:
        shutil.copytree(snapshot_src, snapshot)
    else:
        (snapshot / 'docs').mkdir(parents=True)
        write(snapshot / 'docs/claim.md', '# Claim\n\nThe threshold is 200,000 tokens, measured in run A.\n')
        write(snapshot / 'docs/evidence.md', '# Evidence\n\nRun A compacted at 198,000 tokens. Run B was not measured.\n')
    for p in snapshot.rglob('*'):
        if p.is_file(): p.chmod(0o644)
    write(inputs / 'snapshot-manifest.json', canon(inventory(snapshot)))
    config = inputs / 'pi-config'; config.mkdir()
    write(inputs / 'pi-config-manifest.json', canon(inventory(config)))
    task = write(inputs / 'task.txt', Path(task_src).read_bytes() if task_src else
                 'Audit whether the claim in docs/claim.md is supported by docs/evidence.md. Report supported, unsupported and unknown parts with exact citations.')
    shim = write(inputs / 'pi-coding.mjs', (HOST_SRC / 'receiver/pi-coding.mjs').read_bytes())
    stream_adapter = write(inputs / 'pi-native-tools.mjs', (HOST_SRC / 'receiver/pi-native-tools.mjs').read_bytes())
    shutil.copy((RUNTIME_DIR if CLAUDE else PRIOR) / 'runtime-manifest.json', inputs / 'runtime-manifest.json')
    if live and CLAUDE:
        # Subscription only: no API-key failover route.
        routes = {'routes': [
            {'id': 'primary', 'payer': 'claude-subscription', 'endpoint': 'https:anthropic-oauth'}],
            'max_attempts_per_request': 1}
        write(private / 'hosted-credentials.json', canon({'schema': 'work-engine.pi-hosted-credentials.v1', 'payers': {
            'claude-subscription': {'kind': 'claude-code-auth', 'path': str(CLAUDE_AUTH)}}}), 0o600)
    elif live:
        routes = {'routes': [
            {'id': 'primary', 'payer': 'chatgpt-subscription', 'endpoint': 'https:chatgpt-codex'},
            {'id': 'secondary', 'payer': 'openai-api', 'endpoint': 'https:openai-responses'}],
            'max_attempts_per_request': 2}
        write(private / 'hosted-credentials.json', canon({'schema': 'work-engine.pi-hosted-credentials.v1', 'payers': {
            'chatgpt-subscription': {'kind': 'chatgpt-pi-auth', 'path': str(PI_AUTH)},
            'openai-api': {'kind': 'api-key-env', 'variable': 'OPENAI_API_KEY'}}}), 0o600)
    else:
        routes = {'routes': [
            {'id': 'primary', 'payer': 'synthetic-subscription', 'endpoint': f"loopback:{ready['primary']}"},
            {'id': 'secondary', 'payer': 'synthetic-api', 'endpoint': f"loopback:{ready['secondary']}"}],
            'max_attempts_per_request': 2}
    routes_digest = sha(canon({'schema': 'work-engine.pi-hosted-routes.v1', 'routes': routes['routes'], 'max_attempts_per_request': routes['max_attempts_per_request']}).encode())
    now = int(time.time())
    deadline = now + int(os.environ.get('ANALYSIS_DEADLINE_S', 290))
    job_id = 'dev-vertical-hosted-analysis-' + base.name
    catalog = RUNTIME / CATALOG
    authority = write(inputs / 'authority.json', canon({
        'schema': 'work-engine.pi-coding-runtime-admission.v1', 'job_id': job_id,
        'binary_sha256': fsha(QUAL), 'model': MODEL, 'provider': PROVIDER,
        'pi_config_manifest_sha256': fsha(inputs / 'pi-config-manifest.json'),
        'task_sha256': fsha(task), 'catalog_sha256': fsha(catalog), 'max_requests': MAX_REQUESTS,
        'budget_microusd': BUDGET, 'allow_provider': True, 'hosted_routes_sha256': routes_digest}))
    packet = write(inputs / 'packet.json', canon({'schema': 'work-engine.pi-analysis-packet.v1',
                                                  'max_provider_requests': MAX_REQUESTS, 'max_snapshot_reads': MAX_READS}))
    job = {
        'schema': 'work-engine.pi-docker-custody.v1', 'job_id': job_id, 'state_dir': str(state),
        'authority': art(authority), 'packet': art(packet), 'docker': art('/usr/bin/docker'), 'image': IMAGE,
        'runtime': {'root': str(RUNTIME), 'manifest': art(inputs / 'runtime-manifest.json')},
        'workspace': {'root': str(snapshot), 'manifest': art(inputs / 'snapshot-manifest.json')},
        'deadline_unix_seconds': deadline, 'command_timeout_seconds': 30, 'idle_seconds': 0,
        'coding': {
            'schema': 'work-engine.pi-analysis-hosted.v1', 'shim': art(shim), 'max_tool_calls': MAX_READS,
            'provider': {
                'lane': {'pi_config': {'root': str(config), 'manifest': art(inputs / 'pi-config-manifest.json')}},
                'task': art(task), 'catalog': art(catalog), 'stream_adapter': art(stream_adapter), 'hosted': routes,
                'provider': PROVIDER, 'model': MODEL, 'max_requests': MAX_REQUESTS, 'budget_microusd': BUDGET,
                'max_context_bytes': CONTEXT}}}
    job_path = write(base / 'job.json', json.dumps(job, indent=1))
    return job_path, obs, obs_out

def main():
    root, name, scenario = Path(sys.argv[1]), sys.argv[2], sys.argv[3]
    snapshot_src = sys.argv[4] if len(sys.argv) > 4 and sys.argv[4] != '-' else None
    task_src = sys.argv[5] if len(sys.argv) > 5 else None
    root.mkdir(exist_ok=True)
    base = root / name
    job_path, obs, obs_out = prepare(base, scenario, snapshot_src, task_src)
    env = {**os.environ}
    if scenario == 'live':
        env['WORK_ENGINE_HOSTED_CREDENTIALS'] = str(base / 'private' / 'hosted-credentials.json')
    else:
        env.pop('WORK_ENGINE_HOSTED_CREDENTIALS', None)
    started = time.time()
    q = subprocess.run([str(QUAL), '--execute', str(job_path)], capture_output=True, text=True, env=env, timeout=600)
    elapsed = round(time.time() - started, 1)
    time.sleep(0.5)
    obs.kill()
    events = []
    for line in (obs.stdout.read() or '').splitlines():
        obs_out.write(line + '\n')
        try: events.append(json.loads(line))
        except ValueError: pass
    obs_out.close()
    state = base / 'state'
    j = json.loads((state / 'pi-state.json').read_text())
    c = j['coding']
    result = state / 'analysis-result.json'
    containers = subprocess.run(['docker', 'ps', '-a', '--format', '{{.Names}}'], capture_output=True, text=True).stdout.split()
    summary = {
        'scenario': scenario, 'exit': q.returncode, 'elapsed_s': elapsed,
        'stdout': q.stdout.strip()[-400:], 'stderr': q.stderr.strip()[-600:],
        'phase': j['phase'], 'failures': j['failures'], 'completed': c.get('completed'),
        'ledger': {k: {'settled_microusd': r['settled_microusd'], 'attempts': r.get('attempts', [])} for k, r in c['provider_requests'].items()},
        'snapshot_requests': c.get('snapshot_requests', {}), 'snapshot_tools': c.get('snapshot_tools', {}),
        'native_snapshot_ids': c.get('native_snapshot_ids', []), 'sdk_refused_ids': c.get('sdk_refused_ids', []),
        'observer': [e for e in events if e.get('type') in ('served', 'refused', 'failed', 'complete')],
        'result': json.loads(result.read_text()) if result.exists() else None,
        'containers_after': sorted(n for n in containers if n.startswith(j['container_name'])),
        'qualification_sha256': fsha(QUAL), 'observer_sha256': fsha(OBSERVER),
    }
    (base / 'summary.json').write_text(json.dumps(summary, indent=1))
    print(json.dumps(summary, indent=1))

if __name__ == '__main__':
    main()
