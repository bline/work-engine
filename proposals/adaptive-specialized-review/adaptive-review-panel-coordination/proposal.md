# Proposal: Adaptive Review-Panel Coordination

## Identity and state

- Proposal ID: `work-engine.adaptive-review-panel-coordination`
- Family ID: `work-engine.adaptive-specialized-review`
- State: placement uncertain; not reviewed, evaluated, accepted, or authorized
- Decision owner: user or future explicitly authorized portfolio owner

The canonical lifecycle, placement, uncertainty, relationship, and authority
metadata is in [`packet.json`](packet.json). This narrative owns the proposal's
current meaning. Supporting artifacts describe its [placement](placement.md)
and [relationships](relationships.md).

## Candidate and consequence

Add an adaptive coordination capability that selects independent specialist
perspectives from the material claims and consequences of the actual review
subject, preserves relevant omission rationale, and synthesizes findings into
non-authoritative decision-readiness consequences.

A formed proposal or implementation revision can receive sufficient challenge
from perspectives capable of changing its downstream decision, while irrelevant
review work is omitted and unresolved consequences remain visible to the
authority that owns the decision.

## Proposed product change

The capability would:

- inspect the subject's claims, contracts, boundaries, uncertainty, reach, and
  evidence needs;
- select no model specialist, one focused specialist, or several distinct
  perspectives according to credible decision value;
- preserve why selected dimensions matter and explain material omissions;
- give independently claimed reviewers fresh, role-scoped subject and evidence
  context without contaminating initial judgments with other conclusions;
- retain supported findings, conflicts, limitations, and unresolved state
  without voting or averaging them away;
- synthesize consequences such as readiness for an authority decision,
  warranted revision, unresolved material uncertainty, or contract conflict;
  and
- bind outputs to revision-bound review artifacts so later applicability can be
  judged truthfully.

Potential specialist capabilities remain an open registry. Examples such as
architecture, doctrine, UI/process correspondence, accessibility, security,
state lifecycle, migration, evidence support, and validation feasibility are
affordances, not a fixed roster or routing table.

For the first controlled proposal-review vertical, the source identifies
architectural diagnosis and doctrine alignment as initial capabilities, with a
conditional UI/process reviewer as the representative dogfood case. That
initial subset is a candidate delivery boundary, not a claim that those roles
are universally required or that other materially relevant specialists may be
ignored.

## Invariants

### Selection follows consequence

Reason: a universal panel creates irrelevant work while a fixed small roster can
miss material seams.

Required property: selected perspectives have a credible relationship to the
subject's claims, consequences, uncertainty, or explicit review contracts.

### Independence claims match context

Reason: a reviewer inheriting another role's reasoning cannot establish the
same fresh-perspective claim.

Required property: provenance states the actual subject, evidence, context
isolation, and limitations supplied to each specialist.

### Synthesis preserves disagreement

Reason: a supported invariant conflict is not canceled by several positive
opinions.

Required property: synthesis retains findings, conflicts, uncertainty,
limitations, and next authority needs rather than reducing them to a vote.

### Review does not decide

Reason: diagnostic judgment and product authority have different owners.

Required property: selection, specialist findings, synthesis, proposal
revision, evaluation, acceptance, roadmap mutation, and implementation
authorization remain distinct transitions.

### Persistence does not inflate authority

Reason: a durable judgment records provenance but is not new evidence that the
judgment is correct.

Required property: no review artifact cites its persistence or downstream reuse
as independent support for itself.

## Boundary and placement

Proposal review is the immediate candidate consumer. Final placement remains
uncertain because the same substrate may serve implementation review, and
dogfooding has not established whether panel selection and synthesis need
separate owners or context lifetimes.

This candidate owns coordination judgment only if accepted. Specialist roles
own their scoped diagnoses, the review-artifact contract owns durable subject
and applicability semantics, the proposal former owns candidate revision, and
the named authority owns acceptance, deferral, rejection, or scope change.

## Relationships

This candidate causally depends on revision-bound review-artifact semantics. It
may consume architectural, doctrine, UI/process, strategic, security, or other
specialist capabilities without absorbing their judgment or authority.

## Uncertainty and evidence needs

- Dogfood panel selection on proposals with materially different claims,
  including a case where a seemingly available specialist is irrelevant.
- Compare a single coordination context with separated selection and synthesis
  contexts before choosing their lifetime boundary.
- Test whether the registry can remain open and model-interpreted without
  becoming ambiguous or procedural.
- Observe proposal revision to determine when prior findings remain applicable.
- Exercise implementation review later to test genuine substrate reuse rather
  than designing it from analogy.

## Authority

This formed candidate does not perform review, declare decision readiness,
revise or accept a proposal, change architecture or roadmap state, or authorize
implementation. Those transitions remain with their named owners.

## Acceptance consequence

If later accepted and implemented, a fresh decision owner can see why a panel
was selected, what each independent perspective established, what was omitted,
where findings conflict, and what remains unresolved without treating synthesis
as the decision itself.
