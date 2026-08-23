# Architecture and Placement Review

Role: fresh read-only architecture and placement specialist. Diagnostic and
advisory only; no proposal decision or implementation authority.

- Subject: `30fa4295c53714590b93c68b9666134ffea294e7`
- Claude session: `237c98eb-057c-494e-a0cc-d9e0a42a1ca0`
- Readiness consequence: `revise_before_review_closure`

## Findings

### RIA-ARCH-001 — High — existing reconciliation ownership is unnamed

The repository already records file-level idea dispositions in
`ideas/reconciliation-map.json` and `ideas/README.md`, while the candidate gives
claim-level disposition meaning to intake records. The proposal does not say
whether the current reconciliation artifacts are prior evidence, migration
input, a superseded projection, or a permanently distinct coarser-grained
owner.

Without that boundary, the file-level map and a future claim-level assessment
can diverge for the same raw source without telling consumers which owner
governs which semantic grain. The proposal should name the relationship and
carry the transition or coexistence question into its evidence needs.

### RIA-ARCH-002 — Medium — capability placement is unaddressed

The proposal compares record placement under a top-level `idea-intake/` surface
and an `ideas/` hierarchy, but it does not place the intake capability itself
relative to the existing `proposal-former` and `proposal-packets` skills. An
implementation could therefore absorb interpretation into proposal formation,
contradicting the family boundary, or create an undocumented component.

Name a probable sibling capability owner or explicitly preserve capability
placement as a separate unresolved decision.

### RIA-ARCH-003 — Low — the candidate hierarchy collides with existing history semantics

The origin idea suggests `ideas/history/` for raw sources retained after
reconciliation. That path already contains
`ideas/history/2026-08-22-pre-reconciliation/`, which records a different
historical boundary. Carry this precedent into the placement comparison so a
later hierarchy decision does not give one path two incompatible meanings.

## Supported consequences and limitations

The proposed split among raw source, intake interpretation, proposal formation,
proposal packets, domain evidence, and authorized repository mutation is
internally coherent. The findings concern omitted relationships and placement
questions rather than a contradiction in that split.

The reviewer used the bounded proposal and named governing/origin sources. It
did not run an intake prototype or independently recompute the subject hashes.

