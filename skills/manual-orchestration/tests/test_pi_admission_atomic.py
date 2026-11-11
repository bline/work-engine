"""Atomic Pi admission: separate role_host processes share one per-objective lock and admission journal."""
import json
import os
import pathlib
import subprocess
import sys
import tempfile
import unittest

ROOT = pathlib.Path(__file__).resolve().parents[1]
FAKE_LEDGER = """import json, os, sys, time
a = sys.argv; j = a[a.index("--admission-journal") + 1]; new = float(a[a.index("--max-usd") + 1])
time.sleep(float(os.environ.get("FAKE_SLEEP", "0")))
rows = [json.loads(l) for l in open(j)] if os.path.exists(j) else []
gone = {r["admission_id"] for r in rows if r["event"] == "released"}
held = {r["admission_id"] for r in rows if r["event"] == "pending"} - gone
exposure = 0.6 * len(held) + new
d = "admitted" if exposure <= 1.0 + 1e-9 else "refused"
print(json.dumps({"decision": d, "record": a[a.index("--record") + 1], "exposure_usd": exposure, "limit_usd": 1.0}))
sys.exit(0 if d == "admitted" else 21)
"""
DRIVER = """import os, sys, time
sys.path.insert(0, %r)
import role_host
from unittest import mock
mode = os.environ.get("DRIVER_MODE", "ok")
def launch(self, **kw):
    if mode == "crash":
        os._exit(9)
    if mode == "fail":
        raise role_host.HostError(role_host.NOT_ESTABLISHED, "launch_failed", "docker refused")
    # Mirror the real spawn order: the runner_starting marker precedes anything provider-capable.
    if self.before_runner_start is not None:
        self.before_runner_start(session_id="S-fake", turn=1, spec="spec", container="we-pi-fake")
    if mode == "uncertain":
        raise role_host.HostError(role_host.NOT_ESTABLISHED, "launch_failed", "died after start")
    time.sleep(0.2)
    return {"session_id": "S-%%d" %% os.getpid()}
with mock.patch.object(role_host.Pi, "launch", launch):
    sys.exit(role_host.main(sys.argv[1:]))
""" % str(ROOT / "scripts")


class AtomicAdmission(unittest.TestCase):
    def setUp(self):
        self.tmp = pathlib.Path(tempfile.mkdtemp(prefix="rhatomic-"))
        self.addCleanup(lambda: __import__("shutil").rmtree(self.tmp, ignore_errors=True))
        (self.tmp / "ws").mkdir(); (self.tmp / "prompt").write_text("hi"); (self.tmp / "record.json").write_text("{}")
        (self.tmp / "ledger.py").write_text(FAKE_LEDGER); (self.tmp / "driver.py").write_text(DRIVER)
        self.env = dict(os.environ, ROLE_HOST_USAGE_LEDGER=str(self.tmp / "ledger.py"),
                        ROLE_HOST_OBJECTIVE_LOCK_DIR=str(self.tmp / "locks"))

    def start(self, mode="ok", admission_id=None, sleep="0", state="state"):
        argv = [sys.executable, str(self.tmp / "driver.py"), "--state", str(self.tmp / state), "launch", "pi",
                "--cwd", str(self.tmp / "ws"), "--name", "n", "--prompt-file", str(self.tmp / "prompt"), "--auth", "api",
                "--container", "--max-usd", "0.6", "--objective-record", str(self.tmp / "record.json"), "--parent", "root"]
        if admission_id:
            argv += ["--admission-id", admission_id]
        return subprocess.Popen(argv, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True,
                                env=dict(self.env, DRIVER_MODE=mode, FAKE_SLEEP=sleep))

    def run1(self, *a, **kw):
        p = self.start(*a, **kw); out, err = p.communicate(timeout=60)
        return p.returncode, out + err

    def journal(self):
        files = list((self.tmp / "locks").glob("*.admissions.jsonl"))
        return [json.loads(l) for f in files for l in f.read_text().splitlines()]

    def test_two_concurrent_processes_one_admitted(self):
        # Two 0.60 launches against 1.00 from separate processes (separate --state dirs), reading slowly.
        ps = [self.start(sleep="0.5", state="state-a"), self.start(sleep="0.5", state="state-b")]
        results = [(p.wait(timeout=60), p.stdout.read() + p.stderr.read()) for p in ps]
        codes = sorted(c for c, _ in results)
        self.assertEqual(codes[0], 0, results)
        self.assertNotEqual(codes[1], 0, results)
        self.assertTrue(any("objective_pi_limit_refused" in o for _, o in results), results)
        rows = self.journal()
        self.assertEqual([r["event"] for r in rows], ["pending", "runner_starting", "launched"])

    def test_duplicate_admission_id_never_allocates_twice(self):
        self.assertEqual(self.run1(admission_id="A-1")[0], 0)
        code, out = self.run1(admission_id="A-1")
        self.assertNotEqual(code, 0); self.assertIn("admission_duplicate", out)
        self.assertEqual(sum(r["event"] == "pending" for r in self.journal()), 1)

    def test_failed_launch_before_start_is_released_with_evidence(self):
        self.assertNotEqual(self.run1(mode="fail")[0], 0)
        rows = self.journal()
        self.assertEqual([r["event"] for r in rows], ["pending", "released"])
        self.assertIs(rows[1]["evidence"]["runner_starting"], False)   # positive evidence: no runner marker
        self.assertEqual(self.run1()[0], 0)                       # capacity came back only through the receipt

    def test_uncertain_launch_keeps_the_commitment(self):
        self.assertNotEqual(self.run1(mode="uncertain")[0], 0)
        self.assertEqual([r["event"] for r in self.journal()], ["pending", "runner_starting", "launch_uncertain"])
        code, out = self.run1()
        self.assertNotEqual(code, 0); self.assertIn("objective_pi_limit_refused", out)

    def test_crash_after_reservation_keeps_the_commitment(self):
        self.assertEqual(self.run1(mode="crash")[0], 9)
        self.assertEqual([r["event"] for r in self.journal()], ["pending"])
        code, out = self.run1()                                   # pid gone, no receipt: still committed
        self.assertNotEqual(code, 0); self.assertIn("objective_pi_limit_refused", out)

    def test_corrupted_journal_refuses(self):
        self.assertEqual(self.run1()[0], 0)
        f = next((self.tmp / "locks").glob("*.admissions.jsonl"))
        f.write_text(f.read_text() + "{not json\n")
        code, out = self.run1()
        self.assertNotEqual(code, 0); self.assertIn("objective_accounting_unavailable", out)


if __name__ == "__main__":
    unittest.main()
