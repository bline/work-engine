#!/usr/bin/env python3
"""Pi Host controller-mode run (H1-H5): a scripted controller drives
`work-engine-pi-host serve` over the lifecycle protocol
(native-realization PI_HOST_PROTOCOL.md) on the lifecycle relay profile.

Usage: serve.py <root-dir> <run-name> <observer-scenario|live>
Offline scenarios use pi_hosted_messages_observer (scenario `relay`: two
turns, one relayed snapshot_read and one result_publish in turn 1, a text
answer in each turn) with synthetic Host-held access; `live` uses the Claude
subscription route (operator-authorized only). Provider: claude-sdk on the
runtime root built by ../dev-vertical-h2-route-20261117/claude_runtime.py
(PI_HOST_RUNTIME_DIR). PI_HOST_PAUSE_S=<n>: the controller waits n seconds between the turns.
PI_HOST_CONTROLLER_REFUSE=1: the controller answers every relayed call with
an error (the turn must fail and the thread end). Writes <root-dir>/<run-name>/
and prints a summary.
"""
import hashlib, json, os, select, shutil, subprocess, sys, time
from pathlib import Path

REPO = Path(os.environ.get('PI_HOST_REPO', '/home/bline/cbm-work/wt-pi-host-realization-20261118'))
HOST_SRC = REPO / 'rust/crates/work-engine-manual-host'
HOST = Path(os.environ.get('PI_HOST_BIN', REPO / 'rust/target/debug/work-engine-pi-host'))
OBSERVER = REPO / 'rust/target/debug/examples/pi_hosted_messages_observer'
RUNTIME_DIR = Path(os.environ.get('PI_HOST_RUNTIME_DIR', '/home/bline/.local/state/work-engine/dev-vertical-h2-route-20261117/runtime-claude-sdk'))
RUNTIME = RUNTIME_DIR / 'root'
IMAGE = 'node@sha256:4b150b48b4116e2a22f2c39cc27025fbf4f153e2078fd3b14d159fdb5c7279da'
CATALOG = 'node_modules/@earendil-works/pi-ai/dist/providers/data/anthropic.json'
CLAUDE_AUTH = Path.home() / '.claude/.credentials.json'
MODEL = 'claude-haiku-5-5'
MAX_REQUESTS = int(os.environ.get('PI_HOST_MAX_REQUESTS', 8))
MAX_RELAYS = int(os.environ.get('PI_HOST_MAX_RELAYS', 16))
BUDGET = 1_320_001 * MAX_REQUESTS
PROMPTS = ['Read the claim member, then publish the result and answer.', 'Summarize what you published.']

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

