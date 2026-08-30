#!/usr/bin/env python3
"""Transport native Claude Code Messages turns through OpenRouter Batch."""

from __future__ import annotations

import argparse
import copy
import hashlib
import json
import os
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path
import secrets
import sys
import threading
import time
from typing import Any, Callable
from urllib import error, request

from claude_transport import TransportError, read_routing_attestation


BATCH_URL = "https://openrouter.ai/api/beta/batches"
REALTIME_URL = "https://openrouter.ai/api/v1/messages?beta=true"
MAX_REQUEST_BYTES = 32 * 1024 * 1024
TERMINAL_STATUSES = {"completed", "failed", "expired", "cancelled"}


class BatchProxyError(ValueError):
    """Raised when a turn cannot be transported without weakening the contract."""


class BatchHttpError(BatchProxyError):
    def __init__(self, status: int):
        super().__init__(f"OpenRouter Batch API returned HTTP {status}")
        self.status = status


def request_profile(incoming: dict[str, Any]) -> dict[str, Any]:
    """Return content-free request-shape telemetry for compatibility audits."""
    tools = incoming.get("tools")
    if not isinstance(tools, list):
        tools = []
    tool_schema_bytes = len(json.dumps(
        tools, sort_keys=True, separators=(",", ":"), ensure_ascii=False
    ).encode("utf-8"))
    return {
        "tool_count": len(tools),
        "tool_schema_bytes": tool_schema_bytes,
        "deferred_tool_count": sum(
            1 for tool in tools
            if isinstance(tool, dict) and tool.get("defer_loading") is True
        ),
        "context_management_present": "context_management" in incoming,
    }


def validate_model_binding(batch_model: str, guardrail_model: str) -> None:
    if batch_model != guardrail_model:
        raise BatchProxyError(
            "--batch-model must equal the attested --guardrail-model"
        )


class _TurnEntry:
    def __init__(self) -> None:
        self.ready = threading.Event()
        self.message: dict[str, Any] | None = None
        self.error: BatchProxyError | None = None


class _StreamEntry:
    def __init__(self) -> None:
        self.ready = threading.Event()
        self.stream: bytes | None = None
        self.error: BatchProxyError | None = None


class RealtimeCoordinator:
    """Forward unsupported batch turns unchanged and deduplicate their retries."""

    def __init__(self, client: "OpenRouterRealtimeClient", *,
                 progress: Callable[[dict[str, Any]], None] | None = None):
        self.client = client
        self.progress = progress or (lambda _event: None)
        self.lock = threading.Lock()
        self.entries: dict[str, _StreamEntry] = {}

    def resolve(self, digest: str, incoming: dict[str, Any],
                headers: dict[str, str]) -> bytes:
        with self.lock:
            entry = self.entries.get(digest)
            if entry is None:
                entry = _StreamEntry()
                self.entries[digest] = entry
                owner = True
                self.progress({
                    "event": "realtime_context_management_started",
                    "request_sha256": digest,
                    **request_profile(incoming),
                })
            else:
                owner = False
                self.progress({
                    "event": "realtime_context_management_joined",
                    "request_sha256": digest,
                })
        if owner:
            try:
                entry.stream = self.client.send(incoming, headers)
            except BatchProxyError as failure:
                entry.error = failure
            finally:
                entry.ready.set()
            self.progress({
                "event": "realtime_context_management_completed",
                "request_sha256": digest,
                "successful": entry.error is None,
            })
        else:
            entry.ready.wait()
        if entry.error is not None:
            raise BatchProxyError(str(entry.error))
        if entry.stream is None:
            raise BatchProxyError("realtime turn ended without an SSE stream")
        return entry.stream


