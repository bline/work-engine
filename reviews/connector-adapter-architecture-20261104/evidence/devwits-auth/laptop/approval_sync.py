#!/usr/bin/env python3
"""Move owner decisions between the coordination board and the approval mailbox.

  push   open board requests addressed to the owner -> https://auth.devwits.ai mailbox
  pull   signed answers from the mailbox -> verified -> board replies
  sync   push, pull, push
  keys   approval passkeys registered on the site, and which are pinned here
  pin    trust one registered key on this machine (by fingerprint prefix)
  unpin  stop trusting one

Nothing is trusted from the mailbox: an answer reaches the board only if its
passkey signature verifies against a key pinned on this machine (see
owner_answer.py and docs/owner-answer-v1.md). The mailbox token only moves data.
"""
from __future__ import annotations

import argparse
from contextlib import contextmanager, nullcontext
import datetime as dt
import fcntl
import hashlib
import json
import os
import subprocess
import sys
import tempfile
import urllib.error
import urllib.request
import uuid
from pathlib import Path

import owner_answer as oa

BASE_URL = "https://auth.devwits.ai"
OWNER_USER_ID = "user-test-8b1b7b52-0fa4-4b70-8594-b2b043592a1c"
BOARD_REPOSITORY = Path.home() / "code" / "work-engine"
CONFIG_DIR = Path.home() / ".config" / "devwits"
TOKEN_FILE = CONFIG_DIR / "mailbox.env"
PINS_FILE = CONFIG_DIR / "approval-keys.json"
SETTINGS_FILE = CONFIG_DIR / "approval-sync.json"
STATE_DIR = Path.home() / ".local" / "state" / "devwits-approval-sync"
DEFAULT_OWNER_ADDRESSES = ["role:owner"]
AUTHOR = "owner-signed-answer"
SYNC_SESSION_ID = str(uuid.uuid5(uuid.NAMESPACE_URL, "https://auth.devwits.ai/approval-sync"))
# Rejections the owner can fix without re-signing (by pinning the key), so the
# answer is checked again on every pull. Every other rejection is final.
RETRYABLE = {"key_not_pinned"}


class SyncError(Exception):
    pass


# ---- local files

def read_json(path: Path, default):
    try:
        return json.loads(path.read_text())
    except FileNotFoundError:
        return default


def write_private_json(path: Path, value) -> None:
    path.parent.mkdir(parents=True, exist_ok=True, mode=0o700)
    tmp = path.with_suffix(path.suffix + ".tmp")
    fd = os.open(tmp, os.O_WRONLY | os.O_CREAT | os.O_TRUNC, 0o600)
    with os.fdopen(fd, "w") as handle:
        json.dump(value, handle, indent=1, sort_keys=True)
    os.replace(tmp, path)


@contextmanager
def sync_lock():
    """Keep timer and manual runs from racing on answers or the push cache."""
    STATE_DIR.mkdir(parents=True, exist_ok=True, mode=0o700)
    fd = os.open(STATE_DIR / "sync.lock", os.O_WRONLY | os.O_CREAT, 0o600)
    with os.fdopen(fd, "w") as handle:
        try:
            fcntl.flock(handle, fcntl.LOCK_EX | fcntl.LOCK_NB)
        except BlockingIOError:
            yield False
            return
        try:
            yield True
        finally:
            fcntl.flock(handle, fcntl.LOCK_UN)


def settings() -> dict:
    value = read_json(SETTINGS_FILE, {})
    return {"owner_addresses": value.get("owner_addresses") or DEFAULT_OWNER_ADDRESSES,
            "board_repository": Path(value.get("board_repository") or BOARD_REPOSITORY),
            "task_projection_file": value.get("task_projection_file")}


def mailbox_token() -> str:
    for line in TOKEN_FILE.read_text().splitlines():
        if line.startswith("MAILBOX_TOKEN="):
            return line.split("=", 1)[1].strip()
    raise SyncError(f"no MAILBOX_TOKEN in {TOKEN_FILE}")


