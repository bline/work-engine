from __future__ import annotations

import base64
import contextlib
import hashlib
import hmac
import http.client
import http.server
import importlib.util
import io
import json
import secrets
import shutil
import sys
import tempfile
import threading
import time
import unittest
from pathlib import Path

ROOT = Path(__file__).parents[1]


def load():
    spec = importlib.util.spec_from_file_location("session_events", ROOT / "scripts" / "session_events.py")
    module = importlib.util.module_from_spec(spec)
    sys.modules["session_events"] = module  # dataclasses resolve annotations through sys.modules
    spec.loader.exec_module(module)
    return module


SE = load()
CODEX_ID = "01a18fa8-03de-7353-95d6-69f68492281c"
CLAUDE_ID = "7131be18-febe-47a8-9fef-9f97386eafd8"
URL = "https://hooks.example.com/mcp/abc"


def whsec() -> str:
    return "whsec_" + base64.b64encode(secrets.token_bytes(32)).decode()


def verify(secret: str, headers: dict, body: bytes) -> bool:
    key = base64.b64decode(secret[6:])
    signed = f"{headers['webhook-id']}.{headers['webhook-timestamp']}.".encode() + body
    expected = "v1," + base64.b64encode(hmac.new(key, signed, hashlib.sha256).digest()).decode()
    return expected in headers["webhook-signature"].split(" ")


def codex_line(kind: str, timestamp: str = "2026-10-30T10:00:00.000Z", **payload) -> str:
    return json.dumps({"timestamp": timestamp, "type": kind, "payload": payload}) + "\n"


def message(role: str, text: str) -> str:
    content_type = "output_text" if role == "assistant" else "input_text"
    return codex_line("response_item", type="message", role=role, content=[{"type": content_type, "text": text}])


class Receiver:
    """Fake transport: answers verification challenges, then the queued statuses (default 200)."""

    def __init__(self):
        self.calls, self.statuses, self.verify_status = [], [], 200

    def __call__(self, url, body, headers, policy):
        self.calls.append((url, body, dict(headers)))
        parsed = json.loads(body)
        if parsed.get("type") == "verification":
            if self.verify_status != 200:
                return SE.Outcome(self.verify_status, "http_4xx")
            return SE.Outcome(200, None, json.dumps({"challenge": parsed["challenge"]}).encode())
        status = self.statuses.pop(0) if self.statuses else 200
        category = None if status == 200 else ("http_5xx" if status >= 500 else "http_4xx")
        return SE.Outcome(status, category)

    def events(self):
        return [(json.loads(body), headers) for _, body, headers in self.calls
                if "eventId" in json.loads(body)]


class Fixture(unittest.TestCase):
    def setUp(self):
        self.directory = Path(tempfile.mkdtemp(prefix="session-events-test-"))
        self.addCleanup(shutil.rmtree, self.directory, True)
        self.homes = SE.Homes(self.directory / "codex", self.directory / "claude", self.directory / "state")
        day = self.homes.codex / "sessions" / "2026" / "10" / "30"
        day.mkdir(parents=True)
        self.codex_path = day / f"rollout-2026-10-30T10-00-00-{CODEX_ID}.jsonl"
        meta = codex_line("session_meta", id=CODEX_ID, timestamp="2026-10-30T10:00:00Z", cwd="/work")
        self.codex_path.write_text(meta + message("user", "hello there") + message("assistant", "first reply"))
        (self.homes.codex / "session_index.jsonl").write_text(
            json.dumps({"id": CODEX_ID, "thread_name": "Old name"}) + "\n"
            + json.dumps({"id": CODEX_ID, "thread_name": "Assess Dot event monitoring"}) + "\n")
        project = self.homes.claude / "projects" / "-work"
        project.mkdir(parents=True)
        self.claude_path = project / f"{CLAUDE_ID}.jsonl"
        self.claude_path.write_text(
            json.dumps({"type": "user", "cwd": "/work", "timestamp": "2026-10-30T09:00:00Z",
                        "message": {"role": "user", "content": "start"}}) + "\n"
            + json.dumps({"type": "ai-title", "aiTitle": "Generated title"}) + "\n"
            + json.dumps({"type": "custom-title", "customTitle": "Dot integration"}) + "\n")
        self.clock = [1_800_000_000.0]
        self.receiver = Receiver()
        self.server = SE.Server(self.homes, "tester", SE.CallbackPolicy(), self.receiver, lambda: self.clock[0])
        self.daemon = SE.Daemon(self.homes, SE.CallbackPolicy(), self.receiver, lambda: self.clock[0],
                                log=lambda row: None)

    def rpc(self, method, params=None):
        reply = self.server.handle({"jsonrpc": "2.0", "id": 1, "method": method, "params": params or {}})
        return reply

    def subscribe(self, arguments=None, secret=None, url=URL, **extra):
        self.secret = secret or getattr(self, "secret", None) or whsec()
        params = {"name": SE.EVENT_NAME, "arguments": arguments or {"session_ids": [CODEX_ID], "pattern": "deploy"},
                  "delivery": {"mode": "webhook", "url": url, "secret": self.secret}, "cursor": None, **extra}
        return self.rpc("events/subscribe", params)

    def append(self, path, text):
        with path.open("a") as handle:
            handle.write(text)


