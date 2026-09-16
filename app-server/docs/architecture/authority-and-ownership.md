# Authority and Ownership

> **Question:** Which logical role owns each semantic decision, and what may every other role do with respect to it?

## Purpose

This view shows Work Engine's **authority topology**.

The semantic planning hierarchy describes how work flows through planning and execution. This page asks a different question:

> **Which decisions belong to which vantage, and what kinds of participation are permitted without transferring ownership?**

Work Engine treats authority as more specific than access, capability, or proximity to evidence.

A role may:

- observe a fact,
- recommend an action,
- nominate a decision,
- make a bounded decision,
- admit a consequence,
- execute an admitted consequence,

without necessarily owning all of those operations.

The central rule is:

> **Place each semantic decision with the narrowest authorized logical role whose vantage satisfies the evidence, ownership, continuity, independence, and consequence requirements of that decision.**

Authority does not arise merely because an actor has the information, the capability, or the opportunity to act.

---

## Diagram

```mermaid
flowchart LR
    subgraph PLAN["Semantic Planning Authority"]
        PRE["Preplanner"]
        BP["Branch Planner"]
        PLANOWN["Owns semantic work topology"]
    end

    subgraph ORG["Organizational-Realization Authority"]
        TOPO["Topology / Organizational Compiler"]
        ORGJ["Event-scoped Organizational Judgment"]
        ORGOWN["Owns how accepted work is distributed<br/>across legitimate execution vantages"]
    end

    subgraph EXEC["Execution Authority"]
        SUP["Supervisor"]
        BLD["Builder / Specialist / Execution Vantage"]
        EXECOWN["Owns bounded execution within<br/>accepted semantic and organizational constraints"]
    end

    subgraph EVID["Evidence and Claim Authority"]
        OBS["Evidence Producer / Observer"]
        CLAIM["Claim-Evidence Domain Owner"]
        EVIDOWN["Owns evidence recording, impact nomination,<br/>refresh, and claim-state publication"]
    end

    subgraph PUB["Admission / Publication Authority"]
        ADMIT["Owning Admission Boundary"]
        REV["Authoritative Revision Publication"]
        PUBOWN["Owns whether a consequence becomes authoritative"]
    end

    PRE --> BP
    BP --> TOPO
    TOPO --> SUP
    SUP --> BLD

    OBS --> CLAIM
    SUP --> OBS
    BLD --> OBS

    BP --> ADMIT
    TOPO --> ADMIT
    CLAIM --> ADMIT

    ADMIT --> REV

    SUP -.->|"nominate topology conflict"| BP
    BLD -.->|"surface execution evidence"| OBS
    OBS -.->|"nominate impact"| CLAIM

    PLANOWN --- PRE
    PLANOWN --- BP

    ORGOWN --- TOPO
    ORGOWN --- ORGJ

    EXECOWN --- SUP
    EXECOWN --- BLD

    EVIDOWN --- CLAIM
    PUBOWN --- ADMIT
```

---

## How to Read This View

This diagram is not a call graph.

It shows **authority domains** and the permitted movement of information and consequence between them.

A downstream role can surface evidence upward without acquiring the authority of the role that receives it.

Likewise, an upstream role can authorize a bounded consequence without necessarily performing the work itself.

---

## 1. Semantic Planning Authority

Semantic planning authority answers questions such as:

- What work actually exists?
- What are the accepted workstreams?
- What depends on what?
- Which semantic owners are affected?
- What consequences must be preserved?
- Is the accepted topology still correct?

At orchestration scale, that authority belongs to planning roles such as the **Preplanner**.

At branch scale, it belongs to the **Branch Planner**.

A Supervisor may discover evidence relevant to these questions, but discovery is not ownership.

For example:

```text
Supervisor discovers:
    A and B are not actually independent.

Supervisor may:
    report evidence;
    nominate topology conflict.

Supervisor may not:
    silently revise the branch plan to make A depend on B.
```

The semantic revision moves back to the role that owns the affected planning surface.

