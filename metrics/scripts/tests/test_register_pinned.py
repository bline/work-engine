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
# PINNED_UNDER_TEST names another register.py to run these tests against (used to show that a test fails on
# an earlier or deliberately broken script); the default is the script next to this tests directory.
SCRIPT = os.environ.get("PINNED_UNDER_TEST") or os.path.abspath(os.path.join(HERE, "..", "register.py"))
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


def run(*args, env=None):
    return subprocess.run([sys.executable, "-I", SCRIPT, *args], capture_output=True, text=True, check=False,
                          env={**os.environ, **(env or {})})


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
            "materialized.jsonl": json.dumps({"finding": "s#f", "label": f"label mark{v}x", "claim_id": f"claim-mark{v}x",
                                              "revision_id": f"rev-mark{v}x", "revision_sha256": f"{v}" * 64}) + "\n",
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

    def validate(self, at=None, env=None):
        return run("validate", "--repo", self.repo, *([f"--at={at}"] if at is not None else []), env=env)

    def build(self, at=None, out=None, env=None):
        out = out or self.out_dir()
        return run("build", "--repo", self.repo, "--out", out, *([f"--at={at}"] if at is not None else []),
                   env=env), out

    def put_blob(self, text):
        return subprocess.run(["git", "-C", self.repo, "hash-object", "-w", "--stdin"], input=text.encode(),
                              capture_output=True, check=True, env=GIT_ENV).stdout.decode().strip()

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
        self.assertEqual(m["outputs"], {n: hashlib.sha256(self.files(out)[n]).hexdigest() for n in VIEWS})
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


