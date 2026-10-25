#!/usr/bin/env python3
"""Append-only cross-session Codex messages and advisory resource claims."""

from __future__ import annotations

import argparse
import ctypes
from datetime import datetime, timedelta, timezone
import errno
import hashlib
import json
import math
import os
from pathlib import Path
import re
import select
import struct
import subprocess
import sys
import time
from typing import Any, Callable
from uuid import UUID, uuid4

from durable_state import DurableStateError, GitRefDurableStateStore


BOARD_KEY = "work-engine/codex-chatboard/v1"
NAMESPACE = "codex-chatboard"
MAX_RETRIES = 20
CONTROL_PREFIX = "control:session:"
REGISTRY_PATH = Path(__file__).resolve().parent.parent / "contracts" / "codex-chatboard-kinds.v1.json"
REGISTRY_TYPE = "work_engine_codex_chatboard_kinds_v1"


class ChatboardError(ValueError):
    pass


def timestamp() -> str:
    return datetime.now(timezone.utc).isoformat().replace("+00:00", "Z")


def parse_time(value: str) -> datetime:
    return datetime.fromisoformat(value.replace("Z", "+00:00"))


def empty_board() -> dict[str, Any]:
    return {
        "artifact_type": "work_engine_codex_chatboard_v1",
        "schema_version": 1,
        "next_sequence": 1,
        "messages": [],
        "claims": {},
    }


def decode(payload: bytes) -> dict[str, Any]:
    try:
        value = json.loads(payload)
    except (UnicodeDecodeError, json.JSONDecodeError) as failure:
        raise ChatboardError("chatboard payload is invalid JSON") from failure
    if (not isinstance(value, dict)
            or value.get("artifact_type") != "work_engine_codex_chatboard_v1"
            or value.get("schema_version") != 1
            or not isinstance(value.get("messages"), list)
            or not isinstance(value.get("claims"), dict)
            or not isinstance(value.get("next_sequence"), int)):
        raise ChatboardError("chatboard payload has an unsupported contract")
    return value


def encode(value: dict[str, Any]) -> bytes:
    return (json.dumps(value, indent=2, sort_keys=True, ensure_ascii=False) + "\n").encode()


def mutate(repository: Path, change: Callable[[dict[str, Any]], Any]) -> tuple[dict[str, Any], Any, str]:
    store = GitRefDurableStateStore(repository, NAMESPACE)
    for _ in range(MAX_RETRIES):
        current = store.read(BOARD_KEY)
        board = decode(current.payload) if current else empty_board()
        result = change(board)
        try:
            published = store.publish(
                BOARD_KEY, encode(board), current.revision if current else None
            )
        except DurableStateError as failure:
            if "revision conflict" in str(failure):
                continue
            raise
        return board, result, published.revision
    raise ChatboardError("chatboard remained busy after bounded CAS retries")


def read_board(repository: Path) -> tuple[dict[str, Any], str | None]:
    value = GitRefDurableStateStore(repository, NAMESPACE).read(BOARD_KEY)
    return (decode(value.payload), value.revision) if value else (empty_board(), None)


def canonical_uuid(value: str, label: str) -> str:
    try:
        parsed = UUID(value)
    except ValueError as failure:
        raise ChatboardError(f"{label} must be a UUID") from failure
    if str(parsed) != value.lower():
        raise ChatboardError(f"{label} must use canonical UUID syntax")
    return str(parsed)


def load_registry(path: Path | None = None) -> dict[str, Any]:
    """Open the kind registry. Called only by paths that need it (lazy rule)."""
    target = Path(path) if path is not None else REGISTRY_PATH
    try:
        value = json.loads(target.read_text())
    except OSError as failure:
        raise ChatboardError(f"kind registry not found or unreadable at {target}: {failure}") from failure
    except json.JSONDecodeError as failure:
        raise ChatboardError(f"kind registry at {target} is invalid JSON") from failure
    if (not isinstance(value, dict) or value.get("artifact_type") != REGISTRY_TYPE
            or not isinstance(value.get("version"), int)
            or not isinstance(value.get("kinds"), dict) or not isinstance(value.get("envelope"), dict)):
        raise ChatboardError(f"kind registry at {target} has an unsupported contract")
    return value


def kind_line(registry: dict[str, Any], name: str) -> str:
    spec = registry["kinds"][name]
    enums = "; ".join(f"{path}={'|'.join(values)}" for path, values in spec.get("enums", {}).items())
    return (f"{name} | writer={spec.get('writer', 'post')} | required: {', '.join(spec.get('required', [])) or '-'}"
            f" | enums: {enums or '-'} | optional data: {', '.join(spec.get('optional_data', [])) or '-'}")


def kinds_view(registry: dict[str, Any], kind: str | None = None) -> list[str]:
    if kind is None:
        return [kind_line(registry, name) for name in registry["kinds"]]
    require_kind(registry, kind)
    return [kind_line(registry, kind)]


def require_kind(registry: dict[str, Any], kind: Any) -> None:
    if not isinstance(kind, str) or kind not in registry["kinds"]:
        raise ChatboardError(
            f"kind {kind!r} is not registered; registered kinds: {', '.join(registry['kinds'])} "
            "(use `note` for an untyped message)")


def _lookup(envelope: dict[str, Any], path: str) -> Any:
    value: Any = envelope
    for part in path.split("."):
        if not isinstance(value, dict) or part not in value:
            return None
        value = value[part]
    return value


def _present(value: Any) -> bool:
    if value is None:
        return False
    if isinstance(value, (str, list, dict)) and len(value) == 0:
        return False
    return not (isinstance(value, str) and not value.strip())


