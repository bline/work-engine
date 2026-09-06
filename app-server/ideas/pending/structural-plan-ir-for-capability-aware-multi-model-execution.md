# Structural Plan IR for Capability-Aware Multi-Model Execution

## Status

Design hypothesis for repository validation.

## Relationship to Decision-Gated Implementation Compilation

This is a distinct post-migration representation and execution hypothesis related
to [Decision-Gated Implementation Compilation](proposal-decision-gated-implementation-compilation.md).
That candidate proposal owns the broader implementation-contract workflow,
decision closure, authority boundaries, and plan-conformance gate. This document
explores a possible structural representation for such a contract, loss-aware
executor projections, capability compensation, and the experiment needed to
evaluate them.

If both directions advance, the Plan IR is a candidate representation of the
compiled implementation contract rather than a parallel semantic or authority
owner. This relationship does not accept either direction, authorize
implementation, or add work to the active skills-migration campaign. Final role
and artifact ownership should be reconciled against the post-migration App Server
architecture before implementation.

The companion
[Structural Plan IR Prior Art](structural-plan-ir-prior-art.md) note records
verified representation precedents, limits of the analogies, and the research
questions that should be resolved before schema formation.

This document proposes replacing increasingly verbose model-specific planning with a canonical **structural plan representation** that captures the semantics of implementation work once and allows downstream components to render that plan at different levels of explicitness for different builder models.

The immediate motivation is model heterogeneity: a strong reasoning model such as GPT-5.6 Sol may be able to execute successfully from a relatively compact implementation plan, while a less capable model may require substantially more explicit decomposition and stronger execution constraints.

The proposed solution is not to maintain separate planning semantics for each model. Instead:

> **The planner produces one canonical structural plan. Model-specific adapters compile that plan into representations appropriate to the capability profile of the receiving builder.**

If successful, increased planning precision may produce compression as a side effect rather than requiring a trade between brevity and completeness.

---

# 1. Problem

Work Engine is moving toward heterogeneous multi-model execution.

Different models may be useful for different roles:

- strong models for architectural reasoning and planning;
- less expensive or reserve-capacity models for bounded implementation;
- specialized models for review or other constrained work.

However, models differ significantly in the amount of implicit structure they can reliably recover from a plan.

A capable builder may infer:

- dependency order;
- architectural boundaries;
- which statements are hard requirements;
- which implementation choices remain open;
- which decisions require escalation;
- how acceptance criteria relate to changes;
- which existing abstractions must remain untouched.

A weaker builder may require these relationships to be stated explicitly.

The naive response is to create different planning modes:

```text
Sol plan     → compact
Luna plan    → detailed
Model X plan → even more detailed
```

That creates several problems.

First, plan semantics become entangled with model identity.

Second, the planner may be forced to spend increasingly large amounts of inference generating verbose procedural instructions.

Third, independently authored model-specific plans may drift semantically.

Fourth, supervisors and reviewers may need to interpret different representations of what is supposedly the same implementation contract.

Finally, detailed prose does not necessarily improve precision. It may simply add tokens while preserving ambiguity.

The deeper problem is therefore not:

> How do we write longer plans for weaker models?

It is:

> How do we represent implementation intent so that important reasoning does not have to be reconstructed repeatedly by every downstream model?

---

# 2. Core Proposal

Introduce a canonical **Plan IR**: a structured intermediate representation of an implementation plan.

The Plan IR is the authoritative semantic artifact produced by planning.

It should encode the important relationships within the plan directly rather than relying on narrative prose to imply them.

Conceptually:

```text
operator intent
      │
      ▼
architectural / implementation planning
      │
      ▼
canonical Plan IR
      │
      ├────► compact builder representation
      │          e.g. Sol
      │
      ├────► elaborated builder representation
      │          e.g. Luna or other lower-capability builder
      │
      ├────► supervisor verification view
      │
      ├────► reviewer view
      │
      └────► progress / reconciliation machinery
```

The Plan IR describes **what the implementation means**.

Model-specific compilation describes **how much of that meaning must be made explicit to a particular executor**.

This preserves a critical architectural distinction:

> Model capability may change representation and execution policy without changing the underlying implementation contract.

---

# 3. Structural Rather Than Narrative Planning

