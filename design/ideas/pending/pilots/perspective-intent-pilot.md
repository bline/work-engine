# Perspective and Intent in Governed Agentic Engineering

## An open-ended research program for qualifying evidence triggers and selecting useful perspectives through deterministic orchestration with semantic fallback

**Status:** Draft research proposal; unregistered, unapproved, and not an execution plan  
**Date:** November 22, 2026 (America/Denver)  
**Revision:** 0.4.2. Clarifies in §14.1 that early inference-phase steer trials do not need the Rust prerequisites. 0.4.1: targeted corrections: steer-timing events, an advisory hold with a deadline instead of a mandatory gate, a fresh-review arm that hands its recommendation back into the work, dispatch conditions removed from the P4 predicate, and `would_dispatch`/`would_suppress` vocabulary for shadow results. 0.4: adds blocker-gated detection, an optional deterministic per-turn canary that may only gate inference, and delivery method (steer into the working session versus fresh review) as an experimental variable with steer pilots. 0.3: keeps the 0.2 design (deterministic evaluation, falling back to bounded semantic inference) as operator design intent. Evaluator and dispatch outcomes are separated. Replay gains wrong-perspective and all-lenses checklist controls that validate the mapping step. Trigger evaluation begins with inference, whose evidence nominates deterministic predicates. Research records use the planned Rust shapes, and retrospective trigger claims are limited to evaluability; Rust prerequisites and related ideas are added. Supersedes 0.2 and the open-ended runtime-selector premise of 0.1.  
**Project:** Work Engine — research commons / governed agent engineering  
**Precursor:** [`perspective-intent-research-note.md`](../perspective-intent-research-note.md)  
**Initial evidence:** Work Engine Milestone A design and review history, including the operator interventions and exact-source findings summarized below  
**Implementation effect:** None. This proposal does not authorize research runs, inference spending, A11 implementation, new authority, or changes to existing Milestone A gates.

---

## Executive summary

An engineering agent can competently solve a locally defined problem and still spend many hours on a problem that becomes simpler under a different, authorized framing. Conversely, a difficult constraint may be indispensable; challenging it indiscriminately can make a system unsafe. The research opportunity is to determine **which observable engineering evidence warrants which independently bounded inquiry**, early enough to influence decisions, without introducing an uncontrolled council of agents.

Milestone A offers five seed cases: continuous revocation reframed as one-shot commitment; kill attribution separated from independently observed termination; a second Docker daemon proposal challenged for causal necessity; a short-lived fixed init found to be removed by its own launcher before observation; and a cross-component early task read found before the held-input boundary. Six pilots additionally separate abstraction scope from intent. These are rich *hypothesis-generating cases*, not demonstrated savings or a representative sample.

**Architectural commitment for this research (operator design intent, 2026-11-22):** The acting agent does **not** decide whether it is stuck or needs another perspective; its own framing is the likely source of the blind spot. Work Engine evaluates **versioned, evidence-defined trigger predicates deterministically** from its engineering artifacts and events. Where the deterministic aspect cannot resolve a needed fact, it **falls back to semantic inference**: an explicitly authorized, bounded classifier populates a narrow typed field from cited source material. The deterministic evaluator still computes `fired`, `not_fired`, or `unknown`, and an interpreted field is not independent proof of execution, authority, or compliance. On `fired`, a separate dispatch policy checks applicability, duplicates, cooldown, budget, and authority. It records `dispatched` or `suppressed(reason)` and maps a dispatched trigger to a candidate **perspective, intent, scope and context policy**. The resulting model session provides decision support only; the existing owner retains all decisions and authorization.

**Bootstrap path (operator, 2026-11-22):** The deterministic mechanism does not exist yet. Trigger evaluation therefore begins with **inference**: an evaluator independent of the acting agent applies the same versioned trigger specification to frozen evidence and produces the same typed records the deterministic evaluator will produce. Evidence from these inference-phase rounds then decides which premises are built as deterministic predicates and which remain declared semantic-fallback fields. Inference is the starting state of the planned design, not a competing design. As deterministic predicates land, they replace inference premise by premise.

**Relation to core contracts:** this applies §16, *determinism where judgment is unnecessary*. Mechanics decide what they can, and model judgment enters at the declared semantic fallback where meaning must be interpreted. Core §4 leaves reviewer warrant to model judgment *unless a contract says otherwise*; adopting a qualified trigger policy would be that contract, set by the owner. The research qualifies which predicates, fallback fields and mappings work; it does not reopen the deterministic-first design. Most candidate triggers depend on at least one fallback field, so fallback accuracy is a primary result.

**When detection runs (operator, 2026-11-23):** Detection does not run on every turn unless it is completely deterministic. Inference-phase detection is **gated on blockers**. A blocker is the usual entry point into the failure mode this program studies, and the agent's own recommendation for getting past it is the step most likely to continue down the same path. Once deterministic predicates exist, they may run on every turn as a **canary**. A canary hit may open inference-phase detection, as a blocker does, but a canary never dispatches an intervention through inference by itself.

**How a perspective is delivered (operator, 2026-11-23):** A change of perspective may not need another agent. A short **steer**, delivered into the working session, may be enough: it introduces a perspective and an intent without giving the answer, and needs no review packet or new session. Pi's steering queue supports this directly; a steer arrives after the current turn's tool calls and before the next model call. Delivery method is therefore an experimental variable: no action, neutral steer, targeted steer, fresh review, or a steer followed by review when later evidence justifies it. A steer is cheap, so a more sensitive trigger may be acceptable for steers than for reviews. That tolerance is set from measured disruption cost, not assumed.

This program investigates five linked but independently measured problems:

1. **Failure-mode discovery:** Which recurring failures arise from mismatched scope, intent, ordering, authority or evidence claims?
2. **Evidence-trigger discovery and qualification:** Which concrete facts or transitions predict an opportunity for a perspective intervention, and can they be evaluated deterministically, with narrowly limited semantic classification where necessary?
3. **Perspective effectiveness and mapping:** Which assigned perspective, intent, scope, context and fresh/continuing-session conditions actually help when a trigger fires?
4. **Prospective value and safety:** Do policy-directed interventions improve accepted engineering quality, elapsed time and total work after their own cost, without degrading provenance or owner authority?
5. **Delivery:** For which perspectives does a steer into the working session work, and when is a fresh, independent context necessary? Do steer effects appear immediately, later in the session, or not at all?

The trigger library is **open-world**, but its runtime mechanics are not open-ended model judgment. New rules, mappings and semantic classifiers are researched offline, versioned, independently checked and explicitly adopted. No rule may spontaneously rewrite its own specification or acquire execution authority. A useful outcome may be no trigger, no review, or an unresolved evidence request.

> **Core question:** Can Work Engine discover and independently qualify evidence conditions whose **deterministic evaluation** (falling back to bounded semantic inference for declared ambiguous inputs) selects useful perspective interventions at blockers, delivered by the cheapest effective method, sooner and more economically than ordinary continuation, generic review, or an all-lenses checklist review, while preserving accepted contracts and checked evidence?

## 1. Why this matters

### 1.1 A local answer can be correct while the global question is wrong

Engineering work forms nested and interacting scopes: statement, function, module, subsystem, cross-component workflow, architecture, governance contract, product or research objective. Each scope has a distinct purpose. During implementation, attention is often concentrated on satisfying an immediate function or subsystem contract; the reason for the contract may recede from view. As an agent encounters a race or rejection, the next turn may become increasingly occupied by a repair strategy, even when the original requirement or order of operations merits reconsideration.

The proposed failure mode is **local coherence under an unproductive framing**. It does not imply models cannot reason at higher levels or that humans always reason better. It claims only that particular task framings and accumulated session trajectories may make certain questions systematically less likely to arise.

### 1.2 A change of perspective is not the same as another review

A conventional code reviewer usually looks for defects in a concrete candidate. A premise challenger asks whether a requirement's interpretation is necessary. A lifecycle reviewer asks whether the sequence makes required observations impossible. An evidence skeptic asks what a successful API response actually establishes. Those questions can be applied at the same or different abstraction levels. The goal is to make that difference experimentally visible rather than masking it under the generic label *review*.

### 1.3 Avoiding unnecessary work is a first-class efficiency question

Work Engine already aims to move infrastructure management, evidence handling, and bounded orchestration out of model-directed execution. Perspective selection offers a complementary possibility: prevent whole branches of inference, implementation, and repair that should not be needed. That is a testable efficiency hypothesis, not an assumption that every additional reviewer pays for itself. Unnecessary meta-review could be a significant cost of its own.

## 2. Research objects and working definitions

A useful *intervention unit* is:

**(trigger specification and evidence snapshot, deterministic disposition, qualified mapping, perspective, intent, scope, context policy, authority boundary, outcome measure)**

| Object | Operational meaning | Example |
|---|---|---|
| **Objective** | The outcome the system exists to achieve | Deliver one accepted, governed A11 diagnostic |
| **Obligation / contract** | A requirement with identifiable provenance and authority | Protected task bytes must not be acquired before a permitted boundary |
| **Scope** | The implementation or decision boundary under examination | One function; Server→Host handoff; the complete admission lifecycle |
| **Perspective** | The lens and relevant relationships the agent is directed to examine | Temporal ordering, evidence custody, system architecture, product purpose |
| **Intent** | The action or judgment sought within that lens | Implement, falsify, simplify, compare, trace provenance, challenge a premise |
| **Context policy** | What evidence/history is included, excluded, or sequenced | Original objective first; rejected designs in a second pass |
| **Evidence event** | Attributed, revision- and order-bound fact or claim in the work stream | Source-order edge; blocker identity; proposed new daemon |
| **Trigger specification** | Versioned typed inputs, applicability, deterministic predicate, exclusions and disposition requirements | Observation opportunity absent before cleanup |
| **Trigger evaluation** | Mechanical computation over a frozen evidence snapshot, not agent introspection | `fired`, `not_fired`, or `unknown` |
| **Dispatch decision** | Separate policy step on a `fired` evaluation: qualification, duplicates, cooldown, budget, authority | `dispatched` or `suppressed(reason)` |
| **Semantic fallback** | Bounded evidence classification when a declared field cannot be extracted deterministically | Identify the role of a proposed mechanism from a cited design excerpt |
| **Perspective mapping** | Research-qualified, revisioned association of trigger to one or more perspective/intent/scope/context options | Temporal-observability inquiry following an ordering hazard |
| **Intervention** | A bounded fresh or continuing analysis under a selected lens | Lifecycle review of the exact launch/observe/cleanup ordering |
| **Blocker event** | Typed event showing work cannot proceed as currently framed: blocking review finding, failed gate or acceptance check, guard refusal, explicit blocked or needs-decision report, or a recurring finding lineage | Review rejects the fixed-init qualification because the observation cannot be made |
| **Detection gate** | What causes trigger evaluation to run: a blocker event, or a deterministic canary hit | Blocker recorded, so inference-phase detection runs once for it |
| **Delivery method** | How a dispatched perspective reaches the work: steer into the working session, or fresh review | Steer first; fresh review if the blocker persists |
| **Perspective steer** | Short, versioned message into the working session naming a perspective and intent, restating binding obligations, without stating an answer | "Perspective: lifecycle observability. Examine how the required observation relates to the operations that change or destroy the observed state…" |
| **Disposition** | An evidence-backed finding, refused hypothesis, or owner decision request | Propose a bounded hold before cleanup; do not assert qualification |
| **Measured benefit** | Outcome compared with a meaningful control | Reduced accepted-work cost, fewer invalid proposals, shorter time to a verified decision |

