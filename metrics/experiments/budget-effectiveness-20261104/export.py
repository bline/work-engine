"""Build the committed observation JSON for the 2026-11-04 objective-budget analysis.

Reads the private working directory (BUDGET_ANALYSIS_DIR, default ~/cbm-work/budget-analysis-20261104):
table-full.json (per-objective rows from table.py + labels/outcomes), phases.json, ledger/calls.jsonl,
trees/*.json. Recomputes every summary figure; writes OUT (argv[1]).
"""
import collections, datetime as dt, hashlib, json, os, re, statistics as st, subprocess, sys

B = os.environ.get('BUDGET_ANALYSIS_DIR', os.path.expanduser('~/cbm-work/budget-analysis-20261104'))
OUT = sys.argv[1]
REPO = '/home/bline/code/work-engine'
ts = lambda s: dt.datetime.fromisoformat(str(s).replace('Z', '+00:00'))
sha = lambda p: hashlib.sha256(open(p, 'rb').read()).hexdigest()
tree_path = lambda oid: f"{B}/trees/{oid.replace(':', '_').replace('/', '_')}.json"

T = json.load(open(f'{B}/table-full.json'))
for r in T:
    tight = min([x for x in (r.get('user_limit'), r.get('sup_limit')) if x is not None], default=None)
    r['tight'] = tight
    r['ratio'] = r['spend_lo'] / tight if tight else None
    mh = (r['limits'] or {}).get('max_hours')
    r['hratio'] = r['elapsed_h'] / mh if mh and r['elapsed_h'] is not None else None
    r['closed'] = r['outcome'] != 'open' and not (r['started_at'] >= '2026-11-04T22' and not r.get('closed_at'))
SUBSETS = {
    'explicit_close': lambda r: r['closed'] and r['end_basis'] != 'mtime',
    'plausible_end': lambda r: r['closed'] and not (r['end_basis'] == 'mtime' and r['outcome'] == 'unknown'),
    'all_closed': lambda r: r['closed'],
}
FAIL = ('not_accepted', 'abandoned')

def dist(xs):
    xs = sorted(x for x in xs if x is not None)
    if not xs: return dict(n=0)
    p = lambda f: xs[int(f * (len(xs) - 1) + 0.5)]
    return dict(n=len(xs), median=round(st.median(xs), 4), p25=round(p(.25), 4), p75=round(p(.75), 4),
                p90=round(p(.9), 4), min=round(xs[0], 4), max=round(xs[-1], 4))

def calibration(R):
    L = [r for r in R if r['ratio'] is not None]; H = [r for r in R if r['hratio'] is not None]
    fail = [r for r in L if r['outcome'] in FAIL]; hit = [r for r in L if r['ratio'] >= .95]
    return dict(
        objectives=len(R), spend_usd=round(sum(r['spend_lo'] for r in R), 2),
        spend_over_tightest_usd_limit=dist(r['ratio'] for r in R),
        elapsed_over_hours_limit=dist(r['hratio'] for r in R),
        root_share_of_spend=dist(r['root_share'] for r in R),
        objective_spend_usd=dist(r['spend_lo'] for r in R),
        reached_95pct_usd_limit=sum(r['ratio'] >= .95 for r in L), with_usd_limit=len(L),
        exceeded_usd_limit=[round(r['ratio'], 3) for r in L if r['ratio'] > 1.0001],
        reached_95pct_hours=sum(r['hratio'] >= .95 for r in H), with_hours_limit=len(H),
        exceeded_hours_limit=[round(r['hratio'], 3) for r in H if r['hratio'] > 1.0001],
        max_elapsed_h=round(max(r['elapsed_h'] for r in R), 3),
        outcomes=dict(collections.Counter(r['outcome'] for r in R)),
        not_accepted_with_usd_limit=len(fail),
        not_accepted_below_half_limit=sum(r['ratio'] < .5 for r in fail),
        not_accepted_at_95pct=sum(r['ratio'] >= .95 for r in fail),
        reached_95pct_not_accepted=sum(r['outcome'] in FAIL for r in hit),
        not_accepted_spend_usd=round(sum(r['spend_lo'] for r in R if r['outcome'] in FAIL), 2),
    )

