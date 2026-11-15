"""P1.34 offline reproduction of the sealed Module A findings (A-01..A-09) against the FROZEN revision 5.
Reads only frozen files (revision-5 workspace, deployed release 92a03789); writes only under this repro/ directory.
Each check prints REPRODUCED / NOT REPRODUCED with the concrete evidence. Usage: repro_p134.py"""
import copy, hashlib, json, os, re, shutil, subprocess, sys
sys.dont_write_bytecode = True   # never write into the frozen validator directory

S1 = '/home/bline/cbm-work/p133-s1r5/s1'; MAN = 'c20174532aa0731656a7a749b63000a79f377b97ddab638c420801d34e4af31b'
REL = '/home/bline/.local/share/work-engine/skill-releases/releases/92a03789327777b3cd9b5822c6df75a18fa63660/skills/manual-orchestration/scripts'
PC = f'{REL}/pi_container.py'; PC_SHA = '780837341790780f84163ee6151b8ce66cff56f5a3f6ddf8c15f939dba50f470'
RATES = '/home/bline/cbm-work/p133-s1r5/packet/sources/producer'
HERE = os.path.dirname(os.path.abspath(__file__))
sha = lambda b: hashlib.sha256(b).hexdigest()
assert sha(open(f'{S1}/WORKSPACE.MANIFEST.sha256', 'rb').read()) == MAN
assert subprocess.run(['sha256sum', '-c', '--quiet', 'WORKSPACE.MANIFEST.sha256'], cwd=S1).returncode == 0
assert sha(open(PC, 'rb').read()) == PC_SHA
C = open(f'{S1}/contract/CONTRACT.md').read().split('\n'); T = open(f'{S1}/contract/TRANSITIONS.md').read().split('\n')
P = open(PC).read().split('\n')
case = lambda cid: json.load(open(f'{S1}/corpus/cases/{cid}.json'))
a0 = lambda c: c['expected']['attempts'][0]
def rows(c):
    out = []
    for s in c['inputs']['sessions']:
        for t in (s.get('logs') or {}).values(): out += [json.loads(l) for l in t['lines']]
    return out
res = {}
def report(fid, ok, *ev):
    res[fid] = ok; print(f'{fid}: {"REPRODUCED" if ok else "NOT REPRODUCED"}'); [print('   ', e) for e in ev]

# A-01 (P1): section 6.2 error-status rule (unconditional low 0 / AtLeast(R)) vs section 8 row (supported usage kept)
s62 = '\n'.join(C[430:435]); s8 = C[511]
c = case('P133-A02-error-status-with-priceable-usage'); x = a0(c)['exposure']
by62 = dict(low='0', high=('at_least', a0(c)['settlement']['reserved']))
report('A-01', 'low 0, high AtLeast(R [ProxyReservation])' in s62 and 'valuation low if `Valued`' in s8 and 'max(R, valuation high' in s8
       and x['low']['nano'] != by62['low'] and int(x['high']['nano']) > int(by62['high'][1]),
       f'CONTRACT.md:433: {C[432].strip()[:110]}', f'CONTRACT.md:512: {s8[:150]}',
       f'case P133-A02: expected low {x["low"]["nano"]} high {x["high"]["kind"]}({x["high"]["nano"]}); section 6.2 gives low 0 high AtLeast({by62["high"][1]}) -> lowers both bounds')

# A-02 (P1): N03 under mc-1 (MC-2 row family = provider label; MC-3 equality by family and id)
c = case('N03-api-row-other-provider'); r = [x for x in rows(c) if x['event'] == 'request'][0]
launch = c['inputs']['sessions'][0]['launch']['model']; fam = lambda p: 'openai' if p in (None, 'openai', 'openai-codex') else p
pinned = (fam(launch.split('/', 1)[0]), launch.split('/', 1)[1]); rowid = (fam(r.get('provider')), r['model'])
orates = json.load(open(f'{RATES}/openai_api_rates.json')); has_openai_rate = 'claude-fable-5-2' in json.dumps(orates)
e = a0(c); in_s2a = None
try:
    sys.path.insert(0, f'{S1}/validator'); import validate_corpus as vc
    in_s2a = 'N03-api-row-other-provider' in {k['case_id'] for k in vc.s2a_subset(vc.corpus_cases(f'{S1}/corpus'))}
