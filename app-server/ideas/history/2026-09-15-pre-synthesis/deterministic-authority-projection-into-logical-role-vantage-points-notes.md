# Session Synthesis: From Role Decomposition to Dynamic Context Topology

**Status:** Exploratory architecture synthesis
**Scope:** Role decomposition, authority projection, dynamic role composition, vantage formation, contract compilation, ExecutionEnvelope evolution, and adaptive context management

---

## 1. Starting point: role authority follows semantic vantage

The session began from an existing Work Engine property:

```text
Idea
  ↓
Proposal
  ↓
Planning
  ↓
Slice Supervisor
  ↓
Slice Builder
```

Within this hierarchy, authority is not simply inherited downward or concentrated upward.

Different roles own different semantic decisions because their position gives them the best legitimate **vantage** from which to make those decisions.

This suggested the principle:

> **Place each semantic decision with the narrowest authorized role whose vantage satisfies the evidence, ownership, continuity, independence, and consequence requirements of that decision.**

Hierarchy does not imply semantic authority.

```text
supervisor above builder
    ≠ supervisor owns builder judgment

reviewer sees implementation
    ≠ reviewer owns remediation

observer sees change
    ≠ observer owns semantic consequence
```

We called this **vantage-aligned semantic authority**.

---

# 2. First abstraction jump: authority may be projected rather than hard-coded

Initially, role authority appears as something like:

```text
Builder:
    may decide X
    may mutate Y
    may not decide Z
```

We realized that some of these assignments may instead be consequences of more primitive facts.

A decision may declare requirements such as:

```text
required evidence
required continuity
semantic ownership
independence
effect boundary
authority ceiling
delegability
subject scope
```

A logical role supplies properties such as:

```text
observed state
retained context
owned state
permitted effects
capabilities
independence
authority ceiling
continuity
```

Authority can then potentially be projected mechanically:

```text
decision requirements
        +
logical role vantage
        +
authority source
        +
independence/delegation constraints
        ↓
bounded authority projection
```

The role is not the source of authority.

The underlying semantic decision and upstream authority source are.

The role is the place where that authority can legitimately be exercised.

A critical invariant emerged:

> **Vantage determines where authority may be exercised. It does not determine how much authority exists.**

The projection mechanism must never mint authority.

---

# 3. Role manifests begin to change meaning

This shifted how we interpreted role manifests.

Rather than primarily declaring:

> This role owns decisions A, B, and C.

A role manifest might increasingly describe:

> This is the semantic vantage this kind of role can truthfully occupy.

For example:

```text
Builder-like vantage:
    retained implementation continuity
    repository evidence
    implementation ownership
    bounded mutation

Reviewer-like vantage:
    independent reasoning context
    immutable subject access
    attributed findings
    no implementation mutation
```

Likewise, decision definitions might describe what kind of vantage they require.

The assignment between them can then become partly compiled.

This makes the authority model domain transferable.

Nothing fundamental about:

```text
evidence
ownership
continuity
independence
delegation
effects
authority
```

is specific to software engineering.

---

# 4. Second abstraction jump: roles themselves may be compositions

The discussion then moved beyond authority.

If authority can be derived from semantic requirements, perhaps the **role contract itself** can be derived to a meaningful degree.

A role began to look less like an atomic entity and more like a composition:

```text
ROLE
≈
vantage
+ authority
+ obligation
+ continuity
+ information access
+ effect boundary
+ capability requirements
+ independence constraints
+ lifecycle
```

This suggests that the more fundamental objects may be primitives such as:

```text
DecisionSurface
AuthorityGrant
VantageRequirement
EvidenceRequirement
EffectBoundary
ContinuityRequirement
IndependenceConstraint
Obligation
CapabilityRequirement
SubjectScope
TerminalConsequence
```

A familiar role such as `Builder` may simply be a stable recurring composition of those primitives.

This reverses the conventional relationship:

```text
OLD

Builder
    ↓
properties
```

toward:

```text
POSSIBLE FUTURE

properties required by this work
        ↓
composition
        ↓
builder-like logical role
```

Reusable roles therefore need not disappear.

They could become tested **profiles or macros over role primitives**.

---

# 5. Role contract compilation

This led to a stronger possibility.

Given sufficiently structured semantic inputs, the role contract itself may be largely deterministic.

Conceptually:

```text
semantic obligation
        +
required vantage
        +
authority grant
        +
independence constraints
        +
available capabilities
        +
current ExecutionEnvelope
        +
system invariants
        ↓
ROLE-CONTRACT COMPILER
        ↓
logical role instance contract
```

The compiler might derive:

```text
what the role must observe
what state it must retain
what authority it receives
what effects it may perform
what capabilities it requires
what it must produce
what it may not do
how long it must exist
which role relationships must hold
```

The important boundary is:

> **Do not derive semantic meaning. Derive the contract consequences of semantic meaning that has already been established.**

This produces three useful layers:

```text
SEMANTIC CONTRACT
What judgment or consequence exists,
and what makes it legitimate?

        ↓

LOGICAL ROLE CONTRACT
What must this particular logical role
observe, own, decide, produce, and avoid?

        ↓

RUNTIME CONTRACT
What concrete model/provider/tools/harness
can realize that role now?
```

---

# 6. ExecutionEnvelope becomes a compiled organization

This changed the interpretation of `ExecutionEnvelope`.

Rather than simply being an authored description of:

> Who exists and how they relate,

it could eventually become the compiled result of semantic work structure.

```text
incoming work
        ↓
semantic decisions / obligations
        ↓
dependencies
        ↓
required vantage points
        ↓
authority / independence / continuity constraints
        ↓
role composition
        ↓
role contracts
        ↓
ExecutionEnvelope
```

The envelope becomes a **compiled executable organization**.

This does not require arbitrary role invention.

A nearer-term architecture could remain conservative:

```text
durable reusable role/vantage profiles
        +
problem-specific semantic structure
        ↓
dynamic role instances and topology
```

If no existing role profile can satisfy a required vantage:

```text
required vantage
        ↓
no valid composition
        ↓
ORGANIZATIONAL GAP
```

The system should surface the gap rather than invent a role silently.

---

# 7. Dynamic role creation and transferable authority

We then considered runtime organizational evolution.

An existing role may discover a semantic obligation that deserves another logical vantage.

Rather than merely "spawning an agent," the role could request:

```text
new logical role

subject:
    S

semantic obligation:
    D

delegated authority:
    A

required consequence:
    C
```

Work Engine could then derive the child contract.

Authority movement itself decomposes into different operations:

```text
delegation
    parent retains authority,
    child may exercise bounded subset

transfer
    authority moves from one owner to another

attenuation
    child receives a strictly narrower grant

nomination
    child or parent proposes a consequence,
    another authority admits it
```

A core invariant emerged:

```text
child authority
    ⊆ delegable parent authority
```

New logical roles may appear dynamically.

**New authority may not.**

Authority must trace to an existing legitimate source.

---

# 8. Why create another vantage?

At this point we recognized that dynamic role formation cannot be justified merely by parallelism.

Creating a new vantage is a costly semantic intervention.

Several legitimate pressures can justify it.

## Epistemic separation

A judgment requires independence from another reasoning context.

Example:

```text
implementation author
    ≠
independent reviewer
```

## Context separation

A subproblem requires large temporary evidence that should not occupy durable parent context.

```text
large temporary investigation
        ↓
separate context
        ↓
small durable consequence
```

## Authority/effect separation

Two powers should not coexist in one role.

```text
produce evidence
    ≠
own semantic consequence
```

## Continuity separation

A subproblem needs a distinct lifetime or retained history.

## Work decomposition

A coherent body of work can progress independently and need not remain inside one reasoning loop.

---

# 9. When another vantage is harmful

Every split creates a semantic boundary.

That boundary has costs:

```text
lost tacit shared context
handoff reconstruction
coordination
state synchronization
authority complexity
additional runtime cost
```

A split is likely poor when:

* the child needs nearly all parent context;
* no compact consequence can cross the boundary;
* parent and child must constantly negotiate the same mutable state;
* the proposed child has no distinct epistemic, authority, context, or continuity property;
* authority cannot lawfully move;
* the task is too small to amortize composition and coordination cost.

This led to a powerful general criterion:

> **Create another vantage when a stable semantic boundary allows the resulting contexts to know materially less than the original combined context while preserving or improving the required consequence.**

Good decomposition:

```text
before:
    A + B + all shared reasoning

after:
    role A knows A + contract(B)
    role B knows B + contract(A)
```

Bad decomposition:

```text
A constantly reconstructs B
B constantly reconstructs A
```

The latter duplicates coupling instead of removing it.

---

# 10. Vantage separation can be partially deterministic

The decision to separate can itself be decomposed.

Instead of one opaque "should I spawn another role?" judgment, Work Engine may classify the situation.

```text
MUST SEPARATE

MUST REMAIN

SEPARATION ELIGIBLE

SEMANTIC JUDGMENT REQUIRED
```

Hard constraints can sometimes determine the answer.

For example:

```text
independence required
+
current role authored challenged subject
    → separation required
```

or:

```text
authority is non-transferable
+
same logical owner required
    → semantic decision cannot move
```

For optional cases, deterministic machinery can supply evidence:

```text
context pressure
shared-state coupling
authority pressure
independence pressure
continuity divergence
subproblem boundary strength
projection cost
coordination cost
capability availability
parallelism opportunity
```

The machinery establishes facts.

