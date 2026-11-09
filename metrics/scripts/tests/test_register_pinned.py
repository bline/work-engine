"""Tests for `register.py validate --at` and `build --at` (S4-min: the commit-pinned register reader).

Fixtures are temporary git repositories; the real-register tests read the repository at a pinned commit and
never touch its working tree, index or refs.
"""
import hashlib
import importlib.util
import io
import json
import os
import shutil
import subprocess
import sys
import tarfile
import tempfile
import unittest

HERE = os.path.dirname(os.path.abspath(__file__))
SCRIPT = os.path.abspath(os.path.join(HERE, "..", "register.py"))
REAL_REPO = os.path.abspath(os.path.join(HERE, "..", "..", ".."))
S4_BASE = "c8a50e177e12b5d799430160fe802bb2d8637c6f"  # the S4-min base commit; its register has 9 studies
GIT_ENV = {**os.environ, "GIT_AUTHOR_NAME": "t", "GIT_AUTHOR_EMAIL": "t@example.invalid",
           "GIT_COMMITTER_NAME": "t", "GIT_COMMITTER_EMAIL": "t@example.invalid"}
GIT_ENV.pop("GIT_EDITOR", None)
TOP_FILES = ("applicability.jsonl", "decisions.json", "events.jsonl", "judgments.jsonl", "materialized.jsonl",
             "questions.json")
VIEWS = ("current-knowledge.json", "current-knowledge.md")

spec = importlib.util.spec_from_file_location("register_pinned_under_test", SCRIPT)
reg = importlib.util.module_from_spec(spec)
spec.loader.exec_module(reg)


def git(repo, *args, raw=False):
    p = subprocess.run(["git", "-C", repo, *args], capture_output=True, check=True, env=GIT_ENV)
    return p.stdout if raw else p.stdout.decode().strip()


def slurp(path):
    with open(path, "rb") as fh:
        return fh.read()


def digest_tree(root):
    out = {}
    for base, dirs, files in os.walk(root):
        dirs.sort()
        for d in dirs:
            out[os.path.relpath(os.path.join(base, d), root) + "/"] = None
        for f in sorted(files):
            p = os.path.join(base, f)
            out[os.path.relpath(p, root)] = "link" if os.path.islink(p) else hashlib.sha256(slurp(p)).hexdigest()
    return out


def run(*args):
    return subprocess.run([sys.executable, "-I", SCRIPT, *args], capture_output=True, text=True, check=False)


def own_lines(stdout):
    """The command's output without the snapshot manifest lines."""
    return [ln for ln in stdout.splitlines() if not ln.startswith("snapshot ")]


