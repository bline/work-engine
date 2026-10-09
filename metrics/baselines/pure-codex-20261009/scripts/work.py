import pickle,collections,sys
rows=pickle.load(open(sys.argv[1],"rb")); q={r[0]:r for r in rows}
g=collections.defaultdict(collections.Counter)
for line in open(sys.argv[2]):
    p=line.split()
    if p[0]=="commit": g[p[2]]["commits"]+=1
    else: g[p[2]][p[3]+"+"]+=int(p[4])
eras={"5.6-sol / Pro Lite (Aug13-Oct1)":("2026-08-13","2026-10-01"),"gpt-6 / Pro (Oct3-Oct8)":("2026-10-03","2026-10-08"),"  gpt-6 Rust days only (Oct7-8)":("2026-10-07","2026-10-08")}
for name,(a,b) in eras.items():
    days=[d for d in q if a<=d<=b]; U=sum(q[d][4] for d in days)
    G=collections.Counter()
    for d in g:
        if a<=d<=b: G.update(g[d])
    tok=sum(q[d][5] for d in days)
    print(f"{name}: {len(days)} active days, {U:.0f} Plus-units, {tok/1e9:.2f}B tokens, commits={G['commits']}, code+={G['code+']}, docs+={G['docs+']}")
    print(f"     per 100 Plus-units (=one Plus week): commits {G['commits']/U*100:.1f}, code lines {G['code+']/U*100:.0f}, doc lines {G['docs+']/U*100:.0f};  per 1B tokens: code lines {G['code+']/tok*1e9:.0f}")
