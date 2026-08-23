#!/usr/bin/env python3
"""Validate and project the bounded claim-lineage dogfood records."""

from __future__ import annotations

import argparse
import hashlib
import json
import tempfile
from pathlib import Path
from typing import Any


COLLECTIONS = (
    "authority_grants", "claims", "revisions", "events", "nominations",
    "episodes", "judgments", "reliances", "edges",
)
REQUIRED_TOP = {"schema_version", "run", *COLLECTIONS, "limitations"}
ALLOWED_RELATIONSHIPS = {
    "refreshes", "may_affect", "sourced_by", "reopened_by", "produces",
    "changed_because_of", "relies_on", "contrasts_with",
}
SCHEMA_DEF_BY_TYPE = {
    "authority_grant": "authority", "claim": "claim", "claim_revision": "revision",
    "source_event": "event", "impact_nomination": "nomination",
    "refresh_episode": "episode", "refresh_judgment": "judgment",
    "reliance": "reliance", "lineage_edge": "edge",
}


class RecordError(ValueError):
    pass


def canonical_json(value: Any) -> str:
    return json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=False)


def digest(value: Any) -> str:
    return hashlib.sha256(canonical_json(value).encode()).hexdigest()


def load_json(path: Path) -> Any:
    return json.loads(path.read_text(encoding="utf-8"))


def require(condition: bool, message: str) -> None:
    if not condition:
        raise RecordError(message)


def index_records(data: dict[str, Any]) -> dict[str, dict[str, Any]]:
    index: dict[str, dict[str, Any]] = {}
    for collection in COLLECTIONS:
        require(isinstance(data[collection], list), f"{collection} must be an array")
        for record in data[collection]:
            require(isinstance(record, dict), f"{collection} entries must be objects")
            require(record.get("schema_version") == 1, f"{collection} record has wrong schema version")
            record_id = record.get("id")
            require(isinstance(record_id, str) and record_id, f"{collection} record has no ID")
            require(record_id not in index, f"duplicate record ID: {record_id}")
            index[record_id] = record
    return index


def validate(data: dict[str, Any], schema: dict[str, Any]) -> dict[str, dict[str, Any]]:
    require(isinstance(data, dict), "record document must be an object")
    require(set(data) == REQUIRED_TOP, "record document has missing or unknown top-level fields")
    require(data["schema_version"] == 1, "unsupported record schema version")
    require(schema.get("additionalProperties") is False, "schema must be closed")
    require(set(schema.get("required", [])) == REQUIRED_TOP, "schema top-level contract differs from records")
    run_contract = schema["$defs"]["run"]
    require(run_contract["additionalProperties"] is False, "run schema must be closed")
    require(set(data["run"]) == set(run_contract["properties"]), "run has missing or unknown fields")
    require(data["run"]["selection_classification"] == "purposive_historical_known_outcomes",
            "selection classification must preserve known outcomes")
    require(data["run"]["projection_owner"] == "none_generated_only",
            "projection must not own semantics")
    index = index_records(data)
    for record in index.values():
        definition = SCHEMA_DEF_BY_TYPE.get(record["type"])
        require(definition is not None, f"unknown record type: {record['type']}")
        contract = schema["$defs"][definition]["allOf"][1]
        require(contract["additionalProperties"] is False, f"{definition} schema must be closed")
        require(set(record) == set(contract["properties"]), f"{record['id']} has missing or unknown fields")

    for revision in data["revisions"]:
        require(revision["claim_id"] in index and index[revision["claim_id"]]["type"] == "claim",
                f"revision has missing claim: {revision['id']}")
        require(revision["authority_ref"] in index, f"revision has missing authority: {revision['id']}")
    for edge in data["edges"]:
        require(edge["relationship"] in ALLOWED_RELATIONSHIPS, f"unknown relationship: {edge['id']}")
        require(edge["source"] in index, f"edge source missing: {edge['id']}")
        require(edge["target"] in index, f"edge target missing: {edge['id']}")
        require(index[edge["source"]]["type"] == edge["source_type"], f"edge source type mismatch: {edge['id']}")
        require(index[edge["target"]]["type"] == edge["target_type"], f"edge target type mismatch: {edge['id']}")
        require(edge["source"] != edge["target"], f"self edge: {edge['id']}")
        require(edge["authority_ref"] in index, f"edge authority missing: {edge['id']}")
    for nomination in data["nominations"]:
        require(nomination["source_event_id"] in index, f"nomination source missing: {nomination['id']}")
        require(nomination["target_revision_id"] in index, f"nomination target missing: {nomination['id']}")
    for episode in data["episodes"]:
        require(episode["subject_revision_id"] in index, f"episode subject missing: {episode['id']}")
        require(episode["trigger_nomination_id"] in index, f"episode trigger missing: {episode['id']}")
        require(episode["authority_ref"] in index, f"episode authority missing: {episode['id']}")
    for judgment in data["judgments"]:
        require(judgment["episode_id"] in index, f"judgment episode missing: {judgment['id']}")
        require(judgment["result_revision_id"] in index, f"judgment result missing: {judgment['id']}")
        for event_id in judgment["causal_event_ids"]:
            require(event_id in index and index[event_id]["type"] == "source_event",
                    f"judgment causal event missing: {judgment['id']}")
    for reliance in data["reliances"]:
        require(reliance["claim_revision_id"] in index, f"reliance target missing: {reliance['id']}")
        require(reliance["historicity"] == "prospective_dogfood_reliance",
                f"reliance must be prospective: {reliance['id']}")
        require(reliance["reopening_owner"] == "user", f"reopening owner changed: {reliance['id']}")
    return index


