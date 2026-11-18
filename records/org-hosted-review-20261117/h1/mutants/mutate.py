"""Apply or restore one recorded H1 mutant by exact text replacement.

Adapted from org-hardening-owners-20261117/s1/mutants/mutate.py (same mechanism) for the
work-engine-review-hosted crate in wt-org-hosted-review-20261117.

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
SRC = pathlib.Path("/home/bline/cbm-work/wt-org-hosted-review-20261117/rust/crates/work-engine-review-hosted/src")
REVISION = SRC / "revision.rs"
SUBJECT = SRC / "subject.rs"
SECRETS = SRC / "secrets.rs"
CHECK = SRC / "check.rs"

MUTANTS = {
    "H1-M1-known-incompatibility-not-refused": [(REVISION,
        "    if !known.is_empty() {",
        "    if false && !known.is_empty() {")],
    "H1-M2-patch-identity-unchecked": [(SUBJECT,
        "    if patch_identity != expected.patch_identity {",
        "    if false && patch_identity != expected.patch_identity {")],
    "H1-M3-scope-digest-unbound": [(SUBJECT,
        "    if scope_digest != patch_identity {",
        "    if false && scope_digest != patch_identity {")],
    "H1-M4-size-bound-quadrupled": [(SUBJECT,
        "|| snapshot.bytes() > SNAPSHOT_BYTES {",
        "|| snapshot.bytes() > SNAPSHOT_BYTES * 4 {")],
    "H1-M5-pem-rule-never-matches": [(SECRETS,
        'PRIVATE KEY(?: BLOCK)?-----',
        'PRIVATE KEYS(?: BLOCK)?-----')],
    "H1-M6-fence-stripped-before-parse": [(CHECK,
        "        match parse_json(&result.answer) {",
        '        match parse_json(result.answer.trim().trim_start_matches("```json").trim_end_matches("```").trim()) {')],
    "H1-M7-contract-digest-not-compared": [(CHECK,
        "                frozen.contract_sha256() == record.contract_sha256",
        "                (frozen.contract_sha256() == record.contract_sha256 || true)")],
    "H1-M8-delivery-prefix-unchecked": [(CHECK,
        "        if !texts[0].starts_with(task) {",
        "        if false && !texts[0].starts_with(task) {")],
    "H1-M9-envelope-consumable": [(CHECK,
        "        consumable: false,",
        "        consumable: true,")],
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
