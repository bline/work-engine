#!/usr/bin/env python3
"""Authorized durable state for one independent adversarial-review episode."""

from __future__ import annotations

import argparse
import hashlib
import importlib.util
import json
import sys
from datetime import datetime, timezone
from pathlib import Path
from typing import Any

STORE_PATH = Path(__file__).parents[2] / "durable-state/scripts/durable_state.py"
SPEC = importlib.util.spec_from_file_location("review_state_durable", STORE_PATH)
if SPEC is None or SPEC.loader is None:
    raise RuntimeError("unable to load durable-state primitive")
DURABLE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(DURABLE)

PROFILE = "independent-adversarial-review-episode-v1"
FINDING_STATUSES = {"open", "remediation_presented", "verified_resolved", "withdrawn", "unresolved"}
PHASES = {"initial_review", "remediation", "re_evaluation", "reported"}
READERS = {"independent_reviewer", "adversarial_reviewer", "coordinating_builder", "slice_supervisor"}


def fail(message: str) -> None:
    raise ValueError(message)


def canonical(value: object) -> bytes:
    return json.dumps(value, sort_keys=True, separators=(",", ":")).encode()


def digest(value: object) -> str:
    return hashlib.sha256(canonical(value)).hexdigest()


def is_sha256(value: Any) -> bool:
    return isinstance(value, str) and len(value) == 64 and all(c in "0123456789abcdef" for c in value)


def nonempty(value: Any, label: str) -> str:
    if not isinstance(value, str) or not value:
        fail(f"{label} must be a nonempty string")
    return value


def validate_identity(value: Any) -> dict[str, Any]:
    fields = {"run_id", "slice_number", "attempt_id", "plan_version", "review_obligation_id", "review_episode_id"}
    if not isinstance(value, dict) or set(value) != fields:
        fail("review episode identity fields are invalid")
    for field in fields - {"slice_number"}:
        nonempty(value[field], f"identity.{field}")
    if type(value["slice_number"]) is not int or value["slice_number"] < 1:
        fail("identity.slice_number must be positive")
    return value


def validate_reference(value: Any, label: str) -> dict[str, str]:
    fields = {"owner", "reference", "revision", "integrity_sha256", "freshness_rule"}
    if not isinstance(value, dict) or set(value) != fields:
        fail(f"{label} fields are invalid")
    for field in fields - {"integrity_sha256"}:
        nonempty(value[field], f"{label}.{field}")
    if not is_sha256(value["integrity_sha256"]):
        fail(f"{label}.integrity_sha256 must be lowercase SHA-256")
    return value


def validate_references(value: Any, label: str, *, required: bool = False) -> list[dict[str, str]]:
    if not isinstance(value, list) or (required and not value):
        fail(f"{label} must be {'a nonempty' if required else 'an'} array")
    for index, item in enumerate(value):
        validate_reference(item, f"{label}[{index}]")
    if len({canonical(item) for item in value}) != len(value):
        fail(f"{label} must not contain duplicates")
    return value


def validate_writer(value: Any) -> dict[str, Any]:
    fields = {"logical_actor_id", "provider", "generation", "runtime_session_ref"}
    if not isinstance(value, dict) or set(value) != fields:
        fail("writer fields are invalid")
    nonempty(value["logical_actor_id"], "writer.logical_actor_id")
    nonempty(value["provider"], "writer.provider")
    if type(value["generation"]) is not int or value["generation"] < 1:
        fail("writer.generation must be positive")
    validate_reference(value["runtime_session_ref"], "writer.runtime_session_ref")
    return value


