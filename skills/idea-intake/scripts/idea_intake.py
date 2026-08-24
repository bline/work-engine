#!/usr/bin/env python3
"""Validate source-bound idea intake records and emit candidate projections."""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import subprocess
import sys
from pathlib import Path
from typing import Any, NamedTuple


SCHEMA_PATH = Path(__file__).parents[1] / "schemas" / "intake-record-v1.schema.json"
SCHEMA = json.loads(SCHEMA_PATH.read_text(encoding="utf-8"))
OID = re.compile(r"(?:[0-9a-f]{40}|[0-9a-f]{64})")


class IntakeError(ValueError):
    """Raised when a mechanically decidable intake invariant is false."""


class IntakeRecord(NamedTuple):
    record_path: Path
    repository: Path
    record: dict[str, Any]
    source_bytes: bytes


def require(condition: bool, message: str) -> None:
    if not condition:
        raise IntakeError(message)


def obj(value: Any, path: str) -> dict[str, Any]:
    require(isinstance(value, dict), f"{path} must be an object")
    return value


def array(value: Any, path: str) -> list[Any]:
    require(isinstance(value, list), f"{path} must be an array")
    return value


def text(value: Any, path: str) -> str:
    require(isinstance(value, str) and bool(value.strip()), f"{path} must be a nonempty string")
    return value


def exact_keys(value: dict[str, Any], definition: str | None, path: str) -> None:
    node = SCHEMA if definition is None else SCHEMA["$defs"][definition]
    required = set(node["required"])
    properties = set(node["properties"])
    require(node.get("additionalProperties") is False, f"schema definition {definition or 'record'} must remain closed")
    require(required == properties, f"schema definition {definition or 'record'} must require every property")
    require(not required - set(value), f"{path} missing fields: {', '.join(sorted(required - set(value)))}")
    require(not set(value) - properties, f"{path} unknown fields: {', '.join(sorted(set(value) - properties))}")


def unique(values: list[str], path: str) -> None:
    require(len(values) == len(set(values)), f"{path} must contain unique IDs")


def git(repository: Path, *args: str, binary: bool = False) -> bytes | str:
    try:
        result = subprocess.run(
            ["git", "-C", str(repository), *args], check=True, capture_output=True,
            text=not binary,
        )
    except (OSError, subprocess.CalledProcessError) as error:
        detail = getattr(error, "stderr", b"" if binary else "")
        if isinstance(detail, bytes):
            detail = detail.decode("utf-8", errors="replace")
        raise IntakeError(f"git {' '.join(args)} failed: {str(detail).strip() or error}") from error
    return result.stdout


def resolve_local(record_dir: Path, repository: Path, value: Any, path: str) -> Path:
    relative = Path(text(value, path))
    require(not relative.is_absolute(), f"{path} must be relative")
    resolved = (record_dir / relative).resolve()
    try:
        resolved.relative_to(repository.resolve())
    except ValueError as error:
        raise IntakeError(f"{path} escapes the repository") from error
    require(resolved.is_file(), f"{path} does not resolve to a file")
    return resolved


def validate_range(value: Any, source_lines: list[bytes], path: str) -> tuple[int, int, str]:
    value = obj(value, path)
    exact_keys(value, "source_range", path)
    start, end = value["line_start"], value["line_end"]
    require(isinstance(start, int) and not isinstance(start, bool) and start >= 1, f"{path}.line_start is invalid")
    require(isinstance(end, int) and not isinstance(end, bool) and end >= start, f"{path}.line_end is invalid")
    require(end <= len(source_lines), f"{path} exceeds the bound source")
    digest = hashlib.sha256(b"".join(source_lines[start - 1:end])).hexdigest()
    require(value["content_sha256"] == digest, f"{path}.content_sha256 does not match source bytes")
    return start, end, digest


