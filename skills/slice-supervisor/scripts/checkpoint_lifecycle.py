#!/usr/bin/env python3
"""Validate checkpoint evidence and authorize supervisor lifecycle transitions."""

from __future__ import annotations

import argparse
import importlib.util
import json
import sys
from pathlib import Path
from typing import Any


CHECKPOINT_PATH = (
    Path(__file__).parents[2] / "slice-checkpoint" / "scripts" / "checkpoint.py"
)
SPEC = importlib.util.spec_from_file_location("work_engine_slice_checkpoint", CHECKPOINT_PATH)
if not SPEC or not SPEC.loader:
    raise RuntimeError(f"cannot load checkpoint adapter: {CHECKPOINT_PATH}")
CHECKPOINT = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(CHECKPOINT)

BINDING_FIELDS = {
    "checkpoint_commit_oid", "checkpoint_tree_oid", "task_patch_digest",
    "manifest_digest", "plan_version", "scope_revision", "gate_receipt_digest",
}


def fail(message: str) -> None:
    raise ValueError(message)


def validate_binding(candidate: Any, review_result: Any, gate_receipt_digest: str) -> None:
    CHECKPOINT.validate_candidate_receipt(candidate)
    if not isinstance(review_result, dict) or set(review_result) != BINDING_FIELDS:
        fail("review_result must contain the exact checkpoint binding")
    CHECKPOINT.validate_digest(gate_receipt_digest, "gate_receipt_digest")
    if candidate.get("gate_receipt_digest") != gate_receipt_digest:
        fail("candidate gate_receipt_digest does not match deterministic evidence")
    for field in BINDING_FIELDS:
        if review_result[field] != candidate.get(field):
            fail(f"review result does not match candidate {field}")


def accept(candidate: Any, review_result: Any, gate_receipt_digest: str,
           expected_accepted: str | None = None) -> dict[str, Any]:
    validate_binding(candidate, review_result, gate_receipt_digest)
    return CHECKPOINT.transition(candidate, "accepted", expected_accepted=expected_accepted)


def stop(candidate: Any) -> dict[str, Any]:
    if not isinstance(candidate, dict) or candidate.get("checkpoint_kind") != "candidate":
        fail("candidate must be a candidate checkpoint receipt")
    return CHECKPOINT.transition(candidate, "stopped")


def projection(receipt: Any) -> dict[str, Any]:
    if not isinstance(receipt, dict) or receipt.get("checkpoint_kind") not in {"accepted", "stopped"}:
        fail("checkpoint receipt must be an accepted or stopped lifecycle receipt")
    CHECKPOINT.validate_lifecycle_receipt(receipt, receipt["checkpoint_kind"])
    fields = (
        "schema_version", "checkpoint_id", "checkpoint_kind", "repository", "run_id",
        "slice_number", "candidate_attempt", "checkpoint_commit_oid", "checkpoint_tree_oid",
        "candidate_checkpoint_id", "parent_checkpoint_commit_oid", "task_patch_digest",
        "manifest_digest",
        "plan_version", "scope_revision",
        "gate_receipt_digest", "ref", "limitations",
    )
    return {field: receipt[field] for field in fields}


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--candidate-json", required=True)
    parser.add_argument("--review-result-json")
    parser.add_argument("--gate-receipt-digest")
    parser.add_argument("--stopped", action="store_true")
    parser.add_argument("--expected-accepted")
    args = parser.parse_args()
    try:
        candidate = json.loads(args.candidate_json)
        if args.stopped:
            result = stop(candidate)
        else:
            if args.review_result_json is None or args.gate_receipt_digest is None:
                fail("acceptance requires review result and gate receipt digest")
            result = accept(
                candidate, json.loads(args.review_result_json),
                args.gate_receipt_digest, args.expected_accepted,
            )
        print(json.dumps(result, separators=(",", ":")))
    except (json.JSONDecodeError, OSError, ValueError) as error:
        print(f"checkpoint_lifecycle: {error}", file=sys.stderr)
        return 2
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
