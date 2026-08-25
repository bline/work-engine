# Semantic Context Manager

## Status

Exploratory architectural proposal, revision 2.

Revision 2 replaces the earlier hooks-first prototype assumption with an
App-Server-first runtime direction supported by the bounded experiment recorded
in `ideas/intake/semantic-context-manager/app-server-runtime-evidence.md`. It
also places this specialist as the repurposed Wind Walker role, distinguishes
durable thread identity from replaceable model context windows, and prohibits
opaque summarizing compaction as a continuation mechanism.

## Proposal

Move semantic durability and context-lifetime judgment out of ordinary task agents and into a specialized Wind Walker context-manager role invoked at bounded inspection points.

The App Server host should cheaply monitor context utilization and relevant
lifecycle events through a provider-neutral runtime adapter. When an inspection
point occurs, it should invoke Wind Walker with a bounded, provenance-bearing
projection of the active role's model-visible context, applicable durable
state, context telemetry, lifecycle position, and expected next unit of work.

The context manager should determine:

- what continuation-relevant meaning currently exists;
- which of that meaning is represented only in context;
- which context-only meaning is worth preserving;
- the cheapest sufficiently faithful durable representation for it;
- whether the current context remains economically useful;
- whether context replacement is semantically safe;
- and what preservation or rehydration actions are required before replacement.

The active task agent should not continuously inspect and manage its own context lifetime. Its residual responsibility should be limited to validating or correcting the context manager's proposed continuation state when prompted.

Agent discovery and lifecycle observation should be host-owned. A
provider-neutral runtime adapter should normalize the available App Server
surface so that Wind Walker does not depend on agents registering themselves
or embed unstable provider method names in its role contract.

A logical role instance binds to an App Server thread, and one durable thread
may contain successive model context windows. The thread is a replaceable
runtime binding and reasoning container, not the canonical owner of role or
workflow state.

The core validity condition remains:

> **Context replacement is valid only when correct continuation no longer depends on any meaning, governing instruction, or activated skill state represented solely in the current context.**

This proposal changes who performs the judgment, not the standard the judgment must satisfy.

---

## 1. Motivation

Requiring every task agent to manage its own semantic durability adds a second problem to every task.

The agent must simultaneously:

1. perform the domain work; and
2. monitor what its working context means, what may be lost, what deserves durability, and when the context should be retired.

This expands the agent's decision space throughout implementation, research, review, planning, and other work. It can divert attention toward context survival, encourage premature state compilation, increase tool and token use, and make context management part of the behavioral environment of every role.

Most turns do not require a complete durability judgment. The expensive semantic inspection is only valuable at certain context-pressure or lifecycle conditions. A specialized role can therefore perform the judgment periodically without requiring continuous observation.

The intended separation is:

```text
cheap mechanical monitoring
        ↓
bounded semantic inspection
        ↓
candidate preservation and retirement decision
        ↓
active-agent validation when required
        ↓
durable write and/or context replacement
```

This preserves model judgment while removing persistent self-management from ordinary task execution.

---

## 2. Architectural Roles

### Context monitor

The context monitor is a cheap environmental mechanism.

It observes quantities and events such as:

- active context tokens;
- effective context limit;
- recent context growth;
- turns or inference calls since the last inspection;
- lifecycle boundaries;
- completion of a slice, review, or decision;
- entry into a substantially different work phase;
- and the expected size or shape of the next unit of work.

It does not interpret the semantic contents of the context and does not decide what should be preserved or whether replacement is safe.

Its responsibility is only:

> **Determine when a semantic context-lifetime inspection is warranted.**

### Wind Walker context manager

Wind Walker is a specialized durable model role invoked only at inspection
points. It has its own App Server thread and an external lifecycle ledger; its
own context is not the authoritative record of another role's lifecycle.

Its responsibility is:

> **Compare the active model-visible working set with the durable continuation state, identify continuation-relevant differences, propose sufficient preservation, and judge whether retaining or replacing the context is preferable.**

It is not the owner of canonical domain truth. It proposes writes into
representations owned by the applicable domain or workflow contracts, obtains
bounded validation or correction from the target role, coordinates safe
replacement, classifies the observed transition, and verifies rehydration.

### Active task agent

The active task agent performs the domain work.

It should use context normally as conditionally durable working memory. It should not continuously inventory its own meaning or estimate its context lifetime.

