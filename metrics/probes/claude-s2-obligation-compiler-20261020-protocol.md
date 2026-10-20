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
