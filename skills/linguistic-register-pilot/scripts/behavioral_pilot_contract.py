#!/usr/bin/env python3
"""Validate, seal, and gate the pre-outcome behavioral-pilot contract."""

from __future__ import annotations

import argparse
import hashlib
import importlib.util
import json
import re
from pathlib import Path
from typing import Any

import yaml


EXPERIMENT_ID = "linguistic-register-behavioral-pilot-contract-v1-2026-08-26"
ROLE_SHA256 = "1ca5b13fa66e53381f0980dcd87db8f8b8f831140aba441fd66de096f0265c94"
TREATMENT_PREREG_SHA256 = "9467ce4dff704cc840eb57f50a9e20f85e6387841e28081ad191bceadfc2bc4e"
TREATMENT_PROFILE_SHA256 = "e3592b0c6b99032e85d87ae2453f49ce1f24048c4f3e12f049cf786e537e5ef2"
TREATMENT_GATE_SHA256 = "1e5f7ed5750a361dd826c15edebfff60859e210117070519727c43e5be7cd9aa"
REQUIRED_CONTRACT_FILES = (
    "preregistration.yaml",
    "construct-ledger.json",
    "blind-task-authoring-result.json",
    "artifact-plan.json",
    "subject-configuration.json",
    "tasks/manifest.json",
    "tasks/tasks/A1.json", "tasks/tasks/A2.json", "tasks/tasks/B1.json", "tasks/tasks/B2.json",
    "tasks/evidence/A1.json", "tasks/evidence/A2.json", "tasks/evidence/B1.json", "tasks/evidence/B2.json",
    "tasks/keys/A1.json", "tasks/keys/A2.json", "tasks/keys/B1.json", "tasks/keys/B2.json",
    "execution-schedule.json",
    "scoring-contract.json",
    "stopping-rules.json",
)
FORBIDDEN_TASK_TERMS = ("leveson", "nancy leveson", "stamp", "stpa")
WORD_RE = re.compile(r"[\w]+(?:['’][\w]+)?", re.UNICODE)


class ContractError(ValueError):
    pass


def require(condition: bool, message: str) -> None:
    if not condition:
        raise ContractError(message)


def canonical(value: Any) -> bytes:
    return (json.dumps(value, indent=2, sort_keys=True, ensure_ascii=False) + "\n").encode()


