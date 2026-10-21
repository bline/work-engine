# Pre-registration: worker-owned handoff boundary pilot

Status: pre-registration, 2026-10-21T20:08Z, written before any post-amendment
thread has produced a boundary decision. Authored in the claude-metrics
measurement role: design and scoring only, excluded from both arms.

The pilot is defined by the [orchestration brief](../../docs/codex-orchestration-brief.md)
sections "Context check-in block for task briefs" and "Worker-owned handoff
pilot", and by the [CLI guide](../../docs/context-status-cli.md). This document
adds no instruction to either; it fixes the cohorts, measures and decision rules
in advance so that the later comparison is a comparison and not a
reconstruction. Frozen per-thread counters are in
[the baseline](handoff-boundary-pilot-baseline-20261021.json).

## What the pilot measures, in its own words

The brief states the identification limit itself: the pilot measures
"telemetry-assisted boundary judgment, not a calibrated threshold or the CLI's
isolated causal effect", and "cannot separate the effect of telemetry from
asking an agent to reconsider continuation". This pre-registration adopts that
scope. No arm here estimates the CLI's marginal contribution.

## Cohorts, fixed by thread start time

Membership is decided by `session_meta.timestamp` and is not revisable after
results appear.

| cohort | n | window | instruction state |
| --- | ---: | --- | --- |
| `pre_change` | 35 | start < 2026-10-21T19:47:27Z | no check-in block |
| `transition` | 3 | 19:47:27Z - 20:02:48Z | block delivered, brief amended mid-flight at 20:05:01Z |
| `post_change` | 0 | start >= 2026-10-21T20:05:01Z | amended block from launch |

The `transition` threads (`b1b_identity_adviser`, `bounded_inference_b1b`,
`bounded_inference_b1b/b1b_worker`) all launched before the amendment that added
the epoch field, the pinned-binary binding and the economics labelling. They are
reported separately and excluded from the primary comparison. `b1b_worker`
nevertheless records the amended form after a parent message, so it is evidence
the delivery path works, not evidence about a cohort.

Thread membership derives from `work-engine-context-status agents --recursive`
for roots `01a15b93-56c2-74f1-8c97-e471f81bc6e2` and
`01a15f02-a935-7683-a849-9e919aecfe3c`. That tool reports
`partial_or_ambiguous` over 720 scanned rollouts, so the set is a candidate
enumeration, not a proven-complete tree. Any count below is "within the
enumerated 38", not "in the campaign".

## Counters are live, so the baseline is a snapshot

Several `pre_change` threads were still running at registration.
`session_quota_repair_worker` moved from 914 calls and 5 compaction markers to
1147 and 7 inside roughly 45 minutes of observation. Cohort membership is fixed;
per-thread counters are not. Scoring re-reads every rollout at a declared end
time and reports both the 20:08Z snapshot and the final values, rather than
silently using whichever was read first.

## Observable substrate

Four legs of the contract are readable, which is what makes the pilot scoreable:

1. **Invocation** - the CLI call appears as plaintext in the rollout's exec records.
2. **Observation** - retained JSON, e.g. `worker-context-checkin-N.json`,
   `root-context-boundary-vN.json`.
3. **Decision** - `continue` or `prepare handoff` with its reason, written into
   the result or continuation.
4. **Transfer** - `handoff_ready` with path and SHA-256; the child-to-parent
   return is readable; the successor appears as a `session_meta` carrying
   `parent_thread_id`.

Not observable: parent-to-child dispatch bodies. All 317 enumerated
parent-to-child `agent_message` payloads are `encrypted_content` with an empty
plaintext body, against 58 of 323 child-to-parent returns carrying readable
text. **No scoring rule may depend on dispatch text.** This is why leg 3 - the
decision in a durable artifact - is load-bearing: without it the pilot would be
unscoreable from logs.

## Measures, per thread

Calls; peak and final `last_token_usage.input_tokens`; compaction markers
counted from `lineage`; epoch, with its causes separated; CLI invocations;
check-in records written; decisions recorded; `handoff_ready` declared;
successor spawned and verified; context window.

**Epoch is not a compaction count.** It also increments on model change, context
window change or availability change, and any input-token decrease. The root's
own `root-context-boundary-v2.json` shows epoch 6 from four compaction markers
plus two model changes. Scoring counts markers from `lineage`, never epoch.

## Primary question and decision rules

Does a recorded boundary check-in ever produce `prepare handoff` followed by a
verified successor, and do threads stop reaching compaction?

