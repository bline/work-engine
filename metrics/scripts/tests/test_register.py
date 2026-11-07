"""Tests for metrics/scripts/register.py (standard library unittest).

Fixtures live in a temporary git repository; the real register is validated at the end.
"""
import copy
import hashlib
import importlib.util
import json
import os
import shutil
import stat
import subprocess
import sys
import tempfile
import unittest

HERE = os.path.dirname(os.path.abspath(__file__))
SCRIPT = os.path.abspath(os.path.join(HERE, "..", "register.py"))
REAL_REPO = os.path.abspath(os.path.join(HERE, "..", "..", ".."))

spec = importlib.util.spec_from_file_location("register_under_test", SCRIPT)
reg = importlib.util.module_from_spec(spec)
spec.loader.exec_module(reg)


def git(repo, *args):
    p = subprocess.run(["git", "-C", repo, *args], capture_output=True, text=True, check=True,
                       env={**os.environ, "GIT_AUTHOR_NAME": "t", "GIT_AUTHOR_EMAIL": "t@example.invalid",
                            "GIT_COMMITTER_NAME": "t", "GIT_COMMITTER_EMAIL": "t@example.invalid"})
    return p.stdout.strip()


def slurp(path, mode="rb"):
    with open(path, mode) as fh:
        return fh.read()


def snapshot(root):
    """Map of relative path -> sha256 for every file under root (directories included as None)."""
    out = {}
    for base, dirs, files in os.walk(root):
        dirs.sort()
        for d in dirs:
            out[os.path.relpath(os.path.join(base, d), root) + "/"] = None
        for f in sorted(files):
            p = os.path.join(base, f)
            with open(p, "rb") as fh:
                out[os.path.relpath(p, root)] = hashlib.sha256(fh.read()).hexdigest()
    return out


def write_register(root, data):
    os.makedirs(os.path.join(root, "studies"), exist_ok=True)
    with open(os.path.join(root, "questions.json"), "w") as fh:
        json.dump(data["questions"], fh)
    for s in data["studies"]:
        with open(os.path.join(root, "studies", s["id"] + ".json"), "w") as fh:
            json.dump(s, fh)
    for name in ("events", "judgments", "applicability", "materialized"):
        with open(os.path.join(root, name + ".jsonl"), "w") as fh:
            for r in data[name]:
                fh.write(json.dumps(r) + "\n")
    with open(os.path.join(root, "decisions.json"), "w") as fh:
        json.dump(data["decisions"], fh)


