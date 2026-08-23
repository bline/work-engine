# Persistent Agent State

## Idea

Introduce a durable, provider-independent agent-state layer that preserves the minimum operational state required for long-running Work Engine agents to reconstruct their current position after context loss, compaction, restart, or runtime replacement.

The initial purpose is recovery from context discontinuity.

The design should deliberately preserve future reach for broader uses such as cross-agent inspection, coordination, replacement-agent continuation, observability, and eventually safe parallel work, without requiring those capabilities in the first implementation.

## Problem

Work Engine increasingly runs long-lived, state-aware agents.

Those agents accumulate operational knowledge that may not yet exist in receipts, handoffs, campaign state, or other durable artifacts. Model context currently carries part of that state implicitly.

That creates a correctness problem because model context is not durable.

Observed failures already include context compaction causing an agent to lose its current operational orientation. In one supervisor case, the agent had progressed to a later slice and was awaiting final validation, but after compaction it responded as though an already-handled user event from several slices earlier had just occurred. Some runtime facts remained recoverable, while other current-state relationships were lost or stale information regained salience.

A similar state-loss failure has been observed in a builder.

This means context compaction is not merely a context-quality or token-efficiency concern for long-running agents. When correctness depends on accumulated operational state, context loss can become product-state loss.

The transcript cannot safely be treated as the authoritative representation of that state.

## Proposed consequence

Any long-running agent whose correctness depends on accumulated state has an externally durable state representation sufficient to reconstruct its current operational position without replaying its full transcript or reasoning history.

A useful test is:

> **If this agent's model context disappeared now, would its durable state and referenced authoritative evidence be sufficient for an equivalent replacement context to understand where the work stands and continue without replaying completed reasoning?**

The state should preserve consequences, not transcripts.

It should contain enough information to establish what is true now, what has already been handled, what remains unresolved, and which durable artifacts or authoritative sources establish those facts.

## Core distinction

Work Engine should distinguish:

```text
transcript
→ what was said and observed during model interaction

agent state
→ what is operationally true for this agent now

handoff
→ the bounded projection needed across an agent boundary

receipt
→ durable evidence of what happened

proposal packet
→ durable decision context explaining why the work exists
```

These artifacts have different owners and lifetimes.

The proposal packet should not become a mutable campaign-state store.

Receipts should not be forced to represent unfinished live state.

The transcript should remain evidence rather than product state.

Agent state fills the missing live-state role.

## Initial scope

The first implementation should solve recovery and reconstruction.

It does not need to implement scheduling, locking, distributed coordination, or automatic parallel execution.

However, the initial state model should avoid assumptions that would make those capabilities difficult later.

In particular, it should not assume:

* only one agent may be active;
* agent state is private to the current model session;
* runtime session identity is the durable agent identity;
* parent and child relationships are implicit;
* only a supervisor needs durable state;
* a terminated model context means the logical Work Engine agent has terminated;
* state can only be read by the agent that created it.

## Durable agent identity

Each Work Engine agent should have a stable Work Engine-owned identity independent of the provider session currently executing it.

Conceptually:

```text
Work Engine agent
    stable agent_id

    role
    campaign_id
    parent_agent_id

    current runtime binding
        provider
        runtime session/thread identity

    durable operational state
```

Provider session or thread IDs remain valuable provenance and runtime references, but they should not define durable product identity.

This permits the same logical agent state to survive context replacement, restart, or a future change in runtime binding.

Human-readable names or agent paths may remain useful display metadata but should not serve as the primary durable identity.

## Agent topology

Parent/child relationships should be explicit.

This allows recovery to use multiple sources of operational evidence.

For example, after supervisor context loss:

```text
supervisor state
+ authorized child-agent state
+ receipts / handoffs
+ authoritative workspace/runtime state
→ reconstructed campaign position
```

The supervisor does not need to trust its own stale checkpoint in isolation.

A child may know that it is currently executing a final validation gate even if the supervisor's compacted context has lost that fact.

