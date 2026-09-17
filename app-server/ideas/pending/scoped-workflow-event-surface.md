# Scoped Workflow Event Surface

## Status

Operator-originated raw architectural idea, captured on 2026-09-09 while the
PPCE Slice 2 supervisor was coordinating a separately realized builder.

This document is exploratory. It does not accept a design, amend `DESIGN.md`,
change the migration roadmap, authorize implementation, select infrastructure,
or make telemetry or an execution provider authoritative for workflow state.

```yaml
idea_status:
  architectural_supersession: not_applicable
  architectural_supersession_note: "Corrected 2026-09-17, per re-audit for a category error: the 2026-09-17 unknown-drain finding above is retracted. It treated this document's own internal vocabulary (control plane, telemetry ingress, runtime adapter, operator projection, 'event surface') as if those were themselves candidate canonical dimension/mechanism/substrate names to check coverage against, rather than checking whether the document makes any dimension/mechanism/substrate-LEVEL ownership claim at all. It does not. The document is explicit about its own scope: 'The event surface would not define workflow meaning. It would transport and project events whose meanings remain owned by their producers' (Candidate direction). Re-read the 'Ownership hypothesis' and 'Event families' sections under that constraint: 'the control plane owns event envelopes, routing, scoped subscriptions, cursors, delivery, retention, filtering, and access control' is service/interface design (envelope schema, subscription/delivery mechanics) -- infrastructure responsibility, not a claim about who owns workflow meaning, the same category as a message-queue service's own internal responsibilities. 'Workflow and domain services own authoritative lifecycle events... runtime adapters translate... without promoting their local ontology into Work Engine's ontology... operator-projection services own the presentation' do not assert anything new -- they correctly preserve the ALREADY-established principles (each dimension owns its own facts; runtime-realization.md's port concept already keeps Work Engine as semantic owner while adapters translate; runtime-realization.md SS11/mechanisms/authority-preserving-intent-projection.md already establish that projection never mints authority), applied to a hypothetical transport layer. The 2026-09-17 finding's central claim -- that this proposes 'exactly the cross-cutting promotion runtime-realization.md currently declines' -- conflated sharing a TRANSPORT (moving already-dimension-owned facts to already-legitimate consumers over a wire) with promoting those facts into a new shared SEMANTIC substrate (which the document explicitly disclaims doing). 'Event families' (committed/advisory/telemetry) is a transport-routing/reliance-tier taxonomy for consumers, not a new ownership claim -- the underlying semantic distinctions it packages (authoritative vs. advisory vs. attributed-observation) are already established elsewhere. 'Scope identity' and 'Concurrency and delivery questions' (envelope coordinates, ordering, cursors, backpressure, fencing of stale realizations against masquerading events) are schema/distributed-systems design operating beneath existing ownership -- the fencing bullet applies the already-established mechanisms/transition-fencing-and-leases.md discipline to a new consumer, it does not propose a new fence class. not_applicable: no genuinely novel dimension/mechanism/substrate-level claim was found anywhere in the document; its real content is service/interface architecture for a transport layer, the same register as the software-engineering-domain-detail documents classified not_applicable elsewhere this session."
  residue: unknown
  residue_ledger: "'Questions for later intake' (8 items) tagged by kind: 5 RESIDUE, 3 BACKLOG. All 8 UNCHECKED against current canonical-view text this pass -- residue: unknown, not present. Flagged for the next item-level pass, per the supersession correction above: several of the 5 RESIDUE-kind tags (items 1, 4, 6 in particular) may be BACKLOG-kind service/schema-design questions rather than genuine dimension/mechanism/substrate ownership questions, now that this document is not_applicable rather than a live dimension-level proposal -- not resolved here, since this pass is supersession-only."
  backlog: unknown
  backlog_note: "3 KIND: BACKLOG items exist, all UNCHECKED; no confirmed-open item and no dedicated staged-plan section."
  audit_scope:
    - open-question-ledger
  audit_scope_completeness: partial
  status_as_of: 2026-09-16
```

