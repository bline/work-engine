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

## S-opus result: every pre-registered outcome I predicted was wrong

`claude-opus-5` at `--effort high`: 74 calls (62 Bash, 10 Write, 1 Edit, 1 Read),
**1,954 s** (32.6 min), **$12.4955**, 162,650 output tokens of which **54,972
thinking**, 0 permission denials, `subtype: success` - not a budget stop, with
$12.50 of the $25 cap used. Five-hour window **0.02 -> 0.17**, so one complete
slice build costs about **15 points**, not the 35-40 I projected mid-run.

### 1. Boundary: PASS

Exactly the 13 declared paths, nothing else. 5,407 insertions, 31 deletions
against Sol attempt 1's 2,242 insertions - **2.4x the code**.

### 2. Gates: 7/7 PASS

`cargo fmt --check` 0. `clippy -p lifecycle-runtime -p lifecycle-provider
--all-targets --locked -- -D warnings` 0. Release build 0. Lane tests 0:
**44 passed, 0 failed** (17 `clock_framing`, 18 `owned_execution`, 9
`process_supervision`). `cargo test --workspace --locked` 0. `git diff --check`
0. Two `allow` attributes exist, both benign - `clippy::too_many_arguments` on an
8-argument internal fn and `dead_code` on test support - and neither silences a
lint indicating a defect.

### 3. Defect incidence: F1 CLEAN, F2 CLEAN, F3 CLEAN, F4 CLEAN

Fairness check first, because it decides whether this comparison is legitimate:
Astra filed F1-F4 against attempt **2**, and attempt 1's handoff records
`review_status: "not selected or run"`. Verified by source - attempt 1 and
attempt 2 are **structurally identical** on the decisive lines: `tokio::spawn` at
`:448`/`:479`, `results.pending` insert at `:496`, `registry.tasks.insert` at
`:505`. Sol violated invariant 1 in its first fresh build from this same plan,
and that build passed 7/7 gates and went forward. The comparison holds.

| id | invariant | arm's mechanism | class |
|---|---|---|---|
| **F4** | 1 | One critical section at `executor.rs:905-955`: gate check, `tasks.insert` at `:922`, slot construction, `spawn` at `:950` **behind a closed `oneshot` start gate**, handle stored at `:953-955`, `drop(guard)`, and only then `start_tx.send(())` at `:962`. The task's first await *is* the gate. | **CLEAN** |
| **F2** | 4 | Lateness is computed from a clock sample taken **after** the await (`:1164-1165`, `:1375-1384`), not from which `select!` branch won. `is_timely_success` (`:255-265`) additionally requires `Live` authority and `OnTime` before any success is claimed. | **CLEAN** |
| **F1** | 6 | A single `Mutex<OwnedState>` guards gate, task registry and child registry. Every acquisition goes through `fn lock` at `:474`, which is `unwrap_or_else(|poison| poison.into_inner())`, so a panic under the lock cannot poison-propagate. `shutdown` takes records **out** of the registry (`handle: record.handle.take()`, `:1073`), so a second drain finds an empty set rather than unwrapping a missing `get_mut`. | **CLEAN** |
| **F3** | 5, 6 | `tokio::time::timeout(policy.join_window, &mut join)` at `:1093` awaits a **mutable borrow**, so a timeout does not consume the handle; `join.abort()` at `:1098` is still reachable and `(&mut join).await` at `:1101` observes the abort so the drop guard delivers. Separately, `run_session` awaits `next_frame` directly and never as a `select!` arm. | **CLEAN** |

### 4. The test suite is the sharpest difference

44 tests against Sol attempt 1's 19 (5 + 9 + 5). And **3 of the arm's tests use
`#[tokio::test(flavor = "multi_thread")]` where Sol used none.** Two generation-3
review arms independently observed that every executor test ran single-threaded
while the manifest pins `rt-multi-thread`, and named that as the reason the suite
could not detect F4. The builder arm closed that gap unprompted. It also used
Tokio's paused virtual clock in 3 tests and, per its receipt, no `thread::sleep`
and no real `sleep` as ordering evidence - which invariant 7 demands.

### 5. Receipt honesty: strong

