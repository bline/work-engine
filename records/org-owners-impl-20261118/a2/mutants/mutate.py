"""Apply or restore one recorded A2 mutant by exact text replacement.

Adapted from records/org-hardening-owners-20261117/s1/mutants/mutate.py (same mechanism) for the
A2 worktree (OrganizationAdmission v1, S-A check OA).

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
CRATES = pathlib.Path("/home/bline/cbm-work/wt-org-owners-a2-20261118/rust/crates")
SA_GATE = CRATES / "work-engine-organization-authority/src/gate.rs"
DELIVERY_GATE = CRATES / "work-engine-review-delivery/src/gate.rs"

MUTANTS = {
    "A2-M1-first-of-two-heads": [(SA_GATE,
        "        [(id, admission)] => {",
        "        [(id, admission), ..] => {")],
    "A2-M2-skip-pin": [(SA_GATE,
        "            pins.organization_admissions.insert(*id);\n",
        "")],
    "A2-M3-ignore-scope": [(SA_GATE,
        "\n                    && a.problem_scope == *scope =>",
        " =>")],
    "A2-M4-ignore-candidate": [(SA_GATE,
        "                if a.candidate_content_digest == request.candidate_digest\n                    && a.requirements_digest",
        "                if a.requirements_digest")],
    "A2-M5-ignore-requirements": [(SA_GATE,
        "\n                    && a.requirements_digest == requirements_digest",
        "")],
    "A2-M6-bindings-by-vantage-only": [(SA_GATE,
        "if Some(b.organization) == organization && b.vantage == vantage =>",
        "if b.vantage == vantage =>")],
    "A2-M7-point-check-ignores-organization-pins": [(SA_GATE,
        "        self.organization_admissions\n            .iter()\n            .chain(&self.principals)",
        "        self.principals\n            .iter()")],
    "A2-M8-revoked-admission-counts": [(SA_GATE,
        "    let found: Vec<(IssuanceId, &OrganizationAdmission)> = state\n        .entries()\n        .filter(|(_, e)| e.status == IssuanceStatus::Committed)\n",
        "    let found: Vec<(IssuanceId, &OrganizationAdmission)> = state\n        .entries()\n")],
    "A2-M9-oa-dropped-from-all": [(SA_GATE,
        "    pub const ALL: [CheckId; 11] = [\n        CheckId::O3,\n        CheckId::OA,\n",
        "    pub const ALL: [CheckId; 10] = [\n        CheckId::O3,\n")],
    "A2-M10-separation-discharge-drops-oa": [(DELIVERY_GATE,
        "BlockerKind::Separation => Discharge::Admitted(&[CheckId::OA, CheckId::O8, CheckId::R3]),",
        "BlockerKind::Separation => Discharge::Admitted(&[CheckId::O8, CheckId::R3]),")],
    "A2-M11-regate-ignores-organization-refusals": [(SA_GATE,
        "        if regate.outcomes.all_passed() {",
        "        if regate.outcomes.refused().into_iter().all(|id| {\n            matches!(id, CheckId::OA | CheckId::O8 | CheckId::R3)\n        }) {")],
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