def validate_evidence_binding(item: dict[str, Any], repository: Path, path: str) -> bool:
    verification = obj(item["verification"], f"{path}.verification")
    exact_keys(verification, "evidence_verification", f"{path}.verification")
    mode = verification["mode"]
    require(mode in SCHEMA["$defs"]["evidence_verification"]["properties"]["mode"]["enum"], f"{path}.verification.mode is invalid")
    integrity = text(verification["integrity_sha256"], f"{path}.verification.integrity_sha256")
    require(bool(re.fullmatch(r"[0-9a-f]{64}", integrity)), f"{path}.verification.integrity_sha256 is invalid")
    text(verification["freshness_rule"], f"{path}.verification.freshness_rule")
    if mode == "external_attestation":
        return False
    revision = text(item["revision"], f"{path}.revision")
    require(bool(OID.fullmatch(revision)), f"{path}.revision must be a Git object ID")
    if mode == "repository_file":
        reference = text(item["reference"], f"{path}.reference")
        relative = Path(reference)
        require(not relative.is_absolute() and ".." not in relative.parts, f"{path}.reference must be repository-relative")
        content = bytes(git(repository, "show", f"{revision}:{reference}", binary=True))
    else:
        reference = text(item["reference"], f"{path}.reference")
        require(bool(OID.fullmatch(reference)) and reference == revision, f"{path}.git_object reference must equal revision")
        content = bytes(git(repository, "cat-file", "-p", revision, binary=True))
    require(hashlib.sha256(content).hexdigest() == integrity, f"{path}.verification.integrity_sha256 does not match evidence bytes")
    return True


