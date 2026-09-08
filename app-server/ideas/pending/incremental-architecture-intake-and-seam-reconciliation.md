# Idea: Incremental Architecture Intake and Seam Reconciliation

## Status

Architectural idea for pre-planning integration of related design changes.

This document does not accept any constituent idea, authorize implementation, amend the current architecture, establish a permanent architecture graph, or replace proposal formation and planning.

Its purpose is to define a bounded intake process through which multiple related ideas may be integrated into one coherent prospective architectural model before independent proposal formation or implementation planning begins.

---

## Summary

Work Engine currently admits ideas individually.

That is appropriate when ideas are independent. It becomes costly when several pending ideas affect overlapping architectural boundaries.

If related ideas enter proposal formation one at a time, each proposal may make locally reasonable decisions about ownership, artifacts, lifecycle, authority, or integration based only on the architecture visible at that moment. A later idea may then expose an overlap or contradiction that requires earlier proposal assumptions to be reopened.

The proposed alternative is:

> **Integrate related ideas incrementally against one session-scoped prospective architecture before planning them independently.**

An architecture intake session begins from one exact implemented architecture baseline.

Each idea entering that session is integrated into the current session seam map. The map therefore accumulates a coherent prospective architectural understanding rather than treating each idea as an isolated proposal candidate.

Conceptually:

```text
implemented architecture A
        |
        v
open architecture intake session
        |
        +-- idea 1
        |      |
        |      v
        |   seam map S1
        |
        +-- idea 2
        |      |
        |      v
        |   reconcile against S1
        |      |
        |      v
        |   seam map S2
        |
        +-- idea 3
               |
               v
            reconcile against S2
               |
               v
            seam map S3
        |
        v
coherent prospective architecture
        |
        v
freeze intake session
        |
        v
proposal decomposition / planning
        |
        v
implementation / review / acceptance
        |
        v
implemented architecture B
```

A new architecture-forming intake session should ordinarily begin only after the preceding session has reached an accepted implemented state.

This keeps every session grounded in one truthful implemented architecture rather than forcing intake to reason over several speculative future architectures.

---

# 1. Problem

## 1.1 Single-idea intake creates local architectural optimization

A single idea may be internally coherent while still sharing important seams with other pending ideas.

Examples include:

- two ideas that both introduce a concept of runtime realization;
- one idea that introduces a new artifact while another changes the artifact's lifecycle;
- multiple ideas that independently assign ownership to the same semantic fact;
- one idea that assumes a stable runtime boundary that another intends to replace;
- several ideas that rely on the same role hierarchy, capability contract, history model, or projection mechanism.

When these ideas are formed independently, each proposal may choose a reasonable local representation.

The integration problem appears later.

For example:

```text
idea A
→ proposal A
→ chooses ownership boundary X

idea B
→ proposal B
→ discovers semantic owner should be Y

idea C
→ requires both A and B
→ planning must reconcile X and Y
```

The cost is not merely implementation rework.

It can require reopening:

- authority decisions;
- schema ownership;
- lifecycle semantics;
- artifact identity;
- revision behavior;
- invalidation consequences;
- reconstruction assumptions;
- proposal boundaries; and
- implementation plans already formed around stale assumptions.

---

## 1.2 Planning is too late to discover family-level structure

Planning should determine how an accepted architectural direction is realized.

It should not need to repeatedly reconstruct whether several separately formed ideas actually describe one architecture.

If cross-cutting ownership and seam questions remain unresolved when planning starts, planners either:

1. repeat architecture synthesis independently;
2. silently make decisions outside their intended authority; or
3. return upstream after substantial work has already depended on the unresolved boundary.

The desired architecture should therefore become coherent before implementation planning begins.

---

## 1.3 Simultaneous prospective baselines create unnecessary complexity

Allowing a second architecture intake session to begin while the first remains planned or partially implemented creates a more difficult state model.

A new intake would have to distinguish:

```text
implemented architecture
+
accepted-but-unimplemented architecture
+
partially implemented architecture
+
review-modified architecture
+
prospective new architecture
```