class SigningAndRedaction(unittest.TestCase):
    def test_standard_webhooks_vector(self):
        key = base64.b64decode("MfKQ9r8GKYqrTwjUPD8ILPZIo2LaLaSw")
        self.assertEqual(SE.sign([key], "msg_p5jXN8AQM9LWM0D4loKWxJek", 1614265330, b'{"test": 2432232314}'),
                         "v1,g0hM9SsE+OTPJTGt/tmIKtSyZlE3uFJELVlNIOLJ1OE=")

    def test_secret_format(self):
        SE.secret_bytes(whsec())
        for bad in ("abc", "whsec_" + base64.b64encode(b"short").decode(), "whsec_!!!", None):
            with self.assertRaises(SE.RpcError):
                SE.secret_bytes(bad)

    def test_credentials_redacted_and_identifiers_kept(self):
        text = (f"session {CODEX_ID} commit 4a40d51946dbe0a2b1c3 key sk-proj-{'a' * 30} "
                f"token={CLAUDE_ID} password=hunter2hunter2 ghp_{'b' * 36} Authorization: Bearer {'c' * 30}")
        redacted, count = SE.redact(text)
        self.assertEqual(count, 4)
        for kept in (CODEX_ID, CLAUDE_ID, "4a40d51946dbe0a2b1c3", "Bearer "):
            self.assertIn(kept, redacted)
        for gone in ("sk-proj-", "hunter2", "ghp_", "c" * 30):
            self.assertNotIn(gone, redacted)

    def test_match_offsets_survive_redaction(self):
        text = f"x sk-{'k' * 40} then the deploy failed"
        match = SE.re.search("deploy", text)
        entry = SE.Entry("0", 0, 0, "assistant", text, None, None)
        data = SE.event_data(SE.Session("codex", CODEX_ID, Path("/x")), None, entry, match, "excerpt")
        self.assertEqual(data["text"][data["match"]["start"]:data["match"]["end"]], "deploy")
        self.assertEqual(data["redactions"], 1)

    def test_window_grows_over_a_cut_secret(self):
        secret = "sk-" + "z" * 40
        text = "a" * 9 + " " + secret + " " + "b" * 2000 + "MATCH"
        rendered = SE.render(text, 20, len(text), (len(text) - 5, len(text)), SE.secret_spans(text))
        self.assertTrue(rendered.text.startswith("[REDACTED:openai_key]"))
        self.assertNotIn("zzzz", rendered.text)


class Transcripts(Fixture):
    def test_codex_entries(self):
        start = self.codex_path.stat().st_size
        self.append(self.codex_path,
                    codex_line("response_item", type="function_call", name="shell", arguments='{"cmd":"ls"}')
                    + codex_line("response_item", type="function_call_output", output="file.txt")
                    + codex_line("response_item", type="reasoning", summary=[], encrypted_content="x")
                    + codex_line("event_msg", type="token_count")
                    + message("developer", "rules"))
        entries = SE.entries_between(SE.resolve_session(self.homes, CODEX_ID), start)
        self.assertEqual([(e.role, e.tool) for e in entries],
                         [("tool_call", "shell"), ("tool_result", None), ("system", None)])
        self.assertEqual(entries[0].text, 'shell {"cmd":"ls"}')
        self.assertEqual(entries[0].entry_id, str(start))

    def test_claude_entries_and_block_ids(self):
        start = self.claude_path.stat().st_size
        self.append(self.claude_path, json.dumps({"type": "assistant", "timestamp": "2026-10-30T09:01:00Z", "message": {
            "role": "assistant", "content": [{"type": "thinking", "thinking": "hmm"}, {"type": "text", "text": "ok"},
                                             {"type": "tool_use", "name": "Bash", "input": {"command": "ls"}}]}}) + "\n"
                    + json.dumps({"type": "user", "message": {"role": "user", "content": [
                        {"type": "tool_result", "content": [{"type": "text", "text": "out"}]}]}}) + "\n")
        entries = SE.entries_between(SE.resolve_session(self.homes, CLAUDE_ID), start)
        self.assertEqual([(e.entry_id, e.role) for e in entries],
                         [(f"{start}.0", "reasoning"), (f"{start}.1", "assistant"), (f"{start}.2", "tool_call"),
                          (str(entries[3].line_start), "tool_result")])
        self.assertEqual(entries[2].text, 'Bash {"command": "ls"}')

    def test_partial_line_is_not_read(self):
        size = self.codex_path.stat().st_size
        self.append(self.codex_path, message("user", "complete") + '{"type": "response_item", "payl')
        self.assertEqual(len(list(SE.complete_lines(self.codex_path, size))), 1)
        self.assertEqual(SE.tail_line_start(self.codex_path, self.codex_path.stat().st_size),
                         size + len(message("user", "complete")))
        self.assertEqual(SE.tail_line_start(self.claude_path, self.claude_path.stat().st_size),
                         self.claude_path.stat().st_size)

    def test_resolution_errors(self):
        for value, reason in (("not-a-uuid", "invalid_session_id"),
                              ("00000000-0000-0000-0000-000000000000", "unknown_session")):
            with self.assertRaises(SE.SessionError) as caught:
                SE.resolve_session(self.homes, value)
            self.assertEqual(caught.exception.reason, reason)
        archived = self.homes.codex / "archived_sessions"
        archived.mkdir()
        (archived / self.codex_path.name).write_text("")
        with self.assertRaises(SE.SessionError) as caught:
            SE.resolve_session(self.homes, CODEX_ID)
        self.assertEqual(caught.exception.reason, "ambiguous_session")

    def test_list_sessions_by_title(self):
        everything = SE.list_sessions(self.homes)
        self.assertEqual(everything["searched"], 2)
        titles = {row["sessionId"]: row["title"] for row in everything["sessions"]}
        self.assertEqual(titles, {CODEX_ID: "Assess Dot event monitoring", CLAUDE_ID: "Dot integration"})
        self.assertEqual([r["sessionId"] for r in SE.list_sessions(self.homes, "dot monitoring")["sessions"]],
                         [CODEX_ID])
        self.assertEqual(SE.list_sessions(self.homes, "7131be18")["sessions"][0]["cwd"], "/work")
        self.assertEqual(SE.list_sessions(self.homes, "dot", host="claude")["matched"], 1)
        self.append(self.claude_path, json.dumps({"type": "custom-title", "customTitle": "Renamed"}) + "\n")
        self.assertEqual(SE.list_sessions(self.homes, "renamed")["matched"], 1)  # cached scan resumes


