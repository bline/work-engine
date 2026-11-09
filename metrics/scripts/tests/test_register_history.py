"""Tests for the A3 S1 register extension (standard library unittest): the optional decision keys
`historical`, `revisions` and `evidence_state`, citation of a listed prior revision, their views in
`anchors` and `nominate`, and the read-only `history` command. H1-H17 of the A3 S1 packet.

H2 (the two existing suites) is run as its own custody step; H9 and H10 live here.
"""
import copy
import hashlib
import io
import json
import os
import re
import shutil
import subprocess
import sys
import tarfile
import tempfile
import unittest

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import test_register as tr  # noqa: E402  (module access only, so its tests are not collected here)
import test_register_anchors as ta  # noqa: E402

SCRIPT = os.path.abspath(os.path.join(HERE, "..", "register.py"))
REAL_REPO = os.path.abspath(os.path.join(HERE, "..", "..", ".."))
S1_BASE = "46e5d008dbc3bfdd7d80c23c946af2d6b21e5efd"
S0_STUDY_COMMIT = "9fdf4b7e"
S0_PIN = "97f6486f"
A1_CENSUS_COMMIT = "14031bc23320b84b79149b62cf447a6594326438"
S0_STUDY = "metrics/register/studies/knowledge-oracle-s0-20261108.json"
FROZEN_CENSUS = "/home/bline/cbm-work/register-anchor-a1-20261108/t14-expected-14031bc2.json"
FROZEN_CENSUS_SHA = "bc8b3ef67cf97e258b632c8b39cd03c85aa6ec33f4387fa54bd643859c5f3b06"
A1_R2_ANCHORS_JSON_SHA = "b3f777140cba310e7c7cea53c42221daa25d26349ee7d776a2780658157ece6d"
A1_R2_ANCHORS_MD_SHA = "e6c73493e407dcbb97bb1a86e07c1bd1671e453bc4d6c121a8a6fdbfcc6024d5"
FORBIDDEN_PHRASES = ("owner-authorized", "owner authorized", "authorized by the owner", "owner approved", "approved by")
LIMITATION_NEEDLE = "cannot authenticate"


def sha(data):
    return hashlib.sha256(data).hexdigest()


def run_script(script, *args, env=None, cwd=None):
    return subprocess.run([sys.executable, "-I", "-B", script, *args], capture_output=True, text=True, check=False,
                          env={**os.environ, **(env or {})}, cwd=cwd)


def entry(rev, **kw):
    e = {"revision": f"git:{rev[:10]}", "anchor": "anchor at the prior revision", "superseded_at": "2026-01-03",
         "reason": "the anchored text changed"}
    e.update(kw)
    return e


# ---------------------------------------------------------------------------
# validate: schema fields (H3-H7)
# ---------------------------------------------------------------------------

class SchemaFixture(tr.Fixture):
    @classmethod
    def setUpClass(cls):
        super().setUpClass()
        with open(os.path.join(cls.repo, "docs", "model.md"), "w") as fh:
            fh.write("row zero\nrow one\nrow two\nrow three\n")
        tr.git(cls.repo, "commit", "-qam", "third")
        cls.c3 = tr.git(cls.repo, "rev-parse", "--short=8", "HEAD")
        tree = tr.git(cls.repo, "rev-parse", "HEAD^{tree}")
        cls.orphan = tr.git(cls.repo, "commit-tree", tree, "-m", "orphan")[:8]  # not an ancestor of c2

    @staticmethod
    def dec(data, did="d-model-new"):
        return next(d for d in data["decisions"] if d["decision_id"] == did)

    def with_dec(self, **keys):
        def mutate(data):
            self.dec(data).update(copy.deepcopy(keys))
        return mutate

    def errs(self, mutate):
        names, errors = self.check(mutate)
        return names, errors.items


class PriorRevisionCitationTests(SchemaFixture):
    def test_h3_listed_prior_revision_is_accepted_and_unlisted_is_not(self):
        def cite(rev, extra=None):
            def mutate(data):
                self.dec(data).update(extra or {})
                a = copy.deepcopy(data["applicability"][0])
                a["applicability_id"] = "ap-cites"
                a["decision_revision"] = f"git:{rev}"
                data["applicability"].append(a)
            return mutate
        names, items = self.errs(cite(self.c1, {"revisions": [entry(self.c1)]}))
        self.assertEqual(names, [], items)
        # the current revision is still accepted
        names, items = self.errs(cite(self.c2, {"revisions": [entry(self.c1)]}))
        self.assertEqual(names, [], items)
        # an unlisted revision keeps the base name and message
        names, items = self.errs(cite(self.c3, {"revisions": [entry(self.c1)]}))
        self.assertEqual(names, ["decision-revision-mismatch"])
        self.assertEqual(items[0][2], f"decision_revision 'git:{self.c3}' differs from decisions.json 'git:{self.c2}'")
        # a revision that is listed on another decision, or on none, is not accepted
        names, _ = self.errs(cite(self.c1))
        self.assertEqual(names, ["decision-revision-mismatch"])

    def test_listed_revision_of_one_decision_does_not_serve_another(self):
        def mutate(data):
            self.dec(data).update({"revisions": [entry(self.c1)]})
            a = copy.deepcopy(data["applicability"][1])  # on d-obs at c2
            a["applicability_id"] = "ap-other"
            a["decision_revision"] = f"git:{self.c1}"
            data["applicability"].append(a)
        names, _ = self.errs(mutate)
        self.assertEqual(names, ["decision-revision-mismatch"])


class RevisionsErrorTests(SchemaFixture):
    def test_h4_revisions_errors(self):
        c1, c2 = self.c1, self.c2
        cases = {
            "unresolvable commit": ([entry("deadbeefdeadbeef")], "pin-unresolved"),
            "document missing at that commit": ([entry(c1, document="docs/nope.md")], "pin-unresolved"),
            "lines outside the file": ([entry(c1, lines="1-99")], "pin-lines-out-of-range"),
            "entry equals the current revision": ([entry(c2)], "revision-equals-current"),
            "duplicate entry": ([entry(c1), entry(c1)], "duplicate-revision"),
            "entry not an ancestor of the current revision": ([entry(self.orphan)], "revision-not-ancestor"),
            "unknown key": ([entry(c1, extra="x")], "unknown-key"),
            "missing key": ([{k: v for k, v in entry(c1).items() if k != "reason"}], "missing-key"),
            "bad date": ([entry(c1, superseded_at="yesterday")], "bad-value"),
            "bad revision text": ([entry(c1, revision="c1")], "bad-value"),
            "not a list": ({"revision": f"git:{c1}"}, "bad-value"),
            "an empty list": ([], "bad-value"),
        }
        for label, (value, expected) in cases.items():
            with self.subTest(label):
                names, items = self.errs(self.with_dec(revisions=value))
                self.assertIn(expected, names, items)

    def test_h4_entries_must_be_ancestors_of_the_next_entry(self):
        # c3 is the current revision; c2 and c1 are both ancestors of it, but only c1 precedes c2
        ok = self.with_dec(revision=f"git:{self.c3}", revisions=[entry(self.c1), entry(self.c2)])
        names, items = self.errs(ok)
        self.assertEqual(names, [], items)  # ap-one cites c2, which is now a listed revision
        bad = self.with_dec(revision=f"git:{self.c3}", revisions=[entry(self.c2), entry(self.c1)])
        names, items = self.errs(bad)
        self.assertEqual(names, ["revision-not-ancestor"], items)
        self.assertIn("revisions[0] is not an ancestor of revisions[1]", items[0][2])

    def test_a_valid_chain_passes(self):
        names, items = self.errs(self.with_dec(revisions=[entry(self.c1, lines="1-2", document="docs/model.md")]))
        self.assertEqual(names, [], items)


class HistoricalMarkerTests(SchemaFixture):
    MARK = {"reason": "pins the superseded reading", "marked_by": "user (fixture owner)", "marked_at": "2026-02-01"}

    def test_h5_historical_marker(self):
        names, items = self.errs(self.with_dec(historical=dict(self.MARK)))
        self.assertEqual(names, [], items)
        names, items = self.errs(self.with_dec(historical={**self.MARK, "superseded_by": "d-model-old"}))
        self.assertEqual(names, [], items)
        names, _ = self.errs(self.with_dec(historical={**self.MARK, "superseded_by": "d-missing"}))
        self.assertEqual(names, ["unknown-decision-reference"])
        names, _ = self.errs(self.with_dec(historical={**self.MARK, "superseded_by": "d-model-new"}))
        self.assertEqual(names, ["bad-value"])
        names, _ = self.errs(self.with_dec(historical={k: v for k, v in self.MARK.items() if k != "reason"}))
        self.assertEqual(names, ["missing-key"])
        names, _ = self.errs(self.with_dec(historical={**self.MARK, "marked_at": "soon"}))
        self.assertEqual(names, ["bad-value"])
        names, _ = self.errs(self.with_dec(historical={**self.MARK, "extra": 1}))
        self.assertEqual(names, ["unknown-key"])
        names, _ = self.errs(self.with_dec(historical="yes"))
        self.assertEqual(names, ["bad-value"])

    def test_a_marker_does_not_change_what_applicability_may_cite(self):
        def mutate(data):
            self.dec(data).update({"historical": dict(self.MARK)})
            a = copy.deepcopy(data["applicability"][0])
            a["applicability_id"] = "ap-old"
            a["decision_revision"] = f"git:{self.c1}"
            data["applicability"].append(a)
        names, _ = self.errs(mutate)
        self.assertEqual(names, ["decision-revision-mismatch"])


class RuleSevenTests(SchemaFixture):
    def test_h6_global_usefulness_and_obsolescence_keys_stay_rejected(self):
        mark = dict(HistoricalMarkerTests.MARK)
        for key in ("obsolete", "is_useful", "usefulness", "is_obsolete"):
            for label, keys in (("decision", {key: True}),
                                ("historical", {"historical": {**mark, key: True}}),
                                ("revisions entry", {"revisions": [entry(self.c1, **{key: True})]}),
                                ("evidence_state", {"findings": [], "evidence_state": {"state": "none_linked", key: 1}})):
                with self.subTest(key=key, where=label):
                    names, items = self.errs(self.with_dec(**keys))
                    self.assertIn("forbidden-global-field", names, items)


class EvidenceFreeValidationTests(SchemaFixture):
    CORPUS = {"id": "register-view-14031bc2", "sha256": "ab" * 32}
    FOUND = {"state": "none_found", "corpus": {"id": "register-view-14031bc2", "sha256": "ab" * 32},
             "search_scope": "studies/*.json findings, claims in scope", "coverage": "all 9 studies",
             "assessor": "tester", "date": "2026-02-02"}

    def free(self, state):
        return self.with_dec(findings=[], evidence_state=state)

    def test_h7_none_linked_validates(self):
        names, items = self.errs(self.free({"state": "none_linked"}))
        self.assertEqual(names, [], items)

    def test_h7_empty_findings_without_state_fails_with_todays_message(self):
        names, items = self.errs(self.with_dec(findings=[]))
        self.assertEqual(names, ["bad-value"])
        self.assertEqual(items[0][2], "findings must be a non-empty list")

    def test_h7_none_found_validates_with_digest_or_pin(self):
        names, items = self.errs(self.free(dict(self.FOUND)))
        self.assertEqual(names, [], items)
        with_pin = {**self.FOUND, "corpus": {"id": "doc", "pin": self.pin("1-2")}}
        names, items = self.errs(self.free(with_pin))
        self.assertEqual(names, [], items)

    def test_h7_none_found_fails_once_for_each_missing_item(self):
        def without(path):
            st = copy.deepcopy(self.FOUND)
            if path[0] == "corpus":
                del st["corpus"][path[1]]
                if path[1] == "sha256":
                    st["corpus"].pop("pin", None)
            else:
                del st[path[0]]
            return st
        cases = {"corpus id": ("corpus", "id"), "digest or pin": ("corpus", "sha256"), "search_scope": ("search_scope",),
                 "coverage": ("coverage",), "assessor": ("assessor",), "date": ("date",)}
        for label, path in cases.items():
            with self.subTest(missing=label):
                names, items = self.errs(self.free(without(path)))
                self.assertEqual(names, ["missing-key"], items)
                self.assertEqual(len(items), 1, items)

    def test_h7_evidence_state_with_findings_fails(self):
        names, items = self.errs(self.with_dec(evidence_state={"state": "none_linked"}))
        self.assertEqual(names, ["bad-value"], items)
        self.assertIn("only when findings is []", items[0][2])

    def test_state_vocabulary_and_extra_keys(self):
        for state in ({"state": "unknown"}, {"state": "none_linked", "coverage": "x"}, {"coverage": "x"}):
            with self.subTest(state=state):
                names, _ = self.errs(self.free(state))
                self.assertTrue(names)
        names, _ = self.errs(self.free({**self.FOUND, "corpus": {"id": "x", "sha256": "nothex"}}))
        self.assertEqual(names, ["bad-value"])


    def test_s9_an_unknown_key_under_none_linked_is_one_error(self):
        names, items = self.errs(self.free({"state": "none_linked", "foo": 1}))
        self.assertEqual(names, ["unknown-key"], items)
        self.assertEqual(len(items), 1, items)
        # a key that belongs to none_found only is still refused, once, with the state-specific message
        names, items = self.errs(self.free({"state": "none_linked", "coverage": "x"}))
        self.assertEqual(names, ["unknown-key"], items)
        self.assertEqual(len(items), 1, items)
        self.assertIn("none_linked", items[0][2])


# ---------------------------------------------------------------------------
# views (H8-H10)
# ---------------------------------------------------------------------------

class NominateViewTests(SchemaFixture):
    def nominate(self, mutate):
        root = self.make(mutate)
        out = os.path.join(self.base, f"nom-{os.path.basename(root)}.json")
        p = run_script(SCRIPT, "nominate", "--register", root, "--repo", self.repo, "--out", out)
        self.assertEqual(p.returncode, 0, p.stdout + p.stderr)
        with open(out, encoding="utf-8") as fh:
            return fh.read()

    def test_h8_evidence_free_decision_is_never_reached_and_the_state_is_printed(self):
        def mutate(data):
            data["decisions"].append({"decision_id": "d-free", "document": "docs/model.md", "anchor": "free",
                                      "revision": f"git:{self.c2}", "owner": "user (fixture owner)", "findings": [],
                                      "evidence_state": dict(EvidenceFreeValidationTests.FOUND)})
        plain = json.loads(self.nominate(None))
        text = self.nominate(mutate)
        doc = json.loads(text)
        for nom in doc["nominations"]:
            self.assertNotIn("d-free", [d["decision_id"] for d in nom["decisions"]])
        self.assertEqual(doc["nominations"], plain["nominations"])
        free = doc["evidence_free_decisions"]
        self.assertEqual([f["decision_id"] for f in free], ["d-free"])
        self.assertEqual(free[0]["evidence_state"], EvidenceFreeValidationTests.FOUND)  # verbatim
        self.assertIn("not evidence of absence", free[0]["statement"])
        self.assertNotIn("evidence_free_decisions", plain)
        for phrase in ("no evidence exists", "evidence does not exist", "absence of evidence is established"):
            self.assertNotIn(phrase, text.lower())

    def test_h9_nominate_still_reaches_a_historical_decision_and_labels_it(self):
        mark = dict(HistoricalMarkerTests.MARK)

        def mutate(data):
            self.dec(data, "d-claim").update({"historical": mark})
        plain = json.loads(self.nominate(None))
        doc = json.loads(self.nominate(mutate))
        reached = [[d["decision_id"] for d in n["decisions"]] for n in doc["nominations"]]
        self.assertEqual(reached, [[d["decision_id"] for d in n["decisions"]] for n in plain["nominations"]])
        labelled = [d for n in doc["nominations"] for d in n["decisions"] if d["decision_id"] == "d-claim"]
        self.assertTrue(labelled)
        for d in labelled:
            self.assertIs(d["historical"], True)
            self.assertEqual(d["historical_reason"], mark["reason"])
        for n in doc["nominations"]:  # removing the labels leaves the plain output
            for d in n["decisions"]:
                d.pop("historical", None)
                d.pop("historical_reason", None)
        self.assertEqual(doc, plain)

    def test_h10_nominate_labels_a_record_at_a_prior_revision(self):
        def mutate(data):
            self.dec(data).update({"revisions": [entry(self.c1)]})
            a = copy.deepcopy(data["applicability"][0])
            a["applicability_id"] = "ap-prior"
            a["decision_revision"] = f"git:{self.c1}"
            data["applicability"].append(a)
        doc = json.loads(self.nominate(mutate))
        recs = [r for n in doc["nominations"] for r in n["applicability_records"]]
        prior = [r for r in recs if r["applicability_id"] == "ap-prior"]
        current = [r for r in recs if r["applicability_id"] == "ap-one"]
        self.assertTrue(prior and current)
        self.assertTrue(all(r.get("at_prior_revision") is True for r in prior))
        self.assertTrue(all("at_prior_revision" not in r for r in current))


    def test_s4_a_record_on_a_historical_decision_carries_the_label_reached_or_not(self):
        mark = dict(HistoricalMarkerTests.MARK)

        def mutate(data):
            self.dec(data, "d-claim").update({"historical": mark})
            self.dec(data, "d-model-new").update({"historical": mark})
            for aid, finding, did, rev in (("ap-reached", "beta-study#claim", "d-claim", self.c2),
                                           ("ap-offlist", "beta-study#obs", "d-model-new", self.c2)):
                a = copy.deepcopy(data["applicability"][0])
                a.update({"applicability_id": aid, "finding": finding, "decision_id": did,
                          "decision_revision": f"git:{rev}"})
                a["authorization"] = {"status": "authorized", "owner": "user (fixture owner)", "at": "2026-01-02"}
                data["applicability"].append(a)
        doc = json.loads(self.nominate(mutate))
        challenge = next(n for n in doc["nominations"] if n["event_id"] == "ev-challenge")
        recs = {r["applicability_id"]: r for r in challenge["applicability_records"]}
        self.assertIs(recs["ap-reached"].get("historical"), True)
        self.assertIs(recs["ap-offlist"].get("historical"), True)
        # d-model-new lists neither finding of this event, so it is not reached: the row alone carries the marker
        self.assertNotIn("d-model-new", [d["decision_id"] for d in challenge["decisions"]])
        self.assertIn("d-claim", [d["decision_id"] for d in challenge["decisions"]])
        self.assertNotIn("historical", recs["ap-two"])  # a record on an unmarked decision
        # the key is present only when the decision carries the marker
        plain = json.loads(self.nominate(None))
        for n in plain["nominations"]:
            for r in n["applicability_records"]:
                self.assertNotIn("historical", r)


