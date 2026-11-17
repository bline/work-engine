accept_private_readiness

# S1 recheck of the remediated R1 candidate (RECHECK-1)

Reviewer: the same Claude Code session as REVIEW-1, context retained, read-only, 2026-11-17 UTC. Model id stated by the session environment: `claude-fable-5-2`. Independence claimed: separate session and context from the builders only. Advisory to Pi root `01a1e56e-be12-7424-977b-c7c7983887d0`; this is not acceptance, private readiness or integration.

Paths: `S1` = `/home/bline/cbm-work/org-pi-slices-20261116/s1`, `E2` = `S1/export-1cc77114/rust/crates`. Crate shorthands as in REVIEW-1 (`del`, `prep`, `epi`, `org`). Finding ids are REVIEW-1's.

## 1. Verdict

`accept_private_readiness` for checkpoint `1cc77114`. No P1 or P2. The remediation is five small edits; each does what it says, and none weakens a guard or a causal assertion. F-1, F-3, F-5 and F-6(b) are closed with the limits stated below. The open P3 items to carry are listed in section 4.

## 2. Subject identity

Checked by me:

- `git log -1 --format='%H %T %P' 1cc77114` → commit `1cc771146c72ff7d23b371b730aa5e73ce79db61`, tree `be29bec5a48560682e0a9894ef2ff83a864dbe14`, parent `1ac0bff4190cf757b5d79b5e69f04fbdcbed2124`. Matches the request and `S1/checkpoint-2.json`.
- Ref `refs/work-engine/checkpoints/org-pi-slices-20261116/slice-1/candidate-2` resolves to that commit and tree.
- `git diff 1ac0bff4 1cc77114` (read whole): 5 files, +48/−71. `git diff --stat e20a9221 1cc77114`: 8 files, +337/−979. No file under `epi` or `org` is touched, so the guards I read at `1ac0bff4` are unchanged, including `epi/src/implementation_review_v1.rs:43–47` that the F-1 mutant removed and restored.
- `sha256sum` of the 8 task-owned files in `S1/export-1cc77114` equals the 8 `content_digest` values in `S1/checkpoint-2.json` (8 of 8).

Not established by me: the task patch digest, and equality of the export's other files with the checkpoint tree (that rests on the root's stated export method in `S1/ROOT-GATE-3.json`).

## 3. The five requested checks

### 3.1 F-1: the new test discriminates the result-side version guard and leaves state unchanged — confirmed

`prep/tests/fixture_consumer.rs:668–707`, `a_result_with_another_contract_version_is_refused_by_the_episode_owner`.

- **What it submits.** The acceptable sample with only `schemaVersion` set to `2`, through `consume_fixture_result` with the canonical world, declaration, admissions, writer and current revision. Every earlier check on that path passes for this input, so the refusal can only come from the episode owner's result validation.
- **What it asserts.** `ConsumeRefusal::Episode` of kind `ResultContract`; then `canonical(&state)` equal to its value before the call; then that the unedited sample is consumed `Ok` from the same state. The third assertion is the control that makes the version the only difference, which answers the objection that `ResultContract` is also the kind for any other shape error.
- **Mutant (builder-executed, n = 1).** `S1/remediation-1/mutant-F1/mutant-F1-version-guard-deleted.stdout`: with the guard at `implementation_review_v1.rs:43–47` deleted, `test result: FAILED. 11 passed; 1 failed`; the one failure is this test, panicking at `fixture_consumer.rs:684`, which is the `.expect_err` line. So without the guard a result labelled version 2 is consumed. The other 11 tests of that file pass under the mutant, which confirms REVIEW-1's prediction for this file. The mutant was not run against the other test binaries.
- **At the subject (root-executed).** The test is `ok` by name in `S1/root-gate-3/test-four.stdout`; that binary now reports 12 passed.

Standing of the mutant record: in place on the builder's worktree before the checkpoint, against one test binary. The diff shows `epi` untouched, so the restore is confirmed at the subject. This is the same method limit as F-2.

