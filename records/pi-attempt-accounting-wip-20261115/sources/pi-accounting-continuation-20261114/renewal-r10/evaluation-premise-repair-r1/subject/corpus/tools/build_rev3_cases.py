"""Cases added in revision 3 (contract 1.2-s1, P1.30): one explicit counterexample or control per confirmed P1.29
finding, derived deterministically from revision-3 cases. Every expectation is written out here from the contract text;
nothing is computed by a reader. Historical (A1.3.4) blocks are added separately by historical_a134.py.
Usage: build_rev3_cases.py <corpus_dir> <out_cases_dir>"""
import copy, json, os, sys

corpus, out = sys.argv[1], sys.argv[2]
load = lambda cid: json.load(open(os.path.join(corpus, 'cases', cid + '.json')))
Z = {'low': '0', 'high': {'kind': 'exact', 'nano': '0'}}
SID = '00000000-0000-4000-8000-0000000000c1'
written = []


def esum(low, kind, nano): return {'low': str(low), 'high': {'kind': kind, 'nano': str(nano)}}


def new(src, cid, *, role, cls, reach, findings, controls, invariants, origin, notes, group='P1.29'):
    c = copy.deepcopy(load(src))
    c.update(case_id=cid, group=group, role=role, findings=findings, controls=controls, invariants=invariants, provenance={'origin': origin},
             reachability=reach, notes=notes, rationale=None, ambiguity=None, revisions=[], revisions_r3=[], added_in='1.2-s1', historical={},
             contract='pi-accounting-contract/1.2-s1')
    c['class'] = cls
    for k in ('twin_of', 'evidence_pair'): c.pop(k, None)
    return c


def lines(c, alias='S1', turn='turn-1'): return next(s for s in c['inputs']['sessions'] if s['alias'] == alias)['logs'][turn]['lines']
def sess(c, alias='S1'): return next(s for s in c['inputs']['sessions'] if s['alias'] == alias)
def ev(c, i=0): return c['evaluations'][i]
def obj(c, name='o', i=0): return next(o for o in ev(c, i)['expected']['objectives'] if o['objective'] == name)
def edit_row(c, n, fn, alias='S1'):
    ls = lines(c, alias); r = json.loads(ls[n - 1]); fn(r); ls[n - 1] = json.dumps(r)


def save(c):
    p = os.path.join(out, c['case_id'] + '.json'); json.dump(c, open(p, 'w'), indent=1, sort_keys=True); open(p, 'a').write('\n'); written.append(c['case_id'])


# R-01: Valid(Usage) with incomplete usage (terminal not a success) has no usage bound
c = new('VC01-openai-usage-settlement', 'P130-R01-usage-incomplete-terminal', role='finding', cls='producer_valid', reach='producer_reachable',
        findings=['P1.29-R-01'], controls=['VC01-openai-usage-settlement'], invariants=['INV-F5', 'INV-F12'],
        origin='contract::FB-1, V-5, 8 row 3 (P1.29 R-01)', notes='Producer prices the usage whatever the terminal (pi_container.py:1001-1007). Whether the provider returns usage with response.incomplete is not established.')
edit_row(c, 2, lambda r: r.update(terminal='response.incomplete'))
a = c['expected']['attempts'][0]; a['valuation']['complete'] = False
a['exposure']['high'] = {'kind': 'at_least', 'nano': '15000000', 'basis': 'usage_valuation'}; a['blocking'] = ['UnresolvedExposure']; a['billing'] = 'unresolved'
o = obj(c); o['owned_closed'] = esum(15000000, 'at_least', 15000000); o['reasons'] = ['UnresolvedExposure']; o['status'] = 'not_established'
ev(c)['expected']['admission']['decision'] = 'not_established'; save(c)

