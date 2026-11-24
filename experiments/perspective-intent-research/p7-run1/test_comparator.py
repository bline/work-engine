#!/usr/bin/env python3
"""Tests for the P7 run 1 comparator v3.

Run: python3 -I -B -m unittest discover -s experiments/perspective-intent-research/p7-run1 -p 'test_*.py'
"""

import os
import sys
import unittest

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import comparator as c  # noqa: E402

SCHEMA = "obligation-extract/v3"


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
        e["semantics_owner"] = {"status": "binding", "names": owner, "document": doc, "span": span}
    return e


def ref(text="see parent", target="DOC-1", props=None, kind="specific"):
    return {"kind": kind, "text": text, "target_document": target, "properties": props}


def D(id_, doc, span, asserted_as="inherited", scope="partial", refs=None, cands=None, adopter=None,
      baseline=None, align=None, **fields):
    e = {"id": id_, "document": doc, "role": "derived", "span": span, "fields": fields,
         "asserted_as": asserted_as, "scope": scope,
         "references_made": refs if refs is not None else [ref(props=sorted(fields))],
         "support_candidates": cands or {}, "baseline": baseline or []}
    if align:
        e["candidate_alignment"] = align
    if adopter is not None:
        e["adopted_by"] = {"status": "binding", "names": adopter, "document": doc, "span": span}
    return e


# Synthetic documents for the decision-table tests: every quoted string must occur in them.
DOCS = {"DOC-1": "parent requirement one. parent requirement two. parent requirement three. Owner RL; owner dev-vertical.",
        "DOC-2": "derived statement A. derived statement B. derived statement C. see parent. see doctrine section. adopted by the RL."}
PARENTS, DERIVED = {"DOC-1"}, {"DOC-2"}


def run(*entries, kind="review", fallback=None, docs=DOCS, parents=PARENTS, derived=DERIVED, policy=None):
    return c.report({"schema": SCHEMA, "entries": list(entries)}, docs, parents, derived, kind, fallback, policy)


def flags(rep, phase="pre"):
    return {(f["entry"], f.get("property"), f["flag"]) for f in rep[phase]["derived_flags"]}


def record(rep, entry, phase="pre"):
    return next(r for r in rep[phase]["derived_records"] if r["entry"] == entry)


