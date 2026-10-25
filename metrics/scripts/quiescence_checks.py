#!/usr/bin/env python3
"""Independent quiescence checks for a root rotation. No model, no network.

Neither host CLI can supply a parent-scoped child count. `claude agents --json`
lists machine-wide sessions including the root itself (status busy), so using it
would report live children on every rotation forever; `codex agents` has no
machine-readable output at all. What IS externally visible and parent-scoped is
the coordination board's advisory claims, which doctrine already requires a root
to release before transfer. These checks read those.

A board-claim check establishes "no unexpired claim is held", NOT "no child is
running": a child that never registered, released early, or let its claim lapse
is invisible to it. So `children` is claim-backed and conditional on complete
registration. `descendants` is a SECOND, independent check with a different
failure mode -- it reads the OS process tree and needs no registration at all.
A child must evade both (never hold a claim AND not be a descendant of the root
process) to pass unseen. What still evades both is stated in the probe record:
a detached child (setsid/nohup) or a turn queued into a session hosted
elsewhere. Neither check is a completeness basis on its own.

Contract with the coordinator:
  claims      -> one held resource per line, empty output = none held. Exit 0
                 only when the board was actually read; any failure exits
                 non-zero so empty output can never be mistaken for a clear
                 result.
  children    -> a single integer on stdout, or non-zero exit and no integer.
  descendants -> same integer contract, read from the process tree. The count
                 is an UPPER BOUND on child sessions (one child can show as
                 several processes); the gate is `== 0`, so over-counting only
                 blocks a rotation, never permits one. Exits 32 if the root
                 cannot be located unambiguously, because "I could not find the
                 root" is not "the root has no children".
"""
import argparse, datetime, json, re, subprocess, sys


def board_claims(board_cmd, repo):
    """-> dict of unexpired claims keyed by resource. Raises on any failure."""
    out = subprocess.run(
        ["python3", board_cmd, "--repository", repo, "read", "--active-claims", "--limit", "1"],
        capture_output=True, text=True, timeout=60)
    if out.returncode != 0:
        raise RuntimeError(f"board read failed (exit {out.returncode}): {out.stderr.strip()[:200]}")
    doc = json.loads(out.stdout)
    if "claims" not in doc:
        # The claims live under their own key; the message list is not claims.
        raise RuntimeError("board response has no 'claims' key")
    claims = doc["claims"]
    if not isinstance(claims, dict):
        raise RuntimeError(f"'claims' is {type(claims).__name__}, expected dict")
    # The board filters expired claims itself (verified 2026-10-24); re-filter so
    # a change in that behaviour cannot silently widen what counts as held.
    now = datetime.datetime.now(datetime.UTC)
    live = {}
    for res, c in claims.items():
        exp = c.get("expires_at")
        if exp:
            try:
                if datetime.datetime.fromisoformat(exp.replace("Z", "+00:00")) <= now:
                    continue
            except ValueError:
                raise RuntimeError(f"unparseable expires_at on {res!r}: {exp!r}")
        live[res] = c
    return live


# Counting is deliberately an UPPER BOUND, not an exact session count. A Codex
# CLI launch is two processes (`node /usr/bin/codex` plus the vendored native
# binary it execs), and bash briefly forks a subshell carrying its parent's argv
# before exec, so one child session can show as two or three processes.
#
# An earlier version collapsed a matching parent/child pair sharing a CLI name
# into "one session". That was FAIL-OPEN: a claude root that spawned a claude
# child directly is exactly that shape, so a live child was reported as zero.
# The gate is a zero test, so over-counting only ever blocks a rotation that
# could have proceeded -- recoverable -- while under-counting rotates away from
# a live child. Count every matching descendant and let the gate be `== 0`.
PROC_PATTERN = r"(^|/|\s)(claude|codex)(\s|$)"


def _process_table():
    out = subprocess.run(["ps", "-eo", "pid=,ppid=,args="],
                         capture_output=True, text=True, timeout=60)
    if out.returncode != 0:
        raise RuntimeError(f"ps failed (exit {out.returncode})")
    procs = {}
    for line in out.stdout.splitlines():
        parts = line.strip().split(None, 2)
        if len(parts) < 3:
            continue
        try:
            pid, ppid = int(parts[0]), int(parts[1])
        except ValueError:
            continue
        procs[pid] = {"ppid": ppid, "args": parts[2]}
    return procs


def descendant_session_procs(pane_pid, pattern):
    """-> int matching processes beneath the root session running in pane_pid.

    An upper bound on live child sessions. Raises if the root session cannot be
    located unambiguously: an unlocatable root is 'not established', never
    'quiescent'.
    """
    procs = _process_table()
    if pane_pid not in procs:
        raise RuntimeError(f"pane pid {pane_pid} is not in the process table")
    kids = {}
    for pid, p in procs.items():
        kids.setdefault(p["ppid"], []).append(pid)

    depth, stack = {pane_pid: 0}, [pane_pid]
    while stack:
        cur = stack.pop()
        for k in kids.get(cur, ()):
            if k not in depth:
                depth[k] = depth[cur] + 1
                stack.append(k)

    rx = re.compile(pattern)
    matching = [pid for pid in depth if rx.search(procs[pid]["args"])]
    if not matching:
        raise RuntimeError(f"no {pattern!r} process under pane pid {pane_pid}")
    shallowest = min(depth[pid] for pid in matching)
    roots = [pid for pid in matching if depth[pid] == shallowest]
    if len(roots) != 1:
        raise RuntimeError(
            f"root session ambiguous: {len(roots)} candidates at depth "
            f"{shallowest} ({[procs[p]['args'][:40] for p in roots]})")
    root = roots[0]
    return sum(1 for pid in matching
               if pid != root and depth[pid] > depth[root])


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("mode", choices=["claims", "children", "descendants"])
    ap.add_argument("--board-cmd", help="required for claims/children")
    ap.add_argument("--repo", help="required for claims/children")
    ap.add_argument("--session", help="claims/children: the predecessor session id")
    ap.add_argument("--pane-pid", type=int,
                    help="descendants: pid of the shell in the predecessor's pane")
    ap.add_argument("--proc-pattern", default=PROC_PATTERN,
                    help="descendants: argv regex identifying a session process")
    ap.add_argument("--resource-prefix", default="",
                    help="children: only count claims on resources with this prefix, "
                         "so unrelated roles are not counted as this root's children")
    a = ap.parse_args()

    if a.mode == "descendants":
        if a.pane_pid is None:
            print("descendants mode needs --pane-pid", file=sys.stderr)
            return 32
        try:
            print(descendant_session_procs(a.pane_pid, a.proc_pattern))
        except Exception as e:
            print(f"descendant check could not read the process tree: {e}", file=sys.stderr)
            return 32
        return 0

    for need in ("board_cmd", "repo", "session"):
        if not getattr(a, need):
            print(f"{a.mode} mode needs --{need.replace('_', '-')}", file=sys.stderr)
            return 32

    try:
        claims = board_claims(a.board_cmd, a.repo)
    except Exception as e:
        print(f"quiescence check could not read the board: {e}", file=sys.stderr)
        return 32   # not established -- never a clear result

    if a.mode == "claims":
        for res in sorted(r for r, c in claims.items() if c.get("session_id") == a.session):
            print(res)
        return 0

    others = {c.get("session_id") for r, c in claims.items()
              if c.get("session_id") != a.session and r.startswith(a.resource_prefix)}
    others.discard(None)
    print(len(others))
    return 0


if __name__ == "__main__":
    sys.exit(main())
