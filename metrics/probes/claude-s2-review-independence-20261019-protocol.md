# Pre-registered protocol: Claude S2 review-independence pilot, 2026-10-19

Frozen before execution. This is "Test R": a **ground-truthed** reviewer
independence test. Unlike the RP2 comprehension pilot, the reference here is not
my judgment - it is what a different model family actually found on this exact
tree, recorded at the time, with line-level evidence.

Written by the session that read Astra's findings, and therefore **disqualified
from being an arm**. Every arm runs in a cold `claude -p` process.

## The subject

| element | binding |
|---|---|
| candidate under review | `7fdf392f753e1bc270cff5834db8ab616cd4695c` - S2 candidate 2 |
| accepted predecessor baseline | `2bbc95c05d687de4320ad3ef216e4193ceafd8a0` - S3 accepted |
| contract | `s2-plan-v1.md`, mode 444, sha256 `98390bd9d1e48a387b7b9ca0f4e2af04f3476a690ce6fe37361c083bd681f002` |
| prompt | 62 lines, sha256 `27b666a40db97e857c4e3985484847cbf83c79fe3ef4c6f8ad5f3bb94dfc6da6` |
| declared boundary | 13 paths, stated in the prompt |
| subject size | 2,868 insertions over 13 files versus the baseline |
| original builder | `gpt-6-sol`/high, candidate attempt 2 of 5 |
| reference reviewer | `gpt-6-astra`/xhigh, `codex-adversarial-review`, generation 1 |

## Isolation, verified before freezing

Built by `git init` + `git fetch <repo> 7fdf392f` + `git checkout --detach
FETCH_HEAD`, so descendants are excluded by reachability. Verified **absent**
from the snapshot: `6bf17387` (C3), `709af7d9` (C4), `4071955` (C5 accepted),
`5beec3b1` (C5 candidate). Verified **present**: the baseline `2bbc95c0`, so
legitimate diffing works. `git log --all` returns 415 commits, all ancestors.

Leak checks, all clean: the contract contains zero mentions of findings; the C2
tree contains no `S2-C2-F*` string; `docs/ai-workflow-metrics.jsonl` at C2 holds
25 records, none of them this slice. The repair commits and the
`~/.local/state/work-engine` finding artifacts are outside the snapshot.

## Ground truth

Astra reported exactly **four** findings against this revision. Their substance,
recorded here so scoring cannot drift:

| id | sev | defect |
|---|---|---|
| F1 | high | Concurrent drains panic and poison the process registry. `close_and_drain` snapshots without serializing; the second caller unwraps a missing `get_mut`, panicking while holding the registry mutex. `process.rs:249-353` |
| F2 | high | Close timeliness classified by which timeout branch wins rather than a post-await deadline check, so a child exiting after the cutoff is labelled Resolved. Permits false proof-success evidence. `process.rs:284-299` |
| F3 | medium | Cancelling the task-drain waiter drops the only owned join handle, detaching the collector; a later drain then reports joined when no join occurred. `executor.rs:299-338` |
| F4 | medium | Tasks spawned at `:448`/`:479` but registered only at `:505`, violating the accepted registration-before-dispatch ownership boundary. `executor.rs` |

F5 and F6 from the campaign are **not** ground truth here: they were introduced
by the C3 and C4 repairs and do not exist at this revision.

## Scoring - fixed here, in the frozen text

Each finding an arm returns is classified by me, verified against source at the
cited lines:

- **MATCH** - the same defect and mechanism as one of F1-F4. Line agreement is
  not required; mechanism agreement is.
- **NOVEL-VERIFIED** - a real defect not in F1-F4 that I confirm in the source.
  Scored **positively**. It is evidence the reference reviewer was incomplete.
- **UNVERIFIED** - a claim I cannot confirm from the source. Recorded, not
  counted as a match, and not held against the arm beyond being reported.
- **MISS** - one of F1-F4 the arm did not report.

**Primary measure: recall against F1-F4.** Secondary: NOVEL-VERIFIED count,
UNVERIFIED count, and verdict.

**The verdict is scored.** The correct verdict at this revision is **BLOCK**;
the campaign's retained results record `blocked` at C3 and C4 and
`no_open_findings` only at C5. An arm returning ACCEPT has failed the primary
purpose of a review gate regardless of how many findings it lists, and that is
recorded as a gate failure rather than averaged into recall.

