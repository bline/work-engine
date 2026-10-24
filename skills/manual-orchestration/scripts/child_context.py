#!/usr/bin/env python3
"""Read a session's context pressure and frame the retained-versus-fresh choice.

Usage:
  python3 -I child_context.py --binary BIN --sha256 DIGEST check \
      --session ID --rollout PATH [--next-task-tokens N] [--out FILE]
  python3 -I child_context.py --binary BIN --sha256 DIGEST tree \
      --parent-thread ID [--recursive] [--sessions-dir DIR] [--out FILE]
  python3 -I child_context.py claude --transcript PATH \
      [--next-task-tokens N] [--out FILE]

`check` reads one session through the pinned work-engine-context-status binary
and compares its latest input with two levels on the same window:

  trigger     where a chosen boundary should prefer a fresh context
              (default 150,000 of 258,400, the brief's planning level)
  band floor  the lowest pre-compaction reading observed
              (default 171,267 of 258,400: 42 compactions, 2026-10-21)

Both scale with the session's own window. `claude` reads a Claude Code
subagent transcript (~/.claude/projects/<project>/<session>/subagents/
agent-<id>.jsonl) instead, where context is input + cache-creation +
cache-read tokens of the latest assistant call and no window is recorded, so
it uses absolute levels: trigger 150,000 (cost-per-call basis, not a
compaction level) and band floor 202,533 (lowest of 20 Claude Code
auto-compactions on this machine, mixed windows, 2026-10-21). The output is a record to retain
and a recommendation, not a decision: the parent or worker records the
decision and its reason. Missing usage is reported as unavailable, never as
zero. `tree` lists a parent's children from recorded metadata and checks each.
"""
from __future__ import annotations

import argparse
import datetime
import hashlib
import json
import subprocess
import sys
from pathlib import Path

REFERENCE_WINDOW = 258_400
MAX_COUNTER = (1 << 64) - 1
CLAUDE_TRIGGER_TOKENS = 150_000
CLAUDE_BAND_FLOOR_TOKENS = 202_533
DEFAULT_TRIGGER_FRACTION = 150_000 / REFERENCE_WINDOW
DEFAULT_BAND_FLOOR_FRACTION = 171_267 / REFERENCE_WINDOW


class UsageError(Exception):
    pass


def verify_binary(path: str, digest: str) -> Path:
    binary = Path(path)
    if not binary.is_file():
        raise UsageError(f"binary not found: {binary}")
    actual = hashlib.sha256(binary.read_bytes()).hexdigest()
    if actual != digest.lower():
        raise UsageError(f"binary digest mismatch: expected {digest}, found {actual}")
    return binary


def run_json(binary: Path, args: list[str]) -> dict:
    proc = subprocess.run([str(binary), *args], capture_output=True, text=True)
    if proc.returncode != 0:
        return {"_error": (proc.stderr or proc.stdout).strip()[:2000], "_exit": proc.returncode}
    try:
        return json.loads(proc.stdout)
    except ValueError as error:
        return {"_error": f"unparseable output: {error}", "_exit": proc.returncode}


IDENTITY_MARKERS = ("mismatch", "not found", "multiple", "candidates", "identity")


def classify_failure(detail: str) -> str:
    """Separate a rejected or unresolvable identity from other tool failures."""
    lowered = detail.lower()
    if any(marker in lowered for marker in IDENTITY_MARKERS):
        return "identity_rejection"
    return "launch_or_tool_failure"


