# Idea: Deterministic Authority Projection and Adaptive Organizational Topology

**Status:** Exploratory architecture idea

**Scope:** Semantic authority, role manifests, role composition, organizational compilation, context lifecycle, adaptive context/vantage topology, transition coordination, and domain-transferable role/authority modeling

**Authority:** Exploratory only. This document does not amend current role contracts, manifests, workflow ownership, execution-envelope design, context-lifecycle ownership, or implementation authority. It proposes a candidate architectural direction whose seams must be reconciled against current Work Engine artifacts before adoption.

**Pre-synthesis source material:** this document is a synthesized formulation, not a session transcript. The raw exploratory material it was formed from — including the full genealogy from the original seed hypothesis through the session dialogue that grounded it against real `context-lifecycle` code — is preserved separately at
[`app-server/ideas/history/2026-09-15-pre-synthesis/`](../history/2026-09-15-pre-synthesis/)
for reference, following the same convention as
[`ideas/history/2026-08-22-pre-reconciliation/`](../../../ideas/history/2026-08-22-pre-reconciliation/).
This document is the one that evolves going forward; the source material does not.

---

## Summary

Work Engine already assigns semantic decisions to different roles rather than concentrating judgment in a supervisor, planner, or operator. This idea traces that pattern to its foundation and finds three converging layers, each a deeper reformulation of the one before it:

1. **Authority follows vantage, not hierarchy.** A semantic decision should be exercised by the narrowest authorized role whose logical position supplies the evidence, ownership, continuity, independence, and consequence relationship that decision truthfully requires. Where those requirements and a role's declared properties are fully declared, the resulting authority assignment can become mechanically derivable — not invented, *projected* from an already-authorized source onto the role positioned to exercise it.

2. **Roles are compiled compositions, not atomic primitives.** A role such as "Builder" is a stable, reusable composition of more primitive elements — vantage, authority, obligation, continuity, information access, effect boundary, capability requirements, independence constraints, lifecycle. If those primitives are declared with enough structure, a role's own contract becomes partly a compilation target rather than a wholly hand-authored object, and roles themselves become partly emergent from the requirements of the work rather than fixed in advance.

