#!/usr/bin/env python3
"""Minimal client for the shared local Codex app-server daemon.

The daemon (`codex app-server daemon start`) listens on a Unix socket and speaks
JSON-RPC over WebSocket. `codex`, `codex resume` and `codex agents` all attach to
it, so a thread started here can be joined later from a terminal, including
while a turn is running (probe metrics/probes/codex-daemon-thread-host-20261026.json).

Standard library only. One connection per operation: connect, initialize, a few
requests, close. Turns keep running on the daemon after the client leaves.
"""
from __future__ import annotations

import base64
import hashlib
import json
import os
import socket
import struct
import time
from pathlib import Path

WS_GUID = "258EAFA5-E914-47DA-95CA-C5AB0DC85B11"


class AppServerError(Exception):
    """Transport failure or a JSON-RPC error response. `rpc` holds the error object."""

    def __init__(self, detail: str, rpc: dict | None = None):
        super().__init__(detail)
        self.detail, self.rpc = detail, rpc


def default_socket(codex_home: Path | None = None) -> Path:
    if os.environ.get("ROLE_HOST_CODEX_SOCKET"):
        return Path(os.environ["ROLE_HOST_CODEX_SOCKET"])
    home = Path(codex_home or os.environ.get("CODEX_HOME") or Path.home() / ".codex")
    return home / "app-server-control" / "app-server-control.sock"


class Connection:
    def __init__(self, sock_path: Path, *, client_name="role-host", timeout=30.0):
        self.timeout, self.n, self.buf = timeout, 0, b""
        self.notifications: list[dict] = []
        self.s = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
        self.s.settimeout(timeout)
        try:
            self.s.connect(str(sock_path))
            self._handshake()
        except OSError as e:
            self.s.close()
            raise AppServerError(f"cannot reach app-server at {sock_path}: {e}")
        self.call("initialize", {"clientInfo": {"name": client_name, "version": "1"}})
        self.notify("initialized")

    # ---- websocket framing (RFC 6455, client side) ----
    def _handshake(self):
        key = base64.b64encode(os.urandom(16)).decode()
        self.s.sendall((f"GET / HTTP/1.1\r\nHost: localhost\r\nUpgrade: websocket\r\n"
                        f"Connection: Upgrade\r\nSec-WebSocket-Key: {key}\r\n"
                        f"Sec-WebSocket-Version: 13\r\n\r\n").encode())
        while b"\r\n\r\n" not in self.buf:
            chunk = self.s.recv(4096)
            if not chunk:
                raise AppServerError("app-server closed during handshake")
            self.buf += chunk
        head, self.buf = self.buf.split(b"\r\n\r\n", 1)
        lines = head.decode("latin-1").split("\r\n")
        if " 101 " not in lines[0] + " ":
            raise AppServerError(f"websocket upgrade refused: {lines[0]}")
        want = base64.b64encode(hashlib.sha1((key + WS_GUID).encode()).digest()).decode()
        accept = next((l.split(":", 1)[1].strip() for l in lines[1:]
                       if l.lower().startswith("sec-websocket-accept:")), None)
        if accept != want:
            raise AppServerError("websocket accept key mismatch")

    def _send_frame(self, opcode: int, payload: bytes):
        n = len(payload)
        head = bytes([0x80 | opcode])
        if n < 126:
            head += bytes([0x80 | n])
        elif n < 1 << 16:
            head += bytes([0x80 | 126]) + struct.pack("!H", n)
        else:
            head += bytes([0x80 | 127]) + struct.pack("!Q", n)
        mask = os.urandom(4)
        body = bytes(b ^ mask[i % 4] for i, b in enumerate(payload))
        self.s.sendall(head + mask + body)

    def _read_exact(self, n: int) -> bytes:
        while len(self.buf) < n:
            chunk = self.s.recv(65536)
            if not chunk:
                raise AppServerError("app-server closed the connection")
            self.buf += chunk
        out, self.buf = self.buf[:n], self.buf[n:]
        return out

    def _recv_message(self) -> str:
        parts = []
        while True:
            b0, b1 = self._read_exact(2)
            opcode, n = b0 & 0x0F, b1 & 0x7F
            if n == 126:
                n = struct.unpack("!H", self._read_exact(2))[0]
            elif n == 127:
                n = struct.unpack("!Q", self._read_exact(8))[0]
            mask = self._read_exact(4) if b1 & 0x80 else None
            data = self._read_exact(n)
            if mask:
                data = bytes(b ^ mask[i % 4] for i, b in enumerate(data))
            if opcode == 0x9:
                self._send_frame(0xA, data)
                continue
            if opcode == 0xA:
                continue
            if opcode == 0x8:
                raise AppServerError("app-server closed the websocket")
            parts.append(data)
            if b0 & 0x80:
                return b"".join(parts).decode()

    # ---- JSON-RPC ----
    def notify(self, method: str, params: dict | None = None):
        msg = {"method": method, **({"params": params} if params is not None else {})}
        self._send_frame(0x1, json.dumps(msg).encode())

    def call(self, method: str, params: dict | None = None, timeout: float | None = None) -> dict:
        self.n += 1
        i = self.n
        msg = {"id": i, "method": method, **({"params": params} if params is not None else {})}
        self._send_frame(0x1, json.dumps(msg).encode())
        deadline = time.monotonic() + (timeout or self.timeout)
        while True:
            remaining = deadline - time.monotonic()
            if remaining <= 0:
                raise AppServerError(f"no response to {method}")
            self.s.settimeout(remaining)
            try:
                m = json.loads(self._recv_message())
            except socket.timeout:
                raise AppServerError(f"no response to {method}")
            if m.get("id") == i and "method" not in m:
                if "error" in m:
                    raise AppServerError(f"{method}: {m['error'].get('message', m['error'])}", m["error"])
                return m.get("result") or {}
            if "method" in m and "id" not in m:
                self.notifications.append(m)
            # A server request (it has both id and method) is left unanswered:
            # this client is not a turn's approver and disconnects shortly.

    def close(self):
        try:
            self._send_frame(0x8, struct.pack("!H", 1000))
        except OSError:
            pass
        self.s.close()

    def __enter__(self):
        return self

    def __exit__(self, *exc):
        self.close()
