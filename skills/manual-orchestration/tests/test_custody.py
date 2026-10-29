from __future__ import annotations

import contextlib
import importlib.util
import io
import json
import os
import stat
import sys
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).parents[1]
spec = importlib.util.spec_from_file_location("custody", ROOT / "scripts" / "custody.py")
assert spec and spec.loader
C = importlib.util.module_from_spec(spec)
spec.loader.exec_module(C)


def call(*argv: str) -> tuple[int, str, str]:
    out, err = io.StringIO(), io.StringIO()
    with contextlib.redirect_stdout(out), contextlib.redirect_stderr(err):
        rc = C.main(list(argv))
    return rc, out.getvalue(), err.getvalue()


class RunTest(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.d = Path(self.tmp.name)
        self.out = self.d / "rec"

    def tearDown(self):
        self.tmp.cleanup()

    def record(self, name):
        return json.loads((self.out / f"{name}.json").read_text())

    def test_success_records_custody_and_refuses_overwrite(self):
        (self.d / "input.txt").write_text("x")
        rc, out, _ = call("run", "--out", str(self.out), "--name", "hello", "--cwd", str(self.d),
                          "--digest", "input.txt", "--", "sh", "-c", "echo hi; echo oops >&2")
        self.assertEqual(rc, 0)
        self.assertIn("hello exit=0", out)
        r = self.record("hello")
        self.assertEqual(r["schema"], C.RUN_SCHEMA)
        self.assertEqual(r["returncode"], 0)
        self.assertIsNone(r["signal"])
        self.assertEqual(r["argv"], ["sh", "-c", "echo hi; echo oops >&2"])
        self.assertEqual(r["wrapper_pid"], os.getpid())
        self.assertEqual(r["pgid"], r["pid"])
        self.assertEqual((self.out / "hello.stdout").read_text(), "hi\n")
        self.assertEqual(r["stdout"]["length"], 3)
        self.assertEqual(r["stderr"]["length"], 5)
        self.assertEqual(r["inputs"][0]["path"], "input.txt")
        self.assertEqual(r["inputs"][0]["length"], 1)
        self.assertTrue(r["env_inherited"])
        self.assertEqual(r["leftover_processes"], [])
        self.assertFalse((self.out / "hello.running.json").exists())
        rc, _, err = call("run", "--out", str(self.out), "--name", "hello", "--", "true")
        self.assertEqual(rc, 2)
        self.assertIn("exists", err)

    def test_failure_and_env_clear(self):
        rc, out, _ = call("run", "--out", str(self.out), "--name", "f", "--cwd", str(self.d), "--env-clear",
                          "--env", "ONLY=1", "--env", "PATH=/usr/bin:/bin",
                          "--", "/bin/sh", "-c", "env > envdump; exit 7")
        self.assertEqual(rc, 1)
        self.assertIn("f exit=7", out)
        r = self.record("f")
        self.assertEqual(r["returncode"], 7)
        self.assertEqual(r["env_set"], {"ONLY": "1", "PATH": "/usr/bin:/bin"})
        self.assertFalse(r["env_inherited"])
        dumped = (self.d / "envdump").read_text().split()
        self.assertIn("ONLY=1", dumped)
        self.assertNotIn("HOME=" + os.environ.get("HOME", "/nonexistent"), dumped)

    def test_leftover_processes_are_recorded_and_can_be_stopped(self):
        rc, out, _ = call("run", "--out", str(self.out), "--name", "bg", "--", "sh", "-c", "sleep 30 & exit 0")
        self.assertEqual(rc, 0)
        self.assertIn("leftovers=1", out)
        r = self.record("bg")
        self.assertEqual(len(r["leftover_processes"]), 1)
        C.kill_group(r["pgid"])
        rc, _, _ = call("run", "--out", str(self.out), "--name", "bg2", "--kill-leftovers",
                        "--", "sh", "-c", "sleep 30 & exit 0")
        r = self.record("bg2")
        self.assertTrue(r["leftovers_killed"])
        self.assertEqual(r["leftovers_after_kill"], [])

    def test_timeout_kills_the_group(self):
        rc, out, _ = call("run", "--out", str(self.out), "--name", "slow", "--timeout", "0.3", "--", "sleep", "20")
        self.assertEqual(rc, 1)
        self.assertIn("exit=timeout", out)
        r = self.record("slow")
        self.assertTrue(r["timed_out"])
        self.assertEqual(r["leftover_processes"], [])

    def test_launch_error_is_recorded(self):
        rc, out, _ = call("run", "--out", str(self.out), "--name", "missing", "--", str(self.d / "nope"))
        self.assertEqual(rc, 1)
        self.assertIn("exit=launch-error", out)
        self.assertIn("FileNotFoundError", self.record("missing")["launch_error"])

    def test_plan_with_cargo_executable_and_stop_at_first_failure(self):
        exe = self.d / "fake-test-bin"
        exe.write_text("#!/bin/sh\necho ran \"$@\"\n")
        exe.chmod(0o755)
        line = json.dumps({"reason": "compiler-artifact", "executable": str(exe),
                           "profile": {"test": True}, "target": {"name": "s10_qual2", "kind": ["lib"]}})
        plan = {"cwd": str(self.d), "steps": [
            {"name": "build", "argv": ["sh", "-c", f"echo 'not json'; echo '{line}'"], "cargo_json": True},
            {"name": "test-one", "argv": ["{exe:build}", "case_one", "--exact"], "digest": ["{exe:build:s10_qual2}"]},
            {"name": "test-fail", "argv": ["sh", "-c", "exit 3"]},
            {"name": "never", "argv": ["true"]}]}
        (self.d / "plan.json").write_text(json.dumps(plan))
        rc, out, _ = call("run", "--out", str(self.out), "--plan", str(self.d / "plan.json"))
        self.assertEqual(rc, 1)
        build = self.record("build")
        self.assertEqual(build["cargo_executables"][0]["path"], str(exe))
        t = self.record("test-one")
        self.assertEqual(t["argv"][0], str(exe))
        self.assertEqual(t["inputs"][0]["sha256"], build["cargo_executables"][0]["sha256"])
        self.assertEqual((self.out / "test-one.stdout").read_text(), "ran case_one --exact\n")
        summary = json.loads((self.out / "summary.json").read_text())
        self.assertEqual(summary["status"], "failed")
        self.assertEqual([s["name"] for s in summary["steps"]], ["build", "test-one", "test-fail"])
        self.assertEqual(summary["not_run"], ["never"])
        self.assertIn("summary failed 2/4 not_run=1", out)

    def test_plan_rejects_unknown_keys_and_bad_placeholder(self):
        (self.d / "p1.json").write_text(json.dumps({"steps": [{"name": "a", "argv": ["true"], "shell": True}]}))
        rc, _, err = call("run", "--out", str(self.out), "--plan", str(self.d / "p1.json"))
        self.assertEqual(rc, 2)
        self.assertIn("unknown keys", err)
        (self.d / "p2.json").write_text(json.dumps({"steps": [{"name": "a", "argv": ["{exe:nothing}"]}]}))
        rc, _, err = call("run", "--out", str(self.out), "--plan", str(self.d / "p2.json"))
        self.assertEqual(rc, 2)
        self.assertIn("no earlier step", err)


class FreezeTest(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.d = Path(self.tmp.name)
        self.src = self.d / "src"
        (self.src / "pkg" / "target").mkdir(parents=True)
        (self.src / "pkg" / "lib.rs").write_text("fn a() {}\n")
        (self.src / "pkg" / "target" / "big.o").write_bytes(b"\0" * 10)
        (self.src / "README").write_text("hello\n")
        os.symlink("README", self.src / "link")

    def tearDown(self):
        self.tmp.cleanup()

    def freeze(self, out, *extra):
        rc, stdout, err = call("freeze", "--root", str(self.src), "--out", str(self.d / out),
                               "--exclude", "target", *extra)
        return rc, (json.loads(stdout) if rc == 0 else err)

    def test_manifest_archive_and_determinism(self):
        rc, a = self.freeze("f1", "--label", "one")
        self.assertEqual(rc, 0)
        manifest = json.loads(Path(a["manifest"]["path"]).read_text())
        self.assertEqual([e["path"] for e in manifest["entries"]], ["README", "link", "pkg/lib.rs"])
        self.assertEqual(manifest["entries"][1]["type"], "symlink")
        self.assertEqual(manifest["entries_sha256"], C.sha256(C.canonical(manifest["entries"])))
        rc, b = self.freeze("f2")
        self.assertEqual(a["archive"]["sha256"], b["archive"]["sha256"])
        self.assertEqual(a["entries_sha256"], b["entries_sha256"])
        mode = stat.S_IMODE(Path(a["manifest"]["path"]).stat().st_mode)
        self.assertEqual(mode, 0o444)
        import tarfile
        with tarfile.open(a["archive"]["path"]) as t:
            self.assertEqual(sorted(t.getnames()), ["README", "link", "pkg/lib.rs"])
            self.assertEqual({m.mtime for m in t.getmembers()}, {0})

    def test_refuses_nonempty_or_inner_out(self):
        rc, _ = self.freeze("f1")
        rc, err = self.freeze("f1")
        self.assertEqual(rc, 2)
        self.assertIn("never overwritten", err)
        rc, _, err = call("freeze", "--root", str(self.src), "--out", str(self.src / "inner"))
        self.assertEqual(rc, 2)

    def test_changes_against_baseline_root_and_manifest(self):
        base = self.d / "base"
        rc, first = self.freeze("f0")
        import shutil
        shutil.copytree(self.src, base, symlinks=True)
        (self.src / "pkg" / "lib.rs").write_text("fn a() { b() }\n")
        (self.src / "NEW").write_text("n\n")
        (self.src / "README").unlink()
        rc, r = self.freeze("f1", "--baseline-root", str(base))
        self.assertEqual(rc, 0)
        self.assertEqual((r["changes"]["changed"], r["changes"]["added"], r["changes"]["removed"]), (1, 1, 1))
        changes = json.loads(Path(r["changes"]["path"]).read_text())
        self.assertEqual(changes["changed"], ["pkg/lib.rs"])
        self.assertEqual(changes["added"], ["NEW"])
        self.assertEqual(changes["removed"], ["README"])
        diff = Path(r["diff"]["path"]).read_text()
        self.assertIn("+fn a() { b() }", diff)
        rc, m = self.freeze("f2", "--baseline-manifest", first["manifest"]["path"])
        self.assertEqual((m["changes"]["changed"], m["changes"]["added"], m["changes"]["removed"]), (1, 1, 1))
        self.assertNotIn("diff", m)


if __name__ == "__main__":
    unittest.main()
