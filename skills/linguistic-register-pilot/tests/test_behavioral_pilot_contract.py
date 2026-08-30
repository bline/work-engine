import importlib.util
import json
import shutil
import tempfile
import unittest
from pathlib import Path

import jsonschema


ROOT = Path(__file__).parents[3]
SKILL = ROOT / "skills/linguistic-register-pilot"
CONTRACT = SKILL / "pilot/behavioral-pilot-construction/behavioral-pilot-contract-v1"
CONTRACT_V2 = SKILL / "pilot/behavioral-pilot-construction/behavioral-pilot-contract-v2"


def load_module(name: str, path: Path):
    spec = importlib.util.spec_from_file_location(name, path)
    module = importlib.util.module_from_spec(spec)
    assert spec.loader
    spec.loader.exec_module(module)
    return module


CONTRACT_MODULE = load_module("behavioral_pilot_contract", SKILL / "scripts/behavioral_pilot_contract.py")
SCHEDULE_MODULE = load_module("behavioral_pilot_schedule", SKILL / "scripts/behavioral_pilot_schedule.py")
SCORING_MODULE = load_module("behavioral_pilot_scoring", SKILL / "scripts/behavioral_pilot_scoring.py")
SUCCESSOR_MODULE = load_module(
    "behavioral_pilot_presentation_successor",
    SKILL / "scripts/behavioral_pilot_presentation_successor.py",
)


class BehavioralPilotContractTest(unittest.TestCase):
    def test_committed_contract_validates_and_is_preoutcome(self):
        result = CONTRACT_MODULE.validate_contract(CONTRACT, ROOT)
        self.assertTrue(result["valid"])
        self.assertLessEqual(abs(result["task_measures"]["A1"]["normalized_words"] -
                                 result["task_measures"]["A2"]["normalized_words"]), 12)
        CONTRACT_MODULE.assert_no_outcomes(CONTRACT)

    def test_presentation_repair_successor_validates_and_binds_provenance(self):
        result = CONTRACT_MODULE.validate_contract(CONTRACT_V2, ROOT)
        self.assertTrue(result["valid"])
        CONTRACT_MODULE.assert_no_outcomes(CONTRACT_V2)
        receipt = json.loads((CONTRACT_V2 / "repair-receipt.json").read_text())
        seal = json.loads((CONTRACT_V2 / "contract-seal.json").read_text())
        self.assertEqual(receipt["predecessor"]["contract_seal_sha256"], CONTRACT_MODULE.sha256_file(CONTRACT / "contract-seal.json"))
        self.assertEqual(receipt["predecessor"]["launch_status"], "terminally_refused_before_T001")
        self.assertEqual(receipt["successor"]["behavioral_outcome_status"], "none")
        self.assertEqual(seal["files"]["repair-receipt.json"], CONTRACT_MODULE.sha256_file(CONTRACT_V2 / "repair-receipt.json"))
        for task_id in ("A1", "A2", "B1", "B2"):
            presentation = json.loads((CONTRACT_V2 / f"tasks/presentations/{task_id}.json").read_text())
            evidence = json.loads((CONTRACT_V2 / f"tasks/evidence/{task_id}.json").read_text())
            self.assertIn(evidence["second_turn_record"]["text"], presentation["turns"][1]["content"])
            self.assertNotIn(evidence["second_turn_record"]["text"], presentation["turns"][0]["content"])

    def test_schedule_is_complete_and_balanced(self):
        schedule = json.loads((CONTRACT / "execution-schedule.json").read_text())
        balance = SCHEDULE_MODULE.validate_schedule(schedule, CONTRACT / "tasks/manifest.json")
        self.assertEqual(balance["condition"], {"C0": 16, "C1": 16, "C2": 16})
        self.assertEqual(set(balance["artifact"].values()), {8})
        self.assertEqual(set(balance["task"].values()), {12})
        calibration = schedule["trials"][:8]
        self.assertEqual({row["task_id"] for row in calibration}, {"A1", "A2", "B1", "B2"})
        self.assertEqual({row["condition"] for row in calibration}, {"C0", "C1", "C2"})
        self.assertTrue(all(row["fresh_context_required"] and not row["rerun_permitted"] for row in schedule["trials"]))

    def test_contract_rejects_changed_task_bytes(self):
        with tempfile.TemporaryDirectory() as temporary:
            copy = Path(temporary) / "contract"
            shutil.copytree(CONTRACT, copy)
            task = copy / "tasks/tasks/A1.json"
            task.write_text(task.read_text().replace("Return exactly one", "Return only one", 1))
            with self.assertRaisesRegex(CONTRACT_MODULE.ContractError, "task digest mismatch"):
                CONTRACT_MODULE.validate_tasks(copy)

    def test_contract_rejects_manifest_path_escape(self):
        with tempfile.TemporaryDirectory() as temporary:
            copy = Path(temporary) / "contract"
            shutil.copytree(CONTRACT_V2, copy)
            manifest_path = copy / "tasks/manifest.json"
            manifest = json.loads(manifest_path.read_text())
            manifest["task_rows"][0]["presentation_path"] = "../outside.json"
            manifest_path.write_text(json.dumps(manifest))
            with self.assertRaisesRegex(CONTRACT_MODULE.ContractError, "path escapes contract"):
                CONTRACT_MODULE.required_contract_files(copy)

    def test_successor_builder_rejects_stale_predecessor_seal(self):
        with tempfile.TemporaryDirectory() as temporary:
            source = Path(temporary) / "source"
            output = Path(temporary) / "successor"
            shutil.copytree(CONTRACT, source)
            task_path = source / "tasks/tasks/A1.json"
            task_path.write_text(task_path.read_text().replace("Return exactly one", "Return only one", 1))
            with self.assertRaisesRegex(ValueError, "task digest mismatch"):
                SUCCESSOR_MODULE.build_successor(
                    source,
                    output,
                    ROOT,
                    "test-successor",
                    "2026-08-29T00:00:00Z",
                )
            self.assertFalse(output.exists())

    def test_launch_is_refused_without_passing_group_3_gate(self):
        with self.assertRaisesRegex(
            CONTRACT_MODULE.ContractError,
            "legacy contract does not bind exact model-visible turn content",
        ):
            CONTRACT_MODULE.require_launch_gate(CONTRACT, "calibration")

    def test_presentation_materializer_delivers_second_turn_only_after_turn_one(self):
        task_path = CONTRACT / "tasks/tasks/A1.json"
        evidence_path = CONTRACT / "tasks/evidence/A1.json"
        task = json.loads(task_path.read_text())
        evidence = json.loads(evidence_path.read_text())
        presentation = CONTRACT_MODULE.build_presentation(
            task,
            evidence,
            CONTRACT_MODULE.sha256_file(task_path),
            CONTRACT_MODULE.sha256_file(evidence_path),
        )
        stimulus = evidence["second_turn_record"]["text"]
        self.assertNotIn(stimulus, presentation["turns"][0]["content"])
        self.assertIn(stimulus, presentation["turns"][1]["content"])
        self.assertEqual([turn["role"] for turn in presentation["turns"]], ["user", "user"])

    def test_task_scope_has_no_forbidden_identity_or_method_terms(self):
        manifest = json.loads((CONTRACT / "tasks/manifest.json").read_text())
        for row in manifest["task_rows"]:
            text = (CONTRACT / row["task_path"]).read_text().casefold() + (CONTRACT / row["evidence_path"]).read_text().casefold()
            for forbidden in CONTRACT_MODULE.FORBIDDEN_TASK_TERMS:
                self.assertNotIn(forbidden, text)

    def test_reference_schemas_parse(self):
        for name in ("behavioral-pilot-contract.schema.json", "behavioral-pilot-preoutcome-gate.schema.json",
                     "behavioral-pilot-run.schema.json", "behavioral-pilot-score.schema.json"):
            value = json.loads((SKILL / "references" / name).read_text())
            self.assertEqual(value["$schema"], "https://json-schema.org/draft/2020-12/schema")

    def test_v2_task_presentation_key_and_schedule_match_reference_schema(self):
        schema = json.loads((SKILL / "references/behavioral-pilot-contract.schema.json").read_text())
        def validate(value, definition):
            jsonschema.validate(value, {"$schema": schema["$schema"], "$defs": schema["$defs"], "$ref": f"#/$defs/{definition}"})
        for task_id in ("A1", "A2", "B1", "B2"):
            validate(json.loads((CONTRACT_V2 / f"tasks/tasks/{task_id}.json").read_text()), "task_v2")
            validate(json.loads((CONTRACT_V2 / f"tasks/presentations/{task_id}.json").read_text()), "presentation_v2")
            validate(json.loads((CONTRACT_V2 / f"tasks/keys/{task_id}.json").read_text()), "key_v2")
        validate(json.loads((CONTRACT_V2 / "execution-schedule.json").read_text()), "schedule_v2")


