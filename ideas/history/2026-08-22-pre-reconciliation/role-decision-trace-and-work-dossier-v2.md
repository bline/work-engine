# Role Decision Trace and Work Dossier

## Status

Idea / architecture proposal

## Motivation

Work Engine is increasingly separating information by lifetime, authority, and purpose.

Current and planned layers include:

```text
proposal packet
→ why the work exists
→ what was believed before execution

persistent state
→ what is operationally true now

receipt / transition history
→ what durably happened

raw interaction trace
→ observable forensic execution history

model context
→ transient role-local cognition
```

One important layer is still missing:

> **How did the role's interpretation of the work evolve while it was operating?**

That history matters for more than continuation.

A consequential final judgment may be built from several smaller judgments:

```text
"this caller appears non-authoritative"
        ↓
"this path can probably be ignored"
        ↓
"component A likely owns lifecycle"
        ↓
"place the implementation in A"
```

If only the final placement decision is preserved, later investigation can establish that the placement was wrong but may miss where the reasoning first diverged.

The important error may have entered through a small intermediate judgment.

Those intermediate decisions are valuable for:

- forensic reconstruction;
- debugging role behavior;
- confidence calibration;
- identifying bad premises;
- understanding route formation;
- detecting recurring evidence-selection mistakes;
- evaluating role prompts and environments;
- improving capability design;
- tracing why later decisions became stale;
- closed-loop learning from successful and failed reasoning patterns.

At the same time, Work Engine should not attempt to persist every thought or hidden reasoning token.

The useful target is a durable record of **observable, semantically meaningful role judgments**.

This suggests two related but distinct layers:

```text
ROLE DECISION TRACE
→ fine-grained observable judgments that shaped the role's decision space

ACTIVE DECISION SET
→ the smaller subset of decisions whose consequences remain operationally relevant
```

The first is optimized for forensics and learning.

The second is optimized for continuation and context economy.

---

## Core idea

Each Work Engine role may maintain a durable, role-scoped decision trace associated with the work item.

A broader work container may eventually look like:

```text
work dossier/
├── proposal/
│   └── immutable pre-execution thesis
│
├── decisions/
│   ├── planner.jsonl
│   ├── supervisor.jsonl
│   ├── builder.jsonl
│   ├── reviewer.jsonl
│   └── architect.jsonl
│
├── active-decisions/
│   └── references to currently relied-upon decisions
│
├── state/
│   └── references to authoritative live state
│
├── evidence/
│   └── stable evidence references
│
├── outcomes/
│   └── receipts / metrics / checkpoints
│
└── trace/
    └── references to raw forensic interaction history
```

The proposal remains immutable.

Role decision streams record how interpretation evolved.

The active decision set identifies which decisions currently matter to execution.

Persistent state remains the owner of operational truth.

Receipts remain the owner of durable outcomes.

Raw trace remains the owner of observable interaction history.

---

## Canonical information layers

```text
Proposal
→ original intent, expectations, assumptions, predictions

Decision trace
→ observable role judgments and how they evolved

Active decision set
→ judgments currently relied upon

Persistent state
→ current operational truth

Receipt / transition history
→ execution outcomes and durable state changes

Raw trace
→ observable model/tool interaction history

Model context
→ transient role-local cognitive workspace
```

These layers must not collapse into one another.

The proposal should not become mutable live state.

The decision trace should not become a transcript.

The active decision set should not duplicate full decision records.

State should not become a narrative reasoning archive.

Receipts should not preserve unfinished cognition.

Raw trace should not be required for ordinary continuation.

---

## What belongs in the decision trace

The decision trace should preserve **observable judgments that materially shape, narrow, interpret, or evaluate the role's decision space**, including intermediate judgments that may later explain how a consequential decision was reached.

This is intentionally broader than only recording final behavior-changing decisions.

Examples include:

- an interpretation accepted or rejected;
- an assumption introduced;
- an assumption weakened or strengthened;
- a caller classified as authoritative or non-authoritative;
- an evidence source judged sufficient or insufficient;
- an alternative ruled out;
- a placement hypothesis formed;
- a route selected;
- a route premise falsified;
- confidence materially changed;
- a finding classified;
- a finding dismissed;
- a judgment that no further investigation is needed;
- an uncertainty promoted because later reasoning depends on it;
- an architectural relationship inferred;
- a strategic dependency inferred;
- a recommendation formed;
- a prior decision marked stale;
- a decision superseded.

Examples that normally remain only in model context:

- low-level wording choices;
- trivial tool sequencing;
- incidental thoughts with no effect on interpretation;
- speculative fragments that never become an observable judgment;
- hidden chain-of-thought.

The intent is not to log cognition exhaustively.

It is to capture the **semantic decision events** that make the role's investigation understandable.

---

## Promotion into the active decision set

Not every traced decision belongs in active execution state.

A decision should be promoted when its consequence is currently relied upon by:

- the role itself;
- another authorized role;
- current scope;
- current route;
- current state;
- a pending gate;
- a finding;
- an authority decision;
- a downstream artifact.

For example:

```text
decision trace:
D17  caller B appears non-authoritative
D18  component A likely owns lifecycle
D19  placement A is supportable

active decision set:
D19  placement A is the currently relied-upon placement
```

If later evidence falsifies D17:

```text
E31 falsifies D17
        ↓
D17 stale
        ↓
D18 stale
        ↓
D19 requires reconsideration
```

The trace preserves the ancestry.

The active set changes to reflect what is still operationally relied upon.

---

## Decision graph

The decision trace should be able to represent more than chronology.

Useful relations include:

```text
Evidence ──SUPPORTS────────► Decision
Evidence ──FALSIFIES───────► Decision

Decision ──PREMISE_FOR─────► Decision
Decision ──SUPERSEDES──────► Decision
Decision ──WEAKENS─────────► Decision
Decision ──CONTRADICTS─────► Decision
Decision ──AFFECTS─────────► State / Scope / Route
Decision ──CONSTRAINED_BY──► Invariant
```

This permits **decision ancestry**.

A later forensic investigation could answer:

> Why did the builder choose this placement?

by traversing backward through decisions and evidence.

A route revision could identify not only the final stale decision but the premise that caused it.

This is especially valuable when confidence accumulated on top of an incorrect intermediate premise.

---

## Decision record shape

A decision record may include:

```yaml
decision_id: decision-0042
work_id: work-17
role: builder
agent_id: agent-builder-17
phase: planning

kind: evidence_interpretation

subject:
  type: caller
  ref: symbol:foo

judgment:
  classification: non_authoritative

confidence:
  expressed: 0.76
  semantics: role_expressed_confidence

basis:
  evidence_refs:
    - evidence-18
    - evidence-23
  invariant_refs:
    - INV-011

uncertainty:
  - dynamic registration path not yet inspected

relations:
  premise_for:
    - decision-0044

status: active

consequences:
  - caller excluded from current placement candidate set

created_at: 2026-08-21T14:39:00-06:00
```

The schema should remain minimal and extensible.

Useful properties include:

- stable decision identity;
- role ownership;
- work and agent identity;
- phase/lifecycle position;
- decision kind;
- subject;
- observable judgment;
- expressed confidence;
- evidence references;
- invariant references;
- unresolved uncertainty;
- relations to other decisions;
- status;
- explicit consequences.

---

## Confidence semantics

Confidence is useful only if represented truthfully.

A value such as:

```text
0.82
```

must not be treated as calibrated probability unless calibration has actually been demonstrated.

The safe meaning is:

> **The role's expressed confidence in the judgment at the time it was recorded.**

That still creates valuable data.

Later Work Engine can compare confidence with:

- independent falsification;
- later supersession;
- review outcome;
- repair count;
- route revision;
- regressions;
- acceptance;
- observed cost;
- observed complexity;
- long-term outcome.

This may reveal patterns such as:

```text
high-confidence placement decisions
→ usually survive falsification

medium-confidence caller exclusions
→ disproportionately cause later route revision
```

Those observations can become empirical evidence for future judgment without becoming mandatory procedure.

