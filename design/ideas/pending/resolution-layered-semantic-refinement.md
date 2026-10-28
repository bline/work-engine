# Resolution-Layered Semantic Refinement

## Status and authority

Exploratory architectural idea, directly captured from operator discussion on
2026-09-22.

This document does not accept a design, amend canonical architecture, change
the roadmap, authorize implementation, or establish that a new architectural
dimension exists. It records a candidate composition that has not yet been
formally reconciled against every relevant owner.

```yaml
idea_provenance:
  origin: direct_capture
  captured_on: 2026-09-22
```

The current evidence suggests that most ingredients already exist separately:

- semantic planning hierarchy;
- recursive organizational compilation;
- compiled logical role contracts;
- planning-resolution projections;
- review at different grains; and
- candidate resolution and admission.

The candidate novelty is narrower: make the refinement relationship between
semantic resolutions explicit and governed, while permitting more than one
bounded vantage at every resolution.

---

## Recognition

Work Engine is moving toward deterministic compilation. Existing architecture
already lowers accepted meaning into plans, organizations, role contracts, and
runtime realizations. Stepping back from the named roles suggests another way
to view that lowering:

> Work may pass through dynamically deep semantic-resolution layers, with one
> or more bounded vantages operating at each layer and an explicit gate
> governing refinement into the next layer.

This is not necessarily a strict management hierarchy.

A coarse layer may contain several agents with independent contexts and
responsibilities. A finer layer may contain only one. A role at a deeper layer
may have better semantic vantage for a local decision than any role above it.
Organizational depth therefore does not imply authority rank, and the number of
agents does not define resolution.

One motivating example is architectural review. A reviewer operating at a
coarse resolution can challenge the system model, ownership, decomposition, or
placement directly without needing implementation-detail context. Finer layers
can then review seams, contracts, and implementation consequences at the
resolution where those judgments become legitimate.

---

## Candidate direction

Introduce an explicit concept of a **semantic resolution layer** and a governed
**refinement edge** between adjacent layers.

Conceptually:

```text
accepted layer R0
    architecture / system meaning
    one or more bounded vantages
        |
        | refinement gate G0
        v
accepted layer R1
    subsystem and ownership boundaries
    one or more bounded vantages
        |
        | refinement gate G1
        v
accepted layer R2
    executable obligations and contracts
    one or more bounded vantages
        |
        | refinement gate G2
        v
accepted layer R3
    implementation-ready realization
```

The labels and number of layers are illustrative. The architecture should not
assume that every body of work requires the same depth or that resolution is a
single universal scalar.

The essential proposed relation is:

```text
accepted semantic artifact at resolution Rn
        +
resolution-local evidence, decisions, and vantages
        |
        v
candidate refinement at resolution Rn+1
        |
        v
refinement gate
        |
        +-- faithful and sufficient -> admitted refinement
        +-- material decision exposed -> return to its decision owner
        +-- evidence insufficient -> remain blocked or uncertain
        +-- upstream premise falsified -> topology conflict / replanning
```

The refinement gate does not own the meaning of the artifacts it relates. It
establishes mechanically knowable correspondence and routes any remaining
semantic judgment to the authority that already owns it.

---

## Four independent axes

The word "layer" can accidentally collapse distinct architectural questions.
This idea depends on keeping at least four axes separate.

### Semantic resolution

How finely the meaning of the work has been resolved.

Illustratively:

```text
system architecture
    -> subsystem boundaries
    -> obligations and contracts
    -> implementation-ready consequences
```

Refining semantic resolution may expose decisions that did not exist or were
not material at the coarser layer. Those decisions require their own authority
and cannot be silently selected by the refiner.

### Organizational shape

How many contexts, vantages, or logical roles realize one accepted semantic
layer.

```text
one semantic layer
    -> one vantage

one semantic layer
    -> several parallel vantages

one semantic layer
    -> recursively compiled child organization
```

Organizational Compilation already owns this axis.

### Projection resolution

How much already-established structure is rendered for a particular consumer
or executor.

A compact projection and an explicit projection may represent the same
accepted semantic layer. Increased rendering detail must not be mistaken for a
new semantic decision.

Structural Plan IR and evidence-calibrated plan resolution already explore this
axis.

### Review grain

The subject and resolution at which a governed judgment is produced.

Examples include architectural review, seam review, contract-conformance
review, and implementation review. Review grain may correlate with semantic
resolution, but review remains its own dimension and does not acquire
acceptance authority merely because it guards a refinement edge.

These axes may vary independently:

```text
coarse semantic resolution + several independent reviewers
fine semantic resolution   + one implementation vantage
fine semantic resolution   + compact executor projection
coarse semantic resolution + detailed evidence projection
```