class BuildLabelTests(SchemaFixture):
    """S-1: `build` labels rows that must not read as current authority; unaffected registers are unchanged."""

    def build(self, mutate):
        root = self.make(mutate)
        out = os.path.join(self.base, f"build-{os.path.basename(root)}")
        p = run_script(SCRIPT, "build", "--register", root, "--repo", self.repo, "--out", out)
        self.assertEqual(p.returncode, 0, p.stdout + p.stderr)
        with open(os.path.join(out, "current-knowledge.json"), encoding="utf-8") as fh:
            js = json.load(fh)
        with open(os.path.join(out, "current-knowledge.md"), encoding="utf-8") as fh:
            return js, fh.read()

    @staticmethod
    def rows(js):
        return {a["applicability_id"]: a for f in js["findings"].values() for a in f["applicability"]}

    @staticmethod
    def md_row(md, aid):
        return next(line for line in md.splitlines() if line.rstrip().endswith(aid) or f"| {aid} (" in line)

    def prior_mutate(self, data):
        self.dec(data).update({"revisions": [entry(self.c1)]})
        a = copy.deepcopy(data["applicability"][0])
        a["applicability_id"] = "ap-prior"
        a["decision_revision"] = f"git:{self.c1}"
        a["authorization"] = {"status": "authorized", "owner": "user (fixture owner)", "at": "2026-01-02"}
        data["applicability"].append(a)

    def test_s1_a_prior_revision_row_is_labelled_and_names_the_current_revision(self):
        js, md = self.build(self.prior_mutate)
        rows = self.rows(js)
        self.assertIs(rows["ap-prior"]["at_prior_revision"], True)
        self.assertEqual(rows["ap-prior"]["current_decision_revision"], f"git:{self.c2}")
        self.assertEqual(rows["ap-prior"]["authorization"], "authorized")
        self.assertNotIn("at_prior_revision", rows["ap-one"])
        self.assertNotIn("historical_pin", rows["ap-prior"])
        line = self.md_row(md, "ap-prior")
        self.assertIn("prior revision", line)
        self.assertIn(f"current revision is git:{self.c2}", line)
        self.assertIn("not carried", line)
        self.assertNotIn("prior revision", self.md_row(md, "ap-one"))
        self.assertNotIn("historical pin", md)

    def test_s1_a_historical_decision_row_is_labelled_not_current_authority(self):
        mark = dict(HistoricalMarkerTests.MARK)

        def mutate(data):
            self.dec(data).update({"historical": mark})
            data["applicability"][0]["authorization"] = {"status": "authorized", "owner": "user (fixture owner)",
                                                         "at": "2026-01-02"}
        js, md = self.build(mutate)
        rows = self.rows(js)
        self.assertIs(rows["ap-one"]["historical_pin"], True)
        self.assertNotIn("historical_pin", rows["ap-two"])
        self.assertIn("historical pin, not current authority", self.md_row(md, "ap-one"))
        self.assertNotIn("historical pin", self.md_row(md, "ap-two"))
        self.assertNotIn("at_prior_revision", rows["ap-one"])

    def test_s1_a_register_using_neither_key_gets_no_label(self):
        js, md = self.build(None)
        for row in self.rows(js).values():
            self.assertEqual(set(row), {"applicability_id", "decision_id", "decision_revision", "document", "anchor",
                                        "relation", "authorization"})
        self.assertNotIn("prior revision", md)
        self.assertNotIn("historical pin", md)


class AnchorViewFixture(ta.AnchorFixture):
    def anchor_report(self, at):
        p, out = self.anchors(at)
        self.assertEqual(p.returncode, 0, p.stdout + p.stderr)
        with open(os.path.join(out, "anchors.json"), encoding="utf-8") as fh:
            rep = json.load(fh)
        with open(os.path.join(out, "anchors.md"), encoding="utf-8") as fh:
            return rep, fh.read()

    def anchors(self, at=None, out=None, repo=None, register=None, env=None):
        out = out or self.out_dir()
        args = [sys.executable, "-I", "-B", SCRIPT, "anchors", "--repo", repo or self.repo, "--out", out]
        if at:
            args.append(f"--at={at}")
        p = subprocess.run(args, capture_output=True, text=True, check=False, env={**os.environ, **(env or {})})
        return p, out


class AnchorViewTests(AnchorViewFixture):
    def test_h8_anchors_prints_the_evidence_state_verbatim(self):
        rev = self.commit_files({"docs/a.md": ta.DOC}, "v1")
        state = dict(EvidenceFreeValidationTests.FOUND)
        d = self.decision("free", "docs/a.md", rev, "4-5", findings=())
        d["evidence_state"] = state
        self.commit_register([d, self.decision("plain", "docs/a.md", rev, "4-5")])
        rep, md = self.anchor_report(None)
        e = self.by_id(rep)
        self.assertEqual(e["free"]["evidence_state"], state)
        self.assertIn("not evidence of absence", e["free"]["evidence_state_statement"])
        self.assertIn("not evidence of absence", md)
        self.assertIn(json.dumps(state, sort_keys=True), md)
        self.assertNotIn("evidence_state", e["plain"])
        for phrase in ("no evidence exists", "evidence does not exist"):
            self.assertNotIn(phrase, md.lower())

    def _plain_and_marked(self, marker, with_appl=True):
        rev = self.commit_files({"docs/a.md": ta.DOC}, "v1")
        self.commit_files({"docs/a.md": ta.DOC.replace("gamma", "G")}, "v2")
        decs = [self.decision("d", "docs/a.md", rev, "3"), self.decision("m", "docs/a.md", rev, "4-6")]
        appl = ([self.appl("ap-pend", "m", rev), self.appl("ap-auth", "m", rev, "authorized"),
                 self.appl("ap-other", "d", rev)] if with_appl else [])
        plain = self.commit_register(decs, appl, msg="plain")
        marked = copy.deepcopy(decs)
        marked[1]["historical"] = marker
        marked_commit = self.commit_register(marked, appl, msg="marked")
        return plain, marked_commit

    def test_h9_historical_presentation(self):
        marker = dict(HistoricalMarkerTests.MARK)
        plain, marked = self._plain_and_marked(marker)
        rp, _ = self.anchor_report(plain)
        rm, md = self.anchor_report(marked)
        bp, bm = self.by_id(rp), self.by_id(rm)
        self.assertEqual({k: v["class"] for k, v in bp.items()}, {k: v["class"] for k, v in bm.items()})
        self.assertEqual(rp["summary"], rm["summary"])
        rows = bm["m"]["applicability"]
        self.assertEqual({r["record"]["applicability_id"]: r["after_state"] for r in rows},
                         {"ap-auth": "historical_pin", "ap-pend": "historical_pin"})
        self.assertNotIn("carried_unchanged", [r["after_state"] for r in rows])
        auth = next(r for r in rows if r["record"]["applicability_id"] == "ap-auth")
        self.assertEqual(auth["authorization_after"], "authorized_at_prior_revision")
        self.assertEqual(bm["m"]["authority_status"], "historical, not current authority")
        self.assertEqual(bm["m"]["historical"], marker)
        self.assertIn("## Historical decisions (not current authority)", md)
        self.assertIn("`m` (historical, not current authority)", md)
        # the unmarked decision is untouched
        self.assertEqual(bp["d"], bm["d"])
        self.assertNotIn("historical", bm["d"])

    def test_h9_a_marked_decision_that_is_unchanged_still_gets_historical_rows(self):
        rev = self.commit_files({"docs/a.md": ta.DOC}, "v1")
        d = self.decision("m", "docs/a.md", rev, "4-5")
        d["historical"] = dict(HistoricalMarkerTests.MARK)
        c = self.commit_register([d], [self.appl("ap-auth", "m", rev, "authorized")])
        rep, _ = self.anchor_report(c)
        e = self.by_id(rep)["m"]
        self.assertEqual(e["class"], "unchanged")
        self.assertEqual([(r["after_state"], r["authorization_after"]) for r in e["applicability"]],
                         [("historical_pin", "authorized_at_prior_revision")])

    def test_the_marker_note_replaces_the_fixed_note_for_the_known_pin(self):
        rev = self.commit_files({"docs/a.md": ta.DOC}, "v1")
        did = "model-selection.claude-host-builder-rows-at-df288c3d"
        marked = self.decision(did, "docs/a.md", rev, "4-5")
        marked["historical"] = dict(HistoricalMarkerTests.MARK)
        c = self.commit_register([marked])
        rep, _ = self.anchor_report(c)
        notes = self.by_id(rep)[did]["notes"]
        self.assertEqual(len(notes), 1, notes)
        self.assertIn("historical pin", notes[0])
        self.assertIn(HistoricalMarkerTests.MARK["reason"], notes[0])
        self.assertNotIn("no field marks it", notes[0])
        # unmarked, the base note is unchanged
        plain = self.decision(did, "docs/a.md", rev, "4-5")
        c2 = self.commit_register([plain], msg="plain")
        notes = self.by_id(self.anchor_report(c2)[0])[did]["notes"]
        self.assertEqual(notes, ["deliberate historical pin (named in the register README prose, no field marks it); "
                                 "reported with its class like any other decision"])

    def _prior_history(self, moved=False):
        r1 = self.commit_files({"docs/a.md": ta.DOC}, "v1")
        r2 = self.commit_files({"docs/a.md": ta.DOC.replace("gamma", "G")}, "v2")
        if moved:
            self.commit_files({"docs/a.md": ta.DOC.replace("gamma", "G").replace("# Title\n\n", "# Title\n\nx\ny\n")}, "v3")
        d = self.decision("d", "docs/a.md", r2, "6-7")
        d["revisions"] = [entry(r1, lines="5-6", document="docs/a.md")]
        appl = [self.appl("ap-auth", "d", r1, "authorized"), self.appl("ap-pend", "d", r1), self.appl("ap-now", "d", r2)]
        return self.commit_register([d], appl), r1, r2

    def test_h10_prior_revision_presentation(self):
        c, r1, r2 = self._prior_history()
        rep, md = self.anchor_report(c)
        e = self.by_id(rep)["d"]
        self.assertEqual(e["class"], "unchanged")
        rows = {r["record"]["applicability_id"]: r for r in e["applicability"]}
        self.assertEqual(sorted(rows), ["ap-auth", "ap-pend"])  # the record at the current revision is not listed
        self.assertEqual(rows["ap-auth"]["after_state"], "at_prior_revision")
        self.assertEqual(rows["ap-auth"]["authorization_after"], "authorized_at_prior_revision")
        self.assertEqual(rows["ap-pend"]["after_state"], "at_prior_revision")
        self.assertIsNone(rows["ap-pend"]["authorization_after"])
        for r in rows.values():
            self.assertFalse(r["record_revision_matches_decision"])
            self.assertNotEqual(r["authorization_after"], "carried_unchanged")
        self.assertEqual(e["revisions"][0]["revision"], f"git:{r1[:10]}")
        self.assertIn("## Decisions with listed prior revisions", md)
        self.assertIn("authorization `authorized_at_prior_revision`", md)

    def test_h10_a_moved_current_revision_still_does_not_carry_a_prior_record(self):
        c, r1, r2 = self._prior_history(moved=True)
        rep, _ = self.anchor_report(c)
        e = self.by_id(rep)["d"]
        self.assertEqual(e["class"], "moved")
        states = {r["record"]["applicability_id"]: (r["after_state"], r["authorization_after"]) for r in e["applicability"]}
        self.assertEqual(states["ap-auth"], ("at_prior_revision", "authorized_at_prior_revision"))
        self.assertEqual(states["ap-pend"], ("at_prior_revision", None))
        self.assertEqual(states["ap-now"][0], "carried_unchanged")
        self.assertNotIn("carried_unchanged", [s[1] for s in states.values()])


# ---------------------------------------------------------------------------
# history (H11-H15)
# ---------------------------------------------------------------------------

class HistoryFixture(AnchorViewFixture):
    MARK = HistoricalMarkerTests.MARK

    def setUp(self):
        super().setUp()
        self.r1 = self.commit_files({"docs/a.md": ta.DOC}, "v1")
        self.r2 = self.commit_files({"docs/a.md": ta.DOC.replace("gamma", "G")}, "v2")
        self.r3 = self.commit_files({"docs/a.md": ta.DOC.replace("gamma", "G").replace("delta", "D")}, "v3")

    def base_state(self):
        d = self.decision("d", "docs/a.md", self.r3, "4-5")
        d["revisions"] = [entry(self.r1, lines="3", document="docs/a.md"), entry(self.r2, lines="3-4")]
        h = self.decision("h", "docs/a.md", self.r1, "4-5")
        h["historical"] = dict(self.MARK)
        decisions = [d, h, self.decision("plain", "docs/a.md", self.r1, "3")]
        appl = [self.appl("ap-pend", "d", self.r3), self.appl("ap-auth", "d", self.r2, "authorized"),
                self.appl("ap-h", "h", self.r1)]
        return decisions, appl

    def commit_base(self):
        decisions, appl = self.base_state()
        self.base_commit = self.commit_register(decisions, appl, msg="base register")
        return decisions, appl

    def later(self, mutate):
        decisions, appl = self.base_state()
        mutate(decisions, appl)
        return self.commit_register(decisions, appl, msg="later register")

    def history(self, base=None, at=None, extra=(), env=None):
        args = ["history", "--repo", self.repo]
        if base is not False:
            args += ["--base", base or self.base_commit]
        if at:
            args += ["--at", at]
        args += list(extra)
        p = run_script(SCRIPT, *args, env=env)
        try:
            rep = json.loads(p.stdout) if p.stdout.strip().startswith("{") else None
        except ValueError:
            rep = None
        return p, rep

    @staticmethod
    def names(rep):
        return [v["violation"] for v in rep["violations"]]

    def assert_forbidden_phrases_absent(self, text):
        low = text.lower()
        for phrase in FORBIDDEN_PHRASES:
            self.assertNotIn(phrase, low)


