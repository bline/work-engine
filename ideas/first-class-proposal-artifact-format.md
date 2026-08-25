# First-Class Proposal Artifact Format

## Status

Exploratory idea.

## Summary

Proposals are central enough to the system that they may deserve their own first-class artifact format rather than existing only as a collection of loosely associated files.

A proposal is not merely a document. It is a bounded semantic object with identity, history, evidence, claims, alternatives, review lineage, authority state, unresolved questions, consequences, and eventual disposition.

Representing the complete proposal as one portable artifact could improve:

- semantic durability;
- human sharing and archival;
- transport between repositories or environments;
- agent context efficiency;
- deterministic reconstruction;
- projection into role-specific views;
- provenance integrity;
- compression;
- and long-term evolution of the proposal workflow.

The proposal format should represent the proposal's meaning canonically while allowing multiple generated views and transport encodings.

---

## 1. Motivation

The proposal workflow accumulates substantial structured meaning.

A mature proposal may include:

- proposal identity and revision;
- originating problem or idea;
- scope and boundaries;
- architectural context;
- candidate approaches;
- competing alternatives;
- supporting and contradicting claims;
- evidence references;
- assumptions;
- uncertainties;
- consequences;
- dependencies;
- review findings;
- rejected findings;
- synthesis;
- authority relationships;
- decisions;
- implementation placement;
- lifecycle state;
- revision history;
- and provenance.

When these elements exist only as separate files, the proposal as a whole remains partly implicit.

The repository may know where the pieces are, but a human or another environment must reconstruct the relationship among them.

A first-class proposal artifact would make the proposal itself portable.

---

## 2. The Proposal Is the Object

The core design distinction is:

> **The proposal is the semantic object. Files, Markdown documents, UI screens, model contexts, and review packets are representations or projections of that object.**

This prevents the accidental assumption that whichever Markdown file happens to summarize the proposal is the proposal.

A proposal may have many representations without having many competing identities.

Conceptually:

```text
canonical proposal
        ↓
    projections
        ↓
human view
authority view
reviewer view
builder view
supervisor view
archival view
transport representation
```

Each projection exposes only the meaning needed for its purpose.

---

## 3. Single-Artifact Portability

A proposal should be exportable as a single independently identifiable artifact.

For example:

```text
adaptive-specialized-review.proposal
```

A human should be able to:

- attach it to an issue;
- send it to another person;
- archive it;
- move it between repositories;
- inspect it outside the originating workflow;
- import it into another compatible environment;
- or preserve it long after the original working contexts are gone.

The proposal should remain recognizably the same proposal after transport.

Transport should not require carrying the entire originating repository merely to preserve proposal meaning.

---

## 4. Semantic Model Before File Encoding

The proposal format should not begin as a file codec.

First define the semantic model.

For example:

```text
Proposal
├── identity
├── revision
├── origin
├── objective
├── scope
├── boundaries
├── alternatives
├── claims
├── evidence references
├── uncertainty
├── consequences
├── relationships
├── reviews
├── synthesis
├── authority
├── decision state
├── placement
├── lifecycle
└── provenance
```

Only after the semantic structure is sufficiently stable should the physical representation be optimized.

The progression should be:

```text
proposal semantics
    ↓
canonical representation
    ↓
generated projections
    ↓
transport container
    ↓
optional physical compression
```

This avoids coupling proposal meaning to an early storage decision.

---

## 5. Canonical Representation and Generated Views

The canonical representation should favor semantic precision and deterministic interpretation.

Humans should not necessarily edit or even routinely view that representation directly.

Instead, the system can generate purpose-specific views.

Examples include:

### Human proposal

Readable narrative describing the proposal, reasoning, alternatives, findings, and current decision state.

### Authority decision view

Only the information required to decide whether to accept, reject, revise, defer, or request additional evidence.

### Reviewer projection

The claims, assumptions, relevant evidence, boundaries, and unresolved questions needed by a particular review role.

### Builder projection

Accepted consequences, implementation boundaries, relevant claims, obligations, and placement information.

### Supervisor projection

Lifecycle state, review dispositions, unresolved obligations, authority state, and continuation-relevant consequences.

### Archival view

Complete lineage and provenance sufficient to understand how the proposal evolved.

The proposal therefore becomes capable of producing context rather than merely being injected into context.