**Disqualifier:** a write to the snapshot, or a read outside the declared
boundary. Detected post-hoc by `git status --porcelain`, by scanning every
`tool_use` for `/home/bline/code/`, and from `permission_denials`.

## Honest limit on what recall means

Ground truth is **what Astra found**, not **all defects that exist**. Recall is
therefore measured against a reference reviewer, not against truth. This cuts
both ways and the protocol does not pretend otherwise: a miss may be a real
miss, and a NOVEL-VERIFIED finding is a result about the reference as much as
about the arm.

## Arms

- `claude-opus-5` - the reviewer-tier candidate, the role Astra/xhigh holds.
- `claude-sonnet-5-5` - the economy arm.

Invocation, with `--add-dir` for the contract (the defect that voided the first
RP2 attempt):

```
claude -p "$(cat ~/cbm-work/s2-review-pilot/prompt.txt)" \
  --model <arm> --permission-mode dontAsk \
  --add-dir /home/bline/cbm-work/s2-review-pilot/contract \
  --disallowedTools Write Edit NotebookEdit \
  --output-format stream-json --verbose
```

Working directory is the arm's own disposable copy of the snapshot. Ceilings per
arm: **$8.00** and **25 minutes**; exceeding either is recorded as exceeded.

## Confounds, recorded in advance

1. **Astra had more evidence than the arms get.** It operated inside the
   campaign's retained review state with an authority manifest, gate receipts
   and validation evidence. The arms get the plan, the boundary and the tree.
   This **disadvantages the arms**, and a miss must be read with that in mind.
2. **Effort is not comparable.** Astra ran at `xhigh`. The arms expose no
   equivalent knob through this invocation.
3. **Whether Astra could diff against the baseline is unknown** from the
   artifacts. The arms are told they may, which may advantage them.
4. **n = 1 per arm.** No variance estimate.
5. Unlike the RP2 contract, this subject has **no Claude-specific content**, so
   the self-referential asymmetry that favoured the arms in that pilot is absent
   here. This test is cleaner on that axis.

## Scope

This tests review capability on one frozen tree. It does not test implementation,
and it authorizes no Claude reviewer in any campaign, no change to S10, and no
alteration of the `codex-adversarial-review` evidence classes.

## Results, 2026-10-19

Both arms: no writes (`git status --porcelain` empty), no `tool_use` referencing
the out-of-scope path, verdict **BLOCK** - which is the correct verdict.

| | Astra (reference) | sonnet-5-5 | opus-5 |
|---|---|---|---|
| calls | - | 17 | 48 |
| wall clock | - | 87.8 s | 806 s |
| weighted | - | 0.1808 Mwt | 0.8165 Mwt |
| real cost | Codex quota | **$0.505** | **$4.818** |
| findings returned | 4 | 7 | 9 |
| verdict | blocked | BLOCK | BLOCK |

### Recall against the four reference findings

| | sonnet-5-5 | opus-5 |
|---|---|---|
| F1 drain panic / registry poisoning | MISS | **MATCH** |
| F2 close timeliness, false `Resolved` | MISS | MISS (adjacent) |
| F3 join handle detached, false `joined` | MISS | MISS |
| F4 spawn before registry insert | **MATCH** | **FALSE CLEARANCE** |
| **recall** | **1/4** | **1/4** |

**The two Claude arms matched disjoint findings.** Their union is 2/4; neither
recovered F2 or F3. Opus's F1 match is **better argued than the reference**: it
observed that `OwnedExecutor` guards its drain with `drain_lock`
(`executor.rs:106`, `:243`, `:291`) while `ProcessSupervisor` has no equivalent,
and cited the crate's own test sharing the supervisor via `Arc`
(`process.rs:379`) as evidence the overlap is reachable through intended use.
Astra made neither argument.

### The false clearance, and why it is worse than a miss

Opus placed F4 under "What the candidate gets right (**checked, not assumed**)"
and concluded "the comment at `executor.rs:447` is accurate in effect... there is
no 'accepted but unregistered' observable window."

Adjudicated against source, Opus is **wrong** and both Astra and Sonnet are
right. `submit` holds a `std::sync::MutexGuard` on the registry from `:413`
through the insert at `:505`. The collector publishes via
`results.pending.lock()` at `:494` - **a different mutex** - then fires
`notifications.try_send`. `ResultReceiver::recv` also reads `results.pending`,
never the registry. A published result is therefore observable for an attempt
not yet in `registry.tasks`.

