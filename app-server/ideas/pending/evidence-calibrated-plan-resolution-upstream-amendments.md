# Proposed Upstream Amendments for Evidence-Calibrated Plan Resolution

## Status

Candidate amendments supporting
[evidence-calibrated plan resolution and continuous capability learning](evidence-calibrated-plan-resolution-and-continuous-capability-learning.md).
They are not accepted or applied and do not authorize implementation. Each
target's decision owner retains authority over its disposition.

The entries identify possible changes to existing proposal and architecture
documents. They are design hypotheses, not execution findings. Target content
and ownership need confirmation when an amendment is formed or applied.

```yaml
idea_status:
  architectural_supersession: not_applicable
  architectural_supersession_note: "Checked 2026-09-17, per review: no plausible canonical owner exists to compare against -- this is an editorial amendment sheet proposing changes to other pending idea documents' own text (see residue_note below); it asserts no architecture of its own for a canonical view to state or fail to state."
  residue: none
  residue_note: "Editorial amendment sheet; proposes changes to other pending ideas' text, never asserts new ownership itself. Confirmed clean 2026-09-16 (see 'Citation currency note' below)."
  backlog: none
  backlog_note: "No dedicated staged-plan section; amendments target other documents' own pilot/backlog content, not this document's."
  audit_scope:
    - keyword-scan: full_document
    - close-read: "full document (286 lines)"
  audit_scope_completeness: complete
  status_as_of: 2026-09-17
```

```yaml
idea_provenance:
  origin: direct_capture
  related_reconciliations:
    - evidence-calibrated-plan-resolution-and-continuous-capability-learning.md
```

## Amendment map

| Target | Candidate changes |
| --- | --- |
| Decision-gated compilation | Coordinate the pilot matrices; expose continuation observations; test closure-cost prediction; locate possible split-execution criteria. |
| Structural Plan IR | Investigate renderability and separability checks; clarify cost accounting; connect the resolution pilot to a direct baseline. |
| Execution-profile scoring | Account for context burden and continuation; distinguish reasons excessive resolution can fail. |
| Planner-owned characterization | Test whether characterization predicts additional closure cost. |
| Provider/harness runtime | Clarify cache-reuse observations and their relationship to continuation identity. |
| Revisioned research | Scope cost claims to reconstruction coverage; account for exploration starvation. |
| Slice-bounded builder context | Reference its discussion of context burden from the relevant cost proposals. |

## A. Decision-gated implementation compilation

Target: [decision-gated implementation compilation](proposal-decision-gated-implementation-compilation.md).

This proposal owns the implementation-contract workflow, decision authority,
plan-conformance gate, execution model routing, and measurement contract.

### A1. Coordinate compilation and resolution comparisons

Stage 5 includes direct Sol implementation, Sol compilation followed by Sol
implementation, and Sol compilation followed by Spark implementation. The
direct baseline exists. Plan IR section 20 proposes a separate comparison of
resolution levels.

Candidate change: coordinate those comparisons so the evidence can distinguish
the value of compilation from the value of a particular projection resolution.
One option is to vary resolution within a bounded Stage 5 task class. Another
is to bind the separate Plan IR pilot to compatible subjects and measurement
semantics. The pilot owners should choose the design.

### A2. Describe continuation conditions in cost measurements

Stage 0 includes cache use where observable, phase tokens, and latency.
Candidate additions are cache read/write usage per turn, elapsed time between
branch turns, relevant prefix changes, and observed or inferred causes of
reduced reuse. Possible causes include plan revision, context replacement,
realization change, and expiry; unknown causes remain unknown.

These observations could help distinguish executor effects from differences in
continuation conditions. The measurement contract should state what remains
unestablished when the provider does not expose the required state.

### A3. Test closure-cost prediction before using it for routing

The proposal considers whether compilation and conformance review cost less
than the implementation they displace. Add the question of whether planning
characterization predicts the additional decision closure a candidate
realization requires before that closure is performed.

A pilot could record and score predictions without consulting them for routing.
Whether they become useful decision inputs depends on their demonstrated
accuracy and the consequences of error.

