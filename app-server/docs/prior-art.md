# Work Engine and Its Prior Art: Positioning an Authority-Preserving Semantic Execution Architecture

## Abstract

Work Engine sits at the intersection of several mature architectural traditions: truth-maintenance systems, belief revision, normative and organizational multi-agent systems, institutional authority models, workflow compilation, scientific provenance, distributed control planes, durable execution, and compiler-style lowering. Nearly every important Work Engine primitive has substantial precedent. Revision chains, provenance, branching, role contracts, dynamic organization, candidate reduction, leases, fencing, plan/apply separation, durable execution, and mechanically governed interaction are not individually novel ideas.

The architectural distinction lies elsewhere.

Work Engine treats semantic planning, organizational realization, role contracts, runtime realization, claims, context lifecycle, and other semantic surfaces as **independently owned kinds of truth**. These domains may share mechanisms where their structural invariants are genuinely identical, but shared mechanism does not imply shared semantic ownership. Forward transformations are constrained so that downstream realization does not inherit the authority to reinterpret upstream meaning. Backward evidence propagation is constrained symmetrically: discovering evidence that falsifies an upstream semantic surface does not grant the discovering actor authority to revise that surface.

The resulting architecture can be described as an **authority-preserving semantic execution architecture**, or analytically as a **Revisioned Semantic Control Plane (RSCP)**: independently authoritative semantic state is connected through semantically limited mechanisms, revision-bound publication, and explicit routes for residual judgment and invalidation.

This paper positions Work Engine against its strongest identified antecedents and separates established prior art from the narrower architectural composition that remains unusual.

---

## 0. Implementation Status of the Compared Architecture

This paper compares Work Engine's **planned** architecture against prior art, not a fully built and running system. That distinction is load-bearing enough to state once, explicitly, rather than leave implicit throughout.

Every dimension and mechanism discussed below is drawn from the 20 canonical views under `app-server/docs/architecture/`, each carrying its own four-axis status (`design` / `reconciliation` / `authorization` / `implementation`) per `status-grammar.md`. The capstone synthesizing them, `work-engine-planned-architecture.md`, states its own authority directly:

> Exploratory synthesis only. This document does not accept any idea, authorize implementation, amend a migration roadmap, or select a permanent schema.

Most of what this paper positions against ATMS, commitment MAS, MOISE+/OperA, MACODO, ORA4MAS, Kubernetes, Terraform, and the rest is `design: proposed` or `accepted`, but `implementation: none` or `partial`. Concretely:

```text
Actually implemented today:
    - claim-evidence core substrate (create_claim, publish_revision,
      publish_lineage, record_reliance, retire_reliance, retract_revision)
    - production-path-v1 claim profile
    - Context Lifecycle's pressure detection, checkpoint publication,
      and transition-lease sequence
    - Agent Environment Graph, read side only
    - the executable-generation substrate beneath Runtime Realization
      (a nested instance of revision/CAS and transition fencing)
    - Supervisor / Builder execution itself

Accepted design, not implemented:
    - Organizational Compilation's admission mechanism (no compiler exists)
    - Claim-evidence's refresh lifecycle (nominate_impact /
      open_refresh_episode / publish_refresh_judgment)
    - Candidate Resolution and Admission, as a named mechanism, across
      every one of its confirmed instances
    - Transition Fencing's topology-transition fence class

Proposed or exploratory, not yet accepted:
    - the upper semantic-planning hierarchy (Preplanner, Orchestrator,
      Branch Planner, multi-branch topology) beyond Supervisor/Builder
    - the authority-projection model itself (authority-and-ownership.md's
      own source idea document: "Exploratory only")
    - recursive organizational compilation and its stopping condition
    - cross-cutting realizations (explicitly an open, undecided question)
    - Runtime Realization's own RoleRealization candidate resolution and
      revision lineage (distinct from the implemented executable-generation
      substrate beneath it)
```

None of this weakens the comparisons that follow — architectural composition can be evaluated as a design regardless of build status, and that is the register this paper works in throughout. But a reader should not infer, from the confident present tense used below ("Work Engine treats...", "Work Engine's Organizational Compilation is..."), that the compared system is deployed and operating at the scope described. It is a reconciled architectural direction, tested against prior art precisely to learn what a *future* implementation should and should not claim — not a retrospective description of one already running.

---

## 1. Architectural Thesis

Work Engine's architecture begins from a strict distinction between access to information and standing to decide what that information means.

