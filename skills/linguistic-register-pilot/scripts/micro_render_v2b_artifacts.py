#!/usr/bin/env python3
"""Prepare and verify the fresh exact-once micro-render v2b experiment."""

from __future__ import annotations

import argparse
import copy
import datetime as dt
import hashlib
import importlib.util
import json
import subprocess
import sys
from pathlib import Path
from typing import Any


V2_PATH = Path(__file__).with_name("micro_render_v2_artifacts.py")
SPEC = importlib.util.spec_from_file_location("micro_render_v2_for_v2b", V2_PATH)
if SPEC is None or SPEC.loader is None:
    raise RuntimeError("cannot load micro-render v2 utilities")
V2 = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(V2)

SCHEMA_VERSION = 3
SAMPLE_COUNT = 16


class V2BError(ValueError):
    pass


def require(condition: bool, message: str) -> None:
    if not condition:
        raise V2BError(message)


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


def binding_path(repository: Path, value: Any, label: str) -> Path:
    binding = V2.obj(value, label)
    V2.exact(binding, {"path", "sha256"}, label)
    relative = Path(V2.text(binding["path"], f"{label}.path"))
    require(not relative.is_absolute() and ".." not in relative.parts, f"{label}.path escapes repository")
    path = repository / relative
    require(path.is_file(), f"{label}.path is not readable")
    require(file_digest(path) == binding["sha256"], f"{label} digest mismatch")
    return path


def validate_plan(value: Any, plan_path: Path) -> tuple[dict[str, Any], Path, dict[str, Any]]:
    plan = V2.obj(value, "plan")
    V2.exact(plan, {"artifact_type", "schema_version", "status", "experiment_id", "repository",
                    "base_plan", "freshness", "transport", "launch", "thresholds",
                    "downstream_authority", "execution", "preregistration_owner", "limitations"}, "plan")
    require(plan["artifact_type"] == "linguistic_register_semantic_licensed_micro_render_plan_v2b"
            and plan["schema_version"] == SCHEMA_VERSION and plan["status"] == "frozen",
            "plan identity incompatible")
    V2.text(plan["experiment_id"], "plan.experiment_id")
    repository = Path(V2.text(plan["repository"], "plan.repository"))
    require(repository.is_absolute() and repository.is_dir(), "plan.repository invalid")
    base_path = binding_path(repository, plan["base_plan"], "plan.base_plan")
    execution = V2.obj(plan["execution"], "plan.execution")
    V2.exact(execution, {"runner", "harness"}, "plan.execution")
    runner_path = binding_path(repository, execution["runner"], "plan.execution.runner")
    require(runner_path.name == "sol_packet_job_v2b.py", "v2b runner binding invalid")
    harness_path = binding_path(repository, execution["harness"], "plan.execution.harness")
    require(harness_path.resolve() == Path(__file__).resolve(), "v2b harness binding invalid")
    freshness = V2.obj(plan["freshness"], "plan.freshness")
    V2.exact(freshness, {"rendering_seed", "prior_output_policy"}, "plan.freshness")
    require("No v1 or v2 output" in V2.text(freshness["prior_output_policy"], "plan.freshness.prior_output_policy"),
            "prior-output exclusion missing")
    transport = V2.obj(plan["transport"], "plan.transport")
    V2.exact(transport, {"completeness_gate", "blinded_ingestion", "direct_string", "approved_wrapper",
                         "rejection_rule", "digest_rule"}, "plan.transport")
    for key, item in transport.items():
        V2.text(item, f"plan.transport.{key}")
    launch = V2.obj(plan["launch"], "plan.launch")
    V2.exact(launch, {"manifest_rule", "attempt_rule", "receipt_rule", "batch_rule"}, "plan.launch")
    for key, item in launch.items():
        V2.text(item, f"plan.launch.{key}")
    thresholds = V2.obj(plan["thresholds"], "plan.thresholds")
    V2.exact(thresholds, {"expected_render_jobs", "require_transport_complete"}, "plan.thresholds")
    require(thresholds == {"expected_render_jobs": SAMPLE_COUNT, "require_transport_complete": True},
            "v2b transport threshold differs")
    downstream = V2.obj(plan["downstream_authority"], "plan.downstream_authority")
    V2.exact(downstream, {"semantic_adjudication", "matching"}, "plan.downstream_authority")
    for key, item in downstream.items():
        V2.text(item, f"plan.downstream_authority.{key}")
    V2.text(plan["preregistration_owner"], "plan.preregistration_owner")
    for index, limitation in enumerate(V2.array(plan["limitations"], "plan.limitations")):
        V2.text(limitation, f"plan.limitations[{index}]")
    require(plan_path.resolve().is_relative_to(repository.resolve()), "plan must be inside repository")

    base = V2.V1.BASE.load_yaml(base_path)
    effective = copy.deepcopy(base)
    effective["experiment_id"] = plan["experiment_id"]
    effective["rendering"]["seed"] = freshness["rendering_seed"]
    effective["execution"]["runner"] = plan["execution"]["runner"]
    effective["preregistration_owner"] = plan["preregistration_owner"]
    effective["limitations"] = list(base["limitations"]) + list(plan["limitations"])
    V2.validate_plan(effective, plan_path)
    return plan, repository, effective


