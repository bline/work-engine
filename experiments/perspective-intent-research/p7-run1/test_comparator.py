#!/usr/bin/env python3
"""Tests for the P7 run 1 comparator v2.

Run: python3 -I -B -m unittest discover -s experiments/perspective-intent-research/p7-run1 -p 'test_*.py'
"""

import os
import sys
import unittest

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import comparator as c  # noqa: E402

SCHEMA = "obligation-extract/v2"


def read(rel):
    with open(os.path.join(HERE, rel)) as f:
        return f.read()


def F(status, value=(), readings=None, other_text=None):
    f = {"status": status, "value": list(value)}
    if readings is not None:
        f["readings"] = [list(r) for r in readings]
    if other_text:
        f["other_text"] = other_text
    return f


def P(id_, doc, span, owner=None, **fields):
    e = {"id": id_, "document": doc, "role": "parent", "span": span, "fields": fields}
    if owner is not None:
        e["semantics_owner"] = {"status": "binding", "names": owner, "span": span}
    return e


def D(id_, doc, span, asserted_as="inherited", scope="partial", refs=None, cands=None, adopter=None, **fields):
    e = {"id": id_, "document": doc, "role": "derived", "span": span, "fields": fields,
         "asserted_as": asserted_as, "scope": scope,
         "references_made": refs if refs is not None else [{"kind": "specific", "text": "see parent", "target_document": "DOC-1"}],
         "support_candidates": cands or {}}
    if adopter is not None:
        e["adopted_by"] = {"status": "binding", "names": adopter, "span": span}
    return e


# Synthetic documents for the decision-table tests: spans must be quoted from them.
DOCS = {"DOC-1": "parent requirement one. parent requirement two. parent requirement three.",
        "DOC-2": "derived statement A. derived statement B. derived statement C."}
PARENTS, DERIVED = {"DOC-1"}, {"DOC-2"}


def run(*entries, kind="review", fallback=None, docs=DOCS, parents=PARENTS, derived=DERIVED):
    return c.report({"schema": SCHEMA, "entries": list(entries)}, docs, parents, derived, kind, fallback)


def flags(rep, phase="pre"):
    return {(f["entry"], f.get("property"), f["flag"]) for f in rep[phase]["derived_flags"]}


