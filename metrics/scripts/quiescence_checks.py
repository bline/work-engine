#!/usr/bin/env python3
"""Independent quiescence checks for a root rotation. No model, no network.

Neither host CLI can supply a parent-scoped child count. `claude agents --json`
lists machine-wide sessions including the root itself (status busy), so using it
would report live children on every rotation forever; `codex agents` has no
machine-readable output at all. What IS externally visible and parent-scoped is
the coordination board's advisory claims, which doctrine already requires a root
to release before transfer. These checks read those.

Contract with the coordinator:
  claims   -> one held resource per line, empty output = none held. Exit 0 only
              when the board was actually read; any failure exits non-zero so
              empty output can never be mistaken for a clear result.
  children -> a single integer on stdout, or non-zero exit and no integer.
"""
import argparse, datetime, json, subprocess, sys


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


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("mode", choices=["claims", "children"])
    ap.add_argument("--board-cmd", required=True)
    ap.add_argument("--repo", required=True)
    ap.add_argument("--session", required=True, help="the predecessor's session id")
    ap.add_argument("--resource-prefix", default="",
                    help="children: only count claims on resources with this prefix, "
                         "so unrelated roles are not counted as this root's children")
    a = ap.parse_args()

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
