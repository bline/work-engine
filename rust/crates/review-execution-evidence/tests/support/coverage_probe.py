#!/usr/bin/python3
"""No-provider RP0 mount probe; availability is not reviewer inspection."""

import hashlib
import json
import subprocess
from pathlib import Path


def same_source(record):
    source = Path("/repo/candidate") / record["path"]
    actual = hashlib.sha256(source.read_bytes()).hexdigest()
    assert actual == record["source_sha256"]
    return {"path": record["path"], "sha256": actual}


ledger_path = Path("/cbm/coverage.json")
ledger_raw = ledger_path.read_bytes()
ledger = json.loads(ledger_raw)
assert ledger["schema_version"] == 2
assert ledger["fallback_accounted"] is True
fallback = ledger["fallback"]
assert fallback
fallback_read = same_source(fallback[0])
partial = next((entry for entry in fallback if entry["status"] == "partial"), None)
partial_read = same_source(partial) if partial else None
opaque = ledger["opaque_files"]
opaque_read = same_source(opaque[0]) if opaque else None

write_denied = []
for target in [ledger_path, Path("/repo/candidate") / fallback[0]["path"]]:
    try:
        with target.open("ab") as handle:
            handle.write(b"forbidden")
    except OSError:
        write_denied.append(str(target))
assert len(write_denied) == 2
assert not Path("/home/bline/code/work-engine/AGENTS.md").exists()

# Exercise the existing relay once so the real RP0 hostile-probe harness can
# close cleanly. The coverage/source checks above are plain granted reads.
child = subprocess.Popen(
    ["/cbm/client"], stdin=subprocess.PIPE, stdout=subprocess.PIPE,
    stderr=subprocess.PIPE, text=True, bufsize=1, close_fds=True,
)
child.stdin.write(json.dumps({"jsonrpc": "2.0", "id": 1, "method": "initialize",
    "params": {"protocolVersion": "2024-11-05", "capabilities": {},
               "clientInfo": {"name": "rp0-coverage-probe", "version": "1"}}}) + "\n")
child.stdin.flush()
assert "result" in json.loads(child.stdout.readline())
child.stdin.write(json.dumps({"jsonrpc": "2.0", "method": "notifications/initialized"}) + "\n")
child.stdin.flush()
child.stdin.write(json.dumps({"jsonrpc": "2.0", "id": 2, "method": "tools/call",
    "params": {"name": "index_status", "arguments": {"project": ledger["project"],
        "diagnostics": "none", "format": "json"}}}) + "\n")
child.stdin.flush()
assert "result" in json.loads(child.stdout.readline())
child.stdin.close()
child.wait(timeout=10)

print(json.dumps({
    "ledger_sha256": hashlib.sha256(ledger_raw).hexdigest(),
    "fallback_read": fallback_read,
    "partial_read": partial_read,
    "opaque_read": opaque_read,
    "write_denied": write_denied,
    "host_source_hidden": True,
    "provider_calls": 0,
}, sort_keys=True))
