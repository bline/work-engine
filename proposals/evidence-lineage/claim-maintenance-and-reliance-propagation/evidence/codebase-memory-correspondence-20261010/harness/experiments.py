"""Six correspondence experiments against the real codebase-memory build.

Each experiment builds its own git repo, declares anchors into a stand-in
canonical store, applies controlled transitions, and records raw observations,
the five-state outcome, and what we expected. Negative findings are recorded
as findings, not failures to hide.
"""
import json
import os
import shutil
import subprocess
import sys
import textwrap
import time

import cbm
import observer as ob

ROOT = os.path.join(cbm.SP, "exp")
RESULTS = []
FINDINGS = []

LEDGER = '''\
"""Ledger module."""


def post_entry(ledger, amount):
    if amount == 0:
        raise ValueError("zero")
    ledger.append(amount)
    return balance(ledger)


def balance(ledger):
    return sum(ledger)


class Account:
    def close(self):
        return balance([])
'''
ROUNDING = '''\
def round_half_even(x):
    return round(x)


def clamp(x, lo, hi):
    return max(lo, min(hi, x))
'''
MAIN = '''\
from ledger import post_entry
from util.rounding import round_half_even


def run():
    return post_entry([], round_half_even(3.5))
'''
DESIGN = '''\
# Ledger design

## Posting rules

A posting never accepts a zero amount. The rule is enforced in
`src/ledger.py::post_entry` and the running total comes from
`src/ledger.py::balance`.

## Rounding

Amounts are rounded half-to-even by `src/util/rounding.py::round_half_even`.
'''


# ── helpers ────────────────────────────────────────────────────────────────
def sh(repo, *cmd):
    subprocess.run(cmd, cwd=repo, check=True, capture_output=True)


def write(repo, rel, text):
    p = os.path.join(repo, rel)
    os.makedirs(os.path.dirname(p), exist_ok=True)
    with open(p, "w") as f:
        f.write(text)


def make_repo(name, files):
    repo = os.path.join(ROOT, name)
    shutil.rmtree(repo, ignore_errors=True)
    os.makedirs(repo)
    sh(repo, "git", "init", "-q")
    sh(repo, "git", "config", "user.email", "t@t")
    sh(repo, "git", "config", "user.name", "t")
    for rel, text in files.items():
        write(repo, rel, text)
    sh(repo, "git", "add", "-A")
    sh(repo, "git", "commit", "-qm", "init")
    project = cbm.index(repo)["project"]
    return repo, project


def canonical(repo, claims):
    path = os.path.join(ROOT, os.path.basename(repo) + ".canonical.json")
    with open(path, "w") as f:
        json.dump({"revision": 1, "claims": claims}, f)
    return ob.Canonical(path)


def code_anchor(aid, path, symbol, label):
    return {"id": aid, "kind": "code_structure",
            "locator": {"path": path, "symbol": symbol, "label": label}}


def text_anchor(aid, path, start, end):
    return {"id": aid, "kind": "text", "locator": {"path": path, "start": start, "end": end}}


def declare(project, repo, can, claim_id, anchors):
    data = can.load()
    bound = []
    for a in anchors:
        if a["kind"] == "code_structure":
            res, why = ob.declare_code_anchor(project, a)
        elif a["kind"] == "text":
            res, why = ob.declare_text_anchor(repo, a)
        else:
            res, why = a, "declared (unsupported kind, no binding)"
        bound.append((a["id"], why))
        if res is not None:
            data["claims"].setdefault(claim_id, {"anchors": []})["anchors"].append(res)
    with open(can.path, "w") as f:
        json.dump(data, f, indent=1)
    return bound


def record(exp, case, obs, expected, note=""):
    ok = obs["state"] == expected if isinstance(expected, str) else obs["state"] in expected
    RESULTS.append({"exp": exp, "case": case, "state": obs["state"], "reason": obs["reason"],
                    "expected": expected, "as_expected": ok, "note": note,
                    "successors": obs["raw"].get("successors"), "attempts": obs["attempts"]})
    mark = "ok " if ok else "XX "
    print(f"  {mark}{exp} {case}: {obs['state']} ({obs['reason']}) expected={expected}"
          + (f" successors={[(s['qn'].rsplit('.', 3)[-3:], s["body_digest_equal"], s.get("tail_digest_equal")) for s in obs['raw'].get('successors') or []]}" if obs['raw'].get('successors') else ""))