When the context manager produces a preservation packet, the active agent has a bounded validation responsibility:

> **Identify any material omission or semantic distortion that is apparent from the active work, and accept or correct the proposed continuation state.**

The task agent should not have to reproduce the context manager's entire inspection.

### Supervisor or retirement authority

The system must explicitly assign authority for context replacement. Depending on the surrounding architecture, this may belong to a supervisor, a constrained mechanism, or the context manager after required validation.

Regardless of placement, replacement must remain blocked while a reported semantic or rehydration dependency is unresolved.

### Durable-state owners

Canonical state schemas, claims, decisions, checkpoints, mementos, and domain artifacts retain their existing ownership and validation rules. The context manager does not gain authority to convert tentative meaning into canonical truth merely to qualify the context for replacement.

### Runtime adapter, role registry, and lifecycle ledger

The host environment should discover runtime actors mechanically and bind
durable logical role instances to App Server threads in a host-owned registry.
The runtime adapter owns provider actor enumeration or observation, lifecycle
events, parent-child topology, and capability reporting. The registry records
runtime bindings and references logical actor, role, and authority state
without creating any of them.

Wind Walker's external lifecycle ledger records the observed thread and
context-window identities, inspection revision, token-pressure state,
preservation readiness, target-role validation or correction, transition
authorization, observed transition classification, post-transition
reconciliation, and unresolved uncertainty. That ledger is the authoritative
lifecycle projection; Wind Walker's thread context is working memory.

The context manager consumes provider-neutral runtime observations. It does not require an ordinary task agent to register itself, poll its own context, or select an adapter.

---

## 3. Discovery and Runtime Integration

The intended architecture uses Codex App Server as the first runtime
realization behind one provider-neutral runtime port:

```text
logical role instance
        ↕
role registry ── provider-neutral runtime adapter ── Codex App Server
        │                                              │
        └── lifecycle ledger                    thread → context windows
```

The App Server client can observe thread creation and resumption, child
collaboration events, lifecycle and status changes, token-usage updates,
context-transition events, and thread enumeration. Event delivery should be
the fast path and bounded enumeration/reconciliation the recovery path. Exact
App Server capabilities remain negotiated provider observations, not Work
Engine authority.

Lifecycle hooks may remain a limited compatibility or observation surface, but
they are no longer the planned first implementation and do not define the
provider-neutral contract.

Environment selection should follow explicit configuration or an authenticated capability handshake where available. Automatic detection may select a runtime realization, but it must not infer role identity, authority, or semantic continuity. A logical actor should keep an opaque Work Engine identity while its runtime binding revision changes.

A runtime actor must not be assumed to be the same logical role instance merely
because paths, prompts, parentage, or timestamps resemble one another. Binding
or replacement requires an explicit logical-role identity, a revisioned
runtime binding, and a fenced handoff.

The first implementation should use the smallest App Server scaffold needed to
exercise one real read-only role and one Wind Walker observer vertical. It
should capability-negotiate experimental methods and generated protocol
bindings rather than making one alpha version's method names durable doctrine.

---

## 4. Inspection Points

The context manager should not observe the task continuously.

Candidate inspection triggers include:

- context utilization crossing a learned or provisional pressure band;
- insufficient estimated capacity for the next coherent unit of work;
- slice or phase completion;
- transition from implementation to validation or review;
- acceptance, rejection, or revision of a consequential decision;
- a route being falsified or materially changed;
- creation of a new obligation, boundary, or authority condition;
- completion of a long evidence-gathering sequence;
- before an expected expensive or exploratory computation;
- before a requested context replacement;
- or a supervisor-requested continuity check.

Token pressure is a preservation trigger: rising pressure increases the risk
that continuation-relevant context-only meaning will be lost and can justify
an inspection or preservation request even when the current context remains
useful. It is not itself evidence that clearing is safe or advantageous.

Clearing is a separate optimality judgment. A coherent work-unit boundary is a
good opportunity, but not a mandatory clearing point. Automatic budget
exhaustion is an emergency guardrail rather than the normal lifecycle policy.

Inspection timing should ultimately be calibrated from observed lifecycle cost and outcomes rather than fixed permanently by intuition.

---

## 5. Inspection Input

A context-lifetime inspection should receive the smallest input sufficient for faithful judgment. Candidate input includes:

```yaml
inspection_input:
  active_context:
    model_visible_messages: ...
    relevant_tool_results: ...
    applicable_loaded_instructions: ...

  snapshot_binding:
    context_revision: ...
    last_completed_turn: ...
    instruction_revisions: ...
    activated_skill_revisions: ...
    durable_state_revisions: ...
    active_actor_and_runtime_binding: ...
    expected_next_work_identity: ...

  durable_continuation_state:
    objectives: ...
    decisions: ...
    obligations: ...
    checkpoints: ...
    unresolved_questions: ...
    activated_skills: ...

  lifecycle:
    role: builder
    work_position: implementation_validation
    expected_next_work: ...

  telemetry:
    active_context_tokens: ...
    effective_context_limit: ...
    recent_growth_rate: ...
    prior_rehydration_cost: ...
```

The exact context transport mechanism remains an implementation question. The semantic requirement is that the manager can inspect a trusted, provenance-bearing projection of the model-visible material from which the active agent's next invocation would reconstruct its working understanding. Instructions, user messages, tool results, retrieved data, and authority-bearing events must remain distinguishable rather than becoming one undifferentiated text block.

---

## 6. Semantic Difference Inspection

The primary inspection is a semantic difference, not a transcript summary.

```text
meaning required for correct continuation
        −
meaning already represented durably
        =
context-only continuation meaning
```

Recognition dimensions include:

- objective;
- logical progression;
- current work position;
- obligations;
- decisions and their consequences;
- authority;
- boundaries;
- uncertainty;
- evidence and evidentiary interpretation;
- dependencies;
- identity and relationships;
- intent;
- constraints;
- valid and falsified routes;
- temporal applicability;
- unresolved interaction meaning;
- governing instructions;
- and activated skill state.

These are inspection aids rather than a universal mandatory schema.

The manager should preserve meaning rather than indiscriminately copying transcript text.

---

## 7. Activated Skills and Governing Environment

Installed skill sources and loaded skill state have different durability.

A skill may remain available on disk or in a capability registry while the following information exists only in the current model context:

- that the skill is active for the current work;
- why it was activated;
- which revision or source governed the work;
- which instructions currently apply;
- which required workflow steps have already occurred;
- which required steps remain pending;
- and what obligations or artifacts were produced under the skill.

Therefore activated skills must be treated as continuation state.

A continuation representation may take the form:

```yaml
activated_skills:
  - identity: documents
    source: durable-skill-reference
    source_revision: ...
    activation_reason: editing a DOCX artifact
    reload_on_continuation: true
    completed_requirements:
      - initial document generated
    pending_requirements:
      - render and visually verify
```

When the exact skill source remains durably and reliably addressable, preserving the entire instruction text is unnecessary. The state must instead preserve the reload obligation and any skill-derived progress not recoverable from the source alone.

When the applicable instructions cannot be reliably reloaded, their continuation-relevant semantic effects must themselves be preserved.

This implies a broader rehydration requirement:

> **Correct continuation requires reconstruction of both task state and the governing environment under which the task must continue.**

Applicable role configuration, authority, instructions, and activated skills are part of that environment.

---

## 8. Preservation Judgment

For each context-only continuation candidate, the manager should consider:

- expected future relevance;
- consequence and risk of loss;
- reconstructability;
- durability of required reconstruction inputs;
- reconstruction cost;
- expected loss of fidelity, nuance, attribution, uncertainty, or authority;
- current preservation cost;
- provenance and schema overhead;
- later rehydration cost;
- and the risk of prematurely crystallizing tentative reasoning.

The manager should choose among:

- no preservation;
- reference to durable evidence;
- preservation of sufficient prior inputs;
- preservation of a derived result with provenance;
- canonical typed state;
- an existing domain artifact;
- a claim or decision consequence;
- a checkpoint;
- or a bounded continuation memento.

The relevant principle remains:

> **Preserve whichever representation minimizes expected lifecycle cost while retaining sufficient truth, fidelity, provenance, uncertainty, and authority.**

---

## 9. Delegated Authority and Security

Transferred or delegated authority is continuation-relevant state when later work depends on it.

If a grant exists only in the current context, replacement creates two symmetric risks:

- **authority loss:** a valid grant disappears and authorized continuation becomes incorrectly blocked; or
- **authority fabrication:** a fresh agent infers, assumes, broadens, or recreates permission without a valid grant.

Authority must therefore be durably represented when its required lifetime exceeds the safe lifetime of the current context. However, the preserved representation must distinguish an authoritative grant from a descriptive account of one.

