#!/usr/bin/env python3
"""Validate and durably append one work-engine JSONL receipt under a lock."""

from __future__ import annotations

import argparse
import fcntl
import hashlib
import importlib.util
import json
import os
import sys
from datetime import datetime
from pathlib import Path
from typing import Any


RESOLVER_PATH = (
    Path(__file__).parents[2] / "slice-builder" / "scripts" / "resolve_provider.py"
)
RESOLVER_SPEC = importlib.util.spec_from_file_location(
    "work_engine_resolve_provider", RESOLVER_PATH
)
if not RESOLVER_SPEC or not RESOLVER_SPEC.loader:
    raise RuntimeError(f"cannot load provider resolver: {RESOLVER_PATH}")
PROVIDER_RESOLVER = importlib.util.module_from_spec(RESOLVER_SPEC)
RESOLVER_SPEC.loader.exec_module(PROVIDER_RESOLVER)


CURRENT_SCHEMA_VERSION = 5
SUPPORTED_SCHEMA_VERSIONS = frozenset(range(1, CURRENT_SCHEMA_VERSION + 1))
REVIEW_SELECTION_MIN_SCHEMA_VERSION = 5

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
EVIDENCE_COUNT_FIELDS = {
    "attempts", "successful", "failed", "timed_out", "infrastructure_failed",
}
EVIDENCE_MEASUREMENT_FIELDS = {
    "input_tokens", "cache_creation_tokens", "cache_read_tokens",
    "output_tokens", "thinking_tokens", "cost_usd", "wall_clock_seconds",
}
PROVIDER_FAILURE_REASONS = {
    "network", "timeout", "permission", "protocol", "quota", "other",
}
FALLBACK_REASONS = {
    "index_unavailable", "coverage_gap", "graph_ambiguity", "provider_failure",
}
CONTINUATION_CONTEXT_FIELDS = {
    "schema_version", "durable_decisions", "affected_boundaries",
    "unresolved_concerns", "deferred_scope",
}
CHECKPOINT_PROJECTION_FIELDS = {
    "schema_version", "checkpoint_id", "checkpoint_kind", "repository", "run_id",
    "slice_number", "candidate_attempt", "checkpoint_commit_oid", "checkpoint_tree_oid",
    "candidate_checkpoint_id", "parent_checkpoint_commit_oid", "task_patch_digest",
    "manifest_digest",
    "plan_version", "scope_revision",
    "gate_receipt_digest", "ref", "limitations",
}
COMPLETION_COMMIT_FIELDS = {
    "schema_version", "state", "repository", "run_id", "slice_number", "proposal",
    "proposal_digest", "expected_branch", "expected_head_oid", "commit_oid", "reason",
}
COMPLETION_PROPOSAL_COMMON_FIELDS = {
    "schema_version", "subject", "body", "paths", "checkpoint_commit_oid",
    "checkpoint_tree_oid", "task_patch_digest",
}
COMPLETION_PROVENANCE_FIELDS = {"schema_version", "producer", "evidence"}


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
    if record["schema_version"] not in SUPPORTED_SCHEMA_VERSIONS:
        supported = ", ".join(str(version) for version in sorted(SUPPORTED_SCHEMA_VERSIONS))
        fail(f"schema_version must be one of: {supported}")
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
    if record["schema_version"] >= 2:
        validate_engine_record(record)
    if record["schema_version"] >= 3:
        validate_placement_record(record)
    if record["schema_version"] >= 4:
        validate_evidence_provenance(record)
    if "continuation_context" in record:
        validate_continuation_context(record["continuation_context"])
    checkpoint = record["producer_metrics"].get("slice_checkpoint")
    if checkpoint is not None:
        validate_checkpoint_projection(checkpoint, record)
    completion_commit = record["producer_metrics"].get("slice_completion_commit")
    if completion_commit is not None:
        validate_completion_commit_projection(completion_commit, record, checkpoint)
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


def validate_checkpoint_projection(value: Any, record: dict[str, Any]) -> None:
    if not isinstance(value, dict):
        fail("producer_metrics.slice_checkpoint must be an object")
    require_exact_keys(value, CHECKPOINT_PROJECTION_FIELDS, "producer_metrics.slice_checkpoint")
    if value["schema_version"] != 1:
        fail("producer_metrics.slice_checkpoint.schema_version must be 1")
    if value["run_id"] != record["run_id"] or value["slice_number"] != record["slice_number"]:
        fail("slice checkpoint identity must match the terminal receipt")
    expected_kind = "accepted" if record["status"] == "accepted" else "stopped"
    if value["checkpoint_kind"] != expected_kind:
        fail(f"{record['status']} receipt requires a {expected_kind} checkpoint")
    for field in ("checkpoint_id", "task_patch_digest", "manifest_digest", "gate_receipt_digest"):
        digest = value[field]
        if not isinstance(digest, str) or len(digest) != 64 or any(
            character not in "0123456789abcdef" for character in digest
        ):
            fail(f"producer_metrics.slice_checkpoint.{field} must be lowercase SHA-256")
    for field in ("checkpoint_commit_oid", "checkpoint_tree_oid"):
        oid = value[field]
        if not isinstance(oid, str) or len(oid) not in {40, 64} or any(
            character not in "0123456789abcdef" for character in oid
        ):
            fail(f"producer_metrics.slice_checkpoint.{field} must be a Git object id")
    for field in (
        "repository", "run_id", "candidate_checkpoint_id", "plan_version",
        "scope_revision", "ref",
    ):
        if not isinstance(value[field], str) or not value[field].strip():
            fail(f"producer_metrics.slice_checkpoint.{field} must be nonempty")
    if not value["ref"].startswith("refs/work-engine/checkpoints/"):
        fail("producer_metrics.slice_checkpoint.ref must be a private Work Engine ref")
    if isinstance(value["candidate_attempt"], bool) or not isinstance(value["candidate_attempt"], int) or value["candidate_attempt"] < 1:
        fail("producer_metrics.slice_checkpoint.candidate_attempt must be positive")
    if not isinstance(value["limitations"], list) or not all(
        isinstance(item, str) for item in value["limitations"]
    ):
        fail("producer_metrics.slice_checkpoint.limitations must be an array of strings")