summary = {k: calibration([r for r in T if f(r)]) for k, f in SUBSETS.items()}
P = [r for r in T if SUBSETS['plausible_end'](r)]

by_type = {}
for t in ('plan', 'review', 'test', 'build', 'live'):
    S = [r for r in P if r['type'] == t]
    by_type[t] = dict(n=len(S), accepted=sum(r['outcome'] == 'accepted' for r in S),
                      not_accepted=sum(r['outcome'] in FAIL for r in S),
                      spend_usd=dist(r['spend_lo'] for r in S), tightest_limit_usd=dist(r['tight'] for r in S),
                      spend_over_limit=dist(r['ratio'] for r in S), root_share=dist(r['root_share'] for r in S),
                      not_accepted_spend_usd=round(sum(r['spend_lo'] for r in S if r['outcome'] in FAIL), 2))
by_era = {}
for e in sorted({r['era'] for r in P}):
    S = [r for r in P if r['era'] == e]
    by_era[e] = dict(n=len(S), spend_over_limit=dist(r['ratio'] for r in S), spend_usd=dist(r['spend_lo'] for r in S),
                     root_share=dist(r['root_share'] for r in S), outcomes=dict(collections.Counter(r['outcome'] for r in S)))

# Root setup/closeout (phases.py output, plausible-end set)
ph = {p['objective_id']: p for p in json.load(open(f'{B}/phases.json'))}
Tk = {r['objective_id']: r for r in T}
K = [(Tk[o], p) for o, p in ph.items() if p['kids'] and Tk[o]['spend_lo'] > 0]
tot = sum(r['spend_lo'] for r, _ in K)
rows_sz = sorted(((r['spend_lo'], (p['pre'] + p['post']) / r['spend_lo'], r['outcome']) for r, p in K))
k3 = len(rows_sz) // 3
size = {}
for name, S in (('small', rows_sz[:k3]), ('medium', rows_sz[k3:2 * k3]), ('large', rows_sz[2 * k3:])):
    size[name] = dict(n=len(S), spend_range_usd=[round(S[0][0], 2), round(S[-1][0], 2)],
                      setup_closeout_share_median=round(st.median(x[1] for x in S), 4),
                      accepted=sum(x[2] == 'accepted' for x in S), not_accepted=sum(x[2] in FAIL for x in S))
phases = dict(objectives_with_child_work=len(K), root_only_objectives=sum(1 for p in ph.values() if not p['kids']),
              setup_share_of_spend=round(sum(p['pre'] for _, p in K) / tot, 4),
              closeout_share_of_spend=round(sum(p['post'] for _, p in K) / tot, 4),
              root_during_child_work_share=round(sum(p['root'] - p['pre'] - p['post'] for _, p in K) / tot, 4),
              setup_usd=dist(p['pre'] for _, p in K), closeout_usd=dist(p['post'] for _, p in K),
              setup_closeout_share_per_objective=dist((p['pre'] + p['post']) / r['spend_lo'] for r, p in K),
              by_size_tertile=size)

# Coverage and outside-window root spend, from the ledger and each objective's tree
spans = collections.defaultdict(list); win = collections.defaultdict(list)
for r in T:
    t = json.load(open(tree_path(r['objective_id'])))
    s, u = ts(t['window']['since']), ts(t['window']['until'])
    for n in t['nodes']: spans[n['session_id']].append((s, u))
    win[r['sessions'][0][0]].append((s, u))
