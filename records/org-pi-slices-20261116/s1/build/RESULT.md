# S1 builder result: R1 acceptance evidence made runnable and discriminating

Role: worker (Claude Code, requested and observed model `claude-opus-5-5`). Packet: `s1/build/PACKET.md`. Parent, collector and acceptor: Pi root `01a1e56e-be12-7424-977b-c7c7983887d0`. No children were launched. No processes are left running: every custody record reports `leftovers=0`.

## Entry

- Worktree `/home/bline/cbm-work/wt-p1-vertical-20261113`, branch `p1-pub-vertical-20261113`, HEAD `e20a9221a80544c81ea789148a02c0f3aad9c84c`, status clean (`git status --porcelain` was empty).
- Read: SKILL.md, worker.md, call-cost.md, AGENTS.md, PLAN.md R1, ROOT-S1-DESIGN-DECISION.json, design-recheck/REVIEW.md, and the RL-001/RLA-001..004 passages of the two Astra architecture reviews.
- `docs/contracts/core.md` does not exist in this worktree (Glob over the worktree found no `**/contracts/core.md`). I did not read DESIGN.md, to save budget.
- Not done: the chatboard read (packet: no listeners; the parent observes). No identity binding or context-status binary was delivered, so mid-task context checks were not run (`usage_unavailable`).
- Toolchain: cargo 1.92.0 (344c4567c 2025-10-21), rustc 1.92.0 (ded5c06cf 2025-12-08), node v22.23.2 (found on PATH).

## Files changed (uncommitted; index untouched)

| File | Reason |
|---|---|
| `rust/crates/work-engine-review-delivery/tests/common/mod.rs` | **P-1/P-2.** Deleted the `run_evidence` module (`WE_RUN_DIR`/`WE_INVOCATION_ID`/`WE_SOURCE_MANIFEST_SHA256`/`WE_MAP_SHA256`, the `Evidence` bundle writer, output manifest, retained scratch) and the `capture` module (qualifier bundle files). Added `native_inputs`: `sdk_root()` reads the explicit `WE_PI_SDK_ROOT` and `WE_PI_SDK_MANIFEST` and refuses, with a reason, when either is missing or mismatched. It checks that the manifest bytes have sha256 `cf9d096e…0985`, that the manifest records entry sha256 `4569157e…7d7ff`, and that the entry file under the root has those bytes. Also added a `Scratch` root under `CARGO_TARGET_TMPDIR`, removed on success and kept, with its path printed, on failure. |
| `rust/crates/work-engine-review-delivery/tests/native_vertical.rs` | **P-1.** All verdicts are asserted in-process. `DeliveryCapture` is replaced by a `Diagnostics` guard that prints raw stdout frames, stderr and facts only when the test unwinds. The hardcoded `SDK_ROOT` is gone. Deleted two tests that only tested bundle machinery: `post_delivery_failures_keep_native_evidence_states_and_scratch` and `wrong_control_verdicts_and_manifest_failure_retain_returned_facts`. Rewrote `refusal_controls_are_captured_for_the_qualifier` as `each_refusal_stage_reports_its_actual_consumer_invocations`, keeping the same three stages and counts. Renamed `recursive_wire_duplicates_…_and_retain_exact_bytes` to `recursive_wire_duplicates_refuse_before_consumption` and dropped its file reads. **RL-002:** removed the `attempt_ms + cleanup_ms + 1000` wall-clock assertion. Only "recorded ⊆ caller-observed" remains. The `hang` control now uses the standard 60 s / 5 s bounds. **RLA-003:** added `recorded_elapsed_covers_the_post_loop_digest_and_finalization`. |
| `rust/crates/work-engine-review-delivery/tests/pure.rs` | **P-3** test: `frames_with_unknown_members_are_invalid_and_never_a_permitted_success`. |
| `rust/crates/work-engine-review-delivery/src/lib.rs` | **P-3:** `#[serde(deny_unknown_fields)]` on `Allocation`, `Counters` and `NativeObservation`, with docs. |
| `rust/crates/work-engine-review-delivery/src/native.rs` | Doc only. The `NativeConfig::cleanup` doc no longer claims "the whole attempt ends within timeout + cleanup". That contradicted `run_attempt`'s own doc, which says the bounds are checked, not preemptive. |