def assess(
    report: dict,
    *,
    next_task_tokens: int | None,
    trigger_fraction: float,
    band_floor_fraction: float,
    trigger_tokens: int | None = None,
    band_floor_tokens: int | None = None,
) -> dict:
    """Turn one context-status report into a retained-versus-fresh record."""
    now = datetime.datetime.now(datetime.timezone.utc).isoformat()
    record = {
        "schema": "manual-orchestration-context-check-v1",
        "checked_at": now,
        "session_id": report.get("session_id"),
        "rollout": report.get("rollout"),
        "next_task_tokens": next_task_tokens,
        "next_task_basis": "parent estimate" if next_task_tokens is not None else "not estimated",
    }
    if "_error" in report:
        record.update(
            status="usage_unavailable",
            unavailable_class=classify_failure(report["_error"]),
            detail=report["_error"],
            recommendation="usage_unavailable",
            reasons=["context-status failed; do not substitute parent counters or assume zero"],
        )
        return record

    latest = report.get("latest_request") or {}
    window = report.get("context_window_tokens")
    current = latest.get("input_tokens")
    lineage = report.get("lineage") or []
    record.update(
        observed_at=report.get("observed_at"),
        freshness=report.get("freshness"),
        recorded_model=report.get("recorded_model"),
        recorded_effort=report.get("recorded_effort"),
        input_tokens=current,
        context_window_tokens=window,
        epoch=latest.get("epoch"),
        compaction_markers=sum(1 for item in lineage if "compaction" in str(item)),
        lineage=lineage,
        limitations=report.get("limitations") or [],
    )
    absolute = trigger_tokens is not None and band_floor_tokens is not None
    window_ok = type(window) is int and 0 < window <= MAX_COUNTER
    gap = report.get("last_record_gap_line")
    line = latest.get("line")
    historical = report.get("latest_request_status") == "historical_observation" or (
        type(gap) is int and (type(line) is not int or line <= gap)
    )
    if (type(current) is not int or not 0 < current <= MAX_COUNTER
            or historical or not (window_ok or absolute)):
        record["latest_request_status"] = report.get("latest_request_status")
        record["historical_input_tokens"] = current if historical else None
        record["input_tokens"] = None
        record.update(
            status="usage_unavailable",
            unavailable_class="historical_observation" if historical else "absent_counters",
            recommendation="usage_unavailable",
            reasons=["no valid latest input or window; unavailable is not zero"],
        )
        return record

    if absolute:
        trigger, band_floor = trigger_tokens, band_floor_tokens
    else:
        trigger = round(window * trigger_fraction)
        band_floor = round(window * band_floor_fraction)
    record.update(
        status="observed",
        trigger_tokens=trigger,
        band_floor_tokens=band_floor,
        headroom_to_trigger=trigger - current,
        headroom_to_band_floor=band_floor - current,
        fraction_of_window=round(current / window, 4) if window_ok else None,
    )

    reasons = []
    if current >= band_floor:
        reasons.append(f"input {current} is inside the observed compaction band (floor {band_floor})")
    elif current >= trigger:
        reasons.append(f"input {current} is at or above the trigger {trigger}")
    if next_task_tokens is not None and current + next_task_tokens >= band_floor:
        reasons.append(
            f"input plus estimated next task ({current} + {next_task_tokens}) reaches the band floor {band_floor}"
        )
    if reasons:
        record["recommendation"] = "fresh"
    else:
        record["recommendation"] = "retain_ok"
        reasons.append(f"input {current} is below the trigger {trigger}")
        if next_task_tokens is None:
            reasons.append("next-task size not estimated; headroom to band floor is the margin")
    if record["compaction_markers"]:
        reasons.append(
            f"session has compacted {record['compaction_markers']} time(s): the low reading is real, but "
            "earlier pressure is not erased and retained context holds only what its continuation kept"
        )
    record["reasons"] = reasons
    return record


def claude_report(path: str) -> dict:
    """Shape a Claude Code transcript like a context-status report."""
    transcript = Path(path)
    if not transcript.is_file():
        return {"_error": f"transcript not found: {transcript}", "_exit": 2}
    latest, lineage, agent_id, model, observed = None, [], None, None, None
    gap, latest_line = None, None
    seen = set()
    with transcript.open(encoding="utf-8", errors="replace") as handle:
        for number, line in enumerate(handle, 1):
            try:
                record = json.loads(line)
            except ValueError:
                gap = number
                if not line.endswith("\n"):
                    break  # never read an in-progress trailing record
                continue
            if not isinstance(record, dict):
                continue
            agent_id = agent_id or record.get("agentId") or record.get("sessionId")
            if record.get("compactMetadata") or record.get("subtype") == "compact_boundary":
                lineage.append(f"compaction marker at line {number}")
                latest, latest_line = None, None
                continue
            message = record.get("message")
            usage = message.get("usage") if isinstance(message, dict) else None
            if record.get("type") != "assistant":
                continue
            # Same usable-request contract as the Rust Claude reader: all three
            # input counters, nonnegative u64 values, positive checked sum, and
            # no locally generated synthetic message. Shared fixtures exercise both.
            if not isinstance(message, dict) or message.get("model") == "<synthetic>":
                gap = number
                continue
            key = message.get("id")
            if not isinstance(key, str):
                key = None
            if key is not None and key in seen:
                continue
            if key is not None:
                seen.add(key)
            counters = [usage.get(k) for k in (
                "input_tokens", "cache_creation_input_tokens", "cache_read_input_tokens"
            )] if isinstance(usage, dict) else []
            if (len(counters) != 3 or any(type(n) is not int or not 0 <= n <= MAX_COUNTER for n in counters)
                    or not 0 < sum(counters) <= MAX_COUNTER):
                gap = number
                continue
            latest, latest_line = sum(counters), number
            model, observed = message.get("model"), record.get("timestamp")
    return {
        "session_id": agent_id,
        "rollout": str(transcript),
        "observed_at": observed,
        "recorded_model": model,
        "latest_request": {"input_tokens": latest, "epoch": len(lineage), "line": latest_line} if latest is not None else {},
        "latest_request_status": ("unavailable" if latest is None else
                                  "historical_observation" if gap is not None and latest_line <= gap else
                                  "latest_valid_observation_with_record_gaps" if gap is not None else
                                  "latest_valid_observation"),
        "last_record_gap_line": gap,
        "context_window_tokens": None,
        "lineage": lineage,
        "limitations": ["Claude Code transcripts record no context window; levels are absolute"],
    }


