# Context Compaction as a Role Resume Event

## Status

Formed idea direction; permanent placement, owning contracts, and implementation
remain unaccepted.

This note preserves an observed recovery risk and a candidate architectural
consequence. It does not change an existing role contract, grant authority,
authorize implementation, or declare that current recovery machinery satisfies
the consequence.

## Candidate consequence

Model context is ephemeral execution memory. Model-context compaction, context
replacement, and reconstructed sessions therefore cannot safely be presumed to
preserve reliable procedural or operational state.

A future owning contract should require both:

1. **continuous semantic durability**: a resume-critical consequence is
   published as soon as losing it would risk incorrect continuation, duplicate
   an effect, lose authority context, or require material reconstruction; and
2. **context-discontinuity recovery**: a replacement context crosses a recovery
   barrier before consequential continuation and reconstructs its operational
   position from durable owners and refreshed observations.

Compaction detection may improve recovery latency, but correctness must not
depend on receiving advance warning. A pre-compaction flush is an optimization,
not the durable-state boundary.

Conversation summaries and remembered context may help locate durable state.
They are not sufficient canonical evidence for approvals, dispositions,
completed consequences, pending obligations, accepted boundaries, or the next
authorized action.

Compaction does not itself:

- activate a role or runtime;
- grant or preserve unrecorded authority;
- prove that an attempted action completed;
- make remembered state current;
- change the role's semantic workflow phase;
- retire an outstanding obligation; or
- authorize reconstruction of a missing decision.

If a resume-critical consequence has no durable owner, recovery must expose a
gap instead of silently guessing. The durable consequence should be compact
semantic state, an attributed judgment, or an integrity-bound reference to a
stronger owner—not a transcript or hidden reasoning.

## Runtime continuity is orthogonal to semantic phase

Compaction is a runtime-continuity event for an active role. It is not
necessarily a transition in the role's semantic workflow.

```text
before compaction
  semantic phase: implementation
  continuity: bound
  context generation: 4

after compaction is observed
  semantic phase: implementation
  continuity: recovering
  context generation: 5

after reconciliation
  semantic phase: implementation
  continuity: bound
  context generation: 5
```

Keeping these axes separate prevents runtime replacement from fabricating a
domain transition. A role remains in planning, implementation, review, waiting,
or another role-owned phase unless its owning workflow establishes a real
semantic change.

## Structural placement direction

Recovery should not live only in handwritten skill prose. A compacted model
cannot be trusted to remember to invoke the rule that protects it from
compaction.

The candidate structure is:

```text
canonical invariant and capability definitions
                    ↓ references
system role environment + reusable role profiles
                    ↓
specific role definition
                    ↓
organizational compiler
                    ↓
problem execution envelope
                    ↓
role-scoped runtime projection
                    ↓
generated skill instructions + runtime activation
```

This follows the direction in
[Context-Derived Organizational Execution Envelopes](./context-derived-role-execution-envelopes.md):
static role structure is compiled with problem context and authority into an
immutable execution envelope, from which each activated role receives a scoped
projection.

It also supports the shared structural model proposed by
[Work Engine Studio: Design, Control, and Forensics](./work-engine-studio-design-control-forensics.md):
the same definitions can drive design views, runtime control, and forensic
reconstruction without creating independent interpretations.

## System environment and reusable profiles

Globally applicable role structure should have a separate structured owner.
Truly universal bindings belong in a system environment; requirements that
apply only to a class of roles belong in reusable profiles.

```text
role-definitions/
  system.yaml
  profiles/
    durable-role.yaml
    independent-reviewer.yaml
    repository-mutator.yaml
  roles/
    slice-supervisor.yaml
    slice-builder.yaml
    proposal-former.yaml
    strategic-planner.yaml
```

The exact paths and decomposition remain implementation choices. The important
distinction is semantic:

```text
system environment
  relationships that bind every role

reusable role profile
  relationships that bind a declared class of roles

specific role definition
  role-owned objective, meaning, authority, state profile, and exclusions

execution envelope
  effective organization authorized for one problem and context

runtime overlay
  currently observed availability, binding, generation, and health
```

Context ephemerality may be universal. Durable recovery should apply only to
roles whose operational correctness depends on state surviving context loss.
Disposable reconnaissance roles should not acquire heavyweight recovery
obligations merely because other roles require them.

### Illustrative system environment

