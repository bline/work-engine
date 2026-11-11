"""Anthropic Messages route of the Pi container runner, with mocked providers only.

No Anthropic request is made and no Anthropic key is used: upstreams are local
fakes and keys are fixed fake strings.
"""
from __future__ import annotations

import json
import os
import pathlib
import shutil
import subprocess
import sys
import tempfile
import threading
import time
import types
import unittest
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from unittest import mock

ROOT = pathlib.Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "scripts"))
sys.path.insert(0, str(ROOT))
import pi_container  # noqa: E402
import role_host  # noqa: E402
from tests import test_pi_container as base  # noqa: E402

HOST = ROOT / "scripts" / "role_host.py"
FAKE = ROOT / "tests" / "role_host_fakes" / "fake_anthropic_messages.py"
MODEL_ID = "claude-fable-5-2"
REAL_KEY = "sk-ant-api03-fake-key-held-by-host"
RATES = pi_container.usd_rates(MODEL_ID, provider="anthropic")
USAGE = {"input_tokens": 11, "cache_read_input_tokens": 300, "cache_creation_input_tokens": 50,
         "cache_creation": {"ephemeral_5m_input_tokens": 40, "ephemeral_1h_input_tokens": 10},
         "output_tokens": 1, "service_tier": "standard"}


def expected_usd(i=11, cr=300, w5=40, w1=10, un=0, o=7):
    r = RATES
    return (i * r["input"] + cr * r["cache_read"] + w5 * r["cache_write_5m"] + w1 * r["cache_write_1h"]
            + un * max(r["cache_write_5m"], r["cache_write_1h"]) + o * r["output"]) / 1e6


def request(**over):
    body = {"model": MODEL_ID, "max_tokens": 128000, "stream": True, "system": [{"type": "text", "text": "sys"}],
            "messages": [{"role": "user", "content": [{"type": "text", "text": "hi"}]}],
            "tools": [{"name": "bash", "description": "run", "input_schema": {"type": "object"}}],
            "thinking": {"type": "adaptive", "display": "summarized"}, "output_config": {"effort": "high"}}
    body.update(over)
    return {k: v for k, v in body.items() if v is not None}


class Upstream(BaseHTTPRequestHandler):
    """Streams an Anthropic message. Class settings: status, usage, delta_usage,
    stop (False: no message_stop), pause (seconds before the closing events), model,
    first (raw first data line replacing message_start; b"" sends no events),
    ping (a ping before message_start), tool (a tool_use block), restate (model of a
    second message_start after the first content)."""
    seen = []
    status, usage, delta_usage, stop, pause, model = 200, USAGE, {"output_tokens": 7}, True, 0, None
    first, ping, tool, restate = None, False, False, None

    def log_message(self, *a):
        pass

    def send(self, kind, data):
        self.wfile.write(f"event: {kind}\ndata: {json.dumps({'type': kind, **data})}\n\n".encode())
        self.wfile.flush()

    def do_POST(self):
        body = json.loads(self.rfile.read(int(self.headers["content-length"])))
        Upstream.seen.append({"path": self.path, "body": body, "headers": dict(self.headers.items())})
        if Upstream.status != 200:
            payload = b'{"type": "error", "error": {"type": "overloaded_error", "message": "fake"}}'
            self.send_response(Upstream.status)
            self.send_header("content-length", str(len(payload)))
            self.end_headers()
            self.wfile.write(payload)
            return
        self.send_response(200)
        self.send_header("content-type", "text/event-stream")
        self.end_headers()
        if Upstream.ping:
            self.send("ping", {})
        if Upstream.first is not None:
            if Upstream.first:
                self.wfile.write(b"event: message_start\ndata: " + Upstream.first + b"\n\n")
        else:
            self.send("message_start", {"message": {"id": "msg_1", "model": Upstream.model or body["model"],
                                                    "usage": Upstream.usage}})
        if Upstream.first == b"":
            return
        try:
            if Upstream.tool:
                self.send("content_block_start", {"index": 1, "content_block": {
                    "type": "tool_use", "id": "toolu_1", "name": "bash", "input": {}}})
                self.send("content_block_delta", {"index": 1, "delta": {
                    "type": "input_json_delta", "partial_json": '{"command": "touch /x"}'}})
                self.send("content_block_stop", {"index": 1})
            if Upstream.restate:
                time.sleep(0.2)  # the verified content goes out in its own read
                self.send("message_start", {"message": {"id": "msg_2", "model": Upstream.restate,
                                                        "usage": Upstream.usage}})
            time.sleep(Upstream.pause)
            for _ in range(20 if Upstream.pause else 1):
                self.send("content_block_delta", {"index": 0, "delta": {"type": "text_delta", "text": "x" * 4000}})
                time.sleep(Upstream.pause / 10)
            self.send("message_delta", {"delta": {"stop_reason": "end_turn"}, "usage": Upstream.delta_usage})
            if Upstream.stop:
                self.send("message_stop", {})
        except OSError:
            pass


