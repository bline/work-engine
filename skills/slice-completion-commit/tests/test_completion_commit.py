from __future__ import annotations

import importlib.util
import copy
import hashlib
import json
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


if __name__ == "__main__":
    unittest.main()
