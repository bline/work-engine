# S-B remediation 1: local design correction (host-constructed admission request)

Worker: Claude Code (Sonnet 5.5, `claude-sonnet-5-5`), background session; parent/collector Pi root
`01a1e56e-be12-7424-977b-c7c7983887d0`. No identity binding or context-status binary was delivered,
so the mid-task context check was not run (`usage_unavailable`: no binding). Coordination board not
read (no claims or consequential operations). Cost not measured by this worker (`not established`).
Nothing committed, staged or branched. Status: **`map_ready`** (see "Not done").

Authority read: `ROOT-SB-DESIGN-DECISION.json`, `design-review/REVIEW.md` (all), `review/REVIEW-1.md`
(all). The root decision's `ordinary_repair_authorized` is true; scope = REVIEW §6 items 1-8, T1-T7,
retire the other-subject/candidate test and M5, P3-5/P3-6.

## Entry check

- Worktree `/home/bline/cbm-work/wt-org-r2-core-20261117`, branch `org-r2-core-20261117`.
- `git diff --stat dd68df6c` at entry: only the 9 untracked S-A files appeared (as deletions); no
  tracked file differed from the checkpoint.
- The 9 S-A files hashed with `git hash-object`; each equals its blob in
  `git ls-tree -r dd68df6c rust/crates/work-engine-organization-authority` (9/9).
- Result: the working tree equalled S-B checkpoint `dd68df6c` at entry. The S-A files were not
  touched (no S-A, organization or preparation change).

## Changes (all in `rust/crates/work-engine-review-delivery`), mapped to REVIEW §6

| §6 | Change | Reason |
|---|---|---|
| 1 | `native.rs` `FixtureAuthority`: `request` and `operation` removed. Fields: `source`, `clock: &dyn TimeSource`, `organization_id` (doc: caller-asserted, unanchored), `requirements` (digest-bound preimage), `actor_identity`, `installed`. `gate.rs`: new `TimeSource` trait | The caller no longer hands the gate a request or boundary; the input surface is the §4 inventory |
| 2 | `native.rs` private `construct_request(binding, authority)`: vantage = `binding.preparation().model_context.vantage.id`, subject and candidate digest from the binding, requirements preimage refused as `AdmissionMismatch { field: "requirements" }` unless its digest is the candidate's requirements digest, before any observation; called once and both admissions use it. `compare_with_binding` deleted | Construction through `OperationRequest::new` is total by signature; comparison was partial by enumeration and after construction every branch was a tautology |
| 3 | `native.rs` `pub fn boundary_operation(binding)` = `review-delivery:<frozen contract sha256>`, epoch 1 | Host-derived operation; makes the O10 gap concrete (same binding re-delivered reserves the same boundary identity) |
| 4 | `deliver_and_consume_observed`: `delivery.regate()` immediately before `run_native`, `Current` only, else `NotCurrent` before spawn | The spawn is an effect and had no regate (P3-1) |
| 5 | `HeldAdmission::admit` and `regate` sample the `TimeSource` each time (admission, delivery regate, consumption regate); no admission-time value is reused | P3-2 expiry during the run is now seen |
| 6 | `gate.rs` `HeldAdmission<'a>` holds `source` and `clock`; `regate()` takes no argument; manual `Debug` | `regate` cannot be given a source or time that differs from the admission's |
| 7 | `gate.rs` `AdmissionSummary` carries `request()` (`RequestIdentity`: vantage, subject, candidate digest) and `asserted()` (`CallerAsserted`: organization id, actor identity, label `CALLER_ASSERTED_LABEL = "caller_asserted_unanchored"`), read from the admission itself | A label that does not reach the success output is a defect (S3 F-4) |
| 8 | **Feature split not done; sentences narrowed** (`native.rs` module doc, `run_attempt` doc, `deliver_and_consume` doc). Two `compile_fail` doctests for T6 | See "Decisions" |

Also: module docs in `gate.rs`; P3-5 comment corrected and P3-6 positive assertion restored in
`tests/gate.rs`; tests in `tests/common/mod.rs` (`FixedTime`, `ScriptedClock`, `Shape`, builders) and
`tests/native_vertical.rs` (T1, T3, T4, T5, T7; T2 in `positive_assertions`; retired test removed).

