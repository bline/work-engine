#!/usr/bin/env python3
"""Standalone validator for the Pi attempt-accounting corpus (`pi-accounting-corpus/1`, contract 1.7-s1, revision 8).

It is a development tool, not a reducer, and shares no code with any future implementation. It never derives an
expected outcome from inputs except where a check is stated as arithmetic (price derivations from the rate files),
as a contract rule over raw producer rows (forwarding, execution, tier sets, lifecycle facts, obligation intervals,
closure state), or as internal consistency (expected fields that the contract ties together). Draft2020-12 shape checking uses jsonschema; deterministic, offline.

  validate_corpus.py check      <corpus_dir> <rates_dir>                  validate the corpus itself
  validate_corpus.py results    <corpus_dir> <results.json> [--scope s2a] grade an implementation's result set
  validate_corpus.py selftest   <corpus_dir> <rates_dir>                  prove defective result sets fail, each for its reason
  validate_corpus.py revisions  <corpus_dir> <previous_corpus_dir>        every changed field is a logged revision-6 entry
  validate_corpus.py s2a-subset <corpus_dir>                              print the S2a grading subset (selection rule)

Exit status 0 = pass, 1 = failures (listed), 2 = usage."""
import copy, glob, hashlib, json, os, re, sys
from decimal import Decimal
from fractions import Fraction
import r9_pilot

SCHEMA, CONTRACT, PREVIOUS_CONTRACT = 'pi-accounting-corpus/1', 'pi-accounting-contract/1.7-s1', 'pi-accounting-contract/1.6-s1'
CLASSES = {'producer_valid', 'historical_legacy', 'malformed', 'adversarial_synthetic', 'attribution_uncertain', 'future_vertical'}
PROFILES = {'current_producer', 'future_qualified_gate'}
REACH = {'producer_reachable', 'producer_unreachable', 'legacy', 'not_established'}
ROLES = {'finding', 'control', 'valid_control'}
DECISIONS = {'absent', 'valid', 'rejected', 'conflicting', 'not_applicable'}
BASES = {'usage', 'reservation', 'upstream_error', 'error_status'}
REJECT = {'ReservationConflict', 'NoReservation', 'Order', 'AttemptNumber', 'AmountType', 'ReservedMismatch', 'BasisMissing',
          'BasisUnknown', 'KeepAll', 'ErrorCharge', 'StatusType', 'UsageUnpriceable', 'ModelMismatch', 'Undercharge', 'ToleranceRange'}
OBLIGATION = {('closed', 'settlement_amount'), ('closed', 'reservation_confirmed'), ('closed', 'reservation_bound'),
              ('outstanding', 'reservation_bound'), ('none', 'none')}
BLOCKING = {'SettlementRejected', 'SettlementConflict', 'ReservationConflict', 'SettlementWithoutReservation',
            'IdentityContradiction', 'LaneContradiction', 'UnadmittedMeasurement', 'UnaccountedAttempt', 'UnresolvedExposure',
            'UnreadableEvidence', 'InventoryIncomplete', 'DuplicateResponseIdentity', 'ArithmeticFailure', 'RateTableUnusable',
            'AttributionContradiction', 'ReopenedWithoutAdmission', 'CommitmentInvalid', 'MalformedEvidence', 'UnadmittedExecution',
            'UnenforcedCommitment', 'GateContradicted'}
OBJ_REASONS = BLOCKING | {'OwnershipUncertain', 'HistoricalUnadmitted', 'PlacementUnplaceable', 'PlacementStraddles', 'ClosureNotEstablished'}
DIAGNOSTICS = {'RepeatedIdenticalRow', 'InconsistentCacheSplit', 'NonBillingUnverified', 'IdentityAbsent', 'ProducerLogCopy',
               'ErrorStatusSettlement', 'ReadFailurePaired', 'PartialUsage', 'UnfinishedAdmission', 'UncertainCodexCall'}
LOW_BASES = {'none', 'usage_valuation'}
HIGH_BASES = {'none', 'usage_valuation', 'proxy_charge', 'proxy_reservation'}
PLACEMENTS = {'inside', 'before', 'after', 'straddles_start', 'straddles_closure', 'unplaceable', 'uncertain', 'not_evaluated'}
OWNERSHIP = {'owned', 'uncertain', 'unadmitted', 'unowned'}
FORWARDING = {'forwarded', 'unknown'}                 # NotForwarded requests are non-attempt evidence (contract 6.1)
KEY_JOIN = {'exact', 'ambiguous', 'unattributed'}     # contract 4: the key join; RS rules are auxiliary rows
SHAPE_RULES = {'RS-1', 'RS-2', 'RS-3'}
EXECUTION = {'terminal', 'not_terminal'}
BILLING = {'established', 'unresolved', 'not_applicable'}
TERMINAL_EVENTS = {'request', 'upstream_error', 'stream_error', 'response_model_refused'}
COMMIT_STATES = {'pending', 'launched_open', 'retired', 'retired_resumable', 'released', 'reopened', 'invalid'}
OPEN_COMMIT = {'pending', 'launched_open', 'retired_resumable', 'reopened'}
LIFE = {'open', 'retired', 'contradicted'}
INVARIANTS = ['INV-F1', 'INV-F2', 'INV-F3', 'INV-F4', 'INV-F5', 'INV-F6', 'INV-F7', 'INV-F8', 'INV-F9', 'INV-F10', 'INV-F11', 'INV-F12', 'INV-F13',
              'INV-T1', 'INV-T2', 'INV-T3', 'INV-T4', 'INV-T5', 'INV-A1', 'INV-A2', 'INV-A3', 'INV-A4', 'INV-A5', 'INV-A6', 'INV-A7', 'INV-A8', 'INV-A9', 'INV-A10']
KEY = re.compile(r'^(res|row|adm):(S\d)/turn-(\d+):(rid|line)=(\d+)$')
REF = re.compile(r'^(S\d)/turn-(\d+):(\d+)$')
STAMP = re.compile(r'^\d{4}-\d\d-\d\dT\d\d:\d\d:\d\dZ$')
MIN_BLOCKED = 25
# revision 4 (OQ-1): admitted valid controls are counted per profile. Total minimum 15 is kept; the current producer
# can only admit sessionless or Codex cases, so metered positives live in the future profile (no padding of either).
MIN_ADMITTED = {'current_producer': 1, 'future_qualified_gate': 11}; MIN_VALID_ADMITTED = 15
ADDED_IN = {'1.1-s1', '1.2-s1', '1.3-s1', '1.4-s1', '1.5-s1', '1.6-s1'}
REPLACEMENT_REASONS = {'UsageBound', 'NotUsageSettlement', 'UsageIncomplete', 'UsageUnpriceable', 'NoReservation'}
TIE = {'usage_valuation': 0, 'proxy_reservation': 1, 'proxy_charge': 2}     # X-0 (1.4-s1)
COVERS = ['failures', 'forwarding', 'partial_streams', 'retries', 'settlement']
PROVIDERS = {'openai': 'openai', 'openai-codex': 'openai', 'anthropic': 'anthropic'}
RANK = {'admitted': 0, 'refused': 1, 'not_established': 1}     # OQ-2: refused and not_established are equally non-permissive
VERTICAL = 'vertical_admission_authority'
MECHS = ('new_forwarding', 'retries', 'reopening', 'billing_bound')


class Failures(list):
    def add(self, case, msg): self.append(f'{case}: {msg}')


def nano(s):
    if not isinstance(s, str) or not re.fullmatch(r'-?\d+', s): raise ValueError(f'not a nano integer string: {s!r}')
    return int(s)


def load_rates(rates_dir):
    out = {}
    for name in ('openai_api_rates.json', 'anthropic_api_rates.json'):
        raw = open(os.path.join(rates_dir, name), 'rb').read()
        out[name] = (hashlib.sha256(raw).hexdigest(), json.loads(raw, parse_float=Decimal, parse_int=Decimal))
    return out


def per_token(rate, mult=1):
    v = Fraction(rate) * Fraction(mult) * Fraction(10 ** 9, 10 ** 6)
    if v.denominator != 1: raise ValueError('rate not exact in nano per token')
    return int(v)


def price(d, rates):
    """Arithmetic of one derivation record against the rate files (the contract's pricing primitives, 7.1)."""
    u, m, mult = d['usage'], d['model'], Fraction(Decimal(d['tier_multiplier']))
    if d['provider'] == 'openai':
        oa = rates['openai_api_rates.json'][1]; r = oa['models'][m]
        i, o = u['input_tokens'], u['output_tokens']; c, w = u.get('cached_tokens') or 0, u.get('cache_write_tokens') or 0
        long_ = i > int(oa['long_input_threshold'])
        mi = Fraction(oa['long_input_multiplier']) if long_ else 1; mo = Fraction(oa['long_output_multiplier']) if long_ else 1
        return ((i - c - w) * per_token(r['input'], mult * mi) + c * per_token(r['cached_input'], mult * mi)
                + w * per_token(r['cache_write'], mult * mi) + o * per_token(r['output'], mult * mo))
    an = rates['anthropic_api_rates.json'][1]; r = an['models'][m]
    i, cr, cw, o = (u.get(k, 0) for k in ('input_tokens', 'cache_read_input_tokens', 'cache_creation_input_tokens', 'output_tokens'))
    w5, w1 = u.get('ephemeral_5m_input_tokens') or 0, u.get('ephemeral_1h_input_tokens') or 0
    rate = (max if d.get('remainder_rate', 'max') == 'max' else min)(per_token(r['cache_write_5m']), per_token(r['cache_write_1h']))
    writes = cw * rate if d.get('split') == 'ignored' else (w5 * per_token(r['cache_write_5m']) + w1 * per_token(r['cache_write_1h']) + (cw - w5 - w1) * rate)
    return i * per_token(r['input']) + cr * per_token(r['cache_read']) + writes + o * per_token(r['output'])


def hsum(items):
    """High-only sum with AtLeast propagation. Low is not established by this operation."""
    low = 0; high = sum(x[2] for x in items)  # R8: aggregate low is separately qualified, never sum(local holds)
    return low, ('at_least' if any(x[1] == 'at_least' for x in items) else 'exact'), high


def esum_of(e): return (nano(e['low']), e['high']['kind'], nano(e['high']['nano']))


def att_exp(a):
    x = a['exposure']
    if x['pool'] == 'codex': return None
    return (nano(x['low']['nano']), x['high']['kind'], nano(x['high']['nano']))


def parse(line):
    try: row = json.loads(line) if isinstance(line, str) else None
    except ValueError: return None
    return row if isinstance(row, dict) else None


def source_lines(c):
    """{ref: raw line} and {source: [refs in append order]}."""
    lines, order = {}, {}
    for s in c['inputs']['sessions']:
        for turn, log in s['logs'].items():
            src = f'{s["alias"]}/{turn}'
            for n, ln in enumerate(log['lines'], 1):
                lines[f'{src}:{n}'] = ln; order.setdefault(src, []).append(f'{src}:{n}')
    return lines, order


def forwarded_row(line):
    """Contract 6.1 (1.2-s1): a provider response exists."""
    row = parse(line)
    if row is None: return False
    if row.get('event') in ('response_model_refused', 'stream_error'): return True
    if row.get('event') != 'request': return False
    st = row.get('http_status')
    if isinstance(st, int) and not isinstance(st, bool): return True
    if row.get('response_id') or row.get('response_model'): return True
    u = row.get('usage')
    return isinstance(u, dict) and any(v is not None for v in u.values())


def reopened_of(session):
    """Contract L-5: a deliver follows a retire anywhere in the lifecycle rows."""
    seen = False
    for r in session['lifecycle']:
        if r['event'] == 'retire': seen = True
        elif r['event'] == 'deliver' and seen: return True
    return False


def closure_state(o, ref, profile='current_producer'):
    """Contract 12.4 step 1 (1.3-s1): 'open', 'closed' or 'claimed' (claimed but not established)."""
    ca, ce, cr = o.get('closed_at'), o.get('close'), o.get('closure_record')
    if ca is None and ce is None and cr is None: return 'open'
    rows = [r for a in o.get('admissions', []) for r in a['rows']]
    coherent = isinstance(ca, str) and STAMP.match(ca) is not None and ca >= o['started_at'] and ca <= ref and \
        not any(isinstance(r.get('at'), str) and r['at'] > ca for r in rows)
    if profile == 'current_producer':      # legacy cooperative trust: a consistent scope_budget close entry
        ok = coherent and isinstance(ce, dict) and ce.get('at') == ca and cr is None
    else:                                  # future vertical: the objective's declared admission authority
        auth = o.get('admission_authority') or {}
        ok = coherent and isinstance(cr, dict) and auth.get('kind') == VERTICAL and cr.get('authority_id') == auth.get('id') and \
            cr.get('at') == ca and isinstance(cr.get('receipt'), dict) and cr['receipt'].get('ref') and re.fullmatch(r'[0-9a-f]{64}', str(cr['receipt'].get('sha256')))
    return 'closed' if ok else 'claimed'