def validate_completion_commit_projection(value: Any, record: dict[str, Any], checkpoint: Any) -> None:
    if not isinstance(value, dict):
        fail("producer_metrics.slice_completion_commit must be an object")
    require_exact_keys(value, COMPLETION_COMMIT_FIELDS, "producer_metrics.slice_completion_commit")
    if value["schema_version"] != 1 or value["state"] not in {"pending", "declined", "created", "refused"}:
        fail("slice completion commit must use schema version 1 and a valid state")
    if record["status"] != "accepted":
        fail("slice completion commit requires an accepted terminal receipt")
    if value["run_id"] != record["run_id"] or value["slice_number"] != record["slice_number"]:
        fail("slice completion commit identity must match the terminal receipt")
    proposal = value["proposal"]
    if not isinstance(proposal, dict):
        fail("slice completion commit proposal must be an object")
    if proposal.get("schema_version") == 1:
        require_exact_keys(
            proposal, COMPLETION_PROPOSAL_COMMON_FIELDS | {"origin"},
            "historical slice completion commit proposal",
        )
        if proposal["origin"] != "completing_builder":
            fail("historical slice completion proposal requires completing_builder origin")
    elif proposal.get("schema_version") == 2:
        require_exact_keys(
            proposal, COMPLETION_PROPOSAL_COMMON_FIELDS | {"provenance"},
            "slice completion commit proposal",
        )
        validate_completion_provenance(proposal["provenance"])
    else:
        fail("slice completion proposal must use supported schema version 1 or 2")
    if checkpoint is None or proposal["checkpoint_commit_oid"] != checkpoint["checkpoint_commit_oid"] or proposal["checkpoint_tree_oid"] != checkpoint["checkpoint_tree_oid"] or proposal["task_patch_digest"] != checkpoint["task_patch_digest"]:
        fail("slice completion proposal must bind the accepted checkpoint")
    if hashlib.sha256(json.dumps(proposal, sort_keys=True, separators=(",", ":")).encode()).hexdigest() != value["proposal_digest"]:
        fail("slice completion proposal digest does not match")
    for field in ("subject", "checkpoint_commit_oid", "checkpoint_tree_oid", "task_patch_digest"):
        if not isinstance(proposal[field], str) or not proposal[field].strip():
            fail(f"slice completion proposal.{field} must be nonempty")
    if "\n" in proposal["subject"] or not isinstance(proposal["body"], str):
        fail("slice completion proposal message is invalid")
    if not isinstance(proposal["paths"], list) or not proposal["paths"] or not all(
        isinstance(path, str) and path and not path.startswith("/") for path in proposal["paths"]
    ) or len(set(proposal["paths"])) != len(proposal["paths"]):
        fail("slice completion proposal paths must be a unique nonempty array")
    for field in ("repository", "expected_branch", "expected_head_oid", "proposal_digest"):
        if not isinstance(value[field], str) or not value[field].strip():
            fail(f"slice completion commit.{field} must be nonempty")
    for field in ("expected_head_oid",):
        oid = value[field]
        if len(oid) not in {40, 64} or any(character not in "0123456789abcdef" for character in oid):
            fail(f"slice completion commit.{field} must be a Git object id")
    if value["state"] == "created":
        if not isinstance(value["commit_oid"], str) or not value["commit_oid"] or not isinstance(value["reason"], (str, type(None))):
            fail("created completion commit requires commit_oid and an optional reason")
        if len(value["commit_oid"]) not in {40, 64} or any(character not in "0123456789abcdef" for character in value["commit_oid"]):
            fail("created completion commit.commit_oid must be a Git object id")
    elif value["state"] == "pending":
        if value["commit_oid"] is not None or value["reason"] is not None:
            fail("pending completion commit cannot have commit_oid or reason")
    elif value["commit_oid"] is not None or not isinstance(value["reason"], str) or not value["reason"]:
        fail("declined/refused completion commit requires a reason and no commit_oid")


