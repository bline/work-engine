from __future__ import annotations

import json
import os
from datetime import datetime, timezone
from pathlib import Path
import stat
import subprocess
import sys
import tempfile
import unittest


SCRIPT = Path(__file__).parents[1] / "scripts" / "claude_transport.py"


FAKE_CLAUDE = r'''#!/usr/bin/env python3
import json
import os
from pathlib import Path
import sys

if "--version" in sys.argv:
    print("fake-claude 1.0")
    raise SystemExit(0)

record = {
    "argv": sys.argv[1:],
    "base_url": os.environ.get("ANTHROPIC_BASE_URL"),
    "auth_token_present": bool(os.environ.get("ANTHROPIC_AUTH_TOKEN")),
    "anthropic_api_key": os.environ.get("ANTHROPIC_API_KEY"),
    "sonnet_model": os.environ.get("ANTHROPIC_DEFAULT_SONNET_MODEL"),
}
with Path(os.environ["FAKE_CLAUDE_LOG"]).open("a", encoding="utf-8") as handle:
    handle.write(json.dumps(record, sort_keys=True) + "\n")

openrouter = record["base_url"] == "https://openrouter.ai/api"
scenario = os.environ.get("FAKE_CLAUDE_SCENARIO", "success")
if not openrouter and scenario in {"quota", "quota-retained"}:
    print("HTTP 429 rate_limit_error: session limit reached; resets later", file=sys.stderr)
    raise SystemExit(1)
if not openrouter and scenario == "nonquota":
    print("MCP configuration is malformed", file=sys.stderr)
    raise SystemExit(1)
if openrouter and scenario == "fallback-fails":
    print("OpenRouter model error", file=sys.stderr)
    raise SystemExit(1)
print(json.dumps({"ok": True, "gateway": "openrouter" if openrouter else "anthropic"}))
'''


