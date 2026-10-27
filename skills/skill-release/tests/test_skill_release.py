from __future__ import annotations

import contextlib
import importlib.util
import io
import json
import os
import subprocess
import tempfile
import unittest
from pathlib import Path

SCRIPT = Path(__file__).parents[1] / "scripts" / "skill_release.py"
spec = importlib.util.spec_from_file_location("skill_release", SCRIPT)
release = importlib.util.module_from_spec(spec)
spec.loader.exec_module(release)

GIT_ENV = {"GIT_AUTHOR_NAME": "t", "GIT_AUTHOR_EMAIL": "t@example.invalid",
           "GIT_COMMITTER_NAME": "t", "GIT_COMMITTER_EMAIL": "t@example.invalid"}


def skill_md(name: str, description: str = "does a thing") -> str:
    return f"---\nname: {name}\ndescription: {description}\n---\n\n# {name}\n"


class SkillReleaseTest(unittest.TestCase):
    def setUp(self):
        self.temp = Path(tempfile.mkdtemp())
        self.repo = self.temp / "repo"
        self.root = self.temp / "releases"
        self.backup = self.temp / "backup"
        self.claude = self.temp / "claude-skills"
        self.codex = self.temp / "codex-skills"
        for path in (self.repo, self.claude, self.codex):
            path.mkdir()
        self.git("init", "-q", "-b", "main")
        self.write("skills/alpha/SKILL.md", skill_md("alpha"))
        self.write("skills/alpha/scripts/run.py", "print('alpha')\n", mode=0o755)
        self.write("skills/beta/SKILL.md", skill_md("beta"))
        self.write("skills/gamma/SKILL.md", skill_md("gamma"))
        self.manifest({"claude": ["alpha"], "codex": ["alpha", "beta"]})
        self.first = self.commit("first")

    def tearDown(self):
        if self.root.exists():
            release.remove_tree(self.root)
        release.remove_tree(self.temp)

    # helpers
    def git(self, *args):
        return subprocess.run(["git", "-C", str(self.repo), "-c", "commit.gpgsign=false", *args],
                              check=True, capture_output=True, text=True,
                              env={**os.environ, **GIT_ENV}).stdout.strip()

    def write(self, rel, text, mode=0o644):
        path = self.repo / rel
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(text)
        path.chmod(mode)

    def manifest(self, targets):
        self.write(release.MANIFEST_PATH, json.dumps({"schema": release.MANIFEST_SCHEMA, "targets": targets}))

    def commit(self, message):
        self.git("add", "-A")
        self.git("commit", "-q", "-m", message)
        return self.git("rev-parse", "HEAD")

    def run_cli(self, *args):
        out, err = io.StringIO(), io.StringIO()
        with contextlib.redirect_stdout(out), contextlib.redirect_stderr(err):
            code = release.main(["--repository", str(self.repo), "--release-root", str(self.root),
                                 "--backup-root", str(self.backup),
                                 "--target-dir", f"claude={self.claude}",
                                 "--target-dir", f"codex={self.codex}", *args])
        return code, out.getvalue(), err.getvalue()

    def plan(self, ref="HEAD"):
        code, out, err = self.run_cli("--json", "plan", ref)
        self.assertEqual(code, 0, err)
        return json.loads(out)

    def actions(self, planned, target):
        return {a["name"]: a for a in planned["targets"][target]["actions"]}

    # tests
    def test_plan_classifies_entries_and_changes_nothing(self):
        os.symlink(self.repo / "skills/alpha", self.claude / "alpha")        # dev-tree link
        stale = self.codex / "beta"
        stale.mkdir()
        (stale / "SKILL.md").write_text(skill_md("beta", "old description"))
        (self.codex / "unrelated").mkdir()
        planned = self.plan()
        claude, codex = self.actions(planned, "claude"), self.actions(planned, "codex")
        self.assertEqual(claude["alpha"]["action"], "retarget")
        self.assertEqual(claude["alpha"]["change"]["files"], {"differ": 0, "missing": 0, "extra": 0})
        self.assertFalse(claude["alpha"]["change"]["listing_changes"])
        self.assertEqual(codex["alpha"]["action"], "link")
        self.assertEqual(codex["beta"]["action"], "replace-copy")
        self.assertTrue(codex["beta"]["change"]["listing_changes"])
        self.assertEqual(planned["targets"]["codex"]["unmanaged"], ["unrelated"])
        self.assertFalse(self.root.exists())

    def test_deploy_refuses_real_copy_without_flag_before_changing_anything(self):
        (self.codex / "beta").mkdir()
        code, _, err = self.run_cli("deploy")
        self.assertEqual(code, 1)
        self.assertIn("--replace-copies", err)
        self.assertFalse((self.root / "current").is_symlink())
        self.assertFalse((self.root / "releases" / self.first).exists())
        self.assertFalse((self.claude / "alpha").exists())

    def test_deploy_builds_read_only_release_and_links_every_target(self):
        stale = self.codex / "beta"
        stale.mkdir()
        (stale / "SKILL.md").write_text("old")
        code, out, err = self.run_cli("deploy", "--replace-copies")
        self.assertEqual(code, 0, err)
        record = json.loads(out)
        built = self.root / "releases" / self.first
        self.assertEqual(os.readlink(self.root / "current"), f"releases/{self.first}")
        for target, name in (("claude", "alpha"), ("codex", "alpha"), ("codex", "beta")):
            entry = (self.claude if target == "claude" else self.codex) / name
            self.assertEqual(os.readlink(entry), str(self.root / "current/skills" / name))
            self.assertEqual(entry.resolve(), (built / "skills" / name).resolve())
        self.assertFalse(os.access(built / "skills/alpha/SKILL.md", os.W_OK))
        self.assertFalse(os.access(built / "skills/alpha", os.W_OK))
        self.assertTrue(os.access(built / "skills/alpha/scripts/run.py", os.X_OK))
        self.assertEqual(release.summary(release.verify_release(self.repo, release.Store(self.root), self.first)),
                         "identical")
        meta = json.loads((built / "RELEASE.json").read_text())
        self.assertEqual(meta["manifest"], {"claude": ["alpha"], "codex": ["alpha", "beta"]})
        backup = Path(record["changes"]["codex"][1]["backup"])
        self.assertEqual((backup / "SKILL.md").read_text(), "old")
        self.assertFalse(str(backup).startswith(str(self.codex)))
        self.assertEqual([p.name for p in self.codex.iterdir() if p.name.startswith(".")], [])

    def test_second_deploy_switches_current_keeps_old_release_and_unlinks_dropped(self):
        self.assertEqual(self.run_cli("deploy")[0], 0)
        self.write("skills/alpha/SKILL.md", skill_md("alpha", "new description"))
        self.manifest({"claude": ["alpha"], "codex": ["alpha"]})
        second = self.commit("second")
        planned = self.plan()
        claude, codex = self.actions(planned, "claude"), self.actions(planned, "codex")
        self.assertEqual(claude["alpha"]["action"], "keep")
        self.assertTrue(claude["alpha"]["change"]["listing_changes"])
        self.assertEqual(codex["beta"]["action"], "unlink")
        code, out, err = self.run_cli("deploy")
        self.assertEqual(code, 0, err)
        self.assertEqual(json.loads(out)["previous"], self.first)
        self.assertIn("new description", (self.claude / "alpha/SKILL.md").read_text())
        self.assertFalse((self.codex / "beta").is_symlink())
        self.assertTrue((self.root / "releases" / self.first).is_dir())
        code, out, err = self.run_cli("rollback")
        self.assertEqual(code, 0, err)
        self.assertEqual(os.readlink(self.root / "current"), f"releases/{self.first}")
        self.assertNotIn("new description", (self.claude / "alpha/SKILL.md").read_text())
        self.assertTrue((self.codex / "beta").is_symlink())
        self.assertEqual(second, json.loads(out)["previous"])

    def test_redeploying_current_keeps_rollback_target(self):
        self.run_cli("deploy")
        self.write("skills/beta/SKILL.md", skill_md("beta", "changed"))
        second = self.commit("second")
        self.run_cli("deploy")
        code, out, _ = self.run_cli("deploy")
        self.assertEqual(code, 0)
        self.assertEqual(json.loads(out)["previous"], self.first)
        self.assertEqual(json.loads(out)["commit"], second)

    def test_retire_moves_unmanaged_and_refuses_managed(self):
        self.run_cli("deploy")
        (self.codex / "stale-skill").mkdir()
        (self.codex / "stale-skill/SKILL.md").write_text("x")
        code, _, err = self.run_cli("retire", "codex", "alpha")
        self.assertEqual(code, 1)
        self.assertIn("managed", err)
        code, out, err = self.run_cli("retire", "codex", "stale-skill")
        self.assertEqual(code, 0, err)
        moved = Path(json.loads(out)["moved"][0]["backup"])
        self.assertEqual((moved / "SKILL.md").read_text(), "x")
        self.assertFalse((self.codex / "stale-skill").exists())

    def test_status_reports_undeployed_skill_changes(self):
        self.run_cli("deploy")
        self.write("skills/gamma/SKILL.md", skill_md("gamma", "edited, not committed"))
        self.write("skills/delta/SKILL.md", skill_md("delta"))           # untracked
        self.write("skills/beta/__pycache__/x.pyc", "ignored")
        (self.repo / ".gitignore").write_text("__pycache__/\n")
        index_before = (self.repo / ".git/index").read_bytes()
        code, out, _ = self.run_cli("status", "--verify")
        self.assertEqual(code, 0)
        result = json.loads(out)
        self.assertEqual(result["undeployed_changes"], ["delta", "gamma"])
        self.assertEqual((self.repo / ".git/index").read_bytes(), index_before)
        self.assertEqual(result["release_verification"], "identical")
        self.assertEqual(result["targets"]["claude"]["not_linked"], [])

    def test_manifest_must_exist_and_name_real_skills(self):
        self.manifest({"claude": ["missing"]})
        self.commit("bad manifest")
        code, _, err = self.run_cli("plan")
        self.assertEqual(code, 1)
        self.assertIn("missing has no SKILL.md", err)
        self.git("rm", "-q", release.MANIFEST_PATH)
        self.commit("no manifest")
        code, _, err = self.run_cli("plan")
        self.assertEqual(code, 1)
        self.assertIn("has no", err)

    def test_tampered_release_is_refused(self):
        self.run_cli("deploy")
        target = self.root / "releases" / self.first / "skills/alpha/SKILL.md"
        os.chmod(target.parent, 0o755)
        os.chmod(target, 0o644)
        target.write_text("tampered")
        self.write("skills/beta/SKILL.md", skill_md("beta", "changed"))
        self.commit("second")
        self.run_cli("deploy")
        code, _, err = self.run_cli("rollback")
        self.assertEqual(code, 1)
        self.assertIn("does not match", err)


if __name__ == "__main__":
    unittest.main()
