import json, os, collections
B = os.path.expanduser('~/cbm-work/budget-analysis-20261104')
rows = json.load(open(f'{B}/resolved.json'))
CANON = {'assignment.json', 'scope.json', 'budget.json', 'preflight-accounting.json', 'prior-component-settlement-input.json'}
groups = collections.defaultdict(list); dropped = collections.Counter(); views = []
for r in rows:
    if not r['sessions']: dropped['no_session'] += 1; continue
    if 'smoke' in r['path']: dropped['smoke'] += 1; continue
    if r['file'] not in CANON: views.append(r); continue
    groups[(r['sessions'][0][0], r['started_at'][:16])].append(r)
canon = []
for k, v in groups.items():
    v.sort(key=lambda r: (not r.get('closed_at'), '/Documents/' in r['path'], not r['cost_limits'], r['end_basis'] == 'mtime'))
    canon.append(v[0]); dropped['duplicate_in_group'] += len(v) - 1
canon.sort(key=lambda r: r['started_at'])
json.dump(canon, open(f'{B}/canonical.json', 'w'), indent=1); json.dump(views, open(f'{B}/views.json', 'w'), indent=1)
print('records', len(rows), 'canonical objectives', len(canon), 'views/copies set aside', len(views), dict(dropped))
