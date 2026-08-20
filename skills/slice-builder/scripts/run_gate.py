#!/usr/bin/env python3
"""Run an ordered validation manifest and emit one compact JSON result."""

from __future__ import annotations

import argparse
import json
import subprocess
import sys
import time
from pathlib import Path
from typing import Any


MAX_EXCERPT_CHARS = 2000


def fail(message: str) -> None:
    raise ValueError(message)


def load_manifest(raw: str) -> dict[str, Any]:
    try:
        manifest = json.loads(raw)
    except json.JSONDecodeError as error:
        fail(f"invalid JSON: {error}")
    if not isinstance(manifest, dict):
        fail("manifest must be a JSON object")
    checks = manifest.get("checks")
    if not isinstance(checks, list) or not checks:
        fail("manifest.checks must be a nonempty array")
    for index, check in enumerate(checks):
        if not isinstance(check, dict):
            fail(f"checks[{index}] must be an object")
        for key in ("requirement", "identity"):
            if not isinstance(check.get(key), str) or not check[key].strip():
                fail(f"checks[{index}].{key} must be a nonempty string")
        command = check.get("command")
        if (
            not isinstance(command, list)
            or not command
            or not all(isinstance(part, str) and part for part in command)
        ):
            fail(f"checks[{index}].command must be a nonempty string array")
        timeout_seconds = check.get("timeout_seconds")
        if timeout_seconds is not None and (
            isinstance(timeout_seconds, bool)
            or not isinstance(timeout_seconds, (int, float))
            or timeout_seconds <= 0
        ):
            fail(f"checks[{index}].timeout_seconds must be positive")
    return manifest


def excerpt(stdout: str, stderr: str) -> str:
    combined = "\n".join(part.strip() for part in (stdout, stderr) if part.strip())
    return combined[-MAX_EXCERPT_CHARS:]


def as_text(value: str | bytes | None) -> str:
    if value is None:
        return ""
    return value.decode(errors="replace") if isinstance(value, bytes) else value


def run(manifest: dict[str, Any], cwd: Path) -> dict[str, Any]:
    results: list[dict[str, Any]] = []
    started = time.monotonic()
    for check in manifest["checks"]:
        check_started = time.monotonic()
        try:
            completed = subprocess.run(
                check["command"],
                cwd=cwd,
                capture_output=True,
                text=True,
                timeout=check.get("timeout_seconds"),
                check=False,
            )
            result = {
                "requirement": check["requirement"],
                "identity": check["identity"],
                "state": "passed" if completed.returncode == 0 else "failed",
                "exit_status": completed.returncode,
                "duration_seconds": round(time.monotonic() - check_started, 3),
            }
            if completed.returncode != 0:
                result["error_excerpt"] = excerpt(completed.stdout, completed.stderr)
        except subprocess.TimeoutExpired as error:
            result = {
                "requirement": check["requirement"],
                "identity": check["identity"],
                "state": "failed",
                "exit_status": None,
                "duration_seconds": round(time.monotonic() - check_started, 3),
                "error_excerpt": excerpt(
                    as_text(error.stdout),
                    as_text(error.stderr) + "\ncommand timed out",
                ),
            }
        except OSError as error:
            result = {
                "requirement": check["requirement"],
                "identity": check["identity"],
                "state": "failed",
                "exit_status": None,
                "duration_seconds": round(time.monotonic() - check_started, 3),
                "error_excerpt": str(error)[-MAX_EXCERPT_CHARS:],
            }
        results.append(result)
        if result["state"] != "passed":
            break
    passed = sum(result["state"] == "passed" for result in results)
    status = "passed" if passed == len(manifest["checks"]) else "failed"
    return {
        "status": status,
        "checks": results,
        "failed_check": next(
            (result["identity"] for result in results if result["state"] == "failed"),
            None,
        ),
        "totals": {
            "configured": len(manifest["checks"]),
            "executed": len(results),
            "passed": passed,
            "failed": len(results) - passed,
        },
        "duration_seconds": round(time.monotonic() - started, 3),
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    source = parser.add_mutually_exclusive_group(required=True)
    source.add_argument("--manifest-json")
    source.add_argument("--stdin", action="store_true")
    parser.add_argument("--cwd", type=Path, default=Path.cwd())
    args = parser.parse_args()
    raw = sys.stdin.read() if args.stdin else args.manifest_json
    try:
        result = run(load_manifest(raw), args.cwd.resolve())
    except (OSError, ValueError) as error:
        print(json.dumps({"status": "invalid", "error": str(error)}, separators=(",", ":")))
        return 2
    print(json.dumps(result, separators=(",", ":")))
    return 0 if result["status"] == "passed" else 1


if __name__ == "__main__":
    raise SystemExit(main())
