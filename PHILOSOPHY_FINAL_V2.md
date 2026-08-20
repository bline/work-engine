# Work Engine Philosophy

This document explains the reasoning behind Work Engine's design principles.

It is intentionally **non-normative**.

`DESIGN.md` defines the product's structural contracts and governing design rules. This file explains why those rules exist, what failure modes motivated them, and how we think about model-centered workflows. New implementation requirements should not be inferred from this document unless they are promoted into `DESIGN.md` as explicit product structure.

---

## 1. Model judgment is the default problem-solving mechanism

Work Engine is built around a simple observation:

> A capable model usually performs better when it is given an objective, relevant evidence, available capabilities, and real constraints than when it is given a detailed procedure for solving the problem.

Procedures are attractive because they feel safe and deterministic. They can also be harmful.

A procedural rule creates a high-confidence action path. Once the current situation resembles the rule's trigger, following the rule becomes an inexpensive way for the model to satisfy the instruction. That can suppress contextual alternatives even when those alternatives would be better.

The failure mode is not merely rigidity. Enough procedural rules can make the model appear less intelligent:

```text
observe situation
→ match procedure
→ execute known path
```

replaces:

```text
observe situation
→ build a model of the problem
→ understand consequences
→ compare possible actions
→ choose
```

The purpose of Work Engine is not to remove structure. It is to place structure only where structure is load-bearing.

---

## 2. Structure and judgment are different things

We distinguish several kinds of model-facing information:

```text
invariant structure   → what must remain true
variant structure     → what currently exists and why
state / evidence      → what is true in this run
objective/consequence → what outcome matters and what failure means
judgment              → what to do
```

This separation is central.

The model should not be asked to infer product contracts that the product already knows. At the same time, the product should not convert today's preferred method into tomorrow's mandatory procedure.

---

## 3. Invariant structure as the hard frame

The canonical taxonomy for Work Engine is the five-way split introduced above:

```text
invariant structure
variant structure
state / evidence
objective / consequence
judgment
```

The older **hard map / fuzzy map** language is only a metaphor for one relationship inside that taxonomy. It is not a second ontology.

The **hard frame** corresponds specifically to **invariant structure**: the product facts that must not drift with interpretation.

Examples include:

- ownership;
- authority;
- read/write boundaries;
- required interface shapes;
- security constraints;
- provenance obligations;
- human-approval boundaries;
- other properties whose violation makes the product operate incorrectly.

These are stable reference points.

> **Pin the coordinate system, not the path.**

The purpose of invariant structure is not simply restriction. It removes foundational questions the model should not have to repeatedly infer.

Without structural anchors, a model may have to reason about questions such as:

- Who owns this?
- May this component mutate that?
- Is this artifact authoritative?
- What shape does the downstream consumer require?
- Does this transition require provenance?
- Is this review meant to be independent?

If the product already knows the invariant answers, leaving those dimensions open merely increases the reasoning problem.

Invariant structure removes invalid degrees of freedom.

> **Remove invalid degrees of freedom while preserving every legitimate degree of freedom.**

That is the intended balance.

---

## 4. The fuzzy operational map

The **fuzzy map** is a metaphor for the mutable operational world the model is working inside. It includes the current **variant structure**, the current **state/evidence**, and the model's revisable interpretation of what those facts imply.

"Fuzzy" does not mean vague, low-confidence, or poorly understood.

When the model zooms into the map, it should ideally see a machine it understands: known controls, known capabilities, known state, prior context, and known consequences. What makes the map fuzzy is that its configuration and state can change.

Variant structure is therefore a major part of the fuzzy map:

- the current machinery;
- the controls the model can manipulate;
- the capabilities available;
- agent and reviewer lifetimes;
- provider arrangements;
- component topology;
- current configuration.

State/evidence describes where that machinery and its environment are now.

Judgment determines which available manipulation is appropriate.

A useful relationship is:

```text
INVARIANT STRUCTURE
fixed product frame
        │
        ▼
FUZZY OPERATIONAL MAP
variant machinery + current state + evidence
        │
        ▼
JUDGMENT
choose how to manipulate the environment
        │
        ▼
NEW OPERATIONAL STATE
```

The hard map does not move during normal product operation. The fuzzy map does.

A surprising new fact should usually change the fuzzy operational map without forcing the product's invariant structure to be reconstructed.

The fixed frame keeps foundational product relationships stable while the operational map changes.

The model should not need to reconstruct the machine itself every time it acts. It should be able to understand the machinery, inspect its current state, and decide what state transition makes sense now.

---

## 5. Why this resembles human structure