def pinned_keys() -> dict:
    return read_json(PINS_FILE, {"keys": {}})["keys"]


# ---- mailbox HTTP

def mailbox(method: str, path: str, body=None):
    request = urllib.request.Request(
        BASE_URL + path, method=method,
        data=json.dumps(body).encode() if body is not None else None,
        headers={"Authorization": "Bearer " + mailbox_token(), "Content-Type": "application/json"})
    try:
        with urllib.request.urlopen(request, timeout=30) as response:
            return json.load(response)
    except urllib.error.HTTPError as error:
        try:
            detail = json.load(error)
        except ValueError:
            detail = {}
        raise SyncError(f"{method} {path}: HTTP {error.code} {detail}") from None


# ---- board

def board(repository: Path, *args: str) -> dict:
    script = repository / "skills" / "durable-state" / "scripts" / "codex_chatboard.py"
    result = subprocess.run([sys.executable, str(script), "--repository", str(repository), *args],
                            capture_output=True, text=True, timeout=120)
    if result.returncode != 0:
        raise SyncError(f"board {args[0]} failed: {result.stderr.strip()[-500:]}")
    return json.loads(result.stdout)


def board_row(repository: Path, sequence: int) -> dict | None:
    rows = board(repository, "read", "--since", str(sequence - 1), "--limit", "1")["messages"]
    return rows[0] if rows and rows[0].get("sequence") == sequence else None


def open_owner_requests(repository: Path, addresses: list[str]) -> tuple[str, dict[int, dict]]:
    view = board(repository, "requests")
    sequences = sorted({item["sequence"] for group in view["addressees"] if group["to"] in addresses
                        for item in group["requests"] if item.get("status") == "open"})
    rows = {}
    for sequence in sequences:
        row = board_row(repository, sequence)
        if row is not None:
            rows[sequence] = row
    return view.get("revision", ""), rows


# ---- commands

def task_projection(config: dict):
    """Display transport from a configured owner file, never admission authority.

    Deployment must qualify the producer and protect this path from the worker.
    """
    configured = config.get("task_projection_file")
    if configured is None:
        return None
    if not isinstance(configured, str) or not Path(configured).is_absolute():
        raise SyncError("task_projection_file must be an absolute local path")
    try:
        with Path(configured).open("rb") as handle:
            data = handle.read(2 * 1024 * 1024 + 1)
        if len(data) > 2 * 1024 * 1024:
            raise SyncError("task projection exceeds the snapshot size limit")
        value = json.loads(data)
    except (OSError, ValueError) as error:
        raise SyncError(f"task projection unavailable: {type(error).__name__}") from None
    if (not isinstance(value, dict) or set(value) != {"schema", "observed_at", "source_revision", "tasks"}
            or value.get("schema") != "dot.task-projection/v1-proposal"
            or not isinstance(value.get("observed_at"), str)
            or not isinstance(value.get("source_revision"), str)
            or not isinstance(value.get("tasks"), list) or len(value["tasks"]) > 100):
        raise SyncError("task projection has an unsupported or incomplete envelope")
    return value


def push(config: dict, *, force: bool = False) -> int:
    revision, rows = open_owner_requests(config["board_repository"], config["owner_addresses"])
    requests = [{"board_seq": seq, "request_json": oa.request_json(row), "request_digest": oa.request_digest(row)}
                for seq, row in sorted(rows.items())]
    projection = task_projection(config)
    # Other board activity changes its revision without changing what the owner
    # sees. Cache the open set, including its destination and routing scope.
    scope = {"base_url": BASE_URL, "repository": str(config["board_repository"].resolve()),
             "owner_addresses": sorted(set(config["owner_addresses"])), "requests": requests,
             "task_projection": projection}
    digest = hashlib.sha256(json.dumps(scope, sort_keys=True, separators=(",", ":"),
                                       ensure_ascii=False).encode()).hexdigest()
    cache_file = STATE_DIR / "last-push.json"
    try:
        previous = read_json(cache_file, {})
    except (OSError, ValueError):
        previous = {}
    if not force and isinstance(previous, dict) and previous.get("v") == 1 and previous.get("digest") == digest:
        print(f"push: unchanged ({len(requests)} open request(s)); upload skipped")
        return len(requests)
    result = mailbox("PUT", "/api/requests", {"owner_addresses": config["owner_addresses"],
                                              "board_revision": revision, "requests": requests,
                                              "task_projection": projection})
    # A failed upload must remain retryable on the next run.
    write_private_json(cache_file, {"v": 1, "digest": digest, "count": result["count"],
                                    "pushed_at": dt.datetime.now(dt.timezone.utc).isoformat()})
    print(f"push: {result['count']} open request(s) for {', '.join(config['owner_addresses'])}")
    return result["count"]


