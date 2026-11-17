"""R8-COVERAGE-01 / R8-NAMESPACE-01 controls through the actual `evidence` CLI on the supplied
oracle bundles (read-only copies). Each negative requires the diagnostic fragment naming the
predicate it must hit; each family has positives built from the same file.

Usage: coverage_namespace_controls.py ROOT SPEC_CHECK_BINARY
"""
import copy, json, pathlib, shutil, subprocess, sys, tempfile
root = pathlib.Path(sys.argv[1]).resolve(); binary = sys.argv[2]
results = []
VC07 = "oracle/r8-controls/VC07-conditional.bundle.json"
VC07U = "oracle/r8-controls/VC07-unlinked.bundle.json"
VC11 = "oracle/r8-completion-A/P132-FG-VC11-gated.bundle.json"
VC08 = "oracle/r8-completion-B/VC08-subscription-measured-request.bundle.json"
O1, O2 = "obligation:res:S1/turn-1:rid=1", "obligation:res:S1/turn-2:rid=1"


def result(d): return d["documents"][1]
def inp(d): return d["documents"][0]


def evidence(rel, mutate=None, evidence_root=None, text_edit=None):
    """Run `evidence` on a private copy of the bundle's directory (payload refs stay resolvable)."""
    src = root / rel
    with tempfile.TemporaryDirectory(prefix="cov-ns-") as tmp:
        work = pathlib.Path(tmp) / "work"; shutil.copytree(src.parent, work)
        doc = json.loads(src.read_text())
        if mutate: mutate(doc)
        path = work / src.name; path.write_text(json.dumps(doc, indent=1))
        if text_edit:
            for f in work.glob("*.json"): f.write_text(text_edit(f.read_text()))
        done = subprocess.run([binary, "evidence", str(evidence_root or root), str(path)], capture_output=True, text=True, timeout=120)
    return done.returncode, done.stderr.strip()


def expect(label, accept, rel, mutate=None, fragment=None, **kw):
    rc, err = evidence(rel, mutate, **kw)
    ok = rc == (0 if accept else 1) and (accept or (fragment and fragment in err))
    results.append({"label": label, "ok": ok, "rc": rc, "diagnostic": err[:160]})


# --- coverage: exact positives (budgeted, unbudgeted-with-link, unlinked retains AtLeast0)
expect("positive budgeted VC11", True, VC11)
expect("positive unbudgeted VC07 conditional exact link", True, VC07)
expect("positive unlinked VC07 (anchor stays active)", True, VC07U)
unlinked = json.loads((root / VC07U).read_text())
assert not result(unlinked)["coverage_decisions"] and "AtLeast0" in json.dumps(result(unlinked)["units"]), "VC07-unlinked must keep AtLeast0 unreleased"

# --- coverage: VC07 negatives
expect("uncited link: definite_link_refs=[] (C1-R8-02)", False, VC07, lambda d: result(d)["coverage_decisions"][0].update(definite_link_refs=[]), "released anchor lacks cited exact link")
def redirect(d): inp(d)["exact_links"][0]["to_origin_ref"] = "a"; result(d)["definite_links"][0]["to_origin_ref"] = "a"
expect("self link a->a on both sides (A1-R8-01)", False, VC07, redirect, "self-link cannot establish co-reference")
expect("result link redirected away from admitted link", False, VC07, lambda d: result(d)["definite_links"][0].update(to_origin_ref="a"), "result link endpoints differ from exact admitted link")
expect("wrong valuation usage", False, VC07, lambda d: result(d)["valuation_records"][0].update(usage_origin_refs=["a"]), "coverage valuation usage differs")
expect("endpoint bytes changed under the same id", False, VC07, lambda d: inp(d)["origins"][1].update(raw_bytes_sha256="0" * 64), "exact-link endpoint bytes mismatch")
def no_link(d):
    inp(d)["exact_links"] = []; inp(d)["grants"] = []; result(d)["definite_links"] = []
expect("coverage cites a link that no longer exists", False, VC07, no_link, "dangling coverage definite-link identity")
def smuggle(d):
    c = result(d)["coverage_decisions"][0]; c.update(monetary_origin_refs=["a", "u"], anchor_origin_refs=[], definite_link_refs=[])
expect("anchor retired as monetary origin without a link", False, VC07, smuggle, "coverage monetary origins differ from the selected primary/usage inventory")
expect("partial coverage: one covered unit only", False, VC07, lambda d: result(d)["coverage_decisions"][0].update(unit_refs=["unit:a"]), "coverage is not whole-unit complete")
expect("hold released with no coverage", False, VC07, lambda d: result(d).update(coverage_decisions=[]), "is not retired into its successor by an accepted coverage")

