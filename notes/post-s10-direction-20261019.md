# Where to go after S10

The Rust context-lifecycle plan ends at S10. `docs/rust-context-lifecycle-s8-implementation-plan.md`
lists what S10 retains and then stops; it has no successor section, and the word
"pilot" appears nowhere in it or in any other `docs/rust-*.md`. This note records
the sequence the operator and this session converged on for what follows,
reviewed by ChatGPT across six exchanges on 2026-10-18/19.

**Status: a decided work sequence, not authorization.** It does not change the
status of any owning document, and each of those remains the authority for its
own artifact:

| document | status it keeps |
|---|---|
| [`post-migration-strategic-plan.md`](../app-server/docs/post-migration-strategic-plan.md) | IR pilot is first-class roadmap work; its staging is "a planning hypothesis, not an accepted pilot or implementation plan" |
| [`structural-plan-ir-...md`](../app-server/ideas/pending/structural-plan-ir-for-capability-aware-multi-model-execution.md) §20 | `[PLAN: OPEN - 2026-09-16. No durable execution evidence found]` |
| [`implementation-contract-compilation.md`](../app-server/docs/architecture/implementation-contract-compilation.md) | `implementation: none`, `authorization: exploration_only` |
| [`organizational-compilation.md`](../app-server/docs/architecture/organizational-compilation.md) | `design: proposed`, `authorization: unrecorded`, `implementation: none` |
| [`decision-specific-readiness.md`](../app-server/docs/architecture/decision-specific-readiness.md) | `design: accepted`, `reconciliation: reconciled`, **`authorization: implementation_authorized`**, `implementation: none` |
| [`semantic-planning-hierarchy.md`](../app-server/docs/architecture/semantic-planning-hierarchy.md) | `implementation: partial` - Supervisor/Builder live, upper layers not |

## The sequence

### 1. Finish S10. Nothing displaces it.

S10 retains frozen complete replacement review, all T01-T37 coverage decisions,
separately authorized bounded live proof, and human replacement acceptance
(`rust-context-lifecycle-s8-implementation-plan.md:396`). Those last two are
gates, not work, and a gate that waits on a human costs a whole reset cycle.
**Pre-stage both authorisations before the next reset.**

**Status as of 2026-10-19, after live3.** Live3 failed before its first semantic
replacement; the refusal was terminal and observable rather than the old silent
stall, and evidence was preserved. Live semantic adequacy is **unestablished**,
all three live attempts are **consumed and must not be replayed**, and another
attempt needs a newly bound candidate and preparation. Two bounded P2 findings
are open - `S10-LIVE-COMPILER-IDENTIFIER-CONTRACT-P2` and
`S10-LIVE-EFFECT-CAPTURE-ADEQUACY-P2` - and the authorized next work is offline
tracing, not another trial. So S10 is further from acceptance than a single
remaining gate; step 1 should be read as the offline findings first, then a
newly bound live attempt.

**Both findings are representation defects, not capability defects, and that is
load-bearing for this whole note.** The first: the compiler emitted
`interactions[0].recipient = "task owner"`, and the space violates `valid_id` in
`rust/crates/lifecycle-semantic/src/lib.rs:49-55`, which has always permitted
only ASCII alphanumerics and `-_.:/`. The loaded prompt "describes recipient and
question_id only as strings" and "does not disclose the identifier grammar or
length," so the advisory outcome is to restate the output contract to expose a
constraint the validator already enforced. No amount of reasoning effort recovers
an undisclosed grammar; Astra at `xhigh` would have failed identically.

The second: the only completed-effect source "supplies an opaque operation ID,
delivery ID, and receipt hash" and "does not bind that effect to snapshot_read,
member m1, offset/length, or attributed actual result bytes," so the compiler
proposed redoing a read it acknowledged making. "The native owner has a precise
operation/result association that the compiler prompt lacks."

That is the same failure as the one that defeated the retrospective cost
analysis, in a third place: the record preserved *that* an effect occurred
without preserving *what it was for*. Three independent instances now - telemetry
that cannot attribute inference to purpose, a receipt that cannot attribute an
effect to its operation, and a contract that never represented a required
constraint at all.

