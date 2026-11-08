"""Tests for `register.py anchors` (standard library unittest). T1-T15 of the Slice A1 packet.

T1-T13 build temporary git repositories; T14 and T15 use the real repository at a pinned commit.
"""
import hashlib
import json
import os
import shutil
import subprocess
import sys
import tempfile
import unittest

HERE = os.path.dirname(os.path.abspath(__file__))
SCRIPT = os.path.abspath(os.path.join(HERE, "..", "register.py"))
REAL_REPO = os.path.abspath(os.path.join(HERE, "..", "..", ".."))
BASE_COMMIT = "14031bc23320b84b79149b62cf447a6594326438"
GIT_ENV = {**os.environ, "GIT_AUTHOR_NAME": "t", "GIT_AUTHOR_EMAIL": "t@example.invalid",
           "GIT_COMMITTER_NAME": "t", "GIT_COMMITTER_EMAIL": "t@example.invalid"}
GIT_ENV.pop("GIT_EDITOR", None)


def git(repo, *args, raw=False):
    p = subprocess.run(["git", "-C", repo, *args], capture_output=True, check=True, env=GIT_ENV)
    return p.stdout if raw else p.stdout.decode().strip()


def digest_tree(root):
    """relative path -> sha256 of every file under root (directories listed too)."""
    out = {}
    for base, dirs, files in os.walk(root):
        dirs.sort()
        for d in dirs:
            out[os.path.relpath(os.path.join(base, d), root) + "/"] = None
        for f in sorted(files):
            p = os.path.join(base, f)
            if os.path.islink(p):
                out[os.path.relpath(p, root)] = "link"
                continue
            with open(p, "rb") as fh:
                out[os.path.relpath(p, root)] = hashlib.sha256(fh.read()).hexdigest()
    return out


def slurp(path):
    with open(path, "rb") as fh:
        return fh.read()


def lines_text(*lines):
    return "".join(ln + "\n" for ln in lines)


class AnchorFixture(unittest.TestCase):
    def setUp(self):
        self.base = tempfile.mkdtemp(prefix="anchors-test-")
        self.addCleanup(shutil.rmtree, self.base, ignore_errors=True)
        self.repo = os.path.join(self.base, "repo")
        os.makedirs(self.repo)
        git(self.repo, "init", "-q")
        self.n_out = 0

    # repository helpers
    def write(self, path, text):
        full = os.path.join(self.repo, path)
        os.makedirs(os.path.dirname(full), exist_ok=True)
        with open(full, "w") as fh:
            fh.write(text)

    def commit_files(self, files, msg="c"):
        """files: path -> text to write, or None to delete. Returns the full commit id."""
        for path, text in files.items():
            if text is None:
                git(self.repo, "rm", "-q", "-f", path)
            else:
                self.write(path, text)
        git(self.repo, "add", "-A")
        git(self.repo, "commit", "-q", "-m", msg)
        return git(self.repo, "rev-parse", "HEAD")

    def decision(self, did, doc, rev, lines=None, findings=("s#f",)):
        d = {"decision_id": did, "document": doc, "anchor": f"anchor of {did}", "revision": f"git:{rev[:10]}",
             "owner": "user (fixture owner)", "findings": list(findings)}
        if lines is not None:
            d["lines"] = lines
        return d

    def appl(self, aid, did, rev, status="pending"):
        auth = {"status": status, "owner": "user (fixture owner)"}
        if status != "pending":
            auth["at"] = "2026-01-01"
        return {"applicability_id": aid, "finding": "s#f", "decision_id": did, "decision_revision": f"git:{rev[:10]}",
                "relation": "supports", "rationale": "because", "evidence": [], "proposed_by": "t",
                "proposed_at": "2026-01-01", "authorization": auth}

    def commit_register(self, decisions, appl=(), extra=None, msg="register"):
        reg = os.path.join(self.repo, "metrics", "register")
        os.makedirs(reg, exist_ok=True)
        with open(os.path.join(reg, "decisions.json"), "w") as fh:
            json.dump(decisions, fh)
        with open(os.path.join(reg, "applicability.jsonl"), "w") as fh:
            for r in appl:
                fh.write(json.dumps(r) + "\n")
        files = dict(extra or {})
        return self.commit_files(files, msg) if files else self._commit_all(msg)

    def _commit_all(self, msg):
        git(self.repo, "add", "-A")
        git(self.repo, "commit", "-q", "-m", msg)
        return git(self.repo, "rev-parse", "HEAD")

    # running the command
    def out_dir(self):
        self.n_out += 1
        return os.path.join(self.base, f"out{self.n_out}")

    def anchors(self, at=None, out=None, repo=None, register=None):
        out = out or self.out_dir()
        args = [sys.executable, "-I", SCRIPT, "anchors", "--repo", repo or self.repo, "--out", out]
        if at:
            args.append(f"--at={at}")
        if register:
            args += ["--register", register]
        p = subprocess.run(args, capture_output=True, text=True, check=False)
        return p, out

    def report(self, at=None):
        p, out = self.anchors(at)
        self.assertEqual(p.returncode, 0, p.stdout + p.stderr)
        with open(os.path.join(out, "anchors.json"), encoding="utf-8") as fh:
            return json.load(fh)

    @staticmethod
    def by_id(rep):
        return {e["decision_id"]: e for e in rep["decisions"]}

    def one(self, doc_v1, doc_v2, lines, doc="docs/a.md", rename_to=None, did="d"):
        """Build v1 -> v2 history, register at v1, return the single decision's entry at the final commit."""
        rev = self.commit_files({doc: doc_v1}, "v1")
        files = {doc: doc_v2}
        if rename_to:
            git(self.repo, "mv", doc, rename_to)
            files = {rename_to: doc_v2}
        self.commit_files(files, "v2")
        self.commit_register([self.decision(did, doc, rev, lines)])
        return self.by_id(self.report())[did]


