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

deriving what the role must observe, what state it must retain, what authority it receives, what effects it may perform, what capabilities it requires, what it must produce, what it may not do, how long it must exist, and which role relationships must hold. The important boundary: **do not derive semantic meaning; derive the contract consequences of semantic meaning that has already been established.** This produces three layers:

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

3. **ExecutionEnvelope/workflow authority: admit the organizational consequence.** Even if the active role concludes "split this," that is a semantic nomination, not unilateral authority to rewrite the organization. Something must verify that the parent may delegate this authority, the child grant is correctly attenuated, the workflow permits organizational branching, independence constraints hold, no competing authority owner exists, and required capabilities can be realized — then the ExecutionEnvelope receives a new revision. Role judges semantic usefulness; role does not mint organization.

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

- **`hierarchical-planning-and-multi-supervisor-orchestration.md` — the single most consequential relationship in this document, named here precisely because it has not been checked.** That idea already has a real, detailed `Orchestrator → Branch planner → Branch plan → Supervisor → Builder`, with integration treated as its own workstream — the same roles, the same recursive planner:orchestrator :: planner:supervisor relationship, and much of the same vocabulary (workstream, branch, supervisor, orchestrator, integration) that Part 13's `auto-org` execution mode uses. Part 13 was drafted without checking whether it duplicates, extends, or conflicts with that document's own already-designed hierarchy. It may turn out that `auto-org` is not a new organizational shape at all, but a specific *compilation mechanism* — authority projection and role-contract compilation — underneath a hierarchy that document already treats as authored structure. This is not decided here and should be the first thing checked before any of Part 13 is treated as more than a hypothesis.
- **`proposal-decision-gated-implementation-compilation.md`** — Part 13's "branch" and "workstream" vocabulary sits directly downstream of the `routing.vs.admission` and `decision-gated.vs.hierarchical-orchestration` seams this same session already closed (recorded in both that document and `hierarchical-planning-and-multi-supervisor-orchestration.md` directly) — the sealed-decision-set / implementation-contract layer those rulings placed beneath the branch plan is exactly what Part 13's leaf-level "execution" step would compile down into. Not reconciled here, but the two already-closed rulings should be read before Part 13 is extended further.
- **`organizational-execution-envelopes.md`** — Parts 2 and 5 of this idea (role-contract compilation, organizational admission) bear directly on how that envelope gets compiled and revised; this idea does not decide whether authority projection sits inside the envelope compiler, feeds it as a separate stage, or is a distinct architectural owner.
- **`role-compiler-proposal.md`** — role contract compilation (Part 2.2) overlaps directly with whatever that proposal already owns regarding composition and inheritance; not reconciled.
- **The semantic-context-lifecycle-manager design** — Parts 3, 4, and 7 directly extend and constrain that system; Part 4's findings are current-implementation facts, not proposed changes, and any future work here must be checked against that design document's own current text before this idea is treated as settled.
- **claim-evidence's `expected_state`/heads mechanism and `review-episode`'s writer-generation mechanism** — Part 7's reasoning-environment transition lease is a direct generalization of a fencing/revision-binding pattern already proven in both; this is corroborating precedent, not a coincidence, and any future design should reuse rather than reinvent it.
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
13. Is authority projection part of organizational compilation, a prerequisite to it, a projection over an already-formed envelope, or a separate concern?
14. Are workflows authored structures constrained by authority projection, or can some workflow topology itself eventually be compiled from semantic dependencies?
15. What is the concrete schema for the context observer, and which of its fields are genuinely mechanically observable today versus requiring new externalized state?
16. Where should the decision-episode fence and topology-transition fence mechanism live — inside context-lifecycle itself, a new shared service, or the ExecutionEnvelope authority?
17. Beyond illustrative examples, how is arbitration decided when lifecycle-critical pressure and an in-progress topology episode conflict?
18. Should the observed-context substrate be promoted to a formally shared cross-service dependency now, or allowed to grow organically from its current lifecycle-scoped form?
19. Who realizes an admitted organizational/topology revision concretely — a projection/runtime owner distinct from context-lifecycle, or context-lifecycle itself acting in a narrower "realize this admitted projection" capacity kept separate from its own temporal-applicability judgment? Deliberately left open in Part 5.2 rather than assumed either way.
20. What is the smallest real current workflow from which a deterministic authority projection could be reconstructed without changing behavior?
21. Can the proposed abstraction be demonstrated across one non-software domain without adding domain-specific rules to the projection mechanism?
22. Does `hierarchical-planning-and-multi-supervisor-orchestration.md`'s existing `Orchestrator/Branch planner/Branch plan/Supervisor/Builder` hierarchy already cover what Part 13 proposes, making `auto-org` a compilation mechanism underneath an already-authored shape rather than a new organizational structure? This is the single highest-priority question in this document and should be answered before any of Part 13 is extended further.
23. Does that document's own topology allow a branch to recursively decompose further once accepted, or is its topology assumed fixed after the orchestration plan is admitted? Part 13's recursion depends on the answer.
24. What exactly must a hierarchical plan expose (per Part 13's list — subject, dependencies, consequence, shared invariants, authority ceiling, required capabilities, independence requirements, integration boundary, continuity constraints) for the first organizational layer to be mechanically compiled rather than authored, and does the current planning/Plan-IR architecture already carry any of this?

---

## Part 13: The `auto-org` Execution Mode — A Worked Synthesis

This section works through what Parts 1, 2, 5, and 6 imply when applied recursively to an admitted hierarchical plan. It is the most speculative and least-checked part of this document — see Part 11 and Open Question 22 above before treating any of it as more than a hypothesis.

### 13.1 What `auto-org` does not mean

It does not mean giving one orchestrator a goal and letting it invent arbitrary agents. It means: **given an admitted hierarchical plan plus an organizational-dynamics policy, recursively lower that plan into the smallest lawful execution organization, one layer at a time** — the same chain already established (semantic obligations → required vantages → role composition → bounded authority projection → ExecutionEnvelope → runtime realizations), applied more than once, at successively more local scope.

### 13.2 The first organizational layer compiles mechanically from plan structure

A root orchestrator does not need to understand each workstream deeply. If the plan already states each workstream's subject, dependencies, expected consequence, and constraints, the orchestrator can mechanically derive enough to instantiate distinct branch-supervisor contracts:

```text
Objective
├── Workstream A (dependencies, expected consequence, constraints)
├── Workstream B (dependencies, expected consequence, constraints)
└── Integration

Orchestrator
    +-- Supervisor[A]   subject=A, observes=A-plan+shared contracts+dependency edges,
    |                   authority ceiling=A, capabilities=required-by-A, continuity=A's execution lifetime
    +-- Supervisor[B]   subject=B, observes=B-plan+shared contracts+dependency edges,
    |                   authority ceiling=B, capabilities=required-by-B, continuity=B's execution lifetime
    +-- Integration owner
```

The contracts differ because the plan surfaces differ — exactly the role-contract compilation Part 2.2 already describes, applied to the plan's own declared structure rather than to a role someone hand-authored.

### 13.3 Role formation as a bounded reconnaissance phase, not immediate execution

A newly instantiated supervisor need not begin executing immediately. It can be given a bounded formation phase first:

```text
instantiate Supervisor[A]
        v
branch-local reconnaissance
        v
gather evidence about A's actual world
        v
maintain that evidence durably for the run
        v
determine A's actual semantic shape
        v
decide whether/how A should decompose
        v
admit child topology (if any)
        v
same supervisor continues executing A
```

This is a materially better sequence than having a planner speculate about the complete execution organization before anyone has examined the actual work.

### 13.4 The same retained context performs both reconnaissance and execution

The supervisor that performs organizational reconnaissance should be the same retained logical role and context that subsequently executes or supervises the resulting branch — not a disposable scout that investigates, writes a summary, and is discarded before a cold supervisor reconstructs what it learned:

```text
Supervisor[A]
  formation phase:  investigate branch structure, gather evidence,
                     understand local risks/dependencies, decide child organization
                          | (same retained vantage)
                          v
  execution phase:  supervise the resulting child organization
```

The evidence gathered must still be externalized to run-owned durable state — the context itself is never canonical — but retaining the supervisor's own accumulated understanding across the formation-to-execution boundary has independent value. Both together, not one instead of the other: durable evidence *and* retained context continuity.

### 13.5 Organization recurses; agent spawning does not

A supervisor's own reconnaissance may reveal further asymmetric structure:

```text
Supervisor[A]                          Supervisor[B]
    +-- Builder[A-core]  (owns A1+A2,       +-- Builder[B]
    |    which share heavy state)
    +-- Specialist[A3]   (independently
         coherent compatibility work)
```

Nothing requires symmetry between branches, and any resulting child role may itself later encounter enough structural pressure to warrant its own further decomposition:

```text
plan -> orchestrator -> compile first organizational layer -> supervisors investigate
-> compile second organizational layer -> child roles investigate
-> compile deeper layers only where warranted -> leaves execute
```

This is **recursive organizational compilation**, triggered by locally-discovered evidence at each layer — not recursive agent spawning for its own sake. The distinction matters: nothing here authorizes a role to create children merely because it can.

### 13.6 Authority attenuates down the tree; depth never creates more of it

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

### 13.7 What this implies a hierarchical plan needs to expose

Not `spawn 3 supervisors, spawn 7 builders`, but organizationally-relevant semantics per workstream, from which the organization is *compiled* rather than *authored*:

```text
workstream, subject, dependencies, consequence, shared invariants,
authority ceiling, required capabilities, known independence requirements,
integration boundary, known continuity constraints
```

The plan describes the semantic structure of the work; the organization is a derived consequence of that structure, not a separate thing the plan author must also specify by hand.

### 13.8 This composes with event-scoped JIT organizational reasoning exactly as already designed

Each supervisor runs normally, without permanently carrying auto-org machinery (per Part 2.7). During its formation phase, if the topology service's observer-derived evidence makes an organizational question material, a JIT decision surface is temporarily injected:

```text
Supervisor[A]'s normal context
    + JIT organization skill
    + branch-local topology metrics
    + plan constraints
    + available role/capability profiles
```

resolving a narrow question — does this branch require one retained implementation vantage or several, and why — after which the result persists, the temporary skill is removed, and the supervisor proceeds to steady-state supervision. Nothing about this requires a role to carry organizational-decision doctrine permanently.

### 13.9 Role formation as a named phase, precisely scoped

```text
ROLE FORMATION
provisional logical role instantiated
        v
bounded reconnaissance
        v
evidence accumulation
        v
organizational-fit analysis
        v
topology decision(s)
        v
ExecutionEnvelope revision
        v
ROLE EXECUTION
```

"Provisional" describes only the role's *downstream organization* — not its own authority, which is real and non-provisional from the moment of instantiation. Supervisor[A] already owns supervising A the instant it exists; what remains undecided is only whether, and how, A's own execution should further decompose.

### 13.10 The natural stopping condition

No role should recurse automatically merely "to see what's there" — that is organizational recursion for its own sake, and this document names it explicitly as something to avoid. Recursion should trigger only when evidence establishes genuine separation pressure, reusing the exact generative criterion already stated in Part 2.5: create another vantage only when a stable semantic boundary lets the resulting contexts know materially less while preserving or improving the required consequence. The resulting fixed point:

> No unresolved obligation benefits from another legitimate vantage.

At that point, decomposition stops — not because of a depth limit, but because no further split would be justified by the criterion itself.

### 13.11 The full picture

```text
                 HIERARCHICAL PLAN
                        v
                 ROOT ORCHESTRATOR
                        v
             compile immediate workstreams
             +----------+----------+
             v                     v
       Supervisor[A]         Supervisor[B]
             v                     v
      organizational        organizational
      reconnaissance        reconnaissance
             v                     v
      local topology        local topology
      judgment              judgment
             v                     v
      child organization    child organization
             +----------+----------+
                        v
                    execution
                        v
                integration layer
```

resting throughout on the shared substrate already established in Parts 4, 5, and 7: Context Observer, authority state, run evidence, plan state, ExecutionEnvelope revisions, transition fencing, context lifecycle, capability resolution. The orchestrator stays structural; supervisors acquire local semantic understanding; the organization becomes more specific exactly where the evidence is most local.

### 13.12 `auto-org` is a policy mode, not a second architecture

Extending Part 6's organizational-dynamics-as-policy framing directly, rather than introducing new machinery:

```text
organization_mode: fixed              authored topology only, no compilation
organization_mode: plan_compiled      compile only the first layer from the plan
organization_mode: recursive_bounded  admitted roles may further decompose their own work
organization_mode: auto               recursive organizational formation wherever policy permits
```

Same underlying machinery in every mode; no second architecture per mode; a fixed organization is simply the case where the transition set is constrained to `∅`, exactly as Part 6 already establishes for the non-recursive case.

### 13.13 The operational payoff

If this holds up under reconciliation (starting with Open Question 22), the payoff is that a hierarchical planner no longer needs to predict the entire execution organization from above. It describes the semantic structure of the work; the orchestrator compiles the first lawful organizational layer; and each resulting vantage, after actually examining its own local world, may recursively compile the next layer under an attenuated authority ceiling. The result is an organization that becomes more specific as evidence becomes more local, while remaining reconstructable, authority-bounded, and derived from one admitted plan throughout.

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