class DecisionTable(unittest.TestCase):
    def test_partial_subset_of_required_items_is_entailed(self):
        p = P("p1", "DOC-1", "parent requirement one", acceptance_evidence=F("binding", ["independent_check", "failure_reported_distinctly"]))
        d = D("d1", "DOC-2", "derived statement A", cands={"acceptance_evidence": ["p1"]},
              acceptance_evidence=F("binding", ["failure_reported_distinctly"]))
        self.assertEqual(flags(run(p, d)), set())

    def test_added_item_is_stronger_and_owner_checked(self):
        p = P("p1", "DOC-1", "parent requirement one", owner=["RL"], acceptance_evidence=F("binding", ["outcome_observed"]))
        d = D("d1", "DOC-2", "derived statement A", cands={"acceptance_evidence": ["p1"]}, adopter=["dev-vertical"],
              acceptance_evidence=F("binding", ["actor_attribution"]))
        fl = flags(run(p, d))
        self.assertIn(("d1", "acceptance_evidence", "stronger"), fl)
        self.assertIn(("d1", None, "adopted_without_owner"), fl)

    def test_owner_adoption_matching_owner_passes_authority(self):
        p = P("p1", "DOC-1", "parent requirement one", owner=["RL"], acceptance_evidence=F("binding", ["outcome_observed"]))
        d = D("d1", "DOC-2", "derived statement A", cands={"acceptance_evidence": ["p1"]}, adopter=["the RL"],
              acceptance_evidence=F("binding", ["outcome_observed", "actor_attribution"]))
        fl = flags(run(p, d))
        self.assertIn(("d1", "acceptance_evidence", "stronger"), fl)
        self.assertNotIn(("d1", None, "adopted_without_owner"), fl)

    def test_complete_replacement_widening_actors_is_weaker(self):
        p = P("p1", "DOC-1", "parent requirement one", outcome_owner=F("binding", ["acting_component"]))
        d = D("d1", "DOC-2", "derived statement A", scope="complete", cands={"outcome_owner": ["p1"]},
              outcome_owner=F("binding", ["acting_component", "managing_system"]))
        self.assertIn(("d1", "outcome_owner", "weaker"), flags(run(p, d)))

    def test_partial_widening_actors_is_entailed(self):
        p = P("p1", "DOC-1", "parent requirement one", outcome_owner=F("binding", ["acting_component"]))
        d = D("d1", "DOC-2", "derived statement A", cands={"outcome_owner": ["p1"]},
              outcome_owner=F("binding", ["acting_component", "managing_system"]))
        self.assertEqual(flags(run(p, d)), set())

    def test_complete_replacement_dropping_whole_field(self):
        p = P("p1", "DOC-1", "parent requirement one",
              acceptance_evidence=F("binding", ["independent_check"]), verifier=F("binding", ["independent_observer"]))
        d = D("d1", "DOC-2", "derived statement A", scope="complete",
              cands={"acceptance_evidence": ["p1"], "verifier": ["p1"]},
              acceptance_evidence=F("binding", ["independent_check"]))
        self.assertIn(("d1", "verifier", "dropped"), flags(run(p, d)))

    def test_mixed_add_and_drop_reports_both(self):
        p = P("p1", "DOC-1", "parent requirement one", acceptance_evidence=F("binding", ["independent_check", "failure_reported_distinctly"]))
        d = D("d1", "DOC-2", "derived statement A", scope="complete", cands={"acceptance_evidence": ["p1"]},
              acceptance_evidence=F("binding", ["independent_check", "actor_attribution"]))
        fl = flags(run(p, d))
        self.assertIn(("d1", "acceptance_evidence", "stronger"), fl)
        self.assertIn(("d1", "acceptance_evidence", "weaker"), fl)

    def test_unchanged_must_be_equal(self):
        p = P("p1", "DOC-1", "parent requirement one", acceptance_evidence=F("binding", ["independent_check", "failure_reported_distinctly"]))
        d = D("d1", "DOC-2", "derived statement A", asserted_as="unchanged", cands={"acceptance_evidence": ["p1"]},
              acceptance_evidence=F("binding", ["independent_check"]))
        fl = flags(run(p, d))
        self.assertIn(("d1", None, "unchanged_claim_false"), fl)

    def test_unchanged_equal_is_supported(self):
        p = P("p1", "DOC-1", "parent requirement one", acceptance_evidence=F("binding", ["independent_check"]))
        d = D("d1", "DOC-2", "derived statement A", asserted_as="unchanged", cands={"acceptance_evidence": ["p1"]},
              acceptance_evidence=F("binding", ["independent_check"]))
        rep = run(p, d)
        self.assertEqual(flags(rep), set())
        self.assertEqual(rep["pre"]["derived_records"][0]["standing"], "supported")

    def test_disjoint_and_overlapping_actor_sets(self):
        p = P("p1", "DOC-1", "parent requirement one", outcome_owner=F("binding", ["acting_component", "managing_system"]))
        d1 = D("d1", "DOC-2", "derived statement A", cands={"outcome_owner": ["p1"]}, outcome_owner=F("binding", ["operator"]))
        d2 = D("d2", "DOC-2", "derived statement B", scope="complete", cands={"outcome_owner": ["p1"]},
               outcome_owner=F("binding", ["acting_component", "operator"]))
        fl = flags(run(p, d1, d2))
        self.assertIn(("d1", "outcome_owner", "contradictory"), fl)
        self.assertIn(("d2", "outcome_owner", "changed"), fl)

    def test_any_in_parent_and_derived(self):
        p = P("p1", "DOC-1", "parent requirement one", outcome_owner=F("binding", ["any"]))
        d = D("d1", "DOC-2", "derived statement A", cands={"outcome_owner": ["p1"]}, outcome_owner=F("binding", ["acting_component"]))
        self.assertIn(("d1", "outcome_owner", "stronger"), flags(run(p, d)))

    def test_ambiguous_derived_field_is_not_assessed_not_supported(self):
        p = P("p1", "DOC-1", "parent requirement one", outcome_owner=F("binding", ["acting_component"]))
        d = D("d1", "DOC-2", "derived statement A", scope="complete", cands={"outcome_owner": ["p1"]},
              outcome_owner=F("ambiguous", readings=[["acting_component"], ["any"]]))
        rep = run(p, d)
        self.assertIn(("d1", "outcome_owner", "not_assessed"), flags(rep))
        self.assertNotEqual(rep["pre"]["derived_records"][0]["standing"], "supported")

    def test_parent_unspecified_ambiguous_and_conflict(self):
        p1 = P("p1", "DOC-1", "parent requirement one", outcome_owner=F("ambiguous", readings=[["acting_component"], ["any"]]))
        p2 = P("p2", "DOC-1", "parent requirement two", outcome_owner=F("binding", ["acting_component"]))
        p3 = P("p3", "DOC-1", "parent requirement three", outcome_owner=F("binding", ["managing_system"]))
        a = D("a", "DOC-2", "derived statement A", cands={"outcome_owner": ["p1"]}, outcome_owner=F("binding", ["acting_component"]))
        b = D("b", "DOC-2", "derived statement B", cands={"outcome_owner": []}, outcome_owner=F("binding", ["acting_component"]))
        cc = D("c", "DOC-2", "derived statement C", cands={"outcome_owner": ["p2", "p3"]}, outcome_owner=F("binding", ["acting_component"]))
        fl = flags(run(p1, p2, p3, a, b, cc))
        self.assertIn(("a", "outcome_owner", "parent_ambiguous"), fl)
        self.assertIn(("b", "outcome_owner", "parent_unspecified"), fl)
        self.assertIn(("c", "outcome_owner", "parents_conflict"), fl)

    def test_multi_parent_join_supports_combined_requirement(self):
        h3 = P("h3", "DOC-1", "parent requirement one", acceptance_evidence=F("binding", ["failure_reported_distinctly"]))
        h4 = P("h4", "DOC-1", "parent requirement two", acceptance_evidence=F("binding", ["independent_check"]))
        d = D("d1", "DOC-2", "derived statement A", cands={"acceptance_evidence": ["h3", "h4"]},
              acceptance_evidence=F("binding", ["failure_reported_distinctly", "independent_check"]))
        self.assertEqual(flags(run(h3, h4, d)), set())

    def test_specific_but_insufficient_reference_still_flags_strengthening(self):
        p = P("p1", "DOC-1", "parent requirement one", acceptance_evidence=F("binding", ["outcome_observed"]))
        d = D("d1", "DOC-2", "derived statement A", cands={"acceptance_evidence": ["p1"]},
              acceptance_evidence=F("binding", ["exclusive_causality"]))
        fl = flags(run(p, d))
        self.assertIn(("d1", "acceptance_evidence", "stronger"), fl)
        self.assertNotIn(("d1", None, "deficient_provenance"), fl)

    def test_general_reference_is_deficient_and_still_compared(self):
        p = P("p1", "DOC-1", "parent requirement one", acceptance_evidence=F("binding", ["outcome_observed"]))
        d = D("d1", "DOC-2", "derived statement A", refs=[{"kind": "general", "text": "doctrine section", "target_document": None}],
              cands={"acceptance_evidence": ["p1"]}, acceptance_evidence=F("binding", ["actor_attribution"]))
        fl = flags(run(p, d))
        self.assertIn(("d1", None, "deficient_provenance"), fl)
        self.assertIn(("d1", "acceptance_evidence", "stronger"), fl)

    def test_flags_are_atomic_not_collapsed(self):
        p = P("p1", "DOC-1", "parent requirement one", outcome_owner=F("binding", ["acting_component", "managing_system"]),
              acceptance_evidence=F("binding", ["outcome_observed"]))
        d = D("d1", "DOC-2", "derived statement A", cands={"outcome_owner": ["p1"], "acceptance_evidence": ["p1"]},
              outcome_owner=F("binding", ["acting_component"]), acceptance_evidence=F("binding", ["actor_attribution"]))
        fl = flags(run(p, d))
        self.assertIn(("d1", "outcome_owner", "stronger"), fl)
        self.assertIn(("d1", "acceptance_evidence", "stronger"), fl)

    def test_realization_choice_conforming_and_conflicting(self):
        p = P("p1", "DOC-1", "parent requirement one", outcome_owner=F("binding", ["any"]), verifier=F("binding", ["independent_observer"]))
        ok = D("ok", "DOC-2", "derived statement A", asserted_as="realization_choice", cands={"outcome_owner": ["p1"]},
               outcome_owner=F("selected_realization", ["managing_system"]))
        bad = D("bad", "DOC-2", "derived statement B", asserted_as="realization_choice", scope="complete", cands={"verifier": ["p1"]},
                verifier=F("selected_realization", ["acting_component"]))
        rep = run(p, ok, bad)
        standing = {r["entry"]: r["standing"] for r in rep["pre"]["derived_records"]}
        self.assertEqual(standing["ok"], "realization")
        self.assertEqual(standing["bad"], "realization_conflicts")

    def test_realization_choice_partial_restriction_of_any_is_not_a_conflict(self):
        # Choosing one permitted actor where the parent permits any is ordinary route choice.
        p = P("p1", "DOC-1", "parent requirement one", outcome_owner=F("binding", ["any"]))
        d = D("d1", "DOC-2", "derived statement A", asserted_as="realization_choice", cands={"outcome_owner": ["p1"]},
              outcome_owner=F("selected_realization", ["managing_system"]))
        rep = run(p, d)
        self.assertEqual(rep["pre"]["derived_records"][0]["standing"], "realization")
        self.assertEqual(rep["pre"]["derived_flags"], [])

    def test_partial_realization_choice_outside_permitted_set_conflicts(self):
        p = P("p1", "DOC-1", "parent requirement one", outcome_owner=F("binding", ["managing_system"]))
        d = D("d1", "DOC-2", "derived statement A", asserted_as="realization_choice", cands={"outcome_owner": ["p1"]},
              outcome_owner=F("selected_realization", ["acting_component"]))
        rep = run(p, d)
        self.assertEqual(rep["pre"]["derived_records"][0]["standing"], "realization_conflicts")
        self.assertEqual(rep["pre"]["derived_flags"][0]["detail"], "contradictory")

    def test_realization_choice_weakening_independence_conflicts(self):
        p = P("p1", "DOC-1", "parent requirement one", verifier_independence=F("binding", ["required"]))
        d = D("d1", "DOC-2", "derived statement A", asserted_as="realization_choice", cands={"verifier_independence": ["p1"]},
              verifier_independence=F("selected_realization", ["not_required"]))
        self.assertEqual(run(p, d)["pre"]["derived_records"][0]["standing"], "realization_conflicts")

    def test_proposal_reports_but_does_not_flag(self):
        p = P("p1", "DOC-1", "parent requirement one", acceptance_evidence=F("binding", ["outcome_observed"]))
        d = D("d1", "DOC-2", "derived statement A", asserted_as="proposed", cands={"acceptance_evidence": ["p1"]},
              acceptance_evidence=F("binding", ["exclusive_causality"]))
        rep = run(p, d)
        self.assertEqual(rep["pre"]["derived_flags"], [])
        self.assertEqual(rep["pre"]["derived_records"][0]["standing"], "proposal")


