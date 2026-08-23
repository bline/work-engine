# Proposal: Role-Owned Durable Operational State

## Identity and current state

- Proposal ID: `work-engine.role-owned-durable-operational-state`
- Family ID: `work-engine.agent-state`
- State: placement uncertain; not evaluated or accepted
- Decision owner: user or future explicitly authorized architecture owner
- Evidence cutoff: repository
  `23f2e44dcb43421330c979525234b31ddbf54b93`, including the first
  continuous phase-consequence publication slice
- Review lineage: specialized review dispositions are owned by
  `reviews/proposals/role-owned-durable-operational-state/`; this narrative
  does not infer or duplicate their current closure state

The canonical lifecycle and placement metadata is in [`packet.json`](packet.json).
This narrative owns the proposal's current semantic meaning. Source ideas and
review judgments remain attributed formation inputs, not acceptance or
implementation authority.

## Independently decidable consequence

Define a common semantic contract for compact, role-owned, durable operational
state. The contract protects recovery correctness and requires ordered
transition history sufficient for downstream reconstruction, while leaving the
concrete history representation, role-profile schemas, physical hosting, and
future control-plane integration open to evidence.

The user has identified a future UI timeline as one consumer. The history
contract only needs to provide transitions in deterministic temporal order and
allow a selected retained revision to be read or reconstructed. The UI owns how
that sequence becomes a visual time-slice experience. This decision requires
durable semantic transition lineage; it does not select event sourcing, a
database, Git refs, a graph store, a control plane, or any other physical
realization.

This candidate does **not** authorize or bundle implementation of supervisor,
builder, reviewer, planner, or control-plane profiles. Each profile requires a
separately formed and authorized consequence. A profile may exercise this
contract during evaluation without making its mechanism or placement permanent.

## Problem

Long-running Work Engine roles still keep some resume-critical truth only in
model or provider context. Current durable mechanisms preserve important but
different truths:

- proposal packets preserve candidate and decision context;
- receipts and checkpoints preserve terminal slice evidence and accepted trees;
- schedules preserve future obligations and delivery state;
- provider session IDs preserve a route back to retained runtime context; and
- `durable-state` atomically stores opaque values under stable keys.

None alone preserves the unfinished operational position of an arbitrary role.
A context replacement can therefore force material re-synthesis, lose an
unfinished judgment, or cause an already processed event to be acted on again.

The observed trigger was a proposal-decision transition whose durable slice
state recorded that three user dispositions had been approved without storing
the dispositions or an integrity-bound reference to them. Recovery succeeded
only because conversation compaction happened to retain their content. The
same state vocabulary could not represent the subsequent implementation phase.

## Repository diagnosis

At the evidence cutoff:

- `ARCHITECTURE.md` verifies the bounded slice-supervisor campaign vertical and
  role-owned workflow semantics, while explicitly keeping broader future
  control-plane and UI projection structure exploratory.
- Slice live state records a limited set of supervisor facts for planning and
  review, but not a complete unfinished implementation or remediation position.
- Campaign resume reconstructs accepted terminal state from receipts,
  continuation context, and checkpoints; it does not promise mid-slice role
  recovery.
- Builder continuity and retained reviewer sessions reduce reconstruction cost
  but remain provider/runtime bindings rather than durable correctness owners.
- Proposal formation already writes material candidate consequences to its
  packet. That is the right domain-specific pattern, but it does not cover an
  unfinished role judgment that cannot yet be placed in the packet.
- The strategic planner currently reconstructs from a durable handoff and
  named evidence sources.
- The shared `durable-state` capability supplies opaque integrity-checked
  compare-and-swap publication. It does not supply workflow meaning, authority,
  event completion, visibility, retention, or history semantics.
- The first continuous-publication slice durably preserves the current latest
  implementation or gate consequence and handled consequence identities. It
  does not retain a supported, queryable sequence of full prior role-state
  revisions.
- Claim-lineage dogfood demonstrates stable semantic identity, exact revisions,
  refresh outcomes, and typed lineage for epistemic claims. Those mechanics are
  useful evidence, but claim history does not own operational workflow history.

This establishes a recovery gap and a reusable durability mechanism. It does
not establish one universal schema, mandatory predecessor chains, or the
permanent location of a general control plane.

## State and ownership taxonomy

“Agent state” is a convenience phrase, not a third semantic owner.

