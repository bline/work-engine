#!/usr/bin/env python3
"""Itemize a session's standing context by component, to find what a trim should remove.

Usage: python3 -I standing.py [SINCE]      SINCE is an ISO-8601 prefix, default 2026-10-01

floor.py answers "did the floor move". This answers "what is in the floor".
A session's standing context is assembled from three places in the rollout,
all re-sent on every later model call in the window:

  session_meta.base_instructions.text   the harness's own base prompt
  world_state.state                     permissions, host_skills, agents_md, ...
  the response_item messages before the first token_count

Bytes are exact. **Do not convert them to tokens.** world_state is the client's
own state snapshot, written to the rollout so a reader can reconstruct what the
session knew; only part of it becomes prompt input. agents_md does, as a
user-role message. approved_command_prefixes does not: five codex-tui sessions
identical in every other component measured 18,348 first-call tokens with
31,627 bytes of approval rules and 18,348 with 248 bytes -- two tokens apart.
An earlier version of this script printed an "est tokens" column derived from
one average bytes-per-token ratio, and that column produced a false 5,780-token
reading of those rules. It is gone.

To size a component in tokens, compare first-call input_tokens across a change
that moves only that component (floor.py), or A/B the model-visible prompt for
free with `codex debug prompt-input -c <override>`, which renders the message
list without a model call. Neither the base prompt nor the tool schemas appear
in the rollout or in prompt-input, so they can only be sized the first way.
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
print("%s  %s  %s" % (latest["started"][:19], latest["id"], latest["agent"]))
print("standing payload recorded in the rollout: %d bytes" % total)
print("first call: %d input tokens -- NOT a sum over the components below\n"
      % latest["first_call_tokens"])
print("%-34s %9s %7s  %s" % ("component", "bytes", "share", "prompt input?"))
SENT = {"world_state.agents_md": "yes, as a user message",
        "world_state.permissions": "NO, client-side only (measured)",
        "world_state.host_skills": "in part, inside developer message [0]",
        "base_instructions": "yes", "pre-first-call messages": "yes"}
for name, size in sorted(latest["items"].items(), key=lambda kv: -kv[1]):
    if size < 100: continue
    print("%-34s %9d %6.1f%%  %s" % (name, size, 100 * size / total, SENT.get(name, "unknown")))

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