def validate_authority(value: Any) -> dict[str, Any]:
    fields = {"schema_version", "profile", "grant_id", "identity", "source", "writer", "readers",
              "initial_subject", "expires_at", "predecessor_revision"}
    if not isinstance(value, dict) or set(value) != fields or value["schema_version"] != 1:
        fail("authority manifest fields or schema are invalid")
    if value["profile"] != PROFILE:
        fail("authority manifest profile is invalid")
    nonempty(value["grant_id"], "authority.grant_id")
    validate_identity(value["identity"])
    validate_reference(value["source"], "authority.source")
    validate_writer(value["writer"])
    if (not isinstance(value["readers"], list) or not value["readers"]
            or any(reader not in READERS for reader in value["readers"])
            or len(set(value["readers"])) != len(value["readers"])):
        fail("authority readers are invalid")
    validate_references(value["initial_subject"], "authority.initial_subject", required=True)
    if value["expires_at"] is not None:
        expires = datetime.fromisoformat(nonempty(value["expires_at"], "authority.expires_at").replace("Z", "+00:00"))
        if expires.tzinfo is None or expires <= datetime.now(timezone.utc):
            fail("authority manifest is expired or lacks a timezone")
    predecessor = value["predecessor_revision"]
    if predecessor is not None and (not isinstance(predecessor, str) or not predecessor):
        fail("authority predecessor_revision is invalid")
    if value["writer"]["generation"] == 1 and predecessor is not None:
        fail("initial authority cannot name a predecessor revision")
    if value["writer"]["generation"] > 1 and predecessor is None:
        fail("successor authority requires predecessor_revision")
    return value


def load_authority(path: Path) -> tuple[dict[str, Any], str]:
    raw = path.read_bytes()
    try:
        value = json.loads(raw)
    except json.JSONDecodeError as error:
        fail(f"invalid authority manifest JSON: {error}")
    validate_authority(value)
    return value, hashlib.sha256(canonical(value)).hexdigest()


def stable_key(identity: dict[str, Any]) -> str:
    validate_identity(identity)
    return "independent-review-episode-v1/" + digest(identity)


def validate_finding(value: Any, label: str) -> dict[str, Any]:
    fields = {"finding_id", "attributed_reviewer", "reviewer_generation", "severity", "observation",
              "evidence_references", "status", "remediation_references"}
    if not isinstance(value, dict) or set(value) != fields:
        fail(f"{label} fields are invalid")
    for field in ("finding_id", "attributed_reviewer", "severity", "observation"):
        nonempty(value[field], f"{label}.{field}")
    if type(value["reviewer_generation"]) is not int or value["reviewer_generation"] < 1:
        fail(f"{label}.reviewer_generation must be positive")
    if value["status"] not in FINDING_STATUSES:
        fail(f"{label}.status is invalid")
    validate_references(value["evidence_references"], f"{label}.evidence_references", required=True)
    validate_references(value["remediation_references"], f"{label}.remediation_references")
    if value["status"] in {"remediation_presented", "verified_resolved"} and not value["remediation_references"]:
        fail(f"{label} remediation status requires an exact remediation reference")
    return value


def validate_findings(value: Any) -> list[dict[str, Any]]:
    if not isinstance(value, list):
        fail("findings must be an array")
    for index, item in enumerate(value):
        validate_finding(item, f"findings[{index}]")
    ids = [item["finding_id"] for item in value]
    if len(ids) != len(set(ids)):
        fail("finding IDs must be unique")
    return value


def validate_state(value: Any) -> dict[str, Any]:
    fields = {"schema_version", "profile", "identity", "authority_binding", "writer_binding", "status",
              "current_review_phase", "reviewed_subject", "findings", "unresolved_questions",
              "pending_next_action", "evidence_references", "claim_references", "handled_transition_ids",
              "uncertainty", "retirement", "continuity"}
    if not isinstance(value, dict) or set(value) != fields or value["schema_version"] != 1 or value["profile"] != PROFILE:
        fail("review state fields or schema are invalid")
    validate_identity(value["identity"])
    authority_fields = {"grant_id", "source", "manifest_sha256", "authorized_readers"}
    if not isinstance(value["authority_binding"], dict) or set(value["authority_binding"]) != authority_fields:
        fail("authority binding fields are invalid")
    nonempty(value["authority_binding"]["grant_id"], "authority_binding.grant_id")
    validate_reference(value["authority_binding"]["source"], "authority_binding.source")
    if not is_sha256(value["authority_binding"]["manifest_sha256"]):
        fail("authority manifest digest is invalid")
    if (not isinstance(value["authority_binding"]["authorized_readers"], list)
            or any(reader not in READERS for reader in value["authority_binding"]["authorized_readers"])):
        fail("authority binding readers are invalid")
    validate_writer(value["writer_binding"])
    if value["status"] not in {"active", "uncertain", "retired"} or value["current_review_phase"] not in PHASES:
        fail("review status or phase is invalid")
    validate_references(value["reviewed_subject"], "reviewed_subject", required=True)
    validate_findings(value["findings"])
    if not isinstance(value["unresolved_questions"], list) or any(not isinstance(x, str) or not x for x in value["unresolved_questions"]):
        fail("unresolved_questions are invalid")
    nonempty(value["pending_next_action"], "pending_next_action")
    validate_references(value["evidence_references"], "evidence_references")
    validate_references(value["claim_references"], "claim_references")
    if not isinstance(value["handled_transition_ids"], dict) or any(not isinstance(k, str) or not k or not is_sha256(v) for k, v in value["handled_transition_ids"].items()):
        fail("handled transition identities are invalid")
    if value["status"] == "uncertain":
        if not isinstance(value["uncertainty"], dict) or set(value["uncertainty"]) != {"reason", "reconciliation_action"}:
            fail("uncertain state requires reconciliation details")
        nonempty(value["uncertainty"]["reason"], "uncertainty.reason")
        nonempty(value["uncertainty"]["reconciliation_action"], "uncertainty.reconciliation_action")
    elif value["uncertainty"] is not None:
        fail("only uncertain state may contain uncertainty")
    if value["status"] == "retired":
        if not isinstance(value["retirement"], dict) or set(value["retirement"]) != {"outcome", "reason", "protected_references"}:
            fail("retired state requires retirement details")
        nonempty(value["retirement"]["outcome"], "retirement.outcome")
        nonempty(value["retirement"]["reason"], "retirement.reason")
        validate_references(value["retirement"]["protected_references"], "retirement.protected_references", required=True)
    elif value["retirement"] is not None:
        fail("only retired state may contain retirement")
    if value["continuity"] not in {"fresh_initial", "same_session", "reconstructed_continuation"}:
        fail("continuity is invalid")
    return value


