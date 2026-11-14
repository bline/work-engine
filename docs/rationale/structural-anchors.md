# Structural anchors and logical continuity

Authority: Non-normative explanation; part of [PHILOSOPHY.md](../../PHILOSOPHY.md).
Scope: reasoning behind doctrine, without adding requirements.

Load when: Understanding authority, structural commands, causal explanations, or stale reasoning.

Section numbers retain their original citation identities. Links identify
related topics; they do not require reading every linked document.

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

## 5. Why this resembles human structure

Human reasoning also occurs inside a mixture of stable and changing structure.

A person does not normally reconsider every fundamental relationship or environmental regularity before choosing what to do next. Physical reality, social roles, relationships, responsibilities, institutions, and other durable structures constrain the range of plausible interpretation and action.

A person may be a parent, spouse, employee, student, tenant, citizen, or caregiver. Those roles do not dictate every action. They constrain and organize the space in which action is chosen.

Likewise, ordinary environmental regularities provide a stable frame. People do not need to reconstruct their whole model of the world every morning before acting.

Work Engine deliberately provides its models with an analogous kind of structural certainty.

The analogy is not that a model reasons exactly like a human. The useful point is architectural: **stable structure reduces the number of dimensions that must remain open at once.**

That can make reasoning more efficient and more coherent.

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

## 10. Logical discontinuity and inferential debt

[Section 7](model-judgment.md#7-boundary-trimming-interior-enrichment-and-path-concentration) described how procedural pressure can concentrate traversal before alternatives are explored. A related but distinct failure occurs when a generated assertion is not supported by a reasoning path that later analysis can reconstruct or endorse.

**Logical discontinuity** occurs when an assertion enters the interaction but later analysis, using the surrounding evidence and logic, would reject or materially revise it.

The claim is behavioral rather than introspective. The relevant observation is not that the model secretly held another internal belief. It is simply that the earlier output does not align with what the model produces when that output is itself subjected to analysis.

Once emitted, however, the assertion becomes part of the transcript and therefore part of the evidence presented to later reasoning. If its causal support is missing or stale, subsequent reasoning may continue accommodating, defending, reinterpreting, or reasoning around it.

We call the resulting burden **inferential debt**:

> **Inferential debt is the downstream reasoning cost created when an unsupported or stale assertion remains active in context.**

The original unsupported assertion is the principal. The repeated work required to reconcile later reasoning around it is the interest. That cost can include distorted interpretation of new evidence, suppression of alternatives that conflict with the prior assertion, unnecessary rationalization, and new conclusions that inherit the stale premise.

The propagation mechanism does not depend on procedures specifically. Any unsupported self-generated conclusion may create downstream debt once it becomes part of the interaction history.

The narrower Work Engine hypothesis is that strong procedural or directive pressure can be a particularly reliable producer of the initial discontinuity. The same match-and-execute shortcut discussed in [Section 7](model-judgment.md#7-boundary-trimming-interior-enrichment-and-path-concentration) can produce content that bypasses broader evaluation, not only alternative generation.

### Logical continuity also enables revisability

[Section 9](#9-every-command-should-have-a-causal-parent) introduced the Logical Continuity Principle primarily as a generalization aid. It has a second function: **revisability**.

A command connected to its causal parent gives later reasoning something it can re-evaluate. If the causal premise changes, the model can identify what downstream conclusion has become stale rather than carrying an unexplained authoritative statement forward.

A bare command is therefore dangerous not merely because it is restrictive. **It creates an authoritative node without an explicit dependency edge.**

Logical continuity serves two related purposes:

- **Generalization** — preserve the underlying property when circumstances change.
- **Revisability** — identify which conclusions cease to hold when their causal support changes.

This does not require preserving a full reasoning transcript or an elaborate dependency graph. The system needs only enough durable consequence to support later judgment.

### Route revision retires stale consequences

Route revision is the corresponding repair mechanism when continuity breaks or evidence falsifies a route.

When an attempted route fails, retain the smallest durable fact needed to prevent unnecessary reconsideration: **what was tried, what premise justified trying it, and what observation invalidated it.**

For example:

```text
X was attempted
because Y made it plausible
observation Z invalidated that route
```

The useful durable consequence is not the full history of how X was chosen. It is that **Y no longer supports X under the conditions established by Z**. This is enough to eliminate a previously tested option from the live route space under the same relevant conditions without pretending to preserve a full causal graph.

That failed attempt becomes evidence about the route rather than inert history. Later reasoning can remove that option under the same relevant conditions without reconstructing the entire investigation.

Route revision therefore **retires identified inferential debt**. When evidence invalidates a premise or route, the resulting stale conclusion should be retired explicitly rather than silently reconciled with later evidence. It cannot guarantee discovery of every downstream conclusion that inherited the stale premise, so the claim is deliberately narrower: debt that has been identified should not remain authoritative merely because it is already in the transcript. Observations and decisions that remain valid should survive; only the consequences made stale by the new evidence should be revised.