except Exception as ex: in_s2a = f'not established ({type(ex).__name__})'
report('A-02', pinned != rowid and not has_openai_rate and e['replacement']['eligible'] and e['exposure']['high']['kind'] == 'exact'
       and int(e['exposure']['high']['nano']) < int(e['settlement']['reserved']),
       f'pinned identity {pinned} vs row identity {rowid} (provider label {r.get("provider")!r}); OpenAI rate for claude-fable-5-2: {has_openai_rate}',
       f'MC-3 (CONTRACT.md:283-285): unequal -> section 5 condition 8 ModelMismatch -> Rejected; V-1/V-6: identity without rate -> Unpriceable',
       f'expected: decision {e["settlement"]["decision"]}, replacement {e["replacement"]}, exposure Exact({e["exposure"]["high"]["nano"]}) < R {e["settlement"]["reserved"]}: reservation released',
       f'TRANSITIONS.md:105 also says "another provider ... as above, plus LaneContradiction": {T[104][:120]}', f'in S2a grading subset: {in_s2a}')

# A-03 (P1, later integration): gate receipts and binding are writer data; a forged gate with arbitrary receipts qualifies
src = case('P133-FG-VC14-authenticated-bound'); forged = copy.deepcopy(src); g = forged['inputs']['gates'][0]
for k, m in g['mechanisms'].items():
    m['receipt'] = {'ref': f'forged:{k}', 'sha256': sha(f'not an authority receipt {k}'.encode())}
work = os.path.join(HERE, 'scratch-forged-corpus'); shutil.rmtree(work, ignore_errors=True); shutil.copytree(f'{S1}/corpus', work)
json.dump(forged, open(f'{work}/cases/P133-FG-VC14-authenticated-bound.json', 'w'), indent=1, sort_keys=True)
chk = subprocess.run(['/usr/bin/python3', '-I', f'{S1}/validator/validate_corpus.py', 'check', work, RATES], capture_output=True, text=True)
shutil.rmtree(work)
verif_fields = [k for k in json.dumps(src['inputs']['gates']).split('"') if 'signature' in k or 'verif' in k]
report('A-03', chk.returncode == 0 and '0 failures' in chk.stdout and not verif_fields and any('writer: Authority | SameUidFile | Declaration' in l for l in C[195:200]),
       f'gate fields read by G-1..G-8 include writer="{src["inputs"]["gates"][0]["writer"]}" (a value the writer sets); no verification-result field: {verif_fields or "none"}',
       f'forged receipts (sha256 of arbitrary text) -> validator check on the full corpus: rc {chk.returncode}, "{chk.stdout.strip().splitlines()[-1][-60:]}" -> still admitted',
       'S1 and its validator cannot distinguish an authority receipt from a self-asserted one (contract defers signatures to stage E)')

# A-04 (P2): S constructs amount_source from replacement, which needs the valuation range S may not read; Outstanding is a unit variant
vc1, u08, a06 = a0(case('VC01-openai-usage-settlement')), a0(case('U08-missing-observed-tier')), a0(case('A06-reservation-usage-null-status-no-id'))
report('A-04', 'constructed by stage' in ' '.join(C[383:385]) and 'usage valuation range' in T[24] and C[375].strip().startswith('Obligation = Outstanding ')
       and vc1['settlement']['decision'] == u08['settlement']['decision'] == 'valid' and vc1['obligation']['amount_source'] != u08['obligation']['amount_source']
       and a06['obligation'] == {'state': 'outstanding', 'amount_source': 'reservation_bound'},
       f'CONTRACT.md:384-385: {C[383][:60]}… {C[384][:80]}', f'TRANSITIONS.md:25 S does not read: {T[24].split("|")[3].strip()}',
       f'VC01 and U08 both Valid(Usage): amount_source {vc1["obligation"]["amount_source"]} vs {u08["obligation"]["amount_source"]} (differ only by valuation completeness/priceability)',
       f'CONTRACT.md:376 `Outstanding` has no fields, but A06 expects {a06["obligation"]}')

