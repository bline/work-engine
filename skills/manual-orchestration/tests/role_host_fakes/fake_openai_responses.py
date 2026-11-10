#!/usr/bin/env python3
"""Local fake of the OpenAI Responses streaming endpoint, for driving real Pi.

No provider is contacted and nothing is billed. Each request body is appended to
--log (headers are not recorded). The response echoes a requested service_tier
the way the real API reports the applied tier, unless the mode file overrides
it. Mode file keys (re-read on every request, all optional):

  reply         assistant text (default "OK <request number>")
  tier          service_tier to report regardless of the request
  omit_tier     true: report no service_tier at all
  delay         seconds to wait before streaming (simulates a long turn)
  fail          true: stream response.failed instead of completing
"""
from __future__ import annotations

import argparse
import json
import threading
import time
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path

LOCK = threading.Lock()
COUNT = [0]


def mode(path: Path | None) -> dict:
    if path is None or not path.exists():
        return {}
    try:
        value = json.loads(path.read_text() or "{}")
    except ValueError:
        return {}
    return value if isinstance(value, dict) else {}


class Handler(BaseHTTPRequestHandler):
    log_path: Path
    mode_path: Path | None

    def log_message(self, *args):  # keep stderr quiet
        pass

    def do_POST(self):
        body = self.rfile.read(int(self.headers.get("content-length") or 0))
        try:
            req = json.loads(body or b"{}")
        except ValueError:
            req = {"unparsed": body.decode("utf-8", "replace")}
        with LOCK:
            COUNT[0] += 1
            n = COUNT[0]
            with open(self.log_path, "a") as fh:
                fh.write(json.dumps({"n": n, "path": self.path, "body": req}) + "\n")
        if not self.path.rstrip("/").endswith("/responses"):
            self.send_error(404)
            return
        m = mode(self.mode_path)
        if m.get("delay"):
            time.sleep(float(m["delay"]))
        model = req.get("model", "unknown")
        if m.get("omit_tier"):
            tier = None
        else:
            tier = m.get("tier") or req.get("service_tier") or "default"
        text = m.get("reply") or f"OK {n}"
        rid, mid = f"resp_{n}", f"msg_{n}"
        base = {"id": rid, "object": "response", "created_at": int(time.time()), "model": model}
        if tier is not None:
            base["service_tier"] = tier
        item_done = {"id": mid, "type": "message", "status": "completed", "role": "assistant",
                     "content": [{"type": "output_text", "text": text, "annotations": []}]}
        events = [
            {"type": "response.created", "response": {**base, "status": "in_progress", "output": []}},
            {"type": "response.output_item.added", "output_index": 0,
             "item": {"id": mid, "type": "message", "status": "in_progress", "role": "assistant", "content": []}},
            {"type": "response.content_part.added", "item_id": mid, "output_index": 0, "content_index": 0,
             "part": {"type": "output_text", "text": "", "annotations": []}},
            {"type": "response.output_text.delta", "item_id": mid, "output_index": 0, "content_index": 0, "delta": text},
            {"type": "response.output_text.done", "item_id": mid, "output_index": 0, "content_index": 0, "text": text},
            {"type": "response.output_item.done", "output_index": 0, "item": item_done},
        ]
        if m.get("fail"):
            events.append({"type": "response.failed", "response": {
                **base, "status": "failed", "error": {"code": "fake_failure", "message": "fake failure"}}})
        else:
            events.append({"type": "response.completed", "response": {
                **base, "status": "completed", "output": [item_done],
                "usage": {"input_tokens": 1000, "output_tokens": 10, "total_tokens": 1010,
                          "input_tokens_details": {"cached_tokens": 0},
                          "output_tokens_details": {"reasoning_tokens": 0}}}})
        self.send_response(200)
        self.send_header("content-type", "text/event-stream")
        self.send_header("cache-control", "no-cache")
        self.end_headers()
        for seq, event in enumerate(events):
            event["sequence_number"] = seq
            self.wfile.write(f"event: {event['type']}\ndata: {json.dumps(event)}\n\n".encode())
            self.wfile.flush()
        self.close_connection = True


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("--port-file", type=Path, required=True)
    ap.add_argument("--log", type=Path, required=True)
    ap.add_argument("--mode-file", type=Path)
    a = ap.parse_args()
    Handler.log_path, Handler.mode_path = a.log, a.mode_file
    server = ThreadingHTTPServer(("127.0.0.1", 0), Handler)
    tmp = a.port_file.with_suffix(".tmp")
    tmp.write_text(str(server.server_address[1]))
    tmp.replace(a.port_file)
    server.serve_forever()
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
