#!/usr/bin/env python3
"""Semantically blind durable publication with opaque CAS revisions."""

from __future__ import annotations

import base64
import hashlib
import json
import os
import signal
import select
import sys
import time
import subprocess
from pathlib import Path
from typing import NamedTuple, Protocol


class DurableStateError(ValueError):
    pass


class DurableStateDeadline(DurableStateError):
    """Conditional preparation expired before final publication entry."""


class DurableStateUnknown(DurableStateError):
    """Final publication entry was interrupted; never reinterpret as CAS refusal."""


class _GitRefused(DurableStateError):
    """Owned transaction stopped before this publisher sent any commit bytes."""


def _keeper_main() -> None:
    """Private exec bootstrap: no forked Python state from concurrent callers.

    This process, not the board caller, owns Git and its absolute lifetime. It
    keeps the fence's open file description through group termination/reaping.
    A lost caller does not terminate a publication that can still finish within
    its original deadline. No domain state or authority lives in this helper.
    """
    import ctypes
    request = json.loads(sys.stdin.buffer.read())
    deadline = request["deadline"]
    staged = request["staged"]
    commit_sent = not staged
    child = None
    stdout, stderr = bytearray(), bytearray()
    timed_out = False
    cleaned = False
    failure = None
    returncode = None
    try:
        # Adopt cooperative orphaned hooks so closure includes their collection.
        if ctypes.CDLL(None, use_errno=True).prctl(36, 1, 0, 0, 0) != 0:
            raise OSError("unable to establish Git descendant ownership")
        if deadline is not None and time.monotonic() >= deadline:
            timed_out = True
        else:
            payload = base64.b64decode(request["input"], validate=True)
            if staged:
                if not payload.endswith(b"prepare\ncommit\n"):
                    raise ValueError("unsupported private transaction framing")
                payload = payload[:-len(b"commit\n")]
            child = subprocess.Popen(request["argv"], stdin=subprocess.PIPE,
                                     stdout=subprocess.PIPE, stderr=subprocess.PIPE,
                                     pass_fds=tuple(request["pass_fds"]), start_new_session=True)
            for stream in (child.stdin, child.stdout, child.stderr):
                os.set_blocking(stream.fileno(), False)
            pending = memoryview(payload)
            readers = {child.stdout.fileno(): stdout, child.stderr.fileno(): stderr}
            stdin_open = True
            while True:
                # WNOWAIT reserves the leader PID/PGID until killpg, even when
                # every hook has already closed the captured streams.
                info = os.waitid(os.P_PID, child.pid, os.WEXITED | os.WNOHANG | os.WNOWAIT)
                if info is not None:
                    break
                if deadline is not None and time.monotonic() >= deadline:
                    timed_out = True
                    break
                if staged and not pending and not commit_sent and b"prepare: ok" in stdout.splitlines():
                    pending = memoryview(b"commit\n")
                    # Set before the first write: even a partial write is unknown.
                    commit_sent = True
                if not pending and stdin_open and (not staged or commit_sent):
                    child.stdin.close()
                    stdin_open = False
                ready, writable, _ = select.select(list(readers),
                    [child.stdin.fileno()] if stdin_open and pending else [], [], .01)
                for fd in ready:
                    chunk = os.read(fd, 65536)
                    if chunk:
                        readers[fd].extend(chunk)
                    else:
                        del readers[fd]
                if writable:
                    pending = pending[os.write(writable[0], pending):]
    except BaseException as error:
        failure = str(error)
    finally:
        cleanup_started = time.monotonic()
        if child is not None:
            # Never reap/poll before this signal. The unreaped child reserves
            # the owned group identity; no late killpg is attempted afterwards.
            try:
                os.killpg(child.pid, signal.SIGKILL)
            except ProcessLookupError:
                pass
            end = time.monotonic() + 1.0  # cleanup grace, not a renewed effect deadline
            try:
                returncode = child.wait(timeout=max(.001, end-time.monotonic()))
                while True:
                    try:
                        pid, _ = os.waitpid(-1, os.WNOHANG)
                    except ChildProcessError:
                        cleaned = True
                        break
                    if not pid:
                        if time.monotonic() >= end:
                            break
                        time.sleep(.005)
                # Group closure makes pipe draining finite, including early EOF.
                for stream, output in ((child.stdout, stdout), (child.stderr, stderr)):
                    while time.monotonic() < end:
                        try:
                            chunk = os.read(stream.fileno(), 65536)
                        except BlockingIOError:
                            break
                        if not chunk:
                            break
                        output.extend(chunk)
            except BaseException as error:
                failure = str(error)
            for stream in (child.stdin, child.stdout, child.stderr):
                if stream is not None:
                    stream.close()
        else:
            cleaned = True  # no Git was launched
        report = {"returncode": returncode, "stdout": base64.b64encode(stdout).decode(),
                  "stderr": base64.b64encode(stderr).decode(), "timed_out": timed_out,
                  "cleaned": cleaned, "commit_sent": commit_sent, "failure": failure,
                  "keeper_pid": os.getpid(), "git_pid": child.pid if child is not None else None,
                  "effect_deadline_monotonic": deadline, "cleanup_grace_seconds": 1.0,
                  "cleanup_elapsed_seconds": time.monotonic()-cleanup_started,
                  "deadline_overrun_seconds": max(0, time.monotonic()-deadline) if deadline is not None else 0}
        try:
            sys.stdout.buffer.write(canonical(report))
            sys.stdout.buffer.flush()
        except BrokenPipeError:
            pass  # caller died; all effects were already closed above


