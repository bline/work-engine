from __future__ import annotations

import importlib.util
import json
import re
import subprocess
import sys
import tempfile
import unittest
from copy import deepcopy
from pathlib import Path


ROOT = Path(__file__).parents[1]
SCRIPT = ROOT / "scripts" / "comparison_artifacts.py"
FIXTURE = Path(__file__).parent / "fixtures" / "pilot"
YAML_CONFIG = ROOT.parents[1] / "campaigns" / "comparative-repository-analysis.yaml"
SPEC = importlib.util.spec_from_file_location("comparison_artifacts", SCRIPT)
assert SPEC and SPEC.loader
ARTIFACTS = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(ARTIFACTS)


def load(name: str):
    return json.loads((FIXTURE / name).read_text(encoding="utf-8"))


def make_git_repo(path: Path, remote: str | None = "https://example.test/acme/sample.git") -> str:
    path.mkdir()
    subprocess.run(["git", "-C", str(path), "init", "-q"], check=True)
    subprocess.run(["git", "-C", str(path), "config", "user.email", "test@example.test"], check=True)
    subprocess.run(["git", "-C", str(path), "config", "user.name", "Test"], check=True)
    (path / "README.md").write_text("fixture\n", encoding="utf-8")
    subprocess.run(["git", "-C", str(path), "add", "README.md"], check=True)
    subprocess.run(["git", "-C", str(path), "commit", "-qm", "fixture"], check=True)
    if remote:
        subprocess.run(["git", "-C", str(path), "remote", "add", "origin", remote], check=True)
    return subprocess.check_output(["git", "-C", str(path), "rev-parse", "HEAD"], text=True).strip()