class McpSurface(Fixture):
    def test_initialize_and_listings(self):
        result = self.rpc("initialize", {"protocolVersion": "2025-11-25"})["result"]
        self.assertEqual(result["protocolVersion"], "2025-11-25")
        self.assertIn("events", result["capabilities"])
        events = self.rpc("events/list")["result"]["events"]
        self.assertEqual([e["name"] for e in events], [SE.EVENT_NAME, SE.SIMPLE_EVENT_NAME, SE.TURN_EVENT_NAME])
        self.assertEqual(events[0]["delivery"], ["webhook"])
        tools = {t["name"]: t for t in self.rpc("tools/list")["result"]["tools"]}
        self.assertEqual(set(tools), {"list_sessions", "read_session", "test_pattern"})
        self.assertTrue(all(t["annotations"]["readOnlyHint"] for t in tools.values()))
        self.assertIsNone(self.server.handle({"jsonrpc": "2.0", "method": "notifications/initialized"}))
        self.assertEqual(self.rpc("nope")["error"]["code"], -32601)

    def test_stdio_framing(self):
        out = io.StringIO()
        SE.serve(self.server, io.StringIO('{"jsonrpc":"2.0","id":7,"method":"ping"}\nnot json\n'
                                          '{"jsonrpc":"2.0","method":"notifications/initialized"}\n'), out)
        replies = [json.loads(line) for line in out.getvalue().splitlines()]
        self.assertEqual(replies, [{"jsonrpc": "2.0", "id": 7, "result": {}},
                                   {"jsonrpc": "2.0", "id": None, "error": {"code": -32700, "message": "Parse error"}}])

    def test_request_log_omits_secret_and_callback_path(self):
        secret = whsec()
        request = {"jsonrpc": "2.0", "id": 3, "method": "events/subscribe", "params": {
            "name": SE.EVENT_NAME, "arguments": {"session_ids": [CODEX_ID], "pattern": "deploy"},
            "delivery": {"mode": "webhook", "url": URL, "secret": secret}, "ttlMs": 60000}}
        lines = '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-11-25",' \
                '"clientInfo":{"name":"chatgpt","version":"9"}}}\n' + json.dumps(request) + "\n"
        SE.serve(self.server, io.StringIO(lines), io.StringIO())
        log = (self.homes.state / "requests.log").read_text()
        rows = [json.loads(line) for line in log.splitlines()]
        self.assertEqual([r.get("method") for r in rows], [None, "initialize", "events/subscribe"])
        self.assertEqual(rows[1]["client"], "chatgpt/9")
        self.assertEqual((rows[2]["outcome"], rows[2]["callbackHost"], rows[2]["ttlMs"]), ("ok", "hooks.example.com", 60000))
        self.assertNotIn(secret[6:], log)
        self.assertNotIn("/mcp/abc", log)

    def test_subscribe_verifies_and_is_idempotent(self):
        first = self.subscribe()["result"]
        self.assertTrue(first["id"].startswith("sub_"))
        self.assertFalse(first["truncated"])
        verification = json.loads(self.receiver.calls[0][1])
        headers = self.receiver.calls[0][2]
        self.assertEqual(verification["type"], "verification")
        self.assertTrue(headers["webhook-id"].startswith("msg_verification_"))
        self.assertEqual(headers["X-MCP-Subscription-Id"], first["id"])
        self.assertTrue(verify(self.secret, headers, self.receiver.calls[0][1]))
        self.assertEqual(SE.decode_cursor(first["cursor"]), {CODEX_ID: self.codex_path.stat().st_size})
        self.clock[0] += 60
        second = self.subscribe(arguments={"pattern": "deploy", "session_ids": [CODEX_ID]})["result"]
        self.assertEqual(second["id"], first["id"])  # canonical JSON: key order does not matter
        self.assertGreater(second["refreshBefore"], first["refreshBefore"])
        self.assertEqual(len(self.receiver.calls), 1)  # verification cached for (principal, url)
        record = self.server.store.subscription(first["id"])
        self.assertEqual(record["generation"], 2)

    def test_ttl_grants(self):
        def granted(**ttl):
            result = self.subscribe(**ttl)["result"]
            return SE.parse_time(result["refreshBefore"]) - self.clock[0]
        self.assertAlmostEqual(granted(), SE.DEFAULT_TTL_MS / 1000, places=0)
        self.assertAlmostEqual(granted(ttlMs=1000), SE.MIN_TTL_MS / 1000, places=0)
        self.assertAlmostEqual(granted(ttlMs=None), SE.MAX_TTL_MS / 1000, places=0)

    def test_rejections(self):
        def code(**overrides):
            params = {"name": SE.EVENT_NAME, "arguments": {"session_ids": [CODEX_ID], "pattern": "x"},
                      "delivery": {"mode": "webhook", "url": URL, "secret": whsec()}}
            params.update(overrides)
            reply = self.rpc("events/subscribe", params)
            return reply["error"]["code"], (reply["error"].get("data") or {}).get("reason")
        self.assertEqual(code(name="other.event"), (SE.NOT_FOUND, None))
        self.assertEqual(code(delivery={"mode": "webhook", "url": "http://hooks.example.com/x", "secret": whsec()}),
                         (SE.INVALID_PARAMS, "callback_url"))
        self.assertEqual(code(delivery={"mode": "webhook", "url": URL, "secret": "whsec_abc"}),
                         (SE.INVALID_PARAMS, "secret_format"))
        self.assertEqual(code(delivery={"mode": "push", "url": URL, "secret": whsec()})[0], SE.UNSUPPORTED)
        self.assertEqual(code(arguments={"session_ids": ["00000000-0000-0000-0000-000000000000"], "pattern": "x"}),
                         (SE.INVALID_PARAMS, "unknown_session"))
        self.assertEqual(code(arguments={"session_ids": [CODEX_ID], "pattern": "("}),
                         (SE.INVALID_PARAMS, "pattern_syntax"))
        self.assertEqual(code(arguments={"session_ids": [CODEX_ID], "pattern": "x", "extra": 1}),
                         (SE.INVALID_PARAMS, "unknown_arguments"))
        self.assertEqual(code(arguments={"session_ids": [CODEX_ID], "pattern": "x", "roles": ["bot"]}),
                         (SE.INVALID_PARAMS, "roles"))
        self.receiver.verify_status = 404
        self.assertEqual(code(), (SE.CALLBACK_ENDPOINT_ERROR, "http_4xx"))
        self.assertEqual(self.server.store.all_subscriptions(), [])

    def test_challenge_must_echo(self):
        self.server.transport = lambda url, body, headers, policy: SE.Outcome(200, None, b'{"challenge":"wrong"}')
        self.assertEqual(self.subscribe()["error"]["data"]["reason"], "challenge_failed")

    def test_unsubscribe_and_revoke(self):
        identifier = self.subscribe()["result"]["id"]
        params = {"name": SE.EVENT_NAME, "arguments": {"session_ids": [CODEX_ID], "pattern": "deploy"},
                  "delivery": {"url": URL}}
        self.assertEqual(self.rpc("events/unsubscribe", params)["result"], {})
        self.assertEqual(self.rpc("events/unsubscribe", params)["result"], {})  # idempotent
        self.assertIsNone(self.server.store.subscription(identifier))
        SE.main(["--state-dir", str(self.homes.state), "revoke", "--id", identifier])
        self.assertEqual(self.subscribe()["error"]["code"], SE.FORBIDDEN)
        SE.main(["--state-dir", str(self.homes.state), "revoke", "--id", identifier, "--allow"])
        self.assertIn("result", self.subscribe())

    def test_tools(self):
        def call(name, **arguments):
            return self.rpc("tools/call", {"name": name, "arguments": arguments})["result"]
        sessions = call("list_sessions", query="monitoring")["structuredContent"]["sessions"]
        self.assertEqual([s["sessionId"] for s in sessions], [CODEX_ID])
        self.append(self.codex_path, message("assistant", "the deploy is done"))
        matches = call("test_pattern", session_ids=[CODEX_ID], pattern="deploy")["structuredContent"]["matches"]
        self.assertEqual([m["text"] for m in matches], ["the deploy is done"])
        entry_id = matches[0]["entryId"]
        read = call("read_session", session_id=CODEX_ID, entry_id=entry_id, before=2)["structuredContent"]
        self.assertEqual([e["text"] for e in read["entries"]], ["hello there", "first reply", "the deploy is done"])
        tail = call("read_session", session_id=CODEX_ID, tail=1)["structuredContent"]["entries"]
        self.assertEqual(tail[0]["entryId"], entry_id)
        self.assertTrue(call("read_session", session_id=CODEX_ID, entry_id="5")["isError"])
        self.assertTrue(call("read_session", session_id="bad")["isError"])