def gate_qualification(c, o, adm, purpose, ref, lifecycle_by_sid, sessions_by_sid, cap_nano=None):
    """Contract 2.3 G-1..G-7 for one admission's session. purpose: 'release' (retired) or 'bound' (open/pending).
    Returns (qualified, reasons, contradicted); a qualified 'bound' gate also sets gate_qualification.ceiling."""
    gate_qualification.ceiling = None
    gid = adm.get('gate_id')
    if not gid: return False, ['NoGateEvidence'], False
    g = next((x for x in c['inputs'].get('gates', []) if x.get('gate_id') == gid), None)
    if not isinstance(g, dict) or not all(k in g for k in ('authority', 'session', 'scope', 'effective_from', 'effective_until', 'writer', 'mechanisms')):
        return False, ['GateMalformed'], False
    reasons = []; auth = o.get('admission_authority') or {}
    witness = c.get('trusted_fixture_inputs', {}).get('gate_verifications', {}).get(gid)
    payload = hashlib.sha256(json.dumps(g, sort_keys=True, separators=(',', ':'), ensure_ascii=False).encode()).hexdigest()
    if not isinstance(witness, dict): reasons.append('GateVerificationMissing')
    elif witness.get('source') != 'test_mock' or not witness.get('policy_ref') or not witness.get('verification_evidence'):
        reasons.append('GateVerificationRejected')
    elif witness.get('payload_sha256') != payload or witness.get('authority_id') != (g.get('authority') or {}).get('id') or witness.get('session') != g.get('session'):
        reasons.append('GateVerificationBinding')
    ceiling = None
    if g['writer'] != 'authority': reasons.append('GateWriterNotAuthority')
    if (g['authority'] or {}).get('kind') != VERTICAL or (g['authority'] or {}).get('id') != auth.get('id') or auth.get('kind') != VERTICAL: reasons.append('GateAuthorityMismatch')
    if g['session'] != adm['session']: reasons.append('GateSessionMismatch')
    if (g['scope'] or {}).get('kind') != 'session': reasons.append('GateScopeInsufficient')
    launched = [r.get('at') for r in adm['rows'] if r.get('event') in ('launched', 'objective_admission') and isinstance(r.get('at'), str)]
    if not (isinstance(g['effective_from'], str) and STAMP.match(g['effective_from']) and launched and g['effective_from'] <= min(launched)) or \
            not (g['effective_until'] is None or (isinstance(g['effective_until'], str) and g['effective_until'] >= ref)):
        reasons.append('GatePeriodInsufficient')
    need = ('new_forwarding', 'retries', 'reopening') if purpose == 'release' else ('billing_bound', 'retries')
    m = g['mechanisms'] or {}
    if not all((m.get(k) or {}).get('enforced') is True for k in need): reasons.append('GateMechanismMissing')
    if any((m.get(k) or {}).get('enforced') is True and not ((m[k].get('receipt') or {}).get('ref') and re.fullmatch(r'[0-9a-f]{64}', str((m[k].get('receipt') or {}).get('sha256'))))
           for k in need): reasons.append('GateReceiptMissing')
    if purpose == 'bound' and (m.get('billing_bound') or {}).get('enforced') is True:   # G-8 (1.4-s1, A-03)
        bnd = m['billing_bound'].get('bound')
        if not isinstance(bnd, dict) or not (isinstance(bnd.get('amount_nano'), str) and re.fullmatch(r'\d+', bnd['amount_nano'])): reasons.append('GateBoundMissing')
        else:
            if cap_nano is None or int(bnd['amount_nano']) > cap_nano: reasons.append('GateBoundExceedsCap')
            if bnd.get('currency') != 'USD': reasons.append('GateBoundCurrency')
            if bnd.get('scope') != 'session' or sorted(bnd.get('covers') or []) != COVERS: reasons.append('GateBoundCoverage')
            doc = {'amount_nano': bnd['amount_nano'], 'authority_id': (g['authority'] or {}).get('id'), 'covers': sorted(bnd.get('covers') or []), 'currency': bnd.get('currency'),
                   'effective_from': g['effective_from'], 'effective_until': g['effective_until'], 'gate_id': gid, 'session': g['session']}
            if bnd.get('binding_sha256') != hashlib.sha256(json.dumps(doc, sort_keys=True, separators=(',', ':')).encode()).hexdigest(): reasons.append('GateBoundBinding')
            ceiling = int(bnd['amount_nano'])
    contradicted = False
    sess = sessions_by_sid.get(adm['session'])
    if sess is not None and (m.get('reopening') or {}).get('enforced') is True:
        seen = False
        for r in sess['lifecycle']:
            if r['event'] == 'retire': seen = True
            elif r['event'] == 'deliver' and seen and isinstance(r.get('at'), str) and isinstance(g['effective_from'], str) and r['at'] >= g['effective_from']:
                contradicted = True
    if contradicted: reasons.append('GateContradicted')
    if not reasons: gate_qualification.ceiling = ceiling
    return (not reasons), reasons, contradicted


def gate_projection(c, adm, qualified, reasons):
    out = {'qualified': qualified, 'reasons': reasons}
    if adm.get('gate_id'):
        w = c.get('trusted_fixture_inputs', {}).get('gate_verifications', {}).get(adm['gate_id'])
        out['verification_source'] = 'test_mock' if isinstance(w, dict) and w.get('source') == 'test_mock' else 'unverified'
        if isinstance(w, dict) and w.get('payload_sha256'): out['verification_payload_sha256'] = w['payload_sha256']
    return out


# ------------------------------------------------------------------------------------------------ corpus checks
def check_case(c, rates, F):
    cid = c.get('case_id', '?')
    r9_pilot.case_claim(c, F)
    for k in ('schema', 'contract', 'case_id', 'group', 'role', 'class', 'reachability', 'findings', 'provenance', 'invariants',
              'controls', 'inputs', 'evaluations', 'expected', 'ambiguity', 'notes', 'historical', 'rationale', 'revisions', 'revisions_r3',
              'revisions_r4', 'revisions_r5', 'revisions_r6', 'revisions_r7', 'revisions_r8', 'profile'):
        if k not in c: F.add(cid, f'missing field {k}')
    if c.get('schema') != SCHEMA or c.get('contract') != CONTRACT: F.add(cid, 'schema or contract version mismatch')
    if c.get('class') not in CLASSES: F.add(cid, f'bad class {c.get("class")}')
    if c.get('reachability') not in REACH: F.add(cid, 'bad reachability')
    if c.get('role') not in ROLES: F.add(cid, 'bad role')
    if not c.get('provenance', {}).get('origin'): F.add(cid, 'provenance.origin missing')
    for inv in c.get('invariants', []):
        if inv not in INVARIANTS: F.add(cid, f'unknown invariant {inv}')
    if c.get('role') == 'finding' and not c.get('controls'): F.add(cid, 'finding without a control')
    prof = c.get('profile')
    if prof not in PROFILES: F.add(cid, f'bad profile {prof}')
    if prof == 'current_producer':            # OQ-1: no hypothetical gate or authority evidence on current-producer cases
        if c['inputs'].get('gates') or any(o.get('admission_authority') or o.get('closure_record') or any(a.get('gate_id') for a in o['admissions'])
                                            for e in c['evaluations'] for o in e['objectives']):
            F.add(cid, 'current_producer case carries future gate or authority evidence (OQ-1)')
        if c.get('class') == 'future_vertical': F.add(cid, 'current_producer case classed future_vertical')
    if prof == 'future_qualified_gate':
        if c.get('reachability') != 'producer_unreachable' or c.get('class') != 'future_vertical' or c.get('historical') != {}:
            F.add(cid, 'future_qualified_gate case must be producer_unreachable, class future_vertical, with no historical comparison')
    for r in c.get('revisions', []) + c.get('revisions_r3', []) + c.get('revisions_r4', []) + c.get('revisions_r5', []) + c.get('revisions_r6', []) + c.get('revisions_r7', []) + c.get('revisions_r8', []):
        if set(r) != {'rule', 'field', 'from', 'to'} or not r['rule']: F.add(cid, f'malformed revision entry {r}')
    if c.get('added_in') not in (None,) + tuple(sorted(ADDED_IN)): F.add(cid, 'bad added_in')
    if c.get('added_in') == '1.2-s1' and (c.get('revisions') or c.get('revisions_r3')): F.add(cid, 'a case added in revision 3 has no earlier expectation to revise')
    if c.get('added_in') == '1.3-s1' and (c.get('revisions') or c.get('revisions_r3') or c.get('revisions_r4')): F.add(cid, 'a case added in revision 4 has no earlier expectation to revise')
    if c.get('added_in') == '1.4-s1' and any(c.get(k) for k in ('revisions', 'revisions_r3', 'revisions_r4', 'revisions_r5')): F.add(cid, 'a case added in revision 5 has no earlier expectation to revise')
    inp = c['inputs']
    for name, (digest, _) in rates.items():
        if inp['rates'].get(name) != digest: F.add(cid, f'rate digest mismatch for {name}')
    sessions = {s['alias']: s for s in inp['sessions']}
    for s in inp['sessions']:
        for turn, log in s['logs'].items():
            if not isinstance(log.get('lines'), list) or not isinstance(log.get('final_newline'), bool): F.add(cid, f'bad log {s["alias"]}/{turn}')
    lines, order = source_lines(c)
    ex = c['expected']
    covered, keys = [], set()
    for a in ex['attempts']:
        check_attempt(cid, c, a, lines, rates, F)
        keys.add(a['key']); covered += a['evidence']
    for x in ex['non_attempt'] + ex['unreadable']:
        covered.append(x['evidence'])
        for b in x.get('blocking', []):
            if b not in BLOCKING: F.add(cid, f'unknown blocking {b}')
    for r in covered:
        if r not in lines: F.add(cid, f'evidence ref {r} not in inputs')
    dup = {r for r in covered if covered.count(r) > 1}
    if dup: F.add(cid, f'evidence counted twice (INV-F6): {sorted(dup)}')
    missing = set(lines) - set(covered)
    if missing: F.add(cid, f'evidence not accounted for (INV-F6): {sorted(missing)}')
    for alias in sessions:
        if alias not in ex['lifecycle']: F.add(cid, f'lifecycle missing for {alias}')
    for alias, l in ex['lifecycle'].items():
        if l.get('state') not in LIFE or l.get('clock') not in ('trusted', 'untrusted') or not isinstance(l.get('reopened'), bool):
            F.add(cid, f'bad lifecycle {alias}'); continue
        s = sessions.get(alias)
        if s is None: continue                           # S08 pair: journal-only cases (revision-1 oddity)
        if l['reopened'] != reopened_of(s): F.add(cid, f'lifecycle {alias}: reopened {l["reopened"]} contradicts its rows (L-5, R-10)')
        last = s['lifecycle'][-1] if s['lifecycle'] else None
        if l['state'] == 'retired' and not (last and last['event'] == 'retire' and last.get('at')):
            F.add(cid, f'lifecycle {alias}: retired but the last row is not a dated retire (L-4)')
        if l['state'] == 'open' and last and last['event'] == 'retire' and last.get('at') and l['clock'] == 'trusted':
            F.add(cid, f'lifecycle {alias}: open but the last row is a dated retire (L-4)')
    names = [e['name'] for e in c['evaluations']]
    if len(names) != len(set(names)): F.add(cid, 'duplicate evaluation names')
    for e in c['evaluations']:
        check_evaluation(cid, c, e, keys, sessions, lines, order, F)