Its authority model recognizes separately the abilities to observe, recommend, nominate, decide, admit, and execute. A role may participate in several of those operations without owning all of them. Authority is assigned to the narrowest legitimate vantage satisfying the evidence, ownership, continuity, independence, and consequence requirements of the decision. fileciteturn3file1

This produces an architecture in which semantic work does not simply flow down a hierarchy of increasingly concrete representations. Each lowering stage owns a different question.

Semantic Planning determines what work exists, which obligations and dependencies hold, which consequences must be preserved, and which semantic owners are affected. Organizational Compilation receives that accepted meaning and determines how it should be distributed across legitimate execution vantages. Role/Contract Structure states what each logical vantage must observe, own, decide, emit, mutate, preserve, and avoid. Runtime Realization then binds that logical contract to a concrete provider, model, harness, transport, and mechanism composition. fileciteturn3file9

The distinction is not merely modularity. A downstream layer is explicitly prohibited from silently repairing or redefining the semantics of its source. Organizational authority may group, separate, or assign accepted obligations, but it may not create or remove obligations, change dependency meaning, or repair a false plan by inventing a new planning topology. fileciteturn3file2

The reverse path follows the same principle. Execution may discover evidence that falsifies assumptions embedded in accepted planning state. The discovering actor may surface that evidence and nominate a topology conflict, but semantic revision still belongs to the planning owner. fileciteturn3file5

This symmetry—**authority-preserving lowering downward and authority-preserving feedback upward**—is the central object of comparison in this paper.

---

## 2. Established Prior Art

### 2.1 Truth Maintenance and Assumption-Based Reasoning

Johan de Kleer's Assumption-Based Truth Maintenance System is an important ancestor for Work Engine's dependency-oriented reasoning.

The ATMS separates the truth-maintenance machinery from the domain problem solver. It records justifications and computes the environments under which conclusions hold, while the larger problem-solving system remains a separate component. citeturn794860search1

This establishes strong precedent for:

- explicit dependency structures;
- context-relative conclusions;
- multiple alternative environments;
- mechanically maintained consequence relationships;
- separation between domain reasoning and dependency machinery.

Consequently, Work Engine should not claim novelty for dependency-directed reconsideration, multiple semantic contexts, or externalized maintenance of reasoning dependencies.

The distinction is narrower. ATMS machinery itself carries a logical semantics that determines applicability of propositions within environments. Work Engine's evidence machinery is intentionally weaker where semantic authority is concerned: it may establish an observation, derivation, contradiction, stale dependency, or impact nomination without thereby acquiring the standing to revise the semantic object that depended upon it.

That makes Work Engine less a truth-maintenance system than a system for **maintaining the routes by which truth-bearing owners must reconsider their own state**.

---

### 2.2 Belief Revision and Argumentation

AGM belief-revision theory and its descendants strongly precede the idea that new evidence need not uniquely determine a successor belief state. Partial-meet contraction constructs admissible remainder sets and relies on a selection function to determine which survive. Non-prioritized and relevance-sensitive variants weaken still further the idea that incoming evidence must simply replace existing belief.

Formal argumentation systems likewise separate the existence of attacks or challenges from the eventual acceptance status of claims.

These traditions establish prior art for:

- admissible successor sets;
- relevance-sensitive revision;
- residual choice after mechanical reduction;
- challenge without immediate rejection;
- reasoning about incompatible alternatives.

Therefore Work Engine's `0 / 1 / N` candidate structure is not unprecedented merely because deterministic reduction can terminate in an unresolved set.

What is more unusual is the treatment of the `N` case as an **authority question** rather than merely an unresolved search or ranking problem.

Work Engine's Candidate Resolution and Admission mechanism may establish impossibility, determinacy, or the continued existence of irreducible choice. It explicitly does not own the domain meaning of that choice or the identity of the decision owner. Those are supplied by the consuming semantic domain. fileciteturn2file1L15-L26 fileciteturn2file1L45-L58

---

## 3. Normative and Organizational Multi-Agent Systems

### 3.1 Social Commitments

Commitment-oriented multi-agent systems provide some of the strongest intellectual ancestry for Work Engine's authority model.

Munindar Singh's work distinguishes social commitments from internal psychological states and treats commitments as public relational objects rather than merely beliefs or intentions inside an agent. citeturn794860search10turn794860search12

This matters because Work Engine similarly refuses to reduce semantic authority to an actor's internal reasoning state. An agent may know something, believe something, or be capable of doing something without thereby having institutional standing to produce an authoritative transition.