# R-02: an upward-rounded producer charge above the usage bound stays Exact, basis ProxyCharge
c = new('VC01-openai-usage-settlement', 'P130-R02-rounded-up-charge-admitted', role='valid_control', cls='producer_valid', reach='producer_reachable',
        findings=['P1.29-R-02'], controls=[], invariants=['INV-F5', 'INV-F11', 'INV-A1'],
        origin='contract::FB-1, 2 rounding, X-0 (P1.29 R-02)', notes='gpt-6.1-terra at flex, 21 cached tokens: exact price 525 nano; the producer logs round(5.25e-7, 6) = 0.000001 USD.')
sess(c)['launch']['model'] = 'openai/gpt-6.1-terra'
edit_row(c, 2, lambda r: r.update(model='gpt-6.1-terra', response_model='gpt-6.1-terra', observed_tier='flex',
         usage={'input_tokens': 21, 'output_tokens': 0, 'cached_tokens': 21, 'cache_write_tokens': 0, 'reasoning_tokens': 0},
         usd={**r['usd'], 'charged_usd': 0.000001, 'session_spent_usd': 0.000001}))
a = c['expected']['attempts'][0]; a['settlement']['charged'] = '1000'
a['valuation'] = {'state': 'valued', 'complete': True, 'identities': ['gpt-6.1-terra'], 'low': '525', 'high': '525',
                  'derivations': [{'for': 'both', 'model': 'gpt-6.1-terra', 'nano': '525', 'provider': 'openai', 'tier_multiplier': '0.5',
                                   'usage': {'input_tokens': 21, 'output_tokens': 0, 'cached_tokens': 21, 'cache_write_tokens': 0}}]}
a['exposure'] = {'pool': 'pi_api', 'low': {'nano': '525', 'basis': 'usage_valuation'}, 'high': {'kind': 'exact', 'nano': '1000', 'basis': 'proxy_charge'}}
o = obj(c); o['owned_closed'] = esum(525, 'exact', 1000); save(c)

# R-04 (a): a retired session can overrun its unenforced cap (served at priority)
c = new('VC01-openai-usage-settlement', 'P130-R04-priority-overrun-above-cap', role='finding', cls='producer_valid', reach='producer_reachable',
        findings=['P1.29-R-04'], controls=['VC01-openai-usage-settlement'], invariants=['INV-A8', 'INV-A3'],
        origin='contract::2.2, 12.3 A-3, A-5 (P1.29 R-04)', notes='Reservation 0.48 at the default tier fits the 0.5 cap; served at priority the same tokens cost 0.948, which settle() charges without a cap.')
ev(c)['objectives'][0]['admissions'][0]['rows'][0]['max_usd'] = 0.5
edit_row(c, 1, lambda r: r.update(reserved_usd=0.48, input_bound=2400))
edit_row(c, 2, lambda r: r.update(observed_tier='priority', usage={'input_tokens': 2400, 'output_tokens': 9000, 'cached_tokens': 0, 'cache_write_tokens': 0, 'reasoning_tokens': 0},
         usd={**r['usd'], 'reserved_usd': 0.48, 'charged_usd': 0.948, 'session_spent_usd': 0.948}))
a = c['expected']['attempts'][0]; a['settlement'].update(charged='948000000', reserved='480000000')
a['valuation'] = {'state': 'valued', 'complete': True, 'identities': ['gpt-6-astra'], 'low': '948000000', 'high': '948000000',
                  'derivations': [{'for': 'both', 'model': 'gpt-6-astra', 'nano': '948000000', 'provider': 'openai', 'tier_multiplier': '2',
                                   'usage': {'input_tokens': 2400, 'output_tokens': 9000, 'cached_tokens': 0, 'cache_write_tokens': 0}}]}
a['exposure'] = {'pool': 'pi_api', 'low': {'nano': '948000000', 'basis': 'usage_valuation'}, 'high': {'kind': 'exact', 'nano': '948000000', 'basis': 'usage_valuation'}}
o = obj(c); o['owned_closed'] = esum(948000000, 'exact', 948000000); o['reasons'] = ['CommitmentInvalid']; o['status'] = 'not_established'
ev(c)['expected']['admission']['decision'] = 'not_established'; save(c)

