"""Shared derivations for the revision-5 case tools (contract 1.4-s1): attempt-level exposure rows of §8 that revision 5
changed, gate qualification with the G-8 billing bound, closure by profile, and per-evaluation commitments, predecessor
lines, reasons and admission (§12). Used by revise_rev5.py and build_rev5_cases.py only; the validator has its own code."""
import hashlib, json, re
from decimal import Decimal

STAMP = re.compile(r'^\d{4}-\d\d-\d\dT\d\d:\d\d:\d\dZ$')
OPEN = {'pending', 'launched_open', 'retired_resumable', 'reopened'}
DERIVED = {'UnenforcedCommitment', 'ClosureNotEstablished', 'GateContradicted', 'UnresolvedExposure', 'CommitmentInvalid'}
COVERS = ['failures', 'forwarding', 'partial_streams', 'retries', 'settlement']
TIE = {'usage_valuation': 0, 'proxy_reservation': 1, 'proxy_charge': 2}      # X-0 (1.4-s1) tie order


def binding(gate_id, authority_id, session, amount_nano, currency, effective_from, effective_until, covers):
    doc = {'amount_nano': amount_nano, 'authority_id': authority_id, 'covers': covers, 'currency': currency,
           'effective_from': effective_from, 'effective_until': effective_until, 'gate_id': gate_id, 'session': session}
    return hashlib.sha256(json.dumps(doc, sort_keys=True, separators=(',', ':')).encode()).hexdigest()


def high_of(terms):
    """terms: [(amount, basis)] -> (amount, basis) of the maximum with the X-0 tie order."""
    terms = [t for t in terms if t[0] is not None]
    best = max(t[0] for t in terms)
    return best, sorted((t for t in terms if t[0] == best), key=lambda t: TIE[t[1]])[0][1]


def replacement(a):
    s, v, x = a['settlement'], a['valuation'], a['exposure']
    if x.get('pool') == 'codex' or a['obligation']['state'] == 'none': return {'eligible': False, 'reason': 'NoReservation'}
    if not (s['decision'] == 'valid' and s.get('basis') == 'usage'): return {'eligible': False, 'reason': 'NotUsageSettlement'}
    if v.get('state') == 'valued' and v.get('complete'): return {'eligible': True, 'reason': 'UsageBound'}
    return {'eligible': False, 'reason': 'UsageIncomplete' if v.get('state') == 'valued' else 'UsageUnpriceable'}


def retained_exposure(a):
    """§8 (1.4-s1) for rows that keep the reservation with a valid settlement: ineligible usage and error status."""
    s, v = a['settlement'], a['valuation']
    R = int(s['reserved']); ch = int(s.get('charged') or 0)
    vh = int(v['high']) if v.get('state') == 'valued' else (int(v['priced_high']) if v.get('state') == 'unpriceable' and v.get('priced_high') is not None else None)
    terms = [(R, 'proxy_reservation'), (vh, 'usage_valuation')] + ([(ch, 'proxy_charge')] if s.get('basis') == 'usage' else [])
    hi, basis = high_of(terms)
    if v.get('state') == 'valued': lo = int(v['low'])
    elif v.get('state') == 'unpriceable' and v.get('supported_low') is not None: lo = int(v['supported_low'])
    else: lo = 0
    return {'pool': 'pi_api', 'low': {'nano': str(lo), 'basis': 'usage_valuation' if lo > 0 else 'none'}, 'high': {'kind': 'at_least', 'nano': str(hi), 'basis': basis}}