---

## 2. Organizational-Realization Authority

Organizational-realization authority answers a different class of question:

> **Given accepted semantic work, how should it be distributed across legitimate execution vantages?**

Examples include:

- Should two accepted obligations share one retained implementation context?
- Should a bounded specialist vantage be introduced?
- Does an independence requirement require a separate role?
- Is one execution context carrying too much unrelated semantic responsibility?
- Should a branch remain one execution vantage or become several?

This authority must remain bounded by the accepted semantic topology.

It may:

- group accepted obligations;
- separate accepted obligations;
- assign them to distinct execution vantages;
- project already-admitted authority into child roles;
- choose among lawful organizational realizations.

It may not silently:

- create new semantic obligations;
- remove accepted obligations;
- alter dependency meaning;
- redefine affected semantic owners;
- repair a false plan by inventing a new planning topology.

The unresolved boundary around cross-cutting realizations—where one execution vantage would own only part of one semantic obligation and part of another—belongs to the organizational-compilation view, not this page.

---

## 3. Execution Authority

Execution authority belongs to roles such as Supervisors, Builders, Specialists, and other realized execution vantages.

These roles act within already-admitted constraints.

A Supervisor may own:

- bounded execution coordination;
- branch-local continuity;
- child-role supervision;
- mutation within the accepted boundary;
- local execution decisions;
- evidence production;
- nomination of conflicts outside its authority.

A Builder or Specialist may own:

- the implementation consequence assigned to its role;
- mutations within its granted boundary;
- bounded local judgments necessary to complete that consequence.

Execution authority does not become planning authority merely because execution exposes new facts.

---

## 4. Evidence-Producing Authority

Work Engine separates **being able to observe something** from **being authorized to decide what that observation means for the system**.

Evidence producers may include:

- Supervisors;
- Builders;
- repository observers;
- runtime observers;
- context observers;
- provider adapters;
- deterministic analysis services.

An evidence producer may establish:

```text
"These two components share state."
```

It does not automatically establish:

```text
"The branch topology must change."
```

That second statement is a semantic consequence owned elsewhere.

This separation lets Work Engine externalize observation aggressively without externalizing judgment improperly.

---

## 5. Claim-Evidence Authority

The claim-evidence system owns the lifecycle of materialized claims and their evidence relationships.

Its authority may include operations such as:

```text
nominate_impact
open_refresh_episode
publish_refresh_judgment
```

These operations determine the state of the **claim**.

They do not automatically determine the state of the semantic object from which the claim originated.

For example:

```text
planning-derived claim:
    "A depends on B under branch-plan revision R"

new evidence:
    contradicts the claim

claim-evidence may:
    mark the claim changed through its own governed lifecycle

claim-evidence may NOT:
    rewrite branch-plan revision R
```

The changed claim can support a topology-conflict nomination.

Planning authority remains responsible for semantic revision.

The invariant is:

> **Claim state and source-authority state are related but not interchangeable.**

---

## 6. Admission and Publication Authority

Work Engine distinguishes deciding a consequence from making that consequence authoritative.

A valid semantic judgment may still require admission by the owner of the affected state transition.

Examples include:

- accepting an orchestration plan;
- accepting a branch plan;
- publishing a revised ExecutionEnvelope;
- admitting an organizational-topology revision;
- publishing a claim refresh judgment;
- activating a successor context;
- accepting an implementation's post-execution conformance (confirmed 2026-09-16: `capability-contract.mjs`'s real `capability.checkpoint_lifecycle/accept` and `/stop` capabilities, with `production-path-contract.mjs` mechanically enforcing that the accepting `owner` may never be `reviewer`/`builder`/`adapter`/`terminalizer` — self-authorization is a thrown error, not merely a documented rule).

This gives Work Engine a recurring pattern:

```text
observation
    ↓
bounded judgment
    ↓
admission
    ↓
authoritative publication
    ↓
runtime realization
```

A role that can judge a matter does not automatically gain authority to publish every consequence of that judgment.

