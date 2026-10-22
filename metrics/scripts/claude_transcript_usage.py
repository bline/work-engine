"""Weighted token accounting for one Claude Code transcript.

Usage: python3 -I claude_transcript_usage.py OUT.json [TRANSCRIPT]

One API response is written as several assistant records sharing
`message.id`, each with its own timestamp. Count each id once: input and cache
counters are identical across its records, and `output_tokens` can be partial
on early records, so take the maximum. Records with model `<synthetic>` are not
API calls. Weighted = uncached + cached/10 + 5 x output, the baseline's unit.

Corrected 2026-10-22: the first version deduplicated on (message.id, timestamp),
which counted repeated records of one response separately (1.95x on one
transcript, 2.17x on another).
"""
import json
import sys

DEFAULT = "/home/bline/.claude/projects/-home-bline-code-work-engine/e702a3e4-eb8d-4690-8383-20dbf35b2850.jsonl"
T = sys.argv[2] if len(sys.argv) > 2 else DEFAULT

calls = {}
with open(T, errors="ignore") as f:
    for line in f:
        if '"usage"' not in line:
            continue
        try:
            r = json.loads(line)
        except Exception:
            continue
        m = r.get("message") or {}
        u = m.get("usage") if isinstance(m, dict) else None
        if r.get("type") != "assistant" or not isinstance(u, dict) or m.get("model") == "<synthetic>":
            continue
        row = calls.setdefault(m.get("id"), dict(ts=r.get("timestamp"), uncached=0, cached=0, output=0))
        row["uncached"] = (u.get("input_tokens") or 0) + (u.get("cache_creation_input_tokens") or 0)
        row["cached"] = u.get("cache_read_input_tokens") or 0
        row["output"] = max(row["output"], u.get("output_tokens") or 0)

rows = list(calls.values())
W = lambda r: r["uncached"] + r["cached"] / 10.0 + 5 * r["output"]
tot = sum(W(r) for r in rows)
print(f"distinct API calls (message.id): {len(rows)}")
print(f"cumulative weighted tokens: {tot:,.0f}  ({tot/1e6:.3f} Mwt)")
print(f"  uncached {sum(r['uncached'] for r in rows):,}  cached {sum(r['cached'] for r in rows):,}  output {sum(r['output'] for r in rows):,}")
if rows:
    print(f"first ts {rows[0]['ts']}   last ts {rows[-1]['ts']}")
json.dump(dict(calls=len(rows), weighted=tot,
               uncached=sum(r["uncached"] for r in rows), cached=sum(r["cached"] for r in rows),
               output=sum(r["output"] for r in rows), last_ts=rows[-1]["ts"] if rows else None,
               dedup="message.id, max output"),
          open(sys.argv[1], "w"), indent=1)