class HistoryPassTests(HistoryFixture):
    def test_h11_a_correctly_appended_revision_passes(self):
        self.commit_base()
        # a fourth commit is the new current revision
        r4 = self.commit_files({"docs/a.md": ta.DOC.replace("gamma", "G").replace("delta", "D") + "more\n"}, "v4")

        def mutate4(decisions, appl):
            d = decisions[0]
            d["revisions"].append({"revision": d["revision"], "anchor": d["anchor"], "superseded_at": "2026-02-03",
                                   "reason": "new text", "lines": d["lines"], "document": d["document"]})
            d["revision"] = f"git:{r4[:10]}"
            d["anchor"] = "new anchor"
            appl.append(self.appl("ap-new", "d", r4))
        later = self.later(mutate4)
        p, rep = self.history(at=later)
        self.assertEqual(p.returncode, 0, p.stdout + p.stderr)
        self.assertEqual(rep["violations"], [])
        self.assertEqual(rep["owner_review"], [])
        self.assertEqual(rep["later"]["commit"], later)

    def test_h11_pending_to_authorized_with_at_passes_and_is_reported_for_owner_review(self):
        self.commit_base()

        def mutate(decisions, appl):
            for a in appl:
                if a["applicability_id"] == "ap-pend":
                    a["authorization"] = {"status": "authorized", "owner": "user (fixture owner)", "at": "2026-02-02"}
        p, rep = self.history(at=self.later(mutate))
        self.assertEqual(p.returncode, 0, p.stdout + p.stderr)
        self.assertEqual(rep["violations"], [])
        self.assertEqual(len(rep["owner_review"]), 1)
        t = rep["owner_review"][0]
        self.assertEqual(t["applicability_id"], "ap-pend")
        self.assertEqual(t["before"], {"status": "pending", "owner": "user (fixture owner)"})
        self.assertEqual(t["after"], {"status": "authorized", "owner": "user (fixture owner)", "at": "2026-02-02"})
        self.assertEqual(t["owner_approval"], "not_established")
        self.assertEqual(t["structural_check"], "permitted by register rule 2")
        self.assertIn("structurally permits", rep["limitations"])
        self.assertIn(LIMITATION_NEEDLE, rep["limitations"])
        self.assertIn("owner_approval not_established", rep["limitations"])
        self.assert_forbidden_phrases_absent(p.stdout)

    def test_h11_authorized_to_pending_passes_and_keeps_the_earlier_authorization_visible(self):
        self.commit_base()

        def mutate(decisions, appl):
            for a in appl:
                if a["applicability_id"] == "ap-auth":
                    a["authorization"] = {"status": "pending", "owner": "user (fixture owner)", "at": "2026-01-01"}
        p, rep = self.history(at=self.later(mutate))
        self.assertEqual(p.returncode, 0, p.stdout + p.stderr)
        self.assertEqual(rep["violations"], [])
        t = rep["owner_review"][0]
        self.assertEqual(t["before"], {"status": "authorized", "owner": "user (fixture owner)", "at": "2026-01-01"})
        self.assertEqual(t["after"]["status"], "pending")
        self.assertEqual(t["owner_approval"], "not_established")
        for forbidden in ("withdraw", "revoked"):
            self.assertNotIn(forbidden, p.stdout.lower())
        self.assert_forbidden_phrases_absent(p.stdout)

    def test_h11_adding_at_alone_is_a_reported_transition(self):
        self.commit_base()

        def mutate(decisions, appl):
            for a in appl:
                if a["applicability_id"] == "ap-pend":
                    a["authorization"]["at"] = "2026-03-03"
        p, rep = self.history(at=self.later(mutate))
        self.assertEqual(p.returncode, 0, p.stdout)
        self.assertEqual([t["applicability_id"] for t in rep["owner_review"]], ["ap-pend"])

    def test_h11_a_record_added_with_a_decided_status_is_reported_and_passes(self):
        self.commit_base()

        def mutate(decisions, appl):
            appl.append(self.appl("ap-born-authorized", "d", self.r3, "authorized"))
        p, rep = self.history(at=self.later(mutate))
        self.assertEqual(p.returncode, 0, p.stdout)
        self.assertEqual([(t["kind"], t["applicability_id"], t["owner_approval"]) for t in rep["owner_review"]],
                         [("added_with_decided_status", "ap-born-authorized", "not_established")])

    def test_history_without_at_compares_the_register_files_on_disk(self):
        self.commit_base()
        path = os.path.join(self.repo, "metrics", "register", "applicability.jsonl")
        decisions, appl = self.base_state()
        appl[0]["authorization"] = {"status": "authorized", "owner": "user (fixture owner)", "at": "2026-02-02"}
        with open(path, "w") as fh:
            for r in appl:
                fh.write(json.dumps(r) + "\n")
        p, rep = self.history()
        self.assertEqual(p.returncode, 0, p.stdout + p.stderr)
        self.assertIsNone(rep["later"]["commit"])
        self.assertIn("not pinned", rep["later"]["source"])
        self.assertEqual(len(rep["owner_review"]), 1)
        # and an unchanged disk state is an empty pass
        self.assertEqual(self.history(at="HEAD")[1]["owner_review"], [])


def git_head(repo):
    return ta.git(repo, "rev-parse", "HEAD")


class HistoryViolationTests(HistoryFixture):
    def expect(self, label, mutate, name, rc=1):
        with self.subTest(label):
            later = self.later(mutate)
            p, rep = self.history(at=later)
            self.assertEqual(p.returncode, rc, p.stdout + p.stderr)
            self.assertIn(name, self.names(rep), rep["violations"])
            self.assert_forbidden_phrases_absent(p.stdout)
            return rep

    def test_h12_violations(self):
        r4 = self.commit_files({"docs/a.md": ta.DOC + "r4\n"}, "r4")
        self.commit_base()

        def remove_revision(decisions, appl):
            decisions[0]["revisions"].pop(0)

        def edit_revision(decisions, appl):
            decisions[0]["revisions"][0]["reason"] = "rewritten"

        def edit_revision_document(decisions, appl):
            decisions[0]["revisions"][1]["document"] = "docs/other.md"

        def change_current_without_history(decisions, appl):
            decisions[0]["revision"] = f"git:{r4[:10]}"

        def change_current_with_wrong_entry(decisions, appl):
            d = decisions[0]
            d["revisions"].append({"revision": d["revision"], "anchor": "different anchor", "superseded_at": "2026-02-03",
                                   "reason": "r", "lines": d["lines"], "document": d["document"]})
            d["revision"] = f"git:{r4[:10]}"

        def change_current_with_wrong_lines(decisions, appl):
            d = decisions[0]
            d["revisions"].append({"revision": d["revision"], "anchor": d["anchor"], "superseded_at": "2026-02-03",
                                   "reason": "r", "lines": "1", "document": d["document"]})
            d["revision"] = f"git:{r4[:10]}"

        def old_entry_not_last(decisions, appl):
            d = decisions[0]
            d["revisions"].insert(0, {"revision": d["revision"], "anchor": d["anchor"], "superseded_at": "2026-02-03",
                                      "reason": "r", "lines": d["lines"], "document": d["document"]})
            d["revision"] = f"git:{r4[:10]}"

        def remove_decision(decisions, appl):
            decisions.pop(2)

        def rename_decision(decisions, appl):
            decisions[2]["decision_id"] = "plain-renamed"

        def remove_marker(decisions, appl):
            del decisions[1]["historical"]

        def edit_marker(decisions, appl):
            decisions[1]["historical"]["reason"] = "reworded"

        def remove_record(decisions, appl):
            appl.pop(0)

        def change_rationale(decisions, appl):
            appl[0]["rationale"] = "different"

        def change_relation(decisions, appl):
            appl[0]["relation"] = "challenges"

        def change_owner(decisions, appl):
            appl[1]["authorization"]["owner"] = "someone else"

        def remove_at(decisions, appl):
            del appl[1]["authorization"]["at"]

        def change_citation(decisions, appl):
            appl[0]["decision_revision"] = f"git:{self.r1[:10]}"

        cases = [
            ("listed revision removed", remove_revision, "revision-removed"),
            ("listed revision edited", edit_revision, "revision-edited"),
            ("listed revision's effective document edited", edit_revision_document, "revision-edited"),
            ("current revision changed without moving the old one", change_current_without_history,
             "current-revision-changed-without-history"),
            ("old current entry appended with a different anchor", change_current_with_wrong_entry,
             "current-revision-changed-without-history"),
            ("old current entry appended with different lines", change_current_with_wrong_lines,
             "current-revision-changed-without-history"),
            ("old current entry not last", old_entry_not_last, "current-revision-changed-without-history"),
            ("decision id removed", remove_decision, "decision-id-removed"),
            ("decision id renamed", rename_decision, "decision-id-renamed"),
            ("historical marker removed", remove_marker, "historical-marker-removed"),
            ("historical marker edited", edit_marker, "historical-marker-edited"),
            ("applicability record removed", remove_record, "applicability-record-removed"),
            ("applicability rationale changed", change_rationale, "applicability-record-changed"),
            ("applicability relation changed", change_relation, "applicability-record-changed"),
            ("applicability citation changed", change_citation, "applicability-record-changed"),
            ("authorization owner changed", change_owner, "authorization-owner-changed"),
            ("authorization at removed", remove_at, "authorization-at-removed"),
        ]
        for label, mutate, name in cases:
            self.expect(label, mutate, name)

    def test_s2_a_blanked_authorization_at_is_removed(self):
        self.commit_base()
        for label, value in (("null", None), ("empty string", ""), ("whitespace only", "  "), ("not text", 20260101)):
            def mutate(decisions, appl, value=value):
                appl[1]["authorization"]["at"] = value
            with self.subTest(label):
                p, rep = self.history(at=self.later(mutate))
                self.assertEqual(p.returncode, 1, p.stdout + p.stderr)
                self.assertEqual(self.names(rep), ["authorization-at-removed"])
                self.assertEqual(rep["owner_review"][0]["before"]["at"], "2026-01-01")  # the earlier value stays visible
        with self.subTest("controls: a different date and an absent base at are not removals"):
            def other_date(decisions, appl):
                appl[1]["authorization"]["at"] = "2026-03-03"
            p, rep = self.history(at=self.later(other_date))
            self.assertEqual((p.returncode, self.names(rep)), (0, []))
            self.assertEqual(len(rep["owner_review"]), 1)

            def add_at(decisions, appl):
                appl[0]["authorization"] = {"status": "pending", "owner": "user (fixture owner)", "at": ""}
            p, rep = self.history(at=self.later(add_at))
            self.assertEqual((p.returncode, self.names(rep)), (0, []))

    def test_s5_a_reordered_revisions_list_is_a_violation(self):
        self.commit_base()
        later = self.later(lambda decisions, appl: decisions[0]["revisions"].reverse())
        p, rep = self.history(at=later)
        self.assertEqual(p.returncode, 1, p.stdout + p.stderr)
        self.assertIn("revision-reordered", self.names(rep))
        self.assertNotIn("revision-removed", self.names(rep))
        self.assertNotIn("revision-edited", self.names(rep))

    def test_h12_a_rename_is_told_apart_from_a_removal(self):
        self.commit_base()
        later = self.later(lambda decisions, appl: decisions[2].update({"decision_id": "plain-renamed"}))
        _, rep = self.history(at=later)
        self.assertEqual(self.names(rep), ["decision-id-renamed"])
        later2 = self.later(lambda decisions, appl: decisions.pop(2))
        _, rep2 = self.history(at=later2)
        self.assertEqual(self.names(rep2), ["decision-id-removed"])

    def test_h12_a_status_change_with_an_owner_change_is_a_violation_and_is_still_listed(self):
        self.commit_base()

        def mutate(decisions, appl):
            appl[1]["authorization"] = {"status": "pending", "owner": "other", "at": "2026-01-01"}
        later = self.later(mutate)
        p, rep = self.history(at=later)
        self.assertEqual(p.returncode, 1)
        self.assertEqual(self.names(rep), ["authorization-owner-changed"])
        self.assertEqual(rep["owner_review"][0]["before"]["status"], "authorized")
        self.assertEqual(rep["owner_review"][0]["structural_check"], "not permitted, see violations")

    def test_h12_an_unreadable_later_record_is_a_violation(self):
        self.commit_base()
        decisions, appl = self.base_state()
        self.commit_register(decisions, appl[1:], msg="drops one")
        path = os.path.join(self.repo, "metrics", "register", "applicability.jsonl")
        with open(path, "a") as fh:
            fh.write("{not json\n")
        self._commit_all("bad line")
        p, rep = self.history(at="HEAD")
        self.assertEqual(p.returncode, 1)
        self.assertIn("unreadable-record", self.names(rep))
        self.assertIn("applicability-record-removed", self.names(rep))

    def test_the_older_revision_chain_may_grow_in_the_middle(self):
        self.commit_base()

        def mutate(decisions, appl):  # a newly found earlier revision is inserted before the existing ones
            decisions[0]["revisions"].insert(0, entry(self.r1, lines="1", document="docs/a.md") | {"revision": "git:0123456789"})
        _, rep = self.history(at=self.later(mutate))
        self.assertEqual(self.names(rep), [])


class HistoryCommandTests(HistoryFixture):
    def test_exit_codes_and_errors(self):
        self.commit_base()
        p, _ = self.history(base=False)
        self.assertEqual(p.returncode, 2, p.stderr)  # --base is required
        p, rep = self.history(base="no-such-ref")
        self.assertEqual((p.returncode, p.stdout), (4, ""))
        self.assertIn("does not resolve", p.stderr)
        p, _ = self.history(at="no-such-ref")
        self.assertEqual((p.returncode, p.stdout), (4, ""))
        p, _ = self.history(base=self.r1)  # a commit before the register existed
        self.assertEqual((p.returncode, p.stdout), (4, ""))
        self.assertIn("not found", p.stderr)
        p, _ = self.history(extra=["--register", self.base])  # outside the repository
        self.assertEqual((p.returncode, p.stdout), (2, ""))
        p = run_script(SCRIPT, "history", "--repo", self.repo, "--base", self.base_commit, "--bogus")
        self.assertEqual(p.returncode, 2)
        p, rep = self.history()
        self.assertEqual(p.returncode, 0)
        self.assertEqual(sorted(rep), ["authority", "base", "later", "limitations", "owner_review", "schema", "statement",
                                       "summary", "tool", "violations"])
        self.assertEqual(p.stdout, json.dumps(rep, sort_keys=True, indent=1, ensure_ascii=False) + "\n")
        self.assertEqual(rep["tool"]["register_py_sha256"], sha(ta.slurp(SCRIPT)))

    def test_validate_does_not_run_history(self):
        body = ta.slurp(SCRIPT).decode("utf-8").split("def cmd_validate")[1].split("def k_sort")[0]
        self.assertNotIn("history", body)


def strace_writes(test, argv):
    """Run argv under strace; return the write-mode file operations other than on /dev/null, and the process."""
    if shutil.which("strace") is None:
        test.skipTest("not established: strace is not installed")
    log = os.path.join(test.base, "strace.log")
    trace = ("openat,open,creat,mkdir,mkdirat,rename,renameat,renameat2,unlink,unlinkat,rmdir,truncate,ftruncate,"
             "symlink,symlinkat,link,linkat,chmod,fchmodat,chown,fchownat,utimensat,setxattr")
    p = subprocess.run(["strace", "-f", "-qq", "-o", log, "-e", f"trace={trace}", "--", *argv],
                       capture_output=True, text=True, check=False)
    if not os.path.exists(log) or os.path.getsize(log) == 0:
        test.skipTest(f"not established: strace produced no trace (rc {p.returncode}: {p.stderr[:200]})")
    writes, opens = [], 0
    with open(log, encoding="utf-8", errors="replace") as fh:
        for line in fh:
            body = re.sub(r"^\d+\s+", "", line.strip())
            if "= -1" in body or body.startswith(("---", "+++")):
                continue
            name = body.split("(", 1)[0]
            m = re.search(r'"([^"]*)"', body)
            path = m.group(1) if m else "?"
            if name in ("openat", "open"):
                opens += 1
                if not re.search(r"O_WRONLY|O_RDWR|O_CREAT|O_TRUNC|O_APPEND", body):
                    continue
            writes.append((path, body))
    test.assertGreater(opens, 0, "the trace shows no file opens, so it cannot show the absence of writes")
    return [w for w in writes if w[0] != "/dev/null"], p


