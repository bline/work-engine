# Closed-Loop Engineering Learning

## Status

Reconciled against current app-server implementation and prospective architecture in
`app-server/docs/closed-loop-engineering-learning-reconciliation.md`. The predicted side
is confirmed supplied by `ideas/evidence-backed-proposal-evaluation.md` (item 6, typed
evaluation estimates). The observed side, including review burden (reclassified away from
review findings, which own finding semantics, not burden), decomposes into two
still-pending designs this idea should consume once they land:
`app-server/ideas/pending/incremental-terminal-accounting-projection.md` (scope,
failures/recoveries, validation evidence, review burden) and
`app-server/ideas/pending/candidate-trajectory-remediation-delta-for-native-review.md`/
`candidate-trajectory-builder-side-consumption.md` (route revisions). The primary
remaining open scope is a prediction↔outcome correspondence/comparison contract
(partially sequenced behind those pending designs, partially a genuinely unowned typed-
comparability question — units, scale, tolerance, sufficiency of one outcome for a
probabilistic prediction) and an attributed calibration diagnosis supporting multiple
possible contributing explanations (never a single causal verdict, and never treating an
adverse outcome as automatic proof a prediction was wrong), coupled to
`claim-evidence-service.md`'s existing domain-profile pattern but not built. See the
reconciliation document for the full disposition.

Exploratory product-learning idea.

## Idea

Connect proposal expectations with observed implementation outcomes so completed work can improve future proposal evaluation and planning.

Work Engine already records substantial execution evidence. The missing capability is a durable feedback loop from **predicted consequences** to **observed consequences**.

## Current evidence

Implemented machinery already produces:

- proposal packets and proposal decisions;
- campaign and slice receipts;
- validation outcomes;
- review evidence;
- route revisions;
- provider/runtime telemetry; and
- strategic planning handoffs.

Those artifacts are evidence, but they are not yet systematically used to calibrate future proposal judgments.

## Required consequence

A completed change can preserve a comparison such as:

```text
proposal expectation
    expected value
    expected complexity
    expected architectural reach
    expected validation burden
    expected risk

        ↓ compare with

implementation outcome
    actual scope
    actual review burden
    route revisions
    failures / recoveries
    validation evidence
    observed maintenance consequences
```

The result is calibration evidence that later evaluators and planners may use.

## Does not own

This idea does not own:

- raw execution metrics;
- proposal evaluation itself;
- roadmap priority;
- automatic policy changes;
- model selection;
- reward optimization;
- acceptance of future proposals.

Learning evidence informs later judgment; it does not turn historical correlation into authority.

## Important boundary

A prediction can be wrong because:

- the proposal was poorly understood;
- implementation execution was poor;
- the environment changed;
- the chosen route was unusually difficult;
- the prediction was reasonable but uncertainty realized badly.

The learning system must retain enough provenance to avoid flattening those cases into one score.

## Compact statement

> Compare what the proposal expected with what implementation actually established, and preserve that difference as evidence for future judgment.