class MicrobatchCoordinator:
    """Coalesce concurrent turns and make retries join their original item."""

    def __init__(self, client: "OpenRouterBatchClient", *, batch_model: str,
                 collection_window: float,
                 progress: Callable[[dict[str, Any]], None] | None = None):
        self.client = client
        self.batch_model = batch_model
        self.collection_window = collection_window
        self.progress = progress or (lambda _event: None)
        self.lock = threading.Lock()
        self.entries: dict[str, _TurnEntry] = {}
        self.pending: list[tuple[str, str, dict[str, Any]]] = []
        self.flush_scheduled = False

    def resolve(self, digest: str, incoming: dict[str, Any]) -> dict[str, Any]:
        with self.lock:
            entry = self.entries.get(digest)
            if entry is None:
                entry = _TurnEntry()
                self.entries[digest] = entry
                custom_id = f"turn-{secrets.token_hex(12)}"
                self.pending.append((digest, custom_id, copy.deepcopy(incoming)))
                self.progress({
                    "event": "turn_queued", "request_sha256": digest,
                    **request_profile(incoming),
                })
                if not self.flush_scheduled:
                    self.flush_scheduled = True
                    threading.Thread(target=self._flush_after_window, daemon=True).start()
            else:
                self.progress({"event": "turn_joined", "request_sha256": digest})
        entry.ready.wait()
        if entry.error is not None:
            raise BatchProxyError(str(entry.error))
        if entry.message is None:
            raise BatchProxyError("coordinated turn ended without a Message")
        return copy.deepcopy(entry.message)

    def _flush_after_window(self) -> None:
        time.sleep(self.collection_window)
        with self.lock:
            pending = self.pending
            self.pending = []
            self.flush_scheduled = False
        requests = []
        custom_ids = []
        preparation_errors: dict[str, BatchProxyError] = {}
        for _digest, custom_id, incoming in pending:
            try:
                item = prepare_batch_item(
                    incoming, batch_model=self.batch_model, custom_id=custom_id
                )
            except Exception as failure:  # guarantee every waiter is released
                preparation_errors[custom_id] = BatchProxyError(
                    f"batch item preparation failed: {type(failure).__name__}"
                )
            else:
                requests.append(item)
                custom_ids.append(custom_id)
        self.progress({
            "event": "microbatch_flushed", "item_count": len(requests),
            "rejected_item_count": len(preparation_errors),
            "items": [
                {"request_sha256": digest, "custom_id": custom_id}
                for digest, custom_id, _incoming in pending
            ],
        })
        try:
            if requests:
                results = self.client.submit_and_wait_many({
                    "model": self.batch_model,
                    "endpoint": "/v1/messages",
                    "requests": requests,
                }, custom_ids)
            else:
                results = {}
        except Exception as failure:  # guarantee every waiter is released
            results = {}
            batch_error = BatchProxyError(
                f"batch coordination failed: {type(failure).__name__}"
            )
        else:
            batch_error = None
        for digest, custom_id, _incoming in pending:
            entry = self.entries[digest]
            if custom_id in preparation_errors:
                entry.error = preparation_errors[custom_id]
            elif batch_error is not None:
                entry.error = BatchProxyError(str(batch_error))
            else:
                outcome = results[custom_id]
                if isinstance(outcome, BatchProxyError):
                    entry.error = outcome
                else:
                    entry.message = outcome
            self.progress({
                "event": "turn_completed",
                "request_sha256": digest,
                "custom_id": custom_id,
                "successful": entry.error is None,
                "message_sha256": (
                    hashlib.sha256(json.dumps(
                        entry.message, sort_keys=True, separators=(",", ":"),
                        ensure_ascii=False,
                    ).encode("utf-8")).hexdigest()
                    if entry.message is not None else None
                ),
            })
            entry.ready.set()


def prepare_batch_request(
    incoming: dict[str, Any], *, batch_model: str, custom_id: str
) -> dict[str, Any]:
    """Change only the model and streaming fields required by batch transport."""
    if incoming.get("stream") is not True:
        raise BatchProxyError("batch loopback requires stream=true from Claude Code")
    if not isinstance(incoming.get("messages"), list):
        raise BatchProxyError("Messages request is missing its messages array")
    item = prepare_batch_item(incoming, batch_model=batch_model, custom_id=custom_id)
    # OpenRouter's beta parser currently requires model to precede requests.
    return {
        "model": batch_model,
        "endpoint": "/v1/messages",
        "requests": [item],
    }


def prepare_batch_item(
    incoming: dict[str, Any], *, batch_model: str, custom_id: str
) -> dict[str, Any]:
    if incoming.get("stream") is not True:
        raise BatchProxyError("batch loopback requires stream=true from Claude Code")
    if not isinstance(incoming.get("messages"), list):
        raise BatchProxyError("Messages request is missing its messages array")
    body = copy.deepcopy(incoming)
    body["model"] = batch_model
    body["stream"] = False
    return {"custom_id": custom_id, "body": body}