class Harness(unittest.TestCase):
    def setUp(self):
        self.tmp = pathlib.Path(tempfile.mkdtemp())
        self.addCleanup(shutil.rmtree, self.tmp)
        Upstream.seen = []
        Upstream.status, Upstream.usage, Upstream.delta_usage = 200, USAGE, {"output_tokens": 7}
        Upstream.stop, Upstream.pause, Upstream.model = True, 0, None
        Upstream.first, Upstream.ping, Upstream.tool, Upstream.restate = None, False, False, None
        self.upstream = ThreadingHTTPServer(("127.0.0.1", 0), Upstream)
        threading.Thread(target=self.upstream.serve_forever, daemon=True).start()
        self.addCleanup(self.upstream.server_close)
        self.addCleanup(self.upstream.shutdown)
        self.exhausted = []

    def proxy(self, max_usd=None, cap=None, **kw):
        budget = None
        if max_usd is not None:
            cap = cap or 16000
            budget = pi_container.AnthropicUsdBudget(max_usd=max_usd, rates=RATES, tier=None, max_output_tokens=cap)
        p = pi_container.EgressProxy(str(self.tmp / "egress.sock"), upstream=f"http://127.0.0.1:{self.upstream.server_port}",
                                     key=REAL_KEY, model_id=MODEL_ID, tier=None, log_path=str(self.tmp / "proxy.jsonl"),
                                     max_output_tokens=cap, budget=budget, provider="anthropic",
                                     on_usd_exhausted=self.exhausted.append, **kw).start()
        self.addCleanup(p.stop)
        return p

    def post(self, body, path="/v1/messages?beta=true", method="POST", headers=None):
        conn = base.UnixHTTP(str(self.tmp / "egress.sock"))
        h = {"x-api-key": pi_container.ANTHROPIC_PLACEHOLDER_KEY, "content-type": "application/json",
             "anthropic-version": "2023-06-01", "authorization": "Bearer container-token", "x-extra": "1"}
        h.update(headers or {})
        conn.request(method, path, json.dumps(body), h)
        resp = conn.getresponse()
        data = resp.read()
        conn.close()
        return resp.status, data

    def log(self):
        return [json.loads(line) for line in (self.tmp / "proxy.jsonl").read_text().splitlines()]

    def requests(self):
        return [r for r in self.log() if r.get("event") == "request"]


