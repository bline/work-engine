"""Vertical board + real telemetry CLI checks. Set CONTEXT_STATUS_TEST_BINARY."""
from concurrent.futures import ThreadPoolExecutor
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import threading
import unittest
from unittest.mock import patch
from uuid import uuid4

SCRIPT = Path(__file__).parents[1] / "scripts/codex_chatboard.py"
sys.path.insert(0, str(SCRIPT.parent))
SPEC = importlib.util.spec_from_file_location("control_board", SCRIPT)
BOARD = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(BOARD)
SESSION = "11111111-1111-4111-8111-111111111111"
BINARY = os.environ.get("CONTEXT_STATUS_TEST_BINARY")


@unittest.skipUnless(BINARY and sys.platform == "linux", "requires Linux and CONTEXT_STATUS_TEST_BINARY")
class ControlWaitTest(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        subprocess.run(["git", "init", "-q", str(self.root)], check=True)
        self.rollout = self.root / "session.jsonl"
        self.binary = self.root / "context-status"
        shutil.copyfile(BINARY, self.binary)
        self.binary.chmod(0o700)
        self.digest = hashlib.sha256(self.binary.read_bytes()).hexdigest()

    def usage(self, n):
        return json.dumps({"type": "event_msg", "payload": {"type": "token_count", "info": {
            "last_token_usage": {"input_tokens": n, "cached_input_tokens": 0},
            "model_context_window": 258400}}}) + "\n"

    def write(self, n):
        self.rollout.write_text(json.dumps({"type": "session_meta", "payload": {"id": SESSION}})
                                + "\n" + self.usage(n))

    def control(self, cursor=None):
        return BOARD.ContextControl(session_id=SESSION, rollout=self.rollout, binary=self.binary,
                                    sha256=self.digest, trigger_tokens=150000,
                                    band_floor_tokens=171267, cursor=cursor)

    def command(self):
        return [sys.executable, str(SCRIPT), "--repository", str(self.root), "wait", "--since", "0",
                "--session-id", SESSION, "--context-rollout", str(self.rollout),
                "--context-binary", str(self.binary), "--context-sha256", self.digest,
                "--context-trigger-tokens", "150000", "--context-band-floor-tokens", "171267"]

    def post(self, topic="wanted"):
        return BOARD.post(self.root, author="synthetic", session_id=SESSION, topic=topic,
                          body="fixture", references=[], message_id=str(uuid4()), observed_at=BOARD.timestamp())

    def test_control_wakes_on_transcript_append_and_never_publishes(self):
        self.write(100000)
        entered = threading.Event()
        original = BOARD.RefChanges
        class Signalling(original):
            def wait(self, timeout):
                entered.set()
                return super().wait(timeout)
        with patch.object(BOARD, "RefChanges", Signalling), ThreadPoolExecutor() as pool:
            pending = pool.submit(BOARD.wait_board, self.root, since=0, control=self.control(), timeout_seconds=3)
            self.assertTrue(entered.wait(2))
            with self.rollout.open("a") as handle:
                handle.write(self.usage(160000))
            result = pending.result(timeout=4)
        self.assertEqual(result["status"], "control")
        self.assertEqual(result["control_events"][0]["level"], "planning")
        self.assertEqual(result["next_since"], 0)
        self.assertEqual(BOARD.read_board(self.root), (BOARD.empty_board(), None))

    def test_cursor_suppresses_duplicate_but_new_band_wakes(self):
        self.write(160000)
        first = BOARD.wait_board(self.root, since=0, control=self.control(), timeout_seconds=0)
        second = BOARD.wait_board(self.root, since=0, control=self.control(first["next_control_cursor"]), timeout_seconds=0)
        self.assertEqual(second["status"], "timeout")
        self.assertEqual(second["control_events"], [])
        self.write(180000)
        third = BOARD.wait_board(self.root, since=0, control=self.control(second["next_control_cursor"]), timeout_seconds=0)
        self.assertEqual(third["control_events"][0]["level"], "in_band")

    def test_quiet_control_has_no_polling_and_board_page_cursor_is_preserved(self):
        self.write(100000)
        observer = self.control()
        with patch.object(observer, "read", wraps=observer.read) as read:
            result = BOARD.wait_board(self.root, since=0, control=observer, timeout_seconds=.1)
        self.assertEqual(read.call_count, 2)
        self.assertEqual(result["status"], "timeout")
        for _ in range(3): self.post()
        self.write(160000)
        result = BOARD.wait_board(self.root, since=0, limit=1, topic="wanted", control=observer, timeout_seconds=0)
        self.assertEqual(result["next_since"], 1)
        self.assertTrue(result["has_more"])
        self.assertEqual(result["control_events"][0]["level"], "planning")

    def test_reserved_topic_cannot_be_spoofed_and_control_only_filters_board(self):
        self.write(100000)
        topic = self.control().topic
        with self.assertRaisesRegex(BOARD.ChatboardError, "local telemetry"):
            self.post(topic)
        with self.assertRaisesRegex(BOARD.ChatboardError, "session binding"):
            BOARD.wait_board(self.root, since=0, topics=[topic], timeout_seconds=0)
        self.post()
        result = BOARD.wait_board(self.root, since=0, topics=[topic], control=self.control(), timeout_seconds=0)
        self.assertEqual(result["messages"], [])
        self.assertEqual(result["next_since"], 1)

    def test_deletion_is_unavailable_and_replacement_recovers(self):
        self.write(160000)
        observer = self.control()
        high = BOARD.wait_board(self.root, since=0, control=observer, timeout_seconds=0)
        self.rollout.unlink()
        missing = BOARD.wait_board(self.root, since=0, control=observer, timeout_seconds=0)
        self.assertEqual(missing["control_events"][0]["level"], "unavailable")
        self.write(100000)
        recovered = BOARD.wait_board(self.root, since=0, control=observer, timeout_seconds=0)
        self.assertEqual(recovered["control_events"][0]["level"], "normal")
        self.assertNotEqual(high["next_control_cursor"], recovered["next_control_cursor"])

    def test_changed_binary_is_rejected_before_execution(self):
        self.write(160000)
        observer = self.control()
        observer.read()
        self.binary.write_text("#!/bin/sh\nexit 99\n")
        with self.assertRaisesRegex(BOARD.ChatboardError, "digest changed"):
            observer.read()

    def test_own_post_filter_preserves_control_and_reader_errors(self):
        self.write(160000)
        self.post()
        observer = self.control()
        result = BOARD.wait_board(self.root, since=0, topic="wanted", control=observer,
                                  exclude_session_id=SESSION, timeout_seconds=0)
        self.assertEqual(result["status"], "control")
        self.assertEqual(result["messages"], [])
        self.assertEqual(result["control_events"][0]["level"], "planning")
        self.assertEqual(result["next_since"], 1)
        self.assertIsNotNone(result["next_control_cursor"])
        self.binary.write_text("#!/bin/sh\nexit 99\n")
        with self.assertRaisesRegex(BOARD.ChatboardError, "digest changed"):
            BOARD.wait_board(self.root, since=0, control=observer,
                             exclude_session_id=SESSION, timeout_seconds=0)

    def test_follow_rearms_delivers_both_sources_and_exits_at_total_timeout(self):
        self.write(160000)
        process = subprocess.Popen(self.command() + ["--follow", "--timeout-seconds", "1"],
                                   stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
        self.addCleanup(lambda: process.poll() is None and process.kill())
        first = json.loads(process.stdout.readline())
        self.assertEqual(first["status"], "control")
        self.post()
        output, error = process.communicate(timeout=4)
        self.assertEqual(process.returncode, 2, error)
        rows = [json.loads(line) for line in output.splitlines()]
        self.assertEqual([row["status"] for row in rows], ["messages", "timeout"])
        self.assertTrue(all(not row["control_events"] for row in rows))
        self.assertEqual(rows[-1]["next_since"], 1)

    def test_cli_rejects_incomplete_binding_and_reports_control_success(self):
        bad = subprocess.run([sys.executable, str(SCRIPT), "wait", "--since", "0", "--session-id", SESSION],
                             text=True, capture_output=True)
        self.assertEqual(bad.returncode, 1)
        self.write(160000)
        good = subprocess.run(self.command() + ["--timeout-seconds", "0"], text=True, capture_output=True)
        self.assertEqual(good.returncode, 0, good.stderr)
        self.assertEqual(json.loads(good.stdout)["status"], "control")

    def test_follow_total_deadline_does_not_drain_unbounded_ready_backlog(self):
        self.write(100000)
        for _ in range(3): self.post()
        result = subprocess.run(self.command() + ["--follow", "--limit", "1", "--timeout-seconds", "0"],
                                text=True, capture_output=True, timeout=3)
        self.assertEqual(result.returncode, 2, result.stderr)
        rows = [json.loads(line) for line in result.stdout.splitlines()]
        self.assertEqual([row["status"] for row in rows], ["messages", "timeout"])
        self.assertEqual(rows[-1]["next_since"], 1)
        self.assertTrue(rows[-1]["has_more"])


if __name__ == "__main__":
    unittest.main()
