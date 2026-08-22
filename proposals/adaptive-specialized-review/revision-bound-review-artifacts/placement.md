# Placement: Revision-Bound Review Artifacts

## Current candidate

A Git-backed contract adjacent to the proposal-packet capability is a plausible
initial owner. It would preserve review evidence as references from proposal
state without expanding `packet.json` into a review record or moving proposal
meaning into runtime state.

Placement remains uncertain. The source anticipates reuse by implementation
review, and current evidence does not establish whether the long-term owner is
proposal-review infrastructure or a shared cross-phase review substrate.

## Plausible owners

### Proposal-review planning layer

This is the narrowest first consumer and matches the strategic goal of a
controlled proposal lifecycle. It risks making later implementation reuse an
afterthought if the lifecycle semantics are actually shared.

### Shared review-capability layer

This preserves a common identity and applicability contract across proposal and
implementation subjects. It is premature until a second consumer establishes
shared requirements rather than superficial similarity.

### Proposal packet itself

Packet references may be sufficient for an initial vertical, but absorbing full
review evidence into the packet would mix proposal lifecycle ownership with
review provenance and applicability.

### Persistent runtime state

Rejected as an assumed primary owner. Runtime state may preserve active review
obligations when recovery requires it, but that does not make runtime storage
the canonical owner of review meaning or history.

## Reopening conditions

Reconsider placement when a real proposal revision exercises applicability,
an implementation-review consumer supplies discriminating requirements, or
observed concurrency establishes semantics unavailable from Git-backed state.

