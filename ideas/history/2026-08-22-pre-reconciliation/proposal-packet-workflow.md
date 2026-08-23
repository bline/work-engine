# Interactive Idea-to-Proposal Workflow

## Idea

Introduce an interactive, model-centered workflow that turns lightweight ideas into coherent, architecturally placed proposal packets suitable for later evidence-backed evaluation and roadmap selection.

An idea may produce one proposal, several related proposals, or no viable proposal.

The workflow should preserve the useful freedom of brainstorming while progressively converting worthwhile ideas into durable engineering decision objects.

It is intended to become the first planning layer above Work Engine execution.

## Motivation

Ideas currently begin as inexpensive exploratory discussions and are saved as simple Markdown files.

That is useful because ideation should remain generative and inexpensive. A raw idea does not need to prove that it is correct.

However, a raw idea is not yet suitable for expensive repository analysis, metrics research, prioritization, or implementation.

It may:

* describe a symptom rather than the underlying problem;
* leave the intended consequence ambiguous;
* contain several distinct opportunities that should not be implemented together;
* assume machinery that does not exist;
* duplicate an existing capability or pending proposal;
* belong to the wrong architectural owner;
* cross repository or product boundaries;
* collapse implementation choices prematurely;
* ignore competing placements;
* omit dependencies or affected contracts;
* depend on another proposal without making that relationship explicit;
* combine foundational and optional work into one oversized change;
* rely on assumptions that have not been identified;
* contain useful intuition without enough structure for another agent to evaluate it correctly.

The missing capability is an intermediate process that **interprets and decomposes an idea, then forms coherent proposals before evaluating them**.

## Proposed consequence

A human and capable model can interactively develop a raw idea until it has yielded zero or more proposal packets that another capable agent can evaluate without first having to reconstruct:

* what problem is actually being addressed;
* what consequences matter;
* whether the idea contains multiple separable proposals;
* where each consequence belongs;
* what each proposal does and does not claim;
* which system boundaries may be affected;
* which alternatives remain credible;
* which proposals depend on, enable, overlap, or conflict with one another;
* which assumptions are uncertain;
* and what evidence still needs to be gathered.

The durable results are proposal packets plus explicit relationships among them.

## Governing design relationship

This workflow must follow the same Work Engine distinction between structure and judgment.

It should define:

* the durable properties a formed proposal must expose;
* the packet artifacts available to hold them;
* the relationships that may be preserved between proposals;
* the evidence and repository capabilities available to investigate uncertainty;
* the authority boundaries of participating agents.

It should **not** define a universal sequence of questions or reasoning steps.

Proposal formation is an open semantic task. Different ideas will have different missing information and different natural decomposition.

The system should concentrate interaction on the uncertainties that matter for the current idea rather than forcing every idea through the same questionnaire.

## Ideas and proposals are different units

An idea is the shared origin of exploration.

A proposal is a bounded candidate change with a coherent consequence and plausible placement.

They do not have a one-to-one relationship.

A single idea may result in:

```text
idea
  ├─ proposal A
  ├─ proposal B
  ├─ proposal C
  └─ proposal D
```

For example, an idea about preserving engineering decision context may reveal distinct proposals for:

* a durable proposal-packet artifact;
* an interactive idea-to-proposal workflow;
* evidence-backed proposal evaluation;
* a proposal-backed roadmap;
* closed-loop learning from implementation metrics.

These proposals share an origin but have different consequences, implementation boundaries, dependencies, costs, and potentially different owners.

Keeping them separate allows each proposal to be evaluated, prioritized, deferred, implemented, rejected, or revised independently.

## Proposal decomposition

Proposal formation should explicitly consider whether the current idea contains multiple independently valuable consequences.

Signals that decomposition may be appropriate include:

* different semantic owners;
* different implementation boundaries;
* different affected contracts;
* different evidence requirements;
* different risk or complexity profiles;
* different information lifetimes;
* one change can succeed without another;
* one change enables another but is useful independently;
* one part is foundational while another is optional;
* one part belongs to research while another belongs to product work;
* parts could reasonably receive different roadmap priorities;
* combining them would make placement or acceptance ambiguous.

