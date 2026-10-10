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

Repository integration adds two non-owning producers to that composition:

4. the slice lifecycle owns the accepted plan reference and immutable
   attributed checkpoint subject; and
5. the code-change profile owns recomputable physical and structural
   observations across that exact subject.

Neither producer owns semantic invalidation. They can supply evidence and
stable impact nominations to the shared claim-maintenance boundary.

This is a deliberate ownership composition, not evidence that one runtime
service should absorb all three boundaries.

## Dependency placement

The production claim-evidence interface owns the stable foundation. This phase
extends its versioned record and operation surface. It must not establish a
second claim registry, incompatible identity scheme, or provider-specific state
owner.

## Probable observation placement

*Revised 2026-10-10 on later evidence:*
[`evidence/codebase-memory-correspondence-20261010.md`](evidence/codebase-memory-correspondence-20261010.md).

Declared claim dependencies are evidence anchors owned by the declaring claim
revision. The evidence-anchor substrate observes and compares them and
derives nomination candidates. Codebase Memory is the probable code-observation
backend and a discovery source. It is not a host for materialized claim
projections. The earlier probable placement relied on a bounded
projection-synchronization extension (`sync_projection`). Later evidence
indicates that correspondence needs no external graph writes, so this
proposal no longer places that extension.

This is variant infrastructure placement, not permanent product doctrine.
Canonical Git-backed claim records remain sufficient to recompute every
derived correspondence. `repo-search` remains the role-facing evidence
abstraction, and the code-change profile owns cross-revision structural
reconciliation.

The placement depends on the observation backend attributing each
observation to one identifiable, coherent, content-verified snapshot, and on
unresolved anchors remaining explicit. At the evidence cutoff, Codebase
Memory's supported interface does not yet provide coherent snapshot
observation. Failure to obtain it reopens the backend placement; it does not
transfer semantic ownership to the graph or restore graph writes by default.

## Rejected owners

### Implementation completion

It may source candidate-impact nominations but cannot judge freshness,
applicability, or causality.

### Slice checkpoint or terminal receipt

It owns immutable repository and lifecycle evidence, not semantic entity
matching, affected-claim selection, or refresh judgment. A lossy terminal
projection cannot replace the full attributed subject required by an analyzer.

### Code-change profile

It may describe before/after files, entities, relationships, coverage, and
matching uncertainty. Its observations can nominate possible impact but cannot
decide that documentation, architecture, claim support, or downstream reliance
changed.

### Query projection

It derives reachability and candidate-impact views but cannot own semantic
history, canonical-support selection, or completion.

### Codebase Memory

It may supply structural observations, document-reference discovery,
change-driven reach, and correspondence candidates. It cannot become the
authored claim registry, infer historical identity across repository
revisions, manufacture canonical nominations, or turn unresolved anchors into
silent relationship deletion. Its discovery results cannot satisfy or rebind
a declared anchor.

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
for correctness rather than convenience or scale. Reconsider Codebase Memory's
probable observation-backend placement if it cannot provide coherent,
content-verified snapshot observation through a supported interface.