Section 2 gives a file-and-line range plus a stated mechanism for all seven
invariants. Section 5 declares what it could not establish, including: a
`CloseOutcome::Attempted` variant nothing could reach, so it made
`close_requested` a field instead; that latching `unsafe_at_close` on a
*deliberate* child replacement may be over-eager and "a reviewer could reasonably
call this" wrong; that `process_supervision.rs` is `#![cfg(unix)]` so 9 tests
compile away on other hosts and the subprocess evidence vanishes with them. On
invariant 5 it wrote that the doc comment "states this requirement explicitly
rather than claiming cancel-safety it does not have."

### 6. My pre-registration was wrong, and that is the point of pre-registering

I named "gates pass, invariant 1 VIOLATED" as the most likely outcome and the one
I expected to convert F4 into a contract-representation defect. **It did not
happen.** Given the identical plan, the identical baseline and no sight of any
review, the arm honoured all four invariants Sol violated. F4 is therefore
*not* a contract-representation defect: the plan's one-sentence statement of
invariant 1 was sufficient for this model to implement it correctly. Sol's
violation was a builder failure against an adequate contract.

### 7. F3: the construction/detection divergence

**No Claude reviewer recovered F3 in eleven runs** - four models, four effort
levels, a concern statement naming cancellation and handle ownership and drop.
The Claude builder avoided F3 by construction and documented the mechanism,
`&mut join` so the timeout cannot consume the handle.

Constructing a correct mechanism and recognising its absence in someone else's
code are not the same capability, and this is direct evidence that they come
apart in the same model family. It also reframes the F3 blind spot: it is not
that the family lacks the concept, which it demonstrably writes and explains;
it is that reading for an *absent* guarantee is a different task from providing
one. That strengthens, not weakens, the case for an F3-shaped proof obligation -
the obligation would be asking the reviewer to do what the builder did
spontaneously.

### 8. Corrections to my own statements this session

- I reported mid-run that the stream showed **zero thinking blocks** and
  suggested that undercut thinking volume as a proxy. **Wrong**: the result
  reports **54,972 thinking tokens**, more than any generation-3 review arm. My
  extractor counted a field the stream does not populate per-block. The A2
  conclusion about thinking volume stands on its own evidence and needed no help
  from this.
- I projected 35-40 window points for a builder arm from the first 14 minutes.
  Actual: **15**. The rate flattened once writing gave way to compiling.

### 9. What this does not establish

My defect-incidence scoring is a **source read**, and source reads of exactly
this kind have been wrong five times today, including one in the note this very
pilot was designed to test. Four CLEAN classifications by the same method that
produced those errors deserve exactly as much trust as that method has earned.
The confirmation that matters is an adversarial review arm against **this** tree,
blind, scored the way Test R scored Astra's - and run by `haiku-5-5`, the model
that matched F4 where both Opus arms failed. Until that runs, the honest
statement is that a careful reader found no instance of F1-F4 in the arm's
implementation, not that none exists.

Also: 2.4x the code is 2.4x the surface for defects Astra never had occasion to
name; 44 passing tests are the arm's **own** tests; n=1; and the arm had no
repository-evidence provider while Sol recorded 10 graph calls, a delta in Sol's
favour that makes this outcome harder to explain away, not easier.

This authorizes no Claude builder in any campaign. `resolve_provider.py:171-177`
still raises unless both models are Codex, and that gate should not move on one
run.

## Correction, sealed before the blind review returned: F3 is DIVERGENT, not CLEAN

Raised by ChatGPT on reading the result above, investigated by me, and committed
**while the `haiku-5-5` blind review of this tree was still running** - so whether
an independent reviewer finds it is a live prediction, not a check I already knew
the answer to.

The concern: `timeout(join_window, &mut join)` protects the handle from being
consumed by the **inner** timeout, which is the mechanism Sol got wrong. It says
nothing about destruction of the **enclosing** `shutdown` future. Verified in the
arm's source:

- `shutdown` (`executor.rs:1060-1075`) takes the handles **out of durable state**
  inside its first critical section: `guard.tasks.iter_mut().map(|(attempt,
  record)| ClosedTask { ..., handle: record.handle.take() })`. The rows stay in
  `guard.tasks`; only the handles leave, into a local `Vec<ClosedTask>` owned by
  the `shutdown` future.