class AnthropicRouteTests(Harness):
    def test_key_substituted_headers_rebuilt_usage_captured(self):
        self.proxy()
        betas = "interleaved-thinking-2025-05-14, fine-grained-tool-streaming-2025-05-14"
        status, data = self.post(request(), headers={"anthropic-beta": betas})
        self.assertEqual(status, 200, data)
        self.assertIn(b"message_stop", data)
        sent = Upstream.seen[0]
        h = {k.lower(): v for k, v in sent["headers"].items()}
        self.assertEqual(sent["path"], "/v1/messages?beta=true")
        self.assertEqual(h["x-api-key"], REAL_KEY)
        self.assertEqual(h["anthropic-version"], pi_container.ANTHROPIC_VERSION)
        self.assertEqual(h["anthropic-beta"], "interleaved-thinking-2025-05-14,fine-grained-tool-streaming-2025-05-14")
        self.assertNotIn("authorization", h)
        self.assertNotIn("x-extra", h)
        self.assertEqual(sent["body"]["max_tokens"], 128000)  # no cap without one
        row = self.requests()[0]
        self.assertEqual((row["provider"], row["terminal"], row["response_id"], row["response_model"], row["stop_reason"],
                          row["thinking"], row["effort"]),
                         ("anthropic", "message_stop", "msg_1", MODEL_ID, "end_turn", "adaptive", "high"))
        self.assertEqual(row["usage"], {"input_tokens": 11, "cache_read_input_tokens": 300,
                                        "cache_creation_input_tokens": 50, "ephemeral_5m_input_tokens": 40,
                                        "ephemeral_1h_input_tokens": 10, "output_tokens": 7})
        self.assertNotIn("model_mismatch", row)
        self.assertNotIn(REAL_KEY, (self.tmp / "proxy.jsonl").read_text())

    def assert_identity_refused(self, reason, data, status, response_model=None):
        """Refused before any upstream byte reached the client; reservation kept; audit event; no key."""
        self.assertEqual(status, 502, data)
        self.assertIn(reason.encode(), data)
        for leaked in (b"content_block", b"tool_use", b"text_delta", b"message_start", b"msg_1"):
            self.assertNotIn(leaked, data)
        (ev,) = [r for r in self.log() if r.get("event") == "response_model_refused"]
        self.assertEqual((ev["reason"], ev["authorized_model"], ev["response_model"], ev["verified_bytes_forwarded"]),
                         (reason, MODEL_ID, response_model, False))
        (row,) = self.requests()
        self.assertEqual(row["response_model_check"], reason)
        self.assertEqual((row["usd"]["charge_basis"], row["usd"]["charged_usd"]),
                         ("reservation", row["usd"]["reserved_usd"]))
        totals = pi_container.usd_totals(self.log())
        self.assertAlmostEqual(totals["charged_usd"], row["usd"]["reserved_usd"])
        self.assertEqual(len(Upstream.seen), 1)  # no retry, no fallback request
        self.assertNotIn(REAL_KEY, (self.tmp / "proxy.jsonl").read_text())

    def test_same_model_accepted_and_recorded(self):
        Upstream.ping, Upstream.tool = True, True
        self.proxy(max_usd=5)
        status, data = self.post(request())
        self.assertEqual(status, 200, data)
        self.assertIn(b"tool_use", data)
        self.assertIn(b"message_stop", data)
        row = self.requests()[0]
        self.assertEqual((row["response_model_check"], row["usd"]["charge_basis"]), ("verified", "usage"))
        self.assertFalse([r for r in self.log() if r.get("event") == "response_model_refused"])

    def test_served_by_another_model_is_refused_before_any_content(self):
        Upstream.model, Upstream.tool = "claude-opus-5", True
        self.proxy(max_usd=5)
        status, data = self.post(request())
        self.assert_identity_refused("response_model_mismatch", data, status, "claude-opus-5")
        self.assertTrue(self.requests()[0]["model_mismatch"])

    def test_missing_or_malformed_model_refused(self):
        msg = {"id": "msg_1", "usage": USAGE}
        cases = [
            (json.dumps({"type": "message_start", "message": msg}), "response_model_missing", None),
            (json.dumps({"type": "message_start", "message": {**msg, "model": None}}), "response_model_missing", None),
            (json.dumps({"type": "message_start", "message": {**msg, "model": 5}}), "response_model_malformed", "5"),
            (json.dumps({"type": "message_start", "message": {**msg, "model": ""}}), "response_model_malformed", ""),
            (json.dumps({"type": "message_start", "message": {**msg, "model": [MODEL_ID]}}),
             "response_model_malformed", repr([MODEL_ID])),
            (json.dumps({"type": "message_start", "message": "x"}), "response_model_malformed", None),
            (json.dumps({"type": "content_block_delta", "delta": {"text": "x"}}), "response_model_not_first_event", None),
            ("{not json", "response_model_not_first_event", None),
            (json.dumps({"type": "message_start", "message": {**msg, "model": MODEL_ID + "-x"}}),
             "response_model_mismatch", MODEL_ID + "-x"),
        ]
        for first, reason, logged in cases:
            with self.subTest(reason=reason, first=first):
                self.setUp()
                Upstream.first, Upstream.tool = first.encode(), True
                self.proxy(max_usd=5)
                status, data = self.post(request())
                self.assert_identity_refused(reason, data, status, logged)

    def test_stream_ending_before_identity_fails_and_keeps_reservation(self):
        Upstream.first = b""
        self.proxy(max_usd=5)
        status, data = self.post(request())
        self.assert_identity_refused("response_model_not_established", data, status)

    def test_identity_hold_is_bounded(self):
        Upstream.first = json.dumps({"type": "ping", "pad": "p" * 5000}).encode()
        with mock.patch.object(pi_container, "ANTHROPIC_IDENTITY_HOLD_BYTES", 1000):
            self.proxy(max_usd=5)
            status, data = self.post(request())
        self.assert_identity_refused("response_model_not_established", data, status)

    def test_restated_model_stops_the_stream(self):
        Upstream.restate, Upstream.tool = "claude-opus-5", True
        self.proxy(max_usd=5)
        status, data = self.post(request())
        self.assertEqual(status, 200)  # verified bytes were already sent
        self.assertIn(b"tool_use", data)  # from the verified model, before the restatement
        self.assertNotIn(b"msg_2", data)
        self.assertNotIn(b"message_stop", data)
        (ev,) = [r for r in self.log() if r.get("event") == "response_model_refused"]
        self.assertEqual((ev["reason"], ev["verified_bytes_forwarded"]), ("response_model_restated", True))
        self.assertEqual(self.requests()[0]["usd"]["charge_basis"], "reservation")

    def test_refusals_never_reach_upstream(self):
        self.proxy()
        hosted = {"type": "web_search_20250305", "name": "web_search"}
        inline = [{"role": "user", "content": [{"type": "tool_addition", "tool": {
            "type": "tool_definition", "definition": {"type": "code_execution_20250825", "name": "x"}}}]}]
        cases = [
            (request(), "/v1/responses", {}, "only POST /v1/messages is allowed"),
            (request(), "/v1/messages?beta=false", {}, "query_refused"),
            (request(fallbacks=[{"model": "claude-opus-5"}]), None, {}, "unexpected_field:fallbacks"),
            (request(speed="fast"), None, {}, "unexpected_field:speed"),
            (request(service_tier="priority"), None, {}, "unexpected_field:service_tier"),
            (request(mcp_servers=[]), None, {}, "unexpected_field:mcp_servers"),
            (request(model="claude-opus-5"), None, {}, "model_not_pinned"),
            (request(stream=False), None, {}, "stream_required"),
            (request(), None, {"anthropic-beta": "server-side-fallback-2026-07-01"},
             "beta_refused:server-side-fallback-2026-07-01"),
            (request(tools=[hosted]), None, {}, "hosted_tool_refused:web_search_20250305"),
            (request(messages=inline), None, {}, "hosted_tool_refused:code_execution_20250825"),
            (request(tools={"name": "x"}), None, {}, "tools_not_list"),
            (request(tool_choice="auto"), None, {}, "tool_choice_refused"),
            (request(thinking={"type": "unbounded"}), None, {}, "thinking_refused"),
            (request(output_config={"effort": "high", "task_budget": 1}), None, {}, "output_config_refused"),
        ]
        for body, path, headers, reason in cases:
            status, data = self.post(body, path=path or "/v1/messages?beta=true", headers=headers)
            self.assertEqual(status, 403, (reason, data))
            self.assertIn(reason, json.loads(data)["error"]["message"])
        status, _ = self.post(request(), method="GET")
        self.assertEqual(status, 403)
        self.assertEqual(Upstream.seen, [])

    def test_custom_tools_in_both_places_pass(self):
        self.proxy()
        inline = [{"role": "user", "content": [{"type": "tool_addition", "tool": {
            "type": "tool_definition", "definition": {"type": "custom", "name": "read", "input_schema": {}}}}]}]
        status, _ = self.post(request(messages=inline, tool_choice={"type": "tool", "name": "bash"}),
                              headers={"anthropic-beta": "inline-tools-2026-09-15"})
        self.assertEqual(status, 200)

    def test_error_event_and_missing_stop_leave_no_terminal(self):
        Upstream.stop = False
        self.proxy()
        self.post(request())
        self.assertIsNone(self.requests()[0]["terminal"])

    def test_unsupported_provider_and_tier_rejected_at_construction(self):
        with self.assertRaises(ValueError):
            pi_container.EgressProxy(str(self.tmp / "s"), upstream="http://x", key="k", model_id=MODEL_ID, tier=None,
                                     log_path=str(self.tmp / "l"), provider="google")
        with self.assertRaises(ValueError):
            pi_container.EgressProxy(str(self.tmp / "s"), upstream="http://x", key="k", model_id=MODEL_ID,
                                     tier="priority", log_path=str(self.tmp / "l"), provider="anthropic")
        openai_budget = pi_container.UsdBudget(max_usd=1, rates=pi_container.usd_rates("gpt-6.1-sol"), tier=None,
                                               max_output_tokens=10)
        with self.assertRaises(ValueError):
            pi_container.EgressProxy(str(self.tmp / "s"), upstream="http://x", key="k", model_id=MODEL_ID, tier=None,
                                     log_path=str(self.tmp / "l"), provider="anthropic", max_output_tokens=10,
                                     budget=openai_budget)