**A distinction the pilot needs to keep separate**, which this failure makes
visible: explicit structure can (i) make present-but-implicit relationships
cheaper to recover, which is an *efficiency* claim and what the probe below
measures, or (ii) make absent-but-required constraints present at all, which is
a *correctness* claim and what the identifier finding is an instance of. No
comprehension probe can measure the second, because no model can recover what was
never supplied. Only (i) is what a medium-versus-high comparison can speak to.

S10 is deliberately single-threaded: the T01-T37 table is parallelisable as a
list of obligations, but the surrounding workflow is serial by design - freeze
the candidate, pass the production-composition gate, execute the *selected*
review against that subject, return retained remediation to the *same* reviewer.
That is not a throughput loss at current capacity. One avenue runs about
37 Mweighted/day against a 162.3 Mweighted window arriving every ~2.45 days, so
S10 alone cannot exhaust a window; and after the promotion ends on 2026-11-01 a
single avenue already exceeds the Pro-200 steady allowance of ~23 Mweighted/day.

### 2. Run the Plan IR representation/effort pilot in the capacity S10 cannot consume.

About 71 Mweighted per window is left over after S10's share. That is the
pilot's budget, and the ordering rule is that the pilot never delays an S10 gate.

- **Arms stay at slice level.** The existing `docs/rust-*.md` plans are the
  corpus - *candidate source material* for a structural-representation
  experiment, not Accepted Branch Plans. They do already carry much of what
  implementation-contract compilation requires: predecessor identity and
  implementation-basis digest, bounded objective and explicit non-goals, ordered
  increments, test obligations, stop conditions, and in `rust-compiler-c0-contract.md`
  even an executor nomination. We have been compiling implementation contracts by
  hand; `implementation: none` means there is no compiler, not that the artifact
  is missing.
- **Score on the fully loaded cost of an accepted implementation** - planning,
  execution, review, remediation and integration - with quality gates reported
  beside every cost figure. A structurally precise contract can be worth more
  initial inference if it reduces findings and correction cycles. The scoring
  rule is pre-registered in
  [`rust-inference-cost-20261018.md`](rust-inference-cost-20261018.md).
- **Attribute at dispatch time, not afterwards.** The retrospective attempt to
  recover correction cost from rollout chronology failed, and the failure is
  recorded as a negative result in that note: the logs record what happened, not
  what it was for. Each arm has a declared initial attempt and declared
  correction rounds by construction, so the pilot can produce the phase
  attribution natively. It is the only way we have to get it.
- **Effort and representation vary together, in one experiment.** The arms are a
  2x2 over reasoning effort and plan representation, not two sequential pilots,
  because the question that matters is the interaction: Plan IR's H3 predicts
  *capability compensation* - that a weaker model supplied with explicit
  structure matches a stronger one working from prose. Running effort and
  representation as separate pilots cannot observe that, and it is the single
  most valuable thing the pilot can return, since it converts directly into
  routing policy.
- **The pilot must not settle artifact ownership.** See step 3. Keeping every arm
  at slice level avoids this by construction.

### 2b. Decision-specific readiness - the cheapest pilot, and the only one already authorized.

Of the architecture dimensions in play this is the one with permission in hand:
`authorization: implementation_authorized`, from its own Acceptance section -
"Implementation of the stated residue - the decision-specific readiness contract
and attributed assessment semantics - is authorized to proceed" - with
`implementation: none`. Every other dimension here is `exploration_only` or
`unrecorded`.

It also has a consumer already shipped. `skills/evidence-battery/SKILL.md`
"owns the mechanics of collecting read-only evidence cheaply. It does not decide
what counts as evidence, whether a check passed in the sense the campaign means,
or what any digest authorizes. Callers own all of that." A readiness contract is
exactly the missing caller-side decision, and the measured tool mix says where
the money is: evidence and verification 38.4% of tool calls, read and inspect
26.5% - together 59-71% per avenue. If predefined evidence requirements remove
even part of the model-directed investigation inside that, it is a larger lever
than anything in the builder's generation step.

What the pilot establishes: whether a declared readiness profile lets an
evidence battery be assembled without a model first deciding what to look for.

### 2c. The Sol-medium builder trial (scope narrowed by the 2c-result ruling)

The operator's original plan for the next reset was to switch the live S10
implementation agent's builder to **Sol at medium effort**, dogfooding it with
the Astra reviewer as the backstop. **Superseded in part:** the 2c-result ruling
permits a *controlled* medium builder trial after the reset and does not
authorize changing S10's existing live qualification path. Read what follows as
applying to that controlled trial.

