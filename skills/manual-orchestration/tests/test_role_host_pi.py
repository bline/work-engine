"""Pi host in role_host.py.

Unit tests need no Pi. The integration class drives the installed `pi` binary
against tests/role_host_fakes/fake_openai_responses.py: the openai provider's
baseUrl points at 127.0.0.1, the key is fake, PI_OFFLINE is set and no real
credential is inherited, so no provider is contacted and nothing is billed.
"""
import json
import os
import pathlib
import shutil
import subprocess
import sys
import tempfile
import time
import unittest

ROOT = pathlib.Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "scripts"))
import role_host  # noqa: E402

HOST = ROOT / "scripts" / "role_host.py"
FAKE_SERVER = pathlib.Path(__file__).parent / "role_host_fakes" / "fake_openai_responses.py"
SID = "00000000-0000-4000-8000-000000000001"
MODEL = "openai/gpt-6.1-sol"


def options(*host_args, lane="api", tier=None):
    return role_host.Pi.options(list(host_args), lane, tier)


class OptionTests(unittest.TestCase):
    def refused(self, *host_args, lane="api", tier=None):
        with self.assertRaises(role_host.HostError) as cm:
            options(*host_args, lane=lane, tier=tier)
        self.assertEqual(cm.exception.code, role_host.USAGE)
        return cm.exception.detail

    def test_lane_is_required(self):
        self.assertIn("--auth", self.refused("--model", MODEL, lane=None))

    def test_exact_model_is_required(self):
        self.refused("--thinking", "high")
        self.refused("--model", "gpt-6.1-sol")
        self.refused("--model", "openai/*sol*")
        self.refused("--model", MODEL, "--model", MODEL)

    def test_thinking_goes_through_its_own_flag(self):
        self.assertIn("--thinking", self.refused("--model", MODEL + ":high"))
        self.refused("--model", MODEL, "--thinking", "huge")

    def test_session_mode_and_credential_flags_are_refused(self):
        for flag in (["--api-key", "x"], ["--session", "x"], ["--session-id", "x"], ["--continue"],
                     ["--mode", "rpc"], ["--provider", "openai"], ["--no-session"], ["--fork", "x"]):
            self.refused("--model", MODEL, *flag)

    def test_pass_through_flags_are_canonical(self):
        got = options("--model", MODEL, "--thinking=high", "-t", "read,bash", "--no-mcp", "-e", "x.ts")
        self.assertEqual(got["argv"], ["--model", MODEL, "--thinking", "high", "--tools", "read,bash",
                                       "--no-mcp", "--extension", "x.ts"])
        self.assertEqual((got["provider"], got["thinking"], got["lane"], got["tier"]), ("openai", "high", "api", None))

    def test_tier_only_on_api_lane_for_openai(self):
        self.assertEqual(options("--model", MODEL, tier="priority")["tier"], "priority")
        self.assertIn("subscription", self.refused("--model", "openai-codex/gpt-6.1-sol", lane="subscription", tier="priority"))
        self.assertIn("openai", self.refused("--model", "openrouter/anthropic/claude-sonnet-5", tier="flex"))
        self.refused("--model", MODEL, tier="turbo")


class FakeAuthMixin:
    def fake_pi(self, auth):
        """A `pi` that only answers `auth check` with a fixed JSON reply."""
        exe = pathlib.Path(self.tmp.name) / "pi"
        exe.write_text(f"#!/bin/sh\necho '{json.dumps(auth)}'\n")
        exe.chmod(0o755)
        return role_host.Pi(str(exe))


