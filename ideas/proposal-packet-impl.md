# Idea-to-Proposal Formation Workflow

## Idea

Introduce a bounded workflow that converts a lightweight idea seed into a coherent, architecturally placed proposal packet before expensive evidence-backed evaluation begins.

## Problem

Raw ideas are intentionally speculative. They may:

* describe a symptom rather than the actual problem;
* leave the intended consequence unclear;
* duplicate existing machinery;
* belong to the wrong component, repository, or semantic owner;
* conflict with an existing contract or design principle;
* imply an implementation route before placement is understood;
* omit alternatives, dependencies, invalidation conditions, or important uncertainties.

Sending such an idea directly into evidence-backed scoring wastes expensive repository, design, metrics, and review work because the system may be measuring an idea that has not yet been made coherent or correctly placed.

At the same time, proposal formation must not become a rigid design procedure that forces every idea through the same reasoning path.

## Proposed consequence

Before detailed evaluation begins, an idea can be transformed into a proposal packet whose core proposal is sufficiently coherent and placed that research agents know what they are evaluating.

The formed proposal should establish, as appropriate:

* the problem or opportunity being addressed;
* the intended downstream consequence;
* the high-level mechanism or design hypothesis;
* the likely semantic owner;
* architectural and repository placement;
* affected contracts, capabilities, and boundaries;
* plausible competing placements or alternative approaches;
* dependencies and conflicts;
* known uncertainties;
* invalidation conditions;
* the evidence still needed to evaluate impact, complexity, risk, cost, and alignment.

Formation should preserve uncertainty rather than manufacture certainty where evidence is still missing.

## Relationship to Proposal Packets

The proposal packet is the durable artifact boundary.

Proposal formation is the mechanism that converts an idea into the first meaningful packet state.

A packet may begin with only:

```text
proposal/
  idea.md
```

Formation then produces or enriches:

```text
proposal/
  idea.md
  proposal.md
  placement.md
  dependencies.md
  implementation-notes.md
```

Later evaluation stages add:

```text
proposal/
  evidence/
    design.md
    repo.md
    metrics.md
    history.md
  scoring.md
  decision.md
```

The packet therefore persists across the lifecycle while different stages populate only the information they actually establish.

## Epistemic states

Useful proposal states may include:

```text
idea
→ forming
→ formed
→ placed
→ under-evaluation
→ evaluated
→ roadmap-candidate
→ accepted / deferred / rejected
```

These labels describe what is currently known about the proposal. They must not become a mandatory procedural state machine when evidence permits stages to collapse, reorder, or reopen.

For example, an idea with already-established ownership may move directly from formation to placed. Later evidence may invalidate placement and reopen that part of the packet.

## Formation responsibilities

Proposal formation should answer enough of the following questions to make subsequent evaluation meaningful:

### Objective and consequence

What problem or opportunity exists?

What observable or durable consequence would make the proposal valuable?

Is the proposed consequence actually distinct from the method currently imagined for achieving it?

### Existing machinery

Does the system already provide the required capability?

Would extending an existing primitive preserve the consequence more cleanly than introducing new machinery?

Is the idea actually compensating for an infrastructure, tooling, or documentation failure?

### Placement and ownership

Which component owns the durable semantic consequence?

Which contracts or boundaries would be affected?

Are there credible competing owners?

Does the proposal belong to Work Engine generally, to an adapter, to a consuming project, or to research?

### Alternatives

What materially different approaches could achieve the same consequence?

Does the current proposal unnecessarily collapse valid implementation space?

### Boundaries

What is explicitly outside the proposal?

Which existing invariants must remain true?

Which aspects remain model judgment?

### Uncertainty and invalidation

What assumptions are still unverified?

What evidence would falsify the proposal or its placement?

What questions must the evaluation stage answer before the proposal can become a roadmap candidate?

## Placement consequence

Placement should be established before expensive evaluation when placement materially affects what evidence, metrics, implementation surface, or contracts are relevant.

The goal is not to prove the complete implementation path.

The goal is to establish a defensible answer to:

> Where does the consequence belong, and what existing system boundary should own it?

If placement remains genuinely uncertain, the packet should say so and direct evaluation toward resolving that uncertainty rather than pretending placement has been completed.

## Relationship to Evidence-Backed Evaluation

Proposal formation creates the object that evidence-backed evaluation investigates.

The evaluation stage should not have to reconstruct the proposal itself. It should be able to consume the formed packet and concentrate on producing evidence-backed estimates for impact, complexity, alignment, fan-out, risk, reversibility, validation burden, expected cost, evidence strength, and confidence.

If evaluation falsifies a proposal assumption or placement claim, the relevant packet state should be reopened and revised rather than silently accommodating stale conclusions.

## Context economy

The formation stage should preserve durable proposal consequences, not its entire reasoning transcript.

Later roles should load only the packet sections relevant to their work.

This allows expensive proposal understanding to survive across agents and stages without requiring every agent to ingest the complete brainstorming and research history.

## Key design questions

* What minimum information makes a proposal coherent enough to evaluate?
* What evidence is sufficient to call placement established?
* Which proposal sections are authored during formation versus evaluation?
* How should placement uncertainty be represented without blocking useful research?
* What information should remain speculative until evidence agents validate it?
* How should formation detect that an idea duplicates or extends existing machinery?
* Which packet fields are required product structure, and which should remain flexible descriptive content?

