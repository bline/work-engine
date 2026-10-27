#!/usr/bin/env python3
"""Run native Claude Code with narrow, provenance-bearing quota failover."""

from __future__ import annotations

import argparse
import collections
from collections import Counter
from datetime import datetime, timezone
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import signal
import subprocess
import sys
import tempfile
import threading
import time
from typing import Any, Callable, Sequence


SCHEMA_VERSION = 1
DEFAULT_OPENROUTER_BASE_URL = "https://openrouter.ai/api"
DEFAULT_OPENROUTER_MODEL = "anthropic/claude-sonnet-5-20260630"
QUOTA_PHRASES = (
    "you've hit your limit",
    "you have hit your limit",
    "usage limit reached",
    "session limit reached",
    "session quota",
)
# Opt-in exact-model guard: launcher exit codes that cannot be mistaken for a
# native Claude exit status being zero.
EXIT_GUARD_VIOLATION = 41
EXIT_GUARD_INCOMPLETE = 42
EXIT_GUARD_UNAVAILABLE = 43
EXIT_NATIVE_RESULT_ERROR = 44
EXIT_TIMEOUT = 124
MAX_STREAM_LINE_BYTES = 16 * 1024 * 1024
DRAIN_SECONDS = 5.0
DEFAULT_STOP_GRACE_SECONDS = 5.0
FLUSH_SECONDS = 2.0
DEFAULT_FORWARD_BACKLOG_BYTES = 64 * 1024 * 1024
SYNTHETIC_MODEL = "<synthetic>"
EXACT_MODEL_PATTERN = re.compile(r"^claude-(?=.*\d)[a-z0-9][a-z0-9._-]*$")
GUARD_LIMITS = (
    "Detection reads stream fields after the child has emitted them; the guard "
    "stops the owned process tree but does not establish that no tool effect "
    "preceded detection.",
    "Scoped settings narrow model selection; they are not proof of the model "
    "that served a message. Only assistant and stream_event message fields count "
    "as observations; session init and result modelUsage are recorded as "
    "metadata and, on --resume, modelUsage is cumulative history.",
)
RATE_LIMIT_REASONS = (
    "rate limit",
    "rate_limit",
    "quota",
    "usage limit",
    "session limit",
    "capacity",
)


class TransportError(ValueError):
    """Raised when a requested transport cannot be run safely."""


def read_routing_attestation(
    path: Path,
    *,
    expected_model: str,
    expected_key_hash: str,
    max_age_seconds: int,
) -> dict[str, Any]:
    try:
        value = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        raise TransportError(f"invalid routing attestation: {error}") from error
    if not isinstance(value, dict):
        raise TransportError("routing attestation must be a JSON object")
    required = {
        "schema_version",
        "kind",
        "observed_at",
        "guardrail_id",
        "key_hash",
        "allowed_providers",
        "allowed_models",
        "assignment_guardrail_id",
    }
    if set(value) != required:
        raise TransportError(
            "routing attestation fields must exactly match the version-1 contract"
        )
    if value["schema_version"] != 1 or value["kind"] != "openrouter-key-guardrail":
        raise TransportError("unsupported routing attestation contract")
    if value["allowed_providers"] != ["anthropic"]:
        raise TransportError("routing attestation must allow only provider 'anthropic'")
    if value["allowed_models"] != [expected_model]:
        raise TransportError("routing attestation must allow only the exact requested model")
    if not expected_key_hash or value["key_hash"] != expected_key_hash:
        raise TransportError("routing attestation does not bind the selected API key hash")
    if value["guardrail_id"] != value["assignment_guardrail_id"]:
        raise TransportError("routing attestation does not bind the key to the guardrail")
    try:
        observed_at = datetime.fromisoformat(value["observed_at"].replace("Z", "+00:00"))
    except (AttributeError, ValueError) as error:
        raise TransportError("routing attestation observed_at must be ISO-8601") from error
    if observed_at.tzinfo is None:
        raise TransportError("routing attestation observed_at must include a timezone")
    age = (datetime.now(timezone.utc) - observed_at.astimezone(timezone.utc)).total_seconds()
    if age < -60 or age > max_age_seconds:
        raise TransportError("routing attestation is outside the allowed freshness window")
    return {
        "path": str(path),
        "sha256": _sha256_bytes(path.read_bytes()),
        "kind": value["kind"],
        "observed_at": value["observed_at"],
        "guardrail_id": value["guardrail_id"],
        "key_hash": value["key_hash"],
        "allowed_providers": value["allowed_providers"],
        "allowed_models": value["allowed_models"],
    }


def _canonical_json(value: Any) -> bytes:
    return json.dumps(
        value, sort_keys=True, separators=(",", ":"), ensure_ascii=False
    ).encode("utf-8")


def _sha256_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def _sha256_text(value: str) -> str:
    return _sha256_bytes(value.encode("utf-8"))


def sha256_directory(path: Path) -> str:
    """Digest one frozen config tree without retaining any of its contents."""
    if not path.is_dir():
        raise TransportError(f"Claude config directory does not exist: {path}")
    digest = hashlib.sha256()
    for child in sorted(path.rglob("*"), key=lambda item: item.relative_to(path).as_posix()):
        relative = child.relative_to(path).as_posix()
        if child.is_symlink():
            kind = b"symlink"
            payload = os.readlink(child).encode("utf-8")
        elif child.is_file():
            kind = b"file"
            payload = child.read_bytes()
        elif child.is_dir():
            kind = b"directory"
            payload = b""
        else:
            raise TransportError(f"unsupported entry in Claude config directory: {child}")
        digest.update(kind + b"\0" + relative.encode("utf-8") + b"\0")
        digest.update(hashlib.sha256(payload).digest())
    return digest.hexdigest()


def classify_quota_failure(returncode: int, stdout: str, stderr: str) -> str | None:
    """Return the narrow quota signature that permits failover, if any."""
    if returncode == 0:
        return None
    combined = f"{stdout}\n{stderr}".lower()
    for phrase in QUOTA_PHRASES:
        if phrase in combined:
            return phrase
    has_429 = re.search(r"(?:\bhttp\D{0,12})?\b429\b", combined) is not None
    if has_429:
        for reason in RATE_LIMIT_REASONS:
            if reason in combined:
                return f"429:{reason}"
    if "rate_limit_error" in combined:
        return "rate_limit_error"
    return None


def _has_flag(command: Sequence[str], *flags: str) -> bool:
    return any(token in flags for token in command)


def _session_reference(command: Sequence[str]) -> tuple[str | None, str | None]:
    for index, token in enumerate(command):
        if token in {"--resume", "-r"}:
            value = command[index + 1] if index + 1 < len(command) else None
            return "resume", value
        if token == "--session-id":
            value = command[index + 1] if index + 1 < len(command) else None
            return "new", value
    return None, None


def _requested_model(command: Sequence[str]) -> str | None:
    for index, token in enumerate(command):
        if token == "--model" and index + 1 < len(command):
            return command[index + 1]
        if token.startswith("--model="):
            return token.split("=", 1)[1]
    return None