daily = collections.defaultdict(collections.Counter); inside = collections.Counter(); outside = collections.Counter()
near = collections.Counter(); unb = collections.Counter()
for line in open(f'{B}/ledger/calls.jsonl'):
    c = json.loads(line)
    if not c.get('ts'): continue
    sid = c.get('thread') if c['provider'] == 'codex' else c.get('session')
    t = ts(c['ts']); v = c.get('speed_adjusted_api_usd_low') or 0; day = c['ts'][:10]
    prov = 'codex' if c['provider'] == 'codex' else 'claude'
    daily[day][prov] += v
    covered = any(s <= t <= u for s, u in spans.get(sid, ()))
    if day >= '2026-10-29':
        daily[day]['budgeted'] += v if covered else 0
    if c['ts'] < '2026-10-30': continue
    if not covered:
        unb['budget_root_outside_objectives' if sid in win else ('claude' if prov == 'claude' else 'other_codex')] += v
    if sid in win and not c.get('subagent'):
        W = win[sid]
        if any(s <= t <= u for s, u in W): inside[sid] += v; continue
        outside[sid] += v
        after = min(((t - u).total_seconds() / 60 for s, u in W if u < t), default=None)
        before = min(((s - t).total_seconds() / 60 for s, u in W if s > t), default=None)
        near['within_30min_after_close' if after is not None and after <= 30 else
             'within_30min_before_start' if before is not None and before <= 30 else 'further_than_30min'] += v
cov_days = {d: dict(all_usd=round(x['codex'] + x['claude'], 2), budgeted_usd=round(x['budgeted'], 2))
            for d, x in sorted(daily.items()) if d >= '2026-10-29'}
from_1030 = [v for d, v in cov_days.items() if d >= '2026-10-30']
coverage = dict(per_day=cov_days,
                since_2026_10_30=dict(all_usd=round(sum(x['all_usd'] for x in from_1030), 2),
                                      budgeted_usd=round(sum(x['budgeted_usd'] for x in from_1030), 2)),
                unbudgeted_since_2026_10_30_usd={k: round(v, 2) for k, v in unb.items()},
                budget_roots=dict(inside_usd=round(sum(inside.values()), 2), outside_usd=round(sum(outside.values()), 2),
                                  outside_by_proximity_usd={k: round(v, 2) for k, v in near.items()}))
daily_spend = {d: dict(codex_usd=round(x['codex'], 2), claude_usd=round(x['claude'], 2)) for d, x in sorted(daily.items())}

# Budget-command share of root spend (overhead.py; invocation-only pattern)
def run_overhead(since, until, threads):
    out = subprocess.run([sys.executable, '-I', f'{B}/scripts/overhead.py', f'{B}/ledger/calls.jsonl', since, until, *threads],
                         capture_output=True, text=True, check=True).stdout
    res = []
    for line in out.splitlines():
        m = re.match(r'(\w+) calls\s+(\d+) usd\s+([\d.]+) \| strict\s+([\d.]+)% \((\d+) calls\)', line)
        if m: res.append(dict(root=m[1], calls=int(m[2]), usd=float(m[3]), budget_command_share=float(m[4]) / 100, budget_command_calls=int(m[5])))
    return res
overhead = dict(
    pattern=open(f'{B}/scripts/overhead_re.py').read().strip().splitlines()[-1],
    convention='a model call is one token_usage_record; its tool calls are those issued since the previous record',
    windows=[dict(since='2026-10-30T12:00', until='2026-11-01T00:00',
                  roots=run_overhead('2026-10-30T12:00', '2026-11-01T00:00', ['01a18305-58a4-79c0-b319-7d0cf7ab1e8a', '01a18390-aafb-7252-b402-8c85f55bd86f'])),
             dict(since='2026-11-02T12:00', until='2026-11-05T00:00',
                  roots=run_overhead('2026-11-02T12:00', '2026-11-05T00:00', ['01a1a2c0-7191-76f2-afa1-16d32d5c4883', '01a195d4-b7a7-77f2-ba65-277669c0ec97',
                                     '01a19d59-05c0-7861-b8d5-b227b13a9f1e', '01a1a7ed-45f5-7041-9db5-a6c9aa1cd3f3', '01a1a862-034f-7750-b86b-de29e11cf327']))])

KEEP = ('objective_id', 'path', 'objective', 'sessions', 'started_at', 'end', 'end_basis', 'elapsed_h', 'closed_at',
        'status', 'outcome', 'type', 'era', 'limits', 'limits_set_by', 'user_limit', 'sup_limit', 'tight', 'node_limits',
        'n_cost_entries', 'additions', 'items', 'spend_lo', 'spend_hi', 'ratio', 'hratio', 'root_share', 'nodes',
        'tree_status', 'unattributed_lo', 'close_reason')
