#!/usr/bin/env python3
"""Run one disposable native-Claude review through the batch loopback proxy."""

from __future__ import annotations

import argparse
from datetime import datetime, timezone
import hashlib
import json
import os
from pathlib import Path
import queue
import secrets
import signal
import subprocess
import sys
import tempfile
import threading
import time
from typing import Any, Sequence

from claude_transport import (
    TransportError,
    read_routing_attestation,
    sha256_directory,
)


SCHEMA_VERSION = 1
SCRIPT_DIR = Path(__file__).resolve().parent
PROXY_SCRIPT = SCRIPT_DIR / "claude_batch_loopback_proxy.py"


def canonical_json(value: Any) -> bytes:
    return json.dumps(
        value, sort_keys=True, separators=(",", ":"), ensure_ascii=False
    ).encode("utf-8")


def sha256_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def utc_now() -> str:
    return datetime.now(timezone.utc).isoformat().replace("+00:00", "Z")


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


def claude_version(executable: str, env: dict[str, str]) -> str | None:
    try:
        result = subprocess.run(
            [executable, "--version"], env=env, capture_output=True, text=True,
            timeout=10, check=False,
        )
    except (OSError, subprocess.SubprocessError):
        return None
    if result.returncode != 0:
        return None
    return result.stdout.strip() or result.stderr.strip() or None


def command_shape(command: list[str]) -> list[str]:
    return [Path(command[0]).name] + [
        token.split("=", 1)[0] for token in command[1:] if token.startswith("-")
    ]


def read_events(path: Path) -> list[dict[str, Any]]:
    events: list[dict[str, Any]] = []
    if not path.exists():
        return events
    for number, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
        try:
            value = json.loads(line)
        except json.JSONDecodeError as failure:
            raise TransportError(
                f"batch event log line {number} is invalid JSON"
            ) from failure
        if not isinstance(value, dict):
            raise TransportError(f"batch event log line {number} is not an object")
        events.append(value)
    return events


def summarize_events(events: list[dict[str, Any]]) -> dict[str, Any]:
    starts = [event for event in events if event.get("event") == "proxy_started"]
    submitted = [event for event in events if event.get("event") == "submitted"]
    completed = [event for event in events if event.get("event") == "completed"]
    turns = [event for event in events if event.get("event") == "turn_completed"]
    transient_poll_errors = [
        event for event in events if event.get("event") == "poll_transient_error"
    ]
    abandoned_polls = [
        event for event in events if event.get("event") == "poll_abandoned"
    ]
    submitted_ids = [event.get("batch_id") for event in submitted]
    completed_ids = [event.get("batch_id") for event in completed]
    usage_fields = ("prompt_tokens", "completion_tokens", "total_tokens", "cost")
    aggregate: dict[str, int | float] = {field: 0 for field in usage_fields}
    for event in completed:
        usage = event.get("usage")
        if not isinstance(usage, dict):
            continue
        for field in usage_fields:
            value = usage.get(field)
            if isinstance(value, (int, float)) and not isinstance(value, bool):
                aggregate[field] += value
    return {
        "proxy_start_count": len(starts),
        "submitted_batch_ids": submitted_ids,
        "completed_batch_ids": completed_ids,
        "submitted_batch_count": len(submitted),
        "completed_batch_count": len(completed),
        "successful_turn_count": sum(event.get("successful") is True for event in turns),
        "failed_turn_count": sum(event.get("successful") is False for event in turns),
        "poll_not_found_count": sum(
            event.get("event") == "poll_not_found" for event in events
        ),
        "transient_poll_error_count": len(transient_poll_errors),
        "transient_poll_http_status_counts": {
            str(status): sum(
                event.get("http_status") == status for event in transient_poll_errors
            )
            for status in sorted({
                event.get("http_status") for event in transient_poll_errors
                if isinstance(event.get("http_status"), int)
            })
        },
        "poll_abandoned_batch_ids": [
            event.get("batch_id") for event in abandoned_polls
            if isinstance(event.get("batch_id"), str)
        ],
        "terminal_lineage_complete": (
            len(starts) == 1
            and bool(submitted_ids)
            and len(submitted_ids) == len(set(submitted_ids))
            and sorted(submitted_ids) == sorted(completed_ids)
            and bool(turns)
            and all(event.get("successful") is True for event in turns)
        ),
        "actual_openrouter_usage": aggregate,
    }


