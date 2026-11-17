# S1 remediation 1 — result (F-1, F-3, F-5, F-6(b))

Status: `map_ready`. Worker: Sonnet 5.5 (requested; no runtime identity read). Usage/cost: unavailable to me, not measured. No children, listeners, commits, staging or git mutation.

## Entry check
`git diff --stat 1ac0bff4` (tracked files) was empty at entry. Branch HEAD `e20a9221`, the 5 changed files uncommitted. Held at checkpoint `1ac0bff4`.

## Files changed (all tracked, uncommitted, relative to `1ac0bff4`)
| File | Change | Finding |
| --- | --- | --- |
| `rust/crates/work-engine-review-preparation/tests/fixture_consumer.rs` | +41: `a_result_with_another_contract_version_is_refused_by_the_episode_owner`. The acceptable sample with `"schemaVersion": 2` → `ConsumeRefusal::Episode` with `ErrorKind::ResultContract`; `canonical(&state)` unchanged; the unedited sample is then consumed `Ok` from the same state, so the version is the only difference. | F-1 |
| `rust/crates/work-engine-review-delivery/src/attempt.rs` | Module doc no longer says "the capture writes them" or "the campaign qualifier strictly decodes them"; now "a consumer strictly decodes them". | F-5 |
| `rust/crates/work-engine-review-delivery/src/binding.rs` | `consumer_invocations` doc: "evidence for tests and capture" → "evidence for tests". `consumer_invocations()` doc: "synthetic test bundles supply their own count" → "the count is only what the consumer actually did". | F-5 |
| `rust/crates/work-engine-review-delivery/src/native.rs` | `NativeConfig::sdk_root` doc no longer says "pinned"; it says product code does not verify a pin. Removed `DeliveryStage`, `OutcomeProjection`, `NativeFailure` (about 58 lines). | F-5 |
| `rust/crates/work-engine-review-delivery/tests/native_vertical.rs` | Removed the 5-line `elapsed_ms < 20_000` assertion. The `!timed_out`, `KillReason::WaitFailed`, `ReapFact::WaitFailed` and `w.calls > 1` assertions are kept. | F-6(b) |

F-3: no source change. I ran the default-feature test once (below).

### Removal basis for the three types
I searched `DeliveryStage|OutcomeProjection|NativeFailure` over all of `rust/` (before the edit) and found hits only in `native.rs` lines 1183–1227 (their own definitions and impls). A second search over the whole worktree excluding `rust/` returned one file, `records/organizational-compiler-pi-wip-20261114/qualifier-source/src/outputs.rs` (the frozen qualifier; not counted, per the packet). Nothing under `rust/` used them, and the build and tests after removal pass, so I kept nothing. The search covers `rust/` and the whole worktree by text; I did not search outside the worktree.

### F-5 residue not changed
`lib.rs` lines 102 and 277 and `binding.rs` line 126 etc. use "synthetic result" for the bound result text. That is the live product term for the text Rust supplies. I did not read it as bundle/qualifier residue, so I left it unchanged.

## Final `git diff --stat 1ac0bff4`
```
 .../work-engine-review-delivery/src/attempt.rs     |  6 +--
 .../work-engine-review-delivery/src/binding.rs     |  4 +-
 .../work-engine-review-delivery/src/native.rs      | 63 +---------------------
 .../tests/native_vertical.rs                       |  5 --
 .../tests/fixture_consumer.rs                      | 41 ++++++++++++++
 5 files changed, 48 insertions(+), 71 deletions(-)
```
`git diff --stat` against HEAD lists 8 files because `lib.rs`, `tests/common/mod.rs` and `tests/pure.rs` were already uncommitted at the checkpoint; they are unchanged from it.

## Runs (custody records)
Env for all: `CARGO_TARGET_DIR=…/s1/target`, `WE_PI_SDK_ROOT`, `WE_PI_SDK_MANIFEST` as given. Plan: `s1/remediation-1/plan-validate.json`. Records in `s1/remediation-1/runs/validate/` (summary `summary.json`, sha256 `2d3ae79ed4f1170ad3566f4080502ebadb62c7185a7abcdc39c063fd7c81c6a4`), 5/5 passed, `leftovers=0` for every step:

| Step | Exit | Result (from the recorded stdout) |
| --- | --- | --- |
| `fmt` (`cargo fmt --check` delivery, preparation, review-episode-core) | 0 | clean |
| `clippy` (`-p work-engine-review-delivery --features fixture --all-targets -D warnings`) | 0 | clean |
| `test-default-features` (`-p work-engine-review-delivery -p work-engine-review-preparation`, no features) | 0 | delivery 8 passed; preparation 22 passed; doctests: delivery `lib.rs` line 26 compile_fail 1 passed, preparation `lib.rs` line 25 compile_fail 1 passed (the two feature-absence doctests F-3 said had not been compiled) |
| `test-fixture-consumer` (preparation, `--features fixture`, `--test fixture_consumer`) | 0 | 12 passed, including the new test |
| `test-delivery-fixture` (delivery, `--features fixture`, all targets) | 0 | 6 + 20 (native_vertical, 182.87 s) + 8 + 1 doctest passed, 0 failed |

Post-restore re-run of `fixture_consumer` after the mutant: `runs/post-restore/test-fixture-consumer-post-restore.json`, exit 0.

Scope of the F-3 default-feature claim: one run of the two crates, no features, at this subject. The 3 native_vertical repeats of the root gate were not re-run here.

## Mutant F-1 (n = 1)
- Mutation: deleted the `schemaVersion != 1` guard (the 5-line `if` block, `implementation_review_v1.rs` lines 43–47) from `ReviewResult::parse_inner`.
- Record: `s1/remediation-1/mutant-F1/mutant-F1-version-guard-deleted.json` (+ `.stdout`/`.stderr`), exit 101.
- Result: `test result: FAILED. 11 passed; 1 failed`. Only `a_result_with_another_contract_version_is_refused_by_the_episode_owner` failed (panic at `fixture_consumer.rs:684`, the first assertion on the refusal). The other 11 passed with the guard removed. This is the survivor the review predicted (unverified there) for the pre-existing tests of this file; here it is confirmed for this one test file only. I did not run the other 239 tests against the mutant.
- Restore: re-inserted the identical text; `git diff --stat -- rust/crates/review-episode-core` is empty. The post-restore consumer run (above) passes.
- The mutant ran in place, one at a time, on the worktree. The new test file was already present, so the mutated build is not a separate export.

## Not done / limits
- F-1 optional native case beside `wrong-subject-result` was not added (packet scoped one consumer-level test).
- F-2, F-4, F-6(a), F-6(c), F-7, F-8, F-9 were out of scope and untouched.
- F-3's default-feature gate step belongs to the root; I only ran it once.
- Nothing staged or committed; the index is untouched. The 5 files above remain uncommitted in the worktree.
