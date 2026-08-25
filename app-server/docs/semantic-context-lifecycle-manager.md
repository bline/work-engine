# Semantic Context Lifecycle Manager

## Status

Planning document for the Work Engine App Server runtime.

This document proposes a shared code-owned context-lifecycle service for every
independently controlled Work Engine role thread. It is not a claim that the
required provider capabilities are implemented or stable in the currently
pinned Codex release.

The plan preserves the semantic model developed in
[`ideas/01-semantic-context-manager-proposal.md`](../../ideas/01-semantic-context-manager-proposal.md)
while revising its execution architecture. Context management becomes common
runtime infrastructure rather than a durable Wind Walker peer role or a skill
loaded into ordinary task agents.

## Architectural decision

The App Server runtime should provide one horizontal semantic context-lifecycle
plane above provider mechanics and below Work Engine roles:

```text
Work Engine roles and workflows
────────────────────────────────
Semantic Context Lifecycle Manager
  observation
  compilation
  verification
  checkpointing
  retirement
  rehydration
  reconciliation
────────────────────────────────
Provider-neutral runtime adapter
────────────────────────────────
Codex App Server daemon
```

Ordinary roles should perform domain work without monitoring their own token
pressure, inventorying continuation meaning, or deciding how to persist it.
The shared service should apply the same lifecycle contract to every Work
Engine role and expose role-specific extension points only where the role owns
additional typed state.

The first supported subjects should be daemon-owned root threads. App
Server's parent-owned Codex subagents have different direct-control boundaries;
their lifecycle may need to remain parent-mediated rather than being treated as
independently controllable Work Engine roles.

## Ownership boundary

The lifecycle manager owns:

- thread and turn observation;
- token, context-window, timing, and cost telemetry;
- bounded model-visible history projections;
- inspection scheduling;
- semantic-compilation and verification invocations;
- continuation-checkpoint storage and revision fencing;
- transition readiness and actuation coordination;
- transition classification;
- rehydration delivery and reconciliation evidence; and
- the external lifecycle ledger.

It does not own:

- a role's objective, domain truth, or workflow authority;
- user approval, preference, or unresolved human intent;
- canonical claims, decisions, receipts, schedules, or repository state;
- authority grants merely described by a checkpoint;
- the exact hidden model context assembled inside Codex Core; or
- provider transition semantics inferred from an event name alone.

A compiled continuation checkpoint is a runtime-owned rehydration artifact. It
may preserve and reference canonical meaning, but it is not a replacement owner
for that meaning.

## Runtime components

### Telemetry collector

The collector consumes provider events and host measurements, including:

- thread and turn identity and status;
- completed turns and their visible items;
- token-usage totals, last-call usage, and reported model context limit;
- tool calls and results;
- context-compaction items and event ordering;
- lifecycle boundaries and expected next work; and
- compilation, verification, retirement, and rehydration timings.

Cheap mechanical observation may be continuous. Semantic compilation should be
bounded and selected only when it has credible lifecycle value.

### Observed-context projector

App Server can return persisted turns and items, but that history is not the
literal effective prompt sent to the model. The projector must therefore emit
an attributed observation with explicit unknowns rather than claiming an exact
context snapshot.

```yaml
observed_context:
  logical_role_instance: strategic-planner:main
  runtime_binding:
    thread_id: "..."
    binding_revision: 4
  source_revision: "sha256:..."
  last_completed_turn: "..."
  visible_items: []
  governing_sources: []
  activated_skills: []
  token_usage:
    total: 0
    last: 0
    reported_context_limit: null
  unknowns:
    - exact_effective_model_input
    - provider_internal_instructions
    - hidden_reasoning_state
```

The source revision must bind every input on which a retirement decision
depends: visible items, governing instructions, skill revisions, durable state,
role identity, runtime binding, lifecycle position, and expected next work.

### Transport security and projection trust

