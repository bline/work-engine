# Pre-registered protocol: blind obligation compiler, 2026-10-20

Frozen before execution. Generations 2-3 and Test S established that appending
proof obligations recovers defects that general review misses, with a
specificity result: B's registration-shaped obligations caught F4 and missed F3;
D's ownership-shaped obligations caught F3 and missed F4 while looking at the F4
code region for an unrelated reason. Both obligation sets were **hand-authored by
me, knowing where the implementations failed.**

This is the arm that removes the human compiler. It is the measurement the
economic case depends on, and it is the distinction between an effective prompt
and an organizational compiler.

## E1 - compile obligations with no sight of any implementation

| element | binding |
|---|---|
| model | `claude-opus-5` at `xhigh` - matched to B and D, so authorship is the only changed variable |
| input | `s2-plan-v1.md` (sha256 `98390bd9...`) plus the repository **at the accepted predecessor baseline** |
| snapshot | `2bbc95c0` only |
| prompt | 36 lines, sha256 `32b915d2ce351adbf22dc8361785dd49eeff4aa55a9ef2baf69543cd669445bc` |

**Isolation verified before launch.** `5e0377d0` (Sol attempt 1), `7fdf392f`
(attempt 2) and `4071955` (accepted) are all **absent**. All four of
`clock.rs`, `executor.rs`, `framing.rs`, `process.rs` are **absent** from the
tree - the slice has not been built. E1 therefore cannot see a candidate, a
review, a finding, or any of B's or D's obligations.

E1 is told what makes an obligation useful - that discharging it honestly must be
able to fail, and that it should force a property over a set of states with a
stated exhaustiveness basis - because that is general doctrine this repository
already holds, not information about any defect. It is told nothing about
registration ordering, ownership across cancellation, or any invariant being more
at risk than another.

## E2 - execute the compiled obligations blind

`claude-opus-5` at `xhigh`, the Test R base prompt (sha256 `27b666a4...`)
with **E1's output appended verbatim**, against the sealed key `7fdf392f`.
Identical to B and D in every respect except who wrote the obligations.

**E1's output is frozen by sha256 and committed before E2 runs.** No editing, no
selection, no reordering, no removal of obligations I judge weak. If E1 emits a
poor set, E2 executes the poor set.

## Scoring

**E1, before E2 runs.** For each emitted obligation: can honest discharge fail?
Does it name a property, a state set, and an exhaustiveness basis? And the
decisive question - **which families does the set cover?** Specifically, whether
it contains an obligation that would force the registration/visibility question
B targeted, and one that would force the ownership-across-cancellation question D
targeted. Scored by me from the text, before any candidate is involved.

**E2.** All four reference findings under the frozen generation-2 classes
(MATCH / PARTIAL / FALSE-CLEARANCE / UNRESOLVED-DECLARED / NOVEL-VERIFIED /
UNVERIFIED / MISS), plus novel findings verified against source, plus the
combined cost of E1 + E2 against B's $3.3718 and D's $3.3569.

## Pre-registered outcomes

- **E1 emits both families and E2 recovers F3 and F4** - the strongest available
  result for the compiler path. It would mean obligation authorship is separable
  from defect knowledge, and the amortisation argument becomes testable rather
  than hypothetical.
- **E1 emits both families, E2 recovers neither** - obligation *execution* is the
  bottleneck, not authorship, and the B/D results depended on wording precision a
  general compiler does not reach.
- **E1 emits one family only** - a decomposition-**completeness** failure, which
  is a real finding even if E2 performs well on what it was given. This is the
  outcome I consider most likely: covering seven invariants uniformly is a
  different task from targeting the two that failed, and a compiler with no
  failure signal has no reason to weight them.
- **E2 false-clears something** - worse than a miss, per the generation-2
  finding, and it would indicate that obligations can license confident wrong
  conclusions when they are not the right obligations.

## Limits

n=1 at each stage. One plan. E1 is the same model family as E2, so this does not
test a cheap compiler feeding an expensive reviewer or the reverse - only whether
the authorship step can be performed without defect knowledge at all. A
**revision-validity** constraint surfaced unprompted in both B and D, each of
which scoped its exhaustiveness claim as "only valid at this revision"; if
compiled obligations are to amortise across candidates, that rebinding is a
separate unsolved problem and this arm does not address it.

## E1 result: 31 obligations, $2.072, and a near-miss on family coverage

`claude-opus-5` at `xhigh`: 24 calls, **432 s**, **$2.0720**, 0 denials, window
0.36 -> 0.37 (one point). Output: **31 obligations**, 5,549 words, frozen at
sha256 `c7b61ad234c0fb95dc186af2d93caa7dc5d7edacda087c4e13429e8b0b9f349b`.
Unedited, unselected, unreordered.

### Discriminating power: high, and higher than I expected

E1 did not restate the plan. It wrote obligations that name the weak realisation
and demand its exclusion. Examples, verbatim:

- **OB8** (invariant 1): "demonstrate ordering rather than asserting it: if the
  task is spawned first and then registered, describe the window in which the
  task can complete, attempt delivery into a sink that does not yet exist, and
  either block, drop the result, or panic... State whether any test forces the
  task to run to completion before the submitting thread finishes registering,
  and if none does, **say that the ordering is unproven**."
- **OB19** (invariant 6): "The weak realization is a boolean or atomic flag
  tested before registration, which leaves the interleaving in which a submit
  reads the flag as open, shutdown then closes the gate and snapshots the owned
  set, and the submit afterwards inserts a task that no snapshot contains and no
  drain will ever join."
- **OB16** (invariant 4): "a single pre-await check is not a discharge."
- **OB20**: "only the former detects a silently omitted task."

It also anticipated the vacuous-satisfaction trap that D's OB4 ran into, without
having seen D: **OB17** ends "A candidate that simply avoids `select!` by
serializing reads and writes should be checked instead for whether a pending
write can deadlock behind a blocked read, and you should report that structure
explicitly rather than passing it by default."

### Family coverage, scored before any candidate is involved

