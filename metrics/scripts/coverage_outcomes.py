#!/usr/bin/env python3
"""Coverage-observation outcomes: freshness, content verification, store generation and recommended action, aggregated per session.

Usage: python3 -I coverage_outcomes.py ~/.codex/sessions/2026/10/12/rollout-*.jsonl

Reading Codex rollout logs correctly (two mistakes this script exists to avoid):
  - a tool call's payload carries its body in "input"; "arguments" is the older
    field name and is absent for custom_tool_call entries;
  - MCP tools are invoked *inside* the exec cell as
    tools.mcp__<server>__<tool>(...), usually batched in Promise.allSettled.
    They are not separate function_call entries, so counting calls by payload
    name reports zero MCP use however heavily the graph was queried.
Per-call token usage is info.last_token_usage. info.total_token_usage is
account-lifetime cumulative and must not be read as a session total.
"""
import json, sys, re, os, collections
PAT={k:re.compile(p) for k,p in {
 "freshness":r'"freshness":\s*"(\w+)"',
 "recommended_action":r'"recommended_action":\s*"(\w+)"',
 "content_verified":r'"content_verified":\s*(true|false)',
 "generation_matches":r'"generation_matches":\s*(true|false)',
 "indexed_at":r'"indexed_at":\s*"([^"]+)"',
}.items()}
for f in sys.argv[1:]:
    meta={}; agg=collections.defaultdict(collections.Counter)
    for i,line in enumerate(open(f,errors="ignore")):
        if i==0:
            try: meta=json.loads(line)["payload"]
            except Exception: pass
            continue
        if "freshness" not in line and "indexed_at" not in line and "recommended_action" not in line: continue
        for k,p in PAT.items():
            for m in p.findall(line): agg[k][m]+=1
    if not agg: continue
    print("%-14s %-40s %s" % ((meta.get("timestamp") or "")[5:16], (meta.get("agent_path") or "-")[:40], os.path.basename(f)[8:16]))
    for k in ("indexed_at","freshness","content_verified","generation_matches","recommended_action"):
        if agg[k]: print("    %-20s %s" % (k, dict(agg[k].most_common(6))))
