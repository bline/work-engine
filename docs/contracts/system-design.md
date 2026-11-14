# System design

Authority: Binding Work Engine doctrine; part of [DESIGN.md](../../DESIGN.md).
Scope: the product contracts and design rules stated below.

Load when: Designing architecture, forming plans that choose owners or boundaries, or changing product structure.

Section numbers retain their original citation identities. Links identify
related topics; they do not require reading every linked document.

### 1.2 Consequences

Consequences describe what successful work must accomplish or what a decision must preserve, without prescribing the method used to accomplish it. Prefer terminal or externally observable properties of success and failure over intermediate activities. An activity belongs here only when performing it is itself causally necessary to establish the protected outcome; when possible, state that outcome instead.

Examples:

- a placement decision is supported by enough evidence to be defensible;
- an architectural boundary does not ignore a credible competing owner;
- review evidence retains the independence required by the correctness claim;
- useful context is not reconstructed unnecessarily;
- the intended runtime or operator-visible behavior is demonstrated;
- direct-source observation does not expand beyond evidence relevant to the claim;
- evidence gathering is sufficient for the decision without continuing after additional evidence has no credible decision value.

When a requirement is not truly invariant, prefer expressing its **consequence** rather than converting the current best method into a rule. A consequence must not become a procedure merely by being phrased as a desired result.

---

### 1.3 Capability affordances

Capabilities describe what tools, models, agents, and deterministic machinery are good at. Capability descriptions inform judgment; they do not normally route it.

For example:

- Codebase Memory is strong at structural relationships, symbols, ownership, callers, dependencies, and repository navigation.
- Direct filesystem observation is strong at exact literals, non-code artifacts, unindexed material, host state, and direct verification of source bytes.
- Chrome Vision is strong at rendered-state and interaction evidence.
- Deterministic tests and gates are strong at executable checks with mechanically decidable outcomes.
- Independent reviewers are strong at reducing correlated reasoning error for consequential semantic judgments.

Making a capability available does not require its use. Using one capability rather than another is not itself a fallback.

---

### 1.4 Procedures and default routes

Procedures are accumulated experience about routes that have often worked. They may be valuable, especially as high-level operating shapes, but they are subordinate to contracts and evidence.

A procedure should be treated as a default hypothesis unless the procedure itself is part of an explicit contract.

The model may alter, skip, reorder, combine, or replace non-binding procedural steps when its judgment indicates that another route better achieves the required consequences while preserving all invariants.

Procedures should therefore have **escape by judgment**, not exhaustive exception tables.

---

### 1.5 Outcome-derived design

The hierarchy above can be applied as one compact design method:

> **Define the space, not the solution. Expose the machine, not the route.**

Define the desired and invalid outcomes, authority boundaries, and observable
evidence without embedding a preferred solution. Derive only the structure
required to preserve those distinctions, then expose enough composable
machinery to make valid outcomes reachable and observable. Leave route,
sequence, trigger mechanisms, and tuning to judgment unless causality makes
them part of the contract.

"Complete" means complete enough at the owning contract boundary to distinguish
valid from invalid states. It does not mean that every future situation or
implementation is known. Likewise, the machinery should be sufficient for the
required outcome space, not an exhaustive inventory of every possible tool.

The resulting layers are:

```text
outcomes    → the desired and invalid observable states
structure   → the durable identities, relationships, and boundaries required
mechanics   → ways to realize, enforce, observe, and change that structure
knobs       → bounded adaptation of mechanics that preserves every invariant
projections → consumer views of the owned semantic structure
```

Mechanics and knobs must not silently redefine the outcome space. Retention,
visibility, authority, deletion, and similar choices are ordinary knobs only
inside ranges that preserve the owning contract; outside those ranges they are
contract changes.

This method gives four useful design tests:

- If a requirement can be violated while every protected outcome remains valid,
  it is probably procedure rather than invariant structure.
- If removing a field or relationship makes valid and invalid states
  indistinguishable, it is probably required semantic structure.
- If a capability does not make a meaningful valid outcome reachable or
  observable, it may be redundant machinery.
- If a trigger or sequence can be replaced without changing correctness, it is
  a mechanism rather than doctrine. The transition condition and any timing
  required for correctness may still be binding.

---

### 2.2 Variant structure

Variant structure describes the machinery that currently exists and why it is shaped that way: available capabilities, agent topology, lifetimes, provider arrangements, component boundaries, infrastructure, controls, and other operational mechanisms that may evolve over time.

Variant structure is **descriptive, not imperative**.

Describe:

- what machinery exists;
- what state it exposes;
- what controls or capabilities can change that state;
- what each part affords;
- what design pressure or tradeoff caused it to exist;
- what consequences follow from its current shape;
- what limitations or boundaries matter.

Do not convert the existence of machinery into an interaction procedure.

For example, saying that Codebase Memory is optimized for structural repository understanding while filesystem tools provide direct-source observation describes variant structure. Saying that one must always be used before the other turns current machinery into policy.

