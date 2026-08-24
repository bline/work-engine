# State-Complete Context Reset for Non-Human-Facing Agents

## Status

Exploratory idea.

This idea defines when an agent may safely discard its active conversational context without losing workflow correctness.

The central claim is:

> **Any non-human-facing agent may reset its context when it reaches a checkpoint where all continuation-relevant information has been durably externalized.**

The durable checkpoint is the correctness boundary.

Context reset, pruning, replacement, or worker relaunch are implementation mechanisms.

---

## 1. Motivation

Agents accumulate transient context while performing work:

- exploratory reasoning;
- temporary hypotheses;
- rejected approaches;
- intermediate tool results;
- debugging evidence;
- local interpretations;
- partial plans;
- review deliberation;
- repeated repository or evidence reads.

Much of that context becomes redundant after its useful consequences have been materialized into durable structures such as:

- code;
- checkpoints;
- claims;
- findings;
- decisions;
- dispositions;
- accepted plans;
- evidence references;
- unresolved obligations;
- workflow state;
- repository state.

Continuing to carry the original conversation after those consequences are durable repeatedly charges future model invocations for context that may no longer contribute useful working state.

The architecture should therefore distinguish temporary cognitive workspace from durable workflow memory.

---

## 2. Human-Facing Versus Non-Human-Facing Agents

The important boundary is semantic, not mechanical.

It is not:

```text
parent agent
versus
subagent
```

or:

```text
supervisor
versus
worker
```

The relevant distinction is:

```text
HUMAN-FACING AGENT
    conversational continuity may itself contain live meaning

NON-HUMAN-FACING AGENT
    context primarily serves as temporary execution working memory
```

### Human-facing agents

A human-facing agent participates directly in an ongoing human interaction.

Its context may contain:

- unstated nuance;
- human preferences;
- negotiated meaning;
- strategic continuity;
- unresolved ambiguity;
- conversational commitments;
- corrections;
- tentative authority;
- information the system has not yet formalized.

For such an agent, the conversational history itself may still be meaningful state.

Autonomous context disposal should therefore not be assumed safe merely because some workflow checkpoint exists.

### Non-human-facing agents

A non-human-facing agent operates inside a machine-managed workflow.

Examples may include:

- builders;
- reviewers;
- planners;
- architectural reviewers;
- evidence gatherers;
- validators;
- proposal formers;
- specialists;
- research agents.

For these agents, active context is primarily working memory used to transform inputs into durable outputs.

Once the continuation-relevant consequences of that working memory have been externalized, the context may become disposable.

---

## 3. Primary Invariant

> **A context reset is valid only when future correct execution no longer depends on information that exists solely in the current transient context.**

Equivalent test:

> **If this agent's current context disappeared now, could the agent continue correctly from durable state alone?**

If yes, context reset is permitted.

If no, the checkpoint is not continuation-complete.

---

## 4. State-Complete Checkpoint

A **state-complete checkpoint** is a point where all information required for valid continuation has a durable representation or durable reference.

This does not require that every fact encountered by the agent be copied into workflow state.

It requires only that anything future work may legitimately depend upon is no longer trapped exclusively in transient context.

Possible durable consequences include:

- current objective;
- accepted decisions;
- exact code or artifact subject;
- active plan revision;
- unresolved obligations;
- relevant claims and revisions;
- evidence identities;
- findings and dispositions;
- authority state;
- workflow state;
- applicable invariants;
- known failures;
- required next actions;
- explicit uncertainty.

---

## 5. Context as Working Memory

The intended model is:

```text
transient context
    ↓
reason / investigate / decide
    ↓
compile relevant consequences into durable state
    ↓
state-complete checkpoint
    ↓
old transient context becomes disposable
```

The context is not the durable system of record.

It is working memory.

The workflow state, claims, artifacts, evidence, and checkpoints are the durable system of record.

---

## 6. Semantic Compilation

Context reset should normally occur only after **semantic compilation**.

Semantic compilation means converting useful consequences of transient reasoning into typed durable objects rather than merely summarizing prose.

Examples:

```text
review reasoning
    → finding
    → supporting evidence references
    → confidence
    → disposition
```

