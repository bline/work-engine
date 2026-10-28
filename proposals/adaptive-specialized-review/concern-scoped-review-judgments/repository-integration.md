# Repository Integration: Review Provenance Navigation

This document elaborates the repository integration and query composition
proposed by [`proposal.md`](proposal.md). Canonical review meaning remains with
the review contract and result capability.

*Revised 2026-10-10:* the earlier integration projected review records into
Codebase Memory through the `sync_projection` extension formerly proposed by
[`work-engine.claim-maintenance-and-reliance-propagation`](../../evidence-lineage/claim-maintenance-and-reliance-propagation/repository-integration.md).
That proposal has withdrawn the extension from its placement
([evidence](../../evidence-lineage/claim-maintenance-and-reliance-propagation/evidence/codebase-memory-correspondence-20261010.md)).
This revision replaces graph projection with composed observation and query.
The proposal's meaning is unchanged.

## Integration shape

```text
review-result capability (canonical)
  immutable ReviewResult and ReviewJudgment records
  declared anchors to exact subjects
  judgment lineage and typed review relationships
                     |
     +---------------+-----------------+
     |               |                 |
evidence-anchor   Code Change      Codebase Memory
observation       Profile          structural evidence
(read-only)       (exact deltas)   (read-only discovery)
     |               |                 |
     +---------------+-----------------+
                     ↓
       composed queries (repo-search, control plane)
```

Canonical owners preserve meaning, observers recover evidence, and query
composition provides navigation. No component writes review records into
another owner's store.

## Canonical records and typed relationships

The review-result capability owns results, judgments, references to the
exact concern revisions judged, finding references, evidence-set references,
and their typed relationships. The review contract, not the result capability,
owns concern identities and definitions. The relationships are:

- `REVIEWED_FOR`;
- `ASSERTED_INSPECTED`;
- `RELIED_ON`;
- `FINDING_ABOUT`;
- `VERIFIED_RESOLUTION_OF`;
- `SUCCEEDS_JUDGMENT`;
- `CARRIED_FORWARD_TO`; and
- `POSSIBLY_AFFECTED_BY`.

These are canonical review-domain relationships, not graph edges in another
store. Relationships between review records, such as succession and
carry-forward, resolve entirely within the review-result capability.
Relationships to code, documentation, and test subjects are recorded as
declared anchors (below).

Observed runtime activity remains separately owned, with relations such as
`PRESENTED_TO_RUNTIME`, `RETRIEVED`, `EXECUTED`, and `CAPTURED`. Mechanical
change evidence uses a relation such as `UNCHANGED_UNDER_COMPARATOR`. Neither
can be relabeled as asserted review meaning during query composition.

Judgments remain first-class records with stable identity, producer,
authority, evidence, limitations, disposition, and lineage.

## Composed observation and query

### Declared anchors

A judgment's dependencies on exact code, documentation, and test subjects are
declared anchors. The judgment that declares them owns them, as part of its
immutable record. The
[evidence-anchor substrate](../../../design/architecture/substrates/evidence-anchor.md)
observes each declared anchor against an exact repository revision, compares
it with its bound observation, and reports `matches`, `differs`, `unknown`,
`unsupported`, or `failed`. A difference may nominate `possibly_affected`. It
never decides semantic applicability.

Review judgments do not become claim-maintenance records, and claim
maintenance does not decide review relevance. Both reuse the same observation
contract; each owns its own declarations and dispositions.

### Codebase Memory as structural evidence

Codebase Memory is an optional, replaceable structural evidence provider. It
supplies entities, call and import structure, traversal, and change-driven
blast radius through a read-only evidence adapter. It stores no review
records, and the review ontology does not depend on its query syntax.

A structural question about reviews composes sources rather than joining
inside one graph. For example, "which judgments rely on something `X` calls?":

```text
Codebase Memory: callees of X at observed snapshot S
  -> exact subjects
  -> review anchor index: judgments declaring anchors on those subjects
  -> review-result capability: those judgments, lineage, and current status
```

Discovery results may suggest subjects to look up. They never satisfy, rebind,
or stand in for a declared anchor.

### Bounded, bidirectional navigation

Composed queries must remain efficient and bounded in both directions:

- **Forward**, from a result or judgment to its subjects, evidence, findings,
  and successors, reads canonical records directly.
- **Reverse**, from an exact subject or delta to the judgments that declare
  anchors on it, uses a derived anchor index over canonical declarations. Each
  entry preserves the exact anchor identity, subject kind, and the repository
  revision the anchor was declared against. The index records its cutoff over
  canonical history.