def validate_completion_provenance(value: Any) -> None:
    if not isinstance(value, dict):
        fail("slice completion proposal provenance must be an object")
    require_exact_keys(value, COMPLETION_PROVENANCE_FIELDS, "slice completion proposal provenance")
    if value["schema_version"] != 1:
        fail("slice completion proposal provenance schema_version must be 1")
    if not isinstance(value["producer"], str) or not value["producer"].strip():
        fail("slice completion proposal provenance producer must be nonempty")
    if not isinstance(value["evidence"], list) or not value["evidence"]:
        fail("slice completion proposal provenance evidence must be a nonempty array")
    for index, item in enumerate(value["evidence"]):
        if not isinstance(item, dict) or set(item) != {"kind", "digest"}:
            fail(f"slice completion proposal provenance evidence[{index}] is invalid")
        if not isinstance(item["kind"], str) or not item["kind"].strip():
            fail(f"slice completion proposal provenance evidence[{index}].kind must be nonempty")
        digest = item["digest"]
        if (not isinstance(digest, str) or len(digest) != 64
                or any(character not in "0123456789abcdef" for character in digest)):
            fail(f"slice completion proposal provenance evidence[{index}].digest must be lowercase SHA-256")


def validate_continuation_context(value: Any) -> None:
    if not isinstance(value, dict):
        fail("continuation_context must be an object")
    require_exact_keys(value, CONTINUATION_CONTEXT_FIELDS, "continuation_context")
    if value["schema_version"] != 1:
        fail("continuation_context.schema_version must be 1")
    for field in CONTINUATION_CONTEXT_FIELDS - {"schema_version"}:
        if not isinstance(value[field], list):
            fail(f"continuation_context.{field} must be an array")


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
        "stop_on", "capabilities", "slice_completion_commit", "explicit_fields", "defaulted_fields", "amendments",
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
    if isinstance(config["version"], bool) or config["version"] not in {1, 2}:
        fail("engine_config.version must be 1 or 2")
    completion_policy = config.get("slice_completion_commit")
    if completion_policy is not None and (
        not isinstance(completion_policy, dict)
        or set(completion_policy) != {"prompt"}
        or completion_policy["prompt"] not in {"enabled", "disabled"}
    ):
        fail("engine_config.slice_completion_commit must contain prompt enabled or disabled")
    capabilities = config.get("capabilities", {})
    if not isinstance(capabilities, dict):
        fail("engine_config.capabilities must be an object")
    unknown_capabilities = set(capabilities) - {"chrome_vision"}
    if unknown_capabilities:
        fail(f"unknown engine_config capabilities: {', '.join(sorted(unknown_capabilities))}")
    if "chrome_vision" in capabilities:
        chrome = capabilities["chrome_vision"]
        if not isinstance(chrome, dict):
            fail("engine_config.capabilities.chrome_vision must be an object")
        if chrome.get("source") not in {"file", "inline"}:
            fail("engine_config.capabilities.chrome_vision.source must be file or inline")
        common = {"source", "pathBase", "sha256", "schemaVersion"}
        allowed_chrome = common | ({"authoredReference", "resolvedPath"} if chrome.get("source") == "file" else {"campaignPath"})
        unknown_chrome = set(chrome) - allowed_chrome
        if unknown_chrome:
            fail(f"unknown chrome_vision provenance fields: {', '.join(sorted(unknown_chrome))}")
        for key in common - {"schemaVersion"}:
            if not isinstance(chrome.get(key), str) or not chrome[key]:
                fail(f"engine_config.capabilities.chrome_vision.{key} must be a nonempty string")
        if chrome.get("schemaVersion") != 1:
            fail("engine_config.capabilities.chrome_vision.schemaVersion must be 1")
        if len(chrome["sha256"]) != 64 or any(character not in "0123456789abcdef" for character in chrome["sha256"]):
            fail("engine_config.capabilities.chrome_vision.sha256 must be lowercase SHA-256")
        required_source = ("authoredReference", "resolvedPath") if chrome["source"] == "file" else ("campaignPath",)
        for key in required_source:
            if not isinstance(chrome.get(key), str) or not chrome[key]:
                fail(f"engine_config.capabilities.chrome_vision.{key} must be a nonempty string")
    for key in ("source", "objective"):
        if not config[key].strip():
            fail(f"engine_config.{key} must not be empty")

    if not isinstance(config["builder"].get("skill"), str) or not config["builder"]["skill"].strip():
        fail("engine_config.builder.skill must be a nonempty string")
    validate_builder_context(config["version"], config["builder"].get("context", {}))
    if not isinstance(config["validation"].get("profile"), str) or not config["validation"]["profile"].strip():
        fail("engine_config.validation.profile must be a nonempty string")
    requirements = config["validation"].get("requirements")
    if not isinstance(requirements, list) or not requirements or not all(isinstance(item, str) and item for item in requirements):
        fail("engine_config.validation.requirements must be a nonempty array of nonempty strings")
    if len(set(requirements)) != len(requirements):
        fail("engine_config.validation.requirements must not contain duplicates")
    resolved_context = PROVIDER_RESOLVER.resolve_builder_context(
        config["version"], config["builder"].get("context", {})
    )
    if "adversarial_review" in resolved_context and any(
        requirement in {"independent_review", "independent_adversarial_review"}
        for requirement in requirements
    ):
        fail("accepted same-model review cannot satisfy an independent-review requirement")
    amendment_fields = {
        "timestamp", "changed_fields", "prior_values", "new_values", "reason", "human_approval"
    }
    for index, amendment in enumerate(config["amendments"]):
        path = f"engine_config.amendments[{index}]"
        if not isinstance(amendment, dict):
            fail(f"{path} must be an object")
        require_exact_keys(amendment, amendment_fields, path)
        if not isinstance(amendment["timestamp"], str) or not amendment["timestamp"].strip():
            fail(f"{path}.timestamp must be a nonempty string")
        if not isinstance(amendment["changed_fields"], list) or not amendment["changed_fields"] or not all(
            isinstance(item, str) and item.strip() for item in amendment["changed_fields"]
        ):
            fail(f"{path}.changed_fields must contain nonempty strings")
        for field in ("prior_values", "new_values"):
            if not isinstance(amendment[field], dict):
                fail(f"{path}.{field} must be an object")
        for field in ("reason", "human_approval"):
            if not isinstance(amendment[field], str) or not amendment[field].strip():
                fail(f"{path}.{field} must be a nonempty string")
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


