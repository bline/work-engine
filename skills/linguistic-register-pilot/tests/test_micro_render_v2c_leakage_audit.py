import importlib.util
import json
import tempfile
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
MODULE_PATH = ROOT / "scripts" / "micro_render_v2c_leakage_audit.py"
SPEC = importlib.util.spec_from_file_location("micro_render_v2c_leakage_audit", MODULE_PATH)
AUDIT = importlib.util.module_from_spec(SPEC)
assert SPEC.loader
SPEC.loader.exec_module(AUDIT)


class MicroRenderV2CLeakageAuditTests(unittest.TestCase):
    def test_literal_hits_respects_word_boundaries(self):
        self.assertEqual(AUDIT.literal_hits("direct exposition", ("position",)), [])
        self.assertEqual(AUDIT.literal_hits("Nancy Leveson wrote", ("Nancy Leveson",)), ["Nancy Leveson"])

    def test_metrics_distinguish_structure_without_interpreting_it(self):
        value = "# Heading\n\nOne sentence?\n\n- Another sentence!"
        metrics = AUDIT.sample_metrics(value)
        self.assertEqual(metrics["heading_count"], 1)
        self.assertEqual(metrics["paragraph_count"], 3)
        self.assertEqual(metrics["list_marker_count"], 1)
        self.assertEqual(metrics["sentence_count"], 2)

    def test_skill_content_recovery_preserves_exact_event_bytes(self):
        content, prefix = AUDIT.extract_skill_content("warning\n---\nname: generic\n---\nbody\n")
        self.assertEqual(prefix, "warning\n")
        self.assertEqual(content, "---\nname: generic\n---\nbody\n")

    def test_inventory_verification_uses_git_blob_identity(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory) / "subject"
            root.mkdir()
            artifact = root / "artifact.txt"
            artifact.write_text("evidence\n")
            inventory = Path(directory) / "inventory.json"
            inventory.write_text(json.dumps({
                "checkpoint_commit_oid": AUDIT.SUBJECT_COMMIT,
                "checkpoint_tree_oid": AUDIT.SUBJECT_TREE,
                "entries": [{
                    "path": "artifact.txt", "mode": "100644", "type": "blob",
                    "oid": AUDIT.git_blob_oid(artifact.read_bytes()),
                }],
            }))
            self.assertTrue(AUDIT.verify_inventory(root, inventory)["verified"])
            artifact.write_text("changed\n")
            self.assertFalse(AUDIT.verify_inventory(root, inventory)["verified"])


if __name__ == "__main__":
    unittest.main()
