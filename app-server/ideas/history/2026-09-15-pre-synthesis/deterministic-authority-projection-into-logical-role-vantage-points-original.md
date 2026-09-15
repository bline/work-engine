# Idea: Deterministic Authority Projection into Logical Role Vantage Points

**Status:** Exploratory architecture idea

**Scope:** Semantic authority, role manifests, organizational compilation, workflow formation, execution envelopes, delegation, independence, and domain-transferable role composition

**Authority:** Exploratory only. This document does not amend current role contracts, manifests, workflow ownership, execution-envelope design, or implementation authority. It proposes a candidate principle and compilation model whose seams must be reconciled against current Work Engine architecture before adoption.

---

## Summary

Work Engine already assigns semantic decisions to different roles rather than concentrating judgment in a supervisor, planner, or operator.

The recurring design principle appears to be:

> **A semantic decision should be exercised by the narrowest authorized role whose position gives it the strongest legitimate vantage for that decision.**

The role does not receive that authority because of organizational rank.

It receives it because its logical position supplies the combination of:

- relevant evidence;
- accumulated context;
- semantic ownership;
- temporal position;
- independence;
- effect relationship; and
- authority ceiling

required to make that particular judgment truthfully.

This suggests a more general architecture.

Instead of treating semantic authority primarily as a static property declared directly on a role:

```text
role X
    may decide A
    may decide B
    may not decide C
```

Work Engine may be able to represent:

1. the requirements of semantic decisions;
2. the vantage supplied by logical roles; and
3. deterministic rules for projecting bounded authority onto roles whose vantage satisfies those requirements.

Conceptually:

```text
decision requirements
        +
logical role vantage
        +
upstream authority ceiling
        +
independence / delegation constraints
        |
        v
deterministic authority projection
        |
        v
decision-scoped role authority
```

This does **not** mean software mechanically determines who is wise enough to make an arbitrary judgment.

The semantic requirements and authority boundaries remain explicitly authored.

What may be mechanically derivable is the consequence of those declarations:

> Given this decision's declared requirements and these roles' declared vantage properties, which role is eligible to exercise this bounded authority?

If exactly one valid role exists, the authority assignment may be deterministic.

If several valid arrangements exist, a real semantic or organizational decision remains.

If none exists, Work Engine has discovered an organizational gap rather than silently assigning the judgment elsewhere.

The principle is domain transferable. Nothing in the proposed authority machinery depends on repositories, code review, implementation, or software-development vocabulary.

---

# 1. Motivation

Work Engine's current architecture already contains many decisions whose authority follows role-local context rather than hierarchy.

A supervisor may coordinate a slice without owning repository judgment.

A builder may own implementation reasoning because it retains the implementation context necessary to make those decisions.

A reviewer may own an attributed adversarial judgment while remaining unable to mutate the implementation or accept its own findings.

A deterministic evidence observer may nominate possible impact while remaining unable to decide whether the observed change semantically invalidates a claim.

A claim-maintenance role may adjudicate semantic consequence without thereby acquiring authority over every downstream consumer relying on that claim.

A scheduler may know that an obligation is due without acquiring authority to execute the obligation.

These appear at first to be separate ownership rules.

They may instead be consequences of one deeper rule:

```text
place semantic authority
where the required legitimate vantage exists
```

The current architecture mostly records the resulting assignments.

It does not yet appear to make the underlying authority-placement rule a first-class architectural object.

---

# 2. Vantage-aligned semantic authority

## 2.1 Authority is decision-scoped

Authority should not be treated as a broad property such as:

```text
builder is authoritative
supervisor is authoritative
reviewer is advisory
```

Those statements are too coarse.

A role may be authoritative for one semantic consequence and merely advisory, observational, or prohibited for another.

The meaningful unit is closer to:

```text
decision:
    determine architectural placement

authority:
    builder for this bounded slice

because:
    builder holds the required repository,
    objective, consequence, and continuity vantage
```

Authority therefore binds at least:

```text
decision identity / class
+
logical role
+
bounded subject
+
authority source / ceiling
+
applicable conditions
```

