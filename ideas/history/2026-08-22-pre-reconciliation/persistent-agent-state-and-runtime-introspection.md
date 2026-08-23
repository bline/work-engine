# Persistent Agent State and Runtime Introspection

## Status

**Idea / architecture proposal**

This document describes a proposed Work Engine capability for durable agent state, recovery across context loss, cross-agent state inspection, semantic transition history, and future runtime introspection.

The immediate motivating problem is context compaction and loss of operational state in long-running agents.

The design should solve recovery first while preserving future reach for:

- persistent reviewer loops;
- cross-agent reconstruction;
- replacement-agent continuation;
- runtime observability;
- semantic execution history;
- live workflow visualization;
- eventual multi-agent coordination and resource claims.

Those future capabilities are design consumers, not initial implementation requirements.

---

## 1. Motivation

Work Engine increasingly runs long-lived, state-aware agents.

A supervisor may remain active across many slices. A builder may carry architectural understanding, implementation decisions, validation state, and unresolved findings for a long bounded task. A reviewer may accumulate findings across several remediation iterations.

Today, part of that operational state lives only in model context.

Model context is not durable.

Observed failures already show that compaction can preserve some facts while losing or distorting others. In one supervisor session, the campaign had advanced several slices and the active worker was completing a later validation gate. After context compaction, the supervisor responded to an already-handled user event from several slices earlier as though it were current. Some runtime facts were still recoverable; the supervisor's current operational orientation was not.

A similar state-loss failure has been observed in a builder.

This means context compaction is not merely a token-efficiency or prompt-quality concern.

For a long-running state-aware agent:

> **Context loss can become product-state loss.**

The transcript is therefore insufficient as the authoritative representation of operational state.

---

## 2. Core principle

The model context should be treated as ephemeral cognitive working memory.

The process should not live only inside it.

> **Model context is ephemeral execution memory. Agent state is product state.**

Any long-running agent whose correctness depends on accumulated state should externalize enough durable state that an equivalent replacement context can reconstruct the agent's current operational position without replaying its full transcript or reasoning history.

A useful test is:

> **If this agent's context disappeared now, would its durable state and referenced authoritative evidence be sufficient for a replacement context to understand where the work stands and continue without replaying completed reasoning?**

The target is reconstructability, not exhaustive preservation.

---

## 3. Preserve consequences, not transcripts

The state layer should follow the existing Work Engine principle:

> **Preserve the consequence of reasoning, not necessarily the reasoning transcript.**

The same principle extends to interaction:

> **Preserve the consequence of interaction, not necessarily the interaction transcript.**

For example, if the user commits and pushes during a slice, durable state should not depend on remembering the conversation in which that happened.

It should preserve the resulting consequence:

```text
event:
    user committed and pushed repository

disposition:
    handled

durable consequence:
    baseline re-anchored
    commit classified as user-owned
    active worker informed
    no remaining action
```

After compaction, an old message may regain salience. The durable disposition should make clear that the event is historical and already incorporated.

---

## 4. Distinct durable information layers

Work Engine should distinguish several different information lifetimes and owners.

```text
proposal packet
    why this work exists
    what was believed before execution
    placement / evidence / estimates / decision

agent state
    what is operationally true for this agent now

campaign state
    what is operationally true for the campaign now

handoff
    the bounded projection needed across an execution boundary

receipt
    what durably happened

transition history
    how state changed over time

raw session
    forensic interaction evidence
```

These should not collapse into one artifact.

In particular:

- the proposal packet should not become a mutable campaign database;
- the transcript should not become authoritative live state;
- receipts should not be forced to represent unfinished live work;
- agent state should not duplicate authoritative artifacts unnecessarily.

---

## 5. Persistent agent identity

Each Work Engine agent should have a stable Work Engine-owned identity independent of the provider session currently executing it.

Conceptually:

```text
Work Engine agent
    agent_id                  # stable product identity
    campaign_id
    parent_agent_id
    role

    runtime bindings
        provider
        runtime session/thread identity
        display name / agent path

    durable operational state
```

Provider session IDs, thread IDs, nicknames, or agent paths remain useful provenance and runtime references, but they should not define durable identity.

A logical Work Engine agent may eventually survive:

- context compaction;
- session restart;
- deliberate context retirement;
- process failure;
- replacement by another provider/model instance.

Durable identity should survive those transitions.

---

