"""Revision 3 of the corpus (contract 1.2-s1, P1.30): applies the repairs of the P1.29 findings to the revision-2 cases.
Every changed field is logged in the case's `revisions_r3` list with the contract rule; the revision-2 `revisions`
list and the historical blocks are never touched. Fields that did not exist before are logged with from "__absent__".
The derivations here follow the contract text (CONTRACT 1.2-s1); the validator re-checks every expectation with its own
code. Usage: revise_rev3.py <corpus_dir> <openai_rates.json>"""
import copy, glob, json, os, re, sys
from fractions import Fraction

corpus, oa_rates = sys.argv[1], json.load(open(sys.argv[2]))
TIERS = set(oa_rates['tier_multiplier'])
ABSENT = '__absent__'
STAMP = re.compile(r'^\d{4}-\d\d-\d\dT\d\d:\d\d:\d\dZ$')
TERMINAL_EVENTS = {'request', 'upstream_error', 'stream_error', 'response_model_refused'}
OPEN_COMMIT = {'pending', 'launched_open', 'reopened'}


def add(rev, rule, field, old, new): rev.append({'rule': rule, 'field': field, 'from': old, 'to': new})


def rows_of(c):
    out = {}
    for s in c['inputs']['sessions']:
        for turn, log in s['logs'].items():
            for n, ln in enumerate(log['lines'], 1):
                try: out[f"{s['alias']}/{turn}:{n}"] = json.loads(ln)
                except ValueError: out[f"{s['alias']}/{turn}:{n}"] = None
    return out


def forwarded_row(row):
    """CONTRACT 1.2-s1 6.1."""
    if not isinstance(row, dict): return False
    if row.get('event') in ('response_model_refused', 'stream_error'): return True
    if row.get('event') != 'request': return False
    st = row.get('http_status')
    if isinstance(st, int) and not isinstance(st, bool): return True
    if row.get('response_id') or row.get('response_model'): return True
    u = row.get('usage')
    return isinstance(u, dict) and any(v is not None for v in u.values())


def esum(items):
    if not items: return {'low': '0', 'high': {'kind': 'exact', 'nano': '0'}}
    return {'low': str(sum(int(x['low']['nano']) for x in items)),
            'high': {'kind': 'at_least' if any(x['high']['kind'] == 'at_least' for x in items) else 'exact', 'nano': str(sum(int(x['high']['nano']) for x in items))}}


def setf(rev, rule, obj, key, new, path):
    old = obj.get(key, ABSENT)
    if old != new: add(rev, rule, path, old, new); obj[key] = new


def lifecycle_facts(sess):
    rows = sess['lifecycle']; seen_retire = False; reopened = False
    for r in rows:
        if r['event'] == 'retire': seen_retire = True
        elif r['event'] == 'deliver' and seen_retire: reopened = True
    return reopened


def closure_state(o, ref):
    ca = o.get('closed_at')
    if ca is None: return 'open'
    if isinstance(ca, str) and STAMP.match(ca) and ca <= ref: return 'closed'
    return 'uncertain'


