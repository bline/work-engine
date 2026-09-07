# Idea: Pre-Indexed Capability Cache, Policy Overlay, and Materialized Runtime Realization

**Status:** Architecture idea

**Scope:** Runtime composition, role admission, failover, operator-directed
runtime changes, and executable generations

**Authority:** Exploratory only; this document does not amend the migration
roadmap, admit a runtime, grant budget or provider authority, or authorize
implementation.

## Summary

Work Engine should treat runtime realization as a derived artifact over a
changing capability environment and a versioned operator policy overlay.

Instead of deciding runtime composition repeatedly during execution, permanently
binding a role to its first realization, or predeclaring every possible failover
path, Work Engine should:

1. discover and index available runtime capabilities;
2. record changing capability facts as attributed observations;
3. keep role requirements, operator policy, preferences, and authority distinct;
4. determine which runtime compositions are currently admissible;
5. materialize the admitted composition as an immutable realization;
6. execute each admitted operation under that stable realization;
7. invalidate affected capability facts or policy dependencies when new
   observations or authorized changes make them stale;
8. mark dependent realizations stale without mutating them; and
9. re-evaluate and materialize a successor at the next safe execution boundary.

Conceptually:

```text
role contracts + policy overlay + capability observations + current state
                                |
                                v
                    resolution and admission
                                |
                                v
                 immutable materialized realization
                                |
                                v
                            execution
                                |
                    new observation or policy revision
                                |
                                v
                     targeted invalidation
                                |
                                v
                 safe-boundary successor realization
```

The core principle is:

> Cache admitted decisions, invalidate stale dependencies, and materialize a
> new immutable realization at a safe boundary without treating invalidation as
> authority to change runtime meaning.

The realization is cache-like because its validity is derived and recomputable.
It is also a durable execution and audit artifact: historical identity,
dependencies, admission evidence, use by operations, and invalidation cause
must not disappear merely because the current realization becomes stale.

## Motivation

A heterogeneous Work Engine runtime needs to answer questions such as:

- which provider or harness realizes a role;
- who owns model-turn and tool execution;
- which cancellation mechanism applies;
- how context transitions are performed;
- which approval mechanism is authoritative;
- where lifecycle and raw evidence come from;
- whether required capabilities are presently available; and
- what should happen when observations or operator direction change.

These are host composition, admission, and authority problems rather than
ordinary model-turn inference problems.

The earlier frozen-realization model correctly moved these decisions out of
ordinary role execution. Permanently freezing one concrete realization,
however, pushes adaptability into predefined failover envelopes or repeated
special-case admission paths. Work Engine cannot predict every future runtime,
quota failure, provider change, protocol incompatibility, or operator need.

The better boundary is to freeze each admitted artifact and operation, not its
validity forever. A failed dependency can make an artifact stale without
changing what that artifact meant while it was used.

## 1. Distinct authoritative inputs

A materialized realization is derived from inputs with different owners and
lifetimes. They must not collapse into one runtime configuration object.

```text
Role contract
  requirements, ceilings, authority, continuity
        |
        +-----------------------+
                                |
Operator runtime policy         |
  constraints, preferences,     |
  grants, pins                  |
        |                       |
        +-----------+-----------+
                    |
Capability inventory
  attributed current observations
                    |
Current role and workflow state
  active operation, lifecycle, fences
                    |
                    v
        resolution and admission
```

### Role contract

The role contract defines what must be true for the role to execute. It may
declare semantic requirements, effect ceilings, continuity requirements,
evidence obligations, and authority boundaries without naming Codex, Claude,
OpenCode, Anthropic, OpenRouter, or another implementation.

For example:

```text
Builder requires:
    streaming_inference
    shell_tool_execution
    interruptible_execution
    observable_context_transition
    durable_raw_evidence
```

A realization that cannot satisfy the contract is invalid regardless of
operator preference.

### Operator runtime policy overlay

The operator overlay is a versioned, attributable input that controls the
valid interior left open by role contracts. It is not a direct mutation of an
adapter, thread, provider request, or active realization.

It contains four different kinds of settings:

```text
constraints
    permitted and prohibited providers, transports, review classes,
    continuity changes, evidence custody, tools, and mechanisms

preferences
    ranking among otherwise admissible candidates

authority grants
    bounded permission for cost, provider use, evidence custody, or other
    changes whose owner has delegated that authority

pins
    an instruction to retain a particular realization or route until released,
    invalidated, or made impossible by a stronger contract
```

These categories must remain distinct. "Prefer Claude" is not the same kind of
setting as "never use a paid API," "allow up to $10," or "keep this builder on
Codex."

