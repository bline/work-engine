#!/usr/bin/env python3
"""PTY join of the UI and an owned read_only S4 service, including restart."""
import errno
import fcntl
import json
import os
import pty
import re
import select
import signal
import struct
import subprocess
import sys
import tempfile
import termios
import time
from pathlib import Path


def run(ui_binary, service_binary):
    with tempfile.TemporaryDirectory(prefix="we-ui-s4-") as directory:
        root = Path(directory)
        os.chmod(root, 0o700)
        socket = root / "read.sock"
        config = root / "service.json"
        config.write_text(json.dumps({
            "profile": "read_only", "store_root": str(root / "store"),
            "socket_path": str(socket), "peer_socket_path": str(root / "absent-peer"),
            "trusted_issuer": "issuer-ui-smoke",
            "principal": {"uid": os.getuid(), "principal_ref": "principal-ui-smoke",
                          "producer_ref": "producer-ui-smoke"},
            "subject_id": "subject-ui-smoke", "context_generation": "context-ui-smoke",
            "build_id": "build-ui-smoke", "proof_run_id": "proof-ui-smoke",
            "grants": [], "max_executions": 1, "max_wait_ms": 1000,
        }))
        service = None
        process = None
        master = slave = None

        def start_service():
            nonlocal service
            service = subprocess.Popen([service_binary, str(config)],
                                       stdout=subprocess.DEVNULL, stderr=subprocess.PIPE)
            until = time.monotonic() + 5
            while not socket.exists():
                if service.poll() is not None:
                    raise AssertionError(f"service exited: {service.stderr.read().decode()}")
                assert time.monotonic() < until, "service socket did not appear"
                time.sleep(0.01)

        def stop_service():
            nonlocal service
            assert service is not None
            service.terminate()
            service.wait(timeout=6)
            assert service.returncode == 0, f"service stop status {service.returncode}"
            service.stderr.close()
            service = None

        def read_for(seconds):
            until = time.monotonic() + seconds
            data = bytearray()
            while time.monotonic() < until:
                if select.select([master], [], [], 0.02)[0]:
                    try:
                        part = os.read(master, 65536)
                    except OSError as error:
                        if error.errno == errno.EIO:
                            break
                        raise
                    if not part:
                        break
                    data.extend(part)
            return bytes(data)

        def press(keys):
            os.write(master, keys)
            return read_for(0.18)

        def wait_text(needle, seconds=3, initial=b""):
            until = time.monotonic() + seconds
            data = bytearray(initial)
            while needle not in data and time.monotonic() < until:
                data.extend(read_for(0.1))
            assert needle in data, f"missing {needle!r}; debug {re.findall(rb'.{0,70}(?:U1|Refresh|refresh|connection|CLIENT|busy|complete).{0,100}', bytes(data))[-30:]!r}"
            return bytes(data)

        def command(name):
            press(b"\x10" + name.encode())
            return press(b"\r")

        def full_frame():
            fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack("HHHH", 24, 80, 0, 0))
            os.kill(process.pid, signal.SIGWINCH)
            read_for(0.15)
            fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack("HHHH", 40, 128, 0, 0))
            os.kill(process.pid, signal.SIGWINCH)
            return read_for(0.18)

        try:
            start_service()
            master, slave = pty.openpty()
            original = termios.tcgetattr(slave)
            fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack("HHHH", 40, 128, 0, 0))
            process = subprocess.Popen([ui_binary, "--connect", str(socket), "--subject", "subject-ui-smoke"],
                                       stdin=slave, stdout=slave, stderr=slave,
                                       env={**os.environ, "TERM": "xterm-256color"})
            startup = wait_text(b"coherent read")
            assert b"subject-ui-smoke" in startup and b"read-only" in startup
            assert b"store" in startup and b"context-ui-smoke" in startup
            assert process.poll() is None
            command("connection refresh")
            read_for(0.7)
            refreshed = full_frame()
            assert b"coherent read" in refreshed and b"read-only" in refreshed

            stop_service()
            command("connection refresh")
            read_for(0.7)
            unavailable_frame = full_frame()
            assert b"last known only" in unavailable_frame, "outage lost the last known label"
            assert b"disconnected" not in unavailable_frame.lower(), "outage must not become explicit disconnect"
            start_service()
            command("connection reconnect")
            read_for(0.7)
            restored = full_frame()
            assert b"coherent read" in restored and b"read-only" in restored
            command("connection disconnect")
            read_for(0.1)
            disconnected = full_frame()
            assert b"explicitly disconnected" in disconnected
            assert b"last known only" in full_frame()
            press(b"\x03")
            process.wait(timeout=3)
            assert process.returncode == 0, f"UI exit status {process.returncode}"
            assert termios.tcgetattr(slave) == original, "terminal modes not restored"
            stop_service()
            print("PASS: read_only S4 service/UI PTY join, bounded refresh outage, same-store restart/reconnect, explicit disconnect, clean service and UI exit")
        finally:
            if process is not None and process.poll() is None:
                process.kill()
                process.wait(timeout=3)
            if service is not None:
                service.kill()
                service.wait(timeout=3)
                service.stderr.close()
            if master is not None:
                os.close(master)
            if slave is not None:
                os.close(slave)


if __name__ == "__main__":
    if len(sys.argv) != 3:
        raise SystemExit("usage: connection_smoke.py /path/to/work-engine-terminal-ui /path/to/lifecycle-service")
    run(os.path.abspath(sys.argv[1]), os.path.abspath(sys.argv[2]))