**Permitted.** `rust-context-lifecycle-s8-implementation-plan.md:187` records the
*anticipated* builder as `gpt-6-sol/high` and adds "No model identity or
implementation grant is inferred from those preferences";
`experiments/context-lifecycle-rust/IMPLEMENTATION_PLAN.md:104` states lanes at
`sol/high` as a configuration record, and its reviewer line is explicitly
"Proposed." `docs/model-selection.md` opens by subordinating itself to "explicit
user choices and owning campaign/provider contracts." Nothing is contract-bound,
so this is the operator's open judgment rather than a change needing an owning
authority.

**Below the documented row, deliberately.** Model selection puts implementation
under a sufficiently clear contract at `gpt-6-sol/high`; `medium` is the row for
"bounded repository reconnaissance, routine documentation, or straightforward
edits." The same document calls effort "a starting point" and says defaults
"can be revised when task evidence supports a better authorized choice." So the
trial is best stated as a question about the contract, not the model: *is S10's
hand-written implementation contract clear enough that medium suffices?*

**What it establishes, and what it cannot.** It fills the missing cell of step
2's 2x2 - medium effort against a prose contract - which is the cheapest
available probe of capability compensation. If medium succeeds here, the
representation benefit Plan IR claims has less room to be large; if it fails,
structure has something concrete to compensate for. What it cannot give is a
controlled comparison: different slice, different remaining obligations, no
matched baseline. It is an existence check, so **the failure condition has to be
written down before the reset** or the outcome will be rationalised afterwards.

Three guards:

1. **Keep the reviewer at `xhigh`.** Both the lifecycle plan (line 111) and
   model selection put the consequential adversarial reviewer at Astra/`xhigh`.
   Lowering the builder to medium *and* the reviewer below its documented effort
   would be two reductions on the one slice carrying human replacement
   acceptance. If the reviewer is the reason the builder change is safe, the
   backstop should not move at the same time.
2. **The reviewer protects correctness, not budget.** The plan's own named
   highest risks are authority-shaped: conflating native turn completion with
   delivery success, resuming yielded work without explicit authority, capturing
   domain meaning in a second lifecycle-owned journal, losing owner-currentness
   at successor commit, restarting the host to simulate ordinary replacement
   (lines 385-391). A review finds those *after* the inference is spent, and
   retained remediation returns to the same reviewer by design. So the exposure
   is schedule and quota, not a broken candidate - and it lands on the slice
   whose gates are human-paced, where a lost cycle costs a whole reset.
3. **Abort on finding class, not on test count.** Model selection is explicit
   that "a compiler error, capacity error, or fixed number of failed tests is not
   by itself a reason to change models." So the revert rule should be a finding
   in one of the five authority-shaped classes above, or remediation reopening
   the same obligation a declared number of times - decided now, not mid-slice.

Recording requirement, from model selection's "make selection and execution
distinguishable": capture requested versus observed model and effort, the
effective identity reported by the runtime, and the session reference in the S10
receipt. An unavailable effective identity is unknown, not verified because a
config file names it. Post the lane's model change to the chatboard before
claiming it.

### 2c-probe. Pre-registered Sol-medium comprehension probe, 2026-10-19

Frozen before execution. The S10 trial in 2c is gated on this.

**Why a comprehension probe and not a cost probe.** Per-call weighted cost is
effort-independent on Sol: 15.9 (low), 16.4 (medium), 15.8 (high), 16.5 (xhigh)
kweighted per call across 35,503 October calls. That is arithmetic, not an
empirical surprise - output averages 333 tokens, so at the 5x output weight it
is ~1.7K of a ~16K call, while cached context re-send is ~12.4K, about 77%.
Halving output would make a call ~5% cheaper. So effort cannot pay for itself
per call; it can only pay by changing **how many calls** reaching an accepted
result takes.

**The claim that follows, and its limit.** Effort is a minor lever *on per-call
cost*. It is not therefore a minor economic lever overall: its task-level effect
runs through call count and correction rounds and could dominate in either
direction. That quantity is **unmeasured**. The telemetry establishes the cost
structure; only a task-level experiment establishes the consequence.

**Subject.** RP2, the permanent Rust initial-review host join - not an S10
artifact, so the probe cannot contaminate S10's frozen replacement subject.