def finding(exp, kind, text, evidence=None):
    FINDINGS.append({"exp": exp, "kind": kind, "text": text, "evidence": evidence})
    print(f"  [{kind}] {text}")


def base_files():
    return {"src/ledger.py": LEDGER, "src/util/rounding.py": ROUNDING,
            "src/main.py": MAIN, "docs/design.md": DESIGN}


# ── E1 exact anchor resolution ─────────────────────────────────────────────
def e1():
    print("E1 exact anchor resolution")
    repo, P = make_repo("e1", base_files())
    can = canonical(repo, {})
    b = declare(P, repo, can, "CLM-1", [
        code_anchor("fn", "src/ledger.py", "post_entry", "Function"),
        code_anchor("method", "src/ledger.py", "Account.close", "Method"),
        code_anchor("missing_member", "src/ledger.py", "no_such_fn", "Function"),
        code_anchor("kind_mismatch", "src/ledger.py", "close", "Function"),
        code_anchor("owner_mismatch", "src/ledger.py", "Ledger.close", "Method"),
        {"id": "cfg", "kind": "config_key", "locator": {"path": "pyproject.toml", "key": "x"}},
    ])
    print("  declarations:", b)
    refused = {aid for aid, why in b if why.startswith("refused")}
    expect_refused = {"missing_member", "kind_mismatch", "owner_mismatch"}
    if refused == expect_refused:
        finding("E1", "positive", "strict declaration refused every non-exact locator "
                "(missing member, wrong kind, wrong owner) instead of falling back to the file",
                b)
    else:
        finding("E1", "NEGATIVE", f"declaration refusals {refused} != {expect_refused}", b)
    for aid, exp in (("fn", "matches"), ("method", "matches"), ("cfg", "unsupported")):
        record("E1", aid, ob.observe(P, repo, can, aid), exp)
    # contrast: the same locator text as a doc reference
    write(repo, "docs/contrast.md", "# Contrast\n\nSee `src/ledger.py::no_such_fn`.\n")
    cbm.index(repo)
    rows = cbm.query(P, 'MATCH (s)-[r:MENTIONS]->(t) WHERE s.file_path = "docs/contrast.md" '
                        'RETURN t.label, t.name')
    finding("E1", "NEGATIVE", "doc reference to a nonexistent member resolves as a MENTIONS edge to "
            f"the containing file, with no record of the requested member: {rows}", rows)


# ── E2 rename, move, uncertain coverage ───────────────────────────────────
def e2():
    print("E2 rename / move / uncertain coverage")
    repo, P = make_repo("e2", base_files())
    can = canonical(repo, {})
    declare(P, repo, can, "CLM-2", [code_anchor("clamp", "src/util/rounding.py", "clamp", "Function")])
    record("E2", "baseline", ob.observe(P, repo, can, "clamp"), "matches")

    # a) worktree rename, NOT yet reindexed: the index no longer describes the requested subject
    write(repo, "src/util/rounding.py", ROUNDING.replace("def clamp(", "def clamp_value("))
    record("E2", "rename_before_reindex", ob.observe(P, repo, can, "clamp"), "unknown")
    # b) after reindex: absence established, successor offered but not bound
    cbm.index(repo)
    o = ob.observe(P, repo, can, "clamp")
    record("E2", "rename_after_reindex", o, "differs")

    # c) directory move
    sh(repo, "git", "checkout", "-q", "--", ".")
    sh(repo, "git", "mv", "src/util", "src/helpers")
    cbm.index(repo)
    record("E2", "directory_move", ob.observe(P, repo, can, "clamp"), "differs")
    sh(repo, "git", "reset", "-q", "--hard")
    sh(repo, "git", "clean", "-qfd")

    # d) parse damage inside the anchored function
    broken = ROUNDING.replace("    return max(lo, min(hi, x))", "    return max(lo, min(hi, x)\n    if if if")
    write(repo, "src/util/rounding.py", broken)
    cbm.index(repo)
    o = ob.observe(P, repo, can, "clamp")
    record("E2", "parse_damage", o, ("unknown", "differs"),
           "unknown if coverage reports the damage; differs only if body truly changed and is visible")
    cov = o["raw"].get("coverage", {})
    finding("E2", "observation", f"parse damage: coverage issues={cov.get('issues')} hits={o['raw'].get('hits')}")
    sh(repo, "git", "checkout", "-q", "--", ".")

    # e) file becomes ignored (excluded by design)
    write(repo, ".gitignore", "src/util/\n")
    cbm.index(repo)
    o = ob.observe(P, repo, can, "clamp")
    record("E2", "file_now_ignored", o, "unknown",
           "file exists but is excluded: absence in the graph is not absence in the world")
    os.remove(os.path.join(repo, ".gitignore"))
    cbm.index(repo)