# R-04 (b): the safe counterpart of VC14: the session is retired, so the commitment is released and admission succeeds
c = new('VC14-open-session-commitment', 'P130-R04-retired-session-releases-commitment', role='valid_control', cls='producer_valid', reach='producer_reachable',
        findings=['P1.29-R-04'], controls=[], invariants=['INV-A8', 'INV-A1', 'INV-A5'],
        origin='contract::12.3 A-3, A-5 (P1.29 R-04 positive control)', notes='VC14 with its session retired after the settlement: no open commitment remains, so admission is established.')
sess(c)['lifecycle'].append({'at': '2026-11-10T06:50:00Z', 'event': 'retire'})
c['expected']['lifecycle']['S1'].update(state='retired')
c['evaluations'] = [c['evaluations'][0]]
o = obj(c); o['commitments'] = [{'admission_id': 'a1', 'committed': '0', 'kind': 'exact', 'state': 'retired'}]; o['reasons'] = []; o['status'] = 'established'
ev(c)['expected']['admission']['decision'] = 'admitted'
c['evidence_pair'] = {'other': 'VC14-open-session-commitment', 'relation': 'superset', 'permitted_release': True}; save(c)

# R-05: a conflicting duplicate settlement keeps the valuation high in the maximum
c = new('N01-identity-refusal-dearer-served', 'P130-R05-conflicting-keeps-valuation-high', role='finding', cls='adversarial_synthetic', reach='producer_unreachable',
        findings=['P1.29-R-05'], controls=['N01-identity-refusal-dearer-served'], invariants=['INV-F3', 'INV-A5'],
        origin='contract::8 rejected/conflicting row, V-8 (P1.29 R-05)', notes='N01 plus a distinct copy of its settlement row. The producer writes one settlement per rid, so this is synthetic.')
r3 = json.loads(lines(c)[2]); r3['at'] = '2026-11-10T06:30:05Z'; lines(c).append(json.dumps(r3))
a = c['expected']['attempts'][0]; a['evidence'].append('S1/turn-1:4')
a['settlement'] = {'decision': 'conflicting', 'reasons': [], 'reserved': '50000000', 'charged': '50000000'}
a['obligation'] = {'state': 'outstanding', 'amount_source': 'reservation_bound'}
a['exposure'] = {'pool': 'pi_api', 'low': {'nano': '0', 'basis': 'none'}, 'high': {'kind': 'at_least', 'nano': '80000000', 'basis': 'usage_valuation'}}
a['blocking'] = sorted(set(a['blocking']) | {'SettlementConflict'})
o = obj(c); o['owned_closed'] = copy.deepcopy(Z); o['owned_outstanding'] = esum(0, 'at_least', 80000000)
o['reasons'] = sorted(set(o['reasons']) | {'SettlementConflict'})
c['evidence_pair'] = {'other': 'N01-identity-refusal-dearer-served', 'relation': 'superset', 'permitted_release': False}; save(c)

# R-06 (a): a reservation with no rid keys a Row and stays an unresolved, unaccounted obligation
c = new('VC05-unsettled-reservation', 'P130-R06-reservation-rid-missing', role='finding', cls='malformed', reach='producer_unreachable',
        findings=['P1.29-R-06'], controls=['VC05-unsettled-reservation'], invariants=['INV-F6'],
        origin='contract::C-1 unusable rid (P1.29 R-06)', notes='The producer always writes rid; this is a malformed row.')
edit_row(c, 1, lambda r: r.pop('rid'))
a = c['expected']['attempts'][0]; a['key'] = 'row:S1/turn-1:line=1'; a['correlation'] = {'kind': 'unattributed'}
a['blocking'] = ['MalformedEvidence', 'UnaccountedAttempt', 'UnresolvedExposure']
ev(c)['expected']['attribution'][0]['attempt'] = 'row:S1/turn-1:line=1'
o = obj(c); o['reasons'] = ['MalformedEvidence', 'UnaccountedAttempt', 'UnresolvedExposure']; save(c)

