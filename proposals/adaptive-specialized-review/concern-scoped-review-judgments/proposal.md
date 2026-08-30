# Proposal: Concern-Scoped Review Judgments and Provenance

## Identity and state

- Proposal ID: `work-engine.concern-scoped-review-judgments`
- Family ID: `work-engine.adaptive-specialized-review`
- State: formed; not evaluated, accepted, prioritized, or authorized for
  implementation
- Decision owner: user or future explicitly authorized portfolio owner

The canonical lifecycle, placement, uncertainty, relationship, and authority
metadata is in [`packet.json`](packet.json). This narrative owns the current
proposal meaning. The supporting [semantic model](semantic-model.md),
[repository integration](repository-integration.md), [economics and
validation](economics-and-validation.md), [acceptance](acceptance.md), and
[formation evidence](formation-evidence.md) elaborate that meaning without
becoming competing owners. The [ownership judgments](ownership-judgments.json)
record revisable formation classifications and do not change a contract.

## Candidate and consequence

Define provider-neutral immutable `ReviewResult` and concern-scoped
`ReviewJudgment` records so a fresh consumer can determine what an attributed
reviewer evaluated, for which exact concern and subject revision, using which
evidence, with what disposition and limitations, and why that bounded judgment
was later retained, superseded, invalidated, or left unresolved.

The candidate separates durable review meaning from unfinished operational
continuation:

```text
ReviewEpisode
  writer generation, session continuity, remediation, replacement, uncertainty

ReviewResult
  one published perspective result for one exact candidate

ReviewJudgment
  one concern-scoped positive, negative, incomplete, or uncertain judgment

Finding
  one material adverse observation with its own remediation lineage
```

The proposal preserves the approved meaning of
[`work-engine.revision-bound-review-artifacts`](../revision-bound-review-artifacts/proposal.md)
while responding to its explicit implementation-review reopening conditions.
It does not reopen or supersede that decision automatically.

## Problem

Current retained review state can durably bind an episode to an exact reviewed
subject, findings, evidence references, remediation status, reviewer
generation, and runtime-session reference. It cannot durably express the
smaller positive unit:

```text
For concern X, reviewer generation G evaluated subject Y at revision C1,
relied on evidence Z, observed no material issue within limitations L,
and later retained or reconsidered that judgment for exact delta C1 -> C2.
```

This absence is distinct from the S5 lifecycle defect in which a published
`reported` result was treated as unable to continue. Correcting episode
continuation prevents an unnecessary reviewer replacement. Concern-scoped
judgments address a later problem: preserving and selectively reconsidering
positive review meaning without repeating full-subject review.

## Proposed product structure

### Versioned concern identity

The provider-neutral review contract owns stable, versioned concern identities
and their definitions. A perspective composes exact concern revisions; a
provider profile realizes the perspective but cannot redefine its concerns.
Changing concern meaning is a review-contract revision, not a display-label or
adapter change.

### Immutable result and judgment records

A `ReviewResult` binds one perspective execution to an exact candidate,
episode generation, reviewer configuration, evidence cutoff, declared scope,
overall bounded outcome, limitations, referenced judgments, and produced
findings. Result completion means `complete_for_declared_scope`; it does not
mean implementation acceptance.

A `ReviewJudgment` binds an exact concern revision to one or more explicitly
mapped exact subjects, the evidence the reviewer asserted it relied upon, a
bounded disposition, limitations, and immutable predecessor or successor
relationships. Positive judgments use an outcome such as
`no_material_issue_observed`, not an unbounded correctness or approval claim.

Compact grouped output is permitted only when mapping semantics are explicit.
The host may materialize atomic judgment identities from a declared Cartesian
mapping or an explicit mapping list. It cannot infer concern-to-subject meaning
from parallel arrays, tool activity, prose proximity, or formatting.

### Distinct activity and judgment layers

Mechanically observed activity includes evidence presented to a runtime, files
or symbols retrieved, graph neighborhoods queried, commands and tests executed,
and browser projections requested. It does not establish model attention,
inspection, reliance, understanding, or semantic coverage.

Reviewer assertions separately record what was materially inspected, relied
upon, and judged. An assertion may cite observed activity, but neither layer
silently manufactures the other.

### Carry-forward as successor judgment

A repository or evidence delta first nominates predecessor judgments as
`possibly_affected`. Mechanical evidence may establish that exact bytes,
symbols, projections, scenes, pixels, geometry, accessibility surfaces, or
other bounded artifacts are unchanged under a named comparator. It cannot
decide that a semantic judgment remains applicable.