def validate_record(record_path: Path, repository: Path) -> IntakeRecord:
    repository = repository.resolve()
    record_path = record_path.resolve()
    try:
        record_path.relative_to(repository)
        record = json.loads(record_path.read_text(encoding="utf-8"))
    except (ValueError, OSError, json.JSONDecodeError) as error:
        raise IntakeError(f"cannot read repository-contained record: {error}") from error
    record = obj(record, "record")
    exact_keys(record, None, "record")
    require(record["schema_version"] == 1, "schema_version must be 1")
    require(bool(re.fullmatch(SCHEMA["properties"]["idea_id"]["pattern"], text(record["idea_id"], "idea_id"))), "idea_id is invalid")
    require(bool(re.fullmatch(SCHEMA["properties"]["assessment_id"]["pattern"], text(record["assessment_id"], "assessment_id"))), "assessment_id is invalid")

    source = obj(record["source"], "source")
    exact_keys(source, "source", "source")
    revision = text(source["repository_revision"], "source.repository_revision")
    blob_oid = text(source["blob_oid"], "source.blob_oid")
    require(bool(OID.fullmatch(revision)), "source.repository_revision is invalid")
    require(bool(OID.fullmatch(blob_oid)), "source.blob_oid is invalid")
    source_path = text(source["path"], "source.path")
    require(not Path(source_path).is_absolute() and ".." not in Path(source_path).parts, "source.path must be repository-relative")
    git(repository, "cat-file", "-e", f"{revision}^{{commit}}")
    actual_blob = str(git(repository, "rev-parse", f"{revision}:{source_path}")).strip()
    require(actual_blob == blob_oid, "source.blob_oid does not match revision and path")
    source_bytes = bytes(git(repository, "show", f"{revision}:{source_path}", binary=True))
    source_lines = source_bytes.splitlines(keepends=True)
    source_start, source_end, _ = validate_range(source["range"], source_lines, "source.range")

    assessment = obj(record["assessment"], "assessment")
    exact_keys(assessment, "assessment", "assessment")
    require(isinstance(assessment["revision"], int) and not isinstance(assessment["revision"], bool) and assessment["revision"] >= 1, "assessment.revision is invalid")
    producer = obj(assessment["producer"], "assessment.producer")
    exact_keys(producer, "actor", "assessment.producer")
    text(producer["kind"], "assessment.producer.kind")
    text(producer["id"], "assessment.producer.id")
    authority = obj(assessment["authority"], "assessment.authority")
    exact_keys(authority, "authority", "assessment.authority")
    text(authority["owner"], "assessment.authority.owner")
    text(authority["scope"], "assessment.authority.scope")
    cutoff = obj(assessment["evidence_cutoff"], "assessment.evidence_cutoff")
    exact_keys(cutoff, "evidence_cutoff", "assessment.evidence_cutoff")
    require(bool(OID.fullmatch(text(cutoff["repository_revision"], "assessment.evidence_cutoff.repository_revision"))), "assessment evidence cutoff is invalid")
    git(repository, "cat-file", "-e", f"{cutoff['repository_revision']}^{{commit}}")
    require(assessment["published_state"] in SCHEMA["$defs"]["assessment"]["properties"]["published_state"]["enum"], "assessment.published_state is invalid")
    require(all(isinstance(item, str) and item.strip() for item in array(assessment["reopening_conditions"], "assessment.reopening_conditions")), "assessment.reopening_conditions are invalid")
    narrative = obj(record["narrative"], "narrative")
    exact_keys(narrative, "path_reference", "narrative")
    resolve_local(record_path.parent, repository, narrative["path"], "narrative.path")

    evidence = array(record["evidence"], "evidence")
    evidence_by_id: dict[str, dict[str, Any]] = {}
    mechanically_verified: set[str] = set()
    for index, item in enumerate(evidence):
        item = obj(item, f"evidence[{index}]")
        exact_keys(item, "evidence", f"evidence[{index}]")
        evidence_id = text(item["evidence_id"], f"evidence[{index}].evidence_id")
        require(bool(re.fullmatch(SCHEMA["$defs"]["evidence"]["properties"]["evidence_id"]["pattern"], evidence_id)), f"evidence[{index}].evidence_id is invalid")
        require(evidence_id not in evidence_by_id, "evidence IDs must be unique")
        require(item["kind"] in SCHEMA["$defs"]["evidence"]["properties"]["kind"]["enum"], f"evidence[{index}].kind is invalid")
        for field in ("owner", "reference", "revision", "attribution"):
            text(item[field], f"evidence[{index}].{field}")
        if validate_evidence_binding(item, repository, f"evidence[{index}]"):
            mechanically_verified.add(evidence_id)
        evidence_by_id[evidence_id] = item

    authority_refs = array(authority["evidence_refs"], "assessment.authority.evidence_refs")
    require(bool(authority_refs), "assessment authority requires evidence")
    require(all(ref in evidence_by_id and ref in mechanically_verified and evidence_by_id[ref]["kind"] in {"human_decision", "contract"} for ref in authority_refs), "assessment authority references must resolve to mechanically verified decision or contract evidence")

    claims = array(record["claims"], "claims")
    require(bool(claims), "claims must not be empty")
    claim_by_id: dict[str, dict[str, Any]] = {}
    for index, claim in enumerate(claims):
        claim = obj(claim, f"claims[{index}]")
        exact_keys(claim, "claim", f"claims[{index}]")
        claim_id = text(claim["claim_id"], f"claims[{index}].claim_id")
        require(bool(re.fullmatch(SCHEMA["$defs"]["claim"]["properties"]["claim_id"]["pattern"], claim_id)), f"claims[{index}].claim_id is invalid")
        require(claim_id not in claim_by_id, "claim IDs must be unique")
        require(isinstance(claim["revision"], int) and not isinstance(claim["revision"], bool) and claim["revision"] >= 1, f"claims[{index}].revision is invalid")
        claim_start, claim_end, _ = validate_range(claim["source_range"], source_lines, f"claims[{index}].source_range")
        require(source_start <= claim_start <= claim_end <= source_end, f"claims[{index}].source_range escapes the assessed source range")
        text(claim["statement"], f"claims[{index}].statement")
        require(claim["statement_kind"] in SCHEMA["$defs"]["claim"]["properties"]["statement_kind"]["enum"], f"claims[{index}].statement_kind is invalid")
        disposition = obj(claim["disposition"], f"claims[{index}].disposition")
        expected_disposition = {"state", "status", "rationale", "authority_ref"}
        require(set(disposition) == expected_disposition, f"claims[{index}].disposition fields are invalid")
        require(disposition["state"] in SCHEMA["$defs"]["claim"]["properties"]["disposition"]["properties"]["state"]["enum"], f"claims[{index}].disposition.state is invalid")
        require(disposition["status"] in {"nominated", "adjudicated"}, f"claims[{index}].disposition.status is invalid")
        text(disposition["rationale"], f"claims[{index}].disposition.rationale")
        if disposition["status"] == "adjudicated":
            require(disposition["authority_ref"] in evidence_by_id and disposition["authority_ref"] in mechanically_verified and evidence_by_id[disposition["authority_ref"]]["kind"] in {"human_decision", "contract"}, f"claims[{index}] adjudication requires mechanically verified authority evidence")
        else:
            require(disposition["authority_ref"] is None, f"claims[{index}] nomination cannot claim adjudication authority")
        refs = array(claim["evidence_refs"], f"claims[{index}].evidence_refs")
        require(all(ref in evidence_by_id for ref in refs), f"claims[{index}] has dangling evidence refs")
        require(all(isinstance(item, str) and item.strip() for item in array(claim["uncertainty"], f"claims[{index}].uncertainty")), f"claims[{index}].uncertainty is invalid")
        candidate = claim["candidate"]
        if disposition["state"] == "ready_for_proposal_formation":
            candidate = obj(candidate, f"claims[{index}].candidate")
            exact_keys(candidate, "candidate", f"claims[{index}].candidate")
            for field in ("meaning", "boundary"):
                text(candidate[field], f"claims[{index}].candidate.{field}")
            for field in ("placement_hypotheses", "evidence_still_needed", "unresolved_decisions"):
                require(all(isinstance(item, str) and item.strip() for item in array(candidate[field], f"claims[{index}].candidate.{field}")), f"claims[{index}].candidate.{field} is invalid")
            require(candidate["next_consumer"] == "proposal-former", f"claims[{index}].candidate.next_consumer is invalid")
        else:
            require(candidate is None, f"claims[{index}] candidate is only valid for ready_for_proposal_formation")
        claim_by_id[claim_id] = claim

    relationships = array(record["relationships"], "relationships")
    relationship_by_id: dict[str, dict[str, Any]] = {}
    for index, relationship in enumerate(relationships):
        relationship = obj(relationship, f"relationships[{index}]")
        exact_keys(relationship, "relationship", f"relationships[{index}]")
        relationship_id = text(relationship["relationship_id"], f"relationships[{index}].relationship_id")
        require(bool(re.fullmatch(SCHEMA["$defs"]["relationship"]["properties"]["relationship_id"]["pattern"], relationship_id)), f"relationships[{index}].relationship_id is invalid")
        require(relationship_id not in relationship_by_id, "relationship IDs must be unique")
        require(relationship["type"] in SCHEMA["$defs"]["relationship"]["properties"]["type"]["enum"], f"relationships[{index}].type is invalid")
        require(relationship["source_claim_id"] in claim_by_id, f"relationships[{index}] has dangling source claim")
        target = obj(relationship["target"], f"relationships[{index}].target")
        exact_keys(target, "relationship_target", f"relationships[{index}].target")
        require(target["kind"] in SCHEMA["$defs"]["relationship_target"]["properties"]["kind"]["enum"], f"relationships[{index}].target.kind is invalid")
        text(target["id"], f"relationships[{index}].target.id")
        text(target["reference"], f"relationships[{index}].target.reference")
        if target["kind"] == "claim":
            require(target["id"] in claim_by_id, f"relationships[{index}] has dangling target claim")
        require(relationship["status"] in {"nominated", "adjudicated"}, f"relationships[{index}].status is invalid")
        text(relationship["rationale"], f"relationships[{index}].rationale")
        require(all(ref in evidence_by_id for ref in array(relationship["evidence_refs"], f"relationships[{index}].evidence_refs")), f"relationships[{index}] has dangling evidence refs")
        if relationship["status"] == "adjudicated":
            require(relationship["authority_ref"] in evidence_by_id and relationship["authority_ref"] in mechanically_verified and evidence_by_id[relationship["authority_ref"]]["kind"] in {"human_decision", "contract"}, f"relationships[{index}] adjudication requires mechanically verified authority evidence")
        else:
            require(relationship["authority_ref"] is None, f"relationships[{index}] nomination cannot claim adjudication authority")
        relationship_by_id[relationship_id] = relationship
    for index, claim in enumerate(claims):
        require(all(ref in relationship_by_id for ref in claim["relationship_refs"]), f"claims[{index}] has dangling relationship refs")
        required_relationship = {
            "represented_by_proposal": ("apparently_represented_by_proposal", "proposal"),
            "apparently_implemented": ("apparently_implemented_by", "implementation"),
        }.get(claim["disposition"]["state"])
        if required_relationship:
            relationship_type, target_kind = required_relationship
            linked = [relationship_by_id[ref] for ref in claim["relationship_refs"]]
            require(any(item["type"] == relationship_type and item["target"]["kind"] == target_kind for item in linked), f"claims[{index}] disposition requires a correctly typed relationship")

    proposal_ids: list[str] = []
    for index, proposal in enumerate(array(record["proposal_refs"], "proposal_refs")):
        proposal = obj(proposal, f"proposal_refs[{index}]")
        exact_keys(proposal, "proposal_reference", f"proposal_refs[{index}]")
        proposal_id = text(proposal["proposal_id"], f"proposal_refs[{index}].proposal_id")
        require(bool(re.fullmatch(SCHEMA["$defs"]["proposal_reference"]["properties"]["proposal_id"]["pattern"], proposal_id)), f"proposal_refs[{index}].proposal_id is invalid")
        proposal_ids.append(proposal_id)
        proposal_path = resolve_local(record_path.parent, repository, proposal["path"], f"proposal_refs[{index}].path")
        try:
            proposal_manifest = json.loads(proposal_path.read_text(encoding="utf-8"))
        except (OSError, json.JSONDecodeError) as error:
            raise IntakeError(f"proposal_refs[{index}].path is not readable JSON: {error}") from error
        require(isinstance(proposal_manifest, dict) and proposal_manifest.get("proposal_id") == proposal_id, f"proposal_refs[{index}] does not match the referenced proposal")
        text(proposal["revision"], f"proposal_refs[{index}].revision")
    unique(proposal_ids, "proposal_refs")

    effects = obj(record["non_authorization"], "non_authorization")
    exact_keys(effects, "non_authorization", "non_authorization")
    require(all(value is False for value in effects.values()), "intake record cannot authorize downstream effects")
    require(assessment["published_state"] == "ready_for_handoff" if any(c["disposition"]["state"] == "ready_for_proposal_formation" for c in claims) else assessment["published_state"] != "ready_for_handoff", "assessment state and candidate handoff are inconsistent")
    return IntakeRecord(record_path, repository, record, source_bytes)


