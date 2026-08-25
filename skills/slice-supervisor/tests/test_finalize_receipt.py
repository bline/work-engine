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


FINALIZER = load_module("finalize_receipt", ROOT / "scripts" / "finalize_receipt.py")
ASSEMBLER_TESTS = load_module(
    "assemble_receipt_tests_fixture", Path(__file__).with_name("test_assemble_receipt.py")
)


class FinalizeReceiptTest(unittest.TestCase):
    def handoff(self, semantic):
        return {
            "slice_title": semantic["slice_title"],
            "slice_goal": semantic["slice_goal"],
            "outcome": semantic["outcome"],
            "durable_decisions": ["Keep one terminal record owner."],
            "affected_boundaries": ["slice-supervisor"],
            "placement_certificate": semantic["placement_certificate"],
            "unresolved_concerns": [],
            "deferred_scope": ["mid-slice recovery"],
        }

    def inputs(self):
        semantic = ASSEMBLER_TESTS.ReceiptAssemblerTest().semantic_receipt()
        semantic["engine_config"] = dict(semantic["engine_config"])
        semantic["engine_config"]["objective"] = "model-transcribed objective"
        authoritative_config = dict(semantic["engine_config"])
        authoritative_config["objective"] = "authoritative campaign objective"
        return (
            semantic,
            ASSEMBLER_TESTS.ingress(),
            ASSEMBLER_TESTS.preflight_result(authoritative_config),
        )

    def test_finalizes_authoritative_receipt_once(self) -> None:
        semantic, telemetry, preflight = self.inputs()
        with tempfile.TemporaryDirectory() as directory:
            destination = Path(directory) / "metrics" / "receipts.jsonl"
            finalized = FINALIZER.finalize(
                destination, semantic, telemetry, preflight, self.handoff(semantic)
            )

            records = [json.loads(line) for line in destination.read_text().splitlines()]
            self.assertEqual([finalized], records)
            self.assertEqual(5, records[0]["schema_version"])
            self.assertEqual(
                "authoritative campaign objective", records[0]["engine_config"]["objective"]
            )
            self.assertEqual(120, records[0]["worker_metrics"]["builder_input_tokens"])
            self.assertIsNone(records[0]["worker_metrics"]["builder_context_usage"])
            self.assertEqual(
                preflight["campaignSource"],
                records[0]["producer_metrics"]["campaign_source"],
            )
            self.assertEqual(
                "child-thread",
                records[0]["producer_metrics"]["telemetry_ingress"]["source_identity"]["child_thread_id"],
            )
            self.assertNotIn("continuation_context", records[0])

            before_duplicate = destination.read_bytes()
            with self.assertRaisesRegex(ValueError, "already exists"):
                FINALIZER.finalize(
                    destination, semantic, telemetry, preflight, self.handoff(semantic)
                )
            self.assertEqual(before_duplicate, destination.read_bytes())

    def test_assembly_failure_does_not_change_destination(self) -> None:
        semantic, telemetry, preflight = self.inputs()
        telemetry["run_id"] = "different-run"
        with tempfile.TemporaryDirectory() as directory:
            destination = Path(directory) / "receipts.jsonl"
            seed = b'{"existing":"user-state"}\n'
            destination.write_bytes(seed)
            with self.assertRaisesRegex(ValueError, "run_id"):
                FINALIZER.finalize(
                    destination, semantic, telemetry, preflight, self.handoff(semantic)
                )
            self.assertEqual(seed, destination.read_bytes())

    def test_resumable_acceptance_projects_only_continuation_fields(self) -> None:
        semantic, telemetry, preflight = self.inputs()
        semantic.update(
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
        handoff = self.handoff(semantic)
        with tempfile.TemporaryDirectory() as directory:
            finalized = FINALIZER.finalize(
                Path(directory) / "receipts.jsonl",
                semantic,
                telemetry,
                preflight,
                handoff,
            )
        self.assertEqual(
            {
                "schema_version": 1,
                "durable_decisions": handoff["durable_decisions"],
                "affected_boundaries": handoff["affected_boundaries"],
                "unresolved_concerns": handoff["unresolved_concerns"],
                "deferred_scope": handoff["deferred_scope"],
            },
            finalized["continuation_context"],
        )


if __name__ == "__main__":
    unittest.main()
