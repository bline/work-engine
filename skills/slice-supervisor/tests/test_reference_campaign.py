from __future__ import annotations

import importlib.util
import json
import sys
import tempfile
import unittest
from pathlib import Path


ROOT = Path(__file__).parents[1]
WORK_ENGINE_ROOT = Path(__file__).parents[3]


def load_module(name: str, path: Path):
    spec = importlib.util.spec_from_file_location(name, path)
    assert spec and spec.loader
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


FINALIZER = load_module(
    "reference_campaign_finalizer", ROOT / "scripts" / "finalize_receipt.py"
)
RESUME = load_module(
    "reference_campaign_resume", ROOT / "scripts" / "resume_campaign.py"
)
ASSEMBLER_TESTS = load_module(
    "reference_campaign_assembler", Path(__file__).with_name("test_assemble_receipt.py")
)
RUN_GATE = load_module(
    "reference_campaign_gate",
    WORK_ENGINE_ROOT / "skills" / "slice-builder" / "scripts" / "run_gate.py",
)


class ReferenceCampaignTest(unittest.TestCase):
    def accepted_inputs(
        self,
        slice_number: int,
        preflight: dict[str, object],
        predecessor: dict[str, object] | None,
        *,
        remaining: bool,
    ):
        semantic = ASSEMBLER_TESTS.ReceiptAssemblerTest().semantic_receipt()
        semantic.update(
            engine_config=json.loads(json.dumps(preflight["engineConfig"])),
            slice_number=slice_number,
            slice_title=f"Reference lifecycle slice {slice_number}",
            slice_goal=(
                "Establish the reference lifecycle"
                if predecessor is None
                else f"Continue from {predecessor['slice_title']}"
            ),
            outcome=(
                f"Reference lifecycle slice {slice_number} accepted"
                if remaining
                else "Reference lifecycle objective complete"
            ),
            status="accepted",
            stop_reason=None,
            plan_acceptance="procedural_auto_approval",
            validation_requirement_results={
                requirement: "passed"
                for requirement in preflight["engineConfig"]["validation"][
                    "requirements"
                ]
            },
            placement_certificate={"owner": "terminal audit receipt"},
            placement_verdict="confirmed",
            placement_risk="low",
            vertical_semantic_test="three-slice-reference-lifecycle",
            vertical_semantic_test_passed=True,
            more_in_scope_work_remains=remaining,
        )
        telemetry = ASSEMBLER_TESTS.ingress()
        telemetry["slice_id"] = str(slice_number)
        consumed = (
            ["Established the initial reference lifecycle state."]
            if predecessor is None
            else [
                "Consumed predecessor handoff: "
                f"{predecessor['slice_title']} — {predecessor['outcome']}"
            ]
        )
        handoff = {
            "slice_title": semantic["slice_title"],
            "slice_goal": semantic["slice_goal"],
            "outcome": semantic["outcome"],
            "durable_decisions": consumed,
            "affected_boundaries": ["multi-slice reference lifecycle"],
            "placement_certificate": semantic["placement_certificate"],
            "unresolved_concerns": [],
            "deferred_scope": ["non-reference lifecycle scenarios"],
        }
        return semantic, telemetry, preflight, handoff

    def test_recoverable_validation_failure_finalizes_once_after_repair(self) -> None:
        initial = ASSEMBLER_TESTS.ReceiptAssemblerTest().semantic_receipt()
        preflight = ASSEMBLER_TESTS.preflight_result(initial["engine_config"])

        with tempfile.TemporaryDirectory() as directory:
            working_directory = Path(directory)
            artifact = working_directory / "artifact.txt"
            receipts = working_directory / "receipts.jsonl"
            artifact.write_text("broken", encoding="utf-8")
            manifest = {
                "checks": [
                    {
                        "requirement": "semantic_proof",
                        "identity": "repaired-artifact",
                        "command": [
                            sys.executable,
                            "-c",
                            (
                                "from pathlib import Path; import sys; "
                                "raise SystemExit("
                                "0 if Path(sys.argv[1]).read_text() == 'repaired' else 1)"
                            ),
                            str(artifact),
                        ],
                    }
                ]
            }

            failed = RUN_GATE.run(manifest, working_directory)
            self.assertEqual("failed", failed["status"])
            self.assertEqual("repaired-artifact", failed["failed_check"])
            self.assertFalse(receipts.exists())

            artifact.write_text("repaired", encoding="utf-8")
            passed = RUN_GATE.run(manifest, working_directory)
            self.assertEqual("passed", passed["status"])
            self.assertIsNone(passed["failed_check"])
            self.assertFalse(receipts.exists())

            accepted = self.accepted_inputs(1, preflight, None, remaining=True)
            receipt = FINALIZER.finalize(receipts, *accepted)
            records = [json.loads(line) for line in receipts.read_text().splitlines()]

        self.assertEqual("accepted", receipt["status"])
        self.assertTrue(receipt["more_in_scope_work_remains"])
        self.assertEqual(1, receipt["slice_number"])
        self.assertEqual(1, len(records))
        self.assertEqual(("test-run", 1), (records[0]["run_id"], records[0]["slice_number"]))

    def test_three_slice_reference_campaign_consumes_handoffs_and_completes(self) -> None:
        initial = ASSEMBLER_TESTS.ReceiptAssemblerTest().semantic_receipt()
        preflight = ASSEMBLER_TESTS.preflight_result(initial["engine_config"])

        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "receipts.jsonl"

            first = self.accepted_inputs(1, preflight, None, remaining=True)
            FINALIZER.finalize(path, *first)
            before_resume = path.read_bytes()
            first_state = RESUME.resume(path, preflight, "test-run")
            self.assertEqual(before_resume, path.read_bytes())
            self.assertEqual(first[3], first_state["handoff_receipt"])
            self.assertEqual(2, first_state["next_slice_number"])

            second = self.accepted_inputs(
                2, preflight, first_state["handoff_receipt"], remaining=True
            )
            second_receipt = FINALIZER.finalize(path, *second)
            self.assertIn(
                first_state["handoff_receipt"]["slice_title"],
                second_receipt["slice_goal"],
            )
            self.assertIn(
                first_state["handoff_receipt"]["outcome"],
                second_receipt["continuation_context"]["durable_decisions"][0],
            )
            before_resume = path.read_bytes()
            second_state = RESUME.resume(path, preflight, "test-run")
            self.assertEqual(before_resume, path.read_bytes())
            self.assertEqual(second[3], second_state["handoff_receipt"])
            self.assertEqual(3, second_state["next_slice_number"])

            third = self.accepted_inputs(
                3, preflight, second_state["handoff_receipt"], remaining=False
            )
            third_receipt = FINALIZER.finalize(path, *third)
            self.assertIn(
                second_state["handoff_receipt"]["slice_title"],
                third_receipt["slice_goal"],
            )
            self.assertNotIn("continuation_context", third_receipt)

            before_resume = path.read_bytes()
            completed = RESUME.resume(path, preflight, "test-run")
            self.assertEqual(before_resume, path.read_bytes())
            self.assertFalse(completed["resumable"])
            self.assertEqual("objective_or_scope_complete", completed["reason"])
            self.assertIsNone(completed["next_slice_number"])
            self.assertIsNone(completed["handoff_receipt"])

            records = [json.loads(line) for line in path.read_text().splitlines()]

        self.assertEqual(
            [("test-run", 1), ("test-run", 2), ("test-run", 3)],
            [(record["run_id"], record["slice_number"]) for record in records],
        )

    def test_accepted_continuation_stops_truthfully_and_is_nonresumable(self) -> None:
        initial = ASSEMBLER_TESTS.ReceiptAssemblerTest().semantic_receipt()
        preflight = ASSEMBLER_TESTS.preflight_result(initial["engine_config"])

        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "receipts.jsonl"

            first = self.accepted_inputs(1, preflight, None, remaining=True)
            FINALIZER.finalize(path, *first)
            first_state = RESUME.resume(path, preflight, "test-run")

            stopped = self.accepted_inputs(
                2, preflight, first_state["handoff_receipt"], remaining=True
            )
            semantic, telemetry, campaign, handoff = stopped
            semantic.update(
                outcome="Reference lifecycle stopped for required human judgment",
                status="stopped",
                stop_reason="human_judgment: choose the authoritative continuation",
                plan_acceptance="not_reached",
                validation_requirement_results={
                    requirement: "blocked"
                    for requirement in preflight["engineConfig"]["validation"][
                        "requirements"
                    ]
                },
                vertical_semantic_test_passed=None,
            )
            handoff.update(
                outcome=semantic["outcome"],
                durable_decisions=[
                    "Consumed predecessor handoff before stopping: "
                    f"{first_state['handoff_receipt']['slice_title']} — "
                    f"{first_state['handoff_receipt']['outcome']}"
                ],
                unresolved_concerns=["Authoritative continuation requires human judgment."],
            )

            receipt = FINALIZER.finalize(
                path, semantic, telemetry, campaign, handoff
            )
            self.assertEqual("stopped", receipt["status"])
            self.assertEqual(
                "human_judgment: choose the authoritative continuation",
                receipt["stop_reason"],
            )
            self.assertTrue(receipt["more_in_scope_work_remains"])
            self.assertIn(
                first_state["handoff_receipt"]["slice_title"], receipt["slice_goal"]
            )
            self.assertNotIn("continuation_context", receipt)

            before_resume = path.read_bytes()
            stopped_state = RESUME.resume(path, preflight, "test-run")
            self.assertEqual(before_resume, path.read_bytes())
            self.assertFalse(stopped_state["resumable"])
            self.assertEqual("stopped", stopped_state["reason"])
            self.assertEqual(
                {
                    "status": receipt["status"],
                    "stop_reason": receipt["stop_reason"],
                },
                stopped_state["terminal_state"],
            )
            self.assertIsNone(stopped_state["next_slice_number"])
            self.assertIsNone(stopped_state["handoff_receipt"])

            records = [json.loads(line) for line in path.read_text().splitlines()]

        self.assertEqual(
            [("test-run", 1), ("test-run", 2)],
            [(record["run_id"], record["slice_number"]) for record in records],
        )
        self.assertEqual("accepted", records[0]["status"])
        self.assertEqual("stopped", records[1]["status"])

    def test_accepted_continuation_fails_truthfully_and_is_nonresumable(self) -> None:
        initial = ASSEMBLER_TESTS.ReceiptAssemblerTest().semantic_receipt()
        preflight = ASSEMBLER_TESTS.preflight_result(initial["engine_config"])

        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "receipts.jsonl"

            first = self.accepted_inputs(1, preflight, None, remaining=True)
            FINALIZER.finalize(path, *first)
            first_state = RESUME.resume(path, preflight, "test-run")

            failed = self.accepted_inputs(
                2, preflight, first_state["handoff_receipt"], remaining=True
            )
            semantic, telemetry, campaign, handoff = failed
            semantic.update(
                outcome="Reference lifecycle failed after an unrecoverable product failure",
                status="failed",
                stop_reason="product_failure: reference lifecycle cannot continue",
                validation_requirement_results={
                    requirement: "blocked"
                    for requirement in preflight["engineConfig"]["validation"][
                        "requirements"
                    ]
                },
                vertical_semantic_test_passed=None,
            )
            handoff.update(
                outcome=semantic["outcome"],
                durable_decisions=[
                    "Consumed predecessor handoff before failing: "
                    f"{first_state['handoff_receipt']['slice_title']} — "
                    f"{first_state['handoff_receipt']['outcome']}"
                ],
                unresolved_concerns=["The product failure is unrecoverable."],
            )

            receipt = FINALIZER.finalize(
                path, semantic, telemetry, campaign, handoff
            )
            self.assertEqual("failed", receipt["status"])
            self.assertEqual(
                "product_failure: reference lifecycle cannot continue",
                receipt["stop_reason"],
            )
            self.assertTrue(receipt["more_in_scope_work_remains"])
            self.assertIn(
                first_state["handoff_receipt"]["slice_title"], receipt["slice_goal"]
            )
            self.assertIn(
                first_state["handoff_receipt"]["outcome"],
                handoff["durable_decisions"][0],
            )
            self.assertNotIn("continuation_context", receipt)

            before_resume = path.read_bytes()
            failed_state = RESUME.resume(path, preflight, "test-run")
            self.assertEqual(before_resume, path.read_bytes())
            self.assertFalse(failed_state["resumable"])
            self.assertEqual("failed", failed_state["reason"])
            self.assertEqual(
                {
                    "status": receipt["status"],
                    "stop_reason": receipt["stop_reason"],
                },
                failed_state["terminal_state"],
            )
            self.assertIsNone(failed_state["next_slice_number"])
            self.assertIsNone(failed_state["handoff_receipt"])

            records = [json.loads(line) for line in path.read_text().splitlines()]

        self.assertEqual(
            [("test-run", 1), ("test-run", 2)],
            [(record["run_id"], record["slice_number"]) for record in records],
        )
        self.assertEqual("accepted", records[0]["status"])
        self.assertEqual("failed", records[1]["status"])

    def test_accepted_continuation_reopens_route_and_completes(self) -> None:
        initial = ASSEMBLER_TESTS.ReceiptAssemblerTest().semantic_receipt()
        preflight = ASSEMBLER_TESTS.preflight_result(initial["engine_config"])

        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "receipts.jsonl"

            first = self.accepted_inputs(1, preflight, None, remaining=True)
            FINALIZER.finalize(path, *first)
            before_resume = path.read_bytes()
            first_state = RESUME.resume(path, preflight, "test-run")
            self.assertEqual(before_resume, path.read_bytes())

            revised = self.accepted_inputs(
                2, preflight, first_state["handoff_receipt"], remaining=True
            )
            semantic, telemetry, campaign, handoff = revised
            route_revision = {
                "failed_premise": "The accepted boundary still fits the new evidence.",
                "stale_decisions": ["Continue within the accepted boundary."],
                "preserved_evidence": [
                    "The predecessor handoff remains valid continuation context."
                ],
                "replacement_route": "evidence-guided-replan",
                "reason": "New evidence invalidated the accepted boundary.",
            }
            semantic["worker_metrics"].update(
                workflow_route="evidence-guided-replan",
                route_revisions=[route_revision],
            )
            handoff["durable_decisions"].append(
                "Retired the stale accepted boundary while preserving predecessor evidence."
            )

            revised_receipt = FINALIZER.finalize(
                path, semantic, telemetry, campaign, handoff
            )
            self.assertEqual(
                [route_revision], revised_receipt["worker_metrics"]["route_revisions"]
            )
            self.assertEqual(
                "evidence-guided-replan",
                revised_receipt["worker_metrics"]["workflow_route"],
            )
            self.assertNotIn(
                "route_revisions", revised_receipt["continuation_context"]
            )
            self.assertNotIn(
                "workflow_route", revised_receipt["continuation_context"]
            )
            self.assertIn(
                first_state["handoff_receipt"]["slice_title"],
                revised_receipt["slice_goal"],
            )

            before_resume = path.read_bytes()
            revised_state = RESUME.resume(path, preflight, "test-run")
            self.assertEqual(before_resume, path.read_bytes())
            self.assertEqual(handoff, revised_state["handoff_receipt"])
            self.assertNotIn("route_revisions", revised_state["handoff_receipt"])
            self.assertNotIn("workflow_route", revised_state["handoff_receipt"])

            completed = self.accepted_inputs(
                3, preflight, revised_state["handoff_receipt"], remaining=False
            )
            completed_receipt = FINALIZER.finalize(path, *completed)
            self.assertIn(
                revised_state["handoff_receipt"]["slice_title"],
                completed_receipt["slice_goal"],
            )
            self.assertIn(
                revised_state["handoff_receipt"]["outcome"],
                completed[3]["durable_decisions"][0],
            )

            before_resume = path.read_bytes()
            completed_state = RESUME.resume(path, preflight, "test-run")
            self.assertEqual(before_resume, path.read_bytes())
            self.assertFalse(completed_state["resumable"])
            self.assertEqual(
                "objective_or_scope_complete", completed_state["reason"]
            )
            self.assertIsNone(completed_state["next_slice_number"])
            self.assertIsNone(completed_state["handoff_receipt"])

            records = [json.loads(line) for line in path.read_text().splitlines()]

        self.assertEqual(
            [("test-run", 1), ("test-run", 2), ("test-run", 3)],
            [(record["run_id"], record["slice_number"]) for record in records],
        )
        self.assertEqual(
            [route_revision], records[1]["worker_metrics"]["route_revisions"]
        )
        self.assertNotIn("continuation_context", records[2])


if __name__ == "__main__":
    unittest.main()