def validate_placement_record(record: dict[str, Any]) -> None:
    required = {
        "placement_certificate": (dict, type(None)),
        "placement_verdict": (str, type(None)),
        "placement_risk": (str, type(None)),
        "rejected_placement_alternatives": list,
        "vertical_semantic_test": (dict, str, type(None)),
        "vertical_semantic_test_passed": (bool, type(None)),
    }
    for key, expected in required.items():
        if key not in record or not isinstance(record[key], expected):
            names = " or ".join(kind.__name__ for kind in expected)
            fail(f"{key} must be {names}")

    if record["placement_verdict"] not in {None, "confirmed", "conflict", "unresolved"}:
        fail("placement_verdict must be confirmed, conflict, unresolved, or null")
    if record["placement_risk"] not in {None, "low", "medium", "high"}:
        fail("placement_risk must be low, medium, high, or null")

    if record["plan_acceptance"] != "not_reached":
        if not record["placement_certificate"]:
            fail("an accepted plan requires a placement_certificate")
        if record["placement_verdict"] != "confirmed":
            fail("an accepted plan requires a confirmed placement_verdict")
        if record["placement_risk"] is None:
            fail("an accepted plan requires placement_risk")
        if record["vertical_semantic_test"] is None:
            fail("an accepted plan requires a vertical_semantic_test")
    if record["status"] == "accepted" and record["vertical_semantic_test_passed"] is not True:
        fail("an accepted slice requires a passing vertical semantic test")


def require_nonnegative_integer(value: Any, path: str) -> None:
    if isinstance(value, bool) or not isinstance(value, int) or value < 0:
        fail(f"{path} must be a nonnegative integer")


def require_nullable_measurement(value: Any, path: str) -> None:
    if value is None:
        return
    if isinstance(value, bool) or not isinstance(value, (int, float)) or value < 0:
        fail(f"{path} must be a nonnegative number or null")


def require_exact_keys(value: dict[str, Any], expected: set[str], path: str) -> None:
    missing = expected - set(value)
    unknown = set(value) - expected
    if missing:
        fail(f"{path} missing keys: {', '.join(sorted(missing))}")
    if unknown:
        fail(f"{path} has unknown keys: {', '.join(sorted(unknown))}")


def validate_builder_context(version: int, context: Any) -> None:
    if not isinstance(context, dict):
        fail("engine_config.builder.context must be an object")
    try:
        PROVIDER_RESOLVER.resolve_builder_context(version, context)
    except PROVIDER_RESOLVER.ProviderResolutionError as error:
        fail(str(error))


