#!/usr/bin/env python3
"""Return one OpenRouter realtime Claude review and background its batch shadow."""

from __future__ import annotations

import argparse
from datetime import datetime, timezone
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import time
from typing import Any, Sequence
from uuid import UUID

from claude_transport import TransportError


SCRIPT_DIR = Path(__file__).resolve().parent
TRANSPORT_SCRIPT = SCRIPT_DIR / "claude_transport.py"
BATCH_SCRIPT = SCRIPT_DIR / "claude_batch_review.py"
EVIDENCE_SCRIPT = SCRIPT_DIR / "paired_review_evidence.py"
TERMINAL_BATCH_STATES = {"success", "failed", "infrastructure_failed"}
FORBIDDEN_INITIAL_SESSION_FLAGS = {
    "--no-session-persistence", "--resume", "-r", "--continue", "-c",
    "--session-id",
}
FORBIDDEN_CALLER_CONTROL_FLAGS = {
    "--mcp-config", "--strict-mcp-config", "--tools", "--allowedTools",
    "--allowed-tools", "--disallowedTools", "--disallowed-tools",
    "--model", "--fallback-model",
}
PAIRED_REVIEW_TOOLS = "mcp__codebase-memory-mcp,Read,Glob,Grep"


def now() -> str:
    return datetime.now(timezone.utc).isoformat().replace("+00:00", "Z")


def sha256_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def canonical_json(value: Any) -> bytes:
    return json.dumps(
        value, sort_keys=True, separators=(",", ":"), ensure_ascii=False
    ).encode("utf-8")


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


def atomic_bytes(path: Path, value: bytes) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with tempfile.NamedTemporaryFile("wb", dir=path.parent, delete=False) as handle:
        handle.write(value)
        temporary = Path(handle.name)
    temporary.replace(path)


def pair_directory(root: Path, pair_id: str) -> Path:
    return root / "pairs" / pair_id


def canonicalize_path_arguments(args: argparse.Namespace) -> None:
    """Resolve every cross-process path before registration or cwd changes."""
    for field in (
        "campaign_root", "subject_artifact", "review_packet", "config_manifest",
        "realtime_config_dir", "batch_config_dir", "routing_attestation",
        "paired_mcp_config", "working_directory",
    ):
        setattr(args, field, getattr(args, field).resolve())


def require_active_campaign(campaign_root: Path) -> None:
    registry_path = campaign_root.parent / "active-campaign.json"
    if not registry_path.exists():
        return
    registry = load_object(registry_path)
    selected = Path(registry.get("active_campaign_root", "")).resolve()
    if selected != campaign_root or registry.get("status") != "active":
        raise TransportError(
            "paired-review campaign is not active; inspect "
            f"{registry_path} before registering another pair"
        )


def write_campaign_progress(campaign_root: Path, trigger: str) -> dict[str, Any]:
    campaign = load_object(campaign_root / "campaign.json")
    pairs_root = campaign_root / "pairs"
    registrations = list(pairs_root.glob("*/registration.json")) if pairs_root.exists() else []
    ready = sum(
        load_object(path).get("comparison_ready") is True
        for path in pairs_root.glob("*/pair-receipt.json")
    ) if pairs_root.exists() else 0
    report = {
        "artifact_type": "claude_review_pair_campaign_progress_v1",
        "schema_version": 1,
        "campaign_id": campaign["campaign_id"],
        "observed_at": now(),
        "trigger": trigger,
        "target_pairs": campaign["target_pairs"],
        "registered_pairs": len(registrations),
        "comparison_ready_pairs": ready,
        "campaign_ready_for_adjudication": (
            len(registrations) == campaign["target_pairs"]
            and ready == campaign["target_pairs"]
        ),
    }
    atomic_json(campaign_root / "progress.json", report)
    return report


def progress_warning(progress: dict[str, Any]) -> str:
    return (
        "paired calibration progress: "
        f"{progress['registered_pairs']}/{progress['target_pairs']} registered; "
        f"{progress['comparison_ready_pairs']}/{progress['target_pairs']} comparison-ready; "
        "adjudication remains blocked until the campaign audit passes\n"
    )


