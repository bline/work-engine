#!/usr/bin/env python3
"""Run one isolated packet-only model judgment for the expanded Leveson treatment."""

from __future__ import annotations

import argparse
import datetime as dt
import hashlib
import json
import subprocess
import sys
import tempfile
from pathlib import Path


class JobError(ValueError):
    pass


def require(condition: bool, message: str) -> None:
    if not condition:
        raise JobError(message)


def digest(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def canonical(value: object) -> bytes:
    return (json.dumps(value, indent=2, sort_keys=True) + "\n").encode()


def timestamp() -> str:
    return dt.datetime.now(dt.timezone.utc).isoformat().replace("+00:00", "Z")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--packet", required=True, type=Path)
    parser.add_argument("--schema", required=True, type=Path)
    parser.add_argument("--stage", required=True, choices=("profile-extraction", "semantic-classification", "feature-evidence-recovery", "recognizability"))
    parser.add_argument("--model", default="gpt-5.6-sol")
    parser.add_argument("--reasoning-effort", default="high")
    parser.add_argument("--prompt", required=True)
    parser.add_argument("--raw-output", required=True, type=Path)
    parser.add_argument("--events-output", required=True, type=Path)
    parser.add_argument("--receipt-output", required=True, type=Path)
    args = parser.parse_args()
    for path in (args.packet, args.schema):
        require(path.is_file(), f"missing input: {path}")
    for path in (args.raw_output, args.events_output, args.receipt_output):
        require(not path.exists(), f"refusing to overwrite output: {path}")
        path.parent.mkdir(parents=True, exist_ok=True)
    packet = args.packet.read_bytes()
    schema = args.schema.read_bytes()
    started = timestamp()
    with tempfile.TemporaryDirectory(prefix=f"leveson-expanded-{args.stage}-") as temporary:
        directory = Path(temporary)
        (directory / "packet.json").write_bytes(packet)
        (directory / "output.schema.json").write_bytes(schema)
        raw = directory / "raw-output.json"
        command = [
            "codex", "exec", "--model", args.model,
            "-c", f'model_reasoning_effort="{args.reasoning_effort}"',
            "--sandbox", "read-only", "--ephemeral", "--skip-git-repo-check", "--json",
            "--output-schema", str(directory / "output.schema.json"),
            "--output-last-message", str(raw), args.prompt,
        ]
        completed = subprocess.run(command, cwd=directory, stdout=subprocess.PIPE, stderr=subprocess.PIPE, check=False)
        finished = timestamp()
        require(completed.returncode == 0, f"codex exec failed ({completed.returncode}): {completed.stderr.decode(errors='replace')}")
        require(raw.is_file(), "model did not emit raw output")
        raw_bytes = raw.read_bytes()
        args.raw_output.write_bytes(raw_bytes)
        args.events_output.write_bytes(completed.stdout)
    version = subprocess.run(["codex", "--version"], check=True, stdout=subprocess.PIPE).stdout.decode().strip()
    args.receipt_output.write_bytes(canonical({
        "artifact_type": "linguistic_register_expanded_model_receipt_v1", "schema_version": 1,
        "stage": args.stage, "provider": "OpenAI", "model": args.model,
        "reasoning_effort": args.reasoning_effort, "launcher": "codex exec", "launcher_version": version,
        "started_at_utc": started, "completed_at_utc": finished, "return_code": 0,
        "fresh_process": True, "ephemeral": True, "sandbox": "read-only",
        "isolated_temporary_directory": True, "visible_input_files": ["packet.json", "output.schema.json"],
        "packet_sha256": digest(packet), "schema_sha256": digest(schema), "prompt_sha256": digest(args.prompt.encode()),
        "raw_output_sha256": digest(raw_bytes), "events_output_sha256": digest(completed.stdout),
        "stderr_sha256": digest(completed.stderr),
    }))
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (JobError, OSError, subprocess.SubprocessError) as error:
        print(f"expanded_leveson_model_job: {error}", file=sys.stderr)
        raise SystemExit(2)
