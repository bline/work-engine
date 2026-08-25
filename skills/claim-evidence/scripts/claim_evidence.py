#!/usr/bin/env python3
"""Closed, transport-neutral production claim-evidence mechanics."""

from __future__ import annotations

import argparse
import fcntl
import hashlib
import json
import math
import os
import re
import tempfile
from copy import deepcopy
from pathlib import Path
from typing import Any


SCHEMA_VERSION = 1
BUILD_VERSION = "claim-evidence-python-v1"
PROFILES = {"proposal-research-v1", "revision-bound-review-finding-v1"}
LINEAGE = {"refresh", "correction", "supersession", "composition", "derivation", "identity_fork", "retraction"}
REFERENCE_STATUSES = {"verified", "unavailable", "moved_resolvable", "excluded", "integrity_mismatch"}
PERMISSIONS = {"create_claim", "publish_revision", "publish_lineage", "record_reliance", "retire_reliance", "retract_revision"}
STORE_FIELDS = {"schema_version", "projection_boundary", "authorities", "claims", "revisions", "lineage", "reliances", "operations"}


class ClaimEvidenceError(ValueError):
    pass


def canonical(value: Any) -> bytes:
    return (json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=False) + "\n").encode()


def digest(value: Any) -> str:
    return hashlib.sha256(canonical(value)).hexdigest()


def require(condition: bool, message: str) -> None:
    if not condition:
        raise ClaimEvidenceError(message)


def exact_fields(value: dict[str, Any], fields: set[str], label: str) -> None:
    require(isinstance(value, dict) and set(value) == fields, f"{label} has missing or unknown fields")


def nonempty(value: Any, label: str) -> str:
    require(isinstance(value, str) and value, f"{label} must be a nonempty string")
    return value


def string_list(value: Any, label: str, required: bool = False) -> list[str]:
    require(isinstance(value, list) and all(isinstance(item, str) and item for item in value), f"{label} must be a string array")
    require(not required or bool(value), f"{label} must not be empty")
    return value


def validate_transport_safe_json(value: Any, label: str) -> None:
    if value is None or isinstance(value, (bool, str)):
        return
    if isinstance(value, int):
        require(abs(value) <= 2**53 - 1, f"{label} integer is not lossless across JSON transports")
        return
    if isinstance(value, float):
        require(math.isfinite(value), f"{label} number must be finite")
        return
    if isinstance(value, list):
        for index, item in enumerate(value):
            validate_transport_safe_json(item, f"{label}[{index}]")
        return
    if isinstance(value, dict):
        require(all(isinstance(key, str) for key in value), f"{label} object keys must be strings")
        for key, item in value.items():
            validate_transport_safe_json(item, f"{label}.{key}")
        return
    raise ClaimEvidenceError(f"{label} is not a JSON value")


def load(path: Path) -> Any:
    return json.loads(path.read_text())


def atomic_write(path: Path, value: Any) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    data = canonical(value)
    fd, temporary = tempfile.mkstemp(prefix=f".{path.name}.", dir=path.parent)
    try:
        with os.fdopen(fd, "wb") as handle:
            handle.write(data)
            handle.flush()
            os.fsync(handle.fileno())
        os.replace(temporary, path)
    finally:
        if os.path.exists(temporary):
            os.unlink(temporary)


def blank_store(authorities: list[dict[str, Any]] | None = None) -> dict[str, Any]:
    store = {
        "schema_version": SCHEMA_VERSION,
        "projection_boundary": {
            "actual_content_set": "all records in canonical/store.json",
            "source_watermark": None,
            "excluded_inputs": [],
            "failed_inputs": [],
            "freshness": "current_after_verified_rebuild",
            "completeness": "available",
        },
        "authorities": deepcopy(authorities or []), "claims": [], "revisions": [], "lineage": [], "reliances": [], "operations": [],
    }
    validate_store(store)
    return store


def stable_claim_id(subject: dict[str, Any]) -> str:
    exact_fields(subject, {"namespace", "subject_kind", "stable_subject_id", "evidence_baseline", "content_set"}, "subject")
    identity = {key: nonempty(subject[key], f"subject.{key}") for key in ("namespace", "subject_kind", "stable_subject_id")}
    return f"claim-v1@{digest(identity)}"


