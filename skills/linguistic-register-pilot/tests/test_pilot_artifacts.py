from __future__ import annotations

import importlib.util
import json
import subprocess
import sys
import tempfile
import unittest
from copy import deepcopy
from pathlib import Path

import yaml


ROOT = Path(__file__).parents[1]
SCRIPT = ROOT / "scripts" / "pilot_artifacts.py"
SPEC = importlib.util.spec_from_file_location("pilot_artifacts", SCRIPT)
assert SPEC and SPEC.loader
PILOT = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(PILOT)


def role() -> dict:
    return {
        "artifact_type": "linguistic_register_role_v1",
        "schema_version": 1,
        "status": "frozen",
        "role_id": "fixture-reviewer",
        "source": {
            "repository": "/fixture", "commit": "abc123",
            "files": [{"path": "SKILL.md", "sha256": "a" * 64, "role": "entrypoint"}],
        },
        "semantic_units": [{"id": "unit-1", "meaning": "Return evidence.", "source_locator": "line 1"}],
        "behavior_classifications": [{
            "id": "premise-judgment", "status": "encoded", "semantic_unit_ids": ["unit-1"],
            "rationale": "The fixture explicitly requires it.",
        }],
        "classification_owner": "fixture human",
        "limitations": [],
    }


def corpus() -> dict:
    return {
        "artifact_type": "linguistic_register_mini_corpus_v1",
        "schema_version": 1,
        "candidate_id": "fixture-practice",
        "target_practice": "Fixture practice",
        "selection_basis": "One formal and one responsive work.",
        "sources": [
            {
                "id": "formal", "title": "Formal work", "authors": ["Expert"],
                "authorship_basis": "Solo authored", "genre": "formal",
                "public_url": "https://example.test/formal", "access_basis": "Public fixture",
                "rights_status": "open_license", "content_sha256": "b" * 64,
            },
            {
                "id": "response", "title": "Response", "authors": ["Expert"],
                "authorship_basis": "Solo authored", "genre": "responsive_or_informal",
                "public_url": "https://example.test/response", "access_basis": "Public fixture",
                "rights_status": "public_readable_reuse_unconfirmed", "content_sha256": "c" * 64,
            },
        ],
        "limitations": [],
    }


def profile(role_digest: str, corpus_digest: str) -> dict:
    common = {
        "layer": "surface", "category": "cadence", "distinctiveness_weight": 1.0,
        "evidence": [
            {"source_id": "formal", "locator": "section 1", "observation": "Observed in formal work."},
            {"source_id": "response", "locator": "paragraph 2", "observation": "Observed in response."},
        ],
        "cross_genre": True,
        "disposition_rationale": "Changes realization without changing the role operation.",
    }
    features = []
    for index in range(3):
        feature = deepcopy(common)
        feature.update({
            "id": f"retained-{index}", "description": f"Abstract cadence feature {index}.",
            "disposition": "realization_only",
        })
        features.append(feature)
    excluded = deepcopy(common)
    excluded.update({
        "id": "excluded-method", "layer": "discourse", "category": "claim_posture",
        "description": "A substantive reasoning operation.", "distinctiveness_weight": 2.0,
        "disposition": "semantic_addition",
        "disposition_rationale": "Would introduce a new reasoning operation.",
    })
    features.append(excluded)
    return {
        "artifact_type": "linguistic_register_profile_v1",
        "schema_version": 1,
        "candidate_id": "fixture-practice",
        "role_id": "fixture-reviewer",
        "role_artifact_sha256": role_digest,
        "corpus_artifact_sha256": corpus_digest,
        "features": features,
        "content_screening": {
            "names_removed": True, "copied_language_removed": True, "domain_terms_removed": True,
            "named_methods_removed": True, "reviewer": "fixture reviewer",
            "rationale": "Retained descriptions were screened.",
        },
        "thresholds": {
            "minimum_retained_features": 3, "minimum_weighted_retention": 0.5,
            "minimum_cross_genre_retained_features": 2,
        },
        "judgment_provenance": {
            "extractor": "fixture extractor", "semantic_classifier": "fixture classifier",
            "weighting_basis": "Frozen fixture evidence",
            "weights_assigned_before_semantic_classification": True,
            "performed_before_outcomes": True,
        },
        "limitations": [],
    }


