#!/usr/bin/env python3
"""Discover and validate Git-backed Work Engine proposal packets."""

from __future__ import annotations

import argparse
import json
import re
import subprocess
import sys
from pathlib import Path
from typing import Any, NamedTuple


SCHEMA_PATH = Path(__file__).parents[1] / "schemas" / "proposal-packet-v1.schema.json"
SCHEMA = json.loads(SCHEMA_PATH.read_text(encoding="utf-8"))
DECISION_SCHEMA_PATH = Path(__file__).parents[1] / "schemas" / "proposal-decision-v1.schema.json"
DECISION_SCHEMA = json.loads(DECISION_SCHEMA_PATH.read_text(encoding="utf-8"))
SCHEMA_VERSION = SCHEMA["properties"]["schema_version"]["const"]
PROPOSAL_ID = re.compile(SCHEMA["properties"]["proposal_id"]["pattern"])
LIFECYCLE_STATES = set(SCHEMA["properties"]["lifecycle_state"]["enum"])
PLACEMENT_STATES = set(SCHEMA["$defs"]["placement"]["properties"]["state"]["enum"])
UNCERTAINTY_STATES = set(SCHEMA["$defs"]["uncertainty"]["properties"]["state"]["enum"])
RELATIONSHIP_TYPES = set(SCHEMA["$defs"]["relationship"]["properties"]["type"]["enum"])


class PacketError(ValueError):
    """Raised when a packet violates a closed, mechanically decidable invariant."""


class Packet(NamedTuple):
    manifest_path: Path
    manifest: dict[str, Any]


def require(condition: bool, message: str) -> None:
    if not condition:
        raise PacketError(message)


def obj(value: Any, path: str) -> dict[str, Any]:
    require(isinstance(value, dict), f"{path} must be an object")
    return value


def array(value: Any, path: str) -> list[Any]:
    require(isinstance(value, list), f"{path} must be an array")
    return value


def text(value: Any, path: str) -> str:
    require(isinstance(value, str) and bool(value.strip()), f"{path} must be a nonempty string")
    return value


def exact_keys(value: dict[str, Any], required: set[str], path: str) -> None:
    missing = required - set(value)
    unknown = set(value) - required
    require(not missing, f"{path} missing fields: {', '.join(sorted(missing))}")
    require(not unknown, f"{path} unknown fields: {', '.join(sorted(unknown))}")


def schema_keys(definition: str | None = None) -> set[str]:
    node = SCHEMA if definition is None else SCHEMA["$defs"][definition]
    require(node.get("additionalProperties") is False,
            f"schema definition {definition or 'packet'} must remain closed")
    required = set(node["required"])
    require(required == set(node["properties"]),
            f"schema definition {definition or 'packet'} must require every property")
    return required


def decision_schema_keys(definition: str | None = None) -> set[str]:
    node = DECISION_SCHEMA if definition is None else DECISION_SCHEMA["$defs"][definition]
    require(node.get("additionalProperties") is False,
            f"decision schema definition {definition or 'decision record'} must remain closed")
    required = set(node["required"])
    require(required == set(node["properties"]),
            f"decision schema definition {definition or 'decision record'} must require every property")
    return required


def resolve_local(packet_dir: Path, repository_root: Path, value: Any, path: str) -> Path:
    relative = Path(text(value, path))
    require(not relative.is_absolute(), f"{path} must be relative")
    resolved = (packet_dir / relative).resolve()
    try:
        resolved.relative_to(repository_root.resolve())
    except ValueError as error:
        raise PacketError(f"{path} escapes the packet repository") from error
    require(resolved.is_file(), f"{path} does not resolve to a file")
    return resolved


