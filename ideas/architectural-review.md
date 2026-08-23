# Architectural Diagnostic Review

## Status

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
