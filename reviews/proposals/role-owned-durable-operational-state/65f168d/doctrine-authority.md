# Doctrine and Authority Review

Role: read-only doctrine and authority specialist. The initial pass used a
fresh role-scoped context; the `6460190` reassessment retained that context and
does not claim renewed independence.

Readiness consequence: `revise_before_review_closure`.

## Findings

1. **High — exploratory architecture is promoted into settled authority.** A
   formed control-plane hypothesis cannot become non-reopenable architecture
   merely because the proposal records it durably.

2. **High — attribution is not transition authorization.** For every
   authority-bearing transition, identify the authorized writer and verifier,
   exact subject and revision, scope, preconditions, expiry or revocation where
   relevant, and fail-closed treatment of stale or conflicting evidence.

3. **High — “handled” can suppress an event without proving its consequence.**
   Bind completion to the source event owner/revision, authorized disposition,
   and integrity-bound result. Separate delivery and execution attempts from
   completed consequences, and reconcile uncertain outcomes before suppression.

4. **High — reviewer dispositions collapse distinct owners.** Preserve the
   specialist's observation, builder response, reviewer re-evaluation,
   coordinator synthesis, residual-risk acceptance, gate result, and slice
   decision as separately attributed transitions with explicit visibility.

5. **Medium-high — cleanup can silently transfer or erase authority.** A
   handoff requires target-owner acceptance, integrity-bound lineage, and an
   authorized retirement consequence. Retention/tombstoning and physical
   deletion are separate decisions.

6. **Medium — model judgment lacks epistemic and invalidation structure.** A
   persisted judgment needs epistemic class, author/owner, evidence cutoff,
   governing premises or invalidation surface, freshness state, and consumer
   rules. Durability does not strengthen it.

7. **Medium — independently decidable authority changes are bundled.** Core
   recovery semantics, supervisor/builder recovery, reviewer recovery, planner
   continuity, and future control-plane integration should not receive one
   implicit acceptance or implementation consequence.

## Architecture correction consequence

The correction strengthens finding 1. It closes the architecture-document
concern about planning-layer authority sources and narrows the projection
concern, but leaves the proposal's handoff, visibility, cleanup, deletion, and
bundled-authority defects open. All other findings are unaffected.