The observed-context projection crosses a security boundary. Content does not
acquire governing authority merely because it appears in model-visible context
or describes itself as an instruction. The host-owned projector should
preserve the identity, origin, trust class, instruction applicability,
producer attribution, content reference, and integrity evidence of each item:

```yaml
visible_items:
  - identity: thread-item-123
    origin: user
    trust_class: human_authority_input
    instruction_applicability: contract_defined
    content_ref: "..."

  - identity: thread-item-124
    origin: retrieved_web_content
    trust_class: untrusted_data
    instruction_applicability: none
    content_ref: "..."

  - identity: thread-item-125
    origin: tool_result
    trust_class: attributed_evidence
    instruction_applicability: none
    producer: codebase-memory
    content_ref: "..."
```

The compiler and verifier should receive only the least privilege needed for
their bounded inference. Snapshot construction must apply confidentiality and
secret filtering, destination-specific read and write authorization, and
retention limits before content is copied. Compiler output must not write to a
canonical owner merely because it names that owner; publication uses the
destination owner's authorization boundary.

A content digest binds projected bytes but does not authenticate the projector
or prove that required source material was not omitted. The canonical
projection should therefore be constructed by an authenticated host component,
record its construction identity and completeness limitations, and anchor the
revision in an append-only or otherwise tamper-evident lifecycle ledger. The
semantic verifier challenges the projection and its attribution; it does not
replace those transport-security controls.

### Semantic compiler

The compiler is a bounded inference capability, not a durable peer role. It
receives the observed-context revision plus current canonical references and
emits a versioned continuation candidate. Its inference context is disposable;
only its attributed output and evidence references are durable.

The compiler should identify the semantic difference between meaning required
for continuation and meaning already durably owned elsewhere. The common
extraction vocabulary includes:

- objective and logical progression;
- current work position;
- completed consequences;
- active commitments and obligations;
- decisions, premises, and invalidation conditions;
- boundaries and authority dependencies;
- evidence and its current interpretation;
- uncertainty and unresolved questions;
- dependencies, identity, and relationships;
- valid, falsified, and superseded routes;
- temporal applicability;
- governing instructions and activated skill state;
- human interactions and their semantic closure; and
- the authorized next action.

Roles may contribute a typed `role_state` extension. A role-specific extension
does not transfer ownership of the shared lifecycle machinery to that role.

### Semantic verifier

A separate bounded verification pass should attempt to falsify the candidate's
sufficiency, attribution, authority preservation, interaction closure, and
source binding. The verifier should return blockers and uncertainty rather than
repairing ambiguity into apparent readiness.

Verification reduces semantic omission risk; it cannot establish access to
hidden provider context. Retirement must remain fail-closed when the available
projection is insufficient for a consequential continuation claim.

### Lifecycle ledger

The ledger is the authoritative runtime projection of context lifecycle. It
should record:

- logical role and current runtime binding;
- observed source revision;
- token pressure and inspection reason;
- compiler and verifier identities and versions;
- candidate and accepted checkpoint revisions;
- unresolved blockers;
- transition readiness and authorization;
- model-actuation request and observed tool call;
- prior and successor context-window evidence;
- transition classification;
- rehydration acknowledgement; and
- reconciliation, repair, or uncertainty.

The ledger must not claim a transition, checkpoint publication, or successful
rehydration merely because it was attempted.

## Compiled continuation state

The universal continuation schema should remain compact while preserving exact
source references for authoritative or ambiguous meaning.

```yaml
schema_version: 1

subject:
  logical_role_instance: strategic-planner:main
  runtime_binding_revision: 4
  source_revision: "sha256:..."
  compiled_at: "2026-08-25T00:00:00Z"

objective:
  statement: "Keep the Work Engine roadmap coherent"
  authority_ref: "user-message:..."

work_position:
  phase: implementation
  current_unit: planner-evidence-delivery-remediation

completed_consequences: []
active_commitments: []
decisions: []

human_interactions: []

authority_dependencies:
  canonical_records: []
  revalidation_required: []

unresolved: []

governing_environment:
  role_contract: "..."
  instructions_to_reload: []
  activated_skills: []

canonical_references: []

authorized_next_action:
  kind: remediate
  objective: "Deliver planner evidence through a supported model-visible input"

role_state:
  schema: strategic-planner-continuation-v1
  value: {}

uncertainty: []
```