The later intake would then depend on assumptions about changes that do not yet truthfully exist.

Review, implementation discovery, or rejection could invalidate those assumptions.

The initial design should avoid this complexity.

---

# 2. Core Proposal

Introduce a session-scoped **Architecture Intake Session**.

Each session:

1. binds to an exact implemented architecture baseline;
2. accepts one idea at a time;
3. integrates each idea into a revisioned seam map;
4. reconciles overlaps, contradictions, shared ownership, and lifecycle relationships as they appear;
5. accumulates one prospective architectural model for the session;
6. preserves unresolved questions explicitly;
7. freezes when the idea set is ready to advance;
8. decomposes the coherent result into proposal/planning inputs; and
9. remains historically attributable after implementation changes the architecture.

The seam map is therefore not a permanent global architecture database.

It is a **working structural projection of the architecture being formed during one intake session**.

---

# 3. Session Baseline

Every architecture intake session binds to one exact implemented architecture revision.

Conceptually:

```text
ArchitectureIntakeSession

baseline:
    architecture_revision: A17
    repository_revision: ...
    relevant generated_state: ...
    accepted contracts: ...
```

The session may integrate prospective ideas against that baseline.

It must distinguish clearly between:

```text
implemented structure
```

and:

```text
prospective session structure
```

The session must never present a prospective relationship as already implemented.

---

# 4. Incremental Integration

Each new idea is integrated against the **current session seam map**, not independently against the original baseline.

For example:

```text
S0 = implemented architecture A17

integrate(I1, S0)
    -> S1

integrate(I2, S1)
    -> S2

integrate(I3, S2)
    -> S3
```

This means idea 3 is evaluated against:

```text
implemented architecture
+
prospective consequences of idea 1
+
prospective consequences of idea 2
```

rather than against the baseline alone.

The intake session therefore develops a progressively more complete architectural understanding.

---

# 5. Seam Map

## 5.1 Purpose

The seam map describes where the constituent ideas meet.

A seam is not merely a dependency.

It identifies an architectural relationship through which meaning, authority, state, evidence, invalidation, or lifecycle consequences cross from one concern to another.

A seam may identify:

- shared semantic concepts;
- producer/consumer relationships;
- ownership boundaries;
- authority boundaries;
- artifact relationships;
- projection relationships;
- lifecycle coupling;
- invalidation propagation;
- revision correspondence;
- implementation dependencies;
- shared decisions; or
- incompatible assumptions.

---

## 5.2 Candidate seam relations

The initial seam vocabulary should remain small and may evolve with evidence.

Useful candidate relations include:

### SUPPLIES

One concern produces an input consumed by another.

```text
implementation compiler
    SUPPLIES
Plan IR
```

### CONSTRAINS

One concern limits the valid state space of another.

```text
role manifest
    CONSTRAINS
child capability request
```

### DERIVES

One artifact or state is a rebuildable projection of another.

```text
durable history
    DERIVES
role continuation projection
```

### BINDS

One mechanism selects or materializes an exact realization.

```text
runtime admission
    BINDS
provider + harness realization
```

### INVALIDATES

A change in one concern may stale dependent state elsewhere.

```text
decision revision
    INVALIDATES
dependent implementation plan
```

### CORRESPONDS

Two structures represent related semantic objects across revisions or layers.

### SHARES_DECISION

Two concerns depend on a single architecture decision that must have one owner.

### CONFLICTS

Two prospective structures currently require incompatible meanings, authorities, or consequences.

The exact taxonomy should be validated through actual intake sessions rather than assumed complete.

---

# 6. Seam Record

A seam should carry enough information to prevent later consumers from reconstructing its meaning from prose.

Conceptually:

```yaml
seam:
  id: plan-ir.execution-strategy

  left:
    concern: structural-plan-ir
    semantic_owner: plan-representation

  right:
    concern: execution-strategy-selection
    semantic_owner: routing-policy

  relation: supplies

  shared_semantics:
    - remaining_judgment
    - verification_strength
    - execution_characterization

  authority:
    characterization: planner
    scoring: deterministic_service
    strategy_decision: authorized_owner

  invariants:
    - scoring cannot revise plan meaning
    - routing cannot expand role capability authority

  lifecycle:
    plan_revision:
      consequence: rescore

    scoring_policy_revision:
      consequence: recompute_without_rewriting_plan

  status: aligned
```

