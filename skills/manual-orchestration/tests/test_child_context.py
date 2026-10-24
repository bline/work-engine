from __future__ import annotations

import hashlib
import importlib.util
import json
import os
import stat
import tempfile
import unittest
from pathlib import Path


ROOT = Path(__file__).parents[1]


def load(name: str, path: Path):
    spec = importlib.util.spec_from_file_location(name, path)
    assert spec and spec.loader
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


CHILD = load("child_context", ROOT / "scripts" / "child_context.py")
WINDOW = 258_400


def report(input_tokens, window=WINDOW, lineage=()):
    return {
        "session_id": "s",
        "rollout": "/r.jsonl",
        "latest_request": {"input_tokens": input_tokens, "epoch": len(lineage)},
        "context_window_tokens": window,
        "lineage": list(lineage),
    }


def assess(rep, next_task=None):
    return CHILD.assess(
        rep,
        next_task_tokens=next_task,
        trigger_fraction=CHILD.DEFAULT_TRIGGER_FRACTION,
        band_floor_fraction=CHILD.DEFAULT_BAND_FLOOR_FRACTION,
    )


class AssessTests(unittest.TestCase):
    def test_defaults_reproduce_reference_levels(self) -> None:
        record = assess(report(100_000))
        self.assertEqual(record["trigger_tokens"], 150_000)
        self.assertEqual(record["band_floor_tokens"], 171_267)

    def test_below_trigger_retains(self) -> None:
        record = assess(report(100_000))
        self.assertEqual(record["recommendation"], "retain_ok")
        self.assertEqual(record["next_task_basis"], "not estimated")

    def test_auth_worker_case_recommends_fresh(self) -> None:
        # 190,902 of 258,400 when the retained follow-up was dispatched.
        record = assess(report(190_902))
        self.assertEqual(record["recommendation"], "fresh")
        self.assertIn("compaction band", record["reasons"][0])

    def test_between_trigger_and_band_recommends_fresh(self) -> None:
        record = assess(report(160_000))
        self.assertEqual(record["recommendation"], "fresh")

    def test_next_task_estimate_can_tip_to_fresh(self) -> None:
        self.assertEqual(assess(report(120_000))["recommendation"], "retain_ok")
        record = assess(report(120_000), next_task=60_000)
        self.assertEqual(record["recommendation"], "fresh")
        self.assertEqual(record["next_task_basis"], "parent estimate")

    def test_levels_scale_with_window(self) -> None:
        record = assess(report(200_000, window=475_000))
        self.assertEqual(record["recommendation"], "retain_ok")
        self.assertGreater(record["trigger_tokens"], 200_000)

    def test_missing_counters_are_unavailable_not_zero(self) -> None:
        rep = report(None)
        record = assess(rep)
        self.assertEqual(record["recommendation"], "usage_unavailable")
        self.assertEqual(record["unavailable_class"], "absent_counters")
        self.assertNotIn("headroom_to_trigger", record)

    def test_invalid_and_historical_counters_are_unavailable(self) -> None:
        for current in (0, -1, True, 1.5, 18446744073709551616):
            self.assertEqual(assess(report(current))["recommendation"], "usage_unavailable")
        historical = report(91579)
        historical.update(latest_request_status="historical_observation", last_record_gap_line=4)
        historical["latest_request"]["line"] = 3
        result = assess(historical)
        self.assertEqual(result["recommendation"], "usage_unavailable")
        self.assertNotIn("headroom_to_trigger", result)

    def test_tool_failure_is_classified(self) -> None:
        record = assess({"_error": "permission denied", "_exit": 126})
        self.assertEqual(record["unavailable_class"], "launch_or_tool_failure")

    def test_compaction_markers_counted_from_lineage(self) -> None:
        record = assess(report(30_000, lineage=["compaction marker at line 9", "model change at line 12"]))
        self.assertEqual(record["compaction_markers"], 1)
        self.assertTrue(any("compacted 1 time" in r for r in record["reasons"]))

    def test_identity_rejection_is_distinguished(self) -> None:
        for detail in ("context-status: session_meta.id mismatch at line 1",
                       "context-status: requested session rollout not found"):
            self.assertEqual(assess({"_error": detail, "_exit": 1})["unavailable_class"], "identity_rejection")