The state is compiled meaning, not a transcript summary. Exploratory reasoning,
closed conversation, and raw tool output should be omitted unless legitimate
continuation depends on them.

## Human interaction compilation

Human-authored interaction carries authority and intent that must not be closed
merely to make retirement convenient. The compiler should classify interaction
state separately from its next-context loading disposition.

```yaml
human_interactions:
  - id: "user-message:..."
    kind: implementation_authorization
    status: closed_but_active
    authority_effect: "authorizes changes under app-server/"
    durable_consequence_ref: "authority-record:..."
    source_ref: "thread-item:..."
    next_context_disposition: compiled_consequence

  - id: "user-message:..."
    kind: clarification
    status: open
    source_ref: "thread-item:..."
    next_context_disposition: exact
```

Candidate statuses are:

- `open`: a response, action, choice, clarification, or acknowledgement remains;
- `resolved`: the requested consequence was established;
- `closed_but_active`: the interaction is finished but its decision still governs;
- `superseded`: later human authority explicitly replaced it;
- `historical`: it no longer affects legitimate continuation; and
- `ambiguous`: meaning or closure cannot safely be inferred.

Candidate loading dispositions are:

- `exact`: load the attributed original text;
- `compiled_consequence`: load the durable consequence and its source;
- `reference_only`: retain a resolvable source without loading its text;
- `omit_from_working_context`: retain it in history but not the new window; and
- `escalate`: block retirement or seek human clarification.

Exclusion from the next model context is not deletion, resolution, or loss of
authority. The original interaction remains durably addressable. An assistant
response alone does not prove completion of an implementation request, an
approval decision, or another interaction whose consequence remains pending.

## Retirement and `new_context` actuation

Preservation and retirement are independent decisions. Rising pressure may
justify compilation while retention remains preferable. Retirement should occur
only when the accepted checkpoint is sufficient and replacing the context is
semantically safe and economically advantageous.

The currently observed `new_context` capability is model-invoked,
undocumented, and feature-gated. The host cannot presently treat it as a stable
direct App Server request. Therefore the target model remains a narrow
transition actuator even though every other lifecycle responsibility belongs to
the code-owned service.

The current mechanism hypothesis is:

```text
observe and bind revision C
→ compile and verify C
→ atomically publish checkpoint H
→ verify C still governs the domain state
→ start a semantically sterile retirement control turn
→ target model calls new_context as its only action
→ host observes and classifies the transition
→ host supplies H to the fresh window
→ successor reconciles against H and canonical references
```

The target model needs one small model-facing contract:

> When the runtime supplies a `context_retirement_ready` directive and no later
> human input is present, perform no domain work and invoke `new_context` as the
> only action. If later human input is present or domain work has already
> occurred after the directive, do not invoke `new_context`.

The lifecycle service, not the target model, owns validation that the directive
is revision-bound, published, unblocked, and still current before delivery.

This exact mechanism is not permanent doctrine. If a supported host-callable
fresh-context capability becomes available, the adapter may replace the
model-actuated route while preserving the readiness, fencing, authority, and
rehydration invariants.

The retirement control turn must be sterile. Any new domain output, tool effect,
human input, or continuation-relevant meaning invalidates the readiness subject
and requires a new compilation. The lifecycle ledger should retain the accepted
checkpoint and predecessor evidence until successor reconciliation succeeds.

Final revision comparison and actuator delivery must occur under one
revision-bound host transition lease. While that lease is active, new domain
turns and tool effects cannot enter the retiring revision, the runtime binding
cannot change, and only the sterile actuator turn may proceed. Human input must
either revoke the lease before actuation or remain outside the retiring
revision for explicit successor reconciliation; it cannot race silently with
the actuator turn.

