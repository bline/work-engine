#!/usr/bin/env python3
"""Prepare and score a blinded linguistic-register recognizability check."""

from __future__ import annotations

import argparse
import hashlib
import json
import math
import sys
from pathlib import Path
from typing import Any

import yaml


SCHEMA_VERSION = 1
LAYERS = {"surface", "discourse"}
SCORE_FIELDS = {
    "internal_coherence", "non_neutral_distinctiveness",
}


class RecognizabilityError(ValueError):
    """Raised when recognizability evidence is malformed or incompatible."""


class UniqueKeyLoader(yaml.SafeLoader):
    pass


def _construct_mapping(loader: UniqueKeyLoader, node: yaml.MappingNode, deep: bool = False) -> dict[str, Any]:
    result: dict[str, Any] = {}
    for key_node, value_node in node.value:
        key = loader.construct_object(key_node, deep=deep)
        if key in result:
            raise RecognizabilityError(f"duplicate YAML key: {key}")
        result[key] = loader.construct_object(value_node, deep=deep)
    return result


UniqueKeyLoader.add_constructor(yaml.resolver.BaseResolver.DEFAULT_MAPPING_TAG, _construct_mapping)


def require(condition: bool, message: str) -> None:
    if not condition:
        raise RecognizabilityError(message)


def obj(value: Any, path: str) -> dict[str, Any]:
    require(isinstance(value, dict), f"{path} must be an object")
    return value


def array(value: Any, path: str) -> list[Any]:
    require(isinstance(value, list), f"{path} must be an array")
    return value


def text(value: Any, path: str) -> str:
    require(isinstance(value, str) and bool(value.strip()), f"{path} must be a nonempty string")
    return value


def exact_keys(value: dict[str, Any], required: set[str], optional: set[str], path: str) -> None:
    missing = required - set(value)
    unknown = set(value) - required - optional
    require(not missing, f"{path} missing fields: {', '.join(sorted(missing))}")
    require(not unknown, f"{path} unknown fields: {', '.join(sorted(unknown))}")


def identifier(value: Any, path: str) -> str:
    result = text(value, path)
    require(result.replace("-", "").replace("_", "").isalnum(),
            f"{path} must contain only letters, digits, hyphens, and underscores")
    return result


def sha256(value: Any, path: str) -> str:
    result = text(value, path)
    require(len(result) == 64 and all(character in "0123456789abcdef" for character in result),
            f"{path} must be lowercase sha256")
    return result


