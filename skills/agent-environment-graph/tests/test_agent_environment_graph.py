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

    def test_mermaid_layout_is_top_down_with_invisible_ordering_edges(self):
        catalog, environment = self.load_fixture()
        environment["roles"]["role.builder"]["may_observe"].append("artifact.view")
        rendered = AEG.render(
            catalog, environment,
            FIXTURES / "workflow-invariants.md", FIXTURES / "agent-environments.yaml",
        )
        self.assertIn("flowchart TB", rendered)
        self.assertIn("    direction TB", rendered)
        self.assertIn(
            "N_role_builder_4_state_input ~~~ N_role_builder_4_artifact_view",
            rendered,
        )
        self.assertIn(
            "N_role_builder_1_INV_001 ~~~ N_role_builder_2_capability_validate",
            rendered,
        )
        self.assertEqual(rendered.count("R -->|MAY_OBSERVE|"), 1)

    def test_invariants_use_catalog_text_and_semantic_color(self):
        catalog, environment = self.load_fixture()
        rendered = AEG.render(
            catalog, environment,
            FIXTURES / "workflow-invariants.md", FIXTURES / "agent-environments.yaml",
        )
        self.assertIn(
            'N_role_builder_1_INV_001["INV-001<br/>Human owns contract changes"]',
            rendered,
        )
        self.assertIn("class N_role_builder_1_INV_001 invAuthority", rendered)
        self.assertIn("classDef invAuthority fill:#fef3c7,color:#451a03", rendered)
        self.assertIn("one visible entry edge per relation group", rendered)
        self.assertEqual(rendered.count("## Color legend"), 1)
        self.assertIn("| 🟡\N{NO-BREAK SPACE}yellow | Authority |", rendered)
        self.assertIn("🟡\N{NO-BREAK SPACE}authority", rendered)
        self.assertNotIn(":blue_circle:", rendered)

    def test_relationship_table_surfaces_consumes_and_emits(self):
        catalog, environment = self.load_fixture()
        rendered = AEG.render(
            catalog, environment,
            FIXTURES / "workflow-invariants.md", FIXTURES / "agent-environments.yaml",
        )
        self.assertIn(
            "| `CONSUMES` | `state.input` | Input | 🔵\N{NO-BREAK SPACE}observation\N{WORD JOINER}/\N{WORD JOINER}input |",
            rendered,
        )
        self.assertIn(
            "| `EMITS` | `artifact.view` | View | 🟢\N{NO-BREAK SPACE}owned\N{WORD JOINER}/\N{WORD JOINER}output |",
            rendered,
        )

    def test_relation_matrix_combines_relations_for_one_target(self):
        catalog, environment = self.load_fixture()
        rendered = AEG.render(
            catalog, environment,
            FIXTURES / "workflow-invariants.md", FIXTURES / "agent-environments.yaml",
        )
        self.assertIn("## Role × relation matrix", rendered)
        self.assertIn(
            "| View | 🟢\N{NO-BREAK SPACE}owned\N{WORD JOINER}/\N{WORD JOINER}output OWNS, "
            "🟢\N{NO-BREAK SPACE}owned\N{WORD JOINER}/\N{WORD JOINER}output EMITS |",
            rendered,
        )

    def test_rendered_table_escapes_pipe_in_mutation_boundary(self):
        catalog, environment = self.load_fixture()
        environment["roles"]["role.builder"]["may_mutate"] = [{
            "target": "state.input", "boundary": "accepted | attributed",
        }]
        rendered = AEG.render(
            catalog, environment,
            FIXTURES / "workflow-invariants.md", FIXTURES / "agent-environments.yaml",
        )
        self.assertIn(
            "| `MAY_MUTATE` | `state.input` | Input (boundary: accepted \\| attributed) | "
            "🔴\N{NO-BREAK SPACE}mutation |",
            rendered,
        )

    def test_render_site_builds_overview_index_and_role_detail(self):
        catalog, environment = self.load_fixture()
        overview, pages = AEG.render_site(
            catalog, environment,
            FIXTURES / "workflow-invariants.md", FIXTURES / "agent-environments.yaml",
        )
        self.assertEqual(set(pages), {"README.md", "builder.md"})
        self.assertIn("[Open role view](agent-environment-views/builder.md)", overview)
        self.assertIn("## Role × relation matrix", overview)
        self.assertIn("[Open](./builder.md)", pages["README.md"])
        self.assertIn("# Builder Environment", pages["builder.md"])
        self.assertIn("## At a glance", pages["builder.md"])
        self.assertIn("## Environment graph", pages["builder.md"])
        self.assertIn("## Complete relation table", pages["builder.md"])
        self.assertIn("[shared legend](../agent-environment-graphs.md#color-legend)", pages["builder.md"])

    def test_site_check_detects_stale_role_page(self):
        catalog, environment = self.load_fixture()
        overview, pages = AEG.render_site(
            catalog, environment,
            FIXTURES / "workflow-invariants.md", FIXTURES / "agent-environments.yaml",
        )
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            output = root / "agent-environment-graphs.md"
            role_dir = root / "agent-environment-views"
            self.assertEqual(AEG.write_site(output, role_dir, overview, pages), [])
            AEG.check_site(output, role_dir, overview, pages)
            manual = role_dir / "manual-notes.md"
            stale = role_dir / "removed-role.md"
            manual.write_text("human-authored")
            stale.write_text(AEG.GENERATED_COMMENT + "\nstale")
            self.assertEqual(AEG.write_site(output, role_dir, overview, pages), [str(stale)])
            self.assertTrue(manual.is_file())
            (role_dir / "builder.md").write_text("stale")
            with self.assertRaisesRegex(AEG.GraphError, "role page is stale"):
                AEG.check_site(output, role_dir, overview, pages)


if __name__ == "__main__":
    unittest.main()
