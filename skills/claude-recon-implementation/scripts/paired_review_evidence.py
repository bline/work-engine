#!/usr/bin/env python3
"""Register, finalize, and audit realtime/batch Claude review pairs."""

from __future__ import annotations

import argparse
from datetime import datetime, timezone
import hashlib
import json
from pathlib import Path
import shutil
import tempfile
from typing import Any, Sequence

from claude_transport import TransportError, sha256_directory


CAMPAIGN_TYPE = "claude_review_pair_campaign_v1"
REGISTRATION_TYPE = "claude_review_pair_registration_v1"
RECEIPT_TYPE = "claude_review_pair_receipt_v1"


def now() -> str:
    return datetime.now(timezone.utc).isoformat().replace("+00:00", "Z")


def sha256_file(path: Path) -> str:
    if not path.is_file():
        raise TransportError(f"required artifact is not a file: {path}")
    return hashlib.sha256(path.read_bytes()).hexdigest()


def load_object(path: Path) -> dict[str, Any]:
    try:
        value = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as failure:
        raise TransportError(f"invalid JSON artifact {path}: {failure}") from failure
    if not isinstance(value, dict):
        raise TransportError(f"JSON artifact must be an object: {path}")
    return value


def atomic_json(path: Path, value: dict[str, Any]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    payload = json.dumps(value, indent=2, sort_keys=True, ensure_ascii=False) + "\n"
    with tempfile.NamedTemporaryFile(
        "w", encoding="utf-8", dir=path.parent, delete=False
    ) as handle:
        handle.write(payload)
        temporary = Path(handle.name)
    temporary.replace(path)


def campaign_path(root: Path) -> Path:
    return root / "campaign.json"


def pair_dir(root: Path, pair_id: str) -> Path:
    if not pair_id or any(character not in "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789-_" for character in pair_id):
        raise TransportError("pair ID may contain only letters, digits, '-' and '_'")
    return root / "pairs" / pair_id


def require_campaign(root: Path) -> dict[str, Any]:
    campaign = load_object(campaign_path(root))
    if campaign.get("artifact_type") != CAMPAIGN_TYPE or campaign.get("schema_version") != 1:
        raise TransportError("unsupported paired-review campaign contract")
    return campaign


def initialize(args: argparse.Namespace) -> int:
    path = campaign_path(args.campaign_root)
    if path.exists():
        raise TransportError(f"campaign already exists: {path}")
    if args.target_pairs <= 0:
        raise TransportError("--target-pairs must be positive")
    value = {
        "artifact_type": CAMPAIGN_TYPE,
        "schema_version": 1,
        "campaign_id": args.campaign_id,
        "created_at": now(),
        "target_pairs": args.target_pairs,
        "review_protocol": args.review_protocol,
        "result_contract": "review_bench_result_v2_semantic_fields",
        "harness": "claude-code",
        "gateway": "openrouter",
        "upstream_provider": "anthropic",
        "model": args.model,
        "arms": {
            "realtime": {
                "transport": "openrouter-realtime",
                "experimental_betas_disabled": False,
                "workflow_authority": "caller-facing-review",
            },
            "batch": {
                "transport": "openrouter-batch",
                "experimental_betas_disabled": True,
                "workflow_authority": "shadow-measurement-only",
            },
        },
        "interpretation": (
            "Practical deployable-configuration comparison; transport and the "
            "experimental-beta setting are intentionally bundled."
        ),
    }
    atomic_json(path, value)
    print(path)
    return 0


def copy_bound_artifact(source: Path, destination: Path) -> dict[str, Any]:
    digest = sha256_file(source)
    destination.parent.mkdir(parents=True, exist_ok=True)
    shutil.copyfile(source, destination)
    return {"path": str(destination), "sha256": digest, "source_path": str(source)}


def register(args: argparse.Namespace) -> int:
    campaign = require_campaign(args.campaign_root)
    directory = pair_dir(args.campaign_root, args.pair_id)
    registration_path = directory / "registration.json"
    if directory.exists():
        raise TransportError(f"pair already exists: {args.pair_id}")
    realtime_config_digest = sha256_directory(args.realtime_config_dir)
    batch_config_digest = sha256_directory(args.batch_config_dir)
    if args.realtime_config_dir.resolve() == args.batch_config_dir.resolve():
        raise TransportError("realtime and batch arms require separate config-directory clones")
    if realtime_config_digest != batch_config_digest:
        raise TransportError("realtime and batch config-directory clones are not identical")
    directory.mkdir(parents=True)
    inputs = directory / "inputs"
    subject = copy_bound_artifact(
        args.subject_artifact, inputs / f"subject{args.subject_artifact.suffix}"
    )
    packet = copy_bound_artifact(
        args.review_packet, inputs / f"review-packet{args.review_packet.suffix}"
    )
    config_manifest = copy_bound_artifact(
        args.config_manifest, inputs / f"claude-config-manifest{args.config_manifest.suffix}"
    )
    attestation = copy_bound_artifact(
        args.routing_attestation,
        inputs / f"routing-attestation{args.routing_attestation.suffix}",
    )
    registration = {
        "artifact_type": REGISTRATION_TYPE,
        "schema_version": 1,
        "campaign_id": campaign["campaign_id"],
        "pair_id": args.pair_id,
        "ordinal": args.ordinal,
        "registered_at": now(),
        "subject": {"identity": args.subject_identity, **subject},
        "review_packet": packet,
        "claude_config": {
            "manifest": config_manifest,
            "realtime_directory": str(args.realtime_config_dir),
            "batch_directory": str(args.batch_config_dir),
            "directory_sha256": realtime_config_digest,
        },
        "routing_attestation": attestation,
        "model": campaign["model"],
        "review_protocol": campaign["review_protocol"],
        "batch_shadow_registered_before_realtime": True,
        "realtime_result_must_not_be_an_input_to_batch": True,
    }
    if not isinstance(args.ordinal, int) or args.ordinal <= 0:
        raise TransportError("--ordinal must be positive")
    atomic_json(registration_path, registration)
    print(registration_path)
    return 0


def result_profile(path: Path) -> dict[str, Any]:
    try:
        value = load_object(path)
    except TransportError as failure:
        return {"available": False, "reason": str(failure)}
    fields = (
        "session_id", "subtype", "is_error", "duration_ms", "duration_api_ms",
        "num_turns", "total_cost_usd", "usage", "modelUsage", "model_usage",
    )
    return {
        "available": True,
        **{field: value[field] for field in fields if field in value},
    }


def semantic_profile(path: Path) -> dict[str, Any]:
    outer = load_object(path)
    payload = outer.get("result")
    if isinstance(payload, str):
        try:
            payload = json.loads(payload)
        except json.JSONDecodeError as failure:
            raise TransportError(f"Claude result is not structured JSON: {path}") from failure
    if not isinstance(payload, dict):
        raise TransportError(f"Claude result has no structured review payload: {path}")
    for field in ("findings", "verified_claims", "observations"):
        if not isinstance(payload.get(field), list):
            raise TransportError(f"Claude result field {field!r} is not an array: {path}")
    verdict = payload.get("verdict")
    if verdict not in {"accepted", "rejected", "blocked_unverified"}:
        raise TransportError(f"Claude result has an unsupported verdict: {path}")
    canonical = json.dumps(
        payload, sort_keys=True, separators=(",", ":"), ensure_ascii=False
    ).encode("utf-8")
    return {
        "sha256": hashlib.sha256(canonical).hexdigest(),
        "verdict": verdict,
        "finding_count": len(payload["findings"]),
        "verified_claim_count": len(payload["verified_claims"]),
        "observation_count": len(payload["observations"]),
    }


def validate_inputs(registration: dict[str, Any], errors: list[str]) -> None:
    for label, binding in (
        ("subject", registration["subject"]),
        ("review_packet", registration["review_packet"]),
        ("config_manifest", registration["claude_config"]["manifest"]),
        ("routing_attestation", registration["routing_attestation"]),
    ):
        path = Path(binding["path"])
        try:
            current = sha256_file(path)
        except TransportError as failure:
            errors.append(str(failure))
            continue
        if current != binding["sha256"]:
            errors.append(f"{label} changed after pair registration")


def validate_realtime(
    registration: dict[str, Any], result_path: Path, receipt: dict[str, Any],
    errors: list[str],
) -> None:
    request = receipt.get("request", {})
    attempts = receipt.get("attempts", [])
    attempt = attempts[0] if len(attempts) == 1 and isinstance(attempts[0], dict) else {}
    if receipt.get("result") != "success" or receipt.get("selected_transport") != "openrouter":
        errors.append("realtime arm did not complete successfully through OpenRouter")
    if request.get("openrouter_model") != registration["model"]:
        errors.append("realtime model does not match registration")
    if request.get("experimental_betas_disabled") is not False:
        errors.append("realtime arm did not record experimental betas enabled")
    if request.get("terminal_title_disabled") is not True:
        errors.append("realtime arm did not disable the non-review terminal-title call")
    shape = request.get("command_shape", [])
    if "--json-schema" not in shape or "--output-format" not in shape:
        errors.append("realtime arm did not record structured-output command flags")
    if request.get("claude_config_dir_sha256_before") != registration["claude_config"]["directory_sha256"]:
        errors.append("realtime initial Claude config does not match registration")
    routing = receipt.get("routing_attestation") or {}
    if routing.get("sha256") != registration["routing_attestation"]["sha256"]:
        errors.append("realtime routing attestation does not match registration")
    if attempt.get("requested_upstream_provider") != "anthropic":
        errors.append("realtime arm did not request the Anthropic upstream")
    if attempt.get("stdout_sha256") != sha256_file(result_path):
        errors.append("realtime result does not match its transport receipt")


def validate_batch(
    registration: dict[str, Any], result_path: Path, receipt: dict[str, Any],
    event_log: Path, errors: list[str],
) -> None:
    if receipt.get("artifact_type") != "claude_batch_review_execution_v1":
        errors.append("batch arm has an unsupported execution receipt")
    if receipt.get("status") != "success":
        errors.append("batch arm did not complete successfully")
    if receipt.get("model") != registration["model"]:
        errors.append("batch model does not match registration")
    if receipt.get("requested_upstream_provider") != "anthropic":
        errors.append("batch arm did not request the Anthropic upstream")
    if receipt.get("experimental_betas_disabled") is not True:
        errors.append("batch arm did not record experimental betas disabled")
    if receipt.get("terminal_title_disabled") is not True:
        errors.append("batch arm did not disable the non-review terminal-title call")
    shape = receipt.get("command_shape", [])
    if "--json-schema" not in shape or "--output-format" not in shape:
        errors.append("batch arm did not record structured-output command flags")
    if receipt.get("claude_config_dir_sha256_before") != registration["claude_config"]["directory_sha256"]:
        errors.append("batch initial Claude config does not match registration")
    routing = receipt.get("routing_attestation") or {}
    if routing.get("sha256") != registration["routing_attestation"]["sha256"]:
        errors.append("batch routing attestation does not match registration")
    output = receipt.get("output") or {}
    if output.get("stdout_sha256") != sha256_file(result_path):
        errors.append("batch result does not match its execution receipt")
    proxy = receipt.get("proxy") or {}
    if proxy.get("event_log_sha256") != sha256_file(event_log):
        errors.append("batch event log does not match its execution receipt")
    if not (receipt.get("batch_summary") or {}).get("terminal_lineage_complete"):
        errors.append("batch request-to-terminal lineage is incomplete")


def finalize(args: argparse.Namespace) -> int:
    campaign = require_campaign(args.campaign_root)
    directory = pair_dir(args.campaign_root, args.pair_id)
    registration = load_object(directory / "registration.json")
    realtime = load_object(args.realtime_receipt)
    batch = load_object(args.batch_receipt)
    errors: list[str] = []
    validate_inputs(registration, errors)
    validate_realtime(registration, args.realtime_result, realtime, errors)
    validate_batch(registration, args.batch_result, batch, args.batch_event_log, errors)
    realtime_command = (realtime.get("request") or {}).get("command_sha256")
    batch_command = batch.get("command_sha256")
    if not realtime_command or realtime_command != batch_command:
        errors.append("arms did not execute the exact same native Claude command")
    if realtime.get("claude_version") != batch.get("claude_version"):
        errors.append("arms used different Claude Code versions")
    semantic_profiles: dict[str, dict[str, Any] | None] = {}
    for arm, path in (("realtime", args.realtime_result), ("batch", args.batch_result)):
        try:
            semantic_profiles[arm] = semantic_profile(path)
        except TransportError as failure:
            semantic_profiles[arm] = None
            errors.append(str(failure))

    artifacts = directory / "artifacts"
    artifacts.mkdir(exist_ok=True)
    copies = {}
    for label, source in (
        ("realtime_result", args.realtime_result),
        ("realtime_receipt", args.realtime_receipt),
        ("batch_result", args.batch_result),
        ("batch_receipt", args.batch_receipt),
        ("batch_event_log", args.batch_event_log),
    ):
        destination = artifacts / f"{label}{source.suffix}"
        shutil.copyfile(source, destination)
        copies[label] = {"path": str(destination), "sha256": sha256_file(destination)}
    receipt = {
        "artifact_type": RECEIPT_TYPE,
        "schema_version": 1,
        "campaign_id": campaign["campaign_id"],
        "pair_id": args.pair_id,
        "ordinal": registration["ordinal"],
        "finalized_at": now(),
        "comparison_ready": not errors,
        "validation_errors": errors,
        "bindings": {
            "subject_sha256": registration["subject"]["sha256"],
            "review_packet_sha256": registration["review_packet"]["sha256"],
            "config_sha256": registration["claude_config"]["directory_sha256"],
            "routing_attestation_sha256": registration["routing_attestation"]["sha256"],
            "command_sha256": realtime_command,
            "model": registration["model"],
            "claude_version": realtime.get("claude_version"),
        },
        "arms": {
            "realtime": {
                "experimental_betas_disabled": False,
                "result_profile": result_profile(args.realtime_result),
                "semantic_profile": semantic_profiles["realtime"],
                "artifacts": {
                    "result": copies["realtime_result"],
                    "execution_receipt": copies["realtime_receipt"],
                },
            },
            "batch": {
                "experimental_betas_disabled": True,
                "result_profile": result_profile(args.batch_result),
                "semantic_profile": semantic_profiles["batch"],
                "actual_openrouter_usage": (
                    batch.get("batch_summary") or {}
                ).get("actual_openrouter_usage"),
                "batch_ids": (
                    batch.get("batch_summary") or {}
                ).get("submitted_batch_ids", []),
                "artifacts": {
                    "result": copies["batch_result"],
                    "execution_receipt": copies["batch_receipt"],
                    "event_log": copies["batch_event_log"],
                },
            },
        },
        "authority": {
            "realtime": "caller-facing-review",
            "batch": "shadow-measurement-only",
            "bench_result_is_not_production_approval": True,
        },
        "known_confound": "transport plus experimental-beta setting",
    }
    output = directory / "pair-receipt.json"
    atomic_json(output, receipt)
    print(output)
    return 0 if not errors else 1


def audit(args: argparse.Namespace) -> int:
    campaign = require_campaign(args.campaign_root)
    pairs_root = args.campaign_root / "pairs"
    registrations = sorted(pairs_root.glob("*/registration.json")) if pairs_root.exists() else []
    rows = []
    ordinals: list[int] = []
    for registration_path in registrations:
        registration = load_object(registration_path)
        receipt_path = registration_path.parent / "pair-receipt.json"
        if receipt_path.exists():
            receipt = load_object(receipt_path)
            ready = receipt.get("comparison_ready") is True
            errors = receipt.get("validation_errors", [])
            receipt_sha256 = sha256_file(receipt_path)
        else:
            ready = False
            errors = ["pair has not been finalized"]
            receipt_sha256 = None
        ordinal = registration.get("ordinal")
        if isinstance(ordinal, int):
            ordinals.append(ordinal)
        rows.append({
            "pair_id": registration.get("pair_id"), "ordinal": ordinal,
            "comparison_ready": ready, "errors": errors,
            "pair_receipt_sha256": receipt_sha256,
        })
    ready_count = sum(row["comparison_ready"] for row in rows)
    duplicate_ordinals = sorted({value for value in ordinals if ordinals.count(value) > 1})
    campaign_ready = (
        len(rows) == campaign["target_pairs"]
        and ready_count == campaign["target_pairs"]
        and not duplicate_ordinals
        and sorted(ordinals) == list(range(1, campaign["target_pairs"] + 1))
    )
    report = {
        "artifact_type": "claude_review_pair_campaign_audit_v1",
        "schema_version": 1,
        "campaign_id": campaign["campaign_id"],
        "audited_at": now(),
        "target_pairs": campaign["target_pairs"],
        "registered_pairs": len(rows),
        "comparison_ready_pairs": ready_count,
        "duplicate_ordinals": duplicate_ordinals,
        "campaign_ready_for_adjudication": campaign_ready,
        "pairs": rows,
        "interpretation": campaign["interpretation"],
    }
    output = args.output or (args.campaign_root / "audit.json")
    atomic_json(output, report)
    print(output)
    return 0 if campaign_ready else 1


def compare(args: argparse.Namespace) -> int:
    campaign = require_campaign(args.campaign_root)
    audit_path = args.campaign_root / "audit.json"
    if not audit_path.exists():
        raise TransportError("run campaign audit before comparison")
    audit_value = load_object(audit_path)
    if audit_value.get("campaign_ready_for_adjudication") is not True:
        raise TransportError("campaign is not complete enough to compare")
    audit_bindings = {
        row["pair_id"]: row.get("pair_receipt_sha256")
        for row in audit_value.get("pairs", []) if isinstance(row, dict)
    }
    rows = []
    for receipt_path in sorted((args.campaign_root / "pairs").glob("*/pair-receipt.json")):
        receipt = load_object(receipt_path)
        if audit_bindings.get(receipt.get("pair_id")) != sha256_file(receipt_path):
            raise TransportError("pair receipt changed after campaign audit; rerun audit")
        realtime = receipt["arms"]["realtime"]["semantic_profile"]
        batch = receipt["arms"]["batch"]["semantic_profile"]
        rows.append({
            "pair_id": receipt["pair_id"],
            "ordinal": receipt["ordinal"],
            "realtime": realtime,
            "batch": batch,
            "exact_semantic_payload_match": realtime["sha256"] == batch["sha256"],
            "verdict_match": realtime["verdict"] == batch["verdict"],
            "finding_count_delta_batch_minus_realtime": (
                batch["finding_count"] - realtime["finding_count"]
            ),
            "requires_adjudication": realtime["sha256"] != batch["sha256"],
        })
    report = {
        "artifact_type": "claude_review_pair_descriptive_comparison_v1",
        "schema_version": 1,
        "campaign_id": campaign["campaign_id"],
        "created_at": now(),
        "pair_count": len(rows),
        "exact_semantic_payload_matches": sum(
            row["exact_semantic_payload_match"] for row in rows
        ),
        "verdict_matches": sum(row["verdict_match"] for row in rows),
        "pairs_requiring_adjudication": sum(
            row["requires_adjudication"] for row in rows
        ),
        "pairs": sorted(rows, key=lambda row: row["ordinal"]),
        "limitations": [
            "Exact payload equality is descriptive and is not semantic equivalence.",
            "Finding counts are not precision, recall, severity calibration, or truth.",
            "Transport and the experimental-beta setting remain bundled.",
            "Human or executable adjudication is required before quality conclusions.",
        ],
    }
    output = args.output or (args.campaign_root / "comparison.json")
    atomic_json(output, report)
    print(output)
    return 0


def parser() -> argparse.ArgumentParser:
    root = argparse.ArgumentParser(description=__doc__)
    commands = root.add_subparsers(dest="command", required=True)
    init = commands.add_parser("init")
    init.add_argument("--campaign-root", required=True, type=Path)
    init.add_argument("--campaign-id", required=True)
    init.add_argument("--target-pairs", type=int, default=10)
    init.add_argument("--model", required=True)
    init.add_argument("--review-protocol", default="evidence-calibrated-review/1.2")
    init.set_defaults(handler=initialize)

    add = commands.add_parser("register")
    add.add_argument("--campaign-root", required=True, type=Path)
    add.add_argument("--pair-id", required=True)
    add.add_argument("--ordinal", required=True, type=int)
    add.add_argument("--subject-identity", required=True)
    add.add_argument("--subject-artifact", required=True, type=Path)
    add.add_argument("--review-packet", required=True, type=Path)
    add.add_argument("--config-manifest", required=True, type=Path)
    add.add_argument("--realtime-config-dir", required=True, type=Path)
    add.add_argument("--batch-config-dir", required=True, type=Path)
    add.add_argument("--routing-attestation", required=True, type=Path)
    add.set_defaults(handler=register)

    finish = commands.add_parser("finalize")
    finish.add_argument("--campaign-root", required=True, type=Path)
    finish.add_argument("--pair-id", required=True)
    finish.add_argument("--realtime-result", required=True, type=Path)
    finish.add_argument("--realtime-receipt", required=True, type=Path)
    finish.add_argument("--batch-result", required=True, type=Path)
    finish.add_argument("--batch-receipt", required=True, type=Path)
    finish.add_argument("--batch-event-log", required=True, type=Path)
    finish.set_defaults(handler=finalize)

    check = commands.add_parser("audit")
    check.add_argument("--campaign-root", required=True, type=Path)
    check.add_argument("--output", type=Path)
    check.set_defaults(handler=audit)

    comparison = commands.add_parser("compare")
    comparison.add_argument("--campaign-root", required=True, type=Path)
    comparison.add_argument("--output", type=Path)
    comparison.set_defaults(handler=compare)
    return root


def main(argv: Sequence[str] | None = None) -> int:
    arguments = parser().parse_args(argv)
    try:
        return arguments.handler(arguments)
    except (TransportError, OSError, KeyError, TypeError, ValueError) as failure:
        print(f"error: {failure}", file=__import__("sys").stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
