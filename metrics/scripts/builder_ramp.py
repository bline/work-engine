"""First .rs write and pre-write read calls per builder transcript.

Dedupes assistant records by message.id (one API response is written as several
records), as metrics/scripts/claude_transcript_usage.py establishes. Tool uses
are unioned by tool_use id across a response's records.

A candidate is any tool use that could have written Rust. Each candidate carries
a result status and a class, and both are published rather than collapsed:

  status  ok | error | unknown   an absent tool_result is `unknown`, never
                                 success (BR01, peer audit 2765)
  class   rust_write             some determinate write target ends in .rs
          not_rust_write         every determinate write target is another
                                 file, so the .rs was read or merely named
                                 (BR02, peer audit 2765)
          needs_adjudication     the writer is a python heredoc, an inline
                                 interpreter, patch or git apply, whose target
                                 this module does not interpret

Only an `ok` + `rust_write` candidate is reported as the first effective write.
`unknown` and `needs_adjudication` are reported separately and are decided by
reading the command, not by candidate order: order is not evidence of a write.
"""
import json, sys, re, os
READ_TOOLS={"Read","Grep","Glob","NotebookRead","WebFetch","WebSearch"}
WRITE_TOOLS={"Write","Edit","MultiEdit","NotebookEdit"}
BASH_READ=re.compile(r'(^|[;&|(]\s*|\n\s*)(cat|head|tail|sed\s+-n|less|grep|rg|find|ls|wc|awk|git\s+show|git\s+log|git\s+diff|sha256sum|stat)\b')
WRITEISH=re.compile(r'(>>?)|(\btee\b)|(\bsed\s+-i)|(\bcp\b)|(\bmv\b)|(\bpatch\b)|(open\()|(\.write)|(write_text)|(\bgit\s+apply\b)')
HEREDOC=re.compile(r'<<-?\s*(["\']?)([A-Za-z_][A-Za-z0-9_]*)\1')
REDIRECT=re.compile(r'(\d?>>?)\s*([^\s;&|)<>]+)')
QUOTED=re.compile(r"'[^']*'|\"[^\"]*\"")
INDETERMINATE=re.compile(r'\b(patch|git\s+apply)\b')
INTERPRETER=re.compile(r'\b(python3?|perl|ruby|node)\b')

def strip_heredocs(cmd):
    """Return (shell text with heredoc bodies removed, list of bodies).

    A heredoc body is data, not shell. `cat > result-b-v1.md <<EOF` followed by
    prose naming a .rs file is a write to the .md; reading the body as shell is
    what makes that command look like a Rust write.
    """
    out=[]; bodies=[]; rest=cmd
    while True:
        m=HEREDOC.search(rest)
        if not m: out.append(rest); break
        delim=m.group(2)
        head=rest[:m.start()]; after=rest[m.end():]
        nl=after.find("\n")
        if nl<0: out.append(head); bodies.append(after); break
        out.append(head+after[:nl])
        body=after[nl+1:]
        end=re.search(r'^\s*'+re.escape(delim)+r'\s*$', body, re.M)
        if not end: bodies.append(body); break
        bodies.append(body[:end.start()]); rest=body[end.end():]
    return "".join(out), bodies

def mask_quotes(s):
    """Same-length copy with quoted spans blanked, so `;` and `>` inside a
    sed script or a quoted argument are not read as shell structure."""
    out=list(s); i=0; n=len(s)
    while i<n:
        if s[i] in "'\"":
            j=s.find(s[i],i+1)
            if j<0: j=n-1
            for k in range(i,j+1): out[k]="\x00"
            i=j+1
        else: i+=1
    return "".join(out)

PATHISH=re.compile(r'[\w$./~-]')

def write_targets(cmd):
    """Determinate write targets and indeterminate writer mechanisms."""
    shell,bodies=strip_heredocs(cmd)
    masked=mask_quotes(shell)
    targets=[]; mech=[]
    def keep(t):
        t=t.strip()
        if t and PATHISH.search(t) and not t.startswith("&"): targets.append(t)
    for m in REDIRECT.finditer(masked): keep(shell[m.start(2):m.end(2)])
    for m in re.finditer(r'\btee\b((?:\s+-\S+)*)\s+([^\s;&|)]+)', masked): keep(shell[m.start(2):m.end(2)])
    bounds=[0]+[m.end() for m in re.finditer(r'&&|\|\||[;&|\n]', masked)]+[len(shell)]
    for a,b in zip(bounds,bounds[1:]):
        toks=[(shell[a+m.start():a+m.end()], masked[a+m.start():a+m.end()])
              for m in re.finditer(r'\S+', masked[a:b])]
        bare=[w for w,mk in toks if "\x00" not in mk]
        if not bare: continue
        head=[w for w in bare if not w.startswith("-")]
        flags=" ".join(w for w in bare if w.startswith("-"))
        if re.match(r'^(sed|perl)$', bare[0]) and re.search(r'-\S*i', flags):
            for w in head[1:]: keep(w)
        elif bare[0] in ("cp","mv","install") and len(head)>=3: keep(head[-1])
    if INDETERMINATE.search(masked): mech.append("patch/git-apply")
    if bodies and INTERPRETER.search(masked): mech.append("interpreter-heredoc")
    if re.search(r'\b(python3?|perl|ruby|node)\b[^\n]*\s-c\b', masked): mech.append("interpreter-inline")
    return targets, mech, bodies

