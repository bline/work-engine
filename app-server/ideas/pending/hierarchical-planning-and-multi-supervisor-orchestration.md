# Hierarchical Planning and Multi-Supervisor Orchestration

## Status

Formed architectural direction.

This document proposes moving operator-facing workflow coordination one layer above the current slice supervisor and introducing hierarchical planning for concurrent multi-agent engineering.

The proposal is primarily a **workflow and role-boundary change**. It is intended to reuse the existing Work Engine infrastructure for role realization, durable state, isolated workspaces, checkpoints, review, receipts, context lifecycle, and mediated effects rather than introduce a second execution substrate.

The central change is:

> Plan the topology of the work first. Then plan and execute each coherent branch independently under its own supervisor.

---

## Motivation

The current supervisor model was formed around one coherent engineering campaign.

That topology works well when one supervisor owns one bounded body of work:

```text
operator
   |
supervisor
   |
builder
   |
review / gates / checkpoint
```

As Work Engine moves toward concurrent multi-agent development, the supervisor becomes an increasingly poor place to host the operator interface.

A single supervisor coordinating several independent engineering branches would need to retain:

- global project state;
- multiple implementation contexts;
- cross-branch dependencies;
- several builder lifecycles;
- several review/remediation loops;
- operator interaction;
- provider and resource constraints;
- integration state; and
- replanning consequences.

That recreates the context-concentration problem that Work Engine already addresses at lower levels.

The desired architecture instead decomposes planning and execution hierarchically.

---

# Target topology

```text
                            OPERATOR
                               |
                               v
                          PREPLANNER
                               |
                     orchestration plan
                               |
                               v
                         ORCHESTRATOR
                    /         |         \
                   /          |          \
                  v           v           v
             PLANNER A   PLANNER B   PLANNER C
                  |           |           |
             branch plan  branch plan  branch plan
                  |           |           |
                  v           v           v
          SUPERVISOR A SUPERVISOR B SUPERVISOR C
                  |           |           |
             builder(s)  builder(s)  builder(s)
                  \           |           /
                   \          |          /
                    review / gates /
                 checkpoints / evidence
```

The operator interacts primarily with the orchestrator during execution.

The preplanner determines the topology.

Branch planners determine implementation plans.

Supervisors execute accepted branch plans through bounded slices.

Builders perform repository-domain implementation judgment.

Reviewers challenge immutable implementation subjects.

Existing deterministic and server-owned mechanisms continue to enforce mechanically decidable boundaries.

---

# Core architectural relation

The architecture contains the same planning/execution relationship at two levels:

```text
Preplanner : Orchestrator
        ≈
Planner : Supervisor
```

At the upper level:

- the **preplanner** defines a bounded structure of work;
- the **orchestrator** realizes that structure without acquiring planning authority.

At the branch level:

- a **planner** defines a bounded implementation space;
- a **supervisor** realizes that plan through slices without acquiring implementation judgment.

This preserves the Work Engine principle:

> Define the space, not the solution.

Planning establishes structure and consequences.

Execution retains freedom inside that structure.

---

# 1. Preplanner

## Objective

Transform an accepted strategic objective or body of authorized work into an **orchestration topology** composed of coherent workstreams.

The preplanner reasons primarily about relationships between pieces of work rather than detailed implementation inside each piece.

## Owns

The preplanner may determine:

- coherent workstream boundaries;
- dependency relationships;
- concurrency opportunities;
- shared contracts and invariants;
- cross-branch authority constraints;
- integration points;
- required branch outputs;
- dependency-release consequences;
- which responsibilities require distinct supervisors;
- branch planning packets;
- topology-level stop conditions;
- topology invalidation conditions; and
- conditions requiring later replanning.

## Does not own

The preplanner does not:

- produce detailed implementation plans for every branch;
- supervise builders;
- perform repository mutations;
- accept branch implementations;
- evaluate implementation-review findings;
- control publication;
- infer that one branch's implementation route is required merely because it satisfies the topology.

## Output

The primary artifact is a revision-bound **orchestration plan**.

Conceptually:

```yaml
objective: ...

workstreams:

  runtime-core:
    objective: ...
    depends_on: []
    authority_ceiling: ...
    planner_packet: ...
    releases:
      - artifact: runtime-contract
        consequence: provider runtime contract established

  builder-realization:
    objective: ...
    depends_on:
      - runtime-contract
    authority_ceiling: ...
    planner_packet: ...

  reviewer-research:
    objective: ...
    depends_on: []
    authority_ceiling: ...
    planner_packet: ...

  integration:
    objective: ...
    depends_on:
      - builder-realization
      - reviewer-research
    planner_packet: ...
```