def _event(kind: str, value: dict[str, Any]) -> bytes:
    payload = json.dumps(value, separators=(",", ":"), ensure_ascii=False)
    return f"event: {kind}\ndata: {payload}\n\n".encode("utf-8")


def _initial_usage(usage: dict[str, Any]) -> dict[str, Any]:
    initial = copy.deepcopy(usage)
    initial["output_tokens"] = 0
    details = initial.get("output_tokens_details")
    if isinstance(details, dict) and "thinking_tokens" in details:
        details["thinking_tokens"] = 0
    return initial


def message_to_sse(message: dict[str, Any]) -> bytes:
    """Synthesize a complete Anthropic SSE stream from one final Message."""
    if message.get("type") != "message" or message.get("role") != "assistant":
        raise BatchProxyError("batch result is not an assistant Message")
    content = message.get("content")
    usage = message.get("usage")
    if not isinstance(content, list) or not isinstance(usage, dict):
        raise BatchProxyError("batch Message is missing content or usage")

    supported = {"text", "tool_use", "thinking"}
    for block in content:
        if not isinstance(block, dict) or block.get("type") not in supported:
            raise BatchProxyError("batch Message contains an unsupported content block")
        if block["type"] == "text" and block.get("citations") not in (None, []):
            raise BatchProxyError("citation-bearing text is not supported by the prototype")

    initial = copy.deepcopy(message)
    initial["content"] = []
    initial["stop_reason"] = None
    initial["stop_sequence"] = None
    if "stop_details" in initial:
        initial["stop_details"] = None
    initial["usage"] = _initial_usage(usage)
    chunks = [_event("message_start", {"type": "message_start", "message": initial})]

    for index, block in enumerate(content):
        block_type = block["type"]
        if block_type == "text":
            text = block.get("text")
            if not isinstance(text, str):
                raise BatchProxyError("text content block has invalid text")
            start = {"type": "text", "text": ""}
            if "citations" in block:
                start["citations"] = []
            deltas = [{"type": "text_delta", "text": text}]
        elif block_type == "tool_use":
            if not all(isinstance(block.get(field), str) for field in ("id", "name")):
                raise BatchProxyError("tool_use block is missing its identity")
            if not isinstance(block.get("input"), dict):
                raise BatchProxyError("tool_use input must be an object")
            start = {
                "type": "tool_use", "id": block["id"],
                "name": block["name"], "input": {},
            }
            deltas = [{
                "type": "input_json_delta",
                "partial_json": json.dumps(
                    block["input"], separators=(",", ":"), ensure_ascii=False
                ),
            }]
        else:
            thinking = block.get("thinking")
            signature = block.get("signature")
            if not isinstance(thinking, str) or not isinstance(signature, str):
                raise BatchProxyError("thinking block is missing thinking or signature")
            start = {"type": "thinking", "thinking": "", "signature": ""}
            deltas = []
            if thinking:
                deltas.append({"type": "thinking_delta", "thinking": thinking})
            deltas.append({"type": "signature_delta", "signature": signature})

        chunks.append(_event("content_block_start", {
            "type": "content_block_start", "index": index, "content_block": start,
        }))
        for delta in deltas:
            chunks.append(_event("content_block_delta", {
                "type": "content_block_delta", "index": index, "delta": delta,
            }))
        chunks.append(_event("content_block_stop", {
            "type": "content_block_stop", "index": index,
        }))

    delta = {
        "stop_reason": message.get("stop_reason"),
        "stop_sequence": message.get("stop_sequence"),
    }
    if "stop_details" in message:
        delta["stop_details"] = message.get("stop_details")
    chunks.append(_event("message_delta", {
        "type": "message_delta", "delta": delta, "usage": usage,
    }))
    chunks.append(_event("message_stop", {"type": "message_stop"}))
    return b"".join(chunks)