class DecisionTable(unittest.TestCase):
    def test_partial_subset_of_required_items_is_entailed(self):
        p = P("p1", "DOC-1", "parent requirement one", acceptance_evidence=F("binding", ["independent_check", "failure_reported_distinctly"]))
        d = D("d1", "DOC-2", "derived statement A", cands={"acceptance_evidence": ["p1"]},
              acceptance_evidence=F("binding", ["failure_reported_distinctly"]))
        rep = run(p, d)
        self.assertEqual(flags(rep), set())
        self.assertEqual(record(rep, "d1")["properties"]["acceptance_evidence"]["relation"], ["entailed"])
        self.assertEqual(record(rep, "d1")["standing"], "supported")

    def test_added_item_is_stronger_with_values_and_owner_mismatch(self):
        p = P("p1", "DOC-1", "parent requirement one", owner=["RL"], acceptance_evidence=F("binding", ["outcome_observed"]))
        d = D("d1", "DOC-2", "derived statement A", cands={"acceptance_evidence": ["p1"]}, adopter=["dev-vertical"],
              acceptance_evidence=F("binding", ["actor_attribution"]))
        rep = run(p, d)
        fl = flags(rep)
        self.assertIn(("d1", "acceptance_evidence", "stronger"), fl)
        self.assertIn(("d1", "acceptance_evidence", "adopted_without_owner"), fl)
        prop = record(rep, "d1")["properties"]["acceptance_evidence"]
        self.assertEqual((prop["added"], prop["lost"]), (["actor_attribution"], ["outcome_observed"]))
        self.assertEqual(prop["authority"]["outcome"], "mismatched")

    def test_owner_adoption_matching_owner_passes_authority(self):
        p = P("p1", "DOC-1", "parent requirement one", owner=["RL"], acceptance_evidence=F("binding", ["outcome_observed"]))
        d = D("d1", "DOC-2", "derived statement A", cands={"acceptance_evidence": ["p1"]}, adopter=["the RL"],
              acceptance_evidence=F("binding", ["outcome_observed", "actor_attribution"]))
        rep = run(p, d)
        self.assertIn(("d1", "acceptance_evidence", "stronger"), flags(rep))
        self.assertEqual(record(rep, "d1")["properties"]["acceptance_evidence"]["authority"]["outcome"], "matched")
        self.assertNotIn("adopted_without_owner", {f[2] for f in flags(rep)})

    def test_complete_replacement_widening_actors_is_weaker(self):
        p = P("p1", "DOC-1", "parent requirement one", outcome_owner=F("binding", ["acting_component"]))
        d = D("d1", "DOC-2", "derived statement A", scope="complete", baseline=["p1"],
              outcome_owner=F("binding", ["acting_component", "managing_system"]))
        self.assertIn(("d1", "outcome_owner", "weaker"), flags(run(p, d)))

    def test_partial_widening_actors_is_entailed(self):
        p = P("p1", "DOC-1", "parent requirement one", outcome_owner=F("binding", ["acting_component"]))
        d = D("d1", "DOC-2", "derived statement A", cands={"outcome_owner": ["p1"]},
              outcome_owner=F("binding", ["acting_component", "managing_system"]))
        self.assertEqual(flags(run(p, d)), set())

    def test_complete_dropping_a_baseline_property_without_its_candidate_key(self):
        # Review B: the verifier key is omitted from support_candidates; the baseline still enumerates it.
        p = P("p1", "DOC-1", "parent requirement one",
              acceptance_evidence=F("binding", ["independent_check"]), verifier=F("binding", ["independent_observer"]))
        d = D("d1", "DOC-2", "derived statement A", scope="complete", baseline=["p1"],
              cands={"acceptance_evidence": ["p1"]}, acceptance_evidence=F("binding", ["independent_check"]))
        rep = run(p, d)
        self.assertIn(("d1", "verifier", "dropped"), flags(rep))
        self.assertEqual(record(rep, "d1")["standing"], "changed")

    def test_complete_without_baseline_is_not_established(self):
        p = P("p1", "DOC-1", "parent requirement one", acceptance_evidence=F("binding", ["independent_check"]))
        d = D("d1", "DOC-2", "derived statement A", scope="complete", cands={"acceptance_evidence": ["p1"]},
              acceptance_evidence=F("binding", ["independent_check"]))
        rep = run(p, d)
        self.assertIn(("d1", None, "baseline_missing"), flags(rep))
        self.assertEqual(record(rep, "d1")["standing"], "not_established")

    def test_binding_restated_as_revisable_choice_is_status_weakened(self):
        p = P("p1", "DOC-1", "parent requirement one", verifier=F("binding", ["independent_observer"]))
        d = D("d1", "DOC-2", "derived statement A", scope="complete", baseline=["p1"],
              verifier=F("selected_realization", ["independent_observer"]))
        rep = run(p, d)
        self.assertIn(("d1", "verifier", "status_weakened"), flags(rep))
        self.assertNotEqual(record(rep, "d1")["standing"], "supported")

    def test_mixed_add_and_drop_reports_both(self):
        p = P("p1", "DOC-1", "parent requirement one", acceptance_evidence=F("binding", ["independent_check", "failure_reported_distinctly"]))
        d = D("d1", "DOC-2", "derived statement A", scope="complete", baseline=["p1"],
              acceptance_evidence=F("binding", ["independent_check", "actor_attribution"]))
        fl = flags(run(p, d))
        self.assertIn(("d1", "acceptance_evidence", "stronger"), fl)
        self.assertIn(("d1", "acceptance_evidence", "weaker"), fl)

    def test_unchanged_must_be_equal(self):
        p = P("p1", "DOC-1", "parent requirement one", acceptance_evidence=F("binding", ["independent_check", "failure_reported_distinctly"]))
        d = D("d1", "DOC-2", "derived statement A", asserted_as="unchanged", baseline=["p1"],
              acceptance_evidence=F("binding", ["independent_check"]))
        self.assertIn(("d1", None, "unchanged_claim_false"), flags(run(p, d)))

    def test_unchanged_equal_is_supported(self):
        p = P("p1", "DOC-1", "parent requirement one", acceptance_evidence=F("binding", ["independent_check"]))
        d = D("d1", "DOC-2", "derived statement A", asserted_as="unchanged", baseline=["p1"],
              acceptance_evidence=F("binding", ["independent_check"]))
        rep = run(p, d)
        self.assertEqual(flags(rep), set())
        self.assertEqual(record(rep, "d1")["standing"], "supported")

    def test_disjoint_and_overlapping_actor_sets(self):
        p = P("p1", "DOC-1", "parent requirement one", outcome_owner=F("binding", ["acting_component", "managing_system"]))
        d1 = D("d1", "DOC-2", "derived statement A", cands={"outcome_owner": ["p1"]}, outcome_owner=F("binding", ["operator"]))
        d2 = D("d2", "DOC-2", "derived statement B", scope="complete", baseline=["p1"],
               outcome_owner=F("binding", ["acting_component", "operator"]))
        d3 = D("d3", "DOC-2", "derived statement C", cands={"outcome_owner": ["p1"]},
               outcome_owner=F("binding", ["acting_component", "operator"]))
        rep = run(p, d1, d2, d3)
        fl = flags(rep)
        self.assertIn(("d1", "outcome_owner", "contradictory"), fl)
        self.assertIn(("d2", "outcome_owner", "stronger"), fl)
        self.assertIn(("d2", "outcome_owner", "weaker"), fl)
        # Partial: excluding the managing system flags; admitting the operator is entailed.
        self.assertIn(("d3", "outcome_owner", "stronger"), fl)
        self.assertNotIn(("d3", "outcome_owner", "weaker"), fl)
        prop = record(rep, "d3")["properties"]["outcome_owner"]
        self.assertEqual((prop["lost"], prop["added"]), (["managing_system"], ["operator"]))
        self.assertEqual(prop["relation"], ["entailed", "stronger"])

    def test_any_in_parent_and_derived(self):
        p = P("p1", "DOC-1", "parent requirement one", outcome_owner=F("binding", ["any"]))
        d = D("d1", "DOC-2", "derived statement A", cands={"outcome_owner": ["p1"]}, outcome_owner=F("binding", ["acting_component"]))
        self.assertIn(("d1", "outcome_owner", "stronger"), flags(run(p, d)))

    def test_ambiguous_derived_field_is_not_assessed_not_supported(self):
        p = P("p1", "DOC-1", "parent requirement one", outcome_owner=F("binding", ["acting_component"]))
        d = D("d1", "DOC-2", "derived statement A", scope="complete", baseline=["p1"],
              outcome_owner=F("ambiguous", readings=[["acting_component"], ["any"]]))
        rep = run(p, d)
        self.assertIn(("d1", "outcome_owner", "not_assessed"), flags(rep))
        self.assertEqual(record(rep, "d1")["standing"], "not_established")

    def test_partial_entry_with_only_ambiguous_fields_is_never_supported(self):
        # Review B: the partial counterexample. Previously this reached 'supported'.
        p = P("p1", "DOC-1", "parent requirement one", outcome_owner=F("binding", ["acting_component"]))
        d = D("d1", "DOC-2", "derived statement A", refs=[], cands={"outcome_owner": ["p1"]},
              outcome_owner=F("ambiguous", readings=[["acting_component"], ["managing_system"]]))
        empty = D("d2", "DOC-2", "derived statement B", refs=[], cands={"outcome_owner": ["p1"]})
        rep = run(p, d, empty)
        self.assertEqual(record(rep, "d1")["standing"], "not_established")
        self.assertEqual(record(rep, "d1")["coverage"], {"properties": 1, "established": 0})
        self.assertEqual(record(rep, "d2")["standing"], "not_assessed")

    def test_parent_unspecified_ambiguous_conflict_and_alignment(self):
        p1 = P("p1", "DOC-1", "parent requirement one", outcome_owner=F("ambiguous", readings=[["acting_component"], ["any"]]))
        p2 = P("p2", "DOC-1", "parent requirement two", outcome_owner=F("binding", ["acting_component"]))
        p3 = P("p3", "DOC-1", "parent requirement three", outcome_owner=F("binding", ["managing_system"]))
        a = D("a", "DOC-2", "derived statement A", cands={"outcome_owner": ["p1", "p2"]}, outcome_owner=F("binding", ["acting_component"]))
        b = D("b", "DOC-2", "derived statement B", cands={"outcome_owner": []}, outcome_owner=F("binding", ["acting_component"]))
        cc = D("c", "DOC-2", "derived statement C", cands={"outcome_owner": ["p2", "p3"]}, align={"outcome_owner": "same_condition"},
               outcome_owner=F("binding", ["acting_component"]))
        dd = D("d", "DOC-2", "derived statement C", cands={"outcome_owner": ["p2", "p3"]}, outcome_owner=F("binding", ["acting_component"]))
        rep = run(p1, p2, p3, a, b, cc, dd)
        fl = flags(rep)
        # A binding candidate does not hide an ambiguous one.
        self.assertIn(("a", "outcome_owner", "parent_ambiguous"), fl)
        self.assertIn(("b", "outcome_owner", "parent_unspecified"), fl)
        self.assertIn(("c", "outcome_owner", "parents_conflict"), fl)
        self.assertIn(("d", "outcome_owner", "alignment_unresolved"), fl)
        self.assertEqual(record(rep, "b")["standing"], "changed")
        self.assertEqual(record(rep, "b")["properties"]["outcome_owner"]["authority"]["outcome"], "not_established")

    def test_multi_parent_join_needs_declared_alignment(self):
        h3 = P("h3", "DOC-1", "parent requirement one", acceptance_evidence=F("binding", ["failure_reported_distinctly"]))
        h4 = P("h4", "DOC-1", "parent requirement two", acceptance_evidence=F("binding", ["independent_check"]))
        joint = D("d1", "DOC-2", "derived statement A", cands={"acceptance_evidence": ["h3", "h4"]},
                  align={"acceptance_evidence": "same_condition"},
                  acceptance_evidence=F("binding", ["failure_reported_distinctly", "independent_check"]))
        loose = D("d2", "DOC-2", "derived statement B", cands={"acceptance_evidence": ["h3", "h4"]},
                  acceptance_evidence=F("binding", ["failure_reported_distinctly", "independent_check"]))
        rep = run(h3, h4, joint, loose)
        self.assertEqual(record(rep, "d1")["standing"], "supported")
        self.assertEqual(record(rep, "d2")["standing"], "not_established")

    def test_specific_but_insufficient_reference_still_flags_strengthening(self):
        p = P("p1", "DOC-1", "parent requirement one", acceptance_evidence=F("binding", ["outcome_observed"]))
        d = D("d1", "DOC-2", "derived statement A", cands={"acceptance_evidence": ["p1"]},
              acceptance_evidence=F("binding", ["exclusive_causality"]))
        fl = flags(run(p, d))
        self.assertIn(("d1", "acceptance_evidence", "stronger"), fl)
        self.assertNotIn(("d1", "acceptance_evidence", "deficient_provenance"), fl)

    def test_general_reference_is_deficient_and_still_compared(self):
        p = P("p1", "DOC-1", "parent requirement one", acceptance_evidence=F("binding", ["outcome_observed"]))
        d = D("d1", "DOC-2", "derived statement A", refs=[ref("doctrine section", None, ["acceptance_evidence"], "general")],
              cands={"acceptance_evidence": ["p1"]}, acceptance_evidence=F("binding", ["actor_attribution"]))
        fl = flags(run(p, d))
        self.assertIn(("d1", "acceptance_evidence", "deficient_provenance"), fl)
        self.assertIn(("d1", "acceptance_evidence", "stronger"), fl)

    def test_citation_is_per_property(self):
        # Review B: one specific citation no longer clears every field.
        p = P("p1", "DOC-1", "parent requirement one", acceptance_evidence=F("binding", ["independent_check"]),
              verifier=F("binding", ["independent_observer"]))
        d = D("d1", "DOC-2", "derived statement A", refs=[ref(props=["acceptance_evidence"])],
              cands={"acceptance_evidence": ["p1"], "verifier": ["p1"]},
              acceptance_evidence=F("binding", ["independent_check"]), verifier=F("binding", ["independent_observer"]))
        rep = run(p, d)
        fl = flags(rep)
        self.assertIn(("d1", "verifier", "deficient_provenance"), fl)
        self.assertNotIn(("d1", "acceptance_evidence", "deficient_provenance"), fl)
        # Citation is reported apart from entailment: the entry is still supported.
        self.assertEqual(record(rep, "d1")["standing"], "supported")
        self.assertEqual(record(rep, "d1")["citation"], ["adequate", "none"])

    def test_specific_reference_to_a_non_parent_is_unresolved(self):
        p = P("p1", "DOC-1", "parent requirement one", acceptance_evidence=F("binding", ["independent_check"]))
        d = D("d1", "DOC-2", "derived statement A", refs=[ref(target="DOC-2", props=["acceptance_evidence"])],
              cands={"acceptance_evidence": ["p1"]}, acceptance_evidence=F("binding", ["independent_check"]))
        self.assertEqual(record(run(p, d), "d1")["properties"]["acceptance_evidence"]["citation"], "unresolved_target")

    def test_flags_are_atomic_not_collapsed(self):
        p = P("p1", "DOC-1", "parent requirement one", outcome_owner=F("binding", ["acting_component", "managing_system"]),
              acceptance_evidence=F("binding", ["outcome_observed"]))
        d = D("d1", "DOC-2", "derived statement A", cands={"outcome_owner": ["p1"], "acceptance_evidence": ["p1"]},
              outcome_owner=F("binding", ["acting_component"]), acceptance_evidence=F("binding", ["actor_attribution"]))
        fl = flags(run(p, d))
        self.assertIn(("d1", "outcome_owner", "stronger"), fl)
        self.assertIn(("d1", "acceptance_evidence", "stronger"), fl)

    def test_proposal_reports_but_does_not_flag(self):
        p = P("p1", "DOC-1", "parent requirement one", acceptance_evidence=F("binding", ["outcome_observed"]))
        d = D("d1", "DOC-2", "derived statement A", asserted_as="proposed", cands={"acceptance_evidence": ["p1"]},
              acceptance_evidence=F("binding", ["exclusive_causality"]))
        rep = run(p, d)
        self.assertEqual(rep["pre"]["derived_flags"], [])
        self.assertEqual(record(rep, "d1")["standing"], "proposal")
        self.assertEqual(record(rep, "d1")["entailment"], "changed")