| family | obligation | verdict |
|---|---|---|
| **registration / visibility** (B's target, F4) | **OB8**, supported by OB5, OB6, OB7, OB19 | **COVERED**, and it describes F4's mechanism as the hypothesis to test |
| **ownership across cancellation** (D's target, F3) | OB17, OB20 | **PARTIAL** |

OB17 asks the right question in the right abstraction - *where does the partial
state live relative to the cancellable future*: "say where the partial-read
buffer and the partial-write progress live: inside the future being raced
(unsafe, bytes lost on cancellation) or inside a struct owned by the long-lived
task". But it scopes that question to **framing I/O futures in `select!`**, not to
the owning handles of resources drained at close. Nothing in the set asks the
reviewer to enumerate the states in which a join handle resides across the drain
await and say which survive cancellation of the enclosing future, which is what
D's OB1/OB2 forced and what recovered F3.

OB20 is downstream of F3's consequence and would not catch it: it demands the
report's entry count equal the snapshot's count, but a drain cancelled mid-join
produces no report at all, and the *later* drain's counts match while asserting a
join that never happened.

**So the pre-registered "one family only" outcome obtained, in its weaker form:
a near-miss rather than an absence.** The compiler reached the correct general
analysis - cancellation destroys state owned by the cancelled future - and applied
it to one resource class (I/O buffers) without generalising it to all owned
resources. That is a specific, nameable completeness gap rather than a failure of
the approach.

### The prediction this makes about E2, recorded before it runs

E2 should **recover F4** (OB8 forces it directly) and **miss F3** (nothing forces
handle ownership at close). If E2 recovers F3 anyway, the obligation set's general
discipline is sufficient without the specific obligation, which would be a
stronger result for the compiler than the coverage scoring suggests. If E2 misses
F3, the completeness gap is localised to one derivable obligation: *generalise
cancellation-survival analysis from I/O futures to every owned resource*, which is
a concrete instruction a second compiler pass could carry.

## E2 result: the prediction held, and the compiled path lost on the pre-registered metric

`claude-opus-5` at `xhigh`, Test R base prompt plus E1's 31 obligations verbatim:
25 calls, **881 s**, **$4.0887**, 0 denials, clean tree, **13 findings**, verdict
**BLOCK**. Window 0.39 -> 0.42.

### Recall against the sealed key: 1/4

| reference finding | E2 | note |
|---|---|---|
| F1 drain panic / registry poisoning | **MISS** | no obligation forced concurrent-drain serialisation on the process side; OB19 covered the launch gate, not concurrent drains |
| F2 close timeliness / false `Resolved` | **MISS** | its F2 (HIGH) covers invariant 4 on the *execution* path, a different defect |
| F3 join handle detached | **MISS** | predicted |
| **F4 spawn before registry insert** | **MATCH** (its F10) | predicted |

**Both pre-registered predictions were correct.** OB8 forced F4; nothing forced
F3, and F3 did not appear. Confirmed mechanically: OB17's disposition reads
"**`select!` branches. NONE EXIST; serialized structure reported explicitly**" -
E2 honoured OB17's own fallback clause, so the *discipline* was intact and the
*scoping* was the gap. `cancel` appears 5 times in the whole review and never in
connection with handle ownership.

### Cost, against the hand-authored arms

| arm | obligations | cost | reference recall |
|---|---|---|---|
| B (hand-authored, F4-shaped) | 3, mine | $3.3718 | **2/4** (F4, F1) |
| D (hand-authored, F3-shaped) | 5, mine | $3.3569 | **2/4** (F3, F1) |
| **E1 + E2 (compiled blind)** | 31, E1's | **$6.1607** | **1/4** (F4) |

**Correction to this table, 2026-10-20.** B was first recorded here as 1/4. It is
**2/4**: its F6 is a clean match for reference F1 - "`ProcessSupervisor::close_and_drain`
lacks the executor's drain lock and can panic under a concurrent drain", citing
`process.rs:325-333` and the `get_mut(&id).unwrap()` at `:330-331`. I had never
established B's F1 disposition and recorded an unknown as a miss. Consequence:
the compiled path's deficit is larger than first stated - **both** hand-authored
arms reached 2/4 and the compiled set reached 1/4 at 1.8x the cost. Missing
check: read the arm's own output before entering a disposition, which is the same
omission that produced the F3 CLEAN verdict.

The compiled path cost **1.8x** each hand-authored arm and recovered **half** as
many reference findings. On the metric fixed before any of this ran, **obligation authorship is
not yet separable from defect knowledge.**

### What E2 did better, stated without moving the goalposts

Recall against a sealed key is the pre-registered metric and it says 1/4. Three
things it does not capture, recorded because they bear on the next design:

1. **13 findings against D's 7**, including two HIGH. Its F2 - "Invariant 4's
   deadline recheck does not exist on the execution path", with the deadline
   machinery having *no call sites* outside `clock.rs` - is the generation-2
   novel finding, now graded HIGH and argued from a complete absence of call
   sites rather than from a single missing check.
2. **Its F4 match is better calibrated than the reference.** OB8 asked it to
   describe the window in which a task "attempt[s] delivery into a sink that does
   not yet exist". E2 checked whether that mode is reachable, found it is not
   (the sink is created in `with_clock_and_limits` at `:198-201` before any
   `submit` can run), and downgraded to **LOW** as "a false-claim and
   observability finding, not a lost-result bug" - while still closing with OB8's
   own clause: "No test forces a task to run to completion before the submitting
   thread finishes registering, so the stated ordering is unproven as well as
   unenforced."
3. It identified `executor.rs:447`'s comment as "literally true and causally
   irrelevant", because the spawned tasks never acquire the registry mutex and
   `outer`'s delivery target is a different mutex - the same reasoning the B arm
   needed obligations to reach.

### The conclusion this generation supports

The compiler step **works as a generator of discriminating obligations** - E1's
set names weak realisations, demands exhaustiveness bases, and anticipated a
vacuity trap it had never seen - and **fails on completeness**. The failure is
not diffuse: it is one missing generalisation, visible in E1's text before any
candidate was reviewed, and it propagated exactly as scored.

So the architecture claim splits. *Obligations can be compiled from a contract
without defect knowledge* is supported. *A single blind compilation pass yields a
review at least as good as a targeted one* is refuted at n=1, by a margin that
is explained entirely by one absent obligation family.

The next arm is therefore cheap and well-specified: a **second compiler pass**
whose only instruction is to generalise each cancellation- or
ownership-sensitive obligation from the resource class it names to every owned
resource in the lane, then re-run E2 against the enlarged set. If F3 appears,
decomposition completeness is reachable by iteration rather than by defect
knowledge, which is the result the economics need. If it does not, the gap is in
the compiler's analysis rather than its coverage.

## F1 across every Opus-tier run: packet volume, not packet direction

Assembled after E2, from each arm's own output rather than from summaries.

| run | obligations | F1 |
|---|---|---|
| generation-2 arm, `opus-5`/`xhigh`, prompt `27b666a4` | 0 | **MATCH** |
| B, `opus-5`/`xhigh`, same base prompt | 3 | **MATCH** (its F6) |
| D, `opus-5`/`xhigh`, same base prompt | 5 | **MATCH** (its F4) |
| E2, `opus-5`/`xhigh`, same base prompt | 31 | **MISS** |

Model, effort, subject (`7fdf392f`), base prompt bytes, tooling and isolation
recipe are identical across all four. The only variable is the appended
obligation set.

**This is a matched comparison, not a historical one.** Astra's 4/4 is not the
comparator and cannot be: Test R already records that Astra had gate receipts and
validation evidence inside a retained episode while the arms get plan plus tree.

**Two hypotheses, and the data separates them.** If the *semantic direction* of a
packet determined which defects stay discoverable, B's registration-and-visibility
obligations should have pulled attention away from a process-side drain panic -
they did not. Every packet of 5 or fewer obligations preserved F1; only the
31-obligation packet lost it. At n=1 per point that is a pattern and not a law,
but it points at **volume** rather than direction.

That makes the content-fixed organisational test the right next one: take the
**same frozen 31 obligations**, partition them by concern across independent
execution contexts, and combine the findings. It holds representation constant
and varies only organisation. If the dispersed arrangement recovers F1 while the
monolithic one does not, the finding is about attention allocation within a
context rather than about obligation quality - and it would be the first direct
evidence that organisational decomposition has value independent of what the
obligations say.

Costs to charge against it, so the comparison stays loaded: per-context doctrine
re-read (~20,100 tokens of `AGENTS.md` + `DESIGN.md` + `PHILOSOPHY.md` measured
in this repository), overlapping inference across partitions, cross-concern
interactions that no single partition can see, and integration.

**A dispersed review still cannot establish coverage of obligations the compiler
never generated.** That remains a separate responsibility, and E1's partial
cancellation coverage is the standing instance of it.

## E2-haiku: the frozen packet executes at the cheap tier, and the F1 loss replicates

`claude-haiku-5-5` at `xhigh`, byte-identical prompt to E2 (same 31 frozen
obligations, same sealed key `7fdf392f`): 29 calls, **785 s**, **$0.7243**,
0 denials, clean tree, **16 findings**, verdict **BLOCK**. Window 0.45 -> 0.46.

| | E2 (`opus-5`) | E2-haiku (`haiku-5-5`) |
|---|---|---|
| cost | $4.0887 | **$0.7243** (5.6x cheaper) |
| findings | 13 | **16** |
| reference recall | 1/4 (F4) | 1/4 + 1 partial (F4; F2 partial) |
| F1 | **MISS** | **MISS** |
| verdict | BLOCK | BLOCK |

### Disposition quality at the cheap tier is not degraded

Haiku returned an explicit OB-by-OB table with a **Basis** column, where E2 wrote
prose. Sampled entries, verbatim:

- "OB9 | Pass (weak reading); strong reading untested | No `ExecutionOutcome::*`
  or `EffectSettlement::*` value is constructed in runtime `src`; the only
  transfers are `executor.rs:469-470`... No negative controlled-peer test exists."
- "OB12 | Pass (complete space); assertions partial | Error space is exactly
  `{UnsupportedProfile, EntryUncertain, ObservationUnavailable}`... Retry search
  over `retry`, `backoff`, `resend`, `re-dispatch`, `re-execute`: no hits."
- "OB5 | Fail (convention only) | Three call paths... Nothing links the
  reservation to the claim."

Those are properties with stated bases and named enumerations, and they split weak
from strong readings unprompted. **The claim that obligations make review
mechanical enough for a cheap model is supported on this subject.** The 16
findings are *not* adjudicated, so "16 versus 13" is a count and not value.

### The displacement result is now n=2 matched pairs

| model | effort | obligations | F1 |
|---|---|---|---|
| `opus-5` | xhigh | 0 | **MATCH** |
| `opus-5` | xhigh | 31 | **MISS** |
| `haiku-5-5` | xhigh | 0 | **MATCH** |
| `haiku-5-5` | xhigh | 31 | **MISS** |

Both models matched F1 unaugmented in generation 2 on this exact tree with this
exact base prompt, and both missed it carrying the 31-obligation packet. So the
loss is **not a model-capability artifact and not a one-run accident**: it
replicates across a 5.6x capability-and-price gap.

Combined with B (3 obligations, F1 matched) and D (5 obligations, F1 matched),
the pattern across six runs is that F1 survives no packet and small packets, and
is lost by the large packet at two different tiers.

### What this changes for the recommended order

The economic premise is established cheaply: a compiled packet is dischargeable
at $0.72 with warranted dispositions. The open question is no longer *can a cheap
worker execute a packet* but *what does a large packet cost in discovery*, and
that now has a replicated answer rather than a single observation.

So the content-fixed organisational test moves ahead of further model
comparisons: partition these same 31 obligations by concern across independent
contexts, run them at `haiku-5-5` where the per-context price is $0.72 rather
than $4.09, and see whether F1 returns. If it does, the fix for the displacement
is organisational and affordable. If it does not, the loss is caused by something
other than attention allocation within one context.

## Diagnosis correction: E1's gap is subject binding, not property absence

Recorded earlier in this file as a coverage gap - "nothing in the set asks the
reviewer to enumerate the states in which a join handle resides". That is true
but mis-diagnoses the cause.

**OB17 contains the general property.** It asks where partial state lives
relative to a cancellable future - "inside the future being raced (unsafe, bytes
lost on cancellation) or inside a struct owned by the long-lived task" - which is
ownership-survival-under-cancellation stated correctly. It then binds that
property to `select!` branches and framing I/O buffers.

So the property was present and the **subject class was over-specified**. The
plan's invariant 5 names `select!` and framing explicitly, and invariant 6 names
"reap exact child handles", so a compiler reading intent alone had textual reason
to bind cancellation to I/O and reaping to children. What it could not know is
that the builder would move a Tokio `JoinHandle` out of durable registry state
into a cancellable close frame - a mechanism the plan never mentions because the
plan predates the choice to use Tokio task joins at all.

This distinction matters for the architecture. Pre-implementation compilation
answers *what must any conforming implementation establish*. Post-implementation
compilation answers *what obligations do this implementation's chosen mechanisms
create*. The second **specialises** existing requirements against a realisation
rather than discovering new ones. It is also why E3's closure operation and the
realisation pass below attack the same defect from opposite sides: closure would
widen OB17's subject class from intent, the realisation pass would widen it from
evidence.

Credit: raised by ChatGPT, verified against OB17's text.

## E4/E5 pre-registration: compile from realisation, discharge cheaply

### E4 - the realisation compiler

| element | binding |
|---|---|
| model | `claude-opus-5` at `xhigh`, matched to E1 |
| inputs | the plan (sha256 `98390bd9...`), **E1's frozen OB1-OB31** (sha256 `c7b61ad2...`), and the candidate `7fdf392f` as HEAD |
| prompt | 58 lines, sha256 `52b0ddd110e3182a74b6557e256bd406d6e73fb7a64f95c36296984aef7ee2ca` |
| withheld | every historical finding, D's obligations, the repair commits, the accepted tree, and the answer key |

**Isolation verified before launch.** `6bf17387`, `709af7d9`, `4071955`,
`5beec3b1` absent; zero `S2-C2-F` strings in the tree; D's obligation file is
outside the snapshot and not in `--add-dir`.

**The methodological requirement, built into the prompt rather than left to
scoring.** The pass is forbidden from reporting defects, dispositions or a
verdict: "If you believe you have found a defect, do not say so: express it as an
obligation whose honest discharge would settle the question either way." An
obligation qualifies only if the property is implied by the plan, it arises from a
**named implementation fact**, honest discharge can fail, and OB1-OB31 do not
already force it. A plan-level conflict goes to a separate CONFLICTS section for
referral rather than becoming an invented obligation.

This is the guard against the pass becoming an expensive reviewer in compiler
clothing. The target shape is an obligation that asserts no defect, e.g. *for each
resource handle transferred from durable owner state into a cancellable
asynchronous frame, establish what retains ownership if the enclosing future is
dropped* - which a cheap reviewer can then settle either way.

### E5 - cheap-tier discharge of only the new obligations

`claude-haiku-5-5` at `xhigh`, Test R base prompt plus **E4's new obligations
only** - not OB1-OB31. Two reasons: it isolates whether the derived obligations
alone are sufficient, and it keeps the packet small, avoiding the 31-obligation
discovery loss already measured at both tiers.

### Scoring, fixed here

Adopting the context-lifecycle session's criteria, plus one of mine:

1. **Valid and implied** - each new obligation: is the property implied by the
   plan, or imported?
2. **Provenance** - does it name the implementation fact that creates it? (mine;
   it is the difference between an obligation the implementation implies and one
   the author brought with them)
3. **Defect families exposed** - which of F1-F4 would honest discharge expose?
4. **Waste** - duplication of OB1-OB31, irrelevant obligations, invented
   requirements.
5. **Did E4 identify the defect directly?** Recorded separately. A valid result,
   but it is **not** evidence for separating compilation from judgment, and must
   not be scored as such.
6. **E5 recall** - does cheap discharge actually detect F3, and with what warrant?
7. **Loaded cost** - E4 + E5 against the pipelines below.

### Cost framing, corrected

I previously quoted a replacement-pipeline figure and an additive allowance in
adjacent sentences. Cleanly, with the realisation pass plus its discharge as X:

| pipeline | cost | cheaper than A while |
|---|---|---|
| **A** Opus build + ordinary review | $12.4955 + $4.0887 = **$16.58** | - |
| **B** Sonnet build + ordinary review + X | $8.80 + X | **X < $7.79** |
| **C** Sonnet build + X replacing review | $4.71 + X | **X < $11.87** |

Which applies is an open design question. And neither builder produced accepted
output: both carried an F3-class defect through 7/7 deterministic gates, so
passing gates is not acceptance.

### Pre-registered outcomes

- **E4 omits ownership-under-cancellation for handles** - implementation-informed
  compilation does not repair this gap, and the subject-binding diagnosis above
  predicts the wrong thing.
- **E4 generates it, E5 misses F3** - compilation succeeded, cheap discharge is
  insufficient, and the division of labour needs an expensive executor.
- **E4 generates it, E5 establishes F3** - support for the full two-pass
  mechanism: cheap construction, capable obligation derivation, cheap discharge.
- **E4 reports the defect despite the prohibition** - instruction non-adherence,
  recorded as such, and the arm cannot speak to the separation question.

### E4 attempt 1 VOID: a harness error of mine, caught by audit

**What happened.** I launched E4 with `--add-dir /home/bline/cbm-work/s2-compiler-e`
to expose `E1-obligations.txt`. That directory also contains **`E2-findings.md`
and `E2H-findings.md`** - the two reviews of this same candidate, including the
F4 match and 29 findings between them. The pre-registration withholds every
historical finding; the launch made two of them readable.

**Audit of what it actually accessed before I killed it**, from the stream, all
five tool calls: a listing of its own snapshot, a listing of the contract
directory, `Read` of `s2-plan-v1.md`, `Read` of `E1-obligations.txt`, and a
`git diff --stat` against the baseline. **It did not open either findings file.**

**Disposition: VOID anyway.** Stream preserved at `E4-VOID-attempt1-stream.jsonl`.
Isolation by audit is weaker than isolation by construction, and every other arm
in this pilot met the stronger standard. Accepting the weaker one here to save
about $1 of inference would make this arm's isolation claim unverifiable by anyone
who did not watch me check, so it is rerun rather than defended.

**Fix.** A dedicated input directory `e4-inputs/` containing exactly one file,
`E1-obligations.txt`, mode 444. Prompt path updated; new prompt sha256
`6edd5474204be6758664f06c9ef3b2174389338f04179fd065b5e7e03df7b721`. Fresh
snapshot at `7fdf392f`. The findings files are no longer reachable from any
`--add-dir`.

**Error class:** granting directory access for one file in it. This is the third
`--add-dir` defect in this pilot - the first voided both RP2 arms by omitting the
flag, the second was this. **Missing check:** list a directory's contents before
exposing it, or expose a directory built for the purpose. The latter is now the
practice.

## E4 result: 25 obligations, the prohibition held, and three plan-level conflicts

`claude-opus-5` at `xhigh`, clean rerun: 25 calls, **710 s**, **$3.2495**, 0
denials, **25 obligations (OB32-OB56)**, frozen at sha256
`3dc979f78f4679906c6b386b56add36fb1d861bb7fe417162e5140a10b43133e`.

**Isolation audit: 0 of 25 tool calls touched a withheld artifact.** Verified by
enumerating every `tool_use` in the stream.

### The prohibition held

No verdict, no dispositions, no defect assertions. Two matches on
defect-vocabulary are cross-references to what E1's OB22 and OB31 cover, not
claims about the implementation. So E4 **cannot** be dismissed as an expensive
reviewer in compiler clothing, which was the methodological risk.

### It produced the obligation whose discharge should expose F3

**OB39**, verbatim in part: "state which of three distinct conditions each
`TaskDrainReport.unresolved` entry can denote - join timeout at the deadline, a
`JoinError` from the outer task, or **registry residence with neither join handle
nor pending result** - and whether a consumer can tell an attempt that may still
deliver from one that never will."

That third condition *is* the post-F3 state: entry present, `join == None`, task
detached and possibly still running. Honest discharge requires establishing how
that state is reached. And it names its own lineage - "This specialises OB20 from
single-call cardinality to residue and repeatability" - which clause (d) of the
prompt required.

**OB32** is adjacent and independently useful: it enumerates the two-task
arrangement, notes the `AbortHandle` in the registry belongs to the *inner* task
while the stored `JoinHandle` is the *outer* one, and requires that for each way
the outer task can fail to complete - "runtime teardown dropping it, a panic
anywhere in its body, or the inner handle outliving it" - the reviewer state
whether the reserved slot is filled, released or leaked.

Both obligations assert nothing. Each names the implementation fact that creates
it, as required.

### The CONFLICTS section is the result I did not anticipate

Three plan-level conflicts, referred rather than repaired. All three check out,
and two of them reassign blame that sixteen review runs placed on builders.

1. **Invariant 4 may be unrealisable inside the accepted boundary.** The plan
   requires checking absolute deadlines, but "nothing in the plan's own boundary
   gives the runtime the authority expiry or wait budget of the entry it owns" -
   `AuthorizedEntry` exposes only effect, attempt, subject, incarnation and input,
   and widening it is an S1/core change the plan itself names as a boundary-change
   **stop**. So the deadline machinery having no production caller may be
   conformance with the boundary rather than an omission. This bears directly on
   reference F2 and on the deadline finding that every arm since generation 2 has
   reported as a defect.
2. **The plan is self-contradictory about the owned manifest.** Integration-owner
   request 2 says S2 "adds only `tokio.workspace = true`", while the mandated
   first increment requires a real disposable S1 store and a temporary directory,
   which cannot compile without dev-dependencies in that same manifest. "The plan
   simultaneously requires the proof and forbids the manifest additions the proof
   needs." That explains why Sol attempt 1 and both Claude builders added the
   identical three dev-dependencies and why the campaign's gates and Astra passed
   them: the deviation was plan-mandated. Haiku's F9 and F16 are correct against
   the text and misattributed to the builder.
3. **Root-file attribution is undecidable from a candidate snapshot.** The
   integration owner's Tokio pin and a builder's out-of-lane edit "both land as
   ordinary content in the same candidate commit", so a reviewer discharging OB2
   and OB3 "must choose between two readings the plan does not let the evidence
   separate".

None of these is a defect in the candidate. All three were routed to the owner
by a pass forbidden from reporting defects - which is exactly the authority
boundary the design requires. Their classification is corrected below.

#### Correction, 2026-10-21: two contract defects and one packet defect

Adjudicated against the plan text and the campaign record after the
context-lifecycle session disputed two of my claims. Both disputes were correct.

**Conflict 3 is not a contract defect.** I wrote that root-file attribution is
"undecidable from a candidate snapshot" and filed it against the contract. The
statement is true of the snapshot and the classification is wrong: the split is
machine-represented in the campaign record.
`s2-candidate-2-receipt.json` - the receipt for `7fdf392f`, the exact revision
Astra and every pilot arm reviewed - carries 13 paths, **11 `task_owned` and 2
`generated_dependency`** (`rust/Cargo.toml`, `rust/Cargo.lock`), and the accepted
checkpoint receipt at `4071955` carries the identical split. The evidence existed
at review time. Error class: inferring a contract gap from evidence absent in the
artifact set handed to the reviewer, without searching the campaign record that
owns it - the completeness-basis failure of rule 7, applied to a packet rather
than a finding. Consequence: a defect routed to an owner who cannot fix it, with
the actual remedy left unstated. The remedy is to put the attribution manifest in
the review packet. Check that would have caught it: enumerate the campaign records
that would carry the evidence, and grep them, before calling it unobtainable.

**Conflict 1 does not bear on reference F2, and indicts it instead.** Invariant 4
(`s2-plan-v1.md:34`) bundles two deadline concerns: "typed wall time for S1
authority checks" and "process-local monotonic instants for wait budgets. Check
absolute deadlines after awaits and before final success. A late close is recorded
as late." Only the first is unrealisable. The second is realisable from local
monotonic time - core carries typed `ClockSample { wall, wait_budget }` per the
plan's own line 25 - and it is the clause F2 violates. So invariant 4's partial
unrealisability does not excuse F2; the same invariant both mandates and enables
the check F2 omits. My claim would have excused a real defect against a boundary
limitation that does not reach it. Conflict 1 also stands only on **authority
expiry**: E4's "authority expiry or wait budget of the entry it owns" overreached,
because the plan never assigns the wait budget to `AuthorizedEntry`.

**Both surviving conflicts are detectable from the plan text alone**, because the
plan quotes the repository facts they turn on. Line 25 states that
`AuthorizedEntry` "exposes effect/attempt/subject/incarnation/input getters" and
that "Runtime/provider manifests contain no Tokio dependency". Conflict 1 is a
contradiction between lines 25 and 34; conflict 2 between line 25, line 41's
mandated disposable-store-and-temporary-directory increment, and the scope
clarification's "adds only `tokio.workspace = true`". Repository access
**verifies** those stated observations rather than supplying them - a different
function, and the one a prose-only pass cannot perform.

### Running cost of the compiled path

| step | cost |
|---|---|
| E1 compile from intent | $2.0720 |
| E2 discharge at Opus | $4.0887 |
| E2-haiku discharge at Haiku | $0.7243 |
| **E4 compile from realisation** | **$3.2495** |
| E5 discharge at Haiku | pending |

E1 + E4 + E5 is the two-pass pipeline. Against D's $3.3569 for 2/4 recall, the
two-pass path is already $5.32 before discharge, so it does not win on single-
candidate cost and was never going to. What it would buy, if E5 recovers F3, is a
**reproducible** route to the finding that fifteen general reviews missed, plus
three contract defects no review found at all.

## E5 result: compilation succeeded, cheap discharge did not. Pre-registered outcome 2.

`claude-haiku-5-5` at `xhigh` on E4's 25 realisation obligations only (prompt
sha256 `98b4b733...`): 20 calls, **691 s**, **$0.5542**, 0 denials, **18
findings**, verdict **BLOCK**. Window 0 -> 0.02.

