#!/usr/bin/env python3
"""Durable role scheduler daemon and control client."""

from __future__ import annotations

import argparse
import fcntl
import json
import os
import signal
import socket
import socketserver
import sqlite3
import subprocess
import sys
import threading
import time
import uuid
from datetime import datetime, timezone
from pathlib import Path
from typing import Any


def utc_now() -> str:
    return datetime.now(timezone.utc).isoformat()


def epoch(value: str) -> float:
    return datetime.fromisoformat(value.replace("Z", "+00:00")).timestamp()


def normalized_time(value: str) -> str:
    parsed = datetime.fromisoformat(value.replace("Z", "+00:00"))
    if parsed.tzinfo is None:
        raise ValueError("timestamps must include a timezone")
    return parsed.astimezone(timezone.utc).isoformat()


def default_state_dir() -> Path:
    result = subprocess.run(
        ["git", "rev-parse", "--git-common-dir"],
        check=True,
        stdout=subprocess.PIPE,
        text=True,
    )
    git_dir = Path(result.stdout.strip())
    if not git_dir.is_absolute():
        git_dir = Path.cwd() / git_dir
    return git_dir.resolve() / "work-engine" / "role-scheduler"


class Store:
    def __init__(self, path: Path):
        self.path = path
        self.path.parent.mkdir(parents=True, exist_ok=True)
        self.initialize()

    def connect(self) -> sqlite3.Connection:
        db = sqlite3.connect(self.path, timeout=10)
        db.row_factory = sqlite3.Row
        return db

    def initialize(self) -> None:
        with self.connect() as db:
            db.executescript(
                """
                PRAGMA journal_mode=WAL;
                CREATE TABLE IF NOT EXISTS metadata (
                    key TEXT PRIMARY KEY,
                    value INTEGER NOT NULL
                );
                INSERT OR IGNORE INTO metadata(key, value) VALUES ('revision', 0);
                CREATE TABLE IF NOT EXISTS scheduled_items (
                    id TEXT PRIMARY KEY,
                    repository_id TEXT NOT NULL,
                    logical_role TEXT NOT NULL,
                    logical_agent_id TEXT,
                    intent TEXT NOT NULL,
                    due_at TEXT NOT NULL,
                    on_due TEXT NOT NULL,
                    approval_policy TEXT NOT NULL,
                    catch_up_policy TEXT NOT NULL,
                    result_policy TEXT NOT NULL,
                    payload_json TEXT NOT NULL,
                    status TEXT NOT NULL,
                    created_at TEXT NOT NULL,
                    updated_at TEXT NOT NULL,
                    presented_at TEXT,
                    result_ref TEXT
                );
                CREATE INDEX IF NOT EXISTS scheduled_items_recipient_due
                ON scheduled_items(repository_id, logical_role, logical_agent_id, status, due_at);
                """
            )

    @staticmethod
    def bump(db: sqlite3.Connection) -> int:
        db.execute("UPDATE metadata SET value = value + 1 WHERE key = 'revision'")
        return int(db.execute("SELECT value FROM metadata WHERE key = 'revision'").fetchone()[0])

    def revision(self) -> int:
        with self.connect() as db:
            return int(db.execute("SELECT value FROM metadata WHERE key = 'revision'").fetchone()[0])

    def schedule(self, item: dict[str, Any]) -> dict[str, Any]:
        now = utc_now()
        item_id = item.get("id") or str(uuid.uuid4())
        with self.connect() as db:
            db.execute(
                """INSERT INTO scheduled_items
                (id, repository_id, logical_role, logical_agent_id, intent, due_at,
                 on_due, approval_policy, catch_up_policy, result_policy,
                 payload_json, status, created_at, updated_at)
                VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, 'scheduled', ?, ?)""",
                (
                    item_id,
                    item["repository_id"],
                    item["logical_role"],
                    item.get("logical_agent_id"),
                    item["intent"],
                    normalized_time(item["due_at"]),
                    item["on_due"],
                    item["approval_policy"],
                    item["catch_up_policy"],
                    item["result_policy"],
                    json.dumps(item.get("payload", {}), sort_keys=True),
                    now,
                    now,
                ),
            )
            revision = self.bump(db)
        return {"id": item_id, "status": "scheduled", "revision": revision}

    @staticmethod
    def recipient_sql(request: dict[str, Any]) -> tuple[str, list[Any]]:
        sql = "repository_id = ? AND logical_role = ?"
        values: list[Any] = [request["repository_id"], request["logical_role"]]
        agent = request.get("logical_agent_id")
        if agent:
            sql += " AND (logical_agent_id IS NULL OR logical_agent_id = ?)"
            values.append(agent)
        else:
            sql += " AND logical_agent_id IS NULL"
        return sql, values

    @staticmethod
    def decode(row: sqlite3.Row) -> dict[str, Any]:
        value = dict(row)
        value["payload"] = json.loads(value.pop("payload_json"))
        return value

    def agenda(self, request: dict[str, Any]) -> dict[str, Any]:
        now_text = normalized_time(request.get("now") or utc_now())
        now_value = epoch(now_text)
        horizon = now_value + float(request.get("upcoming_horizon_seconds", 900))
        where, values = self.recipient_sql(request)
        with self.connect() as db:
            rows = db.execute(
                f"SELECT * FROM scheduled_items WHERE {where} AND status NOT IN ('acknowledged', 'cancelled', 'expired', 'superseded') ORDER BY due_at, id",
                values,
            ).fetchall()
            revision = int(db.execute("SELECT value FROM metadata WHERE key = 'revision'").fetchone()[0])
        groups = {"overdue": [], "due": [], "upcoming": [], "presented": []}
        for row in rows:
            item = self.decode(row)
            if item["status"] == "presented":
                groups["presented"].append(item)
            elif epoch(item["due_at"]) < now_value:
                groups["overdue"].append(item)
            elif epoch(item["due_at"]) <= now_value:
                groups["due"].append(item)
            elif epoch(item["due_at"]) <= horizon:
                groups["upcoming"].append(item)
        return {"now": now_text, "revision": revision, **groups}

    def take_due(self, request: dict[str, Any]) -> dict[str, Any] | None:
        now = normalized_time(request.get("now") or utc_now())
        where, values = self.recipient_sql(request)
        with self.connect() as db:
            db.execute("BEGIN IMMEDIATE")
            row = db.execute(
                f"SELECT * FROM scheduled_items WHERE {where} AND status = 'scheduled' AND due_at <= ? ORDER BY due_at, id LIMIT 1",
                [*values, now],
            ).fetchone()
            if row is None:
                db.commit()
                return None
            db.execute(
                "UPDATE scheduled_items SET status = 'presented', presented_at = ?, updated_at = ? WHERE id = ? AND status = 'scheduled'",
                (now, now, row["id"]),
            )
            revision = self.bump(db)
            updated = db.execute("SELECT * FROM scheduled_items WHERE id = ?", (row["id"],)).fetchone()
            db.commit()
        return {"event": "scheduled_item_due", "revision": revision, "item": self.decode(updated)}

    def finish(self, item_id: str, status: str, result_ref: str | None) -> dict[str, Any]:
        if status not in {"acknowledged", "cancelled"}:
            raise ValueError("unsupported terminal status")
        now = utc_now()
        with self.connect() as db:
            changed = db.execute(
                "UPDATE scheduled_items SET status = ?, result_ref = ?, updated_at = ? WHERE id = ? AND status IN ('scheduled', 'presented')",
                (status, result_ref, now, item_id),
            ).rowcount
            if changed != 1:
                raise ValueError("scheduled item is missing or already terminal")
            revision = self.bump(db)
        return {"id": item_id, "status": status, "revision": revision}


