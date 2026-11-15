"""Revision 4 of the corpus (contract 1.3-s1, P1.32): applies OQ-1 to OQ-4 to the revision-3 cases, which all become
profile `current_producer` (release 92a03789: no gate evidence exists). Every changed field is logged in `revisions_r4`
with its rule; `revisions`, `revisions_r3`, inputs and historical blocks are never touched. Fields that did not exist
before are logged with from "__absent__". Usage: revise_rev4.py <corpus_dir>"""
import glob, json, os, re, sys

corpus = sys.argv[1]
ABSENT = '__absent__'
STAMP = re.compile(r'^\d{4}-\d\d-\d\dT\d\d:\d\d:\d\dZ$')
OPEN_KINDS = {'pending', 'launched_open', 'retired_resumable', 'reopened'}


def add(rev, rule, field, old, new): rev.append({'rule': rule, 'field': field, 'from': old, 'to': new})
def setf(rev, rule, obj, key, new, path):
    old = obj.get(key, ABSENT)
    if old != new: add(rev, rule, path, old, new); obj[key] = new


def closure(o, ref, admissions_rows):
    """CONTRACT 1.3-s1 12.4 step 1, current_producer profile (legacy cooperative trust)."""
    ca, ce = o.get('closed_at'), o.get('close')
    if ca is None and ce is None: return 'open'
    ok = (isinstance(ce, dict) and isinstance(ca, str) and STAMP.match(ca) and ce.get('at') == ca and ca >= o['started_at'] and ca <= ref
          and not any(isinstance(r.get('at'), str) and r['at'] > ca for r in admissions_rows))
    return 'closed' if ok else 'claimed_not_established'