Social-commitment systems therefore strongly precede:

- externally represented semantic obligations;
- agent autonomy under public interaction constraints;
- role-sensitive creation and discharge of obligations;
- separation of public social state from private reasoning.

Work Engine extends this style of distinction beyond commitments into multiple independently revisioned semantic surfaces.

---

### 3.2 Institutionalised Power

Jones and Sergot's formal treatment of institutionalised power is direct prior art for Work Engine's distinction between capability and authority.

Their central observation is that designated agents in norm-governed institutions can be empowered to create particular institutional states of affairs through specified acts. citeturn357221search1

This is closely aligned with Work Engine's rule that the ability to produce an action or statement does not imply standing for that action to create authoritative system state.

A reviewer may be capable of producing an acceptance-like judgment. That does not imply implementation-acceptance authority. A builder may observe a contradiction in a plan. That does not grant planning authority. A failed runtime realization does not authorize a previously prohibited alternative.

Accordingly, the distinction

**capability ≠ permission ≠ institutional power**

has strong prior art.

Work Engine's contribution is not that distinction itself, but its mechanical integration into semantic execution and revision publication.

---

### 3.3 MOISE+, OperA, and Organizational Models

MOISE+ models organizations through structural, functional, and deontic viewpoints rather than a single undifferentiated organizational representation. citeturn794860search5

OperA and related systems likewise distinguish organizational roles, interaction structures, normative constraints, and concrete enactment.

These systems strongly precede:

- explicit organizational structure;
- independent organizational concerns;
- role semantics;
- role enactment;
- norms and obligations as architectural state;
- organizational reconfiguration.

The general idea that a system should derive concrete execution organization from higher-level organizational or normative meaning is therefore well established.

Work Engine's Organizational Compilation is nevertheless more restrictive. It begins from already accepted semantic work and may only derive lawful execution consequences from it. The organizational layer does not own the right to reinterpret the work it is realizing. fileciteturn3file0

---

### 3.4 MACODO

MACODO is one of the strongest architectural analogues identified.

Its organization model was created specifically to separate organization lifecycle management from agents. Organizations become first-class entities, and the middleware supports context-driven dynamic organizations that can change in response to environmental conditions. citeturn794860search7turn794860search13

MACODO establishes strong prior art for:

- organization as first-class runtime state;
- role contracts;
- capability-aware organization;
- context-driven reorganization;
- lifecycle management outside individual agents;
- dynamic organizational restructuring.

If Work Engine consisted only of dynamic role organization around autonomous agents, MACODO would substantially weaken any claim of architectural distinctiveness.

The divergence occurs in the treatment of semantic invalidation.

MACODO's organizational laws are allowed to drive reorganization in response to context. Work Engine instead distinguishes whether evidence merely affects **how accepted meaning should be realized** or whether it falsifies the accepted meaning itself.

If the semantic topology is still valid, organizational realization may change.

If the topology is false, Work Engine routes the conflict to the semantic-planning authority rather than allowing organizational adaptation to repair the plan. fileciteturn3file2

This is an example of authority-preserving feedback.

---

## 4. ORA4MAS: The Strongest Challenge to the Mechanism/Judgment Boundary

ORA4MAS and related Agents & Artifacts work provide the strongest prior art found against a broad claim that Work Engine uniquely separates mechanical infrastructure from semantic agency.

ORA4MAS introduces organizational artifacts as first-class infrastructure supporting normative multi-agent organizations. citeturn357221search0

This lineage explicitly separates organizational infrastructure from the autonomous agents operating within it. Artifacts can embody mechanically enforceable or observable aspects of organizational state while agents remain responsible for other judgments and responses.

That means the broad proposition

> infrastructure may establish mechanical facts while leaving deliberative consequence to autonomous agents

is established prior art.

Work Engine should not claim that separation in itself.

The surviving distinction is more specific:

> **Work Engine's shared mechanisms are not merely generic within one semantic family; they are deliberately ignorant of the semantic domains that reuse them.**

Candidate Resolution does not understand organizations or runtime providers.

Revision/CAS does not understand branch plans, claims, checkpoints, or realizations.

Transition fencing does not understand the semantic release condition of the transition it protects.

The mechanism is reused because an invariant is equivalent, not because the participating semantic objects have been unified under one ontology.

This is a materially stronger constraint than generic organizational infrastructure.

---

## 5. Distributed Control Planes

