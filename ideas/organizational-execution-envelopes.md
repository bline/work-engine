# Problem-Derived Organizational Execution Envelopes

## Status

Exploratory architecture direction.

## Idea

Treat roles as reusable, parameterizable organizational functions and skills as reusable capability machinery. For sufficiently qualified work, assemble an immutable **problem-level execution envelope** describing the organization that will perform the work.

Each instantiated role receives a role-scoped projection of that envelope.

## Current evidence

The current Work Engine has a fixed, implemented topology:

```text
Slice Supervisor
    ↓
one Slice Builder
    ├── repository evidence
    ├── independent review
    └── deterministic gates
```

The Agent Environment Graph already represents role contracts, capabilities, observations, mutations, prohibitions, and mediated transitions as structured data.

Proposal research and evidence-lineage work are beginning to establish the upstream evidence required for context-specific organizational decisions.

No general organizational compiler or dynamic team manager exists today.

## Semantic components

### System environment

A structured definition of consequences and machinery genuinely available to
every role. It may bind:

- global invariant references and authority boundaries;
- the non-authoritative, ephemeral status of model context;
- common provenance requirements;
- globally available capabilities and machinery; and
- defaults whose source and override authority remain explicit.

The system environment references canonical invariant and capability identities.
It does not duplicate their definitions or make available machinery mandatory.

### Reusable role profile

Composable structural configuration shared by a declared class of roles, such
as durable operational recovery, independent-review isolation, repository
mutation, or scheduled-role obligations.

A profile is neither a role identity nor a capability implementation. It binds
shared consequences and capability requirements while leaving role-owned state
meaning with the concrete role or workflow.

### Role template

A stable organizational function with:

- objective;
- required consequences;
- ownership;
- authority relationships;
- interfaces;
- intrinsic prohibitions;
- configurable dimensions.

### Skill / capability contract

Reusable operational machinery with:

- inputs and outputs;
- effects;
- intrinsic guarantees;
- supported implementations;
- configurable dimensions.

A skill is not automatically a role.

### Problem specification

The accepted objective, scope, evidence, constraints, authority, and unresolved uncertainty for the concrete problem.

### Execution envelope

The compiled organization for one bounded problem lifecycle:

- instantiated roles;
- ownership and delegation;
- information-flow boundaries;
- observations and mutation authority;
- collaboration and mediation;
- required consequences;
- capability requirements and selected realizations;
- context lifetimes and independence boundaries;
- limits, escalation, and stop conditions;
- provenance for every material organizational choice.

### Role projection

One role instance's authorized view of the envelope.

### Runtime binding

The provider/process realization used to execute a role or capability. Runtime identity is not organizational identity.

## Candidate structured definition layer

The source architecture should keep global structure, reusable profiles, and
concrete role identity distinct. One illustrative layout is:

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

The paths are provisional. The semantic layers are the proposal:

```text
canonical invariant/capability definitions
        ↓ referenced by
system environment
        + selected reusable profiles
        + concrete role definition
        + authorized problem-level changes
        ↓ compiled into
effective role projection / execution envelope
```

For example, a system environment could declare context to be ephemeral and
non-authoritative while making durable-state and agent-state capabilities
available. A `durable-role` profile could then require the consequence that a
replacement context recover role-owned state, refresh authoritative references,
reconcile uncertain actions, and bind a new context generation before
consequential continuation.

That recovery consequence is owned by the formed role-owned durable
operational-state proposal. The profile owns only which class of roles receives
the binding; it does not redefine the consequence or a role's state schema. The
same proposal also owns the continuous-publication boundary: resume-critical
consequences become durable when losing them becomes materially unsafe, rather
than waiting for a phase boundary, handoff, or anticipated context compaction.

## Profile composition and configuration authority

Composition should be deterministic, provenance-bearing, and fail closed on a
semantic conflict. Every effective field should retain the identity and revision
of its contributing source.

Profiles may add compatible constraints, requirements, references, and default
machinery. They may not independently:

- create a role objective or semantic identity;
- redefine a referenced invariant or capability;
- grant authority unavailable to the role contract;
- weaken an intrinsic prohibition;
- transfer state or artifact ownership;
- make an available capability required merely because it exists; or
- resolve conflicting ownership or authority by last-file-wins precedence.

The definition model must keep these states distinct:

- **available** — machinery the environment exposes;
- **authorized** — machinery or effects the governing contract permits;
- **required** — consequences or capabilities necessary for this role/problem;
- **selected** — the realization chosen for this envelope.

An envelope may narrow an authorized affordance. Expansion requires explicit
configuration authority whose scope permits that exact change. Conflicting
authority, ownership, visibility, mutation, independence, or lifecycle claims
require the owning authority or fail compilation.

## Organizational assembly has no grant authority

Deterministic assembly may:

- resolve references;
- apply declared composition rules;
- verify mechanically decidable constraints;
- preserve provenance;
- refuse missing required judgments or approvals;
- emit immutable candidate envelopes.

It may also explain the source and authority of every effective field and reject
an envelope whose required outcome is unreachable with the selected roles and
capabilities.

It may not invent:

- authority;
- roles;
- exemptions;
- semantic ownership;
- independence conclusions;
- contract changes.

Judgment and human authority remain separate inputs.

## Upstream prerequisite

Problem-specific organizational assembly requires an evidence-backed, sufficiently mature proposal or equivalent problem specification.

The packet should usually express **organizational requirements before provider choices**:

```text
requires independent falsification
requires targeted mutation
requires deterministic validation
requires architectural observation
```

rather than:

```text
use Claude
use model X
```

Provider/runtime realization is a later linking problem unless identity itself is contractual.

## Adoption boundary

A first implementation should compile and explain the organization Work Engine already has.

It should not initially:

- invent arbitrary new roles;
- synthesize novel authority;
- dynamically restructure active campaigns;
- claim general semantic type checking.

Fixed-topology envelope compilation can establish whether the representation is useful before dynamic organization is justified.

The first vertical should therefore exercise `system + profiles + role` source
composition for the existing supervisor/builder topology, including a durable
role profile, before attempting dynamic team synthesis.

## Does not own

This idea does not own:

- proposal research;
- the control plane;
- runtime adapters;
- Studio UI;
- role-owned durable state;
- review artifact semantics.

Those are inputs, consumers, or adjacent layers.

## Compact statement

> An execution envelope is the immutable, evidence-backed organizational structure compiled for one concrete problem; each agent receives only its authorized role projection.
