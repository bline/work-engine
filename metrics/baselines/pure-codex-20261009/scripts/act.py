import json,glob,os,collections,pickle,re
from multiprocessing import Pool
files=[f for d in ("sessions","archived_sessions") for f in glob.glob(os.path.expanduser(f"~/.codex/{d}/**/*.jsonl"),recursive=True) if os.path.getmtime(f)>1790900000]
COEF={"g6":7.826,"s56":4.381}
COORD={"wait_agent","send_message","followup_task","wait","list_agents","spawn_agent","sleep","request_user_input_async","interrupt_agent"}
def classify_exec(s):
    s=s.lower()
    if re.search(r"apply_patch|\*\*\* begin patch|cat\s*>|<<\s*'?eof|write_file|tee |\.write\(|python3? - <<",s):
        if re.search(r"\.md\b",s) and not re.search(r"\.rs\b|\.mjs\b|\.sql\b",s): return "write-docs"
        return "edit-code"
    if re.search(r"cargo (test|nextest|clippy|build|check|fmt)|node --test|npm (run )?test|pytest|validate|gate|sha256sum|cargo metadata",s): return "verify/gate"
    if re.search(r"chatboard|claim|release|durable-state",s): return "coordination"
    if re.search(r"git (commit|update-ref|worktree|push|merge|cherry|checkout|stash|apply)",s): return "git/publication"
    return "read/inspect"
def parse(f):
    meta={}; model=None; prev=None; pending=[]; out=collections.Counter(); calls=collections.Counter()
    with open(f,errors="ignore") as fh:
        for i,line in enumerate(fh):
            if i==0:
                try: p=json.loads(line)["payload"]; meta={k:p.get(k) for k in ("id","agent_path","agent_role")}
                except: pass
                continue
            h=line[:200]
            if '"turn_context"' in h:
                try: model=json.loads(line)["payload"].get("model",model)
                except: pass
                continue
            if '"function_call"' in h or '"custom_tool_call"' in h:
                try:
                    p=json.loads(line)["payload"]; n=p.get("name")
                    pending.append("coordinate-agents" if n in COORD else classify_exec(p.get("input") or p.get("arguments") or ""))
                except: pass
                continue
            if '"token_count"' not in line: continue
            try: o=json.loads(line)
            except: continue
            t=o.get("timestamp",""); tu=((o.get("payload") or {}).get("info") or {}).get("total_token_usage")
            if not tu: continue
            if prev is not None and tu.get("total_tokens")==prev.get("total_tokens"): continue
            d=tu if prev is None else {k:max(0,tu.get(k,0)-prev.get(k,0)) for k in tu}
            prev=tu; acts=pending or ["respond/reason"]; pending=[]
            if not ("2026-10-05"<=t[:10]<="2026-10-09"): continue
            fam="g6" if (model or "").startswith("gpt-6") else "s56"
            u=(d.get("input_tokens",0)-d.get("cached_input_tokens",0)+d.get("cached_input_tokens",0)/10+5*d.get("output_tokens",0))/1e6*COEF[fam]
            for a in acts: out[a]+=u/len(acts)
            out["_ctx"]+=d.get("input_tokens",0); out["_n"]+=1
    return dict(meta=meta,act=out)
if __name__=="__main__":
    with Pool(8) as p: R=[r for r in p.map(parse,files) if r["act"]["_n"]]
    pickle.dump(R,open(os.environ["OUT"],"wb"))
