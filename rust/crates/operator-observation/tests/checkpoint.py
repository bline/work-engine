#!/usr/bin/env python3
"""Controlled actual-Events-compositor -> Rust collector checkpoint. No provider."""
import contextlib
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile

os.umask(0o077)
binary, repository, output = map(Path, sys.argv[1:4])
output.mkdir(parents=True, exist_ok=True)
if len(sys.argv) == 6:
    source = Path(sys.argv[4]).read_bytes()
    expected_pin = sys.argv[5]
elif len(sys.argv) == 4:
    source = subprocess.run(["git", "show", "d44b1bbf:skills/session-events/scripts/session_events.py"], cwd=repository, capture_output=True, check=True).stdout
    expected_pin = "933eda1c2af1f3f76819fbe2099269c6a04c33fcdb8adf52969cdf80f5beeefd"
else:
    raise SystemExit("expected binary repository output [selected-owner exact-sha256]")
assert hashlib.sha256(source).hexdigest() == expected_pin
with contextlib.nullcontext(tempfile.mkdtemp(prefix="operator-observation-", dir=output)) as directory:
    root = Path(directory)
    owner = root / "session_events.py"; owner.write_bytes(source)
    sessions = root / "codex" / "sessions" / "2026" / "11" / "13"; sessions.mkdir(parents=True)
    (root / "claude").mkdir()
    session_id = "11111111-1111-4111-8111-111111111111"
    missing_id = "22222222-2222-4222-8222-222222222222"
    transcript = sessions / f"rollout-2026-11-13T00-00-00-{session_id}.jsonl"
    def write_transcript(text):
        rows = [{"type": "session_meta", "payload": {"id": session_id, "cwd": str(root), "timestamp": "2026-11-13T00:00:00Z"}},
                {"type": "response_item", "timestamp": "2026-11-13T00:00:01Z", "payload": {"type": "message", "role": "assistant", "content": [{"type": "output_text", "text": text}]}}]
        transcript.write_text("".join(json.dumps(row) + "\n" for row in rows))
    write_transcript("Controlled source activity: observation one. token=sk-" + "a" * 48)
    profile = root / "profile.json"
    profile.write_text(json.dumps({"version": 1, "owner": str(owner), "owner_sha256": hashlib.sha256(source).hexdigest(), "codex_home": str(root / "codex"), "claude_home": str(root / "claude"), "state_dir": str(root / "events"), "bridge": None, "context": None}))
    profile.chmod(0o600)
    (root / "selected-profile.json").write_bytes(profile.read_bytes())
    target = root / "snapshot.json"
    command = [str(binary), str(profile), "--label", "Controlled interface run", "--session", session_id, "--session", missing_id, "--output", str(target)]
    subprocess.run(command, check=True)
    first = json.loads(target.read_bytes())
    (root / "first.json").write_bytes(target.read_bytes())
    assert first["sessions"][0]["observation"]["status"] == "available", first
    assert first["sessions"][1]["observation"]["status"] == "unavailable", first
    entries = first["sessions"][0]["observation"]["entries"]
    assert any("observation one" in entry["text"] for entry in entries), entries
    assert "sk-" + "a" * 48 not in target.read_text(), "owner redaction lost"
    assert target.stat().st_mode & 0o777 == 0o600
    write_transcript("Controlled source activity: observation two.")
    subprocess.run(command, check=True)
    second = json.loads(target.read_bytes())
    (root / "second.json").write_bytes(target.read_bytes())
    assert first["scope_id"] == second["scope_id"]
    assert any("observation two" in entry["text"] for entry in second["sessions"][0]["observation"]["entries"])
    owner.write_bytes(source + b"\n# changed source\n")
    subprocess.run(command, check=True)
    unavailable = json.loads(target.read_bytes())
    (root / "pin-mismatch.json").write_bytes(target.read_bytes())
    assert all(row["observation"]["status"] == "unavailable" for row in unavailable["sessions"])
    # Preserve the actual selected bytes as well as the deliberate mismatch.
    owner.write_bytes(source)
    artifact = output / "controlled-observation.json"
    artifact.write_text(json.dumps(second, indent=2) + "\n"); artifact.chmod(0o600)
    initial = output / "controlled-observation-first.json"
    initial.write_text(json.dumps(first, indent=2) + "\n"); initial.chmod(0o600)
    (output / "collector-checkpoint.json").write_text(json.dumps({"owner_sha256": hashlib.sha256(source).hexdigest(), "binary_sha256": hashlib.sha256(binary.read_bytes()).hexdigest(), "snapshot_sha256": hashlib.sha256(artifact.read_bytes()).hexdigest(), "observed": ["actual pinned compositor", "available and unavailable sessions", "source update", "owner redaction", "private atomic output", "pin mismatch unavailable"]}, indent=2) + "\n")
print("Controlled Events owner -> Rust collector checkpoint passed")