3. **The organization is an adaptive projection of durable semantic state, and this follows the same architectural pattern context-lifecycle already applies to a narrower problem.** Work Engine's context-lifecycle manager already externalizes a *temporal* question — when should this same vantage receive a fresh context? — using deterministic pressure detection to invoke bounded semantic inference without burdening the working role with lifecycle mechanics. The organizational-topology question is not the same problem: temporal continuity concerns an *existing* vantage's lifetime, while topology concerns the fitness and composition of the vantages themselves. What is shared is the pattern, not the subject — observe externally, mechanically derive what can be derived, invoke bounded inference only at a genuine semantic gap, admit the consequence, realize a revision-bound transition. Both consume the same underlying observations, and the resulting architecture is not two separate systems but a shared observer feeding two *independent* consumers (neither owning the other's decision), coordinated by a shared transition-admission layer.

The unifying principle beneath all three layers:

> **The context window is not the database, and it is not the organization either.** It is a temporary, purpose-specific projection of the durable semantic world. Its contents, reasoning affordances, lifetime, and even its partition into logical vantages may change as the structure of the work changes — but only through governed, admitted, revision-bound transitions, never through inference quietly expanding its own scope.

None of this proposes that software determines who is wise enough to make an arbitrary judgment. The semantic requirements, authority boundaries, and organizational policy remain explicitly authored. What becomes mechanically derivable is the *consequence* of those declarations — and, just as importantly, when no valid consequence exists, an explicit organizational gap rather than a silently misplaced judgment.

---

## Part 1: Deterministic Authority Projection

### 1.1 The seed principle

> **A semantic decision should be exercised by the narrowest authorized role whose position gives it the strongest legitimate vantage for that decision.**

A role does not receive authority because of organizational rank. It receives it because its logical position supplies the combination of relevant evidence, accumulated context, semantic ownership, temporal position, independence, effect relationship, and authority ceiling required to make that particular judgment truthfully.

Work Engine already contains many decisions whose authority follows role-local context rather than hierarchy: a supervisor coordinates a slice without owning repository judgment; a builder owns implementation reasoning because it retains the implementation context necessary to make those decisions; a reviewer owns an attributed adversarial judgment while remaining unable to mutate the implementation or accept its own findings; a deterministic evidence observer nominates possible impact while remaining unable to decide whether the observed change semantically invalidates a claim; a claim-maintenance role adjudicates semantic consequence without acquiring authority over every downstream consumer relying on that claim; a scheduler knows an obligation is due without acquiring authority to execute it. These read as separate ownership rules. They may instead be consequences of one deeper rule: place semantic authority where the required legitimate vantage exists. The current architecture mostly records the resulting assignments; it does not yet make the underlying authority-placement rule a first-class architectural object.

### 1.2 Authority is decision-scoped, not role-scoped

Authority should not be treated as a broad property ("builder is authoritative," "reviewer is advisory"). Those statements are too coarse — a role may be authoritative for one semantic consequence and merely advisory, observational, or prohibited for another. The meaningful unit binds at least:

```text
decision identity / class
+ logical role
+ bounded subject
+ authority source / ceiling
+ applicable conditions
```

### 1.3 Vantage is not hierarchy, and is narrower than knowledge

The supervisor may be organizationally upstream of the builder while possessing a worse vantage for an implementation-local decision. The operator may retain ultimate product authority without being the appropriate role to exercise every local semantic judgment. Therefore:

```text
organizational rank != semantic vantage != decision authority
```

Coordination authority must not silently absorb domain authority. Visibility into another role's work must not silently transfer that role's judgment. Possession of an artifact must not imply authority over its meaning.

A role's vantage is not simply everything it knows — it is the conditions that make a judgment *legitimate*: evidence available to it, durable and transient state visible to it, context accumulated across a required interval, semantic objects it owns, consequences it may change, effects it may perform, temporal position in the work, independence from other roles or decisions, conflict-of-interest constraints, authority granted from an upstream source, explicit prohibitions, delegation rules, continuity requirements. A role can possess enough factual information to make a decision while still lacking the authority or independence required to own its consequence.

### 1.4 Decision requirements and role vantage, illustratively

A semantic decision type could declare the vantage required to exercise it — not proposed syntax, but a possible semantic distinction where the decision describes what must be true of its owner rather than naming the owner directly:

```yaml
decision: implementation-placement
requires:
  evidence: [repository-topology, accepted-objective, semantic-consequence-path]
  continuity: [current-slice]
  ownership: [implementation-result]
independence: none
authority_ceiling: bounded-domain
delegation: non-transferable
consequence: establish-placement
```

```yaml
decision: independent-implementation-review
requires:
  evidence: [immutable-candidate, review-contract]
independence:
  from: [implementation-author]
effects: read-only
consequence: attributed-review-judgment
```

The resulting role assignment may differ even when both decisions concern the same implementation. Symmetrically, a role manifest may eventually need to describe not merely what a role is called or what capabilities it has, but what logical vantage it can truthfully occupy:

```yaml
role: slice-builder
observes: [accepted-objective, repository-topology, current-candidate, gate-results]
owns: [implementation-result]
continuity: [slice]
effects: [mutate-candidate]
independence:
  not-independent-of: [implementation]
authority_ceiling: bounded-slice
```

The conceptual shift matters more than any representation: a role manifest may describe the vantage a role provides rather than serving primarily as a manually authored list of decisions that role owns. Decision authority could then be projected from the relationship between decision requirements and role vantage.

### 1.5 Deterministic authority projection

Where the inputs are fully declared, authority assignment may become mechanically derivable:

```text
decision requirements
        + available logical role vantages
        + authority ceilings
        + independence constraints
        + delegation rules
        |
        v
eligibility projection
```

with distinct outcomes: exactly one eligible role → authority can be projected deterministically; multiple legitimate eligible roles → an unresolved organizational or semantic choice remains; no eligible role → an organizational gap has been discovered, not silently papered over; an eligible role that exceeds the upstream authority ceiling → a forbidden projection; required independence conflicting with role topology → the topology is invalid for that decision. The deterministic machinery does not create semantic authority — it proves that an already-authorized logical role satisfies the declared requirements for exercising it.

### 1.6 Authority projection must never mint authority

This distinction is critical. The projection mechanism cannot turn "role has useful context" into "role has authority" without an upstream authority source permitting that consequence:

```text
human / product / domain authority
            |
            v
declared authority ceiling
            |
decision requirements + role vantage
            |
            v
bounded projected exercise
```

> **Vantage determines where authority may be exercised; it does not determine how much authority exists.**

A role can be ideally positioned to make a decision and still lack authorization to make it.

### 1.7 Delegation is part of the decision model

Not all semantic authority should be transferable in the same way. Candidate authority modes:

- **Non-transferable** — the designated vantage must exercise the judgment itself (e.g., an authorized semantic refresh owner determining whether evidence changes claim meaning; an observer or storage service cannot inherit that judgment).
- **Delegable** — the owner may delegate the complete bounded judgment to another role satisfying declared requirements.
- **Nomination-only** — one role nominates a consequence while another admits it (a role nominates specialist need; a host verifies independence and eligibility; the owning authority admits the final composition).
- **Advisory** — a role produces a recommendation without gaining authority over the consequence.

The model therefore needs to distinguish ability to observe, recommend, nominate, decide, admit, and execute — these are not interchangeable permissions.

### 1.8 Compound decisions decompose into multiple authority surfaces

A useful consequence of this model: concepts currently described as one decision may contain several differently owned decisions. "Select the review panel" may actually decompose into an implementation role judging what technical risks need challenge and what specialist expertise is relevant; a review/control boundary judging (mechanically or semantically) whether a candidate reviewer satisfies independence and whether a required profile or realization is admissible; and an owning authority deciding whether an exceptional policy deviation is allowed. The admitted composition becomes a consequence of several separately owned transitions. This follows the broader principle: shared subject matter does not imply shared ownership, and adjacent transitions are not automatically one transition. Authority projection should preserve, not erase, those distinctions.

### 1.9 Domain transferability

The authority-projection machinery is independent of software-development semantics. The domain supplies decision meanings, evidence types, semantic ownership, consequences, domain-specific services, and authority ceilings; the generic organizational layer supplies vantage matching, independence constraints, continuity constraints, delegation semantics, authority projection, role composition, and unresolved-gap detection:

```text
LEGAL: research sufficiency -> research vantage
       argument strategy -> case-context vantage
       settlement authority -> client / delegated authority

RESEARCH: statistical validity -> data + methodology vantage (possible independence constraint)
          scientific interpretation -> domain-expert vantage
          publication decision -> publication authority

OPERATIONS: incident diagnosis -> live-system evidence vantage
            risk acceptance -> business / safety authority
            execution -> operational capability + bounded effect authority
```

The generic authority model does not need to understand law, research, or operations. It needs to understand declared relationships between decision, vantage, authority, ownership, independence, continuity, delegation, and effect.

---

## Part 2: Roles as Compiled Compositions

### 2.1 Roles decompose into primitives

If authority can be derived from semantic requirements, the role contract itself may be partly derivable. A role begins to look less like an atomic entity and more like a composition:

```text
ROLE ≈ vantage + authority + obligation + continuity + information access
       + effect boundary + capability requirements + independence constraints + lifecycle
```

The more fundamental objects may be primitives such as `DecisionSurface`, `AuthorityGrant`, `VantageRequirement`, `EvidenceRequirement`, `EffectBoundary`, `ContinuityRequirement`, `IndependenceConstraint`, `Obligation`, `CapabilityRequirement`, `SubjectScope`, `TerminalConsequence` — not a proposed schema, but a conceptual reversal. A familiar role such as Builder may simply be a stable recurring composition of those primitives: `properties required by this work → composition → builder-like logical role`, rather than `Builder → properties`. Reusable roles need not disappear; they could become tested profiles or macros over role primitives.

### 2.2 Role contract compilation

Given sufficiently structured semantic inputs, the role contract itself may be largely deterministic:

```text
semantic obligation + required vantage + authority grant + independence constraints
+ available capabilities + current ExecutionEnvelope + system invariants
        |
        v
ROLE-CONTRACT COMPILER
        |
        v
logical role instance contract
```

deriving what the role must observe, what state it must retain, what authority it receives, what effects it may perform, what capabilities it requires, what it must produce, what it may not do, how long it must exist, and which role relationships must hold. The important boundary: **do not derive semantic meaning; derive the contract consequences of semantic meaning that has already been established.**

**Wave 1 reconciliation finding (checked directly against `role-compiler-proposal.md`'s full text, not just the fork research that first flagged this).** This is not the same operation `role-compiler-proposal.md`'s own compiler performs, and is not competing with it, but the boundary between them has to be stated precisely or the risk that document's own "Relationship to environment projections" section names — "it must not create a second owner for relation semantics merely to avoid the current CLI boundary" — becomes real. That compiler takes an *already-authored* `structure.yaml`/`interface.yaml` and renders projections (`SKILL.md`, tests, environment views); its own structural invariants are explicit that it "does not invent, paraphrase, or infer missing skill or role instructions" (invariant 5, "No fabricated semantics") — domain judgment stays in the authored source, never derived by the compiler. Part 2.2 proposes the opposite direction: mechanically *deriving* what a role contract should say from upstream primitives (obligation, vantage, authority grant, independence constraints, ExecutionEnvelope state) that are not yet expressed as an authored `structure.yaml` at all. These are adjacent pipeline stages, not one operation under two names — Part 2.2's derived output is a candidate *source* for `structure.yaml`'s role-profile fields, upstream of `role-compiler-proposal.md`'s own compiler, not a replacement for it. The actual risk is narrower than "don't build this": Part 2.2's derivation must express its output in the same relation vocabulary the Agent Environment Graph / role-compiler already owns (`bound_by`, `may_invoke`, `may_observe`, `may_mutate`, `owns`, `consumes`, `emits`, `mediated_transitions`, `forbidden_from`) rather than inventing a parallel one — that vocabulary is what `role-compiler-proposal.md`'s own "Relationship to environment projections" section names as the thing a second implementation must not re-own. This produces three layers:

```text
SEMANTIC CONTRACT       — what judgment or consequence exists, and what makes it legitimate?
        v
LOGICAL ROLE CONTRACT   — what must this particular logical role observe, own, decide, produce, and avoid?
        v
RUNTIME CONTRACT        — what concrete model/provider/tools/harness can realize that role now?
```

### 2.3 ExecutionEnvelope as a compiled organization

Rather than simply an authored description of who exists and how they relate, `ExecutionEnvelope` could become the compiled result of semantic work structure:

```text
incoming work -> semantic decisions/obligations -> dependencies -> required vantage points
-> authority/independence/continuity constraints -> role composition -> role contracts -> ExecutionEnvelope
```

This does not require arbitrary role invention. A conservative near-term architecture keeps durable, reusable role/vantage profiles and combines them with problem-specific semantic structure to produce dynamic role instances and topology; when no existing profile can satisfy a required vantage, the system should surface an **organizational gap** rather than invent a role silently.

### 2.4 Dynamic role creation with non-transferable authority origin

An existing role may discover a semantic obligation that deserves another logical vantage. Rather than merely "spawning an agent," the role requests a new logical role with a named subject, semantic obligation, delegated authority, and required consequence — and Work Engine derives the child contract. Authority movement itself decomposes into distinct operations: **delegation** (parent retains authority, child may exercise a bounded subset), **transfer** (authority moves from one owner to another), **attenuation** (child receives a strictly narrower grant), **nomination** (child or parent proposes a consequence; another authority admits it). The core invariant:

```text
child authority ⊆ delegable parent authority
```

New logical roles may appear dynamically. **New authority may not.** Authority must trace to an existing legitimate source.

### 2.5 When creating another vantage is justified — and when it is harmful

Dynamic role formation cannot be justified merely by parallelism; creating a new vantage is a costly semantic intervention. Legitimate pressures include:

- **Epistemic separation** — a judgment requires independence from another reasoning context (implementation author ≠ independent reviewer).
- **Context separation** — a subproblem requires large temporary evidence that should not occupy durable parent context.
- **Authority/effect separation** — two powers should not coexist in one role (produce evidence ≠ own semantic consequence).
- **Continuity separation** — a subproblem needs a distinct lifetime or retained history.
- **Work decomposition** — a coherent body of work can progress independently without remaining inside one reasoning loop.

Every split creates a boundary with real costs: lost tacit shared context, handoff reconstruction, coordination, state synchronization, authority complexity, additional runtime cost. A split is likely poor when the child needs nearly all parent context, no compact consequence can cross the boundary, parent and child must constantly negotiate the same mutable state, the proposed child has no distinct epistemic/authority/context/continuity property, authority cannot lawfully move, or the task is too small to amortize composition and coordination cost. The generative criterion:

> **Create another vantage when a stable semantic boundary allows the resulting contexts to know materially less than the original combined context while preserving or improving the required consequence.**

Good decomposition: `role A knows A + contract(B)`, `role B knows B + contract(A)`. Bad decomposition: `A constantly reconstructs B`, `B constantly reconstructs A` — duplicating coupling instead of removing it.

### 2.6 Vantage separation can be partially deterministic

Instead of one opaque "should I spawn another role?" judgment, Work Engine may classify the situation into **must separate**, **must remain**, **separation eligible**, or **semantic judgment required**. Hard constraints can sometimes determine the answer outright — e.g., independence required plus the current role having authored the challenged subject forces separation; non-transferable authority plus a requirement for the same logical owner means the semantic decision cannot move. For optional cases, deterministic machinery supplies evidence (context pressure, shared-state coupling, authority pressure, independence pressure, continuity divergence, subproblem boundary strength, projection cost, coordination cost, capability availability, parallelism opportunity) and the model judges only the remaining, irreducible semantic question — the same recurring Work Engine pattern of using deterministic machinery to establish everything mechanically knowable, then spending inference only on what remains.

### 2.7 Event-scoped organizational reasoning

Ordinary roles should not carry this organizational machinery permanently in context. Most of the time a builder does not need to reason about role decomposition, vantage separation, authority delegation, organizational topology, or role contract generation — that knowledge is cognitive overhead. Instead:

```text
ordinary role context
        v
host observes organizational-decision evidence
        v
organizational decision admitted (as a decision surface, not an outcome)
        v
temporary projection injected: current metrics, applicable constraints, decision-specific skill, available organizational actions
        v
model resolves bounded decision
        v
consequence persisted
        v
temporary projection removed
```

Organizational reasoning becomes an event-scoped capability. The role does not continually ask "should I create another agent?" — externally owned evidence determines when the question is material enough to enter context. A role may nominate such a condition, but nomination is not unilateral topology authority.

---

## Part 3: This Is an Instance of Adaptive Context/Vantage Management

### 3.1 Two dimensions of the same underlying question

Work Engine's context-lifecycle manager already has a conceptual problem that is primarily temporal:

```text
same logical role, same semantic vantage
context epoch N -> pressure/economics -> checkpoint state -> replace context -> context epoch N+1
```

It asks: **when should this same vantage receive a fresh context?** The organizational-topology question this idea adds is different: **is this still the correct context topology for the judgment that now exists?**

```text
TEMPORAL CONTEXT MANAGEMENT        TOPOLOGICAL CONTEXT MANAGEMENT
When should this context           What contexts/vantages
be replaced?                       should exist at all?
```

### 3.2 Context pressure generalizes to context fitness

Token volume is only one kind of context pressure. Others include relevance pressure (retained information no longer matters), semantic-width pressure (too many unrelated decisions coexist), independence pressure (a judgment must not inherit some reasoning), authority pressure (incompatible authority surfaces coexist), continuity pressure (part of the work needs another lifetime), instruction pressure (dormant reasoning frameworks remain loaded), and coupling pressure (a proposed split would create excessive synchronization). This suggests a more general concept — **context fitness**: not merely whether context is full, but whether the current projection remains a good environment for the judgment being performed.

### 3.3 Richer context transitions

The context manager may eventually reason about more than replacement: continue unchanged; temporarily augment (inject event-specific skill/state); contract (remove no-longer-useful projected material); replace (same vantage, new context epoch); open a disposable context (separate temporary evidence lifetime); split vantage (distinct reasoning conditions); instantiate a role (distinct vantage plus semantic authority); retire a vantage (semantic obligation completed). These should not yet be frozen into an API; the important discovery is that they are distinct operations, not variations of one.

### 3.4 Three increasingly strong boundaries

- **Context boundary** — separate information lifetime/context (a builder alongside a reconnaissance context). No independent authority required.
- **Vantage boundary** — separate epistemic or reasoning conditions (a builder alongside an independent reviewer).
- **Role boundary** — separate semantic authority and obligations (a parent role alongside a delegated child role).

The relationship is one-directional: a role boundary generally implies a distinct vantage; a distinct vantage generally requires some context boundary; a context boundary does not imply a new role. This prevents context optimization from automatically becoming organizational proliferation.

### 3.5 Adaptive projection planning

```text
DURABLE WORK ENGINE WORLD
        v
PROJECTION PLANNING
  vantage topology (who should reason?)
  information projection (what should they know?)
  reasoning projection (what temporary skill or decision framework applies?)
        v
CONTEXT LIFECYCLE (when should this projection be regenerated?)
        v
MODEL CONTEXT
```

The existing lifecycle manager operates primarily near the bottom of this stack. This idea extends adaptive management upward into the projection itself.

### 3.6 Unifying principle

> **Do not make a model context permanently carry structure merely because that structure may someday be useful.** Externalize state, authority, role structure, decision requirements, evidence, context-pressure observations, organizational constraints, and reasoning frameworks; project only what the current judgment requires.

This extends "the context window is not the database" into "the context window is not the organization either," and further: the context window is a temporary, purpose-specific projection of the durable semantic world, whose contents, reasoning affordances, lifetime, and partition into logical vantages may change as the structure of the work changes.

---

## Part 4: Grounding Against the Current Implementation

This idea is not purely speculative about context-lifecycle — the current live implementation was inspected directly to establish what already exists versus what this idea adds.

### 4.1 The current live path

```text
token-usage observation
        v
TokenUsagePressureProjector          (deterministic)
        v
ContextPressureController            (deterministic thresholds + hysteresis)
        v
replacement_candidate / critical
        v
LiveContextLifecycleCoordinator
        v
observed-context projection
        v
SemanticContextInferenceRuntime (compiler inference, verifier inference)
        v
checkpoint publication / fences      (deterministic)
        v
transition lease
        v
target model invokes new_context      (sterile actuator)
        v
rehydration / reconciliation
```

The live pressure profile: `approaching` at 65%, `replacement_candidate` at 75%, `critical` at 90%, with `transition_at` bound to `replacement_candidate` and `critical`. `TokenUsagePressureProjector` computes pressure directly from `last.totalTokens / modelContextWindow`; `ContextPressureController` deterministically maps that through the configured bands and hysteresis — there is no model call in that decision. **The model does not currently decide "my context is too full; replace me."** That ownership has already been externalized out of the role.

### 4.2 Where inference actually happens today, and what it does not yet ask

Once pressure triggers lifecycle preparation, `LiveContextLifecycleCoordinator` invokes bounded disposable inference: a semantic compiler produces a continuation candidate (objective and logical progression, current work position, completed consequences, commitments, decisions and premises, authority dependencies, evidence interpretation, unresolved questions, governing instructions, human interaction state, authorized next action); a distinct semantic verifier challenges that candidate on sufficiency, attribution, authority preservation, interaction closure, and source binding. Host code — not the verifier model — derives `accepted`, `unresolved`, or `rejected` (`app-server/src/semantic-context-inference.mjs`, with disposable Codex calls implemented by `CodexAppServerInferenceCapability`).

So context-lifecycle already contains a bounded inference subsystem, but its present semantic question is **"what meaning must survive this context transition, and is that continuation representation sufficient?"** It does not currently ask **"should the semantic organization of the context change?"** — that is the gap this idea's topology layer fills.

There is also a real implementation gap worth naming: the lifecycle design intends that "preservation and retirement are independent decisions, and retirement should happen only when replacement is semantically safe **and economically advantageous**," but the live implementation's replacement trigger is the fixed pressure policy followed by semantic continuation verification — expected remaining work, transition cost, and projected savings do not yet enter the live decision. A prior lifecycle audit observed a real replacement that removed roughly 202k live tokens without demonstrating net token savings, because compiler/verifier/reconciliation overhead was large. Today: `replacement candidate ≈ token-pressure threshold`, `semantic inference ≈ can we preserve/reconstruct meaning safely?` — not yet `replacement decision = pressure + expected remaining work + transition cost + projected savings + semantic fitness`.

### 4.3 The current ownership boundary

Context-lifecycle's own design already states what it owns: thread/turn observation, token/context/cost telemetry, bounded context projections, inspection scheduling, semantic compiler/verifier invocation, checkpoint storage, transition readiness, transition coordination, transition classification, rehydration, lifecycle ledger. It explicitly does **not** own role objective, domain truth, workflow authority, human approval/preference, canonical decisions, or authority grants.

This is exactly where this idea's topology layer hits a real seam. A **context replacement** (same role, same authority, same semantic obligation, same vantage, old context → new context) can legitimately belong to context-lifecycle. A **new vantage or role** changes semantic ownership, authority topology, workflow topology, information boundaries, and continuity ownership — context-lifecycle cannot simply decide "you should create Builder B and delegate D to it" without violating its current ownership boundary.

---

## Part 5: Proposed Architecture — Observer, Lifecycle, and Topology

### 5.1 Neither service owns the other's decision surface — both consume one observer

Context-lifecycle should not sit upstream of topology, and should not gate topology's own applicability question — that would make lifecycle a decision authority over a domain (organizational fitness) it does not own, the same "coordination authority absorbing domain authority" failure Part 1 warns against. Once the Context Observer is recognized as a shared, more primitive layer beneath *both* consumers (§5.3), the ownership separates cleanly:

```text
Context Observer
    owns observations

Context-lifecycle
    owns temporal-context applicability
    ("is a temporal context transition now warranted?")

Vantage/topology service
    owns organizational-decision applicability
    ("is an organizational judgment now warranted?")
    — independently of lifecycle, not gated by it

Active semantic role
    owns the irreducible semantic judgment where one remains

ExecutionEnvelope / organizational authority
    owns admission of the organizational consequence

Shared transition infrastructure (Part 7)
    owns safe ordering/fencing between whatever either consumer admits

Projection/runtime machinery
    realizes the admitted result
```

Lifecycle and topology are independent, parallel consumers of the same observer — neither the parent of the other, neither gating the other's applicability question. Both may conclude a transition is warranted at different times, for different reasons, over the same underlying reasoning environment, which is exactly why a shared transition-admission layer (Part 7) is required rather than either consumer coordinating the other directly.

### 5.2 Three ownership levels for an organizational decision

1. **Topology service: observe (via the shared observer) and nominate.** Not context-lifecycle. It consumes `ContextObservation` plus semantic/work/authority state to derive its own `VantageSeparationEvidence` — coupling, continuity divergence, independence requirements, delegability, separation opportunity — and applies hard constraints (independence requires separation; non-transferable authority forbids transfer; atomic transition forbids splitting; workflow policy freezes topology). From these it determines only whether an organizational question exists at all — keeping this cognitive burden out of the normal role, and out of context-lifecycle.

2. **The active semantic role: judge the irreducible work question.** When the case is not mechanically determined, the current role has the best vantage to answer whether a subproblem is independently coherent, how much accumulated understanding is genuinely required, whether separating it would destroy useful reasoning continuity, and whether the semantic boundary is stable enough to hand off through a contract. A JIT projection can temporarily add an "organizational decision" surface — mechanically established facts (separation lawful, authority delegable, child capabilities satisfiable, estimated projection reduction, mutation overlap, shared dependencies, coordination-cost evidence) plus the one unresolved question — with possible outcomes retain, disposable context, distinct vantage, or delegated logical role. The agent need not know any of this until that decision exists.

3. **Organizational authority: admit the organizational consequence. `ExecutionEnvelope` materializes it — the two are not the same thing.** Even if the active role concludes "split this," that is a semantic nomination, not unilateral authority to rewrite the organization. Something must verify that the parent may delegate this authority, the child grant is correctly attenuated, the workflow permits organizational branching, independence constraints hold, no competing authority owner exists, and required capabilities can be realized — that verifying, selecting act is **organizational authority**. `ExecutionEnvelope` is what receives the resulting new revision: the immutable materialization of that selection, never itself the authority that selects it. Role judges semantic usefulness; role does not mint organization; and the envelope does not mint organization either — it records what organizational authority already admitted.

**Wave 1 reconciliation finding, corrected from an earlier draft's bundling of these two roles into one phrase (checked directly against `organizational-execution-envelopes-reconciliation.md`, not assumed from the fork research that first flagged this relationship).** That reconciliation — already accepted 2026-09-14, authorized for design work, not yet implementation — independently arrives at the identical layering and draws it as two distinct boxes: "Organizational authority / problem specification — owns what organization is permitted or required and any open selection judgment among valid candidates" feeding "`ExecutionEnvelope` — the immutable materialization of that selection for one problem... not itself the authority that selects it," and names the organizational-authority layer as the one thing none of `role-compiler-proposal.md`, `RoleRealization`, or `hierarchical-planning-and-multi-supervisor-orchestration.md` answers — its own words: "missing." This section's three-level model is a candidate design contribution toward exactly that named gap, not new territory and not a duplicate of anything already built: level 1 (topology nominates) and level 2 (active role judges) together constitute organizational authority's own selection process; level 3, corrected above, is `ExecutionEnvelope` receiving that selection's consequence. The reconciliation's own **available / authorized / required / selected** vocabulary — already proven at the realization layer (`RoleRealization`) and explicitly named as needing an "organization-level analogue" that "survives, folded into the residue" — maps cleanly onto this section's existing content rather than requiring new vocabulary: *available* organizational components are what the topology service's `VantageSeparationEvidence` surfaces as separable; *authorized* organizational composition is this section's own hard-constraint set (independence requires separation, non-transferable authority forbids transfer, workflow policy freezes topology); *required* organizational consequence is the active role's own irreducible judgment; *selected* is the specific structure organizational authority admits into a new `ExecutionEnvelope` revision. This should be surfaced back to `organizational-execution-envelopes-reconciliation.md` as a candidate answer, not folded in silently — the reconciliation document, not this idea, is the authorized owner of that residue.

**What happens after admission is deliberately left open.** After organizational admission, the resulting topology revision is realized through the shared projection/runtime machinery; each resulting logical context then becomes a subject of ordinary context-lifecycle management going forward. Whether that projection/runtime machinery is a distinct owner from context-lifecycle, or context-lifecycle acting in a narrower "realize this admitted projection" capacity distinct from its own applicability judgment, is an open seam (Part 12), not settled here — context-lifecycle should not be assumed to own realization merely because it already holds the relevant mechanism today.

### 5.3 A shared, intentionally stupid Context Observer

Both consumers (temporal lifecycle, organizational topology) should sit atop one shared observer rather than each independently inspecting the same underlying reality — otherwise both services eventually reimplement the same inspection logic and produce two subtly different representations of the same context. The observer should own **facts about the current reasoning environment**, never their meaning:

```text
ContextObservation
  identity:      role, thread, context window, binding revision
  usage:         live tokens, input tokens, context-window capacity, cache behavior, recent growth rate
  composition:   active skills, governing instructions, projected durable state, visible evidence classes,
                 temporary material, unresolved interactions
  activity:      current work unit, recent turns, tool activity, mutations, active dependencies
  lifetime:      age, continuation count, checkpoint history, replacement history
  relationships: subjects touched, authority surfaces involved, other active roles/vantages
```

Some of these are already directly observable; others become available only as Work Engine externalizes more semantic structure. There are really three layers, not two, and each consumer owns its own middle layer independently:

```text
ContextObservation                          ContextObservation
      v                                      + semantic/work/authority state
LifecycleEvidence                                  v
(token pressure, replacement economics,     VantageSeparationEvidence
 continuation safety, expected remaining     (coupling, continuity divergence,
 work)                                        independence requirements,
      v                                       delegability, separation opportunity)
LifecycleDecisionSurface                           v
("replace now?")                            OrganizationalDecisionSurface
                                             ("is organizational judgment warranted?")
```

Token usage is an observation; token pressure is a deterministic *lifecycle-owned* derivation; "replace now?" is lifecycle's own decision. Touched subjects, an independence requirement, and mutation overlap are observation-adjacent facts; vantage-separation evidence is a deterministic *topology-owned* derivation from them; "is organizational judgment warranted?" is topology's own decision, only escalating to JIT semantic judgment when unresolved. Metrics like semantic-width pressure, coupling, or projection reduction therefore do not belong in the observer itself — they belong specifically to whichever consumer's own `*Evidence` layer derives them. The observer stays intentionally boring; the derivation and the decision both stay with the consumer that owns the domain in question.

JIT injection becomes a normal consumer pattern rather than a lifecycle special case: an observation feeds a topology analysis; if organizational judgment is warranted, the projection service temporarily adds an organizational skill and the exact decision surface to the existing, still-alive context; the role resolves the decision; the temporary projection is removed. If the decision is "retain," nothing structural happens. If it is "create a separate vantage," organizational admission and compilation happen, and only afterward is context management asked to realize the resulting projections.

### 5.4 What "context" actually names

This clarifies several distinct things carrying the same word: the **provider context window** (physical/model runtime state); the **observed context** (what Work Engine can truthfully observe about it); the **context projection** (what Work Engine intentionally supplies to a model); the **semantic vantage** (the conditions under which judgment occurs); and the **logical role** (vantage + authority + obligation). The observed-context concept — originally built as evidence for safe context replacement, because Work Engine cannot claim knowledge of a provider's hidden effective prompt — is more important than its original purpose suggests: it can become the canonical, shared observation substrate for adaptive context *and* vantage management generally, not merely a replacement-safety mechanism.

---

## Part 6: Organizational Dynamics as a Governed Policy Surface

Once organizational change is represented as an admitted capability over context/vantage topology, "dynamic" versus "static" stops being an architectural fork and becomes policy over the same substrate:

```text
same semantic substrate + same role primitives + same projection machinery + same lifecycle machinery
        + organizational-dynamics policy
        v
different execution regimes
```

An experiment requiring a fixed organization does not need a stripped-down reimplementation of Work Engine — it simply receives an envelope whose permitted topology transitions are closed. Illustratively (not proposed syntax):

```yaml
organizational_dynamics:
  context_augmentation: allowed
  disposable_contexts: allowed
  vantage_split: denied
  role_creation: denied
  authority_delegation: denied
  topology_revision: denied
```

```yaml
organizational_dynamics:
  context_augmentation: allowed
  disposable_contexts: allowed
  vantage_split: allowed
  role_creation: allowed
  authority_delegation: attenuated_only
  topology_revision: bounded
```

These are not different orchestration systems — they are different authority ceilings over the same organizational transition vocabulary, in the same sense that knowledge of a mechanism and authority to exercise it are already kept separate elsewhere in Work Engine. A static organization becomes the easiest special case of the dynamic model (constrain the transition set to ∅), rather than a static workflow architecture plus a separate dynamic-agent architecture bolted alongside it — usually a sign the abstraction is healthy.

This gives experiments something valuable: organizational plasticity becomes a controllable experimental variable. Holding everything else fixed (coordinate, objective, model realization, evidence, initial roles, capabilities), one can vary only the organizational regime — A: topology frozen; B: temporary context decomposition allowed; C: dynamic vantage formation allowed; D: bounded role composition/delegation allowed — and ask whether organizational plasticity actually improves accepted work, token economics, error rates, context pressure, or recovery. Because topology changes are durable state transitions rather than invisible agent behavior, exactly what changed and when remains known. There are likely several useful intermediate regimes, not just static/dynamic — fixed topology, adaptive projection only, disposable context branching, dynamic vantage branching, bounded role/authority composition, revisionable organization — treated as execution policies chosen per workflow, not maturity stages a system must climb.

---

## Part 7: Transition Coordination — the Concurrency Seam Between Lifecycle and Topology

### 7.1 The race

Both context-lifecycle and organizational topology can change the model's effective reasoning environment, so their transitions need shared admission, not mutual awareness. The dangerous sequence:

```text
role running -> topology service detects separation pressure -> JIT organizational judgment injected
-> model decides "create vantage B" -> authority/state begins moving
                          (meanwhile) lifecycle sees 78% pressure -> issues new_context
                          -> mixed transition state
```

At that point several things may be half true: the organizational decision exists but is not durable; the child contract may not yet be compiled; parent authority may be conceptually but not durably attenuated; the parent projection may not yet reflect the new boundary; child state may not be reconstructable; and a lifecycle checkpoint could capture an intermediate world that should never be resumable. This is exactly the kind of state Work Engine's own conventions elsewhere refuse to manufacture.

### 7.2 Shared admission, not peer coordination

The two services should not ask each other "are you busy?" — that creates peer-service coupling and races of its own. Both should instead consume a shared transition-admission/fencing layer, generalizing the pattern the lifecycle implementation already has (transition gate, preparation fences, lifecycle reserve permits, transition leases) from the narrower scope of a *context replacement lease* to the broader scope of a **reasoning-environment transition lease**:

```text
                 shared transition authority
             +------------+------------+
             |                         |
       lifecycle transition      topology transition
       (replace context)         (split/create vantage)
```

### 7.3 Two fence types, and the invariant that governs both

- **Decision-episode fence** — protects an unresolved semantic judgment in general (not only organizational ones; any event-scoped semantic decision whose result would be lost by context replacement). While `organizational_decision.status = active`, `new_context` is prohibited; the lifecycle manager may still observe pressure and compile preparatory evidence, but retirement admission stays closed until the decision is durably settled or aborted.
- **Topology-transition fence** — protects the actual organizational state change: accept split → acquire topology transition → compile authority/contracts/projections → publish coherent successor organization → activate → release. During that window, lifecycle cannot checkpoint or replace the parent against a stale topology revision.

**Revision binding**, not service signaling, is the actual mechanism: lifecycle preparation binds to an exact pair of authoritative revisions (e.g. `(ExecutionEnvelope revision, parent projection revision)`); if the topology service commits a new pair during that window, the lifecycle transition prepared against the old pair fails promotion as stale and must recompute against the successor world. The topology service need not tell lifecycle anything happened — the changed authoritative revision invalidates the prepared transition mechanically. This mirrors the exact CAS/fencing discipline already established elsewhere in Work Engine's own services (claim-evidence's `expected_state`-against-heads checks; `review-episode`'s generation/predecessor-revision checks) — not a new concept, an extension of one already proven.

The reverse race matters too: topology should not inject a new organizational judgment while lifecycle retirement is already in progress for that context. It should either defer the organizational decision until the successor context is reconciled, or — if policy ranks it higher — invalidate and abort the lifecycle preparation and keep the context. That arbitration may itself be partly deterministic (e.g., critical provider pressure with no safe deferral outranks an ordinary in-progress topology episode) without needing to be permanently hard-coded.

The governing invariant, general enough to cover this race and likely others:

> **No transition may activate a successor reasoning environment from a world revision that ceased to be authoritative while that transition was being prepared.**

---

## Part 8: Candidate Generative Principles (compact form)

> Place each semantic decision with the narrowest authorized logical role whose vantage satisfies the evidence, ownership, continuity, independence, and consequence requirements of that decision. When those requirements and role properties fully determine the owner, project that authority mechanically. When they do not, preserve the unresolved organizational decision rather than hiding it in workflow procedure.

> Create another vantage when a stable semantic boundary allows the resulting contexts to know materially less than the original combined context while preserving or improving the required consequence.

> Work Engine should manage model context as an adaptive projection of durable semantic state. It may dynamically change that projection's information, reasoning affordances, lifetime, and topology when evidence establishes that a different context or vantage would improve correctness, independence, locality, continuity, or execution economics. Semantic decomposition remains model judgment where necessary; mechanically determined consequences should be compiled into role contracts, authority grants, context projections, and runtime realizations.

> Externalize the semantic world; dynamically compile the smallest truthful set of vantages needed to judge it.

> No transition may activate a successor reasoning environment from a world revision that ceased to be authoritative while that transition was being prepared.

---

## Part 9: Important Distinctions

This idea depends on preserving all of the following. Any implementation that collapses them defeats its purpose:

```text
capability                != authority
visibility                != ownership
ownership                 != authority over every related consequence
organizational hierarchy  != semantic authority
same actor                != same decision authority
delegation                != authority transfer by default
nomination                != admission
admission                 != execution
runtime realization       != logical role
workflow position         != semantic ownership
mechanical eligibility    != semantic fitness when semantic judgment remains
temporal fitness          != topological correctness (a context can be fresh and still be the wrong shape)
context boundary          != vantage boundary != role boundary (implication runs one direction only)
```

---

## Part 10: Non-Goals

This idea does not currently propose: replacing current role manifests; replacing workflows; generating arbitrary organizations automatically; assigning semantic authority using model confidence scores; inferring authority from observed behavior; treating available capabilities as authority; allowing runtime models to expand their own authority; making every decision dynamically routed; eliminating durable named roles; collapsing human authority into role eligibility; turning organizational design into an optimization problem before semantic correctness is established; a peer-to-peer coordination protocol between context-lifecycle and a topology service; a concrete schema for the context observer; final naming or API shape for the transition-fence mechanism; or implementing a universal authority language before the recurring structure has been reconciled against current Work Engine artifacts.

---

## Part 11: Relationship to Other Work Engine Ideas

Not reconciled here — named so a future reconciliation pass has a starting map, per this idea's own discipline of not pre-deciding seams.

- **`hierarchical-planning-and-multi-supervisor-orchestration.md` — the single most consequential relationship in this document, checked against Part 13's own text and now shaping it directly.** That idea already has a real, detailed `Preplanner → Orchestrator → Branch planner → Branch plan → Supervisor → Builder` hierarchy, with an explicit invariant (§3) that the orchestrator realizes an accepted topology "without acquiring the semantic judgment owned by those actors," a supervisor that realizes rather than replans its branch (§7), and an already-working topology-conflict/replanning route (§12) headed "Branch planner / supervisor." Part 13 is not a competing organizational shape — it is the compilation layer between that document's own semantic-planning topology and the concrete execution organization that realizes each of its accepted layers, and has been rewritten to make that explicit rather than jump straight from Orchestrator to Supervisor as an earlier draft did. Not yet a full, formal reconciliation — see Open Questions 22-25 for exactly what remains unchecked.
- **`proposal-decision-gated-implementation-compilation.md`** — Part 13's "branch" and "workstream" vocabulary sits directly downstream of the `routing.vs.admission` and `decision-gated.vs.hierarchical-orchestration` seams this same session already closed (recorded in both that document and `hierarchical-planning-and-multi-supervisor-orchestration.md` directly) — the sealed-decision-set / implementation-contract layer those rulings placed beneath the branch plan is exactly what Part 13's leaf-level "execution" step would compile down into. Not reconciled here, but the two already-closed rulings should be read before Part 13 is extended further.
- **`organizational-execution-envelopes.md` / `app-server/docs/organizational-execution-envelopes-reconciliation.md` — Wave 1, reconciled 2026-09-15.** That idea is not merely a sibling document to check for overlap; it was already reconciled and **accepted 2026-09-14, authorized for design work (not implementation)**, with its own reconciliation naming an explicit, still-unfilled gap: "Organizational authority / problem specification... missing" between existing orchestration and `ExecutionEnvelope`. Part 5.2's three-level model is a candidate design contribution toward exactly that named gap, not a competing architecture and not something to build independently — corrected in this pass to stop bundling "organizational authority admits" and "`ExecutionEnvelope` materializes" into one phrase, since the reconciliation's own diagram keeps them as two distinct boxes. Part 5.2's content maps directly onto the reconciliation's own **available/authorized/required/selected** vocabulary, which that document explicitly says needs an "organization-level analogue." This should be surfaced back to the reconciliation document as a candidate answer, not merged silently — that document remains the authorized owner of this residue. Part 2.5-2.6 (vantage-separation criteria) remains genuinely outside this idea's scope: its own "Adoption boundary" section explicitly excludes "invent arbitrary new roles" and dynamic team synthesis from its first vertical, confirmed by direct reading, not assumed.
- **`role-compiler-proposal.md` — Wave 1, reconciled 2026-09-15.** Not a duplicate of Part 2.2 and not competing with it, but the boundary needs to be exact: `role-compiler-proposal.md`'s compiler renders projections from an *already-authored* `structure.yaml`, explicitly never deriving semantic content (its own invariant 5, "No fabricated semantics"); Part 2.2 proposes mechanically *deriving* a role contract's content from upstream primitives (obligation, vantage, authority grant) that precede any authored `structure.yaml` at all. These sit in adjacent pipeline stages — Part 2.2's output is a candidate *source* for `structure.yaml`'s role-profile fields, not a second renderer. The real constraint, per that document's own "must not create a second owner for relation semantics" warning: Part 2.2's derivation must express its output in the Agent Environment Graph's existing relation vocabulary (`bound_by`/`may_invoke`/`may_observe`/`may_mutate`/`owns`/`consumes`/`emits`/`mediated_transitions`/`forbidden_from`), never a parallel one. Separately, `organizational-execution-envelopes-reconciliation.md` already classifies "reusable role profile composition" as a `COUPLED_RECONCILIATION` between these two documents, not owned by either alone — any future design touching Part 2.1's role-primitive composition must reconcile with both, not just one.
- **The semantic-context-lifecycle-manager design** — Parts 3, 4, and 7 directly extend and constrain that system; Part 4's findings are current-implementation facts, not proposed changes, and any future work here must be checked against that design document's own current text before this idea is treated as settled.
- **claim-evidence's `expected_state`/heads mechanism and `review-episode`'s writer-generation mechanism** — Part 7's reasoning-environment transition lease is a direct generalization of a fencing/revision-binding pattern already proven in both; this is corroborating precedent, not a coincidence, and any future design should reuse rather than reinvent it.
- **claim-evidence's `nominate_impact`/`open_refresh_episode`/`publish_refresh_judgment` contract (`proposals/evidence-lineage/claim-maintenance-and-reliance-propagation/operation-contract-surface.md`) and `claim-maintenance-and-reliance-propagation-reconciliation.md`** — Part 13.5 proposes reusing this already-passed contract to materialize planning-derived facts as revision-bound claims, cheapening 13.4's reconnaissance phase without granting claim-evidence any planning authority. This requires a new domain profile (`planning-facts-v1`, not yet designed) and is a genuinely separate, unreconciled relationship from the fencing-precedent one above — see Open Question 27.
- **`claude-runtime-adapter-and-context-ownership-pilot.md`** — a real, code-grounded sibling consumer of the same context-lifecycle substrate Part 4 cites (`context-transition-lease.mjs`, `context-pressure-controller.mjs`, `context-lifecycle-evidence.mjs`), answering a different (empirical, provider-adapter) question. Not a joint-reconciliation candidate now, but worth checking once the Context Observer schema (Part 5.3) is concretized, since a Claude-shaped observation source would need to fit it.

---

## Part 12: Open Questions for Future Reconciliation

**On authority and vantage:**
1. What durable object grants the authority ceiling from which projected role authority is derived, and how is projection prevented from minting authority?
2. Where are semantic decision types defined — domain-owned contracts, workflow artifacts, plan-IR structures, role contracts, or another object?
3. Does the existing role manifest already contain sufficient vantage structure, or does this require a new source object?
4. Can authority projection change during execution as evidence, context, or role availability changes — and if so, which decisions can move and which are continuity-bound?
5. How is genuinely non-transferable semantic authority represented, and does runtime replacement preserve logical-role authority without transferring it?
6. What distinctions are required among delegate, nominate, advise, admit, and execute, concretely?
7. How is required independence expressed without making runtime topology itself semantically authoritative?
8. When several roles satisfy the same vantage requirements, is selection deterministic policy, cost optimization, capability resolution, organizational judgment, or domain-specific?
9. Does an unsatisfied vantage requirement become an explicit organizational-compilation failure, and can the compiler propose a missing role without acquiring authority to create one?
10. Under what conditions may two decision surfaces share one logical role, and which incompatibilities force distinct roles?
11. Are human decision owners represented through the same vantage grammar, a separate authority layer, or both?
12. Can execution evidence determine that a previously semantic authority requirement can safely become deterministic — and if so, how is that transition proposed and authorized without letting the measurement system rewrite its own authority model?

**On adaptive context/vantage topology:**
13. **Partially answered, Wave 1 (2026-09-15).** `organizational-execution-envelopes-reconciliation.md` (accepted 2026-09-14, its own residue authorized for design work) independently draws the same layering this question asks about, as two distinct boxes: an "Organizational authority / problem specification" layer that owns the open selection judgment (named there as a "missing" owner), feeding an `ExecutionEnvelope` that is "the immutable materialization of that selection... not itself the authority that selects it." So: authority projection is neither part of organizational compilation nor a projection *over* an already-formed envelope — it is a prerequisite to forming one, and Part 5.2 (corrected in this pass to stop bundling admission with materialization) is a candidate design contribution toward that named, still-unfilled gap. Still open: whether this candidate should actually become the reconciliation document's own answer, which requires that document's own author/owner to accept it, not this idea to assert it.
14. Are workflows authored structures constrained by authority projection, or can some workflow topology itself eventually be compiled from semantic dependencies?
15. What is the concrete schema for the context observer, and which of its fields are genuinely mechanically observable today versus requiring new externalized state?
16. Where should the decision-episode fence and topology-transition fence mechanism live — inside context-lifecycle itself, a new shared service, or the ExecutionEnvelope authority?
17. Beyond illustrative examples, how is arbitration decided when lifecycle-critical pressure and an in-progress topology episode conflict?
18. Should the observed-context substrate be promoted to a formally shared cross-service dependency now, or allowed to grow organically from its current lifecycle-scoped form?
19. Who realizes an admitted organizational/topology revision concretely — a projection/runtime owner distinct from context-lifecycle, or context-lifecycle itself acting in a narrower "realize this admitted projection" capacity kept separate from its own temporal-applicability judgment? Deliberately left open in Part 5.2 rather than assumed either way.
20. What is the smallest real current workflow from which a deterministic authority projection could be reconstructed without changing behavior?
21. Can the proposed abstraction be demonstrated across one non-software domain without adding domain-specific rules to the projection mechanism?
22. **Partially answered, not fully reconciled.** Checked directly: `hierarchical-planning-and-multi-supervisor-orchestration.md`'s existing `Preplanner/Orchestrator/Branch planner/Branch plan/Supervisor/Builder` hierarchy does already own semantic planning topology, and its §3/§7/§12 already separate that from execution realization and already have a working topology-conflict/replanning route — Part 13 now treats `auto-org` as the compilation layer underneath that hierarchy, not a competing one, and has been rewritten accordingly. What remains unchecked: whether this reframing survives a full, formal reconciliation pass against that document's complete text (not just the sections read so far), and whether any other section of it already anticipates organizational compilation in some form this idea hasn't found yet.
23. Does that document's own branch-plan concept (§6) already anticipate being realized by more than one execution vantage, or does it implicitly assume one supervisor per branch with no further compiled decomposition beneath it? Part 13.9's recursion depends on the answer, and this was not checked in the sections read so far.
24. What exactly must a hierarchical plan expose (per Part 13.12's list — subject, dependencies, consequence, shared invariants, authority ceiling, required capabilities, independence requirements, integration boundary, continuity constraints) for the first organizational layer to be mechanically compiled rather than authored — does `hierarchical-planning-and-multi-supervisor-orchestration.md`'s own branch-plan output (§6) already carry enough of this list, would it need extending, and does the current planning/Plan-IR architecture already carry any of this independently?
25. **Partially answered.** `hierarchical-planning-and-multi-supervisor-orchestration.md`'s §12 route is already explicitly headed "Branch planner / supervisor" — a supervisor discovering a topology conflict mid-execution is already an anticipated source, not something Part 13.6 needs to extend. What's still unchecked: whether the route's "current execution evidence" input already accommodates the specific shape of evidence an organizational-*realization* discovery would produce (e.g., "these two obligations turned out to share state" or "a capability I need doesn't exist"), or whether that's a narrower evidence class than what the route was designed around.
26. How much freedom does organizational-realization authority actually have to repartition accepted semantic obligations before it becomes a planning transformation in disguise (Part 13.10)? Grouping and separating whole accepted obligations along their own existing boundaries is clearly organizational-realization territory; a realization that cuts *across* an obligation's own internal boundaries (splitting one obligation's work between two vantages, or merging parts of two different obligations into one vantage) is not yet placed. Part 13.10 states a candidate boundary (group/separate/assign vantages, never alter semantics/dependencies/create/eliminate obligations) but does not adopt it as final, and does not decide whether cross-cutting realizations should ever be permitted at all.
27. **Partially answered.** Sol proposed a narrow three-part reconciliation pass connecting Part 13.5 to the wider planning and claim-evidence architecture. (a) **Planning → claims — mostly answered, deliberately not overclaimed.** `hierarchical-planning-and-multi-supervisor-orchestration.md` §6's own declared branch-plan field list and §16's naming of "branch-plan revisions" as required durable state together give a `planning-facts-v1` profile enough structure to project from today and a real, revision-producing transition to attach to. This proves `planning-facts-v1` *can* materialize something useful now; it does not prove §6's schema already contains every fact organizational compilation will eventually want (continuity constraints, independence requirements, capability requirements, projection/evidence-locality boundaries) — that completeness question remains Open Question 24's, not silently closed here. The profile must also be constrained to pure projection of explicitly declared fields, never inference of a semantic conclusion the plan did not itself assert — otherwise the materializer becomes a second, unaccountable planner. (b) **Claims → organizational compilation — collapsed to a sharper, out-of-scope-for-Part-13.5 question, per Sol's own recommendation to stop drilling here.** Two claim classes behave differently (planning-derived = durable structural input; execution-derived = adaptive realization evidence, and can only become structural through the owning authority's own transition, never by self-promotion). 13.3 already shows the root organizational layer reads its structural inputs directly from the accepted branch plan, not through claims. At recursive depth (13.9), claims must not become a substitute authoritative source merely because the branch plan is inconvenient to reach — Part 2.3/5.2's own already-named `ExecutionEnvelope` revision is the authoritative object each admission produces, and any future `organizational-facts-v1` profile (unproposed, named only as a structural possibility) would materialize a provenance-bearing index over that revision, never stand in for it. The genuinely remaining unknown is which authoritative object a recursively admitted organizational layer actually publishes — already named by Part 12's own item 13 ("is authority projection part of organizational compilation, a prerequisite to it, a projection over an already-formed envelope, or a separate concern?" — since partially answered by Wave 1's reconciliation, see item 13's current text), with item 19 covering the narrower question of who realizes it once admitted — and that question belongs to reconciliation against `organizational-execution-envelopes.md` and `role-compiler-proposal.md`, not to further extension of Part 13.5 in isolation. Sol's assessment, independently checked against this document's own Part 2.3/5.2/12 text rather than accepted at face value: correct. This is the natural stopping point for the claims thread. (c) **Execution evidence → claims → replanning — mostly answered, with one requirement surfaced and then refined.** The reuse chain is complete in shape, and requires branch-plan acceptance and its planning-fact projection to publish as one atomically-visible admission — refined per Sol's correction to mean atomic admission-visibility, not necessarily one physical storage transaction, since branch-plan state and claim-evidence may remain separate services. Unchecked against any real implementation, since none exists yet. Whether high-volume execution evidence needs its own batching/throttling discipline before reaching the topology-conflict route remains fully open.

---

## Part 13: The `auto-org` Execution Mode — A Worked Synthesis

This section works through what Parts 1, 2, 5, and 6 imply when applied recursively to an admitted hierarchical plan. **Checked directly against `hierarchical-planning-and-multi-supervisor-orchestration.md`** — the relationship flagged in Part 11 as this document's highest-priority open question — and revised accordingly: that document already has a real `Preplanner → Orchestration plan → Orchestrator → Branch planner → Branch plan → Supervisor → Builder` hierarchy, with an explicit invariant that the orchestrator "realizes an accepted orchestration topology... without acquiring the semantic judgment owned by those actors" (`hierarchical-planning-and-multi-supervisor-orchestration.md`, §3) and a supervisor that "realizes one accepted branch plan" rather than replans it (§7, "topology-level replanning" is explicitly named as something that moves *upward*, away from the supervisor). What follows treats that hierarchy as already-settled semantic-planning structure and asks only what compiles the execution organization beneath each of its layers.

### 13.1 What `auto-org` does not mean, and what it actually is

It does not mean giving one orchestrator a goal and letting it invent arbitrary agents, and it does not mean inventing a competing hierarchy alongside `hierarchical-planning-and-multi-supervisor-orchestration.md`'s own. It means: **given each layer of an already-accepted hierarchical plan, compile the smallest lawful execution organization that realizes it** — the same chain already established in this document (semantic obligations → required vantages → role composition → bounded authority projection → ExecutionEnvelope → runtime realizations), applied once per accepted layer, never in place of the planning authority that produced that layer.

### 13.2 Two topologies, not one, and they are owned differently

```text
SEMANTIC / PLANNING TOPOLOGY                    EXECUTION / ORGANIZATIONAL REALIZATION

What work exists?                               Given that admitted work structure,
What are the workstreams?                       how many logical vantages should realize it?
What depends on what?                           Which capabilities does each vantage need?
What integration boundaries exist?              Which contexts should be kept separate?
What implementation decisions are               How should authority the plan already
  separately planned?                             granted get projected into those vantages?
```

`hierarchical-planning-and-multi-supervisor-orchestration.md` already owns the left column, in full, through its Preplanner/Orchestration-plan/Branch-planner/Branch-plan chain. `auto-org` owns only the right column: the compilation step between an accepted layer of that plan and the concrete role instances, contracts, contexts, and capability grants that realize it.

```text
Hierarchical planner
    owns semantic work structure
            v (admitted plan)
Auto-org / organizational compiler
    derives lawful execution organization
            v
Supervisor / builders / specialist vantages
```

A planner can say "A consists of implementation obligation X, migration obligation Y, shared invariant Z, and integration consequence Q" without also saying "instantiate one supervisor, two builders, and one compatibility specialist." Those role instances are the compiled organizational consequence of the accepted plan — exactly what authority projection and role-contract compilation (Parts 1 and 2.2) add *underneath* an already-existing hierarchy, not a replacement for it.

### 13.3 The first organizational layer compiles from an accepted branch plan — through the existing planning chain, not around it

The earlier draft of this section jumped directly from `Orchestrator` to `Supervisor`. That skips a layer `hierarchical-planning-and-multi-supervisor-orchestration.md` already separates correctly — the branch planner, and the accepted branch plan it produces:

```text
ROOT PLAN
    v
ORCHESTRATOR                         (realizes the accepted orchestration topology;
    v                                 does not acquire planning authority by doing so)
instantiate branch-planning vantages
    v                    v
BRANCH PLANNER[A]   BRANCH PLANNER[B]
    v                    v
accepted branch plan A   accepted branch plan B
    v                    v
organizational compilation (topology-realization authority begins here)
    v                    v
SUPERVISOR[A]        SUPERVISOR[B]
```

The branch-planning obligation may already be satisfied by an accepted artifact, in which case no new branch-planner inference is required and the supervisor begins immediately — no inference needed is not the same as no owning transition existing; the branch-planning layer still ran, its output was just already sufficient. Once an accepted branch plan exists, the orchestrator can mechanically derive enough from its declared subject, dependencies, expected consequence, and constraints to instantiate a supervisor contract:

```text
accepted branch plan A: subject=A, dependencies=[...], consequence=[...], constraints=[...]
        v
Supervisor[A]:  subject=A, observes=A's accepted plan + shared contracts + dependency edges,
                authority ceiling=A, capabilities=required-by-A, continuity=A's execution lifetime
```

The contract is a compiled consequence of the accepted plan's own declared structure — exactly the role-contract compilation Part 2.2 already describes, applied to planning output rather than to a role someone hand-authored.

### 13.4 Role formation as a bounded reconnaissance phase — scoped to realization, not to replanning

A newly instantiated supervisor need not begin executing immediately. It can be given a bounded formation phase first, but what it may conclude during that phase is bounded by the semantic/execution distinction in 13.2 — and by a second distinction that matters just as much: **gathering reconnaissance evidence is not itself an organizational decision.** The supervisor accumulates evidence in its own retained context; per Part 5.2's own three-level model, it is still the topology service that determines, from that evidence, whether an organizational-decision surface becomes active at all — the supervisor resolves only the irreducible semantic remainder, if and when one exists, not a standing responsibility to reason about topology throughout formation:

```text
instantiate Supervisor[A], given accepted branch plan A (obligations A1, A2, A3; A1->A2; A3 independent)
        v
branch-local reconnaissance against real repository/runtime evidence
        v
topology service derives evidence from this reconnaissance; determines whether
an organizational-decision surface is warranted (per Part 5.2) -- not yet the
supervisor's own judgment
        v
        +-- accepted plan still holds:                  +-- accepted plan is falsified:
        |   A1+A2 need one retained context,             |   A3 is not actually independent --
        |   A3 can live in a separate vantage             |   it creates a new A3->A1 dependency
        |                                                  |
        v                                                  v
   organizational-realization authority                nominate topology conflict
   resolves the surface (JIT, event-scoped,             (per hierarchical-planning-and-multi-
   per 13.13 -- not a standing responsibility)           supervisor-orchestration.md §12 --
        v                                                see 13.7 below)
   admit child topology:
   Supervisor[A] -> Builder[A1+A2], Builder[A3]
        v
   same supervisor continues executing A
```

The left branch has not changed the plan — A1, A2, and A3 still exist, their dependencies and integration contract are unchanged, only *how* to realize them has been decided. The right branch is not a realization question at all; it is evidence that the accepted semantic structure itself is wrong, and the supervisor's only lawful move is to nominate that discovery through the existing conflict route, not to silently repair it by inventing a new semantic topology of its own. Resolving the left branch is an exercise of **organizational-realization authority** — a domain of authority that exists independently of any policy mode; a supervisor that instead resolves the right branch on its own has quietly become an unaccountable branch planner — the exact failure mode `hierarchical-planning-and-multi-supervisor-orchestration.md`'s own ownership boundaries already exist to prevent.

### 13.5 Claims as a lowering consequence of plan publication, not a lazily-populated cache

13.4's reconnaissance phase is bounded by authority, but nothing in it is bounded by *cost* — a newly instantiated supervisor re-derives its evidence from raw repository/runtime state every time, even when some of that evidence was already established, with provenance, during planning itself. Claim-evidence's substrate already exists to hold exactly this kind of thing: a revision-bound, provenance-bearing record of a fact and its consequences, together with a designed mechanism (`nominate_impact` / `open_refresh_episode` / `publish_refresh_judgment`, fully specified in `proposals/evidence-lineage/claim-maintenance-and-reliance-propagation/operation-contract-surface.md`) for the record to be re-examined and re-resolved without being silently overwritten in place.

The ownership distinction has to be stated precisely, because it is easy to blur: the branch plan is the authoritative source of semantic truth; a claim is never that. A claim is, at most, **a materialization of a fact planning already surfaced, bound to the plan revision that justified it** — cache, not owner. Nothing about consulting or refreshing a claim changes who may change the plan; 13.2's and 13.7's authority rules are completely unaffected by anything in this section.

**Corrected framing.** An earlier draft of this section had a newly instantiated supervisor *lazily check* claim-evidence for already-materialized facts, falling back to reconnaissance on a miss. That is weaker than it should be, for a reason this idea's own Part 1 already states as a general principle: *if an authoritative transition already established something, downstream roles should consume its durable consequence rather than reconstruct it.* `hierarchical-planning-and-multi-supervisor-orchestration.md` §16 already names "branch-plan revisions" as required durable state — accepting a branch plan is already an authoritative, revision-producing transition. Materializing planning-fact claims belongs *inside that same transition*, as one of its lowering consequences, not behind a separate opportunistic cache-check a supervisor performs afterward:

```text
branch planner produces branch plan; planning authority accepts it -> revision R
        v
   (one atomic publication consequence, per the point below)
        +-------------------------------+
        |                                |
        v                                v
branch-plan durable state           deterministic planning-fact projection
(§16: orchestration-plan rev,       over R's own declared fields (§6: objective,
branch-plan revisions, ...)          authority/mutation boundary, evidence cutoff,
                                      affected semantic owners, required
                                      consequences, ...)
        |                                |
        v                                v
                      claims bound to R, already present
                                |
                                v
        instantiate Supervisor[A], given accepted branch plan A, revision R
                                |
                                v
        13.4's reconnaissance starts with these claims already given --
        not a lookup that might miss, a durable consequence of acceptance
                                |
                                v
        irreducible remainder needing fresh reconnaissance is only whatever
        R's own declared fields could not already settle
```

**This must be one atomic publication, not two sequenced ones — the exact failure mode `operation-contract-surface.md`'s own first review round caught, one layer up.** That review rejected a design where `publish_refresh_judgment` referenced an already-published revision from a separate, earlier call, because it left a crash window: an authoritative revision with no admitted judgment explaining it. The identical hazard exists here in both directions — a branch plan accepted with no claims yet materialized, or claims materialized bound to a revision `R` that acceptance itself later fails to admit.

**Correction: the requirement is atomic admission-visibility, not necessarily one physical storage transaction (Sol's sharpening).** Stated precisely: *no consumer may observe the accepted branch-plan revision as complete unless every mandatory derived planning-fact claim for that revision is also admitted as part of the same publication state.* Where branch-plan state and claim-evidence share one store, reusing the underlying construction mechanics (`create_claim`/`makeRevision`-shaped operations, one per derived fact) inside one `applyOperation`-style call — exactly Operation 3's own fix — is the direct realization. Where they remain separate services, the same visibility invariant can instead be realized as a validated publication bundle (construct branch plan R plus its claims, validate the whole bundle, publish its head atomically) or a publication manifest (prepare every component, commit one manifest record, and let that manifest becoming authoritative make R and its claims visible together) — the physical mechanism is not decided here, and forcing "one atomic publication" into "one monolithic transaction across services" would be exactly the kind of implementation detail this contract-formation-level document should not presume. What is fixed is the visibility invariant itself, not its physical realization.

**Who is the domain owner.** Planning authority itself — specifically whichever actor's acceptance transition publishes the branch plan — is the natural `open_refresh_episode`/`publish_refresh_judgment` domain-owner class for a `planning-facts-v1` profile, structurally the same separation `operation-contract-surface.md` already enforces between the evidence-producer class (`nominate_impact`) and the domain-owner class. This keeps claim materialization inside the authority that already owns the fact, rather than granting a new, separate observer authority over planning-derived content.

**This is not yet available machinery — it is proposed work.** `app-server/src/services/claim-evidence/contract.mjs` currently defines exactly three domain profiles (`proposal-research-v1`, `revision-bound-review-finding-v1`, `production-path-v1`); none of them materializes planning-derived facts. A `planning-facts-v1` profile does not exist and would need to be designed and reconciled against the planning architecture before any of this diagram is real. What already exists and needs no further verification: `hierarchical-planning-and-multi-supervisor-orchestration.md` §6's own declared branch-plan field list (objective, accepted authority and mutation boundary, evidence cutoff, affected semantic owners, required consequences, and the rest) is a real source to project *from* today — but that establishes only that `planning-facts-v1` can materialize something useful now, not that §6's schema already contains every fact organizational compilation will eventually want (continuity constraints, independence requirements, capability requirements, and projection/evidence-locality boundaries may or may not already be represented there). That broader completeness question is Open Question 24's, unchanged by anything in this section — it is not silently resolved by this profile having enough to start with.

**`planning-facts-v1` must be projection-only — normalizing and binding what the plan already says, never adding a semantic conclusion the plan itself did not establish (Sol's sharpening).** This is the same boundary Part 1 draws everywhere else between deterministic consequence and inference-that-quietly-becomes-authorship, applied to this profile specifically because it is exactly where future convenience pressure will tempt someone to derive a "helpful" fact instead of projecting a declared one:

```text
safe (projection):
    branch-plan field "accepted authority boundary = X"
        -> claim: authority boundary = X, bound to R

    branch-plan field "dependency A -> B"
        -> claim: A depends on B under plan revision R

unsafe (inference -- a second planner in disguise):
    branch plan states evidence E1, E2, E3
        -> claim generator concludes "A and B are implementation-independent"
    -- safe only if the plan itself actually asserted that consequence;
       otherwise the claim has quietly authored new semantic meaning the
       plan never owned
```

A `planning-facts-v1` claim's proposition must trace to one or more explicitly declared branch-plan fields by direct restatement, never by a materializer's own interpretation of what the plan's evidence implies — the same "materialization ≠ authorship" boundary 13.5's cache≠owner framing already establishes for claims relative to plans, now stated as a constraint on the profile's own projection logic, not just on claim-evidence's authority.

Sol's proposal described claims moving between "challenged," "stale," and "invalidated" states. Those are not a third vocabulary alongside the one `operation-contract-surface.md` already fixed after six review rounds; they describe positions within it, and should be named that way rather than informally:

- **"challenged"** is not itself a terminal disposition — it is the state of having an open refresh episode against the claim (a `nominate_impact` has been accepted and `open_refresh_episode` has run) whose disposition has not yet been published. The lifecycle already distinguishes "open" from every terminal disposition; nothing new is needed here.
- **"stale"** maps to the episode-level disposition `superseded`: a later plan revision has already produced a newer claim, and the older one's episode terminates by pointing at that successor rather than by being judged against fresh evidence at all.
- **"invalidated"** maps to the episode-level disposition `changed` (equivalently, `resolved_changed` at the nomination level): a `publish_refresh_judgment` resolved that the materialized fact no longer holds under current evidence.

Mapping Sol's language onto the exact fixed vocabulary matters for the same reason it mattered every time in the operation-contract-surface reviews: a fourth informal vocabulary layered on top of two already-reconciled ones is exactly the kind of drift that produces silent conflation later.

**The currency check needs a third term, and one honest limit stated alongside it.** "No open refresh episode against the claim" is necessary but not sufficient — it only proves nothing *opened* has gone unresolved; it says nothing about a nomination that has been published (`nominate_impact` is itself a caller-asserted, append-only act, per Operation 1) but not yet incorporated into any episode's `reopened_by`. The correct currency condition is:

```text
claim's current revision is bound to the plan's current accepted revision R
+ no open (unresolved) refresh episode against that revision
+ no pending nomination against that revision absent from every episode's reopened_by
```

**What this cannot ever prove, structurally, not as a gap to close:** absence of a nomination that *should* exist but does not yet, because no evidence-producer has observed and asserted it. `nominate_impact` is how real-world impact becomes visible to this substrate at all; a currency check can only assert "no *known* unprocessed impact," never "no unprocessed impact." This is a property of any evidence-producer-driven design, not a defect in this one, and claiming otherwise would overstate what the check supports — the same discipline `operation-contract-surface.md` applied to its own `LINEAGE_RELATIONSHIPS` overclaim. A fourth conjunct — some profile-owned staleness bound (e.g., "evidence older than N revisions is presumptively stale even absent an explicit episode") — is a plausible future extension, symmetric with `refresh_policy`'s own profile-owned extension point, but is not proposed here; no profile currently defines one, and inventing it would be exactly the unsupported connective tissue this session's discipline exists to avoid.

The reverse direction — execution evidence contradicting a plan-derived claim — reuses the same three operations, and terminates the same way 13.4's right-hand branch already does, not by inventing new authority for claim-evidence:

```text
execution evidence contradicts a claim materialized from plan revision R
        v
nominate_impact (evidence-producer authority; caller-asserted nomination_identity)
        v
open_refresh_episode (domain-owner authority, planning-facts-v1 profile)
        v
publish_refresh_judgment: changed  (episode-level disposition; NOT authority
                                     to rewrite the plan -- cache =/= owner)
        v
the discovering actor (e.g. a supervisor per 13.4) nominates a topology
conflict through the existing route -- same route as 13.4's right branch,
same `hierarchical-planning-and-multi-supervisor-orchestration.md` §12 chain
        v
planning authority revises the branch plan -> new accepted revision R'
        v
claims bound to R are not mutated in place; the replan's own acceptance
transition, producing R', projects fresh claims bound to R', per the
atomic-publication requirement above -- the "bound to the revision that
justified it" invariant is what keeps a stale claim from silently outliving
the plan that produced it
```

A `changed` judgment on a claim is a fact about the claim's own currency, exactly as `superseded` and `resolved_changed` already are; none of the three is ever itself the mechanism that rewrites the branch plan. That authority still belongs only to the planning hierarchy, exercised only through the conflict route 13.4 and 13.6 already describe. This section adds a cheaper way to arrive at 13.4's reconnaissance evidence and a reused mechanism for keeping that evidence current — it adds no new authority anywhere in the tree.

**Partial answer to whether organizational compilation itself, not only supervisor formation, should consume claims (Sol's sharpening, then corrected further on Sol's own third review).** Two classes of claim behave differently here, and they should not be treated as one undifferentiated pool:

```text
planning-derived claims (this section):
    accepted authority boundary, dependency topology, mutation boundary,
    declared independence/capability requirements, ...
        -> durable structural inputs

execution-derived claims (13.4's right branch, this section's reverse chain):
    "these two components unexpectedly share state," "migration surface is
    larger than expected," "implementation locality differs from plan
    assumption," ...
        -> adaptive realization evidence, fed to the topology service /
           JIT organizational judgment (Part 5.2)
```

**An execution-derived claim never upgrades its own status to structural, but its accepted semantic consequence can be republished as structural state through the owning authority's own transition.** If evidence that "A and B unexpectedly share state" drives a `changed` refresh judgment, a topology-conflict nomination, and planning authority accepting a revised plan `R'` that now explicitly declares an `A<->B` dependency, the resulting planning-derived claim bound to `R'` is genuinely structural — but only because planning authority's own transition re-established it, never because the originating execution claim accumulated enough evidence to promote itself. Execution evidence may cause structural knowledge to emerge; only the owning authority's transition ever makes it structural.

**Correction to the recursive-layer question — claims must not become a substitute authoritative source merely because the branch plan is inconvenient to reach at depth.** The prior draft of this paragraph left open whether a *deeper* organizational-compilation layer should read a shallower layer's own emitted claims directly. Sol's objection is exactly right and the document's own Part 2.3/5.2 already names the answer this idea should have reached on its own: organizational admission already produces a named authoritative artifact — "the ExecutionEnvelope receives a new revision" (Part 5.2, item 3) — every time a semantic nomination is admitted. So the correct shape is not "claims at depth N feed compilation at depth N+1"; it is the same pattern 13.3 already uses at the root, one recursion level down:

```text
root layer:
    accepted branch plan R  (authoritative)
        -> planning-facts claims(R)      (materialized projection, cache)
        -> organizational compilation reads R directly (13.3)

recursive layer (13.9):
    preceding layer's own organizational admission
        -> ExecutionEnvelope / topology revision  (authoritative, per Part 5.2 item 3)
        -> organizational-facts claims (a symmetric, NOT-YET-PROPOSED profile;
           named here only as a structural possibility, unsettled)
        -> next layer's organizational compilation reads the ExecutionEnvelope/
           topology revision directly, exactly as 13.3 reads the branch plan
```

This preserves cache-never-owner at every depth instead of only at the root: whatever a future `organizational-facts-v1` profile might materialize would be a provenance-bearing index over an already-admitted ExecutionEnvelope/topology revision, never a stand-in for that revision when it is merely inconvenient to fetch directly. It also means Open Question 27(b) has narrowed further than "should recursive compilation consume claims" — the real remaining unknown is which authoritative object a recursively admitted organizational layer actually publishes (ExecutionEnvelope revision, a topology revision, a role-instance contract, or some composition not yet named), a question this document's own Part 12 already leaves open (item 13: "is authority projection part of organizational compilation, a prerequisite to it, a projection over an already-formed envelope, or a separate concern?" — since partially answered by Wave 1's reconciliation against `organizational-execution-envelopes-reconciliation.md`, see item 13's current text; item 19 covers who *realizes* the revision once admitted, a related but narrower sub-question) and that belongs to reconciliation against `organizational-execution-envelopes.md` and `role-compiler-proposal.md`, not to further extension of Part 13.5 in isolation.

### 13.6 Two kinds of lowering, never confused

```text
ADMITTED SEMANTIC STRUCTURE (branch plan)
            v
    organizational compilation
            v
    LOCAL EXECUTION VANTAGE (supervisor, builder, specialist)
            v
  reconnaissance / execution evidence
            v
    +-----------------------+-----------------------+
    |                                                |
plan still valid                              plan inadequacy found
    v                                                v
local topology judgment                    typed topology conflict, routed exactly as
    v                                        hierarchical-planning-and-multi-supervisor-
compile next execution layer  <-----+        orchestration.md §12 already specifies:
                                     |        Branch planner/Supervisor -> Orchestrator
                                     |        -> Preplanner -> Operator/owning authority
                                     |        -> Orchestrator -> new accepted plan
                                     +------------------------------------------------+
```

This is the corrected shape of recursion: organizational compilation lowers an accepted layer into execution; execution either confirms the layer (compile the next layer) or falsifies it (route to the existing, already-designed replanning mechanism, which produces a *new* accepted layer for auto-org to compile again). Nothing here invents a second replanning path — it reuses the one `hierarchical-planning-and-multi-supervisor-orchestration.md` already has.

### 13.7 The authority rule at every level

```text
planner (preplanner / branch planner):
    may change semantic work topology

organizational compiler / topology service
    (the mechanism auto-org's policy modes govern -- see 13.17;
     "auto-org" names a mode over this authority, not the authority itself):
    may derive lawful execution organization from an accepted semantic topology

supervisor (or any realized vantage):
    may judge local execution/vantage fitness within the accepted semantic topology
    may nominate a topology conflict when evidence falsifies that topology
    may NOT silently redefine the topology itself
```

This preserves `hierarchical-planning-and-multi-supervisor-orchestration.md`'s existing architecture rather than superseding it, and prevents the organizational-compiler layer from turning supervisors into unaccountable local managers who accumulate planning authority just because they are closest to the evidence — precisely the failure mode that architecture's own ownership boundaries (§3, §7, §12) were built to avoid.

### 13.8 The same retained context performs both reconnaissance and execution

The supervisor that performs organizational reconnaissance should be the same retained logical role and context that subsequently executes or supervises the resulting branch — not a disposable scout that investigates, writes a summary, and is discarded before a cold supervisor reconstructs what it learned:

```text
Supervisor[A]
  formation phase:  investigate branch structure, gather evidence,
                     understand local risks/dependencies, judge realization fitness
                          | (same retained vantage)
                          v
  execution phase:  supervise the resulting child organization
```

The evidence gathered must still be externalized to run-owned durable state — the context itself is never canonical — but retaining the supervisor's own accumulated understanding across the formation-to-execution boundary has independent value. Both together, not one instead of the other: durable evidence *and* retained context continuity.

### 13.9 Organizational compilation recurses within accepted structure; planning authority and agent spawning do not

A supervisor's own reconnaissance may reveal further asymmetric realization structure within its own accepted obligations:

```text
Supervisor[A]                          Supervisor[B]
    +-- Builder[A-core]  (owns A1+A2,       +-- Builder[B]
    |    which share heavy state)
    +-- Specialist[A3]   (independently
         coherent compatibility work)
```

Nothing requires symmetry between branches, and any resulting child role may itself later encounter enough structural pressure to warrant its own further organizational compilation — bounded, at every layer, by the same rule from 13.6: confirm and compile deeper, or fall back to the existing topology-conflict route if the layer's own accepted structure is what's actually wrong:

```text
plan -> orchestrator -> branch planners produce accepted branch plans
-> compile first execution layer -> supervisors investigate
-> confirmed: compile second execution layer; falsified: nominate conflict, replan, recompile
-> child roles investigate -> compile deeper layers only where warranted -> leaves execute
```

This is **recursive organizational compilation**, triggered by locally-discovered evidence at each layer and always bounded by whatever semantic structure is currently accepted at that layer — not recursive agent spawning, and not recursive planning authority, either of which would collapse the distinction Part 13.2 depends on.

### 13.10 How much freedom does organizational compilation actually have?

13.9's example (`Builder[A1+A2]`, `Builder[A3]`) groups and separates accepted obligations along their own existing boundaries — clean. But nothing said so far rules out a realization that cuts *across* those boundaries instead of merely grouping or separating them:

```text
still grouping/separating (clean):
    Builder X owns A1 + A2
    Builder Y owns A3

cutting across (a real, uncatalogued question):
    Builder X owns part of A1 and part of A2
    Builder Y owns the remaining parts of A1 and A2
```

The second shape may still be a legitimate realization choice, but it is no longer obviously just "how many vantages realize this work" — it starts to look like it is repartitioning the semantic obligations themselves, which is planning authority's territory, not organizational-realization authority's. This document does not resolve where the line sits. A safe initial boundary, stated but not adopted as final:

```text
organizational-realization authority may:
    group accepted obligations under one vantage
    separate accepted obligations into distinct vantages
    assign capabilities and contexts to those vantages

organizational-realization authority may NOT:
    alter what an accepted obligation means
    alter accepted dependency relations between obligations
    create new semantic obligations
    eliminate accepted obligations
```

Under that boundary, cross-cutting realizations (splitting a single obligation's own internal work across more than one vantage) would fall outside organizational-realization authority entirely, and would need its own explicit treatment — deferred to Open Question 26, not decided here.

### 13.11 Authority attenuates down the tree; depth never creates more of it

Part 2.4's invariant (`child authority ⊆ delegable(parent authority)`) applies recursively without modification:

```text
Root authority
      v
Workstream A ceiling
      v
Supervisor[A]
      v (delegable subset)
Builder[A-core]
```

New organizational depth partitions existing authority into increasingly local vantages; it never manufactures new authority. Nondelegable surfaces stay wherever they are actually owned, at whatever depth that happens to be — depth is not itself a claim to authority.

### 13.12 What this implies a hierarchical plan needs to expose

Not `spawn 3 supervisors, spawn 7 builders`, but organizationally-relevant semantics per workstream, from which the organization is *compiled* rather than *authored*:

```text
workstream, subject, dependencies, consequence, shared invariants,
authority ceiling, required capabilities, known independence requirements,
integration boundary, known continuity constraints
```

The plan describes the semantic structure of the work; the organization is a derived consequence of that structure, not a separate thing the plan author must also specify by hand. Whether `hierarchical-planning-and-multi-supervisor-orchestration.md`'s own branch-plan output (§6) already carries enough of this list, or would need to be extended, is not decided here — see Open Question 24.

### 13.13 This composes with event-scoped JIT organizational reasoning exactly as already designed

Each supervisor runs normally, without permanently carrying auto-org machinery (per Part 2.7). During its formation phase, if the topology service's observer-derived evidence makes an organizational-realization question material — never a question about whether the accepted plan itself is correct — a JIT decision surface is temporarily injected:

```text
Supervisor[A]'s normal context
    + JIT organization skill
    + branch-local topology metrics
    + plan constraints
    + available role/capability profiles
```

resolving a narrow question — does this branch require one retained implementation vantage or several, and why — after which the result persists, the temporary skill is removed, and the supervisor proceeds to steady-state supervision. Nothing about this requires a role to carry organizational-decision doctrine permanently, and nothing about it authorizes the same JIT surface to also judge whether the plan itself should change — that question, per 13.6, has its own separate, already-existing route.

### 13.14 Role formation as a named phase, precisely scoped

```text
ROLE FORMATION
provisional logical role instantiated
        v
bounded reconnaissance
        v
evidence accumulation
        v
organizational-fit analysis (never a re-judgment of the accepted plan itself)
        v
topology decision(s), or a nominated topology conflict if the plan is falsified
        v
ExecutionEnvelope revision
        v
ROLE EXECUTION
```

"Provisional" describes only the role's *downstream execution organization* — not its own authority, which is real and non-provisional from the moment of instantiation, and not the semantic plan it was instantiated to realize, which the role has no authority to revise itself. Supervisor[A] already owns supervising A the instant it exists; what remains undecided is only how A's own accepted obligations should be realized.

### 13.15 The natural stopping condition

No role should recurse automatically merely "to see what's there" — that is organizational recursion for its own sake, and this document names it explicitly as something to avoid. Recursion should trigger only when evidence establishes genuine separation pressure, reusing the exact generative criterion already stated in Part 2.5: create another vantage only when a stable semantic boundary lets the resulting contexts know materially less while preserving or improving the required consequence. The resulting fixed point:

> No unresolved obligation benefits from another legitimate vantage.

At that point, recursive organizational compilation stops — not because of a depth limit, but because no further split of the *execution realization* (never the accepted semantic obligations themselves) would be justified by the criterion itself.

### 13.16 The full picture

```text
                       ROOT PLAN
                           v
                       ORCHESTRATOR
                           v
             instantiate branch-planning vantages
             +-------------+-------------+
             v                           v
      BRANCH PLANNER[A]           BRANCH PLANNER[B]
             v                           v
      accepted branch plan A     accepted branch plan B
             v                           v
      organizational compilation (topology-realization authority)
             v                           v
       SUPERVISOR[A]               SUPERVISOR[B]
             v                           v
    reconnaissance -> confirmed    reconnaissance -> confirmed
     or falsified (-> §12          or falsified (-> §12
     topology conflict route)      topology conflict route)
             v                           v
      child organization           child organization
             +-------------+-------------+
                           v
                       execution
                           v
                   integration layer
```

resting throughout on the shared substrate already established in Parts 4, 5, and 7: Context Observer, authority state, run evidence, plan state, ExecutionEnvelope revisions, transition fencing, context lifecycle, capability resolution. The orchestrator and branch planners stay within `hierarchical-planning-and-multi-supervisor-orchestration.md`'s own already-designed roles; auto-org supplies only the compilation step between an accepted plan layer and the execution organization that realizes it.

### 13.17 `auto-org` is a policy mode, not a second architecture

Extending Part 6's organizational-dynamics-as-policy framing directly, rather than introducing new machinery:

```text
organization_mode: fixed              authored execution topology only, no compilation
organization_mode: plan_compiled      compile only the first execution layer from the accepted plan
organization_mode: recursive_bounded  admitted execution vantages may further compile their own realization
organization_mode: auto               recursive organizational compilation wherever policy permits
```

Same underlying machinery in every mode; no second architecture per mode; a fixed organization is simply the case where the transition set is constrained to `∅`, exactly as Part 6 already establishes for the non-recursive case. None of these modes touch semantic planning authority — they only vary how much of the execution organization beneath an accepted plan gets compiled versus authored.

### 13.18 The operational payoff, restated precisely

The genuine novelty here is not a new orchestration shape — `hierarchical-planning-and-multi-supervisor-orchestration.md` likely already has the shape. The novelty is a **missing compilation layer between hierarchical planning and hierarchical execution**:

```text
BEFORE                                  AFTER

planner authors semantic topology       planner authors/adopts semantic topology
+                                                v
architecture prescribes role topology   authority/vantage requirements
                                                 v
                                         organizational compiler
                                                 v
                                         role instances + contracts + contexts + capabilities
```

applied recursively at each accepted layer, alternating with the existing topology-conflict route whenever local evidence falsifies rather than merely refines that layer's own accepted structure. Sharper than "dynamic orchestration": **`auto-org` is recursive organizational realization of an admitted hierarchical plan** — the hierarchical planner no longer needs to predict the entire execution organization from above, but it also never loses its own authority over what the work actually is. If this holds up under full reconciliation (Open Question 22), the resulting chain is:

```text
hierarchical planning -> semantic work topology -> authority projection -> role/vantage composition
-> ExecutionEnvelope -> runtime realization -> supervisor-local evidence -> bounded organizational
compilation -> replan only when semantic topology itself is invalidated (existing §12 route)
```

---

## Working Hypothesis

> Work Engine roles are not fundamentally job titles with attached permissions. They are logical vantage points from which bounded semantic authority can be exercised — and those vantage points are themselves compiled compositions of more primitive elements, situated within a context/vantage topology that Work Engine can adaptively manage using the same discipline it already applies to context replacement: deterministic observation, bounded semantic inference only where a real judgment remains, and governed, revision-fenced admission of the result.

If this hypothesis survives reconciliation, organizational compilation may eventually become:

```text
work to be judged
        v
semantic decision surfaces
        v
required legitimate vantage points
        v
role decomposition / composition
        v
bounded authority projection
        v
workflow and ExecutionEnvelope
        v
runtime realizations
```

compiled around the semantic conditions required for trustworthy judgment rather than primarily around agent identities. A possible long-term formulation: **given a body of work and its semantic, evidentiary, authority, continuity, and independence requirements, compile the smallest truthful organization capable of judging it correctly** — while ensuring that organizational plasticity itself remains a governed, auditable policy surface rather than an emergent property of model behavior. Whether Work Engine should actually reach that destination remains open. The immediate purpose of this idea is narrower: determine whether the repeated role-authority and context-management patterns already present in Work Engine are instances of one common, domain-transferable model.
