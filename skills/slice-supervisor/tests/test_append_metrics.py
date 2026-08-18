from __future__ import annotations

import importlib.util
import unittest
from pathlib import Path


SCRIPT = Path(__file__).parents[1] / "scripts" / "append_metrics.py"
SPEC = importlib.util.spec_from_file_location("append_metrics", SCRIPT)
assert SPEC and SPEC.loader
APPEND_METRICS = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(APPEND_METRICS)


def base_record(version: int) -> dict[str, object]:
    record: dict[str, object] = {
        "schema_version": version,
        "run_id": "test-run",
        "slice_number": 1,
        "timestamp": "2026-08-18T12:00:00-06:00",
        "slice_title": "Contract test",
        "slice_goal": "Prove receipt compatibility",
        "status": "stopped",
        "outcome": "Fixture observed",
        "stop_reason": "test fixture",
        "plan_acceptance": "not_reached",
        "worker_metrics": {},
        "producer_metrics": {},
    }
    if version >= 2:
        requirements = ["focused_checks"]
        record.update(
            {
                "engine_config": {
                    "version": 1,
                    "source": "test",
                    "objective": "test receipts",
                    "work_source": None,
                    "builder": {"skill": "slice-builder"},
                    "validation": {"profile": "focused", "requirements": requirements},
                    "metrics": {},
                    "limits": {},
                    "approval": {},
                    "notifications": {},
                    "stop_on": [],
                    "explicit_fields": [],
                    "defaulted_fields": [],
                    "amendments": [],
                },
                "builder_skill": "slice-builder",
                "validation_profile": "focused",
                "validation_requirement_results": {"focused_checks": "blocked"},
                "more_in_scope_work_remains": None,
            }
        )
    if version == 3:
        record.update(
            {
                "placement_certificate": None,
                "placement_verdict": None,
                "placement_risk": None,
                "rejected_placement_alternatives": [],
                "vertical_semantic_test": None,
                "vertical_semantic_test_passed": None,
            }
        )
    return record


class AppendMetricsCompatibilityTest(unittest.TestCase):
    def test_accepts_historical_and_current_schema_versions(self) -> None:
        for version in (1, 2, 3):
            with self.subTest(version=version):
                self.assertEqual(version, APPEND_METRICS.validate(base_record(version))["schema_version"])

    def test_accepted_v3_requires_confirmed_placement_and_vertical_proof(self) -> None:
        record = base_record(3)
        record.update(
            {
                "status": "accepted",
                "stop_reason": None,
                "plan_acceptance": "procedural_auto_approval",
                "validation_requirement_results": {"focused_checks": "passed"},
                "placement_certificate": {"owner": "slice-builder"},
                "placement_verdict": "unresolved",
                "placement_risk": "low",
                "vertical_semantic_test": "receipt test",
                "vertical_semantic_test_passed": True,
            }
        )
        with self.assertRaisesRegex(ValueError, "confirmed placement_verdict"):
            APPEND_METRICS.validate(record)
        record["placement_verdict"] = "confirmed"
        self.assertEqual("accepted", APPEND_METRICS.validate(record)["status"])

    def test_compact_handoff_is_not_a_durable_audit_receipt(self) -> None:
        handoff = {
            "slice_title": "Work-engine optimization",
            "slice_goal": "Optimize the work engine",
            "outcome": "Contracts separated",
            "durable_decisions": [],
            "affected_boundaries": ["slice-builder"],
            "placement_certificate": {},
            "unresolved_concerns": [],
            "deferred_scope": [],
        }
        with self.assertRaisesRegex(ValueError, "missing required field: schema_version"):
            APPEND_METRICS.validate(handoff)

    def test_audit_identity_fields_match_durable_schema(self) -> None:
        audit = base_record(3)
        self.assertEqual("Contract test", audit["slice_title"])
        self.assertEqual("Prove receipt compatibility", audit["slice_goal"])
        self.assertNotIn("slice_statement", audit)
        self.assertEqual(3, APPEND_METRICS.validate(audit)["schema_version"])


if __name__ == "__main__":
    unittest.main()
