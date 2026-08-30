#!/usr/bin/env python3
"""Run native Claude Code with narrow, provenance-bearing quota failover."""

from __future__ import annotations

import argparse
from datetime import datetime, timezone
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess
import sys
import tempfile
import time
from typing import Any, Sequence


SCHEMA_VERSION = 1
DEFAULT_OPENROUTER_BASE_URL = "https://openrouter.ai/api"
DEFAULT_OPENROUTER_MODEL = "anthropic/claude-sonnet-5-20260630"
QUOTA_PHRASES = (
    "you've hit your limit",
    "you have hit your limit",
    "usage limit reached",
    "session limit reached",
    "session quota",
)
RATE_LIMIT_REASONS = (
    "rate limit",
    "rate_limit",
    "quota",
    "usage limit",
    "session limit",
    "capacity",
)


class TransportError(ValueError):
    """Raised when a requested transport cannot be run safely."""


def _read_routing_attestation(
    path: Path,
    *,
    expected_model: str,
    expected_key_hash: str,
    max_age_seconds: int,
) -> dict[str, Any]:
    try:
        value = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        raise TransportError(f"invalid routing attestation: {error}") from error
    if not isinstance(value, dict):
        raise TransportError("routing attestation must be a JSON object")
    required = {
        "schema_version",
        "kind",
        "observed_at",
        "guardrail_id",
        "key_hash",
        "allowed_providers",
        "allowed_models",
        "assignment_guardrail_id",
    }
    if set(value) != required:
        raise TransportError(
            "routing attestation fields must exactly match the version-1 contract"
        )
    if value["schema_version"] != 1 or value["kind"] != "openrouter-key-guardrail":
        raise TransportError("unsupported routing attestation contract")
    if value["allowed_providers"] != ["anthropic"]:
        raise TransportError("routing attestation must allow only provider 'anthropic'")
    if value["allowed_models"] != [expected_model]:
        raise TransportError("routing attestation must allow only the exact requested model")
    if not expected_key_hash or value["key_hash"] != expected_key_hash:
        raise TransportError("routing attestation does not bind the selected API key hash")
    if value["guardrail_id"] != value["assignment_guardrail_id"]:
        raise TransportError("routing attestation does not bind the key to the guardrail")
    try:
        observed_at = datetime.fromisoformat(value["observed_at"].replace("Z", "+00:00"))
    except (AttributeError, ValueError) as error:
        raise TransportError("routing attestation observed_at must be ISO-8601") from error
    if observed_at.tzinfo is None:
        raise TransportError("routing attestation observed_at must include a timezone")
    age = (datetime.now(timezone.utc) - observed_at.astimezone(timezone.utc)).total_seconds()
    if age < -60 or age > max_age_seconds:
        raise TransportError("routing attestation is outside the allowed freshness window")
    return {
        "path": str(path),
        "sha256": _sha256_bytes(path.read_bytes()),
        "kind": value["kind"],
        "observed_at": value["observed_at"],
        "guardrail_id": value["guardrail_id"],
        "key_hash": value["key_hash"],
        "allowed_providers": value["allowed_providers"],
        "allowed_models": value["allowed_models"],
    }


def _canonical_json(value: Any) -> bytes:
    return json.dumps(
        value, sort_keys=True, separators=(",", ":"), ensure_ascii=False
    ).encode("utf-8")


def _sha256_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def _sha256_text(value: str) -> str:
    return _sha256_bytes(value.encode("utf-8"))


def classify_quota_failure(returncode: int, stdout: str, stderr: str) -> str | None:
    """Return the narrow quota signature that permits failover, if any."""
    if returncode == 0:
        return None
    combined = f"{stdout}\n{stderr}".lower()
    for phrase in QUOTA_PHRASES:
        if phrase in combined:
            return phrase
    has_429 = re.search(r"(?:\bhttp\D{0,12})?\b429\b", combined) is not None
    if has_429:
        for reason in RATE_LIMIT_REASONS:
            if reason in combined:
                return f"429:{reason}"
    if "rate_limit_error" in combined:
        return "rate_limit_error"
    return None


def _has_flag(command: Sequence[str], *flags: str) -> bool:
    return any(token in flags for token in command)


