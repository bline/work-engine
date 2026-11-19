# A2 remediation-1 RESULT

Status: **done. All conflicts resolved, P2-1 and P3-1 addressed, 11/11 mutants killed, gate-3 15/15 passed.**
Worktree `/home/bline/cbm-work/wt-org-owners-a2-20261118`, branch `org-owners-a2-20261118`, HEAD `3c840c36` (unchanged).
Edits only; no mutating git; nothing staged (`git diff --cached` empty); no conflict markers remain
(`grep '^<<<<<<<\|^>>>>>>>'` over `rust` and `docs` finds none). Usage/cost: unknown to me.

## Conflicts (10 hunks, 6 files) and resolutions

| File | Hunk | Resolution |
|---|---|---|
| S-A `Cargo.toml` | description | Both: "Gate O subset (O3, OA, O4, O5, O8, O9), Gate R (R1-R4), S1 (scope digest is the exact-subject digest) and effect-admission states". |
| S-A `src/gate.rs` | module doc | A2 text with A1's "S1" kept: "Gate O subset (O3, OA, O4, O5, O8, O9), Gate R (R1-R4), S1, ...". |
| S-A `src/gate.rs` | `SubjectRef::scope_digest` + `OperationRequest` doc | A1's `impl SubjectRef { scope_digest }` kept verbatim; `OperationRequest` doc is A2's (names no organization, OA derives it). |
| S-A `src/lib.rs` | `admit` module doc | Both: "O3, OA, O4, O5, O8, O9 ... S1 (exact-subject digest) ... the organization is derived from the committed admission of the candidate (OA), never supplied". |
| S-A `tests/admission.rs` | `request_with` | A2 signature (no org) with A1's `subject()` helper: `OperationRequest::new(requirements, digest, VANTAGE, subject())`. |
| S-A `tests/admission.rs` | `g2_o3_no_derived_candidate` expected set | Union: OA, O4, O5, O8, O9, R1, R2 `CandidateUnavailable`; R3 `VantageUnbound` (A2); S1 `CandidateUnavailable` (A1). |
| S-A `tests/admission.rs` | `request_at` | Same as `request_with`: no org, `subject()`. |
| S-A `tests/admission.rs` | tail of file | A1's S1 test block and A2's OA/state block both kept (they were two appended blocks sharing one closing brace). |
| delivery `src/gate.rs` | `discharge` | `ObserverIdentity = [OA, R2, R3]` (A2); `ScopeDigestToSubject = Admitted([S1])`; `ProblemScopeToEpisodeIdentity` and `ConsumptionEvidenceMapping` stay `AssumedDischarge` (A1). Separation `[OA,O8,R3]`, ComponentBinding `[OA,O4,R3]` (A2, already merged cleanly). |
| delivery `tests/gate.rs` | imports | A1's `operation_request_from`, ORG dropped (no longer used); added the new helper `authority_world_admitting`. |

Auto-merged-but-semantically-broken places also fixed (not conflict markers):
- S-A `CheckId::ALL` was `[CheckId; 10]` holding 11 entries (O3, OA, O4, O5, O8, O9, R1-R4, S1): now `[CheckId; 11]`.
  Both S1 and OA are in the enum, `ALL`, `evaluate`, and the no-candidate `CandidateUnavailable` list.
- A1's S-A S1 tests still used the 5-argument `OperationRequest::new(ORG, ..)` and the removed `Req.obligation_digest`
  (one A2 test used it): updated to the 4-argument form and `scope_digest: "b".repeat(64)`.
- `s1_the_base_admission_passes_s1_and_lists_every_check`: id list now includes OA.
- Test worlds whose request differs from the default now admit their own request's terms (the A2 rule), so each A1
  S1 negative keeps its "refuses S1 alone" claim: `s1_a_patch_identity_stand_in_scope_digest_refuses_only_s1` and
  `s1_compares_the_obligation_the_vantage_serves` use `Opts { admitted: Terms::of(&req) }`. In delivery,
  `the_frozen_stand_in_scope_digest_refuses_only_s1` uses a new helper `authority_world_admitting(&request)`
  (`tests/common/mod.rs`: `organization_admission(org, &request)`, `issue_admitting`); `issue_with` delegates to
  it with the assumption world's request, so every other delivery world is unchanged.
- The A1 world's scope digest is the exact-subject digest; the admission's problem scope is unchanged by A1, so
  the delivery/preparation worlds (`organization_admission`) needed no further change.

`cargo fmt` was run on the three crates (formatting only).

## P2-1 (owner definition truthfulness)

