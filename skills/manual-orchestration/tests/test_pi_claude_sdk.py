"""Claude subscription lane (claude-sdk) for containerized Pi: synthetic credentials, local upstream only.

No real Claude credential is read and nothing reaches Anthropic. The request fixture
follows the wire the S0 probe observed (records/pi-container-claude-sdk-s0-20261124).
"""
import json
import os
import pathlib
import shutil
import signal
import subprocess
import sys
import tempfile
import threading
import time
import unittest
from http.server import ThreadingHTTPServer
from types import SimpleNamespace
from unittest.mock import patch
import io

HERE = pathlib.Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parent / "scripts"))
sys.path.insert(0, str(HERE / "role_host_fakes"))
import pi_container as pc  # noqa: E402
import role_host as rh  # noqa: E402
import fake_claude_messages as fake  # noqa: E402
from test_pi_container import UnixHTTP, sample_env  # noqa: E402

MODEL_ID = "claude-haiku-5-5"
HOST_TOKEN = "sk-ant-oat01-" + "H0STSECRETMARKER" * 5
SID = "00000000-0000-4000-8000-0000000000c5"


def request_body(**changes):
    body = {
        "model": MODEL_ID, "stream": True, "max_tokens": 128000,
        "thinking": {"type": "adaptive", "display": "updates"}, "output_config": {"effort": "medium"},
        "context_management": {"edits": [{"type": "clear_thinking_20251015", "keep": "all"}]},
        "metadata": {"user_id": json.dumps({"device_id": "d", "account_uuid": "", "session_id": "s"})},
        "system": [{"type": "text", "text": "x-anthropic-billing-header: cc_version=2.1.325.d8e; cc_entrypoint=sdk-ts;"},
                   {"type": "text", "text": pc.CLAUDE_SDK_IDENTITY, "cache_control": {"type": "ephemeral", "ttl": "1h"}},
                   {"type": "text", "text": "pi prompt", "cache_control": {"type": "ephemeral", "ttl": "1h"}}],
        "tools": [{"name": "DeferredToolPlaceholder", "description": "d", "input_schema": {"type": "object"},
                   "defer_loading": True},
                  {"name": "mcp__pi__read", "description": "r", "input_schema": {"type": "object"}}],
        "messages": [{"role": "user", "content": [{"type": "text", "text": "hi"}]},
                     {"role": "system", "content": [{"type": "text", "text": "reminder"}]}],
    }
    body.update(changes)
    return body


def request_headers(**changes):
    h = {"authorization": "Bearer " + pc.CLAUDE_SDK_PLACEHOLDER, "anthropic-version": "2023-06-01",
         "anthropic-beta": ",".join(sorted(pc.CLAUDE_SDK_BETAS)), "content-type": "application/json",
         "accept": "application/json", "user-agent": "claude-cli/2.1.325 (external, sdk-ts)", "x-app": "cli",
         "x-client-app": "pi-claude-subscription/0.2.4", "x-claude-code-session-id": "s",
         "x-stainless-retry-count": "0", "x-stainless-lang": "js", "x-stainless-timeout": "600",
         "anthropic-dangerous-direct-browser-access": "true"}
    h.update(changes)
    return {k: v for k, v in h.items() if v is not None}


class RequestCheckTests(unittest.TestCase):
    def test_s0_wire_accepted(self):
        self.assertIsNone(pc.check_claude_sdk_request(request_body(), model_id=MODEL_ID))
        self.assertIsNone(pc.claude_sdk_header_refusal(request_headers(), "beta=true"))

    def test_body_refusals(self):
        hosted = request_body(tools=[{"type": "web_search_20250305", "name": "web_search"}])
        inline = request_body(messages=[{"role": "user", "content": [
            {"type": "tool_addition", "tool": {"definition": {"type": "code_execution_20250825", "name": "x"}}}]}])
        cases = {
            "model_not_pinned": request_body(model="claude-opus-5-5"),
            "stream_required": request_body(stream=False),
            "unexpected_field:speed": request_body(speed="fast"),
            "unexpected_field:service_tier": request_body(service_tier="priority"),
            "hosted_tool_refused:web_search_20250305": hosted,
            "hosted_tool_refused:code_execution_20250825": inline,
            "tool_refused": request_body(tools=[{"name": "Bash", "input_schema": {}}]),
            "sdk_billing_header_refused": request_body(system=[{"type": "text", "text": "plain"},
                                                               {"type": "text", "text": pc.CLAUDE_SDK_IDENTITY}]),
            "sdk_identity_refused": request_body(system=[request_body()["system"][0], {"type": "text", "text": "x"}]),
            "context_management_refused": request_body(context_management={"edits": [{"type": "clear_tool_uses_x"}]}),
            "messages_refused": request_body(messages=[{"role": "tool", "content": "x"}]),
            "metadata_refused": request_body(metadata={"user_id": "u", "x": 1}),
        }
        for want, body in cases.items():
            self.assertEqual(pc.check_claude_sdk_request(body, model_id=MODEL_ID), want)

    def test_header_refusals(self):
        cases = {
            "query_refused": (request_headers(), ""),
            "container_credential_not_placeholder": (request_headers(authorization="Bearer sk-ant-oat01-other"), "beta=true"),
            "unexpected_header:x-api-key": (request_headers(**{"x-api-key": "k"}), "beta=true"),
            "retry_refused": (request_headers(**{"x-stainless-retry-count": "1"}), "beta=true"),
            "beta_refused:server-side-fallback-2026-01-01": (request_headers(**{
                "anthropic-beta": "oauth-2025-04-20,server-side-fallback-2026-01-01"}), "beta=true"),
            "oauth_beta_missing": (request_headers(**{"anthropic-beta": "claude-code-20250219"}), "beta=true"),
            "client_not_claude_code": (request_headers(**{"user-agent": "curl/8"}), "beta=true"),
            "client_not_provider_extension": (request_headers(**{"x-client-app": "other/1"}), "beta=true"),
        }
        for want, (headers, query) in cases.items():
            self.assertEqual(pc.claude_sdk_header_refusal(headers, query), want)

    def test_billing_verdicts(self):
        v = pc.claude_sdk_billing_verdict
        self.assertEqual(v([("anthropic-ratelimit-unified-status", "allowed")])["verdict"], "allowed")
        self.assertEqual(v([("Anthropic-Ratelimit-Unified-Status", "allowed_warning")])["verdict"], "allowed")
        self.assertEqual(v([("anthropic-ratelimit-unified-status", "rejected"),
                            ("anthropic-ratelimit-unified-overage-status", "allowed")])["verdict"], "refused")
        self.assertEqual(v([])["verdict"], "not_established")

    def test_digests_separate_billing_block_and_carry_no_content(self):
        a = pc.claude_sdk_request_digests(request_body())
        b = pc.claude_sdk_request_digests(request_body(system=[
            {"type": "text", "text": "x-anthropic-billing-header: cc_version=other; cc_entrypoint=sdk-ts;"},
            *request_body()["system"][1:]]))
        self.assertEqual(a["system_sha256"], b["system_sha256"])
        self.assertNotEqual(a["billing_block_sha256"], b["billing_block_sha256"])
        self.assertEqual(a["tool_names"], ["DeferredToolPlaceholder", "mcp__pi__read"])
        self.assertNotIn("pi prompt", json.dumps(a))

    def test_host_token_shape(self):
        self.assertEqual(pc.claude_oauth_token(HOST_TOKEN), HOST_TOKEN)
        for bad in (pc.CLAUDE_SDK_PLACEHOLDER, "sk-ant-api03-" + "x" * 40, "sk-ant-oat01-short", HOST_TOKEN + "\n"):
            with self.assertRaises(ValueError):
                pc.claude_oauth_token(bad)