class AnthropicBudgetTests(unittest.TestCase):
    def budget(self, max_usd=1.0, cap=1000):
        return pi_container.AnthropicUsdBudget(max_usd=max_usd, rates=RATES, tier=None, max_output_tokens=cap)

    def basis(self, n=2):
        return pi_container.input_basis(request(messages=[{"role": "user", "content": "m%d" % i} for i in range(n)]),
                                        items_key="messages")

    def test_worst_case_uses_dearest_input_rate(self):
        b = self.budget()
        dearest = max(RATES["input"], RATES["cache_write_5m"], RATES["cache_write_1h"])
        self.assertEqual(dearest, RATES["cache_write_1h"])
        self.assertAlmostEqual(b.worst_case(1000), (1000 * dearest + 1000 * RATES["output"]) / 1e6)

    def test_settle_prices_lifetimes_unknown_ttl_and_tier(self):
        b = self.budget()
        u = {"input_tokens": 11, "cache_read_input_tokens": 300, "cache_creation_input_tokens": 60,
             "ephemeral_5m_input_tokens": 40, "ephemeral_1h_input_tokens": 10, "output_tokens": 7}
        t, _ = b.reserve(self.basis())
        out = b.settle(t, http_status=200, response={"usage": u})
        self.assertEqual((out["charge_basis"], out["input_total"]), ("usage", 371))
        self.assertAlmostEqual(out["charged_usd"], round(expected_usd(un=10), 6))
        self.assertEqual(b.prefix["input_tokens"], 371)
        t, _ = b.reserve(self.basis(3))
        self.assertEqual(t["bound_basis"], "prefix")
        out = b.settle(t, http_status=200, response={"usage": {**u, "service_tier": "priority"}})
        self.assertEqual(out["charge_basis"], "reservation")
        t, _ = b.reserve(self.basis(3))
        self.assertEqual(b.settle(t, http_status=529, response={})["charged_usd"], 0.0)
        t, _ = b.reserve(self.basis(3))
        bad = {**u, "ephemeral_1h_input_tokens": 100}  # split exceeds the total
        self.assertEqual(b.settle(t, http_status=200, response={"usage": bad})["charge_basis"], "reservation")

    def test_refusal_latches(self):
        b = self.budget(max_usd=0.05)
        self.assertIsNotNone(b.reserve(self.basis())[0])
        self.assertIsNone(b.reserve(self.basis())[0])
        self.assertTrue(b.exhausted)

    def test_prior_state_reads_input_total(self):
        rows = [{"event": "usd_reserved", "rid": 1, "reserved_usd": 0.5, "bound_basis": "bytes"},
                {"event": "request", "provider": "anthropic", "usage": {"input_tokens": 11},
                 "input_basis": {"items": 1, "chain_sha256": "c", "rest_sha256": "r"},
                 "usd": {"rid": 1, "charged_usd": 0.01, "charge_basis": "usage", "input_total": 361}}]
        t = pi_container.usd_totals(rows)
        self.assertEqual((t["prefix"]["input_tokens"], t["unsettled_reserved_usd"]), (361, 0))

    def test_rates_match_the_usage_ledger(self):
        ledger = base.load_ledger()
        table = json.loads(pi_container.ANTHROPIC_RATES_PATH.read_text())
        self.assertEqual(table["models"], {k: v for k, v in ledger.NATIVE_RATES.items() if k != "claude-haiku-5-5"})
        with self.assertRaises(pi_container.ContainerError):
            pi_container.usd_rates("claude-haiku-5-5", provider="anthropic")


