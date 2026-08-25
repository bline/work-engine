# Semantic Durability and Context-Lifetime Judgment

## Status

Exploratory idea.

This idea defines how an agent should reason about meaning held in model context, when that meaning should be externalized into durable state, and when replacing the current context becomes economically and semantically preferable to continuing within it.

The central distinction is:

> **Context is conditionally durable working memory. Meaning should be preserved when its expected future value, loss risk, and reconstruction cost justify the cost of externalizing it. Context should be replaced when continuing to carry it is expected to cost more than preserving the remaining useful meaning and rehydrating a fresh working set.**

This is not a fixed-threshold policy and does not assume that all future-relevant meaning should be written to state immediately.

---

## 1. Motivation

Model context is useful working memory.

It is not, however, uniformly durable.

Its contents may survive many rounds, but they may later be:

- compacted;
- summarized;
- replaced;
- truncated;
- omitted;
- or otherwise transformed by the harness.

At the same time, externalizing meaning into durable state has its own cost:

- reasoning needed to identify and distill the meaning;
- tokens required to represent it;
- state-write/tool-call cost;
- schema or provenance overhead;
- possible loss of nuance during compression;
- and later rehydration cost.

Therefore neither of these extreme policies is desirable:

```text
keep everything in context forever
```

or:

```text
save every potentially useful thought immediately
```

The desired behavior is adaptive.

---

## 2. Context Has Conditional Durability

The current context should not be described simply as durable or non-durable.

Its durability is conditional.

A piece of meaning may remain safely available through many rounds when:

- substantial context capacity remains;
- replacement is unlikely;
- it is cheap to reconstruct;
- it is needed only briefly;
- or its loss would have little consequence.

The same meaning may become unsafe to leave only in context when:

- context pressure rises;
- automatic replacement becomes more likely;
- future work increasingly depends on it;
- reconstructing it would be expensive;
- or the required source material may itself disappear.

A useful principle is:

> **Externalize meaning when its required lifetime and consequence of loss exceed the durability the current context can safely provide.**

---

## 3. Meaning, Not Transcript

The object being preserved is not necessarily the original text.

Context produces meaning.

Examples include:

- a conclusion;
- a current objective;
- a logical progression;
- a decision;
- an obligation;
- an accepted boundary;
- an unresolved uncertainty;
- a dependency;
- an authority relationship;
- an interpretation of evidence;
- a route that has been falsified;
- an intent that is not yet captured elsewhere;
- a consequence that future work must respect.

The preservation problem is therefore:

> **What meaning has become continuation-relevant, and what is the cheapest sufficiently faithful way to preserve it?**

The answer may be:

- canonical typed state;
- an existing domain artifact;
- a claim;
- a checkpoint;
- a reference to stronger evidence;
- a bounded continuation memento;
- or no additional preservation at all.

---

## 4. Dimensions of Continuation-Relevant Meaning

A universal ontology is not required, but recurring dimensions help the model recognize meaning that may deserve durability.

Candidate dimensions include:

### Objective

What is currently being attempted?

### Logical progression

What has been established, ruled out, or concluded, and what follows from it?

For example:

```text
schema approach falsified
    ↓
existing formation contract is sufficient
    ↓
new production schema is outside the accepted route
```

The current task name alone may not preserve this progression.

### Current work position

Where is the work now?

Examples:

- implementation active;
- implementation complete;
- review pending;
- remediation active;
- awaiting external evidence.

### Obligations

What must still occur before valid completion?

### Decisions

What choices have been made, by whom, and with what consequence?

### Authority

What is permitted, prohibited, delegated, accepted, or awaiting authorization?

### Boundaries

What is in scope, out of scope, or conditionally included?

### Uncertainty

What remains unresolved, provisional, disputed, or unknown?

### Evidence

What evidence matters, what does it support, and what is still missing?

### Dependencies

What future work depends on this meaning?

### Identity

Which exact proposal, run, revision, attempt, actor, artifact, or checkpoint is involved?

### Relationships

How do important objects relate?

Examples:

- supports;
- contradicts;
- supersedes;
- depends on;
- may affect;
- owns;
- reviews.

### Intent

What intended meaning would be lost if only the literal artifacts survived?

### Constraints

What conditions restrict otherwise valid choices?

### Route

Why is the current route valid, and which alternatives remain available?

### Consequences

What became true because an event or decision occurred?

### Temporal applicability

What is current, stale, superseded, pending, or expired?

### Interaction meaning

What unresolved meaning from direct communication may still affect valid continuation?

These dimensions are recognition aids, not mandatory state fields.

