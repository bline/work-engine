#!/usr/bin/env python3
"""Validate and durably append one work-engine JSONL receipt under a lock."""

from __future__ import annotations

import argparse
import fcntl
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
    if record["schema_version"] not in {1, 2, 3, 4}:
        fail("schema_version must be 1, 2, 3, or 4")
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
    if isinstance(config["version"], bool) or config["version"] not in {1, 2}:
        fail("engine_config.version must be 1 or 2")
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

    identity_fields = {"repository_evidence_identity", "independent_review_identity"}
    present_identities = identity_fields & set(metrics)
    if present_identities and present_identities != identity_fields:
        fail("worker_metrics role identities must be reported together")
    if record.get("engine_config", {}).get("version") == 2 and present_identities != identity_fields:
        fail("version 2 receipts require role-separated provider identities")
    for field in identity_fields:
        if field not in metrics:
            continue
        identity = metrics[field]
        if not isinstance(identity, dict):
            fail(f"worker_metrics.{field} must be an object")
        require_exact_keys(identity, {"provider", "skill"}, f"worker_metrics.{field}")
        for name, value in identity.items():
            if not isinstance(value, str) or not value.strip():
                fail(f"worker_metrics.{field}.{name} must be a nonempty string")
    config_version = record.get("engine_config", {}).get("version")
    if present_identities == identity_fields:
        try:
            configured_roles = PROVIDER_RESOLVER.resolve_builder_context(
                config_version,
                record["engine_config"]["builder"].get("context", {}),
            )
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
                actual_roles = PROVIDER_RESOLVER.resolve_builder_context(
                    2,
                    {
                        "repository_evidence": metrics["repository_evidence_identity"],
                        "independent_review": metrics["independent_review_identity"],
                    },
                )
                for role in ("repository_evidence", "independent_review"):
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
            {"repository_evidence", "independent_review"},
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
        review_attempts = role_metrics["independent_review"]["attempts"]
        if metrics["review_gate_calls"] > review_attempts:
            fail("review gate calls cannot exceed independent-review provider attempts")

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

    if metrics["workflow_route"] not in {"direct", "falsified-placement"}:
        fail("worker_metrics.workflow_route must be direct or falsified-placement")

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
        if revision["replacement_route"] not in {"direct", "falsified-placement"}:
            fail(f"{path}.replacement_route is invalid")

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