def classify_bash(cmd):
    """(class, basis) for a Bash candidate. Basis names the deciding evidence."""
    targets,mech,_=write_targets(cmd)
    rs=[t for t in targets if t.endswith(".rs")]
    if rs: return "rust_write", "write target "+", ".join(sorted(set(rs)))
    if mech:
        return "needs_adjudication", ("writer not interpreted here: "+", ".join(mech)
               +("; shell targets "+", ".join(targets) if targets else "; no shell write target"))
    if targets: return "not_rust_write", "write targets "+", ".join(targets)+"; none ends in .rs"
    return "needs_adjudication", "no write target recovered from the command"

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
                    results[c.get("tool_use_id")]="error" if c.get("is_error") else "ok"
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
    """Per-call sequence, classified write candidates, and total read calls."""
    seq,results=load(path); reads=0; cands=[]
    for i,c in enumerate(seq):
        for tid,(name,inp) in c["uses"].items():
            hit=False; txt=""; klass=None; basis=""
            if name in WRITE_TOOLS:
                fp=inp.get("file_path") or inp.get("notebook_path") or ""
                if fp.endswith(".rs"):
                    hit=True; txt=f"{name} -> {fp}"
                    klass,basis="rust_write",f"{name} file_path {fp}"
            elif name=="Bash":
                cmd=inp.get("command") or ""
                if ".rs" in cmd and WRITEISH.search(cmd):
                    hit=True; txt=cmd; klass,basis=classify_bash(cmd)
            if hit:
                cands.append({"call":i,"tool_use_id":tid,"tool":name,"ctx":c["ctx"],
                              "reads_before":reads,"status":results.get(tid,"unknown"),
                              "result_present":tid in results,"class":klass,"basis":basis,
                              "txt":txt,"ts":c["ts"]})
            if name in READ_TOOLS: reads+=1
            elif name=="Bash" and BASH_READ.search(inp.get("command") or ""): reads+=1
    return seq,cands,reads

def first_effective(cands):
    """First candidate that both executed and determinately wrote Rust.

    Candidates before it are returned as the rejection chain, so the selection
    is checkable against the reasons rather than asserted from order.
    """
    for n,c in enumerate(cands):
        if c["status"]=="ok" and c["class"]=="rust_write": return c,cands[:n]
    return None,cands

if __name__=="__main__":
    out={}
    for p in sys.argv[1:]:
        seq,cands,reads=scan(p)
        tot=sum(c["wt"] for c in seq)
        aid=os.path.basename(p).replace("agent-","").replace(".jsonl","")
        f,rejected=first_effective(cands)
        unknown=[c for c in cands if c["status"]=="unknown"]
        pending=[c for c in cands if c["class"]=="needs_adjudication"]
        print(f"{aid} calls={len(seq):3d} Mwt={tot/1e6:.3f} maxctx={max((c['ctx'] for c in seq),default=0):7d} "
              f"reads_total={reads:3d} cands={len(cands):2d} unknown={len(unknown)} pending={len(pending)} "
              f"FIRST_RS ctx={f['ctx'] if f else None} reads={f['reads_before'] if f else None} call#{f['call'] if f else None}")
        for c in rejected:
            print(f"      rejected call#{c['call']} {c['tool_use_id']} status={c['status']} {c['class']}: {c['basis']}")
        if pending:
            print(f"      {len(pending)} candidate(s) need adjudication by reading the command; order is not evidence")
        out[aid]={"calls":len(seq),"weighted":tot,"max_ctx":max((c['ctx'] for c in seq),default=0),
                  "reads_total":reads,"first_rs_ctx":f["ctx"] if f else None,
                  "pre_write_reads":f["reads_before"] if f else None,
                  "first_rs_text":(f["txt"][:200] if f else None),
                  "first_rs_tool_use_id":f["tool_use_id"] if f else None,
                  "candidates":len(cands),"unknown_result":len(unknown),"needs_adjudication":len(pending),
                  "first_ts":seq[0]["ts"] if seq else None,"last_ts":seq[-1]["ts"] if seq else None}
    if os.environ.get("OUT"): json.dump(out,open(os.environ["OUT"],"w"),indent=1)

# Added 2026-10-24: the ramp decomposition quoted in the R2 score discussion is
# reproducible from this module. For each builder, scan() returns the per-call
# (uncached, cached, tool uses) sequence; the "ramp" is calls 0..first .rs write,
# adjudicated by reading the matched command rather than trusting the detector.
# Astra has not independently reproduced the decomposition and declined to
# certify it as a baseline; this script is published so it can be checked or
# refuted rather than argued about.
#
# Repaired 2026-10-24 against peer audit 2765 (ramp-instrument-audit-v1.json,
# bound to source sha256 457a2a1d). BR01: an absent tool_result was `None` and
# `not None` accepted it as an effective write; status is now a tri-state and
# `unknown` is never success. BR02: `.rs` anywhere plus a write token anywhere
# matched a command that only read Rust and redirected elsewhere; candidates are
# now classified on the recovered write target, with heredoc bodies treated as
# data rather than shell. The adjudication ledger over the historical R1 and R2
# builders is a separate artifact and does not depend on this revision.
