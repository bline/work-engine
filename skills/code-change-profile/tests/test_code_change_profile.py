from __future__ import annotations

import importlib.util
import json
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
SCRIPT = ROOT / "skills/code-change-profile/scripts/code_change_profile.py"
FIXTURE = ROOT / "skills/code-change-profile/tests/fixtures/proposal-packets-accepted.json"

spec = importlib.util.spec_from_file_location("code_change_profile", SCRIPT)
assert spec and spec.loader
PROFILE = importlib.util.module_from_spec(spec)
spec.loader.exec_module(PROFILE)


class CodeChangeProfileTest(unittest.TestCase):
    def subject(self):
        return json.loads(FIXTURE.read_text())

    def test_real_checkpoint_is_recomputed_deterministically_for_fresh_consumer(self):
        first = PROFILE.profile(self.subject(), ROOT)
        second = PROFILE.profile(self.subject(), ROOT)
        first_bytes = PROFILE.canonical(first)
        self.assertEqual(first_bytes, PROFILE.canonical(second))

        consumed = json.loads(first_bytes)
        self.assertEqual(consumed["subject"]["checkpoint"]["checkpoint_kind"], "accepted")
        self.assertEqual(consumed["observations"]["file_count"], {"state": "observed", "value": 11})
        self.assertGreater(consumed["observations"]["hunk_count"]["value"], 0)
        self.assertEqual(consumed["observations"]["test_file_count"]["value"], 5)
        symbols = consumed["observations"]["changed_symbols"]["value"]
        self.assertTrue(any(item["measurement"]["state"] == "observed" for item in symbols))
        self.assertTrue(any(item["measurement"]["state"] == "unsupported" for item in symbols))
        self.assertEqual(consumed["coverage"]["manifest_paths"], 11)
        sources = consumed["provenance"]["derivation_sources"]
        self.assertEqual(set(sources["git_runtime"]["identity"]), {
            "version", "executable_sha256"
        })
        self.assertEqual(set(sources["python_runtime"]["identity"]), {
            "implementation", "version", "executable_sha256"
        })
        for name in ("structural_graph", "invariant_catalog", "classifier"):
            self.assertEqual(sources[name], {
                "use_state": "not_used", "reason": "deferred_by_profile_scope"
            })
            self.assertNotIn(name, consumed["observations"])
        self.assertIs(PROFILE.validate_profile(consumed), consumed)

    def test_cli_output_is_byte_identical_and_ignores_current_worktree(self):
        command = [sys.executable, str(SCRIPT), "profile", "--receipt", str(FIXTURE), "--repository", str(ROOT)]
        first = subprocess.run(command, stdout=subprocess.PIPE, stderr=subprocess.PIPE, check=True).stdout
        second = subprocess.run(command, stdout=subprocess.PIPE, stderr=subprocess.PIPE, check=True).stdout
        self.assertEqual(first, second)
        self.assertNotIn(b"code-change-characterization-baseline", first)

    def test_tampered_immutable_binding_is_rejected(self):
        subject = self.subject()
        subject["checkpoint"]["baseline_tree_oid"] = subject["checkpoint"]["checkpoint_tree_oid"]
        with self.assertRaisesRegex(PROFILE.ProfileError, "tree identity"):
            PROFILE.profile(subject, ROOT)

    def test_measurement_states_do_not_collapse(self):
        self.assertEqual(PROFILE.measurement("observed", value=0), {"state": "observed", "value": 0})
        for state in ("unknown", "unsupported", "failed", "not_applicable"):
            self.assertEqual(PROFILE.measurement(state, reason=state), {"state": state, "reason": state})
        with self.assertRaises(PROFILE.ProfileError):
            PROFILE.measurement("unknown", value=0, reason="missing")
        with self.assertRaises(PROFILE.ProfileError):
            PROFILE.measurement("failed")

    def test_lossy_metrics_projection_is_not_a_subject(self):
        projection = dict(self.subject()["checkpoint"])
        for field in ("baseline_commit_oid", "baseline_tree_oid", "paths"):
            projection.pop(field)
        subject = self.subject()
        subject["checkpoint"] = projection
        with self.assertRaises(PROFILE.ProfileError):
            PROFILE.profile(subject, ROOT)

    def test_profile_digest_and_measurement_shape_fail_closed(self):
        result = PROFILE.profile(self.subject(), ROOT)
        result["observations"]["file_count"]["value"] = 12
        with self.assertRaisesRegex(PROFILE.ProfileError, "profile digest"):
            PROFILE.validate_profile(result)

    def test_dirty_checkpoint_validator_fails_closed_before_import(self):
        with tempfile.TemporaryDirectory() as directory:
            repository = Path(directory)
            target = repository / "skills/slice-checkpoint/scripts/checkpoint.py"
            target.parent.mkdir(parents=True)
            target.write_bytes((ROOT / "skills/slice-checkpoint/scripts/checkpoint.py").read_bytes() + b"\n# dirty\n")
            with self.assertRaisesRegex(PROFILE.ProfileError, "analyzer-version binding"):
                PROFILE.checkpoint_module(repository)

    def test_adversarial_checkpoint_code_is_not_executed_before_binding(self):
        with tempfile.TemporaryDirectory() as directory:
            repository = Path(directory)
            marker = repository / "executed"
            target = repository / "skills/slice-checkpoint/scripts/checkpoint.py"
            target.parent.mkdir(parents=True)
            target.write_text(f"from pathlib import Path\nPath({str(marker)!r}).write_text('executed')\n")
            with self.assertRaisesRegex(PROFILE.ProfileError, "analyzer-version binding"):
                PROFILE.checkpoint_module(repository)
            self.assertFalse(marker.exists())

    def test_nested_forged_profiles_are_rejected_even_when_resigned(self):
        def resigned(mutator):
            value = PROFILE.profile(self.subject(), ROOT)
            mutator(value)
            value["subject_digest"] = PROFILE.digest(value["subject"])
            value.pop("profile_digest")
            value["profile_digest"] = PROFILE.digest(value)
            return value

        cases = {
            "null_analyzer": lambda value: value.__setitem__("analyzer", None),
            "null_coverage": lambda value: value.__setitem__("coverage", None),
            "malformed_subject": lambda value: value["subject"].__setitem__("checkpoint", None),
            "fabricated_zero": lambda value: value["observations"]["file_count"].__setitem__("value", 0),
        }
        for name, mutator in cases.items():
            with self.subTest(name=name), self.assertRaises(PROFILE.ProfileError):
                PROFILE.validate_profile(resigned(mutator))

    def test_provenance_omission_contradiction_and_false_use_fail_when_resigned(self):
        def resigned(mutator):
            value = PROFILE.profile(self.subject(), ROOT)
            mutator(value)
            value["subject_digest"] = PROFILE.digest(value["subject"])
            value.pop("profile_digest")
            value["profile_digest"] = PROFILE.digest(value)
            return value

        cases = {
            "removed": lambda value: value.pop("provenance"),
            "producer_contradiction": lambda value: value["provenance"]["producer"].__setitem__("version", "forged"),
            "unsupported_source": lambda value: value["provenance"]["derivation_sources"].__setitem__("browser", {"use_state": "not_used", "reason": "deferred_by_profile_scope"}),
            "false_graph_use_without_identity": lambda value: value["provenance"]["derivation_sources"].__setitem__("structural_graph", {"use_state": "used"}),
            "false_graph_use_with_revision": lambda value: value["provenance"]["derivation_sources"].__setitem__("structural_graph", {"use_state": "used", "identity": {"revision": "invented"}}),
            "tree_contradiction": lambda value: value["provenance"]["derivation_sources"]["repository_trees"]["identity"].__setitem__("baseline_tree_oid", "0" * 40),
        }
        for name, mutator in cases.items():
            with self.subTest(name=name), self.assertRaises(PROFILE.ProfileError):
                PROFILE.validate_profile(resigned(mutator))

    def test_each_forged_runtime_identity_field_fails_when_resigned(self):
        def resigned(source, field, value):
            profile = PROFILE.profile(self.subject(), ROOT)
            profile["provenance"]["derivation_sources"][source]["identity"][field] = value
            profile.pop("profile_digest")
            profile["profile_digest"] = PROFILE.digest(profile)
            return profile

        cases = (
            ("git_runtime", "version", "git version forged"),
            ("git_runtime", "executable_sha256", "0" * 64),
            ("python_runtime", "implementation", "ForgedPython"),
            ("python_runtime", "version", "0.0.0-forged"),
            ("python_runtime", "executable_sha256", "f" * 64),
        )
        for source, field, forged in cases:
            with self.subTest(source=source, field=field), self.assertRaises(PROFILE.ProfileError):
                PROFILE.validate_profile(resigned(source, field, forged))


if __name__ == "__main__":
    unittest.main()
