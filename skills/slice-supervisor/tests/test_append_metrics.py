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
    if version >= 3:
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
    if version >= 4:
        record["worker_metrics"] = {
            "workflow_route": "direct",
            "route_revisions": [],
            "validation_breadth": {
                "selected_stages": ["semantic_proof"],
                "omitted_optional_stages": [],
                "rationale": "The fixture exercises only receipt validation.",
            },
            "provider_successful_calls": 0,
            "provider_failed_calls": 0,
            "provider_timed_out_calls": 0,
            "provider_infrastructure_failed_calls": 0,
            "evidence_mode_metrics": {},
            "provider_failure_reasons": {
                "network": 0,
                "timeout": 0,
                "permission": 0,
                "protocol": 0,
                "quota": 0,
                "other": 0,
            },
            "fallback_reason_counts": {
                "index_unavailable": 0,
                "coverage_gap": 0,
                "graph_ambiguity": 0,
                "provider_failure": 0,
            },
            "fallbacks": [],
        }
    return record


def make_config_v2(record: dict[str, object]) -> None:
    record["engine_config"]["version"] = 2
    record["engine_config"]["builder"]["context"] = {
        "repository_evidence": {
            "provider": "codex-codebase-memory",
            "skill": "repo-search",
        },
        "independent_review": {
            "provider": "claude",
            "skill": "claude-recon-implementation",
        },
    }
    record["worker_metrics"]["repository_evidence_identity"] = {
        "provider": "codex-codebase-memory",
        "skill": "repo-search",
    }
    record["worker_metrics"]["independent_review_identity"] = {
        "provider": "claude",
        "skill": "claude-recon-implementation",
    }
    empty_metrics = {
        "attempts": 0,
        "successful": 0,
        "failed": 0,
        "timed_out": 0,
        "infrastructure_failed": 0,
        "input_tokens": None,
        "cache_creation_tokens": None,
        "cache_read_tokens": None,
        "output_tokens": None,
        "thinking_tokens": None,
        "cost_usd": None,
        "wall_clock_seconds": None,
    }
    record["worker_metrics"]["provider_role_metrics"] = {
        "repository_evidence": dict(empty_metrics),
        "independent_review": dict(empty_metrics),
    }
    record["worker_metrics"].update(
        {
            "evidence_recon_calls": 0,
            "evidence_supplemental_calls": 0,
            "review_gate_calls": 0,
        }
    )