Variant structure should let the model understand and manipulate the machine it is operating inside without dictating the sequence of manipulation.

The action space should be deliberately bounded:

> **Provide enough composable machinery to make intended outcomes reachable, but do not expose redundant or irrelevant degrees of freedom that only enlarge the model's search space.**

A missing control can make a legitimate outcome unreachable. Redundant or overlapping controls can multiply equivalent paths and increase reasoning complexity without adding useful agency.

The three structural effects must remain distinct:

- **Invariant structure trims the boundary** by eliminating invalid states and transitions.
- **Variant structure enriches the valid interior** by exposing useful machinery, affordances, and reachable state transitions.
- **Procedures concentrate traversal** by biasing the model toward a particular route inside already-valid space.

The design goal is therefore:

> **Trim invalid space. Enrich valid space. Avoid unnecessary path concentration inside valid space.**

A procedure is justified only when concentrating traversal is itself required by a real contract or causal constraint. Otherwise, prefer describing the machinery, consequences, and affordances and leave route selection to model judgment.

A useful distinction is:

```text
invariant structure  → what must remain true
variant structure    → what currently exists and why
state / evidence     → what is true in this run
objective/consequence→ what outcome matters and what failure means
judgment             → what to do
```

Changing invariant structure is a product-contract change. Changing variant structure is an architecture or infrastructure change. Changing state/evidence is normal runtime evolution. Changing judgment is expected.

Variant structure therefore defines the model's available affordances: the controls and capabilities that can act on state, bounded by invariant structure and evaluated against the objective and consequences. State/evidence describes what those affordances currently observe or have produced.

---

## 9. Determinism and model judgment

Use deterministic machinery where the required operation is fully determined by known inputs and no contextual interpretation is needed.

Examples include schema validation, exact manifest checks, mechanically decidable tests, receipt consistency checks, and deterministic telemetry harvesting after identity and provenance contracts are defined.

Do not replace model judgment with deterministic machinery merely because a procedure can be written. A deterministic rule is appropriate when variation is a defect; it is harmful when variation is the mechanism by which the system adapts to evidence.

Conversely, do not spend model reasoning on a decision whose correct output is already completely determined by an invariant and available inputs.

---

## 13. Feature-design rules

Every new Work Engine feature should address the following concerns. Their order is a matter of judgment unless a causal dependency or owning contract requires sequencing:

### Identify the owner

Which component owns the contract and durable semantics?

### Identify true invariants

What must remain true regardless of implementation route?

### Identify required consequences

What outcome or evidence must exist for the feature to be successful?

### Expose capabilities and affordances

What can the model or deterministic machinery use, and what is each capability good at?

### Leave non-invariant choices to judgment

Do not prescribe sequencing, routing, escalation, context resets, evidence source, or decomposition unless changing that choice would violate an identified invariant.

### Define provenance

Which choices or transitions must be recorded because downstream validation, auditability, or contract ownership depends on them?

### Define deterministic checks where appropriate

Mechanically enforce actual invariants and schema contracts. Do not use validators to enforce stylistic conformity to a preferred reasoning route.

### Measure consequences

Compare routes by accepted outcome, correctness, repair rate, context occupancy, token/cost consumption, latency, operator attention, and maintenance burden. Metrics inform future judgment and design; they do not silently redefine success.

---

## 14. Anti-patterns

Be suspicious when a proposed feature introduces:

- a fixed tool ladder where several authorized capabilities could establish the claim;
- mandatory phase transitions whose only justification is historical habit;
- a large exception table intended to restore flexibility lost by a rule;
- a forced context restart when the existing context remains useful;
- a fresh reviewer for every remediation iteration without a new independence need;
- provider changes merely to gain an evidence capability the existing provider can safely expose;
- repeated reconstruction of evidence or context the system already possesses;
- preserving or rationalizing a conclusion after the premise supporting it has become false instead of marking the dependent conclusion stale;
- procedural validation of method rather than deterministic validation of an invariant;
- mandatory evidence gathering after the model already has sufficient support for the required consequence;
- model reasoning spent on fully determined mechanical work;
- infrastructure failures "fixed" by adding cognitive procedure to the model;
- a production-path claim whose only consumer is the constraint that requires
  it; or
- an actor narrowing an unestablished claim until its available evidence can
  support the substitute.

A useful warning sign is a rule whose exceptions keep growing. That usually means the rule flattened a decision that belongs in semantic space.

---

## 15. High-level procedures remain useful

Work Engine may retain high-level procedures such as direct versus falsified-placement routes, bounded review loops, or evidence/implementation/validation shapes when they encode useful accumulated experience.

Their role is to provide a strong default, shared vocabulary, and observable structure—not to remove model authority.

A high-level procedure is healthy when:

- its contracts and invariants are explicit;
- the consequence of each major stage is clear;
- the model may revise the route when evidence warrants it;
- revisions preserve required provenance;
- exceptions do not require an ever-growing procedural rulebook;
- acceptance depends on the required consequence, not ceremonial completion of every default step.

The goal is **structured judgment**, not procedural obedience.
