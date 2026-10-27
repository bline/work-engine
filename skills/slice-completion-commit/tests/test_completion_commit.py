from __future__ import annotations

import importlib.util
import copy
import hashlib
import json
import os
import subprocess
import tempfile
import unittest
from unittest import mock
from pathlib import Path


ROOT = Path(__file__).parents[1]
WORK_ENGINE_ROOT = Path(__file__).parents[3]


def load():
    path = ROOT / "scripts" / "completion_commit.py"
    spec = importlib.util.spec_from_file_location("completion_commit_test_adapter", path)
    assert spec and spec.loader
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


ADAPTER = load()


def load_at(name: str, path: Path):
    spec = importlib.util.spec_from_file_location(name, path)
    assert spec and spec.loader
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


CHECKPOINT = load_at("completion_vertical_checkpoint", WORK_ENGINE_ROOT / "skills/slice-checkpoint/scripts/checkpoint.py")
LIFECYCLE = load_at("completion_vertical_lifecycle", WORK_ENGINE_ROOT / "skills/slice-supervisor/scripts/checkpoint_lifecycle.py")
FINALIZER = load_at("completion_vertical_finalizer", WORK_ENGINE_ROOT / "skills/slice-supervisor/scripts/finalize_receipt.py")
RESUME = load_at("completion_vertical_resume", WORK_ENGINE_ROOT / "skills/slice-supervisor/scripts/resume_campaign.py")
ASSEMBLER_TESTS = load_at("completion_vertical_assembler", WORK_ENGINE_ROOT / "skills/slice-supervisor/tests/test_assemble_receipt.py")
OFFER = load_at("preservation_vertical_offer", WORK_ENGINE_ROOT / "skills/slice-supervisor/scripts/completion_offer_lifecycle.py")


def run(repository: Path, *args: str) -> str:
    result = subprocess.run(["git", "-C", str(repository), *args], text=True,
                            stdout=subprocess.PIPE, stderr=subprocess.PIPE, check=False)
    if result.returncode:
        raise AssertionError(result.stderr)
    return result.stdout.strip()


