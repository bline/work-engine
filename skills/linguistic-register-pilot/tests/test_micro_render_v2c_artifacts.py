from __future__ import annotations

import copy
import hashlib
import importlib.util
import json
import tempfile
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
MODULE_PATH = ROOT / "scripts" / "micro_render_v2c_artifacts.py"
SPEC = importlib.util.spec_from_file_location("micro_render_v2c_artifacts", MODULE_PATH)
assert SPEC is not None and SPEC.loader is not None
V2C = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(V2C)
PLAN_PATH = ROOT / "pilot" / "micro-render-v2c" / "preregistration.yaml"


class MicroRenderV2CTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.plan, cls.repository, cls.effective = V2C.validate_plan(
            V2C.V2.V1.BASE.load_yaml(PLAN_PATH), PLAN_PATH)

    def test_plan_is_frozen_fresh_and_targets_above_acceptance_floor(self) -> None:
        self.assertEqual(self.plan["experiment_id"],
                         "semantic-licensed-micro-render-v2c-2026-08-25")
        self.assertIn("No v1, v2, or v2b", self.plan["freshness"]["prior_output_policy"])
        self.assertEqual(self.plan["word_policy"], {
            "generation_target_min": 110, "generation_target_max": 120,
            "acceptance_min": 90, "acceptance_max": 130,
        })

    def test_opaque_ids_are_independent_of_assignment_and_condition_labels(self) -> None:
        ids = V2C.opaque_ids(self.plan)
        changed = copy.deepcopy(self.plan)
        changed["sample_identity"]["assignment_seed"] = "different-assignment"
        self.assertEqual(ids, V2C.opaque_ids(changed))
        self.assertEqual(len(ids), 16)
        self.assertEqual(len(set(ids)), 16)
        for sample_id in ids:
            self.assertRegex(sample_id, r"^X[0-9A-F]{12}$")
            self.assertNotIn("gelman", sample_id.lower())
            self.assertNotIn("leveson", sample_id.lower())
            self.assertNotIn("shaw", sample_id.lower())
            self.assertNotIn("neutral", sample_id.lower())

    def test_assignment_covers_every_cell_once(self) -> None:
        assignments = V2C.assigned_cells(self.plan, self.effective)
        self.assertEqual(set(assignments), set(V2C.opaque_ids(self.plan)))
        self.assertEqual(len(set(assignments.values())), 16)

    def test_launch_set_and_request_digests_reconstruct(self) -> None:
        schema = V2C.binding_path(
            self.repository, self.effective["execution"]["render"]["schema"], "schema")
        with tempfile.TemporaryDirectory() as temporary:
            directory = Path(temporary)
            jobs = []
            for job_id in ("XAAAAAAAAAAAA", "XBBBBBBBBBBBB"):
                packet = directory / f"{job_id}.json"
                packet.write_text(json.dumps({"job_id": job_id}), encoding="utf-8")
                jobs.append((job_id, packet))
            manifest = V2C.make_launch_manifest(
                self.plan, self.effective, self.repository, "render", jobs)
            self.assertEqual(manifest["launch_set_sha256"], V2C.digest(manifest["launch_set"]))
            for request in manifest["requests"]:
                request_body = {key: value for key, value in request.items()
                                if key != "request_digest"}
                self.assertEqual(request["launch_set_sha256"], manifest["launch_set_sha256"])
                self.assertEqual(request["request_digest"], V2C.digest(request_body))
                self.assertEqual(request["output_schema_sha256"], V2C.file_digest(schema))

    def test_launch_set_digest_changes_when_packet_bytes_change(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            packet = Path(temporary) / "packet.json"
            packet.write_text("first", encoding="utf-8")
            first = V2C.make_launch_manifest(
                self.plan, self.effective, self.repository, "render", [("XTEST", packet)])
            packet.write_text("second", encoding="utf-8")
            second = V2C.make_launch_manifest(
                self.plan, self.effective, self.repository, "render", [("XTEST", packet)])
            self.assertNotEqual(first["launch_set_sha256"], second["launch_set_sha256"])

    def test_emit_seals_final_bytes_by_refusing_rewrite(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            path = Path(temporary) / "packet.json"
            V2C.emit({"value": "final"}, path)
            before = hashlib.sha256(path.read_bytes()).hexdigest()
            with self.assertRaisesRegex(V2C.V2CError, "overwrite"):
                V2C.emit({"value": "mutated"}, path)
            self.assertEqual(before, hashlib.sha256(path.read_bytes()).hexdigest())

    def test_receipt_contract_scopes_visibility_to_staged_inputs(self) -> None:
        source = (ROOT / "scripts" / "sol_packet_job_v2c.py").read_text(encoding="utf-8")
        self.assertIn('"staged_input_files": ["packet.json", "output.schema.json"]', source)
        self.assertIn('"filesystem_visibility_claimed": False', source)
        self.assertNotIn('"visible_files"', source)

    def test_exact_once_transport_rule_is_inherited_unchanged(self) -> None:
        prose = " ".join(f"word{number}" for number in range(100))
        direct, direct_mode = V2C.V2B.normalize_transport({"text": prose})
        wrapped, wrapped_mode = V2C.V2B.normalize_transport(
            {"text": json.dumps({"text": prose})})
        self.assertEqual((direct, direct_mode), (prose, "direct"))
        self.assertEqual((wrapped, wrapped_mode), (prose, "unwrapped_once"))
        nested = json.dumps({"text": json.dumps({"text": prose})})
        with self.assertRaisesRegex(V2C.V2B.V2BError, "second nested"):
            V2C.V2B.normalize_transport({"text": nested})

    def test_retained_packet_digests_match_sealed_unblinding(self) -> None:
        run_dir = ROOT / "pilot" / "micro-render-v2c" / "runs"
        unblinding = V2C.load_json(run_dir / "sealed-unblinding.json")
        self.assertEqual(len(unblinding["samples"]), 16)
        for sample_id, record in unblinding["samples"].items():
            packet = run_dir / "samples" / sample_id / "render-packet.json"
            self.assertEqual(record["packet_artifact_sha256"], V2C.file_digest(packet))

    def test_retained_transport_report_reconstructs_from_combined_checkpoint(self) -> None:
        run_dir = ROOT / "pilot" / "micro-render-v2c" / "runs"
        retained = V2C.load_json(run_dir / "transport-gate-report.json")
        reconstructed = V2C.evaluate_transport(
            self.plan, self.effective, self.repository,
            retained["postbatch_checkpoint_commit_oid"], run_dir)
        self.assertEqual(reconstructed, retained)
        self.assertTrue(retained["gate_passed"])
        self.assertEqual(retained["accepted_jobs"], 16)

    def test_retained_semantic_and_matching_gates_are_complete_but_same_family(self) -> None:
        run_dir = ROOT / "pilot" / "micro-render-v2c" / "runs"
        semantic = V2C.load_json(run_dir / "semantic" / "semantic-gate-report.json")
        matching = V2C.load_json(run_dir / "matching" / "matching-report.json")
        self.assertEqual((semantic["accepted_samples"], semantic["required_samples"]), (16, 16))
        self.assertTrue(semantic["gate_passed"])
        self.assertTrue(matching["gate_passed"])
        self.assertEqual([item["correct_assignments"] for item in matching["condition_results"]],
                         [12, 12, 12, 12])
        self.assertEqual([item["correct_by_pass"] for item in matching["condition_results"]],
                         [[4, 4, 4]] * 4)
        self.assertTrue(any("same Sol model family" in item
                            for item in self.effective["limitations"]))


if __name__ == "__main__":
    unittest.main()
