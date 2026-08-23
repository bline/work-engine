# Context-Derived Organizational Execution Envelopes

## Status

Future architecture idea / proposal seed

Durably captured on 2026-08-22. This document records an architectural direction
for later decomposition; it is not an accepted product contract or an
implementation plan.

## Core idea

Work Engine should treat roles as **parameterizable organizational machinery**
and separate their stable semantic identity from the context-specific structure
assembled for a particular problem.

A role should not need a separate configuration file for every workflow, slice, task, or execution context.

Skills and other capability providers supply reusable operational machinery.
Roles organize that machinery around objectives, consequences, ownership,
authority, information flow, and lifecycle boundaries.

Instead:

```text
problem specification
        +
system invariants and human authority
        +
role templates and capability contracts
        +
system environment and reusable role profiles
        +
specific role definitions
        +
current context and authorized configuration changes
        ↓
organizational compiler
        ↓
problem-level execution envelope
        ↓
role-scoped projections and runtime activation
```

The execution envelope is the compiled organizational structure for one
concrete problem or bounded problem lifecycle. It contains instantiated roles,
capabilities, ownership and delegation edges, observation and mutation
boundaries, required consequences, collaboration structure, limits, and
configuration provenance.

Each instantiated role receives a role-scoped projection of the envelope. That
projection is the effective world the role inhabits; it is not a separate source
of organizational truth.

It may:

- expand some observations;
- contract others;
- add or remove capabilities;
- select different machinery implementations;
- change mutation boundaries;
- expose additional state projections;
- change provider/model/profile choices;
- add temporary limits;
- add temporary authority where explicitly permitted;
- remove authority where context requires it.

The important constraint is not monotonic narrowing.

The important constraint is:

> **The compiled organization must remain compatible with every instantiated
> role contract, system invariant, authority rule, and required consequence.**

---

# 1. Motivation

Work Engine roles currently combine several kinds of information that change at different rates:

- the essential purpose of the role;
- invariants that define valid behavior;
- available machinery;
- default provider/tool choices;
- observable state;
- mutable state;
- current scope;
- task-specific permissions;
- runtime availability;
- provider state.

Treating all of those as one static skill document makes role tuning unnecessarily risky.

A prose edit may accidentally change:

- role identity;
- authority;
- machinery;
- workflow behavior;
- provider assumptions;
- current configuration.

The architecture becomes safer if the system makes these categories explicit.

---

# 2. Five source and runtime layers

The model has five primary layers. The second and third layers may be stored as
separate YAML documents while remaining parts of one static organizational
definition system.

## 2.1 Essential role templates and skill contracts

This layer defines reusable organizational and operational component types.

A **role template** defines a parameterizable organizational function: what the
role is for, what consequences it must establish, what it owns, how it relates
to other roles, and which dimensions may vary without changing that identity.

A **skill or capability contract** defines reusable operational machinery: its
inputs, outputs, effects, intrinsic invariants, implementation choices, and
configurable dimensions. A capability-providing skill is not automatically a
role and does not independently own an organizational objective.

Examples:

- objective;
- required consequences;
- intrinsic interfaces;
- essential invariants;
- independence requirements;
- authority relationships that define identity;
- intrinsic prohibitions;
- core behavior implemented by scripts/references/prose.

Changing this layer is a **contract change**.

It should normally require proposal/review/approval rather than ordinary configuration editing.

Example:

```text
Independent Reviewer

Essential:
- produces fresh adversarial/falsification evidence;
- remains independent from builder reasoning;
- does not implement fixes;
- emits a bounded review result.
```

If those change materially, this is no longer merely a different reviewer configuration.

It is a different contract.

---

## 2.2 System environment and reusable role profiles

This layer factors structural relationships that should not be repeated in
every concrete role definition.

A **system environment** binds structure that genuinely applies to every role,
such as user-authority boundaries, the non-authoritative status of model
context, common provenance requirements, and globally available machinery.

A **reusable role profile** binds structure shared by a declared class of roles,
such as durable operational recovery, independent-review isolation, repository
mutation, or scheduled-role obligations. A profile is composable structural
configuration, not a role identity and not a capability implementation.

Conceptually:

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