```text
role-owned workflow projection
  current operational facts and attributed judgments whose meaning belongs to
  the role workflow

runtime binding facts
  provider session, process, child, and observed liveness facts owned by the
  runtime or coordination mechanism that establishes them

stronger domain facts
  proposal, receipt, checkpoint, schedule, repository, review, or authority
  records owned by their existing contracts
```

The recovery view composes these through references. It does not copy them into
one universal owner. A runtime binding may be replaced without changing
workflow truth; a workflow transition cannot be inferred merely from runtime
liveness.

Every field in a role profile must declare one of three modes:

| Mode | Meaning | Required provenance |
| --- | --- | --- |
| role-owned | The workflow owns and may transition the fact or attributed judgment. | role instance, workflow/attempt identity, state revision, transition owner |
| referenced | A stronger artifact or service owns the fact. | owner identity, exact revision or integrity identity, freshness rule |
| derived | The value is recomputable and non-authoritative. | named inputs, derivation version, invalidation rule |

A profile fails stale or reports unresolved state when a required reference no
longer satisfies its integrity or freshness rule. It must not silently preserve
a copied older value as current truth.

## Protected recovery consequences

A conforming role profile must allow a replacement context to determine:

- the stable role instance and workflow or attempt it is recovering;
- the exact accepted boundary or authority record governing the next action;
- what role-owned facts and attributed judgments are operationally current;
- what remains pending, uncertain, stale, blocked, or awaiting authority;
- which source events already produced a protected consequence;
- which runtime binding may be resumed and which generation is allowed to act;
- which stronger artifacts must be refreshed before continuing; and
- which next semantic action is authorized.

Recovery must not require a transcript, hidden reasoning, or accidental
conversation-summary survival. A compact attributed conclusion may be durable
when its loss would risk incorrect continuation or material re-synthesis.
Token cost is evidence of materiality, never evidence that the conclusion is
correct or authoritative.

## Ordered transition history

Current operational state and its transition history are different projections
of the same role-owned semantics. A conforming profile must preserve enough
immutable, ordered lineage to:

- identify every retained semantic revision and its predecessor or other
  explicit ordering relation;
- attribute the transition owner, writer, trigger, authority boundary,
  evidence references, and observed publication time;
- distinguish a proposed transition, an authorized durable consequence, a
  correction, a supersession, uncertainty, and retirement;
- reconstruct the role-owned semantic projection at any retained revision;
- explain why the current projection differs from a selected prior projection;
  and
- serve bounded, deterministically ordered transition queries without replaying
  a model transcript.

A UI may turn the ordered sequence into a visual time-slice replay, and an MCP
service may expose authorized history queries. Those are consumer projections,
not additional requirements on the history representation and not new semantic
owners. Reading or reconstructing history must not re-run a tool, redeliver an
event, recreate a commit, resume a provider session, or repeat any external
effect.

The current projection may be stored directly, derived from history, or use a
snapshot-plus-tail design. Whichever realization a profile chooses must prove
that the current projection and retained history cannot silently disagree.
Snapshots may bound reconstruction cost, but they cannot erase the lineage
required by the profile's retention contract.

## Continuous publication boundary

A role must publish a resume-critical semantic consequence as soon as losing
it would risk incorrect continuation, duplicate an effect, lose authority
context, or require material reconstruction. Publication timing follows the
value and risk of the consequence; it must not wait merely for a convenient
workflow phase, terminal receipt, checkpoint, handoff, or anticipated context
compaction.

Before a runtime return, mailbox delivery, provider-session transition, or
role handoff can leave a transient channel as the only copy, the owning
workflow must publish the consequence itself or an integrity-bound reference
to its stronger canonical owner. A compact phase-completion consequence may be
required even when the enclosing slice is not yet terminal.

Advance warning of compaction may permit an efficient flush, but correctness
must not depend on receiving that warning. If publication cannot be established
after interruption, recovery must expose the consequence as uncertain and
reconcile it from authoritative evidence rather than infer completion from
remembered context or runtime liveness.

## Mechanism-open reconstruction

The binding requirement is that the current projection and retained transition
history be reconstructable, integrity-checked, idempotent, crash-safe, and
explainable from durable consequences. A predecessor-linked record is one
candidate, not a mandated architecture.

A role profile must declare:

- its atomic publication unit and concurrency-precondition boundary;
- how a crash before, during, or after publication is detected and reconciled;
- how the protected consequence behind a current fact can be identified;
- how retained revisions are ordered, queried, and reconstructed for visual
  replay;
