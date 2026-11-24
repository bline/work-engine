#!/usr/bin/env python3
"""Stage 0 evidence capture for the perspective and intent research (P7 first).

Copies pinned source artifacts into this directory, verifies each against its
expected SHA-256, exports cited board messages, records session-transcript
inventories, and writes a sanitized excerpt of the cited session entries.

Transcripts themselves are NOT copied: they are large and contain private
reasoning and prompts. Each excerpt entry carries the source line number and
the SHA-256 of the raw source line, so it can be checked against the original
without storing the original.

Run from the repository root:  python3 -I experiments/perspective-intent-research/stage0/capture.py
No network, no model calls. Fails without writing a manifest if any pinned
digest does not match.
"""

import datetime
import hashlib
import json
import os
import shutil
import subprocess
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
HOME = os.path.expanduser("~")
U4A = os.path.join(HOME, "cbm-work/pi-host-u4a-acceptance-20261121")
SESSIONS = os.path.join(HOME, ".pi/agent/sessions/--home-bline-code-work-engine--")
CHECKIN = os.path.join(HOME, "cbm-work/milestone-a-coordinator-20261121/CHECKIN.md")

# Case F packet: relative path -> (expected SHA-256, where it was pinned before capture).
# 041 and the two escalation briefs are added because the independent plan
# reviews cited them. A None pin source means first pinned at this capture.
CASE_F_PACKET = {
    "review-r1/043-u4a-held-contract.md": "4bf5ee603ce9854febf29d22e063825cc7e9d795049ca52d965b7939cf7dcd04",
    "review-r1/041-u4-protection-qualification-PLAN.md": "c1c6090c136ce75504f63bb0ec4a2c8ab60fb7cd9b25e7b8ea9edb54734fd23f",
    "PLAN.md": "c7511ad79ba57e3de4534576e2fd87ba15abcc9980569aadbc3d0eb852e9b9fd",
    "ARCHITECTURE-REESCALATION.md": "d8125989e607696c09e584a5c115dd3ec44621c3a6a9fd0bbca271c9e5988c3b",
    "architecture-reescalation-review/ASTRA-ARCHITECTURE.md": "12127e463a3064b0292eb3515681da0bb7699f15660113c352301c32bb2e19d5",
    "QUALIFICATION-AMENDMENT.md": "93e004dc8b18c900f5495c0926dd354d5abcd1265143e08bd85add399aa8545e",
    "RESOLVED-OBJECT-ESCALATION.md": "45994d705b5c2aad795ae9f62c5237c66d0d2e103f415e6c9bee8900f94dec1f",
    "resolved-object-review/ASTRA-ARCHITECTURE.md": "e8304cda2ec086702b29a59afc7acea13f84098d1852f043d6f56c7d518d0824",
    "resolved-object-review/OPERATOR-DECISION.md": "d0055826f82250d4e93112d5ba262fab3da588cd870873cc9d705ff350154df9",
    "DOCKER-AUTOREMOVE-PROPOSAL.md": "f3598d2ef1f85e088afea32633a507bc2e28cb785c2b28bce1a205c2122ecf28",
    "DOCKER-AUTOREMOVE-SCOPE-DECISION.md": "b0b2b8fda72d1cf3c4469102c0b4a3b669a3c4eb5ea263dff2e28af4b954c145",
}

PRIOR_PINS = {
    "review-r1/043-u4a-held-contract.md": "pilot section 3; plan reviews",
    "review-r1/041-u4-protection-qualification-PLAN.md": "plan re-review (prefix c1c6090c)",
    "PLAN.md": "pilot section 3; plan reviews",
    "architecture-reescalation-review/ASTRA-ARCHITECTURE.md": "pilot section 3; plan reviews",
    "QUALIFICATION-AMENDMENT.md": "pilot section 3; plan reviews",
    "RESOLVED-OBJECT-ESCALATION.md": "cited inside OPERATOR-DECISION.md",
    "resolved-object-review/ASTRA-ARCHITECTURE.md": "pilot section 3; plan reviews",
    "resolved-object-review/OPERATOR-DECISION.md": "pilot section 3; plan reviews",
    "DOCKER-AUTOREMOVE-PROPOSAL.md": "pilot section 3; plan reviews",
    "DOCKER-AUTOREMOVE-SCOPE-DECISION.md": "pilot section 3; plan reviews",
}