Diff stat vs `dd68df6c` (`git diff --stat dd68df6c -- rust/crates/work-engine-review-delivery
rust/Cargo.toml rust/Cargo.lock`): 5 files, +558 / -152 (`src/gate.rs`, `src/native.rs`,
`tests/common/mod.rs`, `tests/gate.rs`, `tests/native_vertical.rs`). `Cargo.toml` and `Cargo.lock`
unchanged vs the checkpoint (no new feature, no new dependency edge). Blobs after the change:
`src/gate.rs` `28c2c371`, `src/native.rs` `f289fd92` (`git hash-object`).

### Decisions

1. **`test_support` not behind a feature.** Every lifecycle test lives in `tests/native_vertical.rs`
   with the delivery tests. A feature not implied by `fixture` would put that whole file behind
   `required-features`, and the package-wide plan steps (`--features fixture`) would then omit it
   (cargo omits a target whose required features are not enabled; unverified here, not run). That
   is not small. The
   sentences at the old `native.rs` ~593 and ~1463-1468 are narrowed instead (module doc, `run_attempt`
   doc, `deliver_and_consume` doc): "the only function in this crate that takes a binding", with the
   generic runner and `consume_fixture_result` named as routes outside the gate. A feature would be a
   statement of build intent, not access control; none exists here.
2. **The summary reads the identity from the admission** (`EffectAdmission::request()`), not from
   the binding, so what it reports is what the gate evaluated. T2 compares it with the binding.
3. **Installed digests are not in the success summary**: S-A exposes no accessor and S-A may not
   change here. Declared actor identity and organization id are.
4. **Requirements digest check kept** (REVIEW §3): named refusal before any observation. Whether it
   is redundant with O3 is `not established`; N3 shows it changes refusal identity and observation
   count.

## Runs (custody; `CARGO_TARGET_DIR=/home/bline/cbm-work/org-r2-core-20261117/target`)

- `runs/gate-1` (plan `sb/gate-plan-1.json`, unchanged from S-B; no new feature so none added):
  **8/8 passed**, summary sha256 `0c278564dea48c676851b01e1a42bfa9c6bf8eb91e6942ca2835acf96433c94b`:
  toolchain; fmt (5 crates incl. S-A); clippy `-D warnings` all targets with
  `work-engine-review-delivery/fixture` + `work-engine-organization-authority/fixture`; clippy
  default features; `test-four-plus-sa` (94.4 s; includes the `native_vertical` suite, **32 passed**,
  was 28: +5 new tests, -1 retired, and the doctests); default-feature tests; authority-store
  `--test store` (37 passed); `native-1` (`native_vertical`, 32 passed, 68.3 s). Every step
  `leftovers=0`. The native suite ran twice (inside the five-crate step and as `native-1`), as in the
  S-B gate.
- `runs/dev-1`: targeted dev run (fmt, `gate` 9/9, 11 native tests), superseded by gate-1.
- Counts are read from the `test result:` lines of each step's stdout; the S-B gate's earlier
  figures were fixture tests 324 / default 42 (ROOT-GATE), which this run did not recompute, so no
  comparison of totals is claimed.

## Mutants (n = 1 each; `mutants/runs-1`, `mutants/mutate.py`, plan `mutants/mutant-plan-1.json`)

Each applied by exact-once text replacement (original bytes and sha256 saved), the mapped test run
under custody, then restored. `pre-hashes` and `final-hashes` steps print equal blobs for both files
(`gate.rs` `28c2c371257206d912c5dba4f903622538d9398d`, `native.rs`
`f289fd925917ac34d749555ef4a393d96e065d69`); restore steps also check the recorded sha256. All seven
test steps exited 101 by an assertion failure at the lines below (no compile error).

