#!/usr/bin/env python3
"""Derive evidence.json from one run directory written by run.py. Usage: evidence.py <run-dir> <subject-commit>"""
import glob, hashlib, json, os, sys
r, subject = sys.argv[1], sys.argv[2]
s = r + '/state'
def sha(p): return hashlib.sha256(open(p, 'rb').read()).hexdigest()
ins = json.load(open(s + '/0004-inspect-0.stdout'))[0]
d = json.load(open(s + '/pi-state.json')); c = d['coding']
rep = json.load(open(s + '/coding-continuity-result.json'))
hr = json.load(open(s + '/coding-host-result.json'))
auth = json.load(open(r + '/inputs/authority.json'))
obs = [json.loads(l) for l in open(r + '/observer.jsonl')]
hosted = [a for a in d['artifacts'] if '/coding-hosted-' in a['path']]
json.dump({
 'schema': 'work-engine.dev-vertical-hosted-offline-run.v2', 'subject': subject,
 'run': {'job_sha256': sha(r + '/job.json'), 'coding_qualification_sha256': auth['binary_sha256'],
         'hosted_routes_sha256': auth['hosted_routes_sha256'], 'stdout': open(r + '/qualification.stdout').read().strip()},
 'inference_container': {'NetworkMode': ins['HostConfig']['NetworkMode'], 'ReadonlyRootfs': ins['HostConfig'].get('ReadonlyRootfs'),
         'mount_targets': sorted(m['Destination'] for m in ins['Mounts']), 'env_names': sorted(e.split('=')[0] for e in ins['Config']['Env']),
         'private_access_entries': len(glob.glob(s + '/private-access/*'))},
 'ledger': {k: {'settled_microusd': v['settled_microusd'], 'attempts': v['attempts']} for k, v in c['provider_requests'].items()},
 'hosted_journal_artifacts': [{'name': os.path.basename(a['path']), 'sha256': a['sha256'], 'verified': sha(a['path']) == a['sha256']} for a in hosted],
 'gate_attempts': json.load(open(s + '/coding-hosted-gate-attempts.json')),
 'lifecycle_report': {k: rep[k] for k in ('continued_parent', 'provider_requests', 'physical_resets', 'hosted_attempts')},
 'host_receipt': {'status': hr['receipt']['status'], 'receipt_id': hr['receipt_id']},
 'validation': {'validation_exit': c['validation_exit'], 'final_acceptance': c['final_acceptance']},
 'observer': obs[-1], 'failures': d['failures'],
}, sys.stdout, indent=1)