class Authority(unittest.TestCase):
    def test_owner_is_checked_per_changed_property_not_pooled(self):
        # Review C: RL owns the evidence; dev-vertical owns an unchanged property. Pooling cleared it.
        ev = P("p1", "DOC-1", "parent requirement one", owner=["RL"], acceptance_evidence=F("binding", ["outcome_observed"]))
        host = P("p2", "DOC-1", "parent requirement two", owner=["dev-vertical"], outcome_owner=F("binding", ["acting_component"]))
        d = D("d1", "DOC-2", "derived statement A", adopter=["dev-vertical"],
              cands={"acceptance_evidence": ["p1"], "outcome_owner": ["p2"]},
              acceptance_evidence=F("binding", ["outcome_observed", "actor_attribution"]),
              outcome_owner=F("binding", ["acting_component"]))
        rep = run(ev, host, d)
        self.assertIn(("d1", "acceptance_evidence", "adopted_without_owner"), flags(rep))
        self.assertNotIn("authority", record(rep, "d1")["properties"]["outcome_owner"])

    def test_unknown_owner_elsewhere_does_not_force_a_flag(self):
        ev = P("p1", "DOC-1", "parent requirement one", owner=["RL"], acceptance_evidence=F("binding", ["outcome_observed"]))
        other = P("p2", "DOC-1", "parent requirement two", verifier=F("binding", ["independent_observer"]))
        d = D("d1", "DOC-2", "derived statement A", adopter=["RL"], cands={"acceptance_evidence": ["p1"], "verifier": ["p2"]},
              acceptance_evidence=F("binding", ["outcome_observed", "actor_attribution"]), verifier=F("binding", ["independent_observer"]))
        rep = run(ev, other, d)
        self.assertEqual(record(rep, "d1")["properties"]["acceptance_evidence"]["authority"]["outcome"], "matched")
        self.assertFalse({f for f in flags(rep) if f[2] in ("adopted_without_owner", "authority_not_established")})

    def test_no_adopter_is_decision_not_shown_not_a_mismatch(self):
        ev = P("p1", "DOC-1", "parent requirement one", owner=["RL"], acceptance_evidence=F("binding", ["outcome_observed"]))
        d = D("d1", "DOC-2", "derived statement A", cands={"acceptance_evidence": ["p1"]},
              acceptance_evidence=F("binding", ["outcome_observed", "actor_attribution"]))
        fl = flags(run(ev, d))
        self.assertIn(("d1", "acceptance_evidence", "owner_decision_not_shown"), fl)
        self.assertNotIn(("d1", "acceptance_evidence", "adopted_without_owner"), fl)