def edges_for(data: dict[str, Any], relationship: str) -> list[dict[str, Any]]:
    return [edge for edge in data["edges"] if edge["relationship"] == relationship]


def build_proofs(data: dict[str, Any], index: dict[str, dict[str, Any]]) -> dict[str, Any]:
    r_claim = "work-engine.dogfood.claim.reviewer-context-retention"
    v_claim = "work-engine.dogfood.finding.review-context-lineage-gap"
    counter = "work-engine.dogfood.policy.reviewer-context-retention"
    completion = "event.reviewer-retention-completion"
    proposal_change = "event.proposal-context-lineage-change"
    revisions = data["revisions"]
    by_claim = {claim: sorted((r for r in revisions if r["claim_id"] == claim), key=lambda r: r["ordinal"])
                for claim in (r_claim, v_claim, counter)}
    changed_edges = edges_for(data, "changed_because_of")
    reliance_edges = edges_for(data, "relies_on")
    may_affect = edges_for(data, "may_affect")

    stable_identity = (
        len(by_claim[r_claim]) == 2 and len(by_claim[v_claim]) == 2
        and all(items[0]["claim_id"] == items[1]["claim_id"] for items in (by_claim[r_claim], by_claim[v_claim]))
        and len(by_claim[counter]) == 1
        and by_claim[counter][0]["claim_id"] != r_claim
        and any(e["source"] == counter and e["target"] == r_claim for e in edges_for(data, "contrasts_with"))
    )
    nomination = (
        len(may_affect) == 2
        and {e["target"] for e in may_affect} == {by_claim[r_claim][0]["id"], by_claim[v_claim][0]["id"]}
        and all(n["source_event_id"] == completion for n in data["nominations"])
        and not any(e["target"] == completion for e in changed_edges)
    )
    refresh = (
        by_claim[r_claim][1]["outcome"] == "retained_unchanged"
        and by_claim[v_claim][1]["outcome"] == "changed"
        and not any(e["source"] == "judgment.refresh-r" for e in changed_edges)
        and len(changed_edges) == 1
        and changed_edges[0]["source"] == "judgment.refresh-v"
        and changed_edges[0]["target"] == proposal_change
    )
    reliance = (
        len(data["reliances"]) == 2 and len(reliance_edges) == 2
        and {e["target"] for e in reliance_edges} == {by_claim[r_claim][1]["id"], by_claim[v_claim][1]["id"]}
        and all(r["reopening_owner"] == "user" and r["historicity"] == "prospective_dogfood_reliance"
                for r in data["reliances"])
    )
    authority = all(record.get("authority_ref") in index for collection in
                    ("revisions", "episodes", "judgments", "reliances", "edges")
                    for record in data[collection])
    publication_topology = any("first published together" in item for item in data["limitations"])
    known_outcome = any("not outcome-independent" in item for item in data["limitations"])
    cells = {
        "stable_identity_and_fork": stable_identity,
        "non_authoritative_may_affect": nomination,
        "authorized_refresh_both_paths": refresh,
        "exact_revision_reliance": reliance,
        "authority_and_provenance": authority,
        "publication_topology_preserved": publication_topology,
        "known_outcome_limitation_preserved": known_outcome,
    }
    return {
        "cells": {name: {"status": "pass" if passed else "fail"} for name, passed in cells.items()},
        "run_valid": all(cells.values()),
        "positive_four_proof_demonstration": all((stable_identity, nomination, refresh, reliance, authority)),
    }


