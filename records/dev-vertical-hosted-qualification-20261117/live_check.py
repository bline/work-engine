#!/usr/bin/env python3
"""Checks for a live hosted run: secrets absent from every run file, results.

Usage: live_check.py <run-dir>. Reads the real access secrets (Pi auth store,
OPENAI_API_KEY) only to search for them; prints booleans and counts only.
"""
import hashlib, json, os, sys
from pathlib import Path
run = Path(sys.argv[1])
auth = json.loads(Path('/home/bline/.pi/agent/auth.json').read_text())['openai-codex']
secrets = {'chatgpt_access': auth['access'].encode(), 'chatgpt_refresh': auth['refresh'].encode(),
           'openai_api_key': os.environ['OPENAI_API_KEY'].encode()}
found = {k: [] for k in secrets}
files = 0
for p in run.rglob('*'):
    if p.is_file():
        files += 1
        b = p.read_bytes()
        for k, v in secrets.items():
            if v in b:
                found[k].append(str(p.relative_to(run)))
st = json.loads((run / 'state' / 'pi-state.json').read_text())
c = st['coding']
arts = [a for a in st['artifacts'] if '/coding-hosted-' in a['path']]
verified = all(hashlib.sha256(Path(a['path']).read_bytes()).hexdigest() == a['sha256'] for a in arts)
inspect = run / 'state' / '0004-inspect-0.stdout'
ins = json.loads(inspect.read_text())[0] if inspect.exists() else None
out = {
    'files_scanned': files, 'secrets_found': {k: v for k, v in found.items()},
    'phase': st['phase'], 'completed': c.get('completed'), 'provider_failed': c.get('provider_failed'),
    'tool_effects': len(c['tool_requests']), 'hosted_artifacts': len(arts), 'artifacts_verify': verified,
    'inference_network': ins and ins['HostConfig']['NetworkMode'],
    'inference_auth_mounts': ins and [m['Destination'] for m in ins['Mounts'] if m['Destination'].startswith('/auth')],
    'requests': {k: {'settled_microusd': r['settled_microusd'], 'backend_model': r.get('backend_model'),
                     'attempts': [(a['route'], a['payer'], a['disposition'], a.get('http_status'), a.get('framed_end_observed')) for a in r['attempts']],
                     'raw_usage': (r.get('raw_terminal') or {}).get('usage')} for k, r in c['provider_requests'].items()},
}
(run / 'live-check.json').write_text(json.dumps(out, indent=1))
print(json.dumps(out, indent=1))
