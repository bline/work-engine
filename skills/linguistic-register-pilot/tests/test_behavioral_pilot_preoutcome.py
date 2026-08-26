import importlib.util
import json
import tempfile
import unittest
from pathlib import Path


SCRIPT = Path(__file__).parents[1] / "scripts/behavioral_pilot_preoutcome.py"
SPEC = importlib.util.spec_from_file_location("behavioral_pilot_preoutcome", SCRIPT)
MODULE = importlib.util.module_from_spec(SPEC)
assert SPEC and SPEC.loader
SPEC.loader.exec_module(MODULE)


class BehavioralPilotPreoutcomeTest(unittest.TestCase):
    def test_transport_accepts_exact_single_text_wrapper(self):
        text = "word " * 700
        accepted, disposition = MODULE.normalize_render(json.dumps({"text": text}).encode())
        self.assertEqual(disposition, "exact_single_key_text_wrapper")
        self.assertEqual(700, len(MODULE.tokens(accepted)))

    def test_transport_rejects_extra_key_and_nested_wrapper(self):
        text = "word " * 700
        for value in ({"text": text, "extra": 1}, {"text": {"text": text}}):
            with self.assertRaises(MODULE.PreoutcomeError):
                MODULE.normalize_render(json.dumps(value).encode())

    def test_scalar_rule_detects_perfect_condition_separator(self):
        rows = [
            {"condition": "C0", "metrics": {"words": 10}},
            {"condition": "C0", "metrics": {"words": 11}},
            {"condition": "C1", "metrics": {"words": 20}},
            {"condition": "C1", "metrics": {"words": 21}},
            {"condition": "C2", "metrics": {"words": 30}},
            {"condition": "C2", "metrics": {"words": 31}},
        ]
        rules = MODULE.perfect_scalar_rules(rows)
        self.assertTrue(any(row["condition"] == "C0" for row in rules))
        self.assertTrue(any(row["condition"] == "C2" for row in rules))

    def test_longest_exact_preserves_offsets_without_text(self):
        result = MODULE.longest_exact(["a", "b", "c", "d"], ["x", "b", "c", "y"])
        self.assertEqual({"length": 2, "source_start": 1, "render_start": 1}, result)

    def test_manipulation_requires_order_and_five_assignments(self):
        mapping = {f"a{i}": condition for i, condition in enumerate(("C0", "C0", "C1", "C1", "C2", "C2"))}
        rows = []
        for artifact_id, condition in mapping.items():
            rows.append({"artifact_id": artifact_id, "assigned_level": condition,
                         "surface_uptake": {"C0": 1, "C1": 4, "C2": 4}[condition],
                         "discourse_uptake": {"C0": 1, "C1": 1, "C2": 4}[condition]})
        result = MODULE.validate_manipulation({"artifacts": rows}, mapping)
        self.assertTrue(result["gate"])

    def test_emit_new_refuses_overwrite(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "artifact.json"
            MODULE.emit_new(path, {"a": 1})
            with self.assertRaises(MODULE.PreoutcomeError):
                MODULE.emit_new(path, {"a": 1})


if __name__ == "__main__":
    unittest.main()
