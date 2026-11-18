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

Scenario `treatment` (R3, offline): one turn over the H2 treatment snapshot
(the 13 members of the S2 r2 snapshot manifest at their pinned commits, the
ledger as its line range, plus the A4 material built from PI_HOST_A4_FIXTURE),
read in 32 KiB pages: the observer (`steps`) asks for 12 reads in its first
response, the rest in its second, then result_publish, then the answer; the
controller answers each read with the member bytes in the actor's answer
shape. Use with the H2 job parameters (PI_HOST_MAX_REQUESTS=32,
PI_HOST_MAX_RELAYS=32, PI_HOST_DEADLINE_S=1800, PI_HOST_TURN_DEADLINE_S=900).
The summary adds every request's contextBytes and forwarderRefused (from the
container's frames), the retained body total, the thread/read frame size and
the inference container's cgroup memory.peak, sampled while it runs.

PI_HOST_S2_MANIFEST selects another snapshot manifest (default S2 r2). A
manifest with an `a4_request_card_member` (S2 r3) takes that member's bytes
from PI_HOST_A4_MEMBER, checked against its pinned SHA-256, instead of
building the A4 material from PI_HOST_A4_FIXTURE. Scenario `advisory`: the
A4 member alone, read the same way. PI_HOST_MAX_CONTEXT sets the job's
max_context_bytes (default 400,000). PI_HOST_PAD_BYTES=<n> adds a treatment
member `zz/padding.md` of n bytes of plain ASCII text (no JSON escapes), to
measure a context near the bound. PI_HOST_OBSERVER overrides the observer
binary.
"""
import hashlib, json, os, select, shutil, subprocess, sys, threading, time
from pathlib import Path

REPO = Path(os.environ.get('PI_HOST_REPO', Path(__file__).resolve().parents[2]))
HOST_SRC = REPO / 'rust/crates/work-engine-manual-host'
HOST = Path(os.environ.get('PI_HOST_BIN', REPO / 'rust/target/debug/work-engine-pi-host'))
OBSERVER = Path(os.environ.get('PI_HOST_OBSERVER', REPO / 'rust/target/debug/examples/pi_hosted_messages_observer'))
RUNTIME_DIR = Path(os.environ.get('PI_HOST_RUNTIME_DIR', '/home/bline/.local/state/work-engine/dev-vertical-h2-route-20261117/runtime-claude-sdk'))
RUNTIME = RUNTIME_DIR / 'root'
IMAGE = 'node@sha256:4b150b48b4116e2a22f2c39cc27025fbf4f153e2078fd3b14d159fdb5c7279da'
CATALOG = 'node_modules/@earendil-works/pi-ai/dist/providers/data/anthropic.json'
CLAUDE_AUTH = Path.home() / '.claude/.credentials.json'
MODEL = 'claude-haiku-5-5'
MAX_REQUESTS = int(os.environ.get('PI_HOST_MAX_REQUESTS', 8))
MAX_RELAYS = int(os.environ.get('PI_HOST_MAX_RELAYS', 16))
TURN_DEADLINE_S = int(os.environ.get('PI_HOST_TURN_DEADLINE_S', 300))
BUDGET = 1_320_001 * MAX_REQUESTS
PROMPTS = ['Read the claim member, then publish the result and answer.', 'Summarize what you published.']
TREATMENT_PROMPTS = ['Read every snapshot member, then publish the result and answer.']
# S2 r2 treatment arm (snapshot-manifest.json): (member id, commit, path, sha256).
S2 = os.environ.get('PI_HOST_S2_MANIFEST', '/home/bline/cbm-work/research-lifecycle-tool-registry-20261116/016-s2-r2/snapshot-manifest.json')
TREATMENT_SNAPSHOT = 'snapshot:h2-treatment-r2'
LEDGER = 'metrics/efficiency-ledger.md'
LEDGER_LINES = [50] + list(range(295, 375)) + [431]
LEDGER_RANGE_SHA256 = 'e3b452a86361701901fe9d7e97d5ecd156fb2b25b7d31d1993c52296e8389f6a'
PAGE = 32768

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

def git_blob(commit, path):
    return subprocess.run(['git', '-C', str(REPO), 'show', f'{commit}:{path}'], capture_output=True, check=True).stdout

def a4_member(m):
    """The A4 member: (id, bytes), from the manifest's file member (r3) or the fixture (r2)."""
    if 'a4_request_card_member' in m:
        e = m['a4_request_card_member']
        data = Path(os.environ['PI_HOST_A4_MEMBER']).read_bytes()
        assert sha(data) == e['sha256'] and len(data) == int(e['bytes']), 'A4 member pin'
        return e['proposed_member_id'], data
    a4 = Path(os.environ['PI_HOST_A4_FIXTURE'])
    return 'research/a4-material', b'\n'.join((a4 / f).read_bytes() for f in ('request.json', 'card.json', 'reconstruction.json'))

def advisory_members():
    """The advisory snapshot: the A4 member alone."""
    mid, data = a4_member(json.loads(Path(S2).read_text())['treatment_arm_members'])
    return {mid: data}

