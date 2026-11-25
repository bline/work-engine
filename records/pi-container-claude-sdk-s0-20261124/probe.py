#!/usr/bin/env python3
"""S0 offline wire probe: Pi + pi-claude-subscription in the pi_container profile, against a host mock.

No real credential and no network: the container gets the synthetic OAuth credential
(pi_container.CLAUDE_SDK_PLACEHOLDER) in a tmpfs HOME, ANTHROPIC_BASE_URL points at
the entry's loopback relay, and the relay's Unix socket is served here by a scripted
Messages mock. Turn 1 asks for one `read` tool call then answers; turn 2 resumes the
same Pi session in a new container. Only a scrubbed shape of each request is kept.

usage: probe.py RUN_DIR [--model claude-haiku-5-5] [--fault MODE]
  MODE: none | status500 (first request answered 500, to observe retries)
"""
import argparse
import hashlib
import json
import os
import socketserver
import subprocess
import sys
import threading
import uuid
from http.server import BaseHTTPRequestHandler
from pathlib import Path

HERE = Path(__file__).resolve()
SCRIPTS = HERE.parents[2] / "skills/manual-orchestration/scripts"
sys.path.insert(0, str(SCRIPTS))
import pi_container as pc  # noqa: E402

KEEP_HEADERS = ("anthropic-beta", "anthropic-version", "x-stainless-retry-count", "x-app", "accept",
                "content-type", "x-stainless-lang", "x-stainless-package-version", "x-stainless-runtime",
                "x-stainless-timeout", "anthropic-dangerous-direct-browser-access")


def digest(value) -> str:
    return hashlib.sha256(json.dumps(value, sort_keys=True).encode()).hexdigest()[:16]


EXPECT = {}


def shape(method, path, headers, body):
    h = {k.lower(): v for k, v in headers.items()}
    auth = h.get("authorization", "")
    out = {"method": method, "path": path, "header_names": sorted(h),
           "headers": {k: h[k] for k in KEEP_HEADERS if k in h},
           "user_agent_product": h.get("user-agent", "").split(" ")[0],
           "x_client_app": h.get("x-client-app"),
           "authorization": ("placeholder" if auth == "Bearer " + pc.CLAUDE_SDK_PLACEHOLDER
                             else "absent" if not auth else "OTHER"),
           "x_api_key": "present" if "x-api-key" in h else "absent"}
    if isinstance(body, dict):
        msgs = body.get("messages") or []
        out["body"] = {
            "keys": sorted(body), "model": body.get("model"), "stream": body.get("stream"),
            "max_tokens": body.get("max_tokens"), "thinking": body.get("thinking"),
            "output_config": body.get("output_config"), "tool_choice": body.get("tool_choice"),
            "context_management": body.get("context_management"),
            "metadata_keys": sorted(body.get("metadata") or {}),
            "metadata_user_id_shape": sorted(json.loads(body["metadata"]["user_id"]))
            if isinstance((body.get("metadata") or {}).get("user_id"), str) and body["metadata"]["user_id"].startswith("{") else None,
            "tools": [{"name": t.get("name"), "type": t.get("type"), "keys": sorted(t)} for t in body.get("tools") or []],
            "system": [{"type": b.get("type"), "head": (b.get("text") or "")[:60] if i < 2 else None,
                        "sha": digest(b), "cache_control": b.get("cache_control")}
                       for i, b in enumerate(body.get("system") or [])],
            "replaced_prompt": {"third_block_equals": EXPECT.get("prompt") is not None and len(body.get("system") or []) > 2
                                and (body["system"][2].get("text") or "").strip() == EXPECT["prompt"].strip(),
                                "contained": EXPECT.get("prompt") is not None
                                and any(EXPECT["prompt"] in (b.get("text") or "") for b in body.get("system") or []),
                                "third_block_chars": len((body.get("system") or [{}, {}, {}])[2].get("text") or "")
                                if len(body.get("system") or []) > 2 else None},
            "messages": [{"role": m.get("role"), "blocks": [c.get("type") for c in m["content"]]
                          if isinstance(m.get("content"), list) else "text"} for m in msgs],
        }
    return out


def sse(events):
    return b"".join(f"event: {e['type']}\ndata: {json.dumps(e)}\n\n".encode() for e in events)