### Capability inventory

The capability inventory describes what the current environment can actually
provide. It combines relatively stable structural metadata with changing,
attributed runtime observations.

### Current role and workflow state

Resolution must account for active operation identity, context lifecycle,
leases and fences, retained-session state, and reconciliation obligations. A
candidate may be semantically admissible but unavailable until active work has
drained or a successor context has been established.

## 2. Capability inventory as a dependency graph

The inventory should preserve dependencies and semantic differences rather
than flattening them into a feature list.

```text
codex-app-server/0.149.1
    |
    +-- model-access/openai/account-A
    +-- turn-streaming
    +-- turn-interrupt
    +-- shell/harness-native
    +-- approval-routing/codex
    +-- context-transition/new_context

claude-code/runtime-X
    |
    +-- model-access/anthropic/subscription-B
    +-- native-tools
    +-- native-context-lifecycle
    +-- native-session-continuity

anthropic-direct/credential-C
    |
    +-- streaming
    +-- request-cancellation
    +-- context-management
    +-- provider-usage-evidence
```

A realization may depend on nodes from several sources. Dependency identity
must be sufficiently scoped: provider, account or credential, model, endpoint,
workspace, runtime version, transport, and relevant time window may all matter.
A quota observation for one credential must not globally invalidate
`anthropic.access` or `openai.access`.

Dependency edges should state the predicate being relied upon, not merely point
at a vaguely named feature. This permits targeted invalidation and makes
historical admission explainable.

## 3. Capability facts are attributed observations

Work Engine should distinguish at least:

```text
configured
discovered
probed
observed
superseded
invalidated
unknown
```

A capability observation should include enough identity, provenance, scope,
and time information to establish what Work Engine believed and why.

```text
CapabilityObservation

identity:
    codex/openai/account-A/provider-access

state:
    available

generation:
    184

observed_at:
    2026-09-05T21:42:17-06:00

source:
    runtime_handshake

scope:
    runtime = codex-runtime-0.153
    credential = account-A
    model = gpt

evidence:
    ...

validity:
    expires_at = ...
    superseded_by = null
```

Later evidence may supersede it:

```text
state:
    unavailable

generation:
    185

observed_at:
    2026-09-05T22:03:51-06:00

reason:
    quota_exhaustion

supersedes:
    generation 184
```

A failure should first produce an attributed observation. The owner of the
capability semantics or an admitted deterministic policy then determines which
scoped fact that observation supersedes or invalidates. An unexpected failure
must not automatically become a global capability conclusion.

Relevant observations may include:

- quota and rate-limit availability;
- authentication and authorization state;
- provider and model reachability;
- runtime and protocol versions;
- server health;
- tool and workspace access;
- cancellation behavior;
- provider feature availability; and
- malformed, missing, or contradictory evidence.

## 4. Resolution, judgment, and admission

The resolution boundary has two different jobs.

First, deterministic machinery can:

- reject candidates that fail requirements or exceed ceilings;
- apply explicit prohibitions and budget bounds;
- test dependency validity;
- compute the consequences already determined by known inputs; and
- rank candidates when the policy defines a complete, unambiguous ordering.

Second, a decision owner must choose when admissible candidates differ in
meaning that is not fully determined by contract and policy. Examples include
review independence, evidence strength, continuity loss, latency, tool access,
or a new cost/authority tradeoff.

Work Engine should not move those semantic decisions into a deterministic
resolver merely because they can be represented in a table. Depending on the
owning contract, the decision may belong to the supervisor, another authorized
model role, the operator, or the human who owns budget or product authority.

Invalidation does not grant authority. In particular, it cannot by itself:

- increase cost authority;
- weaken an independence requirement;
- change evidence custody;
- change context continuity;
- expand tool or mutation authority; or
- substitute one review class for another.

After a valid decision exists, deterministic machinery materializes and admits
the exact realization.

## 5. Materialized realization

Resolution and admission produce an immutable realization artifact.

```text
RoleRealization: builder-7/realization-f83c1

role_contract:
    builder-v12

policy_overlay_revision:
    runtime-policy-42

capability_inventory_generation:
    381

provider_turn:
    owner = codex_harness

harness_runtime:
    implementation = codex_app_server

tools:
    owner = codex_harness

context_transition:
    implementation = codex.new_context

cancellation:
    implementation = codex.turn.interrupt

approval:
    implementation = codex.approval

raw_evidence:
    source = codex_app_server_events

dependencies:
    codex.runtime              @ generation 31
    openai.account-A.access    @ generation 184
    codex.new_context          @ generation 7
    codex.shell                @ generation 12

decision:
    owner = slice-supervisor
    authority = ...
    evidence = ...
```