class PlanFlags(unittest.TestCase):
    def test_rules_and_coverage(self):
        a = P("a", "DOC-1", "parent requirement one", outcome_owner=F("ambiguous", readings=[["acting_component"], ["any"]]))
        b = P("b", "DOC-1", "parent requirement two", acceptance_evidence=F("binding", ["outcome_observed"]))
        cc = P("c", "DOC-1", "parent requirement three", acceptance_evidence=F("binding", ["independent_check"]),
               verifier=F("binding", ["independent_observer"]))
        rep = run(a, b, cc, kind="plan")
        got = {(f["entry"], f["flag"]) for f in rep["pre"]["plan_flags"]}
        self.assertEqual(got, {("a", "owner_ambiguous"), ("b", "verifier_unassigned"), ("c", "independence_unstated")})
        self.assertEqual(rep["pre"]["plan_entries_evaluated"], 3)

    def test_unspecified_owner_alone_does_not_flag(self):
        e = P("e", "DOC-1", "parent requirement one", acceptance_evidence=F("binding", ["independent_check"]),
              verifier=F("binding", ["independent_observer"]), verifier_independence=F("binding", ["required"]))
        self.assertEqual(run(e, kind="plan")["pre"]["plan_flags"], [])

    def test_plan_flags_not_computed_for_review_tasks(self):
        a = P("a", "DOC-1", "parent requirement one", outcome_owner=F("ambiguous", readings=[["acting_component"], ["any"]]))
        self.assertNotIn("plan_flags", run(a)["pre"])