The exact paths and file count are implementation choices. The semantic split
is the important part.

An illustrative system environment:

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
```

An illustrative reusable profile:

```yaml
profile:
  id: profile.durable_role
  applies_when:
    operational_state: durable

  requires:
    - capability.agent_state

  context_continuity:
    consequence:
      before_consequential_continuation:
        - recover_role_state
        - refresh_authoritative_references
        - reconcile_uncertain_actions
        - bind_new_context_generation

    conversation_summary:
      classification: non_authoritative_hint
```

The profile establishes the shared recovery property. It does not define what
`campaign_phase`, `proposal_revision`, `review_finding`, or another role-owned
state field means. That remains with the concrete role or workflow definition.

System and profile files should bind canonical invariant and capability
identities rather than duplicate their definitions:

```text
invariant definition
  owns what the invariant means

capability contract
  owns what the machinery affords and excludes

system/profile YAML
  owns which class of roles receives those bindings
```

### Profile composition and authority

Composition must be deterministic, provenance-bearing, and fail closed on
semantic conflict. A useful source shape is:

```text
system environment
  + explicitly selected reusable profiles
  + concrete role definition
  + authorized problem-level changes
  → effective role projection
```

Profiles may add shared constraints, requirements, references, and default
machinery. They do not independently:

- create a role objective or semantic identity;
- redefine a referenced invariant or capability;
- grant authority unavailable to the role contract;
- weaken an intrinsic prohibition;
- transfer state or artifact ownership;
- make available machinery required merely because it exists; or
- resolve a conflict by last-file-wins precedence.

Every effective field should retain its contributing source identity and
revision. Compatible set-valued constraints may compose. Contradictory
authority, ownership, mutation, visibility, independence, or lifecycle claims
require the owning authority or fail compilation. An execution envelope may
narrow an authorized affordance; expansion requires configuration authority
whose scope permits that exact change.

This keeps reusable profiles from becoming mixins that silently rewrite role
contracts.

---

## 2.3 Specific role environment definition

This layer defines the role's normal structural environment.

It includes:

- invariant bindings;
- available variant machinery;
- default capability realizations;
- default state observations;
- default mutation affordances;
- default provider/model/profile choices;
- configurable dimensions;
- configuration-authority rules.

Example:

```yaml
role: role.builder

profiles:
  - profile.durable_role
  - profile.repository_mutator

invariants:
  - INV-001
  - INV-006
  - INV-014

variants:
  repository_evidence:
    implementations:
      - indexed
      - direct_source

  independent_review:
    implementations:
      - claude
      - codex_adversarial

defaults:
  repository_evidence: indexed
  independent_review: claude

environment:
  observe:
    - state.repository
    - state.accepted_scope

  invoke:
    - capability.repository_evidence
    - capability.independent_review

  mutate:
    - state.task_changes
```

This is not necessarily a maximum permission set.

It is the static basis from which execution-specific environments may be derived.

---

## 2.4 Context-derived execution envelope

The execution envelope is the compiled organization for a particular problem.
It may contain several instantiated roles and capabilities. Each role instance
has a scoped projection describing its effective environment for a particular
activation or bounded lifecycle.

It is derived from:

- the problem objective and required consequences;
- system invariants and human authority;
- relevant role templates and skill/capability contracts;
- the system environment and selected reusable profiles;
- specific role environments;
- current workflow state;
- accepted scope;
- proposal constraints;
- ownership boundaries;
- risk/context;
- configured machinery;
- current authority;
- decisions by roles authorized to configure the environment.

Example:

```yaml
execution_envelope:
  problem_id: slice-17

  roles:
    builder-17:
      template: role.builder
      observe:
        add:
          - artifact.architectural_review
          - state.proposal_authority
      invoke:
        add:
          - capability.architecture_analysis
        remove:
          - capability.independent_review
      mutate:
        state.task_changes:
          boundary:
            paths:
              - src/parser.py
              - tests/test_parser.py
      variants:
        repository_evidence:
          implementation: direct_source

    reviewer-17:
      template: role.independent_reviewer
      receives_from:
        - artifact.accepted_scope
        - artifact.task_checkpoint
      forbidden_from:
        - state.builder_private_context

  ownership:
    artifact.implementation_receipt: builder-17
    artifact.review_result: reviewer-17