def binding(authority: dict[str, Any], manifest_sha256: str) -> dict[str, Any]:
    return {"grant_id": authority["grant_id"], "source": authority["source"],
            "manifest_sha256": manifest_sha256, "authorized_readers": authority["readers"]}


def decode(stored: Any) -> dict[str, Any]:
    try:
        value = json.loads(stored.payload)
    except (UnicodeDecodeError, json.JSONDecodeError) as error:
        fail(f"invalid review state payload: {error}")
    validate_state(value)
    return {**value, "durable_revision": stored.revision, "predecessor_revision": stored.predecessor_revision,
            "durable_provenance": {"adapter": stored.adapter, "location": stored.location,
                                   "integrity_sha256": stored.integrity_sha256}}


def semantic(value: dict[str, Any]) -> dict[str, Any]:
    return {key: item for key, item in value.items() if key not in {"durable_revision", "predecessor_revision", "durable_provenance"}}


def load(store: Any, identity: dict[str, Any]) -> dict[str, Any] | None:
    stored = store.read(stable_key(identity))
    return None if stored is None else decode(stored)


def assert_authorized(current: dict[str, Any], authority: dict[str, Any], manifest_sha256: str) -> None:
    if current["identity"] != authority["identity"]:
        fail("authority identity does not match review episode")
    if current["writer_binding"] != authority["writer"] or current["authority_binding"] != binding(authority, manifest_sha256):
        fail("authority manifest does not match the current writer generation")


def publish(store: Any, value: dict[str, Any], expected: str | None) -> dict[str, Any]:
    validate_state(value)
    return decode(store.publish(stable_key(value["identity"]), canonical(value), expected))


def begin(store: Any, authority: dict[str, Any], manifest_sha256: str, request: dict[str, Any]) -> dict[str, Any]:
    if authority["writer"]["generation"] != 1 or load(store, authority["identity"]) is not None:
        fail("review episode already exists or initial writer generation is invalid")
    allowed = {"transition_id", "evidence_references", "claim_references", "unresolved_questions"}
    if not isinstance(request, dict) or set(request) != allowed:
        fail("begin request fields are invalid")
    transition_id = nonempty(request["transition_id"], "transition_id")
    validate_references(request["evidence_references"], "evidence_references")
    validate_references(request["claim_references"], "claim_references")
    if not isinstance(request["unresolved_questions"], list):
        fail("unresolved_questions must be an array")
    transition_digest = digest(request)
    return publish(store, {"schema_version": 1, "profile": PROFILE, "identity": authority["identity"],
        "authority_binding": binding(authority, manifest_sha256), "writer_binding": authority["writer"],
        "status": "active", "current_review_phase": "initial_review",
        "reviewed_subject": authority["initial_subject"], "findings": [],
        "unresolved_questions": request["unresolved_questions"], "pending_next_action": "perform_initial_review",
        "evidence_references": request["evidence_references"], "claim_references": request["claim_references"],
        "handled_transition_ids": {transition_id: transition_digest}, "uncertainty": None,
        "retirement": None, "continuity": "fresh_initial"}, None)