These objects are research constructs. **Perspective** and **intent** are operational features of an assigned task, not claims that a model has a human-style internal mental state. They should be varied and evaluated independently when possible.

The taxonomy must remain *open-world*. Research may discover new perspectives, overlapping dimensions, or evidence that some proposed categories cannot be distinguished reliably. A useful classification system should support versioned additions, merges, retirement, and uncertainty rather than presuming a complete list. **Open-world discovery must not become an open-ended runtime selector:** live predicate execution, fallback permissions, and perspective mappings are closed, owner-adopted specifications.

## 3. Seed cases and the actual level of evidence

Five Milestone A cases offer an unusually useful starting corpus because they span different kinds of interventions. They are not an unbiased sample of agent activity and should not be treated as five demonstrated successes.

| Case | Previously salient framing | Different perspective or intent | Observed outcome | Unproved claim to avoid |
|---|---|---|---|---|
| **A — Revocation after admission** | Preserve revocability/currentness through Server effects and Host entry | Ask whether one-shot commitment should be the point of no return | Operator selected an irrevocable-at-admission design direction | That protected authority consumption or the live route is qualified |
| **B — Who killed the process?** | Cleanup actor's success and attribution | Separate termination action from independent proof of terminal absence | Design responsibility clarified: killer acts, independent verifier observes | That a specific termination witness is installed or sufficient |
| **C — Second Docker daemon** | Add an isolated runtime to address uncertain cleanup/observation | Distinguish runtime control, process observation, and settlement | Sol retracted unproven need for a second daemon | That the existing daemon has qualified control separation |
| **D — Fixed init vanishes before observation** | Prearm sophisticated kernel instrumentation because the process may disappear | Inspect lifecycle ordering: Host removes it immediately after `Running` | Source order identified; Sol revised the plan to try a bounded observation hold | That the hold is implemented or proves executable/post-init state |
| **E — Early protected-task read** | Hide or defer task delivery later in the Docker/Pi path | Trace earliest protected access across Server and Host | Agent analysis found that ordinary validation opened the task before hold selection; a metadata-only early input was proposed | That replacing ordinary validation or delivering to the actual held receiver is qualified |

**Why Case E is essential:** It is a warning against turning “challenge every constraint” into an unsafe optimization strategy. The requirement to protect task bytes is substantive. The useful change is to move the boundary to where the access actually occurs, not eliminate the protection. This case also demonstrates that independent AI review can find architectural defects, not only source-level bugs.

The initial evidence should be pinned to the actual historic artifacts, including Milestone A `CHECKIN.md`, the relevant chatboard records and design packets, `pi_public_hold.rs` and the fixed init at `3693e48c52eeb5cd02e39b391bb773dfd0a65545`, and Sol's subsequent plan-only revision. `CHECKIN.md` is not in this repository and must be located and pinned first. The fixed-init Host source is `rust/crates/work-engine-manual-host/src/pi_public_hold.rs`; the `Running` check followed by cleanup is at roughly lines 299–311. The next task is to extract an independently checked case record for each case, not to infer code behavior from retrospective prose.

## 4. A deliberately broad research agenda

The following questions define *workstreams* rather than a predetermined sequence of features. They can branch, combine, or be retired as evidence accumulates.

### RQ1 — Failure-mode discovery

What recurring signatures indicate that a model is solving a locally plausible but globally unproductive formulation? Do they vary by code scope, task type, model family, context length, review history, or stage of engineering? Can we reliably distinguish a framing failure from an ordinary implementation challenge or a genuinely missing capability?

### RQ2 — Trigger discovery and validation

What **observable, attributable events and artifact transitions** distinguish useful moments from ordinary hard engineering? Which signals can be specified as deterministic predicates with explicit evidence coverage, validity windows, and negative controls? For genuinely ambiguous fields, how often is a bounded semantic classifier necessary, what are its error and abstention rates, and how are its outputs prevented from becoming unsupported proof? Measure precision/recall, earliest valid firing, false-positive and missed-opportunity costs, `unknown` prevalence, and suppression after an owner disposition. On the retrospective seed corpus only evaluability can be established; rates need prospective rounds. Which premises does the inference phase resolve reliably enough to justify building them deterministically?

### RQ3 — Perspective and intent discovery

Which perspectives reveal materially distinct information? Are they best described by abstraction level, time, organizational boundary, causal relationship, proof obligation, or something yet undiscovered? Does the same intent applied at different scopes yield different results? Which pairs are complementary, redundant, or counterproductive?

### RQ4 — Context and fresh-session effects

How much of an improvement comes from a **fresh context** versus a changed objective, selective presentation of evidence, withheld repair proposals, or a different model? Can a continuing session achieve the same effect with a cleanly reframed task? When does a fresh session lose valuable facts and rediscover already-settled decisions?

### RQ5 — Intervention timing

Is a premise challenge most valuable before implementation, after the first failed review, after repeated repairs, or when complexity crosses a measurable threshold? Can we discover earlier warning signals without saturating the system with reviews? Is the blocker gate early enough, or does the failure mode often start before any blocker appears? Does intervening before the agent's proposed remedy is accepted matter more than intervening after?

### RQ6 — Outcome quality and proof integrity

Do targeted perspectives improve accepted, independently checked engineering outcomes? Do they introduce unacceptable requirement erosion, overlooked evidence, fabricated authority, or new defects? Can reviewers correctly distinguish **the contract may be reconsidered by its owner** from **the contract may be ignored**?

### RQ7 — Economic and environmental efficiency

How much additional inference and coordination does the intervention cost, and how much useful engineering work does it save? How do findings affect model tokens, calls, retries, context growth, review cycles, container runtime, elapsed time, and accepted outcome quality? Energy/emissions claims require an explicit measurement or defensible proxy, not a direct conversion from token reductions.

### RQ8 — Learning and transfer

Can evidence from one class of blocker improve perspective selection on unseen projects, languages, runtimes, and architectural patterns? When does specialization to Work Engine's governance style overfit? How should the catalog retain negative results and revised hypotheses?

### RQ9 — Organizational integration

If research supports the approach, what **versioned trigger, selector and mapping contracts** should the control plane execute, how does a bounded semantic-classification fallback supply only declared fields, and how should the dispatcher enforce one-attempt budget/cooldown, explicit unknowns and owner authority? What does organizational compilation need to know without giving the acting agent permission to select its own perspectives or silently change a contract?

### RQ10 — Delivery method

When does a steer into the working session reproduce the benefit of a fresh review, and when does accumulated context prevent it? Candidate hypothesis: steers work when the agent lacked a *question* (ordering, earliest access) and work less well when the agent is anchored on an *assumption* it has invested in (revocability, a proposed daemon). How much of a fresh review's benefit comes from the perspective itself, and how much from the fresh context? Does a steer that has no immediate effect change later decisions in the same session, and does it survive compaction?

## 5. Candidate failure-mode taxonomy — version 0, intended to change

The proposal should seed discovery without mistaking these hypotheses for an exhaustive list.

| Candidate mode | Observable symptom | Candidate missed question | Initial case |
|---|---|---|---|
| **Premise lock-in** | More mechanisms are added to satisfy a difficult inherited condition | Is that interpretation of the condition actually required? | A |
| **Proof conflation** | An action or API response is presented as independent evidence of its effect | What does the observer independently know? | B, C |
| **Mechanism substitution** | A new daemon/service/runtime is proposed without a causal link to the missing proof | Which precise missing property would the new mechanism establish? | C |
| **Self-created observability gap** | A process/state disappears before a required witness can inspect it | Can the lifecycle preserve a bounded observation opportunity? | D |
| **Boundary misplaced downstream** | A protection is applied after an earlier access or effect | Where is the *first* crossing of the protected boundary? | E |
| **Scope tunnel vision** | Local fixes satisfy one component but undermine another | What larger relationship makes this local change meaningful? | E, hypothesized generally |
| **Authority ambiguity** | The component providing a fact is also assumed to authorize its use | Who may select, revoke, verify and accept? | A, B |
| **Context-path anchoring** | Repeated repairs reuse the same solution framing across a long session | What would a fresh independent objective-first analysis propose? | Hypothesis, not yet measured |
| **Requirement provenance loss** | An inferred implementation preference is treated as an immutable product obligation | Who accepted the original constraint and why? | A, C |
| **Evidence inflation** | Smoke, static configuration or source inspection is promoted to live qualification | Which level of proof was actually achieved? | C, D |
| **Over-review / meta-loop** | Repeated perspectives are invoked without a decision-relevant new fact | Should the system stop reviewing and wait for the owner or evidence? | Important negative hypothesis |

There should be an explicit **unknown / no new perspective warranted** class. If every blocker receives a retrospective perspective label, the taxonomy cannot discriminate useful intervention from normal work.

## 6. Evidence triggers: versioned predicates, not agent self-assessment

A trigger is a **closed, research-qualified specification of evidence conditions** evaluated by the Work Engine control plane. “The agent seems stuck” is not a trigger: it does not identify admissible evidence, a falsifiable predicate, or a reproducible evaluation procedure. The acting Builder, reviewer or orchestrated model may produce ordinary findings, but it has no standing to select a new perspective by judging its own progress. Until deterministic predicates are built, the inference-phase evaluator (Stage 2a) applies these same specifications and produces the same records.

### 6.1 Candidate signatures from the seed corpus