The receipt should bind at least:

- role, role-instance, and execution-generation identity;
- requirements and contract identity;
- policy-overlay revision and applicable scope;
- capability-inventory generation and exact dependency observations;
- selected provider, harness, model, transport, and mechanism identities;
- tool, approval, cancellation, context-transition, and evidence ownership;
- unsupported or explicitly omitted capabilities;
- decision owner and authority evidence where judgment was required;
- safe-boundary admission evidence; and
- configuration and source revisions needed for reconciliation.

The artifact itself never mutates. It remains valid, becomes stale, or becomes
historical.

## 6. Invalidation

When an authoritative dependency changes, Work Engine marks dependent
realizations stale.

```text
provider quota observation
        |
        v
account-scoped provider access superseded
        |
        v
dependent realizations become stale
```

Dependency propagation determines the affected set:

```text
openai.account-A.access INVALID
        |
        +-- reviewer-2 realization STALE
        +-- builder-7 realization STALE
        +-- supervisor-3 unaffected
```

Staleness means the realization may not be silently reused for a newly
admitted operation. It does not rewrite the historical artifact and does not
necessarily interrupt an in-flight operation.

The response to invalidation depends on the failed property:

- an operation may drain when its already-admitted semantics and safety remain
  trustworthy;
- it may need interruption when continuing would violate a contract or consume
  invalid authority;
- it may need reconciliation when completion, cancellation, or evidence is
  uncertain; and
- subsequent operations remain paused until a valid successor is admitted.

These consequences should be derived from owned contracts and current state,
not from an exhaustive table of anticipated failure names.

## 7. Rematerialization instead of a predefined failover graph

Failover is an ordinary consequence of invalidation and renewed admission.

```text
stale realization
        |
        v
current role requirements
        +
current policy overlay
        +
current capability inventory
        +
current role/workflow state
        |
        v
admissible candidates and consequences
        |
        v
determined policy result or authorized decision
        |
        v
new immutable realization
```

The successor might use Claude Code, OpenRouter, direct Anthropic, another
Codex provider, a newly enabled runtime, or nothing at all. Work Engine does
not need to predict that outcome when admitting the original realization.

If no candidate satisfies the requirements and authority, the role remains
paused with an explainable unresolved state.

A bounded composite runtime may still be admitted when it is genuinely one
mechanism with closed semantics, authority, and evidence behavior. Composite
failover is therefore possible, but it is not the foundational recovery model
and must not be used to smuggle future semantic choices into an apparently
frozen artifact.

## 8. Operator runtime policy overlay

The operator overlay provides the manipulable control surface required for
runtime direction without making UI state canonical runtime truth.

### Scoping and precedence

Policies may be scoped to:

```text
global
  -> workflow or campaign
      -> role class
          -> role instance or review obligation
              -> one operation
```

A narrower preference may override a broader preference. It may not override a
role contract, a stronger prohibition, an authority boundary, or an unavailable
capability. Precedence, inheritance, expiry, and conflict behavior must be
explicit and deterministic.

### Example policy

```yaml
scope:
  campaign: migration-s14
  role_class: implementation-reviewer

constraints:
  minimum_review_class: independent_external
  require_repository_read: true
  prohibit:
    - same_model_for_independence_claim

preferences:
  routes:
    - claude_subscription
    - anthropic_paid_live
    - openrouter_claude

authority:
  paid_api:
    maximum_usd_per_operation: 10
    maximum_usd_per_campaign: 50
    expires_at: 2026-09-10T00:00:00Z

failover:
  quota_exhausted:
    action: reconsider
    candidates:
      - anthropic_paid_live
      - openrouter_claude
  provider_unavailable:
    action: pause_for_operator

pins:
  reviewer-2:
    route: claude_subscription
    until: explicitly_released
```

The `failover` section does not pre-admit a concrete successor graph. It
constrains or suggests the candidate space and the required decision behavior
after invalidation.

### Reviewer-choice matrix

Reviewer alternatives require a matrix because a provider order alone loses
material semantic dimensions.

| Candidate | Trigger | Review class | Independence | Continuity | Tools | Cost | Disposition |
|---|---|---|---|---|---|---|---|
| Claude subscription | normal | external review | preserved | retained | native | included | preferred |
| Anthropic paid | quota exhausted | external review | preserved | reconstructed | bounded | <= $10 | allowed within grant |
| OpenRouter Claude | provider unavailable | external-provider claim uncertain | changed | fresh | limited | <= $5 | requires approval |
| Codex same-model | external reviewers unavailable | same-model review | weakened | fresh | native | included | prohibited for this obligation |

