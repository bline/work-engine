#!/usr/bin/env python3
"""Deterministic per-case/per-scope disposition for the S1 scope-closure plan.
Inputs: the repository subject (experiments/pi-attempt-accounting/s1), the derivation-stage LEDGER,
and the owner records (batch-1 and slice-2 dossiers, REVISIONS). Output: CASE-DISPOSITION.json."""
import json, sys, os, glob, collections, hashlib, re
S, LEDGER, REC, OUT = sys.argv[1:5]
L = json.load(open(LEDGER))
dossiers = {}
for f in glob.glob(REC + '/slice1/oracle-batch1/dossier/*.json') + glob.glob(REC + '/slice2/oracle-r2-evidence/dossier/*.json'):
    if f.endswith('schema.json'): continue
    d = json.load(open(f)); dossiers.setdefault(d['case_id'], []).append(os.path.relpath(f, REC))
def findings_of(cid):
    out = set()
    for rel in dossiers.get(cid, []):
        d = json.load(open(os.path.join(REC, rel)))
        for c in d.get('claims', []):
            for x in c.get('findings', []) or []: out.add(x['finding_id'])
    return out
rev = json.load(open(S + '/oracle/r11-revisions/REVISIONS.json'))
rev_cases = collections.defaultdict(set)
def walk(o):
    if isinstance(o, dict):
        cid = o.get('case_id') or o.get('case'); fid = o.get('finding') or o.get('finding_id')
        if isinstance(cid, str) and isinstance(fid, str): rev_cases[cid].add(fid)
        for v in o.values(): walk(v)
    elif isinstance(o, list):
        for v in o: walk(v)
walk(rev)
for r in rev.get('revisions', []):
    f = r.get('file', ''); stem = os.path.basename(f).split('.')[0]
    m = re.search(r'R8-[A-Z-]+-01', r.get('finding', ''))
    if stem and m: rev_cases[stem].add(m.group(0))
RETIREMENT_EVIDENCE = {  # root-curated by case name and contract role (retire/resume/reopen of legacy sessions)
    'N05-control-consistent-retirement', 'N05-retirement-before-later-delivery',
    'N07-control-retired-journal', 'N07-control-retired-legacy', 'N07-reopened-session-journal', 'N07-reopened-session-legacy',
    'P130-R04-retired-session-releases-commitment', 'P130-R10-reopened-then-retired', 'P130-R11-open-session-before-retirement',
    'P132-CP-retired-session-resumed-without-admission',
    'U03-retirement-before-turn-evidence', 'U03-control'}  # U03 added after plan review F3
# Ledger-independent invariants transferred to the dev vertical (PLAN-R2 section 4.1).
INVARIANTS = {'I1': 'retained raw charge cannot be erased or replaced by a smaller reservation',
              'I2': 'malformed/missing/null/empty identity cannot yield a complete or favourable settlement',
              'I3': 'no self-declared authority; non-empty identities before equality',
              'I4': 'placement from raw events and times',
              'I5': 'distinct money evidence is not merged by equal amounts; exact copies do not double a hold',
              'I6': 'uncertainty never releases a hold'}
FINDING_INVARIANTS = {'R8-FIN-01': ['I1'], 'R8-IDENTITY-01': ['I2'], 'R8-NAMESPACE-01': ['I3'], 'R8-GATE-IDENTITY-01': ['I3'],
                      'R8-COVERAGE-01': ['I3', 'I6'], 'R8-PLACEMENT-01': ['I4'], 'R8-RESERVATION-01': ['I5'],
                      'R8-LOW-INVENTORY-01': ['I6'], 'R8-OWNER-01': [], 'R8-SCHEMA-DOMAIN-01': [], 'R8-DOC-01': []}
