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

## Candidate intake process

Intake begins only when a user or another authorized workflow selects a raw
idea for assessment. Merely creating or editing a file in the raw capture
surface does not activate an assessor, grant cleanup authority, or imply that
the idea deserves proposal formation.

An intake attempt should bind:

- a stable intake-attempt identity;
- the exact raw idea identity and immutable source revision being assessed;
- the purpose and bounded scope of the assessment;
- the actor or role performing the assessment and its authority;
- the repository, proposal, architecture, and external-evidence cutoff;
- any user decisions already applicable to the source; and
- the intended consumer of the result.

The attempt may investigate, decompose, compare, ask questions, or revise its
working interpretation in whatever order best serves the actual idea. The
protected causal shape is:

```text
bind one exact source and assessment purpose
  -> identify independently addressable candidate claims
  -> reconcile material claims with current durable owners and evidence
  -> preserve uncertainty, conflicts, and relationship nominations
  -> form an attributed intake disposition
  -> hand surviving candidate meaning to the appropriate consumer
```

Later activities depend on the earlier consequences: reconciliation is not
truthful until the source and claims are bounded, and proposal formation cannot
consume the assessment until its disposition and uncertainty are durable. The
model remains free to choose its evidence-gathering and reasoning route inside
those boundaries.

### 1. Source binding and applicability

The assessor first establishes which immutable source revision and which part
of it are in scope. If the raw file changes during assessment, the active
attempt remains bound to the earlier revision. The newer source revision may:

- be immaterial to the bounded claims;
- require an applicability judgment;
- nominate the assessment as candidate-stale; or
- justify superseding the attempt with a new source-bound assessment.

It must not silently change the subject underneath an existing disposition.
Path movement alone does not create a new idea identity when the source meaning
and Git lineage remain intact.

### 2. Claim identification and decomposition

The assessor identifies material propositions, desired consequences,
assumptions, constraints, and open questions that affect whether independently
decidable work exists. It should retain source ranges and distinguish:

- claims made by the raw source;
- assessor interpretations and inferred boundaries;
- questions that remain unresolved;
- examples or possible mechanisms that are not the idea's essential meaning;
  and
- multiple candidate changes that should not be forced into one identity.

The assessment need not assign a durable identity to every sentence. Claim
identity has value when a proposition may be compared, revised, relied on,
adjudicated, or handed to proposal formation independently.

### 3. Reconciliation with durable owners

For each material claim, the assessor retrieves only the evidence needed to
distinguish among plausible dispositions. Relevant owners may include:

- current implementation and architecture for what exists now;
- proposal packets for already formed candidate meaning;
- accepted contracts and authority records for binding boundaries;
- other intake records or active ideas for overlap and conflict; and
- claim or review history for prior judgments and their evidence worlds.

The assessor records the evidence cutoff and separates observation from
inference. An apparent implementation, duplicate, conflict, or supersession is
first a nomination. It becomes an adjudicated disposition only under the
authority applicable to that semantic decision.

Evidence gathering should stop when additional evidence has no credible value
for the intake disposition. Intake is not a requirement to research every
possible consequence to proposal-review depth.

### 4. Interaction and unresolved decisions

The assessor may ask the user when source ambiguity, product intent, preference,
or authority prevents a truthful disposition. Questions should target the
smallest decision that materially changes decomposition, ownership, or handoff.

A user answer is preserved as attributed authority evidence only for the
decision actually exercised. It does not automatically accept a proposal,
settle architecture, authorize implementation, or validate every assessor
inference.

If no authorized answer is available, the intake record remains explicitly
unresolved or blocked on that decision. It must not invent missing intent to
keep the pipeline moving.

### 5. Intake disposition

The assessor publishes a disposition for each independently addressable claim
or candidate cluster, rather than forcing one file-wide answer. The disposition
records:

- current classification and rationale;
- evidence and authority supporting it;
- uncertainty and contradictory evidence;
- surviving candidate boundary and semantic owner;
- relationships to other ideas, intake records, proposals, or implementation;
- recommended next consumer;
- reopening conditions; and
- cleanup consequences that remain unexercised.

One source may therefore yield several proposal candidates, a mixture of
promoted and still-novel claims, or no surviving candidate. The enclosing intake
record summarizes those claim-level outcomes without erasing them.

### 6. Proposal-formation handoff

When a candidate is ready for formation, the handoff supplies the proposal
former with the smallest useful projection:

- stable intake and source identities;
- independently decidable candidate meaning;
- claim revisions and adjudicated relationships relevant to that candidate;
- current placement hypotheses and ownership boundaries;
- evidence already used and evidence still needed;
- unresolved human decisions; and
- authority and reopening conditions.

The proposal former may challenge, split, merge, or decline the candidate. It
creates proposal identity only for surviving formed meaning and references the
intake record as provenance. Intake readiness is not proposal acceptance or
implementation authorization.

### 7. Cleanup application

Moving, archiving, merging, redirecting, or deleting a raw idea is a separate
repository consequence after assessment. The intake disposition may recommend
that consequence but does not execute it unless the responsible authority has
granted that mutation.

Before physical cleanup, affected durable references must either be updated or
continue to resolve through stable identity and source lineage. The applied
cleanup records which disposition authorized it and which provenance remains
reachable afterward.

## Intake lifecycle and resumability

The exact state vocabulary remains to be exercised, but consumers need to
distinguish at least these semantic conditions:

```text
source captured, no assessment
assessment active against an exact source revision
awaiting evidence or human decision
assessment published with unresolved consequences
assessment published and ready for a named handoff
superseded by a later source-bound assessment
withdrawn by the source or decision owner
```

These are semantic conditions, not necessarily one mandatory linear state
machine. For example, an assessment can publish useful resolved claim
dispositions while other claims remain awaiting evidence.

An intake attempt should durably publish resume-critical consequences as soon
as losing them would require material reconstruction or risk a different
disposition. Compact resumable state may include:

- source and assessment identity;
- claims already bounded and their current revisions;
- evidence cutoffs and integrity-bound references;
- adjudicated and still-nominated relationships;
- user decisions and their exact authority scope;
- pending evidence or interaction obligations;
- unresolved conflicts and reopening conditions; and
- the next safe semantic obligation.

The durable intake record owns the assessment history. Role-owned operational
state may reference the current attempt, pending interaction, and next
obligation without copying that history. After context replacement, a resumed
assessor reconstructs from those durable owners and refreshes mutable
repository observations; a conversation summary is only a navigation hint.

Repeated intake of the same source revision should reconcile with the existing
attempt or deliberately create a new attributed assessment perspective. It
must not silently overwrite the earlier judgment or report duplicated work as
independent evidence.

## Example intake result

A broad raw note might produce:

```text
idea I-17 at source revision S4

claim A
  disposition: represented_by_proposal
  target: work-engine.example-proposal
  evidence: exact proposal revision

claim B
  disposition: apparently_implemented
  evidence: implementation and test references
  authority: confirmation still required

claim C
  disposition: ready_for_proposal_formation
  boundary: independently decidable remaining consequence
  unresolved: permanent placement

claim D
  disposition: needs_human_decision
  question: product preference that changes candidate meaning
```

The intake summary can recommend linking claim A, verifying claim B, handing
claim C to the proposal former, and retaining claim D as unresolved. It cannot
collapse the whole source into “done” merely because some claims were promoted
or implemented.

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