class PilotArtifactsTest(unittest.TestCase):
    def write_yaml(self, directory: Path, name: str, value: dict) -> Path:
        path = directory / name
        path.write_text(yaml.safe_dump(value, sort_keys=False), encoding="utf-8")
        return path

    def make_artifacts(self, directory: Path) -> tuple[Path, Path, Path]:
        source_path = directory / "SKILL.md"
        source_path.write_text("fixture role\n", encoding="utf-8")
        subprocess.run(["git", "init", "-q", str(directory)], check=True)
        subprocess.run(["git", "-C", str(directory), "add", "SKILL.md"], check=True)
        subprocess.run([
            "git", "-C", str(directory), "-c", "user.name=Fixture", "-c",
            "user.email=fixture@example.test", "commit", "-qm", "fixture role",
        ], check=True)
        commit = subprocess.run(
            ["git", "-C", str(directory), "rev-parse", "HEAD"], check=True, capture_output=True, text=True,
        ).stdout.strip()
        role_value = role()
        role_value["source"]["repository"] = str(directory)
        role_value["source"]["commit"] = commit
        role_value["source"]["files"][0]["sha256"] = PILOT.sha256_file(source_path)
        role_path = self.write_yaml(directory, "role.yaml", role_value)
        corpus_path = self.write_yaml(directory, "corpus.yaml", corpus())
        profile_path = self.write_yaml(
            directory, "profile.yaml", profile(PILOT.artifact_digest(role_path), PILOT.artifact_digest(corpus_path)),
        )
        return role_path, corpus_path, profile_path

    def test_vertical_evaluation_emits_bound_viable_report(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            directory = Path(temporary)
            role_path, corpus_path, profile_path = self.make_artifacts(directory)
            completed = subprocess.run([
                sys.executable, str(SCRIPT), "evaluate", "--role", str(role_path),
                "--corpus", str(corpus_path), "--profile", str(profile_path),
            ], check=True, capture_output=True, text=True)
            report = json.loads(completed.stdout)
            self.assertEqual(report["disposition"], "candidate_viable")
            self.assertEqual(report["metrics"]["weighted_retention"], 0.6)
            self.assertEqual(report["metrics"]["counts_by_disposition"]["semantic_addition"], 1)
            self.assertEqual(report["bindings"]["profile_artifact_sha256"], PILOT.artifact_digest(profile_path))
            self.assertIn("human acceptance", report["authority"])

    def test_profile_rejects_stale_role_binding(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            directory = Path(temporary)
            role_path, corpus_path, profile_path = self.make_artifacts(directory)
            value = yaml.safe_load(profile_path.read_text(encoding="utf-8"))
            value["role_artifact_sha256"] = "f" * 64
            profile_path = self.write_yaml(directory, "stale-profile.yaml", value)
            with self.assertRaisesRegex(PILOT.PilotError, "does not match role artifact"):
                PILOT.validate_profile(
                    PILOT.load_yaml(profile_path), PILOT.validate_role(PILOT.load_yaml(role_path)),
                    PILOT.validate_corpus(PILOT.load_yaml(corpus_path)), PILOT.artifact_digest(role_path),
                    PILOT.artifact_digest(corpus_path),
                )

    def test_cli_rejects_changed_canonical_role_source(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            directory = Path(temporary)
            role_path, _, _ = self.make_artifacts(directory)
            (directory / "SKILL.md").write_text("changed role\n", encoding="utf-8")
            completed = subprocess.run([
                sys.executable, str(SCRIPT), "validate-role", str(role_path),
            ], capture_output=True, text=True)
            self.assertEqual(completed.returncode, 2)
            self.assertIn("does not match recorded bytes", completed.stderr)

    def test_cross_genre_claim_requires_both_genres(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            directory = Path(temporary)
            role_path, corpus_path, profile_path = self.make_artifacts(directory)
            value = yaml.safe_load(profile_path.read_text(encoding="utf-8"))
            value["features"][0]["evidence"] = [value["features"][0]["evidence"][0]]
            profile_path = self.write_yaml(directory, "one-genre.yaml", value)
            with self.assertRaisesRegex(PILOT.PilotError, "requires evidence from both genres"):
                PILOT.validate_profile(
                    PILOT.load_yaml(profile_path), PILOT.validate_role(PILOT.load_yaml(role_path)),
                    PILOT.validate_corpus(PILOT.load_yaml(corpus_path)), PILOT.artifact_digest(role_path),
                    PILOT.artifact_digest(corpus_path),
                )

    def test_evaluation_requires_human_frozen_role(self) -> None:
        role_value = role()
        role_value["status"] = "awaiting_human_acceptance"
        with self.assertRaisesRegex(PILOT.PilotError, "must be frozen"):
            PILOT.evaluate(role_value, corpus(), {"features": [], "thresholds": {}}, "a" * 64, "b" * 64, "c" * 64)

    def test_output_refuses_overwrite(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            directory = Path(temporary)
            role_path, corpus_path, profile_path = self.make_artifacts(directory)
            output = directory / "report.json"
            output.write_text("owned", encoding="utf-8")
            completed = subprocess.run([
                sys.executable, str(SCRIPT), "evaluate", "--role", str(role_path),
                "--corpus", str(corpus_path), "--profile", str(profile_path), "--output", str(output),
            ], capture_output=True, text=True)
            self.assertEqual(completed.returncode, 2)
            self.assertIn("refusing to overwrite", completed.stderr)
            self.assertEqual(output.read_text(encoding="utf-8"), "owned")

    def test_unknown_fields_and_yaml_aliases_fail_closed(self) -> None:
        value = role()
        value["extra"] = "not allowed"
        with self.assertRaisesRegex(PILOT.PilotError, "unknown fields"):
            PILOT.validate_role(value)
        with tempfile.TemporaryDirectory() as temporary:
            path = Path(temporary) / "aliased.yaml"
            path.write_text("base: &base value\ncopy: *base\n", encoding="utf-8")
            with self.assertRaisesRegex(PILOT.PilotError, "anchors and aliases"):
                PILOT.load_yaml(path)


if __name__ == "__main__":
    unittest.main()