### F3 is MISSED, and the miss is inside the right obligation

OB39 was the obligation designed to force the post-F3 state, and E5 discharged it
seriously. Its disposition enumerates exactly the three conditions OB39 demanded:

> "`TaskDrainReport.unresolved` (`92-93`) can mean three things: (i) join timeout
> with the task alive and the join restored (`313-319`); (ii) a `JoinError` from
> the outer task (`312`), join consumed, task ended abnormally and will never
> deliver; (iii) **residence with neither join nor pending** (`340-341`), **which
> arises for (ii) on a later drain**. The report cannot tell (i) from (ii) or
> (iii)."

It found condition (iii) - the post-F3 state - and attributed it to a single
cause, a `JoinError` on a later drain. It never reached the cancellation route:
that dropping the `close_and_drain` future while it holds the moved handle
detaches a **live** task into that same state. So the enumeration was right and
the causal analysis was incomplete.

**Correction, same day.** I first recorded this as "the obligation was adequate
and the failure is execution". That is not established. OB39 required
distinguishing report *conditions*; it never required enumerating the
*transitions* that produce each condition. D's obligations, which did recover F3,
explicitly demanded tracing handle ownership through cancellation of enclosing
futures. So the deficit may be in the obligation's demand rather than in the
executor. Error class: treating "points at the right state" as "demands the right
analysis". Consequence: a committed conclusion that located the deficit in the
executor on no evidence. Missing check: compare the obligation's demand against
the inference the finding requires, not against the state the finding concerns.
Raised by ChatGPT.