```

This envelope expands some dimensions and contracts others while assigning the
relationships required to solve the problem.

The role templates have not changed.

Their parameters, relationships, and effective execution worlds have been
compiled for this problem.

---

## 2.5 Runtime projection

Runtime state answers what is actually available now.

Examples:

- Codex thread loaded;
- provider unavailable;
- Claude quota exhausted;
- capability healthy/unhealthy;
- credential present;
- active turn;
- runtime child agents;
- current lease;
- provider reset time.

Example:

```yaml
runtime:
  independent_review:
    authorized: true
    available: false
    reason: provider_quota

  repository_mutation:
    authorized: true
    available: true
```

Runtime availability is distinct from authority and obligation.

---

# 3. Available, authorized, and required are different

A capability has at least three independent dimensions:

```text
AVAILABLE
Can the machinery execute right now?

AUTHORIZED
May this role use it in this context?

REQUIRED
Does the current workflow require its consequence?
```

These must not be conflated.

Example:

```text
Independent review

available: no
authorized: yes
required: yes
```

The correct consequence may be:

```text
workflow → waiting_on_capability
```

not:

```text
disable review requirement
```

Likewise:

```text
available: yes
authorized: no
```

must not imply the role can invoke it.

---

# 4. The compiled static configuration is not necessarily a ceiling

A previous framing treated the role's compiled system/profile/specific-role
environment as a maximum authority envelope.

That is too restrictive as a general rule.

Context may legitimately require:

- broader observation;
- additional analysis machinery;
- temporary authority;
- reduced mutation;
- alternate providers;
- new bounded state projections.

For example:

```text
Default Builder
    observes repository
    mutates task-owned work

Architecture-sensitive Builder activation
    additionally observes architectural review
    additionally invokes architecture analysis
    mutation narrowed to 3 paths
```

The important test is semantic validity.

Not whether every context-specific permission is a subset of the default.

---

# 5. Some dimensions may still have hard ceilings

Although the static role configuration is not generally a maximum permission set, specific authority dimensions can still have hard system or contract ceilings.

Examples:

```text
Builder may never self-accept a slice.

Reviewer may never mutate repository state.

Supervisor may never silently change the user objective.

Publication may always require explicit user authority.
```

Those are invariant or contract boundaries.

The system should distinguish:

```text
configurable default
```

from:

```text
hard authority ceiling
```

rather than treating all permissions uniformly.

---

# 6. Configuration authority is itself modeled authority

A powerful consequence of execution envelopes is that one role may be
authorized to propose or shape parameters and relationships for another role
instance in the compiled organization.

Example:

```text
Slice Supervisor
    MAY_CONFIGURE
        Builder observations
        Builder capability selection
        Builder mutation boundary
        Builder provider profile
```

The supervisor is not editing the Builder contract.

It is requesting a change to the Builder projection within a problem-level
execution envelope.

This creates two distinct authority questions:

```text
Can Builder do X?

Can Supervisor grant Builder X in this context?
```

Those must be represented separately.

---

# 7. Supervisor-configured Builder projection

A supervisor might contribute this authorized change request to organizational
compilation:

```yaml
organizational_change:
  target_role_instance: builder-17

  observe:
    add:
      - artifact.architectural_review

  invoke:
    add:
      - capability.architecture_analysis

  mutate:
    state.task_changes:
      boundary:
        paths:
          - src/foo.py
          - tests/test_foo.py

  variants:
    independent_review:
      implementation: codex_adversarial
```

The change request is authorized and compiled, and the resulting envelope is
validated before activation.

The supervisor's own authority may prohibit it from granting certain things.

Example:

```text
Supervisor may configure:
- approved engineering capabilities;
- task scope;
- observation projections;
- provider realization;
- mutation boundaries.

Supervisor may not configure:
- exemption from system invariants;
- permission to self-accept;
- user-visible publication authority;
- protected reviewer-context visibility.
```

---

# 8. Organizational compilation pipeline

The execution path becomes:

```text
problem specification
    │
    ▼
system invariants + human authority
    │
    ▼
role templates + capability contracts
    │
    ▼
system environment + selected reusable profiles
    │
    ▼
