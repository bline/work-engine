#!/usr/bin/env python3
"""Corpus-level negative tests for the validator: each sabotage edits one fact in a temporary copy of the corpus; the
validator's `check` (or, where named, `revisions` against the previous corpus) must then fail, and at least one FAIL line
must carry the sabotage's intended reason (EXPECT). Revision 3 (P1.30) adds one sabotage per confirmed P1.29 finding.
Usage: sabotage_test.py <corpus_dir> <rates_dir> <previous_corpus_dir>"""
import json, os, shutil, subprocess, sys, tempfile

HERE = os.path.dirname(os.path.abspath(__file__))
corpus, rates, previous = sys.argv[1], sys.argv[2], sys.argv[3]


def edit(case, fn):
    def apply(d):
        p = os.path.join(d, 'cases', case + '.json'); c = json.load(open(p)); fn(c); json.dump(c, open(p, 'w'), indent=1, sort_keys=True)
    return apply


def a0(c): return c['expected']['attempts'][0]
def ev(c, i=0): return c['evaluations'][i]['expected']


def exact_reservation(c):            # FB-2 undone consistently: attempt, objective line, reasons, status and admission all "repaired"
    a0(c)['exposure']['high']['kind'] = 'exact'; a0(c)['blocking'] = [b for b in a0(c)['blocking'] if b != 'UnresolvedExposure']
    o = ev(c)['objectives'][0]
    for l in ('owned_closed', 'owned_outstanding'): o[l]['high']['kind'] = 'exact'
    o['reasons'] = []; o['status'] = 'established'; ev(c)['admission']['decision'] = 'admitted'


def revisions_entry_dropped(c): c['revisions_r6'] = [r for r in c['revisions_r6'] if r['field'] != 'expected']
def obj(c, name='o', i=0): return next(o for o in ev(c, i)['objectives'] if o['objective'] == name)


def r01_exact(c):
    a0(c)['exposure']['high']['kind'] = 'exact'; a0(c)['billing'] = 'established'; a0(c)['blocking'] = []
    o = obj(c); o['owned_closed']['high']['kind'] = 'exact'; o['reasons'] = []; o['status'] = 'established'; ev(c)['admission']['decision'] = 'admitted'


def r03_finite(c):
    a0(c)['valuation'] = {'state': 'valued', 'complete': True, 'identities': ['gpt-6-astra'], 'low': '7500000', 'high': '30000000', 'derivations': [
        {'for': 'low', 'model': 'gpt-6-astra', 'nano': '7500000', 'provider': 'openai', 'tier_multiplier': '0.5', 'usage': {'cache_write_tokens': 0, 'cached_tokens': 0, 'input_tokens': 1000, 'output_tokens': 100}},
        {'for': 'high', 'model': 'gpt-6-astra', 'nano': '30000000', 'provider': 'openai', 'tier_multiplier': '2', 'usage': {'cache_write_tokens': 0, 'cached_tokens': 0, 'input_tokens': 1000, 'output_tokens': 100}}]}
    a0(c)['exposure'] = {'pool': 'pi_api', 'low': {'nano': '7500000', 'basis': 'usage_valuation'}, 'high': {'kind': 'exact', 'nano': '30000000', 'basis': 'usage_valuation'}}
    a0(c)['blocking'] = []; a0(c)['billing'] = 'established'
    o = obj(c); o['owned_closed'] = {'low': '7500000', 'high': {'kind': 'exact', 'nano': '30000000'}}; o['reasons'] = []; o['status'] = 'established'; ev(c)['admission']['decision'] = 'admitted'


def r04_cap_bound(c):
    for i in range(len(c['evaluations'])):
        o = obj(c, i=i); o['commitments'][0]['kind'] = 'exact'; o['reasons'] = []; o['status'] = 'established'
    ev(c, 0)['admission']['decision'] = 'admitted'; ev(c, 1)['admission']['decision'] = 'refused'


def r07_drop(c):
    s = obj(c, 's'); s['predecessor_obligations'] = {'low': '0', 'high': {'kind': 'exact', 'nano': '0'}}; s['reasons'] = []; s['status'] = 'established'
    ev(c)['admission']['decision'] = 'admitted'


