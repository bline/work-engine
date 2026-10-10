# Repository Integration: Claim Maintenance and Reliance Propagation

This document elaborates the initial bidirectional code-and-documentation
integration summarized in [`proposal.md`](proposal.md). Canonical claim meaning
and authority remain owned by the proposal narrative and its production
dependency.

## Integration consequence

```text
immutable repository subject + accepted plan
                    ↓
physical and structural change profile
                    ↓
current claim-maintenance projection
                    ↓
bounded related-claim and unmapped candidates
                    ↓
canonical may_affect nomination
                    ↓
authorized refresh judgment
```

The route is symmetric. A code change may nominate documentation, claim
support, tests, schemas, configuration, and consumer reliance. A normative
documentation change may nominate implementation and verification surfaces
that may no longer realize its meaning.

## Planned and actual inputs

The slice lifecycle supplies two distinct inputs rather than one reconstructed
scope:

- the accepted plan references an integrity-bound expected semantic-impact
  artifact, including semantic targets believed relevant, expected
  documentation effect, material uncertainty, and the plan or scope revision;
  and
- the immutable candidate or accepted checkpoint supplies the actual
  attributed manifest, before and after trees, task-patch identity, and
  observed change profile.

Planned-versus-actual comparison is evidence of implementation drift or an
omitted expectation. It does not decide that a claim changed. The full
checkpoint lifecycle receipt or a lossless derived subject must have a durable
owner before a terminal projection discards path attribution or before model
context becomes its only copy.

Mechanical nomination is bound to the immutable review candidate when its
output is needed to detect omitted scope before acceptance. A refresh that is
itself an acceptance condition completes before the slice is accepted. Other
refresh and downstream-reconsideration consequences may begin from the
accepted implementation-completion event and remain durable post-slice
obligations. "Post-slice" timing never permits a required documentation
consequence to be silently deferred past its owning gate.

## Historical structural identity

A replaceable change-profile analyzer derives physical and structural entity
lineage across two immutable repository worlds. Candidate observations include
unchanged identity, rename or move, split, merge, modified structure, addition,
deletion, and unresolved matching for code symbols, tests, schemas,
configuration, and documentation anchors.

Every observation retains analyzer identity, evidence cutoff, coverage,
matching method, confidence or ambiguity, and exact repository subjects. A
current-only graph, raw touched-file list, or line-number comparison is
insufficient evidence for a historical identity delta.

Stable claim identity survives ordinary refactors. Changed support creates
immutable successor revisions; removals retain non-destructive history rather
than silently deleting meaning. Structural matching may nominate possible
impact but cannot decide claim identity, causality, applicability, or
retraction.

## Current-world correspondence

The claim-maintenance view joins current structural entities to exact claim
revisions, authored content references, sensitivities, and active reliance. It
is derived on demand from canonical records and current observations. It is
not a separately published or materialized store, so losing it requires only
recomputation, not human reconstruction.

Derived results point back to their exact canonical record and revision.
Deriving them never creates or revises canonical claims, authored
relationships, nominations, judgments, reliance, or obligations, and acquires
no authority to replace or tombstone their canonical sources.

The view can report newly observed behavior or normative documentation
meaning with no mapped claim or relationship as an explicit unmapped
candidate. A clean result means only that no candidate was found within the
declared snapshot, coverage, exclusions, and traversal bounds.

## Probable observation placement

*Revised 2026-10-10 on later evidence:*
[`evidence/codebase-memory-correspondence-20261010.md`](evidence/codebase-memory-correspondence-20261010.md).
The earlier probable placement materialized compiled claim projections inside
Codebase Memory through a bounded, owner-isolated synchronization extension
provisionally called `sync_projection`. Shadow experiments against a later
Codebase Memory revision indicate that correspondence needs no external graph
writes. This revision therefore withdraws that extension from this proposal's
placement. The proposal's meaning and protected consequences are unchanged.

### Declared anchors

A claim's code and documentation dependencies are declared anchors, owned by
the declaring claim revision. The
[evidence-anchor substrate](../../../app-server/docs/architecture/substrates/evidence-anchor.md)
observes each declared anchor against a current snapshot, compares it with its
bound observation, and derives `may_affect` nomination candidates. Claim
maintenance alone publishes durable nominations. The exact code locator and
comparator richness remain evidence-anchor's open questions; this proposal
does not decide them.