---

## 2.2 Vantage is not hierarchy

The supervisor may be organizationally upstream of the builder while possessing a worse vantage for an implementation-local decision.

Likewise, the operator may retain ultimate product authority without being the appropriate role to exercise every local semantic judgment.

Therefore:

```text
organizational rank
    != semantic vantage
    != decision authority
```

Coordination authority must not silently absorb domain authority.

Visibility into another role's work must not silently transfer that role's judgment.

Possession of an artifact must not imply authority over its meaning.

---

## 2.3 Vantage is narrower than knowledge

A role's vantage is not simply "everything this role knows."

It includes the conditions that make a judgment legitimate.

Candidate dimensions include:

- evidence available to the role;
- durable and transient state visible to it;
- context accumulated across a required interval;
- semantic objects it owns;
- consequences it may change;
- effects it may perform;
- temporal position in the work;
- independence from other roles or decisions;
- conflict-of-interest constraints;
- authority granted from an upstream source;
- explicit prohibitions;
- delegation rules; and
- continuity requirements.

A role could possess enough factual information to make a decision while still lacking the authority or independence required to own its consequence.

---

# 3. Decision requirements

A semantic decision type could declare the vantage required to exercise it.

Illustratively:

```yaml
decision: implementation-placement

requires:
  evidence:
    - repository-topology
    - accepted-objective
    - semantic-consequence-path

  continuity:
    - current-slice

  ownership:
    - implementation-result

independence:
  none

authority_ceiling:
  bounded-domain

delegation:
  non-transferable

consequence:
  establish-placement
```

This is not proposed syntax.

It illustrates a possible semantic distinction:

> The decision describes what must be true of its owner rather than naming the owner directly.

Another decision might require:

```yaml
decision: independent-implementation-review

requires:
  evidence:
    - immutable-candidate
    - review-contract

independence:
  from:
    - implementation-author

effects:
  read-only

consequence:
  attributed-review-judgment
```

The resulting role assignment may be different even when both decisions concern the same implementation.

---

# 4. Logical role vantage

A role manifest may eventually need to describe not merely what a role is called or what capabilities it has, but what logical vantage it can truthfully occupy.

Conceptually:

```yaml
role: slice-builder

observes:
  - accepted-objective
  - repository-topology
  - current-candidate
  - gate-results

owns:
  - implementation-result

continuity:
  - slice

effects:
  - mutate-candidate

independence:
  not-independent-of:
    - implementation

authority_ceiling:
  bounded-slice
```

Again, this is not a proposed schema.

The conceptual shift is more important than the representation:

> **A role manifest may describe the vantage a role provides rather than serving primarily as a manually authored list of decisions that role owns.**

Decision authority could then be projected from the relationship between decision requirements and role vantage.

---

# 5. Deterministic authority projection

Where the inputs are fully declared, authority assignment may become mechanically derivable.

```text
decision requirements
        +
available logical role vantages
        +
authority ceilings
        +
independence constraints
        +
delegation rules
        |
        v
eligibility projection
```

Possible outcomes:

```text
exactly one eligible role
    -> authority can be projected deterministically

multiple legitimate eligible roles
    -> unresolved organizational / semantic choice

no eligible role
    -> organizational gap

eligible role exceeds upstream authority ceiling
    -> forbidden projection

required independence conflicts with role topology
    -> topology invalid for that decision
```

The deterministic machinery does not create semantic authority.

It proves that an already-authorized logical role satisfies the declared requirements for exercising it.

---

# 6. Authority projection must not mint authority

This distinction is critical.

The projection mechanism cannot turn:

```text
role has useful context
```

into:

```text
role has authority
```

without an upstream authority source permitting that consequence.

The likely shape is:

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

Therefore:

> **Vantage determines where authority may be exercised; it does not determine how much authority exists.**

A role can be ideally positioned to make a decision and still lack authorization to make it.

---

# 7. Delegation is part of the decision model

Not all semantic authority should be transferable in the same way.

Candidate authority modes may include:

### Non-transferable