def validate_envelope(registry: dict[str, Any], *, kind: str | None = None, role: str | None = None,
                      to: list[str] | None = None, in_reply_to: int | None = None,
                      supersedes: int | None = None, effects: dict[str, Any] | None = None,
                      data: dict[str, Any] | None = None) -> None:
    """Pure single-message envelope checks; raises ChatboardError naming the field."""
    grammar = registry["envelope"]
    if kind is not None:
        require_kind(registry, kind)
    if role is not None:
        if not isinstance(role, str) or not re.fullmatch(grammar["role_pattern"], role):
            raise ChatboardError(f"role {role!r} must match {grammar['role_pattern']}")
    if to is not None:
        if not isinstance(to, list) or not to:
            raise ChatboardError(f"to must be a nonempty list of addresses ({', '.join(grammar['to_forms'])}); "
                                 f"patterns: {' or '.join(grammar['to_patterns'])}")
        for address in to:
            if not isinstance(address, str) or not any(re.fullmatch(pattern, address)
                                                       for pattern in grammar["to_patterns"]):
                raise ChatboardError(f"to address {address!r} must match one of: "
                                     f"{' or '.join(grammar['to_patterns'])}")
    for name, value in (("in_reply_to", in_reply_to), ("supersedes", supersedes)):
        if value is not None and (isinstance(value, bool) or not isinstance(value, int) or value < 1):
            raise ChatboardError(f"{name} must be a positive message sequence number")
    if effects is not None:
        keys = grammar["effects_keys"]
        if not isinstance(effects, dict):
            raise ChatboardError(f"effects must be an object with exactly the keys {' and '.join(keys)}")
        for key in effects:
            if key not in keys:
                raise ChatboardError(f"effects.{key} is not allowed; the only keys are {' and '.join(keys)}")
        for key in keys:
            items = effects.get(key)
            if key not in effects:
                raise ChatboardError(f"effects.{key} is missing; both {' and '.join(keys)} are required "
                                     "(an empty list states none)")
            if (not isinstance(items, list)
                    or not all(isinstance(item, str) and item.strip() for item in items)):
                raise ChatboardError(f"effects.{key} must be a list of nonempty strings "
                                     f"(effects keys: {' and '.join(keys)})")
    if data is not None and not isinstance(data, dict):
        raise ChatboardError("data must be an object")
    if kind is None:
        return
    envelope = {"kind": kind, "role": role, "to": to, "in_reply_to": in_reply_to,
                "supersedes": supersedes, "effects": effects, "data": data}
    spec = registry["kinds"][kind]
    for path in spec.get("required", []):
        if not _present(_lookup(envelope, path)):
            raise ChatboardError(f"kind {kind}: required field {path} is missing")
    for path, allowed in spec.get("enums", {}).items():
        value = _lookup(envelope, path)
        if value is not None and value not in allowed:
            raise ChatboardError(f"kind {kind}: {path} must be one of {', '.join(allowed)} (got {value!r})")
    for path, pattern in spec.get("patterns", {}).items():
        value = _lookup(envelope, path)
        if value is not None and not (isinstance(value, str) and re.fullmatch(pattern, value)):
            raise ChatboardError(f"kind {kind}: {path} must match {pattern} (got {value!r})")


def _post(repository: Path, *, author: str, session_id: str, topic: str,
          body: str, references: list[str], message_id: str, observed_at: str,
          wake: bool, registry: dict[str, Any] | None, kind: str | None, role: str | None,
          to: list[str] | None, in_reply_to: int | None, supersedes: int | None,
          effects: dict[str, Any] | None, data: dict[str, Any] | None) -> dict[str, Any]:
    canonical_uuid(session_id, "session_id")
    canonical_uuid(message_id, "message_id")
    if not all(isinstance(value, str) and value.strip() for value in (author, topic, body)):
        raise ChatboardError("author, topic, and body must be nonempty")
    if topic.startswith(CONTROL_PREFIX):
        raise ChatboardError("control topics are local telemetry subscriptions, not board publications")
    if not isinstance(wake, bool):
        raise ChatboardError("wake must be a boolean")
    envelope = {"kind": kind, "role": role, "to": to, "in_reply_to": in_reply_to,
                "supersedes": supersedes, "effects": effects, "data": data}
    contract_version = None
    if any(value is not None for value in envelope.values()):
        registry = registry if registry is not None else load_registry()
        validate_envelope(registry, **envelope)
        if kind is not None:
            contract_version = registry["version"]

    def change(board: dict[str, Any]) -> dict[str, Any]:
        existing = next((row for row in board["messages"] if row["message_id"] == message_id), None)
        candidate = {
            "message_id": message_id,
            "sequence": existing["sequence"] if existing else board["next_sequence"],
            "observed_at": observed_at,
            "author": author,
            "session_id": session_id,
            "topic": topic,
            "body": body,
            "references": references,
        }
        # Keep ordinary rows byte-compatible with existing idempotent posts.
        # This marker bypasses session suppression, never topic selection.
        if wake:
            candidate["wake"] = True
        # Envelope keys, like wake, are present only when given.
        for key, value in envelope.items():
            if value is not None:
                candidate[key] = value
        if contract_version is not None:
            candidate["contract_version"] = contract_version
        if existing:
            if existing != candidate:
                raise ChatboardError("message_id already exists with different content")
            return existing
        # Cross-message checks run per CAS attempt against the snapshot that
        # would be published, using the candidate's own sequence.
        targets: dict[str, dict[str, Any]] = {}
        for name, value in (("in_reply_to", in_reply_to), ("supersedes", supersedes)):
            if value is None:
                continue
            if value >= candidate["sequence"]:
                raise ChatboardError(f"{name} {value} must name an existing earlier message "
                                     f"(this message would be sequence {candidate['sequence']})")
            target = next((row for row in board["messages"] if row["sequence"] == value), None)
            if target is None:
                raise ChatboardError(f"{name} {value} names no message on the board")
            targets[name] = target
        if (kind == "reply" and isinstance(data, dict)
                and data.get("disposition") in ("answered", "declined")):
            if in_reply_to is None:
                raise ChatboardError("in_reply_to: an answered/declined reply closes a request and must name it")
            if targets["in_reply_to"].get("kind") != "request":
                raise ChatboardError(
                    f"in_reply_to {in_reply_to}: a reply with disposition {data['disposition']!r} closes a request, so it must name "
                    f"a message of kind `request` (message {in_reply_to} has kind "
                    f"{targets['in_reply_to'].get('kind', 'note')!r})")
        board["messages"].append(candidate)
        board["next_sequence"] += 1
        return candidate

    _, message, revision = mutate(repository, change)
    return {"revision": revision, "message": message}


def post(repository: Path, *, author: str, session_id: str, topic: str,
         body: str, references: list[str], message_id: str, observed_at: str,
         wake: bool = False, kind: str | None = None, role: str | None = None,
         to: list[str] | None = None, in_reply_to: int | None = None,
         supersedes: int | None = None, effects: dict[str, Any] | None = None,
         data: dict[str, Any] | None = None,
         registry: dict[str, Any] | None = None) -> dict[str, Any]:
    if kind is not None:
        registry = registry if registry is not None else load_registry()
        require_kind(registry, kind)
        writer = registry["kinds"][kind].get("writer", "post")
        if writer != "post":
            raise ChatboardError(f"kind {kind} is written only by the `{writer}` subcommand, not `post`")
    return _post(repository, author=author, session_id=session_id, topic=topic, body=body,
                 references=references, message_id=message_id, observed_at=observed_at, wake=wake,
                 registry=registry, kind=kind, role=role, to=to, in_reply_to=in_reply_to,
                 supersedes=supersedes, effects=effects, data=data)