def prepare(base, scenario):
    live = scenario == 'live'
    base.mkdir()
    inputs = base / 'inputs'; inputs.mkdir()
    state = base / 'state'; state.mkdir()
    private = base / 'private'; private.mkdir(mode=0o700)
    obs_out = open(base / 'observer.jsonl', 'w')
    if live:
        obs, ready = subprocess.Popen(['sleep', '3600'], stdout=subprocess.PIPE, text=True), None
    else:
        obs = subprocess.Popen([str(OBSERVER)], env={**os.environ, 'HOSTED_OBSERVER_SCENARIO': scenario},
                               stdout=subprocess.PIPE, stderr=open(base / 'observer.stderr', 'w'), text=True)
        ready = json.loads(obs.stdout.readline()); obs_out.write(json.dumps(ready) + '\n'); obs_out.flush()
    # The workspace binds the identity of the snapshot the controller serves;
    # the Host never serves it on this profile.
    workspace = inputs / 'workspace'; workspace.mkdir()
    write(workspace / 'snapshot-identity.json', canon({'snapshot_id': 'snapshot:serve-qual', 'manifest_sha256': '0' * 64}))
    write(inputs / 'workspace-manifest.json', canon(inventory(workspace)))
    config = inputs / 'pi-config'; config.mkdir()
    write(inputs / 'pi-config-manifest.json', canon(inventory(config)))
    task = write(inputs / 'task.txt', 'Pi Host lifecycle relay thread: prompts arrive from the controller.')
    shim = write(inputs / 'pi-coding.mjs', (HOST_SRC / 'receiver/pi-coding.mjs').read_bytes())
    stream_adapter = write(inputs / 'pi-native-tools.mjs', (HOST_SRC / 'receiver/pi-native-tools.mjs').read_bytes())
    shutil.copy(RUNTIME_DIR / 'runtime-manifest.json', inputs / 'runtime-manifest.json')
    if live:
        routes = {'routes': [{'id': 'primary', 'payer': 'claude-subscription', 'endpoint': 'https:anthropic-oauth'}],
                  'max_attempts_per_request': 1}
        write(private / 'hosted-credentials.json', canon({'schema': 'work-engine.pi-hosted-credentials.v1', 'payers': {
            'claude-subscription': {'kind': 'claude-code-auth', 'path': str(CLAUDE_AUTH)}}}), 0o600)
    else:
        routes = {'routes': [
            {'id': 'primary', 'payer': 'synthetic-subscription', 'endpoint': f"loopback:{ready['primary']}"},
            {'id': 'secondary', 'payer': 'synthetic-api', 'endpoint': f"loopback:{ready['secondary']}"}],
            'max_attempts_per_request': 2}
    routes_digest = sha(canon({'schema': 'work-engine.pi-hosted-routes.v1', 'routes': routes['routes'], 'max_attempts_per_request': routes['max_attempts_per_request']}).encode())
    deadline = int(time.time()) + int(os.environ.get('PI_HOST_DEADLINE_S', 900))
    job_id = 'dev-vertical-pi-host-serve-' + base.name
    catalog = RUNTIME / CATALOG
    authority = write(inputs / 'authority.json', canon({
        'schema': 'work-engine.pi-coding-runtime-admission.v1', 'job_id': job_id,
        'binary_sha256': fsha(HOST), 'model': MODEL, 'provider': 'claude-sdk',
        'pi_config_manifest_sha256': fsha(inputs / 'pi-config-manifest.json'),
        'task_sha256': fsha(task), 'catalog_sha256': fsha(catalog), 'max_requests': MAX_REQUESTS,
        'budget_microusd': BUDGET, 'allow_provider': True, 'hosted_routes_sha256': routes_digest}))
    packet = write(inputs / 'packet.json', canon({'schema': 'work-engine.pi-host-serve-packet.v1',
                                                  'max_provider_requests': MAX_REQUESTS, 'max_relays': MAX_RELAYS}))
    job = {
        'schema': 'work-engine.pi-docker-custody.v1', 'job_id': job_id, 'state_dir': str(state),
        'authority': art(authority), 'packet': art(packet), 'docker': art('/usr/bin/docker'), 'image': IMAGE,
        'runtime': {'root': str(RUNTIME), 'manifest': art(inputs / 'runtime-manifest.json')},
        'workspace': {'root': str(workspace), 'manifest': art(inputs / 'workspace-manifest.json')},
        'deadline_unix_seconds': deadline, 'command_timeout_seconds': 30, 'idle_seconds': 0,
        'coding': {
            'schema': 'work-engine.pi-lifecycle-hosted.v1', 'shim': art(shim), 'max_tool_calls': MAX_RELAYS,
            'provider': {
                'lane': {'pi_config': {'root': str(config), 'manifest': art(inputs / 'pi-config-manifest.json')}},
                'task': art(task), 'catalog': art(catalog), 'stream_adapter': art(stream_adapter), 'hosted': routes,
                'provider': 'claude-sdk', 'model': MODEL, 'max_requests': MAX_REQUESTS, 'budget_microusd': BUDGET,
                'max_context_bytes': 400_000}}}
    job_path = write(base / 'job.json', json.dumps(job, indent=1))
    serve_config = write(base / 'serve-config.json', canon({'schema': 'work-engine.pi-host-serve.v1', 'job': art(job_path), 'turn_deadline_secs': 300}))
    return serve_config, obs, obs_out