class OpenRouterBatchClient:
    def __init__(self, api_key: str, *, poll_interval: float, timeout: float,
                 progress: Callable[[dict[str, Any]], None] | None = None):
        self.api_key = api_key
        self.poll_interval = poll_interval
        self.timeout = timeout
        self.progress = progress or (lambda _event: None)

    def _json(self, call: request.Request) -> dict[str, Any]:
        try:
            with request.urlopen(call, timeout=30) as response:
                payload = response.read()
        except error.HTTPError as failure:
            raise BatchHttpError(failure.code) from failure
        except (error.URLError, TimeoutError, OSError) as failure:
            raise BatchProxyError("OpenRouter Batch API request failed") from failure
        try:
            value = json.loads(payload)
        except (UnicodeDecodeError, json.JSONDecodeError) as failure:
            raise BatchProxyError("OpenRouter Batch API returned invalid JSON") from failure
        if not isinstance(value, dict):
            raise BatchProxyError("OpenRouter Batch API response must be an object")
        return value

    def submit_and_wait(self, payload: dict[str, Any], custom_id: str) -> dict[str, Any]:
        outcome = self.submit_and_wait_many(payload, [custom_id])[custom_id]
        if isinstance(outcome, BatchProxyError):
            raise outcome
        return outcome

    def submit_and_wait_many(
        self, payload: dict[str, Any], custom_ids: list[str]
    ) -> dict[str, dict[str, Any] | BatchProxyError]:
        submit = request.Request(
            BATCH_URL,
            data=json.dumps(payload, separators=(",", ":")).encode("utf-8"),
            headers={
                "Authorization": f"Bearer {self.api_key}",
                "Content-Type": "application/json", "Accept": "application/json",
            },
            method="POST",
        )
        state = self._json(submit)
        batch_id = state.get("id")
        if not isinstance(batch_id, str) or not batch_id:
            raise BatchProxyError("batch submission did not return an ID")
        self.progress({
            "event": "submitted", "batch_id": batch_id,
            "status": state.get("status"),
            "custom_ids": list(custom_ids),
            "request_count": len(custom_ids),
            "payload_sha256": hashlib.sha256(json.dumps(
                payload, sort_keys=True, separators=(",", ":"),
                ensure_ascii=False,
            ).encode("utf-8")).hexdigest(),
        })
        deadline = time.monotonic() + self.timeout
        poll_count = 0
        while state.get("status") not in TERMINAL_STATUSES:
            if time.monotonic() >= deadline:
                raise BatchProxyError("batch polling exceeded the configured timeout")
            time.sleep(self.poll_interval)
            poll_count += 1
            try:
                state = self._json(request.Request(
                    f"{BATCH_URL}/{batch_id}",
                    headers={
                        "Authorization": f"Bearer {self.api_key}",
                        "Accept": "application/json",
                    },
                    method="GET",
                ))
            except BatchHttpError as failure:
                if failure.status != 404:
                    raise
                self.progress({
                    "event": "poll_not_found", "batch_id": batch_id,
                    "poll_count": poll_count,
                })
                continue
            self.progress({
                "event": "polled", "batch_id": batch_id,
                "poll_count": poll_count, "status": state.get("status"),
                "request_counts": state.get("request_counts"),
            })
        if state.get("status") != "completed":
            raise BatchProxyError(f"batch ended with status {state.get('status')!r}")
        self.progress({
            "event": "completed", "batch_id": batch_id,
            "request_counts": state.get("request_counts"),
            "usage": state.get("usage"),
        })
        results = state.get("results")
        if not isinstance(results, list):
            raise BatchProxyError("completed batch is missing results")
        output: dict[str, dict[str, Any] | BatchProxyError] = {}
        for custom_id in custom_ids:
            matches = [item for item in results if isinstance(item, dict)
                       and item.get("custom_id") == custom_id]
            if len(matches) != 1:
                output[custom_id] = BatchProxyError(
                    "completed batch does not contain one result for every item"
                )
                continue
            result = matches[0]
            response = result.get("response")
            if result.get("error") is not None or not isinstance(response, dict):
                output[custom_id] = BatchProxyError("batch item failed")
                continue
            if (response.get("status_code") != 200
                    or not isinstance(response.get("body"), dict)):
                output[custom_id] = BatchProxyError(
                    "batch item did not return a successful Message"
                )
                continue
            output[custom_id] = response["body"]
        return output


