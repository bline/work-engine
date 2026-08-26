from __future__ import annotations

import importlib.util
import json
import unittest
from collections import Counter
from pathlib import Path


ROOT = Path(__file__).parents[1]
SCRIPT = ROOT / "scripts" / "calibration_artifacts.py"
PLAN_PATH = ROOT / "pilot" / "profile-calibration-v2" / "preregistration.yaml"
SPEC = importlib.util.spec_from_file_location("calibration_artifacts", SCRIPT)
assert SPEC and SPEC.loader
CAL = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(CAL)


class CalibrationArtifactsTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.plan, cls.base_plan, cls.base_path = CAL.validate_plan(CAL.BASE.load_yaml(PLAN_PATH), PLAN_PATH)

    def prepared(self, pass_number: int) -> tuple[dict, dict]:
        return CAL.prepare_pass(self.plan, self.base_plan, self.base_path, PLAN_PATH, pass_number)

    def synthetic_result(self, packet: dict, key: dict, *, authentic_coherence: int = 5,
                         composite_coherence: int = 3) -> dict:
        identity_by_card = key["anonymous_mapping"]
        scores = []
        for card in packet["cards"]:
            card_id = card["anonymous_card_id"]
            kind = key["kind_by_identity"][identity_by_card[card_id]]
            coherence = authentic_coherence if kind == "authentic" else composite_coherence
            distinctiveness = 1 if kind == "neutral" else 4
            scores.append({
                "anonymous_card_id": card_id,
                "internal_coherence": coherence if kind != "neutral" else 4,
                "non_neutral_distinctiveness": distinctiveness,
                "rationale": "Synthetic fixture score.",
            })
        card_ids = sorted(identity_by_card)
        raw = {
            "card_scores": scores,
            "pairwise_separation": [{
                "card_ids": [left, right], "score": 4, "rationale": "Synthetic fixture separation.",
            } for index, left in enumerate(card_ids) for right in card_ids[index + 1:]],
            "most_neutral_card_id": next(
                card_id for card_id, identity in identity_by_card.items()
                if identity == key["neutral_identity"]
            ),
            "method_limitations": ["Synthetic fixture."],
        }
        provenance = {
            "provider": "fixture", "model": "fixture", "reasoning_effort": "fixture",
            "execution_mode": "fixture", "fresh_context": True, "packet_only": True,
            "started_at_utc": "2026-01-01T00:00:00Z", "completed_at_utc": "2026-01-01T00:01:00Z",
        }
        return CAL.normalize(raw, packet, provenance)

    def test_five_schedules_are_deterministic_and_blinded(self) -> None:
        mappings = []
        candidate_ids = [binding["candidate_id"] for binding in self.base_plan["profiles"]]
        for pass_number in range(1, 6):
            first = self.prepared(pass_number)
            second = self.prepared(pass_number)
            self.assertEqual(first, second)
            packet, key = first
            self.assertEqual(len(packet["cards"]), 7)
            rendered = json.dumps(packet).lower()
            for candidate_id in candidate_ids:
                self.assertNotIn(candidate_id.lower(), rendered)
            mappings.append(key["anonymous_mapping"])
        self.assertEqual(len({json.dumps(mapping, sort_keys=True) for mapping in mappings}), 5)

    def test_composites_preserve_count_layers_and_weights(self) -> None:
        packet, key = self.prepared(1)
        cards_by_identity = {
            key["anonymous_mapping"][card["anonymous_card_id"]]: card["features"]
            for card in packet["cards"]
        }
        for candidate, composite_id in key["matched_composite_by_candidate"].items():
            authentic = cards_by_identity[candidate]
            composite = cards_by_identity[composite_id]
            self.assertEqual(len(authentic), len(composite))
            self.assertEqual(Counter(item["layer"] for item in authentic),
                             Counter(item["layer"] for item in composite))
            self.assertEqual(Counter(item["salience_weight"] for item in authentic),
                             Counter(item["salience_weight"] for item in composite))

    def test_deterministic_validation_rejects_corrupted_mapping(self) -> None:
        packet, key = self.prepared(1)
        corrupted = json.loads(json.dumps(key))
        card_ids = sorted(corrupted["anonymous_mapping"])
        corrupted["anonymous_mapping"][card_ids[1]] = corrupted["anonymous_mapping"][card_ids[0]]
        with self.assertRaisesRegex(CAL.CalibrationError, "key does not match deterministic preparation"):
            CAL.validate_prepared(self.plan, self.base_plan, self.base_path, PLAN_PATH, 1, packet, corrupted)

    def test_aggregate_passes_discriminating_stable_fixture(self) -> None:
        runs = []
        for pass_number in range(1, 6):
            packet, key = self.prepared(pass_number)
            result = self.synthetic_result(packet, key)
            runs.append((packet, key, result, CAL.digest_value(result)))
        plan = dict(self.plan, _plan_path=str(PLAN_PATH))
        report = CAL.aggregate(plan, runs)
        self.assertTrue(report["gate_passed"])
        self.assertEqual(report["neutral_identification"]["correct_passes"], 5)
        self.assertTrue(all(item["authentic_coherence_wins"] == 5 for item in report["candidate_results"]))

    def test_aggregate_rejects_authentic_composite_ceiling_ties(self) -> None:
        runs = []
        for pass_number in range(1, 6):
            packet, key = self.prepared(pass_number)
            result = self.synthetic_result(packet, key, authentic_coherence=5, composite_coherence=5)
            runs.append((packet, key, result, CAL.digest_value(result)))
        plan = dict(self.plan, _plan_path=str(PLAN_PATH))
        report = CAL.aggregate(plan, runs)
        self.assertFalse(report["gate_passed"])
        self.assertTrue(all(item["authentic_coherence_wins"] == 0 for item in report["candidate_results"]))


if __name__ == "__main__":
    unittest.main()
