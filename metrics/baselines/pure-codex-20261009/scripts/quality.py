import os,re,json,glob,collections
B=os.path.expanduser("~/.local/state/work-engine")
dirs=sorted(d for d in glob.glob(B+"/rust-*")+glob.glob(B+"/ui-s5-populated-20261008-*") if os.path.isdir(d))
def walk(d):
    for root,ds,fs in os.walk(d):
        ds[:]=[x for x in ds if x not in ("target",".git","node_modules","source","worktree","repo")]
        if root.count(os.sep)-d.count(os.sep)>3: ds[:]=[]
        for f in fs: yield os.path.join(root,f)
def findings(o,acc):
    if isinstance(o,dict):
        for k,v in o.items():
            kl=k.lower()
            if kl in ("findings","review_findings","material_findings") and isinstance(v,list): acc["lists"]+=1; acc["items"]+=len(v)
            elif kl in ("finding_count","findings_count","material_finding_count","total_findings") and isinstance(v,int): acc["counts"]+=v
            else: findings(v,acc)
    elif isinstance(o,list):
        for x in o: findings(x,acc)
rows=[]
for d in dirs:
    c=collections.Counter(); names=[]
    for p in walk(d):
        n=os.path.basename(p); names.append(n); c["files"]+=1
        m=re.search(r"candidate[-_]?(\d+)",n,re.I)
        if m: c["max_candidate"]=max(c["max_candidate"],int(m.group(1)))
        m=re.search(r"\.v(\d+)\.",n)
        if m: c["max_version"]=max(c["max_version"],int(m.group(1)))
        if re.search(r"fail|invalid|diagnostic|stale|abort|repair",n,re.I): c["failure_named_files"]+=1
        if n.endswith((".log",".tap",".txt")) and os.path.getsize(p)<50e6:
            try: t=open(p,errors="ignore").read()
            except: continue
            if re.search(r"test result: \w+\. \d+ passed",t) or "# pass" in t or "error[E" in t:
                c["test_logs"]+=1
                if "test result: FAILED" in t or "error[E" in t or re.search(r"^# fail [1-9]",t,re.M) or "could not compile" in t: c["failed_test_logs"]+=1
        if n.endswith(".json") and os.path.getsize(p)<5e6:
            try: o=json.load(open(p))
            except: continue
            acc=collections.Counter(); findings(o,acc)
            if acc["lists"] or acc["counts"]: c["review_files"]+=1; c["finding_items"]+=acc["items"]; c["finding_counts"]+=acc["counts"]
            s=json.dumps(o)[:200000].lower()
            if '"verdict": "reject' in s or '"outcome": "rejected' in s or '"status": "rejected' in s: c["rejected_records"]+=1
    rows.append((os.path.basename(d),c))
for n,c in rows:
    print(f"{n:52} files={c['files']:4} cand={c['max_candidate']:2} maxver={c['max_version']:2} failnamed={c['failure_named_files']:3} testlogs={c['test_logs']:3} failedlogs={c['failed_test_logs']:3} reviewfiles={c['review_files']:3} findingitems={c['finding_items']:3} rejected={c['rejected_records']}")
