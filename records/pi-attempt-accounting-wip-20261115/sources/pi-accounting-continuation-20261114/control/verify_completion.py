import hashlib
import json
import pathlib
import tarfile

w = pathlib.Path('/home/bline/cbm-work/pi-accounting-continuation-20261114')
s = w / 'realization/s1'
baseline = w / 'completion/baseline-freeze/subject.tar'
failures = []
old = {}
with tarfile.open(baseline) as archive:
    for member in archive.getmembers():
        rel = member.name.removeprefix('./')
        if member.isfile() and rel.startswith('corpus/cases/') and rel.endswith('.json'):
            old[rel] = json.load(archive.extractfile(member))
        elif member.isfile() and rel == 'oracle/R8-LOW-ASSERTIONS.json':
            old_claims = json.load(archive.extractfile(member))['assertions']

def stripped(value):
    if isinstance(value, dict):
        return {key: stripped(item) for key, item in value.items()
                if key not in ('accounting_result_ref', 'supported_low_evidence_ref')}
    if isinstance(value, list):
        return [stripped(item) for item in value]
    return value

for rel, before in old.items():
    after = json.loads((s / rel).read_bytes())
    for key, value in before.items():
        if key == 'revisions_r8':
            if after.get(key, [])[:len(value)] != value:
                failures.append([rel, key, 'historical R8 prefix changed'])
        elif stripped(after.get(key)) != stripped(value):
            failures.append([rel, key, 'non-reference field changed'])
    added = set(after) - set(before)
    if added - {'revisions_r8'}:
        failures.append([rel, sorted(added), 'unexpected new top-level fields'])

current_claims = json.loads((s / 'oracle/R8-LOW-ASSERTIONS.json').read_bytes())['assertions']
claim_ids = [claim['claim_id'] for claim in current_claims]
if len(claim_ids) != len(set(claim_ids)):
    failures.append(['low assertions', 'duplicate claim id'])
for claim in old_claims:
    if claim not in current_claims:
        failures.append(['low assertions', claim['claim_id'], 'old exact record not preserved'])

bindings = []
for rel in sorted(old):
    p = s / rel
    bindings.append({'path': rel, 'sha256': hashlib.sha256(p.read_bytes()).hexdigest()})
for partition in ['A', 'B']:
    for p in sorted((s / 'oracle' / ('r8-completion-' + partition)).glob('*')):
        if p.is_file():
            bindings.append({'path': str(p.relative_to(s)), 'sha256': hashlib.sha256(p.read_bytes()).hexdigest()})
result = {'cases': len(old), 'low_claims': len(current_claims),
          'old_claims_preserved': len(old_claims), 'failures': failures,
          'standing': 'mechanical preservation/current byte binding only; not financial qualification',
          'files': bindings}
out = w / 'control/completion-current-binding.json'
out.write_text(json.dumps(result, indent=2) + '\n')
print(json.dumps({'cases': len(old), 'claims': len(current_claims),
                  'old_claims': len(old_claims), 'failures': failures,
                  'binding_files': len(bindings), 'binding': str(out),
                  'sha256': hashlib.sha256(out.read_bytes()).hexdigest()}))
raise SystemExit(bool(failures))