# R-06 (b): a malformed reserved amount contributes 0 to R
c = new('VC05-unsettled-reservation', 'P130-R06-reservation-amount-malformed', role='finding', cls='malformed', reach='producer_unreachable',
        findings=['P1.29-R-06'], controls=['VC05-unsettled-reservation'], invariants=['INV-F5'],
        origin='contract::C-1, X-0 malformed reserved (P1.29 R-06)', notes='reserved_usd is a string; the producer writes a number.')
edit_row(c, 1, lambda r: r.update(reserved_usd='0.3'))
a = c['expected']['attempts'][0]; a['settlement'] = {'decision': 'absent', 'reasons': []}
a['exposure'] = {'pool': 'pi_api', 'low': {'nano': '0', 'basis': 'none'}, 'high': {'kind': 'at_least', 'nano': '0', 'basis': 'none'}}
a['blocking'] = ['MalformedEvidence', 'UnresolvedExposure']
o = obj(c); o['owned_outstanding'] = esum(0, 'at_least', 0); o['reasons'] = ['MalformedEvidence', 'UnresolvedExposure']; save(c)

# R-07: a predecessor attempt closed by an upstream error keeps constraining the successor
c = new('VC15-successor-not-charged-settled-predecessor-spend', 'P130-R07-closed-unresolved-predecessor-constrains-successor', role='finding',
        cls='producer_valid', reach='producer_reachable', findings=['P1.29-R-07'], controls=['VC15-successor-not-charged-settled-predecessor-spend'],
        invariants=['INV-A4', 'INV-F12'], origin='contract::6.2, 12.2 (P1.29 R-07)',
        notes='VC15 with the predecessor attempt closed by an upstream error: the exchange ended, the billing did not resolve.')
lines(c)[1] = json.dumps({'at': '2026-11-10T06:30:02Z', 'event': 'upstream_error', 'n': 1, 'detail': 'connection reset',
                          'usd': {'rid': 1, 'reserved_usd': 0.5, 'charged_usd': 0.5, 'charge_basis': 'reservation', 'session_spent_usd': 0.5, 'multiplier_floor': 0.0}})
a = c['expected']['attempts'][0]
a['settlement'] = {'decision': 'valid', 'basis': 'upstream_error', 'reasons': [], 'charged': '500000000', 'reserved': '500000000'}
a['obligation'] = {'state': 'closed', 'amount_source': 'reservation_confirmed'}; a['valuation'] = {'state': 'not_reported'}
a['exposure'] = {'pool': 'pi_api', 'low': {'nano': '0', 'basis': 'none'}, 'high': {'kind': 'at_least', 'nano': '500000000', 'basis': 'proxy_reservation'}}
a['blocking'] = ['UnresolvedExposure']; a['diagnostics'] = []; a['forwarding'] = 'unknown'; a['billing'] = 'unresolved'
p = obj(c, 'p'); p['owned_closed'] = esum(0, 'at_least', 500000000); p['reasons'] = ['UnresolvedExposure']; p['status'] = 'not_established'
s = obj(c, 's'); s['predecessor_obligations'] = esum(0, 'at_least', 500000000); s['reasons'] = ['UnresolvedExposure']; s['status'] = 'not_established'
ev(c)['expected']['admission']['decision'] = 'not_established'; save(c)

# R-08: explicit conflicting admissions block even when no tree holds the session
c = new('OWN-two-admitting-objectives', 'P130-R08-admissions-without-tree-still-block', role='finding', cls='attribution_uncertain', reach='producer_reachable',
        findings=['P1.29-R-08'], controls=['OWN-two-admitting-objectives'], invariants=['INV-T2'],
        origin='contract::O-3a, O-7 (P1.29 R-08)', notes='Both objectives admit the session but neither tree lists it; the admitting objectives stay candidates.')
for ob in ev(c)['objectives']: ob['tree_sessions'] = []
save(c)