---

## 7. Participation Modes Are Not Equivalent

Work Engine benefits from distinguishing several modes of participation explicitly:

```text
observe
recommend
nominate
decide
admit
execute
```

These are not synonyms.

### Observe

The role may perceive or produce evidence.

Example:

```text
Builder observes shared-state coupling.
```

### Recommend

The role may propose a preferred consequence without owning the decision.

Example:

```text
Specialist recommends a separate migration vantage.
```

### Nominate

The role may formally surface an issue to the owning decision boundary.

Example:

```text
Supervisor nominates topology conflict.
```

### Decide

The role owns the bounded semantic judgment.

Example:

```text
Branch Planner determines revised dependency topology.
```

### Admit

The role or service boundary determines whether the consequence becomes authoritative.

Example:

```text
Planning authority accepts branch-plan revision R'.
```

### Execute

The role realizes an already-admitted consequence.

Example:

```text
Builder executes the implementation contract.
```

Collapsing these verbs into a generic notion of "authority" hides important boundaries.

---

## 8. Authority Projection

Authority may be projected downward, but it must be attenuated.

The fundamental invariant is:

```text
child_authority ⊆ delegable(parent_authority)
```

A child role receives only the subset of authority required for its legitimate consequence.

Creating deeper organizational structure must never manufacture new authority.

```text
Root authority
    ↓
Workstream authority ceiling
    ↓
Supervisor authority
    ↓
Builder / Specialist authority
```

Each transition narrows scope.

It does not increase semantic standing.

---

## 9. Delegation Modes

Not all authority is delegable in the same way.

A decision surface may be:

### Non-transferable

The owning vantage must make the decision itself.

Example:

```text
A semantic owner may need to approve a consequence affecting its domain.
```

### Delegable

The owner may grant a bounded subset to another legitimate vantage.

Example:

```text
Supervisor delegates bounded mutation authority to Builder A.
```

### Nomination-only

The role may identify that a decision is required but may not resolve it.

Example:

```text
Supervisor nominates topology conflict.
```

### Advisory

The role may supply analysis without carrying formal consequence authority.

Example:

```text
Reviewer reports a finding.
```

The authority model should represent these differences rather than treating all child-role relationships as generic delegation.

---

## 10. Vantage Is Necessary but Not Sufficient

A role may possess excellent evidence and still lack legitimate authority.

Work Engine therefore treats a decision's legitimate vantage as more than informational access.

A decision may require some combination of:

```text
evidence sufficiency
semantic ownership
continuity
independence
effect boundary
authority ceiling
delegability
temporal position
```

The best-informed actor is not automatically the rightful owner.

Conversely, the formal owner may lack sufficient vantage to decide safely, in which case the architecture must create or consult a legitimate vantage rather than pretending ownership alone produces knowledge.

---

## 11. Deterministic Authority Projection

Some authority placement may eventually be derived mechanically.

Conceptually:

```text
Decision Requirements
    evidence requirements
    semantic owner
    continuity requirements
    independence constraints
    effect boundary
    authority ceiling
    delegability

        +

Candidate Role Vantages
    observations
    owned state
    continuity
    capabilities
    independence
    effects
    prohibitions

        ↓

Authority Projection
```

Possible results:

```text
exactly one eligible role
    -> deterministic projection possible

multiple eligible roles
    -> unresolved semantic / organizational choice

no eligible role
    -> explicit organizational gap
```

The projection mechanism may derive the consequence of already-admitted authority rules.

It may not invent new authority.

---

## 12. Invalidation Never Mints Authority

**Generalized 2026-09-16**, after the same shape appeared independently in two unrelated dimensions (claim refresh in Evidence/Claims; runtime-realization invalidation in Runtime Realization) — evidence this is a general property of this dimension, not a coincidence local to either.

> **Negative evidence may contract what is currently valid, eligible, current, or relied-upon. It may not create or expand who is authorized.**

Concrete instances, each independently arrived at before being recognized as one principle:

```text
a claim becomes changed/stale
    != authority to rewrite the source truth it materialized

a runtime realization becomes invalid
    != authority to raise a cost ceiling, weaken an independence
       requirement, expand tool access, or change evidence custody

a plan realization fails
    != the discovering role gaining planning authority

a context becomes unfit
    != the lifecycle service gaining organizational authority

a resource lease becomes stale or superseded
    != the new lease-holder acquiring more than exercise of the
       authority the owning domain already granted
       (mechanisms/resource-lease-and-fencing.md, added 2026-09-16)

an operator/human intent is collected, rendered, or submitted
    != the projection surface acquiring or enlarging the authority
       it is merely exposing
       (mechanisms/authority-preserving-intent-projection.md,
        added 2026-09-16)
```

The generative form, which subsumes all four:

> **Failure of an authorized candidate does not authorize a previously unauthorized alternative.**

If candidate A becomes invalid, Work Engine does not thereby conclude candidate B is authorized. It reruns candidate resolution and admission (see `organizational-compilation.md`, `runtime-realization.md`, and `material-decision-selection.md` for three confirmed instances of that shared mechanism, plus `portfolio-selection.md` as a likely fourth) against the *same, unchanged* authority ceiling. No authority is manufactured by failure — only the candidate set changes; what may lawfully be selected from it does not.

---

## Key Invariants

1. **Authority comes from an upstream legitimate owner; projection does not mint authority.**

2. **Information access is not authority.**

3. **Capability is not authority.**

4. **Execution proximity is not authority.**

5. **Coordination is not planning authority.**

6. **Claim maintenance is not source-domain authority.**

7. **A role may nominate a decision it may not decide.**

8. **A role may decide a bounded semantic question without owning publication of every consequence.**

9. **Child authority must be a subset of authority the parent is permitted to delegate.**

10. **Deeper organizational structure partitions authority; it does not create more of it.**

11. **Same actor does not imply same role, same decision surface, or same authority.**

12. **When no legitimate vantage owns a required decision, Work Engine should expose an organizational gap rather than silently assign authority to the nearest available actor.**

13. **Negative evidence (invalidation, staleness, failure) may contract what is currently valid; it may never expand who is authorized.**

14. **Failure of an authorized candidate does not authorize a previously unauthorized alternative — invalidation changes the candidate set, never the authority ceiling candidates are resolved against.**

---

## What This View Does Not Show

This page does not describe:

- the complete semantic planning flow;
- exact branch-plan contents;
- organizational-compilation algorithms;
- when another execution vantage should be created;
- context-lifecycle pressure or replacement;
- transition fencing;
- exact claim schemas;
- model/provider selection;
- runtime capability resolution;
- detailed recovery procedures.

Those dimensions have their own views.

In particular, this diagram is not intended to imply that all authority is embodied by named human-like roles.

Some authority may belong to:

- services;
- transition boundaries;
- admission mechanisms;
- durable state owners;
- deterministic compilers.

The named roles shown here are examples of authority-bearing logical vantages within the broader system.

---

## Relationship to Semantic Planning Hierarchy

The semantic-planning view asks:

> **Where does planning occur?**

This view asks:

> **What exactly is each actor allowed to do?**

For example:

```text
Semantic Planning Hierarchy:

Branch Planner
    ↓
Accepted Branch Plan
    ↓
Supervisor
```

The authority view expands that edge:

```text
Branch Planner:
    owns branch semantic topology

Accepted Branch Plan:
    authoritative semantic boundary

Supervisor:
    owns bounded realization / execution
    may surface contradictory evidence
    may nominate topology conflict
    may not silently replan
```

The two pages therefore describe the same system from different dimensions.

---

## Related Architecture Views