> **Transition-lease invariant:** Retirement readiness and actuator delivery
> occur under one revision-bound transition lease. Any competing input, effect,
> or runtime-binding change revokes the lease before actuation.

`thread/compact/start` must not be treated as equivalent to `new_context`
without version-pinned live evidence of its actual semantics. Likewise, an item
or event named `contextCompaction` does not prove whether the provider
summarized, replaced, or otherwise transformed context.

## Rehydration

After observing a fresh window, the host should supply the accepted checkpoint
and the references required to reload governing instructions, activated skills,
authority, and role-specific state. The successor should not receive the prior
transcript merely to recreate meaning already compiled durably.

Reconciliation should establish:

- the same logical role instance remains bound as intended;
- the exact checkpoint revision was loaded;
- governing instructions and skills are applicable;
- authority resolves against canonical sources;
- open human interactions and commitments remain live;
- the authorized next action is still valid; and
- no transition uncertainty is being represented as success.

The prior context need not remain concurrently runnable, but its revision,
checkpoint, source references, and transition evidence must remain recoverable
until reconciliation is accepted.

Until the provider supports a concurrently testable successor or a reversible
context transition, this is a recoverable semantic handoff rather than atomic
two-phase commit. Reconciliation failure can reconstruct continuation from the
checkpoint and predecessor evidence, but it cannot restore the exact retired
model context. The ledger must represent that limitation and any resulting
repair or uncertainty instead of claiming rollback.

## Optimization objective and telemetry

The shared service creates one optimization surface across all Work Engine
roles:

> Minimize total lifecycle cost while preserving correct continuation,
> authority, uncertainty, governing environment, and unresolved human meaning.

Measurements should include:

- retained-context input tokens and latency;
- context growth rate and reported utilization;
- compiler and verifier input/output tokens, latency, and cost;
- checkpoint size and durable-write cost;
- retirement attempts, cancellations, and primary causes;
- rehydration tokens and latency;
- time until productive work resumes;
- rediscovery, repair, and canonical reread cost;
- missed obligations, distorted authority, or reopened decisions;
- semantic-verifier findings and target-role corrections when sampled; and
- final task correctness and review outcomes.

Thresholds and inspection timing are optimization knobs, not product doctrine.
The safety and authority invariants bound the optimization space.

## Invariants

1. Correct continuation must not depend on meaning represented only in the
   retiring context.
2. A compiled checkpoint cannot create, widen, transfer, extend, or reactivate
   authority.
3. Human ambiguity, preference, approval, or obligation cannot be guessed into
   closure.
4. Governing instructions and activated skill state are continuation state when
   later work depends on them.
5. Retirement must bind the exact observed source and fail when a later semantic
   delta overtakes it.
6. Preservation pressure and retirement optimality remain separate judgments.
7. Provider event names and requested operations do not establish transition
   semantics without observed evidence.
8. Opaque provider summaries cannot become the sole owner of required
   continuation meaning.
9. The target model may actuate `new_context`, but it does not thereby acquire
   lifecycle-state ownership or retirement authority.
10. Excluding an interaction from a new context does not delete its history,
    resolve its meaning, or remove its authority.
11. A fresh window must reconcile the exact checkpoint before productive domain
    work resumes.
12. Failed compilation, verification, publication, actuation, transition, or
    reconciliation must remain visibly failed or unresolved.
13. Model-visible content does not gain instruction or authority status merely
    through inclusion in an observed-context projection.
14. Projection construction, destination writes, and retained copies remain
    inside authenticated, least-privilege, confidentiality-aware host
    boundaries.
15. Final readiness comparison and actuator delivery share one revision-bound
    transition lease that competing input, effects, or binding changes revoke.
16. Recovery claims semantic continuation from durable evidence, not restoration
    of the exact retired model context.

## App Server architecture manifest projection

