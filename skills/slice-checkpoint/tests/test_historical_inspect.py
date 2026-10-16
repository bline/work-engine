from __future__ import annotations

import importlib.util
import json
import os
import subprocess
import tempfile
import unittest
from pathlib import Path


SCRIPT = Path(__file__).parents[1] / "scripts/checkpoint.py"
SPEC = importlib.util.spec_from_file_location("historical_checkpoint", SCRIPT)
assert SPEC and SPEC.loader
CHECKPOINT = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(CHECKPOINT)


def git(repository: Path, *args: str, input_bytes: bytes | None = None) -> str:
    env = dict(os.environ, GIT_AUTHOR_NAME="Test", GIT_AUTHOR_EMAIL="test@example.com",
               GIT_COMMITTER_NAME="Test", GIT_COMMITTER_EMAIL="test@example.com")
    return subprocess.run(["git", "-C", str(repository), *args], input=input_bytes,
                          check=True, stdout=subprocess.PIPE, stderr=subprocess.PIPE, env=env).stdout.decode().strip()


class HistoricalCheckpointInspectTest(unittest.TestCase):
    def fixture(self, directory: str, *, wrong_tree: bool = False):
        repository = Path(directory) / "repository"
        repository.mkdir()
        git(repository, "init", "-q")
        tree = git(repository, "mktree", input_bytes=b"")
        baseline = git(repository, "commit-tree", tree, input_bytes=b"baseline\n")
        metadata = {"work_engine_checkpoint": 1, "kind": "candidate", "run_id": "skills-migration-app-server-20260828",
                    "slice_number": 6, "candidate_attempt": 3, "tree": "0" * 40 if wrong_tree else tree,
                    "gate_receipt_digest": "a" * 64, "manifest_digest": "b" * 64,
                    "task_patch_digest": "c" * 64, "plan_version": "plan", "scope_revision": "scope"}
        oid = git(repository, "commit-tree", tree, "-p", baseline,
                  input_bytes=(json.dumps(metadata) + "\n").encode())
        ref = "refs/work-engine/checkpoints/skills-migration-app-server-20260828/slice-6/candidate-3"
        git(repository, "update-ref", ref, oid)
        return repository, ref, oid

    def test_exact_ref_commit_and_metadata_without_worktree_mutation(self):
        with tempfile.TemporaryDirectory() as directory:
            repository, ref, oid = self.fixture(directory)
            before = git(repository, "status", "--porcelain=v2", "--untracked-files=all")
            result = CHECKPOINT.inspect(repository, ref, oid)
            self.assertEqual(oid, result["commit_oid"])
            self.assertEqual("a" * 64, result["metadata"]["gate_receipt_digest"])
            self.assertEqual(before, git(repository, "status", "--porcelain=v2", "--untracked-files=all"))
            with self.assertRaisesRegex(ValueError, "outside"):
                CHECKPOINT.inspect(repository, ref.replace("slice-6", "slice-5"), oid)
            with self.assertRaisesRegex(ValueError, "does not match"):
                CHECKPOINT.inspect(repository, ref, result["parent_commit_oid"])

    def test_commit_message_tree_mismatch_refused(self):
        with tempfile.TemporaryDirectory() as directory:
            repository, ref, oid = self.fixture(directory, wrong_tree=True)
            with self.assertRaisesRegex(ValueError, "metadata"):
                CHECKPOINT.inspect(repository, ref, oid)

    def test_replace_ref_cannot_substitute_admitted_commit(self):
        with tempfile.TemporaryDirectory() as directory:
            repository, ref, oid = self.fixture(directory)
            original = CHECKPOINT.inspect(repository, ref, oid)
            other_tree = git(repository, "mktree", input_bytes=f"100644 blob {git(repository, 'hash-object', '-w', '--stdin', input_bytes=b'alternate')}\tother\n".encode())
            metadata = {**original["metadata"], "tree": other_tree}
            alternate = git(repository, "commit-tree", other_tree, "-p", original["parent_commit_oid"],
                            input_bytes=(json.dumps(metadata) + "\n").encode())
            git(repository, "replace", oid, alternate)
            self.assertEqual(other_tree, git(repository, "show", "-s", "--format=%T", oid))
            result = CHECKPOINT.inspect(repository, ref, oid)
            self.assertEqual(original["tree_oid"], result["tree_oid"])
            self.assertEqual(oid, result["commit_oid"])

    def test_shared_commit_reader_accepts_sha256_repository_objects(self):
        with tempfile.TemporaryDirectory() as directory:
            repository = Path(directory) / "sha256"
            repository.mkdir()
            git(repository, "init", "-q", "--object-format=sha256")
            tree = git(repository, "mktree", input_bytes=b"")
            oid = git(repository, "commit-tree", tree, input_bytes=b'{"case":"sha256"}\n')
            self.assertEqual(64, len(oid))
            self.assertEqual(oid, CHECKPOINT.validate_oid(repository, oid, "sha256_commit"))
            read_tree, parents, metadata = CHECKPOINT.commit_record(repository, oid)
            self.assertEqual((tree, [], {"case": "sha256"}), (read_tree, parents, metadata))
            (repository / "artifact.txt").write_text("sha256 checkpoint\n")
            request = {
                "schema_version": 1, "repository": str(repository), "run_id": "sha256-format-test",
                "slice_number": 1, "candidate_attempt": 1, "baseline_commit_oid": oid,
                "parent_checkpoint_commit_oid": None, "plan_version": "plan", "scope_revision": "scope",
                "gate_receipt_digest": "a" * 64, "created_at": "2026-10-16T00:00:00Z",
                "paths": [{"path": "artifact.txt", "action": "include", "attribution": "task_owned"}],
            }
            candidate = CHECKPOINT.create_candidate(request)
            self.assertEqual(64, len(candidate["checkpoint_commit_oid"]))
            CHECKPOINT.validate_candidate_receipt(candidate)
            accepted = CHECKPOINT.transition(candidate, "accepted")
            CHECKPOINT.validate_lifecycle_receipt(accepted, "accepted")


if __name__ == "__main__":
    unittest.main()
