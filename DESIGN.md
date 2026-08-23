# Work Engine Design

## Purpose

Work Engine is an objective-driven engineering system in which capable models retain decision authority inside explicit contracts.

Its design goal is not to encode the best known procedure for engineering work. It is to make the objective, invariants, available capabilities, evidence, consequences, and authority boundaries clear enough that the model can choose an effective route for the actual situation.

The central design rule is:

> **Contracts constrain what must remain true. Models choose how to make it true.**

A procedure may describe a useful default route, but it is not the source of correctness unless a contract explicitly requires that procedure. Routes are hypotheses. They may be revised when evidence shows that another route better serves the objective while preserving every invariant.

---

## 1. Design hierarchy

Work Engine distinguishes four kinds of guidance. They are not interchangeable.

### 1.1 Contracts and invariants

Contracts define conditions that must remain true. They are binding.

Examples include:

- the user objective and configured acceptance conditions;
- user authority and approval boundaries;
- safety and mutation boundaries such as a read-only reviewer;
- ownership boundaries between supervisor, builder, evidence adapter, gate runner, and receipt schema;
- truthful preservation of unresolved state and user-authored work;
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

### 1.2 Consequences

Consequences describe what successful work must accomplish or what a decision must preserve, without prescribing the method used to accomplish it. Prefer terminal or externally observable properties of success and failure over intermediate activities. An activity belongs here only when performing it is itself causally necessary to establish the protected outcome; when possible, state that outcome instead.

Examples:

- a placement decision is supported by enough evidence to be defensible;
- an architectural boundary does not ignore a credible competing owner;
- review evidence retains the independence required by the correctness claim;
- useful context is not reconstructed unnecessarily;
- the intended runtime or user-visible behavior is demonstrated;
- direct-source observation does not expand beyond evidence relevant to the claim;
- evidence gathering is sufficient for the decision without continuing after additional evidence has no credible decision value.

When a requirement is not truly invariant, prefer expressing its **consequence** rather than converting the current best method into a rule. A consequence must not become a procedure merely by being phrased as a desired result.

### 1.3 Capability affordances

Capabilities describe what tools, models, agents, and deterministic machinery are good at. Capability descriptions inform judgment; they do not normally route it.

For example:

- Codebase Memory is strong at structural relationships, symbols, ownership, callers, dependencies, and repository navigation.
- Direct filesystem observation is strong at exact literals, non-code artifacts, unindexed material, host state, and direct verification of source bytes.
- Chrome Vision is strong at rendered-state and interaction evidence.
- Deterministic tests and gates are strong at executable checks with mechanically decidable outcomes.
- Independent reviewers are strong at reducing correlated reasoning error for consequential semantic judgments.

Making a capability available does not require its use. Using one capability rather than another is not itself a fallback.

### 1.4 Procedures and default routes

Procedures are accumulated experience about routes that have often worked. They may be valuable, especially as high-level operating shapes, but they are subordinate to contracts and evidence.

A procedure should be treated as a default hypothesis unless the procedure itself is part of an explicit contract.

The model may alter, skip, reorder, combine, or replace non-binding procedural steps when its judgment indicates that another route better achieves the required consequences while preserving all invariants.

Procedures should therefore have **escape by judgment**, not exhaustive exception tables.

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


## 2. Model-facing structure

Work Engine presents the model with several distinct kinds of information. Keeping them separate prevents current machinery or accumulated experience from silently becoming mandatory procedure.

### 2.1 Invariant structure

Invariant structure is the product's load-bearing structure: authority, ownership, interface contracts, mutation boundaries, provenance obligations, security constraints, and other properties that must remain true for the product to operate correctly.

Invariant structure may be expressed as commands because variation would break the product contract. Each command should be paired with the causal reason and failure mode that make the invariant necessary.

Invariant structure provides stable reference points for reasoning. Inferred understanding may shift as evidence changes; the structural anchors do not. The purpose is to localize uncertainty, not eliminate judgment.

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

### 3.3 Commands must be minimal

A command should state the smallest behavior necessary to preserve the protected structure.

Do not command a method when only an outcome is structurally required.

For example:

> **Reason:** Review evidence is used to reduce correlated builder error. If the reviewer can mutate the implementation, the claimed independence no longer holds.  
> **Command:** Reviewers used as independent evidence are read-only.

Do not expand this into a procedural ritual such as which files must be opened, which tool must be tried first, or how many review passes must occur unless those details are themselves contractually necessary.

### 3.4 Commands must not collapse semantic space unnecessarily

Strong imperatives create high-confidence policy attractors. That is useful when variation would break the product; it is harmful when contextual variation is exactly what enables good problem solving.

Prefer:

> **Reason:** An actual fallback without provenance leaves the audit trail incomplete and can make the receipt invalid.  
> **Command:** Record required fallback provenance when a fallback occurs.

over:

> Always use capability A first, then capability B, and record why B was needed.

The first preserves the structural contract while leaving capability choice to judgment. The second encodes a solution path and collapses alternatives that may be better in the current situation.

### 3.5 Sequencing is structural only when causality requires it

Commands may impose order when reversing the order would invalidate the result, destroy required evidence, cross an authority boundary, or otherwise violate a contract.

Otherwise, sequencing belongs to model judgment.

"Validate before publication because publication makes the artifact authoritative" may be structural.

"Search before reading because search is usually cheaper" is an affordance or default, not structure.

### 3.6 Exhaustive commands require genuinely closed worlds

Avoid imperative language such as:

> In these cases, always do X.

unless the set of cases is closed by an external specification or product contract.

Open-ended semantic domains should be described through consequences and affordances. Enumerating today's known cases and commanding behavior for them silently flattens future cases into an incomplete taxonomy.

### 3.7 Failure-mode explanations are knowledge, not escape hatches

Explaining why a command exists does not weaken the command.

The model remains bound by the structural invariant, but now possesses causal knowledge that can propagate through later reasoning. When a novel situation appears, the model can preserve the underlying property rather than blindly reproduce an old ritual. This preserves **logical continuity**: the command remains connected to the causal reason that makes it necessary, so that reason can continue to constrain later judgment.

The intended pattern is:

```text
causal reality
→ failure mode
→ structural invariant
→ minimal command
→ model judgment everywhere outside that boundary
```

Invariant structure should provide stable reference points without prescribing the route between them: **pin the coordinate system, not the path**.

The goal is to **remove invalid degrees of freedom while preserving every legitimate degree of freedom**.


### 3.8 Command review test

Before adding or retaining imperative language, ask all of the following:

1. What product property fails if this command is violated?
2. Would the resulting state be invalid regardless of which valid route produced it?
3. Is that property genuinely invariant, or merely desirable in the current situation?
4. Does the command encode only structure, authority, or an irreducible interface contract?
5. Has mutable data, current knowledge, a heuristic, or a preferred route leaked into the command?
6. Could a different method preserve every required consequence? If so, is the command unnecessarily prescribing that method?
7. Is any required sequencing actually causal?
8. Is the command paired with enough explanation for the model to understand why the boundary exists?
9. Would the command remain valid if tools, repository structure, models, or evidence changed while the product contract stayed the same?

If the answer reveals that the statement is not structural, convert it into a consequence, affordance, observation, or default route instead of a command.

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

---

## 6. Evidence and capability selection

Evidence capabilities should normally be available together when doing so does not violate an authority or safety boundary.

The model should understand the purpose, strengths, costs, and limitations of each capability and choose according to the claim being established.

Do not force a capability sequence merely because one route is usually preferred. Do not make the model restart a reasoning process merely because the most useful evidence source changes.

### Capability choice versus fallback

These are distinct events:

1. **Capability selection** — the model chooses the evidence source best suited to a claim. No fallback necessarily occurred.
2. **Evidence fallback** — an earlier evidence route was unavailable, ambiguous, insufficient, or failed, so another route was added or substituted. Required fallback provenance must be recorded.
3. **Provider/configuration fallback** — the configured provider itself changes. Required configuration/provider provenance must be recorded.

Evidence-mode fallback inside a configured provider does not silently rewrite provider identity.

Audit contracts may require facts about how evidence was obtained. Those facts remain mandatory even though the choice of evidence route belongs to model judgment.

---

## 7. Information lifetime is a decision dimension

Context is a resource with value and cost. Its lifetime should reflect how long the information remains useful to correct judgment. Preserve the consequence of reasoning, not necessarily the reasoning transcript.

Work Engine currently recognizes at least these semantic lifetimes:

### Supervisor lifetime

Keep durable campaign state, configuration, approvals, continuation state, and compact handoff information needed across slices.

### Builder lifetime

Preserve the builder's accumulating understanding for the bounded implementation task while that understanding continues to improve decisions.

### Investigation lifetime

Use disposable reconnaissance or diagnostic contexts when they need large temporary evidence that should not occupy the builder's durable context. Return only the conclusions, evidence references, uncertainties, and artifacts needed downstream.

### Reviewer lifetime

Begin a reviewer fresh when independence from the builder is part of the evidence claim. Once that independence has been established, preserve the same isolated reviewer through a bounded remediation loop while its accumulated understanding remains useful.

A typical useful shape is:

```text
fresh isolated reviewer
→ initial review
→ findings
→ builder repair
→ same reviewer evaluates the delta and prior findings
→ repeat as useful
→ acceptance
→ discard reviewer
```

