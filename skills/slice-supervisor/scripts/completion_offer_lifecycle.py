#!/usr/bin/env python3
"""Own one durable, non-worktree completion-offer lifecycle per accepted slice."""

from __future__ import annotations

import hashlib
import importlib.util
import json
import re
import subprocess
from pathlib import Path
from typing import Any, Callable


ADAPTER_PATH = Path(__file__).parents[2] / "slice-completion-commit" / "scripts" / "completion_commit.py"
ADAPTER_SPEC = importlib.util.spec_from_file_location("work_engine_completion_commit", ADAPTER_PATH)
if not ADAPTER_SPEC or not ADAPTER_SPEC.loader:
    raise RuntimeError(f"cannot load completion adapter: {ADAPTER_PATH}")
ADAPTER = importlib.util.module_from_spec(ADAPTER_SPEC)
ADAPTER_SPEC.loader.exec_module(ADAPTER)

TERMINAL_STATES = {"created", "declined", "refused", "expired_unavailable"}
FIELDS = {"schema_version", "offer_id", "state", "request", "result", "reason", "prior_oid"}
IDENTITY = re.compile(r"[A-Za-z0-9][A-Za-z0-9._-]*")


def fail(message: str) -> None:
    raise ValueError(message)


def canonical(value: Any) -> bytes:
    return json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=False).encode()


def git(repository: Path, args: list[str], *, input_bytes: bytes | None = None) -> bytes:
    complete = subprocess.run(
        ["git", "-C", str(repository), *args], input=input_bytes,
        stdout=subprocess.PIPE, stderr=subprocess.PIPE, check=False,
    )
    if complete.returncode:
        fail(complete.stderr.decode(errors="replace").strip() or f"git {' '.join(args)} failed")
    return complete.stdout


def offer_ref(request: dict[str, Any]) -> str:
    if not IDENTITY.fullmatch(request["run_id"]):
        fail("completion offer run_id must be a safe identity")
    return f"refs/work-engine/completion-offers/{request['run_id']}/slice-{request['slice_number']}"


def current_oid(repository: Path, ref: str) -> str | None:
    complete = subprocess.run(
        ["git", "-C", str(repository), "rev-parse", "--verify", "--quiet", ref],
        stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True, check=False,
    )
    return complete.stdout.strip() if complete.returncode == 0 else None


def read_oid(repository: Path, oid: str) -> dict[str, Any]:
    value = json.loads(git(repository, ["cat-file", "blob", oid]).decode())
    return validate(value)


def write(repository: Path, value: dict[str, Any], expected: str | None) -> dict[str, Any]:
    payload = canonical(value)
    oid = git(repository, ["hash-object", "-w", "--stdin"], input_bytes=payload).decode().strip()
    ref = offer_ref(value["request"])
    observed = current_oid(repository, ref)
    if observed == oid:
        return {**value, "artifact_oid": oid, "ref": ref}
    if observed != expected:
        fail("completion offer ref conflict")
    zero = "0" * len(oid)
    git(repository, ["update-ref", ref, oid, expected or zero])
    return {**value, "artifact_oid": oid, "ref": ref}


def offer_id(request: dict[str, Any]) -> str:
    return hashlib.sha256(canonical(request)).hexdigest()


def validate(value: Any) -> dict[str, Any]:
    if not isinstance(value, dict) or set(value) != FIELDS:
        fail("completion offer fields must exactly match schema version 1")
    if value["schema_version"] != 1 or value["state"] not in {"open", *TERMINAL_STATES}:
        fail("completion offer has an invalid schema version or state")
    request = ADAPTER.validate_request(value["request"])
    if value["offer_id"] != offer_id(request):
        fail("completion offer identity does not match its request")
    if value["prior_oid"] is not None and not isinstance(value["prior_oid"], str):
        fail("completion offer prior_oid must be a string or null")
    if value["state"] == "open":
        if value["result"] is not None or value["reason"] is not None or value["prior_oid"] is not None:
            fail("open completion offer cannot claim a result, reason, or predecessor")
    elif value["state"] in {"created", "declined", "refused"}:
        result = ADAPTER.validate_receipt(value["result"])
        if result["state"] != value["state"]:
            fail("completion offer state must match the adapter result")
        if value["reason"] is not None or not value["prior_oid"]:
            fail("adapter-resolved completion offer requires only a predecessor")
    elif (value["result"] is not None or not isinstance(value["reason"], str)
          or not value["reason"] or not value["prior_oid"]):
        fail("expired/unavailable completion offer requires a reason and predecessor")
    return value


def open_offer(raw: Any) -> dict[str, Any]:
    request = ADAPTER.validate_request(raw)
    offer_ref(request)
    repository = Path(request["repository"]).resolve()
    value = {"schema_version": 1, "offer_id": offer_id(request), "state": "open",
             "request": request, "result": None, "reason": None, "prior_oid": None}
    return write(repository, value, None)


def load(repository: Path, run_id: str, slice_number: int) -> dict[str, Any] | None:
    if (not IDENTITY.fullmatch(run_id) or isinstance(slice_number, bool)
            or not isinstance(slice_number, int) or slice_number < 1):
        fail("completion offer lookup identity is invalid")
    ref = f"refs/work-engine/completion-offers/{run_id}/slice-{slice_number}"
    oid = current_oid(repository, ref)
    if oid is None:
        return None
    value = read_oid(repository, oid)
    return {**value, "artifact_oid": oid, "ref": ref}


def transition(open_value: dict[str, Any], state: str, *, result: Any = None,
               reason: str | None = None) -> dict[str, Any]:
    if open_value.get("state") != "open" or not open_value.get("artifact_oid"):
        fail("only a loaded open completion offer may transition")
    request = open_value["request"]
    value = {"schema_version": 1, "offer_id": open_value["offer_id"], "state": state,
             "request": request, "result": result, "reason": reason,
             "prior_oid": open_value["artifact_oid"]}
    validate(value)
    return write(Path(request["repository"]).resolve(), value, open_value["artifact_oid"])


def resolve(open_value: dict[str, Any], decision: str, *,
            after_publish: Callable[[str], None] | None = None) -> dict[str, Any]:
    reconciled = reconcile(open_value)
    if reconciled is not None:
        return reconciled
    if decision == "decline":
        result = ADAPTER.decide(open_value["request"], "decline")
    elif decision == "create":
        result = ADAPTER.decide(open_value["request"], "create", after_publish=after_publish)
    else:
        fail("completion offer decision must be create or decline")
    return transition(open_value, result["state"], result=result)


def expire(open_value: dict[str, Any], reason: str) -> dict[str, Any]:
    reconciled = reconcile(open_value)
    if reconciled is not None:
        return reconciled
    return transition(open_value, "expired_unavailable", reason=reason)


def reconcile(open_value: dict[str, Any]) -> dict[str, Any] | None:
    if open_value.get("state") != "open":
        return open_value
    result = ADAPTER.reconcile_created(open_value["request"])
    return None if result is None else transition(open_value, "created", result=result)