def _observed_models(output: str) -> list[str]:
    """Extract model identifiers from Claude JSON without retaining response text."""
    documents: list[Any] = []
    try:
        documents.append(json.loads(output))
    except json.JSONDecodeError:
        for line in output.splitlines():
            try:
                documents.append(json.loads(line))
            except json.JSONDecodeError:
                continue

    models: set[str] = set()

    def visit(value: Any) -> None:
        if isinstance(value, dict):
            for key, child in value.items():
                if key in {"model", "model_name"} and isinstance(child, str):
                    models.add(child)
                elif key in {"modelUsage", "model_usage"} and isinstance(child, dict):
                    models.update(str(model) for model in child)
                visit(child)
        elif isinstance(value, list):
            for child in value:
                visit(child)

    for document in documents:
        visit(document)
    return sorted(models)


def _safe_command_shape(command: Sequence[str]) -> list[str]:
    """Expose executable and option names without copying prompts into receipts."""
    shape = [Path(command[0]).name]
    for token in command[1:]:
        if token.startswith("-"):
            shape.append(token.split("=", 1)[0])
    return shape


def _claude_version(executable: str, env: dict[str, str]) -> str | None:
    try:
        result = subprocess.run(
            [executable, "--version"],
            env=env,
            capture_output=True,
            text=True,
            timeout=10,
            check=False,
        )
    except (OSError, subprocess.SubprocessError):
        return None
    if result.returncode != 0:
        return None
    return result.stdout.strip() or result.stderr.strip() or None


def _anthropic_environment(source: dict[str, str]) -> dict[str, str]:
    env = dict(source)
    # Auto mode means "the ordinary native Claude route first" even if the
    # parent shell was previously used for an OpenRouter invocation.
    env.pop("ANTHROPIC_BASE_URL", None)
    env.pop("ANTHROPIC_AUTH_TOKEN", None)
    return env


def _openrouter_environment(
    source: dict[str, str], *, base_url: str, model: str
) -> dict[str, str]:
    key = source.get("OPENROUTER_API_KEY", "")
    if not key:
        raise TransportError("OPENROUTER_API_KEY is required for OpenRouter transport")
    env = dict(source)
    env["ANTHROPIC_BASE_URL"] = base_url
    env["ANTHROPIC_AUTH_TOKEN"] = key
    env["ANTHROPIC_API_KEY"] = ""
    env["ANTHROPIC_DEFAULT_SONNET_MODEL"] = model
    return env


def _run_attempt(
    command: Sequence[str],
    env: dict[str, str],
    *,
    transport: str,
    model: str | None,
    stdin_text: str | None = None,
    requested_upstream_provider: str | None = None,
) -> tuple[subprocess.CompletedProcess[str], dict[str, Any]]:
    started = time.monotonic()
    result = subprocess.run(
        list(command), env=env, input=stdin_text, capture_output=True, text=True, check=False
    )
    duration_ms = round((time.monotonic() - started) * 1000)
    quota_signature = classify_quota_failure(
        result.returncode, result.stdout, result.stderr
    )
    attempt = {
        "transport": transport,
        "harness": "claude-code",
        "gateway": "anthropic" if transport == "anthropic" else "openrouter",
        "requested_model": model,
        "requested_upstream_provider": requested_upstream_provider,
        "observed_models": _observed_models(result.stdout),
        "returncode": result.returncode,
        "duration_ms": duration_ms,
        "stdout_sha256": _sha256_text(result.stdout),
        "stderr_sha256": _sha256_text(result.stderr),
        "quota_signature": quota_signature,
    }
    return result, attempt


class GuardUnavailable(Exception):
    """The exact-model guard cannot be established, so the child must not start."""


class _ReceiptUnwritable(Exception):
    """The in-flight receipt could not be published; the running child is stopped."""


def _bounded(value: Any, limit: int = 256) -> Any:
    return value[:limit] if isinstance(value, str) else None


def _flag_values(command: Sequence[str], flag: str) -> list[str | None]:
    values: list[str | None] = []
    for index, token in enumerate(command):
        if token == flag:
            values.append(command[index + 1] if index + 1 < len(command) else None)
        elif token.startswith(flag + "="):
            values.append(token.split("=", 1)[1])
    return values