specific role environments + workflow context
    │
    ▼
authorized organizational changes
    │
    ▼
organizational compiler
    │
    ▼
deterministic resolution + structural validation
    │
    ▼
semantic obligations + candidate findings
    │
    ▼
attributed judgment + required authority decisions
    │
    ▼
outcome-reachability assessment
    │
    ▼
persist problem-level execution envelope
    │
    ▼
emit role-scoped projections
    │
    ▼
bind runtimes and activate roles
```

The envelope should be durable enough for:

- recovery;
- receipts;
- audit;
- forensic replay;
- runtime reconstruction;
- explanation of which organization existed, why each component and relation
  existed, and what every agent could see or do at the time.

“Compiler” describes composition into an immutable problem-level artifact. It
does not imply that all organizational validity is mechanically decidable. The
compiler may verify declared identities, relations, boundaries, references,
digests, and approval artifacts. It may surface semantic obligations and refuse
to emit while required evidence or decisions are absent. It does not decide
novel questions of authority, ownership, independence, conditionality, or
contract equivalence.

---

# 9. Execution envelope as an immutable organizational artifact

Once an organization is activated, the exact problem-level envelope should
normally be versioned or immutable. Role projections must identify the envelope
revision from which they were compiled.

If the organization changes during execution, record a new revision and derive
new affected projections.

Example:

```text
slice-17 envelope rev 4
    ↓ supervisor adds architecture observation
slice-17 envelope rev 5
```

This allows receipts to record:

```yaml
execution_envelope:
  id: envelope-slice-17
  revision: 5
  digest: ...

role_projection:
  role_instance: builder-17
  source_envelope_revision: 5
  digest: ...
```

That answers:

> What organization was compiled for this problem?

and:

> What exact projection did this role activation receive?

---

# 10. Separate identities

A complete forensic identity may eventually include:

```text
problem identity
contract identities
execution-envelope identity
role-projection identity
runtime identity
```

Example:

```yaml
problem:
  id: slice-17
  objective_digest: ...

contracts:
  role.builder:
    revision: 8
    digest: ...

execution_envelope:
  id: envelope-slice-17
  revision: 5
  digest: ...

role_projection:
  role_instance: builder-17
  digest: ...

runtime:
  provider: codex
  binding_ref: runtime-binding:builder-12:7
```

These identities answer different questions.

### Problem identity

Which objective, scope, and authorized amendments was the organization compiled
to serve?

### Contract identities

Which semantic role and capability definitions were instantiated?

### Execution-envelope identity

What complete organizational structure was active?

### Role-projection identity

What effective affordances, restrictions, relationships, and obligations did
this role instance receive?

### Runtime identity

Where did the computation execute?

---

# 11. Organizational completeness and outcome reachability

A structurally valid envelope can still be organizationally or operationally
invalid.

Example:

```text
Builder objective:
    implement accepted slice

Envelope:
    repository mutation removed
```

The role may remain within authority boundaries but can no longer satisfy its objective.

The validator should surface:

```text
Invalid execution environment.

Required consequence:
    implement accepted slice

No realization path remains.

Missing effective affordance:
    repository mutation
```

This is not ordinary permissions validation.

It is **capability-space validation**.

The system should try to preserve legitimate solution space without prescribing a route.

At the problem level, the compiler should also surface candidate organizational
failures such as:

- a required consequence with no owner;
- conflicting exclusive ownership;
- responsibility assigned without a viable realization path;
- information required by a role that cannot legally reach it;
- an acceptance role whose evidence path violates required independence;
- a transition with no authorized initiator or mediator;
- a capability included without an organizational purpose; or
- an envelope change authorized by a role that cannot grant it.

---

# 12. Organizational envelope validation

Candidate validation questions include:

- Does every required consequence have a clear owner and viable realization
  path?
- Are ownership, delegation, collaboration, and information-flow edges mutually
  compatible?
- Does the envelope violate a system invariant?
- Does it violate a role's essential contract?
- Is each granted capability defined?
- Is the granting role authorized to make the change?
- Does an added observation violate independence?
- Does a mutation boundary exceed real ownership/authority?
- Does removing machinery make a required consequence unreachable?
- Does adding machinery create an authority leak?
- Are required collaborators still available?
- Are selected variant implementations compatible with the capability contract?
- Does the environment preserve the original objective?
- Does this change require contract-owner approval?

Deterministic tooling can identify structural inconsistencies and verify that
required judgment or approval artifacts are present. It cannot promote a
candidate semantic finding into truth.

Semantic classification remains model-judged and attributed. Contract changes
remain human-authorized by the owning contract. An unknown classification fails
closed rather than being resolved through an expansive default.

---

# 13. Skills remain engineered components

This architecture does not require a general skill editor.

It also should not collapse roles and skills into one ontology merely because
both currently use `SKILL.md` as a delivery format.

```text
role template
    parameterizable organizational function
    objective + consequences + ownership + authority + relationships