class PinnedRemediationTests(PinnedFixture):
    """S4-min r1: regressions for the reproduced review findings (Astra F1, F2, F3, F5; Sonnet F1, F2)."""

    def verified(self, out):
        """The consumer rule: snapshot.json is present and every `outputs` digest matches its view file."""
        path = os.path.join(out, "snapshot.json")
        if not os.path.isfile(path):
            return False
        outputs = json.loads(slurp(path)).get("outputs") or {}
        return sorted(outputs) == sorted(VIEWS) and all(
            os.path.isfile(os.path.join(out, n)) and hashlib.sha256(slurp(os.path.join(out, n))).hexdigest() == d
            for n, d in outputs.items())

    def test_8_working_tree_symlink_does_not_redirect_the_register(self):
        for rel, text in self.data(1).items():
            self.write(os.path.join("metrics", "alternate", rel), text)
        c = self.commit("register holds mark2x, alternate holds mark1x")
        v1, (b1, out1) = self.validate(c), self.build(c)
        self.assertEqual((v1.returncode, b1.returncode), (0, 0), v1.stdout)
        os.rename(self.register, self.register + ".aside")
        os.symlink("alternate", self.register)
        v2, (b2, out2) = self.validate(c), self.build(c)
        self.assertEqual(v2.stdout, v1.stdout)
        self.assertEqual(self.files(out2), self.files(out1))
        self.assertEqual(json.loads(self.files(out2)["snapshot.json"])["register_path"], "metrics/register")
        self.assertIn(b"mark2x", self.files(out2)["current-knowledge.json"])
        self.assertNotIn(b"mark1x", self.files(out2)["current-knowledge.json"])

    def test_9_pinned_pin_checks_ignore_git_overrides_and_replace_refs(self):
        clean = self.validate(self.c1)
        self.assertEqual(clean.returncode, 0, clean.stdout)
        empty = os.path.join(self.base, "empty-objects")
        os.makedirs(empty)
        for env in ({"GIT_DIR": os.path.join(self.base, "missing")}, {"GIT_OBJECT_DIRECTORY": empty}):
            p = self.validate(self.c1, env=env)
            self.assertEqual((p.returncode, p.stdout), (0, clean.stdout), env)
            self.assertEqual(self.build(self.c1, env=env)[0].returncode, 0, env)
        git(self.repo, "replace", self.blob, self.put_blob(""))
        p = self.validate(self.c1)
        self.assertEqual((p.returncode, p.stdout), (0, clean.stdout), "valid pin, blob replaced by an empty one")
        git(self.repo, "replace", "-d", self.blob)
        s = json.loads(self.data(1)["studies/s.json"])
        s["findings"][0]["sources"][0]["lines"] = "1-3"  # the pinned blob has two lines
        c = self.commit_register(1, {"studies/s.json": json.dumps(s)})
        before = self.validate(c)
        self.assertEqual(before.returncode, 1, before.stdout)
        git(self.repo, "replace", self.blob, self.put_blob("result a\nresult b\nFORGED THIRD LINE\n"))
        after = self.validate(c)
        self.assertEqual((after.returncode, after.stdout), (1, before.stdout), "forged third line accepted")
        b, out = self.build(c)
        self.assertEqual(b.returncode, 1, b.stdout)
        self.assertFalse(os.path.exists(out))
        # unpinned validation keeps its old Git access, replace refs included (preserved legacy behaviour)
        self.assertEqual(self.validate().returncode, 0)

    def test_10_materialized_rows_are_read_parsed_and_checked(self):
        for commit, v in ((self.c1, 1), (self.c2, 2)):
            snap = reg.read_snapshot(reg.AnchorGit(self.repo), commit, self.register, self.repo)
            errors = reg.Errors()
            rows = [r for _, r in reg.PinnedRegister(snap, errors).materialized]
            self.assertFalse(errors, errors.items)
            self.assertEqual(rows, [json.loads(self.data(v)["materialized.jsonl"])])
            entry = next(i for i in snap.manifest()["inputs"] if i["path"].endswith("/materialized.jsonl"))
            self.assertEqual(entry["blob"], git(self.repo, "rev-parse", f"{commit}:metrics/register/materialized.jsonl"))
        row = json.loads(self.data(1)["materialized.jsonl"])
        row["finding"] = "absent#finding"
        c3 = self.commit_register(1, {"materialized.jsonl": json.dumps(row) + "\n"})
        v = self.validate(c3)
        self.assertEqual(v.returncode, 1, v.stdout)
        self.assertIn("ERROR unknown-finding-reference materialized.jsonl:1", v.stdout)

    def test_11_publication_never_leaves_a_manifest_beside_views_it_does_not_describe(self):
        b, out = self.build(self.c1)
        self.assertTrue(self.verified(out), b.stdout)
        first = self.files(out)
        manifest = os.path.join(out, "snapshot.json")
        os.unlink(manifest)
        os.makedirs(manifest)  # the manifest name is obstructed: nothing may be replaced
        p, _ = self.build(self.c2, out=out)
        self.assertEqual(p.returncode, 4, p.stdout)
        for name in VIEWS:
            self.assertEqual(slurp(os.path.join(out, name)), first[name], name)
        b, out = self.build(self.c1)
        md = os.path.join(out, "current-knowledge.md")
        os.unlink(md)
        os.makedirs(md)  # a view name is obstructed: no manifest may remain
        p, _ = self.build(self.c2, out=out)
        self.assertEqual(p.returncode, 4, p.stdout)
        self.assertFalse(os.path.exists(os.path.join(out, "snapshot.json")))
        self.assertFalse(self.verified(out))
        self.assertEqual([n for n in os.listdir(out) if n.startswith(".")], [], "temporary files remain")
        b, out = self.build(self.c1)
        u, _ = self.build(out=out)  # unpinned build of the checkout (c2) into the same directory
        self.assertEqual(u.returncode, 0, u.stdout)
        self.assertTrue(os.path.exists(os.path.join(out, "snapshot.json")))
        self.assertFalse(self.verified(out), "a stale snapshot.json must fail the digest check")


def forge_commit_graph(repo, child, parent):
    """Write a commit-graph (generation v1) and make it claim `parent` as `child`'s first parent."""
    import struct
    git(repo, "-c", "commitGraph.generationVersion=1", "commit-graph", "write", "--reachable")
    path = os.path.join(repo, ".git", "objects", "info", "commit-graph")
    b = bytearray(slurp(path))
    chunks = {bytes(b[8 + 12 * i:12 + 12 * i]): struct.unpack(">Q", b[12 + 12 * i:20 + 12 * i])[0]
              for i in range(b[6] + 1)}
    oidf, oidl, cdat = chunks[b"OIDF"], chunks[b"OIDL"], chunks[b"CDAT"]
    n = struct.unpack(">I", b[oidf + 1020:oidf + 1024])[0]
    oids = [bytes(b[oidl + 20 * i:oidl + 20 * i + 20]).hex() for i in range(n)]
    c, p = cdat + 36 * oids.index(child), cdat + 36 * oids.index(parent)
    level = struct.unpack(">Q", b[p + 28:p + 36])[0] >> 34
    b[c + 20:c + 24] = struct.pack(">I", oids.index(parent))
    low = struct.unpack(">Q", b[c + 28:c + 36])[0] & ((1 << 34) - 1)
    b[c + 28:c + 36] = struct.pack(">Q", ((level + 1) << 34) | low)
    b[-20:] = hashlib.sha1(bytes(b[:-20])).digest()
    os.chmod(path, 0o644)
    with open(path, "wb") as fh:
        fh.write(bytes(b))


