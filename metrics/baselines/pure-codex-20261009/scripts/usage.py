import json,glob,os,sys,datetime as dt,collections,pickle
from multiprocessing import Pool
files=[f for d in ("sessions","archived_sessions") for f in glob.glob(os.path.expanduser(f"~/.codex/{d}/**/*.jsonl"),recursive=True)]
def parse(f):
    out=[]; model=None; eff=None; prev=None; cwd=None
    try:
        fh=open(f,errors="ignore")
    except: return out
    for line in fh:
        if '"turn_context"' in line[:200] or '"type":"turn_context"' in line:
            try:
                p=json.loads(line)["payload"]; model=p.get("model",model); cwd=p.get("cwd",cwd)
                eff=p.get("effort") or (p.get("collaboration_mode") or {}).get("settings",{}).get("reasoning_effort") or eff
            except: pass
            continue
        if '"token_count"' not in line: continue
        try: o=json.loads(line)
        except: continue
        p=o.get("payload",{}); info=p.get("info") or {}
        tu=info.get("total_token_usage"); rl=p.get("rate_limits") or {}
        pr=rl.get("primary") or {}
        t=o.get("timestamp")
        d=None
        if tu:
            if prev is None: d=tu
            elif tu.get("total_tokens",0)!=prev.get("total_tokens",0):
                d={k:max(0,tu.get(k,0)-prev.get(k,0)) for k in tu}
            prev=tu
        out.append((t,model,eff,cwd,d,pr.get("used_percent") if pr.get("window_minutes")==10080 else None,pr.get("resets_at"),rl.get("plan_type")))
    return out
if __name__=="__main__":
    with Pool(8) as pool: res=pool.map(parse,files,chunksize=4)
    ev=[e for r in res for e in r if e[0]]
    ev.sort(key=lambda e:e[0])
    pickle.dump(ev,open(sys.argv[1],"wb"))
    print(len(ev),ev[0][0],ev[-1][0])