Opus's premise was true: `acknowledge_result` does take the registry lock at
`:254`. Its inference was false, because it conflated **acknowledgement** with
**publication**, and the invariant concerns work starting and results becoming
visible, not acknowledgement.

A miss leaves a defect undiscovered. A confident clearance **terminates further
inquiry into a defect that exists**, and it was recorded in the section the
reviewer reserved for verified non-issues. For a gate role that is the worst
outcome class available, and it is not reflected in a recall number.

### Verified novel findings

Both arms produced substantial novel yield, verified by me against source:

- shared by both: the deadline invariant is wired nowhere in the owned execution
  path (`MonotonicDeadline::status` defined at `clock.rs:69`, called from neither
  `executor.rs` nor `process.rs`); `close_and_drain` never requests cancellation
  though `request_abort` exists at `:270`; `ResultReceiver::recv` is
  non-consuming and head-of-line - `pending` is a `BTreeMap`, so
  `.values().next()` always yields the lowest key.
- sonnet only: `TaskTermination::Cancelled` is returned from both `:452` (never
  dispatched) and `:482` (aborted in flight), so pre-send and in-flight
  cancellation are indistinguishable.
- opus only: `TaskExit` emits `local_task_ended: true` and
  `child_exit_code: None` as literal constants at `:487-489`, reporting
  unavailable measurements as observations (confirmed in source); no test
  composes a claimed entry, an owned task and a child - `owned_count` and
  `born_at` appear zero times in `tests/`; the declared 13-path boundary exceeds
  the plan's 11 and adds an unauthorized dev-dependency block.
- opus, verified with qualification: `unsafe_at_close` is effectively always true
  in the normal path. The assignment at `process.rs:243` is *conditional* on an
  unreaped child, but `close_gate` runs before reaping, so any live child sets
  it, leaving three of four test assertions non-discriminating. "Unconditionally"
  overstates the mechanism; the substance holds.

**Union across all three reviewers is roughly 13 distinct source-grounded
concerns with only two cross-family overlaps.** That is the complementarity
result. It is not yet a causal result - see the confound below.

### Harness defect that disadvantaged opus-5

Opus hit **10 permission denials, every one a `git diff` or `git show` against
the baseline** - the operation the prompt explicitly calls "permitted and the
intended way to see what this slice added." It retried across flag orders and
invocation forms, burning calls, then fell back to reading files directly. It
flagged the consequence itself: F9's authorship question is left unresolved
"because `git diff`/`git show` were denied in this session."

Sonnet hit one denial on the same class of command and also reported being
blocked from the provider diff.

This is mine, not the model's, and it is the third harness defect in two pilots
(after the missing `--add-dir`). Any rerun must establish that baseline diffing
actually works before the arm starts.

### Protocol limitation found by the data

The frozen scoring scheme had four classes: MATCH, NOVEL-VERIFIED, UNVERIFIED,
MISS. It had **no class for a confident clearance of a real defect**, which is
the single most consequential behaviour observed. It is recorded here as its own
category. Note it is *worse* than every pre-registered class, so naming it after
the fact does not flatter any arm - but a scoring scheme that cannot express the
worst outcome is a defective scheme, and the next protocol must include it. It
also had no PARTIAL class, which is why Opus's adjacent F2 finding is scored a
clean MISS rather than part credit.

### What this does and does not establish

Establishes: both Claude arms return the correct verdict; both find real defects
the reference missed; neither approaches reference recall on concurrency
mechanisms; the top-tier arm produced a confident false clearance; cost differs
9.5x for identical recall.

Does **not** establish causation. The arms were given the plan and the tree;
Astra had gate receipts and validation evidence inside a retained campaign
episode. The prompt's review sentence is near-verbatim from
`codex-adversarial-review/SKILL.md`, but two deltas - "the plan's stated
invariants and proposed mechanics" and "its tests" where the skill says
"available validation evidence" - tilt toward plan conformance. Complementarity
is demonstrated; whether it is model-family or review-framing is **not
isolated**, and the next arm should be concern-scoped to concurrency against
this same sealed key to separate them.

