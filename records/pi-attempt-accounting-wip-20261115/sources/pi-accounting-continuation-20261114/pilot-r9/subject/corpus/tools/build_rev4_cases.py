"""Cases added in revision 4 (contract 1.3-s1, P1.32).
- future_qualified_gate: synthetic normative cases for the unfinished Rust vertical, with explicit ExecutionGate and
  authority-closure evidence (CONTRACT 2.3, 12.4). They are producer_unreachable, class future_vertical, and carry no
  A1.3.4 comparison. Positives are gated twins of current-producer metered controls; negatives break one gate rule each.
- current_producer: closure-entry, over-limit and resumed-session cases that today's producer can write.
Attempt facts are copied unchanged from the source case; commitments, reasons and admission are re-derived here from the
contract text (finalize). Usage: build_rev4_cases.py <corpus_dir> <out_cases_dir>"""
import copy, hashlib, json, os, re, sys
from decimal import Decimal

corpus, out = sys.argv[1], sys.argv[2]
load = lambda cid: json.load(open(os.path.join(corpus, 'cases', cid + '.json')))
AUTH = {'id': 'wev-admission-authority-1', 'kind': 'vertical_admission_authority'}
STAMP = re.compile(r'^\d{4}-\d\d-\d\dT\d\d:\d\d:\d\dZ$')
OPEN = {'pending', 'launched_open', 'retired_resumable', 'reopened'}
DERIVED = {'UnenforcedCommitment', 'ClosureNotEstablished', 'GateContradicted', 'UnresolvedExposure', 'CommitmentInvalid'}
written = []


def h(x): return hashlib.sha256(x.encode()).hexdigest()
def receipt(name): return {'ref': f'authority-receipt:{name}', 'sha256': h(name)}
def enforced(name): return {'enforced': True, 'receipt': receipt(name)}


def gate_for(sid, alias, frm, mechs=('new_forwarding', 'retries', 'reopening', 'billing_bound'), **over):
    g = {'gate_id': f'g-{alias}', 'authority': dict(AUTH), 'session': sid, 'scope': {'kind': 'session'}, 'effective_from': frm, 'effective_until': None,
         'writer': 'authority', 'mechanisms': {m: (enforced(f'{alias}-{m}') if m in mechs else {'enforced': False}) for m in ('new_forwarding', 'retries', 'reopening', 'billing_bound')}}
    g.update(over); return g


def meta(c, cid, *, profile, role, cls, reach, findings, controls, invariants, origin, notes, group='P1.32'):
    c.update(case_id=cid, group=group, role=role, findings=findings, controls=controls, invariants=invariants, provenance={'origin': origin}, reachability=reach,
             notes=notes, rationale=None, ambiguity=None, revisions=[], revisions_r3=[], revisions_r4=[], added_in='1.3-s1', contract='pi-accounting-contract/1.3-s1',
             profile=profile, historical={} if profile == 'future_qualified_gate' else c.get('historical', {}))
    c['class'] = cls
    for k in ('twin_of', 'evidence_pair'): c.pop(k, None)
    return c


def future(src, cid, *, role='valid_control', findings=(), controls=(), invariants=('INV-A9', 'INV-A8'), notes='', gate_over=None, mechs=None, no_gate=False):
    c = copy.deepcopy(load(src))
    meta(c, cid, profile='future_qualified_gate', role=role, cls='future_vertical', reach='producer_unreachable', findings=list(findings), controls=list(controls),
         invariants=list(invariants), origin=f'contract::2.3 ExecutionGate (future vertical twin of {src})', notes=notes or f'Synthetic future-vertical twin of {src} with qualified gate evidence.')
    c['inputs']['gates'] = []
    for e in c['evaluations']:
        for o in e['objectives']:
            o['admission_authority'] = dict(AUTH)
            for a in o['admissions']:
                alias = next((s['alias'] for s in c['inputs']['sessions'] if s['session_id'] == a['session']), None)
                if alias is None or no_gate: continue
                frm = min(r['at'] for r in a['rows'] if isinstance(r.get('at'), str))
                if not any(g['gate_id'] == f'g-{alias}' for g in c['inputs']['gates']):
                    g = gate_for(a['session'], alias, frm, **({'mechs': mechs} if mechs else {}))
                    if gate_over: gate_over(g)
                    c['inputs']['gates'].append(g)
                a['gate_id'] = f'g-{alias}'
    return c