Human reasoning also occurs inside a mixture of stable and changing structure.

A person does not normally reconsider every fundamental relationship or environmental regularity before choosing what to do next. Physical reality, social roles, relationships, responsibilities, institutions, and other durable structures constrain the range of plausible interpretation and action.

A person may be a parent, spouse, employee, student, tenant, citizen, or caregiver. Those roles do not dictate every action. They constrain and organize the space in which action is chosen.

Likewise, ordinary environmental regularities provide a stable frame. People do not need to reconstruct their whole model of the world every morning before acting.

Work Engine deliberately provides its models with an analogous kind of structural certainty.

The analogy is not that a model reasons exactly like a human. The useful point is architectural: **stable structure reduces the number of dimensions that must remain open at once.**

That can make reasoning more efficient and more coherent.

---

## 6. Good constraints and bad constraints both reduce semantic space

All strong instructions constrain the model's possible continuations.

The important question is **what they remove**.

A good structural invariant removes invalid worlds:

```text
reviewer mutates implementation
→ independence claim becomes false
→ that state is invalid
```

A bad procedural rule removes legitimate solutions:

```text
always use capability A before capability B
→ alternatives disappear
even when B is clearly better for this claim
```

Both reduce the model's action space.

Only one reduces the right part of it.

This distinction explains why Work Engine is not anti-command or anti-constraint. Correctly chosen constraints reduce complexity. Incorrectly chosen procedural constraints reduce capability.

---

## 7. Boundary trimming, interior enrichment, and path concentration

There are three different ways model-facing structure can change the effective problem space.

### Invariant structure trims the boundary

Invariant structure removes worlds the product considers invalid.

It does not choose among the remaining valid solutions. It simply says that some states, transitions, or outcomes are outside the product contract.

This is productive reduction of the search space because the removed possibilities were never legitimate solutions.

### Variant structure enriches the interior

Variant structure does not primarily remove possibilities. It describes the machinery available inside the valid space and can make additional valid outcomes reachable.

Capabilities, controls, lifetimes, provider arrangements, topology, and state transitions add usable dimensions to the model's operational environment.

This enrichment should be meaningful rather than redundant. A new capability is valuable when it adds expressive reach or makes an important distinction legible. Multiple indistinguishable controls that lead to the same result can increase search complexity without adding useful agency.

### Procedures concentrate probability onto a path

Procedures are different from both.

A procedure usually operates **inside** the already-valid space. It says, explicitly or implicitly, that one route should receive much more behavioral weight than other valid routes.

That is the dangerous form of narrowing.

A useful shorthand is:

> **Trim invalid space. Enrich valid space. Avoid unnecessary path concentration inside valid space.**

The concern is not that procedures literally expose a readable probability table. The claim is behavioral and mechanistic: instruction-tuned models are trained to respond strongly to explicit directives, so a literal procedure can sharply bias the continuation toward compliance.

This should not be read as privileged introspection into model logits. It is an architectural model grounded in known instruction-following behavior and observable agent behavior, not a claim that the model can inspect its own token probabilities.

In coding-agent settings, where decoding is often configured for deterministic behavior, that bias can become especially visible.

The important failure mode is that procedural concentration may occur **before** useful exploration.

Instead of:

```text
observe
→ explore evidence
→ generate alternatives
→ compare consequences
→ choose
```

the model may effectively do:

```text
observe
→ match known procedure
→ follow procedure
```

If that happens early enough, the evidence that would have revealed a better route may never be sought or generated.

This gives a plausible explanation for a recurring Work Engine observation: adding more procedural guidance can make a capable model appear less capable even in areas where it technically still has permission to choose.

The model may not be choosing badly after considering alternatives. It may be generating fewer alternatives in the first place.

Coding is particularly sensitive to this because most coding problems are open, multimodal spaces. Many implementations can satisfy the same specification. A procedure that locks in one route therefore discards alternatives the procedure author often had no basis to exclude.

The target state is:

```text
real invariant boundary
        ↓
valid problem space
        ↓
rich, composable machinery
        ↓
model judgment over open valid routes
```

not:

```text
valid problem space
        ↓
rich machinery
        ↓
procedure selects one anticipated route
        ↓
model mostly executes that route
```

---


## 8. Commands are structural anchors

Commands should represent the minimum product structure required for the system to function correctly.

They should encode things such as:

- authority;
- ownership;
- interface contracts;
- security;
- mutation boundaries;
- provenance requirements;
- explicit approval boundaries.

They should not encode mutable problem knowledge or preferred routes.

A command like:

> Reviewers used as independent evidence are read-only.

