# Architecture and Placement Review

Role: fresh read-only architecture and placement specialist. Diagnostic and
advisory only; no proposal decision or implementation authority.

Readiness consequence: `revise_before_review_closure`.

## Findings

1. **High — the compaction idea states a binding invariant without an owning
   contract.** The note disclaims authority and placement, then says compaction
   “must” trigger specified behavior. Reframe this as an observed risk,
   candidate consequence, and formation hypothesis until an authorized
   role/workflow contract adopts it.

2. **Medium — claim identity and relationship topology are under-constrained.**
   Define what remains invariant across revisions, when a new claim identity is
   required, and relation-specific endpoints, direction, multiplicity,
   same-identity rules, and acyclicity. Blanket many-to-many language permits
   identity drift, cycles, and false latest-revision projections.

3. **Medium uncertainty — “review-local use is permitted” could be mistaken for
   implementation authority.** That wording is in the existing user-authored
   decision and cannot be silently changed. The new narrative may clarify that
   no implementation permission is inferred; decision-owner clarification is
   required before treating it as authority to build an adapter.

## Supported consequences

Canonical epistemic history versus rebuildable query projections, exact-revision
downstream reliance, non-authoritative implementation impact nomination,
reference-only role state, delivery without semantic ownership, and
mechanism-open storage remain well placed and compatible with the existing
defer-for-dogfooding decision.
