# Proposal: Revision-Bound Review Artifacts

## Identity and state

- Proposal ID: `work-engine.revision-bound-review-artifacts`
- Family ID: `work-engine.adaptive-specialized-review`
- State: placement uncertain; revised after bootstrap review continuation and
  not closure-reviewed, evaluated, accepted, or authorized
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

The profile requires the semantic consequences proposed by the separately
formed
[`work-engine.claim-centered-evidence-lineage`](../../evidence-lineage/claim-centered-evidence-lineage/proposal.md)
candidate: subject, statement, baseline, provenance, sensitivity,
authority-scope, and lineage semantics. It consumes that candidate if later
accepted as the shared owner; another valid owner must supply equivalent
consequences if dogfooding rejects the shared placement hypothesis. This
candidate owns only the additional semantics that make those statements
truthful review artifacts.

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
- complete attributed disposition of every material finding and conflict
  produced by selected review episodes in the authoritative review artifact;
  profile-owned dispositions may include retained, deferred, inapplicable, or
  omitted, synthesis references every source it retains, and introduced
  inferences remain distinct from specialist findings;
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

Required property: the authoritative review artifact accounts for every
material finding and conflict produced by selected episodes with an attributed
disposition and reason. Each domain profile owns the allowed disposition
vocabulary; examples include retained, deferred, inapplicable, and omitted,
but they are not a universal exhaustive list. Synthesis references every source
it retains, preserves unresolved limitations, and labels any newly introduced
inference. The contract constrains the published consequence, not the
coordinator's internal ordering. Mechanical checks may verify recorded output
coverage and references; they cannot decide whether an item is material,
whether a disposition is wise, or whether synthesis is intellectually
faithful.

### Review judgment and downstream authority remain separate

Reason: reviewers and coordinators produce decision support; they do not own
the decision that consumes it.

Required property: specialists own their scoped findings and the coordinator
owns its synthesis judgment. A review-domain profile names a maintenance owner
only when it maintains canonical live review claims and may name an authorized
role to produce an attributed advisory applicability assessment. The downstream
decision owner decides whether to rely on that assessment, whether evidence is
ready for its exact transition, and whether residual uncertainty is acceptable.
Proposal revision, acceptance, roadmap mutation, and implementation
authorization remain with their named owners.

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

This candidate causally requires the semantic consequences described by
claim-centered evidence lineage and consumes that candidate only if it is
selected as their shared owner. It causally enables adaptive review-panel
coordination. Neither dependency imposes a mandatory acceptance or delivery
order. The coordinator may run ephemerally, but its proposed durable
consequence requires truthful review artifacts.

## Uncertainty and evidence needs

- Represent the existing bootstrap review in the candidate profile without
  rewriting its historical subject or pretending the provisional files already
  establish the schema.
- Review a changed proposal revision to test finding-level applicability,
  retained reviewer context, correction, supersession, and synthesis references.
- Exercise a selected episode that produces an adverse finding and verify that
  synthesis cannot make it disappear by excluding it from a consumed subset.
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
contract, perform review, declare semantic freshness, produce or rely on an
applicability assessment, decide readiness, revise or accept a proposal, alter
roadmap priority, or authorize implementation. Those transitions remain with
their named owners. Future research-maintenance or organizational-planning
roles must be authorized by the domain contract that owns their own future
transition; they are not acceptance or implementation prerequisites for this
review-specific proposal.

## Acceptance consequence

If later accepted and implemented, a fresh consumer can determine the exact
subject, review episode, findings, outcome, conflicts, synthesis sources, and
applicability of review evidence without trusting a transcript, mistaking
absence for success, or confusing persistent decision support with authority.