## Amendment 2, 2026-10-19 - harness defect diagnosed and fixed for any rerun

The 10 denials were not git-specific. `--permission-mode dontAsk` denies Bash
generically outside its allowlist; the denial text is identical for every one and
names the mode, not the command. Verified fix, preflighted for $0.0037:
`--permission-mode bypassPermissions` with `--disallowedTools Write Edit
NotebookEdit` ran the exact baseline diff with **zero denials** and left the tree
clean. Write detection still rests on the disposable copy plus post-hoc
`git status --porcelain`.

The scored results above stand as recorded. They were produced under the
defective invocation, and that limitation is part of the result rather than
something to be corrected away: `opus-5`'s F9 authorship question is unresolved
because of it. Any rerun uses the fixed invocation and must preflight its exact
commands before freezing.

Consequences for current state - the four surviving conformance concerns and the
positive-judgment soundness gap - are in
`notes/s2-review-pilot-consequences-20261019.md`, deliberately outside this
frozen benchmark.

## Generation 2, 2026-10-19 - uniform rerun, and two scoring classes added

Generation 1 stands as recorded. Generation 2 reruns every arm under the fixed
invocation from amendment 2, so `opus-5` gets the baseline diff it was denied and
all arms share one environment.

**Arms:** `claude-haiku-5-5`, `claude-sonnet-5-5`, `claude-opus-5`,
`claude-opus-5-5`, cost-ascending. `claude-fable-5-1` excluded (HTTP 429, requires
usage credits).

**The prompt is unchanged.** Its two conformance-tilting deltas are left in place
deliberately: changing them would break comparability with generation 1 and with
each other. The framing confound is addressed by a separately labelled
concern-scoped variant, not by editing the shared prompt.

**Two scoring classes added, from generation 1's own data:**

- **FALSE-CLEARANCE** - the arm affirmatively declares a real defect to be a
  non-issue. Recorded separately and **never** averaged into recall. It is worse
  than a MISS, because a miss leaves a defect undiscovered while a clearance
  terminates inquiry into one that exists.
- **PARTIAL** - the arm identifies the same defective component or evidence
  surface as a reference finding but by a different mechanism. Generation 1
  scored `opus-5`'s adjacent F2 observation as a clean MISS for want of this
  class.

Ceiling per arm: **$12.00** and 25 minutes.

### Method rule adopted after three failures in one session

No presence or absence claim may be drawn from truncated command output. In this
session I made that error three times - `head -8` on the grep that hid
`startup.rs:357`, a 90-character JSON truncation that hid `opus-5`'s successful
`Read` of `executor.rs`, and `head -c 400` on a model-availability probe. Every
scoring command must print complete match sets or explicit counts, and any
`head`/`-c` limit disqualifies the output as evidence for a claim.

### Correction to confound 2, and effort control in generation 2

Confound 2 above is **wrong** in the same way. `--effort` exists and takes
**low, medium, high, xhigh, max**, matching Codex `reasoning_effort` exactly.

Generation 1 inherited `CLAUDE_EFFORT=high` from the launching environment while
the reference reviewer ran at **`xhigh`**. The arms were therefore one level
**below** the reference, not incomparable to it. Measured thinking:
`sonnet-5-5` 4,123 thinking tokens (43% of output), `opus-5` 39,468 (74%).

This does not excuse the false clearance - a confident exoneration of a real
defect is not a resource-starvation artifact, and `opus-5` had `executor.rs` in
full - but it means generation 1 compared `high` against `xhigh` while claiming
effort was not a variable.

Generation 2 pins `--effort xhigh`, matching the reference, and enforces the
ceiling with `--max-budget-usd 12.00`.

## Generation 2 results, 2026-10-20 - three of four arms

All at `--effort xhigh`, matching the `gpt-6-astra`/`xhigh` reference for the
first time. `claude-opus-5` still running at time of writing. Every arm: verdict
**BLOCK** (correct), **zero permission denials**, clean tree, no boundary
violations. The first attempts at haiku-5-5 and sonnet-5-5 died on an account
five-hour limit and are preserved at `out-void/` with `terminal_reason:
api_error`; they are not model results.

