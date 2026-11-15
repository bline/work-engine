#!/bin/bash
# Revision6 architecture realization one-shot Astra xhigh review; owner pre-registration and pinned packet required.
set -uo pipefail
W=/home/bline/cbm-work/pi-accounting-sprint-20261113/reviews/design-r2
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
  --name p137-architecture-realization --prompt-file /home/bline/cbm-work/pi-accounting-sprint-20261113/reviews/design-r2/packet/prompt-oneshot.md --parent "$SID" --auth subscription \
  --max-requests 1 --max-usd 4.00 \
  --host-arg=--model --host-arg=openai-codex/gpt-6-astra --host-arg=--thinking --host-arg=xhigh \
  --host-arg=--exclude-tools --host-arg='*' --host-arg=--no-context-files --host-arg=--no-mcp --host-arg=--no-skills \
  > $OUT/launch.json 2> $OUT/launch.stderr
rc=$?; echo "launch exit=$rc"; head -c 600 $OUT/launch.json; echo; tail -n 3 $OUT/launch.stderr; exit $rc