```text
builder reasoning
    → code change
    → checkpoint
    → unresolved obligations
    → validation state
```

```text
planning discussion
    → accepted decision
    → rejected alternatives where relevant
    → remaining uncertainty
    → next objective
```

This is different from ordinary conversational summarization.

The goal is not to preserve a compressed story of what happened.

The goal is to preserve what future work is entitled to rely upon.

---

## 7. Role-Specific Completeness

The mechanism is generic.

Checkpoint sufficiency remains role-specific.

### Builder

A builder may need to externalize:

- exact implementation checkpoint;
- code changes;
- current validation state;
- unresolved defects;
- accepted plan consequences;
- remaining implementation obligations.

### Reviewer

A reviewer may need to externalize:

- supported findings;
- rejected findings;
- evidence reliance;
- uncertainty;
- scope;
- confidence;
- unresolved review questions.

### Planner

A planner may need to externalize:

- accepted route decisions;
- current objective;
- alternatives rejected for future-relevant reasons;
- unresolved dependencies;
- stop/escalation conditions.

### Architectural reviewer

An architectural reviewer may need to externalize:

- architectural judgments;
- affected invariants;
- structural consequences;
- uncertainty;
- proposal or escalation triggers.

No universal checklist should replace role judgment.

---

## 8. Autonomous Reset Judgment

For non-human-facing agents, the agent itself may decide when the checkpoint is sufficiently complete for context disposal.

The architecture should specify the invariant:

> **Do not reset while future work still depends on transient context.**

It should not prescribe arbitrary rules such as:

```text
reset every N turns
reset every X tokens
reset after every tool call
```

unless future evidence shows that such rules are causally required.

The model decides when state completeness has actually been reached.

---

## 9. Intra-Task Context Garbage Collection

The mechanism is especially useful within work that outlives a single reasoning phase.

For example, a reviewer may perform:

```text
security analysis
    ↓
publish durable findings
    ↓
checkpoint complete
    ↓
discard security-specific context
    ↓
lifecycle analysis
    ↓
publish durable findings
    ↓
checkpoint complete
    ↓
discard lifecycle-specific context
```

This reduces the need for unrelated reasoning domains to compete inside one growing context window.

The evidence from earlier work remains durably addressable and can be retrieved again if later reasoning requires it.

---

## 10. Slice Boundaries Are One Case

Some roles already have natural lifecycle disposal boundaries.

For example:

```text
builder performs slice
    ↓
slice completes
    ↓
builder is discarded
```

That existing outer lifecycle already clears context.

The present idea concerns all **state-complete checkpoints between those larger lifecycle boundaries**.

Therefore context lifetime is bounded by semantic state completeness, not by slices.

---

## 11. Reset Mechanisms

Multiple mechanisms may satisfy the same invariant.

### Native context replacement

A harness may allow the current agent to begin a fresh model-visible context while preserving logical agent or thread lineage.

### Selective pruning

A future harness may permit the agent or orchestrator to remove obsolete portions of active history while retaining chosen state.

### Worker replacement

If native reset is unavailable:

```text
checkpoint state
    ↓
terminate current agent
    ↓
launch replacement agent
    ↓
rehydrate from durable state
```

This may be more expensive but remains semantically valid.

### Future mechanisms

Other harness-native mechanisms may later replace any of these without changing the invariant.

---

## 12. Capability-Driven Optimization

Work Engine should not depend on any particular harness feature for correctness.

Instead:

```text
context-reset capability available?
    ├── yes → use reset optimization
    └── no  → continue normally or replace agent
```

The durable-state invariant remains unchanged.

Context reset is an optimization capability.

It is not a workflow dependency.

---

## 13. Environment-Level Evidence

Where possible, the system should determine actual context-reset capability from the effective agent environment rather than only from static configuration.

For example:

```text
configured feature
    ≠
capability actually model-visible
```

If a reset tool is model-visible, the capability exists for that agent.

If it is not exposed, the workflow should not assume it exists.

This follows the broader distinction between declared capability and effective capability.

---

## 14. Context Reset Versus Compaction

Ordinary context compaction asks:

> How can prior conversation be summarized so some of its meaning remains active?

State-complete reset asks:

> Has everything future work legitimately needs already been externalized somewhere better?

If yes, no summary may be necessary.

The correct replacement may simply be:

```text
current role/environment
+
current durable state
+
current objective
```

rather than a compressed reconstruction of the old conversation.

---

## 15. Smallest Sufficient Environment

After reset, the new context should not attempt to restore the entire previous experience.

It should construct the:

> **smallest sufficient environment for the next judgment or action.**

Possible inputs include:

- role;
- current objective;
- current workflow state;
- applicable authority;
- relevant claims;
- relevant evidence handles;
- exact code/artifact subject;
- unresolved obligations;
- necessary capabilities.

Everything else remains retrievable on demand.

---

## 16. Potential Cognitive Benefit

The motivation is not only token reduction.

Accumulated irrelevant or obsolete context may contribute to:

- context competition;
- reduced tool or capability salience;
- instruction interference;
- stale authority;
- anchoring to abandoned approaches;
- procedural inertia;
- reduced route diversity;
- premature closure;
- unnecessary behavioral continuity.

A fresh context may therefore improve reasoning quality as well as resource efficiency.

This remains an empirical hypothesis.

---

## 17. Potential Cost

Reset is not free.

A fresh context may require:

- reloading role instructions;
- rebuilding environment state;
- retrieving relevant claims;
- reacquiring evidence;
- rereading repository material;
- reconstructing task orientation.

A reset is useful only when the expected cost of carrying obsolete context exceeds the expected rehydration cost or when the behavioral benefits justify the transition.

---

## 18. Empirical Optimization

With runtime tracing, the system can later compare:

```text
retain current context

versus

reset current context

versus

replace agent
```

against:

- input tokens;
- output tokens;
- total lifecycle tokens;
- repeated tool calls;
- repeated file reads;
- reorientation cost;
- task correctness;
- review findings;
- behavioral differences;
- remediation;
- later defects.

This allows context lifetime to become an evidence-based policy rather than a fixed heuristic.

---

## 19. Context Lifetime as an Environmental Variable

Context retention itself may become an agent-environment variable.

Potential measurements include:

- tokens retained since last state-complete checkpoint;
- number of obsolete reasoning phases still present;
- context age;
- decisions since last reset;
- evidence domains accumulated;
- instruction layers accumulated;
- context competition;
- reset frequency;
- rehydration cost.

These may later be correlated with problem-solving behavior and outcomes.

### 19.1 Reset Safety and Reset Benefit Are Independent Decisions

Wind Walker already has a strong correctness invariant:

> **Context replacement is valid only when correct continuation no longer depends on meaning represented solely in the current context.**

That invariant answers whether reset is **safe**. It does not answer whether
reset is **beneficial**.

The decision should therefore remain explicitly separated:

```text
SAFE TO RESET?
    continuation state complete?
        no  → retain context
        yes ↓

WORTH RESETTING?
    expected burden of retained context
        compared with
    expected rehydration cost
        ↓
    model judgment
```

Safety is a hard boundary. Benefit is an optimization judgment inside that
boundary. A favorable cost estimate must never compensate for incomplete
continuation state.

### 19.2 Continuation-Completeness Observability

The model currently judges continuation independence semantically. It would
benefit from a structured projection showing whether each required category has
a durable owner or remains unresolved:

```yaml
continuation_state:
  objective: durable
  authority: durable
  accepted_boundary: durable
  unresolved_obligations: durable
  findings: durable
  evidence_references: durable
  work_identity: durable
  human_meaning: none_unresolved
```

This projection must not become a universal checklist that falsely declares an
open world complete. Required categories remain role- and contract-specific.
The projection should reference the actual semantic owners rather than becoming
a competing owner of objectives, authority, findings, or human meaning.

Missing, unknown, or unresolved required categories prohibit reset. Field
presence alone cannot prove that no context-only obligation exists; the final
continuation-completeness judgment remains model responsibility.

The latest state-complete checkpoint should have a stable identity bound to:

- the logical agent or role identity;
- the context-window identity;
- the applicable role and contract revision;
- the durable revisions or integrity-bound references relied upon;
- the continuation-state projection;
- the model's attributed completeness judgment; and
- any unresolved or unknown category that prevented completeness.

