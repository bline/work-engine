# Proposal: Role-Owned Durable Operational State

## Identity and current state

- Proposal ID: `work-engine.role-owned-durable-operational-state`
- Family ID: `work-engine.agent-state`
- State: placement uncertain; not evaluated or accepted
- Decision owner: user or future explicitly authorized architecture owner
- Evidence cutoff: repository `16f9914025930ec850f452abcf991edf29254a12`, plus the observed proposal-decision campaign through its accepted private checkpoint

The canonical lifecycle and placement metadata is in [`packet.json`](packet.json).
This narrative owns the proposal's current semantic meaning. The source ideas
remain speculative origin material rather than current proposal authority.

## Problem

Long-running Work Engine roles still keep resume-critical operational truth in
model or provider context. Current durable mechanisms preserve important but
different truths:

- proposal packets preserve candidate and decision context;
- receipts and checkpoints preserve terminal slice evidence and accepted trees;
- the scheduler preserves future obligations;
- provider session IDs preserve a route back to retained reviewer context; and
- `durable-state` atomically stores opaque values under stable keys.

None of those mechanisms by itself preserves the complete live operational
position of every role. A context replacement can therefore force expensive
re-synthesis, lose an unfinished judgment, or resurrect an event whose
consequence was already handled.

This failure was observed during the first proposal-decision transition. The
supervisor durably recorded that the user approved three dispositions, but the
live record did not contain the dispositions themselves. Continuation succeeded
only because the conversation compaction happened to preserve them. The same
mechanism also rejected an `implementation` phase because its current vocabulary
contains only `planning` and `review`.

## Repository diagnosis

The current implementation is internally consistent with its declared baseline,
but incomplete for general recovery:

- `docs/agent-environments.yaml` explicitly says the live runtime overlay is
  absent from the verified supervisor/builder/reviewer environment model.
- Slice live state records identity, actor binding, one pending-obligation
  summary, reference-only authority links, handled transition IDs, waiting, and
  retirement, but only for planning and review.
- Inter-slice campaign resume reconstructs accepted terminal state from receipts,
  continuation context, and checkpoints; it explicitly does not recover
  unfinished mid-slice work.
- The builder contract retains one runtime identity across planning,
  implementation, gate, and remediation, but does not publish a role-owned live
  semantic projection.
- Reviewer continuity retains a provider session through remediation. If the
  provider session is lost, the contract expects reconstruction from durable
  subject, findings, and gate evidence, but current live state preserves the
  subject obligation and session binding rather than the findings themselves.
- The strategic planner explicitly reconstructs from its last handoff and named
  sources until persistent agent-state infrastructure exists.
- Proposal formation correctly writes resume-critical proposal consequences to
  the packet before handoff. That solves proposal-domain continuity, not arbitrary
  unfinished role execution.
- Role scheduling owns scheduled obligations and delivery state, not the active
  role's current reasoning consequence or recovery position.

Direct source search at the evidence cutoff found the shared `durable-state`
adapter consumed by slice-supervisor live-state code, but no equivalent durable
role projection for builder, reviewer, or strategic planner. Generated Python
cache directories are excluded from the structural index and have no bearing on
this source-level conclusion.

## Intended consequence

Every long-running role whose correctness depends on accumulated context has a
small, externally durable, role-owned semantic projection sufficient for a
replacement context to determine:

- what is operationally true now;
- which authoritative boundary and accepted objective govern the work;
- which decisions and external events have already had consequences;
- what remains pending, unresolved, stale, or blocked;
- which provider or child runtime may be resumed when still available;
- which stronger domain artifacts establish referenced facts; and
- which expensive-to-reconstruct judgments must be preserved as attributed
  consequences rather than recomputed from transcripts.

A successful recovery does not require the full transcript, hidden reasoning,
or accidental survival of a conversation summary.

## State taxonomy

Workflow semantic state and agent live state are related but not identical.

```text
workflow semantic state
  what is true about the work independent of one runtime
  phase, accepted boundary, pending authority, route, findings,
  validation, disposition, and terminal consequence

agent live state
  what is true about the logical role's current execution binding
  logical agent identity, provider session, active obligation,
  parent/child binding, continuity mode, and recovery position
```

The workflow remains the semantic owner. Agent live state references the exact
workflow-state revision it is executing and adds only runtime and recovery
facts. A provider session is therefore replaceable without changing workflow
truth, while a workflow transition cannot be inferred merely from a session
appearing active or terminal.

## Current snapshot and consequence history

Recovery and audit require two different durable projections.

### Current snapshot

Each role maintains one integrity-checked current snapshot containing the
smallest state needed to resume efficiently. It identifies the active workflow
revision, logical agent and runtime binding, pending obligation, unresolved
state, handled-consequence frontier, and references to stronger artifacts.

The snapshot answers:

> What is operationally true now, and what must happen next?

### Semantic transition history

Authority decisions, route revisions, handled external events, finding
dispositions, runtime replacements, waits, and terminal transitions require a
predecessor-linked consequence record. Each record should identify:

- the owning workflow and logical role;
- predecessor and resulting state revisions;
- a stable transition or event identity;
- the attributed actor and actual authority evidence where applicable;
- the compact semantic consequence;
- stronger artifact references and their integrity identity;
- observation time and freshness boundary; and
- whether the transition remains active, was superseded, or became stale.

The transition history answers:

> Which authorized or observed consequence produced the current state, and has
> this event already been handled?

The current snapshot may be replaced atomically as state advances. Required
transition consequences must remain traversable until they have been projected
into another durable owner and semantic cleanup is safe. Incidental intermediate
checkpoints need not be retained forever.

The opaque durability primitive may provide predecessor linkage, immutable
revision storage, and compare-and-swap mechanics, but it does not interpret the
event or grant its authority. The role workflow continues to own transition
meaning, retention consequence, and reconstruction rules.

Git object or reflog survival is not sufficient history unless a Work Engine
contract makes the predecessor chain addressable, integrity-checked, and
recoverable. Likewise, a cumulative list of handled IDs can prevent one replay
without explaining the consequence that was handled.

## Ownership and placement consequence

Do not choose between a slice-state owner and a universal agent-state semantic
owner.

The role workflow owns the meaning and transitions of its operational state.
The existing `durable-state` capability remains an opaque, authority-neutral
mechanism for atomic publication, integrity checking, and compare-and-swap
revisions. Stronger domain owners remain unchanged:

```text
role workflow
  owns semantic projection and authorized transitions
        |
        v
durable-state primitive
  owns opaque publication mechanics only

proposal packet / receipt / checkpoint / schedule / repository
  continues to own its existing domain truth
  and is referenced rather than copied
```

The permanent owner of common logical-agent identity, parent/child topology,
visibility policy, lifecycle queries, and cross-workflow observation remains
unsettled. Those concerns may belong to the emerging control plane, but this
proposal does not grant that placement before implementation evidence exists.

## What deserves durable state

Durability should be based on reconstruction consequence, not on whether a fact
is currently present in context.

Persist a consequence when losing it would risk incorrect continuation,
authority loss, duplicated work, event resurrection, or material re-synthesis.
High token cost is useful evidence that re-synthesis is material, but cost alone
does not make remembered reasoning authoritative.

Prefer references when another owner already has the truth. Persist compact
attributed conclusions only when the role itself owns the current operational
judgment. Do not store transcripts, hidden reasoning, raw source dumps, diffs,
or test logs.

## Initial role projections

### Slice supervisor

The supervisor projection should cover every active phase, including
implementation, gate preparation, remediation, acceptance, stopping, and
authority waiting. It should preserve the exact accepted semantic boundary or
an integrity-bound reference to it, active builder identity, handled external
event consequences, current phase obligation, pending authority decisions,
applicable route revisions, and the accepted or stopped consequence.

### Slice builder

The builder projection should preserve the accepted objective and placement
certificate, current route and retired routes, task-owned and baseline-overlap
manifests, established repository evidence references, completed and remaining
validation, open review findings, unresolved questions, and the current
implementation or remediation obligation.

It should not duplicate source code, exploration transcripts, or raw gate
output. A replacement builder should be able to reconstruct the bounded
engineering position and then refresh current repository truth.

### Independent reviewer

The reviewer projection should preserve the review subject identity,
independence boundary, provider/runtime binding, claims examined, attributed
findings and their current dispositions, current delta, gate-evidence
references, and remaining uncertainties.

Visibility must remain restricted so durability does not leak builder reasoning
into a fresh reviewer or turn cross-role state access into universal shared
memory. A retained provider session remains the preferred efficient route; the
durable projection is the truthful recovery source when that session is lost.

### Strategic planner

The planner projection should preserve its current strategic thesis, active and
stale assumptions, dependency and priority rationale, deferred opportunities,
open uncertainties, evidence cutoff, last reconciled proposal/campaign state,
and continuity mode. It remains advisory and cannot mutate roadmap or campaign
authority merely because its state is durable.

### Proposal former and other artifact-centered roles

When a role's authoritative domain artifact already contains everything needed
for resumption, that artifact remains the recovery source. Proposal formation
should continue checkpointing resume-critical semantic consequences into the
packet as soon as losing them would require repeating a material decision.

A separate live projection is justified only for unfinished operational state
that cannot yet truthfully be written to the domain artifact. This test avoids
creating duplicate state merely to make every role look structurally uniform.

### Role scheduler

The scheduler remains the owner of scheduled obligations, routing, delivery,
and acknowledgement state. It may reference a logical role or agent identity,
but schedule durability must not be mistaken for role execution-state
durability or activation authority.

## Required invariants

### Durability does not inflate authority

A role must not treat its own prior judgment as new evidence merely because the
judgment became durable. Publication records an authorized consequence; it does
not accept architecture, priority, implementation, or review findings.

### State preserves consequence, not transcript