### A4. Expose observations to the context lifecycle manager

The context-lifecycle boundary places retention and replacement with the
external manager. Its existing inputs include cached inference and transition
cost.

Candidate additions to the exposed observations are time since the last turn,
provider-reported reuse, estimated warmth with its uncertainty, and pending
plan changes. They inform the existing owner without giving task models
context-management authority.

### A5. Locate possible split-execution criteria

If one compiled contract is assigned to multiple realizations, routing needs
an account of whether the proposed regions can be implemented compatibly and
how joint acceptance is established. The companion's separability hypothesis
is a candidate input.

This is a future design question. Neither dependency independence nor a
region's economic size establishes safe composition, and this entry does not
recommend adopting split execution now.

## B. Structural Plan IR

Target: [Structural Plan IR](structural-plan-ir-for-capability-aware-multi-model-execution.md).

### B1. Investigate renderability and separability checks

Section 16 discusses machine-checkable plan quality. Two candidate extensions
are:

- **Renderability.** Identify projections whose contents follow from the IR,
  and expose unresolved material decisions instead of silently choosing them.
  The relevant compiler outcome is `returned_for_decision`; missing evidence
  may instead require `blocked_by_evidence`. Model-assisted wording alone does
  not establish a missing decision.
- **Separability.** Examine shared invariants, open judgments, forbidden
  boundaries, and acceptance conditions across proposed execution regions.
  Closure disjointness is one conservative hypothesis, not a complete proof or
  a reason to reject other partitions with valid integration contracts.

The pilot should establish which parts are mechanically decidable. Passing
structural checks cannot establish implementation correctness on its own.

### B2. Connect the resolution pilot to a direct baseline

Section 20's proposed arms compare plan-based execution. Bind the resolution
comparison to the direct-implementation baseline and measurement semantics in
A1, so a result can explain both whether compilation helps and which resolution
helps under the tested conditions.

### B3. Clarify the economics of elaboration

Section 12 proposes a ratio of downstream compute avoided to additional upstream
compute. Clarify artifact-production inference, executor context ingestion,
and other runtime costs within that comparison.

Deterministic expansion may require no model inference while increasing input
tokens. Cache treatment can change input cost without changing production
cost. Monetary totals should avoid double-counting usage; context burden needs
quality observations rather than an arbitrary conversion to money.

### B4. Measure deterministic expansion

Section 9 proposes deterministic or low-cost elaboration of relationships
already encoded. Add an explicit hypothesis about how much of a target
projection can be produced that way, with measures that distinguish faithful
rendering from new decision-making.

The result would test an economic premise of Plan IR without assuming that
every non-deterministic rendering operation is a plan defect.

### B5. Distinguish ordering from independent realization

Section 7 includes model assignment among uses of the dependency graph. Add the
limitation that order-independent changes may still require coordinated
judgment or a shared integration contract. Reference the investigation in B1
rather than treating graph independence as an execution guarantee.

## C. Execution-profile scoring and strategy selection

Target: [execution-profile scoring addendum](structural-plan-ir-addendum-execution-profile-scoring-and-strategy-selection.md).

### C1. Account for context burden

The economic objective includes planning, projection, execution, supervision,
verification, repair, latency, and failure consequences. Relate those costs to
the context risks described in
[slice-bounded builder context](../../../ideas/slice-bounded-builder-context.md).

Candidate change: evaluate context burden through quality and behavior
observations alongside economic cost. Whether it needs a separate constraint
or is adequately represented by existing quality measures remains open.

### C2. Share continuation measurement semantics

Use the same continuation definitions and uncertainty treatment as A2 where
both pilots make comparable claims. This preserves comparability without
creating a second owner for the measurement contract.

### C3. Distinguish excessive-resolution mechanisms

The resolution curve should distinguish additional planner inference, executor
context burden, and planner overreach into implementation. The first two are
cost or quality effects to measure. The third violates the planning boundary
regardless of whether the resulting work is cheap.

## D. Planner-owned execution characterization

Target: [planner-owned characterization addendum](structural-plan-ir-addendum-planner-owned-execution-characterization.md).

