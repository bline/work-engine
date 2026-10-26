#!/usr/bin/env python3
"""Fake shared Codex app-server daemon: JSON-RPC over WebSocket on a Unix socket.

Reproduces the behaviour measured on 0.164.0 (probe
metrics/probes/codex-daemon-thread-host-20261026.json):
- turns keep running after the starting client disconnects;
- thread/read on an unknown thread fails with "thread not loaded";
- a just-started turn is invisible to thread/read and thread/turns/list for a
  moment (VISIBLE_AFTER seconds here);
- a turn's shell sees CODEX_THREAD_ID and the daemon's own environment, not the
  caller's.

Each turn runs fake_successor.py with the turn's input as its prompt. Per-turn
behaviour comes from the JSON file named by FAKE_DAEMON_MODE, read at turn start:
  FAKE_SUCCESSOR     passed to fake_successor.py (adopt|silent|wrong_topic|wrong_pred)
  FAKE_TURN=hang     the turn runs until interrupted
  FAKE_CODEX_MISMATCH  thread/resume answers with another thread id

usage: fake_app_server.py SOCKET_PATH
"""
import base64, hashlib, json, os, socket, socketserver, struct, subprocess, sys, tempfile, threading, time, uuid
from pathlib import Path

HERE = Path(__file__).resolve().parent
GUID = "258EAFA5-E914-47DA-95CA-C5AB0DC85B11"
VISIBLE_AFTER = 0.5
CALLER_IDENTITY = ("CLAUDE_CODE_SESSION_ID", "CLAUDE_PID", "CLAUDECODE", "CODEX_THREAD_ID")

LOCK = threading.Lock()
THREADS: dict[str, dict] = {}


def mode():
    p = os.environ.get("FAKE_DAEMON_MODE")
    try:
        return json.loads(Path(p).read_text()) if p else {}
    except (OSError, ValueError):
        return {}


def run_turn(tid, turn, text, m):
    if m.get("FAKE_TURN") == "hang":
        turn["stop"].wait(600)
    else:
        pf = Path(tempfile.mkstemp(prefix="fake-daemon-prompt-")[1]); pf.write_text(text)
        env = {k: v for k, v in os.environ.items() if k not in CALLER_IDENTITY}
        env.update(CODEX_THREAD_ID=tid, FAKE_SUCCESSOR=m.get("FAKE_SUCCESSOR", "adopt"))
        p = subprocess.Popen([sys.executable, str(HERE / "fake_successor.py"), str(pf)], env=env,
                             cwd=THREADS[tid]["cwd"])
        while p.poll() is None and not turn["stop"].wait(0.1):
            pass
        if p.poll() is None:
            p.terminate(); p.wait()
    with LOCK:
        turn["status"] = "interrupted" if turn["stop"].is_set() else "completed"


def visible_turns(t):
    now = time.time()
    return [x for x in t["turns"] if now - x["created"] >= VISIBLE_AFTER]