class Delivery(Fixture):
    def test_delivers_matches_in_order_with_watermark_cursor(self):
        identifier = self.subscribe()["result"]["id"]
        before = self.codex_path.stat().st_size
        first = message("assistant", "deploy started")
        self.append(self.codex_path, message("user", "unrelated") + first + message("user", "deploy finished?")
                    + '{"partial')
        self.daemon.tick()
        events = self.receiver.events()
        self.assertEqual([e["data"]["text"] for e, _ in events], ["deploy started", "deploy finished?"])
        body, headers = events[0]
        self.assertEqual(headers["X-MCP-Subscription-Id"], identifier)
        self.assertEqual(headers["webhook-id"], body["eventId"])
        self.assertTrue(verify(self.secret, headers, self.receiver.calls[1][1]))
        start_of_first = before + len(message("user", "unrelated"))
        self.assertEqual(SE.decode_cursor(body["cursor"]), {CODEX_ID: start_of_first})
        self.assertEqual(body["data"]["sessionTitle"], "Assess Dot event monitoring")
        self.assertEqual(body["timestamp"], "2026-10-30T10:00:00.000Z")
        self.append(self.codex_path, '": 1}\n' + message("assistant", "deploy ok"))
        self.daemon.tick()
        self.assertEqual(self.receiver.events()[-1][0]["data"]["text"], "deploy ok")  # partial line skipped as invalid
        refreshed = self.subscribe()["result"]
        self.assertEqual(SE.decode_cursor(refreshed["cursor"]), {CODEX_ID: self.codex_path.stat().st_size})
        self.assertEqual(refreshed["deliveryStatus"]["lastError"], None)

    def test_role_filter_and_case(self):
        self.subscribe({"session_ids": [CODEX_ID], "pattern": "DEPLOY", "case_insensitive": True,
                        "roles": ["tool_result"]})
        self.append(self.codex_path, message("assistant", "deploy")
                    + codex_line("response_item", type="function_call_output", output="deploy log"))
        self.daemon.tick()
        self.assertEqual([e["data"]["role"] for e, _ in self.receiver.events()], ["tool_result"])

    def test_retry_keeps_event_id_then_succeeds(self):
        self.subscribe()
        self.append(self.codex_path, message("assistant", "deploy one") + message("assistant", "deploy two"))
        self.receiver.statuses = [500, 503]
        wake = self.daemon.tick()
        self.assertEqual(wake, self.clock[0] + SE.RETRY_DELAYS[0])
        self.daemon.tick()  # before the backoff: nothing sent
        self.assertEqual(len(self.receiver.events()), 1)
        self.clock[0] = wake
        self.daemon.tick()
        self.clock[0] += SE.RETRY_DELAYS[1]
        self.daemon.tick()
        ids = [e["eventId"] for e, _ in self.receiver.events()]
        texts = [e["data"]["text"] for e, _ in self.receiver.events()]
        self.assertEqual(ids[0], ids[1])
        self.assertEqual(ids[1], ids[2])
        self.assertEqual(texts, ["deploy one"] * 3 + ["deploy two"])
        attempts = [h["webhook-timestamp"] for _, h in self.receiver.events()]
        self.assertNotEqual(attempts[0], attempts[2])  # each attempt is re-signed

    def test_gone_is_not_retried(self):
        self.subscribe()
        self.append(self.codex_path, message("assistant", "deploy one") + message("assistant", "deploy two"))
        self.receiver.statuses = [410]
        self.daemon.tick()
        self.assertEqual([e["data"]["text"] for e, _ in self.receiver.events()], ["deploy one", "deploy two"])

    def test_abandon_suspend_and_reactivate_on_refresh(self):
        identifier = self.subscribe()["result"]["id"]
        self.append(self.codex_path, "".join(message("assistant", f"deploy {n}") for n in range(4)))
        self.receiver.statuses = [500] * 100
        for _ in range(40):
            wake = self.daemon.tick()
            if not self.server.store.state(identifier)["active"]:
                break
            self.clock[0] = max(self.clock[0], wake)
        state = self.server.store.state(identifier)
        self.assertFalse(state["active"])
        self.assertEqual(state["abandoned"], SE.SUSPEND_AFTER_ABANDONED)
        self.assertEqual(state["lastError"], "http_5xx")
        self.assertEqual(len(self.receiver.events()), SE.SUSPEND_AFTER_ABANDONED * (len(SE.RETRY_DELAYS) + 1))
        refreshed = self.subscribe()["result"]
        self.assertFalse(refreshed["deliveryStatus"]["active"])
        self.receiver.statuses = []
        self.daemon.tick()
        self.assertEqual(self.receiver.events()[-1][0]["data"]["text"], "deploy 3")
        self.assertTrue(self.server.store.state(identifier)["active"])

    def test_cursor_replay_on_new_subscription(self):
        first = self.subscribe()["result"]
        self.append(self.codex_path, message("assistant", "deploy missed"))
        other = self.subscribe(url="https://hooks.example.com/other", cursor=first["cursor"])["result"]
        self.assertFalse(other["truncated"])
        self.server.store.delete(first["id"])
        self.daemon.tick()
        self.assertEqual([e["data"]["text"] for e, _ in self.receiver.events()], ["deploy missed"])
        stale = self.subscribe(url="https://hooks.example.com/third", cursor="garbage")["result"]
        self.assertTrue(stale["truncated"])

    def test_expired_subscription_is_not_delivered_then_removed(self):
        identifier = self.subscribe(ttlMs=SE.MIN_TTL_MS)["result"]["id"]
        self.append(self.codex_path, message("assistant", "deploy"))
        self.clock[0] += SE.MIN_TTL_MS / 1000 + 1
        self.daemon.tick()
        self.assertEqual(self.receiver.events(), [])
        self.clock[0] += SE.EXPIRED_RETENTION_SECONDS
        self.daemon.tick()
        self.assertIsNone(self.server.store.subscription(identifier))

    def test_full_content_fits_the_body_limit(self):
        self.subscribe({"session_ids": [CODEX_ID], "pattern": "NEEDLE", "content": "full"})
        self.append(self.codex_path, message("assistant", "x" * 150_000 + "NEEDLE" + "é" * 150_000))
        self.daemon.tick()
        body = self.receiver.calls[-1][1]
        data = json.loads(body)["data"]
        self.assertLessEqual(len(body), SE.MAX_BODY_BYTES)
        self.assertTrue(data["textTruncated"])
        self.assertEqual(data["text"][data["match"]["start"]:data["match"]["end"]], "NEEDLE")
        self.subscribe({"session_ids": [CODEX_ID], "pattern": "small", "content": "full"})
        self.append(self.codex_path, message("assistant", "a small entry"))
        self.daemon.tick()
        self.assertFalse(json.loads(self.receiver.calls[-1][1])["data"]["textTruncated"])

    def test_multi_session_and_claude_blocks(self):
        self.subscribe({"session_ids": [CODEX_ID, CLAUDE_ID], "pattern": "deploy", "roles": ["assistant", "tool_call"]})
        self.append(self.claude_path, json.dumps({"type": "assistant", "message": {"role": "assistant", "content": [
            {"type": "text", "text": "will deploy"}, {"type": "tool_use", "name": "Bash", "input": {"c": "deploy"}}]}})
                    + "\n")
        self.append(self.codex_path, message("assistant", "deploy codex"))
        self.daemon.tick()
        rows = [(e["data"]["host"], e["data"]["role"]) for e, _ in self.receiver.events()]
        self.assertEqual(rows, [("codex", "assistant"), ("claude", "assistant"), ("claude", "tool_call")])
        cursor = SE.decode_cursor(self.receiver.events()[-1][0]["cursor"])
        self.assertEqual(set(cursor), {CODEX_ID, CLAUDE_ID})