The exact schema is an implementation concern.

The semantic requirement is that the topology make independent work, dependencies, release conditions, and replanning boundaries explicit.

---

# 2. Orchestration plan

The orchestration plan defines the **space of execution across workstreams**.

It should contain enough structure that ordinary orchestration is mechanically decidable wherever possible.

Its responsibilities include:

- workstream identities;
- workstream objectives;
- initial dependency graph;
- branch-planning inputs;
- authority ceilings;
- shared invariants;
- required external contracts;
- dependency-release evidence;
- integration workstreams;
- global resource constraints;
- human decision points;
- replanning triggers;
- terminal conditions; and
- reconstruction expectations.

The orchestration plan should express dependencies in terms of required consequences rather than incidental process completion.

Prefer:

```text
Branch B requires:
    accepted runtime-provider contract R
```

over:

```text
Branch B requires:
    Branch A to finish
```

The first preserves route independence.

A different branch or revised implementation may satisfy the same consequence without changing dependent work unnecessarily.

---

# 3. Orchestrator

## Objective

Realize an accepted orchestration topology by coordinating planners, supervisors, dependencies, and operator decisions without acquiring the semantic judgment owned by those actors.

The orchestrator becomes the primary **operator-facing execution role**.

It is not a larger supervisor.

## Owns

The orchestrator may:

- load and recover an accepted orchestration plan;
- determine which declared workstreams are mechanically runnable;
- create or resume branch-planner instances;
- present branch plans and other required decisions to the operator;
- record exact human decisions through their durable owner;
- create or resume supervisors after branch-plan admission;
- pause, replace, or retire supervisor instances according to declared lifecycle rules;
- route exact artifacts between branches;
- observe dependency-release evidence;
- update rebuildable orchestration projections;
- surface blocked branches;
- coordinate global concurrency and resource admission through their authoritative services;
- request replanning when the accepted topology becomes invalid;
- aggregate truthful portfolio status; and
- determine when the orchestration plan's terminal consequences have been established.

## Does not own

The orchestrator does not decide:

- implementation architecture inside a branch;
- whether a builder's implementation is technically correct;
- whether a review finding is valid;
- whether remediation is sufficient;
- whether a semantic contract may change;
- whether scope should expand;
- how two implementations should be integrated when integration requires domain judgment;
- whether missing evidence is "probably good enough"; or
- any human-owned approval.

The orchestrator routes those questions to their owners.

---

# 4. Operator interface

The operator interface moves from the branch supervisor to the orchestrator.

This does **not** transfer human authority to the orchestrator.

The desired relation is:

```text
Operator
    ^
    | natural-language interaction
    v
Orchestrator
    ^
    | typed/revision-bound decisions and evidence
    v
Durable Work Engine state and authority owners
```

The orchestrator may explain:

- what is currently running;
- what is blocked;
- what changed;
- which branches can proceed;
- what decision is required;
- what evidence supports the decision request; and
- what consequences follow from each authorized option where those consequences are already established.

A human decision must become an exact durable input.

It must not remain authoritative merely because the orchestrator remembers the conversation.

---

# 5. Branch planner

Each workstream receives a fresh branch-planning context.

## Objective

Produce a coherent implementation plan for one workstream within the topology, authority ceiling, dependencies, and shared contracts established by the orchestration plan.

## Inputs

A planner should receive only the material required by its branch, including as applicable:

- branch objective;
- orchestration-plan revision;
- delegated authority;
- repository subject;
- satisfied dependency artifacts;
- shared invariants;
- cross-branch contracts;
- required terminal consequences;
- applicable resource constraints;
- planning evidence;
- branch stop conditions; and
- known topology-sensitive assumptions.

## Owns

The branch planner may reason deeply about:

- repository placement;
- architecture;
- implementation consequences;
- candidate components;
- local sequencing;
- validation requirements;
- review requirements;
- migration consequences;
- compatibility boundaries; and
- branch-specific stopping conditions.

## Does not own

A branch planner cannot silently revise the orchestration topology.

If planning discovers that the branch requires:

- authority owned by another branch;
- modification of a shared contract;
- a previously undeclared dependency;
- a materially different workstream split;
- integration with another branch before its declared release point; or
- scope outside the topology grant,

the planner returns a topology conflict rather than absorbing the change.

---

# 6. Branch plan

The branch implementation plan defines the **space of execution inside one workstream**.

It should preserve the existing Work Engine planning model:

- exact objective;
- accepted authority and mutation boundary;
- repository evidence cutoff;
- affected semantic owners;
- candidate implementation surface;
- required consequences;
- deterministic evidence;
- semantic or behavioral evidence;
- review requirements;
- compatibility expectations;
- effective instruction boundary where relevant;
- human decisions;
- stop conditions; and
- terminal evidence.

Acceptance of the orchestration topology does not automatically accept branch implementation plans.

These are distinct decisions.

Conceptually:

```text
accept orchestration plan
        =
"these workstreams and relationships are authorized for planning"

accept branch plan
        =
"this bounded branch implementation is authorized to proceed"
```

---

# 7. Supervisor

The supervisor becomes deliberately local.

## Objective

Realize one accepted branch plan through bounded engineering slices and bring that workstream to truthful local terminal states.

## Owns

The branch supervisor retains responsibility for:

- branch-local campaign state;
- slice selection;
- slice planning boundaries where still applicable;
- builder lifecycle;
- builder continuity;
- deterministic gates;
- checkpoint lifecycle;
- configured review selection;
- finding delivery;
- remediation loops;
- local recovery;
- local completion evidence; and
- branch-level receipts.

## Moves upward

Responsibilities that concern the portfolio rather than one branch move to the orchestrator or an existing server service, including:

- primary operator interaction;
- supervisor creation and retirement;
- cross-supervisor dependencies;
- global workstream status;
- global provider budgets;
- cross-workstream resource coordination;
- orchestration-plan completion;
- topology-level replanning; and
- decisions about which independent workstreams should execute concurrently.

The supervisor does not need to understand the entire project.

It needs to understand its branch.

---

# 8. Builder

The builder remains the repository-domain implementation actor.

This proposal does not change the essential builder boundary.

The builder:

- receives bounded implementation responsibility;
- understands the relevant repository deeply;
- chooses an implementation route inside the accepted structure;
- mutates only within authorized boundaries;
- evaluates findings assigned to the builder;
- validates its work;
- stops on unresolved scope or authority conflicts; and
- produces exact implementation evidence.

Multi-agent scaling should preferentially create **more coherent branches and bounded builders**, not increase the amount of unrelated reasoning one builder must retain.

---

# 9. Review

Review remains orthogonal to orchestration.

The orchestrator may coordinate review availability or observe review state, but it does not become the reviewer.

Branch supervisors retain the local implementation-review lifecycle unless a later architecture explicitly moves that responsibility.

Review subjects remain immutable and attributable.

Reviewer independence, retained remediation, provider realization, evidence class, and acceptance authority continue to be governed by their existing owners.

---

# 10. Cross-branch dependencies

Dependencies should be released by **evidence-bearing consequences**.

Example:

```text
Runtime branch
      |
      | produces accepted runtime-contract-v1
      v
Orchestrator observes release consequence
      |
      v
Builder-realization branch becomes runnable
```

The orchestrator should not infer:

> Runtime branch looks mostly done, so the dependent branch can probably start.

The orchestration plan defines what evidence releases the dependency.

Where partial concurrency is safe, the preplanner may express narrower release points rather than forcing whole-branch completion.

---

# 11. Integration as a workstream

The orchestrator must not become the semantic integrator of parallel engineering work.

Where multiple branches require judgment-bearing integration, the preplanner should declare an explicit integration workstream.

For example:

```text
Branch A ──┐
Branch B ──┼──> Integration D
Branch C ──┘
```

`Integration D` receives its own:

```text
planner
   ->
branch plan
   ->
supervisor
   ->
builder
   ->
review / evidence
```

The orchestrator only determines that the declared prerequisites for Integration D have become satisfied.

It does not decide how the implementations should be reconciled.

---

# 12. Topology conflict and replanning

Execution will sometimes falsify a premise used by the preplanner.

That is expected.

A branch actor may discover:

```text
the required contract cannot be produced as assumed

a branch must change shared semantics

a missing dependency exists

two supposedly independent branches are coupled

an integration boundary is wrong

a required authority was not delegated
```