```yaml
schema_version: 1
environment_id: work-engine.system

bound_by:
  - INV-001
  - INV-002
  - INV-023

available_capabilities:
  - capability.durable_state
  - capability.agent_state

global_boundaries:
  context:
    authority: non_authoritative
    lifetime: ephemeral
  runtime_binding:
    authority: non_semantic
  durable_state:
    owns: opaque_atomic_publication
    does_not_own:
      - workflow_meaning
      - transition_authority
      - semantic_recovery
```

The identifiers are illustrative. The file should bind canonical invariant and
capability identities; it should not duplicate their definitions or become a
second semantic owner.

### Illustrative durable-role profile

```yaml
profile:
  id: profile.durable_role
  applies_when:
    operational_state: durable

  requires:
    - capability.agent_state

  context_continuity:
    triggers:
      - context_compacted
      - context_replaced
      - runtime_reconstructed

    consequence:
      before_consequential_continuation:
        - recover_role_state
        - refresh_authoritative_references
        - reconcile_uncertain_actions
        - bind_new_context_generation

    conversation_summary:
      classification: non_authoritative_hint
```

This is a structural consequence, not a required internal procedure. The event
vocabulary and realization may vary by provider so long as the effective role
cannot continue consequentially across an unreconciled discontinuity.

### Illustrative role specialization

```yaml
role:
  id: role.slice_supervisor
  profiles:
    - profile.durable_role

state:
  profile: state-profile.slice-supervisor
  owns:
    - campaign_phase
    - pending_obligation
    - handled_consequences

recovery:
  refresh:
    - accepted_plan
    - active_checkpoint
    - builder_binding
    - gate_state
```

The reusable profile establishes the recovery property. The role definition
owns what its state means and which references must be reconciled. A generated
skill may project the relevant binding instruction, but it is not an
independent policy source.

## Ownership boundaries

The candidate split is:

| Concern | Candidate owner |
| --- | --- |
| Meaning of an invariant | Canonical invariant definition |
| Capability affordance and exclusions | Canonical capability definition |
| Universal role bindings | System role environment |
| Durable-role recovery requirement | Reusable durable-role profile |
| Role-specific state and recovery meaning | Specific role/workflow definition |
| Effective identities, capabilities, authority, and policy for one problem | Compiled execution envelope |
| Provider-specific compaction or replacement observation | Runtime adapter |
| Identity, state revision, writer generation, fencing, and recovery mechanics | General agent-state capability |
| Atomic opaque publication | `durable-state` primitive |
| Proposal, receipt, checkpoint, schedule, review, or authority facts | Their existing domain owners |
| Current runtime binding and observed availability | Runtime/control-plane projection |

The runtime adapter detects a discontinuity; it does not interpret workflow
meaning. The agent-state capability supplies common recovery mechanics; it does
not invent a universal phase vocabulary. Each role workflow interprets its own
state and chooses the next authorized semantic action.

## Semantic persistence boundary

"Persist immediately" does not mean saving every token, thought, observation,
or tentative branch. It means publishing a semantic consequence when losing it
would:

- change the correct next action;
- lose an exercised authority decision or accepted boundary;
- resurrect an already handled event;
- duplicate or ambiguously repeat an external effect;
- discard an expensive conclusion whose material premises remain relevant; or
- force a replacement role to repeat a material decision.

Useful durable decision lineage includes:

- what was concluded or decided;
- the attributed role or authority;
- decisive premises and evidence references;
- material uncertainty;
- invalidation or reopening conditions;
- a rejected route only when retaining its failure consequence prevents
  repeated work; and
- the next safe semantic obligation.

This is causal continuity, not chain-of-thought persistence.

For a consequential external effect, a role profile may need a crash-safe shape
such as:

```text
publish intent + idempotency identity
→ perform the effect
→ observe the authoritative outcome
→ publish the protected consequence
```

If context is lost between these points, recovery represents the effect as
uncertain and reconciles it. It does not convert lack of acknowledgement into
completion or blindly retry the action.

## Recovery consequence

Before consequential continuation after an observed or suspected context
discontinuity, a recovering role must be able to establish:

1. the logical role, workflow, attempt, and effective execution envelope;
2. the latest integrity-checked role-owned state revision;
3. the exact accepted boundary and authority governing the next action;
4. which consequences are completed, pending, uncertain, stale, or blocked;
5. which source events have already produced protected consequences;
6. which referenced domain artifacts must be refreshed from their owners;
7. current mutable repository and runtime observations;
8. whether prior judgments remain applicable to the current subject revision;
9. which writer/context generation is allowed to act; and
10. the next authorized semantic obligation.