The schema should encode only distinctions required to preserve architectural meaning.

It should not become a universal architecture modeling language.

---

# 7. Reconciliation

## 7.1 Objective

Reconciliation determines whether the prospective architectural model remains coherent as a new idea enters the session.

For every material seam, intake should determine:

1. whether both sides refer to the same semantic concept;
2. whether there is exactly one authoritative owner;
3. whether information and authority flow in the correct direction;
4. whether representation differences are intentional;
5. what changes invalidate dependent assumptions;
6. whether the two ideas may proceed independently; and
7. whether a shared decision must be resolved above both ideas.

---

## 7.2 Reconciliation states

Candidate states include:

### ALIGNED

The ideas assign compatible semantics, ownership, and lifecycle behavior.

No architecture decision is required.

### TERMINOLOGY

The ideas describe the same meaning using inconsistent vocabulary.

Reconcile naming without changing semantics.

### OWNERSHIP

Multiple ideas appear to claim authority over the same semantic concept.

Choose one owner. Other concerns become consumers, projections, or derived mechanisms.

### REPRESENTATION

The semantic owner is compatible, but representations or schema expectations differ.

Establish the canonical representation or an explicit translation/projection boundary.

### LIFECYCLE

Static structure agrees, but revision, invalidation, recovery, or replacement semantics conflict.

### ORDERING

The ideas are compatible, but one architectural decision or artifact must exist before another can be formed truthfully.

### COUPLED_DECISION

Several ideas depend on one architectural question that none should independently resolve.

The decision is promoted to the intake-session reconciliation boundary.

### FUNDAMENTAL_CONFLICT

The ideas require mutually exclusive architecture.

At least one idea must change, be narrowed, be deferred, or be rejected.

### UNRESOLVED

The intake session lacks sufficient evidence to reconcile the seam truthfully.

Uncertainty remains explicit.

---

# 8. Reconciliation Does Not Mean Premature Design

The intake session should resolve only the structure required for independent proposal formation to remain coherent.

It may determine:

- semantic ownership;
- authority relationships;
- artifact boundaries;
- shared invariants;
- lifecycle ownership;
- projection relationships;
- dependency consequences;
- invalidation direction;
- proposal-family boundaries; and
- which questions require joint disposition.

It should not ordinarily determine:

- exact repository transformations;
- file placement unless placement is itself architectural;
- implementation sequencing unless causality requires it;
- local mechanism choices;
- builder-level design;
- specific code structure;
- executor-specific implementation details; or
- implementation routes that remain legitimately open.

The intake session defines the prospective architectural space.

Planning determines how that space is realized.

---

# 9. Idea Revision During Intake

Ideas should not be treated as immutable just because they entered intake.

A later idea may reveal that an earlier prospective boundary was incomplete or incorrect.

For example:

```text
idea 2
    establishes owner X

idea 4
    reveals that X cannot truthfully own the shared semantic state

reconciliation
    moves ownership to Y

idea 2 successor
    retains its valid consequences
    but no longer owns the disputed state
```

The session should preserve:

- the original idea revision;
- the observation that exposed the seam problem;
- the reconciliation decision;
- the successor prospective interpretation; and
- which downstream seams changed.

The prior state remains historical.

It is not rewritten.

---

# 10. Session Seam-Map Revisions

The seam map should be immutable by revision.

Conceptually:

```text
baseline A17
    |
    v
S0
    |
    + idea I1
    v
S1
    |
    + idea I2
    v
S2
    |
    + idea I3
    v
S3
    |
    reconciliation R7
    v
S4
```

Each revision should preserve:

- predecessor identity;
- baseline architecture identity;
- integrated idea revisions;
- seam additions/removals/changes;
- reconciliation decisions;
- unresolved questions;
- producer;
- evidence cutoff; and
- revision identity.

The seam map therefore has reconstructable history without requiring every consumer to ingest the entire intake conversation.