def canonical_session_id(value: str | None) -> str:
    if value is None:
        raise TransportError("--reviewer-session-id is required")
    try:
        parsed = UUID(value)
    except (ValueError, AttributeError) as failure:
        raise TransportError("--reviewer-session-id must be a valid UUID") from failure
    if str(parsed) != value.lower():
        raise TransportError("--reviewer-session-id must use canonical UUID syntax")
    return str(parsed)


def prepare_initial_command(
    command: list[str], session_id: str, paired_mcp_config: Path
) -> list[str]:
    if not command:
        raise TransportError("a native Claude command is required after '--'")
    forbidden = [
        token for token in command
        if token.split("=", 1)[0] in FORBIDDEN_INITIAL_SESSION_FLAGS
    ]
    if forbidden:
        raise TransportError(
            "initial retained paired review cannot use caller-supplied session, "
            "resume, continue, or no-session-persistence flags"
        )
    caller_tool_flags = [
        token for token in command
        if token.split("=", 1)[0] in FORBIDDEN_CALLER_CONTROL_FLAGS
    ]
    if caller_tool_flags:
        raise TransportError(
            "paired review model, tool, and MCP isolation is controller-owned; "
            "remove caller-supplied control flags"
        )
    prepared = list(command)
    prepared[1:1] = [
        "--session-id", session_id,
        "--strict-mcp-config",
        "--mcp-config", str(paired_mcp_config),
        "--tools", PAIRED_REVIEW_TOOLS,
        "--model", "sonnet",
    ]
    return prepared


def result_session_id(stdout: bytes) -> str | None:
    try:
        value = json.loads(stdout.decode("utf-8"))
    except (UnicodeDecodeError, json.JSONDecodeError):
        return None
    if not isinstance(value, dict):
        return None
    session_id = value.get("session_id")
    return session_id if isinstance(session_id, str) else None


def register_pair(args: argparse.Namespace) -> dict[str, Any]:
    command = [
        sys.executable, str(EVIDENCE_SCRIPT), "register",
        "--campaign-root", str(args.campaign_root),
        "--pair-id", args.pair_id,
        "--ordinal", str(args.ordinal),
        "--subject-identity", args.subject_identity,
        "--subject-artifact", str(args.subject_artifact),
        "--review-packet", str(args.review_packet),
        "--config-manifest", str(args.config_manifest),
        "--realtime-config-dir", str(args.realtime_config_dir),
        "--batch-config-dir", str(args.batch_config_dir),
        "--routing-attestation", str(args.routing_attestation),
        "--reviewer-session-id", args.reviewer_session_id,
        "--paired-mcp-config", str(args.paired_mcp_config),
    ]
    result = subprocess.run(command, capture_output=True, text=True, check=False)
    if result.returncode != 0:
        raise TransportError(
            result.stderr.strip() or "paired-review registration failed"
        )
    return load_object(pair_directory(args.campaign_root, args.pair_id) / "registration.json")


def launch_process(
    command: list[str], *, env: dict[str, str], log_path: Path
) -> subprocess.Popen[bytes]:
    log_path.parent.mkdir(parents=True, exist_ok=True)
    log = log_path.open("ab", buffering=0)
    try:
        process = subprocess.Popen(
            command, env=env, stdout=log, stderr=subprocess.STDOUT,
            start_new_session=True, close_fds=True,
        )
    finally:
        log.close()
    return process


def runtime_paths(args: argparse.Namespace) -> dict[str, Path]:
    runtime = pair_directory(args.campaign_root, args.pair_id) / "runtime"
    return {
        "runtime": runtime,
        "controller_receipt": runtime / "controller-receipt.json",
        "realtime_result": runtime / "realtime-result.json",
        "realtime_stderr": runtime / "realtime-stderr.txt",
        "realtime_receipt": runtime / "realtime-transport-receipt.json",
        "batch_result": runtime / "batch-result.json",
        "batch_stderr": runtime / "batch-stderr.txt",
        "batch_receipt": runtime / "batch-execution-receipt.json",
        "batch_events": runtime / "batch-events.jsonl",
        "batch_worker_log": runtime / "batch-worker.log",
        "finalizer_log": runtime / "finalizer.log",
        "finalizer_receipt": runtime / "finalizer-receipt.json",
        "reviewer_handoff": runtime / "realtime-reviewer-handoff.json",
    }


