map_ready

# S3 remediation 1: F-1, F-2, F-3, F-4

Role: bounded builder. Input: `s3/review/REVIEW-1.md` §2 F-1..F-4 and §7; root ruling on F-1: option 1.
Worktree: `/home/bline/cbm-work/wt-p1-vertical-20261113`. Nothing committed, staged or branched.
Source model: Claude Sonnet 5.5 (`claude-sonnet-5-5`), requested and as stated by the harness; usage and cost: not established (not read).

## Entry check

- `git diff --stat 24597fef` at entry listed exactly four paths, all deletions: `src/gate.rs`,
  `tests/gate.rs`, `tests/fixtures/assumption-world/{DERIVATION.md,requirements.json}`. These are
  untracked in the worktree (checkpoint has them), so git reports them as removed. No other tracked
  file differed.
- `git status --porcelain --untracked-files=all` listed the same four untracked files and ten
  modified tracked files, nothing else.
- `git hash-object` of the four untracked files equals the blob ids in `git ls-tree 24597fef`:
  `gate.rs a2a1e0cb`, `tests/gate.rs e564833a`, `DERIVATION.md dc734046`, `requirements.json b50d1623`.
- Entry state therefore equals the checkpoint. The entry check was run after reading `REVIEW-1.md`
  and before the first edit.

## Changes and reasons

All in `rust/crates/work-engine-review-delivery`.

