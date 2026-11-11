#!/usr/bin/env python3
"""Local fake of the Anthropic Messages streaming endpoint, for driving real Pi.

No provider is contacted and nothing is billed. Each request's path, body and
the headers that matter for custody (x-api-key, authorization, anthropic-beta,
anthropic-version) are appended to --log. Usage reports cache reads and 5m/1h
cache writes. Mode file keys (re-read on every request, all optional):
  reply         assistant text (default "OK <request number>")
  model         model to report (default: the requested one)
  tool_command  a request without a tool_result gets a tool_use asking Pi's
                bash tool to run it; the next request gets a text reply
  error_event   true: send an `error` event and no message_stop
"""
from __future__ import annotations

import argparse
import json
import threading
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path

LOCK = threading.Lock()
COUNT = [0]
USAGE = {"input_tokens": 11, "cache_read_input_tokens": 300, "cache_creation_input_tokens": 50,
         "cache_creation": {"ephemeral_5m_input_tokens": 40, "ephemeral_1h_input_tokens": 10},
         "output_tokens": 1, "service_tier": "standard"}


def has_tool_result(messages) -> bool:
    return any(isinstance(b, dict) and b.get("type") == "tool_result"
               for m in messages or [] if isinstance(m, dict) and isinstance(m.get("content"), list)
               for b in m["content"])


class Handler(BaseHTTPRequestHandler):
    log_path: Path
    mode_path: Path | None

    def log_message(self, *args):
        pass

    def event(self, kind: str, data: dict):
        self.wfile.write(f"event: {kind}\ndata: {json.dumps({'type': kind, **data})}\n\n".encode())
        self.wfile.flush()

    def do_POST(self):
        req = json.loads(self.rfile.read(int(self.headers.get("content-length") or 0)) or b"{}")
        mode = {}
        if self.mode_path and self.mode_path.exists():
            mode = json.loads(self.mode_path.read_text() or "{}")
        with LOCK:
            COUNT[0] += 1
            n = COUNT[0]
            with open(self.log_path, "a") as fh:
                fh.write(json.dumps({"n": n, "path": self.path, "body": req, "headers": {
                    k: self.headers.get(k) for k in ("x-api-key", "authorization", "anthropic-beta",
                                                     "anthropic-version")}}) + "\n")
        self.send_response(200)
        self.send_header("content-type", "text/event-stream")
        self.end_headers()
        model = mode.get("model") or req.get("model")
        self.event("message_start", {"message": {"id": f"msg_fake_{n}", "type": "message", "role": "assistant",
                                                 "model": model, "content": [], "stop_reason": None,
                                                 "stop_sequence": None, "usage": USAGE}})
        if mode.get("error_event"):
            self.event("error", {"error": {"type": "overloaded_error", "message": "fake overload"}})
            return
        if mode.get("tool_command") and not has_tool_result(req.get("messages")):
            self.event("content_block_start", {"index": 0, "content_block": {
                "type": "tool_use", "id": f"toolu_fake_{n}", "name": "bash", "input": {}}})
            self.event("content_block_delta", {"index": 0, "delta": {
                "type": "input_json_delta", "partial_json": json.dumps({"command": mode["tool_command"]})}})
            stop = "tool_use"
        else:
            self.event("content_block_start", {"index": 0, "content_block": {"type": "text", "text": ""}})
            self.event("content_block_delta", {"index": 0, "delta": {
                "type": "text_delta", "text": mode.get("reply") or f"OK {n}"}})
            stop = "end_turn"
        self.event("content_block_stop", {"index": 0})
        self.event("message_delta", {"delta": {"stop_reason": stop, "stop_sequence": None},
                                     "usage": {"output_tokens": 7}})
        self.event("message_stop", {})


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--port-file", type=Path, required=True)
    ap.add_argument("--log", type=Path, required=True)
    ap.add_argument("--mode-file", type=Path)
    a = ap.parse_args()
    Handler.log_path, Handler.mode_path = a.log, a.mode_file
    server = ThreadingHTTPServer(("127.0.0.1", 0), Handler)
    a.port_file.write_text(str(server.server_port))
    server.serve_forever()


if __name__ == "__main__":
    main()