def build_projection(data: dict[str, Any], schema: dict[str, Any]) -> dict[str, Any]:
    index = validate(data, schema)
    return {
        "projection_schema_version": 1,
        "canonical_input_manifest": [{"path": "records/claim-lineage-records.json", "sha256": digest(data)}],
        "record_schema": {"path": "schema/claim-lineage-dogfood-v1.schema.json", "sha256": digest(schema), "version": 1},
        "selection_cutoff": data["run"]["selection_cutoff"],
        "excluded_scope": ["automatic discovery", "transitive propagation", "continuous monitoring", "production storage", "permanent placement"],
        "record_counts": {collection: len(data[collection]) for collection in COLLECTIONS},
        "proofs": build_proofs(data, index),
        "source_ids": sorted(index),
        "completeness_boundary": "only the two pre-bound fixtures and one identity-fork counterfixture",
    }


def render_report(projection: dict[str, Any], data: dict[str, Any]) -> str:
    lines = [
        "# Claim-lineage backbone dogfood evidence report", "",
        "## Result", "",
        f"- Valid run: `{str(projection['proofs']['run_valid']).lower()}`",
        f"- Positive four-proof demonstration: `{str(projection['proofs']['positive_four_proof_demonstration']).lower()}`",
        "- Selection: purposive historical fixtures with known outcomes; not outcome-independent falsification.",
        "- Recommendation: retain the tested minimum as advisory evidence for later real-consumer validation; do not infer permanent placement or parent-proposal acceptance.",
        "", "## Proof cells", "",
    ]
    for name, result in projection["proofs"]["cells"].items():
        lines.append(f"- `{name}`: `{result['status']}`")
    lines.extend(["", "## Projection provenance", "",
                  f"- Canonical input SHA-256: `{projection['canonical_input_manifest'][0]['sha256']}`",
                  f"- Schema SHA-256: `{projection['record_schema']['sha256']}`",
                  f"- Selection cutoff: `{projection['selection_cutoff']}`",
                  f"- Completeness boundary: {projection['completeness_boundary']}",
                  "- The projection is generated and owns no judgment, authority, causality, or completeness beyond these inputs.",
                  "", "## Limitations", ""])
    lines.extend(f"- {item}" for item in data["limitations"])
    lines.extend(["", "## Shared and domain-specific observations", "",
                  "- Shared: stable question identity, immutable evidence baselines, attributed authority, impact nomination, refresh judgment, exact-revision reliance, and typed lineage edges.",
                  "- Research-specific: proposal-formation authority and a dogfood-local retained-unchanged research refresh.",
                  "- Review-specific: diagnostic authority, historical retained-specialist closure, and the non-clean publication topology.",
                  "- The bounded queries required only the rebuildable local projection; no canonical graph or database owner was needed.", ""])
    return "\n".join(lines)


def paths(root: Path) -> tuple[Path, Path, Path, Path]:
    return (root / "records/claim-lineage-records.json",
            root / "schema/claim-lineage-dogfood-v1.schema.json",
            root / "generated/projection.json", root / "evidence-report.md")


def write_outputs(root: Path, output_root: Path) -> None:
    records_path, schema_path, _, _ = paths(root)
    data, schema = load_json(records_path), load_json(schema_path)
    projection = build_projection(data, schema)
    generated = output_root / "generated"
    generated.mkdir(parents=True, exist_ok=True)
    (generated / "projection.json").write_text(json.dumps(projection, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    (output_root / "evidence-report.md").write_text(render_report(projection, data), encoding="utf-8")


def verify(root: Path) -> None:
    _, _, checked_projection, checked_report = paths(root)
    with tempfile.TemporaryDirectory() as directory:
        output_root = Path(directory)
        write_outputs(root, output_root)
        require(checked_projection.read_bytes() == (output_root / "generated/projection.json").read_bytes(),
                "checked projection is stale")
        require(checked_report.read_bytes() == (output_root / "evidence-report.md").read_bytes(),
                "checked evidence report is stale")


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("command", choices=("rebuild", "verify"))
    parser.add_argument("--root", type=Path, required=True)
    parser.add_argument("--out", type=Path)
    args = parser.parse_args()
    root = args.root.resolve()
    if args.command == "rebuild":
        write_outputs(root, (args.out or root).resolve())
    else:
        verify(root)
    print(json.dumps({"command": args.command, "status": "pass", "root": str(root)}, sort_keys=True))


if __name__ == "__main__":
    main()