# ── E3 bidirectional impact ───────────────────────────────────────────────
def e3():
    print("E3 bidirectional impact")
    files = base_files()
    files["docs/repeated.md"] = textwrap.dedent('''\
        # Service A

        ## Implementation

        Uses `src/ledger.py::balance`.

        # Service B

        ## Implementation

        Uses `src/ledger.py::post_entry`.
        ''')
    repo, P = make_repo("e3", files)
    can = canonical(repo, {})
    declare(P, repo, can, "CLM-3", [
        code_anchor("balance", "src/ledger.py", "balance", "Function"),
        code_anchor("post_entry", "src/ledger.py", "post_entry", "Function"),
        text_anchor("posting_rules", "docs/design.md", 3, 7),
        text_anchor("svcA_impl", "docs/repeated.md", 3, 5),
        text_anchor("svcB_impl", "docs/repeated.md", 9, 11),
    ])
    anchors = {a["id"]: a for c in can.load()["claims"].values() for a in c["anchors"]}

    # code -> claims/docs
    write(repo, "src/ledger.py", LEDGER.replace("return sum(ledger)", "return sum(ledger) + 0"))
    cbm.index(repo)
    record("E3", "code_change:anchor_balance", ob.observe(P, repo, can, "balance"), "differs")
    record("E3", "code_change:anchor_post_entry", ob.observe(P, repo, can, "post_entry"), "matches",
           "the caller's own body did not change: mechanical anchor comparison stays matches")
    dc = cbm.detect_changes(P, edge_types=["CALLS", "MENTIONS"], fixpoint=True, via=True)
    impacted = {i.get("qn") or i.get("qualified_name"): i for i in dc.get("impacted", [])}
    bound_qns = {a["bound"]["qn"]: aid for aid, a in anchors.items() if a["kind"] == "code_structure"}
    reach = [(bound_qns[qn], impacted[qn].get("hop")) for qn in impacted if qn in bound_qns]
    sections = sorted(q.rsplit(".", 1)[-1] for q, i in impacted.items() if i.get("label") == "Section")
    finding("E3", "observation", f"discovery (not authority): anchors reachable by blast radius {reach}; "
            f"doc sections reached via MENTIONS {sections}", {"reach": reach, "sections": sections})
    sh(repo, "git", "checkout", "-q", "--", ".")
    cbm.index(repo)

    # normative doc -> code
    write(repo, "docs/design.md", DESIGN.replace("never accepts a zero amount",
                                                  "never accepts a zero or negative amount"))
    cbm.index(repo)
    record("E3", "doc_change:posting_rules", ob.observe(P, repo, can, "posting_rules"), "differs")
    dc = cbm.detect_changes(P, edge_types=["MENTIONS"])
    finding("E3", "NEGATIVE", "detect_changes on a doc-only edit yields seeds="
            f"{dc.get('seed_total', dc.get('seed_symbols'))}, impacted={dc.get('impacted_total')}: "
            "Sections are never seeds, so doc->code needs the adapter's own section mapping")
    rows = cbm.query(P, 'MATCH (s:Section)-[:MENTIONS]->(c) WHERE s.file_path = "docs/design.md" '
                        'AND s.start_line <= 5 RETURN s.name, s.start_line, c.name ORDER BY s.start_line DESC')
    finding("E3", "observation", f"workaround (heading at/above changed line -> outbound MENTIONS): {rows}")
    sh(repo, "git", "checkout", "-q", "--", ".")

    # repeated headings: text anchors stay precise, MENTIONS attribution does not
    write(repo, "docs/repeated.md", files["docs/repeated.md"].replace("Uses `src/ledger.py::balance`.",
                                                                      "Uses only `src/ledger.py::balance`."))
    cbm.index(repo)
    record("E3", "repeated_heading:svcA", ob.observe(P, repo, can, "svcA_impl"), "differs")
    record("E3", "repeated_heading:svcB", ob.observe(P, repo, can, "svcB_impl"), "matches")
    rows = cbm.query(P, 'MATCH (s)-[:MENTIONS]->(c) WHERE s.file_path = "docs/repeated.md" '
                        'RETURN s.label, s.name, s.start_line, c.name')
    finding("E3", "NEGATIVE", f"repeated heading MENTIONS attribution: {rows}", rows)
    sh(repo, "git", "checkout", "-q", "--", ".")