The model judges the unresolved consequence.

This follows the recurring Work Engine pattern:

> **Use deterministic machinery to establish everything mechanically knowable, then spend inference only on the remaining semantic question.**

---

# 11. Event-scoped organizational reasoning

A further implication emerged.

Normal roles should not carry all of this organizational machinery permanently in context.

Most of the time a builder does not need to reason about:

* role decomposition;
* vantage separation;
* authority delegation;
* organizational topology;
* role contract generation.

That knowledge is cognitive overhead.

Instead:

```text
ordinary role context
        ↓
host observes organizational-decision evidence
        ↓
organizational decision admitted
        ↓
temporary projection injected:
    current metrics
    applicable constraints
    decision-specific skill
    available organizational actions
        ↓
model resolves bounded decision
        ↓
consequence persisted
        ↓
temporary projection removed
```

Organizational reasoning therefore becomes an **event-scoped capability**.

The role does not continually ask:

> Should I create another agent?

Externally owned evidence determines when the question is material enough to enter context.

A role may nominate such a condition, but nomination is not unilateral topology authority.

---

# 12. Third abstraction jump: this is really context management

The final step took the idea beyond dynamic organizations.

Work Engine already has a context-lifecycle manager.

Its current conceptual problem is primarily temporal:

```text
same logical role
same semantic vantage

context epoch N
    ↓
pressure/economics
    ↓
checkpoint state
    ↓
replace context
    ↓
context epoch N+1
```

It asks:

> **When should this same vantage receive a fresh context?**

The new architecture asks:

> **Is this still the correct context topology for the judgment that now exists?**

That introduces a second dimension.

```text
TEMPORAL CONTEXT MANAGEMENT

When should this context be replaced?

        +

TOPOLOGICAL CONTEXT MANAGEMENT

What contexts/vantages should exist at all?
```

---

# 13. Context pressure becomes context fitness

Token volume becomes only one type of context pressure.

Potential pressures include:

```text
volume pressure
    too much context

relevance pressure
    retained information no longer matters

semantic-width pressure
    too many unrelated decisions coexist

independence pressure
    a judgment must not inherit some reasoning

authority pressure
    incompatible authority surfaces coexist

continuity pressure
    part of the work needs another lifetime

instruction pressure
    dormant reasoning frameworks remain loaded

coupling pressure
    a proposed split would create excessive synchronization
```

This suggests a more general concept:

> **context fitness**

The question is no longer simply whether context is full.

It is whether the current projection remains a good environment for the judgment being performed.

---

# 14. Context transitions become richer

The context manager may eventually reason about more than replacement.

Possible transitions include:

```text
continue unchanged

temporarily augment
    inject event-specific skill/state

contract
    remove no-longer-useful projected material

replace
    same vantage, new context epoch

open disposable context
    separate temporary evidence lifetime

split vantage
    distinct reasoning conditions

instantiate role
    distinct vantage + semantic authority

retire vantage
    semantic obligation completed
```

These should not yet be frozen into an API.

The important discovery is that they are distinct operations.

---

# 15. Three increasingly strong boundaries

We distinguished three related concepts.

## Context boundary

Separate information lifetime/context.

```text
builder
↔ reconnaissance context
```

No independent authority is required.

## Vantage boundary

Separate epistemic or reasoning conditions.

```text
builder
↔ independent reviewer
```

## Role boundary

Separate semantic authority and obligations.

```text
parent role
↔ delegated child role
```

The relationship is roughly:

```text
role boundary
    generally implies distinct vantage

distinct vantage
    generally requires some context boundary

context boundary
    does not imply a new role
```

This prevents context optimization from automatically becoming organizational proliferation.

---

# 16. Adaptive projection planning

The larger architecture now appears to be:

```text
DURABLE WORK ENGINE WORLD
        ↓
PROJECTION PLANNING
        ↓
+-----------------------------+
| vantage topology            |
| who should reason?          |
|                             |
| information projection      |
| what should they know?      |
|                             |
| reasoning projection        |
| what temporary skill or     |
| decision framework applies? |
+-----------------------------+
        ↓
CONTEXT LIFECYCLE
when should this projection
be regenerated?
        ↓
MODEL CONTEXT
```

The existing lifecycle manager operates primarily near the bottom of this stack.

The new idea extends adaptive management upward into the projection itself.

---

# 17. Role composition and context topology are the same family

The session therefore moved through several layers.

## Level 0 — Authored roles

```text
Builder
Reviewer
Supervisor
Planner
```

Roles are treated as relatively atomic semantic objects.

## Level 1 — Role decomposition

```text
role
    =
vantage
authority
obligations
capabilities
effects
continuity
independence
information
lifecycle
```

## Level 2 — Organizational composition

Those primitives can be partly recomposed at runtime.