---

## Resolution-local ownership

"Ownership by resolution" must not become blanket authority for every actor in
a layer.

The existing authority rule remains:

```text
authority is decision-scoped
not rank-scoped
not role-name-scoped
not resolution-scoped
```

A resolution layer may declare:

- its exact subject and predecessor revision;
- semantic facts inherited from coarser layers;
- decisions already closed and therefore not reopenable locally;
- decision classes that may be resolved at this layer;
- authority owners and ceilings for those decisions;
- unresolved questions and required evidence;
- required consequences and downstream consumers;
- admissible refinement operations;
- participating vantages and their separate contracts;
- review obligations applicable at this grain; and
- invalidation or reopening conditions.

Each participating role still receives only its own explicit obligation,
vantage, authority, effect boundary, and terminal consequence.

The layer is therefore a governed semantic scope, not a shared pool of
authority.

---

## Multiple vantages within one layer

A resolution layer may contain more than one agent or deterministic service.
The reason is not simply parallel throughput. Different vantages may be needed
for different consequences at the same grain.

Candidate forms include:

### Partitioned subject ownership

```text
subsystem-resolution layer
    -> vantage A owns subsystem A refinement
    -> vantage B owns subsystem B refinement
    -> vantage C owns their declared integration boundary
```

### Concern-specific judgment

```text
architecture-resolution layer
    -> ownership/decomposition reviewer
    -> security-boundary reviewer
    -> operational-recovery reviewer
```

Their findings remain attributed and potentially conflicting. A coordinator or
fan-in projection must not turn several judgments into artificial consensus.

### Independent challenge

Two vantages may examine the same resolution subject where independence is
part of the required evidence. Shared resolution does not imply shared context
or shared authority.

### Deterministic and semantic participants

One participant may establish mechanical coverage or correspondence while a
semantic role judges only the irreducible remainder. The deterministic service
does not become the decision owner by producing the gate evidence.

These are applications of existing role, review, and organizational principles.
The candidate addition is that their relationship to one semantic-resolution
layer becomes explicit.

---

## Refinement gate

A refinement gate should protect a consequence, not require a ceremonial
review at every level.

Its work may be layered internally.

### Mechanical identity and coverage

- bind the exact predecessor revision and evidence cutoff;
- establish which required upstream obligations were projected;
- preserve traceability from refined facts to their source facts;
- report omissions, unknowns, and unsupported correspondence honestly;
- establish that the candidate is internally well-formed.

### Authority and decision closure

- establish that authority did not expand during refinement;
- distinguish deterministic elaboration from a new semantic decision;
- detect newly material choices rather than hiding them as elaboration;
- route reserved or delegated decisions to their actual owners;
- reject a refinement that silently changes accepted dependencies,
  obligations, or consequences.

### Resolution-appropriate semantic judgment

Where mechanical checks cannot establish the required consequence, a bounded
review may judge the candidate at its own grain.

Examples:

- architecture review for system-model and ownership coherence;
- seam review for correspondence between independently refined regions;
- contract review for preserved obligations and authority boundaries;
- implementation review for conformance of the realized result.

Review judgment remains distinct from gate admission. The authority that owns
the refinement consequence decides whether the evidence is sufficient to
admit the successor layer.

### Publication

An admitted refinement becomes revision-bound durable state. A later
refinement or correction produces a successor rather than rewriting history.

Existing Candidate Resolution and Admission plus Revision/CAS/Publication may
supply most or all of this mechanism. Whether the refinement edge adds any
irreducible domain semantics beyond those mechanisms is an open question.

---

## Dynamic depth

Layer depth should be derived from the needs of the work and its downstream
consumer, not from a fixed universal stack.

Signals that another semantic refinement may be required include:

- the next consumer would otherwise have to reconstruct a material decision;
- accepted obligations are not yet localized enough to assign lawfully;
- cross-region seams remain implicit;
- acceptance consequences cannot yet be tested or attributed;
- authority boundaries are still ambiguous at the downstream grain;
- a capable executor still faces legitimate but materially different routes
  that belong to another owner;
- review discovers that the current representation is too coarse to judge the
  required property.

Signals that semantic refinement may stop include:

- the downstream consumer can act without reconstructing hidden material
  meaning;
- all remaining choices are legitimately delegated implementation discretion;
- required consequences, evidence, authority, and reopening conditions are
  explicit enough for the consumer's contract;
- additional resolution would merely prescribe a route without protecting a
  new consequence.

Complexity is evidence bearing on these questions. It should not become a
single scalar threshold that mechanically determines layer count.

