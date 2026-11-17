accept_private_readiness

# S3 recheck 1 of the R2-S candidate after remediation 1

Reviewer: Claude Fable 5.2 (`claude-fable-5-2`), same review episode and retained context as
`s3/review/REVIEW-1.md`. Parent and acceptance authority: Pi root
`01a1e56e-be12-7424-977b-c7c7983887d0`.
Subject: checkpoint `1ed99580b9bc765ca23749445239cea9f8f0460a`, tree `a5ffe708…`, parent checkpoint
`24597fef`, baseline `e6a57e7c`. Paths are relative to
`rust/crates/work-engine-review-delivery/` in `s3/export-1ed99580` unless they start with `s3/`.

F-1, F-2, F-3 and F-4 are resolved as requested. No guard was weakened. No new P1 or P2. Bounded
private readiness, fixture standing only, with the P3 items in §4 carried.

## 1. Method and limits

- **Executed by me:** `git diff 24597fef 1ed99580` and `git diff --stat e6a57e7c 1ed99580` in the
  worktree. Nothing else was run; no build or test.
- **Remediation diff:** exactly three files: `src/native.rs`, `tests/native_vertical.rs`,
  `tests/fixtures/assumption-world/DERIVATION.md`. I read every hunk.
- **Whole task patch:** 14 files, all under `work-engine-review-delivery` and
  `work-engine-review-preparation`. `work-engine-organization` is not among them. This closes the limit
  in REVIEW-1 that the file set and the unchanged compiler were not established by me.
- **Executed facts from records:** `s3/root-gate-2/*.stdout` and `s3/remediation-1/mutant-F2/`. I
  matched every `test result` line: 21 lines in `test-four.stdout` sum to 252 passed, 0 failed; default
  features 33; authority store 37; native 24 in each of `native-1..3`. No `FAILED` line in any
  `root-gate-2` stdout. These equal `s3/ROOT-GATE-2.json`.
- Not verified: that the export equals the checkpoint tree byte for byte (two grep sites agree with
  the diff). F-6's corrected record was not re-read; I rely on the root's statement. Board read and
  context check were not performed. My spend is `not established`.

## 2. F-1 to F-4

| Finding | Status | Evidence |
|---|---|---|
| F-1 mutation claim (root ruling: option 1) | **Resolved** | `ReadOnlyClaimed` is kept and reworded as the scoped host claim: what is claimed, what backs it (construction; refusal at allocation and request), what is not established (process-level writes by the unconfined child), and that the evidence gap is the `realization-admission` blocker passed only by the labelled assumed discharge (`src/native.rs:219-226`; DERIVATION.md "What is assumed, and what is not established"; `tests/native_vertical.rs` module doc and the `mutation: none` assertion message). A grep over the whole crate directory in the export for `not a true`, `assumed, not true` and `not true` finds nothing. |
| F-2 description not derived from configuration | **Resolved** | `realization_description` now binds `adapter`; for any path other than `NativeConfig::default_adapter()` it returns `mutation: NotEstablished` with provider, confinement and continuity unstated (`src/native.rs:248-256`). The unchanged comparison in `evaluate` then refuses with `RealizationDiffers` before spawn. The false sentence is gone; the doc now classifies `node`, `sdk_root` and `control` as presumed, unchecked inputs and says path equality is name-level, not a pin. New test `a_non_default_adapter_is_refused_before_spawn_because_the_description_differs` asserts `Refused { BeforeSpawn, RealizationDiffers }`, no run, spawn 0, consumer 0, empty fixture root, unchanged episode state. It passes in 4 recorded root-gate runs. Mutant F2 (ignore `cfg`): the test FAILED, exit 101, panic at `native_vertical.rs:625` (record). |
| F-3 false absence statements | **Resolved** | Both sentences now say "the only route that runs the SDK delivery protocol from a binding" and state that `run_attempt` is a public generic spawn route and is not gated (`src/native.rs:14-17` and the `deliver_and_consume` doc). The grep for the old wording finds nothing. Visibility is unchanged, as agreed. |
| F-4 label dropped on success | **Resolved** | `Delivered` carries `assumed_before_spawn` and `assumed_before_consumption` (`src/native.rs:1250-1253`). `positive_assertions` checks, at both boundaries, the boundary, the label, and that `assumed()` equals the (id, kind) list of the delivered preparation's blockers, which it asserts is non-empty. `cleared` is renamed. |

