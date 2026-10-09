import pickle,collections,datetime as dt,re,sys
ev=pickle.load(open(sys.argv[1],"rb"))
MULT={"plus":1,"prolite":5,"pro":10}
wk=lambda t:"%d-W%02d"%dt.date.fromisoformat(t[:10]).isocalendar()[:2]
day=collections.defaultdict(lambda:[0.0,collections.Counter(),collections.Counter()]); winmax={}
for t,model,eff,cwd,d,u,r,plan in ev:
    D=day[t[:10]]
    if d:
        price=1.79 if (model or "").startswith("gpt-6") else 1.0   # relative quota price from regression
        w=(d.get("input_tokens",0)-d.get("cached_input_tokens",0)+d.get("cached_input_tokens",0)/10+5*d.get("output_tokens",0))*price
        repo="work-engine" if cwd and "work-engine" in cwd else "other"
        D[1][repo]+=w; D[2][(repo,(model or "?").split("-")[1] if model else "?")]+=d.get("total_tokens",0)
    if u is not None and r:
        k=round(r/3600); m=winmax.get(k,0)
        if u>m: D[0]+=(u-m)*MULT.get(plan,1); winmax[k]=u
WU=collections.Counter(); WT=collections.Counter(); WM=collections.defaultdict(collections.Counter)
for k,(q,sh,tk) in day.items():
    tot=sum(sh.values())
    if tot: WU[wk(k)]+=q*sh["work-engine"]/tot
    for (repo,m),v in tk.items():
        if repo=="work-engine": WT[wk(k)]+=v; WM[wk(k)][m]+=v
L=collections.defaultdict(collections.Counter); w=None
for l in open(sys.argv[2]):
    if l.startswith("C "): w=l.split()[1]; L[w]["commits"]+=1; continue
    p=l.rstrip("\n").split("\t")
    if len(p)!=3 or p[0]=="-": continue
    f=p[2]
    if re.search(r"(lock|fixtures?|generated|previews|\.jsonl$)",f): continue
    k="code" if re.search(r"\.(rs|mjs|js|ts|py|sql|sh)$",f) else ("md" if f.endswith(".md") else None)
    if k: L[w][k]+=int(p[0])
    if k=="code" and re.search(r"(^|/)tests?/|_test\.|\.test\.|test_",f): L[w]["test"]+=int(p[0])
print("%-9s %7s %8s %10s %8s %6s %10s %10s  %s"%("week","units","Btok","code+","test%","md+","code/unit","md/unit","models(we tokens)"))
for w in sorted(set(WU)|set(L)):
    if w<"2026-W32": continue
    U=WU[w]; C=L[w]
    m=", ".join("%s %.0f%%"%(a,b/WT[w]*100) for a,b in WM[w].most_common(2)) if WT[w] else ""
    print("%-9s %7.0f %8.2f %10d %7.0f%% %6d %10s %10s  %s"%(w,U,WT[w]/1e9,C["code"],100*C["test"]/C["code"] if C["code"] else 0,C["md"],"%.0f"%(C["code"]/U) if U>20 else "-","%.0f"%(C["md"]/U) if U>20 else "-",m))
def era(a,b,name):
    ws=[w for w in WU if a<=w<=b]; U=sum(WU[w] for w in ws); C=sum((L[w] for w in ws),collections.Counter())
    print(f"{name}: {U:.0f} units (work-engine share), code+ {C['code']} ({100*C['test']/C['code']:.0f}% tests), md+ {C['md']}, code/unit {C['code']/U:.0f}, (code+md)/unit {(C['code']+C['md'])/U:.0f}")
print()
era("2026-W34","2026-W37","Python/Node build-out, 5.6-sol (W34-W37, Aug17-Sep13)")
era("2026-W38","2026-W40","Node migration/campaigns, mostly 5.6-sol (W38-W40)")
era("2026-W41","2026-W41","Rust, gpt-6 (W41, Oct5-9)")
