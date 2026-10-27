from __future__ import annotations

import json
import os
from datetime import datetime, timezone
import hashlib
from pathlib import Path
import signal
import stat
import subprocess
import sys
import tempfile
import time
from typing import Callable
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
    "stdin": sys.stdin.read(),
    "base_url": os.environ.get("ANTHROPIC_BASE_URL"),
    "auth_token_present": bool(os.environ.get("ANTHROPIC_AUTH_TOKEN")),
    "claude_code_oauth_token_present": bool(os.environ.get("CLAUDE_CODE_OAUTH_TOKEN")),
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
        oauth_token: str | None = None,
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
        env.pop("CLAUDE_CODE_OAUTH_TOKEN", None)
        if oauth_token is not None:
            env["CLAUDE_CODE_OAUTH_TOKEN"] = oauth_token
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

    def test_receipt_attests_oauth_token_presence_without_recording_secret(self) -> None:
        token = "fixture-setup-token-that-must-not-be-recorded"
        result = self.invoke(
            "success", "-p", "--no-session-persistence", "prompt", oauth_token=token
        )
        self.assertEqual(result.returncode, 0, result.stderr)
        receipt_text = self.receipt.read_text()
        receipt = json.loads(receipt_text)
        self.assertTrue(receipt["request"]["claude_code_oauth_token_present"])
        self.assertTrue(self.calls()[0]["claude_code_oauth_token_present"])
        self.assertNotIn(token, receipt_text)

    def test_integrity_bound_file_is_forwarded_on_stdin_without_entering_argv(self) -> None:
        prompt = "review payload " + ("x" * 150_000)
        prompt_file = self.root / "prompt.txt"
        prompt_file.write_text(prompt, encoding="utf-8")
        import hashlib
        prompt_sha256 = hashlib.sha256(prompt.encode("utf-8")).hexdigest()
        result = self.invoke(
            "success",
            "-p",
            "--no-session-persistence",
            wrapper_args=("--stdin-file", str(prompt_file), "--stdin-sha256", prompt_sha256),
        )
        self.assertEqual(result.returncode, 0, result.stderr)
        calls = self.calls()
        self.assertEqual(calls[0]["stdin"], prompt)
        self.assertNotIn(prompt, calls[0]["argv"])
        request = json.loads(self.receipt.read_text())["request"]
        self.assertEqual(request["stdin_sha256"], prompt_sha256)
        self.assertEqual(request["stdin_size_bytes"], len(prompt.encode("utf-8")))

    def test_stdin_digest_mismatch_stops_before_provider_entry(self) -> None:
        prompt_file = self.root / "prompt.txt"
        prompt_file.write_text("exact prompt", encoding="utf-8")
        result = self.invoke(
            "success",
            "-p",
            "--no-session-persistence",
            wrapper_args=("--stdin-file", str(prompt_file), "--stdin-sha256", "0" * 64),
        )
        self.assertEqual(result.returncode, 2)
        self.assertIn("stdin payload SHA-256 differs", result.stderr)
        self.assertEqual(self.calls(), [])

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


MODEL = "claude-sonnet-5-5"
SESSION = "33333333-3333-4333-8333-333333333333"

# A synthetic stream-json child. FAKE_GUARD_PLAN is a JSON list of steps:
#   {"emit": <event>} | {"raw": "<text>"} | {"sleep": s} | {"bulk": [stdout_bytes, stderr_bytes]}
#   {"grandchild": true} | {"effect": "<file>"} | {"ignore_term": true} | {"exit": n}
#   {"hold_pipe": s} (a descendant keeps stdout/stderr open for s seconds)
#   {"break_receipt": true} (replace the launcher's receipt file with a directory)
FAKE_STREAM_CLAUDE = r"""#!/usr/bin/env python3
import json, os, signal, subprocess, sys, time
from pathlib import Path

if "--version" in sys.argv:
    print("fake-claude 1.0")
    raise SystemExit(0)

argv = sys.argv[1:]
settings = None
for index, token in enumerate(argv):
    if token == "--settings":
        settings = Path(argv[index + 1])
    elif token.startswith("--settings="):
        settings = Path(token.split("=", 1)[1])
record = {"argv": argv, "pid": os.getpid(), "pgid": os.getpgrp()}
if settings is not None and settings.exists():
    record["settings_bytes"] = settings.read_text()
    record["settings_path"] = str(settings)
plan = json.loads(os.environ["FAKE_GUARD_PLAN"])
log = Path(os.environ["FAKE_CLAUDE_LOG"])

def write_log():
    with log.open("a") as handle:
        handle.write(json.dumps(record) + "\n")

write_log()
for step in plan:
    if "emit" in step:
        sys.stdout.write(json.dumps(step["emit"]) + "\n")
        sys.stdout.flush()
    elif "raw" in step:
        sys.stdout.write(step["raw"])
        sys.stdout.flush()
    elif "sleep" in step:
        time.sleep(step["sleep"])
    elif "effect" in step:
        Path(step["effect"]).write_text("done")
    elif step.get("ignore_term"):
        signal.signal(signal.SIGTERM, signal.SIG_IGN)
    elif step.get("grandchild"):
        child = subprocess.Popen(
            [sys.executable, "-c",
             "import os,signal,time\nsignal.signal(signal.SIGTERM, signal.SIG_IGN)\n"
             "os.setsid()\n"
             "open(os.environ['FAKE_GRANDCHILD_PIDFILE'],'w').write(str(os.getpid()))\n"
             "time.sleep(60)\nopen(os.environ['FAKE_GRANDCHILD_MARK'],'w').write('survived')\n"],
            stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, stdin=subprocess.DEVNULL)
        for _ in range(200):
            if Path(os.environ["FAKE_GRANDCHILD_PIDFILE"]).exists():
                break
            time.sleep(0.01)
    elif "hold_pipe" in step:
        holder = subprocess.Popen(
            [sys.executable, "-c",
             "import os,time\nopen(os.environ['FAKE_GRANDCHILD_PIDFILE'],'w').write(str(os.getpid()))\n"
             "time.sleep(" + str(step["hold_pipe"]) + ")"],
            stdin=subprocess.DEVNULL)  # inherits the stream pipes
        for _ in range(200):
            if Path(os.environ["FAKE_GRANDCHILD_PIDFILE"]).exists():
                break
            time.sleep(0.01)
    elif step.get("break_receipt"):
        receipt_path = Path(os.environ["FAKE_RECEIPT"])
        receipt_path.unlink()
        receipt_path.mkdir()
        (receipt_path / "keep").write_text("x")
    elif "oversize" in step:
        sys.stdout.write(json.dumps({"type": "user", "pad": "x" * step["oversize"]}) + "\n")
        sys.stdout.flush()
    elif "bulk" in step:
        out_bytes, err_bytes = step["bulk"]
        for _ in range(max(out_bytes, err_bytes) // 65536):
            sys.stdout.write("{\"type\":\"user\",\"pad\":\"" + "x" * 65000 + "\"}\n")
            sys.stderr.write("e" * 65536)
        sys.stdout.flush()
        sys.stderr.flush()
    elif "read_stdin" in step:
        record["stdin_bytes"] = len(sys.stdin.read())
        write_log()
    elif "exit" in step:
        raise SystemExit(step["exit"])
"""


