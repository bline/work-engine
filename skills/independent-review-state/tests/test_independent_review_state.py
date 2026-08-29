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


def authority(generation=1, predecessor=None, episode="episode-1", provider="claude",
              runtime_owner="claude-runtime", runtime_reference=None):
    runtime_reference = runtime_reference or f"session-{generation}"
    return {"schema_version": 1, "profile": STATE.PROFILE, "grant_id": f"grant-{generation}",
            "identity": {"run_id": "run", "slice_number": 1, "attempt_id": "attempt",
                         "plan_version": "plan", "review_obligation_id": "review-1",
                         "review_episode_id": episode},
            "source": reference("human-authority", "accepted-review-profile", "decision-1"),
            "writer": {"logical_actor_id": "reviewer", "provider": provider,
                       "generation": generation,
                       "runtime_session_ref": reference(runtime_owner, runtime_reference, f"g{generation}")},
            "readers": ["adversarial_reviewer" if provider == "codex" else "independent_reviewer",
                        "coordinating_builder", "slice_supervisor"],
            "initial_subject": [reference()], "expires_at": None,
            "predecessor_revision": predecessor}


class IndependentReviewStateTest(unittest.TestCase):
    def test_begin_retry_is_idempotent_and_conflicting_content_is_rejected(self):
        with tempfile.TemporaryDirectory() as directory:
            repository = Path(directory)
            subprocess.run(["git", "init", "-q", str(repository)], check=True)
            store = STATE.DURABLE.GitRefDurableStateStore(
                repository, namespace="independent-review-state")
            grant = authority(runtime_reference="initial-session")
            request = {"transition_id": "begin", "evidence_references": [],
                       "claim_references": [], "unresolved_questions": []}

            begun = STATE.begin(store, grant, STATE.digest(grant), request)
            replay = STATE.begin(store, grant, STATE.digest(grant), request)

            self.assertEqual(begun["durable_revision"], replay["durable_revision"])
            history = store.list_revisions(
                STATE.stable_key(grant["identity"]), None, 10)
            self.assertEqual(1, len(history.values))

            with self.assertRaisesRegex(ValueError, "transition identity conflicts"):
                STATE.begin(store, grant, STATE.digest(grant), {
                    **request, "unresolved_questions": ["different durable content"]})
            with self.assertRaisesRegex(ValueError, "review episode already exists"):
                STATE.begin(store, grant, STATE.digest(grant), {
                    **request, "transition_id": "another-begin"})

    def test_uncertain_initial_review_requires_replacement_before_result_publication(self):
        with tempfile.TemporaryDirectory() as directory:
            repository = Path(directory)
            subprocess.run(["git", "init", "-q", str(repository)], check=True)
            store = STATE.DURABLE.GitRefDurableStateStore(repository, namespace="independent-review-state")
            first_authority = authority(runtime_reference="initial-session")
            begun = STATE.begin(store, first_authority, STATE.digest(first_authority), {
                "transition_id": "begin", "evidence_references": [],
                "claim_references": [], "unresolved_questions": []})
            uncertain = STATE.advance(store, first_authority, STATE.digest(first_authority),
                begun["durable_revision"], "mark_uncertain", "uncertain-initial",
                {"reason": "initial result acknowledgement lost",
                 "reconciliation_action": "replace_writer_or_retire"})
            result_payload = {"findings": [], "unresolved_questions": [],
                              "evidence_references": [], "claim_references": []}
            with self.assertRaisesRegex(ValueError, "initial result transition"):
                STATE.advance(store, first_authority, STATE.digest(first_authority),
                    uncertain["durable_revision"], "record_initial_result", "blocked-result",
                    result_payload)
            preserved = STATE.load(store, first_authority["identity"])
            self.assertEqual(uncertain["durable_revision"], preserved["durable_revision"])
            self.assertEqual("uncertain", preserved["status"])
            self.assertEqual("replace_writer_or_retire", preserved["pending_next_action"])
            history = store.list_revisions(STATE.stable_key(first_authority["identity"]), None, 10)
            self.assertEqual(2, len(history.values))

            successor = authority(2, uncertain["durable_revision"], runtime_reference="replacement-initial")
            replaced = STATE.advance(store, successor, STATE.digest(successor),
                uncertain["durable_revision"], "replace_writer", "replace-initial",
                {"reason": "uncertain initial result", "pending_next_action": "perform_initial_review"})
            self.assertEqual("active", replaced["status"])
            self.assertIsNone(replaced["uncertainty"])
            self.assertEqual(2, replaced["writer_binding"]["generation"])
            self.assertEqual("replacement-initial",
                             replaced["writer_binding"]["runtime_session_ref"]["reference"])
            reported = STATE.advance(store, successor, STATE.digest(successor),
                replaced["durable_revision"], "record_initial_result", "replacement-result",
                result_payload)
            self.assertEqual("reported", reported["current_review_phase"])
            self.assertEqual(2, reported["writer_binding"]["generation"])
            with self.assertRaisesRegex(ValueError, "current writer generation"):
                STATE.advance(store, first_authority, STATE.digest(first_authority),
                    reported["durable_revision"], "mark_uncertain", "stale-initial-writer",
                    {"reason": "stale", "reconciliation_action": "stop"})

    def test_uncertain_re_evaluation_requires_replacement_before_result_publication(self):
        with tempfile.TemporaryDirectory() as directory:
            repository = Path(directory)
            subprocess.run(["git", "init", "-q", str(repository)], check=True)
            store = STATE.DURABLE.GitRefDurableStateStore(repository, namespace="independent-review-state")
            first_authority = authority(runtime_reference="reevaluation-session")
            begun = STATE.begin(store, first_authority, STATE.digest(first_authority), {
                "transition_id": "begin", "evidence_references": [],
                "claim_references": [], "unresolved_questions": []})
            finding = {"finding_id": "finding-1", "attributed_reviewer": "reviewer",
                       "reviewer_generation": 1, "severity": "high", "observation": "bounded issue",
                       "evidence_references": [reference("repository", "file.py", "blob-1")],
                       "status": "open", "remediation_references": []}
            initial = STATE.advance(store, first_authority, STATE.digest(first_authority),
                begun["durable_revision"], "record_initial_result", "initial-result",
                {"findings": [finding], "unresolved_questions": [],
                 "evidence_references": finding["evidence_references"], "claim_references": []})
            subject = reference("checkpoint", "candidate", "candidate-2")
            evaluating = STATE.advance(store, first_authority, STATE.digest(first_authority),
                initial["durable_revision"], "record_remediation_subject", "subject-2",
                {"reviewed_subject": [subject], "evidence_references": [subject]})
            uncertain = STATE.advance(store, first_authority, STATE.digest(first_authority),
                evaluating["durable_revision"], "mark_uncertain", "uncertain-reevaluation",
                {"reason": "re-evaluation acknowledgement lost",
                 "reconciliation_action": "replace_writer_or_retire"})
            resolved = {**finding, "status": "verified_resolved", "remediation_references": [subject]}
            result_payload = {"findings": [resolved], "unresolved_questions": [],
                              "evidence_references": [subject], "claim_references": []}
            with self.assertRaisesRegex(ValueError, "re-evaluation transition"):
                STATE.advance(store, first_authority, STATE.digest(first_authority),
                    uncertain["durable_revision"], "record_re_evaluation", "blocked-reevaluation",
                    result_payload)
            preserved = STATE.load(store, first_authority["identity"])
            self.assertEqual(uncertain["durable_revision"], preserved["durable_revision"])
            self.assertEqual("uncertain", preserved["status"])
            self.assertEqual("replace_writer_or_retire", preserved["pending_next_action"])
            history = store.list_revisions(STATE.stable_key(first_authority["identity"]), None, 10)
            self.assertEqual(4, len(history.values))

            successor = authority(2, uncertain["durable_revision"], runtime_reference="replacement-reevaluation")
            replaced = STATE.advance(store, successor, STATE.digest(successor),
                uncertain["durable_revision"], "replace_writer", "replace-reevaluation",
                {"reason": "uncertain re-evaluation", "pending_next_action": "re_evaluate_bound_subject"})
            self.assertEqual("re_evaluation", replaced["current_review_phase"])
            self.assertEqual("reconstructed_continuation", replaced["continuity"])
            reported = STATE.advance(store, successor, STATE.digest(successor),
                replaced["durable_revision"], "record_re_evaluation", "replacement-reevaluation-result",
                result_payload)
            self.assertEqual("reported", reported["current_review_phase"])
            self.assertEqual(2, reported["writer_binding"]["generation"])
            self.assertEqual("replacement-reevaluation",
                             reported["writer_binding"]["runtime_session_ref"]["reference"])

    def test_reported_result_accepts_multiple_exact_same_session_subjects(self):
        with tempfile.TemporaryDirectory() as directory:
            repository = Path(directory)
            subprocess.run(["git", "init", "-q", str(repository)], check=True)
            store = STATE.DURABLE.GitRefDurableStateStore(repository, namespace="independent-review-state")
            grant = authority(runtime_reference="retained-session")
            current = STATE.begin(store, grant, STATE.digest(grant), {
                "transition_id": "begin", "evidence_references": [],
                "claim_references": [], "unresolved_questions": []})
            finding = {"finding_id": "finding-1", "attributed_reviewer": "reviewer",
                       "reviewer_generation": 1, "severity": "high", "observation": "bounded issue",
                       "evidence_references": [reference("repository", "file.py", "blob-1")],
                       "status": "open", "remediation_references": []}
            initial = STATE.advance(store, grant, STATE.digest(grant), current["durable_revision"],
                "record_initial_result", "initial-result", {"findings": [finding],
                    "unresolved_questions": [], "evidence_references": finding["evidence_references"],
                    "claim_references": []})

            subject2 = reference("checkpoint", "candidate", "candidate-2")
            evaluating2 = STATE.advance(store, grant, STATE.digest(grant), initial["durable_revision"],
                "record_remediation_subject", "subject-2",
                {"reviewed_subject": [subject2], "evidence_references": [subject2]})
            resolved = {**finding, "status": "verified_resolved", "remediation_references": [subject2]}
            result2_evidence = reference("review", "candidate-2-result", "result-2")
            result2_claim = reference("claim", "candidate-2-claim", "claim-2")
            reported2 = STATE.advance(store, grant, STATE.digest(grant), evaluating2["durable_revision"],
                "record_re_evaluation", "result-2", {"findings": [resolved],
                    "unresolved_questions": [], "evidence_references": [result2_evidence],
                    "claim_references": [result2_claim]})
            self.assertEqual("reported", reported2["current_review_phase"])

            subject3 = reference("checkpoint", "candidate", "candidate-3")
            evaluating3 = STATE.advance(store, grant, STATE.digest(grant), reported2["durable_revision"],
                "record_remediation_subject", "subject-3",
                {"reviewed_subject": [subject3], "evidence_references": [subject3]})
            self.assertEqual("re_evaluation", evaluating3["current_review_phase"])
            self.assertEqual("re_evaluate_delta_in_same_session", evaluating3["pending_next_action"])
            self.assertEqual([subject3], evaluating3["reviewed_subject"])
            self.assertEqual([resolved], evaluating3["findings"])
            self.assertEqual([result2_claim], evaluating3["claim_references"])
            for field in ("identity", "authority_binding", "writer_binding"):
                self.assertEqual(reported2[field], evaluating3[field])
            self.assertEqual("retained-session",
                             evaluating3["writer_binding"]["runtime_session_ref"]["reference"])
            self.assertEqual("same_session", evaluating3["continuity"])

            retained2 = STATE.decode(store.read_revision(
                STATE.stable_key(grant["identity"]), reported2["durable_revision"]))
            self.assertEqual([subject2], retained2["reviewed_subject"])
            self.assertEqual([result2_evidence], retained2["evidence_references"])
            self.assertEqual([resolved], retained2["findings"])

            result3_evidence = reference("review", "candidate-3-result", "result-3")
            reported3 = STATE.advance(store, grant, STATE.digest(grant), evaluating3["durable_revision"],
                "record_re_evaluation", "result-3", {"findings": [resolved],
                    "unresolved_questions": [], "evidence_references": [result3_evidence],
                    "claim_references": [result2_claim]})
            subject4 = reference("checkpoint", "candidate", "candidate-4")
            evaluating4 = STATE.advance(store, grant, STATE.digest(grant), reported3["durable_revision"],
                "record_remediation_subject", "subject-4",
                {"reviewed_subject": [subject4], "evidence_references": [subject4]})
            replay = STATE.advance(store, grant, STATE.digest(grant), reported3["durable_revision"],
                "record_remediation_subject", "subject-4",
                {"reviewed_subject": [subject4], "evidence_references": [subject4]})
            self.assertEqual(evaluating4["durable_revision"], replay["durable_revision"])
            with self.assertRaisesRegex(ValueError, "transition identity conflicts"):
                STATE.advance(store, grant, STATE.digest(grant), evaluating4["durable_revision"],
                    "record_remediation_subject", "subject-4",
                    {"reviewed_subject": [subject3], "evidence_references": [subject3]})
            with self.assertRaisesRegex(ValueError, "expected durable revision"):
                STATE.advance(store, grant, STATE.digest(grant), reported3["durable_revision"],
                    "record_remediation_subject", "stale-subject",
                    {"reviewed_subject": [subject3], "evidence_references": [subject3]})

    def test_codex_same_model_reviewer_uses_same_episode_contract(self):
        with tempfile.TemporaryDirectory() as directory:
            repository = Path(directory)
            subprocess.run(["git", "init", "-q", str(repository)], check=True)
            store = STATE.DURABLE.GitRefDurableStateStore(repository, namespace="independent-review-state")
            grant = authority(provider="codex", runtime_owner="codex-collaboration",
                              runtime_reference="/root/slice/reviewer")
            current = STATE.begin(store, grant, STATE.digest(grant), {
                "transition_id": "codex-begin", "evidence_references": [],
                "claim_references": [], "unresolved_questions": []})
            reviewed = STATE.advance(store, grant, STATE.digest(grant), current["durable_revision"],
                "record_initial_result", "codex-result", {"findings": [],
                    "unresolved_questions": [], "evidence_references": [], "claim_references": []})

            self.assertEqual("codex", reviewed["writer_binding"]["provider"])
            self.assertIn("adversarial_reviewer",
                          reviewed["authority_binding"]["authorized_readers"])
            self.assertEqual("/root/slice/reviewer",
                             reviewed["writer_binding"]["runtime_session_ref"]["reference"])
            self.assertEqual("reported", reviewed["current_review_phase"])

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
            reported = STATE.advance(store, grant, STATE.digest(grant), current["durable_revision"],
                "record_initial_result", "initial-result", {"findings": [],
                    "unresolved_questions": [], "evidence_references": [], "claim_references": []})
            self.assertEqual("reported", reported["current_review_phase"])
            uncertain = STATE.advance(store, grant, STATE.digest(grant), reported["durable_revision"],
                "mark_uncertain", "uncertain", {"reason": "lost acknowledgement",
                                                  "reconciliation_action": "read_current_revision"})
            self.assertEqual("uncertain", uncertain["status"])
            with self.assertRaisesRegex(ValueError, "remediation subject transition"):
                STATE.advance(store, grant, STATE.digest(grant), uncertain["durable_revision"],
                    "record_remediation_subject", "uncertain-subject",
                    {"reviewed_subject": [reference(revision="r2")],
                     "evidence_references": [reference(revision="r2")]})
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