DOC = lines_text("# Title", "", "alpha", "beta", "gamma", "delta", "epsilon", "zeta")


class ClassTests(AnchorFixture):
    def test_t1_unchanged(self):
        # a later edit elsewhere in the file leaves the anchored lines at the same numbers
        e = self.one(DOC, DOC.replace("zeta", "ZETA"), "4-5")
        self.assertEqual(e["class"], "unchanged")
        self.assertEqual(e["after"]["lines"], "4-5")
        self.assertFalse(e["owner_review_required"])
        self.assertEqual(e["before"]["anchored_text_sha256"],
                         hashlib.sha256(b"beta\ngamma\n").hexdigest())
        self.assertEqual(e["document_at_target"]["path"], "docs/a.md")

    def test_t2_moved_with_context_and_no_flag(self):
        v2 = DOC.replace("# Title\n\n", "# Title\n\nnew one\nnew two\n")
        e = self.one(DOC, v2, "4-5")
        self.assertEqual(e["class"], "moved")
        self.assertEqual((e["before"]["lines"], e["after"]["lines"]), ("4-5", "6-7"))
        self.assertFalse(e["owner_review_required"])
        self.assertEqual(e["context_before"]["heading"]["text"], "# Title")
        self.assertEqual(e["context_after"]["heading"]["text"], "# Title")
        self.assertEqual(e["context_before"]["before"][-1], "alpha")
        self.assertEqual(e["context_after"]["before"][-1], "alpha")
        self.assertEqual(e["context_after"]["after"][0], "delta")

    def test_t3_one_line_of_three_edited_is_modified(self):
        e = self.one(DOC, DOC.replace("gamma", "GAMMA EDITED"), "4-6")
        self.assertEqual(e["class"], "modified")
        self.assertIsNone(e["after"])
        self.assertEqual([c["lines"] for c in e["candidates"]], ["4-6"])
        self.assertIn("not chosen", e["candidates"][0]["basis"])
        self.assertFalse(e["owner_review_required"])

    def test_t4_deleted_lines(self):
        # all three anchored lines deleted and the text occurs nowhere: nothing survives, so absent
        e = self.one(DOC, DOC.replace("beta\ngamma\ndelta\n", ""), "4-6")
        self.assertEqual(e["class"], "absent")
        self.assertEqual(e["candidates"], [])
        self.assertIsNotNone(e["document_at_target"])
        # only the middle line deleted: the range can still be followed to the target, text differs: modified
        self.setUp()
        e = self.one(DOC, DOC.replace("gamma\n", ""), "4-6")
        self.assertEqual(e["class"], "modified")
        self.assertEqual([c["lines"] for c in e["candidates"]], ["4-5"])

    def test_t5_document_deleted_is_absent(self):
        rev = self.commit_files({"docs/a.md": DOC, "docs/keep.md": "keep\n"}, "v1")
        self.commit_files({"docs/a.md": None}, "v2")
        self.commit_register([self.decision("d", "docs/a.md", rev, "3-4")])
        e = self.by_id(self.report())["d"]
        self.assertEqual(e["class"], "absent")
        self.assertIsNone(e["document_at_target"])

    def test_t6_rename_with_content_intact(self):
        e = self.one(DOC, DOC, "4-5", rename_to="docs/renamed.md")
        self.assertEqual(e["class"], "unchanged")
        self.assertTrue(e["document_at_target"]["renamed"])
        self.assertEqual(e["document_at_target"]["path"], "docs/renamed.md")
        self.assertEqual(e["document_at_target"]["rename_status"], "R100")
        self.assertEqual(e["after"]["lines"], "4-5")

    def test_t6b_rename_and_insertion_is_moved(self):
        e = self.one(DOC, DOC.replace("# Title\n\n", "# Title\n\nnew one\n"), "4-5", rename_to="docs/renamed.md")
        self.assertEqual(e["class"], "moved")
        self.assertEqual(e["after"]["lines"], "5-6")
        self.assertEqual(e["document_at_target"]["path"], "docs/renamed.md")

    def test_t7_duplicated_block_is_ambiguous_with_every_occurrence(self):
        e = self.one(DOC, DOC + "\n" + lines_text("beta", "gamma"), "4-5")
        self.assertEqual(e["class"], "ambiguous")
        self.assertEqual([c["lines"] for c in e["candidates"]], ["4-5", "10-11"])
        self.assertIsNone(e["after"])  # no copy is chosen
        # three occurrences
        self.setUp()
        e = self.one(DOC, DOC + lines_text("beta", "gamma") * 2, "4-5")
        self.assertEqual([c["lines"] for c in e["candidates"]], ["4-5", "9-10", "11-12"])

    def test_t8_section_moved_verbatim_is_relocated_and_flagged(self):
        v1 = lines_text("# Doc", "", "## One", "", "first", "anchored a", "anchored b", "last of one", "",
                        "## Two", "", "second", "end")
        v2 = lines_text("# Doc", "", "## One", "", "first", "last of one", "",
                        "## Two", "", "second", "anchored a", "anchored b", "end")
        e = self.one(v1, v2, "6-7")
        self.assertEqual(e["class"], "relocated_verbatim")
        self.assertTrue(e["owner_review_required"])
        self.assertEqual(e["owner_review_reasons"], ["relocated_verbatim"])
        self.assertEqual(e["after"]["lines"], "11-12")
        self.assertEqual(e["context_before"]["heading"]["text"], "## One")
        self.assertEqual(e["context_after"]["heading"]["text"], "## Two")
        self.assertEqual(e["context_after"]["before"][-1], "second")
        self.assertEqual(e["context_after"]["after"], ["end"])

    def test_t8_moved_across_headings_carries_the_r3_flag(self):
        v1 = lines_text("# Doc", "", "## One", "", "anchored a", "anchored b", "tail")
        v2 = lines_text("# Doc", "", "## One", "", "one stays", "## Two", "two intro", "anchored a", "anchored b", "tail")
        e = self.one(v1, v2, "5-6")
        self.assertEqual(e["class"], "moved")  # classification does not change
        self.assertTrue(e["owner_review_required"])
        self.assertEqual(e["owner_review_reasons"], ["enclosing_heading_changed"])
        self.assertEqual(e["context_before"]["heading"]["text"], "## One")
        self.assertEqual(e["context_after"]["heading"]["text"], "## Two")

    def test_t8_non_markdown_moved_raises_no_heading_flag(self):
        v1 = lines_text("# comment", "x = 1", "y = 2", "z = 3")
        v2 = lines_text("# comment", "new = 0", "# another comment", "x = 1", "y = 2", "z = 3")
        e = self.one(v1, v2, "2-3", doc="src/m.py")
        self.assertEqual(e["class"], "moved")
        self.assertFalse(e["owner_review_required"])
        self.assertIn("not applicable", e["context_after"]["heading"])

    def test_t9_whole_document_anchor(self):
        rev = self.commit_files({"docs/a.md": DOC}, "v1")
        self.commit_files({"docs/other.md": "x\n"}, "v2")
        at_same = self.commit_register([self.decision("d", "docs/a.md", rev)], msg="register again")
        e = self.by_id(self.report(at_same))["d"]
        self.assertEqual(e["class"], "unchanged")
        self.assertEqual(e["before"]["document_blob"], e["after"]["document_blob"])
        self.assertNotIn("lines", e["before"])
        self.commit_files({"docs/a.md": DOC + "more\n"}, "v3")
        rep = self.report()
        e = self.by_id(rep)["d"]
        self.assertEqual(e["class"], "modified")
        self.assertEqual(e["candidates"][0]["document"], "docs/a.md")
        self.commit_files({"docs/a.md": None}, "v4")
        self.assertEqual(self.by_id(self.report())["d"]["class"], "absent")

    def test_target_is_the_pinned_commit_not_the_moving_head(self):
        rev = self.commit_files({"docs/a.md": DOC}, "v1")
        at = self.commit_register([self.decision("d", "docs/a.md", rev, "4-5")])
        self.commit_files({"docs/a.md": DOC.replace("beta", "BETA")}, "later")
        self.assertEqual(self.by_id(self.report(at))["d"]["class"], "unchanged")
        rep = self.report()  # HEAD now sees the later edit
        self.assertEqual(self.by_id(rep)["d"]["class"], "modified")
        self.assertEqual(rep["pinned_commit"], git(self.repo, "rev-parse", "HEAD"))