skill / capability contract
    reusable operational machinery
    inputs + outputs + effects + invariants + implementations
```

A role may invoke or be realized partly through several skills. A skill may be
available to several role templates. Some existing packages combine both
responsibilities; the durable model should still represent the two semantic
facets separately.

A skill remains an engineered component:

```text
skill/
├── SKILL.md
├── scripts/
├── references/
└── assets/
```

Its stable implementation defines its basic functionality.

Examples:

```text
durable-state
    opaque durable CAS publication

slice-checkpoint
    attributed recoverable checkpoint machinery

independent-review
    fresh adversarial evidence generation
```

The UI does not rewrite those implementations.

It configures how roles receive and use them.

---

# 14. Core skill prose must avoid mutable environment claims

If a fact is expected to vary through environment configuration, immutable skill prose should refer to it abstractly.

Bad:

> Use Claude for independent review.

Better:

> Use the configured independent-review capability.

Bad:

> The Builder may modify `src/` and `tests/`.

Better:

> Mutate only within the task-owned boundary provided by the effective environment.

Bad:

> Always use indexed repository evidence.

Better:

> Use the configured repository-evidence machinery and choose an evidence route appropriate to the claim.

This can become a lint target:

> **Stable prose must not assert mutable topology or configuration as structural truth.**

---

# 15. Variants and invariants

The role's compiled static definition—system environment, selected profiles,
and specific role environment—is built from two broad categories.

## Invariant structure

Defines invalid worlds that must remain excluded.

Examples:

- user objective remains authoritative;
- reviewer independence;
- task ownership boundaries;
- acceptance conditions;
- publication authority.

## Variant structure

Defines current machinery and configurable affordances.

Examples:

- review provider;
- repository-evidence provider;
- available tools;
- model/profile;
- optional integrations;
- state projections;
- runtime adapter;
- checkpoint mechanism.

The execution envelope instantiates both:

- invariants remain binding;
- variant machinery is selected/configured;
- permissions are adjusted for context.

---

# 16. Role Environment Graph implications

The current Agent Environment Graph can naturally evolve into an intermediate
representation and debugger for compiled organizations. It can represent:

```text
system environment
selected reusable profiles
specific role environment
problem-level execution envelope
role-scoped envelope projection
runtime realization
```

The top-level graph should show instantiated roles, ownership, delegation,
information flow, capabilities, acceptance relationships, mediated transitions,
and hard prohibitions. Each role page can then show its lossless projection and
a semantic diff from the role's baseline environment.

Example:

```text
SLICE BUILDER

Baseline environment

Observe
  repository
  accepted scope

Invoke
  repository evidence
  independent review

Mutate
  task-owned changes


Current role projection

+ architectural review
+ proposal authority projection
- independent review
~ mutation boundary → 3 paths
~ repository evidence → direct source
```

This gives humans both an organizational view of the particular problem and an
intuitive explanation of each role's contextual configuration.

---

# 17. Control-plane implications

The control plane can operate on execution envelopes without owning role semantics.

Possible control flow:

```text
Supervisor judgment
      ↓
proposed organizational change
      ↓
authority validation
      ↓
organizational and environment validation
      ↓
compile and persist envelope revision
      ↓
derive affected role projections
      ↓
control plane activates / updates runtimes
```

The control plane coordinates activation and delivery.

It does not decide whether the environment is semantically valid.

---

# 18. Runtime-adapter implications

The execution envelope remains separate from provider runtime binding.

Example:

```text
Role contract:
    Builder