class DaemonRobustness(Fixture):
    def test_one_failing_subscription_does_not_stop_others(self):
        self.subscribe({"session_ids": [CLAUDE_ID], "pattern": "deploy"})
        self.subscribe({"session_ids": [CODEX_ID], "pattern": "deploy"})
        logged = []
        self.daemon.log = logged.append
        original = self.daemon._deliver

        def flaky(subscription, *args):
            if subscription["sessions"][0]["sessionId"] == CLAUDE_ID:
                raise PermissionError("unreadable")
            return original(subscription, *args)

        self.daemon._deliver = flaky
        self.append(self.codex_path, message("assistant", "deploy"))
        self.daemon.tick()
        self.assertEqual([e["data"]["sessionId"] for e, _ in self.receiver.events()], [CODEX_ID])
        self.assertEqual([row["event"] for row in logged if row["event"] == "subscription_error"],
                         ["subscription_error"])

    def test_unchanged_state_is_not_rewritten(self):
        identifier = self.subscribe()["result"]["id"]
        self.daemon.tick()
        path = self.homes.state / "delivery" / f"{identifier}.json"
        first = path.stat().st_mtime_ns
        time.sleep(0.01)
        self.append(self.codex_path, message("user", "no match here"))
        self.daemon.tick()
        moved = path.stat().st_mtime_ns
        self.assertNotEqual(first, moved)  # position advanced
        time.sleep(0.01)
        self.daemon.tick()
        self.assertEqual(path.stat().st_mtime_ns, moved)