class PinnedFixture(unittest.TestCase):
    def setUp(self):
        self.base = tempfile.mkdtemp(prefix="pinned-test-")
        self.addCleanup(shutil.rmtree, self.base, ignore_errors=True)
        self.repo = os.path.join(self.base, "repo")
        self.register = os.path.join(self.repo, "metrics", "register")
        os.makedirs(os.path.join(self.repo, "docs"))
        git(self.repo, "init", "-q")
        self.write("docs/study.md", "result a\nresult b\n")
        self.write("docs/model.md", "row one\nrow two\n")
        self.c0 = self.commit("docs")
        self.blob = git(self.repo, "rev-parse", f"{self.c0}:docs/study.md")
        self.c1 = self.commit_register(1)
        self.c2 = self.commit_register(2)
        self.n_out = 0

    def write(self, rel, data):
        full = os.path.join(self.repo, rel)
        os.makedirs(os.path.dirname(full), exist_ok=True)
        with open(full, "wb") as fh:
            fh.write(data.encode() if isinstance(data, str) else data)

    def commit(self, msg):
        git(self.repo, "add", "-A")
        git(self.repo, "commit", "-q", "-m", msg)
        return git(self.repo, "rev-parse", "HEAD")

    def data(self, v):
        """A valid register; every text that differs between v=1 and v=2 carries the marker `mark<v>x`."""
        pin = {"path": "docs/study.md", "commit": self.c0, "blob": self.blob}
        finding = lambda slug: {"id": f"s#{slug}", "claim": f"claim {slug} mark{v}x", "value": v, "n": 3,
                                "basis": "measured", "scope": f"scope mark{v}x", "caveats": [f"caveat mark{v}x"],
                                "sources": [dict(pin, lines="1-2")]}
        study = {"id": "s", "title": f"Study mark{v}x", "kind": "pilot", "questions": ["Q1"],
                 "lifecycle": [{"date": "2026-01-01", "state": "concluded", "basis": pin}],
                 "stop_reason_class": None, "findings": [finding("f"), finding("g")], "sources": [pin],
                 "aliases": ["K1"]}
        owner = "user (fixture owner)"
        return {
            "questions.json": json.dumps([{"id": "Q1", "text": f"Question mark{v}x?", "area": "a"}]),
            "studies/s.json": json.dumps(study),
            "events.jsonl": json.dumps({"event_id": "ev", "kind": "challenge", "date": "2026-01-03",
                                        "subject_finding": "s#f", "by_finding": "s#g", "basis": f"basis mark{v}x",
                                        "source": pin}) + "\n",
            "judgments.jsonl": json.dumps({"finding": "s#f", "axis": "evidence_strength",
                                           "value": ["moderate", "low"][v - 1], "basis": f"judged mark{v}x",
                                           "assessor": "t", "date": "2026-01-04"}) + "\n",
            "decisions.json": json.dumps([{"decision_id": "d", "document": "docs/model.md", "anchor": f"anchor mark{v}x",
                                           "revision": f"git:{self.c0}", "owner": owner, "findings": ["s#f"]}]),
            "applicability.jsonl": json.dumps({
                "applicability_id": "ap", "finding": "s#f", "decision_id": "d", "decision_revision": f"git:{self.c0}",
                "relation": "supports", "rationale": f"rationale mark{v}x", "evidence": [pin], "proposed_by": "t",
                "proposed_at": "2026-01-02", "authorization": {"status": "pending", "owner": owner}}) + "\n",
            "materialized.jsonl": "",
        }

    def commit_register(self, v, changes=None):
        files = self.data(v)
        files.update(changes or {})
        for rel, text in files.items():
            path = os.path.join(self.register, rel)
            if text is None:
                if os.path.lexists(path):
                    os.unlink(path)
            else:
                self.write(os.path.join("metrics", "register", rel), text)
        return self.commit(f"register mark{v}x")

    def out_dir(self):
        self.n_out += 1
        return os.path.join(self.base, f"out{self.n_out}")

    def validate(self, at=None):
        return run("validate", "--repo", self.repo, *([f"--at={at}"] if at is not None else []))

    def build(self, at=None, out=None):
        out = out or self.out_dir()
        return run("build", "--repo", self.repo, "--out", out, *([f"--at={at}"] if at is not None else [])), out

    def files(self, out):
        return {name: slurp(os.path.join(out, name)) for name in sorted(os.listdir(out))}

    def dirty(self):
        """Rewrite every register file on disk, add a study, and leave the changes uncommitted."""
        for rel in (*TOP_FILES, "studies/s.json"):
            self.write(os.path.join("metrics", "register", rel), "not json {\n")
        self.write("metrics/register/studies/extra.json", "{}")


