#!/bin/bash
# P1.34 (adapted from P1.32, same steps): run by a fresh single-purpose isolated root (its only permitted command; T-2: never a second turn).
# isolation_check.py --phase oneshot must pass, then exactly one Pi subscription session is launched through the
# installed release 92a03789 with API keys and upstream overrides removed: one tool-less request (--max-requests 1),
# cost-proxy stop 3.00, thinking high (Module A), the frozen one-shot prompt on stdin, no mounts besides an empty working directory.
set -uo pipefail
W=/home/bline/cbm-work/p134-module-a
REL=/home/bline/.local/share/work-engine/skill-releases/releases/92a03789327777b3cd9b5822c6df75a18fa63660/skills/manual-orchestration/scripts
CLEAN=(env -u OPENAI_API_KEY -u ANTHROPIC_API_KEY -u OPENAI_CODEX_API_KEY -u ROLE_HOST_PI_UPSTREAM_CODEX -u ROLE_HOST_PI_UPSTREAM \
       -u ROLE_HOST_PI_UPSTREAM_ANTHROPIC -u PYTHONPATH -u PYTHONHOME -u PYTHONSTARTUP -u ROLE_HOST_PI_RUNTIME -u ROLE_HOST_DOCKER_EXE -u ROLE_HOST_USAGE_LEDGER)
OUT=$W/review
[ ! -e $OUT/launch.json ] || { echo "refused: a review launch was already attempted"; exit 4; }
ME=$("${CLEAN[@]}" python3 -I $REL/role_host.py whoami) || { echo "refused: whoami failed: $ME"; exit 3; }
"${CLEAN[@]}" python3 -I $W/root-actions/isolation_check.py --phase oneshot --out $W/isolation/root-proof-oneshot.json --whoami "$ME" || exit 5
SID=$(python3 -I -c 'import json,sys; print(json.loads(sys.argv[1])["session_id"])' "$ME")
cd /home/bline/code/work-engine
"${CLEAN[@]}" python3 -I $REL/role_host.py launch pi --container --cwd $W/run/review-work \
  --name p134-module-a-review --prompt-file $W/packet/prompt-oneshot.md --parent "$SID" --auth subscription \
  --max-requests 1 --max-usd 3.00 \
  --host-arg=--model --host-arg=openai-codex/gpt-6-astra --host-arg=--thinking --host-arg=high \
  --host-arg=--exclude-tools --host-arg='*' --host-arg=--no-context-files --host-arg=--no-mcp --host-arg=--no-skills \
  > $OUT/launch.json 2> $OUT/launch.stderr
rc=$?; echo "launch exit=$rc"; head -c 600 $OUT/launch.json; echo; tail -n 3 $OUT/launch.stderr; exit $rc
