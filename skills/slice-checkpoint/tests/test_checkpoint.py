from __future__ import annotations

import hashlib
import importlib.util
import json
import os
import subprocess
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


CHECKPOINT = load("checkpoint_vertical_adapter", ROOT / "scripts" / "checkpoint.py")
LIFECYCLE = load(
    "checkpoint_vertical_lifecycle",
    WORK_ENGINE_ROOT / "skills" / "slice-supervisor" / "scripts" / "checkpoint_lifecycle.py",
)
FINALIZER = load(
    "checkpoint_vertical_finalizer",
    WORK_ENGINE_ROOT / "skills" / "slice-supervisor" / "scripts" / "finalize_receipt.py",
)
RESUME = load(
    "checkpoint_vertical_resume",
    WORK_ENGINE_ROOT / "skills" / "slice-supervisor" / "scripts" / "resume_campaign.py",
)
ASSEMBLER_TESTS = load(
    "checkpoint_vertical_assembler_fixture",
    WORK_ENGINE_ROOT / "skills" / "slice-supervisor" / "tests" / "test_assemble_receipt.py",
)


def run(repository: Path, *args: str) -> str:
    return subprocess.run(
        ["git", "-C", str(repository), *args], check=True,
        stdout=subprocess.PIPE, text=True,
    ).stdout.strip()