class PinnedTests(PinnedFixture):
    def test_1_dirty_tree_does_not_change_pinned_output(self):
        v1 = self.validate(self.c1)
        b1, out1 = self.build(self.c1)
        self.assertEqual((v1.returncode, b1.returncode), (0, 0), v1.stdout + b1.stdout)
        self.dirty()
        self.assertEqual(self.validate().returncode, 1, "the dirty tree must really be invalid")
        for step in ("rewritten", "register directory removed"):
            if step == "register directory removed":
                shutil.rmtree(self.register)
            v2 = self.validate(self.c1)
            b2, out2 = self.build(self.c1)
            self.assertEqual(v2.stdout, v1.stdout, step)
            self.assertEqual(v2.returncode, 0, step)
            self.assertEqual(b2.stdout.replace(out2, "OUT"), b1.stdout.replace(out1, "OUT"), step)
            self.assertEqual(self.files(out2), self.files(out1), step)

    def test_2_two_commits_do_not_contaminate_each_other(self):
        b1, out1 = self.build(self.c1)
        b2, out2 = self.build(self.c2)
        self.assertEqual((b1.returncode, b2.returncode), (0, 0))
        for out, own, other in ((out1, "mark1x", "mark2x"), (out2, "mark2x", "mark1x")):
            view = self.files(out)
            for name in VIEWS:
                self.assertIn(own.encode(), view[name])
                at = view[name].find(other.encode())
                self.assertEqual(at, -1, f"{name} at {own} carries {other}: {view[name][max(0, at - 60):at + 20]!r}")
        # in one process, with a shared object cache, reading c2 first leaves c1's snapshot unchanged
        shared = reg.AnchorGit(self.repo)
        s2 = reg.read_snapshot(shared, self.c2, self.register, self.repo)
        s1 = reg.read_snapshot(shared, self.c1, self.register, self.repo)
        fresh = reg.read_snapshot(reg.AnchorGit(self.repo), self.c1, self.register, self.repo)
        self.assertEqual(s1.files, fresh.files)
        self.assertEqual(s1.manifest(), fresh.manifest())
        self.assertNotEqual(s1.register_tree, s2.register_tree)
        errors = reg.Errors()
        pinned = reg.PinnedRegister(s1, errors)
        reg.validate_register(pinned, reg.Git(self.repo))
        self.assertFalse(errors, errors.items)
        self.assertEqual(tuple(t.encode() for t in reg.render_view(pinned, self.repo)),
                         tuple(self.files(out1)[n] for n in VIEWS))

    def test_3_tree_and_blob_identities_match_git(self):
        b, out = self.build(self.c1)
        self.assertEqual(b.returncode, 0, b.stdout)
        m = json.loads(self.files(out)["snapshot.json"])
        self.assertEqual(m["schema"], "register_snapshot_manifest_v1")
        self.assertEqual(m["commit"], git(self.repo, "rev-parse", self.c1))
        self.assertEqual(m["register_path"], "metrics/register")
        self.assertEqual(m["register_tree"], git(self.repo, "rev-parse", f"{self.c1}:metrics/register"))
        expected = sorted(f"metrics/register/{n}" for n in (*TOP_FILES, "studies/s.json"))
        self.assertEqual([i["path"] for i in m["inputs"]], expected)
        self.assertEqual(m["missing"], [])
        for i in m["inputs"]:
            self.assertEqual(i["blob"], git(self.repo, "rev-parse", f"{self.c1}:{i['path']}"), i["path"])
            data = git(self.repo, "cat-file", "blob", i["blob"], raw=True)
            self.assertEqual(i["sha256"], hashlib.sha256(data).hexdigest(), i["path"])
            self.assertEqual(i["mode"], "100644")
        script = slurp(SCRIPT)
        self.assertEqual(m["tool"], {"name": "register.py", "sha256": hashlib.sha256(script).hexdigest(),
                                     "git_blob_sha1": subprocess.run(
                                         ["git", "hash-object", "--stdin"], input=script, cwd=self.repo,
                                         capture_output=True, check=True).stdout.decode().strip()})
        # the validate report carries the same manifest
        v = self.validate(self.c1)
        lines = [ln for ln in v.stdout.splitlines() if ln.startswith("snapshot input ")]
        self.assertEqual(lines, [f"snapshot input {i['mode']} {i['blob']} {i['sha256']} {i['path']}"
                                 for i in m["inputs"]])
        self.assertIn(f"commit {m['commit']} register metrics/register tree {m['register_tree']}", v.stdout)

    def test_4_pinned_equals_unpinned_on_a_clean_checkout(self):
        for commit in (self.c1, self.c2):
            git(self.repo, "checkout", "-q", "--detach", commit)
            self.assertEqual(git(self.repo, "status", "--porcelain"), "")
            vu, vp = self.validate(), self.validate(commit)
            self.assertEqual((vu.returncode, vp.returncode), (0, 0), vp.stdout)
            self.assertEqual(own_lines(vp.stdout), vu.stdout.splitlines())
            (bu, out_u), (bp, out_p) = self.build(), self.build(commit)
            self.assertEqual((bu.returncode, bp.returncode), (0, 0), bp.stdout)
            self.assertEqual(sorted(os.listdir(out_u)), sorted(VIEWS), "unpinned build writes only the view")
            self.assertEqual(sorted(os.listdir(out_p)), sorted((*VIEWS, "snapshot.json")))
            for name in VIEWS:
                self.assertEqual(slurp(os.path.join(out_p, name)), slurp(os.path.join(out_u, name)), name)

    def test_5_unresolved_commit_or_register_cannot_run(self):
        study_tree = git(self.repo, "rev-parse", f"{self.c1}:metrics/register/studies")
        c3 = self.commit_register(2, {rel: None for rel in (*TOP_FILES, "studies/s.json")})
        shutil.rmtree(self.register)
        self.write("metrics/register", "now a file\n")
        c4 = self.commit("register path is a file")
        cases = {"no-such-ref": "does not resolve", "-x": "does not resolve", "0" * 40: "does not resolve",
                 self.blob: "does not resolve", study_tree: "does not resolve",
                 self.c0: "metrics/register is not a directory", c3: "metrics/register is not a directory",
                 c4: "metrics/register is not a directory"}
        for at, reason in cases.items():
            v = self.validate(at)
            b, out = self.build(at)
            for p, cmd in ((v, "validate"), (b, "build")):
                self.assertEqual(p.returncode, 4, (at, p.stdout))
                self.assertIn(f"{cmd}: cannot run", p.stdout, at)
                self.assertIn(reason, p.stdout, at)
                self.assertNotIn("snapshot ", p.stdout, at)
            self.assertFalse(os.path.exists(out), at)
        outside = run("validate", "--repo", self.repo, "--register", self.base, f"--at={self.c1}")
        self.assertEqual(outside.returncode, 4, outside.stdout)
        self.assertIn("is outside the repository", outside.stdout)

    def test_5b_missing_or_malformed_inputs_are_explicit_failures(self):
        c3 = self.commit_register(1, {"events.jsonl": None, "judgments.jsonl": "{bad\n",
                                      "questions.json": b"\xff\xfe not utf-8"})
        os.makedirs(os.path.join(self.register, "studies", "dir.json"))
        self.write("metrics/register/studies/dir.json/x", "x")
        os.unlink(os.path.join(self.register, "decisions.json"))
        os.symlink("applicability.jsonl", os.path.join(self.register, "decisions.json"))
        c4 = self.commit("symlinked decisions, directory named like a study")
        expect = {c3: ["ERROR missing-file events.jsonl: not found at", "ERROR bad-json judgments.jsonl:1:",
                       "ERROR bad-encoding questions.json: not UTF-8 at",
                       "snapshot missing metrics/register/events.jsonl: not found at"],
                  c4: ["ERROR missing-file decisions.json: not a regular file at", "(mode 120000, blob)",
                       "ERROR missing-file studies/dir.json: not a regular file at", "(mode 040000, tree)"]}
        for commit, needles in expect.items():
            v = self.validate(commit)
            b, out = self.build(commit)
            self.assertEqual((v.returncode, b.returncode), (1, 1), v.stdout)
            for needle in needles:
                self.assertIn(needle, v.stdout)
            self.assertIn("build: refused, the register does not validate", b.stdout)
            self.assertFalse(os.path.exists(out))
        shutil.rmtree(os.path.join(self.register, "studies"))
        c5 = self.commit("no studies directory")
        v = self.validate(c5)
        self.assertEqual(v.returncode, 1)
        self.assertIn("ERROR missing-file studies/: directory not found", v.stdout)
        self.assertIn("snapshot missing metrics/register/studies/: directory not found at", v.stdout)

    def test_6_repeat_runs_are_byte_identical(self):
        runs = [self.build(at) for at in (self.c1, self.c1, "HEAD~1", self.c1[:12])]
        for b, _ in runs:
            self.assertEqual(b.returncode, 0, b.stdout)
        first = self.files(runs[0][1])
        for _, out in runs[1:]:
            self.assertEqual(self.files(out), first)
        self.assertEqual(self.validate(self.c1).stdout, self.validate(self.c1).stdout)

    def test_7_failed_pinned_runs_write_nothing_and_change_nothing(self):
        self.dirty()  # an uncommitted state that must survive untouched
        before = digest_tree(self.repo)
        c_missing = git(self.repo, "rev-parse", "HEAD")  # valid commit; failures below come from inputs
        attempts = [("validate", "--at=no-such-ref"), ("build", "--at=no-such-ref"),
                    ("build", f"--at={self.c0}")]
        outs = []
        for cmd, at in attempts:
            out = self.out_dir()
            outs.append(out)
            args = [cmd, "--repo", self.repo, at] + (["--out", out] if cmd == "build" else [])
            self.assertEqual(run(*args).returncode, 4)
        in_register = os.path.join(self.register, "view")
        in_git = os.path.join(self.repo, ".git", "view")
        for out in (in_register, in_git):
            p = run("build", "--repo", self.repo, "--out", out, f"--at={c_missing}")
            self.assertEqual(p.returncode, 2, p.stdout)
            self.assertIn("build: refused", p.stdout)
        self.assertEqual(digest_tree(self.repo), before)
        for out in outs + [in_register, in_git]:
            self.assertFalse(os.path.exists(out), out)
        # a symbolic link at an output name is refused before anything is written
        out = self.out_dir()
        os.makedirs(out)
        target = os.path.join(self.base, "target")
        os.symlink(target, os.path.join(out, "snapshot.json"))
        p = run("build", "--repo", self.repo, "--out", out, f"--at={self.c1}")
        self.assertEqual(p.returncode, 2, p.stdout)
        self.assertEqual(sorted(os.listdir(out)), ["snapshot.json"])
        self.assertFalse(os.path.exists(target))


