#!/usr/bin/env python3
"""Drive one real terminal through collector updates and a failed refresh."""
import fcntl
import hashlib
import json
import os
from pathlib import Path
import pty
import re
import select
import shutil
import signal
import struct
import subprocess
import sys
import tempfile
import termios
import time

binary, evidence = map(Path, sys.argv[1:])
first = evidence / "controlled-observation-first.json"
second = evidence / "controlled-observation.json"
assert first.exists() and second.exists(), "run the collector checkpoint first"
with tempfile.TemporaryDirectory(prefix="operator-terminal-") as directory:
    path = Path(directory) / "observation.json"
    shutil.copyfile(first, path); path.chmod(0o600)
    master, slave = pty.openpty()
    fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack("HHHH", 40, 128, 0, 0))
    original = termios.tcgetattr(slave)
    process = subprocess.Popen([str(binary), "--observe", str(path)], stdin=slave, stdout=slave, stderr=slave, env={**os.environ, "TERM": "xterm-256color"})
    capture = bytearray()
    def until(needle, stage):
        received = bytearray()
        deadline = time.monotonic() + 8
        redraw = time.monotonic() + 1
        width = 128
        while time.monotonic() < deadline:
            # A resize forces a complete redraw; it does not request a source
            # refresh. This avoids treating Ratatui's cursor deltas as text.
            if time.monotonic() >= redraw:
                width = 129 if width == 128 else 128
                fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack("HHHH", 40, width, 0, 0))
                os.kill(process.pid, signal.SIGWINCH)
                redraw = time.monotonic() + 1
            ready, _, _ = select.select([master], [], [], .2)
            if ready:
                chunk = os.read(master, 65536)
                received.extend(chunk); capture.extend(chunk)
                plain = re.sub(rb'\x1b\[[0-?]*[ -/]*[@-~]', b'', received)
                if re.sub(rb'\s+', b'', needle) in re.sub(rb'\s+', b'', plain):
                    (evidence / f"terminal-{stage}-redraw.txt").write_bytes(plain)
                    return
            if process.poll() is not None: break
        raise AssertionError(f"terminal did not display {needle!r}: {received[-3000:]!r}")
    try:
        until(b"observation one", "first")
        replacement = path.with_suffix(".next")
        shutil.copyfile(second, replacement); replacement.chmod(0o600); replacement.replace(path)
        until(b"observation two", "updated")
        path.unlink()
        until(b"Refresh failed", "outage")
        os.write(master, b"q")
        assert process.wait(timeout=5) == 0
        while select.select([master], [], [], .1)[0]:
            capture.extend(os.read(master, 65536))
        restored = termios.tcgetattr(slave)
        assert original == restored, "terminal settings not restored"
        assert b"\x1b[?1049l" in capture, "alternate screen was not restored"
        (evidence / "terminal-pty.bin").write_bytes(capture)
        for width, height in [(128, 40), (60, 24)]:
            result = subprocess.run([str(binary), "--observe", str(second), "--observe-snapshot", str(width), str(height)], capture_output=True, check=True)
            (evidence / f"terminal-{width}x{height}.txt").write_bytes(result.stdout)
            assert b"Controlled interface run" in result.stdout
            assert b"observation two" in result.stdout
        (evidence / "terminal-checkpoint.json").write_text(json.dumps({"checks": ["one running TUI displays first source observation", "atomic collector update refreshes same TUI without refresh input", "read failure retains last snapshot with explicit warning", "q exits and restores all terminal settings", "two frame sizes"], "binary": str(binary), "binary_sha256": hashlib.sha256(binary.read_bytes()).hexdigest(), "snapshot_sha256": hashlib.sha256(second.read_bytes()).hexdigest(), "harness_sha256": hashlib.sha256(Path(__file__).read_bytes()).hexdigest(), "qualification": "Controlled source; PTY uses resize redraws for text assertions, not a terminal emulator. No provider or production workload."}, indent=2) + "\n")
        print("Shared observation TUI checkpoint passed (automatic update, outage, terminal restoration, two frame sizes)")
    finally:
        if process.poll() is None:
            process.terminate(); process.wait(timeout=5)
        os.close(master); os.close(slave)