def adopt(repository: Path, *, author: str, session_id: str, body: str, role: str,
          predecessor_session: str, message_id: str, observed_at: str,
          handoff_sha256: str | None = None, handoff_path: Path | None = None,
          claims_reclaimed: list[str] | None = None, channels: list[str] | None = None,
          registry: dict[str, Any] | None = None) -> dict[str, Any]:
    """Write the one `adoption` row; same mutate() path and replay rule as post."""
    if canonical_uuid(session_id, "session_id") != session_id:
        raise ChatboardError("session_id must be a canonical lowercase UUID: adoption writes topic "
                             "`session:<session_id>`, which the gate matches exactly "
                             "(uppercase is rejected, not normalised)")
    try:
        parsed = UUID(predecessor_session)
    except (ValueError, AttributeError, TypeError) as failure:
        raise ChatboardError("predecessor_session must be a canonical lowercase UUID") from failure
    if str(parsed) != predecessor_session:
        raise ChatboardError("predecessor_session must be a canonical lowercase UUID "
                             "(uppercase or non-canonical forms are rejected, not normalised)")
    if predecessor_session == session_id.lower():
        raise ChatboardError("predecessor_session must differ from the poster's session_id")
    if handoff_sha256 is not None and not re.fullmatch(r"[0-9a-f]{64}", handoff_sha256):
        raise ChatboardError("handoff_sha256 must be 64 lowercase hex characters")
    if handoff_path is not None:
        try:
            actual = hashlib.sha256(Path(handoff_path).read_bytes()).hexdigest()
        except OSError as failure:
            raise ChatboardError(f"handoff_path cannot be read: {failure}") from failure
        if handoff_sha256 is not None and handoff_sha256 != actual:
            raise ChatboardError(f"handoff_sha256 does not equal the sha256 of {handoff_path} ({actual})")
        handoff_sha256 = actual
    if handoff_sha256 is None:
        raise ChatboardError("handoff_sha256 or handoff_path is required")
    data: dict[str, Any] = {"predecessor_session": predecessor_session, "handoff_sha256": handoff_sha256}
    if claims_reclaimed:
        data["claims_reclaimed"] = list(claims_reclaimed)
    if channels:
        data["channels"] = list(channels)
    registry = registry if registry is not None else load_registry()
    if "adoption" not in registry["kinds"]:
        raise ChatboardError("kind registry does not register `adoption`")
    return _post(repository, author=author, session_id=session_id, topic=f"session:{session_id}",
                 body=body, references=[], message_id=message_id, observed_at=observed_at, wake=False,
                 registry=registry, kind="adoption", role=role, to=[f"role:{role}"] if isinstance(role, str) else None,
                 in_reply_to=None, supersedes=None, effects=None, data=data)


def claim(repository: Path, *, resource: str, author: str, session_id: str,
          claim_id: str, ttl_seconds: int, note: str, observed_at: str) -> dict[str, Any]:
    canonical_uuid(session_id, "session_id")
    canonical_uuid(claim_id, "claim_id")
    if not resource.strip() or not author.strip() or not note.strip() or ttl_seconds <= 0:
        raise ChatboardError("resource, author, note, and positive ttl are required")
    expires_at = (parse_time(observed_at) + timedelta(seconds=ttl_seconds)).isoformat().replace("+00:00", "Z")

    def change(board: dict[str, Any]) -> dict[str, Any]:
        existing = board["claims"].get(resource)
        if existing and existing["claim_id"] != claim_id and parse_time(existing["expires_at"]) > parse_time(observed_at):
            raise ChatboardError(
                f"resource is already claimed by {existing['author']} session {existing['session_id']}"
            )
        candidate = {
            "resource": resource, "claim_id": claim_id, "author": author,
            "session_id": session_id, "claimed_at": observed_at,
            "expires_at": expires_at, "note": note,
            "authority": "advisory_coordination_only",
        }
        if existing and existing.get("claim_id") == claim_id and existing != candidate:
            raise ChatboardError("claim_id already exists with different content")
        board["claims"][resource] = candidate
        return candidate

    _, value, revision = mutate(repository, change)
    return {"revision": revision, "claim": value}


def release(repository: Path, *, resource: str, session_id: str, claim_id: str) -> dict[str, Any]:
    canonical_uuid(session_id, "session_id")
    canonical_uuid(claim_id, "claim_id")

    def change(board: dict[str, Any]) -> dict[str, Any]:
        existing = board["claims"].get(resource)
        if not existing:
            return {"resource": resource, "released": False, "reason": "absent"}
        if existing["session_id"] != session_id or existing["claim_id"] != claim_id:
            raise ChatboardError("claim ownership does not match release request")
        del board["claims"][resource]
        return {"resource": resource, "released": True}

    _, value, revision = mutate(repository, change)
    return {"revision": revision, **value}


def row_kind(row: dict[str, Any]) -> Any:
    """A row without `kind` is a legacy or untyped message and reads as `note`.

    A stored kind that the current registry does not know is returned as
    recorded: readers display it, they do not reject it (D5).
    """
    kind = row.get("kind")
    return kind if isinstance(kind, str) else "note"


def read_view(board: dict[str, Any], *, since: int | None, limit: int,
              active_claims: bool, observed_at: str,
              kinds: list[str] | None = None, roles: list[str] | None = None) -> dict[str, Any]:
    # Filters apply before the tail slice and before `limit`: a startup read
    # needs the latest matching coordination, not the latest rows of any kind.
    candidates = board["messages"]
    if kinds:
        candidates = [row for row in candidates if row_kind(row) in kinds]
    if roles:
        candidates = [row for row in candidates if row.get("role") in roles]
    # Without --since, a startup read needs the latest coordination, not the
    # oldest retained messages. With --since, page forward from that sequence.
    if since is None:
        rows = candidates[-limit:]
    else:
        rows = [row for row in candidates if row["sequence"] > since][:limit]
    claims = board["claims"]
    if active_claims:
        now = parse_time(observed_at)
        claims = {resource: value for resource, value in claims.items()
                  if parse_time(value["expires_at"]) > now}
    return {"messages": rows, "claims": claims}


COMPACT_BODY_WIDTH = 80


def compact_line(row: dict[str, Any]) -> str:
    """One tab-separated line per message; the full row stays available in JSON.

    Fields: sequence, kind (`note` when absent), role, `to` (comma-joined),
    links (`reply:N`, `supersedes:M`, comma-joined), author, topic, body cut to
    COMPACT_BODY_WIDTH characters (`...` marks a cut). `-` marks an absent
    field. Whitespace inside a field is collapsed so a field cannot add a tab
    or a line.
    """
    def clean(value: Any) -> str:
        text = " ".join(str(value).split())
        return text or "-"

    to = row.get("to")
    to_text = ",".join(to) if isinstance(to, list) and all(isinstance(a, str) for a in to) and to else "-"
    links = [f"{label}:{row[key]}" for label, key in (("reply", "in_reply_to"), ("supersedes", "supersedes"))
             if key in row]
    body = " ".join(str(row.get("body", "")).split())
    if len(body) > COMPACT_BODY_WIDTH:
        body = body[:COMPACT_BODY_WIDTH - 3] + "..."
    return "\t".join([str(row["sequence"]), clean(row_kind(row)), clean(row.get("role", "")),
                      clean(to_text), ",".join(links) or "-", clean(row.get("author", "")),
                      clean(row.get("topic", "")), body or "-"])