```yaml
idea_provenance:
  origin: direct_capture
```

## Recognition event

The current App Server operator client can observe the supervisor's thread, but
the supervisor and builder execute in different threads. During PPCE Slice 2,
the builder ran for many minutes without its intermediate commentary or phase
progress appearing on the supervisor thread. The operator-facing client saw the
outer terminal result only.

The chatboard carried deliberately published coordination and terminal facts,
but it was not designed as a continuous progress channel. Existing telemetry
captured substantial execution evidence, but that evidence was not exposed as
a scoped live workflow projection.

This exposed a more general requirement:

> An operator should be able to subscribe to one authorized workflow scope and
> observe attributable lifecycle and progress events from every participating
> role, without depending on a provider-specific parent/child thread topology
> or treating transcripts as state.

The requirement becomes stronger if Work Engine removes Codex as a hard
dependency and supports multiple concurrent workflows.

## Candidate direction

Introduce a provider-neutral scoped event surface between runtime realizations,
Work Engine services, telemetry ingress, supervisors, and operator projections.

The event surface would not define workflow meaning. It would transport and
project events whose meanings remain owned by their producers.

```text
runtime and provider adapters       Work Engine semantic services
          |                                      |
          | observations                         | committed lifecycle events
          +------------------+-------------------+
                             |
                    scoped event transport
                             |
             +---------------+----------------+
             |                                |
       telemetry consumers             operator projections
                                              |
                                  authorized workflow feed
```

The main UI could subscribe to a workflow or branch scope and receive events
from its supervisor, builders, reviewers, and service-owned lifecycle without
requiring each parent role to restate every child update.

## Ownership hypothesis

The existing telemetry system may provide much of the collection,
normalization, persistence, and streaming machinery. It should not thereby
become the owner of all event meaning.

- The control plane owns event envelopes, routing, scoped subscriptions,
  cursors, delivery, retention, filtering, and access control.
- Workflow and domain services own authoritative lifecycle events. Such an
  event is authoritative only when emitted by the owner as part of, or after,
  its committed transition.
- Telemetry ingress owns normalized observations about execution: turns, tool
  activity, usage, timing, provider failures, and provider-emitted summaries.
- Runtime adapters translate Codex, Claude, and future runtime-native events
  without promoting their local ontology into Work Engine's ontology.
- Operator-projection services own the presentation of an ordered workflow
  feed, including the visible distinction between committed facts and advisory
  progress.

Telemetry is therefore a producer and possible infrastructure contributor to
the event surface, not the authority from which workflow state is
reconstructed.

## Scope identity

A candidate envelope should be able to bind an event to the smallest useful
scope without assuming that every runtime has Codex-style threads or a strict
parent/child tree. Candidate coordinates include:

- workflow and run identity;
- branch, slice, or other bounded work-unit identity;
- attempt and operation identity;
- logical role and role-instance identity;
- runtime realization and provider session identity when applicable;
- turn identity when the realization has turns;
- producer owner and schema revision;
- event ID and a sequence local to a declared scope;
- causation and correlation references;
- occurrence, observation, and publication times; and
- authority/evidence class, visibility, retention, and payload identity.

The envelope should remain claim-relative. Not every event requires every
coordinate, and missing coordinates must remain unavailable rather than being
inferred from timing, ancestry, or a role label.

## Event families

At least three families must remain distinguishable:

### Committed semantic events

Examples include plan acceptance, phase-consequence publication, candidate
binding, waiting on an acceptance owner, review admission, and terminalization.
Their semantic owner publishes them from authoritative state. Consumers may use
them according to the owner's contract.

### Advisory progress events

Examples include a bounded commentary update, current activity category,
deterministic check progress, or a provider-supplied reasoning summary. These
help an operator understand motion but do not authorize transitions or prove
completion.