`records.rs`, `OrganizationAdmission` doc, chain bullet: now states that the chain is the issuer's obligation, that
at fixture standing the gate does not check the predecessor chain (revision + 1, same organization, predecessor
superseded by this issuance) or per-organization uniqueness, that `OA` refuses only when more than one committed
admission matches the same candidate terms (possibly of different organizations), that an issuer outside fixture
standing requires that check first, and the owner question (admissions keyed by problem scope including the slice
number; one head per organization would block concurrent slices). `a2/RESULT.md` has a new section
"Known gap: admission chain" saying the same. No gate behaviour changed.

## P3-1 (ambiguity at regate)

`tests/admission.rs::state_refused_when_a_second_admission_of_the_candidate_is_committed_after_admission`: admits,
asserts `permits_effect`, issues a second matching admission (`org-2`), then asserts
`AdmissionState::Refused(outcomes)` with the exact set `unadmitted(OrganizationHeadsAmbiguous)`: OA
`OrganizationHeadsAmbiguous` (candidate digest hex), O8 `SeparationUnresolved` x4, R3 `VantageUnbound`; refused ids
`[OA, O8, R3]`. Passes.

## Mutants (custody, `mutants/runs-2/`, plan `mutants/mutant-plan-2.json`, summary sha256 `5e339d92b660004cdd03a9021c2d60f9f9c2e2e4420436b7446749019b5447f5`)

- 11 mutants (A2-M1..M10 regenerated against the rebased text; `mutate.py` M9 anchor updated to `[CheckId; 11]`;
  runs-1 mutation records archived to `mutants/runs-1/mutation-records/`). New **A2-M11-regate-ignores-organization-refusals**
  (regate treats refusals confined to OA/O8/R3 as Current), mapped to the P3-1 test.
- Each mutant: exact-once replacement (apply fails otherwise) -> mapped test(s) -> exact restore. Every mapped test
  ended `test result: FAILED. 0 passed; 1 failed` (13 test steps; 0 `error[` compile kills). **All 11 killed.**
- Summary reads `failed 24/37` by design (the 13 kill steps exit 101; the 24 apply/restore/hash steps exit 0).
- Pre-hashes = post-hashes: S-A `gate.rs` `bb35a376...`, delivery `gate.rs` `b73703bb...`. Original sha256 per mutant
  in each `mutants/<id>/mutation.json` (S-A gate `089ac728...`, delivery gate `31f71982...`).

## Gate (custody, `runs/gate-3/`)

Plan `a2/root-gate-plan.json` (cwd worktree `rust`, `CARGO_TARGET_DIR=.../a2/target`; `hashes` step updated to the 11
current files; previous plan kept as `remediation-1/root-gate-plan.before.json`). Summary `passed 15/15`, sha256
`f18b9cf47e4b7760a5a70c790e3070739767ad835c42cbd90491a9fcddf08245`. Counts: test-four-plus-sa 320 passed;
test-preparation-fixture 42; test-default-features 100; S-A `--doc` 8; authority-store 37; native-1 33; fmt, three
clippy sets and the preparation lib check clean (`-D warnings`); hosted `cargo check` passes; git-status shows only the
11 modified files. Gate hashes for both mutated files equal the mutant pre/post hashes (gate.rs bb35a376.., b73703bb..).

## Diff stat vs `3c840c36` (11 files, 686 insertions, 149 deletions)

S-A `Cargo.toml` 2, `gate.rs` 160, `lib.rs` 29, `records.rs` 98, `tests/admission.rs` 351; delivery `gate.rs` 47,
`native.rs` 14, `tests/common/mod.rs` 50, `tests/gate.rs` 21, `tests/native_vertical.rs` 36; preparation
`tests/common/mod.rs` 27. `git diff HEAD | sha256sum` = `402a2a54cfe2a86824a44dea8a60849832484e19c567a28d38886c8fd033687d`.

## Deviations / notes

1. Added the delivery test helper `authority_world_admitting` and an `issue_admitting` split in `tests/common/mod.rs`
   (needed so A1's stand-in S1 negative keeps refusing S1 alone under A2's admission terms).
2. `CheckId::ALL` array length fixed (auto-merge left a type error); not a conflict marker.
3. P3-2 (match derived candidate digest) not done; optional and not requested.
4. `cargo test --workspace --all-features` fails to compile `review-execution-evidence` (`claude_guarded` test:
   missing `Fixture`/`QualifiedScope`); a different crate, not touched, not in the gate plan. Not investigated further.
5. Mutant plan step names and `mutants/` layout follow the S1/A2 convention; `mutate.py`/`gen_plan.py` were edited
   in place (M9 anchor, M11 added).