class Realization(unittest.TestCase):
    def test_conforming_and_conflicting(self):
        p = P("p1", "DOC-1", "parent requirement one", outcome_owner=F("binding", ["any"]), verifier=F("binding", ["independent_observer"]))
        ok = D("ok", "DOC-2", "derived statement A", asserted_as="realization_choice", refs=[], cands={"outcome_owner": ["p1"]},
               outcome_owner=F("selected_realization", ["managing_system"]))
        bad = D("bad", "DOC-2", "derived statement B", asserted_as="realization_choice", refs=[], cands={"verifier": ["p1"]},
                verifier=F("selected_realization", ["acting_component"]))
        rep = run(p, ok, bad)
        self.assertEqual(record(rep, "ok")["standing"], "realization")
        self.assertEqual(record(rep, "bad")["standing"], "realization_conflicts")
        self.assertEqual(rep["pre"]["derived_flags"][0]["detail"], "contradictory")

    def test_partial_restriction_of_any_is_not_a_conflict(self):
        p = P("p1", "DOC-1", "parent requirement one", outcome_owner=F("binding", ["any"]))
        d = D("d1", "DOC-2", "derived statement A", asserted_as="realization_choice", refs=[], cands={"outcome_owner": ["p1"]},
              outcome_owner=F("selected_realization", ["managing_system"]))
        rep = run(p, d)
        self.assertEqual(record(rep, "d1")["standing"], "realization")
        self.assertEqual(rep["pre"]["derived_flags"], [])

    def test_weakening_independence_conflicts(self):
        p = P("p1", "DOC-1", "parent requirement one", verifier_independence=F("binding", ["required"]))
        d = D("d1", "DOC-2", "derived statement A", asserted_as="realization_choice", refs=[], cands={"verifier_independence": ["p1"]},
              verifier_independence=F("selected_realization", ["not_required"]))
        self.assertEqual(record(run(p, d), "d1")["standing"], "realization_conflicts")

    def test_no_candidates_and_ambiguous_parent_are_not_conflicts(self):
        # Review B: unlinked or ambiguous comparisons establish neither support nor conflict.
        p = P("p1", "DOC-1", "parent requirement one", outcome_owner=F("ambiguous", readings=[["acting_component"], ["managing_system"]]))
        unlinked = D("u", "DOC-2", "derived statement A", asserted_as="realization_choice", refs=[],
                     outcome_owner=F("selected_realization", ["managing_system"]))
        amb = D("a", "DOC-2", "derived statement B", asserted_as="realization_choice", refs=[], cands={"outcome_owner": ["p1"]},
                outcome_owner=F("selected_realization", ["managing_system"]))
        rep = run(p, unlinked, amb)
        self.assertEqual(record(rep, "u")["standing"], "realization_not_established")
        self.assertEqual(record(rep, "a")["standing"], "realization_not_established")
        self.assertNotIn("realization_conflicts", {f[2] for f in flags(rep)})

    def test_binding_demand_inside_a_choice_is_compared_as_a_requirement(self):
        p = P("p1", "DOC-1", "parent requirement one", acceptance_evidence=F("binding", ["outcome_observed"]),
              outcome_owner=F("binding", ["any"]))
        d = D("d1", "DOC-2", "derived statement A", asserted_as="realization_choice", refs=[ref(props=["acceptance_evidence"])],
              cands={"acceptance_evidence": ["p1"], "outcome_owner": ["p1"]},
              outcome_owner=F("selected_realization", ["managing_system"]),
              acceptance_evidence=F("binding", ["outcome_observed", "exclusive_causality"]))
        rep = run(p, d)
        self.assertIn(("d1", "acceptance_evidence", "stronger"), flags(rep))
        self.assertEqual(record(rep, "d1")["standing"], "realization_conflicts")

    def test_choice_with_nothing_compared_is_not_assessed(self):
        d = D("d1", "DOC-2", "derived statement A", asserted_as="realization_choice", refs=[])
        self.assertEqual(record(run(d), "d1")["standing"], "realization_not_assessed")


