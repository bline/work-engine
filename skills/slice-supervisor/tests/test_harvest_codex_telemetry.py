from __future__ import annotations

import copy
import importlib.util
import json
import tempfile
import unittest
from pathlib import Path


ROOT = Path(__file__).parents[1]
FIXTURES = Path(__file__).parent / "fixtures"
SCRIPT = ROOT / "scripts" / "harvest_codex_telemetry.py"
SPEC = importlib.util.spec_from_file_location("harvest_codex_telemetry", SCRIPT)
assert SPEC and SPEC.loader
HARVESTER = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(HARVESTER)

PARENT = FIXTURES / "codex-rollout-parent.jsonl"
COMPLETE = FIXTURES / "codex-rollout-child-complete.jsonl"
INCOMPLETE = FIXTURES / "codex-rollout-child-incomplete.jsonl"


def records(path: Path) -> list[dict[str, object]]:
    return [json.loads(line) for line in path.read_text(encoding="utf-8").splitlines()]


def write_records(directory: Path, name: str, values: list[dict[str, object]]) -> Path:
    path = directory / name
    path.write_text("".join(json.dumps(value) + "\n" for value in values), encoding="utf-8")
    return path


class TelemetryHarvesterTest(unittest.TestCase):
    def harvest(self, children: list[Path] | None = None, agent_path: str = "/root/telemetry_builder"):
        return HARVESTER.harvest(
            run_id="run-fixture",
            slice_id="slice-1",
            agent_path=agent_path,
            parent_rollout=PARENT,
            child_rollouts=children if children is not None else [COMPLETE],
        )

    def test_completed_rollout_produces_bound_ingress(self) -> None:
        ingress = self.harvest()

        self.assertEqual(1, ingress["schema_version"])
        identity = ingress["builder_runtime"]["source_identity"]
        self.assertEqual("parent-thread", identity["parent_thread_id"])
        self.assertEqual("child-complete", identity["child_thread_id"])
        self.assertEqual("child-turn", identity["terminal_turn_id"])

        measurements = ingress["builder_runtime"]["measurements"]
        self.assertEqual(120, measurements["input_tokens"]["value"])
        self.assertEqual(1, measurements["turn_count"]["value"])
        self.assertEqual(3, measurements["wall_clock_seconds"]["value"])
        self.assertEqual("unavailable", measurements["peak_context_tokens"]["availability"])
        self.assertIsNone(measurements["peak_context_tokens"]["value"])
        self.assertEqual([], measurements["peak_context_tokens"]["provenance"])
        self.assertEqual(3, measurements["input_tokens"]["provenance"][0]["ordinal"])
        self.assertEqual(4, ingress["builder_runtime"]["binding_provenance"]["terminal_event"]["ordinal"])
        self.assertEqual([], ingress["builder_runtime"]["binding_provenance"]["interrupted_turns"])

    def test_interrupted_intermediate_turn_preserves_terminal_truth(self) -> None:
        child = records(COMPLETE)
        child.insert(
            1,
            {
                "timestamp": "2026-08-18T18:00:01Z",
                "type": "event_msg",
                "payload": {"type": "task_started", "turn_id": "interrupted-turn", "started_at": 90},
            },
        )
        with tempfile.TemporaryDirectory() as temporary:
            path = write_records(Path(temporary), "interrupted.jsonl", child)
            ingress = self.harvest([path])

        runtime = ingress["builder_runtime"]
        self.assertEqual("child-turn", runtime["source_identity"]["terminal_turn_id"])
        interrupted = runtime["binding_provenance"]["interrupted_turns"]
        self.assertEqual(1, len(interrupted))
        self.assertEqual("interrupted-turn", interrupted[0]["turn_id"])
        self.assertEqual("interrupted", interrupted[0]["state"])
        self.assertEqual("unavailable", interrupted[0]["completion_availability"])
        self.assertEqual([], interrupted[0]["completion_provenance"])
        measurements = runtime["measurements"]
        self.assertEqual(2, measurements["turn_count"]["value"])
        self.assertEqual(2, len(measurements["turn_count"]["provenance"]))
        self.assertEqual(13, measurements["wall_clock_seconds"]["value"])
        self.assertEqual(2, len(measurements["wall_clock_seconds"]["provenance"]))
        self.assertEqual(120, measurements["input_tokens"]["value"])

    def test_duplicate_and_unknown_turn_events_fail_closed(self) -> None:
        for name, mutate, message in (
            (
                "duplicate-start",
                lambda child: child.insert(2, copy.deepcopy(child[1])),
                "duplicate turn ids",
            ),
            (
                "unknown-completion",
                lambda child: child[-1]["payload"].update(turn_id="unknown-turn"),
                "unknown turn",
            ),
        ):
            with self.subTest(name=name), tempfile.TemporaryDirectory() as temporary:
                child = records(COMPLETE)
                mutate(child)
                path = write_records(Path(temporary), f"{name}.jsonl", child)
                with self.assertRaisesRegex(HARVESTER.TelemetryIngressError, message):
                    self.harvest([path])

    def test_trailing_event_after_latest_completion_fails_closed(self) -> None:
        child = records(COMPLETE)
        child.append(copy.deepcopy(child[2]))
        with tempfile.TemporaryDirectory() as temporary:
            path = write_records(Path(temporary), "trailing.jsonl", child)
            with self.assertRaisesRegex(HARVESTER.TelemetryIngressError, "does not end"):
                self.harvest([path])

    def test_missing_child_fails_closed(self) -> None:
        with self.assertRaisesRegex(HARVESTER.TelemetryIngressError, "found 0"):
            self.harvest([])

    def test_duplicate_child_fails_closed_as_ambiguous(self) -> None:
        with self.assertRaisesRegex(HARVESTER.TelemetryIngressError, "found 2"):
            self.harvest([COMPLETE, COMPLETE])

    def test_incomplete_child_fails_closed(self) -> None:
        with self.assertRaisesRegex(HARVESTER.TelemetryIngressError, "incomplete"):
            self.harvest([INCOMPLETE], "/root/incomplete_builder")

    def test_mismatched_child_identity_fails_closed(self) -> None:
        child = copy.deepcopy(records(COMPLETE))
        child[0]["payload"]["parent_thread_id"] = "different-parent"
        with tempfile.TemporaryDirectory() as temporary:
            path = write_records(Path(temporary), "mismatch.jsonl", child)
            with self.assertRaisesRegex(HARVESTER.TelemetryIngressError, "does not corroborate"):
                self.harvest([path])

    def test_missing_session_identity_fails_closed(self) -> None:
        child = copy.deepcopy(records(COMPLETE))
        child[0]["payload"].pop("id")
        with tempfile.TemporaryDirectory() as temporary:
            path = write_records(Path(temporary), "missing-id.jsonl", child)
            with self.assertRaisesRegex(HARVESTER.TelemetryIngressError, "nonempty id"):
                self.harvest([path])

    def test_nonmonotonic_wall_clock_fails_closed(self) -> None:
        child = copy.deepcopy(records(COMPLETE))
        child[-1]["payload"]["completed_at"] = 99
        with tempfile.TemporaryDirectory() as temporary:
            path = write_records(Path(temporary), "reversed-clock.jsonl", child)
            with self.assertRaisesRegex(HARVESTER.TelemetryIngressError, "wall-clock"):
                self.harvest([path])

    def test_nonfinite_rollout_number_fails_closed(self) -> None:
        child = copy.deepcopy(records(COMPLETE))
        child[-1]["payload"]["completed_at"] = float("nan")
        with tempfile.TemporaryDirectory() as temporary:
            path = write_records(Path(temporary), "nan-clock.jsonl", child)
            with self.assertRaisesRegex(HARVESTER.TelemetryIngressError, "non-finite"):
                self.harvest([path])

    def test_agent_path_alone_cannot_bind_interacted_event(self) -> None:
        parent = records(PARENT)
        parent[1]["payload"]["item"]["kind"] = "interacted"
        with tempfile.TemporaryDirectory() as temporary:
            path = write_records(Path(temporary), "parent.jsonl", parent)
            with self.assertRaisesRegex(HARVESTER.TelemetryIngressError, "found 0"):
                HARVESTER.harvest(
                    run_id="run-fixture",
                    slice_id="slice-1",
                    agent_path="/root/telemetry_builder",
                    parent_rollout=path,
                    child_rollouts=[COMPLETE],
                )

    def test_malformed_jsonl_fails_closed(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            path = Path(temporary) / "malformed.jsonl"
            path.write_text("{not-json}\n", encoding="utf-8")
            with self.assertRaisesRegex(HARVESTER.TelemetryIngressError, "cannot parse"):
                self.harvest([path])

    def test_absent_token_telemetry_remains_unavailable(self) -> None:
        child = [record for record in records(COMPLETE) if record.get("payload", {}).get("type") != "token_count"]
        with tempfile.TemporaryDirectory() as temporary:
            path = write_records(Path(temporary), "no-tokens.jsonl", child)
            measurements = self.harvest([path])["builder_runtime"]["measurements"]
            for name in ("input_tokens", "cached_input_tokens", "output_tokens", "reasoning_tokens"):
                self.assertEqual("unavailable", measurements[name]["availability"])
                self.assertIsNone(measurements[name]["value"])
                self.assertEqual([], measurements[name]["provenance"])


if __name__ == "__main__":
    unittest.main()