def validate_review_selection(value: Any, record: dict[str, Any]) -> None:
    path = "worker_metrics.review_selection"
    if not isinstance(value, dict):
        fail(f"{path} must be an object")
    require_exact_keys(
        value,
        {"selection_owner", "state", "state_reason", "subject", "specialists"},
        path,
    )
    if value["selection_owner"] != "slice-supervisor":
        fail(f"{path}.selection_owner must be slice-supervisor")

    state = value["state"]
    if state not in {"not_reached", "undecided", "decided"}:
        fail(f"{path}.state must be decided, undecided, or not_reached")
    if record["status"] == "accepted" and state != "decided":
        fail(f"{path}.state must be decided for an accepted receipt")
    if state != "decided":
        if (
            not isinstance(value["state_reason"], str)
            or not value["state_reason"].strip()
        ):
            fail(f"{path}.state_reason must explain why selection was not decided")
    elif value["state_reason"] is not None:
        fail(f"{path}.state_reason must be null when selection was decided")

    if state == "not_reached":
        if value["subject"] is not None or value["specialists"] != []:
            fail(f"{path} not_reached state cannot contain a subject or specialists")
        return

    subject = value["subject"]
    if not isinstance(subject, dict):
        fail(f"{path}.subject must be an object")
    require_exact_keys(subject, {"revision", "references"}, f"{path}.subject")
    if not isinstance(subject["revision"], str) or not subject["revision"].strip():
        fail(f"{path}.subject.revision must be a nonempty string")
    references = subject["references"]
    if not isinstance(references, list) or not references or not all(
        isinstance(reference, str) and reference.strip() for reference in references
    ):
        fail(f"{path}.subject.references must contain nonempty strings")

    specialists = value["specialists"]
    if state == "undecided":
        if specialists != []:
            fail(f"{path} undecided state cannot contain specialist dispositions")
        return
    if not isinstance(specialists, list) or not specialists:
        fail(f"{path}.specialists must be a nonempty array")
    entry_fields = {
        "skill", "selection", "selection_reason", "execution", "applicability",
        "result_ref", "finding_ids", "unresolved_finding_ids",
    }
    seen: set[str] = set()
    for index, entry in enumerate(specialists):
        entry_path = f"{path}.specialists[{index}]"
        if not isinstance(entry, dict):
            fail(f"{entry_path} must be an object")
        require_exact_keys(entry, entry_fields, entry_path)
        for field in ("skill", "selection_reason"):
            if not isinstance(entry[field], str) or not entry[field].strip():
                fail(f"{entry_path}.{field} must be a nonempty string")
        if entry["skill"] in seen:
            fail(f"{path}.specialists must contain unique skills")
        seen.add(entry["skill"])
        for field in ("finding_ids", "unresolved_finding_ids"):
            identifiers = entry[field]
            if not isinstance(identifiers, list) or not all(
                isinstance(identifier, str) and identifier.strip()
                for identifier in identifiers
            ) or len(identifiers) != len(set(identifiers)):
                fail(f"{entry_path}.{field} must contain unique nonempty strings")
        if not set(entry["unresolved_finding_ids"]).issubset(entry["finding_ids"]):
            fail(f"{entry_path}.unresolved_finding_ids must be a subset of finding_ids")

        if entry["selection"] == "omitted":
            if (
                entry["execution"] != "not_run"
                or entry["applicability"] is not None
                or entry["result_ref"] is not None
                or entry["finding_ids"]
            ):
                fail(f"{entry_path} omitted selection must not contain review execution")
            continue
        if entry["selection"] != "selected":
            fail(f"{entry_path}.selection must be selected or omitted")
        if entry["execution"] == "completed":
            if entry["applicability"] not in {"applicable", "omitted"}:
                fail(f"{entry_path}.applicability must be applicable or omitted")
            if not isinstance(entry["result_ref"], str) or not entry["result_ref"].strip():
                fail(f"{entry_path}.result_ref must bind the completed result")
            if entry["applicability"] == "omitted" and entry["finding_ids"]:
                fail(f"{entry_path} omitted applicability cannot contain findings")
        elif entry["execution"] in {"failed", "unavailable"}:
            if entry["applicability"] is not None or entry["finding_ids"]:
                fail(f"{entry_path} incomplete execution cannot claim applicability or findings")
            if entry["result_ref"] is not None and (
                not isinstance(entry["result_ref"], str) or not entry["result_ref"].strip()
            ):
                fail(f"{entry_path}.result_ref must be a nonempty string or null")
        else:
            fail(f"{entry_path}.execution is invalid for a selected specialist")

    if "agent-instruction-review" not in seen:
        fail(f"{path} must disposition agent-instruction-review exactly once")