The purpose is not to maximize the number of proposals.

The purpose is to preserve meaningful implementation boundaries.

A large idea should remain one proposal when its consequences are genuinely inseparable.

## Proposal families

Proposals derived from a common originating idea form a **proposal family**.

A possible representation is:

```text
ideas/
  proposal-system/
    idea.md
    proposals/
      proposal-packet/
      idea-to-proposal-workflow/
      evidence-backed-evaluation/
      proposal-backed-roadmap/
      closed-loop-learning/
```

The exact filesystem representation remains an implementation choice, but the semantic relationship should survive:

```text
proposal
→ derived-from
→ idea
```

The originating idea retains the shared motivation and exploratory context.

Each proposal packet owns the durable context specific to that candidate change.

This prevents shared brainstorming history from being duplicated into every packet while still allowing later agents to recover why sibling proposals exist.

## Proposal relationships and dependency graph

Proposal formation should preserve meaningful relationships discovered between proposals.

Examples include:

```text
depends-on
enables
blocks
conflicts-with
overlaps-with
alternative-to
supersedes
derived-from
related-to
```

These relationships create a natural graph over pending engineering work.

For example:

```text
proposal packet
      ↓ enables
idea-to-proposal workflow
      ↓ enables
evidence-backed evaluation
      ↓ enables
proposal-backed roadmap
      ↓ provides execution input to
Work Engine
      ↓ produces evidence for
closed-loop learning
      └──────── improves ────────→ evidence-backed evaluation
```

This graph should emerge from proposal understanding rather than being manually reconstructed when the roadmap is assembled.

It becomes useful decision evidence in its own right.

A proposal with modest direct impact may have high **unlock value** because several valuable proposals depend on it.

A proposal with high apparent value may need to wait because an unresolved dependency would make implementation premature.

The dependency graph therefore becomes an input to later portfolio and roadmap judgment.

It should not become a rigid scheduler.

## Interactive model

The workflow should behave more like a **proposal workbench** than a wizard.

A wizard assumes:

```text
step 1
→ step 2
→ step 3
→ step 4
```

The proposed workbench instead maintains an evolving model of the idea and its candidate proposals:

```text
IDEA

candidate proposal A
  problem       established
  consequence   established
  placement     probable
  dependencies  proposal B
  unknowns      two

candidate proposal B
  problem       established
  consequence   partial
  placement     uncertain
  alternatives  three

candidate proposal C
  consequence   duplicate of existing machinery?
  status        needs investigation
```

The model examines the current state and chooses what deserves attention next.

For one idea, decomposition may be the important question.

For another, placement may be obvious and impact unclear.

For another, the proposal may duplicate existing machinery.

For another, almost everything may already be known from prior evidence.

The interaction should follow the semantic needs of the idea and proposals rather than a predefined route.

## Human interaction

The human should not have to fill out a large proposal form.

The model should do the majority of proposal formation from:

* the original idea;
* current repository evidence when useful;
* Work Engine design and philosophy;
* existing proposal packets;
* existing proposal relationships;
* known product boundaries;
* available project context.

The model should surface only material uncertainty or meaningful choices.

Useful interactions may include:

* **Reflect** — show the model's current understanding of the idea.
* **Decompose** — identify independently valuable proposals within the idea.
* **Merge** — recognize that candidate proposals are actually one inseparable change.
* **Fill gaps** — investigate missing proposal dimensions.
* **Challenge** — actively look for flaws or false assumptions.
* **Explore placement** — compare plausible semantic owners.
* **Explore alternatives** — identify materially different ways of achieving a consequence.
* **Inspect relationships** — identify dependencies, conflicts, overlap, or enablement among proposals.
* **Narrow or expand scope** — adjust proposal boundaries.
* **Inspect assumptions** — expose what is currently inferred rather than established.
* **Accept current formulation** — preserve the current proposal state.
* **Reopen** — revisit a previously accepted part when new evidence makes it stale.

These are capabilities, not mandatory phases.

