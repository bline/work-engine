#!/usr/bin/env python3
"""Discover and validate Git-backed Work Engine proposal packets."""

from __future__ import annotations

import argparse
import base64
import contextlib
import fcntl
import hashlib
import json
import os
import re
import subprocess
import sys
import tempfile
from pathlib import Path
from typing import Any, NamedTuple


SCHEMA_PATH = Path(__file__).parents[1] / "schemas" / "proposal-packet-v1.schema.json"
SCHEMA = json.loads(SCHEMA_PATH.read_text(encoding="utf-8"))
DECISION_SCHEMAS = {
    version: json.loads((Path(__file__).parents[1] / "schemas" /
                         f"proposal-decision-v{version}.schema.json").read_text(encoding="utf-8"))
    for version in (1, 2)
}
HISTORY_DIRECTORY = "decision-history"
# Transition locks and journals are local coordination state, kept out of the
# working tree in the repository's per-worktree Git directory.
TRANSITION_DIRECTORY = "proposal-packets/transitions"
SCHEMA_VERSION = SCHEMA["properties"]["schema_version"]["const"]
PROPOSAL_ID = re.compile(SCHEMA["properties"]["proposal_id"]["pattern"])
LIFECYCLE_STATES = set(SCHEMA["properties"]["lifecycle_state"]["enum"])
PLACEMENT_STATES = set(SCHEMA["$defs"]["placement"]["properties"]["state"]["enum"])
UNCERTAINTY_STATES = set(SCHEMA["$defs"]["uncertainty"]["properties"]["state"]["enum"])
RELATIONSHIP_TYPES = set(SCHEMA["$defs"]["relationship"]["properties"]["type"]["enum"])


class PacketError(ValueError):
    """Raised when a packet violates a closed, mechanically decidable invariant.

    Validation attaches the failure category, owning file and packet, and, where
    they help a repair, the expected and observed values.
    """

    def __init__(self, message: str, **details: Any) -> None:
        super().__init__(message)
        self.details = details

    def fill(self, **details: Any) -> "PacketError":
        for key, value in details.items():
            self.details.setdefault(key, value)
        return self

    def report(self) -> dict[str, Any]:
        return {"invariant": str(self), **self.details}


# Failure categories reported by validation. Each names the violated contract.
MALFORMED_PACKET = "malformed_packet"
MISSING_REFERENCE = "missing_reference"
MALFORMED_DECISION = "malformed_decision"
DECISION_PRESENCE = "decision_presence"
DECISION_BINDING = "decision_binding"
DECISION_HISTORY = "decision_history"
TRANSITION_PENDING = "transition_pending"
DUPLICATE_IDENTITY = "duplicate_identity"
RELATIONSHIP = "relationship"
DUPLICATED_STATE = "duplicated_state"
DISCOVERY = "discovery"
# packet.json owns lifecycle, placement, and authority state. A narrative line
# that declares it again is a second representation that drifts.
NARRATIVE_STATE_LINE = re.compile(
    r"^\s*[-*]\s+(?:\*\*)?(?:State|Status|Lifecycle(?: state)?)(?:\*\*)?\s*:", re.IGNORECASE)


class Packet(NamedTuple):
    manifest_path: Path
    manifest: dict[str, Any]


def require(condition: bool, message: str, **details: Any) -> None:
    if not condition:
        raise PacketError(message, **details)


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


def decision_schema_keys(definition: str | None = None, version: int = 1) -> set[str]:
    schema = DECISION_SCHEMAS[version]
    node = schema if definition is None else schema["$defs"][definition]
    require(node.get("additionalProperties") is False,
            f"decision schema definition {definition or 'decision record'} must remain closed")
    required = set(node["required"])
    require(required == set(node["properties"]),
            f"decision schema definition {definition or 'decision record'} must require every property")
    return required


def resolve_local(packet_dir: Path, repository_root: Path, value: Any, path: str) -> Path:
    relative = Path(text(value, path))
    details = {"category": MISSING_REFERENCE, "observed": str(relative)}
    require(not relative.is_absolute(), f"{path} must be relative", **details)
    resolved = (packet_dir / relative).resolve()
    try:
        resolved.relative_to(repository_root.resolve())
    except ValueError as error:
        raise PacketError(f"{path} escapes the packet repository", **details) from error
    require(resolved.is_file(), f"{path} does not resolve to a file", **details)
    return resolved