def validate_evidence_provenance(record: dict[str, Any]) -> None:
    metrics = record["worker_metrics"]
    required = {
        "workflow_route": str,
        "route_revisions": list,
        "validation_breadth": dict,
        "provider_successful_calls": int,
        "provider_failed_calls": int,
        "provider_timed_out_calls": int,
        "provider_infrastructure_failed_calls": int,
        "evidence_mode_metrics": dict,
        "provider_failure_reasons": dict,
        "fallback_reason_counts": dict,
        "fallbacks": list,
    }
    for key, expected in required.items():
        if key not in metrics or not isinstance(metrics[key], expected):
            fail(f"worker_metrics.{key} must be {expected.__name__}")
    if (
        record["schema_version"] >= REVIEW_SELECTION_MIN_SCHEMA_VERSION
        and "review_selection" not in metrics
    ):
        fail(
            f"schema version {REVIEW_SELECTION_MIN_SCHEMA_VERSION} requires "
            "worker_metrics.review_selection"
        )
    if "review_selection" in metrics:
        validate_review_selection(metrics["review_selection"], record)

    config_version = record.get("engine_config", {}).get("version")
    configured_roles = None
    if config_version in {1, 2}:
        try:
            configured_roles = PROVIDER_RESOLVER.resolve_builder_context(
                config_version,
                record["engine_config"]["builder"].get("context", {}),
            )
        except PROVIDER_RESOLVER.ProviderResolutionError as error:
            fail(f"worker_metrics role identity is invalid: {error}")
    review_role = (
        "adversarial_review"
        if configured_roles and "adversarial_review" in configured_roles
        else "independent_review"
    )
    review_identity_field = f"{review_role}_identity"
    identity_fields = {"repository_evidence_identity", review_identity_field}
    all_identity_fields = identity_fields | {
        "independent_review_identity", "adversarial_review_identity"
    }
    present_identities = all_identity_fields & set(metrics)
    if present_identities and present_identities != identity_fields:
        fail("worker_metrics role identities must be reported together and match the configured review role")
    if config_version == 2 and present_identities != identity_fields:
        fail("version 2 receipts require role-separated provider identities")
    for field in identity_fields:
        if field not in metrics:
            continue
        identity = metrics[field]
        if not isinstance(identity, dict):
            fail(f"worker_metrics.{field} must be an object")
        expected_fields = {"provider", "skill"}
        if field == "adversarial_review_identity":
            expected_fields |= {
                "model", "reasoning_effort", "evidence_class", "isolation",
                "builder_context_inherited", "model_relationship", "independence_claimed",
            }
        require_exact_keys(identity, expected_fields, f"worker_metrics.{field}")
        for name in expected_fields - {"builder_context_inherited", "independence_claimed"}:
            if not isinstance(identity[name], str) or not identity[name].strip():
                fail(f"worker_metrics.{field}.{name} must be a nonempty string")
        if field == "adversarial_review_identity":
            if identity["builder_context_inherited"] is not False:
                fail("worker_metrics.adversarial_review_identity.builder_context_inherited must be false")
            if identity["model_relationship"] != "same_model":
                fail("worker_metrics.adversarial_review_identity.model_relationship must be same_model")
            if identity["independence_claimed"] is not False:
                fail("worker_metrics.adversarial_review_identity.independence_claimed must be false")
    if present_identities == identity_fields:
        try:
            if config_version == 1:
                normalized = []
                for field in sorted(identity_fields):
                    identity = metrics[field]
                    normalized.append(
                        PROVIDER_RESOLVER.resolve_builder_context(
                            1,
                            {
                                "evidence_skill": identity["skill"],
                                "reconnaissance": {"provider": identity["provider"]},
                            },
                        )["repository_evidence"]
                    )
                if normalized[0] != normalized[1]:
                    fail("version 1 role identities must preserve one combined legacy role")
                if normalized[0] != configured_roles["repository_evidence"]:
                    fail("version 1 actual role identity must match configured legacy role")
            else:
                actual_review = metrics[review_identity_field]
                actual_context = {
                    "repository_evidence": metrics["repository_evidence_identity"],
                    review_role: {
                        key: actual_review[key]
                        for key in configured_roles[review_role]
                    },
                }
                actual_roles = PROVIDER_RESOLVER.resolve_builder_context(2, actual_context)
                for role in ("repository_evidence", review_role):
                    if actual_roles[role] != configured_roles[role]:
                        fail(f"actual {role} identity must match configured role")
        except PROVIDER_RESOLVER.ProviderResolutionError as error:
            fail(f"worker_metrics role identity is invalid: {error}")

    semantic_call_fields = {
        "evidence_recon_calls", "evidence_supplemental_calls", "review_gate_calls"
    }
    present_call_fields = semantic_call_fields & set(metrics)
    if config_version == 2 and present_call_fields != semantic_call_fields:
        fail("version 2 receipts require semantic evidence and review call counters")
    for field in present_call_fields:
        require_nonnegative_integer(metrics[field], f"worker_metrics.{field}")

    role_metrics = metrics.get("provider_role_metrics")
    if record.get("engine_config", {}).get("version") == 2 and not isinstance(role_metrics, dict):
        fail("version 2 receipts require worker_metrics.provider_role_metrics")
    if role_metrics is not None:
        if not isinstance(role_metrics, dict):
            fail("worker_metrics.provider_role_metrics must be an object")
        require_exact_keys(
            role_metrics,
            {"repository_evidence", review_role},
            "worker_metrics.provider_role_metrics",
        )
        role_fields = EVIDENCE_COUNT_FIELDS | EVIDENCE_MEASUREMENT_FIELDS
        for role, values in role_metrics.items():
            path = f"worker_metrics.provider_role_metrics.{role}"
            if not isinstance(values, dict):
                fail(f"{path} must be an object")
            require_exact_keys(values, role_fields, path)
            for field in EVIDENCE_COUNT_FIELDS:
                require_nonnegative_integer(values[field], f"{path}.{field}")
            outcomes = sum(values[field] for field in EVIDENCE_COUNT_FIELDS if field != "attempts")
            if outcomes != values["attempts"]:
                fail(f"{path} outcome counts must sum to attempts")
            for field in EVIDENCE_MEASUREMENT_FIELDS:
                require_nullable_measurement(values[field], f"{path}.{field}")

    if role_metrics is not None and present_call_fields == semantic_call_fields:
        repository_attempts = role_metrics["repository_evidence"]["attempts"]
        retrieval_stages = metrics["evidence_recon_calls"] + metrics["evidence_supplemental_calls"]
        if retrieval_stages > repository_attempts:
            fail("repository evidence stage calls cannot exceed repository provider attempts")
        review_attempts = role_metrics[review_role]["attempts"]
        if metrics["review_gate_calls"] > review_attempts:
            fail("review gate calls cannot exceed configured review provider attempts")

    provider_count_fields = {
        "provider_successful_calls", "provider_failed_calls",
        "provider_timed_out_calls", "provider_infrastructure_failed_calls",
    }
    for field in provider_count_fields:
        require_nonnegative_integer(metrics[field], f"worker_metrics.{field}")

    if role_metrics is not None:
        aggregate_fields = {
            "successful": "provider_successful_calls",
            "failed": "provider_failed_calls",
            "timed_out": "provider_timed_out_calls",
            "infrastructure_failed": "provider_infrastructure_failed_calls",
        }
        for role_field, aggregate_field in aggregate_fields.items():
            observed = sum(values[role_field] for values in role_metrics.values())
            if observed != metrics[aggregate_field]:
                fail(
                    f"worker_metrics.{aggregate_field} must equal provider-role {role_field} totals"
                )

    if not metrics["workflow_route"].strip():
        fail("worker_metrics.workflow_route must be a nonempty string")

    revision_fields = {
        "failed_premise", "stale_decisions", "preserved_evidence",
        "replacement_route", "reason",
    }
    for index, revision in enumerate(metrics["route_revisions"]):
        path = f"worker_metrics.route_revisions[{index}]"
        if not isinstance(revision, dict):
            fail(f"{path} must be an object")
        require_exact_keys(revision, revision_fields, path)
        for field in ("failed_premise", "reason"):
            if not isinstance(revision[field], str) or not revision[field].strip():
                fail(f"{path}.{field} must be a nonempty string")
        for field in ("stale_decisions", "preserved_evidence"):
            if not isinstance(revision[field], list) or not all(
                isinstance(item, str) and item.strip() for item in revision[field]
            ):
                fail(f"{path}.{field} must contain nonempty strings")
        if (
            not isinstance(revision["replacement_route"], str)
            or not revision["replacement_route"].strip()
        ):
            fail(f"{path}.replacement_route must be a nonempty string")

    breadth = metrics["validation_breadth"]
    breadth_fields = {"selected_stages", "omitted_optional_stages", "rationale"}
    require_exact_keys(
        breadth, breadth_fields, "worker_metrics.validation_breadth"
    )
    selected = breadth["selected_stages"]
    if not isinstance(selected, list) or not selected or not all(
        isinstance(stage, str) and stage.strip() for stage in selected
    ):
        fail("worker_metrics.validation_breadth.selected_stages must contain nonempty strings")
    if len(selected) != len(set(selected)):
        fail("worker_metrics.validation_breadth.selected_stages must not contain duplicates")
    if not isinstance(breadth["rationale"], str) or not breadth["rationale"].strip():
        fail("worker_metrics.validation_breadth.rationale must be a nonempty string")
    omitted = breadth["omitted_optional_stages"]
    if not isinstance(omitted, list):
        fail("worker_metrics.validation_breadth.omitted_optional_stages must be an array")
    omitted_names: list[str] = []
    for index, omission in enumerate(omitted):
        path = f"worker_metrics.validation_breadth.omitted_optional_stages[{index}]"
        if not isinstance(omission, dict):
            fail(f"{path} must be an object")
        require_exact_keys(omission, {"stage", "reason"}, path)
        for field in ("stage", "reason"):
            if not isinstance(omission[field], str) or not omission[field].strip():
                fail(f"{path}.{field} must be a nonempty string")
        omitted_names.append(omission["stage"])
    if len(omitted_names) != len(set(omitted_names)):
        fail("worker_metrics.validation_breadth omitted stages must not contain duplicates")
    overlap = set(selected) & set(omitted_names)
    if overlap:
        fail("worker_metrics.validation_breadth stages cannot be selected and omitted")

    failure_reasons = metrics["provider_failure_reasons"]
    require_exact_keys(
        failure_reasons, PROVIDER_FAILURE_REASONS,
        "worker_metrics.provider_failure_reasons",
    )
    for reason, count in failure_reasons.items():
        require_nonnegative_integer(
            count, f"worker_metrics.provider_failure_reasons.{reason}"
        )
    provider_failures = sum(
        metrics[field]
        for field in (
            "provider_failed_calls", "provider_timed_out_calls",
            "provider_infrastructure_failed_calls",
        )
    )
    if sum(failure_reasons.values()) != provider_failures:
        fail(
            "worker_metrics.provider_failure_reasons must classify every "
            "failed, timed-out, or infrastructure-failed provider call"
        )

    fallback_counts = metrics["fallback_reason_counts"]
    require_exact_keys(
        fallback_counts, FALLBACK_REASONS,
        "worker_metrics.fallback_reason_counts",
    )
    for reason, count in fallback_counts.items():
        require_nonnegative_integer(
            count, f"worker_metrics.fallback_reason_counts.{reason}"
        )

    mode_fields = EVIDENCE_COUNT_FIELDS | EVIDENCE_MEASUREMENT_FIELDS
    for mode, values in metrics["evidence_mode_metrics"].items():
        if not isinstance(mode, str) or not mode.strip():
            fail("worker_metrics.evidence_mode_metrics keys must be nonempty strings")
        if not isinstance(values, dict):
            fail(f"worker_metrics.evidence_mode_metrics.{mode} must be an object")
        path = f"worker_metrics.evidence_mode_metrics.{mode}"
        require_exact_keys(values, mode_fields, path)
        for field in EVIDENCE_COUNT_FIELDS:
            require_nonnegative_integer(values[field], f"{path}.{field}")
        outcomes = sum(
            values[field]
            for field in EVIDENCE_COUNT_FIELDS
            if field != "attempts"
        )
        if outcomes != values["attempts"]:
            fail(f"{path} outcome counts must sum to attempts")
        for field in EVIDENCE_MEASUREMENT_FIELDS:
            require_nullable_measurement(values[field], f"{path}.{field}")

    observed_fallback_counts = {reason: 0 for reason in FALLBACK_REASONS}
    failure_fallback_counts = {reason: 0 for reason in PROVIDER_FAILURE_REASONS}
    event_fields = {"from_mode", "to_mode", "stage", "reason", "failure_kind"}
    for index, event in enumerate(metrics["fallbacks"]):
        path = f"worker_metrics.fallbacks[{index}]"
        if not isinstance(event, dict):
            fail(f"{path} must be an object")
        require_exact_keys(event, event_fields, path)
        for field in ("from_mode", "to_mode", "stage"):
            if not isinstance(event[field], str) or not event[field].strip():
                fail(f"{path}.{field} must be a nonempty string")
        reason = event["reason"]
        if reason not in FALLBACK_REASONS:
            fail(f"{path}.reason is invalid")
        observed_fallback_counts[reason] += 1
        failure_kind = event["failure_kind"]
        if reason == "provider_failure":
            if failure_kind not in PROVIDER_FAILURE_REASONS:
                fail(f"{path}.failure_kind is required for provider_failure")
            failure_fallback_counts[failure_kind] += 1
        elif failure_kind is not None:
            fail(f"{path}.failure_kind must be null unless reason is provider_failure")

    if fallback_counts != observed_fallback_counts:
        fail("worker_metrics.fallback_reason_counts must match fallbacks")
    for reason, count in failure_fallback_counts.items():
        if count > failure_reasons[reason]:
            fail(
                "worker_metrics provider_failure fallback counts must not exceed "
                "provider_failure_reasons"
            )


