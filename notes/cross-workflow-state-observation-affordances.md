# Cross-Workflow State Observation as Role Machinery

## Status

Implementation note / architecture detail

## Purpose

Work Engine workflows should own their own mutable state, but authorized roles may need to inspect state owned by other workflows.

This should not be modeled as shared state ownership.

Instead, **cross-workflow observation is variant machinery available inside the observing role's environment**.

A role may be given a control that allows it to inspect an authorized projection of another workflow's state.

The observed workflow remains the sole owner of that state.

---

## Core distinction

A workflow may expose state without giving another workflow authority to mutate it.

For example:

```text
Proposal Workflow
    owns proposal_state

Slice Supervisor
    MAY_INVOKE → observe_proposal_state
    MAY_READ   → proposal.authority_projection
    MAY_NOT_WRITE → proposal_state
```

The Slice Supervisor can ask whether proposal formation and authority are complete without becoming an owner of proposal workflow state.

The state itself remains authoritative at its owning workflow boundary.

---

## Why this belongs in machinery

The invariant may be:

> **Implementation must not proceed unless required proposal authority and formation consequences are satisfied.**

That invariant does not require one specific method for establishing the fact.

The observing role may have machinery such as:

```text
observe_proposal_state
```

which returns an authorized projection:

```yaml
proposal_id: proposal-17
revision: 6
status: authorized
blocking_uncertainty: none
implementation_handoff_ref: handoff-22
```

The model may use that capability when its judgment requires current authoritative evidence.

The product should not unnecessarily encode:

```text
always inspect proposal state before starting a slice
```

if another authoritative mechanism can establish the same required consequence.

The distinction remains:

```text
invariant
→ what must remain true

machinery
→ how the environment lets the role observe or change relevant state

evidence/state
→ what is currently observed

judgment
→ which capability to use and what the result means
```

---

## Three affordance classes

Agent and workflow environments should distinguish at least three kinds of machinery.

### 1. Observation affordances

Observation affordances change what the role can know.

They do not change the observed state.

Examples:

```text
inspect proposal workflow state
inspect child-agent state
query repository evidence
read gate result
inspect checkpoint identity
inspect strategic-planner state
read current completion-offer state
```

Conceptual edge:

```text
Role ──MAY_OBSERVE────► StateProjection
```

or:

```text
Role ──MAY_INVOKE─────► ObservationCapability
ObservationCapability ──READS────► OwnedState
```

### 2. Mutation affordances

Mutation affordances allow the role to directly change state it is authorized to own or mutate.

Examples:

```text
update own durable agent state
write task-owned repository changes
record own decision trace event
advance owned lifecycle state
```

Conceptual edge:

```text
Role ──MAY_MUTATE─────► OwnedState
```

Mutation authority should remain explicit and bounded.

### 3. Mediated affordances

Mediated affordances let a role request or trigger action owned by another authority without inheriting that authority.

Examples:

```text
request publication
invoke strategic planner
request independent review
ask checkpoint adapter to create candidate
ask supervisor for plan acceptance
```

Conceptual edge:

```text
Role ──MAY_REQUEST────► OwningRoleOrCapability
```

The caller may initiate the interaction, but the target owner remains responsible for deciding or executing the state transition according to its own contract.

---

## State projections

Cross-workflow observation should generally expose a **bounded projection**, not unrestricted access to another workflow's full state.

For example, the Slice Supervisor may need:

```yaml
proposal_authority_projection:
  proposal_id: proposal-17
  revision: 6
  lifecycle_status: authorized
  blocking_conditions: []
  execution_handoff_ref: handoff-22
```

It may not need:

- proposal-former working hypotheses;
- planner decision trace;
- architectural-review private notes;
- raw interaction history;
- unrelated candidate proposals.

Likewise, a Strategic Planner may observe:

```yaml
execution_projection:
  campaign_id: campaign-9
  status: active
  accepted_slice: 14
  current_slice: 15
  unresolved_architecture: false
  observed_cost_ref: metrics-15
  latest_receipt_ref: receipt-14
```

The projection should match the observing role's legitimate decision needs.

---

## Cross-workflow visibility is part of the environment

The Agent Environment Graph should eventually represent authorized cross-workflow observation explicitly.

For example:

```text
Slice Supervisor
    BOUND_BY → proposal-authority invariant
    MAY_INVOKE → observe_proposal_state
    MAY_READ → proposal.authority_projection
    FORBIDDEN_FROM → proposal.state mutation

Strategic Planner
    MAY_READ → proposal.strategic_projection
    MAY_READ → slice.outcome_projection
    FORBIDDEN_FROM → campaign acceptance

Independent Reviewer
    MAY_READ → implementation.review_projection
    INDEPENDENT_OF → builder.decision_trace
```

