#!/usr/bin/env python3
"""Own semantic state and transitions for one active slice attempt."""

from __future__ import annotations

import hashlib
import importlib.util
import json
from pathlib import Path
from typing import Any, Callable

STORE_PATH = Path(__file__).parents[2] / "durable-state/scripts/durable_state.py"
SPEC = importlib.util.spec_from_file_location("work_engine_durable_state", STORE_PATH)
if SPEC is None or SPEC.loader is None:
    raise RuntimeError("unable to load durable-state primitive")
DURABLE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(DURABLE)

PHASES = {"planning", "implementation", "gate", "review"}
STATUSES = {"active", "waiting_on_capability", "retired"}
PHASE_ORDER = {"planning": 0, "implementation": 1, "review": 2, "gate": 3}


def fail(message: str) -> None:
    raise ValueError(message)


def canonical(value: object) -> bytes:
    return json.dumps(value, sort_keys=True, separators=(",", ":")).encode()


def is_sha256(value: Any) -> bool:
    return (isinstance(value, str) and len(value) == 64
            and all(character in "0123456789abcdef" for character in value))


def validate_identity(value: Any) -> None:
    if not isinstance(value, dict) or set(value) != {"run_id", "slice_number", "attempt_id", "plan_version"}:
        fail("identity fields are invalid")
    for field in ("run_id", "attempt_id", "plan_version"):
        if not isinstance(value[field], str) or not value[field]:
            fail(f"identity.{field} must be a nonempty string")
    if type(value["slice_number"]) is not int or value["slice_number"] < 1:
        fail("identity.slice_number must be positive")


def stable_key(identity: dict[str, Any]) -> str:
    validate_identity(identity)
    return "slice-attempt-v1/" + hashlib.sha256(canonical(identity)).hexdigest()


def validate(value: Any) -> dict[str, Any]:
    fields = {"schema_version", "identity", "actor_binding", "phase", "status",
              "pending_obligation", "handled_consequences", "authoritative_refs",
              "accepted_boundary", "latest_phase_consequence", "waiting", "retirement"}
    if not isinstance(value, dict) or set(value) != fields or value["schema_version"] != 2:
        fail("live slice state fields or schema are invalid")
    validate_identity(value["identity"])
    actor = value["actor_binding"]
    if not isinstance(actor, dict) or set(actor) != {"logical_actor_id", "provider", "runtime_session_id"}:
        fail("actor_binding fields are invalid")
    if not isinstance(actor["logical_actor_id"], str) or not actor["logical_actor_id"]:
        fail("logical_actor_id must be nonempty")
    if any(actor[field] is not None and not isinstance(actor[field], str)
           for field in ("provider", "runtime_session_id")):
        fail("provider bindings must be strings or null")
    if value["phase"] not in PHASES or value["status"] not in STATUSES:
        fail("phase or status is invalid")
    obligation = value["pending_obligation"]
    if not isinstance(obligation, dict) or set(obligation) != {"obligation_id", "kind", "summary"}:
        fail("pending_obligation fields are invalid")
    if any(not isinstance(item, str) or not item for item in obligation.values()):
        fail("pending_obligation values must be nonempty strings")
    handled = value["handled_consequences"]
    if not isinstance(handled, list) or any(not isinstance(item, str) or not item for item in handled):
        fail("handled consequences are invalid")
    if len(handled) != len(set(handled)):
        fail("handled consequences must be unique")
    refs = value["authoritative_refs"]
    if not isinstance(refs, list) or any(not isinstance(item, dict)
        or set(item) != {"kind", "reference"}
        or any(not isinstance(part, str) or not part for part in item.values()) for item in refs):
        fail("authoritative_refs must contain reference-only objects")
    boundary = value["accepted_boundary"]
    if boundary is not None:
        if (not isinstance(boundary, dict)
                or set(boundary) != {"reference", "integrity_sha256"}
                or not isinstance(boundary["reference"], str) or not boundary["reference"]
                or not is_sha256(boundary["integrity_sha256"])):
            fail("accepted_boundary is invalid")
    consequence = value["latest_phase_consequence"]
    if consequence is not None:
        validate_phase_consequence(consequence)
    waiting = value["waiting"]
    if value["status"] == "waiting_on_capability":
        if not isinstance(waiting, dict) or set(waiting) != {"capability", "provider", "reason", "obligation_id"}:
            fail("waiting record is invalid")
        if waiting["obligation_id"] != obligation["obligation_id"] or not waiting["capability"] or not waiting["reason"]:
            fail("waiting record is not bound to the obligation")
        if not isinstance(waiting["capability"], str) or not isinstance(waiting["reason"], str):
            fail("waiting capability and reason must be strings")
        if waiting["provider"] is not None and not isinstance(waiting["provider"], str):
            fail("waiting provider must be a string or null")
    elif waiting is not None:
        fail("only waiting state may contain waiting details")
    if value["status"] == "retired":
        if not isinstance(value["retirement"], dict) or set(value["retirement"]) != {"outcome", "reason"}:
            fail("retired state requires its consequence")
    elif value["retirement"] is not None:
        fail("only retired state may contain retirement")
    return value


