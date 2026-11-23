# Perspective, Intent, and Scope in Governed Agentic Engineering

**A Work Engine research note**  
**Status:** Exploratory hypothesis and retrospective case analysis — not an approved architecture or demonstrated research result  
**Date:** November 22, 2026  
**Revision:** 0.3. Keeps the 0.2 design (deterministic evaluation, falling back to bounded semantic inference), recorded as the operator's design intent. Evaluator and dispatch outcomes are separated. This note now holds the case record and hypothesis; the protocol lives only in the [research proposal](pilots/perspective-intent-pilot.md). Supersedes 0.2 and the selector framing of 0.1.  
**Basis:** The Milestone A design conversations, the operator's interventions, Sol's revised plans, and the associated `CHECKIN.md` history. That file is not in this repository; locating and pinning it is a Stage 0 prerequisite.

## Abstract

An engineering agent can competently solve a local technical problem while overlooking that the difficulty comes from an unnecessary interpretation, unsuitable ordering of operations, or mismatch between a local task and the larger objective. Several Work Engine Milestone A blockers became simpler when the operator changed **perspective** (the boundary and relationships being examined) or **intent** (the judgment being sought). Other cases required preserving a hard protection boundary, not simplifying it away.

This note proposes that perspective, intent, scope, and context construction are separable experimental variables. The orchestration design is more specific: **Work Engine, not the acting agent, determines whether an evidence-qualified trigger has fired and whether an additional perspective is warranted.** The acting agent's own framing is the likely source of the blind spot (correlated error).

**Design intent (operator, 2026-11-22):** Work Engine makes this determination deterministically wherever it can, through versioned predicates over observable events and claims. Where the deterministic aspect cannot resolve a needed fact, it **falls back to bounded semantic inference**, which classifies that fact into a fixed schema. A deterministic evaluator still applies the rule. The classifier cannot invent evidence, grant authority, dispatch itself, or silently convert an unknown result into a pass. This is the core-contracts principle *determinism where judgment is unnecessary* (§16): mechanics decide what they can, and model judgment enters at the declared fallback where meaning must be interpreted.

The deterministic mechanism does not exist yet, so trigger evaluation **begins with inference**. An evaluator independent of the acting agent applies the same versioned specifications to frozen evidence, premise by premise, and produces the records the deterministic evaluator will later produce. Evidence from those rounds decides which premises are built deterministically and which remain declared fallback fields. Inference is the starting state of the planned design, not a competing design. The research qualifies *which* predicates and fallback fields work; it does not reopen whether the control plane is deterministic-first. Most candidate triggers in the seed cases need at least one fallback field, so fallback accuracy is a primary measure, not a side one.

The historical cases nominate candidate triggers and perspectives but do not establish causal savings or an implemented detector. Research must separately validate (1) evidence signatures and trigger quality, (2) perspective and intent effectiveness, (3) mappings and intervention timing, and (4) total engineering cost and accepted quality. Correctness, owner authority, and independently checked evidence remain non-negotiable.

## 1. The underlying idea

Software engineering consists of decisions made at interdependent scopes. A line of code implements an operation. A function fulfills a behavioral contract. A module preserves subsystem responsibilities. An architecture coordinates components and invariants. A product or research objective explains why those components should exist at all.

All of these levels carry intent, but a model working within one level may not keep the others equally salient. When implementing a function, attention is often concentrated on how to make that function satisfy its current contract. Repeated failures may focus attention still further on a particular race, lock, parser, or cleanup path. Locally reasonable repairs can then accumulate even if a higher-level decision would eliminate much of the difficulty.

The key hypothesis is **not** that models cannot reason across abstraction levels. They can. The current objective, selected evidence, and accumulated reasoning trajectory may make some perspectives much more likely than others. A fresh session with a distinct purpose may reveal alternatives that an ongoing implementation session is unlikely to prioritize. **That is an intervention hypothesis, not a reason to assign the acting agent responsibility for noticing its own blind spots.** Trigger evaluation belongs to the deterministic control plane, with bounded semantic fallback.