class RefChanges:
    """Linux notifications are wake hints; integrity-checked Git reads own truth."""

    EVENT = struct.Struct("iIII")
    # modify, attrib, close_write, moved_from/to, create/delete, delete/move_self
    WATCH_MASK = 0x00000FCE
    INVALIDATED = 0x0000EC00  # delete_self, move_self, unmount, overflow, ignored

    def __init__(self, repository: Path) -> None:
        if sys.platform != "linux":
            raise ChatboardError("wait requires Linux inotify; polling fallback is not enabled")
        store = GitRefDurableStateStore(repository, NAMESPACE)
        if store._git(["rev-parse", "--show-ref-format"]).strip() != b"files":
            raise ChatboardError("wait requires the Git files ref backend")
        common = Path(store._git([
            "rev-parse", "--path-format=absolute", "--git-common-dir"
        ]).decode().strip())
        self.paths: list[tuple[Path, set[str]]] = []
        directory = common
        for component in store._ref(BOARD_KEY).split("/"):
            names = {component}
            if directory == common:
                names.add("packed-refs")
            self.paths.append((directory, names))
            directory /= component
        self.libc = ctypes.CDLL(None, use_errno=True)
        self.libc.inotify_init1.argtypes = [ctypes.c_int]
        self.libc.inotify_init1.restype = ctypes.c_int
        self.libc.inotify_add_watch.argtypes = [ctypes.c_int, ctypes.c_char_p, ctypes.c_uint32]
        self.libc.inotify_add_watch.restype = ctypes.c_int
        self.fd = -1
        self.watches: dict[int, set[str]] = {}
        self.arm()

    def add_file(self, path: Path) -> None:
        """Also observe an explicit telemetry source, including replacement."""
        path = path.absolute()
        for directory in reversed(path.parents):
            component = path.relative_to(directory).parts[0]
            self.paths.append((directory, {component}))
        self.arm()

    def arm(self) -> None:
        # Watch ancestors before descendants, including when the board has
        # never been posted. Re-arm BEFORE rereading after topology changes.
        fd = self.libc.inotify_init1(os.O_NONBLOCK | os.O_CLOEXEC)
        if fd < 0:
            raise OSError(ctypes.get_errno(), "cannot initialize board notifications")
        watches = {}
        try:
            for directory, names in self.paths:
                wd = self.libc.inotify_add_watch(fd, os.fsencode(directory), self.WATCH_MASK)
                if wd < 0:
                    error = ctypes.get_errno()
                    if error == errno.ENOENT and watches:
                        continue  # each source's nearest existing ancestor is watched
                    raise OSError(error, "cannot watch board ref directory")
                watches.setdefault(wd, set()).update(names)
        except BaseException:
            os.close(fd)
            raise
        self.close()
        self.fd, self.watches = fd, watches

    def wait(self, timeout: float | None) -> bool:
        deadline = None if timeout is None else time.monotonic() + timeout
        while True:
            if deadline is not None and time.monotonic() >= deadline:
                return False
            remaining = None if deadline is None else max(0, deadline - time.monotonic())
            ready, _, _ = select.select([self.fd], [], [], remaining)
            if not ready:
                return False
            try:
                events = os.read(self.fd, 65536)
            except BlockingIOError:
                continue
            offset = 0
            while offset < len(events):
                wd, mask, _, length = self.EVENT.unpack_from(events, offset)
                offset += self.EVENT.size
                name = os.fsdecode(events[offset:offset + length].split(b"\0", 1)[0])
                offset += length
                if mask & self.INVALIDATED or name in self.watches.get(wd, set()):
                    return True
            # Unrelated Git activity does not trigger another board read.

    def close(self) -> None:
        if self.fd >= 0:
            os.close(self.fd)
            self.fd = -1

    def __enter__(self) -> RefChanges:
        return self

    def __exit__(self, *_: object) -> None:
        self.close()


class ContextControl:
    """Thin adapter to the pinned Rust telemetry reader; no policy or persistence."""

    def __init__(self, *, session_id: str, rollout: Path, binary: Path, sha256: str,
                 trigger_tokens: int, band_floor_tokens: int, host: str = "codex",
                 agent: str | None = None, lead_calls: int = 3,
                 cursor: str | None = None) -> None:
        self.topic = f"{CONTROL_PREFIX}{session_id}" + (f"/agent:{agent}" if agent else "")
        self.rollout, self.binary = rollout.absolute(), binary.absolute()
        self.digest, self.cursor = sha256.lower(), cursor
        if len(self.digest) != 64 or any(c not in "0123456789abcdef" for c in self.digest):
            raise ChatboardError("--context-sha256 must be a SHA-256 digest")
        self.command = [str(self.binary), "control", "--session", session_id,
                        "--rollout", str(self.rollout), "--host", host,
                        "--trigger-tokens", str(trigger_tokens),
                        "--band-floor-tokens", str(band_floor_tokens),
                        "--lead-calls", str(lead_calls)]
        if agent:
            self.command += ["--agent", agent]

    def read(self) -> dict[str, Any]:
        # Check at every execution, not just enrollment. On any adapter failure,
        # stop explicitly so the owner can fall back to the boundary check.
        if not self.binary.is_file() or hashlib.sha256(self.binary.read_bytes()).hexdigest() != self.digest:
            raise ChatboardError("context binary missing or digest changed; control listener stopped")
        command = self.command + (["--cursor", self.cursor] if self.cursor else [])
        try:
            result = subprocess.run(command, capture_output=True, text=True, timeout=10)
        except subprocess.TimeoutExpired as failure:
            raise ChatboardError("context reader timed out; control listener stopped") from failure
        if result.returncode:
            raise ChatboardError(f"context reader failed: {result.stderr.strip()[:2000]}")
        try:
            value = json.loads(result.stdout)
        except ValueError as failure:
            raise ChatboardError("invalid context reader output") from failure
        if (not isinstance(value, dict) or value.get("schema") != "manual-context-control-v1"
                or "event" not in value
                or value.get("topic") != self.topic
                or not isinstance(value.get("next_control_cursor"), str)
                or (value.get("event") is not None and (
                    not isinstance(value["event"], dict)
                    or value["event"].get("topic") != self.topic
                    or value["event"].get("authority") != "observation_only"))):
            raise ChatboardError("context reader returned an incompatible or mismatched binding")
        self.cursor = value["next_control_cursor"]
        return value


def addressed_to(row: dict[str, Any], selected: list[str]) -> bool:
    """True when a stored `to` list contains a selected address.

    Rows written outside `post` validation can carry any `to`; anything that
    is not a list of strings does not match. Matching is by row, so a repeated
    address in one `to` cannot deliver the row twice. Control addresses never
    match: a `control:` topic is reachable only through the explicit binding.
    """
    to = row.get("to")
    return isinstance(to, list) and all(isinstance(address, str) for address in to) \
        and any(address in selected and not address.startswith(CONTROL_PREFIX) for address in to)


