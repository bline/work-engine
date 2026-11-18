"""Apply or restore one recorded H1 remediation-1 mutant by exact text replacement.

Same mechanism as h1/mutants/mutate.py (itself adapted from org-hardening-owners-20261117/s1).

usage: python3 -I mutate.py apply|restore <mutant-id>
"""
import hashlib
import json
import pathlib
import sys

HERE = pathlib.Path(__file__).resolve().parent
SRC = pathlib.Path("/home/bline/cbm-work/wt-org-hosted-review-20261117/rust/crates/work-engine-review-hosted/src")
CHECK, SUBJECT, SECRETS, TASK, INPUTS = (SRC / f for f in ("check.rs", "subject.rs", "secrets.rs", "task.rs", "inputs.rs"))

MUTANTS = {
    "R1-M1-context-fit-slack": [(SUBJECT, "    if weighted > MAX_CONTEXT_BYTES * 4 {", "    if weighted > MAX_CONTEXT_BYTES * 4 + 5 {")],
    "R1-M2-prompt-bound-slack": [(TASK, "    if task.len() + HOST_SUFFIX_BYTES > MAX_PROMPT_BYTES {", "    if task.len() + HOST_SUFFIX_BYTES > MAX_PROMPT_BYTES + 1 {")],
    "R1-M3-tree-unchecked": [(SUBJECT, "    if tree != expected.tree {", "    if false && tree != expected.tree {")],
    "R1-M4-subject-equality-unchecked": [(CHECK, "                digest_ok && fields_ok,", "                true || (digest_ok && fields_ok),")],
    "R1-M5-jwt-rule-never-matches": [(SECRETS, r'r"|eyJ[A-Za-z0-9_-]{10,}\.eyJ",', r'r"|eyJ[A-Za-z0-9_-]{10,}\.eyQ",')],
    "R1-M6-snapshot-served-unchecked": [(CHECK, "        pinned && served.as_ref() == Some(&record.snapshot_manifest),", "        pinned || served.as_ref() == Some(&record.snapshot_manifest),")],
    "R1-M7-body-model-unchecked": [(CHECK, '        if parsed["model"] != REQUESTED_MODEL {', '        if false && parsed["model"] != REQUESTED_MODEL {')],
    "R1-M8-user-text-length-unchecked": [(CHECK, "        if texts[0].len() != task.len() + task::HOST_SUFFIX_BYTES {", "        if false && texts[0].len() != task.len() + task::HOST_SUFFIX_BYTES {")],
    "R1-M9-read-budget-slack": [(SUBJECT, "    if needed > u64::from(max_reads) {", "    if needed > u64::from(max_reads) + 1 {")],
    "R1-M10-env-member-path-missed": [(SECRETS, '    base.starts_with(".env")', '    base.starts_with(".envx")')],
    "R1-M11-binary-patch-rule-never-matches": [(SECRETS, r'rule(BINARY_PATCH, r"^GIT binary patch\s*$", None),', r'rule(BINARY_PATCH, r"^GIT binary patchX\s*$", None),')],
    "R1-M12-dotdot-component-allowed": [(SUBJECT, '|| c == ".." ||', '|| c == "..." ||')],
    "R1-M13-job-id-unchecked": [(CHECK, "        job_id == Some(record.job_id.as_str())", "        true")],
    "R1-M14-attempt-digest-unchecked": [(CHECK, "                .any(|d| d.as_deref() != Some(&digest))", "                .any(|d| d.is_none())")],
    "R1-M15-staging-not-removed": [(INPUTS, "        let _ = std::fs::remove_dir_all(&staging);", "        let _ = &staging;")],
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
            files.setdefault(str(path), path.read_bytes())
        for index, data in enumerate(files.values()):
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
            "files": [{"path": p, "backup": f"orig-{i}.bak", "original_sha256": sha(d),
                       "mutated_sha256": sha(texts[p].encode())} for i, (p, d) in enumerate(files.items())],
            "edits": [{"path": str(p), "old": o, "new": n} for p, o, n in edits],
        }
        record_path.write_text(json.dumps(record, indent=1) + "\n")
        print(f"applied {mutant}: " + ", ".join(f["mutated_sha256"][:12] for f in record["files"]))
    elif action == "restore":
        record = json.loads(record_path.read_text())
        for entry in record["files"]:
            pathlib.Path(entry["path"]).write_bytes((out / entry["backup"]).read_bytes())
            if sha(pathlib.Path(entry["path"]).read_bytes()) != entry["original_sha256"]:
                sys.exit(f"{mutant}: restore mismatch for {entry['path']}")
        print(f"restored {mutant}: " + ", ".join(f["original_sha256"][:12] for f in record["files"]))
    else:
        sys.exit("usage: mutate.py apply|restore <mutant-id>")


if __name__ == "__main__":
    main()
