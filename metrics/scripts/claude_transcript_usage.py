import json,sys,os
T="/home/bline/.claude/projects/-home-bline-code-work-engine/e702a3e4-eb8d-4690-8383-20dbf35b2850.jsonl"
seen=set(); rows=[]
with open(T,errors="ignore") as f:
    for line in f:
        if '"usage"' not in line: continue
        try: r=json.loads(line)
        except Exception: continue
        m=r.get("message") or {}
        u=m.get("usage") if isinstance(m,dict) else None
        if not isinstance(u,dict): u=r.get("usage")
        if not isinstance(u,dict): continue
        key=(m.get("id") if isinstance(m,dict) else None, r.get("timestamp"))
        if key in seen: continue
        seen.add(key)
        inp=u.get("input_tokens") or 0
        cc=u.get("cache_creation_input_tokens") or 0
        cr=u.get("cache_read_input_tokens") or 0
        out=u.get("output_tokens") or 0
        rows.append(dict(ts=r.get("timestamp"), uncached=inp+cc, cached=cr, output=out))
# baseline's unit: weighted = uncached + cached/10 + 5*output
W=lambda r: r["uncached"] + r["cached"]/10.0 + 5*r["output"]
tot=sum(W(r) for r in rows)
print(f"distinct assistant calls with usage: {len(rows)}")
print(f"cumulative weighted tokens (session to date): {tot:,.0f}  ({tot/1e6:.3f} Mwt)")
print(f"  uncached {sum(r['uncached'] for r in rows):,}  cached {sum(r['cached'] for r in rows):,}  output {sum(r['output'] for r in rows):,}")
if rows: print(f"first ts {rows[0]['ts']}   last ts {rows[-1]['ts']}")
json.dump(dict(calls=len(rows), weighted=tot,
  uncached=sum(r['uncached'] for r in rows), cached=sum(r['cached'] for r in rows),
  output=sum(r['output'] for r in rows), last_ts=rows[-1]['ts'] if rows else None),
  open(sys.argv[1],"w"), indent=1)
