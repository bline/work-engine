from __future__ import annotations

import json
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path


ROOT = Path(__file__).parents[1]
MANAGE = ROOT / "scripts/manage_active_slice.py"
RESUME = ROOT / "scripts/resume_active_slice.py"


class ActiveSliceWorkflowTest(unittest.TestCase):
    identity = {"run_id": "run-1", "slice_number": 1,
                "attempt_id": "attempt-1", "plan_version": "plan-1"}

    def invoke(self, script: Path, repository: Path, *args: str,
               check: bool = True) -> subprocess.CompletedProcess[str]:
        return subprocess.run([sys.executable, str(script), "--repository", str(repository),
                               *args], text=True, stdout=subprocess.PIPE,
                              stderr=subprocess.PIPE, check=check)

    def manage(self, repository: Path, *args: str) -> dict:
        return json.loads(self.invoke(MANAGE, repository, *args).stdout)

    def test_subprocess_lifecycle_replay_conflict_retirement_and_integrity(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            repository = Path(directory)
            subprocess.run(["git", "init", "-q", str(repository)], check=True)
            head = subprocess.run(["git", "-C", str(repository), "symbolic-ref", "HEAD"],
                                  text=True, stdout=subprocess.PIPE, check=True).stdout
            identity = json.dumps(self.identity)
            active = self.manage(repository, "begin", "--identity-json", identity,
                "--actor-binding-json", json.dumps({"logical_actor_id": "supervisor-1",
                    "provider": "claude", "runtime_session_id": "session-1"}),
                "--phase", "review", "--obligation-json", json.dumps({
                    "obligation_id": "review-1", "kind": "independent_review",
                    "summary": "Review accepted implementation"}),
                "--authoritative-refs-json", json.dumps([
                    {"kind": "accepted_plan", "reference": "plan:plan-1"}]))
            waiting = self.manage(repository, "wait", "--identity-json", identity,
                "--expected-revision", active["durable_revision"],
                "--event-id", "unavailable-1", "--capability", "independent_review",
                "--provider", "claude", "--reason", "temporarily unavailable")
            replay = self.manage(repository, "wait", "--identity-json", identity,
                "--expected-revision", waiting["durable_revision"],
                "--event-id", "unavailable-1", "--capability", "independent_review",
                "--provider", "claude", "--reason", "temporarily unavailable")
            self.assertEqual(waiting["durable_revision"], replay["durable_revision"])

            stale = self.invoke(MANAGE, repository, "wait", "--identity-json", identity,
                "--expected-revision", active["durable_revision"], "--event-id", "late",
                "--capability", "independent_review", "--reason", "late failure", check=False)
            self.assertEqual(2, stale.returncode)
            self.assertIn("expected durable revision", stale.stderr)

            resumed = json.loads(self.invoke(RESUME, repository, "--identity-json", identity,
                "--capability", "independent_review", "--recovery-event-id",
                "recovered-1").stdout)
            self.assertEqual(active["pending_obligation"], resumed["pending_obligation"])
            self.assertEqual("review", resumed["phase"])
            retired = self.manage(repository, "retire", "--identity-json", identity,
                "--expected-revision", resumed["durable_revision"], "--event-id", "done-1",
                "--outcome", "accepted", "--reason", "review obligation discharged")
            self.assertEqual("retired", retired["status"])

            after_retire = self.invoke(MANAGE, repository, "wait", "--identity-json", identity,
                "--expected-revision", retired["durable_revision"], "--event-id", "stale-2",
                "--capability", "independent_review", "--reason", "stale", check=False)
            self.assertEqual(2, after_retire.returncode)
            self.assertIn("retired", after_retire.stderr)
            self.assertEqual(head, subprocess.run(
                ["git", "-C", str(repository), "symbolic-ref", "HEAD"], text=True,
                stdout=subprocess.PIPE, check=True).stdout)
            self.assertEqual("", subprocess.run(
                ["git", "-C", str(repository), "status", "--short"], text=True,
                stdout=subprocess.PIPE, check=True).stdout)

    def test_invalid_input_fails_without_publication(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            repository = Path(directory)
            subprocess.run(["git", "init", "-q", str(repository)], check=True)
            result = self.invoke(MANAGE, repository, "begin", "--identity-json", "{}",
                "--actor-binding-json", "{}", "--phase", "planning",
                "--obligation-json", "{}", check=False)
            self.assertEqual(2, result.returncode)
            self.assertIn("identity fields are invalid", result.stderr)
            self.assertEqual("", subprocess.run(
                ["git", "-C", str(repository), "status", "--short"], text=True,
                stdout=subprocess.PIPE, check=True).stdout)

    def test_phase_consequence_survives_runtime_and_mailbox_loss(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            repository = Path(directory)
            subprocess.run(["git", "init", "-q", str(repository)], check=True)
            identity = json.dumps(self.identity)
            boundary = {"reference": "plan:plan-1", "integrity_sha256": "a" * 64}
            active = self.manage(repository, "begin", "--identity-json", identity,
                "--actor-binding-json", json.dumps({"logical_actor_id": "builder-1",
                    "provider": "codex", "runtime_session_id": "lost-runtime"}),
                "--phase", "planning", "--obligation-json", json.dumps({
                    "obligation_id": "implementation-1", "kind": "implementation",
                    "summary": "Implement accepted slice"}),
                "--accepted-boundary-json", json.dumps(boundary),
                "--authoritative-refs-json", json.dumps([
                    {"kind": "accepted_plan", "reference": "plan:plan-1"}]))
            consequence = {
                "consequence_id": "implementation-complete-1",
                "summary": "Implementation completed and is ready for gate",
                "certainty": "established",
                "uncertainty_reason": None,
                "references": [{"kind": "implementation_manifest",
                    "reference": "manifest:sha256:123", "integrity_sha256": "b" * 64}],
            }
            published = self.manage(repository, "publish-phase",
                "--identity-json", identity,
                "--expected-revision", active["durable_revision"],
                "--phase", "implementation", "--consequence-json", json.dumps(consequence))

            # A new process has neither the builder result nor its runtime/mailbox state.
            recovered = json.loads(self.invoke(RESUME, repository,
                "--identity-json", identity).stdout)
            self.assertEqual(self.identity, recovered["identity"])
            self.assertEqual("implementation", recovered["phase"])
            self.assertEqual(boundary, recovered["accepted_boundary"])
            self.assertEqual(active["pending_obligation"], recovered["pending_obligation"])
            self.assertEqual(consequence, recovered["latest_phase_consequence"])
            self.assertEqual(published["durable_revision"], recovered["durable_revision"])
            self.assertFalse(recovered["runtime_binding"]["semantic"])
            self.assertNotIn("handled_consequences", recovered)

            first_page = json.loads(self.invoke(RESUME, repository,
                "--identity-json", identity, "--history-limit", "1").stdout)
            self.assertEqual([published["durable_revision"]],
                             [item["revision"] for item in first_page["items"]])
            self.assertEqual(published["durable_revision"], first_page["next_cursor"])
            second_page = json.loads(self.invoke(RESUME, repository,
                "--identity-json", identity, "--history-limit", "1",
                "--history-cursor", first_page["next_cursor"]).stdout)
            self.assertEqual([active["durable_revision"]],
                             [item["revision"] for item in second_page["items"]])
            self.assertIsNone(second_page["next_cursor"])

            selected = json.loads(self.invoke(RESUME, repository,
                "--identity-json", identity,
                "--revision", active["durable_revision"]).stdout)
            self.assertEqual("planning", selected["phase"])
            self.assertIsNone(selected["latest_phase_consequence"])
            self.assertEqual(active["durable_revision"], selected["durable_revision"])

            unchanged = json.loads(self.invoke(RESUME, repository,
                "--identity-json", identity).stdout)
            self.assertEqual(published["durable_revision"], unchanged["durable_revision"])
            self.assertEqual("", subprocess.run(
                ["git", "-C", str(repository), "status", "--short"], text=True,
                stdout=subprocess.PIPE, check=True).stdout)


if __name__ == "__main__":
    unittest.main()
