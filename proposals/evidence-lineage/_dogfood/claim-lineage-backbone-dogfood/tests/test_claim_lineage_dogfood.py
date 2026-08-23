from __future__ import annotations

import importlib.util
import json
from copy import deepcopy
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path


ROOT = Path(__file__).parents[1]
SCRIPT = ROOT / "scripts" / "claim_lineage_dogfood.py"
SPEC = importlib.util.spec_from_file_location("claim_lineage_dogfood", SCRIPT)
assert SPEC and SPEC.loader
DOGFOOD = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(DOGFOOD)


class ClaimLineageDogfoodTest(unittest.TestCase):
    def setUp(self) -> None:
        self.records = DOGFOOD.load_json(ROOT / "records" / "claim-lineage-records.json")
        self.schema = DOGFOOD.load_json(ROOT / "schema" / "claim-lineage-dogfood-v1.schema.json")

    def test_vertical_rebuild_proves_semantic_path(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory)
            DOGFOOD.write_outputs(ROOT, output)
            projection = json.loads((output / "generated" / "projection.json").read_text())
            cells = projection["proofs"]["cells"]
            self.assertTrue(projection["proofs"]["run_valid"])
            self.assertTrue(projection["proofs"]["positive_four_proof_demonstration"])
            self.assertEqual({result["status"] for result in cells.values()}, {"pass"})
            report = (output / "evidence-report.md").read_text()
            self.assertIn("not outcome-independent falsification", report)
            self.assertIn("non-clean publication topology", report)

    def test_unchanged_and_changed_causality_are_distinct(self) -> None:
        DOGFOOD.validate(self.records, self.schema)
        changed = DOGFOOD.edges_for(self.records, "changed_because_of")
        self.assertEqual(len(changed), 1)
        self.assertEqual(changed[0]["source"], "judgment.refresh-v")
        self.assertEqual(changed[0]["target"], "event.proposal-context-lineage-change")
        self.assertNotEqual(changed[0]["target"], "event.reviewer-retention-completion")
        unchanged = next(j for j in self.records["judgments"] if j["id"] == "judgment.refresh-r")
        self.assertEqual(unchanged["causal_event_ids"], [])

    def test_reliance_is_exact_prospective_and_user_reopenable(self) -> None:
        targets = {r["claim_revision_id"] for r in self.records["reliances"]}
        self.assertEqual(targets, {
            "work-engine.dogfood.claim.reviewer-context-retention/r2",
            "work-engine.dogfood.finding.review-context-lineage-gap/r2",
        })
        self.assertTrue(all(r["historicity"] == "prospective_dogfood_reliance" for r in self.records["reliances"]))
        self.assertTrue(all(r["reopening_owner"] == "user" for r in self.records["reliances"]))

    def test_counterfixture_forks_only_authority_domain(self) -> None:
        claims = {claim["id"]: claim for claim in self.records["claims"]}
        research = claims["work-engine.dogfood.claim.reviewer-context-retention"]
        policy = claims["work-engine.dogfood.policy.reviewer-context-retention"]
        compared = {key for key in research if key not in {"id", "authority_domain"}}
        self.assertTrue(all(research[key] == policy[key] for key in compared))
        self.assertNotEqual(research["authority_domain"], policy["authority_domain"])
        revisions = {revision["claim_id"]: revision for revision in self.records["revisions"] if revision["ordinal"] == 1}
        self.assertEqual(revisions[research["id"]]["evidence_baseline"], revisions[policy["id"]]["evidence_baseline"])
        self.assertEqual(revisions[research["id"]]["conclusion"], revisions[policy["id"]]["conclusion"])
        self.assertTrue(any(edge["relationship"] == "contrasts_with" and edge["source"] == policy["id"]
                            and edge["target"] == research["id"] for edge in self.records["edges"]))

    def test_checked_outputs_are_fresh(self) -> None:
        DOGFOOD.verify(ROOT)

    def test_closed_contract_rejects_unknown_record_fields(self) -> None:
        changed = deepcopy(self.records)
        changed["claims"][0]["convenient_but_unauthorized"] = True
        with self.assertRaisesRegex(DOGFOOD.RecordError, "missing or unknown fields"):
            DOGFOOD.validate(changed, self.schema)

    def test_experimental_tree_cannot_enter_packet_discovery(self) -> None:
        self.assertEqual(list(ROOT.rglob("packet.json")), [])
        repository = ROOT.parents[3]
        result = subprocess.run(
            [sys.executable, str(repository / "skills/proposal-packets/scripts/proposal_packets.py"),
             "validate", str(repository)], capture_output=True, text=True,
        )
        self.assertEqual(result.returncode, 0, result.stderr)


if __name__ == "__main__":
    unittest.main()