class ProxyTests(unittest.TestCase):
    def setUp(self):
        self.tmp = pathlib.Path(tempfile.mkdtemp(prefix="rhcs-"))
        self.addCleanup(shutil.rmtree, self.tmp)
        self.seen = self.tmp / "upstream.jsonl"
        self.mode = self.tmp / "mode.json"
        self.mode.write_text("{}")
        self.upstream = ThreadingHTTPServer(("127.0.0.1", 0), fake.make_handler(str(self.seen), str(self.mode), "/w/hello.txt"))
        threading.Thread(target=self.upstream.serve_forever, daemon=True).start()
        self.addCleanup(self.upstream.server_close)
        self.addCleanup(self.upstream.shutdown)
        self.halts = []

    def proxy(self, max_requests=4, **kw):
        p = pc.EgressProxy(str(self.tmp / "egress.sock"), upstream=f"http://127.0.0.1:{self.upstream.server_port}",
                           key=HOST_TOKEN, model_id=MODEL_ID, tier=None, log_path=str(self.tmp / "proxy.jsonl"),
                           max_requests=max_requests, provider="claude-sdk",
                           on_usd_exhausted=self.halts.append, **kw).start()
        self.addCleanup(p.stop)
        return p

    def post(self, body=None, headers=None, path="/v1/messages?beta=true", raw=None):
        conn = UnixHTTP(str(self.tmp / "egress.sock"))
        data = raw if raw is not None else json.dumps(body or request_body()).encode()
        conn.request("POST", path, data, headers or request_headers())
        resp = conn.getresponse()
        out = resp.status, resp.read()
        conn.close()
        return out

    def upstream_rows(self):
        return [json.loads(x) for x in self.seen.read_text().splitlines()] if self.seen.exists() else []

    def log(self):
        return [json.loads(x) for x in (self.tmp / "proxy.jsonl").read_text().splitlines()]

    def test_same_bytes_host_bearer_rebuilt_headers_and_receipt(self):
        self.proxy()
        raw = json.dumps(request_body(), separators=(",", ":")).encode()
        status, data = self.post(raw=raw)
        self.assertEqual(status, 200)
        self.assertIn(b"message_stop", data)
        sent = self.upstream_rows()[-1]
        self.assertEqual(sent["path"], "/v1/messages?beta=true")
        self.assertEqual(sent["auth"], "Bearer " + HOST_TOKEN)
        self.assertIsNone(sent["x_api_key"])
        self.assertEqual(sent["body_sha256"], __import__("hashlib").sha256(raw).hexdigest())
        names = {k.lower() for k in sent["headers"]}
        self.assertTrue(names <= pc.CLAUDE_SDK_HEADERS | {"content-length", "host", "accept-encoding"}, names)
        admitted, request = self.log()
        self.assertEqual(admitted["event"], "claude_sdk_admitted")
        self.assertEqual((request["event"], request["n"], request["disposition"]), ("request", 1, "completed"))
        self.assertEqual(request["billing_verdict"]["verdict"], "allowed")
        self.assertEqual(request["response_model"], MODEL_ID)
        self.assertEqual(request["usage"]["output_tokens"], 7)
        for key in ("system_sha256", "tools_sha256", "tool_names", "effort", "body_sha256"):
            self.assertIn(key, admitted)
        text = (self.tmp / "proxy.jsonl").read_text()
        self.assertNotIn(HOST_TOKEN, text)
        self.assertNotIn("pi prompt", text)
        self.assertEqual(pc.claude_sdk_turn_summary(str(self.tmp / "proxy.jsonl"))["evidence"], "complete")
        self.assertEqual(self.halts, [])

    def test_refusal_before_dispatch_halts_turn(self):
        self.proxy()
        status, _ = self.post(headers=request_headers(authorization="Bearer sk-ant-oat01-" + "Z" * 40))
        self.assertEqual(status, 403)
        status, _ = self.post()
        self.assertEqual(status, 429)
        self.assertEqual(self.upstream_rows(), [])
        self.assertEqual(len(self.halts), 1)
        summary = pc.claude_sdk_turn_summary(str(self.tmp / "proxy.jsonl"))
        self.assertEqual((summary["admitted"], summary["evidence"]), (0, "complete"))

    def test_each_refused_shape_never_reaches_upstream(self):
        for body, headers, path in (
                (request_body(tools=[{"type": "web_search_20250305", "name": "w"}]), None, None),
                (request_body(model="claude-opus-5-5"), None, None),
                (None, request_headers(**{"x-stainless-retry-count": "2"}), None),
                (None, request_headers(**{"anthropic-beta": "oauth-2025-04-20,unknown-beta"}), None),
                (None, None, "/v1/messages/count_tokens?beta=true"),
                (None, None, "/v1/messages")):
            self.setUp()
            self.proxy()
            status, _ = self.post(body=body, headers=headers, **({"path": path} if path else {}))
            self.assertEqual(status, 403)
            self.assertEqual(self.upstream_rows(), [])

    def test_upstream_error_not_relayed_and_no_replay(self):
        self.proxy()
        self.mode.write_text(json.dumps({"status": 500}))
        status, data = self.post()
        self.assertEqual(status, 502)
        self.assertNotIn(b"a@b.c", data)
        self.mode.write_text("{}")
        self.assertEqual(self.post()[0], 429)
        self.assertEqual(len(self.upstream_rows()), 1)
        row = [r for r in self.log() if r["event"] == "request"][0]
        self.assertEqual((row["disposition"], row["http_status"], row["error_type"]), ("upstream_error", 500, "api_error"))
        self.assertNotIn("a@b.c", (self.tmp / "proxy.jsonl").read_text())

    def test_billing_not_allowed_releases_no_output(self):
        for billing, want in (("omit", "billing_not_established"), ("rejected", "billing_refused")):
            self.setUp()
            self.proxy()
            self.mode.write_text(json.dumps({"billing": billing}))
            status, data = self.post()
            self.assertEqual(status, 502)
            self.assertNotIn(b"FAKE-DONE", data)
            self.assertNotIn(b"toolu_fake", data)
            row = [r for r in self.log() if r["event"] == "request"][0]
            self.assertEqual(row["disposition"], want)
            self.assertFalse(row["output_released"])
            self.assertEqual(len(self.halts), 1)

    def test_wrong_response_model_releases_no_output(self):
        self.proxy()
        self.mode.write_text(json.dumps({"model": "claude-sonnet-5-5"}))
        status, data = self.post()
        self.assertEqual(status, 502)
        self.assertNotIn(b"toolu_fake", data)
        row = [r for r in self.log() if r["event"] == "request"][0]
        self.assertEqual((row["disposition"], row["response_model"]), ("response_model_mismatch", "claude-sonnet-5-5"))

    def test_truncated_stream_is_not_completed_and_halts(self):
        self.proxy()
        self.mode.write_text(json.dumps({"terminal": False}))
        self.post()
        row = [r for r in self.log() if r["event"] == "request"][0]
        self.assertEqual(row["disposition"], "terminal_missing")
        self.assertEqual(self.post()[0], 429)

    def test_request_cap_halts(self):
        self.proxy(max_requests=1)
        self.assertEqual(self.post()[0], 200)
        self.assertEqual(self.post()[0], 429)
        self.assertEqual(len(self.upstream_rows()), 1)
        self.assertEqual(len(self.halts), 1)

    def test_admission_persistence_failure_never_forwards(self):
        p = self.proxy()
        real = p._log_locked

        def failing(row, *, durable=False):
            if row.get("event") == "claude_sdk_admitted":
                raise OSError("disk full")
            return real(row, durable=durable)
        p._log_locked = failing
        self.assertEqual(self.post()[0], 429)
        self.assertEqual(self.upstream_rows(), [])
        self.assertEqual(len(self.halts), 1)

    def test_dispatched_without_disposition_is_unresolved_not_zero(self):
        self.proxy()
        self.mode.write_text(json.dumps({"hang": 3}))
        t = threading.Thread(target=self.post, daemon=True)
        t.start()
        for _ in range(100):
            if self.upstream_rows():
                break
            time.sleep(0.05)
        summary = pc.claude_sdk_turn_summary(str(self.tmp / "proxy.jsonl"))
        self.assertEqual((summary["evidence"], summary["unresolved"]), ("unresolved", [1]))
        self.assertEqual(summary["requests"][0]["disposition"], "dispatch_unknown")
        t.join(timeout=10)

    def test_queued_request_after_incomplete_exchange_is_never_forwarded(self):
        """Review P1-1: the halt is set before waiters wake, even if halt reporting is slow."""
        p = self.proxy()
        p.on_halt = lambda detail: (time.sleep(0.5), self.halts.append(detail))
        woken = []
        real_notify = p.settled.notify_all

        def notify_all():
            # Waiters woken after the failed exchange must already find the turn halted.
            woken.append(p.halted)
            real_notify()
        p.settled.notify_all = notify_all
        self.mode.write_text(json.dumps({"hang": 1, "terminal": False}))
        results = {}
        a = threading.Thread(target=lambda: results.setdefault("a", self.post()), daemon=True)
        a.start()
        for _ in range(100):
            if self.upstream_rows():
                break
            time.sleep(0.02)
        self.mode.write_text("{}")
        time.sleep(0.2)
        results["b"] = self.post()
        a.join(timeout=10)
        self.assertEqual(woken, ["terminal_missing"])
        self.assertEqual(results["b"][0], 429)
        self.assertEqual(len(self.upstream_rows()), 1)
        self.assertEqual([r["n"] for r in self.log() if r["event"] == "claude_sdk_admitted"], [1])

    def test_ambiguous_json_is_refused_before_dispatch(self):
        """Review P1-2: the forwarded bytes must have exactly the reading that was validated."""
        valid = json.dumps(request_body())
        cases = ['{"model": "claude-sonnet-9-9", ' + valid[1:],
                 valid[:-1] + ', "metadata": {"user_id": "a", "user_id": "b"}}',
                 valid[:-1].replace('"max_tokens": 128000', '"max_tokens": NaN') + '}']
        for raw in cases:
            self.setUp()
            self.proxy()
            status, _ = self.post(raw=raw.encode())
            self.assertEqual(status, 400, raw[:80])
            self.assertEqual(self.upstream_rows(), [])
            self.assertEqual(len(self.halts), 1)
        with self.assertRaises(ValueError):
            pc.strict_json(b'{"a": {"b": 1, "b": 2}}')

    def test_provider_fetched_sources_refused(self):
        body = request_body(messages=[{"role": "user", "content": [
            {"type": "image", "source": {"type": "url", "url": "https://example.invalid/?q=x"}}]}])
        self.assertEqual(pc.check_claude_sdk_request(body, model_id=MODEL_ID), "source_refused:image")
        inline = request_body(messages=[{"role": "user", "content": [
            {"type": "image", "source": {"type": "base64", "media_type": "image/png", "data": "AA=="}}]}])
        self.assertIsNone(pc.check_claude_sdk_request(inline, model_id=MODEL_ID))

    def test_outcome_known_only_when_every_exchange_completed(self):
        self.proxy()
        self.mode.write_text(json.dumps({"terminal": False}))
        self.post()
        s = pc.claude_sdk_turn_summary(str(self.tmp / "proxy.jsonl"))
        self.assertEqual((s["evidence"], s["outcome_known"]), ("complete", False))
        self.setUp()
        self.proxy()
        self.post()
        self.assertTrue(pc.claude_sdk_turn_summary(str(self.tmp / "proxy.jsonl"))["outcome_known"])

    def test_summary_classifies_missing_and_corrupt_logs(self):
        self.assertEqual(pc.claude_sdk_turn_summary(str(self.tmp / "absent.jsonl"))["evidence"], "missing")
        bad = self.tmp / "bad.jsonl"
        bad.write_text('{"event": "request", "provider": "claude-sdk", "n": 1}\n')
        self.assertEqual(pc.claude_sdk_turn_summary(str(bad))["evidence"], "corrupt")
        bad.write_text('{"event": "claude_sdk_admitted", "n": 1}\n{"event": "claude_sdk_admitted"')
        self.assertEqual(pc.claude_sdk_turn_summary(str(bad))["evidence"], "corrupt")

    def test_construction_refuses_spend_controls(self):
        base = dict(upstream="http://127.0.0.1:1", key=HOST_TOKEN, model_id=MODEL_ID, tier=None,
                    log_path=str(self.tmp / "x.jsonl"), provider="claude-sdk")
        for extra in ({"max_requests": None}, {"max_requests": 0}, {"max_requests": 2, "max_output_tokens": 10},
                      {"max_requests": 2, "model_id": "claude-sonnet-5-5"}, {"max_requests": 2, "key": "sk-ant-api03-x"}):
            with self.assertRaises(ValueError):
                pc.EgressProxy(str(self.tmp / "s.sock"), **{**base, **extra})
        with self.assertRaises(ValueError):
            pc.EgressProxy(str(self.tmp / "s.sock"), **{**base, "max_requests": 2, "tier": "priority"})


