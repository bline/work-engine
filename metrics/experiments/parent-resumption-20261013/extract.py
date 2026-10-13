#!/usr/bin/env python3
"""Extract bounded, content-minimized native probe telemetry from local rollouts.

Run with one output JSON path. Only the two experiment agents are included.
Source line numbers and SHA-256 digests retain provenance without copying prompts.
"""
import datetime
import hashlib
import json
import pathlib
import sys

PREFIX = "/root/resumption_probe_parent"
ROOT = "01a1366f-44ff-73f3-9d3b-858d18782d47"
sessions = []
for path in sorted(pathlib.Path.home().joinpath(".codex/sessions/2026/10/13").glob("*.jsonl")):
    raw = path.read_bytes()
    lines = raw.splitlines()
    if not lines:
        continue
    meta = json.loads(lines[0]).get("payload", {})
    agent_path = meta.get("agent_path") or ""
    if agent_path not in (PREFIX, PREFIX + "/leaf"):
        continue
    calls = {}
    events = []
    for line_number, line in enumerate(lines, 1):
        try:
            record = json.loads(line)
        except json.JSONDecodeError:
            continue  # A concurrent writer may leave the last line incomplete.
        payload = record.get("payload", {})
        kind = record.get("type")
        event = {"timestamp": record.get("timestamp"), "source_line": line_number,
                 "line_sha256": hashlib.sha256(line).hexdigest(), "kind": kind}
        subtype = payload.get("type")
        if kind == "inter_agent_communication_metadata":
            event["trigger_turn"] = payload.get("trigger_turn")
        elif kind == "token_usage_record":
            event.update({key: payload.get(key) for key in ("response_id", "turn_id", "usage")})
        elif kind == "turn_context":
            event.update({key: payload.get(key) for key in ("model", "effort", "turn_id")})
        elif kind == "event_msg" and subtype in ("task_started", "task_complete", "turn_aborted"):
            event.update({"event": subtype, "turn_id": payload.get("turn_id"),
                          "turn_attribution": payload.get("turn_attribution")})
            if subtype == "task_complete":
                event["last_agent_message"] = payload.get("last_agent_message")
        elif kind == "response_item" and subtype in ("function_call", "custom_tool_call"):
            name = payload.get("name")
            calls[payload.get("call_id")] = name
            event.update({"event": subtype, "name": name, "call_id": payload.get("call_id")})
            if name in ("wait_agent", "sleep"):
                event["arguments"] = payload.get("arguments") or payload.get("input")
        elif kind == "response_item" and subtype in ("function_call_output", "custom_tool_call_output"):
            name = calls.get(payload.get("call_id"))
            if name not in ("wait_agent", "sleep"):
                continue
            event.update({"event": subtype, "name": name, "call_id": payload.get("call_id"),
                          "output": payload.get("output")})
        elif kind == "response_item" and subtype == "agent_message":
            event["event"] = subtype
            event["author"] = payload.get("author")
            event["recipient"] = payload.get("recipient")
            # Contents may be encrypted on disk; do not infer receipt from markers.
            event["content_copied"] = False
        else:
            continue
        events.append(event)
    sessions.append({"source_path": str(path), "source_bytes": len(raw),
                     "source_sha256": hashlib.sha256(raw).hexdigest(),
                     "metadata": {key: meta.get(key) for key in
                                  ("id", "agent_path", "parent_thread_id", "cli_version", "timestamp")},
                     "events": events})
result = {"observed_at": datetime.datetime.now(datetime.timezone.utc).isoformat(),
          "root_session": ROOT, "sessions": sessions}
pathlib.Path(sys.argv[1]).write_text(json.dumps(result, indent=2) + "\n")
print(json.dumps({"output": sys.argv[1], "sessions": len(sessions),
                  "events": sum(len(s["events"]) for s in sessions)}))