# ── E4 concurrent revisions ───────────────────────────────────────────────
def e4():
    print("E4 concurrent revisions")
    repo, P = make_repo("e4", base_files())
    can = canonical(repo, {})
    declare(P, repo, can, "CLM-4", [code_anchor("balance", "src/ledger.py", "balance", "Function")])

    # a) what does the supported generation signal do across reindexes?
    trail = []
    def snap(label):
        cov = ob.coverage_facts(P, "src/ledger.py")
        trail.append({"step": label, "supported_generation": cov["generation"],
                      "recorded_at": cov["recorded_at"], "oracle": cbm.oracle_generation(P)})
    snap("start")
    cbm.index(repo); snap("noop_reindex")
    write(repo, "src/main.py", MAIN + "\n# touch\n"); cbm.index(repo); snap("content_reindex_1")
    write(repo, "src/main.py", MAIN + "\n# touch 2\n"); cbm.index(repo); snap("content_reindex_2_same_second?")
    for t in trail:
        print("   ", t["step"], t["supported_generation"], t["recorded_at"], t["oracle"].get("mutation_gen"))
    sup = [t["supported_generation"] for t in trail]
    orc = [t["oracle"].get("mutation_gen") for t in trail]
    finding("E4", "observation" if len(set(sup)) == len(set(orc)) else "NEGATIVE",
            f"supported generation sequence {sup} vs oracle mutation_gen {orc}", trail)
    sh(repo, "git", "checkout", "-q", "--", ".")
    cbm.index(repo)

    # b) a reindex lands mid-observation (once): expect retry then consistent result
    def reindex_once(attempt):
        if attempt == 0:
            time.sleep(1.1)  # cross a second boundary so a timestamp generation could move
            write(repo, "src/main.py", MAIN + "\n# concurrent\n")
            cbm.index(repo)
    o = ob.observe(P, repo, can, "balance", hook=reindex_once)
    record("E4", "reindex_mid_observation", o, "matches",
           f"attempts={[(a['gen'], a['consistent']) for a in o['attempts']]}")
    if len(o["attempts"]) < 2:
        finding("E4", "NEGATIVE", "a reindex between the two reads was NOT detected by the supported "
                "generation signal; the result was accepted on the first attempt", o["attempts"])

    # c) canonical revision changes mid-observation, persistently: never a mixed result
    def bump(attempt):
        d = can.load(); can.save(d)
    o = ob.observe(P, repo, can, "balance", hook=bump)
    record("E4", "canonical_changes_every_attempt", o, "unknown")

    # d) stale index: worktree edited, not reindexed
    sh(repo, "git", "checkout", "-q", "--", ".")
    cbm.index(repo)
    write(repo, "src/ledger.py", LEDGER.replace("return sum(ledger)", "return sum(ledger) * 1"))
    o = ob.observe(P, repo, can, "balance")
    record("E4", "stale_index", o, "unknown")
    s = cbm.snippet(P, o["raw"]["hits"][0]) if o["raw"].get("hits") else {}
    finding("E4", "observation", f"get_code_snippet on stale index returns source={s.get('source')!r} "
            f"(disk bytes or indexed bytes?) with graph lines {o['raw'].get('graph_lines')}")

    # e) same-size edit with restored mtime: freshness is mtime+size, not content
    sh(repo, "git", "checkout", "-q", "--", ".")
    cbm.index(repo)
    p = os.path.join(repo, "src/ledger.py")
    st = os.stat(p)
    write(repo, "src/ledger.py", LEDGER.replace("return sum(ledger)", "return max(ledger)"))
    assert os.path.getsize(p) == st.st_size
    os.utime(p, ns=(st.st_atime_ns, st.st_mtime_ns))
    o = ob.observe(P, repo, can, "balance")
    disk = ob.sha(open(p).read())
    idx = cbm.oracle_file_sha(P, "src/ledger.py")
    record("E4", "same_size_same_mtime_edit", o, ("unknown", "differs"),
           "correct answer requires noticing the content change")
    finding("E4", "observation" if o["state"] != "matches" else "NEGATIVE",
            f"same-size/same-mtime edit: freshness={o['raw']['coverage']['freshness']}, state={o['state']}; "
            f"oracle indexed sha matches disk: {idx == disk}")
    sh(repo, "git", "checkout", "-q", "--", ".")