def qualify(c, o, a, purpose, ref, cap_nano=None):
    """CONTRACT 2.3 G-1..G-8. Returns (qualified, reasons, contradicted, ceiling or None)."""
    gid = a.get('gate_id')
    if not gid: return False, ['NoGateEvidence'], False, None
    g = next((x for x in c['inputs'].get('gates', []) if x.get('gate_id') == gid), None)
    keys = ('authority', 'session', 'scope', 'effective_from', 'effective_until', 'writer', 'mechanisms')
    if not isinstance(g, dict) or any(k not in g for k in keys): return False, ['GateMalformed'], False, None
    r = []; auth = o.get('admission_authority') or {}
    if g['writer'] != 'authority': r.append('GateWriterNotAuthority')
    if g['authority'].get('kind') != 'vertical_admission_authority' or g['authority'].get('id') != auth.get('id') or auth.get('kind') != 'vertical_admission_authority': r.append('GateAuthorityMismatch')
    if g['session'] != a['session']: r.append('GateSessionMismatch')
    if g['scope'].get('kind') != 'session': r.append('GateScopeInsufficient')
    launched = [x['at'] for x in a['rows'] if x.get('event') in ('launched', 'objective_admission') and isinstance(x.get('at'), str)]
    if not (launched and g['effective_from'] <= min(launched)) or not (g['effective_until'] is None or g['effective_until'] >= ref): r.append('GatePeriodInsufficient')
    need = ('new_forwarding', 'retries', 'reopening') if purpose == 'release' else ('billing_bound', 'retries')
    m = g['mechanisms']
    if not all(m.get(k, {}).get('enforced') is True for k in need): r.append('GateMechanismMissing')
    if any(m.get(k, {}).get('enforced') is True and not ((m[k].get('receipt') or {}).get('ref') and re.fullmatch(r'[0-9a-f]{64}', str((m[k].get('receipt') or {}).get('sha256')))) for k in need):
        r.append('GateReceiptMissing')
    ceiling = None
    if purpose == 'bound' and m.get('billing_bound', {}).get('enforced') is True:
        b = m['billing_bound'].get('bound')
        if not isinstance(b, dict) or not isinstance(b.get('amount_nano'), str) or not re.fullmatch(r'\d+', b['amount_nano']): r.append('GateBoundMissing')
        else:
            if cap_nano is None or int(b['amount_nano']) > cap_nano: r.append('GateBoundExceedsCap')
            if b.get('currency') != 'USD': r.append('GateBoundCurrency')
            if b.get('scope') != 'session' or sorted(b.get('covers') or []) != COVERS: r.append('GateBoundCoverage')
            if b.get('binding_sha256') != binding(gid, g['authority'].get('id'), g['session'], b['amount_nano'], b.get('currency'), g['effective_from'], g['effective_until'], sorted(b.get('covers') or [])):
                r.append('GateBoundBinding')
            ceiling = int(b['amount_nano'])
    contradicted = False
    sess = next((s for s in c['inputs']['sessions'] if s['session_id'] == a['session']), None)
    if sess and m.get('reopening', {}).get('enforced') is True:
        seen = False
        for x in sess['lifecycle']:
            if x['event'] == 'retire': seen = True
            elif x['event'] == 'deliver' and seen and x.get('at') and x['at'] >= g['effective_from']: contradicted = True
    if contradicted: r.append('GateContradicted')
    return not r, r, contradicted, (ceiling if not r else None)


def closure(c, o, ref):
    ca, ce, cr = o.get('closed_at'), o.get('close'), o.get('closure_record')
    if ca is None and ce is None and cr is None: return 'open'
    rows = [x for a in o['admissions'] for x in a['rows']]
    coh = isinstance(ca, str) and bool(STAMP.match(ca)) and o['started_at'] <= ca <= ref and not any(isinstance(x.get('at'), str) and x['at'] > ca for x in rows)
    if c['profile'] == 'current_producer': ok = coh and isinstance(ce, dict) and ce.get('at') == ca and cr is None
    else:
        auth = o.get('admission_authority') or {}
        ok = coh and isinstance(cr, dict) and cr.get('authority_id') == auth.get('id') and cr.get('at') == ca and (cr.get('receipt') or {}).get('ref') and \
            re.fullmatch(r'[0-9a-f]{64}', str((cr.get('receipt') or {}).get('sha256')))
    return 'closed' if ok else 'claimed'


