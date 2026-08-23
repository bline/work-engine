# Proposal: Raw Idea Intake and Claim Assessment

## Identity and state

- Proposal ID: `work-engine.raw-idea-intake`
- Family ID: `work-engine.idea-to-proposal-system`
- State: formed; placement probable; not reviewed, decided, or authorized for
  implementation
- Decision owner: user or future explicitly authorized portfolio owner

The canonical lifecycle, placement, uncertainty, relationship, and authority
metadata is in [`packet.json`](packet.json). This narrative owns the proposal's
current meaning. Supporting artifacts describe its [placement](placement.md),
[relationships](relationships.md), and a possible
[implementation shape](implementation-plan.md).

## Candidate and consequence

Add a low-friction intake boundary that binds one exact raw-idea revision and
produces an attributed, claim-addressable assessment suitable for reconciliation
and proposal formation. A raw source may remain incomplete, contradictory,
duplicative, or speculative. Intake preserves what the author wrote while
identifying independently addressable candidate meaning, uncertainty, evidence,
relationships, and the authority needed for any disposition.

A fresh proposal former can consume the surviving candidate projection without
rereading the entire idea collection or reconstructing the assessment
conversation. The assessment can also truthfully conclude that no proposal is
ready or warranted.

## Proposed product change

Create an intake capability and durable record profile with these semantic
owners:

- the raw idea owns the author's exact source;
- the current `ideas/README.md` and `ideas/reconciliation-map.json` own the
  attributed manual reconciliation snapshot that predates intake records; they
  are prior evidence for migration, not a second canonical owner after an exact
  source revision has a claim-level intake assessment;
- the intake record owns one attributed interpretation of an immutable source
  revision, including material claim identities, evidence cutoffs, relationship
  nominations, adjudicated dispositions, uncertainty, and reopening conditions;
- the proposal former owns whether surviving candidate meaning should be
  challenged, split, merged, revised, or formed;
- proposal packets own formed proposal identity and lifecycle;
- stronger domain owners retain architecture, implementation, review, evidence,
  and authority facts referenced during assessment; and
- repository mutation owns any later move, archive, redirect, merge, or deletion
  authorized from an intake disposition.

The capability may investigate, compare, decompose, ask focused questions, and
revise its interpretation according to the actual idea. These are affordances,
not a mandatory questionnaire or a fixed assessment route.

## Required properties

### Raw capture remains cheap and faithful

Desired outcome: a person or agent can record incomplete speculative thought
without satisfying a proposal or claim schema, and the exact authored source
remains available after later interpretation.

Invalid outcome: intake requirements become an authoring form, or assessment
silently rewrites the raw source into cleaner meaning.

### Every assessment binds an exact subject

Desired outcome: a disposition identifies the stable idea, immutable source
revision, bounded source content, purpose, evidence cutoff, producer, and actual
authority under which it was formed.

Invalid outcome: edits or path movement silently change the subject beneath an
existing assessment or cause a later role to mistake judgments about different
source worlds for one conclusion.

### Assessment claims do not become facts by extraction

Desired outcome: source assertions, assessor interpretations, repository
observations, relationship nominations, adjudicated dispositions, and human
decisions remain distinguishable and attributable.

Invalid outcome: similarity proves duplication, a code match proves
implementation, persistence proves correctness, or an assessor acquires a
domain owner's authority by recording its inference.

### One source may have several truthful outcomes

Desired outcome: independently addressable claims can be represented by an
existing proposal, apparently implemented, still novel, unresolved, conflicting,
ready for formation, or unsuitable for proposal creation without forcing one
file-wide answer.

Invalid outcome: one promoted or implemented claim collapses the entire source
to done, or every intake attempt manufactures a proposal to signal completion.

### Handoff preserves meaning without transferring ownership

Desired outcome: the proposal former receives stable intake and source identity,
candidate boundaries, applicable claim revisions, adjudicated relationships,
placement hypotheses, evidence, uncertainty, authority, and reopening
conditions.

Invalid outcome: the handoff becomes the only durable copy, intake creates
proposal identity, or formation must reconstruct the material assessment from
raw notes and conversation context.

