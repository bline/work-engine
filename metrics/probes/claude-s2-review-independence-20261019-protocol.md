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