def validate_reference(ref: dict[str, Any], label: str = "reference") -> None:
    exact_fields(ref, {"owner", "reference", "revision", "integrity_sha256", "freshness", "status"}, label)
    require(ref["status"] in REFERENCE_STATUSES, f"{label} has unknown status")
    for key in ("owner", "reference", "revision", "freshness"):
        nonempty(ref[key], f"{label}.{key}")
    require(isinstance(ref["integrity_sha256"], str) and re.fullmatch(r"[0-9a-f]{64}", ref["integrity_sha256"]) is not None, f"{label} integrity is invalid")


def validate_authority(authority: dict[str, Any]) -> None:
    exact_fields(authority, {"schema_version", "grant_id", "actor", "profile", "permissions", "decision_scope", "authority_reference"}, "authority")
    require(authority["schema_version"] == SCHEMA_VERSION, "unsupported authority version")
    for key in ("grant_id", "actor", "decision_scope"):
        nonempty(authority[key], f"authority.{key}")
    require(authority["profile"] in PROFILES, "unknown authority profile")
    require(isinstance(authority["permissions"], list) and set(authority["permissions"]) <= PERMISSIONS, "unknown authority permission")
    require(len(authority["permissions"]) == len(set(authority["permissions"])), "authority permissions must be unique")
    validate_reference(authority["authority_reference"], "authority reference")
    require(authority["authority_reference"]["status"] == "verified", "authority reference is not verified")


def validate_profile_payload(profile: str, payload: dict[str, Any]) -> None:
    if profile == "proposal-research-v1":
        exact_fields(payload, {"materiality", "support_qualification"}, "proposal research payload")
        nonempty(payload["materiality"], "proposal research materiality"); nonempty(payload["support_qualification"], "proposal research support qualification")
    elif profile == "revision-bound-review-finding-v1":
        exact_fields(payload, {"finding_id", "severity", "episode", "outcome"}, "review finding payload")
        for key in ("finding_id", "severity", "episode", "outcome"):
            nonempty(payload[key], f"review finding {key}")
    else:
        raise ClaimEvidenceError("unknown profile")


def indexes(store: dict[str, Any]) -> dict[str, dict[str, Any]]:
    result: dict[str, dict[str, Any]] = {}
    for collection in ("authorities", "claims", "revisions", "lineage", "reliances", "operations"):
        for record in store[collection]:
            key = record.get("grant_id") if collection == "authorities" else record.get("id", record.get("operation_id"))
            require(isinstance(key, str) and key, f"{collection} record has no identity")
            require(key not in result, f"duplicate record identity: {key}")
            result[key] = record
    return result