A candidate recovery realization is:

```text
observe or suspect context discontinuity
→ prevent consequential continuation
→ load execution envelope and role-owned state
→ refresh and reconcile referenced owners
→ classify ambiguous attempted effects
→ fence the previous writer generation
→ bind the replacement runtime/context generation
→ publish the recovered projection with compare-and-swap
→ continue only the recovered pending obligation
```

This sequence illustrates the causal dependencies of recovery. It does not
require every role or provider to expose identical commands.

## Runtime observation and provider independence

Codex rollout evidence currently exposes an explicit compaction boundary with
previous and replacement context-window identity. A future Codex runtime
adapter can normalize that observation into provider-independent runtime
continuity evidence.

Other providers may expose a different event or no reliable event. Therefore:

- explicit compaction observation should trigger recovery when available;
- a new runtime/context generation should also trigger recovery;
- reconstructed activation should reconcile even when the original loss event
  is unavailable; and
- provider window, session, and process identities remain runtime bindings, not
  durable Work Engine identity.

This aligns with
[Work Engine Control Protocol and Environment Affordances](./work-engine-control-protocol-and-environment-affordances.md):
clients and runtime adapters expose observations and controls through a bounded
protocol while workflow owners retain semantic authority.

## Relationship to current work

The current [`agent-environments.yaml`](../docs/agent-environments.yaml) is an
implemented structural precursor: it distinguishes role contracts, effective
environment, and an intentionally absent live overlay. Its generated views
already treat the YAML as the owner of role-projection truth while invariant
and machinery truth remain with their canonical sources.

[Role-Owned Durable Operational State](../proposals/agent-state/role-owned-durable-operational-state/proposal.md)
defines the broader candidate contract for compact role-owned state,
references to stronger owners, recovery identity, event/consequence
separation, fencing, and crash-safe reconstruction. This idea specializes the
context-discontinuity trigger and shows how that contract can enter the future
role-definition and execution-envelope architecture.

Current `slice-supervisor` live state is a bounded precursor rather than a
general solution. It establishes stable attempt identity, a pending obligation,
handled consequences, authoritative references, optional provider binding, and
compare-and-swap transitions for planning and review. It does not yet establish
the complete system/profile/role compilation model or recovery for every
semantic phase and role.

Claim-level evidence lineage may preserve expensive epistemic conclusions that
a resumed role references. Durable operational state preserves the role's
current reliance, pending refresh, and next authorized action. Neither claim
history alone nor a conversation summary is a complete role-recovery contract.

## Evidence needed before acceptance or implementation

- Define one structured system environment, one reusable durable-role profile,
  and one concrete role specialization without duplicating invariant or
  capability ownership.
- Compile them with a bounded problem configuration into an immutable execution
  envelope and lossless role projection.
- Generate or validate the corresponding runtime skill projection rather than
  maintaining the recovery rule independently in prose.
- Exercise context loss before, during, and after a protected state publication
  and an external effect.
- Demonstrate that a replacement context continues the exact pending semantic
  obligation without transcript access, duplicate consequences, or resurrected
  events.
- Demonstrate writer-generation fencing when an old and replacement context
  could both attempt continuation.
- Demonstrate provider-specific compaction observation through a normalized
  runtime boundary without making provider identity the durable role identity.
- Show that disposable roles can remain outside the durable-role profile while
  universal authority and context-truth boundaries still apply.
- Project the same structural definitions into the Agent Environment Graph and
  a forensic compaction/recovery view without creating new owners.

## Remaining placement questions

- Which file or registry owns canonical invariant and capability definitions
  once the current documentation becomes more structured?
- Does the system environment own only universal bindings, or also default
  capability availability that an execution envelope may remove?
- What authorization permits an execution envelope to add or remove a reusable
  role profile?
- Which minimal common fields belong in the agent-state envelope, and which
  remain entirely inside role-specific payloads?
- Does context-generation fencing initially remain role-local or belong in a
  shared logical-identity/control-plane mechanism?
- Which runtime observation boundary replaces direct rollout-file discovery?
- How are recovery gaps surfaced to the user or supervisor when required state
  was never published?

These questions keep physical placement and implementation route open. They do
not weaken the candidate consequence: context discontinuity must not silently
revert, resurrect, or invent operational truth.