class ModelGuard:
    """Judge a stream-json transcript line by line against one exact model.

    Only assistant messages and stream_event message_start records count as model
    observations. System init, result modelUsage and harness-authored
    "<synthetic>" notices are recorded separately and never prove identity.
    A violation is a deviating observation; a defect is evidence that cannot be
    read as complete. Findings keep identifiers and metadata fields, never content.
    """

    VIOLATIONS = ("refusal", "refusal_fallback", "model_mismatch")
    TERMINAL_KINDS = ("assistant", "stream_event", "result")

    def __init__(self, model: str, trigger: Callable[[], None]) -> None:
        self.model = model
        self._trigger = trigger
        self.lines = self.bytes = self.malformed = self.oversize = 0
        self.assistant_models: Counter[str] = Counter()
        self.stream_event_models: Counter[str] = Counter()
        self.synthetic_messages = self.parent_tool_use_records = 0
        self.synthetic_errors: Counter[str] = Counter()
        self.init_models: list[str] = []
        self.usage_models: list[str] = []
        self.session_ids: set[str] = set()
        self.result_session_id: str | None = None
        self.result: dict[str, Any] | None = None
        self.violation: dict[str, Any] | None = None
        self.defect: dict[str, Any] | None = None
        self.lines_after_finding = 0
        self.finding_ms: int | None = None
        self._offset = 0
        self._t0 = time.monotonic()

    def observations(self) -> int:
        return sum(self.assistant_models.values()) + sum(self.stream_event_models.values())

    def native_result_status(self) -> str:
        return self.result["native_status"] if self.result else "absent"

    def _find(self, kinds: list[str], raw: bytes, event: dict[str, Any] | None, **extra: Any) -> None:
        slot = "violation" if kinds[0] in self.VIOLATIONS else "defect"
        if getattr(self, slot) is not None:
            return
        message = event.get("message") if event else None
        message = message if isinstance(message, dict) else {}
        if self.finding_ms is None:
            self.finding_ms = round((time.monotonic() - self._t0) * 1000)
        setattr(self, slot, {
            "kind": kinds[0],
            "kinds": kinds,
            "line": self.lines,
            "byte_offset": self._offset,
            "line_sha256": _sha256_bytes(raw),
            "event_type": _bounded(event.get("type")) if event else None,
            "subtype": _bounded(event.get("subtype")) if event else None,
            "message_id": _bounded(message.get("id")),
            "session_id": _bounded(event.get("session_id")) if event else None,
            "parent_tool_use_id": _bounded(event.get("parent_tool_use_id")) if event else None,
            **extra,
        })
        self._trigger()

    def feed(self, raw: bytes, *, oversize: bool = False) -> None:
        self.lines += 1
        if self.violation or self.defect:
            self.lines_after_finding += 1
        try:
            self._judge(raw, oversize)
        except Exception:  # an unreadable verdict must fail closed, not stop draining
            self._find(["guard_internal_error"], raw, None)
        finally:
            self.bytes += len(raw)
            self._offset += len(raw)

    def _judge(self, raw: bytes, oversize: bool) -> None:
        if oversize:
            self.oversize += 1
            return self._find(["oversize_line"], raw, None, size_bytes=len(raw))
        text = raw.strip()
        if not text:
            return
        try:
            event = json.loads(text)
        except (json.JSONDecodeError, UnicodeDecodeError):
            event = None
        if not isinstance(event, dict):
            self.malformed += 1
            return self._find(["malformed_stream"], raw, None, truncated=not raw.endswith(b"\n"))
        kind, subtype = event.get("type"), event.get("subtype")
        if not isinstance(kind, str):
            return self._find(["envelope_malformed"], raw, event)
        session_id = _bounded(event.get("session_id"), 64)
        if session_id:
            self.session_ids.add(session_id)
        if event.get("parent_tool_use_id"):
            self.parent_tool_use_records += 1  # a marker only: any record type, no role inferred
        if self.result is not None and kind in self.TERMINAL_KINDS:
            # The terminal result cannot vouch for activity that follows it.
            self._find(["post_terminal_event"], raw, event)
        if kind == "system":
            if subtype == "init":
                self.init_models.append(_bounded(event.get("model")) or "")
            elif isinstance(subtype, str) and ("fallback" in subtype or "refusal" in subtype):
                self._find(["refusal_fallback"], raw, event)
        elif kind == "assistant":
            message = event.get("message")
            if not isinstance(message, dict):
                return self._find(["envelope_malformed"], raw, event)
            if message.get("model") == SYNTHETIC_MODEL:
                self.synthetic_errors[_bounded(event.get("error")) or "none"] += 1
            self._observe(message.get("model"), message.get("stop_reason"),
                          self.assistant_models, raw, event, assistant=True)
        elif kind == "stream_event":
            inner = event.get("event")
            if not isinstance(inner, dict) or not isinstance(inner.get("type"), str):
                return self._find(["envelope_malformed"], raw, event)
            if inner["type"] == "message_start":
                message = inner.get("message")
                if not isinstance(message, dict):
                    return self._find(["envelope_malformed"], raw, event)
                self._observe(message.get("model"), None, self.stream_event_models, raw, event)
            elif inner["type"] == "message_delta":
                delta = inner.get("delta")
                if isinstance(delta, dict) and delta.get("stop_reason") == "refusal":
                    self._find(["refusal"], raw, event, stop_reason="refusal")
        elif kind == "result" and self.result is None:
            usage = event.get("modelUsage") or event.get("model_usage")
            self.usage_models = sorted(str(m) for m in usage) if isinstance(usage, dict) else []
            terminal = event.get("terminal_reason")
            well_formed = isinstance(subtype, str) and isinstance(event.get("is_error"), bool)
            self.result_session_id = session_id
            self.result = {
                "subtype": _bounded(subtype),
                "is_error": event.get("is_error") if isinstance(event.get("is_error"), bool) else None,
                "stop_reason": _bounded(event.get("stop_reason")),
                "terminal_reason": _bounded(terminal),
                "well_formed": well_formed,
                "native_status": ("malformed" if not well_formed else
                                  "error" if event["is_error"] or subtype != "success" else "success"),
                "line": self.lines,
            }
            if not well_formed:
                self._find(["result_malformed"], raw, event)
            if event.get("stop_reason") == "refusal" or (
                isinstance(terminal, str) and "refusal" in terminal
            ):
                self._find(["refusal"], raw, event, stop_reason=_bounded(event.get("stop_reason")))

    def _observe(self, model: Any, stop_reason: Any, counts: Counter[str], raw: bytes,
                 event: dict[str, Any], *, assistant: bool = False) -> None:
        kinds: list[str] = []
        if stop_reason == "refusal":
            kinds.append("refusal")
        if assistant and model == SYNTHETIC_MODEL:
            self.synthetic_messages += 1  # harness-authored notice, not a model claim
        elif not isinstance(model, str) or not model:
            self._find(["model_missing"], raw, event, observed_model=None)
        else:
            counts[model] += 1
            if model != self.model:
                kinds.append("model_mismatch")
        if kinds:
            self._find(kinds, raw, event, observed_model=_bounded(model),
                       observed_model_length=len(model) if isinstance(model, str) else None,
                       stop_reason=_bounded(stop_reason))


class _Tee:
    """Capture a stream and hand it to an inherited fd without blocking observation.

    put() never blocks: a consumer that stalls or closes its end costs only the
    writer thread. Capture (digest, size) and queueing happen under one lock, and
    close() freezes the accounts, so every captured byte is in exactly one of
    delivered, dropped, pending (queued, unwritten) or in flight (inside a write
    that had not returned) and the parts sum to the captured size. "Delivered"
    means accepted by the fd; it is not evidence the consumer read it, and an
    in-flight write may already have placed a prefix in the pipe.
    """

    def __init__(self, fd: int | None, backlog_limit: int) -> None:
        self._fd = fd
        self._limit = backlog_limit
        self._queue: collections.deque[bytes] = collections.deque()
        self._cond = threading.Condition()
        self._digest = hashlib.sha256()
        self._captured = self._delivered = self._dropped = 0
        self._queued = self._inflight = 0
        self._failed = self._closed = self._frozen = False
        self._thread = threading.Thread(target=self._run, daemon=True)
        self._thread.start()

    def put(self, data: bytes) -> None:
        with self._cond:
            if self._frozen:
                return
            self._digest.update(data)
            self._captured += len(data)
            if (self._fd is None or self._failed
                    or self._queued + self._inflight + len(data) > self._limit):
                self._dropped += len(data)
                return
            self._queue.append(data)
            self._queued += len(data)
            self._cond.notify()

    def close(self, timeout: float) -> dict[str, Any]:
        """Flush for up to `timeout`, freeze the accounts and return them."""
        with self._cond:
            self._closed = True
            self._cond.notify()
        self._thread.join(timeout)
        with self._cond:
            self._frozen = True
            pending = self._queued
            self._queue.clear()
            self._queued = 0
            self._cond.notify()
            undelivered = self._dropped + pending + self._inflight
            return {
                "complete": undelivered == 0 and not self._failed,
                "captured_bytes": self._captured,
                "delivered_bytes": self._delivered,
                "undelivered_bytes": undelivered,
                "dropped_bytes": self._dropped,
                "pending_bytes": pending,
                "in_flight_bytes": self._inflight,
                "sha256": self._digest.hexdigest(),
            }

    def _run(self) -> None:
        while True:
            with self._cond:
                while not self._queue and not self._closed and not self._frozen:
                    self._cond.wait()
                if self._frozen or not self._queue:
                    return
                data = self._queue.popleft()
                self._queued -= len(data)
                self._inflight = len(data)
            view = memoryview(data)
            try:
                while view:
                    written = os.write(self._fd, view)
                    view = view[written:]
                    with self._cond:
                        if self._frozen:
                            return
                        self._delivered += written
                        self._inflight -= written
            except OSError:
                with self._cond:
                    if not self._frozen:
                        self._failed = True
                        self._dropped += self._inflight + self._queued
                        self._inflight = self._queued = 0
                        self._queue.clear()
                return


