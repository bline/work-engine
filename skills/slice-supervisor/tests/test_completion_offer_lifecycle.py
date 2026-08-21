from __future__ import annotations

import importlib.util
import tempfile
import unittest
from pathlib import Path


ROOT = Path(__file__).parents[1]
WORK_ENGINE_ROOT = Path(__file__).parents[3]


def load(name: str, path: Path):
    spec = importlib.util.spec_from_file_location(name, path)
    assert spec and spec.loader
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


OFFER = load("vertical_completion_offer", ROOT / "scripts" / "completion_offer_lifecycle.py")
FINALIZER = load("vertical_offer_finalizer", ROOT / "scripts" / "finalize_receipt.py")
RESUME = load("vertical_offer_resume", ROOT / "scripts" / "resume_campaign.py")
CHECKPOINT = load("vertical_offer_checkpoint", WORK_ENGINE_ROOT / "skills/slice-checkpoint/scripts/checkpoint.py")
CHECKPOINT_LIFECYCLE = load("vertical_offer_checkpoint_lifecycle", ROOT / "scripts" / "checkpoint_lifecycle.py")
COMPLETION_TESTS = load(
    "vertical_offer_completion_fixtures",
    WORK_ENGINE_ROOT / "skills/slice-completion-commit/tests/test_completion_commit.py",
)
ASSEMBLER_TESTS = load("vertical_offer_assembler", Path(__file__).with_name("test_assemble_receipt.py"))


