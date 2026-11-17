#!/usr/bin/env python3
"""Live cutover run: one hosted coding job through the operational entrypoint.

coding-job-prepare (operator preparation) -> work-engine-server (local-live
class) -> work-engine-command submit/read -> the pinned producing Host
(coding-qualification) -> HTTPS routes with Host-held access.

Usage: cutover.py <root-dir> [primary-payer]
primary-payer: chatgpt-subscription (default) or chatgpt-revoked (a real 401
refusal on the primary, then failover to the API route).
Access: Pi's auth store (subscription) and an owner-only API key file, both
named in the Host's credentials file; neither enters the job, the bundle or
the container. Prints a JSON summary without secrets.
"""
import hashlib, json, os, secrets, shutil, socket, subprocess, sys, time, uuid
from pathlib import Path

REPO = Path('/home/bline/code/work-engine')
REL = REPO / 'rust/target/release'
QUAL = REPO / 'rust/crates/pi-pressure-context-host/target/release/coding-qualification'
RUNTIME = Path('/home/bline/.local/state/work-engine/s10-supervision-20261103/pi-docker-base/builder/pi')
TOOLCHAIN = Path('/home/bline/.local/state/work-engine/s10-supervision-20261103/real-job-20261104/readiness/implementation-evidence/offline-rustc-1.92.0')
IMAGE = 'node@sha256:4b150b48b4116e2a22f2c39cc27025fbf4f153e2078fd3b14d159fdb5c7279da'
CATALOG = RUNTIME / 'node_modules/@earendil-works/pi-ai/dist/providers/data/openai-codex.json'
PI_AUTH = Path('/home/bline/.pi/agent/auth.json')
API_KEY_FILE = Path('/home/bline/.local/state/work-engine/credentials/openai-api-key')


def sha(b): return hashlib.sha256(b).hexdigest()
def fsha(p): return sha(Path(p).read_bytes())
def canon(v): return json.dumps(v, separators=(',', ':'), sort_keys=True)
def write(p, data, mode=0o600):
    p = Path(p); p.write_bytes(data if isinstance(data, bytes) else data.encode()); p.chmod(mode); return p
def run(args, **kw):
    r = subprocess.run([str(a) for a in args], capture_output=True, text=True, **kw)
    return r.returncode, r.stdout, r.stderr