### Authority record

A durable delegated-authority record should identify at least:

- the grant identity;
- issuer and issuer authority;
- intended recipient or role;
- granted capability;
- exact scope;
- constraints and prohibitions;
- time or lifecycle applicability;
- revocability and current revocation status;
- transferability or non-transferability;
- originating evidence or authorization event;
- and current validity status.

For example:

```yaml
delegated_authority:
  grant_id: authority-grant-318
  issuer: supervisor-instance-17
  recipient: builder-instance-42
  capability: modify_validation_path
  scope:
    files:
      - validation/current_write.py
  constraints:
    - preserve_backward_compatibility
  granted_at: ...
  expires_at: slice_completion
  revocable: true
  revoked_at: null
  transferable: false
  evidence_ref: authority-event-318
  status: active
```

The exact schema belongs to the authority subsystem rather than the context manager. The context manager's responsibility is to detect that authority affects continuation and to route preservation into the authoritative representation.

### Descriptive state does not confer authority

A checkpoint, inspection packet, continuation memento, transcript summary, or rehydration note may report that authority existed. Such a report is not itself a grant.

The context manager must not:

- create authority by describing it;
- widen the scope of a grant;
- change its recipient;
- remove or reinterpret constraints;
- extend its duration;
- make a non-transferable grant transferable;
- reactivate expired or revoked authority;
- or treat ambiguous human language as an authorization.

### Rehydration validation

Before an agent relies on preserved authority after context replacement, the authority must be resolved against its canonical source and validated for the new execution context.

Validation should establish:

- that the grant is authentic;
- that the issuer possessed authority to make the grant;
- that the continuing actor is the intended recipient or a valid successor under an explicit transfer rule;
- that the contemplated action remains within scope;
- that all constraints still apply;
- that the grant has not expired or been revoked;
- that context replacement did not terminate it;
- and that any required evidence remains available.

If canonical validation is unavailable, descriptive preservation may prevent semantic loss, but it must not permit execution that requires the grant.

### Identity across replacement

Context replacement must not silently equate a fresh model invocation, agent instance, role instance, or process with the original authority recipient.

The authority subsystem must define whether identity survives replacement and under what conditions. A grant to a role may survive rehydration differently from a grant to a particular agent instance. This is an authority decision, not a context-manager inference.

The governing principle is:

> **Authority reconstructed after context replacement must derive from a durable, verifiable grant; descriptive continuation state alone cannot confer authority.**

---

## 10. Retirement Judgment

Preservation and replacement are separate decisions.

The manager should first determine whether context-only meaning requires preservation. It should separately determine whether the current context remains worth carrying.

Candidate recommendations include:

1. `continue_without_preservation`
2. `preserve_and_continue`
3. `finish_current_computation_then_reinspect`
4. `preserve_then_replace`
5. `replacement_unsafe`
6. `insufficient_evidence`

The economic comparison is:

```text
expected cost of retaining the current context
                    versus
residual preservation + replacement + rehydration + rediscovery
```

Retaining the current context may include repeated input-token cost, latency, context competition, stale-information interference, behavioral effects, and increasing risk of forced replacement.

Replacing it may include state compilation, state writes, fresh-context bootstrap, skill and instruction reload, evidence rereads, tool rediscovery, reconstruction, and time until productive work resumes.

A context may be economically exhausted before it reaches its technical limit. Conversely, a high-pressure context may remain valuable when the agent is midway through a tightly coupled computation whose reconstruction would be expensive or unreliable.

Economic attractiveness never overrides semantic safety.

### Snapshot fence

Every inspection and proposed handoff must bind an immutable context revision together with the relevant governing-instruction, activated-skill, durable-state, actor, runtime-binding, lifecycle, and expected-next-work identities.

Before replacement, the retirement authority must establish either:

- that the active context and every bound dependency still equal the inspected revisions; or
- that every subsequent delta has been inspected, incorporated, and rebound into a successor handoff.

A stale inspection packet cannot authorize retirement. Compare-and-swap publication and exact-identity recovery are useful mechanical precedents, but the owning context-lifetime contract must define the complete snapshot subject.

### Two-phase rehydration

Replacement should be a two-phase handoff:

```text
bind context snapshot
→ manager proposes preservation
→ destination owners validate and publish
→ active agent checks material omissions
→ retirement authority verifies continuation independence and permits native replacement
→ host confirms a new context-window identity on the same bound thread
→ fresh context rehydrates the exact handoff
→ fresh context demonstrates required task and governing state
→ lifecycle ledger records reconciliation or uncertainty
```

The inspected context revision, its preservation decision, and the resulting
durable handoff must remain recoverable until the fresh context acknowledges
the exact handoff. The active old window need not execute concurrently with the
new one, but transition must not destroy the evidence required to diagnose or
repair failed rehydration. If the host cannot preserve that recovery evidence,
effectful replacement is not yet a safe reachable outcome.

Native fresh-context replacement is distinct from opaque summarizing
compaction. Work Engine may permit the former after readiness. It must not use
the latter as a continuation mechanism because an opaque generated summary can
silently change or omit workflow meaning.

---

## 11. Inspection Output

The manager should produce an actionable inspection packet rather than a generic reminder.

```yaml
context_lifetime_inspection:
  snapshot_binding:
    context_revision: ...
    dependency_revisions: ...
    active_actor_and_runtime_binding: ...
    expected_next_work_identity: ...

  assessment:
    context_value: declining
    boundary_quality: high
    next_work_fit: insufficient
    confidence: medium

  context_only_meaning:
    - identity: route-falsification
      meaning: the schema route was rejected because the existing contract is sufficient
      future_dependency: prevents reopening a rejected route
      reconstructability: expensive
      proposed_representation: decision_consequence

  activated_environment:
    skills_to_reload: ...
    pending_skill_obligations: ...
    governing_instructions_to_restore: ...

  authority_dependencies:
    grants_required_for_continuation: ...
    canonical_records: ...
    revalidation_required: true
    unresolved_authority: ...

  proposed_writes:
    - target: decision_state
      operation: append_consequence
      content: ...

  recommendation: preserve_then_replace

  handoff:
    phase: proposed
    successor_revision: ...
    rehydration_acknowledgement: pending

  replacement_blockers:
    - proposed writes have not been validated
    - active context has not been fenced at the inspected revision
    - fresh context has not acknowledged the exact handoff
```

The packet should distinguish detected absence from uncertainty. Failure to detect a continuation dependency is not proof that none exists.

---

## 12. Authority and Safety Invariants

### INV-CM-1: Semantic safety precedes retirement

Context replacement must not occur while correct continuation depends on meaning represented solely in the current context.

### INV-CM-2: Governing environment is continuation state

Context replacement must not occur unless applicable governing instructions and activated skill state can be faithfully rehydrated.

### INV-CM-3: Inspection does not create domain authority

The context manager may propose durable representations but may not promote uncertainty, interpretation, or tentative reasoning into canonical truth without the authority required by the destination contract.

### INV-CM-4: Preservation and replacement remain distinct

The need to preserve meaning does not imply that context should be replaced. The economic attractiveness of replacement does not imply that preservation is complete.

### INV-CM-5: The builder's validation is bounded

The active agent may correct an inspection packet but should not be required to repeat the complete context-lifetime analysis as a condition of ordinary work.

### INV-CM-6: Inspection is opportunistic

Ordinary task execution must not require continuous semantic inventory or continuous context-management reasoning.

### INV-CM-7: Rehydration must restore applicability

Reloading raw state is insufficient when identity, revision, authority, temporal applicability, or skill-derived obligations are required to interpret it correctly.

### INV-CM-8: Unresolved human meaning is not guessed into closure

The manager must not resolve ambiguity, intent, preference, authorization, or negotiated meaning merely to make context replacement possible.

### INV-CM-9: Descriptive state cannot confer authority

An inspection packet, checkpoint, memento, summary, or other descriptive continuation representation must not create an executable grant.

### INV-CM-10: Authority cannot expand through preservation

Preservation and rehydration must not widen a grant's capability, scope, recipient set, duration, transferability, or permitted consequences, and must not remove its constraints or prohibitions.

### INV-CM-11: Rehydrated authority requires canonical validation

An agent must not rely on preserved authority after replacement unless the applicable authoritative source validates the grant for the continuing actor, contemplated action, scope, constraints, and current lifecycle position.

### INV-CM-12: Replacement does not imply identity continuity

Whether the recipient of a grant survives context replacement must be defined by the authority system. The context manager must not infer recipient continuity from role or task similarity.

### INV-CM-13: Invalid authority fails closed