def qualify(c, o, a, purpose, ref):
    gid = a.get('gate_id')
    if not gid: return False, ['NoGateEvidence'], False
    g = next((x for x in c['inputs'].get('gates', []) if x.get('gate_id') == gid), None)
    keys = ('authority', 'session', 'scope', 'effective_from', 'effective_until', 'writer', 'mechanisms')
    if not isinstance(g, dict) or any(k not in g for k in keys): return False, ['GateMalformed'], False
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
    if any(m.get(k, {}).get('enforced') is True and not (m[k].get('receipt') or {}).get('ref') for k in need): r.append('GateReceiptMissing')
    contradicted = False
    sess = next((s for s in c['inputs']['sessions'] if s['session_id'] == a['session']), None)
    if sess and m.get('reopening', {}).get('enforced') is True:
        seen = False
        for x in sess['lifecycle']:
            if x['event'] == 'retire': seen = True
            elif x['event'] == 'deliver' and seen and x.get('at') and x['at'] >= g['effective_from']: contradicted = True
    if contradicted: r.append('GateContradicted')
    return not r, r, contradicted


def closure(c, o, ref):
    ca, ce, cr = o.get('closed_at'), o.get('close'), o.get('closure_record')
    if ca is None and ce is None and cr is None: return 'open'
    rows = [x for a in o['admissions'] for x in a['rows']]
    coh = isinstance(ca, str) and bool(STAMP.match(ca)) and o['started_at'] <= ca <= ref and not any(isinstance(x.get('at'), str) and x['at'] > ca for x in rows)
    if c['profile'] == 'current_producer': ok = coh and isinstance(ce, dict) and ce.get('at') == ca and cr is None
    else:
        auth = o.get('admission_authority') or {}
        ok = coh and isinstance(cr, dict) and cr.get('authority_id') == auth.get('id') and cr.get('at') == ca and (cr.get('receipt') or {}).get('ref')
    return 'closed' if ok else 'claimed'