class AnthropicProxyUsdTests(Harness):
    def test_cap_reserve_settle_and_latch(self):
        self.proxy(max_usd=0.2, cap=1000)
        status, _ = self.post(request())
        self.assertEqual(status, 200)
        self.assertEqual(Upstream.seen[0]["body"]["max_tokens"], 1000)
        row = self.requests()[0]
        self.assertEqual(row["usd"]["charge_basis"], "usage")
        self.assertAlmostEqual(row["usd"]["charged_usd"], round(expected_usd(), 6))
        self.proxy_limit_refusal()

    def proxy_limit_refusal(self):
        big = request(messages=[{"role": "user", "content": "y" * 40000}])
        status, data = self.post(big)
        self.assertEqual(status, 429, data)
        self.assertEqual(len(self.exhausted), 1)
        status, _ = self.post(request())
        self.assertEqual(status, 429)
        self.assertEqual(len(Upstream.seen), 1)

    def test_thinking_budget_and_unpriceable_inputs_refused(self):
        self.proxy(max_usd=1, cap=1000)
        status, data = self.post(request(thinking={"type": "enabled", "budget_tokens": 1000}))
        self.assertIn(b"thinking_budget_not_below_output_cap", data)
        url_image = [{"role": "user", "content": [{"type": "image", "source": {"type": "url", "url": "http://x"}}]}]
        status, data = self.post(request(messages=url_image))
        self.assertIn(b"usd_limit_unpriceable_input:image", data)
        self.assertEqual(Upstream.seen, [])

    def test_errors_charge_nothing_and_no_stop_keeps_reservation(self):
        self.proxy(max_usd=1, cap=1000)
        Upstream.status = 529
        self.post(request())
        Upstream.status, Upstream.stop = 200, False
        self.post(request())
        a, b = self.requests()
        self.assertEqual((a["usd"]["charge_basis"], a["usd"]["charged_usd"]), ("error_status", 0.0))
        self.assertEqual(b["usd"]["charge_basis"], "reservation")

    def test_cancellation_mid_stream_keeps_reservation(self):
        Upstream.pause = 0.3
        self.proxy(max_usd=1, cap=1000)
        conn = base.UnixHTTP(str(self.tmp / "egress.sock"))
        conn.request("POST", "/v1/messages", json.dumps(request()), {"content-type": "application/json"})
        resp = conn.getresponse()
        resp.read(50)
        resp.close()  # the container goes away mid-stream
        conn.close()
        for _ in range(100):
            rows = self.requests()
            if rows:
                break
            time.sleep(0.1)
        row = rows[0]
        self.assertIsNone(row["terminal"])
        self.assertEqual(row["usd"]["charge_basis"], "reservation")
        self.assertTrue(any(r.get("event") == "stream_error" for r in self.log()))