The designated vantage must exercise the semantic judgment itself.

Example shape:

```text
authorized semantic refresh owner
    -> determine whether evidence changes claim meaning
```

An observer or storage service cannot inherit that judgment.

### Delegable

The owner may delegate the complete bounded judgment to another role satisfying declared requirements.

### Nomination-only

One role may nominate a consequence while another role admits it.

For example:

```text
role A
    nominates specialist need

role B / host
    verifies independence and eligibility

owning authority
    admits final composition
```

### Advisory

A role may produce a recommendation without gaining authority over the consequence.

The authority projection model therefore needs to distinguish:

```text
ability to observe
ability to recommend
ability to nominate
ability to decide
ability to admit
ability to execute
```

These are not interchangeable permissions.

---

# 8. Compound decisions may decompose into multiple authority surfaces

A useful consequence of this model is that concepts currently described as one decision may turn out to contain several differently owned decisions.

For example, "select the review panel" may actually contain:

```text
implementation role
    judges:
        what technical risks need challenge?
        what specialist expertise is relevant?

review / control boundary
    judges mechanically or semantically:
        does candidate reviewer satisfy independence?
        is required profile available?
        is provider / realization admissible?

owning authority
    decides:
        whether exceptional policy deviation is allowed
```

The admitted review composition becomes a consequence of several separately owned transitions.

This follows a broader Work Engine principle:

> Shared subject matter does not imply shared ownership, and adjacent transitions are not automatically one transition.

Authority projection should preserve rather than erase those distinctions.

---

# 9. Implication for manifests

The current manifest direction may be describing several things at once:

- role identity;
- authority;
- capabilities;
- state visibility;
- relationships;
- workflow position;
- runtime constraints.

The authority-projection idea suggests these concerns may eventually separate more sharply.

One possible conceptual decomposition is:

## Decision / authority declarations

Describe:

- semantic decision types;
- required vantage;
- consequence ownership;
- independence;
- delegation;
- authority ceiling;
- admissibility conditions.

## Role / vantage declarations

Describe:

- evidence visible;
- state visible;
- semantic ownership;
- continuity;
- capabilities;
- effect ceilings;
- independence relations;
- prohibited positions.

## Organizational composition

Determines:

- which logical role instances exist;
- which required vantage each provides;
- authority projection;
- collaboration and information flow;
- unresolved organizational gaps.

## Runtime realization

Determines:

- model;
- provider;
- harness;
- tools;
- sandbox;
- concrete capability grants;
- runtime identity.

This decomposition is only a candidate.

Current manifest ownership and schema boundaries must be inspected before any change is proposed.

---

# 10. Implication for workflow design

The deeper consequence may be that workflow topology itself is not primary.

A conventional workflow starts with roles and stages:

```text
idea
    -> proposal
    -> planning
    -> supervisor
    -> builder
    -> review
```

But those stages encode underlying semantic dependencies.

A more fundamental representation may look like:

```text
decision A
    requires evidence X

decision B
    requires admitted consequence of A

decision C
    requires independence from owner of B

decision D
    requires human authority

decision E
    requires continuity across B and D
```

From those requirements, Work Engine could potentially derive:

```text
semantic dependency graph
        |
        v
required vantage points
        |
        v
logical role topology
        |
        v
authority projection
        |
        v
workflow / execution topology
```

The workflow would then become an executable projection of deeper semantic and authority structure rather than the primary definition of that structure.

This remains an architectural hypothesis.

Current high-value workflows may still encode useful accumulated experience that should not be discarded merely because a more general compiler becomes possible.

---

# 11. Roles may become partially emergent

If organizational compilation follows decision requirements, logical role boundaries themselves may become partly derivable.

Suppose two decisions require:

```text
same evidence
same context continuity
same authority ceiling
same ownership
compatible effect boundaries
no independence between them
```

They may be safely colocated in one logical role.

Conversely:

```text
decision A:
    requires retained implementation context

decision B:
    requires independence from implementation context
```

Those decisions cannot truthfully inhabit the same logical role realization.

Therefore the organizational compiler may eventually answer:

> What is the smallest role topology that satisfies every required semantic vantage, authority, independence, and continuity constraint?

This is not the same as minimizing agent count.

The correct topology may deliberately contain redundant or separated roles when independence is itself part of the required evidence.

---

# 12. Relationship to the ExecutionEnvelope direction

This idea appears closely related to the emerging ExecutionEnvelope architecture.

An ExecutionEnvelope is expected to represent a problem-specific organization with explicit:

- ownership;
- delegation;
- collaboration;
- observation;
- mutation;
- lifecycle; and
- information flow.

Deterministic authority projection may become one input to producing such an envelope.

Possible future composition:

```text
problem specification
        +
semantic decision requirements
        +
role templates / possible vantage points
        +
system invariants
        +
human authority
        +
available capabilities
        |
        v
organizational compilation
        |
        v
logical role topology
        +
authority projection
        +
information / effect boundaries
        |
        v
ExecutionEnvelope
```

This document does not decide whether authority projection belongs inside the ExecutionEnvelope compiler, feeds it as a separate compiler stage, or is represented by another architectural owner.

That is a seam for reconciliation.

---

# 13. Domain transferability

The authority-projection machinery appears independent of software-development semantics.

The domain supplies:

- decision meanings;
- evidence types;
- semantic ownership;
- consequences;
- domain-specific services;
- authority ceilings.

The generic organizational layer supplies:

- vantage matching;
- independence constraints;
- continuity constraints;
- delegation semantics;
- authority projection;
- role composition;
- unresolved-gap detection.

For example:

```text
LEGAL

research sufficiency
    -> research vantage

argument strategy
    -> case-context vantage

settlement authority
    -> client / delegated authority
```

```text
RESEARCH

statistical validity
    -> data + methodology vantage
    -> possible independence constraint

scientific interpretation
    -> domain-expert vantage

publication decision
    -> publication authority
```

```text
OPERATIONS

incident diagnosis
    -> live-system evidence vantage

risk acceptance
    -> business / safety authority

execution
    -> operational capability + bounded effect authority
```

The generic authority model does not need to understand law, research, medicine, software, or operations.

It needs to understand declared relationships between:

```text
decision
vantage
authority
ownership
independence
continuity
delegation
effect
```

---

# 14. Candidate generative principle

A compact form of the idea is:

> **Place each semantic decision with the narrowest authorized logical role whose vantage satisfies the evidence, ownership, continuity, independence, and consequence requirements of that decision.**

And:

> **When those requirements and role properties fully determine the owner, project that authority mechanically. When they do not, preserve the unresolved organizational decision rather than hiding it in workflow procedure.**

---

# 15. Important distinctions

This idea depends on preserving several distinctions.

```text
capability
    != authority

visibility
    != ownership

ownership
    != authority over every related consequence

organizational hierarchy
    != semantic authority

same actor
    != same decision authority

delegation
    != authority transfer by default

nomination
    != admission

admission
    != execution

runtime realization
    != logical role

workflow position
    != semantic ownership

mechanical eligibility
    != semantic fitness when semantic judgment remains
```

Any implementation that collapses these would defeat the purpose of the model.

---

# 16. Open seams

The following should remain unresolved until reconciled against current architecture.

## 16.1 Authority source vs authority projection

What durable object grants the authority ceiling from which projected role authority is derived?

How is projection prevented from minting authority?

## 16.2 Decision identity

Where are semantic decision types defined?

Are they domain-owned contracts, workflow artifacts, plan-IR structures, role contracts, or another object?

## 16.3 Role manifest boundary

Does the existing manifest already contain sufficient vantage structure?

Would this refine its interpretation or require a new source object?

## 16.4 ExecutionEnvelope relationship

Is authority projection:

- part of organizational compilation;
- a prerequisite to it;
- a projection over an already-formed envelope; or
- a separate concern?

## 16.5 Workflow relationship

Are workflows authored structures constrained by authority projection, or can some workflow topology itself eventually be compiled from semantic dependencies?

## 16.6 Dynamic vantage

