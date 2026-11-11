#!/usr/bin/env python3
"""Keep a supervisor's original scope, additions and budget in its assignment record.

The record is a JSON object file; this script owns only its `scope_budget` key,
so it can live in the existing assignment record. `init` writes the original
items, a dollar limit and any other limits, `add` appends one scope addition
with its reason, `revise` changes a limit (keeping the old one as history),
`cost-limit` sets further dollar limits, and `check` prints one JSON line:
additions, spend against the dollar limits and model-busy time from the usage
ledger, elapsed hours, the weekly quota meter when a Codex rollout is given,
and whether to reconsider or stop.

A budget limits dollars and scope additions, not time. Elapsed and model-busy
hours are reported as information: a wall-clock limit kept running while work
waited for a person, and stopping then repeated setup without saving spend.

Exit codes for `check`: 0 within budget, 3 reconsider (additions reached their
trigger), 4 stop (a user dollar limit or the account-wide ceiling reached), 5
closed. Other commands return 0. All return 2 for a usage error.
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
METER_MAX_AGE_SECONDS = 300
COST_BASIS = 'speed_adjusted_api_usd'
LIMITS = ("max_weekly_points", "max_weekly_percent", "reconsider_after_additions")
POOLS = ("codex", "claude_all_models", "claude_fable", "pi_api")
LEDGER = Path("metrics/scripts/usage_ledger.py")
SPEND_KINDS = ("mark_crossed", "node_mark_crossed", "limit_fraction", "limit_reached")


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
                if limits.get('limit_id') not in (None, 'codex'):
                    continue
                for slot in ("primary", "secondary"):
                    w = limits.get(slot)
                    if isinstance(w, dict) and w.get("window_minutes") == WEEK_MINUTES and w.get("used_percent") is not None:
                        return {"used_percent": float(w["used_percent"]), "observed_at": rec.get("timestamp"),
                                "resets_at": w.get("resets_at"), "window_minutes": WEEK_MINUTES,
                                "limit_id": limits.get("limit_id"), "plan_type": limits.get("plan_type"),
                                "source": str(rollout)}
    return None


def account_quota(meter: dict[str, Any] | None, ceiling: float | None) -> dict[str, Any]:
    """Current headroom is unknown when the last recorded reading is unusable."""
    result = {"status": "not_established", "pool": "codex", "account_wide": True, "used_percent": None,
              "remaining_percent": None, "headroom_to_ceiling_percent": None,
              "ceiling_percent": ceiling, "observed_at": None, "resets_at": None,
              "age_seconds": None, "max_age_seconds": METER_MAX_AGE_SECONDS, "reason": "no_reading"}
    if meter is None:
        return result
    result.update(meter)
    try:
        value = meter["used_percent"]
        if isinstance(value, bool) or not math.isfinite(value) or not 0 <= value <= 100:
            raise ValueError("invalid percent")
        age = (now() - parse(meter["observed_at"])).total_seconds()
        reset = meter.get("resets_at")
        known_reset = isinstance(reset, (int, float)) and not isinstance(reset, bool) and math.isfinite(reset) and reset >= 0
        status, reason = "observed", None
        if age < 0:
            status, reason = "not_established", "future_reading"
        elif known_reset and reset <= now().timestamp():
            status, reason = "expired", "window_reset"
        elif not known_reset:
            status, reason = "not_established", "reset_unknown"
        elif age > METER_MAX_AGE_SECONDS:
            status, reason = "stale", "reading_age"
        result.update(status=status, reason=reason, age_seconds=age, last_reported_remaining_percent=100 - value)
        if status == "observed":
            result.update(remaining_percent=100 - value,
                          headroom_to_ceiling_percent=ceiling - value if ceiling is not None else None)
    except (TypeError, ValueError, KeyError, OverflowError):
        result.update(status="not_established", reason="invalid_reading")
    return result


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
    if args.max_hours is not None:
        raise UsageError("--max-hours is no longer accepted: a budget limits dollars and additions, "
                         "and check reports elapsed and model-busy time; set --max-usd or use cost-limit")


def finite_usd(values: list[float]) -> None:
    if any(not math.isfinite(x) or x < 0 for x in values):
        raise UsageError("give nonnegative finite --max-usd or --mark-usd")


def setter(value: str) -> str:
    return "supervisor" if value == "coordinator" else value

def limits_from(args: argparse.Namespace) -> dict[str, Any]:
    limits = {k: getattr(args, k) for k in LIMITS if getattr(args, k) is not None}
    if any(not math.isfinite(v) or v < 0 for v in limits.values()):
        raise UsageError("limits must be nonnegative and finite")
    if limits.get('max_weekly_percent', 0) > 100:
        raise UsageError("--max-weekly-percent must be between 0 and 100")
    return limits


def cmd_init(args: argparse.Namespace) -> int:
    path = Path(args.record)
    doc = load(path)
    if KEY in doc:
        raise UsageError(f"{path} already has a {KEY} record; use revise or add")
    reject_points(args)
    if bool(args.session) != bool(args.provider):
        raise UsageError("--session and --provider must be supplied together")
    if args.session_from and not args.session:
        raise UsageError("--session-from needs --session")
    limits = limits_from(args)
    finite_usd([args.max_usd, *args.mark_usd])
    started = now()
    session_from = stamp(started)
    if args.session_from:
        if parse(args.session_from) > started:
            raise UsageError("--session-from must not be after the record's start")
        session_from = stamp(parse(args.session_from))
    meter = weekly_percent(Path(args.rollout)) if args.rollout else None
    cost = {"pool": args.pool, "node": None, "max_usd": args.max_usd, "marks_usd": sorted(set(args.mark_usd)),
            "set_by": args.set_by, "set_at": stamp(started), "reason": args.reason, "basis": COST_BASIS}
    doc[KEY] = {"schema": SCHEMA, "objective_id": str(uuid.uuid4()),
                "sessions": ([{"session_id": args.session, "provider": args.provider, "from": session_from, "bound_by": "init", "evidence": args.session_from_evidence}] if args.session else []),
                "bindings": [], "cost_limits": [cost], "objective": args.objective, "original_items": args.item,
                "started_at": stamp(started), "weekly_at_start": meter,
                "limits": limits, "limits_set_by": args.set_by, "limits_reason": args.reason,
                "limit_history": [{"cost_limit": cost, "previous_cost_limit": None}], "additions": []}
    save(path, doc)
    print(json.dumps({"record": str(path), "started_at": stamp(started), "limits": limits, "cost_limit": cost,
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
    if not values:
        raise UsageError("give nonnegative finite --max-usd or --mark-usd")
    finite_usd(values)
    path = Path(args.record)
    doc = load(path)
    sb = section(doc, path)
    require_open(sb)
    upgrade(sb)
    entry = {"pool": args.pool, "node": args.node, "max_usd": args.max_usd,
             "marks_usd": sorted(set(args.mark_usd)), "set_by": args.set_by,
             "set_at": stamp(now()), "reason": args.reason, "basis": COST_BASIS}
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


def ledger_path(repository: str | None) -> Path | None:
    """The usage ledger beside this script (repository or manual-host bundle), else the given or current repository's."""
    if repository:
        candidates = [Path(repository) / LEDGER]
    else:
        candidates = [Path(__file__).resolve().parents[3] / LEDGER]
        try:
            top = subprocess.run(["git", "rev-parse", "--show-toplevel"], capture_output=True, text=True, timeout=10)
            if top.returncode == 0 and top.stdout.strip():
                candidates.append(Path(top.stdout.strip()) / LEDGER)
        except (OSError, subprocess.SubprocessError):
            pass
    return next((p.resolve() for p in candidates if p.is_file()), None)


