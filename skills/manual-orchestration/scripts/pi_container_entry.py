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
import os
import signal
import socket
import subprocess
import sys
import threading

PORT = 8080


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