class Validation(unittest.TestCase):
    def bad(self, *entries, docs=DOCS):
        with self.assertRaises(c.ExtractionError):
            run(*entries, docs=docs)

    def test_rejects_unquoted_span(self):
        self.bad(P("p", "DOC-1", "not in the document", outcome_owner=F("binding", ["acting_component"])))

    def test_rejects_wrong_document_role(self):
        self.bad(P("p", "DOC-2", "derived statement A", outcome_owner=F("binding", ["acting_component"])))

    def test_rejects_inconsistent_fields(self):
        self.bad(P("p", "DOC-1", "parent requirement one", outcome_owner=F("binding", [])))
        self.bad(P("p", "DOC-1", "parent requirement one", outcome_owner=F("unspecified", ["acting_component"])))
        self.bad(P("p", "DOC-1", "parent requirement one", outcome_owner=F("ambiguous", readings=[["any"]])))
        self.bad(P("p", "DOC-1", "parent requirement one", outcome_owner=F("binding", ["other"])))
        self.bad(P("p", "DOC-1", "parent requirement one", verifier_independence=F("binding", ["required", "not_required"])))
        self.bad(P("p", "DOC-1", "parent requirement one", outcome_owner=F("binding", ["the_host"])))

    def test_rejects_bad_derived_shape(self):
        d = D("d", "DOC-2", "derived statement A", outcome_owner=F("binding", ["acting_component"]))
        d["scope"] = "most"
        self.bad(d)
        d2 = D("d2", "DOC-2", "derived statement A", cands={"outcome_owner": ["missing"]}, outcome_owner=F("binding", ["acting_component"]))
        self.bad(d2)

    def test_rejects_missing_entries_and_wrong_schema(self):
        with self.assertRaises(c.ExtractionError):
            c.report({"schema": SCHEMA}, DOCS, PARENTS, DERIVED, "review")
        with self.assertRaises(c.ExtractionError):
            c.report({"schema": "p7-obligation-extract/v1", "entries": []}, DOCS, PARENTS, DERIVED, "review")