---

## 5. Preserve Opportunistically, Not Continuously

“Continuous preservation” can imply an expensive and overly procedural loop:

```text
every turn
    ↓
inspect all meaning
    ↓
write state
    ↓
continue
```

That may increase token use, latency, tool traffic, and behavioral self-monitoring enough to defeat the purpose.

Instead:

> **Preserve meaning opportunistically when its expected future value justifies externalization.**

Durability should preferably piggyback on judgments the agent is already making.

For example:

```text
decision becomes accepted
    ↓
publish its consequence

new obligation becomes real
    ↓
publish the obligation

boundary changes
    ↓
publish the new boundary

unknown but clearly future-relevant meaning appears
    ↓
publish a bounded memento
```

The agent should not perform a full semantic inventory after every turn.

Durability management should remain low-salience during ordinary work. The model should primarily reason about the task, not continuously monitor whether its own working memory is about to disappear.

A useful behavioral principle is:

> **Use context normally as conditionally durable working memory. Preserve meaning when concrete semantic or lifecycle evidence makes durability worthwhile, rather than treating every turn as an opportunity to evacuate context.**

This is important economically as well as behaviorally. If preservation requires enough self-monitoring, semantic inventory, state compilation, or repeated state injection to approach the cost of the context loss it is intended to prevent, the mechanism has failed its purpose.

---

## 6. Preservation Cost

The cost of compiling meaning is part of its preservation priority.

Candidate costs include:

- additional reasoning;
- token generation;
- state-write/tool-call cost;
- provenance construction;
- validation;
- schema friction;
- representation size;
- later context injection;
- possible fidelity loss introduced by compression;
- durability-monitoring and self-management overhead.

Therefore something may legitimately remain transient even though it might matter later.

The question is not merely:

> Could this matter?

It is:

> **Is preserving it now cheaper and safer than leaving it transient and potentially reconstructing it later?**

---

## 7. Reconstructability

Reconstructability is a major factor in preservation value.

For any continuation-relevant meaning, ask:

1. Can it be reconstructed?
2. What inputs are required?
3. Are those inputs themselves durable?
4. How expensive is reconstruction?
5. How reliable is reconstruction?
6. Would reconstruction lose nuance, uncertainty, attribution, or authority information?

A useful decision tree is:

```text
Can the meaning be reconstructed?

NO
    → strong preservation candidate

YES
    ↓
Are the required inputs durable?

NO
    → preserve result and/or missing inputs

YES
    ↓
Is reconstruction cheap, reliable, and faithful?

YES
    → prefer recomputation where appropriate

NO
    → preserve derived meaning with provenance
```

---

## 8. Prior Versus Result

Sometimes preserving the inputs is cheaper than preserving the derived result.

Sometimes the reverse is true.

For example:

```text
durable code diff
    ↓
cheap deterministic count
    ↓
files_changed = 4
```

The result may be cheaper to recompute than to preserve explicitly.

But:

```text
large evidence set
    ↓
expensive independent judgment
    ↓
architectural conclusion
```

may justify preserving the result because reconstructing it would require substantial inference.

The general principle is:

> **Preserve whichever representation minimizes expected future reconstruction cost while retaining sufficient truth, fidelity, provenance, and authority.**

---

## 9. Bounded Continuation Mementos

Not all continuation-relevant meaning will have a typed home.

When meaning matters but no established state representation fits it, the agent may preserve a bounded continuation memento.

The memento is effectively:

> “Future me: this matters, there is no stronger typed representation yet, do not lose it.”

The content may remain flexible.

The surrounding envelope should remain controlled by the durable-state contract.

Repeated memento shapes are evidence that a new canonical state concept may be useful.

They do not automatically create one.

The evolutionary path is:

```text
context-only meaning
    ↓
bounded memento
    ↓
repeated empirical occurrence
    ↓
candidate typed concept
    ↓
explicit schema/profile decision
```

---

## 10. Context Pressure

Context pressure changes the risk of leaving meaning transient.

A basic observable quantity is:

\[
P_c = \frac{\text{active context tokens}}
           {\text{effective context limit}}
\]

This does not by itself determine whether something should be preserved.

It affects the expected durability of context.

As pressure rises, the economics of preservation change gradually. Context pressure is evidence that may alter preservation judgment; it should not by itself create a sense of imminent loss or trigger defensive state capture.

As pressure rises:

- forced replacement becomes more likely;
- there is less time to notice missing state later;
- the cost of a false negative rises;
- the threshold for preserving plausible continuation meaning may rationally decrease.

This suggests:

