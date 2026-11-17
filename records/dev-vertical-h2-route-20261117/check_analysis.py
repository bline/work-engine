#!/usr/bin/env python3
"""Check hosted analysis runs (analyze.py) against their expected outcomes.

Usage: check_analysis.py <root-dir> <subject-commit> <run>... ; prints JSON.
Every run must: exit 0 with a passed Host receipt and a CompletedSettled
analysis turn; retain each request body create-only, with every attempt's
request_body_sha256 equal to it and to the body the synthetic provider
received; match every Host snapshot delivery natively; leave no container,
no access token in any file, an empty private-access directory, and an
inference container without network, /auth or a snapshot mount; and the gate
log must equal the attempt records.
"""
import hashlib, json, sys
from pathlib import Path

root, subject, runs = Path(sys.argv[1]), sys.argv[2], sys.argv[3:]
QUAL = root / 'bin' / 'analysis-qualification'
OBSERVER = root / 'bin' / 'pi_hosted_analysis_observer'
def fsha(p): return hashlib.sha256(Path(p).read_bytes()).hexdigest()

def check(name):
    run = root / name
    s = json.loads((run / 'summary.json').read_text())
    st = run / 'state'
    j = json.loads((st / 'pi-state.json').read_text())
    host = json.loads((st / 'analysis-host-result.json').read_text())
    out = {}
    def expect(label, ok): out[label] = bool(ok)
    expect('run used the checked binaries', s['qualification_sha256'] == fsha(QUAL) and s['observer_sha256'] == fsha(OBSERVER)
           and json.loads((run / 'inputs' / 'authority.json').read_text())['binary_sha256'] == fsha(QUAL))
    expect('exit 0, receipt passed, turn CompletedSettled', s['exit'] == 0 and host['receipt']['status'] == 'component_passed'
           and (s['result'] or {}).get('settlement') == 'CompletedSettled')
    expect('no containers left', s['containers_after'] == [])
    leaked = [str(p) for p in run.rglob('*') if p.is_file() and b'synthetic-A-' in p.read_bytes()]
    expect('Host access token appears in no run file', leaked == [])
    expect('private-access empty', not any((st / 'private-access').iterdir()))
    ins = json.loads(next(st.glob('*-inspect-0.stdout')).read_text())[0]
    mounts = [m['Destination'] for m in ins['Mounts']]
    expect('inference container: network none, no /auth, no /workspace or snapshot mount',
           ins['HostConfig']['NetworkMode'] == 'none' and not any(d.startswith('/auth') or d.startswith('/workspace') for d in mounts)
           and not any(m['Source'].startswith(str(run / 'inputs' / 'snapshot')) for m in ins['Mounts']))
    bodies = {}
    for k in s['ledger']:
        p = st / f'coding-hosted-{k}.body.json'
        bodies[k] = fsha(p) if p.exists() else None
    received = {}
    for e in s['observer']:
        if 'receipt' in e:
            received.setdefault(e['receipt']['bodySha256'], 0)
            received[e['receipt']['bodySha256']] += 1
    attempts = [(k, a) for k, r in s['ledger'].items() for a in r['attempts']]
    expect('every attempt records the retained body digest, and the provider received exactly that body',
           attempts and all(bodies[k] and a['request_body_sha256'] == bodies[k] and a['request_body_sha256'] in received for k, a in attempts))
    expect('registered body artifacts', all(any(x['path'].endswith(f'coding-hosted-{k}.body.json') and x['sha256'] == bodies[k]
                                                for x in j['artifacts']) for k in bodies))
    expect('request 1 refused on the primary, then completed on the secondary with the same body',
           [(a['route'], a['disposition']) for a in s['ledger']['1']['attempts']] == [('primary', 'refused'), ('secondary', 'completed')])
    expect('every request settled', all(r['settled_microusd'] is not None for r in s['ledger'].values()))
    reqs = s['snapshot_requests']
    expect('every Host delivery retained and matched natively',
           reqs and all(v is not None for v in reqs.values()) and sorted(s['native_snapshot_ids']) == sorted(reqs))
    for k, v in reqs.items():
        expect(f'delivery {k} artifact verifies', v and fsha(v['path']) == v['sha256'])
    log = st / 'coding-hosted-gate-attempts.json'
    gate = [(g['intent']['request'], g['intent']['attempt'], g['intent']['route'], g['intent']['payer'], g['deadlineMs'])
            for g in json.loads(log.read_text())]
    ledger = sorted((int(k), a['attempt'], a['route'], a['payer'], a['deadline_ms']) for k, a in attempts if a['gate_entered'])
    expect('gate log equals the attempt records', gate == ledger)
    if s['scenario'] in ('ok', 'long-answer'):
        expect('the shell call was refused by the SDK and never reached the Host',
               len(s['sdk_refused_ids']) == 1 and not set(s['sdk_refused_ids']) & set(reqs)
               and sorted(s['snapshot_tools'].values()) == ['snapshot_list', 'snapshot_read'])
    if s['scenario'] == 'long-answer':
        expect('long answer (16 000 characters) settled', s['result']['answer'] == 'w ' * 8000)
    if s['scenario'] == 'read-all':
        snap = run / 'inputs' / 'snapshot'
        total = sum(p.stat().st_size for p in snap.rglob('*') if p.is_file())
        read = {}
        for v in reqs.values():
            d = json.loads(Path(v['path']).read_text())
            if d['ok'] and 'text' in d['result']:
                r = d['result']; read.setdefault(r['path'], set()).add((r['offset'], r['bytes']))
        covered = sum(max((o + b for o, b in pages), default=0) for pages in read.values())
        expect('every snapshot byte was delivered (pages cover each member)', covered == total)
        biggest = max(json.loads(l).get('contextBytes', 0) for f in st.glob('*-rpc-*.stdout') for l in f.read_text().splitlines()
                      if '"coding_provider_request"' in l)
        out['largest context bytes (observation)'] = biggest
    return out, s

results = {}
for name in runs:
    checks, s = check(name)
    obs = {k: v for k, v in checks.items() if not isinstance(v, bool)}
    checks = {k: v for k, v in checks.items() if isinstance(v, bool)}
    results[name] = {'pass': all(checks.values()), 'checks': checks, 'observations': obs, 'elapsed_s': s['elapsed_s'],
                     'requests': len(s['ledger']), 'deliveries': len(s['snapshot_requests']), 'failures': s['failures'][:3]}
json.dump({'schema': 'work-engine.dev-vertical-hosted-analysis-check.v1', 'subject': subject, 'runs_root': str(root),
           'analysis_qualification_sha256': fsha(QUAL), 'observer_sha256': fsha(OBSERVER),
           'all_pass': all(r['pass'] for r in results.values()), 'runs': results}, sys.stdout, indent=1)
print()