class ErrorAndOutputTests(AnchorFixture):
    def test_t10_unresolvable_revision_is_an_error_and_others_are_classified(self):
        rev = self.commit_files({"docs/a.md": DOC}, "v1")
        decisions = [self.decision("good", "docs/a.md", rev, "4-5"),
                     self.decision("badrev", "docs/a.md", "deadbeefdeadbeef", "4-5"),
                     self.decision("nodoc", "docs/missing.md", rev, "1"),
                     self.decision("range", "docs/a.md", rev, "3-99"),
                     self.decision("badlines", "docs/a.md", rev, "x")]
        decisions[1]["revision"] = "git:deadbeefdeadbeef"
        at = self.commit_register(decisions)
        p, out = self.anchors(at)
        self.assertEqual(p.returncode, 3, p.stdout + p.stderr)
        self.assertNotIn("Traceback", p.stderr)
        self.assertTrue(os.path.isfile(os.path.join(out, "anchors.json")))
        self.assertTrue(os.path.isfile(os.path.join(out, "anchors.md")))
        rep = json.loads(slurp(os.path.join(out, "anchors.json")))
        e = self.by_id(rep)
        self.assertEqual(e["good"]["class"], "unchanged")
        for did, kind in (("badrev", "revision_unresolved"), ("nodoc", "document_missing_at_revision"),
                          ("range", "lines_out_of_range"), ("badlines", "lines_invalid")):
            self.assertIsNone(e[did]["class"], did)
            self.assertEqual(e[did]["error"]["kind"], kind, did)
            self.assertTrue(e[did]["error"]["reason"])
        self.assertEqual(rep["summary"]["error"], 4)
        self.assertIn("ERROR revision_unresolved", slurp(os.path.join(out, "anchors.md")).decode())

    def test_malformed_decision_records_do_not_crash(self):
        rev = self.commit_files({"docs/a.md": DOC}, "v1")
        at = self.commit_register([self.decision("good", "docs/a.md", rev, "4-5"), "not an object",
                                   {"decision_id": 7, "document": None}])
        p, out = self.anchors(at)
        self.assertEqual(p.returncode, 3, p.stdout + p.stderr)
        self.assertNotIn("Traceback", p.stderr)
        rep = json.loads(slurp(os.path.join(out, "anchors.json")))
        self.assertEqual(rep["summary"]["error"], 2)
        self.assertEqual(rep["summary"]["unchanged"], 1)

    def test_cannot_run_cases_use_exit_4_and_write_nothing(self):
        rev = self.commit_files({"docs/a.md": DOC}, "v1")
        self.commit_register([self.decision("d", "docs/a.md", rev, "4-5")])
        p, out = self.anchors("no-such-ref")
        self.assertEqual(p.returncode, 4, p.stdout)
        self.assertFalse(os.path.exists(out))
        p, out = self.anchors("-bad")
        self.assertEqual(p.returncode, 4, p.stdout)
        # register not present at the pinned commit
        p, out = self.anchors(rev)
        self.assertEqual(p.returncode, 4, p.stdout)
        self.assertFalse(os.path.exists(out))

    def test_t11_two_runs_are_byte_identical(self):
        rev = self.commit_files({"docs/a.md": DOC}, "v1")
        self.commit_files({"docs/a.md": DOC.replace("gamma", "G").replace("# Title\n\n", "# Title\n\nx\n")}, "v2")
        self.commit_register([self.decision("a", "docs/a.md", rev, "3"), self.decision("b", "docs/a.md", rev, "4-6"),
                              self.decision("c", "docs/a.md", rev, "8")],
                             [self.appl("ap1", "b", rev, "authorized"), self.appl("ap2", "a", rev)])
        outs = []
        for _ in range(2):
            p, out = self.anchors()
            self.assertEqual(p.returncode, 0, p.stdout + p.stderr)
            outs.append(out)
        for name in ("anchors.json", "anchors.md"):
            self.assertEqual(slurp(os.path.join(outs[0], name)), slurp(os.path.join(outs[1], name)))
        text = slurp(os.path.join(outs[0], "anchors.json")).decode()
        self.assertEqual(sorted(os.listdir(outs[0])), ["anchors.json", "anchors.md"])
        rep = json.loads(text)
        self.assertEqual([e["decision_id"] for e in rep["decisions"]], ["a", "b", "c"])
        self.assertEqual(text, json.dumps(rep, sort_keys=True, indent=1, ensure_ascii=False) + "\n")
        self.assertIn(rep["pinned_commit"], slurp(os.path.join(outs[0], "anchors.md")).decode())
        self.assertEqual(sorted(rep["register"]["inputs"]), ["applicability.jsonl", "decisions.json"])
        for info in rep["register"]["inputs"].values():
            self.assertEqual(info["sha256"], hashlib.sha256(slurp(os.path.join(self.repo, info["path"]))).hexdigest())

    def test_t12_no_write_and_out_inside_register_refused(self):
        rev = self.commit_files({"docs/a.md": DOC}, "v1")
        self.commit_files({"docs/a.md": DOC.replace("gamma", "G")}, "v2")
        self.commit_register([self.decision("a", "docs/a.md", rev, "3"), self.decision("b", "docs/a.md", rev, "4-6")])
        # make the index stat data stale, which is what a refresh would rewrite
        for base, _, files in os.walk(self.repo):
            if ".git" in base.split(os.sep):
                continue
            for f in files:
                os.utime(os.path.join(base, f), (1_000_000_000, 1_000_000_000))
        register = os.path.join(self.repo, "metrics", "register")
        before_register = digest_tree(register)
        before_index = slurp(os.path.join(self.repo, ".git", "index"))
        before_git = digest_tree(os.path.join(self.repo, ".git"))
        before_work = digest_tree(self.repo)
        p, out = self.anchors()
        self.assertEqual(p.returncode, 0, p.stdout + p.stderr)
        self.assertEqual(digest_tree(register), before_register)
        self.assertEqual(slurp(os.path.join(self.repo, ".git", "index")), before_index)
        self.assertEqual(digest_tree(os.path.join(self.repo, ".git")), before_git)
        self.assertEqual(digest_tree(self.repo), before_work)
        # an --out inside the register (or its subtree, or the register itself) is refused
        for target in (os.path.join(register, "view"), register, os.path.join(register, "studies", "n"),
                       os.path.join(self.repo, ".git", "anchors")):
            with self.subTest(out=target):
                p, _ = self.anchors(out=target)
                self.assertEqual(p.returncode, 2, p.stdout)
                self.assertIn("refused", p.stdout)
                self.assertEqual(digest_tree(register), before_register)
                self.assertEqual(digest_tree(os.path.join(self.repo, ".git")), before_git)
        self.assertIn("inside the register directory", self.anchors(out=register)[0].stdout)

    def test_git_user_configuration_does_not_change_the_result(self):
        v2 = DOC.replace("# Title\n\n", "# Title\n\nnew one\nnew two\n")
        rev = self.commit_files({"docs/a.md": DOC}, "v1")
        self.commit_files({"docs/a.md": v2}, "v2")
        self.commit_register([self.decision("d", "docs/a.md", rev, "4-5")])
        plain = self.report()
        for key, value in (("diff.algorithm", "patience"), ("diff.renames", "copies"), ("diff.indentHeuristic", "false"),
                           ("diff.context", "9"), ("diff.interHunkContext", "9"), ("core.quotepath", "false")):
            git(self.repo, "config", key, value)
        self.assertEqual(self.report()["decisions"], plain["decisions"])