class CompletionOfferLifecycleTest(unittest.TestCase):
    def accepted(self, directory: str):
        fixture = COMPLETION_TESTS.CompletionCommitTest()
        repository, head = fixture.repository(directory)
        request = fixture.request(repository, head)
        request["run_id"] = "test-run"
        checkpoint_request = {
            "schema_version": 1, "repository": str(repository), "run_id": "test-run",
            "slice_number": 1, "candidate_attempt": 1, "baseline_commit_oid": head,
            "parent_checkpoint_commit_oid": None, "plan_version": "plan-1",
            "scope_revision": "scope-1", "gate_receipt_digest": "b" * 64,
            "created_at": "2026-08-21T12:00:00+00:00",
            "paths": [{"path": "task.txt", "action": "include", "attribution": "task_owned"}],
        }
        candidate = CHECKPOINT.create_candidate(checkpoint_request)
        binding = {field: candidate[field] for field in CHECKPOINT_LIFECYCLE.BINDING_FIELDS}
        accepted = CHECKPOINT_LIFECYCLE.accept(candidate, binding, "b" * 64)
        request["proposal"].update(
            checkpoint_commit_oid=accepted["checkpoint_commit_oid"],
            checkpoint_tree_oid=accepted["checkpoint_tree_oid"],
            task_patch_digest=accepted["task_patch_digest"],
        )
        semantic = ASSEMBLER_TESTS.ReceiptAssemblerTest().semantic_receipt()
        semantic.update(
            status="accepted", stop_reason=None, plan_acceptance="procedural_auto_approval",
            validation_requirement_results={"focused_checks": "passed"},
            placement_certificate={"owner": "supervisor completion offer"},
            placement_verdict="confirmed", placement_risk="high",
            vertical_semantic_test="completion offer fresh-process recovery",
            vertical_semantic_test_passed=True, more_in_scope_work_remains=True,
        )
        preflight = ASSEMBLER_TESTS.preflight_result(semantic["engine_config"])
        handoff = {
            "slice_title": semantic["slice_title"], "slice_goal": semantic["slice_goal"],
            "outcome": semantic["outcome"], "durable_decisions": [],
            "affected_boundaries": ["completion offer lifecycle"],
            "placement_certificate": semantic["placement_certificate"],
            "unresolved_concerns": [], "deferred_scope": ["prompt defaults"],
        }
        metrics = repository / "metrics" / "receipts.jsonl"
        terminal = FINALIZER.finalize(
            metrics, semantic, ASSEMBLER_TESTS.ingress(), preflight, handoff, accepted
        )
        request["operational_paths"] = ["metrics/receipts.jsonl"]
        return repository, request, accepted, metrics, terminal, preflight

    def test_pending_survives_fresh_resume_and_decline_without_rewriting_terminal(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            repository, request, accepted, metrics, _terminal, preflight = self.accepted(directory)
            terminal_bytes = metrics.read_bytes()
            before_index = COMPLETION_TESTS.run(repository, "write-tree")
            before_status = COMPLETION_TESTS.run(repository, "status", "--short")
            opened = OFFER.open_offer(request)
            self.assertEqual("open", opened["state"])
            self.assertEqual(before_index, COMPLETION_TESTS.run(repository, "write-tree"))
            self.assertEqual(before_status, COMPLETION_TESTS.run(repository, "status", "--short"))

            resumed = RESUME.resume(metrics, preflight, "test-run")
            self.assertTrue(resumed["resumable"])
            self.assertEqual(accepted["checkpoint_commit_oid"], resumed["baseline_checkpoint"]["checkpoint_commit_oid"])
            self.assertEqual(opened["offer_id"], resumed["completion_offer"]["offer_id"])
            declined = OFFER.resolve(resumed["completion_offer"], "decline")
            self.assertEqual("declined", declined["state"])
            self.assertEqual(terminal_bytes, metrics.read_bytes())
            self.assertEqual(declined, OFFER.resolve(declined, "decline"))

    def test_publication_crash_reconciles_created_without_replaying_mutation(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            repository, request, accepted, metrics, _terminal, preflight = self.accepted(directory)
            terminal_bytes = metrics.read_bytes()
            opened = OFFER.open_offer(request)
            published = []

            def crash(commit_oid: str) -> None:
                published.append(commit_oid)
                raise SystemExit("injected crash after publication")

            with self.assertRaises(SystemExit):
                OFFER.resolve(opened, "create", after_publish=crash)
            self.assertEqual(published[0], COMPLETION_TESTS.run(repository, "rev-parse", "HEAD"))
            self.assertEqual("open", OFFER.load(repository, "test-run", 1)["state"])

            recovered_open = RESUME.resume(metrics, preflight, "test-run")["completion_offer"]
            # Even a contrary delayed decision must reconcile the prior publication first.
            reconciled = OFFER.resolve(recovered_open, "decline")
            self.assertIsNotNone(reconciled)
            self.assertEqual("created", reconciled["state"])
            self.assertEqual(published[0], reconciled["result"]["commit_oid"])
            self.assertEqual(published[0], COMPLETION_TESTS.run(repository, "rev-parse", "HEAD"))
            self.assertEqual(terminal_bytes, metrics.read_bytes())
            resumed = RESUME.resume(metrics, preflight, "test-run")
            self.assertEqual("created", resumed["completion_offer"]["state"])
            self.assertEqual(accepted["checkpoint_commit_oid"], resumed["baseline_checkpoint"]["checkpoint_commit_oid"])

    def test_expiration_fails_closed_when_publication_state_is_ambiguous(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            repository, request, _accepted, _metrics, _terminal, _preflight = self.accepted(directory)
            opened = OFFER.open_offer(request)
            unrelated = COMPLETION_TESTS.run(
                repository, "commit-tree", request["proposal"]["checkpoint_tree_oid"],
                "-p", request["expected_head_oid"], "-m", "unrelated publication",
            )
            COMPLETION_TESTS.run(
                repository, "update-ref", "refs/heads/main", unrelated,
                request["expected_head_oid"],
            )
            with self.assertRaisesRegex(ValueError, "ambiguous"):
                OFFER.expire(opened, "offer no longer available")
            self.assertEqual("open", OFFER.load(repository, "test-run", 1)["state"])


if __name__ == "__main__":
    unittest.main()