def packet_manifest(manifest_path: Path, repository_root: Path) -> Path:
    """Locate a packet.json inside the repository without following any link.

    Paths are checked component by component below the repository root before
    anything is resolved, so a symlinked manifest or packet directory cannot
    redirect validation, transition or recovery to files outside the repository.
    """
    require(".." not in manifest_path.parts, f"{manifest_path} must not traverse parent directories")
    root = repository_root.resolve()
    candidate = Path(os.path.abspath(manifest_path))
    relative = None
    for base in (root, Path(os.path.abspath(repository_root))):
        with contextlib.suppress(ValueError):
            relative = candidate.relative_to(base)
            break
    require(relative is not None and relative.parts != (),
            f"{manifest_path} is outside the packet repository")
    require(relative.name == "packet.json", f"{manifest_path} is not a packet.json manifest")
    path = root
    for part in relative.parts:
        path = path / part
        require(not path.is_symlink(),
                f"{path.relative_to(root)} must not be a symbolic link")
    require(path.is_file(), f"{manifest_path} is not a file")
    return path


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
    version = decision.get("schema_version")
    require(version in DECISION_SCHEMAS, "decision record schema_version is invalid")
    schema = DECISION_SCHEMAS[version]
    exact_keys(decision, decision_schema_keys(version=version), "decision record")
    require(bool(PROPOSAL_ID.fullmatch(text(decision["proposal_id"], "decision.proposal_id"))),
            "decision.proposal_id has an invalid stable-ID shape")

    for section in ("source", "decision", "authority", "constraints"):
        value = obj(decision[section], f"decision.{section}")
        exact_keys(value, decision_schema_keys(section, version), f"decision.{section}")

    source = decision["source"]
    require(bool(re.fullmatch(r"[0-9a-f]{40}", text(source["repository_revision"],
                                                        "decision.source.repository_revision"))),
            "decision.source.repository_revision must be a full Git object ID")
    require(source["lifecycle_state"] in
            schema["$defs"]["source"]["properties"]["lifecycle_state"]["enum"],
            "decision.source.lifecycle_state is invalid")
    require(source["placement_state"] in PLACEMENT_STATES,
            "decision.source.placement_state is invalid")

    outcome = decision["decision"]
    require(outcome["disposition"] in
            schema["$defs"]["decision"]["properties"]["disposition"]["enum"],
            "decision.disposition is invalid")
    require(outcome["lifecycle_state"] == "decided", "decision.lifecycle_state must be decided")
    require(outcome["placement_state"] in ("uncertain", "probable"),
            "decision.placement_state must remain conditional")
    # A placement revision leaves the decided disposition in force and may
    # move placement between the conditional states; first decisions pin it.
    expected_placement = {
        "defer_for_dogfooding": "uncertain",
        "approve_proposal_meaning": "probable",
    }.get(outcome["disposition"])
    require(expected_placement is None or outcome["placement_state"] == expected_placement,
            "decision disposition and placement state are inconsistent")
    revision = outcome["disposition"] == "revise_placement"
    require(revision == (source["lifecycle_state"] == "decided"),
            "only a placement revision may start from a decided proposal")
    if version >= 2:
        supersedes = decision["supersedes"]
        require((supersedes is not None) == revision,
                "a placement revision, and only a placement revision, supersedes a prior decision")
        if supersedes is not None:
            supersedes = obj(supersedes, "decision.supersedes")
            exact_keys(supersedes, decision_schema_keys("supersession", version), "decision.supersedes")
            require(bool(re.fullmatch(rf"{HISTORY_DIRECTORY}/[A-Za-z0-9._-]+\.json",
                                      text(supersedes["path"], "decision.supersedes.path"))),
                    "decision.supersedes.path must name a decision-history record")
            require(bool(re.fullmatch(r"[0-9a-f]{64}",
                                      text(supersedes["sha256"], "decision.supersedes.sha256"))),
                    "decision.supersedes.sha256 must be a SHA-256 digest")
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
        # decision.json is the authority; packet.json duplicates its current
        # outcome, so a mismatch is reported against the packet.
        for message, observed, expected in (
            ("packet lifecycle does not match decision",
             manifest["lifecycle_state"], decision["decision"]["lifecycle_state"]),
            ("packet placement state does not match decision",
             manifest["placement"]["state"], decision["decision"]["placement_state"]),
            ("packet placement claim does not match decision",
             manifest["placement"]["claim"], decision["decision"]["placement_claim"]),
            ("packet reopening conditions do not match decision",
             manifest["uncertainty"]["reopening_conditions"],
             decision["decision"]["reopening_conditions"]),
        ):
            details: dict[str, Any] = {"category": DECISION_BINDING,
                                       "expected": expected, "observed": observed}
            if isinstance(expected, list) and isinstance(observed, list):
                details["missing_from_packet"] = [item for item in expected if item not in observed]
                details["not_in_decision"] = [item for item in observed if item not in expected]
            require(observed == expected, message, **details)
    else:
        require(manifest["lifecycle_state"] == decision["source"]["lifecycle_state"],
                "decision source lifecycle is stale")
        require(manifest["placement"]["state"] == decision["source"]["placement_state"],
                "decision source placement is stale")