class AppendMetricsCompatibilityTest(unittest.TestCase):
    def test_accepts_historical_and_current_schema_versions(self) -> None:
        for version in (1, 2, 3, 4):
            with self.subTest(version=version):
                self.assertEqual(version, APPEND_METRICS.validate(base_record(version))["schema_version"])

    def test_v4_accepts_engine_config_v2_with_role_identities(self) -> None:
        record = base_record(4)
        make_config_v2(record)
        self.assertEqual(2, APPEND_METRICS.validate(record)["engine_config"]["version"])

    def test_engine_config_version_rejects_json_boolean(self) -> None:
        record = base_record(4)
        record["engine_config"]["version"] = True
        with self.assertRaisesRegex(ValueError, "version must be 1 or 2"):
            APPEND_METRICS.validate(record)

    def test_v1_and_v2_context_shapes_cannot_be_mixed(self) -> None:
        record = base_record(4)
        record["engine_config"]["builder"]["context"] = {"repository_evidence": {}}
        with self.assertRaisesRegex(ValueError, "unknown builder.context fields"):
            APPEND_METRICS.validate(record)

        record = base_record(4)
        make_config_v2(record)
        record["engine_config"]["builder"]["context"]["evidence_skill"] = "legacy"
        with self.assertRaisesRegex(ValueError, "unknown builder.context fields"):
            APPEND_METRICS.validate(record)

    def test_v2_rejects_inconsistent_configured_and_actual_role_adapters(self) -> None:
        record = base_record(4)
        make_config_v2(record)
        record["engine_config"]["builder"]["context"]["repository_evidence"]["skill"] = (
            "claude-recon-implementation"
        )
        with self.assertRaisesRegex(ValueError, "requires skill 'repo-search'"):
            APPEND_METRICS.validate(record)

        record = base_record(4)
        make_config_v2(record)
        record["worker_metrics"]["independent_review_identity"]["skill"] = "repo-search"
        with self.assertRaisesRegex(ValueError, "role identity is invalid"):
            APPEND_METRICS.validate(record)

        record = base_record(4)
        make_config_v2(record)
        record["worker_metrics"]["repository_evidence_identity"] = {
            "provider": "claude-filesystem",
            "skill": "claude-recon-implementation",
        }
        with self.assertRaisesRegex(ValueError, "must match configured role"):
            APPEND_METRICS.validate(record)

    def test_v2_requires_both_role_identities(self) -> None:
        record = base_record(4)
        make_config_v2(record)
        del record["worker_metrics"]["independent_review_identity"]
        with self.assertRaisesRegex(ValueError, "reported together"):
            APPEND_METRICS.validate(record)

    def test_v2_provider_role_totals_must_match_aggregate_counts(self) -> None:
        record = base_record(4)
        make_config_v2(record)
        role = record["worker_metrics"]["provider_role_metrics"]["repository_evidence"]
        role["attempts"] = 1
        role["successful"] = 1
        with self.assertRaisesRegex(ValueError, "provider-role successful totals"):
            APPEND_METRICS.validate(record)
        record["worker_metrics"]["provider_successful_calls"] = 1
        self.assertEqual(4, APPEND_METRICS.validate(record)["schema_version"])

    def test_v1_optional_identities_must_be_valid_and_equal(self) -> None:
        record = base_record(4)
        record["worker_metrics"]["repository_evidence_identity"] = {
            "provider": "claude-filesystem",
            "skill": "claude-recon-implementation",
        }
        record["worker_metrics"]["independent_review_identity"] = {
            "provider": "claude-codebase-memory",
            "skill": "claude-recon-implementation",
        }
        with self.assertRaisesRegex(ValueError, "one combined legacy role"):
            APPEND_METRICS.validate(record)
        record["worker_metrics"]["independent_review_identity"]["provider"] = "mystery"
        with self.assertRaisesRegex(ValueError, "role identity is invalid"):
            APPEND_METRICS.validate(record)

        record = base_record(4)
        actual = {
            "provider": "claude-filesystem",
            "skill": "claude-recon-implementation",
        }
        record["worker_metrics"]["repository_evidence_identity"] = dict(actual)
        record["worker_metrics"]["independent_review_identity"] = dict(actual)
        with self.assertRaisesRegex(ValueError, "must match configured legacy role"):
            APPEND_METRICS.validate(record)

    def test_v2_semantic_call_counters_are_bounded_by_role_attempts(self) -> None:
        record = base_record(4)
        make_config_v2(record)
        record["worker_metrics"]["review_gate_calls"] = 1
        with self.assertRaisesRegex(ValueError, "review gate calls"):
            APPEND_METRICS.validate(record)
        record["worker_metrics"]["review_gate_calls"] = 0
        record["worker_metrics"]["evidence_recon_calls"] = 1
        with self.assertRaisesRegex(ValueError, "repository evidence stage calls"):
            APPEND_METRICS.validate(record)

    def test_accepted_v4_requires_confirmed_placement_and_vertical_proof(self) -> None:
        record = base_record(4)
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
        audit = base_record(4)
        self.assertEqual("Contract test", audit["slice_title"])
        self.assertEqual("Prove receipt compatibility", audit["slice_goal"])
        self.assertNotIn("slice_statement", audit)
        self.assertEqual(4, APPEND_METRICS.validate(audit)["schema_version"])

    def test_v4_requires_evidence_provenance(self) -> None:
        record = base_record(4)
        del record["worker_metrics"]["fallbacks"]
        with self.assertRaisesRegex(ValueError, "worker_metrics.fallbacks"):
            APPEND_METRICS.validate(record)

    def test_v4_accepts_partitioned_modes_and_provider_failure_fallback(self) -> None:
        record = base_record(4)
        record["worker_metrics"] = {
            "workflow_route": "falsified-placement",
            "route_revisions": [],
            "validation_breadth": {
                "selected_stages": ["semantic_proof", "focused_checks"],
                "omitted_optional_stages": [
                    {
                        "stage": "full_suite",
                        "reason": "The fixture has no repository runtime surface.",
                    }
                ],
                "rationale": "The fixture proves partitioned provenance validation.",
            },
            "provider_successful_calls": 1,
            "provider_failed_calls": 0,
            "provider_timed_out_calls": 1,
            "provider_infrastructure_failed_calls": 0,
            "evidence_mode_metrics": {
                "indexed_structure": {
                    "attempts": 2,
                    "successful": 1,
                    "failed": 0,
                    "timed_out": 1,
                    "infrastructure_failed": 0,
                    "input_tokens": 10,
                    "cache_creation_tokens": 20,
                    "cache_read_tokens": 30,
                    "output_tokens": 40,
                    "thinking_tokens": 5,
                    "cost_usd": 0.25,
                    "wall_clock_seconds": 12.5,
                },
                "direct_source": {
                    "attempts": 1,
                    "successful": 1,
                    "failed": 0,
                    "timed_out": 0,
                    "infrastructure_failed": 0,
                    "input_tokens": None,
                    "cache_creation_tokens": None,
                    "cache_read_tokens": None,
                    "output_tokens": None,
                    "thinking_tokens": None,
                    "cost_usd": None,
                    "wall_clock_seconds": 1.0,
                },
            },
            "provider_failure_reasons": {
                "network": 0,
                "timeout": 1,
                "permission": 0,
                "protocol": 0,
                "quota": 0,
                "other": 0,
            },
            "fallback_reason_counts": {
                "index_unavailable": 0,
                "coverage_gap": 0,
                "graph_ambiguity": 0,
                "provider_failure": 1,
            },
            "fallbacks": [
                {
                    "from_mode": "indexed_structure",
                    "to_mode": "direct_source",
                    "stage": "targeted_reconnaissance",
                    "reason": "provider_failure",
                    "failure_kind": "timeout",
                }
            ],
        }
        self.assertEqual(4, APPEND_METRICS.validate(record)["schema_version"])

    def test_v4_distinguishes_healthy_coverage_fallback(self) -> None:
        record = base_record(4)
        record["worker_metrics"]["fallback_reason_counts"]["coverage_gap"] = 1
        record["worker_metrics"]["fallbacks"] = [
            {
                "from_mode": "indexed_structure",
                "to_mode": "direct_source",
                "stage": "placement",
                "reason": "coverage_gap",
                "failure_kind": None,
            }
        ]
        self.assertEqual(4, APPEND_METRICS.validate(record)["schema_version"])

    def test_v4_rejects_inconsistent_fallback_counts(self) -> None:
        record = base_record(4)
        record["worker_metrics"]["fallback_reason_counts"]["coverage_gap"] = 1
        with self.assertRaisesRegex(ValueError, "must match fallbacks"):
            APPEND_METRICS.validate(record)

    def test_v4_requires_route_and_validation_rationale(self) -> None:
        record = base_record(4)
        record["worker_metrics"]["validation_breadth"]["rationale"] = ""
        with self.assertRaisesRegex(ValueError, "rationale must be a nonempty string"):
            APPEND_METRICS.validate(record)

    def test_v4_rejects_selected_and_omitted_stage_overlap(self) -> None:
        record = base_record(4)
        record["worker_metrics"]["validation_breadth"]["omitted_optional_stages"] = [
            {"stage": "semantic_proof", "reason": "invalid fixture overlap"}
        ]
        with self.assertRaisesRegex(ValueError, "selected and omitted"):
            APPEND_METRICS.validate(record)

    def test_v4_requires_every_provider_failure_to_have_a_primary_cause(self) -> None:
        record = base_record(4)
        record["worker_metrics"]["provider_infrastructure_failed_calls"] = 1
        with self.assertRaisesRegex(ValueError, "must classify every"):
            APPEND_METRICS.validate(record)


if __name__ == "__main__":
    unittest.main()
