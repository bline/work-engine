#!/usr/bin/env python3
"""Prepare, validate, and score blinded fixed-semantics micro-render evidence."""

from __future__ import annotations

import argparse
import datetime as dt
import hashlib
import importlib.util
import json
import statistics
import sys
from pathlib import Path
from typing import Any


BASE_PATH = Path(__file__).with_name("recognizability_artifacts.py")
SPEC = importlib.util.spec_from_file_location("recognizability_for_micro_render", BASE_PATH)
if SPEC is None or SPEC.loader is None:
    raise RuntimeError("cannot load recognizability contract")
BASE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(BASE)

SCHEMA_VERSION = 1
BRIEF_COUNT = 2
CONDITION_COUNT = 4
REPLICAS = 2
SAMPLE_COUNT = BRIEF_COUNT * CONDITION_COUNT * REPLICAS
MATCH_PASSES = 3


class MicroRenderError(ValueError):
    pass


def require(condition: bool, message: str) -> None:
    if not condition:
        raise MicroRenderError(message)


def obj(value: Any, path: str) -> dict[str, Any]:
    require(isinstance(value, dict), f"{path} must be an object")
    return value


def array(value: Any, path: str) -> list[Any]:
    require(isinstance(value, list), f"{path} must be an array")
    return value


def text(value: Any, path: str) -> str:
    require(isinstance(value, str) and bool(value.strip()), f"{path} must be a nonempty string")
    return value


def exact(value: dict[str, Any], fields: set[str], path: str) -> None:
    require(set(value) == fields,
            f"{path} fields differ: missing={sorted(fields-set(value))}, unknown={sorted(set(value)-fields)}")


def canonical_bytes(value: Any) -> bytes:
    return (json.dumps(value, indent=2, sort_keys=True) + "\n").encode()


def digest_value(value: Any) -> str:
    return hashlib.sha256(canonical_bytes(value)).hexdigest()