def git_output(repository_root: Path, *args: str) -> str:
    try:
        return subprocess.run(
            ["git", "-C", str(repository_root), *args],
            check=True, capture_output=True, text=True,
        ).stdout.strip()
    except (OSError, subprocess.CalledProcessError) as error:
        raise PacketError(f"git {' '.join(args)} failed for {repository_root}") from error


def transition_paths(transition_directory: Path, packet_dir: Path) -> tuple[Path, Path]:
    key = hashlib.sha256(str(packet_dir.resolve()).encode("utf-8")).hexdigest()[:32]
    return transition_directory / f"{key}.lock", transition_directory / f"{key}.json"


def transition_directory(repository_root: Path) -> Path:
    return Path(git_output(repository_root, "rev-parse", "--path-format=absolute",
                           "--git-path", TRANSITION_DIRECTORY))


@contextlib.contextmanager
def transition_lock(lock_path: Path):
    """Serialize every read-validate-write sequence on one packet."""
    lock_path.parent.mkdir(parents=True, exist_ok=True)
    descriptor = os.open(lock_path, os.O_RDWR | os.O_CREAT, 0o600)
    try:
        try:
            fcntl.flock(descriptor, fcntl.LOCK_EX | fcntl.LOCK_NB)
        except BlockingIOError as error:
            raise PacketError("another decision transition holds this packet's lock") from error
        yield
    finally:
        os.close(descriptor)


def packet_target(packet_dir: Path, relative: str) -> Path:
    """Resolve a transition write target without following links out of the packet."""
    require(bool(re.fullmatch(rf"packet\.json|decision\.json|{HISTORY_DIRECTORY}/[A-Za-z0-9._-]+\.json",
                              relative)),
            f"transition target {relative} is not a packet record")
    target = packet_dir / relative
    for path in {target.parent, target}:
        require(not path.is_symlink(), f"{path.relative_to(packet_dir)} must not be a symbolic link")
    return target


def fsync_directory(directory: Path) -> None:
    descriptor = os.open(directory, os.O_RDONLY)
    try:
        os.fsync(descriptor)
    finally:
        os.close(descriptor)


def atomic_write(target: Path, content: bytes) -> None:
    """Replace one file by rename so readers see its old or new bytes, never a mixture."""
    target.parent.mkdir(exist_ok=True)
    descriptor, temporary = tempfile.mkstemp(dir=target.parent, prefix=f".{target.name}.",
                                             suffix=".tmp")
    try:
        with os.fdopen(descriptor, "wb") as handle:
            handle.write(content)
            handle.flush()
            os.fsync(handle.fileno())
        os.replace(temporary, target)
    except BaseException:
        with contextlib.suppress(FileNotFoundError):
            os.unlink(temporary)
        raise
    fsync_directory(target.parent)