# Q11 / R-09: an uncertain closure state gives not_established, never a refusal or an approval
c = new('VC01-openai-usage-settlement', 'P130-Q11-closure-uncertain', role='finding', cls='malformed', reach='not_established',
        findings=['P1.29-R-09'], controls=['U04-control-settled-before-closure'], invariants=['INV-A7'],
        origin='contract::12.4 step 1 (Q11, P1.30)', notes='closed_at is not an RFC 3339 stamp, so closure is not established either way.')
ev(c)['objectives'][0]['closed_at'] = 'yesterday'
o = obj(c); o['reasons'] = ['ClosureUncertain']; o['status'] = 'not_established'
ev(c)['expected']['admission']['decision'] = 'not_established'; save(c)

# R-10: retire, deliver, retire: Retired and reopened; the commitment is released, the violation persists
c = new('N07-reopened-session-journal', 'P130-R10-reopened-then-retired', role='finding', cls='producer_valid', reach='producer_reachable',
        findings=['P1.29-R-10'], controls=['N07-control-retired-journal'], invariants=['INV-A3'],
        origin='contract::L-4, L-5, A-3, A-4 (P1.29 R-10)', notes='N07-reopened plus a final dated retirement.')
sess(c)['lifecycle'].append({'at': '2026-11-10T06:40:00Z', 'event': 'retire'})
c['expected']['lifecycle']['S1'].update(state='retired', reopened=True)
o = obj(c); o['commitments'] = [{'admission_id': 'a1', 'committed': '0', 'kind': 'exact', 'state': 'retired'}]; o['reasons'] = ['ReopenedWithoutAdmission']; save(c)

# R-11: the permitted A5 exception: without its retirement the open commitment blocks
c = new('N07-control-retired-journal', 'P130-R11-open-session-before-retirement', role='control', cls='producer_valid', reach='producer_reachable',
        findings=['P1.29-R-11'], controls=[], invariants=['INV-A5', 'INV-A8'],
        origin='contract::A5 permitted release, 12.3 A-3, A-5 (P1.29 R-11)', notes='N07-control-retired-journal without its retire row; adding that row is a permitted release.')
sess(c)['lifecycle'] = [x for x in sess(c)['lifecycle'] if x['event'] != 'retire']
c['expected']['lifecycle']['S1'].update(state='open')
o = obj(c); o['commitments'] = [{'admission_id': 'a1', 'committed': '800000000', 'kind': 'at_least', 'state': 'launched_open'}]
o['reasons'] = ['UnenforcedCommitment']; o['status'] = 'not_established'
ev(c)['expected']['admission']['decision'] = 'not_established'
c['evidence_pair'] = {'other': 'N07-control-retired-journal', 'relation': 'subset', 'permitted_release': True}; save(c)

# A5 forbidden: an added unreadable line never makes admission more permissive
c = new('VC01-openai-usage-settlement', 'P130-A5-unreadable-line-added', role='control', cls='malformed', reach='producer_unreachable',
        findings=[], controls=[], invariants=['INV-A5', 'INV-F6'], group='P1.29-R-11',
        origin='contract::A5 forbidden addition (P1.29 R-11)', notes='VC01 plus a non-object JSON line.')
lines(c).append('[]')
c['expected']['unreadable'] = [{'evidence': 'S1/turn-1:3', 'reason': 'NotObject', 'blocking': ['UnreadableEvidence']}]
o = obj(c); o['reasons'] = ['UnreadableEvidence']; o['status'] = 'not_established'
ev(c)['expected']['admission']['decision'] = 'not_established'
c['evidence_pair'] = {'other': 'VC01-openai-usage-settlement', 'relation': 'superset', 'permitted_release': False}; save(c)

# R-12 (a): an outstanding reservation's interval runs to its bound, so a later-starting tree objective stays a candidate
c = new('OWN-budgeted-session-without-admission', 'P130-R12-outstanding-interval-keeps-candidate', role='finding', cls='attribution_uncertain',
        reach='producer_reachable', findings=['P1.29-R-12'], controls=['Q8-unadmitted-outside-candidate-window'], invariants=['INV-T2'],
        origin='contract::O-7, 11.2 (P1.29 R-12)', notes='Reservation at 06:30, no settlement, no retirement; the tree objective starts at 06:40.')