```text
semantic work
    ↓
required vantages
    ↓
role composition
    ↓
authority / capability projection
    ↓
ExecutionEnvelope
```

## Level 3 — Adaptive context topology

The same semantic structure can determine not merely which roles exist but which **contexts, temporary reasoning projections, and information boundaries** should exist at each point in execution.

```text
durable semantic world
    ↓
adaptive vantage topology
    ↓
adaptive context projections
    ↓
temporary model reasoning environments
```

That is the evolutionary jump reached in this session.

---

# 18. Unifying principle

The common principle underneath all three levels is:

> **Do not make a model context permanently carry structure merely because that structure may someday be useful.**

Externalize:

* state;
* authority;
* role structure;
* decision requirements;
* evidence;
* context-pressure observations;
* organizational constraints;
* reasoning frameworks.

Then project only what the current judgment requires.

This extends the existing Work Engine idea:

> **The context window is not the database.**

Into:

> **The context window is not the organization either.**

And further:

> **The context window is a temporary, purpose-specific projection of the durable semantic world. Its contents, reasoning affordances, lifetime, and even its partition into logical vantages may change as the structure of the work changes.**

---

# 19. Candidate architectural formulation

A compact description of the resulting idea is:

> **Work Engine should manage model context as an adaptive projection of durable semantic state. It may dynamically change that projection's information, reasoning affordances, lifetime, and topology when evidence establishes that a different context or vantage would improve correctness, independence, locality, continuity, or execution economics. Semantic decomposition remains model judgment where necessary; mechanically determined consequences should be compiled into role contracts, authority grants, context projections, and runtime realizations.**

Or more compactly:

> **Externalize the semantic world; dynamically compile the smallest truthful set of vantages needed to judge it.**

---

# 20. Important unresolved boundary

The strongest caution from the session is that the architecture must not become self-amplifying.

A model should not permanently carry instructions that encourage it to create more roles or contexts.

Instead:

```text
durable observations
        ↓
host-owned pressure / applicability detection
        ↓
bounded decision surface becomes active
        ↓
temporary decision projection
        ↓
semantic judgment where necessary
        ↓
governed state transition
```

The model may participate in deciding whether a new vantage is useful.

It must not gain unilateral authority to manufacture organizational complexity.

---

# 21. What changed during this session

The session began with:

> Can role authority be derived from which role has the best semantic vantage?

It progressed to:

> Can roles themselves be decomposed into primitives and composed dynamically?

Then:

> Can role contracts be compiled from the semantic structure of work?

Then:

> Can a running role create a new logical vantage through bounded authority delegation?

Then:

> Can Work Engine partially determine when creating that vantage is beneficial or harmful?

And finally:

> Is dynamic vantage formation actually part of a broader adaptive context-management architecture?

The resulting answer appears to be yes.

We did not merely move one level outward from role design.

We moved from:

```text
role design
```

to:

```text
organizational compilation
```

and then again to:

```text
adaptive semantic projection / context topology
```

That is the important conceptual expansion from this session.


---

Me: It would also be very easy to configure workflows with different levels of organizational dynamics. There may be experiments, for example, that do not want organizational branching. But that becomes trivial once the abstraction is in place.

Sol:
Exactly. Once **organizational change is represented as an admitted capability over context/vantage topology**, “dynamic” versus “static” stops being an architectural fork. It becomes policy.

That is a very useful consequence.

```text
same semantic substrate
same role primitives
same projection machinery
same lifecycle machinery

        +

organizational-dynamics policy

        ↓

different execution regimes
```

So an experiment that requires a fixed organization does not need a stripped-down implementation of Work Engine. It simply receives an envelope whose permitted topology transitions are closed.

Conceptually:

```yaml
organizational_dynamics:
  context_augmentation: allowed
  disposable_contexts: allowed
  vantage_split: denied
  role_creation: denied
  authority_delegation: denied
  topology_revision: denied
```

Another workflow might allow bounded delegation:

```yaml
organizational_dynamics:
  context_augmentation: allowed
  disposable_contexts: allowed
  vantage_split: allowed
  role_creation: allowed
  authority_delegation: attenuated_only
  topology_revision: bounded
```

And perhaps a genuinely exploratory workflow eventually allows much broader adaptation.

The key is that these aren't different orchestration systems. They are **different authority ceilings over the same organizational transition vocabulary**.

That also gives experiments something especially valuable: **organizational dynamics becomes a controllable experimental variable**.

You could hold everything else fixed:

```text
same coordinate
same objective
same model realization
same evidence
same initial roles
same capabilities
```

and vary only:

```text
A: topology frozen

B: temporary context decomposition allowed

C: dynamic vantage formation allowed

D: bounded role composition/delegation allowed
```

Then ask whether organizational plasticity actually improves accepted work, token economics, error rates, context pressure, recovery, etc.

