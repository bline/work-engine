# Pre-registration draft: prospective correspondence trial

**Status: DRAFT PRE-REGISTRATION. Not an authorization and not a launch.** Board
sequence 2266 reserves the next eligible not-yet-launched slice and requires, before
launch: exact subject and baseline, frozen obligations and key, intervention and
control design, independent evaluation, a complete cost schema, and explicit
campaign-owner authorization. This document freezes the four of those that do not
depend on the subject. The subject-dependent fields are listed in §7 as
`not established`, and the trial is not launchable until the owner sets them and
authorizes it.

D's protocol is unchanged by this document. A retrospective correspondence ledger
over D's artifacts remains optional observational evidence and is not a substitute
for this trial.

## 1. The question, in falsifiable form

Does recording property-to-evidence correspondence **during execution** reduce the
total inference needed to reach an equally warranted acceptance decision?

Two quantities, **both measured on the trial subject**, neither carried in from a
previous one:

- **G, the generation charge.** What recording correspondence during execution adds
  to the execution route. Decomposes into `G_direct` and `G_diffuse` per §2.
- **A, the assessment saving.** What having the correspondence removes from the
  assessment route, measured as the difference between §8's two paired assessment
  arms on this subject.

The cost result is `dC = G_direct + C_mapped - C_unmapped` (§2). Nothing measured so
far bears on G at all, and the only existing measurement of A is Pilot 1's on a
different subject under a different procedure, so it serves as context rather than
as a threshold.

## 2. Primary measurement, revised 2026-10-21

**Superseded.** This section first made the assessment saving a settled quantity
and the generation charge the only open one. That was wrong on both counts, and the
revision below replaces it. The challenge came from the context-lifecycle session;
both of its objections are accepted.

### The primary cost result

```
dC  =  G_direct  +  C_mapped_assessment  -  C_unmapped_assessment
```

A negative `dC` is an observed cost reduction **for that slice**, conditional on
adequate and comparable warrant quality per §6. A positive `dC` means recording
cost more in the measured workflow, which does not settle the question because
better-warranted decisions may justify it - see outcome 3.

### Why A must be measured on the new subject

**Correction.** The first draft treated `A = $0.1140` as the break-even threshold
and stated that re-measuring the assessment saving was the one thing the trial did
not need to do. `$0.1140` is Pilot 1's assessment-side difference on **S2 evidence
under one procedure**. A different subject has different obligations, evidence
volume, correspondence complexity and assessor behaviour, so `A_S2` is not `A_new`.
Error class: generalising a single-subject measurement into a parameter of the
mechanism. Consequence: a trial that compared `G_new` against `$0.1140` could have
established feasibility and a comparison against a historical reference, and would
have reported it as a net saving or loss it had not measured.

`$0.1140 / 28%` is retained as **historical context only**, not a threshold, not a
ceiling, and not a decision rule.

### G does not fully decompose, and this bounds outcome 1

`G_new` has two parts and only one is measurable in this design.

- **`G_direct`** - turns and tokens spent recording correspondence. Directly
  attributable and measured.
- **`G_diffuse`** - the effect of the recording discipline on ordinary execution
  reasoning throughout the slice. **Not isolable without a matched execution
  control**, which §8 establishes is unavailable.

The reported `dC` therefore uses `G_direct` and declares `G_diffuse` unmeasured.
**Pre-registered consequence:** a `dC` near zero cannot be signed. Only an effect
larger than the plausible `G_diffuse` term supports a directional claim, and the
result must state that the term was not measured rather than treating it as zero.

## 3. What one slice can and cannot establish

n = 1 per arm. Measured same-task variance in this campaign: builder arms on the
identical S2 subject differed **2.65×** on model choice alone ($12.4955 against
$4.7099); recovery of one specific finding ranged **$0.3697 to $4.0887** across
packet compositions of the same content against the same sealed key.

**Pre-registered now, before any result exists:** a single slice cannot establish a
cost difference unless the observed effect exceeds that variance band. It can
establish three things, and the report may claim only these.

1. **Feasibility** of the recording discipline under live execution, or its failure.
2. **One honestly controlled measurement of G**, with a complete denominator.
3. **Warrant equivalence or its absence** at basis level. Pilot 1 established that
   two assessment routes replicated their per-invariant dispositions *and the bases
   those dispositions rested on*, so basis-level comparison is the established
   method and is the comparison to run.

Any cost verdict beyond "G was X for this slice" is outside what the design
supports and is excluded in advance.

## 4. Cost schema, complete, identical for both arms

Every charge below is in the denominator. Omitting any one of them is what made the
S10 campaign uncostable.