class ApplicabilityTests(AnchorFixture):
    def test_t13_applicability_accounting(self):
        v1 = lines_text("# Doc", "", "## One", "", "m1", "m2", "", "r1", "r2", "", "x1", "x2", "", "u1", "u2",
                        "", "## Two", "", "tail")
        v2 = lines_text("# Doc", "", "add1", "add2", "## One", "", "m1", "m2", "", "", "x1 CHANGED", "x2", "", "u1", "u2",
                        "", "## Two", "", "tail", "r1", "r2")
        rev = self.commit_files({"docs/a.md": v1}, "v1")
        self.commit_files({"docs/a.md": v2}, "v2")
        decisions = [self.decision("moved", "docs/a.md", rev, "5-6"), self.decision("reloc", "docs/a.md", rev, "8-9"),
                     self.decision("modif", "docs/a.md", rev, "11-12"), self.decision("same", "docs/a.md", rev, "14-15")]
        appl = [self.appl("ap-moved-auth", "moved", rev, "authorized"), self.appl("ap-moved-pend", "moved", rev),
                self.appl("ap-reloc-auth", "reloc", rev, "authorized"), self.appl("ap-reloc-pend", "reloc", rev),
                self.appl("ap-modif-auth", "modif", rev, "authorized"), self.appl("ap-modif-dec", "modif", rev, "declined"),
                self.appl("ap-same", "same", rev, "authorized"), self.appl("ap-other", "elsewhere", rev)]
        self.commit_register(decisions, appl)
        rep = self.report()
        e = self.by_id(rep)
        self.assertEqual({k: v["class"] for k, v in e.items()},
                         {"moved": "moved", "reloc": "relocated_verbatim", "modif": "modified", "same": "unchanged"})
        rows = {k: {r["record"]["applicability_id"]: r for r in v.get("applicability", [])} for k, v in e.items()}
        originals = {r["applicability_id"]: r for r in appl}
        for k, recs in rows.items():  # records are copied verbatim from the pinned register
            for aid, row in recs.items():
                self.assertEqual(row["record"], originals[aid])
        # unchanged decisions list no reconciliation block
        self.assertNotIn("applicability", e["same"])
        self.assertEqual(e["same"]["applicability_records_citing"], 1)
        self.assertEqual(rows["moved"]["ap-moved-auth"]["after_state"], "carried_unchanged")
        self.assertEqual(rows["moved"]["ap-moved-auth"]["authorization_after"], "carried_unchanged")
        self.assertEqual(rows["moved"]["ap-moved-pend"]["after_state"], "carried_unchanged")
        self.assertIsNone(rows["moved"]["ap-moved-pend"]["authorization_after"])
        self.assertEqual(rows["reloc"]["ap-reloc-auth"]["after_state"], "owner_review_required")
        self.assertEqual(rows["reloc"]["ap-reloc-auth"]["authorization_after"], "authorized_at_prior_revision")
        self.assertEqual(rows["reloc"]["ap-reloc-pend"]["after_state"], "owner_review_required")
        self.assertEqual(rows["modif"]["ap-modif-auth"]["after_state"], "pending_judgment_required")
        self.assertEqual(rows["modif"]["ap-modif-auth"]["authorization_after"], "authorized_at_prior_revision")
        self.assertEqual(rows["modif"]["ap-modif-dec"]["after_state"], "pending_judgment_required")
        self.assertIsNone(rows["modif"]["ap-modif-dec"]["authorization_after"])
        # an authorized record outside carried_unchanged is never shown as authorized after
        for k in ("reloc", "modif"):
            for row in rows[k].values():
                self.assertNotIn(row["authorization_after"], ("authorized", "carried_unchanged"))
                self.assertIn(row["after_state"], ("owner_review_required", "pending_judgment_required"))
        md = slurp(os.path.join(self.base, "out1", "anchors.md")).decode()
        self.assertIn("authorized_at_prior_revision", md)
        self.assertIn("Nothing was re-anchored, no register record was changed and no authorization was transferred.", md)
        self.assertNotIn("ap-other", md)  # cites no decision in the report

    def test_skill_and_historical_pin_notes(self):
        rev = self.commit_files({"skills/s/references/r.md": DOC, "docs/a.md": DOC}, "v1")
        self.commit_files({"docs/a.md": DOC + "x\n"}, "v2")
        self.commit_register([self.decision("skill", "skills/s/references/r.md", rev, "4-5"),
                              self.decision("model-selection.claude-host-builder-rows-at-df288c3d", "docs/a.md", rev, "4-5")])
        e = self.by_id(self.report())
        self.assertTrue(any("installed skill release was not examined" in n for n in e["skill"]["notes"]))
        pin = e["model-selection.claude-host-builder-rows-at-df288c3d"]
        self.assertTrue(any("historical pin" in n for n in pin["notes"]))
        self.assertEqual(pin["class"], "unchanged")  # the note does not special-case the class