class PlanFlags(unittest.TestCase):
    def kinds(self, *entries):
        return {(f["entry"], f["flag"]) for f in run(*entries, kind="plan")["pre"]["plan_flags"]}

    def test_rules_and_count(self):
        a = P("a", "DOC-1", "parent requirement one", outcome_owner=F("ambiguous", readings=[["acting_component"], ["any"]]))
        b = P("b", "DOC-1", "parent requirement two", outcome_owner=F("binding", ["acting_component"]),
              acceptance_evidence=F("binding", ["outcome_observed"]))
        rep = run(a, b, kind="plan")
        got = {(f["entry"], f["flag"]) for f in rep["pre"]["plan_flags"]}
        self.assertEqual(got, {("a", "owner_unresolved"), ("b", "verifier_unassigned"), ("b", "independence_unstated")})
        self.assertEqual(rep["pre"]["plan_entries_extracted"], 2)

    def test_honest_unspecified_typings_fire_e1_and_both_e7_flags(self):
        # Review C: F-043 typed with an unspecified owner; B-1539 with verifier and independence unstated.
        owner = P("o", "DOC-1", "parent requirement one", acceptance_evidence=F("binding", ["outcome_observed"]),
                  verifier=F("binding", ["any"]), verifier_independence=F("binding", ["not_required"]))
        stop = P("s", "DOC-1", "parent requirement two", outcome_owner=F("binding", ["managing_system"]),
                 acceptance_evidence=F("binding", ["outcome_observed"]))
        self.assertEqual(self.kinds(owner, stop), {("o", "owner_unresolved"), ("s", "verifier_unassigned"),
                                                   ("s", "independence_unstated")})

    def test_deliberately_open_owner_and_full_controls_do_not_flag(self):
        open_ = P("e", "DOC-1", "parent requirement one", outcome_owner=F("binding", ["any"]),
                  acceptance_evidence=F("binding", ["independent_check"]),
                  verifier=F("binding", ["independent_observer"]), verifier_independence=F("binding", ["required"]))
        self_report = P("n", "DOC-1", "parent requirement two", outcome_owner=F("binding", ["acting_component"]),
                        acceptance_evidence=F("binding", ["self_report"]),
                        verifier=F("binding", ["acting_component"]), verifier_independence=F("binding", ["not_required"]))
        self.assertEqual(self.kinds(open_, self_report), set())

    def test_plan_flags_not_computed_for_review_tasks(self):
        a = P("a", "DOC-1", "parent requirement one", outcome_owner=F("ambiguous", readings=[["acting_component"], ["any"]]))
        self.assertNotIn("plan_flags", run(a)["pre"])