def load_packet(manifest_path: Path, repository_root: Path) -> Packet:
    try:
        manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        raise PacketError(f"cannot read {manifest_path}: {error}") from error
    manifest = obj(manifest, str(manifest_path))
    exact_keys(manifest, schema_keys(), "packet")
    require(manifest["schema_version"] == SCHEMA_VERSION,
            f"schema_version must be {SCHEMA_VERSION}")
    proposal_id = text(manifest["proposal_id"], "proposal_id")
    require(bool(PROPOSAL_ID.fullmatch(proposal_id)), "proposal_id has an invalid stable-ID shape")
    text(manifest["family_id"], "family_id")
    text(manifest["title"], "title")
    require(manifest["lifecycle_state"] in LIFECYCLE_STATES, "lifecycle_state is invalid")

    narrative = obj(manifest["narrative"], "narrative")
    exact_keys(narrative, schema_keys("pathReference"), "narrative")
    resolve_local(manifest_path.parent, repository_root, narrative["path"], "narrative.path")

    origins = array(manifest["origin_refs"], "origin_refs")
    require(bool(origins), "origin_refs must not be empty")
    for index, origin_value in enumerate(origins):
        origin = obj(origin_value, f"origin_refs[{index}]")
        exact_keys(origin, schema_keys("originReference"), f"origin_refs[{index}]")
        text(origin["kind"], f"origin_refs[{index}].kind")
        resolve_local(manifest_path.parent, repository_root, origin["path"], f"origin_refs[{index}].path")

    placement = obj(manifest["placement"], "placement")
    exact_keys(placement, schema_keys("placement"), "placement")
    require(placement["state"] in PLACEMENT_STATES, "placement.state is invalid")
    text(placement["claim"], "placement.claim")

    uncertainty = obj(manifest["uncertainty"], "uncertainty")
    exact_keys(uncertainty, schema_keys("uncertainty"), "uncertainty")
    require(uncertainty["state"] in UNCERTAINTY_STATES, "uncertainty.state is invalid")
    conditions = array(uncertainty["reopening_conditions"], "uncertainty.reopening_conditions")
    require(all(isinstance(item, str) and item.strip() for item in conditions),
            "uncertainty.reopening_conditions must contain nonempty strings")

    authority = obj(manifest["authority"], "authority")
    exact_keys(authority, schema_keys("authority"), "authority")
    text(authority["decision_owner"], "authority.decision_owner")
    require(authority["implementation_authorized"] is False,
            "authority.implementation_authorized must be false")

    for index, relationship_value in enumerate(array(manifest["relationships"], "relationships")):
        relationship = obj(relationship_value, f"relationships[{index}]")
        exact_keys(relationship, schema_keys("relationship"),
                   f"relationships[{index}]")
        require(relationship["type"] in RELATIONSHIP_TYPES,
                f"relationships[{index}].type is invalid")
        text(relationship["target_id"], f"relationships[{index}].target_id")
        text(relationship["rationale"], f"relationships[{index}].rationale")
        require(isinstance(relationship["causal"], bool),
                f"relationships[{index}].causal must be boolean")
    return Packet(manifest_path.resolve(), manifest)