```text
low pressure
    preserve clearly valuable meaning

moderate pressure
    become more attentive to likely future relevance

high pressure
    become conservative about leaving important meaning context-only

near exhaustion
    harvest remaining continuation-relevant meaning
```

The exact thresholds should be learned rather than assumed.

The intended transition is increased sensitivity, not urgency. A well-behaved agent should remain focused on useful work at low and moderate pressure and become more conservative about context-only meaning only as the expected risk and cost of loss materially increase.

---

## 11. Context Can Become Economically Exhausted Before It Is Full

The technical hard limit is not necessarily the economically optimal end of a context window.

If the remaining context budget is insufficient for another useful unit of work, consuming it may have low expected value.

A rational agent may instead:

```text
remaining useful context is too small
    ↓
preserve residual continuation meaning
    ↓
replace context
    ↓
rehydrate
    ↓
perform the next meaningful unit of work in a fresh window
```

This gives a distinct principle:

> **A context window may become economically exhausted before it is technically exhausted.**

The hard limit is therefore both:

- a safety boundary;
- and a planning signal.

---

## 12. Context Replacement Remains a Model Judgment

The model should still decide when to replace context.

The environment should provide:

- context-pressure evidence;
- durable-state capabilities;
- reconstruction evidence;
- context-replacement capability;
- and structural constraints on valid continuation.

It should not prescribe:

```text
reset every 20 turns
reset at 70%
reset after every checkpoint
```

unless future evidence demonstrates that such a rule is causally required.

The desired pattern is:

```text
work
    ↓
preserve worthwhile meaning opportunistically
    ↓
observe context cost / remaining capacity
    ↓
judge whether current context remains worth carrying
    ↓
replace when beneficial and semantically safe
```

---

## 13. Safety of Replacement

Context replacement is safe only when correct continuation no longer depends on meaning represented solely in the current context.

This remains distinct from whether replacement is economically beneficial.

Therefore every reset decision has two questions:

### Semantic question

> Would anything required for correct continuation disappear?

If yes, do not replace yet.

### Economic question

> Is retaining this context more valuable than resetting and rehydrating?

If no, replacement is a candidate action.

---

## 14. Human Interaction

Meaning from direct human interaction deserves special caution.

The model cannot reliably assume that every nuance of an ongoing human-facing relationship has been captured merely because formal workflow state exists.

Unresolved human meaning may include:

- intent;
- ambiguity;
- preferences;
- negotiated interpretation;
- informal commitments;
- strategic context;
- unstated but interaction-dependent distinctions.

Human-facing agents therefore require a different context-lifetime policy than machine-managed non-human-facing roles.

This idea primarily concerns contexts where the system can validly externalize continuation meaning into durable workflow state.

---

## 15. Expected-Cost Model

The preservation decision can be represented conceptually as an expected-cost comparison.

One simple form is:

\[
S =
\frac{
R \times L \times (C_r + D_r)
}{
C_p
}
\]

where:

- \(R\) = expected future relevance;
- \(L\) = risk of losing the meaning while it is still needed;
- \(C_r\) = expected cost of reconstructing it;
- \(D_r\) = expected degradation or fidelity loss from reconstruction;
- \(C_p\) = cost of preserving it now.

When \(S\) is sufficiently high, preservation becomes attractive.

This is **not currently a calibrated score**.

The value of the formula is structural:

- it names the relevant variables;
- prevents single-factor reasoning;
- and provides a basis for later empirical calibration.

---

## 16. Prior-Versus-Result Cost Model

Let:

\[
C_p^{prior}
\]

be the cost of preserving sufficient inputs.

Let:

\[
C_p^{result}
\]

be the cost of preserving the derived result.

Let:

\[
C_d
\]

be the future cost of deriving the result again.

Let:

\[
D_d
\]

be the expected fidelity/risk penalty of recomputation.

Preserving prior inputs is preferable when:

\[
C_p^{prior} + E[C_d] + D_d
<
C_p^{result}
\]

subject to required provenance, truth, and authority constraints.

This comparison may differ by meaning type.

---

## 17. Context-Retirement Break-Even

The context reset decision can also be represented as expected cost.

Let:

\[
C_{keep}
\]

represent the expected cost of retaining the current context for the next useful unit of work.

That may include:

- repeated input-token cost;
- latency;
- context competition;
- increasing probability of forced replacement;
- behavioral degradation;
- stale information interference.

Let:

\[
C_{reset}
\]

represent:

- residual meaning preservation;
- reset transition;
- fresh initial context;
- state rehydration;
- rediscovery;
- repeated tool/file work.