A path or qualified name is not continuous identity. Within the declaring
revision, the index matches exactly. Lookup for any other revision or delta
consumes explicit Code Change Profile correspondence to map current subjects to
declared ones. Where that correspondence is missing, ambiguous (for example,
a split or merge), or reports deletion, the affected anchors are reported as
unresolved or possibly affected with their cause. They never silently drop out
of the result.

Every composed result states the per-source cutoffs it combines: the
review-history cutoff, the anchor-index cutoff, the repository revision, and
the observed structural snapshot. It also states its bounds, truncation,
exclusions, and unresolved endpoints. A result that combines inconsistent
cutoffs is reported as incomplete, not merged.

### Durable meaning and disposable derivations

On-demand observation does not make review knowledge disposable. Results,
judgments, declared anchors, lineage, and attributed evidence references are
durable canonical records. They survive independently of any live structural
index.

Derived material is held to different standards:

- **Derived indexes**, such as the anchor index and other query accelerators,
  must be deterministically reconstructible from canonical records. Their loss
  degrades performance or freshness, never meaning.
- **Observations**, including anchor observations and structural snapshots, are
  disposable only when exact reconstruction is supported or their loss is
  explicitly represented. A dirty working tree, an unavailable evidence source,
  or a missing historical index can make a past observation irreproducible.
- **Non-reproducible observations retained as evidence**, for example ones a
  judgment cites, remain durable under their evidence owner's retention
  contract. They are not discarded as caches.

### Empty results are not negative evidence

An empty reverse lookup does not show that no judgments are affected. That
conclusion is permitted only when source coverage, revision binding, and
anchor-index completeness for the queried scope justify it. Otherwise, the
result reports `unknown` or `unavailable` with its cause. A clean result
remains bounded by its stated cutoffs and never proves complete review
coverage.

### Current observation dependency

Production code observation through Codebase Memory is not yet available. At
the claim-maintenance evidence cutoff, its supported interface could not
attribute observations to one coherent, content-verified snapshot. The
upstream report is
[#2585](https://github.com/DeusData/codebase-memory-mcp/issues/2585). The
evidence-anchor substrate is itself exploratory, and a production anchor
registry still needs an owning declaration contract. For review judgments,
that owner is the review-result capability. Until both dependencies are met,
reverse lookup through code observation reports `unknown` rather than
inferring from stale or mixed reads.

## Historical identity and change nomination

Code Change Profile supplies immutable before-and-after physical and structural
observations, including ambiguity around rename, move, split, merge, deletion,
and addition. A current Codebase Memory qualified name is not sufficient
historical identity.

Reverse lookup from changed entities may nominate affected judgments:

```text
changed entity observations
  -> Code Change Profile correspondence to declared subjects
  -> declared review anchors on those subjects (anchor index)
  -> candidate predecessor judgments
  -> relevant concern and perspective revisions
  -> possibly affected nominations
```

The result retains the change-profile revision, anchor-index cutoff,
observed snapshot, coverage, exclusions, and ambiguity. It cannot decide
judgment invalidation, carry-forward, reviewer selection, or acceptance.

## Role-facing retrieval

Agents consume bounded meaning through `repo-search`, not provider-specific
query languages or synchronization procedures. Useful requests include:

- review history for an exact code, documentation, test, claim, configuration,
  or UI-evidence revision;
- exact subjects, evidence, findings, and limitations supporting a result;
- possibly affected judgments for an exact candidate delta;
- unresolved anchors and endpoints;
- concern and perspective coverage at an exact cutoff; and
- provenance paths between findings, remediation, verification, and successor
  judgments.

Every evidence packet preserves the source owner, canonical references,
per-source cutoffs and observed snapshot, coverage, fallbacks, truncation,
unresolved endpoints, and limitations. A clean result does not prove complete
review coverage.

## Control-plane navigation

The control-plane UI consumes the same bounded query layer for two-way semantic
drill-down:

```text
review result -> judgments -> subjects -> evidence -> findings -> successors

code or semantic object -> review history -> perspectives -> judgments
                        -> findings -> later changes -> current bounded status
```

The UI distinguishes canonical review assertions, mechanically derived joins,
observed activity, possibly affected nominations, and current coverage views.
It cannot turn navigation, missing results, or freshness warnings into review
obligations or authority transitions.

## Degraded operation

Canonical result publication and episode recovery do not depend on a live
structural index or derived anchor index, unless an owning workflow
explicitly makes a bounded query result an acceptance condition. Index lag,
unavailable snapshots, unresolved subjects, and incomplete composition remain
visible. Consumers may use exact canonical reads and bounded direct-source
evidence without claiming complete coverage.