## Proposal packet relationship

The proposal packet is the durable object around which proposal-specific interaction occurs.

An idea begins independently:

```text
ideas/
  <idea>/
    idea.md
```

Formation may discover one or more proposal packets:

```text
ideas/
  <idea>/
    idea.md
    proposals/
      <proposal-a>/
        proposal.md
        placement.md
        dependencies.md
        implementation-notes.md
      <proposal-b>/
        proposal.md
        placement.md
        dependencies.md
        implementation-notes.md
```

Later evaluation may enrich each proposal independently:

```text
<proposal>/
  evidence/
    design.md
    repo.md
    metrics.md
    history.md
  scoring.md
  decision.md
```

The packet persists.

Agent context does not.

Each agent or interaction should load only the idea and packet material useful to its current decision.

> **Persistent decision objects, selectively projected context.**

## `idea.md`

The original idea should remain recognizable rather than being overwritten by later conclusions.

It may contain:

* the originating observation;
* the initial intuition or hypothesis;
* possible consequences;
* examples;
* questions;
* brainstorming context worth preserving.

It is explicitly speculative.

An idea is not evidence, a contract, a proposal, or roadmap authority.

The idea may remain valuable even after every proposal currently derived from it has been rejected or completed.

Future evidence may reveal another proposal latent in the original idea.

## `proposal.md`

Each `proposal.md` represents one current formed proposal.

It should capture enough durable meaning to answer:

### Problem or opportunity

What appears to be wrong, missing, expensive, risky, or newly possible?

### Intended consequence

What would become better if this proposal succeeds?

Prefer the property to be established over a prescribed activity.

### Proposal

What change to the system is being proposed at a sufficiently high level to evaluate it?

The proposal should not unnecessarily prescribe implementation choices that remain open to judgment.

### Relationship to originating idea

Which idea produced this proposal?

Which part of the larger idea does this proposal capture?

### Scope and boundaries

What does this proposal include?

What does it explicitly not include?

Which sibling concerns have been separated into their own proposals?

### Assumptions and uncertainty

What is currently believed but not yet established?

What could materially change the proposal?

### Alternatives

What substantially different approaches remain plausible?

### Invalidation conditions

What evidence would cause the proposal to be rejected, substantially revised, merged with another proposal, split further, or moved elsewhere?

## `placement.md`

Placement deserves its own durable artifact because incorrect placement can make all subsequent evaluation misleading.

It should capture:

* likely semantic owner;
* relevant component or product boundary;
* affected contracts;
* consumers of the proposed consequence;
* competing plausible owners;
* reasons supporting the current placement;
* evidence contradicting alternatives;
* unresolved placement uncertainty;
* conditions that would reopen placement.

The purpose is not to determine the implementation path.

The question is:

> **Where does this consequence belong?**

A proposal should not undergo expensive detailed scoring against an implementation surface that has not been plausibly established.

## `dependencies.md`

Proposal relationships deserve durable representation because they affect later evaluation and roadmap judgment.

`dependencies.md` may capture:

* required predecessor proposals;
* proposals enabled by this one;
* conflicts;
* alternatives;
* overlaps;
* shared contracts or implementation surfaces;
* sequencing that is genuinely causal;
* sequencing that is merely convenient and therefore should remain judgment.

Relationships should include enough explanation to distinguish causal dependency from historical or preferred order.

For example:

```text
depends-on: proposal-packet

reason:
The workflow requires a durable packet contract to exist before it can
produce packet state with defined ownership.

dependency-type:
causal
```

versus:

```text
related-to: proposal-backed-roadmap

reason:
Both consume proposal metadata, but either can be implemented independently.

dependency-type:
non-causal
```

This avoids turning the emerging graph into another procedural workflow.

## Formation versus evidence evaluation

Proposal formation and proposal evaluation are distinct.

### Formation asks

* What does this idea contain?
* Is it one proposal or several?
* What is each proposal?
* What consequence does each intend?
* Where does each appear to belong?
* What would each affect?
* How do the proposals relate?
* What assumptions do they contain?
* What alternatives exist?
* What evidence is still needed?