del lines(c)[1:]
sess(c)['lifecycle'] = [x for x in sess(c)['lifecycle'] if x['event'] != 'retire']
c['expected']['lifecycle']['S1'].update(state='open')
a = c['expected']['attempts'][0]; a['evidence'] = ['S1/turn-1:1']
a['settlement'] = {'decision': 'absent', 'reasons': [], 'reserved': '500000000'}; a['obligation'] = {'state': 'outstanding', 'amount_source': 'reservation_bound'}
a['valuation'] = {'state': 'not_reported'}
a['exposure'] = {'pool': 'pi_api', 'low': {'nano': '0', 'basis': 'none'}, 'high': {'kind': 'at_least', 'nano': '500000000', 'basis': 'proxy_reservation'}}
a['blocking'] = ['UnresolvedExposure']; a['diagnostics'] = []; a['forwarding'] = 'unknown'; a['execution'] = 'not_terminal'; a['billing'] = 'unresolved'
ev(c)['objectives'][0]['started_at'] = '2026-11-10T06:40:00Z'
o = obj(c); o['reasons'] = ['HistoricalUnadmitted', 'UnresolvedExposure']; save(c)

# R-12 (b): an untrusted clock makes the interval unplaceable, which keeps the candidate
c = new('VC06-unbudgeted-completed-request', 'P130-R12-untrusted-clock-keeps-candidate', role='finding', cls='attribution_uncertain',
        reach='producer_reachable', findings=['P1.29-R-12'], controls=['Q8-unadmitted-outside-candidate-window'], invariants=['INV-T2', 'INV-T4'],
        origin='contract::O-7, P-2, L-2 (P1.29 R-12)', notes='The request is stamped 06:30:02, before its turn start at 06:35 (L-2), so the clock is untrusted; the tree objective starts at 06:40.')
sess(c)['lifecycle'][0]['at'] = '2026-11-10T06:35:00Z'
c['expected']['lifecycle']['S1'] = {'state': 'contradicted', 'clock': 'untrusted', 'reason': 'TurnEvidenceBeforeStart', 'reopened': False}
ev(c)['objectives'][0]['started_at'] = '2026-11-10T06:40:00Z'
ev(c)['expected']['attribution'][0]['placement'] = 'unplaceable'; save(c)

# R-14: a null Anthropic tier is standard, as the producer accepts it
c = new('VC02-anthropic-usage-settlement', 'P130-R14-anthropic-null-tier-admitted', role='valid_control', cls='producer_valid', reach='producer_reachable',
        findings=['P1.29-R-14'], controls=[], invariants=['INV-A6', 'INV-A1'],
        origin='contract::V-3 Anthropic (P1.29 R-14)', notes='observed_tier null: the producer logs usage.get("service_tier"); not observed in the 148 real Anthropic rows.')
edit_row(c, 2, lambda r: r.update(observed_tier=None)); save(c)

# Table T (focus area 6): an owned attempt wholly before its owner's window stays its money and blocks
c = new('VC01-openai-usage-settlement', 'P130-T-owned-before-window-summed', role='finding', cls='adversarial_synthetic', reach='producer_reachable',
        findings=[], controls=['VC01-openai-usage-settlement'], invariants=['INV-T1'], group='P1.29-focus-6',
        origin='contract::Table T, P-5, 12.1 (P1.29 focus area 6)', notes='The objective starts at 06:40; its admitted session spent at 06:30.')
ev(c)['objectives'][0]['started_at'] = '2026-11-10T06:40:00Z'
ev(c)['expected']['attribution'][0]['placement'] = 'before'
o = obj(c); o['reasons'] = ['AttributionContradiction']; o['status'] = 'not_established'
ev(c)['expected']['admission']['decision'] = 'not_established'; save(c)

print(f'{len(written)} cases written')
