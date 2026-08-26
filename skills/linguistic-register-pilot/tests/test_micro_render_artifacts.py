from __future__ import annotations

import importlib.util
import json
import unittest
from pathlib import Path


ROOT = Path(__file__).parents[1]
SCRIPT = ROOT / "scripts" / "micro_render_artifacts.py"
PLAN_PATH = ROOT / "pilot" / "micro-render-v1" / "preregistration.yaml"
SPEC = importlib.util.spec_from_file_location("micro_render_artifacts", SCRIPT)
assert SPEC and SPEC.loader
MICRO = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MICRO)


class MicroRenderArtifactsTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.plan, cls.base_plan, cls.base_path = MICRO.validate_plan(MICRO.BASE.load_yaml(PLAN_PATH), PLAN_PATH)

    def provenance(self) -> dict:
        return {
            "provider": "fixture", "model": "fixture", "reasoning_effort": "fixture",
            "execution_mode": "fixture", "fresh_context": True, "packet_only": True,
            "started_at_utc": "2026-01-01T00:00:00Z", "completed_at_utc": "2026-01-01T00:01:00Z",
        }

    def test_render_jobs_are_deterministic_complete_and_identity_blind(self) -> None:
        first = MICRO.prepare_render_jobs(self.plan, self.base_plan, self.base_path, PLAN_PATH)
        second = MICRO.prepare_render_jobs(self.plan, self.base_plan, self.base_path, PLAN_PATH)
        self.assertEqual(first, second)
        packets, key = first
        self.assertEqual(len(packets), 16)
        self.assertEqual(len(key["samples"]), 16)
        for packet in packets.values():
            rendered = json.dumps(packet).lower()
            for forbidden in ("gelman", "leveson", "shaw", "candidate_id", "source_id"):
                self.assertNotIn(forbidden, rendered)

    def test_render_normalization_binds_exact_packet(self) -> None:
        packets, _ = MICRO.prepare_render_jobs(self.plan, self.base_plan, self.base_path, PLAN_PATH)
        packet = packets[sorted(packets)[0]]
        fixture_text = " ".join(["valid"] * 75)
        result = MICRO.normalize_render({"text": fixture_text}, packet, self.provenance())
        self.assertEqual(result["packet_artifact_sha256"], MICRO.digest_value(packet))
        self.assertEqual(result["sample_id"], packet["sample_id"])

    def test_render_normalization_rejects_nested_structured_output(self) -> None:
        packets, _ = MICRO.prepare_render_jobs(self.plan, self.base_plan, self.base_path, PLAN_PATH)
        packet = packets[sorted(packets)[0]]
        nested = json.dumps({"text": " ".join(["prose"] * 75)})
        with self.assertRaisesRegex(MICRO.MicroRenderError, "serialized structured output"):
            MICRO.normalize_render({"text": nested}, packet, self.provenance())

    def test_provenance_requires_ordered_iso_timestamps(self) -> None:
        invalid = self.provenance()
        invalid["started_at_utc"] = "not-a-time"
        with self.assertRaisesRegex(MICRO.MicroRenderError, "ISO-8601"):
            MICRO.validate_provenance(invalid, "fixture")
        reversed_times = self.provenance()
        reversed_times["started_at_utc"] = "2026-01-01T00:02:00Z"
        with self.assertRaisesRegex(MICRO.MicroRenderError, "must not precede"):
            MICRO.validate_provenance(reversed_times, "fixture")

    def test_semantic_equivalent_requires_complete_clean_mapping(self) -> None:
        packets, _ = MICRO.prepare_render_jobs(self.plan, self.base_plan, self.base_path, PLAN_PATH)
        packet = packets[sorted(packets)[0]]
        render = MICRO.normalize_render({"text": " ".join(["fixture"] * 75)}, packet, self.provenance())
        review_packet = MICRO.semantic_packet(self.plan, packet, render)
        raw = {
            "verdict": "equivalent",
            "proposition_checks": [{"proposition_id": proposition["id"], "status": "preserved",
                                     "evidence": "Fixture evidence."}
                                    for proposition in review_packet["semantic_brief"]["required_propositions"]],
            "speech_act_equivalent": True, "added_meaning": [], "summary": "Equivalent fixture.",
        }
        result = MICRO.normalize_semantic(raw, review_packet, self.provenance())
        self.assertEqual(result["verdict"], "equivalent")
        raw["added_meaning"] = ["Invented requirement."]
        with self.assertRaisesRegex(MICRO.MicroRenderError, "verdict conflicts"):
            MICRO.normalize_semantic(raw, review_packet, self.provenance())

    def test_matching_result_requires_all_sixteen_assignments(self) -> None:
        packet = {
            "experiment_id": "fixture", "pass_number": 1,
            "anonymous_texts": [{"text_id": f"T{index:02d}"} for index in range(1, 17)],
            "anonymous_references": [{"reference_id": f"R{index:02d}"} for index in range(1, 5)],
        }
        raw = {
            "assignments": [{"text_id": f"T{index:02d}", "reference_id": "R01", "confidence": 3,
                             "rationale": "Fixture."} for index in range(1, 17)],
            "method_limitations": ["Fixture."],
        }
        result = MICRO.normalize_matching(raw, packet, self.provenance())
        self.assertEqual(len(result["assignments"]), 16)
        raw["assignments"].pop()
        with self.assertRaisesRegex(MICRO.MicroRenderError, "assign every text"):
            MICRO.normalize_matching(raw, packet, self.provenance())

    def test_failed_semantic_gate_produces_stopped_report_without_matching(self) -> None:
        _, render_key = MICRO.prepare_render_jobs(self.plan, self.base_plan, self.base_path, PLAN_PATH)
        assessment = {
            "accepted_samples": 15, "required_samples": 16, "passed": False,
            "bindings": [],
            "rejected_samples": [{"sample_id": "S16", "verdict": "not_equivalent",
                                  "added_meaning": ["Fixture addition."]}],
        }
        report = MICRO.aggregate(self.plan, render_key, [], PLAN_PATH, assessment)
        self.assertFalse(report["gate_passed"])
        self.assertEqual(report["matching_status"], "not_run_semantic_gate_failed")
        self.assertEqual(report["matching_runs"], [])
        self.assertEqual(report["condition_results"], [])

    def test_aggregate_rejects_matching_presence_in_failed_gate(self) -> None:
        _, render_key = MICRO.prepare_render_jobs(self.plan, self.base_plan, self.base_path, PLAN_PATH)
        assessment = {"accepted_samples": 15, "required_samples": 16, "passed": False,
                      "bindings": [], "rejected_samples": []}
        with self.assertRaisesRegex(MICRO.MicroRenderError, "must not run"):
            MICRO.aggregate(self.plan, render_key, [({}, {}, {})], PLAN_PATH, assessment)

    def test_aggregate_requires_three_matching_passes_after_semantic_success(self) -> None:
        _, render_key = MICRO.prepare_render_jobs(self.plan, self.base_plan, self.base_path, PLAN_PATH)
        assessment = {"accepted_samples": 16, "required_samples": 16, "passed": True,
                      "bindings": [], "rejected_samples": []}
        with self.assertRaisesRegex(MICRO.MicroRenderError, "all matching passes"):
            MICRO.aggregate(self.plan, render_key, [], PLAN_PATH, assessment)

    def test_retained_run_reconstructs_semantic_gate(self) -> None:
        run_dir = ROOT / "pilot" / "micro-render-v1" / "runs"
        assessment = MICRO.assess_semantic_gate(
            self.plan, self.base_plan, self.base_path, PLAN_PATH, run_dir)
        self.assertEqual(assessment["accepted_samples"], 10)
        self.assertEqual([item["sample_id"] for item in assessment["rejected_samples"]],
                         ["S02", "S06", "S07", "S08", "S11", "S13"])

    def test_adjudication_packet_is_deterministic_and_style_blind(self) -> None:
        run_dir = ROOT / "pilot" / "micro-render-v1" / "runs"
        packet = MICRO.prepare_adjudication_packet(
            self.plan, self.base_plan, self.base_path, PLAN_PATH, run_dir)
        self.assertEqual(len(packet["items"]), 16)
        self.assertEqual(packet, MICRO.prepare_adjudication_packet(
            self.plan, self.base_plan, self.base_path, PLAN_PATH, run_dir))
        serialized = json.dumps(packet).lower()
        for forbidden in ("gelman", "leveson", "shaw", "condition_id", "original_semantic_result"):
            self.assertNotIn(forbidden, serialized)

    def test_successful_aggregate_scores_all_three_passes(self) -> None:
        _, render_key = MICRO.prepare_render_jobs(self.plan, self.base_plan, self.base_path, PLAN_PATH)
        conditions = sorted({item["condition_id"] for item in render_key["samples"].values()})
        samples = sorted(render_key["samples"])
        runs = []
        for pass_number in range(1, 4):
            references = {condition: f"R{index:02d}" for index, condition in enumerate(conditions, 1)}
            texts = {f"T{index:02d}": sample for index, sample in enumerate(samples, 1)}
            packet = {"pass_number": pass_number}
            key = {"reference_mapping": {value: key for key, value in references.items()},
                   "text_mapping": texts,
                   "condition_by_sample": {sample: value["condition_id"]
                                           for sample, value in render_key["samples"].items()}}
            result = {"assignments": [
                {"text_id": text_id,
                 "reference_id": references[render_key["samples"][sample]["condition_id"]],
                 "confidence": 4, "rationale": "Fixture."}
                for text_id, sample in texts.items()]}
            runs.append((packet, key, result))
        assessment = {"accepted_samples": 16, "required_samples": 16, "passed": True,
                      "bindings": [], "rejected_samples": []}
        report = MICRO.aggregate(self.plan, render_key, runs, PLAN_PATH, assessment)
        self.assertTrue(report["gate_passed"])
        self.assertTrue(all(item["correct_assignments"] == 12 for item in report["condition_results"]))


if __name__ == "__main__":
    unittest.main()
