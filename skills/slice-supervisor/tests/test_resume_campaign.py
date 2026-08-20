from __future__ import annotations

import importlib.util
import json
import tempfile
import unittest
from pathlib import Path


ROOT = Path(__file__).parents[1]


def load_module(name: str, path: Path):
    spec = importlib.util.spec_from_file_location(name, path)
    assert spec and spec.loader
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


FINALIZER = load_module("resume_fixture_finalizer", ROOT / "scripts" / "finalize_receipt.py")
RESUME = load_module("resume_campaign", ROOT / "scripts" / "resume_campaign.py")
ASSEMBLER_TESTS = load_module(
    "resume_fixture_assembler", Path(__file__).with_name("test_assemble_receipt.py")
)


class ResumeCampaignTest(unittest.TestCase):
    def accepted_inputs(self, slice_number: int):
        semantic = ASSEMBLER_TESTS.ReceiptAssemblerTest().semantic_receipt()
        semantic.update(
            slice_number=slice_number,
            status="accepted",
            stop_reason=None,
            plan_acceptance="procedural_auto_approval",
            validation_requirement_results={"focused_checks": "passed"},
            placement_certificate={"owner": "terminal audit receipt"},
            placement_verdict="confirmed",
            placement_risk="medium",
            vertical_semantic_test="two-slice recovery",
            vertical_semantic_test_passed=True,
            more_in_scope_work_remains=True,
        )
        telemetry = ASSEMBLER_TESTS.ingress()
        telemetry["slice_id"] = str(slice_number)
        preflight = ASSEMBLER_TESTS.preflight_result(semantic["engine_config"])
        handoff = {
            "slice_title": semantic["slice_title"],
            "slice_goal": semantic["slice_goal"],
            "outcome": semantic["outcome"],
            "durable_decisions": [f"decision-{slice_number}"],
            "affected_boundaries": [f"boundary-{slice_number}"],
            "placement_certificate": semantic["placement_certificate"],
            "unresolved_concerns": [f"concern-{slice_number}"],
            "deferred_scope": [f"deferred-{slice_number}"],
        }
        return semantic, telemetry, preflight, handoff

    def test_resumes_two_accepted_slices_with_latest_exact_handoff(self) -> None:
        first = self.accepted_inputs(1)
        second = self.accepted_inputs(2)
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "receipts.jsonl"
            FINALIZER.finalize(path, *first)
            FINALIZER.finalize(path, *second)
            before = path.read_bytes()
            state = RESUME.resume(path, second[2], "test-run")
            self.assertEqual(before, path.read_bytes())

        self.assertTrue(state["resumable"])
        self.assertEqual("accepted_continuation", state["reason"])
        self.assertEqual("test-run", state["run_id"])
        self.assertEqual(2, state["last_slice_number"])
        self.assertEqual(3, state["next_slice_number"])
        self.assertEqual(second[3], state["handoff_receipt"])

    def test_trusted_terminal_states_are_structured_nonresumable(self) -> None:
        semantic, telemetry, preflight, handoff = self.accepted_inputs(1)
        semantic["more_in_scope_work_remains"] = False
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "receipts.jsonl"
            FINALIZER.finalize(path, semantic, telemetry, preflight, handoff)
            state = RESUME.resume(path, preflight, "test-run")
        self.assertEqual("objective_or_scope_complete", state["reason"])
        self.assertFalse(state["resumable"])
        self.assertIsNone(state["next_slice_number"])
        self.assertIsNone(state["handoff_receipt"])

    def test_binding_mismatch_fails(self) -> None:
        inputs = self.accepted_inputs(1)
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "receipts.jsonl"
            FINALIZER.finalize(path, *inputs)
            mismatched = json.loads(json.dumps(inputs[2]))
            mismatched["engineConfig"]["objective"] = "different"
            with self.assertRaisesRegex(ValueError, "engineConfig"):
                RESUME.resume(path, mismatched, "test-run")


if __name__ == "__main__":
    unittest.main()