class NoWriteTests(HistoryFixture):
    def tree_state(self):
        out = {}
        for base, dirs, files in os.walk(self.repo):
            dirs.sort()
            for d in dirs:
                out[os.path.relpath(os.path.join(base, d), self.repo) + "/"] = None
            for f in sorted(files):
                p = os.path.join(base, f)
                st = os.lstat(p)
                out[os.path.relpath(p, self.repo)] = (st.st_size, st.st_mtime_ns,
                                                      None if os.path.islink(p) else sha(ta.slurp(p)))
        return out

    def test_h14_history_writes_nothing(self):
        self.commit_base()
        ignored = os.path.join(self.repo, ".gitignore")
        with open(ignored, "w") as fh:
            fh.write("ignored.txt\n")
        with open(os.path.join(self.repo, "ignored.txt"), "w") as fh:
            fh.write("x")
        self._commit_all("ignore file")
        for base, _, files in os.walk(self.repo):
            if ".git" in base.split(os.sep):
                continue
            for f in files:
                os.utime(os.path.join(base, f), (1_000_000_000, 1_000_000_000))  # stale stat data: what a refresh rewrites
        before = self.tree_state()
        argv = [sys.executable, "-I", "-B", SCRIPT, "history", "--repo", self.repo, "--base", self.base_commit,
                "--at", "HEAD"]
        status_before = ta.git(self.repo, "--no-optional-locks", "status", "--short", "--ignored")
        writes, p = strace_writes(self, argv)
        self.assertEqual(p.returncode, 0, p.stderr)
        self.assertEqual(writes, [], writes)
        self.assertEqual(self.tree_state(), before)
        # the working-tree variant (no --at) writes nothing either, by strace and by tree state
        disk_argv = [sys.executable, "-I", "-B", SCRIPT, "history", "--repo", self.repo, "--base", self.base_commit]
        writes, p = strace_writes(self, disk_argv)
        self.assertEqual(p.returncode, 0, p.stderr)
        self.assertEqual(writes, [], writes)
        self.assertEqual(self.tree_state(), before)
        p = run_script(SCRIPT, "history", "--repo", self.repo, "--base", self.base_commit)
        self.assertEqual(p.returncode, 0, p.stderr)
        self.assertEqual(self.tree_state(), before)
        self.assertEqual(ta.git(self.repo, "--no-optional-locks", "status", "--short", "--ignored"), status_before)


class ValidateNoWriteTests(SchemaFixture):
    def test_h14_validate_writes_nothing(self):
        root = self.make(lambda data: self.dec(data).update({"revisions": [entry(self.c1)]}))
        before = ta.digest_tree(root), ta.digest_tree(self.repo)
        writes, p = strace_writes(self, [sys.executable, "-I", "-B", SCRIPT, "validate", "--register", root,
                                         "--repo", self.repo])
        self.assertEqual(p.returncode, 0, p.stdout + p.stderr)
        self.assertEqual(writes, [], writes)
        self.assertEqual((ta.digest_tree(root), ta.digest_tree(self.repo)), before)


class MovedAnchorTests(SchemaFixture):
    """H13: anchored text that moves (A1 class `moved`) needs no revision change, on a complete register."""

    def test_h13_moved_anchored_text_needs_no_revision_change(self):
        root = self.make(lambda data: self.dec(data).update({"lines": "2"}))
        reg_dir = os.path.join(self.repo, "metrics", "register")
        shutil.copytree(root, reg_dir)
        tr.git(self.repo, "add", "-A")
        tr.git(self.repo, "commit", "-qm", "complete register")
        base = tr.git(self.repo, "rev-parse", "HEAD")
        # a commit that only moves the anchored text: lines are prepended to the document
        path = os.path.join(self.repo, "docs", "model.md")
        with open(path) as fh:
            text = fh.read()
        with open(path, "w") as fh:
            fh.write("# New first\n\nnew line\n" + text)
        tr.git(self.repo, "commit", "-qam", "doc moves")
        later = tr.git(self.repo, "rev-parse", "HEAD")
        # validate passes on the complete register at the later commit
        v = run_script(SCRIPT, "validate", "--repo", self.repo)
        self.assertEqual(v.returncode, 0, v.stdout + v.stderr)
        self.assertTrue(v.stdout.startswith("validate: ok"), v.stdout)
        # history passes with no new revision entry: the register files are unchanged between the two commits
        self.assertEqual(tr.git(self.repo, "diff", "--name-only", base, later, "--", "metrics/register"), "")
        p = run_script(SCRIPT, "history", "--repo", self.repo, "--base", base, "--at", later)
        self.assertEqual(p.returncode, 0, p.stdout + p.stderr)
        rep = json.loads(p.stdout)
        self.assertEqual(rep["violations"], [])
        self.assertEqual(rep["owner_review"], [])
        decisions = json.loads(tr.git(self.repo, "show", f"{later}:metrics/register/decisions.json"))
        self.assertNotIn("revisions", self.dec({"decisions": decisions}))
        # the anchored text did move: class exactly `moved`, not a revision event
        out = os.path.join(self.base, "moved-anchors")
        a = run_script(SCRIPT, "anchors", "--repo", self.repo, "--at", later, "--out", out)
        self.assertEqual(a.returncode, 0, a.stdout + a.stderr)
        with open(os.path.join(out, "anchors.json"), encoding="utf-8") as fh:
            report = json.load(fh)
        entry_ = next(e for e in report["decisions"] if e["decision_id"] == "d-model-new")
        self.assertEqual(entry_["class"], "moved")


class EnvironmentIndependenceTests(HistoryFixture):
    def test_h15_history_is_pinned_and_environment_independent(self):
        self.commit_base()

        def mutate(decisions, appl):
            appl[0]["rationale"] = "different"
            appl[1]["authorization"]["status"] = "pending"
        later = self.later(mutate)
        p0, rep0 = self.history(at=later)
        self.assertEqual(p0.returncode, 1)
        clean = p0.stdout
        p1, rep1 = self.history()  # no --at: the later state is the register files on disk, which are the later commit's
        self.assertEqual(p1.returncode, 1)
        clean_disk = p1.stdout
        other = os.path.join(self.base, "other")
        os.makedirs(other)
        ta.git(other, "init", "-q")
        ta.git(other, "commit", "--allow-empty", "-q", "-m", "elsewhere")
        cfg = os.path.join(self.base, "hostile.cfg")
        with open(cfg, "w") as fh:
            fh.write("[core]\n\tworktree = /nonexistent\n\tabbrev = 4\n\tquotepath = true\n[diff]\n\tcontext = 9\n"
                     "\talgorithm = histogram\n\trenames = copies\n[log]\n\tshowSignature = true\n"
                     "[alias]\n\trev-parse = !false\n\tcat-file = !false\n\tmerge-base = !false\n")
        envs = {
            "GIT_DIR/GIT_WORK_TREE": {"GIT_DIR": os.path.join(other, ".git"), "GIT_WORK_TREE": other},
            "GIT_CONFIG_COUNT": {"GIT_CONFIG_COUNT": "2", "GIT_CONFIG_KEY_0": "core.abbrev", "GIT_CONFIG_VALUE_0": "4",
                                 "GIT_CONFIG_KEY_1": "alias.cat-file", "GIT_CONFIG_VALUE_1": "!false"},
            "GIT_CONFIG_GLOBAL": {"GIT_CONFIG_GLOBAL": cfg},
            "GIT_CONFIG_SYSTEM": {"GIT_CONFIG_SYSTEM": cfg, "GIT_CONFIG_NOSYSTEM": "0"},
            "GIT_OBJECT_DIRECTORY": {"GIT_OBJECT_DIRECTORY": os.path.join(other, ".git", "objects")},
            "GIT_NAMESPACE/GIT_PREFIX": {"GIT_NAMESPACE": "x", "GIT_PREFIX": "docs/"},
        }
        for label, env in envs.items():
            with self.subTest(label):
                self.assertEqual(self.history(at=later, env=env)[0].stdout, clean)
                self.assertEqual(self.history(env=env)[0].stdout, clean_disk)  # the no --at path
        with self.subTest("replace refs"):
            blob = ta.git(self.repo, "rev-parse", f"{self.base_commit}:metrics/register/decisions.json")
            alt = os.path.join(self.base, "alt.json")
            with open(alt, "w") as fh:
                fh.write("[]")
            alt_blob = ta.git(self.repo, "hash-object", "-w", alt)
            ta.git(self.repo, "replace", blob, alt_blob)
            self.assertEqual(self.history(at=later)[0].stdout, clean)
            self.assertEqual(self.history()[0].stdout, clean_disk)
            # the digest recorded for the base input is the real blob's, not the replacement's
            self.assertEqual(rep0["base"]["inputs"]["decisions.json"]["sha256"],
                             sha(ta.git(self.repo, "--no-replace-objects", "cat-file", "blob", blob, raw=True)))


# ---------------------------------------------------------------------------
# byte identity, S0 preservation, frozen census (H1, H16, H17)
# ---------------------------------------------------------------------------

def show(commit, path):
    return ta.git(REAL_REPO, "show", f"{commit}:{path}", raw=True)


def have(commit):
    return subprocess.run(["git", "-C", REAL_REPO, "cat-file", "-e", f"{commit}^{{commit}}"],
                          capture_output=True).returncode == 0


# A3 S2: the two decisions the user approved marking historical (user approval 2026-11-09T06:39:27Z), with
# exactly the approved marker values. superseded_by names the verified current counterpart on the df288c3d pin only.
S2_MARKED_BY = "user (register and decision owner), per the user approval of 2026-11-09T06:39:27Z"
S2_COUNTERPART = "model-selection.claude-host-builder-rows"
S2_MARKERS = {
    "model-selection.claude-host-builder-rows-at-df288c3d": {
        "marked_at": "2026-11-09", "marked_by": S2_MARKED_BY, "superseded_by": S2_COUNTERPART,
        "reason": ("Deliberate pin of the superseded revision of the Claude-host builder rows, the one that "
                   "cited full-battery counts before the traceability correction reached the document. Kept as "
                   "its own decision record so a correction reaches the revision that relied on the corrected "
                   "reading, not only the current one")},
    "dogfood-plan.round2-status-line": {
        "marked_at": "2026-11-09", "marked_by": S2_MARKED_BY,
        "reason": ("The later pause notice (metrics/dogfood-selection-plan-20261027.md, 'Current status "
                   "(2026-11-07)', round 2 is paused) supersedes this status line as a statement of current "
                   "operational status. The original text and revision are preserved")},
}
S2_HISTORICAL_MARKED = frozenset(S2_MARKERS)

# A3 S3 (user decisions O-1, O-2 and O-5 of 2026-11-09): the two modified current decisions move to the last commits
# that changed their anchored text and keep their Stage 0 revision as their only `revisions` entry. superseded_at is
# the date of the superseding source commit (1906090c, 5149cc27), not of the migration. O-3 was not decided, so S3 adds
# no applicability record; the records citing the Stage 0 revisions stay there and no authorization is carried.
S3_BASE = "7e946c8c3bbb04e1b3a001938cbe27f9e2044d1d"
S3_DATA = "60f364787f2952adcd24918b54d82f50f28afeb5"  # the S3 data commit (decisions.json and README only)
S3_HOST, S3_QUOTA = "model-selection.claude-host-builder-rows", "model-selection.cost-table-quota-row"
S3_NEW = {
    S3_HOST: {"revision": "git:1906090c", "lines": "128-129",
              "anchor": ("Builder rows of the Claude host section (Sonnet 5.5 high; Opus 5.5 high), citing traced "
                         "battery counts only")},
    S3_QUOTA: {"revision": "git:5149cc27", "lines": "268",
               "anchor": ("Cost table row: Codex quota tracks API cost, Astra about 4.5x Sol, $6-7 per 1% of a Pro "
                          "week, marked not established for current model-selection use (owner, 2026-11-07)")},
}
S3_REVISIONS = {
    S3_HOST: [{
        "revision": "git:4ff69ffd", "lines": "39-40", "document": "docs/model-selection.md",
        "anchor": "Builder on the Claude host rows (Sonnet 5.5 high; Opus 5.5 high), citing traced battery counts",
        "superseded_at": "2026-11-07",
        "reason": ("1906090c moved the Claude-host builder rows into the section 'Claude host and Claude-only "
                   "development' and rewrote them there: the traced E1a counts stay (Sonnet 2, 0 and 0 of 12; Opus 0 "
                   "of 12), the full-battery counts are gone, and the Sonnet row now cites S2 probe lines 467-579 and "
                   "R1/R2 use. From 1906090c on, lines 39-40 are the Current defaults table header, not these rows")}],
    S3_QUOTA: [{
        "revision": "git:1a1cb443", "lines": "115", "document": "docs/model-selection.md",
        "anchor": "Cost table row: Codex quota tracks API cost, Astra about 4.5x Sol, $6-7 per 1% of a Pro week",
        "superseded_at": "2026-11-07",
        "reason": ("5149cc27 added the owner's qualification to the row: both claims are not established for current "
                   "model-selection use until the experiment plan section 1 fit is rechecked with the tier (F1.1); the "
                   "values and source are kept as the historical record. The applicability record authorized on "
                   "2026-11-07 stays on 1a1cb443 and is not carried to 5149cc27")}],
}
S3_PRIOR_RECORDS = {"ap-k2-traced-claude-host-rows": S3_HOST, "ap-k3-quota-fit-cost-table-quota-row": S3_QUOTA}
S3_README_OLD = "`document`) is appended unchanged as the last `revisions` entry; text that only moves needs no\nnew revision."
S3_README_NEW = (S3_README_OLD + " A3 S3 moved `model-selection.claude-host-builder-rows` to `1906090c` and\n"
                 "`model-selection.cost-table-quota-row` to `5149cc27`, the last commits that changed their anchored\n"
                 "text; their Stage 0 revisions are listed in `revisions`, and the records citing those stay there.")


