# Raw Idea Intake and Claim-Based Initial Assessment

## Status

Active idea. This document proposes a future intake boundary; it does not
change the current `ideas/` contract, establish a schema, form a proposal, move
existing files, or authorize implementation.

## Motivation

Low-friction idea capture and clean architectural inventory have different
needs. Brainstorming benefits from allowing incomplete, overlapping,
contradictory, and poorly structured notes. Proposal formation benefits from
stable identity, independently decidable meaning, explicit relationships,
evidence, uncertainty, and authority boundaries.

Requiring authors to satisfy proposal-like structure while an idea is still
forming raises the cost of capture. Treating every raw note as an active clean
idea instead moves repeated interpretation and cleanup cost into every later
agent context.

The current [`README.md`](README.md) intentionally makes `ideas/` a clean active
projection. This idea asks whether raw capture should become a separate source
surface and the clean projection should instead be produced by a durable intake
assessment.

## Candidate consequence

A person or agent can record an idea with minimal ceremony. When the idea is
pulled into Work Engine, a distinct assessment creates a validated,
claim-addressable interpretation that can be reconciled with other ideas,
proposals, architecture, and implementation evidence. Proposal formation then
consumes that structured interpretation without pretending that the raw note
was already a proposal.

The semantic flow is:

```text
raw idea source
  -> structured idea intake record
  -> claim-level reconciliation and assessment
  -> zero, one, or several proposal candidates
```

This is not necessarily a mandatory procedural sequence for every passing
thought. It describes distinct owners and artifact meanings. A capable model
may combine work when doing so preserves the same boundaries and durable
consequences.

## Proposed ownership split

### Raw idea source

Owns what the author actually wrote. It may be incomplete, speculative,
duplicative, or internally inconsistent. It should not need to satisfy a claim
schema or proposal contract merely to exist.

### Structured intake record

Owns an attributed interpretation of one exact raw-source revision. It can
identify claims, boundaries, relationships, uncertainty, evidence needs, and a
recommended disposition. It does not rewrite the source or convert extracted
claims into established truth.

### Proposal packet

Owns the current meaning and lifecycle of each independently decidable formed
candidate. Promotion does not make every statement in the originating idea
part of the proposal.

### Git history

Preserves source and assessment revisions. Physical movement of a raw file
must not silently change its stable identity, erase its assessment, or break
durable provenance.

## Candidate physical layout

A separate top-level structured surface is the probable direction because it
makes the validation and ownership boundary visible:

```text
ideas/
  inbox/                 # optional low-friction raw capture
  history/               # retained raw sources after reconciliation

idea-intake/
  <stable-idea-id>/
    record.json          # closed identity, state, authority, and references
    assessment.md        # current human-readable interpretation
    claims.jsonl         # candidate claim identities and revisions, if useful
```

The names and exact file decomposition are provisional. A hierarchy entirely
under `ideas/` could also work if raw and validated artifacts remain
unambiguous to people and tooling. Physical adjacency must not collapse their
semantic ownership.

## Minimal intake identity

A useful intake record may need:

- a stable idea identity independent of its current path or title;
- the exact raw-source Git revision, blob, and bounded source path examined;
- assessment revision and producer provenance;
- actual assessment authority and limitations;
- extracted claim identities with exact source ranges;
- claim kind and explicit speculative status;
- material uncertainty and evidence still needed;
- relationships and their current adjudication state;
- recommended disposition and the authority required to exercise it; and
- proposal references created from the intake record.

The manifest should own only fields that need mechanical closure. Narrative
meaning and nuanced assessment should remain readable without forcing every
idea into a large schema.

## Claim-level early assessment

The intake assessment can nominate relationships such as:

```text
possibly_duplicates
possibly_refines
possibly_conflicts_with
apparently_represented_by_proposal
apparently_implemented_by
still_novel
contains_independent_candidate
```

These are assessment claims, not automatic cleanup decisions. Lexical
similarity does not establish semantic duplication, and a code match does not
establish implementation. Repository evidence and the appropriate semantic
owner must adjudicate material dispositions.

The distinction should remain explicit:

```text
claim extraction
  -> what the assessment believes the source asserts

relationship nomination
  -> what may overlap, conflict, refine, or already exist

adjudicated disposition
  -> what an authorized owner decided to retain, split, promote, or archive
```

Claim revisions allow a later assessment to preserve how interpretation
changed without rewriting the raw source or treating an earlier assessment as
new evidence merely because it became durable.

## Candidate intake outcomes

An assessment may truthfully conclude that a raw idea:

- remains an unresolved active idea;
- needs research before proposal formation;
- is substantially represented by an existing proposal;
- appears implemented but needs evidence-backed confirmation;
- should split into multiple independently decidable candidates;
- should merge with or refine another assessed idea;
- conflicts with an accepted contract or another candidate;
- contains no surviving proposal candidate; or
- is ready for proposal formation.

These outcomes should preserve the source, evidence, attribution, uncertainty,
and reopening conditions. Archival or movement is a separately authorized
repository consequence, not something structural validation performs.

## Stable references and physical movement

Current proposal packets use path-based `origin_refs`. Moving idea files can
therefore invalidate otherwise durable packet validation even when Git retains
the content. A future intake boundary should determine whether stable idea IDs
and source-revision references can survive path changes while still resolving
to human-readable repository artifacts.

Possible consequences include:

- packet references target a stable intake identity rather than a mutable raw
  path;
- an intake record retains historical source paths and immutable Git object
  identities;
- moves publish an explicit successor or redirect record; or
- reconciliation updates every affected durable reference atomically.

The mechanism remains open. The protected consequence is that organizational
cleanup must not silently destroy provenance or leave accepted packets with
dangling origins.

## Validation boundary

Mechanical validation could establish:

- stable-ID uniqueness;
- closed record shape and vocabulary;
- source and proposal reference resolution;
- exact assessment revision and source binding;
- non-authorization of cleanup or implementation;
- relationship endpoint validity; and
- consistency between current state and any separately owned disposition.

It cannot establish that extracted claims are correct, two ideas are genuinely
equivalent, an idea is implemented, research is sufficient, or proposal
formation is valuable. Those remain evidence-backed judgments.

## Relationship to current work

The current reconciliation in [`README.md`](README.md) and
[`reconciliation-map.json`](reconciliation-map.json) is a useful manual
precursor. It distinguishes implemented, promoted, split, narrowed, absorbed,
and retained sources while preserving reasons and successor owners. A future
intake capability could make those consequences independently addressable and
revisioned instead of requiring another model to reconstruct the whole cleanup
exercise.

Claim-centered evidence lineage is a plausible semantic dependency for stable
assessment claims and their revisions. It should not be expanded through this
idea or treated as accepted architecture. The current backbone dogfood
explicitly defers broad claim discovery and propagation. Idea intake is a
prospective later consumer, not added scope for that experiment.

The proposal former is a downstream consumer of assessed idea meaning. It does
not need to own raw capture, claim extraction, or cleanup decisions merely
because it can turn a surviving candidate into a proposal packet.

## Does not own

This idea does not own:

- claim-lineage acceptance or implementation;
- proposal identity, lifecycle, review, evaluation, or decision authority;
- roadmap or portfolio priority;
- repository architecture truth or implementation status;
- automatic deletion, movement, merging, or archival of raw ideas;
- the final taxonomy of every possible idea relationship; or
- a mandatory authoring form for initial brainstorming.

## Evidence needed before formation or implementation

- Exercise the candidate record on several raw ideas: a duplicate, a promoted
  idea, an implemented idea, a split idea, and a genuinely novel idea.
- Determine which fields must be closed and which belong in assessment
  narrative.
- Demonstrate that a moved raw file retains stable provenance and does not
  break existing proposal validation.
- Compare a separate `idea-intake/` surface with an `ideas/` hierarchy using
  actual retrieval, validation, and authoring consequences.
- Show that a fresh proposal former can consume one intake record without
  rereading the entire raw-idea directory or reconstructing the reconciliation
  conversation.
- Preserve a negative or uncertain assessment without forcing proposal
  creation or archival.
- Measure whether claim-level reconciliation actually reduces repeated reading
  and semantic cleanup cost enough to justify the additional artifact layer.
