#!/usr/bin/env python3
"""Run one immutable group-3 model job with a pre-launch marker and receipt."""

from __future__ import annotations

import argparse
import datetime as dt
import hashlib
import json
import subprocess
import sys
import tempfile
from pathlib import Path


class JobError(RuntimeError):
    pass


def require(condition: bool, message: str) -> None:
    if not condition:
        raise JobError(message)


def canonical(value: object) -> bytes:
    return (json.dumps(value, indent=2, sort_keys=True, ensure_ascii=False) + "\n").encode()


def digest(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def timestamp() -> str:
    return dt.datetime.now(dt.timezone.utc).isoformat().replace("+00:00", "Z")


def emit_new(path: Path, value: object | bytes) -> None:
    require(not path.exists(), f"refusing to overwrite: {path}")
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_bytes(value if isinstance(value, bytes) else canonical(value))


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--provider", choices=("codex", "claude"), required=True)
    parser.add_argument("--stage", choices=("render", "semantic-review", "manipulation"), required=True)
    parser.add_argument("--job-id", required=True)
    parser.add_argument("--launch-set-digest", required=True)
    parser.add_argument("--request-digest", required=True)
    parser.add_argument("--packet", required=True, type=Path)
    parser.add_argument("--schema", required=True, type=Path)
    parser.add_argument("--prompt", required=True)
    parser.add_argument("--model", required=True)
    parser.add_argument("--reasoning-effort", required=True)
    parser.add_argument("--attempt-marker", required=True, type=Path)
    parser.add_argument("--raw-output", required=True, type=Path)
    parser.add_argument("--events-output", required=True, type=Path)
    parser.add_argument("--receipt-output", required=True, type=Path)
    args = parser.parse_args()
    packet, schema = args.packet.read_bytes(), args.schema.read_bytes()
    record = {
        "experiment_id": "linguistic-register-behavioral-pilot-group-3-v1-2026-08-26",
        "job_id": args.job_id, "stage": args.stage,
        "packet_sha256": digest(packet), "schema_sha256": digest(schema),
        "prompt_sha256": digest(args.prompt.encode()),
        "provider": "OpenAI" if args.provider == "codex" else "Anthropic",
        "model": args.model, "reasoning_effort": args.reasoning_effort,
        "service_tier": "standard_not_fast" if args.provider == "codex" else "provider_default",
    }
    require(digest(canonical(record)) == args.request_digest, "request digest mismatch")
    started = timestamp()
    emit_new(args.attempt_marker, {"artifact_type": "linguistic_register_group_3_attempt_marker_v1",
                                   "schema_version": 1, **record, "request_sha256": args.request_digest,
                                   "launch_set_sha256": args.launch_set_digest,
                                   "marked_at_utc": started, "state": "model_launch_imminent"})
    with tempfile.TemporaryDirectory(prefix=f"linguistic-register-group3-{args.stage}-") as temporary:
        directory = Path(temporary)
        raw_path = directory / "raw-output.json"
        prompt = args.prompt + "\n\nPACKET:\n" + packet.decode()
        if args.provider == "codex":
            command = [
                "codex", "exec", "--ignore-user-config", "--ignore-rules", "--model", args.model,
                "-c", f'model_reasoning_effort="{args.reasoning_effort}"', "--sandbox", "read-only",
                "--ephemeral", "--skip-git-repo-check", "--json", "--output-schema", str(args.schema.resolve()),
                "--output-last-message", str(raw_path), prompt,
            ]
        else:
            command = [
                "claude", "--print", "--safe-mode", "--disable-slash-commands", "--tools", "",
                "--no-session-persistence", "--model", args.model, "--effort", args.reasoning_effort,
                "--output-format", "json", "--json-schema", schema.decode(),
                "--system-prompt", "Judge only the supplied packet. Use no external context or tools.", prompt,
            ]
        completed = subprocess.run(command, cwd=directory, check=False, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
        if args.provider == "codex":
            raw = raw_path.read_bytes() if raw_path.is_file() else b""
            events = completed.stdout
        else:
            events = completed.stdout
            raw = b""
            if completed.returncode == 0:
                envelope = json.loads(completed.stdout)
                structured = envelope.get("structured_output")
                if structured is None:
                    result = envelope.get("result", "")
                    structured = json.loads(result) if isinstance(result, str) else result
                raw = canonical(structured)
        finished = timestamp()
    emit_new(args.raw_output, raw)
    emit_new(args.events_output, events)
    version_command = ["codex", "--version"] if args.provider == "codex" else ["claude", "--version"]
    version = subprocess.run(version_command, check=True, stdout=subprocess.PIPE).stdout.decode().strip()
    emit_new(args.receipt_output, {
        "artifact_type": "linguistic_register_group_3_model_receipt_v1", "schema_version": 1,
        **record, "request_sha256": args.request_digest, "launch_set_sha256": args.launch_set_digest,
        "launcher_version": version, "started_at_utc": started, "completed_at_utc": finished,
        "return_code": completed.returncode,
        "completion_status": "completed" if completed.returncode == 0 and raw else "failed",
        "fresh_process": True, "ephemeral": True, "tools_requested": "none",
        "isolated_temporary_directory": True, "filesystem_visibility_claimed": False,
        "raw_output_sha256": digest(raw), "events_output_sha256": digest(events),
        "stderr_sha256": digest(completed.stderr),
    })
    require(completed.returncode == 0 and raw, f"model job failed: {completed.stderr.decode(errors='replace')}")
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (JobError, OSError, subprocess.SubprocessError, json.JSONDecodeError) as error:
        print(f"behavioral_pilot_preoutcome_job: {error}", file=sys.stderr)
        raise SystemExit(2)