objectives = []
for r in T:
    row = {k: r.get(k) for k in KEEP}
    p = ph.get(r['objective_id'])
    row['root_setup_usd'] = round(p['pre'], 4) if p and p['kids'] else None
    row['root_closeout_usd'] = round(p['post'], 4) if p and p['kids'] else None
    row['in_plausible_end_set'] = SUBSETS['plausible_end'](r)
    row['tree_sha256'] = sha(tree_path(r['objective_id']))
    objectives.append(row)

scripts = ['discover.py', 'inventory.py', 'resolve.py', 'canon.py', 'run_tree.sh', 'table.py', 'classify.py', 'phases.py',
           'overhead.py', 'overhead_re.py', 'sample_hits.py', 'export.py']
doc = dict(
    schema='objective-budget-effectiveness-v1',
    generated_at=dt.datetime.now(dt.timezone.utc).isoformat(timespec='seconds'),
    question='How have scope budgets performed since they started (2026-10-30), and can the data improve results?',
    unit='speed-adjusted API-equivalent USD (usage-ledger-standard-api-v1); not an invoice or subscription quota',
    reader=dict(path='metrics/scripts/usage_ledger.py', sha256=sha(f'{REPO}/metrics/scripts/usage_ledger.py')),
    ledger=dict(since='2026-10-23T00:00:00Z', built_at='2026-11-04T22:43Z', calls_sha256=sha(f'{B}/ledger/calls.jsonl')),
    discovery=dict(
        record_files=sum(1 for l in open(f'{B}/records-final.txt') if l.strip()),
        records_list_sha256=sha(f'{B}/records-final.txt'),
        distinct_objective_ids=235, canonical_objectives=len(T),
        set_aside=dict(no_bound_session=9, smoke_test=1, views_and_copies=66),
        basis=('Every .json file up to 4 levels below ~/.local/state/work-engine modified since 2026-10-29 (44,178 files, '
               '161 records), all contained in a targeted walk of 133 directories modified since 10-29, 7 named roots, '
               'the B3 qualification package and ~/Documents/Codex/2026-10-29..31 (291 records). Records deeper than 4 levels '
               'in directories outside the targeted set are not established.'),
        dedupe='canonical file names only (assignment.json, scope.json, budget.json, preflight-accounting.json, '
               'prior-component-settlement-input.json); one record per (root session, start minute), preferring closed, '
               'local-state, limit-bearing records'),
    subsets=dict(
        explicit_close='closed objectives whose end comes from scope_budget.closed_at, a record closed_at or a result file closed_at',
        plausible_end='closed objectives, excluding those whose end is file modification time and whose outcome is unknown',
        all_closed='every closed objective; open = status running/awaiting, or started after 2026-11-04T22:00Z without closed_at'),
    summary=summary, by_type=by_type, by_era=by_era, root_phases=phases, coverage=coverage,
    budget_command_overhead=overhead, daily_spend=daily_spend,
    scripts={s: sha(f'{B}/scripts/{s}') for s in scripts},
    type_labels_sha256=sha(f'{B}/type-labels.json'),
    limitations=[
        'Types (plan/review/test/build/live) and the mapping of free-text status to outcome are one rater\'s judgment.',
        '"accepted" is the root\'s recorded disposition and often means a bounded partial result, not product acceptance.',
        '85 of 159 objectives take their end from record file modification time; headline figures are reported on three subsets.',
        'Observational only: no comparison group; associations are not effects of budgeting.',
        'Root setup/closeout uses child first/last call times; root work interleaved with children is not split further.',
        'One objective (FIFTH native-message-g001) attributes $0 with $1.69 unattributed in its window: a lineage gap.',
        'Budget-command share is a lower bound: budget arithmetic inside general record scripts is not counted.',
        'Daily spend fell after 10-28, overlapping the Astra to Sol 6.1 routing change; not attributed to budgets.',
    ],
    objectives=objectives,
)
json.dump(doc, open(OUT, 'w'), indent=1, default=str)
print('wrote', OUT, 'objectives', len(objectives))
print(json.dumps(dict(summary=summary, by_type=by_type, by_era=by_era, root_phases=phases,
                      coverage={k: v for k, v in coverage.items() if k != 'per_day'}, overhead=overhead), indent=1, default=str))
