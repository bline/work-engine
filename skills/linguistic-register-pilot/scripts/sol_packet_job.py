#!/usr/bin/env python3
"""Run one packet-only Sol job and retain mechanically bound execution evidence."""

from __future__ import annotations

import argparse
import datetime as dt
import hashlib
import json
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path


class JobError(ValueError):
    pass


def require(condition: bool, message: str) -> None:
    if not condition:
        raise JobError(message)


def sha256_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def canonical(value: object) -> bytes:
    return (json.dumps(value, indent=2, sort_keys=True) + "\n").encode()


def timestamp() -> str:
    return dt.datetime.now(dt.timezone.utc).isoformat().replace("+00:00", "Z")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--packet", required=True, type=Path)
    parser.add_argument("--schema", required=True, type=Path)
    parser.add_argument("--stage", required=True, choices=("render", "semantic-adjudication", "matching"))
    parser.add_argument("--model", required=True)
    parser.add_argument("--reasoning-effort", required=True)
    parser.add_argument("--prompt", required=True)
    parser.add_argument("--raw-output", required=True, type=Path)
    parser.add_argument("--events-output", required=True, type=Path)
    parser.add_argument("--receipt-output", required=True, type=Path)
    args = parser.parse_args()

    for source, label in ((args.packet, "packet"), (args.schema, "schema")):
        require(source.is_file(), f"{label} is not readable: {source}")
    for output in (args.raw_output, args.events_output, args.receipt_output):
        require(not output.exists(), f"refusing to overwrite output: {output}")
        output.parent.mkdir(parents=True, exist_ok=True)

    packet_bytes = args.packet.read_bytes()
    schema_bytes = args.schema.read_bytes()
    version = subprocess.run(["codex", "--version"], check=True, stdout=subprocess.PIPE,
                             stderr=subprocess.PIPE).stdout.decode().strip()
    started = timestamp()
    with tempfile.TemporaryDirectory(prefix=f"linguistic-register-{args.stage}-") as temporary:
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
        finished = timestamp()
        require(completed.returncode == 0,
                f"codex exec failed with {completed.returncode}: {completed.stderr.decode(errors='replace')}")
        require(raw_temporary.is_file(), "codex exec did not produce raw output")
        raw_bytes = raw_temporary.read_bytes()
        events_bytes = completed.stdout
        args.raw_output.write_bytes(raw_bytes)
        args.events_output.write_bytes(events_bytes)

    receipt = {
        "artifact_type": "linguistic_register_model_execution_receipt_v1",
        "schema_version": 1,
        "stage": args.stage,
        "provider": "OpenAI",
        "model": args.model,
        "reasoning_effort": args.reasoning_effort,
        "launcher": "codex exec",
        "launcher_version": version,
        "started_at_utc": started,
        "completed_at_utc": finished,
        "return_code": 0,
        "fresh_process": True,
        "ephemeral": True,
        "sandbox": "read-only",
        "isolated_temporary_directory": True,
        "visible_input_files": ["packet.json", "output.schema.json"],
        "packet_artifact_sha256": sha256_bytes(packet_bytes),
        "output_schema_sha256": sha256_bytes(schema_bytes),
        "prompt_sha256": sha256_bytes(args.prompt.encode()),
        "raw_output_sha256": sha256_bytes(raw_bytes),
        "events_output_sha256": sha256_bytes(events_bytes),
        "stderr_sha256": sha256_bytes(completed.stderr),
    }
    args.receipt_output.write_bytes(canonical(receipt))
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (JobError, OSError, subprocess.SubprocessError) as error:
        print(f"sol_packet_job: {error}", file=sys.stderr)
        raise SystemExit(2)