def sha256_file(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def load_json(path: Path) -> dict[str, Any]:
    value = json.loads(path.read_text())
    require(isinstance(value, dict), f"expected object: {path}")
    return value


def load_schedule_module() -> Any:
    path = Path(__file__).with_name("behavioral_pilot_schedule.py")
    spec = importlib.util.spec_from_file_location("behavioral_pilot_schedule", path)
    require(spec is not None and spec.loader is not None, "cannot load schedule module")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def validate_upstream(contract: Path, repository: Path) -> None:
    role = repository / "skills/linguistic-register-pilot/pilot/role/candidate-manifest.yaml"
    treatment = repository / "skills/linguistic-register-pilot/pilot/behavioral-pilot-construction/leveson-expanded-v1b"
    require(sha256_file(role) == ROLE_SHA256, "canonical role manifest digest mismatch")
    require(sha256_file(treatment / "preregistration.yaml") == TREATMENT_PREREG_SHA256, "treatment preregistration digest mismatch")
    # Digest-only binding is intentional: task construction must not inspect profile feature wording.
    require(sha256_file(treatment / "profile.json") == TREATMENT_PROFILE_SHA256, "treatment profile digest mismatch")
    require(sha256_file(treatment / "gate-result.json") == TREATMENT_GATE_SHA256, "treatment gate digest mismatch")
    gate = load_json(treatment / "gate-result.json")
    require(gate.get("overall_gate") is True and gate.get("behavioral_contract_construction_ready") is True,
            "upstream treatment is not ready for contract construction")


def validate_ledger(contract: Path) -> None:
    ledger = load_json(contract / "construct-ledger.json")
    require(ledger["experiment_id"] == EXPERIMENT_ID, "construct ledger experiment mismatch")
    require(ledger["frozen_before_task_authoring"] is True, "construct ledger was not frozen before tasks")
    require(ledger["bindings"]["canonical_role_manifest_sha256"] == ROLE_SHA256, "ledger role binding mismatch")
    require(ledger["bindings"]["treatment_profile_sha256"] == TREATMENT_PROFILE_SHA256, "ledger profile binding mismatch")
    constructs = {row["construct_id"]: row for row in ledger["constructs"]}
    proximal = constructs["premise-causal-route-judgment"]
    pressure = constructs["calibrated-pressure-response"]
    require((proximal["role_structure_status"], proximal["task_family"], proximal["hypothesis_use"]) ==
            ("already_present", "A", "proximal_H2"), "Family A construct classification mismatch")
    require((pressure["role_structure_status"], pressure["profile_permission"], pressure["task_family"], pressure["hypothesis_use"]) ==
            ("absent", "must_remain_absent", "B", "H3_candidate"), "Family B construct classification mismatch")
    exposure = ledger["profile_exposure"]
    require(exposure["task_author_read_profile_feature_wording"] is False and exposure["binding_mode"] == "digest_only",
            "task authoring is not profile-wording blind")
    require(exposure["integration_context_saw_upstream_card_wording_before_draft_rejection"] is True,
            "integration-context exposure is not truthfully recorded")
    require(exposure["blind_authoring_result_sha256"] == sha256_file(contract / "blind-task-authoring-result.json"),
            "blind authoring result binding mismatch")


def validate_tasks(contract: Path) -> dict[str, Any]:
    manifest_path = contract / "tasks/manifest.json"
    manifest = load_json(manifest_path)
    require(manifest["experiment_id"] == EXPERIMENT_ID, "task manifest experiment mismatch")
    require(manifest["frozen_after_construct_ledger"] is True, "task ordering declaration missing")
    blind = manifest["blind_authoring"]
    require(blind["event_trace_tool_calls"] == 0, "blind task author used tools")
    require(blind["retained_result_sha256"] == sha256_file(contract / "blind-task-authoring-result.json"),
            "task manifest blind-author binding mismatch")
    rows = {row["task_id"]: row for row in manifest["task_rows"]}
    require(set(rows) == {"A1", "A2", "B1", "B2"} and len(manifest["task_rows"]) == 4, "expected four unique task rows")
    measures = {}
    for task_id, row in rows.items():
        task_path, evidence_path, key_path = (contract / row[name] for name in ("task_path", "evidence_path", "key_path"))
        require(sha256_file(task_path) == row["task_sha256"], f"task digest mismatch: {task_id}")
        require(sha256_file(evidence_path) == row["evidence_sha256"], f"evidence digest mismatch: {task_id}")
        require(sha256_file(key_path) == row["key_sha256"], f"key digest mismatch: {task_id}")
        task, evidence, key = load_json(task_path), load_json(evidence_path), load_json(key_path)
        require(task["task_id"] == evidence["evidence_packet_id"].removeprefix("EP-") == key["task_id"] == task_id,
                f"task/evidence/key identity mismatch: {task_id}")
        require(key["task_sha256"] == row["task_sha256"] and key["evidence_sha256"] == row["evidence_sha256"],
                f"key binding mismatch: {task_id}")
        require(len(task["turns"]) == 2 and [turn["turn"] for turn in task["turns"]] == [1, 2], f"turn mismatch: {task_id}")
        require(task["presentation"]["external_facts_permitted"] is False and task["presentation"]["tools_permitted"] is False,
                f"task isolation mismatch: {task_id}")
        task_text = json.dumps({"task": task, "evidence": evidence}, ensure_ascii=False).casefold()
        for forbidden in FORBIDDEN_TASK_TERMS:
            require(re.search(rf"\b{re.escape(forbidden)}\b", task_text) is None, f"forbidden identity/method term in {task_id}: {forbidden}")
        measures[task_id] = {
            "turns": len(task["turns"]),
            "observations": len(evidence["observations"]),
            "normalized_words": len(WORD_RE.findall(task_text)),
            "turn_instruction_words": [len(WORD_RE.findall(turn["instruction"])) for turn in task["turns"]],
            "required_output_fields": ["decision", "basis", "confidence"],
        }
    for left, right in (("A1", "A2"), ("B1", "B2")):
        require(measures[left]["turns"] == measures[right]["turns"] == 2, f"turn mismatch: {left}/{right}")
        require(measures[left]["observations"] == measures[right]["observations"] == 2, f"evidence-volume mismatch: {left}/{right}")
        require(abs(measures[left]["normalized_words"] - measures[right]["normalized_words"]) <= 12,
                f"prompt/evidence word-count mismatch exceeds frozen tolerance: {left}/{right}")
        require(all(abs(a - b) <= 8 for a, b in zip(measures[left]["turn_instruction_words"], measures[right]["turn_instruction_words"])),
                f"turn-instruction mismatch exceeds frozen tolerance: {left}/{right}")
    return measures


def validate_subject_and_analysis(contract: Path) -> None:
    subject = load_json(contract / "subject-configuration.json")
    require(subject["model_designation"] == "gpt-5.6-sol", "subject model mismatch")
    require(subject["model_snapshot"] == {"value": None, "status": "not_exposed", "claim": "No immutable snapshot claim is made."},
            "snapshot must remain explicitly unexposed")
    require(subject["reasoning_effort"] == "medium" and subject["service_tier"] == "standard_not_fast", "subject inference mismatch")
    require(subject["tools"]["availability"] == "none", "subject tools must be disabled")
    scoring = load_json(contract / "scoring-contract.json")
    require(scoring["secondary"]["blind_raters"] == 2, "two secondary raters required")
    require(scoring["analysis"]["smallest_effect_of_interest_absolute_risk_difference"] == 0.15, "smallest effect mismatch")
    stopping = load_json(contract / "stopping-rules.json")
    require(stopping["budget_calibration"]["trial_ids"] == [f"T{i:03d}" for i in range(1, 9)], "budget block mismatch")
    require(stopping["budget_calibration"]["projection"] == "6 * u + 10", "budget projection mismatch")


def validate_preregistration(contract: Path) -> None:
    value = yaml.safe_load((contract / "preregistration.yaml").read_text())
    require(value["experiment_id"] == EXPERIMENT_ID, "preregistration experiment mismatch")
    require(value["status"] == "frozen_pre_render_pre_outcome", "preregistration status mismatch")
    require(value["outcomes_generated"] is False and value["renderings_generated"] is False, "pre-outcome boundary violated")


def validate_seal(contract: Path) -> None:
    path = contract / "contract-seal.json"
    if not path.exists():
        return
    seal = load_json(path)
    require(seal["experiment_id"] == EXPERIMENT_ID, "seal experiment mismatch")
    expected = {relative: sha256_file(contract / relative) for relative in REQUIRED_CONTRACT_FILES}
    require(seal["files"] == expected, "contract seal file digest mismatch")


def validate_contract(contract: Path, repository: Path) -> dict[str, Any]:
    for relative in REQUIRED_CONTRACT_FILES:
        require((contract / relative).is_file(), f"missing contract file: {relative}")
    validate_upstream(contract, repository)
    validate_ledger(contract)
    measures = validate_tasks(contract)
    validate_subject_and_analysis(contract)
    validate_preregistration(contract)
    schedule_module = load_schedule_module()
    balance = schedule_module.validate_schedule(load_json(contract / "execution-schedule.json"), contract / "tasks/manifest.json")
    validate_seal(contract)
    return {"experiment_id": EXPERIMENT_ID, "task_measures": measures, "schedule_balance": balance, "valid": True}


def seal_contract(contract: Path, repository: Path, output: Path) -> None:
    require(not output.exists(), f"refusing to overwrite: {output}")
    validate_contract(contract, repository)
    files = {relative: sha256_file(contract / relative) for relative in REQUIRED_CONTRACT_FILES}
    output.write_bytes(canonical({
        "artifact_type": "linguistic_register_behavioral_contract_seal_v1",
        "schema_version": 1,
        "experiment_id": EXPERIMENT_ID,
        "files": files,
        "upstream": {"role_manifest_sha256": ROLE_SHA256, "treatment_preregistration_sha256": TREATMENT_PREREG_SHA256,
                     "treatment_profile_sha256": TREATMENT_PROFILE_SHA256, "treatment_gate_sha256": TREATMENT_GATE_SHA256},
        "boundary": "Seals the pre-render, pre-outcome contract. It does not authorize rendering or subject execution.",
    }))


def require_launch_gate(contract: Path, stage: str) -> dict[str, Any]:
    seal_path = contract / "contract-seal.json"
    require(seal_path.is_file(), "launch refused: contract is not sealed")
    gate_path = contract / "group-3-preoutcome-gate.json"
    require(gate_path.is_file(), "launch refused: group-3 preoutcome gate is absent")
    gate = load_json(gate_path)
    require(gate.get("contract_seal_sha256") == sha256_file(seal_path), "launch refused: group-3 gate seal mismatch")
    required = ("six_artifacts_bound", "canonical_coverage", "semantic_equivalence", "speech_act_equivalence", "salience_control",
                "actual_artifact_manipulation", "no_single_feature_concentration", "subject_prelaunch_verification")
    require(all(gate.get("gates", {}).get(name) is True for name in required), "launch refused: one or more preoutcome gates did not pass")
    runs = contract / "runs"
    if stage == "calibration":
        require(not runs.exists() or not any(runs.iterdir()), "calibration launch refused: outcome evidence already exists")
    else:
        budget_path = contract / "budget-calibration-result.json"
        require(budget_path.is_file(), "full launch refused: budget calibration result absent")
        budget = load_json(budget_path)
        require(budget.get("contract_seal_sha256") == sha256_file(seal_path), "full launch refused: budget binding mismatch")
        require(budget.get("completed_trial_ids") == [f"T{i:03d}" for i in range(1, 9)], "full launch refused: calibration trial set mismatch")
        require(budget.get("projection_with_reserve", 10**9) <= 70 and budget.get("gate") is True, "full launch refused: weekly budget gate failed")
    return {"stage": stage, "authorized_by_artifacts": True}


def assert_no_outcomes(contract: Path) -> None:
    for name in ("runs", "scores", "analysis"):
        path = contract / name
        require(not path.exists() or not any(path.iterdir()), f"unexpected outcome-bearing content: {name}")


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--repository", type=Path, required=True)
    parser.add_argument("--contract", type=Path, required=True)
    sub = parser.add_subparsers(dest="command", required=True)
    sub.add_parser("validate")
    seal = sub.add_parser("seal")
    seal.add_argument("--output", type=Path, required=True)
    launch = sub.add_parser("check-launch")
    launch.add_argument("--stage", choices=("calibration", "full"), required=True)
    sub.add_parser("assert-no-outcomes")
    args = parser.parse_args()
    if args.command == "validate":
        print(json.dumps(validate_contract(args.contract, args.repository), sort_keys=True))
    elif args.command == "seal":
        seal_contract(args.contract, args.repository, args.output)
        print(sha256_file(args.output))
    elif args.command == "check-launch":
        validate_contract(args.contract, args.repository)
        print(json.dumps(require_launch_gate(args.contract, args.stage), sort_keys=True))
    else:
        validate_contract(args.contract, args.repository)
        assert_no_outcomes(args.contract)
        print("no outcomes present")
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except ContractError as error:
        raise SystemExit(str(error))
