#!/usr/bin/env python3
"""Provider-free task-fixture launch, navigation, real mouse and restoration check."""
import errno
import fcntl
import os
import pty
import select
import struct
import subprocess
import sys
import termios
import time

binary = sys.argv[1]
master, slave = pty.openpty()
original = termios.tcgetattr(slave)
fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack("HHHH", 40, 128, 0, 0))
process = subprocess.Popen([binary, "--task-fixture", "pending"], stdin=slave,
                           stdout=slave, stderr=slave,
                           env={**os.environ, "TERM": "xterm-256color"})
captured = bytearray()


def read_for(seconds=0.25):
    end = time.monotonic() + seconds
    chunk = bytearray()
    while time.monotonic() < end:
        if select.select([master], [], [], 0.02)[0]:
            try:
                data = os.read(master, 65536)
            except OSError as error:
                if error.errno == errno.EIO:
                    break
                raise
            if not data:
                break
            chunk.extend(data)
    captured.extend(chunk)
    return bytes(chunk)


def press(data):
    os.write(master, data)
    return read_for()


try:
    start = read_for(0.5)
    assert b"PROVISIONAL TASK FIXTURE" in start and b"Overview" in start
    assert b"NEEDS ATTENTION" in start and b"No result published" in start
    press(b"\x10graph\r")
    # Real SGR mouse coordinates for question q1 in the 128x40 bound graph list.
    press(b"\x1b[<0;33;11M\x1b[<0;33;11m")
    opened = press(b"\x1b[<0;33;11M\x1b[<0;33;11m")
    assert b"Pending q1" in opened, "double-click did not open scoped Conversation"
    press(b"\x10task fixture stale\r")
    fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack("HHHH", 30, 80, 0, 0))
    import signal
    process.send_signal(signal.SIGWINCH)
    narrow = read_for(0.4)
    assert b"STALE" in captured and process.poll() is None
    press(b"\x10panel context\r")
    press(b"\x10task fixture refused\r")
    press(b"\x10lifecycle\r")
    assert b"capacity" in captured or b"refused" in captured
    press(b"\x03")
    process.wait(timeout=3)
    read_for()
    assert process.returncode == 0
    assert termios.tcgetattr(slave) == original, "terminal settings not restored"
    assert b"\x1b[?1049l" in captured, "alternate screen not restored"
    print("PASS task PTY: direct launch, scoped double-click, stale/refused, resize, restoration")
finally:
    if process.poll() is None:
        process.kill()
        process.wait()
    os.close(master)
    os.close(slave)