### Working definitions

| Concept | Proposed meaning | Example |
|---|---|---|
| **Objective** | Outcome the work exists to achieve | Complete a governed A11 engineering demonstration |
| **Contract** | Obligations made binding by an authorized owner | Do not expose protected task bytes before the permitted delivery boundary |
| **Perspective** | Boundary, relationships, and abstraction level considered | A function, the Host–Docker lifecycle, or owner-to-execution relationship |
| **Intent** | Question or result a model is assigned to pursue | Implement, falsify, simplify, reconcile, or challenge a premise |
| **Context** | Selected facts, constraints, evidence, and history available to the inquiry | Frozen objective and blocker before the incumbent repair proposal |
| **Evidence event** | Attributed, timestamped and revision-bound observation or claim | A review finding, source-order edge, cleanup response, or proposed daemon |
| **Trigger specification** | Versioned applicability, evidence requirements, predicate and exclusions | Required observation ordered after destructive cleanup |
| **Trigger evaluation** | Deterministic assessment of a spec against a bounded evidence snapshot | `fired`, `not_fired`, or `unknown` with supporting references |
| **Dispatch decision** | Separate policy step applied to a `fired` evaluation: qualification, duplicates, cooldown, budget, authority | `dispatched` or `suppressed(reason)` |
| **Semantic fallback** | Narrow classification of ambiguous evidence to a fixed schema when deterministic extraction cannot resolve it | Classify whether a proposed new component is intended to supply an observation capability |
| **Perspective policy** | Research-qualified mapping from fired trigger to perspective, intent, scope and context, subject to cost and duplicate guards | Temporal-observability review for a verified observation-order hazard |
| **Authority** | Owner allowed to change or adopt the applicable contract | Operator or designated Campaign owner, not the critic or trigger evaluator |

Perspective and intent are not synonyms. Two reviewers can examine one function at the same scope while one seeks defects and the other seeks unnecessary complexity. One intent can also be applied at the level of a line, module, architecture, or product objective.

A useful research unit is therefore **(versioned trigger + evidence snapshot, perspective, intent, scope, context policy, evaluation outcome)**, not a generic agent persona. The trigger is not a feeling of being stuck. Neither the acting agent nor the selected reviewer decides whether the reviewer should have been invoked.

## 2. Case observations from Milestone A

These are retrospective observations of design discussion. They do not establish controlled cost savings, correctness of unimplemented fixes, or that one perspective would always find the same issue.

### Case A — Revocation after admission

**Inherited framing:** Authorization had to remain current through Server effects and actual Host entry. Agents investigated revision checks, live Campaign ownership, rechecking SQLite, and races between revocation, filesystem writes, and detached launch.

**The operator's change:** Some decisions become irrevocable when the operation commits. Consume a one-shot authorization *before the first permitted effect*. Before that point, revocation wins and the operation refuses cleanly. Afterward, changing the authorization is not a clean revocation of completed or ongoing effects: stopping requires abort, termination, and reconciliation. The exact approved Campaign revision can remain authoritative for that attempt rather than chasing a moving head.

**Perspective shift:** From “how do we preserve continuously revocable permission?” to “what is the legitimate point of commitment, and what consequences follow?”

**What changed:** A much simpler candidate admission contract was adopted in the design discussion, avoiding the need to maintain live revocability across the entire run. The implementation and protected authority join remained unqualified.

**Candidate trigger:** Repeated attempts to enforce a continuously changing invariant; especially when the requirement's provenance and necessity have not been reconsidered.

### Case B — Kill action versus proof of termination

**Inherited framing:** The system needed to account for removal and whether the actor requesting cleanup could prove it had performed the kill.

**The operator's change:** The killer acts; an independent verifier determines whether the exact admitted execution is gone. The killer's success report is neither necessary nor sufficient evidence of terminal absence. Unknown settlement keeps the attempt excluded; any prior effects are reconciled separately.

**Perspective shift:** From actor attribution (“who performed removal?”) to outcome verification (“can this attempt still execute?”).

