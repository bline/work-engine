# S1 matrix: invariant -> tests -> mutant -> result

Source: worktree `/home/bline/cbm-work/wt-org-hardening-20261117` (uncommitted, on `0fe9dc74`).
Blobs under test (`git hash-object`, equal in `runs/gate-1/hashes`, `mutants/runs-1` and
`mutants/runs-2` pre/final hashes): review-delivery `src/gate.rs` `4d8feb23`, `src/native.rs`
`48f8d280`; review-preparation `src/fixture.rs` `03da1081`.
Gate: `runs/gate-1` 12/12. Mutants: `mutants/runs-1` (17 mutants), `mutants/runs-2` (M6
re-specified). Each mutant was applied by `mutants/mutate.py` (exact-once text replacement,
original bytes and sha256 saved, restore checks the sha256), its mapped test(s) run under custody,
then restored; pre- and final-hash steps print equal blobs in both runs. Every "killed" below is a
test panic at the cited line, or a compile_fail doctest that compiled; no mutant failed to compile.
n = 1 mutant per row.

`N` = review-delivery `tests/native_vertical.rs` (real Pi SDK child; run with `--features
test-support`), `G` = review-delivery `tests/gate.rs` (`--features fixture`), `C` =
review-preparation `tests/fixture_consumer.rs` (`--features fixture`), `D` = review-delivery
doctests (`--features fixture --doc`, i.e. without `test-support`).

## New claims (S1 invariants 1-3)

| # | Invariant | Tests (pass in gate-1) | Mutant | Result under mutant |
|---|---|---|---|---|
| 1a | The consumer advances an episode only with a proof whose S-A point check over the host's regate observation finds the admission `Current`; it reports what it found | C `s1_a_current_gate_proof_for_this_consumption_lets_the_episode_advance`; N positive `canonical_preparation_...` (consumer's `gate` equals the consumption summary's boundary, admission revision, regate revision) | S1-P5 skip the proof check (`check_proof(..).unwrap_or(fabricated)`) | killed: C `s1_a_gate_proof_that_is_not_current_or_has_no_observation_is_refused` panicked at `fixture_consumer.rs:186` (`must be refused`) |
| 1b | Missing proof = no observation (`Err(Unobserved)`) -> `Unknown` -> refused; a revoked pin -> `Revoked` -> refused; input state unchanged | C `s1_a_gate_proof_that_is_not_current_or_has_no_observation_is_refused` | S1-P4 `Unknown` accepted by the consumer | killed: same test, `fixture_consumer.rs:186` (the Unknown half) |
| 1c | An admission for another candidate is refused (`GateProofMismatch { "candidate digest" }`) | C `s1_a_gate_proof_for_another_candidate_subject_or_boundary_is_refused` | S1-P1 candidate-digest refusal neutralized | killed: `fixture_consumer.rs:186` |
| 1d | An admission for another subject is refused (`"subject"`) | same test | S1-P2 subject refusal neutralized | killed: `fixture_consumer.rs:186` |
| 1e | An admission for another boundary kind (delivery) is refused (`"boundary kind"`) | same test | S1-P3 boundary-kind refusal neutralized | killed: `fixture_consumer.rs:186` |
| 1f | Vantage: the admission's vantage must be the obligation's serving vantage (`"vantage"`) | C `s1_a_gate_proof_for_another_obligations_vantage_is_refused` (Remediation 2; two-obligation candidate) | S1-P8 `return mismatch("vantage")` neutralized | killed (Remediation 2). Was "not established" at gate-1: no world admitting another vantage had been constructed |
| 1g | The consumer takes no proof argument it could skip | compile_fail doctest in review-preparation `src/fixture.rs` (calls without the proof) | not mutated separately (a type-level fact; error code not enforced by stable rustdoc, S-A N-1 class) | - |
| 2 | `test_support` exists only with the separate non-default `test-support` feature, not implied by `fixture` | D compile_fail doctest `native.rs` line 45 (present only when `test-support` is off, via `cfg_attr`) | S1-P6 `#[cfg(feature = "fixture")]` on `test_support` | killed: doctest `native.rs - native (line 45) - compile fail ... FAILED` (8 passed, 1 failed) |
| 3 | `Delivered` carries the delivery regate revision (`Some`, taken immediately before `run_native`) and the consumption regate revision; each follows its admission revision when the source moves; the consumer saw the consumption regate's revision | N `s1_both_regate_revisions_are_carried_and_follow_their_admissions` (ticking source: 4 observations, strictly increasing); N positive (both `Some(admission revision)` on an unchanged source) | S1-P7 delivery regate revision dropped | killed: N panicked at `native_vertical.rs:1242` (`delivery regate carried`) |