---

## 6. Semantic Compression

A specialized proposal format creates an opportunity for compression beyond ordinary byte compression.

Ordinary compression reduces storage or transport size:

```text
proposal
    ↓
gzip / zstd
    ↓
fewer bytes
```

This does not necessarily reduce inference cost because the content must eventually be expanded before model consumption.

Semantic compression can reduce the amount of meaning that must be repeatedly expressed as natural language.

For example, instead of repeatedly serializing:

```text
Reviewer R3 reviewed claim C4 and concluded that the claim may
conflict with evidence E8 and E11.
```

the canonical form might encode:

```yaml
reviews:
  - actor: R3
    target: C4
    verdict: contradiction_candidate
    evidence: [E8, E11]
```

A projection compiler can then render only the representation needed by the receiving role.

Repeated identities, relationships, state transitions, provenance structures, and known vocabulary do not need to be repeatedly explained in prose.

This may provide meaningful token savings.

---

## 7. Compression Should Preserve Meaning

Compression must not become lossy summarization merely to reduce tokens.

A compressed proposal representation should preserve whatever is necessary for correct interpretation, including where relevant:

- uncertainty;
- attribution;
- authority;
- temporal applicability;
- competing interpretations;
- rejected alternatives;
- provenance;
- and unresolved questions.

The useful objective is not:

> smallest possible proposal

It is:

> **the smallest sufficiently faithful representation of the proposal for the intended consumer.**

Different consumers may therefore receive different levels of semantic compression.

---

## 8. Embedded Versus Referenced Evidence

A portable proposal does not necessarily need to contain every byte of every source artifact.

Large evidence may remain external and content-addressed.

The proposal may contain:

```text
evidence identity
digest
source location
relevant range
interpretation
claim relationship
provenance
```

Small or continuation-critical evidence may be embedded when losing external access would materially damage the proposal.

This creates a spectrum:

```text
reference only
    ↓
reference + digest
    ↓
reference + extracted evidence
    ↓
embedded artifact
```

The correct representation depends on portability requirements, reconstruction cost, fidelity, and durability.

---

## 9. Proposal as Semantic Checkpoint

A complete proposal artifact could also function as a high-level semantic checkpoint.

Individual model contexts may disappear.

Agent workers may be relaunched.

Repository organization may change.

Intermediate review conversations may no longer exist.

The proposal should nevertheless preserve enough meaning to answer:

- What was proposed?
- Why?
- What alternatives were considered?
- What evidence mattered?
- What was uncertain?
- What reviews occurred?
- What conclusions survived those reviews?
- Who has authority to decide?
- What was decided?
- What consequences follow?
- What remains unresolved?

This makes proposals highly compatible with context-lifetime and semantic-durability mechanisms.

---

## 10. Revision Rather Than Mutation Without History

A proposal should have stable identity while permitting revision.

Conceptually:

```text
proposal P17
├── revision 1
├── revision 2
└── revision 3
```

Changes to conclusions, claims, evidence interpretation, scope, or authority state should remain attributable.

The current proposal can therefore be reconstructed without erasing how it became current.

A transported proposal may contain either:

- the complete revision history;
- a bounded revision history;
- or the current canonical state plus references to historical revisions.

The appropriate form can be selected by transport policy.

---

## 11. Integrity

A first-class proposal artifact makes stronger integrity guarantees possible.

Potential mechanisms include:

- canonical serialization;
- stable object identity;
- revision identity;
- content digests;
- evidence digests;
- explicit relationship identity;
- deterministic rendering;
- lineage validation;
- and schema/version compatibility checks.

A proposal could therefore be independently validated after transport.

---

## 12. File Container

The eventual `.proposal` representation could be a container rather than a single flat serialization.

Conceptually:

```text
proposal.proposal
├── manifest
├── canonical proposal state
├── relationships
├── claims
├── reviews
├── provenance
├── embedded artifacts
└── optional indexes
```

The physical encoding remains an implementation choice.

Possible initial approaches include:

- canonical JSON;
- canonical YAML;
- directory bundle;
- ZIP-compatible container;
- SQLite;
- or another deterministic structured container.

The semantic contract should remain independent of the chosen mechanism.

---

## 13. Human Accessibility

A specialized format should not make proposals inaccessible to humans.