def preserve_finding_lineage(current: list[dict[str, Any]], updated: list[dict[str, Any]]) -> None:
    old = {item["finding_id"]: item for item in current}
    new = {item["finding_id"]: item for item in updated}
    if not set(old).issubset(new):
        fail("findings cannot be deleted")
    for finding_id, prior in old.items():
        item = new[finding_id]
        for field in ("finding_id", "attributed_reviewer", "reviewer_generation", "severity", "observation", "evidence_references"):
            if item[field] != prior[field]:
                fail("existing finding attribution or evidence cannot be rewritten")


def validate_new_finding_attribution(current: list[dict[str, Any]], updated: list[dict[str, Any]],
                                     writer: dict[str, Any]) -> None:
    existing = {item["finding_id"] for item in current}
    for item in updated:
        if (item["finding_id"] not in existing
                and (item["attributed_reviewer"] != writer["logical_actor_id"]
                     or item["reviewer_generation"] != writer["generation"])):
            fail("new finding attribution must match the authorized writer generation")


def advance(store: Any, authority: dict[str, Any], manifest_sha256: str, expected: str,
            action: str, transition_id: str, payload: dict[str, Any]) -> dict[str, Any]:
    current = load(store, authority["identity"])
    transition_digest = digest({"action": action, "payload": payload})
    previous = None if current is None else current["handled_transition_ids"].get(transition_id)
    if previous is not None:
        if previous == transition_digest:
            return current
        fail("transition identity conflicts with durable content")
    if current is None or current["durable_revision"] != expected:
        fail("expected durable revision does not match current review state")
    if current["status"] == "retired":
        fail("retired review episode cannot transition")
    if action == "replace_writer":
        writer = authority["writer"]
        if (writer["generation"] != current["writer_binding"]["generation"] + 1
                or authority["predecessor_revision"] != current["durable_revision"]
                or set(payload) != {"reason", "pending_next_action"}):
            fail("successor writer authority or replacement request is invalid")
        nonempty(payload["reason"], "replacement.reason")
        nonempty(payload["pending_next_action"], "replacement.pending_next_action")
        updated = {**semantic(current), "authority_binding": binding(authority, manifest_sha256),
                   "writer_binding": writer, "status": "active", "uncertainty": None,
                   "pending_next_action": payload["pending_next_action"],
                   "continuity": "reconstructed_continuation"}
    else:
        assert_authorized(current, authority, manifest_sha256)
        if action == "record_initial_result":
            if current["current_review_phase"] != "initial_review" or set(payload) != {"findings", "unresolved_questions", "evidence_references", "claim_references"}:
                fail("initial result transition is invalid")
            validate_findings(payload["findings"])
            if any(finding["status"] not in {"open", "unresolved"}
                   or finding["remediation_references"] for finding in payload["findings"]):
                fail("initial findings must be open or unresolved without remediation references")
            validate_new_finding_attribution([], payload["findings"], authority["writer"])
            validate_references(payload["evidence_references"], "evidence_references")
            validate_references(payload["claim_references"], "claim_references")
            pending = any(f["status"] in {"open", "unresolved"} for f in payload["findings"]) or bool(payload["unresolved_questions"])
            updated = {**semantic(current), **payload, "current_review_phase": "remediation" if pending else "reported",
                       "pending_next_action": "await_remediation" if pending else "return_review_result_to_builder",
                       "continuity": "same_session"}
        elif action == "record_remediation_subject":
            if current["current_review_phase"] != "remediation" or set(payload) != {"reviewed_subject", "evidence_references"}:
                fail("remediation subject transition is invalid")
            validate_references(payload["reviewed_subject"], "reviewed_subject", required=True)
            validate_references(payload["evidence_references"], "evidence_references", required=True)
            updated = {**semantic(current), **payload, "current_review_phase": "re_evaluation",
                       "pending_next_action": "re_evaluate_delta_in_same_session", "continuity": "same_session"}
        elif action == "record_re_evaluation":
            if current["current_review_phase"] != "re_evaluation" or set(payload) != {"findings", "unresolved_questions", "evidence_references", "claim_references"}:
                fail("re-evaluation transition is invalid")
            validate_findings(payload["findings"])
            preserve_finding_lineage(current["findings"], payload["findings"])
            validate_new_finding_attribution(current["findings"], payload["findings"], authority["writer"])
            validate_references(payload["evidence_references"], "evidence_references")
            validate_references(payload["claim_references"], "claim_references")
            pending = any(f["status"] in {"open", "unresolved", "remediation_presented"} for f in payload["findings"]) or bool(payload["unresolved_questions"])
            updated = {**semantic(current), **payload, "current_review_phase": "remediation" if pending else "reported",
                       "pending_next_action": "await_remediation" if pending else "return_review_result_to_builder",
                       "continuity": "same_session"}
        elif action == "mark_uncertain":
            if set(payload) != {"reason", "reconciliation_action"}:
                fail("uncertainty transition is invalid")
            nonempty(payload["reason"], "uncertainty.reason")
            nonempty(payload["reconciliation_action"], "uncertainty.reconciliation_action")
            updated = {**semantic(current), "status": "uncertain", "uncertainty": payload,
                       "pending_next_action": payload["reconciliation_action"]}
        elif action == "retire_episode":
            if set(payload) != {"outcome", "reason", "protected_references"}:
                fail("retirement transition is invalid")
            nonempty(payload["outcome"], "retirement.outcome")
            nonempty(payload["reason"], "retirement.reason")
            validate_references(payload["protected_references"], "retirement.protected_references", required=True)
            updated = {**semantic(current), "status": "retired", "retirement": payload,
                       "pending_next_action": "none_review_episode_retired", "uncertainty": None}
        else:
            fail("review transition action is not authorized")
    updated["handled_transition_ids"] = {**current["handled_transition_ids"], transition_id: transition_digest}
    return publish(store, updated, expected)


