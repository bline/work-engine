# Proposal: Production Claim Evidence Interface

## Identity and state

- Proposal ID: `work-engine.production-claim-evidence-interface`
- Family ID: `work-engine.evidence-lineage`
- State: formed; probable shared-capability placement; not evaluated,
  prioritized, accepted, or authorized for implementation
- Decision owner: user or future explicitly authorized portfolio owner

The canonical lifecycle, placement, uncertainty, relationship, and authority
metadata is in [`packet.json`](packet.json). This narrative owns the proposal's
current meaning. Supporting artifacts describe its [placement](placement.md)
and [relationships](relationships.md).

The user authorized formation of this production phase and its dependent next
phase so that narrowing the first implementation cannot discard the complete
claim-lineage objective. That formation authority does not accept either
proposal, change roadmap priority, settle permanent placement, or authorize
implementation.

## Candidate and consequence

Create the smallest production claim-evidence capability that real Work Engine
roles can use to publish, discover, resolve, and rely on exact evidence-backed
statements without treating visibility as truth, freshness, applicability, or
authority.

A Codex agent receives a repository-local skill over this capability. An
external reviewer may receive an MCP projection. Both transports expose the
same record identities, revisions, completeness, limitations, and authority
meaning; neither transport owns the semantics.

This phase makes the shared evidence core real. It deliberately leaves active
impact nomination, refresh orchestration, selective reopening, obligation
delivery, and failure recovery to the separately formed
[`claim-maintenance-and-reliance-propagation`](../claim-maintenance-and-reliance-propagation/proposal.md)
proposal.

## Why this is independently decidable

Stable evidence records and useful queries provide production value before an
automated or coordinated maintenance lifecycle exists. Research and review
roles can preserve exact evidence worlds, retrieve prior judgments, and record
which revision supported a decision. That outcome can be validated without
claiming that repository change detection, semantic refresh, or cross-owner
delivery is complete.

Conversely, implementing only files from the synthetic dogfood would not
satisfy this proposal. A production interface must have a canonical owner,
closed versioned records, domain authority profiles, transport-neutral
operations, real consumers, and truthful discovery and applicability results.

## Formation evidence

Repository evidence cutoff for the cited existing artifacts:
`cdc9e3fa5d300e5edc737faf38edf85a336fbdcf`.

Formation used the reviewed semantic parent and its still-binding deferred
decision, the final synthetic dogfood evidence report, the existing bounded
Work Engine MCP projection, the proposal-packet contract, and current Work
Engine ownership doctrine. The dogfood established bounded representability;
it did not establish production placement, a real consumer, or implementation
authority. The current MCP adapter is evidence that an external projection is
feasible, not the canonical production interface design.

## Production semantic core

The shared owner defines a closed, versioned core for:

- bounded subject identity, including namespace, subject kind, stable subject
  ID, immutable evidence baseline, and actual content-set boundary;
- stable claim identity and collision-safe immutable claim-revision identity;
- proposition or finding, support qualification, assumptions, limitations,
  confidence where meaningful, evidence references, sensitivity references,
  and reopening conditions;
- producer provenance, actual evidence mode, judgment kind, decision scope,
  domain profile, and exact authority reference;
- typed non-destructive lineage among exact revisions, including refresh,
  correction, supersession, composition, derivation, and explicit identity
  forks; and
- versioned downstream reliance on an exact claim revision for one identified
  consumer artifact, revision, and decision scope.

The core does not decide whether a claim is substantively correct or whether a
consumer should rely on it. It makes those distinctions addressable and
mechanically checkable.

Domain extensions remain versioned and explicitly named. The shared schema
must not absorb proposal maturity, review severity, reviewer episodes,
synthesis, readiness, or other domain judgments merely because an initial
consumer needs them.

## Initial production domain profiles

At least two real profiles exercise the shared contract:

### Proposal-research claim

The proposal-research domain owns the judgment, its materiality boundary, its
support qualification, and any authority to publish or revise a research
claim. Proposal packets may reference an exact claim revision but do not absorb
its history or silently follow a later revision.

### Revision-bound review finding

The review domain owns the finding, severity, episode, outcome, conflict, and
synthesis. The shared capability owns only the finding's stable statement
identity, evidence baseline, provenance, limitations, authority scope,
sensitivity, and cross-revision lineage.

Each profile names which roles may create a stable claim, publish a revision,
correct or supersede it, assess applicability, and record reliance. Possession
of the skill, files, validator, or MCP connection does not grant those roles.

## Transport-neutral operations

The canonical capability exposes operations by semantic consequence rather
than by provider-specific transport.

### Read and discovery

A consumer can:

- resolve an exact claim or revision identity;
- discover candidate claims by bounded subject, source namespace, artifact or
  contract reference, evidence baseline, producer, profile, sensitivity
  reference, support state, or exact consumer reliance;
- traverse predecessors, successors, corrections, supersessions,
  compositions, and identity forks;
- inspect direct and reverse reliance on an exact revision; and
- request projection identity, schema/build versions, source watermarks,
  excluded or failed inputs, freshness, and completeness needed for the query.

Discovery returns candidates, not automatic applicability. Every result keeps
the exact revision, bounded subject, evidence world, limitations, unresolved
impact or conflict state, and projection boundary together. When several
revisions could govern a decision, the interface returns the conflict or
branching set; it never silently selects the newest record.