def reply_events(model, n, kind, workspace):
    start = {"type": "message_start", "message": {
        "id": f"msg_mock_{n}", "type": "message", "role": "assistant", "model": model, "content": [],
        "stop_reason": None, "stop_sequence": None,
        "usage": {"input_tokens": 100, "cache_read_input_tokens": 0, "cache_creation_input_tokens": 0,
                  "cache_creation": {"ephemeral_5m_input_tokens": 0, "ephemeral_1h_input_tokens": 0},
                  "output_tokens": 1, "service_tier": "standard"}}}
    if kind == "tool":
        blocks = [{"type": "content_block_start", "index": 0,
                   "content_block": {"type": "tool_use", "id": f"toolu_mock_{n}", "name": "mcp__pi__read", "input": {}}},
                  {"type": "content_block_delta", "index": 0, "delta": {
                      "type": "input_json_delta", "partial_json": json.dumps({"path": f"{workspace}/hello.txt"})}},
                  {"type": "content_block_stop", "index": 0}]
        stop = "tool_use"
    else:
        blocks = [{"type": "content_block_start", "index": 0, "content_block": {"type": "text", "text": ""}},
                  {"type": "content_block_delta", "index": 0, "delta": {"type": "text_delta", "text": f"MOCK-DONE-{n}"}},
                  {"type": "content_block_stop", "index": 0}]
        stop = "end_turn"
    return [start, {"type": "ping"}, *blocks,
            {"type": "message_delta", "delta": {"stop_reason": stop, "stop_sequence": None},
             "usage": {"input_tokens": 100, "cache_read_input_tokens": 0, "cache_creation_input_tokens": 0,
                       "output_tokens": 7}},
            {"type": "message_stop"}]


class Mock(socketserver.ThreadingMixIn, socketserver.UnixStreamServer):
    daemon_threads = True