## 6. Agent topology

Parent/child relationships should be explicit.

This enables reconstruction from more than one model-authored source.

For example:

```text
supervisor state
+ child-agent state
+ durable receipts / handoffs
+ authoritative workspace/runtime state
→ reconstructed campaign position
```

If a supervisor loses context, its own state tells it what it believed was happening.

Its child agents may know what they are currently doing.

Receipts establish accepted history.

Git, tests, telemetry, and runtime state establish stronger external facts.

Disagreement between these sources should become evidence to resolve rather than hidden inconsistency.

This suggests that cross-agent state inspection should be a general capability, subject to authority boundaries.

---

## 7. Role-scoped state

The state primitive should be general.

The actual state projection should remain role-specific.

### Supervisor state may include

```text
campaign identity
current slice
latest accepted slice
active child agents
current campaign status
pending validation
pending acceptance
handled user interventions
pending authority decisions
continuation state
remaining work
latest durable artifacts
```

### Builder state may include

```text
bounded objective
accepted plan / current implementation claim
established placement / ownership evidence
current route
failed or retired routes
task-owned changes
known user-owned workspace state
completed validation
remaining validation
open review findings
unresolved questions
important artifact references
```

### Reviewer state may include

```text
review identity
review objective
independence origin / boundary
review scope
reviewed revision
findings
finding dispositions
current delta
remaining uncertainties
```

The state should preserve only what is needed to reconstruct the role's operational position.

---

## 8. Reviewer continuity

Persistent state is particularly useful if independent review changes from:

```text
fresh reviewer
→ finding
→ discard reviewer
→ fresh reviewer
→ re-review everything
```

to:

```text
fresh independent reviewer
→ initial review
→ findings
→ builder repair
→ same reviewer evaluates delta
→ repeat while useful
→ acceptance
```

Freshness establishes independence at the start.

Persistence during the bounded remediation loop preserves useful reviewer understanding without weakening that initial independence.

Reviewer state should give findings stable identities.

A finding lifecycle may include:

```text
open
repaired
rejected
superseded
accepted_as_risk   # if later needed
```

A rejected finding must remain explicit.

Rejected does not mean forgotten.

It means adjudicated.

For example:

```text
finding_id
status: rejected
rejected_by
reason
evidence / authority basis
revision_at_rejection
```

If the same reviewer continues, the agent launching or coordinating the reviewer must return the disposition to the reviewer before the next iteration.

Otherwise the reviewer remains in stale state and may legitimately re-raise the same issue.

A useful invariant is:

> **Every adjudicated review finding must have its disposition reflected in the retained reviewer's active state before the next review iteration.**

The state substrate should support that bidirectional flow:

```text
reviewer raises finding
    ↓
supervisor records finding identity
    ↓
builder / supervisor adjudicates
    ↓
status becomes repaired / rejected / superseded
    ↓
disposition returns to reviewer
    ↓
reviewer updates durable state
```

---

## 9. Authority and state visibility

Cross-agent state inspection should not imply universal shared memory.

Different relationships have different visibility requirements.

A supervisor may normally need read access to descendant operational state.

A builder should not automatically inherit arbitrary sibling or reviewer state.

A reviewer whose evidence claim depends on independence must not silently absorb builder reasoning merely because the state store makes it technically available.

Possible future visibility concepts include:

```text
self
parent-readable
descendant-readable
explicitly-shareable
role-private
independence-protected
```

The first implementation does not need a large permissions taxonomy.

It should simply avoid assuming that all state is globally readable.

---

## 10. Current state and change explanation

An authoritative current-state owner must support truthful reconstruction. The
degree of retained change history depends on the explanation, audit, retry, and
introspection needs of its identified consumers.

The state system should distinguish:

```text
CURRENT STATE
what is true now

CHANGE EVIDENCE
enough identity, order, cause, and provenance to explain consequential changes
```

An append-only semantic transition history is one possible way to supply that
evidence. Transactional snapshots, database journals, compare-and-swap
documents, or another owner-specific design may preserve the same consequences.
The required property is truthful reconstruction and consumer-proportionate
explanation, not a universal event representation.

A possible event shape:

```text
event_id
campaign_id
event_seq
agent_id
event_type

occurred_at
recorded_at

from_state
to_state

source_agent_id
target_agent_id

cause_ref
artifact_refs
finding_id
route_id
gate_run_id
proposal_ref
```