def sha256_file(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def load_yaml(path: Path) -> dict[str, Any]:
    try:
        source = path.read_text(encoding="utf-8")
        for token in yaml.scan(source):
            if isinstance(token, (yaml.tokens.AnchorToken, yaml.tokens.AliasToken)):
                raise RecognizabilityError(f"YAML anchors and aliases are not allowed: {path}")
        return obj(yaml.load(source, Loader=UniqueKeyLoader), str(path))
    except (OSError, yaml.YAMLError) as error:
        raise RecognizabilityError(f"cannot load {path}: {error}") from error


def load_json(path: Path) -> dict[str, Any]:
    try:
        return obj(json.loads(path.read_text(encoding="utf-8")), str(path))
    except (OSError, json.JSONDecodeError) as error:
        raise RecognizabilityError(f"cannot load {path}: {error}") from error


def resolve_bound_path(repository: Path, value: Any, path: str) -> Path:
    recorded = Path(text(value, path))
    require(not recorded.is_absolute() and ".." not in recorded.parts,
            f"{path} must be a repository-relative path without traversal")
    resolved = repository / recorded
    require(resolved.is_file(), f"{path} is not a readable file: {resolved}")
    return resolved


def score(value: Any, path: str) -> int:
    require(isinstance(value, int) and not isinstance(value, bool) and 1 <= value <= 5,
            f"{path} must be an integer from 1 through 5")
    return value


def validate_feature(value: Any, path: str, *, allow_weight: bool) -> dict[str, Any]:
    feature = obj(value, path)
    required = {"layer", "category", "description"}
    if allow_weight:
        required.add("salience_weight")
    exact_keys(feature, required, set(), path)
    require(feature["layer"] in LAYERS, f"{path}.layer is invalid")
    identifier(feature["category"], f"{path}.category")
    text(feature["description"], f"{path}.description")
    if allow_weight:
        weight = feature["salience_weight"]
        require(isinstance(weight, (int, float)) and not isinstance(weight, bool),
                f"{path}.salience_weight must be numeric")
        require(math.isfinite(float(weight)) and float(weight) > 0,
                f"{path}.salience_weight must be finite and positive")
    return feature


def validate_plan(value: Any, plan_path: Path) -> dict[str, Any]:
    plan = obj(value, "plan")
    exact_keys(plan, {
        "artifact_type", "schema_version", "status", "experiment_id", "repository", "role_artifact",
        "profiles", "neutral_decoy", "randomization_seed", "classifier_contract",
        "thresholds", "preregistration_owner", "limitations",
    }, set(), "plan")
    require(plan["artifact_type"] == "linguistic_register_recognizability_plan_v1",
            "plan.artifact_type is incompatible")
    require(plan["schema_version"] == SCHEMA_VERSION, f"plan.schema_version must be {SCHEMA_VERSION}")
    require(plan["status"] == "frozen", "plan.status must be frozen")
    identifier(plan["experiment_id"], "plan.experiment_id")
    repository = Path(text(plan["repository"], "plan.repository"))
    require(repository.is_absolute() and repository.is_dir(),
            "plan.repository must be an existing absolute directory")

    role = obj(plan["role_artifact"], "plan.role_artifact")
    exact_keys(role, {"path", "sha256"}, set(), "plan.role_artifact")
    role_path = resolve_bound_path(repository, role["path"], "plan.role_artifact.path")
    require(sha256_file(role_path) == sha256(role["sha256"], "plan.role_artifact.sha256"),
            "plan.role_artifact.sha256 does not match current bytes")

    profiles = array(plan["profiles"], "plan.profiles")
    require(len(profiles) == 3, "plan.profiles must contain exactly three candidates")
    candidate_ids: set[str] = set()
    profile_digests: set[str] = set()
    for index, profile_value in enumerate(profiles):
        path = f"plan.profiles[{index}]"
        profile = obj(profile_value, path)
        exact_keys(profile, {"candidate_id", "path", "sha256"}, set(), path)
        candidate_id = identifier(profile["candidate_id"], f"{path}.candidate_id")
        require(candidate_id not in candidate_ids, f"duplicate candidate id: {candidate_id}")
        candidate_ids.add(candidate_id)
        profile_path = resolve_bound_path(repository, profile["path"], f"{path}.path")
        digest = sha256(profile["sha256"], f"{path}.sha256")
        require(digest not in profile_digests, f"duplicate profile digest: {digest}")
        profile_digests.add(digest)
        require(sha256_file(profile_path) == digest, f"{path}.sha256 does not match current bytes")
        profile_artifact = load_yaml(profile_path)
        require(profile_artifact.get("artifact_type") == "linguistic_register_profile_v1",
                f"{path}.path is not a profile artifact")
        require(profile_artifact.get("candidate_id") == candidate_id,
                f"{path}.candidate_id does not match profile artifact")
        retained = [item for item in array(profile_artifact.get("features"), f"{path}.features")
                    if obj(item, f"{path}.feature").get("disposition") == "realization_only"]
        require(bool(retained), f"{path} has no retained realization-only features")

    neutral = obj(plan["neutral_decoy"], "plan.neutral_decoy")
    exact_keys(neutral, {"decoy_id", "construction_basis", "features"}, set(), "plan.neutral_decoy")
    identifier(neutral["decoy_id"], "plan.neutral_decoy.decoy_id")
    text(neutral["construction_basis"], "plan.neutral_decoy.construction_basis")
    neutral_features = array(neutral["features"], "plan.neutral_decoy.features")
    require(bool(neutral_features), "plan.neutral_decoy.features must not be empty")
    for index, feature in enumerate(neutral_features):
        validate_feature(feature, f"plan.neutral_decoy.features[{index}]", allow_weight=True)

    text(plan["randomization_seed"], "plan.randomization_seed")
    contract = obj(plan["classifier_contract"], "plan.classifier_contract")
    exact_keys(contract, {
        "task", "blindness", "score_scale", "required_outputs", "prohibited_inference",
    }, set(), "plan.classifier_contract")
    for key in ("task", "blindness", "score_scale", "prohibited_inference"):
        text(contract[key], f"plan.classifier_contract.{key}")
    required_outputs = array(contract["required_outputs"], "plan.classifier_contract.required_outputs")
    require(set(required_outputs) == SCORE_FIELDS | {"pairwise_separation", "most_neutral_card"},
            "plan.classifier_contract.required_outputs is incompatible")

    thresholds = obj(plan["thresholds"], "plan.thresholds")
    exact_keys(thresholds, {
        "minimum_internal_coherence", "minimum_non_neutral_distinctiveness",
        "minimum_separation_from_neutral", "require_correct_neutral_identification",
    }, set(), "plan.thresholds")
    for key in SCORE_FIELDS | {"minimum_separation_from_neutral"}:
        score(thresholds[key if key.startswith("minimum_") else f"minimum_{key}"],
              f"plan.thresholds.minimum_{key.removeprefix('minimum_')}")
    require(thresholds["require_correct_neutral_identification"] is True,
            "plan.thresholds.require_correct_neutral_identification must be true")
    text(plan["preregistration_owner"], "plan.preregistration_owner")
    for index, limitation in enumerate(array(plan["limitations"], "plan.limitations")):
        text(limitation, f"plan.limitations[{index}]")
    return plan


def blinded_order(seed: str, identities: list[str]) -> list[str]:
    return sorted(identities, key=lambda identity: hashlib.sha256(f"{seed}\0{identity}".encode()).hexdigest())


def prepare(plan: dict[str, Any], plan_path: Path) -> tuple[dict[str, Any], dict[str, Any]]:
    cards_by_identity: dict[str, list[dict[str, Any]]] = {}
    bindings: dict[str, str] = {}
    for profile_binding in plan["profiles"]:
        profile_path = resolve_bound_path(Path(plan["repository"]), profile_binding["path"], "plan.profiles.path")
        profile = load_yaml(profile_path)
        retained = []
        for feature in profile["features"]:
            if feature["disposition"] == "realization_only":
                retained.append({
                    "layer": feature["layer"],
                    "category": feature["category"],
                    "description": feature["description"],
                    "salience_weight": feature["distinctiveness_weight"],
                })
        cards_by_identity[profile_binding["candidate_id"]] = retained
        bindings[profile_binding["candidate_id"]] = profile_binding["sha256"]
    decoy_id = plan["neutral_decoy"]["decoy_id"]
    cards_by_identity[decoy_id] = plan["neutral_decoy"]["features"]
    ordered = blinded_order(plan["randomization_seed"], list(cards_by_identity))
    mapping = {f"R{index:02d}": identity for index, identity in enumerate(ordered, start=1)}
    cards = [{
        "anonymous_card_id": card_id,
        "feature_count": len(cards_by_identity[identity]),
        "features": cards_by_identity[identity],
    } for card_id, identity in mapping.items()]
    plan_digest = sha256_file(plan_path)
    packet = {
        "artifact_type": "linguistic_register_blinded_packet_v1",
        "schema_version": SCHEMA_VERSION,
        "experiment_id": plan["experiment_id"],
        "plan_artifact_sha256": plan_digest,
        "classifier_contract": plan["classifier_contract"],
        "cards": cards,
        "authority": "blinded retained-profile recognizability pre-check input; no candidate selection authority",
    }
    packet_digest = hashlib.sha256((json.dumps(packet, indent=2, sort_keys=True) + "\n").encode()).hexdigest()
    key = {
        "artifact_type": "linguistic_register_blinding_key_v1",
        "schema_version": SCHEMA_VERSION,
        "experiment_id": plan["experiment_id"],
        "plan_artifact_sha256": plan_digest,
        "packet_artifact_sha256": packet_digest,
        "anonymous_mapping": mapping,
        "neutral_decoy_id": decoy_id,
        "profile_artifact_sha256_by_candidate": bindings,
        "authority": "unblinding and mechanical scoring only; no candidate selection authority",
    }
    return packet, key


def validate_prepared_artifacts(plan: dict[str, Any], plan_path: Path, packet: dict[str, Any],
                                key: dict[str, Any] | None = None) -> None:
    expected_packet, expected_key = prepare(plan, plan_path)
    require(packet == expected_packet,
            "packet does not exactly match deterministic preparation from the frozen plan")
    if key is not None:
        require(key == expected_key,
                "blinding key does not exactly match deterministic preparation from the frozen plan")


def validate_result(value: Any, packet: dict[str, Any], packet_digest: str) -> dict[str, Any]:
    result = obj(value, "result")
    exact_keys(result, {
        "artifact_type", "schema_version", "experiment_id", "packet_artifact_sha256",
        "classifier_provenance", "card_scores", "pairwise_separation",
        "most_neutral_card_id", "method_limitations",
    }, set(), "result")
    require(result["artifact_type"] == "linguistic_register_classifier_result_v1",
            "result.artifact_type is incompatible")
    require(result["schema_version"] == SCHEMA_VERSION, f"result.schema_version must be {SCHEMA_VERSION}")
    require(result["experiment_id"] == packet["experiment_id"], "result.experiment_id does not match packet")
    require(sha256(result["packet_artifact_sha256"], "result.packet_artifact_sha256") == packet_digest,
            "result.packet_artifact_sha256 does not match packet")
    provenance = obj(result["classifier_provenance"], "result.classifier_provenance")
    exact_keys(provenance, {
        "provider", "model", "reasoning_effort", "execution_mode", "fresh_context",
        "packet_only", "started_at_utc", "completed_at_utc",
    }, set(), "result.classifier_provenance")
    for key in ("provider", "model", "reasoning_effort", "execution_mode", "started_at_utc", "completed_at_utc"):
        text(provenance[key], f"result.classifier_provenance.{key}")
    require(provenance["fresh_context"] is True, "result.classifier_provenance.fresh_context must be true")
    require(provenance["packet_only"] is True, "result.classifier_provenance.packet_only must be true")

    card_ids = {card["anonymous_card_id"] for card in packet["cards"]}
    scores = array(result["card_scores"], "result.card_scores")
    require(len(scores) == len(card_ids), "result.card_scores must score every card exactly once")
    seen: set[str] = set()
    for index, score_value in enumerate(scores):
        path = f"result.card_scores[{index}]"
        card_score = obj(score_value, path)
        exact_keys(card_score, {
            "anonymous_card_id", "internal_coherence", "non_neutral_distinctiveness", "rationale",
        }, set(), path)
        card_id = text(card_score["anonymous_card_id"], f"{path}.anonymous_card_id")
        require(card_id in card_ids and card_id not in seen, f"{path}.anonymous_card_id is unknown or duplicated")
        seen.add(card_id)
        score(card_score["internal_coherence"], f"{path}.internal_coherence")
        score(card_score["non_neutral_distinctiveness"], f"{path}.non_neutral_distinctiveness")
        text(card_score["rationale"], f"{path}.rationale")

    pairs = array(result["pairwise_separation"], "result.pairwise_separation")
    expected_pairs = {tuple(sorted((left, right))) for left in card_ids for right in card_ids if left < right}
    seen_pairs: set[tuple[str, str]] = set()
    for index, pair_value in enumerate(pairs):
        path = f"result.pairwise_separation[{index}]"
        pair = obj(pair_value, path)
        exact_keys(pair, {"card_ids", "score", "rationale"}, set(), path)
        pair_ids = array(pair["card_ids"], f"{path}.card_ids")
        require(len(pair_ids) == 2 and len(set(pair_ids)) == 2 and set(pair_ids) <= card_ids,
                f"{path}.card_ids must identify two different known cards")
        normalized = tuple(sorted(pair_ids))
        require(normalized not in seen_pairs, f"duplicate pairwise score: {normalized}")
        seen_pairs.add(normalized)
        score(pair["score"], f"{path}.score")
        text(pair["rationale"], f"{path}.rationale")
    require(seen_pairs == expected_pairs, "result.pairwise_separation must score every unordered pair exactly once")
    require(result["most_neutral_card_id"] in card_ids, "result.most_neutral_card_id is unknown")
    for index, limitation in enumerate(array(result["method_limitations"], "result.method_limitations")):
        text(limitation, f"result.method_limitations[{index}]")
    return result


def evaluate(plan: dict[str, Any], key: dict[str, Any], result: dict[str, Any],
             key_digest: str, result_digest: str) -> dict[str, Any]:
    mapping = key["anonymous_mapping"]
    inverse = {identity: card_id for card_id, identity in mapping.items()}
    neutral_id = key["neutral_decoy_id"]
    neutral_card = inverse[neutral_id]
    score_by_card = {item["anonymous_card_id"]: item for item in result["card_scores"]}
    pair_by_ids = {tuple(sorted(item["card_ids"])): item for item in result["pairwise_separation"]}
    thresholds = plan["thresholds"]
    candidate_results = []
    for candidate_id in sorted(identity for identity in inverse if identity != neutral_id):
        card_id = inverse[candidate_id]
        card_score = score_by_card[card_id]
        neutral_separation = pair_by_ids[tuple(sorted((card_id, neutral_card)))]["score"]
        checks = {
            "internal_coherence": card_score["internal_coherence"] >= thresholds["minimum_internal_coherence"],
            "non_neutral_distinctiveness": (
                card_score["non_neutral_distinctiveness"] >= thresholds["minimum_non_neutral_distinctiveness"]
            ),
            "separation_from_neutral": neutral_separation >= thresholds["minimum_separation_from_neutral"],
        }
        candidate_results.append({
            "candidate_id": candidate_id,
            "anonymous_card_id": card_id,
            "scores": {
                "internal_coherence": card_score["internal_coherence"],
                "non_neutral_distinctiveness": card_score["non_neutral_distinctiveness"],
                "separation_from_neutral": neutral_separation,
            },
            "checks": checks,
            "disposition": "recognizable" if all(checks.values()) else "not_recognizable",
        })
    neutral_correct = result["most_neutral_card_id"] == neutral_card
    all_candidates = all(item["disposition"] == "recognizable" for item in candidate_results)
    return {
        "artifact_type": "linguistic_register_recognizability_report_v1",
        "schema_version": SCHEMA_VERSION,
        "experiment_id": plan["experiment_id"],
        "bindings": {
            "plan_artifact_sha256": key["plan_artifact_sha256"],
            "packet_artifact_sha256": key["packet_artifact_sha256"],
            "blinding_key_artifact_sha256": key_digest,
            "classifier_result_sha256": result_digest,
        },
        "neutral_check": {
            "expected_anonymous_card_id": neutral_card,
            "selected_anonymous_card_id": result["most_neutral_card_id"],
            "passed": neutral_correct,
        },
        "candidate_results": candidate_results,
        "gate_passed": neutral_correct and all_candidates,
        "disposition": "advance_to_human_selection" if neutral_correct and all_candidates else "revise_profiles_or_check",
        "authority": "mechanical profile-recognizability pre-check only; human acceptance is required before corpus selection",
    }


def normalize_raw_result(raw: dict[str, Any], packet: dict[str, Any], packet_digest: str,
                         provenance: dict[str, Any]) -> dict[str, Any]:
    exact_keys(raw, {
        "card_scores", "pairwise_separation", "most_neutral_card_id", "method_limitations",
    }, set(), "raw_result")
    result = {
        "artifact_type": "linguistic_register_classifier_result_v1",
        "schema_version": SCHEMA_VERSION,
        "experiment_id": packet["experiment_id"],
        "packet_artifact_sha256": packet_digest,
        "classifier_provenance": provenance,
        **raw,
    }
    return validate_result(result, packet, packet_digest)


def emit(value: Any, output: Path | None = None) -> None:
    rendered = json.dumps(value, indent=2, sort_keys=True) + "\n"
    if output is None:
        sys.stdout.write(rendered)
        return
    require(not output.exists(), f"refusing to overwrite existing output: {output}")
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(rendered, encoding="utf-8")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    subparsers = parser.add_subparsers(dest="command", required=True)
    validate_parser = subparsers.add_parser("validate-plan")
    validate_parser.add_argument("plan", type=Path)
    prepare_parser = subparsers.add_parser("prepare")
    prepare_parser.add_argument("plan", type=Path)
    prepare_parser.add_argument("--packet-output", required=True, type=Path)
    prepare_parser.add_argument("--key-output", required=True, type=Path)
    normalize_parser = subparsers.add_parser("normalize-result")
    normalize_parser.add_argument("--plan", required=True, type=Path)
    normalize_parser.add_argument("--packet", required=True, type=Path)
    normalize_parser.add_argument("--raw-result", required=True, type=Path)
    normalize_parser.add_argument("--provider", required=True)
    normalize_parser.add_argument("--model", required=True)
    normalize_parser.add_argument("--reasoning-effort", required=True)
    normalize_parser.add_argument("--execution-mode", required=True)
    normalize_parser.add_argument("--started-at-utc", required=True)
    normalize_parser.add_argument("--completed-at-utc", required=True)
    normalize_parser.add_argument("--output", required=True, type=Path)
    score_parser = subparsers.add_parser("score")
    score_parser.add_argument("--plan", required=True, type=Path)
    score_parser.add_argument("--packet", required=True, type=Path)
    score_parser.add_argument("--key", required=True, type=Path)
    score_parser.add_argument("--result", required=True, type=Path)
    score_parser.add_argument("--output", type=Path)
    arguments = parser.parse_args()

    plan = validate_plan(load_yaml(arguments.plan), arguments.plan)
    if arguments.command == "validate-plan":
        emit({"status": "valid", "artifact_type": plan["artifact_type"]})
    elif arguments.command == "prepare":
        require(not arguments.packet_output.exists(), f"refusing to overwrite existing output: {arguments.packet_output}")
        require(not arguments.key_output.exists(), f"refusing to overwrite existing output: {arguments.key_output}")
        packet, key = prepare(plan, arguments.plan)
        emit(packet, arguments.packet_output)
        emit(key, arguments.key_output)
    elif arguments.command == "normalize-result":
        packet = load_json(arguments.packet)
        validate_prepared_artifacts(plan, arguments.plan, packet)
        packet_digest = sha256_file(arguments.packet)
        provenance = {
            "provider": arguments.provider,
            "model": arguments.model,
            "reasoning_effort": arguments.reasoning_effort,
            "execution_mode": arguments.execution_mode,
            "fresh_context": True,
            "packet_only": True,
            "started_at_utc": arguments.started_at_utc,
            "completed_at_utc": arguments.completed_at_utc,
        }
        emit(normalize_raw_result(load_json(arguments.raw_result), packet, packet_digest, provenance),
             arguments.output)
    else:
        packet = load_json(arguments.packet)
        key = load_json(arguments.key)
        validate_prepared_artifacts(plan, arguments.plan, packet, key)
        packet_digest = sha256_file(arguments.packet)
        result = validate_result(load_json(arguments.result), packet, packet_digest)
        emit(evaluate(plan, key, result, sha256_file(arguments.key), sha256_file(arguments.result)),
             arguments.output)
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except RecognizabilityError as error:
        sys.stderr.write(f"error: {error}\n")
        raise SystemExit(2)
