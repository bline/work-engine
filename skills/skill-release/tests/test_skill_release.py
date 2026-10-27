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
        self.install({"claude": ["alpha"], "codex": ["alpha", "beta"]})
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

    def install(self, targets, extra=None):
        """Write a catalog covering every skill directory; installs follow targets."""
        skills = {}
        for d in sorted((self.repo / "skills").iterdir()):
            if (d / "SKILL.md").exists():
                chosen = [t for t in ("claude", "codex") if d.name in targets.get(t, [])]
                skills[d.name] = {"kind": "tool", "state": "active", "install": chosen}
                if not chosen:
                    skills[d.name]["not_installed_reason"] = "test fixture"
        skills.update(extra or {})
        self.write(release.CATALOG_PATH, json.dumps({"schema": release.CATALOG_SCHEMA, "skills": skills}))

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
        self.install({"claude": ["alpha"], "codex": ["alpha"]})
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

    def test_catalog_rules(self):
        targets = {"claude": None, "codex": None}
        entry = lambda **kw: {"kind": "tool", "state": "active", "install": ["claude"], **kw}
        def problems(skills, dirs=None, others=()):
            return " | ".join(release.catalog_problems(
                {"schema": release.CATALOG_SCHEMA, "skills": skills},
                set(skills) if dirs is None else dirs, set(others), targets))
        self.assertEqual(problems({"a": entry()}), "")
        cases = [
            ({"a": entry()}, {"a"}, {"stub"}, "stub: directory has no SKILL.md, so it is not a skill"),
            ({"a": entry()}, {"a", "b"}, (), "b: SKILL.md has no catalog entry"),
            ({"a": entry(), "ghost": entry()}, {"a"}, (), "ghost: catalog entry has no skill directory"),
            ({"a": entry(state="retired")}, None, (), "a: a retired skill is not installed"),
            ({"a": entry(state="dormant")}, None, (), "installed while dormant, so it needs install_reason"),
            ({"a": entry(kind="research", state="stable")}, None, (), "installed while research, so it needs install_reason"),
            ({"a": entry(install=[])}, None, (), "active but not installed, so it needs not_installed_reason"),
            ({"a": entry(owner="x")}, None, (), "unknown fields ['owner']"),
            ({"a": entry(kind="library")}, None, (), "kind must be one of"),
            ({"a": entry(state="paused")}, None, (), "state must be one of"),
            ({"a": entry(install=["cursor"])}, None, (), "install must list unique targets"),
            ({"a": entry(state="dormant", install_reason="  ")}, None, (), "install_reason must be non-empty text"),
        ]
        for skills, dirs, others, expected in cases:
            with self.subTest(expected=expected):
                self.assertIn(expected, problems(skills, dirs, others))
        self.assertEqual(problems({"a": entry(state="dormant", install_reason="kept by decision"),
                                   "b": entry(state="dormant", install=[]),
                                   "c": entry(install=[], not_installed_reason="operator tool")}), "")

    def test_plan_and_deploy_refuse_a_catalog_that_breaks_rules(self):
        self.write("skills/stub/notes.yaml", "x: 1\n")                      # a directory with no SKILL.md
        self.commit("stub")
        for command in ("plan", "deploy"):
            code, _, err = self.run_cli(command)
            self.assertEqual(code, 1)
            self.assertIn("stub: directory has no SKILL.md", err)
        self.assertFalse((self.root / "current").is_symlink())
        self.assertFalse((self.claude / "alpha").exists())

    def test_commit_without_catalog_or_manifest_is_refused(self):
        self.git("rm", "-q", release.CATALOG_PATH)
        self.commit("no catalog")
        code, _, err = self.run_cli("plan")
        self.assertEqual(code, 1)
        self.assertIn("has neither", err)

    def test_older_commits_with_the_install_manifest_still_deploy(self):
        self.git("rm", "-q", release.CATALOG_PATH)
        self.write(release.MANIFEST_PATH, json.dumps({"schema": release.MANIFEST_SCHEMA,
                                                      "targets": {"claude": ["alpha"], "codex": ["beta"]}}))
        legacy = self.commit("legacy manifest")
        code, out, err = self.run_cli("deploy", legacy)
        self.assertEqual(code, 0, err)
        meta = json.loads((self.root / "releases" / legacy / "RELEASE.json").read_text())
        self.assertEqual(meta["install_source"]["path"], release.MANIFEST_PATH)
        self.assertEqual(meta["manifest"], {"claude": ["alpha"], "codex": ["beta"]})

    def test_redeploying_the_current_release_changes_nothing(self):
        self.assertEqual(self.run_cli("deploy")[0], 0)
        links = {p: os.readlink(p) for d in (self.claude, self.codex) for p in d.iterdir()}
        meta = (self.root / "releases" / self.first / "RELEASE.json").read_bytes()
        code, out, err = self.run_cli("deploy")
        self.assertEqual(code, 0, err)
        record = json.loads(out)
        self.assertEqual(record["changes"], {})
        self.assertFalse(record["release_created"])
        self.assertEqual({p: os.readlink(p) for d in (self.claude, self.codex) for p in d.iterdir()}, links)
        self.assertEqual((self.root / "releases" / self.first / "RELEASE.json").read_bytes(), meta)
        self.assertEqual(json.loads(meta)["install_source"]["path"], release.CATALOG_PATH)

    def test_check_validates_the_working_tree_catalog(self):
        code, out, _ = self.run_cli("check")
        self.assertEqual(code, 0)
        self.assertEqual(json.loads(out)["install"], {"claude": ["alpha"], "codex": ["alpha", "beta"]})
        self.write("skills/delta/SKILL.md", skill_md("delta"))
        code, out, _ = self.run_cli("check")
        self.assertEqual(code, 1)
        self.assertIn("delta: SKILL.md has no catalog entry", json.loads(out)["problems"])

    def test_repository_catalog_follows_the_rules(self):
        repository = Path(__file__).resolve().parents[3]
        out = io.StringIO()
        with contextlib.redirect_stdout(out):
            code = release.main(["--repository", str(repository), "check"])
        self.assertEqual(code, 0, out.getvalue())

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