What is established is narrower: **the obligation located the state and the
discharge did not reconstruct how the state is reached.** Which of the two is at
fault is the subject of E6/E7 below.

### Two results that do land

**Reference F1 MATCHED** (its F5): "`ProcessSupervisor::close_and_drain` has no
drain lock, so concurrent calls can panic under the registry guard." Note this is
*consistent with* and not evidence against the obligation-coverage account: E4's
**OB34** requires enumerating "every `std::sync::Mutex` the lane owns and every
operation performed while one of its guards is held, and establish that no
reachable panic can poison a lock". E2's packet contained no such obligation and
E2 missed F1. Both arms recovered what their packet covered.

**The decomposition defect was reached independently at $0.55.** Its F2: "The
absolute-deadline check in invariant 4 is not realised, **and the lane has no
authority input to perform it**." That is E4's CONFLICTS #1, reached by a cheap
executor from the obligations alone - and it corroborates the source check I ran
separately: `AuthorizedEntry` exposes five getters and none carries authority
expiry or wait budget; `lifecycle-store` uses `AuthorityExpiresAt` only internally
at `sqlite.rs:1289`; the runtime imports the *types* at `clock.rs:3` and has no
value source.

### Two-pass pipeline, measured

| step | model | cost |
|---|---|---|
| E1 compile from intent | `opus-5` | $2.0720 |
| E4 compile from realisation | `opus-5` | $3.2495 |
| E5 discharge | `haiku-5-5` | $0.5542 |
| **total** | | **$5.8757** |

