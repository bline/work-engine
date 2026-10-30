#!/usr/bin/env python3
"""Keep a coordinator's original scope, additions and budget in its assignment record.

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
import datetime as dt
import json
import os
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
    tmp = path.with_name(path.name + ".tmp")
    tmp.write_text(json.dumps(doc, indent=2, ensure_ascii=False) + "\n")
    os.replace(tmp, path)


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

def cmd_check(args: argparse.Namespace) -> int:
    path = Path(args.record)
    sb = section(load(path), path)
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
    result = {"elapsed_hours": round(elapsed, 2), "additions": n, "original_items": len(sb["original_items"]),
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
    c = sub.add_parser("check", help="one JSON line: elapsed, additions, meter, reconsider, stop")
    c.add_argument("--record", required=True)
    c.add_argument("--rollout")
    return p


def main(argv: list[str] | None = None) -> int:
    args = parser().parse_args(argv)
    try:
        return {"init": cmd_init, "add": cmd_add, "revise": cmd_revise, "check": cmd_check, "add-session": cmd_add_session, "bind": cmd_bind, "cost-limit": cmd_cost_limit}[args.command](args)
    except UsageError as exc:
        print(f"scope_budget: {exc}", file=sys.stderr)
        return 2
    except (OSError, ValueError) as exc:
        print(f"scope_budget: {type(exc).__name__}: {exc}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    sys.exit(main())