---

## Decision lifecycle

Possible decision states include:

```text
proposed
active
confirmed
weakened
stale
superseded
rejected
resolved
accepted_as_risk
```

The vocabulary may differ by role or decision kind.

The invariant is that a decision whose meaning changes should not silently disappear.

For example:

```text
D12: component A owns lifecycle
status: active

new evidence
↓

D27: component B owns lifecycle
supersedes: D12

D12:
status: stale
reason: premise D8 falsified by evidence E19
```

---

## Role-specific streams

### Strategic planner

May record:

- roadmap assumptions;
- priority judgments;
- dependency interpretations;
- expected-value changes;
- deferred opportunities;
- strategic uncertainties;
- recommendations to continue, reorder, pause, split, or stop work.

### Supervisor

May record:

- plan acceptance judgments;
- continuation judgments;
- authority interpretations;
- campaign amendment consequences;
- planner-invocation judgments;
- terminal acceptance or stop decisions.

### Builder

May record:

- evidence interpretations;
- ownership/placement hypotheses;
- route selection;
- route invalidation;
- scope interpretation;
- validation interpretation;
- confidence changes;
- judgments that evidence is sufficient;
- judgments that additional investigation is unnecessary.

### Reviewer

May record:

- suspected defects;
- evidence interpretations;
- severity/consequence judgments;
- confidence changes;
- finding retraction;
- repaired/rejected/superseded finding disposition;
- residual uncertainty.

### Architectural reviewer

May record:

- architecture coherence judgments;
- ownership concerns;
- dependency concerns;
- architecture assumptions;
- recommendations to continue or reconsider;
- confidence and evidence basis.

The primitive remains general while role projections remain specific.

---

## Visibility and independence

Durability does not imply universal visibility.

This is critical for independent review.

A reviewer whose evidence claim depends on independence should not inherit the builder's decision trace before forming its own initial judgment.

Role environments should therefore control decision-stream visibility.

Conceptually:

```text
Builder
  OWNS decisions/builder.jsonl
  MAY_READ authorized planner/supervisor decisions

Reviewer
  OWNS decisions/reviewer.jsonl
  MAY_READ proposal core
  MAY_READ accepted scope
  MAY_READ implementation artifact
  MAY_READ evidence allowed by review contract
  INDEPENDENT_OF decisions/builder.jsonl at fresh entry

Supervisor
  MAY_READ promoted decision consequences
  MAY_INSPECT detailed decision trace when authority permits
  OWNS supervisor decision stream
```

After initial independence is established, selected adjudicated consequences may become visible when useful.

Decision visibility should become part of the Agent Environment Graph.

---

## Claude interoperability

The decision-trace mechanism should not depend on one provider's model context or tool runtime.

Claude should be able to participate as a first-class role using the same durable decision protocol.

A practical initial implementation is to expose a **role-specific decision file or append capability** to the Claude process.

For example:

```text
work-dossier/
  decisions/
    reviewer-claude.jsonl
```

Claude's prompt would explain:

- the purpose of the decision trace;
- which observable judgment events belong there;
- that it should not log hidden reasoning or a narrative transcript;
- the schema for appending a decision event;
- the evidence/invariant identifiers it may reference;
- the visibility boundary for other roles' decision streams;
- that expressed confidence is self-reported confidence, not calibrated probability.

Claude may then append structured decision events during review:

```json
{
  "decision_id": "review-D12",
  "role": "reviewer",
  "kind": "finding_hypothesis",
  "subject": "completion publication authority",
  "judgment": "possible authority leak",
  "confidence": 0.61,
  "evidence_refs": ["E31", "E34"],
  "status": "active"
}
```

Later:

```json
{
  "decision_id": "review-D18",
  "role": "reviewer",
  "kind": "finding_disposition",
  "supersedes": ["review-D12"],
  "judgment": "concern resolved by exact publication binding",
  "confidence": 0.89,
  "evidence_refs": ["E44"],
  "status": "confirmed"
}
```

The important implementation property is that Claude writes to **its own role-scoped stream**.