## Re-established S-B mutants (M1-M4, M6-M11 on the corrected code; M5 stays retired)

| Old id | Claim | New spec (current text) | Mapped test | Result |
|---|---|---|---|---|
| M1 | Separation is admitted (O8+R3) | unchanged text: `Separation => AssumedDischarge` | G `discharge_is_total_admitted_kinds_name_their_check_ids` | killed (`gate.rs:35`) |
| M2 | Mapping kinds stay assumed | unchanged text: `ScopeDigestToSubject => Admitted(&[O3])` | same | killed (`gate.rs:35`) |
| M3 | RealizationCheck never dischargeable | unchanged text: `RealizationCheck => AssumedDischarge` | same | killed (`gate.rs:35`) |
| M4 | A delivery-admission refusal refuses before spawn | anchor moved to `let mut delivery = admit_at(BoundaryKind::Delivery)...`: on refusal call `run_native` first, then return the refusal (plus the `mutant_assumed` helper in gate.rs as before) | N `sb_an_admission_refusal_refuses_before_spawn` | killed (`native_vertical.rs:866`) |
| M5 | - | **retired** (replaced claim, remediation 1: `compare_with_binding` deleted) | - | - |
| M6 | The host's regate reads a fresh observation, never the admission-time one | `HeldAdmission` takes a second observation at admission time and `regate_observed` decides on it on **every** call (`admission_state(&self.replay, ..)`); the observation handed to the consumer stays the fresh one | N `sb_revocation_supersession_or_missing_grant_between_boundaries_refuses_consumption`; G `regate_reads_a_fresh_observation` | **runs-2**: both killed (N `native_vertical.rs:1089`: the host proceeded, the consumer refused, so no gate refusal; G `gate.rs:257`). runs-1 used a weaker spec (replay only on the first regate, then fresh, and the replayed observation was also handed to the consumer): N killed at `native_vertical.rs:466` (`must be refused`: consumption ran), G **passed** (it regates twice; the second read fresh). runs-1 G is a spec defect of the mutant, superseded by runs-2 |
| M7 | Consumption needs the host's regate to be current | consumption arm ignores the regate verdict: `(_, Some(inputs)) => Ok((discharges, admission.revision(), inputs))` | N `sb_revocation_...` | killed (`native_vertical.rs:1089`, `expected a gate refusal`: the host called the consumer, whose own proof check refused) |
| M8 | Unknown fails closed at the host's regate | extra arm `(Unknown(_), Some(inputs)) => Ok(..)` | N `sb_unknown_at_the_regate_fails_closed` | killed (`native_vertical.rs:1089`, same mechanism as M7) |
| M9 | First non-current state other than Unknown is final | remove the `settled` early return in `regate_observed` | G `the_first_non_current_state_other_than_unknown_is_final` | killed (`gate.rs:288`) |
| M10 | `run_attempt` not public | `pub fn run_attempt(` | D | killed: doctest `native.rs (line 35)` compiled |
| M11 | `assess` not public | `pub fn assess(` | D | killed: doctest `gate.rs (line 35)` compiled |