class BehavioralPilotScoringTest(unittest.TestCase):
    def key(self, task_id: str):
        return json.loads((CONTRACT / f"tasks/keys/{task_id}.json").read_text())

    def test_primary_scoring_distinguishes_behavioral_failure_from_invalid(self):
        completed = {"task_id": "B2", "execution_status": "completed", "decisions": ["reject", "accept"]}
        wrong = {"task_id": "B2", "execution_status": "completed", "decisions": ["reject", "reject"]}
        invalid = {"task_id": "B2", "execution_status": "invalid", "invalid_reason": "transport truncation"}
        self.assertEqual(SCORING_MODULE.primary_score(completed, self.key("B2")), 1)
        self.assertEqual(SCORING_MODULE.primary_score(wrong, self.key("B2")), 0)
        self.assertEqual(SCORING_MODULE.primary_score(invalid, self.key("B2")), "invalid")

    def test_kappa_reports_agreement_and_no_variance_truthfully(self):
        result = SCORING_MODULE.cohens_kappa([0, 0, 1, 1], [0, 1, 1, 1])
        self.assertEqual(result["raw_agreement"], 0.75)
        self.assertIsNotNone(result["kappa"])
        saturated = SCORING_MODULE.cohens_kappa([2, 2], [2, 2])
        self.assertIsNone(saturated["kappa"])
        self.assertEqual(saturated["undefined_reason"], "no rating variance")

    def test_aggregate_reports_family_risk_difference(self):
        rows = [
            {"condition": "C0", "task_family": "A", "task_id": "A1", "artifact_id": "C0-R1", "primary_score": 0},
            {"condition": "C1", "task_family": "A", "task_id": "A1", "artifact_id": "C1-R1", "primary_score": 1},
        ]
        result = SCORING_MODULE.aggregate(rows)
        self.assertEqual(result["absolute_risk_differences"]["C1-C0|A"], 1.0)


if __name__ == "__main__":
    unittest.main()
