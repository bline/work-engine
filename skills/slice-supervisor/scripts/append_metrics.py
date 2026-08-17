#!/usr/bin/env python3
"""Validate and durably append one work-engine JSONL receipt under a lock."""

from __future__ import annotations

import argparse
import fcntl
import json
import os
import sys
from datetime import datetime
from pathlib import Path
from typing import Any


REQUIRED_TYPES = {
    "schema_version": int,
    "run_id": str,
    "slice_number": int,
    "timestamp": str,
    "slice_title": str,
    "slice_goal": str,
    "status": str,
    "outcome": str,
    "plan_acceptance": str,
    "worker_metrics": dict,
    "producer_metrics": dict,
}
PROHIBITED_KEYS = {
    "raw_transcript", "transcript", "chain_of_thought", "raw_test_output",
    "test_log", "debug_log", "diff", "patch", "source_excerpt",
}


def fail(message: str) -> None:
    raise ValueError(message)


def find_prohibited(value: Any, path: str = "record") -> None:
    if isinstance(value, dict):
        for key, child in value.items():
            if key.lower() in PROHIBITED_KEYS:
                fail(f"prohibited durable field: {path}.{key}")
            find_prohibited(child, f"{path}.{key}")
    elif isinstance(value, list):
        for index, child in enumerate(value):
            find_prohibited(child, f"{path}[{index}]")


def validate(record: Any) -> dict[str, Any]:
    if not isinstance(record, dict):
        fail("receipt must be a JSON object")
    for key, expected in REQUIRED_TYPES.items():
        if key not in record:
            fail(f"missing required field: {key}")
        if not isinstance(record[key], expected) or isinstance(record[key], bool):
            fail(f"{key} must be {expected.__name__}")
    if "stop_reason" not in record or not isinstance(record["stop_reason"], (str, type(None))):
        fail("stop_reason must be a string or null")
    if record["schema_version"] not in {1, 2}:
        fail("schema_version must be 1 or 2")
    if record["slice_number"] < 1:
        fail("slice_number must be positive")
    if record["status"] not in {"accepted", "stopped", "failed"}:
        fail("status must be accepted, stopped, or failed")
    if record["plan_acceptance"] not in {"procedural_auto_approval", "human_approval", "not_reached"}:
        fail("invalid plan_acceptance")
    if record["status"] == "accepted" and record["plan_acceptance"] == "not_reached":
        fail("accepted receipts require an approved plan")
    if record["status"] == "accepted" and record["stop_reason"] is not None:
        fail("accepted receipts require null stop_reason")
    if record["status"] != "accepted" and not record["stop_reason"]:
        fail("stopped/failed receipts require a stop_reason")
    if record["schema_version"] == 2:
        validate_engine_record(record)
    for key in ("run_id", "timestamp", "slice_title", "slice_goal", "outcome"):
        if not record[key].strip():
            fail(f"{key} must not be empty")
    try:
        timestamp = record["timestamp"].replace("Z", "+00:00")
        parsed = datetime.fromisoformat(timestamp)
    except ValueError as error:
        fail(f"timestamp must be ISO 8601: {error}")
    if parsed.tzinfo is None:
        fail("timestamp must include a timezone")
    find_prohibited(record)
    return record


