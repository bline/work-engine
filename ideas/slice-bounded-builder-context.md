# Slice-Bounded Builder Context

## Status

Exploratory idea.

This idea treats builder context as disposable working state rather than durable workflow state.

The central design claim is:

> **A completed slice must not depend on preserving the builder context that produced it.**

Durable slice state is the correctness requirement.

Context reset, pruning, or worker replacement is an optimization mechanism.

---

## Motivation

Builder agents accumulate substantial transient context while implementing a slice:

- exploratory reasoning;
- temporary hypotheses;
- tool outputs;
- rejected routes;
- implementation details;
- intermediate review feedback;
- local debugging evidence;
- repeated repository reads.

Much of that context becomes redundant once its useful consequences have been materialized into durable forms such as:

- code changes;
- accepted plans;
- checkpoints;
- receipts;
- findings and dispositions;
- claim revisions;
- repository state;
- explicit unresolved obligations.

Continuing to carry the original conversation after those consequences are durable causes later turns to repeatedly pay for context that may no longer contribute useful working state.

The problem is particularly important for builders because their work is naturally bounded by slices, while supervisory context may legitimately remain useful across many slices.

---

## Core distinction

Separate:

```text
DURABLE SLICE STATE
what later work must be able to rely on

from

BUILDER CONTEXT
temporary working memory used to produce that state
```

The former is required.

The latter is replaceable.

A slice boundary should therefore be treated as a potential context-lifetime boundary.

---

## Primary invariant

> **After a slice reaches its durable checkpoint, the next slice must be able to proceed correctly without relying on the previous builder's transient conversational history.**

A stronger operational test is:

> **Could a fresh builder continue from the accepted slice state without access to hidden prior conversation?**

If not, the slice has not externalized enough state.

---

## One slice per builder context

A simple policy is:

```text
builder context begins
    ↓
builder performs one slice
    ↓
slice state becomes durable
    ↓
builder context ends or resets
    ↓
next slice begins from durable state
```

This intentionally bounds builder context lifetime to one slice.

The policy does not require a particular context-management mechanism.

---

## Context-reset mechanisms

Several mechanisms may satisfy the same slice-boundary invariant.

### Native context replacement

Where the harness exposes a safe context-reset primitive, the existing builder may begin a fresh model-visible context while preserving its logical agent/thread lineage.

Example current Codex mechanism:

```text
new_context
```

This can replace the previous model-visible history with fresh initial context without requiring a summarizing compaction.

### Worker replacement

If native context replacement is unavailable:

```text
finish current builder
    ↓
spawn replacement builder
    ↓
rehydrate from durable slice state
```

This is semantically equivalent but may incur more startup and reorientation cost.

### Future pruning mechanisms

A future harness may allow selective pruning or replacement of active context.

Such mechanisms may be adopted without changing the slice-state invariant.

---

## Capability-driven optimization

Work Engine should not depend on one harness-specific context feature.

Instead:

```text
context-reset capability available?
        │
        ├── yes → use supported reset optimization
        │
        └── no  → use worker-replacement fallback
```

For Codex, the strongest runtime evidence is the model-visible capability surface itself.

If `new_context` is exposed to the builder, the optimization is available.

If it is absent, Work Engine should continue normally using another mechanism.

Configured feature state should not be treated as stronger evidence than actual capability exposure.

---

## Context reset is not correctness

The architecture should preserve the distinction:

```text
INVARIANT
next slice does not depend on disposable prior builder context

CURRENT MECHANISMS
- native new_context
- replacement builder

FUTURE MECHANISMS
- selective pruning
- explicit context replacement
- other harness-native controls
```

This prevents an experimental or harness-specific context feature from becoming structural Work Engine authority.

---

## Supervisor context remains independent

Builder context policy should not imply the same policy for persistent supervisory agents.

The supervisor may legitimately retain:

- cross-slice objectives;
- campaign state;
- unresolved strategic decisions;
- authority state;
- proposal continuity;
- planner state;
- longer-term coordination context.

Therefore:

> **Builder context and supervisor context have different expected lifetimes.**

A context-management mechanism is useful only if its placement can respect that distinction.

A session-wide feature that forces aggressive reset behavior on the persistent supervisor merely to enable it for builders is not an acceptable dependency.

---

## Current Codex limitation

Current Codex contains a native fresh-context primitive associated with its token-budget feature.