def finalizer_command(args: argparse.Namespace, paths: dict[str, Path]) -> list[str]:
    return [
        sys.executable, str(Path(__file__).resolve()), "finalize-worker",
        "--campaign-root", str(args.campaign_root),
        "--pair-id", args.pair_id,
        "--realtime-result", str(paths["realtime_result"]),
        "--realtime-receipt", str(paths["realtime_receipt"]),
        "--batch-result", str(paths["batch_result"]),
        "--batch-receipt", str(paths["batch_receipt"]),
        "--batch-event-log", str(paths["batch_events"]),
        "--controller-receipt", str(paths["controller_receipt"]),
        "--receipt", str(paths["finalizer_receipt"]),
        "--timeout-seconds", str(args.finalizer_timeout_seconds),
        "--poll-seconds", str(args.finalizer_poll_seconds),
    ]


def run_review(args: argparse.Namespace) -> int:
    command = list(args.claude_command)
    if command and command[0] == "--":
        command = command[1:]
    canonicalize_path_arguments(args)
    if not args.working_directory.is_dir():
        raise TransportError("--working-directory must be an existing directory")
    require_active_campaign(args.campaign_root)
    args.reviewer_session_id = canonical_session_id(args.reviewer_session_id)
    if not command:
        raise TransportError("a native Claude command is required after '--'")
    if any(
        token.split("=", 1)[0] in FORBIDDEN_INITIAL_SESSION_FLAGS
        for token in command
    ):
        raise TransportError(
            "initial retained paired review cannot use caller-supplied session, "
            "resume, continue, or no-session-persistence flags"
        )
    if any(
        token.split("=", 1)[0] in FORBIDDEN_CALLER_CONTROL_FLAGS
        for token in command
    ):
        raise TransportError(
            "paired review model, tool, and MCP isolation is controller-owned; "
            "remove caller-supplied control flags"
        )

    registration = register_pair(args)
    progress = write_campaign_progress(args.campaign_root, f"registered:{args.pair_id}")
    sys.stderr.write(progress_warning(progress))
    sys.stderr.flush()
    command = prepare_initial_command(
        command,
        args.reviewer_session_id,
        Path(registration["paired_mcp_config"]["path"]),
    )
    packet_path = Path(registration["review_packet"]["path"])
    packet_bytes = packet_path.read_bytes()
    try:
        packet = packet_bytes.decode("utf-8")
    except UnicodeDecodeError as failure:
        raise TransportError("registered review packet must be UTF-8") from failure
    command.append(packet)
    command_digest = sha256_bytes(canonical_json(command))
    paths = runtime_paths(args)
    paths["runtime"].mkdir(parents=True, exist_ok=True)
    copied_attestation = Path(registration["routing_attestation"]["path"])

    controller = {
        "artifact_type": "claude_paired_review_controller_v2",
        "schema_version": 2,
        "campaign_id": registration["campaign_id"],
        "pair_id": args.pair_id,
        "ordinal": args.ordinal,
        "registered_at": registration["registered_at"],
        "status": "registered",
        "command_sha256": command_digest,
        "review_packet_sha256": registration["review_packet"]["sha256"],
        "working_directory": str(args.working_directory.resolve()),
        "reviewer_continuity": {
            "mode": "retained_native_claude_session",
            "expected_session_id": args.reviewer_session_id,
            "handoff": str(paths["reviewer_handoff"]),
        },
        "review_state_isolation": {
            "mode": "strict_mcp_read_only_pair",
            "mutable_production_review_state_available": False,
            "paired_mcp_config_sha256": registration["paired_mcp_config"]["sha256"],
            "post_review_bookkeeping": "required_outside_paired_inference",
        },
        "realtime": {"status": "pending", "returncode": None},
        "batch": {"status": "pending", "pid": None},
        "finalizer": {"status": "pending", "pid": None},
    }
    atomic_json(paths["controller_receipt"], controller)
    handoff = {
        "artifact_type": "native_claude_reviewer_handoff_v1",
        "schema_version": 1,
        "campaign_id": registration["campaign_id"],
        "pair_id": args.pair_id,
        "status": "pending_initial_review",
        "session_id": args.reviewer_session_id,
        "realtime_config_dir": str(args.realtime_config_dir.resolve()),
        "working_directory": str(args.working_directory.resolve()),
        "transport": "openrouter-realtime",
        "resume_requirement": "same config directory and --resume session UUID",
        "replacement_authorized_by_this_artifact": False,
        "production_review_state": "pending_external_same_session_bookkeeping",
        "updated_at": now(),
    }
    atomic_json(paths["reviewer_handoff"], handoff)

    batch_command = [
        sys.executable, str(BATCH_SCRIPT),
        "--receipt", str(paths["batch_receipt"]),
        "--event-log", str(paths["batch_events"]),
        "--stdout", str(paths["batch_result"]),
        "--stderr", str(paths["batch_stderr"]),
        "--claude-config-dir", str(args.batch_config_dir),
        "--working-directory", str(args.working_directory),
        "--batch-model", registration["model"],
        "--routing-attestation", str(copied_attestation),
        "--attestation-max-age-seconds", str(args.attestation_max_age_seconds),
        "--poll-interval-seconds", str(args.batch_poll_interval_seconds),
        "--poll-timeout-seconds", str(args.batch_poll_timeout_seconds),
        "--collection-window-seconds", str(args.collection_window_seconds),
        "--", *command,
    ]
    batch_env = dict(os.environ)
    batch = launch_process(
        batch_command, env=batch_env, log_path=paths["batch_worker_log"]
    )
    controller["batch"] = {
        "status": "launched", "pid": batch.pid, "launched_at": now(),
        "worker_sha256": sha256_bytes(BATCH_SCRIPT.read_bytes()),
    }
    atomic_json(paths["controller_receipt"], controller)

    finalizer_env = dict(os.environ)
    for name in (
        "OPENROUTER_API_KEY", "OPENROUTER_MANAGEMENT_KEY",
        "ANTHROPIC_AUTH_TOKEN", "ANTHROPIC_API_KEY",
    ):
        finalizer_env.pop(name, None)
    try:
        finalizer = launch_process(
            finalizer_command(args, paths), env=finalizer_env,
            log_path=paths["finalizer_log"],
        )
    except OSError as failure:
        controller["finalizer"] = {
            "status": "launch_failed",
            "failed_at": now(),
            "error_type": type(failure).__name__,
            "manual_finalization_required": True,
        }
        controller["status"] = "batch_launched_finalizer_failed"
    else:
        controller["finalizer"] = {
            "status": "launched", "pid": finalizer.pid, "launched_at": now(),
        }
        controller["status"] = "background_launched"
    atomic_json(paths["controller_receipt"], controller)

    realtime_env = dict(os.environ)
    realtime_env["CLAUDE_CONFIG_DIR"] = str(args.realtime_config_dir)
    realtime_env["CLAUDE_CODE_DISABLE_TERMINAL_TITLE"] = "1"
    realtime_env.pop("CLAUDE_CODE_DISABLE_EXPERIMENTAL_BETAS", None)
    realtime_command = [
        sys.executable, str(TRANSPORT_SCRIPT),
        "--transport", "openrouter",
        "--continuity", "retained",
        "--receipt", str(paths["realtime_receipt"]),
        "--openrouter-model", registration["model"],
        "--require-anthropic-1p",
        "--routing-attestation", str(copied_attestation),
        "--routing-attestation-max-age-seconds", str(args.attestation_max_age_seconds),
        "--openrouter-key-hash", os.environ.get("OPENROUTER_API_KEY_HASH", ""),
        "--", *command,
    ]
    started = time.monotonic()
    result = subprocess.run(
        realtime_command, env=realtime_env, cwd=args.working_directory,
        capture_output=True, check=False,
    )
    atomic_bytes(paths["realtime_result"], result.stdout)
    observed_session_id = result_session_id(result.stdout)
    continuity_verified = (
        result.returncode == 0 and observed_session_id == args.reviewer_session_id
    )
    continuity_error = b""
    if result.returncode == 0 and not continuity_verified:
        continuity_error = (
            "error: realtime Claude result did not return the registered reviewer "
            "session ID\n"
        ).encode("utf-8")
    reported_stderr = result.stderr + continuity_error
    atomic_bytes(paths["realtime_stderr"], reported_stderr)
    caller_returncode = result.returncode if result.returncode != 0 else (
        0 if continuity_verified else 2
    )
    controller["realtime"] = {
        "status": "success" if caller_returncode == 0 else "failed",
        "provider_returncode": result.returncode,
        "returncode": caller_returncode,
        "completed_at": now(),
        "duration_ms": round((time.monotonic() - started) * 1000),
        "stdout_sha256": sha256_bytes(result.stdout),
        "stderr_sha256": sha256_bytes(reported_stderr),
        "expected_session_id": args.reviewer_session_id,
        "observed_session_id": observed_session_id,
        "continuity_verified": continuity_verified,
    }
    controller["status"] = (
        "realtime_returned_state_bookkeeping_pending_batch_pending"
        if caller_returncode == 0 else "realtime_failed_batch_pending"
    )
    atomic_json(paths["controller_receipt"], controller)
    handoff.update({
        "status": "resume_ready" if continuity_verified else "not_resume_ready",
        "observed_session_id": observed_session_id,
        "initial_review_returncode": caller_returncode,
        "continuity_verified": continuity_verified,
        "updated_at": now(),
    })
    atomic_json(paths["reviewer_handoff"], handoff)
    sys.stdout.buffer.write(result.stdout)
    sys.stderr.buffer.write(reported_stderr)
    return caller_returncode