BOARD_MESSAGES = [6173, 6176, 6184, 6185]

U4A_SESSION = "2026-11-16T04-37-03-444Z_01a1e2a4-f353-743a-ba23-f42394958125.jsonl"
MILESTONE_A_SESSION = "2026-11-21T17-27-16-873Z_01a1ff25-e889-770e-9973-f060fc484249.jsonl"

# Cited Case F session entries: (start, end) ISO timestamp windows, inclusive.
U4A_WINDOWS = [
    ("2026-11-21T08:11:00", "2026-11-21T08:13:59"),  # overnight delegation and acknowledgement
    ("2026-11-21T08:59:00", "2026-11-21T08:59:59"),  # repair writer dispatch
    ("2026-11-21T09:43:00", "2026-11-21T09:43:59"),  # parked message to operator ("unchanged")
    ("2026-11-21T15:07:00", "2026-11-21T15:38:00"),  # operator turn and correction
]

CASE_D_SOURCE = ("3693e48c52eeb5cd02e39b391bb773dfd0a65545",
                 "rust/crates/work-engine-manual-host/src/pi_public_hold.rs")


def sha256_file(path):
    h = hashlib.sha256()
    with open(path, "rb") as f:
        for chunk in iter(lambda: f.read(1 << 20), b""):
            h.update(chunk)
    return h.hexdigest()


def fail(message):
    sys.stderr.write(f"capture failed: {message}\n")
    sys.exit(21)


def capture_case_f(manifest):
    out = os.path.join(HERE, "case-f", "packet")
    entries = []
    for rel, expected in CASE_F_PACKET.items():
        src = os.path.join(U4A, rel)
        if not os.path.isfile(src):
            fail(f"missing {src}")
        digest = sha256_file(src)
        if expected and not digest.startswith(expected):
            fail(f"{rel}: expected {expected}, got {digest}")
        dst = os.path.join(out, rel)
        os.makedirs(os.path.dirname(dst), exist_ok=True)
        shutil.copyfile(src, dst)
        if sha256_file(dst) != digest:
            fail(f"{rel}: copy digest differs")
        entries.append({"path": rel, "source": src, "sha256": digest,
                        "bytes": os.path.getsize(src),
                        "prior_pin": PRIOR_PINS.get(rel, "first pinned at this capture")})
    manifest["case_f_packet"] = entries


def export_board(manifest):
    repo = os.path.abspath(os.path.join(HERE, "../../.."))
    tool = os.path.join(repo, "skills/durable-state/scripts/codex_chatboard.py")
    messages = []
    for seq in BOARD_MESSAGES:
        out = subprocess.run([sys.executable, "-I", tool, "--repository", repo, "read",
                              "--since", str(seq - 1), "--limit", "1"],
                             capture_output=True, text=True, check=True).stdout
        msg = json.loads(out)["messages"][0]
        if msg["sequence"] != seq:
            fail(f"board message {seq} not found (got {msg['sequence']})")
        messages.append(msg)
    path = os.path.join(HERE, "case-f", "board-messages.json")
    with open(path, "w") as f:
        json.dump(messages, f, indent=2, ensure_ascii=False)
        f.write("\n")
    manifest["case_f_board_messages"] = {"path": "case-f/board-messages.json",
                                         "sequences": BOARD_MESSAGES,
                                         "sha256": sha256_file(path)}


def project(entry):
    """Keep timestamps, role, visible text and tool-call names/arguments; drop thinking."""
    message = entry.get("message") or {}
    content = message.get("content")
    texts, calls = [], []
    if isinstance(content, str):
        texts.append(content)
    elif isinstance(content, list):
        for block in content:
            if not isinstance(block, dict):
                continue
            if block.get("type") == "text":
                texts.append(block.get("text", ""))
            elif block.get("type") == "toolCall":
                calls.append({"name": block.get("name"), "arguments": block.get("arguments")})
    return {"timestamp": entry.get("timestamp"), "role": message.get("role"),
            "text": "\n".join(texts), "tool_calls": calls}


