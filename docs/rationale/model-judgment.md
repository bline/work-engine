# Model judgment

Authority: Non-normative explanation; part of [PHILOSOPHY.md](../../PHILOSOPHY.md).
Scope: reasoning behind doctrine, without adding requirements.

Load when: Understanding model judgment, procedures, or the consequences of instruction pressure.

Section numbers retain their original citation identities. Links identify
related topics; they do not require reading every linked document.

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

A useful test is route-invariance: if a resulting state would be valid when reached by one legitimate route but invalid only because another route was not followed, the disputed requirement is probably procedural rather than invariant. Likewise, consequences should normally describe the property that must be established or preserved, not the activity currently believed to establish it. Otherwise procedure can re-enter the system disguised as either an invariant or a consequence.

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

### Procedures create behavioral path attractors

Procedures are different from both.

A procedure usually operates **inside** the already-valid space. It says, explicitly or implicitly, that one route should receive much more behavioral weight than other valid routes, creating a behavioral attractor toward that path.

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

## 15. High-level procedures can still be useful

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

## 16. Why exhaustive rules are dangerous

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
