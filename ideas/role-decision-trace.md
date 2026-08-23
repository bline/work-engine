# Role Decision Trace

## Status

Exploratory observability and recovery idea.

## Idea

Preserve an attributed semantic trace of **observable role judgments that materially shape execution**, without attempting to store hidden chain-of-thought or turning every thought into product state.

## Current evidence

Work Engine already preserves:

- proposal decisions;
- route revisions;
- receipts;
- state transitions;
- review findings;
- strategic handoffs.

Those artifacts capture outcomes, but they do not provide one general lineage for intermediate judgments such as assumptions, exclusions, sufficiency judgments, and decisions later invalidated by evidence.

## Required consequence

A decision record can identify:

- stable decision identity;
- role and logical actor;
- work/proposal/slice identity;
- decision class;
- conclusion;
- confidence as expressed by the role;
- evidence references and cutoff;
- assumptions and limitations;
- relations to earlier decisions;
- whether it remains active, stale, superseded, contradicted, or resolved;
- the later consequence it influenced.

Useful relations may include:

```text
PREMISE_FOR
SUPERSEDES
WEAKENS
CONTRADICTS
AFFECTS
REOPENED_BY
CHANGED_BECAUSE_OF
```

## Active decision set

The full decision trace and the smaller set of decisions currently relied upon are different.

A runtime role may consume an **active decision set** while the historical trace remains available for recovery and forensics.

## Boundaries

The decision trace does not replace:

- proposal meaning;
- workflow state;
- review artifacts;
- receipts;
- evidence claims;
- raw provider/session traces.

It references those owners.

It must not expose protected reasoning to roles whose independence depends on not receiving it.

## Does not own a Work Dossier

A future UI may aggregate proposals, decisions, state, receipts, and raw-trace references into a dossier-like view. That is a projection problem, not a second semantic owner and does not require a separate canonical data object here.

## Compact statement

> Preserve consequential observable judgments and their lineage so later roles can understand which premises governed action and which ones became stale, without storing hidden reasoning.
