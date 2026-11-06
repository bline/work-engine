#!/usr/bin/env python3
"""Where Codex sessions compact, and which sessions follow a configured limit.

Usage:
  python3 -I compaction_points.py --since ISO --change-at ISO [--out FILE] SESSIONS_DIR [...]

Scans every rollout (*.jsonl) under the given directories whose modification
time is at or after --since. For each `compacted` record it takes the last
`last_token_usage.input_tokens` reading before it in the same rollout, the
model from the latest `turn_context`, and the recorded window.

A compaction is `mid_turn` when model or tool items of the same turn precede
it: the provider compacted automatically while the turn was working. It is
`own_turn` when it opens a turn with no earlier items, which is how a requested
compaction (`thread/compact/start`, `/compact`) appears. This is a heuristic.

--change-at splits sessions by start time (session_meta timestamp), so a
configuration change read at session start can be compared between sessions
started before and after it. Sessions started at or after --change-at are also
listed with their peak input and compactions. Read-only.
"""
from __future__ import annotations

import argparse
import datetime
import glob
import json
import os
import statistics
import sys
from collections import defaultdict

ITEM_TYPES = {"function_call", "custom_tool_call", "message", "reasoning", "local_shell_call"}


def parse_time(text: str) -> datetime.datetime:
    return datetime.datetime.fromisoformat(text.replace("Z", "+00:00"))


def scan(path: str) -> dict:
    meta, model, window, last_input, peak = {}, None, None, None, 0
    turn, items_in_turn, events = None, 0, []
    with open(path, encoding="utf-8", errors="replace") as handle:
        for line in handle:
            try:
                record = json.loads(line)
            except ValueError:
                continue
            kind, payload = record.get("type"), record.get("payload") or {}
            if kind == "session_meta" and not meta:
                meta = payload
            elif kind == "turn_context":
                model = payload.get("model") or model
                if payload.get("turn_id") and payload.get("turn_id") != turn:
                    turn, items_in_turn = payload.get("turn_id"), 0
            elif kind == "event_msg" and payload.get("type") == "task_started":
                if payload.get("turn_id") != turn:
                    turn, items_in_turn = payload.get("turn_id"), 0
            elif kind == "response_item" and payload.get("type") in ITEM_TYPES:
                items_in_turn += 1
            elif kind == "event_msg" and payload.get("type") == "token_count":
                info = payload.get("info") or {}
                window = info.get("model_context_window") or window
                value = (info.get("last_token_usage") or {}).get("input_tokens")
                if isinstance(value, int) and value > 0:
                    last_input, peak = value, max(peak, value)
            elif kind == "compacted":
                events.append({"ts": record.get("timestamp"), "model": model, "window": window,
                               "before": last_input, "mid_turn": items_in_turn > 0})
    source = meta.get("source")
    if isinstance(source, dict) and "subagent" in source:
        spawn = (source["subagent"] or {}).get("thread_spawn") or {}
        host = "subagent of " + str(spawn.get("parent_thread_id"))
    else:
        host = str(source)
    return {"thread": meta.get("id"), "started": meta.get("timestamp"), "host": host,
            "model": model, "window": window, "peak_input": peak, "events": events}


def band(values: list[int]) -> dict:
    values = sorted(values)
    return {"n": len(values), "min": values[0], "median": int(statistics.median(values)), "max": values[-1]}


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--since", required=True)
    parser.add_argument("--change-at", required=True)
    parser.add_argument("--out")
    parser.add_argument("dirs", nargs="+")
    args = parser.parse_args()
    since, change = parse_time(args.since), parse_time(args.change_at)
    files = sorted({f for d in args.dirs for f in glob.glob(os.path.join(os.path.expanduser(d), "**", "*.jsonl"), recursive=True)
                    if datetime.datetime.fromtimestamp(os.path.getmtime(f), datetime.timezone.utc) >= since})
    threads = [scan(f) for f in files]
    groups, members = defaultdict(list), defaultdict(set)
    events = []
    for t in threads:
        started_after = bool(t["started"]) and parse_time(t["started"]) >= change
        for e in t["events"]:
            if not e["ts"] or parse_time(e["ts"]) < since or e["before"] is None:
                continue
            key = (e["model"], e["window"], "mid_turn" if e["mid_turn"] else "own_turn",
                   "started_after_change" if started_after else "started_before_change",
                   "event_after_change" if parse_time(e["ts"]) >= change else "event_before_change")
            groups[key].append(e["before"])
            members[key].add(t["thread"])
            events.append({"thread": t["thread"], **e, "thread_started_after_change": started_after})
    summary = [{"model": k[0], "window": k[1], "kind": k[2], "session": k[3], "event": k[4],
                **band(v), "threads": len(members[k])} for k, v in sorted(groups.items(), key=lambda kv: str(kv[0]))]
    post = [{k: t[k] for k in ("thread", "started", "host", "model", "window", "peak_input")}
            | {"compactions_at": [e["before"] for e in t["events"]]}
            for t in threads if t["started"] and parse_time(t["started"]) >= change]
    result = {"schema": "compaction-points-v1", "since": args.since, "change_at": args.change_at,
              "directories": args.dirs, "rollouts_scanned": len(files), "summary": summary,
              "sessions_started_after_change": sorted(post, key=lambda r: r["started"]), "events": events}
    text = json.dumps(result, indent=1)
    if args.out:
        with open(args.out, "w", encoding="utf-8") as handle:
            handle.write(text + "\n")
    json.dump({k: result[k] for k in ("rollouts_scanned", "summary", "sessions_started_after_change")}, sys.stdout, indent=1)
    print()
    return 0


if __name__ == "__main__":
    sys.exit(main())
