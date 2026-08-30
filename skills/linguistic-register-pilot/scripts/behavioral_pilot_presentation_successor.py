#!/usr/bin/env python3
"""Build a fresh pre-outcome contract that binds exact model-visible task turns."""

from __future__ import annotations

import argparse
import importlib.util
import json
import shutil
from pathlib import Path
from typing import Any

import yaml


def load_module(name: str, path: Path) -> Any:
    spec = importlib.util.spec_from_file_location(name, path)
    if spec is None or spec.loader is None:
        raise ValueError(f"cannot load module: {path}")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


HERE = Path(__file__).resolve().parent
CONTRACT = load_module("behavioral_pilot_contract_successor_contract", HERE / "behavioral_pilot_contract.py")
SCHEDULE = load_module("behavioral_pilot_contract_successor_schedule", HERE / "behavioral_pilot_schedule.py")


def write_json(path: Path, value: dict[str, Any]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_bytes(CONTRACT.canonical(value))


def load_json(path: Path) -> dict[str, Any]:
    value = json.loads(path.read_text())
    if not isinstance(value, dict):
        raise ValueError(f"expected object: {path}")
    return value


def replace_experiment_id(path: Path, source_id: str, successor_id: str) -> None:
    value = load_json(path)
    if value.get("experiment_id") != source_id:
        raise ValueError(f"source experiment mismatch: {path}")
    value["experiment_id"] = successor_id
    write_json(path, value)


def validate_predecessor(source: Path, repository: Path) -> Path:
    """Require a valid, outcome-free, terminally failed predecessor contract."""
    CONTRACT.validate_contract(source, repository)
    CONTRACT.assert_no_outcomes(source)
    source_seal = source / "contract-seal.json"
    if not source_seal.is_file():
        raise ValueError("source contract seal is absent")
    source_gate_path = source / "group-3-preoutcome-gate.json"
    if not source_gate_path.is_file():
        raise ValueError("source group-3 preoutcome gate is absent")
    source_gate = load_json(source_gate_path)
    if source_gate.get("contract_seal_sha256") != CONTRACT.sha256_file(source_seal):
        raise ValueError("source group-3 gate seal mismatch")
    if source_gate.get("overall_gate") is not False:
        raise ValueError("source group-3 gate was not terminally failed")
    return source_seal


def build_successor(source: Path, output: Path, repository: Path, successor_id: str, frozen_at: str) -> None:
    if output.exists():
        raise ValueError(f"refusing to overwrite: {output}")
    source_id = CONTRACT.experiment_id(source)
    source_seal = validate_predecessor(source, repository)

    for relative in CONTRACT.BASE_CONTRACT_FILES:
        destination = output / relative
        destination.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(source / relative, destination)

    for relative in (
        "construct-ledger.json",
        "artifact-plan.json",
        "subject-configuration.json",
        "scoring-contract.json",
        "stopping-rules.json",
    ):
        replace_experiment_id(output / relative, source_id, successor_id)

    manifest = load_json(output / "tasks/manifest.json")
    manifest.update({
        "artifact_type": "linguistic_register_task_manifest_v2",
        "schema_version": 2,
        "experiment_id": successor_id,
        "schedule_seed": f"sha256:{successor_id}:balanced-order:v2",
        "presentation_repair": {
            "source_experiment_id": source_id,
            "source_contract_seal_sha256": CONTRACT.sha256_file(source_seal),
            "repair_scope": "Bind exact sequential model-visible user turns without changing task evidence or objective decisions.",
        },
        "provenance_artifacts": ["repair-receipt.json"],
    })

    rows = {row["task_id"]: row for row in manifest["task_rows"]}
    for task_id in ("A1", "A2", "B1", "B2"):
        row = rows[task_id]
        task_path = output / row["task_path"]
        evidence_path = output / row["evidence_path"]
        key_path = output / row["key_path"]
        presentation_relative = f"tasks/presentations/{task_id}.json"
        presentation_path = output / presentation_relative

        task = load_json(task_path)
        task.update({"artifact_type": "linguistic_register_behavioral_task_v2", "schema_version": 2})
        task["presentation"] = {
            "presentation_path": presentation_relative,
            "exact_turn_content_required": True,
            "external_facts_permitted": False,
            "tools_permitted": False,
            "condition_label_visible": False,
            "artifact_id_visible": False,
        }
        write_json(task_path, task)
        task_sha256 = CONTRACT.sha256_file(task_path)
        evidence = load_json(evidence_path)
        evidence_sha256 = CONTRACT.sha256_file(evidence_path)

        presentation = CONTRACT.build_presentation(task, evidence, task_sha256, evidence_sha256)
        write_json(presentation_path, presentation)
        presentation_sha256 = CONTRACT.sha256_file(presentation_path)

        key = load_json(key_path)
        key.update({
            "artifact_type": "linguistic_register_objective_task_key_v2",
            "schema_version": 2,
            "task_sha256": task_sha256,
            "evidence_sha256": evidence_sha256,
            "presentation_sha256": presentation_sha256,
        })
        write_json(key_path, key)

        row.update({
            "task_sha256": task_sha256,
            "evidence_sha256": evidence_sha256,
            "presentation_path": presentation_relative,
            "presentation_sha256": presentation_sha256,
            "key_sha256": CONTRACT.sha256_file(key_path),
        })

    write_json(output / "tasks/manifest.json", manifest)
    schedule = SCHEDULE.build_schedule(output / "tasks/manifest.json")
    write_json(output / "execution-schedule.json", schedule)

    preregistration = yaml.safe_load((output / "preregistration.yaml").read_text())
    preregistration.update({
        "artifact_type": "linguistic_register_behavioral_pilot_preregistration_v2",
        "schema_version": 2,
        "experiment_id": successor_id,
        "frozen_at": frozen_at,
        "renderings_generated": False,
        "outcomes_generated": False,
        "successor_contract": {
            "source_experiment_id": source_id,
            "source_contract_seal_sha256": CONTRACT.sha256_file(source_seal),
            "reason": "The legacy seal did not bind second_turn_record.text into the exact second user message.",
            "retained_scope": "Construct ledger, substantive evidence fixtures, objective decisions, balance, scoring, and stopping rules.",
            "superseded_scope": "Legacy task presentation, task/key/manifest/schedule digests, and every group-3 binding to the legacy seal.",
        },
    })
    preregistration["scope"]["authorized_stage"] = "Presentation-contract repair and fresh preoutcome successor construction only."
    preregistration["task_separation"]["exact_model_visible_turns_bound"] = True
    preregistration["task_separation"]["second_turn_stimulus_delivery_required"] = True
    preregistration["preoutcome_gates"].insert(1, "exact sequential model-visible task turns bound through keys and schedule")
    bindings = preregistration["contract_bindings"]
    binding_files = {
        "construct_ledger_sha256": "construct-ledger.json",
        "artifact_plan_sha256": "artifact-plan.json",
        "subject_configuration_sha256": "subject-configuration.json",
        "task_manifest_sha256": "tasks/manifest.json",
        "execution_schedule_sha256": "execution-schedule.json",
        "scoring_contract_sha256": "scoring-contract.json",
        "stopping_rules_sha256": "stopping-rules.json",
    }
    for field, relative in binding_files.items():
        bindings[field] = CONTRACT.sha256_file(output / relative)
    (output / "preregistration.yaml").write_text(yaml.safe_dump(preregistration, sort_keys=False))

    source_manifest = load_json(source / "tasks/manifest.json")
    source_rows = {row["task_id"]: row for row in source_manifest["task_rows"]}
    successor_rows = {row["task_id"]: row for row in manifest["task_rows"]}
    repair_receipt = {
        "artifact_type": "linguistic_register_behavioral_contract_repair_receipt_v1",
        "schema_version": 1,
        "repair_id": f"{successor_id}:presentation-contract-repair",
        "discovery_status": "preoutcome_protocol_defect_discovered_before_T001",
        "authority": {
            "source": "human-authorized repair and continuation in the active 2026-08-29 session",
            "scope": "Prospectively correct and reseal the behavioral presentation contract; no behavioral execution authority is created.",
        },
        "predecessor": {
            "experiment_id": source_id,
            "contract_seal_sha256": CONTRACT.sha256_file(source_seal),
            "task_manifest_sha256": CONTRACT.sha256_file(source / "tasks/manifest.json"),
            "execution_schedule_sha256": CONTRACT.sha256_file(source / "execution-schedule.json"),
            "group_3_gate_sha256": CONTRACT.sha256_file(source / "group-3-preoutcome-gate.json"),
            "group_3_overall_gate": False,
            "launch_status": "terminally_refused_before_T001",
        },
        "identified_defect": {
            "finding_ids": ["LREG-PRES-001", "LREG-PRES-002", "LREG-PRES-003"],
            "observation": "second_turn_record.text was stored in each evidence packet but was not bound into turns[1].instruction or any exact model-visible message",
            "additional_risk": "presentation.evidence_order was not turn-scoped and could not establish observation availability or prevent premature turn-2 exposure",
            "consequence": "A1/A2 could score successfully without receiving their polarity-matched challenge; B1/B2 delivery was likewise underspecified.",
        },
        "repair_rationale": "The scripted manipulation and availability boundary are experimental structure. Exact sequential user messages and their digests must be owned by the sealed contract.",
        "retained_without_substantive_reauthoring": {
            "upstream_bindings": preregistration["upstream_bindings"],
            "blind_task_authoring_result_sha256": CONTRACT.sha256_file(output / "blind-task-authoring-result.json"),
            "evidence_sha256_by_task": {task_id: source_rows[task_id]["evidence_sha256"] for task_id in sorted(source_rows)},
            "objective_decisions_by_task": {
                task_id: load_json(source / source_rows[task_id]["key_path"])["expected"]
                for task_id in sorted(source_rows)
            },
            "scoring_semantics": "unchanged",
            "stopping_rules": "unchanged apart from successor experiment identity",
        },
        "successor": {
            "experiment_id": successor_id,
            "preregistration_sha256": CONTRACT.sha256_file(output / "preregistration.yaml"),
            "task_manifest_sha256": CONTRACT.sha256_file(output / "tasks/manifest.json"),
            "execution_schedule_sha256": CONTRACT.sha256_file(output / "execution-schedule.json"),
            "bindings_by_task": {
                task_id: {
                    "task_sha256": successor_rows[task_id]["task_sha256"],
                    "evidence_sha256": successor_rows[task_id]["evidence_sha256"],
                    "presentation_sha256": successor_rows[task_id]["presentation_sha256"],
                    "key_sha256": successor_rows[task_id]["key_sha256"],
                }
                for task_id in sorted(successor_rows)
            },
            "rendering_binding_status": "not_created; a fresh v2 Group 3 preregistration, artifacts, reviews, manifest, and passing gate are required",
            "behavioral_outcome_status": "none",
        },
        "boundary": "This receipt is bound by the successor contract seal. It records a prospective protocol repair and does not authorize rendering or T001.",
    }
    write_json(output / "repair-receipt.json", repair_receipt)

    CONTRACT.seal_contract(output, repository, output / "contract-seal.json")


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--repository", type=Path, required=True)
    parser.add_argument("--source", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--experiment-id", required=True)
    parser.add_argument("--frozen-at", required=True)
    args = parser.parse_args()
    build_successor(args.source, args.output, args.repository, args.experiment_id, args.frozen_at)
    print(CONTRACT.sha256_file(args.output / "contract-seal.json"))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
