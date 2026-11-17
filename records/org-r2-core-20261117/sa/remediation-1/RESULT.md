# S-A remediation 1: result

Status: **map_ready** (worker completion; acceptance, readiness and integration are the root's).
Role: bounded builder. Builder: Claude Code, model reported to me as Sonnet 5.5 (`claude-sonnet-5-5`);
effort, cost and usage are `not established` (the parent observes them).
Packet: the S-A remediation 1 brief received in this session (`sa/remediation-1/PACKET.md` exists; I
did not compare it with the brief, so their identity is unverified).
Inputs read: `sa/review/REVIEW-1.md`, `sa/ROOT-RULINGS-1.md`, `sa/RESULT.md`, `sa/MATRIX.md`, all of
`gate.rs`, `lib.rs`, `observation.rs`, `fixture.rs`, `records.rs`, `tests/admission.rs`,
`tests/base_candidate_shape.rs`, and the organization crate's `Profile`, required-authority schema and
boundary coverage lines.

## Entry check

Worktree `/home/bline/cbm-work/wt-org-r2-core-20261117`, branch `org-r2-core-20261117`.

- `git diff --stat 1b8bf301` listed only the 9 crate files, all as deletions: they are untracked in the
  worktree and tracked in the checkpoint. `rust/Cargo.toml` and `rust/Cargo.lock` did not appear, so they
  equal the checkpoint (re-confirmed at exit with `git diff --stat 1b8bf301 -- rust/Cargo.toml rust/Cargo.lock`:
  empty).
- `git status --porcelain -uall`: `M rust/Cargo.lock`, `M rust/Cargo.toml`, 9 `??` crate files.
- `git hash-object` of the 9 crate files equals `git ls-tree -r 1b8bf301` for every file (9 of 9 match):
  Cargo.toml `089f2861`, fixture.rs `e5a485f0`, gate.rs `612d5cb8`, ids.rs `a9cbfc8f`, lib.rs `88c3d853`,
  observation.rs `d1c9ba72`, records.rs `6eab71fa`, tests/admission.rs `f7910eff`,
  tests/base_candidate_shape.rs `8d36e7e7`.
- Result: the working tree equalled the checkpoint at entry.
- `EnterWorktree` (path form) was denied by the session mode; the cwd already is the subject worktree and
  all edits applied there. No commit, stage, branch or other mutating git command was run.

## Changes and reasons

All under `rust/crates/work-engine-organization-authority`, except the two claim documents.

| Ruling | Change | Reason |
|---|---|---|
| F-2 | `admit` takes `SealedObservation` by value (`gate.rs`); doc says a later point check needs another `observe_for`; `admission_state` doc says the same | one observation decides at most one admission; a point check cannot replay the admission-time observation |
| F-4 | `r4` takes the candidate's profile (`request.requirements.profile().as_str()`); one committed acceptance of the digest whose `profile` differs refuses R4 with new `ReasonCode::RealizationProfileMismatch` (subject: the digest). R1/R2 then see no accepted values, as for any R4 refusal | an acceptance attested for another profile no longer admits |
| F-5 | in `covered`, a `PrincipalMatch::Named` name matching more than one **committed** principal returns new `ReasonCode::PrincipalAmbiguous` (subject `authority:<id>`) before any grant is considered | O4 now resolves a named principal like O8's `principal_named` (exactly one committed). With zero committed principals the old path is unchanged, so a revoked principal still reports `PrincipalNotCurrent` |
| F-1 | 11 of the 14 new tests (list below; the other 3 are F-2, F-4, F-5) for the three F4 containment fields (with controls), `AuthorityMissing`, `AuthoritySuperseded`, `ComponentBindingAmbiguous`, the R3 realization-digest mismatch, `VantageUnknown`, `VantageUnbound`, `VantageBindingAmbiguous`, `RealizationAcceptanceAmbiguous`; test world extended with `OwnerDs`, `owner_grant_permission`, `owner_grant_scope`, `binding_digest`, `acceptance_profile`, `Req.authority_boundaries`; recorded mutants (below); `RESULT.md` and `MATRIX.md` corrected | the review's P2: the containment branches had no test while the checkpoint `RESULT.md` said they did |
| F-11 | 3 compile-fail doctests in `lib.rs`: `SealedObservation { .. }` literal, `AuthorityState { .. }` literal, `AuthorityState::new(..)`; plus 1 for F-2 (E0382). Runs with and without `fixture` (the barriers are field/function privacy, independent of the feature) | non-constructibility was static reading only |
| F-12 | `state_refused_when_a_new_commit_makes_the_decision_scope_ambiguous` asserts the exact 4 `AuthorityScopeUnresolved` reasons instead of `refused() == [O4]`; "Explicitly unclaimed" gets the three unrepresented stale conditions | masking control: every negative test asserts reason codes |
| F-7 | "Explicitly unclaimed" gets "O3 is request self-consistency only"; `CheckId::O3` and `OperationRequest::new` docs say so | review F-7; no code change |
| Choice 1 | `EffectAdmission` doc: "the only value that can permit an effect, and only when the consuming host's own regate ... also passes" | root ruling |

New tests (all `tests/admission.rs`, fixture feature; 29 -> 43 = 14 new): `g1_o4_grant_for_another_problem_scope_refuses_only_o4`,
`g1_o4_decision_scope_of_another_problem_scope_refuses_only_o4`, `g1_o4_decision_scope_without_the_required_action_refuses_only_o4`,
`g1_o4_decision_scope_boundaries_must_contain_the_requirements_boundaries` (non-covering: one of two, unrelated; controls: exact, superset, none),
`g1_o4_no_grant_for_the_permission_is_authority_missing`, `g1_o4_a_superseded_grant_does_not_cover_unless_its_replacement_does`,
`g1_o4_two_committed_principals_with_the_named_id_resolve_to_none`, `g1_o4_two_committed_bindings_of_one_component_are_ambiguous`,
`g1_r3_bound_realization_digest_differs_from_the_realization`, `g2_a_vantage_the_candidate_does_not_have_and_no_binding_refuse_r2_and_r3`,
`g2_two_committed_bindings_of_the_vantage_are_ambiguous_for_o8_and_r3`, `g2_two_committed_acceptances_of_one_digest_are_ambiguous_and_state_nothing`,
`g2_r4_acceptance_for_another_profile_is_not_an_acceptance_for_the_candidate`, `state_a_revocation_after_admission_is_seen_only_by_a_new_observation`.
Existing tests that reused an observation after `admit` now take a fresh `observe_for` (positive control, P-oracle, seal test).

Public-API additions the rulings did not name: two `ReasonCode` variants (`PrincipalAmbiguous`,
`RealizationProfileMismatch`). `ReasonCode` is not `non_exhaustive`, so a consumer matching it exhaustively
(S-B) must handle them. Alternative I did not take: reuse `AuthorityMissing` / `RealizationNotAccepted`
(less informative; the ruling said "one comparison, one test").

## Diff stat vs `1b8bf301`

`git diff --no-index --stat` between the review export (its 9 files hash-equal to the checkpoint blobs,
checked) and the working tree:

```
 src/gate.rs          |  59 ++-
 src/lib.rs           |  45 ++
 tests/admission.rs   | 483 +++++++++++++++++++++-
 3 files changed, 560 insertions(+), 27 deletions(-)
```

Unchanged (hash-equal to the checkpoint): Cargo.toml, fixture.rs, ids.rs, observation.rs, records.rs,
tests/base_candidate_shape.rs, and `rust/Cargo.toml` / `rust/Cargo.lock`. Final sha256:
gate.rs `f2158877...effbc8`, lib.rs `5cae02cc...99b`, observation.rs `42ff4d6d...d183`,
tests/admission.rs `a2fc052b...e19e`. `cargo fmt -p ...` (not `--check`) was run once on my edits before the
runs; it reformatted crate files only.

## Runs (custody, `CARGO_TARGET_DIR=/home/bline/cbm-work/org-r2-core-20261117/target`, plan `sa/remediation-1/plan.json`)

`sa/remediation-1/runs/final/` summary sha256
`cc52a651129feb49cad3b73649aa281a52e6b810f3685d903d77b552edc9924b`: **5/5 exit 0**, leftovers 0. An earlier
run on the same source is in `runs/first/` (5/5; summary `471f8e0f...084f`).

| Step | Result (read from the stdout) |
|---|---|
| `cargo fmt --check -p ...` | ok |
| clippy `--offline --locked --all-targets --all-features -D warnings` | ok |
| clippy `--offline --locked --all-targets -D warnings` (default features) | ok |
| `cargo test --offline --locked -p ... --all-features` | lib 1, admission **43**, base_candidate_shape 1, doctests **7**: all pass |
| same, default features | lib 1, admission 0 (file is `cfg(feature = "fixture")`), base_candidate_shape 1, doctests 7: all pass |

The packet did not ask for `-p work-engine-organization`; it was not run (that crate is unchanged).

## Mutants (neutralize, run the mapped test, restore)

Applied by editing the line in place, run via `custody.py run` into `sa/remediation-1/mutants/<name>/test.*`, restored by
reversing the edit; sha256 of the file after restoration equals the value before in every case
(gate.rs `f2158877...`, observation.rs `42ff4d6d...`). Command shape: `cargo test --offline --locked -p
work-engine-organization-authority --all-features --test admission <test>` (`--doc` for the doctest mutants).
Each "killed" below is a `test result: FAILED` line with a panic at an assertion (no compile error), read from the
custody stdout.

| Mutant | Edit | Mapped test | Result |
|---|---|---|---|
| `o4-grant-problem-scope-skip` | `if false && &g.problem_scope != scope` | `g1_o4_grant_for_another_problem_scope...` | killed |
| `o4-ds-problem-scope-skip` | `if false && &d.problem_scope != scope` | `g1_o4_decision_scope_of_another_problem_scope...` | killed |
| `o4-ds-actions-skip` | `if false && !d.actions.contains(permission)` | `g1_o4_decision_scope_without_the_required_action...` | killed |
| `o4-ds-boundaries-inverted` | `!required.is_subset(allowed)` -> `required.is_subset(allowed)` | `g1_o4_decision_scope_boundaries_must...` | killed |
| `r3-skip-realization-digest` | `if false && binding.actor.realization_digest != ...` | `g1_r3_bound_realization_digest...` | killed |
| `o4-named-principal-not-exactly-one` (F-5) | `.count() > 1` -> `> 99` | `g1_o4_two_committed_principals...` | killed |
| `r4-ignore-profile` (F-4) | `if false && record.profile != profile` | `g2_r4_acceptance_for_another_profile...` | killed |
| `state-new-public-doctest` (F-11) | `pub(crate) fn new` -> `pub fn new` in `observation.rs` | doctests | **survived** (7/7 pass) |
| `state-new-and-entry-public-doctest` (F-11) | `new` public **and** `pub(crate) struct Entry` -> `pub struct Entry` | doctests | killed (the `AuthorityState::new` doctest, line 76, `compile fail ... FAILED`) |

The first F-11 mutant surviving is a finding about the doctest, not a clean kill: with `new` public the
doctest still fails to compile, because `Default::default()` needs the private type `Entry` (an uncoded
error). Non-constructibility therefore rests on two independent barriers, and the doctest alone does not
pin the `pub(crate)` on `new`; it does detect both barriers being opened. Whether rustdoc on this toolchain
enforces the code in `compile_fail,E0624`: `not established` (I did not run the check that would show
it). Evidence that each new doctest fails for its intended reason on the unmutated source: a diagnostic run
with the four attributes temporarily set to `no_run`, `sa/remediation-1/diagnostics/doctest-errors/test.stdout`
(E0451 x2, E0624, E0382); `lib.rs` was restored afterwards (sha256 equal to the value before).

## Not done

- No mutant for the F-2 signature change (a `&SealedObservation` mutant is a compile failure, not an
  assertion kill; the E0382 doctest covers it).
- No recorded mutant for the other new tests (`AuthorityMissing`, `AuthoritySuperseded`,
  `ComponentBindingAmbiguous`, the vantage-resolution reasons, `RealizationAcceptanceAmbiguous`), nor for
  `RevisionConflict` / `IssuanceRetracted`; only the containment family, R3 digest, F-4 and F-5 have mutants.
- F-11's O9 wiring (`InternalError` filter, `view(c).err()`) is still tested at function level only; not changed.
- Not taken per ruling: F-6, F-9, `o8_bindings`. S-B obligations (F-2 regate observation, F-3, F-7, F-8, F-10)
  are recorded in `sa/RESULT.md`, not implemented.
- `-p work-engine-organization` tests not re-run. The remediation has not been independently reviewed.
- Coordination: no AGENTS.md board read, no context listener (packet: none), no identity binding message
  received or acknowledged, and the mid-task context check was not run (no pinned binary was provided).
  My own usage and cost are `not established`.
- No forbidden path was read; no network, Docker or commit.

Files changed outside the crate: `sa/RESULT.md` (F-1 correction, F-7/F-12 unclaimed items, choice 1, status),
`sa/MATRIX.md` (rows and corrected closing paragraph), and this directory. Continuation: none (assignment complete).