And because topology changes would be durable state transitions rather than invisible agent behavior, you'd know exactly what changed and when.

There is another nice architectural consequence: **static workflows become the easiest special case of the dynamic model**.

```text
dynamic organizational model
        ↓ constrain transition set to ∅
static organization
```

Rather than:

```text
static workflow architecture
        +
separate dynamic-agent architecture
```

That is usually a sign that the abstraction is getting healthier.

I think you could even view organizational dynamics similarly to capability grants:

> The system may understand how to split a vantage, compose a role, or delegate authority without every execution being permitted to do so.

So knowledge of the mechanism and authority to exercise it remain separate.

This becomes especially clean for research. A pilot can say:

> **No organizational branching is admissible after coordinate C.**

That isn't merely an instruction to the agents. Work Engine can mechanically prevent the transition.

Or:

> **Context augmentation is admissible, but logical vantage and authority topology are frozen.**

Now you've separated **context adaptation** from **organizational adaptation**, which would make experimental claims much sharper.

And there are probably several useful intermediate regimes rather than just static/dynamic:

```text
fixed topology
    ↓
adaptive projection only
    ↓
disposable context branching
    ↓
dynamic vantage branching
    ↓
bounded role/authority composition
    ↓
revisionable organization
```

Those aren't necessarily maturity stages. They can simply be execution policies chosen according to the work.

That reinforces the bigger insight from today: once roles, vantages, contexts, and authority are represented independently enough, **organizational flexibility itself becomes composable and governable rather than something baked into the workflow design**.

So yes — your experiment example is almost the proof of why the abstraction matters. A system with truly dynamic organizational primitives should be able to produce a strictly static organization simply by denying the relevant transitions, without needing a second architecture.


Me: I guess this would integrate into context-lifecycle. We have the context decision owned by context life-cycle right now so we reduce model cognitive load. What we talked about before was using the model inference with JIT organizational metrics based on thresholds with a means to performs the branch and create the vantage. Let's look at the ownership and examine how that decision affects the agent vs the context-lifecycle as it stands. I believe we must be using inference in that context-lifecycle right now. Maybe you could double check the code on github?

Sol:
Yes. I checked current `main`, and the answer is: **you are already using inference inside context-lifecycle—but not for the pressure/replacement trigger itself.** That distinction is important for this new idea.

Today the live path is roughly:

```text
token-usage observation
        ↓
TokenUsagePressureProjector          deterministic
        ↓
ContextPressureController            deterministic thresholds + hysteresis
        ↓
replacement_candidate / critical
        ↓
LiveContextLifecycleCoordinator
        ↓
observed-context projection
        ↓
SemanticContextInferenceRuntime
    compiler inference
    verifier inference
        ↓
checkpoint publication / fences      deterministic
        ↓
transition lease
        ↓
target model invokes new_context      sterile actuator
        ↓
rehydration / reconciliation
```

The relevant live profile currently says:

```text
approaching            65%
replacement_candidate  75%
critical               90%

transition_at:
    replacement_candidate
    critical
```

`TokenUsagePressureProjector` computes pressure directly from `last.totalTokens / modelContextWindow`; `ContextPressureController` deterministically maps that through the configured bands and hysteresis. There is no model call in that decision. The active-turn and post-turn paths both use that deterministic disposition to decide whether lifecycle preparation should begin.

So the model is **not currently deciding “my context is too full; replace me.”**

That ownership has successfully moved out of the role.

### Where inference actually happens today

Once pressure has caused lifecycle preparation to begin, `LiveContextLifecycleCoordinator` calls `inferenceRuntime.inspect(...)`.

That invokes two disposable inference capabilities:

```text
semantic compiler
    ↓
continuation candidate

semantic verifier
    ↓
challenge sufficiency / attribution /
authority preservation / interaction closure /
source binding
```

The compiler already makes genuinely semantic judgments about things including:

* objective and logical progression;
* current work position;
* completed consequences;
* commitments;
* decisions and premises;
* authority dependencies;
* evidence interpretation;
* unresolved questions;
* governing instructions;
* human interaction state;
* authorized next action.

Then a second model challenges that candidate. Host code—not the verifier model—derives `accepted`, `unresolved`, or `rejected`.

That is all in `app-server/src/semantic-context-inference.mjs`, with actual disposable Codex calls implemented by `CodexAppServerInferenceCapability`.

So your intuition was right: **context-lifecycle already contains a bounded inference subsystem.**

But its present semantic question is:

> “What meaning must survive this context transition, and is that continuation representation sufficient?”

It does **not** currently ask:

> “Should the semantic organization of the context change?”

---

There is also an interesting implementation/design gap.

The lifecycle design document says:

> preservation and retirement are independent decisions, and retirement should happen only when replacement is semantically safe **and economically advantageous**.