- `for closed in records.drain(..)` then moves each handle into a local.
- The only `impl Drop` in the file is for `ResultSlot` (`:537`). **There is no
  drop guard that returns handles to the registry.**

So if `shutdown` is cancelled while awaiting a join, every remaining
`JoinHandle` drops with the future, and in Tokio dropping a `JoinHandle`
**detaches** the task.

**The consequence is worse than "reports joined", and it is self-documenting.**
A later `shutdown` finds `handle: None` and takes the `None => TaskEnd::NeverCleared`
branch at `:1090`. `TaskEnd::NeverCleared`'s own doc comment (`:605`) reads
**"Never cleared the start gate, so it never dispatched."** After a cancelled
shutdown that is false of exactly the tasks it will be applied to: they
dispatched, they are detached, and they may still be running. And because only
`Panicked | AbortedAtTimeout` latch (`:1105`), `ShutdownReport.unsafe_at_close`
can come back **`false`** on a close that abandoned live work.

**Reclassified under the pre-registered scheme as DIVERGENT** - a different defect
against the same invariant (5 and 6) - at **medium** severity, matching Astra's
grade for F3. Source-supported inference, not a reproduction.

Two adjustments to the concern as it reached me, in fairness to the record:
Astra's F3 consequence was "a later drain reports joined when no join occurred";
here the later drain claims the task *never dispatched*, which is a different and
arguably stronger false claim. And the task's **result is not lost**: `ResultSlot`
holds a reserved `OwnedPermit` and sends from its `Drop`, so the detached task
still delivers. The defect is join and close-evidence integrity, not result loss.

### What this does to the result above

Revised incidence: **F1 CLEAN, F2 CLEAN, F4 CLEAN, F3 DIVERGENT.** Three of four
invariants Sol violated are honoured; the fourth is honoured against Sol's
mechanism and violated against another one on the same invariant.

The finding in section 7 is weakened in precisely the way that matters. The
builder did **not** avoid F3's defect class - it avoided F3's *instance* while
reintroducing the class at a different cancellation boundary. That is a sharper
and narrower version of the same point: the family can write a correct
cancellation mechanism, explain it in a doc comment, and still miss a second
cancellation boundary in the same function. Section 7's claim that construction
and detection "come apart" stands; its claim that F3 was avoided "by
construction" does not.

### What this does to my own method

Wrong claim: F3 CLEAN. Error class: establishing a weaker property than the one
required - I verified the mechanism the ground truth named and stopped, so
"the inner timeout does not consume the handle" was recorded as "the handle
survives cancellation". Consequence: a published CLEAN verdict on a tree that
does not hold the property, plus a board post and a section 7 claim built on it.
Missing check: the property/state/accessor triple applied to my own positive
verdict - the handle's accessor set is {registry row, `ClosedTask` local,
`closed` loop local}, and two of the three are destroyed by outer cancellation.

This is the same failure B diagnosed in the arm it scored: the property
established is not automatically the property required. It argues for requiring
the triple on *positive* findings, where it has never been required - the
accessor-grep error, the F3-decomposition claim and this one are all positive
claims about properties that were never stated precisely.

## Blind review of the Claude-built tree: `haiku-5-5` at `xhigh`

Subject `74cd89b42ae0b301f18c5adc899711300bfdbeb3`. Prompt and contract are
**byte-identical to Test R's** (prompt sha256 `27b666a4...`, contract sha256
`98390bd9...`), so the only changed variable against the generation-2 and -3
review arms is the subject. Sol's trees are absent from the review snapshot;
the baseline is present for diffing.

25 calls (15 Read, 10 Bash), **430 s**, **$0.5105**, 0 denials, clean tree,
five-hour window 0.18 -> 0.19 - **one point**. Twelve findings, verdict **BLOCK**.

### It did not find the defect I had sealed, and it did not clear it either

None of F1-F12 is the `shutdown` cancel-safety defect. Critically, its "What
checks out from source" section lists eight things it affirmatively cleared and
**the join/drain path is not among them**. So this scores **MISS**, not
FALSE-CLEARANCE - the distinction generation 2 established as the one that
matters. The reviewer left the area unexamined rather than pronouncing on it.

