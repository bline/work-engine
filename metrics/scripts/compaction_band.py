#!/usr/bin/env python3
"""Pre-compaction input readings for every thread under one or more roots.

Usage:
  python3 -I compaction_band.py --binary BIN ROOT_THREAD_ID [ROOT_THREAD_ID ...]

Enumerates descendants with `work-engine-context-status agents --recursive`,
adds each root's own rollout, and for every `compacted` record takes the last
`last_token_usage.input_tokens` observed before it in that rollout. Prints the
band per stratum (children, roots, all) with n, the windows seen, the
enumeration completeness the tool reported, and every event, so the
manual-orchestration skill's band floor can be restated as events accumulate.
An event with no prior usage reading is counted and excluded, not read as zero.
"""
from __future__ import annotations

import argparse
import glob
import hashlib
import json
import os
import statistics
import subprocess
import sys


def descendants(binary: str, root: str, sessions_dir: str) -> tuple[dict, str]:
    out = subprocess.run(
        [binary, "agents", "--parent-thread", root, "--recursive", "--sessions-dir", sessions_dir, "--json"],
        capture_output=True, text=True, check=True,
    ).stdout
    listing = json.loads(out)
    return {a["thread_id"]: (a["agent_path"], a["rollout"]) for a in listing["agents"]}, listing["completeness"]


def events(rollout: str) -> list[dict]:
    found, last, window, last_ts = [], None, None, None
    with open(rollout, encoding="utf-8") as handle:
        for line in handle:
            try:
                record = json.loads(line)
            except ValueError:
                break  # never read an in-progress trailing record
            payload = record.get("payload") or {}
            if record.get("type") == "event_msg" and payload.get("type") == "token_count":
                info = payload.get("info") or {}
                value = (info.get("last_token_usage") or {}).get("input_tokens")
                if value:
                    last, window, last_ts = value, info.get("model_context_window"), record.get("timestamp")
            if record.get("type") == "compacted":
                found.append({"at": record.get("timestamp"), "input_before": last,
                              "reading_at": last_ts, "window": window})
    return found


def summary(rows: list[dict]) -> dict:
    values = [r["input_before"] for r in rows if r["input_before"]]
    if not values:
        return {"n": 0, "excluded_no_prior_reading": len(rows)}
    return {
        "n": len(values),
        "excluded_no_prior_reading": len(rows) - len(values),
        "min": min(values), "median": statistics.median(values), "max": max(values),
        "windows": sorted({r["window"] for r in rows if r["window"]}),
    }


def main(argv: list[str] | None = None) -> int:
    p = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    p.add_argument("--binary", required=True)
    p.add_argument("--sessions-dir", default=os.path.expanduser("~/.codex/sessions"))
    p.add_argument("roots", nargs="+")
    args = p.parse_args(argv)

    threads, completeness = {}, {}
    for root in args.roots:
        found, completeness[root] = descendants(args.binary, root, args.sessions_dir)
        threads.update(found)
        own = glob.glob(os.path.join(args.sessions_dir, "**", f"rollout-*{root}.jsonl"), recursive=True)
        if len(own) != 1:
            print(f"root {root}: expected one rollout, found {len(own)}", file=sys.stderr)
            return 2
        threads[root] = ("ROOT", own[0])

    rows = []
    for thread_id, (path, rollout) in sorted(threads.items()):
        for event in events(rollout):
            rows.append({"thread_id": thread_id, "agent_path": path, "stratum": "root" if path == "ROOT" else "child", **event})

    print(json.dumps({
        "schema": "compaction-band-v1",
        "binary": args.binary,
        "binary_sha256": hashlib.sha256(open(args.binary, "rb").read()).hexdigest(),
        "sessions_dir": args.sessions_dir,
        "roots": args.roots,
        "enumeration_completeness": completeness,
        "threads": len(threads),
        "compaction_events": len(rows),
        "children": summary([r for r in rows if r["stratum"] == "child"]),
        "roots_stratum": summary([r for r in rows if r["stratum"] == "root"]),
        "all": summary(rows),
        "events": rows,
    }, indent=2))
    return 0


if __name__ == "__main__":
    sys.exit(main())