def main():
    root = Path(sys.argv[1]).resolve()
    primary_payer = sys.argv[2] if len(sys.argv) > 2 else 'chatgpt-subscription'
    root.mkdir(mode=0o700)
    for d in ['inputs', 'private', 'codex', 'claude', 'events', 'bin']:
        (root / d).mkdir(mode=0o700)
    inputs, private = root / 'inputs', root / 'private'
    # Immutable copy of the exact producing Host.
    host = root / 'bin' / 'coding-qualification'
    shutil.copy(QUAL, host); host.chmod(0o500)
    workspace = inputs / 'workspace'; (workspace / 'src').mkdir(parents=True)
    write(workspace / 'src/lib.rs', '// hosted cutover fixture\n', 0o644)
    task = write(inputs / 'task.txt', 'Add pub fn answer() -> u32 returning 42 to src/lib.rs using one workspace_exec command, then report briefly.', 0o644)
    acceptance = write(inputs / 'acceptance.rs', '#[path="/workspace/src/lib.rs"] mod task; #[test] fn answer(){assert_eq!(task::answer(),42);}', 0o644)
    config = inputs / 'pi-config'; config.mkdir()
    if not API_KEY_FILE.exists():
        API_KEY_FILE.parent.mkdir(mode=0o700, parents=True, exist_ok=True)
        write(API_KEY_FILE, os.environ['OPENAI_API_KEY'] + '\n')
    credentials = write(private / 'hosted-credentials.json', canon({
        'schema': 'work-engine.pi-hosted-credentials.v1', 'payers': {
            'chatgpt-subscription': {'kind': 'chatgpt-pi-auth', 'path': str(PI_AUTH)},
            'chatgpt-revoked': {'kind': 'chatgpt-invalid-for-qualification', 'path': str(PI_AUTH)},
            'openai-api': {'kind': 'api-key-file', 'path': str(API_KEY_FILE)}}}))
    routes = {'routes': [
        {'id': 'primary', 'payer': primary_payer, 'endpoint': 'https:chatgpt-codex'},
        {'id': 'secondary', 'payer': 'openai-api', 'endpoint': 'https:openai-responses'}],
        'max_attempts_per_request': 2}
    routes_sha = sha(canon({'schema': 'work-engine.pi-hosted-routes.v1', **routes}).encode())
    job_id = 'dev-vertical-cutover-' + root.name
    now = time.time()
    deadline = int(now) + 200
    budget = 700_000
    admission = write(inputs / 'runtime-admission.json', canon({
        'schema': 'work-engine.pi-coding-runtime-admission.v1', 'job_id': job_id,
        'binary_sha256': fsha(host), 'model': 'gpt-6-luna', 'task_sha256': fsha(task),
        'catalog_sha256': fsha(CATALOG), 'pi_config_manifest_sha256': sha(b'{}'),
        'max_requests': 4, 'budget_microusd': budget, 'allow_provider': True,
        'hosted_routes_sha256': routes_sha}))
    spec = {
        'schema': 'work-engine.coding-preparation.v1', 'job_id': job_id,
        'output_dir': str(root / 'bundle'), 'state_dir': str(root / 'state'), 'operation_root': str(root / 'ops'),
        'host_binary': str(host), 'task': str(task), 'workspace': str(workspace), 'validator': str(acceptance),
        'runtime': str(RUNTIME), 'toolchain': str(TOOLCHAIN), 'docker': '/usr/bin/docker', 'image': IMAGE,
        'deadline_unix_seconds': deadline, 'command_timeout_seconds': 30, 'collector_deadline_seconds': 600,
        'cancel_grace_seconds': 10, 'fixture_command': '',
        'hosted': {'pi_config': str(config), 'routes': routes, 'credentials': str(credentials),
                   'runtime_admission': {'path': str(admission), 'sha256': fsha(admission)},
                   'budget_control': {'schema': 'work-engine.coding-budget-control.v1', 'authority': 'observation_only',
                                      'classification': 'live_provider_observation',
                                      'source': 'operator-authorized live cutover (no provider quota observed)',
                                      'generation': 'dev-vertical-cutover', 'sequence': 1,
                                      'observed_ms': int(now * 1000), 'max_age_ms': 300000,
                                      'expires_ms': deadline * 1000, 'available': True,
                                      'max_provider_requests': 4, 'max_tool_calls': 1}}}
    spec_path = write(inputs / 'spec.json', json.dumps(spec))
    out = {'job_id': job_id, 'primary_payer': primary_payer, 'host_sha256': fsha(host)}
    rc, so, se = run([REL / 'coding-job-prepare', 'inspect', spec_path])
    if rc: out['inspect'] = se.strip(); return out
    inspected = json.loads(so)
    credential = secrets.token_hex(32)
    auth_path = write(private / 'operator-authority.json', canon({
        'schema': 'work-engine.coding-preparation-authority.v1', 'spec_sha256': inspected['spec_sha256'],
        'inputs_sha256': inspected['inputs_sha256'], 'job_id': job_id, 'authorization_id': 'cutover-1',
        'caller': 'operator', 'credential_sha256': sha(credential.encode()), 'allow_prepare': True,
        'allow_provider': False}))
    rc, so, se = run([REL / 'coding-job-prepare', 'prepare', spec_path, auth_path])
    if rc: out['prepare'] = se.strip(); return out
    prepared = json.loads(so)
    out['prepared'] = {k: prepared[k] for k in ('host_input_digest', 'packet_sha256')}
    # Server: local-live class with the generated job profile.
    owner = root / 'owner.py'
    shutil.copy(REPO / 'skills/session-events/scripts/session_events.py', owner)
    write(root / 'query.json', canon({'version': 1, 'owner': str(owner), 'owner_sha256': fsha(owner),
                                       'codex_home': str(root / 'codex'), 'claude_home': str(root / 'claude'),
                                       'state_dir': str(root / 'events'), 'bridge': None, 'context': None}))
    bearer = write(root / 'bearer', secrets.token_hex(32))
    s = socket.socket(); s.bind(('127.0.0.1', 0)); port = s.getsockname()[1]; s.close()
    server_cfg = write(root / 'server.json', canon({
        'version': 1, 'listen': f'127.0.0.1:{port}', 'mcp': False, 'query_profile': str(root / 'query.json'),
        'credential_file': str(bearer), 'coding_job_profile': prepared['job_profile']['path'],
        'coding_job_capability': 'coding-job-local-live-v1', 'max_connections': 4, 'max_queries': 2,
        'request_timeout_seconds': 30}))
    server = subprocess.Popen([str(REL / 'work-engine-server'), str(server_cfg)], stdout=open(root / 'server.stdout', 'w'),
                              stderr=open(root / 'server.stderr', 'w'), stdin=subprocess.DEVNULL)
    try:
        for _ in range(200):
            if server.poll() is not None:
                out['server'] = (root / 'server.stderr').read_text()[-500:]; return out
            try:
                socket.create_connection(('127.0.0.1', port), timeout=0.2).close(); break
            except OSError:
                time.sleep(0.05)
        cli = write(root / 'cli-profile.json', canon({'version': 1, 'kind': 'local-service', 'endpoint': f'127.0.0.1:{port}',
                                                       'credential_file': str(bearer), 'timeout_seconds': 30}))
        request_id = str(uuid.uuid4())
        job = prepared['job']
        submit = write(private / 'submit.json', canon({'request_id': request_id, 'authorization_id': 'cutover-1',
                                                       'job_credential': credential, 'job_sha256': job['sha256'],
                                                       'packet_sha256': prepared['packet_sha256']}))
        read = write(private / 'read.json', canon({'request_id': request_id, 'authorization_id': 'cutover-1',
                                                   'job_credential': credential}))
        rc, so, se = run([REL / 'work-engine-command', cli, 'submit_coding_job', '--json', submit])
        out['submit'] = {'exit': rc, 'operation_id': (json.loads(so).get('envelope') or {}).get('owner_result', {}).get('operation_id') if so.strip() else None,
                         'stderr': se.strip()[-300:]}
        state = None
        started = time.time()
        while time.time() - started < 620:
            rc, so, se = run([REL / 'work-engine-command', cli, 'read_coding_job_operation', '--json', read])
            try:
                state = json.loads(so)['envelope']['owner_result']['state']
            except Exception:
                state = f'unreadable: {se.strip()[-200:]}'
            if state in ('collected', 'entry_uncertain'):
                break
            time.sleep(3)
        out['operation_state'] = state
        out['elapsed_s'] = round(time.time() - started, 1)
        rc, so, se = run([REL / 'work-engine-command', cli, 'read_coding_job_result', '--json', read])
        (root / 'result.json').write_text(so)
        try:
            result = json.loads(so)['envelope']['owner_result']
            out['result'] = {k: result.get(k) for k in ('acceptance', 'terminal', 'validation', 'cleanup', 'receipt_id')}
        except Exception:
            out['result'] = {'exit': rc, 'stderr': se.strip()[-300:]}
        # Ledger from the Host journal.
        st = json.loads((root / 'state' / 'pi-state.json').read_text())
        out['ledger'] = {k: {'settled_microusd': r['settled_microusd'], 'attempts': [
            (a['route'], a['payer'], a['disposition'], a.get('http_status'), a.get('framed_end_observed'), a.get('reason'))
            for a in r['attempts']]} for k, r in st['coding']['provider_requests'].items()}
        out['phase'] = st['phase']
        out['tool_effects'] = len(st['coding']['tool_requests'])
        out['completed'] = st['coding'].get('completed')
    finally:
        server.terminate()
        try:
            server.wait(timeout=120)
        except subprocess.TimeoutExpired:
            server.kill()
        out['server_exit'] = server.returncode
    return out


if __name__ == '__main__':
    print(json.dumps(main(), indent=1))