Likewise, supervisor state may establish campaign-level facts that the child does not own.

Disagreement becomes evidence to resolve rather than hidden inconsistency.

## Role-scoped state

The state primitive should be general, while actual state projections remain role-specific.

A supervisor may need information such as:

```text
campaign identity
current slice
latest accepted slice
active children
current campaign status
pending validation or acceptance
handled user interventions
pending authority decisions
continuation state
remaining work
```

A builder may need:

```text
bounded objective
accepted plan or current implementation claim
established ownership / placement evidence
current route
failed or retired routes
task-owned changes
user-owned workspace state that must be preserved
completed validation
remaining validation
open review findings
unresolved questions
```

A reviewer may need:

```text
review objective
independence boundary
claims examined
findings
resolved findings
current delta
remaining uncertainties
```

The system should preserve only state useful to reconstruct the role's operational position.

## Handled events

External events should preserve their durable consequence rather than relying on the transcript to remember that they were handled.

For example:

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

This prevents an old interaction from becoming active again merely because compaction causes it to regain salience.

A useful generalization is:

> **Preserve the consequence of interaction, not necessarily the interaction transcript.**

## State reconstruction

Context replacement should be treated as an expected runtime condition rather than an exceptional loss of identity.

A reconstructed agent should be able to use:

* its own durable state;
* authorized related-agent state;
* durable receipts and handoffs;
* authoritative repository, runtime, or telemetry state;
* unresolved authority decisions;
* current external events.

The exact reconstruction route belongs to model judgment unless a causal dependency requires ordering.

The product requirement is the resulting property:

> **Loss or replacement of model context must not silently revert, resurrect, or invalidate durable agent state.**

## State ownership and authority

Agent-state inspection should follow authority boundaries.

A supervisor may normally need read access to descendant operational state.

A builder should not automatically receive arbitrary sibling or reviewer state.

A reviewer whose value depends on independence must not silently inherit builder reasoning merely because the state store makes it technically accessible.

The state capability should therefore distinguish storage availability from authorized visibility.

Cross-agent state inspection is an affordance, not universal shared memory.

## Storage direction

The storage mechanism is implementation detail, but the initial choice should support:

* stable agent identities;
* parent/child relationships;
* versioned state;
* atomic updates;
* concurrent readers and writers;
* lifecycle/status queries;
* authorized cross-agent inspection;
* crash-safe persistence;
* efficient lookup by campaign and agent;
* future extension to coordination metadata.

A small SQLite store with versioned state documents is a plausible initial implementation because it provides atomicity and queryability without requiring a larger service.

The state payload itself can remain flexible, for example as versioned JSON, while identity, topology, lifecycle, and ownership metadata remain queryable fields.

The product contract should not require SQLite specifically.

## Source of truth

Agent state must not become a second authoritative copy of information that already has a stronger owner.

Where practical, state should reference or project authoritative artifacts rather than restating them.

For example:

```text
latest_terminal_receipt: receipt-11
current_slice: 12
```

is preferable to duplicating every fact contained in receipt 11.

Likewise, authoritative telemetry, repository identity, or host state should continue to come from their owning sources rather than from model-authored state.

Agent state describes operational position and references the evidence that establishes it.

## Lifecycle and cleanup

Agent state has a different lifetime from receipts.

Possible lifecycle states include:

```text
active
terminal
archived / retired
```

An active checkpoint remains available for recovery.

After an agent terminates, its state may remain temporarily useful to its parent or successor.

Once every consequence required for reconstruction, audit, handoff, or continuation exists in an appropriate durable owner, the transient checkpoint may be archived or removed.

Cleanup should therefore be semantic rather than purely time-based.

The invariant is:

> **Do not delete the last durable representation of information required to reconstruct accepted or active state.**

A time-to-live policy may be used only after that condition is satisfied.

## Future capability: integrated agent coordination

Recovery is the immediate use case, but a durable state substrate creates additional possibilities that should not be designed out of the initial model.