Expired, revoked, ambiguous, unverifiable, non-transferable, or otherwise inapplicable authority must not become actionable through context preservation or rehydration.

### INV-CM-14: Discovery is host-owned

Ordinary task agents must not be required to discover or register themselves, poll context pressure, or select the runtime adapter. Runtime observation is host machinery and does not confer semantic identity or authority.

### INV-CM-15: Retirement is fenced to the inspected snapshot

A context must not be retired from an inspection packet unless every bound subject still matches or every later delta has been incorporated into a newly bound handoff.

### INV-CM-16: Rehydration precedes release

The inspected context revision and exact handoff must remain recoverable until
the fresh context has rehydrated and acknowledged the task state, governing
environment, actor binding, and unresolved obligations required for
continuation.

### INV-CM-17: Transition labels are not transition semantics

The host must not infer summarization, replacement, or context continuity from
an event name such as `contextCompaction`. Classification requires the thread
identity, prior and current context-window identities, preservation readiness,
event ordering, summary reliance, and post-transition evidence.

### INV-CM-18: Opaque summaries cannot own continuation

An opaque provider-generated summary must not become the sole owner of meaning
required for correct continuation. Native replacement is permitted only after
continuation independence has been established against explicit durable state.

---

## 13. Why a Separate Role Can Perform the Inspection

The active task agent does not carry a directly accessible durable private mental state between inference calls. Each subsequent invocation reconstructs working understanding from model-visible context and available state.

A context manager given the same model-visible context can inspect substantially the same evidence. Its specialization may improve the inspection because its inference space is organized around semantic difference, preservation economics, and context retirement rather than the domain task.

There is still an asymmetry: the active agent generated the work and may more readily reconstruct its logical progression. This justifies bounded validation by the active agent. It does not require keeping the entire management problem inside every task role.

---

## 14. Expected Benefits

The proposal is intended to:

- reduce the decision space of builders and other task agents;
- keep context management low-salience during ordinary work;
- avoid repeated self-monitoring and semantic inventory;
- reduce premature crystallization of tentative reasoning;
- concentrate continuity judgment in a role that can be evaluated independently;
- produce explicit and inspectable replacement decisions;
- make preservation and rehydration costs measurable;
- and allow context-retirement policy to improve empirically without changing every task role.

The architectural benefit is not that context management becomes free. It becomes bounded, specialized, observable, and removable from the behavioral environment of most work.

---

## 15. Risks and Open Questions

### Duplicate context cost

The manager may need to consume much of the active context during inspection. This cost must be compared with the continual self-management cost removed from the task agent and with the cost of preventable context loss.

### Context transport

The implementation must establish how the host produces a trusted, provenance-bearing, revision-bound projection of the relevant model-visible context and governing environment without accidentally omitting precisely the state it is meant to inspect. A normal subagent fork may be useful evidence, but it is not assumed to reproduce the exact effective model input.

### Runtime capability differences

The observed alpha App Server build inherited token-budget behavior and kept one
thread identity across native fresh-context transitions, both at automatic
budget exhaustion and after manual `thread/compact/start`. Neither observed
route exhibited legacy summarizing-compaction behavior, but the provider
emitted an item named `contextCompaction`. Supported builds must therefore be
capability-negotiated and integration-tested, and transition semantics must be
classified from multiple observations rather than one event or method label.

### Inspection timing

Inspection that occurs too early wastes inference. Inspection that occurs too late may leave insufficient capacity to preserve meaning safely. Token bands and lifecycle triggers require empirical calibration.

### False reassurance

A manager may fail to detect important context-only meaning. Confidence, uncertainty, active-agent validation, and replacement blockers must be represented explicitly.

### Interruption cost

Inspections should occur at coherent boundaries when possible. A context manager that repeatedly interrupts active computation may reproduce the environmental burden it is intended to remove.

### Authority placement

The system must decide who can accept proposed writes and who can authorize replacement for different role types, especially human-facing roles.

It must also define identity continuity across replacement, the canonical source of grants, grant lifetime, revocation behavior, and whether any classes of authority may survive replacement at all.

### Rehydration verification

The system needs evidence that a fresh context actually reloaded the required state, skills, instructions, identities, and obligations before productive continuation begins. The host must also be able to retain or recover the old context until that acknowledgement succeeds.

---

## 16. Prototype Shape

An initial prototype does not require a calibrated optimization policy.