It does not need write access to:

- builder decisions;
- supervisor decisions;
- persistent state owned by another role;
- receipts;
- proposal core.

A coordinator or state service can promote relevant Claude decisions into shared active state when authorized.

### Transport options

The initial transport can remain variant structure.

Possible implementations include:

1. a bounded append-only JSONL file;
2. an MCP/tool capability such as `record_decision`;
3. a local decision-state service;
4. a repository-external Work Engine state database;
5. a provider adapter that translates structured reviewer output into decision events.

The invariant is not "Claude must write JSONL."

The invariant is:

> **A role's promoted observable judgments can be durably recorded with stable identity, provenance, confidence semantics, and visibility boundaries independent of provider session lifetime.**

### Claude review independence

For fresh independent review, Claude should receive:

- proposal/core objective as allowed;
- accepted scope;
- implementation artifact/diff;
- relevant evidence;
- invariant/environment projection;
- its own empty or prior reviewer stream according to reviewer lifecycle.

It should not receive the builder decision trace at fresh independent entry.

If the same Claude reviewer persists through remediation, its own prior reviewer decisions remain visible.

Builder/supervisor adjudication of findings should be returned as explicit dispositions, not by exposing unrestricted builder reasoning.

This gives Claude continuity without contaminating initial independence.

---

## Relationship to model context

Model context remains transient role-local cognition.

It contains:

- active hypotheses;
- comparisons;
- temporary abstractions;
- recently retrieved evidence;
- abandoned branches;
- tentative interpretations;
- local reasoning in motion.

The decision trace contains observable judgments that have semantic relevance beyond the instant they were formed.

A useful distinction is:

> **Context contains thought in motion.  
> The decision trace contains observable decision events.  
> The active decision set contains judgments currently relied upon.**

This preserves flexibility without losing forensic history.

---

## Relationship to raw trace

The decision trace does not replace raw forensic trace.

Raw trace may preserve:

- visible prompts/messages;
- model outputs;
- tool calls;
- tool results;
- evidence retrieval;
- compaction/restart events;
- runtime errors.

The decision trace provides semantic indexing over that history.

A future introspection system could show:

```text
Decision D44
"caller B is non-authoritative"
confidence 0.76
        ↓
evidence E18 / E23
        ↓
premise for D51
        ↓
later falsified by E71
        ↓
raw trace drill-down
```

This makes the investigation process understandable without requiring the UI to interpret a full transcript from scratch.

---

## Relationship to persistent state

Persistent state answers:

> **What is operationally true now?**

The decision trace answers:

> **What judgments shaped the path to the current position?**

The active decision set answers:

> **Which judgments are currently relied upon?**

State may therefore reference decisions:

```yaml
current_route: route-7
route_decision_ref: D51

placement:
  owner: component-B
  decision_ref: D63

open_findings:
  - finding-F3
```

State should not copy the decision history.

---

## Relationship to proposal packets

The proposal packet remains immutable pre-execution context.

It describes:

- objective;
- assumptions;
- evidence;
- placement hypotheses;
- predicted value;
- predicted cost;
- complexity;
- fan-out;
- risk;
- validation burden;
- confidence;
- dependencies.

The decision trace records how those beliefs evolve during implementation.

This creates:

```text
proposal
what we believed before execution
        ↓
decision trace
how interpretation evolved
        ↓
active decisions + state
what currently governs execution
        ↓
receipt / outcome
what happened
```

---

## Work Dossier

The broader concept may be a **Work Dossier** containing the durable information layers associated with one piece of work.

```text
WORK DOSSIER
    │
    ├── Proposal origin
    │      what was believed before execution
    │
    ├── Role decision traces
    │      how interpretation evolved
    │
    ├── Active decision projection
    │      what is currently relied upon
    │
    ├── State references
    │      where execution stands
    │
    ├── Evidence references
    │      what establishes important claims
    │
    ├── Outcome references
    │      receipts / metrics / checkpoints
    │
    └── Raw trace references
           observable forensic execution history
```

The proposal packet remains the immutable origin.

The Work Dossier is the larger lifecycle container.