class Validation(unittest.TestCase):
    def bad(self, *entries, docs=DOCS):
        with self.assertRaises(c.ExtractionError):
            run(*entries, docs=docs)

    def test_rejects_unquoted_span_with_recorded_failures(self):
        with self.assertRaises(c.ExtractionError) as cm:
            run(P("p", "DOC-1", "not in the document", outcome_owner=F("binding", ["acting_component"])))
        self.assertEqual(cm.exception.quote_failures[0]["where"], "span")

    def test_record_policy_reports_quote_failures_without_raising(self):
        rep = run(P("p", "DOC-1", "not in the document", outcome_owner=F("binding", ["acting_component"])), policy="record")
        self.assertEqual(len(rep["quote_failures"]), 1)

    def test_every_relied_on_quote_is_checked(self):
        p = P("p1", "DOC-1", "parent requirement one", acceptance_evidence=F("binding", ["outcome_observed"]))
        self.bad(p, D("d", "DOC-2", "derived statement A", refs=[ref("a citation never written", props=["acceptance_evidence"])],
                      cands={"acceptance_evidence": ["p1"]}, acceptance_evidence=F("binding", ["outcome_observed"])))
        owned = P("p2", "DOC-1", "parent requirement two", acceptance_evidence=F("binding", ["outcome_observed"]))
        owned["semantics_owner"] = {"status": "binding", "names": ["RL"], "document": "DOC-1", "span": "an invented owner line"}
        self.bad(owned)
        unnamed = P("p3", "DOC-1", "parent requirement three", owner=["Research Lifecycle"],
                    acceptance_evidence=F("binding", ["outcome_observed"]))
        self.bad(unnamed)
        self.bad(P("p4", "DOC-1", "parent requirement one", outcome_owner=F("binding", ["other"], other_text="not quoted")))

    def test_quote_normalization_is_named_and_bounded(self):
        docs = {"DOC-1": "Derive it with a **positively attained normal-removal witness**, and the \u201cHost\u201d removes\n   it."}
        ok = P("p", "DOC-1", "a positively attained normal-removal witness, and the \"Host\" removes it.",
               outcome_owner=F("binding", ["acting_component"]))
        run(ok, docs=docs, parents={"DOC-1"}, derived=set())
        for bad_span in ("Derive it ... witness", "positively attained witness"):
            with self.assertRaises(c.ExtractionError):
                run(P("p", "DOC-1", bad_span, outcome_owner=F("binding", ["acting_component"])), docs=docs,
                    parents={"DOC-1"}, derived=set())

    def test_rejects_wrong_document_role(self):
        self.bad(P("p", "DOC-2", "derived statement A", outcome_owner=F("binding", ["acting_component"])))

    def test_rejects_inconsistent_fields(self):
        span = "parent requirement one"
        self.bad(P("p", "DOC-1", span, outcome_owner=F("binding", [])))
        self.bad(P("p", "DOC-1", span, outcome_owner=F("unspecified", ["acting_component"])))
        self.bad(P("p", "DOC-1", span, outcome_owner=F("ambiguous", readings=[["any"]])))
        self.bad(P("p", "DOC-1", span, outcome_owner=F("ambiguous", readings=[["any"], ["any"]])))
        self.bad(P("p", "DOC-1", span, outcome_owner=F("binding", ["any", "operator"])))
        self.bad(P("p", "DOC-1", span, outcome_owner=F("binding", ["other"])))
        self.bad(P("p", "DOC-1", span, verifier_independence=F("binding", ["required", "not_required"])))
        self.bad(P("p", "DOC-1", span, outcome_owner=F("binding", ["the_host"])))

    def test_malformed_json_values_are_extraction_errors_not_crashes(self):
        span = "parent requirement one"
        self.bad(P("p", "DOC-1", span, outcome_owner={"status": "binding", "value": [["acting_component"]]}))
        self.bad(P("p", "DOC-1", span, outcome_owner={"status": ["binding"], "value": ["acting_component"]}))
        bad_owner = P("p", "DOC-1", span, acceptance_evidence=F("binding", ["outcome_observed"]))
        bad_owner["semantics_owner"] = {"status": "binding", "names": [{"n": 1}], "document": "DOC-1", "span": span}
        self.bad(bad_owner)
        self.bad({"id": ["x"], "document": "DOC-1", "role": "parent", "span": span, "fields": {}})

    def test_rejects_bad_derived_shape(self):
        d = D("d", "DOC-2", "derived statement A", outcome_owner=F("binding", ["acting_component"]))
        d["scope"] = "most"
        self.bad(d)
        self.bad(D("d2", "DOC-2", "derived statement A", cands={"outcome_owner": ["missing"]}, outcome_owner=F("binding", ["acting_component"])))
        self.bad(D("d3", "DOC-2", "derived statement A", scope="complete", baseline=["missing"]))
        self.bad(D("d4", "DOC-2", "derived statement A", refs=[{"kind": "specific", "text": "see parent", "target_document": "DOC-1"}]))
        self.bad(D("d5", "DOC-2", "derived statement A", align={"verifier": "probably"}))

    def test_rejects_missing_entries_and_wrong_schema(self):
        with self.assertRaises(c.ExtractionError):
            c.report({"schema": SCHEMA}, DOCS, PARENTS, DERIVED, "review")
        with self.assertRaises(c.ExtractionError):
            c.report({"schema": "obligation-extract/v2", "entries": []}, DOCS, PARENTS, DERIVED, "review")