| element | value |
|---|---|
| contract | `docs/rust-review-provider-rp2-implementation-plan.md` (444 lines) |
| accepted plan digest | `80d067d96f927e0aa2ab09e03f709e4abd748d35d4fd34c1f01b4d54d8c7c5e1` (v4) |
| pre-implementation revision | `596d2c9` (published RP1) |
| implementation actually landed | `90edc6f`, new `rust/crates/review-execution-custody/` |
| independent record | `docs/rust-review-provider-rp2-validation.md`, then RP3 |

RP2 reached `candidate_attempt: 3` under
`scope_revision: human-approved-rp2-plan-v4-20261016`, so correction history
exists and the obligation set was contested rather than trivially clear.

**Environment - an isolated repository, not a detached worktree.** A detached
worktree restores the pre-implementation *files* but shares the original object
database and refs, so `git log --all` would hand the probe the answer. Not
modifying the live repository is not the same as isolating the probe from it.

The built and verified environment is a shallow fetch of the exact revision into
an empty repository, which writes nothing to the live repository:

```
git init iso && cd iso
git fetch --depth=300 /home/bline/code/work-engine 596d2c9f...
git checkout --detach FETCH_HEAD
```

Verified at `~/cbm-work/rp2-probe/iso`: 316 commits of genuine ancestry,
`90edc6f` / `430a2b9` / `58abbfd` / `3c258b6` all absent, and
`rust/crates/review-execution-custody/` absent from the tree. Reachability
excludes descendants by construction, so no post-cutoff object is present.

Two consequences worth stating plainly:

- **Isolation is procedural, not enforced.** The observed sandbox policy is
  `danger-full-access`, so nothing physically prevents the probe reading
  `/home/bline/code/work-engine`. The live tree is declared out of scope in the
  prompt, and the rollout is checked afterwards for any path read outside the
  snapshot. That makes boundary compliance itself a scored observation, which is
  appropriate: respecting a declared boundary is the thing under test.
- **The contract is a supplied input, not repository evidence.** The RP2 plan is
  untracked and absent at `596d2c9`, so it is handed over read-only. Its
  working-tree copy hashes to
  `80d067d96f927e0aa2ab09e03f709e4abd748d35d4fd34c1f01b4d54d8c7c5e1`, byte-for-byte
  the accepted v4 digest cited by the validation record and the candidate
  receipts. Provenance is confirmed rather than assumed.

The historical implementation (`90edc6f`) and all review findings are withheld
until scoring.

**Budget.** **0.8 Mweighted hard ceiling** - a cap, not a target. Stop when the
required evidence is in hand. Actual consumption is recorded, because consumption
is a result and not an overhead.

**Procedure.** Enumerate obligations **twice**: first from the contract alone in
a cold context, then after bounded repository investigation. Before each
investigation step the probe must state which obligation or ambiguity it is
resolving.

Purpose is declared **inline in the same message that issues the retrieval**, not
in a separate turn per retrieval. A turn per retrieval would inflate exactly the
quantity the probe exists to measure.

That requirement is the point of closure worth naming: *why* a retrieval
happened cannot be recovered from a rollout afterwards - that is exactly the
failure that defeated the retrospective phase segmentation. Declared purpose at
retrieval time is dispatch-time attribution in miniature, so the probe also
tests whether that attribution is practical to ask of a builder at all, which is
the mechanism the whole pilot in step 2 depends on.

**Scoring.** The standard is the accepted obligations and authority boundaries.
The historical diff is secondary evidence only: divergence raises a question,
never a mark, because a valid implementation may differ within its discretion
and the historical one may itself be wrong.

The two passes sort every obligation into four classes:

| class | meaning |
|---|---|
| contract-communicated | stated in the contract and recovered from it |
| investigation-discovered | only reachable through repository inspection |
| overlooked | explicitly available and missed |
| genuine ambiguity | legitimately unresolved, requiring escalation |

Also scored: authority boundaries and forbidden changes; stop and
plan-deviation conditions; required verification and acceptance evidence;
inference consumed; and retrieval purpose as declared versus what the rollout
shows. Retrieval of already-cited material is **not** assumed redundant -
re-reading a cited source can be necessary to verify current contents or resolve
an ambiguity - which is why purpose is declared rather than inferred.

