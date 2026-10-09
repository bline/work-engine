import pickle,sys,collections,datetime as dt
ev=pickle.load(open(sys.argv[1],"rb"))
W=lambda t:dt.date.fromisoformat(t[:10]).isocalendar()[:2]
wk=collections.defaultdict(lambda:dict(tok=collections.Counter(),models=collections.Counter(),plans=collections.Counter(),quota=0.0,first=None))
winmax={}  # resets_at bucket -> max used seen
for t,model,eff,cwd,d,u,r,plan in ev:
    w=wk[W(t)]
    if w["first"] is None: w["first"]=t[:10]
    if d:
        for k in ("input_tokens","cached_input_tokens","output_tokens","reasoning_output_tokens","total_tokens"): w["tok"][k]+=d.get(k,0)
        w["models"][f"{model}/{eff}"]+=d.get("total_tokens",0)
    if plan: w["plans"][plan]+=1
    if u is not None and r:
        k=round(r/3600)
        m=winmax.get(k,0)
        if u>m: w["quota"]+=u-m; winmax[k]=u
print(f"{'week':8} {'start':10} {'plan':8} {'quota%':>6} {'total Mtok':>10} {'uncached in':>11} {'out':>7} {'reason':>7} {'Mtok/1%':>8}  top models")
for k in sorted(wk):
    w=wk[k]; T=w["tok"]; tot=T["total_tokens"]/1e6
    if tot<1: continue
    plan=w["plans"].most_common(1)[0][0] if w["plans"] else "-"
    mods=", ".join(f"{m} {v/T['total_tokens']*100:.0f}%" for m,v in w["models"].most_common(3))
    unc=(T["input_tokens"]-T["cached_input_tokens"])/1e6
    print(f"{k[0]}-W{k[1]:02} {w['first']} {plan:8} {w['quota']:6.0f} {tot:10.1f} {unc:11.1f} {T['output_tokens']/1e6:7.2f} {T['reasoning_output_tokens']/1e6:7.2f} {(tot/w['quota'] if w['quota'] else 0):8.2f}  {mods}")
