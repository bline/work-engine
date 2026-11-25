#!/usr/bin/env python3
"""A local stand-in for Anthropic's /v1/messages as the claude-sdk lane's proxy reaches it.

Streams the S0-observed SSE shape: one `read` tool call when the conversation has a
single user message and no tool result, otherwise a text answer. Each request is
appended to --log as {path, auth, headers, body_sha256, body}. The mode file (JSON,
re-read per request) selects faults:
  status: non-200 status (JSON error body)       billing: "omit" | "rejected" | header value
  model: response model override                 terminal: false -> no message_stop
  hang: seconds to wait after headers, before the body
"""
import argparse
import hashlib
import json
import time
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path


def events(model, n, kind, read_path):
    usage = {"input_tokens": 100, "cache_read_input_tokens": 0, "cache_creation_input_tokens": 0,
             "cache_creation": {"ephemeral_5m_input_tokens": 0, "ephemeral_1h_input_tokens": 0},
             "output_tokens": 1, "service_tier": "standard"}
    out = [{"type": "message_start", "message": {"id": f"msg_fake_{n}", "type": "message", "role": "assistant",
                                                 "model": model, "content": [], "stop_reason": None, "usage": usage}},
           {"type": "ping"}]
    if kind == "tool":
        out += [{"type": "content_block_start", "index": 0, "content_block": {
                    "type": "tool_use", "id": f"toolu_fake_{n}", "name": "mcp__pi__read", "input": {}}},
                {"type": "content_block_delta", "index": 0, "delta": {
                    "type": "input_json_delta", "partial_json": json.dumps({"path": read_path})}},
                {"type": "content_block_stop", "index": 0}]
        stop = "tool_use"
    else:
        out += [{"type": "content_block_start", "index": 0, "content_block": {"type": "text", "text": ""}},
                {"type": "content_block_delta", "index": 0, "delta": {"type": "text_delta", "text": f"FAKE-DONE-{n}"}},
                {"type": "content_block_stop", "index": 0}]
        stop = "end_turn"
    out.append({"type": "message_delta", "delta": {"stop_reason": stop},
                "usage": {"input_tokens": 100, "cache_read_input_tokens": 0, "cache_creation_input_tokens": 0,
                          "output_tokens": 7}})
    out.append({"type": "message_stop"})
    return out


def make_handler(log, mode_file, read_path):
    state = {"n": 0}

    class Handler(BaseHTTPRequestHandler):
        def log_message(self, *a):
            pass

        def do_POST(self):
            raw = self.rfile.read(int(self.headers.get("content-length") or 0))
            mode = json.loads(Path(mode_file).read_text()) if mode_file and Path(mode_file).exists() else {}
            state["n"] += 1
            n = state["n"]
            body = json.loads(raw)
            if log:
                with open(log, "a") as fh:
                    fh.write(json.dumps({"n": n, "path": self.path, "auth": self.headers.get("authorization"),
                                         "x_api_key": self.headers.get("x-api-key"), "headers": dict(self.headers),
                                         "body_sha256": hashlib.sha256(raw).hexdigest(), "body": body}) + "\n")
            if mode.get("status"):
                payload = json.dumps({"type": "error", "error": {"type": "api_error",
                                                                  "message": "fake account detail a@b.c"}}).encode()
                self.send_response(mode["status"])
                self.send_header("content-type", "application/json")
                self.send_header("content-length", str(len(payload)))
                self.end_headers()
                self.wfile.write(payload)
                return
            msgs = body.get("messages") or []
            last = msgs[-1] if msgs else {}
            has_result = isinstance(last.get("content"), list) and any(
                isinstance(c, dict) and c.get("type") == "tool_result" for c in last["content"])
            users = [m for m in msgs if m.get("role") == "user"]
            kind = "text" if has_result or len(users) > 1 or mode.get("text_only") else "tool"
            evs = events(mode.get("model") or body.get("model"), n, kind, mode.get("read_path") or read_path)
            if mode.get("terminal") is False:
                evs = [e for e in evs if e["type"] != "message_stop"]
            data = b"".join(f"event: {e['type']}\ndata: {json.dumps(e)}\n\n".encode() for e in evs)
            self.send_response(200)
            self.send_header("content-type", "text/event-stream")
            billing = mode.get("billing", "allowed")
            if billing != "omit":
                self.send_header("anthropic-ratelimit-unified-status", billing)
                self.send_header("anthropic-ratelimit-unified-overage-status", "rejected")
            self.end_headers()
            if mode.get("hang"):
                self.wfile.flush()
                time.sleep(mode["hang"])
            self.wfile.write(data)

    return Handler


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--port-file", required=True)
    ap.add_argument("--log")
    ap.add_argument("--mode-file")
    ap.add_argument("--read-path", default="hello.txt")
    a = ap.parse_args()
    server = ThreadingHTTPServer(("127.0.0.1", 0), make_handler(a.log, a.mode_file, a.read_path))
    Path(a.port_file).write_text(str(server.server_port))
    server.serve_forever()


if __name__ == "__main__":
    main()