def serve(sock, log, model, workspace, fault, text_only=False):
    state = {"n": 0}
    lock = threading.Lock()

    class H(BaseHTTPRequestHandler):
        protocol_version = "HTTP/1.1"

        def log_message(self, *a):
            pass

        def handle_any(self):
            length = int(self.headers.get("content-length") or 0)
            raw = self.rfile.read(length) if length else b""
            try:
                body = json.loads(raw) if raw else None
            except ValueError:
                body = "<non-json>"
            with lock:
                state["n"] += 1
                n = state["n"]
            row = {"n": n, **shape(self.command, self.path, dict(self.headers), body)}
            if self.command != "POST" or not self.path.startswith("/v1/messages"):
                row["mock"] = "404"
                self.send_response(404)
                self.send_header("content-length", "0")
                self.end_headers()
            elif fault == "status500" and n == 1:
                row["mock"] = "500"
                payload = b'{"type":"error","error":{"type":"api_error","message":"mock"}}'
                self.send_response(500)
                self.send_header("content-type", "application/json")
                self.send_header("content-length", str(len(payload)))
                self.end_headers()
                self.wfile.write(payload)
            else:
                msgs = body.get("messages") or []
                last = msgs[-1] if msgs else {}
                has_result = isinstance(last.get("content"), list) and any(
                    c.get("type") == "tool_result" for c in last["content"])
                users = [m for m in msgs if m.get("role") == "user"]
                kind = "text" if has_result or len(users) > 1 or text_only else "tool"
                row["mock"] = kind
                data = sse(reply_events(model, n, kind, workspace))
                self.send_response(200)
                self.send_header("content-type", "text/event-stream")
                self.send_header("anthropic-ratelimit-unified-status", "allowed")
                self.send_header("anthropic-ratelimit-unified-overage-status", "rejected")
                self.send_header("request-id", f"req_mock_{n}")
                self.send_header("content-length", str(len(data)))
                self.end_headers()
                self.wfile.write(data)
            with open(log, "a") as fh:
                fh.write(json.dumps(row) + "\n")

        do_POST = do_GET = do_PUT = do_HEAD = handle_any

    if os.path.exists(sock):
        os.unlink(sock)
    server = Mock(sock, H)
    threading.Thread(target=server.serve_forever, daemon=True).start()
    return server


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("run_dir")
    ap.add_argument("--model", default="claude-haiku-5-5")
    ap.add_argument("--fault", default="none", choices=("none", "status500"))
    ap.add_argument("--thinking", default=None)
    ap.add_argument("--no-tools", action="store_true", help="tool-less session (Pi --no-tools); mock answers text")
    ap.add_argument("--system-prompt", default=None, help="replace Pi's system prompt (Pi --system-prompt)")
    ap.add_argument("--no-lane-env", action="store_true",
                    help="omit pi_container.CLAUDE_SDK_CONTAINER_ENV (retry/fallback/traffic switches)")
    a = ap.parse_args()
    EXPECT["prompt"] = a.system_prompt
    run = Path(a.run_dir).resolve()
    workspace, cdir, sockdir = run / "workspace", run / "container", run / "sock"
    for d in (workspace, cdir / "agent", cdir / "sessions", sockdir):
        d.mkdir(parents=True, exist_ok=True)
    (workspace / "hello.txt").write_text("probe-file-contents\n")
    (cdir / "agent" / "settings.json").write_text(
        ("{}" if a.no_lane_env else json.dumps(pc.CLAUDE_SDK_PI_SETTINGS)) + "\n")
    runtime = pc.runtime_paths(pc.DEFAULT_RUNTIME)
    bundle = pc.claude_sdk_bundle()
    identity = pc.claude_sdk_bundle_identity(bundle)
    sid = str(uuid.uuid4())
    log = run / "wire.jsonl"
    server = serve(str(sockdir / "egress.sock"), log, a.model, str(workspace), a.fault, text_only=a.no_tools)
    home = os.path.expanduser("~")
    env = {"uid": os.getuid(), "gid": os.getgid(), "home": home, "runtime": runtime,
           "limits": {"memory": "2g", "pids": 512, "cpus": "2"}, "mounts": [[str(workspace), "rw"]],
           "host_executables": ["docker"]}
    results = []
    for turn, prompt in ((1, "Read hello.txt with the read tool, then reply."), (2, "Reply once more.")):
        files = {k: cdir / f"turn-{turn}.{k}" for k in ("prompt", "events", "stderr", "exit")}
        files["prompt"].write_text(prompt)
        argv = ["run", "--rm", "--pull", "never", "--name", f"we-claude-s0-{sid[:8]}-{turn}",
                *pc.run_profile(env, cwd=str(workspace), container_dir=str(cdir), socket_dir=str(sockdir))]
        for p in bundle["packages"]:
            argv += ["--mount", f"type=bind,src={p},dst={p},readonly"]
        cenv = {"HOME": home, "PATH": "/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin",
                "LANG": "C.UTF-8", "PI_CODING_AGENT_DIR": str(cdir / "agent"), "PI_OFFLINE": "1",
                "PI_TELEMETRY": "0", "ANTHROPIC_BASE_URL": f"http://127.0.0.1:{pc.RELAY_PORT}",
                "PI_CLAUDE_SDK_CLAUDE_PATH": bundle["claude"],
                "PI_CLAUDE_SDK_DEBUG": str(cdir / f"turn-{turn}.sdk-debug"),
                "WORK_ENGINE_CLAUDE_SDK_CREDENTIAL": "synthetic",
                **({} if a.no_lane_env else pc.CLAUDE_SDK_CONTAINER_ENV)}
        for k, v in cenv.items():
            argv += ["--env", f"{k}={v}"]
        pi_argv = ["--mode", "json", "--session-id", sid, "--session-dir", str(cdir / "sessions"),
                   "--no-extensions", "-e", bundle["extension"], "--model", f"claude-sdk/{a.model}",
                   *(["--no-tools"] if a.no_tools else ["--tools", "read"]),
                   *(["--system-prompt", a.system_prompt] if a.system_prompt else []),
                   *(["--thinking", a.thinking] if a.thinking else [])]
        argv += ["--entrypoint", "/usr/bin/python3", pc.IMAGE, str(SCRIPTS / "pi_container_entry.py"),
                 str(sockdir / "egress.sock"), *(str(files[k]) for k in ("prompt", "events", "stderr", "exit")),
                 "--", runtime["node"], runtime["cli"], *pi_argv]
        r = subprocess.run(["docker", *argv], capture_output=True, text=True, timeout=600)
        events = [json.loads(x) for x in files["events"].read_text().splitlines() if x.strip()] \
            if files["events"].exists() else []
        finals = [e["message"] for e in events if e.get("type") == "message_end"
                  and (e.get("message") or {}).get("role") == "assistant"]
        results.append({"turn": turn, "docker_exit": r.returncode, "docker_stderr": r.stderr[-400:],
                        "pi_exit": files["exit"].read_text().strip() if files["exit"].exists() else None,
                        "header": (events[0].get("type"), events[0].get("id") == sid) if events else None,
                        "assistant": [{"provider": m.get("provider"), "model": m.get("model"),
                                       "stopReason": m.get("stopReason"), "error": (m.get("errorMessage") or "")[:300],
                                       "content": [c.get("type") for c in m.get("content") or []]} for m in finals],
                        "tool_results": sum(1 for e in events if e.get("type") == "tool_execution_end"),
                        "stderr_tail": files["stderr"].read_text()[-600:] if files["stderr"].exists() else ""})
    server.shutdown()
    summary = {"session_id": sid, "no_tools": a.no_tools, "system_prompt_replaced": bool(a.system_prompt), "model": a.model, "fault": a.fault, "lane_env": not a.no_lane_env, "pi_version": runtime["pi_version"],
               "runtime": runtime["resolved"], "bundle": identity, "claude": bundle["claude"], "turns": results}
    (run / "summary.json").write_text(json.dumps(summary, indent=1) + "\n")
    print(json.dumps(summary, indent=1))


if __name__ == "__main__":
    main()
