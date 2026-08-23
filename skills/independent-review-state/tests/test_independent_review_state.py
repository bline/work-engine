from __future__ import annotations

import importlib.util
import subprocess
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).parents[1]
SPEC = importlib.util.spec_from_file_location(
    "independent_review_state_subject", ROOT / "scripts/independent_review_state.py")
assert SPEC and SPEC.loader
STATE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(STATE)

DIGEST = "a" * 64


def reference(owner="checkpoint", name="subject", revision="r1"):
    return {"owner": owner, "reference": name, "revision": revision,
            "integrity_sha256": DIGEST, "freshness_rule": "exact_revision"}


def authority(generation=1, predecessor=None, episode="episode-1"):
    return {"schema_version": 1, "profile": STATE.PROFILE, "grant_id": f"grant-{generation}",
            "identity": {"run_id": "run", "slice_number": 1, "attempt_id": "attempt",
                         "plan_version": "plan", "review_obligation_id": "review-1",
                         "review_episode_id": episode},
            "source": reference("human-authority", "accepted-review-profile", "decision-1"),
            "writer": {"logical_actor_id": "reviewer", "provider": "claude",
                       "generation": generation,
                       "runtime_session_ref": reference("claude-runtime", f"session-{generation}", f"g{generation}")},
            "readers": ["independent_reviewer", "coordinating_builder", "slice_supervisor"],
            "initial_subject": [reference()], "expires_at": None,
            "predecessor_revision": predecessor}


class IndependentReviewStateTest(unittest.TestCase):
    def test_recovery_history_and_writer_fence(self):
        with tempfile.TemporaryDirectory() as directory:
            repository = Path(directory)
            subprocess.run(["git", "init", "-q", str(repository)], check=True)
            store = STATE.DURABLE.GitRefDurableStateStore(repository, namespace="independent-review-state")
            first_authority = authority()
            first = STATE.begin(store, first_authority, STATE.digest(first_authority), {
                "transition_id": "begin-1", "evidence_references": [],
                "claim_references": [], "unresolved_questions": []})
            finding = {"finding_id": "finding-1", "attributed_reviewer": "reviewer",
                       "reviewer_generation": 1, "severity": "high", "observation": "bounded issue",
                       "evidence_references": [reference("repository", "file.py", "blob-1")],
                       "status": "open", "remediation_references": []}
            reviewed = STATE.advance(store, first_authority, STATE.digest(first_authority),
                first["durable_revision"], "record_initial_result", "initial-result-1",
                {"findings": [finding], "unresolved_questions": [],
                 "evidence_references": finding["evidence_references"], "claim_references": []})
            self.assertEqual("await_remediation", reviewed["pending_next_action"])
            replay = STATE.advance(store, first_authority, STATE.digest(first_authority),
                first["durable_revision"], "record_initial_result", "initial-result-1",
                {"findings": [finding], "unresolved_questions": [],
                 "evidence_references": finding["evidence_references"], "claim_references": []})
            self.assertEqual(reviewed["durable_revision"], replay["durable_revision"])

            successor = authority(2, reviewed["durable_revision"])
            replaced = STATE.advance(store, successor, STATE.digest(successor),
                reviewed["durable_revision"], "replace_writer", "replace-1",
                {"reason": "provider session unavailable", "pending_next_action": "reconcile_then_resume"})
            self.assertEqual("reconstructed_continuation", replaced["continuity"])
            self.assertEqual(2, replaced["writer_binding"]["generation"])
            with self.assertRaisesRegex(ValueError, "current writer generation"):
                STATE.advance(store, first_authority, STATE.digest(first_authority),
                    replaced["durable_revision"], "mark_uncertain", "old-writer",
                    {"reason": "stale", "reconciliation_action": "stop"})
            history = store.list_revisions(STATE.stable_key(successor["identity"]), None, 10)
            self.assertEqual(3, len(history.values))

    def test_uncertainty_and_retirement_are_observationally_terminal(self):
        with tempfile.TemporaryDirectory() as directory:
            repository = Path(directory)
            subprocess.run(["git", "init", "-q", str(repository)], check=True)
            store = STATE.DURABLE.GitRefDurableStateStore(repository, namespace="independent-review-state")
            grant = authority()
            current = STATE.begin(store, grant, STATE.digest(grant), {
                "transition_id": "begin", "evidence_references": [],
                "claim_references": [], "unresolved_questions": []})
            uncertain = STATE.advance(store, grant, STATE.digest(grant), current["durable_revision"],
                "mark_uncertain", "uncertain", {"reason": "lost acknowledgement",
                                                  "reconciliation_action": "read_current_revision"})
            self.assertEqual("uncertain", uncertain["status"])
            retired = STATE.advance(store, grant, STATE.digest(grant), uncertain["durable_revision"],
                "retire_episode", "retire", {"outcome": "review_superseded", "reason": "new candidate",
                                                "protected_references": [reference()]})
            self.assertEqual("retired", retired["status"])
            with self.assertRaisesRegex(ValueError, "retired"):
                STATE.advance(store, grant, STATE.digest(grant), retired["durable_revision"],
                    "mark_uncertain", "reactivate", {"reason": "bad", "reconciliation_action": "bad"})

    def test_rejects_cross_episode_claim_fields_and_finding_rewrite(self):
        with tempfile.TemporaryDirectory() as directory:
            repository = Path(directory)
            subprocess.run(["git", "init", "-q", str(repository)], check=True)
            store = STATE.DURABLE.GitRefDurableStateStore(repository, namespace="independent-review-state")
            grant = authority()
            current = STATE.begin(store, grant, STATE.digest(grant), {
                "transition_id": "begin", "evidence_references": [],
                "claim_references": [], "unresolved_questions": []})
            with self.assertRaisesRegex(ValueError, "initial result transition"):
                STATE.advance(store, grant, STATE.digest(grant), current["durable_revision"],
                    "record_initial_result", "claim-mutation", {"claim_revision": "new"})
            bad_finding = {"finding_id": "f", "attributed_reviewer": "someone-else",
                           "reviewer_generation": 99, "severity": "low", "observation": "x",
                           "evidence_references": [reference()], "status": "verified_resolved",
                           "remediation_references": []}
            with self.assertRaisesRegex(ValueError, "remediation status"):
                STATE.advance(store, grant, STATE.digest(grant), current["durable_revision"],
                    "record_initial_result", "bad-status", {"findings": [bad_finding],
                        "unresolved_questions": [], "evidence_references": [], "claim_references": []})
            bad_finding["status"] = "open"
            with self.assertRaisesRegex(ValueError, "authorized writer generation"):
                STATE.advance(store, grant, STATE.digest(grant), current["durable_revision"],
                    "record_initial_result", "bad-attribution", {"findings": [bad_finding],
                        "unresolved_questions": [], "evidence_references": [], "claim_references": []})
            other = authority(episode="other")
            with self.assertRaisesRegex(ValueError, "expected durable revision"):
                STATE.advance(store, other, STATE.digest(other), current["durable_revision"],
                    "mark_uncertain", "cross", {"reason": "x", "reconciliation_action": "stop"})


if __name__ == "__main__":
    unittest.main()