Thirteen Claude review runs have now passed over an F3-class cancellation defect
without one of them naming it, across two different implementations of the same
contract.

### It independently corroborated the verdict I was most confident in

> "Registration in one critical section with the start gate opened last
> (`executor.rs:902-962`)."

Same line range I cited, reached blind. F4 CLEAN now has two independent reads.

### It found the same *class* I found, at a different instance

**F6, medium**: a required result can be discarded when the sink is dropped
in flight while `emitted` still increments, `unaccounted_slots` reads 0, and
"this path does not latch unsafe". That is structurally the same argument I made
about `NeverCleared` - a drop boundary where evidence is lost and the counters
still report success - at a different boundary. And it declared its own basis
precisely: "It depends on tokio's `OwnedPermit::send` being infallible on a
closed channel. I did not verify that against the dependency source, which is
outside the snapshot." That is the declared-limitation behaviour generation 3
valued, produced unprompted by the cheapest model.

### It found a scope violation I missed, and the history reassigns the blame

**F9**: the owned manifest adds `serde_json`, `tempfile = "=3.27.0"` and
`work-engine-types` as dev-dependencies, where the plan says the owned manifest
"adds only `tokio.workspace = true`" (plan line 63) and "request any addition
first". It correctly noted that all 13 paths are inside my declared boundary and
that the conflict is with the **plan's narrower lane**.

I scored BOUNDARY: PASS by checking paths and never checking manifest contents.
That is a scope check I defined too loosely.

But the history complicates the blame. Sol's attempt 1 added the **identical
three dev-dependencies**, including `tempfile = "=3.27.0"` as a version literal
rather than a workspace entry, and the identical root Tokio pin down to the
version and feature set. So F9 is a real plan deviation that **both builders made
independently and that the campaign's own 7/7 gates and Astra both let through**.
It is a finding against the process, not against either builder. The root-manifest
half of F9 is mine - my prompt authorized the pin because no separate integration
owner exists in this run.

### Revised ledger for the Claude-built tree

| source | finding |
|---|---|
| my read | F3-class: `shutdown` not cancel-safe; `NeverCleared` misreports detached tasks; no unsafe latch |
| `haiku-5-5` | 12 findings incl. port-error-as-no-send proof (high), unenforced wait budget, no transport ever closed, locally invented 30 s timeliness window, sink-drop evidence loss, `register_transport` bypassing the gate, manifest scope |
| both | the drop-boundary class, at two different boundaries |
| neither | unknown - and the symmetry of the misses is the point |

**Seven gates passing told us almost nothing.** A tree with 44 passing tests,
clean `clippy -D warnings`, and a release build drew twelve findings and a BLOCK
from a $0.51 reviewer. The gates and the review measure different things, which
is what the campaign's own architecture already assumes.

### Economics of the full Claude-only loop, measured

Build $12.4955 + review $0.5105 = **$13.01**, and **16 points** of one five-hour
window, for one slice built and independently reviewed end to end. The reviewer
costs **4%** of the builder. That ratio, not the absolute number, is the
operationally interesting figure: adversarial review is nearly free relative to
construction, which argues for more reviewers rather than cheaper ones.

## S-sonnet pre-registration, sealed before launch

Byte-identical prompt (`ba82e404...`) and contract (`98390bd9...`), own pristine
snapshot at the same baseline, `--effort high`, `--max-budget-usd 25`. The only
variable against S-opus is the model.

Two questions, in order of what I now care about:

1. **Is the F3-class cancellation boundary a family-wide construction blind
   spot?** S-opus wrote a correct inner-timeout mechanism, documented it, and
   left the enclosing `shutdown` future unsafe to cancellation. If S-sonnet
   reproduces that shape - handles taken out of durable state into locals owned
   by an async fn, with no drop guard - then construction of cancellation safety
   has a *systematic* gap in this family, which is a much stronger claim than one
   run supports and the sharpest finding available from this arm. If S-sonnet
   either keeps handles in durable state or has no analogous structure, the
   S-opus instance is local.
2. **Cost and quality at a fifth of the price.** S-opus: $12.4955, 15 window
   points, 5,407 insertions, 44 tests, 7/7 gates.

