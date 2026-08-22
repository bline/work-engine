# Lifecycle and Evidence Review

Role: read-only review-evidence lifecycle specialist. Consequence: semantic and
feasibility diagnosis only; no acceptance or implementation authority.

Subject: the exact revision and content set in [subject.md](subject.md).

## Supported conclusions

- The family correctly separates durable review-evidence semantics from adaptive
  coordination and preserves their causal dependency without imposing order.
- Exact subject binding, provenance, applicability, initial freshness when
  independence is claimed, non-authoritative synthesis, and conflict
  preservation are the right invariant-level consequences.
- Initial independence and retained remediation context can coexist. A
  continuation can retain the isolated reviewer's context if provenance does
  not falsely describe it as another fresh review.
- Deterministic machinery can check identities, references, status vocabulary,
  provenance presence, lineage edges, allowed transitions, and
  non-authorization. It cannot establish semantic support, correct
  applicability, wise omission, meaningful independence, or faithful synthesis.

## Findings

1. **High, observed — minimum subject identity is undefined.** A repository
   commit can identify a state without identifying whether the review concerned
   the whole tree, one packet family, selected paths, or an implementation
   delta. Define at least repository namespace, subject kind/stable ID,
   immutable revision, and bounded subject-content identity.
2. **High, observed — lineage meanings and ownership are underspecified.**
   Applicability, partial applicability, refresh, composition, and supersession
   are distinct. Partial applicability must identify surviving claims; refresh
   is a new observation; composition does not imply joint review; supersession
   changes downstream preference without declaring the old finding false.
3. **High, observed — per-finding identity is absent.** Stable finding identity,
   reviewed claim, material assumptions, support references, and per-finding
   lineage are needed for partial applicability, conflicts, correction, and
   selective supersession.
4. **High, observed/inferred — synthesis lacks reference integrity.** A synthesis
   must reference source finding identities and distinguish its own inferences.
   Mechanical checks can preserve links and conflicts without pretending to
   judge synthesis quality.
5. **Medium, observed — empty and failure outcomes collapse.** Distinguish not
   requested/not applicable, deliberately omitted, deterministic evidence used,
   unavailable/blocked/failed, completed with no findings, and completed with
   findings. Absence must not be reported as success.
6. **Medium, inferred — review episodes need context lineage.** Distinguish fresh
   initial review from `continuation_of`; record actual context inheritance and
   bind each pass to its subject or delta.
7. **Medium, observed — transition actors are unnamed.** Provenance must
   distinguish reviewer observation, coordinator judgment, proposal-former
   revision, and authority decision.
8. **Medium, observed — correction/invalidation is missing.** Non-destructive
   correction or invalidation must remain distinct from supersession.

## Conflicts and uncertainty

- A whole-tree reading treats commit/tree plus narrative scope as sufficient;
  a bounded-subject reading requires a stable subject and content-set digest.
  The promised mechanical consequence favors the latter.
- A prose-first first vertical could leave lifecycle meanings narrative; a
  mechanically checkable first vertical requires closed identities, relation
  types, transition provenance, and non-success outcomes.
- Whole repository versus packet/family/path-set/delta granularity, the
  applicability authority, closed evidence fields, context lifetimes, and any
  need for runtime coordination remain unresolved.

## Limitations and applicability

No schema, storage design, product-value decision, real proposal revision,
remediation loop, or implementation-review consumer was tested. Re-review if
the proposal, packet contracts, accepted remediation consequences, or governing
doctrine change.

Feasibility verdict: feasible after semantic-contract revision.

Readiness consequence: `revision_warranted`.