def wait_board(repository: Path, *, since: int, limit: int = 100,
               topic: str | None = None, topics: list[str] | None = None,
               timeout_seconds: float | None = None,
               control: ContextControl | None = None,
               exclude_session_id: str | None = None) -> dict[str, Any]:
    if since < 0 or not 1 <= limit <= 500:
        raise ChatboardError("--since must be nonnegative and --limit must be 1..500")
    if exclude_session_id is not None:
        exclude_session_id = canonical_uuid(exclude_session_id, "--exclude-session-id")
    selected = [*(topics or []), *([topic] if topic is not None else [])]
    if any(not name.strip() for name in selected):
        raise ChatboardError("--topic must be nonempty")
    selected = sorted(set(selected))
    if any(name.startswith(CONTROL_PREFIX) and (control is None or name != control.topic)
           for name in selected):
        raise ChatboardError("control topic requires the matching explicit session binding")
    if timeout_seconds is not None and (not math.isfinite(timeout_seconds) or timeout_seconds < 0):
        raise ChatboardError("--timeout-seconds must be finite and nonnegative")
    deadline = None if timeout_seconds is None else time.monotonic() + timeout_seconds
    # Subscribe first, then read: a publication during the read remains queued.
    with RefChanges(repository) as changes:
        if control is not None:
            changes.add_file(control.rollout)
            changes.add_file(control.binary)
        while True:
            board, revision = read_board(repository)
            # D10: a row is delivered once however many paths match it, and it
            # is returned byte-unchanged; how it matched is reported outside it.
            by_path: dict[int, list[str]] = {}
            for row in board["messages"]:
                if (row["sequence"] > since
                        and not row["topic"].startswith(CONTROL_PREFIX)
                        and (exclude_session_id is None
                             or row.get("session_id", "").lower() != exclude_session_id
                             or row.get("wake") is True)):
                    paths = []
                    if selected:
                        if row["topic"] in selected:
                            paths.append("topic")
                        if addressed_to(row, selected):
                            paths.append("to")
                        if not paths:
                            continue
                    by_path[row["sequence"]] = paths
            matches = [row for row in board["messages"] if row["sequence"] in by_path]
            rows = matches[:limit]
            more = len(matches) > limit
            cursor = rows[-1]["sequence"] if more else max(since, board["next_sequence"] - 1)
            result = {"status": "messages" if rows else "timeout", "revision": revision,
                      "messages": rows, "next_since": cursor, "has_more": more,
                      "exclude_session_id": exclude_session_id,
                      "topics": selected, "topic": selected[0] if len(selected) == 1 else None}
            if any("to" in by_path[row["sequence"]] for row in rows):
                result["matches"] = [{"sequence": row["sequence"], "matched_by": by_path[row["sequence"]]}
                                     for row in rows]
            if control is not None:
                observation = control.read()
                result.update(control_events=([observation["event"]] if observation["event"] else []),
                              control_topic=control.topic, next_control_cursor=control.cursor)
                if result["control_events"] and not rows:
                    result["status"] = "control"
            if rows or result.get("control_events") or (deadline is not None and time.monotonic() >= deadline):
                return result
            remaining = None if deadline is None else max(0, deadline - time.monotonic())
            if changes.wait(remaining):
                changes.arm()
            # Also reread once at timeout: return the last authoritative view.


def topics_view(board: dict[str, Any], *, since: int, limit: int, offset: int) -> dict[str, Any]:
    if since < 0 or offset < 0 or not 1 <= limit <= 500:
        raise ChatboardError("--since and --offset must be nonnegative; --limit must be 1..500")
    topics: dict[str, dict[str, Any]] = {}
    for row in board["messages"]:
        if row["sequence"] <= since:
            continue
        entry = topics.setdefault(row["topic"], {"topic": row["topic"], "message_count": 0})
        entry["message_count"] += 1
        entry["latest_sequence"] = row["sequence"]
        entry["latest_observed_at"] = row["observed_at"]
    ordered = [topics[name] for name in sorted(topics)]
    page = ordered[offset:offset + limit]
    more = offset + len(page) < len(ordered)
    return {"topics": page, "total_topics": len(ordered), "since": since,
            "next_offset": offset + len(page) if more else None, "has_more": more}



PROJECTION_NOTE = ("projection of unauthenticated board reports; it is not committed ownership and "
                   "not the rotation gate's acceptance of any transition")
NOT_ESTABLISHED = "not established"
NOT_REPORTED = "not reported"


def _is_sequence(value: Any) -> bool:
    return isinstance(value, int) and not isinstance(value, bool)


def _is_text(value: Any) -> bool:
    return isinstance(value, str) and bool(value.strip())


def _snapshot_rows(board: dict[str, Any]) -> list[dict[str, Any]]:
    """Stored rows a view can use: dicts with an integer sequence, in sequence order."""
    messages = board.get("messages")
    if not isinstance(messages, list):
        return []
    rows = [row for row in messages if isinstance(row, dict) and _is_sequence(row.get("sequence"))]
    return sorted(rows, key=lambda row: row["sequence"])


def _registry_path(registry: dict[str, Any], kind: str, section: str, leaf: str) -> tuple[str, Any]:
    """Find the registry path ending in `.leaf` under `kinds.<kind>.<section>`.

    `required` and `enums` hold envelope-rooted paths (`data.disposition`), so the
    result is applied to a stored row with `_lookup`. `optional_data` entries are
    rooted at `data` and are not read here. Returns the path and its enum values
    (None for a `required` entry).
    """
    try:
        spec = registry["kinds"][kind][section]
    except (KeyError, TypeError) as failure:
        raise ChatboardError(f"kind registry has no {kind}.{section}") from failure
    for path in spec:
        if isinstance(path, str) and path.startswith("data.") and path.split(".")[-1] == leaf:
            return path, spec[path] if isinstance(spec, dict) else None
    raise ChatboardError(f"kind registry {kind}.{section} names no data field `{leaf}`")


def roles_view(board: dict[str, Any], *, roles: list[str] | None = None,
               registry: dict[str, Any] | None = None) -> dict[str, Any]:
    """Per-role projection of adoption and `ending` reports; never a holder or an owner.

    A pure function of the snapshot. An adoption is unended until an `availability`
    row with `data.state` `ending` from the same session and role follows it. Every
    succession entry carries `committed: not established`: no board row shows that
    a rotation committed (D9).
    """
    registry = registry if registry is not None else load_registry()
    predecessor_path, _ = _registry_path(registry, "adoption", "required", "predecessor_session")
    state_path, states = _registry_path(registry, "availability", "enums", "state")
    if not isinstance(states, list) or "ending" not in states:
        raise ChatboardError("kind registry availability does not enumerate `ending`")
    by_role: dict[str, list[dict[str, Any]]] = {}
    for row in _snapshot_rows(board):
        role = row.get("role")
        if _is_text(role) and (not roles or role in roles):
            by_role.setdefault(role, []).append(row)
    entries = [_role_entry(role, by_role[role], predecessor_path, state_path) for role in sorted(by_role)]
    return {"projection": PROJECTION_NOTE, "roles": entries, "total_roles": len(entries)}