class LaneTests(FakeAuthMixin, unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)

    def check(self, auth, opts, env=None):
        return self.fake_pi(auth).check_lane(opts, cwd=self.tmp.name, env=env or {"PATH": os.environ["PATH"]})

    def test_lane_must_match_credential_type(self):
        api = options("--model", MODEL)
        self.assertEqual(self.check({"status": "ready", "provider": "openai", "authType": "api_key"}, api)["auth_type"], "api_key")
        with self.assertRaises(role_host.HostError) as cm:
            self.check({"status": "ready", "provider": "openai", "authType": "oauth"}, api)
        self.assertEqual(cm.exception.status, "lane_mismatch")
        sub = options("--model", "openai-codex/gpt-6.1-sol", lane="subscription")
        with self.assertRaises(role_host.HostError) as cm:
            self.check({"status": "ready", "provider": "openai-codex", "authType": "api_key"}, sub)
        self.assertEqual(cm.exception.status, "lane_mismatch")
        self.assertEqual(self.check({"status": "ready", "provider": "openai-codex", "authType": "oauth"}, sub)["auth_type"], "oauth")

    def test_missing_credential_is_not_ready(self):
        with self.assertRaises(role_host.HostError) as cm:
            self.check({"status": "not_ready", "provider": "openai", "reason": "credentials_not_configured"},
                       options("--model", MODEL))
        self.assertEqual((cm.exception.code, cm.exception.status), (role_host.NOT_ESTABLISHED, "credential_not_ready"))

    def test_non_api_openai_key_is_refused_on_api_lane(self):
        ready = {"status": "ready", "provider": "openai", "authType": "api_key"}
        with self.assertRaises(role_host.HostError) as cm:
            self.check(ready, options("--model", MODEL), env={"PATH": os.environ["PATH"], "OPENAI_API_KEY": "eyJ.token"})
        self.assertEqual(cm.exception.status, "lane_mismatch")
        self.check(ready, options("--model", MODEL), env={"PATH": os.environ["PATH"], "OPENAI_API_KEY": "sk-test"})


class ObservationTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.dir = pathlib.Path(self.tmp.name)

    def turn(self, events, *, exit_code=0, tier_rows=(), requested=None):
        files = {k: self.dir / f"turn-1.{v}" for k, v in role_host.Pi.TURN_FILES.items()}
        files["events"].write_text("".join(json.dumps(e) + "\n" for e in events))
        files["exit"].unlink(missing_ok=True)
        if exit_code is not None:
            files["exit"].write_text(f"{exit_code}\n")
        files["tier_record"].write_text("".join(json.dumps(r) + "\n" for r in tier_rows))
        row = {"session_id": SID, "turn": 1, "model": MODEL, "service_tier": requested,
               "files": {k: str(v) for k, v in files.items()}}
        return role_host.Pi().observe_turn(row)

    @staticmethod
    def events(stop="stop", aborted=False, model="gpt-6.1-sol"):
        reply = {"role": "assistant", "provider": "openai", "model": model, "stopReason": stop,
                 "usage": {"cost": {"total": 0.25}}}
        return [{"type": "session", "id": SID}, {"type": "message_end", "message": reply},
                {"type": "agent_settled", "aborted": aborted}]

    def test_completed_turn(self):
        got = self.turn(self.events())
        self.assertEqual((got["outcome"], got["model_match"], got["pi_reported_cost_usd"]), ("completed", True, 0.25))
        self.assertEqual(got["service_tier"]["status"], "not_requested")

    def test_failure_modes(self):
        self.assertEqual(self.turn(self.events(stop="error"))["outcome"], "failed")
        self.assertEqual(self.turn(self.events(aborted=True))["outcome"], "aborted")
        self.assertEqual(self.turn(self.events(stop="length"))["outcome"], "incomplete")
        self.assertEqual(self.turn(self.events(), exit_code=None)["outcome"], "interrupted")
        self.assertEqual(self.turn(self.events(), exit_code=1)["outcome"], "failed")
        self.assertEqual(self.turn(self.events()[1:])["outcome"], "not_established")

    def test_served_model_mismatch(self):
        got = self.turn(self.events(model="gpt-6.1-terra"))
        self.assertEqual((got["model_match"], got["model_served"]), (False, ["openai/gpt-6.1-terra"]))

    def test_tier_statuses(self):
        injected = {"event": "injected", "tier": "priority"}

        def status(rows, requested="priority"):
            return self.turn(self.events(), tier_rows=rows, requested=requested)["service_tier"]["status"]
        self.assertEqual(status([injected, {"event": "observed", "tier": "priority"}]), "applied")
        self.assertEqual(status([injected, {"event": "observed", "tier": "default"}]), "mismatch")
        self.assertEqual(status([injected]), "not_established")
        self.assertEqual(status([injected, {"event": "observed", "tier": None}]), "not_established")
        self.assertEqual(status([{"event": "skipped", "reason": "model_mismatch"}]), "not_applied")
        self.assertEqual(status([{"event": "injected", "tier": "auto"}, {"event": "observed", "tier": "default"}],
                                requested="auto"), "applied")

    def test_partial_last_record_is_left_out(self):
        files = self.dir / "turn-1.events.jsonl"
        self.turn(self.events())
        files.write_text(files.read_text() + '{"type": "message_')
        records, partial = role_host.Pi._records(files)
        self.assertTrue(partial)
        self.assertEqual(records[-1]["type"], "agent_settled")