Not every event needs every field.

The important property is stable identity and sufficient semantics for reconstruction.

---

## 11. Time and ordering consequences

Timestamps should exist on durable facts and transitions, but wall-clock time should not be the only ordering mechanism.

Useful time semantics include:

```text
created_at
updated_at
observed_at
effective_at
ended_at
```

Updates need enough identity and ordering semantics for the selected owner to
distinguish retries, delayed observation, and conflicting writes. A monotonic
sequence within a campaign is one possible mechanism, not a product-wide
requirement.

For example:

```text
campaign_id
event_seq
event_id
occurred_at
recorded_at
```

This distinguishes:

- when something actually happened;
- when Work Engine learned about it;
- when it was persisted.

That distinction matters under concurrency, retries, delayed reporting, and cross-agent observation.

A useful consequence is:

> **Durable updates retain enough time, identity, order, and provenance to make
> reconstruction and retry truthful for their identified consumers.**

Then:

```text
current state
= truth owned by the selected authoritative state boundary

transition log
= one possible history of how truth changed

timestamps + owner-specific ordering evidence
= enough information to resolve reconstruction and retry
```

---

## 12. Consequential changes, not UI noise

When the selected owner retains change history, it should record
product-meaningful consequences rather than interaction noise.

Examples:

```text
agent spawned
plan accepted
implementation started
route revised
finding opened
finding repaired
finding rejected
authority requested
authority granted
validation started
validation failed
validation repaired
validation passed
slice accepted
handoff emitted
handoff consumed
context lost
agent reconstructed
agent replaced
campaign stopped
campaign completed
```

Avoid polluting the history with low-value interaction noise such as every tool call, printed message, or wait cycle unless it has product meaning.

Raw sessions remain available for forensic detail when needed.

---

## 13. Existing data sources

The state layer should connect existing authoritative or semi-authoritative data rather than becoming a new place where everything is copied.

Current Work Engine already has or can expose several useful evidence surfaces.

### Proposal packets

Likely sources:

```text
objective
intended consequence
placement
architectural owner
alternatives
dependencies
expected impact
expected complexity
fan-out
risk
validation burden
expected cost
confidence
decision
```

These are pre-execution decision context.

### Receipts

Useful durable history may include:

```text
slice identity
terminal status
route revisions
provider identity
evidence mode
fallback events
configuration provenance
validation breadth
unresolved concerns
continuation projection
outcome
```

### Compact handoffs

These preserve inter-slice consequences needed for continuation without requiring transcript replay.

### Metrics and telemetry

Current or planned telemetry may expose:

```text
timing
token use
cost
provider role
evidence mode
route
validation
review
outcome
repair
context occupancy
```

These should come from authoritative telemetry sources where possible rather than model transcription.

### Raw sessions / transcripts

Raw sessions are valuable forensic evidence.

They are useful for:

- debugging state divergence;
- studying compaction failures;
- reconstructing why an event was created;
- auditing model interaction behavior.

They should not be required for normal continuation.

### Git / workspace state

Potential authoritative facts include:

```text
HEAD
branch
dirty paths
commits
diff boundaries
user-owned changes
task-owned changes
```

Agent state should usually reference or summarize the consequence of this data, not copy the repository wholesale.

### Gate / test results

Potential durable references include:

```text
gate_run_id
validation_status
validation_scope
failed checks
passed checks
failure artifact refs
```

### Reviewer findings

Structured review data may include:

```text
review_id
finding_id
status
evidence refs
repair refs
rejection disposition
reviewed revision
```

### Runtime agent registry

Useful runtime data may include:

```text
agent_id
provider runtime identity
parent
children
role
status
last_seen
```

### Repository evidence state

Repository evidence tools may expose:

```text
index generation
repository HEAD/base identity
coverage status
freshness
known evidence gaps
```

### User interventions and authority decisions

Important interaction events may include:

```text
plan approved
scope changed
user commit/push
mutation authorized
finding rejected
risk accepted
stop requested
authority withheld
```

These should gain stable identity and durable disposition when they materially affect execution.

### Route history

A route invalidation can remain compact:

```text
failed premise
stale dependent decisions
still-valid evidence
replacement route
reason
```

### Proposal predictions

Later closed-loop learning may connect:

```text
predicted cost        ↔ observed cost
predicted complexity  ↔ observed effort
predicted fan-out     ↔ observed fan-out
predicted risk        ↔ repairs / regressions
predicted validation  ↔ observed validation cost
predicted impact      ↔ later measured consequence
confidence            ↔ prediction accuracy
```

---

## 14. State should reference authoritative data

A general rule should be:

> **The state layer should connect authoritative data, not become the place where all authoritative data is copied.**

Prefer:

```json
{
  "validation_status": "passed",
  "validation_run_ref": "gate-run-8f12",
  "validation_scope": "full"
}
```

over:

```json
{
  "validation": "all 53 tests passed..."
}
```

Prefer:

```json
{
  "open_findings": ["finding-F7"]
}
```

over copying the full reviewer narrative into every agent-state record.

Prefer:

```json
{
  "latest_terminal_receipt": "receipt-11",
  "current_slice": 12
}
```

over restating every fact already owned by receipt 11.

This keeps state compact, auditable, and less likely to drift from stronger owners.

---

## 15. Reconstruction

Context replacement should be treated as an expected runtime condition.

A reconstructed agent may use:

```text
own durable state
+ authorized related-agent state
+ campaign state
+ receipts / handoffs
+ authoritative repository/runtime facts
+ current external events
→ reconstructed operational position
```

The exact route belongs to model judgment unless causal dependencies require ordering.

The product requirement is the resulting property:

> **Loss or replacement of model context must not silently revert, resurrect, or invalidate durable agent state.**

Reconstruction should be capable of detecting conflicting evidence.

For example:

```text
supervisor checkpoint:
    slice 12
    awaiting final gate

child checkpoint:
    slice 12
    running final gate

latest terminal receipt:
    slice 11 accepted

old user event:
    commit/push handled at slice 9
```

The durable evidence makes it difficult for a reconstructed supervisor to mistake the historical commit event for current work.

---

## 16. Storage direction remains open

The storage mechanism is variant structure.

The product doctrine should not require SQLite specifically.

The initial implementation must preserve the following consequences to the
degree required by its selected owner and consumers:

- stable Work Engine agent identities;
- parent/child topology;
- versioned state;
- atomic updates;
- concurrent readers and writers;
- lifecycle queries;
- authorized cross-agent inspection;
- crash-safe persistence;
- efficient lookup by campaign and agent;
- enough change explanation for consequential recovery decisions;
- room for future coordination metadata without prematurely implementing it.

A small SQLite database is one plausible implementation, alongside
transactional documents, journals, or other crash-safe stores.

One useful shape would be relational identity/topology metadata with flexible versioned JSON state payloads.

Conceptually:

```text
campaigns
agents
agent_state
events
reviews
findings
artifact_refs
```

The first version does not need every future table.

The important choice is not to trap the system in a representation that cannot
serve the identified reconstruction and authority consumers. Cross-agent
queries, transition history, and atomic coordination remain potential future
pressures rather than assumed first-version requirements.

---

## 17. Lifecycle and cleanup

Agent state should have an explicit lifecycle.

Possible states:

```text
active
terminal
archived
retired
```

An active checkpoint remains available for recovery.

A terminal agent's state may remain useful to:

- its parent;
- a successor;
- a reviewer;
- reconstruction;
- audit.

Once every consequence required for continuation, audit, handoff, or accepted state exists in an appropriate durable owner, transient checkpoint state may be archived or removed.

Cleanup should therefore be semantic rather than purely time-based.

Invariant:

> **Do not delete the last durable representation of information required to reconstruct accepted or active state.**

A TTL may be added only after that condition is satisfied.

---

## 18. Future capability: multi-agent coordination

Recovery is the first use case.

The initial state model should deliberately avoid assumptions that would design out future coordination.

Potential later uses include:

- supervisor inspection of active child state;
- multiple builders operating simultaneously;
- detection of conflicting ownership;
- replacement agents;
- state-based scheduling;
- orphan detection;
- explicit waiting/blocking relationships;
- safe ownership transfer.

The first version does not need to implement a scheduler.

It should simply preserve the possibility.

---

## 19. Future capability: resource claims

If Work Engine eventually supports simultaneous mutation in different repo regions, the state substrate could support atomic resource claims or leases.

The useful abstraction is broader than file locking.

Conceptually:

```text
claim_id
agent_id
campaign_id
resource_type
resource_ref
mode
reason
acquired_at
lease_expires_at
status
```