| arm | calls | wall | list-equiv $ | weighted | thinking | findings |
|---|---|---|---|---|---|---|
| `claude-haiku-5-5` | 21 | 425 s | **$0.3850** | 0.8304 Mwt | 88% of output | 8 |
| `claude-sonnet-5-5` | 22 | 440 s | $1.2369 | 0.5364 Mwt | 85% | 11 |
| `claude-opus-5-5` | 20 | 332 s | $1.9350 | 0.4151 Mwt | 77% | 8 |

### Recall against the four reference findings

| reference | haiku-5-5 | sonnet-5-5 | opus-5-5 |
|---|---|---|---|
| F1 drain panic / registry poisoning | **MATCH** (its F5) | **MATCH** (its F7) | **MATCH** (its F6) |
| F2 close timeliness, false `Resolved` | PARTIAL | PARTIAL | PARTIAL |
| F3 join handle detached, false `joined` | PARTIAL | MISS | MISS |
| F4 spawn before registry insert | **MATCH** (its F7) | **UNRESOLVED, declared** | **FALSE CLEARANCE** |

### F4 split three ways, and the split is the result

The single most diagnostic item produced three distinct behaviours at the same
effort on the same tree:

- `haiku-5-5` **established it** - "Work is not gated on registration; the comment
  overstates the protection", independently catching the misleading comment at
  `:447` as sonnet-5-5 did at `high`.
- `sonnet-5-5` **declared it unresolved** - placed under "Hardest parts to check":
  "Whether eager registration holds when the task finishes before `submit`
  returns... I could only reason from tokio semantics, not observe them." It
  neither found nor cleared, and said so.
- `opus-5-5` **cleared it** - "Registration happens under the lock before any
  spawned work can be **acknowledged**."

That clearance is the **same reasoning error** `opus-5` made at `high` in
generation 1, with the same load-bearing word: both reason about
**acknowledgement**, which does take the registry lock at `executor.rs:254`,
when the invariant concerns **publication**, which goes through
`results.pending.lock()` at `:494` and is read by `recv` at `:147`.

**The false clearance therefore reproduces across a model generation and an
effort increase.** Generation 1 could not establish that; this does. And it is
not a capability gradient: the cheapest arm got it right and the most expensive
cleared it.

`sonnet-5-5`'s third outcome is the one a gate should want, and no pre-registered
class covered it either. Recorded as **UNRESOLVED-DECLARED**: the reviewer
identifies the question, fails to settle it, and reports the failure rather than
resolving it in either direction.

### Two correlated blind spots across all three arms

**Severity.** All three matched F1 and all three graded it **low or medium**,
where the reference graded it **high**. Since findings become remediation
obligations with a scoped `repair_boundary`, a low grade on a panic that destroys
the `ProcessShutdownReport` may not generate a blocking repair. Unanimous
downgrade of the most severe reference finding.

**F2 inverted the same way three times.** Every arm found the lateness gap in the
**executor** and every arm cited `process.rs`'s `CloseDisposition::ResolvedLate`
as the **good example the executor lacks**. The reference found that same process
path *unsound* - labelling a child `Resolved` on the timeout branch without a
post-await deadline check. Three models, three times, read as exemplary the exact
code the reference found defective. That is a genuine correlated blind spot, and
it is the strongest argument in either pilot for review by a second family
rather than a second arm of the same one.

### Cost inverts against capability here

`haiku-5-5` matched **two** reference findings including the one both Opus arms
mishandled, for **$0.3850** - one fifth of `opus-5-5`. Its weighted figure is the
**highest** of any arm at 0.8304 Mwt, with 84,593 thinking tokens, because at
`xhigh` it buys a very large amount of reasoning at a low per-token price. On the
weighted scale it is the most expensive arm; on the list-equivalent scale it is
the cheapest. Any single-number cost comparison across model tiers will mislead
in one direction or the other.

## Generation 2 complete, 2026-10-20 - all four arms at xhigh

`claude-opus-5` completed: 23 calls, 576 s, $3.1950 list-equivalent, 0.4953 Mwt,
73% thinking, 7 findings, verdict BLOCK, zero denials, clean tree.

**Correction to my own scoring above.** I initially read `opus-5`'s "poison" and
"ResolvedLate" probe hits as F1 and F2 matches. Both came from its
`unsafe_at_close` finding instead. Reading the findings themselves shows
`opus-5`'s **F7 is a clean F1 match**, and the best-argued version any arm
produced: it cites the asymmetry against the executor's `drain_lock`
(`executor.rs:106`, `:291`), walks the interleaving through the `retain` at
`process.rs:348` to the `get_mut(&id).unwrap()` panic, and adds a refinement
neither the reference nor any other arm made - a *sequential* second drain is
safe, because `retain` keeps exactly the unresolved children. Fourth instance in
this session of a keyword probe misreading a result; the method rule stands.

