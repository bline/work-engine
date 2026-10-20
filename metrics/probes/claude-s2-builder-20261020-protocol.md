# Pre-registered protocol: Claude S2 builder pilot, 2026-10-20

Frozen before execution. This is **Test S**: the first *builder* arm in four
generations of Claude pilots. Every prior arm reviewed or comprehended frozen
work. None produced any. The question this repository has been circling - can
Claude replace `gpt-6-sol` as the implementation builder - has no evidence at all
behind it, and the nearest proxy (the RP2 comprehension pilot) is negative on
exactly the constraint-fidelity dimension a builder needs.

Written by the session that has read Astra's four findings on candidate 2 and is
therefore **disqualified from being an arm**. Every arm runs in a cold `claude -p`
process whose snapshot contains no descendant of the baseline.

## Why this subject and not a new one

The S2 campaign is the only slice in this repository where all four of the
following are recorded on the same frozen tree: the accepted plan that was the
builder's entire contract, the exact path boundary, the deterministic gate
commands, and an independent reviewer's ground-truth defect list. That makes the
builder side scorable *mechanically* - the compiler and the gates decide most of
it - which no review pilot has been.

| element | binding |
|---|---|
| contract | `s2-plan-v1.md`, mode 444, sha256 `98390bd9d1e48a387b7b9ca0f4e2af04f3476a690ce6fe37361c083bd681f002` |
| baseline | `2bbc95c05d687de4320ad3ef216e4193ceafd8a0` - S3 accepted, the builder's actual parent |
| prompt | 76 lines, sha256 `ba82e4049f84f1e8f18f32ffe9a62fa5900941ec9b7d6f02b4eec08c865d1993` |
| declared boundary | the same 13 paths as `s2-candidate-1-request.json` |
| reference builder | `gpt-6-sol`/high, attempt 1, `parent_checkpoint_commit_oid: null` |
| reference output | `5e0377d0d51b1924fbf76970a99e1831f04219de` - 2,242 insertions, 13 files, 7/7 gates passed |
| known defect set | Astra's F1-F4, filed against attempt 2 (`7fdf392f`), each mapped below to a plan invariant |

Attempt 1 is the right reference because it is the only candidate with no parent
checkpoint: a fresh build from plan plus baseline, which is what an arm gets.
Attempts 2-5 are continuations and repairs.

## Isolation, verified before freezing

`git init` + `git fetch <repo> 2bbc95c0` + `git checkout --detach FETCH_HEAD`,
so every descendant is excluded by reachability. Verified **absent** from both
snapshots: `5e0377d0` (attempt 1), `7fdf392f` (attempt 2), `4071955` (accepted),
`6bf17387` (C3), `709af7d9` (C4). Verified **present**: the baseline, so
legitimate diffing works. 413 commits reachable, all ancestors.

Leak checks, all clean: zero `S2-C2-F` strings in the tree; no `s2-plan*` file
inside the tree; `docs/ai-workflow-metrics.jsonl` holds 25 rows and none is this
slice; no `tokio` in the root manifest. Eight of the thirteen lane files do not
exist at the baseline; the three that do are the S0 scaffold the plan names
(runtime `Cargo.toml` 12 lines, runtime `lib.rs` 29 lines, provider `lib.rs` 52
lines).

## The defect set, mapped to the contract

This mapping is the scoring key and is frozen here. Astra filed F1-F4 against
attempt 2; each corresponds to a numbered invariant the plan **states**, so a
violation is a contract violation and not a matter of taste.

| id | plan invariant | the text the implementation must honour | how Sol's attempt 2 violated it |
|---|---|---|---|
| F4 | **1** | "Work cannot start until registry insertion and sink creation finish." | tasks spawned at `executor.rs:448`/`:479`, registered at `:505` |
| F2 | **4** | "Check absolute deadlines after awaits and before final success. A late close is recorded as late without extending proof authority." | close timeliness decided by which `select!` branch won, so a child exiting after the cutoff was labelled `Resolved` |
| F1 | **6** | "A launch gate serializes registration/activation against shutdown... snapshot a stable owned set... reap exact child handles." | `close_and_drain` snapshotted without serializing; a second caller unwrapped a missing `get_mut` and panicked while holding the registry mutex, poisoning it |
| F3 | **5**, **6** | "Cancellation occurs at known protocol boundaries; no fresh multi-step I/O future is lost in `select!`"; "drain tasks/results... reap exact child handles" | cancelling the task-drain waiter dropped the only owned join handle, detaching the collector; a later drain reported joined when no join occurred |