def validate_store(store: dict[str, Any]) -> dict[str, dict[str, Any]]:
    exact_fields(store, STORE_FIELDS, "store")
    require(store["schema_version"] == SCHEMA_VERSION, "unsupported store version")
    boundary = store["projection_boundary"]
    exact_fields(boundary, {"actual_content_set", "source_watermark", "excluded_inputs", "failed_inputs", "freshness", "completeness"}, "projection boundary")
    require(boundary["completeness"] in {"available", "partial", "unavailable"}, "unknown completeness")
    nonempty(boundary["actual_content_set"], "projection boundary actual_content_set")
    require(boundary["source_watermark"] is None or isinstance(boundary["source_watermark"], str), "projection source watermark is invalid")
    string_list(boundary["excluded_inputs"], "projection excluded inputs")
    string_list(boundary["failed_inputs"], "projection failed inputs")
    nonempty(boundary["freshness"], "projection freshness")
    require(all(isinstance(store[key], list) for key in STORE_FIELDS - {"schema_version", "projection_boundary"}), "record collections must be arrays")
    for authority in store["authorities"]:
        validate_authority(authority)
    idx = indexes(store)
    authority_by_id = {record["grant_id"]: record for record in store["authorities"]}
    claims = {record["id"]: record for record in store["claims"]}
    revisions = {record["id"]: record for record in store["revisions"]}
    for claim in store["claims"]:
        exact_fields(claim, {"id", "schema_version", "profile", "subject", "statement_identity", "created_by", "authority_ref"}, "claim")
        require(claim["schema_version"] == SCHEMA_VERSION, "unsupported claim version")
        require(claim["id"] == stable_claim_id(claim["subject"]), "claim identity does not match bounded subject")
        require(claim["profile"] in PROFILES and claim["authority_ref"] in authority_by_id, "claim profile or authority is invalid")
        claim_authority = authority_by_id[claim["authority_ref"]]
        require(claim_authority["profile"] == claim["profile"], "claim authority profile is invalid")
        require(claim["created_by"] == claim_authority["actor"], "claim creator does not match authority actor")
        require("create_claim" in claim_authority["permissions"], "claim authority lacks create_claim permission")
        for key in ("statement_identity", "created_by", "authority_ref"):
            nonempty(claim[key], f"claim.{key}")
        validate_reference(claim["subject"]["evidence_baseline"], "subject evidence baseline")
        require(isinstance(claim["subject"]["content_set"], list) and claim["subject"]["content_set"] and all(isinstance(item, str) and item for item in claim["subject"]["content_set"]), "subject content set is invalid")
    for revision in store["revisions"]:
        exact_fields(revision, {"id", "schema_version", "claim_id", "predecessor_revision", "proposition", "support_qualification", "assumptions", "limitations", "confidence", "evidence_references", "sensitivity_references", "producer", "evidence_mode", "judgment_kind", "decision_scope", "profile_payload", "authority_ref", "reopening_conditions", "tombstone"}, "revision")
        require(revision["claim_id"] in claims and revision["authority_ref"] in authority_by_id, "revision has dangling claim or authority")
        require(revision["schema_version"] == SCHEMA_VERSION, "unsupported revision version")
        require(revision["predecessor_revision"] is None or revision["predecessor_revision"] in revisions, "revision predecessor is dangling")
        require(revision["id"] == f"{revision['claim_id']}@{digest({k: v for k, v in revision.items() if k != 'id'})}", "revision identity mismatch")
        validate_transport_safe_json(revision["confidence"], "revision confidence")
        validate_profile_payload(claims[revision["claim_id"]]["profile"], revision["profile_payload"])
        revision_authority = authority_by_id[revision["authority_ref"]]
        require(revision_authority["profile"] == claims[revision["claim_id"]]["profile"], "revision authority profile is invalid")
        require(revision_authority["decision_scope"] == revision["decision_scope"], "revision authority scope is invalid")
        require(revision["producer"] == revision_authority["actor"], "revision producer does not match authority actor")
        for key in ("proposition", "support_qualification", "producer", "evidence_mode", "judgment_kind", "decision_scope", "authority_ref"):
            nonempty(revision[key], f"revision.{key}")
        string_list(revision["assumptions"], "revision assumptions")
        string_list(revision["limitations"], "revision limitations")
        string_list(revision["reopening_conditions"], "revision reopening conditions")
        require(isinstance(revision["evidence_references"], list) and isinstance(revision["sensitivity_references"], list), "revision references must be arrays")
        require(isinstance(revision["tombstone"], bool), "revision tombstone must be boolean")
        for ref in revision["evidence_references"] + revision["sensitivity_references"]:
            validate_reference(ref)
    adjacency: dict[str, set[str]] = {}
    for edge in store["lineage"]:
        exact_fields(edge, {"id", "schema_version", "relationship", "sources", "target", "authority_ref", "operation_id"}, "lineage")
        require(edge["relationship"] in LINEAGE and edge["target"] in revisions, "lineage type or target is invalid")
        require(edge["schema_version"] == SCHEMA_VERSION, "unsupported lineage version")
        require(edge["sources"] and all(source in revisions for source in edge["sources"]), "lineage source is dangling")
        for key in ("id", "authority_ref", "operation_id"):
            nonempty(edge[key], f"lineage.{key}")
        require(edge["authority_ref"] in authority_by_id, "lineage authority is dangling")
        lineage_authority = authority_by_id[edge["authority_ref"]]
        resource_profiles = {claims[revisions[item]["claim_id"]]["profile"] for item in edge["sources"] + [edge["target"]]}
        require(resource_profiles == {lineage_authority["profile"]}, "lineage authority profile is invalid")
        resource_scopes = {revisions[item]["decision_scope"] for item in edge["sources"] + [edge["target"]]}
        require(resource_scopes == {lineage_authority["decision_scope"]}, "lineage authority scope is invalid")
        for source in edge["sources"]:
            adjacency.setdefault(source, set()).add(edge["target"])
    def visit(node: str, active: set[str], done: set[str]) -> None:
        require(node not in active, "cyclic lineage")
        if node in done:
            return
        active.add(node)
        for target in adjacency.get(node, set()):
            visit(target, active, done)
        active.remove(node); done.add(node)
    done: set[str] = set()
    for node in adjacency:
        visit(node, set(), done)
    for reliance in store["reliances"]:
        exact_fields(reliance, {"id", "schema_version", "consumer", "consumer_revision", "decision_scope", "claim_revision_id", "state", "predecessor_reliance", "authority_ref", "operation_id"}, "reliance")
        require(reliance["claim_revision_id"] in revisions, "reliance target is dangling")
        require(reliance["schema_version"] == SCHEMA_VERSION, "unsupported reliance version")
        require(reliance["state"] in {"active", "retired", "superseded"}, "unknown reliance state")
        require(reliance["predecessor_reliance"] is None or reliance["predecessor_reliance"] in idx, "reliance predecessor is dangling")
        require(reliance["authority_ref"] in authority_by_id, "reliance authority is dangling")
        target_profile = claims[revisions[reliance["claim_revision_id"]]["claim_id"]]["profile"]
        require(authority_by_id[reliance["authority_ref"]]["profile"] == target_profile, "reliance authority profile is invalid")
        require(authority_by_id[reliance["authority_ref"]]["decision_scope"] == reliance["decision_scope"], "reliance authority scope is invalid")
        for key in ("id", "consumer", "consumer_revision", "decision_scope", "claim_revision_id", "authority_ref", "operation_id"):
            nonempty(reliance[key], f"reliance.{key}")
    for operation in store["operations"]:
        exact_fields(operation, {"operation_id", "action", "payload_sha256", "result_identity", "authority_ref"}, "operation receipt")
        for key in ("operation_id", "result_identity", "authority_ref"):
            nonempty(operation[key], f"operation receipt.{key}")
        require(operation["action"] in PERMISSIONS, "operation receipt action is invalid")
        require(re.fullmatch(r"[0-9a-f]{64}", operation["payload_sha256"]) is not None, "operation receipt digest is invalid")
        require(operation["authority_ref"] in authority_by_id, "operation receipt authority is dangling")
        operation_authority = authority_by_id[operation["authority_ref"]]
        require(operation["action"] in operation_authority["permissions"], "operation receipt authority lacks action permission")
        result = idx.get(operation["result_identity"])
        require(result is not None and result.get("authority_ref") == operation["authority_ref"], "operation receipt result authority is invalid")
        if operation["action"] in {"create_claim", "publish_revision"}:
            require(operation["result_identity"] in revisions, "operation receipt result type is invalid")
        elif operation["action"] in {"publish_lineage", "retract_revision"}:
            require(operation["result_identity"] in {item["id"] for item in store["lineage"]}, "operation receipt result type is invalid")
        else:
            require(operation["result_identity"] in {item["id"] for item in store["reliances"]}, "operation receipt result type is invalid")
    return idx