State contains the smallest current facts and attributed judgments needed for
reconstruction. Raw prompts, chain-of-thought, transcripts, broad repository
copies, and logs remain excluded.

### Stronger owners remain authoritative

Agent state references proposal packets, receipts, checkpoints, schedules,
repository revisions, and gate artifacts. It does not silently become a second
canonical copy of their truth.

### Runtime identity is a binding, not durable identity

Provider session and thread IDs are resumable provenance. A stable Work Engine
logical identity owns the role state even when its runtime binding is replaced.

### Independence constrains visibility

Durable storage availability does not imply cross-role read authority. Fresh
reviewers must not inherit builder reasoning, and unrelated roles must not gain
universal memory access.

### Handled events remain handled

External interactions record their durable operational consequence so context
replacement cannot make an old request current again.

### Recovery refreshes reality

Reconstruction restores operational position, then refreshes mutable external
truth from its owner. A durable projection is not assumed current merely because
its bytes remain readable.

### Cleanup is semantic

State may be retired only after every consequence required for active recovery,
authorized handoff, or audit exists in an appropriate durable owner.

## Proposed implementation sequence

### Slice 1: complete the slice-workflow recovery vertical

Expand the supervisor-owned active-slice projection across planning,
implementation, gate, remediation, acceptance, and waiting. Bind the exact
accepted-plan consequence rather than only a conversation reference. Add a
builder-owned projection sufficient to recover the accepted boundary, current
route, task manifest, validation progress, findings, and pending work. Prove a
fresh replacement can continue an interrupted implementation without replaying
the transcript or resurrecting a handled event. Establish the predecessor-linked
transition record and derived-current-snapshot vertical in this slice rather
than treating opaque CAS revisions as semantic history.

### Slice 2: reviewer recovery without research recreation

Persist the bounded review subject, finding ledger, dispositions, delta and gate
references alongside the already durable reviewer session binding. Prove both
same-session continuation and truthful reconstructed replacement when the
provider session is unavailable. Preserve the independence visibility boundary.

### Slice 3: strategic-planner state

Define and dogfood a planner-owned projection over its last durable handoff and
named evidence sources. Prove that a replacement planner restores assumptions,
dependencies, uncertainty, and evidence cutoff without treating remembered
state as current or gaining roadmap authority.

### Slice 4: topology, observation, and cleanup reconciliation

Use control-plane evidence to decide whether logical identity, parent/child
topology, authorized cross-role observation, lifecycle queries, and semantic
cleanup require shared machinery beyond role-owned payloads and the opaque
durable-state primitive. Do not implement coordination, leases, or resource
claims unless observed consumers require them.

## Out of scope

- full transcript or hidden-reasoning persistence;
- automatic authority, acceptance, scheduling, or roadmap mutation;
- universal cross-agent memory;
- distributed locking, parallel mutation claims, or resource leases;
- replacing proposal packets, receipts, checkpoints, schedules, or Git history;
- requiring one storage backend permanently;
- treating token cost as semantic authority;
- broad runtime introspection beyond recovery evidence needed by initial roles.

## Alternatives

### Extend only slice-supervisor live state

Insufficient. It would fix the immediately observed phase gap while leaving the
builder, reviewer, and strategic planner dependent on retained context. It also
encourages the supervisor to own semantic state produced by other roles.

### Create one universal agent-state schema

Not preferred. Role obligations, authority, visibility, and lifetimes differ.
A universal payload would either become vague or flatten meaningful role
boundaries. Share durability mechanics; keep semantic projections role-owned.

### Store everything in proposal packets or receipts

Incorrect ownership. Proposal packets explain why a candidate exists, and
receipts explain terminal outcomes. Neither should become a mutable store for
unfinished campaign, implementation, review, or planning position.

### Depend on retained provider sessions and conversation compaction

Insufficient. Retention is valuable for context economy but is not a durable
correctness boundary. Provider sessions can fail, and conversation summaries
are not role-owned product state.

## Evidence required before authority decision

- confirm the control plane's current or planned ownership of logical-agent
  identity, topology, visibility, and lifecycle observation;
- exercise the proposed supervisor/builder projection against a real context
  replacement at implementation and remediation boundaries;
- show which reviewer findings must be durable outside the provider session
  without leaking builder context;
- measure whether role projections materially reduce reconstruction work while
  remaining smaller than transcripts and domain artifacts; and
- prove that predecessor-linked consequence history can explain the current
  snapshot without relying on Git reflogs, provider transcripts, or model
  recollection; and
- validate that packet, receipt, checkpoint, schedule, and repository owners
  remain non-duplicated and authoritative.

## Acceptance consequence

This proposal is successful when a supervisor, builder, reviewer, and strategic
planner can each lose model context at a representative active boundary and a
truthfully authorized replacement can reconstruct the role's current position,
continue from referenced authoritative evidence, preserve handled consequences,
explain the predecessor-linked consequences that produced the current snapshot,
and avoid replaying completed synthesis—without persisting transcripts,
inflating authority, or collapsing role independence.
