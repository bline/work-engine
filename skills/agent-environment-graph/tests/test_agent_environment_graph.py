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

    def test_escaped_pipe_is_cell_content_not_a_column_boundary(self):
        source = (FIXTURES / "workflow-invariants.md").read_text().replace(
            "Human owns contract changes.", "Human owns A \\| B contract changes."
        )
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "escaped-pipe.md"
            path.write_text(source)
            catalog = AEG.parse_invariants(path)
        self.assertEqual(catalog["invariants"]["INV-001"]["condition"], "Human owns A | B contract changes.")

    def test_same_target_in_two_subgraphs_has_distinct_node_ids(self):
        catalog, environment = self.load_fixture()
        environment["roles"]["role.builder"]["may_observe"].append("artifact.view")
        rendered = AEG.render(
            catalog, environment,
            FIXTURES / "workflow-invariants.md", FIXTURES / "agent-environments.yaml",
        )
        self.assertIn("N_role_builder_3_artifact_view", rendered)
        self.assertIn("N_role_builder_4_artifact_view", rendered)
        self.assertEqual(rendered.count('N_role_builder_3_artifact_view["View"]'), 1)
        self.assertEqual(rendered.count('N_role_builder_4_artifact_view["View"]'), 1)

    def test_relationship_table_surfaces_consumes_and_emits(self):
        catalog, environment = self.load_fixture()
        rendered = AEG.render(
            catalog, environment,
            FIXTURES / "workflow-invariants.md", FIXTURES / "agent-environments.yaml",
        )
        self.assertIn("| `CONSUMES` | `state.input` | Input |", rendered)
        self.assertIn("| `EMITS` | `artifact.view` | View |", rendered)

    def test_relation_matrix_combines_relations_for_one_target(self):
        catalog, environment = self.load_fixture()
        rendered = AEG.render(
            catalog, environment,
            FIXTURES / "workflow-invariants.md", FIXTURES / "agent-environments.yaml",
        )
        self.assertIn("## Role × relation matrix", rendered)
        self.assertIn("| View | OWNS, EMITS |", rendered)

    def test_rendered_table_escapes_pipe_in_mutation_boundary(self):
        catalog, environment = self.load_fixture()
        environment["roles"]["role.builder"]["may_mutate"] = [{
            "target": "state.input", "boundary": "accepted | attributed",
        }]
        rendered = AEG.render(
            catalog, environment,
            FIXTURES / "workflow-invariants.md", FIXTURES / "agent-environments.yaml",
        )
        self.assertIn("| `MAY_MUTATE` | `state.input` | Input (boundary: accepted \\| attributed) |", rendered)


if __name__ == "__main__":
    unittest.main()
