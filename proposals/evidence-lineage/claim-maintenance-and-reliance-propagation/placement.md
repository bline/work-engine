# Placement: Claim Maintenance and Reliance Propagation

## Split ownership

This proposal has one semantic chain with three irreducible owners:

1. the shared claim-evidence capability owns nomination, refresh, support
   selection, reliance, and obligation record identity, schema, lineage,
   validation, and projection semantics;
2. each domain workflow owns claim meaning, refresh judgment, applicability,
   reliance, selective reopening, and semantic completion; and
3. a general control-plane capability may own cross-owner delivery, receipt,
   retries, and reconciliation without owning claim meaning or activation.

This is a deliberate ownership composition, not evidence that one runtime
service should absorb all three boundaries.

## Dependency placement

The production claim-evidence interface owns the stable foundation. This phase
extends its versioned record and operation surface. It must not establish a
second claim registry, incompatible identity scheme, or provider-specific state
owner.

## Rejected owners

### Implementation completion

It may source candidate-impact nominations but cannot judge freshness,
applicability, or causality.

### Query projection

It derives reachability and candidate-impact views but cannot own semantic
history, canonical-support selection, or completion.

### Role state

It may reference exact relied-upon revisions and pending obligations but cannot
copy or advance claim history.

### Scheduler

It may share delivery machinery but cannot own claim obligations, activate a
role, or decide that refresh completed.

### MCP or provider session

Transport and retained context may expose or assist the workflow but cannot own
durable semantic state or authority.

## Reopening conditions

Reconsider the ownership split if real domain workflows cannot publish and
recover one semantic consequence across the boundaries, if delivery cannot
remain generic without losing obligation meaning, if branching support needs a
new domain owner, or if an always-running canonical service becomes necessary
for correctness rather than convenience or scale.