---

# 11. Prospective Architecture

The accumulating session state may be understood as a **prospective architecture projection**.

It consists of:

```text
implemented baseline
+
integrated prospective changes
+
resolved seams
+
shared ownership decisions
+
shared invariants
+
lifecycle relationships
+
unresolved architecture questions
```

The prospective architecture is not yet implementation truth.

It states what the architecture is intended to become if the session advances and its constituent proposals are accepted and successfully realized.

---

# 12. Handoff to Proposal Formation and Planning

When the intake session is sufficiently coherent, it freezes.

The handoff should contain at least:

## 12.1 Frozen prospective architecture

The structural consequence the session intends to realize.

## 12.2 Reconciled seam map

The exact shared relationships, ownership boundaries, lifecycle consequences, and unresolved seams that constrain downstream formation.

## 12.3 Reconciliation set

Versioned architecture decisions that resolved coupled or conflicting seams.

## 12.4 Proposal decomposition

The independently formable bodies of work required to realize the prospective architecture.

## 12.5 Shared constraints

The exact architectural decisions downstream proposals must preserve.

A proposal may discover evidence that invalidates one of these assumptions.

It may not silently reinterpret a shared seam merely because another local design is convenient.

---

# 13. Proposal Decomposition

Proposal decomposition should occur after seam reconciliation because the seam map may reveal that ideas do not correspond one-to-one with proposals.

Possible outcomes include:

```text
idea A
idea B
    -> one proposal because they share one inseparable semantic owner

idea C
    -> independent proposal

idea D
    -> split into two proposals because it spans unrelated owners

idea E
    -> absorbed as a consequence of an existing proposal

idea F
    -> deferred because its seam depends on an unresolved architecture decision
```

Proposal identity should therefore follow coherent architectural responsibility rather than idea-document boundaries.

---

# 14. Relationship to Hierarchical Planning

The architecture intake session operates above implementation planning.

Conceptually:

```text
ideas
    |
    v
architecture intake
    |
    v
prospective architecture + seam map
    |
    v
proposal formation / acceptance
    |
    v
preplanning / orchestration topology
    |
    v
branch planning
    |
    v
implementation
```

Architecture intake determines how proposed architectural concerns coexist.

Hierarchical planning determines how the accepted body of work should be decomposed and executed.

The intake session should not become a preplanner for implementation work.

---

# 15. Relationship to Structural Plan IR

The seam-map concept and Plan IR may share structural ideas without sharing semantic ownership.

Plan IR represents implementation meaning.

The architecture intake seam map represents relationships among prospective architectural changes before implementation planning.

Both may benefit from:

- stable semantic identities;
- typed relationships;
- immutable revisions;
- explicit authority;
- cross-revision correspondence;
- projections; and
- structural verification.

Shared representation infrastructure may eventually be useful.

That is a later architecture question.

The first intake implementation should not force both concepts into one schema merely because their structures resemble one another.

---

# 16. Relationship to Durable History and Reconstruction

Architecture intake should participate naturally in Work Engine's broader historical model.

A later actor should be able to reconstruct:

```text
which implemented architecture formed the baseline?

which ideas had entered the session?

what seams had been discovered?

which reconciliation decisions had been made?

which prospective architecture revision existed at that coordinate?

why did the final proposal decomposition have its shape?
```

This does not require preserving every reasoning transcript.

The system should preserve the durable consequences and evidence required to reconstruct the architectural state.

---

# 17. Relationship to Branching

Normal architecture intake may remain a single successor lineage initially.

However, the representation should not require history rewriting.

If an intake direction must be abandoned and reconsidered from an earlier coordinate, a successor branch may be created rather than modifying historical state.

Conceptually:

```text
S0 -> S1 -> S2 -> S3
             \
              S2b -> S3b
```

Both remain truthful histories.

The initial implementation need not expose general-purpose branching controls unless a demonstrated workflow requires them.

---

# 18. Session Concurrency

## Initial invariant

A new architecture-forming intake session should ordinarily not begin until the preceding session's prospective architecture has reached an accepted implemented terminal state.

