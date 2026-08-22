# Proposal: Durable Proposal Packets

## Identity and state

- Proposal ID: `work-engine.proposal-packets`
- Family ID: `work-engine.idea-to-proposal-system`
- State: formed; placement probable; not evaluated or accepted
- Decision owner: user or future explicitly authorized portfolio owner

The canonical lifecycle and relationship metadata is in [`packet.json`](packet.json).
This narrative owns the proposal's current semantic meaning. Supporting packet
artifacts record its [placement](placement.md), [relationships](relationships.md),
and proposed [implementation plan](implementation-plan.md).

## Problem

Work Engine has durable execution receipts and strategic handoffs, but no
canonical object for a candidate change before it becomes an execution
objective. As a result, proposal framing, placement, rejected alternatives,
dependencies, and uncertainty can be lost, duplicated, or reconstructed by
later roles.

The problem is not that every thought is absent from disk. The repository has
many idea documents. The problem is that speculative source material and a
formed candidate decision are not durably distinguished.

## Intended consequence

A formed candidate change has one stable, human-readable durable identity from
formation through evaluation and portfolio decision. Every consumer can load
the smallest projection needed for its role without treating brainstorming,
transcripts, or roadmap summaries as authoritative proposal state.

## Proposed product change

Add a Git-backed proposal-packet contract owned by Work Engine's planning
layer. One packet represents one independently decidable candidate change.
Shared origin context belongs to a proposal family and is referenced rather
than copied into every packet.

The contract should distinguish a small required core from optional lifecycle
artifacts.

Required core:

```text
proposal metadata
proposal meaning and boundary
placement claim and uncertainty
typed proposal relationships
```

Optional, independently evolving artifacts:

```text
reusable evidence and provenance
evaluation and estimates
portfolio decision
implementation projection
observed outcome and calibration
```

The exact serialization is an implementation decision, but it must support:

- a stable proposal ID independent of title and path;
- a stable family/origin reference without copying the source idea;
- explicit lifecycle state and the authority responsible for changing it;
- typed relationships with rationale and causal/non-causal distinction;
- split, merge, supersession, and replacement lineage;
- schema/version identification for machine validation;
- explicit uncertainty, invalidation, and reopening conditions;
- selective loading by downstream role; and
- truthful provenance for evidence and decisions.

Markdown may own semantic narrative and a small machine-readable manifest may
own identity, state, relationships, and versioning. A database is not required
for the initial consequence: Git already supplies durable history, review, and
single-repository conflict visibility for these slow-moving artifacts.

## Invariants

### One authoritative proposal identity

Reason: downstream evidence, decisions, roadmap references, and receipts must
not silently attach to different candidates after a rename, split, or merge.

Required property: each packet has a stable ID and lineage-preserving identity
transitions.

### Origin is not proposal authority

Reason: brainstorming is intentionally speculative. Treating an idea as the
current proposal would promote unverified assumptions into decision state.

Required property: origin material remains recognizable and referenced, but
the formed proposal owns its current meaning.

### Handoff cannot own the only resumable state

Reason: a context reset or failed handoff would otherwise erase accepted
formation or evaluation work.

Required property: before responsibility transfers, resume-critical proposal
state is represented in the packet or another named canonical artifact.

### Decision authority remains explicit

Reason: creating or editing a packet must not silently authorize
implementation, priority changes, acceptance, or rejection.

Required property: lifecycle transitions identify their authority owner and
preserve user decisions.

### State remains truthful

Reason: field presence is not semantic certainty, and unavailable evidence is
not negative evidence.

Required property: observed, inferred, decided, unresolved, and unavailable
information remain distinguishable where the distinction affects consumers.

## Scope

In scope:

- packet identity, versioning, ownership, and lifecycle semantics;
- required versus optional information boundaries;
- family references and proposal lineage;
- typed relationships and causal-order semantics;
- checkpoint, handoff, and resume consequences;
- role-specific context projections;
- deterministic validation of mechanically decidable packet invariants.

Out of scope:

- a fixed questionnaire or universal formation sequence;
- scoring formulas or automatic prioritization;
- runtime campaign/agent-state storage;
- concurrent scheduling and mutation reservations;
- automatic roadmap mutation;
- implementation authorization.

## Alternatives

### Keep richer entries in `roadmap.md`

Not preferred. The roadmap owns portfolio direction and order; making it also
own every proposal's evidence and revision history couples two different
information lifetimes and makes selective loading difficult.

### Store proposals in the runtime-state database

Not preferred for the initial version. Proposal artifacts are slow-moving,
human-reviewable product decisions. Runtime state owns active execution and
recovery transitions. A stable cross-reference is sufficient unless observed
concurrency, query, or atomicity needs prove otherwise.

### Continue using unstructured idea files

Insufficient. Idea files are valuable speculative sources, but they cannot
truthfully represent a formed, placed, evaluated, or accepted proposal without
acquiring distinct identity and lifecycle semantics.

## Uncertainty and invalidation

- If real usage requires multi-writer atomic transitions that Git review cannot
  provide, reopen the storage boundary rather than layering implicit locks onto
  Markdown.
- If selective role loading cannot be achieved without a normalized store,
  evaluate a hybrid index while preserving human-readable canonical content.
- If the required core cannot remain substantially smaller than the optional
  lifecycle surface, revisit the packet decomposition before implementation.

## Acceptance consequence

This proposal is successful when two independently formed proposals can be
created, related, split or superseded, evaluated, and referenced from planning
without duplicating their origin, losing decision provenance, or requiring a
consumer to load the entire proposal history.
