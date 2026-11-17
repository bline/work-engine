#!/usr/bin/env python3
"""Offline hosted-profile qualification scenario (dev-vertical phase 4 piece (c)).

Offline scenarios: synthetic loopback routes and synthetic Host-held access.
Scenarios named live* use the real HTTPS routes with Host-held access named in
a credentials file (operator-authorized live runs). The inference container
never has network or credentials.
Usage: qualify.py <root-dir> <run-name> <observer-scenario> [none|crash|cancel]
Writes <root-dir>/<run-name>/ and prints a JSON summary (also summary.json).
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


SCENARIO = 'ok'
PI_AUTH = Path('/home/bline/.pi/agent/auth.json')
LIVE = False  # set for scenarios named live*

def prepare(base):

    base.mkdir()
    inputs = base / 'inputs'; inputs.mkdir()
    state = base / 'state'; state.mkdir()
    private = base / 'private'; private.mkdir(mode=0o700)

    obs_out = open(base / 'observer.jsonl', 'w')
    if LIVE:
        # Live routes: no observer. The Host's credentials file names each
        # payer's access source (nonsecret); access stays in the Host process.
        obs = subprocess.Popen(['sleep', '900'], stdout=subprocess.PIPE, text=True)
        ready = None
    else:
        obs = subprocess.Popen([str(OBSERVER)], env={**os.environ, 'HOSTED_OBSERVER_SCENARIO': SCENARIO}, stdout=subprocess.PIPE, stderr=open(base / 'observer.stderr', 'w'), text=True)
        ready = json.loads(obs.stdout.readline()); obs_out.write(json.dumps(ready) + '\n'); obs_out.flush()

    workspace = inputs / 'workspace'; (workspace / 'src').mkdir(parents=True)
    write(workspace / 'src/lib.rs', '// hosted offline fixture\n')
    write(inputs / 'workspace-manifest.json', canon(inventory(workspace)))
    config = inputs / 'pi-config'; config.mkdir()
    write(inputs / 'pi-config-manifest.json', canon(inventory(config)))
    task = write(inputs / 'task.txt', 'Add pub fn answer() -> u32 returning 42 to src/lib.rs using one workspace_exec command, then report briefly.')
    acceptance = write(inputs / 'acceptance.rs', '#[path="/workspace/src/lib.rs"] mod task; #[test] fn answer(){assert_eq!(task::answer(),42);}')
    shim = write(inputs / 'pi-coding.mjs', (HOST_SRC / 'receiver/pi-coding.mjs').read_bytes())
    stream_adapter = write(inputs / 'pi-native-tools.mjs', (HOST_SRC / 'receiver/pi-native-tools.mjs').read_bytes())
    shutil.copy(PRIOR / 'runtime-manifest.json', inputs / 'runtime-manifest.json')
    shutil.copy(PRIOR / 'toolchain-manifest.json', inputs / 'toolchain-manifest.json')

    if LIVE:
        primary_payer = 'chatgpt-revoked' if SCENARIO == 'live-revoked-primary' else 'chatgpt-subscription'
        routes = {'routes': [
            {'id': 'primary', 'payer': primary_payer, 'endpoint': 'https:chatgpt-codex'},
            {'id': 'secondary', 'payer': 'openai-api', 'endpoint': 'https:openai-responses'}],
            'max_attempts_per_request': 2}
        write(private / 'hosted-credentials.json', canon({'schema': 'work-engine.pi-hosted-credentials.v1', 'payers': {
            'chatgpt-subscription': {'kind': 'chatgpt-pi-auth', 'path': str(PI_AUTH)},
            'chatgpt-revoked': {'kind': 'chatgpt-invalid-for-qualification', 'path': str(PI_AUTH)},
            'openai-api': {'kind': 'api-key-env', 'variable': 'OPENAI_API_KEY'}}}), 0o600)
    else:
        routes = {'routes': [
            {'id': 'primary', 'payer': 'synthetic-subscription', 'endpoint': f"loopback:{ready['primary']}"},
            {'id': 'secondary', 'payer': 'synthetic-api', 'endpoint': f"loopback:{ready['secondary']}"}],
            'max_attempts_per_request': 2}
    routes_digest = sha(canon({'schema': 'work-engine.pi-hosted-routes.v1', 'routes': routes['routes'], 'max_attempts_per_request': 2}).encode())

    now = int(time.time())
    deadline = now + 290
    job_id = 'dev-vertical-hosted-qual-' + base.name
    catalog = RUNTIME / CATALOG
    budget = 1_000_000
    authority = write(inputs / 'authority.json', canon({
        'schema': 'work-engine.pi-coding-runtime-admission.v1', 'job_id': job_id,
        'binary_sha256': fsha(QUAL), 'model': 'gpt-6-luna',
        'pi_config_manifest_sha256': fsha(inputs / 'pi-config-manifest.json'),
        'task_sha256': fsha(task), 'catalog_sha256': fsha(catalog), 'max_requests': 4,
        'budget_microusd': budget, 'allow_provider': True,
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
                'lane': {'pi_config': {'root': str(config), 'manifest': art(inputs / 'pi-config-manifest.json')}},
                'task': art(task), 'catalog': art(catalog),
                'stream_adapter': art(stream_adapter), 'hosted': routes, 'model': 'gpt-6-luna',
                'max_requests': 4, 'budget_microusd': budget, 'max_context_bytes': 24000}}}
    job_path = write(base / 'job.json', json.dumps(job, indent=1))

    return base, job_path, obs, obs_out

def ledger_of(j):
    """Ledger facts as the Host recorded them. `charged_microusd_derived` is
    this harness's own application of the ledger rule to the dispositions; the
    Host does not export its charge."""
    ledger = {}
    for k, r in j['coding']['provider_requests'].items():
        att = r.get('attempts', [])
        charged = 0
        for a in att:
            d = a['disposition']
            charged += 0 if d in ('refused', 'not_dispatched') else (a['settled_microusd'] if d == 'completed' and a['settled_microusd'] is not None else a['reserved_microusd'])
        ledger[k] = {'settled_microusd': r['settled_microusd'], 'charged_microusd_derived': charged if att else (r['settled_microusd'] if r['settled_microusd'] is not None else r['reserved_microusd']),
                     'attempts': [{x: a.get(x) for x in ('attempt', 'route', 'payer', 'disposition', 'http_status', 'gate_entered', 'deadline_ms', 'framed_end_observed', 'reason', 'relay_sha256', 'refusal_body_sha256', 'settled_microusd')} for a in att]}
    return ledger

def docker_names(prefix):
    out = subprocess.run(['docker', 'ps', '-a', '--format', '{{.Names}}'], capture_output=True, text=True).stdout.split()
    return sorted(n for n in out if n.startswith(prefix))

def main():
    global SCENARIO, LIVE
    root, name, SCENARIO = Path(sys.argv[1]), sys.argv[2], sys.argv[3]
    LIVE = SCENARIO.startswith('live')
    action = sys.argv[4] if len(sys.argv) > 4 else 'none'
    root.mkdir(exist_ok=True)
    base, job_path, obs, obs_out = prepare(root / name)
    started = time.time()
    env = {**os.environ}
    if LIVE:
        env['WORK_ENGINE_HOSTED_CREDENTIALS'] = str(base / 'private' / 'hosted-credentials.json')
    else:
        env.pop('WORK_ENGINE_HOSTED_CREDENTIALS', None)
    q = subprocess.Popen([str(QUAL), '--execute', str(job_path)], stdout=open(base / 'qualification.stdout', 'w'),
                         stderr=open(base / 'qualification.stderr', 'w'), start_new_session=False, env=env)
    events, acted = [], None
    fd = obs.stdout.fileno()
    os.set_blocking(fd, False)
    def drain():
        try:
            return os.read(fd, 65536).decode()
        except BlockingIOError:
            return ''
    buf = ''
    while q.poll() is None and time.time() - started < 600:
        buf += drain()
        while '\n' in buf:
            line, buf = buf.split('\n', 1)
            obs_out.write(line + '\n'); obs_out.flush()
            try: ev = json.loads(line)
            except ValueError: continue
            events.append(ev)
            if action != 'none' and acted is None and ev.get('type') == 'fault' and ev.get('phase') == 'holding':
                time.sleep(1.0)  # some relayed bytes reach the container first
                acted = {'action': action, 'at_s': round(time.time() - started, 1)}
                if action == 'crash':
                    q.kill()  # SIGKILL: no cleanup, no gate-log write
                else:
                    acted['cancel'] = subprocess.run([str(QUAL), 'pi-docker', 'cancel', '--state', str(base / 'state')],
                                                     capture_output=True, text=True).stdout.strip()[:400]
        time.sleep(0.05)
    if q.poll() is None:
        q.kill()
    q.wait()
    elapsed = round(time.time() - started, 1)
    time.sleep(1)
    def collect():
        nonlocal_buf = buf + drain()
        lines = nonlocal_buf.split('\n')
        for line in lines[:-1]:
            obs_out.write(line + '\n'); obs_out.flush()
            try: events.append(json.loads(line))
            except ValueError: pass
        return lines[-1]
    buf = collect()
    dispatches = lambda: [e for e in events if e.get('type') in ('served', 'refused', 'failed')]
    dispatched_at_exit = len(dispatches())
    state = base / 'state'
    j = json.loads((state / 'pi-state.json').read_text())
    prefix = j['container_name']
    before = docker_names(prefix)
    recon = None
    ledger_after_reconcile = None
    if action == 'crash':
        # The observer stays up through reconcile and the replay attempt, so
        # any dispatch they made would be recorded as a receipt.
        rc = subprocess.run([str(QUAL), 'pi-docker', 'reconcile', '--state', str(state)], capture_output=True, text=True)
        recon = {'exit': rc.returncode, 'stdout_tail': rc.stdout.strip()[-300:], 'stderr_tail': rc.stderr.strip()[-300:]}
        ledger_after_reconcile = ledger_of(json.loads((state / 'pi-state.json').read_text()))
        replay = subprocess.run([str(QUAL), '--execute', str(job_path)], capture_output=True, text=True)
        recon['replay_exit'] = replay.returncode
        recon['replay_stderr'] = replay.stderr.strip()[-200:]
        time.sleep(2)
        buf = collect()
        recon['observer_alive_after_replay'] = obs.poll() is None
        j = json.loads((state / 'pi-state.json').read_text())
    recon_dispatches = len(dispatches()) - dispatched_at_exit
    obs.kill(); obs.wait()
    if buf:
        obs_out.write(buf + '\n')
    obs_out.close()
    after = docker_names(prefix)
    c = j['coding']
    ledger = ledger_of(j)
    arts = [a for a in j['artifacts'] if '/coding-hosted-' in a['path']]
    def fsha(p):
        try: return hashlib.sha256(Path(p).read_bytes()).hexdigest()
        except OSError: return None
    gate_log = state / 'coding-hosted-gate-attempts.json'
    summary = {
        'scenario': SCENARIO, 'action': action, 'acted': acted, 'elapsed_s': elapsed, 'exit': q.returncode,
        'stdout': (base / 'qualification.stdout').read_text().strip()[-300:], 'stderr': (base / 'qualification.stderr').read_text().strip()[-300:],
        'phase': j['phase'], 'failures': j['failures'], 'provider_failed': c.get('provider_failed'), 'completed': c.get('completed'),
        'ledger': ledger,
        'hosted_artifacts': [{'name': os.path.basename(a['path']), 'verified': fsha(a['path']) == a['sha256']} for a in arts],
        'gate_log': json.loads(gate_log.read_text()) if gate_log.exists() else None,
        'observer': [e for e in events if e.get('type') in ('served', 'refused', 'failed', 'fault', 'complete')],
        'containers_before_reconcile': before, 'containers_after': after, 'reconcile': recon,
        'ledger_after_reconcile': ledger_after_reconcile, 'dispatches_after_exit': recon_dispatches,
        'coding_qualification_sha256': fsha(QUAL), 'observer_sha256': fsha(OBSERVER),
    }
    (base / 'summary.json').write_text(json.dumps(summary, indent=1))
    print(json.dumps(summary, indent=1))

if __name__ == '__main__':
    main()
