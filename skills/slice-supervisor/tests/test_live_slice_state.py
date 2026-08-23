from __future__ import annotations

import importlib.util
import json
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).parents[1]
spec = importlib.util.spec_from_file_location("vertical_live_slice", ROOT / "scripts/live_slice_state.py")
assert spec and spec.loader
LIVE = importlib.util.module_from_spec(spec)
spec.loader.exec_module(LIVE)
RESUME = ROOT / "scripts/resume_active_slice.py"


class LiveSliceStateTest(unittest.TestCase):
    identity = {"run_id": "run-1", "slice_number": 1,
                "attempt_id": "attempt-1", "plan_version": "plan-1"}

    def fresh(self, repository: Path, *extra: str) -> dict:
        result = subprocess.run([
            sys.executable, str(RESUME), "--repository", str(repository),
            "--identity-json", json.dumps(self.identity), *extra,
        ], text=True, stdout=subprocess.PIPE, stderr=subprocess.PIPE, check=True)
        return json.loads(result.stdout)

    def test_fresh_recovery_replay_stale_write_and_retirement(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            repository = Path(directory)
            subprocess.run(["git", "init", "-q", str(repository)], check=True)
            head = subprocess.run(["git", "-C", str(repository), "symbolic-ref", "HEAD"],
                                  text=True, stdout=subprocess.PIPE, check=True).stdout
            store = LIVE.DURABLE.GitRefDurableStateStore(repository)
            active = LIVE.create(store, identity=self.identity,
                actor_binding={"logical_actor_id": "builder-1", "provider": "claude",
                               "runtime_session_id": "ephemeral-session"}, phase="review",
                pending_obligation={"obligation_id": "review-1", "kind": "independent_review",
                                    "summary": "Review accepted plan"},
                authoritative_refs=[{"kind": "accepted_plan", "reference": "plan:plan-1"}])
            waiting = LIVE.wait_on_capability(store, active, event_id="provider-failure-1",
                capability="independent_review", provider="claude", reason="temporarily unavailable")
            fresh_waiting = self.fresh(repository)
            self.assertEqual("waiting_on_capability", fresh_waiting["status"])
            self.assertEqual(waiting["pending_obligation"], fresh_waiting["pending_obligation"])
            resumed = self.fresh(repository, "--capability", "independent_review",
                                 "--recovery-event-id", "provider-recovered-1")
            self.assertEqual("active", resumed["status"])
            self.assertEqual(self.identity, resumed["identity"])
            replay = self.fresh(repository, "--capability", "independent_review",
                                "--recovery-event-id", "provider-recovered-1")
            self.assertEqual(resumed["durable_revision"], replay["durable_revision"])
            internal = LIVE.load(store, self.identity)
            self.assertIsNotNone(internal)
            self.assertEqual(1, internal["handled_consequences"].count(
                '["resume_capability","provider-recovered-1"]'))
            retired = LIVE.retire(store, internal, event_id="provider-recovered-1",
                                  outcome="accepted", reason="same raw id, different kind")
            self.assertEqual("retired", retired["status"])
            self.assertIn('["retire","provider-recovered-1"]', retired["handled_consequences"])
            with self.assertRaisesRegex(ValueError, "revision conflict"):
                store.publish(LIVE.stable_key(self.identity), b"stale", waiting["durable_revision"])
            with self.assertRaisesRegex(ValueError, "retired"):
                LIVE.wait_on_capability(store, retired, event_id="late-event",
                    capability="independent_review", provider="claude", reason="stale failure")
            self.assertEqual(head, subprocess.run(
                ["git", "-C", str(repository), "symbolic-ref", "HEAD"],
                text=True, stdout=subprocess.PIPE, check=True).stdout)
            self.assertEqual("", subprocess.run(
                ["git", "-C", str(repository), "status", "--short"],
                text=True, stdout=subprocess.PIPE, check=True).stdout)

    def test_identity_key_boundaries_and_scalar_validation(self) -> None:
        shifted = {**self.identity, "attempt_id": "attempt-1/plan", "plan_version": "version"}
        other = {**self.identity, "attempt_id": "attempt-1", "plan_version": "plan/version"}
        self.assertNotEqual(LIVE.stable_key(shifted), LIVE.stable_key(other))
        with self.assertRaisesRegex(ValueError, "slice_number"):
            LIVE.validate_identity({**self.identity, "slice_number": True})

    def test_phase_publication_replay_conflict_uncertainty_and_ordering(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            repository = Path(directory)
            subprocess.run(["git", "init", "-q", str(repository)], check=True)
            store = LIVE.DURABLE.GitRefDurableStateStore(repository)
            active = LIVE.create(store, identity=self.identity,
                actor_binding={"logical_actor_id": "builder-1", "provider": "codex",
                               "runtime_session_id": None}, phase="planning",
                pending_obligation={"obligation_id": "implementation-1", "kind": "implementation",
                                    "summary": "Implement accepted slice"},
                authoritative_refs=[], accepted_boundary={
                    "reference": "plan:1", "integrity_sha256": "a" * 64})
            consequence = {"consequence_id": "implementation-1", "summary": "complete",
                "certainty": "established", "uncertainty_reason": None,
                "references": [{"kind": "manifest", "reference": "manifest:1",
                                "integrity_sha256": "b" * 64}]}
            published = LIVE.publish_phase_consequence(
                store, active, phase="implementation", consequence=consequence)
            replay = LIVE.publish_phase_consequence(
                store, published, phase="implementation", consequence=consequence)
            self.assertEqual(published["durable_revision"], replay["durable_revision"])
            with self.assertRaisesRegex(ValueError, "conflicts"):
                LIVE.publish_phase_consequence(store, published, phase="implementation",
                    consequence={**consequence, "summary": "different"})
            gate_uncertain = LIVE.publish_phase_consequence(store, published, phase="gate",
                consequence={"consequence_id": "gate-1", "summary": "gate completion unknown",
                    "certainty": "uncertain", "uncertainty_reason": "mailbox interrupted",
                    "references": []})
            self.assertEqual("uncertain", gate_uncertain["latest_phase_consequence"]["certainty"])
            with self.assertRaisesRegex(ValueError, "regress"):
                LIVE.publish_phase_consequence(store, gate_uncertain, phase="implementation",
                    consequence={**consequence, "consequence_id": "implementation-2"})

    def test_legacy_active_attempt_is_recovered_without_inventing_completion(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            repository = Path(directory)
            subprocess.run(["git", "init", "-q", str(repository)], check=True)
            store = LIVE.DURABLE.GitRefDurableStateStore(repository)
            legacy = {"schema_version": 1, "identity": self.identity,
                "actor_binding": {"logical_actor_id": "builder-1", "provider": None,
                                  "runtime_session_id": None},
                "phase": "planning", "status": "active",
                "pending_obligation": {"obligation_id": "plan-1", "kind": "planning",
                                       "summary": "Plan slice"},
                "handled_consequences": [], "authoritative_refs": [],
                "waiting": None, "retirement": None}
            store.publish(LIVE.stable_key(self.identity), LIVE.canonical(legacy), None)
            recovered = LIVE.load(store, self.identity)
            self.assertIsNotNone(recovered)
            self.assertEqual(2, recovered["schema_version"])
            self.assertIsNone(recovered["accepted_boundary"])
            self.assertIsNone(recovered["latest_phase_consequence"])


if __name__ == "__main__":
    unittest.main()