Execution envelope:
    review disabled
    mutation scope = A/B/C

Runtime:
    Codex thread 019f
    active turn 019f
```

A runtime rebind does not alter the envelope.

An envelope change does not redefine the logical actor.

This preserves:

```text
semantic continuity
≠
environment configuration
≠
provider execution continuity
```

---

# 19. Scheduler implications

Scheduled obligations can target a logical role or actor.

When the obligation becomes due:

```text
scheduler
    ↓
workflow/control owner
    ↓
current execution envelope
    ↓
authority validation
    ↓
runtime binding
    ↓
provider delivery
```

The scheduled item should not embed a stale provider runtime identity.

The current envelope and runtime binding are resolved at delivery time.

---

# 20. UI implications

The future Studio does not need a full skill editor.

It can expose three primary views.

## Contract view

Mostly locked.

Shows:

- objective;
- required consequences;
- essential invariants;
- intrinsic authority;
- essential interfaces.

Changes become proposals.

## Organization view

Editable where authorized.

Shows:

- instantiated roles and their parameters;
- ownership, delegation, collaboration, and acceptance relationships;
- capabilities;
- observations;
- mutation boundaries;
- provider/model profiles;
- optional machinery;
- context-specific permissions.

The former “permission editor” is one focused panel within this view. Edits
produce proposed organizational changes, which must be authorized, validated,
compiled, and persisted before they affect an activation.

## Runtime view

Observed.

Shows:

- provider binding;
- loaded/active status;
- capability availability;
- current turn;
- runtime descendants;
- waiting conditions.

---

# 21. Example UI mental model

```text
Slice Builder

CONTRACT
────────
Objective: implement one accepted engineering slice
Independent acceptance required
Cannot publish user-visible history

ENVIRONMENT
───────────
Repository evidence     [direct-source ▼]
Independent review      [disabled]
Architecture analysis   [enabled]

Observe
✓ repository
✓ accepted scope
✓ architectural review

Mutate
✓ src/parser.py
✓ tests/test_parser.py

RUNTIME
───────
Codex                  active
Runtime thread         019f...
Gate capability        available
Claude review          unavailable
```

The editor changes environment state.

It does not rewrite the skill's essential contract.

---

# 22. Configuration-change authority

Environment changes should carry provenance.

Example:

```yaml
change:
  envelope: builder-12-env
  from_revision: 4
  to_revision: 5

  requested_by:
    role: role.supervisor
    logical_actor_id: supervisor-4

  authority_ref: ...

  changes:
    - add observation artifact.architectural_review
    - narrow mutation scope
```

This makes configuration changes first-class semantic events.

---

# 23. Dynamic reconfiguration during execution

A role's envelope may change while its context is still alive.

Examples:

- reviewer finding reveals a need for broader evidence;
- architectural review becomes relevant;
- scope changes through authorized amendment;
- provider becomes unavailable;
- a capability is replaced;
- a task becomes read-only during investigation;
- remediation needs a different mutation boundary.

The role does not necessarily need to restart.

The environment can receive a new revision if the runtime supports safe update.

Whether context must restart is a separate judgment based on:

- independence;
- stale premises;
- provider constraints;
- authority;
- semantic discontinuity.

Do not encode a blanket restart rule merely because an envelope changed.

---

# 24. Proposed terminology

Recommended terms:

### Problem specification

Objective, scope, required consequences, evidence, constraints, and authorized
amendments for the concrete work.

### Role template

Parameterizable organizational function with stable semantic identity and
required guarantees.

### Skill / capability contract

Reusable operational machinery with declared inputs, outputs, effects,
invariants, implementations, and configurable dimensions.

### System environment

Universal role bindings and boundaries that apply to every Work Engine role
without redefining their referenced invariants or capabilities.

### Reusable role profile

Composable structural bindings for an explicitly selected class of roles. A
profile supplies shared constraints, requirements, and machinery relationships
without becoming a role identity, granting authority, or owning role-specific
state meaning.

### Specific role environment

Normal/configurable role structure and machinery.

### Execution envelope

Compiled problem-level organizational structure for one problem or bounded
problem lifecycle.

### Role projection

The lossless role-scoped view of one execution-envelope revision delivered to a
particular role instance.

### Organizational compiler

The boundary that combines the problem specification, invariants, templates,
capability contracts, system environment, reusable profiles, specific role
environments, context, and authorized changes;
performs deterministic resolution and structural validation; coordinates
required semantic judgments and authority decisions without owning them; and
emits an immutable execution envelope and projections.

### Runtime projection

Observed provider/runtime reality.

### Configuration authority

Authority to propose changes that configure a role instance or relationship in
the compiled execution envelope.

### Environment revision

Versioned effective envelope.

---

# 25. Compact model

```text
                 PROBLEM SPECIFICATION
                           │
                SYSTEM INVARIANTS + AUTHORITY
                           │
          ROLE TEMPLATES + CAPABILITY CONTRACTS
                           │
       SYSTEM ENVIRONMENT + REUSABLE PROFILES
                           │
       SPECIFIC ROLE ENVIRONMENTS + CURRENT CONTEXT
                           │
             AUTHORIZED ORGANIZATIONAL CHANGES
                           │
                           ▼
                ORGANIZATIONAL COMPILER
                           │
                           ▼
             PROBLEM-LEVEL EXECUTION ENVELOPE
                           │
             ┌─────────────┼─────────────┐
             ▼             ▼             ▼
       ROLE PROJECTION ROLE PROJECTION CAPABILITY BINDINGS
             │             │             │
             └─────────────┼─────────────┘
                           ▼
                 RUNTIME PROJECTIONS
