import importlib.util
import json
import subprocess
import sys
import tempfile
import time
import unittest
from datetime import datetime, timedelta, timezone
from pathlib import Path


SCRIPT = Path(__file__).parents[1] / "scripts" / "role_scheduler.py"


class RoleSchedulerTest(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.state = Path(self.temp.name)

    def tearDown(self):
        self.cli("stop", check=False)
        self.temp.cleanup()

    def cli(self, *args, check=True):
        result = subprocess.run(
            [sys.executable, str(SCRIPT), "--state-dir", str(self.state), *args],
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            check=check,
        )
        return json.loads(result.stdout) if result.stdout else None

    def schedule(self, item_id, due_at):
        return self.cli(
            "schedule", "--id", item_id, "--repository-id", "repo",
            "--role", "supervisor", "--intent", item_id, "--due-at", due_at,
        )

    def test_agenda_auto_starts_daemon_and_persists_across_restart(self):
        due = (datetime.now(timezone.utc) + timedelta(minutes=5)).isoformat()
        self.schedule("future", due)
        first = self.cli("agenda", "--repository-id", "repo", "--role", "supervisor")
        self.assertEqual(["future"], [item["id"] for item in first["upcoming"]])
        self.cli("stop")
        for _ in range(50):
            if not (self.state / "scheduler.sock").exists():
                break
            time.sleep(0.02)
        second = self.cli("agenda", "--repository-id", "repo", "--role", "supervisor")
        self.assertEqual(["future"], [item["id"] for item in second["upcoming"]])

    def test_wait_blocks_until_due_item_and_presents_once(self):
        waiter = subprocess.Popen(
            [sys.executable, str(SCRIPT), "--state-dir", str(self.state), "wait",
             "--repository-id", "repo", "--role", "supervisor"],
            text=True, stdout=subprocess.PIPE, stderr=subprocess.PIPE,
        )
        time.sleep(0.15)
        due = (datetime.now(timezone.utc) - timedelta(seconds=1)).isoformat()
        self.schedule("due", due)
        stdout, stderr = waiter.communicate(timeout=5)
        self.assertEqual("", stderr)
        event = json.loads(stdout)
        self.assertEqual("due", event["item"]["id"])
        agenda = self.cli("agenda", "--repository-id", "repo", "--role", "supervisor")
        self.assertEqual(["due"], [item["id"] for item in agenda["presented"]])

    def test_acknowledgement_removes_item_from_active_agenda(self):
        due = (datetime.now(timezone.utc) - timedelta(seconds=1)).isoformat()
        self.schedule("done", due)
        self.cli("wait", "--repository-id", "repo", "--role", "supervisor")
        self.cli("acknowledge", "--id", "done", "--result-ref", "test-report:1")
        agenda = self.cli("agenda", "--repository-id", "repo", "--role", "supervisor")
        self.assertFalse(any(agenda[name] for name in ("overdue", "due", "upcoming", "presented")))


if __name__ == "__main__":
    unittest.main()
