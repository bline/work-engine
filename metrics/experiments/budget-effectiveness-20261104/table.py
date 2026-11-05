import json, os, sys, collections
B = os.path.expanduser('~/cbm-work/budget-analysis-20261104')
rows = json.load(open(sys.argv[1] if len(sys.argv) > 1 else f'{B}/resolved.json'))
out = []
def spend(p):  # speed-adjusted, low/high, all pools except fable (subset of claude_all_models)
    lo = p['codex']['quota_usd_low'] + p['claude_all_models']['quota_usd_low']
    hi = p['codex']['quota_usd_high'] + p['claude_all_models']['quota_usd_high']
    return lo, hi
for r in rows:
    f = f"{B}/trees/{r['objective_id'].replace(':','_').replace('/','_')}.json"
    if not os.path.exists(f): r['tree'] = None; out.append(r); continue
    t = json.load(open(f))
    lo, hi = spend(t['totals'])
    r['spend_lo'], r['spend_hi'] = lo, hi
    r['tree_status'] = t['status']
    roots = {s[0] for s in r['sessions']}
    root_lo = sum(n['direct']['codex']['quota_usd_low'] + n['direct']['claude_all_models']['quota_usd_low'] for n in t['nodes'] if n['session_id'] in roots and n['link'] in ('root','successor'))
    r['root_share'] = root_lo / lo if lo else None
    r['nodes'] = len(t['nodes'])
    r['child_models'] = sorted({m for n in t['nodes'] if n['link'] not in ('root','successor') for m in n['models'] if m})
    pc = t.get('post_close')
    r['tail_lo'] = spend(pc['candidate_totals'])[0] if pc and pc.get('candidate_totals') else None
    r['issues'] = collections.Counter(i['kind'] for i in t['issues'])
    r['unattributed_lo'] = spend(t['unattributed']['totals'])[0]
    # limits
    lim = {}
    for c in r['cost_limits'] or []:
        if c.get('node') is None and c.get('max_usd') is not None:
            lim[c['set_by']] = c['max_usd']
    r['user_limit'] = lim.get('user'); r['sup_limit'] = lim.get('supervisor')
    r['node_limits'] = sum(1 for c in r['cost_limits'] or [] if c.get('node'))
    r['n_cost_entries'] = len(r['cost_limits'] or [])
    out.append(r)
json.dump(out, open(sys.argv[2] if len(sys.argv) > 2 else f'{B}/table.json', 'w'), indent=1, default=str)
print('start            h_lim  h_act   user$  sup$   spend$  ratioU ratioT  root%  tail$  st   nodes entries disp  record')
for r in out:
    if r.get('spend_lo') is None: print(r['started_at'][:16], 'NO TREE', r['path']); continue
    lim = r['limits'] or {}
    tight = min([x for x in (r['user_limit'], r['sup_limit']) if x is not None], default=None)
    print(r['started_at'][:16], '%5s %5.2f %6s %6s %7.2f %6s %6s %5s %6s %-4s %3d %3d %-10s %s' % (
        ('%.2f' % lim['max_hours']) if lim.get('max_hours') else '-', r['elapsed_h'] or 0,
        r['user_limit'], r['sup_limit'], r['spend_lo'],
        ('%.2f' % (r['spend_lo']/r['user_limit'])) if r['user_limit'] else '-',
        ('%.2f' % (r['spend_lo']/tight)) if tight else '-',
        ('%.0f' % (100*r['root_share'])) if r['root_share'] is not None else '-',
        ('%.2f' % r['tail_lo']) if r['tail_lo'] is not None else '-',
        r['tree_status'][:4], r['nodes'], r['n_cost_entries'], str(r['status'])[:10], r['path'].replace('/home/bline/','')[-60:]))
