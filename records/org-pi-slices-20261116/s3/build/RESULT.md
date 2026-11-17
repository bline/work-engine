# S3 builder result: R2-S refusal seam with assumed discharges

Role: builder (Claude Opus 5.5, `claude-opus-5-5`), parent/acceptor Pi root
`01a1e56e-be12-7424-977b-c7c7983887d0`. Authority: `s2/OWNER-DECISION-S2.json` (O11 "build R2-S").
No identity binding or context-status binary was delivered to this session, so the mid-task
context check was not run: `usage_unavailable` (no binding). Cost: not observed by this session
(`not established`).

## Entry

- Worktree `/home/bline/cbm-work/wt-p1-vertical-20261113`, branch `p1-pub-vertical-20261113`,
  HEAD `e6a57e7cdce4fc5f7e9a1d6710bfe116812d1e2f`, status clean at entry (verified).
- Nothing committed, staged or branched. `work-engine-organization` has no diff.

## Files changed (uncommitted)

```
 rust/crates/work-engine-review-delivery/Cargo.toml |   5 +
 .../work-engine-review-delivery/src/binding.rs     |  38 ++++
 rust/crates/work-engine-review-delivery/src/lib.rs |   7 +
 .../work-engine-review-delivery/src/native.rs      | 161 +++++++++++++-
 .../work-engine-review-delivery/tests/binding.rs   |  73 ++++++-
 .../tests/common/mod.rs                            |  41 +++-
 .../tests/native_vertical.rs                       | 236 +++++++++++++++++++--
 .../work-engine-review-preparation/src/lib.rs      |  10 +-
 .../work-engine-review-preparation/src/prepare.rs  |  89 +++++++-
 .../tests/preparation.rs                           |  46 +++-
 10 files changed, 663 insertions(+), 43 deletions(-)
untracked:
 rust/crates/work-engine-review-delivery/src/gate.rs
 rust/crates/work-engine-review-delivery/tests/gate.rs
 rust/crates/work-engine-review-delivery/tests/fixtures/assumption-world/requirements.json
 rust/crates/work-engine-review-delivery/tests/fixtures/assumption-world/DERIVATION.md
```

- `preparation/src/prepare.rs`, `lib.rs`: closed `BlockerKind` (9 variants, one per id family),
  `BlockerKind::ALL`, `BlockerKind::of_id`, and a `kind` field on `Blocker`. Ids, classes, scopes,
  requirements, bases and refusals are unchanged (invariant 1).
- `delivery/src/gate.rs` (new, `fixture`): host-owned total `discharge(BlockerKind)`, the labelled
  assumption set (`AssumedDischarge`, `ASSUMED_DISCHARGE_LABEL =
  "fixture_assumed_discharge_not_admission"`), pure `assess` refusing kind/id mismatch, any
  `KnownIncompatibility`, then any undischarged kind. `AssumedDischarges` has no public constructor.
- `delivery/src/native.rs`: `realization_description(&NativeConfig)` (host-derived, exhaustive
  destructuring of the config); gate evaluation before spawn and immediately before consumption
  (re-derive description, recompute preparation, require equality with the delivered one, assess);
  `run_native` made private and requires the pre-spawn gate output; spawn counter; new
  `DeliverError::Refused { boundary, refusal, run }`; add-only fixture seam `BetweenBoundaries` with
  `deliver_and_consume_observed`; `compile_fail,E0603` doctest for `run_native`.
- `delivery/src/binding.rs`: refuses a request carrying a realization (`CallerRealization`);
  prepares with the host's description; `spawn_attempts()`; crate-private `candidate()`.