def _fileno(stream: Any) -> int | None:
    try:
        return stream.fileno()
    except (AttributeError, OSError, ValueError):
        return None


def _proc_table() -> dict[int, tuple[int, int, str]]:
    """pid -> (ppid, session id, state) from /proc; empty where /proc is absent."""
    table: dict[int, tuple[int, int, str]] = {}
    try:
        names = [name for name in os.listdir("/proc") if name.isdigit()]
    except OSError:
        return table
    for name in names:
        try:
            with open(f"/proc/{name}/stat", encoding="utf-8", errors="replace") as handle:
                data = handle.read()
            fields = data[data.rindex(")") + 2:].split()
            table[int(name)] = (int(fields[1]), int(fields[3]), fields[0])
        except (OSError, ValueError, IndexError):
            continue
    return table


def _set_subreaper(enabled: bool) -> bool:
    """Make orphaned descendants reparent to this launcher so they stay discoverable."""
    if not sys.platform.startswith("linux"):
        return False
    try:
        import ctypes

        return ctypes.CDLL(None, use_errno=True).prctl(36, 1 if enabled else 0, 0, 0, 0) == 0
    except (OSError, AttributeError):
        return False


def _owned_pids(root: int) -> set[int]:
    """Live processes in the child's session or descended from it or this launcher.

    The launcher's only children are the native child and, with the subreaper
    set, descendants orphaned from it, so a double-fork or setsid escapee is
    still reachable by ancestry. Zombie orphans are reaped here.
    """
    table = _proc_table()
    me = os.getpid()
    for pid, (ppid, _, state) in list(table.items()):
        if state == "Z" and ppid == me and pid != root:
            try:
                os.waitpid(pid, os.WNOHANG)
            except OSError:
                pass
    owned = {pid for pid, (ppid, sid, state) in table.items()
             if state != "Z" and (pid == root or sid == root or ppid == me)}
    grew = True
    while grew:
        grew = False
        for pid, (ppid, _, state) in table.items():
            if ppid in owned and pid not in owned and state != "Z":
                owned.add(pid)
                grew = True
    owned.discard(me)
    return owned


def _stop_tree(proc: subprocess.Popen[bytes], grace: float) -> dict[str, Any]:
    """TERM then KILL the child's session and every descendant found, escapees included."""
    started = time.monotonic()
    seen = _owned_pids(proc.pid)
    signals: list[str] = []

    def alive(pids: set[int]) -> set[int]:
        table = _proc_table()
        return {pid for pid in pids if pid in table and table[pid][2] != "Z"}

    for sig, wait in ((signal.SIGTERM, grace), (signal.SIGKILL, 5.0)):
        seen |= _owned_pids(proc.pid)
        if not alive(seen) and proc.poll() is not None:
            break
        signals.append(sig.name)
        if proc.poll() is None:  # a reaped pid could be reused: never signal its group
            try:
                os.killpg(proc.pid, sig)
            except (ProcessLookupError, PermissionError):
                pass
        for pid in alive(seen):
            try:
                os.kill(pid, sig)
            except (ProcessLookupError, PermissionError):
                pass
        deadline = time.monotonic() + wait
        while time.monotonic() < deadline:
            seen |= _owned_pids(proc.pid)
            if proc.poll() is not None and not alive(seen):
                break
            time.sleep(0.02)
    return {
        "signals": signals,
        "owned_processes_seen": len(seen),
        "survivor_pids": sorted(alive(seen)),
        "duration_ms": round((time.monotonic() - started) * 1000),
    }


def _prepare_guard_settings(command: Sequence[str], model: str, directory: Path
                            ) -> tuple[list[str], dict[str, Any]]:
    """Merge the guard keys into any caller --settings and pass one effective file."""
    hits = [(i, t) for i, t in enumerate(command) if t == "--settings" or t.startswith("--settings=")]
    if len(hits) > 1:
        raise GuardUnavailable("multiple --settings arguments are ambiguous")
    source_kind, source_sha, merged = "none", None, {}
    if hits:
        index, token = hits[0]
        value = token.split("=", 1)[1] if "=" in token else (
            command[index + 1] if index + 1 < len(command) else "")
        try:
            if value.lstrip().startswith("{"):
                raw, source_kind = value.encode("utf-8"), "inline"
            else:
                raw, source_kind = Path(value).read_bytes(), "file"
            merged = json.loads(raw)
        except (OSError, ValueError) as error:
            raise GuardUnavailable(f"caller --settings is unreadable: {error}") from error
        if not isinstance(merged, dict):
            raise GuardUnavailable("caller --settings must be a JSON object")
        source_sha = _sha256_bytes(raw)
    if merged.get("switchModelsOnFlag", False) is not False:
        raise GuardUnavailable("caller settings conflict: switchModelsOnFlag must be false")
    if merged.get("availableModels", [model]) != [model]:
        raise GuardUnavailable("caller settings conflict: availableModels must be exactly the model")
    merged["switchModelsOnFlag"] = False
    merged["availableModels"] = [model]
    payload = _canonical_json(merged)
    path = directory / "guard-settings.json"
    path.write_bytes(payload)
    path.chmod(0o600)
    rewritten = list(command)
    if hits and "=" in hits[0][1]:
        rewritten[hits[0][0]] = f"--settings={path}"
    elif hits:
        rewritten[hits[0][0] + 1] = str(path)
    else:
        rewritten[1:1] = ["--settings", str(path)]
    return rewritten, {
        "scope": "--settings flag layer; user and project settings are not modified",
        "source_kind": source_kind,
        "source_sha256": source_sha,
        "effective_sha256": _sha256_bytes(payload),
        "effective_sha256_after_run": None,
        "switchModelsOnFlag": False,
        "availableModels": [model],
        "_path": path,
    }


def _check_guard_command(args: argparse.Namespace, command: Sequence[str]) -> str:
    if args.transport != "anthropic":
        raise GuardUnavailable("the exact-model guard requires --transport anthropic")
    models = _flag_values(command, "--model")
    if len(models) != 1 or not models[0] or not EXACT_MODEL_PATTERN.match(models[0]):
        raise GuardUnavailable(
            "exactly one raw --model identifier (for example claude-sonnet-5-5) is "
            "required; aliases and bracketed variants cannot be verified"
        )
    if _flag_values(command, "--output-format") != ["stream-json"] or not _has_flag(command, "--verbose"):
        raise GuardUnavailable("the guard needs --output-format stream-json with --verbose")
    if _flag_values(command, "--fallback-model"):
        raise GuardUnavailable("--fallback-model contradicts an exact-model guard")
    return models[0]