def operation_payload_digest(request: dict[str, Any], authority: dict[str, Any]) -> str:
    return digest({"operation": request, "admitted_grant": authority})


def authorize(store: dict[str, Any], authority: dict[str, Any], action: str, profile: str) -> None:
    validate_authority(authority)
    require(authority["profile"] == profile, "authority profile mismatch")
    require(action in authority["permissions"], "unauthorized publication")
    existing = next((item for item in store["authorities"] if item["grant_id"] == authority["grant_id"]), None)
    require(existing is not None, "authority grant was not admitted by the trusted launcher")
    require(existing == authority, "authority grant conflict")


def apply_operation(store: dict[str, Any], request: dict[str, Any], authority: dict[str, Any]) -> dict[str, Any]:
    exact_fields(request, {"schema_version", "operation_id", "action", "profile", "expected_state", "payload"}, "operation")
    require(request["schema_version"] == SCHEMA_VERSION and request["profile"] in PROFILES, "unsupported operation")
    action, payload = request["action"], deepcopy(request["payload"])
    require(action in PERMISSIONS, "unknown operation action")
    nonempty(request["operation_id"], "operation.operation_id")
    authorize(store, authority, action, request["profile"])
    payload_sha = operation_payload_digest(request, authority)
    prior = next((item for item in store["operations"] if item["operation_id"] == request["operation_id"]), None)
    if prior:
        require(prior["payload_sha256"] == payload_sha, "operation identity conflict")
        return {"idempotent": True, "result_identity": prior["result_identity"], "store": store}
    result_identity: str
    if action == "create_claim":
        exact_fields(payload, {"subject", "statement_identity", "initial_revision"}, "create claim payload")
        require(request["expected_state"] is None, "create claim expected state must be null")
        claim_id = stable_claim_id(payload["subject"])
        require(payload["initial_revision"]["decision_scope"] == authority["decision_scope"], "authority decision scope mismatch")
        require(not any(item["id"] == claim_id for item in store["claims"]), "claim identity already exists")
        claim = {"id": claim_id, "schema_version": 1, "profile": request["profile"], "subject": payload["subject"], "statement_identity": payload["statement_identity"], "created_by": authority["actor"], "authority_ref": authority["grant_id"]}
        store["claims"].append(claim)
        revision = make_revision(claim_id, None, payload["initial_revision"], authority)
        store["revisions"].append(revision); result_identity = revision["id"]
    elif action == "publish_revision":
        exact_fields(payload, {"claim_id", "revision"}, "publish revision payload")
        claim = next((item for item in store["claims"] if item["id"] == payload["claim_id"]), None)
        require(claim is not None and claim["profile"] == request["profile"], "claim is missing or profile mismatched")
        require(payload["revision"]["decision_scope"] == authority["decision_scope"], "authority decision scope mismatch")
        predecessor = request["expected_state"]
        heads = revision_heads(store, payload["claim_id"])
        require(predecessor in heads, "conflicting predecessor")
        revision = make_revision(payload["claim_id"], predecessor, payload["revision"], authority)
        require(not any(item["id"] == revision["id"] for item in store["revisions"]), "revision already exists outside idempotent operation")
        store["revisions"].append(revision); result_identity = revision["id"]
    elif action in {"publish_lineage", "retract_revision"}:
        if action == "retract_revision":
            payload = {"relationship": "retraction", **payload}
        exact_fields(payload, {"relationship", "sources", "target"}, "lineage payload")
        require(isinstance(payload["sources"], list) and payload["sources"] and all(isinstance(item, str) and item for item in payload["sources"]), "lineage sources must be a nonempty string array")
        nonempty(payload["target"], "lineage target")
        resource_ids = set(payload["sources"] + [payload["target"]])
        selected_revisions = [item for item in store["revisions"] if item["id"] in resource_ids]
        scoped = {item["decision_scope"] for item in selected_revisions}
        require(scoped == {authority["decision_scope"]}, "authority decision scope mismatch")
        claim_profiles = {item["id"]: item["profile"] for item in store["claims"]}
        resource_profiles = {claim_profiles[item["claim_id"]] for item in selected_revisions}
        require(resource_profiles == {request["profile"]}, "lineage resource profile mismatch")
        edge = {"schema_version": 1, "relationship": payload["relationship"], "sources": payload["sources"], "target": payload["target"], "authority_ref": authority["grant_id"], "operation_id": request["operation_id"]}
        edge["id"] = f"lineage@{digest(edge)}"; store["lineage"].append(edge); result_identity = edge["id"]
    else:
        exact_fields(payload, {"consumer", "consumer_revision", "decision_scope", "claim_revision_id", "state", "predecessor_reliance"}, "reliance payload")
        require(action != "record_reliance" or payload["state"] == "active", "new reliance must be active")
        require(action != "retire_reliance" or payload["state"] in {"retired", "superseded"}, "retirement state is invalid")
        require(payload["decision_scope"] == authority["decision_scope"], "authority decision scope mismatch")
        if action == "record_reliance":
            require(request["expected_state"] is None and payload["predecessor_reliance"] is None, "new reliance cannot claim a predecessor")
        else:
            predecessor = next((item for item in store["reliances"] if item["id"] == payload["predecessor_reliance"]), None)
            require(predecessor is not None and request["expected_state"] == predecessor["id"], "conflicting reliance predecessor")
            require(predecessor["state"] == "active", "only active reliance can be retired")
            require(all(payload[key] == predecessor[key] for key in ("consumer", "consumer_revision", "decision_scope", "claim_revision_id")), "reliance retirement changed its exact scope")
        reliance = {"schema_version": 1, **payload, "authority_ref": authority["grant_id"], "operation_id": request["operation_id"]}
        reliance["id"] = f"reliance@{digest(reliance)}"; store["reliances"].append(reliance); result_identity = reliance["id"]
    store["operations"].append({"operation_id": request["operation_id"], "action": action, "payload_sha256": payload_sha, "result_identity": result_identity, "authority_ref": authority["grant_id"]})
    validate_store(store)
    return {"idempotent": False, "result_identity": result_identity, "store": store}