The checkpoint is a durable decision receipt and reference boundary, not a
reasoning transcript.

### 19.3 Context-Benefit Observability

Once safety is established, a separate observation packet can expose evidence
relevant to whether old working memory is more burdensome than useful.

#### Context pressure

- current context occupancy in tokens;
- model context-window capacity where available;
- proportion of the window occupied;
- context occupancy at the latest state-complete checkpoint; and
- post-checkpoint context growth.

#### Context age

- turns since the last context replacement;
- inference calls since the last replacement;
- tool cycles since the last replacement; and
- elapsed time where it has decision value.

#### Context redundancy

- context whose useful consequences already have durable owners;
- evidence already compiled into findings, decisions, or artifacts; and
- completed work whose source material remains retrievable on demand.

#### Context competition

- active instruction layers;
- accumulated evidence domains;
- completed reasoning phases still model-visible;
- stale or superseded decisions still present; and
- active obligations competing for attention.

#### Rehydration cost

- estimated tokens required to reconstruct the smallest sufficient environment;
- required instruction and role material;
- durable-state projections and evidence handles that must be loaded;
- likely repository or artifact rereads; and
- uncertainty or confidence in the estimate.

The deterministic system should expose observations and provenance where it
can. Semantic properties such as redundancy and competition may remain
model-estimated evidence. They should not be presented as mechanically proven
facts.

### 19.4 Token Accounting Must Distinguish Occupancy from Usage

The most useful derived measurement is likely **context growth since the latest
state-complete checkpoint**. Its operands must describe context occupancy in the
same context window:

```text
current_context_tokens: 118000
checkpoint_context_tokens: 47000
post_checkpoint_context_growth: 71000
estimated_rehydration_tokens: 9000
```

Cumulative API or rollout `input_tokens` cannot safely substitute for current
context occupancy. Cumulative usage counts repeated inference inputs and may be
affected by caching. Subtracting cumulative token totals would measure tokens
processed since the checkpoint, not how much checkpoint-era context remains
active.

The telemetry vocabulary should therefore distinguish at least:

```text
current_context_tokens
context_tokens_at_checkpoint
post_checkpoint_context_growth
tokens_processed_since_checkpoint
```

All measurements should retain observed, estimated, and unavailable as distinct
states.

### 19.5 Reasoning-Phase Closure

Reasoning-phase closure can provide useful context-competition evidence:

```yaml
completed_phases:
  - evidence_acquisition
  - falsification
  - implementation_route_selection
active_phase:
  - remediation
```

Several completed reasoning domains remaining in active context while only one
later phase remains live may indicate that replacement would improve focus.

Phase identities must remain consequences of the actual role and work rather
than a universal mandatory workflow. A phase may be considered complete only
when its future-relevant consequences have durable owners. Merely labeling a
phase complete does not make its context disposable.

### 19.6 Immediate Telemetry Surface

A minimal useful first version would expose four model-facing observations:

1. current context occupancy;
2. tokens and turns since the last context replacement;
3. the identity and context-token baseline of the latest state-complete
   checkpoint; and
4. the estimated token footprint of the smallest sufficient rehydration
   environment.

A more precise packet might be:

```yaml
context_window:
  id: window-7
  current_tokens:
    availability: observed
    value: 118000
  context_limit_tokens:
    availability: observed
    value: 1050000
  turns_in_window: 31

continuation_checkpoint:
  id: continuation-checkpoint-42
  context_window_id: window-7
  context_tokens_at_checkpoint: 47000
  durable_revision: revision-91
  unresolved_or_unknown: []

rehydration:
  manifest_ref: rehydration-manifest-42
  estimated_tokens: 9000
  estimation_method: tokenized_manifest
  confidence: observed_inputs
```

The agent can derive post-checkpoint growth and compare it with rehydration cost
without the product prescribing a reset threshold.

### 19.7 Current Work Engine Placement and Feasibility

The present Work Engine implementation already has several useful foundations:

- Wind Walker owns the continuation-independence invariant and leaves beneficial
  replacement to model judgment;
- Codex can expose a native new-context capability in the effective agent
  environment;