class AnthropicHostTests(unittest.TestCase):
    def test_model_definition_rules(self):
        tmp = pathlib.Path(tempfile.mkdtemp())
        self.addCleanup(shutil.rmtree, tmp)
        good = {"id": MODEL_ID, "api": "anthropic-messages", "reasoning": True, "contextWindow": 1000000, "maxTokens": 128000}

        def check(d):
            (tmp / "m.json").write_text(json.dumps(d))
            return pi_container.model_definition(str(tmp / "m.json"), provider="anthropic", model_id=MODEL_ID)

        self.assertEqual(check(good), good)
        for bad in ({**good, "id": "claude-opus-5"}, {**good, "api": "openai-responses"},
                    {**good, "baseUrl": "https://elsewhere"}, {**good, "headers": {"x": "y"}},
                    {**good, "compat": {"allowedFallbackModels": [{"provider": "anthropic", "model": "claude-opus-5"}]}}):
            with self.assertRaises(pi_container.ContainerError):
                check(bad)

    def test_container_env_holds_only_the_anthropic_placeholder(self):
        env = base.sample_env(pathlib.Path(tempfile.mkdtemp()))
        env["selection"] = {**env["selection"], "model": f"anthropic/{MODEL_ID}"}
        ce = pi_container.container_env(env, "/r")
        self.assertEqual(ce["ANTHROPIC_API_KEY"], pi_container.ANTHROPIC_PLACEHOLDER_KEY)
        self.assertNotIn("sk-ant-oat", pi_container.ANTHROPIC_PLACEHOLDER_KEY)
        self.assertNotIn("OPENAI_API_KEY", ce)

    def test_provider_key_refuses_oauth_tokens(self):
        fake = types.SimpleNamespace(exe="pi")
        opts = {"provider": "anthropic"}
        for out, ok in (("sk-ant-api03-abc\n", True), ("sk-ant-oat01-abc\n", False), ("sk-proj-x\n", False)):
            with mock.patch.object(role_host, "_run", return_value=subprocess.CompletedProcess([], 0, out, "")):
                if ok:
                    self.assertEqual(role_host.Pi.provider_key(fake, opts, cwd="/"), out.strip())
                else:
                    with self.assertRaises(role_host.HostError):
                        role_host.Pi.provider_key(fake, opts, cwd="/")

    def test_unsupported_provider_and_anthropic_tier_rejected(self):
        tmp = pathlib.Path(tempfile.mkdtemp())
        self.addCleanup(shutil.rmtree, tmp)
        (tmp / "p").write_text("x")
        for model, extra, expect in (("google/gemini-3", [], "routes providers anthropic, openai"),
                                     (f"anthropic/{MODEL_ID}", ["--service-tier", "priority"], "service tier")):
            r = subprocess.run([sys.executable, str(HOST), "--state", str(tmp / "state"), "launch", "pi", "--cwd",
                                str(tmp), "--name", "x", "--prompt-file", str(tmp / "p"), "--auth", "api",
                                "--container", "--host-arg=--model", f"--host-arg={model}", *extra],
                               capture_output=True, text=True, timeout=60)
            self.assertNotEqual(r.returncode, 0)
            self.assertIn(expect, r.stderr + r.stdout)


