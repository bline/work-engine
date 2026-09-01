#!/usr/bin/env python3
"""Own one durable, non-worktree completion-offer lifecycle per accepted slice."""

from __future__ import annotations

import hashlib
import importlib.util
import json
import re
import subprocess
from pathlib import Path
from typing import Any


ADAPTER_PATH = Path(__file__).parents[2] / "slice-completion-commit" / "scripts" / "completion_commit.py"
ADAPTER_SPEC = importlib.util.spec_from_file_location("work_engine_completion_commit", ADAPTER_PATH)
if not ADAPTER_SPEC or not ADAPTER_SPEC.loader:
    raise RuntimeError(f"cannot load completion adapter: {ADAPTER_PATH}")
ADAPTER = importlib.util.module_from_spec(ADAPTER_SPEC)
ADAPTER_SPEC.loader.exec_module(ADAPTER)

TERMINAL_STATES = {"created", "declined", "refused", "expired_unavailable"}
ACTIVE_STATES = {"open", "create_authorized"}
V1_FIELDS = {"schema_version", "offer_id", "state", "request", "result", "reason", "prior_oid"}
V2_FIELDS = V1_FIELDS | {"decision"}
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


def validate_decision(value: Any) -> dict[str, Any]:
    if not isinstance(value, dict) or set(value) != {"decision", "authority"}:
        fail("completion offer decision must contain exactly decision and authority")
    if value["decision"] not in {"create", "decline"}:
        fail("completion offer decision must be create or decline")
    authority = value["authority"]
    if (not isinstance(authority, dict)
            or set(authority) != {"kind", "reference", "observed_at"}
            or authority.get("kind") != "human"):
        fail("completion offer decision requires exact human authority")
    for field in ("reference", "observed_at"):
        if not isinstance(authority[field], str) or not authority[field].strip():
            fail(f"completion offer decision authority.{field} must be nonempty")
    return value


def validate(value: Any) -> dict[str, Any]:
    if not isinstance(value, dict):
        fail("completion offer must be an object")
    version = value.get("schema_version")
    fields = V1_FIELDS if version == 1 else V2_FIELDS if version == 2 else None
    states = {"open", *TERMINAL_STATES} if version == 1 else {
        *ACTIVE_STATES, *TERMINAL_STATES
    }
    projected = fields is not None and set(value) == fields | {"artifact_oid", "ref"}
    if fields is None or (set(value) != fields and not projected) or value.get("state") not in states:
        fail("completion offer has an invalid schema version, fields, or state")
    if projected and (not isinstance(value["artifact_oid"], str)
                      or not isinstance(value["ref"], str)):
        fail("loaded completion offer identity is invalid")
    request = ADAPTER.validate_request(value["request"])
    if value["offer_id"] != offer_id(request):
        fail("completion offer identity does not match its request")
    if value["prior_oid"] is not None and not isinstance(value["prior_oid"], str):
        fail("completion offer prior_oid must be a string or null")
    decision = None if version == 1 else value["decision"]
    if decision is not None:
        validate_decision(decision)
    if value["state"] == "open":
        if value["result"] is not None or value["reason"] is not None or value["prior_oid"] is not None:
            fail("open completion offer cannot claim a result, reason, or predecessor")
        if decision is not None:
            fail("open completion offer cannot claim a human decision")
    elif value["state"] == "create_authorized":
        if (decision is None or decision["decision"] != "create"
                or value["result"] is not None or value["reason"] is not None
                or not value["prior_oid"]):
            fail("create-authorized offer requires only its create decision and predecessor")
    elif value["state"] in {"created", "declined", "refused"}:
        result = ADAPTER.validate_receipt(value["result"])
        if result["state"] != value["state"]:
            fail("completion offer state must match the adapter result")
        if value["reason"] is not None or not value["prior_oid"]:
            fail("adapter-resolved completion offer requires only a predecessor")
        if version == 2 and (decision is None
                or decision["decision"] != ("decline" if value["state"] == "declined" else "create")):
            fail("resolved completion offer does not match its human decision")
    elif (value["result"] is not None or not isinstance(value["reason"], str)
          or not value["reason"] or not value["prior_oid"]):
        fail("expired/unavailable completion offer requires a reason and predecessor")
    return value


def open_offer(raw: Any) -> dict[str, Any]:
    request = ADAPTER.validate_request(raw)
    offer_ref(request)
    repository = Path(request["repository"]).resolve()
    value = {"schema_version": 2, "offer_id": offer_id(request), "state": "open",
             "request": request, "result": None, "reason": None, "prior_oid": None,
             "decision": None}
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


def transition(current: dict[str, Any], state: str, *, result: Any = None,
               reason: str | None = None, decision: Any = None) -> dict[str, Any]:
    if current.get("state") not in ACTIVE_STATES or not current.get("artifact_oid"):
        fail("only a loaded active completion offer may transition")
    request = current["request"]
    version = current["schema_version"]
    value = {"schema_version": version, "offer_id": current["offer_id"], "state": state,
             "request": request, "result": result, "reason": reason,
             "prior_oid": current["artifact_oid"]}
    if version == 2:
        value["decision"] = decision if decision is not None else current.get("decision")
    validate(value)
    return write(Path(request["repository"]).resolve(), value, current["artifact_oid"])


def resolve(open_value: dict[str, Any], decision: Any) -> dict[str, Any]:
    validate(open_value)
    if open_value["schema_version"] != 2:
        fail("historical completion offers cannot acquire inferred human authority")
    resolved_decision = validate_decision(decision)
    if open_value["state"] != "open":
        if open_value.get("decision") == resolved_decision:
            return open_value
        fail("completion offer human decision conflicts with durable state")
    if resolved_decision["decision"] == "decline":
        result = ADAPTER.decide(open_value["request"], "decline")
        return transition(open_value, "declined", result=result, decision=resolved_decision)
    return transition(open_value, "create_authorized", decision=resolved_decision)


def expire(open_value: dict[str, Any], reason: str) -> dict[str, Any]:
    reconciled = reconcile(open_value)
    if reconciled is not None:
        return reconciled
    return transition(open_value, "expired_unavailable", reason=reason)


def reconcile(open_value: dict[str, Any]) -> dict[str, Any] | None:
    validate(open_value)
    if open_value.get("state") in TERMINAL_STATES:
        return open_value
    if open_value["schema_version"] == 2 and open_value["state"] == "open":
        return None
    result = ADAPTER.reconcile_created(open_value["request"])
    return None if result is None else transition(open_value, "created", result=result)