This makes the role's structural environment visible:

- what it must preserve;
- what it can know;
- what it can directly change;
- what it can only request another owner to change;
- what it is explicitly forbidden from accessing or mutating.

---

## Shared work identity, separate state ownership

Multiple workflows may operate on the same work item while maintaining distinct state machines.

Conceptually:

```text
Work Dossier / Work Identity
        │
        ├── Proposal Workflow
        │      owns proposal_state
        │
        ├── Slice Workflow
        │      owns slice_state
        │
        ├── Review Workflow
        │      owns review_state
        │
        └── Strategic Planning Workflow
               owns planner_state
```

Authorized observation edges connect them.

This gives the system:

- modular state ownership;
- cross-workflow coherence;
- smaller handoffs;
- less duplicated state;
- clearer authority;
- easier reconstruction;
- better introspection.

---

## Consequence for handoffs

When workflows can observe each other's authoritative state projections, handoffs can become smaller.

Instead of transferring a large copied representation:

```text
proposal complete
+ all authority fields
+ all proposal lifecycle fields
+ all strategic context
+ all validation state
...
```

a handoff may contain:

```yaml
proposal_ref: proposal-17
proposal_revision: 6
authority_projection_ref: proposal-17/authority@6
```

The downstream role can inspect the authoritative projection when needed.

This reduces token transfer and avoids duplicating truth across workflow boundaries.

---

## Observation is not authority

A role's ability to inspect another workflow's state must never imply authority to change it.

For example:

```text
Slice Supervisor
can observe:
    proposal.status
    proposal.authority_state
    proposal.execution_handoff

cannot:
    mark proposal accepted
    rewrite proposal assumptions
    change proposal revision
    approve proposal for user
```

Observation machinery expands evidence availability.

It does not expand ownership.

---

## Observation is not mandatory sequencing

Exposing an observation capability should not automatically become a procedural rule.

Prefer:

> The proposal workflow exposes an authoritative completion and authority projection. The Slice Supervisor may inspect it when current proposal authority must be established.

over:

> Always call `observe_proposal_state()` before starting every slice.

The first describes the machinery and the consequence it can establish.

The second prescribes a route.

A deterministic gate may enforce the invariant state if the condition is mechanically checkable, but it should validate the required property rather than the invocation of one particular observation mechanism.

---

## Initial implementation direction

A first implementation could provide a generic provider-independent capability such as:

```text
observe_workflow_state(
    workflow_id,
    projection
)
```

or role-specific wrappers such as:

```text
observe_proposal_authority()
observe_slice_status()
observe_review_state()
observe_planner_state()
```

The storage and transport are variant structure.

Possible implementations include:

- SQLite-backed workflow state;
- a local state service;
- a Work Engine MCP capability;
- structured files with versioned projections;
- role-specific adapter calls.

The useful contract is:

> **An authorized role can obtain a bounded, current, attributable projection of another workflow's state without gaining mutation authority over that state.**

---

## Useful metadata

An observed state projection may include:

```yaml
workflow_id: proposal-workflow-17
workflow_type: proposal
projection: authority
state_version: 42
observed_at: 2026-08-21T15:28:00-06:00
owner: proposal-workflow
status: authorized
source_ref: state://proposal-workflow-17@42
```

This allows the observing role to distinguish:

- current from stale state;
- authoritative state from copied summaries;
- workflow owner from observer;
- projection version from mutable live context.

---

## Failure modes to guard against

### State copying

Do not turn observation into a second mutable copy of the same state.

### Authority leakage

Read access must not silently grant write or acceptance authority.

### Projection overreach

Do not expose unrelated private state merely because it exists.

### Independence contamination

Independent roles must not receive state projections that invalidate the independence claim.

### Stale observation

Observed state should carry identity/version so later decisions can detect when their evidence is no longer current.

### Procedure drift

Do not convert a useful observation affordance into a mandatory universal sequence without a causal contract.

---

## Compact statement

Workflows own their own state.

Other roles may need to see parts of that state in order to reason correctly.

> **Cross-workflow observation is machinery: an authorized knob that changes what a role can know without changing what it owns.**

Within an agent environment:

> **Observation affordances change knowledge.  
> Mutation affordances change owned state.  
> Mediated affordances ask another owner to consider a transition.**

This distinction lets Work Engine share durable truth across workflows without collapsing their authority or state boundaries.
