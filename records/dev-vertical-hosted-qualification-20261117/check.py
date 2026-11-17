#!/usr/bin/env python3
"""Check the piece (c) scenario matrix against its expected outcomes.

Usage: check.py <root-dir> <subject-commit>. Reads <root>/<scenario>.summary.json
and each run's state, and prints matrix.json (per-scenario checks and verdict).
"""
import hashlib, json, sys
from pathlib import Path

root, subject = Path(sys.argv[1]), sys.argv[2]
RESERVE = 164001
FAULTS = ['truncate', 'midstream-failed', 'late-close-long', 'trailing', 'stall-head', 'stall-body']
ORDER = ['ok', 'late-close-short', 'refuse-all', 'primary-500', *FAULTS, 'crash', 'cancel', 'cancel-head']


def attempts(s, req='1'):
    return s['ledger'].get(req, {}).get('attempts', [])


def check(name, s, st):
    c = st['coding']
    tools = list(c['tool_requests'])
    served = [e for e in s['observer'] if e['type'] == 'served']
    out = {}

    def expect(label, ok):
        out[label] = bool(ok)

    # Invariants for every scenario.
    expect('no containers left', s['containers_after'] == [])
    expect('retained artifacts verify', all(a['verified'] for a in s['hosted_artifacts']))
    expect('every attempt gate-admitted before dispatch',
           all(a['gate_entered'] for r in s['ledger'].values() for a in r['attempts']))
    run = root / name
    leaked = [str(p.relative_to(run)) for p in run.rglob('*') if p.is_file() and b'synthetic-A-' in p.read_bytes()]
    expect('Host access token appears in no run file', leaked == [])
    expect('private-access empty', not any((run / 'state' / 'private-access').iterdir()))
    inspect = run / 'state' / '0004-inspect-0.stdout'
    if inspect.exists():
        ins = json.loads(inspect.read_text())[0]
        expect('inference container network none, no /auth',
               ins['HostConfig']['NetworkMode'] == 'none' and not any(m['Destination'].startswith('/auth') for m in ins['Mounts']))
    dispatched = [e for e in s['observer'] if e['type'] in ('served', 'refused', 'failed')]
    attempted = [a for r in s['ledger'].values() for a in r['attempts'] if a['disposition'] != 'not_dispatched']
    expect('observer dispatches equal ledger attempts', len(dispatched) == len(attempted))
    expect('only refused/not-dispatched attempts released', all(
        a['disposition'] not in ('refused', 'not_dispatched') or a['relay_sha256'] is None
        for r in s['ledger'].values() for a in r['attempts']))
    for r in s['ledger'].values():
        for a in r['attempts']:
            if a['relay_sha256']:
                p = root / name / 'state' / f"coding-hosted-{[k for k, v in s['ledger'].items() if v is r][0]}-{a['attempt']}.stream.sse"
                expect(f"attempt {a['attempt']} relay bytes match digest",
                       p.exists() and hashlib.sha256(p.read_bytes()).hexdigest() == a['relay_sha256'])
    if name in ('ok', 'late-close-short'):
        expect('exit 0', s['exit'] == 0)
        expect('four requests settled', len(s['ledger']) == 4 and all(r['settled_microusd'] is not None for r in s['ledger'].values()))
        expect('request 1 refused then completed',
               [(a['route'], a['disposition']) for a in attempts(s)] == [('primary', 'refused'), ('secondary', 'completed')])
        expect('exactly one tool effect', len(tools) == 1)
        expect('gate log lists 5 attempts', len(s['gate_log'] or []) == 5)
        return out
    expect('job failed closed', s['exit'] != 0)
    expect('no tool effect after an unsettled request', tools == [])
    expect('only request 1 exists', list(s['ledger']) == ['1'])
    a = attempts(s)
    if name == 'refuse-all':
        expect('both routes refused definitively',
               [(x['route'], x['disposition'], x['http_status']) for x in a] == [('primary', 'refused', 429), ('secondary', 'refused', 402)])
        expect('nothing charged', s['ledger']['1']['charged_microusd'] == 0)
        expect('attempt limit respected', len(s['gate_log'] or []) == 2)
        return out
    if name == 'primary-500':
        expect('5xx is uncertain, no failover',
               [(x['route'], x['disposition'], x['http_status']) for x in a] == [('primary', 'uncertain', 500)])
        expect('secondary never served', served == [])
        expect('full reservation retained', s['ledger']['1']['charged_microusd'] == RESERVE)
        return out
    expect('refusal then exactly one further attempt', [x['disposition'] for x in a][:1] == ['refused'] and len(a) == 2)
    expect('second attempt uncertain', a[-1]['disposition'] == 'uncertain' if len(a) == 2 else False)
    expect('full reservation retained', s['ledger']['1']['charged_microusd'] == RESERVE)
    expect('no second served effect', len(served) == 1)
    if name in FAULTS:
        expect('relayed bytes retained', len(a) == 2 and a[1]['relay_sha256'] is not None)
    if name == 'midstream-failed':
        expect('reason names the provider failure', 'provider failure event' in (a[-1]['reason'] or ''))
    if name in ('late-close-long', 'trailing'):
        expect('completion refusal names the uncertain attempt',
               any('container completion cannot settle' in f for f in s['failures']))
    if name == 'trailing':
        p = run / 'state' / 'coding-hosted-1-2.unforwarded.bin'
        expect('unforwarded trailing bytes retained', p.exists() and b'trailing' in p.read_bytes())
    if name in ('cancel', 'cancel-head'):
        rel = json.loads((run / 'state' / 'coding-hosted-1-2.relay.json').read_text())
        expect('relay evidence reads interrupted, not completed', rel['outcome'] == 'interrupted-uncertain')
    if name in ('stall-head', 'stall-body', 'cancel', 'cancel-head'):
        rel = [e for e in s['observer'] if e['type'] == 'fault' and e.get('phase') == 'released']
        expect('upstream connection closed by the Host', bool(rel) and rel[0]['peerClosed'] is True)
    if name == 'crash':
        r = s['reconcile'] or {}
        expect('process killed before cleanup', s['exit'] == -9 and s['gate_log'] is None)
        expect('containers present before reconcile', len(s['containers_before_reconcile']) > 0)
        expect('reconcile succeeded', r.get('exit') == 0)
        expect('replay refused', r.get('replay_exit', 0) != 0 and 'reconcile instead of launch replay' in r.get('replay_stderr', ''))
        expect('unsettled attempt relabelled at restart', 'unsettled at restart' in (a[-1]['reason'] or ''))
    if name in ('cancel', 'cancel-head'):
        expect('cancellation recorded', 'cancellation requested' in s['failures'])
        expect('relayed bytes retained', a[-1]['relay_sha256'] is not None)
    return out


results = {}
for name in ORDER:
    p = root / f'{name}.summary.json'
    if not p.exists():
        results[name] = {'missing': True}
        continue
    s = json.loads(p.read_text())
    st = json.loads((root / name / 'state' / 'pi-state.json').read_text())
    checks = check(name, s, st)
    results[name] = {'pass': all(checks.values()), 'checks': checks, 'elapsed_s': s['elapsed_s'], 'exit': s['exit'],
                     'ledger': s['ledger'], 'observer': s['observer'], 'reconcile': s['reconcile'],
                     'failures': s['failures'][:3]}
json.dump({'schema': 'work-engine.dev-vertical-hosted-qualification.v1', 'subject': subject,
           'all_pass': all(r.get('pass') for r in results.values()), 'scenarios': results}, sys.stdout, indent=1)