def check_attempt(cid, c, a, lines, rates, F):
    k = a['key']; m = KEY.match(k)
    if not m: F.add(cid, f'bad key {k}'); return
    kind, alias, turn, field, num = m.groups()
    src = f'{alias}/turn-{turn}'
    rows = [parse(lines.get(r)) for r in a['evidence']]
    r9_pilot.attempt_policy(cid, a, rows, F)
    if kind == 'row' and f'{src}:{num}' not in a['evidence']: F.add(cid, f'{k}: row key line not in its evidence')
    if kind in ('res', 'adm') and not any(r and r.get('event') in ('usd_reserved', 'subscription_proxy_admitted') and str(r.get('rid')) == num for r in rows):
        F.add(cid, f'{k}: no reservation/admission with that rid in its evidence')
    corr = a['correlation']                                                       # R-06: key join plus auxiliary rows
    if corr.get('kind') not in KEY_JOIN: F.add(cid, f'{k}: bad key join {corr.get("kind")}')
    for ax in corr.get('auxiliary', []):
        row = parse(lines.get(ax.get('evidence')))
        if ax.get('evidence') not in a['evidence'] or ax.get('rule') not in SHAPE_RULES or row is None: F.add(cid, f'{k}: bad auxiliary {ax}'); continue
        want = {'RS-1': row.get('event') == 'stream_error' and 'detail' in row, 'RS-2': row.get('event') == 'stream_error' and 'detail' not in row,
                'RS-3': row.get('event') == 'response_model_refused'}[ax['rule']]
        if not want: F.add(cid, f'{k}: auxiliary {ax["rule"]} does not match its row shape (E-3)')
    s = a['settlement']
    if s.get('decision') not in DECISIONS: F.add(cid, f'{k}: bad decision'); return
    if s['decision'] == 'valid' and s.get('basis') not in BASES: F.add(cid, f'{k}: valid without a basis')
    if s['decision'] != 'rejected' and s.get('reasons'): F.add(cid, f'{k}: reasons on a non-rejected decision')
    if s['decision'] == 'rejected' and not s.get('reasons'): F.add(cid, f'{k}: rejected without reasons')
    for r in s.get('reasons', []):
        if r not in REJECT: F.add(cid, f'{k}: unknown reject reason {r}')
    ob = a['obligation']; obt = (ob.get('state'), ob.get('amount_source'))
    if obt not in OBLIGATION: F.add(cid, f'{k}: bad obligation {obt}')
    rp_want = replacement_of(a)
    if a.get('replacement') != rp_want: F.add(cid, f'{k}: replacement {a.get("replacement")} != {rp_want} (2.1, A-01, INV-F13)')
    want = {'valid': {'usage': ('closed', 'settlement_amount' if rp_want['eligible'] else 'reservation_bound'), 'reservation': ('closed', 'reservation_confirmed'),
                      'upstream_error': ('closed', 'reservation_confirmed'), 'error_status': ('closed', 'reservation_bound')}.get(s.get('basis')),
            'absent': ('outstanding', 'reservation_bound'), 'rejected': ('outstanding', 'reservation_bound'),
            'conflicting': ('outstanding', 'reservation_bound'), 'not_applicable': ('none', 'none')}[s['decision']]
    if kind == 'row' and s['decision'] in ('rejected', 'conflicting'): want = ('none', 'none')
    if ob.get('state') == 'closed' and ob.get('by') not in [ref for ref, row in zip(a['evidence'], rows) if row and isinstance(row.get('usd'), dict)]: F.add(cid, f'{k}: closed obligation lacks settlement evidence by reference')
    if obt != want: F.add(cid, f'{k}: obligation {obt} does not match decision (want {want}) (INV-F1)')
    if ob.get('state') == 'closed' and not (s['decision'] == 'valid' and corr.get('kind') == 'exact'):
        F.add(cid, f'{k}: closed without a Valid settlement on an Exact key join (INV-F1, R-06)')
    for b in a['blocking']:
        if b not in BLOCKING and not re.fullmatch(r'MalformedEvidence\([A-Za-z_]+\)', b): F.add(cid, f'{k}: unknown blocking {b}')
    for d in a['diagnostics']:
        if d not in DIAGNOSTICS: F.add(cid, f'{k}: unknown diagnostic {d}')
    if s['decision'] == 'rejected' and 'SettlementRejected' not in a['blocking']: F.add(cid, f'{k}: rejected without SettlementRejected')
    if s['decision'] == 'conflicting' and 'SettlementConflict' not in a['blocking']: F.add(cid, f'{k}: conflicting without SettlementConflict')
    fw = a.get('forwarding')                                                       # R-13
    if fw not in FORWARDING: F.add(cid, f'{k}: bad or missing forwarding fact')
    elif fw != ('forwarded' if any(forwarded_row(lines.get(r)) for r in a['evidence']) else 'unknown'):
        F.add(cid, f'{k}: forwarding {fw} contradicts its evidence rows (6.1, R-13)')
    ex_ = a.get('execution')                                                       # R-07
    if ex_ not in EXECUTION: F.add(cid, f'{k}: bad or missing execution fact')
    elif ex_ != ('terminal' if any(r and r.get('event') in TERMINAL_EVENTS for r in rows) else 'not_terminal'):
        F.add(cid, f'{k}: execution {ex_} contradicts its evidence rows (6.2)')
    raw_settlement_checks(cid, c, a, rows, lines, rates, F)
    v = a['valuation']; st = v.get('state')
    if st == 'valued':
        lows, highs = [], []
        for d in v['derivations']:
            try: p = price(d, rates)
            except (KeyError, ValueError) as e: F.add(cid, f'{k}: derivation not computable: {e}'); continue
            if p != nano(d['nano']): F.add(cid, f'{k}: derivation {d["model"]} states {d["nano"]}, rate files give {p}')
            if d['provider'] + '/' + d['model'] not in v['identities']: F.add(cid, f'{k}: derivation model {d["model"]} not among identities')
            if d['for'] in ('low', 'both'): lows.append(p)
            if d['for'] in ('high', 'both'): highs.append(p)
            if d['provider'] == 'anthropic' and d.get('split') != 'ignored':
                u = d['usage']
                rem = (u.get('cache_creation_input_tokens') or 0) - (u.get('ephemeral_5m_input_tokens') or 0) - (u.get('ephemeral_1h_input_tokens') or 0)
                if rem > 0 and d['for'] in ('low', 'high') and d.get('remainder_rate') != {'low': 'min', 'high': 'max'}[d['for']]:
                    F.add(cid, f'{k}: residual Anthropic writes must be at min for the low and max for the high (V-4, R-15)')
        if lows and min(lows) != nano(v['low']): F.add(cid, f'{k}: valuation low {v["low"]} != derivations {min(lows)}')
        if highs and max(highs) != nano(v['high']): F.add(cid, f'{k}: valuation high {v["high"]} != derivations {max(highs)}')
        if nano(v['low']) > nano(v['high']): F.add(cid, f'{k}: valuation low > high')
        if len(v['identities']) > 1 and 'IdentityContradiction' not in a['blocking']: F.add(cid, f'{k}: several identities without IdentityContradiction (D6)')
    tiers = set(rates['openai_api_rates.json'][1]['tier_multiplier'])             # R-03, from raw rows
    oa_req = [r for r in rows if r and r.get('event') == 'request' and r.get('provider', 'openai') != 'anthropic' and isinstance(r.get('usage'), dict)
              and any(x is not None for x in r['usage'].values())]
    if st == 'valued' and any(d['provider'] == 'openai' for d in v['derivations']) and any(
            not (isinstance(r.get('observed_tier'), str) and r['observed_tier'] in tiers) for r in oa_req):
        F.add(cid, f'{k}: OpenAI tier missing or unrecognised but valued as a finite range (V-3, R-03)')
    x = a['exposure']
    bl = a.get('billing')
    if x['pool'] == 'codex':
        if set(x) != {'pool', 'billing'}: F.add(cid, f'{k}: codex pool carries money (INV-F9)')
        if bl != 'not_applicable': F.add(cid, f'{k}: codex attempt billing must be not_applicable')
        return
    lo, kd, hi = att_exp(a)
    if x['low']['basis'] not in LOW_BASES or x['high']['basis'] not in HIGH_BASES: F.add(cid, f'{k}: bad basis')
    if kd not in ('exact', 'at_least'): F.add(cid, f'{k}: bad high kind')
    if lo > hi: F.add(cid, f'{k}: low > high (INV-F11)')
    if kd == 'at_least' and 'UnresolvedExposure' not in a['blocking']: F.add(cid, f'{k}: AtLeast without UnresolvedExposure (D5)')
    if lo > 0 and x['low']['basis'] != 'usage_valuation': F.add(cid, f'{k}: non-zero low not supported by usage valuation (INV-F11)')
    if bl not in BILLING or bl != ('established' if kd == 'exact' else 'unresolved'):
        F.add(cid, f'{k}: billing {bl} does not follow the high kind {kd} (INV-F12, R-07)')
    table_x(cid, k, a, lo, kd, hi, F, lines)


def usage_bound(a):
    """FB-1: a complete usage valuation exists (as the expected valuation states it)."""
    s, v = a['settlement'], a['valuation']
    if v.get('state') != 'valued' or not v.get('complete'): return None
    if s['decision'] == 'valid' and s.get('basis') == 'usage': return nano(v['high'])
    if s['decision'] == 'not_applicable': return nano(v['high'])
    return None


def replacement_of(a):
    """2.1 (1.4-s1): Eligible iff Valid(Usage) with a complete usage valuation over established sets."""
    s_, v, x = a['settlement'], a['valuation'], a['exposure']
    if x.get('pool') == 'codex' or a['obligation'].get('state') == 'none': return {'eligible': False, 'reason': 'NoReservation'}
    if not (s_.get('decision') == 'valid' and s_.get('basis') == 'usage'): return {'eligible': False, 'reason': 'NotUsageSettlement'}
    if v.get('state') == 'valued' and v.get('complete'): return {'eligible': True, 'reason': 'UsageBound'}
    return {'eligible': False, 'reason': 'UsageIncomplete' if v.get('state') == 'valued' else 'UsageUnpriceable'}


def high_of(terms):
    terms = [t for t in terms if t[0] is not None]
    best = max(t[0] for t in terms)
    return best, sorted((t for t in terms if t[0] == best), key=lambda t: TIE[t[1]])[0][1]


def raw_reservation(a, lines):
    """X-0 (1.4-s1, A-09): the largest distinct well-formed reservation amount in the attempt's own rows."""
    vals = set(); n = 0; seen = set()
    for r in a['evidence']:
        row = parse(lines.get(r))
        if row and row.get('event') == 'usd_reserved':
            key = lines.get(r)  # admitted scoped exact-copy rule is byte equality, never normalized JSON
            if key in seen: continue
            seen.add(key); n += 1
            amt = row.get('reserved_usd')
            if isinstance(amt, (int, float)) and not isinstance(amt, bool): vals.add(int(Decimal(str(amt)) * 10 ** 9))
    return (max(vals) if vals else None), n


def table_x(cid, k, a, lo, kd, hi, F, lines=None):
    """Consistency of expected exposure with TRANSITIONS Table X (1.4-s1), including the X-0 basis on ties."""
    s, v = a['settlement'], a['valuation']
    R = nano(s['reserved']) if 'reserved' in s else None    # settlement.reserved is the settlement's claim; R is the reservation rows' amount
    if lines is not None:
        Rraw, nres = raw_reservation(a, lines)
        if Rraw is not None: R = Rraw                                               # X-0 (1.4-s1): the largest distinct reservation (A-09)
    ch = nano(s['charged']) if 'charged' in s else None
    vl, vh = (nano(v['low']), nano(v['high'])) if v.get('state') == 'valued' else (None, None)
    ph = nano(v['priced_high']) if v.get('state') == 'unpriceable' and v.get('priced_high') is not None else None
    sl = nano(v['supported_low']) if v.get('state') == 'unpriceable' and v.get('supported_low') is not None else 0
    known_v = vh if vh is not None else ph
    exp = None; terms = None
    d, b = s['decision'], s.get('basis')
    if d == 'valid' and b == 'usage' and ch is not None:
        if v.get('state') == 'valued' and v.get('complete'): terms = [(ch, 'proxy_charge'), (vh, 'usage_valuation')]; exp = (vl, 'exact')     # FB-1, eligible
        elif R is not None: terms = [(R, 'proxy_reservation'), (ch, 'proxy_charge'), (known_v, 'usage_valuation')]; exp = (vl if vl is not None else sl, 'at_least')  # A-01
    elif d == 'valid' and b == 'reservation' and R is not None: terms = [(R, 'proxy_reservation'), (known_v, 'usage_valuation')]; exp = (vl if vl is not None else sl, 'at_least')
    elif d == 'valid' and b == 'upstream_error' and R is not None: terms = [(R, 'proxy_reservation')]; exp = (0, 'at_least')
    elif d == 'valid' and b == 'error_status' and R is not None: terms = [(R, 'proxy_reservation'), (known_v, 'usage_valuation')]; exp = (vl if vl is not None else sl, 'at_least')   # A-02
    elif d == 'absent' and R is not None: terms = [(R, 'proxy_reservation')]; exp = (0, 'at_least')
    elif d in ('rejected', 'conflicting') and R is not None: terms = [(R, 'proxy_reservation'), (ch, 'proxy_charge'), (known_v, 'usage_valuation')]; exp = (vl if vl is not None else sl, 'at_least')  # R8 local usage support survives C/S rejection
    elif d == 'not_applicable' and v.get('state') == 'valued' and v.get('complete'): terms = [(vh, 'usage_valuation')]; exp = (vl, 'exact')
    if exp is not None:
        h_amt, h_basis = high_of(terms)
        if (exp[0], exp[1], h_amt) != (lo, kd, hi): F.add(cid, f'{k}: exposure {(lo, kd, hi)} != Table X {(exp[0], exp[1], h_amt)}')
        elif a['exposure']['high'].get('basis') != h_basis: F.add(cid, f'{k}: high basis {a["exposure"]["high"].get("basis")} != X-0 tie order {h_basis}')
    ub = usage_bound(a)
    if kd == 'exact' and ub is None: F.add(cid, f'{k}: Exact high without a usage bound (FB-1, FB-2)')
    if kd == 'exact' and a['exposure']['high']['basis'] == 'proxy_charge' and not (ub is not None and ub <= hi):
        F.add(cid, f'{k}: Exact high with basis proxy_charge needs a usage bound below it (FB-1, R-02)')


def openai_range(u, model, mult, rates):
    """V-4 (1.4-s1, A-04) from raw counts: (low, high) over the vertices of unknown cache counts, or 'malformed'."""
    oa = rates['openai_api_rates.json'][1]; r = oa['models'][model]
    i, o = u.get('input_tokens'), u.get('output_tokens')
    if not all(isinstance(x, int) and not isinstance(x, bool) and x >= 0 for x in (i, o)): return 'malformed'
    def state(key):
        if key not in u or u[key] is None: return None
        x = u[key]
        return x if isinstance(x, int) and not isinstance(x, bool) and x >= 0 else 'malformed'
    c, w = state('cached_tokens'), state('cache_write_tokens')
    if 'malformed' in (c, w): return 'malformed'
    if c is not None and w is not None and c + w > i: return 'malformed'
    if c is not None and w is not None: pts = [(c, w)]
    elif c is not None: pts = [(c, 0), (c, i - c)]
    elif w is not None: pts = [(0, w), (i - w, w)]
    else: pts = [(0, 0), (i, 0), (0, i)]
    long_ = i > int(oa['long_input_threshold'])
    mi = Fraction(oa['long_input_multiplier']) if long_ else 1; mo = Fraction(oa['long_output_multiplier']) if long_ else 1
    m = Fraction(Decimal(mult))
    prices = [(i - cc - ww) * per_token(r['input'], m * mi) + cc * per_token(r['cached_input'], m * mi) + ww * per_token(r['cache_write'], m * mi)
              + o * per_token(r['output'], m * mo) for cc, ww in pts]
    return min(prices), max(prices)