| Mutant | Test | Failure under mutant |
|---|---|---|
| N1 other vantage (`vantage-builder`) | T1 `sb_the_admission_is_decided_for_the_preparations_vantage` | `native_vertical.rs:968`, refused set `[R2]` vs expected `[R3]` |
| N2 constant subject | T2 `canonical_preparation_reaches_the_real_sdk_and_the_existing_consumer` | `native_vertical.rs:320`, subject equality |
| N3 no requirements check | T3 `sb_requirements_that_are_not_the_candidates_refuse_before_any_observation` | `native_vertical.rs:943`, refusal identity (frozen-world case) |
| N4 no delivery regate | T4 `sb_a_pin_replaced_between_the_two_admissions_refuses_before_spawn` | `native_vertical.rs:461` "must be refused" (the spawn ran) |
| N5 regate with admission-time sample | T5 `sb_a_grant_that_expires_between_boundaries_refuses_consumption` | `native_vertical.rs:461` "must be refused" (consumption ran) |
| N6 restore `request` field | T6 compile_fail doctest `native.rs` line 41 | doctest FAILED (4 passed, 1 failed) |
| N7 constant operation | T7 `sb_the_boundary_operation_is_derived_from_the_binding` | `native_vertical.rs:1029`, `ne` |

Retired as replaced claims in `sb/MATRIX.md`: `sb_an_admission_for_another_subject_or_candidate_refuses_before_spawn`
and M5. S-B mutants M1-M3, M9-M11 were **not re-run** and M4, M6-M8 anchor on text this change
altered (not re-runnable as written); their mapped tests pass in the gate, but killing those mutants
on the corrected code is `not established`. Two limits of T1/T6 are stated in MATRIX: the T1 world
does not admit V' (R2 refuses, seen under N1), so T1 shows refusal for the preparation's vantage,
not that a caller-chosen V' would have been admitted; T6's error codes are not enforced by stable
rustdoc (S-A N-1 class). The design review's zero-candidate-digest variant of the T2 mutant was not
run (one mutant per row).

## §4 inventory as implemented (every input that reaches `admit` / `evaluate` / `admission_state`)

| Input | Handling as implemented |
|---|---|
| request vantage | **host-derived**: `binding.preparation().model_context.vantage.id` (T1, N1) |
| request subject | **host-derived**: `binding.request().subject` (T2, N2) |
| request candidate digest | **host-derived**: `binding.candidate().content_digest()`; S-A O3 checks the supplied requirements derive it |
| request requirements | **digest-bound preimage**: refused before any observation unless its digest is the candidate's requirements digest (T3, N3) |
| request organization id | **labelled assertion**: `FixtureAuthority.organization_id`, `caller_asserted_unanchored`, reaches `AdmissionSummary::asserted()`; unanchored (no record relates an organization to a candidate or problem scope) |
| realization digest | **host-derived** (unchanged): `realization_digest(&realization_description(cfg))`; inherits S3 `p3_carried[0]` (same digest for every `node`, `sdk_root`, `control`) |
| declared `actor_identity` | **labelled assertion**: `caller_asserted_unanchored` in the summary (owner decision O8) |
| declared `installed` | **labelled assertion** by role (O8); **not** surfaced in the summary (S-A has no accessor) |
| boundary operation | **host-derived**: `boundary_operation(binding)`, epoch 1 (T7, N7); O10 unchanged |
| time (`now_millis`) | **host-sampled** from a synthetic `TimeSource` at each evaluation: 2 admissions + delivery regate + consumption regate (T5, N5); the source itself is synthetic |
| authority `source` | the one input meant to be supplied; synthetic; held in `HeldAdmission` so `regate` cannot differ |

Gate input count: 11 rows. The host-built `OperationRequest` has all five S-A fields set by the one
constructor call, so a field added to `OperationRequest::new` is a compile error there.

## Not done

- No independent review of this correction; no checkpoint/freeze (packet: no commit).
- The `consume_fixture_result` public route, the `test_support` feature split (decision 1), the
  organization anchor record, the declared actor/installed O8 gap, the `node`/`sdk_root`/`control`
  presumed inputs, O10, fixture standing and assumed mapping kinds are carried, listed in
  `sb/RESULT.md` "Added by remediation 1".
- S-B mutants M1-M11 not re-run on the corrected code (see Mutants).
- A world in which a caller-chosen V' would be admitted was not constructed; whether one exists in
  this fixture is `not established`.
- `Cargo.lock` / `Cargo.toml` unchanged; no CI, push or integration.

Process disposition: no child, listener or background process left (the gate and mutant custody runs
reported `leftovers=0` per step). Handoff: this file. Continuation: none (completed).
