#!/usr/bin/env python3
"""Recover a Claude Code session's cost in the pure-Codex baseline's unit.

Weighted tokens = uncached + cached/10 + 5*output, with Anthropic
`cache_creation_input_tokens` counted as uncached and `cache_read_input_tokens`
as cached. Those price ratios are the baseline's assumptions, carried over.

An orchestrating session's subagents write their own transcripts under
<session-id>/subagents/, and their usage does NOT appear in the main
transcript - no sidechain records are written to it. Reading the main
transcript alone therefore undercounts an orchestrated run. This script sums
both, and reports the split so the omission stays visible.

Usage: claude_session_cost.py <transcript.jsonl|session-dir> [since-iso] [until-iso]
"""
import json, os, sys, glob
from datetime import datetime, timezone
from collections import Counter


def parse_time(value):
    return datetime.fromisoformat(value.replace("Z", "+00:00")).astimezone(timezone.utc)


def calls(path):
    """Distinct assistant calls carrying usage, deduplicated by message id."""
    seen, rows = set(), []
    with open(path, errors="ignore") as handle:
        for line in handle:
            if '"usage"' not in line:
                continue
            try:
                record = json.loads(line)
            except ValueError:
                continue
            message = record.get("message") or {}
            usage = message.get("usage") if isinstance(message, dict) else None
            timestamp = record.get("timestamp")
            if not isinstance(usage, dict) or not timestamp:
                continue
            key = (message.get("id"), timestamp)
            if key in seen:
                continue
            seen.add(key)
            rows.append(
                {
                    "at": parse_time(timestamp),
                    "model": message.get("model") or "unknown",
                    "uncached": (usage.get("input_tokens") or 0)
                    + (usage.get("cache_creation_input_tokens") or 0),
                    "cached": usage.get("cache_read_input_tokens") or 0,
                    "output": usage.get("output_tokens") or 0,
                }
            )
    rows.sort(key=lambda row: row["at"])
    return rows


def weighted(row):
    return row["uncached"] + row["cached"] / 10.0 + 5 * row["output"]


def components(target):
    if target.endswith(".jsonl"):
        base = target[: -len(".jsonl")]
        main = target
    else:
        base = target.rstrip("/")
        main = base + ".jsonl"
    found = []
    if os.path.exists(main):
        found.append((main, "orchestrator"))
    for path in sorted(glob.glob(os.path.join(base, "subagents", "*.jsonl"))):
        found.append((path, "subagent " + os.path.basename(path)[6:21]))
    return found


def main():
    target = sys.argv[1]
    since = parse_time(sys.argv[2]) if len(sys.argv) > 2 and sys.argv[2] != "-" else None
    until = parse_time(sys.argv[3]) if len(sys.argv) > 3 and sys.argv[3] != "-" else None
    report = {"target": target, "since": sys.argv[2] if since else None,
              "until": sys.argv[3] if until else None, "components": []}
    everything, orchestrator_weighted = [], 0.0
    for path, label in components(target):
        rows = [
            row
            for row in calls(path)
            if (since is None or row["at"] > since) and (until is None or row["at"] <= until)
        ]
        if not rows:
            continue
        total = sum(weighted(row) for row in rows)
        if label == "orchestrator":
            orchestrator_weighted = total
        everything += rows
        report["components"].append(
            {
                "component": label,
                "calls": len(rows),
                "weighted": total,
                "output": sum(row["output"] for row in rows),
                "models": sorted({row["model"] for row in rows}),
                "first": rows[0]["at"].isoformat(),
                "last": rows[-1]["at"].isoformat(),
            }
        )
    grand = sum(weighted(row) for row in everything)
    by_model = Counter()
    for row in everything:
        by_model[row["model"]] += weighted(row)
    report["total"] = {
        "calls": len(everything),
        "weighted": grand,
        "output": sum(row["output"] for row in everything),
        "weighted_by_model": dict(by_model),
        "orchestrator_share": (orchestrator_weighted / grand) if grand else None,
    }
    json.dump(report, sys.stdout, indent=1)
    print()


if __name__ == "__main__":
    main()