class ComparisonArtifactsTest(unittest.TestCase):
    def setUp(self) -> None:
        self.contract = load("contract.json")
        self.control = load("pass-control.json")
        self.review = load("pass-review.json")

    def test_vertical_pilot_reconciles_compatible_passes_and_validates_separate_decision(self) -> None:
        completed = subprocess.run(
            [sys.executable, str(SCRIPT), "pilot", "--fixture-dir", str(FIXTURE)],
            check=True, capture_output=True, text=True,
        )
        result = json.loads(completed.stdout)
        self.assertEqual(result["status"], "pilot-complete")
        self.assertTrue(result["decision_separate"])
        profile = result["profile"]
        self.assertEqual(profile["dimension_states"], {"control": "analyzed", "review": "not_observed"})
        self.assertEqual(profile["capabilities"][0]["relationships"]["depends_on"], ["route-revision"])
        self.assertNotIn("classification", json.dumps(profile))

    def test_yaml_first_prompt_entry_refuses_unresolved_inputs_and_prepares_valid_contract(self) -> None:
        import yaml

        with tempfile.TemporaryDirectory() as directory:
            directory = Path(directory)
            blocked_output = directory / "blocked.json"
            blocked = subprocess.run(
                [sys.executable, str(SCRIPT), "prepare-run", "--config", str(YAML_CONFIG),
                 "--output", str(blocked_output)],
                capture_output=True, text=True,
            )
            self.assertEqual(blocked.returncode, 3)
            self.assertEqual(json.loads(blocked.stdout)["status"], "not_ready")
            self.assertFalse(blocked_output.exists())

            config = yaml.safe_load(YAML_CONFIG.read_text(encoding="utf-8"))
            config["status"] = "ready"
            repository_path = directory / "sample"
            head = make_git_repo(repository_path)
            config["repositories"] = [{
                "path": str(repository_path), "inclusion_reason": "heterogeneous pilot"
            }]
            config["work_engine_snapshot_policy"]["revision"] = "def456"
            ready_yaml = directory / "ready.yaml"
            ready_yaml.write_text(yaml.safe_dump(config, sort_keys=False), encoding="utf-8")
            output = directory / "contract.json"
            prepared = subprocess.run(
                [sys.executable, str(SCRIPT), "prepare-run", "--config", str(ready_yaml),
                 "--output", str(output)],
                check=True, capture_output=True, text=True,
            )
            self.assertEqual(json.loads(prepared.stdout)["status"], "prepared")
            contract = ARTIFACTS.validate_contract(json.loads(output.read_text(encoding="utf-8")))
            self.assertEqual(contract["repository_corpus"][0]["commit"], head)
            self.assertEqual(contract["work_engine_snapshot"]["commit"], "def456")

    def test_yaml_entry_rejects_aliases(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "aliases.yaml"
            path.write_text("question: &question compare\ncopy: *question\n", encoding="utf-8")
            with self.assertRaisesRegex(ARTIFACTS.ArtifactError, "anchors and aliases"):
                ARTIFACTS.load_yaml_config(path)

    def test_path_shorthand_derives_git_metadata_and_only_requires_semantic_reason(self) -> None:
        import yaml
        with tempfile.TemporaryDirectory() as directory:
            repo = Path(directory) / "derived-repo"
            head = make_git_repo(repo, "https://github.com/acme/derived-repo.git")
            config = yaml.safe_load(YAML_CONFIG.read_text(encoding="utf-8"))
            config["repositories"] = [str(repo)]
            _, missing, derived = ARTIFACTS.normalize_yaml_config(config)
            self.assertTrue(any("inclusion_reason" in value for value in missing))
            self.assertEqual(derived[0]["checkout_path"], str(repo.resolve()))
            self.assertEqual(derived[0]["commit"], head)
            self.assertEqual(derived[0]["id"], "derived-repo")
            self.assertEqual(derived[0]["git_remote"], "https://github.com/acme/derived-repo.git")
            self.assertNotIn("index_project", derived[0])

    def test_git_derivation_allows_detached_and_no_remote_but_rejects_dirty(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            repo = Path(directory) / "local-only"
            head = make_git_repo(repo, None)
            subprocess.run(["git", "-C", str(repo), "checkout", "--detach", "-q", head], check=True)
            derived, problems = ARTIFACTS.derive_repository({"path": str(repo), "inclusion_reason": "local fixture"}, 0)
            self.assertFalse(problems)
            self.assertTrue(derived["detached"])
            self.assertIsNone(derived["git_remote"])
            self.assertEqual(derived["identity_source"], "local_path")
            (repo / "dirty.txt").write_text("dirty\n", encoding="utf-8")
            _, problems = ARTIFACTS.derive_repository({"path": str(repo), "inclusion_reason": "local fixture"}, 0)
            self.assertTrue(any("dirty" in value for value in problems))
            with self.assertRaisesRegex(ARTIFACTS.ArtifactError, "unknown fields: repository"):
                ARTIFACTS.derive_repository({"path": str(repo), "repository": "ignored"}, 0)

    def test_unpinned_index_project_is_discovered_and_content_bound(self) -> None:
        contract = deepcopy(self.contract)
        del contract["repository_corpus"][0]["index_project"]
        artifact = deepcopy(self.control)
        ARTIFACTS.validate_dimension_pass(artifact, ARTIFACTS.validate_contract(contract))
        profile = ARTIFACTS.reconcile(contract, [artifact, deepcopy(self.review)], "sample")
        self.assertEqual(profile["provenance"]["index_receipt"]["observed"]["project"], "fixture-sample")
        profile["provenance"]["index_receipt"]["observed"]["project"] = "substituted"
        with self.assertRaisesRegex(ARTIFACTS.ArtifactError, "digest is incompatible"):
            ARTIFACTS.validate_profile(profile, contract)

    def test_ontology_gap_proposal_is_evidence_backed_and_outside_frozen_ontology(self) -> None:
        proposal = {
            "artifact_type": "ontology_gap_proposal_v1", "compatibility": deepcopy(self.contract["compatibility"]),
            "gap_id": "gap-emergent-coordination", "status": "proposed", "outside_active_ontology": True,
            "repository_id": "sample", "source_dimension": "control",
            "source_pass_sha256": ARTIFACTS.artifact_digest(self.control),
            "claim_ids": ["obs-control"], "evidence_ids": ["ev-control"],
            "proposed_dimension": "emergent_coordination", "proposed_definition": "Coordination behavior not represented by control alone.",
            "why_ontology_is_insufficient": "The mechanism crosses frozen dimensions without equivalent semantics.",
            "decision_relevance": "Could change a later adoption decision.", "confidence": "medium",
            "alternatives": ["Treat as a relationship edge"], "limitations": ["Observed in one repository"]
        }
        ARTIFACTS.validate_ontology_gap(proposal, self.contract, self.control)
        proposal["proposed_dimension"] = "control"
        with self.assertRaisesRegex(ARTIFACTS.ArtifactError, "already exists"):
            ARTIFACTS.validate_ontology_gap(proposal, self.contract, self.control)

    def test_documented_pilot_pipeline_runs_end_to_end(self) -> None:
        skill = (ROOT / "SKILL.md").read_text(encoding="utf-8")
        prefix = "python3 work-engine/skills/comparative-repository-analysis/scripts/comparison_artifacts.py"
        self.assertNotIn("python3 scripts/comparison_artifacts.py", skill)
        self.assertIn("work-engine/campaigns/comparative-repository-analysis.yaml", skill)
        self.assertIn(prefix, skill)
        match = re.search(
            r"<!-- executable-pilot:start -->\s*```bash\n(?P<commands>.*?)\n```\s*<!-- executable-pilot:end -->",
            skill,
            re.DOTALL,
        )
        self.assertIsNotNone(match, "SKILL.md must contain one marked executable pilot block")
        repository_root = ROOT.parents[2]
        completed = subprocess.run(
            ["bash", "-euo", "pipefail", "-c", match.group("commands")],
            cwd=repository_root, capture_output=True, text=True,
        )
        self.assertEqual(completed.returncode, 0, completed.stderr)
        self.assertIn('"gap_id": "gap-emergent-coordination"', completed.stdout)
        self.assertIn('"status": "compatible"', completed.stdout)
        self.assertIn('"classification": "INVESTIGATE"', completed.stdout)
        for schema in (ROOT / "schemas").glob("*.json"):
            self.assertIn("authoritative semantic validator", json.loads(schema.read_text(encoding="utf-8"))["$comment"])

    def test_prepare_run_requires_comparative_kind(self) -> None:
        import yaml

        config = yaml.safe_load(YAML_CONFIG.read_text(encoding="utf-8"))
        self.assertEqual(config.pop("kind"), "comparative_repository_analysis")
        with self.assertRaisesRegex(ARTIFACTS.ArtifactError, "missing fields: kind"):
            ARTIFACTS.normalize_yaml_config(config)

    def test_yaml_run_identity_covers_full_contract_semantics(self) -> None:
        import yaml

        with tempfile.TemporaryDirectory() as directory:
            repo = Path(directory) / "one"
            make_git_repo(repo)
            config = yaml.safe_load(YAML_CONFIG.read_text(encoding="utf-8"))
            config["status"] = "ready"
            config["repositories"] = [{"path": str(repo), "inclusion_reason": "pilot"}]
            config["work_engine_snapshot_policy"]["revision"] = "def"
            first, missing, _ = ARTIFACTS.normalize_yaml_config(config)
            self.assertFalse(missing)
            changed = deepcopy(config)
            changed["question"] = "A materially different comparison question"
            second, missing, _ = ARTIFACTS.normalize_yaml_config(changed)
            self.assertFalse(missing)
            self.assertNotEqual(first["compatibility"]["run_id"], second["compatibility"]["run_id"])

    def test_contract_rejects_malformed_adaptive_dimensions_and_duplicate_repositories(self) -> None:
        malformed = deepcopy(self.contract)
        malformed["adaptive_dimensions"] = [{}]
        with self.assertRaisesRegex(ARTIFACTS.ArtifactError, "adaptive_dimensions"):
            ARTIFACTS.validate_contract(malformed)
        duplicated = deepcopy(self.contract)
        duplicated["repository_corpus"].append(deepcopy(duplicated["repository_corpus"][0]))
        with self.assertRaisesRegex(ARTIFACTS.ArtifactError, "duplicate"):
            ARTIFACTS.validate_contract(duplicated)

    def test_profile_validation_rejects_missing_and_unknown_fields_cleanly(self) -> None:
        profile = ARTIFACTS.reconcile(ARTIFACTS.validate_contract(self.contract), [self.control, self.review], "sample")
        missing = deepcopy(profile)
        del missing["repository_id"]
        with self.assertRaisesRegex(ARTIFACTS.ArtifactError, "missing fields"):
            ARTIFACTS.validate_profile(missing, self.contract)
        profile["surprise"] = True
        with self.assertRaisesRegex(ARTIFACTS.ArtifactError, "unknown fields"):
            ARTIFACTS.validate_profile(profile, self.contract)

    def test_dimension_pass_rejects_malformed_structural_fields(self) -> None:
        for field, value in (("unknowns", {}), ("system_boundary", []), ("provenance", [])):
            with self.subTest(field=field):
                artifact = deepcopy(self.control)
                artifact[field] = value
                with self.assertRaises(ARTIFACTS.ArtifactError):
                    ARTIFACTS.validate_dimension_pass(artifact, ARTIFACTS.validate_contract(self.contract))

    def test_profile_rejects_dimensions_outside_frozen_ontology(self) -> None:
        profile = ARTIFACTS.reconcile(ARTIFACTS.validate_contract(self.contract), [self.control, self.review], "sample")
        profile["dimension_states"]["invented"] = "analyzed"
        with self.assertRaisesRegex(ARTIFACTS.ArtifactError, "frozen ontology"):
            ARTIFACTS.validate_profile(profile, self.contract)

    def test_three_way_dimension_conflict_preserves_observed_values(self) -> None:
        second = deepcopy(self.control)
        second["dimension_state"] = "unknown"
        third = deepcopy(self.control)
        third["dimension_state"] = "not_observed"
        profile = ARTIFACTS.reconcile(ARTIFACTS.validate_contract(self.contract), [self.control, second, third, self.review], "sample")
        conflicts = [item for item in profile["reconciliation"]["conflicts"] if item["kind"] == "dimension_state"]
        self.assertEqual(conflicts[-1]["values"], ["analyzed", "not_observed", "unknown"])
        self.assertNotIn("conflicting_evidence", conflicts[-1]["values"])

    def test_decision_word_in_descriptive_text_is_not_a_decision_artifact(self) -> None:
        profile = ARTIFACTS.reconcile(ARTIFACTS.validate_contract(self.contract), [self.control, self.review], "sample")
        profile["claims"][0]["statement"] = "ADOPT"
        ARTIFACTS.validate_profile(profile, self.contract)

    def test_external_profile_revalidates_semantic_references(self) -> None:
        profile = ARTIFACTS.reconcile(ARTIFACTS.validate_contract(self.contract), [self.control, self.review], "sample")
        profile["claims"][0]["evidence_ids"] = ["missing"]
        with self.assertRaisesRegex(ARTIFACTS.ArtifactError, "unknown evidence"):
            ARTIFACTS.validate_profile(profile, self.contract)

    def test_capability_definition_disagreement_is_preserved_not_aborted(self) -> None:
        second = deepcopy(self.control)
        second["capabilities"][0]["mechanism"] = "A conflicting analyst interpretation of the mechanism."
        profile = ARTIFACTS.reconcile(ARTIFACTS.validate_contract(self.contract), [self.control, second, self.review], "sample")
        conflicts = profile["reconciliation"]["conflicts"]
        identity = next(item for item in conflicts if item["kind"] == "capability_identity")
        self.assertEqual(identity["subject_id"], "route-revision")
        self.assertEqual(len(identity["definitions"]), 2)

    def test_freeze_contract_refuses_to_overwrite_frozen_state(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory) / "contract.json"
            output.write_text("existing", encoding="utf-8")
            completed = subprocess.run(
                [sys.executable, str(SCRIPT), "freeze-contract", str(FIXTURE / "contract.json"), str(output)],
                capture_output=True, text=True,
            )
            self.assertEqual(completed.returncode, 2)
            self.assertEqual(output.read_text(encoding="utf-8"), "existing")

    def test_incompatible_run_is_rejected(self) -> None:
        artifact = deepcopy(self.control)
        artifact["compatibility"]["run_id"] = "another-run"
        with self.assertRaisesRegex(ARTIFACTS.ArtifactError, "run_id is incompatible"):
            ARTIFACTS.validate_dimension_pass(artifact, ARTIFACTS.validate_contract(self.contract))

    def test_closed_uncertainty_vocabulary_is_enforced(self) -> None:
        artifact = deepcopy(self.review)
        artifact["dimension_state"] = "probably_missing"
        with self.assertRaisesRegex(ARTIFACTS.ArtifactError, "dimension_state"):
            ARTIFACTS.validate_dimension_pass(artifact, ARTIFACTS.validate_contract(self.contract))

    def test_confirmed_absent_requires_bounded_exhaustive_search(self) -> None:
        artifact = deepcopy(self.review)
        artifact["dimension_state"] = "confirmed_absent"
        with self.assertRaisesRegex(ARTIFACTS.ArtifactError, "negative_search"):
            ARTIFACTS.validate_dimension_pass(artifact, ARTIFACTS.validate_contract(self.contract))
        artifact["negative_search"] = {"scope": "src/review", "exhaustive": True, "coverage_limitations": ["generated code excluded"]}
        ARTIFACTS.validate_dimension_pass(artifact, ARTIFACTS.validate_contract(self.contract))

    def test_interpretation_requires_observation_parent(self) -> None:
        artifact = deepcopy(self.control)
        artifact["claims"][1]["parent_claim_ids"] = []
        with self.assertRaisesRegex(ARTIFACTS.ArtifactError, "must reference an observation"):
            ARTIFACTS.validate_dimension_pass(artifact, ARTIFACTS.validate_contract(self.contract))

    def test_dangling_relationship_is_rejected(self) -> None:
        artifact = deepcopy(self.control)
        artifact["capabilities"][0]["relationships"]["enables"] = ["missing"]
        with self.assertRaisesRegex(ARTIFACTS.ArtifactError, "unknown capability"):
            ARTIFACTS.validate_dimension_pass(artifact, ARTIFACTS.validate_contract(self.contract))

    def test_reconciliation_preserves_claim_disagreement(self) -> None:
        second = deepcopy(self.control)
        second["claims"] = second["claims"][:2]
        second["claims"][0]["id"] = "obs-control-alternative"
        second["claims"][1]["id"] = "int-control-alternative"
        second["claims"][1]["statement"] = "The route is a terminal retry, not revision."
        second["claims"][1]["parent_claim_ids"] = ["obs-control-alternative"]
        second["evidence"][0]["id"] = "ev-control-alternative"
        second["evidence"][0]["supports"] = ["obs-control-alternative", "int-control-alternative"]
        for claim in second["claims"]:
            claim["evidence_ids"] = ["ev-control-alternative" for _ in claim["evidence_ids"]]
        for capability in second["capabilities"]:
            capability["evidence_ids"] = ["ev-control-alternative"]
        profile = ARTIFACTS.reconcile(ARTIFACTS.validate_contract(self.contract), [self.control, second, self.review], "sample")
        conflicts = profile["reconciliation"]["conflicts"]
        self.assertTrue(any(value["kind"] == "claim_disagreement" for value in conflicts))
        self.assertEqual(len([claim for claim in profile["claims"] if claim["stage"] == "interpretation"]), 2)

    def test_profile_cannot_embed_adoption_decision(self) -> None:
        profile = ARTIFACTS.reconcile(ARTIFACTS.validate_contract(self.contract), [self.control, self.review], "sample")
        profile["decision"] = {"classification": "ADOPT"}
        with self.assertRaisesRegex(ARTIFACTS.ArtifactError, "decision fields"):
            ARTIFACTS.validate_profile(profile, self.contract)

    def test_decision_requires_qualified_existing_profile_claim(self) -> None:
        profile = ARTIFACTS.reconcile(ARTIFACTS.validate_contract(self.contract), [self.control, self.review], "sample")
        decision = load("decision.json")
        ARTIFACTS.validate_decision(decision, self.contract, [profile])
        decision["profile_claim_refs"] = ["imp-control"]
        with self.assertRaisesRegex(ARTIFACTS.ArtifactError, "unknown or unqualified"):
            ARTIFACTS.validate_decision(decision, self.contract, [profile])

    def test_dimension_pass_requires_exact_terminal_index_readiness(self) -> None:
        ready = deepcopy(self.control)
        ARTIFACTS.validate_dimension_pass(ready, ARTIFACTS.validate_contract(self.contract))
        for mutation, message in (
            (("state", "timed_out"), "blocked by index state"),
            (("state", "revision_mismatch"), "blocked by index state"),
        ):
            with self.subTest(state=mutation[1]):
                artifact = deepcopy(ready)
                artifact["index_receipt"][mutation[0]] = mutation[1]
                artifact["index_receipt"]["failure"] = {"kind": mutation[1], "detail": "fixture terminal failure"}
                with self.assertRaisesRegex(ARTIFACTS.ArtifactError, message):
                    ARTIFACTS.validate_dimension_pass(artifact, self.contract)
        missing = deepcopy(ready)
        del missing["index_receipt"]
        with self.assertRaisesRegex(ARTIFACTS.ArtifactError, "missing fields"):
            ARTIFACTS.validate_dimension_pass(missing, self.contract)
        mismatch = deepcopy(ready)
        mismatch["index_receipt"]["observed"]["revision"] = "stale"
        with self.assertRaisesRegex(ARTIFACTS.ArtifactError, "revision is incompatible"):
            ARTIFACTS.validate_dimension_pass(mismatch, self.contract)
        limited = deepcopy(ready)
        limited["index_receipt"]["state"] = "coverage_limited"
        limited["index_receipt"]["coverage"]["signal"] = "limited"
        limited["index_receipt"]["coverage"]["skipped"] = ["generated/unavailable.py"]
        with self.assertRaisesRegex(ARTIFACTS.ArtifactError, "fallback decision"):
            ARTIFACTS.validate_dimension_pass(limited, self.contract)
        limited["index_fallback_decision"] = {
            "artifact_type": "index_fallback_decision_v1",
            "compatibility": deepcopy(self.contract["compatibility"]),
            "repository_id": "sample", "receipt_id": "index-sample-1",
            "receipt_sha256": ARTIFACTS.receipt_digest(limited["index_receipt"]),
            "decision": "targeted_direct_source", "decided_by": "comparison_model",
            "reason": "The skipped generated file is not relevant to this bounded pass."
        }
        ARTIFACTS.validate_dimension_pass(limited, self.contract)
        other_generation = deepcopy(self.review)
        other_generation["index_receipt"]["observed"]["generation"] = "another-generation"
        with self.assertRaisesRegex(ARTIFACTS.ArtifactError, "same complete index receipt"):
            ARTIFACTS.reconcile(self.contract, [ready, other_generation], "sample")

    def test_index_receipt_coverage_action_digest_and_profile_binding_fail_closed(self) -> None:
        artifact = deepcopy(self.control)
        artifact["index_receipt"]["coverage"]["skipped"] = ["src/generated.py"]
        with self.assertRaisesRegex(ARTIFACTS.ArtifactError, "coverage entries"):
            ARTIFACTS.validate_dimension_pass(artifact, self.contract)
        artifact = deepcopy(self.control)
        artifact["index_receipt"]["action"] = "none"
        with self.assertRaisesRegex(ARTIFACTS.ArtifactError, "observed index action"):
            ARTIFACTS.validate_dimension_pass(artifact, self.contract)
        profile = ARTIFACTS.reconcile(self.contract, [self.control, self.review], "sample")
        profile["provenance"]["index_receipt"]["observed"]["generation"] = "tampered"
        with self.assertRaisesRegex(ARTIFACTS.ArtifactError, "digest is incompatible"):
            ARTIFACTS.validate_profile(profile, self.contract)
        oversized = deepcopy(self.control)
        oversized["index_receipt"]["limitations"] = ["x" * 1001]
        with self.assertRaisesRegex(ARTIFACTS.ArtifactError, "compact limit"):
            ARTIFACTS.validate_dimension_pass(oversized, self.contract)


if __name__ == "__main__":
    unittest.main()
