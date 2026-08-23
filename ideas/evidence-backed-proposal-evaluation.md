# Evidence-Backed Proposal Evaluation

## Status

Exploratory planning capability.

## Idea

Evaluate a formed proposal using attributed evidence about value, implementation surface, risk, reversibility, maintenance consequences, validation burden, alternatives, and uncertainty.

Evaluation produces decision support. It does not make the decision.

## Current evidence

Work Engine now has:

- durable proposal packets;
- adaptive proposal formation;
- authority-authored proposal decisions;
- strategic planning;
- formed evidence-lineage candidates.

What is still missing is a reusable evaluation capability between proposal formation and portfolio/roadmap judgment.

## Required consequence

An evaluation can expose:

- which proposal revision was evaluated;
- which claims and evidence support each dimension;
- competing interpretations or alternatives;
- confidence and limitations;
- dimensions that are incomparable or intentionally excluded;
- candidate dominance or tradeoff findings where justified;
- unresolved evidence needed before a consequential decision.

## Evaluation dimensions are not a universal score

Useful dimensions may include:

- expected value;
- urgency;
- risk;
- reversibility;
- implementation complexity;
- architectural reach;
- maintenance burden;
- validation burden;
- unlock value;
- dependency consequences.

Different proposals may require different evidence. A single scalar score should not silently substitute for judgment.

## Typed evaluation model

An evaluation should avoid flattening unlike semantic objects into one list of
apparently comparable numbers. Candidate distinctions are:

```text
facets
    durable domains of investigation

claims
    evidence-backed semantic conclusions within a facet

evidence items
    observations supporting or challenging a claim

measures
    typed evidence items with declared units, direction, scope, and cutoff

readiness judgments
    decision-specific conclusions issued by their authorized owner
```

This is not yet an accepted universal registry. Formation must decide whether a
candidate field is a facet, claim, measure, epistemic metadata, or relationship
before schema adoption. Each claim or measure should retain its evidence basis,
baseline, provenance, freshness, limitations, and confidence.

## Conditional comparison contract

Dominance or Pareto findings are meaningful only when an explicit comparison
contract makes selected measures compatible. That contract must identify:

- included measures and intentionally excluded concerns;
- definitions, units, directionality, and scope;
- treatment of missing, contested, and stale evidence;
- confidence or evidence-quality requirements;
- the evidence cutoff; and
- the authority that selected the comparison surface.

A generated relation is therefore conditional: given comparison contract M and
evidence cutoff E, proposal A is currently no worse than proposal B on every
included measure and better on at least one. It does not establish overall value,
priority, acceptance, or implementation authority. Qualitative and
incommensurable value remains visible to the decision owner.

Consumer projections may present these findings, but must name the evaluated
proposal revisions and comparison contract rather than copying the result into
a new canonical proposal or roadmap record.

## Does not own

Evaluation does not:

- accept or reject a proposal;
- authorize implementation;
- mutate roadmap priority;
- define research maturity;
- choose the execution organization;
- calibrate future evaluation from outcomes.

Those belong to decision authority, research, organizational planning, and closed-loop learning respectively.

## Compact statement

> Proposal evaluation turns evidence into explicit tradeoffs for a decision owner without turning those tradeoffs into automatic authority.