The route is:

```text
Branch planner / supervisor
        |
        | typed topology conflict
        v
Orchestrator
        |
        | current execution evidence
        v
Preplanner
        |
        | revised topology proposal
        v
Operator / owning authority
        |
        | accepted revision
        v
Orchestrator
```

The orchestrator does not repair the topology itself.

The preplanner does not silently rewrite an active orchestration plan.

A changed topology becomes a new revision with explicit consequences for active branches.

Unaffected branches should remain valid whenever their premises and authority have not changed.

---

# 13. Context locality

A primary benefit of this architecture is context separation.

Each layer retains only the context appropriate to its judgment.

```text
Preplanner
    understands topology and cross-workstream meaning

Orchestrator
    understands execution state and dependency structure

Branch planner
    understands one branch deeply enough to plan it

Supervisor
    understands one branch's execution lifecycle

Builder
    understands one bounded implementation problem

Reviewer
    understands one exact review subject
```

No role should receive broader context merely because it is available.

The goal is not maximal isolation.

The goal is **causal context locality**: each actor receives enough information to make the judgments it owns without accumulating unrelated reasoning state.

---

# 14. Concurrency model

The orchestration plan makes safe concurrency explicit.

Example:

```text
                     ┌── Runtime Core ───────────┐
                     │                           │
Objective ─ Preplan ─┼── Reviewer Research ─────┼── Integration
                     │                           │
                     └── Pilot Infrastructure ───┘
```

If all three branches have satisfied entry dependencies and independent resource authority, the orchestrator may run three supervisors concurrently.

Concurrency does not imply shared mutation authority.

Existing worktree isolation, resource coordination, fencing, provider admission, and publication boundaries remain authoritative.

The orchestrator coordinates access to those mechanisms rather than replacing them.

---

# 15. Provider realization

Logical orchestration roles should remain distinct from provider realization.

The architecture should permit, for example:

```text
Orchestrator     -> Spark or another economical orchestration model
Planner A        -> Sol
Supervisor A     -> Claude
Builder A        -> Claude

Planner B        -> Sol
Supervisor B     -> Sol
Builder B        -> Sol

Reviewer         -> separately admitted reviewer realization
```

Provider choice does not redefine role semantics or authority.

The exact provider-routing policy is a replaceable runtime concern.

A lightweight model is particularly suitable for the orchestrator when the orchestration plan makes ordinary transitions mechanically decidable.

High-capability reasoning should be concentrated where actual planning, implementation, review, or reconciliation judgment is required.

---

# 16. Durable state

The operator-facing orchestrator must not become a conversational state owner.

Durable orchestration state should be reconstructable independently of any model context.

At minimum, state should identify:

- orchestration-plan revision;
- active workstreams;
- branch-plan revisions;
- planner identities;
- supervisor identities;
- dependency state;
- released artifacts;
- blocked conditions;
- topology conflicts;
- human decisions;
- active resources;
- terminal branch evidence; and
- orchestration terminal state.

Context lifecycle may preserve a role's reasoning continuity.

It does not replace orchestration state.

---

# 17. Authority model

Conceptually:

```text
Human authority
      |
      v
Strategic objective / accepted topology
      |
      v
Preplanner
    planning judgment over work decomposition
      |
      v
Orchestration plan
      |
      v
Orchestrator
    execution coordination
      |
      +-----------------------+
      |                       |
      v                       v
Branch Planner A        Branch Planner B
      |                       |
      v                       v
Branch Plan A           Branch Plan B
      |                       |
      v                       v
Supervisor A            Supervisor B
      |                       |
      v                       v
Builder A               Builder B
```

No arrow transfers all authority of the layer above.

Each artifact grants only the effects explicitly assigned to the receiving layer.

In particular:

- planning does not accept implementation;
- orchestration does not acquire planning judgment;
- supervision does not acquire builder judgment;
- review does not acquire acceptance authority;
- execution success does not imply publication authority.

---

# 18. Initial implementation direction

This architecture should be implemented incrementally and primarily by recomposing existing Work Engine machinery.

A plausible progression is:

## A. Define the orchestration-plan contract

Introduce the minimum representation needed for:

- workstream identity;
- dependencies;
- branch planner inputs;
- release conditions;
- authority ceilings;
- topology conflict;
- human decision points;
- terminal evidence.

