#!/usr/bin/env python3
"""Report each session's first-call context, to compare floors across a config change.

Usage: python3 -I floor.py [SINCE]      SINCE is an ISO-8601 prefix, default 2026-10-14T23:00

A session's first `token_count` carries the standing context the provider
assembled before any work: system prompt, developer instructions, project doc
and every enabled tool's schema. That is the number a configuration trim moves.
Per-call usage is `info.last_token_usage`; `info.total_token_usage` is
account-lifetime cumulative. A tool call's body is in `input`, not `arguments`,
for custom_tool_call entries and the reverse for function_call.
"""
import json
import os
import pathlib
import statistics
import sys

SINCE = sys.argv[1] if len(sys.argv) > 1 else "2026-10-14T23:00"
# Frozen pre-change baseline: first-call context for the 98 sessions started
# 2026-10-04 to 10-14, all before the 2026-10-14T22:57Z config trim. Compare
# session start against session start; the 24-28K figures in
# ../context-replacement-20261014 are post-replacement windows, which also carry
# retained context and re-sent messages and are not the same quantity.
BASELINE = {"n": 98, "min": 20, "p25": 21, "median": 21, "p75": 25}

def first_call(path: pathlib.Path):
    meta = None
    for number, line in enumerate(path.open(errors="ignore")):
        if number == 0:
            try: meta = json.loads(line).get("payload") or {}
            except json.JSONDecodeError: return None
            if (meta.get("timestamp") or "") < SINCE: return None
            continue
        if '"token_count"' not in line:
            continue
        try: record = json.loads(line)
        except json.JSONDecodeError: continue
        usage = (((record.get("payload") or {}).get("info") or {}).get("last_token_usage")) or {}
        if usage.get("input_tokens"):
            return {
                "id": (meta.get("id") or "")[:8],
                "agent_path": meta.get("agent_path") or "root",
                "started": meta.get("timestamp"),
                "first_call_input_tokens": usage["input_tokens"],
                "cached": usage.get("cached_input_tokens", 0),
            }
    return None

rows = []
for root in (pathlib.Path.home() / ".codex/sessions",):
    for path in sorted(root.rglob("*.jsonl")):
        observed = first_call(path)
        if observed:
            rows.append(observed)

if not rows:
    print("no sessions started at or after %s yet" % SINCE)
    raise SystemExit(0)

rows.sort(key=lambda item: item["first_call_input_tokens"])
print("sessions started at or after %s: %d" % (SINCE, len(rows)))
for item in rows:
    print("   %6.0fK  cached %5.0fK  %-34s %s" % (
        item["first_call_input_tokens"] / 1000, item["cached"] / 1000,
        item["agent_path"][:34], item["started"][:19]))
values = [item["first_call_input_tokens"] for item in rows]
print()
print("after : min %.0fK  median %.0fK  max %.0fK" % (
    min(values) / 1000, statistics.median(values) / 1000, max(values) / 1000))
print("before: min %dK  median %dK  p75 %dK   (frozen, n=%d, pre-trim)" % (
    BASELINE["min"], BASELINE["median"], BASELINE["p75"], BASELINE["n"]))
removed = BASELINE["min"] * 1000 - min(values)
if removed > 0:
    print()
    print("standing context removed at the floor: %.0fK tokens" % (removed / 1000))
    print("  per call after the first in a window: %.4f Plus-units" % (removed / 10 * 7.826e-6))
    print("  once per context window, uncached:    %.4f Plus-units" % (removed * 7.826e-6))
