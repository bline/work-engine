# Agent instruction design

Authority: Binding Work Engine doctrine; part of [DESIGN.md](../../DESIGN.md).
Scope: the product contracts and design rules stated below.

Load when: Authoring or reviewing skills, role prompts, loading rules, or other agent-facing instructions.

Section numbers retain their original citation identities. Links identify
related topics; they do not require reading every linked document.

## 2. Model-facing structure

Work Engine presents the model with several distinct kinds of information. Keeping them separate prevents current machinery or accumulated experience from silently becoming mandatory procedure.

---

### 2.1 Invariant structure

Invariant structure is the product's load-bearing structure: authority, ownership, interface contracts, mutation boundaries, provenance obligations, security constraints, and other properties that must remain true for the product to operate correctly.

Invariant structure may be expressed as commands because variation would break the product contract. Each command should be paired with the causal reason and failure mode that make the invariant necessary.

Invariant structure provides stable reference points for reasoning. Inferred understanding may shift as evidence changes; the structural anchors do not. The purpose is to localize uncertainty, not eliminate judgment.

---

### 2.3 Runtime instruction projection

Runtime agent instructions are role-scoped projections of the owning doctrine, not independent policy layers. They should carry only the invariant structure, consequences, authority boundaries, and capability affordances relevant to that role and current machinery.

A runtime instruction may specialize how an existing contract applies to a role, but it must not silently create a new binding procedure or redefine an owning invariant. Binding runtime rules should remain traceable to the contract or product property that makes them necessary.

This keeps ordinary agent context small while preserving design lineage: expansive reasoning may live in philosophy, canonical doctrine in design, and role-relevant projections in runtime instructions.

---

## 3. Commands are product structure

Commands are the load-bearing structure of Work Engine. They exist where the product cannot function correctly unless a boundary, interface, authority relationship, or invariant is preserved.

Commands do **not** exist to encode the model's preferred solution to a problem.

A useful separation is:

```text
structure  → commands / contracts / authority
knowledge  → reasons / consequences / affordances / observations
judgment   → model chooses actions inside the structure
```

The structure keeps the cogs engaged correctly. Knowledge explains the environment in which they operate. Judgment determines how to move through the current situation.

---

### 3.1 Every command needs a causal parent

A bare command is an opaque policy. It terminates reasoning at "follow this rule."

Whenever possible, express the reality that makes the command necessary before stating the command itself:

> **Reason:** The downstream consumer validates against `ReceiptSchema`. A nonconforming receipt cannot be consumed and invalidates the audit artifact.
> **Command:** Emit a receipt conforming to `ReceiptSchema`.

The explanation gives the model a continuous chain of logic that remains useful when circumstances change. The command preserves the irreducible structural boundary.

A command without a coherent failure mode is suspect.

For every command, the design should be able to answer:

> **What concrete product property fails if this command is violated?**

If there is no clear answer, the statement is probably procedure, preference, or current knowledge rather than product structure.

---

### 3.2 Commands must encode structure, not data

Commands may encode:

- authority boundaries;
- ownership boundaries;
- mutation and read-only boundaries;
- externally required interface contracts;
- security and privacy constraints;
- provenance obligations required for auditability or downstream validation;
- acceptance properties that cannot be relaxed by model judgment;
- explicit human-approval boundaries where authority remains with the human.

Commands must not encode mutable problem knowledge or a preferred solution route.

In particular, commands should not encode:

- file lists or current repository layout;
- inventories of known cases presented as exhaustive semantic space;
- tool preferences or routing tables;
- current architectural observations or conclusions;
- examples promoted into general rules;
- heuristic thresholds unless the threshold is itself an external contract;
- provider preferences unless provider identity is contractually fixed;
- anticipated recovery recipes;
- sequencing unless order is causally required;
- situational facts that may change between runs;
- domain conclusions that belong in evidence or reasoning.

A command should remain valid when repository facts, tools, models, or evidence change. If changing those facts can make the command obsolete without changing the product contract, the command probably contains knowledge that belongs elsewhere.

---

### 3.3 Commands must be minimal

A command should state the smallest behavior necessary to preserve the protected structure.

Do not command a method when only an outcome is structurally required.

For example:

> **Reason:** Review evidence is used to reduce correlated builder error. If the reviewer can mutate the implementation, the claimed independence no longer holds.
>
> **Command:** Reviewers used as independent evidence are read-only.
>
> **Evidence:** Because the reviewer is the acting role whose independence is
> claimed, its own record cannot independently attest the grant. Independently
> observe the realized capability grant and preserve the claim evidence required
> by [sections 3.9 and 12](evidence-and-claims.md).

Do not expand this into a procedural ritual such as which files must be opened, which tool must be tried first, or how many review passes must occur unless those details are themselves contractually necessary.