class SchedulerServer(socketserver.ThreadingUnixStreamServer):
    daemon_threads = True

    def __init__(self, socket_path: Path, store: Store):
        self.store = store
        self.changed = threading.Condition()
        super().__init__(str(socket_path), RequestHandler)

    def notify(self) -> None:
        with self.changed:
            self.changed.notify_all()


class RequestHandler(socketserver.StreamRequestHandler):
    def handle(self) -> None:
        request = json.loads(self.rfile.readline())
        try:
            response = self.dispatch(request)
        except Exception as exc:  # deterministic protocol error surface
            response = {"ok": False, "error": str(exc)}
        self.wfile.write((json.dumps(response, sort_keys=True) + "\n").encode())

    def dispatch(self, request: dict[str, Any]) -> dict[str, Any]:
        server: SchedulerServer = self.server  # type: ignore[assignment]
        operation = request["operation"]
        if operation == "health":
            return {"ok": True, "revision": server.store.revision()}
        if operation == "schedule":
            result = server.store.schedule(request)
            server.notify()
            return {"ok": True, **result}
        if operation == "agenda":
            return {"ok": True, **server.store.agenda(request)}
        if operation == "wait":
            while True:
                event = server.store.take_due(request)
                if event:
                    return {"ok": True, **event}
                with server.changed:
                    server.changed.wait(timeout=1.0)
        if operation in {"acknowledge", "cancel"}:
            status = "acknowledged" if operation == "acknowledge" else "cancelled"
            result = server.store.finish(request["id"], status, request.get("result_ref"))
            server.notify()
            return {"ok": True, **result}
        if operation == "stop":
            threading.Thread(target=server.shutdown, daemon=True).start()
            return {"ok": True, "status": "stopping"}
        raise ValueError(f"unknown operation: {operation}")