| Candidate trigger | Pre-intervention evidence needed | Perspective × intent to test | Major false-positive / false-negative risk |
|---|---|---|---|
| **Repeated blocker identity** | At least two revisions/reviews reject the same pinned obligation or finding lineage | Contract × challenge necessity; integration × seek an alternative | A binding hard guarantee can genuinely require repeated repair; finding IDs may conceal semantic duplicates |
| **Mechanism proliferation** | New locks, state machines, services or protocols attributed to one obligation across revisions | Simplification × find minimum causal mechanism | Necessary complexity for an enforceable security boundary; the counting rule may miss subtler growth |
| **Infrastructure escalation** | Proposed daemon/runtime/platform not in accepted plan; stated reason and missing property | Architecture × test causal necessity | A new runtime may actually be required for exclusive control |
| **Evidence/claim mismatch** | Outcome claim cites only producer action or self-reported effect, with the applicable assurance contract | Evidence × narrow what is established | Producer acknowledgment may be sufficient for a deliberately weaker test |
| **Observation ordering hazard** | Required observation point, lifecycle ordering evidence and a potentially destructive step | Lifecycle × seek a safe observation interval | An already prearmed trustworthy observer may cover the interval |
| **Premature protected access** | Identified open/mount/copy/send precedes an effective admission boundary | Dataflow × locate the first protected touch | Earlier access might be expressly authorized and not a violation |
| **Cross-component assumption mismatch** | Pinned interfaces disagree on identity, phase or permissible effect | Integration × trace the whole obligation | An intended narrowing may look like an inconsistency |
| **Persistent infrastructure requests** | Multiple proposed facilities for a stable missing capability, without a newly evidenced discriminator | Causal sufficiency × reconnect mechanism to objective | A real missing facility may remain indispensable |
| **Long-lived repair trajectory** | Elapsed time/cost, attempts and accepted-evidence delta exceed rule thresholds | Scope/premise × test the framing | A genuinely difficult task can have legitimate duration without scope error |
| **Unsettled asynchronous work** | Start/stop ambiguity; unresolved accepted operation; claim of finality from disappearance | Temporal/evidence × distinguish outstanding work from cleared execution | Incorrectly requiring whole-daemon quiescence can overblock normal operations |

Every row is **a hypothesis, not an installed rule**. The seed cases are selected for memorable interventions and are insufficient to establish useful trigger thresholds. Controls must include identical or similar evidence patterns where no new perspective is justified.

### 6.2 Trigger contract — minimum required fields

A candidate trigger should have a versioned, auditable contract containing:

| Field | Requirement |
|---|---|
| **Rule identity** | Immutable ID, revision, provenance, owner adoption, research qualification status |
| **Scope/applicability** | Task family, component boundary, objective/contract revision, supported software/model versions if relevant |
| **Inputs and evidence coverage** | Typed event kinds, source/claim refs, bounded time or event-order window, expected completeness, freshness |
| **Deterministic predicate** | Explicit conditions over admissible typed values, including the handling of competing evidence |
| **Semantic fallback permission** | Specific ambiguous field(s), exact permitted evidence sources, structured output types, bounds and abstention behavior |
| **Exclusions** | Known negative cases where the predicate should be false (evaluator) |
| **Dispatch suppressions** | Accepted owner decisions, equivalent open findings, cooldown/attempt limits, budgets (dispatch policy, not evaluator) |
| **Perspective mapping** | Qualified perspective, intent, abstraction scope and context policy, not model-selected at execution time |
| **Effect** | May nominate bounded advisory analysis; cannot mint grants, release protected inputs, modify contracts or bypass gates |
| **Audit result** | Exact rule and evidence snapshot digests, typed inputs, classifier provenance if used, evaluation outcome, dispatch decision and reason |

These are conceptual requirements, not a claim that an existing Work Engine schema already provides them. Express them in the planned Rust event, claim, register and decision shapes, not a research-only schema, so that inference-phase records, deterministic evaluation and the eventual adopted policy share one representation. Where the Rust design does not yet define a needed shape, record that as a requirement on it rather than inventing a parallel one.

### 6.3 Deterministic evaluation and missing-evidence semantics

The evaluator applies the pinned specification to a frozen, attributed evidence snapshot. It answers only the evidence question:

| Evaluation | Operational meaning |
|---|---|
| **`fired`** | All required admissible facts are established, the predicate matches, and no exclusion applies; the *separate dispatch policy* may consider an advisory review |
| **`not_fired`** | The evidence coverage is adequate and the predicate is false (or an exclusion applies); do not schedule this perspective |
| **`unknown`** | Coverage missing, evidence stale/crossed, source order unresolved, classification inconclusive or contradictory; do not silently assume true or false |

The dispatch policy acts only on `fired`:

| Dispatch decision | Operational meaning |
|---|---|
| **`dispatched`** | Qualification, applicability, budget and authority permit one bounded advisory review under the mapped perspective |
| **`suppressed(reason)`** | An open equivalent finding, accepted owner disposition, cooldown, owner restriction or budget blocks dispatch; the reason is recorded |

Budget and cooldown are not evidence. Keeping them out of the evaluation lets trigger quality be measured independently of dispatch policy.

A missing event cannot support `not_fired` unless the evidence stream has the coverage required by the spec. `fired` does **not** mean the acting model is deficient or the alternative is correct: it means the evidence satisfies a candidate intervention rule. Status and reasons are retained even when no review runs.

Trigger evaluation should be replayable: same spec, same typed evidence snapshot, same admissible classifications and same policy revision yield the same disposition. If a semantic classification was required, pin its **output and provenance** before running deterministic evaluation; do not claim that rerunning a model necessarily produces identical text.

### 6.4 Bounded semantic fallback

Where structured data is adequate and a deterministic predicate has been built, **do not call a model**. Until then, the inference-phase evaluator (Stage 2a) resolves every premise under these same bounds. If a required primitive is ambiguous in natural-language evidence, a classifier may run under a *separately specified* small evidence packet and return only allowed enum/boolean categories, cited spans, and an explicit uncertainty or `unknown`. For example, a proposal may mention a second daemon: classification might identify whether it was proposed to supply isolation, observation, or recovery control. The classifier is not allowed to conclude that the daemon is necessary or to choose a new orchestration rule.

The fallback must have a strict input/output contract, model/effort and cost bounds, prohibited side effects, and a validation step. It cannot modify trigger definitions or perspective mappings, manufacture source/order/runtime evidence, reinterpret owner-approved obligations, or issue dispatch/authorization commands. Unsupported or inconsistent output becomes `unknown` or a separately controlled evidence-resolution task. Semantic outputs are **interpretations with references**, not independent process attestations or verified source facts.

Two research comparisons are required: deterministic-only extraction/evaluation and deterministic evaluation **plus** bounded semantic classification for declared gaps. Measure additional recall, false positives, abstention rate and classification cost; fallback should not be assumed helpful.

Nearly every candidate signature in §6.1, and every pilot in the §11A feasibility table, depends on at least one classified field. Classifier accuracy is therefore a **primary** determinant of trigger quality, and results must report it per field. A rule whose accuracy rests mostly on classification should be described as classified-then-combined, not as mechanical detection.

### 6.5 Example: source-observation ordering (P4)

An illustrative candidate rule, **not implemented or qualified**, might test the conjunction:

1. An accepted contract identifies a process state that must be observed before a downstream decision.
2. An attributed, source-pinned lifecycle record establishes the launch/start edge and an effect that can terminate or remove the process.
3. The selected implementation permits that terminating effect **before** a separate observer is offered a bounded opportunity to examine the same lifetime.
4. No admissible evidence of a prearmed, appropriately independent observer covering that lifetime is present *and coverage is sufficient to make that absence meaningful*.

An already-open equivalent decision or prior suppression is **not** part of the predicate. A real hazard still evaluates as `fired`; the dispatch policy then records `suppressed(reason)`.

When typed source-order edges and observation coverage are available, the conjunction is deterministically evaluable. If one edge's *meaning* is ambiguous, a specified semantic classifier can propose its type; if the ordering itself cannot be established, evaluation is `unknown`. The rule does not assume static program analysis can automatically discover every relevant execution path or prove runtime effects. That is a separate validation problem.

On `fired`, a qualified policy could nominate a **lifecycle-observability** perspective with the intent to find whether the observation opportunity can be moved before cleanup while preserving the actual security requirements. The reviewing model may propose an alternative but cannot adopt it, execute Docker, or declare qualification. A passing review must still address same-process executable identity, post-init effective state, uncertain start and terminal settlement.

### 6.6 Two different products of the research

**Trigger quality** asks: *Could the system have fired at the actual decision point from evidence already available, with acceptable false positives, missed opportunities, ambiguity and cost?*

**Perspective effectiveness** asks: *Once that trigger is selected, does the mapped intent, scope and context make an independently useful and safe finding more likely than control conditions?*

**Mapping and scheduling policy** asks a third question: *Even if both are individually good, does their combination outperform ordinary work after costs, overlap, and suppressed interventions?*

This separation is essential. A retrospectively insightful perspective does not validate its trigger. A flawlessly evaluated trigger does not validate its perspective mapping.

### 6.7 When detection runs: blocker gate and deterministic canary

Detection is not run on every turn through inference. Two gates are in scope:

- **Blocker gate (primary).** Inference-phase detection runs once per blocker event, deduplicated by finding lineage. A blocker is a typed event, not the agent's mood: a blocking review finding, a failed gate or acceptance check, a guard refusal, an explicit blocked or needs-decision report, or recurrence of the same finding lineage. The evaluation's subject includes the agent's **proposed remedy** for the blocker, since that remedy is what most often extends the unproductive framing. Holding the remedy until detection has run is an **experimental arm, not a general gate**. When tested, the hold has a bounded deadline. If detection times out, is unavailable, or returns `unknown`, work proceeds and the reason is recorded. Measure the latency it adds against what it prevents. Existing security and authorization gates are unaffected; advisory perspective detection never becomes a hard acceptance gate by default.
- **Deterministic canary (after deterministic predicates exist).** Fully deterministic predicates may run every turn with no inference cost. Examples: a new component, service or daemon in a plan or diff; a new lock or state machine attributed to an existing obligation; a destructive step placed before a required observation; access to a protected path. A canary hit may open inference-phase detection exactly as a blocker does. It dispatches an intervention directly only when its full predicate is deterministic and qualified. Before deterministic predicates exist there is no canary, and detection is blocker-only.