Avoid building generalized workflow syntax beyond demonstrated requirements.

## B. Add preplanner role

Create a bounded planning role whose output is the orchestration plan.

Test it initially against one objective that naturally decomposes into at least two independent branches and one dependency or integration point.

## C. Add orchestrator role

Reuse the generic orchestration discipline where applicable.

Give the orchestrator only the capabilities required to:

- observe durable orchestration state;
- launch or resume branch planners;
- launch or resume supervisors;
- route exact packets;
- surface human decisions;
- observe dependency releases;
- request replanning; and
- terminate truthfully.

## D. Move operator interface upward

Route normal multi-branch operator interaction through the orchestrator.

Do not remove the supervisor's current interface until the new boundary has proven equivalent or better consequences.

## E. Bind branch planners to supervisors

Allow an accepted branch plan to instantiate one retained branch supervisor using existing role/runtime machinery.

## F. Demonstrate concurrency

Run two genuinely independent supervisor/builder branches concurrently using existing isolated-worktree and resource-coordination infrastructure.

## G. Demonstrate dependency release

Have one branch produce an exact consequence that mechanically releases another branch.

## H. Demonstrate topology conflict

Exercise one case where a branch discovers that a preplanning premise is invalid.

Prove that:

```text
branch
 -> orchestrator
 -> preplanner
 -> revised topology
```

occurs without the orchestrator silently changing the plan.

## I. Demonstrate integration branch

Run at least one explicit integration workstream dependent on two independently completed branches.

---

# 19. Acceptance evidence

The architecture should not be considered established merely because the roles can be launched.

Useful acceptance evidence includes:

- one objective decomposed by a preplanner into multiple coherent workstreams;
- orchestration-plan validation;
- explicit topology acceptance distinct from branch-plan acceptance;
- two branch planners operating from separate bounded contexts;
- two concurrent supervisors operating without workspace or state collision;
- exact branch-plan-to-supervisor binding;
- one mechanically released cross-branch dependency;
- one blocked dependency that does not execute early;
- one topology conflict returned for replanning;
- one unaffected branch surviving a topology revision when its premises remain valid;
- one explicit integration workstream;
- operator decisions durably recorded outside conversational memory;
- restart and context-replacement recovery of the orchestrator;
- truthful provider and role identities;
- proof that the orchestrator did not acquire builder, reviewer, supervisor, planner, publication, or human authority; and
- terminal evidence reconstructing the relationship between the original objective, orchestration plan, branch plans, branch results, and integration result.

---

# 20. Exclusions

This direction does not require:

- one universal planner model;
- one universal supervisor provider;
- centralized implementation reasoning;
- a global lock over all repository work;
- passing full branch context through the orchestrator;
- making the orchestrator an implementation reviewer;
- making the preplanner a portfolio dictator;
- permanent fixed branch decomposition;
- procedural specification of every valid execution route;
- automatic resolution of semantic integration conflicts;
- treating dependency graph reachability as semantic authority; or
- replacing existing Work Engine slice, review, checkpoint, publication, or context-lifecycle machinery.

---

# 21. Expected consequence

The intended scaling property is:

```text
more engineering work
        does not require
one supervisor to understand more unrelated engineering work
```

Instead:

```text
more engineering work
        ->
more coherent branches
        ->
more bounded planners and supervisors
        ->
mostly structural coordination above them
```

This gives Work Engine a hierarchy in which reasoning depth is spent locally while authority, evidence, and dependency state remain globally reconstructable.

---

# 22. Compact model

The architecture can be summarized as:

> **Preplanner:** What independent or dependent bodies of work exist?

> **Orchestrator:** Which authorized body of work can move now, and which actor owns the next transition?

> **Branch planner:** What implementation space should this workstream permit?

> **Supervisor:** How does this accepted workstream progress through bounded slices?

> **Builder:** What valid implementation route best satisfies this slice?

> **Reviewer:** What is wrong or insufficient in the immutable result?

> **Deterministic machinery:** Which states are mechanically invalid?

> **Operator:** Which decisions remain human-owned?

The intended recursive structure is:

```text
preplanner defines structure
        ↓
orchestrator realizes structure

planner defines structure
        ↓
supervisor realizes structure

builder chooses within structure
        ↓
review challenges the realization
```

This is hierarchical multi-agent development expressed in the same structural terms that already govern Work Engine rather than as a separate multi-agent coordination system.