`git diff --stat` (final):

```
 rust/crates/work-engine-review-delivery/src/lib.rs |  10 +-
 .../work-engine-review-delivery/src/native.rs      |   3 +-
 .../tests/common/mod.rs                            | 490 +++------------
 .../tests/native_vertical.rs                       | 658 ++++++---------------
 .../work-engine-review-delivery/tests/pure.rs      |  36 ++
 5 files changed, 289 insertions(+), 908 deletions(-)
```

No other crate changed. The neighbour-crate files mutated in MATRIX were reverted, and the final diff does not include them.

## Test runs (custody records under `s1/build/runs/`)

Common env: `CARGO_TARGET_DIR=s1/target`, `WE_PI_SDK_ROOT=/home/bline/cbm-work/pi-artifact-handoff-20261113/pi`, `WE_PI_SDK_MANIFEST=…/pi.manifest.json`. All runs used `--offline --locked`. No `WE_RUN_DIR`/`WE_INVOCATION_ID`/`WE_SOURCE_MANIFEST_SHA256`/`WE_MAP_SHA256` was set.

| Record | Steps | Outcome |
|---|---|---|
| `runs/compile-1` | delivery `--no-run` | failed: `adapter_name` is private (my error, fixed) |
| `runs/delivery-dev-1` | delivery tests, `--features fixture` | passed: native 20, pure 8, binding 6, lib 0, doc 1 |
| `runs/final-1` (plan `plan-final.json`) | fmt, clippy, four crates, authority-store, native ×3 | **failed 3/7.** fmt diffs and clippy `type_complexity` were in my test code. native-2 and native-3 each failed `nonzero_exit_…` on the `hang` control: "the observation was received before the timeout". The SDK child emitted no frame within its 20 s deadline. Host load average was ~30 on 12 CPUs (`uptime`), and the suite took 183–360 s against 73 s earlier. native-1 passed. |
| `runs/final-2` (same plan, after the fixes) | `cargo fmt --check` (4 crates); `cargo clippy -p work-engine-review-delivery --features fixture --all-targets -D warnings`; `cargo test` for organization, preparation, review-episode-core and delivery with `--features work-engine-review-delivery/fixture` (enables `work-engine-review-preparation/fixture`; organization and episode-core have no features); authority-store `--features fixture --test store`; native_vertical ×3 | **passed 7/7.** Four crates: 239 passed, 0 failed across 20 test binaries. Authority-store: 37 passed. Native: 20/20 in each of 3 runs (230.8 s, 157.4 s, 354.1 s under load). Summary sha256 `86296208522483a3189fd4cfa84566103e1b1ea60b1c6c9bc897641a4f951dfb`. |

These runs are on the uncommitted worktree, not on a clean export of a checkpoint. The root's checkpoint and clean-export runs are still to be done.

## Matrix

`/home/bline/cbm-work/org-pi-slices-20261116/s1/build/MATRIX.md` has 12 recorded mutants covering the 5 R1 checks, the cross-module requirement, the 5 product findings and P-3. All 12 were killed. Two rows had no discriminating test, so tests were added: RLA-003 and P-3. The result-contract half of the R1 "altered result contract or subject" check has named tests but no mutant. P-6 stays `not established`.

**Method deviation (disclose to reviewer):** mutants were applied in place in the worktree, one at a time, and reverted. A scratch copy was not used because this session's permission mode denied every shell copy. MATRIX.md gives the evidence that nothing was left behind: an identical diff stat after the reverts, and final-2 passing.

## P-3 decision

Strict decode was adopted. The adapter (`adapter/review-delivery-adapter.mjs:99–104`, `:181–186`) emits exactly the struct members of `Allocation` and `NativeObservation` (plus `type`, which is removed before decoding), and `counters` is exactly `{ordinary_stream, fetch}` (`:18`). An extra member therefore means the frame is not the pinned producer's. That is the same class as WR7-001, so the frame is now invalid and the stream is not a permitted success. Nested `model`, `loader`, `loader_calls`, `context` and `history` remain `Value` and are not strict. The frozen qualifier may now disagree with this decoder; it is not maintained.

