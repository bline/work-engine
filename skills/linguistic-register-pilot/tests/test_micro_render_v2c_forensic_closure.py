import importlib.util
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
MODULE_PATH = ROOT / "scripts" / "micro_render_v2c_forensic_closure.py"
SPEC = importlib.util.spec_from_file_location("micro_render_v2c_forensic_closure", MODULE_PATH)
CLOSURE = importlib.util.module_from_spec(SPEC)
assert SPEC.loader
SPEC.loader.exec_module(CLOSURE)


class MicroRenderV2CForensicClosureTests(unittest.TestCase):
    def test_token_normalization_is_nfkc_casefolded(self):
        self.assertEqual(CLOSURE.tokens("Ｃlaim’s  EVIDENCE"), ["claim's", "evidence"])

    def test_longest_exact_sequence_retains_offsets_without_source_text(self):
        render = CLOSURE.tokens("alpha beta gamma delta epsilon")
        source = CLOSURE.tokens("zero beta gamma delta one")
        result = CLOSURE.longest_exact(render, source, CLOSURE.positions_by_token(source))
        self.assertEqual(result["length_tokens"], 3)
        self.assertEqual(result["render_token_offset"], 1)
        self.assertEqual(result["source_token_offset"], 1)
        self.assertNotIn("sequence", result)

    def test_near_verbatim_detects_one_mismatch_in_twelve_tokens(self):
        render = "one two three four five six seven eight nine ten eleven twelve".split()
        source = "zero one two three four five changed seven eight nine ten eleven twelve end".split()
        matches = CLOSURE.near_verbatim_matches(render, source)
        self.assertTrue(any(row["length_tokens"] == 12 and row["identical_positions"] == 11 for row in matches))

    def test_question_rule_metrics_are_exact(self):
        rows = [
            {"condition_id": "shaw-engineering-judgment", "features": {"question_mark_count": 3}},
            {"condition_id": "neutral-editorial-defaults", "features": {"question_mark_count": 0}},
        ]
        rule = {"feature": "question_mark_count", "operator": "gt", "threshold": 0.0}
        result = CLOSURE.metrics_for_predictions(rows, "shaw-engineering-judgment", rule)
        self.assertEqual((result["precision"], result["recall"], result["specificity"]), (1.0, 1.0, 1.0))

    def test_html_extractor_omits_script_and_style(self):
        parser = CLOSURE.VisibleHTML()
        parser.feed("<style>hidden</style><p>Visible</p><script>also hidden</script>")
        self.assertEqual(parser.parts, ["Visible"])


if __name__ == "__main__":
    unittest.main()