def _guard_outcome(guard: ModelGuard, cause: str, *, drained: bool, delivered: bool,
                   settings_intact: bool) -> tuple[str, str | None]:
    """Model-identity verdict. Native result errors are judged separately."""
    if guard.violation:
        return "violation", guard.violation["kind"]
    if guard.defect:
        return "incomplete_evidence", guard.defect["kind"]
    if cause in {"timeout", "cancelled", "launcher_error"}:
        return "incomplete_evidence", cause
    for failed, reason in (
        (not guard.observations(), "no_exact_model_observation"),
        (guard.result is None, "no_result_event"),
        (not drained, "stream_not_drained"),
        (not delivered, "output_not_delivered"),
        (not settings_intact, "settings_modified"),
    ):
        if failed:
            return "incomplete_evidence", reason
    return "verified_exact", None


def _guard_verdict(outcome: str, *, native_returncode: int | None, stopped: bool,
                   native_status: str, cause: str, signum: int | None) -> tuple[int, str]:
    """Launcher exit code and receipt result; exit status never washes out a finding."""
    if outcome == "violation":
        return EXIT_GUARD_VIOLATION, "guard_violation"
    if native_returncode not in (0, None) and not stopped:
        return native_returncode, "failed"
    if outcome != "verified_exact":
        code = EXIT_TIMEOUT if cause == "timeout" else 128 + signum if signum else EXIT_GUARD_INCOMPLETE
        return code, "guard_incomplete"
    if native_status != "success":
        return EXIT_NATIVE_RESULT_ERROR, "failed"
    return 0, "success"