# Root-curated after plan review F2 (cases exercising an invariant without a finding link); selection proxy, not exhaustive.
INVARIANT_TAGS = {
    'P133-A01-incomplete-usage-above-reservation': ['I1'], 'P130-R04-priority-overrun-above-cap': ['I1'],
    'P2-usage-settlement-unpriced-identity-keeps-charge': ['I1', 'I2'], 'S01-reservation-basis-charged-less': ['I1'],
    'S01-reserved-usd-omitted': ['I1'], 'S01-upstream-error-charged-zero': ['I1'], 'R6-C02-non200-success-terminal-usage-control': ['I1'],
    'VC04-upstream-error-keeps-reservation': ['I6'], 'VC05-unsettled-reservation': ['I6'], 'P130-R01-usage-incomplete-terminal': ['I6'],
    'S02-control-error-status-after-reservation': ['I6'], 'P130-Q11-closure-uncertain': ['I6'],
    'P133-A05-model-case-variant': ['I2'], 'N08-model-without-producer-rate': ['I2'], 'U05-inconsistent-charge-without-response-id': ['I2'],
    'U05-usage-settlement-without-response-id': ['I2'], 'P133-A04-cached-malformed': ['I2'],
    'C3-identical-reservation-copy': ['I5'], 'N04-duplicated-read-failure': ['I5'], 'P130-R05-conflicting-keeps-valuation-high': ['I5'],
    'P132-CP-closed-valid-close-entry': ['I3'], 'A1-06-session-file-is-not-evidence': ['I3']}
ADM = {}
_adm = os.path.join(os.path.dirname(os.path.abspath(sys.argv[0])), '..', 'slice2', 'ROOT-ADMISSIONS.json')
for e in json.load(open(_adm))['entries']: ADM.setdefault(e['case_id'], []).append(e['entry_id'])
cases = []; counts = collections.Counter()
for c in L['cases']:
    cid = c['case_id']; prof = c['profile'].get('value')
    fset = sorted(findings_of(cid) | rev_cases.get(cid, set()))
    future = prof == 'future_qualified_gate'
    inv = sorted(set(sum((FINDING_INVARIANTS.get(f, []) for f in fset), [])) | set(INVARIANT_TAGS.get(cid, [])))
    transfer = future or bool(inv)
    rec = {
        'case_id': cid, 'profile': prof, 'scopes': c['scope_ids'],
        'historical_record': 'RETAINED',
        'full_s1_qualification': 'SUPERSEDED',
        'superseded_reason': 'future-profile contract dropped as implementation target (operator R1)' if future else 'legacy role_host/pi_container consumer intended for retirement; no full S1/S2a for it (operator R2)',
        'independent_derivation': ('DERIVED (owner dossier)' if cid in dossiers else 'NOT_DERIVED (not pursued)'),
        'dossiers': sorted(dossiers.get(cid, [])),
        'outputs_corrected_in_campaign': cid in rev_cases,
        'findings': fset,
        'invariants': inv,
        'case_file': f'experiments/pi-attempt-accounting/s1/corpus/cases/{cid}.json',
        'root_admission_entry_ids': ADM.get(cid, []),
        'transferred_qualification': ('DEV_VERTICAL: invariant regression scenario against the hosted ledger' if transfer else None),
        'retirement_evidence': ('LEGACY_RETIREMENT_OWNER: resumable/retired session behaviour to dispose at cutover (CONTRACT 2.3 producer fact)' if cid in RETIREMENT_EVIDENCE else None),
    }
    counts[(prof, rec['independent_derivation'].split()[0], 'transfer' if transfer else 'no-transfer')] += 1
    cases.append(rec)
summary = {'cases': len(cases), 'scopes': sum(len(c['scopes']) for c in cases),
           'by_profile_derivation_transfer': {' / '.join(k): v for k, v in sorted(counts.items())},
           'transferred_cases': sum(1 for c in cases if c['transferred_qualification']),
           'derived_cases': sum(1 for c in cases if c['dossiers']),
           'invariant_counts': dict(collections.Counter(i for c in cases for i in c['invariants'])),
           'transferred_with_root_admission': sorted(c['case_id'] for c in cases if c['transferred_qualification'] and c['root_admission_entry_ids']),
           'retirement_evidence_cases': sum(1 for c in cases if c['retirement_evidence']),
           'outputs_corrected_cases': sorted(c['case_id'] for c in cases if c['outputs_corrected_in_campaign'])}
ids = {x['case_id'] for x in cases}
assert RETIREMENT_EVIDENCE <= ids and set(INVARIANT_TAGS) <= ids and set(ADM) - {'R11-CTL-admission-namespace-grouped'} <= ids, 'unknown curated case'
json.dump({'schema': 'pi-accounting-s1-case-disposition/2', 'standing': 'scope-closure mapping; not qualification',
           'paths': {'case_file': 'relative to repository root /home/bline/code/work-engine', 'dossiers': 'relative to records/pi-attempt-accounting-owner-20261116/'},
           'invariants': INVARIANTS, 'finding_invariants': FINDING_INVARIANTS,
           'summary': summary, 'cases': cases}, open(OUT, 'w'), indent=1)
print(json.dumps(summary, indent=1))