def ledger_control(reader: Path, record: Path) -> dict[str, Any]:
    """One cursorless `control` read: every currently crossed mark or limit is an event."""
    result = subprocess.run([sys.executable, "-I", str(reader), "control", "--record", str(record.resolve())],
                            capture_output=True, text=True, timeout=300)
    if result.returncode:
        raise ValueError(f"usage ledger exited {result.returncode}: {result.stderr.strip()[-300:]}")
    doc = json.loads(result.stdout)
    if not isinstance(doc, dict) or doc.get("schema") != "objective-budget-control-v1" \
            or not isinstance(doc.get("snapshot"), dict) or not isinstance(doc.get("events"), list):
        raise ValueError("incompatible usage ledger output")
    return doc


def spend(sb: dict[str, Any], record: Path, repository: str | None) -> tuple[dict[str, Any], list[str], list[str]]:
    """Spend view plus the user dollar limits reached and those whose state is unknown.

    A user limit is the user's boundary, so reaching it stops; a limit the
    supervisor set is a mark for its next decision. A limit is unknown when the
    ledger is unavailable, the spend has unpriced calls, or coverage is partial.
    """
    user = [c for c in sb.get("cost_limits", []) if setter(c.get("set_by", "")) == "user" and c.get("max_usd") is not None]
    name = lambda c: "max_usd:" + c["pool"] + (":" + c["node"] if c.get("node") else "")
    reader = ledger_path(repository)
    try:
        if reader is None:
            raise ValueError("usage ledger not found; pass --repository")
        reader_sha = hashlib.sha256(reader.read_bytes()).hexdigest()
        control = ledger_control(reader, record)
    except (OSError, ValueError, subprocess.SubprocessError) as exc:
        return {"status": "unavailable", "reason": str(exc), "reader": reader and str(reader)}, [], [name(c) for c in user]
    snap = control["snapshot"]
    crossed = [{k: e.get(k) for k in ("kind", "pool", "node_id", "level", "set_by")}
               for e in control["events"] if isinstance(e, dict) and e.get("kind") in SPEND_KINDS]
    controls = snap.get("cost_controls") or []
    reached, unknown = [], []
    for c in user:
        node = c.get("node")
        if any(e["kind"] == "limit_reached" and e["pool"] == c["pool"] and e["node_id"] == node and e["set_by"] == "user"
               for e in crossed):
            reached.append(name(c))
            continue
        row = next((x for x in controls if x.get("pool") == c["pool"] and x.get("node_id") == node
                    and x.get("set_by") == "user"), None)
        if row is None or row.get("remaining_usd") is None:
            unknown.append(name(c))
    view = {"status": snap.get("status"), "snapshot_at": snap.get("snapshot_at"),
            "reader": {"path": str(reader), "sha256": reader_sha},
            "cost_controls": controls, "crossed": crossed, "time": snap.get("time")}
    return view, reached, unknown


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
    account = account_quota(meter, limits.get('max_weekly_percent'))
    start = (sb.get("weekly_at_start") or {}).get("used_percent")
    points = None if meter is None or start is None else meter["used_percent"] - start
    reached, unknown, advisory = [], [], []
    user_points = sb.get("limits_set_by") == "user" or any(
        h.get("set_by") == "user" and h.get("limits", {}).get("max_weekly_points") == limits.get("max_weekly_points")
        and "max_weekly_points" in h.get("limits", {}) for h in sb.get("limit_history", []))
    view, spend_reached, spend_unknown = spend(sb, path, args.repository)
    reached += spend_reached
    unknown += spend_unknown
    percent = account['used_percent'] if account['status'] in ('observed', 'stale') else None
    for name, value in (("max_weekly_points", points), ("max_weekly_percent", percent)):
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
              "unknown": unknown, "reconsider": reconsider, "stop": bool(reached),
              "ignored_limits": [k for k in ("max_hours",) if k in limits],
              "account_quota": account, "spend": view, "time": view.get("time"),
              "cost_basis": COST_BASIS, "cost_limits": sb.get("cost_limits", []),
              "cost_label": "API-equivalent dollars adjusted for subscription speed (proxy)",
              "estimated_objective_quota_points": {"status": "not_established", "low": None, "high": None,
                                                    "calibration_version": None}}
    print(json.dumps(result))
    return 4 if reached else 3 if reconsider else 0


