#!/usr/bin/env python3
"""Unit tests for the P7 run 1 comparator prototype (hand-typed inputs).

Run: python3 -I -m unittest discover -s experiments/perspective-intent-research/p7-run1 -p 'test_*.py'
"""

import os
import sys
import unittest

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import comparator as c  # noqa: E402

SCHEMA = "p7-obligation-extract/v1"


def field(status, value=(), readings=None):
    f = {"status": status, "value": list(value)}
    if readings is not None:
        f["readings"] = [list(r) for r in readings]
    return f


def parent(id_, doc, **fields):
    return {"id": id_, "document": doc, "role": "parent", "span": f"span of {id_}", "fields": fields}


def derived(id_, doc, asserted_as, cites, replaces=False, **fields):
    e = {"id": id_, "document": doc, "role": "derived", "span": f"span of {id_}", "fields": fields,
         "asserted_as": asserted_as, "cites": cites}
    if replaces:
        e["replaces_parent"] = True
    return e


def ext(*entries):
    return {"schema": SCHEMA, "entries": list(entries)}


def spec(pid):
    return {"kind": "specific", "parent_entry": pid}


# 043 terminal cleanup, typed two ways (protocol section 7, typed methods).
P043_AMBIGUOUS = parent("F-043#cleanup", "F-043",
                        initiator=field("binding", ["host"]),
                        outcome_owner=field("ambiguous", readings=[["host"], ["any"]]),
                        verifier=field("unspecified"),
                        acceptance_evidence=field("binding", ["outcome_observed"]))
P043_BINDING_HOST = parent("F-043#cleanup", "F-043",
                           initiator=field("binding", ["host"]),
                           outcome_owner=field("binding", ["host"]),
                           verifier=field("binding", ["host"]),
                           acceptance_evidence=field("binding", ["outcome_observed", "actor_attributed_removal"]))
PLAN_H3 = parent("F-PLAN#H3", "F-PLAN",
                 acceptance_evidence=field("binding", ["removal_failure_reported_as_failure", "independent_absence"]),
                 outcome_owner=field("selected_realization", ["host"]),
                 verifier=field("binding", ["fixture"]))
E3 = derived("F-RE#affirmative", "F-RE", "inherited", spec("F-043#cleanup"),
             outcome_owner=field("binding", ["host"]),
             acceptance_evidence=field("binding", ["actor_attributed_removal"]))


def disp(rep, entry_id):
    return next(d for d in rep["derived"] if d["entry"] == entry_id)


class PlanTime(unittest.TestCase):
    def test_043_ambiguous_owner_is_flagged(self):
        rep = c.report(ext(P043_AMBIGUOUS))
        self.assertEqual([f["entry"] for f in rep["plan_flags"]], ["F-043#cleanup"])
        self.assertEqual(rep["plan_flags"][0]["properties"], ["outcome_owner", "verifier"])

    def test_nc_a_actor_owns_its_outcome(self):
        nc_a = parent("NC-A#1", "NC-A", initiator=field("binding", ["component"]),
                      outcome_owner=field("binding", ["component"]), verifier=field("binding", ["component"]),
                      acceptance_evidence=field("binding", ["self_report"]))
        self.assertEqual(c.report(ext(nc_a))["plan_flags"], [])

    def test_t2_decision_record_separates_roles(self):
        t2 = parent("B-DECREC#2", "B-DECREC", initiator=field("binding", ["termination_actor"]),
                    outcome_owner=field("binding", ["termination_actor"]),
                    verifier=field("binding", ["independent_observer"]),
                    acceptance_evidence=field("binding", ["independent_termination_verification"]))
        self.assertEqual(c.report(ext(t2))["plan_flags"], [])

    def test_t1_unnamed_verifier_is_flagged(self):
        t1 = parent("B-1539#1", "B-1539", outcome_owner=field("unspecified"), verifier=field("unspecified"),
                    acceptance_evidence=field("binding", ["independent_termination_verification"]))
        self.assertEqual(len(c.report(ext(t1))["plan_flags"]), 1)


