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
            for d in json.load(fh):
                self.assertFalse({"historical", "revisions", "evidence_state"} & set(d), d["decision_id"])


if __name__ == "__main__":
    unittest.main()