def write_credential(path, token=HOST_TOKEN, expires=None, mode=0o600):
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps({"claudeAiOauth": {"accessToken": token, "refreshToken": "sk-ant-ort01-r",
                                                  "expiresAt": expires or (time.time() + 3600) * 1000}}))
    os.chmod(path, mode)
    return path


class LaneOptionTests(unittest.TestCase):
    def setUp(self):
        self.tmp = pathlib.Path(tempfile.mkdtemp(prefix="rhcs-"))
        self.addCleanup(shutil.rmtree, self.tmp)

    def opts(self, *extra, lane="subscription", tier=None):
        return rh.Pi.options(["--model", f"claude-sdk/{MODEL_ID}", *extra], lane, tier)

    def test_v1_spending_semantics(self):
        rh.Pi.claude_sdk_options(self.opts(), {"max_requests": 2})
        for container in ({}, {"max_requests": 0}, {"max_requests": 2, "max_usd": 1.0},
                          {"max_requests": 2, "max_output_tokens": 5}, {"max_requests": 2, "usd_strict_tier": True},
                          {"max_requests": 2, "model_file": "/x"}):
            with self.assertRaises(rh.HostError):
                rh.Pi.claude_sdk_options(self.opts(), container)
        for extra in (("--extension", "/x.ts"), ("--approve",), ("-a",)):
            with self.assertRaises(rh.HostError):
                rh.Pi.claude_sdk_options(self.opts(*extra), {"max_requests": 2})
        with self.assertRaises(rh.HostError):
            rh.Pi.claude_sdk_options(rh.Pi.options(["--model", "claude-sdk/claude-sonnet-5-5"], "subscription", None),
                                     {"max_requests": 2})
        host = rh.Pi("unused")
        with self.assertRaises(rh.HostError):
            host.launch(cwd=str(self.tmp), name="x", prompt="p", host_args=["--model", f"claude-sdk/{MODEL_ID}"],
                        ledger=rh.Ledger(self.tmp / "state"), lane="api", container={"max_requests": 2})

    def test_tool_less_and_replaced_prompt_pass_through(self):
        opts = self.opts("--no-tools", "--system-prompt", "You score.")
        self.assertEqual(opts["argv"], ["--model", f"claude-sdk/{MODEL_ID}", "--no-tools",
                                         "--system-prompt", "You score."])
        rh.Pi.claude_sdk_options(opts, {"max_requests": 1})

    def test_credential_custody_without_refresh(self):
        path = self.tmp / "home/.claude/.credentials.json"
        with patch.dict(os.environ, {"ROLE_HOST_CLAUDE_CREDENTIALS": str(path)}):
            with self.assertRaises(rh.HostError):
                rh.Pi.claude_credential()
            write_credential(path)
            before = path.read_bytes()
            self.assertEqual(rh.Pi.claude_credential(), HOST_TOKEN)
            self.assertEqual(path.read_bytes(), before)
            for bad in (dict(mode=0o644), dict(expires=(time.time() + 30) * 1000), dict(token="sk-ant-api03-" + "x" * 40),
                        dict(token=pc.CLAUDE_SDK_PLACEHOLDER)):
                write_credential(path, **bad)
                with self.assertRaises(rh.HostError) as e:
                    rh.Pi.claude_credential()
                self.assertNotIn(HOST_TOKEN, str(e.exception.detail))
            real = write_credential(self.tmp / "real.json")
            path.unlink()
            path.symlink_to(real)
            with self.assertRaises(rh.HostError):
                rh.Pi.claude_credential()

    def test_credential_and_config_paths_are_reserved(self):
        home = self.tmp / "home"
        (home / ".claude").mkdir(parents=True)
        (self.tmp / "ws").mkdir()
        with patch.dict(os.environ, {"HOME": str(home)}):
            reserved = rh.Pi.claude_reserved()
            for mount in (str(home / ".claude"),):
                with self.assertRaises(pc.ContainerError):
                    pc.mount_plan(str(self.tmp / "ws"), [mount], [], home=str(self.tmp / "other"), reserved=reserved)

    def test_agent_config_rewrite_does_not_follow_planted_symlinks(self):
        agent = self.tmp / "agent"
        agent.mkdir()
        victim = self.tmp / "victim.json"
        victim.write_text("HOST FILE")
        (agent / "settings.json").symlink_to(victim)
        (agent / "claude-sdk.json").write_text(json.dumps({"promptMode": "pi"}))
        (agent / "trust.json").write_text(json.dumps({"/w": True}))
        (agent / "models.json").write_text(json.dumps({"providers": {"claude-sdk": {"baseUrl": "http://evil"}}}))
        rh.Pi.write_claude_sdk_agent_config(agent)
        self.assertEqual(victim.read_text(), "HOST FILE")
        self.assertFalse((agent / "settings.json").is_symlink())
        self.assertEqual(json.loads((agent / "settings.json").read_text()), pc.CLAUDE_SDK_PI_SETTINGS)
        self.assertFalse((agent / "claude-sdk.json").exists())
        self.assertFalse((agent / "models.json").exists())
        self.assertFalse((agent / "trust.json").exists())
        shutil.rmtree(agent)
        agent.symlink_to(self.tmp)
        with self.assertRaises(rh.HostError):
            rh.Pi.write_claude_sdk_agent_config(agent)

    def test_container_env_has_only_synthetic_credential_and_lane_switches(self):
        env = sample_env(self.tmp)
        env["selection"].update(model=f"claude-sdk/{MODEL_ID}", tier=None, lane="subscription")
        env["claude_sdk"] = {"claude": "/b/claude"}
        values = pc.container_env(env, "/record")
        self.assertEqual(values["ANTHROPIC_BASE_URL"], f"http://127.0.0.1:{pc.RELAY_PORT}")
        self.assertEqual(values["WORK_ENGINE_CLAUDE_SDK_CREDENTIAL"], "synthetic")
        self.assertEqual(values["CLAUDE_CODE_MAX_RETRIES"], "0")
        self.assertEqual(values["PI_CLAUDE_SDK_PROMPT"], "claude")
        for k in ("ANTHROPIC_API_KEY", "ANTHROPIC_AUTH_TOKEN", "CLAUDE_CODE_OAUTH_TOKEN"):
            self.assertNotIn(k, values)

    def test_entry_writes_only_the_placeholder(self):
        import pi_container_entry as entry
        self.assertEqual(entry.CLAUDE_SDK_PLACEHOLDER, pc.CLAUDE_SDK_PLACEHOLDER)
        entry.write_claude_credential(str(self.tmp))
        record = json.loads((self.tmp / ".claude/.credentials.json").read_text())
        self.assertEqual(record["claudeAiOauth"]["accessToken"], pc.CLAUDE_SDK_PLACEHOLDER)
        self.assertEqual(os.stat(self.tmp / ".claude/.credentials.json").st_mode & 0o777, 0o600)