def validate_phase_consequence(value: Any) -> dict[str, Any]:
    fields = {"consequence_id", "summary", "certainty", "uncertainty_reason", "references"}
    if not isinstance(value, dict) or set(value) != fields:
        fail("phase consequence fields are invalid")
    if any(not isinstance(value[field], str) or not value[field]
           for field in ("consequence_id", "summary")):
        fail("phase consequence identity and summary must be nonempty strings")
    if value["certainty"] not in {"established", "uncertain"}:
        fail("phase consequence certainty is invalid")
    reason = value["uncertainty_reason"]
    if value["certainty"] == "uncertain":
        if not isinstance(reason, str) or not reason:
            fail("uncertain phase consequence requires a reason")
    elif reason is not None:
        fail("established phase consequence cannot carry an uncertainty reason")
    refs = value["references"]
    if not isinstance(refs, list) or any(
        not isinstance(item, dict)
        or set(item) != {"kind", "reference", "integrity_sha256"}
        or not isinstance(item["kind"], str) or not item["kind"]
        or not isinstance(item["reference"], str) or not item["reference"]
        or not is_sha256(item["integrity_sha256"]) for item in refs):
        fail("phase consequence references are invalid")
    if value["certainty"] == "established" and not refs:
        fail("established phase consequence requires an integrity-bound reference")
    return value


def semantic(value: dict[str, Any]) -> dict[str, Any]:
    return {key: item for key, item in value.items()
            if key not in {"durable_revision", "durable_provenance"}}


def upgrade(value: Any) -> Any:
    legacy_fields = {"schema_version", "identity", "actor_binding", "phase", "status",
                     "pending_obligation", "handled_consequences", "authoritative_refs",
                     "waiting", "retirement"}
    if isinstance(value, dict) and value.get("schema_version") == 1 and set(value) == legacy_fields:
        return {**value, "schema_version": 2, "accepted_boundary": None,
                "latest_phase_consequence": None}
    return value


def decode(stored: Any) -> dict[str, Any]:
    try:
        value = upgrade(json.loads(stored.payload))
    except (UnicodeDecodeError, json.JSONDecodeError) as error:
        fail(f"invalid live slice payload: {error}")
    validate(value)
    return {**value, "durable_revision": stored.revision,
            "durable_provenance": {"adapter": stored.adapter, "location": stored.location,
                                   "integrity_sha256": stored.integrity_sha256}}


def load(store: Any, identity: dict[str, Any]) -> dict[str, Any] | None:
    stored = store.read(stable_key(identity))
    if stored is None:
        return None
    value = decode(stored)
    if value["identity"] != identity:
        fail("durable live slice identity does not match its stable key")
    return value


def publish(store: Any, value: dict[str, Any], revision: str | None) -> dict[str, Any]:
    payload = semantic(value)
    validate(payload)
    return decode(store.publish(stable_key(payload["identity"]), canonical(payload), revision))


def create(store: Any, *, identity: dict[str, Any], actor_binding: dict[str, Any], phase: str,
           pending_obligation: dict[str, Any], authoritative_refs: list[dict[str, str]],
           accepted_boundary: dict[str, str] | None = None) -> dict[str, Any]:
    return publish(store, {"schema_version": 2, "identity": identity, "actor_binding": actor_binding,
        "phase": phase, "status": "active", "pending_obligation": pending_obligation,
        "handled_consequences": [], "authoritative_refs": authoritative_refs,
        "accepted_boundary": accepted_boundary, "latest_phase_consequence": None,
        "waiting": None, "retirement": None}, None)