# --- coverage: VC11 budgeted negatives (witness swap/redirect, class omission, ownership)
def swap_validity(d):
    v = result(d)["primary_validities"]
    for k in ("distinct_reservation_refs", "distinct_settlement_refs"): v[0][k], v[1][k] = v[1][k], v[0][k]
expect("witness refs swapped between obligations", False, VC11, swap_validity, "primary validity representatives differ from membership")
def swap_all(d):
    swap_validity(d); m = result(d)["primary_memberships"]
    for k in ("reservation_classes", "settlement_classes", "usage_origin_refs"): m[0][k], m[1][k] = m[1][k], m[0][k]
expect("witness and membership swapped, coverage keeps its obligation", False, VC11, swap_all, "coverage missing selected primary inventory")
def redirect_rejected(d):
    result(d)["primary_validities"][0]["validity_result"] = "rejected"
    c = result(d)["coverage_decisions"][0]; c["validity_ref"] = O2; c["obligation_id"] = O2
expect("rejected first obligation, coverage redirected to the second (A1-R8-01)", False, VC11, redirect_rejected, "coverage missing selected primary inventory")
def mismatch(d): result(d)["coverage_decisions"][0]["validity_ref"] = O2
expect("validity_ref differs from obligation_id", False, VC11, mismatch, "coverage obligation/validity reference mismatch")
expect("coverage over a rejected witness", False, VC11, lambda d: result(d)["primary_validities"][0].update(validity_result="rejected"), "coverage uses invalid primary witness")
def omit_class(d):
    result(d)["primary_memberships"][0]["reservation_classes"] = []
expect("joint class omission (reservation class dropped)", False, VC11, omit_class, "primary validity representatives differ from membership")
def usage_swap(d):
    m = result(d)["primary_memberships"]; m[0]["usage_origin_refs"], m[1]["usage_origin_refs"] = m[1]["usage_origin_refs"], m[0]["usage_origin_refs"]
expect("usage inventories swapped", False, VC11, usage_swap, "coverage missing selected usage inventory")
expect("valuation of the other obligation", False, VC11, lambda d: result(d)["coverage_decisions"][0].update(valuation_ref="valuation:res:S1/turn-2:rid=1"), "coverage valuation usage differs")
expect("valuation usage is the reservation row", False, VC11, lambda d: result(d)["valuation_records"][0].update(usage_origin_refs=["S1/turn-1:1"]), "coverage valuation usage differs")
all4 = ["S1/turn-1:1", "S1/turn-1:2", "S1/turn-2:1", "S1/turn-2:2"]
expect("first coverage spans both obligations", False, VC11, lambda d: result(d)["coverage_decisions"][0].update(monetary_origin_refs=all4), "coverage is not whole-unit complete")
expect("first coverage retires the other obligation's hold", False, VC11, lambda d: result(d)["coverage_decisions"][0].update(unit_refs=["held:res:S1/turn-2:rid=1"]), "coverage is not whole-unit complete")
expect("first coverage dropped, hold still released", False, VC11, lambda d: result(d)["coverage_decisions"].pop(0), "is not retired into its successor by an accepted coverage")
expect("first coverage marked rejected, hold still released", False, VC11, lambda d: result(d)["coverage_decisions"][0].update(status="rejected"), "is not retired into its successor by an accepted coverage")

# --- namespace. The supplied admission-row bundles are the corrected (`unestablished`, no grant)
# shape; the wrong shape is constructed explicitly from them in a private root.
ADMISSION_CASES = [
    ("VC08", VC08),
    ("U06 unlabelled", "oracle/r8-completion-B/U06-subscription-upstream-error-unlabelled.bundle.json"),
    ("V03 labelled api", "oracle/r8-completion-B/V03-subscription-upstream-error-labelled-api.bundle.json"),
    ("V03 control labelled", "oracle/r8-completion-B/V03-control-labelled-upstream-error.bundle.json"),
    ("N03 contradicted admission", "oracle/r8-completion-B/N03-contradicted-admission.bundle.json"),
    ("N03 control unlabelled admission", "oracle/r8-completion-B/N03-control-unlabelled-admission.bundle.json"),
]
for label, rel in ADMISSION_CASES:
    expect(f"corrected supplied bundle (unestablished, no grant) accepted: {label}", True, rel)


def regroup(d):  # review-r2 D-2: the pre-correction grouped base, but under the corrected unestablished contract
    r = result(d); ids = [o["origin_ref"] for o in r["origins"]]
    r["bases"] = [{"base_id": "base:grouped", "origin_refs": ids, "atomize": False, "atomize_fact_refs": [], "break_copies": False, "break_copies_fact_refs": []}]
    for o in r["origins"]: o["base_id"] = "base:grouped"
for label, rel in ADMISSION_CASES[:4]:
    expect(f"grouping both origins under an unestablished contract: {label}", False, rel, regroup,
           "under an unestablished contract (or without an admitted scope) with no admitted exact link or identity binding joining them")


