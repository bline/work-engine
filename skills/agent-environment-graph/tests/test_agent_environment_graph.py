from __future__ import annotations

import importlib.util
import json
import tempfile
import unittest
from pathlib import Path


ROOT = Path(__file__).parents[1]
FIXTURES = Path(__file__).parent / "fixtures"
SCRIPT = ROOT / "scripts" / "agent_environment_graph.py"
SPEC = importlib.util.spec_from_file_location("agent_environment_graph", SCRIPT)
assert SPEC and SPEC.loader
AEG = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(AEG)


class AgentEnvironmentGraphTest(unittest.TestCase):
    def load_fixture(self):
        return AEG.load_model(FIXTURES / "workflow-invariants.md", FIXTURES / "agent-environments.yaml")

    def test_vertical_pipeline_validates_analyzes_and_renders_stably(self):
        catalog, environment = self.load_fixture()
        report = AEG.analyze(catalog, environment)
        self.assertEqual(report["counts"]["invariants"], 1)
        self.assertEqual(report["candidate_findings"], [])
        first = AEG.render(catalog, environment, FIXTURES / "workflow-invariants.md", FIXTURES / "agent-environments.yaml")
        second = AEG.render(catalog, environment, FIXTURES / "workflow-invariants.md", FIXTURES / "agent-environments.yaml")
        self.assertEqual(first, second)
        self.assertIn("role projection truth remains owned by the YAML input", first)

    def test_unknown_reference_fails_closed(self):
        source = (FIXTURES / "agent-environments.yaml").read_text().replace("INV-001", "INV-999")
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "invalid.yaml"
            path.write_text(source)
            with self.assertRaisesRegex(AEG.GraphError, "unknown invariant"):
                AEG.load_model(FIXTURES / "workflow-invariants.md", path)

    def test_duplicate_yaml_key_fails_closed(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "duplicate.yaml"
            path.write_text("schema_version: 1\nschema_version: 1\n")
            with self.assertRaisesRegex(AEG.GraphError, "duplicate YAML key"):
                AEG.load_yaml(path)

    def test_contract_judgment_requires_human_approval(self):
        artifact = {
            "schema_version": 1, "producer": "test", "judgments": [{
                "id": "j1", "category": "ownership", "subject": "artifact.view",
                "conclusion": "human owns it", "evidence": ["fixture#owner"],
                "changes_contract": True, "human_approval": None,
            }],
        }
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "judgment.json"
            path.write_text(json.dumps(artifact))
            with self.assertRaisesRegex(AEG.GraphError, "requires human approval"):
                AEG.validate_judgments(path)
            artifact["judgments"][0]["human_approval"] = {"approver": "human", "reference": "approval-1"}
            path.write_text(json.dumps(artifact))
            self.assertEqual(AEG.validate_judgments(path)["status"], "valid")

    def test_candidates_do_not_claim_semantic_authority(self):
        catalog, environment = self.load_fixture()
        result = AEG.candidates(catalog, environment)
        self.assertEqual(result["status"], "candidates_only")
        self.assertIn("human approval", result["semantic_authority"])


if __name__ == "__main__":
    unittest.main()