**Gate coverage is itself a result.** The seed cases all came to a head at blockers, but they were chosen because an intervention happened, and interventions happen at blockers. Some drift, such as accumulating mechanisms or escalating infrastructure, may never surface as a blocker. Estimate the gate's missed opportunities by auditing samples of work that hit no blocker, and report coverage limits with every precision/recall figure.

## 7. Candidate interventions and how they differ

The research should explore more than a role catalog. Useful interventions might include:

- **Scope expansion:** step from function to component, component to cross-system relationship, or architecture to original objective.
- **Scope contraction:** stop abstract debate and demand the first concrete source-level falsifier.
- **Intent inversion:** from implementing a contract to testing whether an assumption behind it is necessary.
- **Temporal reframing:** ask who acts first, who can observe when, and when effects become irreversible.
- **Causal responsibility split:** distinguish request, effect, observation, and checked conclusion.
- **Provenance inquiry:** trace which constraints are owner-accepted, derived, inferred or merely historical.
- **Alternative-mechanism search:** ask for the simplest mechanism that satisfies the same invariant, including doing no additional work.
- **Independent first-pass:** provide a fresh session with the objective, obligations and evidence, withholding the incumbent solution until it has offered its own decomposition.
- **Adversarial revalidation:** expose the proposed simplification to the full rejected history, binding constraints and negative tests before owner disposition.
- **Perspective steer:** a short, versioned message delivered into the working session: perspective and intent, the binding obligations, a statement that any contract change is a proposal for the owner, and permission to continue if no material alternative appears. It does not state an answer. It is delivered at a turn boundary, which at a blocker usually comes before the next expensive or irreversible step; a steer cannot undo a tool effect that has already happened.
- **Steer, then escalate:** if the blocker persists or recurs after a steer, the dispatch policy, not the agent, may dispatch a fresh review whose first pass does not see the agent's remedy.
- **No-intervention disposition:** the deterministic policy records that no qualified rule justifies an additional perspective; ordinary work continues or the existing owner awaits missing evidence.

An intervention's **output** should be a bounded, checkable proposal: the original premise, supporting or contrary evidence, new question, alternatives, affected obligations, exact authority needed, and smallest discriminating test. The reviewer should not silently mutate source, initiate protected work, consume grants or approve itself.

## 8. Proposed control-plane model — fixed mechanics, bounded judgment

The desired architecture is a governed evaluator, **not an agent that periodically inspects itself and decides to change intent**:

```text
Revisioned engineering events, source/evidence and owner contracts
                             |
                             v
    Detection gate: blocker event | deterministic canary hit
          (no gate, no evaluation; canary only once
           deterministic predicates exist)
                             |
                             v
               Normalize/validate typed evidence
                             |
                             v
       Deterministic trigger predicate and coverage check
                             | \
                             |  \-- named ambiguous field only
                             |          |
                             |          v
                             |   Bounded semantic classifier
                             |   (typed value + source span
                             |    + uncertainty/abstain)
                             |          |
                             |<---------/
                             v
       Typed evaluation: fired | not_fired | unknown
                             |
                             v   fired only
        Deterministic applicability, mapping, duplication,
           budget and authorization-of-advisory-work policy
                             |
             dispatched | suppressed(reason)
                             |
                    dispatched only
                             v
         Delivery: steer into working session | fresh review
           (versioned per mapping; steer may escalate to
            fresh review on persistent blocker evidence)
                             |
                             v
       Steered session continues | fresh Pi inquiry
            (fixed perspective + intent + scope + context)
                             |
                             v
             Independent evidence/contract adjudication
                             |
                             v
                   Existing owner disposition
                             |
                             v
       Measured outcomes -> versioned research evidence
         (rule revisions require independent adoption)
```

This is a **research target**, not installed Work Engine machinery. Determinism applies to rule evaluation, policy choices and admitted execution mechanics. Model judgment is limited to defined interpretation of ambiguous data and the subsequent bounded inquiry. A model's interpretation is recorded and examined, not treated as a source of new authority.

The control plane also owns **when not to act**: insufficient event coverage yields `unknown`; a rule with adequate negative evidence yields `not_fired`; an already-adjudicated finding yields `suppressed(reason)` at dispatch. Only the explicitly permitted policy can send a new perspective session, and that session cannot request its own recursive invocation. Any new rule proposed by research is offline until reviewed, qualified and adopted as a revised deterministic policy.

A candidate **trigger-to-perspective relation** must be versioned and limited in applicability; do not assume one trigger maps to one unique or universally correct perspective. Research can compare alternatives while the live policy keeps a chosen, auditable mapping. Context packet construction should be reproducible from references and a versioned rule, without relying on the incumbent Builder to summarize its own blind spots.

## 9. Research corpus and observability requirements

Work Engine's normal engineering evidence should provide much of the raw material. The research should minimize new instrumentation and make its limits explicit.

### 9.1 One event record

A research observation should identify:

- Stable event and attempt references; event time and ordering;
- Current approved objective and referenced contract revisions;
- Task scope, source revision, changed files, and review finding identities;
- The original blocker and the concrete observations supporting it;
- Prior candidate mechanisms and known falsifiers, each with provenance;
- Existing session/model/effort/context state, when available and relevant;
- Every evaluated trigger ID/version, predicate inputs, coverage result, evaluation (`fired`/`not_fired`/`unknown`), dispatch decision (`dispatched`/`suppressed(reason)`) and evidence snapshot digest;
- Fallback invocation or abstention, exact classified field, cited spans and interpreter provenance, without promoting it to independently checked fact;
- Perspective/intent/context packet assigned (if an intervention occurs);
- Model response, independent check, owner disposition and final qualification stage;
- Measured cost and process effects, with estimates explicitly separated.

Capture only necessary data and protect private prompts, credentials, provider metadata, protected task bytes and local identities. Research artifacts should use stable pointers and sanitized summaries where sufficient. Cross-session provenance must be traceable without disclosing secrets or creating an uncontrolled transcript archive.

### 9.2 Annotation rules

Research labels should distinguish at least: **confirmed source-level defect; plausible design alternative; accepted owner decision; code implemented; independently qualified; rejected; unknown**. Multiple independent annotators should mark the same seed cases where feasible and record disagreements. A label such as *unnecessary requirement* is a judgment requiring provenance, not a fact inferred from a simpler patch.

### 9.3 Negative evidence is valuable

The corpus must preserve failed perspective reviews, reframe proposals that violated contracts, triggers that fired too late, redundant analyses, and cases where continuing the original task was the correct choice. Without these records, both offline rule discovery and later deterministic trigger qualification would overfit to a misleading sample of memorable successes.

## 10. Proposed experimental program

### Stage 0 — Preserve cases and construct a classification rubric

**Question:** Can independent reviewers characterize these events, pre-intervention evidence availability, and candidate trigger primitives without hindsight-driven certainty?

Produce exact evidence packages for Cases A–E: the initial objective and obligations, the decision point *before* the operator intervention, event/source/evidence snapshots available at that time, subsequent observed events, and the current qualification level. Draft versioned trigger predicates with explicit coverage and `unknown` handling alongside perspective/intent labels and invalid proposal cases. Label additional ordinary blockers, no-trigger examples, missing-evidence cases and situations demanding fidelity to a non-negotiable requirement. Do not leak later operator decisions into a classifier or replay packet.

Annotators who convert cut-time prose into typed events should be blind to the later intervention where feasible. This annotation step is where hindsight most easily leaks into trigger evidence.

**Deliverable:** A small, evidence-pinned seed corpus plus a rubric with recorded ambiguity and disagreements. No implementation requirement on A11, and no dependency on the Rust implementation.

### Stage 1 — Controlled retrospective replay

**Question:** Does a changed perspective causally help, or was the gain mainly a reset, extra context, more inference, or model variation?

For each eligible decision point, hold the source snapshot, authorized obligations, model and reasoning effort, evaluation budget, and tool-access rules fixed. Test a compact set of conditions:

1. **Continuation:** same task framing / continuing session, as supported by a reproducible replay;
2. **Fresh generic review:** fresh context, conventional review intent;
3. **Targeted reframing in the continuing context:** same session, explicit new perspective/intent;
4. **Fresh targeted perspective review:** new session, original objective and evidence first;
5. **Wrong-perspective review:** new session, same packet, a deliberately mismatched lens (for example authority/custody on P4). This measures whether the *specific* perspective matters or any reframing helps;
6. **All-lenses checklist review:** new session, objective-first packet, with the full perspective catalog (research note §4) offered as questions to consider and no single lens assigned.

Conditions 5 and 6 validate the **trigger-to-perspective mapping**, a component of the planned design. If a mismatched lens or the unassigned catalog performs as well as the mapped perspective, the mapping step is not earning its place and should be simplified before mappings are qualified.

The historical sessions no longer exist, so condition 1 is a reconstruction, and token budgets between a long continuing context and a fresh one cannot be matched exactly. Treat condition 1 as a weak proxy from the outset, label it as such, and report the actual context and token sizes rather than claiming matched budgets. Randomize packet order/assignment where possible; repeat under multiple controlled seeds and, eventually, models. Blind the evaluator to the intervention identity and require review against all obligations before scoring. Add a **no-review** control whenever the trigger's expected value is being assessed.

**Primary outcome:** Does the response identify a genuinely decision-relevant alternative or falsifier that survives an independent contract/evidence check? Score against obligation preservation and independent checkability, **not** similarity to the historical fix. The historical answer is one reference solution; a different valid solution counts as a positive. **Secondary outcomes:** token cost, latency, new invalid assumptions, and number of additional iterations. Do not infer dollar savings from unimplemented historical proposals.

### Stage 1b — Steer replay

**Question:** Can a short steer into the working session do what a fresh review does, and over what horizon?

A steer needs a *running agent with tools*, resumed from the cut point and observed over several turns, not a single review response. Reconstruct the working session at the blocker. Use an archived transcript prefix where one exists, otherwise a cut-time reconstruction labeled as a proxy. Give the agent read-only source access at the pinned revision and a bounded turn budget, with no Docker, protected-task or other external effects. Compare five arms from the same resumed state:

1. **No steer:** continue unchanged;
2. **Neutral steer:** a content-free interruption of the same length, testing whether any message refocuses the agent;
3. **Targeted steer:** the mapped perspective and intent, without the answer;
4. **Wrong-perspective steer:** a mismatched lens of the same form;
5. **Fresh targeted review with handoff:** a fresh session runs the Stage 1 condition 4 review. Its recommendation then reaches the resumed incumbent through the authorized handoff, a message into the session, and work continues under the same turn budget. The review's own inference and latency count toward this arm's cost. A review whose advice never enters the work would understate fresh review, so it is not used for this comparison.