### Final recall table

| reference | haiku-5-5 | sonnet-5-5 | opus-5-5 | opus-5 |
|---|---|---|---|---|
| F1 drain panic (ref: **high**) | MATCH (low) | MATCH (low) | MATCH (low) | MATCH (low) |
| F2 close timeliness | PARTIAL | PARTIAL | PARTIAL | PARTIAL |
| F3 join detached, false `joined` | PARTIAL | MISS | MISS | MISS |
| F4 spawn before register | **MATCH** | UNRESOLVED-DECLARED | **FALSE CLEARANCE** | **FALSE CLEARANCE** |

| | haiku-5-5 | sonnet-5-5 | opus-5-5 | opus-5 |
|---|---|---|---|---|
| list-equivalent cost | **$0.3850** | $1.2369 | $1.9350 | $3.1950 |
| findings | 8 | 11 | 8 | 7 |

### Four unanimous results, and only one of them is good

1. **All four matched F1. All four graded it low.** The reference graded it
   **high**. A unanimous four-model severity downgrade of the reference's only
   high-severity finding. Because findings become remediation obligations with a
   scoped `repair_boundary`, a low grade on a panic that destroys the
   `ProcessShutdownReport` may not produce a blocking repair.
2. **All four were PARTIAL on F2, inverted the same way.** Every arm found the
   lateness gap in the **executor** and every arm cited `process.rs`'s
   `CloseDisposition::ResolvedLate` as the **good example the executor lacks**.
   The reference found that same process path unsound. Four models read as
   exemplary the exact code the reference found defective.
3. **No arm recovered F3's mechanism.** `haiku-5-5` came closest, reaching
   detachment by a different route (dropping the executor rather than cancelling
   the drain future) and missing the false-`joined` consequence.
4. **F4 is the only item that discriminated, and it discriminated by family line,
   not by capability.** `haiku-5-5`, the cheapest arm, established it.
   `sonnet-5-5` declared it unresolved. **Both Opus arms cleared it.** With
   `opus-5` at `high` in generation 1, that is **three Opus false clearances
   across two models and two effort levels**, against zero from haiku or sonnet.

### The two false clearances are not equivalent, and the difference validates the candidate addition

`opus-5-5` at `xhigh` cleared F4 in one sentence: "Registration happens under the
lock before any spawned work can be acknowledged." Unfalsifiable as written.

`opus-5` at `xhigh` did something much better and still landed wrong. It **conceded
the reference's mechanism outright** - "holding a `std::sync::Mutex` does not stop
the task spawned at line 448 from calling the provider" - then enumerated the
registry-lock holders it believed closed the gap (`close_gate`, `request_abort`,
`close_and_drain`, `acknowledge_result`, `owned_count`), concluded "none can",
and cleared the invariant on the narrowed question of whether a *consumer* could
act on an unregistered result.

That enumeration **omits `recv`**, which reads `results.pending.lock()`
(`executor.rs:147`) and never touches the registry. So a published result is
observable for an attempt not yet in `registry.tasks`, and the arm's own
concession - that work starts before registration - is the invariant violation it
then cleared.

**This is the clearest possible evidence for the candidate addition recorded in
`notes/s2-review-pilot-consequences-20261019.md`:** requiring a positive judgment
to carry its *discriminating observation* is what made `opus-5`'s error findable
in minutes, because it published the exact set it checked. `opus-5-5`'s
one-sentence clearance supplied no such set and could only be refuted by redoing
the whole analysis. Both were wrong; only one was **checkable**, and the
difference is entirely whether the discriminating observation was stated.

### Deepest novel finding of the pilot

`opus-5`'s F4: child slots are never reclaimed, so `max_children` is a *lifetime*
launch budget rather than a concurrency bound, making the plan's named "T22 child
replacement races" unreachable by construction. No other arm and not the
reference found it, and it interacts with the `unsafe_at_close` finding - the same
reap mechanism would make a safe close reachable.