```

---

# 26. Central architectural statements

> **A role template is parameterizable organizational machinery.**

> **A skill or capability contract defines reusable operational machinery.**

> **The system environment binds structure that genuinely applies to every
> role.**

> **A reusable role profile composes shared structural relationships without
> creating role identity or authority.**

> **A specific role environment defines that role's normal configurable
> world.**

> **An execution envelope is the compiled organizational structure for a
> particular problem.**

> **A role projection defines the world one instantiated role actually
> inhabits within that organization.**

> **Runtime state defines what is available inside that world right now.**

> **Context may expand, contract, replace, or specialize affordances as long as the resulting environment remains semantically valid and appropriately authorized.**

> **One role may configure another role instance or organizational relationship
> only through explicit configuration authority.**

> **Changing an execution envelope does not change its source role templates or
> capability contracts.**

> **Changing a role template's essential contract is a proposal-level
> architectural change.**

---

# 27. Why this matters

This architecture gives Work Engine a practical way to compile fit-for-problem
organizations from reusable role templates and capabilities without collapsing
roles into mutable prompt blobs.

It supports:

- safer hand-edited YAML configuration;
- future visual environment editing;
- context-sensitive capabilities;
- dynamic authority;
- provider substitution;
- smaller launch contracts;
- durable environment identity;
- richer runtime control;
- better forensic replay;
- structural validation;
- organizational completeness and consequence-reachability analysis;
- role-scoped projections from one shared organizational truth;
- proposal-level protection for true contract changes.

Most importantly, it preserves the core Work Engine design principle:

> **Pin what must remain true, provide the machinery needed for legitimate outcomes, and let the model exercise judgment inside the valid space.**

The execution envelope becomes the explicit, inspectable compiled organization
for one concrete problem. Role projections make each participant's valid space
legible without fragmenting the organizational source of truth.

---

# 28. The organizational compiler has no grant authority

The compiler is deterministic organizational assembly machinery, not an actor
that possesses or delegates authority. It must never invent a role, grant,
relationship, exemption, or semantic classification merely because the result
appears useful for the problem.

Every organizational change must identify:

- the requesting human or logical actor;
- the source-envelope revision, when revising an active organization;
- the changed parameters or relations;
- the authority reference permitting those changes;
- the evidence and attributed judgment supporting semantic classifications;
- whether the change stays inside an existing configurable dimension or changes
  a contract; and
- any required contract-owner approval.

The compiler contract should require it to:

- consume only canonical contracts and attributed change requests;
- preserve field-level source and decision provenance;
- fail closed on unknown component, parameter, relation, and authority classes;
- avoid defaults that silently expand authority;
- verify mechanically decidable references, shapes, digests, and approval
  presence;
- emit semantic obligations instead of deciding them;
- produce an immutable candidate envelope before activation; and
- never treat successful structural validation as semantic acceptance.

The authority questions therefore remain separate:

```text
Who owns the assembler contract?