def make_revision(claim_id: str, predecessor: str | None, payload: dict[str, Any], authority: dict[str, Any]) -> dict[str, Any]:
    fields = {"proposition", "support_qualification", "assumptions", "limitations", "confidence", "evidence_references", "sensitivity_references", "evidence_mode", "judgment_kind", "decision_scope", "profile_payload", "reopening_conditions", "tombstone"}
    exact_fields(payload, fields, "revision payload")
    revision = {"schema_version": 1, "claim_id": claim_id, "predecessor_revision": predecessor, **payload, "producer": authority["actor"], "authority_ref": authority["grant_id"]}
    revision["id"] = f"{claim_id}@{digest(revision)}"
    return revision


def revision_heads(store: dict[str, Any], claim_id: str) -> set[str]:
    revisions = {item["id"] for item in store["revisions"] if item["claim_id"] == claim_id}
    predecessors = {item["predecessor_revision"] for item in store["revisions"] if item["claim_id"] == claim_id and item["predecessor_revision"]}
    return revisions - predecessors


def build_projection(store: dict[str, Any]) -> dict[str, Any]:
    validate_store(store)
    boundary = store["projection_boundary"]
    all_references = [item["authority_reference"] for item in store["authorities"]]
    all_references += [item["subject"]["evidence_baseline"] for item in store["claims"]]
    all_references += [ref for revision in store["revisions"] for ref in revision["evidence_references"] + revision["sensitivity_references"]]
    unresolved = sorted(({"owner": ref["owner"], "reference": ref["reference"], "revision": ref["revision"], "status": ref["status"]} for ref in all_references if ref["status"] != "verified"), key=lambda item: (item["owner"], item["reference"], item["revision"], item["status"]))
    claims = []
    for claim in store["claims"]:
        revisions = [item for item in store["revisions"] if item["claim_id"] == claim["id"]]
        heads = sorted(revision_heads(store, claim["id"]))
        claims.append({**claim, "revision_ids": sorted(item["id"] for item in revisions), "head_revision_ids": heads, "branch_state": "conflict_or_branch" if len(heads) > 1 else "single_head"})
    return {
        "projection_schema_version": 1,
        "build_version": BUILD_VERSION,
        "canonical_input": {"path": "canonical/store.json", "sha256": digest(store), "source_watermark": boundary["source_watermark"]},
        "freshness": boundary["freshness"], "completeness": boundary["completeness"],
        "actual_content_set": boundary["actual_content_set"], "excluded_inputs": boundary["excluded_inputs"], "failed_inputs": boundary["failed_inputs"],
        "unresolved_references": unresolved, "authorities": store["authorities"], "claims": claims, "revisions": store["revisions"], "lineage": store["lineage"], "reliances": store["reliances"],
    }


