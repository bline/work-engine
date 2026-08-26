import importlib.util
import json
import tempfile
import unittest
from pathlib import Path


SCRIPT = Path(__file__).parents[1] / "scripts/expanded_leveson_artifacts.py"
SPEC = importlib.util.spec_from_file_location("expanded_leveson_artifacts", SCRIPT)
MODULE = importlib.util.module_from_spec(SPEC)
assert SPEC.loader
SPEC.loader.exec_module(MODULE)


class ExpandedLevesonArtifactsTest(unittest.TestCase):
    def test_normalization_is_nfkc_casefolded(self):
        self.assertEqual(MODULE.normalized_tokens("Ｃausal O’Clock"), ["causal", "o'clock"])

    def test_balanced_ranges_cover_small_document_without_overlap(self):
        ranges = MODULE.segment_ranges(8, 12)
        self.assertEqual(ranges, [("beginning", 0, 3), ("middle", 3, 6), ("end", 6, 8)])

    def test_balanced_ranges_sample_large_document(self):
        ranges = MODULE.segment_ranges(12000, 6000)
        self.assertEqual(ranges, [("beginning", 0, 2000), ("middle", 5000, 7000), ("end", 10000, 12000)])

    def test_testimony_parser_selects_only_exact_article_panel(self):
        parser = MODULE.TestimonyHTML()
        parser.feed("<body>chrome<div class='panel panel--small panel--main'>kept<script>hidden</script><p>text</p></div>tail</body>")
        self.assertEqual(parser.matches, 1)
        self.assertEqual(parser.parts, ["kept", "text"])

    def test_longest_exact_reports_offsets_without_source_text(self):
        result = MODULE.longest_exact(["one", "two", "three"], ["zero", "two", "three"])
        self.assertEqual(result["length_tokens"], 2)
        self.assertEqual(result["profile_token_offset"], 1)
        self.assertEqual(result["source_token_offset"], 1)
        self.assertNotIn("sequence", result)

    def test_near_match_obeys_identity_threshold(self):
        left = [f"w{i}" for i in range(12)]
        right = list(left)
        right[4] = "different"
        rows = MODULE.near_matches(left, right, 12, 12, 0.9, 10)
        self.assertEqual(len(rows), 1)
        self.assertEqual(rows[0]["identical_positions"], 11)

    def test_cue_baseline_flags_cross_replica_word_count_rule(self):
        packet = {"cards": []}
        mapping = {}
        for index, (card_id, candidate, words) in enumerate([
            ("R01", "leveson-system-safety-expanded-v1", "one two three four"),
            ("R02", "neutral-editorial-defaults", "one two"),
            ("R03", "gelman-model-criticism", "one two"),
            ("R04", "shaw-engineering-judgment", "one two"),
        ]):
            packet["cards"].append({"anonymous_card_id": card_id, "features": [
                {"layer": "surface", "description": words},
                {"layer": "discourse", "description": "same"},
            ]})
            mapping[card_id] = candidate
        with tempfile.TemporaryDirectory() as temporary:
            directory = Path(temporary)
            packet_path, key_path, output = (directory / name for name in ("packet.json", "key.json", "result.json"))
            packet_path.write_text(json.dumps(packet))
            key_path.write_text(json.dumps({"mapping": mapping}))
            MODULE.cue_baseline(packet_path, key_path, output)
            result = json.loads(output.read_text())
        self.assertTrue(result["flag"])
        self.assertIn("word_count", {row["metric"] for row in result["cross_replica_perfect_leveson_rules"]})


if __name__ == "__main__":
    unittest.main()