**Decision criterion.** A missed mandatory obligation, or any claim of authority
over a forbidden change, disqualifies *this configuration from the live S10
trial* until further evidence exists. A mispredicted but unmandated
implementation file does not. One failure does not establish that medium is
unsuitable for builder work generally, and success does not establish
equivalence with high.

**All three outcomes are informative, and they imply different remedies:**

- Complete comprehension, little inference spent - evidence that high may be
  unnecessary for bounded work under a clear contract.
- Complete comprehension, much inference spent reconstructing relationships the
  contract already contains - the Plan IR opportunity, measured.
- Incomplete comprehension - a capability-compensation signal. Note that these
  are *narrative* contracts, so this is evidence **for** Plan IR's hypothesis
  that explicit structure can substitute for capability, not evidence against
  medium.

### 2c-result. Probe outcome and the operator's scoring ruling, 2026-10-19

**Executed** 2026-10-19T15:21:53-15:23:41Z. Requested `gpt-6-sol`/`medium`,
**observed** `gpt-6-sol`/`medium` in `turn_context`. 8 calls, 1.8 minutes,
**0.099 Mweighted** - 12% of the 0.800 ceiling, about 0.06 quota points, leaving
~0.94 of the expiring remainder unspent. Read-only; no repository, provider or
S10 artifact touched. Transcript at
`~/cbm-work/rp2-probe/out/probe-result.md`; rollout
`01a154c1-0f45-7cc0-b0d2-262b1b8bd264`.

Eleven mandatory obligations enumerated from the contract alone, including the
mutation boundary with its out-of-scope list, stop and deviation conditions, and
the decisions reserved to campaign, compiler and provider owners. Pass 2: ten
contract-communicated, one genuine owner-reserved ambiguity, one overlooked,
three investigation-discovered. The declared boundary was respected.

**A defect in this note's own pre-registration, recorded rather than smoothed
over.** The frozen decision criterion disqualified a configuration for "a missed
mandatory obligation," while the frozen rubric simultaneously defined OVERLOOKED
as a scored class and called it the most valuable observation in the exercise.
The two conflict, and the criterion never said which pass it scored. One
overlooked item exists, so Pass 1 scoring would disqualify and Pass 2 scoring
would pass. **The ambiguity was identified only after the result was known**, and
the ruling below was therefore made with the outcome visible. That is a weaker
form of pre-registration than this note claims elsewhere, and future protocols
should fix the scoring pass in the frozen text.

**Operator ruling, 2026-10-19 - the standing interpretation.** Qualification is
scored on **Pass 2, the final enumeration**. Medium **passes the bounded
comprehension gate**. The Pass 1 omission is **retained as a separate finding**
and is not erased by the pass. The result is explicitly *not* an unqualified
pass.

Authority granted and withheld:

- **Permits** a *controlled* medium builder trial after the reset.
- **Does not authorize** changing S10's existing live qualification path. Section
  2c above was drafted as an in-situ switch of the live S10 implementation agent;
  that framing is now bounded by this ruling. A controlled trial is the permitted
  shape.
- Establishes nothing about equivalence with high, and nothing about
  implementation capability. This was comprehension, not construction.

**Retained finding - Pass 1 omission.** Pass 1 missed two requirements that were
explicit in the contract: that the status/receipt identify storage policy,
realized runtime limit and capacity refusal separately (line 287), and that the
held-provider test use **four** simultaneous mutation requests with one admitted
(line 389). Both verified present in the contract independently of the agent's
self-report. Comprehension from the contract alone was incomplete; recovery
required repository reads.

**Retained representation findings - three absent scalar constraints.** All three
investigation-discovered obligations are exact limits the accepted plan calls
"bounded" without stating:

| constraint | enforced in | in contract? |
|---|---|---|
| `requestId` 1-128 bytes, eight output slots, NDJSON `version: 1` | `review-workflow/src/protocol.rs`, `work-engine-review-host/src/control.rs` | absent |
| `REQUEST_MAX_BYTES = 1048576` | `review-execution-evidence/src/contract.rs` | absent |
| `STARTUP_LIMIT = 1048576`, combined input <= `4194304` | `review-workflow/src/startup.rs` | absent |

Verified by search: zero occurrences of `1,048,576`, `REQUEST_MAX_BYTES`,
`4,194,304`, `128 bytes` or `eight output` in the contract text.