A conventional plan often expresses relationships through prose:

```text
Add validation at the mutation boundary so that stale fencing
tokens cannot mutate the workspace. Be careful not to move lease
ownership into the mutation layer. The existing coordinator remains
responsible for ownership. Add tests covering stale and current
tokens...
```

The same semantics could instead be represented structurally:

```text
objective
  O1:
    prevent stale workspace actors from mutating state

invariants
  I1:
    stale fencing tokens never authorize mutation

  I2:
    token validation occurs before filesystem mutation

  I3:
    coordinator ownership remains unchanged

changes
  C1:
    target: mutation boundary
    operation: add token validation
    satisfies: [I1, I2]
    depends_on: [C2]

  C2:
    target: lease/token service
    operation: expose required validation result
    preserves: [I3]

acceptance
  A1:
    stale token is rejected
    traces_to: [I1, C1]

  A2:
    current token is accepted
    traces_to: [C1]

  A3:
    coordinator ownership behavior remains unchanged
    traces_to: [I3, C2]

judgment
  J1:
    question: choose among equivalent local implementation forms
    authority: builder
    constrained_by: [I1, I2, I3]

  J2:
    question: introduce a new ownership abstraction
    authority: supervisor
    action: escalate

verification
  V1:
    type: unit
    demonstrates: [A1, A2]

  V2:
    type: regression
    demonstrates: [A3]
```

The exact schema should be derived from existing repository concepts rather than assumed from this example.

The important property is that relationships which otherwise have to be inferred from prose become explicit data.

---

# 4. Semantic Normalization as Compression

The Plan IR may provide meaningful token compression without conventional summarization.

Narrative plans repeatedly express relationships such as:

- “because this requirement...”
- “while preserving...”
- “after completing...”
- “do not change...”
- “as described above...”
- “this should satisfy...”
- “if this requires architectural changes, escalate...”

A structural representation can state each semantic fact once and reference it elsewhere.

For example:

```text
I3 = coordinator ownership remains unchanged
```

Multiple changes, acceptance criteria, and reviewer checks can refer to `I3` without restating it.

Conceptually:

```text
many repeated prose statements
            │
            ▼
      one semantic node
            │
            ▼
       multiple edges
```

This resembles normalization in data systems.

The representation can become simultaneously:

- shorter;
- less ambiguous;
- more machine-verifiable;
- easier to elaborate;
- easier to inspect for completeness.

Compression would therefore not be achieved by removing semantic information.

It would be achieved by **removing redundant linguistic encoding of the same semantic information**.

---

# 5. Typed Semantic Categories

A major source of implementation error is that prose does not reliably distinguish the status of information.

A builder may not know whether a statement is:

- contextual information;
- an established fact;
- an architectural decision;
- a hard invariant;
- a preference;
- an available choice;
- an unresolved question;
- a forbidden action;
- an escalation condition.

The Plan IR should make these distinctions explicit.

Likely categories include some form of:

```text
objective
fact
decision
invariant
constraint
change
dependency
acceptance criterion
open judgment
forbidden change
escalation condition
verification requirement
```

The repository may already contain concepts that should replace or refine these names.

The important invariant is:

> Semantically different kinds of information should not rely on prose interpretation alone to distinguish them.

---

# 6. Explicit Judgment Boundaries

Capability-aware execution requires more than task decomposition.

It requires specifying **where downstream judgment is permitted**.

A builder should be able to distinguish:

```text
DECIDED
The architecture has already chosen this.

OPEN
You may select among implementations satisfying these constraints.

ESCALATE
This decision exceeds builder authority.

FORBIDDEN
Do not solve the problem by changing this boundary.
```

This is particularly important when using less capable models.

The objective should not be to eliminate all builder judgment.

Doing so could cause the planner to perform the implementation indirectly.

Instead, the plan should expose the **boundary of legitimate implementation freedom**.

This gives the builder room to build while preventing local implementation uncertainty from silently becoming architectural authority.

---

# 7. Dependency Representation

Plans frequently contain an implicit graph.

For example:

```text
C2 must exist before C1 can be completed.
C3 and C4 are independent.
C5 requires both C1 and C3.
```

Narrative plans force each builder to reconstruct this graph.

A structural plan should encode it directly:

```text
C2 → C1

C3 || C4

[C1, C3] → C5
```

This can support:

- execution ordering;
- parallelization;
- slice construction;
- model assignment;
- progress tracking;
- failure recovery;
- supervisor reconciliation.

The plan may therefore become useful not merely as builder context, but as an orchestration artifact.

---

# 8. Traceability

Every meaningful plan element should have a stable address.

For example:

```text
O1
I2
C4
J1
A3
V2
```

That enables precise communication throughout the workflow.

Instead of a supervisor producing a long natural-language correction:

```text
The implementation appears to violate the earlier requirement
that validation occur before filesystem mutation...
```

the workflow can communicate:

```text
C4 violates I2.
A3 has not been demonstrated.
J2 exceeded builder authority.
```

The receiving model can then retrieve or receive the referenced nodes.

Stable semantic addressing may substantially reduce the cost of:

- review;
- repair;
- reconciliation;
- handoff;
- context recovery.

It also creates stronger provenance between planning decisions and implementation evidence.

---

# 9. Model-Specific Compilation

The Plan IR should remain model-independent.

Model-specific builder variants should determine how that structure is rendered.

For a high-capability builder, a compiler might produce:

```text
Objective: O1

Implement C1-C4.

Preserve:
I1-I4

Builder judgment:
J1

Escalate:
J2

Acceptance:
A1-A4
```

A more constrained builder representation might expand the same graph:

```text
Step 1 — C2

Purpose:
Required by C1.

Constraints:
I1, I3.

Do not:
F2.

Verification:
A2.


Step 2 — C1

Precondition:
C2 complete.

Required effects:
I1, I2.

Acceptance evidence:
A1, A2.

Escalate if:
J2 is encountered.
```

The additional detail is not newly invented planning.

It is a deterministic or low-cost elaboration of relationships already encoded in the Plan IR.

That distinction is central to the proposal.

---

# 10. Planning Resolution Versus Model Identity

The system may eventually discover that representation should not be binary:

```text
Sol plan
Luna plan
```

Instead, the Plan IR could support multiple **resolution levels**.

For example:

```text
P0 — architectural summary
P1 — implementation boundaries
P2 — explicit execution structure
P3 — tightly constrained execution
```

Builder capability profiles could then declare which representation is normally required.

For example:

```text
Sol
  default: P1
  ambiguous/high-risk work: P2

Luna
  default: P2
  admitted highly constrained work: P3

Other model
  empirically determined
```

The exact levels should not be established before experimentation.

The architectural point is that:

> Planning resolution becomes an execution parameter rather than a different source of truth.

---

# 11. Preventing Planner-to-Builder Work Leakage

There is an important failure mode.

Increasing plan detail can eventually cause the planner to perform the implementation itself.

Bad outcome:

```text
planner:
  reasons through almost every implementation operation
  produces large procedural script

builder:
  mechanically transcribes planner output
```

In that regime, apparent savings from using a cheaper builder are illusory.

The expensive planner is doing the builder's work indirectly.

The structural format should therefore preserve a boundary:

### Planner responsibility

- understand the problem;
- resolve architecture;
- establish invariants;
- identify dependencies;
- define authority boundaries;
- define acceptance;
- expose remaining legitimate judgment.

### Builder responsibility

- inspect the implementation environment;
- choose permitted local implementations;
- write code;
- adapt implementation to discovered local conditions;
- execute tests;
- gather evidence;
- escalate when required.

The goal is **structural determinacy**, not procedural transcription.

---

# 12. Economics Hypothesis

The proposal creates an empirical question.

Adding planning structure consumes upstream reasoning.

However, upstream reasoning may have leverage because it reduces repeated downstream inference.

For example:

```text
additional planner reasoning
        ↓
less builder search
less builder recovery
fewer supervisor interventions
less repeated context
fewer implementation failures
```

A useful conceptual measure may be an elaboration leverage ratio:

```text
downstream compute avoided
──────────────────────────
additional upstream compute
```

This should never override quality gates.

A cheap workflow that produces worse implementation is not successful.

But where quality remains equivalent, the Work Engine could determine whether stronger planning enables economically useful execution by less capable models.

---

# 13. Primary Hypothesis

The central hypothesis is:

> **A sufficiently explicit structural plan representation can reduce the capability required of downstream builders without proportionally increasing planner inference cost.**

Several subordinate hypotheses follow.

### H1 — Structural compression

A normalized Plan IR represents equivalent implementation semantics using fewer tokens than sufficiently explicit narrative planning.

### H2 — Reduced reconstruction

Builders receiving structural plans spend less inference reconstructing dependencies, constraints, authority boundaries, and acceptance relationships.

### H3 — Capability compensation

Lower-capability builders perform better when receiving appropriately elaborated views of a canonical structural plan.

### H4 — Semantic stability

Different model-specific renderings preserve the same underlying implementation contract more reliably than independently generated model-specific plans.

### H5 — Review efficiency

Stable semantic identifiers and traceability reduce supervisor/reviewer context and repair cost.

### H6 — Planning leverage

Additional upstream reasoning required to create the structural plan is smaller than the downstream reasoning/rework it eliminates for at least some task classes.

---

# 14. Relationship to Model-Specific Builder Variants

Model-specific builders would still be useful.

However, their purpose becomes narrower.

A builder variant should primarily define things such as:

- context packaging;
- Plan IR resolution level;
- rendering strategy;
- instruction/register adaptation;
- permitted autonomy;
- escalation thresholds;
- verification expectations;
- tool-use policy;
- retry/recovery policy;
- known model-specific failure signatures.

The builder role itself should continue to own the shared semantic contract.

Conceptually:

```text
builder role
    │
    ├── Sol execution profile
    │
    ├── Luna execution profile
    │
    ├── Claude execution profile
    │
    └── future model profiles
```

The model changes the execution profile.

It does not change what `builder` means.

---

# 15. Potential Broader Value

Although the immediate motivation is deciding whether models such as Luna can safely perform builder work, the Plan IR could have broader architectural value.

A canonical structural plan could potentially support:

- multi-model routing;
- multi-agent parallel execution;
- supervisor assignment;
- slice decomposition;
- dependency-aware scheduling;
- context reconstruction;
- implementation provenance;
- automated plan validation;
- reviewer packet generation;
- repair targeting;
- workflow metrics.

This suggests the Plan IR should be evaluated as a possible architectural primitive rather than merely as prompt formatting.

---

# 16. Machine-Checkable Plan Quality

A structural representation may allow plan quality to be partially validated mechanically.

Possible checks include:

```text
Does every change trace to an objective?

Does every hard invariant constrain relevant changes?

Does every acceptance criterion trace to a change,
objective, or invariant?

Does every unresolved judgment have an authority owner?

Does every escalation condition identify where control transfers?

Do dependencies contain cycles?

Are terminal changes covered by verification?

Are there changes with no acceptance evidence?

Are architectural decisions being left implicitly to builders?

Are forbidden transitions represented?

Are references valid and complete?
```

These checks cannot establish that the plan is correct.

They may establish that the plan is structurally incomplete or internally inconsistent.

That is still valuable.

---

# 17. Risks

## 17.1 Schema overfitting

The Plan IR could become tailored too tightly to current Work Engine workflows.

It should encode durable semantics rather than current implementation accidents.

## 17.2 False precision

Structure can make an incorrect plan look authoritative.

Structural validation does not replace semantic review.

## 17.3 Excessive schema complexity

If every possible planning nuance becomes a field, the representation may become harder for models and humans to use than prose.

The minimum sufficient structure should be preferred.

## 17.4 Planner burden

Creating the Plan IR may consume more high-capability inference than expected.

This must be measured.

## 17.5 Implementation leakage

Over-elaboration could move coding work into the planner.

The planner/builder boundary should remain explicit.

## 17.6 Representation-loss bugs

Model-specific renderers could omit or distort important Plan IR semantics.

Rendering fidelity should therefore be testable.

## 17.7 Premature determinism

Some implementation work genuinely requires exploration.

The structure must represent uncertainty and legitimate judgment rather than attempting to eliminate them.

---

# 18. Repository Validation Before Design Commitment

The next step should not be implementation.

Codex should inspect the repository and determine whether the proposal aligns with the existing architecture and terminology.

The validation should answer at least the following.

### Existing planning representation