changed = 0; report = []
for path in sorted(glob.glob(os.path.join(corpus, 'cases', '*.json'))):
    c = json.load(open(path))
    if c.get('contract') != 'pi-accounting-contract/1.1-s1': continue        # idempotent: revision-2 cases only
    rev = []; R = rows_of(c); ref = c['inputs']['reference_time']
    sessions = {s['alias']: s for s in c['inputs']['sessions']}
    # ---------------------------------------------------------------- attempt level
    for a in c['expected']['attempts']:
        k = a['key']; P = f'attempts[{k}]'
        ev_rows = [R.get(r) for r in a['evidence']]
        # R-06 correlation: key join versus auxiliary shape-rule rows
        if a['correlation'].get('kind') == 'producer_shape':
            aux = [{'evidence': r, 'rule': a['correlation']['rule']} for r in a['evidence']
                   if isinstance(R.get(r), dict) and R[r].get('event') in ('stream_error', 'response_model_refused')]
            assert len(aux) == 1, (c['case_id'], k)
            setf(rev, 'R-06 C-key_join: an RS rule joins an auxiliary row; the key join is exact', a, 'correlation', {'kind': 'exact', 'auxiliary': aux}, f'{P}.correlation')
        # R-13 forwarding
        fw = 'forwarded' if any(forwarded_row(x) for x in ev_rows) else 'unknown'
        setf(rev, 'R-13 6.1: every stream_error proves a provider response; refused rows never decide forwarding', a, 'forwarding', fw, f'{P}.forwarding')
        # R-03 tier set
        x, v, s = a['exposure'], a['valuation'], a['settlement']
        if x.get('pool') == 'pi_api':
            oa_req = [r for r in ev_rows if isinstance(r, dict) and r.get('event') == 'request' and r.get('provider', 'openai') != 'anthropic' and r.get('usage')]
            if v.get('state') == 'valued' and any(d['provider'] == 'openai' for d in v['derivations']) and oa_req \
                    and not all(isinstance(r.get('observed_tier'), str) and r['observed_tier'] in TIERS for r in oa_req):
                ph = max(int(d['nano']) for d in v['derivations'] if d['for'] in ('high', 'both'))
                setf(rev, 'R-03 V-3 (Q5, P1.30): a missing or unrecognised OpenAI tier leaves the tier set unestablished', a, 'valuation',
                     {'state': 'unpriceable', 'reasons': ['TierUnestablished'], 'supported_low': None, 'priced_high': str(ph)}, f'{P}.valuation')
                v = a['valuation']
            if v.get('state') == 'unpriceable' and v.get('reasons') == ['UnknownTier']:
                setf(rev, 'R-03 V-3: one reason TierUnestablished(Missing | Unrecognised)', v, 'reasons', ['TierUnestablished'], f'{P}.valuation.reasons')
            # exposure for the rows whose rule changed in revision 3
            ch = int(s['charged']) if 'charged' in s else None; Rr = int(s['reserved']) if 'reserved' in s else None
            vh = int(v['high']) if v.get('state') == 'valued' else (int(v['priced_high']) if v.get('state') == 'unpriceable' and v.get('priced_high') is not None else None)
            new = None
            if s['decision'] == 'valid' and s.get('basis') == 'usage' and v.get('state') == 'unpriceable':
                hi = max(ch, vh or 0); new = {'low': {'nano': '0', 'basis': 'none'}, 'high': {'kind': 'at_least', 'nano': str(hi), 'basis': 'usage_valuation' if (vh or -1) >= ch else 'proxy_charge'}}
                rule = 'R-03/R-06 8: Valid(Usage) with an Unpriceable valuation is AtLeast(max(charged, priced_high)); tie basis UsageValuation (X-0)'
            elif s['decision'] == 'valid' and s.get('basis') == 'usage' and v.get('state') == 'valued' and not v.get('complete'):
                new = {'low': x['low'], 'high': {**x['high'], 'kind': 'at_least'}}; rule = 'R-01 8: Valid(Usage) with incomplete usage has no usage bound'
            elif s['decision'] in ('rejected', 'conflicting') and Rr is not None and vh is not None:
                hi = max(Rr, ch or 0, vh)
                if hi != int(x['high']['nano']):
                    new = {'low': x['low'], 'high': {'kind': 'at_least', 'nano': str(hi), 'basis': 'usage_valuation' if hi == vh else x['high']['basis']}}
                    rule = 'R-05 8: the rejected/conflicting maximum includes every usage observation'
            if new is not None:
                for part in ('low', 'high'):
                    if x[part] != new[part]: add(rev, rule, f'{P}.exposure.{part}', x[part], new[part]); x[part] = new[part]
                if x['high']['kind'] == 'at_least' and 'UnresolvedExposure' not in a['blocking']:
                    setf(rev, 'K: AtLeast => UnresolvedExposure', a, 'blocking', sorted(a['blocking'] + ['UnresolvedExposure']), f'{P}.blocking')
        # R-07 execution and billing facts
        setf(rev, 'R-07 6.2 Execution', a, 'execution', 'terminal' if any(isinstance(r, dict) and r.get('event') in TERMINAL_EVENTS for r in ev_rows) else 'not_terminal', f'{P}.execution')
        setf(rev, 'R-07 6.2 Billing', a, 'billing', 'not_applicable' if x.get('pool') == 'codex' else ('established' if x['high']['kind'] == 'exact' else 'unresolved'), f'{P}.billing')
    atts = {a['key']: a for a in c['expected']['attempts']}
    # ---------------------------------------------------------------- lifecycle (R-10)
    life = c['expected']['lifecycle']
    for alias, l in life.items():
        ro = lifecycle_facts(sessions[alias]) if alias in sessions else False   # S08 pair: lifecycle entry without an input session (revision-1 oddity)
        setf(rev, 'R-10 10: reopened is a separate fact from the terminal state', l, 'reopened', ro, f'lifecycle[{alias}].reopened')
        if l['state'] == 'reopened':
            last = sessions[alias]['lifecycle'][-1]
            setf(rev, 'R-10 L-4: the state is the terminal state from the last row', l, 'state', 'retired' if last['event'] == 'retire' and last.get('at') else 'open', f'lifecycle[{alias}].state')
    # ---------------------------------------------------------------- per evaluation
    for e in c['evaluations']:
        ee = e['expected']; objs = {o['objective']: o for o in e['objectives']}; exp_objs = {o['objective']: o for o in ee['objectives']}
        for name, oe in exp_objs.items():
            owned = [atts[at['attempt']] for at in ee['attribution'] if at['ownership'] == 'owned' and at['objective'] == name and atts[at['attempt']]['exposure'].get('pool') == 'pi_api']
            for line, sel in (('owned_closed', lambda a: a['obligation']['state'] in ('closed', 'none')), ('owned_outstanding', lambda a: a['obligation']['state'] == 'outstanding')):
                setf(rev, '12.1 rollup of revised attempts', oe, line, esum([a['exposure'] for a in owned if sel(a)]), f"{e['name']}/{name}.{line}")
            for i, cm in enumerate(oe['commitments']):
                kind = 'at_least' if cm['state'] in OPEN_COMMIT else 'exact'
                if 'kind' not in cm or cm['kind'] != kind:
                    add(rev, 'R-04 12.3 A-5 (Q10, P1.30): open and pending commitments rest on an unenforced declared cap', f"{e['name']}/{name}.commitments[{cm['admission_id']}].kind", cm.get('kind', ABSENT), kind)
                    cm['kind'] = kind
        for name, oe in exp_objs.items():                 # predecessor lines (R-07) after every objective's own lines
            pred = objs[name].get('predecessor')
            if pred and pred in exp_objs:
                p = exp_objs[pred]
                unresolved = [atts[at['attempt']]['exposure'] for at in ee['attribution'] if at['ownership'] == 'owned' and at['objective'] == pred
                              and atts[at['attempt']]['exposure'].get('pool') == 'pi_api' and atts[at['attempt']]['billing'] == 'unresolved']
                com = [cm for cm in p['commitments'] if cm['state'] in OPEN_COMMIT]
                kind = 'at_least' if any(x['high']['kind'] == 'at_least' for x in unresolved) or any(cm['kind'] == 'at_least' for cm in com) else 'exact'
                new = {'low': '0', 'high': {'kind': kind, 'nano': str(sum(int(x['high']['nano']) for x in unresolved) + sum(int(cm['committed']) for cm in com))}}
                setf(rev, 'R-07 12.2: the predecessor line selects by billing, not by obligation', oe, 'predecessor_obligations', new, f"{e['name']}/{name}.predecessor_obligations")
        for name, oe in exp_objs.items():
            reasons = set(oe['reasons'])
            if any(cm['kind'] == 'at_least' for cm in oe['commitments']): reasons.add('UnenforcedCommitment')
            if closure_state(objs[name], ref) == 'uncertain': reasons.add('ClosureUncertain')
            for at in ee['attribution']:
                a = atts[at['attempt']]
                if a['exposure'].get('pool') != 'pi_api': continue
                if (at['ownership'] == 'owned' and at['objective'] == name) or (at['ownership'] in ('uncertain', 'unadmitted') and name in at.get('candidates', [])):
                    reasons |= set(a['blocking'])
            if any(oe[l]['high']['kind'] == 'at_least' for l in ('owned_closed', 'owned_outstanding', 'predecessor_obligations')): reasons.add('UnresolvedExposure')
            setf(rev, '12.1 reasons implied by revised facts', oe, 'reasons', sorted(reasons), f"{e['name']}/{name}.reasons")
            setf(rev, '12.1 status', oe, 'status', 'not_established' if oe['reasons'] else 'established', f"{e['name']}/{name}.status")
        ad = ee.get('admission')
        if ad:
            o = objs[e['admission_request']['objective']]; oe = exp_objs[e['admission_request']['objective']]; cs = closure_state(o, ref)
            if cs == 'closed': d = 'refused'; rule = 'Q11 12.4 step 1 (P1.30): authoritative closure refuses before headroom'
            elif cs == 'uncertain' or oe['status'] == 'not_established': d = 'not_established'; rule = '12.4 steps 1-2: not established'
            else:
                tot = sum(int(oe[l]['high']['nano']) for l in ('owned_closed', 'owned_outstanding', 'predecessor_obligations')) + sum(int(cm['committed']) for cm in oe['commitments']) + int(e['admission_request']['new'])
                d = 'admitted' if tot <= int(o['limit']) else 'refused'; rule = '12.4 step 3: headroom'
            setf(rev, rule, ad, 'decision', d, f"{e['name']}/admission.decision")
    c['contract'] = 'pi-accounting-contract/1.2-s1'
    c['revisions_r3'] = rev
    dec = [r for r in rev if r['field'].endswith('admission.decision')]
    if dec:
        rules = sorted({r['rule'].split(' ')[0] for r in rev if not r['rule'][0].isdigit() and r['rule'].split(' ')[0] not in ('K:',)})
        c['rationale'] = ((c['rationale'] + ' ') if c.get('rationale') else '') + 'Revision 3 (contract 1.2-s1): ' + '; '.join(
            f'{r["field"].split("/")[0]} {r["from"]} -> {r["to"]}' for r in dec) + ' under ' + ', '.join(rules) + '.'
    if rev: changed += 1
    report.append((c['case_id'], len(rev), [(r['field'], r['from'], r['to']) for r in dec]))
    json.dump(c, open(path, 'w'), indent=1, sort_keys=True); open(path, 'a').write('\n')
print(f'revision 3 applied; {changed} of {len(report)} cases changed')
for cid, n, d in report:
    if d: print(' ', cid, d)
