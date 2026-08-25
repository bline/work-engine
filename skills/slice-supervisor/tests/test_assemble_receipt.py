from __future__ import annotations

import importlib.util
import unittest
from pathlib import Path


ROOT = Path(__file__).parents[1]


def load_module(name: str, path: Path):
    spec = importlib.util.spec_from_file_location(name, path)
    assert spec and spec.loader
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


ASSEMBLER = load_module("assemble_receipt", ROOT / "scripts" / "assemble_receipt.py")
APPEND_METRICS = load_module("append_metrics_fixture", ROOT / "scripts" / "append_metrics.py")
APPEND_TESTS = load_module("append_metrics_tests_fixture", Path(__file__).with_name("test_append_metrics.py"))


def provenance(ordinal: int, event_type: str) -> dict[str, object]:
    return {
        "artifact_role": "child_rollout",
        "ordinal": ordinal,
        "event_type": event_type,
        "timestamp": "2026-08-18T18:00:04Z",
    }


def measurement(value: int | float | None, ordinal: int = 3) -> dict[str, object]:
    return {
        "availability": "observed" if value is not None else "unavailable",
        "value": value,
        "provenance": [provenance(ordinal, "token_count")] if value is not None else [],
    }


def ingress() -> dict[str, object]:
    return {
        "schema_version": 1,
        "run_id": "test-run",
        "slice_id": "1",
        "builder_runtime": {
            "source_kind": "codex_rollout_jsonl",
            "source_identity": {
                "parent_thread_id": "parent-thread",
                "child_thread_id": "child-thread",
                "agent_path": "/root/builder",
                "terminal_turn_id": "child-turn",
            },
            "artifacts": {"parent": "/rollouts/parent", "child": "/rollouts/child"},
            "binding_provenance": {
                "parent_launch": {
                    "artifact_role": "parent_rollout",
                    "ordinal": 2,
                    "event_type": "item_completed",
                    "timestamp": "2026-08-18T18:00:01Z",
                },
                "child_identity": provenance(1, "session_meta"),
                "terminal_event": provenance(4, "task_complete"),
            },
            "measurements": {
                "input_tokens": measurement(120),
                "cached_input_tokens": measurement(20),
                "output_tokens": measurement(30),
                "reasoning_tokens": measurement(7),
                "peak_context_tokens": measurement(None),
                "turn_count": measurement(2, 4),
                "wall_clock_seconds": measurement(3.5, 4),
            },
        },
        "tool_runtime": [],
    }


def preflight_result(engine_config: dict[str, object]) -> dict[str, object]:
    engine_config["source"] = "campaigns/roadmap.yaml"
    return {
        "campaignPath": "/repo/campaigns/roadmap.yaml",
        "engineConfig": engine_config,
        "campaignSource": {
            "source": "file",
            "authoredReference": "campaigns/roadmap.yaml",
            "resolvedPath": "/repo/campaigns/roadmap.yaml",
            "pathBase": "/repo/campaigns",
            "sha256": "a" * 64,
            "schemaVersion": 1,
        },
        "resolvedCapabilities": {},
    }


