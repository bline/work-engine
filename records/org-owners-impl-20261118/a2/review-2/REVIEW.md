# A2 focused review-2: rebase onto A1 and remediation-1

Reviewer: fresh-entry adversarial code reviewer, read-only. No edits, staging or commits; no cargo runs.
Scope: whether the rebase of A2 onto merged A1 (`3c840c36`) and remediation-1 kept both slices' semantics.
This review does not re-review A2's design or A1.

## Verdict: **accept**

The rebase keeps both slices' semantics. Every conflict resolution is the union of A1 and A2. The
auto-merged `CheckId::ALL` type error is fixed. S1 and OA are both present in `CheckId::ALL`, in both
branches of `evaluate` and in the regate. The delivery `discharge` table is correct for all nine kinds.
No test was deleted, no assertion was removed and no expected set was widened. I found no P1 or P2.
The three P3s are comment or record wording only. None changes behaviour, so the root can fold them in
at its discretion.

## Findings

### P3-1: a stale count in the doc of an A1 test ("ten check ids")

- Where: `rust/crates/work-engine-organization-authority/tests/admission.rs:1658-1659`. The doc comment
  is on `s1_the_base_admission_passes_s1_and_lists_every_check` (line 1661).
- Evidence: the doc says "the admission's outcome list is exactly the ten check ids (S1 included)".
  The rebased assertion on line 1666 lists eleven: `[O3, OA, O4, O5, O8, O9, R1, R2, R3, R4, S1]`. The
  remediation updated the assertion but not its doc.
- Fix: change "ten" to "eleven", or reword it to "every check id (OA and S1 included)".

### P3-2: an inaccurate new comment in `s1_compares_the_obligation_the_vantage_serves`

- Where: `tests/admission.rs:1782`, the comment "The owner admitted this candidate, so only S1 can differ
  between the two vantages."
- Evidence:
  - The world binds only `VANTAGE` (`"vantage:ob-a"`, line 23) and `vantage-builder` (`world()`, lines
    353-368). At `vantage:ob-b`, R3 therefore refuses `VantageUnbound`, but at `vantage:ob-a` it
    passes. So S1 is not the only check that differs between the two vantages.
  - The test is still sound, because `outcome_at` reads `CheckId::S1` only, both from `Ok` and from
    `Err(Checks)`. This test never claimed "S1 alone", so the world change is harmless but not needed
    for its assertion.
  - RESULT.md ("Auto-merged-but-semantically-broken places") lists this test among those that keep a
    "refuses S1 alone" claim. That overstates it slightly.
- Fix: reword the comment to "The owner admitted this candidate; the test reads S1 alone, which differs
  between the two vantages". Alternatively, revert to `world(Opts::default())`, which is A1's text; OA
  does not affect S1.

### P3-3: wording precision in the P2-1 doc and in the RESULT "Known gap"

- Where: `src/records.rs:115` and `a2/RESULT.md:197-200`.
- Evidence:
  - `records.rs:115` says "`OA` refuses only when more than one committed admission matches the same
    candidate terms". Read literally, that is false: `oa` (`src/gate.rs:717-755`) also refuses
    `OrganizationUnadmitted` when no admission matches. The next bullet (`records.rs:123-126`) states
    both outcomes correctly, and the sentence clearly means "the only uniqueness check is per candidate
    terms". Read in context, the doc is accurate.
  - RESULT.md:197-200 says that a rev-2 admission issued without superseding, a bad predecessor or
    organization id, a non-consecutive revision, or a duplicate rev-1 admission "are all admitted
    exactly like a valid one". That holds only when the malformed admission is the unique committed
    match for its terms. Suppose it shares its terms with another committed admission, for example a
    same-terms predecessor left committed or a same-terms duplicate rev-1. OA then refuses
    `OrganizationHeadsAmbiguous`, as a side effect of matching on terms rather than as a chain check.
    The first review carried this caveat ("if the two admit different candidates"); the RESULT text
    dropped it.
  - The substance of both texts is accurate. The gate never reads `revision`, `predecessor` or
    `organization_id` for validation (grep over `src/`: they appear only in the record definition,
    accessors, the constructor and `body_digest`). The constructor exists only under `fixture`, and the
    owner question about the slice key is stated correctly.
- Fix (optional): in records.rs, write "OA's only uniqueness check is that no more than one committed
  admission matches the candidate terms; those may belong to different organizations." In RESULT, add
  "…whenever it is the only committed admission with its terms".

## Judgement by packet question

### 1. Conflict resolutions and the auto-merged-but-broken fixes are correct

