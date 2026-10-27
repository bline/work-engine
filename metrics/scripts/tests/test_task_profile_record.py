"""Tests for metrics/scripts/task_profile_record.py. Run: python3 -m unittest discover -s metrics/scripts/tests

Uses the agent-routing-lab checkout at the pinned commit (AGENT_ROUTING_LAB, default
~/code/agent-routing-lab) through a stand-in tool directory, and a synthetic repository, packet,
sidecar and usage ledger in a temporary directory. Skips if the checkout is not at the pin.
"""
import json
import os
import subprocess
import sys
import tempfile
import unittest

HERE = os.path.dirname(os.path.abspath(__file__))
SCRIPT = os.path.join(os.path.dirname(HERE), "task_profile_record.py")
sys.path.insert(0, os.path.dirname(HERE))
import task_profile_record as rec  # noqa: E402

LAB = os.environ.get("AGENT_ROUTING_LAB", os.path.expanduser("~/code/agent-routing-lab"))
GIT_ENV = dict(os.environ, GIT_AUTHOR_NAME="f", GIT_AUTHOR_EMAIL="f@example.invalid", GIT_COMMITTER_NAME="f",
               GIT_COMMITTER_EMAIL="f@example.invalid", GIT_AUTHOR_DATE="2026-01-01T00:00:00Z",
               GIT_COMMITTER_DATE="2026-01-01T00:00:00Z")


def lab_head():
    r = subprocess.run(["git", "-C", LAB, "rev-parse", "HEAD"], capture_output=True, text=True)
    return r.stdout.strip() if r.returncode == 0 else None


def write(path, text):
    os.makedirs(os.path.dirname(path), exist_ok=True)
    with open(path, "w") as fh:
        fh.write(text)


