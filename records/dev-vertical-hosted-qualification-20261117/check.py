#!/usr/bin/env python3
"""Check the hosted qualification matrix against its expected outcomes.

Usage: check.py <root-dir> <subject-commit>. Reads <root>/<scenario>.summary.json
and each run's state, and prints matrix.json: the binaries' digests, the runs
root, and per-scenario checks and verdicts. Rerunning it over the same run set
reproduces matrix.json exactly.

Charges: the Host does not export its charge. Checks about money compare the
Host-recorded dispositions and settlements; `charged_microusd_derived` (from
qualify.py) is this harness's own application of the ledger rule and is
labelled as such.
"""
import hashlib, json, sys
from pathlib import Path

root, subject = Path(sys.argv[1]), sys.argv[2]
REPO = Path('/home/bline/code/work-engine')
QUAL = REPO / 'rust/crates/pi-pressure-context-host/target/release/coding-qualification'
OBSERVER = REPO / 'rust/target/debug/examples/pi_hosted_coding_observer'
RESERVE = 164001
COMPLETES = ['ok', 'late-close-short', 'late-close-long', 'trailing']
PRIMARY_FAULTS = ['primary-500', 'primary-429-echo', 'primary-429-truncated']
FAULTS = ['truncate', 'midstream-failed', 'trailing-data', 'echo-body', 'echo-trailing', 'stall-head', 'stall-body']
ORDER = [*COMPLETES, 'refuse-all', *PRIMARY_FAULTS, *FAULTS, 'truncate-3', 'crash', 'cancel', 'cancel-head']


def fsha(p):
    return hashlib.sha256(Path(p).read_bytes()).hexdigest()


def attempts(s, req='1'):
    return s['ledger'].get(req, {}).get('attempts', [])


def relay_json(run, req, att):
    p = run / 'state' / f'coding-hosted-{req}-{att}.relay.json'
    return json.loads(p.read_text()) if p.exists() else {}