def verify_checkpoint_files(repository: Path, checkpoint: str, paths: list[Path]) -> None:
    require(isinstance(checkpoint, str) and len(checkpoint) == 40
            and all(char in "0123456789abcdef" for char in checkpoint), "checkpoint must be a full commit OID")
    for path in paths:
        relative = path.resolve().relative_to(repository.resolve()).as_posix()
        retained = subprocess.run(["git", "-C", str(repository), "show", f"{checkpoint}:{relative}"],
                                  check=False, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
        require(retained.returncode == 0, f"checkpoint does not contain {relative}")
        require(hashlib.sha256(retained.stdout).hexdigest() == file_digest(path),
                f"checkpoint differs from {relative}")


def request_record(plan: dict[str, Any], packet: Path, schema: Path, job_id: str, stage: str) -> dict[str, Any]:
    key = "semantic_adjudication" if stage == "semantic-adjudication" else stage
    config = plan["execution"][key]
    request = {
        "experiment_id": plan["experiment_id"],
        "job_id": job_id,
        "stage": stage,
        "packet_artifact_sha256": file_digest(packet),
        "output_schema_sha256": file_digest(schema),
        "model": config["model"],
        "reasoning_effort": config["reasoning_effort"],
        "prompt_sha256": hashlib.sha256(config["prompt"].encode()).hexdigest(),
    }
    return {**request, "request_digest": digest(request)}


def prepare_render(plan: dict[str, Any], plan_path: Path, effective: dict[str, Any], repository: Path,
                   prereg_checkpoint: str, run_dir: Path) -> None:
    require(not run_dir.exists(), "run directory already exists")
    run_dir = run_dir.resolve()
    verify_checkpoint_files(repository, prereg_checkpoint, [plan_path])
    packets, key = V2.prepare_render_jobs(effective, plan_path, prereg_checkpoint)
    key["artifact_type"] = "linguistic_register_semantic_licensed_render_key_v2b"
    key["schema_version"] = SCHEMA_VERSION
    key["authority"] = "sealed until immediate post-batch checkpoint; then unblinding and downstream scoring only"
    emit(key, run_dir / "sealed-render-key.json")
    schema = binding_path(repository, effective["execution"]["render"]["schema"], "effective.execution.render.schema")
    jobs = []
    for sample_id, packet in sorted(packets.items()):
        packet["artifact_type"] = "linguistic_register_semantic_licensed_render_job_v2b"
        packet["schema_version"] = SCHEMA_VERSION
        packet["authority"] = "fresh packet-only v2b render input; no prior output or downstream authority"
        packet_path = run_dir / "samples" / sample_id / "render-packet.json"
        emit(packet, packet_path)
        request = request_record(effective, packet_path, schema, sample_id, "render")
        jobs.append({**request, "sample_id": sample_id, "packet_path": packet_path.relative_to(repository).as_posix(),
                     "planned_state": "not_attempted"})
    manifest = {
        "artifact_type": "linguistic_register_v2b_launch_manifest",
        "schema_version": 1,
        "experiment_id": plan["experiment_id"],
        "plan_artifact_sha256": file_digest(plan_path),
        "preregistration_checkpoint_commit_oid": prereg_checkpoint,
        "expected_job_count": SAMPLE_COUNT,
        "jobs": jobs,
        "authority": "immutable expected-job and request binding; not outcome evidence",
    }
    emit(manifest, run_dir / "launch-manifest.json")


def load_json(path: Path) -> Any:
    return json.loads(path.read_text(encoding="utf-8"))


def validate_marker(marker: Any, request: dict[str, Any]) -> None:
    item = V2.obj(marker, "attempt_marker")
    expected = {key: request[key] for key in ("experiment_id", "job_id", "stage", "packet_artifact_sha256",
                                               "output_schema_sha256", "model", "reasoning_effort",
                                               "prompt_sha256", "request_digest")}
    for key, value in expected.items():
        require(item.get(key) == value, f"attempt marker {key} mismatch")
    require(item.get("artifact_type") == "linguistic_register_v2b_attempt_marker"
            and item.get("state") == "model_launch_imminent", "attempt marker identity invalid")
    V2.parse_time(item.get("marked_at_utc"), "attempt_marker.marked_at_utc")


def validate_receipt(receipt: Any, request: dict[str, Any], raw: Path, events: Path) -> dict[str, Any]:
    item = V2.obj(receipt, "execution_receipt")
    expected = {key: request[key] for key in ("experiment_id", "job_id", "stage", "packet_artifact_sha256",
                                               "output_schema_sha256", "model", "reasoning_effort",
                                               "prompt_sha256", "request_digest")}
    for key, value in expected.items():
        require(item.get(key) == value, f"execution receipt {key} mismatch")
    require(item.get("artifact_type") == "linguistic_register_model_execution_receipt_v2b"
            and item.get("provider") == "OpenAI", "execution receipt identity invalid")
    require(item.get("raw_output_sha256") == file_digest(raw), "raw response digest mismatch")
    require(item.get("events_output_sha256") == file_digest(events), "event digest mismatch")
    require(item.get("completion_status") in {"completed", "failed"}, "completion status invalid")
    V2.parse_time(item.get("started_at_utc"), "execution_receipt.started_at_utc")
    V2.parse_time(item.get("completed_at_utc"), "execution_receipt.completed_at_utc")
    return item


def normalize_transport(raw_value: Any) -> tuple[str, str]:
    outer = V2.obj(raw_value, "raw_render")
    V2.exact(outer, {"text"}, "raw_render")
    raw_text = outer["text"]
    require(isinstance(raw_text, str) and bool(raw_text.strip()), "raw_render.text must be a nonempty string")
    stripped = raw_text.strip()
    require(not stripped.startswith("```"), "code fences are not approved transport")
    mode = "direct"
    prose = raw_text
    if stripped.startswith(("{", "[", '"')):
        try:
            wrapper = json.loads(stripped)
        except json.JSONDecodeError as error:
            raise V2BError("malformed JSON-looking transport") from error
        require(isinstance(wrapper, dict), "arrays and scalar JSON are not approved transport")
        require(set(wrapper) == {"text"}, "approved wrapper must contain only text")
        require(isinstance(wrapper["text"], str) and bool(wrapper["text"].strip()),
                "approved wrapper text must be a nonempty string")
        prose = wrapper["text"]
        nested = prose.strip()
        require(not nested.startswith(("```", "{", "[", '"')), "second nested wrapper is not approved")
        mode = "unwrapped_once"
    prose = V2.validate_prose(prose)
    return prose, mode


def ingest_batch(plan: dict[str, Any], effective: dict[str, Any], repository: Path,
                 launch_checkpoint: str, run_dir: Path) -> dict[str, Any]:
    manifest_path = run_dir / "launch-manifest.json"
    manifest = load_json(manifest_path)
    jobs = V2.array(manifest.get("jobs"), "launch_manifest.jobs")
    require(manifest.get("experiment_id") == plan["experiment_id"] and len(jobs) == SAMPLE_COUNT,
            "launch manifest identity or count invalid")
    retained_paths = [manifest_path, run_dir / "sealed-render-key.json"]
    retained_paths.extend(repository / job["packet_path"] for job in jobs)
    verify_checkpoint_files(repository, launch_checkpoint, retained_paths)
    schema = binding_path(repository, effective["execution"]["render"]["schema"], "effective.execution.render.schema")
    outcomes = []
    for job in jobs:
        sample_id = job["sample_id"]
        directory = run_dir / "samples" / sample_id
        packet = repository / job["packet_path"]
        expected = request_record(effective, packet, schema, sample_id, "render")
        require(all(job.get(key) == expected[key] for key in expected), f"manifest request changed for {sample_id}")
        marker = directory / "attempt-marker.json"
        raw = directory / "raw-render.json"
        events = directory / "render-events.jsonl"
        receipt = directory / "render-execution-receipt.json"
        if not marker.exists():
            outcomes.append({"sample_id": sample_id, "state": "planned_not_attempted", "normalization": "rejected",
                             "reason": "attempt_marker_missing", "raw_response_sha256": None,
                             "extracted_prose_sha256": None})
            continue
        validate_marker(load_json(marker), expected)
        if not (raw.exists() and events.exists() and receipt.exists()):
            outcomes.append({"sample_id": sample_id, "state": "attempted_no_receipt", "normalization": "rejected",
                             "reason": "execution_evidence_incomplete", "raw_response_sha256": None,
                             "extracted_prose_sha256": None})
            continue
        receipt_value = validate_receipt(load_json(receipt), expected, raw, events)
        raw_digest = file_digest(raw)
        if receipt_value["completion_status"] != "completed" or receipt_value["return_code"] != 0:
            outcomes.append({"sample_id": sample_id, "state": "completed_failed", "normalization": "rejected",
                             "reason": "model_execution_failed", "raw_response_sha256": raw_digest,
                             "extracted_prose_sha256": None})
            continue
        try:
            prose, mode = normalize_transport(load_json(raw))
        except (V2BError, V2.V2Error, json.JSONDecodeError) as error:
            outcomes.append({"sample_id": sample_id, "state": "completed_receipt", "normalization": "rejected",
                             "reason": str(error), "raw_response_sha256": raw_digest,
                             "extracted_prose_sha256": None})
            continue
        render = {
            "artifact_type": "linguistic_register_semantic_licensed_render_v2b",
            "schema_version": SCHEMA_VERSION,
            "experiment_id": plan["experiment_id"],
            "sample_id": sample_id,
            "packet_artifact_sha256": file_digest(packet),
            "execution_receipt_sha256": file_digest(receipt),
            "raw_response_sha256": raw_digest,
            "extracted_prose_sha256": hashlib.sha256(prose.encode()).hexdigest(),
            "normalization": mode,
            "text": prose,
            "authority": "blinded transport-normalized prose; no semantic or recognizability judgment",
        }
        emit(render, directory / "render.json")
        outcomes.append({"sample_id": sample_id, "state": "completed_receipt", "normalization": mode,
                         "reason": None, "raw_response_sha256": raw_digest,
                         "extracted_prose_sha256": render["extracted_prose_sha256"]})
    counts = {name: sum(item["state"] == name for item in outcomes)
              for name in ("planned_not_attempted", "attempted_no_receipt", "completed_failed", "completed_receipt")}
    accepted = sum(item["normalization"] in {"direct", "unwrapped_once"} for item in outcomes)
    evidence = {
        "artifact_type": "linguistic_register_v2b_immediate_batch_evidence",
        "schema_version": 1,
        "experiment_id": plan["experiment_id"],
        "launch_manifest_sha256": file_digest(manifest_path),
        "launch_checkpoint_commit_oid": launch_checkpoint,
        "expected_jobs": SAMPLE_COUNT,
        "state_counts": counts,
        "transport_accepted": accepted,
        "transport_rejected": SAMPLE_COUNT - accepted,
        "transport_completeness_gate_passed": accepted == SAMPLE_COUNT,
        "outcomes": outcomes,
        "blinding": "No candidate or condition identity is present; sealed key remains unopened for this artifact.",
        "authority": "immediate causal batch checkpoint input; no semantic, matching, or recognizability authority",
    }
    emit(evidence, run_dir / "immediate-batch-evidence.json")
    return evidence


def evaluate_transport(plan: dict[str, Any], repository: Path, postbatch_checkpoint: str,
                       run_dir: Path) -> dict[str, Any]:
    evidence_path = run_dir / "immediate-batch-evidence.json"
    manifest_path = run_dir / "launch-manifest.json"
    key_path = run_dir / "sealed-render-key.json"
    evidence = load_json(evidence_path)
    manifest = load_json(manifest_path)
    paths = [evidence_path, manifest_path, key_path]
    for outcome in evidence["outcomes"]:
        directory = run_dir / "samples" / outcome["sample_id"]
        paths.append(directory / "attempt-marker.json") if (directory / "attempt-marker.json").exists() else None
        for name in ("raw-render.json", "render-events.jsonl", "render-execution-receipt.json", "render.json"):
            if (directory / name).exists():
                paths.append(directory / name)
    verify_checkpoint_files(repository, postbatch_checkpoint, paths)
    key = load_json(key_path)
    by_condition: dict[str, dict[str, int]] = {}
    for outcome in evidence["outcomes"]:
        condition = key["samples"][outcome["sample_id"]]["condition_id"]
        bucket = by_condition.setdefault(condition, {"direct": 0, "unwrapped_once": 0, "rejected": 0})
        bucket[outcome["normalization"]] += 1
    return {
        "artifact_type": "linguistic_register_v2b_transport_gate_report",
        "schema_version": 1,
        "experiment_id": plan["experiment_id"],
        "postbatch_checkpoint_commit_oid": postbatch_checkpoint,
        "launch_manifest_sha256": file_digest(manifest_path),
        "immediate_batch_evidence_sha256": file_digest(evidence_path),
        "expected_jobs": SAMPLE_COUNT,
        "accepted_jobs": evidence["transport_accepted"],
        "rejected_jobs": evidence["transport_rejected"],
        "gate_passed": evidence["transport_completeness_gate_passed"],
        "normalization_by_condition": by_condition,
        "normalization_is_evidence_for": "transport compliance only",
        "semantic_status": "authorized_not_run" if evidence["transport_completeness_gate_passed"] else "not_run_transport_gate_failed",
        "matching_status": "not_run_pending_semantic_gate" if evidence["transport_completeness_gate_passed"] else "not_run_transport_gate_failed",
        "authority": "transport gate and compliance metadata only; not recognizability or candidate-quality evidence",
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest="command", required=True)
    vp = sub.add_parser("validate-plan"); vp.add_argument("plan", type=Path)
    pr = sub.add_parser("prepare-render"); pr.add_argument("--plan", required=True, type=Path); pr.add_argument("--prereg-checkpoint", required=True); pr.add_argument("--run-dir", required=True, type=Path)
    ib = sub.add_parser("ingest-batch"); ib.add_argument("--plan", required=True, type=Path); ib.add_argument("--launch-checkpoint", required=True); ib.add_argument("--run-dir", required=True, type=Path)
    et = sub.add_parser("evaluate-transport"); et.add_argument("--plan", required=True, type=Path); et.add_argument("--postbatch-checkpoint", required=True); et.add_argument("--run-dir", required=True, type=Path); et.add_argument("--output", required=True, type=Path)
    args = parser.parse_args()
    plan, repository, effective = validate_plan(V2.V1.BASE.load_yaml(args.plan), args.plan)
    if args.command == "validate-plan":
        print(json.dumps({"status": "valid", "artifact_type": plan["artifact_type"]}, sort_keys=True))
    elif args.command == "prepare-render":
        prepare_render(plan, args.plan, effective, repository, args.prereg_checkpoint, args.run_dir)
    elif args.command == "ingest-batch":
        ingest_batch(plan, effective, repository, args.launch_checkpoint, args.run_dir)
    else:
        emit(evaluate_transport(plan, repository, args.postbatch_checkpoint, args.run_dir), args.output)
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (V2BError, V2.V2Error, V2.V1.MicroRenderError, V2.V1.BASE.RecognizabilityError,
            OSError, subprocess.SubprocessError, json.JSONDecodeError) as error:
        print(f"micro_render_v2b_artifacts: {error}", file=sys.stderr)
        raise SystemExit(2)