def build_projection(validated: IntakeRecord) -> dict[str, Any]:
    record = validated.record
    ready = [claim for claim in record["claims"] if claim["disposition"]["state"] == "ready_for_proposal_formation"]
    used_evidence = {ref for claim in ready for ref in claim["evidence_refs"]}
    used_relationships = {ref for claim in ready for ref in claim["relationship_refs"]}
    return {
        "schema_version": 1,
        "projection_kind": "proposal_formation_candidate",
        "canonical_record": str(validated.record_path.relative_to(validated.repository)),
        "idea_id": record["idea_id"],
        "assessment_id": record["assessment_id"],
        "assessment_revision": record["assessment"]["revision"],
        "source": record["source"],
        "authority": record["assessment"]["authority"],
        "reopening_conditions": record["assessment"]["reopening_conditions"],
        "candidates": ready,
        "relationships": [item for item in record["relationships"] if item["relationship_id"] in used_relationships],
        "evidence": [item for item in record["evidence"] if item["evidence_id"] in used_evidence],
        "proposal_refs": record["proposal_refs"],
        "non_authorization": record["non_authorization"],
    }


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("command", choices=("validate", "project"))
    parser.add_argument("record", type=Path)
    parser.add_argument("--repository", required=True, type=Path)
    args = parser.parse_args(argv)
    try:
        validated = validate_record(args.record, args.repository)
        result = {"status": "valid", "assessment_id": validated.record["assessment_id"]}
        if args.command == "project":
            result["projection"] = build_projection(validated)
    except IntakeError as error:
        print(json.dumps({"status": "invalid", "error": str(error)}, sort_keys=True))
        return 2
    print(json.dumps(result, sort_keys=True))
    return 0


if __name__ == "__main__":
    sys.exit(main())