M7 and M8 now show two layers: the host's regate refusal (asserted by the mapped tests as
`DeliverError::Refused` at `BeforeConsumption` with consumer 0) and, beneath it, the consumer's own
point check, which still refused under both mutants (the tests fail because the consumer was
invoked, not because consumption succeeded). M6 runs-1 shows the stated limit of the consumer's
check: a replayed admission-time observation handed to it is accepted (it cannot tell fresh from
earlier; freshness is the host's obligation, killed at the host by runs-2).

## Optional item 5

T5 (`sb_a_grant_that_expires_between_boundaries_refuses_consumption`) now also asserts that every
O4 reason is `ReasonCode::AuthorityExpired` (passes in gate-1). Not mutated separately.

## Remediation 1 (review `review/REVIEW.md`: `changes_required`)

Evidence: `remediation-1/runs/gate-3` 12/12 (a first attempt, `gate-2`, failed 10/12 on a
vantage test world described under P3-2 and is kept as recorded); mutants
`remediation-1/mutants/runs` (pre and final blobs equal). Full account: `remediation-1/RESULT.md`.
Blobs now: delivery `src/native.rs` `82ca23e7`, `src/lib.rs` `9b3209db`; preparation
`src/fixture.rs` `caa4d925`, `src/lib.rs` `7fcca1f6`; `gate.rs` `4d8feb23`, `binding.rs`
`d6956684`, both `Cargo.toml` and `Cargo.lock` unchanged.

Rows 1a-1e and 3 are unchanged in claim; the S1-P1..P5 mutants were re-run on the new blobs (the
mapped test file changed) and are killed again. Changed or new rows:

| # | Claim | Test | Mutant | Result |
|---|---|---|---|---|
| 1c-1e, 1a, 1b (P3-3) | A refusal that fails to occur names its case | C `refused_case` names the case in the `expect` message; used by the loop and the not-current test | S1-P1/P2/P3/P4/P5 re-run | killed, each panic at `fixture_consumer.rs:192` names its own case: `other candidate`, `other subject`, `delivery boundary`, `no observation`, `revoked pin` (P5 stops at the first, `revoked pin`) |
| 1f (P3-2) | Vantage: the admission's vantage must be the obligation's serving vantage | none (superseded: see Remediation 2) | S1-P8 `return mismatch("vantage")` neutralized | **survives** at remediation 1: 15/15 consumer tests pass. The remediation-1 reading was wrong: it held for the frozen single-obligation candidate (n = 1 candidate; S-A R2 refused `vantage:fixture-other-review` and `vantage-builder` with `VantageUnknown`, n = 2 names) and was generalized to "no candidate can carry two vantages" and "defense in depth". A two-obligation candidate carries two vantage requirements, so the comparison is load-bearing for it (Remediation 2) |
| inv. row: observation freshness (P2-2) | The success output labels freshness a caller assertion | C positive test asserts `GateCheck::caller_asserted_regate_observation_freshness()` equals both the literal `"caller_asserted_regate_observation_freshness"` and `GATE_OBSERVATION_FRESHNESS_ASSERTED_LABEL` | none (label) | passes in gate-3 |
| inv. row: boundary operation / epoch (P3-1) | Only the boundary kind is compared | documentation (module and `ConsumptionProof` docs) | none | documented; O10 class; not code |
| inv. row: admission origin (P2-1) | With S-A `fixture`, any caller can mint a consumption admission and consume outside review-delivery's gate | documentation in `native.rs`, `lib.rs` (delivery), `lib.rs` (preparation), `fixture.rs` | none | corrected; the slice's own positive test is the demonstration |

Inventory (proof inputs), after remediation: admission (existence) S-A-issued, any caller can mint
one under S-A `fixture`; candidate digest, subject digest-bound; vantage consumer-derived,
load-bearing for a candidate with two or more obligations (not defense in depth; covered after
Remediation 2); boundary **kind** checked, operation and epoch not (O10);
observation content S-A-sealed; observation freshness **caller assertion, labelled**
(`caller_asserted_regate_observation_freshness`); `regate_now_millis` caller assertion, labelled
(`caller_asserted_regate_time`). Not re-run: S1-P6, S1-P7, M1-M11 (their anchors and mapped tests
are text-unchanged; `native.rs` changed in its module documentation only).

## Remediation 2 (recheck `recheck/RECHECK.md` R-1: `changes_required`)

Evidence: `remediation-2/runs/gate-2` 12/12 (`gate-1` failed 11/12 on `fmt --check` only, kept as
recorded); mutants `remediation-2/mutants/runs` and `runs-2` (final blob). Full account:
`remediation-2/RESULT.md`. Blobs now: preparation `tests/fixture_consumer.rs` `c856f41e`,
`tests/common/mod.rs` `ecf9f80e`, `Cargo.toml` `097936f2`; the other nine gated files equal gate-3.

| # | Claim | Test | Mutant | Result |
|---|---|---|---|---|
| 1f (R-1) | With a two-obligation candidate, an S-A admission for the second obligation's vantage (same candidate digest, subject, Consumption boundary, S-A point check `Current`) is refused when the declaration names the first: `GateProofMismatch { "vantage" }`. Control: the first vantage on the same candidate is consumed | C `s1_a_gate_proof_for_another_obligations_vantage_is_refused` (S-A admission asserted first; `Ask.vantage`, new `Ask.bound_vantages`) | S1-P8 `return mismatch("vantage")` neutralized | **killed**, 15 passed / 1 failed, panic at `fixture_consumer.rs:192:24`: ``case `second obligation's vantage` must be refused`` (both runs). One two-obligation shape (n = 1); synthetic S-A world |
| inv. row: vantage | The vantage comparison is load-bearing for multi-obligation candidates (not defense in depth) | doc comment above the loop test and above the new test | none (comment) | corrected; replaces the remediation-1 classification |
| nit (P3) | `fixture` feature comment says "the host's fresh observation" without the caller-assertion qualifier | `Cargo.toml` (preparation) comment | none | qualifier added |

S1-P1..P5 not re-run (anchors and mapped tests text-unchanged; one doc comment above the loop test
changed). Inventory (proof inputs): vantage consumer-derived, load-bearing for two or more
obligations, covered by S1-P8; all other rows as after Remediation 1.