class ClaudeTransportTest(unittest.TestCase):
    def setUp(self) -> None:
        self.temporary = tempfile.TemporaryDirectory()
        self.root = Path(self.temporary.name)
        self.fake = self.root / "claude"
        self.fake.write_text(FAKE_CLAUDE, encoding="utf-8")
        self.fake.chmod(self.fake.stat().st_mode | stat.S_IXUSR)
        self.log = self.root / "calls.jsonl"
        self.receipt = self.root / "receipt.json"

    def write_routing_attestation(
        self,
        *,
        providers: list[str] | None = None,
        models: list[str] | None = None,
        key_hash: str = "key-hash-1",
    ) -> Path:
        path = self.root / "routing-attestation.json"
        path.write_text(
            json.dumps(
                {
                    "schema_version": 1,
                    "kind": "openrouter-key-guardrail",
                    "observed_at": datetime.now(timezone.utc).isoformat(),
                    "guardrail_id": "guardrail-1",
                    "key_hash": key_hash,
                    "allowed_providers": providers or ["anthropic"],
                    "allowed_models": models or ["anthropic/claude-sonnet-5-20260630"],
                    "assignment_guardrail_id": "guardrail-1",
                }
            ),
            encoding="utf-8",
        )
        return path

    def tearDown(self) -> None:
        self.temporary.cleanup()

    def invoke(
        self,
        scenario: str,
        *claude_args: str,
        wrapper_args: tuple[str, ...] = (),
        include_key: bool = True,
    ) -> subprocess.CompletedProcess[str]:
        env = dict(os.environ)
        env.update(
            {
                "FAKE_CLAUDE_SCENARIO": scenario,
                "FAKE_CLAUDE_LOG": str(self.log),
            }
        )
        if include_key:
            env["OPENROUTER_API_KEY"] = "test-secret-that-must-not-be-recorded"
        else:
            env.pop("OPENROUTER_API_KEY", None)
        return subprocess.run(
            [
                sys.executable,
                str(SCRIPT),
                "--receipt",
                str(self.receipt),
                *wrapper_args,
                "--",
                str(self.fake),
                *claude_args,
            ],
            env=env,
            capture_output=True,
            text=True,
            check=False,
        )

    def calls(self) -> list[dict[str, object]]:
        if not self.log.exists():
            return []
        return [json.loads(line) for line in self.log.read_text().splitlines()]

    def test_primary_success_does_not_contact_openrouter(self) -> None:
        result = self.invoke("success", "-p", "--no-session-persistence", "prompt")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(json.loads(result.stdout)["gateway"], "anthropic")
        self.assertEqual(len(self.calls()), 1)
        receipt = json.loads(self.receipt.read_text())
        self.assertEqual(receipt["selected_transport"], "anthropic")
        self.assertFalse(receipt["failover"]["attempted"])

    def test_quota_failure_replays_disposable_call_through_native_claude(self) -> None:
        result = self.invoke(
            "quota",
            "-p",
            "--no-session-persistence",
            "prompt",
            wrapper_args=("--allow-paid-failover",),
        )
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(json.loads(result.stdout)["gateway"], "openrouter")
        calls = self.calls()
        self.assertEqual(len(calls), 2)
        self.assertIsNone(calls[0]["base_url"])
        self.assertEqual(calls[1]["base_url"], "https://openrouter.ai/api")
        self.assertTrue(calls[1]["auth_token_present"])
        self.assertEqual(calls[1]["anthropic_api_key"], "")
        self.assertEqual(
            calls[1]["sonnet_model"], "anthropic/claude-sonnet-5-20260630"
        )
        receipt_text = self.receipt.read_text()
        self.assertNotIn("test-secret-that-must-not-be-recorded", receipt_text)
        receipt = json.loads(receipt_text)
        self.assertEqual(receipt["selected_transport"], "openrouter")
        self.assertEqual(
            receipt["failover"]["continuity_claim"],
            "exact_disposable_request_replayed",
        )

    def test_nonquota_failure_does_not_spend_on_fallback(self) -> None:
        result = self.invoke("nonquota", "-p", "--no-session-persistence", "prompt")
        self.assertEqual(result.returncode, 1)
        self.assertIn("malformed", result.stderr)
        self.assertEqual(len(self.calls()), 1)
        receipt = json.loads(self.receipt.read_text())
        self.assertEqual(receipt["result"], "failed")
        self.assertFalse(receipt["failover"]["attempted"])

    def test_key_presence_does_not_authorize_paid_auto_failover(self) -> None:
        result = self.invoke("quota", "-p", "--no-session-persistence", "prompt")
        self.assertEqual(result.returncode, 1)
        self.assertEqual(len(self.calls()), 1)
        receipt = json.loads(self.receipt.read_text())
        self.assertEqual(receipt["result"], "paid_failover_not_authorized")
        self.assertFalse(receipt["failover"]["attempted"])

    def test_disposable_auto_requires_no_session_persistence(self) -> None:
        result = self.invoke("success", "-p", "prompt")
        self.assertEqual(result.returncode, 2)
        self.assertIn("requires --no-session-persistence", result.stderr)
        self.assertEqual(self.calls(), [])

    def test_retained_initial_review_stops_for_replacement(self) -> None:
        result = self.invoke(
            "quota-retained",
            "-p",
            "--session-id",
            "11111111-1111-4111-8111-111111111111",
            "prompt",
            wrapper_args=("--continuity", "retained"),
        )
        self.assertEqual(result.returncode, 1)
        self.assertEqual(len(self.calls()), 1)
        receipt = json.loads(self.receipt.read_text())
        self.assertEqual(receipt["result"], "failover_blocked")
        self.assertEqual(
            receipt["failover"]["continuity_claim"], "replacement_required"
        )

    def test_retained_resume_requires_explicit_verified_opt_in(self) -> None:
        session_id = "22222222-2222-4222-8222-222222222222"
        blocked = self.invoke(
            "quota-retained",
            "-p",
            "--resume",
            session_id,
            "prompt",
            wrapper_args=("--continuity", "retained"),
        )
        self.assertEqual(blocked.returncode, 1)
        self.assertEqual(len(self.calls()), 1)

        self.log.unlink()
        allowed = self.invoke(
            "quota-retained",
            "-p",
            "--resume",
            session_id,
            "prompt",
            wrapper_args=(
                "--continuity",
                "retained",
                "--allow-retained-resume-failover",
                "--allow-paid-failover",
            ),
        )
        self.assertEqual(allowed.returncode, 0, allowed.stderr)
        calls = self.calls()
        self.assertEqual(len(calls), 2)
        self.assertEqual(calls[0]["argv"], calls[1]["argv"])
        receipt = json.loads(self.receipt.read_text())
        self.assertEqual(
            receipt["failover"]["continuity_claim"],
            "same_local_claude_session_resumed_across_gateway",
        )

    def test_forced_openrouter_requires_key_before_provider_entry(self) -> None:
        result = self.invoke(
            "success",
            "-p",
            "prompt",
            wrapper_args=("--transport", "openrouter"),
            include_key=False,
        )
        self.assertEqual(result.returncode, 2)
        self.assertIn("OPENROUTER_API_KEY is required", result.stderr)
        self.assertEqual(self.calls(), [])

    def test_batch_route_requires_separate_explicit_compatibility_gate(self) -> None:
        blocked = self.invoke(
            "success",
            "-p",
            "prompt",
            wrapper_args=(
                "--transport",
                "openrouter",
                "--openrouter-model",
                "anthropic/claude-sonnet-5-20260630:batch",
            ),
        )
        self.assertEqual(blocked.returncode, 2)
        self.assertIn("requires --allow-batch-route", blocked.stderr)
        self.assertEqual(self.calls(), [])

        allowed = self.invoke(
            "success",
            "-p",
            "prompt",
            wrapper_args=(
                "--transport",
                "openrouter",
                "--openrouter-model",
                "anthropic/claude-sonnet-5-20260630:batch",
                "--allow-batch-route",
            ),
        )
        self.assertEqual(allowed.returncode, 0, allowed.stderr)
        self.assertEqual(
            self.calls()[0]["sonnet_model"],
            "anthropic/claude-sonnet-5-20260630:batch",
        )

    def test_research_route_requires_exact_anthropic_key_guardrail(self) -> None:
        attestation = self.write_routing_attestation()
        result = self.invoke(
            "success",
            "-p",
            "prompt",
            wrapper_args=(
                "--transport",
                "openrouter",
                "--require-anthropic-1p",
                "--routing-attestation",
                str(attestation),
                "--openrouter-key-hash",
                "key-hash-1",
            ),
        )
        self.assertEqual(result.returncode, 0, result.stderr)
        receipt = json.loads(self.receipt.read_text())
        self.assertEqual(
            receipt["attempts"][0]["requested_upstream_provider"], "anthropic"
        )
        self.assertEqual(
            receipt["routing_attestation"]["allowed_providers"], ["anthropic"]
        )
        self.assertFalse(receipt["upstream_provider_observed"])

    def test_receipt_binds_initial_config_and_compatibility_flags(self) -> None:
        config = self.root / "claude-config"
        config.mkdir()
        (config / "settings.json").write_text('{"tools":[]}', encoding="utf-8")
        prior_config = os.environ.get("CLAUDE_CONFIG_DIR")
        prior_betas = os.environ.get("CLAUDE_CODE_DISABLE_EXPERIMENTAL_BETAS")
        prior_title = os.environ.get("CLAUDE_CODE_DISABLE_TERMINAL_TITLE")
        os.environ["CLAUDE_CONFIG_DIR"] = str(config)
        os.environ["CLAUDE_CODE_DISABLE_EXPERIMENTAL_BETAS"] = "1"
        os.environ["CLAUDE_CODE_DISABLE_TERMINAL_TITLE"] = "1"
        try:
            result = self.invoke(
                "success", "-p", "prompt",
                wrapper_args=("--transport", "openrouter"),
            )
        finally:
            for name, value in (
                ("CLAUDE_CONFIG_DIR", prior_config),
                ("CLAUDE_CODE_DISABLE_EXPERIMENTAL_BETAS", prior_betas),
                ("CLAUDE_CODE_DISABLE_TERMINAL_TITLE", prior_title),
            ):
                if value is None:
                    os.environ.pop(name, None)
                else:
                    os.environ[name] = value
        self.assertEqual(result.returncode, 0, result.stderr)
        request = json.loads(self.receipt.read_text())["request"]
        self.assertEqual(request["claude_config_dir"], str(config))
        self.assertRegex(request["claude_config_dir_sha256_before"], r"^[0-9a-f]{64}$")
        self.assertTrue(request["experimental_betas_disabled"])
        self.assertTrue(request["terminal_title_disabled"])

    def test_research_route_rejects_fallback_provider_or_wrong_key(self) -> None:
        attestation = self.write_routing_attestation(
            providers=["anthropic", "google"]
        )
        wrong_provider = self.invoke(
            "success",
            "-p",
            "prompt",
            wrapper_args=(
                "--transport",
                "openrouter",
                "--require-anthropic-1p",
                "--routing-attestation",
                str(attestation),
                "--openrouter-key-hash",
                "key-hash-1",
            ),
        )
        self.assertEqual(wrong_provider.returncode, 2)
        self.assertIn("allow only provider 'anthropic'", wrong_provider.stderr)
        self.assertEqual(self.calls(), [])

        attestation = self.write_routing_attestation()
        wrong_key = self.invoke(
            "success",
            "-p",
            "prompt",
            wrapper_args=(
                "--transport",
                "openrouter",
                "--require-anthropic-1p",
                "--routing-attestation",
                str(attestation),
                "--openrouter-key-hash",
                "different-key-hash",
            ),
        )
        self.assertEqual(wrong_key.returncode, 2)
        self.assertIn("selected API key hash", wrong_key.stderr)
        self.assertEqual(self.calls(), [])


if __name__ == "__main__":
    unittest.main()
