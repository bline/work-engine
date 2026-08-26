#!/usr/bin/env python3
"""Run one v2c packet with a launch-set commitment and pre-launch marker."""

from __future__ import annotations

import argparse
import datetime as dt
import hashlib
import json
import subprocess
import sys
import tempfile
from pathlib import Path


class RunnerError(RuntimeError):
    pass


def require(condition: bool, message: str) -> None:
    if not condition:
        raise RunnerError(message)


def canonical(value: object) -> bytes:
    return (json.dumps(value, indent=2, sort_keys=True) + "\n").encode()


def sha256_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def timestamp() -> str:
    return dt.datetime.now(dt.timezone.utc).isoformat().replace("+00:00", "Z")


def emit_new(path: Path, value: object) -> None:
    require(not path.exists(), f"refusing to overwrite output: {path}")
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_bytes(canonical(value))


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--experiment-id", required=True)
    parser.add_argument("--job-id", required=True)
    parser.add_argument("--stage", required=True, choices=("render", "semantic-adjudication", "matching"))
    parser.add_argument("--launch-set-digest", required=True)
    parser.add_argument("--request-digest", required=True)
    parser.add_argument("--packet", required=True, type=Path)
    parser.add_argument("--schema", required=True, type=Path)
    parser.add_argument("--model", required=True)
    parser.add_argument("--reasoning-effort", required=True)
    parser.add_argument("--prompt", required=True)
    parser.add_argument("--attempt-marker", required=True, type=Path)
    parser.add_argument("--raw-output", required=True, type=Path)
    parser.add_argument("--events-output", required=True, type=Path)
    parser.add_argument("--receipt-output", required=True, type=Path)
    args = parser.parse_args()

    for source, label in ((args.packet, "packet"), (args.schema, "schema")):
        require(source.is_file(), f"{label} is not readable: {source}")
    for output in (args.attempt_marker, args.raw_output, args.events_output, args.receipt_output):
        require(not output.exists(), f"refusing to overwrite output: {output}")
        output.parent.mkdir(parents=True, exist_ok=True)

    packet_bytes = args.packet.read_bytes()
    schema_bytes = args.schema.read_bytes()
    request = {
        "experiment_id": args.experiment_id,
        "job_id": args.job_id,
        "stage": args.stage,
        "launch_set_sha256": args.launch_set_digest,
        "packet_artifact_sha256": sha256_bytes(packet_bytes),
        "output_schema_sha256": sha256_bytes(schema_bytes),
        "model": args.model,
        "reasoning_effort": args.reasoning_effort,
        "prompt_sha256": sha256_bytes(args.prompt.encode()),
    }
    require(sha256_bytes(canonical(request)) == args.request_digest,
            "request digest differs from committed launch set")
    version = subprocess.run(["codex", "--version"], check=True, stdout=subprocess.PIPE,
                             stderr=subprocess.PIPE).stdout.decode().strip()
    started = timestamp()
    emit_new(args.attempt_marker, {
        "artifact_type": "linguistic_register_v2c_attempt_marker",
        "schema_version": 1,
        **request,
        "request_digest": args.request_digest,
        "marked_at_utc": started,
        "state": "model_launch_imminent",
    })

    raw_bytes = b""
    events_bytes = b""
    stderr_bytes = b""
    return_code = 125
    with tempfile.TemporaryDirectory(prefix=f"linguistic-register-v2c-{args.stage}-") as temporary:
        directory = Path(temporary)
        (directory / "packet.json").write_bytes(packet_bytes)
        (directory / "output.schema.json").write_bytes(schema_bytes)
        raw_temporary = directory / "raw-output.json"
        command = [
            "codex", "exec", "--model", args.model,
            "-c", f'model_reasoning_effort="{args.reasoning_effort}"',
            "--sandbox", "read-only", "--ephemeral", "--skip-git-repo-check", "--json",
            "--output-schema", str(directory / "output.schema.json"),
            "--output-last-message", str(raw_temporary), args.prompt,
        ]
        completed = subprocess.run(command, cwd=directory, stdout=subprocess.PIPE,
                                   stderr=subprocess.PIPE, check=False)
        return_code = completed.returncode
        events_bytes = completed.stdout
        stderr_bytes = completed.stderr
        if raw_temporary.is_file():
            raw_bytes = raw_temporary.read_bytes()
    finished = timestamp()
    args.raw_output.write_bytes(raw_bytes)
    args.events_output.write_bytes(events_bytes)
    emit_new(args.receipt_output, {
        "artifact_type": "linguistic_register_model_execution_receipt_v2c",
        "schema_version": 1,
        **request,
        "request_digest": args.request_digest,
        "provider": "OpenAI",
        "launcher": "codex exec",
        "launcher_version": version,
        "started_at_utc": started,
        "completed_at_utc": finished,
        "return_code": return_code,
        "completion_status": "completed" if return_code == 0 and raw_bytes else "failed",
        "fresh_process": True,
        "ephemeral": True,
        "sandbox": "read-only",
        "isolated_temporary_directory": True,
        "staged_input_files": ["packet.json", "output.schema.json"],
        "filesystem_visibility_claimed": False,
        "raw_output_sha256": sha256_bytes(raw_bytes),
        "events_output_sha256": sha256_bytes(events_bytes),
        "stderr_sha256": sha256_bytes(stderr_bytes),
    })
    if return_code != 0 or not raw_bytes:
        raise RunnerError(f"codex exec did not complete successfully (return code {return_code})")
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (RunnerError, OSError, subprocess.SubprocessError) as error:
        print(f"sol_packet_job_v2c: {error}", file=sys.stderr)
        raise SystemExit(2)
