#!/usr/bin/env python3
"""Append-only cross-session Codex messages and advisory resource claims."""

from __future__ import annotations

import argparse
from datetime import datetime, timedelta, timezone
import json
from pathlib import Path
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


def parser() -> argparse.ArgumentParser:
    root = argparse.ArgumentParser(description=__doc__)
    root.add_argument("--repository", type=Path, default=Path.cwd())
    commands = root.add_subparsers(dest="command", required=True)
    read = commands.add_parser("read")
    read.add_argument("--since", type=int, default=0)
    read.add_argument("--limit", type=int, default=100)
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
            if args.since < 0 or not 1 <= args.limit <= 500:
                raise ChatboardError("--since must be nonnegative and --limit must be 1..500")
            board, revision = read_board(args.repository)
            rows = [row for row in board["messages"] if row["sequence"] > args.since][:args.limit]
            print(json.dumps({"revision": revision, "messages": rows, "claims": board["claims"]}, indent=2))
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
    except (ChatboardError, DurableStateError, OSError) as failure:
        print(f"error: {failure}", file=__import__("sys").stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
