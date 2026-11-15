"""Latest observable Codex subscription rate-limit reading: the newest `token_count` event carrying `rate_limits`
across all local Codex rollouts. Account-wide; other Codex sessions (e.g. the active R2 workstream) move it too, so
it is not a per-request quota cost. No network call. Usage: quota_snapshot.py <label> >> readings.jsonl"""
import glob, json, os, sys, datetime
best = None
for f in glob.glob(os.path.expanduser('~/.codex/sessions/2026/11/1[0-9]/*.jsonl')):
    try:
        if best and os.path.getmtime(f) < best['file_mtime']: continue
        for l in open(f):
            if '"rate_limits"' not in l: continue
            d = json.loads(l); p = d.get('payload') or {}
            if p.get('type') == 'token_count' and p.get('rate_limits') and (not best or d['timestamp'] > best['observed_at']):
                rl = p['rate_limits']
                best = dict(observed_at=d['timestamp'], source_session=os.path.basename(f)[-41:-6], file_mtime=os.path.getmtime(f),
                            primary=rl.get('primary'), secondary=rl.get('secondary'), plan_type=rl.get('plan_type'),
                            credits=rl.get('credits'), rate_limit_reached_type=rl.get('rate_limit_reached_type'))
    except (OSError, ValueError):
        continue
print(json.dumps(dict(label=sys.argv[1], taken_at=datetime.datetime.now(datetime.timezone.utc).isoformat(), latest=best,
                      basis='latest local Codex rollout rate_limits snapshot; account-wide, not attributable per request'), default=str))