### Codebase Memory as observation and discovery

Codebase Memory remains the probable code-observation backend and a discovery
source. It provides:
- structural entities and traversal;
- indexed document references with reason-coded unresolved rows;
- change-driven blast radius; and
- term-scored documentation-to-code candidates.

Discovery results may nominate unmapped candidates. They never satisfy,
rebind, or stand in for a declared anchor. Declared anchors are not resolved
through indexed document references, whose observed fallbacks can reduce
precision without recording it.

### Preserved consequences

The consequences the withdrawn extension was meant to protect remain binding:

- **Namespace ownership.** No external graph writes occur, so nothing can
  overwrite parser-produced structure or another owner's records. Canonical
  records stay outside the graph.
- **Snapshot binding.** Every observation is attributable to one
  identifiable, coherent, content-verified index snapshot and to the exact
  canonical revision it was interpreted against:

  ```yaml
  observed_index_snapshot: ...
  repository_subject: commit-or-checkpoint:...
  structural_manifest_digest: sha256:...
  canonical_revision: ...
  ```

  The exact fields may vary, but the binding distinguishes the immutable
  repository lineage from the precise indexed content, including dirty
  working-tree content. A stale or mixed observation fails visibly as
  `unknown`; it never attaches canonical references to the wrong repository
  world.
- **Completeness.** No separate projection is published, so no reader can
  observe a structural generation that lacks its evaluated projection. An
  observation is complete only when all its dependent reads share one
  snapshot.
- **Truthful unresolved state.** An anchor whose target no longer resolves is
  reported with its canonical source, expected endpoint, snapshot, and cause.
  That is `differs` when absence is established, or `unknown` when coverage
  cannot establish it. Successor candidates never rebind. The code-change
  profile may later supply relocation evidence; only the authorized
  claim-maintenance boundary publishes the resulting canonical disposition.
- **Observation-only authority.** Codebase Memory and evidence-anchor never
  author canonical meaning. Graph reachability alone cannot manufacture a
  nomination.

### Current observation dependency

At the evidence cutoff, Codebase Memory's supported interface cannot yet
attribute observations to one coherent snapshot:
- its exposed generation does not change on incremental re-indexes;
- snippets combine indexed positions with current disk bytes; and
- freshness is judged by mtime and size.

Until a supported coherent-snapshot observation exists, production
correspondence through Codebase Memory cannot establish snapshot binding.
This is an observation-interface dependency, not a reason to restore graph
writes.

## Role-facing retrieval

Roles consume this capability through `repo-search`, not provider-specific
Cypher or synchronization behavior. A claim-aware impact request may specify
changed entities, direction, and a relationship profile. Its result includes:

- the bounded related structural and projected entities;
- traversal paths and relationship types;
- exact canonical revision and observed index snapshot;
- repository subject and freshness;
- coverage, exclusions, failed inputs, and truncation; and
- unresolved endpoints and unmapped candidates.

`repo-search` owns evidence retrieval, not canonical nomination. Claim
maintenance decides whether retrieved candidates justify a durable
`may_affect` record.

## Dependent consumers

[`work-engine.concern-scoped-review-judgments`](../../adaptive-specialized-review/concern-scoped-review-judgments/repository-integration.md)
relies on the `sync_projection` consequence this document formerly formed, to
project review-judgment nodes into the graph. This revision withdraws the
mechanism from this proposal's placement only. It does not decide the review
proposal's placement. That proposal's own fallback applies. Whether review
navigation independently establishes a product consequence for externally
materialized graph records is its owner's question, and it is this document's
reopening condition for a generic projection capability.

## Reopening conditions

Reconsider Codebase Memory as the probable observation backend if it cannot
expose coherent, content-verified snapshot observation through a supported
interface. Reconsider withdrawing graph materialization if a consumer
establishes a product consequence that on-demand correspondence cannot
preserve, or if independent consumers establish a separate product
consequence for a generic projection-synchronization capability.
