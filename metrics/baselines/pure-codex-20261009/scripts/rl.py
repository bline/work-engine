import json,glob,os,datetime as dt
files=[f for d in ("sessions","archived_sessions") for f in glob.glob(os.path.expanduser(f"~/.codex/{d}/**/*.jsonl"),recursive=True)]
cut=dt.datetime(2026,9,28).timestamp()
pts=[]
for f in files:
    if os.path.getmtime(f)<cut: continue
    with open(f,errors="ignore") as fh:
        for line in fh:
            if '"rate_limits"' not in line: continue
            try: o=json.loads(line)
            except: continue
            ts=o.get("timestamp"); p=o.get("payload",{})
            rl=p.get("rate_limits") or (p.get("info") or {}).get("rate_limits")
            if not rl or not ts: continue
            pr=rl.get("primary") or {}
            if pr.get("window_minutes")!=10080: continue
            t=dt.datetime.fromisoformat(ts.replace("Z","+00:00")).timestamp()
            pts.append((t,rl.get("limit_id"),pr.get("used_percent"),pr.get("resets_at"),rl.get("plan_type"),os.path.basename(f)[:40]))
pts.sort()
fmt=lambda s:dt.datetime.fromtimestamp(s).strftime("%m-%d %H:%M")
print("points",len(pts),"from",fmt(pts[0][0]),"to",fmt(pts[-1][0]))
prev={}
for t,lid,u,r,plan,f in pts:
    pv=prev.get(lid)
    if pv is None or abs(r-pv[2])>3600:
        early = pv is not None and t < pv[2]-600
        print(f"{fmt(t)} [{lid}/{plan}] NEW WINDOW resets_at={fmt(r)} used={u}%"+(f"  prev window (resets_at {fmt(pv[2])}) peaked {pv[3]}% -> {'EARLY reset' if early else 'natural rollover'}" if pv else ""))
        prev[lid]=[t,u,r,u]
    else:
        prev[lid][3]=max(prev[lid][3],u)
for lid,pv in prev.items(): print(f"current [{lid}] resets_at={fmt(pv[2])} peak used={pv[3]}%")
print("\n=== distinct weekly windows (keyed by resets_at, rounded to 5 min) ===")
win={}
for t,lid,u,r,plan,f in pts:
    k=round(r/300)
    w=win.setdefault(k,dict(first=t,last=t,peak=u,r=r,plans=set(),hit100=None))
    w["last"]=max(w["last"],t); w["peak"]=max(w["peak"],u); w["plans"].add(plan)
    if u>=99 and w["hit100"] is None: w["hit100"]=t
ws=sorted(win.values(),key=lambda w:w["first"])
for i,w in enumerate(ws):
    start=w["r"]-7*86400
    prevend = ws[i-1]["r"] if i else None
    kind = "" if prevend is None else ("natural rollover" if w["first"]>=prevend-600 else f"EARLY reset ({(prevend-w['first'])/86400:.1f}d before prior window would end)")
    print(f"window start~{fmt(start)} first-seen {fmt(w['first'])} last-seen {fmt(w['last'])} peak {w['peak']:.0f}% "+(f"hit≥99% {fmt(w['hit100'])} " if w['hit100'] else "")+f"{sorted(w['plans'])}  {kind}")