def wait_for_ready(
    process: subprocess.Popen[str], timeout: float
) -> dict[str, Any]:
    lines: queue.Queue[str] = queue.Queue(maxsize=1)

    def read_one() -> None:
        assert process.stdout is not None
        lines.put(process.stdout.readline())

    threading.Thread(target=read_one, daemon=True).start()
    try:
        line = lines.get(timeout=timeout)
    except queue.Empty as failure:
        raise TransportError("batch proxy did not become ready before timeout") from failure
    if not line:
        raise TransportError("batch proxy exited before readiness")
    try:
        value = json.loads(line)
    except json.JSONDecodeError as failure:
        raise TransportError("batch proxy emitted invalid readiness JSON") from failure
    if not isinstance(value, dict) or value.get("kind") != "claude-batch-loopback-ready":
        raise TransportError("batch proxy emitted an unexpected readiness object")
    return value


def stop_proxy(process: subprocess.Popen[str]) -> None:
    if process.poll() is not None:
        return
    process.send_signal(signal.SIGINT)
    try:
        process.wait(timeout=10)
    except subprocess.TimeoutExpired:
        process.kill()
        process.wait(timeout=10)


def run_claude_with_cancellation(
    command: list[str], *, env: dict[str, str], cwd: Path,
    cancel_file: Path | None,
) -> tuple[subprocess.CompletedProcess[bytes], dict[str, Any] | None]:
    claude = subprocess.Popen(
        command, env=env, cwd=cwd,
        stdout=subprocess.PIPE, stderr=subprocess.PIPE,
        start_new_session=True,
    )
    cancellation = None
    while True:
        try:
            stdout, stderr = claude.communicate(timeout=0.1)
            break
        except subprocess.TimeoutExpired:
            if cancel_file is None or not cancel_file.exists():
                continue
            cancellation = json.loads(cancel_file.read_text(encoding="utf-8"))
            try:
                os.killpg(claude.pid, signal.SIGINT)
            except ProcessLookupError:
                pass
            stdout, stderr = claude.communicate()
            break
    return (
        subprocess.CompletedProcess(command, claude.returncode, stdout, stderr),
        cancellation,
    )


def base_receipt(args: argparse.Namespace, command: list[str]) -> dict[str, Any]:
    config_digest = sha256_directory(args.claude_config_dir)
    attestation = read_routing_attestation(
        args.routing_attestation,
        expected_model=args.batch_model,
        expected_key_hash=os.environ.get("OPENROUTER_API_KEY_HASH", ""),
        max_age_seconds=args.attestation_max_age_seconds,
    )
    env = dict(os.environ)
    env["CLAUDE_CONFIG_DIR"] = str(args.claude_config_dir)
    return {
        "artifact_type": "claude_batch_review_execution_v1",
        "schema_version": SCHEMA_VERSION,
        "status": "registered",
        "registered_at": utc_now(),
        "started_at": None,
        "completed_at": None,
        "harness": "claude-code",
        "claude_version": claude_version(command[0], env),
        "gateway": "openrouter-batch",
        "requested_upstream_provider": "anthropic",
        "upstream_provider_observed": False,
        "model": args.batch_model,
        "command_sha256": sha256_bytes(canonical_json(command)),
        "command_shape": command_shape(command),
        "claude_config_dir": str(args.claude_config_dir),
        "claude_config_dir_sha256_before": config_digest,
        "experimental_betas_disabled": True,
        "terminal_title_disabled": True,
        "routing_attestation": attestation,
        "proxy": {
            "script": str(PROXY_SCRIPT),
            "sha256": sha256_bytes(PROXY_SCRIPT.read_bytes()),
            "event_log": str(args.event_log),
            "event_log_sha256": None,
            "stderr": str(args.event_log.with_suffix(args.event_log.suffix + ".proxy.stderr")),
            "stderr_sha256": None,
            "returncode": None,
        },
        "output": {
            "stdout": str(args.stdout), "stdout_sha256": None,
            "stderr": str(args.stderr), "stderr_sha256": None,
            "returncode": None,
        },
        "batch_summary": None,
        "duration_ms": None,
        "failure": None,
    }