class TurnCustodyTests(unittest.TestCase):
    """Turn intent and fault cuts through role_host with the runner and Docker replaced."""

    def setUp(self):
        folder = tempfile.TemporaryDirectory(prefix="rhcs-")
        self.addCleanup(folder.cleanup)
        self.tmp = pathlib.Path(folder.name)
        self.ledger = rh.Ledger(self.tmp / "state")
        self.host = rh.Pi("unused")
        base = self.tmp / "state" / "pi" / SID
        self.env = sample_env(self.tmp)
        self.env["selection"].update(model=f"claude-sdk/{MODEL_ID}", tier=None, lane="subscription")
        self.env.update(container_dir=str(base / "container"), host_dir=str(base / "host"),
                        agent_dir=str(base / "container" / "agent"), session_dir=str(base / "container" / "sessions"),
                        fingerprint={"sha256": "test"}, max_output_tokens=None, usd_limit=None, max_requests=2,
                        claude_sdk={"root": "/b", "extension": "/b/pi-claude-subscription", "claude": "/b/claude",
                                    "packages": ["/b/pi-claude-subscription"]})
        for key in ("container_dir", "host_dir", "agent_dir", "session_dir"):
            pathlib.Path(self.env[key]).mkdir(parents=True, exist_ok=True)
        self.opts = {"argv": [], "model": f"claude-sdk/{MODEL_ID}", "provider": "claude-sdk", "thinking": None,
                     "lane": "subscription", "tier": None, "claude_sdk": True}
        self.enterContext(patch.object(self.host, "provider_key", return_value=HOST_TOKEN))
        self.enterContext(patch.object(self.host, "check_lane", return_value={}))
        self.enterContext(patch.object(self.host, "_verify_header"))
        self.enterContext(patch.object(self.host, "_check_environment", return_value=self.env))
        self.enterContext(patch.object(self.host, "socket_root", return_value=self.tmp / "sockets"))
        self.enterContext(patch.object(self.host, "_container_state", return_value=None))

    def spawn(self, runner=None, event="launch"):
        def default(argv, **kw):
            return SimpleNamespace(pid=os.getpid(), stdin=io.BytesIO())
        with patch.object(rh.subprocess, "Popen", runner or default):
            return self.host._spawn(SID, self.opts, cwd=str(self.tmp), prompt="p", ledger=self.ledger,
                                    event=event, name="n", env=self.env)

    def test_intent_is_durable_before_runner_and_argv_pins_extensions(self):
        seen = {}

        def runner(argv, **kw):
            intent = pathlib.Path(self.env["host_dir"]) / "turn-1.intent.json"
            seen["intent"] = json.loads(intent.read_text())
            spec = json.loads(pathlib.Path(argv[-1]).read_text())
            seen["pi_argv"] = spec["pi_argv"]
            seen["spec"] = json.dumps(spec)
            return SimpleNamespace(pid=os.getpid(), stdin=io.BytesIO())
        row = self.spawn(runner)
        self.assertEqual(row["turn"], 1)
        self.assertEqual(seen["intent"]["container"], f"we-pi-{SID}-1")
        argv = seen["pi_argv"]
        self.assertIn("--no-extensions", argv)
        self.assertEqual(argv[argv.index("--no-extensions") + 1:argv.index("--no-extensions") + 4],
                         ["--no-approve", "-e", "/b/pi-claude-subscription"])
        self.assertNotIn(HOST_TOKEN, seen["spec"])
        self.assertEqual(json.loads((pathlib.Path(self.env["agent_dir"]) / "settings.json").read_text()),
                         pc.CLAUDE_SDK_PI_SETTINGS)

    def test_failed_intent_write_starts_nothing(self):
        real = rh.Pi._write_prompt

        def failing(path, prompt, *, durable=False):
            if str(path).endswith(".intent.json"):
                raise rh.HostError(rh.NOT_ESTABLISHED, "turn_not_established", "disk full")
            return real(path, prompt, durable=durable)

        def runner(*a, **kw):
            self.fail("runner started after a failed intent write")
        with patch.object(rh.Pi, "_write_prompt", staticmethod(failing)):
            with self.assertRaises(rh.HostError):
                self.spawn(runner)
        self.assertEqual(self.ledger.rows(), [])

    def test_launcher_death_after_runner_start_is_discovered_and_blocks_delivery(self):
        def runner_then_die(argv, **kw):
            spec = json.loads(pathlib.Path(argv[-1]).read_text())
            # The proxy admitted and dispatched a request; no disposition, no launch row.
            pathlib.Path(spec["files"]["proxy_log"]).write_text(
                json.dumps({"event": "claude_sdk_admitted", "n": 1, "provider": "claude-sdk"}) + "\n")
            raise KeyboardInterrupt("launcher killed")
        with self.assertRaises(KeyboardInterrupt):
            self.spawn(runner_then_die)
        self.assertEqual(self.ledger.rows(), [])
        st = self.host.status(SID, self.ledger)
        self.assertEqual(st["state"], "unrecorded_turn")
        self.assertEqual(st["unrecorded_turns"][0]["evidence"], "unresolved")
        self.assertEqual(st["unrecorded_turns"][0]["unresolved"], [1])
        with patch.object(self.host, "_collect_leftovers", return_value={}) as collect:
            out = self.host.retire(SID, ledger=self.ledger)
        self.assertEqual(out["retired"], "unrecorded_turns_collected")
        collect.assert_called_once()
        # Evidence stays unresolved after collection: still not zero, still discoverable.
        self.assertEqual(self.host._unrecorded_intents(SID, self.ledger)[0]["evidence"], "unresolved")

    def test_unrecorded_turn_with_live_runner_is_running_and_refuses_delivery_and_plain_retire(self):
        """Review P2-3/P2-8: a live runner without a ledger row reads as running, with a turn."""
        def runner_then_die(argv, **kw):
            spec = json.loads(pathlib.Path(argv[-1]).read_text())
            pathlib.Path(spec["files"]["runner_record"]).write_text(json.dumps(
                {"event": "runner_started", "pid": os.getpid(), "pid_start": pc.process_start_time(os.getpid())}) + "\n")
            raise KeyboardInterrupt("launcher killed")
        with self.assertRaises(KeyboardInterrupt):
            self.spawn(runner_then_die)
        st = self.host.status(SID, self.ledger)
        self.assertEqual((st["state"], st["turn"]), ("running", 1))
        with self.assertRaises(rh.HostError) as e:
            self.spawn(event="deliver")
        self.assertEqual(e.exception.status, "refused_busy")
        with self.assertRaises(rh.HostError) as e:
            self.host.retire(SID, ledger=self.ledger)
        self.assertEqual(e.exception.status, "refused_busy")
        record = pathlib.Path(self.env["host_dir"]) / "turn-1.runner.jsonl"
        with record.open("a") as fh:
            fh.write(json.dumps({"event": "container_removed", "exit": 0}) + "\n")
        self.assertEqual(self.host.status(SID, self.ledger)["state"], "unrecorded_turn")

    def test_runner_death_after_dispatch_blocks_next_delivery(self):
        def runner(argv, **kw):
            spec = json.loads(pathlib.Path(argv[-1]).read_text())
            pathlib.Path(spec["files"]["proxy_log"]).write_text(
                json.dumps({"event": "claude_sdk_admitted", "n": 1, "provider": "claude-sdk"}) + "\n")
            return SimpleNamespace(pid=os.getpid(), stdin=io.BytesIO())
        self.spawn(runner)
        with patch.object(self.host, "status", return_value={"state": "idle"}):
            with self.assertRaises(rh.HostError) as e:
                self.spawn(event="deliver")
        self.assertEqual(e.exception.status, "turn_not_resolved")


