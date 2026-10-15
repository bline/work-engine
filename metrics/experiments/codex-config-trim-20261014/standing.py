#!/usr/bin/env python3
"""Itemize a session's standing context by component, to find what a trim should remove.

Usage: python3 -I standing.py [SINCE]      SINCE is an ISO-8601 prefix, default 2026-10-01

floor.py answers "did the floor move". This answers "what is in the floor".
A session's standing context is assembled from three places in the rollout,
all re-sent on every later model call in the window:

  session_meta.base_instructions.text   the harness's own base prompt
  world_state.state                     permissions, host_skills, agents_md, ...
  the response_item messages before the first token_count

Bytes are exact. Tokens are not: the whole standing payload divided by the
session's first-call input_tokens gives one bytes-per-token ratio, and applying
it to a component assumes that component tokenizes like the average. JSON-escaped
shell strings do not, so per-component token figures are an estimate and are
printed as such. Only the first-call total is measured.
"""
import json
import pathlib
import sys

SINCE = sys.argv[1] if len(sys.argv) > 1 else "2026-10-01"

def standing(path):
    meta = world = None
    message_bytes = 0
    first = None
    with path.open(errors="ignore") as handle:
        for number, line in enumerate(handle):
            if number > 400 and first:
                break
            try: record = json.loads(line)
            except json.JSONDecodeError: continue
            payload = record.get("payload")
            if not isinstance(payload, dict): continue
            if number == 0:
                meta = payload
                if (meta.get("timestamp") or "") < SINCE: return None
                continue
            state = payload.get("state")
            if isinstance(state, dict) and "permissions" in state and world is None:
                world = state
            elif payload.get("type") == "message" and world is None:
                content = payload.get("content")
                if isinstance(content, list):
                    message_bytes += sum(len(c.get("text", "")) for c in content if isinstance(c, dict))
            elif payload.get("type") == "token_count" and first is None:
                usage = ((payload.get("info") or {}).get("last_token_usage")) or {}
                if usage.get("input_tokens"): first = usage["input_tokens"]
    if not (meta and world and first): return None
    base = (meta.get("base_instructions") or {})
    items = {"base_instructions": len(base.get("text", "") if isinstance(base, dict) else base),
             "pre-first-call messages": message_bytes}
    for name, value in world.items():
        items["world_state." + name] = len(json.dumps(value))
    return {"id": (meta.get("id") or "")[:8], "started": meta["timestamp"],
            "agent": meta.get("agent_path") or "root", "first_call_tokens": first, "items": items}

rows = [r for r in (standing(p) for p in sorted((pathlib.Path.home() / ".codex/sessions").rglob("*.jsonl"))) if r]
if not rows:
    print("no sessions with a world_state snapshot at or after %s" % SINCE)
    raise SystemExit(0)
rows.sort(key=lambda r: r["started"])
latest = rows[-1]
total = sum(latest["items"].values())
ratio = total / latest["first_call_tokens"]
print("%s  %s  %s" % (latest["started"][:19], latest["id"], latest["agent"]))
print("standing payload %d bytes -> %d first-call tokens (%.2f bytes/token overall)\n"
      % (total, latest["first_call_tokens"], ratio))
print("%-34s %9s %7s  %9s" % ("component", "bytes", "share", "est tokens"))
for name, size in sorted(latest["items"].items(), key=lambda kv: -kv[1]):
    if size < 100: continue
    print("%-34s %9d %6.1f%%  %9.0f" % (name, size, 100 * size / total, size / ratio))

print("\nper-component change across sessions (bytes):")
names = [n for n, s in sorted(latest["items"].items(), key=lambda kv: -kv[1]) if s >= 1000]
print("%-19s %6s " % ("started", "tokens") + " ".join("%12s" % n[-12:] for n in names))
seen = set()
for row in rows:
    day = row["started"][:10]
    if day in seen and row is not latest: continue
    seen.add(day)
    print("%-19s %6d " % (row["started"][:19], row["first_call_tokens"])
          + " ".join("%12d" % row["items"].get(n, 0) for n in names))