def declared_scope(rel, namespace):
    """Private root holding the corrected bundle re-declared with an admitted scope per source:
    contract `namespace` + `initial_reference_scope` grant + matching provenance payload, i.e.
    bundle, contract, grant and payload all agree with one another."""
    src = root / rel
    case = src.name.removesuffix(".bundle.json")
    tmp = tempfile.TemporaryDirectory(prefix="cov-ns-root-")
    base = pathlib.Path(tmp.name)
    shutil.copytree(root / "spec", base / "spec")
    (base / "corpus/cases").mkdir(parents=True)
    shutil.copy(root / f"corpus/cases/{case}.json", base / "corpus/cases")
    directory = base / pathlib.Path(rel).parent
    directory.mkdir(parents=True)
    doc = json.loads(src.read_text())
    proof = inp(doc)
    raw = proof["origins"][0]["source_revision_ref"].removeprefix("sha256:")
    sources = sorted({o["source_origin_id"] for o in proof["origins"]})
    provenance = {"admission": "conditional_fixture", "authority_ref": "private-fixture", "case_id": proof["lineage_id"],
                  "grant_payloads": [], "initial_scope_contracts": [], "raw_input_sha256": raw}
    proof["initial_contracts"], proof["grants"] = [], []
    for s in sources:
        members = [o["origin_id"] for o in proof["origins"] if o["source_origin_id"] == s]
        contract = {"contract_id": f"scope:{s}", "duplicate_rule": "same_origin_only", "grant_ref": f"grant:{s}",
                    "reference_namespace": namespace, "scope_id": f"{case}/{s}"}
        proof["initial_contracts"].append(contract)
        proof["grants"].append({"admission": "conditional_fixture", "assumption_refs": ["private fixture, not a live guarantee"],
                                "authority_ref": "private-fixture", "covered_extent_ref": f"sha256:{raw}", "covered_origin_refs": members,
                                "exact_payload_ref": f"{rel.replace('.bundle.json', '.provenance.json')}#{s}", "grant_id": f"grant:{s}",
                                "guarantee_kind": "initial_reference_scope", "observer_or_harness_ref": "private-fixture"})
        provenance["initial_scope_contracts"].append(contract)
        provenance["grant_payloads"].append({"auxiliary_exact_row_coreference": "unestablished", "exact_copy_equivalence": False,
                                             "exact_origin_refs": members, "guarantee": f"private fixture {namespace} scope",
                                             "immutable_reference_namespace": contract["scope_id"], "physical_bill_independence": "unestablished",
                                             "source_origin_id": s})
        for o in proof["origins"]:
            if o["source_origin_id"] == s:
                o["initial_contract_ref"] = contract["contract_id"]
    bundle = base / rel
    bundle.write_text(json.dumps(doc))
    bundle.with_name(src.name.replace(".bundle.json", ".provenance.json")).write_text(json.dumps(provenance))
    return tmp, base, bundle


def run_private(base, bundle):
    done = subprocess.run([binary, "evidence", str(base), str(bundle)], capture_output=True, text=True, timeout=120)
    return done.returncode, done.stderr.strip()


for label, rel in ADMISSION_CASES:
    tmp, base, bundle = declared_scope(rel, "dollar_ticket")
    rc, err = run_private(base, bundle)
    results.append({"label": f"dollar_ticket scope (grant, scope, payload agree) applied to admission origins: {label}",
                    "ok": rc == 1 and "raw row is admission-namespace but its initial contract" in err and "declares dollar_ticket" in err,
                    "rc": rc, "diagnostic": err[:160]})
    tmp.cleanup()

tmp, base, bundle = declared_scope(VC08, "admission")
rc, err = run_private(base, bundle)
results.append({"label": "positive: separately declared conditional admission-namespace scope (private fixture, not a live guarantee)",
                "ok": rc == 0, "rc": rc, "diagnostic": err[:160]})
doc = json.loads(bundle.read_text()); inp(doc)["origins"][0]["raw_bytes_sha256"] = "0" * 64
bundle.write_text(json.dumps(doc))
rc, err = run_private(base, bundle)
results.append({"label": "admitted admission scope with origin bytes changed under the same id", "ok": rc == 1 and "raw bytes differ from its bound raw row" in err, "rc": rc, "diagnostic": err[:160]})
tmp.cleanup()
# (dollar rows under an admission declaration: asserted in the namespace.rs unit tests, where raw rows,
# bundle and contract are constructed to agree)

failed = [r for r in results if not r["ok"]]
print(json.dumps({"status": "FAIL" if failed else "PASS", "checks": len(results), "failed": failed, "results": [r["label"] for r in results]}, indent=1))
raise SystemExit(1 if failed else 0)
