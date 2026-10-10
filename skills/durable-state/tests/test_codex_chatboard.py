from __future__ import annotations

import importlib.util
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest


SCRIPT = Path(__file__).parents[1] / "scripts/codex_chatboard.py"
sys.path.insert(0, str(SCRIPT.parent))
SPEC = importlib.util.spec_from_file_location("codex_chatboard", SCRIPT)
assert SPEC and SPEC.loader
BOARD = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(BOARD)
SESSION_1 = "11111111-1111-4111-8111-111111111111"
SESSION_2 = "22222222-2222-4222-8222-222222222222"


class CodexChatboardTest(unittest.TestCase):
    def setUp(self) -> None:
        self.temporary = tempfile.TemporaryDirectory()
        self.repository = Path(self.temporary.name)
        subprocess.run(["git", "init", "-q", str(self.repository)], check=True)

    def tearDown(self) -> None:
        self.temporary.cleanup()

    def test_posts_are_ordered_idempotent_and_retained(self) -> None:
        first = BOARD.post(
            self.repository, author="alpha", session_id=SESSION_1, topic="campaign",
            body="pair 1 running", references=["campaign/v4/pair-01"],
            message_id="aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa",
            observed_at="2026-08-30T04:00:00Z",
        )
        repeated = BOARD.post(
            self.repository, author="alpha", session_id=SESSION_1, topic="campaign",
            body="pair 1 running", references=["campaign/v4/pair-01"],
            message_id="aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa",
            observed_at="2026-08-30T04:00:00Z",
        )
        self.assertEqual(first["message"], repeated["message"])
        board, revision = BOARD.read_board(self.repository)
        self.assertEqual(len(board["messages"]), 1)
        self.assertEqual(board["messages"][0]["sequence"], 1)
        self.assertEqual(revision, repeated["revision"])

    def test_claim_conflict_release_and_expiry(self) -> None:
        claimed = BOARD.claim(
            self.repository, resource="paired-review/ordinal-1", author="alpha",
            session_id=SESSION_1, claim_id="aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa",
            ttl_seconds=60, note="launching pair", observed_at="2026-08-30T04:00:00Z",
        )
        with self.assertRaisesRegex(BOARD.ChatboardError, "already claimed"):
            BOARD.claim(
                self.repository, resource="paired-review/ordinal-1", author="beta",
                session_id=SESSION_2, claim_id="bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb",
                ttl_seconds=60, note="conflicting launch", observed_at="2026-08-30T04:00:30Z",
            )
        released = BOARD.release(
            self.repository, resource="paired-review/ordinal-1", session_id=SESSION_1,
            claim_id=claimed["claim"]["claim_id"],
        )
        self.assertTrue(released["released"])
        replacement = BOARD.claim(
            self.repository, resource="paired-review/ordinal-1", author="beta",
            session_id=SESSION_2, claim_id="bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb",
            ttl_seconds=60, note="new launch", observed_at="2026-08-30T04:02:00Z",
        )
        self.assertEqual(replacement["claim"]["author"], "beta")

    def test_read_view_defaults_to_newest_messages_and_pages_forward_with_since(self) -> None:
        board = BOARD.empty_board()
        board["messages"] = [{"sequence": number} for number in range(1, 11)]
        latest = BOARD.read_view(board, since=None, limit=3, active_claims=False,
                                 observed_at="2026-08-30T04:00:00Z")
        self.assertEqual([row["sequence"] for row in latest["messages"]], [8, 9, 10])
        paged = BOARD.read_view(board, since=0, limit=3, active_claims=False,
                                observed_at="2026-08-30T04:00:00Z")
        self.assertEqual([row["sequence"] for row in paged["messages"]], [1, 2, 3])
        after = BOARD.read_view(board, since=8, limit=3, active_claims=False,
                                observed_at="2026-08-30T04:00:00Z")
        self.assertEqual([row["sequence"] for row in after["messages"]], [9, 10])

    def test_read_view_active_claims_omits_only_expired_claims(self) -> None:
        board = BOARD.empty_board()
        board["claims"] = {
            "expired": {"expires_at": "2026-08-30T03:59:59Z"},
            "active": {"expires_at": "2026-08-30T04:00:01Z"},
        }
        every = BOARD.read_view(board, since=None, limit=1, active_claims=False,
                                observed_at="2026-08-30T04:00:00Z")
        self.assertEqual(set(every["claims"]), {"expired", "active"})
        active = BOARD.read_view(board, since=None, limit=1, active_claims=True,
                                 observed_at="2026-08-30T04:00:00Z")
        self.assertEqual(set(active["claims"]), {"active"})


if __name__ == "__main__":
    unittest.main()