def digest(path: Path) -> str | None:
    return hashlib.sha256(path.read_bytes()).hexdigest() if path.exists() else None


def apply_journal(journal: dict[str, Any]) -> None:
    """Roll a journaled transition forward; each write is idempotent and compare-guarded."""
    packet_dir = Path(journal["packet_dir"])
    for write in journal["writes"]:
        target = packet_target(packet_dir, write["path"])
        after = base64.b64decode(write["after"])
        current = digest(target)
        if current == hashlib.sha256(after).hexdigest():
            continue
        require(current == write["before_sha256"],
                f"{write['path']} changed after the transition was journaled; resolve it by hand")
        atomic_write(target, after)


def complete_transition(manifest_path: Path, repository_root: Path,
                        journal_path: Path, journal: dict[str, Any]) -> Path:
    apply_journal(journal)
    destination = manifest_path.parent / "decision.json"
    decision = load_decision(destination)
    validate_decision_binding(load_packet(manifest_path, repository_root), decision, transitioned=True)
    validate_decision_history(manifest_path.parent, decision)
    journal_path.unlink()
    fsync_directory(journal_path.parent)
    return destination


def transition_packet(manifest_path: Path, decision_input_path: Path,
                      repository_root: Path) -> Path:
    """Apply an authority decision as one recoverable transaction.

    Every check runs under the packet's transition lock. The complete post-state
    is journaled before any packet file changes, so an interruption leaves a
    pending journal that validation reports and recover_transition rolls forward.
    """
    manifest_path = packet_manifest(manifest_path, repository_root)
    packet_dir = manifest_path.parent
    lock_path, journal_path = transition_paths(transition_directory(repository_root), packet_dir)
    with transition_lock(lock_path):
        require(not journal_path.exists(),
                "an interrupted transition is pending for this packet; run recover first")
        packet = load_packet(manifest_path, repository_root)
        decision = load_decision(decision_input_path)
        validate_decision_binding(packet, decision, transitioned=False)
        require(git_output(repository_root, "rev-parse", "HEAD") ==
                decision["source"]["repository_revision"],
                "decision source repository revision is stale")
        destination = packet_target(packet_dir, "decision.json")
        writes: list[tuple[str, bytes]] = []
        if decision["source"]["lifecycle_state"] == "decided":
            # The prior record must be the packet's current, valid decision. Its
            # exact bytes move to decision-history so the revision cannot erase
            # what was decided before or by whose authority.
            require(destination.is_file(), "a placement revision needs the packet's current decision")
            prior_bytes = destination.read_bytes()
            prior = load_decision(destination)
            validate_decision_binding(packet, prior, transitioned=True)
            validate_decision_history(packet_dir, prior)
            require(decision["source"]["placement_state"] == prior["decision"]["placement_state"],
                    "decision source placement is stale")
            supersedes = decision["supersedes"]
            require(hashlib.sha256(prior_bytes).hexdigest() == supersedes["sha256"],
                    "decision.supersedes.sha256 does not match the current decision record")
            require(not packet_target(packet_dir, supersedes["path"]).exists(),
                    "decision-history record already exists")
            writes.append((supersedes["path"], prior_bytes))
        else:
            require(not destination.exists(), "packet already has a decision record")

        transitioned = dict(packet.manifest)
        transitioned["placement"] = dict(transitioned["placement"])
        transitioned["uncertainty"] = dict(transitioned["uncertainty"])
        transitioned["lifecycle_state"] = decision["decision"]["lifecycle_state"]
        transitioned["placement"]["state"] = decision["decision"]["placement_state"]
        transitioned["placement"]["claim"] = decision["decision"]["placement_claim"]
        transitioned["uncertainty"]["reopening_conditions"] = decision["decision"]["reopening_conditions"]
        writes.append(("decision.json", (json.dumps(decision, indent=2) + "\n").encode("utf-8")))
        writes.append(("packet.json", (json.dumps(transitioned, indent=2) + "\n").encode("utf-8")))

        journal = {
            "journal_version": 1,
            "packet_dir": str(packet_dir),
            "writes": [{"path": path,
                        "before_sha256": digest(packet_target(packet_dir, path)),
                        "after": base64.b64encode(content).decode("ascii")}
                       for path, content in writes],
        }
        atomic_write(journal_path, json.dumps(journal, indent=2).encode("utf-8"))
        return complete_transition(manifest_path, repository_root, journal_path, journal)