Against D's $3.3569 for 2/4 recall, the two-pass path costs **1.75x** for 1/4
(F1) plus one contract defect no review found. It does not win on this subject.

### What the generation now supports, stated at the level the evidence reaches

- **Obligations can be compiled from a contract without defect knowledge.**
  Supported (E1).
- **Obligations can be specialised against a realisation without reporting
  defects.** Supported (E4): the prohibition held, every obligation named its
  implementation fact, and three contract-level conflicts were routed to the
  plan's owner rather than repaired.
- **A realisation pass can detect a defect in the decomposition itself.**
  Supported (E4 CONFLICTS #1), verified against source, and independently
  reproduced by E5.
- **Arms recover what their packet covers and not what it omits.** Supported
  across B, D, E2, E5 - four packets, four matching outcomes.
- **Cheap discharge is sufficient for a compiled packet.** **Not supported.** E5
  executed an adequate obligation and produced a true but incomplete answer. The
  gap is one causal step: from "this state exists and is ambiguous" to "here is
  how it is reached".
- **The two-pass path is cheaper than targeted review.** Not supported at n=1 on
  one candidate.

### E6/E7: a matched pair, because my first proposal moved two variables

Running OB39 alone at `opus-5` would change model capability **and** packet size
relative to E5. The matched design, adopted from ChatGPT:

| arm | model | packet | tests |
|---|---|---|---|
| E5 (done) | `haiku-5-5`/`xhigh` | 25 obligations | missed F3 |
| **E6** | `haiku-5-5`/`xhigh` | **OB39 only** | whether packet competition caused the miss |
| **E7** | `opus-5`/`xhigh` | OB39 only | whether capability helps at identical scope |

- **E6 succeeds** - Haiku had the capability and the 25-obligation packet
  suppressed the investigation. Supports the discovery-slack account and makes
  packet sizing the lever.
- **E6 fails, E7 succeeds** - a capability floor exists for this obligation class.
- **both fail** - OB39's wording is the suspect, which is the transition-versus-state
  gap below.
- **both succeed** - the miss belongs to E5's broader context, not to either model.

E6 prompt sha256 recorded at launch; one obligation, base prompt otherwise
byte-identical to Test R's.

### State completeness versus transition completeness

A distinction the property/state/accessor triple does not currently carry, and
which both of this pilot's central failures share.

- **State completeness**: what states can exist, and what evidence is available
  in each.
- **Transition completeness**: what events, ownership transfers, interruptions or
  failures can produce each state.

B's false clearance established a property over one accessor set - state
completeness - and the required property was a different one. E5 identified the
post-F3 state and not its origins - state completeness again, transition
completeness absent. **In both cases the model stopped at a true, insufficient
result**, which is the same shape as every positive-claim error recorded in this
pilot.

Proposed extension, not yet tested: for a property concerning ownership,
ordering, cancellation or lifecycle evidence, the warrant must carry an
exhaustiveness basis for the **transitions** into each relevant state, not only
for the states and their accessors. It should be derived for those property
classes rather than imposed on every review.

## E6 result: the same obligation, alone, recovers F3. Packet size is the lever.

`claude-haiku-5-5` at `xhigh`, **OB39 alone**, base prompt otherwise byte-identical
to Test R's: 18 calls, **452 s**, **$0.3697**, 0 denials, verdict with 7 findings.

**F3 is MATCHED.** Its F6, verbatim:

> "**Cancelling the drain future permanently drops a `JoinHandle`.** Evidence:
> `executor.rs:306` (`task.join.take()`) removes the handle before the await at
> `executor.rs:308`. It is restored only in the timeout arm (`executor.rs:314-318`).
> If the caller drops the drain future at that await, the handle is dropped and the
> entry is left as `join: None` with no pending result."

Graded **low** where the reference graded medium, with a stated reason: "No work
is abandoned, since the registry and result owner remain, but the report is wrong
in the interval." It also declared its own limit under hardest-to-check: "whether
dropping a `JoinHandle` detaches the task as I read it... Both are inference."

### The matched pair

| arm | model | effort | obligation text | packet | F3 |
|---|---|---|---|---|---|
| E5 | `haiku-5-5` | `xhigh` | OB39 verbatim | **25 obligations** | **MISS** |
| E6 | `haiku-5-5` | `xhigh` | OB39 verbatim | **1 obligation** | **MATCH** |

Model, effort, subject (`7fdf392f`), base prompt bytes and the obligation's own
wording are identical. The only variable is how many other obligations
accompanied it. **The pre-registered "E6 succeeds" outcome obtained: Haiku had the
capability, and the 25-obligation packet suppressed the investigation.**

### This resolves the question I got wrong, in favour of my original claim

I recorded OB39 as "adequate" with the failure in execution; ChatGPT correctly
objected that adequacy was not established, since OB39 demands distinguishing
report *conditions* and not enumerating *transitions*. The matched pair settles
it: **the obligation was adequate as written**, because the same text alone
produced the transition analysis. The correction was right to demand the test and
the test vindicates the first claim - which is the only reason the first claim is
now usable.

E7 (`opus-5` on OB39 alone) is no longer discriminating for the capability
question: the cheap tier has the capability. It would only establish whether Opus
also does.

### Two findings, two packet sizes, same direction

| finding | lost at | recovered at |
|---|---|---|
| **F1** drain panic | 31 obligations, at **both** `opus-5` and `haiku-5-5` | 0, 3 and 5 obligations; and at 25 when OB34 covered it |
| **F3** handle detach | 25 obligations | **1 obligation, same text, same model** |

Two independent defects, both suppressed at high packet volume and recovered at
low volume, one of them with every variable but packet size held fixed.

### Consequence for packet design - corrected, because "larger is worse" overreaches

**Withdrawn as stated.** Removing 24 obligations changed both volume **and
composition**, there is one run per condition, and a four-way partition tests an
execution organisation rather than producing a dose-response curve. Raised by
both the context-lifecycle session and ChatGPT.

**Also withdrawn: "the obligation was adequate as written."** One successful
discharge establishes that OB39's wording *can* elicit the transition analysis.
It does **not** establish that enumerating states without their causal
transitions is an invalid discharge under that wording - which is the property
that would matter when accepting a future result. OB39 is demonstrated **usable**,
not demonstrated to **compel**.

**The better model, from ChatGPT, derivable from data already in this file.** Two
effects operate at once:

- **Coverage benefit**: E5 recovered F1 *because* OB34 directed attention at mutex
  operations; E2's 31 obligations lacked that coverage and missed F1.
- **Competition cost**: E5 missed F3 while carrying 25 obligations; the same
  obligation alone recovered it.

Both happened in the **same run**. So the unit to optimise is not one obligation
but a **bounded group whose shared context improves judgment more than its
competition degrades it** - which is the organisational-formation problem, not a
size rule.

**And the economics forbid the degenerate answer.** E6 cost $0.3697 for one
obligation; E5 cost $0.5542 for twenty-five. Discharging all 25 separately at
E6's observed cost is about **$9.24, roughly 17x E5**. One obligation per context
is prohibitive, so the optimum lies strictly between 1 and 25 and probably varies
by concern. No rule mandating one obligation per agent is supported.

What is supported: **the apparent capability of an execution configuration
depends on how the problem is organised for it.** The $0.37 model found what the
$4.09 model missed, at identical wording, when fewer obligations competed.

### A separate dimension this pilot has been conflating: severity calibration

E6 found F3's mechanism exactly and graded it **low**, reasoning "No work is
abandoned, since the registry and result owner remain, but the report is wrong in
the interval." The reference graded it medium, and its stated consequence is that
"a later drain then reports joined when no join occurred" - a false claim in a
close report, which is an evidence-integrity violation rather than a transient
inaccuracy. E6 noticed the report is wrong and scoped it as temporary.

This is the **second** instance: all four generation-2 arms matched F1 and all
four graded it low against a reference high.

**Detection and severity calibration are different capabilities and should be
scored separately from here on.** A model that locates a mechanism is not thereby
competent to assign its governance consequence - and in an architecture where
findings drive remediation authority, the second is the one that decides what
happens next. It is also separable, which is the useful part.

### One measurement caveat accepted

The ~20,100-token doctrine figure is a **measured configuration of this
repository's** `AGENTS.md` requirement, not a universal per-worker floor. Another
workflow's instructions may load less.

That inverts the natural reading of the economics. The cheap tier is not a
compromise to be compensated for with bigger packets - **bigger packets are the
thing that breaks it**. A plausible operating rule, untested: prefer many small
single-concern discharges at the cheap tier over one comprehensive discharge at
any tier, and charge the per-context doctrine re-read (~20,100 tokens measured in
this repository) against the saving.

Total for the F3 recovery by this route: E1 $2.0720 + E4 $3.2495 + E6 $0.3697 =
**$5.69**, against D's $3.3569 which recovered F3 and F1 with hand-authored
obligations. Still not cheaper on one candidate. But E6 alone, given the
obligation, cost **$0.37** - and that is the number that matters if obligations
amortise across candidates.

## E8 pre-registration: the content-fixed partition test

Frozen before any arm runs. This is the organisational test both reviewers
prioritised: hold the obligation **content** exactly fixed and vary only how it is
grouped into execution contexts.

### Partition, frozen and grouped by semantic concern

Grouped by the plan structure the obligations address, **not** by which historical
findings any group might recover. All 31 of E1's obligations are used exactly
once, verbatim.

| context | obligations | n | prompt sha256 (first 16) |
|---|---|---|---|
| **P1** boundary and dependency | OB1-4, OB29-31 | 7 | `22a759df33524947` |
| **P2** ownership and admission | OB5-14 | 10 | `aed78862310190b1` |
| **P3** time, framing and shutdown | OB15-21 | 7 | `f37fe44034a45995` |
| **P4** proof and tests | OB22-28 | 7 | `6ffcae3f8464fad3` |

Each prompt is the Test R base prompt (sha256 `27b666a4...`) plus its own
obligations and nothing else. `claude-haiku-5-5` at `xhigh`, serial, each in its
own fresh snapshot of `7fdf392f`. Serial rather than parallel so per-arm cost is
attributable, since cost is half the comparison.

### Comparators, already measured

| arm | grouping | cost | reference recall |
|---|---|---|---|
| E2 | 31 in one context, `opus-5` | $4.0887 | 1/4 (F4) |
| E2-haiku | 31 in one context, `haiku-5-5` | $0.7243 | 1/4 + F2 partial |
| **E8 = P1+P2+P3+P4** | 31 across four contexts, `haiku-5-5` | to measure | to measure |

### Scoring

1. **Reference recall**, union across the four contexts, under the frozen classes.
2. **Out-of-packet discovery.** F1 is not covered by any obligation in E1's set -
   verified earlier, no obligation addresses concurrent or reentrant drains - so
   F1 appearing in any partition is out-of-packet discovery and is scored in that
   category, not as discharge.
3. **Warrant quality** per obligation: property, states, exhaustiveness basis
   stated, versus assertion.
4. **Severity calibration**, scored separately from detection, per the dimension
   added above.
5. **Cross-concern omissions**: anything that spans two partitions and that no
   single context could see. This is the cost of partitioning and must be looked
   for deliberately rather than inferred from its absence.
6. **Loaded cost**: four discharges plus four doctrine loads plus the synthesis I
   perform to union the findings. The doctrine load is the measured
   ~20,100-token figure for this repository and is charged four times.

### Pre-registered outcomes

- **Union recall exceeds 1/4 at comparable cost** - organisation improves verified
  coverage at fixed content, which is the first direct evidence for smaller
  execution perspectives and the result both reviewers predicted is worth testing.
- **Union recall exceeds 1/4 but cost exceeds the monolithic arms substantially** -
  the trade-off is real and the optimum is an economic question, not a
  correctness one.
- **Union recall equals 1/4** - grouping is not the lever at this granularity, and
  E6's result was about something narrower than partition size.
- **Cross-concern omissions appear** - partitioning has a specific cost, and the
  synthesis step needs an owner rather than being assumed free.

I am the synthesiser, by the method that has erred repeatedly today, so the union
is computed from each arm's own text and recorded per arm before any union claim.

## E8 result: partitioning doubles reference recall and recovers the out-of-packet finding

Four `claude-haiku-5-5`/`xhigh` arms, serial, each on its own concern group of
E1's 31 obligations, verbatim, each in a fresh snapshot of `7fdf392f`.

| context | n | cost | turns | findings |
|---|---|---|---|---|
| P1 boundary and dependency | 7 | $0.4903 | 24 | 11 |
| P2 ownership and admission | 10 | $0.5229 | 28 | 14 |
| P3 time, framing and shutdown | 7 | $0.5669 | 27 | 12 |
| P4 proof and tests | 7 | $0.6708 | 31 | 18 |
| **total** | 31 | **$2.2509** | 110 | **55** |

### Against the monolithic arms, same content

| arrangement | model | cost | reference recall | F1 |
|---|---|---|---|---|
| 31 in one context | `opus-5` | $4.0887 | 1/4 (F4) | **MISS** |
| 31 in one context | `haiku-5-5` | $0.7243 | 1/4 (F4) + F2 partial | **MISS** |
| **31 across four contexts** | `haiku-5-5` | **$2.2509** | **2/4 (F4, F1)** | **MATCH** |

**3.1x the cost of the monolithic Haiku arm, double the reference recall, and
3.4x the findings.** The pre-registered outcome "union recall exceeds 1/4 but cost
exceeds the monolithic arms substantially" obtained: the trade-off is real and the
optimum is economic, not a correctness threshold.

### The decisive detail: F1 was recovered as out-of-packet discovery

**No obligation in E1's set addresses concurrent or reentrant drains** - verified
before these arms ran. So F1 could only appear as discovery outside the packet,
and P3 produced it:

> "Concurrent close. `ProcessSupervisor::close_and_drain` takes no drain lock
> (`process.rs:249-254`); the executor does (`executor.rs:291`). `retain` at
> `process.rs:348` can remove a record that a concurrent close then updates."

That is Astra's F1, found by a $0.57 context carrying 7 obligations, after the
same 31 obligations in one context lost it at **both** `opus-5` and `haiku-5-5`.
**Partitioning restored discovery capacity that the monolithic packet consumed**,
which is the mechanism the slack account predicted and the first direct evidence
for it at fixed content.

P2 recovered F4 with the sharpest statement of it yet - "Neither spawned closure
(`:448-477`, `:479-504`) references `registry`, so the lock does not gate the
task" - and added that the suite uses the current-thread flavour at eight named
sites, "so the suite cannot observe this window".

### F3 was NOT recovered, and a grep would have scored it wrongly

Three partitions mention detachment. All three are a **different** defect:
`OwnedExecutor` and `ResultReceiver` have no `Drop` impl, so dropping the last
owner drops `OwnedTask.join` and detaches tasks. That is a real novel finding and
it is not Astra's F3, which requires cancelling the drain future while it holds
the moved handle. Scored MISS. Recorded because the keyword match was positive and
the finding was not - the same scoring error class as the accessor grep.

### What this does and does not establish

- **Does**: at fixed obligation content, grouping changes which findings are
  produced, including findings no obligation covers. Four contexts at 7-10
  obligations each beat one context at 31, on recall and on out-of-packet
  discovery, at 3.1x the cost.
- **Does not**: identify the optimum. Four points on one subject at n=1 each. The
  content differs per group as well as the count, so composition and size are
  still confounded - the objection raised against "larger is worse" applies here
  too and is not resolved by this arm.
- **Cost accounting**: $2.2509 excludes the per-context doctrine load, which is
  paid four times rather than once. The measured figure for this repository is
  ~20,100 tokens per context.
- **Cross-concern omissions**: not yet audited. Looked for deliberately next;
  their absence is not established by this table.

## Where the partition's extra cost actually sits: output, not startup

Measured from the four E8 streams, per arm.

| arm | turn-1 context | output | output x5 | total Mwt | output share |
|---|---|---|---|---|---|
| P1 boundary | 31,057 | 130,792 | 0.654 | 1.008 | **64.9%** |
| P2 ownership | 31,552 | 134,715 | 0.674 | 1.095 | **61.5%** |
| P3 time/framing | 30,350 | 143,550 | 0.718 | 1.128 | **63.6%** |
| P4 proof/tests | 30,434 | 166,496 | 0.832 | 1.353 | **61.5%** |
| combined | - | 575,553 | 2.878 | 4.584 | **62.8%** |

**Turn-1 context is 30,350-31,552 tokens, a spread of 1,202 - just the differing
obligation text. It is 2-3% of each arm's total.** Three extra contexts add about
93K tokens of entry context, roughly **2% of the four-arm total**.

### Two corrections to claims of mine

1. **"The ~20,100-token doctrine re-read is likely the bulk of the per-context
   penalty"** - wrong. Entry context is ~31K including doctrine, prompt and
   startup material, and it is 3% of an arm. Reducing doctrine load recovers
   ~3%, not the bulk. Charging it against partition savings, as earlier sections
   of this file do, overstates it.
2. **"Output is 65-83% of each arm's total"** - wrong arithmetic. The range is
   **61.5-64.9%**. Raised by ChatGPT; conclusion unchanged.

### What this means for the optimisation

The partition's 3.1x cost is **four perspectives each reasoning and writing**,
not four startups. There is no cheap cache or startup trick that recovers it.

And the extra inference bought something: P3 recovered F1, which both monolithic
arms missed. So the question is not how to minimise context but **when the
expected value of another independently reasoning perspective justifies its
inference** - which is the organisational stopping condition already in this
repository's design: another perspective is justified when its separation
materially improves the required consequence.

**Qualification:** output covers thinking and response generation together. The
accounting locates where tokens were charged, not how much was useful reasoning
versus redundant reasoning or explanation. The streams report thinking counts
without content, so that split is not available here.

### Consequence for D's design

My earlier statement that "D must save reasoning, because that is where
essentially all the cost is" over-specifies. Savings may appear in retrieval,
context reconstruction, reasoning steps, duplicate explanation or avoided
corrective investigation - across both input and output - and better
representation could **increase** output while reducing the fully loaded cost of
an accepted decision. The target stays **inference per equivalently warranted
consequence**, not any token category.

The design point that follows: hold the **number and independence of perspectives
fixed** between D's prospective and retrospective routes. Otherwise the
representation benefit is conflated with a change of execution organisation,
which E8 shows is a large and separate effect.

## E9 pre-registration: does the conflicts channel fire before any implementation exists?

**Frozen before execution, 2026-10-21.** E4 found three plan-level conflicts with
both a CONFLICTS channel *and* the realisation in view. E1 had the plan and the
baseline repository but **no channel** - its 36-line prompt specifies none - so it
reported zero conflicts because it had nowhere to put one. Two variables differed,
and the attribution between them is unresolved.

E9 changes exactly one: **E1's configuration plus a conflicts channel.**

| element | binding |
|---|---|
| model | `claude-opus-5` at `xhigh`, matched to E1 |
| snapshot | fresh `e9-compiler` at `2bbc95c0`, the accepted predecessor baseline |
| input | `contract/` - one file, `s2-plan-v1.md`, sha256 `98390bd9...`, mode 444 |
| prompt | 50 lines, sha256 `4b9e414c0460ddd84f42a49f6c0c4ba3054bba22c2c42c052f3d5053e3d20bc5` |
| withheld | E1's and E4's outputs, every historical finding, every candidate and later revision |

**Isolation verified before launch.** HEAD `2bbc95c0`, working tree clean.
`5e0377d0`, `7fdf392f`, `4071955`, `5beec3b1` all **absent**. `clock.rs`,
`executor.rs`, `framing.rs`, `process.rs` all **absent** - the slice is unbuilt.
Zero `S2-C2-F` strings in the tree. The only `--add-dir` is the one-file contract
directory, built for the purpose, which is the practice adopted after E4 attempt 1.

**The prompt diff against E1 is confined to the output-format section** and adds no
information about what to look for - no mention of manifests, dependencies,
deadlines, authority or any invariant. The added instruction mirrors E4's minimal
phrasing: *"If you find a place where the accepted plan itself cannot be satisfied
as written, do not invent an obligation for it and do not report it as a defect:
state it in a separate CONFLICTS section for referral to the plan's owner. Name the
plan clause and the basis on which you establish the conflict."* E4's own wording is
conditioned on "the implementation reveals", which an arm with no implementation
cannot satisfy, so that condition is the one thing adapted.

**What this tests is a claim of mine, not of the mechanism.** On 2026-10-21 I
asserted that both surviving conflicts are detectable from the plan text alone,
because line 25's "Observed in pinned checkout" quotes the facts that lines 34 and
41 contradict. That assertion has no run behind it.

**Pre-registered outcomes:**

1. **Reports the manifest contradiction** - line 41's mandated disposable-store and
   temporary-directory increment against the scope clarification's "adds only
   `tokio.workspace = true`" - and/or **invariant 4's authority-expiry conflict**.
   My claim is **supported**, and a prose-only pre-dispatch pass has evidence
   behind it.
2. **Reports neither.** My claim is **contradicted by direct test**: the conflicts
   required the realisation, E4's result is attributable to the implementation
   rather than to the channel, and the cheap pre-dispatch check loses its basis.
3. **Reports a conflict I have not identified**, verified against the plan.
   NOVEL-VERIFIED, scored positively - evidence that E4's three were incomplete.
4. **Reports something that is not a conflict.** FALSE-CONFLICT, scored against the
   arm, and a caution that the channel manufactures referrals when offered.

Scoring is mine, against the plan text, with the clause quoted for each entry. I am
already contaminated on this subject and no eligibility is at stake.

**E10 cancelled before launch.** I had pre-registered a builder-with-hatch arm on
the premise that the S2 builders silently absorbed the manifest contradiction. They
did not. **S-opus disclosed it under KNOWN LIMITATIONS, naming the clause**: "The
plan says S2 adds 'only `tokio.workspace = true`'. I also added [four
dev-dependencies]... the others are needed to drive a real S1 store from a test."
S-sonnet disclosed it inline in PATHS CHANGED. So the builder channel already
existed, was used, and worked. Error class: a negative claim about an artifact's
contents asserted without opening it - the receipts were in the rig on disk.
Consequence: an arm proposed to discover what was already recorded. The real failure
is downstream and is not a model question: **the receipt was not in the review
packet**, so sixteen runs attributed to the builder a deviation the builder had
disclosed with its justification. That is the second instance of one failure -
after the attribution manifest, which was likewise present in the candidate receipt
and omitted from the packet - and both remedies are free.

## E9 result: the channel fires with no implementation, and finds a FOURTH conflict

`claude-opus-5` at `xhigh`, fresh baseline snapshot, one-file contract directory:
21 turns, **578 s** (9.6 min), **$2.4981**, 0 denials, clean tree, `subtype:
success`. **37 obligations** and **one conflict**.

Against E1, which is the same configuration without the channel: **+6 obligations
and +$0.4261, a 20.6% increase** for the channel. Weighted tokens 0.407 Mwt
(uncached 92,301; cached 1,148,367; output 40,041).

### Pre-registered outcome 3 obtained: NOVEL-VERIFIED

E9 reported neither conflict I predicted. It reported a fourth one, in the same
clause region as E4's conflict 2 but by a different mechanism: **the lock edge and
the `--locked` gates deadlock request 1's prescribed ordering.**

Its argument, compressed: request 1 has the integration owner serialize the Tokio
pin into `rust/Cargo.toml` and `rust/Cargo.lock` first, after which "S2 then adds
only `tokio.workspace = true` to its owned manifest". But `Cargo.lock` records a
per-member dependency list, so that member edge **cannot be derived by ordinary
resolution until the member manifest declares the dependency** - which the plan
orders afterwards. The builder cannot repair the lock, because the boundary and
`workspace_integrity` forbid root edits by the builder. So request 1 requires
someone to hand-serialize the `lifecycle-runtime -> tokio` edge into the lock
before the builder's manifest edit, and **the plan assigns that step to no one.**

**Verified, six checks, all passing:**

| claim | verification |
|---|---|
| request 1's wording | `s2-plan-v1.md:63`, verbatim |
| builder may not edit root | boundary at `:45`; `workspace_integrity` at `:73` "confirm no root/S1/S3/main edits by this builder" |
| root lock hash pinned at gate input | `:73`, verbatim |
| `--locked` on the proof gates | `semantic_proof` `:71` and all four `risk_proportional_checks` commands `:72` |
| lock records per-member edges | baseline `rust/Cargo.lock`: `lifecycle-runtime` lists exactly `lifecycle-core`, `lifecycle-provider`, `lifecycle-store`, `thiserror` |
| tokio absent at baseline | zero `name = "tokio"` entries in the baseline lock |

The plan is **aware of the hazard and its mitigation is insufficient rather than
absent**: request 1 ends "Do not run `--locked` gates until this root dependency
state is integrated." E9's point is that the integrated state is unreachable in the
prescribed order, so deferring the gates does not resolve it. That sharpens the
finding rather than weakening it.

### The headline: E4's conflicts are not attributable to having the realisation

E9 had the plan and the baseline repository, no implementation, and all four slice
sources absent. It produced a verified plan conflict anyway. **A conflicts channel
fires pre-implementation**, so the prose-only pre-dispatch pass is supported as a
mechanism - at a measured 20.6% premium on the compile step.

### My claim is NOT supported, and my pre-registration was defective

I asserted that both of E4's surviving conflicts are detectable from the plan text
alone, because line 25's "Observed in pinned checkout" quotes the facts that lines
34 and 41 contradict. E9 reported neither. So that claim is **unsupported by this
test**.

It is also **not refuted**, and outcome 2 was unsound as written. Outcomes 1 and 2
both presume the arm enumerates conflicts exhaustively, and **the prompt never asked
it to** - it says "if you find a place", with no completeness requirement. A single
reported conflict therefore does not establish that others were sought and not
found. Error class: a pre-registered negative outcome that treats non-report as
not-found, when the instrument requested neither exhaustiveness nor a search basis.
Consequence: the test cannot discriminate outcome 1 from outcome 2, which is half
of what it was built to do. **Check that would have caught it: read the arm's
prompt against each pre-registered outcome and ask what output would distinguish
them.** A corrected arm would require the CONFLICTS section to state its search
basis - which clauses were examined for realisability - making non-report
interpretable. Rule 7 applied to an instrument rather than to a claim.

### An independent convergence on conflict 3's reclassification

E9's **OB35** asks the reviewer to "establish how the candidate obtains Tokio and
who authored each part of it", to report "whether any `rust/Cargo.toml` or
`rust/Cargo.lock` delta is attributed to the builder rather than to the integration
owner", and "whether any other third-party dependency was added to the runtime
manifest".

So E9 placed root-delta attribution, and the extra-dependency question, in a
**reviewer obligation** rather than in CONFLICTS. That is the same classification I
reached on 2026-10-21 when reclassifying E4's conflict 3 out of the contract
category - reached independently, from the plan and baseline alone, by an arm with
no access to that reasoning. It also means the dev-dependency territory was not
missed so much as **routed to the reviewer as a conformance question**, which is a
different judgment from E4's rather than an absence of one.