### Formed meaning is published before review

Desired outcome: when intake leads to proposal formation, the complete formed
proposal packet is handed to the proposal-formation owner, which publishes it
through the packet contract at an immutable Git revision before semantic review
begins. Review, remediation, and later decisions can bind exact proposal
revisions, making the proposal's initial meaning and subsequent changes
inspectable without granting intake or publication any review or acceptance
authority.

Invalid outcome: reviewers begin from mutable working-tree meaning, formation
and review-driven remediation collapse into one unattributed revision, or a
later reader cannot distinguish what was originally proposed from what changed
because of review.

### Cleanup remains a separate authorized consequence

Desired outcome: an intake record may recommend movement, archival, merging,
redirecting, or deletion while stable provenance and affected references remain
resolvable.

Invalid outcome: assessment, validation, or proposal formation automatically
mutates raw sources or breaks durable origins merely because a disposition was
recorded.

## Boundary and placement

The probable semantic owner is a dedicated Work Engine planning-layer intake
capability upstream of proposal formation. A separate top-level `idea-intake/`
surface is a plausible first adapter because it makes validated assessment
distinct from cheap raw capture, but a clearly partitioned hierarchy under
`ideas/` could preserve the same boundary. Physical layout remains provisional.

The capability may consume claim-lineage semantics or an equivalent local
profile, but this proposal does not accept the shared evidence-lineage placement
hypothesis. Role-owned operational state may preserve a pending assessment
obligation while the intake record retains assessment history and meaning.

## Scope

In scope:

- stable raw-idea and source-revision binding;
- migration from the existing manual reconciliation snapshot without creating
  a second owner for later claim-level dispositions;
- attributed claim identification and assessment revision;
- evidence cutoffs, relationship nomination, and authorized disposition;
- uncertainty, conflicts, reopening conditions, and partial outcomes;
- compact resumable assessment state and proposal-formation handoff;
- a handoff that requires the formation owner to publish a complete proposal
  packet before semantic review;
- stable provenance across authorized source movement; and
- truthful no-proposal and unresolved outcomes.

Out of scope:

- mandatory structure for raw idea capture;
- automatic semantic duplication or implementation decisions;
- claim-lineage acceptance or permanent placement;
- proposal review, evaluation, acceptance, prioritization, or implementation;
- automatic idea cleanup or repository mutation;
- one exhaustive taxonomy for every possible idea relationship; and
- a graphical inbox or continuously running assessor.

## Uncertainty and evidence needs

- Exercise duplicate, promoted, apparently implemented, split, genuinely novel,
  uncertain, and no-proposal source cases.
- Determine the smallest mechanically closed record surface and which assessment
  meaning must remain narrative.
- Demonstrate stable identity and proposal-origin resolution across an actual
  raw-source move.
- Compare a separate intake surface with an `ideas/` hierarchy using retrieval,
  validation, and context-loading evidence.
- Verify that a fresh proposal former can use one intake projection without the
  complete raw-idea directory or assessment conversation.
- Determine whether the bounded claim-lineage minimum can serve intake without
  turning experimental evidence semantics into premature production placement.
- Exercise migration from `ideas/reconciliation-map.json`: preserve its
  attributed file-level disposition as prior evidence, then prove a later
  claim-level intake revision is not overridden by that historical projection.
- Evaluate the existing `ideas/history/2026-08-22-pre-reconciliation/` meaning
  before selecting any hierarchy that would reuse `ideas/history/` for a
  different lifecycle boundary.

## Authority

Formation of this candidate does not assess or move an idea, approve its
meaning, accept a proposal, settle claim-lineage or physical placement, change
roadmap priority, or authorize implementation. Those consequences remain with
their named owners.

## Acceptance consequence

If later accepted and implemented, Work Engine can pull a cheap raw idea into a
durable, source-bound assessment; preserve claim-level novelty, overlap,
uncertainty, evidence, and authority; hand surviving candidate meaning to
proposal formation; and retain truthful unresolved or no-proposal outcomes
without rewriting the source or reconstructing the intake conversation.