def _session_reference(command: Sequence[str]) -> tuple[str | None, str | None]:
    for index, token in enumerate(command):
        if token in {"--resume", "-r"}:
            value = command[index + 1] if index + 1 < len(command) else None
            return "resume", value
        if token == "--session-id":
            value = command[index + 1] if index + 1 < len(command) else None
            return "new", value
    return None, None


def _requested_model(command: Sequence[str]) -> str | None:
    for index, token in enumerate(command):
        if token == "--model" and index + 1 < len(command):
            return command[index + 1]
        if token.startswith("--model="):
            return token.split("=", 1)[1]
    return None


def _observed_models(output: str) -> list[str]:
    """Extract model identifiers from Claude JSON without retaining response text."""
    documents: list[Any] = []
    try:
        documents.append(json.loads(output))
    except json.JSONDecodeError:
        for line in output.splitlines():
            try:
                documents.append(json.loads(line))
            except json.JSONDecodeError:
                continue

    models: set[str] = set()

    def visit(value: Any) -> None:
        if isinstance(value, dict):
            for key, child in value.items():
                if key in {"model", "model_name"} and isinstance(child, str):
                    models.add(child)
                elif key in {"modelUsage", "model_usage"} and isinstance(child, dict):
                    models.update(str(model) for model in child)
                visit(child)
        elif isinstance(value, list):
            for child in value:
                visit(child)

    for document in documents:
        visit(document)
    return sorted(models)


def _safe_command_shape(command: Sequence[str]) -> list[str]:
    """Expose executable and option names without copying prompts into receipts."""
    shape = [Path(command[0]).name]
    for token in command[1:]:
        if token.startswith("-"):
            shape.append(token.split("=", 1)[0])
    return shape


def _claude_version(executable: str, env: dict[str, str]) -> str | None:
    try:
        result = subprocess.run(
            [executable, "--version"],
            env=env,
            capture_output=True,
            text=True,
            timeout=10,
            check=False,
        )
    except (OSError, subprocess.SubprocessError):
        return None
    if result.returncode != 0:
        return None
    return result.stdout.strip() or result.stderr.strip() or None


def _anthropic_environment(source: dict[str, str]) -> dict[str, str]:
    env = dict(source)
    # Auto mode means "the ordinary native Claude route first" even if the
    # parent shell was previously used for an OpenRouter invocation.
    env.pop("ANTHROPIC_BASE_URL", None)
    env.pop("ANTHROPIC_AUTH_TOKEN", None)
    return env


def _openrouter_environment(
    source: dict[str, str], *, base_url: str, model: str
) -> dict[str, str]:
    key = source.get("OPENROUTER_API_KEY", "")
    if not key:
        raise TransportError("OPENROUTER_API_KEY is required for OpenRouter transport")
    env = dict(source)
    env["ANTHROPIC_BASE_URL"] = base_url
    env["ANTHROPIC_AUTH_TOKEN"] = key
    env["ANTHROPIC_API_KEY"] = ""
    env["ANTHROPIC_DEFAULT_SONNET_MODEL"] = model
    return env


def _run_attempt(
    command: Sequence[str],
    env: dict[str, str],
    *,
    transport: str,
    model: str | None,
    requested_upstream_provider: str | None = None,
) -> tuple[subprocess.CompletedProcess[str], dict[str, Any]]:
    started = time.monotonic()
    result = subprocess.run(
        list(command), env=env, capture_output=True, text=True, check=False
    )
    duration_ms = round((time.monotonic() - started) * 1000)
    quota_signature = classify_quota_failure(
        result.returncode, result.stdout, result.stderr
    )
    attempt = {
        "transport": transport,
        "harness": "claude-code",
        "gateway": "anthropic" if transport == "anthropic" else "openrouter",
        "requested_model": model,
        "requested_upstream_provider": requested_upstream_provider,
        "observed_models": _observed_models(result.stdout),
        "returncode": result.returncode,
        "duration_ms": duration_ms,
        "stdout_sha256": _sha256_text(result.stdout),
        "stderr_sha256": _sha256_text(result.stderr),
        "quota_signature": quota_signature,
    }
    return result, attempt


