#!/usr/bin/env python3
"""Prepare and aggregate replicated profile-recognizability calibration runs."""

from __future__ import annotations

import argparse
import hashlib
import importlib.util
import json
import statistics
import sys
from collections import Counter
from pathlib import Path
from typing import Any


RECOGNIZABILITY_PATH = Path(__file__).with_name("recognizability_artifacts.py")
SPEC = importlib.util.spec_from_file_location("recognizability_artifacts_for_calibration", RECOGNIZABILITY_PATH)
if SPEC is None or SPEC.loader is None:
    raise RuntimeError("cannot load recognizability artifact contract")
BASE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(BASE)

SCHEMA_VERSION = 1
PASS_COUNT = 5
CARD_COUNT = 7


class CalibrationError(ValueError):
    """Raised when calibration evidence is malformed or incompatible."""


def require(condition: bool, message: str) -> None:
    if not condition:
        raise CalibrationError(message)


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


def sha256_file(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def canonical_bytes(value: Any) -> bytes:
    return (json.dumps(value, indent=2, sort_keys=True) + "\n").encode()


def digest_value(value: Any) -> str:
    return hashlib.sha256(canonical_bytes(value)).hexdigest()


def resolve(repository: Path, recorded: Any, label: str) -> Path:
    relative = Path(text(recorded, label))
    require(not relative.is_absolute() and ".." not in relative.parts,
            f"{label} must be repository-relative without traversal")
    result = repository / relative
    require(result.is_file(), f"{label} is not a readable file: {result}")
    return result


def validate_plan(value: Any, plan_path: Path) -> tuple[dict[str, Any], dict[str, Any], Path]:
    plan = obj(value, "plan")
    exact_keys(plan, {
        "artifact_type", "schema_version", "status", "experiment_id", "repository",
        "base_precheck_plan", "replications", "randomization_seed", "composite_decoys",
        "classifier_contract", "thresholds", "preregistration_owner", "limitations",
    }, "plan")
    require(plan["artifact_type"] == "linguistic_register_profile_calibration_plan_v1",
            "plan.artifact_type is incompatible")
    require(plan["schema_version"] == SCHEMA_VERSION, "plan.schema_version is incompatible")
    require(plan["status"] == "frozen", "plan.status must be frozen")
    text(plan["experiment_id"], "plan.experiment_id")
    repository = Path(text(plan["repository"], "plan.repository"))
    require(repository.is_absolute() and repository.is_dir(), "plan.repository must be an existing absolute directory")
    base_binding = obj(plan["base_precheck_plan"], "plan.base_precheck_plan")
    exact_keys(base_binding, {"path", "sha256"}, "plan.base_precheck_plan")
    base_path = resolve(repository, base_binding["path"], "plan.base_precheck_plan.path")
    require(sha256_file(base_path) == BASE.sha256(base_binding["sha256"], "plan.base_precheck_plan.sha256"),
            "plan.base_precheck_plan.sha256 does not match current bytes")
    base_plan = BASE.validate_plan(BASE.load_yaml(base_path), base_path)
    require(plan["replications"] == PASS_COUNT, f"plan.replications must be {PASS_COUNT}")
    text(plan["randomization_seed"], "plan.randomization_seed")
    composite = obj(plan["composite_decoys"], "plan.composite_decoys")
    exact_keys(composite, {"construction", "preserved_fields", "donor_rule", "purpose"},
               "plan.composite_decoys")
    for key in ("construction", "donor_rule", "purpose"):
        text(composite[key], f"plan.composite_decoys.{key}")
    require(composite["preserved_fields"] == ["feature_count", "layer_multiset", "salience_weight_multiset"],
            "plan.composite_decoys.preserved_fields is incompatible")
    contract = obj(plan["classifier_contract"], "plan.classifier_contract")
    exact_keys(contract, {"task", "blindness", "score_scale", "required_outputs", "prohibited_inference"},
               "plan.classifier_contract")
    for key in ("task", "blindness", "score_scale", "prohibited_inference"):
        text(contract[key], f"plan.classifier_contract.{key}")
    require(contract["required_outputs"] == [
        "internal_coherence", "non_neutral_distinctiveness", "pairwise_separation", "most_neutral_card",
    ], "plan.classifier_contract.required_outputs is incompatible")
    thresholds = obj(plan["thresholds"], "plan.thresholds")
    exact_keys(thresholds, {
        "minimum_correct_neutral_passes", "minimum_authentic_coherence_wins",
        "minimum_median_non_neutral_distinctiveness", "maximum_distinctiveness_range",
        "minimum_median_real_pair_separation",
    }, "plan.thresholds")
    for key, value in thresholds.items():
        require(isinstance(value, int) and not isinstance(value, bool), f"plan.thresholds.{key} must be integer")
    require(1 <= thresholds["minimum_correct_neutral_passes"] <= PASS_COUNT,
            "minimum_correct_neutral_passes is out of range")
    require(1 <= thresholds["minimum_authentic_coherence_wins"] <= PASS_COUNT,
            "minimum_authentic_coherence_wins is out of range")
    for key in ("minimum_median_non_neutral_distinctiveness", "minimum_median_real_pair_separation"):
        require(1 <= thresholds[key] <= 5, f"plan.thresholds.{key} is out of range")
    require(0 <= thresholds["maximum_distinctiveness_range"] <= 4,
            "maximum_distinctiveness_range is out of range")
    text(plan["preregistration_owner"], "plan.preregistration_owner")
    for index, limitation in enumerate(array(plan["limitations"], "plan.limitations")):
        text(limitation, f"plan.limitations[{index}]")
    return plan, base_plan, base_path


def hashed_order(seed: str, values: list[str]) -> list[str]:
    return sorted(values, key=lambda value: hashlib.sha256(f"{seed}\0{value}".encode()).hexdigest())


def base_cards(base_plan: dict[str, Any], base_path: Path) -> tuple[dict[str, list[dict[str, Any]]], str,
                                                                    dict[str, str]]:
    packet, key = BASE.prepare(base_plan, base_path)
    identity_by_card = key["anonymous_mapping"]
    cards = {identity_by_card[card["anonymous_card_id"]]: card["features"] for card in packet["cards"]}
    neutral_id = key["neutral_decoy_id"]
    candidates = {identity: features for identity, features in cards.items() if identity != neutral_id}
    return candidates, neutral_id, key["profile_artifact_sha256_by_candidate"]


def composite_for(target_id: str, candidates: dict[str, list[dict[str, Any]]], seed: str) -> list[dict[str, Any]]:
    target = candidates[target_id]
    donor_pool = []
    for donor_id, features in candidates.items():
        if donor_id == target_id:
            continue
        for index, feature in enumerate(features):
            donor_pool.append((donor_id, index, feature))
    used: set[tuple[str, int]] = set()
    composite = []
    for slot, target_feature in enumerate(target):
        eligible = [(donor, index, feature) for donor, index, feature in donor_pool
                    if feature["layer"] == target_feature["layer"] and (donor, index) not in used]
        require(bool(eligible), f"cannot construct layer-matched composite for {target_id}")
        ordered = sorted(eligible, key=lambda item: hashlib.sha256(
            f"{seed}\0{target_id}\0{slot}\0{item[0]}\0{item[1]}".encode()).hexdigest())
        donor, index, selected = ordered[0]
        used.add((donor, index))
        composite.append({
            "layer": target_feature["layer"],
            "category": selected["category"],
            "description": selected["description"],
            "salience_weight": target_feature["salience_weight"],
        })
    require(Counter(item["layer"] for item in composite) == Counter(item["layer"] for item in target),
            "composite layer multiset changed")
    require(Counter(item["salience_weight"] for item in composite) ==
            Counter(item["salience_weight"] for item in target), "composite weight multiset changed")
    return composite


def prepare_pass(plan: dict[str, Any], base_plan: dict[str, Any], base_path: Path,
                 plan_path: Path, pass_number: int) -> tuple[dict[str, Any], dict[str, Any]]:
    require(1 <= pass_number <= PASS_COUNT, f"pass_number must be 1 through {PASS_COUNT}")
    candidates, neutral_id, profile_bindings = base_cards(base_plan, base_path)
    _, base_key = BASE.prepare(base_plan, base_path)
    base_packet, _ = BASE.prepare(base_plan, base_path)
    base_identity_by_card = base_key["anonymous_mapping"]
    neutral_features = next(card["features"] for card in base_packet["cards"]
                            if base_identity_by_card[card["anonymous_card_id"]] == neutral_id)
    identities: dict[str, list[dict[str, Any]]] = dict(candidates)
    identities[neutral_id] = neutral_features
    composite_by_candidate: dict[str, str] = {}
    for candidate_id in sorted(candidates):
        composite_id = f"composite-for-{candidate_id}"
        composite_by_candidate[candidate_id] = composite_id
        identities[composite_id] = composite_for(candidate_id, candidates, plan["randomization_seed"])
    pass_seed = f"{plan['randomization_seed']}::pass-{pass_number:02d}"
    ordered_identities = hashed_order(pass_seed, list(identities))
    mapping = {f"C{index:02d}": identity for index, identity in enumerate(ordered_identities, start=1)}
    cards = []
    for card_id, identity in mapping.items():
        features = sorted(identities[identity], key=lambda feature: hashlib.sha256(
            f"{pass_seed}\0{identity}\0{feature['description']}".encode()).hexdigest())
        cards.append({"anonymous_card_id": card_id, "feature_count": len(features), "features": features})
    packet = {
        "artifact_type": "linguistic_register_calibration_packet_v1",
        "schema_version": SCHEMA_VERSION,
        "experiment_id": plan["experiment_id"],
        "pass_number": pass_number,
        "plan_artifact_sha256": sha256_file(plan_path),
        "classifier_contract": plan["classifier_contract"],
        "cards": cards,
        "authority": "blinded profile-card calibration input; no corpus-selection or rendering authority",
    }
    kinds = {identity: ("neutral" if identity == neutral_id else
                        "composite" if identity.startswith("composite-for-") else "authentic")
             for identity in identities}
    key = {
        "artifact_type": "linguistic_register_calibration_key_v1",
        "schema_version": SCHEMA_VERSION,
        "experiment_id": plan["experiment_id"],
        "pass_number": pass_number,
        "plan_artifact_sha256": sha256_file(plan_path),
        "base_precheck_plan_sha256": sha256_file(base_path),
        "packet_artifact_sha256": digest_value(packet),
        "anonymous_mapping": mapping,
        "kind_by_identity": kinds,
        "neutral_identity": neutral_id,
        "matched_composite_by_candidate": composite_by_candidate,
        "profile_artifact_sha256_by_candidate": profile_bindings,
        "authority": "unblinding and aggregate calibration scoring only",
    }
    return packet, key


def validate_prepared(plan: dict[str, Any], base_plan: dict[str, Any], base_path: Path, plan_path: Path,
                      pass_number: int, packet: dict[str, Any], key: dict[str, Any]) -> None:
    expected_packet, expected_key = prepare_pass(plan, base_plan, base_path, plan_path, pass_number)
    require(packet == expected_packet, f"pass {pass_number} packet does not match deterministic preparation")
    require(key == expected_key, f"pass {pass_number} key does not match deterministic preparation")


def validate_result(value: Any, packet: dict[str, Any], packet_digest: str) -> dict[str, Any]:
    result = obj(value, "result")
    exact_keys(result, {
        "artifact_type", "schema_version", "experiment_id", "pass_number", "packet_artifact_sha256",
        "classifier_provenance", "card_scores", "pairwise_separation", "most_neutral_card_id",
        "method_limitations",
    }, "result")
    require(result["artifact_type"] == "linguistic_register_calibration_result_v1",
            "result.artifact_type is incompatible")
    require(result["schema_version"] == SCHEMA_VERSION, "result.schema_version is incompatible")
    require(result["experiment_id"] == packet["experiment_id"], "result.experiment_id does not match packet")
    require(result["pass_number"] == packet["pass_number"], "result.pass_number does not match packet")
    require(result["packet_artifact_sha256"] == packet_digest, "result does not bind packet")
    provenance = obj(result["classifier_provenance"], "result.classifier_provenance")
    exact_keys(provenance, {"provider", "model", "reasoning_effort", "execution_mode", "fresh_context",
                            "packet_only", "started_at_utc", "completed_at_utc"},
               "result.classifier_provenance")
    for key in ("provider", "model", "reasoning_effort", "execution_mode", "started_at_utc", "completed_at_utc"):
        text(provenance[key], f"result.classifier_provenance.{key}")
    require(provenance["fresh_context"] is True and provenance["packet_only"] is True,
            "classifier provenance must attest fresh packet-only execution")
    card_ids = {card["anonymous_card_id"] for card in packet["cards"]}
    require(len(card_ids) == CARD_COUNT, f"packet must contain {CARD_COUNT} unique cards")
    scores = array(result["card_scores"], "result.card_scores")
    require(len(scores) == CARD_COUNT, "result.card_scores must score every card once")
    seen = set()
    for index, item_value in enumerate(scores):
        item = obj(item_value, f"result.card_scores[{index}]")
        exact_keys(item, {"anonymous_card_id", "internal_coherence", "non_neutral_distinctiveness", "rationale"},
                   f"result.card_scores[{index}]")
        card_id = item["anonymous_card_id"]
        require(card_id in card_ids and card_id not in seen, "card score identity is unknown or duplicated")
        seen.add(card_id)
        for field in ("internal_coherence", "non_neutral_distinctiveness"):
            require(isinstance(item[field], int) and not isinstance(item[field], bool) and 1 <= item[field] <= 5,
                    f"result.card_scores[{index}].{field} must be integer 1 through 5")
        text(item["rationale"], f"result.card_scores[{index}].rationale")
    expected_pairs = {tuple(sorted((left, right))) for left in card_ids for right in card_ids if left < right}
    seen_pairs = set()
    pairs = array(result["pairwise_separation"], "result.pairwise_separation")
    require(len(pairs) == len(expected_pairs), "result must score every unordered pair")
    for index, item_value in enumerate(pairs):
        item = obj(item_value, f"result.pairwise_separation[{index}]")
        exact_keys(item, {"card_ids", "score", "rationale"}, f"result.pairwise_separation[{index}]")
        ids = array(item["card_ids"], f"result.pairwise_separation[{index}].card_ids")
        require(len(ids) == 2 and len(set(ids)) == 2 and set(ids) <= card_ids, "pair identities are invalid")
        normalized = tuple(sorted(ids))
        require(normalized not in seen_pairs, "pair score is duplicated")
        seen_pairs.add(normalized)
        require(isinstance(item["score"], int) and not isinstance(item["score"], bool)
                and 1 <= item["score"] <= 5, "pair score must be integer 1 through 5")
        text(item["rationale"], f"result.pairwise_separation[{index}].rationale")
    require(seen_pairs == expected_pairs, "result does not cover every unordered pair")
    require(result["most_neutral_card_id"] in card_ids, "most_neutral_card_id is unknown")
    for index, limitation in enumerate(array(result["method_limitations"], "result.method_limitations")):
        text(limitation, f"result.method_limitations[{index}]")
    return result


def normalize(raw: dict[str, Any], packet: dict[str, Any], provenance: dict[str, Any]) -> dict[str, Any]:
    exact_keys(raw, {"card_scores", "pairwise_separation", "most_neutral_card_id", "method_limitations"},
               "raw_result")
    result = {
        "artifact_type": "linguistic_register_calibration_result_v1",
        "schema_version": SCHEMA_VERSION,
        "experiment_id": packet["experiment_id"],
        "pass_number": packet["pass_number"],
        "packet_artifact_sha256": digest_value(packet),
        "classifier_provenance": provenance,
        **raw,
    }
    return validate_result(result, packet, digest_value(packet))


def aggregate(plan: dict[str, Any], runs: list[tuple[dict[str, Any], dict[str, Any], dict[str, Any], str]]) -> dict[str, Any]:
    require(len(runs) == PASS_COUNT, f"aggregate requires exactly {PASS_COUNT} runs")
    candidates = sorted(plan_candidate for plan_candidate in runs[0][1]["matched_composite_by_candidate"])
    neutral_correct = 0
    candidate_values = {candidate: {"coherence": [], "distinctiveness": [], "composite_coherence": [],
                                    "wins": 0} for candidate in candidates}
    real_pair_values: dict[tuple[str, str], list[int]] = {
        pair: [] for index, left in enumerate(candidates) for pair in [(left, right) for right in candidates[index + 1:]]
    }
    run_bindings = []
    for packet, key, result, result_digest in runs:
        inverse = {identity: card_id for card_id, identity in key["anonymous_mapping"].items()}
        scores = {item["anonymous_card_id"]: item for item in result["card_scores"]}
        pairs = {tuple(sorted(item["card_ids"])): item["score"] for item in result["pairwise_separation"]}
        neutral_card = inverse[key["neutral_identity"]]
        neutral_correct += int(result["most_neutral_card_id"] == neutral_card)
        for candidate in candidates:
            authentic = scores[inverse[candidate]]
            composite = scores[inverse[key["matched_composite_by_candidate"][candidate]]]
            values = candidate_values[candidate]
            values["coherence"].append(authentic["internal_coherence"])
            values["distinctiveness"].append(authentic["non_neutral_distinctiveness"])
            values["composite_coherence"].append(composite["internal_coherence"])
            values["wins"] += int(authentic["internal_coherence"] > composite["internal_coherence"])
        for pair in real_pair_values:
            real_pair_values[pair].append(pairs[tuple(sorted((inverse[pair[0]], inverse[pair[1]])))])
        run_bindings.append({
            "pass_number": packet["pass_number"],
            "packet_artifact_sha256": key["packet_artifact_sha256"],
            "key_artifact_sha256": digest_value(key),
            "classifier_result_sha256": result_digest,
        })
    thresholds = plan["thresholds"]
    candidate_results = []
    for candidate in candidates:
        values = candidate_values[candidate]
        median_distinctiveness = statistics.median(values["distinctiveness"])
        distinctiveness_range = max(values["distinctiveness"]) - min(values["distinctiveness"])
        checks = {
            "authentic_coherence_wins": values["wins"] >= thresholds["minimum_authentic_coherence_wins"],
            "median_non_neutral_distinctiveness": (
                median_distinctiveness >= thresholds["minimum_median_non_neutral_distinctiveness"]
            ),
            "distinctiveness_stability": distinctiveness_range <= thresholds["maximum_distinctiveness_range"],
        }
        candidate_results.append({
            "candidate_id": candidate,
            "authentic_coherence_scores": values["coherence"],
            "matched_composite_coherence_scores": values["composite_coherence"],
            "authentic_coherence_wins": values["wins"],
            "non_neutral_distinctiveness_scores": values["distinctiveness"],
            "median_non_neutral_distinctiveness": median_distinctiveness,
            "distinctiveness_range": distinctiveness_range,
            "checks": checks,
            "passed": all(checks.values()),
        })
    pair_results = []
    for pair, values in sorted(real_pair_values.items()):
        median = statistics.median(values)
        pair_results.append({
            "candidate_ids": list(pair), "scores": values, "median": median,
            "passed": median >= thresholds["minimum_median_real_pair_separation"],
        })
    neutral_passed = neutral_correct >= thresholds["minimum_correct_neutral_passes"]
    gate_passed = neutral_passed and all(item["passed"] for item in candidate_results) \
        and all(item["passed"] for item in pair_results)
    return {
        "artifact_type": "linguistic_register_profile_calibration_report_v1",
        "schema_version": SCHEMA_VERSION,
        "experiment_id": plan["experiment_id"],
        "plan_artifact_sha256": sha256_file(Path(plan["_plan_path"])),
        "run_bindings": run_bindings,
        "neutral_identification": {"correct_passes": neutral_correct, "required": thresholds["minimum_correct_neutral_passes"],
                                   "passed": neutral_passed},
        "candidate_results": candidate_results,
        "real_pair_separation": pair_results,
        "gate_passed": gate_passed,
        "disposition": "instrument_discriminates" if gate_passed else "instrument_not_calibrated",
        "authority": "stability calibration only; no candidate ranking, corpus selection, or rendering authority",
    }


def emit(value: Any, output: Path) -> None:
    require(not output.exists(), f"refusing to overwrite existing output: {output}")
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_bytes(canonical_bytes(value))


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest="command", required=True)
    validate = sub.add_parser("validate-plan")
    validate.add_argument("plan", type=Path)
    prepare = sub.add_parser("prepare")
    prepare.add_argument("plan", type=Path)
    prepare.add_argument("--output-dir", required=True, type=Path)
    normalize_parser = sub.add_parser("normalize-result")
    normalize_parser.add_argument("--plan", required=True, type=Path)
    normalize_parser.add_argument("--packet", required=True, type=Path)
    normalize_parser.add_argument("--key", required=True, type=Path)
    normalize_parser.add_argument("--raw-result", required=True, type=Path)
    normalize_parser.add_argument("--provider", required=True)
    normalize_parser.add_argument("--model", required=True)
    normalize_parser.add_argument("--reasoning-effort", required=True)
    normalize_parser.add_argument("--execution-mode", required=True)
    normalize_parser.add_argument("--started-at-utc", required=True)
    normalize_parser.add_argument("--completed-at-utc", required=True)
    normalize_parser.add_argument("--output", required=True, type=Path)
    aggregate_parser = sub.add_parser("aggregate")
    aggregate_parser.add_argument("--plan", required=True, type=Path)
    aggregate_parser.add_argument("--run-dir", required=True, type=Path)
    aggregate_parser.add_argument("--output", required=True, type=Path)
    args = parser.parse_args()
    plan_path = args.plan
    plan, base_plan, base_path = validate_plan(BASE.load_yaml(plan_path), plan_path)
    if args.command == "validate-plan":
        print(json.dumps({"status": "valid", "artifact_type": plan["artifact_type"]}, sort_keys=True))
    elif args.command == "prepare":
        require(not args.output_dir.exists(), f"refusing to overwrite existing output directory: {args.output_dir}")
        for pass_number in range(1, PASS_COUNT + 1):
            packet, key = prepare_pass(plan, base_plan, base_path, plan_path, pass_number)
            emit(packet, args.output_dir / f"pass-{pass_number:02d}" / "packet.json")
            emit(key, args.output_dir / f"pass-{pass_number:02d}" / "key.json")
    elif args.command == "normalize-result":
        packet = BASE.load_json(args.packet)
        key = BASE.load_json(args.key)
        pass_number = packet.get("pass_number")
        validate_prepared(plan, base_plan, base_path, plan_path, pass_number, packet, key)
        provenance = {
            "provider": args.provider, "model": args.model, "reasoning_effort": args.reasoning_effort,
            "execution_mode": args.execution_mode, "fresh_context": True, "packet_only": True,
            "started_at_utc": args.started_at_utc, "completed_at_utc": args.completed_at_utc,
        }
        emit(normalize(BASE.load_json(args.raw_result), packet, provenance), args.output)
    else:
        runs = []
        for pass_number in range(1, PASS_COUNT + 1):
            directory = args.run_dir / f"pass-{pass_number:02d}"
            packet_path, key_path, result_path = directory / "packet.json", directory / "key.json", directory / "classifier-result.json"
            packet, key, result = BASE.load_json(packet_path), BASE.load_json(key_path), BASE.load_json(result_path)
            validate_prepared(plan, base_plan, base_path, plan_path, pass_number, packet, key)
            validate_result(result, packet, sha256_file(packet_path))
            runs.append((packet, key, result, sha256_file(result_path)))
        plan["_plan_path"] = str(plan_path)
        emit(aggregate(plan, runs), args.output)
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (CalibrationError, BASE.RecognizabilityError) as error:
        print(f"calibration_artifacts: {error}", file=sys.stderr)
        raise SystemExit(2)