class DurableValue(NamedTuple):
    key: str
    revision: str
    payload: bytes
    integrity_sha256: str
    adapter: str
    location: str
    predecessor_revision: str | None


class DurableHistoryPage(NamedTuple):
    values: tuple[DurableValue, ...]
    next_cursor: str | None


class DurableStateStore(Protocol):
    def read(self, key: str) -> DurableValue | None: ...
    def read_revision(self, key: str, revision: str) -> DurableValue: ...
    def list_revisions(self, key: str, cursor: str | None,
                       limit: int) -> DurableHistoryPage: ...
    def publish(self, key: str, payload: bytes, expected_revision: str | None) -> DurableValue: ...


def canonical(value: object) -> bytes:
    return json.dumps(value, sort_keys=True, separators=(",", ":")).encode()


class GitRefDurableStateStore:
    adapter = "git-ref-v1"

    def __init__(self, repository: Path, namespace: str = "live-state", *,
                 deadline: float | None = None) -> None:
        self.repository = repository.resolve()
        if not namespace or any(part in {"", ".", ".."} for part in namespace.split("/")):
            raise DurableStateError("namespace must contain nonempty safe path components")
        self.namespace = namespace
        self._deadline = deadline
        # Protected backend process inheritance; no semantic meaning or DTO knob.
        inherited = os.environ.get("WORK_ENGINE_REGISTRY_FENCE_FD")
        self._pass_fds = () if inherited is None else (int(inherited),)
        for fd in self._pass_fds:
            os.fstat(fd)  # Refuse a broken handoff rather than silently dropping it.
        self._git(["rev-parse", "--git-dir"])
        self._git(["check-ref-format", f"refs/work-engine/{namespace}/{'0' * 64}"])
        self._git(["check-ref-format",
                   f"refs/work-engine/{namespace}-history/{'0' * 64}/{'0' * 40}"])

    def _run_git(self, args: list[str], input_bytes: bytes | None,
                 deadline: float | None) -> subprocess.CompletedProcess:
        deadline = self._deadline if deadline is None else deadline
        argv = ["git", "-C", str(self.repository), *args]
        staged = deadline is None and args[0] == "update-ref"
        if deadline is None and not staged:
            return subprocess.run(argv, input=input_bytes, stdout=subprocess.PIPE,
                                  stderr=subprocess.PIPE, check=False, pass_fds=self._pass_fds)
        if deadline is not None and time.monotonic() >= deadline:
            raise DurableStateDeadline("conditional reply deadline expired")
        request = canonical({"argv": argv, "input": base64.b64encode(input_bytes or b"").decode(),
                             "deadline": deadline, "staged": staged, "pass_fds": self._pass_fds})
        bootstrap = "import runpy,sys; runpy.run_path(sys.argv[1])['_keeper_main']()"
        # Fresh interpreter, separate session, and complete request before Git
        # entry. The keeper survives both original Rust and board-Python death.
        with subprocess.Popen([sys.executable, "-I", "-c", bootstrap, str(Path(__file__).resolve())],
                stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE,
                pass_fds=self._pass_fds, start_new_session=True) as keeper:
            try:
                raw, errors = keeper.communicate(request)
            except (OSError, KeyboardInterrupt) as error:
                # An IPC failure after spawning the owner cannot establish
                # refusal. The context manager still collects its completion.
                raise DurableStateUnknown("Git keeper communication interrupted; outcome unknown") from error
            try:
                report = json.loads(raw)
                if (keeper.returncode or report["cleaned"] is not True
                        or type(report["commit_sent"]) is not bool
                        or type(report["timed_out"]) is not bool):
                    raise ValueError("Git lifetime closure unavailable")
                stdout = base64.b64decode(report["stdout"], validate=True)
                stderr = base64.b64decode(report["stderr"], validate=True)
                if report["timed_out"]:
                    raise subprocess.TimeoutExpired(argv, 0)
                if report["failure"] and (not staged or report["commit_sent"]):
                    raise ValueError(report["failure"])
                result = subprocess.CompletedProcess(argv, report["returncode"], stdout, stderr)
                result.commit_sent = report["commit_sent"]
                return result
            except (KeyError, TypeError, ValueError) as error:
                raise DurableStateUnknown("Git keeper result unavailable; outcome unknown") from error

    def _git(self, args: list[str], input_bytes: bytes | None = None,
             *, deadline: float | None = None) -> bytes:
        # Last owned entry check follows all preparation, never a Git clock predicate.
        try:
            result = self._run_git(args, input_bytes, deadline)
        except subprocess.TimeoutExpired as error:
            if args[0] == "update-ref":
                raise DurableStateUnknown("git deadline interrupted; publication outcome unknown") from error
            raise DurableStateDeadline("conditional reply deadline expired during preparation") from error
        except OSError as error:
            raise DurableStateError(f"unable to execute git: {error}") from error
        if args[0] == "update-ref" and getattr(result, "commit_sent", True) is False:
            raise _GitRefused("Git refused before commit was sent: " + result.stderr.decode(errors="replace").strip())
        if result.returncode:
            detail = result.stderr.decode(errors="replace").strip()
            if args[0] == "update-ref":
                # A committed hook can die after the refs already changed. Neither
                # a changed head nor a failed ACK establishes refusal by this CAS.
                raise DurableStateUnknown("final git failure; publication outcome unknown: " + detail)
            raise DurableStateError(detail)
        return result.stdout

    def _ref(self, key: str) -> str:
        if not isinstance(key, str) or not key:
            raise DurableStateError("key must be a nonempty string")
        return f"refs/work-engine/{self.namespace}/{hashlib.sha256(key.encode()).hexdigest()}"

    def _history_ref(self, key: str, revision: str) -> str:
        key_digest = hashlib.sha256(key.encode()).hexdigest()
        return f"refs/work-engine/{self.namespace}-history/{key_digest}/{revision}"

    def _current(self, ref: str, *, deadline: float | None = None) -> str | None:
        try:
            result = self._run_git(["rev-parse", "--verify", "--quiet", ref], None, deadline)
        except subprocess.TimeoutExpired as error:
            raise DurableStateDeadline("conditional reply deadline expired during preparation") from error
        except OSError as error:
            raise DurableStateError(f"unable to execute git: {error}") from error
        if result.returncode == 0:
            return result.stdout.decode().strip()
        if result.returncode == 1:
            return None
        raise DurableStateError(result.stderr.decode(errors="replace").strip() or "unable to inspect durable state ref")

    def read(self, key: str) -> DurableValue | None:
        ref = self._ref(key)
        revision = self._current(ref)
        if revision is None:
            return None
        return self.read_revision(key, revision)

    def read_revision(self, key: str, revision: str) -> DurableValue:
        if not isinstance(revision, str) or not revision:
            raise DurableStateError("revision must be a nonempty string")
        try:
            raw = self._git(["cat-file", "blob", revision])
        except DurableStateDeadline:
            raise
        except DurableStateError as error:
            raise DurableStateError("durable state revision is unavailable") from error
        try:
            envelope = json.loads(raw)
            payload = base64.b64decode(envelope["payload_base64"], validate=True)
        except (KeyError, TypeError, ValueError, json.JSONDecodeError) as error:
            raise DurableStateError("stored durable value is malformed") from error
        digest = hashlib.sha256(payload).hexdigest()
        schema_version = envelope.get("schema_version")
        predecessor = envelope.get("predecessor_revision") if schema_version == 2 else None
        if predecessor is not None and (not isinstance(predecessor, str) or not predecessor):
            raise DurableStateError("stored durable value has an invalid predecessor revision")
        expected = {"schema_version": schema_version, "key": key,
                    "payload_base64": base64.b64encode(payload).decode(),
                    "payload_sha256": digest}
        if schema_version == 2:
            expected["predecessor_revision"] = predecessor
        elif schema_version != 1:
            raise DurableStateError("stored durable value has an unsupported schema version")
        if envelope != expected:
            raise DurableStateError("stored durable value failed key or integrity validation")
        return DurableValue(key, revision, payload, digest, self.adapter,
                            self._ref(key), predecessor)

    def list_revisions(self, key: str, cursor: str | None,
                       limit: int) -> DurableHistoryPage:
        if isinstance(limit, bool) or not isinstance(limit, int) or not 1 <= limit <= 100:
            raise DurableStateError("history limit must be an integer from 1 through 100")
        current = self._current(self._ref(key))
        if current is None:
            if cursor is not None:
                raise DurableStateError("history cursor is not reachable from current state")
            return DurableHistoryPage((), None)

        revision = current
        seen: set[str] = set()
        if cursor is not None:
            if not isinstance(cursor, str) or not cursor:
                raise DurableStateError("history cursor must be a nonempty revision")
            while revision != cursor:
                if revision in seen:
                    raise DurableStateError("durable state history contains a cycle")
                seen.add(revision)
                value = self.read_revision(key, revision)
                revision = value.predecessor_revision
                if revision is None:
                    raise DurableStateError("history cursor is not reachable from current state")
            cursor_value = self.read_revision(key, revision)
            revision = cursor_value.predecessor_revision

        values: list[DurableValue] = []
        while revision is not None and len(values) < limit:
            if revision in seen:
                raise DurableStateError("durable state history contains a cycle")
            seen.add(revision)
            value = self.read_revision(key, revision)
            values.append(value)
            revision = value.predecessor_revision
        next_cursor = values[-1].revision if revision is not None and values else None
        return DurableHistoryPage(tuple(values), next_cursor)

    def publish(self, key: str, payload: bytes,
                expected_revision: str | None, *, deadline: float | None = None) -> DurableValue:
        if not isinstance(payload, bytes):
            raise DurableStateError("payload must be bytes")
        ref = self._ref(key)
        if self._current(ref, deadline=deadline) != expected_revision:
            raise DurableStateError("durable state revision conflict")
        digest = hashlib.sha256(payload).hexdigest()
        envelope = canonical({"schema_version": 2, "key": key,
                              "payload_base64": base64.b64encode(payload).decode(),
                              "payload_sha256": digest,
                              "predecessor_revision": expected_revision})
        revision = self._git(["hash-object", "-w", "--stdin"], envelope, deadline=deadline).decode().strip()
        commands = ["start", f"create {self._history_ref(key, revision)} {revision}"]
        if (expected_revision is not None
                and self._current(self._history_ref(key, expected_revision), deadline=deadline) is None):
            # A schema-v1 head predates retention refs. Adopt it atomically with
            # the first linked publication instead of fabricating earlier ancestry.
            commands.append(
                f"create {self._history_ref(key, expected_revision)} {expected_revision}")
        commands.extend([
            f"update {ref} {revision} {expected_revision or '0' * len(revision)}",
            "prepare", "commit", "",
        ])
        try:
            self._git(["update-ref", "--stdin"], "\n".join(commands).encode(), deadline=deadline)
        except _GitRefused as error:
            # Changed head only selects conflict AFTER positive no-commit proof.
            # No commit bytes were sent and the owned Git group is stopped.
            if self._current(ref, deadline=deadline) != expected_revision:
                raise DurableStateError("durable state revision conflict") from error
            # Hook-controlled stderr must not impersonate the board's legacy
            # conflict marker. Keep detail in the cause, not its decision text.
            raise DurableStateError("git transaction refused before commit") from error
        return DurableValue(key, revision, payload, digest, self.adapter, ref,
                            expected_revision)