But the current live implementation does not appear to have that full economic decision yet. The trigger is the fixed pressure policy, followed by semantic continuation verification. I don't see expected remaining work, projected future savings, or break-even economics entering the live replacement decision in the modules I traced.

That matches the later lifecycle audit, which explicitly observed that a replacement removed about 202k live tokens but did **not** demonstrate net token savings because compiler/verifier/reconciliation overhead was large.

So today:

```text
replacement candidate
    ≈ token-pressure threshold

semantic inference
    ≈ can we preserve/reconstruct meaning safely?
```

not yet:

```text
replacement decision
    = pressure
    + expected remaining work
    + transition cost
    + projected savings
    + semantic fitness
```

That is actually useful evidence for what we're discussing.

## The ownership boundary today

The design document is explicit about this.

The context-lifecycle manager owns things like:

```text
thread / turn observation
token/context/cost telemetry
bounded context projections
inspection scheduling
semantic compiler/verifier invocation
checkpoint storage
transition readiness
transition coordination
transition classification
rehydration
lifecycle ledger
```

But it explicitly does **not** own:

```text
role objective
domain truth
workflow authority
human approval / preference
canonical decisions
authority grants
```

That tells us where our new idea hits a seam.

A **context replacement** can legitimately belong to context-lifecycle:

```text
same role
same authority
same semantic obligation
same vantage

old context → new context
```

A **new vantage/role** may change:

```text
semantic ownership
authority topology
workflow topology
information boundaries
continuity ownership
```

So context-lifecycle cannot simply decide:

> “You should create Builder B and delegate D to it.”

That would violate its current ownership boundary.

---

## But I think context-lifecycle is still the natural owner of the *decision surface*

This is the important distinction.

It can own:

> **when the current context/vantage configuration has accumulated enough evidence that an organizational decision should be considered.**

It doesn't have to own:

> **what the semantic organization should become.**

That looks very much like the current lifecycle architecture.

Today:

```text
context-lifecycle
    observes pressure
        ↓
deterministically decides inspection is warranted
        ↓
invokes bounded semantic inference
        ↓
host admits or rejects transition
```

Potential future:

```text
context-lifecycle
    observes context-fitness evidence
        ↓
deterministically decides organizational
decision surface is warranted
        ↓
injects / invokes bounded organizational judgment
        ↓
semantic owner decides unresolved consequence
        ↓
organizational authority admits topology revision
        ↓
context-lifecycle realizes resulting
context topology
```

That's remarkably parallel.

## Agent versus lifecycle

I think there are actually three ownership levels, not two.

### 1. Context-lifecycle: observe and nominate

It can mechanically own metrics like:

```text
token pressure
relevance pressure
semantic-width pressure
context lifetime divergence
shared-state coupling
projection size
estimated branch context size
coordination history
```

And hard constraints:

```text
independence requires separation
non-transferable authority forbids transfer
atomic transition forbids splitting
workflow policy freezes topology
```

Then it can determine:

```text
no organizational question exists
```

or:

```text
organizational question is now material
```

That keeps this cognitive burden out of the normal role.

### 2. Active semantic role: judge the irreducible work question

When the case isn't mechanically determined, the current role probably has the best vantage to answer something like:

> Is this subproblem independently coherent?

> How much of my accumulated understanding is genuinely required?

> Would separating it destroy useful reasoning continuity?

> Is the semantic boundary stable enough to hand off through a contract?

Those are exactly the kinds of things the role knows because it is doing the work.

So the JIT projection could temporarily add:

```text
ORGANIZATIONAL DECISION

Mechanically established:
    separation lawful
    authority delegable
    child capabilities satisfiable
    estimated projection reduction = ...
    mutation overlap = ...
    shared dependencies = ...
    coordination cost evidence = ...

Unresolved:
    Does this work form a sufficiently coherent semantic boundary
    that a distinct vantage improves the expected consequence?

Possible outcomes:
    retain
    disposable context
    distinct vantage
    delegated logical role
```

The agent needn't know any of this until that decision exists.

### 3. ExecutionEnvelope/workflow authority: admit the organizational consequence

Even if the builder says:

> Yes, split this.

that should probably be a **semantic nomination/decision**, not unilateral authority to rewrite the organization.

Something has to verify:

```text
parent may delegate this authority
child grant is attenuated correctly
workflow permits organizational branching
independence constraints hold
no competing authority owner exists
required capabilities can be realized
```

Then the ExecutionEnvelope can get a new revision.

So:

```text
role judges semantic usefulness
        ≠
role mints organization
```

That preserves the current ownership model.

---

## And context-lifecycle comes back in after admission

Suppose an organizational decision is accepted:

```text
split subproblem S
into vantage V2
under authority grant G
```

The organizational/compiler layer derives:

```text
child role contract
authority grant
information requirements
continuity requirements
capability requirements
parent/child boundary contract
```

Then context-lifecycle has a perfectly natural job:

```text
parent context projection changes
        +
child context projection created
        +
JIT organizational skill removed
        +
new lifecycle subjects registered
```

So context-lifecycle doesn't become the organizational authority.

It becomes the **projection and lifecycle executor for an admitted organizational state**.

That feels clean.

---

There is another reason I like this split: **the code already contains the exact architectural pattern we need.**

Current lifecycle deliberately does this:

```text
ordinary role
    does NOT monitor its token pressure
    does NOT inventory its own continuation state
    does NOT decide persistence mechanics

lifecycle service
    detects condition
    invokes bounded inference only when needed
    validates result
    performs mechanics
```

Our new idea could preserve the same cognitive-load rule:

```text
ordinary role
    does NOT continuously monitor organizational fitness
    does NOT carry role-composition doctrine
    does NOT continuously consider spawning children

context/topology machinery
    detects structural pressure
    injects bounded decision capability only when warranted

role
    resolves only the irreducible semantic question

system
    compiles and admits consequences
```

That's almost an exact generalization of what you already built.

### One subtle change I would make to our earlier framing

We had been saying:

> JIT organizational metrics based on thresholds cause the role to decide whether to branch.

I think the current lifecycle architecture suggests a slightly better formulation:

```text
metrics / invariants
        ↓
determine whether organizational
judgment is WARRANTED
        ↓
JIT semantic inference
        ↓
organizational decision
        ↓
admission / compilation
```

Not:

```text
threshold crossed
        ↓
branch
```

The threshold is the equivalent of today's `replacement_candidate`.

It opens the semantic decision surface.

It doesn't predetermine its outcome.

And some conditions won't even require inference:

```text
independence contract requires fresh vantage
        → topology constraint is mechanical
```

while others will:

```text
high context separation benefit
+ low coupling
+ coherent-looking subproblem
        → worth asking the semantic question
```

---

So after looking at current code, I think this new architecture lands surprisingly naturally.

The current lifecycle manager is already:

> **a host-owned observer that uses deterministic pressure detection to invoke disposable semantic inference and then performs a governed context transition without burdening the working agent with lifecycle mechanics.**

The evolution would be:

> **a host-owned context-topology manager that uses deterministic context-fitness evidence to invoke JIT semantic organizational judgment and then realizes an independently authorized projection/topology transition without burdening the working agent with permanent organizational machinery.**

I would **extend the lifecycle family**, but I would not give the existing context-lifecycle owner authority over role topology. The context/vantage decision crosses into ExecutionEnvelope/workflow authority, so the clean architecture is likely a collaboration between lifecycle observation/projection, semantic role judgment, and organizational admission.

That seam is now concrete enough that the CLI pass could probably trace it against `semantic-context-lifecycle-manager.md`, `organizational-execution-envelopes.md`, and the role-compiler work rather than exploring from scratch.


Me: There might need to be some kind of coordination. Context lifecycle could step on vantage by issuing a new_context immediately after a new vantage was issues before state is safe.

Sol:
Yes. I think that race is real, and it exposes another boundary we hadn't named yet.

The issue isn't that lifecycle and topology need to know each other's internals. It's that **both can change the model's effective reasoning environment**, so their transitions need shared admission/fencing.

The dangerous sequence is exactly what you described:

```text
role running
    ↓
topology service detects separation pressure
    ↓
JIT organizational judgment injected
    ↓
model decides "create vantage B"
    ↓
authority / state begins moving
    ↓
                    lifecycle sees 78% pressure
                    ↓
                    new_context
                    ↓
💥 mixed transition state
```

At that point several things might be half true:

- the organizational decision exists but is not durable;
- the child contract may not yet be compiled;
- parent authority may have been attenuated conceptually but not durably;
- the parent projection may not yet reflect the new boundary;
- child state may not be reconstructable;
- the lifecycle compiler could checkpoint an intermediate world that should never be resumable.

That is exactly the kind of state Work Engine normally refuses to manufacture.

## I don't think the services should coordinate directly

Something like this would be undesirable:

```text
context-lifecycle:
    ask topology manager "are you busy?"

topology manager:
    ask lifecycle "are you replacing?"
```

That creates peer-service coupling and races anyway.

Instead, both should consume a shared **transition admission/fencing layer**.

The current lifecycle code already gives us the beginning of that pattern with the transition gate, preparation fences, lifecycle reserve permits, and transition leases.

The abstraction could broaden from:

> context replacement lease

to something more like:

> **reasoning-environment transition lease**

Not necessarily that name, but that scope.

Conceptually:

```text
                 shared transition authority
                          |
             +------------+------------+
             |                         |
       lifecycle transition      topology transition
             |                         |
       replace context           split/create vantage
```