def post_reply(repository: Path, answer: dict, row: dict, answer_json: str, credential: dict, pin: dict) -> int:
    decision = answer["decision"]
    data = {
        "disposition": oa.DISPOSITIONS[decision],
        "decision": decision,
        "comment": answer["comment"],
        "answer_id": answer["answer_id"],
        "signed_answer": {
            "format": "work-engine.owner-answer/v1",
            "answer_json": answer_json,
            "credential_id": credential["id"],
            "authenticator_data": credential["response"]["authenticatorData"],
            "client_data_json": credential["response"]["clientDataJSON"],
            "signature": credential["response"]["signature"],
        },
        "verified": {"by": "approval_sync.py", "key_fingerprint": pin["fingerprint"],
                     "at": dt.datetime.now(dt.timezone.utc).isoformat()},
    }
    body = f"Owner answer, passkey-signed: {decision.upper()}" + (f". {answer['comment']}" if answer["comment"] else "")
    STATE_DIR.mkdir(parents=True, exist_ok=True, mode=0o700)
    with tempfile.NamedTemporaryFile("w", dir=STATE_DIR, suffix=".json", delete=False) as handle:
        json.dump(data, handle)
        data_file = handle.name
    try:
        args = ["post", "--author", AUTHOR, "--session-id", SYNC_SESSION_ID, "--topic", row["topic"],
                "--kind", "reply", "--reply-to", str(row["sequence"]), "--message-id", answer["answer_id"],
                "--data-file", data_file, "--body", body]
        if row.get("session_id"):
            args += ["--to", f"session:{row['session_id']}"]
        try:
            return board(repository, *args)["message"]["sequence"]
        except SyncError as error:
            if "message_id already exists" in str(error):
                return -1  # posted on an earlier run
            raise
    finally:
        os.unlink(data_file)


def pull(config: dict) -> None:
    repository = config["board_repository"]
    state_file = STATE_DIR / "answers.json"
    state = read_json(state_file, {"posted": {}, "rejected": {}})
    answers = mailbox("GET", "/api/answers")["answers"]
    if not answers:
        print("pull: no answers waiting")
        return
    _, open_rows = open_owner_requests(repository, config["owner_addresses"])
    pins = pinned_keys()
    for stored in answers:
        answer_json, credential = stored["answer_json"], stored["credential"]
        try:
            answer_id = json.loads(answer_json)["answer_id"]
        except (ValueError, KeyError, TypeError):
            print("pull: skipped an unreadable answer")
            continue
        if answer_id in state["posted"]:
            mailbox("DELETE", f"/api/answers?answer_id={answer_id}")
            continue
        if answer_id in state["rejected"] and state["rejected"][answer_id]["code"] not in RETRYABLE:
            continue
        seq = json.loads(answer_json).get("board_seq")
        try:
            answer = oa.verify_answer(answer_json, credential, pinned_keys=pins, owner=OWNER_USER_ID,
                                      request_row=open_rows.get(seq))
        except oa.AnswerRejected as rejected:
            state["rejected"][answer_id] = {"code": rejected.code, "at": dt.datetime.now(dt.timezone.utc).isoformat()}
            write_private_json(state_file, state)
            print(f"pull: REJECTED answer {answer_id} for #{seq}: {rejected}")
            continue
        reply = post_reply(repository, answer, open_rows[seq], answer_json, credential, pins[credential["id"]])
        state["rejected"].pop(answer_id, None)
        state["posted"][answer_id] = {"board_seq": seq, "reply_seq": reply}
        write_private_json(state_file, state)
        mailbox("DELETE", f"/api/answers?answer_id={answer_id}")
        print(f"pull: #{seq} {answer['decision']} posted as board #{reply}" if reply > 0
              else f"pull: #{seq} {answer['decision']} was already on the board")


