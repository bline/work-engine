import json,glob,os,re,collections,pickle
from multiprocessing import Pool
files=[f for d in ("sessions","archived_sessions") for f in glob.glob(os.path.expanduser(f"~/.codex/{d}/**/*.jsonl"),recursive=True) if os.path.getmtime(f)>1790900000]
COEF=7.826/1e6; INF=10**9
CMD=re.compile(r'cmd:\s*"((?:[^"\\]|\\.)*)"(?:\s*,\s*workdir:\s*"([^"]*)")?')
DOCS={"AGENTS.md","DESIGN.md","PHILOSOPHY.md"}
lens={}
def tok_per_line(path):
    if path not in lens:
        try:
            b=open(path,"rb").read(); n=b.count(b"\n") or 1; lens[path]=max(3.0,len(b)/n/4)
        except Exception: lens[path]=12.0
    return lens[path]
def norm(p,wd):
    p=p.strip("'\"")
    return os.path.normpath(p if p.startswith("/") else os.path.join(wd or "/home/bline/code/work-engine",p))
def reads_in(cmd,wd):
    out=[]
    for part in re.split(r"&&|;|\|\||\n",cmd):
        part=part.strip()
        m=re.match(r"(?:nl -ba\s+(\S+)\s*\|\s*)?sed -n\s+'([^']+)'\s*(\S+)?",part)
        if m and (m.group(1) or m.group(3)):
            path=m.group(1) or m.group(3)
            for a,b in re.findall(r"(\d+),(\d+)\s*\{?p",m.group(2)): out.append((norm(path,wd),int(a),int(b)))
            continue
        m=re.match(r"cat\s+((?:[^|<>&;]+))$",part)
        if m and "<<" not in part:
            for p in m.group(1).split():
                if not p.startswith("-"): out.append((norm(p,wd),1,INF))
            continue
        m=re.match(r"head\s+-n\s*(\d+)\s+(\S+)",part)
        if m: out.append((norm(m.group(2),wd),1,int(m.group(1))))
    return out
def writes_in(s,wd):
    w=set(norm(p,wd) for p in re.findall(r"\*\*\* (?:Update|Add|Delete) File: ([^\\\n\"]+)",s))
    for p in re.findall(r"(?:>|tee)\s+([\w./-]+\.\w+)",s): w.add(norm(p,wd))
    for p in re.findall(r"sed -i\S*\s+'[^']*'\s+(\S+)",s): w.add(norm(p,wd))
    reset = bool(re.search(r"git (checkout|stash|reset|apply|cherry-pick|merge)|cargo fmt",s))
    return w,reset
def overlap(ivs,a,b):
    cov=0
    for x,y in ivs:
        lo,hi=max(a,x),min(b,y)
        if hi>=lo: cov+=hi-lo+1
    return cov
def parse(f):
    seen=collections.defaultdict(list); lastin=0; pend=[]; prev=None; out=[]; sub=False; docs=collections.Counter()
    with open(f,errors="ignore") as fh:
        for i,line in enumerate(fh):
            if i==0: sub='"agent_path"' in line; continue
            h=line[:200]
            if '"custom_tool_call"' in h or '"function_call"' in h:
                try: p=json.loads(line)["payload"]
                except: continue
                s=p.get("input") or p.get("arguments") or ""
                tot=red=0.0
                for m in CMD.finditer(s):
                    cmd=bytes(m.group(1),"utf-8").decode("unicode_escape","ignore"); wd=m.group(2)
                    for path,a,b in reads_in(cmd,wd):
                        if os.path.basename(path) in DOCS and a==1 and b==INF: docs[os.path.basename(path)]+=1
                        try: n=sum(1 for _ in open(path,"rb"))
                        except Exception: n=400
                        b2=min(b,n); a2=min(a,b2)
                        span=b2-a2+1; cov=min(span,overlap(seen[path],a2,b2))
                        t=tok_per_line(path); tot+=span*t; red+=cov*t
                        seen[path].append((a2,b2))
                    w,reset=writes_in(cmd,wd)
                    for x in w: seen.pop(x,None)
                    if reset: seen.clear()
                w,reset=writes_in(s,None)
                for x in w: seen.pop(x,None)
                if reset: seen.clear()
                pend.append([tot,red,None]); continue
            if ('"custom_tool_call_output"' in h or '"function_call_output"' in h) and pend:
                try:
                    p=json.loads(line)["payload"]; o=p.get("output")
                    txt=o if isinstance(o,str) else " ".join(x.get("text","") for x in (o or []) if isinstance(x,dict))
                    pend[-1][2]=len(txt)/4
                except: pass
                continue
            if '"token_count"' not in line: continue
            try: o=json.loads(line)
            except: continue
            t=o.get("timestamp",""); info=(o.get("payload") or {}).get("info") or {}
            tu=info.get("total_token_usage"); lu=info.get("last_token_usage")
            if not tu or not lu: continue
            if prev is not None and tu["total_tokens"]==prev: continue
            prev=tu["total_tokens"]; acts=pend; pend=[]
            cur=lu.get("input_tokens",0)
            if lastin and cur < 0.7*lastin: seen.clear()
            lastin=cur
            if "2026-10-05"<=t[:10]<="2026-10-09":
                out.append((lu.get("input_tokens",0),lu.get("cached_input_tokens",0),lu.get("output_tokens",0),sum(min(a,x) if x is not None else a for a,_,x in acts),sum((b*min(1,x/a) if (x is not None and a>0) else b) for a,b,x in acts)))
    return dict(sub=sub,calls=out,docs=docs)
if __name__=="__main__":
    with Pool(8) as p: R=[r for r in p.map(parse,files) if r["calls"]]
    pickle.dump(R,open(os.environ["S"]+"/reread.pkl","wb"))
    base=red_add=red_carry=read_add=read_carry=0; full_calls=0; full_cost=0; ncalls=0; docs=collections.Counter(); persess=[]
    for r in R:
        docs.update(r["docs"]); c=r["calls"]
        for k,(i,cc,o,tot,red) in enumerate(c):
            ncalls+=1
            cost=(cc*0.1+(i-cc)+5*o)*COEF; base+=cost
            # carry: re-sent each later call until context shrinks
            carry=0
            for j in range(k+1,len(c)):
                if c[j][0] < c[j-1][0]*0.7: break
                carry+=1
            read_add+=tot*COEF; read_carry+=tot*0.1*carry*COEF
            red_add+=red*COEF; red_carry+=red*0.1*carry*COEF
            if tot>0 and red>=0.95*tot: full_calls+=1; full_cost+=cost
    print(f"sessions {len(R)}, calls {ncalls}, est units {base:.0f}")
    print(f"file-read tokens: added {read_add/COEF/1e6:.1f}M -> cost on entry {read_add:.0f} + carried {read_carry:.0f} units = {(read_add+read_carry)/base*100:.1f}% of spend")
    print(f"REDUNDANT re-read tokens: {red_add/COEF/1e6:.1f}M ({red_add/read_add*100:.0f}% of read tokens) -> entry {red_add:.0f} + carried {red_carry:.0f} units = {(red_add+red_carry)/base*100:.1f}% of spend")
    print(f"calls that were (almost) purely redundant reads: {full_calls} calls, call cost {full_cost:.0f} units ({full_cost/base*100:.1f}%)")
    print("full reads of doctrine docs (sessions read):",dict(docs))
