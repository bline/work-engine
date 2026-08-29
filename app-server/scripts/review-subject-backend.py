#!/usr/bin/env python3
from __future__ import annotations

import argparse
import hashlib
import importlib.util
import json
import sys
from pathlib import Path
from typing import Any

BACKEND = "work-engine.review-subject.legacy-v1"
OPERATIONS = {
    "create_candidate", "transition_candidate", "validate_checkpoint",
    "create_physical_profile", "validate_physical_profile",
}
MAX_ERROR_CHARS = 2048


def fail(message: str) -> None:
    raise ValueError(message)


def canonical(value: Any) -> bytes:
    return json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=False).encode()


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def exact(value: Any, fields: set[str], label: str) -> dict[str, Any]:
    if not isinstance(value, dict) or set(value) != fields:
        fail(f"{label} fields do not match schema version 1")
    return value


def load(name: str, source: Path) -> Any:
    spec = importlib.util.spec_from_file_location(name, source)
    if spec is None or spec.loader is None:
        fail(f"{name} backend is unavailable")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def dispatch(operation: str, value: dict[str, Any], checkpoint: Any, profile: Any) -> dict[str, Any]:
    if operation == "create_candidate":
        exact(value, {"request"}, "create_candidate input")
        return checkpoint.create_candidate(value["request"])
    if operation == "transition_candidate":
        exact(value, {"candidate", "kind", "expected_accepted"}, "transition_candidate input")
        return checkpoint.transition(
            value["candidate"], value["kind"], expected_accepted=value["expected_accepted"]
        )
    if operation == "validate_checkpoint":
        exact(value, {"receipt", "kind", "require_paths"}, "validate_checkpoint input")
        return checkpoint.validate_lifecycle_receipt(
            value["receipt"], value["kind"], require_paths=value["require_paths"]
        )
    if operation == "create_physical_profile":
        exact(value, {"subject", "repository"}, "create_physical_profile input")
        repository = Path(value["repository"]).resolve() if value["repository"] is not None else None
        return profile.profile(value["subject"], repository)
    if operation == "validate_physical_profile":
        exact(value, {"profile"}, "validate_physical_profile input")
        return profile.validate_profile(value["profile"])
    fail(f"unsupported review-subject operation: {operation}")


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--workspace-root", required=True, type=Path)
    args = parser.parse_args()
    try:
        request = json.loads(sys.stdin.buffer.read())
        exact(request, {
            "schema_version", "backend", "operation", "expected_backend_sha256", "input"
        }, "review-subject backend request")
        if request["schema_version"] != 1 or request["backend"] != BACKEND:
            fail("review-subject backend request identity mismatch")
        operation = request["operation"]
        if operation not in OPERATIONS:
            fail(f"unsupported review-subject operation: {operation}")
        root = args.workspace_root.resolve()
        checkpoint_path = root / "skills/slice-checkpoint/scripts/checkpoint.py"
        profile_path = root / "skills/code-change-profile/scripts/code_change_profile.py"
        observed = {"checkpoint": sha256(checkpoint_path), "physical_profile": sha256(profile_path)}
        if request["expected_backend_sha256"] != observed:
            fail("review-subject backend source identity mismatch")
        checkpoint = load("work_engine_review_subject_checkpoint", checkpoint_path)
        profile = load("work_engine_review_subject_physical_profile", profile_path)
        if observed["checkpoint"] != profile.CHECKPOINT_VALIDATOR_SHA256:
            fail("physical profile checkpoint-validator binding mismatch")
        # The legacy analyzer locates its validator below the repository it profiles.
        # The host capability keeps code ownership in this workspace instead: inject
        # the already digest-verified canonical module without writing into the
        # subject repository or weakening the analyzer-version binding.
        profile.checkpoint_module = lambda _repository: checkpoint
        result = dispatch(operation, request["input"], checkpoint, profile)
        envelope = {
            "schema_version": 1, "backend": BACKEND, "backend_sha256": observed,
            "operation": operation, "result": result,
        }
        sys.stdout.buffer.write(canonical(envelope) + b"\n")
        return 0
    except Exception as error:
        # Normalize import, identity, schema, and dispatch failures into the one
        # bounded bridge envelope. BaseException process signals remain visible
        # to the host rather than being mistaken for an ordinary backend error.
        message = " ".join(str(error).splitlines())[:MAX_ERROR_CHARS]
        print(f"review-subject: {message}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