def capture_u4a_excerpt(manifest):
    src = os.path.join(SESSIONS, U4A_SESSION)
    excerpt = []
    with open(src, "rb") as f:
        for number, raw in enumerate(f, 1):
            try:
                entry = json.loads(raw)
            except ValueError:
                continue
            ts = entry.get("timestamp") or ""
            if entry.get("type") != "message":
                continue
            if not any(lo <= ts[:19] <= hi for lo, hi in U4A_WINDOWS):
                continue
            item = project(entry)
            if item["role"] not in ("user", "assistant"):
                continue
            if not item["text"].strip() and not item["tool_calls"]:
                continue
            item["source_line"] = number
            item["raw_line_sha256"] = hashlib.sha256(raw).hexdigest()
            excerpt.append(item)
    path = os.path.join(HERE, "case-f", "session-excerpt.json")
    with open(path, "w") as f:
        json.dump({"session_file": U4A_SESSION,
                   "windows_utc": U4A_WINDOWS,
                   "projection": "user/assistant messages; text blocks and tool-call name/arguments only; thinking omitted",
                   "entries": excerpt}, f, indent=2, ensure_ascii=False)
        f.write("\n")
    manifest["case_f_session_excerpt"] = {"path": "case-f/session-excerpt.json",
                                          "entries": len(excerpt), "sha256": sha256_file(path)}


def inventory_sessions(manifest):
    inv = []
    for name, live in ((U4A_SESSION, False), (MILESTONE_A_SESSION, True)):
        path = os.path.join(SESSIONS, name)
        with open(path, "rb") as f:
            data = f.read()
        lines = data.count(b"\n")
        record = {"file": path, "bytes": len(data), "lines": lines,
                  "modified_utc": datetime.datetime.fromtimestamp(
                      os.path.getmtime(path), datetime.timezone.utc).isoformat(timespec="seconds")}
        if live:
            record["live"] = True
            record["prefix_lines"] = lines
            record["prefix_sha256"] = hashlib.sha256(data).hexdigest()
            record["note"] = "Still being written; the prefix digest pins its first prefix_lines lines at capture."
        else:
            record["sha256"] = hashlib.sha256(data).hexdigest()
        inv.append(record)
    manifest["session_inventory"] = inv


def snapshot_checkin(manifest):
    out = os.path.join(HERE, "milestone-a")
    os.makedirs(out, exist_ok=True)
    dst = os.path.join(out, "CHECKIN-snapshot.md")
    shutil.copyfile(CHECKIN, dst)
    digest = sha256_file(dst)
    if digest != sha256_file(CHECKIN):
        fail("CHECKIN changed during copy; re-run")
    manifest["milestone_a_checkin"] = {
        "path": "milestone-a/CHECKIN-snapshot.md", "source": CHECKIN, "sha256": digest,
        "bytes": os.path.getsize(dst),
        "note": "Living newest-first log; this is a point-in-time snapshot."}


def record_case_d(manifest):
    repo = os.path.abspath(os.path.join(HERE, "../../.."))
    commit, path = CASE_D_SOURCE
    blob = subprocess.run(["git", "-C", repo, "rev-parse", f"{commit}:{path}"],
                          capture_output=True, text=True, check=True).stdout.strip()
    manifest["case_d_source"] = {"commit": commit, "path": path, "git_blob": blob,
                                 "note": "In-repository source; not copied."}


def main():
    manifest = {"captured_at_utc": datetime.datetime.now(datetime.timezone.utc).isoformat(timespec="seconds"),
                "script": "stage0/capture.py"}
    capture_case_f(manifest)
    export_board(manifest)
    capture_u4a_excerpt(manifest)
    inventory_sessions(manifest)
    snapshot_checkin(manifest)
    record_case_d(manifest)
    with open(os.path.join(HERE, "MANIFEST.json"), "w") as f:
        json.dump(manifest, f, indent=2, ensure_ascii=False)
        f.write("\n")
    print(json.dumps({k: (v if not isinstance(v, list) else len(v)) for k, v in manifest.items()
                      if k in ("captured_at_utc",)} | {"ok": True}))


if __name__ == "__main__":
    main()
