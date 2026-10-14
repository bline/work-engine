#!/usr/bin/env python3
"""Extract bounded, content-minimized context-replacement telemetry from local rollouts.

Run with one output JSON path. Nothing here copies prompts, tool bodies, agent
messages or retained text: only names, counts, token numbers, byte sizes,
timestamps, and per-line SHA-256 digests with source line numbers, so a reader
can return to the exact source line without the content travelling with the
evidence.

Two rollout-reading traps this script exists to avoid: a tool call's body is in
`input`, not `arguments`; and per-call usage is `info.last_token_usage`, while
`info.total_token_usage` is account-lifetime cumulative. A context replacement
is identified by a payload carrying `window_number` and has no `type` field, so
a scan keyed on payload type finds none.
"""
import collections
import datetime
import hashlib
import json
import pathlib
import sys

COEF = 7.826 / 1e6          # Plus-units per weighted token, gpt-6
SINCE = "2026-10-04"        # rollouts whose entries are newer than this
BANDS = 25000

def band_label(tokens: int) -> str:
    low = min(int(tokens / BANDS) * BANDS, 250000)
    return "%d-%dK" % (low // 1000, low // 1000 + BANDS // 1000)

def digest(payload: bytes) -> str:
    return hashlib.sha256(payload).hexdigest()

def scan(path: pathlib.Path) -> dict | None:
    lines = path.read_bytes().splitlines()
    if not lines:
        return None
    try:
        meta = json.loads(lines[0]).get("payload", {})
    except json.JSONDecodeError:
        return None
    tools: collections.Counter = collections.Counter()
    triggers: collections.Counter = collections.Counter()
    bands: dict[str, list] = {}
    replacements: list[dict] = []
    window = 0
    last_call = None                  # (timestamp, input_tokens) of the previous model call
    contexts: list[int] = []
    units = 0.0
    calls = 0
    climb: list[dict] = []            # the first calls after the last replacement seen
    for number, line in enumerate(lines, 1):
        try:
            record = json.loads(line)
        except json.JSONDecodeError:
            continue                  # a concurrent writer may leave the last line partial
        stamp = record.get("timestamp") or ""
        payload = record.get("payload") or {}
        kind = payload.get("type")
        if "window_number" in payload:
            retained = payload.get("retained_context")
            replacements.append({
                "timestamp": stamp, "source_line": number,
                "line_sha256": digest(line),
                "window_number": payload.get("window_number"),
                "previous_window_id": payload.get("previous_window_id"),
                "window_id": payload.get("window_id"),
                "retained_context_bytes": len(json.dumps(retained)) if retained is not None else None,
                "retained_context_keys": sorted(retained) if isinstance(retained, dict) else None,
                "discarded_bytes": {
                    key: len(json.dumps(payload.get(key)))
                    for key in ("replacement_history", "replacement_history_metadata",
                                "guardian_history") if key in payload},
                "context_before_tokens": last_call[1] if last_call else None,
                "context_before_at": last_call[0] if last_call else None,
            })
            climb = []
            continue
        if kind == "task_started":
            triggers[(payload.get("turn_attribution") or {}).get("turn_trigger") or "unstated"] += 1
            window = max(window, payload.get("model_context_window") or 0)
            continue
        if kind in ("function_call", "custom_tool_call"):
            tools[payload.get("name") or "unnamed"] += 1
            continue
        if kind != "token_count":
            continue
        usage = ((payload.get("info") or {}).get("last_token_usage")) or {}
        taken, cached, produced = (usage.get("input_tokens", 0),
                                   usage.get("cached_input_tokens", 0),
                                   usage.get("output_tokens", 0))
        if not taken and not produced:
            continue
        cost = ((taken - cached) + cached / 10 + 5 * produced) * COEF
        last_call = (stamp, taken)
        contexts.append(taken)
        units += cost
        calls += 1
        entry = bands.setdefault(band_label(taken), [0, 0.0, 0])
        entry[0] += 1
        entry[1] += cost
        entry[2] += taken
        if len(climb) < 14:
            climb.append({"timestamp": stamp, "source_line": number,
                          "context_tokens": taken, "units": round(cost, 4)})
    if not calls:
        return None
    return {
        "id": meta.get("id"), "agent_path": meta.get("agent_path"),
        "agent_role": meta.get("agent_role"), "parent_thread_id": meta.get("parent_thread_id"),
        "rollout": str(path).replace(str(pathlib.Path.home()), "~"),
        "line_count": len(lines), "first_timestamp": meta.get("timestamp"),
        "model_context_window": window or None,
        "tools": dict(tools.most_common()),
        "new_context_tool_present": "new_context" in tools,
        "turn_triggers": dict(triggers.most_common()),
        "model_calls": calls, "estimated_units": round(units, 2),
        "context_mean_tokens": round(sum(contexts) / len(contexts)),
        "context_peak_tokens": max(contexts),
        "replacements": replacements,
        "climb_after_last_replacement": climb if replacements else [],
        "cost_by_context_band": {
            key: {"calls": value[0], "units": round(value[1], 2),
                  "units_per_call": round(value[1] / value[0], 4),
                  "mean_context_tokens": round(value[2] / value[0])}
            for key, value in sorted(bands.items(), key=lambda item: int(item[0].split("-")[0]))},
    }

def main() -> int:
    if len(sys.argv) != 2:
        print("usage: extract.py <output.json>", file=sys.stderr)
        return 2
    home = pathlib.Path.home()
    roots = [home / ".codex/sessions", home / ".codex/archived_sessions"]
    sessions = []
    for root in roots:
        if not root.is_dir():
            continue
        for path in sorted(root.rglob("*.jsonl")):
            if path.stat().st_mtime < datetime.datetime.fromisoformat(SINCE).timestamp():
                continue
            observed = scan(path)
            if observed and observed["replacements"]:
                sessions.append(observed)
    sessions.sort(key=lambda item: item["first_timestamp"] or "")
    combined: dict[str, list] = {}
    for observed in sessions:
        for key, value in observed["cost_by_context_band"].items():
            entry = combined.setdefault(key, [0, 0.0])
            entry[0] += value["calls"]
            entry[1] += value["units"]
    config = home / ".codex/supervisor.config.toml"
    document = {
        "observed_at": datetime.datetime.now(datetime.timezone.utc).isoformat(),
        "method": "local rollout extraction; no provider or network access",
        "units": "estimated Plus-units, %g per weighted token (uncached + cached/10 + 5*output)" % COEF,
        "replacement_configuration": {
            "file": str(config).replace(str(home), "~"),
            "sha256": digest(config.read_bytes()) if config.is_file() else None,
            "settings": [line.strip() for line in config.read_text().splitlines()
                         if line.strip()] if config.is_file() else None,
        },
        "session_count": len(sessions),
        "replacement_count": sum(len(item["replacements"]) for item in sessions),
        "combined_cost_by_context_band": {
            key: {"calls": value[0], "units": round(value[1], 2),
                  "units_per_call": round(value[1] / value[0], 4)}
            for key, value in sorted(combined.items(), key=lambda item: int(item[0].split("-")[0]))},
        "sessions": sessions,
    }
    pathlib.Path(sys.argv[1]).write_text(json.dumps(document, indent=2, sort_keys=True) + "\n")
    print("%d sessions, %d replacements -> %s" % (len(sessions), document["replacement_count"], sys.argv[1]))
    return 0

if __name__ == "__main__":
    raise SystemExit(main())
