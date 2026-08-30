from __future__ import annotations

import importlib.util
import http.client
import json
from pathlib import Path
import sys
import threading
import unittest


SCRIPT = Path(__file__).parents[1] / "scripts" / "claude_batch_loopback_proxy.py"
sys.path.insert(0, str(SCRIPT.parent))
SPEC = importlib.util.spec_from_file_location("claude_batch_loopback_proxy", SCRIPT)
assert SPEC and SPEC.loader
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)


def parse_sse(payload: bytes) -> list[tuple[str, dict[str, object]]]:
    events = []
    for frame in payload.decode().strip().split("\n\n"):
        lines = frame.splitlines()
        events.append((lines[0].removeprefix("event: "), json.loads(lines[1][6:])))
    return events


class ClaudeBatchLoopbackProxyTest(unittest.TestCase):
    def setUp(self) -> None:
        self.usage = {
            "input_tokens": 40,
            "output_tokens": 18,
            "output_tokens_details": {"thinking_tokens": 4},
            "cache_creation_input_tokens": 0,
            "cache_read_input_tokens": 0,
            "service_tier": "batch",
        }

    def test_batch_envelope_changes_only_transport_fields(self) -> None:
        incoming = {
            "model": "claude-sonnet-5",
            "max_tokens": 128,
            "stream": True,
            "system": [{"type": "text", "text": "system"}],
            "messages": [{"role": "user", "content": "hello"}],
            "tools": [{"name": "Read", "input_schema": {"type": "object"}}],
        }
        payload = MODULE.prepare_batch_request(
            incoming,
            batch_model="anthropic/claude-sonnet-5:batch",
            custom_id="turn-1",
        )
        self.assertEqual(list(payload), ["model", "endpoint", "requests"])
        self.assertEqual(payload["endpoint"], "/v1/messages")
        body = payload["requests"][0]["body"]
        self.assertEqual(body["model"], "anthropic/claude-sonnet-5:batch")
        self.assertFalse(body["stream"])
        for field in ("max_tokens", "system", "messages", "tools"):
            self.assertEqual(body[field], incoming[field])
        self.assertTrue(incoming["stream"])

    def test_request_profile_contains_counts_but_no_tool_content(self) -> None:
        profile = MODULE.request_profile({
            "context_management": {"edits": []},
            "tools": [
                {"name": "Read", "description": "sensitive description"},
                {"name": "Search", "defer_loading": True},
            ],
        })
        self.assertEqual(profile["tool_count"], 2)
        self.assertEqual(profile["deferred_tool_count"], 1)
        self.assertTrue(profile["context_management_present"])
        self.assertGreater(profile["tool_schema_bytes"], 0)
        self.assertNotIn("sensitive", json.dumps(profile))

    def test_synthesizes_text_tool_and_thinking_blocks_in_order(self) -> None:
        message = {
            "id": "msg_batch-1:turn-1",
            "type": "message",
            "role": "assistant",
            "model": "anthropic/claude-sonnet-5:batch",
            "content": [
                {"type": "thinking", "thinking": "consider", "signature": "sig-1"},
                {"type": "text", "text": "checking", "citations": []},
                {
                    "type": "tool_use",
                    "id": "toolu-1",
                    "name": "Read",
                    "input": {"file_path": "a.py", "options": {"line": 3}},
                },
            ],
            "stop_reason": "tool_use",
            "stop_sequence": None,
            "usage": self.usage,
        }
        events = parse_sse(MODULE.message_to_sse(message))
        self.assertEqual(events[0][0], "message_start")
        self.assertEqual(events[-2][0], "message_delta")
        self.assertEqual(events[-2][1]["delta"]["stop_reason"], "tool_use")
        self.assertEqual(events[-2][1]["usage"], self.usage)
        self.assertEqual(events[-1][0], "message_stop")

        starts = [value for kind, value in events if kind == "content_block_start"]
        self.assertEqual([item["index"] for item in starts], [0, 1, 2])
        self.assertEqual(
            [item["content_block"]["type"] for item in starts],
            ["thinking", "text", "tool_use"],
        )
        deltas = [value["delta"] for kind, value in events
                  if kind == "content_block_delta"]
        self.assertIn({"type": "signature_delta", "signature": "sig-1"}, deltas)
        tool_delta = next(item for item in deltas if item["type"] == "input_json_delta")
        self.assertEqual(
            json.loads(tool_delta["partial_json"]),
            {"file_path": "a.py", "options": {"line": 3}},
        )

    def test_rejects_unsupported_blocks_before_emitting_success(self) -> None:
        message = {
            "type": "message",
            "role": "assistant",
            "content": [{"type": "redacted_thinking", "data": "opaque"}],
            "usage": self.usage,
        }
        with self.assertRaisesRegex(MODULE.BatchProxyError, "unsupported"):
            MODULE.message_to_sse(message)

    def test_batch_client_matches_custom_id_and_rejects_item_failure(self) -> None:
        completed = {
            "status": "completed",
            "results": [{
                "custom_id": "turn-1",
                "response": {"status_code": 200, "body": {
                    "type": "message", "role": "assistant",
                    "content": [], "usage": self.usage,
                }},
                "error": None,
            }],
        }

        class FakeClient(MODULE.OpenRouterBatchClient):
            def __init__(self, states):
                super().__init__("secret-that-must-not-appear", poll_interval=0, timeout=1)
                self.states = iter(states)

            def _json(self, call):
                if call.data:
                    envelope = json.loads(call.data)
                    self.assert_order = list(envelope)
                return next(self.states)

        client = FakeClient([{"id": "batch-1", "status": "validating"}, completed])
        result = client.submit_and_wait(
            {"model": "batch", "endpoint": "/v1/messages", "requests": []},
            "turn-1",
        )
        self.assertEqual(result["type"], "message")
        self.assertEqual(client.assert_order, ["model", "endpoint", "requests"])

        failed = dict(completed)
        failed["results"] = [{
            "custom_id": "turn-1", "response": None, "error": {"message": "bad"}
        }]
        client = FakeClient([{"id": "batch-2", "status": "validating"}, failed])
        with self.assertRaisesRegex(MODULE.BatchProxyError, "item failed"):
            client.submit_and_wait(
                {"model": "batch", "endpoint": "/v1/messages", "requests": []},
                "turn-1",
            )

    def test_partial_batch_failure_preserves_successful_sibling(self) -> None:
        message = {
            "type": "message", "role": "assistant", "content": [],
            "usage": self.usage,
        }

        class FakeClient(MODULE.OpenRouterBatchClient):
            def __init__(self):
                super().__init__("secret", poll_interval=0, timeout=1)
                self.calls = 0

            def _json(self, call):
                self.calls += 1
                if self.calls == 1:
                    return {"id": "batch-1", "status": "validating"}
                return {
                    "status": "completed",
                    "results": [
                        {"custom_id": "good", "response": {
                            "status_code": 200, "body": message,
                        }, "error": None},
                        {"custom_id": "bad", "response": None,
                         "error": {"message": "unsupported"}},
                    ],
                }

        outcomes = FakeClient().submit_and_wait_many(
            {"model": "model", "endpoint": "/v1/messages", "requests": []},
            ["good", "bad"],
        )
        self.assertEqual(outcomes["good"], message)
        self.assertIsInstance(outcomes["bad"], MODULE.BatchProxyError)

    def test_polling_404_is_observed_without_resubmitting(self) -> None:
        events = []
        message = {
            "type": "message", "role": "assistant", "content": [],
            "usage": self.usage,
        }

        class FakeClient(MODULE.OpenRouterBatchClient):
            def __init__(self):
                super().__init__(
                    "secret", poll_interval=0, timeout=1, progress=events.append
                )
                self.calls = 0

            def _json(self, call):
                self.calls += 1
                if self.calls == 1:
                    return {"id": "batch-1", "status": "validating"}
                if self.calls == 2:
                    raise MODULE.BatchHttpError(404)
                return {
                    "status": "completed",
                    "request_counts": {"total": 1, "completed": 1, "failed": 0},
                    "usage": {"cost": 0.1},
                    "results": [{
                        "custom_id": "turn-1",
                        "response": {"status_code": 200, "body": message},
                        "error": None,
                    }],
                }

        result = FakeClient().submit_and_wait(
            {"model": "batch", "endpoint": "/v1/messages", "requests": []},
            "turn-1",
        )
        self.assertEqual(result, message)
        self.assertEqual([event["event"] for event in events], [
            "submitted", "poll_not_found", "polled", "completed",
        ])

    def test_concurrent_turns_microbatch_and_retry_joins(self) -> None:
        events = []
        calls = []
        results = []

        class FakeBatchClient:
            def submit_and_wait_many(self, payload, custom_ids):
                calls.append(payload)
                return {
                    custom_id: {
                        "type": "message", "role": "assistant",
                        "content": [], "usage": self_usage,
                    }
                    for custom_id in custom_ids
                }

        self_usage = self.usage
        coordinator = MODULE.MicrobatchCoordinator(
            FakeBatchClient(), batch_model="anthropic/claude-sonnet-5:batch",
            collection_window=0.02, progress=events.append,
        )
        incoming = [
            {"stream": True, "messages": [{"role": "user", "content": value}]}
            for value in ("one", "two")
        ]

        threads = [
            threading.Thread(
                target=lambda index=index: results.append(
                    coordinator.resolve(f"digest-{index}", incoming[index])
                )
            )
            for index in range(2)
        ]
        for thread in threads:
            thread.start()
        for thread in threads:
            thread.join()
        self.assertEqual(len(calls), 1)
        self.assertEqual(len(calls[0]["requests"]), 2)
        self.assertEqual(len(results), 2)
        cached = coordinator.resolve("digest-1", incoming[1])
        self.assertEqual(cached["type"], "message")
        self.assertEqual(len(calls), 1)
        self.assertEqual(
            [event["event"] for event in events],
            [
                "turn_queued", "turn_queued", "microbatch_flushed",
                "turn_completed", "turn_completed", "turn_joined",
            ],
        )
        flushed = next(event for event in events
                       if event["event"] == "microbatch_flushed")
        self.assertEqual(
            {item["request_sha256"] for item in flushed["items"]},
            {"digest-0", "digest-1"},
        )
        completed = [event for event in events
                     if event["event"] == "turn_completed"]
        self.assertTrue(all(event["successful"] for event in completed))
        self.assertTrue(all(event["message_sha256"] for event in completed))

    def test_invalid_item_releases_itself_and_valid_sibling(self) -> None:
        results = []
        failures = []

        class FakeBatchClient:
            def submit_and_wait_many(self, payload, custom_ids):
                return {
                    custom_id: {
                        "type": "message", "role": "assistant",
                        "content": [], "usage": self_usage,
                    }
                    for custom_id in custom_ids
                }

        self_usage = self.usage
        coordinator = MODULE.MicrobatchCoordinator(
            FakeBatchClient(), batch_model="model", collection_window=0.01
        )

        def resolve(digest, incoming):
            try:
                results.append(coordinator.resolve(digest, incoming))
            except MODULE.BatchProxyError as failure:
                failures.append(str(failure))

        threads = [
            threading.Thread(target=resolve, args=(
                "good", {"stream": True, "messages": []},
            )),
            threading.Thread(target=resolve, args=(
                "bad", {"stream": False, "messages": []},
            )),
        ]
        for thread in threads:
            thread.start()
        for thread in threads:
            thread.join(timeout=1)
        self.assertFalse(any(thread.is_alive() for thread in threads))
        self.assertEqual(len(results), 1)
        self.assertEqual(len(failures), 1)

    def test_batch_model_must_match_attested_model(self) -> None:
        MODULE.validate_model_binding("canonical", "canonical")
        with self.assertRaisesRegex(MODULE.BatchProxyError, "must equal"):
            MODULE.validate_model_binding("batch-alias", "canonical")

    def test_realtime_context_management_is_unchanged_and_retry_joins(self) -> None:
        calls = []
        events = []

        class FakeRealtimeClient:
            def send(self, incoming, headers):
                calls.append((incoming, headers))
                return b"event: message_stop\ndata: {\"type\":\"message_stop\"}\n\n"

        coordinator = MODULE.RealtimeCoordinator(
            FakeRealtimeClient(), progress=events.append
        )
        incoming = {
            "model": "anthropic/claude-sonnet-5-20260630",
            "stream": True,
            "messages": [{"role": "user", "content": "hello"}],
            "context_management": {"edits": [{"type": "clear_tool_uses_20250919"}]},
        }
        headers = {"anthropic-beta": "context-management-2025-06-27"}
        first = coordinator.resolve("digest", incoming, headers)
        second = coordinator.resolve("digest", incoming, headers)
        self.assertEqual(first, second)
        self.assertEqual(calls, [(incoming, headers)])
        self.assertEqual([event["event"] for event in events], [
            "realtime_context_management_started",
            "realtime_context_management_completed",
            "realtime_context_management_joined",
        ])

    def test_realtime_client_preserves_body_and_beta_headers(self) -> None:
        seen = {}

        class FakeHeaders:
            def get_content_type(self):
                return "text/event-stream"

        class FakeResponse:
            status = 200
            headers = FakeHeaders()

            def __enter__(self):
                return self

            def __exit__(self, *_args):
                return None

            def read(self):
                return b"stream"

        original_urlopen = MODULE.request.urlopen

        def fake_urlopen(call, timeout):
            seen["call"] = call
            seen["timeout"] = timeout
            return FakeResponse()

        MODULE.request.urlopen = fake_urlopen
        try:
            incoming = {
                "model": "exact-model", "stream": True,
                "messages": [{"role": "user", "content": "hello"}],
                "context_management": {"edits": []},
            }
            result = MODULE.OpenRouterRealtimeClient(
                "secret", timeout=12
            ).send(incoming, {
                "anthropic-version": "2023-06-01",
                "anthropic-beta": "context-management-2025-06-27",
            })
        finally:
            MODULE.request.urlopen = original_urlopen
        self.assertEqual(result, b"stream")
        self.assertEqual(json.loads(seen["call"].data), incoming)
        self.assertEqual(seen["call"].get_header("Anthropic-version"), "2023-06-01")
        self.assertEqual(
            seen["call"].get_header("Anthropic-beta"),
            "context-management-2025-06-27",
        )
        self.assertEqual(seen["timeout"], 12)

    def test_http_handler_enforces_route_and_loopback_token(self) -> None:
        class FakeCoordinator:
            def resolve(self, digest, incoming):
                return {
                    "type": "message", "role": "assistant", "content": [],
                    "stop_reason": "end_turn", "stop_sequence": None,
                    "usage": self_usage,
                }

        self_usage = self.usage
        server = MODULE.BatchProxyServer(
            ("127.0.0.1", 0), client_token="local-secret",
            batch_model="canonical", client=object(),
            coordinator=FakeCoordinator(), realtime_coordinator=None,
        )
        thread = threading.Thread(target=server.serve_forever, daemon=True)
        thread.start()
        try:
            connection = http.client.HTTPConnection(
                "127.0.0.1", server.server_address[1], timeout=2
            )
            body = json.dumps({
                "model": "canonical", "stream": True, "messages": [],
            })
            connection.request(
                "POST", "/v1/messages?beta=true", body=body,
                headers={
                    "Authorization": "Bearer local-secret",
                    "Content-Type": "application/json",
                },
            )
            response = connection.getresponse()
            self.assertEqual(response.status, 200)
            self.assertEqual(response.getheader("Content-Type"), "text/event-stream")
            self.assertIn(b"event: message_stop", response.read())
            connection.close()

            connection = http.client.HTTPConnection(
                "127.0.0.1", server.server_address[1], timeout=2
            )
            connection.request(
                "POST", "/v1/messages", body=body,
                headers={"Authorization": "Bearer wrong"},
            )
            response = connection.getresponse()
            self.assertEqual(response.status, 401)
            response.read()
            connection.close()

            connection = http.client.HTTPConnection(
                "127.0.0.1", server.server_address[1], timeout=2
            )
            connection.request(
                "POST", "/other", body=body,
                headers={"Authorization": "Bearer local-secret"},
            )
            response = connection.getresponse()
            self.assertEqual(response.status, 404)
            response.read()
            connection.close()
        finally:
            server.shutdown()
            server.server_close()
            thread.join(timeout=2)


if __name__ == "__main__":
    unittest.main()
