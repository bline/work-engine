#!/bin/bash
# Coordinator preflight (run immediately before launching a root): list every open scope_budget record within depth 3 of
# ~/cbm-work plus every record named by a Pi admission journal, into isolation/open-records.txt (read by isolation_check.py).
W=/home/bline/cbm-work/pi-accounting-sprint-20261113/reviews/design-r2
timeout 120 find /home/bline/cbm-work -maxdepth 3 -name '*.json' -size -2M -mtime -14 -not -path '*/node_modules/*' -not -path '*/work-engine/*' -not -path '*/r2/*' 2>/dev/null \
  | xargs -r grep -l '"scope_budget"' 2>/dev/null > $W/isolation/scope-budget-files.txt
python3 -I - <<'PY' > $W/isolation/open-records.txt
import json, os, glob
W="/home/bline/cbm-work/pi-accounting-sprint-20261113/reviews/design-r2"
cands=set(l.strip() for l in open(f'{W}/isolation/scope-budget-files.txt') if l.strip())
for j in glob.glob(os.path.expanduser('~/.local/state/work-engine/pi-objective-admissions/*.admissions.jsonl')):
    for l in open(j):
        try: r=json.loads(l)
        except ValueError: continue
        if r.get('objective_record'): cands.add(r['objective_record'])
for p in sorted(cands):
    try: sb=json.load(open(p)).get('scope_budget')
    except Exception: continue
    if isinstance(sb,dict) and sb.get('objective_id') and not sb.get('closed_at'):
        print(p, sb.get('objective_id'), sb.get('started_at'))
PY
wc -l < $W/isolation/open-records.txt
