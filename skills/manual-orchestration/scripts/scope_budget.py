#!/usr/bin/env python3
"""Keep a supervisor's original scope, additions and budget in its assignment record.

The record is a JSON object file; this script owns only its `scope_budget` key,
so it can live in the existing assignment record. `init` writes the original
items and limits, `add` appends one scope addition with its reason, `revise`
changes a limit (keeping the old one as history), and `check` prints one JSON
line: elapsed hours, additions, the weekly quota meter when a Codex rollout is
given, and whether to reconsider or stop.

Exit codes for `check`: 0 within budget, 3 reconsider (additions reached their
trigger), 4 stop (hours or quota reached). Other commands return 0. All return
2 for a usage error.
"""

from __future__ import annotations

import argparse
import fcntl
import tempfile
import hashlib
import subprocess
import datetime as dt
import json
import os
import signal
import time
import sys
import uuid
import math
from pathlib import Path
from typing import Any

KEY = "scope_budget"
SCHEMA_V1 = "manual-orchestration.scope-budget.v1"
SCHEMA = "manual-orchestration.scope-budget.v2"
WEEK_MINUTES = 10080
LIMITS = ("max_hours", "max_weekly_points", "max_weekly_percent", "reconsider_after_additions")


class UsageError(Exception):
    pass


def now() -> dt.datetime:
    return dt.datetime.now(dt.timezone.utc)


def stamp(t: dt.datetime) -> str:
    return t.isoformat(timespec="seconds").replace("+00:00", "Z")


def parse(s: str) -> dt.datetime:
    return dt.datetime.fromisoformat(s.replace("Z", "+00:00"))


def load(path: Path) -> dict[str, Any]:
    if not path.exists():
        return {}
    doc = json.loads(path.read_text())
    if not isinstance(doc, dict):
        raise UsageError(f"{path} is not a JSON object")
    return doc


def save(path: Path, doc: dict[str, Any]) -> None:
    tmp = None
    try:
        with tempfile.NamedTemporaryFile(mode="w", encoding="utf-8", dir=path.parent,
                                         prefix=path.name + ".", suffix=".tmp", delete=False) as f:
            tmp = Path(f.name)
            f.write(json.dumps(doc, indent=2, ensure_ascii=False) + "\n")
        os.replace(tmp, path)
    finally:
        if tmp is not None:
            tmp.unlink(missing_ok=True)


def weekly_percent(rollout: Path, block: int = 1 << 20) -> dict[str, Any] | None:
    """Latest Codex 7-day meter from a rollout, read backwards from the end."""
    with rollout.open("rb") as f:
        f.seek(0, os.SEEK_END)
        pos, tail = f.tell(), b""
        while pos > 0:
            step = min(block, pos)
            pos -= step
            f.seek(pos)
            chunk = f.read(step) + tail
            lines = chunk.split(b"\n")
            tail = lines[0] if pos > 0 else b""
            for line in reversed(lines[1:] if pos > 0 else lines):
                if b'"rate_limits"' not in line:
                    continue
                try:
                    rec = json.loads(line)
                except ValueError:
                    continue
                limits = (rec.get("payload") or {}).get("rate_limits") or {}
                for slot in ("primary", "secondary"):
                    w = limits.get(slot)
                    if isinstance(w, dict) and w.get("window_minutes") == WEEK_MINUTES and w.get("used_percent") is not None:
                        return {"used_percent": float(w["used_percent"]), "observed_at": rec.get("timestamp"),
                                "resets_at": w.get("resets_at")}
    return None


def section(doc: dict[str, Any], path: Path) -> dict[str, Any]:
    sb = doc.get(KEY)
    if not isinstance(sb, dict) or sb.get("schema") not in (SCHEMA_V1, SCHEMA):
        raise UsageError(f"{path} has no {KEY} record; run init first")
    return sb



def require_open(sb: dict[str, Any]) -> None:
    if "closed_at" in sb:
        raise UsageError("objective is closed; start a new one with init")


def upgrade(sb: dict[str, Any]) -> None:
    if sb["schema"] == SCHEMA_V1:
        sb.update(schema=SCHEMA, objective_id=str(uuid.uuid4()), sessions=[], bindings=[], cost_limits=[])


def reject_points(args: argparse.Namespace) -> None:
    if args.max_weekly_points is not None:
        raise UsageError("--max-weekly-points is no longer accepted; use cost-limit")


def setter(value: str) -> str:
    return "supervisor" if value == "coordinator" else value