class RealRegisterTests(unittest.TestCase):
    def setUp(self):
        self.base = tempfile.mkdtemp(prefix="history-real-")
        self.addCleanup(shutil.rmtree, self.base, ignore_errors=True)
        for c in (S1_BASE, S0_STUDY_COMMIT, S0_PIN, A1_CENSUS_COMMIT):
            self.assertTrue(have(c), f"commit {c} is not in this clone; the real-register tests cannot run")
        self.old_script = os.path.join(self.base, "register_s1_base.py")
        with open(self.old_script, "wb") as fh:
            fh.write(show(S1_BASE, "metrics/scripts/register.py"))
        self.old_sha, self.new_sha = sha(ta.slurp(self.old_script)), sha(ta.slurp(SCRIPT))
        self.assertNotEqual(self.old_sha, self.new_sha)

    def extract_register(self, commit):
        data = ta.git(REAL_REPO, "archive", "--format=tar", commit, "metrics/register", raw=True)
        root = os.path.join(self.base, f"reg-{commit[:8]}")
        os.makedirs(root)
        with tarfile.open(fileobj=io.BytesIO(data)) as tf:
            tf.extractall(root, filter="data")
        return os.path.join(root, "metrics", "register")

    def norm(self, text, out=None):
        """The report embeds the digest of the script that wrote it; the candidate's differs by construction."""
        if out:
            text = text.replace(out, "OUT")
        return text.replace(self.new_sha, self.old_sha)

    def test_h1_validate_build_nominate_and_anchors_are_byte_identical_to_the_base(self):
        for commit in (S0_PIN, A1_CENSUS_COMMIT, S1_BASE):
            register = self.extract_register(commit)
            with self.subTest(commit=commit, command="validate"):
                a = run_script(self.old_script, "validate", "--repo", REAL_REPO, "--register", register)
                b = run_script(SCRIPT, "validate", "--repo", REAL_REPO, "--register", register)
                self.assertEqual((a.returncode, a.stdout, a.stderr), (b.returncode, b.stdout, b.stderr))
                self.assertIn("validate: ok", b.stdout)
            with self.subTest(commit=commit, command="build"):
                outs = []
                for tag, script in (("old", self.old_script), ("new", SCRIPT)):
                    out = os.path.join(self.base, f"build-{commit[:8]}-{tag}")
                    p = run_script(script, "build", "--repo", REAL_REPO, "--register", register, "--out", out)
                    self.assertEqual(p.returncode, 0, p.stdout + p.stderr)
                    outs.append((p.stdout.replace(out, "OUT"), p.stderr, out))
                self.assertEqual(outs[0][:2], outs[1][:2])
                for name in ("current-knowledge.json", "current-knowledge.md"):
                    self.assertEqual(ta.slurp(os.path.join(outs[0][2], name)), ta.slurp(os.path.join(outs[1][2], name)))
            with self.subTest(commit=commit, command="nominate"):
                outs = []
                for tag, script in (("old", self.old_script), ("new", SCRIPT)):
                    out = os.path.join(self.base, f"nom-{commit[:8]}-{tag}.json")
                    p = run_script(script, "nominate", "--repo", REAL_REPO, "--register", register, "--out", out)
                    self.assertEqual(p.returncode, 0, p.stdout + p.stderr)
                    outs.append((p.stdout.replace(out, "OUT"), p.stderr, ta.slurp(out)))
                self.assertEqual(outs[0], outs[1])
            with self.subTest(commit=commit, command="anchors"):
                outs = []
                for tag, script in (("old", self.old_script), ("new", SCRIPT)):
                    out = os.path.join(self.base, f"anc-{commit[:8]}-{tag}")
                    p = run_script(script, "anchors", "--repo", REAL_REPO, "--at", commit, "--out", out)
                    self.assertEqual(p.returncode, 0, p.stdout + p.stderr)
                    outs.append((p.stdout.replace(out, "OUT"), p.stderr, ta.slurp(os.path.join(out, "anchors.json")).decode(),
                                 ta.slurp(os.path.join(out, "anchors.md")).decode()))
                self.assertIn(self.new_sha, outs[1][2])
                self.assertIn(self.new_sha, outs[1][3])
                self.assertEqual(outs[0], (outs[1][0], outs[1][1], self.norm(outs[1][2]), self.norm(outs[1][3])))
                if commit == A1_CENSUS_COMMIT:  # A1's r2 qualification output, modulo the script digest it embeds
                    self.assertEqual(sha(self.norm(outs[1][2]).encode()), A1_R2_ANCHORS_JSON_SHA)
                    self.assertEqual(sha(self.norm(outs[1][3]).encode()), A1_R2_ANCHORS_MD_SHA)

    def test_h1_synthetic_registers_with_every_class_are_byte_identical(self):
        outs = []
        fx = AnchorViewFixture()
        fx.setUp()
        self.addCleanup(shutil.rmtree, fx.base, ignore_errors=True)
        rev = fx.commit_files({"docs/a.md": ta.DOC}, "v1")
        fx.commit_files({"docs/a.md": ta.DOC.replace("gamma", "G").replace("# Title\n\n", "# Title\n\nx\n")}, "v2")
        fx.commit_register([fx.decision("a", "docs/a.md", rev, "3"), fx.decision("b", "docs/a.md", rev, "4-6"),
                            fx.decision("c", "docs/a.md", rev, "8"), fx.decision("d", "docs/missing.md", rev),
                            fx.decision("model-selection.claude-host-builder-rows-at-df288c3d", "docs/a.md", rev, "4-5")],
                           [fx.appl("ap1", "b", rev, "authorized"), fx.appl("ap2", "a", rev), fx.appl("ap3", "c", rev)])
        for tag, script in (("old", self.old_script), ("new", SCRIPT)):
            out = os.path.join(self.base, f"syn-{tag}")
            p = run_script(script, "anchors", "--repo", fx.repo, "--out", out)
            outs.append((p.returncode, p.stdout.replace(out, "OUT"), p.stderr,
                         ta.slurp(os.path.join(out, "anchors.json")).decode(), ta.slurp(os.path.join(out, "anchors.md")).decode()))
        self.assertEqual(outs[0][:3], outs[1][:3])
        self.assertEqual(outs[0][3], self.norm(outs[1][3]))
        self.assertEqual(outs[0][4], self.norm(outs[1][4]))

    def test_h16_s0_preservation(self):
        # (a) the S0 study file at the S1 base is byte-identical in the candidate
        base_study = show(S1_BASE, S0_STUDY)
        cand_study = ta.slurp(os.path.join(REAL_REPO, S0_STUDY))
        self.assertEqual(cand_study, base_study)
        # (b) every line of the 9fdf4b7e version is still present, in order (the base inserted one finding)
        old_lines = show(S0_STUDY_COMMIT, S0_STUDY).split(b"\n")
        new_lines = cand_study.split(b"\n")
        it = iter(new_lines)
        self.assertTrue(all(any(x == y for y in it) for x in old_lines), "9fdf4b7e lines are not a subsequence")
        self.assertEqual(len(new_lines) - len(old_lines), 96)  # pure insertion of the one finding
        self.assertIn(b"routing-rescored-r3", cand_study)
        self.assertNotIn(b"routing-rescored-r3", show(S0_STUDY_COMMIT, S0_STUDY))
        # (c) every decision id and (id, revision) pair at 97f6486f and 14031bc2 resolves in the candidate
        with open(os.path.join(REAL_REPO, "metrics", "register", "decisions.json"), encoding="utf-8") as fh:
            cand = {d["decision_id"]: d for d in json.load(fh)}
        for commit in (S0_PIN, A1_CENSUS_COMMIT):
            for d in json.loads(show(commit, "metrics/register/decisions.json")):
                with self.subTest(commit=commit, decision=d["decision_id"]):
                    self.assertIn(d["decision_id"], cand)
                    listed = {r["revision"] for r in cand[d["decision_id"]].get("revisions", [])}
                    self.assertIn(d["revision"], {cand[d["decision_id"]]["revision"]} | listed)

    def test_h16_history_from_the_s0_pins_to_the_candidate_passes(self):
        for commit in (S0_PIN, A1_CENSUS_COMMIT):
            with self.subTest(commit=commit):
                p = run_script(SCRIPT, "history", "--repo", REAL_REPO, "--base", commit, "--at", S1_BASE)
                self.assertEqual(p.returncode, 0, p.stdout + p.stderr)
                self.assertEqual(json.loads(p.stdout)["violations"], [])

    def test_h17_frozen_census_at_the_s1_base_matches_11_of_11(self):
        raw = ta.slurp(FROZEN_CENSUS)
        self.assertEqual(sha(raw), FROZEN_CENSUS_SHA)
        census = json.loads(raw)
        groups = census["group_definitions"]
        out = os.path.join(self.base, "h17")
        p = run_script(SCRIPT, "anchors", "--repo", REAL_REPO, "--at", S1_BASE, "--out", out)
        self.assertEqual(p.returncode, 0, p.stdout + p.stderr)
        got = {e["decision_id"]: e for e in json.loads(ta.slurp(os.path.join(out, "anchors.json")))["decisions"]}
        self.assertEqual(len(census["decisions"]), 11)
        self.assertEqual(sorted(got), sorted(d["decision_id"] for d in census["decisions"]))
        matched = 0
        for d in census["decisions"]:
            self.assertIsNone(got[d["decision_id"]]["error"], d["decision_id"])
            if got[d["decision_id"]]["class"] in groups[d["group"]]:
                matched += 1
            else:
                self.fail(f"{d['decision_id']}: {got[d['decision_id']]['class']} not in {d['group']}")
        self.assertEqual(matched, 11)

    def test_the_real_register_is_untouched_by_validation_of_the_new_keys(self):
        p = run_script(SCRIPT, "validate", "--repo", REAL_REPO)
        self.assertEqual(p.returncode, 0, p.stdout)
        with open(os.path.join(REAL_REPO, "metrics", "register", "decisions.json"), encoding="utf-8") as fh:
            # A3 S3: `revisions` on exactly the two re-anchored decisions, with the approved values; `historical` on
            # exactly the S2 pair (exact values: S2MarkersInTheCurrentRegisterTests); no `evidence_state`
            for d in json.load(fh):
                did = d["decision_id"]
                self.assertNotIn("evidence_state", d, did)
                self.assertEqual(d.get("revisions"), S3_REVISIONS.get(did), did)
                self.assertEqual("historical" in d, did in S2_HISTORICAL_MARKED, did)


# ---------------------------------------------------------------------------
# TR-1 and SR-1: `build` shows a prior-revision row's own historical source (R1-R8)
# ---------------------------------------------------------------------------

TR1_BASE = "9ac9826106d6afd303de384ddf86a6043fa11981"
TR1_REAL_COMMITS = (S0_PIN, A1_CENSUS_COMMIT, S1_BASE, TR1_BASE)
HISTORICAL_LABEL_MD = "historical source location, not a current operational location or authority"
HISTORICAL_LABEL_JSON = "historical source location; not a current operational location or authority"


def dumps_row(row):
    return json.dumps(row, sort_keys=True)