---

### 3.4 Commands must not collapse semantic space unnecessarily

Strong imperatives create high-confidence policy attractors. That is useful when variation would break the product; it is harmful when contextual variation is exactly what enables good problem solving.

Prefer:

> **Reason:** An actual fallback without provenance leaves the audit trail incomplete and can make the receipt invalid.
> **Command:** Record required fallback provenance when a fallback occurs.

over:

> Always use capability A first, then capability B, and record why B was needed.

The first preserves the structural contract while leaving capability choice to judgment. The second encodes a solution path and collapses alternatives that may be better in the current situation.

---

### 3.5 Sequencing is structural only when causality requires it

Commands may impose order when reversing the order would invalidate the result, destroy required evidence, cross an authority boundary, or otherwise violate a contract.

Otherwise, sequencing belongs to model judgment.

"Validate before publication because publication makes the artifact authoritative" may be structural.

"Search before reading because search is usually cheaper" is an affordance or default, not structure.

---

### 3.6 Exhaustive commands require genuinely closed worlds

Avoid imperative language such as:

> In these cases, always do X.

unless the set of cases is closed by an external specification or product contract.

Open-ended semantic domains should be described through consequences and affordances. Enumerating today's known cases and commanding behavior for them silently flattens future cases into an incomplete taxonomy.

---

### 3.7 Failure-mode explanations are knowledge, not escape hatches

Explaining why a command exists does not weaken the command.

The model remains bound by the structural invariant, but now possesses causal knowledge that can propagate through later reasoning. When a novel situation appears, the model can preserve the underlying property rather than blindly reproduce an old ritual. This preserves **logical continuity**: the command remains connected to the causal reason that makes it necessary, so that reason can continue to constrain later judgment.

The intended pattern is:

```text
causal reality
→ failure mode
→ structural invariant
→ minimal command
→ required establishment evidence when the invariant is a production-path claim
→ model judgment everywhere outside that boundary
```

When a command protects a required production-path claim, its effective
instruction must expose the evidence obligation defined by [section 3.9](evidence-and-claims.md#39-independently-required-production-path-claims) as well
as the constraint. Constraint without admissible evidence establishes no claim
that a consumer can rely upon.

Invariant structure should provide stable reference points without prescribing the route between them: **pin the coordinate system, not the path**.

The goal is to **remove invalid degrees of freedom while preserving every legitimate degree of freedom**.

---

### 3.8 Command review test

Before adding or retaining imperative language, ask all of the following:

1. What product property fails if this command is violated?
2. Would the resulting state be invalid regardless of which valid route
   produced it? For an independently required production-path claim under
   [section 3.9](evidence-and-claims.md#39-independently-required-production-path-claims), would violating the command make the claim false, or merely
   leave it unestablished? Falsity may justify constraining the work route.
   Unestablishability instead requires admissible evidence and may require
   binding an evidence-capable realization under [section 3.9](evidence-and-claims.md#39-independently-required-production-path-claims).
3. Is that property genuinely invariant, or merely desirable in the current situation?
4. Does the command encode only structure, authority, or an irreducible interface contract?
5. Has mutable data, current knowledge, a heuristic, or a preferred route leaked into the command?
6. Could a different method preserve every required consequence? If so, is the command unnecessarily prescribing that method?
7. Does any required sequencing preserve an observable consequence or prevent
   a violating fact from crossing an independently required claim's consumption
   boundary? If only attribution or independent attestation is required, could
   the route remain open while the relevant path facts are recorded or observed?
8. Is the command paired with enough explanation for the model to understand why the boundary exists?
9. Would the command remain valid if tools, repository structure, models, or evidence changed while the product contract stayed the same?

If the answer reveals that the statement is not structural, convert it into a consequence, affordance, observation, or default route instead of a command.

---

## 5. Contracts are not procedures

A recurring design error is to turn a desired invariant into a prescribed implementation path.

For example, this is a contract:

> An independent review used as evidence of independence must begin independent of the builder's reasoning context.

This is not necessarily a contract:

> Start a brand-new reviewer process after every builder fix.

The first defines the property that matters. The second is one possible implementation and can destroy useful context without increasing independence.

Likewise, this is a contract:

> Every actual evidence fallback must carry the provenance required by the receipt schema.

This is not a contract:

> Always try Codebase Memory first and use filesystem access only after it fails.

The model may freely select the best evidence capability for a claim. A fallback exists only when execution changes or augments an earlier route because that route was unavailable, ambiguous, insufficient, or failed. When that happens, the fallback provenance contract remains binding.

### Design test

Before adding a mandatory rule, ask:

> **What invariant would become false if the model chose a different method?**

If there is no clear answer, the proposed rule is probably procedure rather than contract. Express the desired consequence or capability affordance instead.