def limits_from(args: argparse.Namespace) -> dict[str, Any]:
    return {k: getattr(args, k) for k in LIMITS if getattr(args, k) is not None}


def cmd_init(args: argparse.Namespace) -> int:
    path = Path(args.record)
    doc = load(path)
    if KEY in doc:
        raise UsageError(f"{path} already has a {KEY} record; use revise or add")
    reject_points(args)
    if bool(args.session) != bool(args.provider):
        raise UsageError("--session and --provider must be supplied together")
    limits = limits_from(args)
    if not ({"max_hours", "max_weekly_points", "max_weekly_percent"} & set(limits)):
        raise UsageError("set at least one of --max-hours, --max-weekly-points, --max-weekly-percent")
    started = now()
    meter = weekly_percent(Path(args.rollout)) if args.rollout else None
    if args.max_weekly_points is not None and meter is None:
        raise UsageError("--max-weekly-points needs --rollout with a readable weekly meter")
    doc[KEY] = {"schema": SCHEMA, "objective_id": str(uuid.uuid4()),
                "sessions": ([{"session_id": args.session, "provider": args.provider, "from": stamp(started), "bound_by": "init", "evidence": None}] if args.session else []),
                "bindings": [], "cost_limits": [], "objective": args.objective, "original_items": args.item,
                "started_at": stamp(started), "weekly_at_start": meter,
                "limits": limits, "limits_set_by": args.set_by, "limits_reason": args.reason,
                "limit_history": [], "additions": []}
    save(path, doc)
    print(json.dumps({"record": str(path), "started_at": stamp(started), "limits": limits,
                      "weekly_at_start": meter and meter["used_percent"]}))
    return 0


def cmd_add(args: argparse.Namespace) -> int:
    path = Path(args.record)
    doc = load(path)
    sb = section(doc, path)
    require_open(sb)
    upgrade(sb)
    sb["additions"].append({"at": stamp(now()), "item": args.item, "reason": args.reason})
    save(path, doc)
    n = len(sb["additions"])
    trigger = sb["limits"].get("reconsider_after_additions")
    print(json.dumps({"additions": n, "reconsider_after_additions": trigger,
                      "reconsider": trigger is not None and n >= trigger}))
    return 0


def cmd_revise(args: argparse.Namespace) -> int:
    path = Path(args.record)
    doc = load(path)
    sb = section(doc, path)
    require_open(sb)
    reject_points(args)
    changed = limits_from(args)
    if not changed:
        raise UsageError("give at least one limit to revise")
    upgrade(sb)
    sb["limit_history"].append({"until": stamp(now()), "limits": sb["limits"],
                                "set_by": setter(sb["limits_set_by"]), "reason": sb.get("limits_reason")})
    sb["limits"] = {**sb["limits"], **changed}
    sb["limits_set_by"], sb["limits_reason"] = args.set_by, args.reason
    save(path, doc)
    print(json.dumps({"limits": sb["limits"], "set_by": args.set_by}))
    return 0



def cmd_add_session(args: argparse.Namespace) -> int:
    path = Path(args.record)
    doc = load(path)
    sb = section(doc, path)
    require_open(sb)
    parse(args.from_time)
    entry = {"session_id": args.session, "provider": args.provider, "from": args.from_time,
             "bound_by": args.bound_by, "evidence": args.evidence}
    existing = next((x for x in sb.get("sessions", []) if x["session_id"] == args.session), None)
    if existing is not None:
        if {**existing, "bound_by": setter(existing["bound_by"])} != entry:
            raise UsageError("session already exists with different binding")
        print(json.dumps({"session": existing, "added": False}))
        return 0
    upgrade(sb)
    sb["sessions"].append(entry)
    save(path, doc)
    print(json.dumps({"session": entry, "added": True}))
    return 0


def cmd_bind(args: argparse.Namespace) -> int:
    path = Path(args.record)
    doc = load(path)
    sb = section(doc, path)
    require_open(sb)
    upgrade(sb)
    entry = {"session_id": args.session, "provider": args.provider, "parent_session_id": args.parent,
             "bound_at": stamp(now()), "bound_by": args.bound_by, "reason": args.reason}
    sb["bindings"].append(entry)
    save(path, doc)
    print(json.dumps({"binding": entry}))
    return 0


