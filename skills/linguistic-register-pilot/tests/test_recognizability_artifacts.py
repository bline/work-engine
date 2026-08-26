from __future__ import annotations

import importlib.util
import json
import tempfile
import unittest
from pathlib import Path

import yaml


ROOT = Path(__file__).parents[1]
SCRIPT = ROOT / "scripts" / "recognizability_artifacts.py"
SPEC = importlib.util.spec_from_file_location("recognizability_artifacts", SCRIPT)
assert SPEC and SPEC.loader
PILOT = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(PILOT)


class RecognizabilityArtifactsTest(unittest.TestCase):
    def fixture(self, directory: Path) -> tuple[dict, Path]:
        profiles = []
        for candidate in ("alpha", "beta", "gamma"):
            profile_path = directory / f"{candidate}.yaml"
            profile_path.write_text(yaml.safe_dump({
                "artifact_type": "linguistic_register_profile_v1",
                "candidate_id": candidate,
                "features": [{
                    "layer": "surface", "category": "cadence",
                    "description": f"Abstract feature for {candidate}.",
                    "distinctiveness_weight": 3, "disposition": "realization_only",
                }, {
                    "layer": "discourse", "category": "method",
                    "description": "Excluded semantic method.",
                    "distinctiveness_weight": 5, "disposition": "semantic_addition",
                }],
            }, sort_keys=False), encoding="utf-8")
            profiles.append({
                "candidate_id": candidate, "path": profile_path.name,
                "sha256": PILOT.sha256_file(profile_path),
            })
        role_path = directory / "role.yaml"
        role_path.write_text("role fixture\n", encoding="utf-8")
        plan = {
            "artifact_type": "linguistic_register_recognizability_plan_v1",
            "schema_version": 1, "status": "frozen", "experiment_id": "fixture-check",
            "repository": str(directory),
            "role_artifact": {"path": role_path.name, "sha256": PILOT.sha256_file(role_path)},
            "profiles": profiles,
            "neutral_decoy": {
                "decoy_id": "neutral-decoy", "construction_basis": "Generic prose controls.",
                "features": [{"layer": "surface", "category": "cadence",
                              "description": "Use ordinary sentence lengths.", "salience_weight": 3}],
            },
            "randomization_seed": "frozen fixture seed",
            "classifier_contract": {
                "task": "Score anonymous cards.", "blindness": "Only packet content is visible.",
                "score_scale": "Integers 1 through 5.",
                "required_outputs": ["internal_coherence", "non_neutral_distinctiveness",
                                     "pairwise_separation", "most_neutral_card"],
                "prohibited_inference": "Do not infer identities.",
            },
            "thresholds": {
                "minimum_internal_coherence": 3, "minimum_non_neutral_distinctiveness": 3,
                "minimum_separation_from_neutral": 3, "require_correct_neutral_identification": True,
            },
            "preregistration_owner": "fixture human", "limitations": [],
        }
        plan_path = directory / "plan.yaml"
        plan_path.write_text(yaml.safe_dump(plan, sort_keys=False), encoding="utf-8")
        return plan, plan_path

    def result(self, packet: dict, packet_digest: str, neutral_card: str) -> dict:
        card_ids = [card["anonymous_card_id"] for card in packet["cards"]]
        return {
            "artifact_type": "linguistic_register_classifier_result_v1", "schema_version": 1,
            "experiment_id": packet["experiment_id"], "packet_artifact_sha256": packet_digest,
            "classifier_provenance": {
                "provider": "fixture", "model": "fixture-model", "reasoning_effort": "medium",
                "execution_mode": "fresh fixture", "fresh_context": True, "packet_only": True,
                "started_at_utc": "2026-01-01T00:00:00Z", "completed_at_utc": "2026-01-01T00:01:00Z",
            },
            "card_scores": [{
                "anonymous_card_id": card_id, "internal_coherence": 4,
                "non_neutral_distinctiveness": 2 if card_id == neutral_card else 4,
                "rationale": "Fixture score.",
            } for card_id in card_ids],
            "pairwise_separation": [{
                "card_ids": [left, right], "score": 4, "rationale": "Fixture separation.",
            } for index, left in enumerate(card_ids) for right in card_ids[index + 1:]],
            "most_neutral_card_id": neutral_card, "method_limitations": ["Fixture only."],
        }

    def test_prepare_is_deterministic_and_excludes_semantic_features(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            directory = Path(temporary)
            _, plan_path = self.fixture(directory)
            plan = PILOT.validate_plan(PILOT.load_yaml(plan_path), plan_path)
            first_packet, first_key = PILOT.prepare(plan, plan_path)
            second_packet, second_key = PILOT.prepare(plan, plan_path)
            self.assertEqual(first_packet, second_packet)
            self.assertEqual(first_key, second_key)
            descriptions = [feature["description"] for card in first_packet["cards"] for feature in card["features"]]
            self.assertNotIn("Excluded semantic method.", descriptions)
            self.assertNotIn("candidate_id", json.dumps(first_packet))

    def test_evaluate_passes_only_with_neutral_and_candidate_gates(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            directory = Path(temporary)
            _, plan_path = self.fixture(directory)
            plan = PILOT.validate_plan(PILOT.load_yaml(plan_path), plan_path)
            packet, key = PILOT.prepare(plan, plan_path)
            packet_path = directory / "packet.json"
            packet_path.write_text(json.dumps(packet, indent=2, sort_keys=True) + "\n", encoding="utf-8")
            neutral_card = next(card for card, identity in key["anonymous_mapping"].items()
                                if identity == key["neutral_decoy_id"])
            result = self.result(packet, PILOT.sha256_file(packet_path), neutral_card)
            validated = PILOT.validate_result(result, packet, PILOT.sha256_file(packet_path))
            report = PILOT.evaluate(plan, key, validated, "c" * 64, "d" * 64)
            self.assertTrue(report["gate_passed"])
            self.assertTrue(all(item["disposition"] == "recognizable" for item in report["candidate_results"]))
            self.assertEqual(report["bindings"]["blinding_key_artifact_sha256"], "c" * 64)

    def test_result_requires_every_pair_once(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            directory = Path(temporary)
            _, plan_path = self.fixture(directory)
            plan = PILOT.validate_plan(PILOT.load_yaml(plan_path), plan_path)
            packet, key = PILOT.prepare(plan, plan_path)
            packet_path = directory / "packet.json"
            packet_path.write_text(json.dumps(packet, indent=2, sort_keys=True) + "\n", encoding="utf-8")
            neutral_card = next(card for card, identity in key["anonymous_mapping"].items()
                                if identity == key["neutral_decoy_id"])
            result = self.result(packet, PILOT.sha256_file(packet_path), neutral_card)
            result["pairwise_separation"].pop()
            with self.assertRaisesRegex(PILOT.RecognizabilityError, "every unordered pair"):
                PILOT.validate_result(result, packet, PILOT.sha256_file(packet_path))

    def test_changed_profile_breaks_frozen_plan_binding(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            directory = Path(temporary)
            plan, plan_path = self.fixture(directory)
            (directory / "alpha.yaml").write_text("changed\n", encoding="utf-8")
            with self.assertRaisesRegex(PILOT.RecognizabilityError, "does not match current bytes"):
                PILOT.validate_plan(plan, plan_path)

    def test_prepared_artifacts_reject_corrupted_key_mappings(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            directory = Path(temporary)
            _, plan_path = self.fixture(directory)
            plan = PILOT.validate_plan(PILOT.load_yaml(plan_path), plan_path)
            packet, key = PILOT.prepare(plan, plan_path)
            card_ids = sorted(key["anonymous_mapping"])
            candidates = [identity for identity in key["anonymous_mapping"].values()
                          if identity != key["neutral_decoy_id"]]
            corruptions = []

            duplicate = json.loads(json.dumps(key))
            duplicate["anonymous_mapping"][card_ids[1]] = duplicate["anonymous_mapping"][card_ids[0]]
            corruptions.append(duplicate)

            missing = json.loads(json.dumps(key))
            del missing["anonymous_mapping"][card_ids[0]]
            corruptions.append(missing)

            swapped = json.loads(json.dumps(key))
            left = next(card for card, identity in swapped["anonymous_mapping"].items() if identity == candidates[0])
            right = next(card for card, identity in swapped["anonymous_mapping"].items() if identity == candidates[1])
            swapped["anonymous_mapping"][left], swapped["anonymous_mapping"][right] = (
                swapped["anonymous_mapping"][right], swapped["anonymous_mapping"][left]
            )
            corruptions.append(swapped)

            for corrupted in corruptions:
                with self.subTest(mapping=corrupted["anonymous_mapping"]):
                    with self.assertRaisesRegex(PILOT.RecognizabilityError, "does not exactly match"):
                        PILOT.validate_prepared_artifacts(plan, plan_path, packet, corrupted)

    def test_prepared_artifacts_reject_packet_key_mismatch(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            directory = Path(temporary)
            _, plan_path = self.fixture(directory)
            plan = PILOT.validate_plan(PILOT.load_yaml(plan_path), plan_path)
            packet, key = PILOT.prepare(plan, plan_path)
            corrupted = json.loads(json.dumps(packet))
            corrupted["cards"][0]["features"][0]["description"] = "Changed after freezing."
            with self.assertRaisesRegex(PILOT.RecognizabilityError, "packet does not exactly match"):
                PILOT.validate_prepared_artifacts(plan, plan_path, corrupted, key)


if __name__ == "__main__":
    unittest.main()
