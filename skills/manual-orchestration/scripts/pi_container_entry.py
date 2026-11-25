#!/usr/bin/env python3
"""Container entry point for one Pi turn started by role_host.py.

The container has no network. This relays 127.0.0.1:8080, where Pi's openai
provider is pointed, to the host proxy's Unix socket mounted into the container,
then runs Pi with the prompt on stdin and its events and stderr in the mounted
turn directory. Pi's exit status is written atomically when it exits; a killed
container leaves none.

This runs as the container's PID 1 (Docker's --init cannot be injected when the
host's /usr is mounted read-only). It passes SIGTERM and SIGINT to Pi and reaps
every orphan. When it exits, the kernel kills whatever is left in the container,
including tool processes Pi started detached.

usage: pi_container_entry.py SOCKET PROMPT EVENTS STDERR EXIT -- PI_ARGV...
"""
import json
import os
import signal
import socket
import subprocess
import sys
import threading

PORT = 8080
# Claude subscription lane: a public, non-authenticating credential so Claude Code
# selects its OAuth (subscription) request form. The host proxy swaps in the real
# bearer; this value must equal pi_container.CLAUDE_SDK_PLACEHOLDER.
CLAUDE_SDK_PLACEHOLDER = "sk-ant-oat01-" + "D" * 95


def write_claude_credential(home):
    """Write the synthetic credential into the turn's tmpfs home, never anything real."""
    path = os.path.join(home, ".claude")
    os.makedirs(path, mode=0o700, exist_ok=True)
    far = 4102444800000  # 2100-01-01: Claude Code must never try to refresh it
    record = {"claudeAiOauth": {"accessToken": CLAUDE_SDK_PLACEHOLDER,
                                "refreshToken": "sk-ant-ort01-" + "D" * 95,
                                "expiresAt": far, "scopes": ["user:inference", "user:profile"],
                                "subscriptionType": "max"}}
    fd = os.open(os.path.join(path, ".credentials.json"), os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600)
    with os.fdopen(fd, "w") as fh:
        json.dump(record, fh)


def pump(src, dst):
    try:
        while True:
            data = src.recv(65536)
            if not data:
                break
            dst.sendall(data)
    except OSError:
        pass
    finally:
        for s in (src, dst):
            try:
                s.shutdown(socket.SHUT_RDWR)
            except OSError:
                pass


def relay(server, path):
    while True:
        client, _ = server.accept()
        upstream = socket.socket(socket.AF_UNIX)
        try:
            upstream.connect(path)
        except OSError:
            client.close()
            upstream.close()
            continue
        threading.Thread(target=pump, args=(client, upstream), daemon=True).start()
        threading.Thread(target=pump, args=(upstream, client), daemon=True).start()


def main(argv):
    if len(argv) < 7 or argv[5] != "--":
        print(__doc__, file=sys.stderr)
        return 64
    sock_path, prompt, events, stderr, status = argv[:5]
    pi_argv = argv[6:]
    if os.environ.get("WORK_ENGINE_CLAUDE_SDK_CREDENTIAL") == "synthetic":
        write_claude_credential(os.environ["HOME"])
    server = socket.socket()
    server.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
    server.bind(("127.0.0.1", PORT))
    server.listen(64)
    threading.Thread(target=relay, args=(server, sock_path), daemon=True).start()
    with open(prompt, "rb") as fin, open(events, "wb") as fout, open(stderr, "wb") as ferr:
        child = subprocess.Popen(pi_argv, stdin=fin, stdout=fout, stderr=ferr)

    def forward(signum, _frame):
        try:
            os.kill(child.pid, signum)
        except ProcessLookupError:
            pass

    signal.signal(signal.SIGTERM, forward)
    signal.signal(signal.SIGINT, forward)
    rc = None
    while rc is None:
        try:
            pid, wstatus = os.waitpid(-1, 0)  # reap orphans too; signals retry it
        except ChildProcessError:
            rc = 255
            break
        if pid == child.pid:
            rc = os.waitstatus_to_exitcode(wstatus)
    child.returncode = rc  # reaped here, not by Popen
    tmp = status + ".tmp"
    with open(tmp, "w") as fh:
        fh.write(f"{rc}\n")
    os.replace(tmp, status)
    return 0 if rc == 0 else 1


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