P-6 can now be recorded as: subject half established by test and mutant (M03); version half established by test and mutant (this one); contract-id half unchanged from REVIEW-1 section 4.5 (the v1 result shape carries no contract id; the binding is the single supported-contract constant plus `Selection::refuse_unsupported`).

### 3.2 Removal of `DeliveryStage`, `OutcomeProjection`, `NativeFailure` — safe; nothing lost under `rust/`

- **No user under `rust/`.** Search for `DeliveryStage|OutcomeProjection|NativeFailure` over every file of `S1/export-1cc77114` returns one file: `records/organizational-compiler-pi-wip-20261114/qualifier-source/src/outputs.rs`. At `1ac0bff4` the same search returned that file and the definitions in `del/src/native.rs` only (REVIEW-1 F-5). REVIEW-1's search over every `.rs` and `.toml` file found no workspace crate other than `del` that depends on or names the delivery crate, and this diff adds none.
- **Executed.** The crate compiles and its tests pass in both feature configurations at the subject: clippy `--features fixture --all-targets -D warnings` exit 0; `test-four` and `test-default-features` exit 0, with `Compiling work-engine-review-delivery … (…/export-1cc77114/…)` in both stderr records.
- **Behaviour.** The three types were derived projections of `Result<Delivered, DeliverError>` with serde derives. No function in the delivery path read them, so no check, refusal or fact is removed. `DeliverError` and its three variants are unchanged.
- **Consequence to record.** The frozen qualifier source under `records/` still names the removed types, so by reading it would no longer compile against this delivery crate. That matches the root's decision that the qualifier is frozen history and not maintained. I did not build it.

### 3.3 Doc edits — truthful, with two small wording residuals

- `del/src/binding.rs:137`, `:351`: "evidence for tests" and "the count is only what the consumer actually did". Accurate: the counter is incremented only in `consume`, immediately before the consumer runs.
- `del/src/native.rs:166–167`: no longer claims a pin; states that product code verifies none and the caller checks. Accurate. Residual: "read-only to the child" describes the adapter's behaviour ("writes nothing", adapter header), not an enforced property; there is no sandbox.
- `del/src/attempt.rs:1–4`: the capture and qualifier are no longer named. Residual: "a consumer strictly decodes them" names no consumer. In `del/src` nothing deserializes `AttemptFacts` (the only typed frame decodes are `Allocation` and `NativeObservation` at `del/src/lib.rs:491,503`); the facts are produced and checked in memory. The only decoder I know of is the frozen qualifier. The sentence is accurate as a description of what the strict serde attributes permit, and reads slightly stronger than that.

Neither residual changes a claim the acceptance record relies on. I fold both into F-5 as carried wording notes.

### 3.4 F-6(b): the removed assertion — causal assertions kept

The diff removes only `elapsed_ms < 20_000` from `an_injected_wait_failure_is_not_termination_and_the_child_is_still_stopped`. Still asserted in the persistent-failure half: kill is `Sent { WaitFailed }`, reap is `WaitFailed` with the injected error, `wait_errors` equals the injected list, `pid_present_after_reap` is `None`, `!timed_out`, `w.calls > 1`, the verdict names the failed wait observations, and the test's own `waitpid` collects the child with `SIGKILL`. The transient half is untouched. M11 failed at the kill-reason assertion, which is unaffected. The test is `ok` in `test-four` and in `native-1..3` of the new gate (n = 4 executions).

### 3.5 F-3: the default-feature configuration — now executed by the gate

`S1/root-gate-plan-2.json` adds step `test-default-features`: `cargo test --offline --locked -p work-engine-review-delivery -p work-engine-review-preparation`, no features. `S1/root-gate-3/test-default-features.stdout`: 6 result lines summing to 32 passed, 0 failed (`pure` 8, `preparation` 22, two doc-test runs of 1 each). The two doctests are the feature-absence ones by name: `crates/work-engine-review-delivery/src/lib.rs - (line 26) - compile fail ... ok` and `crates/work-engine-review-preparation/src/lib.rs - (line 25) - compile fail ... ok`. `preparation` shows 22 here against 23 with the feature because one test is gated (`prep/tests/preparation.rs:1031`). So "without the feature the native runner and the consumer do not exist" is now established by execution at this subject, as is the default build of both crates.

