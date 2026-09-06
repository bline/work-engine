# Idea: Pre-Indexed Capability Resolution and Frozen Runtime Realization

**Status:** Architecture idea

**Scope:** Runtime composition, role admission, and executable generations

**Authority:** Exploratory only; this document does not amend the migration
roadmap, admit a runtime, or authorize implementation.

## Summary

Work Engine should avoid repeatedly deciding runtime composition while a role
is executing. It should instead:

1. discover and index available provider, harness, tool, approval, evidence,
   context-transition, cancellation, and operator-projection capabilities;
2. resolve a role's semantic requirements against that inventory when the
   relevant execution identity is admitted;
3. inject the selected concrete mechanisms;
4. publish an attributed realization receipt; and
5. freeze that realization for the admitted role or executable generation.

Runtime conditions remain observable and dynamic. Runtime meaning and
mechanism ownership do not silently drift after admission.

The core principle is:

> Discover capabilities globally, resolve requirements at admission, inject
> concrete mechanisms, and freeze the resulting realization before execution.

## Motivation

A heterogeneous runtime can otherwise force execution-time reasoning about
questions the host can answer deterministically:

- Which component owns tool execution?
- Which cancellation mechanism applies?
- Which context-transition mechanism realizes the requested lifecycle meaning?
- Which approval protocol applies?
- Where does authoritative raw evidence come from?
- Is this role admissible on this runtime?
- Which provider- or harness-native capabilities are actually available?

These are composition and authority questions, not ordinary inference tasks.
Resolving them before execution improves reproducibility, reconciliation, and
failure behavior while reducing agent context and repeated judgment.

## Capability inventory

At host or executable-generation startup, Work Engine can build an attributed
inventory of available mechanisms. The inventory should preserve semantic
differences rather than flattening capabilities into false equivalence.

For example:

```text
CodexAppServer:
    inference
    streaming
    tool_execution = harness_native
    cancellation = turn_interrupt
    context_transition = new_context
    approval_protocol = codex
    raw_evidence = codex_events

ClaudeCode:
    inference
    tool_execution = harness_native
    cancellation = claude_native
    context_transition = claude_native
    raw_evidence = claude_transport_and_events

AnthropicDirect:
    inference
    streaming
    tool_execution = provider
    cancellation = request_abort
    context_editing = context_management
    raw_evidence = provider_events

OpenRouterProviderTurn:
    inference
    streaming
    cancellation = request_abort
    usage_observation
```

Inventory entries need provenance, version or generation identity, capability
semantics, relevant restrictions, and enough evidence to distinguish observed
support from configured or merely assumed support.

## Semantic role requirements

Roles should declare requirements without selecting a provider or harness.

```text
Builder requires:
    streaming_inference
    shell_tool_execution
    interruptible_execution
    observable_context_transition
    durable_raw_evidence

Reviewer requires:
    streaming_inference
    structured_output
    usage_observation
```

Role requirements may also express capability and effect ceilings. A concrete
runtime is admissible only if it satisfies the required semantics without
granting prohibited authority or claiming unsupported equivalence.

## Admission-time resolution

Resolve a role as soon as its execution identity exists and before it may
execute. The capability inventory may be shared, while each admitted role has
an immutable local realization.

```text
RoleRealization: builder-3

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

operator_projection:
    owner = work_engine
```

Admission fails closed when the inventory cannot satisfy the role's complete
requirements. The realization receipt should bind at least:

- role and execution-generation identity;
- requirements identity;
- capability-inventory identity;
- selected provider, harness, and mechanism identities;
- tool, approval, cancellation, context-transition, and evidence ownership;
- admitted fallback policy, if any;
- unsupported or explicitly omitted capabilities; and
- the configuration and source revisions needed for reconciliation.

## Freeze and invalidation

Once admitted, execution uses the frozen realization rather than reselecting
mechanisms turn by turn.

```text
discover -> index -> resolve -> admit -> inject -> freeze -> execute
```

If a required capability disappears, becomes invalid, or can no longer satisfy
its admitted semantics, Work Engine must not silently rebind the role:

```text
capability invalidation
        -> pause or fail closed
        -> reconcile current work and evidence
        -> new admission decision
        -> new frozen realization
```

A new realization is a governed lifecycle event, not an invisible retry.