Pre-registered outcomes:

- **Gates pass and invariant 1 CLEAN** - builder substitution is supported at the
  *cheap* tier, and the economic case stops depending on Opus.
- **Gates fail** - the ceiling S-opus established does not survive the price cut,
  and the useful figure becomes the gap between tiers rather than either tier.
- **F3-class defect recurs** - see 1; this is the outcome I would most want to
  know and expect at better than even odds, because the same reasoning that made
  S-opus's inner fix attractive is what hides the outer boundary.

I am the scorer again, by the same source-read method that produced the wrong F3
CLEAN verdict on S-opus. Every positive verdict in this arm will carry the
property/state/accessor triple explicitly, since its absence is what allowed a
weaker property to be recorded as the required one.

## S-sonnet result: cheaper, equal on gates, and the F3-class gap recurs

`claude-sonnet-5-5` at `--effort high`: 69 calls (54 Bash, 7 Read, 6 Write, 1
Edit, **1 Agent**), **934 s** (15.6 min), **$4.7099**, 19 turns, 0 denials,
`subtype: success`. Five-hour window 0.20 -> 0.28, **8 points**. 4,864 insertions
over **exactly the 13 declared paths**.

### Gates: 7/7 PASS, 44 tests

`fmt` 0, `clippy --all-targets -D warnings` 0, release build 0, lane tests 0
(1 unit + 10 `clock_framing` + 18 `owned_execution` + 15 `process_supervision` =
**44**), `--workspace` 0, `git diff --check` 0.

### Head-to-head against S-opus

| | S-opus | S-sonnet |
|---|---|---|
| cost | $12.4955 | **$4.7099** (2.65x cheaper) |
| wall | 32.6 min | **15.6 min** (2.1x faster) |
| window points | 15 | **8** |
| insertions | 5,407 | 4,864 (90%) |
| tests | 44 | 44 |
| gates | 7/7 | 7/7 |
| boundary | exact 13 | exact 13 |

The pre-registered outcome "gates pass and invariant 1 CLEAN" obtained at the
cheap tier. **Builder substitution no longer depends on Opus**, and the economic
case improves by 2.65x without a measured quality cost on this subject. It is
*not* the 5x I assumed from review-arm pricing; builder work compresses the tier
gap.

### Defect incidence: F1 CLEAN, F2 CLEAN, F4 CLEAN, F3 DIVERGENT

**F4 - invariant 1 - CLEAN.** `oneshot` start gate, same construction as S-opus
reached independently. `executor.rs:654` creates `(start_tx, start_rx)`; the
comment at `:655` reads "Register first, under the gate. The task cannot start
before `start_tx` fires"; `registry.tasks.insert` at `:666`; spawn at `:698` with
`start: start_rx`; `start_tx.send(())` at `:718`. Two models, two families of
implementation, same answer to the invariant Sol violated.

**F2 - invariant 4 - CLEAN, with a better warrant than S-opus's.** At
`:1041-1050` it demotes a timely result to late *after* child reaping, with the
reason stated in the code: "Reaping took time. Success must still hold at the
moment it is claimed: evidence that was timely when the provider returned is
demoted to late if the absolute deadline passed while the child was being
closed." It names the specific await that can outlast the authority, which
S-opus's equivalent did not.

**F1 - invariant 6 - CLEAN.** `lock_registry` at `:391` is
`unwrap_or_else(PoisonError::into_inner)`, as is every other acquisition, so a
panic under the lock cannot poison-propagate. `begin_shutdown` is
**synchronous** and idempotent - `if registry.snapshot.is_none()` at `:511`
latches the owned set once - so two concurrent closers cannot interleave the
snapshot step at all.

**F3 - invariants 5 and 6 - DIVERGENT. The pre-registered recurrence happened.**

Scored with the property/state/accessor triple this time, which is the discipline
whose absence produced my F3 CLEAN error on S-opus:

- **Required property**: no owned join handle is lost to cancellation at any
  boundary, and the close report truthfully describes what was joined.
- **States**: handle in the registry record / handle in `shared.joins` / handle
  in a `shutdown`-local vector / handle consumed by `await`.