def cmd_cost_limit(args: argparse.Namespace) -> int:
    values = ([args.max_usd] if args.max_usd is not None else []) + args.mark_usd
    if not values or any(not math.isfinite(x) or x < 0 for x in values):
        raise UsageError("give nonnegative finite --max-usd or --mark-usd")
    path = Path(args.record)
    doc = load(path)
    sb = section(doc, path)
    require_open(sb)
    upgrade(sb)
    entry = {"pool": args.pool, "node": args.node, "max_usd": args.max_usd,
             "marks_usd": sorted(set(args.mark_usd)), "set_by": args.set_by,
             "set_at": stamp(now()), "reason": args.reason}
    previous = next ( (x for x in sb["cost_limits"] if x["pool"] == args.pool and x["node"] == args.node and setter(x["set_by"]) == args.set_by), None)
    sb["cost_limits"] = [x for x in sb["cost_limits"] if (x["pool"], x["node"], setter(x["set_by"])) != (args.pool, args.node, args.set_by)]
    sb["cost_limits"].append(entry)
    if previous is not None:
        previous = {**previous, "set_by": setter(previous["set_by"])}
    sb["limit_history"].append({"cost_limit": entry, "previous_cost_limit": previous})
    save(path, doc)
    print(json.dumps({"cost_limit": entry}))
    return 0

def listener_state(sb: dict[str, Any], path: Path) -> dict[str, Any]:
    entry = sb.get("listener")
    if not entry:
        return {"state": "none", "pid": None}
    pid = entry.get("pid")
    try:
        parts = Path(f"/proc/{pid}/cmdline").read_bytes().split(b"\0")
        alive = str(path.resolve()).encode() in parts
    except (OSError, ValueError):
        alive = False
    return {"state": "alive" if alive else "exited", "pid": pid}


def cmd_close(args: argparse.Namespace) -> int:
    path = Path(args.record)
    doc = load(path)
    sb = section(doc, path)
    require_open(sb)
    at = parse(args.at) if args.at else now()
    if at.tzinfo is None:
        raise UsageError("--at must include a timezone")
    if at < parse(sb["started_at"]) or at > now():
        raise UsageError("--at must be between started_at and now")
    closed_at = stamp(at.astimezone(dt.timezone.utc))
    entry = {"at": closed_at, "disposition": args.disposition, "reason": args.reason, "set_by": args.set_by}
    sb["closed_at"], sb["close"] = closed_at, entry
    save(path, doc)
    print(json.dumps(entry))
    return 0


def stop_listener_group(proc: subprocess.Popen) -> None:
    """Stop the detached listener and its readers after publication fails."""
    pgid = proc.pid  # start_new_session makes the child the group leader.
    def signal_group(sig: int) -> bool:
        try:
            os.killpg(pgid, sig)
            return True
        except ProcessLookupError:
            return False

    try:
        if signal_group(signal.SIGTERM):
            deadline = time.monotonic() + 5.0
            while True:
                proc.poll()  # Reap the leader so it cannot keep the group present.
                if not signal_group(0):
                    break
                remaining = deadline - time.monotonic()
                if remaining <= 0:
                    signal_group(signal.SIGKILL)
                    break
                time.sleep(min(0.05, remaining))
    finally:
        proc.wait()