It can begin with:

1. the thin App Server adapter, generated protocol bindings, and capability negotiation;
2. a logical-role registry that keeps role identity distinct from thread identity;
3. exact skill resolution and per-turn skill injection;
4. a thread-scoped dynamic-tool bridge;
5. `strategic-planner` as the first read-only role vertical;
6. a Wind Walker role and external lifecycle-ledger contract;
7. exact snapshot identity for thread, context window, turn, instructions, skills, and durable state;
8. active-role validation of proposed preservation;
9. native fresh-context replacement after a fenced readiness decision;
10. transition classification and post-transition rehydration verification; and
11. telemetry for inspection, preservation, validation, rehydration, rediscovery, and retained-context cost.

The first vertical should prove observe → preserve → replace → rehydrate →
reconcile on the planner before more stateful roles are migrated. Manual
`thread/compact/start` is the observed host control for requesting a fresh
window after readiness; automatic budget exhaustion remains an emergency
fallback. Automatic runtime migration remains a separate, unverified
operation.

Early evaluation should compare:

- task-agent tokens and behavior with builder-owned management versus specialist management;
- omitted continuation dependencies;
- unnecessary preservation;
- inspection and interruption cost;
- rehydration success;
- time until productive continuation;
- and final correctness or review outcomes.

The prototype should initially test whether the separation is beneficial. It should not prematurely optimize threshold values.

---

## 17. Relationship to the Existing Semantic-Durability Model

The existing semantic-durability and context-lifetime judgment remains the conceptual basis for the specialist role.

Its core ideas are retained:

- context is conditionally durable working memory;
- meaning rather than transcript is the preservation object;
- preservation should be opportunistic;
- preservation cost and reconstructability matter;
- inputs and derived results have different economics;
- bounded continuation mementos cover meaning without a typed home;
- context pressure alters loss risk;
- economic exhaustion may precede technical exhaustion;
- semantic safety and economic benefit are separate questions;
- and the optimal policy should be learned from lifecycle evidence.

The architectural revision is:

> **These judgments should ordinarily be coordinated by a durable Wind Walker
> observer role, not carried as a continuous responsibility by every active
> task agent.**

Wind Walker is repurposed from a generally loaded task-agent skill into that
central observer. Target roles retain semantic ownership and perform only
bounded correction or veto. Wind Walker does not require recursive management:
one inspection is a bounded logical work unit whose result is published to the
lifecycle ledger before its own context is cleared when advantageous.

The provider-neutral runtime-adapter idea and neighboring App Server scaffold
idea supply the execution seam for discovering and observing actors. The
context-manager idea owns the candidate allocation of monitoring, inspection,
validation, rehydration, and retirement responsibilities; it does not absorb
runtime implementation, role semantics, or environment capability negotiation.

---

## 18. Compact Findings

> **Having every agent manage its own context adds a second decision domain to every task.**

> **Cheap mechanical monitoring can determine when inspection is warranted; a specialized model role can determine what the context means.**

> **Agent discovery belongs to a host-owned runtime adapter, with Codex App
> Server as the first scaffold realization and lifecycle hooks only a bounded
> compatibility surface.**

> **The manager should inspect the semantic difference between the active working set and durable continuation state, not summarize the transcript.**

> **Loaded skill state and governing instructions are part of continuation state even when their source files remain durable.**

> **Delegated authority must survive when continuation depends on it, but descriptive state cannot create, widen, transfer, extend, or reactivate a grant.**

> **Rehydrated authority is actionable only after validation against its canonical source for the continuing actor and contemplated action.**

> **The builder should validate a bounded candidate continuation packet rather than continuously perform context-lifetime reasoning.**

> **Preserving state, continuing in the current context, and replacing context are distinct decisions.**

> **Pressure triggers preservation; optimality and semantic safety govern clearing.**

> **A logical role instance binds to a durable thread, while the thread may
> contain successive independent model context windows.**

> **An event named `contextCompaction` does not establish whether the runtime
> summarized or replaced a context.**

> **Opaque summarizing compaction cannot be the owner of continuation meaning.**

> **Replacement must be fenced to the exact inspected snapshot, and that
> revision plus its durable handoff must remain recoverable until the fresh
> context acknowledges continuation.**

> **The value of the design must be demonstrated through total lifecycle cost, continuation reliability, reasoning behavior, and final outcomes.**