def _role_entry(role: str, rows: list[dict[str, Any]], predecessor_path: str, state_path: str) -> dict[str, Any]:
    adoptions: list[dict[str, Any]] = []
    endings: dict[str, list[int]] = {}
    reporting: set[str] = set()
    for row in rows:
        session = row.get("session_id")
        if not _is_text(session):
            continue
        reporting.add(session)
        kind = row_kind(row)
        if kind == "adoption":
            predecessor = _lookup(row, predecessor_path)
            if _is_text(predecessor):
                adoptions.append({"sequence": row["sequence"], "session": session, "predecessor": predecessor})
        elif kind == "availability" and _lookup(row, state_path) == "ending":
            endings.setdefault(session, []).append(row["sequence"])

    def ended_at(adoption: dict[str, Any]) -> int | None:
        later = [seq for seq in endings.get(adoption["session"], []) if seq > adoption["sequence"]]
        return min(later) if later else None

    def earlier_adoption(session: str, before: int) -> dict[str, Any] | None:
        found = [a for a in adoptions if a["session"] == session and a["sequence"] < before]
        return max(found, key=lambda a: a["sequence"]) if found else None

    def predecessor_ending(adoption: dict[str, Any]) -> int | None:
        link = earlier_adoption(adoption["predecessor"], adoption["sequence"])
        floor = link["sequence"] if link else 0
        # Bounded above by the predecessor's next adoption: a later stint's ending is not this one's.
        ceiling = min((a["sequence"] for a in adoptions
                       if a["session"] == adoption["predecessor"] and a["sequence"] > floor), default=None)
        later = [seq for seq in endings.get(adoption["predecessor"], [])
                 if seq > floor and (ceiling is None or seq < ceiling)]
        return min(later) if later else None

    def chain(adoption: dict[str, Any]) -> list[dict[str, Any]]:
        # Traverses ended adoptions: an ended adoption leaves the unended set but stays a link.
        links = [adoption]
        while True:
            nxt = earlier_adoption(links[-1]["predecessor"], links[-1]["sequence"])
            if nxt is None:
                return links
            links.append(nxt)

    def sessions_of(links: list[dict[str, Any]]) -> list[str]:
        return [links[0]["session"]] + [link["predecessor"] for link in links]

    unended = sorted((a for a in adoptions if ended_at(a) is None), key=lambda a: a["sequence"])
    unended_sequences = {a["sequence"] for a in unended}
    relations: list[dict[str, Any]] = []
    for later in unended:
        links = chain(later)
        ancestors = [link for link in links[1:] if link["sequence"] in unended_sequences]
        for earlier in ancestors:
            path = sessions_of(links[:links.index(earlier) + 1])
            relations.append({"relation": "overlap", "successor_session": later["session"],
                              "successor_adoption_sequence": later["sequence"],
                              "predecessor_session": later["predecessor"],
                              "earlier_session": earlier["session"],
                              "earlier_adoption_sequence": earlier["sequence"], "path": path})
        if not ancestors and predecessor_ending(later) is None:
            relations.append({"relation": "overlap", "successor_session": later["session"],
                              "successor_adoption_sequence": later["sequence"],
                              "predecessor_session": later["predecessor"],
                              "earlier_session": later["predecessor"], "earlier_adoption_sequence": NOT_REPORTED,
                              "path": [later["session"], later["predecessor"]]})
    for index, first in enumerate(unended):
        for second in unended[index + 1:]:
            first_links, second_links = chain(first), chain(second)
            if second["sequence"] in {a["sequence"] for a in first_links} \
                    or first["sequence"] in {a["sequence"] for a in second_links}:
                continue  # one chain: reported as overlap above
            first_path, second_path = sessions_of(first_links), sessions_of(second_links)
            common = next((s for s in first_path if s in second_path), None)
            members = [{"adoption_sequence": first["sequence"], "session": first["session"]},
                       {"adoption_sequence": second["sequence"], "session": second["session"]}]
            if common is None:
                relations.append({"relation": "conflict", "adoptions": members})
            else:
                members[0]["path"] = first_path[:first_path.index(common) + 1]
                members[1]["path"] = second_path[:second_path.index(common) + 1]
                relations.append({"relation": "fork", "at_session": common, "adoptions": members})
    present = {relation["relation"] for relation in relations}
    if "conflict" in present:
        state = "conflict"
    elif "fork" in present:
        state = "fork"
    elif "overlap" in present:
        state = "overlap"
    elif unended:
        state = "single"
    else:
        state = "ended" if adoptions else "no_adoption"
    entry: dict[str, Any] = {"role": role, "state": state}
    if state in ("single", "overlap"):
        latest = unended[-1]
        entry["reported_holder"] = {"session": latest["session"], "adoption_sequence": latest["sequence"]}
    else:
        entry["reported_holder"] = NOT_ESTABLISHED
    if state in ("fork", "conflict"):
        entry["candidates"] = [{"session": a["session"], "adoption_sequence": a["sequence"]} for a in unended]
    if state in ("no_adoption", "ended"):
        entry["reporting_sessions"] = sorted(reporting)
    if state == "ended":
        entry["last_ending_sequence"] = max(seq for seq in (ended_at(a) for a in adoptions) if seq is not None)

    def or_not_reported(value: int | None) -> Any:
        return value if value is not None else NOT_REPORTED

    entry["successions"] = [
        {"adoption_sequence": a["sequence"], "predecessor_session": a["predecessor"],
         "successor_session": a["session"],
         "predecessor_ending_sequence": or_not_reported(predecessor_ending(a)),
         "successor_ending_sequence": or_not_reported(ended_at(a)),
         "committed": NOT_ESTABLISHED}
        for a in sorted(adoptions, key=lambda a: a["sequence"])]
    entry["relations"] = relations
    return entry


