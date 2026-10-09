import pickle,collections,numpy as np
ev=pickle.load(open(__import__("sys").argv[1],"rb"))
MULT={"plus":1,"prolite":5,"pro":10}
day=collections.defaultdict(lambda:[0.0,collections.Counter()]); winmax={}
for t,model,eff,cwd,d,u,r,plan in ev:
    D=day[t[:10]]
    if d and model:
        fam="g6" if model.startswith("gpt-6") else ("s56" if "5.6" in model else None)
        if fam:
            D[1][fam+"_unc"]+=d.get("input_tokens",0)-d.get("cached_input_tokens",0)
            D[1][fam+"_cac"]+=d.get("cached_input_tokens",0); D[1][fam+"_out"]+=d.get("output_tokens",0)
    if u is not None and r:
        k=round(r/3600); m=winmax.get(k,0)
        if u>m: D[0]+=(u-m)*MULT.get(plan,1); winmax[k]=u
X=[];y=[]
for k,(q,c) in day.items():
    if q<5 or k<"2026-08-06": continue
    X.append([c["s56_unc"]+c["s56_cac"]/10+c["s56_out"]*5, c["g6_unc"]+c["g6_cac"]/10+c["g6_out"]*5]); y.append(q)
X=np.array(X)/1e6;y=np.array(y)
coef,res,_,_=np.linalg.lstsq(X,y,rcond=None)
print("Plus-units per M 'weighted tokens' (uncached + cached/10 + 5*output):  5.6-sol=%.3f  gpt-6=%.3f  ratio gpt6/5.6=%.2f"%(coef[0],coef[1],coef[1]/coef[0]))
pred=X@coef; print("R^2=%.2f n=%d"%(1-((y-pred)**2).sum()/((y-y.mean())**2).sum(),len(y)))
