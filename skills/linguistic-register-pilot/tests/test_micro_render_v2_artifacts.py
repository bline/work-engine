from __future__ import annotations

import importlib.util
import json
import shutil
import tempfile
import unittest
from pathlib import Path


ROOT = Path(__file__).parents[1]
SCRIPT = ROOT / "scripts" / "micro_render_v2_artifacts.py"
PLAN_PATH = ROOT / "pilot" / "micro-render-v2" / "preregistration.yaml"
SPEC = importlib.util.spec_from_file_location("micro_render_v2_artifacts", SCRIPT)
assert SPEC and SPEC.loader
V2 = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(V2)


class MicroRenderV2ArtifactsTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.plan, cls.repository = V2.validate_plan(V2.V1.BASE.load_yaml(PLAN_PATH), PLAN_PATH)

    def test_plan_and_source_feature_licenses_are_valid(self) -> None:
        self.assertEqual(self.plan["status"], "frozen")
        self.assertEqual(len(self.plan["candidate_styles"]), 3)
        for style in self.plan["candidate_styles"]:
            self.assertEqual(len(style["features"]), 5)

    def test_render_packets_are_deterministic_complete_and_identity_blind(self) -> None:
        first = V2.prepare_render_jobs(self.plan, PLAN_PATH, "a" * 40)
        second = V2.prepare_render_jobs(self.plan, PLAN_PATH, "a" * 40)
        self.assertEqual(first, second)
        packets, key = first
        self.assertEqual(len(packets), 16)
        self.assertEqual(len(key["samples"]), 16)
        for packet in packets.values():
            serialized = json.dumps(packet).lower()
            for forbidden in ("gelman", "leveson", "shaw", "condition_id", "source_feature_id"):
                self.assertNotIn(forbidden, serialized)

    def test_nested_json_and_bad_prose_shape_fail_before_normalization(self) -> None:
        nested = json.dumps({"text": " ".join(["word"] * 100)})
        with self.assertRaisesRegex(V2.V2Error, "serialized structured output"):
            V2.validate_prose(nested)
        with self.assertRaisesRegex(V2.V2Error, "physical paragraph"):
            V2.validate_prose("\n".join(["word"] * 100))
        with self.assertRaisesRegex(V2.V2Error, "90 through 130"):
            V2.validate_prose(" ".join(["word"] * 89))

    def test_semantic_normalization_requires_uniform_complete_decisions(self) -> None:
        packets, _ = V2.prepare_render_jobs(self.plan, PLAN_PATH, "a" * 40)
        items = []
        for sample_id, packet in sorted(packets.items()):
            items.append({"sample_id": sample_id, "semantic_brief": packet["semantic_brief"],
                          "rendered_text": "Fixture.", "render_artifact_sha256": "a" * 64})
        packet = {"experiment_id": self.plan["experiment_id"], "items": items}
        decisions = []
        for item in items:
            decisions.append({
                "sample_id": item["sample_id"], "verdict": "equivalent",
                "proposition_checks": [
                    {"proposition_id": proposition["id"], "status": "preserved", "evidence": "Fixture."}
                    for proposition in item["semantic_brief"]["required_propositions"]],
                "speech_act_equivalent": True, "added_meaning": [], "rationale": "Fixture.",
            })
        with tempfile.TemporaryDirectory() as temporary:
            receipt = Path(temporary) / "receipt.json"
            receipt.write_text("{}\n", encoding="utf-8")
            result = V2.normalize_semantic({"decisions": decisions, "method_limitations": []}, packet, receipt)
            self.assertEqual(len(result["decisions"]), 16)
            decisions[0]["added_meaning"] = ["Added."]
            with self.assertRaisesRegex(V2.V2Error, "conflicts"):
                V2.normalize_semantic({"decisions": decisions, "method_limitations": []}, packet, receipt)

    def test_matching_requires_every_text_once(self) -> None:
        packet = {
            "experiment_id": self.plan["experiment_id"], "pass_number": 1,
            "anonymous_texts": [{"text_id": f"T{index:02d}"} for index in range(1, 17)],
            "anonymous_references": [{"reference_id": f"R{index:02d}"} for index in range(1, 5)],
        }
        raw = {"assignments": [
            {"text_id": f"T{index:02d}", "reference_id": "R01", "confidence": 3, "rationale": "Fixture."}
            for index in range(1, 17)], "method_limitations": []}
        with tempfile.TemporaryDirectory() as temporary:
            receipt = Path(temporary) / "receipt.json"
            receipt.write_text("{}\n", encoding="utf-8")
            result = V2.normalize_matching(raw, packet, receipt)
            self.assertEqual(len(result["assignments"]), 16)
            raw["assignments"].pop()
            with self.assertRaisesRegex(V2.V2Error, "all sixteen"):
                V2.normalize_matching(raw, packet, receipt)

    def test_preoutcome_checkpoint_must_retain_exact_plan(self) -> None:
        subprocess = __import__("subprocess")
        with tempfile.TemporaryDirectory() as temporary:
            repository = Path(temporary)
            subprocess.run(["git", "init", "-q", str(repository)], check=True)
            subprocess.run(["git", "-C", str(repository), "config", "user.name", "Test User"], check=True)
            subprocess.run(
                ["git", "-C", str(repository), "config", "user.email", "test@example.com"],
                check=True,
            )
            sentinel = repository / "sentinel.txt"
            sentinel.write_text("checkpoint without plan\n", encoding="utf-8")
            subprocess.run(["git", "-C", str(repository), "add", "sentinel.txt"], check=True)
            subprocess.run(
                ["git", "-C", str(repository), "commit", "-q", "-m", "fixture checkpoint"],
                check=True,
            )
            head = subprocess.run(
                ["git", "-C", str(repository), "rev-parse", "HEAD"],
                check=True,
                stdout=subprocess.PIPE,
            ).stdout.decode().strip()
            plan_path = repository / "pilot" / "micro-render-v2" / "preregistration.yaml"
            plan_path.parent.mkdir(parents=True)
            shutil.copyfile(PLAN_PATH, plan_path)
            with self.assertRaisesRegex(V2.V2Error, "does not contain the plan"):
                V2.verify_preoutcome_checkpoint(repository, plan_path, head)

    def test_transport_stop_requires_an_observed_failure(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            run_dir = Path(temporary)
            packets, key = V2.prepare_render_jobs(self.plan, PLAN_PATH, "a" * 40)
            V2.emit(key, run_dir / "render-key.json")
            for sample_id, packet in packets.items():
                V2.emit(packet, run_dir / "samples" / sample_id / "render-packet.json")
            with self.assertRaisesRegex(V2.V2Error, "requires at least one"):
                V2.aggregate_transport_stop(self.plan, PLAN_PATH, run_dir)

    def retained_run_copy(self, temporary: str) -> Path:
        source = ROOT / "pilot" / "micro-render-v2" / "runs"
        target = Path(temporary) / "runs"
        shutil.copytree(source, target)
        return target

    def test_retained_transport_stop_reconstructs(self) -> None:
        run_dir = ROOT / "pilot" / "micro-render-v2" / "runs"
        report = V2.aggregate_transport_stop(self.plan, PLAN_PATH, run_dir)
        retained = V2.V1.BASE.load_json(
            ROOT / "pilot" / "micro-render-v2" / "transport-stop-report-qualified.json")
        self.assertEqual(report, retained)
        self.assertEqual(report["transport_valid_samples"], ["S03", "S04", "S08"])
        self.assertEqual([item["sample_id"] for item in report["transport_invalid_samples"]],
                         ["S01", "S02", "S05", "S06", "S07"])

    def test_transport_stop_rejects_incomplete_evidence_and_downstream_artifacts(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            run_dir = self.retained_run_copy(temporary)
            (run_dir / "samples" / "S01" / "render-events.jsonl").unlink()
            with self.assertRaisesRegex(V2.V2Error, "incomplete execution evidence"):
                V2.aggregate_transport_stop(self.plan, PLAN_PATH, run_dir)
        with tempfile.TemporaryDirectory() as temporary:
            run_dir = self.retained_run_copy(temporary)
            (run_dir / "semantic").mkdir()
            with self.assertRaisesRegex(V2.V2Error, "semantic artifacts exist"):
                V2.aggregate_transport_stop(self.plan, PLAN_PATH, run_dir)

    def test_transport_stop_rejects_receipt_tampering_and_missing_valid_render(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            run_dir = self.retained_run_copy(temporary)
            raw_path = run_dir / "samples" / "S01" / "raw-render.json"
            raw = json.loads(raw_path.read_text(encoding="utf-8"))
            raw["text"] += " tampered"
            raw_path.write_text(json.dumps(raw), encoding="utf-8")
            with self.assertRaisesRegex(V2.V2Error, "raw-output digest mismatch"):
                V2.aggregate_transport_stop(self.plan, PLAN_PATH, run_dir)
        with tempfile.TemporaryDirectory() as temporary:
            run_dir = self.retained_run_copy(temporary)
            (run_dir / "samples" / "S03" / "render.json").unlink()
            with self.assertRaisesRegex(V2.V2Error, "lacks normalized render"):
                V2.aggregate_transport_stop(self.plan, PLAN_PATH, run_dir)


if __name__ == "__main__":
    unittest.main()
