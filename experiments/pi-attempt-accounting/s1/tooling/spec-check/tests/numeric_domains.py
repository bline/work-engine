"""Numeric/string domain agreement between the published schemas and the actual Rust CLI.

Usage: numeric_domains.py ROOT SPEC_CHECK_BINARY

Expectations are computed here with exact decimal arithmetic (never binary floats) from the
published domain, then compared with (a) the Rust CLI exit status and, wherever Python can hold
the literal exactly, (b) Python Draft 2020-12 `jsonschema`. Rust must exit 0 (accept) or 1
(controlled rejection); any other status (panic 101, signal) is a failure. Each negative names the
predicate it must hit by requiring a message fragment from the Rust diagnostic.
"""
import copy, itertools, json, pathlib, re, subprocess, sys, tempfile
from decimal import Decimal, localcontext
from fractions import Fraction
import jsonschema
sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
from nano_domain import I128_MAX, nano_pattern

root = pathlib.Path(sys.argv[1]); binary = sys.argv[2]
U32, U64 = 2**32 - 1, 2**64 - 1
failures = []; counts = {}


def run(command, value_text, extra=()):
    with tempfile.TemporaryDirectory(prefix="numeric-domains-") as tmp:
        path = pathlib.Path(tmp) / "subject.json"; path.write_text(value_text)
        done = subprocess.run([binary, command, str(root), str(path), *extra], capture_output=True, text=True, timeout=60)
    return done.returncode, done.stderr.strip()


def exact_integer(literal):
    """Exact integer denoted by a JSON number literal, else None (non-number or non-integral)."""
    if not re.fullmatch(r"-?(0|[1-9][0-9]*)(\.[0-9]+)?([eE][+-]?[0-9]+)?", literal):
        return None
    exponent = int(re.split("[eE]", literal)[1]) if re.search("[eE]", literal) else 0
    if abs(exponent) > 10**6:
        mantissa = Decimal(re.split("[eE]", literal)[0])
        if mantissa == 0: return 0
        if exponent < 0: return None
        return -(10**100) if literal.startswith("-") else 10**100  # integral, beyond every published bound
    with localcontext() as ctx:
        ctx.prec = 5000
        d = Decimal(literal)
        return int(d) if d == d.to_integral_value() else None


def record(label, ok, detail):
    domain = label.split("[")[0] + (":python" if label.endswith(":python") else "")
    counts[domain] = counts.get(domain, 0) + 1
    if not ok: failures.append((label, detail))


def check(label, command, text, accept, fragment=None, schema=None, python_comparable=True, extra=()):
    rc, err = run(command, text, extra)
    ok = rc == (0 if accept else 1) and (accept or fragment is None or fragment in err)
    record(label, ok, {"rc": rc, "stderr": err[:300], "wanted_accept": accept, "fragment": fragment})
    if schema is not None and python_comparable:
        py = jsonschema.Draft202012Validator(schema).is_valid(json.loads(text))
        record(label + ":python", py == accept, {"python_valid": py, "wanted_accept": accept})


LITERALS = [
    "0", "1", "2", "4294967294", "4294967295", "4294967296", "4294967297", "1.0", "1e0", "1E0", "1.0e0", "1E+0", "10e-1",
    "0.0", "-0", "-0.0", "0e5", "-1", "1.5", "0.5", "0.9999999999999999", "4294967295.0", "4294967296.0", "4294967295.5",
    "4.294967295e9", "42949672950e-1", "4294967295.000000000000000000000001", "4294967294.9999999999999999999999999",
    "4294967296.000000000000000000000001", "1e400", "1e-400", "1e999999999999", "-1e999999999999", "1e-999999999999",
    "0e999999999999", "true", "false", "null", '"1"', '"4294967295"', "[1]", '{"v":1}',
    "9223372036854775807", "9223372036854775808", "-9223372036854775808", "-9223372036854775809",
    "18446744073709551614", "18446744073709551615", "18446744073709551616", "18446744073709551617",
    "18446744073709551615.0", "18446744073709551616.0", "18446744073709551615.5",
    "18446744073709551614.99999999999999999999", "18446744073709551615.00000000000000000000001",
    "1.8446744073709551615e19", "1.8446744073709551616e19", "184467440737095516150e-1", "1e19", "1e20", "1e38", "1e39", "1e40", "1e41", "1e308", "1e309",
    "340282366920938463463374607431768211455", "340282366920938463463374607431768211456",
]