def discover(projection: dict[str, Any], criteria: dict[str, Any]) -> dict[str, Any]:
    allowed = {"namespace", "subject_kind", "stable_subject_id", "profile", "producer", "support_qualification", "sensitivity_reference", "evidence_baseline", "content_reference", "consumer"}
    require(criteria and set(criteria) <= allowed, "discovery criteria are empty or unknown")
    require(projection["completeness"] != "unavailable", "projection completeness is unavailable")
    revisions = {item["id"]: item for item in projection["revisions"]}
    matches = []
    for claim in projection["claims"]:
        subject = claim["subject"]
        if any(criteria.get(key) != subject.get(key) for key in ("namespace", "subject_kind", "stable_subject_id") if key in criteria): continue
        if "profile" in criteria and criteria["profile"] != claim["profile"]: continue
        if "evidence_baseline" in criteria and criteria["evidence_baseline"] != subject["evidence_baseline"]["reference"]: continue
        if "content_reference" in criteria and criteria["content_reference"] not in subject["content_set"]: continue
        candidate_revisions = [revisions[item] for item in claim["revision_ids"]]
        if "producer" in criteria and not any(item["producer"] == criteria["producer"] for item in candidate_revisions): continue
        if "support_qualification" in criteria and not any(item["support_qualification"] == criteria["support_qualification"] for item in candidate_revisions): continue
        if "sensitivity_reference" in criteria and not any(any(ref["reference"] == criteria["sensitivity_reference"] for ref in item["sensitivity_references"]) for item in candidate_revisions): continue
        if "consumer" in criteria and not any(item["consumer"] == criteria["consumer"] and item["claim_revision_id"] in claim["revision_ids"] for item in projection["reliances"]): continue
        matches.append({"claim": claim, "revisions": candidate_revisions})
    return {"applicability": "not_assessed", "criteria": criteria, "candidates": matches, **projection_context(projection)}