def raw_usage_policy_checks(cid, a, rows, F):
    """Root P1/P2 assertions; auxiliary counts do not establish a billing report."""
    requests = [r for r in rows if r and r.get('event') == 'request']
    v = a['valuation']; k = a['key']
    for row in requests:
        fam = PROVIDERS.get(row.get('provider', 'openai'))
        billing = ('input_tokens', 'output_tokens', 'cached_tokens', 'cache_write_tokens') if fam == 'openai' else ('input_tokens', 'output_tokens', 'cache_read_input_tokens', 'cache_creation_input_tokens', 'ephemeral_5m_input_tokens', 'ephemeral_1h_input_tokens')
        u = row.get('usage')
        if not isinstance(u, dict):
            if u is not None and v.get('state') == 'valued': F.add(cid, f'{k}: non-object usage valued (UsageShape)')
            continue
        report = any(u.get(f) is not None for f in billing)
        if a['settlement']['decision'] not in ('rejected', 'conflicting'):
            for f in billing:
                if u.get(f) is not None and not (type(u[f]) is int and u[f] >= 0) and 'MalformedEvidence(' + f + ')' not in a['blocking']:
                    F.add(cid, f'{k}: malformed billing evidence lacks field diagnostic (P2)')

        contrary = ('cache_read_input_tokens', 'cache_creation_input_tokens', 'ephemeral_5m_input_tokens', 'ephemeral_1h_input_tokens') if fam == 'openai' else ('cached_tokens', 'cache_write_tokens')
        mismatch = fam is None or any(u.get(f) is not None for f in contrary)
        if mismatch and v.get('state') == 'valued': F.add(cid, f'{k}: incompatible provider billing schema priced (UsageSchemaMismatch)')
        if not report and len(requests) == 1 and v.get('state') == 'valued': F.add(cid, f'{k}: auxiliary-only usage cannot establish priceable report (P2)')
        if fam == 'anthropic' and v.get('state') == 'valued':
            tier = row.get('observed_tier')
            if tier is not None and tier != 'standard': F.add(cid, f'{k}: unestablished Anthropic tier priced (P2)')
        if fam == 'anthropic' and v.get('complete'):
            total = u.get('cache_creation_input_tokens')
            parts = [u.get(f) if u.get(f) is not None else 0 for f in ('ephemeral_5m_input_tokens', 'ephemeral_1h_input_tokens')]
            if type(total) is not int or any(type(x) is not int or x < 0 for x in parts) or sum(parts) > total:
                F.add(cid, f'{k}: inconsistent lifetime split cannot be complete (P2)')
        if v.get('complete'):
            status = row.get('http_status'); success = row.get('terminal') in ({'response.completed', 'response.done'} if fam == 'openai' else {'message_stop'})
            required = ('input_tokens', 'output_tokens') if fam == 'openai' else ('input_tokens', 'output_tokens', 'cache_read_input_tokens', 'cache_creation_input_tokens')
            counts = all(isinstance(u.get(f), int) and not isinstance(u.get(f), bool) and u[f] >= 0 for f in required)
            if not (type(status) is int and status == 200 and success and counts and not mismatch): F.add(cid, f'{k}: complete valuation lacks status+terminal+counts conjunction (P2)')
        for f in ('reasoning_tokens',) if fam == 'openai' else ():
            if u.get(f) is not None and not (type(u[f]) is int and u[f] >= 0):
                if 'MalformedEvidence(' + f + ')' not in a['blocking']: F.add(cid, f'{k}: malformed auxiliary evidence lacks field diagnostic (P2)')


def raw_settlement_checks(cid, c, a, rows, lines, rates, F):
    """Settlement conditions and valuation re-derived from raw producer rows where the contract makes them mechanical:
    status by path (5.7, A-06), model canonicalisation (MC-1..MC-3, A-05), and the OpenAI valuation range (V-4, A-04)."""
    raw_usage_policy_checks(cid, a, rows, F)
    k = a['key']; s_ = a['settlement']; d, b = s_.get('decision'), s_.get('basis')
    setl = [r for r in rows if r and isinstance(r.get('usd'), dict) and r.get('event') in ('request', 'upstream_error')]
    distinct_reservations = {lines[ref] for ref in a['evidence'] if isinstance(parse(lines.get(ref)), dict) and parse(lines[ref]).get('event') == 'usd_reserved'}
    if len(distinct_reservations) != 1 and setl:
        forbidden = set(s_.get('reasons', [])) & {'ReservedMismatch', 'Order', 'KeepAll'}
        if forbidden: F.add(cid, f'{k}: reservation-relative checks inapplicable without UniqueReservation: {sorted(forbidden)} (P3)')
        if d == 'valid': F.add(cid, f'{k}: valid settlement without UniqueReservation (P3)')

    if d == 'valid' and setl:
        row = setl[0]; st = row.get('http_status', '__absent__')
        isint = isinstance(st, int) and not isinstance(st, bool)
        ok = {'usage': isint and st == 200, 'error_status': isint and st != 200, 'reservation': (isint if row.get('event') == 'request' else st == '__absent__'),
              'upstream_error': row.get('event') == 'upstream_error' and st == '__absent__'}.get(b, True)
        if not ok: F.add(cid, f'{k}: Valid({b}) but the settlement row status {st!r} is not accepted on this path (5.7, A-06)')
        if b == 'usage':
            alias = k.split(':')[1].split('/')[0]
            launch = next((s2.get('launch') for s2 in c['inputs']['sessions'] if s2['alias'] == alias), None) or {}
            lm = launch.get('model') if isinstance(launch.get('model'), str) else ''
            prov, _, mid = lm.partition('/')
            if prov not in PROVIDERS or not mid or row.get('model') != mid or PROVIDERS.get(row.get('provider', 'openai')) != PROVIDERS.get(prov):
                F.add(cid, f'{k}: Valid(Usage) but the request model {row.get("model")!r} does not equal the canonical pinned id of {lm!r} (MC-1..MC-3, A-05)')
    # OpenAI valuation range from raw counts (single priced identity, recognised tier)
    tiers = rates['openai_api_rates.json'][1]['tier_multiplier']; models = rates['openai_api_rates.json'][1]['models']
    v = a['valuation']
    for row in rows:
        if not (row and row.get('event') == 'request' and row.get('provider', 'openai') != 'anthropic' and isinstance(row.get('usage'), dict)): continue
        u = row['usage']
        if not any(x is not None for x in u.values()): continue
        tier = row.get('observed_tier')
        ids = {x for x in (row.get('model'), row.get('response_model')) if isinstance(x, str) and x}
        if len(ids) != 1 or not (isinstance(tier, str) and tier in tiers): continue
        model = next(iter(ids))
        if model not in models: continue
        rng = openai_range(u, model, str(tiers[tier]), rates)
        if rng != 'malformed' and v.get('state') == 'not_reported':
            F.add(cid, f'{k}: priceable usage in its rows but valuation not_reported (V is independent of the decision, A-06/S06)')
        if rng == 'malformed':
            if v.get('state') == 'valued': F.add(cid, f'{k}: malformed cache counts but valued (V-4, A-04)')
        elif v.get('state') == 'valued' and len(v.get('identities', [])) == 1 and len([x for x in rows if x and x.get('event') == 'request']) == 1:
            if (nano(v['low']), nano(v['high'])) != rng: F.add(cid, f'{k}: valuation {(v["low"], v["high"])} != raw V-4 range {rng} (A-04)')


def obligation_interval(c, a, lines, order, lifecycle):
    """Contract 11.2 interval with P-2 neighbour bounds: (start_lo, end_hi) as stamps, None meaning unbounded; or
    'unplaceable'. Closed: [reservation, settlement]; Outstanding: [reservation, following dated retirement or open];
    None (unbudgeted, orphan): [first row, last row]."""
    alias = a['key'].split(':')[1].split('/')[0]
    if lifecycle.get(alias, {}).get('clock') == 'untrusted': return 'unplaceable'
    refs = sorted(a['evidence'], key=lambda r: (r.rsplit(':', 1)[0], int(r.rsplit(':', 1)[1])))
    def dated(ref):
        row = parse(lines.get(ref)); at = row.get('at') if row else None
        return at if isinstance(at, str) and STAMP.match(at) else None
    def bounds(ref):
        at = dated(ref)
        if at: return at, at
        src = ref.rsplit(':', 1)[0]; seq = order[src]; i = seq.index(ref)
        before = next((dated(r) for r in reversed(seq[:i]) if dated(r)), None)
        after = next((dated(r) for r in seq[i + 1:] if dated(r)), None)
        if before and after and before > after: return 'inverted', 'inverted'
        return before, after
    first_lo, _ = bounds(refs[0])
    _, last_hi = bounds(refs[-1])
    if 'inverted' in (first_lo, last_hi): return 'unplaceable'
    if a['obligation']['state'] == 'outstanding':
        sess = next(s for s in c['inputs']['sessions'] if s['alias'] == alias)
        rets = [r['at'] for r in sess['lifecycle'] if r['event'] == 'retire' and isinstance(r.get('at'), str) and STAMP.match(r['at'])
                and (first_lo is None or r['at'] >= first_lo)]
        return first_lo, (min(rets) if rets else None)          # open when no following dated retirement
    return first_lo, last_hi


def o7_candidates(c, e, a, lines, order, lifecycle, ref):
    """O-7 (1.2-s1): tree members whose window could contain the obligation interval; unplaceable keeps them all."""
    alias = a['key'].split(':')[1].split('/')[0]
    sid = next(s_['session_id'] for s_ in c['inputs']['sessions'] if s_['alias'] == alias)
    iv = obligation_interval(c, a, lines, order, lifecycle); out = []
    for o in e['objectives']:
        if sid not in o['tree_sessions']: continue
        if iv != 'unplaceable':
            lo, hi = iv
            closed = o['closed_at'] if closure_state(o, ref, c.get('profile')) == 'closed' else None
            if (hi is not None and hi < o['started_at']) or (closed and lo is not None and lo > closed): continue
        out.append(o['objective'])
    return sorted(out)


def admitting_objectives(c, e, a):
    alias = a['key'].split(':')[1].split('/')[0]
    sid = next(s_['session_id'] for s_ in c['inputs']['sessions'] if s_['alias'] == alias)
    return sorted({o['objective'] for o in e['objectives'] for ad in o['admissions'] if ad['session'] == sid})


def independent_low_assertions():
    """Read authored shards at consumption time, so partition checks need no merge."""
    root = os.path.abspath(os.path.join(os.path.dirname(__file__), '..'))
    paths = [os.path.join(root, 'oracle', 'R8-LOW-ASSERTIONS.json')]
    paths += sorted(glob.glob(os.path.join(root, 'oracle', 'r8-completion-*', 'LOW-ASSERTIONS.json')))
    claims = {}
    for path in paths:
        for claim in json.load(open(path))['assertions']:
            key = tuple(claim[k] for k in ('case_id', 'evaluation', 'objective', 'label'))
            if key in claims and claims[key]['expected_low'] != claim['expected_low']:
                raise ValueError('conflicting independent low assertions: ' + repr(key))
            claims[key] = claim
    return claims


def check_independent_low_assertion(cid, c, evaluation, objective, label, result, F):
    """Missing finite scopes fail; expected assertions never grant release."""
    assertion = r9_pilot.control_low_assertion(c, evaluation, objective, label) or independent_low_assertions().get((cid, evaluation, objective, label))
    if assertion is None:
        F.add(cid, f'{evaluation}/{objective}/{label}: independent low scope REQUIRED/UNMET'); return
    actual = hashlib.sha256(json.dumps(c['inputs'], sort_keys=True, separators=(',', ':')).encode()).hexdigest()
    if assertion['raw_input_sha256'] != actual:
        F.add(cid, 'independent low assertion raw input binding changed; qualification invalidated')
    elif result['low'] != assertion['expected_low']:
        F.add(cid, f"independent low assertion {evaluation}/{objective}/{label}: {result['low']} != {assertion['expected_low']} ({assertion['claim_id']})")