changed = 0; report = []
for path in sorted(glob.glob(os.path.join(corpus, 'cases', '*.json'))):
    c = json.load(open(path))
    if c.get('contract') != 'pi-accounting-contract/1.2-s1': continue          # idempotent: revision-3 cases only
    rev = []; ref = c['inputs']['reference_time']
    sess = {s['session_id']: s['alias'] for s in c['inputs']['sessions']}
    setf(rev, 'OQ-1 2.3: capability profile (release 92a03789 writes no gate evidence)', c, 'profile', 'current_producer', 'profile')
    atts = {a['key']: a for a in c['expected']['attempts']}
    life = c['expected']['lifecycle']
    for e in c['evaluations']:
        ee = e['expected']; objs = {o['objective']: o for o in e['objectives']}; exp_objs = {o['objective']: o for o in ee['objectives']}
        for name, oe in exp_objs.items():
            o = objs[name]; adm = {a['admission_id']: a for a in o['admissions']}
            for cm in oe['commitments']:
                P = f"{e['name']}/{name}.commitments[{cm['admission_id']}]"
                setf(rev, 'OQ-1 A-5: every commitment records its gate qualification; current producer: no gate evidence', cm, 'gate',
                     {'qualified': False, 'reasons': ['NoGateEvidence']}, P + '.gate')
                a = adm.get(cm['admission_id'])
                if cm['state'] in ('retired', 'launched_open', 'reopened') and a is not None:
                    caps = [r for r in a['rows'] if r.get('event') in ('pending', 'objective_admission')]
                    cap = int(round(caps[0]['max_usd'] * 10 ** 9)) if caps and isinstance(caps[0].get('max_usd'), (int, float)) and not isinstance(caps[0].get('max_usd'), bool) else None
                    alias = sess.get(a['session'])
                    spent = sum(int(atts[at['attempt']]['exposure']['high']['nano']) for at in ee['attribution'] if at['ownership'] == 'owned' and at['objective'] == name
                                and atts[at['attempt']]['exposure'].get('pool') == 'pi_api' and at['attempt'].split(':')[1].split('/')[0] == alias)
                    if cm['state'] == 'retired':
                        setf(rev, 'OQ-1 A-3/L-7: retirement without a qualified gate keeps the commitment (retired_resumable)', cm, 'state', 'retired_resumable', P + '.state')
                    if cap is not None:
                        setf(rev, 'OQ-1 A-3: committed = max(0, cap - owned high) while the session can resume', cm, 'committed', str(max(0, cap - spent)), P + '.committed')
                    setf(rev, 'OQ-1 A-5: a resumable session commitment is AtLeast without a qualified gate', cm, 'kind', 'at_least', P + '.kind')
        for name, oe in exp_objs.items():                     # predecessor lines: retired_resumable commitments now count
            pred = objs[name].get('predecessor')
            if pred and pred in exp_objs:
                p = exp_objs[pred]
                unres = [atts[at['attempt']]['exposure'] for at in ee['attribution'] if at['ownership'] == 'owned' and at['objective'] == pred
                         and atts[at['attempt']]['exposure'].get('pool') == 'pi_api' and atts[at['attempt']]['billing'] == 'unresolved']
                com = [cm for cm in p['commitments'] if cm['state'] in OPEN_KINDS]
                kind = 'at_least' if unres or any(cm['kind'] == 'at_least' for cm in com) else 'exact'
                new = {'low': '0', 'high': {'kind': kind, 'nano': str(sum(int(x['high']['nano']) for x in unres) + sum(int(cm['committed']) for cm in com))}}
                setf(rev, 'OQ-1 12.2: the predecessor line includes retired-resumable commitments', oe, 'predecessor_obligations', new, f"{e['name']}/{name}.predecessor_obligations")
        for name, oe in exp_objs.items():
            o = objs[name]; rows = [r for a in o['admissions'] for r in a['rows']]
            reasons = set(oe['reasons'])
            if 'ClosureUncertain' in reasons: reasons.discard('ClosureUncertain')
            if closure(o, ref, rows) == 'claimed_not_established': reasons.add('ClosureNotEstablished')
            if any(cm['kind'] == 'at_least' for cm in oe['commitments']): reasons.add('UnenforcedCommitment')
            if oe['predecessor_obligations']['high']['kind'] == 'at_least': reasons.add('UnresolvedExposure')
            setf(rev, 'OQ-1/OQ-4 12.1 reasons implied by revised facts', oe, 'reasons', sorted(reasons), f"{e['name']}/{name}.reasons")
            setf(rev, '12.1 status', oe, 'status', 'not_established' if oe['reasons'] else 'established', f"{e['name']}/{name}.status")
        ad = ee.get('admission')
        if ad:
            o = objs[e['admission_request']['objective']]; oe = exp_objs[e['admission_request']['objective']]
            cs = closure(o, ref, [r for a in o['admissions'] for r in a['rows']])
            lim = o['limit']; lim_ok = isinstance(lim, str) and re.fullmatch(r'\d+', lim) is not None
            known = sum(int(oe[l]['high']['nano']) for l in ('owned_closed', 'owned_outstanding', 'predecessor_obligations')) + \
                sum(int(cm['committed']) for cm in oe['commitments'] if cm['state'] != 'invalid') + int(e['admission_request']['new'])
            if cs == 'closed': d, rule = 'refused', 'OQ-4 12.4 step 1: established closure refuses'
            elif cs == 'claimed_not_established': d, rule = 'not_established', 'OQ-4 12.4 step 1: a claimed closure without a consistent close entry is not established'
            elif not lim_ok: d, rule = 'not_established', '12.4 step 2: limit not established'
            elif known > int(lim): d, rule = 'refused', 'OQ-2 12.4 step 3: known owned amounts, valid commitments and the request exceed the limit'
            elif oe['status'] == 'not_established': d, rule = 'not_established', '12.4 step 4: known within the limit, unknown exposure may exceed it'
            else: d, rule = 'admitted', '12.4 step 5: headroom'
            setf(rev, rule, ad, 'decision', d, f"{e['name']}/admission.decision")
    pair = c.get('evidence_pair')
    if pair and pair.get('permitted_release'):
        old = dict(pair); pair['permitted_release'] = False
        add(rev, 'OQ-1 A5: a current-producer retirement is not a permitted release (no qualified gate)', 'evidence_pair.permitted_release', True, False)
    c['contract'] = 'pi-accounting-contract/1.3-s1'
    c['revisions_r4'] = rev
    dec = [r for r in rev if r['field'].endswith('admission.decision')]
    if dec:
        c['rationale'] = ((c['rationale'] + ' ') if c.get('rationale') else '') + 'Revision 4 (contract 1.3-s1): ' + '; '.join(
            f'{r["field"].split("/")[0]} {r["from"]} -> {r["to"]} ({r["rule"].split(":")[0]})' for r in dec) + '.'
    if rev: changed += 1
    report.append((c['case_id'], [(r['field'], r['from'], r['to']) for r in dec]))
    json.dump(c, open(path, 'w'), indent=1, sort_keys=True); open(path, 'a').write('\n')
print(f'revision 4 applied; {changed} of {len(report)} cases changed')
for cid, d in report:
    if d: print(' ', cid, d)