### Evaluation asks

* Is the problem actually present?
* How strongly does repository evidence support the proposal?
* How well does it align with Work Engine design?
* What do historical metrics say?
* What is the likely impact?
* What is the likely implementation complexity?
* What is the fan-out?
* What is the risk?
* How reversible is it?
* What is the expected cost?
* What is the proposal's dependency or unlock value?
* How confident are these estimates?

Formation creates coherent objects to investigate.

Evaluation investigates them.

## Placement before expensive evaluation

Placement should usually be resolved enough to guide evaluation before expensive research begins.

This does not require absolute certainty.

A packet may say:

```text
placement: probable
confidence: moderate

preferred owner:
proposal subsystem

credible alternative:
campaign supervisor

unresolved question:
whether proposal state must survive campaign creation
```

The evaluation machinery can then explicitly investigate that uncertainty.

What should be avoided is silently assuming placement and spending substantial research effort scoring the wrong implementation surface.

## Proposal decomposition remains revisable

Decomposition is itself a judgment based on current understanding.

Later evidence may show that:

```text
proposal A + proposal B
→ actually one inseparable consequence
```

or:

```text
proposal C
→ contains two independently valuable consequences
→ split into proposal C1 and C2
```

Those changes should preserve lineage rather than pretending the earlier formulation never existed.

A split or merge should retain enough provenance for later consumers to understand what happened to existing evidence, scores, dependencies, and roadmap references.

## Epistemic state

The idea/proposal system may expose descriptive states such as:

```text
idea
forming
decomposed
formed
placement-uncertain
placed
under-evaluation
evaluated
roadmap-candidate
accepted
deferred
rejected
superseded
merged
split
```

These represent what is currently known.

They are not a mandatory state-transition procedure.

Evidence may cause a proposal to move backward or change identity relationships.

For example:

```text
placed
→ new repository evidence
→ placement-invalidated
→ placement investigation
→ newly placed
```

or:

```text
formed proposal
→ evidence reveals independent consequence
→ split
→ two formed proposals
```

That is normal route revision.

## Completion consequence for proposal formation

Proposal formation is sufficiently complete when:

> **The idea has been decomposed to a defensible granularity, and another capable agent can evaluate each resulting proposal's value and feasibility without first having to determine what that proposal means, where its consequence belongs, or which known proposal relationships materially constrain it.**

This is a semantic completion criterion, not a field-completion checklist.

A proposal may still contain uncertainty.

That uncertainty should be explicit enough that evaluation can investigate it deliberately.

## Context economy

The workflow should preserve the consequence of proposal reasoning, not its entire transcript.

Useful rejected alternatives, decomposition decisions, placement conclusions, dependencies, assumptions, and invalidation evidence should survive when they prevent later reconstruction.

Shared context should remain attached to the originating idea when it genuinely applies to the whole family rather than being copied into every proposal packet.

Exploratory conversation that no longer contributes to future decisions should not automatically become durable state.

Each file should exist because some future consumer benefits from that information lifetime.

## Relationship to the future planning system

The intended larger flow is:

```text
brainstorm
    ↓
idea.md
    ↓
interactive interpretation + decomposition
    ↓
proposal family
    ├─ proposal packet A
    ├─ proposal packet B
    └─ proposal packet C
    ↓
placement + dependency understanding
    ↓
evidence-backed evaluation per proposal
    ↓
evaluated proposal portfolio
    ↓
roadmap selection informed by value + dependencies
    ↓
thin roadmap entries referencing packets
    ↓
Work Engine campaigns
    ↓
execution receipts + observed metrics
    ↓
future calibration and learning
```

The roadmap eventually becomes a compact portfolio of selected proposal references rather than the place where expensive design context is compressed.

The proposal relationship graph becomes available to roadmap judgment without the roadmap having to reconstruct it.

## Relationship to Work Engine

This planning system sits above Work Engine.

Work Engine answers:

> Given a bounded objective, evidence, capabilities, contracts, and authority, how should the work reach a truthful terminal state?

The proposal system answers earlier questions:

> Which engineering changes are worth turning into bounded objectives?

and:

> Which independently executable changes are actually contained in the ideas we are considering, and how do those changes relate?

Proposal packets are the durable bridge between those layers.

## Initial implementation direction

The first implementation should prioritize the semantic workflow, packet artifacts, proposal-family relationships, and dependency representation over sophisticated UI.

A usable first version could operate through model interaction and Markdown files while establishing:

* idea identity;
* proposal-family identity;
* proposal decomposition semantics;
* packet identity;
* packet file ownership;
* proposal formation semantics;
* placement representation;
* explicit uncertainty;
* dependency and relationship representation;
* split/merge lineage;
* reopening/revision behavior;
* selective context loading;
* handoff to evidence-backed evaluation.

A richer visual workbench may follow once actual proposal-formation sessions reveal which interactions, relationships, and states deserve dedicated controls.

## Design risks

### Assuming one idea equals one proposal

This would force unrelated consequences into oversized implementation units and weaken placement, scoring, and roadmap decisions.

### Fragmenting ideas excessively

Not every distinguishable concern deserves an independent proposal.

Decomposition should preserve meaningful implementation and decision boundaries, not maximize granularity.

### Turning formation into a questionnaire

A fixed list of required questions would recreate procedural narrowing.

The model should determine which uncertainties matter.

### Turning dependencies into mandatory workflow

A proposal relationship graph describes real dependencies and useful relationships.

Only causally necessary edges should constrain execution order.

### Treating completeness as certainty

A formed proposal can remain uncertain.

The requirement is that uncertainty is represented clearly enough to evaluate.

### Premature implementation planning

Formation should establish proposals and placement without locking future builders into implementation paths.

### Duplicating shared evidence

Evidence applying to the whole proposal family should not be copied unnecessarily into each sibling packet.

### Duplicating existing proposals

Formation should inspect existing proposal families when useful and recognize overlap, extension, alternatives, or prior rejected work.

### Treating current decomposition as permanent truth

Proposal boundaries may change as understanding improves.

Splits, merges, and supersession should preserve provenance.

### Letting packet files become another policy layer

Packets record the current proposal model and its evidence.

They do not redefine Work Engine contracts.

## Key design questions

* What is the minimum durable structure required for a formed proposal?
* What signals indicate that an idea contains multiple proposals?
* What signals indicate that candidate proposals should remain one?
* How should proposal-family identity be represented?
* Which relationships belong in the pending-proposal graph?
* How do we distinguish causal dependencies from useful but non-binding relationships?
* How should splits, merges, and supersession preserve lineage?
* Which packet files have distinct information lifetimes or consumers?
* How should agents determine which proposal uncertainty deserves attention next?
* How should placement confidence and competing owners be represented?
* When should repository investigation occur during formation rather than evaluation?
* How are stale proposal conclusions identified and reopened?
* Which packet artifacts should a later supervisor receive?
* Which should a builder receive?
* Which proposal context should remain available only on demand?
* How should the system detect duplication with existing proposals or existing machinery?
* How should shared evidence be attached to an idea family without unnecessary duplication?
* At what point is each proposal sufficiently formed to justify expensive evidence-backed evaluation?

## Expected value

The workflow should reduce:

* repeated reconstruction of proposal context;
* expensive evaluation of poorly formed ideas;
* oversized proposals containing unrelated consequences;
* implementation against incorrect placement;
* dependency reconstruction during roadmap planning;
* roadmap compression loss;
* repeated rediscovery of rejected alternatives;
* duplicate proposals;
* unnecessary context carried between planning stages.

It should improve:

* proposal granularity;
* proposal clarity;
* placement quality;
* dependency visibility;
* evidence targeting;
* prioritization quality;
* identification of high-unlock foundational work;
* implementation handoff;
* long-lived engineering memory;
* token and context efficiency.

The deeper goal is not to automate creativity.

It is to give creative engineering ideas a durable structure that allows them to **fan out into the right implementation units**, preserve their relationships, and let later agents reason about them without losing the understanding that made the original idea valuable.