The exact storage layout and name remain variant structure.

---

## Closed-loop learning

The decision trace can become a uniquely valuable dataset.

It can support analysis of:

- confidence calibration by role;
- bad-premise propagation;
- decision ancestry;
- recurring evidence-selection mistakes;
- placement survival under falsification;
- route-revision causes;
- reviewer false-positive and false-negative patterns;
- which small judgments most often precede later failure;
- whether certain environment configurations improve judgment;
- which capabilities actually change decisions;
- how role-specific uncertainty evolves.

For example:

```text
D17  caller B non-authoritative   confidence .76
D18  owner A likely              confidence .81
D19  placement A supportable     confidence .88

later:
E31 establishes B authoritative
```

The interesting failure is not merely that D19 was wrong.

The useful learning signal is that confidence propagated through D17 into later decisions.

That may imply:

- evidence availability was poor;
- role instructions underweighted caller verification;
- the environment lacked a useful capability;
- prior context biased interpretation;
- confidence was poorly calibrated.

This is much richer than terminal outcome metrics alone.

---

## Introspection value

A future introspection UI could render decision evolution directly:

```text
Proposal
"Owner may be A"
confidence .63
       ↓
Builder D17
"Caller B appears non-authoritative"
confidence .76
       ↓
Builder D18
"A likely owns lifecycle"
confidence .81
       ↓
Reviewer D24
"B is authoritative"
confidence .91
       ↓
Builder D27
"Prior placement premise falsified"
confidence .97
       ↓
Route revision
       ↓
Outcome
```

Every node can connect to:

- evidence;
- invariant;
- state transition;
- role environment;
- receipt;
- raw trace.

This creates a durable representation of the role's path through decision space.

---

## Initial implementation target

The first version should remain narrow.

It does not require:

- full graph storage;
- a UI;
- calibrated confidence;
- automatic extraction of every decision;
- hidden chain-of-thought logging;
- every role;
- a complete Work Dossier service.

A useful first implementation may provide:

1. stable `decision_id`;
2. `work_id`, `agent_id`, and role;
3. decision kind;
4. observable judgment;
5. expressed confidence;
6. evidence/invariant references;
7. status;
8. decision relations such as `premise_for` and `supersedes`;
9. explicit consequences;
10. role-scoped append/read capability;
11. active-decision projection;
12. visibility rules preserving independent review;
13. provider-independent support for Codex and Claude.

The first implementation should prove that:

- intermediate judgments can be reconstructed after context replacement;
- a falsified premise identifies dependent decisions that deserve reconsideration;
- a superseded decision does not become active after compaction;
- a replacement agent can understand the current route without transcript replay;
- Claude can record reviewer decisions through the same provider-independent protocol;
- fresh Claude review remains independent of builder decision history;
- forensic inspection can trace a final decision backward through intermediate judgments and evidence.

---

## Design boundaries

### Truth

Expressed confidence is not calibrated probability unless calibration is demonstrated.

Evidence, inference, judgment, active reliance, and outcome remain distinct.

### Context economy

Do not require every thought to become a durable record.

### Forensics

Do not discard intermediate observable judgments merely because they did not directly change final behavior.

### Independence

Decision visibility must respect role and review boundaries.

### Ownership

Each role owns its decision trace.

### Provider independence

Decision semantics belong to Work Engine, not Codex, Claude, or any provider-specific session format.

### Provenance

Decisions should reference evidence and invariants when those relationships materially establish their meaning.

### Maintainability

The trace records what the role decided; it does not prescribe how future roles must reason.

---

## Compact statement

The process needs more than final decisions.

Small judgments can become premises for larger judgments, and their mistakes can propagate invisibly if only the final decision survives.

> **Context contains thought in motion.  
> Decision trace preserves observable judgment history.  
> Active decisions preserve what execution currently relies upon.  
> State preserves current operational truth.  
> Receipts preserve durable outcomes.  
> Raw trace preserves forensic execution history.**

Together, these layers let Work Engine preserve the evolution of a role's decision space without requiring model context itself to be durable.