def parser() -> argparse.ArgumentParser:
    p = argparse.ArgumentParser(description="Original scope, additions and budget in an assignment record.")
    sub = p.add_subparsers(dest="command", required=True)

    def limit_args(q: argparse.ArgumentParser) -> None:
        q.add_argument("--max-hours", type=float, help=argparse.SUPPRESS)  # retired: refused with a reason
        q.add_argument("--max-weekly-points", type=float, help="rise in the weekly meter since init")
        q.add_argument("--max-weekly-percent", type=float, help="account-wide used-percent ceiling, independent of objective dollars")
        q.add_argument("--reconsider-after-additions", type=int)
        q.add_argument("--set-by", required=True, choices=("user", "supervisor"))
        q.add_argument("--reason")

    i = sub.add_parser("init", help="record the original items and the budget")
    i.add_argument("--record", required=True)
    i.add_argument("--objective", required=True)
    i.add_argument("--item", action="append", required=True)
    i.add_argument("--rollout", help="Codex rollout, to read the weekly meter at the start")
    i.add_argument("--session")
    i.add_argument("--session-from", help="the session's first transcript time, when orientation began before init; "
                                        "the ledger counts from the earliest session start")
    i.add_argument("--session-from-evidence")
    i.add_argument("--provider", choices=("codex", "claude"))
    i.add_argument("--pool", required=True, choices=POOLS, help="pool of the objective's dollar limit")
    i.add_argument("--max-usd", type=float, required=True,
                   help="objective dollar limit (API-equivalent, speed-adjusted); cost-limit adds pools and nodes")
    i.add_argument("--mark-usd", type=float, action="append", default=[])
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
    cl = sub.add_parser("cost-limit", help="limits in API-equivalent dollars adjusted for subscription speed; not quota points")
    cl.add_argument("--record", required=True)
    cl.add_argument("--pool", required=True, choices=POOLS)
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
    c = sub.add_parser("check", help="one JSON line: additions, spend, time, meter, reconsider, stop")
    c.add_argument("--record", required=True)
    c.add_argument("--rollout")
    c.add_argument("--repository", help="repository whose usage ledger to read, if none sits beside this script")
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