class Fallback(unittest.TestCase):
    def setUp(self):
        self.p = P("p1", "DOC-1", "parent requirement one", outcome_owner=F("ambiguous", readings=[["acting_component"], ["any"]]),
                   acceptance_evidence=F("binding", ["outcome_observed"]))
        self.ext = {"schema": SCHEMA, "entries": [self.p]}
        self.targets = c.fallback_targets(self.ext)

    def test_targets_fixed_from_original(self):
        self.assertEqual([(t["entry"], t["property"]) for t in self.targets], [("p1", "outcome_owner")])

    def test_binding_to_an_original_reading(self):
        rep = run(self.p, kind="plan", fallback=(self.targets, [
            {"entry": "p1", "property": "outcome_owner", "status": "binding", "value": ["acting_component"], "reason": "text names it"}]))
        self.assertEqual(rep["pre"]["plan_flags"][0]["flag"], "owner_ambiguous")
        self.assertNotIn("owner_ambiguous", {f["flag"] for f in rep["post"]["plan_flags"]})

    def test_invented_value_rejected(self):
        rep = run(self.p, kind="plan", fallback=(self.targets, [
            {"entry": "p1", "property": "outcome_owner", "status": "binding", "value": ["operator"], "reason": "guess"}]))
        self.assertTrue(rep["fallback_log"][0].get("invented"))
        self.assertIn("owner_ambiguous", {f["flag"] for f in rep["post"]["plan_flags"]})

    def test_abstention_preserves_readings_and_duplicates_rejected(self):
        responses = [{"entry": "p1", "property": "outcome_owner", "status": "ambiguous", "value": [], "reason": "two readings"},
                     {"entry": "p1", "property": "outcome_owner", "status": "binding", "value": ["any"], "reason": "second try"}]
        rep = run(self.p, kind="plan", fallback=(self.targets, responses))
        reasons = [l.get("reason") or l.get("result") for l in rep["fallback_log"]]
        self.assertIn("duplicate response for target", reasons)
        self.assertIn("abstained", reasons)
        self.assertIn("owner_ambiguous", {f["flag"] for f in rep["post"]["plan_flags"]})

    def test_missing_reason_and_non_target(self):
        rep = run(self.p, kind="plan", fallback=(self.targets, [
            {"entry": "p1", "property": "outcome_owner", "status": "binding", "value": ["any"]},
            {"entry": "p1", "property": "verifier", "status": "binding", "value": ["any"], "reason": "x"}]))
        reasons = [l.get("reason") for l in rep["fallback_log"]]
        self.assertIn("missing reason", reasons)
        self.assertIn("not a fixed target", reasons)