def recover_transition(manifest_path: Path, repository_root: Path) -> Path:
    """Finish an interrupted transition from its journal; never re-decides anything."""
    manifest_path = packet_manifest(manifest_path, repository_root)
    lock_path, journal_path = transition_paths(transition_directory(repository_root),
                                               manifest_path.parent)
    with transition_lock(lock_path):
        require(journal_path.is_file(), "no interrupted transition is pending for this packet")
        journal = json.loads(journal_path.read_text(encoding="utf-8"))
        require(Path(journal["packet_dir"]) == manifest_path.parent,
                "transition journal belongs to another packet")
        return complete_transition(manifest_path, repository_root, journal_path, journal)


def validate_decision_history(packet_dir: Path, decision: dict[str, Any]) -> None:
    """Every superseded decision is retained byte-exact and every archived record is reachable."""
    history = packet_dir / HISTORY_DIRECTORY
    require(not history.is_symlink(), f"{HISTORY_DIRECTORY} must not be a symbolic link")
    reachable: set[Path] = set()
    current = decision
    while current.get("supersedes"):
        supersedes = current["supersedes"]
        path = packet_dir / supersedes["path"]
        require(not path.is_symlink(), f"superseded decision {supersedes['path']} must not be a symbolic link")
        require(path.is_file(), f"superseded decision {supersedes['path']} is missing")
        require(path not in reachable, "decision history contains a cycle")
        require(hashlib.sha256(path.read_bytes()).hexdigest() == supersedes["sha256"],
                f"superseded decision {supersedes['path']} does not match its recorded digest")
        prior = load_decision(path)
        require(prior["proposal_id"] == current["proposal_id"],
                "superseded decision belongs to another proposal")
        require(current["source"]["placement_state"] == prior["decision"]["placement_state"],
                "placement revision source does not match the superseded decision")
        reachable.add(path)
        current = prior
    archived = set(history.glob("*.json")) if history.is_dir() else set()
    require(archived == reachable,
            "decision-history contains records not reachable from the current decision")


def pending_transitions(repository_root: Path) -> Path | None:
    """Locate transition journals; a directory that is not a Git checkout cannot have any."""
    try:
        return transition_directory(repository_root)
    except PacketError:
        return None


class ValidationReport(NamedTuple):
    repository_root: Path
    packet_root: Path
    manifests: list[Path]
    packets: dict[str, Packet]
    failures: list[PacketError]


def validate_narrative_state(packet: Packet, repository_root: Path) -> None:
    narrative = packet.manifest_path.parent / packet.manifest["narrative"]["path"]
    for number, line in enumerate(narrative.read_text(encoding="utf-8").splitlines(), 1):
        require(not NARRATIVE_STATE_LINE.match(line),
                "narrative restates lifecycle state that packet.json owns; remove the line",
                category=DUPLICATED_STATE,
                file=str(narrative.relative_to(repository_root)),
                line=number, observed=line.strip(),
                expected=f"lifecycle_state {packet.manifest['lifecycle_state']!r} only in packet.json")