class Controller:
    def __init__(self, proc, log):
        self.p, self.log, self.buf, self.next = proc, log, b'', 1
        self.frames = []
    def send(self, v):
        self.log.write(json.dumps({'dir': 'out', 'frame': v}) + '\n'); self.log.flush()
        self.p.stdin.write((json.dumps(v) + '\n').encode()); self.p.stdin.flush()
    def request(self, method, params):
        rid = self.next; self.next += 1
        self.send({'jsonrpc': '2.0', 'id': rid, 'method': method, 'params': params})
        return rid
    def recv(self, timeout):
        end = time.time() + timeout
        while b'\n' not in self.buf:
            left = end - time.time()
            if left <= 0:
                raise TimeoutError('no frame from the Host')
            r, _, _ = select.select([self.p.stdout], [], [], left)
            if r:
                chunk = os.read(self.p.stdout.fileno(), 1 << 20)
                if not chunk:
                    raise EOFError('Host stdout closed')
                self.buf += chunk
        line, self.buf = self.buf.split(b'\n', 1)
        v = json.loads(line)
        self.frames.append(v)
        self.log.write(json.dumps({'dir': 'in', 'frame': v}) + '\n'); self.log.flush()
        return v
    def reply_to(self, rid, timeout=600):
        while True:
            v = self.recv(timeout)
            if v.get('id') == rid and 'method' not in v:
                return v

def tool_answer(params):
    tool, args = params['tool'], params['arguments']
    if tool == 'snapshot_read':
        text = json.dumps({'snapshot_id': args.get('snapshot_id'), 'member_id': args.get('member_id'),
                           'encoding': 'utf8', 'bytes': '# Claim\nThe threshold is 200,000 tokens.\n', 'sha256': '1' * 64})
    elif tool == 'result_publish':
        text = json.dumps({'result_id': args.get('result_id'), 'status': 'published'})
    else:
        text = json.dumps({'question_id': args.get('question_id'), 'status': 'open'})
    return {'success': True, 'contentItems': [{'type': 'inputText', 'text': text}]}

def run_turn(c, thread, prompt, answers):
    rid = c.request('turn/start', {'threadId': thread, 'input': [{'type': 'text', 'text': prompt}]})
    started = c.reply_to(rid, 60)
    turn = started.get('result', {}).get('turn', {}).get('id')
    while True:
        v = c.recv(600)
        if v.get('method') == 'item/tool/call':
            answers.append(v['params'])
            if os.environ.get('PI_HOST_CONTROLLER_REFUSE'):
                c.send({'jsonrpc': '2.0', 'id': v['id'], 'error': {'code': -32000, 'message': 'refused by the scripted controller'}})
            else:
                c.send({'jsonrpc': '2.0', 'id': v['id'], 'result': tool_answer(v['params'])})
        elif v.get('method') == 'turn/completed':
            return turn, v['params']['turn']

