"""Inventory every scope_budget record; dedupe by objective_id, prefer the live assignment.json."""
import json, sys, os, collections
paths = [l.strip() for l in open(sys.argv[1]) if l.strip()]
by_obj = collections.defaultdict(list); skipped = []
for p in paths:
    try: d = json.load(open(p))
    except Exception as e: skipped.append((p, 'unreadable')); continue
    sb = d.get('scope_budget') if isinstance(d, dict) else None
    if not isinstance(sb, dict) or 'objective_id' not in sb and 'objective' not in sb:
        skipped.append((p, 'no scope_budget key at top')); continue
    oid = sb.get('objective_id') or ('v1:' + p)
    by_obj[oid].append((p, sb))
rows = []
for oid, lst in by_obj.items():
    # prefer a file literally named assignment.json; among those, the one with closed_at or latest limit_history
    lst.sort(key=lambda x: (os.path.basename(x[0]) != 'assignment.json', not x[1].get('closed_at'), -len(json.dumps(x[1]))))
    p, sb = lst[0]
    rows.append(dict(objective_id=oid, path=p, copies=len(lst), alt=[x[0] for x in lst[1:]],
        objective=sb.get('objective'), started_at=sb.get('started_at'), closed_at=sb.get('closed_at'),
        disposition=(sb.get('close') or {}).get('disposition'), close_reason=(sb.get('close') or {}).get('reason'),
        limits=sb.get('limits'), limits_set_by=sb.get('limits_set_by'), cost_limits=sb.get('cost_limits'),
        items=len(sb.get('original_items') or []), additions=len(sb.get('additions') or []),
        sessions=[(s.get('session_id'), s.get('provider')) for s in sb.get('sessions') or []],
        bindings=len(sb.get('bindings') or []), revisions=len(sb.get('limit_history') or []), schema=sb.get('schema')))
rows.sort(key=lambda r: r['started_at'] or '')
json.dump(dict(rows=rows, skipped=skipped, files=len(paths)), open(sys.argv[2], 'w'), indent=1)
print('files', len(paths), 'objectives', len(rows), 'skipped', len(skipped))
for s in skipped: print('  skip', s)