class SimpleEvent(Fixture):
    def test_schemas_follow_openai_example_shape(self):
        for schema in (SE.SIMPLE_EVENT_DESCRIPTOR["inputSchema"], SE.SIMPLE_EVENT_DESCRIPTOR["payloadSchema"]):
            self.assertEqual(set(schema["required"]), set(schema["properties"]))
            self.assertIs(schema["additionalProperties"], False)
            for prop in schema["properties"].values():
                self.assertEqual(set(prop) - {"description"}, {"type"})
                self.assertEqual(prop["type"], "string")

    def subscribe_simple(self, arguments):
        self.secret = whsec()
        return self.rpc("events/subscribe", {
            "name": SE.SIMPLE_EVENT_NAME, "arguments": arguments,
            "delivery": {"mode": "webhook", "url": URL, "secret": self.secret}, "cursor": None})

    def test_delivers_payload_matching_its_schema(self):
        identifier = self.subscribe_simple({"session_id": CODEX_ID, "pattern": "PING-TEST"})["result"]["id"]
        self.append(self.codex_path, message("user", "say PING-TEST now") + message("developer", "PING-TEST rules"))
        self.daemon.tick()
        (body, headers), = self.receiver.events()
        self.assertEqual(body["name"], SE.SIMPLE_EVENT_NAME)
        self.assertEqual(headers["X-MCP-Subscription-Id"], identifier)
        schema = SE.SIMPLE_EVENT_DESCRIPTOR["payloadSchema"]
        self.assertEqual(set(body["data"]), set(schema["required"]))
        self.assertTrue(all(isinstance(v, str) for v in body["data"].values()))
        self.assertEqual((body["data"]["text"], body["data"]["match"], body["data"]["role"]),
                         ("say PING-TEST now", "PING-TEST", "user"))
        self.assertEqual(body["data"]["session_title"], "Assess Dot event monitoring")

    def test_rejects_extra_or_missing_arguments(self):
        extra = self.subscribe_simple({"session_id": CODEX_ID, "pattern": "x", "schedule": ""})
        self.assertEqual(extra["error"]["data"], {"reason": "unknown_arguments", "fields": ["schedule"]})
        self.assertEqual(self.subscribe_simple({"pattern": "x"})["error"]["data"]["reason"], "session_id")


class HttpTransport(Fixture):
    def setUp(self):
        super().setUp()
        self.httpd = http.server.ThreadingHTTPServer(("127.0.0.1", 0), SE.make_http_handler(self.server, "/mcp"))
        threading.Thread(target=self.httpd.serve_forever, daemon=True).start()
        self.addCleanup(self.httpd.server_close)
        self.addCleanup(self.httpd.shutdown)
        self.port = self.httpd.server_address[1]

    def post(self, payload, path="/mcp", headers=None, method="POST"):
        connection = http.client.HTTPConnection("127.0.0.1", self.port, timeout=5)
        body = payload if isinstance(payload, bytes) else json.dumps(payload).encode()
        sent = {"Content-Type": "application/json", "Accept": "application/json, text/event-stream"}
        sent.update(headers or {})
        connection.request(method, path, body=body if method == "POST" else None, headers=sent)
        response = connection.getresponse()
        data = response.read()
        connection.close()
        return response.status, (json.loads(data) if data else None)

    def test_json_rpc_over_post(self):
        status, reply = self.post({"jsonrpc": "2.0", "id": 1, "method": "initialize",
                                   "params": {"protocolVersion": "2025-11-25"}})
        self.assertEqual(status, 200)
        self.assertIn("events", reply["result"]["capabilities"])
        self.assertEqual(self.post({"jsonrpc": "2.0", "method": "notifications/initialized"}), (202, None))
        # no handshake needed before a call: each request stands alone
        status, reply = self.post({"jsonrpc": "2.0", "id": 2, "method": "events/subscribe", "params": {
            "name": SE.SIMPLE_EVENT_NAME, "arguments": {"session_id": CODEX_ID, "pattern": "x"},
            "delivery": {"mode": "webhook", "url": URL, "secret": whsec()}}})
        self.assertEqual(status, 200)
        self.assertTrue(reply["result"]["id"].startswith("sub_"))
        rows = [json.loads(line) for line in (self.homes.state / "requests.log").read_text().splitlines()]
        self.assertEqual([r.get("transport") for r in rows], ["http"] * 3)

    def test_refusals(self):
        ping = {"jsonrpc": "2.0", "id": 1, "method": "ping"}
        self.assertEqual(self.post(ping, headers={"Origin": "https://evil.example"})[0], 403)
        self.assertEqual(self.post(ping, headers={"Host": "evil.example:80"})[0], 403)
        self.assertEqual(self.post(ping, path="/other")[0], 404)
        self.assertEqual(self.post(ping, headers={"Content-Type": "text/plain"})[0], 415)
        self.assertEqual(self.post(ping, method="GET")[0], 405)
        self.assertEqual(self.post(b"not json")[1]["error"]["code"], -32700)

    def test_listen_must_be_loopback(self):
        with self.assertRaises(SystemExit):
            SE.serve_http(self.server, "0.0.0.0:0")


def task_complete(message_text, turn_id="turn-1", duration="61000"):
    return codex_line("event_msg", type="task_complete", turn_id=turn_id, last_agent_message=message_text,
                      duration_ms=duration, completed_at="1793331560")