- **A.** At least one check-in yields `prepare handoff`, a `handoff_ready`
  artifact, and a successor whose session and rollout the parent verified. The
  mechanism is demonstrated end to end at the stated n. No efficiency claim
  follows.
- **B.** Check-ins are recorded, all decide `continue`, and no thread reaches a
  compaction marker. Consistent with judgment working, but indistinguishable
  from the assignments being small enough not to need a handoff. Recorded as
  `not established`, not as success.
- **C.** A thread records `continue` at a check-in and later compacts. This is
  the failure the pilot targets. Inspect the recorded reason against the
  observed trajectory; this is the only outcome that can show the judgment
  itself, rather than its availability, to be miscalibrated.
- **D.** Check-ins absent, or `usage unavailable`. Classify by the brief's three
  classes - launch/tool failure, identity rejection, absent counters - and treat
  as a delivery or instrument result, not a judgment result.

Outcomes are not exclusive across threads; each thread is classified separately
and n is stated per class.

## Confounds, stated before results

- **No matched control.** Avoided context re-send is unobserved. The brief's
  "compare the extra handoff/reconstruction cost with avoided context re-send"
  is therefore a bound, not a measurement: the handoff-side cost is observable
  and the avoided-cost side is not.
- **The outcome is zero-inflated and single-thread dominated.** 20 compaction
  markers fall in 9 of 35 `pre_change` threads, 7 of them in one thread. Means
  across threads are not usable; report per thread.
- **Role and model heterogeneity.** `pre_change` is `gpt-6-sol` 20,
  `gpt-6-astra` 13, `gpt-6.1-sol` 2. The brief's model-selection paragraph
  records existing practice rather than co-intervening: builder-type threads
  were already `gpt-6-sol` in 10 of 12 cases, and the two `gpt-6.1-sol` threads
  are the two aborted dispatches, at 3 and 4 calls.
- **Adoption measures exclude the tool's own tree.** 137 of 161 `pre_change` CLI
  invocations are in `context_cli_supervisor` and its children, which built and
  reviewed the CLI.
- **Task mix does not repeat across cohorts**, so any before/after on calls or
  cost is descriptive.
- Two context-window values appear in the enumerated set, 258,400 and 475,000,
  so a fraction-of-window measure must use each thread's own value.

## Reproduction

```
BIN=rust/target/debug/work-engine-context-status
$BIN agents --parent-thread <root> --recursive --sessions-dir ~/.codex/sessions
$BIN --session <thread> --sessions-dir ~/.codex/sessions --json   # lineage, epoch
```

Compaction markers come from the `lineage` array. Per-call context is
`event_msg.token_count.info.last_token_usage.input_tokens`;
`info.total_token_usage` is account-lifetime cumulative and is not a thread
measure. Message direction and payload readability come from `response_item`
records of type `agent_message`, comparing `author` with `recipient`.

## Amendment 1, 2026-10-21T20:18Z

The brief was revised a second time at 20:15:07Z, after this registration. The
revision moved three requirements from the brief's prose into the quoted block:
the pilot's scope caveat, the reason not to build in the shared workspace, and
`Record economics as not estimated unless an explicit scenario supplies its
assumptions`. Because the block is the only part copied into a worker's brief,
a faithfully delivered pre-revision block could omit the economics labelling and
the scope caveat. That is a change in what a worker receives, so the cohort
boundary has to move with it.

**What was visible when this amendment was written**, so its independence from
results is auditable: three `continue` decisions from `b1b_worker` (transition
cohort) at 39,249 / 94,199 / 100,613 input tokens, all epoch 0; and two root
`continue` decisions - `01a15b93` at 117,899 of 258,400, epoch 6, and
`01a15f02` at 201,121 of 258,400, epoch 2. **No `post_change` thread has
recorded a decision**; `b1b_reviewer` has self-queried its own session and
written nothing yet.

### Cohort change

`post_change` splits, by thread start time:

| cohort | n at amendment | window |
| --- | ---: | --- |
| `post_change_r1` | 1 (`b1b_reviewer`, 20:11:17Z) | 20:05:01Z <= start < 20:15:07Z |
| `post_change_r2` | 0 | start >= 20:15:07Z |

The frozen [baseline](handoff-boundary-pilot-baseline-20261021.json) keeps its
original 20:08Z labelling and is not rewritten; this section supersedes its
`cohort` field where they disagree.

### New stratum: roots

