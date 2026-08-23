# Adaptive Specialized Review Panels

## Status

Idea / architecture proposal.

This document preserves current design consequences for later proposal
formation. It is not a product contract, a fixed review procedure, an accepted
proposal, or authority to change roadmap or implementation state.

## Motivation

Proposal review covers a wider semantic space than ordinary implementation
review. A proposal may make consequential claims about architecture, product
value, user experience, doctrine, evidence, state ownership, migration,
security, validation feasibility, and strategic timing before an implementation
exists.

No single reviewer is naturally authoritative across all of those dimensions.
Running every conceivable reviewer would be expensive, repetitive, and
procedural. It would also create review work with no credible decision value,
such as UI review for a change with no rendered or interaction consequence.

The intended capability is therefore an adaptive review panel selected by a
capable model from the material claims and consequences of the actual subject.

## Core consequence

A formed proposal or implementation revision can receive sufficient independent
semantic challenge from the specialist perspectives that matter to its
acceptance, while irrelevant reviewers are omitted and unresolved consequences
remain visible to the owning decision authority.

The panel establishes decision support. It does not exercise product,
architecture, roadmap, scope, or implementation authority.

## Formation, review, evaluation, and decision remain distinct

```text
formation
→ what coherent candidate change is being considered

review
→ whether its material claims are coherent, supported, and decision-ready

evaluation
→ expected impact, feasibility, cost, risk, and value

decision
→ whether the owning authority accepts, defers, rejects, or requests revision
```

Formation may challenge its own working interpretation, but that is not a
substitute for independent proposal review. Review may produce evidence useful
to evaluation, but review does not own scoring or prioritization. Neither
formation, review, nor evaluation authorizes implementation.

## Review planning is model judgment

Review selection should consider, as useful:

- the proposal or implementation's material claims and intended consequences;
- affected contracts, owners, boundaries, and seams;
- placement uncertainty and architectural reach;
- reversibility, fan-out, migration burden, and failure consequence;
- rendered, interactive, security, privacy, persistence, or authority effects;
- what evidence already exists and what remains unresolved;
- which independent perspective could credibly change the downstream decision;
- explicit review or independence requirements owned by an accepted contract;
- available reviewer capabilities, limitations, context cost, and provenance.

These are affordances for judgment, not a deterministic routing table. The
review planner may select no model reviewer when deterministic evidence is
sufficient, one focused reviewer for a narrow semantic change, or several
specialists for a cross-cutting proposal. Initial findings may justify revising
the panel when they reveal a material dimension that was not previously known.

Review selection should preserve the dimensions selected and why. Omission
rationale has value when a seemingly relevant dimension was deliberately
excluded or an explicit requirement was judged inapplicable. It should not
create administrative ceremony around every obviously irrelevant capability.

## Open specialist capability registry

Potential reviewers include:

- architectural diagnosis and placement;
- UI-to-process correspondence and interaction design;
- accessibility when a rendered or interactive consequence makes it material;
- doctrine alignment against binding `DESIGN.md` structure;
- principles-balance or seam coherence;
- evidence, confidence, and epistemic support;
- user authority and ownership correspondence;
- state, identity, lifecycle, recovery, and concurrency;
- security and privacy;
- operability, compatibility, and migration;
- validation feasibility and proof adequacy;
- documentation-to-implementation correspondence.

This list is illustrative rather than exhaustive. Adding a specialist should
make a materially distinct question answerable, not add another equivalent
route to generic review.

## Doctrine and philosophy

`DESIGN.md` is normative product doctrine. `PHILOSOPHY.md` is non-normative
rationale.

A doctrine-alignment reviewer may identify a blocking conflict only by tracing
it to a binding contract, authority boundary, or accepted consequence.
Philosophical tension may explain risk, expose likely proceduralization, or
motivate redesign, but disagreement with non-normative philosophy is not itself
a product-invalid state.

## Architectural reviewer

Architectural review is diagnostic.

During proposal design it may challenge ownership, placement, boundaries,
coupling, architectural assumptions, and the consequence of continuing with the
current system model.

During implementation it may determine that the accepted placement premise has
become stale or that observed reach crosses a material boundary.

It may not mutate accepted architecture, rewrite a proposal, reopen scope,
amend a campaign, or authorize a redesign. Its output creates a consequence
that the proposal former, supervisor, planner, human, or other owning authority
must act on.

The same architectural-review capability may serve both phases through
different evidence and output projections. This does not require the same model
context. Fresh independent falsification and retained diagnostic continuity
have different information-lifetime consequences.

## Strategic planner

During proposal design, the planner may advise on dependencies, priority,
timing, adjacent proposals, expected value, and whether a candidate should be
split, merged, deferred, or reconsidered.

During implementation, the planner may conclude that evidence has changed
roadmap assumptions, dependency order, expected value, release readiness, or
the wisdom of continuing the campaign unchanged.