# ── E5 reconstruction ─────────────────────────────────────────────────────
def e5():
    print("E5 reconstruction")
    repo, P = make_repo("e5", base_files())
    can = canonical(repo, {})
    declare(P, repo, can, "CLM-5", [
        code_anchor("balance", "src/ledger.py", "balance", "Function"),
        code_anchor("close", "src/ledger.py", "Account.close", "Method"),
        code_anchor("clamp", "src/util/rounding.py", "clamp", "Function"),
        text_anchor("posting_rules", "docs/design.md", 3, 7),
    ])
    write(repo, "src/util/rounding.py", ROUNDING.replace("def clamp(", "def clamp_value("))
    write(repo, "src/ledger.py", LEDGER.replace("return sum(ledger)", "return sum(ledger) + 0"))
    cbm.index(repo)
    ids = ["balance", "close", "clamp", "posting_rules"]

    def norm(o):
        return {"state": o["state"], "reason": o["reason"],
                "hits": [h.rsplit(".", 2)[-2:] for h in o["raw"].get("hits", [])],
                "successors": sorted((s["file"], s["body_digest_equal"]) for s in o["raw"].get("successors") or [])}
    before = {i: norm(ob.observe(P, repo, can, i)) for i in ids}
    gen_before = cbm.oracle_generation(P)
    cbm.delete_project(P)
    cbm.index(repo)
    after = {i: norm(ob.observe(P, repo, can, i)) for i in ids}
    gen_after = cbm.oracle_generation(P)
    same = before == after
    RESULTS.append({"exp": "E5", "case": "rebuild_equivalence", "state": "equal" if same else "DIFFERENT",
                    "expected": "equal", "as_expected": same, "reason": "",
                    "note": json.dumps({"before": before, "after": after})})
    print(f"  {'ok ' if same else 'XX '}E5 rebuild equivalence: {same}")
    for i in ids:
        print(f"     {i}: {before[i]['state']} / {after[i]['state']}")
    finding("E5", "observation", f"generation ids before {gen_before} after {gen_after} "
            "(incidental, expected to differ)")


# ── E6 identity collision ─────────────────────────────────────────────────
def e6():
    print("E6 identity collision")
    svc = 'def process(item):\n    return item.strip().lower()\n'
    files = {"src/old/service.py": svc, "src/other.py": "def helper():\n    return 1\n"}
    repo, P = make_repo("e6", files)
    can = canonical(repo, {})
    declare(P, repo, can, "CLM-6", [code_anchor("process", "src/old/service.py", "process", "Function")])
    record("E6", "baseline", ob.observe(P, repo, can, "process"), "matches")

    sh(repo, "git", "mv", "src/old", "src/new")
    cbm.index(repo)
    record("E6", "identical_twin_elsewhere", ob.observe(P, repo, can, "process"), "differs",
           "a structurally identical entity elsewhere must stay a candidate, never a match")

    write(repo, "src/copy/service.py", svc)
    cbm.index(repo)
    o = ob.observe(P, repo, can, "process")
    record("E6", "two_identical_twins", o, "differs")

    # same path + same name recreated with a different body
    sh(repo, "git", "reset", "-q", "--hard"); sh(repo, "git", "clean", "-qfd")
    write(repo, "src/old/service.py", 'def process(item):\n    return item\n')
    cbm.index(repo)
    record("E6", "same_locator_different_body", ob.observe(P, repo, can, "process"), "differs",
           "qualified-name identity alone would call this present")

    # same path + same name + same body, but it was deleted and re-created (history broken)
    sh(repo, "git", "reset", "-q", "--hard")
    os.remove(os.path.join(repo, "src/old/service.py")); cbm.index(repo)
    o_gone = ob.observe(P, repo, can, "process")
    write(repo, "src/old/service.py", svc); cbm.index(repo)
    o_back = ob.observe(P, repo, can, "process")
    record("E6", "deleted_then_identical_recreate:gone", o_gone, "differs")
    record("E6", "deleted_then_identical_recreate:back", o_back, "matches",
           "mechanically indistinguishable from never-deleted: current-state observation cannot see history")