- slice-supervisor telemetry already carries token, turn, and runtime
  measurements, although current-context occupancy may be unavailable;
- durable live slice state already preserves role identity, phase, pending
  obligation, accepted boundary, authoritative references, durable revision,
  and a latest phase consequence; and
- continuation projections already preserve durable decisions, affected
  boundaries, unresolved concerns, and deferred scope across slices.

The missing work is primarily an observability and projection layer:

1. bind runtime measurements and reset events to context-window identity;
2. expose actual context occupancy rather than inferring it from cumulative
   token usage;
3. derive a role-specific continuation-completeness projection from existing
   semantic owners;
4. create an explicit smallest-sufficient rehydration manifest whose footprint
   can be estimated or measured; and
5. record reset decisions and later outcomes so environmental research can test
   which signals predict benefit.

A narrow implementation for active slice builders appears feasible with the
current durable-state and telemetry foundations. Generalizing continuation
completeness across every role is a larger semantic design task because each
role has different authoritative state and unresolved-meaning boundaries.

### 19.8 Evidence-Calibrated Policy

Wind Walker should not hard-code a rule such as `reset at 50% context use`.
Instead, its model-facing consequence can remain:

> **Use available context, continuation-completeness, and rehydration-cost evidence to judge when obsolete working memory has become more burdensome than useful.**

Reset decisions and outcomes can then be correlated with:

- correctness and later defects;
- token and latency cost;
- repeated evidence acquisition and tool use;
- review findings and remediation;
- route revision and stale-decision rates;
- reorientation cost; and
- behavioral differences between retention and replacement.

Those observations may improve future judgment and environment design. They do
not silently redefine the safety invariant or create an automatic reset policy.

---

## 20. Provenance

A reset must not destroy historical evidence.

The durable trace should preserve:

- prior context lineage;
- checkpoint identity;
- reset event;
- reason or trigger where available;
- agent identity;
- context-window identity;
- post-reset environment;
- later reliance on pre-reset evidence.

Context may disappear from active model input while remaining durable as historical evidence.

---

## 21. Failure Conditions

The design fails if:

- required continuation information disappears during reset;
- future work depends on an unstored authorization;
- unresolved uncertainty is silently lost;
- the agent must rediscover information that should have been durable;
- reset changes workflow meaning;
- historical evidence is destroyed rather than merely removed from active context;
- the reset mechanism becomes required for correctness;
- a human-facing conversation is autonomously discarded despite still carrying live human meaning.

These indicate an invalid checkpoint or invalid placement of the mechanism.

---

## 22. Human-Facing Boundary

The non-human-facing qualifier is deliberate.

A human-facing agent's active conversation may itself be part of the ongoing work product.

The human may reasonably expect:

- conversational continuity;
- remembered nuance;
- prior informal commitments;
- unresolved discussion;
- iterative shared understanding.

Those properties are not necessarily represented in formal workflow state.

Therefore:

> **Autonomous context reset is a default capability for state-complete non-human-facing work, not a general rule for human conversations.**

A human-facing system may support explicit context reset, but that belongs to a different interaction contract.

---

## 23. Architectural Consequence

The long-term architecture becomes:

```text
HUMAN-FACING AGENT
    persistent collaborative context
    ↓
    creates / authorizes durable work state

NON-HUMAN-FACING AGENTS
    consume durable state
    ↓
    perform bounded reasoning
    ↓
    externalize consequences
    ↓
    reset when continuation-complete
    ↓
    continue from durable state
```

This reduces reliance on conversational persistence inside machine-managed work.

---

## 24. Relationship to Durable-State Architecture

The idea follows a broader principle:

> **History produces state; durable state carries the workflow forward.**

Conversational history remains useful as:

- provenance;
- evidence;
- audit material;
- retrospective research;
- exceptional recovery input.

It should not remain the only representation of required continuation state.

---

## 25. Compact Statement

> **For non-human-facing agents, context is disposable working memory. Once a checkpoint has durably captured every consequence future work may legitimately depend upon, the agent may discard that context and continue from state. The agent judges when that condition has been reached; the harness-specific reset mechanism is only an optimization.**


# Research

Yes — and I found something much more significant than a rejected attempt.