def validate_packet_decision(packet: Packet, journals: Path | None) -> None:
    path = packet.manifest_path
    proposal_id = packet.manifest["proposal_id"]
    if journals is not None:
        require(not transition_paths(journals, path.parent)[1].exists(),
                f"proposal '{proposal_id}' has an interrupted decision transition; run recover",
                category=TRANSITION_PENDING)
    decision_path = path.parent / "decision.json"
    require(not decision_path.is_symlink(),
            f"proposal '{proposal_id}' decision.json must not be a symbolic link",
            category=MALFORMED_DECISION, file=decision_path)
    if packet.manifest["lifecycle_state"] == "decided":
        require(decision_path.is_file(), f"decided proposal '{proposal_id}' has no decision.json",
                category=DECISION_PRESENCE, file=decision_path)
        try:
            decision = load_decision(decision_path)
        except PacketError as error:
            raise error.fill(category=MALFORMED_DECISION, file=decision_path)
        validate_decision_binding(packet, decision, transitioned=True)
        try:
            validate_decision_history(path.parent, decision)
        except PacketError as error:
            raise error.fill(category=DECISION_HISTORY, file=path.parent / HISTORY_DIRECTORY)
    else:
        require(not decision_path.exists(),
                f"undecided proposal '{proposal_id}' cannot have a decision.json",
                category=DECISION_PRESENCE, file=decision_path,
                observed=packet.manifest["lifecycle_state"])
        require(not (path.parent / HISTORY_DIRECTORY).exists(),
                f"undecided proposal '{proposal_id}' cannot have a decision history",
                category=DECISION_PRESENCE, file=path.parent / HISTORY_DIRECTORY,
                observed=packet.manifest["lifecycle_state"])


def raw_proposal_id(path: Path) -> str | None:
    """Best-effort identity of a packet that failed validation, for relationship checks."""
    with contextlib.suppress(OSError, ValueError, AttributeError):
        value = json.loads(path.read_text(encoding="utf-8")).get("proposal_id")
        return value if isinstance(value, str) else None
    return None


def validate_repository(repository_root: Path, packet_root: Path | None = None) -> ValidationReport:
    """Validate every packet.json under packet_root, collecting each failure.

    Discovery is bounded by packet_root, so unrelated files elsewhere in the
    repository cannot block it. References stay relative to their packet and
    may resolve anywhere inside repository_root. A packet in scope that fails
    to load is reported, never skipped.
    """
    repository_root = repository_root.resolve()
    require(repository_root.is_dir(), f"packet repository {repository_root} must be a directory",
            category=DISCOVERY)
    packet_root = repository_root if packet_root is None else Path(os.path.abspath(packet_root))
    require(packet_root.is_dir(), f"packet root {packet_root} must be a directory", category=DISCOVERY)
    require(packet_root.resolve().is_relative_to(repository_root),
            f"packet root {packet_root} is outside repository {repository_root}", category=DISCOVERY)
    packet_root = packet_root.resolve()
    manifests = sorted(packet_root.rglob("packet.json"))
    require(bool(manifests), f"no packet.json manifests under {packet_root}", category=DISCOVERY)

    def where(path: Path) -> str:
        path = Path(path)
        with contextlib.suppress(ValueError):
            return str(path.resolve().relative_to(repository_root)) if path.is_absolute() else str(path)
        return str(path)

    journals = pending_transitions(repository_root)
    packets: dict[str, Packet] = {}
    failures: list[PacketError] = []
    invalid_ids: set[str] = set()
    for path in manifests:
        label = where(path)
        try:
            path = packet_manifest(path, repository_root)
            packet = load_packet(path, repository_root)
        except PacketError as error:
            failures.append(error.fill(category=MALFORMED_PACKET, packet=label, file=label))
            if (identity := raw_proposal_id(path)) is not None:
                invalid_ids.add(identity)
            continue
        proposal_id = packet.manifest["proposal_id"]
        context = {"packet": label, "proposal_id": proposal_id, "file": label}
        if proposal_id in packets:
            failures.append(PacketError(f"duplicate proposal_id '{proposal_id}'",
                                        category=DUPLICATE_IDENTITY,
                                        observed=[where(packets[proposal_id].manifest_path), label],
                                        **context))
            continue
        packets[proposal_id] = packet
        for check in (lambda: validate_packet_decision(packet, journals),
                      lambda: validate_narrative_state(packet, repository_root)):
            try:
                check()
            except PacketError as error:
                if "file" in error.details:
                    error.details["file"] = where(error.details["file"])
                failures.append(error.fill(**context))
    for proposal_id, packet in packets.items():
        for index, relationship in enumerate(packet.manifest["relationships"]):
            target = relationship["target_id"]
            context = {"category": RELATIONSHIP, "packet": where(packet.manifest_path),
                       "proposal_id": proposal_id, "file": where(packet.manifest_path),
                       "observed": f"relationships[{index}].target_id = {target}"}
            if target == proposal_id:
                failures.append(PacketError(f"proposal '{proposal_id}' cannot relate to itself", **context))
            elif target not in packets and target not in invalid_ids:
                failures.append(PacketError(
                    f"proposal '{proposal_id}' has unresolved target_id '{target}'", **context))
    for error in failures:
        for key in ("file", "packet"):
            if isinstance(error.details.get(key), Path):
                error.details[key] = where(error.details[key])
    return ValidationReport(repository_root, packet_root, manifests, packets, failures)