is structural.

A command like:

> Use Codebase Memory first and direct filesystem reads only when Codebase Memory fails.

is procedural.

The first establishes a product property. The second encodes one solution strategy.

---

## 9. Every command should have a causal parent

A bare command is easy to follow but poor at supporting generalization.

Compare:

> Always output `ReceiptSchema`.

with:

> The downstream consumer validates against `ReceiptSchema`. A nonconforming receipt cannot be consumed and invalidates the audit artifact. Emit a receipt conforming to `ReceiptSchema`.

The second contains both reason and command.

That matters because the reason creates **continuity of logic**.

The model can carry the causal relationship forward into later reasoning:

```text
causal reality
→ failure mode
→ invariant
→ minimal command
→ later judgment
```

The command remains firm, but it is not an unexplained island in the context.

We call this the **Logical Continuity Principle**:

> A structural command should be connected to the causal reality that makes it necessary, so the reason for the invariant remains available to later reasoning.

This is especially valuable in novel situations. The model can preserve the underlying property instead of blindly repeating a ritual whose purpose it does not understand.

---

## 10. Variant structure: describe the machine, not how to use it

Not all structure is invariant.

The product also has **variant structure**: machinery that exists today but may evolve.

Examples include:

- available capabilities;
- agent topology;
- persistence mechanisms;
- provider arrangements;
- current component boundaries;
- retrieval infrastructure;
- review infrastructure;
- telemetry mechanisms;
- repository adapters.

Variant structure should be presented as:

- what exists;
- what it affords;
- why it exists;
- what tradeoffs shaped it;
- what consequences follow from its current design.

It should not automatically become a procedure.

For example:

> Codebase Memory is optimized for structural repository understanding. Direct filesystem tools provide source-level observation and can reach material outside the index.

This teaches the model what the machinery is good at.

It does not say:

> Always use Codebase Memory first.

The distinction is important.

Variant structure should enrich the model's understanding of the environment, not dictate interaction with that environment.

A useful pattern is:

```text
current machinery
→ rationale
→ affordances and limitations
→ model judgment
```

There is deliberately no mandatory action at the end unless an invariant requires one.

---

## 11. Capability affordances instead of routing tables

A capability description should tell the model what a tool can establish and what kinds of evidence it exposes.

The model then chooses among available capabilities according to the current claim.

This has practical benefits.

A reviewer with both indexed repository knowledge and direct read-only filesystem access can move between them without losing context:

```text
structural question
→ indexed evidence

exact source claim
→ direct source observation

new relationship question
→ indexed evidence again
```

There is no need to model this as a rigid transition between modes.

However, actual fallback remains auditable.

These are different events:

**Capability selection**

The model freely chooses the evidence source best suited to the claim.

**Evidence fallback**

An earlier evidence route proves unavailable, ambiguous, insufficient, or failed, and another route is used because of that failure.

**Provider/configuration fallback**

The configured provider itself must change.

Only the latter two are fallback events. Their required provenance remains a product contract.

Flexibility must not weaken auditability.

---

## 12. Variant structure as a manipulable state machine

Variant structure is not merely a description of components. It is the model's **manipulable environment**.

A useful mental model is a state machine with controls.

The product can either prescribe a path:

> To accomplish X, turn knob Y and press button Z.

or describe the machine:

> Knob Y controls pressure. Button Z commits the current configuration. Indicator Q reports stability.

The second form preserves agency.

The model understands what each control does and can compose controls according to the current state and objective. The combinations of those controls become the model's way of manipulating environmental state.

This makes interface design an action-space design problem.

### Insufficient agency

If the model is not given enough controls, some legitimate outcomes are unreachable.

The model may correctly understand what should happen but have no available sequence of actions capable of making it happen.

That is a product failure even if the model's reasoning is correct.

### Excessive agency

If the model is given too many overlapping, redundant, or poorly distinguished controls, the number of possible action combinations grows without adding equivalent value.

The model must then spend reasoning effort choosing among multiple paths that lead to the same state or interpreting distinctions that do not matter.

That enlarges the search space unnecessarily.

### The intended balance

The goal is not maximum freedom.

The goal is:

> **Provide enough composable machinery to make intended outcomes reachable, but do not expose redundant or irrelevant degrees of freedom that only enlarge the model's search space.**

This is the canonical action-space principle from `DESIGN.md`; the discussion below expands its reasoning rather than redefining it.

The machinery should be compositionally rich enough that the model can construct novel solutions from a small set of meaningful primitives, without requiring one dedicated control for every anticipated situation.