# A-05 (P2): all-null usage object -> NotReported has no stated mapping
c = case('S02-control-error-status-after-reservation'); r = [x for x in rows(c) if x['event'] == 'request'][0]
nr = [l for l in C if 'NotReported' in l and ('//' in l or 'no usage' in l)]
report('A-05', isinstance(r.get('usage'), dict) and all(r['usage'].get(k) is None for k in ('input_tokens', 'output_tokens')) and a0(c)['valuation']['state'] == 'not_reported',
       f'S02-control row usage {r["usage"]}; expected valuation {a0(c)["valuation"]["state"]}', f'only definition: {nr[0].strip()[:100] if nr else "none"} (a present object with null counts is not mapped)')

# A-06 (P2): null-terminal rows that the writer cannot produce (response = {} when terminal is None)
writer = '\n'.join(P[1548:1551]); bad = []
for cid in ('VC03-reservation-settlement-with-usage', 'S02-control-error-status-after-reservation', 'P133-A02-error-status-with-priceable-usage'):
    c = case(cid)
    for x in rows(c):
        if x['event'] == 'request' and x.get('terminal') is None:
            nonnull = [k for k in ('response_model', 'observed_tier', 'response_id') if x.get(k) is not None] + [f'usage.{k}' for k, v in (x.get('usage') or {}).items() if v not in (None, 0) or (v == 0 and k != 'reasoning_tokens')]
            if nonnull: bad.append((cid, c['reachability'], nonnull))
blk = '\n'.join(P[1514:1570]); nonok200 = 'if resp.status != 200 and len(head) < 2000' in blk and 'if event.get("type") in TERMINAL_EVENTS:' in blk and 'settle(resp.status if terminal is not None or resp.status != 200 else None, response)' in blk
report('A-06', len(bad) == 3 and 'response = (terminal or {}).get("response") or {}' in writer,
       f'pi_container.py:1549: {P[1548].strip()}', *[f'{cid} [{reach}]: terminal null but non-null {nn}' for cid, reach, nn in bad],
       f'non-200 bodies are parsed for a terminal event and settle(resp.status, response) receives its usage (pi_container.py:1531-1543, :1569): {nonok200} -> an error-status row WITH usage is writer-reachable if the upstream streams a terminal event')

# A-07 (P2): P133-A01 reservation impossible under reserve_usd
c = case('P133-A01-incomplete-usage-above-reservation'); rv = [x for x in rows(c) if x['event'] == 'usd_reserved'][0]
rate = orates['models']['gpt-6-astra'] if 'models' in orates else orates['gpt-6-astra']
floor = rv['input_bound'] * max(rate['input'], rate['cache_write']) / 1e6 * min(orates.get('tier_multiplier', {'flex': 0.5}).values() if isinstance(orates.get('tier_multiplier'), dict) else [0.5])
report('A-07', rv['reserved_usd'] < floor and c['reachability'] != 'producer_unreachable',
       f'reserved_usd {rv["reserved_usd"]} with input_bound {rv["input_bound"]}; reserve_usd floor (input term only, cheapest multiplier) {floor:.4f} (pi_container.py:689-694)', f'label: class {c["class"]}, reachability {c["reachability"]}')

# A-08 (P2): malformed cache count without MalformedEvidence
c = case('P133-A04-cached-malformed'); e = a0(c)
report('A-08', e['valuation']['state'] == 'unpriceable' and not any('Malformed' in b for b in e['blocking']) and e['settlement']['decision'] == 'valid',
       f'expected blocking {e["blocking"]}, settlement {e["settlement"]["decision"]}/{e["settlement"]["basis"]}', f'CONTRACT.md:574-575: {C[573].strip()[:120]}')

# A-09 (P2): condition 4 has no unique referent under conflicting reservations
c = case('P133-A09-conflicting-reservations-with-settlement'); rs = [x['reserved_usd'] for x in rows(c) if x['event'] == 'usd_reserved']; e = a0(c)
report('A-09', len(set(rs)) > 1 and "the reservation's amount" in C[332] and e['settlement']['reasons'] == ['ReservationConflict'],
       f'reservations {rs}; settlement.reserved {e["settlement"]["reserved"]}; expected reasons {e["settlement"]["reasons"]}', f'CONTRACT.md:333: {C[332].strip()}',
       'comparing against the 0.3 reservation would also give ReservedMismatch; "every failing reason" (CONTRACT.md:329) is not determined')

print('\nSUMMARY', {k: v for k, v in res.items()}, f'{sum(res.values())}/{len(res)} reproduced')
sys.exit(0 if all(res.values()) else 1)
