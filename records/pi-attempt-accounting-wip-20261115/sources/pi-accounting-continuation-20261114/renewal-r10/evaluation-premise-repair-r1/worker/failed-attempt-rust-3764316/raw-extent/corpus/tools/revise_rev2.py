"""Revision 2 of the corpus (contract 1.1-s1): applies owner dispositions Q2 (FB-2), Q6 (forwarding fact) and Q8
(O-6/O-7 unadmitted ownership) to the revision-1 cases written by build_corpus.py. Every changed field is logged in the
case's `revisions` list with the contract rule that changes it; nothing else is touched. Objective lines, reasons and
admission decisions that follow from a changed attempt are recomputed by the contract's rollup rules (§12.1) and logged;
the validator re-checks them independently. Q5 changes no revision-1 case (U-08's missing tier is unchanged); its new
case is written by build_rev2_cases.py. Usage: revise_rev2.py <corpus_dir>"""
import glob, json, os, sys

corpus = sys.argv[1]
RESERVATION_ONLY = {('valid', 'reservation'), ('valid', 'upstream_error'), ('valid', 'error_status'), ('absent', None), ('rejected', None), ('conflicting', None)}


def forwarded_row(row):
    """Contract 6.1: a provider response exists."""
    if not isinstance(row, dict): return False
    if row.get('event') == 'response_model_refused': return True
    if row.get('event') != 'request': return False
    st = row.get('http_status')
    if isinstance(st, int) and not isinstance(st, bool): return True
    if row.get('response_id') or row.get('response_model'): return True
    u = row.get('usage')
    return isinstance(u, dict) and any(v is not None for v in u.values())


def forwarding(case, att):
    """Q6 rule 6.1 over the attempt's own evidence rows: Forwarded or Unknown (a NotForwarded request is not an attempt)."""
    lines = {}
    for s in case['inputs']['sessions']:
        for turn, log in s['logs'].items():
            for n, ln in enumerate(log['lines'], 1): lines[f"{s['alias']}/{turn}:{n}"] = ln
    for ref in att['evidence']:
        try: row = json.loads(lines[ref])
        except (ValueError, KeyError): continue
        if forwarded_row(row): return 'forwarded'
    return 'unknown'


def add(rev, rule, field, old, new): rev.append({'rule': rule, 'field': field, 'from': old, 'to': new})


def esum(items):
    if not items: return {'low': '0', 'high': {'kind': 'exact', 'nano': '0'}}
    return {'low': str(sum(int(x['low']['nano']) for x in items)),
            'high': {'kind': 'at_least' if any(x['high']['kind'] == 'at_least' for x in items) else 'exact', 'nano': str(sum(int(x['high']['nano']) for x in items))}}