class RealDocuments(unittest.TestCase):
    """Representative extractions over the actual fixture and Case F texts."""

    def test_nc_a_fully_assigned_no_plan_flags(self):
        docs = {"DOC-1": read("fixtures/NC-A-contract.md")}
        e = P("a1", "DOC-1", "the renderer must delete the temporary file it created and record in its own result whether the deletion succeeded",
              initiator=F("binding", ["acting_component"]), outcome_owner=F("binding", ["acting_component"]),
              verifier=F("binding", ["acting_component"]), verifier_independence=F("binding", ["not_required"]),
              acceptance_evidence=F("binding", ["self_report", "failure_reported_distinctly"]))
        rep = c.report({"schema": SCHEMA, "entries": [e]}, docs, {"DOC-1"}, set(), "plan")
        self.assertEqual(rep["pre"]["plan_flags"], [])

    def test_nc_b_owner_explicit_review_supported(self):
        docs = {"DOC-1": read("fixtures/NC-B-contract.md"), "DOC-2": read("fixtures/NC-B-review.md")}
        p = P("b1", "DOC-1", "Clean cancel additionally requires that the Host itself performs the normal removal of the original container and records an affirmative, attributable result for that removal.",
              owner=["receipt-semantics owner"], outcome_owner=F("binding", ["acting_component"]),
              acceptance_evidence=F("binding", ["actor_attribution", "independent_check"]))
        d = D("r1", "DOC-2", "report clean cancel only when the Host's own normal removal succeeded with an attributable result, and the independent post-exit absence check also passed",
              refs=[{"kind": "specific", "text": "in its recorded owner decision", "target_document": "DOC-1"}],
              cands={"outcome_owner": ["b1"], "acceptance_evidence": ["b1"]},
              outcome_owner=F("binding", ["acting_component"]), acceptance_evidence=F("binding", ["actor_attribution", "independent_check"]))
        rep = c.report({"schema": SCHEMA, "entries": [p, d]}, docs, {"DOC-1"}, {"DOC-2"}, "review")
        self.assertEqual(rep["pre"]["derived_flags"], [])

    def test_nc_c_choices_and_unchanged_acceptance(self):
        docs = {"DOC-1": read("fixtures/NC-C-parent.md"), "DOC-2": read("fixtures/NC-C-disposition.md")}
        p = P("c1", "DOC-1", "an independent check, run after the job's process has exited, must confirm that the job's container, identified by its full container ID, is absent",
              outcome_owner=F("unspecified"), verifier=F("binding", ["independent_observer"]),
              verifier_independence=F("binding", ["required"]), acceptance_evidence=F("binding", ["independent_check"]),
              path_fact=F("binding", ["exact_identity", "same_system_instance"]))
        choice = D("k1", "DOC-2", "We will create each job's container with the container engine's automatic-removal option",
                   asserted_as="realization_choice", refs=[], cands={"outcome_owner": ["c1"]},
                   outcome_owner=F("selected_realization", ["managing_system"]))
        acc = D("k2", "DOC-2", "a cancelled job is reported as cleanly cancelled only when the independent post-exit check confirms the container's full ID is absent from the launching engine",
                asserted_as="unchanged", refs=[{"kind": "general", "text": "Unchanged from the cleanup requirement", "target_document": "DOC-1"}],
                cands={"verifier": ["c1"], "verifier_independence": ["c1"], "acceptance_evidence": ["c1"], "path_fact": ["c1"]},
                verifier=F("binding", ["independent_observer"]), verifier_independence=F("binding", ["required"]),
                acceptance_evidence=F("binding", ["independent_check"]), path_fact=F("binding", ["exact_identity", "same_system_instance"]))
        rep = c.report({"schema": SCHEMA, "entries": [p, choice, acc]}, docs, {"DOC-1"}, {"DOC-2"}, "review")
        standing = {r["entry"]: r["standing"] for r in rep["pre"]["derived_records"]}
        self.assertEqual(standing["k1"], "realization")
        fl = {(f["entry"], f["flag"]) for f in rep["pre"]["derived_flags"]}
        self.assertNotIn(("k2", "unchanged_claim_false"), fl)
        # A truthful restatement citing the whole requirement is still recorded as deficient provenance.
        self.assertIn(("k2", "deficient_provenance"), fl)

    def test_case_f_043_span_validates(self):
        docs = {"DOC-1": read("../stage0/case-f/packet/review-r1/043-u4a-held-contract.md")}
        e = P("f1", "DOC-1", "exact container removal observed against the owned identity",
              outcome_owner=F("ambiguous", readings=[["acting_component"], ["any"]]),
              acceptance_evidence=F("binding", ["outcome_observed"]))
        rep = c.report({"schema": SCHEMA, "entries": [e]}, docs, {"DOC-1"}, set(), "plan")
        self.assertIn("owner_ambiguous", {f["flag"] for f in rep["pre"]["plan_flags"]})


if __name__ == "__main__":
    unittest.main()