def require_fresh_projection(store: dict[str, Any], projection: dict[str, Any]) -> None:
    require(projection.get("canonical_input", {}).get("sha256") == digest(store), "projection is stale for the canonical store")


def projection_context(projection: dict[str, Any]) -> dict[str, Any]:
    return {
        "projection_schema_version": projection["projection_schema_version"],
        "build_version": projection["build_version"],
        "projection_identity": projection["canonical_input"],
        "freshness": projection["freshness"],
        "completeness": projection["completeness"],
        "actual_content_set": projection["actual_content_set"],
        "excluded_inputs": projection["excluded_inputs"],
        "failed_inputs": projection["failed_inputs"],
        "unresolved_references": projection["unresolved_references"],
    }


def resolve_record(projection: dict[str, Any], identity: str) -> dict[str, Any]:
    claims = [item for item in projection["claims"] if item["id"] == identity]
    revisions = [item for item in projection["revisions"] if item["id"] == identity]
    require(len(claims) + len(revisions) == 1, "claim or revision not found")
    if claims:
        claim = claims[0]
        return {"kind": "claim", "claim": claim, "authority": next(item for item in projection["authorities"] if item["grant_id"] == claim["authority_ref"]), "revisions": [item for item in projection["revisions"] if item["claim_id"] == identity], **projection_context(projection)}
    revision = revisions[0]
    return {"kind": "revision", "revision": revision, "authority": next(item for item in projection["authorities"] if item["grant_id"] == revision["authority_ref"]), "direct_reliance": [item for item in projection["reliances"] if item["claim_revision_id"] == identity], "lineage": [item for item in projection["lineage"] if identity == item["target"] or identity in item["sources"]], **projection_context(projection)}


def traverse(projection: dict[str, Any], revision_id: str, direction: str) -> dict[str, Any]:
    require(direction in {"predecessors", "successors", "both"}, "unknown traversal direction")
    require(any(item["id"] == revision_id for item in projection["revisions"]), "revision not found")
    implicit = [{"id": f"revision-predecessor@{item['id']}", "relationship": "revision_predecessor", "sources": [item["predecessor_revision"]], "target": item["id"], "derived": True} for item in projection["revisions"] if item["predecessor_revision"] is not None]
    relationships = projection["lineage"] + implicit
    seen = {revision_id}; frontier = {revision_id}; edges = []
    while frontier:
        next_frontier: set[str] = set()
        for edge in relationships:
            predecessors = set(edge["sources"])
            successors = {edge["target"]}
            matched = (direction in {"successors", "both"} and frontier & predecessors) or (direction in {"predecessors", "both"} and frontier & successors)
            if matched:
                edges.append(edge)
                candidates = (successors if frontier & predecessors else set()) | (predecessors if frontier & successors else set())
                next_frontier |= candidates - seen
        seen |= next_frontier; frontier = next_frontier
    return {"revision_id": revision_id, "direction": direction, "revision_ids": sorted(seen), "lineage": sorted({item["id"]: item for item in edges}.values(), key=lambda item: item["id"]), **projection_context(projection)}


