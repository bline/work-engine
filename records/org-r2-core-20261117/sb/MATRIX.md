# S-B matrix: invariant → tests → mutant

Source: worktree `/home/bline/cbm-work/wt-org-r2-core-20261117` (uncommitted, on top of S-A checkpoint
`bfaa455c`); `src/gate.rs` blob `3d4ab245`, `src/native.rs` blob `7566332c` (crate
`work-engine-review-delivery`). Gate run `runs/gate-1` (8/8 passed). Mutants `mutants/runs-1` (M1–M6,
M8–M11) and `mutants/runs-2` (M7; the runs-1 M7 attempt did not compile and is not counted). Each mutant was
applied by `mutants/mutate.py` (exact-once text replacement, original bytes saved), its mapped test run under
custody, then restored; post-restore blob hashes equal the pre-mutation ones (`final-hashes` step in both runs).
n = 1 mutant per row.

Test files: `G` = `tests/gate.rs` (no native child), `N` = `tests/native_vertical.rs` (real Pi SDK 1.5.0
child where a spawn occurs).

| # | Invariant | Tests (all pass in gate-1) | Mutant (neutralization) | Mapped test result under mutant |
|---|---|---|---|---|
| 1a | Discharge mapping, admitted family (Separation→O8+R3, ComponentBinding→O4+R3, AuthorityRequirement→O4, RealizationAdmission→R1+R2+R4, ObserverIdentity→R2+R3) | G `discharge_is_total_admitted_kinds_name_their_check_ids`; G `the_assumption_world_screens_into_admitted_and_assumed_mapping_kinds`; N positive `canonical_preparation_reaches_the_real_sdk_and_the_existing_consumer` (admitted ids with check ids in `Delivered`) | M1: Separation → `AssumedDischarge` | FAILED (gate.rs:35) — killed |
| 1b | Mapping kinds stay labelled assumptions (O7) | same G table test; positive N test asserts assumed set = mapping kinds only | M2: ScopeDigestToSubject → `Admitted(&[O3])` | FAILED — killed |
| 1c | RealizationCheck never dischargeable | same G table test; G `an_unestablished_realization_check_has_no_discharge_and_refuses`; G/N frozen-world tests | M3: RealizationCheck → `AssumedDischarge` | FAILED — killed |
| 2a | Admission at delivery: host reserves delivery boundary, observes, `admit::<Fixture>`; any refusal refuses before spawn (spawn 0, consumer 0, fixture root untouched) | N `sb_an_admission_refusal_refuses_before_spawn` (revoked owner grant → O4 refusal; unobservable source → `NoObservation`); G `the_fixture_authority_world_admits_and_an_empty_world_refuses` | M4: on delivery-admission refusal, spawn first (`run_native`) and then return the refusal | FAILED (native_vertical.rs:823, spawn count) — killed |
| 2b | **REPLACED CLAIM (remediation 1).** F-7 digest comparison (candidate, requirements, subject, realization) with the binding, before spawn | ~~N `sb_an_admission_for_another_subject_or_candidate_refuses_before_spawn`~~ retired: the inputs it varied (`FixtureAuthority.request`) no longer exist | ~~M5: `compare_with_binding` returns `Ok(())`~~ retired: `compare_with_binding` is deleted (after host construction every branch was a tautology). Not coverage kept. | superseded by T1-T7 below |
| 3a | Regate before consumption reads a **fresh** observation (F-2); replaying the admission-time observation must not satisfy it | N `sb_revocation_supersession_or_missing_grant_between_boundaries_refuses_consumption`; G `regate_reads_a_fresh_observation` | M6: `HeldAdmission` takes a second observation at admission time and the regate replays it | both mapped tests FAILED — killed |
| 3b | Revocation / supersession / missing grant between spawn and consumption → consumer 0 | N `sb_revocation_supersession_or_missing_grant_between_boundaries_refuses_consumption` (`Revoked([owner grant])`, `Stale(IssuanceSuperseded([reviewer binding]))`, `Unknown(IssuanceRetracted([owner grant]))`; spawn 1, complete verified run, consumer 0) | M7: regate replaced by `Current { revision: admission.revision() }` | FAILED (runs-2) — killed |
| 3c | Unknown fails closed | N `sb_unknown_at_the_regate_fails_closed`; G `unknown_does_not_permit_and_is_not_final` | M8: `Unknown(_)` permits consumption | FAILED — killed |
| 3d | First non-current state other than Unknown is final for the admission (F-3) | G `the_first_non_current_state_other_than_unknown_is_final` (Refused stays final after the ambiguity is revoked although S-A alone says Current; Stale stays final) | M9: remove the `settled` early return | FAILED (gate.rs:281) — killed |
| 4 | Labels: `Delivered` carries admission summaries (outcomes by id, revision, `Domain::Fixture`, label, boundary kind/epoch, regate revision) and the labelled assumed discharges for mapping kinds only; bound to the binding | N positive test (`positive_assertions`) | covered by M1/M2 (classification) — no separate mutant | — |
| 5a | `run_attempt` / `run_attempt_observed` not public spawn routes (crate-private; `native::test_support` wrappers under `fixture`) | compile_fail doctests in `src/native.rs` (lines 25, 29) | M10: `pub fn run_attempt` | doctest native.rs line 25 FAILED — killed |
| 5b | `AssumedDischarges` not publicly mintable (crate-private `assess`, private fields) and bound to the binding (`bound_to` = frozen contract digest, checked by `run_native`) | compile_fail doctests in `src/gate.rs` (lines 30, 34); N positive test asserts `bound_to` | M11: `pub fn assess` | doctest gate.rs line 30 FAILED — killed |
| 6 | Fixture authority world matching the S3 assumption world admits; frozen world still refuses before spawn | G `the_fixture_authority_world_admits_and_an_empty_world_refuses`; N `the_frozen_world_is_refused_before_spawn_by_known_realization_incompatibility`; G `the_frozen_world_is_a_known_incompatibility` | not mutated separately | — |