The purpose is to preserve one truthful architecture baseline per session.

Without this restriction, a later intake would have to reason against uncertain intermediate states such as:

- accepted but not implemented;
- partially implemented;
- implementation-modified;
- awaiting review;
- rejected;
- or conditionally accepted.

Those semantics may eventually be representable, but they are unnecessary complexity for the initial design.

---

## 18.1 What this restriction does not mean

It does not require all Work Engine activity to stop.

Independent implementation, research, review, or operational work may continue where its validity does not depend on the prospective architecture under formation.

The restriction applies specifically to opening another architecture-forming intake lineage whose correctness depends on the result of the current one.

---

# 19. Architecture-Change Completion Boundary

The next architecture intake session should bind to the architecture that actually survived:

```text
planning
+
implementation
+
review
+
reconciliation
+
acceptance
```

not merely the architecture imagined at intake freeze.

If implementation or review changes the realized structure within authorized bounds, the resulting accepted architecture becomes the next baseline.

This preserves the rule:

> **New intake reasons from architecture that truthfully exists, not architecture that was merely expected to exist.**

---

# 20. Context Locality

The architecture intake session may require broad cross-cutting context while it is forming the seam map.

Downstream consumers should not inherit that entire context.

Instead:

```text
architecture intake
    understands the full prospective family
        |
        v
durable seam map + reconciliation decisions
        |
        +--> proposal A receives relevant seams
        +--> proposal B receives relevant seams
        +--> proposal C receives relevant seams
```

This applies Work Engine's context-locality principle at the architecture layer.

Do expensive cross-cutting synthesis once where the relevant information is present.

Preserve its durable consequences.

Project only the required structure downstream.

---

# 21. Mechanical Validation

Some seam-map properties should be mechanically checkable.

Candidate checks include:

```text
Does every shared semantic concept have one authoritative owner?

Does every derived artifact identify its source?

Does every CONFLICTS seam have a disposition before freeze?

Does every COUPLED_DECISION identify its decision owner?

Do invalidation edges identify the dependent semantic state?

Do proposal decompositions preserve all required shared seams?

Are references revision-bound?

Does any downstream proposal claim architecture authority retained by intake?

Does the prospective architecture falsely label an unimplemented state as current?
```

Mechanical checks establish structural consistency.

They cannot determine whether the architecture itself is semantically correct.

---

# 22. Initial Workflow

A minimal first implementation could be:

## A. Open session

Bind to the current implemented architecture revision.

## B. Admit first idea

Identify its architectural concerns and initial seams with the baseline.

## C. Integrate subsequent ideas

For each idea:

```text
current seam map
+
new idea
+
relevant architecture evidence
    |
    v
integration / reconciliation
    |
    v
successor seam-map revision
```

## D. Surface coupled decisions

Return only genuinely shared architecture decisions to the appropriate authority.

## E. Reconcile

Publish exact reconciliation records.

## F. Freeze

When the session is coherent enough for proposal formation, freeze the prospective architecture and seam-map revision.

## G. Decompose

Determine coherent proposal boundaries and shared inputs.

## H. Hand off

Proposal and planning roles receive bounded projections of the frozen session state.

## I. Realize architecture

Implementation and review proceed through existing Work Engine machinery.

## J. Establish next baseline

The accepted implemented architecture becomes the basis for the next architecture intake session.

---

# 23. Suggested First Pilot

The first pilot should use a real set of related pending Work Engine ideas whose seams are already suspected.

For example, a session might include:

- provider/harness/operator runtime separation;
- capability realization and runtime isolation;
- structural Plan IR;
- decision-gated implementation compilation;
- execution-profile scoring;
- hierarchical planning and orchestration;
- durable role-history reconstruction.

The pilot should not attempt to redesign every document.

Its purpose should be to test whether incremental seam integration can produce a more coherent proposal basis than independent intake.

Useful observations include:

- number of shared seams discovered;
- number of ownership conflicts;
- number of coupled decisions;
- ideas that were merged or split during proposal decomposition;
- contradictions discovered before planning;
- downstream questions avoided because seams were already resolved;
- intake inference/token cost;
- proposal/planning context reduction;
- architectural decisions later reopened during implementation;
- integration rework avoided; and
- seam-map complexity.