def validate_engine_record(record: dict[str, Any]) -> None:
    required = {
        "engine_config": dict,
        "builder_skill": str,
        "validation_profile": str,
        "validation_requirement_results": dict,
        "more_in_scope_work_remains": (bool, type(None)),
    }
    for key, expected in required.items():
        if key not in record or not isinstance(record[key], expected):
            names = " or ".join(kind.__name__ for kind in expected) if isinstance(expected, tuple) else expected.__name__
            fail(f"{key} must be {names}")
    for key in ("builder_skill", "validation_profile"):
        if not record[key].strip():
            fail(f"{key} must not be empty")

    config = record["engine_config"]
    allowed_config_keys = {
        "version", "source", "objective", "work_source", "builder",
        "validation", "metrics", "limits", "approval", "notifications",
        "stop_on", "explicit_fields", "defaulted_fields", "amendments",
    }
    unknown_config_keys = set(config) - allowed_config_keys
    if unknown_config_keys:
        fail(f"unknown engine_config fields: {', '.join(sorted(unknown_config_keys))}")
    config_types = {
        "version": int,
        "source": str,
        "objective": str,
        "work_source": (dict, type(None)),
        "builder": dict,
        "validation": dict,
        "metrics": dict,
        "limits": dict,
        "approval": dict,
        "notifications": dict,
        "stop_on": list,
        "explicit_fields": list,
        "defaulted_fields": list,
        "amendments": list,
    }
    for key, expected in config_types.items():
        if key not in config or not isinstance(config[key], expected):
            names = " or ".join(kind.__name__ for kind in expected) if isinstance(expected, tuple) else expected.__name__
            fail(f"engine_config.{key} must be {names}")
    if config["version"] != 1:
        fail("engine_config.version must be 1")
    for key in ("source", "objective"):
        if not config[key].strip():
            fail(f"engine_config.{key} must not be empty")

    if not isinstance(config["builder"].get("skill"), str) or not config["builder"]["skill"].strip():
        fail("engine_config.builder.skill must be a nonempty string")
    if not isinstance(config["validation"].get("profile"), str) or not config["validation"]["profile"].strip():
        fail("engine_config.validation.profile must be a nonempty string")
    requirements = config["validation"].get("requirements")
    if not isinstance(requirements, list) or not requirements or not all(isinstance(item, str) and item for item in requirements):
        fail("engine_config.validation.requirements must be a nonempty array of nonempty strings")
    if len(set(requirements)) != len(requirements):
        fail("engine_config.validation.requirements must not contain duplicates")
    results = record["validation_requirement_results"]
    if set(results) != set(requirements):
        fail("validation_requirement_results keys must exactly match configured requirements")
    configured_skill = config["builder"].get("skill")
    if configured_skill != record["builder_skill"]:
        fail("builder_skill must match engine_config.builder.skill")
    configured_profile = config["validation"].get("profile")
    if configured_profile != record["validation_profile"]:
        fail("validation_profile must match engine_config.validation.profile")
    for key in ("explicit_fields", "defaulted_fields", "stop_on"):
        if not all(isinstance(item, str) and item for item in config[key]):
            fail(f"engine_config.{key} must contain only nonempty strings")
    overlap = set(config["explicit_fields"]) & set(config["defaulted_fields"])
    if overlap:
        fail("engine_config explicit_fields and defaulted_fields must not overlap")
    allowed = {"passed", "failed", "blocked", "not_applicable"}
    for requirement, result in results.items():
        state = result if isinstance(result, str) else result.get("state") if isinstance(result, dict) else None
        if state not in allowed:
            fail(f"invalid validation result for {requirement}")
        if state == "not_applicable" and (not isinstance(result, dict) or not str(result.get("reason", "")).strip()):
            fail(f"not_applicable validation result requires a reason: {requirement}")
        if record["status"] == "accepted" and state != "passed":
            fail(f"accepted receipt requires passed validation: {requirement}")


def load_record(args: argparse.Namespace) -> dict[str, Any]:
    if args.record_json is not None:
        raw = args.record_json
    else:
        raw = sys.stdin.read()
    try:
        return validate(json.loads(raw))
    except json.JSONDecodeError as error:
        fail(f"invalid JSON: {error}")


def append(path: Path, record: dict[str, Any]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    line = json.dumps(record, ensure_ascii=False, separators=(",", ":")) + "\n"
    with path.open("a", encoding="utf-8") as stream:
        fcntl.flock(stream.fileno(), fcntl.LOCK_EX)
        stream.write(line)
        stream.flush()
        os.fsync(stream.fileno())
        fcntl.flock(stream.fileno(), fcntl.LOCK_UN)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--path", required=True, type=Path)
    source = parser.add_mutually_exclusive_group(required=True)
    source.add_argument("--record-json")
    source.add_argument("--stdin", action="store_true")
    args = parser.parse_args()
    try:
        append(args.path, load_record(args))
    except (OSError, ValueError) as error:
        print(f"append_metrics: {error}", file=sys.stderr)
        return 2
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