class IdentityTests(unittest.TestCase):
    def test_pi_identity_variables_are_not_inherited(self):
        for var in ("PI_SESSION_ID", "PI_SESSION_FILE", "PI_PROVIDER", "PI_MODEL", "PI_REASONING_LEVEL"):
            self.assertIn(var, role_host.CALLER_IDENTITY)

    def run_host(self, *args, env):
        return subprocess.run([sys.executable, str(HOST), *args], capture_output=True, text=True, env=env)

    def test_whoami_reads_pi_session(self):
        base = {k: v for k, v in os.environ.items() if k not in role_host.CALLER_IDENTITY}
        r = self.run_host("whoami", env={**base, "PI_SESSION_ID": SID})
        self.assertEqual(r.returncode, 0, r.stderr)
        self.assertEqual(json.loads(r.stdout), {"host": "pi", "session_id": SID, "source": "PI_SESSION_ID"})
        r = self.run_host("whoami", env={**base, "PI_SESSION_ID": SID, "CODEX_THREAD_ID": SID})
        self.assertEqual(r.returncode, role_host.NOT_ESTABLISHED)

    def test_pi_options_refused_for_other_hosts(self):
        with tempfile.TemporaryDirectory() as tmp:
            prompt = pathlib.Path(tmp) / "p"
            prompt.write_text("x")
            r = self.run_host("--state", tmp, "launch", "codex", "--cwd", tmp, "--name", "n",
                              "--prompt-file", str(prompt), "--auth", "api", env=dict(os.environ))
        self.assertEqual(r.returncode, role_host.USAGE)