This is the same defect class as `S10-LIVE-COMPILER-IDENTIFIER-CONTRACT-P2`,
where the compiler prompt described `recipient` only as a string while
`valid_id` enforced a grammar. Independently reproduced on a different artifact,
by a different model, at a different effort, hours apart. It is a **systematic
property of the repository's prose contracts**, not a single field, which is
also what the live3 reviewer advised - check neighbouring identifier, scope,
enum and size constraints together rather than patching one field. These belong
to the *correctness* branch of the distinction in step 1: no reasoning effort
recovers a constraint that was never supplied.

**Candidate invariant arising from these findings, 2026-10-19.** Proposed by
ChatGPT, recorded here as a candidate and not a decision:

> Every validator-enforced constraint relevant to an execution contract must
> either be represented in the contract, or explicitly identified as requiring
> discovery or verification.

This is stronger than "write more detailed plans" because it is a completeness
obligation with a falsifier. Two notes on where it belongs and what it costs:

- **Owner.** As an invariant on the compiled implementation contract it belongs
  to `implementation-contract-compilation.md`, which is `exploration_only` with
  `implementation: none`, so nothing can be built against it yet. But the
  *discover-or-supply* half is a readiness question, and
  `decision-specific-readiness.md` is accepted and
  `implementation_authorized` (step 2b). That is the surface where this invariant
  could be tested before any compiler exists.
- **Test cost: sample first, do not project across the corpus.** An earlier
  version of this entry multiplied the probe's 0.099 Mweighted by the 65
  `rust-*.md` documents to get ~6.5 Mweighted. **Withdrawn.** That extrapolation
  takes one bounded comprehension probe as a unit cost for a different and
  larger task - a validator-contract completeness audit - across documents that
  differ substantially in size and in how many validators they touch. It is the
  same error this note flags elsewhere, committed one paragraph after flagging
  it. The correct design is a small representative sample with its cost measured,
  and only then a corpus estimate if the sample supports one.

One claim this does **not** support: that the three findings generalise. They are
three absent constraints in one contract, found by a probe that was not looking
for all of them. The invariant is falsifiable; its cost is not yet known.

**Positive pilot evidence - verified retrieval attribution.** Both
self-classifications were checked independently and held: the overlooked item is
genuinely present in the contract, and the three discovered limits are genuinely
absent. The probe also reported, unprompted, that most of its investigation
reconfirmed relationships the contract already supplied and that the genuinely
new facts were the exact limits.

Self-declared, inline, four-way attribution of retrievals to the obligation that
motivated them was therefore **practical to ask of a builder and accurate when
audited**. That is the mechanism step 2's pilot depends on, and the only evidence
we have that it works. It cost 8 calls to obtain.

### 3. Reconcile artifact ownership before any schema is committed.

Plan IR's own §0: it is "a candidate representation of **the compiled
implementation contract** rather than a parallel semantic or authority owner,"
and "final role and artifact ownership should be reconciled against the
post-migration App Server architecture before implementation." Making it the
canonical representation of the Accepted Branch Plan would be a new
architectural decision.

The three artifacts the architecture distinguishes:

| artifact | what it represents | owner |
|---|---|---|
| Accepted Branch Plan | what semantic work is accepted, with dependencies and obligations | semantic planning hierarchy |
| Admitted execution organization | which vantages and authority projections realize it | organizational compilation |
| Compiled implementation contract | how a bounded increment is specified for execution | implementation-contract compilation |

This step is a careful hour against those two documents, not a window of
inference. It is not blocked by the promotion and does not compete with S10.

Already reconciled, and not reopened here: the P0-P3 ladder versus the
multidimensional projection profile is recorded as seam
`resolution-ladder.vs.projection-profile`, **`ALIGNED`**, with P0-P3 surviving as
named presets.

### 4. Only then authorize compiler implementation.

The organizational compiler is the larger prize and the reason the sequence
exists: it derives logical vantages, authority, continuity, capability, context
and effect boundaries from already-accepted semantic work, deciding mechanically
what can be decided and reserving genuinely unresolved choices for bounded
inference. The predefined scope of a predefined vantage is what makes evidence
collection cheap - the problem is decomposed before any model is dispatched,
rather than rediscovered per agent.

Its own justification test applies to anything we build under it
(`organizational-compilation.md:620`): "a new vantage is justified only when a
stable boundary allows the resulting roles to know materially less while
preserving or improving the required consequence."