def cmd_listen(args: argparse.Namespace) -> int:
    path = Path(args.record).resolve()
    doc = load(path)
    sb = section(doc, path)
    require_open(sb)
    if listener_state(sb, path)["state"] == "alive":
        raise UsageError("listener is already alive")
    try:
        if str(uuid.UUID(args.session)) != args.session:
            raise ValueError
    except ValueError:
        raise UsageError("--session must be a canonical UUID")
    if args.since is not None and args.since < 0:
        raise UsageError("--since must be nonnegative")
    if any(not topic.strip() or topic.startswith("control:") for topic in args.topic):
        raise UsageError("--topic must be nonempty and must not start with control:")
    if not math.isfinite(args.interval) or args.interval <= 0:
        raise UsageError("--interval must be positive and finite")
    repo = Path(args.repository).resolve() if args.repository else Path(subprocess.check_output(
        ["git", "rev-parse", "--show-toplevel"], text=True).strip()).resolve()
    reader = repo / "metrics/scripts/usage_ledger.py"
    adjacent = Path(__file__).resolve().parents[2] / "durable-state/scripts/codex_chatboard.py"
    chatboard = adjacent if adjacent.is_file() else repo / "skills/durable-state/scripts/codex_chatboard.py"
    reader_sha = hashlib.sha256(reader.read_bytes()).hexdigest()
    chatboard_sha = hashlib.sha256(chatboard.read_bytes()).hexdigest()
    upgrade(sb)
    objective_id = sb["objective_id"]
    stdout = path.parent / f"budget-listener-{objective_id}.jsonl"
    stderr = path.parent / f"budget-listener-{objective_id}.stderr"
    since, cursor = args.since, None
    incomplete = False
    recorded = [sb.get("listener"), *sb.get("listener_history", [])]
    owns_output = any(isinstance(entry, dict) and entry.get("stdout") == str(stdout) for entry in recorded)
    if owns_output and stdout.exists():
        data = stdout.read_bytes()
        complete = data.split(b"\n")[:-1]
        incomplete = bool(data and not data.endswith(b"\n"))
        for line in reversed(complete):
            try:
                last = json.loads(line)
            except (ValueError, UnicodeDecodeError):
                continue
            since, cursor = last["next_since"], last.get("next_budget_cursor")
            break
    if since is None:
        raise UsageError("--since is required without earlier listener output")
    if not isinstance(since, int) or isinstance(since, bool) or since < 0:
        raise UsageError("listener cursor next_since must be a nonnegative integer")
    argv = [sys.executable, str(chatboard), "--repository", str(repo), "wait", "--since", str(since),
            "--topic", f"session:{args.session}"]
    for topic in args.topic:
        argv += ["--topic", topic]
    argv += ["--exclude-session-id", args.session, "--budget-record", str(path), "--budget-reader", str(reader),
             "--budget-sha256", reader_sha, "--budget-interval", str(args.interval)]
    if cursor is not None:
        argv += ["--budget-cursor", cursor if isinstance(cursor, str) else json.dumps(cursor, separators=(",", ":"))]
    argv += ["--follow"]
    with stdout.open("ab") as out, stderr.open("ab") as err:
        if incomplete:
            out.write(b"\n")
            out.flush()
        proc = subprocess.Popen(argv, stdin=subprocess.DEVNULL, stdout=out, stderr=err, start_new_session=True)
    entry = {"pid": proc.pid, "started_at": stamp(now()), "argv": argv, "stdout": str(stdout), "stderr": str(stderr),
             "reader": str(reader), "reader_sha256": reader_sha, "chatboard": str(chatboard), "chatboard_sha256": chatboard_sha}
    try:
        try:
            exit_code = proc.wait(timeout=1.5)
        except subprocess.TimeoutExpired:
            exit_code = None
        if sb.get("listener"):
            sb.setdefault("listener_history", []).append(sb.pop("listener"))
        if exit_code is not None:
            entry["exit_code"] = exit_code
            sb.setdefault("listener_history", []).append(entry)
            save(path, doc)
            print(f"scope_budget: listener exited during startup; see {stderr}", file=sys.stderr)
            return 3
        sb["listener"] = entry
        save(path, doc)
    except BaseException:
        stop_listener_group(proc)
        raise
    print(json.dumps(entry))
    return 0


def cmd_check(args: argparse.Namespace) -> int:
    path = Path(args.record)
    sb = section(load(path), path)
    status = {"closed": "closed_at" in sb, "closed_at": sb.get("closed_at"), "listener": listener_state(sb, path)}
    if status["closed"]:
        print(json.dumps(status))
        return 5
    limits = sb["limits"]
    elapsed = (now() - parse(sb["started_at"])).total_seconds() / 3600
    meter = weekly_percent(Path(args.rollout)) if args.rollout else None
    start = (sb.get("weekly_at_start") or {}).get("used_percent")
    points = None if meter is None or start is None else meter["used_percent"] - start
    reached, unknown, advisory = [], [], []
    user_points = sb.get("limits_set_by") == "user" or any(
        h.get("set_by") == "user" and h.get("limits", {}).get("max_weekly_points") == limits.get("max_weekly_points")
        and "max_weekly_points" in h.get("limits", {}) for h in sb.get("limit_history", []))
    if "max_hours" in limits and elapsed >= limits["max_hours"]:
        reached.append("hours")
    for name, value in (("max_weekly_points", points), ("max_weekly_percent", meter and meter["used_percent"])):
        if name in limits:
            if name == "max_weekly_points" and not user_points:
                advisory.append(name)
                continue
            if value is None:
                unknown.append(name)
            elif value >= limits[name]:
                reached.append(name)
    n = len(sb["additions"])
    trigger = limits.get("reconsider_after_additions")
    reconsider = trigger is not None and n >= trigger
    result = {**status, "elapsed_hours": round(elapsed, 2), "additions": n, "original_items": len(sb["original_items"]),
              "weekly_percent": meter and meter["used_percent"], "account_weekly_points": points, "account_wide": True, "advisory": advisory,
              "limits": limits, "limits_set_by": setter(sb["limits_set_by"]), "reached": reached,
              "unknown": unknown, "reconsider": reconsider, "stop": bool(reached)}
    print(json.dumps(result))
    return 4 if reached else 3 if reconsider else 0