def r12_drop(c):
    ev(c)['attribution'][0]['candidates'] = []; o = obj(c); o['reasons'] = [r for r in o['reasons'] if r not in ('HistoricalUnadmitted', 'UnresolvedExposure')]
    o['status'] = 'not_established' if o['reasons'] else 'established'; ev(c)['admission']['decision'] = 'not_established' if o['reasons'] else 'admitted'


def r10_lose(c): c['expected']['lifecycle']['S1']['reopened'] = False; obj(c)['reasons'] = []; obj(c)['status'] = 'established'


def u03_release(c):
    o = obj(c); o['commitments'] = [{'admission_id': 'a1', 'committed': '0', 'kind': 'exact', 'state': 'retired'}]; o['reasons'] = ['PlacementUnplaceable']


def r15_low_max(c):
    v = a0(c)['valuation']; d = next(x for x in v['derivations'] if x['for'] == 'low'); hi = next(x for x in v['derivations'] if x['for'] == 'high')
    d['remainder_rate'] = 'max'; d['nano'] = hi['nano']; v['low'] = hi['nano']


SABOTAGE = {
    'derivation arithmetic wrong': edit('VC01-openai-usage-settlement', lambda c: a0(c)['valuation']['derivations'][0].update(nano='15000001')),
    'valuation high not the dearest derivation': edit('N01-identity-refusal-dearer-served', lambda c: a0(c)['valuation'].update(high='20000000')),
    'exposure contradicts Table X (usage replaced by reservation)': edit('VC01-openai-usage-settlement', lambda c: a0(c)['exposure']['high'].update(nano='500000000')),
    'unknown made finite (AtLeast -> Exact)': edit('N01-identity-refusal-dearer-served', lambda c: a0(c)['exposure']['high'].update(kind='exact')),
    'AtLeast without UnresolvedExposure': edit('N01-identity-refusal-dearer-served', lambda c: a0(c).update(blocking=['IdentityContradiction'])),
    'rejected without SettlementRejected': edit('S01-upstream-error-charged-zero', lambda c: a0(c).update(blocking=[])),
    'release without valid settlement': edit('S01-upstream-error-charged-zero', lambda c: a0(c).update(obligation={'state': 'closed', 'amount_source': 'settlement_amount'})),
    'evidence line dropped (totality)': edit('R05-non-object-json-rows', lambda c: c['expected'].update(unreadable=c['expected']['unreadable'][:1])),
    'evidence line counted twice': edit('VC01-openai-usage-settlement', lambda c: a0(c)['evidence'].append('S1/turn-1:1')),
    'objective sum wrong': edit('VC11-multi-turn-session', lambda c: c['evaluations'][0]['expected']['objectives'][0]['owned_closed'].update(low='15000000')),
    'objective reason missing': edit('N01-identity-refusal-dearer-served', lambda c: c['evaluations'][0]['expected']['objectives'][0].update(reasons=['IdentityContradiction'])),
    'admission inconsistent with totals': edit('VC14-open-session-commitment', lambda c: c['evaluations'][1]['expected']['admission'].update(decision='admitted')),
    'commitment amount wrong': edit('VC14-open-session-commitment', lambda c: c['evaluations'][0]['expected']['objectives'][0]['commitments'][0].update(committed='500000000')),
    'predecessor obligations double-count settled spend': edit('VC15-successor-not-charged-settled-predecessor-spend',
        lambda c: c['evaluations'][0]['expected']['objectives'][1]['predecessor_obligations']['high'].update(nano='15000000')),
    'finding without control': edit('N01-identity-refusal-dearer-served', lambda c: c.update(controls=[])),
    'rate digest altered': edit('VC01-openai-usage-settlement', lambda c: c['inputs']['rates'].update({'openai_api_rates.json': '0' * 64})),
    'twin differs': edit('ORD-sessions-reversed', lambda c: c['evaluations'][0]['expected']['objectives'][0]['owned_outstanding']['high'].update(nano='300000001')),
    'historical differs without rationale': edit('VC08-subscription-measured-request', lambda c: c.update(historical={'a134_086bdec1': {'decision': 'refused'}}, rationale=None)),
    'Codex attempt carries money': edit('VC08-subscription-measured-request', lambda c: a0(c)['exposure'].update(low={'nano': '0', 'basis': 'none'})),
    'several identities without IdentityContradiction': edit('V01-usage-settlement-other-response-model', lambda c: a0(c).update(blocking=[])),
    # revision 2 (contract 1.1-s1)
    'FB-2: unsettled reservation made Exact, consistently through admission': edit('VC05-unsettled-reservation', exact_reservation),
    'FB-2: reservation settlement made Exact, consistently through admission': edit('VC03-reservation-settlement-with-usage', exact_reservation),
    'Q2: error status made Exact(R)': edit('S02-control-error-status-after-reservation', exact_reservation),
    'Q2: error status made Exact(0)': edit('S02-control-error-status-after-reservation', lambda c: (exact_reservation(c), a0(c)['exposure']['high'].update(nano='0'),
        ev(c)['objectives'][0]['owned_closed']['high'].update(nano='0'))),
    'Q5: unrecognised tier made Exact': edit('Q5-unrecognised-openai-tier', exact_reservation),
    'Q5: unrecognised tier given an unsupported low': edit('Q5-unrecognised-openai-tier', lambda c: (a0(c)['exposure']['low'].update(nano='7500000', basis='usage_valuation'),
        ev(c)['objectives'][0]['owned_closed'].update(low='7500000'))),
    'Q8: unadmitted attempt given an owner': edit('VC06-unbudgeted-completed-request', lambda c: ev(c)['attribution'][0].update(objective='o')),
    'Q8: unadmitted reported as OwnershipUncertain': edit('VC06-unbudgeted-completed-request', lambda c: ev(c)['objectives'][0].update(reasons=['OwnershipUncertain'])),
    'Q8: unadmitted blocks nobody': edit('VC06-unbudgeted-completed-request', lambda c: (ev(c)['objectives'][0].update(reasons=[], status='established'),
        ev(c)['admission'].update(decision='admitted'))),
    'O-7: out-of-window objective kept as candidate': edit('Q8-unadmitted-outside-candidate-window', lambda c: (ev(c, 1)['attribution'][0].update(candidates=['o']),
        ev(c, 1)['objectives'][0].update(reasons=['HistoricalUnadmitted'], status='not_established'), ev(c, 1)['admission'].update(decision='not_established'))),
    'O-7: in-window candidate dropped': edit('Q8-unadmitted-outside-candidate-window', lambda c: (ev(c)['attribution'][0].update(candidates=[]),
        ev(c)['objectives'][0].update(reasons=[], status='established'), ev(c)['admission'].update(decision='admitted'))),
    'Q6: identity refusal recorded as not forwarded': edit('N01-identity-refusal-dearer-served', lambda c: a0(c).update(forwarding='unknown')),
    'Q6: reservation-only attempt recorded as forwarded': edit('VC05-unsettled-reservation', lambda c: a0(c).update(forwarding='forwarded')),
    # revision 3 (contract 1.2-s1, P1.30): one per confirmed P1.29 finding, plus the strengthened structural checks
    'R-01 incomplete usage made Exact, consistently': edit('P130-R01-usage-incomplete-terminal', r01_exact),
    'R-02 proxy-charge Exact without a complete usage bound': edit('P130-R02-rounded-up-charge-admitted', lambda c: a0(c)['valuation'].update(complete=False)),
    'R-03 missing tier valued as a finite range, consistently': edit('U08-missing-observed-tier', r03_finite),
    'R-04 open commitment counted as a finite bound, consistently': edit('VC14-open-session-commitment', r04_cap_bound),
    'R-05 conflicting maximum drops the valuation high': edit('P130-R05-conflicting-keeps-valuation-high', lambda c: (a0(c)['exposure']['high'].update(nano='50000000'), obj(c)['owned_outstanding']['high'].update(nano='50000000'))),
    'R-06 reservation with no rid keyed as a reservation': edit('P130-R06-reservation-rid-missing', lambda c: (a0(c).update(key='res:S1/turn-1:rid=1'), ev(c)['attribution'][0].update(attempt='res:S1/turn-1:rid=1'))),
    'R-06 closed on an RS-rule join instead of the key join': edit('N01-identity-refusal-dearer-served', lambda c: a0(c).update(correlation={'kind': 'unattributed', 'auxiliary': a0(c)['correlation']['auxiliary']})),
    'R-06 auxiliary rule does not match its row shape': edit('N01-identity-refusal-dearer-served', lambda c: a0(c)['correlation']['auxiliary'][0].update(rule='RS-1')),
    'R-07 successor line drops a closed-but-unresolved predecessor attempt, consistently': edit('P130-R07-closed-unresolved-predecessor-constrains-successor', r07_drop),
    'R-08 admitting objective dropped from an Uncertain attempt': edit('P130-R08-admissions-without-tree-still-block', lambda c: (ev(c)['attribution'][0].update(candidates=['o']), obj(c, 'p').update(reasons=[], status='established'))),
    'R-09 closed established objective admitted': edit('P132-CP-closed-valid-close-entry', lambda c: ev(c)['admission'].update(decision='admitted')),
    'Q11 uncertain closure refused': edit('P130-Q11-closure-uncertain', lambda c: (ev(c)['admission'].update(decision='refused'), obj(c).update(reasons=[], status='established'))),
    'R-10 reopened fact lost after the final retirement': edit('P130-R10-reopened-then-retired', r10_lose),
    'R-11 forbidden addition claimed more permissive (pair relation inverted)': edit('P130-A5-unreadable-line-added', lambda c: c['evidence_pair'].update(relation='subset')),
    'R-11 permitted release that adds more than a retirement': edit('P130-R11-open-session-before-retirement', lambda c: c['inputs']['sessions'][0]['lifecycle'].append({'at': '2026-11-10T06:10:00Z', 'event': 'deliver', 'turn': 2})),
    'R-11 permitted release whose larger case adds a non-retirement row': edit('P132-FG-VC01-gated', lambda c: c['inputs']['sessions'][0]['lifecycle'].insert(1, {'at': '2026-11-10T06:10:00Z', 'event': 'deliver', 'turn': 2})),
    'R-12 outstanding interval treated as a single stamp': edit('P130-R12-outstanding-interval-keeps-candidate', r12_drop),
    'R-12 untrusted clock excluded by a row stamp': edit('P130-R12-untrusted-clock-keeps-candidate', r12_drop),
    'R-13 header-write stream_error recorded as not forwarded': edit('N04-budgeted-header-failure-own-reservation', lambda c: next(a for a in c['expected']['attempts'] if a['key'].endswith('rid=2')).update(forwarding='unknown')),
    'R-15 residual Anthropic writes at the max rate for the low': edit('V02-control-producer-price', r15_low_max),
    'F12 billing derived from the closed obligation': edit('VC04-upstream-error-keeps-reservation', lambda c: a0(c).update(billing='established')),
    'execution fact contradicts the rows': edit('VC05-unsettled-reservation', lambda c: a0(c).update(execution='terminal')),
    'lifecycle retired although the last row is not a retirement': edit('VC14-open-session-commitment', lambda c: c['expected']['lifecycle']['S1'].update(state='retired')),
    'commitment released on a contradicted lifecycle': edit('U03-retirement-before-turn-evidence', u03_release),
    # revision 4 (contract 1.3-s1, P1.32): OQ-1 to OQ-4
    'OQ-1 current-producer retired session released': edit('VC01-openai-usage-settlement', lambda c: (obj(c)['commitments'][0].update(state='retired', committed='0', kind='exact'), obj(c).update(reasons=[], status='established'), ev(c)['admission'].update(decision='admitted'))),
    'OQ-1 self-asserted gate trusted': edit('P132-FG-gate-self-asserted-file', lambda c: (obj(c)['commitments'][0].update(state='retired', committed='0', kind='exact', gate={'qualified': True, 'reasons': []}), obj(c).update(reasons=[], status='established'), ev(c)['admission'].update(decision='admitted'))),
    'OQ-1 qualified gate ignored': edit('P132-FG-VC01-gated', lambda c: (obj(c)['commitments'][0].update(state='retired_resumable', committed='985000000', kind='at_least', gate={'qualified': False, 'reasons': ['NoGateEvidence']}), obj(c).update(reasons=['UnenforcedCommitment'], status='not_established'), ev(c)['admission'].update(decision='refused'))),
    'OQ-1 gate evidence corrupted, expectation unchanged': edit('P132-FG-VC01-gated', lambda c: c['inputs']['gates'][0].update(writer='same_uid_file')),
    'OQ-1 gate receipt digest removed, expectation unchanged': edit('P132-FG-VC02-gated', lambda c: c['inputs']['gates'][0]['mechanisms']['retries']['receipt'].pop('sha256')),
    'OQ-1 open-session billing bound ignored': edit('P133-FG-VC14-authenticated-bound', lambda c: (obj(c)['commitments'][0].update(kind='at_least'), obj(c).update(reasons=['UnenforcedCommitment'], status='not_established'), ev(c)['admission'].update(decision='not_established'))),
    'OQ-1 resumed turn attributed to the original admission': edit('P132-CP-retired-session-resumed-without-admission', lambda c: obj(c).update(reasons=[r for r in obj(c)['reasons'] if r != 'ReopenedWithoutAdmission'])),
    'OQ-1 gate contradiction hidden': edit('P132-FG-gate-contradicted-by-reopening', lambda c: obj(c).update(reasons=[r for r in obj(c)['reasons'] if r != 'GateContradicted'])),
    'OQ-2 known over limit not refused': edit('P132-CP-known-over-limit', lambda c: ev(c)['admission'].update(decision='not_established')),
    'OQ-2 unknown below limit refused': edit('P132-CP-unknown-below-limit', lambda c: ev(c)['admission'].update(decision='refused')),
    'OQ-4 contradictory close entry refused': edit('P132-CP-close-entry-contradicts-closed-at', lambda c: (ev(c)['admission'].update(decision='refused'), obj(c).update(reasons=[r for r in obj(c)['reasons'] if r != 'ClosureNotEstablished']))),
    'OQ-4 legacy close entry trusted in the vertical profile': edit('P132-FG-closure-legacy-entry-only', lambda c: (ev(c)['admission'].update(decision='refused'), obj(c).update(reasons=[r for r in obj(c)['reasons'] if r != 'ClosureNotEstablished']))),
    'OQ-4 closed predecessor outstanding dropped from successor': edit('P132-CP-closed-predecessor-outstanding', lambda c: obj(c, 's').update(predecessor_obligations={'low': '0', 'high': {'kind': 'exact', 'nano': '0'}})),
    'OQ-4 future gated case relabelled as historical evidence': edit('P132-FG-VC01-gated', lambda c: c.update(historical={'a134_086bdec1': {'decision': 'admitted'}})),
    'OQ-1 current-producer case carrying a hypothetical gate': edit('VC01-openai-usage-settlement', lambda c: c['inputs'].update(gates=[{'gate_id': 'g-S1'}])),
    # revision 5 (contract 1.4-s1, P1.33): one or more per Module A finding
    'A-01 incomplete usage releases the reservation': edit('U08-missing-observed-tier', lambda c: (a0(c).update(replacement={'eligible': True, 'reason': 'UsageBound'}, obligation={'state': 'closed', 'amount_source': 'settlement_amount'}), a0(c)['exposure']['high'].update(nano='30000000', basis='usage_valuation'))),
    'A-01 incomplete usage made finite': edit('P133-A01-incomplete-usage-above-reservation', lambda c: (a0(c)['exposure']['high'].update(kind='exact'), a0(c).update(billing='established', blocking=[]))),
    'A-02 error status discards supported usage': edit('P133-A02-error-status-with-priceable-usage', lambda c: a0(c).update(exposure={'pool': 'pi_api', 'low': {'nano': '0', 'basis': 'none'}, 'high': {'kind': 'at_least', 'nano': '800000000', 'basis': 'proxy_reservation'}})),
    'A-03 bound above the cap treated as qualified': edit('P133-FG-bound-exceeds-cap', lambda c: (obj(c)['commitments'][0].update(kind='exact', gate={'qualified': True, 'reasons': []}), obj(c).update(reasons=[], status='established'), ev(c)['admission'].update(decision='admitted'))),
    'A-03 bound amount tampered, expectation unchanged': edit('P133-FG-VC14-authenticated-bound', lambda c: c['inputs']['gates'][0]['mechanisms']['billing_bound']['bound'].update(amount_nano='400000000')),
    'A-04 absent cache count assumed zero': edit('P133-A04-cache-write-absent-legacy', lambda c: (a0(c)['valuation'].update(high='15000000', derivations=a0(c)['valuation']['derivations'][:1]), a0(c)['valuation']['derivations'][0].update({'for': 'both'}), a0(c)['exposure']['high'].update(nano='15000000'), obj(c).update())),
    'A-04 malformed cache count valued': edit('P133-A04-cached-malformed', lambda c: a0(c).update(valuation={'state': 'valued', 'complete': False, 'identities': ['gpt-6-astra'], 'low': '15000000', 'high': '15000000', 'derivations': [{'for': 'both', 'model': 'gpt-6-astra', 'nano': '15000000', 'provider': 'openai', 'tier_multiplier': '1', 'usage': {'input_tokens': 1000, 'output_tokens': 100, 'cached_tokens': 0, 'cache_write_tokens': 0}}]})),
    'A-05 case-variant model accepted as the pinned model': edit('P133-A05-model-case-variant', lambda c: a0(c).update(settlement={'decision': 'valid', 'basis': 'usage', 'reasons': [], 'reserved': '500000000', 'charged': '15000000'}, obligation={'state': 'closed', 'amount_source': 'settlement_amount'}, replacement={'eligible': True, 'reason': 'UsageBound'}, valuation={'state': 'valued', 'complete': True, 'identities': ['gpt-6-astra'], 'low': '15000000', 'high': '15000000', 'derivations': [{'for': 'both', 'model': 'gpt-6-astra', 'nano': '15000000', 'provider': 'openai', 'tier_multiplier': '1', 'usage': {'input_tokens': 1000, 'output_tokens': 100, 'cached_tokens': 0, 'cache_write_tokens': 0}}]}, exposure={'pool': 'pi_api', 'low': {'nano': '15000000', 'basis': 'usage_valuation'}, 'high': {'kind': 'exact', 'nano': '15000000', 'basis': 'usage_valuation'}}, blocking=[], billing='established')),
    'A-06 unsupported missing status accepted on the usage path': edit('P133-A06-usage-settlement-status-absent', lambda c: a0(c).update(settlement={'decision': 'valid', 'basis': 'usage', 'reasons': [], 'reserved': '500000000', 'charged': '15000000'}, obligation={'state': 'closed', 'amount_source': 'reservation_bound'}, replacement={'eligible': False, 'reason': 'UsageIncomplete'}, blocking=['UnresolvedExposure'], exposure={'pool': 'pi_api', 'low': {'nano': '15000000', 'basis': 'usage_valuation'}, 'high': {'kind': 'at_least', 'nano': '500000000', 'basis': 'proxy_reservation'}})),
    'A-06 S06 valuation erased again': edit('S06-string-http-status', lambda c: a0(c).update(valuation={'state': 'not_reported'})),
    'A-08 amount_source outside the typed variants': edit('VC04-upstream-error-keeps-reservation', lambda c: a0(c)['obligation'].update(amount_source='reservation_kept')),
    'A-09 conflicting reservations summed': edit('R04-reused-reservation-id', lambda c: a0(c)['exposure']['high'].update(nano='900000000')),
    'A-09 smaller conflicting reservation chosen': edit('P133-A09-conflicting-reservations-with-settlement', lambda c: a0(c)['exposure']['high'].update(nano='300000000')),
    'X-0 tie basis reversed': edit('VC04-upstream-error-keeps-reservation', lambda c: a0(c)['exposure']['high'].update(basis='proxy_charge')),
    'revision log: changed expectation without its entry': (edit('VC14-open-session-commitment', revisions_entry_dropped), 'revisions'),
    'revision log: historical outcome altered': (edit('VC05-unsettled-reservation', lambda c: c['historical']['a134_086bdec1'].update(decision='refused')), 'revisions'),
    'revision log: unlogged normative change': (edit('VC01-openai-usage-settlement', lambda c: ev(c)['objectives'][0].update(provisional=True)), 'revisions'),
    'revision log: previous rationale overwritten': (edit('VC06-unbudgeted-completed-request', lambda c: c.update(rationale='replaced')), 'revisions'),
}