- how supersession, correction, invalidation, and uncertainty are represented;
- which records remain reachable for recovery or audit;
- its retention, replay-suppression marker or equivalent, and physical-deletion
  rules; and
- how multiple or replaced writers are fenced.

If a profile chooses a predecessor chain, journal, or multi-object protocol, it
must additionally prove atomic publication or a prepare/commit recovery rule,
immutable roots, reachability, and garbage-collection safety. Git object or
reflog survival does not satisfy this contract by itself.

## Event and consequence identity

Do not use one “handled ID” for several lifecycle meanings. A profile dealing
with external events distinguishes:

```text
source event identity
  identity and revision assigned by the event's authoritative source

delivery attempt identity
  one routing or redelivery attempt

transition attempt identity
  one proposed role-state mutation

protected consequence identity
  the integrity-bound result whose completion makes replay unnecessary
```

An event is complete only when an authorized disposition and its protected
consequence are durable. A timeout or lost acknowledgement produces an
uncertain state that must be reconciled; it must not be converted into
“handled.” Late or out-of-order delivery consults the source revision and
protected consequence before suppression. Terminal suppression remains
explainable and durably marked by the profile's declared mechanism for the
required retention period.

## Authority and epistemic contract

Actor attribution is not authentication or authorization. Every
authority-bearing state transition names:

- the authorized transition owner and writer;
- the verifier or verification rule;
- exact subject, revision, scope, and intended consequence;
- preconditions and, when applicable, expiry, revocation, or generation;
- integrity-bound authority evidence; and
- fail-closed behavior for missing, stale, conflicting, or inapplicable input.

A stored model judgment additionally identifies its epistemic class, attributed
author or owning role, evidence cutoff, material premises, invalidation or
freshness surface, and permitted consumer use. The role may continue from that
judgment, refresh it, or mark it stale according to the profile. It may not
treat its own persisted judgment as new evidence merely because it became
durable.

## Handoff, retirement, and deletion

A source role cannot transfer semantic ownership merely by writing a target
reference. A completed handoff requires an integrity-bound consequence proving
that the authorized target has assumed responsibility without an ownership gap.
Explicit target acceptance is one realization; a domain-authorized atomic
transfer may be another. The source remains responsible for its prior
consequence until the governing contract establishes transfer or a different
authorized terminal disposition.

Retirement records why the role no longer acts and which owner now protects
each required consequence. A tombstone or equivalent durable suppression
mechanism prevents ambiguity and replay for the declared retention consequence.
Physical deletion is a separate authorized operation; storage cleanup cannot
manufacture semantic retirement.

## Minimum first-profile safeguards

Any first dogfood profile—regardless of which role is selected—must include
rather than defer:

- stable role-instance, workflow/attempt, and exact governing configuration and
  authority-boundary identity, including an execution-envelope identity only
  when an authorized envelope exists;
- one authorized-writer generation, fence, or evidence of quiescence;
- explicit read visibility for every consumer and reviewer-independence rule;
- integrity and freshness checks for every required reference;
- event/consequence separation when external events are consumed;
- crash reconciliation and retention behavior; and
- bounded historical reconstruction and sequential history listing without
  repeating effects;
- proof of the next resumed semantic action.

These safeguards may initially be local to the exercised profile. Their
presence does not settle future shared control-plane placement.

## Role-profile formation boundaries

The following are evidence-seeking profile candidates, not consequences
accepted or authorized by this proposal.

### Supervisor and builder recovery

A slice-workflow profile could preserve the supervisor-owned phase and accepted
boundary while a separate builder profile owns its unfinished engineering
position. The builder references the accepted boundary rather than copying it.
Repository evidence and validation output remain referenced artifacts. This
profile should test interruption during implementation and remediation.

### Reviewer recovery

A review profile must keep these distinct:

- specialist observation or finding;
- builder response;
- specialist re-evaluation;
- coordinator synthesis;
- residual-risk acceptance by its authorized owner;
- gate result; and
- slice or proposal decision.

Same-session continuation should remain the efficient route. Reconstructed
replacement should consume only the review subject, the reviewer's own durable
findings and limitations, the bounded delta, and permitted gate evidence. It
must be labeled continuity or reconstructed continuation of the original review
episode—not a new fresh-independence claim—and preserve the original visibility
boundary rather than exposing universal cross-role memory.