def finalize(c):
    """Re-derive commitments, predecessor lines, reasons, status and admission for every evaluation (CONTRACT 1.3-s1)."""
    ref = c['inputs']['reference_time']; atts = {a['key']: a for a in c['expected']['attempts']}; life = c['expected']['lifecycle']
    for e in c['evaluations']:
        ee = e['expected']; objs = {o['objective']: o for o in e['objectives']}; eo = {o['objective']: o for o in ee['objectives']}
        for name, oe in eo.items():
            o = objs[name]; new = []
            for a in o['admissions']:
                caps = [x for x in a['rows'] if x.get('event') in ('pending', 'objective_admission')]
                cap = caps[0].get('max_usd') if caps else None
                okcap = isinstance(cap, (int, float)) and not isinstance(cap, bool) and cap > 0
                alias = next((s['alias'] for s in c['inputs']['sessions'] if s['session_id'] == a['session']), None)
                spent = sum(int(atts[at['attempt']]['exposure']['high']['nano']) for at in ee['attribution'] if at['ownership'] == 'owned' and at['objective'] == name
                            and atts[at['attempt']]['exposure'].get('pool') == 'pi_api' and at['attempt'].split(':')[1].split('/')[0] == alias)
                launched = any(x.get('event') in ('launched', 'objective_admission') for x in a['rows']); released = any(x.get('event') == 'released' for x in a['rows'])
                gate, contra = {'qualified': False, 'reasons': ['NoGateEvidence']}, False
                if not okcap: st, com, kind = 'invalid', '0', 'exact'
                elif released and not launched: st, com, kind = 'released', '0', 'exact'
                else:
                    capn = int(Decimal(str(cap)) * 10 ** 9); rem = str(max(0, capn - spent)); l = life.get(alias) or {}
                    if not launched:
                        q, rs, contra = qualify(c, o, a, 'bound', ref); gate = {'qualified': q, 'reasons': rs}; st, com, kind = 'pending', str(capn), 'exact' if q else 'at_least'
                    elif l.get('state') == 'retired':
                        q, rs, contra = qualify(c, o, a, 'release', ref); gate = {'qualified': q, 'reasons': rs}
                        st, com, kind = ('retired', '0', 'exact') if q else ('retired_resumable', rem, 'at_least')
                    elif l.get('reopened'):
                        q, rs, contra = qualify(c, o, a, 'release', ref); gate = {'qualified': False, 'reasons': rs if not q else ['ReopenedWithoutAdmission']}; st, com, kind = 'reopened', rem, 'at_least'
                    else:
                        q, rs, contra = qualify(c, o, a, 'bound', ref); gate = {'qualified': q, 'reasons': rs}; st, com, kind = 'launched_open', rem, 'exact' if q else 'at_least'
                new.append({'admission_id': a['admission_id'], 'state': st, 'committed': com, 'kind': kind, 'gate': gate, '_contra': contra, '_over': okcap and launched and spent > int(Decimal(str(cap)) * 10 ** 9)})
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
                    rs |= set(a['blocking']) & DERIVED
            if any(oe[l]['high']['kind'] == 'at_least' for l in ('owned_closed', 'owned_outstanding', 'predecessor_obligations')): rs.add('UnresolvedExposure')
            for cm in oe['commitments']:
                if cm['kind'] == 'at_least': rs.add('UnenforcedCommitment')
                if cm['state'] == 'invalid' or cm.pop('_over'): rs.add('CommitmentInvalid')
                if cm.pop('_contra'): rs.add('GateContradicted')
            if closure(c, o, ref) == 'claimed': rs.add('ClosureNotEstablished')
            oe['reasons'] = sorted(rs); oe['status'] = 'not_established' if rs else 'established'
        ad = ee.get('admission')
        if ad:
            o = objs[e['admission_request']['objective']]; oe = eo[e['admission_request']['objective']]; cs = closure(c, o, ref)
            known = sum(int(oe[l]['high']['nano']) for l in ('owned_closed', 'owned_outstanding', 'predecessor_obligations')) + \
                sum(int(cm['committed']) for cm in oe['commitments'] if cm['state'] != 'invalid') + int(e['admission_request']['new'])
            ad['limit'] = o['limit']
            ad['decision'] = 'refused' if cs == 'closed' else 'not_established' if cs == 'claimed' else 'not_established' if not re.fullmatch(r'\d+', o['limit']) else \
                'refused' if known > int(o['limit']) else 'not_established' if oe['status'] == 'not_established' else 'admitted'
    return c


def save(c):
    finalize(c); p = os.path.join(out, c['case_id'] + '.json'); json.dump(c, open(p, 'w'), indent=1, sort_keys=True); open(p, 'a').write('\n'); written.append(c['case_id'])


def drop_retire(c):
    for s in c['inputs']['sessions']: s['lifecycle'] = [x for x in s['lifecycle'] if x['event'] != 'retire']
    for l in c['expected']['lifecycle'].values(): l['state'] = 'open'


# ---------------- future_qualified_gate positives: gated, settled, sufficiently funded metered sessions
POS = ['VC01-openai-usage-settlement', 'VC02-anthropic-usage-settlement', 'VC11-multi-turn-session', 'VC13-equal-timestamps', 'V01-control-producer-price',
       'V02-control-producer-price', 'U03-control', 'N07-control-retired-journal', 'P130-R02-rounded-up-charge-admitted', 'P130-R14-anthropic-null-tier-admitted',
       'VC15-successor-not-charged-settled-predecessor-spend', 'N06-null-cache-counts', 'P130-R04-retired-session-releases-commitment']