class CompletionCommitTest(unittest.TestCase):
    def repository(self, directory: str) -> tuple[Path, str]:
        repository = Path(directory) / "repository"
        repository.mkdir()
        run(repository, "init", "-q", "-b", "main")
        run(repository, "config", "user.name", "Test User")
        run(repository, "config", "user.email", "test@example.com")
        (repository / "task.txt").write_text("baseline\n")
        run(repository, "add", "task.txt")
        run(repository, "commit", "-q", "-m", "baseline")
        return repository, run(repository, "rev-parse", "HEAD")

    def request(self, repository: Path, head: str) -> dict[str, object]:
        (repository / "task.txt").write_text("accepted slice\n")
        with tempfile.NamedTemporaryFile() as index:
            env = dict(**__import__("os").environ, GIT_INDEX_FILE=index.name)
            Path(index.name).unlink(missing_ok=True)
            subprocess.run(["git", "-C", str(repository), "read-tree", "HEAD"], env=env, check=True)
            subprocess.run(["git", "-C", str(repository), "add", "task.txt"], env=env, check=True)
            tree = subprocess.run(["git", "-C", str(repository), "write-tree"], env=env,
                                  text=True, stdout=subprocess.PIPE, check=True).stdout.strip()
        checkpoint = run(repository, "commit-tree", tree, "-p", head, "-m", "private checkpoint")
        proposal = {
            "schema_version": 2, "subject": "feat: record accepted slice",
            "body": "Preserve the completing builder's semantic explanation.",
            "paths": ["task.txt"], "checkpoint_commit_oid": checkpoint,
            "checkpoint_tree_oid": tree, "task_patch_digest": "a" * 64,
            "provenance": {
                "schema_version": 1,
                "producer": "completing builder with retained context",
                "evidence": [{"kind": "accepted task patch", "digest": "a" * 64}],
            },
        }
        return {"repository": str(repository), "run_id": "run-1", "slice_number": 1,
                "expected_branch": "main", "expected_head_oid": head,
                "accepted_paths": [{"path": "task.txt", "action": "include"}],
                "proposal": proposal}

    def test_approved_commit_is_exact_and_clean(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            repository, head = self.repository(directory)
            request = self.request(repository, head)
            request["run_id"] = "test-run"
            result = ADAPTER.decide(request, "create")
            self.assertEqual("created", result["state"])
            self.assertEqual(request["proposal"]["checkpoint_tree_oid"], run(repository, "rev-parse", "HEAD^{tree}"))
            self.assertEqual("feat: record accepted slice", run(repository, "show", "-s", "--format=%s", "HEAD"))
            self.assertIn("semantic explanation", run(repository, "show", "-s", "--format=%b", "HEAD"))
            self.assertEqual("", run(repository, "status", "--porcelain=v2", "--untracked-files=all"))

    def test_identical_authorized_content_accepts_distinct_truthful_provenance(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            repository, head = self.repository(directory)
            retained = self.request(repository, head)
            reconstructed = copy.deepcopy(retained)
            reconstructed["proposal"]["provenance"] = {
                "schema_version": 1,
                "producer": "replacement builder from durable evidence",
                "evidence": [
                    {"kind": "accepted task patch", "digest": "a" * 64},
                    {"kind": "terminal continuation context", "digest": "c" * 64},
                ],
            }

            retained_result = ADAPTER.decide(retained, "pending")
            reconstructed_result = ADAPTER.decide(reconstructed, "pending")
            self.assertEqual("pending", retained_result["state"])
            self.assertEqual("pending", reconstructed_result["state"])
            self.assertNotEqual(retained_result["proposal_digest"], reconstructed_result["proposal_digest"])

            for provenance in (
                {"schema_version": 1, "producer": "replacement", "evidence": []},
                {"schema_version": 1, "producer": "", "evidence": [
                    {"kind": "accepted checkpoint receipt", "digest": "b" * 64}
                ]},
                {"schema_version": 1, "producer": "replacement", "evidence": [
                    {"kind": "accepted checkpoint receipt", "digest": "weak"}
                ]},
            ):
                malformed = copy.deepcopy(reconstructed)
                malformed["proposal"]["provenance"] = provenance
                with self.subTest(provenance=provenance), self.assertRaises(ValueError):
                    ADAPTER.decide(malformed, "pending")

    def test_historical_v1_proposal_remains_readable_but_origin_is_closed(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            repository, head = self.repository(directory)
            request = self.request(repository, head)
            proposal = request["proposal"]
            proposal["schema_version"] = 1
            proposal["origin"] = "completing_builder"
            del proposal["provenance"]
            self.assertEqual("pending", ADAPTER.decide(request, "pending")["state"])
            proposal["origin"] = "replacement_builder"
            with self.assertRaisesRegex(ValueError, "historical proposal"):
                ADAPTER.decide(request, "pending")

    def test_created_receipt_verification_rejects_false_git_claims_without_mutation(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            repository, head = self.repository(directory)
            request = self.request(repository, head)
            created = ADAPTER.decide(request, "create")
            self.assertEqual("created", created["state"])
            before_head = run(repository, "rev-parse", "HEAD")
            before_index = run(repository, "write-tree")

            cases = {}
            missing_commit = copy.deepcopy(created)
            missing_commit["commit_oid"] = "b" * 40
            cases["missing commit"] = missing_commit
            wrong_parent = copy.deepcopy(created)
            wrong_parent["expected_head_oid"] = request["proposal"]["checkpoint_commit_oid"]
            cases["wrong parent"] = wrong_parent
            wrong_tree = copy.deepcopy(created)
            wrong_tree["proposal"]["checkpoint_tree_oid"] = "c" * 40
            wrong_tree["proposal_digest"] = hashlib.sha256(
                ADAPTER.canonical(wrong_tree["proposal"])
            ).hexdigest()
            cases["wrong tree"] = wrong_tree
            wrong_message = copy.deepcopy(created)
            wrong_message["proposal"]["subject"] = "fabricated subject"
            wrong_message["proposal_digest"] = hashlib.sha256(
                ADAPTER.canonical(wrong_message["proposal"])
            ).hexdigest()
            cases["wrong message"] = wrong_message
            wrong_publication = copy.deepcopy(created)
            wrong_publication["expected_branch"] = "other"
            cases["publication mismatch"] = wrong_publication
            option_shaped = copy.deepcopy(created)
            option_shaped["commit_oid"] = "--upload-pack=hostile"
            cases["option-shaped object"] = option_shaped

            for name, fabricated in cases.items():
                with self.subTest(name=name), self.assertRaises(ValueError):
                    ADAPTER.validate_receipt(fabricated)
                self.assertEqual(before_head, run(repository, "rev-parse", "HEAD"))
                self.assertEqual(before_index, run(repository, "write-tree"))
                self.assertEqual("", run(repository, "status", "--porcelain=v2", "--untracked-files=all"))

    def test_created_receipt_verification_accepts_later_same_branch_history(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            repository, head = self.repository(directory)
            created = ADAPTER.decide(self.request(repository, head), "create")
            (repository / "later.txt").write_text("later accepted work\n")
            run(repository, "add", "later.txt")
            run(repository, "commit", "-q", "-m", "later accepted work")
            before_head = run(repository, "rev-parse", "HEAD")
            before_index = run(repository, "write-tree")

            self.assertEqual(created, ADAPTER.validate_receipt(created))
            self.assertEqual(before_head, run(repository, "rev-parse", "HEAD"))
            self.assertEqual(before_index, run(repository, "write-tree"))
            self.assertEqual("", run(repository, "status", "--porcelain=v2", "--untracked-files=all"))

    def test_created_receipt_verification_rejects_diverged_or_wrong_attached_branch(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            repository, head = self.repository(directory)
            created = ADAPTER.decide(self.request(repository, head), "create")
            created_commit = created["commit_oid"]
            (repository / "later.txt").write_text("later accepted work\n")
            run(repository, "add", "later.txt")
            run(repository, "commit", "-q", "-m", "later accepted work")

            run(repository, "checkout", "-q", "-b", "other")
            with self.assertRaisesRegex(ValueError, "branch attachment"):
                ADAPTER.validate_receipt(created)

            run(repository, "checkout", "-q", "-B", "main", head)
            (repository / "sibling.txt").write_text("diverged work\n")
            run(repository, "add", "sibling.txt")
            run(repository, "commit", "-q", "-m", "diverged work")
            before_head = run(repository, "rev-parse", "HEAD")
            before_index = run(repository, "write-tree")
            self.assertNotEqual(created_commit, before_head)
            with self.assertRaisesRegex(ValueError, "does not contain"):
                ADAPTER.validate_receipt(created)
            self.assertEqual(before_head, run(repository, "rev-parse", "HEAD"))
            self.assertEqual(before_index, run(repository, "write-tree"))
            self.assertEqual("", run(repository, "status", "--porcelain=v2", "--untracked-files=all"))

    def test_postpublication_failure_receipt_can_be_finalized_when_publication_is_intact(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            repository, head = self.repository(directory)
            request = self.request(repository, head)
            original_git = ADAPTER.git
            injected = False

            def dirty_after_publication(repo, args, **kwargs):
                nonlocal injected
                if not injected and args[:2] == ["status", "--porcelain=v2"]:
                    injected = True
                    (repository / "postpublication.txt").write_text("appeared after publication\n")
                return original_git(repo, args, **kwargs)

            with mock.patch.object(ADAPTER, "git", side_effect=dirty_after_publication):
                created = ADAPTER.decide(request, "create")
            self.assertEqual("created", created["state"])
            self.assertIn("postcondition_failed", created["reason"])
            self.assertEqual(created, ADAPTER.validate_receipt(created))

    def test_approved_deletion_is_part_of_complete_delta(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            repository, head = self.repository(directory)
            (repository / "task.txt").unlink()
            with tempfile.NamedTemporaryFile() as index:
                env = dict(**__import__("os").environ, GIT_INDEX_FILE=index.name)
                Path(index.name).unlink(missing_ok=True)
                subprocess.run(["git", "-C", str(repository), "read-tree", "HEAD"], env=env, check=True)
                subprocess.run(["git", "-C", str(repository), "add", "-A"], env=env, check=True)
                tree = subprocess.run(
                    ["git", "-C", str(repository), "write-tree"], env=env,
                    text=True, stdout=subprocess.PIPE, check=True,
                ).stdout.strip()
            checkpoint = run(repository, "commit-tree", tree, "-p", head, "-m", "private checkpoint")
            request = {
                "repository": str(repository), "run_id": "test-run", "slice_number": 1,
                "expected_branch": "main", "expected_head_oid": head,
                "accepted_paths": [{"path": "task.txt", "action": "delete"}],
                "proposal": {
                    "schema_version": 1, "subject": "chore: remove task", "body": "",
                    "paths": ["task.txt"], "checkpoint_commit_oid": checkpoint,
                    "checkpoint_tree_oid": tree, "task_patch_digest": "a" * 64,
                    "origin": "completing_builder",
                },
            }
            result = ADAPTER.decide(request, "create")
            self.assertEqual("created", result["state"])
            self.assertEqual("D\ttask.txt", run(repository, "diff-tree", "--no-commit-id", "--name-status", "-r", "HEAD"))

    def test_approved_rename_is_authorized_as_delete_and_include(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            repository, head = self.repository(directory)
            (repository / "task.txt").rename(repository / "renamed.txt")
            with tempfile.NamedTemporaryFile() as index:
                env = dict(**__import__("os").environ, GIT_INDEX_FILE=index.name)
                Path(index.name).unlink(missing_ok=True)
                subprocess.run(["git", "-C", str(repository), "read-tree", "HEAD"], env=env, check=True)
                subprocess.run(["git", "-C", str(repository), "add", "-A"], env=env, check=True)
                tree = subprocess.run(
                    ["git", "-C", str(repository), "write-tree"], env=env,
                    text=True, stdout=subprocess.PIPE, check=True,
                ).stdout.strip()
            checkpoint = run(repository, "commit-tree", tree, "-p", head, "-m", "private checkpoint")
            request = {
                "repository": str(repository), "run_id": "test-run", "slice_number": 1,
                "expected_branch": "main", "expected_head_oid": head,
                "accepted_paths": [
                    {"path": "task.txt", "action": "delete"},
                    {"path": "renamed.txt", "action": "include"},
                ],
                "proposal": {
                    "schema_version": 1, "subject": "chore: rename task", "body": "",
                    "paths": ["task.txt", "renamed.txt"], "checkpoint_commit_oid": checkpoint,
                    "checkpoint_tree_oid": tree, "task_patch_digest": "a" * 64,
                    "origin": "completing_builder",
                },
            }
            result = ADAPTER.decide(request, "create")
            self.assertEqual("created", result["state"])
            self.assertEqual(
                ["A\trenamed.txt", "D\ttask.txt"],
                run(repository, "diff-tree", "--no-commit-id", "--name-status", "--no-renames", "-r", "HEAD").splitlines(),
            )

    def test_declined_and_unsafe_states_do_not_mutate(self) -> None:
        for mode in ("decline", "dirty", "staged", "drift", "hook", "signing"):
            with self.subTest(mode=mode), tempfile.TemporaryDirectory() as directory:
                repository, head = self.repository(directory)
                request = self.request(repository, head)
                if mode == "dirty":
                    (repository / "unrelated.txt").write_text("user work\n")
                if mode == "staged":
                    (repository / "staged.txt").write_text("staged user work\n")
                    run(repository, "add", "staged.txt")
                if mode == "drift":
                    (repository / "drift.txt").write_text("drift\n")
                    run(repository, "add", "drift.txt")
                    run(repository, "commit", "-q", "-m", "user commit")
                if mode == "hook":
                    hook = repository / ".git" / "hooks" / "pre-commit"
                    hook.write_text("#!/bin/sh\nexit 1\n")
                    hook.chmod(0o755)
                if mode == "signing":
                    run(repository, "config", "commit.gpgsign", "true")
                before = run(repository, "rev-parse", "HEAD")
                result = ADAPTER.decide(request, "decline" if mode == "decline" else "create")
                self.assertEqual("declined" if mode == "decline" else "refused", result["state"])
                self.assertEqual(before, run(repository, "rev-parse", "HEAD"))

    def test_atomic_publication_does_not_overwrite_concurrent_head(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            repository, head = self.repository(directory)
            request = self.request(repository, head)
            concurrent = request["proposal"]["checkpoint_commit_oid"]
            original_git = ADAPTER.git
            injected = False

            def racing_git(repo, args, **kwargs):
                nonlocal injected
                if not injected and args[:2] == ["update-ref", "refs/heads/main"]:
                    injected = True
                    subprocess.run(
                        ["git", "-C", str(repository), "update-ref", "refs/heads/main", concurrent, head],
                        check=True,
                    )
                return original_git(repo, args, **kwargs)

            with mock.patch.object(ADAPTER, "git", side_effect=racing_git):
                result = ADAPTER.decide(request, "create")
            self.assertEqual("refused", result["state"])
            self.assertIn("atomic publication", result["reason"])
            self.assertEqual(concurrent, run(repository, "rev-parse", "HEAD"))

    def test_postpublication_crash_reconciles_without_replaying_create(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            repository, head = self.repository(directory)
            request = self.request(repository, head)
            published = []

            def crash(commit_oid):
                published.append(commit_oid)
                raise SystemExit("injected post-publication crash")

            with self.assertRaises(SystemExit):
                ADAPTER.decide(request, "create", after_publish=crash)
            recovered = ADAPTER.reconcile_created(request)
            self.assertIsNotNone(recovered)
            self.assertEqual("created", recovered["state"])
            self.assertEqual(published[0], recovered["commit_oid"])
            self.assertEqual(published[0], run(repository, "rev-parse", "HEAD"))

    def test_reconciliation_fails_closed_when_publication_state_is_ambiguous(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            repository, head = self.repository(directory)
            request = self.request(repository, head)
            unrelated = run(
                repository, "commit-tree", request["proposal"]["checkpoint_tree_oid"],
                "-p", head, "-m", "different message",
            )
            run(repository, "update-ref", "refs/heads/main", unrelated, head)
            with self.assertRaisesRegex(ValueError, "ambiguous"):
                ADAPTER.reconcile_created(request)

    def test_path_order_is_not_authority_relevant(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            repository, head = self.repository(directory)
            request = self.request(repository, head)
            request["accepted_paths"] = [
                {"path": "second.txt", "action": "delete"},
                {"path": "task.txt", "action": "include"},
            ]
            request["proposal"]["paths"] = ["task.txt", "second.txt"]
            self.assertEqual("pending", ADAPTER.decide(request, "pending")["state"])

    def test_declined_prior_checkpoint_cannot_enter_slice_only_commit(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            repository, head = self.repository(directory)
            (repository / "slice1.txt").write_text("slice one\n")
            first_request = {
                "schema_version": 1, "repository": str(repository), "run_id": "test-run",
                "slice_number": 1, "candidate_attempt": 1, "baseline_commit_oid": head,
                "parent_checkpoint_commit_oid": None, "plan_version": "plan-1",
                "scope_revision": "scope-1", "gate_receipt_digest": "b" * 64,
                "created_at": "2026-08-21T12:00:00+00:00",
                "paths": [{"path": "slice1.txt", "action": "include", "attribution": "task_owned"}],
            }
            first_candidate = CHECKPOINT.create_candidate(first_request)
            first_binding = {field: first_candidate[field] for field in LIFECYCLE.BINDING_FIELDS}
            first = LIFECYCLE.accept(first_candidate, first_binding, "b" * 64)
            first_completion = {
                "repository": str(repository), "run_id": "test-run", "slice_number": 1,
                "expected_branch": "main", "expected_head_oid": head,
                "accepted_paths": [{"path": "slice1.txt", "action": "include"}],
                "proposal": {
                    "schema_version": 1, "subject": "feat: slice one", "body": "",
                    "paths": ["slice1.txt"], "checkpoint_commit_oid": first["checkpoint_commit_oid"],
                    "checkpoint_tree_oid": first["checkpoint_tree_oid"],
                    "task_patch_digest": first["task_patch_digest"], "origin": "completing_builder",
                },
            }
            self.assertEqual("declined", ADAPTER.decide(first_completion, "decline")["state"])

            (repository / "slice2.txt").write_text("slice two\n")
            second_request = {
                "schema_version": 1, "repository": str(repository), "run_id": "test-run",
                "slice_number": 2, "candidate_attempt": 1,
                "baseline_commit_oid": first["checkpoint_commit_oid"],
                "parent_checkpoint_commit_oid": first["checkpoint_commit_oid"],
                "plan_version": "plan-2", "scope_revision": "scope-2",
                "gate_receipt_digest": "c" * 64,
                "created_at": "2026-08-21T12:01:00+00:00",
                "paths": [{"path": "slice2.txt", "action": "include", "attribution": "task_owned"}],
            }
            second_candidate = CHECKPOINT.create_candidate(second_request)
            second_binding = {field: second_candidate[field] for field in LIFECYCLE.BINDING_FIELDS}
            second = LIFECYCLE.accept(second_candidate, second_binding, "c" * 64)
            second_completion = {
                "repository": str(repository), "run_id": "test-run", "slice_number": 2,
                "expected_branch": "main", "expected_head_oid": head,
                "accepted_paths": [{"path": "slice2.txt", "action": "include"}],
                "proposal": {
                    "schema_version": 1, "subject": "feat: slice two", "body": "",
                    "paths": ["slice2.txt"], "checkpoint_commit_oid": second["checkpoint_commit_oid"],
                    "checkpoint_tree_oid": second["checkpoint_tree_oid"],
                    "task_patch_digest": second["task_patch_digest"], "origin": "completing_builder",
                },
            }
            before_index = run(repository, "write-tree")
            result = ADAPTER.decide(second_completion, "create")
            self.assertEqual("refused", result["state"])
            self.assertIn("complete commit delta", result["reason"])
            self.assertEqual(head, run(repository, "rev-parse", "HEAD"))
            self.assertEqual(before_index, run(repository, "write-tree"))

            second_completion["accepted_paths"] = [
                {"path": "slice1.txt", "action": "include"},
                {"path": "slice2.txt", "action": "include"},
            ]
            second_completion["proposal"]["paths"] = ["slice1.txt", "slice2.txt"]
            aggregate = ADAPTER.decide(second_completion, "create")
            self.assertEqual("created", aggregate["state"])
            self.assertEqual(
                ["slice1.txt", "slice2.txt"],
                run(repository, "diff-tree", "--no-commit-id", "--name-only", "-r", "HEAD").splitlines(),
            )

    def test_created_receipt_is_durable_and_resume_keeps_private_baseline(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            repository, head = self.repository(directory)
            request = self.request(repository, head)
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
            binding = {field: candidate[field] for field in LIFECYCLE.BINDING_FIELDS}
            accepted = LIFECYCLE.accept(candidate, binding, "b" * 64)
            request["proposal"].update(
                checkpoint_commit_oid=accepted["checkpoint_commit_oid"],
                checkpoint_tree_oid=accepted["checkpoint_tree_oid"],
                task_patch_digest=accepted["task_patch_digest"],
            )
            pending = ADAPTER.decide(request, "pending")
            created = ADAPTER.decide(request, "create")
            self.assertEqual("created", created["state"])

            fabricated = dict(created)
            fabricated["repository"] = "/definitely/not/a/repository"
            receipts = Path(directory) / "receipts.jsonl"
            seed = b'{"existing":"user-state"}\n'
            receipts.write_bytes(seed)

            semantic = ASSEMBLER_TESTS.ReceiptAssemblerTest().semantic_receipt()
            semantic.update(
                status="accepted", stop_reason=None, plan_acceptance="procedural_auto_approval",
                validation_requirement_results={"focused_checks": "passed"},
                placement_certificate={"owner": "slice completion commit"},
                placement_verdict="confirmed", placement_risk="high",
                vertical_semantic_test="completion commit vertical",
                vertical_semantic_test_passed=True, more_in_scope_work_remains=True,
            )
            preflight = ASSEMBLER_TESTS.preflight_result(semantic["engine_config"])
            handoff = {
                "slice_title": semantic["slice_title"], "slice_goal": semantic["slice_goal"],
                "outcome": semantic["outcome"], "durable_decisions": [],
                "affected_boundaries": ["ordinary Git history"],
                "placement_certificate": semantic["placement_certificate"],
                "unresolved_concerns": [], "deferred_scope": [],
            }
            with self.assertRaises(ValueError):
                FINALIZER.finalize(
                    receipts, semantic, ASSEMBLER_TESTS.ingress(), preflight, handoff,
                    accepted, fabricated,
                )
            self.assertEqual(seed, receipts.read_bytes())

            with self.assertRaisesRegex(ValueError, "live state"):
                FINALIZER.finalize(
                    receipts, semantic, ASSEMBLER_TESTS.ingress(), preflight, handoff,
                    accepted, pending,
                )
            self.assertEqual(seed, receipts.read_bytes())

            receipts.unlink()

            terminal = FINALIZER.finalize(
                receipts, semantic, ASSEMBLER_TESTS.ingress(), preflight, handoff,
                accepted, created,
            )
            self.assertEqual("created", terminal["producer_metrics"]["slice_completion_commit"]["state"])
            resumed = RESUME.resume(receipts, preflight, "test-run")
            self.assertEqual(accepted["checkpoint_commit_oid"], resumed["baseline_checkpoint"]["checkpoint_commit_oid"])
            self.assertEqual(created["commit_oid"], resumed["slice_completion_commit"]["commit_oid"])


def preservation_request(repository, request, accepted, root):
    """Exact approval inputs prepared before the offer; not builder authority."""
    request = copy.deepcopy(request)
    request.pop("operational_paths", None)
    root = Path(root)
    root.mkdir()
    accepted_path = root / "accepted.json"
    accepted_path.write_bytes(ADAPTER.request_canonical(accepted))
    manifest = root / "preservation.json"
    manifest.write_bytes(ADAPTER.request_canonical(ADAPTER.preservation_snapshot(repository, request["proposal"]["paths"])))
    request["preservation"] = {
        "mode": "preserve_unrelated_v1", "worktree": str(repository),
        "expected_index_sha256": ADAPTER.sha256(ADAPTER.index_path(repository).read_bytes()),
        "accepted_receipt": {"path": str(accepted_path), "sha256": ADAPTER.sha256(accepted_path.read_bytes())},
        "manifest": {"path": str(manifest), "sha256": ADAPTER.sha256(manifest.read_bytes())},
        "approved_delta": ADAPTER.full_delta(repository, request["expected_head_oid"], request["proposal"]["checkpoint_tree_oid"]),
        "attempt_directory": str(root / "attempt"),
    }
    return request


def preservation_fixture(directory):
    fixture = CompletionCommitTest()
    repository, _head = fixture.repository(directory)
    for name in ("outside.txt", "deleted.txt", "executable.txt"):
        (repository / name).write_text("outside baseline\n")
    (repository / "executable.txt").chmod(0o755)
    (repository / "link").symlink_to("outside.txt")
    run(repository, "add", ".")
    run(repository, "commit", "-qm", "outside baseline")
    head = run(repository, "rev-parse", "HEAD")
    request = fixture.request(repository, head)
    request["run_id"] = "test-run"
    candidate = CHECKPOINT.create_candidate({
        "schema_version": 1, "repository": str(repository), "run_id": "test-run", "slice_number": 1,
        "candidate_attempt": 1, "baseline_commit_oid": head, "parent_checkpoint_commit_oid": None,
        "plan_version": "preservation-1", "scope_revision": "task", "gate_receipt_digest": "b" * 64,
        "created_at": "2026-10-14T01:00:00Z",
        "paths": [{"path": "task.txt", "action": "include", "attribution": "task_owned"}],
    })
    accepted = LIFECYCLE.accept(candidate, {k: candidate[k] for k in LIFECYCLE.BINDING_FIELDS}, "b" * 64)
    for k in ("checkpoint_commit_oid", "checkpoint_tree_oid", "task_patch_digest"):
        request["proposal"][k] = accepted[k]
    (repository / "outside.txt").write_text("outside staged\n")
    (repository / "added.txt").write_text("outside addition\n")
    (repository / "deleted.txt").unlink()
    run(repository, "add", "outside.txt", "added.txt", "deleted.txt")
    (repository / "outside.txt").write_text("outside unstaged on staged\n")
    (repository / "untracked").mkdir()
    (repository / "untracked/nested").write_text("retain me\n")
    request = preservation_request(repository, request, accepted, Path(directory) / "evidence")
    return repository, request, accepted


def preservation_terminal_inputs():
    semantic = ASSEMBLER_TESTS.ReceiptAssemblerTest().semantic_receipt()
    semantic.update(status="accepted", stop_reason=None, plan_acceptance="procedural_auto_approval",
        validation_requirement_results={"focused_checks": "passed"}, placement_certificate={"owner": "completion preservation"},
        placement_verdict="confirmed", placement_risk="high", vertical_semantic_test="preservation vertical",
        vertical_semantic_test_passed=True, more_in_scope_work_remains=True)
    preflight = ASSEMBLER_TESTS.preflight_result(semantic["engine_config"])
    handoff = {k: semantic[k] for k in ("slice_title", "slice_goal", "outcome", "placement_certificate")}
    handoff.update(durable_decisions=[], affected_boundaries=["Git publication"], unresolved_concerns=[], deferred_scope=[])
    return semantic, ASSEMBLER_TESTS.ingress(), preflight, handoff


class PreservationCommitTest(unittest.TestCase):
    def test_update_ref_effect_before_unsuccessful_completion_is_created(self):
        for outcome in ("nonzero", "os_error", "value_error"):
            with self.subTest(outcome=outcome), tempfile.TemporaryDirectory() as directory:
                repository, request, _accepted = preservation_fixture(directory)
                original_index = ADAPTER.index_path(repository).read_bytes()
                before = ADAPTER.preservation_snapshot(repository, request["proposal"]["paths"])
                original_git = ADAPTER.git
                attempts = []

                def effect_then_error(repo, args, **kwargs):
                    result = original_git(repo, args, **kwargs)
                    if args[0] == "update-ref":
                        attempts.append(args)
                        self.assertEqual(0, result.returncode, result.stderr)
                        self.assertEqual(args[2], run(repository, "rev-parse", "HEAD"))
                        if outcome == "os_error":
                            raise OSError("synthetic lost command completion")
                        if outcome == "value_error":
                            raise ValueError("synthetic lost command completion")
                        return subprocess.CompletedProcess(result.args, 1, result.stdout, "synthetic lost command completion")
                    return result

                with mock.patch.object(ADAPTER, "git", side_effect=effect_then_error):
                    result = ADAPTER.decide(request, "create")
                self.assertEqual(1, len(attempts))
                self.assertEqual("created", result["state"], result)
                self.assertEqual(run(repository, "rev-parse", "HEAD"), result["commit_oid"])
                self.assertEqual("unestablished", result["evidence"]["index_transition"])
                self.assertEqual("unestablished", result["evidence"]["preservation"])
                self.assertEqual(result, ADAPTER.validate_receipt(result))
                self.assertEqual(original_index, ADAPTER.index_path(repository).read_bytes())
                self.assertEqual(before, ADAPTER.preservation_snapshot(repository, request["proposal"]["paths"]))
                lock = Path(str(ADAPTER.index_path(repository)) + ".lock")
                self.assertTrue(lock.exists())
                lock_bytes = lock.read_bytes()
                recovered = ADAPTER.reconcile_created(request)
                self.assertEqual(result["commit_oid"], recovered["commit_oid"])
                self.assertEqual(original_index, ADAPTER.index_path(repository).read_bytes())
                self.assertEqual(lock_bytes, lock.read_bytes())

    def test_update_ref_uncertainty_retains_prepared_artifacts_without_refusal(self):
        for outcome in ("no_effect", "read_failure_after_effect"):
            with self.subTest(outcome=outcome), tempfile.TemporaryDirectory() as directory:
                repository, request, _accepted = preservation_fixture(directory)
                original_index = ADAPTER.index_path(repository).read_bytes()
                original_git = ADAPTER.git
                attempted = False
                attempts = []

                def unknown_completion(repo, args, **kwargs):
                    nonlocal attempted
                    if args[0] == "update-ref":
                        attempted = True
                        attempts.append(args)
                        if outcome == "read_failure_after_effect":
                            self.assertEqual(0, original_git(repo, args, **kwargs).returncode)
                        return subprocess.CompletedProcess(args, 1, "", "synthetic unknown completion")
                    if attempted and outcome == "read_failure_after_effect" and args == ["rev-parse", "refs/heads/main"]:
                        raise OSError("synthetic observation unavailable")
                    return original_git(repo, args, **kwargs)

                with mock.patch.object(ADAPTER, "git", side_effect=unknown_completion):
                    with self.assertRaisesRegex(ValueError, "publication ambiguous"):
                        ADAPTER.decide(request, "create")
                self.assertEqual(1, len(attempts))
                self.assertEqual(original_index, ADAPTER.index_path(repository).read_bytes())
                self.assertTrue(Path(str(ADAPTER.index_path(repository)) + ".lock").exists())
                attempt = Path(request["preservation"]["attempt_directory"])
                for name in ("original.index", "proposed.index", "prepared.json"):
                    self.assertTrue((attempt / name).is_file())
                self.assertFalse((attempt / "result.json").exists())
                expected = request["expected_head_oid"] if outcome == "no_effect" else attempts[0][2]
                self.assertEqual(expected, run(repository, "rev-parse", "HEAD"))

    def test_published_prepared_commit_rejects_noncreated_or_wrong_commit_cache(self):
        for state in ("pending", "refused", "declined", "wrong_commit"):
            with self.subTest(state=state), tempfile.TemporaryDirectory() as directory:
                repository, request, _accepted = preservation_fixture(directory)
                created = ADAPTER.decide(request, "create")
                self.assertEqual("created", created["state"], created)
                if state == "wrong_commit":
                    cached = copy.deepcopy(created)
                    cached["commit_oid"] = request["expected_head_oid"]
                else:
                    cached = ADAPTER.receipt(request, state, reason=None if state == "pending" else "synthetic cached nonpublication")
                self.assertEqual(cached, ADAPTER.validate_receipt_shape(cached))
                result_path = Path(request["preservation"]["attempt_directory"]) / "result.json"
                result_path.write_bytes(ADAPTER.request_canonical(cached))
                before = ADAPTER.preservation_snapshot(repository, request["proposal"]["paths"])
                index = ADAPTER.index_path(repository).read_bytes()
                saved_cache = result_path.read_bytes()
                with self.assertRaisesRegex(ValueError, "contradicts observed prepared publication"):
                    ADAPTER.reconcile_created(request)
                self.assertEqual(created["commit_oid"], run(repository, "rev-parse", "HEAD"))
                self.assertEqual(index, ADAPTER.index_path(repository).read_bytes())
                self.assertEqual(before, ADAPTER.preservation_snapshot(repository, request["proposal"]["paths"]))
                self.assertEqual(saved_cache, result_path.read_bytes())

    def test_vertical_offer_commit_finalizer_fresh_resume_preserves_user_state(self):
        with tempfile.TemporaryDirectory() as directory:
            repository, request, accepted = preservation_fixture(directory)
            before = ADAPTER.preservation_snapshot(repository, request["proposal"]["paths"])
            decision = {"decision": "create", "authority": {"kind": "human", "reference": "synthetic-user-grant", "observed_at": "2026-10-14T01:00:00Z"}}
            authorized = OFFER.resolve(OFFER.open_offer(request), decision)
            created = ADAPTER.decide(request, "create")
            self.assertEqual("created", created["state"], created)
            self.assertIsNone(created["reason"], created)
            self.assertEqual("verified", created["evidence"]["preservation"])
            self.assertEqual(created, ADAPTER.validate_receipt(created))
            terminal_offer = OFFER.transition(authorized, "created", result=created)
            self.assertEqual(created, terminal_offer["result"])
            after = ADAPTER.preservation_snapshot(repository, request["proposal"]["paths"])
            self.assertEqual(before["working"], after["working"])
            self.assertEqual(ADAPTER.expected_index(before["index_entries"], request["preservation"]["approved_delta"]), after["index_entries"])
            self.assertEqual("", run(repository, "status", "--porcelain", "--", "task.txt"))
            self.assertEqual("Test User", run(repository, "show", "-s", "--format=%an", "HEAD"))
            semantic = ASSEMBLER_TESTS.ReceiptAssemblerTest().semantic_receipt()
            semantic.update(status="accepted", stop_reason=None, plan_acceptance="procedural_auto_approval",
                validation_requirement_results={"focused_checks": "passed"}, placement_certificate={"owner": "completion preservation"},
                placement_verdict="confirmed", placement_risk="high", vertical_semantic_test="preservation vertical",
                vertical_semantic_test_passed=True, more_in_scope_work_remains=True)
            preflight = ASSEMBLER_TESTS.preflight_result(semantic["engine_config"])
            handoff = {k: semantic[k] for k in ("slice_title", "slice_goal", "outcome", "placement_certificate")}
            handoff.update(durable_decisions=[], affected_boundaries=["Git publication"], unresolved_concerns=[], deferred_scope=[])
            metrics = Path(directory) / "terminal.jsonl"
            FINALIZER.finalize(metrics, semantic, ASSEMBLER_TESTS.ingress(), preflight, handoff, accepted, created)
            resumed = RESUME.resume(metrics, preflight, "test-run")
            self.assertEqual(created, resumed["slice_completion_commit"])
            # A new interpreter reads persisted offer state through the real owner.
            code = "import importlib.util,json,sys; from pathlib import Path; s=importlib.util.spec_from_file_location('o',sys.argv[1]); m=importlib.util.module_from_spec(s); s.loader.exec_module(m); print(json.dumps(m.load(Path(sys.argv[2]),'test-run',1)))"
            fresh = subprocess.run([__import__("sys").executable, "-B", "-c", code, str(WORK_ENGINE_ROOT / "skills/slice-supervisor/scripts/completion_offer_lifecycle.py"), str(repository)], capture_output=True, text=True)
            self.assertEqual(0, fresh.returncode, fresh.stderr)
            self.assertEqual(created, json.loads(fresh.stdout)["result"])

    def test_crash_windows_reconcile_exact_commit_without_replay_or_index_repair(self):
        for cut in ("before_prepared", "prepared", "published", "index_installed", "receipt_written"):
            with self.subTest(cut=cut), tempfile.TemporaryDirectory() as directory:
                repository, request, _accepted = preservation_fixture(directory)
                original = ADAPTER.index_path(repository).read_bytes()
                def crash(phase):
                    if phase == cut:
                        raise SystemExit("synthetic crash: " + cut)
                with mock.patch.object(ADAPTER, "phase_boundary", side_effect=crash):
                    with self.assertRaises(SystemExit):
                        ADAPTER.decide(request, "create")
                head = run(repository, "rev-parse", "HEAD")
                index = ADAPTER.index_path(repository).read_bytes()
                lock = Path(str(ADAPTER.index_path(repository)) + ".lock")
                lock_before = lock.read_bytes() if lock.exists() else None
                recovered = ADAPTER.reconcile_created(request)
                if cut in {"before_prepared", "prepared"}:
                    self.assertIsNone(recovered)
                    self.assertEqual(request["expected_head_oid"], head)
                else:
                    self.assertEqual(head, recovered["commit_oid"])
                    self.assertEqual("verified" if cut == "receipt_written" else "unestablished", recovered["evidence"]["preservation"])
                if cut == "published":
                    self.assertEqual(original, index)
                    self.assertTrue(lock.exists())
                self.assertEqual(head, run(repository, "rev-parse", "HEAD"))
                self.assertEqual(index, ADAPTER.index_path(repository).read_bytes())
                self.assertEqual(lock_before, lock.read_bytes() if lock.exists() else None)

    def test_refusals_preserve_branch_index_and_foreign_lock(self):
        cases = ("index", "source", "manifest", "checkpoint", "delta", "partial", "lock", "signing", "pre-commit", "prepare-commit-msg", "commit-msg", "post-commit", "filter", "intent", "skip", "assume", "attempt_reuse")
        for case in cases:
            with self.subTest(case=case), tempfile.TemporaryDirectory() as directory:
                repository, request, _accepted = preservation_fixture(directory)
                if case == "index":
                    run(repository, "add", "outside.txt")
                elif case == "source":
                    (repository / "untracked/nested").write_text("concurrent")
                elif case in {"manifest", "checkpoint"}:
                    key = "manifest" if case == "manifest" else "accepted_receipt"
                    Path(request["preservation"][key]["path"]).write_text("{}")
                elif case == "delta":
                    request["preservation"]["approved_delta"][0]["after"]["oid"] = "a" * 40
                elif case in {"partial", "intent", "skip", "assume"}:
                    if case == "partial":
                        (repository / "task.txt").write_text("other staged bytes")
                        run(repository, "add", "task.txt")
                        (repository / "task.txt").write_text("accepted slice\n")
                    elif case == "intent":
                        (repository / "intent").write_text("intent")
                        run(repository, "add", "-N", "intent")
                    else:
                        run(repository, "update-index", "--skip-worktree" if case == "skip" else "--assume-unchanged", "outside.txt")
                    request["preservation"]["expected_index_sha256"] = ADAPTER.sha256(ADAPTER.index_path(repository).read_bytes())
                    if case == "partial":
                        ref = request["preservation"]["manifest"]
                        Path(ref["path"]).write_bytes(ADAPTER.request_canonical(ADAPTER.preservation_snapshot(repository, ["task.txt"])))
                        ref["sha256"] = ADAPTER.sha256(Path(ref["path"]).read_bytes())
                elif case == "lock":
                    Path(str(ADAPTER.index_path(repository)) + ".lock").write_text("foreign lock")
                elif case == "signing":
                    run(repository, "config", "commit.gpgsign", "true")
                elif case == "filter":
                    (repository / ".git/info/attributes").write_text("task.txt filter=unsafe\n")
                elif case == "attempt_reuse":
                    Path(request["preservation"]["attempt_directory"]).mkdir()
                else:
                    hook = repository / ".git/hooks" / case
                    hook.write_text("#!/bin/sh\nexit 1\n")
                    hook.chmod(0o755)
                original = ADAPTER.index_path(repository).read_bytes()
                result = ADAPTER.decide(request, "create")
                self.assertEqual("refused", result["state"], result)
                self.assertEqual(request["expected_head_oid"], run(repository, "rev-parse", "HEAD"))
                self.assertEqual(original, ADAPTER.index_path(repository).read_bytes())
                if case == "lock":
                    self.assertEqual("foreign lock", Path(str(ADAPTER.index_path(repository)) + ".lock").read_text())

    def test_races_and_postpublication_uncertainty_are_not_false_refusals(self):
        for cut in ("source", "index", "config", "attachment", "cas", "post_source"):
            with self.subTest(cut=cut), tempfile.TemporaryDirectory() as directory:
                repository, request, _accepted = preservation_fixture(directory)
                initial = ADAPTER.index_path(repository).read_bytes()
                def change(phase):
                    if phase == ("published" if cut == "post_source" else "prepared"):
                        if cut in {"source", "post_source"}:
                            (repository / "outside.txt").write_text("racing writer")
                        elif cut == "index":
                            with ADAPTER.index_path(repository).open("ab") as stream:
                                stream.write(b"concurrent")
                        elif cut == "config":
                            run(repository, "config", "user.name", "Other User")
                        elif cut == "attachment":
                            run(repository, "symbolic-ref", "HEAD", "refs/heads/other")
                        else:
                            run(repository, "update-ref", "refs/heads/main", request["proposal"]["checkpoint_commit_oid"], request["expected_head_oid"])
                with mock.patch.object(ADAPTER, "phase_boundary", side_effect=change):
                    result = ADAPTER.decide(request, "create")
                self.assertEqual("created" if cut == "post_source" else "refused", result["state"])
                if cut == "post_source":
                    self.assertEqual("unestablished", result["evidence"]["preservation"])
                    self.assertEqual(initial, ADAPTER.index_path(repository).read_bytes())
                    self.assertEqual(result, ADAPTER.validate_receipt(result))

    def test_readback_rejects_forgery_and_recovers_prepared_ancestor(self):
        with tempfile.TemporaryDirectory() as directory:
            repository, request, _accepted = preservation_fixture(directory)
            result = ADAPTER.decide(request, "create")
            self.assertEqual("created", result["state"], result)
            descendant = run(repository, "commit-tree", request["proposal"]["checkpoint_tree_oid"], "-p", result["commit_oid"], "-m", "later history")
            run(repository, "update-ref", "refs/heads/main", descendant, result["commit_oid"])
            self.assertEqual(result, ADAPTER.reconcile_created(request))
            for key in ("commit_oid", "proposal_digest"):
                forged = copy.deepcopy(result)
                forged[key] = "a" * len(forged[key])
                with self.assertRaises(ValueError):
                    ADAPTER.validate_receipt(forged)
            post = Path(result["evidence"]["post"]["path"])
            post.write_text("{}")
            with self.assertRaisesRegex(ValueError, "digest"):
                ADAPTER.validate_receipt(result)


if __name__ == "__main__":
    unittest.main()
