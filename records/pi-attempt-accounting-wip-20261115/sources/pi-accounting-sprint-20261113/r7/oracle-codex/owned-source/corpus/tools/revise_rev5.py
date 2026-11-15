"""Revision 5 of the corpus (contract 1.4-s1, P1.33): applies the Module A repairs to the revision-4 cases. Every changed
field is logged in `revisions_r5` with its rule; inputs, historical blocks and earlier logs are untouched.
Usage: revise_rev5.py <corpus_dir>"""
import copy, glob, json, os, sys
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import rev5_lib as L

corpus = sys.argv[1]
ABSENT = '__absent__'


def add(rev, rule, field, old, new): rev.append({'rule': rule, 'field': field, 'from': old, 'to': new})
def setf(rev, rule, obj, key, new, path):
    old = obj.get(key, ABSENT)
    if old != new: add(rev, rule, path, old, new); obj[key] = copy.deepcopy(new)


def rows_of(c):
    out = {}
    for s in c['inputs']['sessions']:
        for t, log in s['logs'].items():
            for n, ln in enumerate(log['lines'], 1):
                try: out[f"{s['alias']}/{t}:{n}"] = json.loads(ln)
                except ValueError: out[f"{s['alias']}/{t}:{n}"] = None
    return out


changed = 0; report = []
for path in sorted(glob.glob(os.path.join(corpus, 'cases', '*.json'))):
    c = json.load(open(path))
    if c.get('contract') != 'pi-accounting-contract/1.3-s1': continue           # idempotent
    rev = []; R = rows_of(c); cid = c['case_id']
    before = copy.deepcopy(c['evaluations'])
    # ---- targeted corrections (A-06, A-07, A-03)
    if cid == 'A06-reservation-usage-null-status-no-id':
        a = c['expected']['attempts'][0]
        setf(rev, 'A-06 5.7: a null status on a request row is not producer-supported (always an integer since 385f3174; 1,588/1,588 real rows)', a, 'settlement',
             {**a['settlement'], 'decision': 'rejected', 'reasons': ['StatusType'], 'basis': ABSENT} if False else {'decision': 'rejected', 'reasons': ['StatusType'], 'reserved': a['settlement']['reserved'], 'charged': a['settlement']['charged']},
             f'attempts[{a["key"]}].settlement')
        setf(rev, 'A-06 6: a rejected settlement leaves the obligation outstanding', a, 'obligation', {'state': 'outstanding', 'amount_source': 'reservation_bound'}, f'attempts[{a["key"]}].obligation')
        setf(rev, 'A-06 8: rejected row: low 0, high AtLeast(max(R, charged, usage high)), tie basis ProxyReservation', a, 'exposure',
             {'pool': 'pi_api', 'low': {'nano': '0', 'basis': 'none'}, 'high': {'kind': 'at_least', 'nano': '250000000', 'basis': 'proxy_reservation'}}, f'attempts[{a["key"]}].exposure')
        setf(rev, 'A-06 9: rejected => SettlementRejected', a, 'blocking', ['SettlementRejected', 'UnresolvedExposure'], f'attempts[{a["key"]}].blocking')
        setf(rev, 'A-06: the shape is not producer-written; the case is a synthetic test of the status rule', c, 'class', 'adversarial_synthetic', 'class')
        setf(rev, 'A-06: not producer-reachable under any pi_container revision', c, 'reachability', 'producer_unreachable', 'reachability')
    if cid == 'S06-string-http-status':
        a = c['expected']['attempts'][0]; row = R[a['evidence'][1]]; u = row['usage']
        val = 1000 * 10000 + 100 * 50000
        assert (u['input_tokens'], u['output_tokens'], u['cached_tokens'], u['cache_write_tokens']) == (1000, 100, 0, 0), u
        setf(rev, 'A-06 V: valuation is computed from usage whatever the decision; a malformed status makes it incomplete (V-5), not absent', a, 'valuation',
             {'state': 'valued', 'complete': False, 'identities': ['gpt-6-astra'], 'low': str(val), 'high': str(val),
              'derivations': [{'for': 'both', 'model': 'gpt-6-astra', 'nano': str(val), 'provider': 'openai', 'tier_multiplier': '1',
                               'usage': {'input_tokens': 1000, 'output_tokens': 100, 'cached_tokens': 0, 'cache_write_tokens': 0}}]}, f'attempts[{a["key"]}].valuation')
    if cid == 'V01-usage-settlement-other-response-model':
        setf(rev, 'A-07: release 92a03789 refuses a mismatched Anthropic response model before settlement (pi_container.py:625-639, 1260-1266)', c, 'class', 'adversarial_synthetic', 'class')
        setf(rev, 'A-07: not producer-reachable under release 92a03789', c, 'reachability', 'producer_unreachable', 'reachability')
    if cid == 'P132-FG-VC14-open-session-billing-bound':
        setf(rev, 'A-03 G-8: the gate carries no numerical enforced bound, so it does not qualify; the case becomes a negative control', c, 'role', 'finding', 'role')
        setf(rev, 'A-03: negative control for an unbound billing-bound gate', c, 'findings', ['A-03'], 'findings')
        setf(rev, 'A-03: control is the authenticated-bound twin', c, 'controls', ['P133-FG-VC14-authenticated-bound'], 'controls')
    # ---- attempt level (A-01, A-02, A-08, A-09)
    for a in c['expected']['attempts']:
        P = f'attempts[{a["key"]}]'; s = a['settlement']
        res = [R.get(r) for r in a['evidence'] if isinstance(R.get(r), dict) and R[r].get('event') == 'usd_reserved']
        amounts = sorted({int(round(r['reserved_usd'] * 10 ** 9)) for r in res if isinstance(r.get('reserved_usd'), (int, float)) and not isinstance(r.get('reserved_usd'), bool)})
        if len({json.dumps(r, sort_keys=True) for r in res}) > 1 and a['exposure'].get('pool') == 'pi_api':
            Rmax = max(amounts); x = a['exposure']
            if s['decision'] == 'absent':
                setf(rev, 'A-09 X-0: conflicting reservations: R is the largest, never their sum', x, 'high', {'kind': 'at_least', 'nano': str(Rmax), 'basis': 'proxy_reservation'}, P + '.exposure.high')
        rp = L.replacement(a)
        setf(rev, 'A-01 2.1: replacement eligibility is an explicit fact', a, 'replacement', rp, P + '.replacement')
        if s['decision'] == 'valid' and s.get('basis') == 'usage' and not rp['eligible']:
            setf(rev, 'A-01 6/8: an ineligible usage settlement keeps the reservation (ReservationBound)', a, 'obligation', {'state': 'closed', 'amount_source': 'reservation_bound'}, P + '.obligation')
            setf(rev, 'A-01 8: ineligible usage: low = supported usage, high = AtLeast(max(R, charged, usage high or priced high))', a, 'exposure', L.retained_exposure(a), P + '.exposure')
        if s['decision'] == 'valid' and s.get('basis') == 'error_status' and a['valuation'].get('state') in ('valued', 'unpriceable'):
            setf(rev, 'A-02 8: error status keeps supported usage', a, 'exposure', L.retained_exposure(a), P + '.exposure')
        if a['exposure'].get('pool') == 'pi_api':
            setf(rev, 'R-07 6.2 billing follows the high kind', a, 'billing', 'established' if a['exposure']['high']['kind'] == 'exact' else 'unresolved', P + '.billing')
    # ---- evaluation level
    L.finalize(c)
    for e, b in zip(c['evaluations'], before):
        eo = {o['objective']: o for o in e['expected']['objectives']}; bo = {o['objective']: o for o in b['expected']['objectives']}
        for name, o in eo.items():
            for k in ('owned_closed', 'owned_outstanding', 'predecessor_obligations', 'reasons', 'status'):
                if o[k] != bo[name][k]: add(rev, '12 rollup re-derived from the revised attempts and commitments', f"{e['name']}/{name}.{k}", bo[name][k], o[k])
            bc = {cm['admission_id']: cm for cm in bo[name]['commitments']}
            for cm in o['commitments']:
                for k in ('state', 'committed', 'kind', 'gate'):
                    if cm.get(k) != bc.get(cm['admission_id'], {}).get(k):
                        add(rev, ('A-03 12.3 G-8: commitment re-derived under the numerical bound rule' if 'P132-FG' in cid and k in ('kind', 'gate') else '12.3 commitment re-derived from revised owned spend'), f"{e['name']}/{name}.commitments[{cm['admission_id']}].{k}", bc.get(cm['admission_id'], {}).get(k, ABSENT), cm.get(k))
        if e['expected'].get('admission') and e['expected']['admission']['decision'] != b['expected']['admission']['decision']:
            add(rev, '12.4 admission re-derived', f"{e['name']}/admission.decision", b['expected']['admission']['decision'], e['expected']['admission']['decision'])
    c['contract'] = 'pi-accounting-contract/1.4-s1'
    c['revisions_r5'] = rev
    dec = [r for r in rev if r['field'].endswith('admission.decision')]
    if dec:
        c['rationale'] = ((c['rationale'] + ' ') if c.get('rationale') else '') + 'Revision 5 (contract 1.4-s1): ' + '; '.join(
            f'{r["field"].split("/")[0]} {r["from"]} -> {r["to"]}' for r in dec) + ' (' + ', '.join(sorted({x['rule'].split(' ')[0] for x in rev if x['rule'][0] == 'A'})) + ').'
    if rev: changed += 1
    report.append((cid, len(rev), sorted({r['rule'].split(' ')[0] for r in rev}), [(r['field'], r['from'], r['to']) for r in dec]))
    json.dump(c, open(path, 'w'), indent=1, sort_keys=True); open(path, 'a').write('\n')
print(f'revision 5 applied; {changed} of {len(report)} cases changed')
for cid, n, rules, d in report:
    if [x for x in rules if x not in ('A-01',)] or d: print(' ', cid, n, rules, d)