def treatment_members():
    """The treatment snapshot: member id -> bytes, each checked against its pin."""
    m = json.loads(Path(S2).read_text())['treatment_arm_members']
    entries = m['ordinary_policy_members'] + m['research_bundle_members'] + [m['decision_revision_member']]
    members = {}
    for e in entries:
        data = git_blob(e['commit'], e['path'])
        assert sha(data) == e['sha256'], e['path']
        if e['path'] == LEDGER:
            lines = data.split(b'\n')
            data = b''.join(lines[n - 1] + b'\n' for n in LEDGER_LINES)
            assert sha(data) == LEDGER_RANGE_SHA256
        members[e['proposed_member_id']] = data
    mid, data = a4_member(m)
    members[mid] = data
    pad = int(os.environ.get('PI_HOST_PAD_BYTES', 0))
    if pad:
        line = b'Padding for the context bound measurement; plain text without escapes. '
        members['zz/padding.md'] = (line * (pad // len(line) + 1))[:pad]
    return dict(sorted(members.items()))

def treatment_steps(members):
    """The observer's responses: 12 reads, the remaining reads, result_publish, the answer."""
    reads = [{'tool': 'mcp__pi__snapshot_read', 'input': {'snapshot_id': TREATMENT_SNAPSHOT, 'member_id': mid,
              'offset': off, 'length': min(PAGE, len(data) - off)}}
             for mid, data in members.items() for off in range(0, len(data), PAGE)]
    publish = {'tool': 'mcp__pi__result_publish', 'input': {'result_id': 'result-1',
               'body': 'The treatment members were read in full.',
               'source_refs': [{'id': 'src-1', 'owner_namespace': 'task', 'owner_ref': 'owner-1', 'revision': 1, 'sha256': '1' * 64}],
               'attribution': 'native:claude-haiku-5-5'}}
    first, rest = reads[:12], reads[12:]
    return [{'tools': first}] + ([{'tools': rest}] if rest else []) + [publish, {'text': 'Published result-1 over the snapshot members.'}]

def prepare(base, scenario, members=None):
    live = scenario == 'live'
    base.mkdir()
    inputs = base / 'inputs'; inputs.mkdir()
    state = base / 'state'; state.mkdir()
    private = base / 'private'; private.mkdir(mode=0o700)
    obs_out = open(base / 'observer.jsonl', 'w')
    if live:
        obs, ready = subprocess.Popen(['sleep', '3600'], stdout=subprocess.PIPE, text=True), None
    else:
        obs_env = {**os.environ, 'HOSTED_OBSERVER_SCENARIO': scenario}
        if scenario in ('treatment', 'advisory'):
            steps = write(base / 'observer-steps.json', json.dumps(treatment_steps(members)))
            obs_env.update(HOSTED_OBSERVER_SCENARIO='steps', HOSTED_OBSERVER_STEPS=str(steps))
        obs = subprocess.Popen([str(OBSERVER)], env=obs_env,
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
                'max_context_bytes': int(os.environ.get('PI_HOST_MAX_CONTEXT', 400_000))}}}
    job_path = write(base / 'job.json', json.dumps(job, indent=1))
    serve_config = write(base / 'serve-config.json', canon({'schema': 'work-engine.pi-host-serve.v1', 'job': art(job_path), 'turn_deadline_secs': TURN_DEADLINE_S}))
    return serve_config, obs, obs_out

class Controller:
    def __init__(self, proc, log):
        self.p, self.log, self.buf, self.next = proc, log, b'', 1
        self.frames = []
        self.read_sizes = []  # raw bytes of each thread/read reply
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
        if isinstance(v.get('result'), dict) and 'turns' in (v['result'].get('thread') or {}):
            self.read_sizes.append(len(line))
        self.log.write(json.dumps({'dir': 'in', 'frame': v}) + '\n'); self.log.flush()
        return v
    def reply_to(self, rid, timeout=600):
        while True:
            v = self.recv(timeout)
            if v.get('id') == rid and 'method' not in v:
                return v

def tool_answer(params, members=None):
    tool, args = params['tool'], params['arguments']
    if tool == 'snapshot_read' and members is not None:
        # The actor's answer shape (canonical JSON of the read page).
        off, n = args['offset'], args['length']
        page = members[args['member_id']][off:off + n]
        text = canon({'encoding': 'utf8', 'length': len(page), 'member_id': args['member_id'], 'offset': off,
                      'sha256': sha(page), 'snapshot_id': args['snapshot_id'], 'text': page.decode()})
    elif tool == 'snapshot_read':
        text = json.dumps({'snapshot_id': args.get('snapshot_id'), 'member_id': args.get('member_id'),
                           'encoding': 'utf8', 'bytes': '# Claim\nThe threshold is 200,000 tokens.\n', 'sha256': '1' * 64})
    elif tool == 'result_publish':
        text = json.dumps({'result_id': args.get('result_id'), 'status': 'published'})
    else:
        text = json.dumps({'question_id': args.get('question_id'), 'status': 'open'})
    return {'success': True, 'contentItems': [{'type': 'inputText', 'text': text}]}