EXPECT = {   # the intended reason: a substring that at least one FAIL line must carry
    'derivation arithmetic wrong': 'rate files give', 'valuation high not the dearest derivation': 'valuation high',
    'exposure contradicts Table X (usage replaced by reservation)': 'Table X', 'unknown made finite (AtLeast -> Exact)': 'without a usage bound',
    'AtLeast without UnresolvedExposure': 'AtLeast without UnresolvedExposure', 'rejected without SettlementRejected': 'without SettlementRejected',
    'release without valid settlement': 'INV-F1', 'evidence line dropped (totality)': 'INV-F6', 'evidence line counted twice': 'counted twice',
    'objective sum wrong': 'sum of owned attempts', 'objective reason missing': 'implied by expected facts', 'admission inconsistent with totals': '12.4 decision',
    'commitment amount wrong': 'commitment', 'predecessor obligations double-count settled spend': 'INV-A4', 'finding without control': 'finding without a control',
    'rate digest altered': 'rate digest', 'twin differs': 'sum of owned attempts', 'historical differs without rationale': 'without a rationale',
    'Codex attempt carries money': 'INV-F9', 'several identities without IdentityContradiction': 'IdentityContradiction',
    'R-01 incomplete usage made Exact, consistently': 'Table X', 'R-02 proxy-charge Exact without a complete usage bound': 'usage bound',
    'R-03 missing tier valued as a finite range, consistently': 'R-03', 'R-04 open commitment counted as a finite bound, consistently': 'Table G',
    'R-05 conflicting maximum drops the valuation high': 'Table X', 'R-06 reservation with no rid keyed as a reservation': 'no reservation/admission with that rid',
    'R-06 closed on an RS-rule join instead of the key join': 'R-06', 'R-06 auxiliary rule does not match its row shape': 'E-3',
    'R-07 successor line drops a closed-but-unresolved predecessor attempt, consistently': 'R-07', 'R-08 admitting objective dropped from an Uncertain attempt': 'R-08',
    'R-09 closed established objective admitted': '12.4 decision refused', 'Q11 uncertain closure refused': 'ClosureNotEstablished',
    'R-10 reopened fact lost after the final retirement': 'R-10', 'R-11 forbidden addition claimed more permissive (pair relation inverted)': 'INV-A5',
    'R-11 permitted release that adds more than a retirement': 'not a subset', 'R-12 outstanding interval treated as a single stamp': 'R-12',
    'R-12 untrusted clock excluded by a row stamp': 'R-12', 'R-11 permitted release whose larger case adds a non-retirement row': 'A5 exception', 'R-13 header-write stream_error recorded as not forwarded': 'R-13',
    'R-15 residual Anthropic writes at the max rate for the low': 'R-15', 'F12 billing derived from the closed obligation': 'INV-F12',
    'execution fact contradicts the rows': '6.2', 'lifecycle retired although the last row is not a retirement': 'L-4',
    'commitment released on a contradicted lifecycle': 'Table G', 'revision log: changed expectation without its entry': 'beyond its logged revisions',
    'revision log: historical outcome altered': 'historical', 'revision log: unlogged normative change': 'beyond its logged revisions',
    'revision log: previous rationale overwritten': 'previous rationale not preserved',
    'OQ-1 current-producer retired session released': 'Table G', 'OQ-1 self-asserted gate trusted': 'Table G', 'OQ-1 qualified gate ignored': 'Table G',
    'OQ-1 gate evidence corrupted, expectation unchanged': 'A9', 'OQ-1 gate receipt digest removed, expectation unchanged': 'A9',
    'OQ-1 open-session billing bound ignored': 'Table G', 'OQ-1 resumed turn attributed to the original admission': 'implied by expected facts',
    'OQ-1 gate contradiction hidden': 'implied by expected facts', 'OQ-2 known over limit not refused': '12.4 decision refused',
    'OQ-2 unknown below limit refused': '12.4 decision not_established', 'OQ-4 contradictory close entry refused': 'implied by expected facts',
    'OQ-4 legacy close entry trusted in the vertical profile': 'implied by expected facts', 'OQ-4 closed predecessor outstanding dropped from successor': 'INV-A4',
    'OQ-4 future gated case relabelled as historical evidence': 'no historical comparison', 'OQ-1 current-producer case carrying a hypothetical gate': 'OQ-1',
    'A-01 incomplete usage releases the reservation': 'A-01', 'A-01 incomplete usage made finite': 'usage bound', 'A-02 error status discards supported usage': 'Table X',
    'A-03 bound above the cap treated as qualified': 'Table G', 'A-03 bound amount tampered, expectation unchanged': 'A9',
    'A-04 absent cache count assumed zero': 'raw V-4 range', 'A-04 malformed cache count valued': 'malformed cache counts but valued',
    'A-05 case-variant model accepted as the pinned model': 'MC-1', 'A-06 unsupported missing status accepted on the usage path': 'A-06',
    'A-06 S06 valuation erased again': 'not_reported', 'A-08 amount_source outside the typed variants': 'bad obligation',
    'A-09 conflicting reservations summed': 'Table X', 'A-09 smaller conflicting reservation chosen': 'Table X', 'X-0 tie basis reversed': 'X-0',
    'FB-2: unsettled reservation made Exact, consistently through admission': 'without a usage bound',
    'FB-2: reservation settlement made Exact, consistently through admission': 'without a usage bound',
    'Q2: error status made Exact(R)': 'without a usage bound', 'Q2: error status made Exact(0)': 'without a usage bound',
    'Q5: unrecognised tier made Exact': 'without a usage bound', 'Q5: unrecognised tier given an unsupported low': 'Table X',
    'Q8: unadmitted attempt given an owner': 'O-6', 'Q8: unadmitted reported as OwnershipUncertain': 'implied by expected facts',
    'Q8: unadmitted blocks nobody': 'implied by expected facts', 'O-7: out-of-window objective kept as candidate': 'O-7',
    'O-7: in-window candidate dropped': 'O-7', 'Q6: identity refusal recorded as not forwarded': '6.1',
    'Q6: reservation-only attempt recorded as forwarded': '6.1'}

