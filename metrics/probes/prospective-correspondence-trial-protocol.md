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

Two quantities, measured separately:

- **G, the generation charge.** What recording correspondence during execution adds
  to the execution route.
- **A, the assessment saving.** What having the correspondence removes from the
  assessment route.

The mechanism wins on cost only if **G < A**. Nothing measured so far bears on G.

## 2. Primary measurement: G, the generation charge

This is the pre-registered primary endpoint, and it is chosen because the other
side is already measured and bounded.

Pilot 1 measured A under one procedure: bundled assessment $0.4103 against
adequacy-only $0.2963, so **A = $0.1140, 28%** — recorded as measured
assessment-side headroom for that procedure, explicitly not a ceiling and not a net
saving (`notes/s2-boundary-audit-20261020.md`). The same note states what was left
out: "Generating the correspondence during execution is unmeasured. Prospective
recording is not free; its cost is charged to the execution route and only D can
measure it."

So re-measuring A is the one thing this trial does not need to do. If G exceeds A
at comparable scale, the mechanism loses on cost regardless of its recall benefits,
and its case must then rest entirely on what it lets a reviewer **establish** —
a different claim requiring different evidence, pre-registered here as outcome 3.

**Break-even is pre-registered, not computed after the fact:** G < A at matched
assessment scale, with both figures in the same units and both carrying their
adjudication charges per §4.

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
| **recording** | the correspondence-generation turns themselves — **this is G** |
| **adjudication** | the human or agent checking the arms. Pilot 1's accounting omitted mine and that omission can invert the result |

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
- **Held-out omission key.** A set of obligations, defects or required evidence
  deliberately withheld from the packets, recorded verbatim in this protocol before
  execution, against which omission recall is scored. Without it, "no lost
  obligations" measures the observer, not the route.
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

## 8. The control, and the one honest option

A matched same-slice comparison is not available: an engineering slice executes
once, and a second arm on the same subject either works a stale tree or knows the
first arm's answers.

The available control is **the campaign's own prior slices as the comparison class**,
with their costs recovered from rollouts under §4's schema, and the differences
between slices stated as a limit rather than controlled away. That is a weaker
design than a matched pair and must be labelled so in the result. The alternative —
splitting one slice into two halves and giving each arm one half — changes the
subject and is rejected here.

## 9. Pre-registered outcomes

Fixed before execution so the result cannot be re-narrated afterwards.

1. **G < A, warrant equivalent at basis level.** The mechanism's cost case is
   supported for this slice, n = 1, with the §8 control limit stated.
2. **G > A, warrant equivalent.** The cost case fails. The mechanism's case rests
   entirely on recall, and that is a separate claim needing separate evidence.
3. **G > A, warrant improved.** Record as a quality result, not a cost result, and
   state the price of the improvement. This is the outcome the recall evidence
   (E8's doubled reference recall, E6 recovering a finding fifteen general reviews
   missed) makes most likely.
4. **Recording did not survive live execution.** A feasibility failure, which is a
   publishable result and the cheapest useful one.

## 10. What voids the trial

- Obligations or key not frozen before the first execution turn.
- A session that read the key acting as an arm.
- Session ids not captured, making the denominator unrecoverable.
- The subject changing after authorization without a recorded re-registration.
- Any attempt to relabel already-started work as prospective. D's exclusion stands
  and this document does not reopen it.