### 5.1 Kubernetes

Kubernetes supplies extensive prior art for the mechanical side of Work Engine:

- independently operating controllers;
- resource-specific ownership;
- desired versus observed state;
- versioned resources;
- optimistic concurrency;
- admission boundaries;
- leases and active ownership;
- reconciliation after state change.

Work Engine should therefore make no claim to inventing control-plane reconciliation, leases, fencing, or optimistic concurrency.

The important difference is that Kubernetes controllers ordinarily operate in domains with an available reconciliation function.

A controller can determine what it means to move an observed resource toward its declared desired state.

Work Engine frequently cannot.

It may mechanically determine that an evidence relationship changed and identify every exact dependency affected, while remaining prohibited from deciding what that change means for the semantic object.

This is why Work Engine's architecture is control-plane-like without being reducible to ordinary desired-state reconciliation.

---

### 5.2 Terraform

Terraform is strong precedent for the structure:

**observe → plan → inspect/authorize → execute**.

A saved plan concretizes proposed operations and can later be applied as that specific plan. citeturn794860search17

This strongly precedes Work Engine's interest in sealed artifacts and the separation between planning and execution.

However, Terraform's desired infrastructure semantics are sufficiently formal for configuration and observed state to produce a plan mechanically.

Work Engine instead preserves:

**claim ≠ decision ≠ plan ≠ publication authority**.

Its planning and admission layers therefore represent distinct semantic acts rather than phases of one deterministic diff.

---

## 6. Provenance and Workflow Systems

### 6.1 W3C PROV

W3C PROV provides domain-agnostic representations for entities, activities, agents, derivations, attribution, and provenance relationships. citeturn794860search0turn794860search3

Provenance is therefore clearly established prior art for:

- source attribution;
- derivation;
- lineage;
- responsibility;
- historical reconstruction;
- evidence about how an artifact came to exist.

Work Engine's evidence/claims model should not be described as novel merely because it tracks provenance or lineage.

Its stronger rule is:

> **a claim is a materialization, not the thing it describes.**

The claim has its own revision, provenance, lineage, and reliance lifecycle, but changing the claim does not authorize rewriting its source. fileciteturn1file3L5-L13

That "cache, never owner" rule turns provenance into a subordinate semantic relation rather than a global truth model.

---

### 6.2 Scientific Workflows and Pegasus

Pegasus provides important prior art for progressive execution lowering.

Its planning process takes an abstract workflow and produces a concrete executable workflow, adding resource-specific tasks such as data staging, directory creation, cleanup, and output registration. citeturn794860search15

Scientific workflow systems more broadly distinguish prospective process descriptions from retrospective execution provenance, and systems such as VisTrails preserve workflow evolution and branching histories.

This establishes strong precedent for:

- abstract-to-concrete workflow lowering;
- execution-independent versus execution-specific representation;
- retrospective execution provenance;
- versioned workflow evolution and replay.

The difference is ownership.

In workflow systems, abstract and concrete workflows are normally representations of the same underlying computation at different levels.

Work Engine deliberately gives different lowering levels different semantic authority.

The Role/Contract layer, for example, is explicitly a consequence of already admitted semantic and organizational meaning and cannot author new semantic meaning itself. Runtime Realization then consumes that contract without owning it. fileciteturn3file9

Thus the Work Engine chain is not simply progressive representation refinement. It is **authority-constrained refinement**.

---

## 7. Contemporary Agent Runtimes

Modern agent frameworks increasingly provide durable execution, sessions, handoffs, tracing, guardrails, replay, and human intervention.

The OpenAI Agents SDK, for example, includes agents, handoffs, guardrails, sessions, human-in-the-loop mechanisms, and built-in tracing of generations, tool calls, handoffs, guardrails, and custom events. citeturn794860search4turn794860search2turn794860search8

These systems demonstrate that durable and inspectable agent execution is quickly becoming ordinary infrastructure.

They do not, however, generally model:

- semantic-planning authority;
- organizational-realization authority;
- claim/source separation;
- explicit admission authority;
- separately revisioned semantic owners;
- domain-blind reusable transition mechanisms.

Their principal state remains agent or workflow execution state rather than a network of independently authoritative semantic states.

The contemporary convergence is therefore strongest on Work Engine's runtime substrate, not on its authority topology.

---

## 8. What the Prior Art Falsifies

The research materially weakens any claim that Work Engine is distinctive because it contains the following ideas.

