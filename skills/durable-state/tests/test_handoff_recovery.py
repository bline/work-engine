"""Recovery regressions against the real CAS-backed board in scratch Git repos."""
import hashlib
import uuid
from concurrent.futures import ThreadPoolExecutor

from test_codex_chatboard_contracts import BOARD, RepoCase, SESSION_1, SESSION_2


class HandoffRecovery(RepoCase):
    role = "qualification-root"

    def publish(self, text, session=SESSION_1, replaces=()):
        path = self.repository / (hashlib.sha256(text.encode()).hexdigest() + ".md")
        path.write_text(text)
        return BOARD.publish_handoff(self.repository, author="test", session_id=session,
            role=self.role, handoff_path=path, title="S10 repairs", replaces=list(replaces),
            message_id=str(uuid.uuid4()), observed_at="2026-10-25T00:00:00Z")["message"]

    def adopt(self, digest, session=SESSION_2, predecessor=SESSION_1):
        return BOARD.adopt(self.repository, author="test", session_id=session, body="adopted",
            role=self.role, predecessor_session=predecessor, handoff_sha256=digest,
            message_id=str(uuid.uuid4()), observed_at="2026-10-25T00:00:00Z")

    def view(self, digest, session=None, role=None):
        return BOARD.handoff_view(self.board()[0], role=role or self.role,
                                  digest=digest, session_id=session)

    def test_restart_of_consumed_handoff_is_rejected_and_points_to_new_preparation(self):
        old = self.publish("Claude selected; original repairs pending")
        digest = old["data"]["handoff_sha256"]
        self.adopt(digest)
        self.assertEqual(self.view(digest)["state"], "already_adopted")
        self.assertEqual(self.view(digest, SESSION_2)["state"], "adopted_by_self")
        new = self.publish("User selected Sol Medium for this slice; SREV10 remains", SESSION_2)
        for _ in range(21):
            self.post()
        stale = self.view(digest)
        self.assertEqual(stale["state"], "superseded")
        self.assertEqual(stale["continuations"][0]["handoff_path"], new["data"]["handoff_path"])
        with self.assertRaisesRegex(BOARD.ChatboardError, "superseded"):
            self.adopt(digest, str(uuid.uuid4()))
        self.assertEqual(self.view(digest, role="another-root")["state"], "unregistered")

    def test_competing_adopters_only_one_can_publish(self):
        digest = self.publish("handoff")["data"]["handoff_sha256"]
        def attempt(_):
            try:
                self.adopt(digest, str(uuid.uuid4()))
                return True
            except BOARD.ChatboardError:
                return False
        with ThreadPoolExecutor(max_workers=2) as pool:
            self.assertEqual(sorted(pool.map(attempt, range(2))), [False, True])

    def test_publication_is_idempotent_and_predecessor_cannot_reclaim_adopted_work(self):
        old = self.publish("handoff")
        self.assertEqual(old, self.publish("handoff"))
        digest = old["data"]["handoff_sha256"]
        self.adopt(digest)
        with self.assertRaisesRegex(BOARD.ChatboardError, "recorded adopter"):
            self.publish("obsolete root attempts new instructions")

    def test_changed_preparation_supersedes_without_timestamp_ordering(self):
        old = self.publish("first")
        new = self.publish("revised")
        self.assertEqual(self.view(old["data"]["handoff_sha256"])["state"], "superseded")
        self.assertEqual(self.view(new["data"]["handoff_sha256"])["state"], "available")
        with self.assertRaises(BOARD.ChatboardError):
            self.publish("first")

    def test_legacy_adoption_still_prevents_second_owner(self):
        digest = "a" * 64
        self.adopt(digest)
        with self.assertRaisesRegex(BOARD.ChatboardError, "already_adopted"):
            self.adopt(digest, str(uuid.uuid4()))

    def test_cli_reports_unregistered_and_obsolete(self):
        self.assertEqual(self.run_cli("handoff-status", "--role", self.role,
                                     "--handoff-sha256", "a" * 64).returncode, 32)
        old = self.publish("first")
        self.publish("second")
        self.assertEqual(self.run_cli("handoff-status", "--role", self.role,
            "--handoff-sha256", old["data"]["handoff_sha256"]).returncode, 21)

    def test_explicit_reconciliation_preserves_old_conflict_and_exposes_successor(self):
        old = self.publish("original")
        digest = old["data"]["handoff_sha256"]
        self.adopt(digest)
        # Simulate historical conflicting adoptions written before the guard.
        def duplicate(board):
            row = dict(board["messages"][-1], session_id=str(uuid.uuid4()),
                       message_id=str(uuid.uuid4()), sequence=board["next_sequence"])
            board["messages"].append(row)
            board["next_sequence"] += 1
        self.raw_rewrite(duplicate)
        with self.assertRaises(BOARD.ChatboardError):
            self.publish("reconciled", SESSION_2)
        new = self.publish("reconciled", SESSION_2, replaces=[digest])
        self.assertEqual(self.view(digest)["state"], "conflict")
        self.assertEqual(self.view(digest)["continuations"][0]["handoff_sha256"],
                         new["data"]["handoff_sha256"])
        # Explicit replacement still cannot absorb an unseen newer continuation.
        with self.assertRaises(BOARD.ChatboardError):
            self.publish("competing reconciliation", str(uuid.uuid4()), replaces=[digest])