| class | what it covers |
|---|---|
| coordinator | dispatch, collection, refresh, every checking turn |
| supervisor | per-slice fixed cost; measured at 1.065 and 1.013 Mwt on slices A and B |
| worker | all builder segments, including retries and voided attempts |
| reviewer | every review episode, including retained and successor episodes |
| repair | remediation turns and the re-review they trigger |
| handoff | packet authoring, refresh authoring, invalidation records |
| **recording** | the correspondence-generation turns themselves — this is `G_direct` |
| **assessment, control** | §8's unmapped arm, every turn — this is `C_unmapped` |
| **assessment, intervention** | §8's mapped arm, every turn — this is `C_mapped` |
| **adjudication** | the human or agent checking the arms. Pilot 1's accounting omitted mine and that omission can invert the result |

`G_diffuse` has no row because it is not separately observable in this design (§2).
Its absence is declared in the result, not silently treated as zero.

**Capture requirement, mandatory at launch.** Record each process's Codex session
id in its handoff JSON **at launch time**. The counter is
`~/.codex/sessions/YYYY/MM/DD/rollout-<ts>-<session-id>.jsonl`, last
`total_token_usage`, cumulative per session. Without the id the rollout is
unfindable; with it the full denominator is recoverable after the fact. S10
recorded "counters unavailable" when they were merely uncaptured.

**Weighted tokens:** `uncached + cached/10 + 5*output`, where
`uncached = input_tokens - cached_input_tokens`. Report USD and Mwt both, and state
the model and effort for every arm.

## 5. Obligations, key and scoring

**Frozen before execution, or the trial is void.** The central control is that the
obligation set and the held-out key exist before any execution turn. This is why D
could not be enrolled retrospectively.

- **Obligation set.** Compiled and committed before dispatch. Count stated.
- **Held-out omission key.** Recorded verbatim in this protocol before execution,
  and **stated as semantic requirements, never as predicted artifacts.** The key
  freezes what matters and what a valid demonstration would have to distinguish; it
  does not name a test filename, a test implementation or an imagined proof
  strategy, because execution may find a different valid way to establish the
  property. For a cancellation-safety property the admissible form is "evidence of
  retained join ownership when an outer waiter is cancelled during the vulnerable
  interval", not the name of the test that happened to provide it.

  **This rule is derived from a failure of mine, not from theory.** Four withdrawn
  F3 absence claims treated a test name, a finding status and an invariant-level
  warrant as interchangeable. A key written in artifact terms reproduces that
  error class by construction. Without a key, "no lost obligations" measures the
  observer rather than the route.
- **Scoring classes**, as used throughout this campaign: MATCH, PARTIAL,
  FALSE-CLEARANCE, UNRESOLVED-DECLARED, NOVEL-VERIFIED, MISS. NOVEL-VERIFIED scores
  positively; it is evidence the key was incomplete.
- **State versus transition completeness** is scored separately. A route may
  enumerate every state and still miss every transition between them.

## 6. Independent evaluation

- The adjudicator **may not** have authored the packets, the obligations or the key.
- Any session that has read the key is **disqualified from being an arm**. Test R
  established this rule and applied it to its own author.
- Disqualifiers, checked post hoc: a write to a read-only subject, a read outside
  the declared boundary, or a provider call an arm was not authorized to make.
  Detected from `git status --porcelain`, from scanning every `tool_use` for
  out-of-boundary paths, and from `permission_denials`.
- The adjudication is itself charged, per §4.

### Three judgments, established separately

The frozen key defines what matters; the evaluator judges what the evidence
actually establishes. Neither assessment defines its own success criteria after
seeing its own result.

1. **Evidence coverage** - did the assessment acquire the relevant evidence?
2. **Evidence adequacy** - does that evidence discriminate satisfaction from an
   important violating behaviour? A test that passes on the defective path is
   coverage without adequacy.
3. **Disposition correctness** - does the conclusion follow from that evidence,
   including its stated unresolved limitations?

Recorded separately per property. A route may score well on coverage and badly on
adequacy, and collapsing them is what lets a reviewer's clean verdict stand on
evidence that could not have failed.

**Adjudication order is controlled.** The adjudicator scores each arm against the
frozen key, with arm order randomised and recorded, or with a separate adjudicator
per arm. Scoring arm B while holding arm A's result is an exposure as real as the
arms seeing each other.

## 7. Subject-dependent fields: `not established`

Per rule 2 these are recorded as not established rather than guessed. The owner
sets them; the trial is not launchable until they are set and authorized.

