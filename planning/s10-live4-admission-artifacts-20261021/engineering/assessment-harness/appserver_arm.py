#!/usr/bin/env python3
"""Candidate Codex app-server arm adapter. Static preparation; run only after admission.

The app-server owns auth. The thread has environments=[] and exactly two dynamic
corpus readers. This file does not start a provider call when imported.
"""
import argparse
import base64
import hashlib
import json
import os
import signal
from pathlib import Path
import queue
import subprocess
import sys
import threading
import time

MAX = 32768
CODEX = "/usr/lib/node_modules/@openai/codex/node_modules/@openai/codex-linux-x64/vendor/x86_64-unknown-linux-musl/bin/codex"


def sha(data):
    return hashlib.sha256(data).hexdigest()


def inventory(root):
    result = {}
    for path in root.rglob("*"):
        if path.is_symlink():
            raise ValueError("symlink in corpus")
        if path.is_file():
            rel = path.relative_to(root).as_posix()
            data = path.read_bytes()
            result[rel] = {"bytes": len(data), "sha256": sha(data)}
    return dict(sorted(result.items()))


def reader(root, files, audit, name, args):
    if name == "list_artifacts":
        answer = files
        outcome = "listed"
    elif name == "read_artifact":
        path = args.get("path")
        offset = args.get("offset", 0)
        length = args.get("length", MAX)
        valid = (
            isinstance(path, str) and path in files
            and type(offset) is int and type(length) is int
            and offset >= 0 and 1 <= length <= MAX
        )
        data = None
        if valid:
            source = root / path
            valid = source.is_file() and not source.is_symlink() and source.resolve().is_relative_to(root.resolve())
        if not valid:
            answer = "Unavailable"
            outcome = "denied"
        else:
            whole = source.read_bytes()
            if len(whole) != files[path]["bytes"] or sha(whole) != files[path]["sha256"]:
                answer = "Unavailable"
                outcome = "denied"
                data = None
            else:
                data = whole[offset:offset + length]
        if valid and data is not None:
            answer = {
                "path": path, "offset": offset, "returned_bytes": len(data),
                "file_bytes": files[path]["bytes"], "file_sha256": files[path]["sha256"],
            }
            try:
                answer["utf8"] = data.decode("utf-8")
            except UnicodeDecodeError:
                answer["base64"] = base64.b64encode(data).decode("ascii")
            outcome = {"returned_bytes": len(data), "sha256": sha(data)}
    else:
        answer = "Unavailable"
        outcome = "denied"
    with audit.open("a") as stream:
        stream.write(json.dumps({"at": time.time(), "method": name, "arguments": args if name == "read_artifact" and outcome != "denied" else {}, "outcome": outcome}, sort_keys=True) + "\n")
    return {"contentItems": [{"type": "inputText", "text": json.dumps(answer, sort_keys=True) if answer != "Unavailable" else answer}], "success": outcome != "denied"}


