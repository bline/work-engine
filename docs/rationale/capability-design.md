# Capability and interface design

Authority: Non-normative explanation; part of [PHILOSOPHY.md](../../PHILOSOPHY.md).
Scope: reasoning behind doctrine, without adding requirements.

Load when: Reasoning about capability affordances, mutable machinery, or the available action space.

Section numbers retain their original citation identities. Links identify
related topics; they do not require reading every linked document.

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

## 11. Variant structure: describe the machine, not how to use it

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

## 12. Capability affordances instead of routing tables

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

## 13. Variant structure as a manipulable state machine

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
