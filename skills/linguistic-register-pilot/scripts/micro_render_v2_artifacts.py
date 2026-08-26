#!/usr/bin/env python3
"""Prepare, validate, and score the semantic-licensed micro-render v2 experiment."""

from __future__ import annotations

import argparse
import datetime as dt
import hashlib
import importlib.util
import json
import statistics
import subprocess
import sys
from pathlib import Path
from typing import Any


V1_PATH = Path(__file__).with_name("micro_render_artifacts.py")
SPEC = importlib.util.spec_from_file_location("micro_render_v1_for_v2", V1_PATH)
if SPEC is None or SPEC.loader is None:
    raise RuntimeError("cannot load micro-render v1 utilities")
V1 = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(V1)

SCHEMA_VERSION = 2
BRIEF_COUNT = 2
CONDITION_COUNT = 4
FEATURE_COUNT = 5
REPLICAS = 2
SAMPLE_COUNT = BRIEF_COUNT * CONDITION_COUNT * REPLICAS
MATCH_PASSES = 3
PROPOSITION_COUNT = 6


class V2Error(ValueError):
    pass


def require(condition: bool, message: str) -> None:
    if not condition:
        raise V2Error(message)


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


def canonical(value: Any) -> bytes:
    return (json.dumps(value, indent=2, sort_keys=True) + "\n").encode()


def digest(value: Any) -> str:
    return hashlib.sha256(canonical(value)).hexdigest()