def check_accounting_references(c, F, pilot_binding=None):
    """Resolve supplied finite documents against this exact raw/case/unit/scope.

    Financial literals come from the separately authored shard, never from a
    generator. Draft2020-12 handles shape; this check handles wire references.
    """
    from jsonschema import Draft202012Validator
    root = str(r9_pilot.root())
    cid = c['case_id']; part = 'A' if cid.startswith(('P130-', 'P132-', 'P133-')) else 'B'
    rel = f'oracle/r8-completion-{part}/{cid}.bundle.json'
    path = os.path.join(root, pilot_binding['bundle_file']) if pilot_binding else os.path.join(root, rel)
    if not os.path.isfile(path): F.add(cid, 'supplied accounting bundle REQUIRED/UNMET'); return
    bundle = json.load(open(path)); docs = bundle['documents']
    if not pilot_binding: r9_pilot.case_claim(c, F, bundle)
    r9_pilot.reference_relations(bundle, F, cid)
    schema = json.load(open(os.path.join(root, 'spec/accounting.schema.json')))
    validator = Draft202012Validator(schema)
    by = {}
    for doc in docs:
        errors = list(validator.iter_errors(doc))
        if errors: F.add(cid, 'accounting schema: ' + errors[0].message); return
        if doc['id'] in by: F.add(cid, 'duplicate accounting document id'); return
        by[doc['id']] = doc
    inp = by.get('input:' + cid); result = by.get('result:' + cid)
    if not inp or inp['kind'] != 'accounting_input_proof' or not result or result['kind'] != 'accounting_result':
        F.add(cid, 'missing/interchanged accounting document kind'); return
    if result['input_proof_ref'] != inp['id'] or inp['lineage_id'] != cid or result['lineage_id'] != cid:
        F.add(cid, 'wrong accounting subject/lineage/input ref')
    digest = hashlib.sha256(json.dumps(c['inputs'], sort_keys=True, separators=(',', ':')).encode()).hexdigest()
    raw = {}
    for session in c['inputs']['sessions']:
        for turn, log in session['logs'].items():
            for pos, line in enumerate(log['lines'], 1):
                raw[f"{session['alias']}/{turn}:{pos}"] = (session['alias'] + '/' + turn, pos, hashlib.sha256((line if isinstance(line, str) else json.dumps(line, sort_keys=True, separators=(',', ':'))).encode()).hexdigest())
    origins = {o['origin_id']: o for o in inp['origins']}
    for oid, origin in origins.items():
        actual = raw.get(oid)
        if actual != (origin['source_origin_id'], origin['original_position'], origin['raw_bytes_sha256']) or origin['source_revision_ref'] != 'sha256:' + digest:
            F.add(cid, 'wrong raw origin/input digest binding: ' + oid)
    wire_schema = json.load(open(os.path.join(root, 'spec/wire.schema.json')))
    wire_validator = Draft202012Validator(wire_schema)
    for attempt in c['expected']['attempts']:
        wire = {k: v for k, v in attempt.items() if k in wire_schema['properties']}
        errors = list(wire_validator.iter_errors(wire))
        if errors: F.add(cid, 'wire schema: ' + errors[0].message)
    units = {u['unit_id']: u for u in result['units'] if u['state'] == 'active'}
    attempts = {a['key']: a for a in c['expected']['attempts']}
    if set(units) != set(attempts): F.add(cid, 'total active unit/attempt coverage mismatch')
    for key, attempt in attempts.items():
        if attempt.get('accounting_result_ref') != rel + '#' + result['id']:
            F.add(cid, 'missing/dangling/wrong-kind accounting_result_ref: ' + key)
        unit = units.get(key)
        if not unit: continue
        if set(unit['origin_refs']) != set(attempt['evidence']) or unit['exposure_ref'] != f'case:{cid}#attempt:{key}':
            F.add(cid, 'wrong unit/exposure/evidence reference: ' + key)
        if not set(unit['origin_refs']) <= set(origins): F.add(cid, 'unit refers to absent raw origin')
    all_units = {u['unit_id']: u for u in result['units']}
    validities = {v['obligation_id']: v for v in result['primary_validities']}
    valuations = {v['valuation_id']: v for v in result.get('valuation_records', [])}
    accepted = {v['result_unit_ref']: v for v in result['coverage_decisions'] if v['status'] == 'accepted'}
    for key, attempt in attempts.items():
        if attempt['exposure']['pool'] != 'pi_api' or attempt['exposure']['high']['kind'] != 'exact': continue
        coverage = accepted.get(key)
        if not coverage: F.add(cid, 'Exact wire unit lacks complete F witness: ' + key); continue
        members = set(coverage['monetary_origin_refs']) | set(coverage['anchor_origin_refs'])
        held = [all_units.get(k) for k in coverage['unit_refs']]
        if any(u is None or u['state'] != 'inactive' for u in held) or set().union(*(set(u['origin_refs']) for u in held if u)) != members or members != set(attempt['evidence']):
            F.add(cid, 'F witness lacks whole covered origin/unit set: ' + key)
        valuation = valuations.get(coverage['valuation_ref'])
        if not valuation or not valuation['complete'] or valuation['wire_ref'] != f'case:{cid}#attempt:{key}/valuation' or not attempt['valuation'].get('complete'):
            F.add(cid, 'F valuation wire/reference disagreement: ' + key)
        if coverage['route'] == 'budgeted':
            validity = validities.get(coverage.get('validity_ref'))
            if not validity or validity['validity_result'] != 'valid' or attempt['settlement']['decision'] != 'valid' or not attempt['replacement']['eligible'] or len(validity['distinct_reservation_refs']) != 1 or len(validity['distinct_settlement_refs']) != 1:
                F.add(cid, 'F validity wire/reference disagreement: ' + key)
        else:
            completions = {x['completion_id']: x for x in result.get('unbudgeted_completions', [])}
            applicability = {x['applicability_id']: x for x in result.get('unbudgeted_applicabilities', [])}
            completion = completions.get(coverage.get('completion_ref'), {})
            app = applicability.get(completion.get('applicability_ref'), {})
            if not completion or not app or attempt['settlement']['decision'] != 'not_applicable' or attempt['obligation']['state'] != 'none' or app.get('dollar_origin_refs'):
                F.add(cid, 'unbudgeted F applicability wire/reference disagreement: ' + key)
        for anchor in coverage['anchor_origin_refs']:
            if not any(anchor in (l['from_origin_ref'], l['to_origin_ref']) for l in result['definite_links']): F.add(cid, 'unlinked auxiliary anchor released')
    owners = {o['origin_ref']: o['active_high_unit_id'] for o in result['origins']}
    if set(owners) != set(origins): F.add(cid, 'total origin owner coverage mismatch')
    for key, unit in units.items():
        for oid in unit['origin_refs']:
            if owners.get(oid) != key: F.add(cid, 'switched origin owner: ' + oid)
    lows = {r['scope_ref']: r for r in result['low_rollups']}
    if len(lows) != len(result['low_rollups']): F.add(cid, 'duplicate low scope')
    claims = independent_low_assertions(); required = set()
    for ev in c['evaluations']:
        for obj in ev['expected']['objectives']:
            for label in ('owned_closed', 'owned_outstanding'):
                scope = f"{cid}/{ev['name']}/{obj['objective']}/{label}"; required.add(scope)
                rollup = obj[label]; low = lows.get(scope)
                if rollup.get('supported_low_evidence_ref') != rel + '#' + scope or low is None:
                    F.add(cid, 'missing/dangling/wrong-scope supported_low_evidence_ref: ' + scope); continue
                claim = r9_pilot.control_low_assertion(c, ev['name'], obj['objective'], label) or claims.get((cid, ev['name'], obj['objective'], label))
                if not claim: continue  # check_evaluation reports missing literal
                owned = sorted(at['attempt'] for at in ev['expected']['attribution'] if at['ownership'] == 'owned' and at['objective'] == obj['objective'] and at['attempt'] in attempts and (attempts[at['attempt']]['obligation']['state'] == 'outstanding') == (label == 'owned_outstanding') and attempts[at['attempt']]['exposure']['pool'] == 'pi_api')
                if sorted(claim.get('owned_unit_refs', [])) != owned: F.add(cid, 'independent owned scope membership mismatch: ' + scope)
                if low['selected_low'] != claim['expected_low'] or rollup['low'] != low['selected_low']:
                    F.add(cid, 'independent consistent-wrong low/result/rollup: ' + scope)
                for field in ('seed_origin_refs', 'alternative_origin_refs'):
                    if field not in claim or set(claim[field]) != set(low[field]) or not set(low[field]) <= set(origins):
                        F.add(cid, 'independent low membership mismatch: ' + scope + '/' + field)
                if low['basis'] != ('usage_valuation' if int(claim['expected_low']) else 'none'):
                    F.add(cid, 'low basis mismatch: ' + scope)
    if set(lows) != required: F.add(cid, 'total low scope coverage mismatch')


def check_evaluation(cid, c, e, keys, sessions, lines, order, F):
    ex, ee = c['expected'], e['expected']
    ref = c['inputs']['reference_time']
    objs = {o['objective']: o for o in e['objectives']}
    attempts = {a['key']: a for a in ex['attempts']}
    for at in ee['attribution']:
        if at['attempt'] not in keys: F.add(cid, f'{e["name"]}: attribution names unknown attempt {at["attempt"]}'); continue
        if at['ownership'] not in OWNERSHIP or at['placement'] not in PLACEMENTS: F.add(cid, f'{e["name"]}: bad attribution {at}')
        if at['ownership'] == 'owned' and at['objective'] not in objs: F.add(cid, f'{e["name"]}: owner {at["objective"]} not an objective')
        a = attempts[at['attempt']]
        if at['ownership'] == 'uncertain':
            if not at.get('candidates'): F.add(cid, f'{e["name"]}: uncertain without candidates (INV-T2)')
            miss = set(admitting_objectives(c, e, a)) - set(at.get('candidates', []))
            if miss: F.add(cid, f'{e["name"]}: uncertain attempt drops admitting objectives {sorted(miss)} (O-3a, R-08)')
        if at['ownership'] == 'unadmitted':
            if at.get('objective') is not None: F.add(cid, f'{e["name"]}: unadmitted attempt {at["attempt"]} names an owner (O-6)')
            if admitting_objectives(c, e, a): F.add(cid, f'{e["name"]}: unadmitted although an admission names its session (O-6)')
            want = o7_candidates(c, e, a, lines, order, ex['lifecycle'], ref)
            if sorted(at.get('candidates', [])) != want: F.add(cid, f'{e["name"]}: unadmitted candidates {at.get("candidates")} != O-7 {want} (R-12)')
    attributed = [a['attempt'] for a in ee['attribution']]
    for k in keys:
        if attributed.count(k) != 1: F.add(cid, f'{e["name"]}: attempt {k} attributed {attributed.count(k)} times')
    exp_objs = {o['objective']: o for o in ee['objectives']}
    for name in objs:
        if name not in exp_objs: F.add(cid, f'{e["name"]}: no expected result for objective {name}')
    spent = {}
    for at in ee['attribution']:
        if at['ownership'] == 'owned':
            x = att_exp(attempts[at['attempt']])
            if x: spent.setdefault(at['objective'], {}).setdefault(at['attempt'].split(':')[1].split('/')[0], []).append(x)
    for name, oe in exp_objs.items():
        o = objs.get(name)
        if o is None: F.add(cid, f'{e["name"]}: expected result for unknown objective {name}'); continue
        if oe['status'] not in ('established', 'not_established'): F.add(cid, 'bad status')
        for r in oe['reasons']:
            if r not in OBJ_REASONS and not re.fullmatch(r'MalformedEvidence\([A-Za-z_]+\)', r): F.add(cid, f'{e["name"]}/{name}: unknown reason {r}')
        if (oe['status'] == 'established') != (not oe['reasons']): F.add(cid, f'{e["name"]}/{name}: status/reasons disagree')
        owned = [at for at in ee['attribution'] if at['ownership'] == 'owned' and at['objective'] == name]
        want_provisional = financial_provisional(attempts, ee['attribution'], oe)
        if type(oe.get('provisional')) is not bool or oe['provisional'] != want_provisional: F.add(cid, f'{e["name"]}/{name}: provisional violates financial unresolved-billing predicate')
        closed = [att_exp(attempts[at['attempt']]) for at in owned if attempts[at['attempt']]['obligation']['state'] in ('closed', 'none') and att_exp(attempts[at['attempt']])]
        outst = [att_exp(attempts[at['attempt']]) for at in owned if attempts[at['attempt']]['obligation']['state'] == 'outstanding']
        for label, items in (('owned_closed', closed), ('owned_outstanding', outst)):
            want = hsum(items) if items else (0, 'exact', 0)
            if esum_of(oe[label])[1:] != want[1:]: F.add(cid, f'{e["name"]}/{name}: {label} high {esum_of(oe[label])[1:]} != sum of owned unit highs {want[1:]}')
            check_independent_low_assertion(cid, c, e['name'], name, label, oe[label], F)
        implied = set()
        for at in owned:
            implied |= set(attempts[at['attempt']]['blocking'])
            if at['placement'] == 'unplaceable': implied.add('PlacementUnplaceable')
            if at['placement'] in ('straddles_start', 'straddles_closure'): implied.add('PlacementStraddles')
            if at['placement'] in ('before', 'after'): implied.add('AttributionContradiction')
        for at in ee['attribution']:
            if at['ownership'] in ('uncertain', 'unadmitted') and name in at.get('candidates', []) and attempts[at['attempt']]['exposure']['pool'] == 'pi_api':
                implied.add('OwnershipUncertain' if at['ownership'] == 'uncertain' else 'HistoricalUnadmitted'); implied |= set(attempts[at['attempt']]['blocking'])
        scope = set(o['tree_sessions']) | {a['session'] for a in o['admissions']}
        scope_alias = {s_['alias'] for s_ in c['inputs']['sessions'] if s_['session_id'] in scope}
        for x in ex['non_attempt'] + ex['unreadable']:
            if x['evidence'].split('/')[0] in scope_alias: implied |= set(x.get('blocking', []))
        for x in ex['inventory']:
            if x['session'] in scope_alias: implied |= set(x.get('blocking', []))
        admitted_alias = {s_['alias'] for s_ in c['inputs']['sessions'] if s_['session_id'] in {a['session'] for a in o['admissions']}}
        for alias in admitted_alias:
            if ex['lifecycle'].get(alias, {}).get('reopened'): implied.add('ReopenedWithoutAdmission')     # R-10: persists after retirement
        for line_ in ('owned_closed', 'owned_outstanding', 'predecessor_obligations'):
            if oe[line_]['high']['kind'] == 'at_least': implied.add('UnresolvedExposure')
        if closure_state(o, ref, c.get('profile')) == 'claimed': implied.add('ClosureNotEstablished')
        adm_by_id = {a['admission_id']: a for a in o['admissions']}                      # commitments (12.3, Table G)
        if sorted(cm['admission_id'] for cm in oe['commitments']) != sorted(adm_by_id): F.add(cid, f'{e["name"]}/{name}: commitments do not list every admission')
        sessions_by_sid = {s_['session_id']: s_ for s_ in c['inputs']['sessions']}
        for cm in oe['commitments']:
            if cm['state'] not in COMMIT_STATES: F.add(cid, f'{e["name"]}/{name}: bad commitment state'); continue
            a = adm_by_id.get(cm['admission_id'])
            if not a: continue
            cap_rows = [r for r in a['rows'] if r.get('event') in ('pending', 'objective_admission')]
            cap = cap_rows[0].get('max_usd') if cap_rows else None
            alias = next((s_['alias'] for s_ in c['inputs']['sessions'] if s_['session_id'] == a['session']), None)
            sp = hsum(spent.get(name, {}).get(alias, [])) if spent.get(name, {}).get(alias) else (0, 'exact', 0)
            ok_cap = isinstance(cap, (int, float)) and not isinstance(cap, bool) and cap > 0
            launched = any(r.get('event') in ('launched', 'objective_admission') for r in a['rows'])
            released = any(r.get('event') == 'released' for r in a['rows'])
            life = ex['lifecycle'].get(alias) if alias else None
            capn = int(Decimal(str(cap)) * 10 ** 9) if ok_cap else None
            gate = {'qualified': False, 'reasons': ['NoGateEvidence']}; contradicted = False
            if not ok_cap: want = ('invalid', '0', 'exact')
            elif released and not launched: want = ('released', '0', 'exact')
            elif not launched:
                q, rs, contradicted = gate_qualification(c, o, a, 'bound', ref, ex['lifecycle'], sessions_by_sid, capn); gate = gate_projection(c, a, q, rs)
                want = ('pending', str(gate_qualification.ceiling if q else capn), 'exact' if q else 'at_least')
            else:
                rem = str(max(0, capn - sp[2]))
                retired = life is not None and life['state'] == 'retired'; reopened = life is not None and life['reopened']
                if retired:
                    q, rs, contradicted = gate_qualification(c, o, a, 'release', ref, ex['lifecycle'], sessions_by_sid); gate = gate_projection(c, a, q, rs)
                    want = ('retired', '0', 'exact') if q else ('retired_resumable', rem, 'at_least')
                elif reopened:
                    q, rs, contradicted = gate_qualification(c, o, a, 'release', ref, ex['lifecycle'], sessions_by_sid); gate = gate_projection(c, a, False, rs if not q else ['ReopenedWithoutAdmission'])
                    want = ('reopened', rem, 'at_least')
                else:
                    q, rs, contradicted = gate_qualification(c, o, a, 'bound', ref, ex['lifecycle'], sessions_by_sid, capn); gate = gate_projection(c, a, q, rs)
                    want = ('launched_open', str(max(0, gate_qualification.ceiling - sp[2])) if q else rem, 'exact' if q else 'at_least')
            got = (cm['state'], cm['committed'], cm.get('kind'))
            if got != want: F.add(cid, f'{e["name"]}/{name}: commitment {cm["admission_id"]} {got} != Table G {want} (A-2..A-5, OQ-1)')
            if cm.get('gate') != gate: F.add(cid, f'{e["name"]}/{name}: commitment {cm["admission_id"]} gate {cm.get("gate")} != G-1..G-7 {gate} (A9)')
            if cm.get('kind') == 'at_least': implied.add('UnenforcedCommitment')
            if cm['state'] == 'invalid': implied.add('CommitmentInvalid')
            if contradicted: implied.add('GateContradicted')
            # P5: a proxy high above a commitment is not a contradictory bill          # A-3: spend above cap
        if set(oe['reasons']) != implied:
            F.add(cid, f'{e["name"]}/{name}: reasons {sorted(oe["reasons"])} != implied by expected facts {sorted(implied)}')
    for name, oe in exp_objs.items():                                                  # predecessor lines (12.2, R-07)
        o = objs.get(name)
        if not o: continue
        if o.get('predecessor'):
            p = exp_objs.get(o['predecessor'])
            if not p: F.add(cid, f'predecessor {o["predecessor"]} has no expected result'); continue
            unres = [att_exp(attempts[at['attempt']]) for at in ee['attribution'] if at['ownership'] == 'owned' and at['objective'] == o['predecessor']
                     and att_exp(attempts[at['attempt']]) and attempts[at['attempt']]['exposure']['high']['kind'] == 'at_least']
            com = [cm for cm in p['commitments'] if cm['state'] in OPEN_COMMIT]
            kind = 'at_least' if unres or any(cm.get('kind') == 'at_least' for cm in com) else 'exact'
            want = (0, kind, sum(x[2] for x in unres) + sum(nano(cm['committed']) for cm in com))
            if esum_of(oe['predecessor_obligations']) != want:
                F.add(cid, f'{name}: predecessor_obligations {esum_of(oe["predecessor_obligations"])} != unresolved predecessor attempts + open commitments {want} (INV-A4, R-07)')
        elif esum_of(oe['predecessor_obligations']) != (0, 'exact', 0): F.add(cid, f'{name}: predecessor line without a predecessor')
    ad = ee.get('admission')                                                           # admission (12.4, 1.3-s1)
    req = e['admission_request']
    if ad:
        o = objs[req['objective']]; oe = exp_objs[req['objective']]
        if ad['new'] != req['new'] or ad['limit'] != o['limit'] or ad['objective'] != req['objective']: F.add(cid, f'{e["name"]}: admission record does not match the request/limit')
        ad_want, why = admission_decision(c, o, oe, req, ref)
        if ad['decision'] != ad_want: F.add(cid, f'{e["name"]}: admission {ad["decision"]} != 12.4 decision {ad_want} ({why})')


