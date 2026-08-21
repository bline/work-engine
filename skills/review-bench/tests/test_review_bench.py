from __future__ import annotations

import importlib.util
import json
import subprocess
import sys
import tempfile
import unittest
from copy import deepcopy
from pathlib import Path


ROOT = Path(__file__).parents[1]
SCRIPT = ROOT / "scripts" / "review_bench.py"
FIXTURE = Path(__file__).parent / "fixtures"
SPEC = importlib.util.spec_from_file_location("review_bench", SCRIPT)
assert SPEC and SPEC.loader
BENCH = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(BENCH)


def load(path: Path):
    return json.loads(path.read_text(encoding="utf-8"))


class ReviewBenchTest(unittest.TestCase):
    def setUp(self) -> None:
        self.corpus = BENCH.validate_corpus(load(FIXTURE / "corpus.json"))
        self.truth = BENCH.validate_truth(load(FIXTURE / "truth.json"), self.corpus)

    def test_vertical_compare_preserves_complementarity_and_false_block_cost(self) -> None:
        completed = subprocess.run(
            [
                sys.executable,
                str(SCRIPT),
                "compare",
                "--corpus",
                str(FIXTURE / "corpus.json"),
                "--truth",
                str(FIXTURE / "truth.json"),
                "--results-dir",
                str(FIXTURE / "results"),
                "--scoring-dir",
                str(FIXTURE / "scoring"),
            ],
            check=True,
            capture_output=True,
            text=True,
        )
        report = json.loads(completed.stdout)
        claude = next(value for value in report["configurations"].values() if value["provider"] == "anthropic")
        web_sol = next(value for value in report["configurations"].values() if value["harness"] == "chatgpt-web-attachment")
        self.assertEqual(claude["defect_recall"], 0.5)
        self.assertEqual(web_sol["defect_recall"], 0.5)
        self.assertEqual(claude["false_blocks"], 0)
        self.assertEqual(web_sol["false_blocks"], 1)
        self.assertEqual(claude["severity_overstatements"], 0)
        self.assertEqual(claude["blocking_label_understatements"], 0)
        self.assertEqual(claude["nonblocking_observation_findings"], 0)
        self.assertEqual(web_sol["reasoning_effort"], "high")
        self.assertEqual(web_sol["resources"]["input_tokens"]["total"], 2900)
        self.assertIn("persistent_state_identity_integrity", report["by_route_class"])
        pair = report["pairwise_complementarity"][0]
        self.assertEqual(pair["truth_unique_to_left"], 1)
        self.assertEqual(pair["truth_unique_to_right"], 1)
        self.assertEqual(pair["truth_jointly_missed"], 0)
        self.assertEqual(pair["joint_miss_rate"], 0.0)
        self.assertEqual(pair["right_conditional_recall_given_left_miss"], 1.0)

    def test_result_must_bind_to_case_snapshot(self) -> None:
        result = load(FIXTURE / "results" / "claude-defect.json")
        result["snapshot_digest"] = "f" * 64
        with self.assertRaisesRegex(BENCH.BenchError, "snapshot_digest"):
            BENCH.validate_result(result, self.corpus)

    def test_scoring_must_classify_all_findings_and_exact_misses(self) -> None:
        result = BENCH.validate_result(load(FIXTURE / "results" / "claude-defect.json"), self.corpus)
        scoring = load(FIXTURE / "scoring" / "claude-defect.json")
        scoring["missed_truth_ids"] = []
        with self.assertRaisesRegex(BENCH.BenchError, "missed_truth_ids"):
            BENCH.validate_scoring(scoring, self.corpus, self.truth, result)

    def test_export_is_blinded_and_contains_exact_snapshot_digest(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            receipt = BENCH.export_case(self.corpus, "defective-state", Path(directory), None, False)
            packet = load(Path(receipt["packet"]))
            prompt = Path(receipt["prompt"]).read_text(encoding="utf-8")
            self.assertEqual(packet["case"]["snapshot"]["digest"], "a" * 64)
            self.assertNotIn("identity-replaced", json.dumps(packet))
            self.assertNotIn("stale-mutates", prompt)
            self.assertIn("review_bench_result_v1", prompt)

    def test_inventory_truthfully_marks_head_plus_manifest_as_partial(self) -> None:
        receipt = {
            "run_id": "run-1",
            "slice_number": 1,
            "slice_title": "fixture",
            "status": "accepted",
            "review_findings": {"high": 1},
            "task_owned_files": ["src/state.py"],
            "worker_metrics": {
                "review_gate_calls": 1,
                "review_fix_iterations": 1,
                "additional_metrics": {"workspace": {"head": "abc123"}},
            },
        }
        with tempfile.TemporaryDirectory() as directory:
            metrics = Path(directory) / "metrics.jsonl"
            metrics.write_text(json.dumps(receipt) + "\n", encoding="utf-8")
            inventory = BENCH.inventory_history([metrics], None)
            self.assertEqual(inventory["candidate_cases"], 1)
            self.assertEqual(inventory["candidates"][0]["reconstruction_state"], "partial")
            self.assertIn("immutable dirty-worktree tree or checkpoint", inventory["candidates"][0]["missing_for_exact_case"])

    def test_inventory_prefers_exact_slice_agent_path_over_run_only_matches(self) -> None:
        rows = [
            {
                "run_id": "run-1", "slice_number": number, "slice_title": f"slice {number}", "status": "accepted",
                "task_owned_files": [f"slice-{number}.py"],
                "worker_metrics": {"review_gate_calls": 1, "review_fix_iterations": 0},
            }
            for number in (1, 2)
        ]
        with tempfile.TemporaryDirectory() as directory:
            directory = Path(directory)
            metrics = directory / "metrics.jsonl"
            metrics.write_text("".join(json.dumps(row) + "\n" for row in rows), encoding="utf-8")
            sessions = directory / "sessions"
            sessions.mkdir()
            for number in (1, 2):
                source = {"subagent": {"thread_spawn": {"agent_path": f"/root/roadmap_run_slice{number}"}}}
                events = [
                    {"type": "session_meta", "payload": {"source": source}},
                    {"type": "response_item", "payload": {"text": "run-1"}},
                ]
                (sessions / f"slice-{number}.jsonl").write_text(
                    "".join(json.dumps(event) + "\n" for event in events), encoding="utf-8"
                )
            inventory = BENCH.inventory_history([metrics], sessions)
            self.assertEqual([len(case["codex_session_candidates"]) for case in inventory["candidates"]], [1, 1])
            self.assertEqual({case["session_match_quality"] for case in inventory["candidates"]}, {"run_and_slice"})

    def test_unknown_fields_fail_closed(self) -> None:
        corpus = deepcopy(self.corpus)
        corpus["cases"][0]["historical_finding"] = "leak"
        with self.assertRaisesRegex(BENCH.BenchError, "unknown fields"):
            BENCH.validate_corpus(corpus)

    def test_reasoning_effort_is_part_of_configuration_identity(self) -> None:
        result = BENCH.validate_result(load(FIXTURE / "results" / "web-sol-defect.json"), self.corpus)
        changed = deepcopy(result)
        changed["reviewer"]["reasoning_effort"] = "max"
        self.assertNotEqual(BENCH.configuration_key(result), BENCH.configuration_key(changed))

    def test_single_pass_cannot_claim_aggregation(self) -> None:
        result = load(FIXTURE / "results" / "web-sol-defect.json")
        result["reviewer"]["aggregation_strategy"] = "self_aggregation"
        with self.assertRaisesRegex(BENCH.BenchError, "must be none"):
            BENCH.validate_result(result, self.corpus)

    def test_aggregated_result_identifies_member_configurations(self) -> None:
        result = load(FIXTURE / "results" / "web-sol-defect.json")
        result["reviewer"]["pass_count"] = 3
        result["reviewer"]["aggregation_strategy"] = "self_aggregation"
        result["reviewer"]["aggregation_members"] = ["openai|gpt-5.6-sol|codex|high"]
        BENCH.validate_result(result, self.corpus)
        result["reviewer"]["aggregation_members"] = []
        with self.assertRaisesRegex(BENCH.BenchError, "aggregation_members"):
            BENCH.validate_result(result, self.corpus)

    def test_conditional_recall_uses_the_other_reviewers_misses(self) -> None:
        claude = BENCH.validate_result(load(FIXTURE / "results" / "claude-defect.json"), self.corpus)
        claude_scoring = BENCH.validate_scoring(
            load(FIXTURE / "scoring" / "claude-defect.json"), self.corpus, self.truth, claude
        )
        sol = load(FIXTURE / "results" / "web-sol-defect.json")
        sol["verdict"] = "accepted"
        sol["findings"] = []
        sol = BENCH.validate_result(sol, self.corpus)
        sol_scoring = load(FIXTURE / "scoring" / "web-sol-defect.json")
        sol_scoring["finding_dispositions"] = []
        sol_scoring["missed_truth_ids"] = ["identity-replaced", "stale-mutates"]
        sol_scoring = BENCH.validate_scoring(sol_scoring, self.corpus, self.truth, sol)
        report = BENCH.compare(self.corpus, self.truth, [claude, sol], [claude_scoring, sol_scoring])
        pair = report["pairwise_complementarity"][0]
        self.assertEqual(pair["right_conditional_recall_given_left_miss"], 0.0)
        self.assertEqual(pair["left_conditional_recall_given_right_miss"], 0.5)
        sol_metrics = next(value for value in report["configurations"].values() if value["provider"] == "openai")
        self.assertEqual(sol_metrics["false_accepts"], 1)

    def test_rejection_is_a_false_block_when_expected_verdict_accepts_nonblocking_truth(self) -> None:
        truth = deepcopy(self.truth)
        truth_case = next(case for case in truth["cases"] if case["case_id"] == "defective-state")
        truth_case["expected_verdict"] = "accepted"
        for finding in truth_case["findings"]:
            finding["blocking"] = False
        result = BENCH.validate_result(load(FIXTURE / "results" / "claude-defect.json"), self.corpus)
        scoring = BENCH.validate_scoring(
            load(FIXTURE / "scoring" / "claude-defect.json"), self.corpus, truth, result
        )
        report = BENCH.compare(self.corpus, truth, [result], [scoring])
        metrics = next(iter(report["configurations"].values()))
        self.assertEqual(metrics["false_blocks"], 1)

    def test_truth_v2_verifies_superseded_digest_and_exact_amendment(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            directory = Path(directory)
            base_path = directory / "truth.v1.json"
            base_path.write_text(json.dumps(load(FIXTURE / "truth.json")), encoding="utf-8")
            amended = deepcopy(load(FIXTURE / "truth.json"))
            amended["artifact_type"] = "review_bench_truth_v2"
            amended["schema_version"] = 2
            amended["cases"][0]["expected_verdict"] = "accepted"
            amended["supersedes"] = {
                "artifact_type": "review_bench_truth_v1",
                "path": "truth.v1.json",
                "sha256": BENCH.sha256_file(base_path),
            }
            amended["adjudication_amendments"] = [{
                "amendment_id": "fixture-correction",
                "adjudicated_at": "2026-08-20T18:30:00-06:00",
                "adjudicator": "fixture-adjudicator",
                "reason": "Caller evidence changed the verdict.",
                "evidence": ["A supported caller reaches the acceptance-criterion violation."],
                "changes": [{
                    "case_id": "defective-state",
                    "finding_id": None,
                    "field": "expected_verdict",
                    "before": "rejected",
                    "after": "accepted",
                }],
                "limitations": [],
            }]
            amended_path = directory / "truth.json"
            amended_path.write_text(json.dumps(amended), encoding="utf-8")
            validated = BENCH.validate_truth(amended, self.corpus, amended_path)
            self.assertEqual(validated["artifact_type"], "review_bench_truth_v2")
            amended["supersedes"]["sha256"] = "f" * 64
            with self.assertRaisesRegex(BENCH.BenchError, "sha256 does not match"):
                BENCH.validate_truth(amended, self.corpus, amended_path)

    def test_result_v2_separates_findings_verified_claims_and_observations(self) -> None:
        result = load(FIXTURE / "results" / "claude-clean.json")
        result["artifact_type"] = "review_bench_result_v2"
        result["schema_version"] = 2
        result["reviewer"]["review_protocol"] = "evidence-calibrated-review/1.2"
        result["verified_claims"] = [{
            "claim_id": "V1",
            "claim": "Reload preserves the original identity.",
            "evidence": ["The restoration test proves stable identity."],
            "confidence": "high",
            "acceptance_criteria": ["Both regressions are covered and production behavior is corrected."],
        }]
        result["observations"] = [{
            "observation_id": "O1",
            "category": "documentation",
            "claim": "A nearby example can be clearer.",
            "evidence": ["The example omits an explanatory label."],
            "confidence": "high",
        }]
        validated = BENCH.validate_result(result, self.corpus)
        self.assertEqual(validated["artifact_type"], "review_bench_result_v2")
        result["verified_claims"][0]["severity"] = "low"
        with self.assertRaisesRegex(BENCH.BenchError, "unknown fields"):
            BENCH.validate_result(result, self.corpus)
        del result["verified_claims"][0]["severity"]
        result["verified_claims"][0]["acceptance_criteria"] = ["An invented criterion."]
        with self.assertRaisesRegex(BENCH.BenchError, "quote criteria"):
            BENCH.validate_result(result, self.corpus)
        result["verified_claims"][0]["acceptance_criteria"] = [
            "Both regressions are covered and production behavior is corrected."
        ]
        result["verified_claims"][0]["confidence"] = "medium"
        with self.assertRaisesRegex(BENCH.BenchError, "must be high"):
            BENCH.validate_result(result, self.corpus)

    def test_accepted_result_v2_requires_complete_verified_coverage(self) -> None:
        result = load(FIXTURE / "results" / "claude-clean.json")
        result["artifact_type"] = "review_bench_result_v2"
        result["schema_version"] = 2
        result["reviewer"]["review_protocol"] = "evidence-calibrated-review/1.2"
        result["verified_claims"] = []
        result["observations"] = []
        with self.assertRaisesRegex(BENCH.BenchError, "verify every"):
            BENCH.validate_result(result, self.corpus)

    def test_protocol_1_2_export_requests_result_v2_sections(self) -> None:
        corpus = deepcopy(self.corpus)
        corpus["protocol"]["version"] = "1.2"
        with tempfile.TemporaryDirectory() as directory:
            receipt = BENCH.export_case(corpus, "clean-closure", Path(directory), None, False)
            prompt = Path(receipt["prompt"]).read_text(encoding="utf-8")
            self.assertIn("review_bench_result_v2", prompt)
            self.assertIn("verified_claims", prompt)
            self.assertIn("observations", prompt)

    def test_report_counts_v2_verification_separately_from_findings(self) -> None:
        result = load(FIXTURE / "results" / "claude-clean.json")
        result["artifact_type"] = "review_bench_result_v2"
        result["schema_version"] = 2
        result["reviewer"]["review_protocol"] = "evidence-calibrated-review/1.2"
        result["verified_claims"] = [{
            "claim_id": "V1", "claim": "The repair is established.",
            "evidence": ["Focused evidence."], "confidence": "high",
            "acceptance_criteria": ["Both regressions are covered and production behavior is corrected."],
        }]
        result["observations"] = [{
            "observation_id": "O1", "category": "documentation",
            "claim": "One example could be clearer.", "evidence": ["Direct source."],
            "confidence": "high",
        }]
        result = BENCH.validate_result(result, self.corpus)
        scoring = BENCH.validate_scoring(
            load(FIXTURE / "scoring" / "claude-clean.json"), self.corpus, self.truth, result
        )
        report = BENCH.compare(self.corpus, self.truth, [result], [scoring])
        metrics = next(iter(report["configurations"].values()))
        self.assertEqual(metrics["reported_findings"], 0)
        self.assertEqual(metrics["verified_claims"], 1)
        self.assertEqual(metrics["reviewer_observations"], 1)


if __name__ == "__main__":
    unittest.main()
