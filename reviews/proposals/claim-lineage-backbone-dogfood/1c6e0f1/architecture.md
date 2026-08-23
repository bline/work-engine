# Architecture and Experimental-Placement Review

## Role and boundary

Fresh, read-only architecture review of proposal
`work-engine.claim-lineage-backbone-dogfood` at commit
`1c6e0f19dfb35af1917defac56ac1d823458318d`, tree
`7ce27ed672fff9af2f787c5d61f717ee456ae58a`. This review addresses only
experimental placement, ownership, adjacency, canonical-history/query
separation, fixture placement, and reversibility. It does not decide roadmap
priority, select an implementation, mutate the subject, or grant proposal or
implementation authority.

## Readiness consequence

`ready_for_authority_decision`

The candidate is coherently placed as a bounded experiment. It makes the
dogfood representation canonical only inside the experiment, preserves every
existing domain owner, keeps the query projection derived and replaceable, and
explicitly prevents successful persistence or querying from selecting a shared
owner, schema, store, or production boundary. The residual constraints below
belong in any authorized execution plan and evidence report; they do not
warrant revising this proposal.

## Findings

### 1. No material finding: ownership is provisional without being ownerless

Severity: none.

The proposal assigns enough local ownership to make the experiment auditable:
dogfood records own the representation under test, domain workflows own the
original and refresh judgments, existing Git/packet/review/receipt artifacts
retain their canonical facts, the projection owns no meaning, and downstream
decision owners retain reliance and reopening authority. That is a coherent
experimental boundary rather than a premature shared-owner selection
(`proposals/evidence-lineage/claim-lineage-backbone-dogfood/proposal.md:116-138`;
`proposals/evidence-lineage/claim-lineage-backbone-dogfood/placement.md:22-34`;
`proposals/evidence-lineage/claim-lineage-backbone-dogfood/relationships.md:25-43`). It is
also consistent with the authority decision that permanent architecture remains
unsettled and implementation unauthorized
(`proposals/evidence-lineage/claim-centered-evidence-lineage/decision.json:9-31`).

Consequence: an execution may reference existing owners but must not extend
their schemas, reinterpret their facts, or use physical custody of dogfood
records as semantic authority. A need to do so triggers the packet's reopening
conditions rather than an implementation workaround.

### 2. No material finding: canonical history and query placement remain separate

Severity: none.

Canonical dogfood records are independently readable and non-destructive; the
projection must rebuild from empty state and disclose source identities and its
completeness boundary
(`proposals/evidence-lineage/claim-lineage-backbone-dogfood/proposal.md:118-138`,
`:140-152`). The
placement permits several local projection mechanisms without choosing one and
denies the projection claim meaning, causal judgment, authority, or undeclared
completeness
(`proposals/evidence-lineage/claim-lineage-backbone-dogfood/placement.md:10-20`,
`:29-32`). This preserves the
parent candidate's canonical-history/query split
(`proposals/evidence-lineage/claim-centered-evidence-lineage/placement.md:10-21`,
`:74-79`).

Consequence: projection success is evidence about bounded query needs only.
Failure, partial coverage, or a need for manual interpretation must remain
visible; none is evidence that the projection should become the canonical
store.

### 3. Execution constraint: adjacency must not become packet ownership by filename

Severity: advisory.

“Adjacent to the evidence-lineage proposal family” is a reasonable reversible
default because it keeps fixtures near the hypothesis while leaving the
planner freedom over the physical layout
(`proposals/evidence-lineage/claim-lineage-backbone-dogfood/placement.md:3-20`). However, the
current proposal-packet implementation recursively discovers every file named
`packet.json` under its repository root and validates it as a proposal,
including proposal-ID uniqueness, decision-state correspondence, and
relationship targets (`skills/proposal-packets/scripts/proposal_packets.py:265-289`).