Both must acquire compatible authority before changing the effective reasoning environment.

---

### There are actually several transition phases to protect

For an organizational decision:

```text
1. organizational judgment becomes active
2. JIT decision projection injected
3. model judgment occurs
4. judgment persisted
5. organizational consequence admitted
6. role contracts / grants compiled
7. parent and child projections established
8. new topology becomes active
```

Lifecycle replacement should probably be prohibited across some of those boundaries.

The simplest invariant is:

> **A context epoch may not be retired while it contains unresolved semantic state that has not yet been externalized sufficiently for truthful reconstruction.**

That already sounds like lifecycle doctrine.

An active organizational judgment is simply a new kind of unresolved semantic state.

So while:

```text
organizational_decision.status = active
```

the lifecycle manager can still:

- observe pressure;
- compile preparatory evidence;
- maybe prepare a checkpoint;

but **retirement admission stays closed**.

Once the organizational decision is durably settled, lifecycle can reevaluate against the new world.

---

## This suggests two different locks/fences

We probably don't want to block too much.

A JIT injection itself does not necessarily need a huge topology transaction.

There may be:

### Decision-episode fence

Protects an unresolved semantic judgment.

```text
org judgment injected
        ↓
DECISION EPISODE ACTIVE
        ↓
new_context prohibited
        ↓
decision durable / aborted
        ↓
fence released
```

This could apply beyond organizational decisions too.

Any event-scoped semantic decision whose result would be lost by context replacement could use it.

Then there is a stronger:

### Topology-transition fence

Protects the actual organizational state change.

```text
accepted split
    ↓
acquire topology transition
    ↓
compile authority / contracts / projections
    ↓
publish coherent successor organization
    ↓
activate
    ↓
release
```

During that period, lifecycle cannot checkpoint/replace the parent based on a stale topology revision.

That distinction seems useful.

---

## Revision binding probably solves much of this elegantly

Rather than just saying “busy,” everything can bind to revisions.

Suppose:

```text
ExecutionEnvelope revision: E17
Parent projection revision: P42
```

Lifecycle begins preparation against:

```text
(E17, P42)
```

Meanwhile the organizational decision commits:

```text
E18
P43
child P1
```

Then the lifecycle transition prepared against `E17/P42` must fail promotion.

It is stale.

That's much stronger than service signaling.

```text
lifecycle preparation
    bound to E17/P42

topology transition
    commits E18/P43

lifecycle promotion
    compares fence
        ↓
STALE
        ↓
recompute against successor world
```

That looks very Work Engine.

The topology service doesn't need to tell lifecycle what happened. The changed authoritative revision invalidates its prepared transition mechanically.

---

## And the reverse race matters too

Suppose lifecycle has already closed input admission and is preparing retirement.

Then topology shouldn't begin an organizational judgment inside that context.

```text
lifecycle preparation active
        ↓
topology service detects interesting evidence
        ↓
do NOT inject decision now
```

It should either:

- defer the organizational decision until the successor context is reconciled; or
- if policy says the decision is more important than retirement, invalidate/abort lifecycle preparation and keep the context.

That arbitration may sometimes itself be deterministic.

For example:

```text
critical provider pressure
    + no safe deferral
        → lifecycle priority

ordinary replacement_candidate
    + active topology judgment
        → topology episode finishes first
```

But I wouldn't hard-code that hierarchy yet. The important thing is that the conflict is explicit and admitted.

---

## The resulting architecture is getting clearer

```text
                    CONTEXT OBSERVER
                           |
             +-------------+-------------+
             |                           |
             v                           v
      CONTEXT LIFECYCLE            VANTAGE/TOPOLOGY
         ANALYSIS                     ANALYSIS
             |                           |
             +-------------+-------------+
                           |
                           v
             TRANSITION ADMISSION /
                    FENCING
                           |
              +------------+-----------+
              |                        |
              v                        v
       context transition       topology transition
              |                        |
              +------------+-----------+
                           |
                           v
                PROJECTION REALIZATION
```

The shared middle layer owns neither semantic domain.

It owns things like:

- mutual exclusion where necessary;
- revision fences;
- transition ordering;
- stale-transition rejection;
- atomic activation boundaries;
- recovery when a transition dies halfway through.

That is infrastructure authority rather than semantic authority.

And it means our earlier idea of two consumer services is still intact. We just discovered that **their effects converge on one mutable resource: the active reasoning environment**.

So yes: I think some shared coordination is required, but I would frame it as **transition coordination over context/vantage state**, not lifecycle coordinating with topology directly.

The strongest invariant I see is:

> **No transition may activate a successor reasoning environment from a world revision that ceased to be authoritative while that transition was being prepared.**

That covers your exact `new_context` race and probably a lot more.
