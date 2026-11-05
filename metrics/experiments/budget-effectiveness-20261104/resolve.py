"""For each inventoried objective: resolve window end and outcome from the record and its result files."""
import json, os, sys, datetime as dt
inv = json.load(open(sys.argv[1]))
def ts(s):
    try: return dt.datetime.fromisoformat(str(s).replace('Z', '+00:00'))
    except Exception: return None
def find_closed(obj, depth=0):
    """closed_at/status in a dict or in a result file it points at."""
    out = {}
    if not isinstance(obj, dict) or depth > 2: return out
    for k in ('closed_at', 'closedAt'):
        if obj.get(k) and ts(obj[k]): out.setdefault('closed_at', obj[k])
    for k in ('status',):
        if isinstance(obj.get(k), str): out.setdefault('status', obj[k])
    for k in ('result', 'rootResult', 'root_result'):
        v = obj.get(k); path = v if isinstance(v, str) else (v.get('path') if isinstance(v, dict) else None)
        if isinstance(v, dict):
            sub = find_closed(v, depth + 1)
            for kk, vv in sub.items(): out.setdefault(kk, vv)
        if isinstance(path, str) and path.endswith('.json') and os.path.exists(path):
            try: sub = find_closed(json.load(open(path)), depth + 1)
            except Exception: sub = {}
            for kk, vv in sub.items(): out.setdefault('result_' + kk, vv)
    return out
rows = []
for r in inv['rows']:
    d = json.load(open(r['path'])); sb = d['scope_budget']
    info = find_closed(d)
    end, basis = None, None
    if sb.get('closed_at'): end, basis = sb['closed_at'], 'scope_budget.closed_at'
    elif info.get('result_closed_at'): end, basis = info['result_closed_at'], 'result.closed_at'
    elif info.get('closed_at'): end, basis = info['closed_at'], 'record.closed_at'
    else:
        end, basis = dt.datetime.fromtimestamp(os.path.getmtime(r['path']), dt.timezone.utc).isoformat(), 'mtime'
    status = (sb.get('close') or {}).get('disposition') or info.get('status') or info.get('result_status')
    r2 = dict(r, end=end, end_basis=basis, status=status, file=os.path.basename(r['path']))
    s, e = ts(r['started_at']), ts(end)
    r2['elapsed_h'] = (e - s).total_seconds() / 3600 if s and e else None
    rows.append(r2)
json.dump(rows, open(sys.argv[2], 'w'), indent=1)
for r in rows:
    print(r['started_at'][:16], r['end_basis'][:14].ljust(14), '%.2fh' % r['elapsed_h'] if r['elapsed_h'] is not None else 'NA', str(r['status'])[:55].ljust(55), r['file'], r['path'].split('work-engine/')[1].split('/')[0][:28])