class HistoricalSourceBuildTests(SchemaFixture):
    """R1-R8. A row at a listed prior revision shows the revision entry's own document, anchor and lines, labelled
    as historical, never the decision's current location; everything else in `build`, and every other command, is
    byte-identical to the script at 9ac98261."""

    CUR_LINES = "2-3"          # the decision's current lines (docs/model.md has 3 lines at the current revision)
    PRIOR_ANCHOR = "anchor as it stood at the prior revision"

    def setUp(self):
        self.out_base = tempfile.mkdtemp(prefix="tr1-", dir=self.base)

    def build(self, mutate, script=None):
        return self.build_root(self.make(mutate), script)

    def build_root(self, root, script=None):
        out = tempfile.mkdtemp(prefix="build-", dir=self.out_base)
        p = run_script(script or SCRIPT, "build", "--register", root, "--repo", self.repo, "--out", out)
        self.assertEqual(p.returncode, 0, p.stdout + p.stderr)
        return (p.stdout.replace(out, "OUT"), ta.slurp(os.path.join(out, "current-knowledge.json")).decode(),
                ta.slurp(os.path.join(out, "current-knowledge.md")).decode())

    @staticmethod
    def rows(text):
        return {a["applicability_id"]: a for f in json.loads(text)["findings"].values() for a in f["applicability"]}

    @staticmethod
    def md_row(md, aid):
        return next(line for line in md.splitlines() if f"| {aid}" in line)

    def prior_mutate(self, **entry_keys):
        """Decision d-model-new with current lines, one prior revision entry, and a record citing that entry."""
        def mutate(data):
            d = self.dec(data)
            d["lines"] = self.CUR_LINES
            d["revisions"] = [entry(self.c1, **entry_keys)]
            a = copy.deepcopy(data["applicability"][0])
            a["applicability_id"] = "ap-prior"
            a["decision_revision"] = f"git:{self.c1}"
            a["authorization"] = {"status": "authorized", "owner": "user (fixture owner)", "at": "2026-01-02"}
            data["applicability"].append(a)
        return mutate

    def full_c1(self):
        return tr.git(self.repo, "rev-parse", self.c1)

    def assert_no_current_location(self, row, line_md):
        for text in (json.dumps(row), line_md):
            self.assertNotIn("docs/model.md", text)
            self.assertNotIn("anchor d-model-new", text)
            self.assertNotIn(self.CUR_LINES, text)
        for k in ("document", "anchor", "lines"):
            self.assertNotIn(k, row)

    def test_r1_prior_row_shows_the_entrys_own_location_and_no_current_one(self):
        mutate = self.prior_mutate(document="docs/study.md", anchor=self.PRIOR_ANCHOR, lines="1-2")
        _, js, md = self.build(mutate)
        row = self.rows(js)["ap-prior"]
        self.assertEqual(row["historical_source"], {
            "revision": f"git:{self.c1}", "commit": self.full_c1(), "document": "docs/study.md",
            "document_basis": "entry", "anchor": self.PRIOR_ANCHOR, "lines": "1-2",
            "label": HISTORICAL_LABEL_JSON, "status": "resolved"})
        self.assertEqual(len(row["historical_source"]["commit"]), 40)
        self.assertIs(row["at_prior_revision"], True)
        self.assertEqual(row["current_decision_revision"], f"git:{self.c2}")
        self.assertEqual(row["authorization"], "authorized")
        line = self.md_row(md, "ap-prior")
        self.assert_no_current_location(row, line)
        self.assertIn(HISTORICAL_LABEL_MD, line)
        self.assertIn(f"docs/study.md ({self.PRIOR_ANCHOR}), lines 1-2 at git:{self.c1} ({self.full_c1()})", line)
        self.assertLess(line.index(HISTORICAL_LABEL_MD), line.index("docs/study.md"))
        self.assertLess(line.index(self.full_c1()), line.index("authorization: authorized"))
        self.assertLess(line.index("authorization: authorized"), line.index("ap-prior"))
        self.assertLess(line.index("ap-prior"), line.index("not carried"))
        self.assertNotIn("schema default", line)
        # the current-revision row of the same decision keeps its current location
        self.assertEqual(self.rows(js)["ap-one"]["document"], "docs/model.md")

    def test_r2_an_entry_without_lines_gives_null_and_says_so(self):
        _, js, md = self.build(self.prior_mutate(document="docs/study.md", anchor=self.PRIOR_ANCHOR))
        row = self.rows(js)["ap-prior"]
        self.assertIsNone(row["historical_source"]["lines"])
        line = self.md_row(md, "ap-prior")
        self.assertIn("no line range recorded in the revision entry", line)
        self.assert_no_current_location(row, line)

    def test_r3_an_entry_without_document_uses_the_schema_default_and_says_so(self):
        _, js, md = self.build(self.prior_mutate(anchor=self.PRIOR_ANCHOR, lines="1-2"))
        row = self.rows(js)["ap-prior"]
        h = row["historical_source"]
        self.assertEqual((h["document"], h["document_basis"], h["anchor"], h["lines"]),
                         ("docs/model.md", "schema_default", self.PRIOR_ANCHOR, "1-2"))
        self.assertEqual(h["status"], "resolved")
        self.assertNotIn("document", row)
        line = self.md_row(md, "ap-prior")
        self.assertIn(f"(document path by schema default: the decision's path, read at git:{self.c1})", line)
        self.assertIn("lines 1-2", line)
        self.assertNotIn("anchor d-model-new", line)
        self.assertNotIn(self.CUR_LINES, line)

    def old_script_path(self):
        path = os.path.join(self.out_base, "register_9ac98261.py")
        if not os.path.exists(path):
            with open(path, "wb") as fh:
                fh.write(show(TR1_BASE, "metrics/scripts/register.py"))
        return path

    def test_r4_current_revision_rows_and_a_historical_decision_are_byte_identical_to_9ac98261(self):
        self.assertTrue(have(TR1_BASE))
        old = self.old_script_path()
        root = self.make(self.prior_mutate(document="docs/study.md", anchor=self.PRIOR_ANCHOR, lines="1-2"))
        o_out, o_js, o_md = self.build_root(root, old)
        n_out, n_js, n_md = self.build_root(root)
        self.assertEqual(o_out, n_out)
        o_rows, n_rows = self.rows(o_js), self.rows(n_js)
        self.assertEqual(set(o_rows), set(n_rows))
        for aid in o_rows:
            if aid != "ap-prior":
                self.assertEqual(dumps_row(o_rows[aid]), dumps_row(n_rows[aid]), aid)
                self.assertEqual(self.md_row(o_md, aid), self.md_row(n_md, aid), aid)

        def without_prior(md):
            return [ln for ln in md.splitlines() if "ap-prior" not in ln]
        self.assertEqual(without_prior(o_md), without_prior(n_md))
        o_doc, n_doc = json.loads(o_js), json.loads(n_js)
        for doc in (o_doc, n_doc):
            for f in doc["findings"].values():
                f["applicability"] = [a for a in f["applicability"] if a["applicability_id"] != "ap-prior"]
        self.assertEqual(o_doc, n_doc)

        def historical(data):
            self.dec(data)["historical"] = dict(HistoricalMarkerTests.MARK)
            data["applicability"][0]["authorization"] = {"status": "authorized", "owner": "user (fixture owner)",
                                                         "at": "2026-01-02"}
        root = self.make(historical)
        self.assertEqual(self.build_root(root, old), self.build_root(root))

    # --- byte identity for registers without revisions (R5) and the other commands (R6) ----------------------

    @staticmethod
    def norm(text, old_sha, new_sha):
        """Only for outputs that embed the writing script's digest: normalise that value and nothing else."""
        return text.replace(new_sha, old_sha)

    def run_commands(self, script, repo, register, anchors_at, history_args, tag):
        """validate, build (both files), nominate, anchors and history; returns {name: comparable}."""
        res = {}
        if register:
            p = run_script(script, "validate", "--repo", repo, "--register", register)
            res["validate"] = (p.returncode, p.stdout, p.stderr)
            out = os.path.join(self.out_base, f"b-{tag}")
            p = run_script(script, "build", "--repo", repo, "--register", register, "--out", out)
            ok = p.returncode == 0
            res["build"] = (p.returncode, p.stdout.replace(out, "OUT"), p.stderr,
                            ta.slurp(os.path.join(out, "current-knowledge.json")).decode() if ok else None,
                            ta.slurp(os.path.join(out, "current-knowledge.md")).decode() if ok else None)
            out = os.path.join(self.out_base, f"n-{tag}.json")
            p = run_script(script, "nominate", "--repo", repo, "--register", register, "--out", out)
            res["nominate"] = (p.returncode, p.stdout.replace(out, "OUT"), p.stderr,
                               ta.slurp(out).decode() if p.returncode == 0 else None)
        out = os.path.join(self.out_base, f"a-{tag}")
        p = run_script(script, "anchors", "--repo", repo, f"--at={anchors_at}", "--out", out)
        files = [ta.slurp(os.path.join(out, f)).decode() if os.path.exists(os.path.join(out, f)) else None
                 for f in ("anchors.json", "anchors.md")]
        res["anchors"] = (p.returncode, p.stdout.replace(out, "OUT"), p.stderr, *files)
        p = run_script(script, "history", "--repo", repo, *history_args)
        res["history"] = (p.returncode, p.stdout, p.stderr)
        return res

    def compare(self, a, b, name, old_sha, new_sha, label):
        """Strict equality; the digest exception only for anchors and history, which embed the script's digest."""
        if name not in ("anchors", "history"):
            self.assertEqual(a, b, f"{label} {name}")
            return
        old_text, new_text = json.dumps(a), json.dumps(b)
        if old_sha in old_text or new_sha in new_text:  # an output that actually embeds the digest
            self.assertIn(old_sha, old_text, f"{label} {name}")
            self.assertIn(new_sha, new_text, f"{label} {name}")
            if old_sha != new_sha:  # equal only when the script under test is the 9ac98261 script itself
                self.assertNotIn(new_sha, old_text)
                self.assertNotIn(old_sha, new_text)
        b = tuple(self.norm(x, old_sha, new_sha) if isinstance(x, str) else x for x in b)
        self.assertEqual(a, b, f"{label} {name}")

    def assert_commands_identical(self, repo, register, anchors_at, history_args, label):
        old = self.old_script_path()
        old_sha, new_sha = sha(ta.slurp(old)), sha(ta.slurp(SCRIPT))
        a = self.run_commands(old, repo, register, anchors_at, history_args, f"{label}-old")
        b = self.run_commands(SCRIPT, repo, register, anchors_at, history_args, f"{label}-new")
        self.assertEqual(set(a), set(b))
        for name in a:
            with self.subTest(label=label, command=name):
                self.compare(a[name], b[name], name, old_sha, new_sha, label)
        return a

    def extract_real_register(self, commit):
        data = ta.git(REAL_REPO, "archive", "--format=tar", commit, "metrics/register", raw=True)
        root = os.path.join(self.out_base, f"reg-{commit[:8]}")
        os.makedirs(root)
        with tarfile.open(fileobj=io.BytesIO(data)) as tf:
            tf.extractall(root, filter="data")
        return os.path.join(root, "metrics", "register")

    def test_r5_registers_without_revisions_are_byte_identical_in_every_command(self):
        for c in TR1_REAL_COMMITS:
            self.assertTrue(have(c), f"commit {c} is not in this clone")
        for commit in TR1_REAL_COMMITS:
            register = self.extract_real_register(commit)
            data = ta.slurp(os.path.join(register, "decisions.json")).decode()
            self.assertNotIn('"revisions"', data, commit)  # the real register uses no revision history
            res = self.assert_commands_identical(REAL_REPO, register, commit, ["--base", S0_PIN, "--at", commit],
                                                 f"real-{commit[:8]}")
            self.assertEqual(res["validate"][0], 0, res["validate"])
            self.assertEqual(res["build"][0], 0)
        # the S1 H1 fixture states: a register of every anchor class, then a changed state
        fx = AnchorViewFixture()
        fx.setUp()
        self.addCleanup(shutil.rmtree, fx.base, ignore_errors=True)
        rev = fx.commit_files({"docs/a.md": ta.DOC}, "v1")
        fx.commit_files({"docs/a.md": ta.DOC.replace("gamma", "G").replace("# Title\n\n", "# Title\n\nx\n")}, "v2")
        decisions = [fx.decision("a", "docs/a.md", rev, "3"), fx.decision("b", "docs/a.md", rev, "4-6"),
                     fx.decision("c", "docs/a.md", rev, "8"), fx.decision("d", "docs/missing.md", rev),
                     fx.decision("model-selection.claude-host-builder-rows-at-df288c3d", "docs/a.md", rev, "4-5")]
        first = fx.commit_register(decisions, [fx.appl("ap1", "b", rev, "authorized"), fx.appl("ap2", "a", rev)])
        second = fx.commit_register(decisions, [fx.appl("ap1", "b", rev, "pending"), fx.appl("ap2", "a", rev)])
        res = self.assert_commands_identical(fx.repo, None, second, ["--base", first, "--at", second], "h1-fixture")
        self.assertIsNotNone(res["anchors"][3])  # the report was written, so the comparison is not vacuous
        # a complete synthetic register without revisions
        res = self.assert_commands_identical(self.repo, self.make(None), "HEAD", ["--base", "HEAD", "--at", "HEAD"],
                                             "schema-fixture")
        self.assertEqual(res["validate"][0], 0, res["validate"])

    def test_r6_the_other_commands_are_byte_identical_on_the_r1_to_r3_registers(self):
        reg_dir = os.path.join(self.repo, "metrics", "register")

        def commit_register(root):
            shutil.rmtree(reg_dir, ignore_errors=True)
            shutil.copytree(root, reg_dir)
            tr.git(self.repo, "add", "-A")
            tr.git(self.repo, "commit", "-qm", "register state")
            return tr.git(self.repo, "rev-parse", "HEAD")

        base_commit = commit_register(self.make(None))
        cases = {
            "r1": self.prior_mutate(document="docs/study.md", anchor=self.PRIOR_ANCHOR, lines="1-2"),
            "r2": self.prior_mutate(document="docs/study.md", anchor=self.PRIOR_ANCHOR),
            "r3": self.prior_mutate(anchor=self.PRIOR_ANCHOR, lines="1-2"),
        }
        old = self.old_script_path()
        old_sha, new_sha = sha(ta.slurp(old)), sha(ta.slurp(SCRIPT))
        for label, mutate in cases.items():
            root = self.make(mutate)
            commit = commit_register(root)
            args = ["--base", base_commit, "--at", commit]
            outs = {}
            for tag, script in (("old", old), ("new", SCRIPT)):
                res = self.run_commands(script, self.repo, root, commit, args, f"{label}-{tag}")
                outs[tag] = {k: v for k, v in res.items() if k != "build"}  # build is the command that changed
            self.assertEqual(set(outs["old"]), {"validate", "nominate", "anchors", "history"})
            self.assertEqual(outs["old"]["validate"][0], 0, outs["old"]["validate"])
            for name in outs["old"]:
                with self.subTest(label=label, command=name):
                    self.compare(outs["old"][name], outs["new"][name], name, old_sha, new_sha, label)

    def test_r7_readme_describes_build_by_what_it_emits(self):
        with open(os.path.join(REAL_REPO, "metrics", "register", "README.md"), encoding="utf-8") as fh:
            flat = " ".join(fh.read().split())
        self.assertNotIn("does not render decision state", flat)
        start = flat.index("- `build --out DIR`")
        bullet = flat[start:flat.index("- `nominate --out FILE")]
        for needle in ("historical source location", "historical_source", "not_established", "schema_default",
                       "decision revision", "authorization status", "not current operational locations or authority",
                       "`build` does not render a decision's `evidence_state`", "A register that uses neither"):
            self.assertIn(needle, bullet)
        self.assertIn("(`build` does not render `evidence_state`)", flat)

    def assert_unresolved(self, module, d, a, git, label):
        h = module.historical_source(d, a, git)
        self.assertEqual(h["status"], "not_established", label)
        self.assertEqual(h["revision"], a["decision_revision"], label)
        self.assertEqual(h["label"], HISTORICAL_LABEL_JSON, label)
        row = module.build_applicability_row(a, d, git) if module.at_prior_revision(d, a) else {"historical_source": h}
        for k in ("commit", "document", "anchor", "lines", "document_basis"):
            self.assertNotIn(k, h, label)
            self.assertNotIn(k, row, label)
        md = module.historical_source_md(h)
        self.assertIn("historical source: not_established", md)
        for text in (json.dumps(row), md):
            self.assertNotIn("docs/model.md", text, label)
            self.assertNotIn("anchor d-model-new", text, label)
            self.assertNotIn(self.CUR_LINES, text, label)

    def test_r8_an_unresolved_historical_source_is_reported_and_shows_no_location(self):
        module = tr.reg
        git = module.AnchorGit(self.repo)
        d = {"decision_id": "d", "document": "docs/model.md", "anchor": "anchor d-model-new", "lines": self.CUR_LINES,
             "revision": f"git:{self.c2}", "owner": "o", "findings": []}
        a = {"applicability_id": "ap-x", "finding": "f", "decision_id": "d", "relation": "supports",
             "authorization": {"status": "authorized"}}
        unborn = "git:" + "0" * 10
        listed = entry(self.c1, document="docs/study.md")
        cases = [
            ("no entry matches", [listed], f"git:{self.c3}", git),
            ("two entries match", [listed, dict(listed)], f"git:{self.c1}", git),
            ("commit does not resolve", [dict(listed, revision=unborn)], unborn, git),
            ("no git access", [listed], f"git:{self.c1}", None),
        ]
        for label, revisions, cited, access in cases:
            with self.subTest(case=label):
                self.assert_unresolved(module, dict(d, revisions=revisions), dict(a, decision_revision=cited), access,
                                       label)

    def test_r8_the_build_output_reports_a_row_whose_commit_does_not_resolve(self):
        module = tr.reg
        root = self.make(self.prior_mutate(document="docs/study.md", anchor=self.PRIOR_ANCHOR, lines="1-2"))
        r, errors, _ = module.load_valid(root, self.repo)
        self.assertEqual(errors.names(), [])

        class NoCommits:
            def commit_by_hex(self, hexid):
                return None
        per = module.derive(r, NoCommits())[4]
        fid = next(f for f, v in per.items() if any(a["applicability_id"] == "ap-prior" for a in v["applicability"]))
        row = next(a for a in per[fid]["applicability"] if a["applicability_id"] == "ap-prior")
        self.assertEqual(row["historical_source"]["status"], "not_established")
        for k in ("commit", "document", "anchor", "lines"):
            self.assertNotIn(k, row)
            self.assertNotIn(k, row["historical_source"])
        md = module.render_md({"k_index": {}, "findings": {fid: per[fid]}, "questions": {}})
        line = next(ln for ln in md.splitlines() if "ap-prior" in ln)
        self.assertIn("historical source: not_established", line)
        for text in (json.dumps(row), line):
            self.assertNotIn("docs/model.md", text)
            self.assertNotIn("anchor d-model-new", text)
            self.assertNotIn(self.CUR_LINES, text)


# ---------------------------------------------------------------------------
# A3 S2: the register with the two historical markers, against the register at PRE_S2
# ---------------------------------------------------------------------------
PRE_S2 = "5f042f45ad10bc3adb2e3c5ce4cbacecb8809b3f"
# The S2 integration commit. Its register is the S2 result (f8303d57 markers, d71f32b6 README; cac68c5a changed
# only tests). Later approved register changes start after it, so the S2 delta is PRE_S2 -> S2_COMMIT.
S2_COMMIT = "cac68c5a4e5b2efcdff90a68b4afcf6b143aabaa"
S2_LIVE = os.path.join(REAL_REPO, "metrics", "register")
HISTORICAL_PIN_SUFFIX = " (historical pin, not current authority)"
AUTHORITY_STATUS = "historical, not current authority"


def s2_note(m):
    succ = f"; superseded_by {m['superseded_by']}" if m.get("superseded_by") else ""
    return (f"historical pin, marked by {m['marked_by']} on {m['marked_at']}: {m['reason']}{succ}. "
            "A historical marker never makes this record current authority.")


def walk_dicts(obj):
    if isinstance(obj, dict):
        yield obj
        for v in obj.values():
            yield from walk_dicts(v)
    elif isinstance(obj, list):
        for v in obj:
            yield from walk_dicts(v)


def read_appl(reg):
    with open(os.path.join(reg, "applicability.jsonl"), encoding="utf-8") as fh:
        return [json.loads(line) for line in fh if line.strip()]


def read_decisions(reg):
    with open(os.path.join(reg, "decisions.json"), encoding="utf-8") as fh:
        return json.load(fh)


