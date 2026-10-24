"""First .rs write and pre-write read calls per builder transcript.

Dedupes assistant records by message.id (one API response is written as several
records), as metrics/scripts/claude_transcript_usage.py establishes. Tool uses
are unioned by tool_use id across a response's records. A write whose tool_result
is is_error=True did not happen and is reported separately from the first
effective write.
"""
import json, sys, re, os
READ_TOOLS={"Read","Grep","Glob","NotebookRead","WebFetch","WebSearch"}
WRITE_TOOLS={"Write","Edit","MultiEdit","NotebookEdit"}
BASH_READ=re.compile(r'(^|[;&|(]\s*|\n\s*)(cat|head|tail|sed\s+-n|less|grep|rg|find|ls|wc|awk|git\s+show|git\s+log|git\s+diff|sha256sum|stat)\b')
WRITEISH=re.compile(r'(>>?)|(\btee\b)|(\bsed\s+-i)|(\bcp\b)|(\bmv\b)|(\bpatch\b)|(open\()|(\.write)|(write_text)|(\bgit\s+apply\b)')

def load(path):
    calls={}; order=[]; results={}; prefix=False; inherited=set()
    for num,line in enumerate(open(path,errors="ignore"),1):
        try: r=json.loads(line)
        except Exception: continue
        if not isinstance(r,dict): continue
        if r.get("type")=="fork-context-ref": prefix = (num==1); continue
        m=r.get("message") or {}
        is_inh = prefix and r.get("type")=="assistant" and "parentUuid" in r and r["parentUuid"] is None
        if prefix and not is_inh: prefix=False
        cont=m.get("content") if isinstance(m,dict) else None
        if isinstance(cont,list):
            for c in cont:
                if isinstance(c,dict) and c.get("type")=="tool_result":
                    results[c.get("tool_use_id")]=bool(c.get("is_error"))
        if is_inh: inherited.add(m.get("id")); continue
        u=m.get("usage") if isinstance(m,dict) else None
        if r.get("type")!="assistant" or not isinstance(u,dict) or m.get("model")=="<synthetic>": continue
        mid=m.get("id")
        if mid not in calls:
            calls[mid]={"ts":r.get("timestamp"),"uncached":0,"cached":0,"output":0,"uses":{},"model":m.get("model")}
            order.append(mid)
        row=calls[mid]
        row["uncached"]=(u.get("input_tokens") or 0)+(u.get("cache_creation_input_tokens") or 0)
        row["cached"]=u.get("cache_read_input_tokens") or 0
        row["output"]=max(row["output"],u.get("output_tokens") or 0)
        for c in (cont or []):
            if isinstance(c,dict) and c.get("type")=="tool_use":
                row["uses"][c.get("id")]=(c.get("name"),c.get("input") or {})
    seq=[calls[m] for m in order]
    for r in seq:
        r["ctx"]=r["uncached"]+r["cached"]
        r["wt"]=r["uncached"]+r["cached"]/10.0+5*r["output"]
    return seq,results

def scan(path):
    seq,results=load(path); reads=0; cands=[]
    for i,c in enumerate(seq):
        for tid,(name,inp) in c["uses"].items():
            hit=False; txt=""
            if name in WRITE_TOOLS:
                fp=inp.get("file_path") or inp.get("notebook_path") or ""
                if fp.endswith(".rs"): hit=True; txt=f"{name} -> {fp}"
            elif name=="Bash":
                cmd=inp.get("command") or ""
                if ".rs" in cmd and WRITEISH.search(cmd): hit=True; txt=cmd
            if hit: cands.append({"call":i,"ctx":c["ctx"],"reads_before":reads,"err":results.get(tid),"txt":txt,"ts":c["ts"]})
            if name in READ_TOOLS: reads+=1
            elif name=="Bash" and BASH_READ.search(inp.get("command") or ""): reads+=1
    return seq,cands,reads

if __name__=="__main__":
    out={}
    for p in sys.argv[1:]:
        seq,cands,reads=scan(p)
        tot=sum(c["wt"] for c in seq)
        aid=os.path.basename(p).replace("agent-","").replace(".jsonl","")
        ok=[c for c in cands if not c["err"]]
        f=ok[0] if ok else None
        print(f"{aid} calls={len(seq):3d} Mwt={tot/1e6:.3f} maxctx={max((c['ctx'] for c in seq),default=0):7d} "
              f"reads_total={reads:3d} cands={len(cands):2d} "
              f"FIRST_RS ctx={f['ctx'] if f else None} reads={f['reads_before'] if f else None} call#{f['call'] if f else None}")
        if cands and cands[0]["err"]:
            print(f"      (refused earlier attempt at ctx={cands[0]['ctx']} reads={cands[0]['reads_before']})")
        out[aid]={"calls":len(seq),"weighted":tot,"max_ctx":max((c['ctx'] for c in seq),default=0),
                  "reads_total":reads,"first_rs_ctx":f["ctx"] if f else None,
                  "pre_write_reads":f["reads_before"] if f else None,
                  "first_rs_text":(f["txt"][:200] if f else None),
                  "first_ts":seq[0]["ts"] if seq else None,"last_ts":seq[-1]["ts"] if seq else None}
    if os.environ.get("OUT"): json.dump(out,open(os.environ["OUT"],"w"),indent=1)

# Added 2026-10-24: the ramp decomposition quoted in the R2 score discussion is
# reproducible from this module. For each builder, scan() returns the per-call
# (uncached, cached, tool uses) sequence; the "ramp" is calls 0..first .rs write,
# adjudicated by reading the matched command rather than trusting the detector.
# Astra has not independently reproduced the decomposition and declined to
# certify it as a baseline; this script is published so it can be checked or
# refuted rather than argued about.
