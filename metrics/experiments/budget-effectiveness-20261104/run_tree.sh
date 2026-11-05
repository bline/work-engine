#!/bin/bash
# args: objective_id record since until
oid="$1"; rec="$2"; since="$3"; until="$4"
out=~/cbm-work/budget-analysis-20261104/trees/"${oid//[:\/]/_}".json
[ -s "$out" ] && exit 0
python3 -I /home/bline/code/work-engine/metrics/scripts/usage_ledger.py tree --record "$rec" --since "$since" --until "$until" --out "$out" > /dev/null 2> "$out.err" || echo "FAIL $oid" >&2
