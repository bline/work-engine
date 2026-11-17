from __future__ import annotations

import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import time
import unittest
from uuid import uuid4

SCRIPTS = Path(__file__).parents[1] / "scripts"
JOB = SCRIPTS / "chatboard_job.py"
BOARD = SCRIPTS / "codex_chatboard.py"


class ChatboardJobTest(unittest.TestCase):
    def setUp(self) -> None:
        self.temporary = tempfile.TemporaryDirectory()
        root = Path(self.temporary.name)
        self.repository = root / "repo"
        subprocess.run(["git", "init", "-q", str(self.repository)], check=True)
        self.env = {**os.environ, "XDG_STATE_HOME": str(root / "state")}
        self.session = str(uuid4())

    def tearDown(self) -> None:
        self.temporary.cleanup()

    def start(self, *command: str, extra: tuple[str, ...] = ()) -> dict:
        out = subprocess.run(
            [sys.executable, str(JOB), "--repository", str(self.repository), "run",
             "--author", "tester", "--session-id", self.session, *extra, "--", *command],
            capture_output=True, text=True, check=True, env=self.env,
        ).stdout
        return json.loads(out)

    def wait(self, job: dict, timeout: int = 30) -> dict:
        out = subprocess.run(
            [sys.executable, str(BOARD), "--repository", str(self.repository), "wait",
             "--since", str(job["since"]), "--topic", job["topic"],
             "--exclude-session-id", self.session, "--timeout-seconds", str(timeout)],
            capture_output=True, text=True, check=True,
        ).stdout
        return json.loads(out)

    def record(self, message: dict) -> dict:
        # The supervisor records `posted` just after its board post lands.
        path = Path(message["references"][0])
        for _ in range(100):
            record = json.loads(path.read_text())
            if "posted" in record:
                return record
            time.sleep(0.05)
        return record

    def test_returns_at_once_and_the_wait_wakes_on_completion(self) -> None:
        started = time.monotonic()
        job = self.start("bash", "-c", "sleep 2; echo secret-output; exit 3")
        self.assertLess(time.monotonic() - started, 2, "run must not block on the command")
        self.assertEqual(job["topic"], f"job:{self.session}")
        self.assertIn(f"--session-id {self.session}", job["wait"])
        result = self.wait(job)
        [message] = result["messages"]
        self.assertIn("exit 3", message["body"])
        self.assertNotIn("secret-output", message["body"], "output is not posted by default")
        record = self.record(message)
        self.assertEqual(record["exit"], 3)
        self.assertTrue(record["posted"])
        self.assertEqual(Path(record["stdout"]).read_text(), "secret-output\n")

    def test_tail_lines_and_unstartable_command(self) -> None:
        job = self.start("printf", "a\\nb\\nc\\n", extra=("--tail-lines", "2", "--name", "tail"))
        body = self.wait(job)["messages"][0]["body"]
        self.assertIn("b\nc", body)
        self.assertNotIn("a\nb", body)
        missing = self.start("/nonexistent/command")
        record = self.record(self.wait(missing)["messages"][0])
        self.assertEqual(record["exit"], 127)


if __name__ == "__main__":
    unittest.main()