This stopping condition is related to, but distinct from, Organizational
Compilation's existing fixed point:

```text
organizational stopping condition:
    no unresolved obligation benefits from another legitimate vantage

candidate semantic-refinement stopping condition:
    the next legitimate consumer can act without reconstructing unresolved
    material meaning owned elsewhere
```

---

## Relationship to Supervisor and Builder decomposition

The current Supervisor and Builder names may represent recurring compiled
profiles rather than indivisible roles.

A supervisor-shaped bundle may currently contain:

- branch-local coordination;
- organizational-fit observation;
- slice or increment selection;
- dependency and waiting-state interpretation;
- checkpoint and review lifecycle coordination;
- operator-facing explanation; and
- terminal-consequence accounting.

A builder-shaped bundle may contain:

- repository reconnaissance;
- placement and implementation judgment;
- mutation;
- local validation;
- finding disposition; and
- implementation evidence production.

This idea does not assert that these must be split. It suggests a second basis
for investigating the split:

> Do some responsibilities belong to different semantic resolutions, require
> different context, or require independent authority and review even when
> they currently travel under one role name?

If so, Organizational Compilation could derive several layer-local vantages
instead of instantiating one monolithic Supervisor or Builder. If not, the
tested reusable profile remains the more economical organization.

---

## Relationship to the existing semantic hierarchy

Preplanner, Orchestrator, Branch Planner, Supervisor, and Builder describe
different ownership relationships in the current semantic-planning and
execution architecture.

Resolution layers are not proposed replacements for those roles.

The two shapes answer different questions:

```text
semantic hierarchy:
    who forms or realizes which accepted topology?

resolution refinement:
    at what semantic grain is an artifact or decision currently expressed,
    and what establishes that refinement into the next grain is legitimate?
```

One role may participate in more than one resolution at different times
without collapsing the resolutions. Several roles may participate in the same
resolution without acquiring one another's authority.

The candidate benefit is that the current hierarchy becomes one possible
organization over a more explicit refinement structure rather than the only
way to understand decomposition.

---

## Relationship to existing architecture

This idea appears to compose rather than replace:

- [`semantic-planning-hierarchy.md`](../../architecture/semantic-planning-hierarchy.md)
  for planning authority and accepted semantic topology;
- [`material-decision-selection.md`](../../architecture/material-decision-selection.md)
  for route-variant material decisions and their authority;
- [`implementation-contract-compilation.md`](../../architecture/implementation-contract-compilation.md)
  for deriving an implementation-ready contract from accepted meaning;
- [`organizational-compilation.md`](../../architecture/organizational-compilation.md)
  for compiling one or more lawful execution vantages at any accepted layer;
- [`role-and-contract-structure.md`](../../architecture/role-and-contract-structure.md)
  for each resulting logical role's contract;
- [`review.md`](../../architecture/review.md) for semantic judgments at
  architecture, seam, contract, and implementation grains;
- [`mechanisms/candidate-resolution-and-admission.md`](../../architecture/mechanisms/candidate-resolution-and-admission.md)
  for deterministic zero/one/many reduction and residual judgment;
- [`mechanisms/revision-cas-and-publication.md`](../../architecture/mechanisms/revision-cas-and-publication.md)
  for durable predecessor-bound publication;
- [`structural-plan-ir-for-capability-aware-multi-model-execution.md`](structural-plan-ir-for-capability-aware-multi-model-execution.md)
  for multiple projection-resolution levels over one canonical plan; and
- [`evidence-calibrated-plan-resolution-and-continuous-capability-learning.md`](evidence-calibrated-plan-resolution-and-continuous-capability-learning.md)
  for empirically selecting sufficient projection detail.

The relationship is deliberately unresolved. Possible outcomes include:

1. **Pure composition.** Existing owners and mechanisms already determine the
   whole behavior; only a cross-view explanation is missing.
2. **New mechanism.** A reusable refinement-edge protocol is needed, but it
   owns no new class of semantic truth.
3. **New dimension.** Resolution-layer identity, refinement sufficiency, or
   another irreducible truth requires a distinct semantic owner.
4. **Representation only.** Structural Plan IR can express the relationship
   without any new architecture beyond its schema.

No outcome is selected by this capture.

---

## Candidate acceptance evidence for the idea

Evidence that would support a distinct refinement concept includes:

- one real objective whose semantic meaning must be admitted at more than one
  resolution before implementation can begin safely;
- two or more agents operating legitimately at one resolution with different
  contexts and responsibilities;
- a refinement gate catching a material choice that deterministic rendering
  would otherwise hide;