**What changed:** A clear separation emerged between termination mechanics, independent observation, and permanent one-shot fencing. This was a design decision, not yet an implemented independent proof.

**Candidate trigger:** A component is being asked to certify its own effect, or an API acknowledgment is being promoted into an independently verified outcome.

### Case C — The proposed second Docker daemon

**Inherited framing:** A source-level counterexample showed that successful Docker/containerd deletion and empty registries need not prove every shim or asynchronous operation has settled. Review then favored a separate administrator-controlled Docker runtime as a candidate qualification environment.

**The operator's challenge:** What exact observed conflict requires a second daemon? A private socket can simplify who submits work, but it does not establish which executable ran, whether it reached the reduced-privilege state, or whether all relevant work stopped. If no control-boundary conflict is demonstrated, start with the existing daemon.

**Perspective shift:** From runtime-topology selection to separation of **control**, **observation**, and **terminal settlement** responsibilities.

**What changed:** Sol withdrew the implicit preference for another daemon. The shared daemon became the baseline, with a separate administrative-control test and independent process observation still required. No claim was made that the actual governed Builder was qualified.

**Candidate trigger:** An infrastructure migration or new service is proposed in response to missing evidence, without explaining which missing evidence the proposed technology can provide.

### Case D — The init removed before it could be observed

**Inherited framing:** The fixed-init qualification needed an independently observable post-reduction process. Agents considered a prearmed kernel event recorder because the init could disappear before a late observer attached.

**Source-level observation:** In `rust/crates/work-engine-manual-host/src/pi_public_hold.rs` at `3693e48c` (around lines 299–311), the Host starts the init, confirms Docker's `Running` projection, and proceeds directly toward cleanup. The fixed init checks its own state and sleeps for 30 seconds, but that sleep is not a guaranteed observation window if Host removes the container immediately.

**Alternative question:** Why is the launcher allowed to remove the process before an outside observer has had a bounded opportunity to examine it?

**Perspective shift:** From compensating for a fast-disappearing process with more sophisticated instrumentation to arranging the lifecycle so the required state can be observed.

**What changed:** Sol revised the plan to investigate a bounded observation hold **before normal cleanup**, first testing process/cgroup readback techniques already exercised in the earlier #6411 smoke test. The observer's acknowledgment does not itself prove success, extend the hold indefinitely, or authorize release. This change was planning-only when recorded; no new Docker run or implemented qualification was claimed.

**Remaining subtlety:** The observer must establish that the *same actual process* executed approved bytes and reached its post-init checkpoint. A `Running` bit or pre-initialization UID alone is insufficient. Readiness can synchronize sampling but is not independent attestation. Lost or unsettled execution remains unqualified and fenced.

**Candidate trigger:** A difficult observation problem arises because the implementation orders destruction, release, or handoff before its required witness can inspect the state.

### Case E — Protected task read before held-job selection

**Inherited framing:** To prepare a Pi session without exposing the protected task, agents explored hold flags, omitted mounts, and deferred delivery.

**Cross-component finding:** Server and Host ordinary job validation could open or snapshot the task **before** the hold became active. Merely removing the later task mount or moving a hold flag would not eliminate the earlier access.

**Alternative design:** Select a separate, owner-bound **metadata-only held input** before ordinary validation on both sides. It contains opaque task identity, authorized digest and bound, public-job/attempt identity, but no source locator or task bytes. A separate, later decision governs actual acquisition and delivery.

**Perspective shift:** From modifying a downstream container configuration to examining the earliest point where protected information crosses a boundary.

**What changed:** The requirement for a distinct early-selected held input was identified. Delivery to the actual Pi receiver and protected admission remained independently blocked.

**Why this case matters:** The crucial cross-component flaw was identified by agent review rather than by the operator's simplification. The research must not assume that agents only implement locally or that human interventions always produce the best architecture. Different reviews reveal different issues.

## 3. The pattern — and its limits

Across these cases, the unproductive trajectory was often not a lack of technical reasoning. The agents found genuine races and evidence limits. The problem was an implicit decision to keep solving **inside** a particular framing:

| Local question | Higher-level question that changed the discussion |
|---|---|
| How do we keep permission revocable during execution? | Must it be revocable after the commitment point? |
| How does the cleanup actor prove it killed the process? | What independently establishes that execution can no longer act? |
| What stronger Docker infrastructure is needed? | Is the missing property control, observation, or settlement? |
| How do we capture an init before it disappears? | Why is the launcher removing it before observation? |
| How do we hide task bytes at the final mount? | Where is the earliest task access, and can input selection precede it? |

This suggests a specific failure mode: **locally coherent problem-solving under a framing that is incomplete, overly restrictive, or misplaced**. The Milestone A history also shows the scale can be many hours of agent work, not merely a few review rounds. The cases do not provide a controlled estimate of avoidable hours, tokens, or dollars; that cost must be reconstructed or measured separately.

However, some constraints are genuinely non-negotiable. Changing a security guarantee merely because implementation is difficult would be a regression. A premise challenger may **propose** alternatives, identify provenance and consequences, and seek an authorized decision; it may not silently relax a contract. Likewise, simplifying an observation protocol cannot be called an accepted verification result until the exact evidence is collected and checked.

## 4. A candidate catalog of perspectives and intents

| Perspective | Intent | Candidate evidence signature, subject to qualification | Expected output |
|---|---|---|---|
| **Implementation** | Realize an accepted contract | Authorized, bounded engineering task | Code, tests, traced obligations |
| **Contract / premise** | Challenge necessity or interpretation of inherited requirements | Repeated difficult invariants; proposed special-case machinery | Alternative contracts, provenance, consequences, owner decision needed |
| **Simplification** | Remove avoidable mechanisms and coordination | Growing components, locks, state, or services | Minimal equivalent design, invariants preserved, unproved tradeoffs |
| **Lifecycle / temporal** | Examine ordering of preparation, observation, effect, cleanup, and restart | Races, lost clients, missed observations, delayed starts | Causal timeline and corrected observation/termination boundaries |
| **Authority / custody** | Identify who may select, modify, execute, observe and accept | Cross-principal handoff or protected data access | Authority map, effective grants, first trust-boundary failure |
| **Evidence skeptic** | Separate observed facts from interpretations and projections | “Verified,” “cleaned,” “accepted,” or “isolated” claims based on self-report | Exact evidence gaps and falsifying checks |
| **Integration** | Trace obligations across component boundaries | Two locally passing modules connected by a handoff | Earliest exposure, mismatched identity, or incompatible assumptions |
| **Product / research objective** | Reconnect implementation with the reason it exists | Long detours, mounting infrastructure, shifting goals | Objective-to-mechanism trace; what can be deferred or eliminated |
| **Cost / efficiency** | Determine whether added reasoning is worth doing | Repeated reviews, high iteration cost, expensive branches | Estimated value, confidence, cost, stop condition |

These are **research candidates**, not a recommendation to create nine permanent agent roles. The first experiment should determine which distinct perspectives are materially useful and which are redundant.

## 5. Independent trigger evaluation, with bounded semantic fallback

**Design direction:** Work Engine's control plane decides whether a versioned trigger has fired, deterministically first and falling back to bounded semantic inference. An agent doing implementation, code review, or tool work is *not* expected to diagnose that its own perspective has become unproductive. It can emit normal structured findings, but it cannot promote them into new authority or schedule a perspective review. The full trigger contract, evaluation semantics, and fallback bounds are specified in [proposal §6](pilots/perspective-intent-pilot.md#6-evidence-triggers-versioned-predicates-not-agent-self-assessment); this section records only the shape the cases suggest.

**Two layers, two vocabularies.** The trigger evaluator answers an evidence question and returns `fired`, `not_fired` (adequate coverage, predicate false), or `unknown` (missing, stale, crossed, or unclassifiable evidence). A separate dispatch policy acts only on `fired`. It checks qualification, applicability, open duplicate findings, cooldown, budget, and authority, and records `dispatched` or `suppressed(reason)`. Budget and cooldown are not evidence, so they do not belong in the evaluator's outcome. Both layers record decisions where no review runs; silent non-dispatch would bias the research.