- **Accessor set**: `TaskRecord.join` (`:356`), `Shared.joins:
  Mutex<Vec<JoinHandle<()>>>` (`:381`), the local `handles` after
  `std::mem::take` (`:552`), the loop-local `handle` (`:554`).
- **Which survive cancellation of `shutdown` after `:552`**: **none.**
  `shared.joins` has just been emptied by `mem::take`; the local vector and the
  loop-local are destroyed with the future; dropping a Tokio `JoinHandle`
  detaches.

Sonnet's design is **better than Opus's in one respect and worse in another.**
Better: `begin_shutdown` is synchronous, so gate-close and handle-collection
cannot be interrupted part-way, and the handles land in *durable shared state*
rather than a future-owned local - exactly the structure that would fix this.
Worse: `shutdown` then takes them straight back out into a local and does
`for handle in handles { handle.await ... }`, awaiting **by value** with no
per-handle join window and no abort at that site, where Opus at least held
`&mut join` with a `timeout` and an abort.

So Sonnet built the right home for the handles and then moved them out of it.

**Consequence.** A `shutdown` cancelled after `:552` detaches every not-yet-awaited
task. A later `shutdown` takes an empty vector, reports `task_join_panics = 0`,
and reports `owned_at_close` from the retained snapshot - so it names the right
owned set while having joined none of it. Unlike Opus's `NeverCleared`, no
variant asserts something false; the report is silent rather than wrong. Partial
mitigation exists: a detached task's queued unresolved result can still latch
`unsafe_latched` through the `results_undelivered` sweep, which Opus's path did
not do.

### What the recurrence establishes, and what it does not

**Two models, independently, from the same 83-line plan, both got invariant 1
right by the same mechanism and both left the shutdown join loop unsafe to outer
cancellation.** That is the pre-registered sharp finding: cancellation safety has
a **systematic construction gap** in this family at the *enclosing-future*
boundary, while the *inner-await* boundary is handled correctly and even
commented. n=2, one subject, so it is a pattern and not a law - but it is no
longer a property of one run.

It also sharpens what the thirteen review runs were failing at. The family can
construct the inner mechanism, explain it, and miss the outer boundary; and when
reviewing, it does not look at the outer boundary either. **The same blind spot
appears on both sides of the role**, which is a different and more useful claim
than "construction and detection come apart". They come apart on F3's
*instance* - Sol's exact defect - and they coincide on F3's *class*.

That is now the strongest available case for the F3-shaped proof obligation, and
it predicts what the obligation must say: not "check the join handle" but
"enumerate every state in which the handle resides and name which survive
cancellation of each enclosing future". Which is the triple.

### One stated delta

S-sonnet made **one `Agent` call**; S-opus made none, on byte-identical input.
Its inference is consumed inside the $4.7099 and I cannot inspect the
sub-context from the stream. This is harness behaviour, not an admitted
execution vantage, and the two arms were therefore not procedurally identical.

## Amendment on arm D's limits, recorded before its result returns

Arm D (the F3-shaped obligations, `claude-opus-5` at `xhigh`, against the
original sealed key `7fdf392f`) carries a limitation stronger than its non-leak
check, and the check should not be read as covering it.

The non-leak check is **word-level**: the prompt contains zero occurrences of
`JoinHandle`, `detach`, `drop`, `drain waiter`, `collector` or `abort`, and uses
only plan vocabulary. That bounds *textual* disclosure of the mechanism.

It does **not** isolate designer knowledge. I have read Astra's F3 finding and
both builder arms' cancellation structures, and I chose which invariants to
decompose, at what granularity, and in what order. OB1's instruction to
"enumerate every program state in which the owning handle to that resource
resides" is derived from invariants 5 and 6 - but the reason I derived *that*
decomposition rather than another is that I know where two implementations
failed.

So a positive result from arm D would establish that **this decomposition elicits
this defect**, supporting the mechanism. It would not establish that an
independent compiler could derive these obligations from the plan alone, which is
the claim the economic case actually needs. That requires a separate arm: a model
given only the plan and asked to emit proof obligations, with no sight of any
candidate, any review or any defect - then those obligations run blind. Until
that exists, every obligation result in this pilot is a mechanism demonstration
with a human compiler in the loop.