def handle(method, params):
    with LOCK:
        if method == "initialize":
            return {"userAgent": "fake"}
        if method == "thread/start":
            tid = str(uuid.uuid4())
            THREADS[tid] = {"cwd": params.get("cwd"), "name": None, "turns": [], "params": params}
            return {"thread": {"id": tid, "status": {"type": "idle"}}, "model": params.get("model", "fake-model"),
                    "reasoningEffort": None}
        tid = params.get("threadId")
        t = THREADS.get(tid)
        # Threads on disk but not loaded here (FAKE_DISK_THREADS in the mode
        # file): read as notLoaded, and their last turn reads as interrupted
        # whatever another writer is doing, as the real daemon reports.
        if t is None and tid in mode().get("FAKE_DISK_THREADS", "").split(","):
            if method == "thread/read":
                return {"thread": {"id": tid, "name": None, "status": {"type": "notLoaded"}}}
            if method == "thread/turns/list":
                return {"data": [{"id": "disk-turn", "status": "interrupted"}]}
        if t is None:
            raise LookupError(f"thread not loaded: {tid}")
        if method == "thread/name/set":
            t["name"] = params["name"]; return {}
        if method == "thread/read":
            turns = visible_turns(t)
            active = any(x["status"] == "inProgress" for x in turns)
            return {"thread": {"id": tid, "name": t["name"],
                               "status": {"type": "active", "activeFlags": []} if active else {"type": "idle"}}}
        if method == "thread/turns/list":
            turns = list(reversed(visible_turns(t)))[: params.get("limit") or 50]
            return {"data": [{"id": x["id"], "status": x["status"]} for x in turns]}
        if method == "thread/resume":
            got = str(uuid.uuid4()) if mode().get("FAKE_CODEX_MISMATCH") else tid
            return {"thread": {"id": got, "status": {"type": "idle"}}}
        if method == "turn/start":
            if any(x["status"] == "inProgress" for x in t["turns"]):
                raise RuntimeError("a turn is already in progress")
            turn = {"id": str(uuid.uuid4()), "status": "inProgress", "created": time.time(),
                    "stop": threading.Event(), "effort": params.get("effort")}
            t["turns"].append(turn)
            text = "".join(i.get("text", "") for i in params.get("input", []))
            threading.Thread(target=run_turn, args=(tid, turn, text, mode()), daemon=True).start()
            return {"turn": {"id": turn["id"], "status": "inProgress"}}
        if method == "turn/interrupt":
            for x in t["turns"]:
                if x["id"] == params.get("turnId"):
                    x["stop"].set(); return {}
            raise LookupError("no such turn")
    raise NotImplementedError(method)


class WS(socketserver.StreamRequestHandler):
    def send(self, obj):
        data = json.dumps(obj).encode(); n = len(data)
        head = bytes([0x81]) + (bytes([n]) if n < 126 else bytes([126]) + struct.pack("!H", n)
                                if n < 65536 else bytes([127]) + struct.pack("!Q", n))
        self.wfile.write(head + data); self.wfile.flush()

    def frame(self):
        b = self.rfile.read(2)
        if len(b) < 2:
            return None
        op, n = b[0] & 0x0F, b[1] & 0x7F
        if n == 126: n = struct.unpack("!H", self.rfile.read(2))[0]
        elif n == 127: n = struct.unpack("!Q", self.rfile.read(8))[0]
        mask = self.rfile.read(4) if b[1] & 0x80 else b"\0\0\0\0"
        data = bytes(c ^ mask[i % 4] for i, c in enumerate(self.rfile.read(n)))
        return None if op == 0x8 else data

    def handle(self):
        headers = {}
        self.rfile.readline()
        while (line := self.rfile.readline().decode()) not in ("\r\n", ""):
            k, _, v = line.partition(":"); headers[k.strip().lower()] = v.strip()
        acc = base64.b64encode(hashlib.sha1((headers["sec-websocket-key"] + GUID).encode()).digest()).decode()
        self.wfile.write(f"HTTP/1.1 101 Switching Protocols\r\nUpgrade: websocket\r\nConnection: Upgrade\r\n"
                         f"Sec-WebSocket-Accept: {acc}\r\n\r\n".encode()); self.wfile.flush()
        while (data := self.frame()) is not None:
            msg = json.loads(data)
            if "id" not in msg:
                continue
            try:
                self.send({"id": msg["id"], "result": handle(msg["method"], msg.get("params") or {})})
            except Exception as e:  # noqa: BLE001 - reported to the client as a JSON-RPC error
                self.send({"id": msg["id"], "error": {"code": -32600, "message": str(e)}})


class Server(socketserver.ThreadingMixIn, socketserver.UnixStreamServer):
    daemon_threads = True


if __name__ == "__main__":
    path = sys.argv[1]
    if os.path.exists(path):
        os.unlink(path)
    Server(path, WS).serve_forever()