def file_digest(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def emit(value: Any, output: Path) -> None:
    require(not output.exists(), f"refusing to overwrite output: {output}")
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_bytes(canonical(value))


def binding_path(repository: Path, binding_value: Any, path: str) -> Path:
    binding = obj(binding_value, path)
    exact(binding, {"path", "sha256"}, path)
    relative = Path(text(binding["path"], f"{path}.path"))
    require(not relative.is_absolute() and ".." not in relative.parts, f"{path}.path escapes repository")
    result = repository / relative
    require(result.is_file(), f"{path}.path is not readable")
    require(file_digest(result) == V1.BASE.sha256(binding["sha256"], f"{path}.sha256"),
            f"{path} digest mismatch")
    return result


def validate_feature(value: Any, path: str, *, profile_features: dict[str, dict[str, Any]] | None) -> dict[str, Any]:
    feature = obj(value, path)
    exact(feature, {"source_feature_id", "layer", "category", "salience_weight", "licensed_instruction"}, path)
    feature_id = V1.BASE.identifier(feature["source_feature_id"], f"{path}.source_feature_id")
    require(feature["layer"] in {"surface", "discourse"}, f"{path}.layer invalid")
    text(feature["category"], f"{path}.category")
    require(isinstance(feature["salience_weight"], int) and not isinstance(feature["salience_weight"], bool)
            and 1 <= feature["salience_weight"] <= 5, f"{path}.salience_weight invalid")
    text(feature["licensed_instruction"], f"{path}.licensed_instruction")
    if profile_features is not None:
        require(feature_id in profile_features, f"{path} source feature missing from bound profile")
        source = profile_features[feature_id]
        require(source["disposition"] == "realization_only", f"{path} source feature is not realization_only")
        require(feature["layer"] == source["layer"] and feature["category"] == source["category"]
                and feature["salience_weight"] == source["distinctiveness_weight"],
                f"{path} source metadata mismatch")
    return feature


def validate_plan(value: Any, plan_path: Path) -> tuple[dict[str, Any], Path]:
    plan = obj(value, "plan")
    exact(plan, {"artifact_type", "schema_version", "status", "experiment_id", "repository",
                 "prior_run_qualification", "briefs", "candidate_styles", "neutral_style",
                 "semantic_license", "rendering", "semantic_adjudication", "matching", "thresholds",
                 "execution", "preregistration_owner", "limitations"}, "plan")
    require(plan["artifact_type"] == "linguistic_register_semantic_licensed_micro_render_plan_v2",
            "plan type incompatible")
    require(plan["schema_version"] == SCHEMA_VERSION and plan["status"] == "frozen",
            "plan version/status incompatible")
    text(plan["experiment_id"], "plan.experiment_id")
    repository = Path(text(plan["repository"], "plan.repository"))
    require(repository.is_absolute() and repository.is_dir(), "plan.repository invalid")
    binding_path(repository, plan["prior_run_qualification"], "plan.prior_run_qualification")

    briefs = array(plan["briefs"], "plan.briefs")
    require(len(briefs) == BRIEF_COUNT, f"plan.briefs must contain {BRIEF_COUNT} items")
    brief_ids = set()
    for index, brief_value in enumerate(briefs):
        path = f"plan.briefs[{index}]"
        brief = obj(brief_value, path)
        exact(brief, {"brief_id", "context", "required_propositions", "required_speech_act", "output_constraint"}, path)
        brief_id = V1.BASE.identifier(brief["brief_id"], f"{path}.brief_id")
        require(brief_id not in brief_ids, "brief IDs must be unique")
        brief_ids.add(brief_id)
        text(brief["context"], f"{path}.context")
        propositions = array(brief["required_propositions"], f"{path}.required_propositions")
        require(len(propositions) == PROPOSITION_COUNT, f"{path} must contain six propositions")
        proposition_ids = set()
        for p_index, proposition_value in enumerate(propositions):
            proposition = obj(proposition_value, f"{path}.required_propositions[{p_index}]")
            exact(proposition, {"id", "meaning"}, f"{path}.required_propositions[{p_index}]")
            proposition_id = V1.BASE.identifier(proposition["id"], f"{path}.required_propositions[{p_index}].id")
            require(proposition_id not in proposition_ids, "proposition IDs must be unique")
            proposition_ids.add(proposition_id)
            text(proposition["meaning"], f"{path}.required_propositions[{p_index}].meaning")
        text(brief["required_speech_act"], f"{path}.required_speech_act")
        text(brief["output_constraint"], f"{path}.output_constraint")

    styles = array(plan["candidate_styles"], "plan.candidate_styles")
    require(len(styles) == CONDITION_COUNT - 1, "plan.candidate_styles must contain three items")
    condition_ids = set()
    for index, style_value in enumerate(styles):
        path = f"plan.candidate_styles[{index}]"
        style = obj(style_value, path)
        exact(style, {"condition_id", "profile", "features"}, path)
        condition_id = V1.BASE.identifier(style["condition_id"], f"{path}.condition_id")
        require(condition_id not in condition_ids, "condition IDs must be unique")
        condition_ids.add(condition_id)
        profile_path = binding_path(repository, style["profile"], f"{path}.profile")
        profile = V1.BASE.load_yaml(profile_path)
        require(profile.get("candidate_id") == condition_id, f"{path}.profile candidate mismatch")
        profile_features = {item["id"]: item for item in profile.get("features", [])}
        features = array(style["features"], f"{path}.features")
        require(len(features) == FEATURE_COUNT, f"{path}.features must contain five items")
        seen = set()
        for f_index, feature_value in enumerate(features):
            feature = validate_feature(feature_value, f"{path}.features[{f_index}]", profile_features=profile_features)
            require(feature["source_feature_id"] not in seen, "source feature IDs must be unique per style")
            seen.add(feature["source_feature_id"])

    neutral = obj(plan["neutral_style"], "plan.neutral_style")
    exact(neutral, {"condition_id", "construction_basis", "features"}, "plan.neutral_style")
    neutral_id = V1.BASE.identifier(neutral["condition_id"], "plan.neutral_style.condition_id")
    require(neutral_id not in condition_ids, "neutral condition duplicates candidate")
    condition_ids.add(neutral_id)
    text(neutral["construction_basis"], "plan.neutral_style.construction_basis")
    neutral_features = array(neutral["features"], "plan.neutral_style.features")
    require(len(neutral_features) == FEATURE_COUNT, "neutral style must contain five features")
    for index, feature in enumerate(neutral_features):
        validate_feature(feature, f"plan.neutral_style.features[{index}]", profile_features=None)

    license_value = obj(plan["semantic_license"], "plan.semantic_license")
    exact(license_value, {"invariant", "prohibited_additions"}, "plan.semantic_license")
    text(license_value["invariant"], "plan.semantic_license.invariant")
    text(license_value["prohibited_additions"], "plan.semantic_license.prohibited_additions")
    rendering = obj(plan["rendering"], "plan.rendering")
    exact(rendering, {"replicas_per_brief_condition", "seed", "task", "blindness"}, "plan.rendering")
    require(rendering["replicas_per_brief_condition"] == REPLICAS, "render replicas invalid")
    for key in ("seed", "task", "blindness"):
        text(rendering[key], f"plan.rendering.{key}")
    semantic = obj(plan["semantic_adjudication"], "plan.semantic_adjudication")
    exact(semantic, {"task", "blindness", "equivalence_rule"}, "plan.semantic_adjudication")
    for key in semantic:
        text(semantic[key], f"plan.semantic_adjudication.{key}")
    matching = obj(plan["matching"], "plan.matching")
    exact(matching, {"task", "blindness", "score_scale", "passes"}, "plan.matching")
    for key in ("task", "blindness", "score_scale"):
        text(matching[key], f"plan.matching.{key}")
    require(matching["passes"] == MATCH_PASSES, "matching pass count invalid")
    thresholds = obj(plan["thresholds"], "plan.thresholds")
    exact(thresholds, {"require_all_semantically_equivalent", "minimum_correct_assignments_per_condition",
                       "minimum_passing_match_passes_per_condition", "minimum_correct_per_condition_in_passing_pass"},
          "plan.thresholds")
    require(thresholds["require_all_semantically_equivalent"] is True, "semantic all-pass gate required")
    require(thresholds["minimum_correct_assignments_per_condition"] == 9
            and thresholds["minimum_passing_match_passes_per_condition"] == 2
            and thresholds["minimum_correct_per_condition_in_passing_pass"] == 3,
            "matching thresholds differ from frozen v2 design")

    execution = obj(plan["execution"], "plan.execution")
    exact(execution, {"runner", "render", "semantic_adjudication", "matching"}, "plan.execution")
    binding_path(repository, execution["runner"], "plan.execution.runner")
    for stage in ("render", "semantic_adjudication", "matching"):
        config = obj(execution[stage], f"plan.execution.{stage}")
        exact(config, {"provider", "model", "reasoning_effort", "prompt", "schema"}, f"plan.execution.{stage}")
        for key in ("provider", "model", "reasoning_effort", "prompt"):
            text(config[key], f"plan.execution.{stage}.{key}")
        binding_path(repository, config["schema"], f"plan.execution.{stage}.schema")
    text(plan["preregistration_owner"], "plan.preregistration_owner")
    for index, limitation in enumerate(array(plan["limitations"], "plan.limitations")):
        text(limitation, f"plan.limitations[{index}]")
    require(plan_path.resolve().is_relative_to(repository.resolve()), "plan must be inside repository")
    return plan, repository


def condition_styles(plan: dict[str, Any]) -> dict[str, list[dict[str, Any]]]:
    result = {item["condition_id"]: item["features"] for item in plan["candidate_styles"]}
    result[plan["neutral_style"]["condition_id"]] = plan["neutral_style"]["features"]
    return result


def anonymous_style(features: list[dict[str, Any]]) -> list[dict[str, Any]]:
    return [{"feature_id": f"F{index:02d}", "layer": feature["layer"], "category": feature["category"],
             "salience_weight": feature["salience_weight"],
             "licensed_instruction": feature["licensed_instruction"]}
            for index, feature in enumerate(features, 1)]


def hashed_order(seed: str, values: list[str]) -> list[str]:
    return sorted(values, key=lambda value: hashlib.sha256(f"{seed}\0{value}".encode()).hexdigest())


def verify_preoutcome_checkpoint(repository: Path, plan_path: Path, checkpoint: str) -> str:
    require(isinstance(checkpoint, str) and len(checkpoint) == 40
            and all(character in "0123456789abcdef" for character in checkpoint),
            "pre-outcome checkpoint must be a full commit OID")
    resolved = subprocess.run(["git", "-C", str(repository), "rev-parse", f"{checkpoint}^{{commit}}"],
                              check=False, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
    require(resolved.returncode == 0 and resolved.stdout.decode().strip() == checkpoint,
            "pre-outcome checkpoint is not an available commit")
    relative = plan_path.resolve().relative_to(repository.resolve()).as_posix()
    retained = subprocess.run(["git", "-C", str(repository), "show", f"{checkpoint}:{relative}"],
                              check=False, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
    require(retained.returncode == 0, "pre-outcome checkpoint does not contain the plan")
    require(hashlib.sha256(retained.stdout).hexdigest() == file_digest(plan_path),
            "pre-outcome checkpoint plan differs from working plan")
    return checkpoint


def prepare_render_jobs(plan: dict[str, Any], plan_path: Path, checkpoint: str) -> tuple[dict[str, dict[str, Any]], dict[str, Any]]:
    styles = condition_styles(plan)
    identities = [f"{brief['brief_id']}::{condition}::replica-{replica}"
                  for brief in plan["briefs"] for condition in sorted(styles)
                  for replica in range(1, REPLICAS + 1)]
    ordered = hashed_order(plan["rendering"]["seed"], identities)
    briefs = {brief["brief_id"]: brief for brief in plan["briefs"]}
    packets = {}
    records = {}
    for index, identity in enumerate(ordered, 1):
        brief_id, condition_id, replica_label = identity.split("::")
        sample_id = f"S{index:02d}"
        packet = {
            "artifact_type": "linguistic_register_semantic_licensed_render_job_v2",
            "schema_version": SCHEMA_VERSION,
            "experiment_id": plan["experiment_id"],
            "sample_id": sample_id,
            "plan_artifact_sha256": file_digest(plan_path),
            "preoutcome_checkpoint_commit_oid": checkpoint,
            "semantic_brief": briefs[brief_id],
            "anonymous_licensed_style": anonymous_style(styles[condition_id]),
            "semantic_license": plan["semantic_license"],
            "rendering_contract": {"task": plan["rendering"]["task"], "blindness": plan["rendering"]["blindness"]},
            "authority": "packet-only v2 render input; no matching or selection authority",
        }
        packets[sample_id] = packet
        records[sample_id] = {"identity": identity, "brief_id": brief_id, "condition_id": condition_id,
                              "replica": int(replica_label.removeprefix("replica-")),
                              "packet_artifact_sha256": digest(packet)}
    key = {
        "artifact_type": "linguistic_register_semantic_licensed_render_key_v2",
        "schema_version": SCHEMA_VERSION,
        "experiment_id": plan["experiment_id"],
        "plan_artifact_sha256": file_digest(plan_path),
        "preoutcome_checkpoint_commit_oid": checkpoint,
        "samples": records,
        "neutral_condition_id": plan["neutral_style"]["condition_id"],
        "authority": "unblinding, semantic aggregation, and matching score only",
    }
    return packets, key


def parse_time(value: Any, path: str) -> dt.datetime:
    raw = text(value, path)
    try:
        parsed = dt.datetime.fromisoformat(raw.replace("Z", "+00:00"))
    except ValueError as error:
        raise V2Error(f"{path} must be ISO-8601") from error
    require(parsed.tzinfo is not None, f"{path} must include an offset")
    return parsed


def validate_receipt(value: Any, *, stage: str, packet_path: Path, raw_path: Path, events_path: Path,
                     plan: dict[str, Any]) -> dict[str, Any]:
    receipt = obj(value, "execution_receipt")
    exact(receipt, {"artifact_type", "schema_version", "stage", "provider", "model", "reasoning_effort",
                    "launcher", "launcher_version", "started_at_utc", "completed_at_utc", "return_code",
                    "fresh_process", "ephemeral", "sandbox", "isolated_temporary_directory",
                    "visible_input_files", "packet_artifact_sha256", "output_schema_sha256", "prompt_sha256",
                    "raw_output_sha256", "events_output_sha256", "stderr_sha256"}, "execution_receipt")
    require(receipt["artifact_type"] == "linguistic_register_model_execution_receipt_v1"
            and receipt["schema_version"] == 1 and receipt["stage"] == stage, "receipt identity incompatible")
    config_key = "semantic_adjudication" if stage == "semantic-adjudication" else stage
    config = plan["execution"][config_key]
    require(receipt["provider"] == config["provider"] and receipt["model"] == config["model"]
            and receipt["reasoning_effort"] == config["reasoning_effort"], "receipt model configuration mismatch")
    require(receipt["launcher"] == "codex exec" and text(receipt["launcher_version"], "receipt.launcher_version"),
            "receipt launcher invalid")
    require(receipt["return_code"] == 0 and receipt["fresh_process"] is True and receipt["ephemeral"] is True
            and receipt["sandbox"] == "read-only" and receipt["isolated_temporary_directory"] is True,
            "receipt execution isolation invalid")
    require(receipt["visible_input_files"] == ["packet.json", "output.schema.json"],
            "receipt visible inputs differ")
    started = parse_time(receipt["started_at_utc"], "receipt.started_at_utc")
    completed = parse_time(receipt["completed_at_utc"], "receipt.completed_at_utc")
    require(started <= completed, "receipt completion precedes start")
    schema_path = binding_path(Path(plan["repository"]), config["schema"], f"plan.execution.{config_key}.schema")
    require(receipt["packet_artifact_sha256"] == file_digest(packet_path), "receipt packet digest mismatch")
    require(receipt["output_schema_sha256"] == file_digest(schema_path), "receipt schema digest mismatch")
    require(receipt["prompt_sha256"] == hashlib.sha256(config["prompt"].encode()).hexdigest(),
            "receipt prompt digest mismatch")
    require(receipt["raw_output_sha256"] == file_digest(raw_path), "receipt raw-output digest mismatch")
    require(receipt["events_output_sha256"] == file_digest(events_path), "receipt events digest mismatch")
    V1.BASE.sha256(receipt["stderr_sha256"], "receipt.stderr_sha256")
    return receipt


def validate_prose(value: Any) -> str:
    rendered = text(value, "raw_render.text")
    stripped = rendered.strip()
    try:
        nested = json.loads(stripped)
    except json.JSONDecodeError:
        nested = None
    require(not isinstance(nested, (dict, list, str)), "render text is serialized structured output")
    require(rendered == stripped, "render text must not have surrounding whitespace")
    require("\n" not in rendered and "\r" not in rendered, "render text must be one physical paragraph")
    require(not any(rendered.lstrip().startswith(prefix) for prefix in ("#", "- ", "* ", "•")),
            "render text must not begin with heading or bullet syntax")
    words = rendered.split()
    require(90 <= len(words) <= 130, "render text must contain 90 through 130 words")
    return rendered


def normalize_render(raw: Any, packet: dict[str, Any], receipt: dict[str, Any], receipt_path: Path) -> dict[str, Any]:
    raw_value = obj(raw, "raw_render")
    exact(raw_value, {"text"}, "raw_render")
    rendered = validate_prose(raw_value["text"])
    return {
        "artifact_type": "linguistic_register_semantic_licensed_render_v2",
        "schema_version": SCHEMA_VERSION,
        "experiment_id": packet["experiment_id"],
        "sample_id": packet["sample_id"],
        "packet_artifact_sha256": digest(packet),
        "execution_receipt_sha256": file_digest(receipt_path),
        "model": receipt["model"],
        "text": rendered,
    }


def validate_render(value: Any, packet: dict[str, Any], receipt_path: Path) -> dict[str, Any]:
    render = obj(value, "render")
    exact(render, {"artifact_type", "schema_version", "experiment_id", "sample_id", "packet_artifact_sha256",
                   "execution_receipt_sha256", "model", "text"}, "render")
    require(render["artifact_type"] == "linguistic_register_semantic_licensed_render_v2"
            and render["schema_version"] == SCHEMA_VERSION, "render identity incompatible")
    require(render["experiment_id"] == packet["experiment_id"] and render["sample_id"] == packet["sample_id"],
            "render sample identity mismatch")
    require(render["packet_artifact_sha256"] == digest(packet), "render packet binding mismatch")
    require(render["execution_receipt_sha256"] == file_digest(receipt_path), "render receipt binding mismatch")
    validate_prose(render["text"])
    return render


def prepare_semantic_packet(plan: dict[str, Any], plan_path: Path, run_dir: Path) -> dict[str, Any]:
    key = V1.BASE.load_json(run_dir / "render-key.json")
    items = []
    for sample_id in sorted(key["samples"]):
        directory = run_dir / "samples" / sample_id
        packet_path = directory / "render-packet.json"
        raw_path = directory / "raw-render.json"
        events_path = directory / "render-events.jsonl"
        receipt_path = directory / "render-execution-receipt.json"
        packet = V1.BASE.load_json(packet_path)
        receipt = validate_receipt(V1.BASE.load_json(receipt_path), stage="render", packet_path=packet_path,
                                   raw_path=raw_path, events_path=events_path, plan=plan)
        render = validate_render(V1.BASE.load_json(directory / "render.json"), packet, receipt_path)
        require(render["model"] == receipt["model"], f"{sample_id} render model mismatch")
        items.append({"sample_id": sample_id, "semantic_brief": packet["semantic_brief"],
                      "rendered_text": render["text"], "render_artifact_sha256": file_digest(directory / "render.json")})
    return {
        "artifact_type": "linguistic_register_semantic_licensed_adjudication_packet_v2",
        "schema_version": SCHEMA_VERSION,
        "experiment_id": plan["experiment_id"],
        "plan_artifact_sha256": file_digest(plan_path),
        "items": items,
        "contract": plan["semantic_adjudication"],
        "authority": "all-sample style-blind semantic gate input; no matching or selection authority",
    }


def normalize_semantic(raw: Any, packet: dict[str, Any], receipt_path: Path) -> dict[str, Any]:
    raw_value = obj(raw, "raw_semantic")
    exact(raw_value, {"decisions", "method_limitations"}, "raw_semantic")
    expected = {item["sample_id"]: item for item in packet["items"]}
    decisions = array(raw_value["decisions"], "raw_semantic.decisions")
    require(len(decisions) == SAMPLE_COUNT, "semantic result must decide all sixteen samples")
    seen = set()
    normalized = []
    for index, decision_value in enumerate(decisions):
        path = f"raw_semantic.decisions[{index}]"
        decision = obj(decision_value, path)
        exact(decision, {"sample_id", "verdict", "proposition_checks", "speech_act_equivalent",
                         "added_meaning", "rationale"}, path)
        sample_id = decision["sample_id"]
        require(sample_id in expected and sample_id not in seen, "semantic sample unknown or duplicated")
        seen.add(sample_id)
        require(decision["verdict"] in {"equivalent", "not_equivalent", "uncertain"}, "semantic verdict invalid")
        proposition_ids = [item["id"] for item in expected[sample_id]["semantic_brief"]["required_propositions"]]
        checks = array(decision["proposition_checks"], f"{path}.proposition_checks")
        require(len(checks) == PROPOSITION_COUNT, "semantic decision must check all propositions")
        observed = []
        for check_value in checks:
            check = obj(check_value, f"{path}.proposition_checks[]")
            exact(check, {"proposition_id", "status", "evidence"}, f"{path}.proposition_checks[]")
            observed.append(check["proposition_id"])
            require(check["status"] in {"preserved", "omitted", "altered", "uncertain"},
                    "semantic proposition status invalid")
            text(check["evidence"], f"{path}.proposition_checks[].evidence")
        require(sorted(observed) == sorted(proposition_ids) and len(set(observed)) == len(observed),
                "semantic proposition IDs incomplete or duplicated")
        require(isinstance(decision["speech_act_equivalent"], bool), "semantic speech act must be boolean")
        additions = array(decision["added_meaning"], f"{path}.added_meaning")
        for addition in additions:
            text(addition, f"{path}.added_meaning[]")
        text(decision["rationale"], f"{path}.rationale")
        equivalent = all(check["status"] == "preserved" for check in checks) and decision["speech_act_equivalent"] and not additions
        require((decision["verdict"] == "equivalent") == equivalent,
                "semantic verdict conflicts with detailed checks")
        normalized.append(decision)
    for limitation in array(raw_value["method_limitations"], "raw_semantic.method_limitations"):
        text(limitation, "raw_semantic.method_limitations[]")
    return {
        "artifact_type": "linguistic_register_semantic_licensed_adjudication_result_v2",
        "schema_version": SCHEMA_VERSION,
        "experiment_id": packet["experiment_id"],
        "packet_artifact_sha256": digest(packet),
        "execution_receipt_sha256": file_digest(receipt_path),
        "decisions": normalized,
        "method_limitations": raw_value["method_limitations"],
        "authority": "v2 semantic gate evidence; no matching or selection authority",
    }


def validate_semantic_result(value: Any, packet: dict[str, Any], receipt_path: Path) -> dict[str, Any]:
    result = obj(value, "semantic_result")
    exact(result, {"artifact_type", "schema_version", "experiment_id", "packet_artifact_sha256",
                   "execution_receipt_sha256", "decisions", "method_limitations", "authority"}, "semantic_result")
    require(result["artifact_type"] == "linguistic_register_semantic_licensed_adjudication_result_v2"
            and result["schema_version"] == SCHEMA_VERSION and result["experiment_id"] == packet["experiment_id"],
            "semantic result identity incompatible")
    require(result["packet_artifact_sha256"] == digest(packet), "semantic packet binding mismatch")
    require(result["execution_receipt_sha256"] == file_digest(receipt_path), "semantic receipt binding mismatch")
    reconstructed = normalize_semantic({"decisions": result["decisions"],
                                        "method_limitations": result["method_limitations"]}, packet, receipt_path)
    require(result == reconstructed, "semantic result does not reconstruct")
    return result


def prepare_matching_pass(plan: dict[str, Any], plan_path: Path, run_dir: Path, pass_number: int) -> tuple[dict[str, Any], dict[str, Any]]:
    require(1 <= pass_number <= MATCH_PASSES, "matching pass out of range")
    semantic_packet_path = run_dir / "semantic" / "packet.json"
    semantic_receipt_path = run_dir / "semantic" / "execution-receipt.json"
    semantic_packet = V1.BASE.load_json(semantic_packet_path)
    require(semantic_packet == prepare_semantic_packet(plan, plan_path, run_dir), "semantic packet changed")
    semantic_result = validate_semantic_result(V1.BASE.load_json(run_dir / "semantic" / "result.json"),
                                               semantic_packet, semantic_receipt_path)
    require(all(item["verdict"] == "equivalent" for item in semantic_result["decisions"]),
            "semantic gate failed; matching is prohibited")
    key = V1.BASE.load_json(run_dir / "render-key.json")
    styles = condition_styles(plan)
    seed = f"{plan['rendering']['seed']}::matching-pass-{pass_number}"
    condition_order = hashed_order(seed, list(styles))
    reference_mapping = {f"R{index:02d}": condition for index, condition in enumerate(condition_order, 1)}
    sample_order = hashed_order(seed, sorted(key["samples"]))
    text_mapping = {f"T{index:02d}": sample for index, sample in enumerate(sample_order, 1)}
    references = [{"reference_id": reference_id,
                   "licensed_style_features": anonymous_style(styles[condition])}
                  for reference_id, condition in reference_mapping.items()]
    texts = []
    for text_id, sample_id in text_mapping.items():
        render = V1.BASE.load_json(run_dir / "samples" / sample_id / "render.json")
        texts.append({"text_id": text_id, "text": render["text"]})
    packet = {
        "artifact_type": "linguistic_register_semantic_licensed_matching_packet_v2",
        "schema_version": SCHEMA_VERSION,
        "experiment_id": plan["experiment_id"],
        "pass_number": pass_number,
        "plan_artifact_sha256": file_digest(plan_path),
        "anonymous_references": references,
        "anonymous_texts": texts,
        "contract": plan["matching"],
        "authority": "blinded v2 matching input; no selection authority",
    }
    match_key = {
        "artifact_type": "linguistic_register_semantic_licensed_matching_key_v2",
        "schema_version": SCHEMA_VERSION,
        "experiment_id": plan["experiment_id"],
        "pass_number": pass_number,
        "packet_artifact_sha256": digest(packet),
        "reference_mapping": reference_mapping,
        "text_mapping": text_mapping,
        "condition_by_sample": {sample: record["condition_id"] for sample, record in key["samples"].items()},
        "authority": "matching unblinding and aggregate score only",
    }
    return packet, match_key


def normalize_matching(raw: Any, packet: dict[str, Any], receipt_path: Path) -> dict[str, Any]:
    raw_value = obj(raw, "raw_matching")
    exact(raw_value, {"assignments", "method_limitations"}, "raw_matching")
    assignments = array(raw_value["assignments"], "raw_matching.assignments")
    require(len(assignments) == SAMPLE_COUNT, "matching must assign all sixteen texts")
    text_ids = {item["text_id"] for item in packet["anonymous_texts"]}
    reference_ids = {item["reference_id"] for item in packet["anonymous_references"]}
    seen = set()
    for index, assignment_value in enumerate(assignments):
        path = f"raw_matching.assignments[{index}]"
        assignment = obj(assignment_value, path)
        exact(assignment, {"text_id", "reference_id", "confidence", "rationale"}, path)
        require(assignment["text_id"] in text_ids and assignment["text_id"] not in seen,
                "matching text unknown or duplicated")
        seen.add(assignment["text_id"])
        require(assignment["reference_id"] in reference_ids, "matching reference unknown")
        require(isinstance(assignment["confidence"], int) and not isinstance(assignment["confidence"], bool)
                and 1 <= assignment["confidence"] <= 5, "matching confidence invalid")
        text(assignment["rationale"], f"{path}.rationale")
    for limitation in array(raw_value["method_limitations"], "raw_matching.method_limitations"):
        text(limitation, "raw_matching.method_limitations[]")
    return {
        "artifact_type": "linguistic_register_semantic_licensed_matching_result_v2",
        "schema_version": SCHEMA_VERSION,
        "experiment_id": packet["experiment_id"],
        "pass_number": packet["pass_number"],
        "packet_artifact_sha256": digest(packet),
        "execution_receipt_sha256": file_digest(receipt_path),
        "assignments": assignments,
        "method_limitations": raw_value["method_limitations"],
    }


def validate_matching_result(value: Any, packet: dict[str, Any], receipt_path: Path) -> dict[str, Any]:
    result = obj(value, "matching_result")
    exact(result, {"artifact_type", "schema_version", "experiment_id", "pass_number", "packet_artifact_sha256",
                   "execution_receipt_sha256", "assignments", "method_limitations"}, "matching_result")
    reconstructed = normalize_matching({"assignments": result["assignments"],
                                        "method_limitations": result["method_limitations"]}, packet, receipt_path)
    require(result == reconstructed, "matching result does not reconstruct")
    return result


def aggregate(plan: dict[str, Any], plan_path: Path, run_dir: Path) -> dict[str, Any]:
    semantic_packet_path = run_dir / "semantic" / "packet.json"
    semantic_raw_path = run_dir / "semantic" / "raw-result.json"
    semantic_events_path = run_dir / "semantic" / "events.jsonl"
    semantic_receipt_path = run_dir / "semantic" / "execution-receipt.json"
    semantic_packet = V1.BASE.load_json(semantic_packet_path)
    require(semantic_packet == prepare_semantic_packet(plan, plan_path, run_dir), "semantic packet changed")
    validate_receipt(V1.BASE.load_json(semantic_receipt_path), stage="semantic-adjudication",
                     packet_path=semantic_packet_path, raw_path=semantic_raw_path,
                     events_path=semantic_events_path, plan=plan)
    semantic_result = validate_semantic_result(V1.BASE.load_json(run_dir / "semantic" / "result.json"),
                                               semantic_packet, semantic_receipt_path)
    accepted = sum(item["verdict"] == "equivalent" for item in semantic_result["decisions"])
    semantic_passed = accepted == SAMPLE_COUNT
    condition_results = []
    matching_bindings = []
    if semantic_passed:
        key = V1.BASE.load_json(run_dir / "render-key.json")
        conditions = sorted(condition_styles(plan))
        totals = {condition: 0 for condition in conditions}
        per_pass = {condition: [] for condition in conditions}
        confidence = {condition: [] for condition in conditions}
        for pass_number in range(1, MATCH_PASSES + 1):
            directory = run_dir / "matching" / f"pass-{pass_number:02d}"
            packet_path = directory / "packet.json"
            raw_path = directory / "raw-result.json"
            events_path = directory / "events.jsonl"
            receipt_path = directory / "execution-receipt.json"
            packet = V1.BASE.load_json(packet_path)
            match_key = V1.BASE.load_json(directory / "key.json")
            expected_packet, expected_key = prepare_matching_pass(plan, plan_path, run_dir, pass_number)
            require(packet == expected_packet and match_key == expected_key, "matching packet or key changed")
            validate_receipt(V1.BASE.load_json(receipt_path), stage="matching", packet_path=packet_path,
                             raw_path=raw_path, events_path=events_path, plan=plan)
            result = validate_matching_result(V1.BASE.load_json(directory / "result.json"), packet, receipt_path)
            reference_for = {condition: reference for reference, condition in match_key["reference_mapping"].items()}
            counts = {condition: 0 for condition in conditions}
            for assignment in result["assignments"]:
                sample = match_key["text_mapping"][assignment["text_id"]]
                condition = key["samples"][sample]["condition_id"]
                correct = assignment["reference_id"] == reference_for[condition]
                totals[condition] += int(correct)
                counts[condition] += int(correct)
                if correct:
                    confidence[condition].append(assignment["confidence"])
            for condition in conditions:
                per_pass[condition].append(counts[condition])
            matching_bindings.append({"pass_number": pass_number, "packet_artifact_sha256": file_digest(packet_path),
                                      "result_artifact_sha256": file_digest(directory / "result.json"),
                                      "execution_receipt_sha256": file_digest(receipt_path)})
        thresholds = plan["thresholds"]
        for condition in conditions:
            passing_passes = sum(value >= thresholds["minimum_correct_per_condition_in_passing_pass"]
                                 for value in per_pass[condition])
            checks = {"total": totals[condition] >= thresholds["minimum_correct_assignments_per_condition"],
                      "passes": passing_passes >= thresholds["minimum_passing_match_passes_per_condition"]}
            condition_results.append({"condition_id": condition, "correct_assignments": totals[condition],
                                      "possible_assignments": 12, "correct_by_pass": per_pass[condition],
                                      "passing_match_passes": passing_passes,
                                      "median_confidence_when_correct": (statistics.median(confidence[condition])
                                                                         if confidence[condition] else None),
                                      "checks": checks, "passed": all(checks.values())})
    else:
        require(not (run_dir / "matching").exists(), "matching artifacts exist after semantic failure")
    gate_passed = semantic_passed and all(item["passed"] for item in condition_results)
    return {
        "artifact_type": "linguistic_register_semantic_licensed_micro_render_report_v2",
        "schema_version": SCHEMA_VERSION,
        "experiment_id": plan["experiment_id"],
        "plan_artifact_sha256": file_digest(plan_path),
        "preoutcome_checkpoint_commit_oid": V1.BASE.load_json(run_dir / "render-key.json")["preoutcome_checkpoint_commit_oid"],
        "semantic_gate": {"accepted_samples": accepted, "required_samples": SAMPLE_COUNT,
                          "passed": semantic_passed, "packet_artifact_sha256": file_digest(semantic_packet_path),
                          "result_artifact_sha256": file_digest(run_dir / "semantic" / "result.json"),
                          "execution_receipt_sha256": file_digest(semantic_receipt_path)},
        "matching_status": "completed" if semantic_passed else "not_run_semantic_gate_failed",
        "matching_runs": matching_bindings,
        "condition_results": condition_results,
        "gate_passed": gate_passed,
        "disposition": "micro_render_v2_recognizable" if gate_passed else "micro_render_v2_not_established",
        "authority": "v2 upstream selection evidence only; no corpus-selection, role-artifact, or behavioral-trial authority",
    }


def aggregate_transport_stop(plan: dict[str, Any], plan_path: Path, run_dir: Path) -> dict[str, Any]:
    key = V1.BASE.load_json(run_dir / "render-key.json")
    checkpoint = key["preoutcome_checkpoint_commit_oid"]
    expected_packets, expected_key = prepare_render_jobs(plan, plan_path, checkpoint)
    require(key == expected_key, "render key changed")
    launched = []
    valid = []
    invalid = []
    without_execution_evidence = []
    bindings = []
    for sample_id, expected_packet in sorted(expected_packets.items()):
        directory = run_dir / "samples" / sample_id
        packet_path = directory / "render-packet.json"
        packet = V1.BASE.load_json(packet_path)
        require(packet == expected_packet, f"{sample_id} render packet changed")
        raw_path = directory / "raw-render.json"
        events_path = directory / "render-events.jsonl"
        receipt_path = directory / "render-execution-receipt.json"
        render_path = directory / "render.json"
        present = [path.exists() for path in (raw_path, events_path, receipt_path)]
        require(len(set(present)) == 1, f"{sample_id} has an incomplete execution evidence set")
        if not any(present):
            require(not render_path.exists(), f"{sample_id} has a render without execution evidence")
            without_execution_evidence.append(sample_id)
            continue
        launched.append(sample_id)
        receipt = validate_receipt(V1.BASE.load_json(receipt_path), stage="render", packet_path=packet_path,
                                   raw_path=raw_path, events_path=events_path, plan=plan)
        raw = V1.BASE.load_json(raw_path)
        record = {"sample_id": sample_id, "packet_artifact_sha256": file_digest(packet_path),
                  "raw_output_sha256": file_digest(raw_path), "events_output_sha256": file_digest(events_path),
                  "execution_receipt_sha256": file_digest(receipt_path)}
        try:
            reconstructed = normalize_render(raw, packet, receipt, receipt_path)
        except V2Error as error:
            require(not render_path.exists(), f"{sample_id} retained a normalized render after transport failure")
            invalid.append({"sample_id": sample_id, "reason": str(error)})
        else:
            require(render_path.is_file(), f"{sample_id} valid transport lacks normalized render")
            render = validate_render(V1.BASE.load_json(render_path), packet, receipt_path)
            require(render == reconstructed, f"{sample_id} normalized render does not reconstruct")
            record["render_artifact_sha256"] = file_digest(render_path)
            valid.append(sample_id)
        bindings.append(record)
    require(invalid, "transport-stop report requires at least one invalid launched sample")
    require(not (run_dir / "semantic").exists(), "semantic artifacts exist after transport failure")
    require(not (run_dir / "matching").exists(), "matching artifacts exist after transport failure")
    return {
        "artifact_type": "linguistic_register_semantic_licensed_transport_stop_report_v2",
        "schema_version": SCHEMA_VERSION,
        "experiment_id": plan["experiment_id"],
        "plan_artifact_sha256": file_digest(plan_path),
        "preoutcome_checkpoint_commit_oid": checkpoint,
        "planned_samples": SAMPLE_COUNT,
        "launched_samples": launched,
        "transport_valid_samples": valid,
        "transport_invalid_samples": invalid,
        "samples_without_execution_evidence": without_execution_evidence,
        "execution_bindings": bindings,
        "semantic_status": "not_run_transport_gate_failed",
        "matching_status": "not_run_transport_gate_failed",
        "gate_passed": False,
        "disposition": "micro_render_v2_transport_contract_failed",
        "reporting_limitations": [
            "Absence of retained execution evidence does not prove that no unretained or aborted invocation occurred.",
            "Execution receipts were first immutably checkpointed with reporting artifacts, so Git proves final binding consistency but not pre-report receipt immutability.",
        ],
        "authority": "stopped transport evidence only; no semantic, recognizability, ranking, or selection authority",
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest="command", required=True)
    vp = sub.add_parser("validate-plan"); vp.add_argument("plan", type=Path)
    pr = sub.add_parser("prepare-render-jobs"); pr.add_argument("--plan", required=True, type=Path); pr.add_argument("--preoutcome-checkpoint", required=True); pr.add_argument("--run-dir", required=True, type=Path)
    nr = sub.add_parser("normalize-render"); nr.add_argument("--plan", required=True, type=Path); nr.add_argument("--packet", required=True, type=Path); nr.add_argument("--raw", required=True, type=Path); nr.add_argument("--events", required=True, type=Path); nr.add_argument("--receipt", required=True, type=Path); nr.add_argument("--output", required=True, type=Path)
    ps = sub.add_parser("prepare-semantic"); ps.add_argument("--plan", required=True, type=Path); ps.add_argument("--run-dir", required=True, type=Path); ps.add_argument("--output", required=True, type=Path)
    ns = sub.add_parser("normalize-semantic"); ns.add_argument("--plan", required=True, type=Path); ns.add_argument("--packet", required=True, type=Path); ns.add_argument("--raw", required=True, type=Path); ns.add_argument("--events", required=True, type=Path); ns.add_argument("--receipt", required=True, type=Path); ns.add_argument("--output", required=True, type=Path)
    pm = sub.add_parser("prepare-matching"); pm.add_argument("--plan", required=True, type=Path); pm.add_argument("--run-dir", required=True, type=Path); pm.add_argument("--output-dir", required=True, type=Path)
    nm = sub.add_parser("normalize-matching"); nm.add_argument("--plan", required=True, type=Path); nm.add_argument("--packet", required=True, type=Path); nm.add_argument("--raw", required=True, type=Path); nm.add_argument("--events", required=True, type=Path); nm.add_argument("--receipt", required=True, type=Path); nm.add_argument("--output", required=True, type=Path)
    ag = sub.add_parser("aggregate"); ag.add_argument("--plan", required=True, type=Path); ag.add_argument("--run-dir", required=True, type=Path); ag.add_argument("--output", required=True, type=Path)
    ats = sub.add_parser("aggregate-transport-stop"); ats.add_argument("--plan", required=True, type=Path); ats.add_argument("--run-dir", required=True, type=Path); ats.add_argument("--output", required=True, type=Path)
    args = parser.parse_args()
    plan_path = args.plan
    plan, repository = validate_plan(V1.BASE.load_yaml(plan_path), plan_path)
    if args.command == "validate-plan":
        print(json.dumps({"status": "valid", "artifact_type": plan["artifact_type"]}, sort_keys=True)); return 0
    if args.command == "prepare-render-jobs":
        require(not args.run_dir.exists(), "run directory already exists")
        checkpoint = verify_preoutcome_checkpoint(repository, plan_path, args.preoutcome_checkpoint)
        packets, key = prepare_render_jobs(plan, plan_path, checkpoint)
        emit(key, args.run_dir / "render-key.json")
        for sample_id, packet in packets.items():
            emit(packet, args.run_dir / "samples" / sample_id / "render-packet.json")
    elif args.command == "normalize-render":
        packet = V1.BASE.load_json(args.packet)
        raw = V1.BASE.load_json(args.raw)
        receipt = validate_receipt(V1.BASE.load_json(args.receipt), stage="render", packet_path=args.packet,
                                   raw_path=args.raw, events_path=args.events, plan=plan)
        emit(normalize_render(raw, packet, receipt, args.receipt), args.output)
    elif args.command == "prepare-semantic":
        emit(prepare_semantic_packet(plan, plan_path, args.run_dir), args.output)
    elif args.command == "normalize-semantic":
        packet = V1.BASE.load_json(args.packet)
        validate_receipt(V1.BASE.load_json(args.receipt), stage="semantic-adjudication", packet_path=args.packet,
                         raw_path=args.raw, events_path=args.events, plan=plan)
        emit(normalize_semantic(V1.BASE.load_json(args.raw), packet, args.receipt), args.output)
    elif args.command == "prepare-matching":
        require(not args.output_dir.exists(), "matching output directory already exists")
        for pass_number in range(1, MATCH_PASSES + 1):
            packet, match_key = prepare_matching_pass(plan, plan_path, args.run_dir, pass_number)
            emit(packet, args.output_dir / f"pass-{pass_number:02d}" / "packet.json")
            emit(match_key, args.output_dir / f"pass-{pass_number:02d}" / "key.json")
    elif args.command == "normalize-matching":
        packet = V1.BASE.load_json(args.packet)
        validate_receipt(V1.BASE.load_json(args.receipt), stage="matching", packet_path=args.packet,
                         raw_path=args.raw, events_path=args.events, plan=plan)
        emit(normalize_matching(V1.BASE.load_json(args.raw), packet, args.receipt), args.output)
    elif args.command == "aggregate":
        emit(aggregate(plan, plan_path, args.run_dir), args.output)
    else:
        emit(aggregate_transport_stop(plan, plan_path, args.run_dir), args.output)
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (V2Error, V1.MicroRenderError, V1.BASE.RecognizabilityError, OSError, subprocess.SubprocessError) as error:
        print(f"micro_render_v2_artifacts: {error}", file=sys.stderr)
        raise SystemExit(2)