TOOLS = [
    {"type": "function", "name": "list_artifacts", "description": "List the exact admitted artifact names, sizes and SHA-256 hashes.", "inputSchema": {"type": "object", "properties": {}, "additionalProperties": False}},
    {"type": "function", "name": "read_artifact", "description": "Read up to 32768 bytes from an exact admitted artifact.", "inputSchema": {"type": "object", "properties": {"path": {"type": "string"}, "offset": {"type": "integer", "minimum": 0}, "length": {"type": "integer", "minimum": 1, "maximum": MAX}}, "required": ["path"], "additionalProperties": False}},
]


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("input_dir", type=Path)
    parser.add_argument("output_dir", type=Path)
    parser.add_argument("state_dir", type=Path)
    parser.add_argument("--model", default="gpt-6-sol")
    parser.add_argument("--effort", default="medium")
    parser.add_argument("--synthetic-base-url")
    parser.add_argument("--deadline-seconds", type=int, default=1200)
    args = parser.parse_args()
    if args.synthetic_base_url:
        if os.environ.get("S10_SYNTHETIC_NO_PROVIDER") != "1" or not args.synthetic_base_url.startswith("http://127.0.0.1:"):
            raise SystemExit("BLOCKED: synthetic probe must use local fake endpoint")
    elif os.environ.get("S10_LIVE4_ADMISSION") != "authorized" or os.environ.get("S10_LIVE4_ASSESSMENT") != "qualified":
        raise SystemExit("BLOCKED: owner admission and effective assessment qualification required")
    if not args.synthetic_base_url:
        gate_path = os.environ.get("S10_LIVE4_ASSESSMENT_GATE_FILE")
        if not gate_path:
            raise SystemExit("BLOCKED: exact assessment gate missing")
        gate = json.loads(Path(gate_path).read_text())
        required = {
            "schema": "s10-live4-appserver-assessment-qualified-v1",
            "status": "qualified", "model": args.model, "effort": args.effort,
            "adapter_sha256": sha(Path(__file__).read_bytes()),
            "environments": [], "dynamic_tool_names": ["corpus__list_artifacts", "corpus__read_artifact"],
            "no_model_visible_credential_path": True, "no_model_visible_shell_or_patch": True,
            "no_model_visible_human_input": True, "no_other_mcp_resources": True,
        }
        if any(gate.get(key) != value for key, value in required.items()):
            raise SystemExit("BLOCKED: assessment gate does not bind this adapter and boundary")
        state_entries = list(args.state_dir.iterdir())
        if any(path.is_symlink() or not path.is_file() for path in state_entries) or set(path.name for path in state_entries) - {"auth.json", "models_cache.json"}:
            raise SystemExit("BLOCKED: assessment state includes extra configuration")
        if not (args.state_dir / "auth.json").is_file():
            raise SystemExit("BLOCKED: private auth state absent")
    for directory in (args.input_dir, args.output_dir, args.state_dir):
        if not directory.is_dir() or directory.is_symlink():
            raise SystemExit("missing or symlinked directory")
    prompt = (args.input_dir / "prompt.md").read_text()
    files = inventory(args.input_dir)
    if "prompt.md" not in files:
        raise SystemExit("prompt.md missing")
    audit = args.output_dir / "reads.jsonl"
    events = args.output_dir / "appserver-events.jsonl"
    if list(args.output_dir.iterdir()):
        raise SystemExit("BLOCKED: assessment output directory must be fresh")
    environment = {"HOME": "/state", "CODEX_HOME": "/state", "PATH": "/usr/bin:/bin"}
    for key in ("SSL_CERT_FILE", "HTTPS_PROXY", "HTTP_PROXY", "NO_PROXY"):
        if key in os.environ:
            environment[key] = os.environ[key]
    cmd = [CODEX, "--strict-config", "--disable", "shell_tool", "--disable", "unified_exec", "--disable", "apps", "--disable", "plugins", "--disable", "multi_agent", "--disable", "goals", "--disable", "browser_use", "--disable", "computer_use", "--disable", "image_generation", "--disable", "view_image", "-c", 'web_search="disabled"', "-c", "agents.enabled=false", "-c", "tools.experimental_request_user_input.enabled=false", "-c", "model_reasoning_effort=" + args.effort]
    if args.synthetic_base_url:
        environment["S10_FAKE_KEY"] = "synthetic-no-secret"
        cmd += ["-c", 'model_provider="synthetic"', "-c", 'model_providers.synthetic.name="Synthetic"', "-c", 'model_providers.synthetic.base_url="' + args.synthetic_base_url + '"', "-c", 'model_providers.synthetic.env_key="S10_FAKE_KEY"', "-c", 'model_providers.synthetic.wire_api="responses"', "-c", "model_providers.synthetic.requires_openai_auth=false", "-c", "model_providers.synthetic.supports_websockets=false"]
    cmd += ["app-server", "--stdio"]
    sandbox = ["/usr/bin/bwrap", "--unshare-all", "--share-net", "--die-with-parent", "--new-session",
               "--ro-bind", "/usr", "/usr", "--symlink", "usr/bin", "/bin",
               "--symlink", "usr/lib", "/lib", "--symlink", "usr/lib64", "/lib64",
               "--ro-bind", "/etc", "/etc", "--dev", "/dev", "--proc", "/proc", "--tmpfs", "/tmp",
               "--ro-bind", str(args.input_dir.resolve()), "/in",
               "--bind", str(args.state_dir.resolve()), "/state",
               "--chdir", "/in", "--clearenv"]
    for key, value in environment.items():
        sandbox += ["--setenv", key, value]
    child = subprocess.Popen(sandbox + ["--"] + cmd, stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=(args.output_dir / "appserver.stderr").open("wb"), text=True, bufsize=1, cwd=args.input_dir, env={"PATH": "/usr/bin:/bin"}, start_new_session=True)
    incoming = queue.Queue()
    def consume():
        for line in child.stdout:
            incoming.put(line)
        incoming.put(None)
    threading.Thread(target=consume, daemon=True).start()
    next_id = 1
    def send(payload):
        child.stdin.write(json.dumps(payload, separators=(",", ":")) + "\n")
        child.stdin.flush()
    def receive(deadline):
        remain = deadline - time.monotonic()
        if remain <= 0:
            raise TimeoutError("app-server arm deadline")
        raw = incoming.get(timeout=remain)
        if raw is None:
            raise RuntimeError("app-server exited")
        with events.open("a") as stream:
            stream.write(raw)
        item = json.loads(raw)
        if item.get("method") == "item/tool/call" and "id" in item:
            params = item.get("params") or {}
            if params.get("namespace") == "corpus" and params.get("tool") in ("list_artifacts", "read_artifact"):
                try:
                    result = reader(args.input_dir, files, audit, params["tool"], params.get("arguments") or {})
                except Exception:
                    result = {"contentItems": [{"type": "inputText", "text": "Unavailable"}], "success": False}
            else:
                result = {"contentItems": [{"type": "inputText", "text": "Unavailable"}], "success": False}
            send({"id": item["id"], "result": result})
        elif "id" in item and "method" in item:
            send({"id": item["id"], "error": {"code": -32601, "message": "Unavailable"}})
        return item
    def call(method, params, deadline):
        nonlocal next_id
        ident = next_id
        next_id += 1
        send({"id": ident, "method": method, "params": params})
        while True:
            item = receive(deadline)
            if item.get("id") == ident and "method" not in item:
                if "error" in item:
                    raise RuntimeError(method + ": " + str(item["error"]))
                return item["result"]
    deadline = time.monotonic() + args.deadline_seconds
    outcome = "failed_before_turn"
    try:
        call("initialize", {"clientInfo": {"name": "s10_live4_assessment", "title": "S10 Live4 Assessment", "version": "0.1.0"}, "capabilities": {"experimentalApi": True}}, deadline)
        send({"method": "initialized", "params": {}})
        start = call("thread/start", {"cwd": "/in", "model": args.model, "modelProvider": "synthetic" if args.synthetic_base_url else "openai", "approvalPolicy": "never", "sandbox": "read-only", "environments": [], "dynamicTools": [{"type": "namespace", "name": "corpus", "description": "Read the admitted arm corpus only.", "tools": TOOLS}], "ephemeral": False, "historyMode": "legacy"}, deadline)
        thread_id = start["thread"]["id"]
        session_receipt = {"thread_id": thread_id, "thread_path": start["thread"].get("path"), "model": args.model, "effort": args.effort, "environments": [], "files": files}
        (args.output_dir / "session.json").write_text(json.dumps(session_receipt, indent=2, sort_keys=True))
        outcome = "turn_launched"
        turn = call("turn/start", {"threadId": thread_id, "input": [{"type": "text", "text": prompt}], "environments": []}, deadline)
        turn_id = turn["turn"]["id"]
        rollouts = []
        for _ in range(100):
            rollouts = list(args.state_dir.glob("sessions/**/rollout-*" + thread_id + ".jsonl"))
            if len(rollouts) == 1 and rollouts[0].stat().st_size:
                break
            time.sleep(0.01)
        if len(rollouts) != 1:
            raise RuntimeError("exact rollout unavailable at assessment turn start")
        first = json.loads(rollouts[0].open().readline())
        session_meta_id = first.get("payload", {}).get("id") if first.get("type") == "session_meta" else None
        if not session_meta_id:
            raise RuntimeError("rollout session_meta unavailable at assessment turn start")
        session_receipt["session_meta_id"] = session_meta_id
        session_receipt["rollout_path"] = str(rollouts[0])
        session_receipt["turn_id"] = turn_id
        (args.output_dir / "session.json").write_text(json.dumps(session_receipt, indent=2, sort_keys=True))
        while True:
            item = receive(deadline)
            if item.get("method") == "turn/completed" and (item.get("params") or {}).get("turn", {}).get("id") == turn_id:
                break
        session_receipt["turn_status"] = item["params"]["turn"].get("status")
        (args.output_dir / "session.json").write_text(json.dumps(session_receipt, indent=2, sort_keys=True))
        outcome = "turn_" + str(session_receipt["turn_status"])
        print(json.dumps({"session": session_receipt, "turn": item["params"]["turn"]}, sort_keys=True))
        if not args.synthetic_base_url and session_receipt["turn_status"] != "completed":
            raise RuntimeError("assessment turn did not complete")
    finally:
        (args.output_dir / "run-status.json").write_text(json.dumps({"outcome": outcome, "session_receipt": str(args.output_dir / "session.json") if (args.output_dir / "session.json").exists() else None, "event_log": str(events), "read_audit": str(audit) if audit.exists() else None}, indent=2, sort_keys=True))
        if child.poll() is None:
            os.killpg(child.pid, signal.SIGTERM)
            try:
                child.wait(timeout=3)
            except subprocess.TimeoutExpired:
                os.killpg(child.pid, signal.SIGKILL)
                child.wait()


if __name__ == "__main__":
    main()
