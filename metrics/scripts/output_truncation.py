#!/usr/bin/env python3
"""Tool output discarded by harness truncation before the model could read it, classified by what was lost.

Usage: python3 -I output_truncation.py ~/.codex/sessions/2026/10/12/rollout-*.jsonl

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
import json,sys,re,os,collections
for f in sys.argv[1:]:
    meta=json.loads(open(f,errors="ignore").readline())["payload"]
    kinds=collections.Counter()
    for line in open(f,errors="ignore"):
        if "truncated output (original token count" not in line: continue
        p=json.loads(line).get("payload") or {}
        s=json.dumps(p.get("output") or p)
        m=re.search(r"truncated output \(original token count: (\d+)\)",s)
        tok=int(m.group(1)) if m else 0
        # classify by what the surviving head looks like
        if '\\"name\\":\\"mcp__codebase_memory_mcp__' in s or '"name":"mcp__codebase_memory_mcp__' in s: k="mcp tool-schema dump"
        elif "codebase_memory" in s or "qualified_name" in s or "freshness" in s: k="codebase-memory result"
        elif "cargo" in s.lower() or "test result" in s or "warning:" in s: k="cargo/test output"
        elif "diff --git" in s or "@@" in s: k="git diff"
        else: k="other"
        kinds[k]+=1
    if kinds: print("%-42s %s"%((meta.get("agent_path") or "-")[:42], dict(kinds.most_common())))