for src in POS:
    save(future(src, 'P132-FG-' + src.split('-')[0] + ('' if src.split('-')[0] not in ('P130',) else '-' + src.split('-')[1]) + '-gated', controls=[], findings=[]))
# bounded execution: an open session whose gate enforces a billing bound (VC14 limit-1.0)
c = future('VC14-open-session-commitment', 'P132-FG-VC14-open-session-billing-bound', notes='VC14 (open session) with a qualified billing_bound gate: the commitment is a finite, enforced allocation.')
c['evaluations'] = c['evaluations'][:1]; save(c)

# ---------------- future negatives: the same session without a qualifying gate cannot admit
def neg(cid, src='VC01-openai-usage-settlement', ctl='P132-FG-VC01-gated', notes='', **kw):
    c = future(src, cid, role='finding', findings=['OQ-1'], controls=[ctl], invariants=['INV-A9', 'INV-A8'], notes=notes, **kw); return c
save(neg('P132-FG-VC01-no-gate', no_gate=True, notes='Future authority declared, but no gate evidence for the session.'))
save(neg('P132-FG-gate-self-asserted-file', notes='Gate written by a same-UID file, not by the authority.', gate_over=lambda g: g.update(writer='same_uid_file')))
c = neg('P132-FG-gate-flag-only', notes='A plain `gated: true` record.'); c['inputs']['gates'] = [{'gate_id': 'g-S1', 'gated': True, 'session': c['inputs']['sessions'][0]['session_id']}]; save(c)
save(neg('P132-FG-gate-wrong-authority', notes='Gate from another authority.', gate_over=lambda g: g['authority'].update(id='another-authority')))
save(neg('P132-FG-gate-wrong-session', notes='Gate names another session.', gate_over=lambda g: g.update(session='00000000-0000-4000-8000-0000000000ff')))
save(neg('P132-FG-gate-request-scope', notes='Gate covers one request, not the session.', gate_over=lambda g: g.update(scope={'kind': 'request', 'request_id': 'resp_1'})))
save(neg('P132-FG-gate-expired', notes='Gate expired before the reference time.', gate_over=lambda g: g.update(effective_until='2026-11-10T06:45:00Z')))
save(neg('P132-FG-gate-starts-after-launch', notes='Gate effective only after the session launched.', gate_over=lambda g: g.update(effective_from='2026-11-10T06:20:00Z')))
save(neg('P132-FG-gate-reopening-only', mechs=('reopening',), notes='Only refusal of delivery after retirement is enforced; new forwarding and retries are not.'))
save(neg('P132-FG-gate-receipt-missing', notes='new_forwarding claimed enforced without a receipt.', gate_over=lambda g: g['mechanisms'].update(new_forwarding={'enforced': True})))
save(neg('P132-FG-gate-contradicted-by-reopening', src='N07-reopened-session-journal', ctl='P132-FG-N07-gated', notes='Gate claims reopening enforced, yet the session was reopened.'))
c = neg('P132-FG-VC14-open-no-billing-bound', src='VC14-open-session-commitment', ctl='P132-FG-VC14-open-session-billing-bound', mechs=('new_forwarding', 'retries', 'reopening'),
        notes='Open session; the gate enforces forwarding, retries and reopening but no billing bound, so the commitment stays AtLeast (known portion below the limit).')
c['evaluations'] = c['evaluations'][:1]; save(c)
c = future('VC01-openai-usage-settlement', 'P132-FG-VC01-before-retirement', role='control', invariants=['INV-A5', 'INV-A9'],
           notes='The gated VC01 session before its retirement: open, with a billing bound; adding the dated retirement is a permitted release.')