1. **F-1** (`src/native.rs` doc of `realization_description`; `DERIVATION.md` "What is assumed, and what is
   not established"; `tests/native_vertical.rs` module doc and the `mutation: none` assertion message).
   Removed "not a true statement" / "assumed, not true" for the mutation member. Now: claimed = no
   granted tool, hence no model mutation route; backed by construction (adapter `noTools`, empty tools)
   and refusal at allocation (`BoundAttempt::bind`) and request (`verify`); not established =
   process-level writes by the unconfined child; evidence gap = `realization-admission` blocker, passed
   only by the labelled assumed discharge; realization checks stay never-assumable. `ReadOnlyClaimed`
   is unchanged.
2. **F-2** (`src/native.rs`). `realization_description(cfg)` now binds `adapter` and, when it differs from
   `NativeConfig::default_adapter()`, returns `provider_family: None`, `execution_confinement: None`,
   `mutation: NotEstablished`, no tools, `continuity: None`. Otherwise `runner_realization()` as before.
   `control` is not keyed. The old sentence "No NativeConfig field selects a provider…" is replaced:
   `node`, `sdk_root`, `control` are presumed inputs; path equality is name-level correspondence, not a
   pin. The exhaustive destructuring is kept. Test added:
   `a_non_default_adapter_is_refused_before_spawn_because_the_description_differs`
   (assumption world, `cfg.adapter` = a path in the scratch root) asserting `Refused{BeforeSpawn,
   RealizationDiffers}`, no run, `spawn_attempts() == 0`, `consumer_invocations() == 0`, empty fixture
   root, episode state unchanged.
3. **F-3** (`src/native.rs`, module doc and `deliver_and_consume` doc). The absence sentences now say
   "the only route that runs the SDK delivery protocol from a binding" and state that `run_attempt` is
   a public generic spawn route and not gated. `run_attempt*` visibility unchanged.
4. **F-4** (`src/native.rs`). `Delivered` gains `assumed_before_spawn` and `assumed_before_consumption`
   (both `AssumedDischarges`). `deliver_and_consume_observed` keeps both evaluations; refusal paths
   are unchanged. Variable `cleared` renamed (`pre_spawn` in `run_native`, `assumed_before_spawn` in the
   entrypoint). `positive_assertions` now asserts, for both boundaries, the boundary value, the label
   `ASSUMED_DISCHARGE_LABEL`, and that `assumed()` equals the (id, kind) list of the delivered
   preparation's blockers, which is non-empty on the assumption world.

`rustfmt` was applied to the two crates once (`runs/fmt-apply`); the preceding `--check`
(`runs/fmt-crates`) reported diffs only at two places, `native.rs:1367` and `native_vertical.rs:246`,
both in code I had just written.

`git diff --stat 24597fef` for the three changed files:

```
 .../work-engine-review-delivery/src/native.rs      | 93 +++++++++++++++-------
 .../tests/fixtures/assumption-world/DERIVATION.md  | 37 ---------
 .../tests/native_vertical.rs                       | 70 +++++++++++++++-
 3 files changed, 132 insertions(+), 68 deletions(-)
```

The `DERIVATION.md` row is an artifact: the file is untracked, so git counts it as removed (37 lines)
rather than as edited. Its edit is the one bullet group named in change 1. Content hashes now:
`DERIVATION.md 8c50e8ff`, `native.rs 513210b4`, `native_vertical.rs b6a18687` (git blob ids).
`git status` after the work lists the same 14 paths as at entry (no new files in the repository).

## Runs

Under `custody.py run`, outputs in `s3/remediation-1/runs/<name>/`. Env as specified
(`CARGO_TARGET_DIR`, `WE_PI_SDK_ROOT`, `WE_PI_SDK_MANIFEST`); cwd `<worktree>/rust`.

| Name | Command | Exit | Result |
|---|---|---|---|
| `fmt-check` | `cargo fmt -p work-engine-review-delivery -p work-engine-review-preparation --check` | 0 | clean after fmt-apply |
| `clippy` | `cargo clippy -p …delivery -p …preparation --features work-engine-review-delivery/fixture --all-targets -- -D warnings` | 0 | no warnings |
| `test-fixture` | `cargo test --offline --locked -p work-engine-review-delivery --features fixture` | 0 | 0, 7, 5, 24 (native, was 23 at checkpoint), 8, 2 passed; 0 failed; leftovers 0 |
| `test-default` | `cargo test --offline --locked -p work-engine-review-delivery -p work-engine-review-preparation` | 0 | 0, 8, 0, 23, 1, 1 passed; 0 failed |

Deviation from the task text: the task lists `fmt --check` without a scope. `cargo fmt --all --check`
(`runs/fmt`) exited 1 with a 338 KB diff; I did not read it, so whether it comes from crates outside
this change is unverified. I scoped the check to the two crates under change, as above.
Native positive tests need the SDK; they ran (`test-fixture`, 70.5 s for the native binary) with the
pinned SDK paths, one run, n = 1.

## Mutant (F-2: ignore `cfg`)

Mutation: `if *adapter != NativeConfig::default_adapter()` → `if false && *adapter != …` in
`src/native.rs` (the description no longer depends on `cfg`). Before: sha256 of `native.rs`
`e21088137d4b4a9cfd7ac56a244d7583e63f6d9570685508baa8a3de579cb7ac`.
Run: `custody.py run … cargo test --offline --locked -p work-engine-review-delivery --features fixture
--test native_vertical a_non_default_adapter` → exit 101, `1 failed; 0 passed; 23 filtered out`; the
test panicked at `native_vertical.rs:625` (`expected a gate refusal`, i.e. the outcome was not a
gate refusal). Record: `s3/remediation-1/mutant-F2/mutant-F2-ignore-cfg.{json,stdout,stderr}`.
Restored by the inverse edit; sha256 after restore is identical to before. The unmutated test passes
in `test-fixture` (above). n = 1 mutant, 1 run, hand-applied; only the one mapped test was run
against it, not the whole suite.

## Not done / limits

- F-5..F-9 are untouched, as instructed (carried as recorded limits).
- I did not re-run the S1/S3 mutants; whether each is still killed after these edits is not established.
- `Delivered` is a public struct and gained two public fields; no other workspace consumer was
  searched for (the review's earlier grep reported none for this crate, not re-run by me).
- Outcome class of the mutant run beyond the panic site (what the host did after the gate passed) was
  not read from the stdout.
- Usage and cost of this session: not established.
- Context check, board read and identity binding: not performed; no binding was delivered to me.