def discover_packets(repository_root: Path, packet_root: Path | None = None) -> dict[str, Packet]:
    """Return every valid packet, raising the first failure if any packet is invalid."""
    report = validate_repository(repository_root, packet_root)
    if report.failures:
        raise report.failures[0]
    return report.packets


def describe(failure: dict[str, Any]) -> str:
    lines = [f"{failure.get('file') or failure.get('packet') or '<repository>'}"
             f"{':' + str(failure['line']) if 'line' in failure else ''}: "
             f"[{failure.get('category', 'invalid')}] {failure['invariant']}"]
    for key in ("proposal_id", "expected", "observed", "missing_from_packet", "not_in_decision"):
        if key in failure and failure[key] not in ([], None):
            lines.append(f"    {key}: {json.dumps(failure[key], ensure_ascii=False)}")
    return "\n".join(lines)


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    subparsers = parser.add_subparsers(dest="command", required=True)
    validate = subparsers.add_parser(
        "validate", help="discover and validate every packet under a packet root",
        description="Validate every packet.json under PACKET_ROOT. With --repository, "
                    "references resolve anywhere in that repository and PACKET_ROOT "
                    "defaults to its proposals/ directory; without it, PACKET_ROOT is "
                    "also the repository boundary.")
    validate.add_argument("packet_root", type=Path, nargs="?")
    validate.add_argument("--repository", type=Path)
    transition = subparsers.add_parser("transition", help="apply an authority-authored decision")
    transition.add_argument("manifest", type=Path)
    transition.add_argument("decision", type=Path)
    transition.add_argument("--repository", required=True, type=Path)
    recover = subparsers.add_parser("recover", help="finish an interrupted decision transition")
    recover.add_argument("manifest", type=Path)
    recover.add_argument("--repository", required=True, type=Path)
    args = parser.parse_args(argv)
    try:
        if args.command in ("transition", "recover"):
            destination = (transition_packet(args.manifest, args.decision, args.repository)
                           if args.command == "transition"
                           else recover_transition(args.manifest, args.repository))
            print(json.dumps({"status": "transitioned", "decision": str(destination)},
                             sort_keys=True))
            return 0
        if args.repository is None and args.packet_root is None:
            parser.error("validate needs a packet root, --repository, or both")
        repository = args.repository if args.repository is not None else args.packet_root
        packet_root = (args.packet_root if args.packet_root is not None
                       else Path(repository) / "proposals")
        report = validate_repository(repository, packet_root)
    except PacketError as error:
        failure = error.report()
        print(json.dumps({"status": "invalid", "error": str(error), "failures": [failure]},
                         sort_keys=True, default=str))
        print(describe(failure), file=sys.stderr)
        return 2
    summary = {
        "repository": str(report.repository_root),
        "packet_root": str(report.packet_root.relative_to(report.repository_root) or "."),
        "discovered_count": len(report.manifests),
        "packet_count": len(report.packets),
        "decided_count": sum(packet.manifest["lifecycle_state"] == "decided"
                             for packet in report.packets.values()),
        "proposal_ids": sorted(report.packets),
    }
    if report.failures:
        failures = [error.report() for error in report.failures]
        print(json.dumps({"status": "invalid", "error": str(report.failures[0]),
                          "failure_count": len(failures), "failures": failures, **summary},
                         sort_keys=True, default=str))
        print(f"{len(failures)} proposal-packet failure(s) under {summary['packet_root']}:",
              file=sys.stderr)
        for failure in failures:
            print(describe(failure), file=sys.stderr)
        return 2
    print(json.dumps({"status": "valid", **summary}, sort_keys=True))
    return 0


if __name__ == "__main__":
    sys.exit(main())