- `delivery/src/lib.rs`, `Cargo.toml`: `gate` module, docs, `[[test]] gate`.
- Tests: `tests/gate.rs` (new, 5 pure tests); `tests/native_vertical.rs` (suite on the assumption
  world, 3 new tests, wrong-session test via the gated entrypoint); `tests/binding.rs` (frozen world
  kept for join tests, host request, 1 new test); `tests/common/mod.rs` (`ASSUMPTION_REQ`,
  `assumption_candidate`, `world`/`frozen_world`/`world_from`, `host_request`); preparation
  kind test.
- Fixture: `tests/fixtures/assumption-world/` with derivation (invariant 5).

## Runs (custody, `s3/build/runs/`)

- `iter-1`: fmt (write) ok; clippy failed (1 `cloned_ref_to_slice_refs` lint); binding test failed
  on my expected order of constraint blockers (the compiler orders `execution_confinement` before
  `provider_family`; I had predicted the requirements' order). Both fixed.
- `iter-2`: 4/4 passed (native 23/23).
- `final` (plan `plan-final.json`, modeled on `s1/root-gate-plan-2.json`, cwd worktree `rust/`):
  toolchain, fmt `--check` (4 crates), clippy `-D warnings` (delivery `fixture` all-targets, plus
  preparation `fixture` all-targets), test-four with fixture features, default-feature
  delivery+preparation, authority-store `--test store`, native suite once: **8/8 passed**, 29
  `test result: ok` lines, 0 failures; `native-1` 23 passed. Summary sha256
  `29c9c078df71a5f7ade67733549db0a0f92361dc3c3fc1117dd3424cb46f752e`. (`test-four` also runs the
  native suite, so it ran twice in this plan.)
- Mutants M1–M6 under `s3/build/mutants/`: all six killed, all restored before `final`.

Matrix: `/home/bline/cbm-work/org-pi-slices-20261116/s3/build/MATRIX.md`.

## Deviations and decisions for the root

1. **`mutation: none` sits in the host description as a labelled assumption.** The host claims
   `MutationClaim::ReadOnlyClaimed` with no tools. That is the only way the second world can admit
   under the ruling (realization checks are never assumable, and the mutation check is emitted for
   every condition, constraints or not). A strictly truthful `NotEstablished` would make the
   assumption world refuse. The doc comment on `realization_description` and DERIVATION.md label it
   "assumed, not true". This is a tension between "describe truthfully" and invariant 5 that the
   root should rule on; I did not redesign around it.
2. **Provider/confinement values are descriptor free text**, not v1 schema classes:
   `none_provider_disabled_fixture_stream`, `none_unconfined_child_process`. They produce
   `KnownIncompatible` through the existing `against_allowed` machinery. `continuity` uses the
   existing v1 value `fresh_initial` (every attempt is a new child and session; verification requires
   a three-entry history).
3. **The re-check is made discriminable by an add-only fixture seam** (`BetweenBoundaries`, public
   under `fixture`). It can only add blockers, so it cannot widen what passes. In R2-S nothing the
   gate reads can change between the two boundaries.
4. `Blocker.kind` is serialized, so the frozen contract bytes now include a `kind` per blocker
   (contract digests differ from S1's).
5. Binding join tests stay on the frozen world (they test the join, not the gate); one of them
   clears constraints, which is a no-op on the assumption world.
6. `DeliverError` gained a variant (not `non_exhaustive`); external exhaustive matches would break.
   No consumer outside this crate's tests was searched for, so whether one exists is `not established`.

## Unfinished / not established

- S1 mutants were not re-run on the second world (their mapped tests pass).
- `RealizationDiffers` and `PreparationChanged` are not separately discriminated (see MATRIX).
- The root's clean-export repeat of the native suite is still to run.

## Explicitly unclaimed

Fixture standing only. Assumed discharges are not admissions and establish nothing. No Gate O / Gate
R, no authority records, no stale/revoked/unknown states, no epochs or single-use ledger, no
storage. No confinement of the SDK child; `mutation: none` is assumed. Production admission still
refuses (the gate exists only with the `fixture` feature; default builds have no runner). No live
provider, network or Docker was used.
