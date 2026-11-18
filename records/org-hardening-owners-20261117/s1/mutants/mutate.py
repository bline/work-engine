"""Apply or restore one recorded S1 mutant by exact text replacement.

Adapted from org-r2-core-20261117/sb/mutants/mutate.py (same mechanism) for the S1 worktree.

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
CRATES = pathlib.Path("/home/bline/cbm-work/wt-org-hardening-20261117/rust/crates")
GATE = CRATES / "work-engine-review-delivery/src/gate.rs"
NATIVE = CRATES / "work-engine-review-delivery/src/native.rs"
CONSUMER = CRATES / "work-engine-review-preparation/src/fixture.rs"

# A GateCheck fabricated from the proof's admission (used by mutants that let the consumer pass
# without a current point check).
FAKE_CHECK = (
    "GateCheck {\n"
    "            boundary: admission.boundary().clone(),\n"
    "            admission_revision: admission.revision(),\n"
    "            regate_revision: admission.revision(),\n"
    "            regate_now_millis: proof.regate_now_millis,\n"
    "        }"
)

MUTANTS = {
    # --- S-B M1-M11 re-specified on the current text (M5 retired: replaced claim) ---
    "M1-admitted-family": [(GATE,
        "BlockerKind::Separation => Discharge::Admitted(&[CheckId::O8, CheckId::R3]),",
        "BlockerKind::Separation => Discharge::AssumedDischarge,")],
    "M2-assumed-family": [(GATE,
        "        BlockerKind::ScopeDigestToSubject\n        | BlockerKind::ProblemScopeToEpisodeIdentity",
        "        BlockerKind::ScopeDigestToSubject => Discharge::Admitted(&[CheckId::O3]),\n        BlockerKind::ProblemScopeToEpisodeIdentity")],
    "M3-realization-check": [(GATE,
        "BlockerKind::RealizationCheck => Discharge::Undischarged,",
        "BlockerKind::RealizationCheck => Discharge::AssumedDischarge,")],
    "M4-refusal-after-spawn": [
        (NATIVE,
         "    let mut delivery = admit_at(BoundaryKind::Delivery).map_err(before_spawn)?;",
         "    let mut delivery = match admit_at(BoundaryKind::Delivery) {\n        Ok(d) => d,\n        Err(refusal) => {\n            let _ = run_native(cfg, binding, &crate::gate::mutant_assumed(binding));\n            return Err(before_spawn(refusal));\n        }\n    };"),
        (GATE,
         "/// One admission the host obtained itself at one reserved boundary, held for its regate.\n",
         "pub(crate) fn mutant_assumed(binding: &ExperimentBinding) -> AssumedDischarges {\n    AssumedDischarges { boundary: Boundary::BeforeSpawn, bound_to: binding.frozen().contract_sha256().to_owned(), assumed: vec![] }\n}\n\n/// One admission the host obtained itself at one reserved boundary, held for its regate.\n"),
    ],
    # runs-1 used a weaker spec (replayed only on the first regate, then fresh): the gate test,
    # which regates twice, passed under it. runs-2 replays on every regate, as S-B M6 did; the
    # observation handed to the consumer stays the fresh one (the mutant is the host's decision).
    "M6-replay-admission-observation": [
        (GATE,
         "    settled: Option<AdmissionState>,\n}",
         "    settled: Option<AdmissionState>,\n    replay: Result<SealedObservation, Unobserved>,\n}"),
        (GATE,
         "        Ok(Self {\n            source,\n            clock,\n            reservation,\n            admission,\n            settled: None,\n        })",
         "        let replay = observe_for(source, &reservation);\n        Ok(Self {\n            source,\n            clock,\n            reservation,\n            admission,\n            settled: None,\n            replay,\n        })"),
        (GATE,
         "        let state = self.admission.admission_state(&fresh, now_millis);",
         "        let state = self.admission.admission_state(&self.replay, now_millis);"),
    ],
    "M7-no-regate": [(NATIVE,
        "        (AdmissionState::Current { revision }, Some(inputs)) => Ok((discharges, revision, inputs)),\n        (other, _) => Err(GateRefusal::NotCurrent(other)),",
        "        (_, Some(inputs)) => Ok((discharges, held_consumption.admission().revision(), inputs)),\n        (other, None) => Err(GateRefusal::NotCurrent(other)),")],
    "M8-unknown-permits": [(NATIVE,
        "        (AdmissionState::Current { revision }, Some(inputs)) => Ok((discharges, revision, inputs)),\n        (other, _) => Err(GateRefusal::NotCurrent(other)),",
        "        (AdmissionState::Current { revision }, Some(inputs)) => Ok((discharges, revision, inputs)),\n        (AdmissionState::Unknown(_), Some(inputs)) => Ok((discharges, held_consumption.admission().revision(), inputs)),\n        (other, _) => Err(GateRefusal::NotCurrent(other)),")],
    "M9-no-finality": [(GATE,
        "        if let Some(settled) = &self.settled {\n            return (settled.clone(), None);\n        }\n",
        "")],
    "M10-public-run-attempt": [(NATIVE,
        "pub(crate) fn run_attempt(",
        "pub fn run_attempt(")],
    "M11-public-assess": [(GATE,
        "pub(crate) fn assess(",
        "pub fn assess(")],
    # --- S1 new claims ---
    "S1-P1-other-candidate": [(CONSUMER,
        "        return mismatch(\"candidate digest\");",
        "        let _ = mismatch(\"candidate digest\");")],
    "S1-P2-other-subject": [(CONSUMER,
        "        return mismatch(\"subject\");",
        "        let _ = mismatch(\"subject\");")],
    "S1-P3-other-boundary": [(CONSUMER,
        "        return mismatch(\"boundary kind\");",
        "        let _ = mismatch(\"boundary kind\");")],
    "S1-P4-missing-proof-accepted": [(CONSUMER,
        "        other => Err(ConsumeRefusal::GateProofNotCurrent(other)),",
        "        AdmissionState::Unknown(_) => Ok(" + FAKE_CHECK + "),\n        other => Err(ConsumeRefusal::GateProofNotCurrent(other)),")],
    "S1-P5-skip-proof-check": [(CONSUMER,
        "    let gate = check_proof(candidate, declaration, proof)?;",
        "    let admission = proof.admission;\n    let gate = check_proof(candidate, declaration, proof).unwrap_or(" + FAKE_CHECK + ");")],
    "S1-P6-test-support-under-fixture": [(NATIVE,
        "#[cfg(feature = \"test-support\")]\n#[doc(hidden)]\npub mod test_support {",
        "#[cfg(feature = \"fixture\")]\n#[doc(hidden)]\npub mod test_support {")],
    "S1-P7-delivery-regate-revision-dropped": [(NATIVE,
        "        AdmissionState::Current { revision } => admitted_before_spawn.with_regate(revision),",
        "        AdmissionState::Current { revision: _ } => admitted_before_spawn,")],
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