def sha256_file(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def resolve(repository: Path, recorded: Any, label: str) -> Path:
    relative = Path(text(recorded, label))
    require(not relative.is_absolute() and ".." not in relative.parts,
            f"{label} must be repository-relative without traversal")
    result = repository / relative
    require(result.is_file(), f"{label} is not readable: {result}")
    return result


def validate_plan(value: Any, plan_path: Path) -> tuple[dict[str, Any], dict[str, Any], Path]:
    plan = obj(value, "plan")
    exact(plan, {
        "artifact_type", "schema_version", "status", "experiment_id", "repository",
        "base_precheck_plan", "briefs", "replicas_per_brief_condition", "rendering_seed",
        "rendering_contract", "semantic_review_contract", "matching_contract", "thresholds",
        "preregistration_owner", "limitations",
    }, "plan")
    require(plan["artifact_type"] == "linguistic_register_micro_render_plan_v1", "plan type is incompatible")
    require(plan["schema_version"] == SCHEMA_VERSION and plan["status"] == "frozen",
            "plan schema/status is incompatible")
    text(plan["experiment_id"], "plan.experiment_id")
    repository = Path(text(plan["repository"], "plan.repository"))
    require(repository.is_absolute() and repository.is_dir(), "plan.repository must be an existing absolute directory")
    binding = obj(plan["base_precheck_plan"], "plan.base_precheck_plan")
    exact(binding, {"path", "sha256"}, "plan.base_precheck_plan")
    base_path = resolve(repository, binding["path"], "plan.base_precheck_plan.path")
    require(sha256_file(base_path) == BASE.sha256(binding["sha256"], "plan.base_precheck_plan.sha256"),
            "base precheck plan digest does not match")
    base_plan = BASE.validate_plan(BASE.load_yaml(base_path), base_path)
    briefs = array(plan["briefs"], "plan.briefs")
    require(len(briefs) == BRIEF_COUNT, f"plan.briefs must contain {BRIEF_COUNT} briefs")
    brief_ids = set()
    for index, brief_value in enumerate(briefs):
        path = f"plan.briefs[{index}]"
        brief = obj(brief_value, path)
        exact(brief, {"brief_id", "context", "required_propositions", "required_speech_act", "output_constraint"}, path)
        brief_id = BASE.identifier(brief["brief_id"], f"{path}.brief_id")
        require(brief_id not in brief_ids, "brief IDs must be unique")
        brief_ids.add(brief_id)
        text(brief["context"], f"{path}.context")
        propositions = array(brief["required_propositions"], f"{path}.required_propositions")
        require(len(propositions) == 4, f"{path}.required_propositions must contain exactly four items")
        proposition_ids = set()
        for p_index, proposition_value in enumerate(propositions):
            proposition = obj(proposition_value, f"{path}.required_propositions[{p_index}]")
            exact(proposition, {"id", "meaning"}, f"{path}.required_propositions[{p_index}]")
            proposition_id = BASE.identifier(proposition["id"], f"{path}.required_propositions[{p_index}].id")
            require(proposition_id not in proposition_ids, "proposition IDs must be unique within a brief")
            proposition_ids.add(proposition_id)
            text(proposition["meaning"], f"{path}.required_propositions[{p_index}].meaning")
        text(brief["required_speech_act"], f"{path}.required_speech_act")
        text(brief["output_constraint"], f"{path}.output_constraint")
    require(plan["replicas_per_brief_condition"] == REPLICAS, f"replicas must be {REPLICAS}")
    text(plan["rendering_seed"], "plan.rendering_seed")
    for contract_name in ("rendering_contract", "semantic_review_contract", "matching_contract"):
        contract = obj(plan[contract_name], f"plan.{contract_name}")
        expected = {"task", "blindness", "prohibitions"}
        if contract_name == "matching_contract":
            expected |= {"score_scale", "passes"}
        exact(contract, expected, f"plan.{contract_name}")
        for key in expected - {"passes"}:
            text(contract[key], f"plan.{contract_name}.{key}")
        if "passes" in contract:
            require(contract["passes"] == MATCH_PASSES, f"matching passes must be {MATCH_PASSES}")
    thresholds = obj(plan["thresholds"], "plan.thresholds")
    exact(thresholds, {
        "require_all_semantically_equivalent", "minimum_correct_assignments_per_condition",
        "minimum_passing_match_passes_per_condition", "minimum_correct_per_condition_in_passing_pass",
    }, "plan.thresholds")
    require(thresholds["require_all_semantically_equivalent"] is True,
            "all renderings must be semantically equivalent")
    for key in set(thresholds) - {"require_all_semantically_equivalent"}:
        require(isinstance(thresholds[key], int) and not isinstance(thresholds[key], bool),
                f"plan.thresholds.{key} must be integer")
    require(1 <= thresholds["minimum_correct_assignments_per_condition"] <= MATCH_PASSES * BRIEF_COUNT * REPLICAS,
            "minimum_correct_assignments_per_condition is out of range")
    require(1 <= thresholds["minimum_passing_match_passes_per_condition"] <= MATCH_PASSES,
            "minimum_passing_match_passes_per_condition is out of range")
    require(1 <= thresholds["minimum_correct_per_condition_in_passing_pass"] <= BRIEF_COUNT * REPLICAS,
            "minimum_correct_per_condition_in_passing_pass is out of range")
    text(plan["preregistration_owner"], "plan.preregistration_owner")
    for index, limitation in enumerate(array(plan["limitations"], "plan.limitations")):
        text(limitation, f"plan.limitations[{index}]")
    return plan, base_plan, base_path


def styles(base_plan: dict[str, Any], base_path: Path) -> tuple[dict[str, list[dict[str, Any]]], str, dict[str, str]]:
    packet, key = BASE.prepare(base_plan, base_path)
    by_card = {card["anonymous_card_id"]: card["features"] for card in packet["cards"]}
    style_by_condition = {identity: by_card[card_id] for card_id, identity in key["anonymous_mapping"].items()}
    return style_by_condition, key["neutral_decoy_id"], key["profile_artifact_sha256_by_candidate"]


def sample_identities(plan: dict[str, Any], conditions: list[str]) -> list[str]:
    return [f"{brief['brief_id']}::{condition}::replica-{replica}"
            for brief in plan["briefs"] for condition in sorted(conditions)
            for replica in range(1, REPLICAS + 1)]


def hashed_order(seed: str, values: list[str]) -> list[str]:
    return sorted(values, key=lambda value: hashlib.sha256(f"{seed}\0{value}".encode()).hexdigest())


def prepare_render_jobs(plan: dict[str, Any], base_plan: dict[str, Any], base_path: Path,
                        plan_path: Path) -> tuple[dict[str, dict[str, Any]], dict[str, Any]]:
    style_by_condition, neutral_id, profile_bindings = styles(base_plan, base_path)
    identities = sample_identities(plan, list(style_by_condition))
    ordered = hashed_order(plan["rendering_seed"], identities)
    mapping = {f"S{index:02d}": identity for index, identity in enumerate(ordered, start=1)}
    briefs = {brief["brief_id"]: brief for brief in plan["briefs"]}
    packets = {}
    records = {}
    for sample_id, identity in mapping.items():
        brief_id, condition_id, replica_label = identity.split("::")
        features = hashed_order(
            f"{plan['rendering_seed']}::{sample_id}",
            [json.dumps(feature, sort_keys=True) for feature in style_by_condition[condition_id]],
        )
        style_features = [json.loads(feature) for feature in features]
        packet = {
            "artifact_type": "linguistic_register_micro_render_job_v1",
            "schema_version": SCHEMA_VERSION,
            "experiment_id": plan["experiment_id"],
            "sample_id": sample_id,
            "semantic_brief": briefs[brief_id],
            "anonymous_style": {"features": style_features},
            "rendering_contract": plan["rendering_contract"],
            "authority": "fixed-semantics micro-render input; no candidate-selection authority",
        }
        packets[sample_id] = packet
        records[sample_id] = {
            "identity": identity, "condition_id": condition_id, "brief_id": brief_id,
            "replica": int(replica_label.removeprefix("replica-")),
            "kind": "neutral" if condition_id == neutral_id else "candidate",
            "packet_artifact_sha256": digest_value(packet),
        }
    key = {
        "artifact_type": "linguistic_register_micro_render_key_v1",
        "schema_version": SCHEMA_VERSION,
        "experiment_id": plan["experiment_id"],
        "plan_artifact_sha256": sha256_file(plan_path),
        "base_precheck_plan_sha256": sha256_file(base_path),
        "samples": records,
        "neutral_condition_id": neutral_id,
        "profile_artifact_sha256_by_candidate": profile_bindings,
        "authority": "unblinding, semantic-gate aggregation, and matching score only",
    }
    return packets, key


def validate_render_artifact(value: Any, packet: dict[str, Any], packet_digest: str) -> dict[str, Any]:
    artifact = obj(value, "render")
    exact(artifact, {"artifact_type", "schema_version", "experiment_id", "sample_id",
                     "packet_artifact_sha256", "renderer_provenance", "text"}, "render")
    require(artifact["artifact_type"] == "linguistic_register_micro_render_v1", "render type incompatible")
    require(artifact["schema_version"] == SCHEMA_VERSION, "render schema incompatible")
    require(artifact["experiment_id"] == packet["experiment_id"] and artifact["sample_id"] == packet["sample_id"],
            "render identity does not match packet")
    require(artifact["packet_artifact_sha256"] == packet_digest, "render does not bind packet")
    validate_provenance(artifact["renderer_provenance"], "render.renderer_provenance")
    rendered = text(artifact["text"], "render.text")
    word_count = len(rendered.split())
    require(70 <= word_count <= 110, "render.text must contain 70 through 110 whitespace-delimited words")
    require("\n\n" not in rendered and "\r" not in rendered,
            "render.text must be one paragraph")
    require(not any(line.lstrip().startswith(("#", "- ", "* ", "•")) for line in rendered.splitlines()),
            "render.text must not contain heading or bullet syntax")
    return artifact


def reject_nested_structured_render(value: Any) -> None:
    rendered = text(value, "raw_render.text").strip()
    try:
        nested = json.loads(rendered)
    except json.JSONDecodeError:
        return
    require(not isinstance(nested, (dict, list, str)),
            "raw_render.text must be prose, not serialized structured output")


def validate_provenance(value: Any, path: str) -> dict[str, Any]:
    provenance = obj(value, path)
    exact(provenance, {"provider", "model", "reasoning_effort", "execution_mode", "fresh_context",
                       "packet_only", "started_at_utc", "completed_at_utc"}, path)
    for key in ("provider", "model", "reasoning_effort", "execution_mode", "started_at_utc", "completed_at_utc"):
        text(provenance[key], f"{path}.{key}")
    timestamps = []
    for key in ("started_at_utc", "completed_at_utc"):
        try:
            parsed = dt.datetime.fromisoformat(provenance[key].replace("Z", "+00:00"))
        except ValueError as error:
            raise MicroRenderError(f"{path}.{key} must be an ISO-8601 timestamp") from error
        require(parsed.tzinfo is not None, f"{path}.{key} must include a UTC offset")
        timestamps.append(parsed)
    require(timestamps[0] <= timestamps[1], f"{path} completion must not precede start")
    require(provenance["fresh_context"] is True and provenance["packet_only"] is True,
            f"{path} must attest fresh packet-only execution")
    return provenance


def normalize_render(raw: dict[str, Any], packet: dict[str, Any], provenance: dict[str, Any]) -> dict[str, Any]:
    exact(raw, {"text"}, "raw_render")
    reject_nested_structured_render(raw["text"])
    artifact = {
        "artifact_type": "linguistic_register_micro_render_v1", "schema_version": SCHEMA_VERSION,
        "experiment_id": packet["experiment_id"], "sample_id": packet["sample_id"],
        "packet_artifact_sha256": digest_value(packet), "renderer_provenance": provenance,
        "text": raw["text"],
    }
    return validate_render_artifact(artifact, packet, digest_value(packet))


def semantic_packet(plan: dict[str, Any], render_packet: dict[str, Any], render: dict[str, Any]) -> dict[str, Any]:
    return {
        "artifact_type": "linguistic_register_semantic_review_job_v1", "schema_version": SCHEMA_VERSION,
        "experiment_id": plan["experiment_id"], "sample_id": render["sample_id"],
        "semantic_brief": render_packet["semantic_brief"], "rendered_text": render["text"],
        "semantic_review_contract": plan["semantic_review_contract"],
        "render_artifact_sha256": digest_value(render),
        "authority": "blinded semantic-equivalence review input; style identity withheld",
    }


def validate_semantic_result(value: Any, packet: dict[str, Any], packet_digest: str) -> dict[str, Any]:
    result = obj(value, "semantic_result")
    exact(result, {"artifact_type", "schema_version", "experiment_id", "sample_id", "packet_artifact_sha256",
                   "reviewer_provenance", "verdict", "proposition_checks", "speech_act_equivalent",
                   "added_meaning", "summary"}, "semantic_result")
    require(result["artifact_type"] == "linguistic_register_semantic_result_v1", "semantic result type incompatible")
    require(result["schema_version"] == SCHEMA_VERSION, "semantic result schema incompatible")
    require(result["experiment_id"] == packet["experiment_id"] and result["sample_id"] == packet["sample_id"],
            "semantic result identity mismatch")
    require(result["packet_artifact_sha256"] == packet_digest, "semantic result does not bind packet")
    validate_provenance(result["reviewer_provenance"], "semantic_result.reviewer_provenance")
    require(result["verdict"] in {"equivalent", "not_equivalent", "uncertain"}, "semantic verdict invalid")
    expected_ids = [item["id"] for item in packet["semantic_brief"]["required_propositions"]]
    checks = array(result["proposition_checks"], "semantic_result.proposition_checks")
    require(len(checks) == len(expected_ids), "semantic result must check every proposition once")
    observed = []
    for index, check_value in enumerate(checks):
        check = obj(check_value, f"semantic_result.proposition_checks[{index}]")
        exact(check, {"proposition_id", "status", "evidence"}, f"semantic_result.proposition_checks[{index}]")
        observed.append(check["proposition_id"])
        require(check["status"] in {"preserved", "omitted", "altered", "uncertain"}, "proposition status invalid")
        text(check["evidence"], f"semantic_result.proposition_checks[{index}].evidence")
    require(sorted(observed) == sorted(expected_ids) and len(set(observed)) == len(observed),
            "semantic result proposition IDs are incomplete or duplicated")
    require(isinstance(result["speech_act_equivalent"], bool), "speech_act_equivalent must be boolean")
    additions = array(result["added_meaning"], "semantic_result.added_meaning")
    for index, addition in enumerate(additions):
        text(addition, f"semantic_result.added_meaning[{index}]")
    text(result["summary"], "semantic_result.summary")
    mechanically_equivalent = (all(check["status"] == "preserved" for check in checks)
                               and result["speech_act_equivalent"] and not additions)
    require((result["verdict"] == "equivalent") == mechanically_equivalent,
            "semantic verdict conflicts with proposition, speech-act, or addition checks")
    return result


def normalize_semantic(raw: dict[str, Any], packet: dict[str, Any], provenance: dict[str, Any]) -> dict[str, Any]:
    exact(raw, {"verdict", "proposition_checks", "speech_act_equivalent", "added_meaning", "summary"},
          "raw_semantic_result")
    result = {
        "artifact_type": "linguistic_register_semantic_result_v1", "schema_version": SCHEMA_VERSION,
        "experiment_id": packet["experiment_id"], "sample_id": packet["sample_id"],
        "packet_artifact_sha256": digest_value(packet), "reviewer_provenance": provenance, **raw,
    }
    return validate_semantic_result(result, packet, digest_value(packet))


def prepare_matching_pass(plan: dict[str, Any], base_plan: dict[str, Any], base_path: Path,
                          plan_path: Path, run_dir: Path, pass_number: int) -> tuple[dict[str, Any], dict[str, Any]]:
    require(1 <= pass_number <= MATCH_PASSES, "matching pass number out of range")
    expected_packets, expected_key = prepare_render_jobs(plan, base_plan, base_path, plan_path)
    render_key = BASE.load_json(run_dir / "render-key.json")
    require(render_key == expected_key, "render key does not match deterministic preparation")
    accepted_texts = {}
    for sample_id, expected_packet in expected_packets.items():
        directory = run_dir / "samples" / sample_id
        packet = BASE.load_json(directory / "render-packet.json")
        require(packet == expected_packet, f"{sample_id} render packet changed")
        render = validate_render_artifact(BASE.load_json(directory / "render.json"), packet,
                                          sha256_file(directory / "render-packet.json"))
        review_packet = semantic_packet(plan, packet, render)
        retained_review_packet = BASE.load_json(directory / "semantic-packet.json")
        require(retained_review_packet == review_packet, f"{sample_id} semantic packet changed")
        result = validate_semantic_result(BASE.load_json(directory / "semantic-result.json"), review_packet,
                                          sha256_file(directory / "semantic-packet.json"))
        require(result["verdict"] == "equivalent", f"{sample_id} is not semantically equivalent")
        accepted_texts[sample_id] = render["text"]
    style_by_condition, neutral_id, profile_bindings = styles(base_plan, base_path)
    pass_seed = f"{plan['rendering_seed']}::matching-pass-{pass_number}"
    condition_order = hashed_order(pass_seed, list(style_by_condition))
    reference_mapping = {f"R{index:02d}": condition for index, condition in enumerate(condition_order, start=1)}
    sample_order = hashed_order(pass_seed, list(accepted_texts))
    text_mapping = {f"T{index:02d}": sample for index, sample in enumerate(sample_order, start=1)}
    references = [{"reference_id": reference_id, "features": style_by_condition[condition]}
                  for reference_id, condition in reference_mapping.items()]
    texts = [{"text_id": text_id, "text": accepted_texts[sample]} for text_id, sample in text_mapping.items()]
    packet = {
        "artifact_type": "linguistic_register_micro_render_matching_packet_v1", "schema_version": SCHEMA_VERSION,
        "experiment_id": plan["experiment_id"], "pass_number": pass_number,
        "plan_artifact_sha256": sha256_file(plan_path), "matching_contract": plan["matching_contract"],
        "anonymous_references": references, "anonymous_texts": texts,
        "authority": "blinded micro-render recognizability matching input; no selection authority",
    }
    sample_conditions = {sample_id: record["condition_id"] for sample_id, record in render_key["samples"].items()}
    key = {
        "artifact_type": "linguistic_register_micro_render_matching_key_v1", "schema_version": SCHEMA_VERSION,
        "experiment_id": plan["experiment_id"], "pass_number": pass_number,
        "plan_artifact_sha256": sha256_file(plan_path), "packet_artifact_sha256": digest_value(packet),
        "reference_mapping": reference_mapping, "text_mapping": text_mapping,
        "condition_by_sample": sample_conditions, "neutral_condition_id": neutral_id,
        "profile_artifact_sha256_by_candidate": profile_bindings,
        "authority": "unblinding and aggregate matching score only",
    }
    return packet, key


def validate_matching_result(value: Any, packet: dict[str, Any], packet_digest: str) -> dict[str, Any]:
    result = obj(value, "matching_result")
    exact(result, {"artifact_type", "schema_version", "experiment_id", "pass_number", "packet_artifact_sha256",
                   "classifier_provenance", "assignments", "method_limitations"}, "matching_result")
    require(result["artifact_type"] == "linguistic_register_micro_render_matching_result_v1",
            "matching result type incompatible")
    require(result["schema_version"] == SCHEMA_VERSION and result["experiment_id"] == packet["experiment_id"]
            and result["pass_number"] == packet["pass_number"], "matching result identity incompatible")
    require(result["packet_artifact_sha256"] == packet_digest, "matching result does not bind packet")
    validate_provenance(result["classifier_provenance"], "matching_result.classifier_provenance")
    text_ids = {item["text_id"] for item in packet["anonymous_texts"]}
    reference_ids = {item["reference_id"] for item in packet["anonymous_references"]}
    assignments = array(result["assignments"], "matching_result.assignments")
    require(len(assignments) == SAMPLE_COUNT, "matching result must assign every text once")
    seen = set()
    for index, assignment_value in enumerate(assignments):
        assignment = obj(assignment_value, f"matching_result.assignments[{index}]")
        exact(assignment, {"text_id", "reference_id", "confidence", "rationale"},
              f"matching_result.assignments[{index}]")
        require(assignment["text_id"] in text_ids and assignment["text_id"] not in seen,
                "matching text identity unknown or duplicated")
        seen.add(assignment["text_id"])
        require(assignment["reference_id"] in reference_ids, "matching reference identity unknown")
        require(isinstance(assignment["confidence"], int) and not isinstance(assignment["confidence"], bool)
                and 1 <= assignment["confidence"] <= 5, "matching confidence must be integer 1 through 5")
        text(assignment["rationale"], f"matching_result.assignments[{index}].rationale")
    for index, limitation in enumerate(array(result["method_limitations"], "matching_result.method_limitations")):
        text(limitation, f"matching_result.method_limitations[{index}]")
    return result


def normalize_matching(raw: dict[str, Any], packet: dict[str, Any], provenance: dict[str, Any]) -> dict[str, Any]:
    exact(raw, {"assignments", "method_limitations"}, "raw_matching_result")
    result = {
        "artifact_type": "linguistic_register_micro_render_matching_result_v1", "schema_version": SCHEMA_VERSION,
        "experiment_id": packet["experiment_id"], "pass_number": packet["pass_number"],
        "packet_artifact_sha256": digest_value(packet), "classifier_provenance": provenance, **raw,
    }
    return validate_matching_result(result, packet, digest_value(packet))


def assess_semantic_gate(plan: dict[str, Any], base_plan: dict[str, Any], base_path: Path,
                         plan_path: Path, run_dir: Path) -> dict[str, Any]:
    expected_packets, expected_key = prepare_render_jobs(plan, base_plan, base_path, plan_path)
    require(BASE.load_json(run_dir / "render-key.json") == expected_key,
            "render key does not match deterministic preparation")
    bindings = []
    rejected = []
    accepted = 0
    for sample_id, expected_packet in sorted(expected_packets.items()):
        directory = run_dir / "samples" / sample_id
        packet = BASE.load_json(directory / "render-packet.json")
        require(packet == expected_packet, f"{sample_id} render packet changed")
        render = validate_render_artifact(BASE.load_json(directory / "render.json"), packet,
                                          sha256_file(directory / "render-packet.json"))
        expected_review_packet = semantic_packet(plan, packet, render)
        retained_review_packet = BASE.load_json(directory / "semantic-packet.json")
        require(retained_review_packet == expected_review_packet, f"{sample_id} semantic packet changed")
        result = validate_semantic_result(BASE.load_json(directory / "semantic-result.json"),
                                          expected_review_packet,
                                          sha256_file(directory / "semantic-packet.json"))
        accepted += int(result["verdict"] == "equivalent")
        bindings.append({
            "sample_id": sample_id,
            "render_artifact_sha256": sha256_file(directory / "render.json"),
            "semantic_result_sha256": sha256_file(directory / "semantic-result.json"),
            "verdict": result["verdict"],
        })
        if result["verdict"] != "equivalent":
            rejected.append({"sample_id": sample_id, "verdict": result["verdict"],
                             "added_meaning": result["added_meaning"]})
    return {
        "accepted_samples": accepted,
        "required_samples": SAMPLE_COUNT,
        "passed": accepted == SAMPLE_COUNT,
        "bindings": bindings,
        "rejected_samples": rejected,
    }


def prepare_adjudication_packet(plan: dict[str, Any], base_plan: dict[str, Any], base_path: Path,
                                plan_path: Path, run_dir: Path) -> dict[str, Any]:
    expected_packets, _ = prepare_render_jobs(plan, base_plan, base_path, plan_path)
    items = []
    for sample_id, expected_packet in sorted(expected_packets.items()):
        directory = run_dir / "samples" / sample_id
        packet = BASE.load_json(directory / "render-packet.json")
        require(packet == expected_packet, f"{sample_id} render packet changed")
        render = validate_render_artifact(BASE.load_json(directory / "render.json"), packet,
                                          sha256_file(directory / "render-packet.json"))
        items.append({"sample_id": sample_id, "semantic_brief": packet["semantic_brief"],
                      "rendered_text": render["text"],
                      "render_artifact_sha256": sha256_file(directory / "render.json")})
    return {
        "artifact_type": "linguistic_register_semantic_adjudication_packet_v1",
        "schema_version": SCHEMA_VERSION,
        "experiment_id": plan["experiment_id"],
        "plan_artifact_sha256": sha256_file(plan_path),
        "items": items,
        "calibration_rules": [
            "Apply one decision standard uniformly across all items; do not infer or score style.",
            "An evaluative modifier adds meaning when its degree, appropriateness, relevance, familiarity, or argumentative status is not stated or entailed by the brief.",
            "Discourse organization, grammatical bridges, and non-evaluative restatement do not alone add meaning.",
            "Judge semantic equivalence independently from transport validity. A rendered text that is itself a serialized JSON object, array, or quoted JSON string is transport-invalid even if its inner prose preserves meaning.",
            "Equivalent requires every proposition preserved, the speech act preserved, and no added meaning.",
        ],
        "blindness": "All sixteen brief/text pairs are visible for calibration; style cards, conditions, original verdicts, hypotheses, and answer keys are withheld.",
        "authority": "post-review adjudication evidence only; does not replace or reopen the frozen semantic gate",
    }


def normalize_adjudication(raw: dict[str, Any], packet: dict[str, Any], provenance_value: dict[str, Any]) -> dict[str, Any]:
    exact(raw, {"decisions", "method_limitations"}, "raw_adjudication")
    decisions = array(raw["decisions"], "raw_adjudication.decisions")
    expected = {item["sample_id"]: item for item in packet["items"]}
    require(len(decisions) == SAMPLE_COUNT, "adjudication must decide every sample once")
    seen = set()
    for index, decision_value in enumerate(decisions):
        path = f"raw_adjudication.decisions[{index}]"
        decision = obj(decision_value, path)
        exact(decision, {"sample_id", "semantic_verdict", "proposition_checks",
                         "speech_act_equivalent", "added_meaning", "transport_valid", "rationale"}, path)
        sample_id = decision["sample_id"]
        require(sample_id in expected and sample_id not in seen, "adjudication sample unknown or duplicated")
        seen.add(sample_id)
        require(decision["semantic_verdict"] in {"equivalent", "not_equivalent", "uncertain"},
                "adjudication semantic verdict invalid")
        proposition_ids = [item["id"] for item in expected[sample_id]["semantic_brief"]["required_propositions"]]
        checks = array(decision["proposition_checks"], f"{path}.proposition_checks")
        require(len(checks) == len(proposition_ids), "adjudication must check every proposition")
        observed = []
        for check_value in checks:
            check = obj(check_value, f"{path}.proposition_checks[]")
            exact(check, {"proposition_id", "status", "evidence"}, f"{path}.proposition_checks[]")
            observed.append(check["proposition_id"])
            require(check["status"] in {"preserved", "omitted", "altered", "uncertain"},
                    "adjudication proposition status invalid")
            text(check["evidence"], f"{path}.proposition_checks[].evidence")
        require(sorted(observed) == sorted(proposition_ids) and len(set(observed)) == len(observed),
                "adjudication proposition IDs incomplete or duplicated")
        require(isinstance(decision["speech_act_equivalent"], bool), "adjudication speech act must be boolean")
        additions = array(decision["added_meaning"], f"{path}.added_meaning")
        for addition in additions:
            text(addition, f"{path}.added_meaning[]")
        require(isinstance(decision["transport_valid"], bool), "adjudication transport validity must be boolean")
        text(decision["rationale"], f"{path}.rationale")
        mechanically_equivalent = (all(check["status"] == "preserved" for check in checks)
                                   and decision["speech_act_equivalent"] and not additions)
        require((decision["semantic_verdict"] == "equivalent") == mechanically_equivalent,
                "adjudication verdict conflicts with detailed checks")
    for limitation in array(raw["method_limitations"], "raw_adjudication.method_limitations"):
        text(limitation, "raw_adjudication.method_limitations[]")
    provenance_checked = validate_provenance(provenance_value, "adjudicator_provenance")
    return {
        "artifact_type": "linguistic_register_semantic_adjudication_result_v1",
        "schema_version": SCHEMA_VERSION,
        "experiment_id": packet["experiment_id"],
        "packet_artifact_sha256": digest_value(packet),
        "adjudicator_provenance": provenance_checked,
        "decisions": decisions,
        "method_limitations": raw["method_limitations"],
        "authority": "post-review adjudication evidence only; original semantic gate remains frozen",
    }


def aggregate(plan: dict[str, Any], render_key: dict[str, Any], matching_runs: list[tuple[dict[str, Any], dict[str, Any], dict[str, Any]]],
              plan_path: Path, semantic_assessment: dict[str, Any]) -> dict[str, Any]:
    all_semantic = semantic_assessment["passed"]
    if all_semantic:
        require(len(matching_runs) == MATCH_PASSES, "all matching passes are required after semantic acceptance")
    else:
        require(not matching_runs, "matching must not run after semantic-gate failure")
    conditions = sorted({record["condition_id"] for record in render_key["samples"].values()})
    total_correct = {condition: 0 for condition in conditions}
    per_pass_correct = {condition: [] for condition in conditions}
    confidence_correct = {condition: [] for condition in conditions}
    run_bindings = []
    for packet, key, result in matching_runs:
        reference_for_condition = {condition: reference_id for reference_id, condition in key["reference_mapping"].items()}
        sample_for_text = key["text_mapping"]
        pass_counts = {condition: 0 for condition in conditions}
        for assignment in result["assignments"]:
            sample = sample_for_text[assignment["text_id"]]
            condition = key["condition_by_sample"][sample]
            correct = assignment["reference_id"] == reference_for_condition[condition]
            total_correct[condition] += int(correct)
            pass_counts[condition] += int(correct)
            if correct:
                confidence_correct[condition].append(assignment["confidence"])
        for condition in conditions:
            per_pass_correct[condition].append(pass_counts[condition])
        run_bindings.append({"pass_number": packet["pass_number"], "packet_artifact_sha256": digest_value(packet),
                             "key_artifact_sha256": digest_value(key), "result_artifact_sha256": digest_value(result)})
    thresholds = plan["thresholds"]
    results = []
    if all_semantic:
        for condition in conditions:
            passing_passes = sum(value >= thresholds["minimum_correct_per_condition_in_passing_pass"]
                                 for value in per_pass_correct[condition])
            checks = {
                "total_correct_assignments": total_correct[condition] >= thresholds["minimum_correct_assignments_per_condition"],
                "passing_match_passes": passing_passes >= thresholds["minimum_passing_match_passes_per_condition"],
            }
            results.append({
                "condition_id": condition, "correct_assignments": total_correct[condition],
                "possible_assignments": MATCH_PASSES * BRIEF_COUNT * REPLICAS,
                "correct_by_pass": per_pass_correct[condition], "passing_match_passes": passing_passes,
                "median_confidence_when_correct": (statistics.median(confidence_correct[condition])
                                                   if confidence_correct[condition] else None),
                "checks": checks, "passed": all(checks.values()),
            })
    gate_passed = all_semantic and all(item["passed"] for item in results)
    return {
        "artifact_type": "linguistic_register_micro_render_report_v1", "schema_version": SCHEMA_VERSION,
        "experiment_id": plan["experiment_id"], "plan_artifact_sha256": sha256_file(plan_path),
        "semantic_equivalence": semantic_assessment,
        "matching_status": "completed" if all_semantic else "not_run_semantic_gate_failed",
        "matching_runs": run_bindings, "condition_results": results,
        "gate_passed": gate_passed,
        "disposition": "micro_render_recognizable" if gate_passed else "micro_render_not_established",
        "authority": "micro-render selection evidence only; no full role-artifact or behavioral-trial authority",
    }


def provenance(args: argparse.Namespace) -> dict[str, Any]:
    return {"provider": args.provider, "model": args.model, "reasoning_effort": args.reasoning_effort,
            "execution_mode": args.execution_mode, "fresh_context": True, "packet_only": True,
            "started_at_utc": args.started_at_utc, "completed_at_utc": args.completed_at_utc}


def emit(value: Any, output: Path) -> None:
    require(not output.exists(), f"refusing to overwrite existing output: {output}")
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_bytes(canonical_bytes(value))


def add_provenance_arguments(parser: argparse.ArgumentParser) -> None:
    for name in ("provider", "model", "reasoning-effort", "execution-mode", "started-at-utc", "completed-at-utc"):
        parser.add_argument(f"--{name}", required=True)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest="command", required=True)
    validate = sub.add_parser("validate-plan"); validate.add_argument("plan", type=Path)
    prepare = sub.add_parser("prepare-render-jobs"); prepare.add_argument("plan", type=Path); prepare.add_argument("--run-dir", required=True, type=Path)
    nr = sub.add_parser("normalize-render"); nr.add_argument("--plan", required=True, type=Path); nr.add_argument("--packet", required=True, type=Path); nr.add_argument("--raw", required=True, type=Path); nr.add_argument("--output", required=True, type=Path); add_provenance_arguments(nr)
    sp = sub.add_parser("prepare-semantic-job"); sp.add_argument("--plan", required=True, type=Path); sp.add_argument("--render-packet", required=True, type=Path); sp.add_argument("--render", required=True, type=Path); sp.add_argument("--output", required=True, type=Path)
    ns = sub.add_parser("normalize-semantic"); ns.add_argument("--plan", required=True, type=Path); ns.add_argument("--packet", required=True, type=Path); ns.add_argument("--raw", required=True, type=Path); ns.add_argument("--output", required=True, type=Path); add_provenance_arguments(ns)
    pm = sub.add_parser("prepare-matching"); pm.add_argument("--plan", required=True, type=Path); pm.add_argument("--run-dir", required=True, type=Path); pm.add_argument("--output-dir", required=True, type=Path)
    nm = sub.add_parser("normalize-matching"); nm.add_argument("--plan", required=True, type=Path); nm.add_argument("--packet", required=True, type=Path); nm.add_argument("--raw", required=True, type=Path); nm.add_argument("--output", required=True, type=Path); add_provenance_arguments(nm)
    ag = sub.add_parser("aggregate"); ag.add_argument("--plan", required=True, type=Path); ag.add_argument("--run-dir", required=True, type=Path); ag.add_argument("--output", required=True, type=Path)
    pa = sub.add_parser("prepare-adjudication"); pa.add_argument("--plan", required=True, type=Path); pa.add_argument("--run-dir", required=True, type=Path); pa.add_argument("--output", required=True, type=Path)
    na = sub.add_parser("normalize-adjudication"); na.add_argument("--plan", required=True, type=Path); na.add_argument("--run-dir", required=True, type=Path); na.add_argument("--packet", required=True, type=Path); na.add_argument("--raw", required=True, type=Path); na.add_argument("--output", required=True, type=Path); add_provenance_arguments(na)
    args = parser.parse_args()
    plan_path = args.plan
    plan, base_plan, base_path = validate_plan(BASE.load_yaml(plan_path), plan_path)
    if args.command == "validate-plan":
        print(json.dumps({"status": "valid", "artifact_type": plan["artifact_type"]}, sort_keys=True)); return 0
    if args.command == "prepare-render-jobs":
        require(not args.run_dir.exists(), f"refusing to overwrite run directory: {args.run_dir}")
        packets, key = prepare_render_jobs(plan, base_plan, base_path, plan_path)
        emit(key, args.run_dir / "render-key.json")
        for sample_id, packet in packets.items(): emit(packet, args.run_dir / "samples" / sample_id / "render-packet.json")
    elif args.command == "normalize-render":
        packet = BASE.load_json(args.packet)
        expected_packets, _ = prepare_render_jobs(plan, base_plan, base_path, plan_path)
        require(packet == expected_packets.get(packet.get("sample_id")), "render packet does not match deterministic preparation")
        emit(normalize_render(BASE.load_json(args.raw), packet, provenance(args)), args.output)
    elif args.command == "prepare-semantic-job":
        packet = BASE.load_json(args.render_packet)
        expected_packets, _ = prepare_render_jobs(plan, base_plan, base_path, plan_path)
        require(packet == expected_packets.get(packet.get("sample_id")), "render packet does not match deterministic preparation")
        render = validate_render_artifact(BASE.load_json(args.render), packet, sha256_file(args.render_packet))
        emit(semantic_packet(plan, packet, render), args.output)
    elif args.command == "normalize-semantic":
        packet = BASE.load_json(args.packet); emit(normalize_semantic(BASE.load_json(args.raw), packet, provenance(args)), args.output)
    elif args.command == "prepare-matching":
        require(not args.output_dir.exists(), f"refusing to overwrite matching directory: {args.output_dir}")
        for pass_number in range(1, MATCH_PASSES + 1):
            packet, key = prepare_matching_pass(plan, base_plan, base_path, plan_path, args.run_dir, pass_number)
            emit(packet, args.output_dir / f"pass-{pass_number:02d}" / "packet.json")
            emit(key, args.output_dir / f"pass-{pass_number:02d}" / "key.json")
    elif args.command == "normalize-matching":
        packet = BASE.load_json(args.packet); emit(normalize_matching(BASE.load_json(args.raw), packet, provenance(args)), args.output)
    elif args.command == "prepare-adjudication":
        emit(prepare_adjudication_packet(plan, base_plan, base_path, plan_path, args.run_dir), args.output)
    elif args.command == "normalize-adjudication":
        packet = BASE.load_json(args.packet)
        require(packet == prepare_adjudication_packet(plan, base_plan, base_path, plan_path,
                                                      args.run_dir),
                "adjudication packet does not match retained run")
        emit(normalize_adjudication(BASE.load_json(args.raw), packet, provenance(args)), args.output)
    else:
        render_key = BASE.load_json(args.run_dir / "render-key.json")
        semantic_assessment = assess_semantic_gate(plan, base_plan, base_path, plan_path, args.run_dir)
        runs = []
        if semantic_assessment["passed"]:
            for pass_number in range(1, MATCH_PASSES + 1):
                directory = args.run_dir / "matching" / f"pass-{pass_number:02d}"
                packet, key, result = (BASE.load_json(directory / name)
                                       for name in ("packet.json", "key.json", "result.json"))
                expected_packet, expected_key = prepare_matching_pass(
                    plan, base_plan, base_path, plan_path, args.run_dir, pass_number)
                require(packet == expected_packet and key == expected_key,
                        f"matching pass {pass_number} changed")
                validate_matching_result(result, packet, sha256_file(directory / "packet.json"))
                runs.append((packet, key, result))
        else:
            require(not (args.run_dir / "matching").exists(),
                    "matching artifacts exist despite semantic-gate failure")
        emit(aggregate(plan, render_key, runs, plan_path, semantic_assessment), args.output)
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (MicroRenderError, BASE.RecognizabilityError) as error:
        print(f"micro_render_artifacts: {error}", file=sys.stderr)
        raise SystemExit(2)