# Revision6 negatives retain separately bound positive cases in the ordinary check.
SABOTAGE.update({
 'R6 terminal conjunction bypass': edit('R6-C02-non200-success-terminal-usage-control', lambda c: a0(c)['valuation'].update(complete=True)),
 'R6 family schema silently repaired': edit('N03-api-row-other-provider', lambda c: a0(c).update(valuation={'state':'valued','complete':True,'identities':['anthropic/claude-fable-5-2'],'low':'28000000','high':'28000000','derivations':[]})),
 'R6 gate fixture witness omitted': edit('P133-FG-VC14-authenticated-bound', lambda c: c.pop('trusted_fixture_inputs')),
 'R6 gate fixture witness binding forged': edit('P133-FG-VC14-authenticated-bound', lambda c: next(iter(c['trusted_fixture_inputs']['gate_verifications'].values())).update(payload_sha256='0'*64)),
 'R6 provider metadata verification forged': edit('P133-FG-VC14-authenticated-bound', lambda c: (c.pop('trusted_fixture_inputs'),c['inputs']['gates'][0].update(verified=True,verification={'state':'verified','provenance':'trusted_adapter'}))),
 'R6 malformed billing evidence blocker erased': edit('P133-A04-cached-malformed', lambda c: a0(c).update(blocking=['UnresolvedExposure'])),
 'R6 arbitrary conflicting reservation comparison': edit('P133-A09-conflicting-reservations-with-settlement', lambda c: a0(c)['settlement']['reasons'].append('ReservedMismatch')),
 'R6 closed by reference erased': edit('VC01-openai-usage-settlement', lambda c: a0(c)['obligation'].pop('by')),
})
EXPECT.update({'R6 terminal conjunction bypass':'status+terminal+counts conjunction','R6 family schema silently repaired':'UsageSchemaMismatch','R6 gate fixture witness omitted':'A9','R6 gate fixture witness binding forged':'A9','R6 provider metadata verification forged':'A9','R6 malformed billing evidence blocker erased':'malformed billing evidence lacks field diagnostic','R6 arbitrary conflicting reservation comparison':'UniqueReservation','R6 closed by reference erased':'by reference'})

results, ok = {}, True
for name, spec in SABOTAGE.items():
    fn, cmd = spec if isinstance(spec, tuple) else (spec, 'check')
    with tempfile.TemporaryDirectory() as d:
        shutil.copytree(corpus, os.path.join(d, 'c'), ignore=shutil.ignore_patterns('historical'))
        fn(os.path.join(d, 'c'))
        r = subprocess.run([sys.executable, '-I', os.path.join(HERE, 'validate_corpus.py'), cmd, os.path.join(d, 'c'), rates if cmd == 'check' else previous],
                           capture_output=True, text=True)
        caught = r.returncode == 1
        fails = [l[5:] for l in r.stdout.splitlines() if l.startswith('FAIL ')]
        hit = next((l for l in fails if EXPECT.get(name, '\0') in l), None)
        results[name] = ('caught: ' + hit[:170]) if caught and hit else ('CAUGHT FOR ANOTHER REASON: ' + (fails[0][:150] if fails else '')) if caught else 'MISSED'
        caught = caught and hit is not None
        ok &= caught
print(json.dumps(results, indent=1))
print('ALL CAUGHT' if ok else 'SOME MISSED')
sys.exit(0 if ok else 1)
