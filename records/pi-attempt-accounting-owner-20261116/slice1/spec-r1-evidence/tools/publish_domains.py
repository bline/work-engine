"""One-shot, reproducible edit: publish numeric/string domains in the canonical schema nodes."""
import json, sys, pathlib
root = pathlib.Path(sys.argv[1])
sys.path.insert(0, str(root / "tooling/spec-check/tests"))
from nano_domain import nano_pattern, OLD_PATTERN
U32, U64 = 4294967295, 18446744073709551615

def walk(node, hits):
    if isinstance(node, dict):
        if node.get("pattern") == OLD_PATTERN:
            node["pattern"] = nano_pattern(); hits["nano"] += 1
        if node.get("type") == "integer" and "minimum" in node and "maximum" not in node:
            node["maximum"] = U64; hits["u64"] += 1
        for v in node.values(): walk(v, hits)
    elif isinstance(node, list):
        for v in node: walk(v, hits)

ck = json.loads((root / "spec/ck.json").read_text())
for node in ck["invariants"]:
    if node["id"] in ("SCHEMA-ACCOUNTING", "SCHEMA-WIRE"):
        hits = {"nano": 0, "u64": 0}
        schema = json.loads(node["text"]); walk(schema, hits)
        node["text"] = json.dumps(schema, sort_keys=True)
        node["revision"] += 1
        print(node["id"], hits, "revision ->", node["revision"])
(root / "spec/ck.json").write_text(json.dumps(ck, indent=2, sort_keys=True) + "\n")

path = root / "spec/ck.schema.json"
ckschema = json.loads(path.read_text()); hits = {"nano": 0, "u64": 0}
def rev(node):
    if isinstance(node, dict):
        if node.get("type") == "integer" and node.get("minimum") == 1 and "maximum" not in node:
            node["maximum"] = U32; hits["u64"] += 1
        for v in node.values(): rev(v)
    elif isinstance(node, list):
        for v in node: rev(v)
rev(ckschema)
path.write_text(json.dumps(ckschema, indent=2, sort_keys=True) + "\n")
print("ck.schema.json revision maxima", hits["u64"])