**Semantic fallback.** Where a predicate needs a fact that only prose expresses, such as whether a proposed daemon is meant *to obtain evidence*, a bounded classifier may return a typed value with source spans and may abstain. Its output is an attributed interpretation, not proof of a runtime fact or an owner decision. Because most candidate triggers in the seed cases need at least one such field, classifier quality largely determines trigger quality.

**Inference first, deterministic as evidence allows.** Until the deterministic mechanism is built, an inference-phase evaluator resolves each premise under the trigger specification, with cited evidence and abstention allowed. Premises it resolves reliably from structured evidence are nominated for deterministic extraction and become Rust requirements. Premises that need interpretation stay as declared fallback fields. A premise moves to deterministic evaluation only after replay on the same snapshots shows no regression. Trigger quality is measured against no-review and generic-review operation. A positive trigger does not establish that the current approach is wrong, that the mapped perspective will help, or that any change is authorized.

### Illustrative trigger from Case D

The fixed-init problem suggests a candidate *temporal evidence-order* predicate:

- An accepted qualification obligation requires observing a process in a specified state.
- A source-pinned lifecycle sequence contains launch, potential observation, and cleanup or other effect-terminating operations.
- The selected path can perform cleanup before a separately controlled observer has had a bounded observation opportunity.
- No other admissible, prearmed observation path is established for the same task and lifetime.

Typed source-order and evidence records could make this deterministically checkable. Identifying a call's role as *destructive cleanup* may need the bounded fallback. A match nominates a **lifecycle/observability perspective with the intent to inspect ordering**, not an order to modify code; any alternative must still preserve termination, one-shot fencing, effective privilege proof, and the accepted owner contract. Similar orderings may be intentional, or an earlier trustworthy recorder may already cover the interval; such counterexamples are required for qualification.

## 6. Context construction matters as much as role selection

A fresh session is not automatically independent. If it receives a builder's preferred solution as an established premise, it may reproduce the same blind spot. Conversely, withholding binding constraints or known counterevidence could generate elegant but invalid proposals. The packet is constructed from the **frozen trigger evidence, the qualified perspective policy, and the accepted objective**, not by asking the incumbent agent what kind of reviewer it wants.

For a **premise or simplification review**, the first-pass packet should prioritize the original objective, authoritative accepted obligations, provenance of inherited assumptions, current blocking facts, and the exact decision owner. It can initially withhold detailed proposed repairs to reduce anchoring, then require a second pass against the actual rejected proposals and all relevant counterevidence before the result is considered actionable.

For **code and evidence reviews**, independence often requires seeing the exact candidate and its source/evidence, because the intent is to find defects in that concrete artifact. Selective context is a tool for improving the question, not permission to hide unfavorable facts.

### Proposed perspective-review contract

| Input or output | Required content |
|---|---|
| **Frozen task/trigger reference** | Objective, task/attempt ID, source and contract revisions, trigger ID/version and evidence snapshot |
| **Current obligations** | Required invariants; which are explicit owner decisions versus derived interpretations |
| **Observed blocker** | Evidence and failure boundary, not just a summary label |
| **Review assignment** | Policy-selected perspective, intent, scope, context version, allowed questions and prohibited actions |
| **Alternatives** | Keep current contract; simplify implementation; propose changing contract through owner decision |
| **Consequence analysis** | Effects on safety, evidence, compatibility, cost and downstream gates |
| **Disposition** | Evidence-supported finding, open question, or explicitly proposed owner decision |
| **Revalidation** | The smallest check that could falsify the new approach |

A reviewer produces **decision support**, not new execution standing. A change in a contract must be accepted by the appropriate owner and revisioned. A claim about verified effects requires independently checked evidence.

## 7. Research design (summary)