def parser() -> argparse.ArgumentParser:
    p = argparse.ArgumentParser(description="Original scope, additions and budget in an assignment record.")
    sub = p.add_subparsers(dest="command", required=True)

    def limit_args(q: argparse.ArgumentParser) -> None:
        q.add_argument("--max-hours", type=float)
        q.add_argument("--max-weekly-points", type=float, help="rise in the weekly meter since init")
        q.add_argument("--max-weekly-percent", type=float, help="ceiling on the weekly meter")
        q.add_argument("--reconsider-after-additions", type=int)
        q.add_argument("--set-by", required=True, choices=("user", "supervisor"))
        q.add_argument("--reason")

    i = sub.add_parser("init", help="record the original items and the budget")
    i.add_argument("--record", required=True)
    i.add_argument("--objective", required=True)
    i.add_argument("--item", action="append", required=True)
    i.add_argument("--rollout", help="Codex rollout, to read the weekly meter at the start")
    i.add_argument("--session")
    i.add_argument("--provider", choices=("codex", "claude"))
    limit_args(i)
    a = sub.add_parser("add", help="record one scope addition")
    a.add_argument("--record", required=True)
    a.add_argument("--item", required=True)
    a.add_argument("--reason", required=True)
    r = sub.add_parser("revise", help="change a limit, keeping the old one")
    r.add_argument("--record", required=True)
    limit_args(r)
    a2 = sub.add_parser("add-session")
    a2.add_argument("--record", required=True)
    a2.add_argument("--session", required=True)
    a2.add_argument("--provider", required=True, choices=("codex", "claude"))
    a2.add_argument("--from", dest="from_time", required=True)
    a2.add_argument("--bound-by", required=True, choices=("init", "rotation", "supervisor", "user"))
    a2.add_argument("--evidence")
    b = sub.add_parser("bind")
    b.add_argument("--record", required=True)
    b.add_argument("--session", required=True)
    b.add_argument("--provider", required=True, choices=("codex", "claude"))
    b.add_argument("--parent")
    b.add_argument("--bound-by", required=True, choices=("supervisor", "user"))
    b.add_argument("--reason", required=True)
    cl = sub.add_parser("cost-limit")
    cl.add_argument("--record", required=True)
    cl.add_argument("--pool", required=True, choices=("codex", "claude_all_models", "claude_fable"))
    cl.add_argument("--node")
    cl.add_argument("--max-usd", type=float)
    cl.add_argument("--mark-usd", type=float, action="append", default=[])
    cl.add_argument("--set-by", required=True, choices=("user", "supervisor"))
    cl.add_argument("--reason", required=True)
    close = sub.add_parser("close", help="close the objective")
    close.add_argument("--record", required=True)
    close.add_argument("--disposition", required=True, choices=("accepted", "abandoned"))
    close.add_argument("--reason", required=True)
    close.add_argument("--set-by", required=True, choices=("user", "supervisor"))
    close.add_argument("--at")
    listen = sub.add_parser("listen", help="start the detached budget listener")
    listen.add_argument("--record", required=True)
    listen.add_argument("--session", required=True)
    listen.add_argument("--since", type=int)
    listen.add_argument("--topic", action="append", default=[])
    listen.add_argument("--interval", type=float, default=60)
    listen.add_argument("--repository")
    c = sub.add_parser("check", help="one JSON line: elapsed, additions, meter, reconsider, stop")
    c.add_argument("--record", required=True)
    c.add_argument("--rollout")
    return p


def main(argv: list[str] | None = None) -> int:
    args = parser().parse_args(argv)
    try:
        commands = {"init": cmd_init, "add": cmd_add, "revise": cmd_revise, "check": cmd_check,
                    "add-session": cmd_add_session, "bind": cmd_bind, "cost-limit": cmd_cost_limit,
                    "close": cmd_close, "listen": cmd_listen}
        if args.command == "check":
            return commands[args.command](args)
        lock = Path(str(Path(args.record).resolve()) + ".lock")
        with lock.open("a") as f:
            fcntl.flock(f, fcntl.LOCK_EX)
            return commands[args.command](args)
    except UsageError as exc:
        print(f"scope_budget: {exc}", file=sys.stderr)
        return 2
    except (OSError, ValueError, KeyError, subprocess.SubprocessError) as exc:
        print(f"scope_budget: {type(exc).__name__}: {exc}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    sys.exit(main())
