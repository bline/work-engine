import json,glob,os,collections,pickle
from multiprocessing import Pool
files=[f for d in ("sessions","archived_sessions") for f in glob.glob(os.path.expanduser(f"~/.codex/{d}/**/*.jsonl"),recursive=True) if os.path.getmtime(f)>1790900000]
COEF={"g6":7.826,"s56":4.381}
def parse(f):
    meta={}; model=eff=None; prev=None; cost=collections.Counter(); first_task=None; tools=collections.Counter(); t0=t1=None
    with open(f,errors="ignore") as fh:
        for i,line in enumerate(fh):
            if i==0:
                try: p=json.loads(line)["payload"]; p.pop("base_instructions",None); meta={k:p.get(k) for k in ("id","agent_path","agent_role","agent_nickname","parent_thread_id","forked_from_id","cwd","timestamp")}
                except: pass
                continue
            if '"turn_context"' in line[:120]:
                try:
                    p=json.loads(line)["payload"]; model=p.get("model",model)
                    eff=p.get("effort") or ((p.get("collaboration_mode") or {}).get("settings") or {}).get("reasoning_effort") or eff
                except: pass
                continue
            if first_task is None and '"role":"user"' in line and '"input_text"' in line:
                try:
                    o=json.loads(line); c=o["payload"].get("content") or []
                    txt=" ".join(x.get("text","") for x in c if isinstance(x,dict))
                    if txt and not txt.startswith("<") and "AGENTS.md" not in txt[:200]: first_task=txt[:300].replace("\n"," ")
                except: pass
            if '"function_call"' in line[:150] or '"custom_tool_call"' in line[:150]:
                try: tools[json.loads(line)["payload"].get("name")]+=1
                except: pass
            if '"token_count"' not in line: continue
            try: o=json.loads(line)
            except: continue
            t=o.get("timestamp",""); tu=((o.get("payload") or {}).get("info") or {}).get("total_token_usage")
            if not tu: continue
            if prev is not None and tu.get("total_tokens")==prev.get("total_tokens"): continue
            d=tu if prev is None else {k:max(0,tu.get(k,0)-prev.get(k,0)) for k in tu}
            prev=tu
            if not ("2026-10-05" <= t[:10] <= "2026-10-09"): continue
            t0=t0 or t; t1=t
            fam="g6" if (model or "").startswith("gpt-6") else "s56"
            unc=d.get("input_tokens",0)-d.get("cached_input_tokens",0); cac=d.get("cached_input_tokens",0); out=d.get("output_tokens",0)
            cost["units"]+=(unc+cac/10+5*out)/1e6*COEF[fam]
            cost["unc"]+=unc; cost["cac"]+=cac; cost["out"]+=out; cost["reason"]+=d.get("reasoning_output_tokens",0); cost["calls"]+=1
            cost["eff:%s/%s"%(model,eff)]+=(unc+cac/10+5*out)/1e6*COEF[fam]
    return dict(file=f,meta=meta,cost=cost,first=first_task,tools=tools,t0=t0,t1=t1)
if __name__=="__main__":
    with Pool(8) as p: R=[r for r in p.map(parse,files) if r["cost"]["units"]>0]
    pickle.dump(R,open(os.environ["OUT"],"wb"))
    R.sort(key=lambda r:-r["cost"]["units"])
    print(len(R),"sessions; est units",round(sum(r["cost"]["units"] for r in R)))
    for r in R:
        m=r["meta"]; c=r["cost"]
        print(f"{c['units']:7.0f} {str(m.get('agent_path') or 'ROOT:'+str(m.get('id'))[:13]):48.48} {str(m.get('agent_role')):8} {r['t0'][5:16] if r['t0'] else ''}→{r['t1'][5:16] if r['t1'] else ''} | {(r['first'] or '')[:110]}")
