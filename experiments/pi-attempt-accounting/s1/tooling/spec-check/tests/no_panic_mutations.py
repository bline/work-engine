"""Controlled-error audit: no mutated leaf may make the CLI panic or die by signal.

Usage: no_panic_mutations.py ROOT SPEC_CHECK_BINARY
Every leaf of representative inputs (evidence bundles incl. grants/links, wire fragment, a seeded
sample of canonical leaves) is replaced by hostile values. The CLI must exit 0 (accepted) or 1
(controlled error); 101 (Rust panic) or a negative status (signal) fails the audit.
"""
import concurrent.futures, copy, json, pathlib, random, subprocess, sys, tempfile
root = pathlib.Path(sys.argv[1]); binary = sys.argv[2]
HOSTILE = [None, True, -1, 1.5, "", [], {}, 18446744073709551616]
RAW_HOSTILE = ["1e400", "1e999999999999", "18446744073709551615.5", "1.0"]
bad = []; runs = 0


def leaves(node, path=()):
    if isinstance(node, dict):
        for k, v in node.items(): yield from leaves(v, path + (k,))
    elif isinstance(node, list):
        for i, v in enumerate(node): yield from leaves(v, path + (i,))
    else:
        yield path


def set_path(doc, path, value):
    at = doc
    for p in path[:-1]: at = at[p]
    at[path[-1]] = value


def run(command, text):
    global runs
    runs += 1
    with tempfile.TemporaryDirectory(prefix="mut-") as tmp:
        f = pathlib.Path(tmp) / "subject.json"; f.write_text(text)
        done = subprocess.run([binary, command, str(root), str(f)], capture_output=True, text=True, timeout=120)
    return done.returncode, done.stderr


def audit(label, command, base, sample=None, seed=0):
    paths = list(leaves(base))
    if sample and len(paths) > sample:
        paths = random.Random(seed).sample(paths, sample)
    jobs = []
    for path in paths:
        for value in HOSTILE:
            doc = copy.deepcopy(base); set_path(doc, path, value)
            jobs.append((path, value, json.dumps(doc)))
        for literal in RAW_HOSTILE:
            doc = copy.deepcopy(base); marker = "@@RAW@@"; set_path(doc, path, marker)
            jobs.append((path, literal, json.dumps(doc).replace(json.dumps(marker), literal)))
    with concurrent.futures.ThreadPoolExecutor(max_workers=8) as pool:
        for (path, value, _), (rc, err) in zip(jobs, pool.map(lambda j: run(command, j[2]), jobs)):
            if rc not in (0, 1): bad.append((label, path, value, rc, err[-160:]))


for name in ["VC07-unlinked.bundle.json", "VC07-conditional.bundle.json"]:
    audit("evidence:" + name, "evidence", json.loads((root / "oracle/r8-controls" / name).read_text()), sample=40, seed=1)
audit("wire", "wire", {"exposure": {"pool": "pi_api", "low": {"nano": "15", "basis": "usage_valuation"}, "high": {"kind": "at_least", "nano": "500", "basis": "proxy_reservation"}},
                       "valuation": {"state": "valued", "complete": False, "low": "15", "high": "15", "identities": ["openai/model"], "derivations": []},
                       "obligation": {"state": "none", "amount_source": "none", "cause": "orphan"}})
audit("canonical", "canonical", json.loads((root / "spec/ck.json").read_text()), sample=15, seed=2)
print(json.dumps({"status": "FAIL" if bad else "PASS", "runs": runs, "panics_or_signals": bad[:15]}, indent=1))
raise SystemExit(1 if bad else 0)