def _guarded_run(args: argparse.Namespace, command: list[str], env: dict[str, str],
                 stdin_text: str | None, receipt: dict[str, Any]) -> int:
    """Run the child under live exact-model observation; the receipt is always written."""
    guard_receipt: dict[str, Any] = {
        "schema_version": 1,
        "requested_model": None,
        "outcome": "unavailable",
        "reason": None,
        "limits": list(GUARD_LIMITS),
    }
    receipt["model_guard"] = guard_receipt
    receipt["request"]["exact_model_guard"] = True
    scratch = Path(tempfile.mkdtemp(prefix="claude-transport-guard-"))

    def unavailable(reason: str) -> int:
        guard_receipt.update(
            outcome="unavailable", reason=reason, acceptance="invalidated",
            native={"started": False, "returncode": None}, launcher_exit_code=EXIT_GUARD_UNAVAILABLE,
        )
        receipt["result"] = "guard_unavailable"
        _write_receipt(args.receipt, receipt)
        print(f"exact-model guard unavailable: {reason}", file=sys.stderr)
        return EXIT_GUARD_UNAVAILABLE

    subreaper = False
    proc: subprocess.Popen[bytes] | None = None
    stop: dict[str, Any] | None = None
    received: list[int] = []
    previous: dict[int, Any] = {}

    def on_signal(signum: int, _frame: Any) -> None:
        received.append(signum)
        wake.set()

    try:
        try:
            model = _check_guard_command(args, command)
            guard_receipt["requested_model"] = model
            effective_command, settings = _prepare_guard_settings(command, model, scratch)
        except GuardUnavailable as error:
            return unavailable(str(error))
        settings_path = settings.pop("_path")
        guard_receipt["settings"] = settings

        wake = threading.Event()
        guard = ModelGuard(model, wake.set)
        sys.stdout.flush()
        sys.stderr.flush()
        tees = {name: _Tee(_fileno(stream), args.guard_forward_backlog_bytes)
                for name, stream in (("stdout", sys.stdout), ("stderr", sys.stderr))}
        # Cancellation is recorded from here on, including while the child spawns.
        if threading.current_thread() is threading.main_thread():
            for signum in (signal.SIGINT, signal.SIGTERM, signal.SIGHUP):
                previous[signum] = signal.signal(signum, on_signal)
        subreaper = _set_subreaper(True)
        started = time.monotonic()
        try:
            proc = subprocess.Popen(
                effective_command, env=env, stdin=subprocess.PIPE if stdin_text is not None else None,
                stdout=subprocess.PIPE, stderr=subprocess.PIPE, start_new_session=True,
            )
        except OSError as error:
            return unavailable(f"native command could not start: {error.__class__.__name__}")

        # The launcher owns the child from here. Every later failure, including
        # one in publishing the in-flight receipt, still stops and collects it.
        cause, stop_started_ms = "exited", None
        failure: BaseException | None = None
        threads: list[threading.Thread] = []
        eof = {"stdout": False, "stderr": False}
        stopped_pump = {"stdout": False, "stderr": False}
        deadline = started + args.guard_timeout_seconds if args.guard_timeout_seconds else None

        def pump(name: str, read: Callable[[], None]) -> None:
            try:
                read()
                eof[name] = True
            finally:
                stopped_pump[name] = True
                wake.set()

        def read_stdout() -> None:
            stream = proc.stdout
            while True:
                raw = stream.readline(MAX_STREAM_LINE_BYTES + 1)
                if not raw:
                    return
                oversize = len(raw) > MAX_STREAM_LINE_BYTES and not raw.endswith(b"\n")
                guard.feed(raw[:64] if oversize else raw, oversize=oversize)  # judge first
                tees["stdout"].put(raw)
                while oversize:  # consume the rest of the line without judging it
                    raw = stream.readline(MAX_STREAM_LINE_BYTES)
                    tees["stdout"].put(raw)
                    oversize = bool(raw) and not raw.endswith(b"\n")

        def read_stderr() -> None:
            while True:
                chunk = proc.stderr.read1(65536)
                if not chunk:
                    return
                tees["stderr"].put(chunk)

        def write_stdin() -> None:
            try:
                proc.stdin.write(stdin_text.encode("utf-8"))
                proc.stdin.close()
            except (OSError, ValueError):
                pass

        def monitor() -> str:
            """Wait until the child has exited and its output is drained, or a stop condition.

            Every condition is read after wake.clear(), and each producer sets its
            durable fact before wake.set(), so no notification can be lost between
            the check and the wait. Cancellation and the deadline stay live through
            the drain.
            """
            exit_seen: float | None = None
            while True:
                wake.clear()
                now = time.monotonic()
                if received:
                    return "cancelled"
                if guard.violation or guard.defect:
                    return "guard_stop"
                if deadline is not None and now >= deadline:
                    return "timeout"
                if any(stopped_pump[n] and not eof[n] for n in eof):
                    raise RuntimeError("an output pump failed before end of stream")
                limits = [] if deadline is None else [deadline]
                if proc.poll() is not None:
                    exit_seen = now if exit_seen is None else exit_seen
                    if all(eof.values()) or now >= exit_seen + DRAIN_SECONDS:
                        return "exited"
                    limits.append(exit_seen + DRAIN_SECONDS)
                wake.wait(max(min(limits) - time.monotonic(), 0) if limits else None)

        def stop_owned_tree() -> dict[str, Any]:
            nonlocal stop_started_ms
            stop_started_ms = round((time.monotonic() - started) * 1000)
            return _stop_tree(proc, args.guard_stop_grace_seconds)

        try:
            guard_receipt.update(
                outcome="incomplete_evidence", reason="in_flight", acceptance="invalidated",
                native={"started": True, "pid": proc.pid, "returncode": None},
            )
            receipt["result"] = "pending"
            try:  # before the first stream byte, so an interrupted launcher leaves a record
                _write_receipt(args.receipt, receipt)
            except OSError as error:
                raise _ReceiptUnwritable(error) from error
            threads.extend([
                threading.Thread(target=pump, args=("stdout", read_stdout), daemon=True),
                threading.Thread(target=pump, args=("stderr", read_stderr), daemon=True),
                threading.Thread(target=lambda: (proc.wait(), wake.set()), daemon=True)])
            if stdin_text is not None:
                threads.append(threading.Thread(target=write_stdin, daemon=True))
            for thread in threads:
                thread.start()
            cause = monitor()
        except BaseException as error:
            failure, cause = error, "launcher_error"
        try:
            if cause != "exited":
                stop = stop_owned_tree()
            for thread in threads[:2]:  # the monitor has already waited out the drain
                thread.join(DRAIN_SECONDS if cause != "exited" else 0.5)
            if stop is None and (guard.violation or guard.defect):
                # Exited on its own: a late finding still owns any leftover tree.
                stop = stop_owned_tree()
        except BaseException as error:
            failure, cause = failure or error, "launcher_error"
            stop = stop or _stop_tree(proc, args.guard_stop_grace_seconds)
        drained = all(eof.values())
        if drained or not threads:
            for pipe in (proc.stdout, proc.stderr):
                pipe.close()
        native_returncode = proc.poll()
        remaining = sorted(_owned_pids(proc.pid) - {proc.pid}) if stop is None else None
        delivery = {name: tee.close(FLUSH_SECONDS) for name, tee in tees.items()}
        effective_after = (_sha256_bytes(settings_path.read_bytes())
                           if settings_path.exists() else None)
        settings["effective_sha256_after_run"] = effective_after
        expected_session = _session_reference(command)[1]
        session_mode = _session_reference(command)[0]

        def publish() -> tuple[int, OSError | None]:
            """Build the final receipt from the current state and try to write it.

            Returns the launcher exit and the write error, if any.
            """
            outcome, reason = _guard_outcome(
                guard, cause, drained=drained, delivered=all(d["complete"] for d in delivery.values()),
                settings_intact=effective_after == settings["effective_sha256"])
            stopped = cause != "exited" and stop is not None and bool(stop["signals"])
            signum = received[0] if received else None
            exit_code, receipt["result"] = _guard_verdict(
                outcome, native_returncode=native_returncode, stopped=stopped,
                native_status=guard.native_result_status(), cause=cause, signum=signum)
            if stop is None:
                collection = ("not_established" if not subreaper
                              else "leftover_processes" if remaining else "none_remaining")
            else:
                collection = ("survivors_remaining" if stop["survivor_pids"]
                              else "none_remaining" if subreaper else "not_established")
            guard_receipt.update(
                outcome=outcome,
                reason=reason,
                acceptance="not_invalidated_by_model_guard" if exit_code == 0 else "invalidated",
                tool_effects_before_stop="not_established",
                launcher_exit_code=exit_code,
                native={
                    "started": True,
                    "pid": proc.pid,
                    "returncode": native_returncode,
                    "stopped_by_launcher": stopped,
                    "result_status": guard.native_result_status(),
                },
                termination={
                    "cause": cause,
                    "launcher_error": failure.__class__.__name__ if failure else None,
                    "signals_received": [signal.Signals(n).name for n in received],
                    "first_finding_ms": guard.finding_ms,
                    "stop_started_ms": stop_started_ms,
                    "stop": stop,
                    "tree_discovery": "subreaper_ancestry_session" if subreaper else "ancestry_session_only",
                    "process_collection": collection,
                    "owned_processes_remaining_after_exit": remaining,
                },
                identity={
                    "requested_session_mode": session_mode,
                    "requested_session_id": expected_session,
                    "observed_session_ids": sorted(guard.session_ids),
                    "result_session_id": guard.result_session_id,
                    "session_identity": (
                        "not_requested" if expected_session is None
                        else "not_observed" if not guard.session_ids
                        else "matches" if guard.session_ids == {expected_session} else "differs"
                    ),
                },
                observations={
                    "assistant_models": dict(guard.assistant_models),
                    "stream_event_models": dict(guard.stream_event_models),
                    "synthetic_assistant_messages": guard.synthetic_messages,
                    "synthetic_error_categories": dict(guard.synthetic_errors),
                    "parent_tool_use_records": guard.parent_tool_use_records,
                    "metadata_only": {
                        "init_models": guard.init_models,
                        "result_model_usage_models": guard.usage_models,
                        "model_usage_scope": ("cumulative_conversation_history"
                                              if session_mode == "resume" else "this_session"),
                        "usage_models_other_than_requested": [
                            m for m in guard.usage_models if m != model],
                    },
                },
                violation=guard.violation,
                evidence_defect=guard.defect,
                stream={
                    "lines": guard.lines,
                    "bytes": delivery["stdout"]["captured_bytes"],
                    "stdout_sha256": delivery["stdout"]["sha256"],
                    "malformed_lines": guard.malformed,
                    "oversize_lines": guard.oversize,
                    "lines_after_first_finding": guard.lines_after_finding,
                    "result_event": guard.result,
                    "drained": drained,
                },
                delivery={
                    name: {key: value for key, value in account.items() if key != "sha256"}
                    for name, account in delivery.items()
                },
            )
            receipt["attempts"][:] = [{
                "transport": "anthropic",
                "harness": "claude-code",
                "gateway": "anthropic",
                "requested_model": model,
                "requested_upstream_provider": None,
                "observed_models": sorted(set(guard.assistant_models) | set(guard.stream_event_models)),
                "returncode": native_returncode,
                "duration_ms": round((time.monotonic() - started) * 1000),
                "stdout_sha256": delivery["stdout"]["sha256"],
                "stderr_sha256": delivery["stderr"]["sha256"],
                "stderr_size_bytes": delivery["stderr"]["captured_bytes"],
                "quota_signature": None,
            }]
            receipt["selected_transport"] = "anthropic" if exit_code == 0 else None
            try:
                _write_receipt(args.receipt, receipt)
            except OSError as error:
                return exit_code or EXIT_GUARD_INCOMPLETE, error
            return exit_code, None

        # Cancellation and the deadline stay in force until the receipt is
        # written: delivery and publication take time, so re-evaluate until a
        # pass saw no new signal. A receipt that cannot be written is a launcher
        # failure: the owned tree is stopped even though the native process has
        # already exited, then the receipt is tried once more with that cause.
        write_error: OSError | None = None
        retried = False
        while True:
            seen = len(received)
            if cause == "exited":
                if received:
                    cause = "cancelled"
                elif deadline is not None and time.monotonic() >= deadline:
                    cause = "timeout"
                if cause != "exited":
                    stop = stop or stop_owned_tree()
            exit_code, write_error = publish()
            if write_error is not None and not retried:
                retried = True
                failure = failure or _ReceiptUnwritable(write_error)
                cause = "launcher_error" if cause == "exited" else cause
                stop = stop or stop_owned_tree()
                continue
            # Publishing takes time: a deadline crossed inside it is re-evaluated once
            # (cause timeout is terminal, so this cannot repeat).
            late = cause == "exited" and deadline is not None and time.monotonic() >= deadline
            if len(received) == seen and not late:
                break
        if write_error is not None:
            print(f"exact-model guard: receipt could not be written ({write_error.__class__.__name__}); "
                  "the run is not accepted", file=sys.stderr)
        if failure is not None and not isinstance(failure, _ReceiptUnwritable):
            raise failure
        return exit_code
    finally:
        if proc is not None and stop is None and (proc.poll() is None or sys.exc_info()[0]):
            _stop_tree(proc, args.guard_stop_grace_seconds)  # an unfinished launcher never leaves its child
        for signum, handler in previous.items():
            signal.signal(signum, handler)
        if subreaper:
            _set_subreaper(False)
        shutil.rmtree(scratch, ignore_errors=True)


