#!/usr/bin/env python3
"""Fixed JSON bridge for legacy-owned slice-supervisor campaign mechanics."""

from __future__ import annotations

import hashlib
import importlib.util
import json
import sys
from pathlib import Path
from typing import Any


ROOT = Path(__file__).resolve().parents[2]
BACKENDS = {
    "checkpoint_lifecycle": ROOT / "skills/slice-supervisor/scripts/checkpoint_lifecycle.py",
    "completion_offer_lifecycle": ROOT / "skills/slice-supervisor/scripts/completion_offer_lifecycle.py",
    "finalize_receipt": ROOT / "skills/slice-supervisor/scripts/finalize_receipt.py",
    "resume_campaign": ROOT / "skills/slice-supervisor/scripts/resume_campaign.py",
    "append_metrics": ROOT / "skills/slice-supervisor/scripts/append_metrics.py",
    "assemble_receipt": ROOT / "skills/slice-supervisor/scripts/assemble_receipt.py",
    "slice_checkpoint": ROOT / "skills/slice-checkpoint/scripts/checkpoint.py",
    "completion_commit": ROOT / "skills/slice-completion-commit/scripts/completion_commit.py",
}
OPERATIONS = {
    "checkpoint.accept", "checkpoint.stop",
    "receipt.finalize", "receipt.validate",
    "offer.open", "offer.load", "offer.resolve", "offer.reconcile", "offer.expire",
    "resume.terminal",
}


def fail(message: str) -> None:
    raise ValueError(message)


def exact(value: Any, fields: set[str], label: str) -> dict[str, Any]:
    if not isinstance(value, dict) or set(value) != fields:
        fail(f"{label} fields are invalid")
    return value


def load(name: str):
    path = BACKENDS[name]
    spec = importlib.util.spec_from_file_location(f"work_engine_host_{name}", path)
    if not spec or not spec.loader:
        raise RuntimeError(f"cannot load fixed supervisor backend: {name}")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def digest(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def verify_digests(expected: Any) -> dict[str, str]:
    exact(expected, set(BACKENDS), "expected_backend_sha256")
    observed = {name: digest(path) for name, path in BACKENDS.items()}
    if expected != observed:
        fail("fixed supervisor backend digest changed")
    return observed


def dispatch(operation: str, value: Any) -> Any:
    if operation == "checkpoint.accept":
        item = exact(value, {"candidate", "review_result", "gate_receipt_digest", "expected_accepted"}, "checkpoint.accept input")
        return load("checkpoint_lifecycle").accept(
            item["candidate"], item["review_result"], item["gate_receipt_digest"],
            item["expected_accepted"],
        )
    if operation == "checkpoint.stop":
        item = exact(value, {"candidate"}, "checkpoint.stop input")
        return load("checkpoint_lifecycle").stop(item["candidate"])
    if operation == "receipt.finalize":
        item = exact(value, {
            "path", "semantic_receipt", "telemetry_ingress", "campaign_preflight",
            "handoff_receipt", "checkpoint_receipt", "completion_commit_receipt",
        }, "receipt.finalize input")
        return load("finalize_receipt").finalize(
            Path(item["path"]), item["semantic_receipt"], item["telemetry_ingress"],
            item["campaign_preflight"], item["handoff_receipt"],
            item["checkpoint_receipt"], item["completion_commit_receipt"],
        )
    if operation == "receipt.validate":
        item = exact(value, {"receipt"}, "receipt.validate input")
        return load("append_metrics").validate_current_write(item["receipt"])
    if operation == "offer.open":
        item = exact(value, {"request"}, "offer.open input")
        return load("completion_offer_lifecycle").open_offer(item["request"])
    if operation == "offer.load":
        item = exact(value, {"repository", "run_id", "slice_number"}, "offer.load input")
        return load("completion_offer_lifecycle").load(
            Path(item["repository"]), item["run_id"], item["slice_number"],
        )
    if operation == "offer.resolve":
        item = exact(value, {"offer", "decision"}, "offer.resolve input")
        return load("completion_offer_lifecycle").resolve(item["offer"], item["decision"])
    if operation == "offer.reconcile":
        item = exact(value, {"offer"}, "offer.reconcile input")
        return load("completion_offer_lifecycle").reconcile(item["offer"])
    if operation == "offer.expire":
        item = exact(value, {"offer", "reason"}, "offer.expire input")
        return load("completion_offer_lifecycle").expire(item["offer"], item["reason"])
    if operation == "resume.terminal":
        item = exact(value, {"path", "campaign_preflight", "run_id"}, "resume.terminal input")
        return load("resume_campaign").resume(
            Path(item["path"]), item["campaign_preflight"], item["run_id"],
        )
    fail("fixed supervisor backend operation is unsupported")


def main() -> int:
    try:
        request = json.loads(sys.stdin.read())
        exact(
            request,
            {"schema_version", "operation", "expected_backend_sha256", "input"},
            "supervisor backend request",
        )
        if request["schema_version"] != 1 or request["operation"] not in OPERATIONS:
            fail("supervisor backend request version or operation is unsupported")
        observed = verify_digests(request["expected_backend_sha256"])
        result = dispatch(request["operation"], request["input"])
        print(json.dumps({
            "schema_version": 1,
            "operation": request["operation"],
            "backend_sha256": observed,
            "result": result,
        }, ensure_ascii=False, separators=(",", ":")))
    except (json.JSONDecodeError, OSError, RuntimeError, ValueError) as error:
        print(f"supervisor_campaign_backend: {error}", file=sys.stderr)
        return 2
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
