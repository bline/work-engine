# Placement: Claim-Centered Evidence Lineage

## Current candidate

A shared Git-backed evidence-lineage capability beneath proposal research and
review artifacts is the current candidate semantic owner. Its first adapter may
live adjacent to proposal packets because that is the first controlled consumer,
but physical proximity must not settle permanent ownership.

Canonical history and query acceleration are separate placement questions. The
candidate semantic owner preserves stable claim identity, immutable revisions,
refresh judgments, evidence baselines, and typed lineage. A SQLite edge table,
graph database, in-memory traversal, or other index may project that history for
many-to-many queries without becoming its semantic owner. Dogfooding should
record the required queries and observed scale before selecting a durable query
adapter.

Placement remains uncertain. The proposal-research workflow is not yet formed,
and review artifacts have not yet exercised the shared vocabulary across a real
revision. The current evidence establishes credible consumers, not a confirmed
common schema or runtime boundary.

## Plausible owners

### Shared evidence-lineage capability

This gives research claims and review findings common subject, baseline,
provenance, sensitivity, authority-scope, and lineage semantics while leaving
domain judgment with each workflow. It risks premature abstraction until both
consumers dogfood the proposed minimum.

### Proposal-research evidence layer

This is the broadest source of the current vocabulary and could own research
claims directly. It would make review artifacts depend on a research-specific
owner even when their reviewer-episode and synthesis semantics differ.

### Review-artifact layer

This has the first concrete findings from the bootstrap review and could host a
narrow first adapter. It is rejected as the assumed general semantic owner
because proposal research has distinct producers, consumers, and authority.

### Proposal packet itself

Rejected as an assumed owner. The packet owns proposal identity and current
meaning. It may reference evidence-lineage artifacts without expanding its
manifest into research history, review findings, or freshness state.

### Persistent runtime state

Rejected as an assumed canonical owner. Runtime machinery may later retain an
active reliance on an exact claim revision or a pending refresh obligation, but
it does not own the durable meaning or history of claims, evidence, or
judgments.

### Implementation-completion evidence

Neighboring producer, not semantic owner. Completion evidence may produce an
independently identified impact nomination that targets exact claim revisions
through `may_affect` and records the implementation's observed facts, change
evidence, baseline transition, and consequences already established under
separately named authority. It cannot declare a claim stale, false, refreshed,
applicable, or causally changed merely because implementation completed.

### Control-plane delivery or scheduler

Transport and coordination candidates, not evidence owners. A general delivery
capability may route a durable refresh obligation to an active authorized role.
The scheduler may share that delivery machinery for time-triggered work, but
neither mechanism acquires claim meaning, refresh authority, or role-activation
authority.

### Query projection

Generated view, not canonical owner. It may materialize reverse dependencies,
candidate-impact views, transitive reliance, and epistemic-churn statistics.
It must be rebuildable from canonical claim and judgment history and must expose
its source revision, projection version, and coverage limits.

## Placement certificate candidate

When a research or review role records an evidence-backed statement, that
domain workflow produces the judgment and a shared evidence-lineage boundary
preserves its identity, baseline, provenance, sensitivity, authority scope, and
lineage. A domain profile names any role authorized to maintain a canonical
live claim or produce an advisory applicability assessment. A downstream
decision owner decides whether to rely on that assessment, whether the evidence
is ready for its exact transition, and whether residual uncertainty is
acceptable within its authority. Dogfooding research and review examples must
prove that the shared fields remain truthful. This is not satisfied by storing
prose in a packet, reporting a changed file, or validating that required fields
exist.

## Reopening conditions

Reconsider placement when separately formed research and review consumers
exercise the candidate, when a real revision distinguishes shared from
domain-specific lineage, or when external evidence, concurrency, or recovery
requires semantics unavailable from Git-backed on-demand artifacts. Also
reconsider the canonical/query split when representative many-to-many traversals
show that the first projection cannot meet consumer scale or freshness needs,
without treating projection pressure as proof that a graph store should own
semantic history.