def check(binary: Path, args: argparse.Namespace) -> dict:
    command = ["--session", args.session, "--json"]
    if args.rollout:
        command += ["--rollout", args.rollout]
    if args.sessions_dir:
        command += ["--sessions-dir", args.sessions_dir]
    return assess(
        run_json(binary, command),
        next_task_tokens=args.next_task_tokens,
        trigger_fraction=args.trigger_fraction,
        band_floor_fraction=args.band_floor_fraction,
    )


def tree(binary: Path, args: argparse.Namespace) -> dict:
    command = ["agents", "--parent-thread", args.parent_thread, "--json"]
    if args.recursive:
        command.append("--recursive")
    if args.sessions_dir:
        command += ["--sessions-dir", args.sessions_dir]
    listing = run_json(binary, command)
    if "_error" in listing:
        return {"schema": "manual-orchestration-context-tree-v1", "status": "unavailable", "detail": listing["_error"]}
    children = []
    for agent in listing.get("agents", []):
        report = run_json(binary, ["--session", agent["thread_id"], "--rollout", agent["rollout"], "--json"])
        record = assess(
            report,
            next_task_tokens=None,
            trigger_fraction=args.trigger_fraction,
            band_floor_fraction=args.band_floor_fraction,
        )
        record["agent_path"] = agent.get("agent_path")
        children.append(record)
    return {
        "schema": "manual-orchestration-context-tree-v1",
        "parent_thread_id": args.parent_thread,
        "completeness": listing.get("completeness"),
        "skipped_metadata": listing.get("skipped_metadata"),
        "children": children,
    }


def parser() -> argparse.ArgumentParser:
    p = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    p.add_argument("--binary", help="pinned work-engine-context-status path (check, tree)")
    p.add_argument("--sha256", help="expected binary SHA-256 (check, tree)")
    p.add_argument("--trigger-fraction", type=float, default=DEFAULT_TRIGGER_FRACTION)
    p.add_argument("--band-floor-fraction", type=float, default=DEFAULT_BAND_FLOOR_FRACTION)
    p.add_argument("--out", help="also write the JSON record here")
    sub = p.add_subparsers(dest="command", required=True)
    c = sub.add_parser("check", help="assess one session")
    c.add_argument("--session", required=True)
    c.add_argument("--rollout")
    c.add_argument("--sessions-dir")
    c.add_argument("--next-task-tokens", type=int, help="parent's estimate of the next task's growth")
    c.add_argument("--out", dest="sub_out", help="also write the JSON record here")
    t = sub.add_parser("tree", help="assess every child of a parent thread")
    t.add_argument("--parent-thread", required=True)
    t.add_argument("--recursive", action="store_true")
    t.add_argument("--sessions-dir")
    t.add_argument("--out", dest="sub_out", help="also write the JSON record here")
    a = sub.add_parser("claude", help="assess a Claude Code subagent transcript")
    a.add_argument("--transcript", required=True)
    a.add_argument("--next-task-tokens", type=int, help="parent's estimate of the next task's growth")
    a.add_argument("--trigger-tokens", type=int, default=CLAUDE_TRIGGER_TOKENS)
    a.add_argument("--band-floor-tokens", type=int, default=CLAUDE_BAND_FLOOR_TOKENS)
    a.add_argument("--out", dest="sub_out", help="also write the JSON record here")
    return p


def main(argv: list[str] | None = None) -> int:
    args = parser().parse_args(argv)
    if args.command == "claude":
        result = assess(
            claude_report(args.transcript),
            next_task_tokens=args.next_task_tokens,
            trigger_fraction=args.trigger_fraction,
            band_floor_fraction=args.band_floor_fraction,
            trigger_tokens=args.trigger_tokens,
            band_floor_tokens=args.band_floor_tokens,
        )
        result["host"] = "claude-code"
    else:
        try:
            if not args.binary or not args.sha256:
                raise UsageError("--binary and --sha256 are required for check and tree")
            binary = verify_binary(args.binary, args.sha256)
        except UsageError as error:
            print(json.dumps({"status": "usage_unavailable", "unavailable_class": "launch_or_tool_failure", "detail": str(error)}))
            return 2
        result = check(binary, args) if args.command == "check" else tree(binary, args)
    text = json.dumps(result, indent=2)
    out = args.sub_out or args.out
    if out:
        Path(out).write_text(text + "\n")
    print(text)
    return 0


if __name__ == "__main__":
    sys.exit(main())