class AnthropicLedgerTests(unittest.TestCase):
    def test_build_pi_prices_anthropic_rows(self):
        ledger = base.load_ledger()
        tmp = pathlib.Path(tempfile.mkdtemp())
        self.addCleanup(shutil.rmtree, tmp)
        log = tmp / "turn-1.proxy.jsonl"
        usage = {"input_tokens": 11, "cache_read_input_tokens": 300, "cache_creation_input_tokens": 60,
                 "ephemeral_5m_input_tokens": 40, "ephemeral_1h_input_tokens": 10, "output_tokens": 7}
        log.write_text(json.dumps({"at": "2026-11-10T23:00:00Z", "event": "request", "provider": "anthropic",
                                   "http_status": 200, "model": MODEL_ID, "response_model": MODEL_ID,
                                   "observed_tier": "standard", "response_id": "msg_1", "usage": usage,
                                   "seconds": 1.5}) + "\n")
        (tmp / "ledger.jsonl").write_text("\n".join(json.dumps(r) for r in (
            {"host": "pi", "event": "launch", "session_id": "s1", "auth_lane": "api", "parent_session_id": "p"},
            {"host": "pi", "session_id": "s1", "turn": 1, "files": {"proxy_log": str(log)}})) + "\n")
        rows, cov = ledger.build_pi(None, None, state=str(tmp))
        self.assertEqual(cov["pi_container_turns"], 1)
        (row,) = rows
        self.assertEqual((row["pools"], row["usage_shape"], row["model"]), (["pi_api"], "anthropic", MODEL_ID))
        self.assertAlmostEqual(row["usd_low"], expected_usd(un=10) - 10 * (RATES["cache_write_1h"] - RATES["cache_write_5m"]) / 1e6)
        self.assertAlmostEqual(row["usd_high"], expected_usd(un=10))
        self.assertEqual(row["input_context_tokens"], 371)
        self.assertAlmostEqual(row["speed_adjusted_api_usd_high"], row["usd_high"])


SKIP = base.SKIP


