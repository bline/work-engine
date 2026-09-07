# Idea: Provider Turn, Harness Runtime, and Operator Projection

**Status:** Architecture idea

**Scope:** Provider execution, coding-harness integration, operator interfaces,
and the boundaries through which Work Engine composes them

**Authority:** Exploratory only; this document does not amend the migration
roadmap, admit an implementation, select a provider, authorize spending, or
authorize implementation.

## Summary

Work Engine should preserve three independently replaceable architectural
ports:

```text
Work Engine authority and durable services
        |
        +-- ProviderTurnPort
        |     direct providers or provider-protocol substrates
        |
        +-- HarnessRuntimePort
        |     Codex App Server, Claude Code, or later harnesses
        |
        +-- OperatorProjection
              Codex TUI, OpenCode TUI, web UI, or another client
```

These ports separate three concerns that are often bundled by an agent product:

1. executing one provider request and observing its result;
2. realizing a coding-agent session with tools, approvals, and native lifecycle
   behavior; and
3. presenting Work Engine state and authorized controls to an operator.

The split is provider-neutral. Codex, Claude Code, OpenCode, direct APIs, and
future systems are candidate implementations of one or more ports; none defines
Work Engine's runtime ontology.

The ports define what mechanisms can be supplied. The independently proposed
[capability inventory, policy overlay, and materialized realization](./pre-indexed-capability-resolution-and-frozen-runtime-realization.md)
defines which compatible mechanisms are presently admitted, why they were
selected, and how Work Engine safely replaces them when their dependencies
change.

## 1. Architectural boundary

Work Engine retains ownership of:

- orchestration and role topology;
- role contracts, authority, and capability ceilings;
- durable workflow and logical role identity;
- context-lifecycle policy;
- admission, fencing, reconciliation, and restart semantics;
- runtime-selection policy and spending authority;
- checkpoint and evidence requirements; and
- the canonical interpretation of operator commands.

Port implementations supply mechanisms and observations. They must not acquire
those Work Engine authorities merely because they expose a convenient session,
router, event schema, or user interface.

## 2. ProviderTurnPort

`ProviderTurnPort` represents one bounded provider request or stream.

Its contract should cover:

- request preparation and attributed provider/model selection;
- streaming text, reasoning, tool-call, finish, usage, and error observations;
- cancellation and transport termination;
- provider capability discovery;
- provider-native request options and escape hatches;
- stable event and operation identity;
- raw-evidence custody or explicit authorized omission;
- retry and reconciliation boundaries; and
- explicit unsupported-operation behavior.

It must not own:

- Work Engine continuation decisions;
- context-transition policy;
- workflow or logical-role state;
- tool-loop orchestration unless the admitted realization explicitly assigns it
  that ownership; or
- authority to select a paid provider merely because credentials are available.

Portable events should normalize only demonstrated common meaning. Native
provider facts must remain attributable rather than being discarded to create
false equivalence.

## 3. HarnessRuntimePort

`HarnessRuntimePort` represents a coding-agent harness and its native execution
semantics.

Its contract should expose, where supported:

- thread or session realization and continuity;
- harness-owned provider-turn execution;
- native tools and tool-loop behavior;
- workspace and shell execution;
- sandbox and approval mechanics;
- interruption and steering;
- native context-transition mechanisms;
- lifecycle, usage, and completion observations;
- restart and reconciliation evidence; and
- explicit capability discovery and unsupported-operation behavior.

Harness capabilities must not be mislabeled as provider capabilities. Codex
`new_context`, for example, is a Codex harness mechanism even when the harness
ultimately calls an OpenAI model. A direct provider adapter does not become
equivalent merely because it can send inference requests.

Harness adapters may preserve genuinely different semantics. The port is a
boundary for truthful composition, not a requirement that Codex App Server,
Claude Code, and future harnesses pretend to be interchangeable.

## 4. OperatorProjection

`OperatorProjection` presents Work Engine-owned state and accepts bounded
operator requests.

It may expose:

- logical roles and active realization identities;
- admitted, running, paused, stale, or queued operations;
- capability observations and invalidation reasons;
- context lifecycle and transition state;
- leases, fences, approvals, and reconciliation state;
- runtime-selection and cost-policy settings;
- administrative commands; and
- attributable history of operator-directed changes.

The projection must not make a UI session the canonical role, workflow,
context, or policy identity. UI controls submit commands or proposed overlay
revisions to Work Engine; they do not directly mutate active adapters.

This port is intentionally asymmetric with the two execution ports. A
projection observes and controls Work Engine but ordinarily is not a dependency
of role execution. Replacing a TUI or web client should not stale an active
runtime realization unless an admitted operation specifically depends on that
projection for an interaction such as approval.

## 5. Composition and ownership

A realization must describe ownership, not just select one implementation from
each list.

### Harness-owned provider turn

```text
HarnessRuntimePort = Codex App Server
provider-turn owner = Codex harness
tools owner = Codex harness
context transition = Codex new_context
```

The underlying provider route may be configurable, but Work Engine must not
also start an independent provider turn for the same logical operation.

### Direct-provider composition

```text
ProviderTurnPort = direct Anthropic adapter
HarnessRuntimePort = Work Engine generic harness
provider-turn owner = Work Engine generic harness through ProviderTurnPort
tools owner = Work Engine generic harness
context transition = separately admitted Work Engine mechanism
```

### Candidate substrate composition

```text
ProviderTurnPort = provider-protocol substrate
HarnessRuntimePort = independently selected harness
OperatorProjection = independently selected client
```

A product that offers all three facilities is not automatically admitted for
all three ports. Each seam requires its own capability and authority evidence.

The realization contract should reject incompatible or ambiguous ownership,
including:

- two provider-turn owners;
- two tool-loop owners;
- a cancellation mechanism that cannot identify the operation it cancels;
- a context transition with no authoritative lifecycle observer;
- an approval surface that bypasses Work Engine authority; or
- an operator projection that silently changes runtime state.

## 6. Relationship to capability inventory and policy overlay

The ports and the realization mechanism occupy different layers:

```text
role contracts + operator policy overlay
                    |
capability observations from port implementations
                    |
                    v
          resolution and admission
                    |
                    v
       immutable runtime realization
                    |
          +---------+---------+
          |                   |
 ProviderTurnPort      HarnessRuntimePort
          |                   |
          +------ execution --+
                    |
          evidence and observations

OperatorProjection reads this state and submits authorized requests
```

Port implementations advertise possible mechanisms. Inventory adapters record
which mechanisms are currently usable and the evidence supporting that belief.
The policy overlay records operator-owned constraints and preferences. The
resolver admits a compatible composition, and the materialized realization
binds that composition for execution.

Examples of overlay inputs include:

- whether paid API fallback is permitted and within what budget;
- provider or harness allow/deny rules;
- reviewer-profile preference matrices;
- subscription-first preferences;
- role-specific pins;
- temporary maintenance exclusions; and
- required operator approval before selected transitions.

An overlay revision can make a realization stale, but it is not itself
permission to interrupt an admitted operation or violate its role contract.
Work Engine applies the change through admission at a safe boundary.

## 7. Invalidation consequences by port

The split makes failure consequences more precise.

### Provider invalidation

Quota exhaustion, authentication loss, provider unavailability, model removal,
or protocol incompatibility may invalidate provider-turn capabilities. A
provider-only successor is valid only when the current harness permits the
provider path to be replaced without changing its promised semantics.

### Harness invalidation

A harness failure may affect session continuity, tool ownership, approvals,
sandboxing, context transitions, and evidence. Replacing it therefore usually
requires a broader successor realization and a stronger reconciliation
boundary than provider-only replacement.

### Projection invalidation

A disconnected or replaced UI affects presentation. It ordinarily should not
invalidate provider or harness execution. An approval-dependent operation may
pause because its required approval channel is unavailable, but that dependency
must be explicit rather than inferred from the existence of a UI.

## 8. Evidence and event ownership

Work Engine should define the canonical evidence required to admit and
reconcile operations. A provider or harness event schema is adapter input, not
automatically Work Engine's canonical schema.

Each execution record should identify:

- logical role and operation;
- immutable realization;
- selected port implementations;
- ownership assignments;
- relevant capability observation generations;
- raw or authenticated evidence sources; and
- any authorized omissions or lossy projections.

Operator projections consume this evidence but do not become its sole durable
owner.

## 9. Current App Server precursors

The current App Server implementation contains useful seams but does not yet
implement these three explicit ports.

- `ManifestRoleRuntime` accepts an injected turn-delivery dependency and checks
  role requirements, but the contract remains narrower than a complete
  provider-neutral port.
- executable-generation composition directly constructs
  `CodexAppServerAdapter`, so the principal builder path remains statically
  Codex-bound.
- reviewer runtime adapters demonstrate more than one execution source, while
  the production native-review host still selects a fixed native Claude
  profile.
- Codex capability negotiation is a valuable harness inventory adapter, not a
  global runtime ontology.
- the operator switchboard exposes a bounded command surface, but it does not
  yet own a versioned runtime-selection policy overlay.
- existing operation and context-transition gates provide precursors for safe
  realization replacement boundaries.

These are repository observations, not evidence that the proposed port
contracts or dynamic realization mechanism already exist.

## 10. Candidate migration shape

The split can be introduced incrementally:

1. Specify the minimum closed contract needed by the current provider-turn
   adapters without replacing their execution paths.
2. Describe Codex App Server and native Claude behavior through truthful harness
   capability descriptors.
3. Make ownership explicit in realization and execution evidence.
4. Route existing fixed selections through admission while preserving current
   behavior.
5. Add the versioned operator policy overlay and read-only projection.
6. Permit bounded rematerialization only after safe-boundary and reconciliation
   behavior is demonstrated.
7. Evaluate additional provider, harness, and UI candidates independently.

This ordering is illustrative rather than roadmap authority.

## 11. Invariants

### Work Engine authority

Ports supply mechanisms. Work Engine retains orchestration, lifecycle policy,
admission, workflow truth, and interpretation of operator commands.

### Independent admission

Provider transport, harness execution, and operator presentation are evaluated
and admitted separately even when one product implements all three.

### Stable admitted execution

One admitted operation executes under one immutable realization. Work Engine
does not splice incompatible port implementations into an in-flight operation.

### Truthful capability fidelity

Native mechanisms remain attributable to their provider or harness. Portability
is demonstrated, not declared by naming similar operations alike.

### Single semantic owner

Provider-turn, tool-loop, context-transition, approval, cancellation, evidence,
and projection ownership are explicit. A realization cannot silently create
competing owners.

### Replaceable presentation

No operator client becomes the sole canonical owner of Work Engine state.

### Policy is not capability

The existence of credentials or an implementation does not grant authority to
use it. Preferences cannot make an invalid composition valid, and capability
observations cannot grant spending or workflow authority.

## 12. Non-goals

This idea does not propose:

- a lowest-common-denominator agent API;
- replacing every harness with direct provider calls;
- adopting OpenCode or any other product wholesale;
- making runtime selection an inference-time agent decision;
- making the UI the runtime router;
- permitting mid-operation failover;
- treating capability discovery as policy authority; or
- requiring every role realization to include all three ports.

## 13. Related pending ideas

- [Authority-Backed Architecture Directions as Workflow Inputs](./authority-backed-architecture-directions-as-workflow-inputs.md)
  proposes how this architecture could become an exact, scoped input to intake,
  proposal formation, planning, implementation compilation, and review if it is
  accepted by the appropriate decision owner.
- [Pre-Indexed Capability Cache, Policy Overlay, and Materialized Runtime Realization](./pre-indexed-capability-resolution-and-frozen-runtime-realization.md)
  owns observation, policy, admission, invalidation, and successor-realization
  semantics above these ports.
- [OpenCode Substrate Evaluation](./opencode-substrate-evaluation.md) evaluates
  OpenCode as a candidate implementation source for the provider-turn and
  operator-projection ports, and conditionally for the harness-runtime port.

## Core principle

> Separate provider turns, coding-harness runtime, and operator presentation;
> compose them only through explicit capability, ownership, admission, and
> evidence contracts controlled by Work Engine.
