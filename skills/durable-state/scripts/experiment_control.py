#!/usr/bin/env python3
"""Storage/transport only. Rust owns experiment meaning; no live host hooks.

All cooperating adapters serialize read/reduce/CAS/output under one filesystem
lock. The lock file is never unlinked. A crash releases it, leaving the retained
receipt honest about output whose emission/collection may be unknown.
"""
from __future__ import annotations

import argparse
import fcntl
import hashlib
import json
import os
import math
import select
import time
from pathlib import Path
import subprocess
import sys

from durable_state import DurableStateError, GitRefDurableStateStore, canonical

NAMESPACE = "experiment-control-v1"
MAX_BYTES = 8 * 1024 * 1024
HARD_MAX_BYTES = 32 * 1024 * 1024


def timeout_value(value):
    value = float(value)
    if not math.isfinite(value) or not 0 < value <= 300:
        raise ValueError("timeout must be finite, positive and at most 300 seconds")
    return value


def emit_stdout(value, timeout):
    """Bound backpressure under the lock; partial output is uncertain delivery."""
    data = memoryview((json.dumps(value, separators=(",", ":")) + "\n").encode())
    fd = sys.stdout.fileno()
    blocking = os.get_blocking(fd)
    deadline = time.monotonic() + timeout_value(timeout)
    try:
        os.set_blocking(fd, False)
        while data:
            remaining = deadline - time.monotonic()
            if remaining <= 0:
                raise TimeoutError("output deadline exceeded; emission may be partial")
            if not select.select([], [fd], [], remaining)[1]:
                raise TimeoutError("output deadline exceeded; emission may be partial")
            try:
                written = os.write(fd, data)
            except BlockingIOError:
                continue
            data = data[written:]
    finally:
        os.set_blocking(fd, blocking)


def execute(repository: Path, experiment: str, binary: Path, sha256: str,
            request: bytes, expected_revision: str | None, emit, lock_timeout: float = 30) -> None:
    lock_timeout = timeout_value(lock_timeout)
    if not experiment or len(experiment) > 128 or any(
            not (c.isascii() and (c.isalnum() or c in "-_.")) for c in experiment):
        raise ValueError("invalid experiment identifier")
    if len(request) > MAX_BYTES:
        raise ValueError("request exceeds size bound")
    store = GitRefDurableStateStore(repository, NAMESPACE)
    common = Path(store._git(["rev-parse", "--path-format=absolute", "--git-common-dir"]).decode().strip())
    locks = common / "work-engine-locks" / NAMESPACE
    locks.mkdir(parents=True, exist_ok=True)
    # This sidecar is exclusion machinery, never a second state owner. All
    # linked worktrees resolve the same common directory and lock identity.
    with (locks / hashlib.sha256(experiment.encode()).hexdigest()).open("a+b") as lock:
        deadline = time.monotonic() + lock_timeout
        while True:
            try:
                fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
                break
            except BlockingIOError:
                remaining = deadline - time.monotonic()
                if remaining <= 0:
                    raise TimeoutError("experiment lock deadline exceeded")
                time.sleep(min(0.02, remaining))
        current = store.read(experiment)
        revision = current.revision if current else None
        if expected_revision is not None and expected_revision != (revision or "absent"):
            raise ValueError("experiment revision conflict")
        payload = current.payload if current else b"null"
        if len(payload) > HARD_MAX_BYTES:
            raise ValueError("stored experiment exceeds size bound")
        # Preserve raw request syntax so the Rust strict decoder rejects duplicate
        # keys rather than Python silently changing their meaning first.
        wire = b'{"state":' + payload + b',"request":' + request + b'}'
        with binary.open("rb") as executable:
            if hashlib.file_digest(executable, "sha256").hexdigest() != sha256:
                raise ValueError("experiment binary digest mismatch")
            executable.seek(0)
            # Bind execution to the opened inode (Linux). Provisioned binaries
            # must still be immutable: concurrent in-place writes are unsupported.
            process = subprocess.run([f"/proc/self/fd/{executable.fileno()}"],
                input=wire, stdout=subprocess.PIPE, stderr=subprocess.PIPE,
                pass_fds=(executable.fileno(),), timeout=30)
        if process.returncode:
            # Rust policy failures never include withheld payloads.
            raise ValueError(process.stderr.decode(errors="replace").strip()[:2000])
        if len(process.stdout) > HARD_MAX_BYTES * 3:
            raise ValueError("reducer output exceeds size bound")
        result = json.loads(process.stdout)
        if (set(result) != {"state", "result", "changed"}
                or not isinstance(result["changed"], bool)
                or result["state"].get("experiment") != experiment):
            raise ValueError("incompatible reducer output")
        next_payload = canonical(result["state"])
        if len(next_payload) > HARD_MAX_BYTES:
            raise ValueError("next experiment exceeds size bound")
        if result["changed"]:
            revision = store.publish(experiment, next_payload, revision).revision
        elif current is None or json.loads(current.payload) != result["state"]:
            raise ValueError("read-only reducer changed state")
        # Publication can succeed before output fails. Do not retry or claim
        # delivery; the issued receipt survives for explicit recovery.
        emit({"revision": revision, "result": result["result"]})


def main() -> int:
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument("--repository", type=Path, required=True)
    p.add_argument("--experiment", required=True)
    p.add_argument("--binary", type=Path, required=True)
    p.add_argument("--sha256", required=True)
    p.add_argument("--request", type=Path, help="JSON Request file; stdin if omitted")
    p.add_argument("--expected-revision", help="opaque revision, or 'absent' for initial registration")
    p.add_argument("--output-timeout-seconds", type=float, default=10,
                   help="stdout backpressure deadline, 0 < seconds <= 300")
    p.add_argument("--lock-timeout-seconds", type=float, default=30,
                   help="lock acquisition deadline, 0 < seconds <= 300")
    args = p.parse_args()
    try:
        timeout_value(args.output_timeout_seconds)
        timeout_value(args.lock_timeout_seconds)
        if args.request:
            with args.request.open("rb") as f:
                request = f.read(MAX_BYTES + 1)
        else:
            request = sys.stdin.buffer.read(MAX_BYTES + 1)
        def emit(value):
            emit_stdout(value, args.output_timeout_seconds)
        execute(args.repository, args.experiment, args.binary, args.sha256,
                request, args.expected_revision, emit, args.lock_timeout_seconds)
        return 0
    except (ValueError, KeyError, TypeError, OSError, DurableStateError,
            subprocess.TimeoutExpired) as error:
        print(f"error: {error}", file=sys.stderr)
        return 1
    except KeyboardInterrupt:
        return 130


if __name__ == "__main__":
    raise SystemExit(main())