class RealRegisterPinnedTests(unittest.TestCase):
    """The real register at the S4-min base: pinned commands equal unpinned ones run on that commit's
    register files, extracted with `git archive` (read-only for the repository)."""

    def setUp(self):
        self.base = tempfile.mkdtemp(prefix="pinned-real-")
        self.addCleanup(shutil.rmtree, self.base, ignore_errors=True)
        data = git(REAL_REPO, "archive", "--format=tar", S4_BASE, "metrics/register", raw=True)
        with tarfile.open(fileobj=io.BytesIO(data)) as tar:
            tar.extractall(self.base, filter="data")
        self.extracted = os.path.join(self.base, "metrics", "register")

    def test_pinned_validate_and_build_equal_the_clean_register(self):
        vu = run("validate", "--repo", REAL_REPO, "--register", self.extracted)
        vp = run("validate", "--repo", REAL_REPO, f"--at={S4_BASE}")
        self.assertEqual((vu.returncode, vp.returncode), (0, 0), vp.stdout)
        self.assertEqual(own_lines(vp.stdout), vu.stdout.splitlines())
        out_u, out_p = os.path.join(self.base, "u"), os.path.join(self.base, "p")
        bu = run("build", "--repo", REAL_REPO, "--register", self.extracted, "--out", out_u)
        bp = run("build", "--repo", REAL_REPO, f"--at={S4_BASE}", "--out", out_p)
        self.assertEqual((bu.returncode, bp.returncode), (0, 0), bp.stdout)
        for name in VIEWS:
            self.assertEqual(slurp(os.path.join(out_p, name)), slurp(os.path.join(out_u, name)), name)
        m = json.loads(slurp(os.path.join(out_p, "snapshot.json")))
        self.assertEqual(m["commit"], S4_BASE)
        self.assertEqual(m["register_tree"], git(REAL_REPO, "rev-parse", f"{S4_BASE}:metrics/register"))
        self.assertEqual(len(m["inputs"]), 15)  # 6 files and 9 studies
        self.assertEqual(m["missing"], [])


if __name__ == "__main__":
    unittest.main()