def _write_receipt(path: Path, receipt: dict[str, Any]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    payload = json.dumps(receipt, indent=2, sort_keys=True, ensure_ascii=False) + "\n"
    with tempfile.NamedTemporaryFile(
        "w", encoding="utf-8", dir=path.parent, delete=False
    ) as handle:
        temporary = Path(handle.name)
        try:
            handle.write(payload)
        except OSError:
            temporary.unlink(missing_ok=True)
            raise
    try:
        temporary.replace(path)
    except OSError:
        temporary.unlink(missing_ok=True)
        raise


def _base_receipt(
    args: argparse.Namespace,
    command: Sequence[str],
    *,
    stdin_sha256: str | None,
    stdin_size_bytes: int | None,
) -> dict[str, Any]:
    session_mode, session_id = _session_reference(command)
    script_digest = _sha256_bytes(Path(__file__).read_bytes())
    config_dir_value = os.environ.get("CLAUDE_CONFIG_DIR")
    config_digest = (
        sha256_directory(Path(config_dir_value)) if config_dir_value else None
    )
    return {
        "schema_version": SCHEMA_VERSION,
        "launcher": {
            "path": "skills/claude-recon-implementation/scripts/claude_transport.py",
            "sha256": script_digest,
        },
        "request": {
            "transport": args.transport,
            "continuity": args.continuity,
            "command_sha256": _sha256_bytes(_canonical_json(list(command))),
            "command_shape": _safe_command_shape(command),
            "stdin_sha256": stdin_sha256,
            "stdin_size_bytes": stdin_size_bytes,
            "openrouter_base_url": args.openrouter_base_url,
            "openrouter_model": args.openrouter_model,
            "batch_route_explicitly_allowed": args.allow_batch_route,
            "paid_failover_explicitly_allowed": args.allow_paid_failover,
            "anthropic_1p_required": args.require_anthropic_1p,
            "claude_config_dir": config_dir_value,
            "claude_config_dir_sha256_before": config_digest,
            # Presence only: never persist the bearer token or a correlatable
            # digest of it. This attests what the transport process received;
            # _anthropic_environment preserves this variable for the child.
            "claude_code_oauth_token_present": bool(
                os.environ.get("CLAUDE_CODE_OAUTH_TOKEN")
            ),
            "experimental_betas_disabled": (
                os.environ.get("CLAUDE_CODE_DISABLE_EXPERIMENTAL_BETAS") == "1"
            ),
            "terminal_title_disabled": (
                os.environ.get("CLAUDE_CODE_DISABLE_TERMINAL_TITLE") == "1"
            ),
            "session_mode": session_mode,
            "session_id": session_id,
        },
        "attempts": [],
        "selected_transport": None,
        "failover": {
            "attempted": False,
            "allowed": False,
            "reason": None,
            "continuity_claim": None,
        },
        "upstream_provider_observed": False,
        "routing_attestation": None,
        "result": "pending",
    }


def _emit(result: subprocess.CompletedProcess[str]) -> None:
    sys.stdout.write(result.stdout)
    sys.stderr.write(result.stderr)


def run(args: argparse.Namespace) -> int:
    command = list(args.command)
    if command and command[0] == "--":
        command = command[1:]
    if not command:
        raise TransportError("a native Claude command is required after '--'")

    if (args.stdin_file is None) != (args.stdin_sha256 is None):
        raise TransportError("--stdin-file and --stdin-sha256 must be supplied together")
    stdin_text = None
    stdin_size_bytes = None
    if args.stdin_file is not None:
        try:
            stdin_bytes = args.stdin_file.read_bytes()
        except OSError as error:
            raise TransportError(f"could not read stdin payload: {error}") from error
        observed_stdin_sha256 = _sha256_bytes(stdin_bytes)
        if observed_stdin_sha256 != args.stdin_sha256:
            raise TransportError("stdin payload SHA-256 differs from --stdin-sha256")
        try:
            stdin_text = stdin_bytes.decode("utf-8")
        except UnicodeDecodeError as error:
            raise TransportError("stdin payload must be valid UTF-8") from error
        stdin_size_bytes = len(stdin_bytes)

    receipt = _base_receipt(args, command, stdin_sha256=args.stdin_sha256,
                            stdin_size_bytes=stdin_size_bytes)
    anthropic_env = _anthropic_environment(os.environ)
    receipt["claude_version"] = _claude_version(command[0], anthropic_env)

    if args.openrouter_model.endswith(":batch") and not args.allow_batch_route:
        raise TransportError(
            "a :batch model requires --allow-batch-route after compatibility validation"
        )

    if args.require_anthropic_1p:
        if args.transport == "anthropic":
            raise TransportError(
                "--require-anthropic-1p applies only when OpenRouter is configured"
            )
        if args.routing_attestation is None:
            raise TransportError(
                "--require-anthropic-1p requires --routing-attestation"
            )
        receipt["routing_attestation"] = read_routing_attestation(
            args.routing_attestation,
            expected_model=args.openrouter_model,
            expected_key_hash=args.openrouter_key_hash,
            max_age_seconds=args.routing_attestation_max_age_seconds,
        )

    if args.transport == "auto" and args.continuity == "disposable":
        if not _has_flag(command, "--no-session-persistence"):
            raise TransportError(
                "disposable auto failover requires --no-session-persistence"
            )

    if args.exact_model_guard:
        return _guarded_run(args, command, anthropic_env, stdin_text, receipt)
    if (args.guard_timeout_seconds is not None
            or args.guard_stop_grace_seconds != DEFAULT_STOP_GRACE_SECONDS
            or args.guard_forward_backlog_bytes != DEFAULT_FORWARD_BACKLOG_BYTES):
        raise TransportError("--guard-* options require --exact-model-guard")

    if args.transport == "openrouter":
        env = _openrouter_environment(
            os.environ,
            base_url=args.openrouter_base_url,
            model=args.openrouter_model,
        )
        result, attempt = _run_attempt(
            command,
            env,
            transport="openrouter",
            model=args.openrouter_model,
            stdin_text=stdin_text,
            requested_upstream_provider=(
                "anthropic" if args.require_anthropic_1p else None
            ),
        )
        receipt["attempts"].append(attempt)
        receipt["selected_transport"] = "openrouter" if result.returncode == 0 else None
        receipt["result"] = "success" if result.returncode == 0 else "failed"
        _write_receipt(args.receipt, receipt)
        _emit(result)
        return result.returncode

    primary, primary_attempt = _run_attempt(
        command,
        anthropic_env,
        transport="anthropic",
        model=_requested_model(command),
        stdin_text=stdin_text,
    )
    receipt["attempts"].append(primary_attempt)
    if primary.returncode == 0 or args.transport == "anthropic":
        receipt["selected_transport"] = (
            "anthropic" if primary.returncode == 0 else None
        )
        receipt["result"] = "success" if primary.returncode == 0 else "failed"
        _write_receipt(args.receipt, receipt)
        _emit(primary)
        return primary.returncode

    quota_signature = primary_attempt["quota_signature"]
    if quota_signature is None:
        receipt["failover"]["reason"] = "primary failure was not a quota failure"
        receipt["result"] = "failed"
        _write_receipt(args.receipt, receipt)
        _emit(primary)
        return primary.returncode

    receipt["failover"]["reason"] = quota_signature
    session_mode, _ = _session_reference(command)
    if args.continuity == "retained":
        can_resume = session_mode == "resume" and args.allow_retained_resume_failover
        if not can_resume:
            receipt["failover"]["reason"] = (
                "retained review requires an explicit, verified --resume failover "
                "or truthful reviewer replacement"
            )
            receipt["failover"]["continuity_claim"] = "replacement_required"
            receipt["result"] = "failover_blocked"
            _write_receipt(args.receipt, receipt)
            _emit(primary)
            return primary.returncode
        receipt["failover"]["continuity_claim"] = (
            "same_local_claude_session_resumed_across_gateway"
        )
    else:
        receipt["failover"]["continuity_claim"] = "exact_disposable_request_replayed"

    if not args.allow_paid_failover:
        receipt["failover"]["reason"] = (
            "quota failure observed but paid failover was not explicitly allowed"
        )
        receipt["result"] = "paid_failover_not_authorized"
        _write_receipt(args.receipt, receipt)
        _emit(primary)
        return primary.returncode

    try:
        openrouter_env = _openrouter_environment(
            os.environ,
            base_url=args.openrouter_base_url,
            model=args.openrouter_model,
        )
    except TransportError as error:
        receipt["failover"]["reason"] = str(error)
        receipt["result"] = "failover_unavailable"
        _write_receipt(args.receipt, receipt)
        _emit(primary)
        return primary.returncode

    receipt["failover"]["attempted"] = True
    receipt["failover"]["allowed"] = True
    fallback, fallback_attempt = _run_attempt(
        command,
        openrouter_env,
        transport="openrouter",
        model=args.openrouter_model,
        stdin_text=stdin_text,
        requested_upstream_provider=(
            "anthropic" if args.require_anthropic_1p else None
        ),
    )
    receipt["attempts"].append(fallback_attempt)
    receipt["selected_transport"] = (
        "openrouter" if fallback.returncode == 0 else None
    )
    receipt["result"] = "success" if fallback.returncode == 0 else "failed"
    _write_receipt(args.receipt, receipt)
    _emit(fallback)
    return fallback.returncode


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        description="Run native Claude Code with bounded Anthropic/OpenRouter routing"
    )
    parser.add_argument(
        "--transport",
        choices=("auto", "anthropic", "openrouter"),
        default="auto",
    )
    parser.add_argument(
        "--continuity",
        choices=("disposable", "retained"),
        default="disposable",
    )
    parser.add_argument("--receipt", type=Path, required=True)
    parser.add_argument(
        "--stdin-file",
        type=Path,
        help="read the exact UTF-8 Claude prompt from this file and forward it on stdin",
    )
    parser.add_argument(
        "--stdin-sha256",
        help="required SHA-256 of --stdin-file bytes",
    )
    parser.add_argument(
        "--openrouter-base-url",
        default=DEFAULT_OPENROUTER_BASE_URL,
    )
    parser.add_argument(
        "--openrouter-model",
        default=os.environ.get(
            "CLAUDE_REVIEW_OPENROUTER_MODEL", DEFAULT_OPENROUTER_MODEL
        ),
    )
    parser.add_argument(
        "--allow-retained-resume-failover",
        action="store_true",
        help="allow only after cross-gateway --resume has been externally verified",
    )
    parser.add_argument(
        "--allow-paid-failover",
        action="store_true",
        help="authorize an auto-mode quota retry through the paid OpenRouter route",
    )
    parser.add_argument(
        "--allow-batch-route",
        action="store_true",
        help="allow an exact :batch model after native-Claude compatibility validation",
    )
    parser.add_argument(
        "--require-anthropic-1p",
        action="store_true",
        help="require a fresh Anthropic-only OpenRouter key-guardrail attestation",
    )
    parser.add_argument("--routing-attestation", type=Path)
    parser.add_argument(
        "--routing-attestation-max-age-seconds", type=int, default=900
    )
    parser.add_argument(
        "--openrouter-key-hash",
        default=os.environ.get("OPENROUTER_API_KEY_HASH", ""),
        help="non-secret OpenRouter key hash returned by the management API",
    )
    parser.add_argument(
        "--exact-model-guard",
        action="store_true",
        help="opt in: observe the stream-json transcript live and stop the native "
        "process tree on refusal, fallback or any assistant model other than the "
        "exact --model identifier; requires --transport anthropic",
    )
    parser.add_argument(
        "--guard-timeout-seconds",
        type=float,
        help="with --exact-model-guard, stop the owned process tree after this long",
    )
    parser.add_argument(
        "--guard-stop-grace-seconds",
        type=float,
        default=DEFAULT_STOP_GRACE_SECONDS,
        help="with --exact-model-guard, SIGTERM grace before SIGKILL",
    )
    parser.add_argument(
        "--guard-forward-backlog-bytes",
        type=int,
        default=DEFAULT_FORWARD_BACKLOG_BYTES,
        help="with --exact-model-guard, cap on stream bytes queued for a slow consumer; "
        "bytes beyond it are counted as undelivered instead of blocking observation",
    )
    parser.add_argument("command", nargs=argparse.REMAINDER)
    return parser


def main(argv: Sequence[str] | None = None) -> int:
    parser = build_parser()
    args = parser.parse_args(argv)
    try:
        return run(args)
    except TransportError as error:
        # Argument errors precede a safe provider entry. Avoid inventing a
        # partial receipt whose request fields may not have been established.
        parser.error(str(error))
    except OSError as error:
        parser.error(f"could not execute native Claude command: {error}")
    return 2


if __name__ == "__main__":
    raise SystemExit(main())
