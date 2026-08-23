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

- describe a symptom rather than the underlying problem;
- leave the intended consequence ambiguous;
- contain several distinct opportunities that should not be implemented together;
- assume machinery that does not exist;
- duplicate an existing capability or pending proposal;
- belong to the wrong architectural owner;
- cross repository or product boundaries;
- collapse implementation choices prematurely;
- ignore competing placements;
- omit dependencies or affected contracts;
- depend on another proposal without making that relationship explicit;
- combine foundational and optional work into one oversized change;
- rely on assumptions that have not been identified.

The missing capability is an intermediate process that **interprets and decomposes an idea, then forms coherent proposals before evaluating them**.

## Proposed consequence

A human and capable model can interactively develop a raw idea until it has yielded zero or more proposal packets that another capable agent can evaluate without first having to reconstruct:

- what problem is actually being addressed;
- what consequences matter;
- whether the idea contains multiple separable proposals;
- where each consequence belongs;
- what each proposal does and does not claim;
- which system boundaries may be affected;
- which alternatives remain credible;
- which proposals depend on, enable, overlap, or conflict with one another;
- which assumptions are uncertain;
- what evidence still needs to be gathered.

The durable results are proposal packets plus explicit relationships among them.

## Governing design relationship

This workflow must follow the same Work Engine distinction between structure and judgment.

It should define the durable properties a formed proposal must expose, the packet artifacts available to hold them, relationships that may be preserved between proposals, available investigation capabilities, and authority boundaries.

It should **not** define a universal sequence of questions or reasoning steps.

Proposal formation is an open semantic task. Different ideas will have different missing information and different natural decomposition. The system should concentrate interaction on the uncertainties that matter for the current idea rather than forcing every idea through the same questionnaire.

## Ideas and proposals are different units

An idea is the shared origin of exploration.

A proposal is a bounded candidate change with a coherent consequence and plausible placement.

They do not have a one-to-one relationship.

```text
idea
  ├─ proposal A
  ├─ proposal B
  ├─ proposal C
  └─ proposal D
```

Proposals derived from the same idea may have different consequences, implementation boundaries, dependencies, costs, evidence requirements, and owners.

Keeping them separate allows each proposal to be evaluated, prioritized, deferred, implemented, rejected, or revised independently.

## Proposal decomposition

Proposal formation should explicitly consider whether the current idea contains multiple independently valuable consequences.

Signals that decomposition may be appropriate include:

- different semantic owners;
- different implementation boundaries;
- different affected contracts;
- different evidence requirements;
- different risk or complexity profiles;
- different information lifetimes;
- one change can succeed without another;
- one change enables another but is useful independently;
- one part is foundational while another is optional;
- one part belongs to research while another belongs to product work;
- parts could reasonably receive different roadmap priorities;
- combining them would make placement or acceptance ambiguous.

The purpose is not to maximize the number of proposals. The purpose is to preserve meaningful implementation and decision boundaries.

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

The originating idea retains shared motivation and exploratory context. Each proposal packet owns the durable context specific to that candidate change.

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

The graph should emerge from proposal understanding rather than being reconstructed when the roadmap is assembled. It becomes decision evidence in its own right.

A proposal with modest direct impact may have high **unlock value** because several valuable proposals depend on it. A proposal with high apparent value may need to wait because an unresolved causal dependency would make implementation premature.

The dependency graph should inform roadmap judgment without becoming a rigid scheduler. Only genuinely causal relationships should constrain ordering.

## Interactive model

The workflow should behave more like a **proposal workbench** than a wizard.

A wizard assumes:

```text
step 1
→ step 2
→ step 3
→ step 4
```