def project(value: dict[str, Any]) -> dict[str, Any]:
    return {**value, "field_modes": {"role_owned": ["status", "current_review_phase", "findings", "unresolved_questions", "pending_next_action", "uncertainty", "retirement"],
             "referenced": ["authority_binding", "writer_binding.runtime_session_ref", "reviewed_subject", "evidence_references", "claim_references"],
             "derived": ["durable_revision", "predecessor_revision", "durable_provenance"]},
            "limitations": ["scope verification is not caller authentication", "review state cannot accept a slice or mutate referenced owners"]}


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repository", required=True, type=Path)
    parser.add_argument("--authority-file", required=True, type=Path)
    commands = parser.add_subparsers(dest="command", required=True)
    begin_parser = commands.add_parser("begin")
    begin_parser.add_argument("--request-json", required=True)
    read_parser = commands.add_parser("read")
    read_parser.add_argument("--revision")
    history = commands.add_parser("history")
    history.add_argument("--cursor")
    history.add_argument("--limit", type=int, default=20)
    transition = commands.add_parser("transition")
    transition.add_argument("--expected-revision", required=True)
    transition.add_argument("--action", required=True)
    transition.add_argument("--transition-id", required=True)
    transition.add_argument("--payload-json", required=True)
    args = parser.parse_args()
    try:
        authority, authority_digest = load_authority(args.authority_file)
        store = DURABLE.GitRefDurableStateStore(args.repository, namespace="independent-review-state")
        if args.command == "begin":
            output = begin(store, authority, authority_digest, json.loads(args.request_json))
        elif args.command == "read":
            key = stable_key(authority["identity"])
            if args.revision:
                store.list_revisions(key, args.revision, 1)
                output = decode(store.read_revision(key, args.revision))
            else:
                output = load(store, authority["identity"])
            if output is not None:
                output = project(output)
        elif args.command == "history":
            page = store.list_revisions(stable_key(authority["identity"]), args.cursor, args.limit)
            output = {"items": [project(decode(item)) for item in page.values], "next_cursor": page.next_cursor}
        else:
            output = advance(store, authority, authority_digest, args.expected_revision, args.action,
                             nonempty(args.transition_id, "transition_id"), json.loads(args.payload_json))
            output = project(output)
    except (OSError, RuntimeError, ValueError, json.JSONDecodeError) as error:
        print(f"independent_review_state: {error}", file=sys.stderr)
        return 2
    print(json.dumps(output, sort_keys=True, separators=(",", ":")))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