@unittest.skipIf(SKIP, SKIP or "")
class RealAnthropicContainerTests(unittest.TestCase):
    """Real Pi in a real container against the local fake Anthropic endpoint."""

    def setUp(self):
        self.tmp = pathlib.Path(tempfile.mkdtemp(prefix="rhpa-"))
        self.addCleanup(shutil.rmtree, self.tmp, ignore_errors=True)
        for d in ("agent", "home", "ws", "state", "run"):
            (self.tmp / d).mkdir()
        os.chmod(self.tmp / "run", 0o700)
        (self.tmp / "agent" / "settings.json").write_text("{}")
        self.mode, self.log = self.tmp / "mode.json", self.tmp / "requests.jsonl"
        self.mode.write_text("{}")
        port = self.tmp / "port"
        self.server = subprocess.Popen([sys.executable, str(FAKE), "--port-file", str(port), "--log", str(self.log),
                                        "--mode-file", str(self.mode)])
        self.addCleanup(self.server.wait)
        self.addCleanup(self.server.terminate)
        for _ in range(100):
            if port.exists():
                break
            time.sleep(0.05)
        self.env = {"PATH": os.environ["PATH"], "HOME": str(self.tmp / "home"), "USER": os.environ.get("USER", ""),
                    "XDG_RUNTIME_DIR": str(self.tmp / "run"), "PI_CODING_AGENT_DIR": str(self.tmp / "agent"),
                    "PI_OFFLINE": "1", "PI_TELEMETRY": "0", "ANTHROPIC_API_KEY": REAL_KEY,
                    "ROLE_HOST_PI_UPSTREAM_ANTHROPIC": f"http://127.0.0.1:{port.read_text()}"}
        self.model_file = self.tmp / "model.json"
        self.model_file.write_text(json.dumps({
            "id": MODEL_ID, "name": "Claude Fable 5.2", "api": "anthropic-messages", "reasoning": True,
            "input": ["text"], "cost": {"input": 8, "output": 40, "cacheRead": 0.2, "cacheWrite": 10},
            "contextWindow": 1000000, "maxTokens": 128000, "compat": {"forceAdaptiveThinking": True},
            "thinkingLevelMap": {"off": None, "xhigh": "xhigh", "max": "max"}}))
        self.addCleanup(self.remove_containers)

    def remove_containers(self):
        ledger = self.tmp / "state" / "ledger.jsonl"
        if ledger.exists():
            for line in ledger.read_text().splitlines():
                name = (json.loads(line).get("container") or {}).get("name")
                if name:
                    subprocess.run(["docker", "rm", "-f", name], capture_output=True)

    def host(self, *args):
        r = subprocess.run([sys.executable, str(HOST), "--state", str(self.tmp / "state"), *args],
                           capture_output=True, text=True, env=self.env, timeout=300)
        return r.returncode, (json.loads(r.stdout) if r.returncode == 0 else (r.stderr or r.stdout))

    def test_tool_turn_custody_usage_and_ledger(self):
        self.mode.write_text(json.dumps({"tool_command": "echo KEY=$ANTHROPIC_API_KEY; echo TOOL-RAN"}))
        prompt = self.tmp / "prompt"
        prompt.write_text("Run the check.")
        code, row = self.host("launch", "pi", "--cwd", str(self.tmp / "ws"), "--name", "anthropic-test",
                              "--prompt-file", str(prompt), "--auth", "api", "--container", "--max-usd", "1",
                              "--container-model-file", str(self.model_file),
                              "--host-arg=--model", f"--host-arg=anthropic/{MODEL_ID}",
                              "--host-arg=--thinking", "--host-arg=high")
        self.assertEqual(code, 0, row)
        sid = row["session_id"]
        for _ in range(600):
            code, st = self.host("status", "pi", sid)
            if code != 0 or st["state"] != "running":
                break
            time.sleep(0.1)
        turn = st["last_turn"]
        self.assertEqual((turn["outcome"], turn["egress"]["requests"], turn["egress"]["refused"]),
                         ("completed", 2, []), turn)
        self.assertTrue(turn["container"]["removed"], turn)
        seen = [json.loads(line) for line in self.log.read_text().splitlines()]
        self.assertEqual([s["headers"]["x-api-key"] for s in seen], [REAL_KEY, REAL_KEY])
        self.assertEqual({s["path"] for s in seen}, {"/v1/messages?beta=true"})
        self.assertTrue(all(s["body"]["model"] == MODEL_ID and s["body"]["max_tokens"] == 16000 for s in seen))
        results = json.dumps(seen[1]["body"]["messages"])
        self.assertIn("TOOL-RAN", results)
        self.assertIn(f"KEY={pi_container.ANTHROPIC_PLACEHOLDER_KEY}", results)
        self.assertNotIn(REAL_KEY, results)
        usd = turn["egress"]["usd"]
        self.assertAlmostEqual(usd["charged_usd"], round(2 * expected_usd(), 6), places=5)
        self.assertEqual(turn["egress"]["usage"]["ephemeral_1h_input_tokens"], 20)
        rows, _ = base.load_ledger().build_pi(None, None, state=str(self.tmp / "state"))
        self.assertEqual(len(rows), 2)
        self.assertAlmostEqual(sum(r["usd"] for r in rows), 2 * expected_usd())
        code, out = self.host("retire", "pi", sid)
        self.assertEqual(code, 0, out)


    def test_wrong_model_tool_call_never_runs(self):
        marker = self.tmp / "ws" / "TOOL-RAN"
        self.mode.write_text(json.dumps({"tool_command": f"touch {marker}", "model": "claude-opus-5"}))
        prompt = self.tmp / "prompt"
        prompt.write_text("Run the check.")
        code, row = self.host("launch", "pi", "--cwd", str(self.tmp / "ws"), "--name", "anthropic-wrong-model",
                              "--prompt-file", str(prompt), "--auth", "api", "--container", "--max-usd", "1",
                              "--container-model-file", str(self.model_file),
                              "--host-arg=--model", f"--host-arg=anthropic/{MODEL_ID}")
        self.assertEqual(code, 0, row)
        sid = row["session_id"]
        for _ in range(600):
            code, st = self.host("status", "pi", sid)
            if code != 0 or st["state"] != "running":
                break
            time.sleep(0.1)
        turn = st["last_turn"]
        self.assertTrue(turn["container"]["removed"], turn)
        self.assertFalse(marker.exists())
        seen = [json.loads(line) for line in self.log.read_text().splitlines()]
        self.assertTrue(seen)
        self.assertNotIn("tool_result", json.dumps([s["body"]["messages"] for s in seen]))
        proxy_log = pathlib.Path(turn["files"]["proxy_log"]) if "files" in turn else None
        rows = [json.loads(line) for line in proxy_log.read_text().splitlines()] if proxy_log else []
        refused = [r for r in rows if r.get("event") == "response_model_refused"]
        self.assertEqual(len(refused), len(seen), rows)
        self.assertTrue(all(r["reason"] == "response_model_mismatch" for r in refused))
        self.assertGreater(turn["egress"]["usd"]["charged_usd"], 0)
        state_text = "".join(p.read_text(errors="replace") for p in (self.tmp / "state").rglob("*") if p.is_file())
        self.assertNotIn(REAL_KEY, state_text)
        self.host("retire", "pi", sid)


if __name__ == "__main__":
    unittest.main()