The proposed workbench instead maintains an evolving model:

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
```

The model examines the current state and chooses what deserves attention next.

The interaction should follow the semantic needs of the idea and proposals rather than a predefined route.

## Human interaction

The human should not have to fill out a large proposal form.

The model should do the majority of proposal formation from the original idea, repository evidence when useful, Work Engine design and philosophy, existing proposal packets and relationships, known product boundaries, and current project context.

The model should surface only material uncertainty or meaningful choices.

Useful interactions may include:

- **Reflect** — show the current understanding.
- **Decompose** — identify independently valuable proposals.
- **Merge** — recognize that candidate proposals are actually one inseparable change.
- **Fill gaps** — investigate missing dimensions.
- **Challenge** — look for flaws or false assumptions.
- **Explore placement** — compare plausible semantic owners.
- **Explore alternatives** — identify materially different ways to achieve the consequence.
- **Inspect relationships** — identify dependencies, conflicts, overlap, or enablement.
- **Narrow or expand scope** — adjust proposal boundaries.
- **Inspect assumptions** — expose what is inferred rather than established.
- **Accept current formulation** — preserve current proposal state.
- **Reopen** — revisit previously accepted understanding when new evidence makes it stale.

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

The packet persists. Agent context does not.

Each actor should load only the idea and packet material useful to the current decision.

> **Persistent decision objects, selectively projected context.**

## Working persistence and checkpoints

The packet should be updated during work when newly acquired understanding has durable downstream value.

Proposal formation can involve expensive repository exploration, placement analysis, dependency discovery, or alternative elimination before a formal proposal or handoff exists. Waiting until the end of the stage to persist this work makes interruption unnecessarily expensive.

The workflow therefore distinguishes checkpoint persistence from handoff persistence.

### Checkpoint persistence

During ongoing work, the agent may checkpoint durable consequences when losing the current context would cause meaningful reconstruction cost.

Useful signals include:

- an important placement conclusion becomes supported;
- a competing owner is ruled out for a durable reason;
- expensive repository exploration yields reusable evidence;
- a material assumption is falsified;
- one idea is split into several proposals;
- proposals are merged;
- a causal dependency is discovered;
- an existing proposal is found to overlap or conflict;
- accumulated context is becoming large, fragile, or likely to compact;
- the agent is about to enter a substantially different investigation area;
- a human decision materially changes direction.

These are signals for judgment, not mandatory triggers.

> **Persist by value, not by phase.**

The checkpoint should preserve the smallest durable consequence that future work would otherwise need to reconstruct, not the full reasoning transcript.

### Handoff persistence

Before control passes to another agent, stage, or process, all information required to resume correctly must be durably represented in the packet or another canonical owned artifact.

> **No handoff may be the sole durable owner of information required to resume the proposal lifecycle correctly.**

A handoff is a context projection for the next actor, not the authoritative storage location.

### Resume behavior

After a crash, restart, lost connection, context reset, or agent replacement, a new agent should begin from the latest durable packet state.

It should normally:

- load only the packet sections relevant to the current task;
- inspect unresolved assumptions and current placement;
- check whether decisive evidence has become stale;
- continue from the latest durable proposal understanding;
- reconstruct only what was not already preserved.

The objective is not perfect transcript recovery. It is to avoid repeating expensive semantic work that had already produced durable value.

## `idea.md`

The original idea should remain recognizable rather than being overwritten by later conclusions.

It may contain the originating observation, initial intuition or hypothesis, possible consequences, examples, questions, and brainstorming context worth preserving.

It is explicitly speculative.

An idea is not evidence, a contract, a proposal, or roadmap authority.

## `proposal.md`

Each `proposal.md` represents one current formed proposal.

It should capture enough durable meaning to establish:

- the problem or opportunity;
- the intended consequence;
- the high-level proposed change;
- relationship to the originating idea;
- scope and explicit exclusions;
- sibling concerns separated into other proposals;
- assumptions and uncertainty;
- materially different alternatives;
- invalidation conditions.

The proposal should not unnecessarily prescribe implementation choices that remain open to judgment.

## `placement.md`

Placement deserves its own durable artifact because incorrect placement can make all subsequent evaluation misleading.

It should capture:

- likely semantic owner;
- relevant component or product boundary;
- affected contracts;
- consumers of the proposed consequence;
- competing plausible owners;
- evidence supporting current placement;
- evidence contradicting alternatives;
- unresolved placement uncertainty;
- conditions that would reopen placement.

The purpose is not to determine the implementation path.

The question is:

> **Where does this consequence belong?**

## `dependencies.md`

Proposal relationships deserve durable representation because they affect later evaluation and roadmap judgment.

`dependencies.md` may capture:

- required predecessor proposals;
- proposals enabled by this one;
- conflicts;
- alternatives;
- overlaps;
- shared contracts or implementation surfaces;
- sequencing that is genuinely causal;
- sequencing that is merely convenient and therefore should remain judgment.

Relationships should contain enough explanation to distinguish causal dependency from preferred order.

## Formation versus evidence evaluation

Proposal formation and proposal evaluation are distinct.

### Formation asks

- What does this idea contain?
- Is it one proposal or several?
- What is each proposal?
- What consequence does each intend?
- Where does each appear to belong?
- What would each affect?
- How do the proposals relate?
- What assumptions do they contain?
- What alternatives exist?
- What evidence is still needed?

### Evaluation asks

- Is the problem actually present?
- How strongly does repository evidence support the proposal?
- How well does it align with Work Engine design?
- What do historical metrics say?
- What is the likely impact?
- What is the likely implementation complexity?
- What is the fan-out?
- What is the risk?
- How reversible is it?
- What is the expected cost?
- What is the proposal's dependency or unlock value?
- How confident are these estimates?

Formation creates coherent objects to investigate. Evaluation investigates them.

## Placement before expensive evaluation

Placement should usually be resolved enough to guide evaluation before expensive research begins.

This does not require absolute certainty. Placement uncertainty should be represented explicitly and may itself become a target of evaluation.

What should be avoided is silently assuming placement and spending substantial research effort scoring the wrong implementation surface.

## Proposal decomposition remains revisable

Decomposition is itself a judgment based on current understanding.

Later evidence may show that proposals should be merged, split, superseded, or moved.

Those changes should preserve lineage rather than pretending the earlier formulation never existed. Existing evidence, scores, dependencies, and roadmap references should remain traceable through the change.

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

These represent what is currently known. They are not a mandatory state-transition procedure.

Evidence may cause a proposal to move backward or change identity relationships. That is normal route revision.

## Completion consequence for proposal formation

Proposal formation is sufficiently complete when:

> **The idea has been decomposed to a defensible granularity, and another capable agent can evaluate each resulting proposal's value and feasibility without first having to determine what that proposal means, where its consequence belongs, or which known proposal relationships materially constrain it.**

This is a semantic completion criterion, not a field-completion checklist.

A proposal may still contain uncertainty, but that uncertainty should be explicit enough for evaluation to investigate deliberately.

## Context economy

The workflow should preserve the consequence of proposal reasoning, not its entire transcript.

Useful rejected alternatives, decomposition decisions, placement conclusions, dependencies, assumptions, invalidation evidence, and expensive reusable discoveries should survive when they prevent later reconstruction.

Shared context should remain attached to the originating idea when it genuinely applies to the whole family rather than being copied into every proposal packet.

Exploratory conversation that no longer contributes to future decisions should remain disposable.

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

## Relationship to Work Engine

This planning system sits above Work Engine.

Work Engine answers:

> Given a bounded objective, evidence, capabilities, contracts, and authority, how should the work reach a truthful terminal state?

The proposal system answers earlier questions:

> Which engineering changes are worth turning into bounded objectives?

and:

> Which independently executable changes are contained in the ideas we are considering, and how do those changes relate?

Proposal packets are the durable bridge between those layers.

## Initial implementation direction

The first implementation should prioritize semantic workflow, packet artifacts, proposal-family relationships, dependency representation, and durable checkpoint behavior over sophisticated UI.

A usable first version could operate through model interaction and Markdown files while establishing:

- idea identity;
- proposal-family identity;
- proposal decomposition semantics;
- packet identity and ownership;
- proposal formation semantics;
- placement representation;
- explicit uncertainty;
- dependency and relationship representation;
- split/merge lineage;
- incremental checkpoint persistence;
- handoff persistence;
- resume behavior;
- reopening and revision;
- selective context loading;
- handoff to evidence-backed evaluation.

A richer visual workbench may follow once actual formation sessions reveal which interactions and states deserve dedicated controls.

## Design risks

### Assuming one idea equals one proposal

This would force unrelated consequences into oversized implementation units and weaken placement, scoring, and roadmap decisions.

### Fragmenting ideas excessively

Not every distinguishable concern deserves an independent proposal.

### Turning formation into a questionnaire

A fixed list of required questions would recreate procedural narrowing.

### Turning checkpointing into a timer or phase ritual

Checkpointing should preserve newly valuable durable understanding when reconstruction would be costly. It should not require arbitrary save intervals.

### Treating handoff summaries as durable state

If important resume information exists only in a handoff message, interruption can destroy accepted work.

### Turning dependencies into mandatory workflow

Only causally necessary edges should constrain execution order.

### Treating completeness as certainty

A formed proposal can remain uncertain.

### Premature implementation planning

Formation should establish proposals and placement without locking future builders into implementation paths.

### Duplicating shared evidence

Evidence applying to the whole proposal family should not be copied unnecessarily into every sibling packet.

### Duplicating existing proposals

Formation should recognize overlap, extension, alternatives, and prior rejected work when useful.

### Treating current decomposition as permanent truth

Splits, merges, and supersession should preserve provenance.

### Letting packet files become another policy layer

Packets record the current proposal model and its evidence. They do not redefine Work Engine contracts.

## Key design questions

- What is the minimum durable structure required for a formed proposal?
- What signals indicate that an idea contains multiple proposals?
- What signals indicate that candidate proposals should remain one?
- How should proposal-family identity be represented?
- Which relationships belong in the pending-proposal graph?
- How do we distinguish causal dependencies from useful but non-binding relationships?
- How should splits, merges, and supersession preserve lineage?
- Which packet files have distinct information lifetimes or consumers?
- How should agents determine which proposal uncertainty deserves attention next?
- How should placement confidence and competing owners be represented?
- When should repository investigation occur during formation rather than evaluation?
- When has intermediate work become valuable enough to checkpoint?
- What information must be durable before handoff?
- How should interrupted work resume from packet state?
- How are stale proposal conclusions identified and reopened?
- Which packet artifacts should later supervisors, builders, reviewers, or evidence agents receive?
- How should the system detect duplication with existing proposals or machinery?
- How should shared evidence be attached to an idea family without unnecessary duplication?
- At what point is each proposal sufficiently formed to justify expensive evidence-backed evaluation?

## Expected value

The workflow should reduce:

- repeated reconstruction of proposal context;
- loss of expensive research after crashes, resets, or lost connections;
- expensive evaluation of poorly formed ideas;
- oversized proposals containing unrelated consequences;
- implementation against incorrect placement;
- dependency reconstruction during roadmap planning;
- roadmap compression loss;
- repeated rediscovery of rejected alternatives;
- duplicate proposals;
- unnecessary context carried between planning stages.

It should improve:

- proposal granularity;
- proposal clarity;
- placement quality;
- dependency visibility;
- interruption resilience;
- resume quality;
- evidence targeting;
- prioritization quality;
- identification of high-unlock foundational work;
- implementation handoff;
- long-lived engineering memory;
- token and context efficiency.

The deeper goal is not to automate creativity.

It is to give creative engineering ideas a durable structure that allows them to **fan out into the right implementation units, preserve valuable work as it is discovered, retain their relationships, and let later agents reason about them without losing the understanding that made the original idea valuable.**
