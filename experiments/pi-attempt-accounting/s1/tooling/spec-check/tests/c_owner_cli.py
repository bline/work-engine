"""C-validity sole ownership through the actual `canonical` CLI (report C1-R8-01 counterexample).

Usage: c_owner_cli.py ROOT SPEC_CHECK_BINARY
Each negative requires the owner-violation diagnostic (not a digest/schema/partition failure) and is
paired with a C-stage positive built from the identical constructor.
"""
import copy, json, pathlib, subprocess, sys, tempfile
root = pathlib.Path(sys.argv[1]); binary = sys.argv[2]
base = json.loads((root / "spec/ck.json").read_text())
VALID = {"type_id": "SettlementDecision", "variant": "valid", "fields": ["basis", "charged", "settlement"], "bindings": {}}
results = []


def canonical(document):
    with tempfile.TemporaryDirectory(prefix="c-owner-") as tmp:
        path = pathlib.Path(tmp) / "ck.json"; path.write_text(json.dumps(document))
        done = subprocess.run([binary, "canonical", str(root), str(path)], capture_output=True, text=True, timeout=120)
    return done.returncode, done.stderr.strip()


def mutated(rule_id, outputs):
    document = copy.deepcopy(base)
    next(r for r in document["rules"] if r["id"] == rule_id)["outputs"] = outputs
    return document


def expect(label, document, accept, fragment=None):
    rc, err = canonical(document)
    ok = rc == (0 if accept else 1) and (accept or (fragment in err))
    results.append({"label": label, "ok": ok, "rc": rc, "stderr": err[:200]})


expect("unmodified canonical (S forwards C reference)", base, True)
expect("V-PRESENCE-1 constructs SettlementDecision.valid (report counterexample)", mutated("V-PRESENCE-1", [VALID]), False, "C-validity owner violation: stage V rule V-PRESENCE-1")
for stage, rule in [("S", "S-zero-zero"), ("F", "F-REPLACEMENT-1"), ("V", "V-PRESENCE-9"), ("S", "S-TIME-1")]:
    expect(f"{rule} ({stage}) constructs C-owned decision", mutated(rule, [VALID]), False, f"stage {stage} rule {rule}")
expect("C-KEY-1 constructs SettlementDecision.valid (positive)", mutated("C-KEY-1", [VALID]), True)
document = copy.deepcopy(base)
next(s for s in document["stages"] if s["id"] == "V")["outputs"].append("settlement_validity")
expect("stage V declares settlement_validity", document, False, "declares C-owned output settlement_validity")
print(json.dumps({"status": "PASS" if all(r["ok"] for r in results) else "FAIL", "results": results}, indent=1))
raise SystemExit(0 if all(r["ok"] for r in results) else 1)