The protocol, controls, and gates are owned by [proposal §10–§12](pilots/perspective-intent-pilot.md#10-proposed-experimental-program). The points that follow from these cases are:

- **Perspective effectiveness and trigger performance are separate results.** A useful review attached to a moment chosen in hindsight does not show that the moment was detectable, and a correct trigger does not show that its mapped perspective helps.
- **Stages 0–1 (case packets and controlled replay) need no Work Engine runtime.** They can run through ordinary Pi sessions under normal spending authorization.
- **Replay must be able to fail.** The perspective prompt for a case like D nearly states the answer, so the replay includes a *wrong-perspective* arm and an *all-lenses checklist* arm: a fresh, objective-first review given the full §4 catalog. Both validate the trigger-to-perspective mapping in the planned design. If either matches the mapped perspective, the mapping step should be simplified before mappings are qualified. Responses are scored against obligation preservation and independently checkable decision relevance, not against similarity to the historical fix.
- **Retrospective trigger work can establish evaluability, not precision or recall.** The historical record is prose, so typed events must be annotated after the fact, which is where hindsight leaks in. With five hand-picked positives, the defensible claim is that a predicate is evaluable at the cut-time snapshot and behaves correctly on constructed negative and `unknown` controls. Precision and recall need prospective rounds. The inference-phase evaluator can run those in shadow over ongoing work before Rust event capture exists, and later the deterministic evaluator runs them over typed events.
- Report observed spending and wall-clock time separately from estimates. Long historical runs show the *possible scale* of the problem, not measured savings from an intervention that was never performed.

## 8. Risks and failure modes of the research idea

**Premise erosion.** A targeted reviewer could challenge difficult mandatory guarantees. Mitigation: trigger rules and review packets retain contract provenance; only authorized owners may revise obligations.

**Semantic evidence laundering.** An ambiguous prose classification could be silently treated as confirmed source, runtime, or authority evidence. Mitigation: source-spanned typed outputs, explicit `unknown`, no model-authored trigger policy, and independent fact validation where required.

**Missingness mistaken for absence.** A lack of events might be treated as evidence that the trigger did not fire. Mitigation: `not_fired` requires adequate declared coverage; missing or crossed evidence returns `unknown`.

**Endless meta-review.** The same blocker might cause repeated dispatches and prolonged analysis. Mitigation: deterministic deduplication by evidence/finding lineage, research-qualified cooldowns, budgets and stop rules; no recursively triggered review by the reviewer itself.

**False independence.** A new session may inherit the same framing or misleading context selection. Mitigation: objective-first packets, policy-compiled context, complete counterevidence in final adjudication.

**Overattribution and cost inflation.** A design change may look like a large saving without a measured counterfactual; a trigger may also increase cost. Mitigation: state the actual qualification level, observe total workflow costs, and preserve unfavorable trials.

**Excessive zooming out.** Local implementation work may genuinely need local reasoning, not new perspective review. Mitigation: negative controls, no-trigger decisions, explicit applicability and false-positive costs.

**Nominal determinism.** A predicate that combines several model-classified fields deterministically can look mechanical while its accuracy rests on the classifier. Mitigation: report, for each rule, which fields came from the fallback and with what accuracy, and push fields into deterministic extraction as structured evidence becomes available.

**Answer-anchored evaluation.** Cut-time packets, perspective prompts, and rubrics written by people who know the historical fix can lead the reviewer toward it, or penalise a different valid solution. Mitigation: wrong-perspective and all-lenses checklist controls, scoring against obligations rather than the historical answer, and evaluators blind to condition.

## 9. Relationship to Work Engine

This research builds on Work Engine's separation between **deterministic mechanics**, **bounded model judgment**, **accepted authority**, and **independently checked evidence**. The candidate control-plane sequence is specified in [proposal §8](pilots/perspective-intent-pilot.md#8-proposed-control-plane-model--fixed-mechanics-bounded-judgment).

**Related pending ideas.** This research should be positioned against, and reuse rather than duplicate:

- [Vantage composition quality, cost and lifetime](vantage-composition-quality-cost-and-lifetime.md): whether an additional perspective improves the result enough to justify its cost, and where deterministic consequence ends and inference begins. Perspective/intent assignments are a candidate vantage dimension.
- [Change-specific review concerns and cross-sections](change-specific-review-concerns-and-cross-sections.md) and the [adaptive specialized review](../../../proposals/adaptive-specialized-review/) proposal family: concern selection, evidence cross-sections, and truthful omission accounting for reviews that were not run. A perspective nomination is close to a concern nomination whose origin is a process signature rather than a change.
- [Governed causal diagnosis and epistemic learning](governed-causal-diagnosis-and-epistemic-learning.md): competing explanations and controlled comparison feeding governed policy.
- [Work Engine Research Commons](work-engine-research-commons.md): the register that would hold trigger specifications, negative results, and qualification status.

**Rust prerequisites for deterministic evaluation.** Case packets, replay and inference-phase trigger rounds do not depend on the Rust implementation; their records should use the planned Rust event, claim and register shapes so nothing needs translating later. Deterministic evaluation needs, at minimum:

1. typed, attributed, revision-bound event and claim capture, with explicit coverage assertions so absence can be distinguished from missing data;
2. stable finding-lineage identities across review rounds and revisions;
3. reproducible construction of revision-pinned context packets from references;
4. separately authorized, budgeted advisory dispatch, with recorded non-dispatch decisions.

These, plus the evidence needs of the premises that inference rounds nominate for deterministic extraction, form the readiness gate for deterministic evaluation; none is claimed to exist today.

The efficiency objective is not simply more reviews. It is avoiding unnecessary inference, implementation, coordination, tool use, and delay **when independently established evidence justifies a targeted intervention**, and avoiding the intervention when it does not.

## 10. Near-term recommendation

Preserve the five Milestone A cases and the six seed pilots (P1–P6, defined in [proposal §11A](pilots/perspective-intent-pilot.md#11a-seed-pilots-extrapolated-from-the-existing-five-cases)) as **candidate research evidence**, not retrospective successes. Do not install a perspective selector on the A11 demonstration path.

The first pilot set is Stage 0–1 on **P4** (fixed-init lifecycle ordering), **P5** (earliest protected access), and **P3** (mechanism necessity), with P6 interleaved on the P4/P5 source slices. Each runs with wrong-perspective and all-lenses checklist controls, an adversarial companion where a hard boundary must remain, and blind scoring against obligations. Trigger work in this phase writes versioned specifications in the planned Rust shapes and evaluates them per premise with the inference-phase evaluator at cut time. Its records nominate which premises to build deterministically. Deterministic evaluation waits for the Rust prerequisites in §9.

> **Central research question:** Can Work Engine discover and qualify observable evidence conditions that a **deterministic control-plane evaluator**, falling back to bounded semantic inference only for declared ambiguities, can use to select an effective perspective, intent, scope and context at the right decision boundary, and improve verified engineering outcomes per total work without weakening accepted obligations?

That is different from teaching agents to notice that they are stuck. It assigns detection and dispatch to governed mechanics and keeps model judgment within explicitly bounded inquiry and evidence-classification tasks.

---

### Source and status notes

This note retains the original case observations. Revision 0.3 keeps the deterministic-first design with semantic fallback, separates evaluation from dispatch, and moves protocol detail to the proposal. It is grounded in the discussion and the Milestone A `CHECKIN.md` records, notably #6411, #6495/#6511, #6534/#6544, #6548, #6553, #6569, #6628, #6648, and #6657–#6692, plus Sol's subsequent revised plan. `CHECKIN.md` is not in this repository; those records must be located, pinned, and linked before Stage 0. The reviewed fixed-init Host source was `rust/crates/work-engine-manual-host/src/pi_public_hold.rs` on `rescue-detached-20260911` at `3693e48c52eeb5cd02e39b391bb773dfd0a65545`. Sol's bounded-observation change was a plan revision, not an implemented or executed qualification. This note does not approve deployment, access changes, protected task delivery, A11 dispatch, or a gate.