def python_exact(literal):
    """True when Python's json parse holds the literal exactly (so Python jsonschema is a fair oracle).

    Python parses decimal/exponent literals to binary64; where that rounds, Python itself is wrong about the
    published schema and only the exact arithmetic above is authoritative.
    """
    try:
        parsed = json.loads(literal)
    except ValueError:
        return False
    if not isinstance(parsed, float):
        return True
    if not parsed == parsed or parsed in (float("inf"), float("-inf")):
        return False
    if re.search("[eE][+-]?[0-9]{4,}", literal):
        return False
    with localcontext() as ctx:
        ctx.prec = 5000
        return Fraction(parsed) == Fraction(Decimal(literal))


canonical = json.loads((root / "spec/ck.json").read_text())
ck_schema = json.loads((root / "spec/ck.schema.json").read_text())
acct_schema = json.loads((root / "spec/accounting.schema.json").read_text())
wire_schema = json.loads((root / "spec/wire.schema.json").read_text())


def with_revision(literal):
    text = json.dumps(canonical, indent=1, sort_keys=True)
    marker = f'"id": "{canonical["types"][0]["id"]}",\n   "revision": {canonical["types"][0]["revision"]},'
    assert marker in text
    return text.replace(marker, marker.rsplit(":", 1)[0] + ": " + literal + ",", 1)


# 1. ck.types[0].revision: domain 1..=u32::MAX
for lit in LITERALS:
    value = exact_integer(lit)
    wanted = value is not None and 1 <= value <= U32
    text = with_revision(lit)
    rc_label = f"revision[{lit}]"
    fragment = None if wanted else "Draft2020-12"
    comparable = python_exact(lit)
    check(rc_label, "canonical", text, wanted, fragment, ck_schema if comparable else None)

BUNDLE = {"documents": [
    {"kind": "accounting_input_proof", "id": "input", "version": "1.7-s1", "lineage_id": "L", "origins": [{"origin_id": "o", "source_origin_id": "s", "original_position": 1, "raw_bytes_sha256": "a" * 64, "source_revision_ref": "raw:revision", "initial_contract_ref": "c"}], "initial_contracts": [{"contract_id": "c", "reference_namespace": "unestablished", "duplicate_rule": "same_origin_only"}], "grants": [], "identity_bindings": [], "exact_links": [], "continuity": {"predecessor_subject_refs": [], "preserved_initial_contract_refs": [], "preserved_fact_refs": [], "exact_origin_map_ref": "raw:origin-map"}},
    {"kind": "accounting_result", "id": "result", "version": "1.7-s1", "input_proof_ref": "input", "lineage_id": "L", "origins": [{"origin_ref": "o", "base_id": "b", "evidence_atom_id": "o", "active_high_unit_id": "u"}], "bases": [{"base_id": "b", "origin_refs": ["o"], "atomize": False, "atomize_fact_refs": [], "break_copies": False, "break_copies_fact_refs": []}], "units": [{"unit_id": "u", "state": "active", "origin_refs": ["o"], "exposure_ref": "computed:exposure"}], "primary_validities": [], "financial_candidates": [], "definite_links": [], "coverage_decisions": [], "low_rollups": [], "continuity_result": "unestablished", "failure_refs": []},
    {"kind": "accounting_expected_assertion", "id": "expected", "version": "1.7-s1", "raw_subject_ref": "raw:revision", "policy_refs": ["PLAN"], "authored_by": "independent-owner", "exact_subject_refs": ["input", "result"], "assertions": []},
]}
bundle_text = json.dumps(BUNDLE)
assert run("evidence", bundle_text)[0] == 0, run("evidence", bundle_text)


def bundle_with(literal):
    text = json.dumps(BUNDLE)
    return text.replace('"original_position": 1,', '"original_position": ' + literal + ",", 1)


# 2. input origin original_position: domain 0..=u64::MAX (the former panic site)
for lit in LITERALS:
    value = exact_integer(lit)
    wanted = value is not None and 0 <= value <= U64
    text = bundle_with(lit)
    comparable = python_exact(lit)
    doc_schema = acct_schema
    check(f"original_position[{lit}]", "evidence", text, wanted, None if wanted else "Draft2020-12", None)
    if comparable:
        py = jsonschema.Draft202012Validator(doc_schema).is_valid(json.loads(text)["documents"][0])
        record(f"original_position[{lit}]:python", py == wanted, {"python_valid": py, "wanted": wanted})