An attributed retained reviewer or separately authorized concern-specific
policy may publish an immutable successor judgment that carries a predecessor
forward to an exact successor candidate. The successor cites the delta,
mechanical evidence, authority, rationale, and limitations. The predecessor is
never mutated with a current-validity flag.

Current coverage is a cutoff-bound projection over exact judgment lineage, not
a timeless property. Candidate change alone may produce `possibly_affected` or
`awaiting_relevance_judgment`; it cannot silently produce semantic invalidation.

### Distributed ownership and composed projection

The semantic interface is a composition, not a universal canonical database:

- the review contract owns concern and judgment meaning;
- the review-result capability owns immutable result and judgment records;
- the review-episode service owns unfinished continuation state;
- finding state owns adverse-observation remediation lineage;
- Claim Evidence may own published production finding revisions and reliance;
- runtime and raw-evidence capabilities own observed activity and retained raw
  bytes or authorized omissions;
- Code Change Profile owns recomputable exact-subject change observations;
- UI evidence capabilities own browser and rendered-state evidence;
- Codebase Memory owns a replaceable navigation projection; and
- the supervisor and human retain their existing selection, acceptance, and
  contract-change authority.

## Protected boundaries

### Positive review meaning remains bounded

Reason: a no-finding result can otherwise be misread as complete inspection,
universal correctness, approval, or acceptance.

Required property: every judgment binds exact concern, subject, candidate,
evidence cutoff, relied-on evidence, exclusions, limitations, producer, and
disposition. Absence of a finding does not imply a judgment for an unlisted
concern or subject.

### Mechanical evidence cannot decide semantic applicability

Reason: unchanged output under one comparator may be irrelevant to a broader
concern, while a mechanically changed entity may not affect that concern.

Required property: mechanical change evidence nominates relevance; an
attributed semantic judgment or explicitly authorized concern-specific policy
owns retention, supersession, invalidation, or unresolved disposition.

### Projection gaps do not create work authority

Reason: a rich provenance graph can otherwise cause the system to invoke
reviewers merely to fill empty regions.

Required property: missing projection or provenance coverage may expose
uncertainty or nominate possible work, but every review invocation originates
from an independently authorized production, audit, research, incident, or
other domain objective.

### Review provenance does not imply cognition

Reason: tool use and context delivery are observable while attention,
understanding, and reliance are not mechanically recoverable.

Required property: observed activity and reviewer-authored assertions remain
distinct in schema, provenance, UI presentation, and downstream reasoning.

### Review completion does not accept implementation

Reason: reviewers provide bounded decision support while supervisors and
humans own downstream transitions.

Required property: a result can be complete only for its declared review scope.
Review selection, synthesis, implementation acceptance, contract change, and
human authorization remain separate records and owners.

## Placement

The probable placement is a shared App Server review-result capability adjacent
to, but semantically distinct from, the provider-neutral review-episode service
planned by migration slice S9. One service package may realize both capabilities
if their record types, mutation authority, retention, and recovery boundaries
remain explicit.

Codebase Memory is the probable default current-world projection and traversal
provider after the separately proposed owner-isolated, generation-bound
projection synchronization extension. It is not the result ledger, historical
identity adjudicator, or coverage authority.

## Economic consequence

The semantic model must reduce repeated inference rather than justify broader
contexts. Rich durable storage, deterministic indexing, change profiling,
projection traversal, and activity capture are cheap relative to model calls.
Reviewers receive narrow perspective- and concern-specific projections and
expand references only when judgment requires it.

Review provenance is ordinarily produced as a byproduct of independently
authorized review. A human may authorize an audit whose objective is to
establish missing knowledge, but a projection gap cannot authorize that audit.
The expected direction is less reconstruction, replacement, repeated evidence
retrieval, and candidate churn without degraded adjudicated finding quality.

## Authority and non-effects

This formed proposal does not change the accepted review-artifact meaning,
define a production schema, authorize an App Server implementation, modify the
skills-migration sequence, select reviewers, declare any existing review
coverage valid, publish claims, authorize Codebase Memory mutation, change
roadmap priority, or accept an implementation.

## Intended acceptance consequence

If later accepted and implemented, a fresh authorized consumer can traverse
from an exact review result to the concern-scoped judgments, subjects, evidence,
findings, limitations, and successors that support it, and from an exact code,
documentation, test, claim, or UI-evidence revision to the bounded review
judgments that currently apply at a named cutoff. The consumer can distinguish
observed activity from asserted review meaning and possible impact from
attributed semantic disposition without trusting transcripts or treating the
projection as authority.
