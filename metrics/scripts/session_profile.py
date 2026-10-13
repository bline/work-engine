#!/usr/bin/env python3
"""Per-session profile: effort, exec calls, codebase-memory intensity, source reads and estimated Plus-units.

Usage: python3 -I session_profile.py ~/.codex/sessions/2026/10/12/rollout-*.jsonl

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
COEF=7.826/1e6
MCP=re.compile(r"tools\.mcp__codebase_memory_mcp__(\w+)")
CLI=re.compile(r"codebase-memory-mcp\s+(?:cli\s+)?(?:--json\s+)?(\w+)")
CMD=re.compile(r'cmd:\s*"((?:[^"\\]|\\.)*)"')
READ=re.compile(r"\b(?:sed -n|cat|nl -ba|head|tail)\b")
RG=re.compile(r"\brg\b|\bgrep\b")
def scan(f):
    meta={}; rows=[]; names=collections.Counter(); models=collections.Counter()
    mcp=collections.Counter(); cli=collections.Counter()
    ex=withmcp=withread=withrg=reads=rgs=tasks=0
    for i,line in enumerate(open(f,errors="ignore")):
        if i==0:
            try: meta=json.loads(line)["payload"]
            except Exception: pass
            continue
        if '"token_count"' in line:
            try:
                p=json.loads(line)["payload"]; info=p.get("info") or {}
                rows.append((info.get("last_token_usage") or {}, (p.get("rate_limits") or {}).get("primary") or {}))
            except Exception: pass
            continue
        if '"turn_context"' in line[:140]:
            try:
                p=json.loads(line)["payload"]
                eff=p.get("effort") or ((p.get("collaboration_mode") or {}).get("settings") or {}).get("reasoning_effort")
                models[(p.get("model"),eff)]+=1
            except Exception: pass
            continue
        if '"task_started"' in line[:200]: tasks+=1; continue
        if '"custom_tool_call"' not in line[:200] and '"function_call"' not in line[:200]: continue
        try: p=json.loads(line)["payload"]
        except Exception: continue
        if p.get("type") not in ("custom_tool_call","function_call"): continue
        names[p.get("name")]+=1
        s=p.get("input") or p.get("arguments") or ""
        if p.get("name") not in ("exec","shell"): continue
        ex+=1
        m=MCP.findall(s); c=[x for x in CLI.findall(s) if x not in ("cli",)]
        for x in m: mcp[x]+=1
        for x in c: cli[x]+=1
        if m or c: withmcp+=1
        cmds=[bytes(x,"utf-8").decode("unicode_escape","ignore") for x in CMD.findall(s)]
        r=sum(1 for x in cmds if READ.search(x)); g=sum(1 for x in cmds if RG.search(x))
        reads+=r; rgs+=g; withread+=1 if r else 0; withrg+=1 if g else 0
    wt=0.0; ctx=[]
    for l,rl in rows:
        i_=l.get("input_tokens",0); c=l.get("cached_input_tokens",0); o=l.get("output_tokens",0)
        if i_==0 and o==0: continue
        wt+=(i_-c)+c/10+5*o; ctx.append(i_)
    q=(rows[0][1].get("used_percent") if rows else None, rows[-1][1].get("used_percent") if rows else None)
    return dict(f=os.path.basename(f), path=meta.get("agent_path") or "-", role=meta.get("agent_role") or "root",
      ts=meta.get("timestamp"), models=dict(models), calls=sum(names.values()), ex=ex, tasks=tasks,
      names=dict(names.most_common(6)), mcp=dict(mcp.most_common()), cli=dict(cli.most_common()),
      withmcp=withmcp, withread=withread, withrg=withrg, reads=reads, rgs=rgs,
      units=wt*COEF, ctx=(sum(ctx)/len(ctx) if ctx else 0), nctx=len(ctx), quota=q)
out=[]
for f in sys.argv[1:]:
    try: out.append(scan(f))
    except Exception as e: print("ERR",f,e)
out.sort(key=lambda r:(r["ts"] or ""))
for r in out:
    eff="/".join("%s %s"%(str(m).replace("gpt-6-","").replace("gpt-6","6"),e) for (m,e) in r["models"]) or "-"
    print("%-16s %-40s %-18s tasks=%-3d exec=%-4d units=%6.1f ctx=%5.0fK" % (
      (r["ts"] or "")[5:16], r["path"][:40], eff[:18], r["tasks"], r["ex"], r["units"], r["ctx"]/1000))
    print("     CBM: %d calls in %d/%d exec (%.0f%%) | reads=%d in %d (%.0f%%) | rg=%d in %d | quota %s"%(
      sum(r["mcp"].values())+sum(r["cli"].values()), r["withmcp"], r["ex"], 100*r["withmcp"]/max(r["ex"],1),
      r["reads"], r["withread"], 100*r["withread"]/max(r["ex"],1), r["rgs"], r["withrg"], r["quota"]))
    if r["mcp"] or r["cli"]: print("     tools:", r["mcp"], r["cli"] or "")
    print("     other:", {k:v for k,v in r["names"].items() if k not in ("exec","shell")})