Reasoning summaries are optional projections. Some providers expose them,
others expose different summaries, and some expose none. Raw chain-of-thought
is not required by this idea and should not become a portable event contract.

### Execution telemetry events

Examples include turn start/completion, tool activity, latency, token usage,
transport failure, retry evidence, and capability observations. These remain
attributable observations even when they share transport with semantic events.

An operator feed may interleave all three families while preserving their
different ownership and reliance rules.

## Concurrency and delivery questions

Multiple concurrent workflows make a global ordered stream both expensive and
semantically misleading. A candidate design should consider:

- ordering within a declared scope rather than a fictional global order;
- at-least-once delivery with stable event IDs and consumer deduplication;
- resumable cursors and bounded replay;
- backpressure and summary coalescing for high-volume tool events;
- authorization at workflow, role, evidence, and payload scopes;
- explicit gaps when retention or access prevents replay;
- per-consumer projections so an operator UI, supervisor, metrics collector,
  and researcher need not receive the same payload; and
- fencing of stale realizations so their later events cannot masquerade as the
  current logical role.

## Relationship to existing directions

This idea appears to extend rather than replace:

- `provider-turn-harness-runtime-and-operator-projection.md`, which separates
  provider turns, harness runtimes, and operator projections;
- `hierarchical-planning-and-multi-supervisor-orchestration.md`, whose
  concurrent branches need scoped operator visibility;
- `claude-runtime-adapter-and-context-ownership-pilot.md`, which makes runtime
  replacement an explicit goal; and
- `revisioned-research-and-execution-architecture.md`, whose durable state,
  coordinates, projections, receipts, and branching rules provide the
  authority and identity model the event surface must preserve.

The current quiet-client child-summary gap is one observed example, not the
sole justification and not the intended permanent implementation boundary.
Following Codex child threads would be a useful temporary bridge at most.

## Questions for later intake

1. [KIND: RESIDUE] [UNCHECKED — ownership/placement question] Does an existing telemetry/event component already own enough transport
   machinery to host the surface without also acquiring semantic ownership?
2. [KIND: BACKLOG] [UNCHECKED — investigation of current system state] Which current service transitions can emit owner-authored committed events,
   and which are presently reconstructable only from logs or telemetry?
3. [KIND: BACKLOG] [UNCHECKED — schema design] What is the minimum provider-neutral event vocabulary that preserves native
   facts without false equivalence?
4. [KIND: RESIDUE] [UNCHECKED — placement among durable states] Should durable semantic events and high-volume advisory events share one
   physical log, separate logs behind one subscription API, or another shape?
5. [KIND: RESIDUE] [UNCHECKED — authority/evidence-boundary question] How are authorization and redaction applied when one workflow includes roles
   with different evidence custody or capability ceilings?
6. [KIND: RESIDUE] [UNCHECKED — placement among durable states] Which summaries should be materialized, and which should be generated as
   consumer-specific projections from retained events?
7. [KIND: RESIDUE] [UNCHECKED — bears directly on mechanisms/transition-fencing-and-leases.md and context-lifecycle.md ownership boundaries, not independently cross-checked this pass] How does the event surface bind current realization fencing and context
   replacement without making a session or thread authoritative?
8. [KIND: BACKLOG] [UNCHECKED — implementation/sequencing question] What bounded compatibility bridge is worthwhile before the provider-neutral
   surface exists?

## Non-goals

This raw idea does not propose:

- using telemetry as a database from which canonical workflow state is rebuilt;
- making commentary, thinking summaries, tool output, or liveness authoritative;
- exposing raw transcripts or hidden reasoning to every subscriber;
- requiring all runtimes to emulate Codex threads or parent/child agents;
- replacing durable state, receipts, Claim Evidence, or service-owned
  transitions with an event bus; or
- authorizing the temporary quiet-client bridge as the final architecture.