@unittest.skipUnless(shutil.which("pi"), "pi is not installed")
class RealPiTests(unittest.TestCase):
    """Real pi, fake OpenAI endpoint. Each turn takes about a second."""

    def setUp(self):
        self.tmp = pathlib.Path(tempfile.mkdtemp(prefix="role-host-pi-"))
        self.addCleanup(shutil.rmtree, self.tmp, ignore_errors=True)
        for d in ("agent", "home", "ws", "ws2", "state"):
            (self.tmp / d).mkdir()
        subprocess.run(["git", "init", "-q", str(self.tmp / "ws")], check=True)
        (self.tmp / "ws" / "AGENTS.md").write_text("ROLE-HOST-PI-MARKER\n")
        self.mode = self.tmp / "mode.json"
        self.mode.write_text("{}")
        self.log = self.tmp / "requests.jsonl"
        port_file = self.tmp / "port"
        self.server = subprocess.Popen([sys.executable, str(FAKE_SERVER), "--port-file", str(port_file),
                                        "--log", str(self.log), "--mode-file", str(self.mode)])
        self.addCleanup(self.server.wait)
        self.addCleanup(self.server.terminate)
        for _ in range(100):
            if port_file.exists():
                break
            time.sleep(0.05)
        port = port_file.read_text()
        (self.tmp / "agent" / "models.json").write_text(json.dumps(
            {"providers": {"openai": {"baseUrl": f"http://127.0.0.1:{port}/v1"}}}))
        (self.tmp / "agent" / "settings.json").write_text("{}")
        # Built from scratch: no real provider credential reaches pi.
        self.env = {"PATH": os.environ["PATH"], "HOME": str(self.tmp / "home"),
                    "PI_CODING_AGENT_DIR": str(self.tmp / "agent"), "PI_OFFLINE": "1", "PI_TELEMETRY": "0",
                    "OPENAI_API_KEY": "sk-fake-role-host-test"}
        self.addCleanup(self.stop_all)

    def stop_all(self):
        ledger = self.tmp / "state" / "ledger.jsonl"
        if not ledger.exists():
            return
        for line in ledger.read_text().splitlines():
            row = json.loads(line)
            if row.get("pid"):
                try:
                    os.killpg(row["pid"], 15)
                except (ProcessLookupError, PermissionError):
                    pass

    def host(self, *args):
        r = subprocess.run([sys.executable, str(HOST), "--state", str(self.tmp / "state"), *args],
                           capture_output=True, text=True, env=self.env, timeout=120)
        return r.returncode, (json.loads(r.stdout) if r.returncode == 0 else json.loads(r.stderr or "{}"))

    def launch(self, *extra, model=MODEL, auth="api", prompt="Reply OK."):
        pf = self.tmp / f"prompt-{time.monotonic_ns()}"
        pf.write_text(prompt)
        return self.host("launch", "pi", "--cwd", str(self.tmp / "ws"), "--name", "pi-test", "--prompt-file", str(pf),
                         "--parent", SID, "--auth", auth, "--host-arg=--model", f"--host-arg={model}",
                         "--host-arg=--thinking", "--host-arg=low", *extra)

    def wait_idle(self, sid):
        for _ in range(300):
            code, st = self.host("status", "pi", sid)
            if code != 0 or st["state"] != "running":
                return code, st
            time.sleep(0.1)
        self.fail("turn still running")

    def requests(self):
        return [json.loads(line)["body"] for line in self.log.read_text().splitlines()]

    def test_launch_deliver_and_tier(self):
        code, row = self.launch("--service-tier", "priority")
        self.assertEqual(code, 0, row)
        sid = row["session_id"]
        self.assertTrue(row["session_header_verified"])
        self.assertEqual(row["credential"], {"provider": "openai", "auth_type": "api_key"})
        code, st = self.wait_idle(sid)
        self.assertEqual((code, st["state"]), (0, "idle"), st)
        turn = st["last_turn"]
        self.assertEqual((turn["outcome"], turn["model_match"]), ("completed", True), turn)
        self.assertEqual(turn["service_tier"]["status"], "applied", turn)
        first = self.requests()[-1]
        self.assertEqual(first["service_tier"], "priority")
        self.assertIn("ROLE-HOST-PI-MARKER", json.dumps(first))

        message = self.tmp / "msg"
        message.write_text("Second.")
        self.mode.write_text(json.dumps({"tier": "default"}))
        code, got = self.host("deliver", "pi", sid, "--cwd", str(self.tmp / "ws"), "--message-file", str(message))
        self.assertEqual((code, got.get("turn")), (0, 2), got)
        code, st = self.wait_idle(sid)
        self.assertEqual(st["last_turn"]["service_tier"]["status"], "mismatch", st)
        self.assertGreater(len(self.requests()[-1]["input"]), len(first["input"]))
        code, ident = self.host("identity", "pi", sid)
        self.assertEqual(code, 0, ident)
        self.assertEqual(len(list((self.tmp / "agent" / "sessions").glob("*/*.jsonl"))), 1)

        code, got = self.host("deliver", "pi", sid, "--cwd", str(self.tmp / "ws2"), "--message-file", str(message))
        self.assertEqual(code, role_host.USAGE, got)
        code, kids = self.host("children", SID)
        self.assertEqual([(c["host"], c["session_id"], c["state"]) for c in kids["children"]], [("pi", sid, "idle")])

    def test_running_turn_is_busy_until_forced(self):
        self.mode.write_text(json.dumps({"delay": 60}))
        code, row = self.launch()
        self.assertEqual(code, 0, row)
        sid = row["session_id"]
        code, st = self.host("status", "pi", sid)
        self.assertEqual(st["state"], "running", st)
        message = self.tmp / "msg"
        message.write_text("x")
        self.assertEqual(self.host("deliver", "pi", sid, "--cwd", str(self.tmp / "ws"), "--message-file", str(message))[0],
                         role_host.BUSY)
        self.assertEqual(self.host("retire", "pi", sid)[0], role_host.BUSY)
        code, got = self.host("retire", "pi", sid, "--force")
        self.assertEqual((code, got.get("retired")), (0, "turn_terminated"), got)
        code, st = self.host("status", "pi", sid)
        self.assertEqual((st["state"], st["last_turn"]["outcome"]), ("stopped", "interrupted"), st)

    def test_lane_mismatch_launches_nothing(self):
        code, err = self.launch(auth="subscription")
        self.assertEqual((code, err["status"]), (role_host.NOT_ESTABLISHED, "lane_mismatch"), err)
        self.assertFalse((self.tmp / "state" / "ledger.jsonl").exists())
        self.assertFalse(self.log.exists())

    def test_tier_refused_on_subscription_lane(self):
        code, err = self.launch("--service-tier", "priority", auth="subscription", model="openai-codex/gpt-6.1-sol")
        self.assertEqual(code, role_host.USAGE, err)


if __name__ == "__main__":
    unittest.main()