class ReceiptAssemblerTest(unittest.TestCase):
    def semantic_receipt(self) -> dict[str, object]:
        receipt = APPEND_TESTS.base_record(5)
        receipt["worker_metrics"].update(
            {
                "builder_input_tokens": 999,
                "builder_output_tokens": 999,
                "builder_context_usage": 999,
                "builder_turn_count": 999,
                "builder_wall_clock_seconds": 999,
            }
        )
        return receipt

    def test_observed_runtime_telemetry_overrides_semantic_self_report(self) -> None:
        semantic = self.semantic_receipt()
        authoritative_config = semantic["engine_config"].copy()
        telemetry = ingress()
        telemetry["builder_runtime"]["binding_provenance"]["interrupted_turns"] = [
            {
                "turn_id": "interrupted-turn",
                "state": "interrupted",
                "start_event": provenance(2, "task_started"),
                "completion_availability": "unavailable",
                "completion_provenance": [],
            }
        ]
        assembled = ASSEMBLER.assemble(
            semantic, telemetry, preflight_result(authoritative_config)
        )

        metrics = assembled["worker_metrics"]
        self.assertEqual(120, metrics["builder_input_tokens"])
        self.assertEqual(30, metrics["builder_output_tokens"])
        self.assertIsNone(metrics["builder_context_usage"])
        self.assertEqual(2, metrics["builder_turn_count"])
        self.assertEqual(3.5, metrics["builder_wall_clock_seconds"])
        authoritative = assembled["producer_metrics"]["telemetry_ingress"]
        self.assertEqual(20, authoritative["measurements"]["cached_input_tokens"]["value"])
        self.assertEqual(7, authoritative["measurements"]["reasoning_tokens"]["value"])
        self.assertEqual("child-thread", authoritative["source_identity"]["child_thread_id"])
        self.assertEqual("/rollouts/child", authoritative["artifacts"]["child"])
        self.assertEqual(
            "interrupted-turn",
            authoritative["binding_provenance"]["interrupted_turns"][0]["turn_id"],
        )
        self.assertNotIn("tool_runtime", authoritative)
        APPEND_METRICS.validate(assembled)

        self.assertEqual(999, semantic["worker_metrics"]["builder_input_tokens"])
        self.assertNotIn("telemetry_ingress", semantic["producer_metrics"])

    def test_preflight_projects_authorized_same_model_review_provenance(self) -> None:
        semantic = self.semantic_receipt()
        APPEND_TESTS.make_same_model_review_v2(semantic)
        authoritative_config = semantic["engine_config"].copy()
        authoritative_config["amendments"] = [
            {
                "timestamp": "2026-08-21T00:00:00-06:00",
                "changed_fields": ["builder.context.adversarial_review"],
                "prior_values": {"review": "claude"},
                "new_values": {"review": "codex"},
                "reason": "Claude quota unavailable; same-model review approved.",
                "human_approval": "I approve Sol low as accepted same-model review.",
            }
        ]
        assembled = ASSEMBLER.assemble(
            semantic, ingress(), preflight_result(authoritative_config)
        )
        review = assembled["worker_metrics"]["adversarial_review_identity"]
        self.assertEqual(review["evidence_class"], "accepted_same_model_review")
        self.assertEqual(review["model"], "gpt-5.6-sol")
        self.assertEqual(review["reasoning_effort"], "low")
        self.assertFalse(review["independence_claimed"])
        self.assertEqual(len(assembled["engine_config"]["amendments"]), 1)
        APPEND_METRICS.validate(assembled)

    def test_preflight_configuration_overrides_semantic_transcription(self) -> None:
        semantic = self.semantic_receipt()
        semantic["engine_config"] = dict(semantic["engine_config"])
        semantic["engine_config"]["objective"] = "model-transcribed objective"
        authoritative_config = dict(semantic["engine_config"])
        authoritative_config["objective"] = "authoritative campaign objective"

        preflight = preflight_result(authoritative_config)
        assembled = ASSEMBLER.assemble(semantic, ingress(), preflight)

        self.assertEqual(authoritative_config, assembled["engine_config"])
        self.assertEqual(
            preflight["campaignSource"],
            assembled["producer_metrics"]["campaign_source"],
        )
        self.assertEqual(
            "model-transcribed objective", semantic["engine_config"]["objective"]
        )
        APPEND_METRICS.validate(assembled)

    def test_unavailable_measurements_replace_self_report_with_null(self) -> None:
        telemetry = ingress()
        for name in ("input_tokens", "output_tokens", "peak_context_tokens"):
            telemetry["builder_runtime"]["measurements"][name] = measurement(None)
        semantic = self.semantic_receipt()
        assembled = ASSEMBLER.assemble(
            semantic, telemetry, preflight_result(semantic["engine_config"])
        )
        self.assertIsNone(assembled["worker_metrics"]["builder_input_tokens"])
        self.assertIsNone(assembled["worker_metrics"]["builder_output_tokens"])
        self.assertIsNone(assembled["worker_metrics"]["builder_context_usage"])

    def test_identity_mismatch_fails_closed(self) -> None:
        telemetry = ingress()
        telemetry["run_id"] = "different-run"
        with self.assertRaisesRegex(ASSEMBLER.ReceiptAssemblyError, "run_id"):
            semantic = self.semantic_receipt()
            ASSEMBLER.assemble(
                semantic, telemetry, preflight_result(semantic["engine_config"])
            )

    def test_slice_identity_mismatch_fails_closed(self) -> None:
        telemetry = ingress()
        telemetry["slice_id"] = "2"
        with self.assertRaisesRegex(ASSEMBLER.ReceiptAssemblyError, "slice_id"):
            semantic = self.semantic_receipt()
            ASSEMBLER.assemble(
                semantic, telemetry, preflight_result(semantic["engine_config"])
            )

    def test_observed_zero_remains_zero(self) -> None:
        telemetry = ingress()
        telemetry["builder_runtime"]["measurements"]["input_tokens"] = measurement(0)
        semantic = self.semantic_receipt()
        assembled = ASSEMBLER.assemble(
            semantic, telemetry, preflight_result(semantic["engine_config"])
        )
        self.assertEqual(0, assembled["worker_metrics"]["builder_input_tokens"])
        self.assertIsNotNone(assembled["worker_metrics"]["builder_input_tokens"])

    def test_already_assembled_receipt_fails_closed(self) -> None:
        semantic = self.semantic_receipt()
        assembled = ASSEMBLER.assemble(
            semantic, ingress(), preflight_result(semantic["engine_config"])
        )
        with self.assertRaisesRegex(ASSEMBLER.ReceiptAssemblyError, "already contains"):
            ASSEMBLER.assemble(
                assembled, ingress(), preflight_result(semantic["engine_config"])
            )

    def test_malformed_measurement_fails_closed(self) -> None:
        telemetry = ingress()
        telemetry["builder_runtime"]["measurements"]["input_tokens"] = {
            "availability": "unavailable",
            "value": 4,
            "provenance": [],
        }
        with self.assertRaisesRegex(ASSEMBLER.ReceiptAssemblyError, "value"):
            semantic = self.semantic_receipt()
            ASSEMBLER.assemble(
                semantic, telemetry, preflight_result(semantic["engine_config"])
            )

    def test_malformed_campaign_source_fails_closed(self) -> None:
        semantic = self.semantic_receipt()
        preflight = preflight_result(semantic["engine_config"])
        preflight["campaignSource"]["sha256"] = "copied-not-a-digest"
        with self.assertRaisesRegex(ASSEMBLER.ReceiptAssemblyError, "sha256"):
            ASSEMBLER.assemble(semantic, ingress(), preflight)


if __name__ == "__main__":
    unittest.main()