### Strategic-planner recovery

A planner profile could preserve a current thesis, assumptions, dependencies,
priority rationale, uncertainties, evidence cutoff, and last reconciled state.
It remains advisory. A durable recommendation cannot mutate a roadmap or make
continuation strategically authorized.

### Artifact-centered roles

When a proposal packet, review artifact, receipt, or other domain artifact
already contains everything needed for recovery, that artifact remains the
source. A separate live projection is justified only for unfinished operational
state that cannot yet truthfully be written to its domain owner.

## Placement questions kept open

The current repository verifies a bounded campaign controller. Product
direction describes a broader control plane, but its permanent decomposition is
not accepted architecture merely because it is useful to this proposal.

Future formation must separately determine for identity, parent/child topology,
visibility, and lifecycle queries:

- source policy: who defines the rule;
- authoritative owner: who may change the fact;
- enforcement point: who prevents invalid action;
- observation source: who can establish current runtime truth; and
- projection/query owner: who serves bounded views to consumers.

Logical identity, routing, activation, runtime bindings, delivery,
subscriptions, reconciliation, and health are plausible control-plane concerns.
They remain placement hypotheses until accepted or demonstrated by the relevant
contract. Workflow meaning remains with the workflow even if one process hosts
several mechanisms.

## Evidence required before an authority decision

- Form one independently decidable role profile with the minimum safeguards
  above and identify every field as role-owned, referenced, or derived.
- Exercise recovery after process/context loss without a transcript and have
  the replacement perform the next authorized semantic action, producing a
  downstream artifact or consequence.
- Inject failures before, during, and after publication; retry event delivery;
  test stale references, stale authority, lost acknowledgement, and an old
  writer attempting to continue after replacement.
- Demonstrate that stronger packets, receipts, checkpoints, schedules, review
  artifacts, and repository revisions remain canonical and are not copied.
- Measure reconstruction work and durable payload size without treating token
  savings as correctness evidence.
- Reconstruct at least one earlier operational revision and list a bounded,
  deterministically ordered sequence explaining the transition to current state
  without invoking an external effect.
- Demonstrate snapshot/history consistency, correction without destructive
  rewriting, and retained-history behavior across replacement and cleanup.
- Compare at least two viable history/publication realizations or record why
  the exercised constraints discriminate in favor of one.
- Use exercised consumers to evaluate whether identity, visibility, topology,
  or lifecycle projection belongs in a shared control plane or remains local.

Parsing a saved snapshot, recreating a prompt, or merely opening a replacement
session does not satisfy the recovery proof.

## Out of scope

- full transcript, hidden-reasoning, raw source, diff, or test-log persistence;
- effectful replay of tools, messages, commits, provider sessions, or external
  actions from historical state;
- automatic acceptance, authority, scheduling, gate passage, or roadmap change;
- universal cross-role memory or one universal semantic state owner;
- permanently selecting predecessor chains, event sourcing, Git refs, or any
  other storage/history realization;
- permanently allocating future control-plane ownership;
- distributed resource claims not required by an exercised profile; and
- implementation authorization for any role profile.

## Alternatives

### Extend only slice-supervisor live state

This may be a valid first profile but is not a complete semantic foundation. It
must not cause the supervisor to own builder, reviewer, or planner judgments.

### Create one universal agent-state schema

Not preferred. Roles have different obligations, authorities, visibility, and
lifetimes. Reuse the recovery contract and opaque durability mechanics while
keeping profile semantics role-owned.

### Put unfinished work in packets or receipts

Only when those artifacts' domain contracts already own the consequence.
Proposal packets explain candidates and decisions; receipts explain terminal
slice outcomes. Neither should become a generic mutable live-state store.

### Depend on retained sessions and conversation compaction

Useful for context economy, insufficient for correctness. Provider sessions can
fail and conversation summaries are not role-owned product state.

## Acceptance consequence

An authority decision on this proposal decides only whether Work Engine should
adopt the mechanism-open semantic contract above for future role-state profiles:
role-owned current operational meaning, retained and sequentially queryable
transition lineage, references to stronger owners, explicit authority and
epistemic provenance, distinct event and consequence identity, fenced and
crash-safe recovery, and truthful handoff and retirement.

It does not select a schema, storage/history mechanism, role-profile rollout,
control-plane placement, roadmap priority, or implementation authority. Those
remain separately formed and authorized decisions.
