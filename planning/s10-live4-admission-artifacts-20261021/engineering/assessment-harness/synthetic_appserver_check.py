#!/usr/bin/env python3
"""Local fake-provider app-server protocol canary; no model or external network."""
import http.server
import json
import os
import pathlib
import shutil
import subprocess
import tempfile
import threading

ROOT = pathlib.Path(__file__).resolve().parent
captured = []
raw_requests = []


class Handler(http.server.BaseHTTPRequestHandler):
    def do_POST(self):
        body = self.rfile.read(int(self.headers.get("Content-Length", "0")))
        raw_requests.append(body.decode("utf-8", errors="replace"))
        try:
            data = json.loads(body)
            inputs = data.get("input", [])
            record = {
                "path": self.path, "model": data.get("model"),
                "tools": data.get("tools"),
                "additional_tools": [
                    [{"namespace": tool.get("name"), "tools": [entry.get("name") for entry in tool.get("tools", [])], "description_has_corpus": "corpus" in tool.get("description", "")}
                     for tool in item.get("tools", [])]
                    for item in inputs if isinstance(item, dict) and item.get("type") == "additional_tools"
                ],
                "input_types": [item.get("type") for item in inputs if isinstance(item, dict)],
                "tool_output": [str(item.get("output", ""))[:2500] for item in inputs if isinstance(item, dict) and item.get("type") == "custom_tool_call_output"],
            }
        except Exception as error:
            record = {"parse_error": type(error).__name__}
        captured.append(record)
        if len(captured) == 1:
            events = [
                {"type": "response.created", "response": {"id": "synthetic-app-1"}},
                {"type": "response.output_item.done", "item": {"type": "custom_tool_call", "call_id": "synthetic-app-call-1", "namespace": "functions", "name": "exec", "input": "text(ALL_TOOLS.map(t => t.name)); text(await tools.corpus__list_artifacts({})); text(await tools.corpus__read_artifact({path: 'prompt.md', offset: 0, length: 5})); text(await tools.corpus__read_artifact({path: '/state/auth.json', offset: 0, length: 5})); text(await tools.list_mcp_resources({})); text(await tools.list_mcp_resource_templates({}));"}},
                {"type": "response.completed", "response": {"id": "synthetic-app-1", "usage": {"input_tokens": 0, "output_tokens": 0, "total_tokens": 0}}},
            ]
            reply = "".join("event: " + e["type"] + "\ndata: " + json.dumps(e) + "\n\n" for e in events).encode()
            self.send_response(200)
            self.send_header("Content-Type", "text/event-stream")
        else:
            reply = b'{"error":{"message":"synthetic endpoint deliberately stops before model response","type":"invalid_request_error"}}'
            self.send_response(400)
            self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(reply)))
        self.end_headers()
        self.wfile.write(reply)

    def log_message(self, *_args):
        pass


server = http.server.HTTPServer(("127.0.0.1", 0), Handler)
threading.Thread(target=server.serve_forever, daemon=True).start()
try:
    with tempfile.TemporaryDirectory() as temp:
        base = pathlib.Path(temp)
        for name in ("in", "out", "state"):
            (base / name).mkdir()
        (base / "in" / "prompt.md").write_text("Synthetic corpus only.\n")
        cache = pathlib.Path("/home/bline/.codex/models_cache.json")
        if cache.is_file():
            shutil.copyfile(cache, base / "state" / "models_cache.json")
        cmd = [
            "/usr/bin/python3", str(ROOT / "appserver_arm.py"),
            str(base / "in"), str(base / "out"), str(base / "state"),
            "--synthetic-base-url", "http://127.0.0.1:" + str(server.server_port) + "/v1",
        ]
        result = subprocess.run(cmd, capture_output=True, text=True, timeout=25, env={"PATH": "/usr/bin:/bin", "S10_SYNTHETIC_NO_PROVIDER": "1"})
        events = (base / "out" / "appserver-events.jsonl")
        names = captured[0].get("additional_tools", []) if captured else []
        output = captured[1].get("tool_output", []) if len(captured) > 1 else []
        session = base / "out" / "session.json"
        evidence_path = os.environ.get("S10_EVIDENCE_PATH")
        if evidence_path:
            pathlib.Path(evidence_path).write_text(json.dumps({
                "synthetic_only": True,
                "raw_http_request_bodies": raw_requests,
                "appserver_events_jsonl": events.read_text() if events.exists() else "",
                "read_audit_jsonl": (base / "out" / "reads.jsonl").read_text() if (base / "out" / "reads.jsonl").exists() else "",
                "session_receipt": json.loads(session.read_text()) if session.exists() else None,
            }, sort_keys=True) + "\n")
        print(json.dumps({
            "returncode": result.returncode,
            "request_count": len(captured),
            "model": captured[0].get("model") if captured else None,
            "transport_tools_null": captured[0].get("tools") is None if captured else None,
            "additional_tool_names": names,
            "tool_output": output,
            "session": json.loads(session.read_text()) if session.exists() else None,
            "read_audit": (base / "out" / "reads.jsonl").read_text().splitlines() if (base / "out" / "reads.jsonl").exists() else [],
            "turn_failed_only_from_synthetic_endpoint": "synthetic endpoint deliberately stops before model response" in result.stdout,
            "appserver_stderr": (base / "out" / "appserver.stderr").read_text()[-500:],
        }, sort_keys=True))
finally:
    server.shutdown()
