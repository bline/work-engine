import json,glob,os,collections,re
from multiprocessing import Pool
files=[f for d in ("sessions","archived_sessions") for f in glob.glob(os.path.expanduser(f"~/.codex/{d}/**/*.jsonl"),recursive=True) if os.path.getmtime(f)>1790900000]
def parse(f):
    prev=None; pending=[]; c=collections.Counter(); first_edit=False; sub=False
    with open(f,errors="ignore") as fh:
        for i,line in enumerate(fh):
            if i==0:
                sub='"agent_path"' in line; continue
            h=line[:200]
            if '"function_call"' in h or '"custom_tool_call"' in h:
                try:
                    p=json.loads(line)["payload"]; n=p.get("name"); s=(p.get("input") or p.get("arguments") or "")
                    pending.append(n if n!="exec" else ("edit" if re.search(r"apply_patch|begin patch",s,re.I) else "exec"))
                    if re.search(r"(AGENTS|DESIGN|PHILOSOPHY|SKILL)\.md",s): pending.append("docs-bootstrap")
                except: pass
                continue
            if '"token_count"' not in line: continue
            try: o=json.loads(line)
            except: continue
            t=o.get("timestamp",""); tu=((o.get("payload") or {}).get("info") or {}).get("total_token_usage")
            if not tu: continue
            if prev is not None and tu.get("total_tokens")==prev.get("total_tokens"): continue
            d=tu if prev is None else {k:max(0,tu.get(k,0)-prev.get(k,0)) for k in tu}
            prev=tu; acts=pending; pending=[]
            if not ("2026-10-05"<=t[:10]<="2026-10-09"): continue
            u=(d.get("input_tokens",0)-d.get("cached_input_tokens",0)+d.get("cached_input_tokens",0)/10+5*d.get("output_tokens",0))/1e6*7.826
            if "edit" in acts: first_edit=True
            k="sub" if sub else "root"
            c[k+":calls"]+=1; c[k+":units"]+=u
            c[k+":in"]+=d.get("input_tokens",0); c[k+":cached"]+=d.get("cached_input_tokens",0); c[k+":out"]+=d.get("output_tokens",0)
            c[k+":u_cached"]+=d.get("cached_input_tokens",0)/10/1e6*7.826; c[k+":u_unc"]+=(d.get("input_tokens",0)-d.get("cached_input_tokens",0))/1e6*7.826; c[k+":u_out"]+=5*d.get("output_tokens",0)/1e6*7.826
            if acts and all(a in ("wait_agent","wait","sleep","list_agents") for a in acts): c[k+":poll_calls"]+=1; c[k+":poll_units"]+=u
            if not first_edit: c[k+":pre_edit_units"]+=u
            if "docs-bootstrap" in acts: c[k+":bootstrap_units"]+=u
    c["sessions_"+("sub" if sub else "root")]+=1
    return c
if __name__=="__main__":
    with Pool(8) as p: C=sum(p.map(parse,files),collections.Counter())
    for k in ("root","sub"):
        U=C[k+":units"]
        print(f"[{k}] sessions={C['sessions_'+k]} calls={C[k+':calls']} units={U:.0f} avg ctx/call={C[k+':in']/C[k+':calls']/1e3:.0f}K cached={C[k+':cached']/C[k+':in']*100:.0f}%")
        print(f"   cost split: cached input {C[k+':u_cached']/U*100:.0f}%, uncached input {C[k+':u_unc']/U*100:.0f}%, output {C[k+':u_out']/U*100:.0f}%")
        print(f"   pure polling calls (wait/list/sleep only): {C[k+':poll_calls']} calls, {C[k+':poll_units']:.0f} units ({C[k+':poll_units']/U*100:.0f}%)")
        print(f"   before first code edit: {C[k+':pre_edit_units']:.0f} units ({C[k+':pre_edit_units']/U*100:.0f}%); turns reading AGENTS/DESIGN/PHILOSOPHY/SKILL: {C[k+':bootstrap_units']:.0f} units")
