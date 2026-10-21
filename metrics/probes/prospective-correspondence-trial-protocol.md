# Pre-registration draft: prospective correspondence trial

**Status: subject-independent design FROZEN 2026-10-21. Not an authorization and
not a launch.** Changes to §§1-6 and §§8-10 from here require a recorded
re-registration, not an edit. Only §7's subject-dependent fields remain to be set. Board
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

**Revised 2026-10-21.** This section previously forbade any cost conclusion beyond
`G`, on the strength of a variance band taken from builder arms. That contradicted
§2's paired endpoint, and the band was the wrong statistic.

**Correction.** The figures I cited - builder arms differing **2.65×** on the
identical S2 subject ($12.4955 against $4.7099), and single-finding recovery ranging
**$0.3697 to $4.0887** - are differences across **model tier and packet
composition**, not sampling error between two assessment arms reading the same
evidence. E2 at $4.0887 was Opus and E6 at $0.3697 was Haiku, so that range
conflates tier with composition twice over. Error class: importing a spread produced
by varying the independent variable as an uncertainty bound for a design that holds
it fixed. Consequence: the draft excluded in advance the one measurement §8 exists to
make.

**What a single paired trial establishes, pre-registered:**

1. **An observed cost difference for that subject.** `dC` is a measurement, not an
   estimate of a population parameter. Reportable as such.
2. **Feasibility** of the recording discipline under live execution, or its failure.
3. **Warrant equivalence or its absence** at property level per §6, and at basis
   level per Pilot 1's established method - two assessment routes replicated their
   per-invariant dispositions *and the bases those dispositions rested on*.

**What it does not establish:** a generalisable cost advantage, a population
estimate, or any claim about `G_diffuse`, which is unmeasured by construction (§2).

### Arm matching, required before launch

The two assessment arms **must hold model and effort identical**. A difference there
reintroduces exactly the confound this section's withdrawn variance band was made of,
and it is the one confound this design can eliminate for free. If matching is
impossible for a stated reason, the mismatch is recorded in §7 before launch and the
cost difference is reported as confounded rather than as `dC`.

## 4. Cost schema, complete, identical for both arms

**Two ledgers, reported separately.** The first draft put every charge in one
denominator while the primary formula held only three terms, which would have
attributed the experiment's own overhead to a production workflow.

- **Operational ledger** - what the two alternative workflows would cost in
  production: prospective recording (`G_direct`) plus each assessment arm
  (`C_mapped`, `C_unmapped`). This is the ledger `dC` is computed from.
- **Experimental ledger** - everything spent conducting, controlling and evaluating
  the trial: independent adjudication, key construction, isolation setup, manifest
  freezing, this protocol. Real cost, reported in full, attributed to **neither**
  production workflow.

Omitting any charge from either ledger is what made the S10 campaign uncostable;
merging them is what would make this trial's result misattributed. Each class below
is marked with the ledger it belongs to.

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
| **adjudication** | *experimental ledger.* The independent evaluation of the arms. Pilot 1's accounting omitted mine, and an omitted verification charge can invert a cheapness result - but it is measurement overhead, not a cost either production route would bear |
| **key and manifest** | *experimental ledger.* Constructing the held-out key, freezing the evidence manifest, assembling isolated input directories |

Coordinator, supervisor, worker, reviewer, repair and handoff belong to the
operational ledger as ordinary engineering cost; recording and the two assessment
arms are the operational terms `dC` uses.

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

### The common evidence corpus is a frozen manifest

Because correspondence was recorded *during* execution, retained transcripts and
receipts may themselves mention the map, which would reveal the intervention to the
control through ordinary evidence rather than through a packaging mistake.

So the eligible evidence corpus is **frozen as an explicit manifest before either
arm launches**: every artifact listed with its content hash, both arms receiving
byte-identical copies of exactly that set, and the intervention receiving **exactly
one** additional artifact, the correspondence map.

**If correspondence metadata cannot be separated from an artifact without removing
evidence relevant to the assessment, that is an interpretability limitation of the
trial and is declared as one.** It is not repaired by filtering the control's copy:
filtering changes the evidence between arms and destroys the pairing that is the
whole design. The admissible responses are to declare the limitation and report the
affected properties as confounded, or to abandon the paired comparison for that
subject. Quiet filtering is a void condition (§10).

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
   the price of the improvement. E8's doubled reference recall and E6's recovery of
   a finding fifteen general reviews missed **motivate** this hypothesis; they do
   not establish its probability for a different subject, and the first draft's
   claim that it was the most likely outcome is withdrawn as a comparative claim
   with no check behind it.
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
- **Filtering the control arm's copy of a shared artifact** to remove correspondence
  metadata (§8). The limitation is declared, not engineered away.
- The two assessment arms running at different model or effort settings without that
  mismatch being recorded in §7 before launch (§3).

## 11. Eligibility register (additive, 2026-10-21)

§6 is unchanged. This section does not alter a rule; it records which people and
sessions those rules already exclude, because eligibility depends on what each
party has read and nobody will reconstruct that from memory weeks later. **Append
an entry when a party reads the key, authors a packet, or takes a role. An
unrecorded exposure is treated as an exposure.**

Roles to be filled, with their §6 exclusions restated:

| role | excluded if |
|---|---|
| key author | also serving as an assessment arm |
| assessment arm, control | has read the key, the map, or the intervention packet |
| assessment arm, intervention | has read the key |
| independent adjudicator | authored the packets, the obligations, or the key |

### Recorded exposures

- **`claude-metrics`, session `e702a3e4`** (this protocol's author). Has read: the
  S2 sealed key for Test R - Astra's four findings on `7fdf392f` - the full E1-E8
  results, and the public obligations in
  `planning/s10-next-slice-scope-20261021.md`. The public obligations are public by
  construction and carry no exclusion.
  **Excluded from: both assessment arms**, under the rule Test R applied to its own
  author.
  **Declines: independent adjudicator.** Formally eligible, since authoring the
  protocol is not authoring the packets, obligations or key. Declined anyway: this
  session argued the mechanism's case in both directions all through its
  development and published a prediction of the likely outcome before withdrawing
  it, so its scoring would not be disinterested even where it was correct. Formal
  eligibility is not the standard this role should meet.
  **Remains available for:** recovering the cost denominators from rollouts under
  §4, checking the arithmetic, and checking the reported result against what this
  protocol pre-registered. All three are mechanical and none of them touches the
  key.

### Interpretation is not adjudication

Reading the adjudicated output, verifying its arithmetic, and deciding what it
supports is not an experimental role and carries no exclusion. **Scoring the arms
against the key is**, and if the protocol's author does that, the result is authored
by the party whose design it tests - which is the single confound §6 exists to
prevent. The two activities are kept separate by name in the final report.