The two orchestrator roots are inside the pilot's scope - the block is for
"coordinator, supervisor, worker and reviewer briefs" - but they were absent
from the cohort table, which enumerated only `agents --recursive` descendants.
Both are in fact running boundary checks and writing decisions in the prescribed
schema (`root-context-boundary-v2.json`, `root-boundary-decision-v2.json`).
Report them as a separate stratum and never pool them with child threads.

### New decision class E: constrained continue

A recorded `continue` whose stated reason cites the absence of an eligible
successor or untransferable ownership, rather than next-task size, retained
context value or reconstruction cost.

The motivating instance is `01a15f02` at 20:07:39Z: 201,121 of 258,400 - 77.8% -
two compaction markers, deciding `continue` because "no verified successor root
owns this conversation" and to "avoid absorbing implementation context". A root
cannot execute the contract's transfer step at all: that step requires a parent
to verify the successor's session and publish the binding, and a root has no
parent. The same applies to any thread whose ownership cannot transfer safely,
which the brief already requires be deferred.

Class E is **not** evidence of calibrated judgment and does not count toward
outcome A or B. It records a structural ceiling in the role. Its presence at
77.8% is the clearest available demonstration that telemetry can expose a
boundary the agent is not permitted to act on, which is a finding about the
protocol rather than about the judgment.

Consequently the primary question is restricted to threads with an eligible
successor path - non-root threads with a live parent - and class E threads are
reported with their observed fraction and marker count, so a ceiling reached
without a permitted handoff is visible rather than scored as a continue.

## Amendment 2, 2026-10-21T20:31Z

The brief was revised a third time - worktree edit at 20:22:24Z, committed as
`84fe837c` at 20:28:44Z - and the revision supersedes amendment 1's class E with
a better instrument. No thread launched between amendment 1 and this one, so
`post_change_r2` remains empty and `post_change_r3` (start >= 20:22:24Z,
instruction revision `84fe837c`) opens at n=0. Cohorts from here are anchored to
the brief's commit where one exists; the first two revisions were uncommitted
when delivered and keep their mtime anchors. Delivery time is the worktree edit,
not the commit.

### Class E is now a recorded field, not a scorer's inference

Amendment 1 defined class E by reading the stated reason and judging whether it
cited successor absence rather than economics. The brief now requires
**transfer availability** - `available`, `unavailable`, or `unestablished` -
recorded *before* the continue/handoff decision, naming the actual mechanism and
owner, with `continue-transfer unavailable/unestablished` as a distinct decision
value. Records in use already carry `transfer_availability`, `transfer_owner`,
`assessment_class` and `primary_handoff_choice_eligible`.

Scoring therefore **reads the field** and falls back to reason prose only for
records written before 20:22:24Z, marking any such classification as inferred.
This removes the inference step that made amendment 1's class E the least
reliable part of this registration.

**`unavailable` and `unestablished` are not pooled.** Amendment 1 conflated
them; rule 2 applies, and the two root records already differ -
`root-context-boundary-v2-corrected.json` records `unestablished` ("no verified
root-successor transfer/collection mechanism in this session"), while
`root-boundary-decision-v4.json` records `unavailable`. Report the two values
separately and never sum them into a single "could not hand off" count.

The brief also separates the two failure modes outright: "Missing telemetry and
missing transfer are separate." Outcome D (telemetry) and class E (transfer) are
therefore **orthogonal axes**, not alternative classes of one taxonomy. A thread
can be both, and the scoring table is decision x telemetry availability x
transfer availability, not a single column.

### Provenance rule for corrected records

Earlier root records were reclassified under the new schema while the originals
were preserved: the corrected files carry `historical_decision`, `correction_at`
and an explicit note to "preserve original records". Scoring uses the corrected
record where one exists, reports that a correction occurred with its
`correction_at`, and never presents a corrected record as an original
observation. The underlying datum is unchanged - `01a15f02`'s 201,121 of 258,400
at epoch 2 survives in `root-boundary-decision-v4.json` as `prior_pressure`.

### Roots cannot produce handoff evidence in this pilot

The brief now states the root requirement: "A root has no parent: transfer is
available only through a separately verified mechanism that preserves supervision
and collection ownership. Merely spawning a child, writing a packet or undergoing
compaction does not transfer the root."

No such mechanism exists at registration. Both roots are therefore class E by
construction for the pilot's duration unless one is defined and verified, which
means **the pilot cannot yield root-level handoff evidence at all**. This is a
scope limit, registered in advance: a root reaching high usage and continuing is
a measurement of the missing mechanism, not of the judgment the pilot studies.
Root boundary records remain worth collecting for observed pressure, epoch and
lineage.