## 3. Guards

By reading the diff:

- `gate.rs`, `binding.rs` and `evaluate` are untouched.
- `deliver_and_consume_observed` keeps its order: pre-spawn evaluation, `run_native`, completeness and
  `verify`, the seam, pre-consumption evaluation, `consume`. Both refusal returns are unchanged in
  content. The only change is that the two `Ok` values are kept.
- The F-2 change can only add a refusal. For the default adapter the description is the same constant
  as before. For any other adapter the value has `mutation: NotEstablished`, so it can never equal the
  binding's description; if a later change prepared with it, the mutation check would be Unestablished
  and refuse as undischarged.
- `RealizationDiffers` now has a mapped test. The recorded mutant is on the description side. By
  reading, removing the comparison itself would also fail that test (the gate would pass and a spawn
  would be attempted); no mutant on the comparison is recorded.

## 4. P3 items to carry

1. **F-2 residual.** For the default adapter path the description is name-level correspondence. A
   different file at that path, a different `node`, or an unselected SDK is not detected by product
   code. The source now says so; the acceptance record's unclaimed list should too.
2. **F-3 residual.** `run_attempt` and `run_attempt_observed` are ungated generic spawn routes (the
   new parenthetical names the first only). `fixture::consume_fixture_result` is callable with no gate.
3. **F-5.** `AssumedDischarges` is not bound to a binding and is publicly mintable through
   `gate::assess`.
4. **F-7.** The contract bytes changed in two ways from S1: `kind` per blocker and a non-null
   `realization_evaluation`. RESULT.md for the build names only the first.
5. **F-8, reduced.** `PreparationChanged`, `GateRefusal::Preparation` and `run_native`'s boundary check
   have no test or mutant. Pre-spawn `Undischarged` is not produced through the host path.
6. **F-9, extended.** All seven S3 mutants (M1–M6, F2) are hand-applied, n = 1 per row, run on the
   uncommitted worktree before the checkpoint they support; F2 ran its one mapped test only. S1 and S3
   mutants were not re-run at `1ed99580`, so that each is still killed is `not established`.
7. **Label on the consumer-refusal path.** `DeliverError::Consumer` is raised after both evaluations
   passed and carries neither. Minor.
8. **Second-world coverage.** No native positive run delivers non-empty constraint contexts
   (REVIEW-1 §4); S1's carried F-9 remains.
9. **Workspace formatting.** The remediation RESULT reports `cargo fmt --all --check` exit 1 with an
   unread diff. The root gate's four-crate fmt passes. Whether the remainder is outside this change is
   `not established`.

## 5. Correction to REVIEW-1

REVIEW-1 §2 F-1 and §7 listed three places carrying the "assumed, not true" wording. There were four:
the `tests/native_vertical.rs` module doc also carried it. Error class: an enumeration built from
partial reads without a crate-wide search. Consequence: none here, because the builder reworded the
fourth; had the list been followed literally, one copy of the disowned wording would have survived.
The check that catches it is the crate-wide grep, now run on the new export with no hits.

## 6. Standing

Unchanged from REVIEW-1 §6: fixture standing only; assumed discharges are labelled assumptions, not
admissions; no Gate O or Gate R; no confinement of the SDK child; production admission still refuses
(no runner without `fixture`). Independence is separate session and context only, same provider family
as the builders. Private readiness is not integration.