**Status of rows 1-6 after remediation 1.** M1-M3, M9-M11 anchor on code that remediation 1 did not
change (`discharge`, the `settled` return, `pub(crate)` visibility); they were recorded at `dd68df6c`
and were **not re-run** on the corrected code. M4, M6, M7 and M8 anchor on text remediation 1 changed
(`admit_at`, `HeldAdmission` fields and `regate` signature, the consumption `regate` call): their
mutation specs no longer apply as written, they were **not re-run**, and their mapped tests
(`sb_an_admission_refusal_*`, `regate_reads_a_fresh_observation`, `sb_revocation_supersession_*`,
`sb_unknown_at_the_regate_fails_closed`) pass in the remediation-1 gate (see RESULT) but are `not
established` as killing those mutants on the corrected code. n = 1 per row as before.

## Remediation 1 (design correction: host-constructed admission request)

Source: `src/gate.rs` blob `28c2c371`, `src/native.rs` blob `f289fd92` (crate
`work-engine-review-delivery`). Mutants: `remediation-1/mutants/runs-1` (applied by
`remediation-1/mutants/mutate.py`, exact-once text replacement, original bytes saved; pre- and
post-restore blob hashes equal, `pre-hashes`/`final-hashes` steps). n = 1 mutant per row.
`N` = `tests/native_vertical.rs` (real Pi SDK child where a spawn occurs).

| # | Claim | Test | Mutant | Result under mutant |
|---|---|---|---|---|
| T1 | The admission is decided for the preparation's vantage, not a caller's: in a world where that vantage is bound to another actor and a second vantage V' to the declared actor and host digest, delivery refuses before spawn with exactly R3 refused, `ActorMismatch`, subject = the preparation's vantage | N `sb_the_admission_is_decided_for_the_preparations_vantage` | N1: construct the request with vantage `vantage-builder` (V') | FAILED (native_vertical.rs:968: refused set `[R2]` instead of `[R3]`) - killed. The mutant's V' is not admitted in this world (R2 refuses for it): the world shows refusal for the preparation's vantage, not that a caller-chosen V' would have been admitted (a world admitting V' is `not established`). |
| T2 | `Delivered` carries the constructed request identity equal to the binding's (vantage = preparation's, subject, candidate digest) and the asserted inputs under `caller_asserted_unanchored`, at both boundaries | N positive `canonical_preparation_reaches_the_real_sdk_and_the_existing_consumer` (`positive_assertions`) | N2: construct with a constant subject | FAILED (native_vertical.rs:320, subject) - killed. The zero-candidate-digest variant listed in the design review was not run (one mutant per row). |
| T3 | Requirements that are not the candidate's (frozen world's; edited admissible variant) refuse with `AdmissionMismatch { requirements }`, before spawn and before any observation (0 observations) | N `sb_requirements_that_are_not_the_candidates_refuse_before_any_observation` | N3: skip the requirements digest check | FAILED (native_vertical.rs:943, refusal identity for the frozen-world case) - killed |
| T4 | A pin superseded between the two admissions by a passing replacement refuses before spawn as `NotCurrent(Stale(IssuanceSuperseded([binding])))`, spawn 0, 3 observations (two admissions, one regate) | N `sb_a_pin_replaced_between_the_two_admissions_refuses_before_spawn` | N4: remove the delivery regate | FAILED (native_vertical.rs:461, no refusal; the spawn ran) - killed |
| T5 | A grant expiring between the delivery admission and the consumption regate is `Refused` (O4 only) at consumption, consumer 0, run retained; the time source is sampled 4 times (two admissions, two regates) | N `sb_a_grant_that_expires_between_boundaries_refuses_consumption` | N5: regate with the admission-time sample | FAILED (native_vertical.rs:461, no refusal; consumption ran) - killed |
| T6 | `FixtureAuthority` has no `request` field and no `operation` field (stable rustdoc asserts non-compilation only, not the error code: S-A N-1 class) | compile_fail doctests in `src/native.rs` (lines 41, 47) | N6: restore `pub request: OperationRequest` | doctest native.rs line 41 FAILED - killed |
| T7 | The boundary operation is derived from the binding: two bindings differ, one binding is stable, the summary's boundary operation equals `boundary_operation(binding)` | N `sb_the_boundary_operation_is_derived_from_the_binding` (+ positive test assertion) | N7: constant operation | FAILED (native_vertical.rs:1029, `ne`) - killed |

Also carried with the correction (no mutant): P3-5 (the supersession half of the finality test is
vacuous; the comment now says so; the `Refused` half still carries F-3), P3-6 (the S3 positive
assertion that the assumption world's `realization_evaluation` has no `realization:`-prefixed check
row is restored in `the_assumption_world_screens_into_admitted_and_assumed_mapping_kinds`), P3-4
(dissolved with `compare_with_binding`).

S1/S3 matrix tests: all existing review-delivery, review-preparation, organization and episode-core tests pass
in gate-1 (`test-four-plus-sa`, `test-default-features`, `native-1` 28/28); the S3 gate tests were rewritten
from `assess`/`assess_preparation` to the token-free `screen` (same refusal assertions).

Not mutated: the `NotAdmitted` branch of `assess` (an admitted kind whose named check did not pass). It is
unreachable through S-A: `admit` returns an `EffectAdmission` only when every check passed, and no test can
construct one with a refused outcome. CheckId keying is by construction (`discharge` table, `outcomes.passed(id)`),
pinned by the table test, not by a behavioural negative.