def load_record(args: argparse.Namespace) -> dict[str, Any]:
    if args.record_json is not None:
        raw = args.record_json
    else:
        raw = sys.stdin.read()
    try:
        return validate(json.loads(raw))
    except json.JSONDecodeError as error:
        fail(f"invalid JSON: {error}")


def validate_current_write(record: dict[str, Any]) -> dict[str, Any]:
    validate(record)
    if record["schema_version"] != CURRENT_SCHEMA_VERSION:
        fail(f"new durable receipts require schema_version {CURRENT_SCHEMA_VERSION}")
    continuation = record.get("continuation_context")
    resumable = (
        record["status"] == "accepted"
        and record["more_in_scope_work_remains"] is True
    )
    named_campaign = "campaign_source" in record["producer_metrics"]
    if resumable and named_campaign and continuation is None:
        fail("continuation_context is required for a resumable named-campaign write")
    if continuation is not None and not named_campaign:
        fail("continuation_context is only valid for a named-campaign write")
    if not resumable and continuation is not None:
        fail("continuation_context must be omitted unless accepted work remains")
    completion = record["producer_metrics"].get("slice_completion_commit")
    if completion is not None and completion["state"] == "pending":
        fail("pending completion offers are live state and cannot enter new terminal receipts")
    return record


def fsync_parent_directory(path: Path) -> None:
    directory_fd = os.open(path.parent, os.O_RDONLY | getattr(os, "O_DIRECTORY", 0))
    try:
        os.fsync(directory_fd)
    finally:
        os.close(directory_fd)