def keys(_config: dict) -> None:
    remote = mailbox("GET", "/api/approval-keys")["keys"]
    pins = pinned_keys()
    if not remote:
        print("no approval passkeys registered (https://auth.devwits.ai/approve/setup)")
    for key in remote:
        mark = "PINNED" if key["credential_id"] in pins else "not pinned"
        sync = "synced" if key.get("backed_up") else "device-bound"
        print(f"{key['fingerprint'][:16]}  {mark:10}  {sync:12}  registered {key['created_at']}")


def pin(_config: dict, prefix: str) -> None:
    if len(prefix) < 8:
        raise SyncError("give at least 8 hex characters of the fingerprint")
    matches = [k for k in mailbox("GET", "/api/approval-keys")["keys"] if k["fingerprint"].startswith(prefix.lower())]
    if len(matches) != 1:
        raise SyncError(f"{len(matches)} registered keys match {prefix!r}; need exactly one")
    key = matches[0]
    spki = oa.b64url_decode(key["spki_b64url"])
    if oa.key_fingerprint(spki) != key["fingerprint"]:
        raise SyncError("the key's fingerprint does not match its public key")
    pins = read_json(PINS_FILE, {"keys": {}})
    pins["keys"][key["credential_id"]] = {"spki_b64url": key["spki_b64url"], "fingerprint": key["fingerprint"],
                                          "cose_alg": key.get("cose_alg"), "backed_up": key.get("backed_up"),
                                          "pinned_at": dt.datetime.now(dt.timezone.utc).isoformat()}
    write_private_json(PINS_FILE, pins)
    print(f"pinned {key['fingerprint']} in {PINS_FILE}")


def unpin(_config: dict, prefix: str) -> None:
    pins = read_json(PINS_FILE, {"keys": {}})
    gone = [cid for cid, k in pins["keys"].items() if k["fingerprint"].startswith(prefix.lower())]
    if len(gone) != 1:
        raise SyncError(f"{len(gone)} pinned keys match {prefix!r}; need exactly one")
    del pins["keys"][gone[0]]
    write_private_json(PINS_FILE, pins)
    print(f"unpinned {prefix}")


def main(argv=None) -> int:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--owner-address", action="append",
                        help=f"board addressee that means the owner (repeatable; default {DEFAULT_OWNER_ADDRESSES})")
    commands = parser.add_subparsers(dest="command", required=True)
    for name in ("push", "pull", "sync", "keys"):
        command = commands.add_parser(name)
        if name in {"push", "sync"}:
            command.add_argument("--force", action="store_true", help="upload the snapshot even if unchanged")
    for name in ("pin", "unpin"):
        commands.add_parser(name).add_argument("fingerprint_prefix")
    args = parser.parse_args(argv)
    config = settings()
    if args.owner_address:
        config["owner_addresses"] = args.owner_address
    try:
        lock = sync_lock() if args.command in {"push", "pull", "sync"} else nullcontext(True)
        with lock as acquired:
            if not acquired:
                print("sync: another process is already running; skipped")
                return 0
            if args.command == "push":
                push(config, force=args.force)
            elif args.command == "pull":
                pull(config)
            elif args.command == "sync":
                push(config, force=args.force)
                pull(config)
                push(config)
            elif args.command == "keys":
                keys(config)
            elif args.command == "pin":
                pin(config, args.fingerprint_prefix)
            elif args.command == "unpin":
                unpin(config, args.fingerprint_prefix)
    except SyncError as error:
        print(f"error: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
