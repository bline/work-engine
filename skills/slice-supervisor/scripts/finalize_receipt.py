#!/usr/bin/env python3
"""Assemble and durably append one named-campaign terminal receipt."""

from __future__ import annotations

import argparse
import importlib.util
import json
import sys
from pathlib import Path
from typing import Any


def load_sibling(name: str):
    path = Path(__file__).with_name(f"{name}.py")
    spec = importlib.util.spec_from_file_location(f"work_engine_{name}", path)
    if not spec or not spec.loader:
        raise RuntimeError(f"cannot load terminal receipt dependency: {path}")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


ASSEMBLER = load_sibling("assemble_receipt")
APPEND_METRICS = load_sibling("append_metrics")
CHECKPOINT_LIFECYCLE = load_sibling("checkpoint_lifecycle")

COMPLETION_COMMIT_PATH = (
    Path(__file__).parents[2] / "slice-completion-commit" / "scripts" / "completion_commit.py"
)
COMPLETION_COMMIT_SPEC = importlib.util.spec_from_file_location(
    "work_engine_slice_completion_commit", COMPLETION_COMMIT_PATH
)
if not COMPLETION_COMMIT_SPEC or not COMPLETION_COMMIT_SPEC.loader:
    raise RuntimeError(f"cannot load completion-commit adapter: {COMPLETION_COMMIT_PATH}")
COMPLETION_COMMIT = importlib.util.module_from_spec(COMPLETION_COMMIT_SPEC)
COMPLETION_COMMIT_SPEC.loader.exec_module(COMPLETION_COMMIT)

HANDOFF_FIELDS = {
    "slice_title", "slice_goal", "outcome", "durable_decisions",
    "affected_boundaries", "placement_certificate", "unresolved_concerns",
    "deferred_scope",
}
CONTINUATION_FIELDS = (
    "durable_decisions", "affected_boundaries", "unresolved_concerns",
    "deferred_scope",
)


def apply_continuation(receipt: dict[str, Any], handoff: Any) -> dict[str, Any]:
    if not isinstance(handoff, dict) or set(handoff) != HANDOFF_FIELDS:
        raise ValueError("handoff_receipt fields must exactly match the handoff contract")
    for field in CONTINUATION_FIELDS:
        if not isinstance(handoff[field], list):
            raise ValueError(f"handoff_receipt.{field} must be an array")
    for field in ("slice_title", "slice_goal", "outcome", "placement_certificate"):
        if handoff[field] != receipt[field]:
            raise ValueError(f"handoff_receipt.{field} must match the assembled receipt")

    resumable = (
        receipt["status"] == "accepted"
        and receipt["more_in_scope_work_remains"] is True
    )
    if resumable:
        receipt["continuation_context"] = {
            "schema_version": 1,
            **{field: handoff[field] for field in CONTINUATION_FIELDS},
        }
    else:
        receipt.pop("continuation_context", None)
    return receipt


def finalize(
    path: Path,
    semantic_receipt: Any,
    telemetry_ingress: Any,
    campaign_preflight: Any,
    handoff_receipt: Any,
    checkpoint_receipt: Any = None,
    completion_commit_receipt: Any = None,
) -> dict[str, Any]:
    assembled = ASSEMBLER.assemble(
        semantic_receipt, telemetry_ingress, campaign_preflight
    )
    apply_continuation(assembled, handoff_receipt)
    if checkpoint_receipt is not None:
        assembled["producer_metrics"]["slice_checkpoint"] = (
            CHECKPOINT_LIFECYCLE.projection(checkpoint_receipt)
        )
    if completion_commit_receipt is not None:
        assembled["producer_metrics"]["slice_completion_commit"] = (
            COMPLETION_COMMIT.validate_receipt(completion_commit_receipt)
        )
    current = APPEND_METRICS.validate_current_write(assembled)
    APPEND_METRICS.append(path, current)
    return current


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--path", required=True, type=Path)
    parser.add_argument("--semantic-receipt-json", required=True)
    parser.add_argument("--telemetry-ingress-json", required=True)
    parser.add_argument("--campaign-preflight-json", required=True)
    parser.add_argument("--handoff-receipt-json", required=True)
    parser.add_argument("--checkpoint-receipt-json")
    parser.add_argument("--completion-commit-receipt-json")
    args = parser.parse_args()
    try:
        semantic = json.loads(args.semantic_receipt_json)
        telemetry = json.loads(args.telemetry_ingress_json)
        preflight = json.loads(args.campaign_preflight_json)
        handoff = json.loads(args.handoff_receipt_json)
        checkpoint = (
            json.loads(args.checkpoint_receipt_json)
            if args.checkpoint_receipt_json is not None else None
        )
        completion_commit = (
            json.loads(args.completion_commit_receipt_json)
            if args.completion_commit_receipt_json is not None else None
        )
        finalize(args.path, semantic, telemetry, preflight, handoff, checkpoint, completion_commit)
    except (json.JSONDecodeError, OSError, ValueError) as error:
        print(f"finalize_receipt: {error}", file=sys.stderr)
        return 2
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
