import pickle,sys,collections
ev=pickle.load(open(sys.argv[1],"rb"))
MULT={"plus":1,"prolite":5,"pro":10}
day=collections.defaultdict(lambda:dict(tok=collections.Counter(),fam=collections.Counter(),q=0.0,plan=collections.Counter(),cwd=collections.Counter()))
winmax={}
for t,model,eff,cwd,d,u,r,plan in ev:
    D=day[t[:10]]
    if d:
        for k in ("input_tokens","cached_input_tokens","output_tokens","reasoning_output_tokens","total_tokens"): D["tok"][k]+=d.get(k,0)
        fam="gpt-6" if model and model.startswith("gpt-6") else ("5.6-sol" if model and "5.6" in model else str(model))
        D["fam"][fam]+=d.get("total_tokens",0); D["cwd"][cwd]+=d.get("total_tokens",0)
    if plan: D["plan"][plan]+=1
    if u is not None and r:
        k=round(r/3600); m=winmax.get(k,0)
        if u>m: D["q"]+=(u-m)*MULT.get(plan,1); winmax[k]=u
rows=[]
for k in sorted(day):
    D=day[k]; T=D["tok"]
    if D["q"]<5 or T["total_tokens"]<1e7: continue
    top,share=max(D["fam"].items(),key=lambda x:x[1]); share/=T["total_tokens"]
    unc=T["input_tokens"]-T["cached_input_tokens"]
    rows.append((k,D["plan"].most_common(1)[0][0],top,share,D["q"],T["total_tokens"],unc,T["cached_input_tokens"],T["output_tokens"]))
    print(f"{k} {rows[-1][1]:8} {top:8} {share*100:4.0f}%  plus-units={D['q']:6.0f}  Mtok/unit={T['total_tokens']/D['q']/1e6:5.2f} uncachedMtok/unit={unc/D['q']/1e6:5.3f} outKtok/unit={T['output_tokens']/D['q']/1e3:6.1f}")
pickle.dump(rows,open(sys.argv[2],"wb"))
print("\nSummary for days where one family >=85% of tokens (units = % of a 1x Plus week):")
for fam in ("5.6-sol","gpt-6"):
  for plan in ("prolite","pro"):
    R=[r for r in rows if r[2]==fam and r[3]>=.85 and r[1]==plan]
    if not R: continue
    q=sum(r[4] for r in R)
    print(f"  {fam:8} on {plan:8}: {len(R):2} days, total Mtok/unit={sum(r[5] for r in R)/q/1e6:.3f}, uncached-in Mtok/unit={sum(r[6] for r in R)/q/1e6:.4f}, output Ktok/unit={sum(r[8] for r in R)/q/1e3:.1f}")
top=collections.Counter()
for D in day.values(): top.update(D["cwd"])
print("\nTop cwds by tokens:"); [print(f"  {v/1e9:.2f}B {c}") for c,v in top.most_common(8)]
