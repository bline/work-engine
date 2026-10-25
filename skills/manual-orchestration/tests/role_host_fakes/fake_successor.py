#!/usr/bin/env python3
"""A scripted successor that follows the gate's successor prompt literally.

It holds no capability a real successor lacks: it learns its id from the host
environment through `role_host.py whoami`, collects the handoff with the board
`wait` the prompt names, and acknowledges with the `adopt` command the prompt
names. Behaviour (FAKE_SUCCESSOR):
  adopt        do the steps
  silent       end the turn without adopting
  wrong_topic  adopt, then rewrite that row onto another topic, addressed `to`
               the successor (the S2<->S5 fixture shape real adopt cannot write)
  wrong_pred   adopt naming a different predecessor
"""
import json, os, re, shlex, subprocess, sys
from pathlib import Path

prompt = Path(sys.argv[1]).read_text()
mode = os.environ.get("FAKE_SUCCESSOR", "adopt")
if mode == "silent":
    sys.exit(0)

def line_after(marker):
    for line in prompt.splitlines():
        if marker in line:
            return shlex.split(line.strip())
    raise SystemExit(f"prompt has no line with {marker!r}")

who = subprocess.run(line_after("whoami"), capture_output=True, text=True, check=True)
sid = json.loads(who.stdout)["session_id"]
wait = [w.replace("session:SID", f"session:{sid}") for w in line_after(" wait --since ")]
subprocess.run(wait, capture_output=True, text=True, check=True)
adopt = [w.replace("SID", sid) for w in line_after(" adopt ")]
if mode == "wrong_pred":
    i = adopt.index("--predecessor-session")
    adopt[i + 1] = "00000000-0000-4000-8000-000000000000"
if mode != "wrong_topic":
    r = subprocess.run(adopt, capture_output=True, text=True)
    if r.returncode != 0:
        sys.exit(f"adopt failed: {r.stderr}")
    sys.exit(0)

# wrong_topic: no valid adoption may exist even briefly, or the gate would be
# right to accept it. Real `adopt` writes a genuine row on a SCRATCH board; that
# row is moved to another topic, addressed `to` the successor, and appended to
# the real board directly.
import tempfile
repo = adopt[adopt.index("--repository") + 1]
scratch = tempfile.mkdtemp(prefix="scratch-board-")
subprocess.run(["git", "init", "-q", scratch], check=True)
adopt[adopt.index("--repository") + 1] = scratch
r = subprocess.run(adopt, capture_output=True, text=True)
if r.returncode != 0:
    sys.exit(f"scratch adopt failed: {r.stderr}")
sys.path.insert(0, str(Path(adopt[1]).resolve().parent))
import codex_chatboard as BOARD
from durable_state import GitRefDurableStateStore
row = json.loads(r.stdout)["message"]
store = GitRefDurableStateStore(Path(repo), BOARD.NAMESPACE)
cur = store.read(BOARD.BOARD_KEY)
board = BOARD.decode(cur.payload)
row.update(topic="aspect:handoff", to=[f"session:{sid}"], sequence=board["next_sequence"])
board["messages"].append(row)
board["next_sequence"] += 1
store.publish(BOARD.BOARD_KEY, BOARD.encode(board), cur.revision)