Potential future applications include:

* supervisor inspection of active child status without parsing transcripts;
* replacement agents inheriting operational state after intentional context retirement;
* live campaign topology and observability;
* detection of orphaned, contradictory, or stale agent state;
* coordination of multiple agents working simultaneously;
* explicit resource ownership or mutation claims;
* safe transfer of ownership between agents;
* scheduling based on active ownership and proposal placement;
* state-derived handoff projections;
* richer closed-loop execution analysis.

These remain future possibilities rather than initial requirements.

## Future capability: resource claims

If Work Engine later supports simultaneous mutation by multiple agents, the same state substrate may support atomic resource claims or leases.

The useful abstraction is broader than file locking:

```text
agent
→ claims temporary mutation authority
→ over a deterministically identifiable resource
→ for a bounded lifetime
```

Resources might eventually include files, directories, components, symbols, or contracts.

Filesystem-level conflicts may be mechanically enforceable.

Semantic overlap may remain evidence for model judgment rather than deterministic locking.

The initial agent-state design should leave room for such relationships without implementing them prematurely.

## Relationship to proposal packets

Proposal packets and agent state solve complementary problems.

```text
proposal packet
→ durable decision context before and around execution

agent state
→ live operational truth during execution

receipt / outcome
→ durable historical evidence after execution

observed outcome
→ future proposal calibration
```

The proposal packet explains why the work exists and what evidence supported selecting it.

Agent state explains where execution currently stands.

Receipts explain what actually happened.

Closed-loop learning can later compare proposal predictions with execution outcomes without forcing transient runtime state into the proposal artifact.

## Design principles

This capability should preserve the existing Work Engine doctrine.

### Truth

State must distinguish observed, inferred, unresolved, stale, and unavailable information rather than fabricating continuity.

### Context economy

The state should contain the smallest durable consequences sufficient for reconstruction, not full transcripts or hidden reasoning chains.

### Model judgment

The state store exposes current machinery and evidence. It should not become a fixed recovery procedure or scheduling policy.

### Authority

Cross-agent inspection and mutation must respect role, independence, and ownership boundaries.

### Maintainability

Durable identity and state semantics belong to Work Engine rather than to one provider's session model.

### Explainability

A maintainer should be able to determine why an agent believes work is active, complete, blocked, or awaiting authority without reconstructing hours of transcript history.

## Key design questions

1. What is the minimum state required for reliable reconstruction for each agent role?
2. Which state should be model-authored, deterministically derived, or referenced from another authoritative owner?
3. When must state be refreshed so that context loss cannot create an unsafe reconstruction gap?
4. How should conflicting parent, child, receipt, and workspace state be represented and resolved without pretending certainty?
5. What cross-agent state is visible under each authority relationship?
6. How long should terminal agent state survive after its consequences have been durably projected elsewhere?
7. Which runtime identities can be captured reliably from current providers without coupling Work Engine identity to them?
8. What schema choices preserve future multi-agent coordination without forcing coordination machinery into the first implementation?
9. How should reconstruction itself be tested so that successful resume demonstrates restored operational state rather than merely successful parsing?
10. At what point does agent state contain enough generally useful semantics that handoffs, recovery projections, and observability should be derived from it instead of independently authored?

## Initial completion evidence

A first implementation should be considered successful when:

* a long-running supervisor can lose or replace its model context and recover the correct active slice, active child, handled events, pending work, and authority state;
* a builder can recover after context loss without replaying already-established placement, implementation, and validation work;
* an already-handled user event is not resurrected as current work after reconstruction;
* parent and child state can be compared under authorized access;
* durable state remains independent of provider session identity;
* reconstructed state is checked against authoritative artifacts where they exist;
* no full transcript or hidden reasoning history is required for continuation;
* state cleanup does not remove information still required for accepted or active work.

The first version should prove recovery.

Future coordination, parallel scheduling, resource claims, and broader state-derived orchestration should remain reachable without becoming requirements of the initial slice.