class Fallback(unittest.TestCase):
    def setUp(self):
        self.p = P("p1", "DOC-1", "parent requirement one", outcome_owner=F("ambiguous", readings=[["acting_component"], ["any"]]),
                   acceptance_evidence=F("binding", ["outcome_observed"]))
        self.q = P("p2", "DOC-1", "parent requirement two", verifier=F("ambiguous", readings=[["acting_component"], ["independent_observer"]]))
        self.ext = {"schema": SCHEMA, "entries": [self.p, self.q]}
        self.targets = c.fallback_targets(self.ext)

    def go(self, *outcomes):
        return run(self.p, self.q, kind="plan", fallback=(self.targets, list(outcomes)))

    @staticmethod
    def ok(entry, prop, status, value, reason="the text names it"):
        return {"status": "valid", "response": {"entry": entry, "property": prop, "status": status, "value": value, "reason": reason}}

    def test_targets_fixed_from_original(self):
        self.assertEqual([(t["entry"], t["property"]) for t in self.targets], [("p1", "outcome_owner"), ("p2", "verifier")])

    def test_resolution_within_list_records_before_and_after(self):
        rep = self.go(self.ok("p1", "outcome_owner", "binding", ["acting_component"]), self.ok("p2", "verifier", "ambiguous", []))
        self.assertIn("owner_unresolved", {f["flag"] for f in rep["pre"]["plan_flags"]})
        self.assertNotIn("owner_unresolved", {f["flag"] for f in rep["post"]["plan_flags"]})
        row = rep["fallback_log"][0]
        self.assertEqual((row["outcome"], row["after"]["value"]), ("resolved_within_list", ["acting_component"]))
        self.assertEqual(row["before"]["readings"], [["acting_component"], ["any"]])
        self.assertEqual(rep["fallback_log"][1]["outcome"], "abstained")

    def test_out_of_list_value_is_an_invention(self):
        rep = self.go(self.ok("p1", "outcome_owner", "binding", ["operator"]), None)
        self.assertTrue(rep["fallback_log"][0]["invented"])
        self.assertEqual(rep["fallback_log"][1]["outcome"], "missing")
        self.assertIn("owner_unresolved", {f["flag"] for f in rep["post"]["plan_flags"]})

    def test_reply_for_another_target_is_invalid(self):
        # Review F: an answer for target 1 recorded at index 0 must not resolve anything.
        rep = self.go(self.ok("p2", "verifier", "binding", ["independent_observer"]),
                      self.ok("p2", "verifier", "binding", ["independent_observer"]))
        self.assertEqual(rep["fallback_log"][0]["outcome"], "invalid_response")
        self.assertEqual(rep["fallback_log"][1]["outcome"], "resolved_within_list")

    def test_malformed_replies_are_invalid_not_crashes(self):
        bad = [{"status": "valid", "response": []},
               self.ok("p1", "outcome_owner", "binding", ["any"], reason=None),
               self.ok("p1", "outcome_owner", "ambiguous", ["any"]),
               self.ok("p1", "outcome_owner", "maybe", ["any"]),
               {"status": "invalid_json", "error": "Expecting value"}]
        for o in bad:
            rep = self.go(o, None)
            self.assertEqual(rep["fallback_log"][0]["outcome"], "invalid_response", o)

    def test_outcomes_must_pair_with_targets(self):
        with self.assertRaises(c.ExtractionError):
            self.go(None)


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
        span = "Clean cancel additionally requires that the Host itself performs the normal removal of the original container and records an affirmative, attributable result for that removal."
        p = P("b1", "DOC-1", span, owner=["receipt-semantics owner"], outcome_owner=F("binding", ["acting_component"]),
              acceptance_evidence=F("binding", ["actor_attribution", "independent_check"]))
        p["semantics_owner"]["span"] = "Owner decision (receipt-semantics owner, recorded in this contract)"
        d = D("r1", "DOC-2", "report clean cancel only when the Host's own normal removal succeeded with an attributable result, and the independent post-exit absence check also passed",
              refs=[ref("in its recorded owner decision", "DOC-1", ["outcome_owner", "acceptance_evidence"])],
              cands={"outcome_owner": ["b1"], "acceptance_evidence": ["b1"]},
              outcome_owner=F("binding", ["acting_component"]), acceptance_evidence=F("binding", ["actor_attribution", "independent_check"]))
        rep = c.report({"schema": SCHEMA, "entries": [p, d]}, docs, {"DOC-1"}, {"DOC-2"}, "review")
        self.assertEqual(rep["pre"]["derived_flags"], [])
        self.assertEqual(record(rep, "r1")["standing"], "supported")

    def test_nc_c_choices_and_unchanged_acceptance(self):
        docs = {"DOC-1": read("fixtures/NC-C-parent.md"), "DOC-2": read("fixtures/NC-C-disposition.md")}
        p = P("c1", "DOC-1", "an independent check, run after the job's process has exited, must confirm that the job's container, identified by its full container ID, is absent",
              outcome_owner=F("unspecified"), verifier=F("binding", ["independent_observer"]),
              verifier_independence=F("binding", ["required"]), acceptance_evidence=F("binding", ["independent_check"]),
              path_fact=F("binding", ["exact_identity", "same_system_instance"]))
        choice = D("k1", "DOC-2", "We will create each job's container with the container engine's automatic-removal option",
                   asserted_as="realization_choice", refs=[], cands={"outcome_owner": ["c1"]},
                   outcome_owner=F("selected_realization", ["managing_system"]))
        props = ["verifier", "verifier_independence", "acceptance_evidence", "path_fact"]
        acc = D("k2", "DOC-2", "a cancelled job is reported as cleanly cancelled only when the independent post-exit check confirms the container's full ID is absent from the launching engine",
                asserted_as="unchanged", refs=[ref("Unchanged from the cleanup requirement", "DOC-1", props, "general")],
                baseline=["c1"], verifier=F("binding", ["independent_observer"]), verifier_independence=F("binding", ["required"]),
                acceptance_evidence=F("binding", ["independent_check"]), path_fact=F("binding", ["exact_identity", "same_system_instance"]))
        rep = c.report({"schema": SCHEMA, "entries": [p, choice, acc]}, docs, {"DOC-1"}, {"DOC-2"}, "review")
        self.assertEqual(record(rep, "k1")["standing"], "realization")
        fl = {(f["entry"], f["flag"]) for f in rep["pre"]["derived_flags"]}
        self.assertNotIn(("k2", "unchanged_claim_false"), fl)
        # Review B: a truthful restatement citing the whole requirement is supported; the general
        # citation is reported separately and does not make the entry 'flagged'.
        self.assertEqual(record(rep, "k2")["standing"], "supported")
        self.assertIn(("k2", "deficient_provenance"), fl)

    def test_case_f_043_span_validates(self):
        docs = {"DOC-1": read("../stage0/case-f/packet/review-r1/043-u4a-held-contract.md")}
        e = P("f1", "DOC-1", "exact container removal observed against the owned identity",
              outcome_owner=F("ambiguous", readings=[["acting_component"], ["any"]]),
              acceptance_evidence=F("binding", ["outcome_observed"]))
        rep = c.report({"schema": SCHEMA, "entries": [e]}, docs, {"DOC-1"}, set(), "plan")
        self.assertIn("owner_unresolved", {f["flag"] for f in rep["pre"]["plan_flags"]})

    def test_case_f_amendment_markdown_span_validates(self):
        docs = {"DOC-1": read("../stage0/case-f/packet/QUALIFICATION-AMENDMENT.md")}
        e = P("f2", "DOC-1", "including a positively attained normal-removal witness",
              acceptance_evidence=F("binding", ["actor_attribution"]))
        c.report({"schema": SCHEMA, "entries": [e]}, docs, {"DOC-1"}, set(), "plan")


if __name__ == "__main__":
    unittest.main()