def init_event(model: str = MODEL) -> dict[str, object]:
    return {"type": "system", "subtype": "init", "session_id": SESSION, "model": model}


def assistant(model: object = MODEL, *, stop_reason: object = "end_turn",
              parent: str | None = None, text: str = "ok") -> dict[str, object]:
    message: dict[str, object] = {
        "id": "msg_1", "type": "message", "role": "assistant", "stop_reason": stop_reason,
        "content": [{"type": "text", "text": text}],
    }
    if model is not None:
        message["model"] = model
    return {"type": "assistant", "session_id": SESSION, "parent_tool_use_id": parent,
            "message": message}


def result_event(usage: dict[str, object] | None = None, **extra: object) -> dict[str, object]:
    event: dict[str, object] = {"type": "result", "subtype": "success", "is_error": False,
                                "session_id": SESSION,
                                "modelUsage": usage if usage is not None else {MODEL: {}}}
    event.update(extra)
    return event


def emit(event: dict[str, object]) -> dict[str, object]:
    return {"emit": event}


GOOD = [emit(init_event()), emit(assistant()), emit(result_event())]
GUARD_FLAGS = ("-p", "--model", MODEL, "--output-format", "stream-json", "--verbose",
               "--session-id", SESSION)


class ModelGuardTest(unittest.TestCase):
    def setUp(self) -> None:
        self.temporary = tempfile.TemporaryDirectory()
        self.root = Path(self.temporary.name)
        self.fake = self.root / "claude"
        self.fake.write_text(FAKE_STREAM_CLAUDE, encoding="utf-8")
        self.fake.chmod(self.fake.stat().st_mode | stat.S_IXUSR)
        self.log = self.root / "calls.jsonl"
        self.receipt = self.root / "receipt.json"
        self.pidfile = self.root / "grandchild.pid"
        self.mark = self.root / "grandchild.mark"
        self.effect = self.root / "effect.txt"
        self.addCleanup(self.temporary.cleanup)
        self.addCleanup(self.kill_stragglers)

    def kill_stragglers(self) -> None:
        for pid in [*(c["pid"] for c in self.calls()), self.grandchild_pid()]:
            if pid:
                try:
                    os.kill(pid, signal.SIGKILL)
                except OSError:
                    pass

    def grandchild_pid(self) -> int | None:
        return int(self.pidfile.read_text()) if self.pidfile.exists() else None

    def environment(self, plan: list[dict[str, object]]) -> dict[str, str]:
        env = dict(os.environ)
        env.update({
            "FAKE_GUARD_PLAN": json.dumps(plan),
            "FAKE_CLAUDE_LOG": str(self.log),
            "FAKE_GRANDCHILD_PIDFILE": str(self.pidfile),
            "FAKE_GRANDCHILD_MARK": str(self.mark),
            "FAKE_RECEIPT": str(self.receipt),
        })
        env.pop("CLAUDE_CODE_OAUTH_TOKEN", None)
        return env

    def command(self, plan: list[dict[str, object]], *claude_args: str,
                wrapper_args: tuple[str, ...] = ("--exact-model-guard",),
                transport: str = "anthropic") -> list[str]:
        return [sys.executable, str(SCRIPT), "--transport", transport,
                "--continuity", "retained", "--receipt", str(self.receipt), *wrapper_args,
                "--", str(self.fake), *(claude_args or GUARD_FLAGS)]

    def run_guard(self, plan: list[dict[str, object]], *claude_args: str,
                  wrapper_args: tuple[str, ...] = ("--exact-model-guard",),
                  transport: str = "anthropic", timeout: float = 60
                  ) -> subprocess.CompletedProcess[str]:
        return subprocess.run(
            self.command(plan, *claude_args, wrapper_args=wrapper_args, transport=transport),
            env=self.environment(plan), capture_output=True, text=True, check=False,
            stdin=subprocess.DEVNULL, timeout=timeout)

    def calls(self) -> list[dict[str, object]]:
        if not self.log.exists():
            return []
        return [json.loads(line) for line in self.log.read_text().splitlines()]

    def guard(self) -> dict[str, object]:
        return json.loads(self.receipt.read_text())["model_guard"]

    def assert_dead(self, pid: int) -> None:
        deadline = time.monotonic() + 5
        while time.monotonic() < deadline:
            try:
                state = Path(f"/proc/{pid}/stat").read_text().rsplit(")", 1)[1].split()[0]
            except OSError:
                return
            if state == "Z":
                return
            time.sleep(0.02)
        self.fail(f"process {pid} is still alive")

    def wait_for_native_pid(self) -> int:
        deadline = time.monotonic() + 10
        while time.monotonic() < deadline:
            if self.calls():
                return self.calls()[0]["pid"]
            time.sleep(0.01)
        self.fail("the native child never started")

    def live_fake_processes(self) -> list[int]:
        """Pids of live (not zombie) processes whose command line names this test's fake claude."""
        found = []
        for entry in Path("/proc").iterdir():
            if not entry.name.isdigit():
                continue
            try:
                cmdline = (entry / "cmdline").read_bytes()
                state = (entry / "stat").read_text().rsplit(")", 1)[1].split()[0]
            except OSError:
                continue
            if str(self.fake).encode() in cmdline and state != "Z":
                found.append(int(entry.name))
        return found

    def cancel_when(self, plan: list[dict[str, object]], ready: Callable[[], None],
                    wrapper_args: tuple[str, ...]) -> subprocess.Popen[str]:
        """Start the launcher with unread output pipes, wait for `ready()`, send SIGTERM, collect."""
        proc = subprocess.Popen(
            self.command(plan, wrapper_args=wrapper_args), env=self.environment(plan),
            stdin=subprocess.DEVNULL, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
        try:
            ready()
            proc.send_signal(signal.SIGTERM)
            proc.communicate(timeout=30)
        finally:
            if proc.poll() is None:
                proc.kill()
                proc.communicate(timeout=30)
        return proc

    # exact identity and what counts as evidence

    def test_exact_model_succeeds_and_binds_effective_settings(self) -> None:
        source = self.root / "caller-settings.json"
        source.write_text(json.dumps({"permissions": {"allow": ["Read"]}, "model": "x"}))
        result = self.run_guard(
            GOOD, *GUARD_FLAGS, "--settings", str(source))
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn('"type": "result"', result.stdout)  # stream forwarded
        receipt = json.loads(self.receipt.read_text())
        guard = receipt["model_guard"]
        self.assertEqual((guard["outcome"], receipt["result"]), ("verified_exact", "success"))
        self.assertEqual(guard["observations"]["assistant_models"], {MODEL: 1})
        self.assertEqual(guard["identity"]["session_identity"], "matches")
        call = self.calls()[0]
        effective = json.loads(call["settings_bytes"])
        self.assertEqual(effective["permissions"], {"allow": ["Read"]})
        self.assertIs(effective["switchModelsOnFlag"], False)
        self.assertEqual(effective["availableModels"], [MODEL])
        self.assertEqual(
            guard["settings"]["effective_sha256"],
            hashlib.sha256(call["settings_bytes"].encode()).hexdigest())
        self.assertEqual(guard["settings"]["source_sha256"],
                         hashlib.sha256(source.read_bytes()).hexdigest())
        self.assertEqual(call["argv"].count("--settings"), 1)
        self.assertFalse(Path(call["settings_path"]).exists())  # scratch removed
        self.assertEqual(json.loads(source.read_text())["model"], "x")  # source untouched
        self.assertEqual(receipt["attempts"][0]["observed_models"], [MODEL])

    def test_settings_are_supplied_when_the_caller_has_none(self) -> None:
        self.assertEqual(self.run_guard(GOOD).returncode, 0)
        effective = json.loads(self.calls()[0]["settings_bytes"])
        self.assertEqual(effective, {"availableModels": [MODEL], "switchModelsOnFlag": False})
        self.assertEqual(self.guard()["settings"]["source_kind"], "none")

    def test_inline_settings_equals_form_is_merged(self) -> None:
        result = self.run_guard(GOOD, *GUARD_FLAGS, '--settings={"env":{"A":"1"}}')
        self.assertEqual(result.returncode, 0, result.stderr)
        effective = json.loads(self.calls()[0]["settings_bytes"])
        self.assertEqual(effective["env"], {"A": "1"})
        self.assertEqual(self.guard()["settings"]["source_kind"], "inline")

    def test_wrong_model_stops_child_and_preserves_pre_stop_effect(self) -> None:
        plan = [emit(init_event()), {"effect": str(self.effect)},
                emit(assistant("claude-sonnet-5-5-other", text="SECRET-TASK-TEXT")),
                {"sleep": 60}, {"effect": str(self.mark)}]
        started = time.monotonic()
        result = self.run_guard(plan)
        self.assertLess(time.monotonic() - started, 20)
        self.assertEqual(result.returncode, 41)
        self.assertTrue(self.effect.exists())  # effects before detection are not erased
        self.assertFalse(self.mark.exists())
        self.assert_dead(self.calls()[0]["pid"])
        receipt = json.loads(self.receipt.read_text())
        guard = receipt["model_guard"]
        self.assertEqual((guard["outcome"], receipt["result"]), ("violation", "guard_violation"))
        self.assertEqual(guard["violation"]["kind"], "model_mismatch")
        self.assertEqual(guard["violation"]["observed_model"], "claude-sonnet-5-5-other")
        self.assertEqual(guard["violation"]["line"], 2)
        self.assertEqual(guard["acceptance"], "invalidated")
        self.assertEqual(guard["tool_effects_before_stop"], "not_established")
        self.assertTrue(guard["native"]["stopped_by_launcher"])
        self.assertEqual(guard["termination"]["cause"], "guard_stop")
        self.assertNotIn("SECRET-TASK-TEXT", self.receipt.read_text())
        self.assertIsNone(receipt["selected_transport"])

    def test_alias_and_missing_model_are_never_accepted_as_exact(self) -> None:
        for model in ("sonnet", "default", "claude-sonnet-5-5[1m]"):
            with self.subTest(model=model):
                result = self.run_guard([emit(assistant(model)), emit(result_event())])
                self.assertEqual(result.returncode, 41, result.stderr)
                self.assertEqual(self.guard()["violation"]["observed_model"], model)
        for event in (assistant(None), assistant(7), assistant("")):
            with self.subTest(event=event["message"]):
                result = self.run_guard([emit(event), emit(result_event())])
                self.assertEqual(result.returncode, 42, result.stderr)
                self.assertEqual(self.guard()["outcome"], "incomplete_evidence")
                self.assertEqual(self.guard()["evidence_defect"]["kind"], "model_missing")

    def test_exit_zero_does_not_wash_out_a_violation(self) -> None:
        result = self.run_guard([emit(assistant("claude-opus-5-5")), emit(result_event()),
                                 {"exit": 0}])
        self.assertEqual(result.returncode, 41)
        guard = self.guard()
        self.assertEqual(guard["native"]["returncode"], 0)
        self.assertEqual(guard["outcome"], "violation")
        self.assertEqual(json.loads(self.receipt.read_text())["result"], "guard_violation")

    def test_subagent_assistant_with_another_model_is_a_violation(self) -> None:
        result = self.run_guard([emit(assistant()), emit(assistant("claude-haiku-5-5", parent="toolu_1")),
                                 emit(result_event())])
        self.assertEqual(result.returncode, 41)
        self.assertEqual(self.guard()["violation"]["parent_tool_use_id"], "toolu_1")

    def test_tool_progress_parent_marker_is_counted_neutrally(self) -> None:
        progress = {"type": "tool_progress", "tool_use_id": "toolu_1", "tool_name": "Bash",
                    "parent_tool_use_id": "toolu_0", "session_id": SESSION}
        result = self.run_guard([emit(init_event()), emit(progress), emit(assistant()), emit(progress),
                                 emit(result_event())])
        self.assertEqual(result.returncode, 0, result.stderr)
        observations = self.guard()["observations"]
        self.assertEqual(observations["parent_tool_use_records"], 2)
        self.assertNotIn("subagent_messages", observations)
        self.assertEqual(observations["assistant_models"], {MODEL: 1})  # progress is not model evidence
        self.assertEqual(observations["stream_event_models"], {})
        self.assertEqual(self.guard()["outcome"], "verified_exact")
        # With no assistant message the markers alone cannot make the run exact.
        self.assertEqual(self.run_guard([emit(init_event()), emit(progress), emit(result_event())]).returncode, 42)

    def test_init_and_model_usage_are_metadata_not_assistant_evidence(self) -> None:
        # History on resume lists other models; only assistant messages decide.
        plan = [emit(init_event("claude-opus-5-5")), emit(assistant()),
                emit(result_event({MODEL: {}, "claude-haiku-5-5": {}}))]
        result = self.run_guard(plan, "-p", "--model", MODEL, "--output-format", "stream-json",
                                "--verbose", "--resume", SESSION)
        self.assertEqual(result.returncode, 0, result.stderr)
        meta = self.guard()["observations"]["metadata_only"]
        self.assertEqual(meta["init_models"], ["claude-opus-5-5"])
        self.assertEqual(meta["usage_models_other_than_requested"], ["claude-haiku-5-5"])
        self.assertEqual(meta["model_usage_scope"], "cumulative_conversation_history")
        # The converse: exact only in metadata is no evidence at all.
        result = self.run_guard([emit(init_event()), emit(result_event())])
        self.assertEqual(result.returncode, 42)
        self.assertEqual(self.guard()["reason"], "no_exact_model_observation")

    def test_synthetic_notice_alone_is_not_an_exact_observation(self) -> None:
        result = self.run_guard([emit(assistant("<synthetic>")), emit(result_event())])
        self.assertEqual(result.returncode, 42)
        self.assertEqual(self.guard()["observations"]["synthetic_assistant_messages"], 1)
        result = self.run_guard([emit(assistant("<synthetic>")), emit(assistant()),
                                 emit(result_event())])
        self.assertEqual(result.returncode, 0, result.stderr)

    def test_partial_message_stream_is_checked_before_the_final_message(self) -> None:
        start = {"type": "stream_event", "event": {"type": "message_start",
                                                    "message": {"id": "m", "model": "claude-opus-5-5"}}}
        result = self.run_guard([emit(start), {"sleep": 60}, emit(result_event())], timeout=30)
        self.assertEqual(result.returncode, 41)
        self.assertEqual(self.guard()["observations"]["stream_event_models"],
                         {"claude-opus-5-5": 1})

    # refusal and fallback

    def test_refusal_and_fallback_signals_stop_without_retry(self) -> None:
        refusal_delta = {"type": "stream_event",
                         "event": {"type": "message_delta", "delta": {"stop_reason": "refusal"}}}
        fallback = {"type": "system", "subtype": "model_refusal_fallback", "session_id": SESSION}
        cases = {
            "refusal": ([emit(assistant(stop_reason="refusal")), {"sleep": 60}], "refusal"),
            "refusal_before_output": ([emit(refusal_delta), {"sleep": 60}], "refusal"),
            "fallback": ([emit(assistant()), emit(fallback), {"sleep": 60}], "refusal_fallback"),
            "result": ([emit(assistant()), emit(result_event(stop_reason="refusal")), {"sleep": 60}],
                       "refusal"),
        }
        for name, (plan, kind) in cases.items():
            with self.subTest(name):
                self.log.unlink(missing_ok=True)
                result = self.run_guard(plan, wrapper_args=("--exact-model-guard", "--allow-paid-failover"),
                                        timeout=30)
                self.assertEqual(result.returncode, 41, result.stderr)
                self.assertEqual(self.guard()["violation"]["kind"], kind)
                self.assertEqual(len(self.calls()), 1)  # a refusal is terminal, never replayed
                self.assert_dead(self.calls()[0]["pid"])

    def test_refusal_with_exact_model_and_text_keeps_content_out_of_receipt(self) -> None:
        plan = [emit(assistant(stop_reason="refusal", text="CANARY-PROMPT-TEXT"))]
        result = self.run_guard(plan)
        self.assertEqual(result.returncode, 41)
        self.assertIn("CANARY-PROMPT-TEXT", result.stdout)  # the stream itself is the artifact
        self.assertNotIn("CANARY-PROMPT-TEXT", self.receipt.read_text())
        self.assertEqual(self.guard()["violation"]["observed_model"], MODEL)

    # incomplete evidence

    def test_incomplete_streams_fail_closed_even_with_exit_zero(self) -> None:
        cases = {
            "no_result": ([emit(init_event()), emit(assistant())], "no_result_event"),
            "no_assistant": ([emit(result_event())], "no_exact_model_observation"),
            "empty": ([], "no_exact_model_observation"),
            "malformed": ([emit(assistant()), {"raw": "not json\n"}, emit(result_event())],
                          "malformed_stream"),
            "truncated": ([emit(assistant()), {"raw": '{"type":"result","sub'}], "malformed_stream"),
        }
        for name, (plan, reason) in cases.items():
            with self.subTest(name):
                result = self.run_guard(plan)
                self.assertEqual(result.returncode, 42, result.stderr)
                guard = self.guard()
                self.assertEqual((guard["outcome"], guard["reason"]), ("incomplete_evidence", reason))
                self.assertEqual(guard["native"]["returncode"], 0)
                self.assertEqual(json.loads(self.receipt.read_text())["result"], "guard_incomplete")

    def test_native_failure_keeps_its_own_exit_status(self) -> None:
        result = self.run_guard([emit(assistant()), emit(result_event(is_error=True)), {"exit": 3}])
        self.assertEqual(result.returncode, 3)
        receipt = json.loads(self.receipt.read_text())
        self.assertEqual((receipt["result"], receipt["model_guard"]["outcome"]),
                         ("failed", "verified_exact"))

    def test_oversize_line_is_incomplete_evidence(self) -> None:
        big = {"oversize": 17 * 1024 * 1024}
        result = self.run_guard([emit(assistant()), big, emit(result_event())])
        self.assertEqual(result.returncode, 42)
        guard = self.guard()
        self.assertEqual(guard["reason"], "oversize_line")
        self.assertEqual(guard["stream"]["oversize_lines"], 1)
        digest = hashlib.sha256(result.stdout.encode()).hexdigest()
        self.assertEqual(guard["stream"]["stdout_sha256"], digest)  # whole stream still hashed

    # preconditions: child must not start

    def test_unavailable_guard_writes_receipt_and_never_starts_the_child(self) -> None:
        source = self.root / "bad.json"
        cases = {
            "alias": ("-p", "--model", "sonnet", "--output-format", "stream-json", "--verbose"),
            "no_model": ("-p", "--output-format", "stream-json", "--verbose"),
            "two_models": GUARD_FLAGS + ("--model", "claude-opus-5-5"),
            "not_stream": ("-p", "--model", MODEL, "--output-format", "json"),
            "no_verbose": ("-p", "--model", MODEL, "--output-format", "stream-json"),
            "fallback_model": GUARD_FLAGS + ("--fallback-model", "claude-opus-5-5"),
        }
        for name, args in cases.items():
            with self.subTest(name):
                result = self.run_guard(GOOD, *args)
                self.assertEqual(result.returncode, 43, result.stderr)
                guard = self.guard()
                self.assertEqual(guard["outcome"], "unavailable")
                self.assertEqual(json.loads(self.receipt.read_text())["result"], "guard_unavailable")
                self.assertFalse(guard["native"]["started"])
                self.assertEqual(self.calls(), [])
        source.write_text(json.dumps({"switchModelsOnFlag": True}))
        conflicts = {
            "flag_true": json.dumps({"switchModelsOnFlag": True}),
            "other_models": json.dumps({"availableModels": [MODEL, "claude-opus-5-5"]}),
            "not_object": "[]",
            "bad_json": "{nope",
        }
        for name, body in conflicts.items():
            with self.subTest(name):
                source.write_text(body)
                result = self.run_guard(GOOD, *GUARD_FLAGS, "--settings", str(source))
                self.assertEqual(result.returncode, 43, result.stderr)
                self.assertEqual(self.calls(), [])
        result = self.run_guard(GOOD, *GUARD_FLAGS, "--settings", "/nonexistent.json")
        self.assertEqual(result.returncode, 43)
        result = self.run_guard(GOOD, *GUARD_FLAGS, "--settings", "{}", "--settings", "{}")
        self.assertEqual(result.returncode, 43)
        self.assertEqual(self.calls(), [])

    def test_singleton_available_models_in_caller_settings_is_compatible(self) -> None:
        source = self.root / "ok.json"
        source.write_text(json.dumps({"availableModels": [MODEL], "switchModelsOnFlag": False}))
        self.assertEqual(self.run_guard(GOOD, *GUARD_FLAGS, "--settings", str(source)).returncode, 0)

    def test_guard_requires_native_anthropic_transport(self) -> None:
        for transport in ("auto", "openrouter"):
            with self.subTest(transport):
                result = self.run_guard(GOOD, transport=transport)
                self.assertEqual(result.returncode, 43, result.stderr)
                self.assertEqual(self.calls(), [])

    def test_missing_executable_is_unavailable_not_a_crash(self) -> None:
        result = subprocess.run(
            [sys.executable, str(SCRIPT), "--transport", "anthropic", "--receipt", str(self.receipt),
             "--exact-model-guard", "--", str(self.root / "absent"), *GUARD_FLAGS],
            capture_output=True, text=True, check=False, stdin=subprocess.DEVNULL)
        self.assertEqual(result.returncode, 43)
        self.assertEqual(self.guard()["outcome"], "unavailable")

    def test_guard_tuning_options_require_the_guard(self) -> None:
        result = self.run_guard(GOOD, wrapper_args=("--guard-timeout-seconds", "5"))
        self.assertEqual(result.returncode, 2)
        self.assertEqual(self.calls(), [])

    def test_unguarded_receipt_is_unchanged(self) -> None:
        result = subprocess.run(
            [sys.executable, str(SCRIPT), "--transport", "anthropic", "--receipt", str(self.receipt),
             "--", str(self.fake), "-p"],
            env=self.environment(GOOD), capture_output=True, text=True, check=False,
            stdin=subprocess.DEVNULL)
        self.assertEqual(result.returncode, 0, result.stderr)
        receipt = json.loads(self.receipt.read_text())
        self.assertNotIn("model_guard", receipt)
        self.assertNotIn("exact_model_guard", receipt["request"])
        self.assertEqual(receipt["attempts"][0]["observed_models"], [MODEL, MODEL][:1])

    # liveness, cleanup and volume

    def test_stream_is_forwarded_while_the_child_is_still_running(self) -> None:
        plan = [emit(assistant()), {"sleep": 30}, emit(result_event())]
        proc = subprocess.Popen(self.command(plan), env=self.environment(plan),
                                stdin=subprocess.DEVNULL, stdout=subprocess.PIPE,
                                stderr=subprocess.PIPE, text=True)
        try:
            first = proc.stdout.readline()  # arrives long before the child's 30 s sleep ends
            self.assertEqual(json.loads(first)["type"], "assistant")
            self.assertIsNone(proc.poll())
            in_flight = json.loads(self.receipt.read_text())
            self.assertEqual(in_flight["result"], "pending")
            self.assertEqual(in_flight["model_guard"]["outcome"], "incomplete_evidence")
            self.assertEqual(in_flight["model_guard"]["reason"], "in_flight")
            self.assertTrue(in_flight["model_guard"]["native"]["pid"])
        finally:
            proc.terminate()
            proc.communicate(timeout=30)

    def test_violation_stops_sigterm_ignoring_child_and_session_escapee(self) -> None:
        plan = [{"ignore_term": True}, {"grandchild": True}, emit(assistant("claude-opus-5-5")),
                {"sleep": 60}]
        started = time.monotonic()
        result = self.run_guard(plan, wrapper_args=("--exact-model-guard", "--guard-stop-grace-seconds", "0.3"))
        self.assertEqual(result.returncode, 41, result.stderr)
        self.assertLess(time.monotonic() - started, 20)
        self.assert_dead(self.calls()[0]["pid"])
        self.assert_dead(self.grandchild_pid())
        termination = self.guard()["termination"]
        self.assertEqual(termination["process_collection"], "none_remaining")
        self.assertIn("subreaper", termination["tree_discovery"])
        stop = termination["stop"]
        self.assertEqual(stop["signals"], ["SIGTERM", "SIGKILL"])
        self.assertEqual(stop["survivor_pids"], [])
        self.assertGreaterEqual(stop["owned_processes_seen"], 2)
        self.assertFalse(self.mark.exists())

    def test_late_violation_after_exit_collects_leftover_descendants(self) -> None:
        plan = [{"grandchild": True}, emit(assistant("claude-opus-5-5")), {"exit": 0}]
        result = self.run_guard(plan)
        self.assertEqual(result.returncode, 41, result.stderr)
        self.assert_dead(self.grandchild_pid())

    def test_clean_exit_reports_but_does_not_kill_leftover_processes(self) -> None:
        plan = [{"grandchild": True}, *GOOD, {"exit": 0}]
        result = self.run_guard(plan)
        self.assertEqual(result.returncode, 0, result.stderr)
        pid = self.grandchild_pid()
        termination = self.guard()["termination"]
        self.assertIn(pid, termination["owned_processes_remaining_after_exit"])
        self.assertEqual(termination["process_collection"], "leftover_processes")
        os.kill(pid, signal.SIGKILL)

    def test_timeout_stops_tree_and_is_incomplete(self) -> None:
        plan = [{"grandchild": True}, emit(assistant()), {"sleep": 60}]
        started = time.monotonic()
        result = self.run_guard(plan, wrapper_args=("--exact-model-guard", "--guard-timeout-seconds", "1.5",
                                                   "--guard-stop-grace-seconds", "0.3"))
        self.assertEqual(result.returncode, 124, result.stderr)
        self.assertLess(time.monotonic() - started, 20)
        self.assert_dead(self.calls()[0]["pid"])
        self.assert_dead(self.grandchild_pid())
        guard = self.guard()
        self.assertEqual((guard["outcome"], guard["reason"]), ("incomplete_evidence", "timeout"))
        self.assertEqual(guard["termination"]["cause"], "timeout")

    def test_cancellation_stops_tree_and_writes_final_receipt(self) -> None:
        plan = [{"grandchild": True}, emit(assistant()), {"sleep": 60}]
        proc = subprocess.Popen(
            self.command(plan, wrapper_args=("--exact-model-guard", "--guard-stop-grace-seconds", "0.3")),
            env=self.environment(plan), stdin=subprocess.DEVNULL,
            stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
        try:
            proc.stdout.readline()
            self.assertTrue(self.pidfile.exists())
            proc.send_signal(signal.SIGTERM)
            proc.communicate(timeout=30)
        finally:
            if proc.poll() is None:
                proc.kill()
        self.assertEqual(proc.returncode, 128 + signal.SIGTERM)
        self.assert_dead(self.calls()[0]["pid"])
        self.assert_dead(self.grandchild_pid())
        guard = self.guard()
        self.assertEqual(guard["termination"]["cause"], "cancelled")
        self.assertEqual(json.loads(self.receipt.read_text())["result"], "guard_incomplete")

    def test_high_volume_on_both_pipes_with_large_stdin_does_not_deadlock(self) -> None:
        payload = "p" * (3 * 1024 * 1024)
        prompt = self.root / "prompt.txt"
        prompt.write_text(payload, encoding="utf-8")
        digest = hashlib.sha256(payload.encode()).hexdigest()
        plan = [{"bulk": [24 * 1024 * 1024, 24 * 1024 * 1024]}, {"read_stdin": True}, *GOOD]
        result = self.run_guard(plan, wrapper_args=("--exact-model-guard", "--stdin-file", str(prompt),
                                                   "--stdin-sha256", digest), timeout=120)
        self.assertEqual(result.returncode, 0, result.stderr[-300:])
        guard = self.guard()
        self.assertEqual(self.calls()[-1]["stdin_bytes"], len(payload))
        self.assertEqual(guard["stream"]["stdout_sha256"], hashlib.sha256(result.stdout.encode()).hexdigest())
        attempt = json.loads(self.receipt.read_text())["attempts"][0]
        self.assertEqual(attempt["stderr_sha256"], hashlib.sha256(result.stderr.encode()).hexdigest())
        self.assertGreater(attempt["stderr_size_bytes"], 20 * 1024 * 1024)
        self.assertTrue(guard["stream"]["drained"])

    # terminal evidence, envelopes and delivery

    def test_result_event_must_be_well_formed_and_successful(self) -> None:
        bare = {"type": "result", "session_id": SESSION}
        for name, event in {"no_subtype_or_flag": bare,
                            "flag_not_bool": result_event(is_error="no")}.items():
            with self.subTest(name):
                result = self.run_guard([emit(assistant()), emit(event)])
                self.assertEqual(result.returncode, 42, result.stderr)
                self.assertEqual(self.guard()["reason"], "result_malformed")
        for name, event in {"is_error": result_event(is_error=True),
                            "error_subtype": result_event(subtype="error_max_turns")}.items():
            with self.subTest(name):
                result = self.run_guard([emit(assistant()), emit(event), {"exit": 0}])
                self.assertEqual(result.returncode, 44, result.stderr)
                receipt = json.loads(self.receipt.read_text())
                guard = receipt["model_guard"]
                # The model was exact; the native run still did not succeed.
                self.assertEqual((guard["outcome"], guard["native"]["result_status"]),
                                 ("verified_exact", "error"))
                self.assertEqual(guard["native"]["returncode"], 0)
                self.assertEqual((receipt["result"], guard["acceptance"], receipt["selected_transport"]),
                                 ("failed", "invalidated", None))

    def test_activity_after_the_terminal_result_cannot_prove_completion(self) -> None:
        for name, tail in {"assistant": [emit(assistant())],
                           "second_result": [emit(result_event())],
                           "partial_message": [emit({"type": "stream_event", "event": {
                               "type": "message_start", "message": {"model": MODEL}}})]}.items():
            with self.subTest(name):
                result = self.run_guard([emit(assistant()), emit(result_event()), *tail])
                self.assertEqual(result.returncode, 42, result.stderr)
                self.assertEqual(self.guard()["reason"], "post_terminal_event")
        # A violation after the result still outranks the shape defect.
        result = self.run_guard([emit(assistant()), emit(result_event()),
                                 emit(assistant("claude-opus-5-5"))])
        self.assertEqual(result.returncode, 41)
        self.assertEqual(self.guard()["evidence_defect"]["kind"], "post_terminal_event")
        # Non-model housekeeping after the result is tolerated.
        tail = [emit({"type": "system", "subtype": "hook_response"}), emit({"type": "user"})]
        self.assertEqual(self.run_guard([*GOOD, *tail]).returncode, 0)

    def test_malformed_envelopes_are_incomplete_and_unknown_types_are_tolerated(self) -> None:
        cases = {
            "no_type": {"subtype": "init"},
            "assistant_without_message": {"type": "assistant", "message": "text"},
            "stream_event_not_object": {"type": "stream_event", "event": "x"},
            "stream_event_without_type": {"type": "stream_event", "event": {}},
            "message_start_without_message": {"type": "stream_event",
                                              "event": {"type": "message_start"}},
        }
        for name, event in cases.items():
            with self.subTest(name):
                result = self.run_guard([emit(event), *GOOD])
                self.assertEqual(result.returncode, 42, result.stderr)
                self.assertEqual(self.guard()["reason"], "envelope_malformed")
        unknown = [emit({"type": "rate_limit_event", "info": {}}), *GOOD]
        self.assertEqual(self.run_guard(unknown).returncode, 0)

    def test_native_api_error_notice_is_neither_a_model_switch_nor_proof(self) -> None:
        notice = {"type": "assistant", "error": "server_error", "session_id": SESSION,
                  "message": {"id": "msg_x", "model": "<synthetic>", "stop_reason": "stop_sequence",
                              "content": [{"type": "text", "text": "API Error: EAI_AGAIN"}]}}
        plan = [emit(init_event()), emit(notice), emit(result_event(is_error=True)), {"exit": 1}]
        result = self.run_guard(plan)
        self.assertEqual(result.returncode, 1, result.stderr)
        receipt = json.loads(self.receipt.read_text())
        guard = receipt["model_guard"]
        self.assertEqual(receipt["result"], "failed")
        self.assertIsNone(guard["violation"])
        self.assertEqual((guard["outcome"], guard["reason"]),
                         ("incomplete_evidence", "no_exact_model_observation"))
        self.assertEqual(guard["observations"]["synthetic_error_categories"], {"server_error": 1})
        self.assertEqual(guard["observations"]["assistant_models"], {})
        self.assertNotIn("EAI_AGAIN", self.receipt.read_text())
        # Even a clean exit cannot turn the notice into exact-model success.
        plan[-1] = {"exit": 0}
        self.assertEqual(self.run_guard(plan).returncode, 42)

    def test_stalled_output_consumer_does_not_delay_stop_or_receipt(self) -> None:
        plan = [{"bulk": [4 * 1024 * 1024, 4 * 1024 * 1024]}, emit(assistant("claude-opus-5-5")),
                {"sleep": 60}]
        wrapper = ("--exact-model-guard", "--guard-forward-backlog-bytes", "1048576")
        proc = subprocess.Popen(self.command(plan, wrapper_args=wrapper), env=self.environment(plan),
                                stdin=subprocess.DEVNULL, stdout=subprocess.PIPE,
                                stderr=subprocess.PIPE)  # neither pipe is read while running
        try:
            started = time.monotonic()
            self.assertEqual(proc.wait(timeout=40), 41)
            self.assertLess(time.monotonic() - started, 30)
        finally:
            if proc.poll() is None:
                proc.kill()
            proc.communicate(timeout=30)
        self.assert_dead(self.calls()[0]["pid"])
        guard = self.guard()
        self.assertEqual(guard["violation"]["kind"], "model_mismatch")
        self.assertFalse(guard["delivery"]["stdout"]["complete"])
        self.assertGreater(guard["delivery"]["stdout"]["undelivered_bytes"], 0)
        self.assertGreater(guard["stream"]["bytes"], 4_000_000)  # observed in full, not capped by backlog

    def test_consumer_that_closed_its_pipe_makes_delivery_incomplete(self) -> None:
        plan = [{"sleep": 0.5}, *GOOD]
        proc = subprocess.Popen(self.command(plan), env=self.environment(plan),
                                stdin=subprocess.DEVNULL, stdout=subprocess.PIPE,
                                stderr=subprocess.PIPE, text=True)
        proc.stdout.close()
        proc.communicate(timeout=30)
        self.assertEqual(proc.returncode, 42)
        guard = self.guard()
        self.assertEqual((guard["outcome"], guard["reason"]),
                         ("incomplete_evidence", "output_not_delivered"))
        self.assertFalse(guard["delivery"]["stdout"]["complete"])
        self.assertEqual(guard["observations"]["assistant_models"], {MODEL: 1})

    def test_session_identity_differences_are_reported_not_hidden(self) -> None:
        other = assistant()
        other["session_id"] = "44444444-4444-4444-8444-444444444444"
        result = self.run_guard([emit(init_event()), emit(other), emit(result_event())])
        self.assertEqual(result.returncode, 0, result.stderr)
        identity = self.guard()["identity"]
        self.assertEqual(identity["session_identity"], "differs")
        self.assertEqual(identity["requested_session_id"], SESSION)
        self.assertEqual(len(identity["observed_session_ids"]), 2)
        self.assertEqual(identity["result_session_id"], SESSION)

    def test_child_rewriting_the_effective_settings_invalidates_the_run(self) -> None:
        plan = [{"rewrite_settings": True}, *GOOD]
        script = self.fake.read_text().replace(
            '    elif "exit" in step:',
            '    elif step.get("rewrite_settings"):\n        settings.write_text("{}")\n    elif "exit" in step:')
        self.fake.write_text(script, encoding="utf-8")
        result = self.run_guard(plan)
        self.assertEqual(result.returncode, 42)
        self.assertEqual(self.guard()["reason"], "settings_modified")

    # owned-process lifetime: spawn through finalization

    def test_unwritable_initial_receipt_still_stops_the_native_child(self) -> None:
        self.receipt.mkdir()  # the receipt target cannot be written
        result = self.run_guard([{"grandchild": True}, {"sleep": 60}])
        self.assertEqual(result.returncode, 42, result.stderr)
        self.assertIn("receipt", result.stderr)
        self.assertEqual(self.live_fake_processes(), [])
        if self.grandchild_pid():
            self.assert_dead(self.grandchild_pid())
        self.assertEqual(list(self.root.glob("tmp*")), [])  # no stray receipt temp file

    def test_unwritable_final_receipt_is_never_a_silent_success(self) -> None:
        result = self.run_guard([{"break_receipt": True}, *GOOD])
        self.assertEqual(result.returncode, 42, result.stderr)
        self.assertIn("receipt could not be written", result.stderr)
        self.assertTrue(self.receipt.is_dir())  # nothing claimed success on disk
        self.assertEqual(list(self.root.glob("tmp*")), [])

    def test_cancellation_while_output_is_still_draining_is_not_success(self) -> None:
        plan = [{"hold_pipe": 60}, *GOOD, {"exit": 0}]

        def native_exited_but_pipes_held() -> None:
            self.assert_dead(self.wait_for_native_pid())
            self.assertTrue(self.grandchild_pid())

        proc = self.cancel_when(plan, native_exited_but_pipes_held,
                                ("--exact-model-guard", "--guard-stop-grace-seconds", "0.3"))
        self.assertEqual(proc.returncode, 128 + signal.SIGTERM)
        guard = self.guard()
        self.assertEqual(guard["termination"]["cause"], "cancelled")
        self.assertEqual(guard["termination"]["signals_received"], ["SIGTERM"])
        self.assertEqual((guard["outcome"], guard["reason"]), ("incomplete_evidence", "cancelled"))
        self.assertEqual(guard["acceptance"], "invalidated")
        self.assertEqual(json.loads(self.receipt.read_text())["result"], "guard_incomplete")
        self.assert_dead(self.grandchild_pid())  # the descendant holding the pipes was stopped

    def test_timeout_while_output_is_still_draining_stops_holders(self) -> None:
        plan = [{"hold_pipe": 60}, *GOOD, {"exit": 0}]
        result = self.run_guard(plan, wrapper_args=("--exact-model-guard", "--guard-timeout-seconds", "1",
                                                   "--guard-stop-grace-seconds", "0.3"))
        self.assertEqual(result.returncode, 124, result.stderr)
        guard = self.guard()
        self.assertEqual((guard["outcome"], guard["reason"]), ("incomplete_evidence", "timeout"))
        self.assertEqual(guard["acceptance"], "invalidated")
        self.assert_dead(self.grandchild_pid())

    def test_cancellation_during_final_delivery_flush_is_not_erased(self) -> None:
        plan = [{"bulk": [262144, 262144]}, *GOOD]  # neither output pipe is read while running

        def native_exited() -> None:
            self.assert_dead(self.wait_for_native_pid())
            time.sleep(0.5)  # inside the launcher's bounded wait to deliver the backlog

        proc = self.cancel_when(plan, native_exited, ("--exact-model-guard",))
        self.assertEqual(proc.returncode, 128 + signal.SIGTERM)
        guard = self.guard()
        self.assertEqual(guard["termination"]["cause"], "cancelled")
        self.assertEqual(guard["acceptance"], "invalidated")
        self.assertNotEqual(guard["outcome"], "verified_exact")

    def test_native_exit_between_check_and_wait_cannot_hang_the_launcher(self) -> None:
        # A valid interleaving: the launcher thread is suspended just before it clears its
        # wake-up until another thread has posted one (here: the child exiting and its
        # streams ending). A wake-up cleared after the exit was seen must not be waited on.
        harness = r"""
import importlib.util, sys, threading
spec = importlib.util.spec_from_file_location("transport", sys.argv[1])
transport = importlib.util.module_from_spec(spec)
spec.loader.exec_module(transport)
original_clear = threading.Event.clear
def clear_after_a_post(self):
    if threading.current_thread() is threading.main_thread() and not self.is_set():
        self.wait(10)
    original_clear(self)
threading.Event.clear = clear_after_a_post
raise SystemExit(transport.main(sys.argv[2:]))
"""
        plan = [{"sleep": 0.2}, *GOOD]
        command = [sys.executable, "-c", harness, str(SCRIPT), *self.command(plan)[2:]]
        result = subprocess.run(command, env=self.environment(plan), capture_output=True,
                                text=True, stdin=subprocess.DEVNULL, timeout=30, check=False)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(self.guard()["outcome"], "verified_exact")

    def test_unwritable_final_receipt_stops_a_descendant_of_an_exited_native(self) -> None:
        plan = [{"grandchild": True}, {"break_receipt": True}, *GOOD, {"exit": 0}]
        result = self.run_guard(plan, wrapper_args=("--exact-model-guard", "--guard-stop-grace-seconds", "0.3"))
        self.assertEqual(result.returncode, 42, result.stderr)
        self.assertIn("receipt could not be written", result.stderr)
        self.assertTrue(self.receipt.is_dir())
        self.assert_dead(self.grandchild_pid())  # known owned descendant, native already gone

    def delayed_consumer_run(self, plan: list[dict[str, object]], delay: float,
                             wrapper_args: tuple[str, ...]) -> tuple[int, bytes]:
        """Run the launcher with output nobody reads for `delay` seconds; return (exit, stderr)."""
        proc = subprocess.Popen(self.command(plan, wrapper_args=wrapper_args), env=self.environment(plan),
                                stdin=subprocess.DEVNULL, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
        try:
            time.sleep(delay)
            _, err = proc.communicate(timeout=30)
        finally:
            if proc.poll() is None:
                proc.kill()
                proc.communicate(timeout=30)
        return proc.returncode, err

    def test_deadline_expiring_during_final_delivery_is_a_timeout(self) -> None:
        plan = [{"grandchild": True}, {"bulk": [262144, 262144]}, *GOOD, {"exit": 0}]
        wrapper = ("--exact-model-guard", "--guard-timeout-seconds", "0.5", "--guard-stop-grace-seconds", "0.3")
        returncode, _ = self.delayed_consumer_run(plan, 0.8, wrapper)  # native is done well before the deadline
        self.assertEqual(returncode, 124)
        guard = self.guard()
        self.assertEqual(guard["termination"]["cause"], "timeout")
        self.assertEqual((guard["outcome"], guard["reason"]), ("incomplete_evidence", "timeout"))
        self.assertEqual(guard["acceptance"], "invalidated")
        self.assertEqual(json.loads(self.receipt.read_text())["result"], "guard_incomplete")
        self.assert_dead(self.grandchild_pid())

    def test_timeout_during_delivery_with_an_unwritable_receipt_stays_nonzero_and_collects(self) -> None:
        plan = [{"grandchild": True}, {"break_receipt": True}, {"bulk": [262144, 262144]}, *GOOD, {"exit": 0}]
        wrapper = ("--exact-model-guard", "--guard-timeout-seconds", "0.5", "--guard-stop-grace-seconds", "0.3")
        returncode, err = self.delayed_consumer_run(plan, 0.8, wrapper)
        self.assertEqual(returncode, 124)
        self.assertIn(b"receipt could not be written", err)
        self.assert_dead(self.grandchild_pid())

    def test_deadline_crossed_inside_receipt_publication_is_a_timeout(self) -> None:
        # Deterministic: every receipt write after the in-flight one takes 0.8s, so the
        # 0.5s deadline passes inside the first final publication, after the native
        # process exited cleanly and all output was delivered.
        harness = r"""
import importlib.util, sys, time
spec = importlib.util.spec_from_file_location("transport", sys.argv[1])
transport = importlib.util.module_from_spec(spec)
spec.loader.exec_module(transport)
original, calls = transport._write_receipt, []
def slow_write(path, receipt):
    calls.append(1)
    if len(calls) > 1:
        time.sleep(0.8)
    original(path, receipt)
transport._write_receipt = slow_write
raise SystemExit(transport.main(sys.argv[2:]))
"""
        plan = [{"grandchild": True}, *GOOD, {"exit": 0}]
        wrapper = ("--exact-model-guard", "--guard-timeout-seconds", "0.5", "--guard-stop-grace-seconds", "0.3")
        command = [sys.executable, "-c", harness, str(SCRIPT), *self.command(plan, wrapper_args=wrapper)[2:]]
        result = subprocess.run(command, env=self.environment(plan), capture_output=True, text=True,
                                stdin=subprocess.DEVNULL, timeout=60, check=False)
        self.assertEqual(result.returncode, 124, result.stderr)
        guard = self.guard()
        self.assertEqual(guard["termination"]["cause"], "timeout")
        self.assertEqual((guard["outcome"], guard["reason"]), ("incomplete_evidence", "timeout"))
        self.assertEqual(guard["acceptance"], "invalidated")
        self.assertEqual(json.loads(self.receipt.read_text())["result"], "guard_incomplete")
        self.assert_dead(self.grandchild_pid())

    # synthetic harness notices

    def test_recovered_synthetic_notice_does_not_change_the_model_verdict(self) -> None:
        notice = {"type": "assistant", "error": "server_error", "session_id": SESSION,
                  "message": {"id": "msg_n", "model": "<synthetic>", "stop_reason": "stop_sequence",
                              "content": [{"type": "text", "text": "API Error: EAI_AGAIN"}]}}
        result = self.run_guard([emit(init_event()), emit(assistant()), emit(notice),
                                 emit(assistant()), emit(result_event())])
        self.assertEqual(result.returncode, 0, result.stderr)
        guard = self.guard()
        self.assertEqual(guard["outcome"], "verified_exact")
        self.assertEqual(guard["observations"]["assistant_models"], {MODEL: 2})  # notice not counted
        self.assertEqual(guard["observations"]["synthetic_assistant_messages"], 1)
        self.assertEqual(guard["observations"]["synthetic_error_categories"], {"server_error": 1})
        self.assertNotIn("EAI_AGAIN", self.receipt.read_text())

    def test_synthetic_notice_does_not_hide_a_later_model_mismatch(self) -> None:
        notice = {"type": "assistant", "error": "server_error", "session_id": SESSION,
                  "message": {"id": "msg_n", "model": "<synthetic>", "stop_reason": "stop_sequence"}}
        result = self.run_guard([emit(assistant()), emit(notice), emit(assistant("claude-opus-5-5")),
                                 emit(result_event())])
        self.assertEqual(result.returncode, 41, result.stderr)
        self.assertEqual(self.guard()["violation"]["kind"], "model_mismatch")

    # delivery accounting

    def test_delivery_accounts_reconcile_when_the_consumer_stalls(self) -> None:
        plan = [{"bulk": [262144, 262144]}, *GOOD]  # below the backlog cap; pipes are not read
        proc = subprocess.Popen(self.command(plan), env=self.environment(plan),
                                stdin=subprocess.DEVNULL, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
        try:
            proc.wait(timeout=30)
        except subprocess.TimeoutExpired:
            proc.kill()
        out, err = proc.communicate(timeout=30)  # what the consumer was actually handed
        self.assertEqual(proc.returncode, 42)
        guard = self.guard()
        receipt = json.loads(self.receipt.read_text())
        self.assertEqual((guard["outcome"], guard["reason"]), ("incomplete_evidence", "output_not_delivered"))
        for name, received in (("stdout", len(out)), ("stderr", len(err))):
            account = guard["delivery"][name]
            self.assertFalse(account["complete"])
            self.assertGreater(account["undelivered_bytes"], 0)
            self.assertEqual(account["captured_bytes"],
                             account["delivered_bytes"] + account["undelivered_bytes"])
            self.assertEqual(account["undelivered_bytes"],
                             account["dropped_bytes"] + account["pending_bytes"] + account["in_flight_bytes"])
            # What reached the consumer: at least what was confirmed, at most that plus the write
            # that had not returned (a prefix of it may already sit in the pipe).
            self.assertGreaterEqual(received, account["delivered_bytes"])
            self.assertLessEqual(received, account["delivered_bytes"] + account["in_flight_bytes"])
        self.assertEqual(guard["delivery"]["stdout"]["captured_bytes"], guard["stream"]["bytes"])
        self.assertEqual(guard["delivery"]["stderr"]["captured_bytes"],
                         receipt["attempts"][0]["stderr_size_bytes"])
        self.assertGreater(guard["stream"]["bytes"], 250_000)  # all of it was captured and judged

    def test_complete_delivery_accounts_every_captured_byte(self) -> None:
        result = self.run_guard([{"bulk": [131072, 131072]}, *GOOD])
        self.assertEqual(result.returncode, 0, result.stderr)
        for name, text in (("stdout", result.stdout), ("stderr", result.stderr)):
            account = self.guard()["delivery"][name]
            self.assertTrue(account["complete"])
            self.assertEqual(account["captured_bytes"], len(text.encode()))
            self.assertEqual(account["delivered_bytes"], account["captured_bytes"])
            self.assertEqual((account["undelivered_bytes"], account["in_flight_bytes"]), (0, 0))


if __name__ == "__main__":
    unittest.main()