def financial_provisional(attempts, attribution, objective):
    """Bounded assertion over declared facts; no money reduction or lifecycle inference."""
    def unresolved(a): return a['obligation']['state'] == 'outstanding' or a['exposure'].get('high', {}).get('kind') == 'at_least'
    name = objective['objective']
    return (any(unresolved(attempts[t['attempt']]) for t in attribution if t['ownership'] == 'owned' and t['objective'] == name)
            or objective['predecessor_obligations']['high']['kind'] == 'at_least'
            or any(cm['state'] != 'invalid' and cm.get('kind') == 'at_least' for cm in objective['commitments'])
            or any(attempts[t['attempt']]['exposure']['pool'] == 'pi_api' and unresolved(attempts[t['attempt']]) for t in attribution
                   if t['ownership'] in ('uncertain', 'unadmitted') and name in t.get('candidates', [])))


def admission_decision(c, o, oe, req, ref):
    """Contract 12.4 (1.3-s1): closure, limit, known over-limit (OQ-2), accounting, headroom."""
    cs = closure_state(o, ref, c.get('profile'))
    if cs == 'closed': return 'refused', 'established closure (OQ-4)'
    if cs == 'claimed': return 'not_established', 'claimed closure not established (OQ-4)'
    lim = o['limit']
    if not (isinstance(lim, str) and re.fullmatch(r'\d+', lim)): return 'not_established', 'limit not established'
    known = sum(nano(oe[l]['high']['nano']) for l in ('owned_closed', 'owned_outstanding', 'predecessor_obligations')) + \
        sum(nano(cm['committed']) for cm in oe['commitments'] if cm['state'] != 'invalid') + nano(req['new'])
    if known > nano(lim): return 'refused', f'known {known} > limit {lim} (OQ-2)'
    if oe['status'] == 'not_established': return 'not_established', 'known within limit, unknown may exceed'
    return 'admitted', 'headroom'


def corpus_cases(corpus):
    out = []
    for p in sorted(glob.glob(os.path.join(corpus, 'cases', '*.json'))):
        c = json.load(open(p))
        if os.path.basename(p)[:-5] != c.get('case_id'): print(f'FAIL {p}: file name != case_id')
        out.append(c)
    return out


def first_decision(c): return (c['evaluations'][0]['expected'].get('admission') or {}).get('decision')


def valid_admitted(cases, profile=None):
    return [c for c in cases if c['role'] in ('valid_control', 'control') and first_decision(c) == 'admitted' and not any(a['blocking'] for a in c['expected']['attempts'])
            and (profile is None or c.get('profile') == profile)]


def check_corpus(corpus, rates_dir, partition=None):
    rates = load_rates(rates_dir); F = Failures(); cases = corpus_cases(corpus)
    if partition: cases = [c for c in cases if ('A' if c['case_id'].startswith(('P130-', 'P132-', 'P133-')) else 'B') == partition]
    ids = [c['case_id'] for c in cases]
    for i in {x for x in ids if ids.count(x) > 1}: F.add(i, 'duplicate case id')
    by = {c['case_id']: c for c in cases}
    seen_inputs = {}
    for c in cases:
        try:
            check_case(c, rates, F)
            check_accounting_references(c, F)
        except (KeyError, TypeError, ValueError, StopIteration) as e: F.add(c.get('case_id', '?'), f'structure error: {type(e).__name__} {e}')
        for ctl in c.get('controls', []):
            if not partition and ctl not in by: F.add(c['case_id'], f'control {ctl} does not exist')
        dig = hashlib.sha256(json.dumps([c['inputs'], [(e['objectives'], e['admission_request']) for e in c['evaluations']]], sort_keys=True).encode()).hexdigest()
        if dig in seen_inputs:
            other = by[seen_inputs[dig]]
            same = json.dumps([other['expected'], [e['expected'] for e in other['evaluations']]], sort_keys=True) == \
                json.dumps([c['expected'], [e['expected'] for e in c['evaluations']]], sort_keys=True)
            F.add(c['case_id'], f'same inputs as {seen_inputs[dig]} ({"duplicate" if same else "CONFLICTING expectations"})')
        seen_inputs[dig] = c['case_id']
        h = c.get('historical') or {}
        if 'expected' in h: F.add(c['case_id'], 'historical block contains normative fields')
        a134 = h.get('a134_086bdec1')
        if a134 and first_decision(c) and a134.get('decision') != first_decision(c) and not c.get('rationale'):
            F.add(c['case_id'], 'historical A1.3.4 decision differs from the normative one without a rationale')
        pair = c.get('evidence_pair')
        if pair and not partition:                                                     # INV-A5 (R-11)
            other = by.get(pair.get('other'))
            if not other or pair.get('relation') not in ('subset', 'superset') or not isinstance(pair.get('permitted_release'), bool):
                F.add(c['case_id'], 'bad evidence_pair'); continue
            more, less = (other, c) if pair['relation'] == 'subset' else (c, other)
            lm = {s['alias']: s for s in more['inputs']['sessions']}; ll = {s['alias']: s for s in less['inputs']['sessions']}
            if not set(ll) <= set(lm): F.add(c['case_id'], 'evidence_pair: sessions are not a subset'); continue
            def contained(small, big): return all(small.count(x) <= big.count(x) for x in small)
            if not all(contained(ll[a_]['lifecycle'], lm[a_]['lifecycle']) and all(t in lm[a_]['logs'] and contained(log['lines'], lm[a_]['logs'][t]['lines'])
                                                                                     for t, log in ll[a_]['logs'].items()) for a_ in ll):
                F.add(c['case_id'], 'evidence_pair: the smaller case holds evidence the larger one lacks, so it is not a subset (INV-A5)'); continue
            if pair['permitted_release']:
                gated = more.get('profile') == 'future_qualified_gate' and all(cm.get('gate', {}).get('qualified') for e_ in more['evaluations']
                                                                                for o_ in e_['expected']['objectives'] for cm in o_['commitments'] if cm['state'] == 'retired')
                if not gated or not any(cm['state'] == 'retired' for e_ in more['evaluations'] for o_ in e_['expected']['objectives'] for cm in o_['commitments']):
                    F.add(c['case_id'], 'evidence_pair: a permitted release needs a retirement with a qualified gate (A5, OQ-1)')
                logs_same = all(lm[a_]['logs'] == ll[a_]['logs'] for a_ in ll)
                extra = [r for a_ in ll for r in lm[a_]['lifecycle'] if r not in ll[a_]['lifecycle']]
                if not logs_same or not extra or any(r['event'] != 'retire' or not r.get('at') for r in extra):
                    F.add(c['case_id'], 'evidence_pair: a permitted release must add only dated retire rows (A5 exception)')
            elif RANK[first_decision(more)] < RANK[first_decision(less)]:
                F.add(c['case_id'], f'evidence_pair: added non-settlement evidence turned a denial into admission ({first_decision(less)} -> {first_decision(more)}) (INV-A5)')
    if partition: return F, cases
    total_scopes = sum(2 * len(e['expected']['objectives']) for c in cases for e in c['evaluations'])
    if len(cases) != 175 or total_scopes != 374: F.add('coverage', f'total complete corpus REQUIRED175/374 got{len(cases)}/{total_scopes}')
    for inv in INVARIANTS:
        if not any(inv in c['invariants'] for c in cases): F.add('coverage', f'invariant {inv} covered by no case')
    for cl in CLASSES:
        if not any(c['class'] == cl for c in cases): F.add('coverage', f'no case of class {cl}')
    va = valid_admitted(cases)
    blocked = [c for c in cases if any((e['expected'].get('admission') or {}).get('decision') == 'not_established' for e in c['evaluations'])]
    if len(va) < MIN_VALID_ADMITTED: F.add('coverage', f'only {len(va)} admitted valid controls (< {MIN_VALID_ADMITTED})')
    for prof, mn in MIN_ADMITTED.items():
        n = len(valid_admitted(cases, prof))
        if n < mn: F.add('coverage', f'profile {prof}: only {n} admitted valid controls (< {mn})')
    metered = [c for c in valid_admitted(cases, 'future_qualified_gate') if any(a['exposure'].get('pool') == 'pi_api' for a in c['expected']['attempts'])]
    if len(metered) < MIN_ADMITTED['future_qualified_gate']: F.add('coverage', f'only {len(metered)} future-gated admitted controls with metered attempts')
    if len(blocked) < MIN_BLOCKED: F.add('coverage', f'only {len(blocked)} blocking cases (< {MIN_BLOCKED})')
    for c in cases:
        if c.get('twin_of'):
            t_ = by.get(c['twin_of'])
            if not t_: F.add(c['case_id'], 'twin_of names no case'); continue
            norm = lambda x: sorted(x['inputs']['sessions'], key=lambda s_: s_['alias'])
            if norm(c) != norm(t_) or canon(twin_projection(c['expected'])) != canon(twin_projection(t_['expected'])) or canon(twin_projection([e['expected'] for e in c['evaluations']])) != canon(twin_projection([e['expected'] for e in t_['evaluations']])):
                F.add(c['case_id'], f'twin of {c["twin_of"]} differs beyond presentation order (INV-F8)')
    if not [c for c in cases if len(c['evaluations']) > 1]: F.add('coverage', 'no multi-window case (INV-F7)')
    return F, cases


# ------------------------------------------------------------------------------------------------ revision log
ABSENT = '__absent__'


