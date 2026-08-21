#!/usr/bin/env python3
"""Recover the next legal inter-slice state for one named campaign run."""

from __future__ import annotations

import argparse
import importlib.util
import json
import subprocess
import sys
from pathlib import Path
from typing import Any


def load_validator():
    path = Path(__file__).with_name("append_metrics.py")
    spec = importlib.util.spec_from_file_location("work_engine_append_metrics", path)
    if not spec or not spec.loader:
        raise RuntimeError(f"cannot load receipt validator: {path}")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


APPEND_METRICS = load_validator()


def load_checkpoint_lifecycle():
    path = Path(__file__).with_name("checkpoint_lifecycle.py")
    spec = importlib.util.spec_from_file_location("work_engine_checkpoint_lifecycle", path)
    if not spec or not spec.loader:
        raise RuntimeError(f"cannot load checkpoint lifecycle: {path}")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


CHECKPOINT_LIFECYCLE = load_checkpoint_lifecycle()
completion_offer_path = Path(__file__).with_name("completion_offer_lifecycle.py")
completion_offer_spec = importlib.util.spec_from_file_location(
    "work_engine_completion_offer", completion_offer_path
)
if not completion_offer_spec or not completion_offer_spec.loader:
    raise RuntimeError(f"cannot load completion offer lifecycle: {completion_offer_path}")
COMPLETION_OFFER = importlib.util.module_from_spec(completion_offer_spec)
completion_offer_spec.loader.exec_module(COMPLETION_OFFER)


def fail(message: str) -> None:
    raise ValueError(message)


def load_history(path: Path) -> list[dict[str, Any]]:
    records = []
    with path.open(encoding="utf-8") as stream:
        for line_number, line in enumerate(stream, start=1):
            try:
                records.append(APPEND_METRICS.validate(json.loads(line)))
            except (json.JSONDecodeError, ValueError) as error:
                fail(f"invalid receipt at line {line_number}: {error}")
    return records


def nonresumable(last: dict[str, Any], reason: str) -> dict[str, Any]:
    result = {
        "resumable": False,
        "reason": reason,
        "run_id": last["run_id"],
        "last_slice_number": last["slice_number"],
        "next_slice_number": None,
        "handoff_receipt": None,
    }
    if last["status"] in {"stopped", "failed"}:
        result["terminal_state"] = {
            "status": last["status"],
            "stop_reason": last["stop_reason"],
        }
    return result


def accepted_checkpoint(last: dict[str, Any]) -> dict[str, Any] | None:
    checkpoint = last.get("producer_metrics", {}).get("slice_checkpoint")
    if checkpoint is None:
        return None
    APPEND_METRICS.validate_checkpoint_projection(checkpoint, last)
    CHECKPOINT_LIFECYCLE.CHECKPOINT.validate_lifecycle_receipt(
        checkpoint, "accepted", require_paths=False
    )
    repository = Path(checkpoint["repository"])
    complete = subprocess.run(
        ["git", "-C", str(repository), "rev-parse", "--verify", checkpoint["ref"]],
        stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True, check=False,
    )
    if complete.returncode or complete.stdout.strip() != checkpoint["checkpoint_commit_oid"]:
        fail("accepted checkpoint ref is missing or does not match the terminal receipt")
    tree = subprocess.run(
        ["git", "-C", str(repository), "rev-parse", f"{checkpoint['checkpoint_commit_oid']}^{{tree}}"],
        stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True, check=False,
    )
    if tree.returncode or tree.stdout.strip() != checkpoint["checkpoint_tree_oid"]:
        fail("accepted checkpoint tree does not match the terminal receipt")
    return checkpoint


def resume(path: Path, campaign_preflight: Any, run_id: str) -> dict[str, Any]:
    if not isinstance(campaign_preflight, dict):
        fail("campaign_preflight must be an object")
    if not isinstance(run_id, str) or not run_id.strip():
        fail("run_id must be a nonempty string")
    selected = [record for record in load_history(path) if record["run_id"] == run_id]
    if not selected:
        fail(f"run_id not found: {run_id}")
    numbers = [record["slice_number"] for record in selected]
    if numbers != list(range(1, len(numbers) + 1)):
        fail("selected run slice numbers must be unique, ordered, and contiguous from 1")

    last = selected[-1]
    if campaign_preflight.get("engineConfig") != last.get("engine_config"):
        fail("campaign_preflight.engineConfig does not match the terminal engine_config")
    campaign_source = last.get("producer_metrics", {}).get("campaign_source")
    if campaign_preflight.get("campaignSource") != campaign_source:
        fail("campaign_preflight.campaignSource does not match terminal campaign_source")

    if last["status"] == "stopped":
        return nonresumable(last, "stopped")
    if last["status"] == "failed":
        return nonresumable(last, "failed")
    if last["more_in_scope_work_remains"] is False:
        return nonresumable(last, "objective_or_scope_complete")
    context = last.get("continuation_context")
    if context is None:
        return nonresumable(last, "continuation_unavailable")
    APPEND_METRICS.validate_continuation_context(context)
    checkpoint = accepted_checkpoint(last)
    handoff = {
        "slice_title": last["slice_title"],
        "slice_goal": last["slice_goal"],
        "outcome": last["outcome"],
        "durable_decisions": context["durable_decisions"],
        "affected_boundaries": context["affected_boundaries"],
        "placement_certificate": last["placement_certificate"],
        "unresolved_concerns": context["unresolved_concerns"],
        "deferred_scope": context["deferred_scope"],
    }
    result = {
        "resumable": True,
        "reason": "accepted_continuation",
        "run_id": run_id,
        "last_slice_number": last["slice_number"],
        "next_slice_number": last["slice_number"] + 1,
        "handoff_receipt": handoff,
    }
    if checkpoint is not None:
        result["baseline_checkpoint"] = checkpoint
        offer = COMPLETION_OFFER.load(
            Path(checkpoint["repository"]), run_id, last["slice_number"]
        )
        if offer is not None:
            result["completion_offer"] = offer
    completion_commit = last.get("producer_metrics", {}).get("slice_completion_commit")
    if completion_commit is not None:
        APPEND_METRICS.validate_completion_commit_projection(completion_commit, last, checkpoint)
        result["slice_completion_commit"] = completion_commit
    return result


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--path", required=True, type=Path)
    parser.add_argument("--campaign-preflight-json", required=True)
    parser.add_argument("--run-id", required=True)
    args = parser.parse_args()
    try:
        preflight = json.loads(args.campaign_preflight_json)
        result = resume(args.path, preflight, args.run_id)
    except (json.JSONDecodeError, OSError, ValueError) as error:
        print(f"resume_campaign: {error}", file=sys.stderr)
        return 2
    print(json.dumps(result, ensure_ascii=False, separators=(",", ":")))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