def check(name, s, st):
    c = st['coding']
    tools = list(c['tool_requests'])
    served = [e for e in s['observer'] if e['type'] == 'served']
    run = root / name
    out = {}

    def expect(label, ok):
        out[label] = bool(ok)

    # Invariants for every scenario.
    expect('run used the checked binaries',
           s['coding_qualification_sha256'] == BINARIES['coding_qualification_sha256']
           and s['observer_sha256'] == BINARIES['observer_sha256']
           and json.loads((run / 'inputs' / 'authority.json').read_text())['binary_sha256'] == BINARIES['coding_qualification_sha256'])
    expect('no containers left', s['containers_after'] == [])
    expect('retained artifacts verify', all(a['verified'] for a in s['hosted_artifacts']))
    expect('every attempt gate-admitted before dispatch',
           all(a['gate_entered'] for r in s['ledger'].values() for a in r['attempts']))
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
    every = [(k, a) for k, r in s['ledger'].items() for a in r['attempts']]
    expect('released (refused/not-dispatched) attempts relayed nothing; every other attempt is completed or uncertain',
           all((a['disposition'] in ('refused', 'not_dispatched') and a['relay_sha256'] is None)
               or a['disposition'] in ('completed', 'uncertain') for _, a in every))
    for k, a in every:
        if a['relay_sha256']:
            p = run / 'state' / f"coding-hosted-{k}-{a['attempt']}.stream.sse"
            expect(f"request {k} attempt {a['attempt']} retained stream matches its digest and is registered",
                   p.exists() and fsha(p) == a['relay_sha256']
                   and any(x['name'] == p.name for x in s['hosted_artifacts']))
    # The gate's own log, per attempt (absent only when the Host was killed).
    if name != 'crash':
        log = [(g['intent']['request'], g['intent']['attempt'], g['intent']['route'], g['intent']['payer'], g['deadlineMs'])
               for g in s['gate_log'] or []]
        ledger = sorted((int(k), a['attempt'], a['route'], a['payer'], a['deadline_ms']) for k, a in every if a['gate_entered'])
        expect('gate log matches the attempt records (request, attempt, route, payer, deadline)',
               s['gate_log'] is not None and log == ledger)

    if name in COMPLETES:
        st_attempts = c['provider_requests']['1']['attempts']
        if name == 'late-close-long':
            # K-a (operator decision 2026-11-17): settled on the terminal event.
            expect('settled on terminal event, framed end not observed',
                   st_attempts[-1].get('framed_end_observed') is False
                   and 'framed end not observed' in (st_attempts[-1].get('reason') or ''))
        else:
            expect('framed end observed', st_attempts[-1].get('framed_end_observed') is True)
        if name == 'trailing':
            # Q-a: a comment after the terminal is clean with the framed end.
            p = run / 'state' / 'coding-hosted-1-2.unforwarded.bin'
            expect('unforwarded trailing comment screened and retained', p.exists() and b'trailing' in p.read_bytes())
            rel = relay_json(run, 1, 2)
            expect('drain ran to the framed end, nothing dropped',
                   rel.get('drained') is True and rel.get('heldDropped') == 0 and rel.get('outcome') == 'released-after-upstream-end')
        expect('exit 0', s['exit'] == 0)
        expect('four requests settled', len(s['ledger']) == 4 and all(r['settled_microusd'] is not None for r in s['ledger'].values()))
        expect('request 1 refused then completed',
               [(a['route'], a['disposition']) for a in attempts(s)] == [('primary', 'refused'), ('secondary', 'completed')])
        expect('exactly one tool effect', len(tools) == 1)
        expect('five gate-admitted attempts', len(s['gate_log'] or []) == 5)
        return out
    expect('job failed closed', s['exit'] != 0)
    if name == 'truncate-3':
        expect('requests 1 and 2 settled', all(s['ledger'][k]['settled_microusd'] is not None for k in ('1', '2')))
        a3 = attempts(s, '3')
        expect('request 3 (after the reset) one attempt, uncertain, relayed bytes retained',
               [(x['route'], x['disposition']) for x in a3] == [('secondary', 'uncertain')] and a3[0]['relay_sha256'] is not None)
        expect('request 3 stays unsettled', s['ledger']['3']['settled_microusd'] is None)
        expect('no request 4', '4' not in s['ledger'])
        expect('exactly one tool effect (request 1)', len(tools) == 1)
        expect('three served effects, none after the fault', len(served) == 3)
        return out
    expect('no tool effect after an unsettled request', tools == [])
    expect('only request 1 exists', list(s['ledger']) == ['1'])
    a = attempts(s)
    if name == 'refuse-all':
        expect('both routes refused definitively',
               [(x['route'], x['disposition'], x['http_status']) for x in a] == [('primary', 'refused', 429), ('secondary', 'refused', 402)])
        expect('every attempt released (derived)', s['ledger']['1']['charged_microusd_derived'] == 0)
        expect('attempt limit respected', len(s['gate_log'] or []) == 2)
        expect('completion refusal says the released attempt was released',
               any('reservation was released' in f for f in s['failures']))
        return out
    if name in PRIMARY_FAULTS:
        expect('primary is uncertain, no failover',
               [(x['route'], x['disposition'], x['http_status']) for x in a] == [('primary', 'uncertain', 500 if name == 'primary-500' else 429)])
        expect('secondary never served', served == [])
        expect('full reservation retained (derived)', s['ledger']['1']['charged_microusd_derived'] == RESERVE)
        if name == 'primary-429-echo':
            expect('refusal body with access material not retained',
                   not (run / 'state' / 'coding-hosted-1-1.refusal.body').exists()
                   and 'hosted_access_exclusion' in (a[0]['reason'] or ''))
        if name == 'primary-429-truncated':
            expect('truncated refusal is not definitive',
                   relay_json(run, 1, 1).get('interruption') == 'upstream_truncated')
        return out
    expect('refusal then exactly one further attempt', [x['disposition'] for x in a][:1] == ['refused'] and len(a) == 2)
    expect('second attempt uncertain', a[-1]['disposition'] == 'uncertain' if len(a) == 2 else False)
    expect('full reservation retained (derived)', s['ledger']['1']['charged_microusd_derived'] == RESERVE)
    expect('no second served effect', len(served) == 1)
    if name in FAULTS:
        if name in ('stall-head', 'cancel-head'):
            expect('relay digest recorded (no head, zero bytes)', len(a) == 2 and a[1]['relay_sha256'] == hashlib.sha256(b'').hexdigest())
        else:
            expect('relayed bytes retained', len(a) == 2 and a[1]['relay_sha256'] not in (None, hashlib.sha256(b'').hexdigest()))
    if name == 'midstream-failed':
        expect('reason names the provider failure', 'provider failure event' in (a[-1]['reason'] or ''))
    if name == 'trailing-data':
        expect('completion refusal names the uncertain attempt',
               any('container completion cannot settle' in f and 'reservation retained' in f for f in s['failures']))
        p = run / 'state' / 'coding-hosted-1-2.unforwarded.bin'
        expect('unforwarded data event retained', p.exists() and b'after the terminal event' in p.read_bytes())
    if name == 'echo-body':
        rel = relay_json(run, 1, 2)
        expect('access screen interrupted the relay before the echoed chunk',
               rel.get('interruption') == 'hosted_access_exclusion' and rel.get('forwardedBytes', 0) > 0)
    if name == 'echo-trailing':
        rel = relay_json(run, 1, 2)
        expect('drain screened the echoed trailing bytes and dropped them',
               rel.get('interruption') == 'hosted_access_exclusion' and rel.get('drained') is True
               and rel.get('heldDropped', 0) > 0
               and not (run / 'state' / 'coding-hosted-1-2.unforwarded.bin').exists())
    if name in ('cancel', 'cancel-head'):
        rel = relay_json(run, 1, 2)
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
        expect('observer alive through reconcile and replay; no dispatch after the kill',
               r.get('observer_alive_after_replay') is True and s['dispatches_after_exit'] == 0)
        expect('ledger after the replay equals the ledger after reconcile', s['ledger'] == s['ledger_after_reconcile'])
        expect('unsettled attempt relabelled at restart', 'unsettled at restart' in (a[-1]['reason'] or ''))
        p = run / 'state' / 'coding-hosted-1-2.stream.sse'
        expect('bytes relayed before the crash retained, registered and digested; head status kept',
               p.exists() and p.stat().st_size > 0 and a[-1]['relay_sha256'] == fsha(p)
               and a[-1]['http_status'] == 200 and 'relayed bytes retained' in (a[-1]['reason'] or ''))
    if name in ('cancel', 'cancel-head'):
        expect('cancellation recorded', 'cancellation requested' in s['failures'])
    return out


BINARIES = {'coding_qualification_sha256': fsha(QUAL), 'observer_sha256': fsha(OBSERVER)}
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
json.dump({'schema': 'work-engine.dev-vertical-hosted-qualification.v2', 'subject': subject, 'runs_root': str(root),
           **BINARIES, 'all_pass': all(r.get('pass') for r in results.values()), 'scenarios': results}, sys.stdout, indent=1)
print()