## 4. Root gate 3, as read

From `S1/root-gate-3` and `S1/ROOT-GATE-3.json`:

- `toolchain` step output: `cargo 1.92.0 (344c4567c 2025-10-21)`, `rustc 1.92.0 (ded5c06cf 2025-12-08)`, `v22.23.2`. This closes REVIEW-1's "not established" entry for the gate's own toolchain. It observes the first `node` on the gate's `PATH`, which is the lookup the tests use; the positive test still asserts only that the reported node version starts with `v`.
- `test-four`: 20 result lines summing to 240 passed, 0 failed (239 at the previous subject plus the new test). `test-authority-store`: 37 passed. `native-1..3`: 20 passed each. Together with `test-four`, the native binary ran 4 times at this export, 20/20 each.
- The gate reuses `S1/target-export` from the previous export. `test-four.stderr` shows `Compiling` for `epi`, `org`, `prep` and `del` from the `export-1cc77114` paths, so the four crates were rebuilt from the new export before the tests ran.
- `ROOT-GATE-3.json`'s result lines agree with the records. Its "20 binaries" and "6 binaries" count doc-test runs as binaries (16 + 4 and 4 + 2).

## 5. P3 items after this recheck

Closed:

- **F-1** — test and mutant present; limit: mutant run in place, one test binary, n = 1. The optional native case was not added.
- **F-3** — executed by the gate.
- **F-5** — the three types are removed and the three doc sites corrected; two wording residuals carried (3.3).
- **F-6(b)** — removed.

Open, to carry in the acceptance record:

- **F-2** — mutant evidence is in-place, pre-checkpoint, n = 1 per row; the candidate-side result-contract guard (`prep/src/select.rs:171–179`, `org/src/enforce.rs:223`) has tests and no mutant.
- **F-4** — `BoundAttempt::verify` constrains only the system context's preamble section, against the adapter's own report; the canary is checked in the positive test only.
- **F-6(a)** — the elapsed-coverage test's `digest_ms >= 20` precondition is dev-profile evidence. **F-6(c)** — every SDK control, including `hang`, needs the SDK to allocate and observe within 60 s.
- **F-7** — "complete attempt before consumption" is enforced by `deliver_and_consume`, not by the types; the `Control` enum sits in product configuration; "uncertain, not replayed automatically" is a documented caller obligation only.
- **F-8** — (a) the view's `hosting`, `episode_access` and `terminalization` are neither projected nor named as gaps; (b) the `realization-admission` blocker text does not name continuity, integrity or evidence custody; (c) a frame omitting `prompt_error` or `context` still decodes.
- **F-9** — per-condition normative and constraint values are not asserted against the view.

Also to carry, unchanged from REVIEW-1 (a semantic limit, not a finding): the fixture vertical never consults `blockers` or `realization_evaluation`; a realization mismatch is carried as text and unchanged blockers, and never refuses delivery or consumption. The vertical was exercised with one world (n = 1 candidate, obligation and subject).

## 6. Coverage and process

Read for this recheck: the whole remediation diff; `S1/remediation-1/RESULT.md`; `S1/ROOT-GATE-3.json`; `S1/root-gate-plan-2.json`; `S1/checkpoint-2.json`; result, doc-test, toolchain and `Compiling` lines of the `root-gate-3` outputs; the result lines of the F-1 mutant output. Searches: the three removed type names over the whole new export; typed-decode and feature-gate sites in `del` and `prep/tests/preparation.rs`.

Not read: the new export's files beyond the diff (I relied on the diff against the tree I reviewed at `1ac0bff4`); the builder's `runs/validate` and `post-restore` records beyond RESULT.md's table; custody JSON records; the frozen qualifier.

No builds, tests, network, Docker or git mutation. The only file written is this report. No child agents, no listeners, no process of mine remains. Cost of this recheck: not established by me; the parent observes.