A UI may project states such as:

```text
selected
preferred
admissible
admissible_within_grant
requires_operator_approval
requires_pause_and_readmission
temporarily_unavailable
prohibited
```

The matrix is an operator projection over owned contracts, policy, capability
observations, and consequences. It is not itself the resolver or source of
authority.

### Operator change lifecycle

```text
operator submits policy revision
        |
        v
validate identity and authority
        |
        v
publish immutable overlay revision
        |
        v
identify dependent realizations
        |
        v
mark affected realizations stale
        |
        v
drain, interrupt, or reconcile at the owned safe boundary
        |
        v
resolve and admit a successor
```

Rejected changes should distinguish missing authority, unsatisfied
requirements, prohibition, unavailable capability, exhausted budget, policy
conflict, and active work that cannot yet be reconciled.

## 9. Safe execution boundaries

Dynamic resolution must not mean mid-operation mutation.

A provider turn, tool operation, lifecycle transition, native review episode,
or other admitted operation executes under one stable realization.

```text
operation executing under realization A
        |
runtime or policy change observed
        |
operation drains, terminates, or is reconciled under A
        |
dependent facts and A marked stale
        |
return to safe Work Engine boundary
        |
realization B admitted
        |
next operation executes under B
```

Work Engine must not splice incompatible runtimes into one logical operation.
Switching provider or harness may also require a successor context or execution
identity rather than reuse of the prior provider thread.

The realization scope should therefore distinguish:

- role-instance lineage;
- executable generation;
- retained context or runtime session;
- admitted operation; and
- the active binding between them.

An operation always records the exact realization it used. A role may acquire
a successor realization only through an admitted binding transition.

## 10. Realization identity and evidence

Every execution record should identify the realization under which it occurred.

```text
role: builder-7
operation: turn-18
realization: realization-f83c1
policy_overlay_revision: runtime-policy-42
capability_inventory_generation: 381
```

After rematerialization:

```text
role: builder-7
operation: turn-19
realization: realization-a912e
policy_overlay_revision: runtime-policy-43
capability_inventory_generation: 385
predecessor_realization: realization-f83c1
transition_reason: openai.account-A.access invalidated
```

Reconciliation should be able to reconstruct:

- which runtime and mechanisms were used;
- what requirements and policy applied;
- what capabilities Work Engine believed existed;
- what evidence supported those beliefs;
- which observation or policy revision made the realization stale;
- who owned any required successor decision;
- what authority supported the new choice; and
- why the new realization was admitted.

## 11. Runtime conditions remain dynamic

Materialize into a realization:

- provider, model, transport, and harness identity;
- tool ownership;
- context-transition mechanism;
- cancellation mechanism;
- approval mechanism;
- evidence source and custody;
- provider options;
- required capability satisfaction;
- applicable policy revision; and
- the authority and decision that selected the composition.

Continuously observe:

- quota and rate limits;
- authentication and provider availability;
- model availability;
- context pressure;
- server and transport health;
- tool failure;
- cancellation results;
- leases and fence state; and
- malformed, missing, or contradictory evidence.

Observations may invalidate realization dependencies. They do not mutate the
realization or authorize its successor.

## 12. Relationship to existing Work Engine boundaries

The current App Server implementation contains partial precursors, not this
complete architecture.

### Runtime manifests and compiled role environments

The runtime manifest and compiler outputs own role requirements, capability and
effect ceilings, static role environment, and deploy-time inputs. They should
feed resolution without becoming the live capability-observation store.

### Capability negotiation

Pinned Codex capability negotiation is one inventory adapter and admission
check. It should not become the global capability ontology.

### Role binding registry

The role binding registry can bind a logical role instance to the active
realization, provider thread, protocol version, and environment identity. It
should reference realization identity rather than own policy or capability
truth.

### Context-transition and operation admission gates

Existing gates already protect important safe boundaries. Realization changes
must compose with those gates so a binding cannot change during an in-flight
context transition or operation.

### Agent Environment Graph

The canonical Agent Environment Graph describes role contracts and effective
configured environments. Its current baseline intentionally omits the live
overlay. The dynamic capability inventory may project into an operator or
analysis view associated with that graph, but it must not mutate generated
views or checked-in baseline configuration into runtime truth.

### Runtime ports

This mechanism complements the independently replaceable ports defined in
[Provider Turn, Harness Runtime, and Operator Projection](./provider-turn-harness-runtime-and-operator-projection.md):