drop_retire(c); c['evidence_pair'] = {'other': 'P132-FG-VC01-gated', 'relation': 'subset', 'permitted_release': True}; save(c)
# future closure
c = future('VC01-openai-usage-settlement', 'P132-FG-closure-authority', role='control', invariants=['INV-A7'], notes='Objective closed by its admission authority (receipt).')
o = c['evaluations'][0]['objectives'][0]; o['closed_at'] = '2026-11-10T06:55:00Z'; o['closure_record'] = {'authority_id': AUTH['id'], 'at': '2026-11-10T06:55:00Z', 'receipt': receipt('close-o')}; save(c)
c = future('VC01-openai-usage-settlement', 'P132-FG-closure-legacy-entry-only', role='finding', findings=['OQ-4'], controls=['P132-FG-closure-authority'], invariants=['INV-A7'],
           notes='Future profile with only a legacy scope_budget close entry: the legacy format does not establish the authority.')
o = c['evaluations'][0]['objectives'][0]; o['closed_at'] = '2026-11-10T06:55:00Z'; o['close'] = {'at': '2026-11-10T06:55:00Z', 'disposition': 'complete', 'reason': 'done', 'set_by': 'user'}; save(c)

# ---------------- current_producer additions
def cp(src, cid, *, role, findings, controls, invariants, notes, cls='producer_valid'):
    c = copy.deepcopy(load(src))
    meta(c, cid, profile='current_producer', role=role, cls=cls, reach='producer_reachable', findings=findings, controls=controls, invariants=invariants,
         origin=f'contract::12.4 (P1.32, from {src})', notes=notes)
    c['historical'] = {}; return c
CLOSE = lambda at: {'at': at, 'disposition': 'complete', 'reason': 'objective finished', 'set_by': 'user'}
c = cp('U04-control-settled-before-closure', 'P132-CP-closed-valid-close-entry', role='control', findings=['OQ-4'], controls=[], invariants=['INV-A7'],
       notes='scope_budget close entry matching closed_at: operationally established closure under legacy cooperative trust; set_by is not identity evidence.')
c['evaluations'][0]['objectives'][0]['close'] = CLOSE(c['evaluations'][0]['objectives'][0]['closed_at']); save(c)
c = cp('U04-control-settled-before-closure', 'P132-CP-close-entry-contradicts-closed-at', role='finding', findings=['OQ-4'], controls=['P132-CP-closed-valid-close-entry'], invariants=['INV-A7'],
       notes='The close entry time differs from closed_at: contradictory closure evidence.')
c['evaluations'][0]['objectives'][0]['close'] = CLOSE('2026-11-10T06:41:00Z'); save(c)
c = cp('U04-control-settled-before-closure', 'P132-CP-close-after-reference-time', role='finding', findings=['OQ-4'], controls=['P132-CP-closed-valid-close-entry'], invariants=['INV-A7'],
       notes='A matching close entry dated after the reference time is not a coherent closure.')
o = c['evaluations'][0]['objectives'][0]; o['closed_at'] = '2026-11-10T07:30:00Z'; o['close'] = CLOSE('2026-11-10T07:30:00Z'); save(c)
c = cp('VC05-unsettled-reservation', 'P132-CP-known-over-limit', role='finding', findings=['OQ-2'], controls=['P132-CP-unknown-below-limit'], invariants=['INV-A10'],
       notes='Known owned amounts (outstanding 0.3, resumable commitment 0.7) plus the request 0.1 exceed the 1.05 limit: refused under the conservative policy while billing stays unresolved.')
c['evaluations'][0]['objectives'][0]['limit'] = '1050000000'; save(c)
c = cp('VC05-unsettled-reservation', 'P132-CP-unknown-below-limit', role='control', findings=['OQ-2'], controls=[], invariants=['INV-A10'],
       notes='Same facts with limit 1.2: the known portion 1.1 is within the limit, but unknown exposure may exceed it: not established.')