## Static semantics and dynamic conditions

Freezing a realization does not assert that the environment is static.

Resolve before execution:

- provider and harness selection;
- tool ownership;
- cancellation mechanism;
- context-transition mechanism;
- approval mechanism;
- evidence source and custody;
- provider- and harness-capability requirements; and
- any admitted failover envelope.

Observe during execution:

- provider availability, quota, and rate limits;
- token and context pressure;
- tool and cancellation outcomes;
- workspace leases and fencing state;
- malformed, missing, or contradictory evidence; and
- runtime or transport failure.

Dynamic observations may invalidate a realization. They do not themselves
authorize a different realization.

## Failover is part of the admitted meaning

Quota or transport failover needs an explicit interpretation under frozen
realization.

One valid design is to admit a composite mechanism whose ordered routes,
transition conditions, semantic equivalence requirements, evidence custody,
and cost authority are frozen together. For example, a reviewer realization
might admit Claude subscription execution followed by a bounded paid Anthropic
fallback only when both routes preserve the required review semantics.

Another valid design is to pause and require a new admission before changing
routes. The host must choose one of these meanings explicitly. A runtime error
must never silently expand the admitted fallback set or change harness,
provider, context, tool, approval, or evidence semantics.

## Runtime preferences and operator overrides

The resolver needs a configuration layer between semantic requirements and a
frozen realization. That layer expresses how an authorized operator prefers to
rank otherwise admissible choices; it does not weaken requirements or create
capabilities.

Keep three inputs distinct:

```text
role requirements
    what every valid realization must preserve

runtime preferences
    how to rank realizations that satisfy those requirements

operator override request
    an authorized request to resolve again under different preferences
```

For example:

```yaml
runtime_preferences:
  provider_order:
    - claude_subscription
    - anthropic_paid_live
    - anthropic_paid_batch
  cost:
    paid_api: allow_after_subscription_exhaustion
    maximum_usd_per_operation: 5.00
  latency:
    interactive_review_requires_live_tools: true
    maximum_batch_wait_minutes: 30
  continuity:
    prefer_retained_session: true
    context_reconstruction: require_explicit_admission
  review:
    preferred_classes:
      - independent_external
      - fresh_same_model
    prohibited_substitutions:
      - deterministic_gates_as_semantic_review
```

The exact schema remains unresolved. Its contract should preserve these
distinctions:

- A preference ranks valid candidates; it cannot make an invalid candidate
  admissible.
- Cost-bearing routes require the human or policy authority that owns the
  budget.
- Provider, model, live/batch transport, harness, tool access, continuity, and
  review class are separate dimensions even when a convenient adapter bundles
  them.
- Selecting a different review class may change the strength or meaning of the
  resulting review claim. It is not always an equivalent runtime fallback.
- A fallback envelope may contain only alternatives whose semantic effects and
  evidence consequences are explicitly admitted.
- An override against active work produces a pause, reconciliation, and new
  realization unless the requested change was already a bounded knob inside
  the frozen realization.
- Increasing cost authority, weakening independence, changing evidence
  custody, or changing continuity cannot be treated as an ordinary live knob.

An operator override should therefore be modeled as a request, not a direct
mutation:

```text
override requested
       -> validate operator authority
       -> evaluate role requirements
       -> show consequences and admissible candidates
       -> pause or drain affected work when necessary
       -> publish a new realization receipt
       -> resume under the new realization
```

The old and new realization identities remain visible in lifecycle evidence.
Rejected overrides should report whether the cause was missing authority,
unsatisfied requirements, unavailable capability, budget, or an active-work
transition that could not be reconciled.

### UI projection

Any operator UI should receive a Work Engine-owned projection of choices, not
raw provider controls that bypass admission. For each option, the projection
can distinguish:

```text
selected
admissible
admissible_after_pause_and_readmission
unavailable
prohibited
```

It should also project the consequences needed for operator judgment:

- expected cost and the authority covering it;
- expected latency or batch delay;
- provider, model, harness, and transport identity;
- session or context-continuity consequences;
- tool and approval availability;
- review class and independence consequences;
- raw-evidence source and custody; and
- the runtime observation that made an alternative relevant, such as quota
  exhaustion.

For example, an OpenCode or Codex operator surface might render:

```text
Current realization
  Claude subscription / native harness / retained session

On quota exhaustion
  1. Pause and wait for reset                 admissible
  2. Anthropic paid live / native tools       admissible, up to $5
  3. Anthropic batch / reconstructed context  requires readmission
  4. Same-model Codex review                  prohibited by independence requirement
```

The UI may let the operator edit preference profiles or request an override.
Work Engine remains responsible for authority validation, resolution,
admission, freezing, and evidence. This keeps the preference UI replaceable
and prevents presentation state from becoming runtime truth.

## Relationship to runtime ports

This mechanism complements the three independently replaceable ports described
in
[OpenCode as a Provider, Runtime, and Operator-Interface Substrate](./opencode-as-a-provider-runtime-and-operator-interface-substrate.md):

```text
Work Engine authority and durable services
        |
        +-- ProviderTurnPort
        +-- HarnessRuntimePort
        +-- OperatorProjection
```

The capability inventory describes implementations available through these
ports. Admission resolves a role's requirements into one concrete composition,
and execution receives that frozen composition.

The ports still require explicit contracts for capability discovery, event
identity, cancellation, raw-evidence custody, reconciliation, and unsupported
operations. A capability name alone is not a semantic contract.

## Relationship to operator control and replaceable UIs

Operator commands should express Work Engine meaning rather than expose the
selected harness mechanism:

```text
operator: interrupt slice-supervisor:migration
        -> Work Engine OperatorControl
        -> frozen role realization
        -> codex.turn.interrupt
```

A Codex TUI, OpenCode TUI, external control CLI, or future UI should see that
an admitted role is interruptible. It should not need to know or select the
concrete interruption mechanism. Detach, interrupt, pause, resume, workflow
closure, and server shutdown therefore remain Work Engine command semantics
even when their realization involves a harness-specific operation.

This makes the planned out-of-band control-plane repair a useful first vertical
through the larger architecture rather than a Codex-TUI-specific patch.

## Benefits

- Removes recurring inference-time composition decisions.
- Makes execution reproducible from an attributed realization receipt.
- Lets reconciliation interpret observations against the exact admitted
  mechanisms.
- Prevents silent semantic drift during quota, availability, or runtime change.
- Preserves provider- and harness-native capabilities without pretending they
  are interchangeable.
- Supports heterogeneous roles within one workflow.
- Simplifies execution code and operator projections by injecting resolved
  dependencies.

## Existing alignment and missing work

Work Engine already has partial precursors:

- pinned capability negotiation;
- verified runtime-requirement satisfaction against role grants and ceilings;
- role projections carrying capability, effect, continuity, and environment
  revisions; and
- immutable content-addressed executable-generation snapshots.

This idea generalizes those mechanisms. It does not claim that Work Engine
already has a complete multi-runtime capability inventory, semantic resolver,
immutable `RoleRealization` receipt, invalidation protocol, or composite
failover contract.

## Non-goals

This idea does not:

- authorize OpenCode or any other runtime;
- require all capabilities to share one lowest-common-denominator interface;
- make runtime availability static;
- allow a UI to become canonical workflow or context state;
- permit silent provider, harness, tool, lifecycle, approval, or evidence
  substitution; or
- amend the active skills-migration or S13 boundary.

## Questions for proposal formation

1. Is realization frozen per role instance, per executable generation, per
   admitted operation, or through a precisely defined combination of these?
2. Which capability facts are configuration claims, startup observations, or
   continuously attested runtime facts?
3. What invalidates a realization, and which active work may drain after
   invalidation?
4. When is a composite failover policy semantically equivalent enough to be one
   realization rather than a new admission?
5. Which authority admits cost-bearing fallbacks and changes to evidence
   custody?
6. How are realization revisions projected to operators without leaking
   provider-specific implementation into the UI contract?
7. Which parts belong to executable-generation bootstrap, runtime manifests,
   role binding state, and durable lifecycle evidence?
8. Which preferences may be scoped globally, by workflow, by role class, by
   role instance, or by one operation, and how is precedence made explicit?
9. Which settings are bounded live knobs inside one realization, and which
   necessarily create a new realization?
10. How should the resolver compare review classes whose independence,
    continuity, tool access, latency, cost, and evidentiary claims differ?