At minimum, tooling should be able to render the proposal into an ordinary durable format such as Markdown.

For example:

```text
something.proposal
    ↓
proposal render
    ↓
something.md
```

A human recipient should not need the full originating system merely to understand the proposal.

The specialized format exists to preserve structure and enable efficient machine use, not to hide information behind proprietary machinery.

---

## 14. Role-Specific Context Projection

One of the strongest potential benefits is context efficiency.

A complete proposal may contain far more information than any individual role requires.

Instead of:

```text
entire proposal
    ↓
every agent
```

the system can perform:

```text
canonical proposal
    ↓
role + current decision
    ↓
relevant semantic projection
    ↓
agent context
```

A reviewer may receive claims and evidence.

A builder may receive accepted consequences and boundaries.

An authority may receive alternatives, confidence, unresolved uncertainty, and review synthesis.

A supervisor may receive lifecycle and obligation state.

This turns proposal structure into an active context-management mechanism.

---

## 15. Proposal Format as Interface

A first-class proposal representation may become more than storage.

It can act as an interface between stages of the workflow.

For example:

```text
idea
  ↓
proposal formation
  ↓
proposal artifact
  ↓
specialist reviews
  ↓
proposal revisions
  ↓
authority decision
  ↓
accepted semantic consequences
  ↓
placement / implementation
```

Each stage consumes and contributes structured meaning without requiring all prior working context.

The proposal becomes a stable semantic boundary between agents.

---

## 16. Potential Failure Modes

### Premature format design

Creating a sophisticated container before proposal semantics stabilize may harden accidental structure.

### Excessive schema

Attempting to type every nuance may increase maintenance and inference cost while destroying useful flexibility.

### Opaque encoding

An efficient representation that humans cannot recover without specialized infrastructure creates unnecessary fragility.

### Lossy compression

Reducing token count by eliminating uncertainty, provenance, alternatives, or authority information may preserve text economy while destroying proposal meaning.

### Monolithic loading

A single proposal file should not imply that the entire proposal must always be loaded into context.

Portability and context projection are separate concerns.

### Duplicated truth

Generated views should not become independently authoritative copies of proposal state.

---

## 17. Evolution Path

A conservative implementation path could be:

```text
existing proposal artifacts
    ↓
identify recurring semantic objects
    ↓
define canonical proposal model
    ↓
build deterministic import/export
    ↓
generate current Markdown views
    ↓
introduce role-specific projections
    ↓
measure context savings
    ↓
experiment with semantic compression
    ↓
introduce portable .proposal container
```

This lets the format emerge from observed proposal behavior rather than speculation.

---

## 18. Research Questions

Useful questions include:

- What proposal meaning is currently duplicated across files?
- Which information is required by all consumers?
- Which information is role-specific?
- How much proposal context is actually consumed by each agent?
- Which repeated prose structures can become canonical relationships?
- How much token reduction can structural encoding produce?
- What information must survive independent transport?
- Which evidence should be embedded versus referenced?
- How much revision history should travel with a proposal?
- Can proposal projections measurably improve model reasoning by reducing irrelevant context?
- Can semantic compression reduce cost without increasing reconstruction error?
- Which recurring untyped structures indicate missing proposal concepts?

---

## 19. Compact Findings

> **A proposal is sufficiently central to the workflow to justify treatment as a first-class semantic artifact.**

> **The proposal should be the canonical object; Markdown files, UI views, review packets, and model contexts should be projections of that object.**

> **A single portable proposal artifact can preserve identity and meaning across repositories, humans, agents, and context lifetimes.**

> **The semantic model should be defined before committing to a specialized physical file encoding.**

> **A structured proposal representation creates opportunities for semantic compression that ordinary byte compression cannot provide.**

> **Compression should minimize representation cost while preserving sufficient truth, uncertainty, provenance, authority, and relationships.**

> **The proposal format should support role-specific projections so agents consume only the portion of the proposal relevant to their current judgment.**

> **A proposal artifact can serve as a semantic checkpoint and stable interface between otherwise disposable agent contexts.**

> **Portability does not require embedding all evidence; external evidence may remain content-addressed when sufficient identity, integrity, and reconstruction information is preserved.**

> **The proposal format should evolve empirically from the proposal workflow rather than forcing the workflow to conform to a premature schema.**