Arms 3 and 5 both end with a message into the same session. They differ in what the message carries: a perspective question, or independent findings produced in a fresh context. That difference is what RQ10 measures.

Record four timing events in every arm: **blocker observed**; **intervention queued and actually delivered** (a steer arrives only after the current turn's tool calls finish); **first consequential remedy action begins**; and **first decision attributable to the steered or handed-off continuation**. They separate an intervention that arrived before the opportunity from one that arrived after the agent had already committed to its remedy. Late delivery is reported as such, not as steer failure.

Observe at three horizons: the **immediate** response, **later decisions** within the turn budget, and the **end** state. Record whether a decision-relevant alternative appears and when; whether the agent's proposed remedy changes; whether binding obligations are preserved; tokens added, including the steer carried through later turns; and whether the steer survives compaction. A delayed effect counts only against the matched no-steer and neutral-steer arms, never by itself.

### Stage 2 — Trigger qualification in shadow mode, inference first

**Question:** Can a declared trigger be evaluated *at the relevant time*, without asking the acting model to decide that another perspective is necessary? Which of its premises should become deterministic?

All phases are **non-dispatching** and produce the same record: per-premise values with cited evidence, the evaluation (`fired`, `not_fired` or `unknown`), and the hypothetical dispatch decision (`would_dispatch` or `would_suppress(reason)`), with reasons and coverage. The policy vocabulary is unchanged; the `would_` form marks results from shadow runs so they cannot be mistaken for actual dispatches. Records use the planned Rust shapes (§6.2).

*Stage 2a — Inference-phase evaluation (no Rust dependency).* An evaluator session, independent of the acting agent and given no authority, receives a frozen snapshot and the versioned trigger specification. For each premise it returns a typed value with cited spans or abstains; a deterministic combination of those values yields the evaluation. Evaluating **per premise**, not as one overall judgment, is what lets later rounds move premises into deterministic extraction one at a time. Pin every output as research evidence.
- *Retrospective:* on the seed corpus, establish **evaluability and per-premise agreement** with independent adjudication. The trigger should fire on its seed case, return `not_fired` on constructed negatives, and return `unknown` on constructed missing-coverage cases. Five hand-picked positives cannot support precision or recall.
- *Prospective:* where authorized, run the same evaluator in shadow over ordinary ongoing work, using the artifacts that already exist (commits, review findings, board records). Ordinary negatives occur naturally there, and first rates become available, with their coverage limits stated.

*Stage 2b — Deterministic nomination.* From the 2a records, classify each premise. **Deterministic candidate:** resolved reliably, from structured evidence that exists or that Rust can capture. **Declared fallback field:** needs interpretation of meaning. **Unresolved:** the evidence is insufficient either way. Deterministic candidates and the evidence they need become concrete requirements on the Rust implementation (§14.1). This output is the main input to building the deterministic mechanism.

*Stage 2c — Deterministic evaluation with semantic fallback.* Once the §14.1 prerequisites exist, run the deterministic evaluator, with bounded semantic fallback for the declared fields, over live typed events. Replay the frozen 2a snapshots as well: for each premise moved to deterministic extraction, confirm it agrees with adjudicated outcomes at least as well as the inference phase did. Investigate any regression before the inference premise is retired.

Report, across the prospective phases, each trigger's first eligible firing, event coverage, precision/recall against independently adjudicated opportunities, false positive and missed-opportunity costs, duplicate/suppression behavior, semantic-fallback utilization and cost, classifier validity, unknown/abstention rate, and time-to-detection. Explicitly compare an early trigger that fires correctly against a late trigger that merely describes a failure after hours of work. Do not allow hindsight evidence into the event snapshot.

**Deliverable:** A versioned, independently inspectable trigger specification with positive, negative, unknown, suppression and fallback cases; per-premise inference-phase records; the deterministic-nomination classification; and empirical performance as each phase becomes available. Shadow firings do not schedule reviewers or alter normal engineering.

### Stage 3 — Bounded prospective interventions

**Question:** Does the outcome benefit survive without hindsight when a **qualified deterministic policy** decides whether and when an additional perspective is requested?

**Steers may enter live trials earlier than fresh reviews.** A steer is short, advisory and cheap, so small blocker-gated steer trials on ordinary work may begin once Stage 1b shows that steers are not harmful and the inference-phase evaluator (Stage 2a) is available. They run under normal spending authorization, with a per-session steer budget, a matched no-steer control, and the same stop conditions. Fresh-review dispatch keeps the full qualification bar below.

Only after separate evidence supports trigger quality and perspective effectiveness, test a small owner-adopted mapping on ordinary eligible engineering. Compare policy-directed bounded perspective reviews against ordinary continuation, no-extra-review, generic review, the all-lenses checklist review, and, where informative, always-review. Randomize or otherwise control eligible opportunities and report exclusions. The acting Builder cannot call its own trigger or alter the mapping. A permitted fresh reviewer provides decision support; the existing owner accepts, rejects or parks it. Normal build, review and gate requirements remain unchanged.

Stop on contract erosion, unsupported evidence claims, repeated dispatch without new facts, cost overrun, missing independent custody, or a rule that fires after its opportunity has passed. Include total wall-clock and inference/coordination expenses, not just the price of the added review.

**Deliverable:** Prospective comparisons of verified outcome quality and net work, including harm, negative results, duplicate suppression and uncertainty.

### Stage 4 — Research-qualified deterministic trigger and mapping policy

Only if earlier stages show meaningful benefit, nominate a minimal **versioned deterministic rule set** with observed applicability, evidence coverage, explicit semantic-fallback permissions, `unknown` and suppression semantics, budget/cooldown limits, and separately validated trigger-to-perspective mappings. Compare the adopted configuration with simpler policies, the all-lenses checklist, and no-extra-review controls. Where a rule depends heavily on fallback fields, record that and prefer moving those fields into deterministic extraction as structured evidence becomes available.

A model-authored runtime selector is not part of the design. Models aid *offline rule discovery* and the *bounded semantic fallback*. The acting agent never selects its own perspective review, and changing a live rule or mapping requires explicit independent qualification and owner adoption.

### Stage 5 — Governed research updates and transfer

Evaluate whether new research can **propose** revised trigger definitions, evidence primitives, perspective categories, context policies and applicability. Those proposals enter the register and must be independently reviewed, versioned, and accepted before the control plane can use them. No runtime self-modification or autonomous expansion of authority follows from model observations. Test transfer to unseen codebases, software versions and model families, including deterioration that should retire or narrow a rule.

## 11. The first concrete experiment

**Candidate:** The fixed-init observation race (Case D), with the exact **pre-intervention** source and decision point frozen. Evaluate both the lifecycle perspective and the **candidate deterministic temporal-order trigger**; do not substitute success of one for success of the other.

**Why:** There is a source-observable ordering error: the Host proceeds from Docker `Running` to cleanup without giving a separate observer an opportunity. The historical discussion contains competing mechanisms (kernel-wide prearm versus a bounded observation hold). The case tests whether an explicit *lifecycle and observability* perspective changes what an agent proposes.

**Experimental questions:** (a) At what pre-intervention evidence snapshot could a typed ordering predicate have first fired, and which required facts were known, absent or ambiguous? This is an evaluability question for the inference-phase evaluator (Stage 2a), not a precision/recall estimate. Its per-premise output also indicates whether the ordering premises could be extracted deterministically. (b) Under matched model, source/evidence constraints and reported token use, is a targeted lifecycle reviewer more likely than continuation, generic review, a wrong-perspective review, or an all-lenses checklist review to identify the missing pre-cleanup observation opportunity and propose a safe bounded hold **without falsely claiming that readiness or Docker inspect independently proves the post-init state**?

**Validity checks:** A proposal passes only if it preserves single-start/no-replay, bounds the hold, handles early exit/timeouts/uncertainty, distinguishes synchronization from independent observation, and does not promote cleanup success into settlement proof. Evaluators should also identify incomplete positive evidence: observing UID/NNP alone does not necessarily prove that the init completed its own checks. A proposal that simply adds an arbitrary sleep is not automatically equivalent to a verified observation protocol.

**Trigger controls:** A negative ordering example with a genuinely prearmed trustworthy observer should **not** fire; a case with incomplete source-order or observation-coverage evidence should return `unknown`, not `not_fired`. An identical matched finding with an already accepted owner disposition should be suppressed. A separate mandatory-boundary case (for example, an early protected task read) tests whether the perspective review irresponsibly weakens security.

**Leakage caveat:** the P4 perspective assignment (§11A) names launch, observation and cleanup ordering, which is close to stating the answer. A positive result from condition 4 alone therefore shows mostly that asking the right question yields the answer. The informative comparisons are against the wrong-perspective and all-lenses checklist arms, and the question of whether anything could have selected that lens at the time (Stage 2).

**Result classification:** design-relevant proposal, correct falsifier, incorrect simplification, redundant finding, or abstention. No live Docker test is part of this first replay.

## 11A. Seed pilots extrapolated from the existing five cases

The current evidence is sufficient to nominate **initial perspectives, intents, scope boundaries, and trigger signatures**. It is **not** sufficient to infer how frequently those perspectives succeed or whether research-qualified deterministic trigger selection would improve engineering. To avoid confusing discovery with validation, **the five case-specific pilots (P1–P5) and one cross-cutting scope/intent pilot (P6)** begin with a researcher-selected perspective (an *oracle-selected* intervention) and test whether it helps under controlled replay. Trigger detection is evaluated separately against pre-intervention records in Stage 2. A researcher selecting a perspective for a pilot does not count as automatic trigger selection.

The proposed perspectives are deliberately functional descriptions, not agent personas or mutually exclusive taxonomic classes. A single useful intervention may combine two lenses; the analysis must record that rather than force one label.

| Pilot | Pre-intervention evidence signature (candidate trigger) | Perspective / scope | Intent: the question to ask | What counts as a useful finding |
|---|---|---|---|---|
| **P1 — Commitment semantics** (A) | Repeated attempts to recheck a moving Campaign head; conflict between clean refusal and earlier filesystem effects | **Authority-and-commitment lifecycle**, Campaign → Server → Host | Challenge whether continuing revocability is essential; identify the authorized point of no return | Proposes a one-shot, owner-consumed commitment *before first effect*, clean pre-admission refusal, post-admission abort/reconciliation, and explicitly preserves atomic ordering and owner approval |
| **P2 — Outcome vs. action** (B) | Cleanup or kill acknowledgment is asked to stand in for proof of terminal state; outcome depends on detached execution | **Causal/evidentiary responsibility**, killer → runtime → independent verifier | Separate the action that changes state from the witness that establishes the result | Distinguishes termination request, independent attempt-scoped absence/settlement, no delayed entry and permanent fencing; does not claim an acknowledgment or empty listing proves everything |
| **P3 — Mechanism necessity** (C) | A second daemon or administrative runtime is proposed while the missing evidence concerns process identity and post-reduction state | **Architectural causal sufficiency**, requirements → mechanisms → trust boundary | Demand the exact property a new component supplies and ask whether the current arrangement can supply it | Retains one daemon unless a demonstrable authority conflict calls for another; does not pretend one daemon already supplies trusted observations |
| **P4 — Observability ordering** (D) | Source sequence starts, inspects `Running`, and removes a short-lived fixed init, while a later proof requires observing that init | **Temporal/lifecycle design**, Host launch → process checkpoint → observer → cleanup | Find whether the procedure itself destroys the needed evidence and identify a safer ordering | Offers a bounded observer opportunity before ordinary cleanup; still requires actual executable/process/post-init evidence, timeout/early-exit handling and terminal settlement |
| **P5 — First protected touch** (E) | A proposed hold is selected *after* Server/Host ordinary validation opens the protected task | **End-to-end access/dataflow**, Server → Host → receiver | Trace the first place task bytes or locators cross a trust boundary, not merely the final send/mount | Proposes an owner-bound metadata-only held input selected before ordinary validation on both sides; preserves independent authorization, never treats caller digest as authoritative and leaves actual delivery proof separate |

### P1 — Commitment semantics: distinguish a negotiable contract from a missing lock

**Frozen decision point.** Present the accepted objective, the existing Campaign/Server/Host order, the clean-refusal requirement, and the two failed SQL recheck routes *up to but not including* the operator's one-shot admission intervention. Do not supply the operator's resulting formulation as a hint.

**Perspective assignment.** “Examine the meaning and temporal scope of the authorization obligation. Distinguish owner-accepted requirements from implementation interpretations; identify whether any owner decision could reduce mechanism count without allowing unauthorized effects. Do not change the contract yourself.”

**Target observation.** An independently checkable alternative that distinguishes pre-admission revocation from post-admission abort, identifies an atomic first-effect commitment, and retains uncertainty and no-replay constraints. An agent merely proposing a longer lock without explaining commitment provenance is **not** a positive result. An agent declaring revocation irrelevant or allowing writes before atomic consumption fails the safety check.

**Adversarial companion.** Give a separate task where an owner explicitly and non-negotiably requires revocation to remain effective until a specified later point. The perspective must not “simplify” that accepted requirement away; it should instead identify an implementation blocker or request an authorized contract revision.

### P2 — Outcome versus action: distinguish doing from knowing

**Frozen decision point.** Provide the live/detached collector arrangement, the pending-kill problem and the stated termination requirement, but withhold the operator's “killer acts, verifier proves gone” formulation.

**Perspective assignment.** “For each claim, name the actor that requests the effect, the actor able to observe it, and which independent evidence would justify the claim. Identify any self-attestation and the smallest attempt-scoped terminal property.”

**Target observation.** The model must avoid equating process removal success, Host cleanup reports, or one absent PID with actual terminal settlement. It should not require all Docker helper processes to disappear if their continued presence is demonstrably irrelevant to the admitted attempt. It must account for already-accepted starts and prevent replacement entry while unknown.

**Adversarial companion.** Present a normal low-assurance cleanup smoke test whose contract only asks whether Docker returned success. The reviewer should avoid demanding independent kernel-level proof for a claim that was never made.

### P3 — Mechanism necessity: challenge causal fit, not protection itself

**Frozen decision point.** Supply the source-level shim/cleanup counterexample, Astra's conditional private-runtime recommendation, and the current qualification objective; omit Sol's later retraction.

**Perspective assignment.** “Explain which required property each proposed new daemon/service/runtime would establish, and which properties it cannot. Compare a minimal existing-runtime path before recommending topology change. Preserve the real authorization, isolation and observation obligations.”

**Target observation.** An evidence-based decomposition into **control**, **observation**, and **settlement**, with a concrete discriminating test for any claim that the existing daemon cannot satisfy the required control boundary. A blanket “Docker is fine” dismissal fails, as does recommending a second daemon merely because it sounds more isolated.

**Adversarial companion.** Supply an independently verified case where the shared daemon is administrable by the actual untrusted Builder and no effective exclusion is available. The model should accept that separation may require a different administrative boundary instead of insisting on one daemon.

### P4 — Observability ordering: source-level, high-confidence first pilot

**Frozen decision point.** Pin `pi_public_hold.rs` and the fixed-init source at `3693e48c`; provide the objective requiring independent evidence of the post-reduction state. Present the concern about the process disappearing, but withhold the subsequent proposed observation hold.

**Perspective assignment.** “Trace the time order of launch, security transition, observation and cleanup. Determine whether the existing procedure makes an essential observation difficult or impossible. Suggest the smallest safe sequencing change, or show why no such change is available.”

**Leakage note.** This assignment names the ordering at issue and leans toward the answer; see the caveat in §11. Interpret it only against the wrong-perspective and checklist arms.

**Target observation.** Recognizes that `Running` is followed by cleanup without an outside-observer turn. Proposes a **bounded** observation opportunity while the process is still alive, with no task ingress or indefinite hold. Distinguishes the init's own readiness signal from independent proof, and understands that UID/capability observations may precede the init's internal completion. A sleep-only proposal, or a claim that the revised flow is already qualified, fails.

**Adversarial companion.** Present a workload whose security contract prohibits keeping the process alive after a fixed deadline or does not permit a pause. The reviewer must not assume every lifecycle can be held; an event-recorder or explicit impossibility may then be the appropriate answer.

### P5 — First protected touch: verify that simplification preserves a hard boundary

**Frozen decision point.** Provide the Server and Host validation call order, the intended held-only behavior, and the authorized rule that protected task bytes cannot be opened before the proper boundary. Withhold the later owner-bound metadata-only proposal.

**Perspective assignment.** “Trace the earliest read, materialization, mount or transmission of the protected task and its source locator across all relevant components. Ask whether the named hold is upstream of *every* prohibited access. Preserve the owner who establishes the task identity/digest association.”

**Target observation.** Detects the early validation read in both Server and Host before the hold; proposes a closed, path-free metadata selection *before* ordinary validation while leaving ordinary jobs unchanged and positive receiver-delivery evidence separate. Merely removing a snapshot, moving a flag, or trusting caller-supplied digest/size is an invalid shortcut.

**Adversarial companion.** Include a scenario where the task is explicitly public and ordinary validation is authorized; the reviewer must not introduce unnecessary held-only machinery.

### P6 — Cross-level scope pilot, using the same evidence

The operator's observation about line, function, module and system-level intent warrants a **separate cross-cutting test**. On a frozen source slice from P4 or P5, assign the *same intent* but change what the reviewer can see:

1. **Local:** one function and its direct contract;
2. **Component:** its module plus adjacent call sites;
3. **Cross-component:** the actual Server/Host/runtime/evidence sequence;
4. **Objective-first:** the original product/assurance objective plus the same cross-component source.

Evaluate whether the correct issue first becomes discoverable at a particular scope, rather than concluding that a larger context is always better. Record added tokens, irrelevant findings, and whether a larger context decreases rather than increases focus. A complementary version holds scope fixed but changes **intent** (implement, defect review, premise challenge, lifecycle review, evidence-sufficiency check), enabling a partial separation of the two dimensions.

### Steer pilots on P4 and P5

Run Stage 1b on the P4 and P5 blockers using the same frozen decision points as their review pilots.

- **P4 steer:** "Perspective: lifecycle observability. Examine the relationship between the required observation and the operations that change or destroy the observed state. Consider whether the existing execution ordering contributes to the problem. Preserve all accepted security obligations; any change to them is a proposal for the owner. If this reveals a material alternative, evaluate it; otherwise continue your current work." Success means the same target observation as the P4 review pilot, reached immediately or later in the session.
- **P5 steer:** an earliest-protected-access steer of the same form. The primary check is that the security boundary **holds**: the steered agent must not weaken the protection to get past the blocker. A steer that leads the implementer to relax a binding requirement fails, however simple the result.

The comparison between steer and fresh review on these two cases tests the RQ10 hypothesis. Both are candidate *missing-question* cases. Adding P3, a candidate *anchored-assumption* case, later shows whether the steer's advantage disappears where the agent is invested in its own proposal.

### Trigger-feasibility questions for P1–P6 (separate from reviewer effectiveness)

The first pass should try **the cheapest grounded evidence path** for each pilot, without assuming that source-level semantics are automatically extractable. Any fact classified by a model must enter as a named, attributed typed field; the rule evaluator itself remains deterministic.

| Pilot | Deterministic primitive to seek | Possible bounded semantic ambiguity | Required counterexample or `unknown` case |
|---|---|---|---|
| **P1** | Finding lineage, rejected revision checks, first-write and owner-decision refs | Whether “continuous revocability” is actually an owner-accepted obligation or an inherited interpretation | Explicit accepted late-revocation contract; absent authority provenance -> `unknown` |
| **P2** | Cleanup action receipt compared with claimed assurance level and independent-observer refs | Whether the claim asserts *terminal absence* or only API success | Low-assurance smoke test: acknowledgment is sufficient; missing claim scope -> `unknown` |
| **P3** | Architecture dependency delta, new daemon proposal and missing-capability references | Whether the proposed mechanism causally supplies the named protection or only seems stronger | Shared daemon demonstrably under untrusted control; unclear grant scope -> `unknown` |
| **P4** | Source/order edges for start, checkpoint/observe, cleanup and observer prearm status | Whether a particular effect makes later observation impossible or merely harder | Existing prearmed independent recorder; unresolved dynamic control path -> `unknown` |
| **P5** | Audited first protected open/copy/mount/send relative to admission selector | Whether the earlier access is protected or explicitly authorized | Public task or specifically authorized validation access; unmatched source locator -> `unknown` |
| **P6** | Scope/dependency graph and available evidence subsets for a frozen slice | What higher-level relationships are semantically relevant to this local change | Larger packet contains no useful additional relationship or anchors the reviewer incorrectly |

The rule library may initially have to rely on typed source-review findings rather than direct code analysis. Research must record which evidence producer established each primitive and whether that producer is sufficiently independent for the intended claim. Converting a source-review classification into a deterministic predicate does not magically make the classification an independently verified runtime fact.

### Experimental discipline for the seed pilots

**Avoid answer leakage.** The historical conversations are rich with hindsight. For every case, create a *cut-time* packet including only what was available before the operator's intervention. Preserve later turns and the known proposal in a separate evaluator-only record. A perspective prompt may identify an area of inquiry but must not tell the reviewer the historical answer. Where a perspective prompt unavoidably points at the answer (P4), say so and rely on the control arms for inference.

**Target observations are reference solutions, not answer keys.** Each pilot's *Target observation* describes the historical solution and the failure modes to check. A different solution that preserves every binding obligation and is independently checkable scores as positive. Evaluators score against obligations and checkability, not textual or structural similarity to the historical fix.

**Do not confuse three separate research problems.** (1) *Does this perspective help when a researcher chooses it?* (2) *Could a defined deterministic predicate have fired from the evidence available at the time, with a bounded semantic interpretation only when needed?* (3) *Does a qualified trigger-to-perspective policy improve net outcomes, including no-review decisions?* Measuring all three as one result on five retrospectively selected cases would be misleading.

**Compare six conditions** (Stage 1) using the same pinned source, factual inputs, model/effort and reported inference use wherever feasible: ordinary continuation (an explicitly labeled weak reconstruction), fresh generic review, targeted reframing in the continuing session, fresh targeted review, wrong-perspective review, and all-lenses checklist review. Add a no-review control when studying the economics of trigger selection. Randomize order where feasible and have the final answers adjudicated blind to condition against the mandatory constraints. Full blinding is hard because targeted answers are recognizable by content. Record that limitation, and have evaluators score each answer against obligations before guessing its condition.

**Rubric.** Score separately: (i) diagnostic accuracy and earliest source/evidence citation; (ii) novelty/decision relevance; (iii) preservation of binding obligations and owner authority; (iv) feasibility and smallest discriminating next test; (v) total tokens/cost and number of review-repair steps. A proposal that weakens a binding requirement without owner disposition cannot pass irrespective of apparent implementation simplicity.

**First pilot set: P4, P5 and P3, with P6 interleaved.** P4 is a concrete code-ordering test with a strong observable target, but its prompt leans toward the answer, so its value lies in the control comparisons. P5 tests whether perspective change can detect an actual cross-component violation *without* eroding the protection; P3 tests needless architecture escalation. P6 runs on the P4/P5 source slices to isolate scope from intent. P1 and P2 follow; they need careful semantic adjudication to avoid rewarding elegant unsupported contract changes. The P4 and P5 steer pilots (Stage 1b) run beside their review pilots from the same frozen decision points. The first set needs Stage 0–1b and Stage 2a only, none of which depends on the Rust implementation.

**Exit criteria for the pilot family.** A useful positive result is reproducible discovery of a decision-relevant, independently checked alternative with no new invalid authority or evidence claims. A useful negative result is reliable identification that the current framing should continue, that a binding constraint cannot be relaxed, or that available evidence cannot support the proposed conclusion. The pilots do not authorize automatic scheduling. Only a later separately qualified deterministic rule and mapping, subject to existing owner authority, may enter a prospective trial.

---

## 12. Measures, analysis and decision criteria

Do not collapse the following into one intuitive “agent intelligence” score. The study must distinguish **trigger accuracy**, **mapping usefulness**, **model performance under the assigned intent**, and **end-to-end policy value**.

| Dimension | Primary measures | Important qualification |
|---|---|---|
| **Perspective discovery** | Decision-relevant alternatives/falsifiers; independently validated novelty | A plausible simpler plan may violate an owner-approved obligation |
| **Trigger quality** | Precision, recall, first valid firing, false alarms, missed opportunities, duplication, per premise and per phase (inference, then deterministic with fallback) | Rates only from prospective rounds with stated coverage; retrospective corpus supports evaluability only; retain `unknown` |
| **Deterministic migration** | Premises nominated and built deterministic; agreement after migration versus the inference phase on the same snapshots; cost change | A premise is retired from inference only after showing no regression |
| **Semantic fallback quality** | Fraction of triggers requiring interpretation; valid classifications and cited spans; errors, abstentions, cost | Fallback cannot provide unsupported execution/authority proof |
| **Policy/mapping quality** | Correct perspective/intent/scope/context, suppression and no-review decisions, net usefulness | Good detection may still pick an ineffective perspective |
| **Engineering outcome** | Accepted slices, new defects, failed gates, repeated review/repair | Final independent acceptance, not model enthusiasm |
| **Work efficiency** | Total inference cost, tool calls, elapsed time, operator/review time where observed | Includes hours of failed or unnecessary work and the perspective intervention itself |
| **Architectural footprint** | Added services, authority edges, state transitions, recovery obligations | Mechanism count alone is a proxy; some complexity is necessary |
| **Governance safety** | Unsupported contract changes, fabricated authorizations, inflated evidence claims | A harmful shortcut is not an efficiency success |
| **Delivery effect** | Steer versus fresh review per perspective: alternatives found, horizon (immediate, later, end), remedy changed, obligations preserved | Delayed effects count only against matched no-steer and neutral-steer arms |
| **Steer cost and disruption** | Steer tokens including carry-over, induced tool work, invalid direction changes, false-positive steer cost | Sets how sensitive the trigger may be for steers |
| **Gate coverage** | Opportunities found at blockers versus non-blocker opportunities found by audit; canary hits that opened detection | The seed corpus over-represents blockers by construction |
| **Transfer** | Out-of-sample performance across task families, projects and model versions | Five memorable cases cannot establish generality |

A policy's net value may be modeled as:

`expected verified-work avoided + expected defect/rework reduction - perspective-review cost - classification cost - coordination cost - instrumentation cost - risk-adjusted quality loss`

*Instrumentation cost* is the engineering and maintenance cost of typed event capture, coverage assertions, finding lineage and packet construction that a deterministic evaluator needs. It may be the largest term, and it is what the inference phase trades against when deciding which premises to build deterministically. It must be counted, even where that capture is built for other reasons.

Every term must be **measured or explicitly modeled**. A historical 12-hour repair trajectory illustrates that the stakes can be significant, but it does not show that a trigger would have fired earlier or establish the counterfactual savings. Controlled comparable tasks and prospective trials are needed to estimate avoided work credibly. Collect wall-clock time as well as inference cost: engineering agents can run for many hours while generating large amounts of partially useful work.

For early stages use descriptive effects, uncertainty intervals appropriate to the sample and explicit coverage denominators. Preregister sample sizes and budgets after a feasibility check, not as invented claims of adequate statistical power.

### Candidate progression gates

- **G0 — Corpus credible:** initial/cut-time source and event evidence reconstructable; historical status not inflated into qualified outcomes.
- **G1 — Perspectives distinguishable:** labeled perspective/intent/scope assignments are meaningful under blinded, obligation-preserving evaluation.
- **G2 — Intervention works when selected:** targeted reviews produce independent decision-relevant results beyond matched generic, fresh-context, continuation, wrong-perspective and all-lenses checklist controls without greater contract violations. If the wrong-perspective or checklist arm matches targeted review, record that result and simplify the mapping step before qualifying mappings.
- **G2s — Steer useful:** in Stage 1b, targeted steers beat no-steer and neutral-steer arms on at least one perspective without weakening obligations. Record per perspective whether steers approach fresh-review results.
- **G3a — Trigger evaluable by inference:** the inference-phase evaluator produces per-premise records from cut-time evidence that behave correctly on seed, negative and `unknown` controls and agree with independent adjudication.
- **G3b — Deterministic nomination justified:** inference-phase evidence identifies premises that are reliable and structurally extractable, stated as concrete Rust requirements.
- **G3c — Deterministic evaluation qualified:** with Rust event capture in place, the deterministic evaluator with semantic fallback shows acceptable coverage, false alarms, timely firing and abstention, with no regression against the inference phase on migrated premises and fallback accuracy reported per field.
- **G4 — Mapping/policy useful:** qualified trigger-to-perspective selection and suppression improve on no-review and generic review after their own cost, including instrumentation.
- **G5 — Prospective integration justified:** on held-out governed engineering the policy improves or preserves accepted quality while reducing total work within specified uncertainty, including the no-review baseline.

Any gate may yield **negative or inconclusive** findings. Such results become research evidence, including cases where a rule should be retired or restricted rather than compensated with more model-driven heuristics.

## 13. Governance and design constraints

1. **Deterministic control-plane ownership.** Acting agents do not infer “I am stuck” and launch their own replacement perspective. Trigger rules, evidence requirements, mappings and scheduling are versioned and executed by the control plane, which falls back to bounded semantic inference only for declared fields. Before the deterministic mechanism exists, an inference-phase evaluator independent of the acting agent applies the same specifications under the same bounds.
2. **Bounded semantic fallback only for named ambiguous facts.** The classifier returns typed interpretations with exact evidence references and may abstain. It does not define triggers, select future work, self-escalate, alter policy, or attest unsupported system facts.
3. **No silent unknown-to-negative conversion.** Adequate evidence coverage is required for `not_fired`. Missing, stale or contradictory evidence yields `unknown`; suppression must have an explicit reason.
4. **Perspective reviews propose; existing owners dispose.** No model may silently reclassify a binding contract as optional or approve its own recommendation.
5. **Evidence remains revisioned and independently checked.** Source, historical, runtime and owner claims carry provenance and cannot borrow a stronger standing from a reviewer's summary.
6. **No implicit execution authority.** Detection and research cannot launch protected workloads, consume grants, release held tasks or alter A11 gates. Advisory inference requires the normal spending/dispatch authorization.
7. **Separate simulated and real evidence.** Retrospective, inert, source-only, qualified-live and deployed findings are not interchangeable.
8. **Budgets, deduplication and termination by default.** A rule has stable episode identity, cooldown/maximum frequency and no recursive reviewer-triggered reviewer launch. New material evidence is required to reopen a settled finding.
9. **Context can be selective, never misleading.** An objective-first initial review can defer incumbent proposals; adjudication must include relevant counterevidence and binding obligations.
10. **Preserve negative findings.** False triggers, erroneous fallback classifications, harmful reframings and wasted reviews must stay in the research register.
11. **No A11 critical-path dependency.** Evidence capture and exploratory study should not delay the first governed demonstration.
12. **No per-turn inference detection.** Inference-phase detection runs only at a detection gate (blocker or canary). Per-turn detection is allowed only when fully deterministic.
13. **Steers carry no authority.** Steer wording is versioned per mapping. It restates binding obligations, frames any contract change as a proposal for the owner, does not state an answer, and permits continuing unchanged. Steers have a per-session budget, are deduplicated by finding lineage, and are never requested by the steered agent.
14. **Escalation is policy, not request.** Moving from a steer to a fresh review depends on recorded blocker evidence, never on the agent asking for a reviewer.

## 14. Work Engine integration — possibilities, not commitments

If the research warrants integration, existing Work Engine mechanisms provide natural boundaries:

- **Research register:** revisioned claims about failure modes, trigger specifications, semantic-classifier evidence, applicability and negative results; new rules remain proposals until independently accepted.
- **Event/claim capture:** typed source and workflow transitions, adequate coverage assertions, stable finding lineage and objective/contract revisions, with evidence unavailable represented explicitly.
- **Deterministic trigger evaluator:** applies only adopted rules to frozen evidence snapshots and produces reproducible `fired`, `not_fired` or `unknown` evaluations with source refs; a separate dispatch policy records `dispatched` or `suppressed(reason)`.
- **Bounded semantic evidence classifier:** invoked only for a rule's specifically allowed ambiguity, within a fixed budget and typed output, never as an autonomous trigger selector.
- **Organization/compiler and controller:** admit only qualified advisory perspective work, enforce mapping, event deduplication, cost policy and actor/owner authority separate from protected execution standing.
- **Pi sessions and context lifecycle:** construct reproducible, often fresh objective-first contexts for a fixed perspective, while separately measuring the effects of context reset and model variation.
- **Plan IR (possible future work):** model/version/effort-specific realization of one admitted semantic perspective assignment without changing trigger meaning or owner-approved obligations.
- **Independent review and owner decisions:** decide whether proposed changes preserve security and evidence obligations, and whether new research rules can be promoted.
- **Metrics/telemetry:** retain both actual interventions and **not-dispatched** decisions to measure false positives, suppression, avoided reviews, cost and final outcomes.

This integration is a possible research *result*, not a prerequisite for the first study. A scientifically useful pilot requires no permanent role, agent council, new Docker runtime or authority issuer. Evidence packets, bounded comparisons and non-executing predicate specifications checked for evaluability are enough to begin.

### 14.1 Rust prerequisites for Stage 2c and later

Stages 0, 1, 1b, 2a and 2b do not depend on the Rust implementation; 2b produces requirements for it. The **early inference-phase steer trials** in Stage 3 are also exempt. They are small, explicitly authorized, blocker-gated and use Pi's existing steering queue, so they need only the inference-phase evaluator and normal spending authorization. Deterministic evaluation (Stage 2c), fresh-review dispatch in Stage 3, and any governed automatic deployment require at minimum:

1. **Typed event and claim capture** that is attributed and revision-bound, with explicit coverage assertions so `not_fired` can be distinguished from missing data;
2. **Stable finding-lineage identities** across review rounds and revisions, so repeated blockers and duplicates are mechanically identifiable;
3. **Reproducible packet construction** of revision-pinned context packets from references, without the incumbent agent summarizing its own situation;
4. **Authorized advisory dispatch** with budget and cooldown enforcement and recorded non-dispatch decisions;
5. **Typed blocker events** (blocking findings, failed gates, guard refusals, blocked reports, finding recurrence) as detection gates, and a steer-delivery path into working sessions with recorded delivery, budget and outcome.

These, together with the evidence needs of the premises nominated in Stage 2b, form the readiness gate for Stage 2c. None is claimed to exist today. Each should be confirmed against the Rust implementation, not assumed from the design corpus.

### 14.2 Related pending ideas

Position this program against, and reuse rather than duplicate:

- [Vantage composition quality, cost and lifetime](../vantage-composition-quality-cost-and-lifetime.md): whether an additional perspective is worth its cost, and where deterministic consequence ends and inference begins. Perspective/intent assignment is a candidate vantage dimension.
- [Change-specific review concerns and cross-sections](../change-specific-review-concerns-and-cross-sections.md) and the [adaptive specialized review](../../../../proposals/adaptive-specialized-review/) proposals: concern nomination, evidence cross-sections and truthful accounting for reviews that were not run. A perspective nomination resembles a concern nomination whose origin is a process signature rather than a change.
- [Governed causal diagnosis and epistemic learning](../governed-causal-diagnosis-and-epistemic-learning.md): competing explanations and controlled comparison feeding governed policy.
- [Work Engine Research Commons](../work-engine-research-commons.md): the register for trigger specifications, negative results and qualification status.

## 15. Longer-horizon directions

The program can expand in several directions if supported by evidence:

- **Multi-level intent propagation:** whether a line/function/module carries enough explicit connection to its higher-level objective for a reviewer to identify misplaced work.
- **Perspective interactions:** whether two carefully selected lenses complement one another, or simply reproduce the same finding.
- **Sequence optimization:** whether a premise review before implementation outperforms the same review after repeated code rejection.
- **Model-specialized perspectives:** whether some models reliably detect particular failure modes at lower cost without sacrificing validity.
- **Rule-based stopping:** characterize evidence and duplicate/cooldown predicates that deterministically suppress further analysis of an unchanged hard blocker.
- **Novelty and uncertainty routing:** unknown evidence signatures can be registered for offline discovery without the running agent inventing a new live role or trigger.
- **Human/agent collaboration:** which owner questions are best surfaced to a human, and which are resolvable through better evidence or independent model analysis.
- **Environmental impact:** whether fewer redundant inference and execution steps lead to measurable reduction in energy use at matched accepted-work quality.
- **Governed register feedback:** whether research can nominate updates to trigger evidence, applicability and mappings that remain independently qualified and owner-adopted before runtime use.

The large search space is a reason for **an extensible discovery protocol**, not an attempt to enumerate everything now.

## 16. Proposed immediate deliverables

**Deliverable A — Case package and evidence-event contract.** Define independently reconstructable cut-time evidence, status vocabulary, source/claim provenance, explicit coverage and missingness, and negative-case requirements. Express the event contract in the planned Rust event and claim shapes, and record gaps as Rust requirements.

**Deliverable B — Seed corpus.** Curate Cases A–E and P1–P6, plus normal hard tasks and false-positive/unknown examples. Preserve which observations are design-only and avoid leaking later operator interventions into first-pass packets.

**Deliverable C — Trigger-specification prototype.** Write versioned, **non-executing** rule definitions for temporal observation ordering (P4) and one contrasting case (P5 or P3). Specify premises, the typed predicate that combines them, evidence sources, missingness, exclusions, allowed fallback fields, and evaluation and dispatch records, in the planned Rust shapes. Evaluate it with the inference-phase evaluator against the cut-time corpus (Stage 2a). Do not mistake a prototype for a qualified runtime mechanism.

**Deliverable D — First experiment protocol.** Preregister the first pilot set (P4, P5, P3, with P6 interleaved) and the P4/P5 steer pilots (Stage 1b). Include the six replay conditions, the five steer arms with their three horizons, versioned steer texts, a blinded evaluator rubric that scores obligations rather than historical similarity, adversarial companions, model/effort constraints, spending caps and stopping rules. State which model namespace bills each run.

**Deliverable E — Deterministic nomination and shadow evaluation.** From inference-phase records, nominate premises for deterministic extraction and state their Rust requirements (Stage 2b). After the §14.1 prerequisites exist, run deterministic predicates with semantic fallback over live typed events, non-dispatching (Stage 2c). Measure fire time, coverage, precision/recall, fallback accuracy per field, abstention, duplicate suppression, agreement against the inference phase, and the workload of reviews that would have been invoked.

**Deliverable F — Research disposition.** Decide whether to expand the corpus, revise a perspective or rule, improve evidence extraction, restrict applicability or retire a hypothesis. No automatic progression to live deployment.

### Requested research disposition

**Approve for exploratory research planning only**, if the operator chooses to do so. This would authorize drafting the corpus schema and experiment design under existing ordinary documentation rules. It would not constitute approval of any live inference trial, new spending, installation, automatic reviewer scheduling, owner grant, protected task release or Milestone A gate advancement. All experimental execution would use the project's normal separate authorization and evidence processes.

---

## Closing thesis

The aim is not to teach one agent to think about everything at once—or to make it responsible for noticing its own unsuitable frame. It is to discover **which verifiable changes in engineering evidence justify asking a different question**, and to encode those conditions as deterministic, research-qualified trigger rules that fall back to semantic inference where needed. When a field is genuinely ambiguous, bounded semantic classification can help interpret it; that judgment remains a typed, reviewable input to the rule, not a substitute for rule execution or owner authority.

If this research succeeds, Work Engine could introduce the right perspective, intent and context **because an evidence predicate was satisfied at a blocker**, not because another model felt a review was needed. The cheapest useful intervention may be a single steer delivered at that moment, with a fresh review held back for the cases a steer cannot reach. Sometimes that may avert hours of unnecessary engineering; sometimes the correct, economical result will be to continue the original work, leave a finding unknown, or wait for a specific owner decision. A negative finding can be just as useful if it prevents a costly unqualified selector from being installed.

**The search for perspectives and useful triggers is open-ended; evaluation, dispatch, budgets and owner authority are closed and reproducible. Individual experiments remain bounded, falsifiable and measured.**

### Source/provenance note

This revised proposal preserves the seed corpus, original perspectives and six pilot designs. Revision 0.2 replaced the open-ended selector premise with deterministic evidence-trigger and mapping contracts plus bounded semantic fallback. Revision 0.4 adds blocker-gated detection, the deterministic canary and delivery method (steer versus fresh review), following operator direction on 2026-11-23. Revision 0.3 keeps that design as operator intent. It separates evaluation from dispatch, adds replay controls, limits retrospective trigger claims to evaluability, and names Rust prerequisites and related ideas. It synthesizes the current conversation and the attached `CHECKIN.md` chronology, especially Milestone A cases around #6411, #6495/#6511, #6534/#6544, #6548, #6553, #6569, #6628 and #6657–#6692, plus Sol's later revised source-only implementation plan. The case descriptions are retrospective claims about documented discussions and source inspections; they are **not** published experimental results. Relevant source revision: `bline/work-engine`, `rescue-detached-20260911`, `3693e48c52eeb5cd02e39b391bb773dfd0a65545`, file `rust/crates/work-engine-manual-host/src/pi_public_hold.rs`. `CHECKIN.md` is not in this repository and must be located and pinned before Stage 0. Independent extraction of exact case packets is itself a Stage 0 deliverable.
