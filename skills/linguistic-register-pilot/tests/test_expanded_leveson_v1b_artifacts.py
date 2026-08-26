import importlib.util
import json
import tempfile
import unittest
from pathlib import Path


SCRIPT = Path(__file__).parents[1] / "scripts/expanded_leveson_v1b_artifacts.py"
SPEC = importlib.util.spec_from_file_location("expanded_leveson_v1b_artifacts", SCRIPT)
MODULE = importlib.util.module_from_spec(SPEC)
assert SPEC.loader
SPEC.loader.exec_module(MODULE)


class ExpandedLevesonV1BArtifactsTest(unittest.TestCase):
    def test_frozen_cards_are_length_matched(self):
        counts = {
            candidate: len(MODULE.tokens(" ".join(row[3] for row in rows)))
            for candidate, rows in MODULE.CARD_ROWS.items()
        }
        self.assertEqual(set(counts.values()), {72})

    def test_frozen_cards_preserve_preregistered_layer_composition(self):
        expected = {
            "leveson-system-safety-expanded-v1": (3, 2),
            "shaw-engineering-judgment": (3, 2),
            "gelman-model-criticism": (4, 1),
            "neutral-editorial-defaults": (4, 1),
        }
        for candidate, rows in MODULE.CARD_ROWS.items():
            layers = [row[1] for row in rows]
            self.assertEqual((layers.count("surface"), layers.count("discourse")), expected[candidate])

    def test_shallow_metrics_cover_preregistered_scalars(self):
        rows = MODULE.CARD_ROWS["gelman-model-criticism"]
        features = [
            {"layer": row[1], "description": row[3], "salience_weight": weight}
            for row, weight in zip(rows, [4, 4, 3, 3, 3])
        ]
        metrics = MODULE.shallow_metrics(features)
        self.assertEqual(metrics["normalized_word_count"], 72)
        self.assertEqual(metrics["feature_count"], 5)
        self.assertEqual(metrics["salience_weight_sum"], 17)
        self.assertEqual(metrics["surface_layer_count"], 4)

    def test_cue_baseline_flags_a_unique_scalar(self):
        mapping = {"B01": MODULE.TARGET, "B02": "a", "B03": "b", "B04": "c"}
        cards = []
        for card_id, candidate in mapping.items():
            description = "one two three" if candidate == MODULE.TARGET else "one two"
            cards.append({"anonymous_card_id": card_id, "features": [
                {"layer": "surface", "description": description, "salience_weight": 3},
                {"layer": "discourse", "description": "same", "salience_weight": 3},
            ]})
        with tempfile.TemporaryDirectory() as temporary:
            directory = Path(temporary)
            packet, key, output = (directory / name for name in ("packet.json", "key.json", "output.json"))
            packet.write_text(json.dumps({"cards": cards}))
            key.write_text(json.dumps({"mapping": mapping}))
            MODULE.cue_baseline(packet, key, output)
            result = json.loads(output.read_text())
        self.assertFalse(result["gate"])
        self.assertIn("normalized_word_count", {row["metric"] for row in result["cross_replica_perfect_leveson_rules"]})


if __name__ == "__main__":
    unittest.main()
