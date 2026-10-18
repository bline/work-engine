#!/usr/bin/env python3
"""Bounded, read-only inspection of admitted historical slice-builder gate captures."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import stat
import sys
from datetime import datetime, timezone
from pathlib import Path
from typing import Any


def fail(message: str) -> None:
    raise ValueError(message)


def canonical(value: Any) -> bytes:
    return json.dumps(value, sort_keys=True, separators=(",", ":")).encode()


REPAIR_DESCRIPTOR_SHA256 = "d9ea143a84b57f84dd884d81983376a5f0d8589d8ae95f08da4c1dd924576858"
V3_FIELDS = ["checkpoint", "contract_repair", "gate", "metric_discrepancy",
             "metrics_comparison", "provider_attempts", "review_summary",
             "scheduler_item", "source_limit"]


def unique_pairs(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    result: dict[str, Any] = {}
    for key, value in pairs:
        if key in result:
            fail("duplicate inspection grant field")
        result[key] = value
    return result


def selected_line(path: Path, number: int) -> bytes:
    if number < 1 or number > 10000:
        fail("capture line is outside the bounded range")
    # O_NONBLOCK prevents a FIFO open from waiting before fstat can reject it.
    fd = os.open(path, os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK | os.O_CLOEXEC)
    try:
        if not stat.S_ISREG(os.fstat(fd).st_mode):
            fail("capture is not a regular file")
        with os.fdopen(fd, "rb", closefd=False) as stream:
            consumed = 0
            for index in range(1, number + 1):
                line_limit = 1024 * 1024 if index == number else 3 * 1024 * 1024
                raw = stream.readline(line_limit + 1)
                consumed += len(raw)
                if consumed > 32 * 1024 * 1024 or len(raw) > line_limit:
                    fail("capture prefix or line exceeds the read limit")
                if not raw:
                    break
                if not raw.endswith(b"\n") and stream.read(1):
                    fail("capture line exceeds the read limit")
                if index == number:
                    return raw
    finally:
        os.close(fd)
    fail("capture line is unavailable")


def trusted_descriptor(path: Path, attempt: int) -> dict[str, Any]:
    """Take a capture selector only from an opened private local grant."""
    if not path.is_absolute():
        fail("inspection grant path must be absolute")
    parent_fd = os.open(path.parent, os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW)
    try:
        parent = os.fstat(parent_fd)
        if parent.st_uid != os.geteuid() or stat.S_IMODE(parent.st_mode) != 0o700:
            fail("inspection grant directory owner or mode is invalid")
        fd = os.open(path.name, os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK, dir_fd=parent_fd)
        try:
            info = os.fstat(fd)
            if (not stat.S_ISREG(info.st_mode) or info.st_uid != os.geteuid()
                    or stat.S_IMODE(info.st_mode) != 0o600 or info.st_size > 64 * 1024):
                fail("inspection grant file type, owner, mode or size is invalid")
            with os.fdopen(fd, "rb", closefd=False) as stream:
                raw = stream.read(64 * 1024 + 1)
                if len(raw) > 64 * 1024:
                    fail("inspection grant grew beyond read limit")
                grant = json.loads(raw, object_pairs_hook=unique_pairs)
        finally:
            os.close(fd)
    finally:
        os.close(parent_fd)
    version = grant.get("schema_version")
    profile = grant.get("profile")
    if ((version, profile) not in {
            (1, "historical-s5-local-inspection-v1"),
            (2, "historical-s5-local-inspection-v2"),
            (3, "historical-s5-local-inspection-v3"),
        }
            or (version in (2, 3) and (not isinstance(grant.get("provider"), dict)
                                  or not isinstance(grant.get("scheduler"), dict)
                                  or not isinstance(grant.get("metrics"), dict)))
            or (version == 3 and (
                set(grant) != {"schema_version", "profile", "grant_id", "repository",
                               "run_id", "slice_number", "issued_at", "expires_at",
                               "issuer_provenance", "peer_uid", "audience", "fields",
                               "checkpoints", "gates", "reviews", "provider", "scheduler",
                               "metrics", "repair_contract"}
                or grant.get("fields") != V3_FIELDS
                or not isinstance(grant.get("repair_contract"), dict)
                or hashlib.sha256(canonical(grant["repair_contract"])).hexdigest()
                   != REPAIR_DESCRIPTOR_SHA256))
            or grant.get("run_id") != "skills-migration-app-server-20260828"
            or grant.get("slice_number") != 6 or grant.get("peer_uid") != os.geteuid()
            or not isinstance(grant.get("gates"), list) or len(grant["gates"]) != 5):
        fail("inspection grant does not admit the S5 gate case")
    expires = datetime.fromisoformat(grant["expires_at"].replace("Z", "+00:00"))
    if expires.tzinfo is None or expires <= datetime.now(timezone.utc):
        fail("inspection grant expired")
    if attempt not in range(1, 6) or grant["gates"][attempt - 1].get("attempt") != attempt:
        fail("gate attempt is not in the trusted descriptor")
    return grant["gates"][attempt - 1]


def nested_json(value: Any):
    if isinstance(value, str):
        try:
            value = json.loads(value)
        except json.JSONDecodeError:
            return
    if isinstance(value, dict):
        if isinstance(value.get("checks"), list):
            yield value
        for key in ("result", "output", "content", "text"):
            if key in value:
                yield from nested_json(value[key])
    elif isinstance(value, list):
        for item in value:
            yield from nested_json(item)


def extract_runner(row: dict[str, Any], call_id: str) -> dict[str, Any]:
    payload = row.get("payload")
    if (row.get("type") != "response_item" or not isinstance(payload, dict)
            or payload.get("type") != "custom_tool_call_output" or payload.get("call_id") != call_id):
        fail("runner capture envelope does not match trusted call")
    matches = [value for value in nested_json(payload.get("output"))
               if {"status", "checks", "failed_check", "totals", "duration_seconds"}.issubset(value)]
    if len(matches) != 1:
        fail("runner capture must contain one gate result")
    return matches[0]


def extract_coordinator(row: dict[str, Any], event_id: str) -> dict[str, Any]:
    payload = row.get("payload")
    if (row.get("type") != "event_msg" or not isinstance(payload, dict)
            or payload.get("type") != "item_completed" or not isinstance(payload.get("item"), dict)
            or payload["item"].get("id") != event_id):
        fail("coordinator capture event does not match trusted event")
    stdout = payload["item"].get("stdout")
    if not isinstance(stdout, str) or not stdout:
        fail("coordinator event has no output")
    try:
        return json.loads(stdout.splitlines()[0])
    except json.JSONDecodeError as error:
        fail(f"coordinator receipt is malformed: {error}")


def verify_coordinator_output(descriptor: dict[str, Any], event: dict[str, Any]) -> str:
    line = descriptor.get("output_line")
    if type(line) is not int or line <= descriptor["capture_line"]:
        fail("coordinator output line is invalid")
    raw = selected_line(Path(descriptor["capture_path"]), line)
    digest = hashlib.sha256(raw).hexdigest()
    if digest != descriptor.get("output_sha256"):
        fail("coordinator output bytes do not match trusted descriptor")
    output = json.loads(raw)
    payload = output.get("payload", {})
    if (output.get("type") != "response_item" or payload.get("type") != "custom_tool_call_output"
            or payload.get("call_id") != descriptor.get("call_id")
            or not isinstance(payload.get("output"), list)):
        fail("coordinator output envelope does not match trusted call")
    stdout = event["payload"]["item"].get("stdout", "").splitlines()[0]
    if not any(isinstance(item, dict) and stdout in item.get("text", "") for item in payload["output"]):
        fail("coordinator output does not echo the captured receipt")
    return digest


def inspect(descriptor: dict[str, Any], attempt: int, checkpoint: dict[str, Any]) -> dict[str, Any]:
    if (type(attempt) is not int or attempt not in range(1, 6)
            or descriptor.get("attempt") != attempt
            or descriptor.get("checkpoint_ref") != checkpoint.get("ref")
            or descriptor.get("checkpoint_oid") != checkpoint.get("commit_oid")):
        fail("gate capture is not paired with the inspected checkpoint")
    metadata = checkpoint.get("metadata")
    if (not isinstance(metadata, dict)
            or metadata.get("run_id") != "skills-migration-app-server-20260828"
            or metadata.get("slice_number") != 6
            or metadata.get("candidate_attempt") != attempt
            or metadata.get("kind") != "candidate"):
        fail("inspected checkpoint is outside the candidate attempt")
    path = Path(descriptor["capture_path"])
    if not path.is_absolute() or not isinstance(descriptor.get("capture_line"), int):
        fail("capture descriptor path or line is invalid")
    raw = selected_line(path, descriptor["capture_line"])
    if hashlib.sha256(raw).hexdigest() != descriptor.get("capture_sha256"):
        fail("capture bytes do not match trusted descriptor")
    row = json.loads(raw)
    if descriptor.get("source_kind") == "slice_builder_runner_output" and attempt <= 4:
        receipt = extract_runner(row, descriptor["call_id"])
        if (not {"status", "checks", "failed_check", "totals", "duration_seconds"}.issubset(receipt)
                or any(key in receipt for key in ("run_id", "slice_number", "candidate_attempt"))
                or receipt["status"] not in {"passed", "failed"}
                or not isinstance(receipt["totals"], dict)):
            fail("runner receipt schema is invalid")
        field = "state"
    elif descriptor.get("source_kind") == "coordinator_constructed_receipt" and attempt == 5:
        receipt = extract_coordinator(row, descriptor["event_id"])
        output_sha256 = verify_coordinator_output(descriptor, row)
        if (receipt.get("kind") != "deterministic_gate_receipt"
                or receipt.get("run_id") != metadata["run_id"]
                or receipt.get("slice_number") != metadata["slice_number"]
                or receipt.get("candidate_attempt") != attempt):
            fail("coordinator receipt identity is invalid")
        field = "result"
    else:
        fail("capture source kind does not match attempt")
    checks = receipt.get("checks")
    if (not isinstance(checks, list) or not checks or len(checks) > 64
            or any(not isinstance(check, dict) or not isinstance(check.get("identity"), str)
                   or not check["identity"] or check.get(field) not in {"passed", "failed", "skipped"}
                   for check in checks)):
        fail("gate checks are invalid")
    digest = hashlib.sha256(canonical(receipt)).hexdigest()
    if digest != metadata["gate_receipt_digest"] or digest != descriptor.get("receipt_sha256"):
        fail("gate receipt does not match inspected checkpoint digest")
    passed_count = sum(check[field] == "passed" for check in checks)
    status = receipt["status"] if field == "state" else (
        "all_reported_checks_passed" if passed_count == len(checks) else "reported_checks_not_all_passed")
    return {
        "source_owner": "slice-builder", "source_kind": descriptor["source_kind"],
        "capture_sha256": descriptor["capture_sha256"], "capture_line": descriptor["capture_line"],
        "corroborating_output_sha256": output_sha256 if attempt == 5 else None,
        "receipt_sha256": digest, "checkpoint_ref": checkpoint["ref"],
        "checkpoint_oid": checkpoint["commit_oid"], "candidate_attempt": attempt,
        "status": status,
        "check_count": len(checks),
        "passed_count": passed_count,
        "checks": [{"identity": check["identity"], "outcome": check[field]} for check in checks],
        "integrity": "capture_and_checkpoint_digest_verified",
        "source_limit": "historical producer capture; no independent execution attestation",
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--inspection-grant", required=True, type=Path)
    parser.add_argument("--attempt", required=True, type=int)
    parser.add_argument("--checkpoint-json", required=True)
    args = parser.parse_args()
    try:
        result = inspect(trusted_descriptor(args.inspection_grant, args.attempt),
                         args.attempt, json.loads(args.checkpoint_json))
    except (OSError, KeyError, TypeError, ValueError, json.JSONDecodeError) as error:
        print(f"legacy_gate_inspect: {error}", file=sys.stderr)
        return 2
    print(json.dumps(result, sort_keys=True, separators=(",", ":")))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
