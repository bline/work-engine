# Proposal: Interactive Proposal Formation

## Identity and state

- Proposal ID: `work-engine.proposal-formation`
- Family ID: `work-engine.idea-to-proposal-system`
- State: formed; placement probable; not evaluated or accepted
- Decision owner: user or future explicitly authorized portfolio owner

The canonical lifecycle and relationship metadata is in [`packet.json`](packet.json).
This narrative owns the proposal's current semantic meaning. Supporting packet
artifacts record its [placement](placement.md), [relationships](relationships.md),
and proposed [implementation plan](implementation-plan.md).

## Problem

Raw engineering ideas are intentionally cheap and speculative. Before they can
be evaluated or scheduled, someone must determine what candidate changes they
contain, what consequence each change seeks, where it belongs, how candidates
relate, and which uncertainties matter. Today that interpretation is ad hoc and
its durable consequences are easily lost.

A fixed form would make the state visible but would not solve the semantic
problem. Different ideas need different investigation, decomposition, and human
judgment.

## Intended consequence

A human and capable model can turn an idea into zero or more bounded proposal
packets without following a universal questionnaire. The model concentrates on
material uncertainty, persists durable consequences as they become valuable,
and asks the human only for decisions that require human authority, preference,
or missing product context.

## Proposed product change

Add a `proposal-former` planning capability to Work Engine. It operates as an
interactive workbench over proposal-family and packet state.

The capability can:

- reflect the current interpretation of an idea;
- discover, split, merge, or retire candidate proposals;
- compare plausible semantic owners;
- inspect existing machinery and proposals for duplication or overlap;
- expose assumptions, alternatives, dependencies, and invalidation conditions;
- gather repository evidence when it has credible decision value;
- persist durable proposal consequences incrementally;
- reopen conclusions made stale by new evidence; and
- produce role-scoped handoffs without duplicating canonical state.

These are affordances, not mandatory phases. The model chooses which operation
has decision value in the current state.

The initial implementation should be a repository-local skill using the packet
contract, repository evidence capabilities, and ordinary Git-backed files. A
dedicated UI, persistent agent runtime, portfolio database, and automatic
scheduler are not required to learn whether the semantic workflow works.

## Invariants

### Formation cannot grant downstream authority

Reason: a coherent proposal is still a candidate, not permission to implement
or reprioritize work.

Required property: formation may recommend a transition but records acceptance,
rejection, roadmap selection, or implementation authority only when the owning
authority actually provides it.

### Source classes remain distinct

Reason: an idea, repository observation, model inference, human decision, and
accepted product contract carry different authority.

Required property: the capability does not flatten them into interchangeable
facts.

### Durable state precedes handoff

Reason: a summary-only handoff can lose placement, lineage, uncertainty, or
human decisions after context replacement.

Required property: resume-critical proposal state is persisted before control
passes; the handoff is a projection, not the canonical copy.

### Semantic completion is not form completion

Reason: required fields can be filled with shallow or invented content while a
proposal remains ambiguous or incorrectly placed.

Required property: completion means another capable evaluator can understand
the candidate, placement, boundaries, relationships, and known uncertainty
without reconstructing what the proposal means.

### Human attention is reserved for material choices

Reason: forcing the human to answer questions already resolvable from available
context turns the capability into a form-filling burden.

Required property: the model investigates authorized evidence and makes
reversible formation judgments itself, escalating only genuine authority,
preference, or product decisions.

## Scope

In scope:

- interactive semantic formation and decomposition;
- repository-grounded placement and duplication checks;
- incremental packet updates and resume;
- proposal-family and relationship maintenance;
- explicit uncertainty and route revision;
- compact completion and handoff projections.

Out of scope:

- mandatory interview phases or question lists;
- evidence scoring and portfolio ranking;
- automatic proposal acceptance or roadmap mutation;
- campaign supervision or implementation planning;
- a graphical workbench;
- long-running autonomous planning cadence.

## Alternatives

### Static proposal template

Useful as storage structure but insufficient as the formation capability. It
cannot decide decomposition, investigate placement, challenge assumptions, or
identify which uncertainty matters.

### Make the strategic planner perform formation

Not preferred. Strategic planning reconciles portfolio direction and priority
from already meaningful durable candidates. Combining it with raw-idea
interpretation would mix two context lifetimes and authority surfaces. The
planner should consume formed proposals and may request reopening when evidence
invalidates them.

### Make the campaign supervisor perform formation

Rejected. Campaign supervision begins with a bounded objective and owns
execution continuity. Requiring it to discover what the objective means would
reintroduce planning ambiguity inside execution.

## Uncertainty and invalidation

- If dogfooding shows formation and strategic reconciliation cannot be
  separated without repeated reconstruction, revisit the role boundary.
- If a skill cannot preserve useful interactive continuity, add durable session
  support only after identifying the missing state and its owner.
- If model-generated decomposition repeatedly creates unstable proposal
  identities, strengthen lineage and human confirmation at the consequential
  transition rather than adding a universal decomposition rubric.

## Acceptance consequence

The proposal succeeds when the capability can take materially different raw
ideas—including one that yields no viable proposal and one that splits—and
produce truthful durable packet state that a fresh evaluator can use without a
fixed questionnaire or reconstruction of the formation conversation.
