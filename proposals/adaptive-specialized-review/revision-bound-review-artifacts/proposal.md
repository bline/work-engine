# Proposal: Revision-Bound Review Artifacts

## Identity and state

- Proposal ID: `work-engine.revision-bound-review-artifacts`
- Family ID: `work-engine.adaptive-specialized-review`
- State: placement uncertain; not reviewed, evaluated, accepted, or authorized
- Decision owner: user or future explicitly authorized portfolio owner

The canonical lifecycle, placement, uncertainty, relationship, and authority
metadata is in [`packet.json`](packet.json). This narrative owns the proposal's
current meaning. Supporting artifacts describe its [placement](placement.md)
and [relationships](relationships.md).

## Candidate and consequence

Introduce a durable review-artifact contract that binds specialist findings and
synthesis to the exact immutable revision of the proposal or implementation
examined. A later consumer can determine who produced a finding, what evidence
and governing contracts it used, which subject revision it applies to, what
remains unresolved, and whether revision has made it stale, partially
applicable, refreshed, or superseded.

The contract would preserve decision support, not decisions. A stored finding
is evidence that a role made a judgment under recorded conditions; persistence,
stable identity, or downstream reuse does not make the judgment correct,
independent, accepted, or authoritative.

## Proposed product change

Define the smallest human-readable, mechanically checkable review-artifact
surface needed by real proposal-review consumers. Its candidate semantics are:

- immutable identity for the reviewed subject revision;
- specialist role, evidence scope, provenance, and applicable contracts;
- supported findings, limitations, conflicts, and unresolved consequences;
- an applicability relationship to later subject revisions;
- explicit refresh, partial-applicability, and supersession lineage; and
- a non-authoritative synthesis or decision-readiness consequence that remains
  distinguishable from an authority decision.

The proposal does not select a schema, storage engine, runtime service, or
review procedure. Those choices require consumer evidence and later authority.

## Invariants

### Exact subject identity

Reason: findings against one revision can become false or incomplete after the
subject changes.

Required property: every review artifact identifies the immutable subject
revision examined, and later reuse requires an explicit applicability judgment.

### Provenance is not correctness

Reason: durable or repeated judgments can otherwise become circular support for
themselves.

Required property: provenance records who judged what under which evidence; it
never promotes the judgment into independent evidence, acceptance, or authority.

### Review state is not proposal decision state

Reason: supported review findings can inform a decision without owning it.

Required property: findings, synthesis, resolution, proposal lifecycle, and
authority transitions remain distinguishable.

### Revision preserves useful lineage

Reason: replacing all prior review on every edit loses still-applicable evidence,
while silently carrying it forward hides staleness.

Required property: refresh and supersession preserve applicable provenance and
make invalidated claims visible.

## Boundary and placement

The probable initial shape is a Git-backed contract adjacent to proposal
packets, referenced rather than absorbed by `packet.json`. Placement remains
uncertain because the same semantics may be required by implementation review,
and no implemented consumer yet proves whether a proposal-local or shared
review owner is correct.

The proposal-packet manifest continues to own proposal identity, lifecycle,
uncertainty, relationships, and decision authority. This candidate owns only
review-subject and review-evidence semantics if accepted.

## Relationships

This candidate causally enables adaptive review-panel coordination by providing
the durable target for specialist outputs and synthesis. It is related to
proposal packets through subject references, but does not replace packet
identity or proposal lifecycle ownership.

## Uncertainty and evidence needs

- Dogfood a real formed proposal to determine which review consequences must be
  queryable or mechanically closed rather than narrative.
- Observe at least one proposal revision to test applicability and supersession.
- Determine whether implementation review requires the same artifact shape
  before choosing a proposal-local or shared owner.
- Identify any remediation lifecycle that genuinely needs runtime coordination
  rather than Git history.

## Authority

This formed candidate does not establish the artifact contract, review any
proposal, accept architecture, alter roadmap priority, or authorize
implementation. Those transitions remain with their named owners.

## Acceptance consequence

If later accepted and implemented, a fresh consumer can determine the exact
subject and applicability of review evidence without trusting a transcript,
mistaking persistence for correctness, or confusing review readiness with an
authority decision.

