Yes — I think that separation is stronger.

The architectural review should be **diagnostic**, while the proposal workflow is **constructive**.

That gives you a much cleaner chain of responsibility:

```text
accumulated execution evidence
        ↓
architectural review
        ↓
"there is / is not a material architectural problem"
        ↓
if blocking:
    stop execution
    surface review
        ↓
proposal workflow
        ↓
develop candidate response
        ↓
falsify / compare / refine
        ↓
human approves proposal
        ↓
architecture changes
```

The architectural reviewer does **not** need to solve the problem. In fact, I think it is probably better if it doesn't. Its job is to say things like:

* boundary A is no longer holding;
* responsibility B is repeatedly leaking into C;
* assumption X has been contradicted by execution evidence;
* the current decomposition is creating repeated cross-cutting changes;
* continuing under the existing architecture is likely to compound the problem.

Then it produces enough evidence for the proposal system to work from.

That keeps two different reasoning modes separate:

> **Architectural review:** “What appears to be wrong with the current system model?”

> **Proposal workflow:** “Given that problem, what should we change, and why is this solution preferable?”

That is a really useful distinction because otherwise the reviewer can become attached to its own diagnosis *and* its own solution. A separate proposal process gets a fresh reasoning context and can even conclude that the review identified a real symptom but proposed the wrong interpretation.

### Confidence as blocking authority also makes sense

I would make confidence part of the architecture review's consequence, but I wouldn't let confidence alone determine blocking. Something like:

```text
architectural finding:
    severity
    confidence
    evidence
    affected assumptions
    expected consequence of continuing
```

Then the planner judges the combined result.

For example:

```text
low confidence + severe possibility
    → surface / accumulate evidence
    → usually continue

medium confidence + material architectural risk
    → planner may stop

high confidence + material architectural defect
    → stop strongly indicated
```

Your idea of **medium or high confidence as a necessary condition for blocking** is a good guardrail. It prevents speculative architectural criticism from repeatedly halting execution.

I'd phrase the invariant approximately as:

> **An architectural review may block continued execution only when it identifies a material architectural concern with at least medium confidence.**

But I would avoid:

```text
if confidence >= medium:
    block
```

because medium confidence in something trivial shouldn't stop anything.

The real condition is closer to:

```text
material_consequence
AND
confidence >= medium
AND
continuing unchanged is no longer reasonably supportable
```

That preserves model judgment.

### Low-confidence findings still have value

I wouldn't throw them away. They can become **architectural observations** in durable state.

That's actually where this system gets interesting:

```text
slice 4:
  low-confidence boundary concern

slice 7:
  another low-confidence observation

slice 9:
  same boundary causes placement ambiguity

slice 11:
  review finds cross-layer repair

architectural review:
  accumulated evidence now supports
  medium/high-confidence finding
```

So you don't need every review to discover a blocker in one shot. The architecture reviewer can reason over accumulated observations.

That fits extremely well with your planner's existing idea that execution produces semantic deltas and durable consequences instead of forcing higher-level agents to replay all the raw work. The planner proposal already expects execution evidence to include things like new dependencies, route revisions, limitations, prediction-vs-observation deltas, and changed assumptions. 

I think you've now got three nicely separated artifacts:

```text
ARCHITECTURAL REVIEW
diagnosis
- what appears wrong
- evidence
- confidence
- consequence of continuing
- affected architectural assumptions


PROPOSAL
response
- proposed change
- alternatives
- placement
- expected impact
- risk
- migration implications
- falsification evidence


ARCHITECTURE
authority
- the human-approved governing design
```

And that separation gives you an especially valuable property:

**A reviewer can stop the machine without being allowed to redesign it.**

Then the proposal machinery can take over and do the much richer exploratory reasoning necessary to decide what the redesign should actually be. I think that's substantially safer and conceptually cleaner than making “architectural review” mean both diagnosis and repair.

