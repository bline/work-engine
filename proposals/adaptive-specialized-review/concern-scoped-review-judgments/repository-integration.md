# Repository Integration: Review Provenance Projection

This document elaborates the repository and query projection proposed by
[`proposal.md`](proposal.md). Canonical review meaning remains with the review
contract and result capability.

## Integration shape

```text
immutable ReviewResult and ReviewJudgment records
                     +
exact code-change and evidence subjects
                     ↓
owner-scoped projection specification
                     ↓
Codebase Memory structural generation
                     ↓
repo-search and control-plane queries
```

Codebase Memory is the probable navigation and current-world projection
provider, not the review-result ledger, concern owner, historical identity
adjudicator, or coverage authority.

## Projected entities and relationships

Candidate projected nodes include result, judgment, concern revision, finding,
evidence-set reference, candidate, and unresolved endpoint. Candidate
relationships include:

- `REVIEWED_FOR`;
- `ASSERTED_INSPECTED`;
- `RELIED_ON`;
- `FINDING_ABOUT`;
- `VERIFIED_RESOLUTION_OF`;
- `SUCCEEDS_JUDGMENT`;
- `CARRIED_FORWARD_TO`; and
- `POSSIBLY_AFFECTED_BY`.

Observed runtime activity uses separately owned event projections such as
`PRESENTED_TO_RUNTIME`, `RETRIEVED`, `EXECUTED`, and `CAPTURED`. Mechanical
change evidence uses a relation such as `UNCHANGED_UNDER_COMPARATOR`. These
edges cannot be relabeled as asserted review meaning during projection.

Judgments should normally be reified nodes rather than bare edges because they
have stable identity, producer, authority, evidence, limitations, disposition,
and lineage. Projection edges provide navigation to and from their exact
subjects.

## Projection synchronization

The proposal relies on the bounded `sync_projection` consequence currently
formed by
[`work-engine.claim-maintenance-and-reliance-propagation`](../../evidence-lineage/claim-maintenance-and-reliance-propagation/repository-integration.md):

- projection owners can replace only their own namespace;
- preparation binds an exact repository subject, structural manifest,
  projection revision, and expected graph generation;
- endpoints resolve in the staging structural generation before atomic
  publication;
- unresolved endpoints remain visible with their expected identity and reason;
- readers receive projection revision, generation, freshness, completeness,
  exclusions, failures, and truncation; and
- graph publication never mutates canonical review records.

If that generic mechanism is not accepted or cannot preserve these
consequences, another replaceable projection provider may be used. The review
ontology does not depend on Codebase Memory-specific query syntax.

## Historical identity and change nomination

Code Change Profile supplies immutable before-and-after physical and structural
observations, including ambiguity around rename, move, split, merge, deletion,
and addition. A current Codebase Memory qualified name is not sufficient
historical identity.

Reverse traversal from changed entities may nominate affected judgments:

```text
changed entity observations
  -> projected exact-subject relationships
  -> candidate predecessor judgments
  -> relevant concern and perspective revisions
  -> possibly affected nominations
```

The traversal result retains structural generation, change-profile revision,
coverage, exclusions, ambiguity, and projection cutoff. It cannot decide
judgment invalidation, carry-forward, reviewer selection, or acceptance.

## Role-facing retrieval

Agents consume bounded meaning through `repo-search`, not provider-specific
Cypher or projection synchronization. Useful requests include:

- review history for an exact code, documentation, test, claim, configuration,
  or UI-evidence revision;
- exact subjects, evidence, findings, and limitations supporting a result;
- possibly affected judgments for an exact candidate delta;
- unresolved and stale projection endpoints;
- concern and perspective coverage at an exact cutoff; and
- provenance paths between findings, remediation, verification, and successor
  judgments.

Every evidence packet preserves source owner, canonical references, graph
generation, projection revision, coverage, fallbacks, truncation, unresolved
endpoints, and limitations. A clean traversal result does not prove complete
review coverage.

## Control-plane projection

The control-plane UI consumes the same bounded query layer for two-way semantic
drill-down:

```text
review result -> judgments -> subjects -> evidence -> findings -> successors

code or semantic object -> review history -> perspectives -> judgments
                        -> findings -> later changes -> current bounded status
```

The UI distinguishes canonical review assertions, mechanically derived graph
joins, observed activity, possibly affected nominations, and current coverage
projections. It cannot turn navigation, missing edges, or freshness warnings
into review obligations or authority transitions.

## Degraded operation

Canonical result publication and episode recovery do not depend on a live graph
projection unless an owning workflow explicitly makes a bounded query result an
acceptance condition. Projection lag, unavailable generations, unresolved
subjects, and incomplete traversal remain visible. Consumers may use exact
canonical reads and bounded direct-source evidence without claiming graph
completeness.
