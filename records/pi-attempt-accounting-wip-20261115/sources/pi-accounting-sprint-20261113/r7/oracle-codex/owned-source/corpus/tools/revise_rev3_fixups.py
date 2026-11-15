"""Revision-3 corrections of pre-existing (revision-1) corpus defects that the strengthened revision-3 validator found
on its first run (evidence/validator-first-run-check.txt). Each change is appended to the case's `revisions_r3` log
with its rule. Run once, after revise_rev3.py. Usage: revise_rev3_fixups.py <corpus_dir>"""
import json, os, sys

corpus = sys.argv[1]
FIX = {
    # A-3 (since 1.0-s1): owned spend above the cap is NotEstablished{CommitmentInvalid}; known owned high 1.1 > cap 1.0
    'A1-01-upstream-error-and-unknown-event': [('default', 'o', 'reasons_add', 'CommitmentInvalid',
        'A-3 (since 1.0-s1, first checked in revision 3): owned spend above the cap gives CommitmentInvalid')],
    # A-3 (since 1.0-s1, N-07 repair): only a lifecycle state Retired releases; a contradicted lifecycle does not
    'R06-launch-row-missing': [('default', 'o', 'commitment', ('a1', 'launched_open', '500000000', 'at_least'),
        'A-3 (since 1.0-s1): a contradicted lifecycle is not Retired, so the commitment is not released; A-5: it is AtLeast')],
    'U03-retirement-before-turn-evidence': [('default', 'o', 'commitment', ('a1', 'launched_open', '985000000', 'at_least'),
        'A-3 (since 1.0-s1): a contradicted lifecycle is not Retired, so the commitment is not released; A-5: it is AtLeast')],
}
for cid, fixes in FIX.items():
    p = os.path.join(corpus, 'cases', cid + '.json'); c = json.load(open(p)); rev = c['revisions_r3']
    for ev, obj, kind, val, rule in fixes:
        oe = next(o for o in next(e for e in c['evaluations'] if e['name'] == ev)['expected']['objectives'] if o['objective'] == obj)
        if kind == 'commitment':
            aid, state, committed, k = val
            cm = next(x for x in oe['commitments'] if x['admission_id'] == aid)
            for f, new in (('state', state), ('committed', committed), ('kind', k)):
                if cm[f] != new: rev.append({'rule': rule, 'field': f'{ev}/{obj}.commitments[{aid}].{f}', 'from': cm[f], 'to': new}); cm[f] = new
            val = 'UnenforcedCommitment'
        new = sorted(set(oe['reasons']) | {val})
        if new != oe['reasons']:
            rev.append({'rule': rule, 'field': f'{ev}/{obj}.reasons', 'from': oe['reasons'], 'to': new}); oe['reasons'] = new
    json.dump(c, open(p, 'w'), indent=1, sort_keys=True); open(p, 'a').write('\n')
    print('fixed', cid)