- an architecture-level review finding that changes or blocks downstream
  refinement without becoming implementation review;
- a finer layer falsifying an upstream premise and routing the conflict to the
  correct owner without silently repairing it;
- two objectives legitimately reaching different refinement depths;
- evidence that a named ResolutionLayer or refinement edge removes ambiguity
  not already resolved by accepted-plan revisions and existing admission
  mechanisms.

Evidence against a distinct concept includes:

- every candidate layer maps exactly onto an already-owned artifact revision;
- every candidate refinement gate is merely Candidate Resolution and Admission
  over an existing domain's candidates;
- "resolution" reduces entirely to executor-facing Plan IR projection detail;
- multiple agents per layer reduce entirely to Organizational Compilation;
- review at each layer reduces entirely to existing review subjects and
  invocation rules; or
- no downstream decision or consumer can distinguish a first-class layer from
  the existing composition.

---

## Failure modes

### Hierarchy renamed as resolution

Calling Preplanner, Planner, Supervisor, and Builder "resolution layers"
without adding a distinct refinement relation changes vocabulary but not
architecture.

### Authority by altitude

Treating a coarser layer as universally superior, or a finer layer as locally
sovereign, would violate decision-scoped authority and legitimate vantage.

### Mandatory ceremonial gates

Requiring a review ritual between every pair of layers without an identified
protected consequence would create procedure rather than structure.

### Semantic invention disguised as elaboration

A finer layer may quietly select material architecture or product decisions
while claiming merely to add detail.

### Projection detail confused with semantic refinement

Rendering an accepted fact more explicitly does not create a new semantic
layer.

### Agent count used as a complexity score

More agents do not prove that a problem is more complex, more important, or
better decomposed.

### Premature scalar resolution

Different dimensions may need different degrees of refinement. A single
`R0`-to-`R3` scale may obscure rather than clarify that structure.

### Local validity mistaken for integrated validity

Several layer-local results can each be valid while their seams or combined
consequences fail. Integration and seam judgment need explicit ownership.

---

## Questions for later intake and reconciliation

1. Does a semantic resolution layer own any truth not already owned by an
   existing accepted-plan or contract revision?
2. Is the refinement edge a new mechanism, or simply Candidate Resolution and
   Admission plus Revision/CAS over existing domain artifacts?
3. Is resolution scalar, a dimension-specific vector, or only a relation
   between a producer and a downstream consumer?
4. What exact property distinguishes deterministic elaboration from semantic
   refinement?
5. Who owns refinement sufficiency when the downstream consumer, upstream
   semantic owner, and reviewer are different actors?
6. Can multiple vantages refine one layer concurrently without one becoming an
   implicit layer coordinator or semantic owner?
7. What fan-in artifact preserves agreement, conflict, coverage, and omitted
   concerns across parallel layer participants?
8. Which refinement checks are mechanical, which belong to Review, and which
   belong to the owning admission authority?
9. Does architectural review act as a gate only when explicitly required by a
   downstream consequence, or does some class of refinement require it as an
   invariant?
10. How does a layer expose enough semantic structure for Organizational
    Compilation without prescribing the execution organization?
11. Can the current branch-plan and implementation-contract chain serve as a
    first demonstrator without inventing new artifacts?
12. What evidence would establish that separating Supervisor or Builder
    responsibilities by resolution improves correctness, context locality, or
    total accepted-work cost?

---

## Non-goals

This idea does not currently propose:

- replacing the existing semantic-planning hierarchy;
- granting authority according to organizational rank or resolution depth;
- requiring a fixed number of semantic layers;
- requiring multiple agents at every layer;
- requiring model review at every refinement edge;
- treating Plan IR projection detail as semantic authority;
- allowing a finer layer to revise an upstream semantic artifact silently;
- making complexity a universal scalar or agent-count formula;
- changing Review's separation from acceptance authority;
- changing Organizational Compilation's ownership of execution topology;
- authorizing recursive planning by execution roles; or
- implementing a new compiler, gate, role, or durable schema from this capture.

---

## Compact hypothesis

> Work Engine may benefit from treating semantic refinement as a dynamically
> deep, revision-bound chain. Each resolution layer can contain one or more
> bounded vantages with separate context and responsibility. A refinement gate
> mechanically establishes identity, coverage, correspondence, and authority
> preservation; exposes unresolved material decisions; invokes grain-appropriate
> semantic judgment only where needed; and admits the next layer only through
> the authority that owns its consequence. Organizational depth, projection
> detail, and review grain remain independent of semantic resolution.

The unresolved architectural question is whether this hypothesis names a real
missing mechanism or merely provides a useful view over structures Work Engine
already owns.
