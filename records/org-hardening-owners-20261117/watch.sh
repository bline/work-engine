#!/bin/bash
# usage: watch.sh <sid> <cap_usd> <timeout_s> [objective_stop_usd]
# Returns when the child is idle, its cap is hit (stop verified), the objective threshold is hit, or a root wait event.
# A stop counts only when `claude agents` confirms the session is no longer working; otherwise STOP-UNCONFIRMED.
N5=$(dirname "$(readlink -f "$0")"); sid=$1; cap=$2; to=$3; ostop=${4:-999}; t0=$(date +%s)
W=/home/bline/.local/state/work-engine/org-root-pi-01a1e56e/context-wait/wait.jsonl; base=$(wc -l < $W)
st(){ claude agents --json --all 2>/dev/null | python3 -c "import json,sys; d=json.load(sys.stdin); r=[r for r in (d if isinstance(d,list) else d.get('agents',[])) if r.get('sessionId')=='$sid']; print((str(r[0].get('status',''))+'/'+str(r[0].get('state',''))) if r else 'absent')"; }
stop_verified(){
  for i in 1 2 3; do
    claude stop "${sid:0:8}" >/dev/null 2>&1
    sleep 5; s=$(st)
    if [[ "$s" != */working && "$s" != busy/* ]]; then echo "STOP-VERIFIED attempt=$i state=$s"; return 0; fi
  done
  echo "STOP-UNCONFIRMED state=$s"; return 1
}
while :; do
  out=$(python3 $N5/cost.py 2>/dev/null | python3 -c "import json,sys;d=json.load(sys.stdin);c=d['children'].get('$sid');print((c or {}).get('usd',0) if isinstance(c,dict) else 0, d['total'])" 2>/dev/null)
  read c tot <<< "$out"
  if [[ -n "$c" && -n "$tot" ]] && python3 -c "import sys;sys.exit(0 if float('$c')>=float('$cap') or float('$tot')>=float('$ostop') else 1)"; then
    echo "CAP child=$c total=$tot"; stop_verified; break
  fi
  s=$(st); if [[ "$s" != */working ]]; then sleep 20; s=$(st); if [[ "$s" != */working ]]; then echo "DONE state=$s child=$c total=$tot elapsed=$(( $(date +%s)-t0 ))"; break; fi; fi
  n=$(wc -l < $W); if (( n > base )); then echo "ROOT-WAIT-EVENT lines=$n"; tail -1 $W | cut -c1-400; break; fi
  (( $(date +%s)-t0 > to )) && { echo "TIMEOUT state=$s child=$c total=$tot"; break; }
  sleep 30
done