def run_turn(c, thread, prompt, answers, members=None):
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
                c.send({'jsonrpc': '2.0', 'id': v['id'], 'result': tool_answer(v['params'], members)})
        elif v.get('method') == 'turn/completed':
            return turn, v['params']['turn']

class MemorySampler(threading.Thread):
    """Samples the inference container's cgroup (v2, systemd driver) while it
    runs: the last memory.peak read and the highest memory.current."""
    def __init__(self, state):
        super().__init__(daemon=True)
        self.state, self.stop, self.result = state, threading.Event(), {'samples': 0}
    def run(self):
        while not self.stop.wait(0.25):
            try:
                cid = json.loads((self.state / 'pi-state.json').read_text()).get('container_id')
            except (OSError, ValueError):
                continue
            cg = Path(f'/sys/fs/cgroup/system.slice/docker-{cid}.scope') if cid else None
            if not cg or not cg.is_dir():
                continue
            try:
                peak, current = int((cg / 'memory.peak').read_text()), int((cg / 'memory.current').read_text())
                events = dict(l.split() for l in (cg / 'memory.events').read_text().splitlines())
            except (OSError, ValueError):
                continue
            r = self.result
            r.update(container_id=cid, memory_peak=peak, memory_max=(cg / 'memory.max').read_text().strip(),
                     oom_kill=int(events.get('oom_kill', 0)), samples=r['samples'] + 1)
            r['current_max'] = max(r.get('current_max', 0), current)

def container_frames(state):
    """The container's coding_provider_request and coding_history frames (retained rpc stdout)."""
    requests, histories = [], []
    for f in sorted(state.glob('*-rpc-*.stdout')):
        for line in f.read_bytes().splitlines():
            try: v = json.loads(line)
            except ValueError: continue
            if v.get('type') == 'coding_provider_request':
                requests.append({'id': v.get('id'), 'contextBytes': v.get('contextBytes'), 'forwarderRefused': v.get('forwarderRefused'), 'frame_bytes': len(line)})
            elif v.get('type') == 'coding_history':
                histories.append({'invocationId': v.get('invocationId'), 'forwarderRefused': v.get('forwarderRefused'), 'frame_bytes': len(line)})
    return requests, histories

def main():
    root, name, scenario = Path(sys.argv[1]), sys.argv[2], sys.argv[3]
    root.mkdir(exist_ok=True)
    base = root / name
    members = treatment_members() if scenario == 'treatment' else advisory_members() if scenario == 'advisory' else None
    prompts = TREATMENT_PROMPTS if members else PROMPTS
    serve_config, obs, obs_out = prepare(base, scenario, members)
    sampler = MemorySampler(base / 'state'); sampler.start()
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
        for prompt in prompts if thread else []:
            turn, completed = run_turn(c, thread, prompt, out['answers'], members)
            out['turns'].append(completed)
            read = c.reply_to(c.request('thread/read', {'threadId': thread, 'includeTurns': True}), 30)
            out['reads'].append(read.get('result') or read.get('error'))
            cap = c.reply_to(c.request('pi-host/capture', {'threadId': thread, 'turnId': turn}), 30)
            out['captures'].append(cap.get('result') or cap.get('error'))
            if completed.get('status') != 'completed':
                break
            # PI_HOST_PAUSE_S: idle the thread between turns (H4: a relay
            # thread outlives the 300 s bound of other Host processes).
            time.sleep(int(os.environ.get('PI_HOST_PAUSE_S', 0)) if prompt != prompts[-1] else 0)
    except (TimeoutError, EOFError) as e:
        out['controller_error'] = str(e)
    p.stdin.close()
    try:
        p.wait(timeout=180)
    except subprocess.TimeoutExpired:
        p.kill(); p.wait()
    out['exit'] = p.returncode
    out['elapsed_s'] = round(time.time() - started, 1)
    sampler.stop.set(); sampler.join(2)
    out['memory'] = sampler.result
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
                                             'attempts': doc.get('attempts'),
                                             'observation_verified': (doc.get('observation') or {}).get('containment_verified')})
    requests, histories = container_frames(state)
    bodies = sorted(state.glob('coding-hosted-*.body.json'))
    out['sizes'] = {'requests': requests, 'histories': histories,
                    'body_bytes': {b.name: b.stat().st_size for b in bodies}, 'body_total': sum(b.stat().st_size for b in bodies),
                    'thread_read_frame_bytes': c.read_sizes}
    if members:
        out['members'] = {k: {'bytes': len(v), 'sha256': sha(v)} for k, v in members.items()}
        out['member_total'] = sum(len(v) for v in members.values())
    serve_record = state / 'pi-host-serve-record.json'
    out['serve_record'] = json.loads(serve_record.read_text())['status'] if serve_record.exists() else None
    out['host_sha256'], out['observer_sha256'] = fsha(HOST), fsha(OBSERVER)
    (base / 'summary.json').write_text(json.dumps(out, indent=1))
    print(json.dumps(out, indent=1))

if __name__ == '__main__':
    main()