def main():
    root, name, scenario = Path(sys.argv[1]), sys.argv[2], sys.argv[3]
    root.mkdir(exist_ok=True)
    base = root / name
    serve_config, obs, obs_out = prepare(base, scenario)
    env = {**os.environ}
    if scenario == 'live':
        env['WORK_ENGINE_HOSTED_CREDENTIALS'] = str(base / 'private' / 'hosted-credentials.json')
    else:
        env.pop('WORK_ENGINE_HOSTED_CREDENTIALS', None)
    dynamic = json.loads(subprocess.run([str(HOST), 'dynamic-tools'], capture_output=True, text=True, check=True).stdout)
    started = time.time()
    p = subprocess.Popen([str(HOST), 'serve', '--config', str(serve_config)], stdin=subprocess.PIPE, stdout=subprocess.PIPE,
                         stderr=open(base / 'host.stderr', 'w'), env=env)
    log = open(base / 'protocol.jsonl', 'w')
    c = Controller(p, log)
    out = {'scenario': scenario, 'turns': [], 'reads': [], 'captures': [], 'answers': []}
    try:
        init = c.reply_to(c.request('pi-host/initialize', {}), 30)
        out['initialize'] = init.get('result') or init.get('error')
        ts = c.reply_to(c.request('thread/start', {'model': MODEL, 'dynamicTools': dynamic}), 300)
        thread = ts.get('result', {}).get('thread', {}).get('id')
        out['thread_start'] = ts.get('result') or ts.get('error')
        if thread:
            ob = c.reply_to(c.request('pi-host/observe', {'threadId': thread}), 60)
            out['observation'] = ob.get('result') or ob.get('error')
        for prompt in PROMPTS if thread else []:
            turn, completed = run_turn(c, thread, prompt, out['answers'])
            out['turns'].append(completed)
            read = c.reply_to(c.request('thread/read', {'threadId': thread, 'includeTurns': True}), 30)
            out['reads'].append(read.get('result') or read.get('error'))
            cap = c.reply_to(c.request('pi-host/capture', {'threadId': thread, 'turnId': turn}), 30)
            out['captures'].append(cap.get('result') or cap.get('error'))
            if completed.get('status') != 'completed':
                break
            # PI_HOST_PAUSE_S: idle the thread between turns (H4: a relay
            # thread outlives the 300 s bound of other Host processes).
            time.sleep(int(os.environ.get('PI_HOST_PAUSE_S', 0)) if prompt != PROMPTS[-1] else 0)
    except (TimeoutError, EOFError) as e:
        out['controller_error'] = str(e)
    p.stdin.close()
    try:
        p.wait(timeout=180)
    except subprocess.TimeoutExpired:
        p.kill(); p.wait()
    out['exit'] = p.returncode
    out['elapsed_s'] = round(time.time() - started, 1)
    time.sleep(0.5)
    obs.kill()
    events = []
    for line in (obs.stdout.read() or '').splitlines():
        obs_out.write(line + '\n')
        try: events.append(json.loads(line))
        except ValueError: pass
    obs_out.close()
    log.close()
    out['observer'] = [e for e in events if e.get('type') in ('served', 'refused', 'failed', 'complete')]
    out['host_stderr_tail'] = (base / 'host.stderr').read_text()[-1500:]
    state = base / 'state'
    if (state / 'pi-state.json').exists():
        j = json.loads((state / 'pi-state.json').read_text())
        out['phase'], out['failures'] = j['phase'], j['failures']
        out['ledger'] = {k: {'settled_microusd': r['settled_microusd'], 'attempts': [{x: a.get(x) for x in ('attempt', 'route', 'disposition', 'settled_microusd')} for a in r.get('attempts', [])]} for k, r in j['coding']['provider_requests'].items()}
        containers = subprocess.run(['docker', 'ps', '-a', '--format', '{{.Names}}'], capture_output=True, text=True).stdout.split()
        out['containers_after'] = sorted(n for n in containers if n.startswith(j['container_name']))
    out['capture_documents'] = []
    for cap in out['captures']:
        if isinstance(cap, dict) and cap.get('path'):
            data = Path(cap['path']).read_bytes()
            doc = json.loads(data)
            out['capture_documents'].append({'sha_matches': sha(data) == cap['capture_sha256'], 'eligibility': doc['eligibility'],
                                             'turn': doc['turn'], 'calls': doc['calls'], 'payloads': len(doc['payloads']),
                                             'observation_verified': (doc.get('observation') or {}).get('containment_verified')})
    out['host_sha256'], out['observer_sha256'] = fsha(HOST), fsha(OBSERVER)
    (base / 'summary.json').write_text(json.dumps(out, indent=1))
    print(json.dumps(out, indent=1))

if __name__ == '__main__':
    main()
