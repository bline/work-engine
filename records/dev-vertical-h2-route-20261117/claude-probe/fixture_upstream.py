#!/usr/bin/env python3
"""Scripted Anthropic Messages SSE fixture: request with no tool_result -> tool_use; else final text."""
import http.server, json, sys

PORT = int(sys.argv[1])


def sse(events):
    out = b""
    for e in events:
        out += b"event: " + e["type"].encode() + b"\ndata: " + json.dumps(e).encode() + b"\n\n"
    return out


def usage(o):
    return {"input_tokens": 10, "output_tokens": o, "cache_creation_input_tokens": 0, "cache_read_input_tokens": 0}


class H(http.server.BaseHTTPRequestHandler):
    protocol_version = "HTTP/1.1"

    def log_message(self, *a):
        pass

    def do_POST(self):
        body = json.loads(self.rfile.read(int(self.headers["content-length"])))
        model = body.get("model", "fixture")
        has_result = any(isinstance(m.get("content"), list) and any(b.get("type") == "tool_result" for b in m["content"])
                         for m in body["messages"] if m["role"] == "user")
        mid = "msg_fixture_%d" % (2 if has_result else 1)
        start = {"type": "message_start", "message": {"id": mid, "type": "message", "role": "assistant", "model": model,
                 "content": [], "stop_reason": None, "stop_sequence": None, "usage": usage(1)}}
        if not has_result:
            args = {"snapshot_id": "snap-1", "member_id": "docs/claim.md", "offset": 0, "length": 48}
            ev = [start,
                  {"type": "content_block_start", "index": 0, "content_block": {"type": "tool_use", "id": "toolu_fixture_1", "name": "mcp__we__snapshot_read", "input": {}}},
                  {"type": "content_block_delta", "index": 0, "delta": {"type": "input_json_delta", "partial_json": json.dumps(args)}},
                  {"type": "content_block_stop", "index": 0},
                  {"type": "message_delta", "delta": {"stop_reason": "tool_use", "stop_sequence": None}, "usage": {"output_tokens": 20}},
                  {"type": "message_stop"}]
        else:
            ev = [start,
                  {"type": "content_block_start", "index": 0, "content_block": {"type": "text", "text": ""}},
                  {"type": "content_block_delta", "index": 0, "delta": {"type": "text_delta", "text": "Fixture answer: the claim member was read."}},
                  {"type": "content_block_stop", "index": 0},
                  {"type": "message_delta", "delta": {"stop_reason": "end_turn", "stop_sequence": None}, "usage": {"output_tokens": 9}},
                  {"type": "message_stop"}]
        data = sse(ev)
        self.send_response(200)
        self.send_header("content-type", "text/event-stream")
        self.send_header("request-id", "req_fixture")
        self.send_header("content-length", str(len(data)))
        self.end_headers()
        self.wfile.write(data)


http.server.ThreadingHTTPServer(("127.0.0.1", PORT), H).serve_forever()