changed = 0
for path in sorted(glob.glob(os.path.join(corpus, 'cases', '*.json'))):
    c = json.load(open(path))
    if c.get('contract') != 'pi-accounting-contract/1.0-s1': continue   # idempotent: revision-1 cases only
    rev = []
    atts = {a['key']: a for a in c['expected']['attempts']}
    for a in c['expected']['attempts']:
        a['forwarding'] = forwarding(c, a)          # new field (Q6); recorded once, not a change of an existing expectation
        x, s = a['exposure'], a['settlement']
        if x.get('pool') != 'pi_api': continue
        key = (s['decision'], s.get('basis') if s['decision'] == 'valid' else None)
        if key in RESERVATION_ONLY and x['high']['kind'] == 'exact':
            add(rev, 'FB-2 (Q2): reservation or proxy-charge support is never a finite bound', f"attempts[{a['key']}].exposure.high.kind", 'exact', 'at_least')
            x['high']['kind'] = 'at_least'
            if 'UnresolvedExposure' not in a['blocking']:
                add(rev, 'K: AtLeast => UnresolvedExposure', f"attempts[{a['key']}].blocking", list(a['blocking']), sorted(a['blocking'] + ['UnresolvedExposure']))
                a['blocking'] = sorted(a['blocking'] + ['UnresolvedExposure'])
    for e in c['evaluations']:
        ee = e['expected']; objs = {o['objective']: o for o in e['objectives']}
        for at in ee['attribution']:
            if at['ownership'] == 'uncertain' and at.get('cause') == 'NoAdmission':
                add(rev, 'O-6 (Q8): no admission record => Unadmitted', f"{e['name']}/attribution[{at['attempt']}].ownership", 'uncertain', 'unadmitted')
                add(rev, 'O-6 (Q8): the NoAdmission cause is now the Unadmitted state itself', f"{e['name']}/attribution[{at['attempt']}].cause", at['cause'], None)
                at['ownership'] = 'unadmitted'; at.pop('cause')
                if at.get('objective') is not None:
                    add(rev, 'O-6 (Q8): an unadmitted attempt has no owner', f"{e['name']}/attribution[{at['attempt']}].objective", at['objective'], None)
                    at['objective'] = None
        exp_objs = {o['objective']: o for o in ee['objectives']}
        for name, oe in exp_objs.items():
            owned = [atts[at['attempt']] for at in ee['attribution'] if at['ownership'] == 'owned' and at['objective'] == name and atts[at['attempt']]['exposure'].get('pool') == 'pi_api']
            for line, sel in (('owned_closed', lambda a: a['obligation']['state'] in ('closed', 'none')), ('owned_outstanding', lambda a: a['obligation']['state'] == 'outstanding')):
                new = esum([a['exposure'] for a in owned if sel(a)])
                if new != oe[line]: add(rev, '12.1 rollup of revised attempts', f"{e['name']}/{name}.{line}", oe[line], new); oe[line] = new
            reasons = set(oe['reasons'])
            def names(own): return any(at['ownership'] == own and name in at.get('candidates', []) and atts[at['attempt']]['exposure'].get('pool') == 'pi_api' for at in ee['attribution'])
            if not names('uncertain'): reasons.discard('OwnershipUncertain')
            if names('unadmitted'): reasons.add('HistoricalUnadmitted')
            for a in owned:
                reasons |= set(a['blocking'])
            for at in ee['attribution']:
                if at['ownership'] in ('uncertain', 'unadmitted') and name in at.get('candidates', []) and atts[at['attempt']]['exposure'].get('pool') == 'pi_api':
                    reasons |= set(atts[at['attempt']]['blocking'])
            if [l for l in ('owned_closed', 'owned_outstanding', 'predecessor_obligations') if oe[l]['high']['kind'] == 'at_least']: reasons.add('UnresolvedExposure')
            if sorted(reasons) != oe['reasons']:
                add(rev, '12.1 reasons implied by revised facts', f"{e['name']}/{name}.reasons", oe['reasons'], sorted(reasons)); oe['reasons'] = sorted(reasons)
            st = 'not_established' if oe['reasons'] else 'established'
            if st != oe['status']: add(rev, '12.1 status', f"{e['name']}/{name}.status", oe['status'], st); oe['status'] = st
        for name, oe in exp_objs.items():           # predecessor lines after the predecessor itself is revised
            pred = objs[name].get('predecessor')
            if pred and pred in exp_objs:
                p = exp_objs[pred]
                com = sum(int(cm['committed']) for cm in p['commitments'] if cm['state'] in ('pending', 'launched_open', 'reopened'))
                new = {'low': '0', 'high': {'kind': p['owned_outstanding']['high']['kind'], 'nano': str(int(p['owned_outstanding']['high']['nano']) + com)}}
                if new != oe['predecessor_obligations']:
                    add(rev, 'D1 12.2 predecessor line', f"{e['name']}/{name}.predecessor_obligations", oe['predecessor_obligations'], new); oe['predecessor_obligations'] = new
                if new['high']['kind'] == 'at_least' and 'UnresolvedExposure' not in oe['reasons']:
                    add(rev, '12.1 reasons implied by revised facts', f"{e['name']}/{name}.reasons", oe['reasons'], sorted(oe['reasons'] + ['UnresolvedExposure']))
                    oe['reasons'] = sorted(oe['reasons'] + ['UnresolvedExposure'])
                    if oe['status'] != 'not_established': add(rev, '12.1 status', f"{e['name']}/{name}.status", oe['status'], 'not_established'); oe['status'] = 'not_established'
        ad = ee.get('admission')
        if ad:
            oe = exp_objs[e['admission_request']['objective']]
            if oe['status'] == 'not_established' and ad['decision'] != 'not_established':
                add(rev, '12 admission: not established', f"{e['name']}/admission.decision", ad['decision'], 'not_established'); ad['decision'] = 'not_established'
    c['contract'] = 'pi-accounting-contract/1.1-s1'
    c['revisions'] = rev
    if rev:
        changed += 1
        if any(r['field'].endswith('admission.decision') for r in rev):
            rules = sorted({r['rule'].split(':')[0] for r in rev if not r['rule'][0].isdigit()})
            c['rationale'] = ((c['rationale'] + ' ') if c.get('rationale') else '') + ('Revision 2 (contract 1.1-s1): the admission becomes not established by '
                              + ', '.join(rules) + '. A1.3.4 counts a reservation as a finite amount; the contract keeps the reserved amount as a known '
                              'amount with unknown additional billing (FB-2), and D5 does not admit an AtLeast exposure.')
    json.dump(c, open(path, 'w'), indent=1, sort_keys=True); open(path, 'a').write('\n')
print(f'revision 2 applied; {changed} cases changed')
