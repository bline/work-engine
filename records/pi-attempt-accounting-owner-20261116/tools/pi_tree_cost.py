#!/usr/bin/env python3
"""Sum Pi-recorded API-equivalent cost for one Pi root session file and every
child session under its sibling stem directory. Prints JSON. Calls are keyed by
responseId (falling back to file+line) so a response is counted once."""
import json, os, sys, glob
root = sys.argv[1]
stem = root[:-len('.jsonl')]
files = [root] + sorted(glob.glob(os.path.join(stem, '**', '*.jsonl'), recursive=True))
seen = {}; unpriced = 0; per_file = {}; models = {}
for f in files:
    sub = 0.0
    for i, line in enumerate(open(f)):
        try: r = json.loads(line)
        except Exception: continue
        m = r.get('message') or {}
        u = m.get('usage')
        if m.get('role') != 'assistant' or not u: continue
        key = m.get('responseId') or f'{f}:{i}'
        if key in seen: continue
        c = (u.get('cost') or {}).get('total')
        if c is None: unpriced += 1; c = 0.0
        seen[key] = c; sub += c
        models[m.get('model')] = models.get(m.get('model'), 0) + c
    per_file[f] = round(sub, 6)
total = sum(seen.values())
print(json.dumps({'root': root, 'files': len(files), 'calls': len(seen), 'unpriced_calls': unpriced,
                  'api_equivalent_usd': round(total, 6), 'by_model': {k: round(v, 6) for k, v in models.items()},
                  'per_file': per_file}, indent=1))