## Negative-control dispositions

| Finding | Disposition | Basis |
|---|---|---|
| ORG-PV-RL-001 (send after peer close sometimes `Completed`) | **Kept** | The current control (`a_send_after_the_peer_closed_…`) observes the peer's fd 0 closed. It also waits until a scan of the readable `/proc` fd tables shows no reader of the pipe before it returns the send. A transiently inherited reference, such as a sibling fork that has not yet exec'd, is therefore waited out rather than raced. It passed in 4 of 4 native runs (dev-1, final-2 ×3) plus final-1 native-1. In final-1 native-2 and native-3 it passed, and only the `hang` case failed. |
| ORG-PV-RLA-001 (duplicate start adjudication) | **Kept** | The adapter emits its observation only after host input ends (`await hostEnded`, adapter:138–146, 187), so readline processes the second start before any observation can exist. `the_adapter_refuses_a_wrong_session_and_a_duplicate_start` passed in every run. |
| ORG-PV-RLA-004 (sleep-based Completed/partial controls) | **Kept** | The controls now observe write progress causally: unread bytes are read only in a callback after the one that returned the send, with 50 ms TICKs keeping callbacks recurring. A deterministic seam test, `coalesced_markers_never_move_…`, covers the probe itself. The 300 ms owner delay forces coalescing; it does not stand in for causality. Passed in every run. |
| ORG-PV-RL-002 (wall-clock bound breached) | **Assertion removed** | No owning requirement sets a timing bound. `run_attempt`'s doc states that the bounds are checked, not preemptive. The `attempt+cleanup+1000 ms` assertion was removed rather than tuned. The untruthful `NativeConfig::cleanup` doc was corrected. Deadline termination is still asserted causally: `timed_out`, a Deadline kill and a signalled reap. |
| `hang` control (newly observed, final-1) | **Fixed** | Its 20 s deadline made "the observation arrived before the deadline" depend on host load. It now uses the 60 s / 5 s bounds that every other SDK control already uses, and it passed in 3 of 3 runs under the same load. It is still a timing precondition, a less tight one. Under heavier load it could fail the same way, and so could the positive control. |

The injected-wait test keeps `elapsed_ms < 20_000`, which is an 18.5 s margin between a 1.5 s cleanup and a 20 s deadline. It is redundant with `!timed_out` and `kill = WaitFailed`, and I kept it.

## Unfinished / for the root

- Checkpoint commit and clean-export custody runs (root).
- The fresh semantic review should look at the in-place mutant method and at the M12 timing-margin test (MATRIX notes).
- P-6 (`ReviewResult::parse` and contract version) remains `not established`. P-4 and P-5 are unchanged.
- The frozen qualifier, preparation scripts and `verifier_bootstrap.py` were not run or modified. They will no longer find the bundles they read.
- Cost of this session: `not established`. I have no usage reader.

## Explicitly unclaimed

- Fixture standing only: synthetic declaration, admissions and result. There is no live provider, no model-quality claim and no real review.
- No `AdmittedOrganization` or `DispatchAuthorization`. Preparation blockers and the realization mismatch remain.
- No hermetic or reproducible build and no build-input closure. The binaries are whatever Cargo built under the ambient registry, toolchain and configuration.
- The SDK pin covers the handoff manifest's digest and the entry module's bytes only. The rest of the SDK tree is not re-verified against the manifest.
- No tamper evidence: the run records are actor-recorded custody logs.
- No confinement of the SDK child; the descendant check is a point-in-time sample.
- No exhaustive SDK audit. "Provider-disabled" rests on the adapter's stream replacement plus zero counters, empty tools and one request in the observed runs.
- No timing guarantee, and no claim that a peer consumed a sent frame. Pipe acceptance is the only claim.
- These are worker results on an uncommitted tree. They are not acceptance, private readiness or integration.

`map_ready`. The result is at `/home/bline/cbm-work/org-pi-slices-20261116/s1/build/RESULT.md`, and the matrix is next to it as MATRIX.md.
