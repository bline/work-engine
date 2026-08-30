from __future__ import annotations

import argparse
import hashlib
import importlib.util
import io
import json
import os
from pathlib import Path
import stat
import subprocess
import sys
import tempfile
import time
import unittest


SCRIPT_DIR = Path(__file__).parents[1] / "scripts"
SCRIPT = SCRIPT_DIR / "claude_paired_review.py"
EVIDENCE = SCRIPT_DIR / "paired_review_evidence.py"
sys.path.insert(0, str(SCRIPT_DIR))
SPEC = importlib.util.spec_from_file_location("claude_paired_review", SCRIPT)
assert SPEC and SPEC.loader
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)
MODEL = "anthropic/claude-sonnet-5-20260630"
SESSION_ID = "11111111-1111-4111-8111-111111111111"


FAKE_TRANSPORT = r'''#!/usr/bin/env python3
import hashlib
import json
import os
from pathlib import Path
import sys

receipt_path = Path(sys.argv[sys.argv.index("--receipt") + 1])
model = sys.argv[sys.argv.index("--openrouter-model") + 1]
attestation = Path(sys.argv[sys.argv.index("--routing-attestation") + 1])
separator = sys.argv.index("--")
command = sys.argv[separator + 1:]
expected_session_id = command[command.index("--session-id") + 1]
session_id = os.environ.get("FAKE_REALTIME_SESSION_ID", expected_session_id)
payload = {
    "verdict": "accepted", "findings": [],
    "verified_claims": [{"claim_id": "v1"}], "observations": [],
}
stdout = (json.dumps({"session_id": session_id, "result": json.dumps(payload)}) + "\n").encode()
returncode = int(os.environ.get("FAKE_REALTIME_RETURNCODE", "0"))
receipt = {
    "schema_version": 1,
    "result": "success" if returncode == 0 else "failed",
    "selected_transport": "openrouter" if returncode == 0 else None,
    "claude_version": "fake-claude 1.0",
    "request": {
        "openrouter_model": model,
        "continuity": "retained",
        "session_mode": "new",
        "session_id": expected_session_id,
        "experimental_betas_disabled": False,
        "terminal_title_disabled": os.environ.get("CLAUDE_CODE_DISABLE_TERMINAL_TITLE") == "1",
        "claude_config_dir_sha256_before": os.environ["EXPECTED_CONFIG_DIGEST"],
        "command_sha256": hashlib.sha256(json.dumps(
            command, sort_keys=True, separators=(",", ":"), ensure_ascii=False
        ).encode()).hexdigest(),
        "command_shape": [
            "claude", "--session-id", "--strict-mcp-config", "--mcp-config",
            "--tools", "--model", "-p", "--output-format", "--json-schema",
        ],
    },
    "routing_attestation": {"sha256": hashlib.sha256(attestation.read_bytes()).hexdigest()},
    "attempts": [{
        "requested_upstream_provider": "anthropic",
        "stdout_sha256": hashlib.sha256(stdout).hexdigest(),
    }],
}
receipt_path.parent.mkdir(parents=True, exist_ok=True)
receipt_path.write_text(json.dumps(receipt))
sys.stdout.buffer.write(stdout)
raise SystemExit(returncode)
'''


FAKE_BATCH = r'''#!/usr/bin/env python3
import hashlib
import json
import os
from pathlib import Path
import sys
import time

def argument(name):
    return sys.argv[sys.argv.index(name) + 1]

receipt_path = Path(argument("--receipt"))
event_path = Path(argument("--event-log"))
stdout_path = Path(argument("--stdout"))
stderr_path = Path(argument("--stderr"))
model = argument("--batch-model")
attestation = Path(argument("--routing-attestation"))
separator = sys.argv.index("--")
command = sys.argv[separator + 1:]
session_id = command[command.index("--session-id") + 1]
payload = {
    "verdict": "accepted", "findings": [],
    "verified_claims": [{"claim_id": "v1"}], "observations": [],
}
stdout = (json.dumps({"session_id": session_id, "result": json.dumps(payload)}) + "\n").encode()
time.sleep(0.1)
for path in (receipt_path, event_path, stdout_path, stderr_path):
    path.parent.mkdir(parents=True, exist_ok=True)
stdout_path.write_bytes(stdout)
stderr_path.write_bytes(b"")
events = [
    {"event": "proxy_started"},
    {"event": "submitted", "batch_id": "batch-1"},
    {"event": "completed", "batch_id": "batch-1", "usage": {
        "prompt_tokens": 10, "completion_tokens": 2, "total_tokens": 12, "cost": 0.01,
    }},
    {"event": "turn_completed", "successful": True},
]
event_path.write_text("".join(json.dumps(item) + "\n" for item in events))
receipt = {
    "artifact_type": "claude_batch_review_execution_v1",
    "schema_version": 1, "status": "success", "model": model,
    "requested_upstream_provider": "anthropic",
    "experimental_betas_disabled": True, "terminal_title_disabled": True,
    "claude_version": "fake-claude 1.0",
    "command_sha256": hashlib.sha256(json.dumps(
        command, sort_keys=True, separators=(",", ":"), ensure_ascii=False
    ).encode()).hexdigest(),
    "command_shape": [
        "claude", "--session-id", "--strict-mcp-config", "--mcp-config",
        "--tools", "--model", "-p", "--output-format", "--json-schema",
    ],
    "claude_config_dir_sha256_before": os.environ["EXPECTED_CONFIG_DIGEST"],
    "routing_attestation": {"sha256": hashlib.sha256(attestation.read_bytes()).hexdigest()},
    "output": {"stdout_sha256": hashlib.sha256(stdout).hexdigest()},
    "proxy": {"event_log_sha256": hashlib.sha256(event_path.read_bytes()).hexdigest()},
    "batch_summary": {
        "terminal_lineage_complete": True,
        "actual_openrouter_usage": {"cost": 0.01},
        "submitted_batch_ids": ["batch-1"],
    },
}
receipt_path.write_text(json.dumps(receipt))
'''