class TurnEnded(Fixture):
    def subscribe_turns(self, session_ids):
        self.secret = whsec()
        return self.rpc("events/subscribe", {
            "name": SE.TURN_EVENT_NAME, "arguments": {"session_ids": session_ids},
            "delivery": {"mode": "webhook", "url": URL, "secret": self.secret}, "cursor": None})

    def test_codex_turn_ends_are_entries(self):
        start = self.codex_path.stat().st_size
        self.append(self.codex_path, task_complete("All done.")
                    + codex_line("event_msg", type="turn_aborted", turn_id="turn-2", reason="interrupted")
                    + codex_line("event_msg", type="token_count"))
        entries = SE.entries_between(SE.resolve_session(self.homes, CODEX_ID), start)
        self.assertEqual([(e.role, e.text, e.meta["status"]) for e in entries],
                         [("turn_end", "All done.", "completed"), ("turn_end", "(turn aborted: interrupted)", "interrupted")])

    def test_claude_final_text_ends_the_turn(self):
        start = self.claude_path.stat().st_size
        self.append(self.claude_path, json.dumps({"type": "assistant", "message": {
            "id": "msg_1", "role": "assistant", "stop_reason": "tool_use",
            "content": [{"type": "text", "text": "working"}]}}) + "\n" + json.dumps({"type": "assistant", "message": {
            "id": "msg_2", "role": "assistant", "stop_reason": "end_turn",
            "content": [{"type": "text", "text": "Finished."}]}}) + "\n")
        entries = SE.entries_between(SE.resolve_session(self.homes, CLAUDE_ID), start)
        self.assertEqual([(e.role, e.text) for e in entries],
                         [("assistant", "working"), ("assistant", "Finished."), ("turn_end", "Finished.")])
        self.assertEqual(entries[2].meta["turn_id"], "msg_2")

    def test_delivers_each_turn_end_with_schema_payload(self):
        self.subscribe_turns([CODEX_ID, CLAUDE_ID])
        self.append(self.codex_path, message("assistant", "progress") + task_complete("Built and tested."))
        self.append(self.codex_path, codex_line("event_msg", type="turn_aborted", turn_id="turn-2", reason="interrupted"))
        self.daemon.tick()
        bodies = [body for body, _ in self.receiver.events()]
        self.assertEqual([b["name"] for b in bodies], [SE.TURN_EVENT_NAME] * 2)
        schema = SE.TURN_EVENT_DESCRIPTOR["payloadSchema"]
        for body in bodies:
            self.assertEqual(set(body["data"]), set(schema["required"]))
        first, second = (b["data"] for b in bodies)
        self.assertEqual((first["status"], first["final_message"], first["duration_ms"], first["turn_id"]),
                         ("completed", "Built and tested.", 61000, "turn-1"))
        self.assertFalse(first["final_message_truncated"])
        self.assertEqual((second["status"], second["reason"], second["duration_ms"], second["final_message"]),
                         ("interrupted", "interrupted", -1, ""))
        self.assertEqual(first["session_title"], "Assess Dot event monitoring")
        self.assertEqual([b["data"]["event_id"] for b in bodies], [b["eventId"] for b in bodies])

    def test_status_without_final_message_and_unknown_abort(self):
        self.subscribe_turns([CODEX_ID])
        self.append(self.codex_path, task_complete("") + codex_line("event_msg", type="turn_aborted", reason="crashed"))
        self.daemon.tick()
        rows = [(b["data"]["status"], b["data"]["reason"], b["data"]["final_message"]) for b, _ in self.receiver.events()]
        self.assertEqual(rows, [("completed", "", ""), ("failed", "crashed", "")])

    def test_long_final_message_is_capped(self):
        self.subscribe_turns([CODEX_ID])
        self.append(self.codex_path, task_complete("x" * 5000 + " sk-" + "s" * 40))
        self.daemon.tick()
        data = self.receiver.events()[0][0]["data"]
        self.assertEqual(len(data["final_message"]), SE.MAX_FINAL_MESSAGE_CHARS)
        self.assertTrue(data["final_message_truncated"])

    def test_schema_and_arguments(self):
        schema = SE.TURN_EVENT_DESCRIPTOR["inputSchema"]
        self.assertEqual(schema["required"], ["session_ids"])
        self.assertIs(SE.TURN_EVENT_DESCRIPTOR["payloadSchema"]["additionalProperties"], False)
        reply = self.rpc("events/subscribe", {"name": SE.TURN_EVENT_NAME,
                                              "arguments": {"session_ids": [CODEX_ID], "pattern": "x"},
                                              "delivery": {"mode": "webhook", "url": URL, "secret": whsec()}})
        self.assertEqual(reply["error"]["data"], {"reason": "unknown_arguments", "fields": ["pattern"]})


class LocalPull(Fixture):
    def setUp(self):
        super().setUp()
        self.local = SE.LocalSubscriptions(self.homes)

    def make(self, **extra):
        arguments = {"session_ids": [CODEX_ID, CLAUDE_ID], "pattern": "PING-TEST", **extra}
        return self.local.create("ping", arguments)

    def test_returns_each_match_once_from_creation_onward(self):
        self.append(self.codex_path, message("user", "PING-TEST before subscribing"))
        created = self.make()
        self.assertEqual(SE.decode_cursor(created["cursor"])[CODEX_ID], self.codex_path.stat().st_size)
        self.assertEqual(self.local.next("ping")["status"], "timeout")
        self.append(self.codex_path, message("user", "first PING-TEST") + message("user", "no match"))
        self.append(self.claude_path, json.dumps({"type": "user", "message": {"role": "user",
                                                                              "content": "second PING-TEST"}}) + "\n")
        result = self.local.next("ping")
        self.assertEqual(result["status"], "events")
        self.assertEqual([(e["data"]["host"], e["data"]["text"]) for e in result["events"]],
                         [("codex", "first PING-TEST"), ("claude", "second PING-TEST")])
        self.assertEqual(result["events"][0]["data"]["sessionTitle"], "Assess Dot event monitoring")
        self.assertEqual(self.local.next("ping")["events"], [])

    def test_limit_peek_and_replay(self):
        start = self.make()["cursor"]
        self.append(self.codex_path, "".join(message("user", f"PING-TEST {n}") for n in range(3)))
        peeked = self.local.next("ping", peek=True)
        self.assertEqual(len(peeked["events"]), 3)
        first = self.local.next("ping", limit=2)
        self.assertEqual([e["data"]["text"] for e in first["events"]], ["PING-TEST 0", "PING-TEST 1"])
        self.assertEqual([e["data"]["text"] for e in self.local.next("ping")["events"]], ["PING-TEST 2"])
        replay = self.local.next("ping", from_cursor=first["events"][1]["cursor"])
        self.assertEqual([e["data"]["text"] for e in replay["events"]], ["PING-TEST 1", "PING-TEST 2"])
        self.assertEqual(len(self.local.next("ping", from_cursor=start)["events"]), 3)

    def test_limit_stops_inside_a_multi_block_line(self):
        self.make(roles=["assistant", "tool_call"])
        self.append(self.claude_path, json.dumps({"type": "assistant", "message": {"role": "assistant", "content": [
            {"type": "text", "text": "PING-TEST a"}, {"type": "tool_use", "name": "Bash", "input": {"c": "PING-TEST b"}}]}})
                    + "\n")
        self.assertEqual([e["data"]["role"] for e in self.local.next("ping", limit=1)["events"]], ["assistant"])
        self.assertEqual([e["data"]["role"] for e in self.local.next("ping", limit=1)["events"]], ["tool_call"])

    def test_wait_wakes_on_append(self):
        self.make()

        def later():
            time.sleep(0.3)
            self.append(self.codex_path, message("user", "PING-TEST while waiting"))

        threading.Thread(target=later, daemon=True).start()
        started = time.monotonic()
        result = self.local.next("ping", wait=5)
        self.assertEqual([e["data"]["text"] for e in result["events"]], ["PING-TEST while waiting"])
        self.assertLess(time.monotonic() - started, 2.0)
        self.assertEqual(self.local.next("ping", wait=0.2)["status"], "timeout")

    def test_names_and_lifecycle_via_cli(self):
        base = ["--codex-home", str(self.homes.codex), "--claude-home", str(self.homes.claude),
                "--state-dir", str(self.homes.state)]
        out = io.StringIO()
        with contextlib.redirect_stdout(out):
            self.assertEqual(SE.main(base + ["subscribe", "--name", "ping", "--session", CODEX_ID,
                                             "--pattern", "PING-TEST", "--role", "user"]), 0)
        with contextlib.redirect_stderr(io.StringIO()):
            self.assertEqual(SE.main(base + ["subscribe", "--name", "ping", "--session", CODEX_ID,
                                             "--pattern", "x"]), 2)  # exists
            self.assertEqual(SE.main(base + ["subscribe", "--name", "Bad Name", "--session", CODEX_ID,
                                             "--pattern", "x"]), 2)
            self.assertEqual(SE.main(base + ["next", "--name", "missing"]), 2)
        self.append(self.codex_path, message("user", "PING-TEST via cli"))
        out = io.StringIO()
        with contextlib.redirect_stdout(out):
            SE.main(base + ["next", "--name", "ping", "--wait", "1"])
        self.assertEqual(json.loads(out.getvalue())["events"][0]["data"]["text"], "PING-TEST via cli")
        out = io.StringIO()
        with contextlib.redirect_stdout(out):
            SE.main(base + ["subscriptions"])
            SE.main(base + ["unsubscribe", "--name", "ping"])
        self.assertIn('"removed": true', out.getvalue())
        self.assertEqual(self.local.all(), [])