def transition(store: Any, current: dict[str, Any], event_kind: str, event_id: str,
               transform: Callable[[dict[str, Any]], dict[str, Any]]) -> dict[str, Any]:
    validate(semantic(current))
    if not event_kind or not event_id:
        fail("event_kind and event_id must be nonempty")
    consequence_key = json.dumps([event_kind, event_id], separators=(",", ":"))
    if consequence_key in current["handled_consequences"]:
        return current
    if current["status"] == "retired":
        fail("retired slice attempt cannot transition")
    updated = transform(semantic(current))
    updated["handled_consequences"] = [*updated["handled_consequences"], consequence_key]
    return publish(store, updated, current["durable_revision"])


def wait_on_capability(store: Any, current: dict[str, Any], *, event_id: str,
                       capability: str, provider: str | None, reason: str) -> dict[str, Any]:
    def apply(value: dict[str, Any]) -> dict[str, Any]:
        if value["status"] != "active":
            fail("only active state may wait on capability")
        return {**value, "status": "waiting_on_capability", "waiting": {
            "capability": capability, "provider": provider, "reason": reason,
            "obligation_id": value["pending_obligation"]["obligation_id"]}}
    return transition(store, current, "wait_on_capability", event_id, apply)


def resume_capability(store: Any, current: dict[str, Any], *, event_id: str,
                      capability: str) -> dict[str, Any]:
    def apply(value: dict[str, Any]) -> dict[str, Any]:
        if value["status"] != "waiting_on_capability" or value["waiting"]["capability"] != capability:
            fail("state is not waiting on the specified capability")
        return {**value, "status": "active", "waiting": None}
    return transition(store, current, "resume_capability", event_id, apply)


def publish_phase_consequence(store: Any, current: dict[str, Any], *, phase: str,
                              consequence: dict[str, Any]) -> dict[str, Any]:
    validate(semantic(current))
    validate_phase_consequence(consequence)
    if phase not in {"implementation", "gate"}:
        fail("only implementation or gate phase consequences may be published")
    if current["status"] == "retired":
        fail("retired slice attempt cannot transition")
    digest = hashlib.sha256(canonical(consequence)).hexdigest()
    for encoded in current["handled_consequences"]:
        parts = json.loads(encoded)
        if (len(parts) >= 2 and parts[0] == "publish_phase_consequence"
                and parts[1] == consequence["consequence_id"]):
            if len(parts) == 3 and parts[2] == digest:
                return current
            fail("phase consequence identity conflicts with its durable content")
    if PHASE_ORDER[phase] < PHASE_ORDER[current["phase"]]:
        fail("phase consequence cannot regress the active phase")
    updated = {**semantic(current), "phase": phase,
               "latest_phase_consequence": consequence}
    updated["handled_consequences"] = [*updated["handled_consequences"], json.dumps(
        ["publish_phase_consequence", consequence["consequence_id"], digest],
        separators=(",", ":"))]
    return publish(store, updated, current["durable_revision"])


def projection(value: dict[str, Any]) -> dict[str, Any]:
    validate(semantic(value))
    actor = value["actor_binding"]
    return {
        "schema_version": 1,
        "identity": value["identity"],
        "phase": value["phase"],
        "status": value["status"],
        "pending_obligation": value["pending_obligation"],
        "accepted_boundary": value["accepted_boundary"],
        "latest_phase_consequence": value["latest_phase_consequence"],
        "authoritative_refs": value["authoritative_refs"],
        "waiting": value["waiting"],
        "retirement": value["retirement"],
        "runtime_binding": {"logical_actor_id": actor["logical_actor_id"],
                            "provider": actor["provider"],
                            "runtime_session_id": actor["runtime_session_id"],
                            "semantic": False},
        "durable_revision": value["durable_revision"],
        "durable_provenance": value["durable_provenance"],
    }


def retire(store: Any, current: dict[str, Any], *, event_id: str,
           outcome: str, reason: str) -> dict[str, Any]:
    return transition(store, current, "retire", event_id, lambda value: {
        **value, "status": "retired", "waiting": None,
        "retirement": {"outcome": outcome, "reason": reason}})
