"""Per objective: root spend before the first child call (setup), between first and last child call (middle),
after the last child call (closeout). Objectives with no child calls are reported separately."""
import json, os, sys, collections, bisect, statistics as st
B = os.path.expanduser('~/cbm-work/budget-analysis-20261104')
tab = json.load(open(sys.argv[1]))
need = {}
for r in tab:
    if r.get('nodes') is None: continue
    t = json.load(open(f"{B}/trees/{r['objective_id'].replace(':','_').replace('/','_')}.json"))
    roots = {s[0] for s in r['sessions']}
    kids = [n for n in t['nodes'] if n['link'] not in ('root', 'successor') and n['first_call']]
    r['_w'] = (t['window']['since'], t['window']['until'])
    r['_kid'] = (min(n['first_call'] for n in kids), max(n['last_call'] for n in kids)) if kids else None
    for s in roots: need.setdefault(s, []).append(r)
calls = collections.defaultdict(list)
for line in open(f'{B}/ledger/calls.jsonl'):
    c = json.loads(line); sid = c.get('thread') if c['provider'] == 'codex' else c.get('session')
    if sid in need and c.get('ts') and not c.get('subagent'): calls[sid].append((c['ts'].replace('Z', '+00:00'), c.get('speed_adjusted_api_usd_low') or 0))
def norm(s): return s.replace('Z', '+00:00')
res = []
seen = set()
for sid, objs in need.items():
    for r in objs:
        if r['objective_id'] in seen: continue
        seen.add(r['objective_id'])
        s, u = norm(r['_w'][0]), norm(r['_w'][1])
        roots = {x[0] for x in r['sessions']}
        inw = [(t, v) for rs in roots for t, v in calls.get(rs, []) if s <= t <= u]
        tot = sum(v for _, v in inw)
        if r['_kid']:
            a, b = norm(r['_kid'][0]), norm(r['_kid'][1])
            pre = sum(v for t, v in inw if t < a); post = sum(v for t, v in inw if t > b)
            res.append(dict(r=r, root=tot, pre=pre, post=post, mid=tot - pre - post, kids=True))
        else:
            res.append(dict(r=r, root=tot, kids=False))
K = [x for x in res if x['kids'] and x['r']['spend_lo'] > 0]
tot_spend = sum(x['r']['spend_lo'] for x in K)
print('objectives with child work: %d; root-only: %d' % (len(K), sum(1 for x in res if not x['kids'])))
print('share of total objective spend: setup %.0f%%, closeout %.0f%%, root during child work %.0f%%' % (
    100 * sum(x['pre'] for x in K) / tot_spend, 100 * sum(x['post'] for x in K) / tot_spend, 100 * sum(x['mid'] for x in K) / tot_spend))
for lab, f in (('setup $', 'pre'), ('closeout $', 'post')):
    xs = sorted(x[f] for x in K); print(' %-10s median %.2f  p75 %.2f  max %.2f' % (lab, st.median(xs), xs[int(.75 * (len(xs) - 1))], xs[-1]))
sh = sorted((x['pre'] + x['post']) / x['r']['spend_lo'] for x in K)
print(' setup+closeout as share of each objective: median %.0f%%  p25 %.0f%%  p75 %.0f%%' % (100 * st.median(sh), 100 * sh[len(sh) // 4], 100 * sh[3 * len(sh) // 4]))
by = collections.defaultdict(list)
for x in K: by[x['r'].get('era', '?')].append(x)
for e, L in sorted(by.items()):
    ts_ = sum(x['r']['spend_lo'] for x in L)
    print('  era %s n=%d setup %.0f%% closeout %.0f%%' % (e, len(L), 100 * sum(x['pre'] for x in L) / ts_, 100 * sum(x['post'] for x in L) / ts_))
json.dump([dict(objective_id=x['r']['objective_id'], root=x['root'], pre=x.get('pre'), post=x.get('post'), kids=x['kids']) for x in res], open(f'{B}/phases.json', 'w'), indent=1)