class BinaryTests(unittest.TestCase):
    def setUp(self) -> None:
        self.directory = Path(tempfile.mkdtemp())
        self.binary = self.directory / "work-engine-context-status"
        payload = json.dumps(report(190_902))
        self.binary.write_text(f"#!/bin/sh\necho '{payload}'\n")
        self.binary.chmod(self.binary.stat().st_mode | stat.S_IXUSR)
        self.digest = hashlib.sha256(self.binary.read_bytes()).hexdigest()

    def tearDown(self) -> None:
        for path in self.directory.iterdir():
            path.unlink()
        self.directory.rmdir()

    def test_digest_mismatch_refuses(self) -> None:
        with self.assertRaises(CHILD.UsageError):
            CHILD.verify_binary(str(self.binary), "0" * 64)

    def test_check_end_to_end(self) -> None:
        out = self.directory / "record.json"
        code = CHILD.main(
            ["--binary", str(self.binary), "--sha256", self.digest, "--out", str(out),
             "check", "--session", "s", "--rollout", "/r.jsonl"]
        )
        self.assertEqual(code, 0)
        self.assertEqual(json.loads(out.read_text())["recommendation"], "fresh")

    def test_documented_form_out_after_subcommand(self) -> None:
        # The form context-boundary.md documents: --out after `check`.
        out = self.directory / "documented.json"
        code = CHILD.main(
            ["--binary", str(self.binary), "--sha256", self.digest,
             "check", "--session", "s", "--rollout", "/r.jsonl", "--out", str(out)]
        )
        self.assertEqual(code, 0)
        self.assertEqual(json.loads(out.read_text())["recommendation"], "fresh")


class ClaudeTranscriptTests(unittest.TestCase):
    def write(self, records):
        handle = tempfile.NamedTemporaryFile("w", suffix=".jsonl", delete=False)
        for record in records:
            handle.write(json.dumps(record) + "\n")
        handle.close()
        self.addCleanup(os.unlink, handle.name)
        return handle.name

    def assistant(self, mid, inp, created, read):
        return {"type": "assistant", "agentId": "agent-x", "requestId": mid, "timestamp": "t",
                "message": {"id": mid, "model": "claude-opus-5-5",
                            "usage": {"input_tokens": inp, "cache_creation_input_tokens": created,
                                      "cache_read_input_tokens": read, "output_tokens": 9}}}

    def run_claude(self, path, *extra):
        out = Path(tempfile.mkdtemp()) / "out.json"
        self.addCleanup(lambda: out.exists() and out.unlink())
        code = CHILD.main(["claude", "--transcript", path, "--out", str(out), *extra])
        self.assertEqual(code, 0)
        return json.loads(out.read_text())

    def test_context_is_sum_of_latest_call_and_needs_no_binary(self) -> None:
        path = self.write([self.assistant("a", 10, 1_000, 50_000), self.assistant("b", 5, 2_000, 160_000)])
        record = self.run_claude(path)
        self.assertEqual(record["input_tokens"], 162_005)
        self.assertEqual(record["recommendation"], "fresh")
        self.assertEqual(record["trigger_tokens"], 150_000)
        self.assertEqual(record["band_floor_tokens"], 202_533)
        self.assertIsNone(record["fraction_of_window"])
        self.assertEqual(record["host"], "claude-code")

    def test_below_trigger_retains(self) -> None:
        record = self.run_claude(self.write([self.assistant("a", 10, 1_000, 60_000)]))
        self.assertEqual(record["recommendation"], "retain_ok")

    def test_compaction_counted(self) -> None:
        path = self.write([{"type": "system", "subtype": "compact_boundary",
                            "compactMetadata": {"trigger": "auto", "preTokens": 210_000}},
                           self.assistant("a", 10, 1_000, 20_000)])
        record = self.run_claude(path)
        self.assertEqual(record["compaction_markers"], 1)

    def test_shared_usable_request_cases_and_recovery(self) -> None:
        fixture = ROOT.parents[1] / "rust/crates/work-engine-context-status/tests/fixtures/claude-usable-requests.json"
        for case in json.loads(fixture.read_text()):
            with self.subTest(case=case["name"]):
                rows = [self.assistant("before", 10, 1000, 90569),
                        {"type": "assistant", "timestamp": "2026-10-23T15:18:38.416Z",
                         "message": case["message"]}]
                result = self.run_claude(self.write(rows))
                expected = "usage_unavailable" if case["context_tokens"] is None else "retain_ok"
                self.assertEqual(result["recommendation"], expected)
                if expected == "usage_unavailable":
                    self.assertNotIn("headroom_to_trigger", result)
                rows.append(self.assistant("after", 0, 1000, 24000))
                result = self.run_claude(self.write(rows))
                self.assertEqual(result["recommendation"], "retain_ok")
                self.assertEqual(result["input_tokens"], 25000)

    def test_compaction_without_new_usage_invalidates_prior_reading(self) -> None:
        result = self.run_claude(self.write([self.assistant("before", 10, 1000, 60000),
            {"type": "system", "subtype": "compact_boundary"}]))
        self.assertEqual(result["recommendation"], "usage_unavailable")

    def test_missing_transcript_is_unavailable(self) -> None:
        record = self.run_claude("/nonexistent/agent.jsonl")
        self.assertEqual(record["recommendation"], "usage_unavailable")

    def test_no_usage_is_unavailable_not_zero(self) -> None:
        record = self.run_claude(self.write([{"type": "user", "message": {"content": "hi"}}]))
        self.assertEqual(record["unavailable_class"], "absent_counters")


if __name__ == "__main__":
    unittest.main()
