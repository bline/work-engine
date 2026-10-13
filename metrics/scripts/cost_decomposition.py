#!/usr/bin/env python3
"""Per-call cost decomposition: new input, cached context re-send and output,
as shares of estimated weighted tokens.

Usage: python3 -I cost_decomposition.py ~/.codex/sessions/2026/10/12/rollout-*.jsonl

Per-call token usage is info.last_token_usage. info.total_token_usage is
account-lifetime cumulative and must not be read as a session total.
"""
import json,sys,os
COEF=7.826/1e6
for f in sys.argv[1:]:
    meta=json.loads(open(f,errors="ignore").readline())["payload"]
    n=0; unc=cac=out=0
    for line in open(f,errors="ignore"):
        if '"token_count"' not in line: continue
        try: info=(json.loads(line)["payload"].get("info") or {})
        except Exception: continue
        l=info.get("last_token_usage") or {}
        i_=l.get("input_tokens",0); c=l.get("cached_input_tokens",0); o=l.get("output_tokens",0)
        if i_==0 and o==0: continue
        n+=1; unc+=i_-c; cac+=c; out+=o
    if not n: continue
    wt=unc+cac/10+5*out
    print("%-40s n=%-5d unc/call=%6.0f cached/call=%7.0f out/call=%5.0f | units/call=%.3f  (new-input %.0f%%, cached re-send %.0f%%, output %.0f%%)" % (
      (meta.get("agent_path") or "-")[:40], n, unc/n, cac/n, out/n, wt*COEF/n,
      100*unc/wt, 100*(cac/10)/wt, 100*(5*out)/wt))
