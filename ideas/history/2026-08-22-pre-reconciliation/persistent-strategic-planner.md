# Persistent Strategic Planner

## Status

Idea / architecture proposal

## Motivation

Work Engine now has increasingly strong execution machinery for bounded campaigns:

```text
plan
→ execute
→ validate
→ record outcome
```

What is still weak is the feedback loop from execution back into strategic planning:

```text
record outcome
→ reconsider priorities, dependencies, assumptions, and route
```

A long-running campaign can successfully complete many slices while the roadmap that launched it becomes stale.

Execution itself can change the strategic landscape:

- foundational capabilities may become complete earlier than expected;
- implementation evidence may invalidate roadmap assumptions;
- a newly discovered dependency may become the real bottleneck;
- a capability added by one slice may make later work cheaper or obsolete;
- predicted complexity may diverge sharply from observed complexity;
- repeated review, recovery, or provider failures may expose a more important architectural problem;
- work that was initially high priority may become less valuable after adjacent discoveries.

A campaign supervisor should not be required to continuously reconsider the entire product strategy while also maintaining execution coherence.

This suggests a distinct planning role.

---

## Core idea

Introduce a persistent **Strategic Planner** whose responsibility is to maintain a coherent long-horizon model of:

- where the product is now;
- where it is intended to go;
- why the current roadmap is ordered as it is;
- which assumptions remain active;
- which assumptions have been falsified;
- which dependencies constrain future work;
- which ideas are deferred and why;
- what execution evidence has materially changed expected value, risk, or priority.

The planner does not manage individual implementation steps.

Its question is:

> **Given what we now know about the product, are we still working on the right thing in the right order?**

The campaign supervisor asks a different question:

> **Given the current campaign objective, how do we keep execution moving correctly?**

Those responsibilities should remain distinct.

---

## Initial authority model

A useful first iteration is:

```text
Strategic Planner
    establishes or revises strategic direction
             │
             ▼
Campaign Supervisor
    executes a bounded campaign
    observes material execution changes
    decides whether strategic review is warranted
             │
             ├── no material strategic change
             │       → continue campaign
             │
             └── strategic review warranted
                     ↓
               invoke planner
                     ↓
               planner reconciles
               current evidence
                     ↓
               revised or confirmed
               strategic guidance
                     ↓
               supervisor continues
```

The supervisor decides when to invoke the planner.

This avoids inventing a planning cadence before Work Engine has enough evidence to know what cadence is useful.

---

## Consequence for planner invocation

Planner invocation should not be governed initially by fixed rules such as:

```text
after N slices
→ invoke planner

complexity > threshold
→ invoke planner

route revision count > threshold
→ invoke planner
```

Those may become useful evidence, but they should not become procedural triggers merely because they are measurable.

The relevant consequence is:

> **Strategic planning should be revisited when execution produces evidence that could materially change roadmap priority, dependency order, architectural assumptions, expected value, or the wisdom of continuing the current campaign unchanged.**

Evidence that may contribute to that judgment includes:

- proposal complexity;
- predicted fan-out;
- observed fan-out;
- unexpected architectural reach;
- route invalidation;
- major divergence between predicted and observed effort;
- repeated review or repair cycles;
- provider or capability limitations;
- newly available capabilities;
- newly discovered dependencies;
- significant changes in release readiness;
- completion of foundational work that changes downstream options.

These are signals, not commands.

---

## Persistent planner context

The planner should be able to remain persistent across a long planning horizon.

Its accumulated context has value because it contains:

- the rationale behind current priorities;
- historical assumptions;
- prior route revisions;
- deferred opportunities;
- earlier strategic tradeoffs;
- patterns observed across campaigns;
- context for why particular work exists.

Recreating that strategic understanding from scratch after every slice would be expensive and would discard useful continuity.

However, the planner's understanding of the current product state can become stale as execution proceeds.

The intended model is:

```text
persistent strategic context
        +
semantic execution delta
        +
updated durable product state
        ↓
planner refreshes strategic map
```

Persistence preserves long-horizon understanding.

Refresh keeps the planner synchronized with current reality.

---

## Semantic deltas instead of full replay

The planner should not need to reread entire campaign transcripts after every planning break.

Execution should eventually provide compact durable consequences such as:

```text
slice outcome
accepted / stopped / failed

what changed
new capability
new dependency
route revision
new limitation
resolved uncertainty

prediction vs observation
expected effort
observed effort
expected fan-out
observed fan-out
expected risk
observed repair / review burden

strategic consequence
roadmap assumption confirmed
roadmap assumption weakened
priority changed
dependency order changed
no strategic change
```

The planner can combine those deltas with durable state, receipts, proposal packets, and the current roadmap.

This is consistent with the broader Work Engine principle:

> **Preserve the consequence of reasoning, not necessarily the reasoning transcript.**

---

## Relationship to persistent agent state

A persistent planner is itself a long-running state-aware agent.

Its durable state may eventually include:

```text
current strategic thesis
active roadmap assumptions
known dependencies
priority rationale
deferred opportunities
open strategic uncertainties
last reconciled campaign / receipt / proposal state
current planning horizon
```

The planner should not depend entirely on model context for those facts.

Context provides working understanding.

Durable planner state provides reconstructability after compaction, restart, or replacement.

The exact reconstruction route belongs to model judgment.

---

## Relationship to proposal packets

Proposal packets should make planner judgment substantially stronger.

They provide structured pre-execution expectations such as:

- expected impact;
- complexity;
- fan-out;
- risk;
- validation burden;
- expected cost;
- confidence;
- dependencies;
- architectural placement.

Observed execution can then be compared with those expectations.

Conceptually:

```text
proposal
    predicted value / risk / cost / complexity
              ↓
execution
              ↓
receipts / state / telemetry / review
              ↓
observed outcome
              ↓
strategic planner
              ↓
confirm or revise roadmap
```

The proposal packet therefore creates evidence for planning breaks without determining when a planning break must occur.

---

## Planning opportunities in the roadmap

The roadmap may identify places where strategic review is likely to be valuable.

For example:

```text
planning opportunity:
this region contains high architectural uncertainty
and may materially change downstream priorities
```

This is descriptive planning metadata, not a mandatory checkpoint.

The supervisor may consider it and decide that no planner invocation is warranted.

Likewise, the supervisor may invoke the planner somewhere the roadmap did not anticipate because execution has produced strategically important evidence.

---

## Why the planner should remain separate from the supervisor

The supervisor's persistent context is valuable because it becomes deeply informed about the current campaign.

That same context is not necessarily the best place for long-horizon strategic evaluation.

A planner should reason primarily from durable consequences rather than inheriting every implementation detail, debugging exchange, and local route decision.

Separating the roles provides:

- cleaner context boundaries;
- reduced execution-context pollution;
- a fresh strategic perspective when needed;
- long-term strategic continuity without forcing it into every campaign decision;
- a natural place to reconcile roadmap assumptions with execution evidence.

The planner should influence **what work should exist and how it should be prioritized**.

The supervisor should retain authority over **how bounded campaign execution proceeds**.

---

## Initial version

The first implementation should remain simple.

It does not need:

- automatic scheduling;
- a dedicated planning daemon;
- numeric planning thresholds;
- a portfolio database;
- automatic roadmap mutation;
- a new deterministic planning procedure.

A useful first version needs only:

1. a planner role with a persistent context;
2. a bounded strategic objective;
3. access to the roadmap, ideas, durable campaign outcomes, proposal packets when available, and relevant product state;
4. a supervisor affordance for invoking the planner;
5. a compact planning handoff back to the supervisor;
6. durable planner state when persistent-agent-state infrastructure is available.

The supervisor uses model judgment to decide when the strategic planning capability is worth invoking.

---

## Planning output

The planner should return consequences useful to execution rather than a large prose report by default.

A planning result may include:

```text
strategic verdict
continue
revise
pause
reorder
split campaign
stop campaign

changed assumptions
confirmed assumptions
invalidated assumptions

priority changes
dependency changes
newly important work
deferred work

current strategic rationale
open strategic uncertainties

recommended campaign objective
```

The exact output contract should remain as small as downstream consumers require.

---

## Design principle

The planner exists to close the loop between execution and strategy without converting planning into procedure.

The intended control loop is:

```text
plan
→ execute
→ observe
→ learn
→ reconsider
→ plan
```

The strategic planner should preserve long-horizon understanding, while the supervisor decides when current execution evidence has become strategically important enough to justify invoking it.

A compact statement is:

> **Let execution accumulate evidence. Let the supervisor recognize when that evidence may change strategy. Let a persistent planner reconcile the larger map.**
