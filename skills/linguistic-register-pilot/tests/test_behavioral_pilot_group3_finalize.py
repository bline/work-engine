import importlib.util
import unittest
from pathlib import Path


SCRIPT = Path(__file__).parents[1] / "scripts/behavioral_pilot_group3_finalize.py"
SPEC = importlib.util.spec_from_file_location("behavioral_pilot_group3_finalize", SCRIPT)
MODULE = importlib.util.module_from_spec(SPEC)
assert SPEC and SPEC.loader
SPEC.loader.exec_module(MODULE)


class Group3FinalizeTest(unittest.TestCase):
    def test_exact_match_reports_token_offsets(self):
        self.assertEqual({"length": 3, "source_start": 1, "render_start": 1},
                         MODULE.exact_match(["a", "b", "c", "d", "e"], ["x", "b", "c", "d", "y"]))

    def test_near_match_requires_preregistered_identity(self):
        source = list("abcdefghijkl")
        render = list("abcdefghijxl")
        self.assertTrue(MODULE.near_matches(source, render))
        self.assertFalse(MODULE.near_matches(source, list("abcdefxxxxxx")))

    def test_scalar_rule_finds_perfect_separator(self):
        rows = [{"condition": condition, "metrics": {"length": value}}
                for condition, value in (("C0", 1), ("C0", 2), ("C1", 5), ("C1", 6), ("C2", 9), ("C2", 10))]
        self.assertTrue(MODULE.scalar_rules(rows))

    def test_manipulation_gate_requires_five_exact_and_order(self):
        mapping = {f"a{i}": condition for i, condition in enumerate(("C0", "C0", "C1", "C1", "C2", "C2"))}
        rows = [{"artifact_id": artifact, "assigned_level": condition,
                 "surface_uptake": {"C0": 1, "C1": 4, "C2": 4}[condition],
                 "discourse_uptake": {"C0": 0, "C1": 1, "C2": 4}[condition]}
                for artifact, condition in mapping.items()]
        self.assertTrue(MODULE.manipulation({"artifacts": rows}, mapping)["gate"])


if __name__ == "__main__":
    unittest.main()