# 3. Nano strings: canonical decimal 0..=i128::MAX in every money location (wire + evidence)
NANOS = [("0", True), ("1", True), ("00", False), ("01", False), ("-0", False), ("-1", False), ("+1", False), ("1.0", False), ("1e3", False),
         ("", False), (" 1", False), ("1 ", False), ("1\n", False), ("0x1", False), ("１", False),
         ("9" * 38, True), ("9" * 39, False), ("1" + "0" * 38, True),
         (str(I128_MAX - 1), True), (str(I128_MAX), True), (str(I128_MAX + 1), False), (str(I128_MAX + 2), False),
         ("170141183460469231731687303715884105728" + "0", False), ("2" + "0" * 38, False), ("9" * 40, False),
         ("17014118346046923173168730371588410572", True), ("170141183460469231731687303715884105730", False), ("180000000000000000000000000000000000000", False)]
WIRE = {"exposure": {"pool": "pi_api", "low": {"nano": "15", "basis": "usage_valuation"}, "high": {"kind": "at_least", "nano": "500", "basis": "proxy_reservation"}},
        "valuation": {"state": "valued", "complete": False, "low": "15", "high": "15", "identities": ["openai/model"], "derivations": []},
        "obligation": {"state": "none", "amount_source": "none", "cause": "orphan"}}
UNPRICEABLE = {"exposure": WIRE["exposure"], "valuation": {"state": "unpriceable", "reasons": ["ModelUnresolved"], "supported_low": "1", "priced_high": "2", "identities": [], "derivations": []}}
wire_places = {
    "exposure.low.nano": (WIRE, ("exposure", "low", "nano")), "exposure.high.nano": (WIRE, ("exposure", "high", "nano")),
    "valuation.low": (WIRE, ("valuation", "low")), "valuation.high": (WIRE, ("valuation", "high")),
    "valuation.priced_high": (UNPRICEABLE, ("valuation", "priced_high")), "valuation.supported_low": (UNPRICEABLE, ("valuation", "supported_low")),
}
for place, (base, keys) in wire_places.items():
    for text_value, wanted in NANOS:
        value = copy.deepcopy(base); at = value
        for k in keys[:-1]: at = at[k]
        at[keys[-1]] = text_value
        text = json.dumps(value)
        check(f"wire-nano[{place}={text_value!r}]", "wire", text, wanted, None if wanted else "Draft2020-12",
              wire_schema if "\n" not in text_value else None)
RESULT = copy.deepcopy(BUNDLE["documents"][1])
RESULT["retained_components"] = [{"component_id": "k", "origin_ref": "o", "role": "charge", "nano_usd": "5"}]
RESULT["units"][0]["retained_component_refs"] = ["k"]
for text_value, wanted in NANOS:
    bundle = copy.deepcopy(BUNDLE); r = copy.deepcopy(RESULT); r["retained_components"][0]["nano_usd"] = text_value; bundle["documents"][1] = r
    check(f"evidence-nano[retained.nano_usd={text_value!r}]", "evidence", json.dumps(bundle), wanted, None if wanted else "Draft2020-12")
# the pattern itself against the exact integer interval, independent of any validator
compiled = re.compile(nano_pattern())
probe = set()
for c in [0, 1, 9, 10, 10**38 - 1, 10**38, 10**38 + 1, I128_MAX - 1, I128_MAX, I128_MAX + 1, 2 * 10**38, 10**39 - 1, 10**39]:
    probe.update(range(max(0, c - 3), c + 4))
top = str(I128_MAX)
for i in range(len(top)):
    for d in "0123456789":
        probe.add(int(top[:i] + d + top[i + 1:]))
for n in probe:
    record("pattern-frontier", bool(compiled.fullmatch(str(n))) == (0 <= n <= I128_MAX), {"n": n})

# 4. continuity compares integers, not spellings
def predecessor_bundle(second_position):
    b = copy.deepcopy(BUNDLE)
    i2 = copy.deepcopy(b["documents"][0]); i2["id"] = "input2"; i2["origins"][0]["original_position"] = "@@POSITION@@"
    i2["continuity"]["predecessor_subject_refs"] = ["input"]
    r2 = copy.deepcopy(b["documents"][1]); r2["id"] = "result2"; r2["input_proof_ref"] = "input2"
    b["documents"][2]["exact_subject_refs"] = ["input", "result", "input2", "result2"]
    b["documents"] += [i2, r2]
    return json.dumps(b).replace('"@@POSITION@@"', second_position)
for spelling, wanted in [("1", True), ("1.0", True), ("1e0", True), ("10e-1", True), ("2", False), ("1.5", False)]:
    text = predecessor_bundle(spelling)
    check(f"continuity-position[{spelling}]", "evidence", text, wanted, None if wanted else ("Draft2020-12" if spelling == "1.5" else "continuity drops or rewrites origin"))

print(json.dumps({"status": "FAIL" if failures else "PASS", "checks": sum(counts.values()), "by_domain": counts, "failure_count": len(failures), "failures": failures[:25]}, indent=1))
raise SystemExit(1 if failures else 0)
