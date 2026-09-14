# Architectural Diagnostic Review

## Status

**Accepted 2026-09-14** (explicit user decision after reviewing this reconciliation): the
reconciliation's findings are confirmed accurate. The stated residue — an architectural-
finding domain profile coupled to `claim-evidence-service.md`'s existing pattern, plus the
routing step from a recommendation to whichever owning decision boundary applies — is
authorized for **design work only, not implementation yet**, pending the domain-profile
coupling decision and the routing design.

Reconciled jointly with `ideas/cross-cutting-seam-review.md` against current app-server
implementation and prospective architecture in
`app-server/docs/cross-cutting-seam-review-and-architectural-review-reconciliation.md`
(per the `COUPLED_RECONCILIATION` flag in `app-server/docs/root-ideas-reconciliation.md`).
Its "Blocking consequence remains separately owned" boundary is confirmed achievable:
`app-server/roles/strategic-planning-handoff.mjs` is a real, concrete campaign-level
consumer (verdicts: continue/revise/pause/reorder/split_campaign/stop_campaign, dogfooded
in `app-server/docs/post-migration-strategic-plan.md`) — but not the universal owner of
every architectural-finding consequence; proposal reconsideration, architecture-direction
reopening, and human authority are other named owning boundaries. The remaining open scope
is an architectural-finding domain profile coupled to `claim-evidence-service.md`'s
existing domain-profile pattern (not yet built), plus a routing step from a pause/stop
recommendation to whichever owning decision boundary actually applies. See the
reconciliation document for the full disposition.

Exploratory capability idea.

## Idea

Provide a dedicated diagnostic review that asks whether the **current system model, ownership, decomposition, or placement appears wrong**.

Its output is evidence for proposal formation. It does not design or approve the repair.

## Current evidence

Work Engine already separates:

- campaign supervision from repository-domain work;
- proposal formation from proposal decision authority;
- independent implementation review from builder judgment; and
- strategic planning from campaign execution.

What is still missing is a reusable capability whose explicit subject is the architecture itself.

## Required consequence

A useful architectural review can identify:

- the architectural claim being challenged;
- the observed symptoms and supporting evidence;
- suspected ownership, placement, or decomposition defects;
- affected contracts or invariants;
- credible competing explanations;
- confidence and limitations;
- the consequence of continuing without reconsideration; and
- conditions that should reopen or deepen the review.

The result should be diagnostic enough that proposal formation can work from it without repeating the same investigation.

## Authority boundary

Architectural review:

- may diagnose;
- may challenge current ownership or placement;
- may recommend proposal formation or reopening;
- may identify that continued execution is unsafe when the evidence is material;
- may emit an explicitly attributed recommendation to pause or stop, including
  severity, confidence, expected consequence of continuing, and limitations.

It does not:

- author the final architecture;
- accept a proposal;
- authorize implementation;
- mutate the repository;
- replace strategic planning; or
- certify its own proposed repair.

### Blocking consequence remains separately owned

The historical idea allowed a material architectural finding with at least
medium confidence to block continued execution. This clean capability retains
the need to represent that consequence, but does not silently grant blocking
authority to the reviewer.

Whether a finding actually pauses a campaign belongs to the owning campaign,
planning, or human-authority contract. A future adoption decision must identify:

- who may disposition a stop recommendation;
- whether any finding class is automatically blocking;
- the materiality and confidence evidence required;
- what can continue safely while disposition is pending; and
- how the stop and later resumption are recorded.

Until that authority is established, the review produces diagnostic evidence
and an escalation recommendation, not an authoritative workflow transition.

## Relationship to other ideas

- **Proposal research maturity** determines whether enough evidence exists to rely on the diagnosis for a later decision.
- **Cross-cutting seam review** judges coherence across boundaries after or around concrete changes; it is not a substitute for architectural diagnosis.
- **Organizational execution envelopes** may eventually consume architecture-qualified requirements, but do not own this diagnostic function.

## Compact statement

> Architectural review diagnoses the system model. Proposal formation decides what change to propose.
