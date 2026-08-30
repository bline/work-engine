from __future__ import annotations

import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest


SCRIPT = Path(__file__).parents[1] / "scripts" / "paired_review_evidence.py"
MODEL = "anthropic/claude-sonnet-5-20260630"
SESSION_ID = "11111111-1111-4111-8111-111111111111"


def digest(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


class PairedReviewEvidenceTest(unittest.TestCase):
    def setUp(self) -> None:
        self.temporary = tempfile.TemporaryDirectory()
        self.root = Path(self.temporary.name)
        self.campaign = self.root / "campaign"
        self.subject = self.root / "subject.json"
        self.packet = self.root / "packet.md"
        self.config_manifest = self.root / "config-manifest.json"
        self.realtime_config = self.root / "realtime-config"
        self.batch_config = self.root / "batch-config"
        self.attestation = self.root / "attestation.json"
        self.paired_mcp = self.root / "paired-mcp.json"
        self.subject.write_text('{"checkpoint":"abc"}', encoding="utf-8")
        self.packet.write_text("review this exact checkpoint", encoding="utf-8")
        self.config_manifest.write_text('{"mcp":[],"tools":["Read"]}', encoding="utf-8")
        self.realtime_config.mkdir()
        self.batch_config.mkdir()
        for config in (self.realtime_config, self.batch_config):
            (config / "settings.json").write_text(
                '{"mcpServers":{}}', encoding="utf-8"
            )
        self.attestation.write_text('{"route":"anthropic"}', encoding="utf-8")
        self.paired_mcp.write_text(json.dumps({
            "mcpServers": {
                "codebase-memory-mcp": {
                    "type": "stdio", "command": "codebase-memory-mcp",
                }
            }
        }), encoding="utf-8")

    def tearDown(self) -> None:
        self.temporary.cleanup()

    def invoke(self, *arguments: str) -> subprocess.CompletedProcess[str]:
        return subprocess.run(
            [sys.executable, str(SCRIPT), *arguments],
            capture_output=True, text=True, check=False,
        )

    def register(self, *, target: int = 1) -> None:
        initialized = self.invoke(
            "init", "--campaign-root", str(self.campaign),
            "--campaign-id", "calibration-1", "--target-pairs", str(target),
            "--model", MODEL,
        )
        self.assertEqual(initialized.returncode, 0, initialized.stderr)
        registered = self.invoke(
            "register", "--campaign-root", str(self.campaign),
            "--pair-id", "pair-01", "--ordinal", "1",
            "--subject-identity", "checkpoint-abc",
            "--subject-artifact", str(self.subject),
            "--review-packet", str(self.packet),
            "--config-manifest", str(self.config_manifest),
            "--realtime-config-dir", str(self.realtime_config),
            "--batch-config-dir", str(self.batch_config),
            "--routing-attestation", str(self.attestation),
            "--reviewer-session-id", SESSION_ID,
            "--paired-mcp-config", str(self.paired_mcp),
        )
        self.assertEqual(registered.returncode, 0, registered.stderr)

    def write_attempts(
        self, *, batch_betas: bool = True, same_command: bool = True,
        controller_command_matches: bool = True,
    ) -> tuple[Path, Path, Path, Path, Path, Path]:
        registration = json.loads(
            (self.campaign / "pairs/pair-01/registration.json").read_text()
        )
        realtime_result = self.root / "realtime.json"
        batch_result = self.root / "batch.json"
        realtime_result.write_text(
            json.dumps({
                "session_id": SESSION_ID, "total_cost_usd": 0.4,
                "result": json.dumps({
                    "verdict": "accepted", "findings": [],
                    "verified_claims": [{"claim_id": "v1"}], "observations": [],
                }),
            }),
            encoding="utf-8",
        )
        batch_result.write_text(
            json.dumps({
                "session_id": SESSION_ID,
                "result": json.dumps({
                    "verdict": "accepted", "findings": [],
                    "verified_claims": [{"claim_id": "v1"}], "observations": [],
                }),
            }), encoding="utf-8"
        )
        command = "a" * 64
        controller_receipt = self.root / "controller-receipt.json"
        controller_receipt.write_text(json.dumps({
            "artifact_type": "claude_paired_review_controller_v2",
            "schema_version": 2,
            "campaign_id": registration["campaign_id"],
            "pair_id": registration["pair_id"],
            "command_sha256": command if controller_command_matches else "c" * 64,
            "review_state_isolation": {
                "mutable_production_review_state_available": False,
                "paired_mcp_config_sha256": registration["paired_mcp_config"]["sha256"],
            },
            "realtime": {"status": "success", "continuity_verified": True},
        }), encoding="utf-8")
        realtime_receipt = self.root / "realtime-receipt.json"
        realtime_receipt.write_text(json.dumps({
            "result": "success", "selected_transport": "openrouter",
            "claude_version": "2.1.237 (Claude Code)",
            "request": {
                "openrouter_model": MODEL,
                "continuity": "retained",
                "session_mode": "new",
                "session_id": SESSION_ID,
                "experimental_betas_disabled": False,
                "terminal_title_disabled": True,
                "claude_config_dir_sha256_before": registration["claude_config"]["directory_sha256"],
                "command_sha256": command,
                "command_shape": [
                    "claude", "--session-id", "--strict-mcp-config",
                    "--mcp-config", "--tools", "--model", "-p", "--output-format",
                    "--json-schema",
                ],
            },
            "routing_attestation": {"sha256": registration["routing_attestation"]["sha256"]},
            "attempts": [{
                "requested_upstream_provider": "anthropic",
                "stdout_sha256": digest(realtime_result),
            }],
        }), encoding="utf-8")
        event_log = self.root / "batch-events.jsonl"
        event_log.write_text(json.dumps({"event": "completed", "batch_id": "b1"}) + "\n", encoding="utf-8")
        batch_receipt = self.root / "batch-receipt.json"
        batch_receipt.write_text(json.dumps({
            "artifact_type": "claude_batch_review_execution_v1",
            "status": "success", "model": MODEL,
            "requested_upstream_provider": "anthropic",
            "experimental_betas_disabled": batch_betas,
            "terminal_title_disabled": True,
            "claude_version": "2.1.237 (Claude Code)",
            "command_sha256": command if same_command else "b" * 64,
            "command_shape": [
                "claude", "--session-id", "--strict-mcp-config",
                "--mcp-config", "--tools", "--model", "-p", "--output-format",
                "--json-schema",
            ],
            "claude_config_dir_sha256_before": registration["claude_config"]["directory_sha256"],
            "routing_attestation": {"sha256": registration["routing_attestation"]["sha256"]},
            "output": {"stdout_sha256": digest(batch_result)},
            "proxy": {"event_log_sha256": digest(event_log)},
            "batch_summary": {
                "terminal_lineage_complete": True,
                "actual_openrouter_usage": {"cost": 0.2},
                "submitted_batch_ids": ["b1"],
            },
        }), encoding="utf-8")
        return (
            realtime_result, realtime_receipt, batch_result, batch_receipt,
            event_log, controller_receipt,
        )

    def finalize(
        self, paths: tuple[Path, Path, Path, Path, Path, Path]
    ) -> subprocess.CompletedProcess[str]:
        (realtime_result, realtime_receipt, batch_result, batch_receipt,
         event_log, controller_receipt) = paths
        return self.invoke(
            "finalize", "--campaign-root", str(self.campaign),
            "--pair-id", "pair-01",
            "--realtime-result", str(realtime_result),
            "--realtime-receipt", str(realtime_receipt),
            "--batch-result", str(batch_result),
            "--batch-receipt", str(batch_receipt),
            "--batch-event-log", str(event_log),
            "--controller-receipt", str(controller_receipt),
        )

    def test_complete_pair_is_auditable_and_retains_both_results(self) -> None:
        self.register()
        finalized = self.finalize(self.write_attempts())
        self.assertEqual(finalized.returncode, 0, finalized.stderr)
        receipt = json.loads(
            (self.campaign / "pairs/pair-01/pair-receipt.json").read_text()
        )
        self.assertTrue(receipt["comparison_ready"])
        self.assertEqual(receipt["known_confound"], "transport plus experimental-beta setting")
        self.assertEqual(receipt["arms"]["batch"]["batch_ids"], ["b1"])
        self.assertTrue(receipt["arms"]["realtime"]["evidence_valid"])
        self.assertEqual(receipt["bindings"]["reviewer_session_id"], SESSION_ID)
        self.assertTrue(receipt["authority"]["bench_result_is_not_production_approval"])
        self.assertFalse(receipt["authority"]["production_state_bookkeeping_assessed"])
        audited = self.invoke("audit", "--campaign-root", str(self.campaign))
        self.assertEqual(audited.returncode, 0, audited.stderr)
        report = json.loads((self.campaign / "audit.json").read_text())
        self.assertTrue(report["campaign_ready_for_adjudication"])
        compared = self.invoke("compare", "--campaign-root", str(self.campaign))
        self.assertEqual(compared.returncode, 0, compared.stderr)
        comparison = json.loads((self.campaign / "comparison.json").read_text())
        self.assertEqual(comparison["exact_semantic_payload_matches"], 1)
        self.assertEqual(comparison["verdict_matches"], 1)

    def test_command_or_beta_mismatch_fails_closed_but_remains_visible(self) -> None:
        self.register()
        finalized = self.finalize(
            self.write_attempts(batch_betas=False, same_command=False)
        )
        self.assertEqual(finalized.returncode, 1)
        receipt = json.loads(
            (self.campaign / "pairs/pair-01/pair-receipt.json").read_text()
        )
        self.assertFalse(receipt["comparison_ready"])
        self.assertTrue(receipt["arms"]["realtime"]["evidence_valid"])
        self.assertFalse(receipt["arms"]["batch"]["evidence_valid"])
        self.assertTrue(any("same native Claude command" in item
                            for item in receipt["validation_errors"]))
        self.assertTrue(any("betas disabled" in item
                            for item in receipt["validation_errors"]))
        audited = self.invoke("audit", "--campaign-root", str(self.campaign))
        self.assertEqual(audited.returncode, 1)
        report = json.loads((self.campaign / "audit.json").read_text())
        self.assertEqual(report["registered_pairs"], 1)
        self.assertEqual(report["comparison_ready_pairs"], 0)

    def test_ten_pair_campaign_cannot_finish_with_only_one_pair(self) -> None:
        self.register(target=10)
        finalized = self.finalize(self.write_attempts())
        self.assertEqual(finalized.returncode, 0, finalized.stderr)
        audited = self.invoke("audit", "--campaign-root", str(self.campaign))
        self.assertEqual(audited.returncode, 1)
        report = json.loads((self.campaign / "audit.json").read_text())
        self.assertEqual(report["target_pairs"], 10)
        self.assertEqual(report["registered_pairs"], 1)
        self.assertEqual(report["comparison_ready_pairs"], 1)
        self.assertFalse(report["campaign_ready_for_adjudication"])

    def test_controller_command_binding_is_required(self) -> None:
        self.register()
        finalized = self.finalize(
            self.write_attempts(controller_command_matches=False)
        )
        self.assertEqual(finalized.returncode, 1)
        receipt = json.loads(
            (self.campaign / "pairs/pair-01/pair-receipt.json").read_text()
        )
        self.assertTrue(any(
            "controller-bound Claude command" in error
            for error in receipt["validation_errors"]
        ))

    def test_registration_rejects_mutable_review_state_mcp(self) -> None:
        self.paired_mcp.write_text(json.dumps({
            "mcpServers": {
                "work-engine": {
                    "command": "node",
                    "args": ["server.mjs", "--review-authority", "authority.json"],
                }
            }
        }), encoding="utf-8")
        initialized = self.invoke(
            "init", "--campaign-root", str(self.campaign),
            "--campaign-id", "calibration-1", "--target-pairs", "1",
            "--model", MODEL,
        )
        self.assertEqual(initialized.returncode, 0, initialized.stderr)
        registered = self.invoke(
            "register", "--campaign-root", str(self.campaign),
            "--pair-id", "pair-01", "--ordinal", "1",
            "--subject-identity", "checkpoint-abc",
            "--subject-artifact", str(self.subject),
            "--review-packet", str(self.packet),
            "--config-manifest", str(self.config_manifest),
            "--realtime-config-dir", str(self.realtime_config),
            "--batch-config-dir", str(self.batch_config),
            "--routing-attestation", str(self.attestation),
            "--reviewer-session-id", SESSION_ID,
            "--paired-mcp-config", str(self.paired_mcp),
        )
        self.assertEqual(registered.returncode, 2)
        self.assertIn("must contain only", registered.stderr)
        self.assertFalse((self.campaign / "pairs/pair-01").exists())


if __name__ == "__main__":
    unittest.main()
