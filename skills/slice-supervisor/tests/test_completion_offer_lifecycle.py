from __future__ import annotations

import importlib.util
import copy
import json
import subprocess
import sys
import tempfile
import unittest
from unittest import mock
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
    def decision(self, value: str, reference: str = "user-message-1"):
        return {
            "decision": value,
            "authority": {
                "kind": "human",
                "reference": reference,
                "observed_at": "2026-09-01T09:00:00-06:00",
            },
        }

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
            decision = self.decision("decline")
            declined = OFFER.resolve(resumed["completion_offer"], decision)
            self.assertEqual("declined", declined["state"])
            self.assertEqual(terminal_bytes, metrics.read_bytes())
            self.assertEqual(decision, declined["decision"])
            self.assertEqual(declined, OFFER.resolve(declined, decision))
            with self.assertRaisesRegex(ValueError, "conflicts"):
                OFFER.resolve(declined, self.decision("create", "user-message-2"))

    def test_create_decision_is_durable_before_later_publication_and_reconciliation(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            repository, request, accepted, metrics, _terminal, preflight = self.accepted(directory)
            terminal_bytes = metrics.read_bytes()
            opened = OFFER.open_offer(request)
            before = COMPLETION_TESTS.run(repository, "rev-parse", "HEAD")
            decision = self.decision("create")
            authorized = OFFER.resolve(opened, decision)
            self.assertEqual("create_authorized", authorized["state"])
            self.assertEqual(before, COMPLETION_TESTS.run(repository, "rev-parse", "HEAD"))
            self.assertIsNone(OFFER.reconcile(authorized))

            # This direct adapter call stands in for the later, separately mediated
            # completion_publication capability. Offer resolution never performs it.
            publication = OFFER.ADAPTER.decide(request, "create")
            self.assertEqual("created", publication["state"])
            recovered = RESUME.resume(metrics, preflight, "test-run")["completion_offer"]
            self.assertEqual("create_authorized", recovered["state"])
            reconciled = OFFER.reconcile(recovered)
            self.assertIsNotNone(reconciled)
            self.assertEqual("created", reconciled["state"])
            self.assertEqual(publication["commit_oid"], reconciled["result"]["commit_oid"])
            self.assertEqual(decision, reconciled["decision"])
            self.assertEqual(terminal_bytes, metrics.read_bytes())
            resumed = RESUME.resume(metrics, preflight, "test-run")
            self.assertEqual("created", resumed["completion_offer"]["state"])
            self.assertEqual(accepted["checkpoint_commit_oid"], resumed["baseline_checkpoint"]["checkpoint_commit_oid"])

    def test_expiration_fails_closed_when_publication_state_is_ambiguous(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            repository, request, _accepted, _metrics, _terminal, _preflight = self.accepted(directory)
            opened = OFFER.open_offer(request)
            authorized = OFFER.resolve(opened, self.decision("create"))
            unrelated = COMPLETION_TESTS.run(
                repository, "commit-tree", request["proposal"]["checkpoint_tree_oid"],
                "-p", request["expected_head_oid"], "-m", "unrelated publication",
            )
            COMPLETION_TESTS.run(
                repository, "update-ref", "refs/heads/main", unrelated,
                request["expected_head_oid"],
            )
            with self.assertRaisesRegex(ValueError, "ambiguous"):
                OFFER.expire(authorized, "offer no longer available")
            self.assertEqual("create_authorized", OFFER.load(repository, "test-run", 1)["state"])

    def test_supersession_preserves_authorized_offer_and_installs_distinct_open_successor(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            repository, request, _accepted, _metrics, _terminal, _preflight = self.accepted(directory)
            authorized = OFFER.resolve(OFFER.open_offer(request), self.decision("create"))
            successor = copy.deepcopy(request)
            successor["proposal"]["subject"] = "Replacement accepted checkpoint"
            operation_id = "replace-stale-offer-v1"
            publication_state = {
                "operation_id": f"completion-{authorized['offer_id']}", "status": "absent",
            }
            result = OFFER.supersede(
                authorized, successor, operation_id, "accepted checkpoint was superseded",
                publication_state,
            )
            self.assertEqual("open", result["offer"]["state"])
            self.assertNotEqual(authorized["offer_id"], result["offer"]["offer_id"])
            self.assertEqual(result["offer"], OFFER.load(repository, "test-run", 1))
            self.assertEqual(authorized["artifact_oid"], result["supersession"]["prior_offer_oid"])
            self.assertEqual(result, OFFER.supersede(
                authorized, successor, operation_id, "accepted checkpoint was superseded",
                publication_state,
            ))
            replacement_authorized = OFFER.resolve(result["offer"], self.decision("create", "user-message-2"))
            self.assertEqual("create_authorized", replacement_authorized["state"])
            self.assertEqual("user-message-2", replacement_authorized["decision"]["authority"]["reference"])

    def test_supersession_fails_closed_when_prior_offer_was_published(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            repository, request, _accepted, _metrics, _terminal, _preflight = self.accepted(directory)
            authorized = OFFER.resolve(OFFER.open_offer(request), self.decision("create"))
            OFFER.ADAPTER.decide(request, "create")
            successor = copy.deepcopy(request)
            successor["proposal"]["subject"] = "Replacement accepted checkpoint"
            with self.assertRaisesRegex(ValueError, "prior publication is present"):
                OFFER.supersede(
                    authorized, successor, "replace-published-offer-v1",
                    "must not replace published offer",
                    {"operation_id": f"completion-{authorized['offer_id']}", "status": "absent"},
                )

    def test_open_offer_never_implies_human_authority(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            repository, request, _accepted, _metrics, _terminal, _preflight = self.accepted(directory)
            opened = OFFER.open_offer(request)
            self.assertEqual(2, opened["schema_version"])
            self.assertIsNone(opened["decision"])
            with self.assertRaisesRegex(ValueError, "exact human authority"):
                OFFER.resolve(opened, {
                    "decision": "create",
                    "authority": {"kind": "agent", "reference": "guess", "observed_at": "now"},
                })
            self.assertEqual("open", OFFER.load(repository, "test-run", 1)["state"])

    def test_preservation_complete_approved_request_join_on_every_consumption_path(self):
        for state, differing in ((s, d) for s in ("created", "refused", "declined") for d in ("manifest", "index", "mode")):
            with self.subTest(state=state, differing=differing), tempfile.TemporaryDirectory() as directory:
                repository, request_b, _accepted = COMPLETION_TESTS.preservation_fixture(directory)
                request_a = copy.deepcopy(request_b)
                original = Path(request_a["preservation"]["manifest"]["path"])
                alternate = original.with_name("separately-approved-manifest.json")
                alternate.write_bytes(original.read_bytes())
                request_a["preservation"]["manifest"]["path"] = str(alternate)
                if differing == "index":
                    request_a = copy.deepcopy(request_b)
                    request_a["preservation"]["expected_index_sha256"] = "f" * 64
                elif differing == "mode":
                    request_a = copy.deepcopy(request_b)
                    del request_a["preservation"]
                decision = "decline" if state == "declined" else "create"
                opened = OFFER.open_offer(request_a)
                authorized = (opened if state == "declined" else OFFER.resolve(opened, self.decision("create")))
                if state == "created":
                    result_b = OFFER.ADAPTER.decide(request_b, "create")
                    self.assertIsNone(result_b["reason"], result_b)
                else:
                    result_b = OFFER.ADAPTER.receipt(request_b, state, reason="synthetic non-effect")
                # This must be a valid B, not a malformed-receipt rejection test.
                self.assertEqual(result_b, OFFER.ADAPTER.validate_receipt(result_b))
                before_ref = authorized["artifact_oid"]
                with self.assertRaisesRegex(ValueError, "complete approved offer request"):
                    OFFER.transition(authorized, state, result=result_b, decision=self.decision(decision))
                self.assertEqual(before_ref, OFFER.load(repository, "test-run", 1)["artifact_oid"])
                if state == "created":
                    with mock.patch.object(OFFER.ADAPTER, "reconcile_created", return_value=result_b):
                        with self.assertRaisesRegex(ValueError, "complete approved offer request"):
                            OFFER.reconcile(authorized)
                terminal = {k: v for k, v in authorized.items() if k not in {"ref", "artifact_oid"}}
                terminal.update(state=state, result=result_b, decision=self.decision(decision), prior_oid=before_ref)
                persisted = OFFER.write(repository, terminal, before_ref)
                with self.assertRaisesRegex(ValueError, "complete approved offer request"):
                    OFFER.read_oid(repository, persisted["artifact_oid"])
                with self.assertRaisesRegex(ValueError, "complete approved offer request"):
                    OFFER.reconcile(persisted)
                code = "import importlib.util,sys; from pathlib import Path; s=importlib.util.spec_from_file_location('o',sys.argv[1]); m=importlib.util.module_from_spec(s); s.loader.exec_module(m); m.load(Path(sys.argv[2]),'test-run',1)"
                fresh = subprocess.run([sys.executable, "-B", "-c", code, str(ROOT / "scripts/completion_offer_lifecycle.py"), str(repository)], capture_output=True, text=True)
                self.assertNotEqual(0, fresh.returncode)
                self.assertIn("complete approved offer request", fresh.stderr)

    def test_preservation_binding_codec_golden_vectors_and_downgrade_refusal(self):
        with tempfile.TemporaryDirectory() as directory:
            repository, request, _accepted = COMPLETION_TESTS.preservation_fixture(directory)
            request["proposal"]["body"] = "Café — exact Unicode authority"
            reordered = dict(reversed(list(request.items())))
            self.assertEqual(OFFER.canonical(request), OFFER.ADAPTER.request_canonical(reordered))
            self.assertEqual(OFFER.offer_id(request), OFFER.ADAPTER.request_binding(reordered)["sha256"])
            opened = OFFER.open_offer(request)
            declined = OFFER.resolve(opened, self.decision("decline"))
            self.assertEqual(declined, OFFER.validate(declined))
            for mutation in ("missing", "codec", "digest", "field", "receipt1", "offer1"):
                bad = copy.deepcopy(declined)
                if mutation == "missing":
                    del bad["result"]["request_binding"]
                elif mutation == "codec":
                    bad["result"]["request_binding"]["canonical_contract"] = "other-codec"
                elif mutation == "digest":
                    bad["result"]["request_binding"]["sha256"] = "a" * 64
                elif mutation == "field":
                    bad["result"]["request_binding"]["request"]["preservation"]["expected_index_sha256"] = "b" * 64
                elif mutation == "receipt1":
                    bad["result"]["schema_version"] = 1
                    del bad["result"]["request_binding"], bad["result"]["evidence"]
                else:
                    bad["schema_version"] = 1
                    del bad["decision"]
                with self.subTest(mutation=mutation), self.assertRaises(ValueError):
                    OFFER.validate(bad)


if __name__ == "__main__":
    unittest.main()
