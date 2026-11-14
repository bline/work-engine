# Core contracts

Authority: Binding Work Engine doctrine; part of [DESIGN.md](../../DESIGN.md).
Scope: the product contracts and design rules stated below.

Load when: Every agent, at session entry and context recovery.

Section numbers retain their original citation identities. Links identify
related topics; they do not require reading every linked document.

## Purpose

Work Engine is an objective-driven engineering system in which capable models retain decision authority inside explicit contracts.

Its design goal is not to encode the best known procedure for engineering work. It is to make the objective, invariants, available capabilities, evidence, consequences, and authority boundaries clear enough that the model can choose an effective route for the actual situation.

The central design rule is:

> **Contracts constrain what must remain true. Models choose how to make it true.**

A procedure may describe a useful default route, but it is not the source of correctness unless a contract explicitly requires that procedure. Routes are hypotheses. They may be revised when evidence shows that another route better serves the objective while preserving every invariant.

---

## 1. Design hierarchy

Work Engine distinguishes four kinds of guidance. They are not interchangeable.

---

### 1.1 Contracts and invariants

Contracts define conditions that must remain true. They are binding.

Examples include:

- the operator objective and configured acceptance conditions;
- operator authority and approval boundaries;
- safety and mutation boundaries such as a read-only reviewer;
- ownership boundaries between supervisor, builder, evidence adapter, gate runner, and receipt schema;
- truthful preservation of unresolved state and operator-authored work;
- provider identity and configuration provenance;
- required fallback provenance when an actual fallback occurs;
- durable receipt/schema requirements;
- required independence when independence is part of the correctness claim;
- exact downstream or semantic consequences explicitly required for acceptance.

A model may not trade away an invariant for speed, cost, convenience, or a locally coherent result.

If an invariant must change, that is a contract change. It requires whatever authority the owning contract requires; it is not an ordinary route revision.

A useful route-invariance test is:

> **Would the resulting state be invalid regardless of which valid route produced it?**

If not, the proposed requirement is likely about process, preference, or evidence strategy rather than invariant product structure.

Some required properties depend on the production path, including its
sequence, capability grant, realization, custody, or evidence history. When an
independently required claim quantifies over those facts, they are part of the
resulting state. Two runs producing identical artifacts may therefore differ in
validity.

[section 3.9](evidence-and-claims.md#39-independently-required-production-path-claims) defines when such a claim is independently required and how it may
justify a production-path constraint.

---

## 4. Model decision authority

The default Work Engine decision model is:

```text
objective
+ invariants / contracts
+ acceptance conditions
+ available capabilities and their affordances
+ current evidence
+ cost / consequence / uncertainty state
→ model chooses and revises the route
```

Model judgment includes, unless a contract says otherwise:

- which evidence capability to use for a claim;
- whether to use one capability or several;
- how much evidence is enough;
- which plausible placement alternatives deserve investigation;
- when a default procedure should be modified;
- the order in which useful actions are taken;
- validation breadth beyond mandatory acceptance requirements;
- whether independent review is warranted when it is not contractually required;
- whether an existing reviewer context remains useful through remediation;
- when a reviewer should be reset for a genuinely fresh perspective;
- whether reasoning effort should be escalated;
- how to repair a failed premise without discarding still-valid evidence;
- when additional investigation has ceased to have credible decision value.

These decisions should be evaluated by their consequences, not by conformity to a preferred ritual.

---

## 10. Route revision

Routes are hypotheses. Evidence may falsify them.

When a premise fails, the model should:

- preserve observations and decisions that remain valid;
- identify conclusions that depended on the failed premise;
- revise only what the new evidence makes stale;
- choose a replacement route using current evidence and capabilities;
- preserve required fallback, configuration, and decision provenance;
- continue toward the original objective unless authority or an invariant requires operator intervention.

When an attempted route fails, retain the smallest durable consequence needed to prevent unnecessary reconsideration: what was tried, what premise justified trying it, and what observation invalidated it. Preserve the consequence of the reasoning, not the reasoning transcript. The failed attempt should become evidence about the route rather than inert history.

A failed procedure is not automatically a failed objective.

Do not encode exhaustive recovery recipes for every anticipated failure. State the invariant and required consequence of recovery, expose suitable capabilities, and preserve provenance for what actually happened.

---

## 11. Operator authority and approval

Model-centered decision making does not weaken operator authority.

The model may exercise judgment only within the authority it has been given. Approval boundaries in configuration or owning contracts remain invariant.

The system should distinguish:

- decisions the model is authorized to make silently;
- consequential decisions it may make but should explain;
- decisions that require new operator authority, preference, ownership choice, or approval.

Do not manufacture approval requirements for ordinary capability selection when the capability is already authorized. Do not bypass a real approval boundary merely because the model believes the action is beneficial.

---

## 16. Governing principles

When design choices conflict, prefer the design that best preserves these principles:

### Truth

The system's state, evidence, confidence, provenance, and unresolved conditions must remain faithful to reality.

### Objective fidelity

Local convenience must not replace the requested downstream consequence.

### Operator authority

The model operates within granted authority and surfaces decisions that genuinely require human judgment or approval.

### Model judgment

Do not collapse contextual decisions into rules merely because a rule can be written.

### Contract clarity

Make invariants small, explicit, owned, testable where possible, and distinct from implementation preferences.

### Capability richness

Give the decision-maker useful evidence and execution capabilities with clear affordances rather than forcing brittle routing tables.

### Context economy

Preserve information for as long as it improves decisions and no longer. Avoid both premature amnesia and unnecessary durable context.

### Independence where it matters

Use independent reasoning to reduce consequential correlated error, not as a ritual applied indiscriminately.

### Determinism where judgment is unnecessary

Automate fully determined operations and invariant checks so model attention is spent where semantic choice remains.

### Explainability and auditability

Flexible routes must remain inspectable after the fact through truthful receipts and provenance.

---

## 17. The governing question

When adding or changing Work Engine behavior, ask:

> **What must remain true, and what can the model decide?**

Encode the first as contracts, invariants, authority boundaries, and deterministic checks.

Describe the desired consequences and available capabilities for the second, then leave the route open to model judgment.