def finalize_worker(args: argparse.Namespace) -> int:
    receipt = {
        "artifact_type": "claude_paired_review_finalizer_v2",
        "schema_version": 2,
        "pair_id": args.pair_id,
        "status": "waiting",
        "started_at": now(),
        "completed_at": None,
        "finalize_returncode": None,
    }
    atomic_json(args.receipt, receipt)
    deadline = time.monotonic() + args.timeout_seconds
    while time.monotonic() < deadline:
        artifacts_ready = all(path.exists() for path in (
            args.realtime_result, args.realtime_receipt, args.batch_result,
            args.batch_receipt, args.batch_event_log, args.controller_receipt,
        ))
        if artifacts_ready:
            try:
                realtime = load_object(args.realtime_receipt)
                batch = load_object(args.batch_receipt)
                controller = load_object(args.controller_receipt)
            except TransportError:
                pass
            else:
                if (realtime.get("result") != "pending"
                        and batch.get("status") in TERMINAL_BATCH_STATES
                        and (controller.get("realtime") or {}).get("status")
                        in {"success", "failed"}):
                    break
        time.sleep(args.poll_seconds)
    else:
        receipt["status"] = "timed_out"
        receipt["completed_at"] = now()
        atomic_json(args.receipt, receipt)
        return 1

    command = [
        sys.executable, str(EVIDENCE_SCRIPT), "finalize",
        "--campaign-root", str(args.campaign_root),
        "--pair-id", args.pair_id,
        "--realtime-result", str(args.realtime_result),
        "--realtime-receipt", str(args.realtime_receipt),
        "--batch-result", str(args.batch_result),
        "--batch-receipt", str(args.batch_receipt),
        "--batch-event-log", str(args.batch_event_log),
        "--controller-receipt", str(args.controller_receipt),
    ]
    result = subprocess.run(command, capture_output=True, text=True, check=False)
    receipt["status"] = "finalized" if result.returncode in {0, 1} else "failed"
    receipt["comparison_ready"] = result.returncode == 0
    receipt["finalize_returncode"] = result.returncode
    receipt["stdout_sha256"] = sha256_bytes(result.stdout.encode("utf-8"))
    receipt["stderr_sha256"] = sha256_bytes(result.stderr.encode("utf-8"))
    receipt["completed_at"] = now()
    atomic_json(args.receipt, receipt)
    write_campaign_progress(args.campaign_root, f"finalized:{args.pair_id}")
    return 0 if result.returncode in {0, 1} else 1


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest="mode", required=True)
    run = commands.add_parser("run")
    run.add_argument("--campaign-root", required=True, type=Path)
    run.add_argument("--pair-id", required=True)
    run.add_argument("--ordinal", required=True, type=int)
    run.add_argument("--subject-identity", required=True)
    run.add_argument("--subject-artifact", required=True, type=Path)
    run.add_argument("--review-packet", required=True, type=Path)
    run.add_argument("--config-manifest", required=True, type=Path)
    run.add_argument("--realtime-config-dir", required=True, type=Path)
    run.add_argument("--batch-config-dir", required=True, type=Path)
    run.add_argument("--routing-attestation", required=True, type=Path)
    run.add_argument("--paired-mcp-config", required=True, type=Path)
    run.add_argument("--working-directory", required=True, type=Path)
    run.add_argument(
        "--reviewer-session-id",
        required=True,
        help="pre-registered canonical UUID for the retained realtime reviewer",
    )
    run.add_argument("--attestation-max-age-seconds", type=int, default=900)
    run.add_argument("--batch-poll-interval-seconds", type=float, default=2.0)
    run.add_argument("--batch-poll-timeout-seconds", type=float, default=3600.0)
    run.add_argument("--collection-window-seconds", type=float, default=0.5)
    run.add_argument("--finalizer-timeout-seconds", type=float, default=7200.0)
    run.add_argument("--finalizer-poll-seconds", type=float, default=5.0)
    run.add_argument("claude_command", nargs=argparse.REMAINDER)
    run.set_defaults(handler=run_review)

    finalize = commands.add_parser("finalize-worker")
    finalize.add_argument("--campaign-root", required=True, type=Path)
    finalize.add_argument("--pair-id", required=True)
    finalize.add_argument("--realtime-result", required=True, type=Path)
    finalize.add_argument("--realtime-receipt", required=True, type=Path)
    finalize.add_argument("--batch-result", required=True, type=Path)
    finalize.add_argument("--batch-receipt", required=True, type=Path)
    finalize.add_argument("--batch-event-log", required=True, type=Path)
    finalize.add_argument("--controller-receipt", required=True, type=Path)
    finalize.add_argument("--receipt", required=True, type=Path)
    finalize.add_argument("--timeout-seconds", required=True, type=float)
    finalize.add_argument("--poll-seconds", required=True, type=float)
    finalize.set_defaults(handler=finalize_worker)
    return parser


def main(argv: Sequence[str] | None = None) -> int:
    arguments = build_parser().parse_args(argv)
    try:
        if arguments.mode == "run":
            if arguments.ordinal <= 0:
                raise TransportError("--ordinal must be positive")
            if arguments.attestation_max_age_seconds <= 0:
                raise TransportError("--attestation-max-age-seconds must be positive")
            if arguments.batch_poll_interval_seconds <= 0:
                raise TransportError("--batch-poll-interval-seconds must be positive")
            if arguments.batch_poll_timeout_seconds <= 0:
                raise TransportError("--batch-poll-timeout-seconds must be positive")
            if arguments.collection_window_seconds < 0:
                raise TransportError("--collection-window-seconds cannot be negative")
            if arguments.finalizer_timeout_seconds <= 0:
                raise TransportError("--finalizer-timeout-seconds must be positive")
            if arguments.finalizer_poll_seconds <= 0:
                raise TransportError("--finalizer-poll-seconds must be positive")
        if getattr(arguments, "poll_seconds", 1) <= 0:
            raise TransportError("poll interval must be positive")
        return arguments.handler(arguments)
    except (TransportError, OSError, ValueError) as failure:
        print(f"error: {failure}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
