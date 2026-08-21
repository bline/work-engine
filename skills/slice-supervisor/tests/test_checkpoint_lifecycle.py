from __future__ import annotations

import importlib.util
import unittest
from unittest import mock
from pathlib import Path


ROOT = Path(__file__).parents[1]


def load_module(name: str, path: Path):
    spec = importlib.util.spec_from_file_location(name, path)
    assert spec and spec.loader
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


LIFECYCLE = load_module(
    "test_checkpoint_lifecycle_module", ROOT / "scripts" / "checkpoint_lifecycle.py"
)


class CheckpointLifecycleTest(unittest.TestCase):
    def candidate(self):
        return {
            "schema_version": 1, "checkpoint_id": "b" * 64,
            "checkpoint_kind": "candidate", "repository": "/tmp/repository",
            "run_id": "run", "slice_number": 1, "candidate_attempt": 1,
            "checkpoint_commit_oid": "1" * 40, "checkpoint_tree_oid": "2" * 40,
            "task_patch_digest": "3" * 64, "manifest_digest": "5" * 64,
            "plan_version": "plan-1",
            "scope_revision": "scope-1", "gate_receipt_digest": "4" * 64,
        }

    def test_binding_rejects_every_mismatch(self) -> None:
        candidate = self.candidate()
        binding = {field: candidate[field] for field in LIFECYCLE.BINDING_FIELDS}
        with mock.patch.object(LIFECYCLE.CHECKPOINT, "validate_candidate_receipt"):
            LIFECYCLE.validate_binding(candidate, binding, "4" * 64)
            for field in LIFECYCLE.BINDING_FIELDS:
                with self.subTest(field=field):
                    mismatched = dict(binding)
                    mismatched[field] = "different"
                    with self.assertRaisesRegex(ValueError, field):
                        LIFECYCLE.validate_binding(candidate, mismatched, "4" * 64)

    def test_gate_evidence_must_match_candidate(self) -> None:
        candidate = self.candidate()
        binding = {field: candidate[field] for field in LIFECYCLE.BINDING_FIELDS}
        with mock.patch.object(LIFECYCLE.CHECKPOINT, "validate_candidate_receipt"):
            with self.assertRaisesRegex(ValueError, "deterministic evidence"):
                LIFECYCLE.validate_binding(candidate, binding, "5" * 64)


if __name__ == "__main__":
    unittest.main()