def requests_view(board: dict[str, Any], *, to: str | None = None,
                  registry: dict[str, Any] | None = None) -> dict[str, Any]:
    """Requests by addressee. A request is open until a closing reply names it.

    A closing reply (`answered` or `declined`) closes only a stored `request` row
    with a lower sequence; one whose target is anything else is listed under
    `unattached_replies` and closes nothing (X3). `deferred` and `partial` replies
    and receipts are attached but never close. `to` filters requests only;
    unattached replies are always listed.
    """
    registry = registry if registry is not None else load_registry()
    disposition_path, dispositions = _registry_path(registry, "reply", "enums", "disposition")
    closing = {value for value in ("answered", "declined") if isinstance(dispositions, list) and value in dispositions}
    rows = _snapshot_rows(board)
    by_sequence = {row["sequence"]: row for row in rows}
    closed_by: dict[int, dict[str, Any]] = {}
    replies: dict[int, list[dict[str, Any]]] = {}
    receipts: dict[int, list[dict[str, Any]]] = {}
    unattached: list[dict[str, Any]] = []
    for row in rows:
        kind = row_kind(row)
        target = row.get("in_reply_to")
        attached = _is_sequence(target) and target < row["sequence"] \
            and target in by_sequence and row_kind(by_sequence[target]) == "request"
        if kind == "reply":
            disposition = _lookup(row, disposition_path)
            is_closing = isinstance(disposition, str) and disposition in closing
            if is_closing and not attached:
                target_row = by_sequence.get(target) if _is_sequence(target) else None
                unattached.append({"sequence": row["sequence"],
                                   "in_reply_to": target if _is_sequence(target) else None,
                                   "disposition": disposition,
                                   "target_kind": row_kind(target_row) if target_row is not None else "missing"})
            elif attached:
                replies.setdefault(target, []).append(
                    {"sequence": row["sequence"],
                     "disposition": disposition if _is_text(disposition) else NOT_ESTABLISHED,
                     "session_id": row.get("session_id")})
                if is_closing and target not in closed_by:
                    closed_by[target] = {"sequence": row["sequence"], "disposition": disposition}
        elif kind == "receipt" and attached:
            receipts.setdefault(target, []).append({"sequence": row["sequence"], "session_id": row.get("session_id")})
    groups: dict[str, list[dict[str, Any]]] = {}
    for row in rows:
        if row_kind(row) != "request":
            continue
        addresses = row.get("to")
        valid = [a for a in addresses if _is_text(a)] if isinstance(addresses, list) else []
        close = closed_by.get(row["sequence"])
        item = {"sequence": row["sequence"], "author": row.get("author"), "session_id": row.get("session_id"),
                "topic": row.get("topic"), "status": close["disposition"] if close else "open",
                "closed_by_sequence": close["sequence"] if close else None,
                "replies": replies.get(row["sequence"], []), "receipts": receipts.get(row["sequence"], [])}
        for name in sorted(set(valid)) or [NOT_ESTABLISHED]:
            if to is None or name == to:
                groups.setdefault(name, []).append(item)
    addressees = [{"to": name, "open_count": sum(1 for r in groups[name] if r["status"] == "open"),
                   "requests": groups[name]} for name in sorted(groups)]
    return {"projection": PROJECTION_NOTE, "addressees": addressees, "unattached_replies": unattached,
            "total_requests": len({r["sequence"] for g in groups.values() for r in g})}


def parser() -> argparse.ArgumentParser:
    root = argparse.ArgumentParser(description=__doc__)
    root.add_argument("--repository", type=Path, default=Path.cwd())
    commands = root.add_subparsers(dest="command", required=True)
    read = commands.add_parser("read")
    read.add_argument("--since", type=int, default=None)
    read.add_argument("--limit", type=int, default=100)
    read.add_argument("--active-claims", action="store_true")
    read.add_argument("--kind", action="append", dest="kinds", default=[],
                      help="only messages of this kind (repeatable); `note` also matches rows without a kind")
    read.add_argument("--role", action="append", dest="roles", default=[],
                      help="only messages posted under this role (repeatable)")
    read.add_argument("--format", choices=["json", "compact"], default="json",
                      help="compact prints one tab-separated line per message and omits claims and revision")
    wait = commands.add_parser("wait", help="block for new messages without periodic polling")
    wait.add_argument("--since", type=int, required=True)
    wait.add_argument("--limit", type=int, default=100)
    wait.add_argument("--topic", action="append", dest="topics", default=[],
                      help="exact topic match; repeat to match any selected topic; omit for all")
    wait.add_argument("--timeout-seconds", type=float, default=None,
                      help="default: indefinite; 0 checks once; timeout exits 2")
    wait.add_argument("--follow", action="store_true",
                      help="stream JSON lines and rearm until the total timeout or cancellation")
    wait.add_argument("--exclude-session-id",
                      help="suppress this session's ordinary board posts; explicit --wake posts still match")
    wait.add_argument("--session-id", help="subscribe to this session's local control events")
    wait.add_argument("--context-rollout", type=Path, help="explicit bound rollout/transcript")
    wait.add_argument("--context-binary", type=Path, help="pinned context-status binary with control support")
    wait.add_argument("--context-sha256")
    wait.add_argument("--context-host", choices=["codex", "claude"], default="codex")
    wait.add_argument("--context-agent", help="Claude child agent identity within the session")
    wait.add_argument("--context-trigger-tokens", type=int)
    wait.add_argument("--context-band-floor-tokens", type=int)
    wait.add_argument("--context-lead-calls", type=int, default=3)
    wait.add_argument("--control-cursor", help="opaque next_control_cursor from the last consumed result")
    topics = commands.add_parser("topics", help="list recorded topics with counts and latest sequence")
    topics.add_argument("--since", type=int, default=0)
    topics.add_argument("--limit", type=int, default=100)
    topics.add_argument("--offset", type=int, default=0)
    roles_cmd = commands.add_parser("roles", help="project adoption and ending reports per role (reported, not committed)")
    roles_cmd.add_argument("--role", action="append", dest="roles", default=[],
                           help="limit to this role (repeatable)")
    requests_cmd = commands.add_parser("requests", help="list requests by addressee with replies and receipts")
    requests_cmd.add_argument("--to", help="limit to requests addressed to this role:<name> or session:<uuid>")
    post_cmd = commands.add_parser("post")
    for name in ("author", "session-id", "topic", "body"):
        post_cmd.add_argument(f"--{name}", required=True)
    post_cmd.add_argument("--message-id", default=None)
    post_cmd.add_argument("--ref", action="append", default=[])
    post_cmd.add_argument("--wake", action="store_true",
                          help="mark an intentional wake signal that bypasses listener session suppression")
    post_cmd.add_argument("--kind", help="registered message kind (see the `kinds` subcommand)")
    post_cmd.add_argument("--role", help="the poster's stable role name")
    post_cmd.add_argument("--to", action="append", default=None,
                          help="addressee role:<name> or session:<uuid>; repeatable")
    post_cmd.add_argument("--reply-to", type=int, default=None, dest="in_reply_to",
                          help="sequence of the message this answers")
    post_cmd.add_argument("--supersedes", type=int, default=None,
                          help="sequence of the message this corrects or replaces")
    post_cmd.add_argument("--effects-file", type=Path, help="JSON file: {shared_writes, external_actions}")
    post_cmd.add_argument("--data-file", type=Path, help="JSON file: kind-specific data object")
    kinds_cmd = commands.add_parser("kinds", help="print the registered kinds, one line each; reads no board")
    kinds_cmd.add_argument("kind", nargs="?", default=None)
    adopt_cmd = commands.add_parser("adopt", help="record a root adoption on session:<own session id>")
    for name in ("author", "session-id", "body", "role", "predecessor-session"):
        adopt_cmd.add_argument(f"--{name}", required=True)
    adopt_cmd.add_argument("--message-id", default=None)
    adopt_cmd.add_argument("--handoff-sha256")
    adopt_cmd.add_argument("--handoff-path", type=Path)
    adopt_cmd.add_argument("--claims-reclaimed", action="append", default=None)
    adopt_cmd.add_argument("--channels", action="append", default=None)
    claim_cmd = commands.add_parser("claim")
    for name in ("resource", "author", "session-id", "note"):
        claim_cmd.add_argument(f"--{name}", required=True)
    claim_cmd.add_argument("--claim-id", default=None)
    claim_cmd.add_argument("--ttl-seconds", type=int, default=3600)
    release_cmd = commands.add_parser("release")
    for name in ("resource", "session-id", "claim-id"):
        release_cmd.add_argument(f"--{name}", required=True)
    return root


