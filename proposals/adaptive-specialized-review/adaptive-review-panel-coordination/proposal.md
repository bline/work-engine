# Proposal: Adaptive Review-Panel Coordination

## Identity and state

- Proposal ID: `work-engine.adaptive-review-panel-coordination`
- Family ID: `work-engine.adaptive-specialized-review`
- State: placement uncertain; revised after bootstrap review continuation and
  not closure-reviewed, evaluated, accepted, or authorized
- Decision owner: user or future explicitly authorized portfolio owner

The canonical lifecycle, placement, uncertainty, relationship, and authority
metadata is in [`packet.json`](packet.json). This narrative owns the proposal's
current meaning. Supporting artifacts describe its [placement](placement.md)
and [relationships](relationships.md).

## Candidate and consequence

Add an adaptive coordination capability that selects independent specialist
perspectives from the material claims and consequences of the actual review
subject, preserves truthful selected and omitted outcomes, and synthesizes
finding-linked, non-authoritative consequences for a named downstream decision
owner.

A formed proposal or implementation revision can receive sufficient challenge
from perspectives capable of changing its downstream decision, while irrelevant
review work is omitted, failed or unavailable work is not reported as passed,
and unresolved consequences remain visible to the authority that owns the next
decision.

## Proposed product change

The capability would:

- inspect the subject's claims, contracts, boundaries, uncertainty, reach,
  research limitations, and evidence needs;
- select no model specialist, one focused specialist, or several distinct
  perspectives according to credible decision value;
- preserve why selected dimensions matter and record truthful outcomes for
  material omissions, inapplicability, deterministic-only evidence,
  unavailability, failure, no findings, and findings;
- give independently claimed reviewers fresh, role-scoped subject and evidence
  context without contaminating initial judgments with other conclusions;
- preserve continuation context when useful while labeling it as continuation
  rather than another fresh review;
- ensure the authoritative review artifact accounts for every material finding
  and conflict produced by selected review episodes with an attributed,
  profile-defined disposition and reason, without voting or averaging them
  away;
- synthesize consequences such as advisory readiness for an authority decision,
  warranted revision, unresolved material uncertainty, or binding-contract
  conflict;
- reference every retained finding and conflict in synthesis, keep every other
  material output visibly dispositioned, and distinguish coordinator inference
  from specialist findings; and
- bind outputs to revision-bound review artifacts so later freshness and
  decision-specific applicability can be judged by their authorized owners.

Potential specialist capabilities remain an open registry. Examples such as
architecture, binding doctrine, philosophical alignment, UI/process
correspondence, accessibility, security, state lifecycle, migration, evidence
support, and validation feasibility are affordances, not a fixed roster or
routing table.

For the first controlled proposal-review vertical, architectural diagnosis and
doctrine/authority review remain plausible initial capabilities, with a
conditional UI/process reviewer as a representative omission case. This is a
candidate dogfood boundary, not a universal panel.

## Invariants

### Selection follows consequence

Reason: a universal panel creates irrelevant work while a fixed small roster can
miss material seams.

Required property: selected perspectives have a credible relationship to the
subject's claims, consequences, uncertainty, evidence limitations, or explicit
review contracts. Availability alone does not justify selection.

### Independence claims match context

Reason: a reviewer inheriting another role's reasoning cannot establish the
same fresh-perspective claim.

Required property: provenance states the actual subject, evidence, context
isolation, inheritance, and limitations supplied to each review episode.

### Omission and failure remain truthful

Reason: no requested review, deliberate omission, deterministic-only evidence,
provider unavailability, reviewer failure, and completed review have different
decision consequences.

Required property: coordination emits the corresponding review-artifact outcome
and reason. It never projects an omitted, unavailable, blocked, or failed stage
as passed.

### Synthesis preserves disagreement and source identity

Reason: a supported invariant conflict is not canceled by several positive
opinions, and a coordinator inference is not an independent finding.