Consequence: an authorized plan should use an unmistakably dogfood-local
namespace and filenames, and should demonstrate in a disposable checkout that
adding and removing the experiment does not change proposal-packet or
review-artifact meaning or validation. After execution, the evidence report and
records it cites may remain as historical evidence while executable machinery
stays separable. This follows the proposal's existing owner-preservation
requirement; it does not require the formation packet to choose a permanent
directory or schema.

### 4. Residual limitation: the experiment can inform placement, not certify it

Severity: advisory.

The family explicitly says no proposal-research workflow has yet been formed
and that this backbone exercise cannot satisfy the larger lifecycle and
recovery boundary (`proposals/evidence-lineage/family.md:26-32`, `:59-64`). The parent placement also
says credible consumers do not yet prove a common schema or runtime boundary
and calls for separately formed consumers before placement is settled
(`proposals/evidence-lineage/claim-centered-evidence-lineage/placement.md:18-30`,
`:95-104`). The
dogfood proposal truthfully narrows its claim: it binds one real research claim
and one real review finding before outcomes, records shared and domain-specific
pressure, and makes the next placement decision better informed without
settling it
(`proposals/evidence-lineage/claim-lineage-backbone-dogfood/proposal.md:88-114`,
`:221-228`;
`proposals/evidence-lineage/claim-lineage-backbone-dogfood/placement.md:63-76`).

Consequence: even a clean four-proof result is preliminary placement evidence.
It cannot establish a production owner or discharge the parent review's
adversarial lifecycle, recovery, delivery, authority-loss, or compaction
boundary (`reviews/proposals/claim-centered-evidence-lineage/cd50e79/synthesis.md:26-37`).
This is a disclosed limit, not a conflicting premise.

### 5. No stale premise requiring revision was found

Severity: none.

The parent authority disposition is deliberately bound to an earlier immutable
revision and remains `defer_for_dogfooding`; the dogfood packet does not portray
the later semantic amendment or review closure as implementation authority
(`proposals/evidence-lineage/claim-lineage-backbone-dogfood/proposal.md:16-18`,
`:178-198`, `:212-219`). Existing
completion receipts are also used only as immutable source-event evidence. The
current validator re-establishes commit, checkpoint tree, parent, resulting
tree, message, publication target, branch attachment, and HEAD facts; it does
not confer claim semantics
(`skills/slice-completion-commit/scripts/completion_commit.py:132-197`;
`proposals/evidence-lineage/claim-lineage-backbone-dogfood/relationships.md:34-43`).

Consequence: fixture selection may cite those immutable facts, but any
`may_affect`, refresh, or `changed_because_of` judgment must remain a separately
identified, properly authorized dogfood fact.

## Limitations and evidence provenance

- This review did not select fixtures or prove that qualifying fixtures exist;
  inability to bind truthful pre-change examples is already an explicit
  reopening condition.
- It did not validate runtime behavior, projection determinism, schema shape,
  or removal in an implemented experiment; those are future execution evidence.
- Repository retrieval used Verify-tier evidence from Codebase Memory project
  `home-bline-code-work-engine`, indexed at HEAD
  `1c6e0f19dfb35af1917defac56ac1d823458318d`. The checked paths returned
  `no_recorded_issue` with matching metadata and no recorded parse-partial or
  skipped files. That is best-effort coverage, not proof of completeness.
  Proposal, decision, design, review, and procedure semantics were read
  directly. Persistence of this review is not new evidence.

## Governing references

- `DESIGN.md:20-46`, `:103-155`, `:497-527`, `:537-570` — contracts,
  variant structure, user authority, receipts, and feature ownership.
- `PHILOSOPHY.md:431-471`, `:531-613` — describe provisional machinery and
  preserve meaningful, non-redundant action space without turning it into a
  permanent route.
- `reviews/proposals/bootstrap-review-procedure.md:1-20`, `:78-102` — fresh
  specialist boundary and non-authoritative closure.
- `reviews/proposals/claim-lineage-backbone-dogfood/1c6e0f1/subject.md` — exact
  immutable subject and review boundary.

This artifact is diagnostic and advisory. It grants no proposal, placement,
schema, fixture-selection, or implementation authority.