def finalize(c):
    """Re-derive commitments, predecessor lines, owned lines, reasons, status and admission for every evaluation."""
    ref = c['inputs']['reference_time']; atts = {a['key']: a for a in c['expected']['attempts']}; life = c['expected']['lifecycle']
    def esum(xs):
        if not xs: return {'low': '0', 'high': {'kind': 'exact', 'nano': '0'}}
        return {'low': str(sum(int(x['low']['nano']) for x in xs)), 'high': {'kind': 'at_least' if any(x['high']['kind'] == 'at_least' for x in xs) else 'exact', 'nano': str(sum(int(x['high']['nano']) for x in xs))}}
    for e in c['evaluations']:
        ee = e['expected']; objs = {o['objective']: o for o in e['objectives']}; eo = {o['objective']: o for o in ee['objectives']}
        for name, oe in eo.items():
            owned = [atts[at['attempt']] for at in ee['attribution'] if at['ownership'] == 'owned' and at['objective'] == name and atts[at['attempt']]['exposure'].get('pool') == 'pi_api']
            oe['owned_closed'] = esum([a['exposure'] for a in owned if a['obligation']['state'] in ('closed', 'none')])
            oe['owned_outstanding'] = esum([a['exposure'] for a in owned if a['obligation']['state'] == 'outstanding'])
            o = objs[name]; new = []
            for a in o['admissions']:
                caps = [x for x in a['rows'] if x.get('event') in ('pending', 'objective_admission')]
                cap = caps[0].get('max_usd') if caps else None
                okcap = isinstance(cap, (int, float)) and not isinstance(cap, bool) and cap > 0
                capn = int(Decimal(str(cap)) * 10 ** 9) if okcap else None
                alias = next((s['alias'] for s in c['inputs']['sessions'] if s['session_id'] == a['session']), None)
                spent = sum(int(atts[at['attempt']]['exposure']['high']['nano']) for at in ee['attribution'] if at['ownership'] == 'owned' and at['objective'] == name
                            and atts[at['attempt']]['exposure'].get('pool') == 'pi_api' and at['attempt'].split(':')[1].split('/')[0] == alias)
                launched = any(x.get('event') in ('launched', 'objective_admission') for x in a['rows']); released = any(x.get('event') == 'released' for x in a['rows'])
                gate, contra = {'qualified': False, 'reasons': ['NoGateEvidence']}, False
                if not okcap: st, com, kind = 'invalid', '0', 'exact'
                elif released and not launched: st, com, kind = 'released', '0', 'exact'
                else:
                    l = life.get(alias) or {}
                    if not launched:
                        q, rs, contra, ceil = qualify(c, o, a, 'bound', ref, capn); gate = {'qualified': q, 'reasons': rs}
                        st, com, kind = 'pending', str(ceil if q else capn), 'exact' if q else 'at_least'
                    elif l.get('state') == 'retired':
                        q, rs, contra, _ = qualify(c, o, a, 'release', ref, capn); gate = {'qualified': q, 'reasons': rs}
                        st, com, kind = ('retired', '0', 'exact') if q else ('retired_resumable', str(max(0, capn - spent)), 'at_least')
                    elif l.get('reopened'):
                        q, rs, contra, _ = qualify(c, o, a, 'release', ref, capn); gate = {'qualified': False, 'reasons': rs if not q else ['ReopenedWithoutAdmission']}
                        st, com, kind = 'reopened', str(max(0, capn - spent)), 'at_least'
                    else:
                        q, rs, contra, ceil = qualify(c, o, a, 'bound', ref, capn); gate = {'qualified': q, 'reasons': rs}
                        st, com, kind = 'launched_open', str(max(0, (ceil if q else capn) - spent)), 'exact' if q else 'at_least'
                new.append({'admission_id': a['admission_id'], 'state': st, 'committed': com, 'kind': kind, 'gate': gate, '_contra': contra,
                            '_over': okcap and launched and spent > capn})
            oe['commitments'] = new
        for name, oe in eo.items():
            pred = objs[name].get('predecessor')
            if pred and pred in eo:
                p = eo[pred]
                unres = [atts[at['attempt']]['exposure'] for at in ee['attribution'] if at['ownership'] == 'owned' and at['objective'] == pred and atts[at['attempt']]['exposure'].get('pool') == 'pi_api'
                         and atts[at['attempt']]['exposure']['high']['kind'] == 'at_least']
                com = [cm for cm in p['commitments'] if cm['state'] in OPEN]
                kind = 'at_least' if unres or any(cm['kind'] == 'at_least' for cm in com) else 'exact'
                oe['predecessor_obligations'] = {'low': '0', 'high': {'kind': kind, 'nano': str(sum(int(x['high']['nano']) for x in unres) + sum(int(cm['committed']) for cm in com))}}
        for name, oe in eo.items():
            o = objs[name]; rs = set(oe['reasons']) - DERIVED
            for at in ee['attribution']:
                a = atts[at['attempt']]
                if a['exposure'].get('pool') != 'pi_api': continue
                if (at['ownership'] == 'owned' and at['objective'] == name) or (at['ownership'] in ('uncertain', 'unadmitted') and name in at.get('candidates', [])):
                    rs |= set(a['blocking'])
            if any(oe[l]['high']['kind'] == 'at_least' for l in ('owned_closed', 'owned_outstanding', 'predecessor_obligations')): rs.add('UnresolvedExposure')
            for cm in oe['commitments']:
                over, contra = cm.pop('_over'), cm.pop('_contra')
                if cm['kind'] == 'at_least': rs.add('UnenforcedCommitment')
                if cm['state'] == 'invalid' or over: rs.add('CommitmentInvalid')
                if contra: rs.add('GateContradicted')
            if closure(c, o, ref) == 'claimed': rs.add('ClosureNotEstablished')
            oe['reasons'] = sorted(rs); oe['status'] = 'not_established' if rs else 'established'
        ad = ee.get('admission')
        if ad:
            o = objs[e['admission_request']['objective']]; oe = eo[e['admission_request']['objective']]; cs = closure(c, o, ref)
            known = sum(int(oe[l]['high']['nano']) for l in ('owned_closed', 'owned_outstanding', 'predecessor_obligations')) + \
                sum(int(cm['committed']) for cm in oe['commitments'] if cm['state'] != 'invalid') + int(e['admission_request']['new'])
            ad['limit'] = o['limit']
            ad['decision'] = 'refused' if cs == 'closed' else 'not_established' if cs == 'claimed' else 'not_established' if not re.fullmatch(r'\d+', str(o['limit'])) else \
                'refused' if known > int(o['limit']) else 'not_established' if oe['status'] == 'not_established' else 'admitted'
    return c
