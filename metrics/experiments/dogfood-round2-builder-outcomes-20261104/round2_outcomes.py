#!/usr/bin/env python3
"""Round 2 dogfood (Terra vs Sol builders): outcomes and attributable spend per arm.

Usage: python3 -I round2_outcomes.py RUN_DIR LEDGER_CALLS_JSONL

Reads the round's enrollment, first-review, repair and close records and the
assignment records' child sessions, and prices builder and first-reviewer
sessions from a usage_ledger.py calls.jsonl. A session shared by two
enrollments is counted once per arm in spend totals and medians. Re-reviews and
root coordination are not attributed. Costs are the ledger's API-equivalent
USD, not quota.
"""
import collections, glob, itertools, json, math, os, statistics, sys

run, ledger = sys.argv[1], sys.argv[2]
spend = collections.defaultdict(float); models = collections.defaultdict(collections.Counter)
for line in open(ledger):
    d = json.loads(line); k = d.get('thread') or d.get('session')
    spend[k] += d.get('usd') or 0; models[k][f"{d['model']}/{d.get('effort')}"] += 1

def load(p): return json.load(open(p)) if os.path.exists(p) else {}
rows = []
for d in sorted(glob.glob(f'{run}/enrollments/*')):
    tid = os.path.basename(d); e = load(f'{d}/enrollment.json'); r = load(f'{d}/review-first.json'); c = load(f'{d}/close.json')
    kids = tuple(s for p in glob.glob(f'{run}/task-profiles/{tid}@*/assignment.json') for s in load(p).get('child_sessions') or [])
    rows.append(dict(task=tid, arm=e['arm'], first=r.get('verdict'), p=(r.get('p1'), r.get('p2'), r.get('p3')),
                     repairs=c.get('repair_rounds', len(glob.glob(f'{d}/repair-*.json'))), disposition=c.get('disposition', 'open'),
                     builders=kids, reviewer=r.get('reviewer_session'),
                     builder_usd=sum(spend[k] for k in kids), review_usd=spend[r['reviewer_session']] if r.get('reviewer_session') else None,
                     builder_models=dict(sum((models[k] for k in kids), collections.Counter()))))

print('| Arm | Task | First review | P1/P2/P3 | Repair rounds | Disposition | Builder $ | First review $ |')
print('|---|---|---|---|---:|---|---:|---:|')
for x in rows:
    rv = f"{x['review_usd']:.2f}" if x['review_usd'] is not None else '-'
    print(f"| {x['arm'].replace('gpt-', '')} | `{x['task']}` | {x['first'] or 'none'} | {'/'.join(str(v) for v in x['p']) if x['first'] else '-'} | {x['repairs']} | {x['disposition']} | {x['builder_usd']:.2f} | {rv} |")

print()
for arm in sorted({x['arm'] for x in rows}):
    a = [x for x in rows if x['arm'] == arm]
    bs = {x['builders']: x['builder_usd'] for x in a if x['builders']}           # each builder session set once
    rs = {x['reviewer']: x['review_usd'] for x in a if x['reviewer']}             # each reviewer session once
    pairs = {(x['builders'], x['reviewer']): (x['builder_usd'], x['review_usd']) for x in a if x['builders'] and x['reviewer']}
    ratios = sorted(r / b for b, r in pairs.values() if b)
    print(f"{arm}: n={len(a)}; first review accept {sum(x['first']=='accept' for x in a)}, changes_required {sum(x['first']=='changes_required' for x in a)}, none {sum(not x['first'] for x in a)}; "
          f"repair rounds {sum(x['repairs'] for x in a)} ({sum(x['repairs'] for x in a)/len(a):.2f}/task); tasks with a P1 at first review {sum((x['p'][0] or 0) > 0 for x in a)}; "
          f"dispositions {dict(collections.Counter(x['disposition'] for x in a))}")
    print(f"   builder sessions {len(bs)}: total ${sum(bs.values()):.2f}, median ${statistics.median(bs.values()):.2f}; first-review sessions {len(rs)}: total ${sum(rs.values()):.2f}, median ${statistics.median(rs.values()):.2f}; "
          f"first review / build per pair: median {statistics.median(ratios):.1f}x, range {ratios[0]:.1f}-{ratios[-1]:.1f}x (n={len(ratios)})")

sol = [x['repairs'] for x in rows if x['arm'] == 'gpt-6.1-sol']; terra = [x['repairs'] for x in rows if x['arm'] == 'gpt-6.1-terra']
allv = sol + terra; obs = sum(terra) / len(terra) - sum(sol) / len(sol); hits = tot = 0
for idx in itertools.combinations(range(len(allv)), len(terra)):
    t = [allv[i] for i in idx]; s = [allv[i] for i in range(len(allv)) if i not in idx]
    tot += 1; hits += sum(t) / len(t) - sum(s) / len(s) >= obs - 1e-12
acc = lambda arm: (sum(x['first'] == 'accept' for x in rows if x['arm'] == arm), sum(bool(x['first']) for x in rows if x['arm'] == arm))
(sa, sn), (ta, tn) = acc('gpt-6.1-sol'), acc('gpt-6.1-terra')
def fisher_ge(a, n1, c, n2):   # one-sided: P(first arm accepts >= a | margins)
    k = a + c; N = n1 + n2
    return sum(math.comb(n1, x) * math.comb(n2, k - x) for x in range(a, min(n1, k) + 1)) / math.comb(N, k)
print(f"\nrepair rounds per task, Terra minus Sol = {obs:.2f}; one-sided permutation p = {hits/tot:.3f} ({tot} splits)")
print(f"first-review accept Sol {sa}/{sn} vs Terra {ta}/{tn}; one-sided Fisher p = {fisher_ge(sa, sn, ta, tn):.3f}")
