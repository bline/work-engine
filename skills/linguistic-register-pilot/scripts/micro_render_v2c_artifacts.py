#!/usr/bin/env python3
"""Prepare, gate, and score the corrected micro-render v2c experiment."""

from __future__ import annotations

import argparse
import copy
import datetime as dt
import hashlib
import importlib.util
import json
import statistics
import subprocess
import sys
from pathlib import Path
from typing import Any


V2B_PATH = Path(__file__).with_name("micro_render_v2b_artifacts.py")
SPEC = importlib.util.spec_from_file_location("micro_render_v2b_for_v2c", V2B_PATH)
if SPEC is None or SPEC.loader is None:
    raise RuntimeError("cannot load micro-render v2b utilities")
V2B = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(V2B)
V2 = V2B.V2

SCHEMA_VERSION = 4
SAMPLE_COUNT = 16
MATCH_PASSES = 3


class V2CError(ValueError):
    pass


def require(condition: bool, message: str) -> None:
    if not condition:
        raise V2CError(message)


def canonical(value: Any) -> bytes:
    return (json.dumps(value, indent=2, sort_keys=True) + "\n").encode()


def digest(value: Any) -> str:
    return hashlib.sha256(canonical(value)).hexdigest()


def file_digest(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def emit(value: Any, path: Path) -> None:
    require(not path.exists(), f"refusing to overwrite output: {path}")
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_bytes(canonical(value))


def load_json(path: Path) -> Any:
    return json.loads(path.read_text(encoding="utf-8"))


def binding_path(repository: Path, value: Any, label: str) -> Path:
    return V2B.binding_path(repository, value, label)


def validate_plan(value: Any, plan_path: Path) -> tuple[dict[str, Any], Path, dict[str, Any]]:
    plan = V2.obj(value, "plan")
    V2.exact(plan, {"artifact_type", "schema_version", "status", "experiment_id", "repository",
                    "base_plan", "freshness", "sample_identity", "transport", "launch_commitment",
                    "word_policy", "thresholds", "downstream_authority", "execution",
                    "preregistration_owner", "limitations"}, "plan")
    require(plan["artifact_type"] == "linguistic_register_semantic_licensed_micro_render_plan_v2c"
            and plan["schema_version"] == SCHEMA_VERSION and plan["status"] == "frozen",
            "plan identity incompatible")
    V2.text(plan["experiment_id"], "plan.experiment_id")
    repository = Path(V2.text(plan["repository"], "plan.repository"))
    require(repository.is_absolute() and repository.is_dir(), "plan.repository invalid")
    base_path = binding_path(repository, plan["base_plan"], "plan.base_plan")
    freshness = V2.obj(plan["freshness"], "plan.freshness")
    V2.exact(freshness, {"rendering_seed", "prior_output_policy"}, "plan.freshness")
    for key in freshness:
        V2.text(freshness[key], f"plan.freshness.{key}")
    identity = V2.obj(plan["sample_identity"], "plan.sample_identity")
    V2.exact(identity, {"opaque_id_seed", "assignment_seed", "rule"}, "plan.sample_identity")
    for key in identity:
        V2.text(identity[key], f"plan.sample_identity.{key}")
    transport = V2.obj(plan["transport"], "plan.transport")
    V2.exact(transport, {"normalization", "blinded_ingestion", "digest_rule"}, "plan.transport")
    for key in transport:
        V2.text(transport[key], f"plan.transport.{key}")
    commitment = V2.obj(plan["launch_commitment"], "plan.launch_commitment")
    V2.exact(commitment, {"definition", "binding_rule", "checkpoint_rule"}, "plan.launch_commitment")
    for key in commitment:
        V2.text(commitment[key], f"plan.launch_commitment.{key}")
    word_policy = V2.obj(plan["word_policy"], "plan.word_policy")
    V2.exact(word_policy, {"generation_target_min", "generation_target_max",
                           "acceptance_min", "acceptance_max"}, "plan.word_policy")
    require(word_policy == {"generation_target_min": 110, "generation_target_max": 120,
                            "acceptance_min": 90, "acceptance_max": 130}, "word policy incompatible")
    thresholds = V2.obj(plan["thresholds"], "plan.thresholds")
    V2.exact(thresholds, {"expected_render_jobs", "require_transport_complete"}, "plan.thresholds")
    require(thresholds == {"expected_render_jobs": 16, "require_transport_complete": True},
            "transport threshold incompatible")
    downstream = V2.obj(plan["downstream_authority"], "plan.downstream_authority")
    V2.exact(downstream, {"semantic_adjudication", "matching"}, "plan.downstream_authority")
    execution = V2.obj(plan["execution"], "plan.execution")
    V2.exact(execution, {"runner", "harness"}, "plan.execution")
    runner = binding_path(repository, execution["runner"], "plan.execution.runner")
    harness = binding_path(repository, execution["harness"], "plan.execution.harness")
    require(runner.name == "sol_packet_job_v2c.py" and harness.resolve() == Path(__file__).resolve(),
            "v2c execution binding invalid")
    V2.text(plan["preregistration_owner"], "plan.preregistration_owner")
    for item in V2.array(plan["limitations"], "plan.limitations"):
        V2.text(item, "plan.limitations[]")
    require(plan_path.resolve().is_relative_to(repository.resolve()), "plan must be inside repository")

    effective = copy.deepcopy(V2.V1.BASE.load_yaml(base_path))
    effective["experiment_id"] = plan["experiment_id"]
    effective["rendering"]["seed"] = freshness["rendering_seed"]
    effective["execution"]["runner"] = execution["runner"]
    effective["preregistration_owner"] = plan["preregistration_owner"]
    effective["limitations"] = list(effective["limitations"]) + list(plan["limitations"])
    V2.validate_plan(effective, plan_path)
    return plan, repository, effective


def verify_checkpoint_files(repository: Path, checkpoint: str, paths: list[Path]) -> None:
    require(isinstance(checkpoint, str) and len(checkpoint) == 40, "checkpoint must be a full commit OID")
    resolved = subprocess.run(["git", "-C", str(repository), "rev-parse", f"{checkpoint}^{{commit}}"],
                              stdout=subprocess.PIPE, stderr=subprocess.PIPE, check=False)
    require(resolved.returncode == 0 and resolved.stdout.decode().strip() == checkpoint,
            "checkpoint is not an available commit")
    for path in paths:
        relative = path.resolve().relative_to(repository.resolve()).as_posix()
        retained = subprocess.run(["git", "-C", str(repository), "show", f"{checkpoint}:{relative}"],
                                  stdout=subprocess.PIPE, stderr=subprocess.PIPE, check=False)
        require(retained.returncode == 0, f"checkpoint does not contain {relative}")
        require(hashlib.sha256(retained.stdout).hexdigest() == file_digest(path),
                f"checkpoint differs from {relative}")


def checkpoint_time(repository: Path, checkpoint: str) -> dt.datetime:
    completed = subprocess.run(
        ["git", "-C", str(repository), "show", "-s", "--format=%cI", checkpoint],
        stdout=subprocess.PIPE, stderr=subprocess.PIPE, check=False,
    )
    require(completed.returncode == 0, "checkpoint timestamp is unavailable")
    value = completed.stdout.decode().strip()
    try:
        parsed = dt.datetime.fromisoformat(value)
    except ValueError as error:
        raise V2CError("checkpoint timestamp is invalid") from error
    require(parsed.tzinfo is not None, "checkpoint timestamp must be timezone-aware")
    return parsed


def opaque_ids(plan: dict[str, Any]) -> list[str]:
    seed = plan["sample_identity"]["opaque_id_seed"]
    values = ["X" + hashlib.sha256(f"{seed}\0slot-{index:02d}".encode()).hexdigest()[:12].upper()
              for index in range(1, SAMPLE_COUNT + 1)]
    require(len(set(values)) == SAMPLE_COUNT, "opaque sample IDs collided")
    return values


def assigned_cells(plan: dict[str, Any], effective: dict[str, Any]) -> dict[str, str]:
    styles = V2.condition_styles(effective)
    cells = [f"{brief['brief_id']}::{condition}::replica-{replica}"
             for brief in effective["briefs"] for condition in sorted(styles)
             for replica in range(1, 3)]
    ordered = V2.hashed_order(plan["sample_identity"]["assignment_seed"], cells)
    return dict(zip(opaque_ids(plan), ordered, strict=True))


def config_for(effective: dict[str, Any], stage: str) -> dict[str, Any]:
    return effective["execution"]["semantic_adjudication" if stage == "semantic-adjudication" else stage]


def make_launch_manifest(plan: dict[str, Any], effective: dict[str, Any], repository: Path,
                         stage: str, jobs: list[tuple[str, Path]]) -> dict[str, Any]:
    config = config_for(effective, stage)
    schema = binding_path(repository, config["schema"], f"effective.execution.{stage}.schema")
    descriptors = []
    for job_id, packet in sorted(jobs):
        descriptors.append({
            "experiment_id": plan["experiment_id"], "job_id": job_id, "stage": stage,
            "packet_artifact_sha256": file_digest(packet), "output_schema_sha256": file_digest(schema),
            "model": config["model"], "reasoning_effort": config["reasoning_effort"],
            "prompt_sha256": hashlib.sha256(config["prompt"].encode()).hexdigest(),
        })
    launch_set = {"experiment_id": plan["experiment_id"], "stage": stage, "jobs": descriptors}
    launch_digest = digest(launch_set)
    requests = []
    for descriptor in descriptors:
        request = {**descriptor, "launch_set_sha256": launch_digest}
        requests.append({**request, "request_digest": digest(request)})
    return {
        "artifact_type": "linguistic_register_v2c_launch_manifest",
        "schema_version": 1,
        "experiment_id": plan["experiment_id"],
        "stage": stage,
        "launch_set": launch_set,
        "launch_set_sha256": launch_digest,
        "requests": requests,
        "authority": "pre-launch content commitment; no outcome or unblinding authority",
    }


def request_from_manifest(manifest: dict[str, Any], job_id: str) -> dict[str, Any]:
    matches = [item for item in manifest["requests"] if item["job_id"] == job_id]
    require(len(matches) == 1, f"manifest request missing or duplicated for {job_id}")
    return matches[0]


def validate_manifest(manifest: Any, plan: dict[str, Any], effective: dict[str, Any], repository: Path,
                      stage: str, jobs: list[tuple[str, Path]]) -> dict[str, Any]:
    expected = make_launch_manifest(plan, effective, repository, stage, jobs)
    require(manifest == expected, f"{stage} launch manifest does not reconstruct")
    return expected


def prepare_render(plan: dict[str, Any], plan_path: Path, effective: dict[str, Any], repository: Path,
                   prereg_checkpoint: str, run_dir: Path) -> None:
    require(not run_dir.exists(), "run directory already exists")
    run_dir = run_dir.resolve()
    verify_checkpoint_files(repository, prereg_checkpoint, [plan_path])
    styles = V2.condition_styles(effective)
    briefs = {item["brief_id"]: item for item in effective["briefs"]}
    assignments = assigned_cells(plan, effective)
    records = {}
    jobs = []
    for sample_id, identity in sorted(assignments.items()):
        brief_id, condition_id, replica_label = identity.split("::")
        brief = copy.deepcopy(briefs[brief_id])
        brief["output_constraint"] = (
            "One self-contained paragraph targeting 110 to 120 words. Acceptance remains 90 to 130 words; "
            "use no heading, bullet list, or serialized wrapper."
        )
        packet = {
            "artifact_type": "linguistic_register_semantic_licensed_render_job_v2c",
            "schema_version": SCHEMA_VERSION,
            "experiment_id": plan["experiment_id"],
            "sample_id": sample_id,
            "plan_artifact_sha256": file_digest(plan_path),
            "preregistration_checkpoint_commit_oid": prereg_checkpoint,
            "semantic_brief": brief,
            "anonymous_licensed_style": V2.anonymous_style(styles[condition_id]),
            "semantic_license": effective["semantic_license"],
            "rendering_contract": {"task": effective["rendering"]["task"],
                                   "blindness": effective["rendering"]["blindness"],
                                   "generation_target_words": [110, 120],
                                   "acceptance_words": [90, 130]},
            "authority": "final-byte blinded v2c render packet; no unblinding or downstream authority",
        }
        packet_path = run_dir / "samples" / sample_id / "render-packet.json"
        emit(packet, packet_path)
        jobs.append((sample_id, packet_path))
        records[sample_id] = {
            "brief_id": brief_id, "condition_id": condition_id,
            "replica": int(replica_label.removeprefix("replica-")),
            "packet_artifact_sha256": file_digest(packet_path),
        }
    unblinding = {
        "artifact_type": "linguistic_register_v2c_sealed_unblinding",
        "schema_version": 1,
        "experiment_id": plan["experiment_id"],
        "samples": records,
        "neutral_condition_id": effective["neutral_style"]["condition_id"],
        "authority": "sealed condition mapping for post-checkpoint aggregation and matching only",
    }
    emit(unblinding, run_dir / "sealed-unblinding.json")
    emit(make_launch_manifest(plan, effective, repository, "render", jobs), run_dir / "render-launch-manifest.json")


def render_jobs(run_dir: Path) -> list[tuple[str, Path]]:
    manifest = load_json(run_dir / "render-launch-manifest.json")
    return [(item["job_id"], run_dir / "samples" / item["job_id"] / "render-packet.json")
            for item in manifest["requests"]]


def verify_render_launch(plan: dict[str, Any], effective: dict[str, Any], repository: Path,
                         launch_checkpoint: str, run_dir: Path) -> dict[str, Any]:
    manifest_path = run_dir / "render-launch-manifest.json"
    unblinding_path = run_dir / "sealed-unblinding.json"
    jobs = render_jobs(run_dir)
    manifest = validate_manifest(load_json(manifest_path), plan, effective, repository, "render", jobs)
    unblinding = load_json(unblinding_path)
    require(set(unblinding["samples"]) == {job_id for job_id, _ in jobs}, "unblinding sample set differs")
    for job_id, packet in jobs:
        require(unblinding["samples"][job_id]["packet_artifact_sha256"] == file_digest(packet),
                f"unblinding packet digest mismatch for {job_id}")
    verify_checkpoint_files(repository, launch_checkpoint,
                            [manifest_path, unblinding_path, *[path for _, path in jobs]])
    return manifest


def validate_marker(value: Any, request: dict[str, Any], not_before: dt.datetime) -> dt.datetime:
    marker = V2.obj(value, "attempt_marker")
    for key, expected in request.items():
        require(marker.get(key) == expected, f"attempt marker {key} mismatch")
    require(marker.get("artifact_type") == "linguistic_register_v2c_attempt_marker"
            and marker.get("state") == "model_launch_imminent", "attempt marker identity invalid")
    marked = V2.parse_time(marker.get("marked_at_utc"), "attempt_marker.marked_at_utc")
    require(marked >= not_before, "attempt marker predates immutable launch checkpoint")
    return marked


def validate_receipt(value: Any, request: dict[str, Any], raw: Path, events: Path,
                     not_before: dt.datetime) -> dict[str, Any]:
    receipt = V2.obj(value, "execution_receipt")
    for key, expected in request.items():
        require(receipt.get(key) == expected, f"execution receipt {key} mismatch")
    require(receipt.get("artifact_type") == "linguistic_register_model_execution_receipt_v2c"
            and receipt.get("provider") == "OpenAI", "execution receipt identity invalid")
    require(receipt.get("staged_input_files") == ["packet.json", "output.schema.json"]
            and receipt.get("filesystem_visibility_claimed") is False, "receipt visibility scope invalid")
    require(receipt.get("raw_output_sha256") == file_digest(raw), "raw output digest mismatch")
    require(receipt.get("events_output_sha256") == file_digest(events), "events digest mismatch")
    started = V2.parse_time(receipt.get("started_at_utc"), "execution_receipt.started_at_utc")
    completed = V2.parse_time(receipt.get("completed_at_utc"), "execution_receipt.completed_at_utc")
    require(started >= not_before and completed >= started, "execution receipt chronology invalid")
    return receipt


def ingest_render(plan: dict[str, Any], effective: dict[str, Any], repository: Path,
                  launch_checkpoint: str, run_dir: Path) -> dict[str, Any]:
    manifest = verify_render_launch(plan, effective, repository, launch_checkpoint, run_dir)
    launch_time = checkpoint_time(repository, launch_checkpoint)
    outcomes = []
    for request in manifest["requests"]:
        sample_id = request["job_id"]
        directory = run_dir / "samples" / sample_id
        marker = directory / "attempt-marker.json"
        raw = directory / "raw-render.json"
        events = directory / "render-events.jsonl"
        receipt = directory / "render-execution-receipt.json"
        if not marker.exists():
            outcomes.append({"sample_id": sample_id, "state": "planned_not_attempted",
                             "normalization": "rejected", "reason": "attempt_marker_missing",
                             "raw_response_sha256": None, "extracted_prose_sha256": None})
            continue
        marked = validate_marker(load_json(marker), request, launch_time)
        if not all(path.exists() for path in (raw, events, receipt)):
            outcomes.append({"sample_id": sample_id, "state": "attempted_no_receipt",
                             "normalization": "rejected", "reason": "execution_evidence_incomplete",
                             "raw_response_sha256": None, "extracted_prose_sha256": None})
            continue
        receipt_value = validate_receipt(load_json(receipt), request, raw, events, marked)
        raw_sha = file_digest(raw)
        if receipt_value.get("completion_status") != "completed" or receipt_value.get("return_code") != 0:
            outcomes.append({"sample_id": sample_id, "state": "completed_failed",
                             "normalization": "rejected", "reason": "model_execution_failed",
                             "raw_response_sha256": raw_sha, "extracted_prose_sha256": None})
            continue
        try:
            prose, mode = V2B.normalize_transport(load_json(raw))
        except (V2B.V2BError, V2.V2Error, json.JSONDecodeError) as error:
            outcomes.append({"sample_id": sample_id, "state": "completed_receipt",
                             "normalization": "rejected", "reason": str(error),
                             "raw_response_sha256": raw_sha, "extracted_prose_sha256": None})
            continue
        render = {
            "artifact_type": "linguistic_register_semantic_licensed_render_v2c",
            "schema_version": SCHEMA_VERSION,
            "experiment_id": plan["experiment_id"], "sample_id": sample_id,
            "packet_artifact_sha256": request["packet_artifact_sha256"],
            "execution_receipt_sha256": file_digest(receipt),
            "raw_response_sha256": raw_sha,
            "extracted_prose_sha256": hashlib.sha256(prose.encode()).hexdigest(),
            "normalization": mode, "text": prose,
            "authority": "blinded transport-normalized prose; no semantic or recognizability judgment",
        }
        emit(render, directory / "render.json")
        outcomes.append({"sample_id": sample_id, "state": "completed_receipt", "normalization": mode,
                         "reason": None, "raw_response_sha256": raw_sha,
                         "extracted_prose_sha256": render["extracted_prose_sha256"]})
    accepted = sum(item["normalization"] in {"direct", "unwrapped_once"} for item in outcomes)
    evidence = {
        "artifact_type": "linguistic_register_v2c_immediate_batch_evidence",
        "schema_version": 1, "experiment_id": plan["experiment_id"],
        "launch_set_sha256": manifest["launch_set_sha256"],
        "launch_checkpoint_commit_oid": launch_checkpoint,
        "expected_jobs": SAMPLE_COUNT,
        "state_counts": {state: sum(item["state"] == state for item in outcomes)
                         for state in ("planned_not_attempted", "attempted_no_receipt",
                                       "completed_failed", "completed_receipt")},
        "transport_accepted": accepted, "transport_rejected": SAMPLE_COUNT - accepted,
        "transport_completeness_gate_passed": accepted == SAMPLE_COUNT,
        "outcomes": outcomes,
        "blinding": "Sample-level transport evidence contains no condition labels.",
        "authority": "immediate checkpoint input; no semantic, matching, or condition-comparison authority",
    }
    emit(evidence, run_dir / "immediate-batch-evidence.json")
    return evidence


def observed_render_paths(run_dir: Path) -> list[Path]:
    paths = [run_dir / "render-launch-manifest.json", run_dir / "sealed-unblinding.json",
             run_dir / "immediate-batch-evidence.json"]
    for sample_id, packet in render_jobs(run_dir):
        paths.append(packet)
        directory = run_dir / "samples" / sample_id
        for name in ("attempt-marker.json", "raw-render.json", "render-events.jsonl",
                     "render-execution-receipt.json", "render.json"):
            path = directory / name
            if path.exists():
                paths.append(path)
    return paths


def evaluate_transport(plan: dict[str, Any], effective: dict[str, Any], repository: Path,
                       postbatch_checkpoint: str, run_dir: Path) -> dict[str, Any]:
    verify_checkpoint_files(repository, postbatch_checkpoint, observed_render_paths(run_dir))
    evidence = load_json(run_dir / "immediate-batch-evidence.json")
    manifest = validate_manifest(load_json(run_dir / "render-launch-manifest.json"), plan, effective,
                                 repository, "render", render_jobs(run_dir))
    unblinding = load_json(run_dir / "sealed-unblinding.json")
    by_condition: dict[str, dict[str, int]] = {}
    for outcome in evidence["outcomes"]:
        sample_id = outcome["sample_id"]
        packet = run_dir / "samples" / sample_id / "render-packet.json"
        record = unblinding["samples"][sample_id]
        require(record["packet_artifact_sha256"] == file_digest(packet),
                f"condition aggregation packet binding failed for {sample_id}")
        bucket = by_condition.setdefault(record["condition_id"],
                                         {"direct": 0, "unwrapped_once": 0, "rejected": 0})
        bucket[outcome["normalization"]] += 1
    passed = evidence["transport_completeness_gate_passed"]
    return {
        "artifact_type": "linguistic_register_v2c_transport_gate_report",
        "schema_version": 1, "experiment_id": plan["experiment_id"],
        "postbatch_checkpoint_commit_oid": postbatch_checkpoint,
        "launch_set_sha256": manifest["launch_set_sha256"],
        "immediate_batch_evidence_sha256": file_digest(run_dir / "immediate-batch-evidence.json"),
        "expected_jobs": SAMPLE_COUNT, "accepted_jobs": evidence["transport_accepted"],
        "rejected_jobs": evidence["transport_rejected"], "gate_passed": passed,
        "normalization_by_condition": by_condition,
        "normalization_is_evidence_for": "transport compliance only",
        "semantic_status": "authorized_not_run" if passed else "not_run_transport_gate_failed",
        "matching_status": "not_run_pending_semantic_gate" if passed else "not_run_transport_gate_failed",
        "authority": "transport gate and compliance metadata only; not recognizability or candidate quality",
    }


def require_transport_pass(run_root: Path) -> dict[str, Any]:
    report = load_json(run_root / "transport-gate-report.json")
    require(report.get("gate_passed") is True, "transport gate failed; semantic adjudication prohibited")
    return report


def prepare_semantic(plan: dict[str, Any], plan_path: Path, effective: dict[str, Any], repository: Path,
                     run_dir: Path) -> None:
    require_transport_pass(run_dir.parent)
    require(not run_dir.exists(), "semantic directory already exists")
    items = []
    for sample_id, _ in sorted(render_jobs(run_dir.parent)):
        packet = load_json(run_dir.parent / "samples" / sample_id / "render-packet.json")
        render_path = run_dir.parent / "samples" / sample_id / "render.json"
        render = load_json(render_path)
        items.append({"sample_id": sample_id, "semantic_brief": packet["semantic_brief"],
                      "rendered_text": render["text"], "render_artifact_sha256": file_digest(render_path)})
    packet = {
        "artifact_type": "linguistic_register_semantic_licensed_adjudication_packet_v2c",
        "schema_version": SCHEMA_VERSION, "experiment_id": plan["experiment_id"],
        "plan_artifact_sha256": file_digest(plan_path), "items": items,
        "contract": effective["semantic_adjudication"],
        "authority": "all-sample style-blind semantic gate input; no matching authority",
    }
    packet_path = run_dir / "packet.json"
    emit(packet, packet_path)
    emit(make_launch_manifest(plan, effective, repository, "semantic-adjudication",
                              [("semantic-adjudication", packet_path)]), run_dir / "launch-manifest.json")


def ingest_semantic(plan: dict[str, Any], effective: dict[str, Any], repository: Path,
                    launch_checkpoint: str, run_dir: Path) -> dict[str, Any]:
    packet_path = run_dir / "packet.json"
    jobs = [("semantic-adjudication", packet_path)]
    manifest_path = run_dir / "launch-manifest.json"
    manifest = validate_manifest(load_json(manifest_path), plan, effective, repository,
                                 "semantic-adjudication", jobs)
    verify_checkpoint_files(repository, launch_checkpoint, [packet_path, manifest_path])
    launch_time = checkpoint_time(repository, launch_checkpoint)
    request = request_from_manifest(manifest, "semantic-adjudication")
    marker = run_dir / "attempt-marker.json"; raw = run_dir / "raw-result.json"
    events = run_dir / "events.jsonl"; receipt = run_dir / "execution-receipt.json"
    marked = validate_marker(load_json(marker), request, launch_time)
    validate_receipt(load_json(receipt), request, raw, events, marked)
    result = V2.normalize_semantic(load_json(raw), load_json(packet_path), receipt)
    result["artifact_type"] = "linguistic_register_semantic_licensed_adjudication_result_v2c"
    result["schema_version"] = SCHEMA_VERSION
    result["authority"] = "v2c semantic gate evidence; no matching or selection authority"
    emit(result, run_dir / "result.json")
    accepted = sum(item["verdict"] == "equivalent" for item in result["decisions"])
    report = {
        "artifact_type": "linguistic_register_v2c_semantic_gate_report",
        "schema_version": 1, "experiment_id": plan["experiment_id"],
        "launch_set_sha256": manifest["launch_set_sha256"],
        "accepted_samples": accepted, "required_samples": SAMPLE_COUNT,
        "gate_passed": accepted == SAMPLE_COUNT,
        "matching_status": "authorized_not_run" if accepted == SAMPLE_COUNT else "not_run_semantic_gate_failed",
        "authority": "semantic-equivalence gate only; no recognizability or selection authority",
    }
    emit(report, run_dir / "semantic-gate-report.json")
    return report


def prepare_matching(plan: dict[str, Any], plan_path: Path, effective: dict[str, Any], repository: Path,
                     run_dir: Path) -> None:
    semantic_report = load_json(run_dir.parent / "semantic" / "semantic-gate-report.json")
    require(semantic_report.get("gate_passed") is True, "semantic gate failed; matching prohibited")
    require(not run_dir.exists(), "matching directory already exists")
    unblinding = load_json(run_dir.parent / "sealed-unblinding.json")
    styles = V2.condition_styles(effective)
    jobs = []
    for pass_number in range(1, MATCH_PASSES + 1):
        seed = f"{effective['rendering']['seed']}::matching-pass-{pass_number}"
        conditions = V2.hashed_order(seed, list(styles))
        reference_mapping = {f"R{index:02d}": condition for index, condition in enumerate(conditions, 1)}
        samples = V2.hashed_order(seed, sorted(unblinding["samples"]))
        text_mapping = {f"T{index:02d}": sample for index, sample in enumerate(samples, 1)}
        references = [{"reference_id": reference, "licensed_style_features": V2.anonymous_style(styles[condition])}
                      for reference, condition in reference_mapping.items()]
        texts = [{"text_id": text_id,
                  "text": load_json(run_dir.parent / "samples" / sample / "render.json")["text"]}
                 for text_id, sample in text_mapping.items()]
        packet = {
            "artifact_type": "linguistic_register_semantic_licensed_matching_packet_v2c",
            "schema_version": SCHEMA_VERSION, "experiment_id": plan["experiment_id"],
            "pass_number": pass_number, "plan_artifact_sha256": file_digest(plan_path),
            "anonymous_references": references, "anonymous_texts": texts,
            "contract": effective["matching"], "authority": "blinded v2c matching input",
        }
        directory = run_dir / f"pass-{pass_number:02d}"
        packet_path = directory / "packet.json"
        emit(packet, packet_path)
        emit({"artifact_type": "linguistic_register_v2c_matching_key", "schema_version": 1,
              "experiment_id": plan["experiment_id"], "pass_number": pass_number,
              "packet_artifact_sha256": file_digest(packet_path), "reference_mapping": reference_mapping,
              "text_mapping": text_mapping,
              "condition_by_sample": {sample: record["condition_id"]
                                      for sample, record in unblinding["samples"].items()},
              "authority": "sealed matching scoring key only"}, directory / "key.json")
        jobs.append((f"matching-pass-{pass_number:02d}", packet_path))
    emit(make_launch_manifest(plan, effective, repository, "matching", jobs), run_dir / "launch-manifest.json")


def ingest_matching(plan: dict[str, Any], effective: dict[str, Any], repository: Path,
                    launch_checkpoint: str, run_dir: Path) -> dict[str, Any]:
    jobs = [(f"matching-pass-{number:02d}", run_dir / f"pass-{number:02d}" / "packet.json")
            for number in range(1, MATCH_PASSES + 1)]
    manifest_path = run_dir / "launch-manifest.json"
    manifest = validate_manifest(load_json(manifest_path), plan, effective, repository, "matching", jobs)
    paths = [manifest_path]
    for _, packet in jobs:
        paths.extend([packet, packet.parent / "key.json"])
    verify_checkpoint_files(repository, launch_checkpoint, paths)
    launch_time = checkpoint_time(repository, launch_checkpoint)
    totals: dict[str, int] = {}
    by_pass: dict[str, list[int]] = {}
    confidences: dict[str, list[int]] = {}
    for pass_number, (job_id, packet_path) in enumerate(jobs, 1):
        directory = packet_path.parent
        request = request_from_manifest(manifest, job_id)
        marker = directory / "attempt-marker.json"; raw = directory / "raw-result.json"
        events = directory / "events.jsonl"; receipt = directory / "execution-receipt.json"
        marked = validate_marker(load_json(marker), request, launch_time)
        validate_receipt(load_json(receipt), request, raw, events, marked)
        result = V2.normalize_matching(load_json(raw), load_json(packet_path), receipt)
        result["artifact_type"] = "linguistic_register_semantic_licensed_matching_result_v2c"
        result["schema_version"] = SCHEMA_VERSION
        emit(result, directory / "result.json")
        key = load_json(directory / "key.json")
        require(key["packet_artifact_sha256"] == file_digest(packet_path), "matching key packet mismatch")
        reference_for = {condition: reference for reference, condition in key["reference_mapping"].items()}
        counts: dict[str, int] = {condition: 0 for condition in reference_for}
        for assignment in result["assignments"]:
            sample = key["text_mapping"][assignment["text_id"]]
            condition = key["condition_by_sample"][sample]
            correct = assignment["reference_id"] == reference_for[condition]
            totals[condition] = totals.get(condition, 0) + int(correct)
            counts[condition] += int(correct)
            if correct:
                confidences.setdefault(condition, []).append(assignment["confidence"])
        for condition, count in counts.items():
            by_pass.setdefault(condition, []).append(count)
    condition_results = []
    thresholds = effective["thresholds"]
    for condition in sorted(totals):
        passing = sum(value >= thresholds["minimum_correct_per_condition_in_passing_pass"]
                      for value in by_pass[condition])
        checks = {"total": totals[condition] >= thresholds["minimum_correct_assignments_per_condition"],
                  "passes": passing >= thresholds["minimum_passing_match_passes_per_condition"]}
        condition_results.append({"condition_id": condition, "correct_assignments": totals[condition],
                                  "possible_assignments": 12, "correct_by_pass": by_pass[condition],
                                  "passing_match_passes": passing,
                                  "median_confidence_when_correct": statistics.median(confidences.get(condition, []))
                                  if confidences.get(condition) else None,
                                  "checks": checks, "passed": all(checks.values())})
    report = {
        "artifact_type": "linguistic_register_v2c_matching_report", "schema_version": 1,
        "experiment_id": plan["experiment_id"], "launch_set_sha256": manifest["launch_set_sha256"],
        "condition_results": condition_results,
        "gate_passed": all(item["passed"] for item in condition_results),
        "authority": "upstream matching evidence only; no corpus selection or production authority",
    }
    emit(report, run_dir / "matching-report.json")
    return report


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest="command", required=True)
    vp = sub.add_parser("validate-plan"); vp.add_argument("plan", type=Path)
    pr = sub.add_parser("prepare-render"); pr.add_argument("--plan", required=True, type=Path); pr.add_argument("--prereg-checkpoint", required=True); pr.add_argument("--run-dir", required=True, type=Path)
    vl = sub.add_parser("verify-render-launch"); vl.add_argument("--plan", required=True, type=Path); vl.add_argument("--launch-checkpoint", required=True); vl.add_argument("--run-dir", required=True, type=Path)
    ir = sub.add_parser("ingest-render"); ir.add_argument("--plan", required=True, type=Path); ir.add_argument("--launch-checkpoint", required=True); ir.add_argument("--run-dir", required=True, type=Path)
    et = sub.add_parser("evaluate-transport"); et.add_argument("--plan", required=True, type=Path); et.add_argument("--postbatch-checkpoint", required=True); et.add_argument("--run-dir", required=True, type=Path); et.add_argument("--output", required=True, type=Path)
    ps = sub.add_parser("prepare-semantic"); ps.add_argument("--plan", required=True, type=Path); ps.add_argument("--run-dir", required=True, type=Path)
    ins = sub.add_parser("ingest-semantic"); ins.add_argument("--plan", required=True, type=Path); ins.add_argument("--launch-checkpoint", required=True); ins.add_argument("--run-dir", required=True, type=Path)
    pm = sub.add_parser("prepare-matching"); pm.add_argument("--plan", required=True, type=Path); pm.add_argument("--run-dir", required=True, type=Path)
    inm = sub.add_parser("ingest-matching"); inm.add_argument("--plan", required=True, type=Path); inm.add_argument("--launch-checkpoint", required=True); inm.add_argument("--run-dir", required=True, type=Path)
    args = parser.parse_args()
    plan, repository, effective = validate_plan(V2.V1.BASE.load_yaml(args.plan), args.plan)
    if args.command == "validate-plan":
        print(json.dumps({"status": "valid", "artifact_type": plan["artifact_type"]}, sort_keys=True))
    elif args.command == "prepare-render":
        prepare_render(plan, args.plan, effective, repository, args.prereg_checkpoint, args.run_dir)
    elif args.command == "verify-render-launch":
        verify_render_launch(plan, effective, repository, args.launch_checkpoint, args.run_dir)
    elif args.command == "ingest-render":
        ingest_render(plan, effective, repository, args.launch_checkpoint, args.run_dir)
    elif args.command == "evaluate-transport":
        emit(evaluate_transport(plan, effective, repository, args.postbatch_checkpoint, args.run_dir), args.output)
    elif args.command == "prepare-semantic":
        prepare_semantic(plan, args.plan, effective, repository, args.run_dir)
    elif args.command == "ingest-semantic":
        ingest_semantic(plan, effective, repository, args.launch_checkpoint, args.run_dir)
    elif args.command == "prepare-matching":
        prepare_matching(plan, args.plan, effective, repository, args.run_dir)
    else:
        ingest_matching(plan, effective, repository, args.launch_checkpoint, args.run_dir)
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (V2CError, V2B.V2BError, V2.V2Error, V2.V1.MicroRenderError,
            V2.V1.BASE.RecognizabilityError, OSError, subprocess.SubprocessError,
            json.JSONDecodeError) as error:
        print(f"micro_render_v2c_artifacts: {error}", file=sys.stderr)
        raise SystemExit(2)