c['evaluations'][0]['objectives'][0]['limit'] = '1200000000'; save(c)
c = cp('D1-successor-constrained-by-predecessor-obligations', 'P132-CP-closed-predecessor-outstanding', role='finding', findings=['OQ-4', 'R-07'], controls=['P132-CP-closed-valid-close-entry'],
       invariants=['INV-A7', 'INV-A4'], notes='Predecessor p is closed (valid close entry) with an outstanding reservation: p refuses new admission; its obligation still constrains the successor s.')
e0 = c['evaluations'][0]; p = next(o for o in e0['objectives'] if o['objective'] == 'p'); p['close'] = CLOSE(p['closed_at'])
e0['name'] = 'request-successor'; next(o for o in e0['objectives'] if o['objective'] == 's')['limit'] = '2000000000'
e1 = copy.deepcopy(e0); e1['name'] = 'request-predecessor'; e1['admission_request'] = {'objective': 'p', 'new': '100000000'}
e1['expected']['admission'] = {'objective': 'p', 'new': '100000000', 'limit': p['limit'], 'decision': None}
c['evaluations'] = [e0, e1]; save(c)
c = cp('VC01-openai-usage-settlement', 'P132-CP-retired-session-resumed-without-admission', role='finding', findings=['OQ-1'], controls=['VC01-openai-usage-settlement'],
       invariants=['INV-A3', 'INV-A8'], notes='VC01 retired at 06:40 and delivered again at 06:45 with no new admission (role_host deliver permits it); the resumed turn spends again.')
s1 = c['inputs']['sessions'][0]
s1['lifecycle'] = [x for x in s1['lifecycle'] if x['event'] != 'retire'] + [{'at': '2026-11-10T06:40:00Z', 'event': 'retire'}, {'at': '2026-11-10T06:45:00Z', 'event': 'deliver', 'turn': 2}]
s1['logs']['turn-2'] = {'final_newline': True, 'lines': [
    json.dumps({'at': '2026-11-10T06:46:00Z', 'event': 'usd_reserved', 'n': 1, 'rid': 1, 'reserved_usd': 0.5, 'input_bound': 6000, 'bound_basis': 'bytes'}),
    json.dumps({'at': '2026-11-10T06:46:02Z', 'event': 'request', 'n': 1, 'http_status': 200, 'model': 'gpt-6-astra', 'requested_tier': None, 'observed_tier': 'default',
                'terminal': 'response.completed', 'response_id': 'resp_2', 'response_model': 'gpt-6-astra',
                'usage': {'input_tokens': 1000, 'output_tokens': 100, 'cached_tokens': 0, 'cache_write_tokens': 0, 'reasoning_tokens': 0}, 'seconds': 1.0,
                'usd': {'rid': 1, 'reserved_usd': 0.5, 'charged_usd': 0.015, 'charge_basis': 'usage', 'session_spent_usd': 0.03, 'multiplier_floor': 0.0}})]}
a2 = copy.deepcopy(c['expected']['attempts'][0]); a2['key'] = 'res:S1/turn-2:rid=1'; a2['evidence'] = ['S1/turn-2:1', 'S1/turn-2:2']; c['expected']['attempts'].append(a2)
c['expected']['lifecycle']['S1'] = {'state': 'open', 'clock': 'trusted', 'reopened': True}
e = c['evaluations'][0]; e['objectives'][0]['limit'] = '2000000000'
e['expected']['attribution'].append({'attempt': 'res:S1/turn-2:rid=1', 'objective': None, 'ownership': 'uncertain', 'cause': 'ReopenedWithoutAdmission', 'candidates': ['o'], 'placement': 'uncertain'})
oe = e['expected']['objectives'][0]; oe['reasons'] = sorted(set(oe['reasons']) | {'OwnershipUncertain', 'ReopenedWithoutAdmission'}); save(c)
print(f'{len(written)} cases written')
