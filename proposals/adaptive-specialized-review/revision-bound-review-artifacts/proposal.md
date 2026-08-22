# Proposal: Revision-Bound Review Artifacts

## Identity and state

- Proposal ID: `work-engine.revision-bound-review-artifacts`
- Family ID: `work-engine.adaptive-specialized-review`
- State: placement uncertain; this revision has not been reviewed, evaluated,
  accepted, or authorized
- Decision owner: user or future explicitly authorized portfolio owner

The canonical lifecycle, placement, uncertainty, relationship, and authority
metadata is in [`packet.json`](packet.json). This narrative owns the proposal's
current meaning. Supporting artifacts describe its [placement](placement.md)
and [relationships](relationships.md).

## Candidate and consequence

Define a durable review-artifact profile that binds review episodes, specialist
findings, conflicts, outcomes, and synthesis to an exact immutable subject and
to stable evidence-backed statements. A later consumer can determine what was
reviewed, which findings and assumptions remain applicable, what context each
reviewer actually received, whether a review was omitted or failed rather than
passed, and how synthesis preserved the source findings without becoming an
authority decision.

The profile would consume the separately formed
[`work-engine.claim-centered-evidence-lineage`](../../evidence-lineage/claim-centered-evidence-lineage/proposal.md)
candidate for shared subject, statement, baseline, provenance, sensitivity,
authority-scope, and lineage semantics. This candidate owns only the additional
semantics that make those statements truthful review artifacts.

## Proposed product change

Define the smallest human-readable, mechanically checkable review profile
needed by real proposal-review consumers. Candidate semantics include:

- an exact shared evidence-lineage subject and content-set identity for the
  proposal, implementation delta, or other bounded review subject;
- a review episode identity stating whether it is a fresh initial perspective,
  a continuation with retained context, or another explicitly described
  context relationship;
- stable identity for each finding, the reviewed claim or consequence,
  observed or inferred status, severity, support references, material
  assumptions, limitations, and conflicts;
- truthful outcomes that distinguish not requested or inapplicable, deliberately
  omitted, deterministic-only evidence, reviewer unavailable or failed,
  completed with no findings, and completed with findings;
- synthesis references to every source finding and conflict it consumes, with
  synthesis-introduced inferences distinguished from specialist findings;
- review-specific applicability, partial-applicability, refresh, composition,
  supersession, correction, and invalidation consequences over shared lineage;
  and
- a non-authoritative decision-readiness projection whose producer, scope,
  limitations, and next decision owner remain explicit.

The proposal does not select a schema, storage engine, runtime service, review
procedure, provider, or fixed panel. Those choices require consumer evidence
and later authority.

## Invariants

### Review subjects and findings remain addressable

Reason: a commit identifies a repository world but not necessarily the bounded
proposal family, packet revision, path set, or implementation delta reviewed;
artifact-level identity cannot preserve finding-specific applicability.

Required property: each artifact uses the shared bounded-subject contract and
gives every material finding stable identity, reviewed consequence, support,
assumptions, limitations, and lineage.

### Independence claims match review episodes

Reason: a reviewer inheriting another role's conclusions or continuing a prior
remediation pass cannot truthfully claim the same fresh perspective as an
isolated initial review.

Required property: every episode records actual context isolation and
inheritance. Freshness applies only to the episode where it was established;
useful retained context is labeled as continuation rather than another fresh
review.

### Absence is not success

Reason: an irrelevant review, an intentional omission, deterministic evidence,
provider failure, and a completed review with no findings have different
downstream consequences.

Required property: the artifact preserves a truthful closed outcome and reason
without reporting omitted, unavailable, blocked, or failed work as passed.

### Synthesis preserves reference integrity

Reason: prose synthesis can otherwise paraphrase away a supported conflict or
launder a coordinator inference into an independent finding.

Required property: synthesis references source finding and conflict identities,
preserves unresolved limitations, and labels any newly introduced inference.
Mechanical checks may verify references; they cannot judge intellectual
faithfulness.

### Review judgment and downstream authority remain separate

Reason: reviewers and coordinators produce decision support; they do not own
the decision that consumes it.

Required property: specialists own their scoped findings, the coordinator owns
its synthesis judgment, an authorized domain maintenance owner judges semantic
freshness, and the downstream decision owner judges applicability and readiness
for its exact transition. Proposal revision, acceptance, roadmap mutation, and
implementation authorization remain with their named owners.

### Persistence does not create evidence or authority

Reason: durable judgments can otherwise cite their own continued existence or
reuse as independent support.

Required property: no finding, synthesis, applicability judgment, or readiness
projection treats persistence, repetition, or downstream reference as evidence
that its own semantic conclusion is correct.

## Boundary and placement

The probable initial shape is a Git-backed review profile adjacent to proposal
packets, referenced rather than absorbed by `packet.json`, over the candidate
shared evidence-lineage contract. That is a plausible adapter location, not a
decision that proposal review owns all review semantics permanently.

Placement remains uncertain because implementation review may share the review
episode, outcome, finding, and synthesis profile, while its remediation and
acceptance lifecycles may differ materially. Dogfooding has also not established
whether selection, specialist findings, synthesis, and resolution need separate
owners or information lifetimes.

The proposal-packet manifest continues to own proposal identity, lifecycle,
uncertainty, relationships, and decision authority. The shared evidence-lineage
candidate owns only its proposed cross-domain statement semantics. This
candidate owns only the proposed review-specific profile if accepted.

## Relationships

This candidate causally depends on claim-centered evidence lineage and causally
enables adaptive review-panel coordination. Neither dependency imposes a
mandatory acceptance or delivery order. The coordinator may run ephemerally,
but its proposed durable consequence requires truthful review artifacts.

## Uncertainty and evidence needs

- Represent the existing bootstrap review in the candidate profile without
  rewriting its historical subject or pretending the provisional files already
  establish the schema.
- Review a changed proposal revision to test finding-level applicability,
  retained reviewer context, correction, supersession, and synthesis references.
- Exercise omitted, unavailable, failed, no-finding, and finding-bearing review
  outcomes without converting absence into success.
- Determine which review fields must be mechanically closed rather than
  narrative.
- Exercise implementation review later to distinguish genuinely shared review
  semantics from proposal-local adapter assumptions.
- Identify any remediation lifecycle that genuinely needs runtime coordination
  rather than Git history and on-demand reconciliation.

## Authority

This revised candidate does not establish the shared or review artifact
contract, perform review, declare semantic freshness, decide applicability or
readiness, revise or accept a proposal, alter roadmap priority, or authorize
implementation. Those transitions remain with their named owners. The
currently unresolved research-maintenance and organizational-planning roles are
acceptance or implementation conditions, not authority inferred by this packet.

## Acceptance consequence

If later accepted and implemented, a fresh consumer can determine the exact
subject, review episode, findings, outcome, conflicts, synthesis sources, and
applicability of review evidence without trusting a transcript, mistaking
absence for success, or confusing persistent decision support with authority.