```text
Work Engine authority and durable services
        |
        +-- ProviderTurnPort
        +-- HarnessRuntimePort
        +-- OperatorProjection
```

The ports expose possible mechanisms. Inventory adapters observe which
mechanisms are currently usable. Resolution and admission select a valid
composition. The realization binds that selection for execution. The operator
projection exposes state and accepts policy-change requests without becoming
the owner of workflow or runtime truth.

OpenCode, Codex, Claude, Anthropic, OpenRouter, and future runtimes remain
capability and implementation sources rather than Work Engine's runtime
ontology.

## 13. Relationship to the operator UI

The UI should project both current realization state and the policy and
capability facts relevant to operator judgment.

```text
builder-7

Current realization:
    Codex App Server / GPT / native shell / new_context

Status:
    valid

Policy:
    prefer subscription runtimes
    paid API permitted up to $10 per operation

Dependencies:
    Codex runtime              available
    account-A model access     available
    shell                      available
    new_context                available
```

After invalidation:

```text
builder-7

Current realization:
    STALE

Reason:
    account-A provider access superseded
    quota exhausted at 22:03:51

Successor candidates:
    Claude subscription        preferred, available
    Anthropic paid             allowed within grant
    OpenRouter                 requires operator approval

State:
    waiting at safe rematerialization boundary
```

The UI may edit policy overlays, release pins, grant bounded authority, or
request re-evaluation. It must not directly mutate the active adapter, provider
thread, runtime session, or realization.

## 14. Benefits

- Keeps provider and harness composition outside ordinary role inference.
- Avoids predicting every future failure and replacement route.
- Preserves one immutable realization for each admitted operation.
- Makes operator direction explicit, scoped, attributable, and revisable.
- Retains human and workflow authority across failover.
- Supports unknown failures through observations and targeted invalidation.
- Makes historical runtime changes reproducible and explainable.
- Invalidates only realizations that depend on a changed fact or policy input.
- Preserves provider- and harness-native capabilities without false
  equivalence.
- Fits Work Engine's route-revision model: preserve valid evidence, retire stale
  conclusions, and recompute only what changed.

## 15. Non-goals

This idea does not:

- authorize OpenCode or any other runtime;
- grant paid API authority;
- require all capabilities to share a lowest-common-denominator interface;
- make availability observations permanently true;
- make a UI, matrix, or settings form canonical workflow state;
- permit silent provider, harness, model, tool, lifecycle, approval, evidence,
  continuity, or review-class substitution;
- require every failure to have a predefined route;
- imply that invalidation always interrupts active work;
- make deterministic resolution the owner of semantic judgment; or
- amend the active skills-migration or S13 boundary.

## 16. Questions for proposal formation

1. Which service owns capability-observation identity, supersession, expiry,
   and dependency propagation?
2. Which realization semantics belong per role instance, executable generation,
   retained session, and admitted operation?
3. What exact conditions allow an in-flight operation to drain after a
   dependency becomes stale, and which require interruption or reconciliation?
4. Which resolution results are fully determined by policy, and which require
   supervisor, operator, or human judgment?
5. How are policy-overlay scopes, precedence, expiry, conflict, and authority
   references represented?
6. Which changes are preferences, constraints, authority grants, and pins, and
   which owners may publish each kind?
7. How are budget consumption and remaining authority reconciled atomically
   across concurrent realizations?
8. How are reviewer alternatives compared without flattening independence,
   continuity, tool access, latency, evidence strength, and review class?
9. Which capability observations require probes, authenticated runtime
   evidence, expiration, or manual operator attestation?
10. How does the role binding registry reference successor realizations while
    preserving provider-thread and context lineage?
11. Which realization and invalidation evidence belongs in operation receipts,
    durable lifecycle state, and operator projections?
12. What bounded composite mechanisms are legitimately one realization rather
    than hidden future admissions?
13. How should a pin behave when its target becomes unavailable, prohibited, or
    semantically insufficient?
14. What initial vertical proves invalidation, safe-boundary transition, and
    successor admission without requiring multiple live paid providers?

## Core principle

> Work Engine should pre-index runtime capabilities, maintain a versioned
> operator policy overlay, materialize each admitted runtime composition as an
> immutable durable artifact, invalidate its continued use when authoritative
> dependencies change, and admit a successor from current requirements, policy,
> authority, observations, and state at a safe execution boundary.

Failover is therefore not primarily a special routing graph. It is truthful
observation, targeted invalidation, renewed judgment where meaning remains
open, deterministic materialization where the result is known, and explicit
admission of a successor realization.