However, native subagents do not presently provide a clean general mechanism for selecting an independent configuration profile or feature policy at spawn time.

Therefore, if enabling the feature at the supervisor level materially changes supervisor compaction behavior, it should not be required merely to provide context reset to builders.

Until that configuration boundary is sufficiently selective, the feature should be treated as opportunistic rather than required.

---

## Manual slice-boundary policy

A practical interim policy is:

> **Do not rely on automatic token-pressure resets. Intentionally end or reset builder context at semantic slice boundaries after durable state has been established.**

If a native reset capability is enabled but automatic thresholds are undesirable, configuration may be chosen so that normal builder slices do not approach automatic reset thresholds.

The workflow then owns the semantic reset point:

```text
slice accepted
    ↓
verify durable continuation state
    ↓
reset/retire builder context
    ↓
begin next slice
```

The slice boundary, not token pressure, determines when transient context stops being useful.

---

## Rehydration contract

A fresh builder should receive only what is necessary to continue correctly.

Candidate inputs include:

- current objective;
- exact accepted checkpoint;
- active plan revision;
- unresolved obligations;
- relevant current claims;
- applicable invariants;
- authority state;
- current code-change subject;
- necessary evidence references;
- relevant reviewer findings and dispositions;
- permitted next actions.

Historical conversation should remain retrievable evidence rather than mandatory startup context.

---

## Smallest sufficient environment

The goal is not the smallest possible prompt.

The goal is:

> **the smallest sufficient environment for the next slice's judgment and execution.**

Anything omitted must remain recoverable when needed.

Anything included should have a reason to remain behaviorally relevant.

This supports both token efficiency and environmental clarity.

---

## Why the optimization may matter

Repeated builder turns compound the cost of retained context.

Even with prompt caching, larger persistent context may contribute to:

- additional token usage;
- context competition;
- reduced capability salience;
- behavioral drift;
- compaction risk;
- duplicated evidence;
- repeated processing of already-materialized state.

A semantic reset after a slice can eliminate context whose useful consequences have already been externalized.

---

## Relaunch cost

Fresh workers are not free.

Replacement may require:

- role and skill loading;
- tool/environment bootstrap;
- repository orientation;
- state rehydration;
- repeated evidence discovery;
- repeated file reads.

Therefore context replacement should eventually be evaluated empirically as a tradeoff:

```text
CARRY FORWARD COST
context growth
+ repeated token processing
+ context competition
+ compaction risk

versus

REHYDRATION COST
startup context
+ rediscovery
+ repeated tool work
+ orientation
```

The optimum may differ by role, task, model, and harness.

---

## Research opportunity

Once rollout tracing and resource telemetry are available, Work Engine can compare:

```text
continue same builder context

versus

native fresh-context reset

versus

replacement builder
```

against:

- input tokens;
- output tokens;
- total lifecycle tokens;
- startup overhead;
- repeated file/tool work;
- task correctness;
- review findings;
- remediation;
- behavioral differences;
- later defects.

This permits an empirical context-lifetime policy rather than a fixed heuristic.

---

## Relationship to durable state

The idea depends on a broader architectural pattern:

> **History produces state; durable state carries the workflow forward.**

Conversational history remains valuable as provenance and evidence.

It should not remain the only place where required continuation state exists.

At a sufficiently complete checkpoint, prior context can become optional historical evidence instead of active working memory.

---

## Failure conditions

The design fails if:

- a new builder cannot continue correctly without prior hidden conversation;
- required authorization exists only in transient context;
- unresolved obligations disappear during reset;
- the replacement builder must rediscover large amounts of state that should have been durable;
- resetting context materially changes workflow meaning;
- context reset becomes an implicit substitute for valid checkpointing;
- a harness-specific mechanism becomes required for workflow correctness.

These failures indicate incomplete durable state or an invalid abstraction boundary.

---

## Relationship to context compaction

This idea is distinct from ordinary compaction.

Compaction asks:

> How can prior conversation be summarized so that some of it remains available?

Slice-bounded context asks:

> Has prior conversation already produced enough durable state that the next slice no longer needs it at all?

The latter may legitimately choose no summary.

---

## Compact statement

> **Builder context is disposable working memory. Slice state is durable workflow memory. Once a slice has externalized everything required for continuation, the next slice should be free to begin in a fresh context. Context-reset mechanisms are optimizations; durable continuation is the invariant.**