class CheckpointTest(unittest.TestCase):
    def make_repository(self, directory: str) -> tuple[Path, str]:
        repository = Path(directory) / "repository"
        repository.mkdir()
        run(repository, "init", "-q", "-b", "main")
        run(repository, "config", "user.name", "Test User")
        run(repository, "config", "user.email", "test@example.com")
        (repository / ".gitignore").write_text("ignored.txt\n.env\n", encoding="utf-8")
        (repository / "task.txt").write_text("baseline\n", encoding="utf-8")
        (repository / "overlap.txt").write_text("baseline overlap\n", encoding="utf-8")
        run(repository, "add", ".gitignore", "task.txt", "overlap.txt")
        run(repository, "commit", "-q", "-m", "baseline")
        return repository, run(repository, "rev-parse", "HEAD")

    def snapshot(self, repository: Path) -> dict[str, object]:
        index = Path(run(repository, "rev-parse", "--git-path", "index"))
        if not index.is_absolute():
            index = repository / index
        files = {
            path.relative_to(repository).as_posix(): hashlib.sha256(path.read_bytes()).hexdigest()
            for path in repository.rglob("*")
            if path.is_file() and ".git" not in path.relative_to(repository).parts
        }
        return {
            "branch": run(repository, "branch", "--show-current"),
            "head": run(repository, "rev-parse", "HEAD"),
            "index": hashlib.sha256(index.read_bytes()).hexdigest(),
            "status": run(repository, "status", "--porcelain=v2", "--untracked-files=all"),
            "files": files,
        }

    def request(self, repository: Path, baseline: str, attempt: int,
                parent: str | None = None) -> dict[str, object]:
        return {
            "schema_version": 1, "repository": str(repository), "run_id": "test-run",
            "slice_number": 1, "candidate_attempt": attempt,
            "baseline_commit_oid": baseline, "parent_checkpoint_commit_oid": parent,
            "plan_version": "plan-1", "scope_revision": "scope-1",
            "gate_receipt_digest": "a" * 64, "created_at": "2026-08-20T12:00:00+00:00",
            "paths": [
                {"path": "task.txt", "action": "include", "attribution": "task_owned"},
                {"path": "overlap.txt", "action": "include", "attribution": "pre_existing_overlap"},
            ],
        }

    def test_candidate_repair_accept_resume_preserves_user_git_state(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            repository, baseline = self.make_repository(directory)
            (repository / "user-staged.txt").write_text("staged user state\n", encoding="utf-8")
            run(repository, "add", "user-staged.txt")
            (repository / "overlap.txt").write_text("declared dirty overlap\n", encoding="utf-8")
            (repository / "unrelated.txt").write_text("unrelated\n", encoding="utf-8")
            (repository / "ignored.txt").write_text("ignored\n", encoding="utf-8")
            (repository / ".env").write_text("SECRET=value\n", encoding="utf-8")
            (repository / "task.txt").write_text("candidate zero\n", encoding="utf-8")
            before = self.snapshot(repository)

            candidate0 = CHECKPOINT.create_candidate(self.request(repository, baseline, 1))
            self.assertEqual(before, self.snapshot(repository))
            tree_paths = run(repository, "ls-tree", "-r", "--name-only", candidate0["checkpoint_tree_oid"])
            self.assertNotIn("unrelated.txt", tree_paths)
            self.assertNotIn("ignored.txt", tree_paths)
            self.assertNotIn(".env", tree_paths)
            overlap = next(item for item in candidate0["paths"] if item["path"] == "overlap.txt")
            self.assertEqual("pre_existing_overlap", overlap["attribution"])

            forged = dict(candidate0, plan_version="forged-plan")
            forged_binding = {field: forged[field] for field in LIFECYCLE.BINDING_FIELDS}
            with self.assertRaisesRegex(ValueError, "immutable commit metadata"):
                LIFECYCLE.accept(forged, forged_binding, "a" * 64)

            binding = {field: candidate0[field] for field in LIFECYCLE.BINDING_FIELDS}
            bad = dict(binding, checkpoint_tree_oid="f" * 40)
            with self.assertRaisesRegex(ValueError, "checkpoint_tree_oid"):
                LIFECYCLE.accept(candidate0, bad, "a" * 64)

            (repository / "task.txt").write_text("candidate one repaired\n", encoding="utf-8")
            before_repair = self.snapshot(repository)
            candidate1 = CHECKPOINT.create_candidate(
                self.request(repository, baseline, 2, candidate0["checkpoint_commit_oid"])
            )
            self.assertEqual(before_repair, self.snapshot(repository))
            self.assertNotEqual(candidate0["checkpoint_tree_oid"], candidate1["checkpoint_tree_oid"])
            self.assertEqual(
                candidate0["checkpoint_commit_oid"],
                run(repository, "rev-parse", candidate0["ref"]),
            )

            binding = {field: candidate1[field] for field in LIFECYCLE.BINDING_FIELDS}
            accepted = LIFECYCLE.accept(candidate1, binding, "a" * 64)
            forged_accepted = json.loads(json.dumps(accepted))
            forged_accepted["paths"][0]["attribution"] = "validation_dependency"
            with self.assertRaisesRegex(ValueError, "path attribution manifest"):
                LIFECYCLE.projection(forged_accepted)
            repeated = LIFECYCLE.accept(candidate1, binding, "a" * 64)
            self.assertEqual(accepted["checkpoint_commit_oid"], repeated["checkpoint_commit_oid"])
            old_binding = {field: candidate0[field] for field in LIFECYCLE.BINDING_FIELDS}
            with self.assertRaisesRegex(ValueError, "ref conflict"):
                LIFECYCLE.accept(candidate0, old_binding, "a" * 64)
            stopped = LIFECYCLE.stop(candidate0)
            stopped_repair = LIFECYCLE.stop(candidate1)
            self.assertNotEqual(stopped["ref"], stopped_repair["ref"])
            self.assertEqual(accepted["checkpoint_commit_oid"], run(repository, "rev-parse", accepted["ref"]))
            self.assertNotEqual(stopped["checkpoint_commit_oid"], run(repository, "rev-parse", accepted["ref"]))
            self.assertEqual(before_repair, self.snapshot(repository))

            semantic = ASSEMBLER_TESTS.ReceiptAssemblerTest().semantic_receipt()
            preflight = ASSEMBLER_TESTS.preflight_result(semantic["engine_config"])
            semantic.update(
                status="accepted", stop_reason=None,
                plan_acceptance="procedural_auto_approval",
                validation_requirement_results={
                    requirement: "passed"
                    for requirement in semantic["engine_config"]["validation"]["requirements"]
                },
                placement_certificate={"owner": "slice checkpoint"},
                placement_verdict="confirmed", placement_risk="high",
                vertical_semantic_test="checkpoint lifecycle",
                vertical_semantic_test_passed=True,
                more_in_scope_work_remains=True,
            )
            handoff = {
                "slice_title": semantic["slice_title"], "slice_goal": semantic["slice_goal"],
                "outcome": semantic["outcome"], "durable_decisions": [],
                "affected_boundaries": ["slice checkpoint"],
                "placement_certificate": semantic["placement_certificate"],
                "unresolved_concerns": [], "deferred_scope": [],
            }
            receipts = repository.parent / "receipts.jsonl"
            terminal = FINALIZER.finalize(
                receipts, semantic, ASSEMBLER_TESTS.ingress(), preflight, handoff, accepted
            )
            self.assertEqual(
                accepted["checkpoint_commit_oid"],
                terminal["producer_metrics"]["slice_checkpoint"]["checkpoint_commit_oid"],
            )
            resumed = RESUME.resume(receipts, preflight, "test-run")
            self.assertEqual(
                accepted["checkpoint_commit_oid"],
                resumed["baseline_checkpoint"]["checkpoint_commit_oid"],
            )
            record = json.loads(receipts.read_text(encoding="utf-8"))
            record["producer_metrics"]["slice_checkpoint"]["plan_version"] = "forged-plan"
            receipts.write_text(json.dumps(record) + "\n", encoding="utf-8")
            with self.assertRaisesRegex(ValueError, "immutable commit metadata"):
                RESUME.resume(receipts, preflight, "test-run")
            record["producer_metrics"]["slice_checkpoint"]["plan_version"] = "plan-1"
            receipts.write_text(json.dumps(record) + "\n", encoding="utf-8")
            run(repository, "update-ref", accepted["ref"], candidate0["checkpoint_commit_oid"])
            with self.assertRaisesRegex(ValueError, "private ref does not match"):
                RESUME.resume(receipts, preflight, "test-run")
            self.assertEqual(before_repair, self.snapshot(repository))

    def test_rejects_ignored_sensitive_and_ref_conflicts(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            repository, baseline = self.make_repository(directory)
            (repository / ".env").write_text("secret\n", encoding="utf-8")
            request = self.request(repository, baseline, 1)
            request["paths"] = [
                {"path": ".env", "action": "include", "attribution": "task_owned"}
            ]
            with self.assertRaisesRegex(ValueError, "sensitive-looking"):
                CHECKPOINT.create_candidate(request)

            outside = repository.parent / "outside"
            outside.mkdir()
            (outside / "secret.txt").write_text("outside\n", encoding="utf-8")
            os.symlink(outside, repository / "linked-directory")
            request = self.request(repository, baseline, 1)
            request["paths"] = [
                {
                    "path": "linked-directory/secret.txt", "action": "include",
                    "attribution": "task_owned",
                }
            ]
            with self.assertRaisesRegex(ValueError, "unsafe directory"):
                CHECKPOINT.create_candidate(request)


if __name__ == "__main__":
    unittest.main()