### First measured quota reading on the plan adopted 2026-10-20

The two void arms recorded the exhaustion directly: five-hour window **95% ->
100%**. That was the binding constraint, confirming the earlier inference.

On the new plan, all four arms started at **2%** five-hour and **0%** seven-day:

| window | four parallel xhigh review arms |
|---|---|
| five_hour | 2% -> ~10%, about **8 points** |
| seven_day | 0% -> 1%, about **1 point** |

Attribution is aggregate only, since the arms ran concurrently. On this plan a
full four-arm `xhigh` review batch costs roughly **8 points of a five-hour
window**, which makes the five-hour window the binding constraint at about a
dozen such batches per window, while the seven-day window is nowhere near
binding. This is the Claude-side denominator that was previously unmeasured.

## Retraction, 2026-10-20 - the "unanimous F2 inversion" was my construction, not the data

I wrote above that "every arm found the lateness gap in the **executor** and every
arm cited `process.rs`'s `CloseDisposition::ResolvedLate` as the **good example
the executor lacks**", and called it a correlated blind spot and "the strongest
argument in either pilot" for a second review family. **That is not supported by
the arms' text.** Checked sentence by sentence:

| arm | every mention of `ResolvedLate` | what it actually does |
|---|---|---|
| `haiku-5-5` | **none** | Its F2 is purely executor-side plus `clock.rs`, `lifecycle-core` and `lifecycle-store`. It never references `process.rs` for lateness at all. |
| `opus-5` | one, at its line 72 | Inside a **remediation proposal** for its `unsafe_at_close` finding ("latch only on children whose close was `TimedOut`/`Rejected`/`ResolvedLate`"). Not a claim about process-side correctness. |
| `opus-5-5` | one, at its line 29 | A structural cite of the **enum definition** at `process.rs:30-35` - "`TaskDrainReport` ... has no 'late' category (compare ...)". Says the variant exists, not that its assignment is sound. |
| `sonnet-5-5` | two | The only arm resembling my claim: "`ProcessShutdownReport` does have `ResolvedLate`." **But it also filed a finding against process-side labelling** at `process.rs:284-302` - a child "killed at once and labelled `ResolvedLate`, although the lateness came from the task wait" - overlapping the reference's own cited `:284-299`. |

So one arm of four made a structural reference I over-read, and that same arm
independently found a *different* labelling-soundness defect at nearly the same
lines the reference cited.

**What is actually true:** no arm recovered the reference's F2 mechanism (a
`Resolved` disposition assigned on the timeout branch with no post-await deadline
check, `previous_exit` relabelled unconditionally, `ChildExit` carrying no
timestamp). The lateness work was executor-side in every arm. `sonnet-5-5`'s
PARTIAL stands, but for a better reason than I gave it.

**How the error propagated, recorded because the mechanism matters.** I built a
striking four-model symmetry from one arm's sentence, committed it, and in the
next exchange it was restated back to me as "they weren't merely overlooking the
defect; they were treating its implementation as an example to follow" - further
from the text than my own version. An overstatement that is interesting travels
faster than one that is dull, and a second reader sharpened rather than checked
it. The unanimous **severity downgrade** on F1 is unaffected and remains
measured: four arms, four `low` grades, reference `high`.

This is the sixth instance of the restatement failure catalogued in
`notes/rust-inference-cost-20261018.md` and the fourth of mine.

## Verified, 2026-10-20: `opus-5`'s child-slot finding holds

Checked in the pinned snapshot:

- `process.rs:159` gates admission on `registry.children.len() >= self.max_children`.
- `process.rs:348` reclaims entries only inside `close_and_drain`, via
  `registry.children.retain(|_, record| record.exit.is_none())`.
- There is **no** `try_reap`/`try_wait` that records `exit` while the gate is open.

So while the gate is open an exited child keeps `exit: None`, `children.len()`
never decreases, and `max_children` bounds **total launches over the supervisor's
open lifetime** rather than concurrency. Child replacement after a death is
unreachable until shutdown. The plan's named "T22 launch gate/**child
replacement** races" is therefore not merely untested but **unreachable by
construction**, which is a conformance defect of a different kind from a missing
test. No other arm and not the reference found it.

It also interacts with the `unsafe_at_close` finding three arms raised: the same
open-gate reap that would make replacement reachable is what would make a safe
close reachable.