Required property: synthesis retains and references findings, conflicts,
uncertainty, limitations, and next authority needs rather than reducing them to
a vote. In the authoritative review artifact, coordination accounts for every
material finding and conflict produced by selected episodes with an attributed
disposition and reason from the applicable domain profile. The contract does
not prescribe the coordinator's internal ordering or impose one universal
exhaustive disposition list. Any synthesis-introduced inference remains
attributed to the coordinator.

### Binding conflict and philosophical alignment are distinct

Reason: `DESIGN.md` and accepted contracts define invalid states, while
`PHILOSOPHY.md` supplies non-normative guidance that may reveal tension without
creating a blocking command.

Required property: a `contract_conflict` consequence cites binding doctrine, an
accepted contract, invariant, or authority boundary. Philosophy-only alignment
findings are advisory and nonblocking unless they expose a separately supported
binding conflict.

### Review does not decide applicability, readiness, or acceptance

Reason: diagnostic judgment and downstream authority have different owners.

Required property: selection, specialist findings, synthesis, any
profile-specific freshness maintenance, advisory applicability assessment,
downstream reliance, decision readiness, proposal revision, evaluation,
acceptance, roadmap mutation, and implementation authorization remain distinct
transitions. A synthesis or another authorized role may provide an attributed
advisory applicability assessment; the named decision owner decides whether to
rely on it and judges readiness and residual uncertainty for that exact
transition.

### Persistence does not inflate authority

Reason: a durable judgment records provenance but is not new evidence that the
judgment is correct.

Required property: no review artifact cites its persistence, repetition,
composition, or downstream reuse as independent support for itself.

## Boundary and placement

Proposal review is the immediate candidate consumer. Final placement remains
uncertain because implementation review may share selection and synthesis
behavior, and dogfooding has not established whether panel selection and
synthesis need separate owners, context lifetimes, or durable-state lifecycles.

This candidate owns coordination judgment only if accepted. Specialist roles
own their scoped diagnoses. The review-artifact profile owns review
representation. The shared evidence-lineage candidate owns only its proposed
cross-domain statement semantics if later selected as their owner. A domain
profile names a freshness-maintenance role only when needed and may authorize
an advisory applicability assessor. The downstream decision owner decides
whether to rely on that assessment and judges readiness. The proposal former
owns candidate revision, and the named authority owns acceptance, deferral,
rejection, or scope change.

## Relationships

This candidate causally depends on revision-bound review-artifact semantics and
therefore indirectly consumes the candidate shared evidence-lineage foundation.
It may invoke architectural, doctrine, philosophical-alignment, UI/process,
strategic, security, or other specialist capabilities without absorbing their
judgment or authority.

## Uncertainty and evidence needs

- Dogfood selection on proposals with materially different claims, including a
  case where a seemingly available specialist is irrelevant.
- Exercise truthful omitted, unavailable, failed, no-finding, and finding-bearing
  outcomes.
- Compare a single coordination context with separated selection and synthesis
  contexts before choosing their lifetime boundary.
- Test whether the registry can remain open and model-interpreted without
  becoming ambiguous or procedural.
- Review a revised proposal to exercise finding applicability and retained
  remediation context, using a fresh-perspective reset only when a material
  premise or context-fitness judgment warrants it.
- Verify that synthesis references preserve conflicts and that a downstream
  decision owner can make applicability and readiness judgments without the
  coordinator acquiring them.
- Exercise implementation review later to test genuine coordination reuse
  rather than designing it from analogy.

## Authority

This revised candidate does not perform review, declare semantic freshness,
produce or rely on an applicability assessment, decide readiness, revise or
accept a proposal, change architecture or roadmap state, waive binding
doctrine, or authorize implementation. Those transitions remain with their
named owners.

## Acceptance consequence

If later accepted and implemented, a fresh decision owner can see why a panel
was selected, what actually happened in each review episode, what each
perspective established, what was omitted or failed, where findings conflict,
how synthesis used them, and what remains unresolved without treating review
coordination as the decision itself.