class S2LiveRegisterTests(unittest.TestCase):
    """The S2 change itself, as qualified: same script and repository; only the register differs: the register at
    PRE_S2 versus the register at S2_COMMIT (the live register when S2 was integrated). `anchors` reads the register
    from its pinned commit, so it compares --at PRE_S2 with --at S2_COMMIT. Both sides are fixed commits, so later
    approved register changes do not alter these comparisons; the current register is checked by
    S2MarkersInTheCurrentRegisterTests, S2LiveRegisterHistoryViolationTests and the live `validate` tests."""

    def setUp(self):
        for c in (PRE_S2, S2_COMMIT):
            self.assertTrue(have(c), f"commit {c} is not in this clone; the S2 tests cannot run")
        self.base = tempfile.mkdtemp(prefix="history-s2-")
        self.addCleanup(shutil.rmtree, self.base, ignore_errors=True)
        for tag, commit in (("pre", PRE_S2), ("s2", S2_COMMIT)):
            data = ta.git(REAL_REPO, "archive", "--format=tar", commit, "metrics/register", raw=True)
            with tarfile.open(fileobj=io.BytesIO(data)) as tf:
                tf.extractall(os.path.join(self.base, tag), filter="data")
        self.pre = os.path.join(self.base, "pre", "metrics", "register")
        self.s2 = os.path.join(self.base, "s2", "metrics", "register")
        self.at = S2_COMMIT
        self.marked_appl = {r["applicability_id"] for r in read_appl(self.s2) if r["decision_id"] in S2_MARKERS}

    def run_cmd(self, cmd, reg, tag):
        out = os.path.join(self.base, f"{cmd}-{tag}")
        if cmd == "anchors":
            where = ("--at", PRE_S2 if reg == self.pre else self.at)
        else:
            where = ("--register", reg)
        p = run_script(SCRIPT, cmd, "--repo", REAL_REPO, "--out", out, *where)
        self.assertEqual(p.returncode, 0, p.stdout + p.stderr)
        if cmd == "nominate":
            with open(out, encoding="utf-8") as fh:
                return json.load(fh), None
        name = {"build": "current-knowledge", "anchors": "anchors"}[cmd]
        with open(os.path.join(out, name + ".json"), encoding="utf-8") as fh:
            j = json.load(fh)
        with open(os.path.join(out, name + ".md"), encoding="utf-8") as fh:
            return j, fh.read().split("\n")

    def both(self, cmd):
        return self.run_cmd(cmd, self.pre, "pre"), self.run_cmd(cmd, self.s2, "s2")

    def test_s2_decisions_receive_exactly_the_approved_markers_and_nothing_else(self):
        pre, live = read_decisions(self.pre), read_decisions(self.s2)
        self.assertEqual([d["decision_id"] for d in pre], [d["decision_id"] for d in live])
        self.assertFalse(any("historical" in d for d in pre))
        others = 0
        for old, new in zip(pre, live):
            did = new["decision_id"]
            if did in S2_MARKERS:
                self.assertEqual(new["historical"], S2_MARKERS[did], did)
                self.assertEqual({k: v for k, v in new.items() if k != "historical"}, old, did)
            else:
                self.assertEqual(new, old, did)
                others += 1
        self.assertEqual(others, 9)
        live_by_id = {d["decision_id"]: d for d in live}
        self.assertIn(S2_COUNTERPART, live_by_id)
        self.assertNotIn("historical", live_by_id[S2_COUNTERPART])  # the counterpart stays current
        self.assertNotIn("superseded_by", S2_MARKERS["dogfood-plan.round2-status-line"])

    def test_s2_other_register_files_are_byte_identical_and_the_readme_changes_one_line(self):
        def files(root):
            return sorted(os.path.relpath(os.path.join(d, f), root) for d, _, fs in os.walk(root) for f in fs)
        self.assertEqual(files(self.pre), files(self.s2))
        for rel in files(self.s2):
            if rel not in ("decisions.json", "README.md"):
                self.assertEqual(ta.slurp(os.path.join(self.pre, rel)), ta.slurp(os.path.join(self.s2, rel)), rel)
        old = ta.slurp(os.path.join(self.pre, "README.md")).decode().split("\n")
        new = ta.slurp(os.path.join(self.s2, "README.md")).decode().split("\n")
        self.assertEqual(len(old), len(new))
        changed = [(a, b) for a, b in zip(old, new) if a != b]
        self.assertEqual(len(changed), 1)
        self.assertIn("no field marks it", changed[0][0])
        self.assertIn("now has a structured `historical` marker", changed[0][1])

    def test_s2_build_changes_only_the_rows_citing_a_marked_decision(self):
        (pj, pm), (lj, lm) = self.both("build")
        self.assertFalse(any("historical_pin" in d for d in walk_dicts(pj)))
        popped = 0
        for d in walk_dicts(lj):
            if "historical_pin" in d:
                self.assertIn(d.get("decision_id"), S2_MARKERS)
                self.assertIs(d.pop("historical_pin"), True)
                popped += 1
        self.assertEqual(popped, len(self.marked_appl))
        self.assertEqual(lj, pj)
        self.assertEqual(len(lm), len(pm))
        changed = [(a, b) for a, b in zip(pm, lm) if a != b]
        self.assertEqual(len(changed), len(self.marked_appl))
        for a, b in changed:
            self.assertEqual(b.replace(HISTORICAL_PIN_SUFFIX, "", 1), a)
            self.assertTrue(any(aid in b for aid in self.marked_appl), b)

    def test_s2_anchors_change_only_as_the_markers_require(self):
        (pj, pm), (lj, lm) = self.both("anchors")
        self.assertEqual((pj["pinned_commit"], lj["pinned_commit"]), (PRE_S2, self.at))
        self.assertEqual(pj["target_requested"], PRE_S2)
        lj["pinned_commit"], lj["target_requested"] = pj["pinned_commit"], pj["target_requested"]
        lsha, psha = (j["register"]["inputs"]["decisions.json"] for j in (lj, pj))
        self.assertEqual(lsha["sha256"], sha(show(self.at, "metrics/register/decisions.json")))
        self.assertEqual(psha["sha256"], sha(show(PRE_S2, "metrics/register/decisions.json")))
        lsha["sha256"] = psha["sha256"]
        pre_by_id = {e["decision_id"]: e for e in pj["decisions"]}
        seen = set()
        for e in lj["decisions"]:
            did = e["decision_id"]
            old = pre_by_id[did]
            self.assertNotIn("historical", old)
            self.assertNotIn("authority_status", old)
            if did not in S2_MARKERS:
                continue
            seen.add(did)
            self.assertEqual(e.pop("historical"), S2_MARKERS[did])
            self.assertEqual(e.pop("authority_status"), AUTHORITY_STATUS)
            self.assertEqual(e["notes"], [s2_note(S2_MARKERS[did])])
            self.assertFalse(any("no field marks it" in n for n in e["notes"]))
            e["notes"] = old["notes"]
            self.assertEqual(len(e["applicability"]), len(old["applicability"]))
            for row, prow in zip(e["applicability"], old["applicability"]):
                self.assertEqual(row["after_state"], "historical_pin", did)
                self.assertNotEqual(prow["after_state"], "historical_pin", did)
                row["after_state"] = prow["after_state"]
        self.assertEqual(seen, set(S2_MARKERS))
        self.assertEqual(lj, pj)
        # markdown: every changed line belongs to a marked decision, its applicability, the input digest or the
        # new historical section
        allowed = tuple(S2_MARKERS) + tuple(self.marked_appl) + (
            "Pinned commit: `", "input `metrics/register/decisions.json` sha256", "## Historical decisions (not current authority)",
            "- note: historical pin, marked by ")
        import difflib
        fixed_note = "- note: deliberate historical pin (named in the register README prose, no field marks it)"
        for line in difflib.ndiff(pm, lm):
            if line[:2] in ("- ", "+ ") and line[2:].strip():
                ok = allowed + ((fixed_note,) if line[:2] == "- " else ())
                self.assertTrue(any(a in line[2:] for a in ok), line)
        self.assertFalse(any(fixed_note in x for x in lm))
        self.assertIn("## Historical decisions (not current authority)", lm)

    def test_s2_nominate_keeps_the_historical_flag_and_reason_and_changes_nothing_else(self):
        (pj, _), (lj, _) = self.both("nominate")
        self.assertFalse(any("historical" in d or "historical_reason" in d for d in walk_dicts(pj)))
        reached = {did: 0 for did in S2_MARKERS}
        counterpart_beside_pin = 0
        for nom in lj["nominations"]:
            ids = {d["decision_id"] for d in nom["decisions"]}
            for d in nom["decisions"]:
                did = d["decision_id"]
                if did in S2_MARKERS:
                    self.assertIs(d.pop("historical"), True, did)
                    self.assertEqual(d.pop("historical_reason"), S2_MARKERS[did]["reason"], did)
                    reached[did] += 1
                else:
                    self.assertNotIn("historical", d, did)  # current decisions carry no historical flag
            for r in nom["applicability_records"]:
                if r["decision_id"] in S2_MARKERS:
                    self.assertIs(r.pop("historical"), True, r["applicability_id"])
                else:
                    self.assertNotIn("historical", r, r["applicability_id"])
            if {S2_COUNTERPART, "model-selection.claude-host-builder-rows-at-df288c3d"} <= ids:
                counterpart_beside_pin += 1
        self.assertTrue(all(reached.values()), reached)  # a historical decision is still reached, and labelled
        self.assertGreaterEqual(counterpart_beside_pin, 1)  # the pin and its current counterpart are distinguishable
        self.assertEqual(lj, pj)

    def test_s2_no_applicability_authorization_is_transferred_or_changed(self):
        def auths(recs):
            return {r["applicability_id"]: (r["decision_id"], r["decision_revision"], r["authorization"]) for r in recs}
        self.assertEqual(auths(read_appl(self.s2)), auths(read_appl(self.pre)))
        for cmd in ("anchors", "nominate", "build"):
            (pj, _), (lj, _) = self.both(cmd)

            def seen(j):
                return sorted(json.dumps([d.get("applicability_id"), d.get("decision_id"), d.get("authorization"),
                                          d.get("authorized")], sort_keys=True)
                              for d in walk_dicts(j) if "applicability_id" in d)
            self.assertEqual(seen(lj), seen(pj), cmd)

    def test_s2_history_accepts_the_markers_from_the_pre_s2_register(self):
        p = run_script(SCRIPT, "history", "--repo", REAL_REPO, "--base", PRE_S2, "--at", S2_COMMIT)
        self.assertEqual(p.returncode, 0, p.stdout + p.stderr)
        rep = json.loads(p.stdout)
        self.assertEqual((rep["violations"], rep["owner_review"]), ([], []))


class S2MarkersInTheCurrentRegisterTests(unittest.TestCase):
    """What S2 established must still hold in the current register after later approved changes: the two markers
    keep their approved values, `history` from before and after S2 finds no violation (owner review entries are
    expected for later approved status changes and are not asserted), and every view labels exactly the marked
    decisions. Unlike S2LiveRegisterTests, nothing here compares whole outputs, so an approved change elsewhere in
    the register does not require restating these tests."""

    def setUp(self):
        for c in (PRE_S2, S2_COMMIT):
            self.assertTrue(have(c), f"commit {c} is not in this clone; the S2 tests cannot run")
        self.base = tempfile.mkdtemp(prefix="history-s2-current-")
        self.addCleanup(shutil.rmtree, self.base, ignore_errors=True)

    def view(self, cmd, *where):
        out = os.path.join(self.base, cmd)
        p = run_script(SCRIPT, cmd, "--repo", REAL_REPO, "--out", out, *where)
        self.assertEqual(p.returncode, 0, p.stdout + p.stderr)
        name = {"build": "current-knowledge.json", "anchors": "anchors.json"}.get(cmd)
        with open(os.path.join(out, name) if name else out, encoding="utf-8") as fh:
            return json.load(fh)

    def test_the_s2_markers_keep_their_approved_values(self):
        by_id = {d["decision_id"]: d for d in read_decisions(S2_LIVE)}
        for did, marker in S2_MARKERS.items():
            self.assertEqual(by_id[did].get("historical"), marker, did)
        self.assertNotIn("historical", by_id[S2_COUNTERPART])

    def test_history_from_before_and_after_s2_to_the_current_register_has_no_violation(self):
        for base in (PRE_S2, S2_COMMIT):
            with self.subTest(base=base):
                p = run_script(SCRIPT, "history", "--repo", REAL_REPO, "--base", base)
                self.assertEqual(p.returncode, 0, p.stdout + p.stderr)
                self.assertEqual(json.loads(p.stdout)["violations"], [])

    def test_the_current_views_label_exactly_the_marked_decisions(self):
        decisions = read_decisions(S2_LIVE)
        markers = {d["decision_id"]: d["historical"] for d in decisions if "historical" in d}
        self.assertTrue(set(S2_MARKERS) <= set(markers), markers)
        marked_appl = {r["applicability_id"] for r in read_appl(S2_LIVE) if r["decision_id"] in markers}

        nom = self.view("nominate", "--register", S2_LIVE)
        reached = set()
        for n in nom["nominations"]:
            for d in n["decisions"]:
                did = d["decision_id"]
                if did in markers:
                    self.assertIs(d.get("historical"), True, did)
                    self.assertEqual(d.get("historical_reason"), markers[did]["reason"], did)
                    reached.add(did)
                else:
                    self.assertNotIn("historical", d, did)
            for r in n["applicability_records"]:
                self.assertEqual(r.get("historical"), True if r["decision_id"] in markers else None,
                                 r["applicability_id"])
        self.assertEqual(reached & set(S2_MARKERS), set(S2_MARKERS))  # still reached, and labelled

        pins = [d for d in walk_dicts(self.view("build", "--register", S2_LIVE)) if "historical_pin" in d]
        self.assertTrue(all(d.get("decision_id") in markers and d["historical_pin"] is True for d in pins), pins)
        self.assertEqual(len(pins), len(marked_appl))

        # anchors reads the committed register, so its markers come from HEAD, not the working tree
        head_markers = {d["decision_id"]: d["historical"]
                        for d in json.loads(show("HEAD", "metrics/register/decisions.json")) if "historical" in d}
        for e in self.view("anchors", "--at", "HEAD")["decisions"]:
            did = e["decision_id"]
            if did in head_markers:
                self.assertEqual((e.get("historical"), e.get("authority_status")), (head_markers[did], AUTHORITY_STATUS))
                self.assertTrue(all(row["after_state"] == "historical_pin" for row in e["applicability"]), did)
            else:
                self.assertFalse({"historical", "authority_status"} & set(e), did)


class S2LiveRegisterHistoryViolationTests(HistoryFixture):
    """`history` from the live register (markers included) refuses to lose or rewrite what it records."""

    def base_state(self):
        return copy.deepcopy(read_decisions(S2_LIVE)), read_appl(S2_LIVE)

    @staticmethod
    def by_id(decisions, did):
        return next(d for d in decisions if d["decision_id"] == did)

    def test_s2_the_marker_addition_is_accepted(self):
        def strip(decisions, appl):
            for d in decisions:
                d.pop("historical", None)
        unmarked = self.later(strip)
        self.commit_base()
        p, rep = self.history(base=unmarked, at=self.base_commit)
        self.assertEqual((p.returncode, self.names(rep), rep["owner_review"]), (0, [], []))

    def test_s2_removing_or_rewriting_a_marker_or_a_prior_record_is_rejected(self):
        self.commit_base()
        pin, dog = "model-selection.claude-host-builder-rows-at-df288c3d", "dogfood-plan.round2-status-line"
        aid = next(r["applicability_id"] for r in read_appl(S2_LIVE) if r["decision_id"] == dog)

        def mark(did, f):
            return lambda decisions, appl: f(self.by_id(decisions, did)["historical"])

        def rec(f):
            return lambda decisions, appl: f(next(r for r in appl if r["applicability_id"] == aid), appl)

        cases = [
            ("pin marker removed", lambda ds, ap: self.by_id(ds, pin).pop("historical"), "historical-marker-removed"),
            ("dogfood marker removed", lambda ds, ap: self.by_id(ds, dog).pop("historical"),
             "historical-marker-removed"),
            ("reason reworded", mark(pin, lambda h: h.update(reason="reworded")), "historical-marker-edited"),
            ("marked_by changed", mark(dog, lambda h: h.update(marked_by="someone else")), "historical-marker-edited"),
            ("marked_at backdated", mark(dog, lambda h: h.update(marked_at="2026-11-01")), "historical-marker-edited"),
            ("superseded_by dropped", mark(pin, lambda h: h.pop("superseded_by")), "historical-marker-edited"),
            ("superseded_by retargeted", mark(pin, lambda h: h.update(superseded_by="model-selection.builder-default")),
             "historical-marker-edited"),
            ("superseded_by added to the dogfood marker", mark(dog, lambda h: h.update(superseded_by=S2_COUNTERPART)),
             "historical-marker-edited"),
            ("historical decision removed", lambda ds, ap: ds.remove(self.by_id(ds, pin)), "decision-id-removed"),
            ("historical decision renamed", lambda ds, ap: self.by_id(ds, dog).update(decision_id=dog + "-renamed"),
             "decision-id-renamed"),
            ("historical decision's revision changed without history",
             lambda ds, ap: self.by_id(ds, pin).update(revision="git:4ff69ffd"),
             "current-revision-changed-without-history"),
            ("another decision removed", lambda ds, ap: ds.remove(self.by_id(ds, S2_COUNTERPART)),
             "decision-id-removed"),
            ("record citing a marked decision removed", rec(lambda r, ap: ap.remove(r)), "applicability-record-removed"),
            ("record moved to the counterpart", rec(lambda r, ap: r.update(decision_id=S2_COUNTERPART)),
             "applicability-record-changed"),
            ("record's authorization owner changed", rec(lambda r, ap: r["authorization"].update(owner="someone else")),
             "authorization-owner-changed"),
        ]
        for label, mutate, name in cases:
            with self.subTest(label):
                p, rep = self.history(at=self.later(mutate))
                self.assertEqual(p.returncode, 1, label + ": " + p.stdout + p.stderr)
                self.assertIn(name, self.names(rep), rep["violations"])
                self.assert_forbidden_phrases_absent(p.stdout)


# ---------------------------------------------------------------------------
# A3 S3: the two re-anchored current decisions (T-3, T-4)
# ---------------------------------------------------------------------------