---

# 24. Failure Conditions

The idea should be reconsidered if architecture intake:

- becomes implementation planning under another name;
- requires a universal architecture ontology before useful operation;
- produces seam maps more expensive to understand than the original ideas;
- creates duplicated authority beside existing proposal or architecture owners;
- forces unrelated ideas into one intake session;
- requires speculative tracking of many future implementation states;
- makes every overlap a human decision;
- prevents local proposal judgment where no shared architecture consequence exists; or
- becomes permanent conversational state rather than reconstructable durable structure.

---

# 25. Expected Benefits

If successful, incremental architecture intake should:

- detect overlap before proposal formation;
- surface contradictory ownership early;
- prevent individually reasonable proposals from composing into an incoherent architecture;
- reduce repeated cross-cutting reasoning during planning;
- reduce later integration work;
- make proposal boundaries reflect architectural ownership rather than idea-document boundaries;
- preserve explicit shared decisions;
- improve context locality downstream;
- allow architecture evolution to remain reconstructable;
- avoid reasoning against speculative implementation states; and
- provide a coherent prospective vision before expensive implementation planning begins.

---

# 26. Governing Invariants

### One implemented baseline

Every architecture intake session binds to one truthful implemented architecture revision.

### Prospective is not implemented

Session state must distinguish intended future structure from current architectural truth.

### Incremental integration

Each idea is reconciled against the current prospective architecture, not independently against the original baseline.

### Single semantic ownership

Shared concepts must not silently acquire multiple authoritative owners.

### No authority by locality

An intake role, proposal former, planner, or implementer does not acquire architecture authority merely because it encounters an unresolved seam.

### Preserve valid prior work

When a new idea invalidates one prospective assumption, retain all prior seams and decisions that remain valid.

### History is not rewritten

Revisions and reconsiderations create successor state rather than mutating historical meaning.

### Planning remains downstream

Architecture intake defines coherent structural space. It does not perform implementation planning.

### Context locality

Downstream roles receive relevant seam decisions and constraints, not the complete architecture-intake reasoning context.

### Truthful next baseline

A subsequent intake session begins from the architecture actually accepted after implementation and review.

---

# 27. Open Questions

1. What exact artifact represents the implemented architecture baseline?
2. Which existing Work Engine artifacts can provide architecture evidence without constructing a new global architecture database?
3. What minimum seam vocabulary is sufficient for the first pilot?
4. Which reconciliation outcomes require human authority versus architecture-role judgment?
5. When should two ideas be merged into one proposal?
6. When should one idea split into multiple proposals?
7. What constitutes sufficient coherence to freeze an intake session?
8. Which seam relationships must be canonical versus rebuildable?
9. How should architecture intake receive implementation feedback when planning or review falsifies a frozen assumption?
10. Under what conditions, if any, should a frozen architecture session be reopened rather than replaced by a successor?
11. How much historical intake state belongs in durable records versus reconstructable projections?
12. Can existing proposal/decision artifacts carry reconciliation records rather than introducing another decision schema?
13. Should the seam map eventually share representation infrastructure with Plan IR or remain independently specialized?
14. What measured reduction in downstream rework would justify the added intake inference cost?
15. What future evidence would justify allowing architecture intake sessions against non-implemented prospective baselines?

---

# 28. Compact Model

```text
implemented architecture
        |
        v
architecture intake session
        |
        v
integrate idea
        |
        v
discover seams
        |
        v
reconcile shared structure
        |
        v
update prospective architecture
        |
        +---- repeat for next idea
        |
        v
freeze coherent session
        |
        v
decompose into proposals
        |
        v
plan / implement / review
        |
        v
accepted implemented architecture
        |
        v
next intake session
```

---

# Core Principle

> **Integrate ideas against one coherent prospective architecture before planning them independently.**

The purpose of architecture intake is not to predict implementation.

It is to ensure that downstream proposals begin from one structurally coherent understanding of how the ideas fit together.