def append(path: Path, record: dict[str, Any]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    line = json.dumps(record, ensure_ascii=False, separators=(",", ":")) + "\n"
    with path.open("a+", encoding="utf-8") as stream:
        fcntl.flock(stream.fileno(), fcntl.LOCK_EX)
        try:
            stream.seek(0)
            for line_number, existing_line in enumerate(stream, start=1):
                try:
                    existing = json.loads(existing_line)
                    existing_identity = (
                        existing["run_id"],
                        existing["slice_number"],
                    )
                except (json.JSONDecodeError, KeyError, TypeError) as error:
                    fail(f"invalid existing receipt at line {line_number}: {error}")
                if existing_identity == (record["run_id"], record["slice_number"]):
                    fail(
                        "terminal receipt already exists for "
                        f"run_id={record['run_id']} "
                        f"slice_number={record['slice_number']}"
                    )

            stream.seek(0, os.SEEK_END)
            stream.write(line)
            stream.flush()
            os.fsync(stream.fileno())
            fsync_parent_directory(path)
        finally:
            fcntl.flock(stream.fileno(), fcntl.LOCK_UN)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--path", required=True, type=Path)
    source = parser.add_mutually_exclusive_group(required=True)
    source.add_argument("--record-json")
    source.add_argument("--stdin", action="store_true")
    args = parser.parse_args()
    try:
        record = validate_current_write(load_record(args))
        append(args.path, record)
    except (OSError, ValueError) as error:
        print(f"append_metrics: {error}", file=sys.stderr)
        return 2
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