Nothing in step 4 is costed here. The economic case for it is **projected**, and
the pilot in step 2 is what converts it to measured.

### 5. In parallel, and small: port the pressure controller to Rust.

`TokenUsagePressureProjector` (`app-server/src/token-usage-pressure-projection.mjs:59`)
and `ContextPressureController` (`app-server/src/context-pressure-controller.mjs:192`)
are live and deterministic - no model call in the transition decision - but they
exist **only** in `app-server/src/*.mjs`. There is no pressure controller
anywhere in `rust/crates/`. Until one exists, M22's measured handoff thresholds
have nothing to configure and the 1.43x they imply must not be booked when
planning capacity. This is a named integration track, not a consequence of S10
landing.

**The 30% is a loss, not a saving, and the distinction sets the pilot's
question.** M22 measured that at the observed **116K** post-replacement floor
every handoff threshold loses about **-30%**. A saving appears only if total
restart cost - standing floor plus brief plus rediscovery - falls below the
break-even of **~75K**: at a 40K restart a 70K trigger is worth **+29.6%**, at
60K a 100K trigger is worth +11.4%. About 80K of the current 116K floor is
harness-fixed (system prompt, tool schemas, reconstituted state), so the lever
is the floor, not the threshold. The lifecycle pilot must therefore ask *can a
verified continuation get total restart under ~75K*, not *can earlier
continuation deliver 30%*. Stated the second way the number has silently changed
class, from a measured penalty into a projected benefit.

## What this sequence deliberately does not do

- It does not treat the 65 `rust-*.md` documents as Accepted Branch Plans.
- It does not authorize the organizational compiler, implementation-contract
  compilation, or a Plan IR schema.
- It does not claim a linear lowering path through the architecture.
  Organizational compilation consumes accepted semantic structure; it need not
  wait for every Plan IR projection capability to exist.
- It does not book any M22 saving against the remaining promotional window.

## Why this ordering and not the other one

The operator's proposal was to shift after S10 to the org compiler and possibly
Plan IR. The reason the pilot comes first is not that the compiler is the wrong
target - it is that we cannot currently measure the thing the compiler is
supposed to improve. Six weeks of telemetry establish where inference goes by
role (build 60.4%, coordination 20.9%, review and remediation 12.1%, planning
and contract 6.6%) but not what any of it was *for*, and every retrospective
estimate of correction cost failed on exactly that. The pilot is the cheapest
instrument that produces the missing field, and it produces it as a by-product of
work we would do anyway.

The secondary reason is sequencing cost. Step 3 is an hour; discovering a wrong
artifact owner after a schema is committed is not.

## Scoring this note

Revisit when S10 is accepted. The claims that can be checked then: that S10's
gates did not cost a reset cycle; that the pilot fit in leftover capacity without
delaying S10; that the arms produced per-obligation attribution; and that
artifact ownership was reconciled before any schema was written.

## Reconciled against ChatGPT's five-pilot framing, 2026-10-19

ChatGPT proposed the same transition - S10 ends infrastructure-building and
begins measuring whether Work Engine makes developing Work Engine cheaper - and
the same primary metric, inference cost per accepted implementation, inclusive of
planning, review, correction, integration and recovery. It also agreed that the
first demonstrated saving from Work Engine's own execution architecture, as
opposed to a projection off Codex logs, is the milestone worth naming.

Three differences, with what survived:

1. **Context-lifecycle optimization as pilot 1.** Accepted as a pilot, kept at
   step 5 by readiness rather than value: it has a build prerequisite that no
   other item has, and ChatGPT says so itself. Its framing is corrected above -
   the ~30% is the measured penalty at today's floor, not a projected saving.
2. **Model effort as a separate pilot 3.** Folded into step 2's arms. Separating
   effort from representation cannot observe capability compensation, which is
   the hypothesis worth testing.
3. **Decision-specific readiness as pilot 4.** Adopted as step 2b, and promoted:
   it is the only item in the sequence whose implementation is already
   authorized, and it has a live consumer in the evidence battery. My earlier
   draft omitted it.

One thing ChatGPT folds into "additional design reconciliation" that is kept
separate here: step 3 is specifically the artifact-ownership question - whether
Plan IR represents the compiled implementation contract or the Accepted Branch
Plan - and it is cheap, bounded, and must precede schema commitment. It is not a
general design pass.