class ReviewTime(unittest.TestCase):
    def test_e2_supported_by_h3(self):
        e2 = derived("F-RE#failure", "F-RE", "inherited", spec("F-PLAN#H3"),
                     acceptance_evidence=field("binding", ["removal_failure_reported_as_failure"]))
        self.assertEqual(disp(c.report(ext(PLAN_H3, e2)), "F-RE#failure")["disposition"], "supported")

    def test_e3_against_ambiguous_043_is_flagged(self):
        d = disp(c.report(ext(P043_AMBIGUOUS, E3)), "F-RE#affirmative")
        self.assertTrue(d["flag"])
        self.assertEqual(d["disposition"], "requires_owner_decision")  # evidence stricter outranks ambiguous owner
        self.assertTrue(any("ambiguous" in r for r in d["reasons"]))

    def test_e3_against_043_typed_binding_host_is_supported(self):
        # The extraction-miss case: resolving 043 to 'binding host' makes E3 pass.
        d = disp(c.report(ext(P043_BINDING_HOST, E3)), "F-RE#affirmative")
        self.assertEqual(d["disposition"], "supported")

    def test_general_citation(self):
        e = derived("F-RO#positive", "F-RO", "inherited", {"kind": "general", "text": "evidence-and-claims 3.9"},
                    acceptance_evidence=field("binding", ["actor_attributed_removal"]))
        self.assertEqual(disp(c.report(ext(e)), "F-RO#positive")["disposition"], "requires_owner_decision")

    def test_unchanged_claim_with_general_citation(self):
        e = derived("F-DEC#unchanged", "F-DEC", "unchanged", {"kind": "general", "text": "H1-H5"},
                    acceptance_evidence=field("binding", ["actor_attributed_removal"]))
        d = disp(c.report(ext(e)), "F-DEC#unchanged")
        self.assertTrue(d["flag"])
        self.assertTrue(any("unchanged" in r for r in d["reasons"]))

    def test_no_parent(self):
        e = derived("X#1", "X", "new", {"kind": "none"}, acceptance_evidence=field("binding", ["exclusive_causality"]))
        self.assertEqual(disp(c.report(ext(e)), "X#1")["disposition"], "requires_owner_decision")

    def test_realization_choice_not_flagged(self):
        e = derived("NC-C#1", "NC-C", "realization_choice", {"kind": "none"},
                    outcome_owner=field("selected_realization", ["container_engine"]))
        d = disp(c.report(ext(e)), "NC-C#1")
        self.assertEqual(d["disposition"], "realization")
        self.assertFalse(d["flag"])

    def test_proposal_not_binding(self):
        e = derived("X#2", "X", "proposed", {"kind": "none"}, acceptance_evidence=field("binding", ["exclusive_causality"]))
        d = disp(c.report(ext(e)), "X#2")
        self.assertEqual(d["disposition"], "proposal")
        self.assertFalse(d["flag"])

    def test_owner_explicit_requirement_supported(self):
        # NC-b: the parent itself plainly requires Host-attributed removal.
        p = parent("NC-B#owner", "NC-B", outcome_owner=field("binding", ["host"]),
                   acceptance_evidence=field("binding", ["actor_attributed_removal", "independent_absence"]))
        e = derived("NC-B#review", "NC-B-REVIEW", "inherited", spec("NC-B#owner"),
                    outcome_owner=field("binding", ["host"]),
                    acceptance_evidence=field("binding", ["actor_attributed_removal"]))
        self.assertEqual(disp(c.report(ext(p, e)), "NC-B#review")["disposition"], "supported")

    def test_narrowing_is_contradictory(self):
        p = parent("P#1", "P", acceptance_evidence=field("binding", ["independent_absence", "removal_failure_reported_as_failure"]))
        e = derived("D#1", "D", "inherited", spec("P#1"), replaces=True,
                    acceptance_evidence=field("binding", ["independent_absence"]))
        self.assertEqual(disp(c.report(ext(p, e)), "D#1")["disposition"], "contradictory")

    def test_disjoint_actor_is_contradictory(self):
        p = parent("P#2", "P", outcome_owner=field("binding", ["container_engine"]))
        e = derived("D#2", "D", "inherited", spec("P#2"), outcome_owner=field("binding", ["host"]))
        self.assertEqual(disp(c.report(ext(p, e)), "D#2")["disposition"], "contradictory")

    def test_fewer_actors_is_stricter(self):
        p = parent("P#3", "P", outcome_owner=field("binding", ["host", "container_engine"]))
        e = derived("D#3", "D", "inherited", spec("P#3"), outcome_owner=field("binding", ["host"]))
        self.assertEqual(disp(c.report(ext(p, e)), "D#3")["disposition"], "requires_owner_decision")

    def test_other_not_established(self):
        p = parent("P#4", "P", path_fact=field("binding", ["other"]))
        e = derived("D#4", "D", "inherited", spec("P#4"), path_fact=field("binding", ["other"]))
        self.assertEqual(disp(c.report(ext(p, e)), "D#4")["disposition"], "not_established")

    def test_missing_parent(self):
        e = derived("D#5", "D", "inherited", spec("nope"), outcome_owner=field("binding", ["host"]))
        self.assertEqual(disp(c.report(ext(e)), "D#5")["disposition"], "not_established")

    def test_parent_selected_realization_is_not_binding(self):
        e = derived("D#6", "D", "inherited", spec("F-PLAN#H3"), outcome_owner=field("binding", ["host"]))
        self.assertEqual(disp(c.report(ext(PLAN_H3, e)), "D#6")["disposition"], "requires_owner_decision")


class Fallback(unittest.TestCase):
    def test_fallback_resolves_only_ambiguous_fields(self):
        rep = c.report(ext(P043_AMBIGUOUS, E3), resolutions=[
            {"entry": "F-043#cleanup", "property": "outcome_owner", "status": "binding", "value": ["host"]},
            {"entry": "F-043#cleanup", "property": "verifier", "status": "binding", "value": ["host"]},
        ])
        applied = [l["applied"] for l in rep["fallback_log"]]
        self.assertEqual(applied, [True, False])  # verifier was unspecified, not ambiguous

    def test_fallback_can_stay_ambiguous(self):
        rep = c.report(ext(P043_AMBIGUOUS), resolutions=[
            {"entry": "F-043#cleanup", "property": "outcome_owner", "status": "ambiguous", "value": []}])
        self.assertEqual(len(rep["plan_flags"]), 1)


class Validation(unittest.TestCase):
    def test_rejects_unknown_actor(self):
        bad = ext(parent("P#9", "P", outcome_owner=field("binding", ["the_host_process"])))
        with self.assertRaises(c.ExtractionError):
            c.report(bad)

    def test_rejects_wrong_schema(self):
        with self.assertRaises(c.ExtractionError):
            c.report({"schema": "other", "entries": []})


if __name__ == "__main__":
    unittest.main()