Who authorized each organizational input or change?

Who judged each semantic obligation?

Who approved any resulting contract change?
```

The compiler answers none of these questions. It preserves and validates their
attributed answers.

---

# 29. Proposal research is an upstream prerequisite

A problem-specific organization cannot be responsibly compiled from a thin or
under-researched proposal. Choices about roles, capabilities, information flow,
independence, mutation, validation, and runtime compatibility depend on durable
evidence about the actual problem.

The ordinary campaign supervisor should not reconstruct that evidence or choose
an organization through unsupported repository-domain judgment. The upstream
flow is instead:

```text
raw idea
    ↓
proposal formation
    ↓
evidence-backed proposal research
    ↓
execution qualification
    ↓
organizational requirements
    ↓
authorized organizational proposal
    ↓
compiled execution envelope
    ↓
campaign supervision
```

The proposal packet should preserve or reference enough evidence to establish:

- the problem and causal parent;
- intended consequences and non-goals;
- architectural placement and credible competing owners;
- affected contracts and invariants;
- dependencies and related proposals;
- required and conditional capabilities;
- ownership, independence, information-flow, and mutation requirements;
- validation and acceptance consequences;
- risks, reversibility, and unresolved uncertainty; and
- evidence provenance, freshness, limitations, and confidence.

The packet should express organizational requirements before prescribing one
provider or topology. An authorized planning judgment may then select compatible
role templates and capability implementations. Provider binding remains a
runtime linking decision unless provider identity is itself contractual.

Mechanical packet validity is not execution readiness. Research maturity,
readiness profiles, evidence snapshots, and staleness are developed in
[Research Maturity, Evidence Snapshots, and Staleness](./research-maturity-evidence-snapshots-and-staleness.md).

The organizational compiler may consume a readiness judgment only when it names
the canonical owner authorized to make the organizational-compilation decision,
the exact packet and evidence revisions considered, and any required approval.
A researcher, packet validator, or freshness detector cannot supply that
authority merely by producing useful evidence.

---

# 30. Bootstrap research organization

Research needed to qualify a proposal also requires some organization, but this
does not require the final problem-specific organization to exist first.

A small, bounded research organization can gather placement, ownership,
contract, risk, capability, and validation evidence:

```text
minimal proposal-research organization
        ↓
durable evidence and unresolved questions
        ↓
execution-qualified proposal
        ↓
problem-specific organizational compilation
```

The research organization should be stable enough to bootstrap evidence without
presupposing the conclusion of the organizational-design question. Deeper or
specialized research machinery may be added when existing evidence shows that
it has credible decision value.

---

# 31. Adoption boundary

The vocabulary and representation may be useful before general dynamic
organizational compilation is justified. The first implementation should compile
and explain the organization Work Engine already has rather than invent new
organizational topology.

A behavior-preserving first vertical may:

- consume the existing fixed role contracts and campaign configuration;
- separate universal system bindings, one reusable profile, and one concrete
  role definition while preserving current behavior;
- produce an immutable effective-envelope artifact;
- retain exact provenance for selected capabilities, providers, scope, and
  authority;
- derive role-scoped projections;
- distinguish required, authorized, and runtime-available capabilities; and
- render baseline-versus-effective differences for Studio and audit use.

The first profile should prove reuse across at least two compatible role
definitions or establish why its shared ownership is still preferable to
duplication. Compilation should retain field-level source provenance and reject
an attempted profile composition that weakens authority, ownership, mutation,
visibility, independence, or lifecycle constraints.

It should not initially:

- create arbitrary new roles;
- synthesize novel authority delegation;
- claim general semantic type checking;
- require continuous research dependency-surface monitoring;
- dynamically restructure active campaigns; or
- make contract-affecting decisions without their existing owner.

Observed limitations in that fixed-topology vertical can supply the causal
parent for later support such as parallel builders, specialist roles, dynamic
delegation, or mid-run organizational revision. A compelling metaphor alone is
not sufficient authority to build those mechanisms.