These should be treated as established architectural traditions:

1. **Dependency-aware reasoning** — TMS/ATMS.
2. **Alternative contexts and branching semantic worlds** — ATMS and related systems.
3. **Non-forced belief revision and admissible successor sets** — belief revision.
4. **Public semantic commitments separate from private cognition** — commitment MAS.
5. **Institutional authority distinct from physical capability** — institutionalised-power theory.
6. **Roles, contracts, norms, and organizational structure** — MOISE+, OperA, electronic institutions.
7. **Dynamic organization external to agents** — MACODO.
8. **Mechanical organizational infrastructure that stops before some agent judgment** — ORA4MAS.
9. **Provenance and revision lineage** — W3C PROV and scientific workflows.
10. **Abstract-to-concrete workflow lowering** — Pegasus and workflow systems.
11. **Plan/apply separation** — Terraform.
12. **Revision/CAS/fencing/leases** — distributed systems and control planes.
13. **Durable multi-agent execution and observability** — contemporary agent runtimes.

A defensible Work Engine position should explicitly acknowledge these traditions rather than presenting familiar mechanisms as inventions.

---

## 9. The Architectural Composition That Survives

After repeated attempts to falsify the architecture against these traditions, four coupled properties remain unusually difficult to match as one integrated design.

### 9.1 Heterogeneous Semantic Ownership

Work Engine does not define one universal state object, organizational model, workflow graph, belief base, or blackboard as canonical semantic truth.

Different questions have different owners.

Planning owns the semantic topology of the work.

Organizational Compilation owns the lawful distribution of accepted work across execution vantages.

Role/Contract Structure owns the logical requirements of a vantage.

Runtime Realization owns the concrete composition satisfying that contract.

Evidence/Claims owns the lifecycle of materialized propositions without owning their sources.

Context Lifecycle owns replacement and continuation of the same vantage without owning organizational topology.

This is stronger than modular decomposition because the boundaries constrain who may author successor meaning.

---

### 9.2 Invariant Sharing Without Semantic Unification

Work Engine permits multiple domains to reuse one mechanism when—and only when—their structural invariant is genuinely the same.

Revision/CAS can publish different kinds of revisioned state because predecessor identity, stale-write rejection, and atomic visibility mean the same thing mechanically across those domains.

Candidate Resolution can reduce different candidate universes because impossibility, determinacy, and irreducible remainder have the same structural meaning even when the candidates themselves mean entirely different things.

The architecture explicitly refuses the common inference:

**same mechanism ⇒ same semantic abstraction**.

This is one of Work Engine's clearest distinguishing disciplines.

---

### 9.3 Authority-Preserving Lowering

Forward transformation is allowed to derive consequences but not inherit authorship rights over the semantic source.

Conceptually:

**Semantic Planning**  
→ accepted semantic structure

**Organizational Compilation**  
→ lawful execution vantages

**Role/Contract Structure**  
→ logical role constraints

**Runtime Realization**  
→ concrete provider/model/harness composition

Each step answers a narrower downstream question.

The architecture does not allow the organizational compiler to repair a false plan, the role contract to create new obligations, or runtime resolution to redefine the logical role because its preferred provider became unavailable.

This is more restrictive than normal compilation or workflow lowering because semantic authority must be preserved as well as behavioral intent.

---

### 9.4 Authority-Preserving Feedback

The same protection operates in reverse.

A downstream actor can discover evidence outside its authority.

The result is not automatic adaptation of the nearest modifiable object.

Instead, the evidence is routed toward the owner of the semantic surface it actually challenges.

The planning hierarchy states this directly: local evidence can invalidate accepted topology without granting the discovering actor authority to replace it. fileciteturn3file5

Evidence/Claims follows the same pattern. A changed planning-derived claim can motivate a topology-conflict nomination, but claim-evidence may not rewrite the planning revision it describes. fileciteturn3file2

This gives Work Engine a characteristic bidirectional discipline:

**meaning lowers without authority flowing downward; evidence rises without authority flowing upward.**

---

## 10. A Refined Description: Revisioned Semantic Control Plane

"Revisioned Semantic Control Plane" is useful as an analytical description, provided it is not mistaken for a claim that Work Engine has one centralized semantic controller.

A more precise definition is:

> **A Revisioned Semantic Control Plane is an execution architecture in which heterogeneous semantic surfaces retain independent revision identity and decision authority; semantically limited mechanisms carry structural invariants across those surfaces; forward lowering derives lawful downstream consequences without transferring ownership of upstream meaning; and backward evidence propagation identifies affected authority surfaces without granting the discovering actor authority to revise them.**

