#!/usr/bin/env python3
"""Linux/macOS PTY check of the built binary. No engine or third-party modules."""
import base64
import re
import errno
import fcntl
import os
import pty
import select
import signal
import struct
import subprocess
import sys
import termios
import time


def run(binary):
    master, slave = pty.openpty()
    original = termios.tcgetattr(slave)
    fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack("HHHH", 40, 128, 0, 0))
    process = subprocess.Popen(
        [binary], stdin=slave, stdout=slave, stderr=slave,
        env={**os.environ, "TERM": "xterm-256color"},
    )
    captured = bytearray()

    def read_for(seconds):
        deadline = time.monotonic() + seconds
        chunk = bytearray()
        while time.monotonic() < deadline:
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

    def press(keys):
        os.write(master, keys)
        return read_for(0.15)

    try:
        startup = read_for(0.5)
        assert b"WORK ENGINE" in startup and b"FIXTURE" in startup, "no initial screen"
        assert b"\x1b[?1049h" in startup, "alternate screen not entered"
        assert b"\x1b[>1u" in startup, "enhanced keyboard reporting not requested"
        assert b"\x1b[?1006h" in startup, "SGR mouse reporting not enabled"
        # Real SGR mouse -> framework -> source selection -> OSC 52, captured in
        # this private PTY. No host clipboard is touched by the test.
        press(b"\x10conversation\r")
        press(b"\x1b[<0;2;10M")
        press(b"\x1b[<32;11;10M")
        press(b"\x1b[<0;11;10m")
        copied = press(b"\x03")
        osc = re.search(rb"\x1b\]52;c;([A-Za-z0-9+/=]+)\x1b\\", copied)
        assert osc, "mouse-selected Markdown did not request clipboard copy"
        assert base64.b64decode(osc.group(1)) == b"The standa", "copy included chrome or wrong source range"
        assert process.poll() is None, "copy must not quit"
        press(b"\x1b")  # Clear selection.
        # A long message extends well beyond the viewport. Hold at the top
        # edge without additional mouse motion, then copy text from above it.
        history = "\n".join(f"edge row {n:02}" for n in range(80)).encode()
        press(b"\x1b[200~" + history + b"\x1b[201~\r")
        press(b"\x1b")  # Sent message stays at the bottom; composer keeps keyboard input.
        press(b"\x1b[<0;70;31M")
        press(b"\x1b[<32;2;6M")
        read_for(1.8)  # Stationary pointer: only timer ticks can scroll.
        press(b"\x1b[<0;2;6m")
        copied = press(b"\x03")
        osc = re.search(rb"\x1b\]52;c;([A-Za-z0-9+/=]+)\x1b\\", copied)
        assert osc and b"edge row 45" in base64.b64decode(osc.group(1)), "stationary edge drag did not select offscreen history"
        press(b"\x1b")
        press(b"\x10dashboard\r")
        # Waiting objective -> review -> decision -> conversation -> explicit recipient.
        press(b"\x10follow dependency\r\x10follow dependency\r\x10conversation\r\x1b[18~")  # F7 = xterm CSI 18~
        press(b"\x1b[200~Keep history inspect-only.\nRetain my draft.\x1b[201~")
        newline = press(b"\x1b[13;2uExtra line")  # Shift-Enter via keyboard protocol.
        assert b"Simulated locally" not in newline, "Shift-Enter must not send"
        newline = press(b"\x1b\rFallback line")  # Legacy Alt-Enter.
        assert b"Simulated locally" not in newline, "Alt-Enter must not send"
        # Historical view blocks send, then live view permits the local simulation.
        press(b"\x1b[15~\r")  # F5, Enter: historical send remains blocked.
        press(b"\x1b[15~\r")  # Return live and simulate send with Enter.
        press(b"\x1b[200~A second unsent draft.\x1b[201~")
        large = "\n".join(f"Pasted trace row {n}" for n in range(40)).encode()
        folded = press(b"\x1b[200~" + large + b"\x1b[201~")
        assert b"[Paste" in folded and b"#1" in folded, "folded token missing from terminal update"
        inspected = press(b"\x0f")  # Ctrl-O inspects the full retained draft.
        assert b"Full draft" in inspected, "paste inspection should open"
        press(b"\x1b")
        palette = press(b"\x10")  # Ctrl-P; open without touching the draft.
        assert b"Commands" in palette, "command picker should open"
        press(b"detail evid\t\r")  # Complete and run a local view command.
        palette = press(b"\x10")
        assert b"Commands" in palette, "command picker should reopen"
        press(b"\x1b")  # Cancel back to the preserved draft.
        fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack("HHHH", 24, 80, 0, 0))
        os.kill(process.pid, signal.SIGWINCH)
        read_for(0.2)
        prompt = press(b"\x03")
        assert b"Exit prototype" in prompt, "nonempty draft should prompt on Ctrl-C"
        press(b"n")
        assert process.poll() is None, "cancel exit should retain running UI"
        press(b"\x03y")
        process.wait(timeout=3)
        read_for(0.1)
        assert process.returncode == 0, f"exit status {process.returncode}"
        assert b"panicked" not in captured, "Rust panic"
        assert b"\x1b[?1049l" in captured, "alternate screen not restored"
        assert b"\x1b[?1006l" in captured, "mouse reporting not disabled"
        assert b"\x1b[?2004l" in captured, "bracketed paste not disabled"
        assert b"\x1b[<1u" in captured, "enhanced keyboard mode not popped"
        assert termios.tcgetattr(slave) == original, "terminal modes not restored"
        print("PASS: PTY startup, mouse selection/edge autoscroll/Markdown clipboard, journey keys, paste folding/inspection, command picker, resize, draft exit/cancel, clean exit, terminal restoration")
        print("Semantic journey assertions are covered separately by cargo test.")
    finally:
        if process.poll() is None:
            process.kill()
            process.wait(timeout=3)
        os.close(master)
        os.close(slave)


if __name__ == "__main__":
    if len(sys.argv) != 2:
        raise SystemExit("usage: pty_smoke.py /absolute/path/to/work-engine-terminal-ui")
    run(os.path.abspath(sys.argv[1]))