def load_decision(decision_path: Path) -> dict[str, Any]:
    try:
        decision = json.loads(decision_path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        raise PacketError(f"cannot read {decision_path}: {error}") from error
    decision = obj(decision, str(decision_path))
    exact_keys(decision, decision_schema_keys(), "decision record")
    require(decision["schema_version"] == DECISION_SCHEMA["properties"]["schema_version"]["const"],
            "decision record schema_version is invalid")
    require(bool(PROPOSAL_ID.fullmatch(text(decision["proposal_id"], "decision.proposal_id"))),
            "decision.proposal_id has an invalid stable-ID shape")

    for section in ("source", "decision", "authority", "constraints"):
        value = obj(decision[section], f"decision.{section}")
        exact_keys(value, decision_schema_keys(section), f"decision.{section}")

    source = decision["source"]
    require(bool(re.fullmatch(r"[0-9a-f]{40}", text(source["repository_revision"],
                                                        "decision.source.repository_revision"))),
            "decision.source.repository_revision must be a full Git object ID")
    require(source["lifecycle_state"] in
            DECISION_SCHEMA["$defs"]["source"]["properties"]["lifecycle_state"]["enum"],
            "decision.source.lifecycle_state is invalid")
    require(source["placement_state"] in PLACEMENT_STATES,
            "decision.source.placement_state is invalid")

    outcome = decision["decision"]
    require(outcome["disposition"] in
            DECISION_SCHEMA["$defs"]["decision"]["properties"]["disposition"]["enum"],
            "decision.disposition is invalid")
    require(outcome["lifecycle_state"] == "decided", "decision.lifecycle_state must be decided")
    require(outcome["placement_state"] in ("uncertain", "probable"),
            "decision.placement_state must remain conditional")
    expected_placement = {
        "defer_for_dogfooding": "uncertain",
        "approve_proposal_meaning": "probable",
    }[outcome["disposition"]]
    require(outcome["placement_state"] == expected_placement,
            "decision disposition and placement state are inconsistent")
    text(outcome["placement_claim"], "decision.placement_claim")
    text(outcome["rationale"], "decision.rationale")
    conditions = array(outcome["reopening_conditions"], "decision.reopening_conditions")
    require(bool(conditions) and all(isinstance(item, str) and item.strip() for item in conditions),
            "decision.reopening_conditions must contain nonempty strings")

    authority = decision["authority"]
    for field in ("decision_owner", "exercised_by", "evidence"):
        text(authority[field], f"decision.authority.{field}")
    constraints = decision["constraints"]
    for field in ("permanent_architecture_settled", "roadmap_priority_changed",
                  "implementation_authorized"):
        require(constraints[field] is False, f"decision.constraints.{field} must be false")
    return decision


def validate_decision_binding(packet: Packet, decision: dict[str, Any], *, transitioned: bool) -> None:
    manifest = packet.manifest
    require(decision["proposal_id"] == manifest["proposal_id"],
            "decision proposal_id does not match packet")
    require(decision["authority"]["decision_owner"] == manifest["authority"]["decision_owner"],
            "decision owner does not match packet authority")
    require(manifest["authority"]["implementation_authorized"] is False,
            "decision transition cannot authorize implementation")
    if transitioned:
        require(manifest["lifecycle_state"] == decision["decision"]["lifecycle_state"],
                "packet lifecycle does not match decision")
        require(manifest["placement"]["state"] == decision["decision"]["placement_state"],
                "packet placement state does not match decision")
        require(manifest["placement"]["claim"] == decision["decision"]["placement_claim"],
                "packet placement claim does not match decision")
        require(manifest["uncertainty"]["reopening_conditions"] ==
                decision["decision"]["reopening_conditions"],
                "packet reopening conditions do not match decision")
    else:
        require(manifest["lifecycle_state"] == decision["source"]["lifecycle_state"],
                "decision source lifecycle is stale")
        require(manifest["placement"]["state"] == decision["source"]["placement_state"],
                "decision source placement is stale")


def transition_packet(manifest_path: Path, decision_input_path: Path,
                      repository_root: Path) -> Path:
    packet = load_packet(manifest_path, repository_root)
    decision = load_decision(decision_input_path)
    validate_decision_binding(packet, decision, transitioned=False)
    try:
        revision = subprocess.run(
            ["git", "-C", str(repository_root), "rev-parse", "HEAD"],
            check=True, capture_output=True, text=True,
        ).stdout.strip()
    except (OSError, subprocess.CalledProcessError) as error:
        raise PacketError("packet repository must have a readable Git HEAD") from error
    require(revision == decision["source"]["repository_revision"],
            "decision source repository revision is stale")
    destination = manifest_path.parent / "decision.json"
    require(not destination.exists(), "packet already has a decision record")

    transitioned = dict(packet.manifest)
    transitioned["placement"] = dict(transitioned["placement"])
    transitioned["uncertainty"] = dict(transitioned["uncertainty"])
    transitioned["lifecycle_state"] = decision["decision"]["lifecycle_state"]
    transitioned["placement"]["state"] = decision["decision"]["placement_state"]
    transitioned["placement"]["claim"] = decision["decision"]["placement_claim"]
    transitioned["uncertainty"]["reopening_conditions"] = decision["decision"]["reopening_conditions"]

    manifest_path.write_text(json.dumps(transitioned, indent=2) + "\n", encoding="utf-8")
    destination.write_text(json.dumps(decision, indent=2) + "\n", encoding="utf-8")
    validate_decision_binding(load_packet(manifest_path, repository_root),
                              load_decision(destination), transitioned=True)
    return destination


def discover_packets(repository_root: Path) -> dict[str, Packet]:
    repository_root = repository_root.resolve()
    require(repository_root.is_dir(), "packet repository must be a directory")
    packets: dict[str, Packet] = {}
    paths = sorted(repository_root.rglob("packet.json"))
    require(bool(paths), "packet repository contains no packet.json manifests")
    for path in paths:
        packet = load_packet(path, repository_root)
        proposal_id = packet.manifest["proposal_id"]
        require(proposal_id not in packets, f"duplicate proposal_id '{proposal_id}'")
        packets[proposal_id] = packet
        decision_path = path.parent / "decision.json"
        if packet.manifest["lifecycle_state"] == "decided":
            require(decision_path.is_file(), f"decided proposal '{proposal_id}' has no decision.json")
            validate_decision_binding(packet, load_decision(decision_path), transitioned=True)
        else:
            require(not decision_path.exists(),
                    f"undecided proposal '{proposal_id}' cannot have a decision.json")
    for proposal_id, packet in packets.items():
        for relationship in packet.manifest["relationships"]:
            target = relationship["target_id"]
            require(target in packets,
                    f"proposal '{proposal_id}' has unresolved target_id '{target}'")
            require(target != proposal_id, f"proposal '{proposal_id}' cannot relate to itself")
    return packets


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    subparsers = parser.add_subparsers(dest="command", required=True)
    validate = subparsers.add_parser("validate", help="discover and validate a packet repository")
    validate.add_argument("repository", type=Path)
    transition = subparsers.add_parser("transition", help="apply an authority-authored decision")
    transition.add_argument("manifest", type=Path)
    transition.add_argument("decision", type=Path)
    transition.add_argument("--repository", required=True, type=Path)
    args = parser.parse_args(argv)
    try:
        if args.command == "transition":
            destination = transition_packet(args.manifest, args.decision, args.repository)
            print(json.dumps({"status": "transitioned", "decision": str(destination)},
                             sort_keys=True))
            return 0
        packets = discover_packets(args.repository)
    except PacketError as error:
        print(json.dumps({"status": "invalid", "error": str(error)}, sort_keys=True))
        return 2
    print(json.dumps({
        "status": "valid",
        "packet_count": len(packets),
        "proposal_ids": sorted(packets),
    }, sort_keys=True))
    return 0


if __name__ == "__main__":
    sys.exit(main())