def query_reliance(projection: dict[str, Any], revision_id: str | None, consumer: str | None) -> dict[str, Any]:
    require(bool(revision_id) != bool(consumer), "supply exactly one reliance query key")
    matches = [item for item in projection["reliances"] if (revision_id and item["claim_revision_id"] == revision_id) or (consumer and item["consumer"] == consumer)]
    return {"query": {"revision_id": revision_id, "consumer": consumer}, "reliances": matches, **projection_context(projection)}


def paths(root: Path) -> tuple[Path, Path]:
    return root / "canonical" / "store.json", root / "generated" / "projection.json"


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--root", type=Path, required=True)
    sub = parser.add_subparsers(dest="command", required=True)
    init_parser = sub.add_parser("init"); init_parser.add_argument("--authority", type=Path, action="append", default=[])
    apply_parser = sub.add_parser("apply"); apply_parser.add_argument("--request", type=Path, required=True); apply_parser.add_argument("--authority", type=Path, required=True)
    sub.add_parser("rebuild"); sub.add_parser("verify")
    discover_parser = sub.add_parser("discover"); discover_parser.add_argument("--criteria-json", required=True)
    resolve_parser = sub.add_parser("resolve"); resolve_parser.add_argument("--identity", required=True)
    traverse_parser = sub.add_parser("traverse"); traverse_parser.add_argument("--revision", required=True); traverse_parser.add_argument("--direction", choices=["predecessors", "successors", "both"], default="both")
    reliance_parser = sub.add_parser("reliance"); reliance_key = reliance_parser.add_mutually_exclusive_group(required=True); reliance_key.add_argument("--revision"); reliance_key.add_argument("--consumer")
    args = parser.parse_args(); store_path, projection_path = paths(args.root)
    try:
        lock_path = store_path.parent / "store.lock"
        exclusive = args.command in {"init", "apply", "rebuild"}
        if exclusive:
            store_path.parent.mkdir(parents=True, exist_ok=True)
        with lock_path.open("a+" if exclusive else "r") as lock:
            fcntl.flock(lock, fcntl.LOCK_EX if exclusive else fcntl.LOCK_SH)
            if args.command == "init":
                require(not store_path.exists(), "store already exists"); store = blank_store([load(path) for path in args.authority]); atomic_write(store_path, store); atomic_write(projection_path, build_projection(store)); result = {"store": str(store_path), "projection": str(projection_path), "canonical_revision": digest(store), "admitted_authority_grants": [item["grant_id"] for item in store["authorities"]]}
            else:
                store = load(store_path); validate_store(store)
                if args.command == "apply":
                    result = apply_operation(store, load(args.request), load(args.authority)); atomic_write(store_path, result.pop("store")); atomic_write(projection_path, build_projection(store)); result["canonical_revision"] = digest(store)
                elif args.command == "rebuild":
                    projection = build_projection(store); atomic_write(projection_path, projection); result = projection["canonical_input"]
                else:
                    require(projection_path.exists(), "projection is missing")
                    projection = load(projection_path); require_fresh_projection(store, projection)
                    if args.command == "verify":
                        expected = build_projection(store); require(projection == expected, "projection is missing or stale"); result = {"valid": True, "canonical_sha256": digest(store), "projection_sha256": digest(expected)}
                    elif args.command == "discover": result = discover(projection, json.loads(args.criteria_json))
                    elif args.command == "resolve": result = resolve_record(projection, args.identity)
                    elif args.command == "traverse": result = traverse(projection, args.revision, args.direction)
                    else: result = query_reliance(projection, args.revision, args.consumer)
            fcntl.flock(lock, fcntl.LOCK_UN)
        print(json.dumps(result, sort_keys=True)); return 0
    except (ClaimEvidenceError, FileNotFoundError, json.JSONDecodeError) as error:
        parser.error(str(error))


if __name__ == "__main__":
    raise SystemExit(main())