class CallbackSafety(unittest.TestCase):
    def test_public_address(self):
        for address, public in (("8.8.8.8", True), ("10.0.0.1", False), ("127.0.0.1", False), ("169.254.1.1", False),
                                ("::1", False), ("::ffff:127.0.0.1", False), ("fe80::1", False), ("224.0.0.1", False)):
            self.assertEqual(SE.public_address(address), public, address)

    def test_private_destination_is_refused_without_connecting(self):
        outcome = SE.post("https://127.0.0.1:9/x", b"{}", {}, SE.CallbackPolicy())
        self.assertEqual(outcome.category, "connection_refused")
        with self.assertRaises(SE.RpcError):
            SE.post("http://example.com/x", b"{}", {}, SE.CallbackPolicy())
        with self.assertRaises(SE.RpcError):
            SE.post("https://user:pw@example.com/x", b"{}", {}, SE.CallbackPolicy())


class RealHttp(Fixture):
    """End to end over a loopback HTTP server, allowed only by the test-only policy."""

    def test_verification_and_delivery_over_http(self):
        received, redirect = [], {"on": False}

        class Handler(http.server.BaseHTTPRequestHandler):
            def do_POST(self):
                body = self.rfile.read(int(self.headers["Content-Length"]))
                received.append((dict(self.headers), body))
                if redirect["on"]:
                    self.send_response(302)
                    self.send_header("Location", "http://127.0.0.1:1/")
                    self.end_headers()
                    return
                parsed = json.loads(body)
                reply = json.dumps({"challenge": parsed["challenge"]}).encode() if "challenge" in parsed else b"{}"
                self.send_response(200)
                self.send_header("Content-Length", str(len(reply)))
                self.end_headers()
                self.wfile.write(reply)

            def log_message(self, *args):
                pass

        httpd = http.server.ThreadingHTTPServer(("127.0.0.1", 0), Handler)
        threading.Thread(target=httpd.serve_forever, daemon=True).start()
        self.addCleanup(httpd.server_close)
        self.addCleanup(httpd.shutdown)
        url = f"http://127.0.0.1:{httpd.server_address[1]}/hook?x=1"
        policy = SE.CallbackPolicy(allow_private=True, timeout=5)
        server = SE.Server(self.homes, "tester", policy, SE.post, lambda: self.clock[0])
        daemon = SE.Daemon(self.homes, policy, SE.post, lambda: self.clock[0], log=lambda row: None)
        secret = whsec()
        reply = server.handle({"jsonrpc": "2.0", "id": 1, "method": "events/subscribe", "params": {
            "name": SE.EVENT_NAME, "arguments": {"session_ids": [CODEX_ID], "pattern": "deploy"},
            "delivery": {"mode": "webhook", "url": url, "secret": secret}}})
        self.assertIn("result", reply)
        self.append(self.codex_path, message("assistant", "deploy now"))
        daemon.tick()
        headers, body = received[-1]
        self.assertEqual(json.loads(body)["data"]["text"], "deploy now")
        lower = {k.lower(): v for k, v in headers.items()}
        self.assertTrue(verify(secret, lower, body))
        redirect["on"] = True
        self.append(self.codex_path, message("assistant", "deploy again"))
        daemon.tick()
        self.assertEqual(server.store.state(reply["result"]["id"])["lastError"], "http_4xx")  # 3xx not followed


class InotifyWake(Fixture):
    def test_daemon_wakes_on_append(self):
        self.subscribe()
        self.daemon.clock = time.time
        self.server.clock = time.time
        thread = threading.Thread(target=SE.run_daemon, args=(self.daemon, None, 3.0), daemon=True)
        thread.start()
        time.sleep(0.5)
        started = time.monotonic()
        self.append(self.codex_path, message("assistant", "deploy via inotify"))
        while not self.receiver.events() and time.monotonic() - started < 2.0:
            time.sleep(0.02)
        latency = time.monotonic() - started
        thread.join()
        self.assertEqual([e["data"]["text"] for e, _ in self.receiver.events()], ["deploy via inotify"])
        self.assertLess(latency, 1.0)


if __name__ == "__main__":
    unittest.main()