def _locate(c, field):
    """Container and key for a revision field path (revise_rev2.py, revise_rev3.py)."""
    if field.startswith('attempts['):
        key, rest = field[len('attempts['):].split('].', 1)
        node = next(a for a in c['expected']['attempts'] if a['key'] == key)
    elif field.startswith('lifecycle['):
        alias, rest = field[len('lifecycle['):].split('].', 1)
        node = c['expected']['lifecycle'][alias]
    elif '/' not in field:                                    # top-level case fields (revision 4: profile, evidence_pair)
        node, rest = c, field
    else:
        ev_name, rest = field.split('/', 1)
        ee = next(e for e in c['evaluations'] if e['name'] == ev_name)['expected']
        if rest.startswith('attribution['):
            key, rest = rest[len('attribution['):].split('].', 1)
            node = next(at for at in ee['attribution'] if at['attempt'] == key)
        elif rest.startswith('admission.'):
            node, rest = ee['admission'], rest[len('admission.'):]
        else:
            obj, rest = rest.split('.', 1)
            node = next(o for o in ee['objectives'] if o['objective'] == obj)
            m = re.match(r'commitments\[([^\]]+)\]\.(.*)$', rest)
            if m: node, rest = next(cm for cm in node['commitments'] if cm['admission_id'] == m.group(1)), m.group(2)
    parts = rest.split('.')
    for p_ in parts[:-1]: node = node[p_]
    return node, parts[-1]


R5_ADDED = ('contract', 'revisions_r8')


def check_revisions(corpus, previous):
    """Undo every revision-6 entry of each case and require the revision-5 case back exactly (apart from the contract
    string, the revision-6 log and an appended rationale). Added cases must be marked added_in 1.5-s1. Historical
    blocks and the earlier logs must be unchanged."""
    F = Failures(); prev = {c['case_id']: c for c in corpus_cases(previous)}; cur = corpus_cases(corpus)
    for c in cur:
        cid = c['case_id']
        if cid not in prev:
            if c.get('added_in') != '1.6-s1': F.add(cid, 'not in the previous corpus and not marked added_in 1.6-s1')
            continue
        p = prev[cid]
        if p.get('contract') != PREVIOUS_CONTRACT: F.add(cid, 'previous case is not revision 7')
        u = copy.deepcopy(c)
        for r in reversed(u.get('revisions_r8', [])):
            try:
                node, k = _locate(u, r['field'])
                if node.get(k, ABSENT) != r['to']: F.add(cid, f'revision {r["field"]}: current value is not its logged "to"')
                if r['from'] == ABSENT: node.pop(k, None)
                else: node[k] = r['from']
            except (StopIteration, KeyError, ValueError) as e_: F.add(cid, f'revision {r["field"]} does not resolve: {e_}')
        for k in R5_ADDED: u.pop(k, None)
        q = {k: v for k, v in p.items() if k not in R5_ADDED}
        if canon(u) != canon(q):
            F.add(cid, f'differs from the previous case beyond its logged revisions in {sorted(k for k in set(u) | set(q) if canon(u.get(k)) != canon(q.get(k)))}')
        if canon(c['historical']) != canon(p['historical']): F.add(cid, 'historical outcome changed')
        if p.get('rationale') and not (c.get('rationale') or '').startswith(p['rationale']): F.add(cid, 'previous rationale not preserved')
    curids = {c['case_id'] for c in cur}
    for cid in prev:
        if cid not in curids: F.add(cid, 'previous case missing from the revised corpus')
    fp = {f for c in prev.values() for f in c['findings']}; fc = {f for c in cur for f in c['findings']}
    if fp - fc: F.add('coverage', f'findings lost: {sorted(fp - fc)}')
    return F, len(prev), len(cur), len(fp), len(fc)


# ------------------------------------------------------------------------------------------------ grading result sets
def expected_results(cases):
    """The corpus's own expectations in result-set form (one entry per evaluation)."""
    out = {}
    for c in cases:
        ex = c['expected']
        # each evaluation gets its own copy: a shared `attempts` object would hide window-dependent defects (P1.30 fix)
        out[c['case_id']] = {'evaluations': {e['name']: copy.deepcopy({'attempts': ex['attempts'], 'non_attempt': ex['non_attempt'], 'unreadable': ex['unreadable'],
                                                                        'inventory': ex['inventory'], 'lifecycle': ex['lifecycle'], **e['expected']})
                                              for e in c['evaluations']}}
    return out


def twin_projection(value):
    """Case-local locator strings differ for presentation twins.

    Actual references are resolved separately before this financial/presentation
    comparison. Strip only these locators, preserving every financial field.
    """
    if isinstance(value, dict): return {k: twin_projection(v) for k, v in value.items() if k not in ('accounting_result_ref', 'supported_low_evidence_ref')}
    if isinstance(value, list): return [twin_projection(v) for v in value]
    return value


def canon(x):
    if isinstance(x, dict): return {k: canon(v) for k, v in sorted(x.items())}
    if isinstance(x, list): return sorted((canon(v) for v in x), key=lambda v: json.dumps(v, sort_keys=True))
    return x


GRADED = ('attempts', 'non_attempt', 'unreadable', 'inventory', 'lifecycle', 'attribution', 'objectives', 'admission')
S2A_GRADED = ('attempts',)


def s2a_subset(cases):
    """P6 input-only current-producer scope; missing-launch dollar evidence stays an unresolved negative control.
    Selection does not establish E admissibility, correlation, lane identity or producer reachability."""
    out = []
    for c in cases:
        if c.get('profile') != 'current_producer': continue
        selected = False
        for session in c['inputs']['sessions']:
            launch = session.get('launch')
            if launch is not None and launch.get('auth_lane') != 'api': continue
            for log in session.get('logs', {}).values():
                for line in log['lines']:
                    row = parse(line)
                    if isinstance(row, dict) and (row.get('event') == 'usd_reserved' or 'usd' in row): selected = True
        if selected: out.append(c)
    return out


def grade(cases, results, fields=GRADED):
    """Exact equality per graded field: a result more restrictive than the expectation differs like any other."""
    F = Failures(); diff_fields = {}
    for c in cases:
        r = results.get(c['case_id'])
        if r is None: F.add(c['case_id'], 'no result'); continue
        exp = expected_results([c])[c['case_id']]['evaluations']
        evs = r.get('evaluations', {})
        first = None
        for name, want in exp.items():
            got = evs.get(name)
            if got is None: F.add(c['case_id'], f'{name}: no result'); continue
            for f in fields:
                if canon(got.get(f)) != canon(want.get(f)): F.add(c['case_id'], f'{name}: {f} differs'); diff_fields[f] = diff_fields.get(f, 0) + 1
            facts = canon([got.get('attempts'), got.get('non_attempt'), got.get('unreadable')])
            if first is None: first = facts
            elif facts != first: F.add(c['case_id'], f'{name}: attempt facts change with the objective window (INV-F7)'); diff_fields['window'] = diff_fields.get('window', 0) + 1
    return F, diff_fields


# ------------------------------------------------------------------------------------------------ self-tests
def walk_money(obj, fn):
    if isinstance(obj, dict):
        for k, v in list(obj.items()):
            if k in ('nano', 'low', 'high', 'committed') and isinstance(v, str) and re.fullmatch(r'-?\d+', v): obj[k] = fn(v)
            else: walk_money(v, fn)
    elif isinstance(obj, list):
        for v in obj: walk_money(v, fn)