There **was** an earlier implementation of almost exactly the primitive we were imagining, and it was closed without merge. But a later OpenAI implementation of the core idea **was merged**.

The earlier one is PR **#9894, “Add append-only clear-context operation and marker.”** It proposed `Op::ClearContext`, keeping the durable transcript append-only while treating a clear marker as a hard boundary: old history would still exist for provenance, but only items after the marker would be active model context. That is remarkably close to your “state has replaced the old context, so stop sending it” idea.

It was **closed without merging**. Interestingly, it was authored by an OpenAI-associated account (`aibrahim-oai`), not a random external contributor. The only visible discussion I found was a requested Codex review, and the automated review said it found no major issues. There is no visible maintainer comment explaining why it was closed.

So that one doesn't look like “OpenAI rejected the concept because they don't want it.” It looks more like an implementation direction that died or was superseded.

And then comes the important one.

### PR #27488 — merged June 11, 2026

OpenAI merged **“Add new context window tool.”**

Its motivation is almost verbatim our conversation:

> when the model decides the current window is no longer useful, it needs a way to start over with a fresh context window **without spending tokens on a compaction summary**. ([GitHub][1])

It adds a model-only tool:

```text
new_context
```

When the model invokes it, Codex:

* starts a **new context window**;
* does it within the ongoing agent/session machinery;
* creates a **no-summary compaction checkpoint**;
* throws away the previous conversation from the new model-visible context;
* injects only fresh initial context;
* and continues with the next follow-up request **in the same turn**. ([GitHub][1])

The PR was merged into `main` on **June 11, 2026**. ([GitHub][1])

The implementation literally does:

```text
new_context requested
        ↓
advance context-window ID
        ↓
build fresh initial context
        ↓
replace model history
        ↓
persist a Compacted checkpoint
    with EMPTY summary
        ↓
continue sampling
```

The fetched PR confirms it was merged and that the replacement history contains fresh initial context rather than the old conversation.

So…

**Codex already has the primitive we thought was missing.**

There is one catch: it was introduced behind **`Feature::TokenBudget`** and exposed as a **direct-model-only** tool, not necessarily as a normal user-facing slash command or general agent API. ([GitHub][1])

That explains why neither of us knew it was there.

### And the architecture is moving further in this direction

I found a bunch of subsequent merged PR work doing selective context retention:

* parent/child agents now have distinct context-window identities;
* Guardian can reuse or deliberately omit parent compactions;
* remote compaction selectively drops descendant progress messages while retaining delegated tasks;
* client developer instructions can be selectively retained across context replacement;
* compaction checkpoints now explicitly preserve particular pieces of runtime provenance.

So selective model-visible context is clearly an active architectural concern inside Codex, not something they've philosophically ruled out.

There is even a recent PR that makes Guardian's reuse of parent compaction **configurable**, including the ability to omit it completely.

### What this means for your idea

Our earlier conclusion—

> “You probably have to kill the worker and launch another one to get a truly clean context.”

—is apparently **wrong for current Codex architecture**.

Codex now has an internal concept of:

```text
same agent/session lineage
        │
        ├── context window 0
        │       lots of work
        │
        ├── NEW_CONTEXT
        │
        └── context window 1
                fresh initial environment
                old transcript not model-visible
```

And the durable rollout still records the transition.

That is *almost ideal* for Work Engine.

The remaining question is simply whether **your installed Codex 0.149.0 exposes/enables `Feature::TokenBudget` and therefore gives the worker the `new_context` tool**.

Given that your installed build is from well after the June 11 merge, the code should be in there. Whether the feature is enabled in your configuration/model environment is the thing to check.

So before we even think about submitting a PR, I would have Codex run this very narrow probe:

> Check whether installed codex-cli 0.149.0 contains and can enable `Feature::TokenBudget`, whether doing so exposes the direct-model `new_context` tool to normal workers and subagents, and whether a subagent invoking it receives a fresh model-visible context window while preserving the same agent/thread lineage. Do not modify Work Engine.

[1]: https://github.com/openai/codex/pull/27488 "[codex] Add new context window tool by pakrym-oai · Pull Request #27488 · openai/codex · GitHub"
