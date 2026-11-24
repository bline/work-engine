#!/usr/bin/env python3
"""Stage 0 evidence capture for Case B (P7 transfer test).

Writes only stage0/case-b/ and stage0/MANIFEST-case-b.json, so it does not
disturb the earlier capture (whose CHECKIN snapshot and live-session prefix
digest are point-in-time pins). Same rules as capture.py: byte-identical
copies verified by digest, whole board messages, and a sanitized session
excerpt whose entries carry the 1-based source line and raw-line SHA-256.

Run from the repository root:
  python3 -I experiments/perspective-intent-research/stage0/capture_case_b.py
"""

import datetime
import hashlib
import json
import os
import shutil
import subprocess
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from capture import fail, project, sha256_file  # noqa: E402

HERE = os.path.dirname(os.path.abspath(__file__))
HOME = os.path.expanduser("~")
MA = os.path.join(HOME, "cbm-work/milestone-a-coordinator-20261121")
SESSIONS = os.path.join(HOME, ".pi/agent/sessions/--home-bline-code-work-engine--")
SESSION = "2026-11-21T17-27-16-873Z_01a1ff25-e889-770e-9973-f060fc484249.jsonl"

# relative path -> (expected SHA-256 or None, prior pin)
PACKET = {
    "ORG-ASTRA-DESIGN-REVIEW.md": (None, "first pinned at this capture (pre-intervention review brief)"),
    "ORG-ASTRA-DESIGN-REVIEW-RESULT.md": (None, "first pinned at this capture (pre-intervention review)"),
    "CONTROLLED-BUILDER-ADMISSION-REVISION.md": (
        "2343a8f888bfa80219660532e0d8e573edb37392102e912a3148a3f4c7dcd491",
        "board #6550 and CHECKIN snapshot line 269"),
}

BOARD_MESSAGES = [6549, 6550]

# The operator turn and its immediate context, UTC.
WINDOWS = [("2026-11-22T15:29:00", "2026-11-22T15:51:00")]


def main():
    manifest = {"captured_at_utc": datetime.datetime.now(datetime.timezone.utc).isoformat(timespec="seconds"),
                "script": "stage0/capture_case_b.py"}
    out = os.path.join(HERE, "case-b", "packet")
    os.makedirs(out, exist_ok=True)
    entries = []
    for rel, (expected, prior) in PACKET.items():
        src = os.path.join(MA, rel)
        if not os.path.isfile(src):
            fail(f"missing {src}")
        digest = sha256_file(src)
        if expected and digest != expected:
            fail(f"{rel}: expected {expected}, got {digest}")
        dst = os.path.join(out, rel)
        shutil.copyfile(src, dst)
        if sha256_file(dst) != digest:
            fail(f"{rel}: copy digest differs")
        entries.append({"path": rel, "source": src, "sha256": digest,
                        "bytes": os.path.getsize(src), "prior_pin": prior,
                        "modified_utc": datetime.datetime.fromtimestamp(
                            os.path.getmtime(src), datetime.timezone.utc).isoformat(timespec="seconds")})
    manifest["case_b_packet"] = entries

    repo = os.path.abspath(os.path.join(HERE, "../../.."))
    tool = os.path.join(repo, "skills/durable-state/scripts/codex_chatboard.py")
    messages = []
    for seq in BOARD_MESSAGES:
        raw = subprocess.run([sys.executable, "-I", tool, "--repository", repo, "read",
                              "--since", str(seq - 1), "--limit", "1"],
                             capture_output=True, text=True, check=True).stdout
        msg = json.loads(raw)["messages"][0]
        if msg["sequence"] != seq:
            fail(f"board message {seq} not found")
        messages.append(msg)
    path = os.path.join(HERE, "case-b", "board-messages.json")
    with open(path, "w") as f:
        json.dump(messages, f, indent=2, ensure_ascii=False)
        f.write("\n")
    manifest["case_b_board_messages"] = {"path": "case-b/board-messages.json",
                                         "sequences": BOARD_MESSAGES, "sha256": sha256_file(path)}

    src = os.path.join(SESSIONS, SESSION)
    excerpt = []
    with open(src, "rb") as f:
        for number, raw in enumerate(f, 1):
            try:
                entry = json.loads(raw)
            except ValueError:
                continue
            if entry.get("type") != "message":
                continue
            ts = (entry.get("timestamp") or "")[:19]
            if ts > WINDOWS[-1][1]:
                break
            if not any(lo <= ts <= hi for lo, hi in WINDOWS):
                continue
            item = project(entry)
            if item["role"] not in ("user", "assistant") or not (item["text"].strip() or item["tool_calls"]):
                continue
            item["source_line"] = number
            item["raw_line_sha256"] = hashlib.sha256(raw).hexdigest()
            excerpt.append(item)
    path = os.path.join(HERE, "case-b", "session-excerpt.json")
    with open(path, "w") as f:
        json.dump({"session_file": SESSION, "windows_utc": WINDOWS,
                   "projection": "user/assistant messages; text blocks and tool-call name/arguments only; thinking omitted",
                   "verification": "source lines fall within the 13,087-line prefix pinned in MANIFEST.json session_inventory",
                   "entries": excerpt}, f, indent=2, ensure_ascii=False)
        f.write("\n")
    manifest["case_b_session_excerpt"] = {"path": "case-b/session-excerpt.json",
                                          "entries": len(excerpt), "sha256": sha256_file(path)}

    with open(os.path.join(HERE, "MANIFEST-case-b.json"), "w") as f:
        json.dump(manifest, f, indent=2, ensure_ascii=False)
        f.write("\n")
    print(json.dumps({"captured_at_utc": manifest["captured_at_utc"], "excerpt_entries": len(excerpt), "ok": True}))


if __name__ == "__main__":
    main()