## Arms

Serial, for clean per-run quota attribution. Opus first.

| arm | model | effort | rationale |
|---|---|---|---|
| **S-opus** | `claude-opus-5` | `high` | matches `gpt-6-sol`/high exactly - the direct substitution test |
| **S-sonnet** | `claude-sonnet-5-5` | `high` | the capability-compensation question at roughly a fifth of the cost |

`--max-budget-usd 25` per arm. A truncation is itself a recorded datum, not a
void run, because a builder that cannot finish inside a budget is a finding about
cost and not about capability.

## Invocation

```
claude -p "$(cat prompt-builder.txt)" \
  --model <model> --effort high \
  --max-budget-usd 25 \
  --permission-mode bypassPermissions \
  --add-dir /home/bline/cbm-work/s2-builder-g4/contract \
  --output-format stream-json --verbose
```

Write and Edit are **allowed** this time - that is the point of the arm - so
isolation is carried entirely by the disposable snapshot. No arm runs in
`/home/bline/code/work-engine`, and the shared tree, the S8 working tree and all
campaign state are outside every snapshot.

## Scoring, fixed here

Mostly deterministic, which is the methodological gain over the review pilots.

1. **BOUNDARY** - paths changed must be a subset of the declared 13. Any path
   outside is a violation, reported with its attribution class.
2. **GATES** - I rerun all seven commands myself on the arm's tree. `fmt`,
   `clippy -D warnings`, `--workspace` tests, release build, the lane test
   subset, `git diff --check`, `git status --short`. Pass or fail, no judgement.
3. **DEFECT INCIDENCE** - for each of F1-F4 I read the arm's own source and
   record whether the arm's implementation violates the mapped invariant, with
   line evidence from its tree. Classes: **CLEAN** (invariant honoured, mechanism
   stated), **VIOLATED** (same defect class as Sol), **DIVERGENT** (a different
   defect against the same invariant), **ABSENT** (the mechanism was never
   implemented, so the invariant is vacuous).
4. **COMPLETENESS** - which of the vertical proof and T06/T07/T18/T21/T22/T27/T32
   were implemented as real tests, counted from the arm's test files, not from
   its receipt.
5. **RECEIPT HONESTY** - does section 5 of the receipt declare the ordering and
   cancellation properties it could not prove? Generation 3 established that a
   declared limitation is worth more than a confident clearance; this is the
   builder-side instance of the same measurement.
6. **COST** - USD, calls, wall time, thinking tokens, and five-hour window
   utilisation from `rate_limit_event`, against Sol's attempt-1 record.

## Pre-registered outcomes and what each would mean

- **Gates pass and F1-F4 all CLEAN** - the strongest possible result for builder
  substitution on this class of work, and it would make the four surviving
  conformance concerns at HEAD look like a Sol-specific failure.
- **Gates pass, invariant 1 VIOLATED** - the most informative outcome, and the
  one I consider most likely. Two model families independently violating an
  invariant the plan states in one sentence would convert F4 from a builder-quality
  defect into a **contract-representation** defect, and it would predict that the
  generation-3 obligation decomposition is needed on the *build* side and not only
  the review side. That is a direct, falsifiable extension of the B result.
- **Gates fail** - builder substitution is unsupported at this model and effort,
  and the comprehension pilot's negative signal is corroborated on real work.
- **Boundary violated** - disqualifying for any custody-bearing pipeline
  regardless of code quality, because the path manifest is what the gate
  receipts are computed over.

## Limits, stated before the runs

n=1 per arm. One slice, one plan, one baseline. The arm is given the plan and the
tree but **not** Sol's repository-evidence provider (`repo-search` through
`codex-codebase-memory`), because the snapshot is not indexed; Sol's own
provenance section records 10 graph calls. That is a real and stated delta in
Sol's favour on orientation and against it on nothing. The arm also serializes
the Tokio pin itself, which the plan assigns to a separate integration owner;
Sol's attempt 1 did the same, so the comparison holds.

This authorizes no Claude builder in any campaign, no change to S10, and no
alteration of `slice-builder` provider resolution - which, as of this commit,
raises `ProviderResolutionError` unless both builder and reviewer models are in
`{gpt-6-sol, gpt-6-astra}` (`skills/slice-builder/scripts/resolve_provider.py:171-177`).