Restarting the reviewer is a judgment call unless a contract requires freshness again. Reasons may include a material architectural or placement change, a changed review premise, degraded or oversized context, or a need for a genuinely new independent perspective. These are signals for judgment, not a deterministic reset table.

Instruction freshness and reasoning-context lifetime are separate concerns. When instructions or skills change, refresh them in place when the runtime permits and the existing reasoning context remains valuable; do not destroy useful context merely to refresh instructions.

---

## 8. Independence is a property, not ritualized amnesia

Independent reasoning is valuable when correlated error is consequential. The required property is meaningful independence from the reasoning being challenged.

Freshness at the start of an independent review can establish that property. Repeatedly discarding the reviewer's own accumulated understanding during remediation does not inherently create more independence and may instead create repeated cost and inconsistent reconstruction.

When a placement certificate, adversarial semantic review, or other contract explicitly requires fresh independent falsification, preserve that requirement. Do not generalize it into compulsory freshness for unrelated retrieval, deterministic validation, or every repair iteration.

---

## 9. Determinism and model judgment

Use deterministic machinery where the required operation is fully determined by known inputs and no contextual interpretation is needed.

Examples include schema validation, exact manifest checks, mechanically decidable tests, receipt consistency checks, and deterministic telemetry harvesting after identity and provenance contracts are defined.

Do not replace model judgment with deterministic machinery merely because a procedure can be written. A deterministic rule is appropriate when variation is a defect; it is harmful when variation is the mechanism by which the system adapts to evidence.

Conversely, do not spend model reasoning on a decision whose correct output is already completely determined by an invariant and available inputs.

---

## 10. Route revision

Routes are hypotheses. Evidence may falsify them.

When a premise fails, the model should:

- preserve observations and decisions that remain valid;
- identify conclusions that depended on the failed premise;
- revise only what the new evidence makes stale;
- choose a replacement route using current evidence and capabilities;
- preserve required fallback, configuration, and decision provenance;
- continue toward the original objective unless authority or an invariant requires user intervention.

When an attempted route fails, retain the smallest durable consequence needed to prevent unnecessary reconsideration: what was tried, what premise justified trying it, and what observation invalidated it. Preserve the consequence of the reasoning, not the reasoning transcript. The failed attempt should become evidence about the route rather than inert history.

A failed procedure is not automatically a failed objective.

Do not encode exhaustive recovery recipes for every anticipated failure. State the invariant and required consequence of recovery, expose suitable capabilities, and preserve provenance for what actually happened.

---

## 11. User authority and approval

Model-centered decision making does not weaken user authority.

The model may exercise judgment only within the authority it has been given. Approval boundaries in configuration or owning contracts remain invariant.

The system should distinguish:

- decisions the model is authorized to make silently;
- consequential decisions it may make but should explain;
- decisions that require new user authority, preference, ownership choice, or approval.

Do not manufacture approval requirements for ordinary capability selection when the capability is already authorized. Do not bypass a real approval boundary merely because the model believes the action is beneficial.

---

## 12. Provenance and receipts

Flexibility increases the importance of truthful provenance; it does not reduce it.

The audit record must describe what actually happened according to the owning receipt/schema contracts, including required route revisions, fallback transitions, provider/configuration changes, evidence modes, validation breadth, unavailable measurements, and unresolved concerns.

Important distinctions include:

- available capability versus capability actually used;
- capability selection versus fallback;
- evidence-mode fallback versus provider change;
- observed measurement versus inferred or unavailable measurement;
- current route versus historical route revision.

Never infer zero from unavailable evidence. Never rewrite provider identity because an already-configured provider used another authorized evidence capability. Never suppress a required fallback event because the final route succeeded.

The receipt records the path taken; it does not prescribe the path that must be taken next time.

For consequential semantic decisions, preserve only the decision support required by the owning artifact or contract: the accepted claim, decisive evidence, material uncertainty, and any invalidation condition that matters downstream. This is a decision receipt, not a reasoning transcript. When an existing certificate or receipt already carries that information, extend or generalize that artifact rather than inventing a parallel rationale schema.

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

Compare routes by accepted outcome, correctness, repair rate, context occupancy, token/cost consumption, latency, user attention, and maintenance burden. Metrics inform future judgment and design; they do not silently redefine success.

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
- infrastructure failures "fixed" by adding cognitive procedure to the model.

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

---

## 16. Governing principles

When design choices conflict, prefer the design that best preserves these principles:

### Truth

The system's state, evidence, confidence, provenance, and unresolved conditions must remain faithful to reality.

### Objective fidelity

Local convenience must not replace the requested downstream consequence.

### User authority

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
