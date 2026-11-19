"""Apply or restore one recorded A1 mutant by exact text replacement.

Adapted from records/org-hardening-owners-20261117/s1/mutants/mutate.py (same mechanism) for the
A1 worktree.

usage: python3 -I mutate.py apply|restore <mutant-id>

apply: every `old` must occur exactly once in its file; the original bytes and their SHA-256 are
saved under <mutant-id>/ before the file is rewritten. restore: writes the saved bytes back and
checks the file's SHA-256 equals the recorded original. Tests are run by custody, not here.
"""
import hashlib
import json
import pathlib
import sys

HERE = pathlib.Path(__file__).resolve().parent
CRATES = pathlib.Path("/home/bline/cbm-work/wt-org-owners-a1-20261118/rust/crates")
SA = CRATES / "work-engine-organization-authority/src/gate.rs"
GATE = CRATES / "work-engine-review-delivery/src/gate.rs"

MUTANTS = {
    "A1-M1-s1-always-passes": [(SA,
        "                Some(actual) if actual == expected => None,\n                Some(actual) => Some(reason(",
        "                Some(_) => None,\n                Some(actual) => Some(reason(")],
    "A1-M2-digest-ignores-commit-and-tree": [(SA,
        "            (\"commit\", JsValue::text(&self.commit)),\n            (\"tree\", JsValue::text(&self.tree)),",
        "            (\"commit\", JsValue::text(\"\")),\n            (\"tree\", JsValue::text(\"\")),")],
    "A1-M3-first-candidate-obligation": [(SA,
        "                .find(|o| o.id == *id)\n                .map(|o| o.scope_digest.to_hex());",
        "                .next()\n                .map(|o| o.scope_digest.to_hex());")],
    "A1-M4-discharge-assumed": [(GATE,
        "BlockerKind::ScopeDigestToSubject => Discharge::Admitted(&[CheckId::S1]),",
        "BlockerKind::ScopeDigestToSubject => Discharge::AssumedDischarge,")],
    "A1-M5-s1-dropped-from-all": [(SA,
        "        CheckId::R4,\n        CheckId::S1,\n    ];",
        "        CheckId::R4,\n    ];"),
        (SA, "pub const ALL: [CheckId; 10]", "pub const ALL: [CheckId; 9]")],
    "A1-M6-unserved-vantage-passes": [(SA,
        "    if served.is_empty() {\n        return vec![reason(ReasonCode::ScopeDigestUnserved, vantage)];\n    }\n",
        "")],
    "A1-M7-unknown-vantage-passes": [(SA,
        "    if participants.is_empty() {\n        return vec![reason(ReasonCode::VantageUnknown, vantage)];\n    }\n",
        "")],
}


def sha(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def main() -> None:
    action, mutant = sys.argv[1], sys.argv[2]
    edits = MUTANTS[mutant]
    out = HERE / mutant
    record_path = out / "mutation.json"
    if action == "apply":
        out.mkdir(parents=True, exist_ok=True)
        files = {}
        for path, _, _ in edits:
            if str(path) not in files:
                files[str(path)] = path.read_bytes()
        for index, (path, data) in enumerate(files.items()):
            (out / f"orig-{index}.bak").write_bytes(data)
        texts = {p: d.decode() for p, d in files.items()}
        for path, old, new in edits:
            count = texts[str(path)].count(old)
            if count != 1:
                sys.exit(f"{mutant}: expected one occurrence in {path}, found {count}: {old!r}")
            texts[str(path)] = texts[str(path)].replace(old, new)
        for path, text in texts.items():
            pathlib.Path(path).write_text(text)
        record = {
            "mutant": mutant,
            "files": [
                {"path": p, "backup": f"orig-{i}.bak", "original_sha256": sha(d),
                 "mutated_sha256": sha(texts[p].encode())}
                for i, (p, d) in enumerate(files.items())
            ],
            "edits": [{"path": str(p), "old": o, "new": n} for p, o, n in edits],
        }
        record_path.write_text(json.dumps(record, indent=1) + "\n")
        print(f"applied {mutant}: " + ", ".join(f["mutated_sha256"][:12] for f in record["files"]))
    elif action == "restore":
        record = json.loads(record_path.read_text())
        for entry in record["files"]:
            data = (out / entry["backup"]).read_bytes()
            pathlib.Path(entry["path"]).write_bytes(data)
            restored = sha(pathlib.Path(entry["path"]).read_bytes())
            if restored != entry["original_sha256"]:
                sys.exit(f"{mutant}: restore mismatch for {entry['path']}")
        print(f"restored {mutant}: " + ", ".join(f["original_sha256"][:12] for f in record["files"]))
    else:
        sys.exit("usage: mutate.py apply|restore <mutant-id>")


if __name__ == "__main__":
    main()