def defects(cases):
    """Defective result sets, each with the graded field it must be caught in (its intended reason)."""
    base = expected_results(cases)
    by = {c['case_id']: c for c in cases}
    def each(fn):
        d = copy.deepcopy(base)
        for cid, r in d.items():
            for name, ev in r['evaluations'].items(): fn(cid, name, ev)
        return d
    def restatus(o): o['status'] = 'not_established' if o['reasons'] else 'established'
    def accept(cid, name, ev):
        if ev.get('admission'): ev['admission']['decision'] = 'admitted'
        for o in ev['objectives']: o['status'], o['reasons'] = 'established', []
        for a in ev['attempts']: a['blocking'] = []
    def reject(cid, name, ev):
        if ev.get('admission'): ev['admission']['decision'] = 'not_established'
        for o in ev['objectives']: o['status'] = 'not_established'; o['reasons'] = sorted(set(o['reasons']) | {'UnresolvedExposure'})
    def zero(cid, name, ev): walk_money(ev, lambda v: '0')
    def unknown(cid, name, ev):
        for a in ev['attempts']:
            if a['exposure'].get('pool') == 'pi_api': a['exposure']['high']['kind'] = 'at_least'
    def drop_malformed(cid, name, ev):
        ev['unreadable'] = []; ev['non_attempt'] = [x for x in ev['non_attempt'] if not x.get('blocking')]
        ev['attempts'] = [a for a in ev['attempts'] if a['correlation']['kind'] not in ('unattributed', 'ambiguous')]
    def settle_to_res(cid, name, ev):
        for a in ev['attempts']:
            s = a['settlement']
            if s.get('decision') == 'valid' and s.get('basis') == 'usage' and 'reserved' in s:
                a['exposure']['low'] = {'nano': '0', 'basis': 'none'}; a['exposure']['high'] = {'kind': 'exact', 'nano': s['reserved'], 'basis': 'proxy_reservation'}
    def no_diag(cid, name, ev):
        for a in ev['attempts']: a['diagnostics'] = []
    first_names = {cid: next(iter(r['evaluations'])) for cid, r in base.items()}
    def window_money(cid, name, ev):
        if name != first_names[cid] and ev['attempts']:
            for a in ev['attempts']:
                if a['exposure'].get('pool') == 'pi_api':
                    a['exposure']['high']['nano'] = str(int(a['exposure']['high']['nano']) + 1); break
    def unknown_as_exact(cid, name, ev):
        for a in ev['attempts']:
            if a['exposure'].get('pool') == 'pi_api': a['exposure']['high']['kind'] = 'exact'
    def unresolved_as_zero(cid, name, ev):
        for a in ev['attempts']:
            if a['exposure'].get('pool') == 'pi_api' and a['exposure']['high']['kind'] == 'at_least': a['exposure']['high'] = {'kind': 'exact', 'nano': '0', 'basis': 'none'}
    def lose_obligations(cid, name, ev):
        for a in ev['attempts']:
            if a['obligation']['state'] == 'outstanding': a['obligation'] = {'state': 'closed', 'amount_source': 'reservation_confirmed'}
        for o in ev['objectives']: o['owned_outstanding'] = {'low': '0', 'high': {'kind': 'exact', 'nano': '0'}}
    def double_count(cid, name, ev):
        for o in ev['objectives']:
            oc = o['owned_closed']; oc['low'] = str(2 * int(oc['low'])); oc['high']['nano'] = str(2 * int(oc['high']['nano']))
    def early_release(cid, name, ev):
        for o in ev['objectives']:
            for cm in o['commitments']:
                if cm['state'] in OPEN_COMMIT: cm['state'], cm['committed'], cm['kind'] = 'released', '0', 'exact'
    def diagnostics_move_money(cid, name, ev):
        for a in ev['attempts']:
            if a['diagnostics'] and a['exposure'].get('pool') == 'pi_api': a['exposure']['low'] = {'nano': '0', 'basis': 'none'}; a['exposure']['high']['nano'] = str(int(a['exposure']['high']['nano']) + 1)
    def unadmitted_ignored(cid, name, ev):
        for at in ev['attribution']:
            if at['ownership'] == 'unadmitted': at['ownership'] = 'unowned'; at['candidates'] = []
        for o in ev['objectives']: o['reasons'] = [r for r in o['reasons'] if r != 'HistoricalUnadmitted']; restatus(o)
    def forwarding_unknown(cid, name, ev):
        for a in ev['attempts']: a['forwarding'] = 'unknown'
    # revision-3 defect classes (P1.30)
    def lost_predecessor(cid, name, ev):            # R-07: closed-but-unresolved predecessor spend dropped from the successor line
        for o in ev['objectives']:
            pl = o['predecessor_obligations']
            if pl['high']['nano'] != '0' or pl['high']['kind'] == 'at_least':
                o['predecessor_obligations'] = {'low': '0', 'high': {'kind': 'exact', 'nano': '0'}}
                o['reasons'] = [r for r in o['reasons'] if r != 'UnresolvedExposure']; restatus(o)
                if ev.get('admission') and ev['admission']['objective'] == o['objective'] and o['status'] == 'established': ev['admission']['decision'] = 'admitted'
    def erased_contradiction(cid, name, ev):        # R-08: admitting objectives filtered out of an Uncertain attempt's candidates
        for at in ev['attribution']:
            if at['ownership'] == 'uncertain': at['candidates'] = []
        for o in ev['objectives']: o['reasons'] = [r for r in o['reasons'] if r != 'OwnershipUncertain']; restatus(o)
    def closure_after_headroom(cid, name, ev):      # Q11/R-09: closure evaluated after accounting and headroom
        c = by[cid]; e = next(x for x in c['evaluations'] if x['name'] == name)
        if ev.get('admission'):
            o = next(x for x in e['objectives'] if x['objective'] == ev['admission']['objective'])
            if closure_state(o, c['inputs']['reference_time'], c.get('profile')) == 'closed':
                oe = next(x for x in ev['objectives'] if x['objective'] == o['objective'])
                ev['admission']['decision'] = 'not_established' if oe['status'] == 'not_established' else 'admitted'
    def timestamp_only(cid, name, ev):              # R-12: candidates from isolated row stamps
        c = by[cid]; e = next(x for x in c['evaluations'] if x['name'] == name)
        atts = {a['key']: a for a in ev['attempts']}; lines, _ = source_lines(c)
        for at in ev['attribution']:
            if at['ownership'] != 'unadmitted': continue
            stamps = [parse(lines.get(r)).get('at') for r in atts[at['attempt']]['evidence'] if parse(lines.get(r)) and isinstance(parse(lines.get(r)).get('at'), str)]
            if stamps: at['candidates'] = [o['objective'] for o in e['objectives'] if o['objective'] in at['candidates'] and max(stamps) >= o['started_at']]
        for o in ev['objectives']:
            if not any(o['objective'] in at.get('candidates', []) for at in ev['attribution'] if at['ownership'] == 'unadmitted'):
                o['reasons'] = [r for r in o['reasons'] if r != 'HistoricalUnadmitted']; restatus(o)
    def false_finite_tier(cid, name, ev):           # R-03: a missing or unrecognised tier priced as a finite range
        for a in ev['attempts']:
            v = a['valuation']
            if v.get('state') == 'unpriceable' and any(r.startswith('TierUnestablished(') for r in v.get('reasons', [])):
                a['exposure']['high']['kind'] = 'exact'; a['billing'] = 'established'; a['blocking'] = [b for b in a['blocking'] if b != 'UnresolvedExposure']
    def cap_as_bound(cid, name, ev):                # R-04: an unenforced cap counted as a finite commitment
        for o in ev['objectives']:
            hit = False
            for cm in o['commitments']:
                if cm.get('kind') == 'at_least': cm['kind'] = 'exact'; hit = True
            if hit: o['reasons'] = [r for r in o['reasons'] if r != 'UnenforcedCommitment']; restatus(o)
    def billing_from_obligation(cid, name, ev):     # R-07/F12: billing derived from the obligation state
        for a in ev['attempts']:
            if a.get('billing') != 'not_applicable': a['billing'] = 'established' if a['obligation']['state'] == 'closed' else 'unresolved'
    def retired_released(cid, name, ev):            # OQ-1: a current-producer retirement treated as a release
        if by[cid].get('profile') != 'current_producer': return
        for o in ev['objectives']:
            for cm in o['commitments']:
                if cm['state'] == 'retired_resumable': cm.update(state='retired', committed='0', kind='exact')
    def self_asserted_trusted(cid, name, ev):       # OQ-1: an unqualified gate claim trusted
        for o in ev['objectives']:
            for cm in o['commitments']:
                if cm.get('gate', {}).get('reasons') and cm['gate']['reasons'] != ['NoGateEvidence']: cm['gate'] = {'qualified': True, 'reasons': []}
    def gate_bound_ignored(cid, name, ev):          # OQ-1/OQ-3: a qualified billing bound treated as unenforced
        for o in ev['objectives']:
            for cm in o['commitments']:
                if cm['state'] == 'launched_open' and cm.get('kind') == 'exact': cm['kind'] = 'at_least'
    def known_over_not_refused(cid, name, ev):      # OQ-2: refusal by known amounts reported as not_established
        c = by[cid]; e = next(x for x in c['evaluations'] if x['name'] == name)
        if ev.get('admission') and ev['admission']['decision'] == 'refused':
            o = next(x for x in e['objectives'] if x['objective'] == ev['admission']['objective'])
            if closure_state(o, c['inputs']['reference_time'], c.get('profile')) == 'open': ev['admission']['decision'] = 'not_established'
    def unknown_below_refused(cid, name, ev):       # OQ-2: unknown exposure below the limit reported as refusal
        if ev.get('admission') and ev['admission']['decision'] == 'not_established': ev['admission']['decision'] = 'refused'
    def release_incomplete(cid, name, ev):          # A-01: an ineligible usage settlement replaces the reservation
        for a in ev['attempts']:
            if a.get('replacement', {}).get('reason') in ('UsageIncomplete', 'UsageUnpriceable'):
                a['replacement'] = {'eligible': True, 'reason': 'UsageBound'}; a['obligation'] = {'state': 'closed', 'amount_source': 'settlement_amount'}
                v = a['valuation']; vh = v.get('high') or v.get('priced_high') or a['settlement'].get('charged', '0')
                a['exposure']['high'] = {'kind': 'at_least', 'nano': str(max(int(vh), int(a['settlement'].get('charged', '0')))), 'basis': 'usage_valuation'}
    def discard_usage(cid, name, ev):               # A-02: error status drops its supported usage
        for a in ev['attempts']:
            if a['settlement'].get('basis') == 'error_status' and a['valuation'].get('state') == 'valued':
                a['exposure'] = {'pool': 'pi_api', 'low': {'nano': '0', 'basis': 'none'}, 'high': {'kind': 'at_least', 'nano': a['settlement']['reserved'], 'basis': 'proxy_reservation'}}
    def finite_from_incomplete(cid, name, ev):      # A-01/FB-1: incomplete or unpriceable usage made a finite high
        for a in ev['attempts']:
            if a.get('replacement', {}).get('reason') in ('UsageIncomplete', 'UsageUnpriceable'): a['exposure']['high']['kind'] = 'exact'; a['billing'] = 'established'
    def fake_bound_qualified(cid, name, ev):        # A-03: an unbound or mis-bound billing gate treated as qualified
        for o in ev['objectives']:
            for cm in o['commitments']:
                if any(r.startswith('GateBound') for r in cm.get('gate', {}).get('reasons', [])): cm['gate'] = {'qualified': True, 'reasons': []}; cm['kind'] = 'exact'
    def reservations_summed(cid, name, ev):         # A-09: conflicting reservations summed
        if cid == 'R04-reused-reservation-id':
            for a in ev['attempts']: a['exposure']['high']['nano'] = '900000000'
    def more_restrictive(cid, name, ev):            # a result stricter than the expectation must still fail
        for a in ev['attempts']:
            if a['exposure'].get('pool') == 'pi_api' and 'UnresolvedExposure' not in a['blocking']: a['blocking'] = sorted(a['blocking'] + ['UnresolvedExposure'])
    return {'always_accept': (each(accept), 'admission'), 'always_reject': (each(reject), 'admission'), 'always_zero': (each(zero), 'attempts'),
            'always_unknown': (each(unknown), 'attempts'), 'drop_malformed': (each(drop_malformed), 'attempts'),
            'settlement_replaced_by_reservation': (each(settle_to_res), 'attempts'), 'suppress_diagnostics': (each(no_diag), 'attempts'),
            'money_changes_with_window': (each(window_money), 'window'), 'unknown_as_exact': (each(unknown_as_exact), 'attempts'),
            'unresolved_as_zero': (each(unresolved_as_zero), 'attempts'), 'lost_obligations': (each(lose_obligations), 'objectives'),
            'double_count_closed': (each(double_count), 'objectives'), 'early_release': (each(early_release), 'objectives'),
            'diagnostics_move_money': (each(diagnostics_move_money), 'attempts'), 'unadmitted_ignored': (each(unadmitted_ignored), 'attribution'),
            'forwarding_always_unknown': (each(forwarding_unknown), 'attempts'),
            'lost_predecessor_obligations': (each(lost_predecessor), 'objectives'), 'erased_admission_contradiction': (each(erased_contradiction), 'attribution'),
            'closure_precedence': (each(closure_after_headroom), 'admission'), 'timestamp_only_placement': (each(timestamp_only), 'attribution'),
            'false_finite_missing_tier': (each(false_finite_tier), 'attempts'), 'cap_as_bound': (each(cap_as_bound), 'objectives'),
            'billing_from_obligation': (each(billing_from_obligation), 'attempts'), 'more_restrictive_than_expected': (each(more_restrictive), 'attempts'),
            'current_retirement_released': (each(retired_released), 'objectives'), 'self_asserted_gate_trusted': (each(self_asserted_trusted), 'objectives'),
            'qualified_billing_bound_ignored': (each(gate_bound_ignored), 'objectives'), 'known_over_limit_not_refused': (each(known_over_not_refused), 'admission'),
            'unknown_below_limit_refused': (each(unknown_below_refused), 'admission'),
            'incomplete_usage_releases_reservation': (each(release_incomplete), 'attempts'), 'error_status_discards_usage': (each(discard_usage), 'attempts'),
            'finite_high_from_incomplete_pricing': (each(finite_from_incomplete), 'attempts'), 'fake_billing_bound_qualified': (each(fake_bound_qualified), 'objectives'),
            'conflicting_reservations_summed': (each(reservations_summed), 'attempts')}


def selftest(corpus, rates_dir):
    F, cases = check_corpus(corpus, rates_dir)
    report = {'corpus_failures': len(F)}
    ok = not F
    good, _ = grade(cases, expected_results(cases))
    report['correct_result_set_failures'] = len(good)
    ok &= not good
    for name, (d, field) in defects(cases).items():
        fails, fields = grade(cases, d)
        caught = len(fails) > 0 and fields.get(field, 0) > 0
        report[name] = {'differences': len(fails), 'intended_field': field, 'intended_field_differences': fields.get(field, 0), 'caught_for_intended_reason': caught}
        ok &= caught
    print(json.dumps(report, indent=1, sort_keys=True))
    return ok


def check_exact_raw_assertions(raw_bytes, supplied, expected):
    """Consume finite literal expected records. No expressions or financial evaluation.

    Source/provenance bytes, supplied computed records and independently authored
    expected assertions are distinct inputs. A pass proves only named assertions.
    """
    F = Failures()
    if supplied.get('kind') in ('accounting_expected_assertion', 'accounting_input_proof'):
        F.add('assertions', 'computed result kind interchange'); return F
    try: raw_value = json.loads(raw_bytes)
    except (ValueError, TypeError): raw_value = None
    if isinstance(raw_value, dict) and raw_value.get('kind') in ('accounting_expected_assertion', 'accounting_result'):
        F.add('assertions', 'source/provenance kind interchange'); return F
    if expected.get('kind') != 'accounting_expected_assertion':
        F.add('assertions', 'expected kind interchange'); return F
    if expected.get('raw_subject_ref') != 'sha256:' + hashlib.sha256(raw_bytes).hexdigest():
        F.add('assertions', 'raw source binding changed; qualification invalidated'); return F
    for assertion in expected.get('assertions', []):
        for pointer, literal in assertion['expected_fields_or_relations'].items():
            try:
                actual = supplied
                for part in pointer.split('/')[1:]:
                    part = part.replace('~1', '/').replace('~0', '~')
                    actual = actual[int(part)] if isinstance(actual, list) else actual[part]
                if actual != literal: F.add(assertion['claim_id'], f'independent exact assertion failed at {pointer}')
            except (KeyError, IndexError, ValueError, TypeError):
                F.add(assertion['claim_id'], f'independent exact assertion missing at {pointer}')
    return F


def pilot_qualification(subject):
    F = Failures(); rates = load_rates(os.path.join(subject, 'oracle/rates')); consumed=[]
    for claim in r9_pilot.load_registry():
        for b in claim['cases']:
            if 'control_file' in b:
                c=json.load(open(os.path.join(subject,'oracle/r9-pilot',b['control_file'])))
                r9_pilot.control_claim(claim,b,c,F)
                check_case(c,rates,F)
                check_accounting_references(c,F,b)
            else:
                c=json.load(open(os.path.join(subject,'corpus/cases',b['case_id']+'.json')))
                check_case(c,rates,F);check_accounting_references(c,F)
        consumed.append(claim['claim_id'])
    r9_pilot.conditional_raw_qualification(F)
    for f in F: print('FAIL',f)
    print('R9 consumed4 claims:', ','.join(consumed), '; 7 required case bindings plus2 conditional bundles; finite staged pilot only;',len(F),'failures')
    return 1 if F else 0


def main(argv):
    if len(argv) < 3: print(__doc__); return 2
    cmd = argv[1]
    if cmd == 'check' and '--pilot' in argv:
        os.environ['R9_QUALIFICATION_SUBJECT'] = os.path.abspath(os.path.join(argv[2], '..'))
        return pilot_qualification(os.environ['R9_QUALIFICATION_SUBJECT'])
    if cmd == 'check-pilot':
        os.environ['R9_QUALIFICATION_SUBJECT'] = os.path.abspath(argv[2])
        return pilot_qualification(argv[2])
    if cmd == 'assertions':
        raw_bytes = open(argv[2], 'rb').read()
        F = check_exact_raw_assertions(raw_bytes, json.load(open(argv[3])), json.load(open(argv[4])))
        for f in F: print('FAIL', f)
        print(f'Named finite raw-bound assertions only: {len(F)} failures; full raw-to-output conformance NOT ESTABLISHED')
        return 1 if F else 0
    if cmd in ('check', 'partition'):
        partition = argv[4] if cmd == 'partition' else None
        if partition not in (None, 'A', 'B'): raise ValueError('partition must be A or B')
        F, cases = check_corpus(argv[2], argv[3], partition)
        for f in F: print('FAIL', f)
        by = {p_: len([c for c in cases if c.get('profile') == p_]) for p_ in sorted(PROFILES)}
        adm = {p_: len(valid_admitted(cases, p_)) for p_ in sorted(PROFILES)}
        print(f'{len(cases)} structural/legacy cases {by}, admitted legacy controls {adm}, {len(F)} failures')
        claims = [a for a in independent_low_assertions().values() if a['case_id'] in {c['case_id'] for c in cases}]
        total = sum(2 * len(e['expected']['objectives']) for c in cases for e in c['evaluations'])
        print(f'BOUNDED LOW QUALIFICATION: {len(claims)} named raw-bound assertions; {total-len(claims)} scopes REQUIRED/UNMET. Finite supplied records only; full raw reducer/live authority NOT ESTABLISHED.')
        return 1 if F else 0
    if cmd == 'results':
        cases = corpus_cases(argv[2]); fields = GRADED
        if '--scope' in argv and argv[argv.index('--scope') + 1] == 's2a': cases, fields = s2a_subset(cases), S2A_GRADED
        F, _ = grade(cases, json.load(open(argv[3])), fields)
        for f in F: print('FAIL', f)
        print(f'{len(cases)} cases graded, {len(F)} differences'); return 1 if F else 0
    if cmd == 'selftest':
        return 0 if selftest(argv[2], argv[3]) else 1
    if cmd == 'revisions':
        F, np_, nc, fp, fc = check_revisions(argv[2], argv[3])
        for f in F: print('FAIL', f)
        print(f'previous {np_} cases / {fp} finding tags; current {nc} cases / {fc} finding tags; {len(F)} failures'); return 1 if F else 0
    if cmd == 's2a-subset':
        sub = s2a_subset(corpus_cases(argv[2]))
        for c in sub: print(c['case_id'])
        print(f'{len(sub)} cases'); return 0
    print(__doc__); return 2


if __name__ == '__main__':
    sys.exit(main(sys.argv))