@unittest.skipUnless(shutil.which("docker") and pathlib.Path(pc.DEFAULT_CLAUDE_SDK_BUNDLE).exists()
                     and subprocess.run(["docker", "image", "inspect", pc.IMAGE], capture_output=True).returncode == 0,
                     "Docker image or Claude SDK bundle unavailable")
class RealClaudeSdkContainerTests(unittest.TestCase):
    """Pi + pinned pi-claude-subscription bundle in real containers against the local fake upstream.

    The 'host credential' is a synthetic marker in a temporary HOME; nothing reaches Anthropic.
    """
    bundle = pc.DEFAULT_CLAUDE_SDK_BUNDLE

    def setUp(self):
        self.tmp = pathlib.Path(tempfile.mkdtemp(prefix="rhcs-", dir=os.path.expanduser("~/.cache")))
        self.addCleanup(shutil.rmtree, self.tmp, ignore_errors=True)
        for d in ("home", "ws", "state", "run", "agent"):
            (self.tmp / d).mkdir()
        os.chmod(self.tmp / "run", 0o700)
        (self.tmp / "ws" / "hello.txt").write_text("container-file-contents\n")
        self.credential = write_credential(self.tmp / "home/.claude/.credentials.json")
        self.mode = self.tmp / "mode.json"
        self.mode.write_text("{}")
        self.seen = self.tmp / "upstream.jsonl"
        port = self.tmp / "port"
        self.server = subprocess.Popen([sys.executable, str(HERE / "role_host_fakes/fake_claude_messages.py"),
                                        "--port-file", str(port), "--log", str(self.seen), "--mode-file", str(self.mode),
                                        "--read-path", str(self.tmp / "ws" / "hello.txt")])
        self.addCleanup(self.server.wait)
        self.addCleanup(self.server.terminate)
        for _ in range(100):
            if port.exists() and port.read_text():
                break
            time.sleep(0.05)
        self.env = {"PATH": os.environ["PATH"], "HOME": str(self.tmp / "home"), "USER": os.environ.get("USER", ""),
                    "XDG_RUNTIME_DIR": str(self.tmp / "run"), "PI_CODING_AGENT_DIR": str(self.tmp / "agent"),
                    "PI_OFFLINE": "1", "PI_TELEMETRY": "0", "ROLE_HOST_CLAUDE_SDK_BUNDLE": self.bundle,
                    "ROLE_HOST_PI_UPSTREAM_CLAUDE_SDK": f"http://127.0.0.1:{port.read_text()}"}
        self.addCleanup(self.remove_containers)

    def remove_containers(self):
        for path in (self.tmp / "state").rglob("turn-*.intent.json"):
            subprocess.run(["docker", "rm", "-f", json.loads(path.read_text())["container"]], capture_output=True)

    def host(self, *args):
        r = subprocess.run([sys.executable, str(HERE.parent / "scripts/role_host.py"), "--state", str(self.tmp / "state"),
                            *args], capture_output=True, text=True, env=self.env, timeout=300)
        return r.returncode, (json.loads(r.stdout) if r.returncode == 0 else json.loads(r.stderr or "{}"))

    def wait_settled(self, sid):
        for _ in range(1200):
            code, st = self.host("status", "pi", sid)
            if code != 0 or st["state"] != "running":
                return code, st
            time.sleep(0.1)
        self.fail("turn still running")

    def launch(self, prompt="Read hello.txt with the read tool, then reply."):
        p = self.tmp / f"prompt-{time.monotonic_ns()}"
        p.write_text(prompt)
        return self.host("launch", "pi", "--cwd", str(self.tmp / "ws"), "--name", "claude-sdk-test",
                         "--prompt-file", str(p), "--auth", "subscription", "--container", "--max-requests", "3",
                         "--host-arg=--model", f"--host-arg=claude-sdk/{MODEL_ID}", "--host-arg=--tools",
                         "--host-arg=read")

    def test_launch_tool_round_deliver_status_and_custody(self):
        code, row = self.launch()
        self.assertEqual(code, 0, row)
        sid = row["session_id"]
        code, st = self.wait_settled(sid)
        self.assertEqual(code, 0, st)
        turn = st["last_turn"]
        self.assertEqual(turn["outcome"], "completed", turn)
        sdk = turn["egress"]["claude_sdk"]
        self.assertEqual((sdk["evidence"], sdk["admitted"], sdk["completed"]), ("complete", 2, 2), sdk)
        self.assertEqual(sdk["response_models"], [MODEL_ID])
        self.assertEqual(sdk["billing_verdicts"], ["allowed"])
        self.assertTrue(turn["container"]["removed"])
        sent = [json.loads(x) for x in self.seen.read_text().splitlines()]
        self.assertEqual({r["auth"] for r in sent}, {"Bearer " + HOST_TOKEN})
        self.assertTrue(any(c.get("type") == "tool_result" for m in sent[-1]["body"]["messages"]
                            if isinstance(m.get("content"), list) for c in m["content"]))
        p = self.tmp / "msg"
        p.write_text("Reply once more.")
        code, got = self.host("deliver", "pi", sid, "--cwd", str(self.tmp / "ws"), "--message-file", str(p))
        self.assertEqual((code, got.get("turn")), (0, 2), got)
        code, st = self.wait_settled(sid)
        self.assertEqual(st["last_turn"]["outcome"], "completed", st)
        self.assertEqual(st["last_turn"]["egress"]["claude_sdk"]["evidence"], "complete")
        for path in (self.tmp / "state").rglob("*"):
            if path.is_file():
                self.assertNotIn(HOST_TOKEN.encode(), path.read_bytes(), str(path))
        self.assertEqual(json.loads(self.credential.read_text())["claudeAiOauth"]["accessToken"], HOST_TOKEN)
        code, retired = self.host("retire", "pi", sid)
        self.assertEqual(code, 0, retired)
        # Join: the same host evidence through the usage ledger, once, outside the API pool.
        rows = self.ledger_rows()
        self.assertEqual(len(rows), 3, rows)
        self.assertEqual({r["claude_sdk"]["disposition"] for r in rows}, {"completed"})
        self.assertEqual({tuple(r["pools"]) for r in rows}, {("claude_all_models",)})
        self.assertEqual({r["usd"] for r in rows}, {None})

    def ledger_rows(self):
        sys.path.insert(0, str(HERE.parents[2] / "metrics/scripts"))
        import usage_ledger
        usage_ledger.ISSUES.clear()
        rows, _ = usage_ledger.build_pi(None, None, str(self.tmp / "state"))
        return [r for r in rows if r.get("claude_sdk")]

    def test_upstream_error_halts_without_replay(self):
        self.mode.write_text(json.dumps({"status": 500}))
        code, row = self.launch()
        self.assertEqual(code, 0, row)
        code, st = self.wait_settled(row["session_id"])
        self.assertEqual(len(self.seen.read_text().splitlines()), 1, "request was replayed")
        sdk = st["last_turn"]["egress"]["claude_sdk"]
        self.assertEqual((sdk["admitted"], sdk["not_completed"], sdk["evidence"]), (1, 1, "complete"))

    def test_runner_killed_after_dispatch_leaves_unresolved_and_blocks_delivery(self):
        self.mode.write_text(json.dumps({"hang": 60}))
        code, row = self.launch()
        self.assertEqual(code, 0, row)
        sid = row["session_id"]
        for _ in range(600):
            if self.seen.exists() and self.seen.read_text():
                break
            time.sleep(0.1)
        os.killpg(row["pid"], signal.SIGKILL)
        code, st = self.host("status", "pi", sid)
        self.assertEqual(code, 0, st)
        code, retired = self.host("retire", "pi", sid, "--force")
        self.assertEqual(code, 0, retired)
        code, st = self.host("status", "pi", sid)
        self.assertEqual(st["last_turn"]["egress"]["claude_sdk"]["evidence"], "unresolved", st)
        p = self.tmp / "msg"
        p.write_text("again")
        code, got = self.host("deliver", "pi", sid, "--cwd", str(self.tmp / "ws"), "--message-file", str(p))
        self.assertNotEqual(code, 0)
        self.assertEqual(got.get("status"), "turn_not_resolved", got)
        rows = self.ledger_rows()
        self.assertEqual([r["claude_sdk"]["disposition"] for r in rows], ["dispatch_unknown"])
        self.assertEqual(rows[0]["usage"], {})


if __name__ == "__main__":
    unittest.main()