def run(args: argparse.Namespace) -> int:
    command = list(args.command)
    if command and command[0] == "--":
        command = command[1:]
    if not command:
        raise TransportError("a native Claude command is required after '--'")
    if not args.claude_config_dir.is_dir():
        raise TransportError("--claude-config-dir must be an existing directory")
    receipt = base_receipt(args, command)
    atomic_json(args.receipt, receipt)

    token = secrets.token_urlsafe(32)
    proxy_env = dict(os.environ)
    proxy_env["CLAUDE_BATCH_PROXY_TOKEN"] = token
    proxy_command = [
        sys.executable, str(PROXY_SCRIPT),
        "--port", "0",
        "--batch-model", args.batch_model,
        "--guardrail-model", args.batch_model,
        "--routing-attestation", str(args.routing_attestation),
        "--attestation-max-age-seconds", str(args.attestation_max_age_seconds),
        "--poll-interval-seconds", str(args.poll_interval_seconds),
        "--poll-timeout-seconds", str(args.poll_timeout_seconds),
        "--collection-window-seconds", str(args.collection_window_seconds),
        "--event-log", str(args.event_log),
    ]
    proxy_stderr = args.event_log.with_suffix(args.event_log.suffix + ".proxy.stderr")
    proxy_stderr.parent.mkdir(parents=True, exist_ok=True)
    started = time.monotonic()
    receipt["status"] = "running"
    receipt["started_at"] = utc_now()
    atomic_json(args.receipt, receipt)
    proxy: subprocess.Popen[str] | None = None
    try:
        with proxy_stderr.open("w", encoding="utf-8") as proxy_error:
            proxy = subprocess.Popen(
                proxy_command, env=proxy_env, stdout=subprocess.PIPE,
                stderr=proxy_error, text=True,
            )
            ready = wait_for_ready(proxy, args.proxy_start_timeout_seconds)
            claude_env = dict(os.environ)
            claude_env.pop("OPENROUTER_API_KEY", None)
            claude_env["CLAUDE_CONFIG_DIR"] = str(args.claude_config_dir)
            claude_env["ANTHROPIC_BASE_URL"] = ready["base_url"]
            claude_env["ANTHROPIC_AUTH_TOKEN"] = token
            claude_env["ANTHROPIC_API_KEY"] = ""
            claude_env["ANTHROPIC_DEFAULT_SONNET_MODEL"] = args.batch_model
            claude_env["CLAUDE_CODE_DISABLE_EXPERIMENTAL_BETAS"] = "1"
            claude_env["CLAUDE_CODE_DISABLE_TERMINAL_TITLE"] = "1"
            result, cancellation = run_claude_with_cancellation(
                command, env=claude_env, cwd=args.working_directory,
                cancel_file=getattr(args, "cancel_file", None),
            )
            atomic_bytes(args.stdout, result.stdout)
            atomic_bytes(args.stderr, result.stderr)
            receipt["output"].update({
                "stdout_sha256": sha256_bytes(result.stdout),
                "stderr_sha256": sha256_bytes(result.stderr),
                "returncode": result.returncode,
            })
            if cancellation is not None:
                receipt["status"] = "cancelled"
                receipt["cancellation"] = {
                    "request": cancellation,
                    "request_sha256": sha256_bytes(
                        args.cancel_file.read_bytes()
                    ),
                    "claude_returncode": result.returncode,
                }
            else:
                receipt["status"] = "success" if result.returncode == 0 else "failed"
    except Exception as failure:
        receipt["status"] = "infrastructure_failed"
        receipt["failure"] = {
            "type": type(failure).__name__, "message": str(failure),
        }
        result = None
    finally:
        if proxy is not None:
            stop_proxy(proxy)
            receipt["proxy"]["returncode"] = proxy.returncode
            if proxy.stdout is not None:
                proxy.stdout.close()
        if proxy_stderr.exists():
            receipt["proxy"]["stderr_sha256"] = sha256_bytes(
                proxy_stderr.read_bytes()
            )
        if not args.stdout.exists():
            atomic_bytes(args.stdout, b"{}")
            receipt["output"]["stdout_sha256"] = sha256_bytes(b"{}")
        if not args.stderr.exists():
            atomic_bytes(args.stderr, b"")
            receipt["output"]["stderr_sha256"] = sha256_bytes(b"")
        receipt["completed_at"] = utc_now()
        receipt["duration_ms"] = round((time.monotonic() - started) * 1000)
        if args.event_log.exists():
            receipt["proxy"]["event_log_sha256"] = sha256_bytes(
                args.event_log.read_bytes()
            )
            try:
                receipt["batch_summary"] = summarize_events(read_events(args.event_log))
            except TransportError as failure:
                receipt["failure"] = {
                    "type": type(failure).__name__, "message": str(failure),
                }
                receipt["status"] = "infrastructure_failed"
        try:
            receipt["claude_config_dir_sha256_after"] = sha256_directory(
                args.claude_config_dir
            )
        except TransportError:
            receipt["claude_config_dir_sha256_after"] = None
        if (receipt["status"] == "success"
                and not receipt.get("batch_summary", {}).get("terminal_lineage_complete")):
            receipt["status"] = "infrastructure_failed"
            receipt["failure"] = {
                "type": "IncompleteBatchLineage",
                "message": "Claude succeeded without complete terminal batch lineage",
            }
        atomic_json(args.receipt, receipt)

    if result is not None:
        sys.stdout.buffer.write(result.stdout)
        sys.stderr.buffer.write(result.stderr)
        return result.returncode if receipt["status"] != "infrastructure_failed" else 1
    print(f"error: {receipt['failure']['message']}", file=sys.stderr)
    return 1


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--receipt", required=True, type=Path)
    parser.add_argument("--event-log", required=True, type=Path)
    parser.add_argument("--stdout", required=True, type=Path)
    parser.add_argument("--stderr", required=True, type=Path)
    parser.add_argument("--claude-config-dir", required=True, type=Path)
    parser.add_argument("--working-directory", required=True, type=Path)
    parser.add_argument("--batch-model", required=True)
    parser.add_argument("--routing-attestation", required=True, type=Path)
    parser.add_argument("--attestation-max-age-seconds", type=int, default=900)
    parser.add_argument("--poll-interval-seconds", type=float, default=2.0)
    parser.add_argument("--poll-timeout-seconds", type=float, default=3600.0)
    parser.add_argument("--collection-window-seconds", type=float, default=0.5)
    parser.add_argument("--cancel-file", type=Path)
    parser.add_argument("--proxy-start-timeout-seconds", type=float, default=15.0)
    parser.add_argument("command", nargs=argparse.REMAINDER)
    return parser


def main(argv: Sequence[str] | None = None) -> int:
    parser = build_parser()
    args = parser.parse_args(argv)
    try:
        return run(args)
    except (TransportError, OSError, ValueError) as failure:
        parser.error(str(failure))
    return 2


if __name__ == "__main__":
    raise SystemExit(main())