This is similar to a good instruction set: a small number of coherent primitives can generate sophisticated behavior when their semantics are clear and their combinations are expressive.

For example, reviewer recovery need not require a dedicated `recover_from_stale_reviewer_context()` procedure if the environment already exposes meaningful primitives such as:

- inspect reviewer state;
- continue reviewer;
- checkpoint context;
- spawn a new reviewer;
- supply context;
- terminate reviewer.

The model can compose those controls according to the actual situation.

### Interface complexity is reasoning complexity

Every control exposed to the model introduces another possible state transition or combination of transitions.

That means interface design directly shapes the model's reasoning space.

A useful test for a new capability is:

> Does this control make a legitimate state transition reachable that the model could not already express cleanly?

And the inverse:

> Does this control merely add another path to an already-reachable state without adding useful meaning?

Variant structure exists to shape that action space deliberately.

Invariant structure removes invalid states and invalid transitions.

Variant structure exposes the valid machinery.

State/evidence tells the model where it is.

Objectives and consequences tell it what matters.

Judgment chooses the transition.

```text
current state
+ available transitions
+ invariant constraints
+ objective / consequences
+ evidence
────────────────────────
judgment
    ↓
chosen manipulation
    ↓
new state
```

This is the operational core of model-centered control.

---

## 13. Context has value and lifetime

Model context is not merely temporary storage. It contains accumulated interpretation.

Discarding useful context forces the system to reconstruct:

- architectural understanding;
- prior findings;
- accepted invariants;
- unresolved questions;
- evidence already evaluated;
- reasoning about why previous findings mattered.

This can be extremely expensive.

For that reason, context lifetime should be chosen according to the value of retained reasoning.

A reviewer is a good example.

A useful lifecycle is:

```text
fresh independent reviewer
→ initial review
→ findings
→ builder remediation
→ same reviewer evaluates the delta
→ repeat while context remains useful
→ discard
```

Freshness matters at entry because independence matters.

Persistence matters during remediation because the reviewer already understands the problem.

The choice to retain or reset that context should itself be model judgment unless a product contract imposes a boundary.

This follows the same philosophy: preserve useful structure, preserve useful knowledge, and avoid procedural lifecycle rules that destroy value without a causal reason.

---

## 14. High-level procedures can still be useful

Work Engine does not require the absence of procedures.

A high-level procedure can encode accumulated experience and provide a good default path.

For example, a high-assurance change may commonly benefit from:

```text
explore
→ choose
→ state the architectural claim
→ falsify
→ implement
→ prove
```

The important distinction is that such a route is a **hypothesis about a useful process**, not an invariant product contract.

The model may revise it when current evidence makes another route better, provided all true invariants and required consequences remain satisfied.

Procedures should therefore have escape by judgment, not exhaustive exception tables.

An exception table tries to enumerate semantic space.

Judgment allows the model to interpret semantic space.

---

## 15. Why exhaustive rules are dangerous

A procedural author can rarely anticipate the complete set of future situations.

Rules such as:

> If X, do Y.

are especially dangerous when X belongs to an open semantic domain.

As rules accumulate, the product may accidentally flatten a rich problem space into the designer's finite taxonomy of known cases.

The model then has a cheap path:

```text
classify case
→ find matching rule
→ execute
```

This is attractive computationally and can suppress deeper reasoning even where judgment was intended.

The preferred alternative is to describe:

- the real constraint;
- the consequence of violating it;
- the machinery available;
- the evidence currently known;
- the objective.

Then allow the model to reason.

---

## 16. The role of the design document

The design philosophy should not itself become workflow bloat.

That is why this file is separate from `DESIGN.md`.

`DESIGN.md` should contain the compressed product doctrine:

- the invariant structure;
- the first-class categories;
- the minimum design tests needed to keep new features aligned.

This file may contain:

- metaphors;
- examples;
- historical reasoning;
- failure modes;
- competing formulations;
- explanations of why the design evolved.

The distinction is deliberate:

> **Conversation and philosophy may be expansive. Product doctrine should be compressed.**

A future author should be able to understand the "why" here without forcing every agent to ingest the entire reasoning history during ordinary execution.

---

## 17. A compact statement of the philosophy

Work Engine aims to give models:

- a small, stable set of structural anchors;
- clear causal explanations for those anchors;
- an accurate description of the machinery currently available;
- the relevant state and evidence;
- explicit objectives and consequences;
- freedom to choose the method.

The intended result is neither unrestricted agency nor procedural automation.

It is a model-centered system in which:

> **Structure defines the valid world. Knowledge describes the current world. Judgment chooses the path through it.**