def run_daemon(state_dir: Path) -> None:
    state_dir.mkdir(parents=True, exist_ok=True)
    daemon_lock = open(state_dir / "daemon.lock", "a+b")
    try:
        fcntl.flock(daemon_lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
    except BlockingIOError:
        return
    socket_path = state_dir / "scheduler.sock"
    if socket_path.exists():
        socket_path.unlink()
    server = SchedulerServer(socket_path, Store(state_dir / "scheduler.sqlite3"))
    (state_dir / "scheduler.pid").write_text(f"{os.getpid()}\n")
    signal.signal(signal.SIGTERM, lambda *_: threading.Thread(target=server.shutdown, daemon=True).start())
    try:
        server.serve_forever(poll_interval=0.2)
    finally:
        server.server_close()
        socket_path.unlink(missing_ok=True)
        (state_dir / "scheduler.pid").unlink(missing_ok=True)


def request(state_dir: Path, payload: dict[str, Any], *, timeout: float | None = 5) -> dict[str, Any]:
    with socket.socket(socket.AF_UNIX, socket.SOCK_STREAM) as client:
        client.settimeout(timeout)
        client.connect(str(state_dir / "scheduler.sock"))
        client.sendall((json.dumps(payload, sort_keys=True) + "\n").encode())
        chunks = bytearray()
        while not chunks.endswith(b"\n"):
            chunk = client.recv(65536)
            if not chunk:
                break
            chunks.extend(chunk)
    response = json.loads(chunks)
    if not response.get("ok"):
        raise RuntimeError(response.get("error", "scheduler request failed"))
    return response


def ensure(state_dir: Path) -> dict[str, Any]:
    try:
        return request(state_dir, {"operation": "health"}, timeout=0.25)
    except (OSError, RuntimeError):
        pass
    state_dir.mkdir(parents=True, exist_ok=True)
    startup_lock = open(state_dir / "startup.lock", "a+b")
    fcntl.flock(startup_lock, fcntl.LOCK_EX)
    try:
        try:
            return request(state_dir, {"operation": "health"}, timeout=0.25)
        except (OSError, RuntimeError):
            pass
        with open(state_dir / "scheduler.log", "ab", buffering=0) as log:
            subprocess.Popen(
                [sys.executable, str(Path(__file__).resolve()), "--state-dir", str(state_dir), "daemon"],
                stdin=subprocess.DEVNULL,
                stdout=log,
                stderr=log,
                start_new_session=True,
                close_fds=True,
            )
        deadline = time.monotonic() + 5
        while time.monotonic() < deadline:
            try:
                return request(state_dir, {"operation": "health"}, timeout=0.25)
            except (OSError, RuntimeError):
                time.sleep(0.05)
        raise RuntimeError("scheduler daemon did not become ready")
    finally:
        fcntl.flock(startup_lock, fcntl.LOCK_UN)
        startup_lock.close()


def identity(args: argparse.Namespace) -> dict[str, Any]:
    value = {"repository_id": args.repository_id, "logical_role": args.role}
    if args.agent:
        value["logical_agent_id"] = args.agent
    return value


def parser() -> argparse.ArgumentParser:
    root = argparse.ArgumentParser()
    root.add_argument("--state-dir", type=Path)
    commands = root.add_subparsers(dest="command", required=True)
    commands.add_parser("daemon")
    commands.add_parser("ensure")
    commands.add_parser("status")
    commands.add_parser("stop")
    for name in ("agenda", "wait", "subscribe"):
        item = commands.add_parser(name)
        item.add_argument("--repository-id", required=True)
        item.add_argument("--role", required=True)
        item.add_argument("--agent")
        if name == "agenda":
            item.add_argument("--horizon-seconds", type=int, default=900)
    schedule = commands.add_parser("schedule")
    schedule.add_argument("--id")
    schedule.add_argument("--repository-id", required=True)
    schedule.add_argument("--role", required=True)
    schedule.add_argument("--agent")
    schedule.add_argument("--intent", required=True)
    schedule.add_argument("--due-at", required=True)
    schedule.add_argument("--on-due", default="request_approval")
    schedule.add_argument("--approval-policy", default="require_confirmation")
    schedule.add_argument("--catch-up-policy", default="ask_if_missed")
    schedule.add_argument("--result-policy", default="present_proposed_action")
    for name in ("acknowledge", "cancel"):
        item = commands.add_parser(name)
        item.add_argument("--id", required=True)
        item.add_argument("--result-ref")
    return root


def main() -> int:
    args = parser().parse_args()
    state_dir = (args.state_dir or default_state_dir()).resolve()
    if args.command == "daemon":
        run_daemon(state_dir)
        return 0
    if args.command == "status":
        print(json.dumps(request(state_dir, {"operation": "health"}), sort_keys=True))
        return 0
    if args.command == "stop":
        try:
            response = request(state_dir, {"operation": "stop"})
        except OSError:
            response = {"ok": True, "status": "not_running"}
        print(json.dumps(response, sort_keys=True))
        return 0
    ensure(state_dir)
    if args.command == "ensure":
        response = request(state_dir, {"operation": "health"})
    elif args.command == "schedule":
        response = request(state_dir, {
            "operation": "schedule", **identity(args), "id": args.id,
            "intent": args.intent, "due_at": args.due_at, "on_due": args.on_due,
            "approval_policy": args.approval_policy,
            "catch_up_policy": args.catch_up_policy, "result_policy": args.result_policy,
        })
    elif args.command == "agenda":
        response = request(state_dir, {
            "operation": "agenda", **identity(args),
            "upcoming_horizon_seconds": args.horizon_seconds,
        })
    elif args.command == "wait":
        response = request(state_dir, {"operation": "wait", **identity(args)}, timeout=None)
    elif args.command == "subscribe":
        while True:
            response = request(state_dir, {"operation": "wait", **identity(args)}, timeout=None)
            print(json.dumps(response, sort_keys=True), flush=True)
        return 0
    else:
        response = request(state_dir, {
            "operation": args.command, "id": args.id, "result_ref": args.result_ref,
        })
    print(json.dumps(response, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