# ── E4b follow-ups ────────────────────────────────────────────────────────
def e4b():
    print("E4b generation signal + freshness follow-ups")
    # 1) is recorded_at a usable generation signal? reindex within the same second
    repo, P = make_repo("e4b", base_files())
    rows = []
    for i in range(4):
        write(repo, "src/main.py", MAIN + f"\n# n{i}\n")
        t0 = time.time(); cbm.index(repo); dt = time.time() - t0
        cov = ob.coverage_facts(P, "src/main.py")
        rows.append((round(dt, 2), cov["recorded_at"], cbm.oracle_generation(P)["mutation_gen"]))
    print("   (index secs, recorded_at, oracle mutation_gen):", rows)
    dup = len({r[1] for r in rows}) < len({r[2] for r in rows})
    finding("E4b", "NEGATIVE" if dup else "observation",
            f"recorded_at as generation: {len({r[1] for r in rows})} distinct values for "
            f"{len({r[2] for r in rows})} distinct generations (second resolution)", rows)

    # 2) clean same-size, same-mtime edit on a freshly indexed file
    repo, P = make_repo("e4c", base_files())
    can = canonical(repo, {})
    declare(P, repo, can, "C", [code_anchor("balance", "src/ledger.py", "balance", "Function")])
    p = os.path.join(repo, "src/ledger.py")
    record("E4b", "fresh_baseline", ob.observe(P, repo, can, "balance"), "matches")
    st = os.stat(p)
    write(repo, "src/ledger.py", LEDGER.replace("return sum(ledger)", "return max(ledger)"))
    os.utime(p, ns=(st.st_atime_ns, st.st_mtime_ns))
    o = ob.observe(P, repo, can, "balance")
    record("E4b", "same_size_same_mtime_edit", o, ("unknown", "differs"))
    finding("E4b", "NEGATIVE" if o["state"] == "matches" else "observation",
            f"same-size/same-mtime edit -> freshness={o['raw']['coverage']['freshness']} state={o['state']} "
            f"(snippet reads disk, so body digest {'caught' if o['state']=='differs' else 'did not catch'} it)")

    # 3) does reindexing identical content leave a stale mtime record behind?
    repo, P = make_repo("e4d", base_files())
    p = os.path.join(repo, "src/ledger.py")
    write(repo, "src/ledger.py", LEDGER + "\n")          # edit, no reindex
    write(repo, "src/ledger.py", LEDGER)                 # restore identical bytes, new mtime
    cbm.index(repo)
    f1 = ob.coverage_facts(P, "src/ledger.py")["freshness"]
    cbm.index(repo)
    f2 = ob.coverage_facts(P, "src/ledger.py")["freshness"]
    finding("E4b", "NEGATIVE" if "changed" in f1 + f2 else "observation",
            f"identical bytes with a new mtime, after reindex: freshness={f1}, after a second reindex={f2}")


if __name__ == "__main__":
    os.makedirs(ROOT, exist_ok=True)
    which = sys.argv[1:] or ["e1", "e2", "e3", "e4", "e4b", "e5", "e6"]
    for name in which:
        try:
            globals()[name]()
        except Exception as e:  # keep going; a crashed experiment is itself a result
            import traceback
            traceback.print_exc()
            FINDINGS.append({"exp": name.upper(), "kind": "HARNESS_ERROR", "text": repr(e)})
    out = os.path.join(cbm.SP, "results-" + "-".join(which) + ".json")
    with open(out, "w") as f:
        json.dump({"results": RESULTS, "findings": FINDINGS}, f, indent=1)
    bad = [r for r in RESULTS if not r["as_expected"]]
    print(f"\n{len(RESULTS)} cases, {len(bad)} not as expected; {len(FINDINGS)} findings -> {out}")