### Publication

An authorized producer can:

- create one stable claim identity under a named domain profile;
- publish an immutable revision against an exact evidence baseline;
- publish an allowed correction, supersession, composition, or identity fork;
- record, retire, or supersede an exact-revision reliance; and
- retract a defective publication through an explicit non-destructive
  successor or tombstone allowed by the profile.

Every mutation binds a stable operation identity, expected predecessor or
state, producer, authority grant, and exact payload. Duplicate delivery is
idempotent. Conflicting publication fails closed or creates only the explicitly
authorized branch; it never overwrites canonical history.

Attribution is not admission. The production boundary validates the authority
envelope supplied by a trusted launcher or domain owner before accepting a
transition. Git authorship, MCP connectivity, local process identity, or a
self-declared role is not sufficient by itself.

## Codex skill and external projection

The repository-local skill teaches Codex roles the capability's affordances,
boundaries, and result semantics. It exposes or invokes the canonical
validation, publication, and query mechanics without embedding a fixed query
sequence. Agents choose evidence routes according to the current claim and
record actual provenance.

The external MCP adapter is a bounded projection over the same read owner. It
may expose fewer operations or remain read-only, but equivalent queries cannot
change identity, completeness, freshness, applicability, or authority meaning.
Provider transport is not a new claim owner.

## Canonical history and replaceable projection

Canonical records are human-readable, mechanically validatable, and durable in
Git or another explicitly selected immutable publication boundary. The query
projection is rebuildable and non-owning. It deterministically rejects or
reports duplicate, cyclic, dangling, retracted, unsupported-version, partial,
and stale input according to the closed contract.

The projection exposes enough provenance for a consumer to decide whether the
result supports its bounded query. Unavailable completeness is never reported
as zero results. Derived traversal or candidate relevance cannot become a
semantic judgment merely because the projection emitted it.

Storage and index choices remain implementation mechanisms until observed
production query scale makes one consequential. This proposal does not require
a graph database or always-running service.

## Reference integrity

Subject, evidence, authority, producer, consumer, and lineage references carry
stable owner, exact revision, integrity, and freshness semantics appropriate to
their source. The production capability distinguishes a verified reference,
an unavailable reference, a moved-but-resolvable reference, an excluded source,
and an integrity mismatch. It never treats an unresolved reference as verified
evidence or destroys the record that truthfully captured an earlier reliance.

## Invariants

- A visible record is not automatically true, current, applicable, sufficient,
  accepted, or authoritative.
- Consumers rely on exact claim revisions and decision scopes, never an
  implicit latest revision.
- Domain judgment and shared record mechanics retain separate owners.
- Publication cannot inflate authority from possession of a transport or tool.
- Corrections, supersession, branching, and retirement preserve history.
- Query projections expose freshness, completeness, exclusions, and source
  provenance and remain rebuildable from canonical history.
- Phase-one completion cannot claim impact maintenance, refresh orchestration,
  selective reopening, or obligation delivery from the dependent proposal.

## Boundary

This proposal does not implement:

- automatic change-to-claim impact nomination;
- refresh episodes or `changed_because_of` adjudication;
- automatic canonical-support advancement after refresh;
- selective-reopening obligations or cross-owner delivery;
- continuous monitoring, graph watching, or role activation; or
- proposal acceptance, review acceptance, readiness, roadmap, or execution
  authority.

It preserves all of those formed consequences through the dependent proposal
rather than treating them as optional deferred cleanup.

## Evidence and acceptance needs

Production acceptance requires:

- a closed schema and validator for the shared core and both initial profiles;
- deterministic identity, reference, lineage, branch/conflict, migration, and
  projection-completeness checks;
- authority-bound idempotent publication and exact-revision reliance;
- a Codex role using the skill to discover and rely on a material claim without
  receiving its identity in advance;
- an external read-only consumer observing equivalent state through the MCP
  projection;
- one real proposal-research claim and one real revision-bound review finding
  published and consumed without domain-ownership collapse; and
- failure cases for unauthorized publication, conflicting predecessors,
  dangling evidence, partial projections, unavailable evidence, and misleading
  newest-revision selection.

## Uncertainty

- Whether Git-backed sibling records remain the best canonical publication
  boundary after real consumer use.
- The smallest closed sensitivity vocabulary for structural, negative,
  external, and runtime claims.
- Which discovery keys produce actual decision value without encouraging agents
  to treat retrieval rank as applicability.
- Whether proposal-research and review profiles retain enough shared structure
  to justify the common core under production use.
- Which launcher or authority-manifest mechanism should authenticate each
  production writer while preserving transport-neutral semantics.

## Authority

This proposal formation does not accept the parent proposal, settle its
permanent placement, prioritize either production phase, or authorize
implementation. It does not grant any role authority to publish a claim,
applicability assessment, reliance, proposal decision, review outcome, or
workflow transition. Those authorities must be named by the accepted domain
and implementation contracts.

## Acceptance consequence

If accepted and implemented, a fresh Codex or external review consumer can
discover, inspect, and rely on exact production evidence-backed statements with
truthful authority, scope, limitations, lineage, and projection provenance.
The same substrate is sufficient for the next phase to add active maintenance
and reliance propagation without changing stable identity or inventing a
second claim owner.