class S3RegisterTests(unittest.TestCase):
    """The S3 change itself, pinned at both ends: the register at S3_BASE versus the register at S3_DATA. Same script
    and repository; `anchors` compares --at S3_BASE with --at S3_DATA and the other commands read the two extracted
    registers, so neither the working tree nor later approved register changes alter these comparisons."""

    def setUp(self):
        for c in (S3_BASE, S3_DATA):
            self.assertTrue(have(c), f"commit {c} is not in this clone; the S3 tests cannot run")
        self.tmp = tempfile.mkdtemp(prefix="history-s3-")
        self.addCleanup(shutil.rmtree, self.tmp, ignore_errors=True)
        self.reg = {}
        for tag, commit in (("base", S3_BASE), ("s3", S3_DATA)):
            data = ta.git(REAL_REPO, "archive", "--format=tar", commit, "metrics/register", raw=True)
            with tarfile.open(fileobj=io.BytesIO(data)) as tf:
                tf.extractall(os.path.join(self.tmp, tag), filter="data")
            self.reg[tag] = os.path.join(self.tmp, tag, "metrics", "register")

    def run_cmd(self, cmd, tag):
        out = os.path.join(self.tmp, f"{cmd}-{tag}")
        where = ("--at", {"base": S3_BASE, "s3": S3_DATA}[tag]) if cmd == "anchors" else ("--register", self.reg[tag])
        p = run_script(SCRIPT, cmd, "--repo", REAL_REPO, "--out", out, *where)
        self.assertEqual(p.returncode, 0, p.stdout + p.stderr)
        if cmd == "nominate":
            with open(out, encoding="utf-8") as fh:
                return json.load(fh), None
        name = {"build": "current-knowledge", "anchors": "anchors"}[cmd]
        with open(os.path.join(out, name + ".json"), encoding="utf-8") as fh:
            j = json.load(fh)
        with open(os.path.join(out, name + ".md"), encoding="utf-8") as fh:
            return j, fh.read().split("\n")

    def both(self, cmd):
        return self.run_cmd(cmd, "base"), self.run_cmd(cmd, "s3")

    def test_s3_exactly_the_two_decisions_change_by_the_approved_values(self):
        pre, new = read_decisions(self.reg["base"]), read_decisions(self.reg["s3"])
        self.assertEqual([d["decision_id"] for d in pre], [d["decision_id"] for d in new])
        self.assertFalse(any("revisions" in d for d in pre))
        changed = 0
        for old, cur in zip(pre, new):
            did = cur["decision_id"]
            if did not in S3_NEW:
                self.assertEqual(cur, old, did)  # the moved and historical decisions, markers included, are untouched
                continue
            changed += 1
            self.assertEqual(cur["revisions"], S3_REVISIONS[did], did)
            self.assertEqual({k: v for k, v in cur.items() if k != "revisions"}, {**old, **S3_NEW[did]}, did)
            # the revisions entry is the base's current pin, copied unchanged
            prior = {k: v for k, v in S3_REVISIONS[did][0].items() if k not in ("superseded_at", "reason")}
            self.assertEqual(prior, {k: old[k] for k in ("revision", "lines", "document", "anchor")}, did)
        self.assertEqual(changed, 2)
        self.assertEqual({d["decision_id"]: d["historical"] for d in new if "historical" in d}, S2_MARKERS)
        p = run_script(SCRIPT, "validate", "--repo", REAL_REPO, "--register", self.reg["s3"])
        self.assertEqual(p.returncode, 0, p.stdout + p.stderr)
        self.assertIn("11 decisions, 18 applicability records", p.stdout)

    def test_s3_superseded_at_is_the_date_of_each_superseding_commit(self):
        for did, (prior,) in S3_REVISIONS.items():
            new_rev = S3_NEW[did]["revision"][len("git:"):]
            dates = subprocess.run(["git", "-C", REAL_REPO, "show", "-s", "--format=%aI %cI", new_rev],
                                   capture_output=True, text=True, check=True).stdout.split()
            self.assertEqual(len(dates), 2, did)
            self.assertTrue(all(x.startswith(prior["superseded_at"]) for x in dates), (did, dates))
            start, _, end = prior["lines"].partition("-")
            old_text = show(prior["revision"][len("git:"):], prior["document"]).decode().split("\n")
            old_lines = old_text[int(start) - 1:int(end or start)]
            parent = show(new_rev + "^", prior["document"]).decode().split("\n")
            child = show(new_rev, prior["document"]).decode().split("\n")
            self.assertTrue(old_lines and all(x in parent for x in old_lines), did)  # still there before it
            self.assertFalse(any(x in child for x in old_lines), did)  # superseded by it

    def test_s3_applicability_is_unchanged_and_the_readme_adds_one_sentence(self):
        def files(root):
            return sorted(os.path.relpath(os.path.join(d, f), root) for d, _, fs in os.walk(root) for f in fs)
        base, s3 = self.reg["base"], self.reg["s3"]
        self.assertEqual(files(base), files(s3))
        for rel in files(s3):
            if rel not in ("decisions.json", "README.md"):  # applicability.jsonl included: no record is added
                self.assertEqual(ta.slurp(os.path.join(base, rel)), ta.slurp(os.path.join(s3, rel)), rel)
        old = ta.slurp(os.path.join(base, "README.md")).decode()
        new = ta.slurp(os.path.join(s3, "README.md")).decode()
        self.assertEqual(old.count(S3_README_OLD), 1)
        self.assertEqual(new, old.replace(S3_README_OLD, S3_README_NEW))

    def test_s3_build_shows_the_two_prior_revision_records_at_their_historical_source(self):
        (pj, pm), (lj, lm) = self.both("build")
        self.assertFalse(any("at_prior_revision" in d for d in walk_dicts(pj)))
        seen = set()
        for d in walk_dicts(lj):
            if "at_prior_revision" not in d:
                continue
            aid = d.get("applicability_id")
            self.assertIn(aid, S3_PRIOR_RECORDS)
            did = S3_PRIOR_RECORDS[aid]
            (prior,) = S3_REVISIONS[did]
            self.assertIs(d.pop("at_prior_revision"), True)
            self.assertEqual(d.pop("current_decision_revision"), S3_NEW[did]["revision"])
            src = d.pop("historical_source")
            self.assertTrue(src.pop("commit").startswith(prior["revision"][len("git:"):]), aid)
            self.assertEqual(src, {"anchor": prior["anchor"], "document": prior["document"], "document_basis": "entry",
                                   "label": HISTORICAL_LABEL_JSON, "lines": prior["lines"],
                                   "revision": prior["revision"], "status": "resolved"}, aid)
            self.assertFalse({"anchor", "document"} & set(d), aid)  # no current location is shown for it
            d["anchor"], d["document"] = prior["anchor"], prior["document"]
            seen.add(aid)
        self.assertEqual(seen, set(S3_PRIOR_RECORDS))
        self.assertEqual(lj, pj)
        self.assertEqual(len(lm), len(pm))
        changed = [(a, b) for a, b in zip(pm, lm) if a != b]
        self.assertEqual(len(changed), 2)
        for a, b in changed:
            aid = next(x for x in S3_PRIOR_RECORDS if x in a)
            self.assertIn(aid, b)
            self.assertIn(HISTORICAL_LABEL_MD, b)
            self.assertIn("not carried", b)
            self.assertNotIn(HISTORICAL_LABEL_MD, a)

    def test_s3_anchors_change_only_the_two_reanchored_entries(self):
        (pj, pm), (lj, lm) = self.both("anchors")
        self.assertEqual((pj["pinned_commit"], lj["pinned_commit"]), (S3_BASE, S3_DATA))
        for j, c in ((pj, S3_BASE), (lj, S3_DATA)):
            self.assertEqual(j["register"]["inputs"]["decisions.json"]["sha256"],
                             sha(show(c, "metrics/register/decisions.json")))
        count = lambda j: tuple(j["summary"][k] for k in ("modified", "moved", "unchanged"))
        self.assertEqual((count(pj), count(lj)), ((3, 4, 4), (1, 5, 5)))
        expect = {S3_HOST: ("unchanged", "128-129"), S3_QUOTA: ("moved", "276")}
        pre_by_id = {e["decision_id"]: e for e in pj["decisions"]}
        others = 0
        for e in lj["decisions"]:
            did = e["decision_id"]
            old = pre_by_id[did]
            if did not in S3_NEW:
                self.assertEqual(e, old, did)
                others += 1
                continue
            self.assertEqual(old["class"], "modified", did)
            self.assertEqual((e["class"], e["after"]["lines"], e["error"]), expect[did] + (None,), did)
            for k, v in S3_NEW[did].items():
                self.assertEqual(e[k], v, (did, k))
            self.assertEqual((e["before"]["revision"], e["before"]["lines"]),
                             (S3_NEW[did]["revision"], S3_NEW[did]["lines"]), did)
            self.assertEqual(e["revisions"], S3_REVISIONS[did], did)
            self.assertEqual(len(e["applicability"]), len(old["applicability"]))
            moving = ("after_state", "record_revision_matches_decision")
            for row, prow in zip(e["applicability"], old["applicability"]):
                self.assertEqual(S3_PRIOR_RECORDS.get(row["record"]["applicability_id"]), did)
                self.assertEqual([prow[k] for k in moving], ["pending_judgment_required", True])
                self.assertEqual([row[k] for k in moving], ["at_prior_revision", False])
                self.assertEqual({k: v for k, v in row.items() if k not in moving},
                                 {k: v for k, v in prow.items() if k not in moving})
        self.assertEqual(others, 9)
        import difflib
        allowed = tuple(S3_NEW) + tuple(S3_PRIOR_RECORDS) + (
            "Pinned commit: `", "input `metrics/register/decisions.json` sha256", "- modified: ", "- moved: ",
            "- unchanged: ", "## Decisions with listed prior revisions", "  - prior revision `git:",
            "- before: revision `git:", "- after: lines ", "- at the revision:", "- at the target:", "  - lines ",
            "    - before: `", "    - after: `", "- candidate (not chosen): ")
        for line in difflib.ndiff(pm, lm):
            if line[:2] in ("- ", "+ ") and line[2:].strip():
                self.assertTrue(any(a in line[2:] for a in allowed), line)
        self.assertEqual((pm.count("## Decisions with listed prior revisions"),
                          lm.count("## Decisions with listed prior revisions")), (0, 1))

    def test_s3_nominate_changes_only_the_two_decisions_and_flags_their_prior_records(self):
        (pj, _), (lj, _) = self.both("nominate")
        self.assertFalse(any("at_prior_revision" in d for d in walk_dicts(pj)))
        self.assertEqual(len(lj["nominations"]), len(pj["nominations"]))
        changed = flagged = 0
        for nom, pnom in zip(lj["nominations"], pj["nominations"]):
            self.assertEqual(len(nom["decisions"]), len(pnom["decisions"]))
            for d, pd in zip(nom["decisions"], pnom["decisions"]):
                if d["decision_id"] in S3_NEW:
                    for k, v in S3_NEW[d["decision_id"]].items():
                        self.assertEqual(d[k], v, (d["decision_id"], k))
                        d[k] = pd[k]
                    changed += 1
            for r in nom["applicability_records"]:
                if r["applicability_id"] in S3_PRIOR_RECORDS:
                    self.assertIs(r.pop("at_prior_revision"), True, r["applicability_id"])
                    flagged += 1
                else:
                    self.assertNotIn("at_prior_revision", r, r["applicability_id"])
        self.assertEqual((changed, flagged), (3, 3))
        self.assertEqual(lj, pj)

    def test_s3_no_authorization_is_carried_moved_or_added(self):
        def auths(reg):
            return {r["applicability_id"]: (r["decision_id"], r["decision_revision"], r["authorization"])
                    for r in read_appl(reg)}
        base, s3 = auths(self.reg["base"]), auths(self.reg["s3"])
        self.assertEqual(s3, base)
        for aid, did in S3_PRIOR_RECORDS.items():
            self.assertEqual(s3[aid][:2], (did, S3_REVISIONS[did][0]["revision"]), aid)  # stays on the prior revision
        self.assertEqual({aid: s3[aid][2]["status"] for aid in S3_PRIOR_RECORDS},
                         {"ap-k2-traced-claude-host-rows": "pending", "ap-k3-quota-fit-cost-table-quota-row": "authorized"})
        self.assertFalse([aid for aid, (did, rev, _) in s3.items() if did in S3_NEW and rev == S3_NEW[did]["revision"]])
        for cmd in ("anchors", "nominate", "build"):
            (pj, _), (lj, _) = self.both(cmd)

            def seen(j):
                return sorted(json.dumps([d.get("applicability_id"), d.get("decision_id"), d.get("authorization"),
                                          d.get("authorized")], sort_keys=True)
                              for d in walk_dicts(j) if "applicability_id" in d)
            self.assertEqual(seen(lj), seen(pj), cmd)

    def test_s3_history_shows_no_violation_and_no_owner_review(self):
        p = run_script(SCRIPT, "history", "--repo", REAL_REPO, "--base", S3_BASE, "--at", S3_DATA)
        self.assertEqual(p.returncode, 0, p.stdout + p.stderr)
        rep = json.loads(p.stdout)
        self.assertEqual((rep["violations"], rep["owner_review"]), ([], []))


class S3HistoryRefusalTests(HistoryFixture):
    """`history` from the S3 register (pinned at S3_DATA) refuses to lose or rewrite a listed prior revision, to change
    the current revision again without history, or to move, remove or re-own the records citing the prior revisions.
    The S3 change itself passes from the S3_BASE register."""

    @staticmethod
    def pinned(commit):
        lines = show(commit, "metrics/register/applicability.jsonl").decode().split("\n")
        return json.loads(show(commit, "metrics/register/decisions.json")), [json.loads(x) for x in lines if x.strip()]

    def base_state(self):
        return self.pinned(S3_DATA)

    def test_s3_the_reanchoring_is_accepted_from_the_s3_base_register(self):
        pre = self.commit_register(*self.pinned(S3_BASE), msg="S3 base register")
        self.commit_base()
        p, rep = self.history(base=pre, at=self.base_commit)
        self.assertEqual((p.returncode, self.names(rep), rep["owner_review"]), (0, [], []))

    def test_s3_losing_or_rewriting_a_prior_revision_or_carrying_its_records_is_rejected(self):
        self.commit_base()

        def by_id(ds, did):
            return next(d for d in ds if d["decision_id"] == did)

        def rev(did, f):
            return lambda ds, ap: f(by_id(ds, did)["revisions"][0])

        def rec(aid, f):
            return lambda ds, ap: f(next(r for r in ap if r["applicability_id"] == aid), ap)

        k2, k3 = "ap-k2-traced-claude-host-rows", "ap-k3-quota-fit-cost-table-quota-row"
        cases = [
            ("host revisions removed", lambda ds, ap: by_id(ds, S3_HOST).pop("revisions"), "revision-removed"),
            ("quota revisions emptied", lambda ds, ap: by_id(ds, S3_QUOTA)["revisions"].clear(), "revision-removed"),
            ("prior anchor edited", rev(S3_HOST, lambda e: e.update(anchor="edited")), "revision-edited"),
            ("prior lines edited", rev(S3_QUOTA, lambda e: e.update(lines="116")), "revision-edited"),
            ("reason edited", rev(S3_HOST, lambda e: e.update(reason="edited")), "revision-edited"),
            ("superseded_at set to the migration date", rev(S3_QUOTA, lambda e: e.update(superseded_at="2026-11-09")),
             "revision-edited"),
            ("revision changed again without history",
             lambda ds, ap: by_id(ds, S3_QUOTA).update(revision="git:869a5add"),
             "current-revision-changed-without-history"),
            ("ap-k2 moved to the new revision", rec(k2, lambda r, ap: r.update(decision_revision="git:1906090c")),
             "applicability-record-changed"),
            ("ap-k3 moved to the new revision", rec(k3, lambda r, ap: r.update(decision_revision="git:5149cc27")),
             "applicability-record-changed"),
            ("ap-k3 authorization owner changed", rec(k3, lambda r, ap: r["authorization"].update(owner="someone else")),
             "authorization-owner-changed"),
            ("ap-k2 removed", rec(k2, lambda r, ap: ap.remove(r)), "applicability-record-removed"),
        ]
        for label, mutate, name in cases:
            with self.subTest(label):
                p, rep = self.history(at=self.later(mutate))
                self.assertEqual(p.returncode, 1, label + ": " + p.stdout + p.stderr)
                self.assertIn(name, self.names(rep), rep["violations"])
                self.assert_forbidden_phrases_absent(p.stdout)


if __name__ == "__main__":
    unittest.main()
