"""Cases added in revision 2 (contract 1.1-s1), derived deterministically from two revision-2 cases.
  Q5-unrecognised-openai-tier          V-3 (Q5): an unrecognised observed_tier string leaves the tier set unestablished
  Q8-unadmitted-outside-candidate-window  O-6/O-7 (Q8): an unadmitted attempt blocks only objectives whose window could contain it
Usage: build_rev2_cases.py <corpus_dir> <out_cases_dir>   (run after revise_rev2.py; historical blocks are added separately)"""
import copy, json, os, sys

corpus, out = sys.argv[1], sys.argv[2]
load = lambda cid: json.load(open(os.path.join(corpus, 'cases', cid + '.json')))
Z = {'low': '0', 'high': {'kind': 'exact', 'nano': '0'}}


def base(c, cid, group, role, cls, reach, origin, invariants, controls, notes):
    c = copy.deepcopy(c)
    c.update(case_id=cid, group=group, role=role, findings=[], controls=controls, invariants=invariants, provenance={'origin': origin},
             reachability=reach, notes=notes, rationale=None, ambiguity=None, revisions=[], added_in='1.1-s1', historical={},
             contract='pi-accounting-contract/1.1-s1')
    c['class'] = cls
    c.pop('twin_of', None)
    return c


# ---- Q5: unrecognised OpenAI tier string
c = base(load('U08-missing-observed-tier'), 'Q5-unrecognised-openai-tier', 'Q5', 'control', 'producer_valid', 'producer_reachable',
         'contract::V-3 unrecognised observed_tier string (owner disposition Q5, P1.29)', ['INV-F5'],
         ['U08-missing-observed-tier', 'VC01-openai-usage-settlement'],
         'The proxy records the response service tier verbatim and charges an unrecognised tier at the highest table multiplier (2.0), '
         'so the settlement is valid. Whether the provider ever returns a tier outside the table is not established.')
log = c['inputs']['sessions'][0]['logs']['turn-1']['lines']
row = json.loads(log[1]); assert row['observed_tier'] is None; row['observed_tier'] = 'scale'
log[1] = json.dumps(row)
a = c['expected']['attempts'][0]
a['valuation'] = {'state': 'unpriceable', 'reasons': ['UnknownTier'], 'supported_low': None, 'priced_high': '30000000'}
a['exposure'] = {'pool': 'pi_api', 'low': {'nano': '0', 'basis': 'none'}, 'high': {'kind': 'at_least', 'nano': '30000000', 'basis': 'proxy_charge'}}
a['blocking'] = ['UnresolvedExposure']
ee = c['evaluations'][0]['expected']; o = ee['objectives'][0]
o['owned_closed'] = {'low': '0', 'high': {'kind': 'at_least', 'nano': '30000000'}}
o['reasons'] = ['UnresolvedExposure']; o['status'] = 'not_established'; ee['admission']['decision'] = 'not_established'
json.dump(c, open(os.path.join(out, c['case_id'] + '.json'), 'w'), indent=1, sort_keys=True); open(os.path.join(out, c['case_id'] + '.json'), 'a').write('\n')

# ---- Q8 / O-7: unadmitted request before a candidate objective's window
c = base(load('VC06-unbudgeted-completed-request'), 'Q8-unadmitted-outside-candidate-window', 'Q8', 'control', 'attribution_uncertain',
         'producer_reachable', 'contract::O-7 temporal candidate restriction (owner disposition Q8, P1.29)', ['INV-T2', 'INV-F7'],
         ['VC06-unbudgeted-completed-request'],
         'Same unbudgeted request (06:30:02) in two windows of the objective whose tree holds the session. In "window-contains" the '
         'objective started at 06:00 and is a candidate; in "window-starts-later" it started at 06:40, so the request lies wholly '
         'before its window and the objective is not a candidate. The attempt facts are identical in both evaluations.')
e1 = c['evaluations'][0]; e1['name'] = 'window-contains'
assert e1['expected']['attribution'][0]['ownership'] == 'unadmitted' and e1['expected']['objectives'][0]['reasons'] == ['HistoricalUnadmitted']
e2 = copy.deepcopy(e1); e2['name'] = 'window-starts-later'
e2['objectives'][0]['started_at'] = '2026-11-10T06:40:00Z'
e2['expected']['attribution'][0].update(candidates=[], placement='before')
o = e2['expected']['objectives'][0]; o['reasons'] = []; o['status'] = 'established'; e2['expected']['admission']['decision'] = 'admitted'
c['evaluations'] = [e1, e2]
json.dump(c, open(os.path.join(out, c['case_id'] + '.json'), 'w'), indent=1, sort_keys=True); open(os.path.join(out, c['case_id'] + '.json'), 'a').write('\n')
print('2 cases written')
