from __future__ import annotations

import importlib.util
import json
import tempfile
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
MODULE_PATH = ROOT / "scripts" / "micro_render_v2b_artifacts.py"
SPEC = importlib.util.spec_from_file_location("micro_render_v2b_artifacts", MODULE_PATH)
assert SPEC is not None and SPEC.loader is not None
V2B = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(V2B)
PLAN_PATH = ROOT / "pilot" / "micro-render-v2b" / "preregistration.yaml"


def prose() -> str:
    return " ".join(f"word{index}" for index in range(1, 101))


class MicroRenderV2BTests(unittest.TestCase):
    def test_plan_is_frozen_and_uses_fresh_experiment(self) -> None:
        plan, repository, effective = V2B.validate_plan(V2B.V2.V1.BASE.load_yaml(PLAN_PATH), PLAN_PATH)
        self.assertEqual(plan["experiment_id"], "semantic-licensed-micro-render-v2b-2026-08-25")
        self.assertEqual(effective["experiment_id"], plan["experiment_id"])
        self.assertIn("v2b", effective["rendering"]["seed"])
        self.assertTrue(repository.is_absolute())

    def test_plain_string_is_accepted_unchanged(self) -> None:
        value = prose()
        extracted, mode = V2B.normalize_transport({"text": value})
        self.assertEqual(extracted, value)
        self.assertEqual(mode, "direct")

    def test_exact_single_text_wrapper_is_unwrapped_once(self) -> None:
        value = prose()
        extracted, mode = V2B.normalize_transport({"text": json.dumps({"text": value})})
        self.assertEqual(extracted, value)
        self.assertEqual(mode, "unwrapped_once")

    def test_extra_keys_arrays_scalars_and_fences_are_rejected(self) -> None:
        invalid = [
            json.dumps({"text": prose(), "extra": True}),
            json.dumps([prose()]),
            json.dumps(prose()),
            "```json\n" + json.dumps({"text": prose()}) + "\n```",
        ]
        for value in invalid:
            with self.subTest(value=value[:20]), self.assertRaises(V2B.V2BError):
                V2B.normalize_transport({"text": value})

    def test_malformed_and_second_nested_wrapper_are_rejected(self) -> None:
        with self.assertRaisesRegex(V2B.V2BError, "malformed"):
            V2B.normalize_transport({"text": '{"text": "broken"'})
        nested = json.dumps({"text": json.dumps({"text": prose()})})
        with self.assertRaisesRegex(V2B.V2BError, "second nested"):
            V2B.normalize_transport({"text": nested})

    def test_non_string_outer_text_is_rejected(self) -> None:
        with self.assertRaisesRegex(V2B.V2BError, "nonempty string"):
            V2B.normalize_transport({"text": [prose()]})

    def test_transport_artifact_digest_is_of_extracted_bytes(self) -> None:
        value = prose()
        extracted, _ = V2B.normalize_transport({"text": json.dumps({"text": value})})
        self.assertEqual(V2B.hashlib.sha256(extracted.encode()).hexdigest(),
                         V2B.hashlib.sha256(value.encode()).hexdigest())

    def test_emit_refuses_overwrite(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            path = Path(temporary) / "artifact.json"
            V2B.emit({"first": True}, path)
            with self.assertRaisesRegex(V2B.V2BError, "overwrite"):
                V2B.emit({"second": True}, path)

    def test_retained_transport_report_reconstructs_and_stops_downstream(self) -> None:
        plan, repository, _ = V2B.validate_plan(V2B.V2.V1.BASE.load_yaml(PLAN_PATH), PLAN_PATH)
        run_dir = ROOT / "pilot" / "micro-render-v2b" / "runs"
        retained = V2B.load_json(ROOT / "pilot" / "micro-render-v2b" / "transport-gate-report.json")
        reconstructed = V2B.evaluate_transport(
            plan, repository, retained["postbatch_checkpoint_commit_oid"], run_dir)
        self.assertEqual(reconstructed, retained)
        self.assertFalse(retained["gate_passed"])
        self.assertEqual(retained["accepted_jobs"], 15)
        self.assertEqual(retained["semantic_status"], "not_run_transport_gate_failed")
        self.assertFalse((run_dir / "semantic").exists())
        self.assertFalse((run_dir / "matching").exists())

    def test_review_qualification_binds_and_withdraws_invalid_aggregations(self) -> None:
        qualification_path = ROOT / "pilot" / "micro-render-v2b" / "review-qualification.json"
        qualified = V2B.load_json(
            ROOT / "pilot" / "micro-render-v2b" / "transport-gate-report-qualified.json")
        self.assertEqual(qualified["review_qualification_sha256"], V2B.file_digest(qualification_path))
        self.assertFalse(qualified["gate_passed"])
        self.assertEqual(qualified["normalization_totals"], {
            "direct": 7, "unwrapped_once": 8, "rejected": 1})
        self.assertEqual(qualified["condition_aggregation_status"],
                         "withdrawn_sealed_key_packet_digest_mismatch")

    def test_retained_sealed_key_packet_digests_all_mismatch(self) -> None:
        run_dir = ROOT / "pilot" / "micro-render-v2b" / "runs"
        key = V2B.load_json(run_dir / "sealed-render-key.json")
        mismatches = []
        for sample_id, record in key["samples"].items():
            actual = V2B.file_digest(run_dir / "samples" / sample_id / "render-packet.json")
            mismatches.append(record["packet_artifact_sha256"] != actual)
        self.assertEqual(mismatches, [True] * 16)

    def test_retained_batch_has_complete_attempt_and_receipt_chain(self) -> None:
        evidence = V2B.load_json(
            ROOT / "pilot" / "micro-render-v2b" / "runs" / "immediate-batch-evidence.json")
        self.assertEqual(evidence["state_counts"], {
            "planned_not_attempted": 0,
            "attempted_no_receipt": 0,
            "completed_failed": 0,
            "completed_receipt": 16,
        })
        rejected = [item for item in evidence["outcomes"] if item["normalization"] == "rejected"]
        self.assertEqual([(item["sample_id"], item["reason"]) for item in rejected],
                         [("S04", "render text must contain 90 through 130 words")])


if __name__ == "__main__":
    unittest.main()