class Fixture(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.base = tempfile.mkdtemp(prefix="register-test-")
        cls.repo = os.path.join(cls.base, "repo")
        os.makedirs(os.path.join(cls.repo, "docs"))
        git(cls.repo, "init", "-q")
        with open(os.path.join(cls.repo, "docs", "model.md"), "w") as fh:
            fh.write("row one\nrow two\n")
        with open(os.path.join(cls.repo, "docs", "study.md"), "w") as fh:
            fh.write("result a\nresult b\n")
        git(cls.repo, "add", "-A")
        git(cls.repo, "commit", "-q", "-m", "first")
        cls.c1 = git(cls.repo, "rev-parse", "--short=8", "HEAD")
        with open(os.path.join(cls.repo, "docs", "model.md"), "a") as fh:
            fh.write("row three\n")
        git(cls.repo, "commit", "-qam", "second")
        cls.c2 = git(cls.repo, "rev-parse", "--short=8", "HEAD")
        cls.model_c1 = git(cls.repo, "rev-parse", f"{cls.c1}:docs/model.md")
        cls.model_c2 = git(cls.repo, "rev-parse", f"{cls.c2}:docs/model.md")
        cls.study_blob = git(cls.repo, "rev-parse", f"{cls.c2}:docs/study.md")
        cls.offrepo_file = os.path.join(cls.base, "offrepo.txt")
        with open(cls.offrepo_file, "w") as fh:
            fh.write("primary evidence\n")
        cls.offrepo_sha = hashlib.sha256(b"primary evidence\n").hexdigest()

    @classmethod
    def tearDownClass(cls):
        shutil.rmtree(cls.base, ignore_errors=True)

    def pin(self, lines=None, blob=None):
        d = {"path": "docs/study.md", "commit": self.c2, "blob": blob or self.study_blob}
        if lines:
            d["lines"] = lines
        return d

    def offrepo(self, path=None, sha=None, status="verified"):
        return {"location": path or self.offrepo_file, "sha256": sha if sha is not None else self.offrepo_sha,
                "custody": "off_repo", "status": status}

    def data(self):
        def finding(study, slug, **kw):
            f = {"id": f"{study}#{slug}", "claim": f"claim {slug}", "value": 1, "n": 3, "basis": "measured",
                 "scope": "fixture scope", "caveats": ["c"], "sources": [self.pin("1-2")]}
            f.update(kw)
            return f

        def study(sid, alias, findings, qs):
            return {"id": sid, "title": f"Study {sid}", "kind": "pilot", "questions": qs,
                    "lifecycle": [{"date": "2026-01-01", "state": "concluded", "basis": self.pin()}],
                    "stop_reason_class": None, "findings": findings, "sources": [self.pin(), self.offrepo()],
                    "aliases": [alias]}

        owner = "user (fixture owner)"
        def decision(did, doc, rev, findings):
            return {"decision_id": did, "document": doc, "anchor": f"anchor {did}", "revision": f"git:{rev}",
                    "owner": owner, "findings": findings}

        def applicability(aid, finding, did, rev, relation="supports"):
            return {"applicability_id": aid, "finding": finding, "decision_id": did, "decision_revision": f"git:{rev}",
                    "relation": relation, "rationale": "because", "evidence": [self.pin()], "proposed_by": "tester",
                    "proposed_at": "2026-01-02", "authorization": {"status": "pending", "owner": owner}}

        return {
            "questions": [{"id": "Q1", "text": "First?", "area": "a"}, {"id": "Q2", "text": "Second?", "area": "b"}],
            "studies": [study("alpha-study", "K1", [finding("alpha-study", "original"), finding("alpha-study", "corrected")],
                              ["Q1"]),
                        study("beta-study", "K2", [finding("beta-study", "claim"), finding("beta-study", "obs")], ["Q1", "Q2"])],
            "events": [
                {"event_id": "ev-correction", "kind": "correction", "date": "2026-01-03",
                 "subject_finding": "alpha-study#original", "by_finding": "alpha-study#corrected", "basis": "b",
                 "source": self.pin("1-2")},
                {"event_id": "ev-challenge", "kind": "challenge", "date": "2026-01-04",
                 "subject_finding": "beta-study#claim", "by_finding": "beta-study#obs", "basis": "b", "source": self.offrepo()},
                {"event_id": "ev-status", "kind": "status_discrepancy", "date": "2026-01-05",
                 "subject_finding": "alpha-study#corrected", "by_finding": None, "basis": "b", "source": self.pin("1-2")},
            ],
            "judgments": [
                {"finding": "alpha-study#original", "axis": "standing", "value": "standing", "basis": "b",
                 "assessor": "tester", "date": "2026-01-01"},
                {"finding": "alpha-study#original", "axis": "standing", "value": "qualified", "basis": "later",
                 "assessor": "tester", "date": "2026-01-06"},
                {"finding": "alpha-study#corrected", "axis": "currency", "value": "current", "basis": "b",
                 "assessor": "tester", "date": "2026-01-01"},
            ],
            "decisions": [
                decision("d-model-old", "docs/model.md", self.c1, ["alpha-study#original"]),
                decision("d-model-new", "docs/model.md", self.c2, ["alpha-study#corrected"]),
                decision("d-claim", "docs/model.md", self.c2, ["beta-study#claim"]),
                decision("d-obs", "docs/study.md", self.c2, ["beta-study#obs"]),
            ],
            "applicability": [
                applicability("ap-one", "alpha-study#corrected", "d-model-new", self.c2),
                applicability("ap-two", "beta-study#obs", "d-obs", self.c2, "constrains"),
            ],
            "materialized": [],
        }

    def make(self, mutate=None):
        data = self.data()
        if mutate:
            mutate(data)
        root = tempfile.mkdtemp(prefix="reg-", dir=self.base)
        write_register(root, data)
        return root

    def check(self, mutate=None, offrepo=False):
        root = self.make(mutate)
        r, errors, _ = reg.load_valid(root, self.repo)
        names = errors.names()
        if offrepo:
            mism, report = reg.check_offrepo(r)
            names += [m[0] for m in mism]
            return names, report
        return names, errors

    def cli(self, *args, check=False):
        return subprocess.run([sys.executable, "-I", SCRIPT, *args], capture_output=True, text=True, check=check)


class ValidateTests(Fixture):
    def test_valid_fixture_passes(self):
        names, errors = self.check()
        self.assertEqual(names, [], errors.items)

    def test_cli_exit_codes(self):
        good = self.make()
        self.assertEqual(self.cli("validate", "--register", good, "--repo", self.repo).returncode, 0)
        bad = self.make(lambda d: d["studies"][0]["findings"][0]["sources"][0].update(blob="0" * 40))
        p = self.cli("validate", "--register", bad, "--repo", self.repo)
        self.assertEqual(p.returncode, 1)
        self.assertIn("ERROR pin-blob-mismatch", p.stdout)

    def test_wrong_blob_fails(self):
        names, errors = self.check(lambda d: d["studies"][0]["findings"][0]["sources"][0].update(blob="0" * 40))
        self.assertEqual(names, ["pin-blob-mismatch"])
        self.assertIn("alpha-study.json", errors.items[0][1])

    def test_unresolvable_pin_fails(self):
        names, _ = self.check(lambda d: d["studies"][0]["findings"][0]["sources"][0].update(path="docs/missing.md"))
        self.assertEqual(names, ["pin-unresolved"])

    def test_unknown_key_fails(self):
        names, _ = self.check(lambda d: d["studies"][0].update(surprise=1))
        self.assertEqual(names, ["unknown-key"])

    def test_applicability_without_decision_revision_fails(self):
        names, _ = self.check(lambda d: d["applicability"][0].pop("decision_revision"))
        self.assertEqual(names, ["missing-decision-revision"])

    def test_null_or_non_string_decision_revision_fails_in_applicability(self):
        names, errors = self.check(lambda d: d["applicability"][0].update(decision_revision=None))
        self.assertEqual(names, ["missing-decision-revision"])
        self.assertIn("applicability.jsonl:1", errors.items[0][1])
        for bad in (7, ["git:" + self.c2], {"r": 1}, True):
            with self.subTest(bad=bad):
                names, errors = self.check(lambda d: d["applicability"][0].update(decision_revision=bad))
                self.assertEqual(names, ["bad-value"])
                self.assertIn("must be a string of the form git:<commit>", errors.items[0][2])
        names, _ = self.check(lambda d: d["applicability"][0].update(decision_revision=self.c2))
        self.assertEqual(names, ["bad-value"])  # no git: prefix

    def test_null_or_non_string_revision_fails_in_decisions(self):
        for bad in (None, 7, ["git:" + self.c2], {"r": 1}):
            with self.subTest(bad=bad):
                names, errors = self.check(lambda d: d["decisions"][1].update(revision=bad))
                self.assertIn("bad-value", names)
                self.assertTrue(any(loc == "decisions.json[1]" for _, loc, _ in errors.items))

    def test_wrongly_typed_ids_references_and_enums_give_named_errors(self):
        weird = (["x"], {"k": "v"}, 7, None)
        cases = [
            ("question id", lambda d, v: d["questions"][0].update(id=v), {"bad-value", "unknown-question-reference"}),
            ("study kind", lambda d, v: d["studies"][0].update(kind=v), {"bad-value"}),
            ("study question ref", lambda d, v: d["studies"][0]["questions"].append(v), {"unknown-question-reference"}),
            ("finding basis", lambda d, v: d["studies"][0]["findings"][0].update(basis=v), {"bad-value"}),
            ("decision id", lambda d, v: d["decisions"][0].update(decision_id=v), {"bad-value"}),
            ("decision finding ref", lambda d, v: d["decisions"][0]["findings"].append(v),
             {"unknown-finding-reference"}),
            ("event id", lambda d, v: d["events"][0].update(event_id=v), {"bad-value"}),
            ("event kind", lambda d, v: d["events"][0].update(kind=v), {"bad-value"}),
            ("event subject", lambda d, v: d["events"][0].update(subject_finding=v), {"unknown-finding-reference"}),
            ("event by_finding", lambda d, v: d["events"][0].update(by_finding=v), {"unknown-finding-reference"}),
            ("judgment finding", lambda d, v: d["judgments"][0].update(finding=v), {"unknown-finding-reference"}),
            ("judgment axis", lambda d, v: d["judgments"][0].update(axis=v), {"bad-value"}),
            ("judgment value", lambda d, v: d["judgments"][0].update(value=v), {"bad-value"}),
            ("applicability id", lambda d, v: d["applicability"][0].update(applicability_id=v), {"bad-value"}),
            ("applicability finding", lambda d, v: d["applicability"][0].update(finding=v),
             {"unknown-finding-reference"}),
            ("applicability decision", lambda d, v: d["applicability"][0].update(decision_id=v),
             {"unknown-decision-reference"}),
            ("applicability relation", lambda d, v: d["applicability"][0].update(relation=v), {"bad-value"}),
            ("authorization status", lambda d, v: d["applicability"][0]["authorization"].update(status=v),
             {"bad-value"}),
            ("offrepo status", lambda d, v: d["studies"][0]["sources"].append(
                {"location": "x", "sha256": None, "custody": "off_repo", "status": v}), {"offrepo-malformed"}),
            ("materialized finding", lambda d, v: d["materialized"].append(
                {"finding": v, "claim_id": "c", "revision_id": "r", "revision_sha256": "s"}),
             {"unknown-finding-reference"}),
        ]
        for site, mutate, expected in cases:
            for v in weird:
                with self.subTest(site=site, value=v):
                    names, errors = self.check(lambda d: mutate(d, v))
                    if site == "event by_finding" and v is None:
                        self.assertEqual(names, [])  # by_finding may be null
                        continue
                    self.assertTrue(names, "no error reported")
                    self.assertTrue(set(names) <= expected, (site, v, errors.items))
                    for _, loc, _ in errors.items:
                        self.assertTrue(loc)

    def test_wrongly_typed_value_is_not_a_traceback_on_the_command_line(self):
        root = self.make(lambda d: d["events"][0].update(subject_finding=["alpha-study#original"]))
        p = self.cli("validate", "--register", root, "--repo", self.repo)
        self.assertEqual(p.returncode, 1)
        self.assertNotIn("Traceback", p.stderr)
        self.assertIn("ERROR unknown-finding-reference events.jsonl:1", p.stdout)

    def test_pin_lines_are_checked_for_format_and_range(self):
        names, errors = self.check(lambda d: d["studies"][0]["findings"][0]["sources"][0].update(lines="1-3"))
        self.assertEqual(names, ["pin-lines-out-of-range"])  # study.md has 2 lines
        self.assertIn("alpha-study.json", errors.items[0][1])
        names, _ = self.check(lambda d: d["studies"][0]["findings"][0]["sources"][0].update(lines="2-2"))
        self.assertEqual(names, [])
        for bad in ("0", "3-2", "x", "1-", 5, None, ["1"]):
            with self.subTest(bad=bad):
                names, _ = self.check(lambda d: d["studies"][0]["findings"][0]["sources"][0].update(lines=bad))
                self.assertEqual(names, ["bad-value"])
        names, _ = self.check(lambda d: d["decisions"][0].update(lines="1-9"))
        self.assertEqual(names, ["pin-lines-out-of-range"])  # model.md at the first commit has 2 lines
        names, _ = self.check(lambda d: d["decisions"][0].update(lines={"a": 1}))
        self.assertEqual(names, ["bad-value"])

    def test_pin_to_a_directory_fails(self):
        tree = git(self.repo, "rev-parse", f"{self.c2}:docs")
        names, _ = self.check(lambda d: d["studies"][0]["findings"][0]["sources"][0].update(path="docs", blob=tree))
        self.assertEqual(names, ["pin-not-blob"])

    def test_decision_revision_must_match_decisions_file(self):
        names, _ = self.check(lambda d: d["applicability"][0].update(decision_revision=f"git:{self.c1}"))
        self.assertEqual(names, ["decision-revision-mismatch"])

    def test_global_usefulness_or_obsolescence_field_fails(self):
        for key in ("useful", "obsolete", "global_usefulness"):
            with self.subTest(key=key):
                names, _ = self.check(lambda d: d["applicability"][0].update({key: True}))
                self.assertEqual(names, ["forbidden-global-field"])
        names, _ = self.check(lambda d: d["studies"][0]["findings"][0].update(obsolete=True))
        self.assertEqual(names, ["forbidden-global-field"])

    def test_missing_authorization_status_fails(self):
        names, _ = self.check(lambda d: d["applicability"][0].update(authorization={"owner": "x"}))
        self.assertEqual(names, ["missing-authorization-status"])
        names, _ = self.check(lambda d: d["applicability"][0].pop("authorization"))
        self.assertEqual(names, ["missing-authorization-status"])

    def test_decided_authorization_needs_a_time(self):
        names, _ = self.check(lambda d: d["applicability"][0]["authorization"].update(status="authorized"))
        self.assertEqual(names, ["bad-value"])

    def test_event_naming_unknown_finding_fails(self):
        names, _ = self.check(lambda d: d["events"][0].update(subject_finding="alpha-study#nope"))
        self.assertEqual(names, ["unknown-finding-reference"])
        names, _ = self.check(lambda d: d["events"][0].update(by_finding="alpha-study#nope"))
        self.assertEqual(names, ["unknown-finding-reference"])

    def test_other_references_must_resolve(self):
        names, _ = self.check(lambda d: d["studies"][0]["questions"].append("Q99"))
        self.assertEqual(names, ["unknown-question-reference"])
        names, _ = self.check(lambda d: d["applicability"][0].update(decision_id="d-nope"))
        self.assertEqual(names, ["unknown-decision-reference"])
        names, _ = self.check(lambda d: d["decisions"][0]["findings"].append("alpha-study#nope"))
        self.assertEqual(names, ["unknown-finding-reference"])

    def test_duplicate_ids_fail(self):
        names, _ = self.check(lambda d: d["events"].append(copy.deepcopy(d["events"][0])))
        self.assertEqual(names, ["duplicate-id"])

    def test_bad_enum_values_fail(self):
        names, _ = self.check(lambda d: d["judgments"][0].update(value="great"))
        self.assertEqual(names, ["bad-value"])
        names, _ = self.check(lambda d: d["applicability"][0].update(relation="useful_to"))
        self.assertEqual(names, ["bad-value"])

    def test_offrepo_shape_is_checked(self):
        names, _ = self.check(lambda d: d["studies"][0]["sources"].append(
            {"location": "x", "sha256": None, "custody": "off_repo", "status": "verified"}))
        self.assertEqual(names, ["offrepo-malformed"])

    def test_offrepo_digest_mismatch_fails_and_absence_is_only_reported(self):
        names, report = self.check(offrepo=True)
        self.assertEqual(names, [])
        self.assertTrue(any(line.startswith("MATCH") for line in report))
        names, _ = self.check(lambda d: d["studies"][0]["sources"].append(self.offrepo(sha="f" * 64)), offrepo=True)
        self.assertEqual(names, ["offrepo-digest-mismatch"])
        missing = os.path.join(self.base, "gone.txt")
        names, report = self.check(lambda d: d["studies"][0]["sources"].append(
            self.offrepo(path=missing, status="unavailable")), offrepo=True)
        self.assertEqual(names, [])
        self.assertTrue(any(line.startswith("ABSENT") and missing in line for line in report))


class OutputTests(Fixture):
    def run_twice(self, cmd, outname):
        root = self.make()
        work = tempfile.mkdtemp(prefix="out-", dir=self.base)
        before = snapshot(self.base)
        results = []
        for i in (1, 2):
            out = os.path.join(work, f"{outname}{i}")
            self.cli(cmd, "--register", root, "--repo", self.repo, "--out", out, check=True)
            results.append(out)
        after = snapshot(self.base)
        created = {k for k in after if k not in before}
        return results, created, work, before, after

    def test_build_is_deterministic_and_writes_only_out(self):
        (a, b), created, work, before, after = self.run_twice("build", "view")
        for name in ("current-knowledge.json", "current-knowledge.md"):
            self.assertEqual(slurp(os.path.join(a, name)), slurp(os.path.join(b, name)))
        rel = os.path.relpath(work, self.base)
        self.assertTrue(all(k.startswith(rel) for k in created), created)
        self.assertEqual({k: v for k, v in before.items()}, {k: after[k] for k in before})

    def test_build_view_content(self):
        (a, _), _, _, _, _ = self.run_twice("build", "view")
        doc = json.loads(slurp(os.path.join(a, "current-knowledge.json"), "r"))
        self.assertEqual(list(doc["k_index"]), ["K1", "K2"])
        orig = doc["findings"]["alpha-study#original"]
        self.assertEqual(orig["standing_from_events"], "corrected")
        self.assertEqual(orig["judgments"]["standing"]["value"], "qualified")  # latest judgment wins
        self.assertEqual(orig["judgments"]["currency"]["value"], "not established")
        self.assertEqual(doc["findings"]["beta-study#claim"]["standing_from_events"], "challenged")
        corrected = doc["findings"]["alpha-study#corrected"]
        self.assertTrue(corrected["status_discrepancy"])
        self.assertEqual(corrected["standing_from_events"], "none")
        self.assertEqual(corrected["applicability"][0]["authorization"], "pending")
        self.assertEqual(doc["questions"]["Q2"]["findings"], ["beta-study#claim", "beta-study#obs"])
        md = slurp(os.path.join(a, "current-knowledge.md"), "r")
        self.assertIn("### K1:", md)
        self.assertIn("standing from events: corrected", md)

    def test_markdown_view_keeps_findings_of_a_study_without_aliases(self):
        root = self.make(lambda d: d["studies"][0].update(aliases=[]))
        out = os.path.join(tempfile.mkdtemp(prefix="out-", dir=self.base), "view")
        self.cli("build", "--register", root, "--repo", self.repo, "--out", out, check=True)
        md = slurp(os.path.join(out, "current-knowledge.md"), "r")
        doc = json.loads(slurp(os.path.join(out, "current-knowledge.json"), "r"))
        for fid in doc["findings"]:
            self.assertIn(f"`{fid}`", md)
        self.assertIn("### no alias: `alpha-study#original`", md)

    def test_build_and_nominate_refuse_an_out_path_inside_the_register(self):
        root = self.make()
        before = snapshot(root)
        for cmd, out in (("build", os.path.join(root, "view")), ("build", root),
                         ("nominate", os.path.join(root, "events.jsonl")),
                         ("nominate", os.path.join(root, "studies", "n.json"))):
            with self.subTest(cmd=cmd, out=out):
                p = self.cli(cmd, "--register", root, "--repo", self.repo, "--out", out)
                self.assertEqual(p.returncode, 2)
                self.assertIn("inside the register directory", p.stdout)
                self.assertEqual(snapshot(root), before)

    def test_build_refuses_an_invalid_register(self):
        root = self.make(lambda d: d["events"][0].update(subject_finding="x#y"))
        out = os.path.join(self.base, "refused")
        p = self.cli("build", "--register", root, "--repo", self.repo, "--out", out)
        self.assertEqual(p.returncode, 1)
        self.assertFalse(os.path.exists(out))

    def test_nominate_is_deterministic_and_writes_only_out(self):
        root = self.make()
        work = tempfile.mkdtemp(prefix="nom-", dir=self.base)
        before = snapshot(self.base)
        outs = []
        for i in (1, 2):
            out = os.path.join(work, f"n{i}.json")
            self.cli("nominate", "--register", root, "--repo", self.repo, "--out", out, check=True)
            outs.append(out)
        after = snapshot(self.base)
        self.assertEqual(slurp(outs[0]), slurp(outs[1]))
        created = {k for k in after if k not in before}
        self.assertEqual(created, {os.path.relpath(o, self.base) for o in outs})
        self.assertEqual(before, {k: after[k] for k in before})

    def nominations(self, *extra):
        root = self.make()
        out = os.path.join(tempfile.mkdtemp(prefix="nom-", dir=self.base), "n.json")
        self.cli("nominate", "--register", root, "--repo", self.repo, "--out", out, *extra, check=True)
        return {n["event_id"]: n for n in json.loads(slurp(out, "r"))["nominations"]}

    def test_nominate_yields_the_expected_decision_set(self):
        noms = self.nominations()
        self.assertEqual(sorted(noms), ["ev-challenge", "ev-correction", "ev-status"])
        got = {k: {d["decision_id"]: d["via"] for d in v["decisions"]} for k, v in noms.items()}
        self.assertEqual(got["ev-correction"], {"d-model-old": "subject_finding", "d-model-new": "by_finding"})
        self.assertEqual(got["ev-challenge"], {"d-claim": "subject_finding", "d-obs": "by_finding"})
        self.assertEqual(got["ev-status"], {"d-model-new": "subject_finding"})
        self.assertEqual({a["applicability_id"] for a in noms["ev-challenge"]["applicability_records"]}, {"ap-two"})
        for n in noms.values():
            self.assertEqual(n["disposition"], "review_nominated")
            self.assertEqual(n["claim_revisions"]["status"], "not_queried")
            self.assertTrue(any("not established" not in x and "claim revisions" in x for x in n["not_established"]))

    def test_nominate_with_a_claims_host_is_optional_and_read_only(self):
        host = os.path.join(self.base, "fake-host")
        with open(host, "w") as fh:
            fh.write("#!/usr/bin/env python3\nimport json,sys\n"
                     "req=json.loads(sys.stdin.read())\n"
                     "print(json.dumps({'items':[{'current_head':False,'head_revision_ids':['h'],"
                     "'reliances':[{'record':{'consumer':'doc:x','consumer_revision':'git:1','state':'active'}}]}]}))\n")
        os.chmod(host, os.stat(host).st_mode | stat.S_IXUSR)
        data_mut = lambda d: d["materialized"].append(
            {"finding": "alpha-study#original", "claim_id": "c", "revision_id": "r", "revision_sha256": "s"})
        root = self.make(data_mut)
        out = os.path.join(tempfile.mkdtemp(prefix="nom-", dir=self.base), "n.json")
        self.cli("nominate", "--register", root, "--repo", self.repo, "--out", out, "--claims-host", host,
                 "--startup", "s.json", "--startup-sha256", "ab", check=True)
        nom = {n["event_id"]: n for n in json.loads(slurp(out, "r"))["nominations"]}["ev-correction"]
        self.assertEqual(nom["claim_revisions"]["status"], "queried")
        self.assertEqual(nom["claim_revisions"]["revisions"][0]["reliances"][0]["consumer"], "doc:x")
        partial = self.cli("nominate", "--register", root, "--repo", self.repo, "--out", out, "--claims-host", host)
        self.assertEqual(partial.returncode, 2)

    def test_nominate_refuses_an_invalid_register(self):
        root = self.make(lambda d: d["events"][0].update(by_finding="x#y"))
        out = os.path.join(self.base, "refused.json")
        self.assertEqual(self.cli("nominate", "--register", root, "--repo", self.repo, "--out", out).returncode, 1)
        self.assertFalse(os.path.exists(out))


class RealRegisterTests(unittest.TestCase):
    def test_real_register_validates(self):
        register = os.path.join(REAL_REPO, "metrics", "register")
        if not os.path.isdir(register):
            self.skipTest("metrics/register not present")
        have = subprocess.run(["git", "-C", REAL_REPO, "rev-parse", "--verify", "-q", "d80dd242^{commit}"],
                              capture_output=True).returncode == 0
        if not have:
            self.skipTest("pinned commit d80dd242 is not in this clone")
        p = subprocess.run([sys.executable, "-I", SCRIPT, "validate", "--register", register, "--repo", REAL_REPO],
                           capture_output=True, text=True)
        self.assertEqual(p.returncode, 0, p.stdout)

    def test_real_h2_decision_rests_on_k6_not_k5(self):
        register = os.path.join(REAL_REPO, "metrics", "register")
        if not os.path.isdir(register):
            self.skipTest("metrics/register not present")
        with open(os.path.join(register, "decisions.json"), encoding="utf-8") as fh:
            decisions = {d["decision_id"]: d for d in json.load(fh)}
        h2 = decisions["context-boundary.h2-levels"]["findings"]
        self.assertTrue(h2 and all(f.startswith("codex-compaction-points#") for f in h2), h2)


if __name__ == "__main__":
    unittest.main()
