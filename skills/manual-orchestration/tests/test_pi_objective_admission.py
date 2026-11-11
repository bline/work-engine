"""role_host launch pi --objective-record: the usage ledger's pi-admit decides before anything starts."""
import contextlib
import io
import json
import os
import pathlib
import sys
import tempfile
import unittest
from unittest import mock

ROOT = pathlib.Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "scripts"))
import role_host  # noqa: E402

FAKE = """import json, sys
a = sys.argv
out = {"decision": DECISION, "record": a[a.index("--record") + 1], "limit_usd": 1.0, "spent_high_usd": 0.1,
       "committed_usd": 0.2, "exposure_usd": 0.8, "argv": a[1:]}
print(json.dumps(out)); sys.exit({"admitted": 0, "refused": 21}.get(DECISION, 32))
"""


class ObjectiveAdmission(unittest.TestCase):
    def setUp(self):
        self.tmp = pathlib.Path(tempfile.mkdtemp(prefix="rhadm-"))
        self.addCleanup(lambda: __import__("shutil").rmtree(self.tmp, ignore_errors=True))
        (self.tmp / "ws").mkdir()
        self.prompt = self.tmp / "prompt"; self.prompt.write_text("hi")
        self.record = self.tmp / "record.json"; self.record.write_text("{}")

    def fake(self, decision):
        path = self.tmp / f"ledger-{decision}.py"
        path.write_text(FAKE.replace("DECISION", repr(decision)))
        return path

    def run_main(self, decision, *extra, launch=None):
        argv = ["--state", str(self.tmp / "state"), "launch", "pi", "--cwd", str(self.tmp / "ws"), "--name", "n",
                "--prompt-file", str(self.prompt), "--auth", "api", "--container", "--max-usd", "0.5",
                "--objective-record", str(self.record), *extra]
        out, err = io.StringIO(), io.StringIO()
        launch = launch or mock.MagicMock(return_value={"session_id": "S1"})
        with mock.patch.dict(os.environ, {"ROLE_HOST_USAGE_LEDGER": str(self.fake(decision))}), \
                mock.patch.object(role_host.Pi, "launch", launch), \
                contextlib.redirect_stdout(out), contextlib.redirect_stderr(err):
            code = role_host.main(argv)
        return code, out.getvalue() + err.getvalue(), launch

    def ledger_rows(self):
        path = self.tmp / "state" / "ledger.jsonl"
        return [json.loads(l) for l in path.read_text().splitlines()] if path.exists() else []

    def test_admitted_launch_records_the_admission(self):
        code, out, launch = self.run_main("admitted", "--parent", "root")
        self.assertEqual(code, 0, out)
        launch.assert_called_once()
        [row] = [r for r in self.ledger_rows() if r.get("event") == "objective_admission"]
        self.assertEqual((row["session_id"], row["max_usd"], row["objective_record"]), ("S1", 0.5, str(self.record.resolve())))
        self.assertEqual(row["admission"]["decision"], "admitted")
        self.assertEqual(len(row["admission"]["reader_sha256"]), 64)

    def test_refused_and_unavailable_never_launch(self):
        for decision, status in (("refused", "objective_pi_limit_refused"), ("not_established", "objective_accounting_unavailable")):
            code, out, launch = self.run_main(decision, "--parent", "root")
            self.assertEqual(code, role_host.NOT_ESTABLISHED, out)
            self.assertIn(status, out)
            launch.assert_not_called()
        self.assertEqual([r for r in self.ledger_rows() if r.get("event") == "objective_admission"], [])

    def test_parent_and_max_usd_are_required(self):
        code, out, launch = self.run_main("admitted")
        self.assertEqual(code, role_host.USAGE, out)
        launch.assert_not_called()

    def test_reader_gets_the_ledger_path_and_limit(self):
        with mock.patch.dict(os.environ, {"ROLE_HOST_USAGE_LEDGER": str(self.fake("admitted"))}):
            out = role_host.objective_admission(self.record, 0.5, "root", self.tmp / "state" / "ledger.jsonl")
        self.assertIn("--role-host-ledger", out["argv"])
        self.assertEqual(out["argv"][out["argv"].index("--max-usd") + 1], "0.5")


if __name__ == "__main__":
    unittest.main()