Method: I rebuilt the pre-rebase A2 tree, which is `f772612f` plus `a2-1.patch` (it applies cleanly),
in `/tmp`. I then ran `git merge-file` for each file, with A2-pre as ours, `f772612f` as the base and
`3c840c36` as theirs. The result has exactly 10 conflict hunks in 6 files (Cargo.toml 1, S-A gate.rs 2,
lib.rs 1, admission.rs 4, delivery gate.rs 1, delivery tests/gate.rs 1), which matches the RESULT table.
I then diffed that mechanical merge against the worktree.

- Every conflict hunk resolves to the union: the Cargo description, the gate.rs module doc, the lib.rs
  `admit` doc, `request_with` and `request_at` (4-argument form with A1's `subject()`), the delivery
  imports, and the `discharge` table.
- `SubjectRef::scope_digest` is A1's text verbatim, and the `OperationRequest` doc is A2's.
- `g2_o3_no_derived_candidate…` (`admission.rs:665-697`) expects OA, O4, O5, O8, O9, R1 and R2
  `CandidateUnavailable`, R3 `VantageUnbound` and S1 `CandidateUnavailable`. That is the exact union.
- `CheckId::ALL` (`gate.rs:67-79`) is `[CheckId; 11]` and holds O3, OA, O4, O5, O8, O9, R1-R4 and S1.
  This was the only non-marker source fix.
- `evaluate`:
  - The no-candidate list (`gate.rs:557-568`) contains OA, O4, O5, O8, R1, R2 and S1. O3, O9, R4 and
    R3 are decided outside the match, R3 with no organization.
  - The `Some` branch inserts OA (`:572-583`) and S1 (`:596`). The function panics unless every id in
    `ALL` was evaluated.
- Regate: `admission_state` (`gate.rs:1476-1485`) re-runs the same `evaluate`, so OA and S1 are both
  in it.
- Delivery `discharge` (`review-delivery/src/gate.rs:98-117`):

  | Kind | Discharge |
  |---|---|
  | Separation | `[OA, O8, R3]` |
  | ComponentBinding | `[OA, O4, R3]` |
  | AuthorityRequirement | `[O4]` |
  | RealizationAdmission | `[R1, R2, R4]` |
  | ObserverIdentity | `[OA, R2, R3]` |
  | ScopeDigestToSubject | `Admitted([S1])` |
  | ProblemScopeToEpisodeIdentity, ConsumptionEvidenceMapping | `AssumedDischarge` |
  | RealizationCheck | `Undischarged` |

  This is A2's OA additions plus A1's S1 row, across all 9 `BlockerKind::ALL` kinds. The whole table
  is pinned by `discharge_is_total_admitted_kinds_name_their_check_ids` (`tests/gate.rs:32-77`).
- The other five files merged cleanly: records.rs, native.rs, delivery `tests/common/mod.rs`,
  `native_vertical.rs` and preparation `tests/common/mod.rs`. Apart from the P2-1 doc and the delivery
  helper split, they are byte-identical to the mechanical merge.
  - In preparation's world, A2's organization admission is computed from the request after A1's scoped
    requirements are used (`issue(&src, &request, …)` comes after the request is built). The two
    slices compose correctly there.

### 2. Negatives still isolate their checks, and no test was weakened

- `assert_exact` (`admission.rs:456-470`) iterates over `CheckId::ALL`. Every exact negative therefore
  now also requires OA and S1 to pass unless they are listed.
- A1's S1 negatives:
  - `s1_a_patch_identity_stand_in_scope_digest_refuses_only_s1` (`:1724`) builds its world after its
    request, with `admitted: Terms::of(&req)`. `assert_exact` still lists S1 alone. This keeps A1's
    claim and slightly sharpens it: S1 refuses the stand-in even when the owner admitted that candidate.
  - Delivery `the_frozen_stand_in_scope_digest_refuses_only_s1` (`tests/gate.rs:258`) uses
    `authority_world_admitting(&request)` and still asserts `outcomes.refused() == [S1]` with the exact
    reason.
    - `authority_world_admitting` has the same shape as `authority_world()` (owner grant, no expiry, no
      swap).
    - `issue_with` now delegates to `issue_admitting(…, &operation_request())`. That is the same record
      the old `organization_admission(ORG)` built internally, so every other delivery world is
      unchanged.
  - `s1_a_subject_differing_in_each_field_refuses_only_s1` is unchanged apart from dropping `ORG`. It
    uses the base candidate, so OA passes and the claim holds.
  - `s1_compares…` reads S1 only (see P3-2). `s1_a_vantage_that_serves_no_obligation…` is unchanged.
- A2's negatives:
  - The A2 tail block is byte-identical to pre-rebase A2 with two exceptions: `obligation_digest: 'b'`
    became `scope_digest: "b".repeat(64)`, which yields the same 64-character `b` digest that the old
    `char.repeat(64)` produced, and the new P3-1 test was added.
  - Each A2 negative uses `Req::default()`, whose `scope_digest` is now A1's exact-subject digest, so S1
    passes. `unadmitted(...)` still lists exactly OA, O8 and R3.
  - `native_vertical.rs:939` (`[OA, O8, R3]`) is unchanged.
- Test counts:
  - `admission.rs`: base 43, A1 50, A2-pre 49, rebased 57. That is 43 + 7 + 6 + 1 (P3-1).
  - Delivery `tests/gate.rs`: 9, 10, 9, 10. `native_vertical.rs`: 33 in all four trees.
- Assertion-line counts are additive:
  - `admission.rs`: 85, then +12 (A1) and +13 (A2), then +2 (P3-1), giving 112.
  - Delivery `gate.rs`: 28, then +4 (A1), giving 32.
  - `native_vertical.rs`: 332, then +4 (A1) and +2 (A2), giving 338.
- Gate-3 `test-four-plus-sa` shows `57 passed` for `admission`.

### 3. P2-1, P3-1 and A2-M11

- P2-1: the doc and the Known-gap text are accurate in substance (see P3-3 for wording). This is the
  minimum fix the first review required, and no gate behaviour changed.
- P3-1: `state_refused_when_a_second_admission_of_the_candidate_is_committed_after_admission`
  (`admission.rs:1921-1940`) asserts the exact outcome.
  - It first asserts `permits_effect()`, then issues a same-terms `org-2` admission, which leaves every
    pin committed.
  - It then requires `AdmissionState::Refused(outcomes)` with
    `assert_exact(&outcomes, &unadmitted(OrganizationHeadsAmbiguous))`. Those are the exact reasons
    for OA, O8 (four subjects) and R3, with every other check passing. That is stricter than the
    `refused() == [OA, O8, R3]` the first review asked for.
- A2-M11 targets a real claim. The edit replaces `if regate.outcomes.all_passed()` with
  "refused ⊆ {OA, O8, R3}".
  - It is the only test path where the regate refuses on an organization change that touches no pin.
    Revocation and supersession return before the regate.
  - The kill record `runs-2/A2-M11-….test0.stdout` shows `expected refused, got Current {…}` at
    `admission.rs:1936` and `0 passed; 1 failed`, with exit code 101. The kill is behavioural, not a
    compile error.
  - `orig-0.bak` has sha256 `089ac728…`, which equals the current S-A `gate.rs`. The pre-hashes
    (`bb35a376…`, `b73703bb…`) are git blob ids and equal `git hash-object` of the current files.

### 4. Nothing outside A2 scope changed

- `git diff 3c840c36` matches `a2-2.patch` byte for byte (sha256 `402a2a54…`). Both patches touch the
  same 11 files.
- Nothing is staged, there are no untracked files and there are no conflict markers.
- I compared A2's +/- lines before the rebase (`f772612f` to A2-pre) with those after it (`3c840c36`
  to the worktree), file by file. The only differences are:
  - the merged doc and description texts;
  - the `ALL` length;
  - the P2-1 doc;
  - the removal of `ORG` in A1's S1 tests;
  - the two S1-test world adaptations and the delivery stand-in world;
  - the `ids` list gaining OA;
  - `obligation_digest` becoming `scope_digest`;
  - the P3-1 test;
  - the delivery helper split (`organization_admission(org, &request)`, `issue_admitting`,
    `authority_world_admitting`) and its import.
- A2's deltas in delivery `src/gate.rs`, `native.rs`, `native_vertical.rs` and preparation
  `tests/common/mod.rs` are identical before and after the rebase.
- Gate-3 `hashes.stdout` equals `git hash-object` of all 11 current files, so gate-3 ran over these
  exact bytes.

## Not verified

- I did not run cargo. I relied on gate-3 (summary `passed`; its hashes match the worktree) and on the
  M11 run record.
- A1's mutants were not re-run after the rebase. By reading, each A1 kill test keeps its assertions, so
  the kills should still hold:
  - A1-M1 (S1 always passes) and A1-M2 (digest ignores commit and tree): the S1 negatives still assert
    S1 refusals.
  - A1-M3 (first-candidate obligation): `s1_compares…` still asserts both vantages.
  - A1-M4: the discharge-table test.
  - A1-M5: the `ids` list.
  - A1-M6 and A1-M7: the vantage tests are unchanged.
- I did not re-derive the M1-M10 regenerated edit strings beyond the RESULT summary.