def read_json_file(path: Path | None, label: str) -> Any:
    if path is None:
        return None
    try:
        value = json.loads(path.read_text())
    except (OSError, json.JSONDecodeError) as failure:
        raise ChatboardError(f"{label} {path} must be readable JSON: {failure}") from failure
    if not isinstance(value, dict):
        raise ChatboardError(f"{label} must contain a JSON object")
    return value


def preview_kind_help(argv: list[str]) -> None:
    """`post --kind K --help` prints K's registry line above the ordinary help."""
    if "post" not in argv or not ({"--help", "-h"} & set(argv)) or "--kind" not in argv:
        return
    index = argv.index("--kind")
    if index + 1 < len(argv):
        try:
            print(kinds_view(load_registry(), argv[index + 1])[0])
        except ChatboardError as failure:
            print(f"kind: {failure}", file=sys.stderr)


def main() -> int:
    preview_kind_help(sys.argv[1:])
    args = parser().parse_args()
    try:
        if args.command == "read":
            if (args.since is not None and args.since < 0) or not 1 <= args.limit <= 500:
                raise ChatboardError("--since must be nonnegative and --limit must be 1..500")
            board, revision = read_board(args.repository)
            if args.format == "compact" and args.active_claims:
                raise ChatboardError("--format compact prints messages only; drop --active-claims")
            view = read_view(board, since=args.since, limit=args.limit,
                             active_claims=args.active_claims, observed_at=timestamp(),
                             kinds=args.kinds, roles=args.roles)
            if args.format == "compact":
                for row in view["messages"]:
                    print(compact_line(row))
            else:
                print(json.dumps({"revision": revision, **view}, indent=2))
        elif args.command == "wait":
            required = [args.session_id, args.context_rollout, args.context_binary,
                        args.context_sha256, args.context_trigger_tokens, args.context_band_floor_tokens]
            control = None
            if any(value is not None for value in required):
                if not all(value is not None for value in required):
                    raise ChatboardError("control requires --session-id, --context-rollout, --context-binary, "
                                         "--context-sha256, --context-trigger-tokens and --context-band-floor-tokens")
                control = ContextControl(
                    session_id=args.session_id, rollout=args.context_rollout,
                    binary=args.context_binary, sha256=args.context_sha256,
                    trigger_tokens=args.context_trigger_tokens, band_floor_tokens=args.context_band_floor_tokens,
                    host=args.context_host, agent=args.context_agent, lead_calls=args.context_lead_calls,
                    cursor=args.control_cursor)
            elif args.control_cursor or args.context_agent or args.context_host != "codex" or args.context_lead_calls != 3:
                raise ChatboardError("control options require a session binding")
            started = time.monotonic()
            remaining = args.timeout_seconds
            while True:
                result = wait_board(args.repository, since=args.since, limit=args.limit,
                                    topics=args.topics, timeout_seconds=remaining, control=control,
                                    exclude_session_id=args.exclude_session_id)
                print(json.dumps(result, indent=None if args.follow else 2), flush=True)
                if result["status"] == "timeout":
                    return 2
                if not args.follow:
                    return 0
                args.since = result["next_since"]
                if args.timeout_seconds is not None:
                    remaining = args.timeout_seconds - (time.monotonic() - started)
                    if remaining <= 0:
                        # A perpetually ready source must not extend ownership
                        # beyond the total lifetime. Preserve the last emitted
                        # cursors; a successor can drain remaining board pages.
                        terminal = {**result, "status": "timeout", "messages": []}
                        if control is not None:
                            terminal["control_events"] = []
                        print(json.dumps(terminal), flush=True)
                        return 2
        elif args.command == "topics":
            board, revision = read_board(args.repository)
            view = topics_view(board, since=args.since, limit=args.limit, offset=args.offset)
            print(json.dumps({"revision": revision, **view}, indent=2))
        elif args.command == "roles":
            board, revision = read_board(args.repository)
            print(json.dumps({"revision": revision, **roles_view(board, roles=args.roles)}, indent=2))
        elif args.command == "requests":
            board, revision = read_board(args.repository)
            print(json.dumps({"revision": revision, **requests_view(board, to=args.to)}, indent=2))
        elif args.command == "post":
            print(json.dumps(post(
                args.repository, author=args.author, session_id=args.session_id,
                topic=args.topic, body=args.body, references=args.ref,
                message_id=args.message_id or str(uuid4()), observed_at=timestamp(),
                wake=args.wake, kind=args.kind, role=args.role, to=args.to,
                in_reply_to=args.in_reply_to, supersedes=args.supersedes,
                effects=read_json_file(args.effects_file, "--effects-file"),
                data=read_json_file(args.data_file, "--data-file"),
            ), indent=2))
        elif args.command == "kinds":
            print("\n".join(kinds_view(load_registry(), args.kind)))
        elif args.command == "adopt":
            print(json.dumps(adopt(
                args.repository, author=args.author, session_id=args.session_id, body=args.body,
                role=args.role, predecessor_session=args.predecessor_session,
                message_id=args.message_id or str(uuid4()), observed_at=timestamp(),
                handoff_sha256=args.handoff_sha256, handoff_path=args.handoff_path,
                claims_reclaimed=args.claims_reclaimed, channels=args.channels,
            ), indent=2))
        elif args.command == "claim":
            print(json.dumps(claim(
                args.repository, resource=args.resource, author=args.author,
                session_id=args.session_id, claim_id=args.claim_id or str(uuid4()),
                ttl_seconds=args.ttl_seconds, note=args.note, observed_at=timestamp(),
            ), indent=2))
        else:
            print(json.dumps(release(
                args.repository, resource=args.resource, session_id=args.session_id,
                claim_id=args.claim_id,
            ), indent=2))
    except KeyboardInterrupt:
        if args.command != "wait":
            raise
        print("wait cancelled; board messages remain available", file=sys.stderr)
        return 130
    except (ChatboardError, DurableStateError, OSError, OverflowError) as failure:
        print(f"error: {failure}", file=__import__("sys").stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