@unittest.skipUnless(lab_head() == rec.PIN["commit"], "agent-routing-lab checkout is not at the pinned commit")
class RecorderTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        t = self.tmp.name
        # stand-in tool directory for the pinned commit
        self.tools_root = os.path.join(t, "tools")
        self.tool_dir = os.path.join(self.tools_root, rec.PIN["commit"])
        write(os.path.join(self.tool_dir, "bin", "taskprofile"),
              f"#!/bin/sh\nexec {sys.executable} -I -c \"import sys; sys.path.insert(0, '{LAB}/src'); "
              "from agent_routing_lab.taskprofile import main; main()\" \"$@\"\n")
        os.chmod(os.path.join(self.tool_dir, "bin", "taskprofile"), 0o755)
        write(os.path.join(self.tool_dir, "lib", "python3", "site-packages", "agent_routing_lab-0.2.0.dist-info", "direct_url.json"),
              json.dumps({"url": rec.PIN["repository"], "vcs_info": {"vcs": "git", "commit_id": rec.PIN["commit"]}}))
        # repository, packet, sidecar
        self.repo = os.path.join(t, "repo")
        os.makedirs(self.repo)
        subprocess.run(["git", "-C", self.repo, "init", "-q", "-b", "main"], check=True, env=GIT_ENV)
        write(os.path.join(self.repo, "rust/crates/alpha/src/lib.rs"), "pub fn a() {}\n")
        subprocess.run(["git", "-C", self.repo, "add", "-A"], check=True, env=GIT_ENV)
        subprocess.run(["git", "-C", self.repo, "commit", "-q", "-m", "f"], check=True, env=GIT_ENV)
        commit = subprocess.run(["git", "-C", self.repo, "rev-parse", "HEAD"], capture_output=True, text=True, env=GIT_ENV).stdout.strip()
        self.packet = os.path.join(t, "packet.md")
        write(self.packet, "# Change alpha\nEdit rust/crates/alpha/src/lib.rs.\n")
        self.run_dir = os.path.join(t, "run")
        self.sidecar = self.make_sidecar("builder", commit)
        self.ledger = os.path.join(t, "ledger")

    def tearDown(self):
        self.tmp.cleanup()

    def make_sidecar(self, role, commit):
        out = os.path.join(self.tmp.name, f"sidecar-{role}.json")
        self.cli_lab(["adapt", "markdown", "--task-id", "alpha-change", "--target-role", role, "--packet", self.packet, "-o", out])
        with open(out) as fh:
            s = json.load(fh)
        s["input_snapshot"]["git"] = {"status": "declared", "value": {"commit": commit}, "basis": "test"}
        s["provenance"]["declared_by"] = {"status": "declared", "value": {
            "role": "coordinator", "model": "model-x", "effort": "high", "harness": "test", "guidance_sha256": "0" * 64}}
        with open(out, "w") as fh:
            json.dump(s, fh)
        return out

    def cli_lab(self, args):
        return subprocess.run([os.path.join(self.tool_dir, "bin", "taskprofile")] + args, capture_output=True, text=True, check=True)

    def cli(self, *args, check=True):
        r = subprocess.run([sys.executable, "-I", SCRIPT] + list(args), capture_output=True, text=True)
        if check and r.returncode != 0:
            self.fail(f"{args[0]} failed: {r.stdout} {r.stderr}")
        return r

    def profile(self, sidecar=None):
        r = self.cli("profile", "--run-dir", self.run_dir, "--sidecar", sidecar or self.sidecar, "--repo", self.repo,
                     "--declare-started-at", "2026-01-01T00:00:00+00:00", "--tool-dir", self.tool_dir)
        return json.loads(r.stdout)

    def assign(self, rev, child="sess-1", *extra):
        return self.cli("assign", "--run-dir", self.run_dir, "--task-revision", rev, "--role", "builder",
                        "--requested-model", "model-b", "--requested-effort", "medium", "--harness", "h", "--run", "r",
                        "--dispatched-packet", self.packet, "--child-session", child, *extra)

    def ledger_rows(self, rows):
        write(os.path.join(self.ledger, "calls.jsonl"), "".join(json.dumps(r) + "\n" for r in rows))

    def verify(self, *extra):
        return self.cli("verify", "--run-dir", self.run_dir, "--repo", self.repo, "--tools-root", self.tools_root, *extra, check=False)

    def test_full_cycle_verifies(self):
        out = self.profile()
        self.assertTrue(out["valid"])
        self.assign(out["task_revision"])
        self.ledger_rows([{"provider": "anthropic", "session": "sess-1", "model": "model-b", "effort": "medium", "ts": "2026-01-01T00:01:00Z"},
                          {"provider": "anthropic", "session": "other", "model": "model-z", "effort": "low", "ts": "2026-01-01T00:02:00Z"}])
        att = json.loads(self.cli("attribute", "--run-dir", self.run_dir, "--task-revision", out["task_revision"], "--ledger", self.ledger).stdout)
        self.assertEqual((att["status"], att["models_seen"]), ("resolved", ["model-b"]))
        r = self.verify()
        self.assertEqual(r.returncode, 0, r.stdout)
        self.assertIn("verified", r.stdout)

    def test_substitution_is_reported(self):
        out = self.profile()
        self.assign(out["task_revision"], "thread-9")
        self.ledger_rows([{"provider": "codex", "thread": "thread-9", "model": "model-c", "effort": "medium", "ts": "2026-01-01T00:01:00Z"}])
        self.cli("attribute", "--run-dir", self.run_dir, "--task-revision", out["task_revision"], "--ledger", self.ledger)
        d = rec.record_dir(self.run_dir, out["task_revision"])
        with open(os.path.join(d, "attribution-assignment.json")) as fh:
            self.assertFalse(json.load(fh)["model_matches_request"])

    def test_unknown_child_is_unresolvable(self):
        out = self.profile()
        self.assign(out["task_revision"], "unknown")
        self.ledger_rows([])
        att = json.loads(self.cli("attribute", "--run-dir", self.run_dir, "--task-revision", out["task_revision"], "--ledger", self.ledger).stdout)
        self.assertTrue(att["status"].startswith("unresolvable"))

    def test_builder_and_reviewer_profiles_are_separate_revisions(self):
        a = self.profile()
        commit = subprocess.run(["git", "-C", self.repo, "rev-parse", "HEAD"], capture_output=True, text=True).stdout.strip()
        b = self.profile(self.make_sidecar("reviewer", commit))
        self.assertNotEqual(a["task_revision"], b["task_revision"])

    def test_verify_fails_without_sums(self):
        out = self.profile()
        os.remove(os.path.join(rec.record_dir(self.run_dir, out["task_revision"]), "SHA256SUMS"))
        self.assertEqual(self.verify().returncode, 1)

    def test_verify_fails_on_unlisted_file(self):
        out = self.profile()
        write(os.path.join(rec.record_dir(self.run_dir, out["task_revision"]), "extra.json"), "{}")
        self.assertEqual(self.verify().returncode, 1)

    def test_verify_catches_edit_even_if_sums_rewritten(self):
        out = self.profile()
        d = rec.record_dir(self.run_dir, out["task_revision"])
        write(os.path.join(d, "validation.json"), '{"valid": true, "errors": [], "edited": 1}\n')
        with open(os.path.join(d, "SHA256SUMS"), "w") as fh:  # rewrite the index as an editor might
            fh.writelines(f"{rec.sha256(os.path.join(d, n))}  {n}\n" for n in rec.record_files(d))
        r = self.verify()
        self.assertEqual(r.returncode, 1)
        self.assertIn("overhead.json recorded validation.json with a different hash", r.stdout)

    def test_append_refuses_changed_listed_file(self):
        out = self.profile()
        d = rec.record_dir(self.run_dir, out["task_revision"])
        write(os.path.join(d, "validation.json"), "{}\n")
        r = self.cli("assign", "--run-dir", self.run_dir, "--task-revision", out["task_revision"], "--role", "builder",
                     "--requested-model", "m", "--requested-effort", "e", "--harness", "h", "--run", "r",
                     "--dispatched-packet", self.packet, "--child-session", "s", check=False)
        self.assertNotEqual(r.returncode, 0)
        self.assertIn("no longer matches", r.stderr + r.stdout)

    def test_skipped_recompute_is_not_reported_as_verified(self):
        self.profile()
        r = self.verify("--no-recompute")
        self.assertEqual(r.returncode, 2)
        self.assertIn("recompute not done", r.stdout)

    def test_invalid_revision_is_refused(self):
        r = self.cli("assign", "--run-dir", self.run_dir, "--task-revision", "a b@1", "--role", "builder",
                     "--requested-model", "m", "--requested-effort", "e", "--harness", "h", "--run", "r",
                     "--dispatched-packet", self.packet, "--child-session", "s", check=False)
        self.assertNotEqual(r.returncode, 0)
        self.assertIn("not a valid task revision id", r.stderr + r.stdout)

    def test_leak_check(self):
        out = self.profile()
        leak = os.path.join(self.tmp.name, "leaky.md")
        write(leak, f"context: {out['task_revision']}\n")
        r = self.cli("assign", "--run-dir", self.run_dir, "--task-revision", out["task_revision"], "--role", "builder",
                     "--requested-model", "m", "--requested-effort", "e", "--harness", "h", "--run", "r",
                     "--dispatched-packet", leak, "--child-session", "s")
        self.assertFalse(json.loads(r.stdout)["profile_absent_from_dispatch"])

    # ---- second source review (recorder) ----
    def run_profile(self, sidecar=None, tool_dir=None):
        return self.cli("profile", "--run-dir", self.run_dir, "--sidecar", sidecar or self.sidecar, "--repo", self.repo,
                        "--declare-started-at", "2026-01-01T00:00:00+00:00", "--tool-dir", tool_dir or self.tool_dir, check=False)

    def test_missing_tool_is_recorded_not_blocking(self):
        r = self.run_profile(tool_dir=os.path.join(self.tmp.name, "no-tool"))
        self.assertEqual(r.returncode, 0, r.stderr)
        out = json.loads(r.stdout)
        self.assertEqual((out["failure"], out["dispatch"]), ("tool", "may continue"))
        with open(os.path.join(out["record_dir"], "profile-failed.json")) as fh:
            self.assertEqual(json.load(fh)["stage"], "tool")

    def test_invalid_sidecar_is_recorded_not_blocking(self):
        bad = os.path.join(self.tmp.name, "bad.json")
        write(bad, "{not json")
        r = self.run_profile(sidecar=bad)
        self.assertEqual(r.returncode, 0, r.stderr)
        self.assertEqual(json.loads(r.stdout)["failure"], "read-sidecar")

    def test_duplicate_profile_exits_3(self):
        self.profile()
        r = self.run_profile()
        self.assertEqual(r.returncode, 3)
        self.assertIn("already profiled", r.stdout)

    def test_verify_with_no_records_exits_3(self):
        os.makedirs(os.path.join(self.run_dir, "task-profiles"), exist_ok=True)
        r = self.verify()
        self.assertEqual(r.returncode, 3)
        self.assertIn("no task-profile records", r.stdout)

    def test_timestamps_and_selection_order(self):
        out = self.profile()
        d = rec.record_dir(self.run_dir, out["task_revision"])
        with open(os.path.join(d, "overhead.json")) as fh:
            frozen = json.load(fh)["frozen_at"]
        self.assertRegex(frozen, r"\.\d{6}\+00:00$", "microsecond timestamps")
        self.assign(out["task_revision"], "s1", "--selected-at", "2000-01-01T00:00:00+00:00")
        self.assign(out["task_revision"], "s1", "--selected-at", "2999-01-01T00:00:00+00:00", "--retained")
        with open(os.path.join(d, "assignment.json")) as fh:
            first = json.load(fh)
        with open(os.path.join(d, "assignment-02.json")) as fh:
            second = json.load(fh)
        self.assertFalse(first["profile_frozen_before_selection"])
        self.assertTrue(second["profile_frozen_before_selection"])
        self.assertTrue(first["profile_frozen_before_assignment_record"])

    def test_attribution_window(self):
        out = self.profile()
        self.assign(out["task_revision"], "sess-w")
        self.ledger_rows([{"provider": "anthropic", "session": "sess-w", "model": "old-model", "effort": "low", "ts": "2026-01-01T00:00:00Z"},
                          {"provider": "anthropic", "session": "sess-w", "model": "model-b", "effort": "medium", "ts": "2026-01-02T00:00:00Z"}])
        att = json.loads(self.cli("attribute", "--run-dir", self.run_dir, "--task-revision", out["task_revision"], "--ledger", self.ledger,
                                  "--since", "2026-01-01T12:00:00+00:00").stdout)
        self.assertEqual(att["models_seen"], ["model-b"])

    def test_sources_archived_and_checked(self):
        out = self.profile()
        d = rec.record_dir(self.run_dir, out["task_revision"])
        with open(os.path.join(d, "overhead.json")) as fh:
            sources = json.load(fh)["sources"]
        self.assertTrue(sources and all(v["archived"] for v in sources.values()))
        name = next(iter(sources))
        write(os.path.join(d, "sources", name), "tampered\n")
        r = self.verify()
        self.assertEqual(r.returncode, 1)
        self.assertIn("does not match its hash", r.stdout)


if __name__ == "__main__":
    unittest.main()