### D1. Test characterization as an input to closure-cost prediction

The pilot considers incremental characterization effort, disagreement with
observed difficulty, additional investigation, and strategy prediction.
Candidate addition: measure whether characterization predicts the additional
closure needed by a candidate realization.

This would test the companion's economic hypothesis while keeping
characterization separate from executor selection.

## E. Provider turn, harness runtime, and operator projection

Target: [provider/harness runtime direction](provider-turn-harness-runtime-and-operator-projection.md).

### E1. Define the boundary for cache-reuse observations

Section 7 describes invalidation consequences across provider, harness, and
operator-projection boundaries. Its projection port concerns the UI, not a
Plan IR rendering. Clarify which runtime changes affect cache reuse, including
expiry without a realization change, without conflating those projections.

A changed plan or rendering may alter only part of a prefix; a boundary change
alone does not establish a cache miss. The runtime can expose what it observes,
while the lifecycle owner retains continuation decisions. The documents should
make that relationship explicit.

### E2. Relate continuation lineage to realization and projection identity

Candidate question: what continuation reference is needed to interpret cache
observations alongside an exact realization and projection?

Reuse existing identity where it preserves the required distinction. A
projection digest can identify projection content, but cannot by itself prove
the provider's complete prefix identity or current cache state. Any additional
identity should follow from the measurement claim rather than duplicate an
existing semantic owner.

## F. Revisioned research and execution architecture

Target: [revisioned research and execution architecture](../../docs/revisioned-research-and-execution-architecture.md).

### F1. Make cost-claim coverage explicit

Section 27 makes coordinate adequacy claim-relative; section 29 discusses hidden
ambient state and reconstruction cost. Apply those distinctions to cache and
continuation conditions in cost comparisons.

Candidate change: a cost consumer declares which continuation conditions its
claim depends on, and admission evaluates whether reconstruction or observation
covers them. A controlled cold run supports a cold-path claim. A controlled
reuse experiment supports its declared reuse conditions. Unavailable cache
state does not establish either class.

A replay may omit an original incumbent's reuse advantage and bias a switching
comparison. The scope and direction of that effect require evidence for the
particular design. Research remains subject to the value and cost of obtaining
the required observations.

### F2. Account for exploration starvation

A policy may repeatedly favor an incumbent because its context is reusable,
leaving alternatives under-observed. This differs from overfitting an existing
benchmark corpus: the missing evidence may never be collected.

Candidate change: describe this failure mode and evaluate ways to preserve
useful authorized exploration. A separately governed allocation is one option;
a universal rule to ignore continuation cost is not established here.

## G. Slice-bounded builder context

Target: [slice-bounded builder context](../../../ideas/slice-bounded-builder-context.md).

### G1. Reference its context-burden discussion

Reference this discussion from the cost and measurement proposals where
relevant. It supplies an existing account of context competition, capability
salience, drift, and compaction risk. No claim of exclusive ownership or
exhaustive repository coverage is needed to make that connection.

## Citation currency note (2026-09-16)

Checked against a null hypothesis (stale ownership / target no longer
matches current architecture) before being committed. All seven targets
remain real, existing documents; no factual drift found in how this
document characterizes any of them.

One completeness note: Section A's target, `proposal-decision-gated-
implementation-compilation.md`, has since become the cited source idea
behind two settled canonical views — `implementation-contract-
compilation.md` and `material-decision-selection.md`. This does not make
A1-A5's proposed amendments stale: they target that idea's still-exploratory
pilot-design and measurement content (Stage 5 comparison coordination,
closure-cost prediction, split-execution criteria), not the structural
content already homed in those two views, and both views remain their own
`proposed`/`exploration_only` status — nothing here has been decided out
from under this document.

## Boundaries and evidence limits

This document does not own target content, acceptance of related proposals,
measurement schemas, routing policy, or implementation authority. The companion
owns the architectural hypothesis; each target retains its own semantics.

The amendments have no pilot results attached. Document observations describe
proposal text, not implemented runtime behavior. Provider mechanics and prices
need validation for the realization used in any future measurement. Broader
ownership and implementation coverage remain outside this editorial document.