def _write_receipt(path: Path, receipt: dict[str, Any]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    payload = json.dumps(receipt, indent=2, sort_keys=True, ensure_ascii=False) + "\n"
    with tempfile.NamedTemporaryFile(
        "w", encoding="utf-8", dir=path.parent, delete=False
    ) as handle:
        handle.write(payload)
        temporary = Path(handle.name)
    temporary.replace(path)


def _base_receipt(args: argparse.Namespace, command: Sequence[str]) -> dict[str, Any]:
    session_mode, session_id = _session_reference(command)
    script_digest = _sha256_bytes(Path(__file__).read_bytes())
    return {
        "schema_version": SCHEMA_VERSION,
        "launcher": {
            "path": "skills/claude-recon-implementation/scripts/claude_transport.py",
            "sha256": script_digest,
        },
        "request": {
            "transport": args.transport,
            "continuity": args.continuity,
            "command_sha256": _sha256_bytes(_canonical_json(list(command))),
            "command_shape": _safe_command_shape(command),
            "openrouter_base_url": args.openrouter_base_url,
            "openrouter_model": args.openrouter_model,
            "batch_route_explicitly_allowed": args.allow_batch_route,
            "paid_failover_explicitly_allowed": args.allow_paid_failover,
            "anthropic_1p_required": args.require_anthropic_1p,
            "claude_config_dir": os.environ.get("CLAUDE_CONFIG_DIR"),
            "session_mode": session_mode,
            "session_id": session_id,
        },
        "attempts": [],
        "selected_transport": None,
        "failover": {
            "attempted": False,
            "allowed": False,
            "reason": None,
            "continuity_claim": None,
        },
        "upstream_provider_observed": False,
        "routing_attestation": None,
        "result": "pending",
    }


def _emit(result: subprocess.CompletedProcess[str]) -> None:
    sys.stdout.write(result.stdout)
    sys.stderr.write(result.stderr)


def run(args: argparse.Namespace) -> int:
    command = list(args.command)
    if command and command[0] == "--":
        command = command[1:]
    if not command:
        raise TransportError("a native Claude command is required after '--'")

    receipt = _base_receipt(args, command)
    anthropic_env = _anthropic_environment(os.environ)
    receipt["claude_version"] = _claude_version(command[0], anthropic_env)

    if args.openrouter_model.endswith(":batch") and not args.allow_batch_route:
        raise TransportError(
            "a :batch model requires --allow-batch-route after compatibility validation"
        )

    if args.require_anthropic_1p:
        if args.transport == "anthropic":
            raise TransportError(
                "--require-anthropic-1p applies only when OpenRouter is configured"
            )
        if args.routing_attestation is None:
            raise TransportError(
                "--require-anthropic-1p requires --routing-attestation"
            )
        receipt["routing_attestation"] = _read_routing_attestation(
            args.routing_attestation,
            expected_model=args.openrouter_model,
            expected_key_hash=args.openrouter_key_hash,
            max_age_seconds=args.routing_attestation_max_age_seconds,
        )

    if args.transport == "auto" and args.continuity == "disposable":
        if not _has_flag(command, "--no-session-persistence"):
            raise TransportError(
                "disposable auto failover requires --no-session-persistence"
            )

    if args.transport == "openrouter":
        env = _openrouter_environment(
            os.environ,
            base_url=args.openrouter_base_url,
            model=args.openrouter_model,
        )
        result, attempt = _run_attempt(
            command,
            env,
            transport="openrouter",
            model=args.openrouter_model,
            requested_upstream_provider=(
                "anthropic" if args.require_anthropic_1p else None
            ),
        )
        receipt["attempts"].append(attempt)
        receipt["selected_transport"] = "openrouter" if result.returncode == 0 else None
        receipt["result"] = "success" if result.returncode == 0 else "failed"
        _write_receipt(args.receipt, receipt)
        _emit(result)
        return result.returncode

    primary, primary_attempt = _run_attempt(
        command,
        anthropic_env,
        transport="anthropic",
        model=_requested_model(command),
    )
    receipt["attempts"].append(primary_attempt)
    if primary.returncode == 0 or args.transport == "anthropic":
        receipt["selected_transport"] = (
            "anthropic" if primary.returncode == 0 else None
        )
        receipt["result"] = "success" if primary.returncode == 0 else "failed"
        _write_receipt(args.receipt, receipt)
        _emit(primary)
        return primary.returncode

    quota_signature = primary_attempt["quota_signature"]
    if quota_signature is None:
        receipt["failover"]["reason"] = "primary failure was not a quota failure"
        receipt["result"] = "failed"
        _write_receipt(args.receipt, receipt)
        _emit(primary)
        return primary.returncode

    receipt["failover"]["reason"] = quota_signature
    session_mode, _ = _session_reference(command)
    if args.continuity == "retained":
        can_resume = session_mode == "resume" and args.allow_retained_resume_failover
        if not can_resume:
            receipt["failover"]["reason"] = (
                "retained review requires an explicit, verified --resume failover "
                "or truthful reviewer replacement"
            )
            receipt["failover"]["continuity_claim"] = "replacement_required"
            receipt["result"] = "failover_blocked"
            _write_receipt(args.receipt, receipt)
            _emit(primary)
            return primary.returncode
        receipt["failover"]["continuity_claim"] = (
            "same_local_claude_session_resumed_across_gateway"
        )
    else:
        receipt["failover"]["continuity_claim"] = "exact_disposable_request_replayed"

    if not args.allow_paid_failover:
        receipt["failover"]["reason"] = (
            "quota failure observed but paid failover was not explicitly allowed"
        )
        receipt["result"] = "paid_failover_not_authorized"
        _write_receipt(args.receipt, receipt)
        _emit(primary)
        return primary.returncode

    try:
        openrouter_env = _openrouter_environment(
            os.environ,
            base_url=args.openrouter_base_url,
            model=args.openrouter_model,
        )
    except TransportError as error:
        receipt["failover"]["reason"] = str(error)
        receipt["result"] = "failover_unavailable"
        _write_receipt(args.receipt, receipt)
        _emit(primary)
        return primary.returncode

    receipt["failover"]["attempted"] = True
    receipt["failover"]["allowed"] = True
    fallback, fallback_attempt = _run_attempt(
        command,
        openrouter_env,
        transport="openrouter",
        model=args.openrouter_model,
        requested_upstream_provider=(
            "anthropic" if args.require_anthropic_1p else None
        ),
    )
    receipt["attempts"].append(fallback_attempt)
    receipt["selected_transport"] = (
        "openrouter" if fallback.returncode == 0 else None
    )
    receipt["result"] = "success" if fallback.returncode == 0 else "failed"
    _write_receipt(args.receipt, receipt)
    _emit(fallback)
    return fallback.returncode


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        description="Run native Claude Code with bounded Anthropic/OpenRouter routing"
    )
    parser.add_argument(
        "--transport",
        choices=("auto", "anthropic", "openrouter"),
        default="auto",
    )
    parser.add_argument(
        "--continuity",
        choices=("disposable", "retained"),
        default="disposable",
    )
    parser.add_argument("--receipt", type=Path, required=True)
    parser.add_argument(
        "--openrouter-base-url",
        default=DEFAULT_OPENROUTER_BASE_URL,
    )
    parser.add_argument(
        "--openrouter-model",
        default=os.environ.get(
            "CLAUDE_REVIEW_OPENROUTER_MODEL", DEFAULT_OPENROUTER_MODEL
        ),
    )
    parser.add_argument(
        "--allow-retained-resume-failover",
        action="store_true",
        help="allow only after cross-gateway --resume has been externally verified",
    )
    parser.add_argument(
        "--allow-paid-failover",
        action="store_true",
        help="authorize an auto-mode quota retry through the paid OpenRouter route",
    )
    parser.add_argument(
        "--allow-batch-route",
        action="store_true",
        help="allow an exact :batch model after native-Claude compatibility validation",
    )
    parser.add_argument(
        "--require-anthropic-1p",
        action="store_true",
        help="require a fresh Anthropic-only OpenRouter key-guardrail attestation",
    )
    parser.add_argument("--routing-attestation", type=Path)
    parser.add_argument(
        "--routing-attestation-max-age-seconds", type=int, default=900
    )
    parser.add_argument(
        "--openrouter-key-hash",
        default=os.environ.get("OPENROUTER_API_KEY_HASH", ""),
        help="non-secret OpenRouter key hash returned by the management API",
    )
    parser.add_argument("command", nargs=argparse.REMAINDER)
    return parser


def main(argv: Sequence[str] | None = None) -> int:
    parser = build_parser()
    args = parser.parse_args(argv)
    try:
        return run(args)
    except TransportError as error:
        # Argument errors precede a safe provider entry. Avoid inventing a
        # partial receipt whose request fields may not have been established.
        parser.error(str(error))
    except OSError as error:
        parser.error(f"could not execute native Claude command: {error}")
    return 2


if __name__ == "__main__":
    raise SystemExit(main())
