#!/usr/bin/env python3
"""Log live hosted-route events worth revisiting (operator, 2026-11-17).

Scans hosted Host state directories (each with pi-state.json) and appends one
JSON line per noteworthy provider attempt to the observation log, once per
(state dir, request, attempt). Noteworthy: any attempt that is not a clean
completion (refused, uncertain, not dispatched), a quota-class or credential
refusal status (401, 402, 403, 429), a completion without the framed end, and
any job that ended cancelled, crashed (reconciled) or failed. Reads nonsecret
journal fields only; refusal bodies are referenced by digest, never copied.

Usage: hosted_live_observations.py <log.jsonl> <state-or-run-dir>...
A run dir is searched for state/pi-state.json.
"""
import json, sys
from pathlib import Path

log = Path(sys.argv[1])
seen = set()
if log.exists():
    for line in log.read_text().splitlines():
        try:
            e = json.loads(line); seen.add((e['state_dir'], e.get('request'), e.get('attempt'), e['kind']))
        except (ValueError, KeyError):
            pass
out = []
for arg in sys.argv[2:]:
    root = Path(arg).expanduser().resolve()
    for journal in ([root / 'pi-state.json'] if (root / 'pi-state.json').exists() else root.rglob('state/pi-state.json')):
        st = json.loads(journal.read_text())
        coding = st.get('coding') or {}
        if not st.get('job', {}).get('coding', {}).get('schema', '').endswith('pi-coding-hosted.v1'):
            continue
        routes = {r['id']: r for r in st['job']['coding']['provider']['hosted']['routes']}
        live = any(r['endpoint'].startswith('https:') for r in routes.values())
        base = {'state_dir': str(journal.parent), 'job_id': st.get('job', {}).get('job_id'), 'live_routes': live}
        for req, r in sorted(coding.get('provider_requests', {}).items(), key=lambda kv: int(kv[0])):
            for a in r.get('attempts', []):
                notes = []
                if a['disposition'] != 'completed':
                    notes.append(a['disposition'])
                if a.get('http_status') in (401, 402, 403, 429):
                    notes.append(f"status_{a['http_status']}")
                if a['disposition'] == 'completed' and a.get('framed_end_observed') is False:
                    notes.append('framed_end_not_observed')
                for kind in notes:
                    key = (base['state_dir'], int(req), a['attempt'], kind)
                    if key in seen:
                        continue
                    seen.add(key)
                    out.append({**base, 'kind': kind, 'request': int(req), 'attempt': a['attempt'],
                                'route': a['route'], 'payer': a['payer'],
                                'endpoint': routes.get(a['route'], {}).get('endpoint'),
                                'http_status': a.get('http_status'), 'reason': a.get('reason'),
                                'refusal_body_sha256': a.get('refusal_body_sha256'),
                                'relay_sha256': a.get('relay_sha256')})
        failures = st.get('failures') or []
        job_kind = ('cancelled' if 'cancellation requested' in failures
                    else 'failed' if coding.get('provider_failed') or failures else None)
        if job_kind and (base['state_dir'], None, None, job_kind) not in seen:
            seen.add((base['state_dir'], None, None, job_kind))
            out.append({**base, 'kind': job_kind, 'phase': st.get('phase'), 'failures': failures[:3]})
with log.open('a') as f:
    for e in out:
        f.write(json.dumps(e, sort_keys=True) + '\n')
print(json.dumps({'appended': len(out), 'log': str(log)}))