| field | value |
|---|---|
| exact subject slice | `not established` |
| baseline commit | `not established` |
| accepted plan / contract revision | `not established` |
| obligation count | `not established` |
| held-out key contents | `not established` |
| control arm definition | `not established` — see §8 |
| models and effort per arm | `not established` |
| adjudicator identity | `not established` |
| campaign-owner authorization | `not established` |

## 8. The control: paired isolated assessments of one execution

**Revised 2026-10-21.** The first draft concluded that no matched pair was
available because an engineering slice executes once, and fell back to prior slices
as a comparison class. That conflated two different controls. A matched **execution**
control is genuinely unavailable. A matched **assessment** control is available, and
it is the one that bears on the intervention, since the intervention concerns
correspondence between obligations and evidence rather than the implementation
itself. Pilot 1 already ran this design in miniature - staged against bundled
assessment on identical S2 evidence, replicating dispositions at basis level - which
is the strongest argument that the first draft's conclusion was wrong.

**The design.** One authorized engineering execution, with frozen obligations and
correspondence recorded prospectively, retaining all artifacts. Then two isolated
assessments of that same completed slice:

| arm | input |
|---|---|
| **control** | the frozen obligations and the raw retained evidence, **without** the correspondence map |
| **intervention** | the same obligations and the same raw evidence, **with** the prospectively recorded map |

This measures `A_new` directly and paired on the subject, which a comparison against
historical slices cannot do. Prior slices remain useful for contextualising execution
cost and its variability, and are **not** the primary control.

### Isolation mechanics, which are the part that actually fails

The control must be shielded from incidental correspondence metadata, not only from
the mapping file. **E4 attempt 1 of this campaign was voided because `--add-dir` on
a working directory exposed two findings files**, so this is an observed failure mode
of this exact harness, not a hypothetical.

- Each arm reads a **dedicated input directory** containing only its own inputs.
  Never a working directory, never a repository checkout that also holds the map.
- **List and count the files in each arm's input directory before launch**, state
  the count and the expected total in the result, and set inputs mode `444`.
- The control's packet must not be inferable as a redaction: it is assembled from
  the raw evidence, not produced by deleting fields from the intervention's packet,
  and no filename, index, ordering artifact or stub announces the map's existence.
- Verified post hoc per arm: `git status --porcelain` on the snapshot, every
  `tool_use` scanned for paths outside the declared input directory, and
  `permission_denials` inspected. Any leak voids that arm rather than being noted.

### The hazard that is also the mechanism

A map can reduce inference by narrowing attention, and narrowing attention is both
how the mechanism saves and how it can lose. This campaign has it in both
directions: **E8 recovered F1 as out-of-packet discovery**, while **E5 missed F3
from inside the correct obligation**. So the paired comparison scores **evidence the
arm did not acquire because the map did not point at it** as a first-class outcome
under §6's coverage judgment, not as a caveat in the discussion.

## 9. Pre-registered outcomes

Fixed before execution so the result cannot be re-narrated afterwards.

1. **`dC` negative beyond the unmeasured `G_diffuse` term, warrant equivalent on
   all three §6 judgments.** The cost case is supported for this slice, n = 1, with
   §2's signing limit and §8's control limits stated.
2. **`dC` positive, warrant equivalent.** The cost case fails in the measured
   workflow. The mechanism's case then rests entirely on what it lets a reviewer
   establish, which is a separate claim needing separate evidence.
3. **`dC` positive, warrant improved.** A quality result, not a cost result: state
   the price of the improvement. The recall evidence - E8's doubled reference
   recall, E6 recovering a finding fifteen general reviews missed - makes this the
   most likely outcome.
4. **`dC` negative, warrant worse.** The map saved inference by narrowing attention
   past the evidence. Scored, not discussed, per §8.
5. **`dC` within the unsigned band.** No directional claim. Reported as such rather
   than rounded into outcome 1 or 2.
6. **Recording did not survive live execution.** A feasibility failure, which is a
   publishable result and the cheapest useful one.

## 10. What voids the trial

- Obligations or key not frozen before the first execution turn.
- A session that read the key acting as an arm.
- Session ids not captured, making the denominator unrecoverable.
- The subject changing after authorization without a recorded re-registration.
- Any attempt to relabel already-started work as prospective. D's exclusion stands
  and this document does not reopen it.
- **Correspondence metadata reaching the control arm**, by any route: the mapping
  file, an index, a filename, an ordering artifact, a stub, or a shared working
  directory. Voids that arm, per §8's isolation mechanics.
- **A control packet produced by redacting the intervention packet.** The control's
  inputs are assembled from the raw evidence; a packet with fields removed announces
  what was removed.
- The key written in artifact terms rather than semantic requirements (§5), since a
  key naming a test or proof strategy cannot score an execution that established the
  property another way.
