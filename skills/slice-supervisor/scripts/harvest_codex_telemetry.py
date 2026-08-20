#!/usr/bin/env python3
"""Bind a launched Codex builder to its rollout and harvest runtime telemetry."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any, Iterable


class TelemetryIngressError(ValueError):
    """The supplied rollout artifacts do not prove one terminal builder run."""


MEASUREMENT_NAMES = (
    "input_tokens",
    "cached_input_tokens",
    "output_tokens",
    "reasoning_tokens",
    "peak_context_tokens",
    "turn_count",
    "wall_clock_seconds",
)


def _read_jsonl(path: Path) -> list[dict[str, Any]]:
    records: list[dict[str, Any]] = []
    try:
        with path.open(encoding="utf-8") as source:
            for ordinal, line in enumerate(source, start=1):
                if not line.strip():
                    continue
                value = json.loads(line)
                if not isinstance(value, dict):
                    raise TelemetryIngressError(f"{path}:{ordinal}: event must be an object")
                value = dict(value)
                value["_ordinal"] = ordinal
                records.append(value)
    except (OSError, json.JSONDecodeError) as error:
        raise TelemetryIngressError(f"cannot parse rollout {path}: {error}") from error
    if not records:
        raise TelemetryIngressError(f"rollout is empty: {path}")
    return records


def _session_meta(records: list[dict[str, Any]], role: str) -> dict[str, Any]:
    matches = [record for record in records if record.get("type") == "session_meta"]
    if len(matches) != 1 or not isinstance(matches[0].get("payload"), dict):
        raise TelemetryIngressError(f"{role} rollout must contain exactly one session_meta")
    return matches[0]


def _event_payloads(records: Iterable[dict[str, Any]], event_type: str) -> list[dict[str, Any]]:
    return [
        record
        for record in records
        if record.get("type") == "event_msg"
        and isinstance(record.get("payload"), dict)
        and record["payload"].get("type") == event_type
    ]


def _launch(parent: list[dict[str, Any]], agent_path: str) -> tuple[dict[str, Any], dict[str, Any]]:
    parent_meta = _session_meta(parent, "parent")
    matches: list[dict[str, Any]] = []
    for record in _event_payloads(parent, "item_completed"):
        item = record["payload"].get("item")
        if (
            isinstance(item, dict)
            and item.get("type") == "SubAgentActivity"
            and item.get("kind") == "started"
            and item.get("agent_path") == agent_path
            and isinstance(item.get("agent_thread_id"), str)
            and item["agent_thread_id"]
        ):
            matches.append(record)
    if len(matches) != 1:
        raise TelemetryIngressError(
            f"expected one started launch for agent_path {agent_path!r}; found {len(matches)}"
        )
    return parent_meta, matches[0]


def _bound_child(
    parent_meta: dict[str, Any],
    launch: dict[str, Any],
    candidates: list[tuple[Path, list[dict[str, Any]]]],
    agent_path: str,
) -> tuple[Path, list[dict[str, Any]], dict[str, Any]]:
    parent_id = parent_meta["payload"].get("id")
    child_id = launch["payload"]["item"]["agent_thread_id"]
    matches: list[tuple[Path, list[dict[str, Any]], dict[str, Any]]] = []
    for path, records in candidates:
        meta = _session_meta(records, "child candidate")
        payload = meta["payload"]
        if payload.get("id") != child_id:
            continue
        source = payload.get("source")
        subagent = source.get("subagent") if isinstance(source, dict) else None
        spawn = subagent.get("thread_spawn") if isinstance(subagent, dict) else None
        spawn = spawn if isinstance(spawn, dict) else {}
        if (
            payload.get("parent_thread_id") != parent_id
            or payload.get("thread_source") != "subagent"
            or spawn.get("parent_thread_id") != parent_id
            or spawn.get("agent_path") != agent_path
        ):
            raise TelemetryIngressError("child rollout identity does not corroborate parent launch")
        matches.append((path, records, meta))
    if len(matches) != 1:
        raise TelemetryIngressError(
            f"expected one corroborated child rollout for thread {child_id!r}; found {len(matches)}"
        )
    return matches[0]


def _terminal_state(records: list[dict[str, Any]]) -> tuple[dict[str, Any], int]:
    starts = _event_payloads(records, "task_started")
    completions = _event_payloads(records, "task_complete")
    started_ids = [record["payload"].get("turn_id") for record in starts]
    completed_ids = [record["payload"].get("turn_id") for record in completions]
    if not started_ids or any(not isinstance(turn_id, str) for turn_id in started_ids):
        raise TelemetryIngressError("child rollout has no valid task_started event")
    if (
        len(started_ids) != len(set(started_ids))
        or len(completed_ids) != len(set(completed_ids))
        or set(started_ids) != set(completed_ids)
    ):
        raise TelemetryIngressError("child rollout contains incomplete or duplicate turns")
    terminal = completions[-1]
    if records[-1] is not terminal or terminal["payload"].get("turn_id") != started_ids[-1]:
        raise TelemetryIngressError("child rollout does not end in completion of its latest turn")
    return terminal, len(started_ids)


def _provenance(record: dict[str, Any], artifact_role: str) -> dict[str, Any]:
    return {
        "artifact_role": artifact_role,
        "ordinal": record["_ordinal"],
        "event_type": record.get("payload", {}).get("type", record.get("type")),
        "timestamp": record.get("timestamp"),
    }


def _measurement(value: int | float | None, provenance: dict[str, Any] | None) -> dict[str, Any]:
    if value is not None and (
        isinstance(value, bool) or not isinstance(value, (int, float)) or value < 0
    ):
        raise TelemetryIngressError("observed measurement must be a nonnegative number")
    return {
        "availability": "observed" if value is not None else "unavailable",
        "value": value,
        "provenance": [provenance] if provenance is not None else [],
    }


def harvest(
    *,
    run_id: str,
    slice_id: str,
    agent_path: str,
    parent_rollout: Path,
    child_rollouts: list[Path],
) -> dict[str, Any]:
    if not run_id or not slice_id or not agent_path:
        raise TelemetryIngressError("run_id, slice_id, and agent_path must be nonempty")
    parent = _read_jsonl(parent_rollout)
    parent_meta, launch = _launch(parent, agent_path)
    candidates = [(path, _read_jsonl(path)) for path in child_rollouts]
    child_path, child, child_meta = _bound_child(
        parent_meta, launch, candidates, agent_path
    )
    terminal, turn_count = _terminal_state(child)

    token_events = _event_payloads(child, "token_count")
    token_event = token_events[-1] if token_events else None
    usage = (
        token_event.get("payload", {}).get("info", {}).get("total_token_usage", {})
        if token_event
        else {}
    )
    if not isinstance(usage, dict):
        usage = {}
    token_provenance = _provenance(token_event, "child_rollout") if token_event else None

    starts = _event_payloads(child, "task_started")
    start_seconds = starts[0]["payload"].get("started_at")
    completed_seconds = terminal["payload"].get("completed_at")
    wall_clock = (
        completed_seconds - start_seconds
        if isinstance(start_seconds, (int, float))
        and isinstance(completed_seconds, (int, float))
        and completed_seconds >= start_seconds
        else None
    )
    turn_provenance = _provenance(terminal, "child_rollout")

    measurements = {
        "input_tokens": _measurement(usage.get("input_tokens"), token_provenance),
        "cached_input_tokens": _measurement(usage.get("cached_input_tokens"), token_provenance),
        "output_tokens": _measurement(usage.get("output_tokens"), token_provenance),
        "reasoning_tokens": _measurement(usage.get("reasoning_output_tokens"), token_provenance),
        "peak_context_tokens": _measurement(None, None),
        "turn_count": _measurement(turn_count, turn_provenance),
        "wall_clock_seconds": _measurement(wall_clock, turn_provenance if wall_clock is not None else None),
    }
    assert set(measurements) == set(MEASUREMENT_NAMES)

    parent_payload = parent_meta["payload"]
    child_payload = child_meta["payload"]
    return {
        "schema_version": 1,
        "run_id": run_id,
        "slice_id": slice_id,
        "builder_runtime": {
            "source_kind": "codex_rollout_jsonl",
            "source_identity": {
                "parent_thread_id": parent_payload["id"],
                "child_thread_id": child_payload["id"],
                "agent_path": agent_path,
                "terminal_turn_id": terminal["payload"]["turn_id"],
            },
            "artifacts": {
                "parent": str(parent_rollout),
                "child": str(child_path),
            },
            "binding_provenance": {
                "parent_launch": _provenance(launch, "parent_rollout"),
                "child_identity": _provenance(child_meta, "child_rollout"),
                "terminal_event": _provenance(terminal, "child_rollout"),
            },
            "measurements": measurements,
        },
        "tool_runtime": [],
    }


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--run-id", required=True)
    parser.add_argument("--slice-id", required=True)
    parser.add_argument("--agent-path", required=True)
    parser.add_argument("--parent-rollout", required=True, type=Path)
    parser.add_argument("--child-rollout", required=True, action="append", type=Path)
    args = parser.parse_args()
    try:
        result = harvest(
            run_id=args.run_id,
            slice_id=args.slice_id,
            agent_path=args.agent_path,
            parent_rollout=args.parent_rollout,
            child_rollouts=args.child_rollout,
        )
    except TelemetryIngressError as error:
        parser.error(str(error))
    print(json.dumps(result, sort_keys=True, separators=(",", ":")))


if __name__ == "__main__":
    main()
