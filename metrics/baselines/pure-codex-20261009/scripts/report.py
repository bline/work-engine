import pickle,collections,re
R=pickle.load(open("SCR/act.pkl".replace("SCR",__import__("os").environ["S"]),"rb"))
NODE={"/root/completion_boundary_gpt6_medium_builder"}
NODE_ROOTS={"01a088a4","01a0eb80"}
def role(p):
    if p is None: return "orchestrator (root sessions)"
    n=p.split("/")[-1]
    if re.search(r"ui_|palette",p): return "UI"
    if re.search(r"review|candidate_|contract_rev|agent_instru|placement_fa",n): return "review"
    if re.search(r"plan|design|recon|explanation|diagnosis|characterization",n): return "planning/diagnosis"
    return "build/integrate"
SL=[("S8 (in progress)",r"s8_"),("S6/S7",r"s67|s6_|s7_|rust_lifecycle_s[67]"),("S5",r"_s5"),("S4",r"_s4|rust_lifecycle_offline"),("S0-S3 + early lifecycle",r"rust_lifecycle_s[0-3]|rust_s0|rust_lifecycle_(design|implementation)|rust_native_lifecycle|rust_live_admission|codex_adapter"),
    ("Compiler C1-C3",r"compiler|/c[123]_|c2_|c3_"),("Review episode R0-R3",r"episode_r0|r0_review|/r[23]_|review_episode_plan"),("Host HP1/HP2 + bridge",r"host|hp1|hp2|bridge_perf|generation_"),
    ("Claims/Campaign CE1-3, SC0-2",r"/ce[0-9]|/sc[0-9]"),("J1/E1/ER1/P1 + planning J2/HP3",r"j1|j2|e0_|evidence|er1|p1_|hp3|episode_join|history_perf"),("UI",r"ui_|palette")]
def slice_of(p,rid):
    if p is None: return "Orchestrator root sessions"
    if p in NODE: return "Node-era work (pre-Rust)"
    for n,rx in SL:
        if re.search(rx,p): return n
    return "Other ("+p+")"
by_role=collections.Counter(); by_act=collections.defaultdict(collections.Counter); by_slice=collections.defaultdict(collections.Counter); tot=collections.Counter(); other=collections.Counter()
for r in R:
    p=r["meta"].get("agent_path"); rid=str(r["meta"].get("id"))[:8]; a=r["act"]
    u=sum(v for k,v in a.items() if not k.startswith("_"))
    if p is None and rid in NODE_ROOTS: rl="Node-era root sessions"; sl="Node-era work (pre-Rust)"
    else: rl=role(p); sl=slice_of(p,rid)
    if p in NODE: rl="Node-era builder"
    by_role[rl]+=u; by_slice[sl][rl]+=u
    for k,v in a.items():
        if not k.startswith("_"): by_act[rl][k]+=v; tot[k]+=v
    if sl.startswith("Other"): other[p]+=u
T=sum(by_role.values())
print(f"TOTAL est units Oct5-9 (all codex): {T:.0f}\n\n== By role ==")
for k,v in by_role.most_common(): print(f"  {v:6.0f} {v/T*100:4.0f}%  {k}")
print("\n== By activity (all) ==")
for k,v in tot.most_common(): print(f"  {v:6.0f} {v/T*100:4.0f}%  {k}")
print("\n== Activity within role (%) ==")
acts=[k for k,_ in tot.most_common()]
print(" "*30+"".join(f"{a[:11]:>12}" for a in acts))
for rl,_ in by_role.most_common():
    s=sum(by_act[rl].values()); print(f"{rl[:29]:30}"+"".join(f"{by_act[rl][a]/s*100:11.0f}%" for a in acts))
print("\n== By slice ==")
for sl,c in sorted(by_slice.items(),key=lambda x:-sum(x[1].values())):
    s=sum(c.values()); print(f"  {s:6.0f}  {sl:38} "+", ".join(f"{k.split(' ')[0]} {v:.0f}" for k,v in c.most_common()))
if other: print("other:",other.most_common())