Can authority projection change during execution as evidence, context, or role availability changes?

If so, which decisions can move and which are continuity-bound?

## 16.7 Non-transferability

How is genuinely non-transferable semantic authority represented?

Does runtime replacement preserve logical-role authority without transferring it to another semantic role?

## 16.8 Delegation

What distinctions are required among:

- delegate;
- nominate;
- advise;
- admit;
- execute?

## 16.9 Independence

How is required independence expressed without making runtime topology itself semantically authoritative?

## 16.10 Multiple eligible roles

When several roles satisfy the same vantage requirements, is selection:

- deterministic policy;
- cost optimization;
- capability resolution;
- organizational judgment; or
- domain-specific?

## 16.11 No eligible role

Does an unsatisfied vantage requirement become an explicit organizational compilation failure?

Can the compiler propose a missing role without acquiring authority to create one?

## 16.12 Role merging and splitting

Under what conditions may two decision surfaces share one logical role?

Which incompatibilities force distinct roles?

## 16.13 Human roles

Are human decision owners represented through the same vantage grammar, a separate authority layer, or both?

## 16.14 Research and capability learning

Can execution evidence determine that a previously semantic authority requirement can safely become deterministic?

If so, how is that transition proposed and authorized without letting the measurement system rewrite its own authority model?

---

# 17. Non-goals

This idea does not currently propose:

- replacing current role manifests;
- replacing workflows;
- generating arbitrary organizations automatically;
- assigning semantic authority using model confidence scores;
- inferring authority from observed behavior;
- treating available capabilities as authority;
- allowing runtime models to expand their own authority;
- making every decision dynamically routed;
- eliminating durable named roles;
- collapsing human authority into role eligibility;
- turning organizational design into an optimization problem before semantic correctness is established; or
- implementing a universal authority language before the recurring structure has been reconciled against current Work Engine artifacts.

---

# 18. Questions for the first reconciliation pass

The next investigation should be evidence-first.

1. Where does current Work Engine already assign semantic authority because of role-local vantage, even if it does not use that terminology?

2. Which assignments are manually encoded in:
   - role manifests;
   - role instructions;
   - workflow logic;
   - service contracts;
   - supervisor code;
   - proposal / planning architecture?

3. Which current authorities are genuinely static role properties, and which are consequences of a narrower decision requirement?

4. Does the existing role manifest already contain enough information to derive any authority assignments mechanically?

5. Where are capability, authority, ownership, visibility, and effect currently conflated?

6. Which current compound responsibilities decompose into separately owned semantic decisions?

7. Does the ExecutionEnvelope proposal already imply an authority-projection phase?

8. Are there existing examples where one logical actor occupies multiple authority surfaces without those surfaces becoming the same authority?

9. Which current authorities are explicitly non-transferable?

10. Which current role boundaries exist because of independence rather than specialization?

11. What is the smallest real current workflow from which a deterministic authority projection could be reconstructed without changing behavior?

12. Can the proposed abstraction be demonstrated across one non-software domain without adding domain-specific rules to the projection mechanism?

---

# 19. Working hypothesis

The current working hypothesis is:

> Work Engine roles are not fundamentally job titles with attached permissions. They are logical vantage points from which bounded semantic authority can be exercised.

If that hypothesis survives reconciliation, then organizational compilation may eventually become:

```text
work to be judged
        |
        v
semantic decision surfaces
        |
        v
required legitimate vantage points
        |
        v
role decomposition / composition
        |
        v
bounded authority projection
        |
        v
workflow and ExecutionEnvelope
        |
        v
runtime realizations
```

The resulting organization would not be compiled primarily around agent identities.

It would be compiled around the semantic conditions required for trustworthy judgment.

A possible long-term formulation is:

> **Given a body of work and its semantic, evidentiary, authority, continuity, and independence requirements, compile the smallest truthful organization capable of judging it correctly.**

Whether Work Engine should actually reach that destination remains open.

The immediate purpose of this idea is narrower: determine whether the repeated role-authority patterns already present in Work Engine are instances of a common, domain-transferable authority-projection model.