Possible resource types might later include:

```text
file
directory
symbol
component
contract
```

Filesystem claims may be mechanically enforceable.

Semantic overlap may remain evidence for model judgment rather than a deterministic lock.

The initial state design should leave room for this without implementing it prematurely.

---

## 20. Future capability: introspection UI

The emerging runtime is naturally suited to an introspection surface.

The UI should not maintain a second interpretation of runtime state.

It should project the authoritative live-state owner, any change evidence that
owner exposes, and referenced stronger evidence. The UI must not make one
storage or history design mandatory merely because it can render it.

A future UI could expose several complementary views.

### Live topology

```text
Campaign
├── Supervisor [active]
│   ├── Builder 12 [validating]
│   │   └── Reviewer 7 [waiting on repair]
│   └── Evidence agent 4 [complete]
│
├── Slice 11 [accepted]
├── Slice 12 [active]
└── Pending authority [none]
```

This answers:

> **What is happening right now?**

### Execution flow

```text
plan
 ↓
implement
 ↓
review ─── finding ───→ repair
 ↑                        │
 └────────────────────────┘
 ↓
validate
 ↓
accept
```

This answers:

> **How did this work actually proceed?**

Unlike a static workflow diagram, this would show the route that really happened, including:

- route revisions;
- failures;
- retries;
- review loops;
- user interventions;
- context recovery;
- abandoned routes;
- parallel branches.

### Timeline / replay

```text
13:04 agent spawned
13:11 finding F3 opened
13:18 repair submitted
13:23 F3 rejected
13:31 validation passed
13:32 slice accepted
```

A future UI could scrub through campaign time and reconstruct the graph as it existed at any point.

### Evidence drill-down

Nodes and edges should be able to map back to underlying data.

Examples:

```text
Finding F3
→ reviewer state
→ evidence refs
→ repair revision
→ rejection disposition
```

```text
Builder 12: validating
→ agent state
→ gate run
→ workspace revision
→ open findings
```

---

## 21. State should be observable by construction

The introspection UI does not need to be built now.

But the state model should be designed so that a future UI does not need an LLM to reinterpret opaque prose.

Prefer structured semantics:

```json
{
  "agent_id": "agent-12",
  "status": "active",
  "activity": "validation",
  "parent_agent_id": "agent-supervisor",
  "waiting_on": ["gate:regression"],
  "open_findings": ["F3"]
}
```

over:

```text
I'm mostly done and I think we're just waiting for the tests,
except the reviewer had one concern...
```

A useful future-facing consequence is:

> **State and transition semantics should remain sufficiently structured and referential that a future introspection surface can reconstruct live topology, execution history, and meaningful causal relationships without parsing model transcripts.**

---

## 22. Stable identity for graphable concepts

If a concept may matter in execution history or introspection later, giving it stable identity now is valuable.

Possible identifiers include:

```text
campaign_id
proposal_id
agent_id
review_id
finding_id
route_id
gate_run_id
handoff_id
receipt_id
event_id
authority_event_id
resource_claim_id
```

Not every identifier must exist in the first implementation.

The design should simply avoid representing important relationships only as free-form prose.

---

## 23. History as execution evidence

Transition history creates value beyond visualization.

It can support:

- recovery;
- debugging;
- campaign replay;
- performance analysis;
- failure attribution;
- reviewer-loop measurement;
- route-revision analysis;
- context-loss analysis;
- closed-loop engineering learning.

Instead of merely recording:

```text
slice took 48 minutes
```

future analysis may be able to determine:

```text
48 min total
├── exploration
├── implementation
├── review / repair
└── validation
```

The purpose is not to impose one workflow taxonomy.

It is to preserve enough semantic execution evidence for later analysis.

---

## 24. Relationship to closed-loop learning

Persistent state and transitions may become the execution-side evidence source for proposal calibration.

Conceptually:

```text
proposal packet
    predicted impact / cost / complexity / risk
          ↓
execution
          ↓
agent-state transitions
receipts
metrics
review findings
validation events
          ↓
observed outcome
          ↓
future proposal evaluation improves
```

Historical observations should become empirical priors, not new mandatory procedures.

For example:

```text
proposal class
→ historically high placement-error rate
→ evaluator receives stronger evidence that
   placement uncertainty deserves attention
```

not:

```text
proposal class
→ placement failed before
→ always run procedure X
```

---

