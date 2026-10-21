from __future__ import annotations

import importlib.util
from concurrent.futures import ThreadPoolExecutor
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import threading
import unittest
from unittest.mock import patch
from uuid import uuid4


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


@unittest.skipUnless(sys.platform == "linux", "event-driven wait requires Linux inotify")
class ChatboardWaitTest(unittest.TestCase):
    setUp = CodexChatboardTest.setUp
    tearDown = CodexChatboardTest.tearDown

    def post_message(self, topic: str = "wanted", repository: Path | None = None) -> dict:
        return BOARD.post(
            repository or self.repository, author="alpha", session_id=SESSION_1,
            topic=topic, body="synthetic wait test", references=[],
            message_id=str(uuid4()), observed_at=BOARD.timestamp(),
        )

    def wait_for_post(self, *, repository: Path | None = None, since: int = 0,
                      before_post=None) -> dict:
        entered = threading.Event()
        original = BOARD.RefChanges

        class SignallingChanges(original):
            def wait(self, timeout):
                entered.set()
                return super().wait(timeout)

        with patch.object(BOARD, "RefChanges", SignallingChanges), ThreadPoolExecutor() as pool:
            waiting = pool.submit(BOARD.wait_board, repository or self.repository,
                                  since=since, timeout_seconds=3, topic="wanted")
            self.assertTrue(entered.wait(2), "waiter never reached blocking select")
            if before_post:
                before_post()
            self.post_message(repository=repository)
            result = waiting.result(timeout=4)
        self.assertEqual(result["status"], "messages")
        return result

    def test_wait_observes_first_publication_in_empty_repository(self) -> None:
        result = self.wait_for_post()
        self.assertEqual(result["messages"][0]["sequence"], 1)
        self.assertEqual(result["next_since"], 1)

    def test_publication_between_initial_read_and_block_is_not_lost(self) -> None:
        original = BOARD.read_board
        calls = 0

        def publish_after_snapshot(repository):
            nonlocal calls
            observed = original(repository)
            calls += 1
            if calls == 1:
                self.post_message()
            return observed

        with patch.object(BOARD, "read_board", side_effect=publish_after_snapshot):
            result = BOARD.wait_board(self.repository, since=0, timeout_seconds=2)
        self.assertEqual(result["status"], "messages")
        self.assertEqual(result["messages"][0]["sequence"], 1)
        self.assertEqual(calls, 2)

    def test_existing_messages_filter_before_limit_and_cursor_does_not_skip_matches(self) -> None:
        for topic in ["other", "wanted", "other", "wanted", "other"]:
            self.post_message(topic)
        first = BOARD.wait_board(self.repository, since=0, topic="wanted", limit=1,
                                 timeout_seconds=0)
        self.assertEqual([m["sequence"] for m in first["messages"]], [2])
        self.assertEqual(first["next_since"], 2)
        self.assertTrue(first["has_more"])
        second = BOARD.wait_board(self.repository, since=first["next_since"], topic="wanted",
                                  limit=1, timeout_seconds=0)
        self.assertEqual([m["sequence"] for m in second["messages"]], [4])
        self.assertEqual(second["next_since"], 5)
        self.assertFalse(second["has_more"])
        timeout = BOARD.wait_board(self.repository, since=second["next_since"], topic="wanted",
                                   timeout_seconds=0)
        self.assertEqual(timeout["status"], "timeout")
        self.assertEqual(timeout["next_since"], 5)

    def test_idle_wait_reads_only_initial_and_final_snapshots(self) -> None:
        with patch.object(BOARD, "read_board", wraps=BOARD.read_board) as read:
            result = BOARD.wait_board(self.repository, since=0, timeout_seconds=0.15)
        self.assertEqual(result["status"], "timeout")
        self.assertEqual(read.call_count, 2)
        self.assertIsNone(result["revision"])
        self.assertEqual(result["next_since"], 0)

    def test_all_and_multiple_topics_match_union_without_duplicates(self) -> None:
        for topic in ["alpha", "other", "beta", "alpha"]:
            self.post_message(topic)
        all_topics = BOARD.wait_board(self.repository, since=0, timeout_seconds=0)
        self.assertEqual([m["sequence"] for m in all_topics["messages"]], [1, 2, 3, 4])
        self.assertEqual(all_topics["topics"], [])
        command = [sys.executable, str(SCRIPT), "--repository", str(self.repository),
                   "wait", "--since", "0", "--topic", "alpha", "--topic", "beta",
                   "--topic", "alpha", "--limit", "2", "--timeout-seconds", "0"]
        found = subprocess.run(command, text=True, capture_output=True)
        self.assertEqual(found.returncode, 0, found.stderr)
        first = json.loads(found.stdout)
        self.assertEqual(first["topics"], ["alpha", "beta"])
        self.assertEqual([m["sequence"] for m in first["messages"]], [1, 3])
        self.assertTrue(first["has_more"])
        second = BOARD.wait_board(self.repository, since=first["next_since"],
                                  topics=first["topics"], timeout_seconds=0)
        self.assertEqual([m["sequence"] for m in second["messages"]], [4])
        with self.assertRaises(BOARD.ChatboardError):
            BOARD.wait_board(self.repository, since=0, topics=["alpha", " "], timeout_seconds=0)

    def test_topics_listing_counts_filters_and_pages_without_changing_board(self) -> None:
        command = [sys.executable, str(SCRIPT), "--repository", str(self.repository), "topics"]
        empty = json.loads(subprocess.check_output(command, text=True))
        self.assertEqual(empty["topics"], [])
        self.assertEqual(empty["total_topics"], 0)
        for topic in ["beta", "alpha", "beta", "gamma"]:
            self.post_message(topic)
        before = BOARD.read_board(self.repository)[1]
        page = json.loads(subprocess.check_output(command + ["--limit", "2"], text=True))
        self.assertEqual([t["topic"] for t in page["topics"]], ["alpha", "beta"])
        self.assertEqual(page["topics"][1]["message_count"], 2)
        self.assertEqual(page["topics"][1]["latest_sequence"], 3)
        self.assertEqual(page["next_offset"], 2)
        self.assertEqual(page["total_topics"], 3)
        last = json.loads(subprocess.check_output(command + ["--offset", "2"], text=True))
        self.assertEqual([t["topic"] for t in last["topics"]], ["gamma"])
        self.assertFalse(last["has_more"])
        recent = json.loads(subprocess.check_output(command + ["--since", "2"], text=True))
        self.assertEqual([t["topic"] for t in recent["topics"]], ["beta", "gamma"])
        self.assertEqual(recent["topics"][0]["message_count"], 1)
        self.assertEqual(BOARD.read_board(self.repository)[1], before)
        bad = subprocess.run(command + ["--offset", "-1"], text=True, capture_output=True)
        self.assertEqual(bad.returncode, 1)

    def test_unmatched_post_does_not_return_or_reset_deadline(self) -> None:
        entered = threading.Event()
        original = BOARD.RefChanges

        class SignallingChanges(original):
            def wait(self, timeout):
                entered.set()
                return super().wait(timeout)

        with patch.object(BOARD, "RefChanges", SignallingChanges), ThreadPoolExecutor() as pool:
            waiting = pool.submit(BOARD.wait_board, self.repository, since=0,
                                  topic="wanted", timeout_seconds=0.4)
            self.assertTrue(entered.wait(2))
            self.post_message("other")
            result = waiting.result(timeout=2)
        self.assertEqual(result["status"], "timeout")
        self.assertEqual(result["messages"], [])
        self.assertEqual(result["next_since"], 1)

    def test_packed_ref_update_wakes_waiter(self) -> None:
        self.post_message()
        subprocess.run(["git", "-C", str(self.repository), "pack-refs", "--all", "--prune"], check=True)
        self.assertTrue((self.repository / ".git/packed-refs").exists())
        result = self.wait_for_post(since=1)
        self.assertEqual(result["messages"][0]["sequence"], 2)

    def test_ref_directory_recreated_during_wait_does_not_lose_publication(self) -> None:
        self.post_message()

        def replace_directory():
            subprocess.run(["git", "-C", str(self.repository), "pack-refs", "--all", "--prune"],
                           check=True)
            directory = self.repository / ".git/refs/work-engine/codex-chatboard"
            if directory.exists():
                directory.rmdir()
            self.assertFalse(directory.exists())  # Git may already prune empty directories.

        result = self.wait_for_post(since=1, before_post=replace_directory)
        self.assertEqual(result["messages"][0]["sequence"], 2)

    def test_notification_overflow_is_a_resync_hint(self) -> None:
        with BOARD.RefChanges(self.repository) as changes:
            overflow = changes.EVENT.pack(-1, 0x4000, 0, 0)
            with patch.object(BOARD.select, "select", return_value=([changes.fd], [], [])), \
                 patch.object(BOARD.os, "read", return_value=overflow):
                self.assertTrue(changes.wait(1))

    def test_unsupported_watcher_backend_fails_explicitly(self) -> None:
        with patch.object(BOARD.sys, "platform", "unsupported"):
            with self.assertRaisesRegex(BOARD.ChatboardError, "Linux inotify"):
                BOARD.wait_board(self.repository, since=0, timeout_seconds=0)
        original = BOARD.GitRefDurableStateStore._git

        def reftable(store, args, *rest):
            return b"reftable\n" if args == ["rev-parse", "--show-ref-format"] else original(store, args, *rest)

        with patch.object(BOARD.GitRefDurableStateStore, "_git", reftable):
            with self.assertRaisesRegex(BOARD.ChatboardError, "Git files ref backend"):
                BOARD.wait_board(self.repository, since=0, timeout_seconds=0)

    def test_worktree_wait_uses_common_git_directory(self) -> None:
        subprocess.run(["git", "-C", str(self.repository), "-c", "user.name=Test",
                        "-c", "user.email=test@example.invalid", "commit", "--allow-empty",
                        "-qm", "fixture"], check=True)
        worktree = self.repository / "linked"
        subprocess.run(["git", "-C", str(self.repository), "worktree", "add", "--detach",
                        "-q", str(worktree)], check=True)
        result = self.wait_for_post(repository=worktree)
        board, _ = BOARD.read_board(self.repository)
        self.assertEqual(result["messages"], board["messages"])

    def test_multiple_waiters_observe_without_consuming(self) -> None:
        first = self.post_message()
        with ThreadPoolExecutor() as pool:
            pending = [pool.submit(BOARD.wait_board, self.repository, since=0, timeout_seconds=0)
                       for _ in range(2)]
            for future in pending:
                self.assertEqual(future.result()["messages"], [first["message"]])
        board, revision = BOARD.read_board(self.repository)
        self.assertEqual(revision, first["revision"])
        self.assertEqual(len(board["messages"]), 1)

    def test_timeout_validation_and_cli_exit_contract(self) -> None:
        for kwargs in [{"since": -1}, {"since": 0, "limit": 0},
                       {"since": 0, "topic": " "}, {"since": 0, "timeout_seconds": -1},
                       {"since": 0, "timeout_seconds": float("nan")},
                       {"since": 0, "timeout_seconds": float("inf")}]:
            with self.assertRaises(BOARD.ChatboardError):
                BOARD.wait_board(self.repository, **kwargs)
        command = [sys.executable, str(SCRIPT), "--repository", str(self.repository),
                   "wait", "--since", "0", "--timeout-seconds", "0"]
        empty = subprocess.run(command, text=True, capture_output=True)
        self.assertEqual(empty.returncode, 2, empty.stderr)
        self.assertEqual(json.loads(empty.stdout)["status"], "timeout")
        self.post_message()
        found = subprocess.run(command, text=True, capture_output=True)
        self.assertEqual(found.returncode, 0, found.stderr)
        self.assertEqual(json.loads(found.stdout)["status"], "messages")

    def test_cancellation_closes_watcher_without_mutation(self) -> None:
        observed = []
        original = BOARD.RefChanges

        class CancelledChanges(original):
            def wait(self, timeout):
                observed.append(self)
                raise KeyboardInterrupt

        with patch.object(BOARD, "RefChanges", CancelledChanges):
            with self.assertRaises(KeyboardInterrupt):
                BOARD.wait_board(self.repository, since=0)
        self.assertEqual(observed[0].fd, -1)
        self.assertIsNone(BOARD.read_board(self.repository)[1])


if __name__ == "__main__":
    unittest.main()
