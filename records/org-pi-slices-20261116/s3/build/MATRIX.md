# S3 (R2-S) evidence matrix: refusal seam with assumed discharges

Source: worktree `/home/bline/cbm-work/wt-p1-vertical-20261113`, branch `p1-pub-vertical-20261113`,
entry HEAD `e6a57e7c` (clean), changes uncommitted. Runs: `runs/final/` (custody summary sha256
`29c9c078df71a5f7ade67733549db0a0f92361dc3c3fc1117dd3424cb46f752e`, 8/8 passed). Each mutant was
applied by hand, its mapped test(s) run under custody into `mutants/<id>/`, then restored; every
`mutant.patch` records the patch, the outcome and the failing line. `runs/final` ran after all
restorations.

## R2-S invariants

| # | Invariant | Guard (site) | Mapped test(s) | Mutant | Record | Result under mutant |
|---|---|---|---|---|---|---|
| 1 | Closed `BlockerKind`; each emitted blocker has exactly one kind, the one its id family names; `prepare` otherwise unchanged | `work-engine-review-preparation/src/prepare.rs` (`BlockerKind`, `of_id`, `Blocker.kind`) | `preparation::every_emitted_blocker_has_exactly_the_kind_its_id_family_names` (all 9 kinds seen across 3 claims on the frozen world); all pre-existing preparation/fixture_consumer tests unchanged and passing | none recorded for the enum itself; its use by the gate is M6 | — | — |
| 2 | Host-derived realization; `realization_evaluation` exists on the delivered path; caller cannot supply or bypass it | `native::realization_description` / `runner_realization`; `ExperimentBinding::build` (`CallerRealization`, host request); gate `RealizationDiffers` | `binding::the_host_derives_the_realization_description_and_a_caller_cannot_supply_or_bypass_it`; `native_vertical::the_frozen_world_is_refused_before_spawn_by_known_realization_incompatibility`; positive assertions (`realization_evaluation.is_some()`) | **M5** binding accepts the caller's claim and prepares with it | `mutants/m5-host-realization/` | Both FAILED: binding.rs:336 (no realization evaluation); native: `RealizationDiffers` instead of `KnownIncompatibility` (gate's second layer still refused before spawn) |
| 3a | Refusal before spawn: spawn attempts 0, consumer 0, fixture root untouched | `native::deliver_and_consume_observed` (pre-spawn `evaluate`), private `run_native(&AssumedDischarges)` | `native_vertical::the_frozen_world_is_refused_before_spawn_by_known_realization_incompatibility` | **M1** pre-spawn evaluation over an empty blocker list | `mutants/m1-pre-spawn/` | FAILED: boundary `BeforeConsumption` vs `BeforeSpawn` (a child was spawned) |
| 3b | Same evaluation immediately before consumption; refuses after a complete verified attempt; consumer 0 | `native::deliver_and_consume_observed` (pre-consumption `evaluate`) | `native_vertical::the_pre_consumption_recheck_refuses_after_a_verified_attempt` (KnownIncompatibility and Undischarged cases via the add-only `BetweenBoundaries` seam) | **M2** pre-consumption evaluation replaced by `Ok` | `mutants/m2-pre-consume/` | FAILED: `must be refused` (delivered and consumed) |
| 3c | No public spawn route without the gate | `run_native` private, takes `&AssumedDischarges` with `Boundary::BeforeSpawn` | doctest `compile_fail,E0603` in `native.rs` module docs (runs in `test-four`); `the_adapter_refuses_a_wrong_session_and_a_duplicate_start` now goes through `deliver_and_consume` | not mutated (compile-time property) | — | — |
| 4a | Undischarged kind refuses | `gate::assess` (undischarged branch) | `gate::an_unestablished_realization_check_has_no_discharge_and_refuses` | **M3** undischarged branch disabled | `mutants/m3-undischarged/` | FAILED (gate.rs:123); other 4 gate tests passed |
| 4b | Unknown / mislabelled kind refuses | `gate::assess` (kind == `of_id(id)`) | `gate::an_unknown_or_mislabelled_blocker_kind_refuses` | **M6** kind/id check disabled | `mutants/m6-kind-mismatch/` | FAILED (gate.rs:154) |
| 4c | Discharge total; assumption set = root ruling; realization checks never assumable | `gate::discharge` | `gate::discharge_is_total_and_realization_checks_are_never_assumable`; `gate::an_unestablished_realization_check_has_no_discharge_and_refuses` | **M4** `RealizationCheck => AssumedDischarge` | `mutants/m4-non-assumable/` | Both FAILED (gate.rs:24, :123); frozen-world test still passed (KnownIncompatibility refuses by class) |
| 4d | Assumed discharges are labelled, never admissions | `gate::ASSUMED_DISCHARGE_LABEL`, `AssumedDischarges` (no public constructor) | `gate::the_assumption_world_passes_only_by_labelled_assumed_discharges` | not mutated | — | — |
| 5 | Second world: frozen requirements without `realizationConstraints`, unchanged compiler, documented derivation; native suite on it; frozen world is the negative | `tests/fixtures/assumption-world/{requirements.json,DERIVATION.md}`; `common::world::{world, frozen_world}` | `native_vertical::assumption_world_is_the_frozen_world_without_realization_constraints`; the whole native suite (23 tests) runs on `world()` = assumption world; negative as 3a | not mutated | — | — |

## S1 matrix rows on the second world

The S1 rows (`s1/build/MATRIX.md`, M01–M12 and the unnumbered rows) map to tests in
`preparation`, `fixture_consumer`, `pure` and `native_vertical`. All of those tests pass in
`runs/final` (`test-four`, `test-default-features`, `native-1`); the native ones now run on the
assumption world. The S1 mutants were **not** re-run here, so whether each S1 mutant is still
killed on the second world is `not established`. One S1-mapped test changed shape:
`the_adapter_refuses_a_wrong_session_and_a_duplicate_start` reads the run from the gated
entrypoint's `DeliverError::Delivery` instead of a direct `run_native` call; its assertions are kept
and it adds `consumer_invocations() == 0` and `spawn_attempts() == 1`.

## Not discriminated

- The gate's `RealizationDiffers` check is a second layer behind the binding: under the current API
  only the binding builds the request, so neutralizing that check alone is an equivalent mutant.
- `PreparationChanged` (recomputed preparation differs from the delivered one) cannot be produced
  through the public API (the binding is immutable and `prepare` is deterministic); not tested.
- The re-check's discrimination rests on the add-only fixture seam `BetweenBoundaries`; in R2-S
  nothing real can change between the two boundaries.