## 25. Design boundaries

### Truth

State must distinguish:

```text
observed
inferred
decided
unresolved
conflicting
unavailable
stale
```

Reconstruction must not fabricate certainty.

### Context economy

Persist the smallest useful consequences, not complete transcripts or hidden reasoning chains.

### Model judgment

State exposes the environment.

It should not become a deterministic workflow script.

### Authority

Cross-agent visibility, mutation, review independence, and eventual resource claims must respect role and ownership boundaries.

### Provenance

Durable state should point to the evidence and artifacts that establish important facts.

### Maintainability

Work Engine identity and state semantics should remain provider-independent.

### Explainability

A maintainer should be able to understand current agent/campaign state without reconstructing hours of transcript history.

### Aesthetics

Future introspection should reflect the actual structure of the engine rather than inventing a cosmetic workflow diagram unrelated to runtime truth.

---

## 26. Initial implementation target

The first implementation should remain narrow.

It should prove durable reconstruction for state-aware long-running agents.

A reasonable first target should establish these consequences:

- Work Engine-owned `agent_id`;
- explicit campaign and parent-agent relationships;
- role-specific versioned state;
- atomic state updates;
- timestamps and state versions;
- enough identity and ordering evidence for truthful retry and reconstruction;
- references to authoritative artifacts;
- authorized parent inspection of child state;
- supervisor reconstruction after context loss;
- builder reconstruction after context loss;
- handled-event persistence;
- reviewer state compatibility, even if persistent reviewer loops arrive later.

It should not require:

- a visual UI;
- general parallel scheduling;
- resource locking;
- distributed execution;
- full event sourcing;
- complete historical analytics.

Those should remain reachable.

---

## 27. Initial completion evidence

The first version should be considered successful when:

- a long-running supervisor can lose or replace context and recover the correct active slice, active child, pending action, handled events, and authority state;
- a builder can recover after context loss without replaying already-established implementation and validation work;
- an already-handled user event is not resurrected as current work after reconstruction;
- parent and child state can be compared under authorized access;
- state identity survives provider/runtime session replacement;
- reconstructed state can be checked against stronger authoritative artifacts;
- no full transcript or hidden reasoning history is required for continuation;
- repeated recovery across a relevant write boundary does not duplicate a
  consequence or resurrect stale work;
- cleanup cannot remove the last representation required for accepted or active work;
- the resulting data model remains capable of supporting future introspection without transcript parsing.

---

## 28. Key design questions

1. What is the minimum reconstructive state for each agent role?
2. Which state is model-authored, deterministically derived, or merely a reference to another owner?
3. What state transitions are meaningful enough to persist?
4. When must state be refreshed to prevent a recovery gap?
5. How should conflicting parent, child, receipt, runtime, and workspace state be represented?
6. Which cross-agent state relationships are authorized?
7. How should reviewer independence interact with state visibility?
8. What runtime provider identities can be captured reliably without becoming durable product identity?
9. How should state versioning and schema migration work?
10. What ordering guarantees are required for concurrent transition writers?
11. Should current state be materialized directly, reconstructed from events, or maintained as a projection of both?
12. Which event and artifact identifiers deserve stable product identity now?
13. What state must survive agent termination, and for how long?
14. Which state should become a handoff projection rather than a separately authored artifact?
15. Which reviewer finding transitions must be returned to a retained reviewer before the next iteration?
16. How should raw sessions map back to semantic events for forensic investigation without becoming runtime dependencies?
17. What proposal-packet fields should be referenced during execution rather than copied into live state?
18. What metrics and telemetry should be linked directly to transitions?
19. What minimal schema preserves future resource claims and parallel coordination without implementing them now?
20. What information must be structured today so that a future live execution graph can be rendered directly from runtime truth?

---

## 29. Compact statement

The immediate problem is context loss.

The larger opportunity is durable agent execution state.

The system should evolve toward:

> **A Work Engine process whose truth survives any individual model context.**

Agent context becomes temporary cognitive workspace.

Persistent state records where the process is now. When an identified consumer
needs change history, the selected owner records enough evidence to explain how
that truth changed.

Artifacts and evidence establish why those states are believed.

Receipts preserve what durably happened.

Proposal packets preserve why the work was chosen.

A future introspection surface can project those same semantics into a live graph of the workflow that is actually happening, rather than a diagram of the workflow someone expected to happen.