- How are plans represented today?
- Which existing structures already approximate a Plan IR?
- Which semantics currently exist only in prose?
- Are objectives, invariants, consequences, decisions, acceptance conditions, or judgment boundaries already represented separately?

### Workflow placement

- Where is the current planner output consumed?
- What component should own canonical plan construction?
- Is there already a natural compiler/adapter boundary between planning and builder execution?
- Would introducing a Plan IR conflict with current planner/supervisor responsibilities?

### Builder contract

- Which parts of builder input are shared role semantics?
- Which parts are currently model-specific or prompt-specific?
- Could model-specific execution profiles be introduced without changing the builder contract?

### Supervisor integration

- Could a supervisor consume the same structural plan?
- Could findings reference stable plan identifiers?
- Could acceptance evidence be traced directly to Plan IR nodes?

### Existing schemas and abstractions

- What existing repository schemas should be reused rather than duplicated?
- Does the repository already have typed concepts corresponding to:
  - objectives;
  - invariants;
  - decisions;
  - constraints;
  - implementation changes;
  - acceptance;
  - dependencies;
  - escalation;
  - judgment authority;
  - verification?

### Context and lifecycle integration

- Can the Plan IR participate in current handoff/context lifecycle mechanisms?
- Would structural references improve successor reconciliation?
- Are there provenance or checkpoint requirements the design must preserve?

### Orchestration implications

- Could the Plan IR support the emerging planner → orchestrator → supervisor architecture?
- Could dependency information support supervisor decomposition or parallel branch construction?
- Should orchestration consume the canonical Plan IR directly or a separate orchestration projection?

### Cost and complexity

- What new infrastructure would actually be required?
- Which parts could initially remain plain structured text or existing document formats?
- Can the concept be piloted without committing to a large schema or runtime subsystem?

---

# 19. Suggested Validation Outcome

Codex should return one of three broad conclusions.

### A — Existing architecture already contains most of the primitive

Identify the existing components and propose the smallest changes required to make the representation canonical and model-renderable.

### B — Concept fits, but requires a new seam

Identify the appropriate ownership boundary and propose a minimal pilot implementation.

### C — Concept conflicts with current architecture

Identify the specific conflicts and determine whether the useful properties can be achieved through an existing abstraction instead.

The validation should prefer reuse and convergence over introducing parallel concepts.

---

# 20. Suggested First Pilot

If repository validation supports the idea, the first experiment should remain small.

Select a set of implementation slices with sufficiently clear historical outcomes.

Compare at least:

```text
Sol builder
+ current plan representation

Sol builder
+ structural plan representation

lower-capability builder
+ compact structural representation

lower-capability builder
+ elaborated structural representation
```

Potential measurements:

- first-pass acceptance;
- supervisor intervention count;
- missed requirements;
- architectural deviations;
- unauthorized judgment;
- escaped defects;
- builder input tokens;
- builder output/reasoning tokens;
- planner tokens;
- supervisor/reviewer tokens;
- repair turns;
- wall time;
- escalation quality;
- semantic fidelity between plan representations.

The experiment should distinguish:

```text
plan quality
representation fidelity
builder capability
implementation outcome
```

A failure in one should not automatically be attributed to another.

---

# 21. Longer-Term Possibility

If the hypothesis survives validation and experimentation, Work Engine could eventually treat planning similarly to compilation.

```text
intent
  ↓
reasoning
  ↓
semantic plan
  ↓
validated Plan IR
  ↓
target-specific compilation
  ↓
execution
  ↓
evidence
  ↓
verification
```

Models would then become execution targets with empirically established capability profiles.

The scheduler would not need to assume that models are interchangeable.

Instead, it could ask:

```text
What semantic work is required?

How much unresolved judgment remains?

What plan resolution is available?

Which admitted model can execute this representation reliably?
```

This would allow model routing to be based on demonstrated role capability rather than model reputation alone.

---

# 22. Architectural Principle

The central design principle can be stated simply:

> **Reason once, represent structurally, elaborate as required, and preserve one semantic source of truth.**

The immediate value may be the ability to use lower-capability builders safely.

The larger value may be a planning representation that reduces repeated inference throughout the entire Work Engine workflow.

Compression would then cease to be something imposed on the plan after it is written.

It would become a natural consequence of a better specification.
