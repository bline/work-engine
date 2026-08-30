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

from claude_transport import TransportError


SCRIPT_DIR = Path(__file__).resolve().parent
TRANSPORT_SCRIPT = SCRIPT_DIR / "claude_transport.py"
BATCH_SCRIPT = SCRIPT_DIR / "claude_batch_review.py"
EVIDENCE_SCRIPT = SCRIPT_DIR / "paired_review_evidence.py"
TERMINAL_BATCH_STATES = {"success", "failed", "infrastructure_failed"}


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
        "--receipt", str(paths["finalizer_receipt"]),
        "--timeout-seconds", str(args.finalizer_timeout_seconds),
        "--poll-seconds", str(args.finalizer_poll_seconds),
    ]


def run_review(args: argparse.Namespace) -> int:
    command = list(args.claude_command)
    if command and command[0] == "--":
        command = command[1:]
    if not command:
        raise TransportError("a native Claude command is required after '--'")
    if not args.working_directory.is_dir():
        raise TransportError("--working-directory must be an existing directory")

    registration = register_pair(args)
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
        "artifact_type": "claude_paired_review_controller_v1",
        "schema_version": 1,
        "campaign_id": registration["campaign_id"],
        "pair_id": args.pair_id,
        "ordinal": args.ordinal,
        "registered_at": registration["registered_at"],
        "status": "registered",
        "command_sha256": command_digest,
        "review_packet_sha256": registration["review_packet"]["sha256"],
        "working_directory": str(args.working_directory.resolve()),
        "realtime": {"status": "pending", "returncode": None},
        "batch": {"status": "pending", "pid": None},
        "finalizer": {"status": "pending", "pid": None},
    }
    atomic_json(paths["controller_receipt"], controller)

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
        "--continuity", args.continuity,
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
    atomic_bytes(paths["realtime_stderr"], result.stderr)
    controller["realtime"] = {
        "status": "success" if result.returncode == 0 else "failed",
        "returncode": result.returncode,
        "completed_at": now(),
        "duration_ms": round((time.monotonic() - started) * 1000),
        "stdout_sha256": sha256_bytes(result.stdout),
        "stderr_sha256": sha256_bytes(result.stderr),
    }
    controller["status"] = (
        "realtime_returned_batch_pending"
        if result.returncode == 0 else "realtime_failed_batch_pending"
    )
    atomic_json(paths["controller_receipt"], controller)
    sys.stdout.buffer.write(result.stdout)
    sys.stderr.buffer.write(result.stderr)
    return result.returncode


def finalize_worker(args: argparse.Namespace) -> int:
    receipt = {
        "artifact_type": "claude_paired_review_finalizer_v1",
        "schema_version": 1,
        "pair_id": args.pair_id,
        "status": "waiting",
        "started_at": now(),
        "completed_at": None,
        "finalize_returncode": None,
    }
    atomic_json(args.receipt, receipt)
    deadline = time.monotonic() + args.timeout_seconds
    while time.monotonic() < deadline:
        if args.realtime_receipt.exists() and args.batch_receipt.exists():
            try:
                realtime = load_object(args.realtime_receipt)
                batch = load_object(args.batch_receipt)
            except TransportError:
                pass
            else:
                if (realtime.get("result") != "pending"
                        and batch.get("status") in TERMINAL_BATCH_STATES):
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
    ]
    result = subprocess.run(command, capture_output=True, text=True, check=False)
    receipt["status"] = "finalized" if result.returncode in {0, 1} else "failed"
    receipt["comparison_ready"] = result.returncode == 0
    receipt["finalize_returncode"] = result.returncode
    receipt["stdout_sha256"] = sha256_bytes(result.stdout.encode("utf-8"))
    receipt["stderr_sha256"] = sha256_bytes(result.stderr.encode("utf-8"))
    receipt["completed_at"] = now()
    atomic_json(args.receipt, receipt)
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
    run.add_argument("--working-directory", required=True, type=Path)
    run.add_argument("--continuity", choices=("disposable", "retained"), default="retained")
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