class OpenRouterRealtimeClient:
    """Forward one native Anthropic Messages request without schema changes."""

    def __init__(self, api_key: str, *, timeout: float):
        self.api_key = api_key
        self.timeout = timeout

    def send(self, incoming: dict[str, Any], headers: dict[str, str]) -> bytes:
        outbound_headers = {
            "Authorization": f"Bearer {self.api_key}",
            "Content-Type": "application/json",
            "Accept": "text/event-stream",
        }
        for name in ("anthropic-version", "anthropic-beta", "x-anthropic-beta"):
            value = headers.get(name)
            if value:
                outbound_headers[name] = value
        call = request.Request(
            REALTIME_URL,
            data=json.dumps(incoming, separators=(",", ":")).encode("utf-8"),
            headers=outbound_headers,
            method="POST",
        )
        try:
            with request.urlopen(call, timeout=self.timeout) as response:
                content_type = response.headers.get_content_type()
                if response.status != 200 or content_type != "text/event-stream":
                    raise BatchProxyError(
                        "OpenRouter realtime route did not return an SSE stream"
                    )
                return response.read()
        except error.HTTPError as failure:
            raise BatchHttpError(failure.code) from failure
        except (error.URLError, TimeoutError, OSError) as failure:
            raise BatchProxyError("OpenRouter realtime request failed") from failure


class BatchProxyServer(ThreadingHTTPServer):
    daemon_threads = True

    def __init__(self, address: tuple[str, int], *, client_token: str,
                 batch_model: str, client: OpenRouterBatchClient,
                 coordinator: MicrobatchCoordinator,
                 realtime_coordinator: RealtimeCoordinator | None):
        super().__init__(address, BatchProxyHandler)
        self.client_token = client_token
        self.batch_model = batch_model
        self.batch_client = client
        self.coordinator = coordinator
        self.realtime_coordinator = realtime_coordinator


class BatchProxyHandler(BaseHTTPRequestHandler):
    server: BatchProxyServer

    def log_message(self, _format: str, *_args: Any) -> None:
        return

    def _error(self, status: int, message: str) -> None:
        payload = json.dumps({
            "type": "error",
            "error": {"type": "batch_proxy_error", "message": message},
        }, separators=(",", ":")).encode("utf-8")
        self.send_response(status)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(payload)))
        self.end_headers()
        try:
            self.wfile.write(payload)
        except (BrokenPipeError, ConnectionResetError):
            return

    def do_POST(self) -> None:  # noqa: N802 - stdlib handler contract
        if self.path.split("?", 1)[0] != "/v1/messages":
            self._error(404, "only POST /v1/messages is supported")
            return
        supplied_auth = self.headers.get("Authorization", "")
        expected_auth = f"Bearer {self.server.client_token}"
        if not secrets.compare_digest(supplied_auth, expected_auth):
            self._error(401, "invalid loopback authorization")
            return
        try:
            length = int(self.headers.get("Content-Length", ""))
        except ValueError:
            self._error(400, "invalid Content-Length")
            return
        if length <= 0 or length > MAX_REQUEST_BYTES:
            self._error(413, "request body is empty or too large")
            return
        try:
            incoming = json.loads(self.rfile.read(length))
            if not isinstance(incoming, dict):
                raise BatchProxyError("Messages request must be an object")
            canonical = json.dumps(
                {"batch_model": self.server.batch_model, "body": incoming},
                sort_keys=True, separators=(",", ":"), ensure_ascii=False,
            ).encode("utf-8")
            digest = hashlib.sha256(canonical).hexdigest()

            if ("context_management" in incoming
                    and self.server.realtime_coordinator is not None):
                forwarded_headers = {
                    name.lower(): value for name, value in self.headers.items()
                }
                stream = self.server.realtime_coordinator.resolve(
                    digest, incoming, forwarded_headers
                )
            else:
                message = self.server.coordinator.resolve(digest, incoming)
                stream = message_to_sse(message)
        except (BatchProxyError, json.JSONDecodeError) as failure:
            self._error(502, str(failure))
            return
        self.send_response(200)
        self.send_header("Content-Type", "text/event-stream")
        self.send_header("Cache-Control", "no-cache")
        self.send_header("Connection", "close")
        self.end_headers()
        try:
            self.wfile.write(stream)
            self.wfile.flush()
        except (BrokenPipeError, ConnectionResetError):
            return


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--host", choices=("127.0.0.1", "::1"), default="127.0.0.1")
    parser.add_argument("--port", type=int, default=0)
    parser.add_argument("--batch-model", required=True)
    parser.add_argument("--guardrail-model", required=True)
    parser.add_argument("--routing-attestation", required=True, type=Path)
    parser.add_argument("--attestation-max-age-seconds", type=int, default=900)
    parser.add_argument("--poll-interval-seconds", type=float, default=2.0)
    parser.add_argument("--poll-timeout-seconds", type=float, default=3600.0)
    parser.add_argument("--collection-window-seconds", type=float, default=0.5)
    parser.add_argument(
        "--realtime-context-management", action="store_true",
        help=("forward context_management turns unchanged through OpenRouter's "
              "native realtime Anthropic endpoint"),
    )
    parser.add_argument("--event-log", type=Path)
    return parser