Then the economically attractive reset region begins when:

\[
E[C_{keep}] > E[C_{reset}]
\]

provided semantic safety is already satisfied.

The actual optimum should be learned from real runs.

---

## 18. Metrics Required for Calibration

Potential telemetry includes:

### Context

- active context tokens;
- effective hard limit;
- percentage used;
- tokens since last reset;
- turns since last reset;
- inference calls since last reset;
- compaction/reset events.

### Preservation

- meaning-preservation events;
- preservation representation type;
- tokens spent producing the preserved representation;
- tool-call/write cost;
- bytes/tokens stored;
- provenance overhead.

### Reconstruction

- whether saved meaning was later used;
- whether it was recomputed;
- tokens required for recomputation;
- evidence rereads;
- repeated tool calls;
- fidelity corrections after reconstruction.

### Rehydration

- fresh-context bootstrap tokens;
- state injected;
- files reread;
- tools rediscovered;
- time/tokens until productive work resumes.

### Outcomes

- correctness;
- review findings;
- remediation;
- route diversity;
- premise challenges;
- later defects;
- recovery success.

These measurements allow preservation and reset policies to become empirical rather than rhetorical.

---

## 19. Avoiding False Precision

The variables above should initially remain observable or separately estimated.

Do not begin with:

```text
meaning_save_score = 7.43
```

and treat it as objective.

The weights and thresholds are unknown.

The progression should be:

```text
conceptual variables
    ↓
measurement
    ↓
historical correlation
    ↓
candidate predictive relationships
    ↓
controlled or prospective validation
    ↓
calibrated policy
```

---

## 20. Environmental Implications

Context-lifetime policy is itself part of the agent environment.

Very long contexts and very aggressively reset contexts may elicit different behavior.

Potential effects include:

- attention diverted toward context survival or state management;
- premature crystallization of tentative reasoning into durable state;
- excessive defensive preservation;
- evidence seeking;
- premise challenging;
- route diversity;
- anchoring;
- stale authority;
- creativity;
- tool salience;
- uncertainty behavior;
- mechanistic compliance;
- reorientation cost.

Therefore:

> **“Tighter context is better” should not become doctrine without behavioral evidence.**

Likewise, **“context is not durable” should not become operational doctrine.** That framing is too coarse and may induce needless preservation behavior. The more accurate environmental description is that context is conditionally durable working memory whose adequacy depends on required lifetime, consequence of loss, reconstructability, and current lifecycle conditions.

The durability mechanism should therefore be judged by total lifecycle behavior, not merely by whether information survives. Its target is simultaneous improvement in resource cost, continuation reliability, and reasoning quality. A mechanism that prevents loss by consuming comparable or greater tokens, latency, attention, or rehydration cost is not a successful optimization.

A useful empirical question is:

> **After controlling for the task/change profile, how does context-retirement frequency affect reasoning behavior, resource use, and final outcomes?**

---

## 21. Relationship to Wind Walker

Wind Walker can consume these ideas without owning every underlying mechanism.

Its semantic responsibility is:

- recognize context as conditionally durable working memory;
- recognize continuation-relevant meaning;
- use available durable representations when preservation is justified;
- avoid losing context-only meaning required for valid continuation;
- judge when retaining the current context is no longer worthwhile;
- use available context-replacement machinery when safe and beneficial.

It should not own:

- role-specific state schemas;
- canonical domain truth;
- durable-storage internals;
- context telemetry implementation;
- statistical model calibration;
- universal reset thresholds.

---

## 22. Compact Findings

The current design can be summarized by several claims:

> **Context is conditionally durable working memory; it should not be framed as uniformly durable or uniformly non-durable.**

> **Meaning should be preserved because its expected future value justifies durability, not merely because a reset is imminent.**

> **The cost of compiling meaning is part of the decision to preserve it.**

> **Reconstructability, reconstruction cost, and reconstruction fidelity determine whether preserving inputs or derived results is preferable.**

> **Context pressure changes the expected risk of leaving meaning transient; it should alter preservation sensitivity gradually rather than create a perpetual sense of imminent context loss.**

> **A context window may become economically exhausted before it is technically exhausted.**

> **Context replacement remains a runtime judgment constrained by semantic safety and informed by expected cost.**

> **Durability management should reduce total lifecycle cost without becoming a significant cognitive workload of its own.**

> **A successful policy should improve cost, continuation reliability, and reasoning quality together; preventing information loss alone is not sufficient.**

> **The optimal preservation and reset policy should ultimately be learned from measured lifecycle behavior rather than fixed by intuition.**