# The root's frozen expectation for the real register at BASE_COMMIT (D/t14-expected-14031bc2.json,
# sha256 bc8b3ef67cf97e258b632c8b39cd03c85aa6ec33f4387fa54bd643859c5f3b06), transcribed as a literal.
T14_GROUPS = {"unchanged": {"unchanged"}, "verbatim_elsewhere": {"moved", "relocated_verbatim"},
              "not_verbatim": {"modified", "ambiguous", "absent"}}
T14_EXPECTED = {
    "model-selection.builder-default": "verbatim_elsewhere",
    "model-selection.terra-mechanical-builder": "verbatim_elsewhere",
    "model-selection.claude-host-builder-rows": "not_verbatim",
    "model-selection.claude-host-builder-rows-at-df288c3d": "not_verbatim",
    "model-selection.cost-table-quota-row": "not_verbatim",
    "model-selection.cost-table-effort-row": "verbatim_elsewhere",
    "context-boundary.h2-levels": "unchanged",
    "roadmap.current-strategic-route": "unchanged",
    "pi-docker-convergence.context-replacement": "unchanged",
    "pi-coding-profile.acceptance-contract": "unchanged",
    "dogfood-plan.round2-status-line": "verbatim_elsewhere",
}


class RealRegisterTests(unittest.TestCase):
    def setUp(self):
        self.base = tempfile.mkdtemp(prefix="anchors-real-")
        self.addCleanup(shutil.rmtree, self.base, ignore_errors=True)

    def test_t14_real_register_classes_fall_in_the_frozen_groups(self):
        out = os.path.join(self.base, "out")
        p = subprocess.run([sys.executable, "-I", SCRIPT, "anchors", "--repo", REAL_REPO, "--at", BASE_COMMIT, "--out", out],
                           capture_output=True, text=True, check=False)
        self.assertEqual(p.returncode, 0, p.stdout + p.stderr)
        rep = json.loads(slurp(os.path.join(out, "anchors.json")))
        self.assertEqual(rep["pinned_commit"], BASE_COMMIT)
        got = {e["decision_id"]: e for e in rep["decisions"]}
        self.assertEqual(sorted(got), sorted(T14_EXPECTED))
        self.assertEqual(len(got), 11)
        for did, group in T14_EXPECTED.items():
            self.assertIn(got[did]["class"], T14_GROUPS[group], f"{did}: {got[did]['class']} not in {group}")
            self.assertIsNone(got[did]["error"], did)
        # census lines recorded in the frozen file for the moved decisions
        for did, after in (("model-selection.builder-default", "41"), ("model-selection.terra-mechanical-builder", "42"),
                           ("model-selection.cost-table-effort-row", "278"), ("dogfood-plan.round2-status-line", "13")):
            self.assertEqual(got[did]["after"]["lines"], after, did)
        self.assertTrue(any("installed skill release was not examined" in n for n in got["context-boundary.h2-levels"]["notes"]))
        self.assertTrue(any("historical pin" in n for n in got["model-selection.claude-host-builder-rows-at-df288c3d"]["notes"]))
        self.assertEqual(got["pi-docker-convergence.context-replacement"]["before"]["document_blob"],
                         "5a24de8073add1e047019bb8d6aa39587f15ab53")
        self.assertEqual(got["pi-docker-convergence.context-replacement"]["after"]["document_blob"],
                         "5a24de8073add1e047019bb8d6aa39587f15ab53")

    def cli(self, script, *args):
        return subprocess.run([sys.executable, "-I", script, *args], capture_output=True, text=True, check=False)

    def test_t15_existing_commands_are_unchanged_against_the_base(self):
        # test_register.py is untouched, and passes
        base_test = git(REAL_REPO, "show", f"{BASE_COMMIT}:metrics/scripts/tests/test_register.py", raw=True)
        self.assertEqual(slurp(os.path.join(HERE, "test_register.py")), base_test)
        p = subprocess.run([sys.executable, "-I", os.path.join(HERE, "test_register.py")], capture_output=True, text=True,
                           check=False, cwd=self.base)
        self.assertEqual(p.returncode, 0, p.stderr[-2000:])
        # validate / build / nominate agree with the base script on the real register
        old = os.path.join(self.base, "register_base.py")
        with open(old, "wb") as fh:
            fh.write(git(REAL_REPO, "show", f"{BASE_COMMIT}:metrics/scripts/register.py", raw=True))
        v_old, v_new = (self.cli(s, "validate", "--repo", REAL_REPO) for s in (old, SCRIPT))
        self.assertEqual((v_old.returncode, v_old.stdout), (v_new.returncode, v_new.stdout))
        self.assertEqual(v_new.returncode, 0, v_new.stdout)
        self.assertIn("validate: ok", v_new.stdout)
        for cmd, names in (("build", ("current-knowledge.json", "current-knowledge.md")), ("nominate", None)):
            outs = []
            for tag, s in (("old", old), ("new", SCRIPT)):
                out = os.path.join(self.base, f"{cmd}-{tag}")
                if names is None:
                    out = out + ".json"
                p = self.cli(s, cmd, "--repo", REAL_REPO, "--out", out)
                self.assertEqual(p.returncode, 0, p.stdout + p.stderr)
                outs.append((p.stdout.replace(out, "OUT"), out))
            self.assertEqual(outs[0][0], outs[1][0])
            if names is None:
                self.assertEqual(slurp(outs[0][1]), slurp(outs[1][1]))
            else:
                for n in names:
                    self.assertEqual(slurp(os.path.join(outs[0][1], n)), slurp(os.path.join(outs[1][1], n)))


if __name__ == "__main__":
    unittest.main()