def event_logger(path: Path | None) -> Callable[[dict[str, Any]], None]:
    lock = threading.Lock()

    def emit(event: dict[str, Any]) -> None:
        value = dict(event)
        value["observed_at"] = time.time()
        line = json.dumps(value, sort_keys=True, separators=(",", ":")) + "\n"
        if path is None:
            print(line, end="", file=sys.stderr, flush=True)
            return
        path.parent.mkdir(parents=True, exist_ok=True)
        with lock, path.open("a", encoding="utf-8") as handle:
            handle.write(line)
            handle.flush()

    return emit


def main() -> int:
    args = build_parser().parse_args()
    api_key = os.environ.get("OPENROUTER_API_KEY", "")
    key_hash = os.environ.get("OPENROUTER_API_KEY_HASH", "")
    client_token = os.environ.get("CLAUDE_BATCH_PROXY_TOKEN", "")
    if not api_key or not key_hash or not client_token:
        print(
            "error: OPENROUTER_API_KEY, OPENROUTER_API_KEY_HASH, and "
            "CLAUDE_BATCH_PROXY_TOKEN are required",
            file=sys.stderr,
        )
        return 2
    if (args.poll_interval_seconds <= 0 or args.poll_timeout_seconds <= 0
            or args.collection_window_seconds < 0):
        print("error: polling intervals must be positive", file=sys.stderr)
        return 2
    try:
        validate_model_binding(args.batch_model, args.guardrail_model)
    except BatchProxyError as failure:
        print(f"error: {failure}", file=sys.stderr)
        return 2
    try:
        attestation = read_routing_attestation(
            args.routing_attestation,
            expected_model=args.guardrail_model,
            expected_key_hash=key_hash,
            max_age_seconds=args.attestation_max_age_seconds,
        )
    except TransportError as failure:
        print(f"error: {failure}", file=sys.stderr)
        return 2
    progress = event_logger(args.event_log)
    progress({
        "event": "proxy_started",
        "schema_version": 1,
        "proxy_sha256": hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
        "batch_model": args.batch_model,
        "guardrail_model": args.guardrail_model,
        "routing_attestation_sha256": attestation["sha256"],
        "poll_interval_seconds": args.poll_interval_seconds,
        "poll_timeout_seconds": args.poll_timeout_seconds,
        "collection_window_seconds": args.collection_window_seconds,
        "realtime_context_management": args.realtime_context_management,
    })
    client = OpenRouterBatchClient(
        api_key, poll_interval=args.poll_interval_seconds,
        timeout=args.poll_timeout_seconds, progress=progress,
    )
    coordinator = MicrobatchCoordinator(
        client, batch_model=args.batch_model,
        collection_window=args.collection_window_seconds, progress=progress,
    )
    realtime_coordinator = None
    if args.realtime_context_management:
        realtime_coordinator = RealtimeCoordinator(
            OpenRouterRealtimeClient(
                api_key, timeout=args.poll_timeout_seconds,
            ),
            progress=progress,
        )
    server = BatchProxyServer(
        (args.host, args.port), client_token=client_token,
        batch_model=args.batch_model, client=client,
        coordinator=coordinator, realtime_coordinator=realtime_coordinator,
    )
    host, port = server.server_address[:2]
    print(json.dumps({
        "schema_version": 1, "kind": "claude-batch-loopback-ready",
        "base_url": f"http://{host}:{port}",
        "batch_model": args.batch_model,
        "guardrail_model": args.guardrail_model,
        "routing_attestation_sha256": attestation["sha256"],
    }, sort_keys=True), flush=True)
    try:
        server.serve_forever()
    except KeyboardInterrupt:
        pass
    finally:
        server.server_close()
        progress({"event": "proxy_stopped"})
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