- **`semantic-planning-hierarchy.md`** — where semantic planning and replanning occur.
- **`organizational-compilation.md`** — how accepted obligations become concrete execution vantages under bounded authority.
- **`runtime-realization.md`** — the sibling instance of Candidate Resolution and Admission, and §12's other source for the invalidation-never-mints-authority invariant.
- **`mechanisms/candidate-resolution-and-admission.md`** — the mechanism whose residual-judgment and admission steps depend directly on this dimension's own decide/admit vocabulary.
- **`role-and-contract-structure.md`** — this dimension's authority/effect/independence model, instantiated concretely per logical role.
- **`context-lifecycle.md`** — how reasoning environments change without violating ownership or revision consistency.
- **`evidence-and-claims.md`** — how materialized facts remain subordinate to their authoritative source.
- **`mechanisms/revision-cas-and-publication.md`** — the shared succession/publication discipline this dimension's own authority-to-publish model (§6 above) governs, without owning the mechanics itself.
- **`review.md`** — produces judgments about fitness, correspondence, or acceptance-relevant properties, but never acceptance or disposition authority itself; `strategic-planning-handoff.mjs` is one concrete instance of this dimension's own decide/admit vocabulary (§7 above) applied to a review finding's consequence.
- **`mechanisms/resource-lease-and-fencing.md`** — a valid lease means the holder may currently exercise authority this dimension already granted, never that the lease itself mints new authority; §12's own generalized invariant, another concrete instance.
- **`mechanisms/authority-preserving-intent-projection.md`** — a projected intent exposes or encodes authority this dimension already granted, never manufactures it by being collected or displayed; another concrete instance of §12's own invariant.

Revisioned state, predecessor lineage, CAS publication, and atomic visibility (where authoritative state, derived state, and lineage live across all of these) are treated as a shared cross-cutting mechanism, not a truth dimension — settled, not open. Its canonical mechanism view is `mechanisms/revision-cas-and-publication.md`.

---

## Source and Status

**Retrofitted 2026-09-16** — this page predates `status-grammar.md` and had no formal `architecture_status` block, discovered by the three-category structural audit's own metadata pass. Each field below is re-derived independently, not summarized as one uniform value — this page mixes a mature, directly-settled invariant with proposed representation details, and the grammar exists precisely to keep those distinct.

```yaml
architecture_status:
  design: proposed
  reconciliation: reconciled
  authorization: unrecorded
  implementation: partial
  owner: app-server/ideas/pending/deterministic-authority-projection-and-adaptive-organizational-topology.md
  status_as_of: 2026-09-16
```

This default describes §1–§11: the authority-projection model, the observe/nominate/decide/admit/execute vocabulary, delegation modes, and deterministic authority projection. `design: proposed`, not `accepted` — the source idea document's own top-level Authority line is explicit: "Exploratory only." `reconciliation: reconciled` — checked directly against `hierarchical-planning-and-multi-supervisor-orchestration.md`'s ownership boundaries, `proposal-decision-gated-implementation-compilation.md`'s decision/admission distinctions, and claim-evidence's own authority separation. `authorization: unrecorded` — no citable authorization decision names this vocabulary at its current scope. `implementation: partial` — the vocabulary itself is not one runtime type, but concrete instances of it are real: claim-evidence's own evidence-producer/domain-owner permission classes are live code (`app-server/src/services/claim-evidence/contract.mjs`'s `PERMISSIONS`); the general "decision requirements declared against role vantage, projected deterministically" model in §4–§5 is not built anywhere.

```yaml
status_override:
  design: accepted
  reconciliation: reconciled
  authorization: design_work_authorized
  implementation: none
  source: app-server/docs/architecture/authority-and-ownership.md
```

Applies to §12 (Invalidation Never Mints Authority) specifically. Explicitly settled through direct discussion 2026-09-16 — the same bar every mechanism-recognition elsewhere in this architecture is held to — after the identical invariant appeared independently in `evidence-and-claims.md` and `runtime-realization.md` before either was recognized as one shape. `authorization: design_work_authorized` because naming and generalizing the invariant is what was authorized; nothing about this page authorizes building anything. `implementation: none` — §12 is a stated invariant, not a buildable artifact in its own right; its concrete instances are each dimension's own content, already covered by their own pages.