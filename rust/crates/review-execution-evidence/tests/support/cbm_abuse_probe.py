"""Adversarial RP0 relay frames, executed only inside the reviewer namespace."""

import json
import pathlib
import socket
import time


def exchange(sock, method, params=None, request_id=1):
    request = {"jsonrpc": "2.0", "id": request_id, "method": method}
    if params is not None:
        request["params"] = params
    sock.sendall(json.dumps(request).encode() + b"\n")
    line = bytearray()
    while not line.endswith(b"\n"):
        chunk = sock.recv(1)
        if not chunk:
            raise RuntimeError("relay closed before the expected reply")
        line.extend(chunk)
    return json.loads(line)


def main():
    mode = pathlib.Path("/session/probe-mode").read_text().strip()
    marker = pathlib.Path("/scratch/probe-abuse.json")
    marker.write_text(json.dumps({"mode": mode, "stage": "started"}) + "\n")
    sock = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
    sock.settimeout(25)
    sock.connect("/cbm/socket/analysis.sock")
    marker.write_text(json.dumps({"mode": mode, "stage": "connected"}) + "\n")
    if mode == "early_disconnect":
        sock.close()
    else:
        assert "result" in exchange(sock, "initialize", {
            "protocolVersion": "2024-11-05", "capabilities": {},
            "clientInfo": {"name": "rp0-abuse-peer", "version": "1"},
        })
        marker.write_text(json.dumps({"mode": mode, "stage": "initialized"}) + "\n")
        sock.sendall(b'{"jsonrpc":"2.0","method":"notifications/initialized"}\n')
        status = exchange(sock, "tools/call", {"name": "index_status", "arguments": {
            "project": "rp0_fixture", "diagnostics": "none", "format": "json"}}, 2)
        assert "result" in status
        marker.write_text(json.dumps({"mode": mode, "stage": "status_complete"}) + "\n")
        if mode == "oversize":
            marker.write_text(json.dumps({"mode": mode, "stage": "oversize_attempted"}) + "\n")
            try:
                sock.sendall(b"{" + b"x" * 65536 + b"}\n")
                closed = sock.recv(1) == b""
            except (BrokenPipeError, ConnectionResetError):
                closed = True
            assert closed, "oversize frame was not closed"
        elif mode == "flood":
            marker.write_text(json.dumps({"mode": mode, "stage": "flood_attempted"}) + "\n")
            sent = 0
            for request_id in range(3, 5000):
                try:
                    sock.sendall(json.dumps({"jsonrpc": "2.0", "id": request_id,
                                             "method": "tools/list"}).encode() + b"\n")
                    sent += 1
                except (BrokenPipeError, socket.timeout, ConnectionResetError):
                    break
            assert sent > 64, sent
            time.sleep(6)
        else:
            raise RuntimeError("unexpected abuse mode")
        sock.close()
    marker.write_text(json.dumps({"mode": mode, "stage": "completed"}) + "\n")
    print(json.dumps({"mode": mode}))


if __name__ == "__main__":
    main()
