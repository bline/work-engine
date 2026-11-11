"""Launch safety: the real container launch control flow with only the runner Popen faked (no container starts).
A reservation is released only when the journal positively shows no runner_starting marker."""
import contextlib
import io
import json
import os
import pathlib
import subprocess
import sys
import unittest
from unittest import mock

ROOT = pathlib.Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT)); sys.path.insert(0, str(ROOT / "scripts"))
import role_host  # noqa: E402
from tests import test_pi_anthropic as A  # noqa: E402

REAL_POPEN = subprocess.Popen


class FakeRunner:
    pid = 4242424
    def __init__(self): self.stdin = io.BytesIO()
    def poll(self): return None


@unittest.skipIf(A.base.docker_missing() if hasattr(A.base, "docker_missing") else False, "docker or pi missing")
class LaunchSafety(unittest.TestCase):
    def setUp(self):
        self.h = A.RealAnthropicContainerTests("test_tool_turn_custody_usage_and_ledger"); self.h.setUp()
        self.addCleanup(self.h.doCleanups)
        tmp = self.tmp = self.h.tmp
        (tmp / "ledger.py").write_text('import json,sys\na=sys.argv\nprint(json.dumps({"decision":"admitted",'
                                       '"record":a[a.index("--record")+1],"limit_usd":1.0}))\n')
        (tmp / "record.json").write_text("{}"); (tmp / "prompt").write_text("Say OK.")
        self.runners = []

    def popen(self, argv, *a, **kw):
        if "run-turn" in argv:
            self.runners.append(argv); return FakeRunner()
        return REAL_POPEN(argv, *a, **kw)

    def main(self, *patches):
        argv = ["--state", str(self.tmp / "state"), "launch", "pi", "--cwd", str(self.tmp / "ws"), "--name", "safety",
                "--prompt-file", str(self.tmp / "prompt"), "--auth", "api", "--container", "--max-usd", "1",
                "--container-model-file", str(self.h.model_file), "--host-arg=--model", f"--host-arg=anthropic/{A.MODEL_ID}",
                "--objective-record", str(self.tmp / "record.json"), "--parent", "root"]
        env = dict(self.h.env, ROLE_HOST_USAGE_LEDGER=str(self.tmp / "ledger.py"), ROLE_HOST_OBJECTIVE_LOCK_DIR=str(self.tmp / "locks"))
        out = io.StringIO()
        with contextlib.ExitStack() as st:
            st.enter_context(mock.patch.dict(os.environ, env, clear=True))
            st.enter_context(mock.patch.object(subprocess, "Popen", self.popen))
            st.enter_context(mock.patch.object(role_host.Pi, "before_runner_start", None))
            for p in patches: st.enter_context(p)
            st.enter_context(contextlib.redirect_stdout(out)); st.enter_context(contextlib.redirect_stderr(out))
            try:
                code = role_host.main(argv)
            except Exception as e:  # non-HostError failures propagate after the journal row is written
                code = f"raised {type(e).__name__}"
        return code, out.getvalue()

    def journal(self):
        return [json.loads(l) for f in (self.tmp / "locks").glob("*.admissions.jsonl") for l in f.read_text().splitlines()]

    def events(self):
        return [r["event"] for r in self.journal()]

    def boom(self, *a, **kw):
        raise RuntimeError("injected")

    def test_failure_before_marker_releases(self):
        code, out = self.main(mock.patch.object(role_host.Pi, "provider_key", self.boom))
        self.assertNotEqual(code, 0)
        self.assertEqual(self.runners, [])
        self.assertEqual(self.events(), ["pending", "released"], out)
        self.assertIs(self.journal()[-1]["evidence"]["runner_starting"], False)

    def test_marker_write_failure_never_starts_the_runner(self):
        real = role_host.journal_append
        def append(journal, row):
            if row["event"] == "runner_starting": raise OSError("disk full")
            real(journal, row)
        code, out = self.main(mock.patch.object(role_host, "journal_append", append))
        self.assertNotEqual(code, 0)
        self.assertEqual(self.runners, [])
        self.assertEqual(self.events(), ["pending", "released"], out)    # no marker exists: positively not started

    def test_unverified_marker_never_starts_and_is_kept(self):
        code, out = self.main(mock.patch.object(role_host, "journal_has", lambda *a: False))
        self.assertNotEqual(code, 0)
        self.assertEqual(self.runners, [])
        self.assertIn("runner_marker_unverified", out)
        # The marker row exists on disk although verification failed: kept, never released.
        self.assertEqual(self.events(), ["pending", "runner_starting", "launch_uncertain"])

    def test_partial_marker_write_fails_closed(self):
        def append(journal, row):
            if row["event"] == "runner_starting":
                with open(journal, "a") as fh: fh.write('{"event": "runner_sta')
                raise OSError("torn write")
            with open(journal, "a") as fh: fh.write(json.dumps(row) + "\n")
        code, out = self.main(mock.patch.object(role_host, "journal_append", append))
        self.assertEqual(self.runners, [])
        text = next((self.tmp / "locks").glob("*.admissions.jsonl")).read_text()
        self.assertNotIn('"event": "released"', text)                   # unreadable journal: kept, never released
        self.assertIn('"launch_uncertain"', text)

    def test_failure_immediately_after_popen_is_kept(self):
        code, out = self.main(mock.patch.object(role_host.Pi, "_turn_row", self.boom))
        self.assertEqual(len(self.runners), 1)
        self.assertEqual(self.events(), ["pending", "runner_starting", "launch_uncertain"], out)

    def test_launch_receipt_write_failure_is_kept(self):
        real = role_host.Ledger.append
        def append(ledger, row):
            if row.get("event") == "launch": raise OSError("ledger write failed")
            real(ledger, row)
        code, out = self.main(mock.patch.object(role_host.Ledger, "append", append))
        self.assertEqual(len(self.runners), 1)
        self.assertEqual(self.events(), ["pending", "runner_starting", "launch_uncertain"], out)

    def test_successful_launch_records_marker_then_launched(self):
        code, out = self.main(mock.patch.object(role_host.Pi, "_verify_header", lambda *a, **kw: None))
        self.assertEqual(code, 0, out)
        self.assertEqual(len(self.runners), 1)
        self.assertEqual(self.events(), ["pending", "runner_starting", "launched"])
        marker = self.journal()[1]
        self.assertEqual((marker["turn"], marker["container"].startswith("we-pi-")), (1, True))


if __name__ == "__main__":
    unittest.main()