The future App Server architecture manifest can describe this as a service
rather than a role:

```yaml
services:
  semantic-context-lifecycle-manager:
    implementation: services/semantic-context-lifecycle-manager.mjs
    applies_to: work-engine-root-roles

    observes:
      - thread-events
      - turn-items
      - token-usage
      - context-transition-items

    semantic_compiler:
      kind: ephemeral-inference
      output_schema: continuation-state-v1
      retains_context: false

    semantic_verifier:
      kind: ephemeral-inference
      retains_context: false

    ledger:
      schema: context-lifecycle-ledger-v1
      storage: external-durable-state

    transition:
      capability: new_context
      invocation: target-model
      feature_gate: required
      expected_revision_fence: required
      fail_closed: true
```

This manifest describes runtime composition. Role contracts continue to own
role behavior, and the provider adapter continues to own version-specific App
Server mechanics.

## Current evidence gaps

The current scaffold does not yet prove the complete lifecycle:

- the pinned protocol exposes persisted turns and token telemetry but not an
  exact effective-context read;
- `new_context` is not a pinned direct App Server client request;
- its model-side feature gate, invocation evidence, and successor-window signals
  require a version-bounded live probe;
- direct fresh-window identity and transition classification remain unresolved;
- the adapter now compiles request-bound evidence into a deterministic text
  item inside the pinned `TurnStartParams.input` field; a live temporary
  manifest role returned a runtime-random context value, and the production
  strategic-planner role returned a version-1 handoff that passed exact
  objective, evidence-cutoff, continuity, schema, and terminal validation; and
- these live turns establish request transport and the first planning handoff,
  not observed-context projection, context-transition, or reconciliation
  semantics.

These are implementation premises to test, not reasons to weaken the semantic
contract.

## Implementation sequence

The sequence below is a planning hypothesis. Evidence may justify revising it
while every invariant remains preserved.

1. **Completed foundation evidence:** prove one live request-bound planning
   handoff using the pinned model-visible request-context input implemented by
   the adapter.
2. Add the provider-neutral telemetry and observed-context projection contract.
3. Define and validate `continuation-state-v1` and the lifecycle-ledger schema.
4. Implement the hidden semantic compiler and verifier against recorded bounded
   fixtures before allowing retirement.
5. Add human-interaction closure and loading-disposition evaluation.
6. Probe and capability-gate the model-side `new_context` actuation path.
7. Implement revision-fenced checkpoint publication and the sterile retirement
   control turn.
8. Prove fresh-window detection, checkpoint injection, and reconciliation on
   the strategic planner.
9. Run shadow mode across multiple roles: compile and score candidates without
   clearing context.
10. Enable bounded retirement experiments, retain predecessor evidence, and
    compare total lifecycle cost and continuation correctness.

## Relationship to neighboring documents

- [`ideas/01-semantic-context-manager-proposal.md`](../../ideas/01-semantic-context-manager-proposal.md)
  owns the broader semantic-context reasoning and historical proposal lineage.
- [`codex-agents.md`](codex-agents.md) records the observed shared-daemon and
  operator-console topology that makes independently controlled root roles
  practical.
- [`../README.md`](../README.md) describes the current implemented App Server
  boundary rather than this future lifecycle plan.
- [`ideas/priority-one-new-runtime-direction.md`](../../ideas/priority-one-new-runtime-direction.md)
  remains the user-authored migration direction and is referenced, not modified,
  by this plan.

## Open decisions

- Which component owns final retirement authorization once compiler and verifier
  agree?
- Which model/provider and freshness contract should compile and verify state?
- What exact App Server evidence identifies the model-side `new_context` call
  and its successor window?
- How should live human input preempt a pending retirement control turn?
- Which interaction classes require exact source loading rather than a compiled
  consequence?
- How are role-specific continuation schemas registered and versioned?
- What uncertainty threshold blocks retirement rather than triggering another
  bounded inspection?
- How long must predecessor checkpoints and transition evidence be retained?
