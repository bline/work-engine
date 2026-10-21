#!/usr/bin/env python3
"""Append-only cross-session Codex messages and advisory resource claims."""

from __future__ import annotations

import argparse
import ctypes
from datetime import datetime, timedelta, timezone
import errno
import json
import math
import os
from pathlib import Path
import select
import struct
import sys
import time
from typing import Any, Callable
from uuid import UUID, uuid4

from durable_state import DurableStateError, GitRefDurableStateStore


BOARD_KEY = "work-engine/codex-chatboard/v1"
NAMESPACE = "codex-chatboard"
MAX_RETRIES = 20


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


def post(repository: Path, *, author: str, session_id: str, topic: str,
         body: str, references: list[str], message_id: str, observed_at: str) -> dict[str, Any]:
    canonical_uuid(session_id, "session_id")
    canonical_uuid(message_id, "message_id")
    if not all(isinstance(value, str) and value.strip() for value in (author, topic, body)):
        raise ChatboardError("author, topic, and body must be nonempty")

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
        if existing:
            if existing != candidate:
                raise ChatboardError("message_id already exists with different content")
            return existing
        board["messages"].append(candidate)
        board["next_sequence"] += 1
        return candidate

    _, message, revision = mutate(repository, change)
    return {"revision": revision, "message": message}


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


def read_view(board: dict[str, Any], *, since: int | None, limit: int,
              active_claims: bool, observed_at: str) -> dict[str, Any]:
    # Without --since, a startup read needs the latest coordination, not the
    # oldest retained messages. With --since, page forward from that sequence.
    if since is None:
        rows = board["messages"][-limit:]
    else:
        rows = [row for row in board["messages"] if row["sequence"] > since][:limit]
    claims = board["claims"]
    if active_claims:
        now = parse_time(observed_at)
        claims = {resource: value for resource, value in claims.items()
                  if parse_time(value["expires_at"]) > now}
    return {"messages": rows, "claims": claims}


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
                        break  # the nearest existing ancestor is already watched
                    raise OSError(error, "cannot watch board ref directory")
                watches[wd] = names
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


def wait_board(repository: Path, *, since: int, limit: int = 100,
               topic: str | None = None, topics: list[str] | None = None,
               timeout_seconds: float | None = None) -> dict[str, Any]:
    if since < 0 or not 1 <= limit <= 500:
        raise ChatboardError("--since must be nonnegative and --limit must be 1..500")
    selected = [*(topics or []), *([topic] if topic is not None else [])]
    if any(not name.strip() for name in selected):
        raise ChatboardError("--topic must be nonempty")
    selected = sorted(set(selected))
    if timeout_seconds is not None and (not math.isfinite(timeout_seconds) or timeout_seconds < 0):
        raise ChatboardError("--timeout-seconds must be finite and nonnegative")
    deadline = None if timeout_seconds is None else time.monotonic() + timeout_seconds
    # Subscribe first, then read: a publication during the read remains queued.
    with RefChanges(repository) as changes:
        while True:
            board, revision = read_board(repository)
            matches = [row for row in board["messages"] if row["sequence"] > since
                       and (not selected or row["topic"] in selected)]
            rows = matches[:limit]
            more = len(matches) > limit
            cursor = rows[-1]["sequence"] if more else max(since, board["next_sequence"] - 1)
            result = {"status": "messages" if rows else "timeout", "revision": revision,
                      "messages": rows, "next_since": cursor, "has_more": more,
                      "topics": selected, "topic": selected[0] if len(selected) == 1 else None}
            if rows or (deadline is not None and time.monotonic() >= deadline):
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


def parser() -> argparse.ArgumentParser:
    root = argparse.ArgumentParser(description=__doc__)
    root.add_argument("--repository", type=Path, default=Path.cwd())
    commands = root.add_subparsers(dest="command", required=True)
    read = commands.add_parser("read")
    read.add_argument("--since", type=int, default=None)
    read.add_argument("--limit", type=int, default=100)
    read.add_argument("--active-claims", action="store_true")
    wait = commands.add_parser("wait", help="block for new messages without periodic polling")
    wait.add_argument("--since", type=int, required=True)
    wait.add_argument("--limit", type=int, default=100)
    wait.add_argument("--topic", action="append", dest="topics", default=[],
                      help="exact topic match; repeat to match any selected topic; omit for all")
    wait.add_argument("--timeout-seconds", type=float, default=None,
                      help="default: indefinite; 0 checks once; timeout exits 2")
    topics = commands.add_parser("topics", help="list recorded topics with counts and latest sequence")
    topics.add_argument("--since", type=int, default=0)
    topics.add_argument("--limit", type=int, default=100)
    topics.add_argument("--offset", type=int, default=0)
    post_cmd = commands.add_parser("post")
    for name in ("author", "session-id", "topic", "body"):
        post_cmd.add_argument(f"--{name}", required=True)
    post_cmd.add_argument("--message-id", default=None)
    post_cmd.add_argument("--ref", action="append", default=[])
    claim_cmd = commands.add_parser("claim")
    for name in ("resource", "author", "session-id", "note"):
        claim_cmd.add_argument(f"--{name}", required=True)
    claim_cmd.add_argument("--claim-id", default=None)
    claim_cmd.add_argument("--ttl-seconds", type=int, default=3600)
    release_cmd = commands.add_parser("release")
    for name in ("resource", "session-id", "claim-id"):
        release_cmd.add_argument(f"--{name}", required=True)
    return root


def main() -> int:
    args = parser().parse_args()
    try:
        if args.command == "read":
            if (args.since is not None and args.since < 0) or not 1 <= args.limit <= 500:
                raise ChatboardError("--since must be nonnegative and --limit must be 1..500")
            board, revision = read_board(args.repository)
            view = read_view(board, since=args.since, limit=args.limit,
                             active_claims=args.active_claims, observed_at=timestamp())
            print(json.dumps({"revision": revision, **view}, indent=2))
        elif args.command == "wait":
            result = wait_board(args.repository, since=args.since, limit=args.limit,
                                topics=args.topics, timeout_seconds=args.timeout_seconds)
            print(json.dumps(result, indent=2))
            return 0 if result["status"] == "messages" else 2
        elif args.command == "topics":
            board, revision = read_board(args.repository)
            view = topics_view(board, since=args.since, limit=args.limit, offset=args.offset)
            print(json.dumps({"revision": revision, **view}, indent=2))
        elif args.command == "post":
            print(json.dumps(post(
                args.repository, author=args.author, session_id=args.session_id,
                topic=args.topic, body=args.body, references=args.ref,
                message_id=args.message_id or str(uuid4()), observed_at=timestamp(),
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