class PinnedR2Tests(PinnedFixture):
    """S4-min r2: ancestry from metadata outside the commit objects (r1 re-review R1-1; shallow; commit-graph)."""

    def graft(self, text):
        path = os.path.join(self.repo, ".git", "info", "grafts")
        os.makedirs(os.path.dirname(path), exist_ok=True)
        with open(path, "w") as fh:
            fh.write(text + "\n")
        self.addCleanup(lambda: os.path.lexists(path) and os.unlink(path))

    def decision_revisions(self, current, prior):
        d = json.loads(self.data(1)["decisions.json"])
        d[0]["revision"] = f"git:{current}"
        d[0]["revisions"] = [{"revision": f"git:{prior}", "anchor": "old", "superseded_at": "2026-01-01",
                              "reason": "update"}]
        return self.commit_register(1, {"decisions.json": json.dumps(d)})

    def test_12_graft_file_cannot_forge_or_sever_pinned_ancestry(self):
        tree = git(self.repo, "rev-parse", f"{self.c0}^{{tree}}")
        unrelated = git(self.repo, "commit-tree", tree, "-m", "unrelated root, same documents")
        c = self.decision_revisions(unrelated, self.c0)
        before = self.validate(c)
        self.assertEqual(before.returncode, 1, before.stdout)
        self.assertIn("revision-not-ancestor", before.stdout)
        self.graft(f"{unrelated} {self.c0}")  # fabricated parent
        after = self.validate(c)
        self.assertEqual((after.returncode, after.stdout), (1, before.stdout), "graft forged pinned ancestry")
        b, out = self.build(c)
        self.assertEqual(b.returncode, 1, b.stdout)
        self.assertFalse(os.path.exists(out))
        self.assertEqual(self.validate().returncode, 0)  # unpinned validation keeps honouring grafts (legacy)
        child = git(self.repo, "commit-tree", tree, "-p", self.c0, "-m", "child, same documents")
        c2 = self.decision_revisions(child, self.c0)
        ok = self.validate(c2)
        self.assertEqual(ok.returncode, 0, ok.stdout)
        self.graft(child)  # severed: the child shown without parents
        self.assertEqual(self.validate(c2).stdout, ok.stdout, "graft severed pinned ancestry")
        self.assertEqual(self.build(c2)[0].returncode, 0)

    def test_13_shallow_repository_cannot_run_pinned(self):
        self.assertEqual(self.validate(self.c1).returncode, 0)
        path = os.path.join(self.repo, ".git", "shallow")
        with open(path, "w") as fh:
            fh.write(self.c1 + "\n")
        self.addCleanup(lambda: os.path.lexists(path) and os.unlink(path))
        p = self.validate(self.c1)
        self.assertEqual(p.returncode, 4, p.stdout)
        self.assertIn("shallow repository", p.stdout)
        b, out = self.build(self.c1)
        self.assertEqual(b.returncode, 4, b.stdout)
        self.assertFalse(os.path.exists(out))
        self.assertEqual(self.validate().returncode, 0)  # unpinned validation is unchanged

    def test_14_hardened_git_reads_parents_from_commit_objects(self):
        import importlib.util
        tree = git(self.repo, "rev-parse", f"{self.c0}^{{tree}}")
        root = git(self.repo, "commit-tree", tree, "-m", "root without parents")
        git(self.repo, "update-ref", "refs/heads/root", root)
        forge_commit_graph(self.repo, root, self.c0)
        self.assertEqual(git(self.repo, "log", "-1", "--format=%P", root), self.c0)  # the forgery is live
        spec = importlib.util.spec_from_file_location("register_under_test", SCRIPT)
        mod = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(mod)
        self.assertEqual(mod.AnchorGit(self.repo).run("log", "-1", "--format=%P", root).strip(), b"")
        self.assertFalse(mod.PinnedGit(self.repo).is_ancestor(self.c0, root))


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