Under this interpretation, the control plane is the **network of authority-preserving transitions**, not a central authority.

The architecture's recurring shape is therefore:

**observation**  
→ mechanically knowable derivation  
→ bounded residual judgment  
→ owning decision surface  
→ admission  
→ revision-safe publication  
→ concrete realization  
→ observation of consequence

The critical rule is that no arrow silently grants the authority of the node it reaches.

---

## 11. Strongest Prior-Art Comparators

The most useful external comparators are not competitors in a product sense. Each exposes one architectural lineage Work Engine draws upon.

**ATMS** should be cited for dependency-sensitive reasoning and multiple contexts.

**Commitment-oriented MAS** should be cited for public semantic obligations and autonomous actors.

**Jones and Sergot** should be cited for institutional authority distinct from practical capability.

**MOISE+/OperA** should be cited for explicit organizational structure, roles, norms, and enactment.

**MACODO** should be cited for first-class dynamic organizations and organization lifecycle externalized from agents.

**ORA4MAS** should be treated as perhaps the strongest historical challenge, because it explicitly places organizational infrastructure outside agents and allows infrastructure to enforce or observe some facts without absorbing every deliberative judgment. citeturn357221search0

**Kubernetes/Terraform** should be cited for the mechanical control-plane lineage.

**W3C PROV** should be cited for provenance and lineage.

**Pegasus/scientific workflow systems** should be cited for progressive lowering and execution provenance.

**Modern agent runtimes** should be cited as evidence that durable agent execution is becoming infrastructure rather than an architectural differentiator.

---

## 12. Position

The strongest defensible position is not that Work Engine invented its constituent mechanisms.

It did not.

Nor is the evidence sufficient to claim that no previous architecture has ever independently combined the same ideas. Such a conclusion would require a much more exhaustive literature and patent search than an architectural prior-art investigation can provide.

The narrower architectural position is stronger:

> **Work Engine combines established ideas under an unusually strict non-collapse discipline. Heterogeneous semantic authorities remain distinct even where they share infrastructure; reusable mechanisms are justified by invariant equivalence rather than semantic commonality; downstream realization cannot silently acquire upstream semantic authority; and downstream evidence cannot silently acquire the authority to revise the upstream meaning it falsifies.**

This composition appears meaningfully different from the strongest identified antecedents.

Truth-maintenance systems centralize logical consequence.

Workflow systems centralize computation semantics.

Blackboard systems centralize shared semantic state.

Organizational MAS centralize organizational semantics.

Control planes centralize reconciliation around formally representable desired states.

Work Engine instead attempts to govern computation in a setting where **there may be no single legitimate owner of "the state" and no universal reconciliation function for semantic consequence**.

The architecture therefore does not try to automate semantic authority away.

It tries to make every boundary around semantic authority explicit enough that everything outside that boundary can be automated aggressively.

That may be the most concise statement of Work Engine's architectural direction:

> **Externalize everything mechanically knowable without externalizing judgment that still belongs to an authorized semantic owner.**

Or, stated as the structural doctrine that emerged from the prior-art comparison:

> **Information may cross an ownership boundary. Authority does not cross with it unless the architecture explicitly says that it does.**

---

## 13. Conclusion

The prior-art investigation changes the interpretation of Work Engine in an important way.

Work Engine is not best understood as a new agent framework. Many systems already orchestrate agents, maintain context, provide durable execution, reorganize roles, enforce norms, preserve provenance, or separate planning from execution.

It is better understood as an attempt to build an **authority-preserving execution substrate for semantic computation**.

Its architectural novelty, if any, is therefore likely combinatorial and structural rather than primitive: the disciplined coexistence of multiple independently owned semantic truths with generic mechanisms that remain too weak to appropriate those truths.

That interpretation is consistent with the architecture's repeated distinctions:

**observation is not judgment;**  
**claim is not source;**  
**decision is not admission;**  
**plan is not execution;**  
**role contract is not runtime realization;**  
**shared mechanism is not shared semantic ownership;**  
**new evidence is not new authority.**

The most important result of the prior-art work is therefore not that Work Engine appears unprecedented.

It is that the investigation has progressively removed weak novelty claims while leaving a smaller, clearer architectural core intact.

That surviving core is now specific enough to be falsifiable, implementable, and worth continuing to test.