class Capture:
    def __init__(self):
        self.buffer = io.BytesIO()

    def write(self, value):
        self.buffer.write(value.encode("utf-8"))

    def flush(self):
        return None


class ClaudePairedReviewTest(unittest.TestCase):
    def setUp(self) -> None:
        self.temporary = tempfile.TemporaryDirectory()
        self.root = Path(self.temporary.name)
        self.campaign = self.root / "campaign"
        self.subject = self.root / "subject.json"
        self.packet = self.root / "packet.md"
        self.manifest = self.root / "config-manifest.json"
        self.attestation = self.root / "attestation.json"
        self.paired_mcp = self.root / "paired-mcp.json"
        self.workspace = self.root / "workspace"
        self.realtime_config = self.root / "realtime-config"
        self.batch_config = self.root / "batch-config"
        self.workspace.mkdir()
        self.realtime_config.mkdir()
        self.batch_config.mkdir()
        for config in (self.realtime_config, self.batch_config):
            (config / "settings.json").write_text("{}", encoding="utf-8")
        self.subject.write_text('{"commit":"abc"}', encoding="utf-8")
        self.packet.write_text("the exact paired prompt", encoding="utf-8")
        self.manifest.write_text('{"tools":["Read"]}', encoding="utf-8")
        self.attestation.write_text('{"route":"anthropic"}', encoding="utf-8")
        self.paired_mcp.write_text(json.dumps({
            "mcpServers": {
                "codebase-memory-mcp": {
                    "type": "stdio", "command": "codebase-memory-mcp",
                }
            }
        }), encoding="utf-8")
        subprocess.run([
            sys.executable, str(EVIDENCE), "init",
            "--campaign-root", str(self.campaign),
            "--campaign-id", "paired-test", "--target-pairs", "1",
            "--model", MODEL,
        ], check=True, capture_output=True)
        self.fake_transport = self.root / "fake_transport.py"
        self.fake_batch = self.root / "fake_batch.py"
        self.fake_transport.write_text(FAKE_TRANSPORT, encoding="utf-8")
        self.fake_batch.write_text(FAKE_BATCH, encoding="utf-8")
        for path in (self.fake_transport, self.fake_batch):
            path.chmod(path.stat().st_mode | stat.S_IXUSR)

    def tearDown(self) -> None:
        self.temporary.cleanup()

    def config_digest(self) -> str:
        process = subprocess.run([
            sys.executable, "-c",
            "from pathlib import Path; from claude_transport import sha256_directory; "
            "print(sha256_directory(Path(__import__('sys').argv[1])))",
            str(self.realtime_config),
        ], cwd=SCRIPT_DIR, capture_output=True, text=True, check=True)
        return process.stdout.strip()

    def arguments(self) -> argparse.Namespace:
        return argparse.Namespace(
            campaign_root=self.campaign, pair_id="pair-01", ordinal=1,
            subject_identity="checkpoint-abc", subject_artifact=self.subject,
            review_packet=self.packet, config_manifest=self.manifest,
            realtime_config_dir=self.realtime_config,
            batch_config_dir=self.batch_config,
            routing_attestation=self.attestation,
            paired_mcp_config=self.paired_mcp,
            reviewer_session_id=SESSION_ID,
            working_directory=self.workspace,
            attestation_max_age_seconds=900,
            batch_poll_interval_seconds=0.01,
            batch_poll_timeout_seconds=5,
            collection_window_seconds=0,
            finalizer_timeout_seconds=5,
            finalizer_poll_seconds=0.02,
            claude_command=[
                "claude", "-p", "--output-format", "json",
                "--json-schema", "{}",
            ],
        )

    def run_controller(
        self, realtime_returncode: int = 0, observed_session_id: str | None = None,
        arguments: argparse.Namespace | None = None,
    ) -> int:
        old_transport = MODULE.TRANSPORT_SCRIPT
        old_batch = MODULE.BATCH_SCRIPT
        old_stdout = MODULE.sys.stdout
        old_stderr = MODULE.sys.stderr
        old_environment = dict(os.environ)
        MODULE.TRANSPORT_SCRIPT = self.fake_transport
        MODULE.BATCH_SCRIPT = self.fake_batch
        MODULE.sys.stdout = Capture()
        MODULE.sys.stderr = Capture()
        self.stderr_capture = MODULE.sys.stderr
        os.environ["EXPECTED_CONFIG_DIGEST"] = self.config_digest()
        os.environ["FAKE_REALTIME_RETURNCODE"] = str(realtime_returncode)
        if observed_session_id is not None:
            os.environ["FAKE_REALTIME_SESSION_ID"] = observed_session_id
        try:
            return MODULE.run_review(arguments or self.arguments())
        finally:
            MODULE.TRANSPORT_SCRIPT = old_transport
            MODULE.BATCH_SCRIPT = old_batch
            MODULE.sys.stdout = old_stdout
            MODULE.sys.stderr = old_stderr
            os.environ.clear()
            os.environ.update(old_environment)

    def wait_for_pair_receipt(self) -> dict[str, object]:
        path = self.campaign / "pairs/pair-01/pair-receipt.json"
        finalizer_path = self.campaign / "pairs/pair-01/runtime/finalizer-receipt.json"
        deadline = time.monotonic() + 5
        while time.monotonic() < deadline:
            if path.exists() and finalizer_path.exists():
                finalizer = json.loads(finalizer_path.read_text())
                if finalizer.get("status") in {"finalized", "failed"}:
                    return json.loads(path.read_text())
            time.sleep(0.02)
        self.fail("background finalizer did not write the pair receipt")

    def test_one_call_returns_realtime_and_finalizes_background_pair(self) -> None:
        self.assertEqual(self.run_controller(), 0)
        pair = self.wait_for_pair_receipt()
        self.assertTrue(pair["comparison_ready"])
        self.assertEqual(pair["arms"]["batch"]["batch_ids"], ["batch-1"])
        registration = json.loads(
            (self.campaign / "pairs/pair-01/registration.json").read_text()
        )
        controller = json.loads(
            (self.campaign / "pairs/pair-01/runtime/controller-receipt.json").read_text()
        )
        self.assertEqual(
            controller["review_packet_sha256"], registration["review_packet"]["sha256"]
        )
        self.assertEqual(
            controller["subject_prompt_projection"]["source_sha256"],
            registration["subject"]["sha256"],
        )
        self.assertEqual(controller["realtime"]["status"], "success")
        self.assertTrue(controller["realtime"]["continuity_verified"])
        command = MODULE.prepare_initial_command(
            ["claude", "-p"], SESSION_ID, self.paired_mcp
        )
        self.assertIn("--dangerously-skip-permissions", command)
        tools = command[command.index("--tools") + 1].split(",")
        self.assertIn("mcp__codebase-memory-mcp__search_graph", tools)
        self.assertNotIn("mcp__codebase-memory-mcp", tools)
        self.assertFalse(any("delete_project" in tool for tool in tools))
        self.assertEqual(controller["batch"]["status"], "launched")
        self.assertEqual(
            registration["realtime_continuity"]["session_id"], SESSION_ID
        )
        self.assertEqual(pair["bindings"]["reviewer_session_id"], SESSION_ID)
        handoff = json.loads(
            (self.campaign / "pairs/pair-01/runtime/realtime-reviewer-handoff.json").read_text()
        )
        self.assertEqual(handoff["status"], "resume_ready")
        self.assertEqual(
            handoff["production_review_state"],
            "pending_external_same_session_bookkeeping",
        )
        self.assertIn(b"1/1 registered", self.stderr_capture.buffer.getvalue())
        progress = json.loads((self.campaign / "progress.json").read_text())
        self.assertEqual(progress["comparison_ready_pairs"], 1)
        self.assertTrue(progress["campaign_ready_for_adjudication"])

    def test_relative_paths_are_canonicalized_before_cross_process_launch(self) -> None:
        arguments = self.arguments()
        for field in (
            "campaign_root", "subject_artifact", "review_packet", "config_manifest",
            "realtime_config_dir", "batch_config_dir", "routing_attestation",
            "paired_mcp_config", "working_directory",
        ):
            setattr(arguments, field, Path(getattr(arguments, field).name))
        previous = Path.cwd()
        os.chdir(self.root)
        try:
            self.assertEqual(self.run_controller(arguments=arguments), 0)
        finally:
            os.chdir(previous)
        pair = self.wait_for_pair_receipt()
        self.assertTrue(pair["comparison_ready"])
        registration = json.loads(
            (self.campaign / "pairs/pair-01/registration.json").read_text()
        )
        self.assertTrue(Path(registration["claude_config"]["realtime_directory"]).is_absolute())
        self.assertTrue(Path(registration["paired_mcp_config"]["path"]).is_absolute())

    def test_retired_or_paused_campaign_is_rejected_before_registration(self) -> None:
        (self.root / "active-campaign.json").write_text(json.dumps({
            "active_campaign_root": str(self.campaign.resolve()),
            "status": "paused_batch_diagnosis",
        }), encoding="utf-8")
        with self.assertRaisesRegex(MODULE.TransportError, "campaign is not active"):
            MODULE.run_review(self.arguments())
        self.assertFalse((self.campaign / "pairs/pair-01").exists())

    def test_realtime_failure_is_returned_and_shadow_remains_in_denominator(self) -> None:
        self.assertEqual(self.run_controller(realtime_returncode=1), 1)
        pair = self.wait_for_pair_receipt()
        self.assertFalse(pair["comparison_ready"])
        controller = json.loads(
            (self.campaign / "pairs/pair-01/runtime/controller-receipt.json").read_text()
        )
        self.assertEqual(
            controller["status"], "realtime_failed_batch_cancellation_requested"
        )
        self.assertIn(
            controller["batch"]["cancellation"]["status"],
            {"requested", "already_terminal"},
        )
        self.assertTrue(any(
            "realtime arm did not complete successfully" in error
            for error in pair["validation_errors"]
        ))

    def test_session_mismatch_fails_closed_and_is_not_resume_ready(self) -> None:
        self.assertEqual(
            self.run_controller(
                observed_session_id="22222222-2222-4222-8222-222222222222"
            ),
            2,
        )
        pair = self.wait_for_pair_receipt()
        self.assertFalse(pair["comparison_ready"])
        self.assertFalse(pair["arms"]["realtime"]["evidence_valid"])
        handoff = json.loads(
            (self.campaign / "pairs/pair-01/runtime/realtime-reviewer-handoff.json").read_text()
        )
        self.assertEqual(handoff["status"], "not_resume_ready")

    def test_no_session_persistence_is_rejected_before_registration(self) -> None:
        arguments = self.arguments()
        arguments.claude_command.insert(2, "--no-session-persistence")
        with self.assertRaisesRegex(
            MODULE.TransportError, "initial retained paired review"
        ):
            MODULE.run_review(arguments)
        self.assertFalse((self.campaign / "pairs/pair-01").exists())

    def test_caller_model_override_is_rejected_before_registration(self) -> None:
        arguments = self.arguments()
        arguments.claude_command[2:2] = ["--model", "opus"]
        with self.assertRaisesRegex(
            MODULE.TransportError, "controller-owned"
        ):
            MODULE.run_review(arguments)
        self.assertFalse((self.campaign / "pairs/pair-01").exists())

    def test_mutable_review_state_mcp_is_rejected_before_registration(self) -> None:
        self.paired_mcp.write_text(json.dumps({
            "mcpServers": {
                "work-engine": {
                    "command": "node",
                    "args": ["server.mjs", "--review-authority", "authority.json"],
                }
            }
        }), encoding="utf-8")
        with self.assertRaisesRegex(
            MODULE.TransportError, "paired MCP config"
        ):
            MODULE.run_review(self.arguments())
        self.assertFalse((self.campaign / "pairs/pair-01").exists())

    def test_finalizer_launch_failure_returns_realtime_and_requires_recovery(self) -> None:
        original_launch = MODULE.launch_process
        launches = 0

        def fail_second_launch(command, *, env, log_path):
            nonlocal launches
            launches += 1
            if launches == 2:
                raise OSError("synthetic finalizer launch failure")
            return original_launch(command, env=env, log_path=log_path)

        MODULE.launch_process = fail_second_launch
        try:
            self.assertEqual(self.run_controller(), 0)
        finally:
            MODULE.launch_process = original_launch

        controller = json.loads(
            (self.campaign / "pairs/pair-01/runtime/controller-receipt.json").read_text()
        )
        self.assertEqual(controller["finalizer"]["status"], "launch_failed")
        self.assertTrue(controller["finalizer"]["manual_finalization_required"])
        self.assertEqual(controller["realtime"]["status"], "success")

        batch_receipt = self.campaign / "pairs/pair-01/runtime/batch-execution-receipt.json"
        deadline = time.monotonic() + 5
        while time.monotonic() < deadline and not batch_receipt.exists():
            time.sleep(0.02)
        self.assertTrue(batch_receipt.exists())


if __name__ == "__main__":
    unittest.main()
