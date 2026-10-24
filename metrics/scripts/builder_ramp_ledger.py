"""Persisted first-.rs-write adjudication ledger for the R1 and R2 builders.

Answers peer audit 2765: publish call id, tool_use id, result presence and
status, and the decision basis, for every candidate up to and including the
adjudicated first effective write, for all fifteen builders.

`builder_ramp.scan` classifies each candidate from the write target it can
recover. A candidate it will not interpret -- a python heredoc, an inline
interpreter, patch, git apply -- is `needs_adjudication` and is decided here by
a recorded human reading of the command, keyed by exact tool_use id. Candidate
order decides nothing: a decision is required for every unadjudicated candidate
that precedes the selection, and the run fails if one is missing.
"""
import json, os, sys
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import builder_ramp as B

SUB = "/home/bline/.claude/projects/-home-bline-code-work-engine/%s/subagents/agent-%s.jsonl"
R1_ROOT = "bf60b4c1-34d1-4900-bbe0-15042efb0e2c"
R2_ROOT = "c1d31e88-2169-456f-8673-e29f643e9fb2"
COHORT = [
    ("R1", "a27d1ec9c1e4acca4", "s1"),      ("R1", "a74b04b4ca3160232", "s2_b2"),
    ("R1", "abeb3da4543a8545f", "s1"),      ("R1", "a68b863f021a779a8", "s2_a"),
    ("R1", "aed2ceafc33b90035", "s2_a2"),   ("R1", "a5d3ffe262a707e38", "s2_b"),
    ("R2", "a74f07e00db29f8e0", "E1"),      ("R2", "a68d60a23bab394e2", "E1"),
    ("R2", "a4753525221f23204", "E2"),      ("R2", "aa3038194d5ecde25", "E2"),
    ("R2", "a69f21553464cbc6e", "E2"),      ("R2", "a8b36f8b30274b385", "P1"),
    ("R2", "ac604a3dcfdd0db66", "P1"),      ("R2", "a131e8667687eea02", "P1"),
    ("R2", "aa725a9ba777a9a84", "P1"),
]

# Recorded decisions, keyed by exact tool_use id, for candidates the detector
# will not interpret. Each basis is what was read in the command itself.
ADJUDICATED = {
 "toolu_01Kvbvhpz4cczkXE8ZXtCHtK": ("not_rust_write",
   "python heredoc; its only write is open('.../part-b/mutation/mutate.py','w').write(...). "
   "The shell redirects are > /tmp/x and 2>/dev/null. src/check.rs, src/cli.rs and src/lib.rs "
   "appear only as data inside the mutants table the script emits."),
 "toolu_01S8tGy7A4uCUtDmjTBvsniH": ("not_rust_write",
   "writes $R/result-b-v1.md via cat and $R/result-b-v1.json via json.dump(open(...,'w')). "
   "tests/rederive.rs is named in the report prose, not written."),
 "toolu_01Esi6VZxS5SMtpQVsLXQC1K": ("rust_write",
   "python heredoc; p=sys.argv[1] and argv[1] is $T=.../decision-readiness/tests/rederive.rs; "
   "final statement is open(p,'w').write(t). This is the first effective Rust write."),
 "toolu_01MUbgNssj7u7wPHULwDhXsn": ("not_rust_write",
   "git add + git commit of tests/rederive.rs. It records the content written at the previous "
   "candidate; it writes no source itself."),
 "toolu_01Q5WqccrBygeqaiBYHQMfWE": ("not_rust_write",
   "writes $R/result-b-v2.md and $R/result-b-v2.json; the .rs paths are report prose."),
 "toolu_01XtPNfZHN7kJruYRrHL2Acv": ("rust_write",
   "python heredoc; p='src/plan.rs', t=open(p).read(), string replacements, final statement "
   "open(p,'w').write(t). This is the first effective Rust write."),
 "toolu_016q3CcDRNNK9KFZjE7JguQE": ("rust_write",
   "python heredoc; p='src/plan.rs', final statement open(p,'w').write(t). Follows the first "
   "effective write and cannot change it; recorded for completeness."),
 "toolu_01Cp3FTc8hBYLGsTV3JAo8AY": ("not_rust_write",
   "ls -R, cat Cargo.toml, wc -l src/*.rs, sha256sum and grep -n over src/*.rs. Every clause "
   "reads; the .rs paths are arguments to read commands."),
}

def ledger():
    rows=[]; missing=[]; totals={"candidates":0,"unknown_result":0,"error_result":0}
    for arm,aid,slice_ in COHORT:
        path=SUB%(R1_ROOT if arm=="R1" else R2_ROOT, aid)
        seq,cands,reads=B.scan(path)
        totals["candidates"]+=len(cands)
        totals["unknown_result"]+=sum(1 for c in cands if c["status"]=="unknown")
        totals["error_result"]+=sum(1 for c in cands if c["status"]=="error")
        chain=[]; selected=None
        for c in cands:
            klass,basis,how=c["class"],c["basis"],"detector"
            if c["tool_use_id"] in ADJUDICATED:
                klass,basis=ADJUDICATED[c["tool_use_id"]]; how="read"
            elif klass=="needs_adjudication" and selected is None:
                missing.append((aid,c["call"],c["tool_use_id"]))
            entry={"call":c["call"],"tool_use_id":c["tool_use_id"],"tool":c["tool"],
                   "ctx":c["ctx"],"reads_before":c["reads_before"],
                   "result_present":c["result_present"],"status":c["status"],
                   "class":klass,"decided_by":how,"basis":basis,"command":c["txt"][:220]}
            chain.append(entry)
            if selected is None and c["status"]=="ok" and klass=="rust_write": selected=entry
            if selected is not None and entry is not selected and entry["call"]>selected["call"]:
                chain.pop(); break
        # every candidate sharing the selected call, for the multi-tool_use case
        same=[e for e in chain if selected and e["call"]==selected["call"]]
        rows.append({"arm":arm,"agent":aid,"slice":slice_,"transcript":path,
                     "calls":len(seq),"reads_total":reads,"candidates_total":len(cands),
                     "first_rs_ctx":selected["ctx"] if selected else None,
                     "pre_write_reads":selected["reads_before"] if selected else None,
                     "selected":selected,
                     "selection_independent_of_choice": len([e for e in same
                        if e["status"]=="ok" and e["class"]=="rust_write"])>1,
                     "chain":chain})
    return rows,missing,totals

if __name__=="__main__":
    rows,missing,totals=ledger()
    if missing:
        for aid,call,tid in missing:
            print(f"UNADJUDICATED before selection: {aid} call#{call} {tid}", file=sys.stderr)
        sys.exit(1)
    doc={"artifact":"builder first-.rs-write adjudication ledger",
         "answers":"peer audit 2765 / ramp-instrument-audit-v1.json",
         "instrument":"metrics/scripts/builder_ramp.py",
         "builders":len(rows),"totals":totals,"rows":rows}
    out=os.environ.get("OUT")
    if out: json.dump(doc,open(out,"w"),indent=1)
    for r in rows:
        s=r["selected"]
        print(f"{r['arm']} {r['agent']} {r['slice']:>5} cands={r['candidates_total']:2d} "
              f"chain={len(r['chain']):2d} ctx={s['ctx'] if s else None} reads={s['reads_before'] if s else None} "
              f"tid={s['tool_use_id'] if s else None} by={s['decided_by'] if s else None}")
    print(f"\ncandidates={totals['candidates']} unknown_result={totals['unknown_result']} "
          f"error_result={totals['error_result']} builders={len(rows)}")