The planner remains advisory. A statement that continued execution is no longer
strategically justified does not amend the campaign or roadmap until the owning
authority accepts the route change.

Planner continuity may span both phases when retained context improves
long-horizon judgment, but current product state must be refreshed from durable
evidence at every planning break.

## Reviewer independence and lifetime

When independence supports the review claim, each specialist begins without
the proposal former's, builder's, or other reviewers' reasoning context. It may
receive the exact subject, allowed evidence, governing contracts, and the role
projection required to understand its question.

Initial specialist judgments should not be contaminated by other reviewers'
conclusions when independent perspectives are being composed. A later synthesis
may compare all results. Once initial independence is established, the same
isolated reviewer may evaluate revisions through a bounded remediation loop
while its accumulated understanding remains useful.

## Review synthesis and gate

Review synthesis is not voting, averaging confidence, or replacing findings
with one consensus narrative. A single supported invariant conflict is not
outweighed by several positive opinions.

Synthesis preserves:

- the exact subject revision and claims examined;
- confirmed support and material findings;
- conflicting interpretations;
- unresolved uncertainty and limitations;
- review provenance and independence claims;
- applicability to later proposal or implementation revisions;
- authority or evidence required next.

A decision-readiness gate may report consequences such as:

```text
ready_for_authority_decision
revision_warranted
material_uncertainty_unresolved
contract_conflict
```

Those states neither accept nor reject the proposal. The proposal former may
revise canonical proposal state within its authority. Acceptance, deferral,
rejection, scope reopening, campaign amendment, and implementation authority
remain with their owning authorities.

## Proposal review and implementation review

The same review substrate can serve both subjects while their expected breadth
differs.

Proposal review asks whether the candidate itself is coherent. It may need to
explore architecture, evidence, UX, doctrine, migration, uncertainty, and
strategic relationships before the candidate is ready for evaluation or an
authority decision.

Implementation review starts from an accepted bounded objective and observed
change. It normally tests more concrete claims: whether the implementation
preserves accepted placement, establishes the intended consequence, avoids
regression, and supplies proportionate proof. Specialized review remains useful
when the implementation crosses a material seam, such as rendered behavior,
security, persistence, or an unexpectedly changed architectural boundary.

Implementation review is therefore often narrower, but file count or change
size alone does not determine review breadth. A small bug fix can expose a stale
architectural premise, and a large mechanical change can preserve architecture
without requiring architectural diagnosis.

## Durability does not inflate authority

> **Persistence preserves provenance, not epistemic or decision authority.**

A durable judgment is evidence that a role made that judgment under recorded
conditions. It is not new evidence that the judgment is correct. A role must
not treat its own prior judgment as independent support merely because that
judgment has a stable identifier, appears in a decision trace, or was consumed
by another durable artifact.

Review findings, architectural diagnoses, planner recommendations, proposal
decisions, and authority transitions must remain distinguishable. Later reuse
requires an applicability judgment against the current subject and evidence.

## Initial proposal-workflow boundary

The first controlled proposal lifecycle should support:

1. immutable proposal-revision identity for review subjects;
2. revision-bound read-only specialist review artifacts;
3. an open specialist capability registry;
4. consequence-driven panel selection and relevant omission provenance;
5. role-scoped independent outputs;
6. finding resolution and review applicability after proposal revision;
7. non-authoritative review synthesis and decision-readiness consequences;
8. explicit authority-controlled proposal decisions;
9. architectural and doctrine-alignment reviewers as initial capabilities; and
10. a conditional UI/process reviewer as a representative dogfood case.

The first version does not require every specialist, a universal review panel,
numeric reviewer-selection thresholds, majority voting, automatic acceptance,
evidence-backed portfolio scoring, or the full role-decision-trace system.

## Relationship to persistent state and decision traces

Persistent state may need to preserve active review obligations, exact subject
identity, current applicability, unresolved authority needs, and reviewer
bindings when those facts are required for correct recovery. It should not
absorb the full review record or make review-panel infrastructure a prerequisite
for the initial state owner.

A future role-decision trace may preserve reviewer judgments and their lineage.
Durable review artifacts remain the authority for review outcomes required by
the proposal lifecycle. Neither artifact may cite the other's existence as
proof that the underlying judgment is correct.

## Open questions

- Does one proposal-review coordinator own both panel selection and synthesis,
  or do those contexts have sufficiently different independence and lifetime
  needs to justify separate capabilities?
- Which review consequences belong in the proposal packet, and which remain in
  separately owned review artifacts referenced by the packet?
- What constitutes a material proposal revision requiring refreshed review,
  partial applicability, or explicit supersession?
- Which specialist capabilities add genuinely distinct judgment rather than
  duplicating general adversarial review?

## Compact principles

> **Select reviewers for the material claims and seams of the actual change.**

> **Let specialists diagnose; let synthesis preserve consequences; let the
> owning authority decide.**

> **Durability preserves provenance, not authority.**
